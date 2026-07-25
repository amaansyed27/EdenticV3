mod assistant;
mod asset_commands;
mod commands;
mod indexing;
mod media;
mod models;
mod openrouter;
mod recovery;
mod semantic;
mod slice2_storage;
mod storage;
mod theme;

use models::{AiJob, GlobalData, IndexJob};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tauri::Manager;

pub struct RuntimeState {
    pub data: Mutex<GlobalData>,
    pub jobs: Arc<Mutex<HashMap<String, IndexJob>>>,
    pub ai_jobs: Arc<Mutex<HashMap<String, AiJob>>>,
}

impl RuntimeState {
    fn new() -> Self {
        Self {
            data: Mutex::new(storage::load_global_data()),
            jobs: Arc::new(Mutex::new(HashMap::new())),
            ai_jobs: Arc::new(Mutex::new(HashMap::new())),
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
            semantic::build_local_semantic_map,
            semantic::prepare_semantic_analysis,
            semantic::run_semantic_analysis,
            assistant::prepare_assistant_request,
            assistant::prepare_decision_regeneration,
            assistant::start_assistant_request,
            assistant::get_ai_jobs,
            assistant::cancel_ai_job,
            assistant::save_edit_plan,
            assistant::set_edit_plan_status,
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
