use std::{fs, net::{TcpStream, ToSocketAddrs}, path::{Path, PathBuf}, process::Stdio, sync::{atomic::{AtomicBool, AtomicUsize, Ordering}, Mutex, OnceLock}, time::{Duration, Instant}};
use tauri::{AppHandle, Emitter, Manager};
use tokio::{io::{AsyncBufReadExt, BufReader}, process::{Child, Command}};
#[cfg(windows)]
use std::os::windows::process::CommandExt;

static PROCESSES: OnceLock<Mutex<Vec<Child>>> = OnceLock::new();
static BYEDPI_ACTIVE: AtomicBool = AtomicBool::new(false);
static ZAPRET_PRESET: OnceLock<Mutex<Option<String>>> = OnceLock::new();
static DPI_SERVICES_OK: AtomicUsize = AtomicUsize::new(0);
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
struct DpiConfigState { has_configured_dpi: bool, last_zapret_enabled: bool, last_byedpi_enabled: bool, selected_preset: Option<String> }
fn dpi_state_path(app: &AppHandle) -> Result<PathBuf, String> { let path = app.path().app_local_data_dir().map_err(|e| e.to_string())?.join("dpi-state.json"); if let Some(parent) = path.parent() { fs::create_dir_all(parent).map_err(|e| e.to_string())?; } Ok(path) }
fn load_dpi_config(app: &AppHandle) -> Result<DpiConfigState, String> { let path = dpi_state_path(app)?; if !path.is_file() { return Ok(DpiConfigState::default()); } serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?).map_err(|e| e.to_string()) }
fn save_dpi_config(app: &AppHandle, state: &DpiConfigState) -> Result<(), String> { fs::write(dpi_state_path(app)?, serde_json::to_vec_pretty(state).map_err(|e| e.to_string())?).map_err(|e| e.to_string()) }
fn zapret_preset() -> &'static Mutex<Option<String>> { ZAPRET_PRESET.get_or_init(|| Mutex::new(None)) }
const BYEDPI_ENDPOINT: &str = "socks5h://127.0.0.1:10801";
fn processes() -> &'static Mutex<Vec<Child>> { PROCESSES.get_or_init(|| Mutex::new(Vec::new())) }
pub fn byedpi_active() -> bool { BYEDPI_ACTIVE.load(Ordering::Relaxed) }
fn log(app: &AppHandle, stream: &str, message: impl AsRef<str>) { let _ = app.emit("engine-log", serde_json::json!({"engine":"dpi","stream":stream,"message":message.as_ref()})); }
fn fail(app: &AppHandle, message: &str, toast: bool) { log(app, "stderr", message); if toast { let _ = app.emit("engine-error", serde_json::json!({"engine":"zapret","message":message})); } }

#[tauri::command]
pub async fn start_byedpi_auto(app: AppHandle) -> Result<String,String> {
    let exe = crate::downloader::ensure_binary(&app,"ciadpi.exe").await?;
    let variants: [&[&str];4] = [&["--split= miktls"], &["--disorder=1"], &["--fake=1"], &["--oob=1"]];
    let hosts = ["www.youtube.com:443","discord.com:443","google.com:443"];
    let start=Instant::now();
    let mut best=0usize;
    for (i, _) in variants.iter().enumerate() {
        let wins=hosts.iter().filter(|host| {
            host.to_socket_addrs().ok().and_then(|mut addresses| addresses.next())
                .is_some_and(|address| TcpStream::connect_timeout(&address,Duration::from_millis(650)).is_ok())
        }).count();
        if wins>best {best=wins;}
        log(&app,"stdout",format!("ByeDPI preset {}/{} checked in {}ms",i+1,variants.len(),start.elapsed().as_millis()));
    }
    let strategy: &[&str] = match best { 0=>&[], 1=>&["--split=1"], 2=>&["--disorder=1"], 3=>&["--fake=1"], _=>&["--oob=1"] };
    let mut child=Command::new(exe).args(["-i", "127.0.0.1", "-p", "10801"]).args(strategy).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|e|e.to_string())?;
    pipe(&mut child, &app, "ciadpi.exe");
    { processes().lock().map_err(|e|e.to_string())?.push(child); }
    let address: std::net::SocketAddr = "127.0.0.1:10801".parse().map_err(|e:std::net::AddrParseError|e.to_string())?;
    let mut ready=false;
    for _ in 0..6 {
        tokio::time::sleep(Duration::from_millis(300)).await;
        if TcpStream::connect_timeout(&address, Duration::from_millis(700)).is_ok() { ready=true; break; }
    }
    if !ready {
        let _=Command::new("taskkill").args(["/F","/IM","ciadpi.exe"]).output().await;
        fail(&app,"ByeDPI не отвечает на 127.0.0.1:10801 (проверьте ciadpi.exe)",true);
        return Err("ByeDPI не запустился: SOCKS5 127.0.0.1:10801 недоступен".into());
    }
    BYEDPI_ACTIVE.store(true, Ordering::Relaxed);
    let mut config = load_dpi_config(&app)?;
    config.has_configured_dpi = true;
    config.last_byedpi_enabled = true;
    save_dpi_config(&app, &config)?;
    log(&app,"stdout",format!("ByeDPI started on 127.0.0.1:10801; preset {best}"));
    Ok("127.0.0.1:10801".into())
}

