use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub hotkey: HotkeyConfig,
    pub stt: SttConfig,
    pub audio: AudioConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct HotkeyConfig {
    /// Modifier keys: "super", "ctrl", "alt", "shift"
    #[serde(default = "default_modifiers")]
    pub modifiers: Vec<String>,
    /// Key name (evdev key name, e.g. "space")
    #[serde(default = "default_key")]
    pub key: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SttConfig {
    /// Deepgram API key
    pub api_key: Option<String>,
    /// Deepgram model (default: nova-2)
    #[serde(default = "default_model")]
    pub model: String,
    /// Language code (e.g. "en", "fr", "auto")
    #[serde(default = "default_language")]
    pub language: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AudioConfig {
    /// Sample rate (default: 16000)
    #[serde(default = "default_sample_rate")]
    pub sample_rate: u32,
    /// Channels (default: 1)
    #[serde(default = "default_channels")]
    pub channels: u16,
}

fn default_modifiers() -> Vec<String> {
    vec!["ctrl".into()]
}
fn default_key() -> String {
    "space".into()
}
fn default_model() -> String {
    "nova-2-general".into()
}
fn default_language() -> String {
    "en".into()
}
fn default_sample_rate() -> u32 {
    16000
}
fn default_channels() -> u16 {
    1
}

impl Default for Config {
    fn default() -> Self {
        let toml_str = include_str!("../default_config.toml");
        toml::from_str(toml_str).expect("default config must be valid")
    }
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = Self::config_path();
        let mut cfg = if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let cfg: Config = toml::from_str(&content)?;
            log::info!("Loaded config from {}", config_path.display());
            cfg
        } else {
            log::info!("No config found, using defaults");
            Self::default()
        };

        // Env var fallback for API key (env takes precedence)
        if let Ok(key) = std::env::var("DEEPGRAM_API_KEY") {
            cfg.stt.api_key = Some(key);
        }

        Ok(cfg)
    }

    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("whisp-rs")
            .join("config.toml")
    }
}
