use crate::{
    media,
    models::{
        AppSettings, BootstrapPayload, CreateProjectInput, MediaAsset, OpenRouterModel,
        OpenRouterStatus, ProjectContext, ProjectManifest, ProjectSnapshot, ProjectSummary,
    },
    openrouter, storage, RuntimeState,
};
use chrono::Utc;
use std::{
    fs,
    path::{Path, PathBuf},
};
use tauri::State;
use uuid::Uuid;

fn upsert_recent(state: &RuntimeState, summary: ProjectSummary) -> Result<(), String> {
    let mut data = state
        .data
        .lock()
        .map_err(|_| "Settings lock was poisoned".to_string())?;
    data.recent_projects
        .retain(|project| project.id != summary.id);
    data.recent_projects.insert(0, summary);
    data.recent_projects.truncate(40);
    storage::save_global_data(&data)
}

fn filtered_recents(state: &RuntimeState) -> Result<Vec<ProjectSummary>, String> {
    let mut data = state
        .data
        .lock()
        .map_err(|_| "Settings lock was poisoned".to_string())?;
    let mut refreshed = Vec::new();
    for recent in &data.recent_projects {
        let path = Path::new(&recent.path);
        if path.exists() {
            if let Ok(summary) = storage::project_summary(path) {
                refreshed.push(summary);
            }
        }
    }
    data.recent_projects = refreshed.clone();
    storage::save_global_data(&data)?;
    Ok(refreshed)
}

#[tauri::command]
pub fn get_bootstrap(state: State<'_, RuntimeState>) -> Result<BootstrapPayload, String> {
    let projects = filtered_recents(&state)?;
    let mut data = state
        .data
        .lock()
        .map_err(|_| "Settings lock was poisoned".to_string())?;
    data.settings.openrouter_configured = openrouter::has_key();
    Ok(BootstrapPayload {
        settings: data.settings.clone(),
        hardware: media::hardware_diagnostics(),
        projects,
    })
}

