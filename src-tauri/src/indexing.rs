use crate::{
    models::{AppSettings, IndexJob, MediaAsset, Scene, TranscriptSegment},
    storage,
    RuntimeState,
};
use chrono::Utc;
use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::{Arc, Mutex},
};
use tauri::State;
use uuid::Uuid;

fn compact_process_error(label: &str, stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    let mut lines = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| {
            !line.starts_with("ffmpeg version")
                && !line.starts_with("configuration:")
                && !line.starts_with("libav")
                && !line.starts_with("built on")
        })
        .rev()
        .take(5)
        .collect::<Vec<_>>();
    lines.reverse();
    let mut detail = lines.join(" · ");
    if detail.is_empty() {
        detail = "FFmpeg returned an unknown error".into();
    }
    if detail.chars().count() > 420 {
        detail = detail.chars().take(417).collect::<String>() + "…";
    }
    format!("{label}: {detail}")
}

fn even(value: u32) -> u32 {
    let value = value.max(2);
    if value % 2 == 0 { value } else { value + 1 }
}

fn scaled_dimensions(asset: &MediaAsset, max_width: u32) -> (u32, u32) {
    let source_width = asset.width.max(2);
    let source_height = asset.height.max(2);
    let width = even(source_width.min(max_width.max(2)));
    let height = even(((f64::from(width) * f64::from(source_height)) / f64::from(source_width)).round() as u32);
    (width, height)
}

fn run_ffmpeg(command: &mut Command, label: &str) -> Result<Output, String> {
    let output = command.output().map_err(|_| {
        "ffmpeg was not found. Install a current FFmpeg build and restart Edentic.".to_string()
    })?;
    if output.status.success() {
        Ok(output)
    } else {
        Err(compact_process_error(label, &output.stderr))
    }
}

fn create_poster(asset: &MediaAsset, project_path: &Path) -> Result<PathBuf, String> {
    let output_path = project_path.join("Cache/posters").join(format!("{}.jpg", asset.id));
    let seek = if asset.duration > 4.0 { "2" } else { "0" };
    let (width, height) = scaled_dimensions(asset, 960);
    let scale = format!("scale={width}:{height}");
    let mut command = Command::new("ffmpeg");
    command
        .args(["-hide_banner", "-loglevel", "error", "-y", "-ss", seek, "-i"])
        .arg(&asset.managed_path)
        .args(["-frames:v", "1", "-vf", &scale, "-q:v", "3"])
        .arg(&output_path);
    run_ffmpeg(&mut command, "Poster generation failed")?;
    Ok(output_path)
}

fn create_waveform(asset: &MediaAsset, project_path: &Path) -> Result<PathBuf, String> {
    let output_path = project_path.join("Cache/waveforms").join(format!("{}.svg", asset.id));
    let mut command = Command::new("ffmpeg");
    command
        .args(["-hide_banner", "-loglevel", "error", "-i"])
        .arg(&asset.managed_path)
        .args(["-vn", "-ac", "1", "-ar", "2000", "-f", "s16le", "-"]);
    let output = run_ffmpeg(&mut command, "Waveform decoding failed")?;
    let samples = output
        .stdout
        .chunks_exact(2)
        .map(|bytes| i16::from_le_bytes([bytes[0], bytes[1]]))
        .collect::<Vec<_>>();
    if samples.is_empty() {
        return Err("Waveform decoding failed: this source has no usable audio".into());
    }

    let bucket_count = samples.len().min(600).max(1);
    let bucket_size = samples.len().div_ceil(bucket_count);
    let mut lines = String::with_capacity(bucket_count * 48);
    for (index, bucket) in samples.chunks(bucket_size).take(bucket_count).enumerate() {
        let peak = bucket
            .iter()
            .map(|sample| f64::from((*sample as i32).abs()) / f64::from(i16::MAX))
            .fold(0.0_f64, f64::max)
            .clamp(0.02, 1.0);
        let x = if bucket_count == 1 {
            600.0
        } else {
            (index as f64 / (bucket_count - 1) as f64) * 1200.0
        };
        let extent = peak * 70.0;
        lines.push_str(&format!(
            "<line x1=\"{x:.2}\" y1=\"{:.2}\" x2=\"{x:.2}\" y2=\"{:.2}\"/>",
            80.0 - extent,
            80.0 + extent
        ));
    }
    let svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 1200 160\" preserveAspectRatio=\"none\"><g stroke=\"#E5B63E\" stroke-width=\"1.5\" opacity=\"0.82\">{lines}</g></svg>"
    );
    fs::write(&output_path, svg).map_err(|error| format!("Could not save waveform: {error}"))?;
    Ok(output_path)
}

