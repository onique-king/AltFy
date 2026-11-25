use serde_json::{json, Value};
use std::{fs, path::PathBuf, process::Command};
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use tauri::{AppHandle, Manager};

fn path(app: &AppHandle) -> Result<PathBuf, String> {
    let path = app.path().app_local_data_dir().map_err(|error| error.to_string())?.join("settings.json");
    if let Some(parent) = path.parent() { fs::create_dir_all(parent).map_err(|error| error.to_string())?; }
    Ok(path)
}
fn read(app: &AppHandle) -> Result<Value, String> {
    let path = path(app)?;
    if !path.exists() { return Ok(json!({})); }
    serde_json::from_slice(&fs::read(path).map_err(|error| error.to_string())?).map_err(|error| error.to_string())
}
#[tauri::command]
pub fn get_app_settings(app: AppHandle) -> Result<Value, String> { Ok(settings(&app)) }
#[tauri::command]
pub fn save_app_settings(app: AppHandle, settings: Value) -> Result<(), String> {
    fs::write(path(&app)?, serde_json::to_vec_pretty(&settings).map_err(|error| error.to_string())?).map_err(|error| error.to_string())?;
    apply_startup_settings(app)
}
#[tauri::command]
pub fn apply_startup_settings(app: AppHandle) -> Result<(), String> {
    let settings = read(&app)?;
    #[cfg(windows)] {
        let task = "AltFy";
        let enabled = settings["autostart"].as_bool().unwrap_or(false);
        let mut command = Command::new("schtasks.exe");
        command.creation_flags(0x08000000);
        if enabled {
            let executable = std::env::current_exe().map_err(|error| error.to_string())?;
            command.args(["/Create", "/F", "/SC", "ONLOGON", "/TN", task, "/TR"])
                .arg(format!("\"{}\"", executable.display()));
        } else { command.args(["/Delete", "/F", "/TN", task]); }
        let _ = command.output();
    }
    Ok(())
}
#[tauri::command]
pub fn get_device_hwid(app: AppHandle) -> String { crate::vpn::device_hwid(&app) }
#[derive(serde::Serialize, serde::Deserialize)]
pub struct AppSettings {
    #[serde(default = "default_minimize_to_tray")]
    pub minimize_to_tray: bool,
}
impl Default for AppSettings {
    fn default() -> Self { Self { minimize_to_tray: true } }
}
fn default_minimize_to_tray() -> bool { true }
pub fn settings(app: &AppHandle) -> Value {
    let mut settings = read(app).unwrap_or_else(|_| json!({}));
    if !settings.is_object() { settings = json!({}); }
    if settings.get("minimizeToTray").is_none() && settings.get("minimize_to_tray").is_none() {
        settings["minimize_to_tray"] = json!(AppSettings::default().minimize_to_tray);
    }
    settings
}
