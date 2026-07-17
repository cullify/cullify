use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::AppResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AppConfig {
    pub provider_id: String,
    pub active_model_id: String,
    pub active_model_provider_id: String,
    pub active_local_model_id: String,
    pub local_models: Vec<LocalModelConfig>,
    pub third_party_providers: Vec<ThirdPartyProviderConfig>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LocalModelConfig {
    pub id: String,
    pub name: String,
    pub file_name: String,
    pub size: String,
    pub speed: String,
    pub download_url: String,
    pub local_path: Option<String>,
    pub downloaded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ThirdPartyProviderConfig {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub kind: String,
    pub enabled: bool,
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
            active_model_provider_id: "llama.cpp".to_string(),
            active_local_model_id: "gemma-3-4b".to_string(),
            local_models: default_local_models(),
            third_party_providers: default_third_party_providers(),
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

impl Default for LocalModelConfig {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            file_name: String::new(),
            size: String::new(),
            speed: String::new(),
            download_url: String::new(),
            local_path: None,
            downloaded: false,
        }
    }
}

impl Default for ThirdPartyProviderConfig {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            base_url: String::new(),
            api_key: String::new(),
            model: String::new(),
            kind: "openai-compatible".to_string(),
            enabled: true,
        }
    }
}

pub fn load_app_config(app_data_dir: &Path) -> AppResult<AppConfigEnvelope> {
    let path = config_path(app_data_dir);
    let config = sync_local_model_files(load_config_from_path(&path)?, app_data_dir)?;
    Ok(envelope(config, app_data_dir, path))
}

pub fn save_app_config(app_data_dir: &Path, config: &AppConfig) -> AppResult<AppConfigEnvelope> {
    let path = config_path(app_data_dir);
    let config = sync_local_model_files(normalize_config(config.clone()), app_data_dir)?;
    save_config_to_path(&path, &config)?;
    Ok(envelope(config, app_data_dir, path))
}

