use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::AppResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AppConfig {
    pub provider_id: String,
    pub active_model_id: String,
    pub blur_threshold: f64,
    pub exposure_tolerance: f64,
    pub cull_line: u8,
    pub vlm_threads: u8,
    pub arena_target: String,
    pub auto_group: bool,
    pub gpu_metal: bool,
    pub shortcuts: Vec<ShortcutConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutConfig {
    pub id: String,
    pub keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfigEnvelope {
    pub config: AppConfig,
    pub config_path: String,
    pub app_data_dir: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            provider_id: "builtin".to_string(),
            active_model_id: "gemma-3-4b".to_string(),
            blur_threshold: 100.0,
            exposure_tolerance: 0.018,
            cull_line: 40,
            vlm_threads: 3,
            arena_target: "20%".to_string(),
            auto_group: true,
            gpu_metal: true,
            shortcuts: default_shortcuts(),
        }
    }
}

pub fn load_app_config(app_data_dir: &Path) -> AppResult<AppConfigEnvelope> {
    let path = config_path(app_data_dir);
    let config = load_config_from_path(&path)?;
    Ok(envelope(config, app_data_dir, path))
}

pub fn save_app_config(app_data_dir: &Path, config: &AppConfig) -> AppResult<AppConfigEnvelope> {
    let path = config_path(app_data_dir);
    save_config_to_path(&path, config)?;
    Ok(envelope(config.clone(), app_data_dir, path))
}

fn load_config_from_path(path: &Path) -> AppResult<AppConfig> {
    if !path.exists() {
        return Ok(AppConfig::default());
    }

    toml::from_str::<AppConfig>(&fs::read_to_string(path)?).map_err(Into::into)
}

fn save_config_to_path(path: &Path, config: &AppConfig) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let serialized = toml::to_string_pretty(config)?;
    fs::write(path, serialized)?;
    Ok(())
}

fn config_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("config.toml")
}

fn envelope(config: AppConfig, app_data_dir: &Path, path: PathBuf) -> AppConfigEnvelope {
    AppConfigEnvelope {
        config,
        config_path: path.to_string_lossy().to_string(),
        app_data_dir: app_data_dir.to_string_lossy().to_string(),
    }
}

fn default_shortcuts() -> Vec<ShortcutConfig> {
    [
        ("keep", vec!["K"]),
        ("cull", vec!["X"]),
        ("nav", vec!["Left", "Right"]),
        ("skip", vec!["S"]),
        ("arena", vec!["A", "D"]),
        ("mark", vec!["Space"]),
        ("fullscreen", vec!["F"]),
        ("grid", vec!["G"]),
        ("undo", vec!["Cmd", "Z"]),
        ("all", vec!["Up", "Down"]),
    ]
    .into_iter()
    .map(|(id, keys)| ShortcutConfig {
        id: id.to_string(),
        keys: keys.into_iter().map(str::to_string).collect(),
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_roundtrips_as_toml() {
        let path =
            std::env::temp_dir().join(format!("cullify-config-{}.toml", uuid::Uuid::new_v4()));
        let mut config = AppConfig::default();
        config.provider_id = "ollama".to_string();
        config.active_model_id = "qwen-2-5-vl-7b".to_string();
        config.blur_threshold = 135.0;
        config.auto_group = false;
        config.shortcuts[0].keys = vec!["P".to_string()];

        save_config_to_path(&path, &config).expect("config should save");
        let loaded = load_config_from_path(&path).expect("config should load");
        let _ = fs::remove_file(path);

        assert_eq!(loaded.provider_id, "ollama");
        assert_eq!(loaded.active_model_id, "qwen-2-5-vl-7b");
        assert_eq!(loaded.blur_threshold, 135.0);
        assert!(!loaded.auto_group);
        assert_eq!(loaded.shortcuts[0].keys, ["P"]);
    }

    #[test]
    fn missing_config_uses_defaults() {
        let path = std::env::temp_dir().join(format!(
            "cullify-missing-config-{}.toml",
            uuid::Uuid::new_v4()
        ));
        let loaded = load_config_from_path(&path).expect("missing config should default");

        assert_eq!(loaded.provider_id, "builtin");
        assert_eq!(loaded.active_model_id, "gemma-3-4b");
        assert_eq!(loaded.shortcuts.len(), 10);
    }
}