#[tauri::command]
pub fn choose_projects_root() -> Option<String> {
    rfd::FileDialog::new()
        .set_title("Choose where Edentic projects live")
        .pick_folder()
        .map(|path| path.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn complete_onboarding(
    projects_root: String,
    state: State<'_, RuntimeState>,
) -> Result<BootstrapPayload, String> {
    let root = PathBuf::from(&projects_root);
    fs::create_dir_all(&root)
        .map_err(|error| format!("Could not create the projects folder: {error}"))?;
    {
        let mut data = state
            .data
            .lock()
            .map_err(|_| "Settings lock was poisoned".to_string())?;
        data.settings.projects_root = root.to_string_lossy().into_owned();
        data.settings.onboarding_complete = true;
        data.settings.openrouter_configured = openrouter::has_key();
        storage::save_global_data(&data)?;
    }
    get_bootstrap(state)
}

#[tauri::command]
pub fn save_settings(
    mut settings: AppSettings,
    state: State<'_, RuntimeState>,
) -> Result<AppSettings, String> {
    settings.openrouter_configured = openrouter::has_key();
    settings.max_concurrent_jobs = settings.max_concurrent_jobs.clamp(1, 8);
    settings.cache_limit_gb = settings.cache_limit_gb.clamp(5, 500);
    if !["dark", "light", "system"].contains(&settings.theme.as_str()) {
        return Err("Unknown appearance theme".into());
    }
    if !["auto", "gpu", "hybrid", "cpu"].contains(&settings.compute_mode.as_str()) {
        return Err("Unknown processing mode".into());
    }
    let mut data = state
        .data
        .lock()
        .map_err(|_| "Settings lock was poisoned".to_string())?;
    data.settings = settings.clone();
    storage::save_global_data(&data)?;
    Ok(settings)
}

#[tauri::command]
pub fn create_project(
    input: CreateProjectInput,
    state: State<'_, RuntimeState>,
) -> Result<ProjectSummary, String> {
    let name = storage::sanitize_project_name(&input.name);
    if name.is_empty() {
        return Err("Enter a valid project name".into());
    }
    let projects_root = {
        let data = state
            .data
            .lock()
            .map_err(|_| "Settings lock was poisoned".to_string())?;
        PathBuf::from(&data.settings.projects_root)
    };
    fs::create_dir_all(&projects_root).map_err(|error| error.to_string())?;
    let project_path = projects_root.join(&name);
    if project_path.exists() {
        return Err("A project with this name already exists in the selected location".into());
    }
    storage::initialize_project_folders(&project_path)?;
    let now = Utc::now().to_rfc3339();
    let manifest = ProjectManifest {
        id: Uuid::new_v4().to_string(),
        name,
        created_at: now.clone(),
        updated_at: now,
        aspect_ratio: input.aspect_ratio,
        resolution: input.resolution,
        frame_rate: input.frame_rate,
        schema_version: 1,
    };
    storage::save_manifest(&project_path, &manifest)?;
    storage::open_database(&project_path)?;
    let summary = manifest.summary(&project_path, 0, String::new());
    upsert_recent(&state, summary.clone())?;
    Ok(summary)
}

fn snapshot(project_path: &Path, state: &RuntimeState) -> Result<ProjectSnapshot, String> {
    let project = storage::project_summary(project_path)?;
    let jobs = state
        .jobs
        .lock()
        .map_err(|_| "Job lock was poisoned".to_string())?
        .values()
        .filter(|job| Path::new(&job.project_path) == project_path)
        .cloned()
        .collect();
    Ok(ProjectSnapshot {
        project,
        assets: storage::list_assets(project_path)?,
        scenes: storage::list_scenes(project_path)?,
        transcript: storage::list_transcript(project_path)?,
        contexts: storage::list_contexts(project_path)?,
        analysis_frames: crate::slice2_storage::list_analysis_frames(project_path)?,
        semantic_segments: crate::slice2_storage::list_semantic_segments(project_path)?,
        conversations: crate::slice2_storage::list_conversations(project_path)?,
        messages: crate::slice2_storage::list_messages(project_path)?,
        edit_plans: crate::slice2_storage::list_plans(project_path)?,
        remote_analysis_approved: crate::slice2_storage::remote_analysis_approved(project_path)?,
        jobs,
    })
}

#[tauri::command]
pub fn open_project(
    project_path: String,
    state: State<'_, RuntimeState>,
) -> Result<ProjectSnapshot, String> {
    let path = PathBuf::from(project_path);
    let manifest = storage::touch_project(&path)?;
    let assets = storage::list_assets(&path)?;
    let thumbnail = assets
        .iter()
        .find(|asset| !asset.poster_path.is_empty())
        .map(|asset| asset.poster_path.clone())
        .unwrap_or_default();
    upsert_recent(&state, manifest.summary(&path, assets.len(), thumbnail))?;
    snapshot(&path, &state)
}

#[tauri::command]
pub fn pick_project(state: State<'_, RuntimeState>) -> Result<Option<ProjectSnapshot>, String> {
    let Some(path) = rfd::FileDialog::new()
        .set_title("Open an Edentic project")
        .pick_folder()
    else {
        return Ok(None);
    };
    open_project(path.to_string_lossy().into_owned(), state).map(Some)
}

#[tauri::command]
pub fn get_project_snapshot(
    project_path: String,
    state: State<'_, RuntimeState>,
) -> Result<ProjectSnapshot, String> {
    snapshot(Path::new(&project_path), &state)
}

#[tauri::command]
pub fn forget_project(project_id: String, state: State<'_, RuntimeState>) -> Result<bool, String> {
    let mut data = state
        .data
        .lock()
        .map_err(|_| "Settings lock was poisoned".to_string())?;
    data.recent_projects
        .retain(|project| project.id != project_id);
    storage::save_global_data(&data)?;
    Ok(true)
}

#[tauri::command]
pub fn import_media(
    project_path: String,
    state: State<'_, RuntimeState>,
) -> Result<Vec<MediaAsset>, String> {
    let project_path = PathBuf::from(project_path);
    let manifest = storage::load_manifest(&project_path)?;
    const VIDEO: &[&str] = &[
        "mp4", "mov", "mkv", "avi", "webm", "m4v", "mpeg", "mpg", "wmv", "flv", "ts", "m2ts", "3gp",
    ];
    const AUDIO: &[&str] = &[
        "wav", "mp3", "m4a", "aac", "flac", "ogg", "opus", "wma", "aiff", "aif",
    ];
    const IMAGE: &[&str] = &[
        "png", "jpg", "jpeg", "webp", "gif", "bmp", "tif", "tiff", "avif",
    ];
    let supported = VIDEO
        .iter()
        .chain(AUDIO)
        .chain(IMAGE)
        .copied()
        .collect::<Vec<_>>();
    let Some(files) = rfd::FileDialog::new()
        .set_title("Import media")
        .add_filter("Supported media", &supported)
        .add_filter("Video", VIDEO)
        .add_filter("Audio", AUDIO)
        .add_filter("Images", IMAGE)
        .pick_files()
    else {
        return Ok(Vec::new());
    };
    let mut imported = Vec::new();
    for source in files {
        let file_name = source
            .file_name()
            .ok_or_else(|| "A selected file has no file name".to_string())?
            .to_string_lossy()
            .into_owned();
        let managed_directory = project_path.join(media::managed_media_directory(&source));
        fs::create_dir_all(&managed_directory).map_err(|error| error.to_string())?;
        let destination = media::unique_destination(&managed_directory, &file_name);
        fs::copy(&source, &destination)
            .map_err(|error| format!("Could not copy {}: {error}", source.display()))?;
        match media::probe_media(&destination, &manifest.id, &source) {
            Ok(asset) => {
                storage::insert_asset(&project_path, &asset)?;
                imported.push(asset);
            }
            Err(error) => {
                let _ = fs::remove_file(&destination);
                return Err(error);
            }
        }
    }
    let manifest = storage::touch_project(&project_path)?;
    let all_assets = storage::list_assets(&project_path)?;
    let thumbnail = all_assets
        .iter()
        .find(|asset| !asset.poster_path.is_empty())
        .map(|asset| asset.poster_path.clone())
        .unwrap_or_default();
    upsert_recent(
        &state,
        manifest.summary(&project_path, all_assets.len(), thumbnail),
    )?;
    Ok(imported)
}

#[tauri::command]
pub fn add_pasted_context(
    project_path: String,
    name: String,
    content: String,
) -> Result<ProjectContext, String> {
    if name.trim().is_empty() || content.trim().is_empty() {
        return Err("Context needs both a name and content".into());
    }
    let context = ProjectContext {
        id: Uuid::new_v4().to_string(),
        name: name.trim().to_string(),
        source: "pasted".into(),
        content: content.trim().to_string(),
        created_at: Utc::now().to_rfc3339(),
    };
    let project_path = Path::new(&project_path);
    storage::insert_context(project_path, &context)?;
    storage::touch_project(project_path)?;
    storage::list_contexts(project_path)?
        .into_iter()
        .find(|saved| saved.id == context.id)
        .ok_or_else(|| "Context could not be verified after saving".to_string())
}

#[tauri::command]
pub fn import_context_file(project_path: String) -> Result<Option<ProjectContext>, String> {
    let Some(path) = rfd::FileDialog::new()
        .set_title("Import project context")
        .add_filter("Text", &["txt", "md"])
        .pick_file()
    else {
        return Ok(None);
    };
    let content =
        fs::read_to_string(&path).map_err(|error| format!("Could not read context: {error}"))?;
    let context = ProjectContext {
        id: Uuid::new_v4().to_string(),
        name: path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        source: "file".into(),
        content,
        created_at: Utc::now().to_rfc3339(),
    };
    let project_path = Path::new(&project_path);
    storage::insert_context(project_path, &context)?;
    storage::touch_project(project_path)?;
    let saved = storage::list_contexts(project_path)?
        .into_iter()
        .find(|saved| saved.id == context.id)
        .ok_or_else(|| "Context could not be verified after saving".to_string())?;
    Ok(Some(saved))
}

#[tauri::command]
pub fn save_openrouter_key(
    api_key: String,
    state: State<'_, RuntimeState>,
) -> Result<serde_json::Value, String> {
    openrouter::save_key(api_key.trim())?;
    let mut data = state
        .data
        .lock()
        .map_err(|_| "Settings lock was poisoned".to_string())?;
    data.settings.openrouter_configured = true;
    storage::save_global_data(&data)?;
    Ok(serde_json::json!({ "configured": true }))
}

#[tauri::command]
pub fn delete_openrouter_key(state: State<'_, RuntimeState>) -> Result<serde_json::Value, String> {
    openrouter::delete_key()?;
    let mut data = state
        .data
        .lock()
        .map_err(|_| "Settings lock was poisoned".to_string())?;
    data.settings.openrouter_configured = false;
    storage::save_global_data(&data)?;
    Ok(serde_json::json!({ "configured": false }))
}

#[tauri::command]
pub fn test_openrouter() -> Result<OpenRouterStatus, String> {
    openrouter::test_connection()
}

#[tauri::command]
pub fn list_openrouter_models() -> Result<Vec<OpenRouterModel>, String> {
    openrouter::list_models()
}