fn load_config_from_path(path: &Path) -> AppResult<AppConfig> {
    if !path.exists() {
        return Ok(AppConfig::default());
    }

    toml::from_str::<AppConfig>(&fs::read_to_string(path)?)
        .map(normalize_config)
        .map_err(Into::into)
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

fn normalize_config(mut config: AppConfig) -> AppConfig {
    if config.active_model_provider_id.is_empty() {
        config.active_model_provider_id = match config.provider_id.as_str() {
            "ollama" => "ollama-local".to_string(),
            "openai" => "openai-compatible".to_string(),
            _ => "llama.cpp".to_string(),
        };
    }
    if config.active_local_model_id.is_empty() {
        config.active_local_model_id = config.active_model_id.clone();
    }
    if config.local_models.is_empty() {
        config.local_models = default_local_models();
    }
    if config.third_party_providers.is_empty() {
        config.third_party_providers = default_third_party_providers();
    }

    if config.active_model_provider_id == "llama.cpp" {
        config.provider_id = "builtin".to_string();
        config.active_model_id = config.active_local_model_id.clone();
    } else {
        config.provider_id = config.active_model_provider_id.clone();
        if let Some(provider) = config
            .third_party_providers
            .iter()
            .find(|provider| provider.id == config.active_model_provider_id)
        {
            config.active_model_id = provider.model.clone();
        }
    }

    config
}

fn sync_local_model_files(mut config: AppConfig, app_data_dir: &Path) -> AppResult<AppConfig> {
    let models_dir = app_data_dir.join("models");
    if !models_dir.exists() {
        return Ok(config);
    }

    let mut discovered = Vec::new();
    for entry in fs::read_dir(&models_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("gguf") {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        discovered.push((file_name.to_string(), path.to_string_lossy().to_string()));
    }

    for model in &mut config.local_models {
        if let Some((_, path)) = discovered
            .iter()
            .find(|(file_name, _)| file_name.eq_ignore_ascii_case(&model.file_name))
        {
            model.downloaded = true;
            model.local_path = Some(path.clone());
        }
    }

    let existing_files = config
        .local_models
        .iter()
        .map(|model| model.file_name.to_lowercase())
        .collect::<Vec<_>>();
    for (file_name, path) in discovered {
        if existing_files
            .iter()
            .any(|existing| existing == &file_name.to_lowercase())
        {
            continue;
        }
        let id = local_model_id_from_file(&file_name);
        config.local_models.push(LocalModelConfig {
            id,
            name: display_name_from_file(&file_name),
            file_name,
            size: "本地文件".to_string(),
            speed: "已识别".to_string(),
            download_url: String::new(),
            local_path: Some(path),
            downloaded: true,
        });
    }

    if !config
        .local_models
        .iter()
        .any(|model| model.id == config.active_local_model_id)
    {
        config.active_local_model_id = config
            .local_models
            .first()
            .map(|model| model.id.clone())
            .unwrap_or_else(|| "gemma-3-4b".to_string());
    }

    Ok(normalize_config(config))
}

fn local_model_id_from_file(file_name: &str) -> String {
    let stem = Path::new(file_name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("local-model");
    let id = stem
        .to_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    if id.is_empty() {
        "local-model".to_string()
    } else {
        id
    }
}

fn display_name_from_file(file_name: &str) -> String {
    Path::new(file_name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(file_name)
        .replace(['-', '_'], " ")
}

fn default_local_models() -> Vec<LocalModelConfig> {
    [
        (
            "gemma-3-4b",
            "Gemma 3 4B Vision",
            "gemma-3-4b-it-Q4_K_M.gguf",
            "3.2 GB",
            "本地 · 快",
            "https://huggingface.co/ggml-org/gemma-3-4b-it-GGUF/resolve/main/gemma-3-4b-it-Q4_K_M.gguf",
        ),
        (
            "qwen-2-5-vl-7b",
            "Qwen2.5-VL 7B",
            "Qwen2.5-VL-7B-Instruct-Q4_K_M.gguf",
            "5.4 GB",
            "本地 · 均衡",
            "https://huggingface.co/unsloth/Qwen2.5-VL-7B-Instruct-GGUF/resolve/main/Qwen2.5-VL-7B-Instruct-Q4_K_M.gguf",
        ),
    ]
    .into_iter()
    .map(|(id, name, file_name, size, speed, download_url)| LocalModelConfig {
        id: id.to_string(),
        name: name.to_string(),
        file_name: file_name.to_string(),
        size: size.to_string(),
        speed: speed.to_string(),
        download_url: download_url.to_string(),
        local_path: None,
        downloaded: false,
    })
    .collect()
}

fn default_third_party_providers() -> Vec<ThirdPartyProviderConfig> {
    vec![
        ThirdPartyProviderConfig {
            id: "ollama-local".to_string(),
            name: "Ollama 本地服务".to_string(),
            base_url: "http://localhost:11434/v1".to_string(),
            api_key: String::new(),
            model: "llava:latest".to_string(),
            kind: "openai-compatible".to_string(),
            enabled: true,
        },
        ThirdPartyProviderConfig {
            id: "openai-compatible".to_string(),
            name: "OpenAI 兼容端点".to_string(),
            base_url: "http://localhost:1234/v1".to_string(),
            api_key: String::new(),
            model: String::new(),
            kind: "openai-compatible".to_string(),
            enabled: true,
        },
    ]
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
        config.active_model_provider_id = "ollama-local".to_string();
        config.third_party_providers[0].model = "qwen2.5vl:7b".to_string();
        config.blur_threshold = 135.0;
        config.auto_group = false;
        config.shortcuts[0].keys = vec!["P".to_string()];

        save_config_to_path(&path, &config).expect("config should save");
        let loaded = load_config_from_path(&path).expect("config should load");
        let _ = fs::remove_file(path);

        assert_eq!(loaded.provider_id, "ollama-local");
        assert_eq!(loaded.active_model_id, "qwen2.5vl:7b");
        assert_eq!(loaded.active_model_provider_id, "ollama-local");
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
        assert_eq!(loaded.active_model_provider_id, "llama.cpp");
        assert_eq!(loaded.local_models.len(), 2);
        assert_eq!(loaded.third_party_providers.len(), 2);
        assert_eq!(loaded.shortcuts.len(), 10);
    }

    #[test]
    fn app_data_models_directory_is_discovered() {
        let app_data_dir =
            std::env::temp_dir().join(format!("cullify-app-data-{}", uuid::Uuid::new_v4()));
        let models_dir = app_data_dir.join("models");
        fs::create_dir_all(&models_dir).unwrap();
        fs::write(models_dir.join("custom-vision-model.gguf"), "fake model").unwrap();
        fs::write(models_dir.join("gemma-3-4b-it-Q4_K_M.gguf"), "fake model").unwrap();

        let loaded = load_app_config(&app_data_dir)
            .expect("config should load and discover local models")
            .config;
        let _ = fs::remove_dir_all(app_data_dir);

        let gemma = loaded
            .local_models
            .iter()
            .find(|model| model.id == "gemma-3-4b")
            .expect("default model should exist");
        assert!(gemma.downloaded);
        assert!(
            gemma
                .local_path
                .as_deref()
                .unwrap()
                .ends_with("gemma-3-4b-it-Q4_K_M.gguf")
        );
        assert!(
            loaded
                .local_models
                .iter()
                .any(|model| model.id == "custom-vision-model" && model.downloaded)
        );
    }
}
