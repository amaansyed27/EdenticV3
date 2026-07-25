use crate::{
    models::{
        AiJob, AssistantConversation, AssistantMessage, EditPlan, PlanDecision,
        RemoteRequestPreview, TextOverlaySuggestion, VoiceoverSuggestion,
    },
    openrouter, semantic, slice2_storage, storage, RuntimeState,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tauri::State;
use uuid::Uuid;

fn title(prompt: &str) -> String {
    let words = prompt.split_whitespace().take(8).collect::<Vec<_>>();
    let value = words.join(" ");
    if prompt.split_whitespace().count() > words.len() {
        format!("{value}…")
    } else {
        value
    }
}

#[tauri::command]
pub fn prepare_assistant_request(
    project_path: String,
    prompt: String,
    asset_ids: Vec<String>,
    conversation_id: String,
    state: State<'_, RuntimeState>,
) -> Result<RemoteRequestPreview, String> {
    if prompt.trim().is_empty() {
        return Err("Describe the edit you want to plan".into());
    }
    let model = state
        .data
        .lock()
        .map_err(|_| "Settings lock was poisoned".to_string())?
        .settings
        .openrouter_model
        .clone();
    semantic::prepare_preview(
        Path::new(&project_path),
        "assistant",
        asset_ids,
        prompt.trim().into(),
        if conversation_id.is_empty() {
            Uuid::new_v4().to_string()
        } else {
            conversation_id
        },
        String::new(),
        model,
    )
}

#[tauri::command]
pub fn prepare_decision_regeneration(
    project_path: String,
    plan_id: String,
    decision_id: String,
    state: State<'_, RuntimeState>,
) -> Result<RemoteRequestPreview, String> {
    let path = Path::new(&project_path);
    let plan = slice2_storage::load_plan(path, &plan_id)?;
    let decision = plan
        .decisions
        .iter()
        .find(|value| value.id == decision_id)
        .ok_or_else(|| "The selected plan decision no longer exists".to_string())?;
    let model = state
        .data
        .lock()
        .map_err(|_| "Settings lock was poisoned".to_string())?
        .settings
        .openrouter_model
        .clone();
    semantic::prepare_preview(
        path,"decision",vec![decision.asset_id.clone()],
        format!("Regenerate only this edit-plan decision.\nOverall goal: {}\nCurrent decision: {} ({:.3}s to {:.3}s)\nCurrent reason: {}",
            plan.prompt,decision.title,decision.start,decision.end,decision.reason),
        plan_id,decision_id,model,
    )
}

fn messages(preview: &RemoteRequestPreview) -> Result<Value, String> {
    let mut content = vec![
        json!({"type":"text","text":serde_json::to_string_pretty(&json!({
        "editingGoal":preview.prompt,"sources":preview.sources,
        "semanticMap":preview.semantic_segments,"transcript":preview.transcript,
        "projectContext":preview.contexts
    })).map_err(|error|error.to_string())?}),
    ];
    for frame in &preview.frames {
        content.push(json!({"type":"text","text":format!(
            "Disclosed frame {} · asset {} · {:.3}s · {}",
            frame.id,frame.asset_id,frame.timestamp,frame.reason
        )}));
        content.push(json!({"type":"image_url","image_url":{
            "url":semantic::frame_data_url(frame)?,"detail":"low"
        }}));
    }
    Ok(json!([
        {"role":"system","content":"You are Edentic's assisted-edit planner. Create a proposed plan only; never claim media was edited. Every decision needs a source range and evidence-based reason. Use warnings for uncertainty."},
        {"role":"user","content":content}
    ]))
}

fn schema(single: bool) -> Value {
    json!({"type":"object","additionalProperties":false,"properties":{
        "assistantSummary":{"type":"string","minLength":1,"maxLength":1200},
        "decisions":{"type":"array","minItems":1,"maxItems":if single{1}else{80},
        "items":{"type":"object","additionalProperties":false,"properties":{
            "assetId":{"type":"string"},"start":{"type":"number","minimum":0},
            "end":{"type":"number","minimum":0},"title":{"type":"string","minLength":1,"maxLength":120},
            "action":{"type":"string","enum":["keep","remove","shorten"]},
            "reason":{"type":"string","minLength":1,"maxLength":700},
            "textOverlays":{"type":"array","maxItems":8,"items":{
                "type":"object","additionalProperties":false,"properties":{
                    "text":{"type":"string","minLength":1,"maxLength":240},
                    "placement":{"type":"string","maxLength":80},"start":{"type":"number","minimum":0},
                    "end":{"type":"number","minimum":0}},"required":["text","placement","start","end"]}},
            "voiceoverSections":{"type":"array","maxItems":8,"items":{
                "type":"object","additionalProperties":false,"properties":{
                    "text":{"type":"string","minLength":1,"maxLength":600},
                    "start":{"type":"number","minimum":0},"end":{"type":"number","minimum":0}},
                "required":["text","start","end"]}},
            "pacing":{"type":"string","minLength":1,"maxLength":240},
            "warnings":{"type":"array","maxItems":8,"items":{"type":"string","maxLength":300}},
            "confidence":{"type":"number","minimum":0,"maximum":1}},
        "required":["assetId","start","end","title","action","reason","textOverlays",
            "voiceoverSections","pacing","warnings","confidence"]}}},
        "required":["assistantSummary","decisions"]})
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RemotePlan {
    assistant_summary: String,
    decisions: Vec<RemoteDecision>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RemoteDecision {
    asset_id: String,
    start: f64,
    end: f64,
    title: String,
    action: String,
    reason: String,
    text_overlays: Vec<TextOverlaySuggestion>,
    voiceover_sections: Vec<VoiceoverSuggestion>,
    pacing: String,
    warnings: Vec<String>,
    confidence: f64,
}

fn subrange(start: f64, end: f64, parent_start: f64, parent_end: f64) -> bool {
    start.is_finite()
        && end.is_finite()
        && start >= parent_start
        && end >= start
        && end <= parent_end + 0.25
}

fn validate_remote(
    project_path: &Path,
    preview: &RemoteRequestPreview,
    decisions: Vec<RemoteDecision>,
) -> Result<Vec<PlanDecision>, String> {
    decisions
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            if !preview.asset_ids.contains(&value.asset_id) {
                return Err("The model planned against a source that was not disclosed".into());
            }
            let asset = storage::find_asset(project_path, &value.asset_id)?;
            if !subrange(value.start, value.end, 0.0, asset.duration)
                || !["keep", "remove", "shorten"].contains(&value.action.as_str())
                || !(0.0..=1.0).contains(&value.confidence)
            {
                return Err(format!(
                    "The model returned an invalid decision for {}",
                    asset.name
                ));
            }
            if value
                .text_overlays
                .iter()
                .any(|item| !subrange(item.start, item.end, value.start, value.end))
                || value
                    .voiceover_sections
                    .iter()
                    .any(|item| !subrange(item.start, item.end, value.start, value.end))
            {
                return Err(format!(
                    "The model returned an out-of-range suggestion for {}",
                    asset.name
                ));
            }
            Ok(PlanDecision {
                id: Uuid::new_v4().to_string(),
                asset_id: asset.id,
                source_name: asset.name,
                start: value.start,
                end: value.end,
                order_index: index as i64,
                enabled: true,
                title: value.title,
                action: value.action,
                reason: value.reason,
                text_overlays: value.text_overlays,
                voiceover_sections: value.voiceover_sections,
                pacing: value.pacing,
                warnings: value.warnings,
                confidence: value.confidence,
            })
        })
        .collect()
}

fn update_job(
    jobs: &Arc<Mutex<HashMap<String, AiJob>>>,
    id: &str,
    update: impl FnOnce(&mut AiJob),
) -> bool {
    let Ok(mut values) = jobs.lock() else {
        return false;
    };
    let Some(job) = values.get_mut(id) else {
        return false;
    };
    update(job);
    job.status != "cancelled"
}

fn finish_new(
    project_path: &Path,
    preview: &RemoteRequestPreview,
    output: RemotePlan,
) -> Result<String, String> {
    let now = Utc::now().to_rfc3339();
    let decisions = validate_remote(project_path, preview, output.decisions)?;
    let existing = slice2_storage::list_conversations(project_path)?
        .into_iter()
        .find(|value| value.id == preview.conversation_id);
    let conversation = AssistantConversation {
        id: preview.conversation_id.clone(),
        title: existing
            .as_ref()
            .map(|value| value.title.clone())
            .unwrap_or_else(|| title(&preview.prompt)),
        created_at: existing
            .as_ref()
            .map(|value| value.created_at.clone())
            .unwrap_or_else(|| now.clone()),
        updated_at: now.clone(),
    };
    slice2_storage::upsert_conversation(project_path, &conversation)?;
    slice2_storage::insert_message(
        project_path,
        &AssistantMessage {
            id: Uuid::new_v4().to_string(),
            conversation_id: conversation.id.clone(),
            role: "assistant".into(),
            content: format!(
                "I created a reviewable edit plan. No source media or timeline was changed.\n\n{}",
                output.assistant_summary
            ),
            created_at: now.clone(),
        },
    )?;
    let plan = EditPlan {
        id: Uuid::new_v4().to_string(),
        conversation_id: conversation.id,
        prompt: preview.prompt.clone(),
        assistant_summary: output.assistant_summary,
        status: "draft".into(),
        decisions,
        created_at: now.clone(),
        updated_at: now,
    };
    let id = plan.id.clone();
    slice2_storage::save_plan(project_path, &plan)?;
    Ok(id)
}

fn finish_decision(
    project_path: &Path,
    preview: &RemoteRequestPreview,
    output: RemotePlan,
) -> Result<String, String> {
    let mut plan = slice2_storage::load_plan(project_path, &preview.conversation_id)?;
    let mut replacements = validate_remote(project_path, preview, output.decisions)?;
    let replacement = replacements
        .pop()
        .ok_or_else(|| "The model did not return a replacement".to_string())?;
    let target = plan
        .decisions
        .iter_mut()
        .find(|value| value.id == preview.decision_id)
        .ok_or_else(|| "The original decision no longer exists".to_string())?;
    let id = target.id.clone();
    let order = target.order_index;
    let enabled = target.enabled;
    *target = PlanDecision {
        id,
        order_index: order,
        enabled,
        ..replacement
    };
    plan.updated_at = Utc::now().to_rfc3339();
    plan.status = "draft".into();
    let id = plan.id.clone();
    slice2_storage::save_plan(project_path, &plan)?;
    Ok(id)
}

fn run_job(
    project_path: PathBuf,
    preview: RemoteRequestPreview,
    jobs: Arc<Mutex<HashMap<String, AiJob>>>,
    job_id: String,
) {
    update_job(&jobs, &job_id, |job| {
        job.status = "running".into();
        job.stage = "Sending disclosed context to OpenRouter".into();
        job.progress = 0.08;
    });
    if let Err(error) =
        openrouter::validate_model_capabilities(&preview.model, !preview.frames.is_empty())
    {
        update_job(&jobs, &job_id, |job| {
            job.status = "failed".into();
            job.stage = "Model validation failed".into();
            job.error = error;
            job.finished_at = Some(Utc::now().to_rfc3339());
        });
        return;
    }
    let messages = match messages(&preview) {
        Ok(value) => value,
        Err(error) => {
            update_job(&jobs, &job_id, |job| {
                job.status = "failed".into();
                job.stage = "Could not prepare disclosed context".into();
                job.error = error;
                job.finished_at = Some(Utc::now().to_rfc3339());
            });
            return;
        }
    };
    let cancel_jobs = Arc::clone(&jobs);
    let cancel_id = job_id.clone();
    let chunk_jobs = Arc::clone(&jobs);
    let chunk_id = job_id.clone();
    let response = openrouter::stream_structured_response(
        &preview.model,
        messages,
        if preview.kind == "decision" {
            "edentic_plan_decision"
        } else {
            "edentic_edit_plan"
        },
        schema(preview.kind == "decision"),
        move || {
            cancel_jobs
                .lock()
                .ok()
                .and_then(|values| values.get(&cancel_id).map(|job| job.status == "cancelled"))
                .unwrap_or(true)
        },
        move |chunk| {
            update_job(&chunk_jobs, &chunk_id, |job| {
                job.received_chars += chunk.chars().count();
                job.stage = format!("Receiving plan · {} characters", job.received_chars);
                job.progress = (0.18 + job.received_chars as f64 / 18_000.0).min(0.86);
            });
        },
    );
    let response = match response {
        Ok(value) => value,
        Err(error) => {
            update_job(&jobs, &job_id, |job| {
                if job.status != "cancelled" {
                    job.status = "failed".into();
                    job.stage = "OpenRouter request failed".into();
                    job.error = error;
                } else {
                    job.stage = "Cancelled".into();
                }
                job.finished_at = Some(Utc::now().to_rfc3339());
            });
            return;
        }
    };
    update_job(&jobs, &job_id, |job| {
        job.stage = "Validating structured edit plan".into();
        job.progress = 0.92;
    });
    let output: RemotePlan = match serde_json::from_str(response.trim()) {
        Ok(value) => value,
        Err(error) => {
            update_job(&jobs, &job_id, |job| {
                job.status = "failed".into();
                job.stage = "Structured plan validation failed".into();
                job.error = format!("The model response failed strict validation: {error}");
                job.finished_at = Some(Utc::now().to_rfc3339());
            });
            return;
        }
    };
    let saved = if preview.kind == "decision" {
        finish_decision(&project_path, &preview, output)
    } else {
        finish_new(&project_path, &preview, output)
    };
    match saved {
        Ok(plan_id) => {
            let _ = slice2_storage::mark_preview_status(&project_path, &preview.id, "completed");
            let _ = storage::touch_project(&project_path);
            update_job(&jobs, &job_id, |job| {
                job.status = "completed".into();
                job.stage = "Reviewable plan saved".into();
                job.progress = 1.0;
                job.result_plan_id = plan_id;
                job.finished_at = Some(Utc::now().to_rfc3339());
            });
        }
        Err(error) => {
            let _ = slice2_storage::mark_preview_status(&project_path, &preview.id, "failed");
            update_job(&jobs, &job_id, |job| {
                job.status = "failed".into();
                job.stage = "Plan could not be saved".into();
                job.error = error;
                job.finished_at = Some(Utc::now().to_rfc3339());
            });
        }
    }
}

#[tauri::command]
pub fn start_assistant_request(
    project_path: String,
    preview_id: String,
    state: State<'_, RuntimeState>,
) -> Result<AiJob, String> {
    let path = PathBuf::from(&project_path);
    let preview = slice2_storage::load_preview(&path, &preview_id)?;
    if !["assistant", "decision"].contains(&preview.kind.as_str()) {
        return Err("This disclosure is not an assistant request".into());
    }
    if preview.kind == "assistant" {
        let now = Utc::now().to_rfc3339();
        let existing = slice2_storage::list_conversations(&path)?
            .into_iter()
            .find(|value| value.id == preview.conversation_id);
        let conversation = AssistantConversation {
            id: preview.conversation_id.clone(),
            title: existing
                .as_ref()
                .map(|value| value.title.clone())
                .unwrap_or_else(|| title(&preview.prompt)),
            created_at: existing
                .as_ref()
                .map(|value| value.created_at.clone())
                .unwrap_or_else(|| now.clone()),
            updated_at: now.clone(),
        };
        slice2_storage::upsert_conversation(&path, &conversation)?;
        slice2_storage::insert_message(
            &path,
            &AssistantMessage {
                id: Uuid::new_v4().to_string(),
                conversation_id: conversation.id,
                role: "user".into(),
                content: preview.prompt.clone(),
                created_at: now,
            },
        )?;
    }
    let job = slice2_storage::empty_ai_job(&project_path, &preview_id, &preview.kind);
    state
        .ai_jobs
        .lock()
        .map_err(|_| "Assistant job lock was poisoned".to_string())?
        .insert(job.id.clone(), job.clone());
    slice2_storage::mark_preview_status(&path, &preview.id, "sending")?;
    let jobs = Arc::clone(&state.ai_jobs);
    let id = job.id.clone();
    std::thread::spawn(move || run_job(path, preview, jobs, id));
    Ok(job)
}

#[tauri::command]
pub fn get_ai_jobs(
    project_path: String,
    state: State<'_, RuntimeState>,
) -> Result<Vec<AiJob>, String> {
    let mut jobs = state
        .ai_jobs
        .lock()
        .map_err(|_| "Assistant job lock was poisoned".to_string())?
        .values()
        .filter(|job| job.project_path == project_path)
        .cloned()
        .collect::<Vec<_>>();
    jobs.sort_by(|left, right| left.started_at.cmp(&right.started_at));
    Ok(jobs)
}

#[tauri::command]
pub fn cancel_ai_job(job_id: String, state: State<'_, RuntimeState>) -> Result<bool, String> {
    let mut jobs = state
        .ai_jobs
        .lock()
        .map_err(|_| "Assistant job lock was poisoned".to_string())?;
    let job = jobs
        .get_mut(&job_id)
        .ok_or_else(|| "Assistant request not found".to_string())?;
    if ["queued", "running"].contains(&job.status.as_str()) {
        job.status = "cancelled".into();
        job.stage = "Cancelling remote request".into();
    }
    Ok(true)
}

fn validate_saved(project_path: &Path, plan: &EditPlan) -> Result<(), String> {
    if !["draft", "accepted", "rejected"].contains(&plan.status.as_str())
        || plan.decisions.is_empty()
    {
        return Err("Invalid edit plan status or empty plan".into());
    }
    let mut ids = HashSet::new();
    let mut order = HashSet::new();
    for (index, value) in plan.decisions.iter().enumerate() {
        let asset = storage::find_asset(project_path, &value.asset_id)?;
        if !subrange(value.start, value.end, 0.0, asset.duration)
            || !["keep", "remove", "shorten"].contains(&value.action.as_str())
            || value.title.trim().is_empty()
            || value.reason.trim().is_empty()
            || value.pacing.trim().is_empty()
            || !value.confidence.is_finite()
            || !(0.0..=1.0).contains(&value.confidence)
            || value.order_index < 0
            || value.order_index >= plan.decisions.len() as i64
            || !ids.insert(value.id.as_str())
            || !order.insert(value.order_index)
        {
            return Err(format!(
                "Decision {} has invalid structured data",
                index + 1
            ));
        }
        if value.text_overlays.iter().any(|item| {
            item.text.trim().is_empty() || !subrange(item.start, item.end, value.start, value.end)
        }) || value.voiceover_sections.iter().any(|item| {
            item.text.trim().is_empty() || !subrange(item.start, item.end, value.start, value.end)
        }) {
            return Err(format!(
                "Decision {} has an out-of-range suggestion",
                index + 1
            ));
        }
    }
    Ok(())
}

#[tauri::command]
pub fn save_edit_plan(project_path: String, mut plan: EditPlan) -> Result<EditPlan, String> {
    let path = Path::new(&project_path);
    validate_saved(path, &plan)?;
    plan.updated_at = Utc::now().to_rfc3339();
    slice2_storage::save_plan(path, &plan)?;
    storage::touch_project(path)?;
    Ok(plan)
}

#[tauri::command]
pub fn set_edit_plan_status(
    project_path: String,
    plan_id: String,
    status: String,
) -> Result<EditPlan, String> {
    if !["accepted", "rejected"].contains(&status.as_str()) {
        return Err("A plan can only be accepted or rejected here".into());
    }
    let path = Path::new(&project_path);
    let mut plan = slice2_storage::load_plan(path, &plan_id)?;
    plan.status = status;
    plan.updated_at = Utc::now().to_rfc3339();
    validate_saved(path, &plan)?;
    slice2_storage::save_plan(path, &plan)?;
    storage::touch_project(path)?;
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strict_plan_rejects_extra_fields() {
        let value = r#"{"assistantSummary":"x","decisions":[{"assetId":"a","start":0,"end":1,"title":"x","action":"keep","reason":"r","textOverlays":[],"voiceoverSections":[],"pacing":"fast","warnings":[],"confidence":0.8,"edited":true}]}"#;
        assert!(serde_json::from_str::<RemotePlan>(value).is_err());
    }
    #[test]
    fn subranges_stay_inside_parent() {
        assert!(subrange(2.0, 4.0, 1.0, 5.0));
        assert!(!subrange(0.5, 4.0, 1.0, 5.0));
    }
}
