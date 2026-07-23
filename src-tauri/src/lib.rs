mod asset_commands;
mod commands;
mod indexing;
mod media;
mod models;
mod openrouter;
mod recovery;
mod storage;
mod theme;

use models::{GlobalData, IndexJob};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tauri::Manager;

pub struct RuntimeState {
    pub data: Mutex<GlobalData>,
    pub jobs: Arc<Mutex<HashMap<String, IndexJob>>>,
}

impl RuntimeState {
    fn new() -> Self {
        Self {
            data: Mutex::new(storage::load_global_data()),
            jobs: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(RuntimeState::new())
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = theme::apply_window_theme(&window, "dark");
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_bootstrap,
            commands::choose_projects_root,
            commands::complete_onboarding,
            commands::save_settings,
            commands::create_project,
            commands::open_project,
            commands::pick_project,
            commands::get_project_snapshot,
            commands::forget_project,
            commands::import_media,
            asset_commands::delete_media_asset,
            commands::add_pasted_context,
            commands::import_context_file,
            indexing::start_index,
            indexing::get_index_jobs,
            indexing::cancel_index_job,
            commands::save_openrouter_key,
            commands::delete_openrouter_key,
            commands::test_openrouter,
            commands::list_openrouter_models,
            recovery::reset_settings,
            recovery::reset_app_data,
            recovery::reset_cache,
            recovery::repair_app,
            recovery::reset_all,
            theme::sync_window_theme,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Edentic");
}
