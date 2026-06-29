use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub mode: String,
    pub status: String,
    pub folder_path: String,
    pub total_photos: i64,
    pub kept_count: i64,
    pub culled_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Photo {
    pub id: String,
    pub project_id: String,
    pub original_name: String,
    pub file_path: String,
    pub thumbnail_path: Option<String>,
    pub file_size: i64,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub mime_type: String,
    pub analysis_status: String,
    pub quality_score: Option<f64>,
    pub is_blurred: bool,
    pub blur_score: Option<f64>,
    pub is_overexposed: bool,
    pub is_underexposed: bool,
    pub exposure_score: Option<f64>,
    pub user_decision: Option<String>,
    pub is_auto_culled: bool,
    pub perceptual_hash: Option<String>,
    pub group_label: String,
    pub captured_at: Option<String>,
    pub camera: Option<String>,
    pub lens: Option<String>,
    pub focal: Option<String>,
    pub aperture: Option<String>,
    pub shutter: Option<String>,
    pub iso: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct NewPhoto {
    pub id: String,
    pub original_name: String,
    pub file_path: String,
    pub thumbnail_path: Option<String>,
    pub file_size: i64,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub mime_type: String,
    pub quality_score: f64,
    pub is_blurred: bool,
    pub blur_score: f64,
    pub is_overexposed: bool,
    pub is_underexposed: bool,
    pub exposure_score: f64,
    pub is_auto_culled: bool,
    pub perceptual_hash: String,
    pub group_label: String,
    pub captured_at: Option<String>,
    pub camera: Option<String>,
    pub lens: Option<String>,
    pub focal: Option<String>,
    pub aperture: Option<String>,
    pub shutter: Option<String>,
    pub iso: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanSummary {
    pub project: Project,
    pub inserted: usize,
    pub skipped: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectRequest {
    pub folder_path: String,
    pub name: Option<String>,
    pub mode: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchDecisionRequest {
    pub project_id: String,
    pub photo_ids: Vec<String>,
    pub decision: Option<String>,
}
