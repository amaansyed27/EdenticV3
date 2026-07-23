use crate::{
    models::{AppSettings, GlobalData},
    openrouter, storage, RuntimeState,
};
use rusqlite::params;
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};
use tauri::State;

fn known_projects(state: &RuntimeState) -> Result<Vec<PathBuf>, String> {
    let data = state
        .data
        .lock()
        .map_err(|_| "Settings lock was poisoned".to_string())?;
    let mut seen = HashSet::new();
    Ok(data
        .recent_projects
        .iter()
        .filter_map(|project| {
            if seen.insert(project.path.clone()) {
                Some(PathBuf::from(&project.path))
            } else {
                None
            }
        })
        .collect())
}

fn stop_jobs(state: &RuntimeState) -> Result<(), String> {
    let mut jobs = state
        .jobs
        .lock()
        .map_err(|_| "Job lock was poisoned".to_string())?;
    for job in jobs.values_mut() {
        if ["queued", "running"].contains(&job.status.as_str()) {
            job.status = "cancelled".into();
            job.stage = "Cancelled by recovery".into();
        }
    }
    jobs.clear();
    Ok(())
}

fn directory_stats(path: &Path) -> Result<(usize, u64), String> {
    if !path.exists() {
        return Ok((0, 0));
    }
    let metadata = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if metadata.file_type().is_symlink() || metadata.is_file() {
        return Ok((1, metadata.len()));
    }
    let mut files = 0;
    let mut bytes = 0;
    for entry in fs::read_dir(path).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let (entry_files, entry_bytes) = directory_stats(&entry.path())?;
        files += entry_files;
        bytes += entry_bytes;
    }
    Ok((files, bytes))
}

fn purge_directory(path: &Path) -> Result<(usize, u64), String> {
    let stats = directory_stats(path)?;
    if path.exists() {
        fs::remove_dir_all(path)
            .map_err(|error| format!("Could not clear {}: {error}", path.display()))?;
    }
    fs::create_dir_all(path).map_err(|error| error.to_string())?;
    Ok(stats)
}

fn clear_project_cache(project_path: &Path) -> Result<(usize, u64), String> {
    storage::load_manifest(project_path)?;
    let targets = [
        project_path.join("Cache"),
        project_path.join("Proxies"),
        project_path.join("Edit/index-data"),
    ];
    let mut files = 0;
    let mut bytes = 0;
    for target in targets {
        let (target_files, target_bytes) = purge_directory(&target)?;
        files += target_files;
        bytes += target_bytes;
    }
    storage::initialize_project_folders(project_path)?;
    let connection = storage::open_database(project_path)?;
    connection
        .execute_batch(
            "
            DELETE FROM scenes;
            DELETE FROM transcript;
            UPDATE assets
               SET proxy_path = '', poster_path = '', waveform_path = '', index_status = 'waiting';
            ",
        )
        .map_err(|error| error.to_string())?;
    Ok((files, bytes))
}

fn missing_path(path: &str) -> bool {
    !path.is_empty() && !Path::new(path).exists()
}

