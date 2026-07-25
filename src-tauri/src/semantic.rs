use crate::{
    media::{self, MediaKind},
    models::{
        AnalysisFrame, DisclosedSource, ProjectContext, RemoteRequestPreview, Scene,
        SemanticSegment, TranscriptSegment,
    },
    openrouter, slice2_storage, storage, RuntimeState,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tauri::State;
use uuid::Uuid;

const MAX_FRAMES: usize = 24;
const MAX_CONTEXTS: usize = 8;
const MAX_CONTEXT_CHARS: usize = 16_000;
const MAX_TRANSCRIPT: usize = 500;

fn frame_hash(source: &Path, timestamp: f64) -> Result<u64, String> {
    let output = Command::new("ffmpeg")
        .args([
            "-loglevel",
            "error",
            "-ss",
            &format!("{timestamp:.3}"),
            "-i",
        ])
        .arg(source)
        .args([
            "-frames:v",
            "1",
            "-vf",
            "scale=8:8,format=gray",
            "-f",
            "rawvideo",
            "-",
        ])
        .output()
        .map_err(|_| "ffmpeg was not found while sampling semantic frames".to_string())?;
    if !output.status.success() || output.stdout.len() < 64 {
        return Err("FFmpeg could not decode a semantic frame".into());
    }
    let pixels = &output.stdout[..64];
    let average = pixels.iter().map(|value| u64::from(*value)).sum::<u64>() / 64;
    Ok(pixels.iter().enumerate().fold(0, |hash, (index, value)| {
        if u64::from(*value) >= average {
            hash | (1_u64 << index)
        } else {
            hash
        }
    }))
}

fn render_frame(source: &Path, timestamp: f64, output: &Path) -> Result<(), String> {
    let result = Command::new("ffmpeg")
        .args([
            "-loglevel",
            "error",
            "-y",
            "-ss",
            &format!("{timestamp:.3}"),
            "-i",
        ])
        .arg(source)
        .args([
            "-frames:v",
            "1",
            "-vf",
            "scale=960:-2:force_original_aspect_ratio=decrease",
            "-q:v",
            "3",
        ])
        .arg(output)
        .output()
        .map_err(|_| "ffmpeg was not found while rendering semantic frames".to_string())?;
    if result.status.success() {
        Ok(())
    } else {
        Err("FFmpeg could not render a semantic frame".into())
    }
}

fn candidates(duration: f64, scenes: &[Scene]) -> Vec<(f64, String)> {
    let mut values = Vec::new();
    if scenes.is_empty() {
        values.push((0.0, "Source opening".into()));
        let mut time = 15.0;
        while time < duration {
            values.push((time, "Timed visual sample".into()));
            time += 15.0;
        }
    } else {
        for scene in scenes {
            values.push((
                (scene.start + 0.25).min(duration),
                format!("Visual change near {}", scene.label),
            ));
            if scene.end - scene.start >= 5.0 {
                values.push((
                    ((scene.start + scene.end) / 2.0).min(duration),
                    format!("Representative frame inside {}", scene.label),
                ));
            }
        }
    }
    values.sort_by(|left, right| left.0.total_cmp(&right.0));
    values.dedup_by(|left, right| (left.0 - right.0).abs() < 0.2);
    if values.len() <= MAX_FRAMES {
        return values;
    }
    (0..MAX_FRAMES)
        .map(|index| values[index * (values.len() - 1) / (MAX_FRAMES - 1)].clone())
        .collect()
}

fn sample_frames(
    project_path: &Path,
    asset: &crate::models::MediaAsset,
    scenes: &[Scene],
) -> Result<Vec<AnalysisFrame>, String> {
    if media::media_kind(Path::new(&asset.managed_path)) == MediaKind::Audio {
        return Ok(Vec::new());
    }
    let directory = project_path.join("Cache").join("analysis").join(&asset.id);
    if directory.exists() {
        fs::remove_dir_all(&directory).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let mut hashes = Vec::new();
    let mut frames = Vec::new();
    for (timestamp, reason) in candidates(asset.duration.max(0.0), scenes) {
        let Ok(hash) = frame_hash(Path::new(&asset.managed_path), timestamp) else {
            continue;
        };
        if hashes
            .iter()
            .any(|previous: &u64| (previous ^ hash).count_ones() <= 7)
        {
            continue;
        }
        let id = Uuid::new_v4().to_string();
        let path = directory.join(format!("{id}.jpg"));
        if render_frame(Path::new(&asset.managed_path), timestamp, &path).is_err() {
            continue;
        }
        hashes.push(hash);
        frames.push(AnalysisFrame {
            id,
            asset_id: asset.id.clone(),
            timestamp,
            path: path.to_string_lossy().into_owned(),
            reason,
            perceptual_hash: format!("{hash:016x}"),
        });
    }
    Ok(frames)
}

fn transcript_excerpt(
    asset_id: &str,
    start: f64,
    end: f64,
    transcript: &[TranscriptSegment],
) -> String {
    transcript
        .iter()
        .filter(|value| value.asset_id == asset_id && value.end >= start && value.start <= end)
        .map(|value| value.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

fn local_segments(
    asset: &crate::models::MediaAsset,
    scenes: &[Scene],
    transcript: &[TranscriptSegment],
    contexts: &[ProjectContext],
) -> Vec<SemanticSegment> {
    let mut ranges = scenes
        .iter()
        .map(|scene| (scene.start, scene.end))
        .collect::<Vec<_>>();
    if ranges.is_empty() {
        ranges = transcript
            .iter()
            .filter(|value| value.asset_id == asset.id)
            .map(|value| (value.start, value.end))
            .collect();
    }
    if ranges.is_empty() {
        ranges.push((0.0, asset.duration.max(0.0)));
    }
    let context_names = contexts
        .iter()
        .take(4)
        .map(|value| value.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    ranges.into_iter().enumerate().map(|(index,(start,end))| {
        let excerpt = transcript_excerpt(&asset.id,start,end,transcript);
        let has_transcript = !excerpt.is_empty();
        let words = excerpt.split_whitespace().take(8).collect::<Vec<_>>();
        SemanticSegment {
            id: Uuid::new_v4().to_string(),asset_id: asset.id.clone(),start,end,
            title: if words.is_empty() { format!("Visual section {}",index+1) } else { words.join(" ") },
            description: if context_names.is_empty() {
                format!("Locally detected source section from {:.1}s to {:.1}s.",start,end)
            } else {
                format!("Locally detected source section from {:.1}s to {:.1}s. Project context available: {context_names}.",start,end)
            },
            transcript_excerpt: excerpt,
            visual_observations: vec!["Representative frames were selected locally around visual changes.".into()],
            importance: if index == 0 || has_transcript { "high" } else { "medium" }.into(),
            suggested_decision: if end-start > 35.0 && !has_transcript { "shorten" } else { "keep" }.into(),
            confidence: if has_transcript { 0.58 } else { 0.38 },provenance: "local".into(),
        }
    }).collect()
}

pub fn rebuild_local_semantics(
    project_path: &Path,
    asset_ids: &[String],
) -> Result<Vec<SemanticSegment>, String> {
    if asset_ids.is_empty() {
        return Err("Select at least one source".into());
    }
    let all_scenes = storage::list_scenes(project_path)?;
    let transcript = storage::list_transcript(project_path)?;
    let contexts = storage::list_contexts(project_path)?;
    let mut result = Vec::new();
    for asset_id in asset_ids {
        let asset = storage::find_asset(project_path, asset_id)?;
        let scenes = all_scenes
            .iter()
            .filter(|value| value.asset_id == asset_id.as_str())
            .cloned()
            .collect::<Vec<_>>();
        let frames = sample_frames(project_path, &asset, &scenes)?;
        let segments = local_segments(&asset, &scenes, &transcript, &contexts);
        slice2_storage::replace_local_analysis(project_path, asset_id, &frames, &segments)?;
        result.extend(segments);
    }
    storage::touch_project(project_path)?;
    Ok(result)
}

fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.into();
    }
    format!(
        "{}\n[truncated for this remote request]",
        value.chars().take(max).collect::<String>()
    )
}

pub fn prepare_preview(
    project_path: &Path,
    kind: &str,
    asset_ids: Vec<String>,
    prompt: String,
    conversation_id: String,
    decision_id: String,
    model: String,
) -> Result<RemoteRequestPreview, String> {
    rebuild_local_semantics(project_path, &asset_ids)?;
    let sources = asset_ids
        .iter()
        .map(|id| {
            let asset = storage::find_asset(project_path, id)?;
            Ok(DisclosedSource {
                asset_id: asset.id,
                name: asset.name,
                duration: asset.duration,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let frames = slice2_storage::list_analysis_frames(project_path)?
        .into_iter()
        .filter(|value| asset_ids.contains(&value.asset_id))
        .collect::<Vec<_>>();
    let semantic_segments = if kind == "semantic" {
        Vec::new()
    } else {
        slice2_storage::list_semantic_segments(project_path)?
            .into_iter()
            .filter(|value| asset_ids.contains(&value.asset_id))
            .collect()
    };
    let transcript = storage::list_transcript(project_path)?
        .into_iter()
        .filter(|value| asset_ids.contains(&value.asset_id))
        .take(MAX_TRANSCRIPT)
        .collect::<Vec<_>>();
    let contexts = storage::list_contexts(project_path)?
        .into_iter()
        .take(MAX_CONTEXTS)
        .map(|mut value| {
            value.content = truncate(&value.content, MAX_CONTEXT_CHARS);
            value
        })
        .collect::<Vec<_>>();
    let frame_bytes = frames
        .iter()
        .map(|value| fs::metadata(&value.path).map(|m| m.len()).unwrap_or(0))
        .sum::<u64>();
    let text_bytes = serde_json::to_vec(&(
        &sources,
        &semantic_segments,
        &transcript,
        &contexts,
        &prompt,
    ))
    .map_err(|error| error.to_string())?
    .len() as u64;
    let preview = RemoteRequestPreview {
        id: Uuid::new_v4().to_string(),
        kind: kind.into(),
        model,
        asset_ids,
        sources,
        frames,
        semantic_segments,
        transcript,
        contexts,
        prompt,
        conversation_id,
        decision_id,
        estimated_bytes: frame_bytes + text_bytes,
        first_approval_required: kind == "semantic"
            && !slice2_storage::remote_analysis_approved(project_path)?,
        created_at: Utc::now().to_rfc3339(),
    };
    slice2_storage::save_preview(project_path, &preview)?;
    Ok(preview)
}

#[tauri::command]
pub fn build_local_semantic_map(
    project_path: String,
    asset_ids: Vec<String>,
) -> Result<Vec<SemanticSegment>, String> {
    rebuild_local_semantics(Path::new(&project_path), &asset_ids)
}

#[tauri::command]
pub fn prepare_semantic_analysis(
    project_path: String,
    asset_ids: Vec<String>,
    state: State<'_, RuntimeState>,
) -> Result<RemoteRequestPreview, String> {
    let model = state
        .data
        .lock()
        .map_err(|_| "Settings lock was poisoned".to_string())?
        .settings
        .openrouter_model
        .clone();
    prepare_preview(
        Path::new(&project_path),
        "semantic",
        asset_ids,
        "Understand these sources and produce a semantic source map.".into(),
        String::new(),
        String::new(),
        model,
    )
}

pub(crate) fn frame_data_url(frame: &AnalysisFrame) -> Result<String, String> {
    let bytes = fs::read(&frame.path)
        .map_err(|error| format!("Could not read disclosed frame {}: {error}", frame.path))?;
    Ok(format!("data:image/jpeg;base64,{}", BASE64.encode(bytes)))
}

fn messages(preview: &RemoteRequestPreview) -> Result<Value, String> {
    let manifest = preview.frames.iter().map(|frame| json!({
        "frameId": frame.id,"assetId": frame.asset_id,"timestamp": frame.timestamp,"reason": frame.reason
    })).collect::<Vec<_>>();
    let mut content = vec![
        json!({"type":"text","text":serde_json::to_string_pretty(&json!({
        "task":"Create a truthful semantic map. Context is background, not proof of what is visible.",
        "sources":preview.sources,"frames":manifest,"transcript":preview.transcript,
        "projectContext":preview.contexts
    })).map_err(|error|error.to_string())?}),
    ];
    for frame in &preview.frames {
        content.push(json!({"type":"text","text":format!(
            "Frame {} · asset {} · {:.3}s · {}",frame.id,frame.asset_id,frame.timestamp,frame.reason
        )}));
        content.push(
            json!({"type":"image_url","image_url":{"url":frame_data_url(frame)?,"detail":"low"}}),
        );
    }
    Ok(json!([
        {"role":"system","content":"Describe only supported visual or transcript evidence. Flag uncertainty. Never claim an edit was performed."},
        {"role":"user","content":content}
    ]))
}

fn schema() -> Value {
    json!({"type":"object","additionalProperties":false,"properties":{"segments":{
        "type":"array","minItems":1,"items":{"type":"object","additionalProperties":false,
        "properties":{
            "assetId":{"type":"string"},"start":{"type":"number","minimum":0},
            "end":{"type":"number","minimum":0},"title":{"type":"string","minLength":1,"maxLength":100},
            "description":{"type":"string","minLength":1,"maxLength":700},
            "transcriptExcerpt":{"type":"string","maxLength":900},
            "visualObservations":{"type":"array","maxItems":8,"items":{"type":"string","maxLength":300}},
            "importance":{"type":"string","enum":["low","medium","high"]},
            "suggestedDecision":{"type":"string","enum":["keep","remove","shorten"]},
            "confidence":{"type":"number","minimum":0,"maximum":1}},
        "required":["assetId","start","end","title","description","transcriptExcerpt",
            "visualObservations","importance","suggestedDecision","confidence"]}}},
        "required":["segments"]})
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RemoteOutput {
    segments: Vec<RemoteSegment>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RemoteSegment {
    asset_id: String,
    start: f64,
    end: f64,
    title: String,
    description: String,
    transcript_excerpt: String,
    visual_observations: Vec<String>,
    importance: String,
    suggested_decision: String,
    confidence: f64,
}

fn validate(
    project_path: &Path,
    preview: &RemoteRequestPreview,
    output: RemoteOutput,
) -> Result<Vec<SemanticSegment>, String> {
    if output.segments.is_empty() {
        return Err("The model returned an empty semantic map".into());
    }
    output
        .segments
        .into_iter()
        .map(|value| {
            if !preview.asset_ids.contains(&value.asset_id) {
                return Err("The model referenced a source that was not disclosed".into());
            }
            let asset = storage::find_asset(project_path, &value.asset_id)?;
            if !value.start.is_finite()
                || !value.end.is_finite()
                || value.start < 0.0
                || value.end < value.start
                || value.end > asset.duration + 0.25
                || !(0.0..=1.0).contains(&value.confidence)
                || !["low", "medium", "high"].contains(&value.importance.as_str())
                || !["keep", "remove", "shorten"].contains(&value.suggested_decision.as_str())
            {
                return Err(format!(
                    "The model returned invalid semantic data for {}",
                    asset.name
                ));
            }
            Ok(SemanticSegment {
                id: Uuid::new_v4().to_string(),
                asset_id: value.asset_id,
                start: value.start,
                end: value.end,
                title: value.title,
                description: value.description,
                transcript_excerpt: value.transcript_excerpt,
                visual_observations: value.visual_observations,
                importance: value.importance,
                suggested_decision: value.suggested_decision,
                confidence: value.confidence,
                provenance: "both".into(),
            })
        })
        .collect()
}

#[tauri::command]
pub fn run_semantic_analysis(
    project_path: String,
    preview_id: String,
    approve_first: bool,
) -> Result<Vec<SemanticSegment>, String> {
    let project_path = PathBuf::from(project_path);
    let preview = slice2_storage::load_preview(&project_path, &preview_id)?;
    if preview.kind != "semantic" {
        return Err("This disclosure is not a semantic-analysis request".into());
    }
    if preview.first_approval_required && !slice2_storage::remote_analysis_approved(&project_path)?
    {
        if !approve_first {
            return Err("Approve the first remote semantic analysis before continuing".into());
        }
        slice2_storage::approve_remote_analysis(&project_path, &Utc::now().to_rfc3339())?;
    }
    openrouter::validate_model_capabilities(&preview.model, !preview.frames.is_empty())?;
    slice2_storage::mark_preview_status(&project_path, &preview.id, "sending")?;
    let response = openrouter::stream_structured_response(
        &preview.model,
        messages(&preview)?,
        "edentic_semantic_map",
        schema(),
        || false,
        |_| {},
    )
    .map_err(|error| {
        let _ = slice2_storage::mark_preview_status(&project_path, &preview.id, "failed");
        error
    })?;
    let output: RemoteOutput = serde_json::from_str(response.trim())
        .map_err(|error| format!("The model response failed strict validation: {error}"))?;
    let segments = validate(&project_path, &preview, output)?;
    slice2_storage::replace_remote_segments(&project_path, &preview.asset_ids, &segments)?;
    slice2_storage::mark_preview_status(&project_path, &preview.id, "completed")?;
    storage::touch_project(&project_path)?;
    Ok(segments)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn candidates_span_long_sources() {
        let scenes = (0..40)
            .map(|index| Scene {
                id: index.to_string(),
                asset_id: "a".into(),
                start: f64::from(index) * 10.0,
                end: f64::from(index + 1) * 10.0,
                label: index.to_string(),
                thumbnail_path: String::new(),
            })
            .collect::<Vec<_>>();
        let values = candidates(400.0, &scenes);
        assert_eq!(values.len(), MAX_FRAMES);
        assert!(values.first().unwrap().0 < 1.0 && values.last().unwrap().0 > 380.0);
    }
    #[test]
    fn rejects_extra_fields() {
        let value = r#"{"segments":[{"assetId":"a","start":0,"end":1,"title":"t","description":"d","transcriptExcerpt":"","visualObservations":[],"importance":"high","suggestedDecision":"keep","confidence":0.8,"edited":true}]}"#;
        assert!(serde_json::from_str::<RemoteOutput>(value).is_err());
    }
}
