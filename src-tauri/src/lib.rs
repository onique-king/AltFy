mod downloader;
mod tg_proxy;
mod dpi;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, tg_proxy::start_tg_proxy, tg_proxy::stop_tg_proxy, tg_proxy::get_tg_proxy_status, tg_proxy::generate_tg_secret, tg_proxy::check_tg_proxy_ping, tg_proxy::check_cf_server_ping, tg_proxy::ping_cf_servers, tg_proxy::open_external_url, dpi::start_byedpi_auto, dpi::stop_byedpi, dpi::start_zapret, dpi::stop_dpi, dpi::list_zapret_presets, dpi::get_dpi_versions, dpi::set_system_dns, dpi::repair_windivert, dpi::update_ipset, dpi::update_hosts, dpi::scan_dpi_strategies])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