fn repair_project(project_path: &Path) -> Result<(), String> {
    storage::load_manifest(project_path)?;
    storage::initialize_project_folders(project_path)?;
    let connection = storage::open_database(project_path)?;
    let integrity: String = connection
        .query_row("PRAGMA quick_check(1)", [], |row| row.get(0))
        .map_err(|error| format!("Database check failed: {error}"))?;
    if integrity != "ok" {
        return Err(format!("Database integrity check returned: {integrity}"));
    }

    for asset in storage::list_assets(project_path)? {
        let source_missing = !Path::new(&asset.managed_path).exists();
        let proxy_missing = missing_path(&asset.proxy_path);
        let poster_missing = missing_path(&asset.poster_path);
        let waveform_missing = missing_path(&asset.waveform_path);
        let derived_missing = proxy_missing || poster_missing || waveform_missing;
        let proxy_path = if proxy_missing { "" } else { &asset.proxy_path };
        let poster_path = if poster_missing { "" } else { &asset.poster_path };
        let waveform_path = if waveform_missing { "" } else { &asset.waveform_path };
        let status = if source_missing {
            "missing"
        } else if derived_missing {
            "waiting"
        } else {
            &asset.index_status
        };

        connection
            .execute(
                "UPDATE assets SET proxy_path=?1, poster_path=?2, waveform_path=?3, index_status=?4 WHERE id=?5",
                params![proxy_path, poster_path, waveform_path, status, asset.id.clone()],
            )
            .map_err(|error| error.to_string())?;
        if source_missing || derived_missing {
            connection
                .execute("DELETE FROM scenes WHERE asset_id=?1", [asset.id.as_str()])
                .map_err(|error| error.to_string())?;
            connection
                .execute("DELETE FROM transcript WHERE asset_id=?1", [asset.id.as_str()])
                .map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn report(message: String, projects: usize, files: usize, bytes: u64, warnings: Vec<String>) -> Value {
    json!({
        "message": message,
        "projects": projects,
        "files": files,
        "bytes": bytes,
        "warnings": warnings,
    })
}

#[tauri::command]
pub fn reset_settings(state: State<'_, RuntimeState>) -> Result<Value, String> {
    let mut data = state
        .data
        .lock()
        .map_err(|_| "Settings lock was poisoned".to_string())?;
    let onboarding_complete = data.settings.onboarding_complete;
    let projects_root = data.settings.projects_root.clone();
    let mut settings = AppSettings::default();
    settings.onboarding_complete = onboarding_complete;
    settings.projects_root = projects_root;
    settings.openrouter_configured = openrouter::has_key();
    data.settings = settings;
    storage::save_global_data(&data)?;
    Ok(report(
        "Settings restored to defaults".into(),
        data.recent_projects.len(),
        0,
        0,
        Vec::new(),
    ))
}

#[tauri::command]
pub fn reset_app_data(state: State<'_, RuntimeState>) -> Result<Value, String> {
    stop_jobs(&state)?;
    openrouter::delete_key()?;
    let mut data = state
        .data
        .lock()
        .map_err(|_| "Settings lock was poisoned".to_string())?;
    let removed = data.recent_projects.len();
    data.recent_projects.clear();
    data.settings.openrouter_configured = false;
    storage::save_global_data(&data)?;
    Ok(report(
        "App data cleared · project folders were preserved".into(),
        removed,
        0,
        0,
        Vec::new(),
    ))
}

#[tauri::command]
pub fn reset_cache(state: State<'_, RuntimeState>) -> Result<Value, String> {
    stop_jobs(&state)?;
    let paths = known_projects(&state)?;
    let mut projects = 0;
    let mut files = 0;
    let mut bytes = 0;
    let mut warnings = Vec::new();
    for path in paths {
        match clear_project_cache(&path) {
            Ok((project_files, project_bytes)) => {
                projects += 1;
                files += project_files;
                bytes += project_bytes;
            }
            Err(error) => warnings.push(format!("{}: {error}", path.display())),
        }
    }

    let mut data = state
        .data
        .lock()
        .map_err(|_| "Settings lock was poisoned".to_string())?;
    let recent_projects = data.recent_projects.clone();
    data.recent_projects = recent_projects
        .iter()
        .filter_map(|project| storage::project_summary(Path::new(&project.path)).ok())
        .collect();
    storage::save_global_data(&data)?;
    Ok(report(
        format!("Generated cache cleared for {projects} project(s)"),
        projects,
        files,
        bytes,
        warnings,
    ))
}

#[tauri::command]
pub fn repair_app(state: State<'_, RuntimeState>) -> Result<Value, String> {
    stop_jobs(&state)?;
    let mut data = state
        .data
        .lock()
        .map_err(|_| "Settings lock was poisoned".to_string())?;
    if !data.settings.projects_root.is_empty() {
        fs::create_dir_all(&data.settings.projects_root)
            .map_err(|error| format!("Could not repair the projects folder: {error}"))?;
    }

    let existing = data.recent_projects.clone();
    let mut repaired = Vec::new();
    let mut warnings = Vec::new();
    for project in existing {
        let path = PathBuf::from(&project.path);
        if !path.exists() {
            warnings.push(format!("Removed missing recent project: {}", project.name));
            continue;
        }
        match repair_project(&path).and_then(|_| storage::project_summary(&path)) {
            Ok(summary) => repaired.push(summary),
            Err(error) => warnings.push(format!("{}: {error}", project.name)),
        }
    }
    let repaired_count = repaired.len();
    data.recent_projects = repaired;
    data.settings.openrouter_configured = openrouter::has_key();
    storage::save_global_data(&data)?;
    Ok(report(
        format!("Repair completed for {repaired_count} project(s)"),
        repaired_count,
        0,
        0,
        warnings,
    ))
}

#[tauri::command]
pub fn reset_all(state: State<'_, RuntimeState>) -> Result<Value, String> {
    stop_jobs(&state)?;
    let paths = known_projects(&state)?;
    let mut projects = 0;
    let mut files = 0;
    let mut bytes = 0;
    let mut warnings = Vec::new();
    for path in paths {
        match clear_project_cache(&path) {
            Ok((project_files, project_bytes)) => {
                projects += 1;
                files += project_files;
                bytes += project_bytes;
            }
            Err(error) => warnings.push(format!("{}: {error}", path.display())),
        }
    }
    openrouter::delete_key()?;
    let mut data = state
        .data
        .lock()
        .map_err(|_| "Settings lock was poisoned".to_string())?;
    *data = GlobalData::default();
    storage::save_global_data(&data)?;
    Ok(report(
        "Edentic reset complete · onboarding will open next".into(),
        projects,
        files,
        bytes,
        warnings,
    ))
}