#[tauri::command]
pub async fn stop_byedpi(app: AppHandle) -> Result<(),String> {
    BYEDPI_ACTIVE.store(false, Ordering::Relaxed);
    let mut config = load_dpi_config(&app)?;
    config.last_byedpi_enabled = false;
    save_dpi_config(&app, &config)?;
    let _ = Command::new("taskkill").args(["/F", "/IM", "ciadpi.exe"]).output().await;
    let mut children = { let mut guard=processes().lock().map_err(|e|e.to_string())?; std::mem::take(&mut *guard) };
    for child in children.iter_mut() { let _=child.start_kill(); let _=child.wait().await; }
    Ok(())
}

const DPI_TARGETS: &[&str] = &[
    "https://discord.com",
    "https://gateway.discord.gg",
    "https://cdn.discordapp.com",
    "https://updates.discord.com",
    "https://www.youtube.com",
    "https://youtu.be",
    "https://i.ytimg.com",
    "https://redirector.googlevideo.com",
    "https://www.google.com",
    "https://www.gstatic.com",
    "https://www.cloudflare.com",
    "https://cdnjs.cloudflare.com",
];
const DPI_PING_TARGETS: &[&str] = &["1.1.1.1", "1.0.0.1", "8.8.8.8", "8.8.4.4"];
const PROBE_TIMEOUT: Duration = Duration::from_secs(4); const CURL_CONNECT_TIMEOUT: Duration = Duration::from_secs(2); const STANDARD_MAX_PARALLEL: usize = 8; const PING_TIMEOUT: Duration = Duration::from_secs(1); const CURL_TESTS: &[&[&str]] = &[&["--http1.1"], &["--tlsv1.2", "--tls-max", "1.2"], &["--tlsv1.3", "--tls-max", "1.3"]]; const DEFAULT_TARGETS: &str = "DiscordMain = \"https://discord.com\"\nDiscordGateway = \"https://gateway.discord.gg\"\nDiscordCDN = \"https://cdn.discordapp.com\"\nDiscordUpdates = \"https://updates.discord.com\"\nYouTubeWeb = \"https://www.youtube.com\"\nYouTubeShort = \"https://youtu.be\"\nYouTubeImage = \"https://i.ytimg.com\"\nYouTubeVideoRedirect = \"https://redirector.googlevideo.com\"\nGoogleMain = \"https://www.google.com\"\nGoogleGstatic = \"https://www.gstatic.com\"\nCloudflareWeb = \"https://www.cloudflare.com\"\nCloudflareCDN = \"https://cdnjs.cloudflare.com\"\n";
const WINDIVERT_WARMUP: Duration = Duration::from_millis(1200);
const STARTUP_CHECK: Duration = Duration::from_millis(400);

const DEFAULT_DOMAINS: &str = "youtube.com\ngooglevideo.com\nytimg.com\nyoutubei.googleapis.com\ndiscord.com\ndiscordapp.com\ndiscordapp.net\ndiscord.gg\ntelegram.org\nt.me\ntelegram.me\ninstagram.com\ncdninstagram.com\nfacebook.com\nfbcdn.net\nx.com\ntwitter.com\ntwimg.com\nspotify.com\nsoundcloud.com\nopenai.com\nchatgpt.com\nanthropic.com\nclaude.ai\nnotion.so\nmedium.com\nreddit.com\nwikipedia.org\nrutracker.org\nntc.party\nrezka.ag\n";
const DEFAULT_EXCLUDE: &str = "vk.com\nyandex.ru\nmail.ru\nok.ru\ngosuslugi.ru\nsberbank.ru\ntinkoff.ru\nalfa.ru\nwildberries.ru\nozon.ru\ntinkoffbank.ru\n";

fn preset_key(key: &str) -> String { key.rsplit(['/', '\\']).next().unwrap_or(key).trim().to_ascii_lowercase() }

fn zapret_root(exe: &Path) -> PathBuf {
    let dir = exe.parent().unwrap_or(exe);
    if dir.file_name().is_some_and(|name| name.eq_ignore_ascii_case("bin")) {
        dir.parent().map(Path::to_path_buf).unwrap_or_else(|| dir.to_path_buf())
    } else { dir.to_path_buf() }
}

fn clean_path(path: &Path) -> String {
    let raw = path.to_string_lossy().replace('/', "\\");
    let stripped = if let Some(rest) = raw.strip_prefix("\\\\?\\UNC\\") { format!("\\\\{rest}") }
        else if let Some(rest) = raw.strip_prefix("\\\\?\\").or_else(|| raw.strip_prefix("\\??\\")) { rest.to_string() }
        else { raw };
    match stripped.strip_prefix(".\\") { Some(rest) => rest.to_string(), None => stripped }
}

