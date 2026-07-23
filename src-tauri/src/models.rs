use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub onboarding_complete: bool,
    pub projects_root: String,
    pub theme: String,
    pub compute_mode: String,
    pub proxy_quality: String,
    pub cache_limit_gb: u32,
    pub max_concurrent_jobs: u8,
    pub whisper_model: String,
    pub openrouter_configured: bool,
    pub openrouter_model: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        let root = dirs::video_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("Edentic Projects");
        Self {
            onboarding_complete: false,
            projects_root: root.to_string_lossy().into_owned(),
            theme: "dark".into(),
            compute_mode: "auto".into(),
            proxy_quality: "balanced".into(),
            cache_limit_gb: 40,
            max_concurrent_jobs: 2,
            whisper_model: "small".into(),
            openrouter_configured: false,
            openrouter_model: "openrouter/free".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    pub id: String,
    pub name: String,
    pub path: String,
    pub created_at: String,
    pub updated_at: String,
    pub aspect_ratio: String,
    pub resolution: String,
    pub frame_rate: f64,
    pub thumbnail_path: String,
    pub asset_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectManifest {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
    pub aspect_ratio: String,
    pub resolution: String,
    pub frame_rate: f64,
    pub schema_version: u32,
}

impl ProjectManifest {
    pub fn summary(&self, path: &std::path::Path, asset_count: usize, thumbnail: String) -> ProjectSummary {
        ProjectSummary {
            id: self.id.clone(),
            name: self.name.clone(),
            path: path.to_string_lossy().into_owned(),
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
            aspect_ratio: self.aspect_ratio.clone(),
            resolution: self.resolution.clone(),
            frame_rate: self.frame_rate,
            thumbnail_path: thumbnail,
            asset_count,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectInput {
    pub name: String,
    pub aspect_ratio: String,
    pub resolution: String,
    pub frame_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaAsset {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub original_path: String,
    pub managed_path: String,
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    pub frame_rate: f64,
    pub size_bytes: u64,
    pub video_codec: String,
    pub audio_codec: String,
    pub proxy_path: String,
    pub poster_path: String,
    pub waveform_path: String,
    pub index_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scene {
    pub id: String,
    pub asset_id: String,
    pub start: f64,
    pub end: f64,
    pub label: String,
    pub thumbnail_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptSegment {
    pub id: String,
    pub asset_id: String,
    pub start: f64,
    pub end: f64,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectContext {
    pub id: String,
    pub name: String,
    pub source: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexJob {
    pub id: String,
    pub project_path: String,
    pub asset_id: String,
    pub status: String,
    pub progress: f64,
    pub stage: String,
    pub error: String,
    pub started_at: String,
    pub finished_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HardwareDiagnostics {
    pub gpu_name: String,
    pub ffmpeg_version: String,
    pub ffprobe_version: String,
    pub python_version: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapPayload {
    pub settings: AppSettings,
    pub hardware: HardwareDiagnostics,
    pub projects: Vec<ProjectSummary>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSnapshot {
    pub project: ProjectSummary,
    pub assets: Vec<MediaAsset>,
    pub scenes: Vec<Scene>,
    pub transcript: Vec<TranscriptSegment>,
    pub contexts: Vec<ProjectContext>,
    pub jobs: Vec<IndexJob>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenRouterStatus {
    pub ok: bool,
    pub message: String,
    pub model_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenRouterModel {
    pub id: String,
    pub name: String,
    pub context_length: u64,
    pub is_free: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalData {
    pub settings: AppSettings,
    pub recent_projects: Vec<ProjectSummary>,
}

impl Default for GlobalData {
    fn default() -> Self {
        Self {
            settings: AppSettings::default(),
            recent_projects: Vec::new(),
        }
    }
}
