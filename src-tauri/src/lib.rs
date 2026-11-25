mod downloader;
mod tg_proxy;
mod dpi;
mod vpn;
mod settings;

use tauri::{menu::{Menu, MenuItem}, tray::TrayIconBuilder, Manager, WindowEvent};
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::process::Command;

const DEFAULT_MINIMIZE_TO_TRAY: bool = true;

fn stop_all() {
    let _ = vpn::disconnect_vpn();
    dpi::stop_winws_on_exit();
    tg_proxy::stop_tg_proxy_on_exit();
}

fn kill_process(image: &str) {
    let mut command = Command::new("taskkill.exe");
    command.args(["/F", "/T", "/IM", image]);
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    let _ = command.status();
}

#[cfg(windows)]
#[tauri::command]
fn get_running_processes() -> Result<Vec<String>, String> {
    let mut command = Command::new("tasklist.exe");
    command.args(["/FO", "CSV", "/NH"]);
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    let output = command.output().map_err(|error| error.to_string())?;
    if !output.status.success() { return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned()); }
    let processes = String::from_utf8_lossy(&output.stdout).lines()
        .filter_map(|line| line.trim().strip_prefix('"')?.split('"').next())
        .filter(|name| name.to_ascii_lowercase().ends_with(".exe"))
        .map(str::to_owned).collect::<std::collections::BTreeSet<_>>();
    Ok(processes.into_iter().collect())
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let _ = vpn::reset_system_proxy();
            let open = MenuItem::with_id(app, "open", "Открыть AltFy", true, None::<&str>)?;
            let stop = MenuItem::with_id(app, "stop", "Отключить всё", true, None::<&str>)?;
            let exit = MenuItem::with_id(app, "exit", "Выход", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &stop, &exit])?;
            TrayIconBuilder::new().icon(app.default_window_icon().cloned().expect("AltFy icon is missing")).menu(&menu).show_menu_on_left_click(false).on_menu_event(|app, event| match event.id.as_ref() {
                "open" => { if let Some(window) = app.get_webview_window("main") { let _ = window.show(); let _ = window.set_focus(); } }
                "stop" => stop_all(),
                "exit" => { stop_all(); app.exit(0); }
                _ => {}
            }).build(app)?;
            if let Some(window) = app.get_webview_window("main") {
                let hidden_window = window.clone();
                let app_handle = app.handle().clone();
                let _ = window.on_window_event(move |event| { if let WindowEvent::CloseRequested { api, .. } = event {
                    let settings = settings::settings(&app_handle);
                    if settings["minimize_to_tray"].as_bool().or_else(|| settings["minimizeToTray"].as_bool()).unwrap_or(DEFAULT_MINIMIZE_TO_TRAY) {
                        api.prevent_close();
                        let _ = hidden_window.hide();
                    } else {
                        for process in ["sing-box.exe", "winws.exe", "ciadpi.exe", "tg-ws-proxy.exe"] { kill_process(process); }
                        let _ = vpn::reset_system_proxy();
                        stop_all();
                        api.prevent_close();
                        app_handle.exit(0);
                    }
                } });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet, get_running_processes, downloader::update_binary, downloader::get_binary_status, downloader::check_app_updates, vpn::check_network_diagnostics, tg_proxy::start_tg_proxy, tg_proxy::stop_tg_proxy, tg_proxy::get_tg_proxy_status, tg_proxy::generate_tg_secret, tg_proxy::check_tg_proxy_ping, tg_proxy::check_cf_server_ping, tg_proxy::ping_cf_servers, tg_proxy::open_external_url, dpi::start_byedpi_auto, dpi::stop_byedpi, dpi::start_zapret, dpi::stop_dpi, dpi::set_dpi_enabled, dpi::list_zapret_presets, dpi::get_dpi_versions, dpi::get_dpi_state, dpi::get_system_dns, dpi::set_system_dns, dpi::repair_windivert, dpi::update_ipset, dpi::update_hosts, dpi::scan_dpi_strategies, dpi::test_dpi_strategy, vpn::import_vpn_source, vpn::update_vpn_subscription, vpn::delete_vpn_item, vpn::get_vpn_state, vpn::select_vpn_server, vpn::ping_vpn_servers, vpn::connect_vpn, vpn::disconnect_vpn, vpn::get_vpn_status, settings::get_app_settings, settings::save_app_settings, settings::apply_startup_settings, settings::get_device_hwid])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");
    app.run(|_, event| {
        if matches!(event, tauri::RunEvent::Exit | tauri::RunEvent::ExitRequested { .. }) {
            stop_all();
        }
    });
}
