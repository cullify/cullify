use std::{
    fs,
    path::{Path, PathBuf},
};

use walkdir::WalkDir;

use crate::{
    error::AppResult,
    image_analysis::{AnalysisConfig, analyze_image_with_config},
    models::NewPhoto,
    photo_metadata::read_photo_metadata,
};

pub struct ScanResult {
    pub photos: Vec<NewPhoto>,
    pub skipped: usize,
}

pub fn scan_folder_with_config(
    folder_path: &Path,
    config: &AnalysisConfig,
) -> AppResult<ScanResult> {
    let mut photos = Vec::new();
    let mut skipped = 0usize;

    for entry in WalkDir::new(folder_path).follow_links(false) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        if !is_supported_image(path) {
            skipped += 1;
            continue;
        }

        match photo_from_path(path, config) {
            Ok(photo) => photos.push(photo),
            Err(_) => skipped += 1,
        }
    }

    photos.sort_by(|left, right| left.file_path.cmp(&right.file_path));
    if config.auto_group {
        assign_similarity_groups(&mut photos);
    }

    Ok(ScanResult { photos, skipped })
}

fn photo_from_path(path: &Path, config: &AnalysisConfig) -> AppResult<NewPhoto> {
    let metadata = std::fs::metadata(path)?;
    let analysis = analyze_image_with_config(path, config)?;
    let photo_metadata = read_photo_metadata(path);
    let original_name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "untitled".to_string());

    Ok(NewPhoto {
        id: uuid::Uuid::new_v4().to_string(),
        original_name,
        file_path: normalize_path(path),
        thumbnail_path: None,
        file_size: metadata.len() as i64,
        width: Some(analysis.width as i64),
        height: Some(analysis.height as i64),
        mime_type: mime_for_path(path).to_string(),
        quality_score: analysis.quality_score,
        is_blurred: analysis.is_blurred,
        blur_score: analysis.blur_score,
        is_overexposed: analysis.is_overexposed,
        is_underexposed: analysis.is_underexposed,
        exposure_score: analysis.exposure_score,
        is_auto_culled: analysis.is_auto_culled,
        perceptual_hash: format!("{:016x}", analysis.perceptual_hash),
        group_label: "未分组".to_string(),
        captured_at: photo_metadata.captured_at,
        camera: photo_metadata.camera,
        lens: photo_metadata.lens,
        focal: photo_metadata.focal,
        aperture: photo_metadata.aperture,
        shutter: photo_metadata.shutter,
        iso: photo_metadata.iso,
    })
}

pub fn generate_thumbnails(photos: &mut [NewPhoto], thumbnail_dir: &Path) -> AppResult<()> {
    fs::create_dir_all(thumbnail_dir)?;

    for photo in photos {
        let thumbnail_path = thumbnail_dir.join(format!("{}.png", photo.id));
        let image = image::open(&photo.file_path)?;
        let thumbnail = image.thumbnail(800, 800);
        thumbnail.save(&thumbnail_path)?;
        photo.thumbnail_path = Some(normalize_path(&thumbnail_path));
    }

    Ok(())
}

fn assign_similarity_groups(photos: &mut [NewPhoto]) {
    let mut groups: Vec<Vec<usize>> = Vec::new();

    for index in 0..photos.len() {
        let hash = parse_hash(&photos[index].perceptual_hash);
        let matching_group = groups.iter().position(|group| {
            group.iter().any(|candidate_index| {
                let candidate_hash = parse_hash(&photos[*candidate_index].perceptual_hash);
                hamming_distance(hash, candidate_hash) <= 8
            })
        });

        if let Some(group_index) = matching_group {
            groups[group_index].push(index);
        } else {
            groups.push(vec![index]);
        }
    }

    let mut burst_number = 1usize;
    for group in groups {
        if group.len() < 2 {
            continue;
        }

        let label = format!("连拍组 {:02}", burst_number);
        burst_number += 1;
        for index in group {
            photos[index].group_label = label.clone();
        }
    }
}

fn parse_hash(value: &str) -> u64 {
    u64::from_str_radix(value, 16).unwrap_or(0)
}

fn hamming_distance(left: u64, right: u64) -> u32 {
    (left ^ right).count_ones()
}

pub fn is_supported_image(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "jpg" | "jpeg" | "png" | "webp" | "tif" | "tiff"
            )
        })
        .unwrap_or(false)
}

pub fn folder_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| "Untitled Project".to_string())
}

fn mime_for_path(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
        .as_deref()
    {
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("webp") => "image/webp",
        Some("tif" | "tiff") => "image/tiff",
        _ => "application/octet-stream",
    }
}

fn normalize_path(path: &Path) -> String {
    PathBuf::from(path).to_string_lossy().to_string()
}