fn create_proxy(
    asset: &MediaAsset,
    project_path: &Path,
    settings: &AppSettings,
) -> Result<Option<PathBuf>, String> {
    let target_width = match settings.proxy_quality.as_str() {
        "performance" => 960,
        "quality" => 1920,
        _ => 1280,
    };
    let needs_proxy = asset.width > target_width || asset.video_codec != "h264";
    if !needs_proxy {
        return Ok(None);
    }

    let output_path = project_path.join("Proxies").join(format!("{}.mp4", asset.id));
    let (width, height) = scaled_dimensions(asset, target_width);
    let scale = format!("scale={width}:{height}");
    let use_gpu = settings.compute_mode != "cpu"
        && Command::new("nvidia-smi")
            .arg("-L")
            .output()
            .is_ok_and(|output| output.status.success());

    let run = |encoder: &str| {
        let mut command = Command::new("ffmpeg");
        command
            .args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
            .arg(&asset.managed_path)
            .args(["-vf", &scale, "-c:v", encoder, "-pix_fmt", "yuv420p"]);
        if encoder == "h264_nvenc" {
            command.args(["-preset", "p4", "-cq", "24"]);
        } else {
            command.args(["-preset", "veryfast", "-crf", "23"]);
        }
        command
            .args(["-c:a", "aac", "-b:a", "128k", "-movflags", "+faststart"])
            .arg(&output_path);
        run_ffmpeg(&mut command, "Proxy generation failed")
    };

    let result = if use_gpu {
        run("h264_nvenc").or_else(|_| {
            let _ = fs::remove_file(&output_path);
            run("libx264")
        })
    } else {
        run("libx264")
    };
    result?;
    Ok(Some(output_path))
}

fn scene_boundaries(asset: &MediaAsset) -> (Vec<f64>, bool) {
    let output = Command::new("ffmpeg")
        .args(["-hide_banner", "-i"])
        .arg(&asset.managed_path)
        .args(["-vf", "select=gt(scene\\,0.32),showinfo", "-an", "-f", "null", "-"])
        .output();
    let Ok(output) = output else {
        return (vec![0.0], false);
    };
    let log = String::from_utf8_lossy(&output.stderr);
    let mut values = vec![0.0];
    for line in log.lines() {
        let Some(position) = line.find("pts_time:") else { continue };
        let value = line[position + 9..]
            .split_whitespace()
            .next()
            .and_then(|value| value.parse::<f64>().ok());
        if let Some(value) = value {
            let sufficiently_far = values.last().is_none_or(|last| value - last > 0.75);
            if value > 0.5 && value < asset.duration && sufficiently_far {
                values.push(value);
            }
        }
    }
    let detected_visual_changes = values.len() > 1;
    if !detected_visual_changes && asset.duration > 35.0 {
        let mut point = 30.0;
        while point < asset.duration {
            values.push(point);
            point += 30.0;
        }
    }
    values.truncate(80);
    (values, detected_visual_changes)
}

fn create_scenes(asset: &MediaAsset, project_path: &Path) -> Vec<Scene> {
    let (boundaries, detected_visual_changes) = scene_boundaries(asset);
    let (width, height) = scaled_dimensions(asset, 420);
    let scale = format!("scale={width}:{height}");
    boundaries
        .iter()
        .enumerate()
        .map(|(index, start)| {
            let end = boundaries.get(index + 1).copied().unwrap_or(asset.duration);
            let thumbnail = project_path
                .join("Cache/scenes")
                .join(format!("{}-{index:03}.jpg", asset.id));
            let seek = (start + 0.25).min(asset.duration.max(0.0));
            let seek_value = format!("{seek:.3}");
            let mut command = Command::new("ffmpeg");
            command
                .args([
                    "-hide_banner",
                    "-loglevel",
                    "error",
                    "-y",
                    "-ss",
                    &seek_value,
                    "-i",
                ])
                .arg(&asset.managed_path)
                .args(["-frames:v", "1", "-vf", &scale, "-q:v", "4"])
                .arg(&thumbnail);
            let thumbnail_path = command
                .output()
                .ok()
                .filter(|output| output.status.success())
                .map(|_| thumbnail.to_string_lossy().into_owned())
                .unwrap_or_default();
            Scene {
                id: Uuid::new_v4().to_string(),
                asset_id: asset.id.clone(),
                start: *start,
                end,
                label: if detected_visual_changes {
                    format!("Scene {}", index + 1)
                } else {
                    format!("Timed section {}", index + 1)
                },
                thumbnail_path,
            }
        })
        .collect()
}