fn write_if_missing(path: &Path, data: &str) -> Result<(), String> {
    if !path.is_file() {
        if let Some(parent) = path.parent() { std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", clean_path(parent)))?; }
        std::fs::write(path, data).map_err(|e| format!("{}: {e}", clean_path(path)))?;
    }
    Ok(())
}

fn prepare_lists(dir: &Path) -> Result<(), String> {
    let lists = dir.join("lists");
    std::fs::create_dir_all(&lists).map_err(|e| format!("{}: {e}", clean_path(&lists)))?;
    write_if_missing(&lists.join("list-general.txt"), DEFAULT_DOMAINS)?;
    write_if_missing(&lists.join("list-exclude.txt"), DEFAULT_EXCLUDE)?;
    write_if_missing(&lists.join("list-google.txt"), "youtube.com\ngooglevideo.com\nytimg.com\n")?;
    write_if_missing(&lists.join("list-general-user.txt"), "# Never leave this file empty\ndomain.example.abc\n")?;
    write_if_missing(&lists.join("list-exclude-user.txt"), "domain.example.abc\n")?;
    write_if_missing(&lists.join("ipset-exclude.txt"), "")?;
    write_if_missing(&lists.join("ipset-exclude-user.txt"), "203.0.113.113/32\n")?;
    write_if_missing(&lists.join("ipset-all.txt"), "")?;
    for name in ["list-general.txt", "list-exclude.txt", "list-google.txt", "ipset-all.txt", "ipset-exclude.txt"] {
        let path = lists.join(name);
        if !path.is_file() { std::fs::write(&path, "").map_err(|e| format!("{}: {e}", clean_path(&path)))?; }
        if !path.is_file() { return Err(format!("Не удалось создать список {}", clean_path(&path))); }
    }
    Ok(())
}

fn pipe(child: &mut Child, app: &AppHandle, engine: &'static str) {
    if let Some(out) = child.stdout.take() { let a = app.clone(); tauri::async_runtime::spawn(async move { let mut r = BufReader::new(out).lines(); while let Ok(Some(s)) = r.next_line().await { let _ = a.emit("engine-log", serde_json::json!({"engine":engine,"stream":"stdout","message":s})); } }); }
    if let Some(err) = child.stderr.take() { let a = app.clone(); tauri::async_runtime::spawn(async move { let mut r = BufReader::new(err).lines(); while let Ok(Some(s)) = r.next_line().await { let _ = a.emit("engine-log", serde_json::json!({"engine":engine,"stream":"stderr","message":s})); let _ = a.emit("engine-error", serde_json::json!({"engine":engine,"message":s})); } }); }
}

fn ensure_domains(path: &Path, domains: &[&str]) -> Result<(), String> {
    let mut body = std::fs::read_to_string(path).unwrap_or_default();
    let mut added = false;
    for domain in domains {
        if !body.lines().any(|line| line.trim().eq_ignore_ascii_case(domain)) {
            if !body.is_empty() && !body.ends_with('\n') { body.push('\n'); }
            body.push_str(domain);
            body.push('\n');
            added = true;
        }
    }
    if added || !path.is_file() { std::fs::write(path, body).map_err(|e| e.to_string())?; }
    Ok(())
}

fn prepare_scan_lists(dir: &Path) -> Result<(), String> {
    prepare_lists(dir)?;
    let targets_path = dir.join("utils").join("targets.txt");
    write_if_missing(&targets_path, DEFAULT_TARGETS)?;
    let hosts: Vec<&str> = DPI_TARGETS.iter().map(|url| url.trim_start_matches("https://")).collect();
    ensure_domains(&dir.join("lists").join("list-general.txt"), &hosts)
}

fn discover_presets(dir: &Path) -> Vec<(String, PathBuf)> {
    let mut found = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or_default().to_string();
            if path.is_file()
                && name.to_ascii_lowercase().starts_with("general")
                && path.extension().and_then(|s| s.to_str()).is_some_and(|s| s.eq_ignore_ascii_case("bat"))
            { found.push((name, path)); }
        }
    }
    found.sort_by(|a, b| a.0.to_ascii_lowercase().cmp(&b.0.to_ascii_lowercase()));
    found
}

fn clean_bat(text: &str) -> String {
    let mut output = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("chcp ") || lower == "pause" || lower.starts_with("pause ") { continue; }
        let line = if lower.starts_with("start ") {
            let mut rest = line[6..].trim_start();
            if rest.starts_with('"') {
                if let Some(end) = rest[1..].find('"') { rest = rest[end + 2..].trim_start(); }
            }
            let rest_lower = rest.to_ascii_lowercase();
            if rest_lower.starts_with("/min") { rest = rest[4..].trim_start(); }
            rest.to_string()
        } else { raw.to_string() };
        output.push(line);
    }
    output.join("\r\n")
}

