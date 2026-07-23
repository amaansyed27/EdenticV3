use crate::{
    models::{AppSettings, HardwareDiagnostics, MediaAsset, Scene, TranscriptSegment},
    storage::{self, NativeResult},
};
use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct ProbePayload {
    #[serde(default)]
    streams: Vec<ProbeStream>,
    #[serde(default)]
    format: ProbeFormat,
}

#[derive(Debug, Default, Deserialize)]
struct ProbeFormat {
    duration: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProbeStream {
    codec_type: Option<String>,
    codec_name: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    r_frame_rate: Option<String>,
    duration: Option<String>,
}

#[derive(Debug)]
pub struct IndexResult {
    pub asset: MediaAsset,
    pub scenes: Vec<Scene>,
    pub transcript: Vec<TranscriptSegment>,
    pub warning: Option<String>,
}

fn command_version(program: &str) -> String {
    Command::new(program)
        .arg("-version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .and_then(|output| output.lines().next().map(str::to_string))
        .unwrap_or_else(|| "Not found".into())
}

fn python_version() -> String {
    #[cfg(target_os = "windows")]
    let result = Command::new("py").args(["-3.12", "--version"]).output();
    #[cfg(not(target_os = "windows"))]
    let result = Command::new("python3").arg("--version").output();
    result
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| {
            let value = if output.stdout.is_empty() { output.stderr } else { output.stdout };
            String::from_utf8(value).ok()
        })
        .map(|output| output.trim().to_string())
        .unwrap_or_else(|| "Not found".into())
}

fn gpu_name() -> String {
    let output = Command::new("nvidia-smi")
        .args(["--query-gpu=name", "--format=csv,noheader"])
        .output();
    output
        .ok()
        .filter(|result| result.status.success())
        .and_then(|result| String::from_utf8(result.stdout).ok())
        .and_then(|result| result.lines().next().map(str::trim).map(str::to_string))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "No supported GPU detected".into())
}

pub fn hardware_diagnostics() -> HardwareDiagnostics {
    HardwareDiagnostics {
        gpu_name: gpu_name(),
        ffmpeg_version: command_version("ffmpeg"),
        ffprobe_version: command_version("ffprobe"),
        python_version: python_version(),
    }
}

fn parse_rate(value: Option<&str>) -> f64 {
    let Some(value) = value else { return 0.0 };
    let mut pieces = value.split('/');
    let numerator = pieces.next().and_then(|part| part.parse::<f64>().ok()).unwrap_or(0.0);
    let denominator = pieces.next().and_then(|part| part.parse::<f64>().ok()).unwrap_or(1.0);
    if denominator == 0.0 { 0.0 } else { numerator / denominator }
}

