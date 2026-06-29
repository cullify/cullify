use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

use chrono::Utc;
use serde::Serialize;
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

use crate::{
    db::{Database, app_data_dir},
    error::{AppError, AppResult},
    models::{Photo, Project},
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportSummary {
    pub export_dir: String,
    pub json_path: String,
    pub csv_path: String,
    pub zip_path: String,
    pub total: usize,
    pub kept: usize,
    pub culled: usize,
    pub pending: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportManifest {
    exported_at: String,
    project: Project,
    summary: ExportCounts,
    photos: Vec<Photo>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportCounts {
    total: usize,
    kept: usize,
    culled: usize,
    pending: usize,
}

pub fn export_project(db: &Database, project_id: &str) -> AppResult<ExportSummary> {
    export_project_with_root(db, project_id, default_export_root()?)
}

pub fn export_project_with_root(
    db: &Database,
    project_id: &str,
    export_root: PathBuf,
) -> AppResult<ExportSummary> {
    let project = db
        .get_project(project_id)?
        .ok_or_else(|| AppError::ProjectNotFound(project_id.to_string()))?;
    let photos = db.list_photos(project_id)?;
    export_project_to_dir(&project, photos, export_root)
}

pub(crate) fn export_project_to_dir(
    project: &Project,
    photos: Vec<Photo>,
    export_root: PathBuf,
) -> AppResult<ExportSummary> {
    if photos.is_empty() {
        return Err(AppError::EmptyExport(project.id.clone()));
    }

    let exported_at = Utc::now().to_rfc3339();
    let counts = count_photos(&photos);
    let export_name = export_folder_name(project);
    let export_dir = export_root.join(&export_name);
    let staging_dir = export_root.join(format!(".{export_name}.tmp"));
    fs::create_dir_all(&export_root)?;

    let manifest = ExportManifest {
        exported_at,
        project: project.clone(),
        summary: counts,
        photos: photos.clone(),
    };
    let json_path = staging_dir.join("cullify-export.json");
    let csv_path = staging_dir.join("photos.csv");
    let zip_path = staging_dir.join("cullify-export.zip");
    let json = serde_json::to_string_pretty(&manifest)?;
    let csv = photos_to_csv(&photos);

    let write_result = (|| -> AppResult<()> {
        if staging_dir.exists() {
            fs::remove_dir_all(&staging_dir)?;
        }
        fs::create_dir(&staging_dir)?;
        fs::write(&json_path, &json)?;
        fs::write(&csv_path, &csv)?;
        write_zip(&zip_path, &json, &csv, &photos)?;
        fs::rename(&staging_dir, &export_dir)?;
        Ok(())
    })();

    if let Err(error) = write_result {
        let _ = fs::remove_dir_all(&staging_dir);
        return Err(error);
    }

    Ok(ExportSummary {
        export_dir: export_dir.to_string_lossy().to_string(),
        json_path: export_dir
            .join("cullify-export.json")
            .to_string_lossy()
            .to_string(),
        csv_path: export_dir.join("photos.csv").to_string_lossy().to_string(),
        zip_path: export_dir
            .join("cullify-export.zip")
            .to_string_lossy()
            .to_string(),
        total: photos.len(),
        kept: manifest.summary.kept,
        culled: manifest.summary.culled,
        pending: manifest.summary.pending,
    })
}

fn export_folder_name(project: &Project) -> String {
    let suffix = uuid::Uuid::new_v4()
        .simple()
        .to_string()
        .chars()
        .take(8)
        .collect::<String>();
    format!(
        "{}-{}-{suffix}",
        slugify(&project.name),
        Utc::now().format("%Y%m%d-%H%M%S")
    )
}

fn write_zip(path: &Path, json: &str, csv: &str, photos: &[Photo]) -> AppResult<()> {
    let file = fs::File::create(path)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    zip.start_file("cullify-export.json", options)?;
    zip.write_all(json.as_bytes())?;
    zip.start_file("photos.csv", options)?;
    zip.write_all(csv.as_bytes())?;

    let mut buffer = Vec::new();
    for (index, photo) in photos.iter().enumerate() {
        let source_path = Path::new(&photo.file_path);
        if !source_path.is_file() {
            return Err(AppError::SourcePhotoMissing(photo.file_path.clone()));
        }

        buffer.clear();
        fs::File::open(source_path)?.read_to_end(&mut buffer)?;
        zip.start_file(photo_zip_entry_name(index, photo), options)?;
        zip.write_all(&buffer)?;
    }

    zip.finish()?;
    Ok(())
}

fn default_export_root() -> AppResult<PathBuf> {
    Ok(app_data_dir()?.join("exports"))
}

fn count_photos(photos: &[Photo]) -> ExportCounts {
    let kept = photos
        .iter()
        .filter(|photo| photo.user_decision.as_deref() == Some("keep"))
        .count();
    let culled = photos
        .iter()
        .filter(|photo| photo.user_decision.as_deref() == Some("cull") || photo.is_auto_culled)
        .count();
    ExportCounts {
        total: photos.len(),
        kept,
        culled,
        pending: photos.len().saturating_sub(kept + culled),
    }
}

fn photos_to_csv(photos: &[Photo]) -> String {
    let mut output = String::from(
        "id,original_name,file_path,group_label,perceptual_hash,captured_at,camera,lens,focal,aperture,shutter,iso,user_decision,is_auto_culled,quality_score,is_blurred,blur_score,is_overexposed,is_underexposed,exposure_score,width,height,file_size,mime_type,created_at\n",
    );
    for photo in photos {
        let row = [
            photo.id.as_str(),
            photo.original_name.as_str(),
            photo.file_path.as_str(),
            photo.group_label.as_str(),
            photo.perceptual_hash.as_deref().unwrap_or(""),
            photo.captured_at.as_deref().unwrap_or(""),
            photo.camera.as_deref().unwrap_or(""),
            photo.lens.as_deref().unwrap_or(""),
            photo.focal.as_deref().unwrap_or(""),
            photo.aperture.as_deref().unwrap_or(""),
            photo.shutter.as_deref().unwrap_or(""),
            photo.iso.as_deref().unwrap_or(""),
            photo.user_decision.as_deref().unwrap_or("pending"),
            if photo.is_auto_culled {
                "true"
            } else {
                "false"
            },
            &format_optional_f64(photo.quality_score),
            if photo.is_blurred { "true" } else { "false" },
            &format_optional_f64(photo.blur_score),
            if photo.is_overexposed {
                "true"
            } else {
                "false"
            },
            if photo.is_underexposed {
                "true"
            } else {
                "false"
            },
            &format_optional_f64(photo.exposure_score),
            &format_optional_i64(photo.width),
            &format_optional_i64(photo.height),
            &photo.file_size.to_string(),
            photo.mime_type.as_str(),
            photo.created_at.as_str(),
        ];
        output.push_str(
            &row.iter()
                .map(|value| csv_escape(value))
                .collect::<Vec<_>>()
                .join(","),
        );
        output.push('\n');
    }
    output
}

fn photo_zip_entry_name(index: usize, photo: &Photo) -> String {
    let decision = if photo.user_decision.as_deref() == Some("keep") {
        "keep"
    } else if photo.user_decision.as_deref() == Some("cull") || photo.is_auto_culled {
        "cull"
    } else {
        "pending"
    };
    format!(
        "photos/{decision}/{:04}-{}",
        index + 1,
        safe_file_name(&photo.original_name)
    )
}

fn safe_file_name(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0' => '-',
            ch if ch.is_control() => '-',
            ch => ch,
        })
        .collect::<String>();
    let trimmed = sanitized.trim_matches([' ', '.']).trim();
    if trimmed.is_empty() {
        "photo".to_string()
    } else {
        trimmed.chars().take(160).collect()
    }
}

fn csv_escape(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn format_optional_f64(value: Option<f64>) -> String {
    value.map(|value| format!("{value:.4}")).unwrap_or_default()
}

fn format_optional_i64(value: Option<i64>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}

fn slugify(value: &str) -> String {
    let slug = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    let slug = slug
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .take(48)
        .collect::<String>();
    if slug.is_empty() {
        "cullify-project".to_string()
    } else {
        slug
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::NewPhoto;
    use zip::ZipArchive;

    #[test]
    fn csv_escapes_commas_quotes_and_newlines() {
        assert_eq!(csv_escape("plain"), "plain");
        assert_eq!(csv_escape("a,b"), "\"a,b\"");
        assert_eq!(csv_escape("a\"b"), "\"a\"\"b\"");
        assert_eq!(csv_escape("a\nb"), "\"a\nb\"");
    }

    #[test]
    fn export_writes_json_csv_and_zip() {
        let db_path =
            std::env::temp_dir().join(format!("cullify-export-{}.sqlite", uuid::Uuid::new_v4()));
        let export_root =
            std::env::temp_dir().join(format!("cullify-export-{}", uuid::Uuid::new_v4()));
        let photo_root =
            std::env::temp_dir().join(format!("cullify-photos-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&photo_root).expect("photo root should create");
        let keep_path = photo_root.join("keep.jpg");
        let soft_path = photo_root.join("soft,shot.jpg");
        fs::write(&keep_path, b"keep image bytes").expect("keep source should exist");
        fs::write(&soft_path, b"soft image bytes").expect("soft source should exist");

        let db = Database::open(db_path.clone()).expect("database should open");
        let project = db
            .create_project("Wedding, Day 1", &photo_root, "quick")
            .expect("project should create");
        db.insert_photos(
            &project.id,
            &[
                new_photo("a", &keep_path, "keep.jpg", 88.0, false),
                new_photo("b", &soft_path, "soft,shot.jpg", 28.0, true),
            ],
        )
        .expect("photos should insert");
        db.set_photo_decision("a", Some("keep"))
            .expect("decision should save");
        drop(db);

        let db = Database::open(db_path.clone()).expect("database should reopen");
        let project = db.get_project(&project.id).unwrap().unwrap();
        let photos = db.list_photos(&project.id).unwrap();
        let summary = export_project_to_dir(&project, photos, export_root.clone())
            .expect("export should write files");
        let json = fs::read_to_string(&summary.json_path).expect("json should exist");
        let csv = fs::read_to_string(&summary.csv_path).expect("csv should exist");
        let zip_file = fs::File::open(&summary.zip_path).expect("zip should exist");
        let mut zip = ZipArchive::new(zip_file).expect("zip should open");
        let zip_names = (0..zip.len())
            .map(|index| zip.by_index(index).unwrap().name().to_string())
            .collect::<Vec<_>>();
        let _ = fs::remove_file(db_path);
        let _ = fs::remove_dir_all(export_root);
        let _ = fs::remove_dir_all(photo_root);

        assert_eq!(summary.total, 2);
        assert_eq!(summary.kept, 1);
        assert_eq!(summary.culled, 1);
        assert!(json.contains("\"project\""));
        assert!(csv.contains("\"soft,shot.jpg\""));
        assert!(csv.contains("captured_at,camera,lens"));
        assert!(csv.contains("Test Camera"));
        assert!(zip_names.contains(&"cullify-export.json".to_string()));
        assert!(zip_names.contains(&"photos.csv".to_string()));
        assert!(zip_names.contains(&"photos/keep/0001-keep.jpg".to_string()));
        assert!(zip_names.contains(&"photos/cull/0002-soft,shot.jpg".to_string()));
    }

    #[test]
    fn export_project_with_root_uses_requested_directory() {
        let db_path = std::env::temp_dir().join(format!(
            "cullify-export-root-{}.sqlite",
            uuid::Uuid::new_v4()
        ));
        let export_root =
            std::env::temp_dir().join(format!("cullify-picked-export-{}", uuid::Uuid::new_v4()));
        let photo_root =
            std::env::temp_dir().join(format!("cullify-photos-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&photo_root).expect("photo root should create");
        let source_path = photo_root.join("selected.jpg");
        fs::write(&source_path, b"selected image bytes").expect("source should exist");

        let db = Database::open(db_path.clone()).expect("database should open");
        let project = db
            .create_project("Picked Export", &photo_root, "quick")
            .expect("project should create");
        db.insert_photos(
            &project.id,
            &[new_photo(
                "picked",
                &source_path,
                "selected.jpg",
                72.0,
                false,
            )],
        )
        .expect("photo should insert");

        let summary = export_project_with_root(&db, &project.id, export_root.clone())
            .expect("export should use selected root");
        let _ = fs::remove_file(db_path);
        let _ = fs::remove_dir_all(&export_root);
        let _ = fs::remove_dir_all(photo_root);

        assert!(PathBuf::from(&summary.export_dir).starts_with(&export_root));
        assert!(PathBuf::from(&summary.zip_path).ends_with("cullify-export.zip"));
    }

    #[test]
    fn export_fails_when_source_photo_is_missing() {
        let db_path = std::env::temp_dir().join(format!(
            "cullify-export-missing-{}.sqlite",
            uuid::Uuid::new_v4()
        ));
        let export_root =
            std::env::temp_dir().join(format!("cullify-export-missing-{}", uuid::Uuid::new_v4()));
        let db = Database::open(db_path.clone()).expect("database should open");
        let project = db
            .create_project("Missing Source", &PathBuf::from("/missing/photos"), "quick")
            .expect("project should create");
        let missing_path = PathBuf::from("/missing/photos/lost.jpg");
        db.insert_photos(
            &project.id,
            &[new_photo("lost", &missing_path, "lost.jpg", 80.0, false)],
        )
        .expect("photo metadata should insert");

        let project = db.get_project(&project.id).unwrap().unwrap();
        let photos = db.list_photos(&project.id).unwrap();
        let result = export_project_to_dir(&project, photos, export_root.clone());
        let leftover_entries = if export_root.exists() {
            fs::read_dir(&export_root)
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
        } else {
            Vec::new()
        };
        let _ = fs::remove_file(db_path);
        let _ = fs::remove_dir_all(export_root);

        assert!(matches!(result, Err(AppError::SourcePhotoMissing(_))));
        assert!(
            leftover_entries.is_empty(),
            "failed export should not leave partial files"
        );
    }

    fn new_photo(id: &str, path: &Path, name: &str, score: f64, auto_culled: bool) -> NewPhoto {
        NewPhoto {
            id: id.to_string(),
            original_name: name.to_string(),
            file_path: path.to_string_lossy().to_string(),
            thumbnail_path: None,
            file_size: 1024,
            width: Some(4000),
            height: Some(3000),
            mime_type: "image/jpeg".to_string(),
            quality_score: score,
            is_blurred: auto_culled,
            blur_score: if auto_culled { 0.1 } else { 0.9 },
            is_overexposed: false,
            is_underexposed: false,
            exposure_score: 0.7,
            is_auto_culled: auto_culled,
            perceptual_hash: format!("{:016x}", 0xff00ff00ff00ff00u64),
            group_label: "未分组".to_string(),
            captured_at: Some("2026-01-01T00:00:00+00:00".to_string()),
            camera: Some("Test Camera".to_string()),
            lens: Some("Test Lens".to_string()),
            focal: Some("35 mm".to_string()),
            aperture: Some("f/2.8".to_string()),
            shutter: Some("1/125 s".to_string()),
            iso: Some("800".to_string()),
        }
    }
}
