use std::{
    collections::{HashMap, HashSet},
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use chrono::{DateTime, Utc};
use reqwest::{
    StatusCode,
    header::{CONTENT_RANGE, RANGE},
};
use serde::{Deserialize, Serialize};
use sysinfo::{ProcessesToUpdate, System};
use tauri::{Emitter, Manager, State};

use crate::{
    config::{AppConfig, AppConfigEnvelope},
    db::Database,
    error::{AppError, AppResult},
    export::ExportSummary,
    models::{BatchDecisionRequest, CreateProjectRequest, Photo, Project, ScanSummary},
    scanner::{folder_name, generate_thumbnails, scan_folder_with_config},
};

const MODEL_DOWNLOAD_PROGRESS_EVENT: &str = "model-download-progress";

pub struct AppState {
    pub db: Arc<Database>,
    pub app_data_dir: PathBuf,
    download_cancellations: Arc<Mutex<HashSet<String>>>,
    active_downloads: Arc<Mutex<HashSet<String>>>,
}

impl AppState {
    pub fn new(app_data_dir: PathBuf) -> AppResult<Self> {
        fs::create_dir_all(&app_data_dir)?;
        Ok(Self {
            db: Arc::new(Database::open(app_data_dir.join("cullify.db"))?),
            app_data_dir,
            download_cancellations: Arc::new(Mutex::new(HashSet::new())),
            active_downloads: Arc::new(Mutex::new(HashSet::new())),
        })
    }

    fn download_cancellations(&self) -> Arc<Mutex<HashSet<String>>> {
        Arc::clone(&self.download_cancellations)
    }

    fn active_downloads(&self) -> Arc<Mutex<HashSet<String>>> {
        Arc::clone(&self.active_downloads)
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemResourceSnapshot {
    pub cpu_usage: f32,
    pub app_memory_bytes: u64,
    pub used_memory_bytes: u64,
    pub total_memory_bytes: u64,
}

#[tauri::command]
pub fn system_resource_snapshot() -> Result<SystemResourceSnapshot, String> {
    let mut system = System::new_all();
    system.refresh_cpu_all();
    system.refresh_memory();

    let app_memory_bytes = sysinfo::get_current_pid()
        .ok()
        .and_then(|pid| {
            system.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);
            system.process(pid).map(|process| process.memory())
        })
        .unwrap_or(0);

    Ok(SystemResourceSnapshot {
        cpu_usage: system.global_cpu_usage(),
        app_memory_bytes,
        used_memory_bytes: system.used_memory(),
        total_memory_bytes: system.total_memory(),
    })
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
    allow_asset_directory(&app, &thumbnail_root(&state.app_data_dir)).map_err(String::from)?;
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
    allow_asset_directory(&app, &thumbnail_root(&state.app_data_dir)).map_err(String::from)?;
    state.db.list_photos(&project_id).map_err(Into::into)
}

#[tauri::command]
pub fn create_project_from_folder(
    request: CreateProjectRequest,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<ScanSummary, String> {
    let config = crate::config::load_app_config(&state.app_data_dir)
        .map_err(String::from)?
        .config;
    let thumbnail_root = thumbnail_root(&state.app_data_dir);
    let summary = create_project_from_folder_inner(request, &state.db, &config, &thumbnail_root)
        .map_err(String::from)?;
    allow_asset_directory(&app, summary.project.folder_path.as_str()).map_err(String::from)?;
    allow_asset_directory(&app, &thumbnail_root).map_err(String::from)?;
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
    delete_project_inner(&project_id, &state.db, &thumbnail_root(&state.app_data_dir))
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
pub fn load_app_config(state: State<'_, AppState>) -> Result<AppConfigEnvelope, String> {
    crate::config::load_app_config(&state.app_data_dir).map_err(Into::into)
}

#[tauri::command]
pub fn save_app_config(
    config: AppConfig,
    state: State<'_, AppState>,
) -> Result<AppConfigEnvelope, String> {
    crate::config::save_app_config(&state.app_data_dir, &config).map_err(Into::into)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadModelRequest {
    pub model_id: String,
    pub file_name: String,
    pub download_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadModelSummary {
    pub model_id: String,
    pub path: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDownloadProgress {
    pub model_id: String,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub percent: Option<u8>,
}

#[tauri::command]
pub async fn download_model(
    request: DownloadModelRequest,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<DownloadModelSummary, String> {
    let app_data_dir = state.app_data_dir.clone();
    let model_id = request.model_id.trim().to_string();
    if model_id.is_empty() {
        return Err("model id cannot be empty".to_string());
    }
    let cancellations = state.download_cancellations();
    let active_downloads = state.active_downloads();
    mark_model_download_active(&active_downloads, &cancellations, &model_id)
        .map_err(String::from)?;
    let cleanup_model_id = model_id.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = download_model_inner(request, &app_data_dir, &app, Arc::clone(&cancellations));
        let _ = clear_model_download_state(&active_downloads, &cancellations, &cleanup_model_id);
        result
    })
    .await
    .map_err(|error| format!("model download task failed: {error}"))?
    .map_err(Into::into)
}

#[tauri::command]
pub fn cancel_model_download(model_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let model_id = model_id.trim();
    if model_id.is_empty() {
        return Err("model id cannot be empty".to_string());
    }
    let mut cancellations = state
        .download_cancellations
        .lock()
        .map_err(|_| "model download cancellation state is unavailable".to_string())?;
    let active_downloads = state
        .active_downloads
        .lock()
        .map_err(|_| "model download state is unavailable".to_string())?;
    if !active_downloads.contains(model_id) {
        return Err(format!("no active model download: {model_id}"));
    }
    cancellations.insert(model_id.to_string());
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HuggingFaceVisionModel {
    pub id: String,
    pub repo_id: String,
    pub name: String,
    pub author: String,
    pub task: String,
    pub license: String,
    pub file_name: String,
    pub download_url: String,
    pub downloads: u64,
    pub likes: u64,
    pub last_modified: String,
    pub downloaded: bool,
    pub update_available: bool,
    pub local_path: Option<String>,
    #[serde(default)]
    pub partial_downloaded_bytes: u64,
    #[serde(default)]
    pub partial_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HuggingFaceCatalogRequest {
    pub offset: usize,
    pub limit: usize,
    pub refresh: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HuggingFaceCatalogPage {
    pub models: Vec<HuggingFaceVisionModel>,
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
    pub has_more: bool,
    pub cache_date: String,
    pub refreshed: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HuggingFaceModelResponse {
    id: Option<String>,
    model_id: Option<String>,
    author: Option<String>,
    downloads: Option<u64>,
    likes: Option<u64>,
    last_modified: Option<String>,
    pipeline_tag: Option<String>,
    tags: Option<Vec<String>>,
    siblings: Option<Vec<HuggingFaceSibling>>,
}

#[derive(Debug, Deserialize)]
struct HuggingFaceSibling {
    rfilename: String,
}

#[tauri::command]
pub async fn list_huggingface_vision_models(
    request: HuggingFaceCatalogRequest,
    state: State<'_, AppState>,
) -> Result<HuggingFaceCatalogPage, String> {
    let app_data_dir = state.app_data_dir.clone();
    tauri::async_runtime::spawn_blocking(move || {
        list_huggingface_vision_models_inner(&app_data_dir, request)
    })
    .await
    .map_err(|error| format!("model catalog task failed: {error}"))?
    .map_err(Into::into)
}

pub fn start_huggingface_catalog_refresh(app_data_dir: PathBuf) {
    thread::spawn(move || {
        if let Err(error) = refresh_huggingface_catalog_if_needed(&app_data_dir) {
            eprintln!("Unable to refresh Hugging Face model catalog cache: {error}");
        }
    });
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
        crate::export::export_project(&state.db, &project_id, &state.app_data_dir)
            .map_err(Into::into)
    }
}

fn download_model_inner(
    request: DownloadModelRequest,
    app_data_dir: &Path,
    app: &tauri::AppHandle,
    cancellations: Arc<Mutex<HashSet<String>>>,
) -> AppResult<DownloadModelSummary> {
    let model_id = request.model_id.trim().to_string();
    if model_id.is_empty() {
        return Err(AppError::InvalidModelDownload(
            "model id cannot be empty".to_string(),
        ));
    }
    let download_url = request.download_url.trim();
    if !(download_url.starts_with("https://") || download_url.starts_with("http://")) {
        return Err(AppError::InvalidModelDownload(
            "download URL must start with http:// or https://".to_string(),
        ));
    }

    let file_name = Path::new(request.file_name.trim())
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .ok_or_else(|| AppError::InvalidModelDownload("file name is invalid".to_string()))?;

    let models_dir = app_data_dir.join("models");
    fs::create_dir_all(&models_dir)?;
    let target_path = models_dir.join(file_name);
    let temp_path = models_dir.join(format!("{file_name}.download"));

    let client = reqwest::blocking::Client::builder()
        .user_agent("Cullify/0.1 model downloader")
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(60))
        .build()?;
    let resume_from = partial_download_size(&temp_path)?;
    let mut response = send_model_download_request(&client, download_url, resume_from)?;
    let mut bytes = resume_from;
    let mut append_to_partial = false;

    match response.status() {
        StatusCode::PARTIAL_CONTENT if resume_from > 0 => {
            append_to_partial = true;
        }
        StatusCode::OK => {
            bytes = 0;
        }
        StatusCode::RANGE_NOT_SATISFIABLE if resume_from > 0 => {
            let _ = fs::remove_file(&temp_path);
            bytes = 0;
            response = send_model_download_request(&client, download_url, 0)?;
        }
        _ => {}
    }

    let mut response = response.error_for_status()?;
    let total_bytes = if append_to_partial {
        response
            .headers()
            .get(CONTENT_RANGE)
            .and_then(parse_content_range_total)
            .or_else(|| response.content_length().map(|remaining| remaining + bytes))
    } else {
        response.content_length()
    };

    let mut output = if append_to_partial {
        fs::OpenOptions::new().append(true).open(&temp_path)?
    } else {
        fs::File::create(&temp_path)?
    };

    let mut buffer = vec![0_u8; 1024 * 1024];
    let mut last_progress_emit = Instant::now();

    emit_model_download_progress(app, &model_id, bytes, total_bytes);
    loop {
        if is_model_download_cancelled(&cancellations, &model_id)? {
            output.flush()?;
            clear_model_download_cancellation(&cancellations, &model_id)?;
            return Err(AppError::ModelDownloadCancelled(model_id));
        }

        let read = response.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        output.write_all(&buffer[..read])?;
        bytes += read as u64;

        let is_complete = total_bytes
            .map(|total| total > 0 && bytes >= total)
            .unwrap_or(false);
        if is_complete || last_progress_emit.elapsed() >= Duration::from_millis(250) {
            emit_model_download_progress(app, &model_id, bytes, total_bytes);
            last_progress_emit = Instant::now();
        }
    }
    output.flush()?;
    if is_model_download_cancelled(&cancellations, &model_id)? {
        clear_model_download_cancellation(&cancellations, &model_id)?;
        return Err(AppError::ModelDownloadCancelled(model_id));
    }
    emit_model_download_progress(app, &model_id, bytes, total_bytes);
    fs::rename(&temp_path, &target_path)?;

    Ok(DownloadModelSummary {
        model_id,
        path: target_path.to_string_lossy().to_string(),
        bytes,
    })
}

fn send_model_download_request(
    client: &reqwest::blocking::Client,
    download_url: &str,
    resume_from: u64,
) -> Result<reqwest::blocking::Response, reqwest::Error> {
    let request = client.get(download_url);
    if resume_from > 0 {
        request
            .header(RANGE, format!("bytes={resume_from}-"))
            .send()
    } else {
        request.send()
    }
}

fn partial_download_size(temp_path: &Path) -> AppResult<u64> {
    match fs::metadata(temp_path) {
        Ok(metadata) if metadata.is_file() => Ok(metadata.len()),
        Ok(_) => Ok(0),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(0),
        Err(error) => Err(error.into()),
    }
}

fn parse_content_range_total(value: &reqwest::header::HeaderValue) -> Option<u64> {
    let value = value.to_str().ok()?;
    let (_, total) = value.rsplit_once('/')?;
    if total == "*" {
        return None;
    }
    total.parse::<u64>().ok()
}

fn is_model_download_cancelled(
    cancellations: &Arc<Mutex<HashSet<String>>>,
    model_id: &str,
) -> AppResult<bool> {
    let cancellations = cancellations.lock().map_err(|_| {
        AppError::InvalidModelDownload(
            "model download cancellation state is unavailable".to_string(),
        )
    })?;
    Ok(cancellations.contains(model_id))
}

fn mark_model_download_active(
    active_downloads: &Arc<Mutex<HashSet<String>>>,
    cancellations: &Arc<Mutex<HashSet<String>>>,
    model_id: &str,
) -> AppResult<()> {
    let mut cancellations = cancellations.lock().map_err(|_| {
        AppError::InvalidModelDownload(
            "model download cancellation state is unavailable".to_string(),
        )
    })?;
    cancellations.remove(model_id);

    let mut active_downloads = active_downloads.lock().map_err(|_| {
        AppError::InvalidModelDownload("model download state is unavailable".to_string())
    })?;
    if active_downloads.contains(model_id) {
        return Err(AppError::InvalidModelDownload(format!(
            "model download already active: {model_id}"
        )));
    }
    active_downloads.insert(model_id.to_string());
    Ok(())
}

fn clear_model_download_cancellation(
    cancellations: &Arc<Mutex<HashSet<String>>>,
    model_id: &str,
) -> AppResult<()> {
    let mut cancellations = cancellations.lock().map_err(|_| {
        AppError::InvalidModelDownload(
            "model download cancellation state is unavailable".to_string(),
        )
    })?;
    cancellations.remove(model_id);
    Ok(())
}

fn clear_model_download_state(
    active_downloads: &Arc<Mutex<HashSet<String>>>,
    cancellations: &Arc<Mutex<HashSet<String>>>,
    model_id: &str,
) -> AppResult<()> {
    clear_model_download_cancellation(cancellations, model_id)?;
    let mut active_downloads = active_downloads.lock().map_err(|_| {
        AppError::InvalidModelDownload("model download state is unavailable".to_string())
    })?;
    active_downloads.remove(model_id);
    Ok(())
}

fn emit_model_download_progress(
    app: &tauri::AppHandle,
    model_id: &str,
    downloaded_bytes: u64,
    total_bytes: Option<u64>,
) {
    let percent = total_bytes
        .filter(|total| *total > 0)
        .map(|total| ((downloaded_bytes.saturating_mul(100) / total).min(100)) as u8);

    let _ = app.emit(
        MODEL_DOWNLOAD_PROGRESS_EVENT,
        ModelDownloadProgress {
            model_id: model_id.to_string(),
            downloaded_bytes,
            total_bytes,
            percent,
        },
    );
}

fn list_huggingface_vision_models_inner(
    app_data_dir: &Path,
    request: HuggingFaceCatalogRequest,
) -> AppResult<HuggingFaceCatalogPage> {
    cleanup_old_huggingface_catalog_caches(app_data_dir)?;
    let refreshed = request.refresh;
    if request.refresh {
        let remote_models = fetch_huggingface_vision_models()?;
        save_huggingface_catalog_cache(app_data_dir, &remote_models)?;
    }

    let mut models = load_huggingface_catalog_cache(app_data_dir).unwrap_or_default();
    overlay_local_model_status(&mut models, app_data_dir)?;

    let total = models.len();
    let limit = request.limit.clamp(1, 50);
    let offset = request.offset.min(total);
    let end = (offset + limit).min(total);
    Ok(HuggingFaceCatalogPage {
        models: models[offset..end].to_vec(),
        total,
        offset,
        limit,
        has_more: end < total,
        cache_date: today_cache_key(),
        refreshed,
    })
}

fn refresh_huggingface_catalog_if_needed(app_data_dir: &Path) -> AppResult<()> {
    cleanup_old_huggingface_catalog_caches(app_data_dir)?;
    let cache_path = huggingface_catalog_cache_path(app_data_dir);
    if cache_path.exists() {
        return Ok(());
    }
    let models = fetch_huggingface_vision_models()?;
    save_huggingface_catalog_cache(app_data_dir, &models)
}

fn fetch_huggingface_vision_models() -> AppResult<Vec<HuggingFaceVisionModel>> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("Cullify/0.1 model catalog")
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(20))
        .build()?;
    let endpoints = [
        "https://huggingface.co/api/models?filter=image-text-to-text&full=true&sort=downloads&direction=-1&limit=200",
        "https://huggingface.co/api/models?filter=image-to-text&full=true&sort=downloads&direction=-1&limit=200",
        "https://huggingface.co/api/models?search=vision%20gguf&full=true&sort=downloads&direction=-1&limit=200",
        "https://huggingface.co/api/models?search=llava%20gguf&full=true&sort=downloads&direction=-1&limit=200",
        "https://huggingface.co/api/models?search=vl%20gguf&full=true&sort=downloads&direction=-1&limit=200",
    ];

    let mut models = Vec::new();
    for endpoint in endpoints {
        let response_text = client.get(endpoint).send()?.error_for_status()?.text()?;
        let response = serde_json::from_str::<Vec<HuggingFaceModelResponse>>(&response_text)?;
        for model in response {
            if let Some(catalog_model) = catalog_model_from_response(model) {
                models.push(catalog_model);
            }
        }
    }

    let mut deduped = HashMap::<String, HuggingFaceVisionModel>::new();
    for model in models {
        deduped
            .entry(format!("{}:{}", model.repo_id, model.file_name))
            .and_modify(|existing| {
                if model.downloads > existing.downloads {
                    *existing = model.clone();
                }
            })
            .or_insert(model);
    }

    let mut models = deduped.into_values().collect::<Vec<_>>();
    models.sort_by(|left, right| {
        right
            .downloads
            .cmp(&left.downloads)
            .then_with(|| left.repo_id.cmp(&right.repo_id))
    });
    models.truncate(200);
    Ok(models)
}

fn catalog_model_from_response(model: HuggingFaceModelResponse) -> Option<HuggingFaceVisionModel> {
    let repo_id = model.model_id.or(model.id)?;
    let tags = model.tags.unwrap_or_default();
    let siblings = model.siblings.unwrap_or_default();
    let file_name = preferred_gguf_file(&siblings)?;
    if !is_supported_vision_model(&repo_id, &tags, model.pipeline_tag.as_deref()) {
        return None;
    }

    let file_basename = Path::new(&file_name)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&file_name)
        .to_string();

    Some(HuggingFaceVisionModel {
        id: local_model_id_from_repo(&repo_id, &file_basename),
        name: display_name_from_repo(&repo_id),
        author: model.author.unwrap_or_else(|| {
            repo_id
                .split('/')
                .next()
                .unwrap_or("huggingface")
                .to_string()
        }),
        task: model
            .pipeline_tag
            .or_else(|| {
                tags.iter()
                    .find(|tag| tag.contains("image") || *tag == "vision")
                    .cloned()
            })
            .unwrap_or_else(|| "vision-language".to_string()),
        license: tags
            .iter()
            .find_map(|tag| tag.strip_prefix("license:").map(str::to_string))
            .unwrap_or_else(|| "未标注".to_string()),
        file_name: file_basename,
        download_url: format!(
            "https://huggingface.co/{}/resolve/main/{}",
            repo_id,
            encode_huggingface_path(&file_name)
        ),
        repo_id,
        downloads: model.downloads.unwrap_or(0),
        likes: model.likes.unwrap_or(0),
        last_modified: model.last_modified.unwrap_or_else(|| "unknown".to_string()),
        downloaded: false,
        update_available: false,
        local_path: None,
        partial_downloaded_bytes: 0,
        partial_path: None,
    })
}

fn overlay_local_model_status(
    models: &mut [HuggingFaceVisionModel],
    app_data_dir: &Path,
) -> AppResult<()> {
    let local_files = local_model_file_index(&app_data_dir.join("models"))?;
    let partial_files = partial_model_file_index(&app_data_dir.join("models"))?;
    for model in models {
        let local = local_files.get(&model.file_name.to_lowercase());
        let partial = partial_files.get(&model.file_name.to_lowercase());
        let remote_modified = DateTime::parse_from_rfc3339(&model.last_modified)
            .ok()
            .map(|value| value.with_timezone(&Utc));
        model.downloaded = local.is_some();
        model.local_path = local.map(|file| file.path.clone());
        model.partial_downloaded_bytes = partial.map(|file| file.bytes).unwrap_or(0);
        model.partial_path = partial.map(|file| file.path.clone());
        model.update_available = match (local, remote_modified) {
            (Some(local), Some(remote)) => local.modified < remote,
            _ => false,
        };
    }
    Ok(())
}

fn huggingface_catalog_cache_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("cache").join("huggingface-models")
}

fn huggingface_catalog_cache_path(app_data_dir: &Path) -> PathBuf {
    huggingface_catalog_cache_dir(app_data_dir).join(format!("{}.json", today_cache_key()))
}

fn today_cache_key() -> String {
    Utc::now().format("%Y-%m-%d").to_string()
}

fn load_huggingface_catalog_cache(app_data_dir: &Path) -> AppResult<Vec<HuggingFaceVisionModel>> {
    let path = huggingface_catalog_cache_path(app_data_dir);
    let cache = fs::read_to_string(path)?;
    serde_json::from_str(&cache).map_err(Into::into)
}

fn save_huggingface_catalog_cache(
    app_data_dir: &Path,
    models: &[HuggingFaceVisionModel],
) -> AppResult<()> {
    let cache_dir = huggingface_catalog_cache_dir(app_data_dir);
    fs::create_dir_all(&cache_dir)?;
    fs::write(
        huggingface_catalog_cache_path(app_data_dir),
        serde_json::to_string_pretty(models)?,
    )?;
    Ok(())
}

fn cleanup_old_huggingface_catalog_caches(app_data_dir: &Path) -> AppResult<()> {
    let cache_dir = huggingface_catalog_cache_dir(app_data_dir);
    if !cache_dir.exists() {
        return Ok(());
    }
    let today = format!("{}.json", today_cache_key());
    for entry in fs::read_dir(cache_dir)? {
        let entry = entry?;
        if entry.file_name().to_str() != Some(today.as_str()) {
            let path = entry.path();
            if path.is_file() {
                fs::remove_file(path)?;
            }
        }
    }
    Ok(())
}

fn preferred_gguf_file(siblings: &[HuggingFaceSibling]) -> Option<String> {
    let mut files = siblings
        .iter()
        .map(|sibling| sibling.rfilename.as_str())
        .filter(|file| file.to_lowercase().ends_with(".gguf"))
        .filter(|file| !file.to_lowercase().contains("mmproj"))
        .collect::<Vec<_>>();
    files.sort_by_key(|file| {
        let lower = file.to_lowercase();
        if lower.contains("q4_k_m") {
            0
        } else if lower.contains("q5_k_m") {
            1
        } else if lower.contains("q8_0") {
            2
        } else if lower.contains("q4") {
            3
        } else {
            9
        }
    });
    files.first().map(|file| (*file).to_string())
}

fn is_supported_vision_model(repo_id: &str, tags: &[String], pipeline_tag: Option<&str>) -> bool {
    let searchable = format!(
        "{} {}",
        repo_id.to_lowercase(),
        tags.join(" ").to_lowercase()
    );
    let has_gguf = tags.iter().any(|tag| tag == "gguf") || repo_id.to_lowercase().contains("gguf");
    let visual = pipeline_tag
        .map(|tag| tag.contains("image"))
        .unwrap_or(false)
        || searchable.contains("image-text-to-text")
        || searchable.contains("image-to-text")
        || searchable.contains("vision")
        || searchable.contains("multimodal")
        || searchable.contains("llava")
        || searchable.contains("-vl")
        || searchable.contains("vl-");
    has_gguf && visual
}

#[derive(Debug)]
struct LocalModelFile {
    path: String,
    modified: DateTime<Utc>,
}

#[derive(Debug)]
struct PartialModelFile {
    path: String,
    bytes: u64,
}

fn local_model_file_index(models_dir: &Path) -> AppResult<HashMap<String, LocalModelFile>> {
    let mut files = HashMap::new();
    if !models_dir.exists() {
        return Ok(files);
    }
    for entry in fs::read_dir(models_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("gguf") {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let modified = entry
            .metadata()?
            .modified()
            .map(DateTime::<Utc>::from)
            .unwrap_or_else(|_| Utc::now());
        files.insert(
            file_name.to_lowercase(),
            LocalModelFile {
                path: path.to_string_lossy().to_string(),
                modified,
            },
        );
    }
    Ok(files)
}

fn partial_model_file_index(models_dir: &Path) -> AppResult<HashMap<String, PartialModelFile>> {
    let mut files = HashMap::new();
    if !models_dir.exists() {
        return Ok(files);
    }
    for entry in fs::read_dir(models_dir)? {
        let entry = entry?;
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(model_file_name) = file_name.strip_suffix(".download") else {
            continue;
        };
        if !model_file_name.to_lowercase().ends_with(".gguf") {
            continue;
        }
        let metadata = entry.metadata()?;
        if !metadata.is_file() || metadata.len() == 0 {
            continue;
        }
        files.insert(
            model_file_name.to_lowercase(),
            PartialModelFile {
                path: path.to_string_lossy().to_string(),
                bytes: metadata.len(),
            },
        );
    }
    Ok(files)
}

fn local_model_id_from_repo(repo_id: &str, file_name: &str) -> String {
    format!("{}-{}", repo_id, file_name)
        .to_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn display_name_from_repo(repo_id: &str) -> String {
    repo_id
        .split('/')
        .next_back()
        .unwrap_or(repo_id)
        .trim_end_matches("-GGUF")
        .trim_end_matches("-gguf")
        .replace(['-', '_'], " ")
}

fn encode_huggingface_path(path: &str) -> String {
    path.bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
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

fn thumbnail_root(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("thumbnails")
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
    fn cached_huggingface_catalog_is_paginated_without_network() {
        let root = unique_temp_dir("hf-catalog-page");
        fs::create_dir_all(&root).unwrap();
        let models = (0..25)
            .map(|index| test_catalog_model(index))
            .collect::<Vec<_>>();
        save_huggingface_catalog_cache(&root, &models).unwrap();

        let first_page = list_huggingface_vision_models_inner(
            &root,
            HuggingFaceCatalogRequest {
                offset: 0,
                limit: 20,
                refresh: false,
            },
        )
        .unwrap();

        assert_eq!(first_page.models.len(), 20);
        assert_eq!(first_page.total, 25);
        assert_eq!(first_page.offset, 0);
        assert!(first_page.has_more);
        assert!(!first_page.refreshed);

        let second_page = list_huggingface_vision_models_inner(
            &root,
            HuggingFaceCatalogRequest {
                offset: 20,
                limit: 20,
                refresh: false,
            },
        )
        .unwrap();

        assert_eq!(second_page.models.len(), 5);
        assert!(!second_page.has_more);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn old_huggingface_catalog_cache_files_are_removed() {
        let root = unique_temp_dir("hf-cache-cleanup");
        let cache_dir = huggingface_catalog_cache_dir(&root);
        fs::create_dir_all(&cache_dir).unwrap();
        let today = cache_dir.join(format!("{}.json", today_cache_key()));
        let stale = cache_dir.join("2024-01-01.json");
        fs::write(&today, "[]").unwrap();
        fs::write(&stale, "[]").unwrap();

        cleanup_old_huggingface_catalog_caches(&root).unwrap();

        assert!(today.exists());
        assert!(!stale.exists());

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

    fn test_catalog_model(index: usize) -> HuggingFaceVisionModel {
        HuggingFaceVisionModel {
            id: format!("model-{index}"),
            repo_id: format!("org/model-{index}"),
            name: format!("Model {index}"),
            author: "org".to_string(),
            task: "image-text-to-text".to_string(),
            license: "mit".to_string(),
            file_name: format!("model-{index}.gguf"),
            download_url: format!(
                "https://huggingface.co/org/model-{index}/resolve/main/model-{index}.gguf"
            ),
            downloads: index as u64,
            likes: 0,
            last_modified: "unknown".to_string(),
            downloaded: false,
            update_available: false,
            local_path: None,
            partial_downloaded_bytes: 0,
            partial_path: None,
        }
    }
}
