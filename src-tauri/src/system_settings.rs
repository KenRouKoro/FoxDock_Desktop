use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// 与 `tauri.conf.json` 的 `identifier` 对齐，用于应用配置目录名
const APP_CONFIG_DIR_NAME: &str = "com.foxapplication.foxdock";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemSettings {
    #[serde(default)]
    pub language_preference: LanguagePreference,
    #[serde(default)]
    pub debug_enabled: bool,
    #[serde(default = "default_auto_check_update")]
    pub auto_check_update: bool,
    #[serde(default = "default_auto_dock_on_startup")]
    pub auto_dock_on_startup: bool,
    #[serde(default)]
    pub dock_always_on_top: bool,
    #[serde(default)]
    pub follow_slime_vr_window: bool,
    #[serde(default)]
    pub snap_style_approximation: bool,
}

impl Default for SystemSettings {
    fn default() -> Self {
        Self {
            language_preference: LanguagePreference::default(),
            debug_enabled: false,
            auto_check_update: default_auto_check_update(),
            auto_dock_on_startup: default_auto_dock_on_startup(),
            dock_always_on_top: false,
            follow_slime_vr_window: false,
            snap_style_approximation: false,
        }
    }
}

fn default_auto_check_update() -> bool {
    true
}

fn default_auto_dock_on_startup() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum LanguagePreference {
    #[default]
    System,
    Zh,
    En,
}

fn settings_path() -> Result<PathBuf, String> {
    let base = dirs::config_dir().ok_or_else(|| "config directory not available".to_string())?;
    let dir = base.join(APP_CONFIG_DIR_NAME);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("system_settings.json"))
}

pub fn load_system_settings_disk() -> SystemSettings {
    let path = match settings_path() {
        Ok(p) => p,
        Err(_) => return SystemSettings::default(),
    };
    let data = match fs::read_to_string(&path) {
        Ok(d) => d,
        Err(_) => return SystemSettings::default(),
    };
    serde_json::from_str::<SystemSettings>(&data).unwrap_or_default()
}

#[tauri::command]
pub fn load_system_settings() -> SystemSettings {
    load_system_settings_disk()
}

#[tauri::command]
pub fn save_system_settings(settings: SystemSettings) -> Result<(), String> {
    let path = settings_path()?;
    let data = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    fs::write(path, data).map_err(|e| e.to_string())
}
