use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use tauri::{Manager, State};

use crate::{
    config::{AppConfig, AppConfigEnvelope},
    db::{Database, app_data_dir},
    error::{AppError, AppResult},
    export::ExportSummary,
    models::{BatchDecisionRequest, CreateProjectRequest, Photo, Project, ScanSummary},
    scanner::{folder_name, generate_thumbnails, scan_folder_with_config},
};

pub struct AppState {
    pub db: Arc<Database>,
}

impl AppState {
    pub fn new() -> AppResult<Self> {
        Ok(Self {
            db: Arc::new(Database::open_default()?),
        })
    }
}

#[tauri::command]
pub fn list_projects(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<Project>, String> {
    let projects = state.db.list_projects().map_err(String::from)?;
    for project in &projects {
        allow_asset_directory(&app, project.folder_path.as_str()).map_err(String::from)?;
    }
    allow_asset_directory(&app, &thumbnail_root().map_err(String::from)?).map_err(String::from)?;
    Ok(projects)
}

#[tauri::command]
pub fn list_photos(
    project_id: String,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<Photo>, String> {
    if let Some(project) = state.db.get_project(&project_id).map_err(String::from)? {
        allow_asset_directory(&app, project.folder_path.as_str()).map_err(String::from)?;
    }
    allow_asset_directory(&app, &thumbnail_root().map_err(String::from)?).map_err(String::from)?;
    state.db.list_photos(&project_id).map_err(Into::into)
}

#[tauri::command]
pub fn create_project_from_folder(
    request: CreateProjectRequest,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<ScanSummary, String> {
    let config = crate::config::load_app_config()
        .map_err(String::from)?
        .config;
    let summary = create_project_from_folder_inner(
        request,
        &state.db,
        &config,
        &thumbnail_root().map_err(String::from)?,
    )
    .map_err(String::from)?;
    allow_asset_directory(&app, summary.project.folder_path.as_str()).map_err(String::from)?;
    allow_asset_directory(&app, &thumbnail_root().map_err(String::from)?).map_err(String::from)?;
    Ok(summary)
}

#[tauri::command]
pub fn rename_project(
    project_id: String,
    name: String,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<Project, String> {
    let project = state
        .db
        .rename_project(&project_id, &name)
        .map_err(String::from)?;
    allow_asset_directory(&app, project.folder_path.as_str()).map_err(String::from)?;
    Ok(project)
}

#[tauri::command]
pub fn delete_project(project_id: String, state: State<'_, AppState>) -> Result<(), String> {
    delete_project_inner(
        &project_id,
        &state.db,
        &thumbnail_root().map_err(String::from)?,
    )
    .map_err(String::from)
}

#[tauri::command]
pub fn set_photo_decision(
    photo_id: String,
    decision: Option<String>,
    state: State<'_, AppState>,
) -> Result<Photo, String> {
    state
        .db
        .set_photo_decision(&photo_id, decision.as_deref())
        .map_err(Into::into)
}

#[tauri::command]
pub fn set_photo_decisions(
    request: BatchDecisionRequest,
    state: State<'_, AppState>,
) -> Result<Vec<Photo>, String> {
    state
        .db
        .set_photo_decisions(
            &request.project_id,
            &request.photo_ids,
            request.decision.as_deref(),
        )
        .map_err(Into::into)
}

#[tauri::command]
pub fn load_app_config() -> Result<AppConfigEnvelope, String> {
    crate::config::load_app_config().map_err(Into::into)
}

#[tauri::command]
pub fn save_app_config(config: AppConfig) -> Result<AppConfigEnvelope, String> {
    crate::config::save_app_config(&config).map_err(Into::into)
}

#[tauri::command]
pub fn export_project(
    project_id: String,
    export_root: Option<String>,
    state: State<'_, AppState>,
) -> Result<ExportSummary, String> {
    if let Some(export_root) = export_root {
        crate::export::export_project_with_root(&state.db, &project_id, PathBuf::from(export_root))
            .map_err(Into::into)
    } else {
        crate::export::export_project(&state.db, &project_id).map_err(Into::into)
    }
}

fn create_project_from_folder_inner(
    request: CreateProjectRequest,
    db: &Database,
    config: &AppConfig,
    thumbnail_root: &Path,
) -> AppResult<ScanSummary> {
    let mode = request.mode.unwrap_or_else(|| "quick".to_string());
    if mode != "quick" {
        return Err(AppError::UnsupportedMode(mode));
    }

    let folder_path = PathBuf::from(&request.folder_path);
    if !folder_path.is_dir() {
        return Err(AppError::InvalidFolder(request.folder_path));
    }

    let analysis_config = crate::image_analysis::AnalysisConfig::from(config);
    let mut scan = scan_folder_with_config(&folder_path, &analysis_config)?;
    if scan.photos.is_empty() {
        return Err(AppError::NoSupportedImages(
            folder_path.to_string_lossy().to_string(),
        ));
    }

    let name = request.name.unwrap_or_else(|| folder_name(&folder_path));

    let project = db.create_project(&name, &folder_path, &mode)?;
    let project_thumbnail_dir = thumbnail_root.join(&project.id);
    if let Err(error) = generate_thumbnails(&mut scan.photos, &project_thumbnail_dir) {
        let _ = db.delete_project(&project.id);
        let _ = fs::remove_dir_all(&project_thumbnail_dir);
        return Err(error);
    }

    let inserted = match db.insert_photos(&project.id, &scan.photos) {
        Ok(inserted) => inserted,
        Err(error) => {
            let _ = db.delete_project(&project.id);
            let _ = fs::remove_dir_all(&project_thumbnail_dir);
            return Err(error);
        }
    };
    let project = db
        .get_project(&project.id)?
        .ok_or_else(|| AppError::ProjectNotFound(project.id.clone()))?;

    Ok(ScanSummary {
        project,
        inserted,
        skipped: scan.skipped,
    })
}

fn delete_project_inner(project_id: &str, db: &Database, thumbnail_root: &Path) -> AppResult<()> {
    db.delete_project(project_id)?;
    let project_thumbnail_dir = thumbnail_root.join(project_id);
    if project_thumbnail_dir.exists() {
        fs::remove_dir_all(project_thumbnail_dir)?;
    }
    Ok(())
}

fn thumbnail_root() -> AppResult<PathBuf> {
    Ok(app_data_dir()?.join("thumbnails"))
}

fn allow_asset_directory(app: &tauri::AppHandle, folder_path: impl AsRef<Path>) -> AppResult<()> {
    app.asset_protocol_scope()
        .allow_directory(folder_path.as_ref().to_path_buf(), true)
        .map_err(AppError::from)
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;
    use crate::export::export_project_to_dir;
    use image::{Rgb, RgbImage};
    use zip::ZipArchive;

    #[test]
    fn empty_import_does_not_create_project() {
        let root = unique_temp_dir("empty-import");
        let photo_dir = root.join("photos");
        fs::create_dir_all(&photo_dir).unwrap();
        fs::write(photo_dir.join("notes.txt"), "not an image").unwrap();

        let db = Database::open(root.join("cullify.db")).unwrap();
        let result = create_project_from_folder_inner(
            CreateProjectRequest {
                folder_path: photo_dir.to_string_lossy().to_string(),
                name: None,
                mode: Some("quick".to_string()),
            },
            &db,
            &AppConfig::default(),
            &root.join("thumbnails"),
        );

        assert!(matches!(result, Err(AppError::NoSupportedImages(_))));
        assert!(db.list_projects().unwrap().is_empty());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_import_folder_is_rejected() {
        let root = unique_temp_dir("missing-import");
        fs::create_dir_all(&root).unwrap();

        let db = Database::open(root.join("cullify.db")).unwrap();
        let missing_path = root.join("missing");
        let result = create_project_from_folder_inner(
            CreateProjectRequest {
                folder_path: missing_path.to_string_lossy().to_string(),
                name: None,
                mode: Some("quick".to_string()),
            },
            &db,
            &AppConfig::default(),
            &root.join("thumbnails"),
        );

        assert!(matches!(result, Err(AppError::InvalidFolder(_))));
        assert!(db.list_projects().unwrap().is_empty());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unsupported_import_mode_does_not_create_project() {
        let root = unique_temp_dir("unsupported-mode");
        let photo_dir = root.join("photos");
        fs::create_dir_all(&photo_dir).unwrap();
        write_checker_png(photo_dir.join("sample.png"));

        let db = Database::open(root.join("cullify.db")).unwrap();
        let result = create_project_from_folder_inner(
            CreateProjectRequest {
                folder_path: photo_dir.to_string_lossy().to_string(),
                name: Some("Expert Attempt".to_string()),
                mode: Some("expert".to_string()),
            },
            &db,
            &AppConfig::default(),
            &root.join("thumbnails"),
        );

        assert!(matches!(result, Err(AppError::UnsupportedMode(mode)) if mode == "expert"));
        assert!(db.list_projects().unwrap().is_empty());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn real_image_project_roundtrips_import_decisions_and_export() {
        let root = unique_temp_dir("roundtrip");
        let photo_dir = root.join("client shoot");
        let export_root = root.join("exports");
        let thumbnail_root = root.join("thumbnails");
        fs::create_dir_all(&photo_dir).unwrap();
        write_checker_png(photo_dir.join("sharp.png"));
        write_solid_png(photo_dir.join("bright.png"), 245);
        fs::write(photo_dir.join("notes.txt"), "skip me").unwrap();

        let db = Database::open(root.join("cullify.db")).unwrap();
        let summary = create_project_from_folder_inner(
            CreateProjectRequest {
                folder_path: photo_dir.to_string_lossy().to_string(),
                name: Some("Client Shoot".to_string()),
                mode: Some("quick".to_string()),
            },
            &db,
            &AppConfig::default(),
            &thumbnail_root,
        )
        .expect("real image folder should import");

        assert_eq!(summary.inserted, 2);
        assert_eq!(summary.skipped, 1);
        assert_eq!(summary.project.name, "Client Shoot");
        assert_eq!(summary.project.total_photos, 2);

        let projects = db.list_projects().unwrap();
        assert_eq!(projects.len(), 1);
        let photos = db.list_photos(&summary.project.id).unwrap();
        assert_eq!(photos.len(), 2);
        assert!(photos.iter().all(|photo| photo.mime_type == "image/png"));
        assert!(photos.iter().all(|photo| photo.width == Some(8)));
        assert!(photos.iter().all(|photo| photo.height == Some(8)));
        assert!(photos.iter().all(|photo| photo.captured_at.is_some()));
        assert!(photos.iter().all(|photo| photo.thumbnail_path.is_some()));
        assert!(
            photos
                .iter()
                .all(|photo| PathBuf::from(photo.thumbnail_path.as_ref().unwrap()).is_file())
        );

        db.set_photo_decision(&photos[0].id, Some("keep"))
            .expect("keep decision should persist");
        db.set_photo_decision(&photos[1].id, Some("cull"))
            .expect("cull decision should persist");
        let project = db
            .get_project(&summary.project.id)
            .unwrap()
            .expect("project should still exist");
        assert_eq!(project.kept_count, 1);
        assert_eq!(project.culled_count, 1);

        let export_summary =
            export_project_to_dir(&project, db.list_photos(&project.id).unwrap(), export_root)
                .expect("roundtrip project should export");
        assert_eq!(export_summary.total, 2);
        assert_eq!(export_summary.kept, 1);
        assert_eq!(export_summary.culled, 1);
        assert_eq!(export_summary.pending, 0);
        assert!(PathBuf::from(&export_summary.json_path).is_file());
        assert!(PathBuf::from(&export_summary.csv_path).is_file());
        assert!(PathBuf::from(&export_summary.zip_path).is_file());

        let zip_file = fs::File::open(&export_summary.zip_path).unwrap();
        let mut zip = ZipArchive::new(zip_file).unwrap();
        assert!(zip.by_name("cullify-export.json").is_ok());
        assert!(zip.by_name("photos.csv").is_ok());
        assert!(zip.by_name("photos/keep/0001-sharp.png").is_ok());
        assert!(zip.by_name("photos/cull/0002-bright.png").is_ok());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn project_can_be_renamed_and_rejects_blank_names() {
        let root = unique_temp_dir("rename-project");
        let photo_dir = root.join("photos");
        fs::create_dir_all(&photo_dir).unwrap();
        write_checker_png(photo_dir.join("sample.png"));

        let db = Database::open(root.join("cullify.db")).unwrap();
        let summary = create_project_from_folder_inner(
            CreateProjectRequest {
                folder_path: photo_dir.to_string_lossy().to_string(),
                name: Some("Original".to_string()),
                mode: Some("quick".to_string()),
            },
            &db,
            &AppConfig::default(),
            &root.join("thumbnails"),
        )
        .unwrap();

        let renamed = db
            .rename_project(&summary.project.id, "  Client Selects  ")
            .unwrap();
        assert_eq!(renamed.name, "Client Selects");
        assert!(matches!(
            db.rename_project(&summary.project.id, "   "),
            Err(AppError::EmptyProjectName)
        ));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn photo_decisions_can_be_updated_in_batch() {
        let root = unique_temp_dir("batch-decisions");
        let photo_dir = root.join("photos");
        fs::create_dir_all(&photo_dir).unwrap();
        write_checker_png(photo_dir.join("keep-a.png"));
        write_solid_png(photo_dir.join("keep-b.png"), 210);

        let db = Database::open(root.join("cullify.db")).unwrap();
        let summary = create_project_from_folder_inner(
            CreateProjectRequest {
                folder_path: photo_dir.to_string_lossy().to_string(),
                name: Some("Batch".to_string()),
                mode: Some("quick".to_string()),
            },
            &db,
            &AppConfig::default(),
            &root.join("thumbnails"),
        )
        .unwrap();
        let photo_ids = db
            .list_photos(&summary.project.id)
            .unwrap()
            .into_iter()
            .map(|photo| photo.id)
            .collect::<Vec<_>>();

        let updated = db
            .set_photo_decisions(&summary.project.id, &photo_ids, Some("keep"))
            .unwrap();
        assert_eq!(updated.len(), 2);
        assert!(
            updated
                .iter()
                .all(|photo| photo.user_decision.as_deref() == Some("keep"))
        );
        let project = db.get_project(&summary.project.id).unwrap().unwrap();
        assert_eq!(project.kept_count, 2);
        assert_eq!(project.culled_count, 0);

        db.set_photo_decisions(&summary.project.id, &photo_ids, Some("cull"))
            .unwrap();
        let project = db.get_project(&summary.project.id).unwrap().unwrap();
        assert_eq!(project.kept_count, 0);
        assert_eq!(project.culled_count, 2);

        db.set_photo_decisions(&summary.project.id, &photo_ids, None)
            .unwrap();
        let project = db.get_project(&summary.project.id).unwrap().unwrap();
        assert_eq!(project.kept_count, 0);
        assert_eq!(project.culled_count, 0);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn batch_decision_rejects_photos_outside_project_without_partial_update() {
        let root = unique_temp_dir("batch-mismatch");
        let first_dir = root.join("first");
        let second_dir = root.join("second");
        fs::create_dir_all(&first_dir).unwrap();
        fs::create_dir_all(&second_dir).unwrap();
        write_checker_png(first_dir.join("first.png"));
        write_checker_png(second_dir.join("second.png"));

        let db = Database::open(root.join("cullify.db")).unwrap();
        let first = create_project_from_folder_inner(
            CreateProjectRequest {
                folder_path: first_dir.to_string_lossy().to_string(),
                name: Some("First".to_string()),
                mode: Some("quick".to_string()),
            },
            &db,
            &AppConfig::default(),
            &root.join("thumbnails"),
        )
        .unwrap();
        let second = create_project_from_folder_inner(
            CreateProjectRequest {
                folder_path: second_dir.to_string_lossy().to_string(),
                name: Some("Second".to_string()),
                mode: Some("quick".to_string()),
            },
            &db,
            &AppConfig::default(),
            &root.join("thumbnails"),
        )
        .unwrap();
        let first_photo = db.list_photos(&first.project.id).unwrap()[0].id.clone();
        let second_photo = db.list_photos(&second.project.id).unwrap()[0].id.clone();

        let result = db.set_photo_decisions(
            &first.project.id,
            &[first_photo.clone(), second_photo],
            Some("keep"),
        );

        assert!(matches!(
            result,
            Err(AppError::PhotoBatchMismatch {
                expected: 2,
                updated: 1
            })
        ));
        assert_eq!(db.get_photo(&first_photo).unwrap().user_decision, None);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn deleting_project_removes_database_rows_and_thumbnail_cache() {
        let root = unique_temp_dir("delete-project");
        let photo_dir = root.join("photos");
        let thumbnail_root = root.join("thumbnails");
        fs::create_dir_all(&photo_dir).unwrap();
        write_checker_png(photo_dir.join("sample.png"));

        let db = Database::open(root.join("cullify.db")).unwrap();
        let summary = create_project_from_folder_inner(
            CreateProjectRequest {
                folder_path: photo_dir.to_string_lossy().to_string(),
                name: Some("To Delete".to_string()),
                mode: Some("quick".to_string()),
            },
            &db,
            &AppConfig::default(),
            &thumbnail_root,
        )
        .unwrap();
        let project_thumbnail_dir = thumbnail_root.join(&summary.project.id);
        assert!(project_thumbnail_dir.is_dir());
        assert!(!db.list_photos(&summary.project.id).unwrap().is_empty());

        delete_project_inner(&summary.project.id, &db, &thumbnail_root).unwrap();

        assert!(db.get_project(&summary.project.id).unwrap().is_none());
        assert!(db.list_photos(&summary.project.id).unwrap().is_empty());
        assert!(!project_thumbnail_dir.exists());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn saved_analysis_thresholds_affect_auto_cull_decisions() {
        let root = unique_temp_dir("configured-analysis");
        let photo_dir = root.join("photos");
        fs::create_dir_all(&photo_dir).unwrap();
        write_solid_png(photo_dir.join("soft.png"), 32);

        let default_db = Database::open(root.join("default.db")).unwrap();
        let default_summary = create_project_from_folder_inner(
            CreateProjectRequest {
                folder_path: photo_dir.to_string_lossy().to_string(),
                name: Some("Default".to_string()),
                mode: Some("quick".to_string()),
            },
            &default_db,
            &AppConfig::default(),
            &root.join("default-thumbnails"),
        )
        .unwrap();
        let default_photo = default_db
            .list_photos(&default_summary.project.id)
            .unwrap()
            .into_iter()
            .next()
            .unwrap();

        let lenient_db = Database::open(root.join("lenient.db")).unwrap();
        let lenient_config = AppConfig {
            blur_threshold: 0.0,
            exposure_tolerance: 1.0,
            cull_line: 0,
            ..AppConfig::default()
        };
        let lenient_summary = create_project_from_folder_inner(
            CreateProjectRequest {
                folder_path: photo_dir.to_string_lossy().to_string(),
                name: Some("Lenient".to_string()),
                mode: Some("quick".to_string()),
            },
            &lenient_db,
            &lenient_config,
            &root.join("lenient-thumbnails"),
        )
        .unwrap();
        let lenient_photo = lenient_db
            .list_photos(&lenient_summary.project.id)
            .unwrap()
            .into_iter()
            .next()
            .unwrap();

        assert!(default_photo.is_auto_culled);
        assert!(!lenient_photo.is_auto_culled);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn similar_images_are_persisted_as_burst_group() {
        let root = unique_temp_dir("burst-group");
        let photo_dir = root.join("photos");
        fs::create_dir_all(&photo_dir).unwrap();
        write_checker_png(photo_dir.join("burst-a.png"));
        write_checker_png(photo_dir.join("burst-b.png"));
        write_solid_png(photo_dir.join("single.png"), 92);

        let db = Database::open(root.join("cullify.db")).unwrap();
        let summary = create_project_from_folder_inner(
            CreateProjectRequest {
                folder_path: photo_dir.to_string_lossy().to_string(),
                name: Some("Burst Group".to_string()),
                mode: Some("quick".to_string()),
            },
            &db,
            &AppConfig::default(),
            &root.join("thumbnails"),
        )
        .unwrap();
        let photos = db.list_photos(&summary.project.id).unwrap();
        let burst = photos
            .iter()
            .filter(|photo| photo.original_name.starts_with("burst-"))
            .collect::<Vec<_>>();
        let single = photos
            .iter()
            .find(|photo| photo.original_name == "single.png")
            .unwrap();

        assert_eq!(burst.len(), 2);
        assert!(burst.iter().all(|photo| photo.group_label == "连拍组 01"));
        assert!(burst.iter().all(|photo| photo.perceptual_hash.is_some()));
        assert_eq!(single.group_label, "未分组");

        fs::remove_dir_all(root).unwrap();
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("cullify-{label}-{stamp}"))
    }

    fn write_checker_png(path: PathBuf) {
        let mut image = RgbImage::new(8, 8);
        for y in 0..8 {
            for x in 0..8 {
                let value = if (x + y) % 2 == 0 { 16 } else { 240 };
                image.put_pixel(x, y, Rgb([value, value, value]));
            }
        }
        image.save(path).unwrap();
    }

    fn write_solid_png(path: PathBuf, value: u8) {
        let image = RgbImage::from_pixel(8, 8, Rgb([value, value, value]));
        image.save(path).unwrap();
    }
}
