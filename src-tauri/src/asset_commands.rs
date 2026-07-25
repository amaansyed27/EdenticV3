use crate::{models::ProjectSnapshot, storage, RuntimeState};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tauri::State;

fn remove_project_file(project_path: &Path, value: &str) {
    if value.is_empty() {
        return;
    }
    let path = PathBuf::from(value);
    if path.starts_with(project_path) && path.is_file() {
        let _ = fs::remove_file(path);
    }
}

#[tauri::command]
pub fn delete_media_asset(
    project_path: String,
    asset_id: String,
    state: State<'_, RuntimeState>,
) -> Result<ProjectSnapshot, String> {
    let project_path = PathBuf::from(project_path);
    let project_path_string = project_path.to_string_lossy().into_owned();
    {
        let jobs = state.jobs.lock().map_err(|_| "Job lock was poisoned".to_string())?;
        let busy = jobs.values().any(|job| {
            job.project_path == project_path_string
                && job.asset_id == asset_id
                && ["queued", "running"].contains(&job.status.as_str())
        });
        if busy {
            return Err("Cancel indexing and wait for it to stop before removing this source.".into());
        }
    }
    {
        let jobs = state.ai_jobs.lock()
            .map_err(|_| "Assistant job lock was poisoned".to_string())?;
        if jobs.values().any(|job| job.project_path == project_path_string
            && ["queued", "running"].contains(&job.status.as_str()))
        {
            return Err("Cancel the active assistant request before removing a source.".into());
        }
    }

    let asset = storage::find_asset(&project_path, &asset_id)?;
    let scene_thumbnails = storage::list_scenes(&project_path)?
        .into_iter()
        .filter(|scene| scene.asset_id == asset_id)
        .map(|scene| scene.thumbnail_path)
        .collect::<Vec<_>>();

    let connection = storage::open_database(&project_path)?;
    connection
        .execute("DELETE FROM assets WHERE id = ?1", [asset_id.as_str()])
        .map_err(|error| format!("Could not remove the source from the project: {error}"))?;

    remove_project_file(&project_path, &asset.managed_path);
    remove_project_file(&project_path, &asset.proxy_path);
    remove_project_file(&project_path, &asset.poster_path);
    remove_project_file(&project_path, &asset.waveform_path);
    for thumbnail in scene_thumbnails {
        remove_project_file(&project_path, &thumbnail);
    }
    let analysis_directory = project_path.join("Cache").join("analysis").join(&asset_id);
    if analysis_directory.is_dir() {
        let _ = fs::remove_dir_all(analysis_directory);
    }

    let jobs = {
        let mut jobs = state.jobs.lock().map_err(|_| "Job lock was poisoned".to_string())?;
        jobs.retain(|_, job| !(job.project_path == project_path_string && job.asset_id == asset_id));
        jobs.values()
            .filter(|job| job.project_path == project_path_string)
            .cloned()
            .collect::<Vec<_>>()
    };

    storage::touch_project(&project_path)?;
    Ok(ProjectSnapshot {
        project: storage::project_summary(&project_path)?,
        assets: storage::list_assets(&project_path)?,
        scenes: storage::list_scenes(&project_path)?,
        transcript: storage::list_transcript(&project_path)?,
        contexts: storage::list_contexts(&project_path)?,
        analysis_frames: crate::slice2_storage::list_analysis_frames(&project_path)?,
        semantic_segments: crate::slice2_storage::list_semantic_segments(&project_path)?,
        conversations: crate::slice2_storage::list_conversations(&project_path)?,
        messages: crate::slice2_storage::list_messages(&project_path)?,
        edit_plans: crate::slice2_storage::list_plans(&project_path)?,
        remote_analysis_approved: crate::slice2_storage::remote_analysis_approved(&project_path)?,
        jobs,
    })
}