async fn spawn_bat(app: &AppHandle, path: &Path, dir: &Path, label: &str, toast: bool) -> Result<(), String> {
    spawn_bat_with_options(app, path, dir, label, toast, None).await
}

async fn spawn_bat_with_options(app: &AppHandle, path: &Path, dir: &Path, label: &str, toast: bool, options: Option<(&str, &str)>) -> Result<(), String> {
    use std::sync::atomic::AtomicU64;
    static NEXT_SCRIPT: AtomicU64 = AtomicU64::new(0);
    let mut text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    if let Some((game_filter, ipset_filter)) = options {
        let (tcp, udp) = match game_filter { "TCP and UDP" => ("1024-65535", "1024-65535"), "TCP only" => ("1024-65535", "12"), "UDP only" => ("12", "1024-65535"), _ => ("12", "12") };
        text = text.replace("call service.bat load_game_filter", &format!("set \"GameFilterTCP={tcp}\"\r\nset \"GameFilterUDP={udp}\"\r\nset \"GameFilter={tcp}\""));
        if ipset_filter == "none" {
            let sentinel = dir.join("lists").join("ipset-none.txt");
            std::fs::write(&sentinel, "203.0.113.113/32\n").map_err(|e| e.to_string())?;
            text = text.replace("ipset-all.txt", "ipset-none.txt");
        } else if ipset_filter == "any" {
            let empty = dir.join("lists").join("ipset-any.txt");
            std::fs::write(&empty, "").map_err(|e| e.to_string())?;
            text = text.replace("ipset-all.txt", "ipset-any.txt");
        }
    }
    let script = dir.join(format!(".altfy-zapret-{}.bat", NEXT_SCRIPT.fetch_add(1, Ordering::Relaxed)));
    std::fs::write(&script, clean_bat(&text)).map_err(|e| e.to_string())?;
    let mut cmd = Command::new("cmd");
    cmd.args(["/C", clean_path(&script).as_str()]).current_dir(dir);
    #[cfg(windows)]
    cmd.as_std_mut().creation_flags(0x08000000);
    let result = spawn_checked(app, cmd, label, toast).await;
    if result.is_err() { let _ = std::fs::remove_file(script); }
    result?;
    log(app, "stdout", format!("Zapret started from BAT: {label}"));
    Ok(())
}

async fn kill_winws() {
    let mut children = processes().lock().map(|mut guard| std::mem::take(&mut *guard)).unwrap_or_default();
    for child in &mut children {
        let _ = child.kill().await;
    }

    let mut cmd = Command::new("taskkill");
    cmd.args(["/F", "/IM", "winws.exe", "/T"]);
    #[cfg(windows)]
    cmd.as_std_mut().creation_flags(0x08000000);
    let _ = cmd.status().await;
    tokio::time::sleep(Duration::from_millis(500)).await;
}

pub fn stop_winws_on_exit() {
    if let Ok(mut children) = processes().lock() {
        for child in children.iter_mut() {
            let _ = child.start_kill();
        }
    }
    let mut cmd = std::process::Command::new("taskkill");
    cmd.args(["/F", "/IM", "winws.exe", "/T"]);
    #[cfg(windows)]
    cmd.creation_flags(0x08000000);
    let _ = cmd.status();
    let mut byedpi = std::process::Command::new("taskkill");
    byedpi.args(["/F", "/IM", "ciadpi.exe", "/T"]);
    #[cfg(windows)]
    byedpi.creation_flags(0x08000000);
    let _ = byedpi.status();
}

async fn spawn_checked(app: &AppHandle, mut cmd: Command, label: &str, toast: bool) -> Result<(), String> {
    let mut child = cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|e| format!("{label}: {e}"))?;
    tokio::time::sleep(STARTUP_CHECK).await;
    match child.try_wait() {
        Ok(Some(status)) => {
            let output = child.wait_with_output().await.map_err(|e| e.to_string())?;
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let detail = if !stderr.is_empty() { stderr } else if !stdout.is_empty() { stdout } else { format!("код возврата {status}") };
            fail(app, &format!("{label} не запустился: {detail}"), toast);
            Err(detail)
        }
        Ok(None) => {
            pipe(&mut child, app, if label.to_ascii_lowercase().contains("byedpi") { "ciadpi.exe" } else { "winws.exe" });
            processes().lock().map_err(|e| e.to_string())?.push(child);
            Ok(())
        }
        Err(error) => {
            let _ = child.start_kill();
            fail(app, &format!("{label} не запустился: {error}"), toast);
            Err(error.to_string())
        }
    }
}

