use std::{net::{SocketAddr, TcpStream}, process::Stdio, sync::OnceLock, time::{Duration, Instant}};
use rand::{rngs::OsRng, RngCore};
use serde::Deserialize;
use tauri::{AppHandle, Emitter};
use tokio::{io::{AsyncBufReadExt, AsyncRead, BufReader}, process::{Child, Command}, sync::Mutex};

static PROCESS: OnceLock<Mutex<Option<Child>>> = OnceLock::new();

fn process() -> &'static Mutex<Option<Child>> {
    PROCESS.get_or_init(|| Mutex::new(None))
}

fn forward_logs<R: AsyncRead + Unpin + Send + 'static>(app: AppHandle, reader: R, stream: &'static str) {
    tokio::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = app.emit("engine-log", serde_json::json!({ "engine": "tg-ws-proxy", "stream": stream, "message": line }));
        }
    });
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TgProxyConfig {
    port: u16,
    auto_dc: bool,
    pool_size: u8,
    secret: String,
    use_cf_cdn: bool,
    cf_priority: bool,
    cf_server: String,
}

#[tauri::command]
pub async fn open_external_url(url: String) -> Result<(), String> {
    let (kind, params) = url.strip_prefix("tg://").and_then(|s| s.split_once('?')).ok_or("Недопустимая ссылка")?;
    let params = params.strip_prefix("server=127.0.0.1&port=").ok_or("Недопустимый адрес прокси")?;
    let (port, secret) = match params.split_once("&secret=") { Some((port, secret)) => (port, Some(secret)), None => (params, None) };
    if port.is_empty() || !port.bytes().all(|c| c.is_ascii_digit()) || port.parse::<u16>().ok().filter(|p| *p > 0).is_none() { return Err("Недопустимый порт".into()); }
    match (kind, secret) {
        ("socks", None) => {},
        ("proxy", Some(secret)) if secret.len() == 32 && secret.bytes().all(|c| c.is_ascii_hexdigit()) => {},
        _ => return Err("Недопустимая ссылка прокси".into()),
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let status = tauri::async_runtime::spawn_blocking(move || {
            std::process::Command::new("cmd.exe")
                .args(["/D", "/C", "start", ""])
                .raw_arg(format!("\"{url}\""))
                .creation_flags(0x08000000)
                .status()
        }).await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;
        if !status.success() { return Err("Не удалось открыть Telegram".into()); }
        Ok(())
    }
    #[cfg(not(windows))]
    { Err("Открытие Telegram поддерживается только в Windows".into()) }
}

#[tauri::command]
pub fn generate_tg_secret() -> String {
    let mut bytes = [0u8; 16];
    OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[tauri::command]
pub async fn check_tg_proxy_ping(port: u16) -> Result<u64, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let address = SocketAddr::from(([127, 0, 0, 1], port));
        let start = Instant::now();
        TcpStream::connect_timeout(&address, Duration::from_secs(2)).map_err(|e| e.to_string())?;
        Ok(start.elapsed().as_millis() as u64)
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn check_cf_server_ping(server: String) -> Result<u64, String> {
    tauri::async_runtime::spawn_blocking(move || {
        if !["1.1.1.1", "1.0.0.1"].contains(&server.as_str()) { return Err("Неизвестный сервер".into()); }
        let address: SocketAddr = format!("{server}:443").parse().map_err(|e: std::net::AddrParseError| e.to_string())?;
        let start = Instant::now();
        TcpStream::connect_timeout(&address, Duration::from_secs(2)).map_err(|e| e.to_string())?;
        Ok(start.elapsed().as_millis() as u64)
    }).await.map_err(|e| e.to_string())?
}

const CF_SERVERS: [&str; 20] = [
    "162.159.192.1", "162.159.193.1", "188.114.96.1", "188.114.97.1",
    "188.114.98.1", "188.114.99.1", "104.16.132.229", "104.16.133.229",
    "172.67.70.1", "104.18.2.1", "104.18.3.1", "104.19.192.1",
    "104.20.1.1", "104.21.1.1", "104.22.1.1", "104.24.1.1",
    "104.25.1.1", "104.26.1.1", "104.27.1.1", "1.1.1.1",
];

#[tauri::command]
pub async fn ping_cf_servers() -> Result<std::collections::HashMap<String, Option<u64>>, String> {
    let jobs: Vec<_> = CF_SERVERS.iter().map(|&server| {
        tauri::async_runtime::spawn_blocking(move || {
            let address: SocketAddr = format!("{server}:443").parse().expect("static IP address");
            let start = Instant::now();
            let ping = TcpStream::connect_timeout(&address, Duration::from_secs(2))
                .ok().map(|_| start.elapsed().as_millis() as u64);
            (server.to_string(), ping)
        })
    }).collect();
    let mut results = std::collections::HashMap::new();
    for job in jobs {
        let (server, ping) = job.await.map_err(|e| e.to_string())?;
        results.insert(server, ping);
    }
    Ok(results)
}

#[tauri::command]
pub async fn start_tg_proxy(app: AppHandle, config: TgProxyConfig) -> Result<String, String> {
    if config.port == 0 { return Err("Порт должен быть от 1 до 65535".into()); }
    if ![2, 4, 6].contains(&config.pool_size) { return Err("Недопустимый размер пула".into()); }
    if config.secret.len() != 32 || !config.secret.bytes().all(|c| c.is_ascii_hexdigit()) { return Err("Секрет должен содержать 32 hex-символа".into()); }
    let mut guard = process().lock().await;
    if let Some(child) = guard.as_mut() {
        child.kill().await.map_err(|e| format!("Не удалось остановить прокси: {e}"))?;
    }
    *guard = None;
    let path = crate::downloader::ensure_binary(&app, "tg-ws-proxy.exe").await?;
    let mut command = Command::new(path);
    command.arg("--port").arg(config.port.to_string())
        .arg("--pool-size").arg(config.pool_size.to_string())
        .arg("--secret").arg(&config.secret);
    if config.auto_dc { command.arg("--auto-dc"); }
    if config.use_cf_cdn { command.arg("--use-cf-cdn"); }
    if config.cf_priority { command.arg("--cf-priority"); }
    if !config.cf_server.is_empty() { command.arg("--cf-server").arg(&config.cf_server); }
    command.stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    let mut child = command.spawn().map_err(|e| format!("Не удалось запустить tg-ws-proxy: {e}"))?;
    if let Some(stdout) = child.stdout.take() { forward_logs(app.clone(), stdout, "stdout"); }
    if let Some(stderr) = child.stderr.take() { forward_logs(app, stderr, "stderr"); }
    *guard = Some(child);
    Ok("TG-WS Proxy запущен".into())
}

#[tauri::command]
pub async fn stop_tg_proxy() -> Result<(), String> {
    let mut guard = process().lock().await;
    if let Some(child) = guard.as_mut() {
        child.kill().await.map_err(|e| format!("Не удалось остановить прокси: {e}"))?;
    }
    *guard = None;
    Ok(())
}

#[tauri::command]
pub async fn get_tg_proxy_status() -> bool {
    let mut guard = process().lock().await;
    if let Some(child) = guard.as_mut() {
        match child.try_wait() {
            Ok(None) => return true,
            Ok(Some(_)) => *guard = None,
            Err(_) => return false,
        }
    }
    false
}
