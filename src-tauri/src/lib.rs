mod commands;
mod config;
mod db;
mod error;
mod export;
mod image_analysis;
mod models;
mod photo_metadata;
mod scanner;

use std::path::PathBuf;

use commands::{
    AppState, create_project_from_folder, delete_project, download_model, export_project,
    list_huggingface_vision_models, list_photos, list_projects, load_app_config, rename_project,
    save_app_config, set_photo_decision, set_photo_decisions, start_huggingface_catalog_refresh,
    system_resource_snapshot,
};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_data_dir = resolved_app_data_dir(app)?;
            start_huggingface_catalog_refresh(app_data_dir.clone());
            let state = AppState::new(app_data_dir)?;
            app.manage(state);
            Ok(())
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            list_projects,
            list_photos,
            create_project_from_folder,
            rename_project,
            delete_project,
            set_photo_decision,
            set_photo_decisions,
            load_app_config,
            save_app_config,
            download_model,
            list_huggingface_vision_models,
            system_resource_snapshot,
            export_project
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn resolved_app_data_dir(app: &tauri::App) -> tauri::Result<PathBuf> {
    let app_data_dir = app.path().app_data_dir()?;
    let identifier = app.config().identifier.as_str();
    let directory_name = app_data_dir.file_name().and_then(|name| name.to_str());
    if directory_name != Some(identifier) {
        if let Some(parent) = app_data_dir.parent() {
            return Ok(parent.join(identifier));
        }
    }
    Ok(app_data_dir)
}