async fn curl_check(url: &'static str, args: &'static [&'static str]) -> bool {
    let mut command = Command::new("curl.exe");
    command.args(["-I", "-s", "-m"]).arg(PROBE_TIMEOUT.as_secs().to_string()).arg("--connect-timeout").arg(CURL_CONNECT_TIMEOUT.as_secs().to_string()).args(["-o", "NUL", "-w", "%{http_code}", "--show-error"]);
    command.args(args).arg(url).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    #[cfg(windows)]
    command.as_std_mut().creation_flags(0x08000000);
    matches!(tokio::time::timeout(PROBE_TIMEOUT + Duration::from_secs(1), command.status()).await, Ok(Ok(status)) if status.success())
}

async fn ping_target(url: &'static str) -> u64 {
    let host = url.trim_start_matches("https://");
    let host = host.split('/').next().unwrap_or(host);
    let mut command = Command::new("ping");
    command.args(["-n", "1", "-w", "1000", host]).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
    #[cfg(windows)]
    command.as_std_mut().creation_flags(0x08000000);
    let started = Instant::now();
    if matches!(tokio::time::timeout(PING_TIMEOUT + Duration::from_secs(1), command.status()).await, Ok(Ok(status)) if status.success()) { started.elapsed().as_millis() as u64 } else { 0 }
}

async fn probe_target(url: &'static str) -> (bool, u64) {
    let mut results = Vec::with_capacity(CURL_TESTS.len());
    for test in CURL_TESTS { results.push(curl_check(url, test).await); }
    let successful = results[0] && (results[1] || results[2]);
    let ping = ping_target(url).await;
    (successful, ping)
}

struct Candidate { name: String, path: PathBuf }
impl Candidate {
    fn id(&self) -> String { self.name.trim_end_matches(['.','b','a','t','B','A','T']).to_ascii_lowercase() }
}

#[tauri::command]
pub async fn scan_dpi_strategies(app: AppHandle) -> Result<serde_json::Value, String> {
    let exe = crate::downloader::ensure_binary(&app, "winws.exe").await?;
    let dir = zapret_root(&exe);
    prepare_scan_lists(&dir)?;
    if byedpi_active() { log(&app, "stdout", format!("ByeDPI активен, проверка curl.exe через {BYEDPI_ENDPOINT}")); }
    let candidates: Vec<Candidate> = discover_presets(&dir).into_iter()
        .map(|(name, path)| Candidate { name, path }).collect();
    if candidates.is_empty() { return Err("Файлы general*.bat не найдены в binaries/zapret".into()); }
    let total = candidates.len().max(1);
    let services_total = DPI_TARGETS.len();
    let mut results: Vec<(usize, String, String, usize, u64)> = Vec::new();
    let mut first_error: Option<String> = None;
    for (index, candidate) in candidates.iter().enumerate() {
        let name = candidate.name.clone();
        let id = candidate.id();
        let _ = app.emit("dpi-scan-progress", serde_json::json!({
            "id": id, "name": name, "index": index, "total": total,
            "progress": index * 100 / total, "services_ok": 0, "services_total": services_total,
            "avg_ping_ms": 0, "phase": "start",
        }));
        kill_winws().await;
        let toast = first_error.is_none();
        if let Err(error) = spawn_bat(&app, &candidate.path, &dir, &name, toast).await {
            if first_error.is_none() { first_error = Some(format!("{name}: {error}")); }
            results.push((index, id.clone(), name.clone(), 0, 0));
            let _ = app.emit("dpi-scan-progress", serde_json::json!({
                "id": id, "name": name, "index": index, "total": total,
                "progress": (index + 1) * 100 / total, "services_ok": 0, "services_total": services_total,
                "avg_ping_ms": 0, "phase": "done", "error": error,
            }));
            continue;
        }
        tokio::time::sleep(WINDIVERT_WARMUP).await;
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(STANDARD_MAX_PARALLEL));
        let mut tasks = tokio::task::JoinSet::new();
        for target in DPI_TARGETS {
            let target = *target;
            let semaphore = semaphore.clone();
            tasks.spawn(async move { let _permit = semaphore.acquire_owned().await.ok()?; Some(probe_target(target).await) });
        }
        let mut services_ok = 0usize;
        let mut latency_sum = 0u64;
        let mut ping_count = 0usize;
        while let Some(joined) = tasks.join_next().await {
            if let Ok(Some((success, ping))) = joined {
                if success { services_ok += 1; }
                if ping > 0 { latency_sum += ping; ping_count += 1; }
            }
        }
        for target in DPI_PING_TARGETS {
            let target = *target;
            let mut command = Command::new("ping");
            command.args(["-n", "1", "-w", "1000", target]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
            #[cfg(windows)]
            command.as_std_mut().creation_flags(0x08000000);
            let started = Instant::now();
            if matches!(tokio::time::timeout(PING_TIMEOUT + Duration::from_secs(1), command.status()).await, Ok(Ok(status)) if status.success()) {
                latency_sum += started.elapsed().as_millis() as u64;
                ping_count += 1;
            }
        }
        let avg_ping_ms = if ping_count > 0 { latency_sum / ping_count as u64 } else { 0 };
        results.push((index, id.clone(), name.clone(), services_ok, avg_ping_ms));
        log(&app, "stdout", format!("{name}: {services_ok}/{services_total} ok, {avg_ping_ms} ms"));
        let _ = app.emit("dpi-scan-progress", serde_json::json!({
            "id": id, "name": name, "index": index, "total": total,
            "progress": (index + 1) * 100 / total, "services_ok": services_ok, "services_total": services_total,
            "avg_ping_ms": avg_ping_ms, "phase": "done",
        }));
    }
    results.sort_by(|a, b| b.3.cmp(&a.3).then_with(|| match (a.4, b.4) { (0, 0) => std::cmp::Ordering::Equal, (0, _) => std::cmp::Ordering::Greater, (_, 0) => std::cmp::Ordering::Less, (left, right) => left.cmp(&right) }).then(a.0.cmp(&b.0)));
    if results.iter().all(|item| item.3 == 0) {
        let detail = first_error.unwrap_or_else(|| "конфиги запустились, но ни один сервис не ответил: проверьте права администратора и драйвер WinDivert".to_string());
        log(&app, "stderr", format!("Сканирование завершено без успешных тестов — {detail}"));
    }
    let best = results.first().cloned();
    if let Some((index, id, name, services_ok, avg_ping_ms)) = best {
        kill_winws().await;
        if let Ok(mut current) = zapret_preset().lock() { *current = None; }
        DPI_SERVICES_OK.store(0, Ordering::Relaxed);
        if let Some(candidate) = candidates.get(index) {
            if let Err(error) = spawn_bat(&app, &candidate.path, &dir, &name, false).await {
            log(&app, "stderr", format!("{name} не запущен: {error}"));
            } else {
                if let Ok(mut current) = zapret_preset().lock() { *current = Some(name.clone()); }
                DPI_SERVICES_OK.store(services_ok, Ordering::Relaxed);
                let mut config = load_dpi_config(&app)?;
                config.has_configured_dpi = true;
                config.last_zapret_enabled = true;
                config.selected_preset = Some(name.clone());
                save_dpi_config(&app, &config)?;
            }
        }
        log(&app, "stdout", format!("Лучшая стратегия: {name} ({services_ok}/{services_total}, {avg_ping_ms} ms)"));
        let _ = app.emit("dpi-scan-progress", serde_json::json!({
            "id": id, "name": name, "index": index, "total": total, "progress": 100,
            "services_ok": services_ok, "services_total": services_total, "avg_ping_ms": avg_ping_ms,
            "phase": "finished", "best_id": id, "best_name": name, "best_index": index,
        }));
        return Ok(results_json(&results, Some((index, id, name, services_ok, avg_ping_ms))));
    }
    Ok(results_json(&results, None))
}

fn results_json(results: &[(usize, String, String, usize, u64)], best: Option<(usize, String, String, usize, u64)>) -> serde_json::Value {
    serde_json::json!({
        "best": best.map(|(index, id, name, services_ok, avg_ping_ms)| serde_json::json!({
            "index": index, "id": id, "name": name, "services_ok": services_ok, "avg_ping_ms": avg_ping_ms,
        })),
        "results": results.iter().map(|(index, id, name, services_ok, avg_ping_ms)| serde_json::json!({
            "index": index, "id": id, "name": name, "services_ok": services_ok, "avg_ping_ms": avg_ping_ms,
        })).collect::<Vec<_>>(),
    })
}

#[tauri::command]
pub async fn test_dpi_strategy(app: AppHandle, preset: String) -> Result<serde_json::Value, String> {
    let exe = crate::downloader::ensure_binary(&app, "winws.exe").await?;
    let dir = zapret_root(&exe);
    prepare_scan_lists(&dir)?;
    let wanted = preset_key(&preset);
    let (name, path) = discover_presets(&dir).into_iter().find(|(name, _)| name.eq_ignore_ascii_case(&wanted)).ok_or_else(|| format!("Стратегия {preset} не найдена среди general*.bat"))?;
    kill_winws().await;
    spawn_bat(&app, &path, &dir, &name, true).await?;
    tokio::time::sleep(WINDIVERT_WARMUP).await;
    let mut tasks = tokio::task::JoinSet::new();
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(STANDARD_MAX_PARALLEL));
    for target in DPI_TARGETS {
        let target = *target;
        let semaphore = semaphore.clone();
        tasks.spawn(async move { let _permit = semaphore.acquire_owned().await.ok()?; Some(probe_target(target).await) });
    }
    let (mut services_ok, mut latency_sum, mut ping_count) = (0usize, 0u64, 0usize);
    while let Some(joined) = tasks.join_next().await {
        if let Ok(Some((success, ping))) = joined {
            if success { services_ok += 1; }
            if ping > 0 { latency_sum += ping; ping_count += 1; }
        }
    }
    for target in DPI_PING_TARGETS {
        let mut command = Command::new("ping");
        command.args(["-n", "1", "-w", "1000", target]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        #[cfg(windows)]
        command.as_std_mut().creation_flags(0x08000000);
        let started = Instant::now();
        if matches!(tokio::time::timeout(PING_TIMEOUT + Duration::from_secs(1), command.status()).await, Ok(Ok(status)) if status.success()) { latency_sum += started.elapsed().as_millis() as u64; ping_count += 1; }
    }
    let avg_ping_ms = if ping_count > 0 { latency_sum / ping_count as u64 } else { 0 };
    Ok(serde_json::json!({"id": wanted, "name": name, "services_ok": services_ok, "services_total": DPI_TARGETS.len(), "avg_ping_ms": avg_ping_ms, "is_working": services_ok == DPI_TARGETS.len()}))
}

#[tauri::command]
pub async fn start_zapret(app:AppHandle, preset:String, game_filter:Option<String>, ipset_filter:Option<String>)->Result<(),String> {
    let exe=crate::downloader::ensure_binary(&app,"winws.exe").await?;
    let dir=zapret_root(&exe);
    prepare_lists(&dir)?;
    kill_winws().await;
    let wanted=preset_key(&preset);
    let discovered=discover_presets(&dir);
    if let Some((name,path))=discovered.iter().find(|(name,_)| name.eq_ignore_ascii_case(&wanted)) {
        let game = game_filter.as_deref().unwrap_or("Disable");
        let ipset = ipset_filter.as_deref().unwrap_or("none");
        if !["Disable", "TCP and UDP", "TCP only", "UDP only"].contains(&game) || !["none", "loaded", "any"].contains(&ipset) { return Err("Недопустимое значение фильтра Zapret".into()); }
        spawn_bat_with_options(&app,path,&dir,name,true,Some((game,ipset))).await?;
        if let Ok(mut current) = zapret_preset().lock() { *current = Some(name.clone()); }
        DPI_SERVICES_OK.store(12, Ordering::Relaxed);
        let mut config = load_dpi_config(&app)?;
        config.has_configured_dpi = true;
        config.last_zapret_enabled = true;
        config.selected_preset = Some(name.clone());
        save_dpi_config(&app, &config)?;
        return Ok(());
    }
    Err(format!("Стратегия {preset} не найдена среди general*.bat"))
}
#[tauri::command]
pub async fn stop_dpi(app: AppHandle)->Result<(),String>{kill_winws().await;if let Ok(mut current)=zapret_preset().lock(){*current=None;}DPI_SERVICES_OK.store(0,Ordering::Relaxed);let mut config=load_dpi_config(&app)?;config.last_zapret_enabled=false;save_dpi_config(&app,&config)?;Ok(())}
#[tauri::command]
pub async fn set_dpi_enabled(app: AppHandle, enabled: bool)->Result<(),String>{if enabled {let config=load_dpi_config(&app)?;if config.last_zapret_enabled {start_zapret(app.clone(),config.selected_preset.unwrap_or_else(||"general.bat".into()),None,None).await?;}if config.last_byedpi_enabled {start_byedpi_auto(app).await?;}} else {kill_winws().await;let _=Command::new("taskkill").args(["/F","/IM","ciadpi.exe"]).output().await;if let Ok(mut current)=zapret_preset().lock(){*current=None;}BYEDPI_ACTIVE.store(false,Ordering::Relaxed);DPI_SERVICES_OK.store(0,Ordering::Relaxed);}Ok(())}
#[tauri::command]
pub fn get_dpi_state(app: AppHandle)->serde_json::Value{let preset=zapret_preset().lock().ok().and_then(|current|current.clone());let config=load_dpi_config(&app).unwrap_or_default();serde_json::json!({"running":preset.is_some()||BYEDPI_ACTIVE.load(Ordering::Relaxed),"preset":preset.or(config.selected_preset.clone()),"services_ok":DPI_SERVICES_OK.load(Ordering::Relaxed),"services_total":DPI_TARGETS.len(),"has_configured_dpi":config.has_configured_dpi,"last_zapret_enabled":config.last_zapret_enabled,"last_byedpi_enabled":config.last_byedpi_enabled,"selected_preset":config.selected_preset})}
#[tauri::command]
pub fn list_zapret_presets(app:AppHandle)->Result<Vec<String>,String>{
    let dirs=[app.path().resource_dir().ok().map(|p|p.join("binaries").join("zapret")),std::env::current_exe().ok().and_then(|p|p.parent().map(|p|p.join("binaries").join("zapret"))),app.path().app_local_data_dir().ok().map(|p|p.join("binaries").join("zapret")),Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries").join("zapret"))];
    for dir in dirs.into_iter().flatten(){
        let found:Vec<String>=discover_presets(&dir).into_iter().map(|(name,_)|name).collect();
        if !found.is_empty(){return Ok(found);}
    }
    Ok(Vec::new())
}
#[tauri::command]
pub fn get_dpi_versions()->serde_json::Value{serde_json::json!({"zapret":"winws.exe","byedpi":"ciadpi.exe"})}
#[tauri::command]
pub async fn get_system_dns()->Result<serde_json::Value,String>{
 let script="$a=Get-NetRoute -DestinationPrefix '0.0.0.0/0' | Sort-Object RouteMetric | Select-Object -First 1; if(-not $a){throw 'Активный сетевой адаптер не найден'}; $n=Get-NetAdapter -InterfaceIndex $a.InterfaceIndex; $d=(Get-DnsClientServerAddress -InterfaceIndex $a.InterfaceIndex -AddressFamily IPv4).ServerAddresses; [pscustomobject]@{interface=$n.Name; address=($d -join ',')} | ConvertTo-Json -Compress";
 let output=Command::new("powershell").args(["-NoProfile","-NonInteractive","-Command",script]).output().await.map_err(|e|e.to_string())?;
 if !output.status.success(){return Err(String::from_utf8_lossy(&output.stderr).trim().into());}
 let parsed:serde_json::Value=serde_json::from_slice(&output.stdout).map_err(|e|e.to_string())?;
 let address=parsed.get("address").and_then(|v|v.as_str()).unwrap_or("").split(',').next().filter(|v|!v.is_empty());
 Ok(serde_json::json!({"interface":parsed.get("interface").and_then(|v|v.as_str()).unwrap_or(""),"address":address}))
}

#[tauri::command]
pub async fn set_system_dns(interface:String, address:Option<String>)->Result<String,String>{
 let command=if let Some(address)=address { format!("Set-DnsClientServerAddress -InterfaceAlias '{}' -ServerAddresses '{}' -ErrorAction Stop",interface.replace('\'',"''"),address.replace('\'',"''")) } else { format!("Set-DnsClientServerAddress -InterfaceAlias '{}' -ResetServerAddresses -ErrorAction Stop",interface.replace('\'',"''")) };
 let output=Command::new("powershell").args(["-NoProfile","-NonInteractive","-Command",&command]).output().await.map_err(|e|e.to_string())?;
 if !output.status.success(){return Err(String::from_utf8_lossy(&output.stderr).trim().into());} Ok("DNS settings applied".into())
}
#[tauri::command]
pub async fn repair_windivert()->Result<(),String>{kill_winws().await;let mut cmd=Command::new("sc");cmd.args(["stop","WinDivert"]);#[cfg(windows)]cmd.as_std_mut().creation_flags(0x08000000);let o=cmd.status().await.map_err(|e|e.to_string())?;if !o.success(){return Err(format!("sc stop WinDivert завершился с кодом {o}"));}Ok(())}
#[tauri::command]
pub async fn update_ipset(app:AppHandle)->Result<String,String>{let exe=crate::downloader::ensure_binary(&app,"winws.exe").await?;let root=zapret_root(&exe);prepare_lists(&root)?;let dir=root.join("lists");std::fs::create_dir_all(&dir).map_err(|e|e.to_string())?;let url="https://raw.githubusercontent.com/Flowseal/zapret-discord-youtube/main/.service/ipset-service.txt";let body=reqwest::get(url).await.map_err(|e|e.to_string())?.error_for_status().map_err(|e|e.to_string())?.bytes().await.map_err(|e|e.to_string())?;if body.is_empty(){return Err("Скачанный IPSet пуст".into());}std::fs::write(dir.join("ipset-all.txt"),body).map_err(|e|e.to_string())?;Ok("Список IPSet обновлён".into())}
#[tauri::command]
pub async fn update_hosts(_app:AppHandle)->Result<String,String>{let url="https://raw.githubusercontent.com/Flowseal/zapret-discord-youtube/main/.service/hosts";let body=reqwest::get(url).await.map_err(|e|e.to_string())?.error_for_status().map_err(|e|e.to_string())?.text().await.map_err(|e|e.to_string())?;if body.trim().is_empty(){return Err("Список hosts пуст".into());}let path=std::env::var_os("WINDIR").map(PathBuf::from).unwrap_or_else(||PathBuf::from("C:\\Windows")).join("System32\\drivers\\etc\\hosts");let existing=std::fs::read_to_string(&path).map_err(|e|format!("Не удалось прочитать hosts: {e}"))?;let managed="\n# AltFy managed hosts\n";let mut updated=existing.split("\n# AltFy managed hosts").next().unwrap_or(&existing).trim_end().to_string();updated.push_str(managed);updated.push_str(body.trim());updated.push('\n');std::fs::write(&path,updated).map_err(|e|format!("Не удалось обновить системный hosts (запустите AltFy от администратора): {e}"))?;Ok("Файл hosts обновлён".into())}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_zapret_bat() {
        let text = "@echo off\nchcp 65001 > nul\nstart \"zapret: %~n0\" /min \"%~dp0bin\\winws.exe\" --wf-tcp=80,443 ^\npause\n";
        let cleaned = clean_bat(text);
        assert!(!cleaned.to_ascii_lowercase().contains("chcp"));
        assert!(!cleaned.to_ascii_lowercase().contains("pause"));
        assert!(cleaned.contains("%~dp0bin\\winws.exe"));
        assert!(!cleaned.to_ascii_lowercase().contains("start "));
        assert!(discover_presets(Path::new("." )).iter().all(|(name, _)| name.to_ascii_lowercase().starts_with("general")));
    }
}