fn transcribe(asset: &MediaAsset, settings: &AppSettings) -> Result<Vec<TranscriptSegment>, String> {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../python/transcribe.py");
    if !script.exists() {
        return Err("The local transcription helper is missing".into());
    }
    let mut command = if cfg!(target_os = "windows") {
        let mut command = Command::new("py");
        command.arg("-3.12");
        command
    } else {
        Command::new("python3")
    };
    let output = command
        .arg(script)
        .arg(&asset.managed_path)
        .arg(&settings.whisper_model)
        .arg(&settings.compute_mode)
        .output()
        .map_err(|_| {
            "Python 3.12 was not found. Install the transcription requirements or continue without transcripts."
                .to_string()
        })?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if detail.is_empty() {
            "Local transcription was unavailable".into()
        } else {
            detail.chars().take(360).collect()
        });
    }
    #[derive(Deserialize)]
    struct RawSegment {
        start: f64,
        end: f64,
        text: String,
    }
    let raw: Vec<RawSegment> = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("Invalid transcription output: {error}"))?;
    Ok(raw
        .into_iter()
        .filter(|segment| !segment.text.trim().is_empty())
        .map(|segment| TranscriptSegment {
            id: Uuid::new_v4().to_string(),
            asset_id: asset.id.clone(),
            start: segment.start,
            end: segment.end,
            text: segment.text.trim().to_string(),
        })
        .collect())
}

fn build_index<F>(
    project_path: &Path,
    mut asset: MediaAsset,
    settings: &AppSettings,
    mut progress: F,
) -> Result<Option<String>, String>
where
    F: FnMut(f64, &str) -> bool,
{
    let mut warnings = Vec::new();

    if !progress(0.08, "Creating source poster") {
        return Err("Indexing cancelled".into());
    }
    match create_poster(&asset, project_path) {
        Ok(path) => asset.poster_path = path.to_string_lossy().into_owned(),
        Err(error) => warnings.push(error),
    }

    if !progress(0.20, "Creating playback proxy when needed") {
        return Err("Indexing cancelled".into());
    }
    match create_proxy(&asset, project_path, settings) {
        Ok(Some(path)) => asset.proxy_path = path.to_string_lossy().into_owned(),
        Ok(None) => asset.proxy_path.clear(),
        Err(error) => warnings.push(error),
    }

    if !progress(0.36, "Rendering audio waveform") {
        return Err("Indexing cancelled".into());
    }
    match create_waveform(&asset, project_path) {
        Ok(path) => asset.waveform_path = path.to_string_lossy().into_owned(),
        Err(error) => warnings.push(error),
    }

    if !progress(0.52, "Detecting scenes") {
        return Err("Indexing cancelled".into());
    }
    let scenes = create_scenes(&asset, project_path);

    if !progress(0.78, "Transcribing locally") {
        return Err("Indexing cancelled".into());
    }
    let transcript = match transcribe(&asset, settings) {
        Ok(transcript) => transcript,
        Err(error) => {
            warnings.push(format!("Transcript unavailable: {error}"));
            Vec::new()
        }
    };

    if !progress(0.94, "Saving Video Map") {
        return Err("Indexing cancelled".into());
    }
    let status = if warnings.is_empty() { "ready" } else { "partial" };
    storage::replace_index(project_path, &asset, &scenes, &transcript, status)?;
    Ok((!warnings.is_empty()).then(|| warnings.join(" · ")))
}

fn update_job(
    jobs: &Arc<Mutex<std::collections::HashMap<String, IndexJob>>>,
    job_id: &str,
    updater: impl FnOnce(&mut IndexJob),
) -> bool {
    let Ok(mut jobs) = jobs.lock() else { return false };
    let Some(job) = jobs.get_mut(job_id) else { return false };
    updater(job);
    job.status != "cancelled"
}