pub fn probe_media(path: &Path, project_id: &str, original_path: &Path) -> NativeResult<MediaAsset> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration:stream=codec_type,codec_name,width,height,r_frame_rate,duration",
            "-of",
            "json",
        ])
        .arg(path)
        .output()
        .map_err(|_| "ffprobe was not found. Install a current FFmpeg build and restart Edentic.".to_string())?;
    if !output.status.success() {
        return Err(format!(
            "ffprobe could not read {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let payload: ProbePayload =
        serde_json::from_slice(&output.stdout).map_err(|error| format!("Invalid ffprobe response: {error}"))?;
    let video = payload.streams.iter().find(|stream| stream.codec_type.as_deref() == Some("video"));
    let audio = payload.streams.iter().find(|stream| stream.codec_type.as_deref() == Some("audio"));
    let video = video.ok_or_else(|| "The selected file does not contain a video stream".to_string())?;
    let duration = payload
        .format
        .duration
        .as_deref()
        .or(video.duration.as_deref())
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or_default();
    let metadata = fs::metadata(path).map_err(|error| error.to_string())?;
    Ok(MediaAsset {
        id: Uuid::new_v4().to_string(),
        project_id: project_id.to_string(),
        name: path.file_name().unwrap_or_default().to_string_lossy().into_owned(),
        original_path: original_path.to_string_lossy().into_owned(),
        managed_path: path.to_string_lossy().into_owned(),
        duration,
        width: video.width.unwrap_or_default(),
        height: video.height.unwrap_or_default(),
        frame_rate: parse_rate(video.r_frame_rate.as_deref()),
        size_bytes: metadata.len(),
        video_codec: video.codec_name.clone().unwrap_or_else(|| "unknown".into()),
        audio_codec: audio
            .and_then(|stream| stream.codec_name.clone())
            .unwrap_or_else(|| "none".into()),
        proxy_path: String::new(),
        poster_path: String::new(),
        waveform_path: String::new(),
        index_status: "waiting".into(),
    })
}

pub fn unique_destination(directory: &Path, file_name: &str) -> PathBuf {
    let initial = directory.join(file_name);
    if !initial.exists() {
        return initial;
    }
    let source = Path::new(file_name);
    let stem = source.file_stem().unwrap_or_default().to_string_lossy();
    let extension = source.extension().map(|value| value.to_string_lossy().into_owned());
    for index in 2..10_000 {
        let candidate_name = match &extension {
            Some(extension) => format!("{stem} ({index}).{extension}"),
            None => format!("{stem} ({index})"),
        };
        let candidate = directory.join(candidate_name);
        if !candidate.exists() {
            return candidate;
        }
    }
    directory.join(format!("{}-{}", Uuid::new_v4(), file_name))
}

fn create_poster(asset: &MediaAsset, project_path: &Path) -> NativeResult<PathBuf> {
    let output = project_path.join("Cache/posters").join(format!("{}.jpg", asset.id));
    let seek = if asset.duration > 4.0 { "2" } else { "0" };
    let result = Command::new("ffmpeg")
        .args(["-y", "-ss", seek, "-i"])
        .arg(&asset.managed_path)
        .args(["-frames:v", "1", "-vf", "scale=960:-2"])
        .arg(&output)
        .output()
        .map_err(|_| "ffmpeg was not found".to_string())?;
    if !result.status.success() {
        return Err(format!("Poster generation failed: {}", String::from_utf8_lossy(&result.stderr)));
    }
    Ok(output)
}

fn create_waveform(asset: &MediaAsset, project_path: &Path) -> NativeResult<PathBuf> {
    let output = project_path.join("Cache/waveforms").join(format!("{}.png", asset.id));
    let result = Command::new("ffmpeg")
        .args(["-y", "-i"])
        .arg(&asset.managed_path)
        .args([
            "-filter_complex",
            "aformat=channel_layouts=mono,showwavespic=s=1200x160:colors=E5B63E",
            "-frames:v",
            "1",
        ])
        .arg(&output)
        .output()
        .map_err(|_| "ffmpeg was not found".to_string())?;
    if !result.status.success() {
        return Err("This source has no usable audio waveform".into());
    }
    Ok(output)
}

fn create_proxy(asset: &MediaAsset, project_path: &Path, settings: &AppSettings) -> NativeResult<Option<PathBuf>> {
    let target_width = match settings.proxy_quality.as_str() {
        "performance" => 960,
        "quality" => 1920,
        _ => 1280,
    };
    let needs_proxy = asset.width > target_width || !["h264", "hevc", "vp9"].contains(&asset.video_codec.as_str());
    if !needs_proxy {
        return Ok(None);
    }
    let output = project_path.join("Proxies").join(format!("{}.mp4", asset.id));
    let use_gpu = settings.compute_mode != "cpu"
        && Command::new("nvidia-smi").arg("-L").output().is_ok();
    let run = |encoder: &str| {
        let mut command = Command::new("ffmpeg");
        command
            .args(["-y", "-i"])
            .arg(&asset.managed_path)
            .args(["-vf", &format!("scale={target_width}:-2"), "-c:v", encoder]);
        if encoder == "h264_nvenc" {
            command.args(["-preset", "p4", "-cq", "24"]);
        } else {
            command.args(["-preset", "veryfast", "-crf", "23"]);
        }
        command
            .args(["-c:a", "aac", "-b:a", "128k", "-movflags", "+faststart"])
            .arg(&output)
            .output()
    };
    let mut result = if use_gpu { run("h264_nvenc") } else { run("libx264") }
        .map_err(|_| "ffmpeg was not found".to_string())?;
    if !result.status.success() && use_gpu {
        result = run("libx264").map_err(|_| "ffmpeg was not found".to_string())?;
    }
    if !result.status.success() {
        return Err(format!("Proxy generation failed: {}", String::from_utf8_lossy(&result.stderr)));
    }
    Ok(Some(output))
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
            if value > 0.5 && value < asset.duration && values.last().is_none_or(|last| value - last > 0.75) {
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
    boundaries
        .iter()
        .enumerate()
        .map(|(index, start)| {
            let end = boundaries.get(index + 1).copied().unwrap_or(asset.duration);
            let thumbnail = project_path
                .join("Cache/scenes")
                .join(format!("{}-{index:03}.jpg", asset.id));
            let seek = (start + 0.25).min(asset.duration.max(0.0));
            let result = Command::new("ffmpeg")
                .args(["-y", "-ss", &format!("{seek:.3}"), "-i"])
                .arg(&asset.managed_path)
                .args(["-frames:v", "1", "-vf", "scale=420:-2"])
                .arg(&thumbnail)
                .output();
            let thumbnail_path = result
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

fn transcribe(asset: &MediaAsset, settings: &AppSettings) -> NativeResult<Vec<TranscriptSegment>> {
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
        .map_err(|_| "Python 3.12 was not found. Install the transcription requirements or continue without transcripts.".to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    #[derive(Deserialize)]
    struct RawSegment {
        start: f64,
        end: f64,
        text: String,
    }
    let raw: Vec<RawSegment> =
        serde_json::from_slice(&output.stdout).map_err(|error| format!("Invalid transcription output: {error}"))?;
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

pub fn build_index<F>(
    project_path: &Path,
    mut asset: MediaAsset,
    settings: &AppSettings,
    mut progress: F,
) -> NativeResult<IndexResult>
where
    F: FnMut(f64, &str) -> bool,
{
    if !progress(0.08, "Creating source poster") {
        return Err("Indexing cancelled".into());
    }
    asset.poster_path = create_poster(&asset, project_path)?.to_string_lossy().into_owned();

    if !progress(0.20, "Creating playback proxy when needed") {
        return Err("Indexing cancelled".into());
    }
    asset.proxy_path = create_proxy(&asset, project_path, settings)?
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_default();

    if !progress(0.36, "Rendering audio waveform") {
        return Err("Indexing cancelled".into());
    }
    asset.waveform_path = create_waveform(&asset, project_path)
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_default();

    if !progress(0.52, "Detecting scenes") {
        return Err("Indexing cancelled".into());
    }
    let scenes = create_scenes(&asset, project_path);

    if !progress(0.78, "Transcribing locally") {
        return Err("Indexing cancelled".into());
    }
    let (transcript, warning) = match transcribe(&asset, settings) {
        Ok(transcript) => (transcript, None),
        Err(error) => (Vec::new(), Some(error)),
    };

    progress(0.94, "Saving Video Map");
    let status = if warning.is_some() { "partial" } else { "ready" };
    storage::replace_index(project_path, &asset, &scenes, &transcript, status)?;
    asset.index_status = status.into();
    Ok(IndexResult {
        asset,
        scenes,
        transcript,
        warning,
    })
}
