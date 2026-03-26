use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_notification_timeout")]
    pub notification_timeout: u64,
    #[serde(default = "default_font_size")]
    pub font_size: u16,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_shell")]
    pub shell: String,
    #[serde(default = "default_font_name")]
    pub font_name: String,
    #[serde(default)]
    pub keybindings: Keybindings,
    #[serde(default)]
    pub auto_close_on_exit: bool,
}

fn default_font_name() -> String { "Cascadia Mono".to_string() }
fn default_notification_timeout() -> u64 { 2 }
fn default_font_size() -> u16 { 14 }
fn default_theme() -> String { "dark".to_string() }
#[cfg(windows)]
fn default_shell() -> String { "pwsh.exe".to_string() }
#[cfg(not(windows))]
fn default_shell() -> String { std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string()) }

#[derive(Debug, Clone, Hash, Deserialize)]
pub struct Keybindings {
    #[serde(default = "default_split_h")]
    pub split_horizontal: String,
    #[serde(default = "default_split_v")]
    pub split_vertical: String,
    #[serde(default = "default_pane_next")]
    pub pane_next: String,
    #[serde(default = "default_pane_prev")]
    pub pane_prev: String,
    #[serde(default = "default_new_pane")]
    pub new_pane: String,
    #[serde(default = "default_close_pane")]
    pub close_pane: String,
}

fn default_split_h()    -> String { "ctrl+shift+h".to_string() }
fn default_split_v()    -> String { "ctrl+shift+v".to_string() }
fn default_pane_next()  -> String { "ctrl+tab".to_string() }
fn default_pane_prev()  -> String { "ctrl+shift+tab".to_string() }
fn default_new_pane()   -> String { "ctrl+shift+n".to_string() }
fn default_close_pane() -> String { "ctrl+shift+w".to_string() }

impl Default for Keybindings {
    fn default() -> Self {
        Self {
            split_horizontal: default_split_h(),
            split_vertical:   default_split_v(),
            pane_next:        default_pane_next(),
            pane_prev:        default_pane_prev(),
            new_pane:         default_new_pane(),
            close_pane:       default_close_pane(),
        }
    }
}

#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Parse(toml::de::Error),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ConfigError::Io(e)    => write!(f, "Config I/O error: {}", e),
            ConfigError::Parse(e) => write!(f, "Config parse error: {}", e),
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path).map_err(ConfigError::Io)?;
        let config: Config = toml::from_str(&content).map_err(ConfigError::Parse)?;
        Ok(config)
    }

    pub fn load_or_default() -> Result<Self, ConfigError> {
        let path = config_file_path();
        if path.exists() { Self::load(&path) } else { Ok(Self::default()) }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            notification_timeout: default_notification_timeout(),
            font_size:            default_font_size(),
            theme:                default_theme(),
            shell:                default_shell(),
            font_name:            default_font_name(),
            keybindings:          Keybindings::default(),
            auto_close_on_exit:   false,
        }
    }
}

fn config_file_path() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("termpp")
        .join("config.toml")
}