#[tauri::command]
pub fn start_index(
    project_path: String,
    asset_id: String,
    state: State<'_, RuntimeState>,
) -> Result<IndexJob, String> {
    let path = PathBuf::from(&project_path);
    let asset = storage::find_asset(&path, &asset_id)?;
    let settings = state
        .data
        .lock()
        .map_err(|_| "Settings lock was poisoned".to_string())?
        .settings
        .clone();
    let job = IndexJob {
        id: Uuid::new_v4().to_string(),
        project_path: project_path.clone(),
        asset_id: asset_id.clone(),
        status: "queued".into(),
        progress: 0.0,
        stage: "Waiting to index".into(),
        error: String::new(),
        started_at: Utc::now().to_rfc3339(),
        finished_at: None,
    };
    state
        .jobs
        .lock()
        .map_err(|_| "Job lock was poisoned".to_string())?
        .insert(job.id.clone(), job.clone());

    let jobs = Arc::clone(&state.jobs);
    let job_id = job.id.clone();
    std::thread::spawn(move || {
        loop {
            let claimed = {
                let Ok(mut all_jobs) = jobs.lock() else { return };
                let cancelled = all_jobs
                    .get(&job_id)
                    .is_none_or(|job| job.status == "cancelled");
                if cancelled {
                    return;
                }
                let active = all_jobs.values().filter(|job| job.status == "running").count();
                if active < usize::from(settings.max_concurrent_jobs) {
                    if let Some(job) = all_jobs.get_mut(&job_id) {
                        job.status = "running".into();
                        job.stage = "Preparing source".into();
                    }
                    true
                } else {
                    false
                }
            };
            if claimed {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(180));
        }

        let result = build_index(&path, asset, &settings, |progress, stage| {
            update_job(&jobs, &job_id, |job| {
                job.progress = progress;
                job.stage = stage.into();
            })
        });
        match result {
            Ok(warning) => {
                update_job(&jobs, &job_id, |job| {
                    job.status = "completed".into();
                    job.progress = 1.0;
                    job.stage = if warning.is_some() {
                        "Video Map ready with warnings".into()
                    } else {
                        "Video Map ready".into()
                    };
                    job.error = warning.unwrap_or_default();
                    job.finished_at = Some(Utc::now().to_rfc3339());
                });
            }
            Err(error) => {
                update_job(&jobs, &job_id, |job| {
                    if job.status != "cancelled" {
                        job.status = "failed".into();
                    }
                    job.stage = "Indexing stopped".into();
                    job.error = error;
                    job.finished_at = Some(Utc::now().to_rfc3339());
                });
            }
        }
    });
    Ok(job)
}

#[tauri::command]
pub fn get_index_jobs(
    project_path: String,
    state: State<'_, RuntimeState>,
) -> Result<Vec<IndexJob>, String> {
    let mut jobs = state
        .jobs
        .lock()
        .map_err(|_| "Job lock was poisoned".to_string())?
        .values()
        .filter(|job| job.project_path == project_path)
        .cloned()
        .collect::<Vec<_>>();
    jobs.sort_by(|left, right| left.started_at.cmp(&right.started_at));
    Ok(jobs)
}

#[tauri::command]
pub fn cancel_index_job(job_id: String, state: State<'_, RuntimeState>) -> Result<bool, String> {
    let mut jobs = state.jobs.lock().map_err(|_| "Job lock was poisoned".to_string())?;
    let job = jobs.get_mut(&job_id).ok_or_else(|| "Index job not found".to_string())?;
    if ["queued", "running"].contains(&job.status.as_str()) {
        job.status = "cancelled".into();
        job.stage = "Cancelling".into();
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(width: u32, height: u32) -> MediaAsset {
        MediaAsset {
            id: "asset".into(),
            project_id: "project".into(),
            name: "source.mp4".into(),
            original_path: String::new(),
            managed_path: String::new(),
            duration: 10.0,
            width,
            height,
            frame_rate: 30.0,
            size_bytes: 0,
            video_codec: "h264".into(),
            audio_codec: "aac".into(),
            proxy_path: String::new(),
            poster_path: String::new(),
            waveform_path: String::new(),
            index_status: "waiting".into(),
        }
    }

    #[test]
    fn portrait_dimensions_are_positive_and_even() {
        assert_eq!(scaled_dimensions(&asset(918, 1140), 960), (918, 1140));
        assert_eq!(scaled_dimensions(&asset(1081, 1921), 420), (420, 746));
    }

    #[test]
    fn ffmpeg_errors_are_compact() {
        let error = compact_process_error(
            "Poster generation failed",
            b"ffmpeg version old\nconfiguration: long\nFailed to configure output pad\nError opening filters!",
        );
        assert!(error.contains("Failed to configure output pad"));
        assert!(!error.contains("configuration:"));
    }
}
