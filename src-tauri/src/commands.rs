use crate::{
    media,
    models::{
        AppSettings, BootstrapPayload, CreateProjectInput, IndexJob, MediaAsset, OpenRouterModel,
        OpenRouterStatus, ProjectContext, ProjectManifest, ProjectSnapshot, ProjectSummary,
    },
    openrouter, storage, RuntimeState,
};
use chrono::Utc;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tauri::State;
use uuid::Uuid;

fn upsert_recent(state: &RuntimeState, summary: ProjectSummary) -> Result<(), String> {
    let mut data = state.data.lock().map_err(|_| "Settings lock was poisoned".to_string())?;
    data.recent_projects.retain(|project| project.id != summary.id);
    data.recent_projects.insert(0, summary);
    data.recent_projects.truncate(40);
    storage::save_global_data(&data)
}

fn filtered_recents(state: &RuntimeState) -> Result<Vec<ProjectSummary>, String> {
    let mut data = state.data.lock().map_err(|_| "Settings lock was poisoned".to_string())?;
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
    let mut data = state.data.lock().map_err(|_| "Settings lock was poisoned".to_string())?;
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
    fs::create_dir_all(&root).map_err(|error| format!("Could not create the projects folder: {error}"))?;
    {
        let mut data = state.data.lock().map_err(|_| "Settings lock was poisoned".to_string())?;
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
    let mut data = state.data.lock().map_err(|_| "Settings lock was poisoned".to_string())?;
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
        let data = state.data.lock().map_err(|_| "Settings lock was poisoned".to_string())?;
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
    let mut data = state.data.lock().map_err(|_| "Settings lock was poisoned".to_string())?;
    data.recent_projects.retain(|project| project.id != project_id);
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
    let Some(files) = rfd::FileDialog::new()
        .set_title("Import source video")
        .add_filter("Video", &["mp4", "mov", "mkv", "avi", "webm", "m4v"])
        .pick_files()
    else {
        return Ok(Vec::new());
    };
    let originals = project_path.join("Media/Originals");
    let mut imported = Vec::new();
    for source in files {
        let file_name = source
            .file_name()
            .ok_or_else(|| "A selected file has no file name".to_string())?
            .to_string_lossy()
            .into_owned();
        let destination = media::unique_destination(&originals, &file_name);
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
    upsert_recent(&state, manifest.summary(&project_path, all_assets.len(), thumbnail))?;
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
    storage::insert_context(Path::new(&project_path), &context)?;
    storage::touch_project(Path::new(&project_path))?;
    Ok(context)
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
    let content = fs::read_to_string(&path).map_err(|error| format!("Could not read context: {error}"))?;
    let context = ProjectContext {
        id: Uuid::new_v4().to_string(),
        name: path.file_stem().unwrap_or_default().to_string_lossy().into_owned(),
        source: "file".into(),
        content,
        created_at: Utc::now().to_rfc3339(),
    };
    storage::insert_context(Path::new(&project_path), &context)?;
    storage::touch_project(Path::new(&project_path))?;
    Ok(Some(context))
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
        let result = media::build_index(&path, asset, &settings, |progress, stage| {
            update_job(&jobs, &job_id, |job| {
                job.progress = progress;
                job.stage = stage.into();
            })
        });
        match result {
            Ok(result) => {
                update_job(&jobs, &job_id, |job| {
                    job.status = "completed".into();
                    job.progress = 1.0;
                    job.stage = if result.warning.is_some() {
                        "Video Map ready · transcript unavailable".into()
                    } else {
                        "Video Map ready".into()
                    };
                    job.error = result.warning.unwrap_or_default();
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

#[tauri::command]
pub fn save_openrouter_key(api_key: String, state: State<'_, RuntimeState>) -> Result<serde_json::Value, String> {
    openrouter::save_key(api_key.trim())?;
    let mut data = state.data.lock().map_err(|_| "Settings lock was poisoned".to_string())?;
    data.settings.openrouter_configured = true;
    storage::save_global_data(&data)?;
    Ok(serde_json::json!({ "configured": true }))
}

#[tauri::command]
pub fn delete_openrouter_key(state: State<'_, RuntimeState>) -> Result<serde_json::Value, String> {
    openrouter::delete_key()?;
    let mut data = state.data.lock().map_err(|_| "Settings lock was poisoned".to_string())?;
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
