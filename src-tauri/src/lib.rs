mod commands;
mod config;
mod db;
mod error;
mod export;
mod image_analysis;
mod models;
mod photo_metadata;
mod scanner;

use commands::{
    AppState, create_project_from_folder, delete_project, export_project, list_photos,
    list_projects, load_app_config, rename_project, save_app_config, set_photo_decision,
    set_photo_decisions,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = AppState::new().expect("failed to initialize Cullify application state");

    tauri::Builder::default()
        .manage(state)
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
            export_project
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
