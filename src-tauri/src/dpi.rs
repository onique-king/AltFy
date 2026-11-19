use std::{net::TcpStream, path::{Path, PathBuf}, process::Stdio, sync::{atomic::{AtomicBool, Ordering}, Mutex, OnceLock}, time::{Duration, Instant}};
use tauri::{AppHandle, Emitter, Manager};
use tokio::{io::{AsyncBufReadExt, BufReader}, process::{Child, Command}};

static PROCESSES: OnceLock<Mutex<Vec<Child>>> = OnceLock::new();
static BYEDPI_ACTIVE: AtomicBool = AtomicBool::new(false);
const BYEDPI_ENDPOINT: &str = "socks5h://127.0.0.1:10801";
fn processes() -> &'static Mutex<Vec<Child>> { PROCESSES.get_or_init(|| Mutex::new(Vec::new())) }
pub fn byedpi_active() -> bool { BYEDPI_ACTIVE.load(Ordering::Relaxed) }
fn log(app: &AppHandle, stream: &str, message: impl AsRef<str>) { let _ = app.emit("engine-log", serde_json::json!({"engine":"dpi","stream":stream,"message":message.as_ref()})); }
fn fail(app: &AppHandle, message: &str, toast: bool) { log(app, "stderr", message); if toast { let _ = app.emit("engine-error", serde_json::json!({"engine":"zapret","message":message})); } }
fn bin_dir(app: &AppHandle) -> Result<std::path::PathBuf,String> { app.path().app_local_data_dir().map(|p|p.join("binaries")).map_err(|e|e.to_string()) }

#[tauri::command]
pub async fn start_byedpi_auto(app: AppHandle) -> Result<String,String> {
    let exe = crate::downloader::ensure_binary(&app,"ciadpi.exe").await?;
    let variants: [&[&str];4] = [&["--split= miktls"], &["--disorder=1"], &["--fake=1"], &["--oob=1"]];
    let hosts = ["www.youtube.com:443","discord.com:443","google.com:443"];
    let start=Instant::now();
    let mut best=0usize;
    for (i, _) in variants.iter().enumerate() {
        let wins=hosts.iter().filter(|host| TcpStream::connect_timeout(&host.parse().unwrap(),Duration::from_millis(650)).is_ok()).count();
        if wins>best {best=wins;}
        log(&app,"stdout",format!("ByeDPI preset {}/{} checked in {}ms",i+1,variants.len(),start.elapsed().as_millis()));
    }
    let args: &[&str] = match best { 0=>&["--port=10801"], 1=>&["--split=1","--port=10801"], 2=>&["--disorder=1","--port=10801"], 3=>&["--fake=1","--port=10801"], _=>&["--oob=1","--port=10801"] };
    let child=Command::new(exe).args(args).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|e|e.to_string())?;
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
    log(&app,"stdout",format!("ByeDPI started on 127.0.0.1:10801; preset {best}"));
    Ok("127.0.0.1:10801".into())
}

#[tauri::command]
pub async fn stop_byedpi() -> Result<(),String> {
    BYEDPI_ACTIVE.store(false, Ordering::Relaxed);
    let _ = Command::new("taskkill").args(["/F", "/IM", "ciadpi.exe"]).output().await;
    let mut children = { let mut guard=processes().lock().map_err(|e|e.to_string())?; std::mem::take(&mut *guard) };
    for child in children.iter_mut() { let _=child.start_kill(); let _=child.wait().await; }
    Ok(())
}

struct ZapretPreset { id: &'static str, name: &'static str, args: &'static [&'static str] }

const ZAPRET_PRESETS: &[ZapretPreset] = &[
    ZapretPreset { id: "general-fake-alt3", name: "general (FAKE TLS AUTO ALT3).bat", args: &[
        "--wf-tcp=80,443", "--wf-udp=443,50000-50100",
        "--filter-udp=443", "--hostlist=lists/list-general.txt", "--hostlist-exclude=lists/list-exclude.txt", "--dpi-desync=fake", "--dpi-desync-repeats=6", "--new",
        "--filter-tcp=80", "--hostlist=lists/list-general.txt", "--hostlist-exclude=lists/list-exclude.txt", "--dpi-desync=fake,split2", "--dpi-desync-autottl=3", "--dpi-desync-fooling=md5sig", "--new",
        "--filter-tcp=443", "--hostlist=lists/list-general.txt", "--hostlist-exclude=lists/list-exclude.txt", "--dpi-desync=fake,split2", "--dpi-desync-autottl=3", "--dpi-desync-fooling=md5sig", "--new",
        "--filter-udp=50000-50100", "--filter-l7=discord,stun", "--dpi-desync=fake", "--dpi-desync-repeats=6",
    ]},
    ZapretPreset { id: "general-alt11", name: "general (ALT11).bat", args: &[
        "--wf-tcp=80,443", "--wf-udp=443,50000-50100",
        "--filter-udp=443", "--hostlist=lists/list-general.txt", "--hostlist-exclude=lists/list-exclude.txt", "--dpi-desync=fake", "--dpi-desync-repeats=6", "--new",
        "--filter-tcp=80", "--hostlist=lists/list-general.txt", "--hostlist-exclude=lists/list-exclude.txt", "--dpi-desync=fake,disorder2", "--dpi-desync-autottl=11", "--dpi-desync-fooling=badseq,md5sig", "--new",
        "--filter-tcp=443", "--hostlist=lists/list-general.txt", "--hostlist-exclude=lists/list-exclude.txt", "--dpi-desync=fake,disorder2", "--dpi-desync-autottl=11", "--dpi-desync-fooling=badseq,md5sig", "--new",
        "--filter-udp=50000-50100", "--filter-l7=discord,stun", "--dpi-desync=fake", "--dpi-desync-repeats=6",
    ]},
    ZapretPreset { id: "general-fake-auto", name: "general (FAKE TLS AUTO).bat", args: &[
        "--wf-tcp=80,443", "--wf-udp=443,50000-50100",
        "--filter-udp=443", "--hostlist=lists/list-general.txt", "--hostlist-exclude=lists/list-exclude.txt", "--dpi-desync=fake", "--dpi-desync-repeats=6", "--new",
        "--filter-tcp=80", "--hostlist=lists/list-general.txt", "--hostlist-exclude=lists/list-exclude.txt", "--dpi-desync=fake,split2", "--dpi-desync-autottl=1", "--dpi-desync-fooling=md5sig", "--new",
        "--filter-tcp=443", "--hostlist=lists/list-general.txt", "--hostlist-exclude=lists/list-exclude.txt", "--dpi-desync=fake,split2", "--dpi-desync-autottl=1", "--dpi-desync-fooling=md5sig", "--new",
        "--filter-udp=50000-50100", "--filter-l7=discord,stun", "--dpi-desync=fake", "--dpi-desync-repeats=6",
    ]},
    ZapretPreset { id: "general-simple-fake", name: "general (SIMPLE FAKE).bat", args: &[
        "--wf-tcp=80,443", "--wf-udp=443,50000-50100",
        "--filter-udp=443", "--hostlist=lists/list-general.txt", "--hostlist-exclude=lists/list-exclude.txt", "--dpi-desync=fake", "--dpi-desync-repeats=6", "--new",
        "--filter-tcp=80,443", "--hostlist=lists/list-general.txt", "--hostlist-exclude=lists/list-exclude.txt", "--dpi-desync=fake", "--dpi-desync-repeats=6", "--dpi-desync-fooling=md5sig", "--new",
        "--filter-udp=50000-50100", "--filter-l7=discord,stun", "--dpi-desync=fake", "--dpi-desync-repeats=6",
    ]},
    ZapretPreset { id: "discord", name: "discord.bat", args: &[
        "--wf-tcp=443", "--wf-udp=50000-50100",
        "--filter-udp=50000-50100", "--filter-l7=discord,stun", "--dpi-desync=fake", "--dpi-desync-repeats=6", "--new",
        "--filter-tcp=443", "--hostlist=lists/list-general.txt", "--hostlist-exclude=lists/list-exclude.txt", "--dpi-desync=fake,split2", "--dpi-desync-autottl=3", "--dpi-desync-fooling=md5sig",
    ]},
];

const DPI_TARGETS: &[&str] = &[
    "https://youtube.com",
    "https://ytimg.com",
    "https://googlevideo.com",
    "https://discord.com",
    "https://discord.gg",
    "https://discordapp.com",
    "https://x.com",
    "https://twimg.com",
    "https://instagram.com",
    "https://cdninstagram.com",
    "https://rutracker.org",
    "https://soundcloud.com",
    "https://medium.com",
    "https://ntc.party",
    "https://rezka.ag",
];
const PROBE_TIMEOUT: Duration = Duration::from_millis(4000);
const WINDIVERT_WARMUP: Duration = Duration::from_millis(1200);
const STARTUP_CHECK: Duration = Duration::from_millis(400);

const DEFAULT_DOMAINS: &str = "youtube.com\ngooglevideo.com\nytimg.com\nyoutubei.googleapis.com\ndiscord.com\ndiscordapp.com\ndiscordapp.net\ndiscord.gg\ntelegram.org\nt.me\ntelegram.me\ninstagram.com\ncdninstagram.com\nfacebook.com\nfbcdn.net\nx.com\ntwitter.com\ntwimg.com\nspotify.com\nsoundcloud.com\nopenai.com\nchatgpt.com\nanthropic.com\nclaude.ai\nnotion.so\nmedium.com\nreddit.com\nwikipedia.org\nrutracker.org\nntc.party\nrezka.ag\n";
const DEFAULT_EXCLUDE: &str = "vk.com\nyandex.ru\nmail.ru\nok.ru\ngosuslugi.ru\nsberbank.ru\ntinkoff.ru\nalfa.ru\nwildberries.ru\nozon.ru\ntinkoffbank.ru\n";

fn preset_key(key: &str) -> String { key.rsplit(['/', '\\']).next().unwrap_or(key).trim().to_ascii_lowercase() }

fn find_preset(key: &str) -> Option<&'static ZapretPreset> {
    let target = preset_key(key);
    let stem = target.strip_suffix(".bat").unwrap_or(&target);
    ZAPRET_PRESETS.iter().find(|p| p.name.eq_ignore_ascii_case(&target) || p.id.eq_ignore_ascii_case(stem) || p.name.trim_end_matches(".bat").eq_ignore_ascii_case(stem))
}

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

fn join_path(dir: &Path, value: &str) -> String {
    let trimmed = value.trim().trim_matches('"');
    let path = Path::new(trimmed);
    if path.is_absolute() || trimmed.starts_with("\\\\") { clean_path(path) } else { clean_path(&dir.join(trimmed)) }
}

fn looks_like_path(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    value.contains('\\') || value.contains('/') || [".bin", ".txt", ".dat", ".json", ".list"].iter().any(|ext| lower.ends_with(ext))
}

fn is_path_key(key: &str) -> bool {
    key.contains("hostlist") || key.contains("ipset") || (key.starts_with("--dpi-desync-fake") && !key.ends_with("-mod"))
}

fn normalize_args(dir: &Path, args: &[String]) -> Vec<String> {
    args.iter().map(|arg| match arg.split_once('=') {
        Some((key, value)) if is_path_key(key) => {
            let parts: Vec<String> = value.split(',').filter_map(|part| {
                let part = part.trim();
                if part.is_empty() { return None; }
                let (at, body) = match part.strip_prefix('@') { Some(rest) => ("@", rest), None => ("", part) };
                if looks_like_path(body) { Some(format!("{at}{}", join_path(dir, body))) } else { Some(part.to_string()) }
            }).collect();
            if parts.is_empty() { arg.clone() } else { format!("{key}={}", parts.join(",")) }
        }
        _ => arg.clone(),
    }).collect()
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
    for name in ["ipset-exclude.txt", "ipset-all.txt", "list-general.txt", "list-exclude.txt", "list-google.txt"] {
        let path = lists.join(name);
        if !path.is_file() { std::fs::write(&path, "").map_err(|e| format!("{}: {e}", clean_path(&path)))?; }
    }
    Ok(())
}

fn pipe(child: &mut Child, app: &AppHandle) {
    if let Some(out) = child.stdout.take() { let a = app.clone(); tauri::async_runtime::spawn(async move { let mut r = BufReader::new(out).lines(); while let Ok(Some(s)) = r.next_line().await { log(&a, "stdout", s); } }); }
    if let Some(err) = child.stderr.take() { let a = app.clone(); tauri::async_runtime::spawn(async move { let mut r = BufReader::new(err).lines(); while let Ok(Some(s)) = r.next_line().await { log(&a, "stderr", s); } }); }
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
    let hosts: Vec<&str> = DPI_TARGETS.iter().map(|url| url.trim_start_matches("https://")).collect();
    ensure_domains(&dir.join("lists").join("list-general.txt"), &hosts)
}

fn split_args(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    for ch in input.chars() {
        match ch {
            '"' => quoted = !quoted,
            '&' | '|' | '>' if !quoted => { if !current.is_empty() { out.push(std::mem::take(&mut current)); } break; }
            c if c.is_whitespace() && !quoted => { if !current.is_empty() { out.push(std::mem::take(&mut current)); } }
            c => current.push(c),
        }
    }
    if !current.is_empty() { out.push(current); }
    out.into_iter().filter(|arg| arg.starts_with("--")).collect()
}

fn clean_arg(token: &str) -> Option<String> {
    match token.split_once('=') {
        None => Some(token.to_string()),
        Some((key, value)) => {
            let parts: Vec<&str> = value.split(',').map(str::trim).filter(|part| !part.is_empty()).collect();
            if parts.is_empty() { None } else { Some(format!("{key}={}", parts.join(","))) }
        }
    }
}

fn replace_ci(input: &str, needle: &str, replacement: &str) -> String {
    let haystack = input.to_ascii_lowercase();
    let pattern = needle.to_ascii_lowercase();
    let mut out = String::new();
    let mut start = 0;
    while let Some(found) = haystack[start..].find(&pattern) {
        let at = start + found;
        out.push_str(&input[start..at]);
        out.push_str(replacement);
        start = at + needle.len();
    }
    out.push_str(&input[start..]);
    out
}

fn expand_vars(input: &str, vars: &std::collections::HashMap<String, String>) -> String {
    let mut out = String::new();
    let mut chars = input.chars();
    while let Some(ch) = chars.next() {
        if ch != '%' { out.push(ch); continue; }
        let mut name = String::new();
        let mut closed = false;
        for next in chars.by_ref() {
            if next == '%' { closed = true; break; }
            name.push(next);
        }
        if !closed { out.push('%'); out.push_str(&name); continue; }
        if let Some(value) = vars.get(&name.trim().to_ascii_uppercase()) { out.push_str(value); }
    }
    out
}

fn bat_args(path: &Path, dir: &Path) -> Option<Vec<String>> {
    let text = std::fs::read_to_string(path).ok()?;
    let base = format!("{}\\", clean_path(dir).trim_end_matches(['\\', '/']));
    let mut vars: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut joined = String::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('@') || line.starts_with("::") { continue; }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("rem ") || lower.starts_with("rem\t") { continue; }
        if lower.starts_with("set ") && line.contains('=') {
            let body = line[4..].trim().trim_matches('"');
            if let Some((key, value)) = body.split_once('=') {
                vars.insert(key.trim().to_ascii_uppercase(), replace_ci(value.trim().trim_matches('"'), "%~dp0", &base));
            }
            continue;
        }
        match line.strip_suffix('^') {
            Some(head) => { joined.push_str(head.trim_end()); joined.push(' '); }
            None => { joined.push_str(line); joined.push('\n'); }
        }
    }
    for key in ["GAMEFILTER", "GAMEFILTERTCP", "GAMEFILTERUDP"] { vars.entry(key.to_string()).or_insert_with(|| "12".to_string()); }
    vars.insert("~DP0".to_string(), base.clone());
    let lower = joined.to_ascii_lowercase();
    let start = lower.find("winws.exe")? + "winws.exe".len();
    let mut tail = replace_ci(joined.get(start..)?.trim_start_matches('"'), "%~dp0", &base);
    for _ in 0..8 {
        let expanded = expand_vars(&tail, &vars);
        if expanded == tail { break; }
        tail = expanded;
    }
    let args: Vec<String> = split_args(&tail).into_iter().filter_map(|token| clean_arg(&token)).collect();
    if args.iter().any(|arg| arg.starts_with("--wf-")) { Some(args) } else { None }
}

fn discover_presets(dir: &Path) -> Vec<(String, PathBuf, Vec<String>)> {
    let mut found = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() || !path.extension().and_then(|s| s.to_str()).is_some_and(|s| s.eq_ignore_ascii_case("bat")) { continue; }
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or_default().to_string();
            if ["service", "install", "uninstall", "remove", "cleanup", "update"].iter().any(|bad| name.to_ascii_lowercase().contains(bad)) { continue; }
            if let Some(args) = bat_args(&path, dir) { found.push((name, path, args)); }
        }
    }
    found.sort_by(|a, b| a.0.to_ascii_lowercase().cmp(&b.0.to_ascii_lowercase()));
    found
}

async fn kill_winws() {
    let _ = Command::new("taskkill").args(["/F", "/IM", "winws.exe"]).output().await;
    if let Ok(mut guard) = processes().lock() { guard.retain_mut(|child| matches!(child.try_wait(), Ok(None))); }
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
            pipe(&mut child, app);
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

async fn spawn_args(app: &AppHandle, exe: &Path, dir: &Path, args: &[String], label: &str, toast: bool) -> Result<(), String> {
    let exe_path = PathBuf::from(clean_path(exe));
    let work_dir = PathBuf::from(clean_path(dir));
    let args = normalize_args(&work_dir, args);
    let mut cmd = Command::new(&exe_path);
    cmd.args(&args).current_dir(&work_dir);
    spawn_checked(app, cmd, label, toast).await?;
    log(app, "stdout", format!("Zapret started: {label} ({} args, cwd {})", args.len(), clean_path(&work_dir)));
    Ok(())
}

async fn probe(client: &reqwest::Client, url: &str) -> Option<u64> {
    let started = Instant::now();
    let head = client.head(url).send().await;
    let status = match head {
        Ok(response) if response.status().as_u16() < 400 => Some(response.status()),
        _ => client.get(url).send().await.ok().map(|response| response.status()),
    }?;
    let code = status.as_u16();
    if code < 500 && code != 451 { Some(started.elapsed().as_millis() as u64) } else { None }
}

enum Candidate { Bat { name: String, args: Vec<String> }, Builtin(&'static ZapretPreset) }
impl Candidate {
    fn name(&self) -> &str { match self { Candidate::Bat { name, .. } => name, Candidate::Builtin(preset) => preset.name } }
    fn id(&self) -> String {
        match self { Candidate::Bat { name, .. } => name.trim_end_matches(['.','b','a','t','B','A','T']).to_ascii_lowercase(), Candidate::Builtin(preset) => preset.id.to_string() }
    }
    fn args(&self) -> Vec<String> { match self { Candidate::Bat { args, .. } => args.clone(), Candidate::Builtin(preset) => preset.args.iter().map(|arg| arg.to_string()).collect() } }
}

#[tauri::command]
pub async fn scan_dpi_strategies(app: AppHandle) -> Result<serde_json::Value, String> {
    let exe = crate::downloader::ensure_binary(&app, "winws.exe").await?;
    let dir = zapret_root(&exe);
    prepare_scan_lists(&dir)?;
    let mut builder = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .connect_timeout(PROBE_TIMEOUT)
        .timeout(PROBE_TIMEOUT)
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0 Safari/537.36");
    builder = if byedpi_active() {
        log(&app, "stdout", format!("ByeDPI активен, проверка через {BYEDPI_ENDPOINT}"));
        builder.proxy(reqwest::Proxy::all(BYEDPI_ENDPOINT).map_err(|e| e.to_string())?)
    } else { builder.no_proxy() };
    let client = builder.build().map_err(|e| e.to_string())?;
    let mut candidates: Vec<Candidate> = discover_presets(&dir).into_iter()
        .map(|(name, _path, args)| Candidate::Bat { name, args }).collect();
    if candidates.is_empty() {
        log(&app, "stdout", "Конфиги .bat не найдены, используются встроенные пресеты");
        candidates = ZAPRET_PRESETS.iter().map(Candidate::Builtin).collect();
    }
    let total = candidates.len().max(1);
    let services_total = DPI_TARGETS.len();
    let mut results: Vec<(usize, String, String, usize, u64)> = Vec::new();
    let mut first_error: Option<String> = None;
    for (index, candidate) in candidates.iter().enumerate() {
        let name = candidate.name().to_string();
        let id = candidate.id();
        let _ = app.emit("dpi-scan-progress", serde_json::json!({
            "id": id, "name": name, "index": index, "total": total,
            "progress": index * 100 / total, "services_ok": 0, "services_total": services_total,
            "avg_ping_ms": 0, "phase": "start",
        }));
        kill_winws().await;
        let toast = first_error.is_none();
        if let Err(error) = spawn_args(&app, &exe, &dir, &candidate.args(), &name, toast).await {
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
        let mut tasks = tokio::task::JoinSet::new();
        for target in DPI_TARGETS {
            let probe_client = client.clone();
            tasks.spawn(async move { probe(&probe_client, target).await });
        }
        let mut services_ok = 0usize;
        let mut latency_sum = 0u64;
        while let Some(joined) = tasks.join_next().await {
            if let Ok(Some(ms)) = joined { services_ok += 1; latency_sum += ms; }
        }
        let avg_ping_ms = if services_ok > 0 { latency_sum / services_ok as u64 } else { 0 };
        results.push((index, id.clone(), name.clone(), services_ok, avg_ping_ms));
        log(&app, "stdout", format!("{name}: {services_ok}/{services_total} ok, {avg_ping_ms} ms"));
        let _ = app.emit("dpi-scan-progress", serde_json::json!({
            "id": id, "name": name, "index": index, "total": total,
            "progress": (index + 1) * 100 / total, "services_ok": services_ok, "services_total": services_total,
            "avg_ping_ms": avg_ping_ms, "phase": "done",
        }));
    }
    if results.iter().all(|item| item.3 == 0) {
        let detail = first_error.unwrap_or_else(|| "конфиги запустились, но ни один сервис не ответил: проверьте права администратора и драйвер WinDivert".to_string());
        return Err(format!("Сканирование не дало результатов — {detail}"));
    }
    let best = results.iter().max_by(|a, b| a.3.cmp(&b.3).then(b.4.cmp(&a.4))).cloned();
    if let Some((index, id, name, services_ok, avg_ping_ms)) = best {
        kill_winws().await;
        let args = candidates.get(index).map(Candidate::args).unwrap_or_default();
        if let Err(error) = spawn_args(&app, &exe, &dir, &args, &name, false).await {
            log(&app, "stderr", format!("{name} не запущен: {error}"));
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
pub async fn start_zapret(app:AppHandle, preset:String)->Result<(),String> {
    let exe=crate::downloader::ensure_binary(&app,"winws.exe").await?;
    let dir=zapret_root(&exe);
    prepare_lists(&dir)?;
    kill_winws().await;
    let wanted=preset_key(&preset);
    let discovered=discover_presets(&dir);
    if let Some((name,_path,args))=discovered.iter().find(|(name,_,_)| name.eq_ignore_ascii_case(&wanted)) {
        spawn_args(&app,&exe,&dir,args,name,true).await?;
        return Ok(());
    }
    if let Some(p)=find_preset(&preset){
        let args:Vec<String>=p.args.iter().map(|arg|arg.to_string()).collect();
        spawn_args(&app,&exe,&dir,&args,p.name,true).await?;
        return Ok(());
    }
    let path=PathBuf::from(&preset);
    if path.is_file() && path.extension().and_then(|s|s.to_str()).is_some_and(|s|s.eq_ignore_ascii_case("bat")) {
        let mut child=Command::new("cmd").args(["/C",clean_path(&path).as_str()]).current_dir(PathBuf::from(clean_path(&dir))).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|e|e.to_string())?;
        pipe(&mut child,&app);
        processes().lock().map_err(|e|e.to_string())?.push(child);
        return Ok(());
    }
    let fallback=&ZAPRET_PRESETS[0];
    let args:Vec<String>=fallback.args.iter().map(|arg|arg.to_string()).collect();
    spawn_args(&app,&exe,&dir,&args,fallback.name,true).await?;
    log(&app,"stdout",format!("Preset «{preset}» not found, started {}",fallback.name));
    Ok(())
}
#[tauri::command]
pub async fn stop_dpi()->Result<(),String>{kill_winws().await;Ok(())}
#[tauri::command]
pub fn list_zapret_presets(app:AppHandle)->Result<Vec<String>,String>{
    let dirs=[app.path().app_local_data_dir().ok().map(|p|p.join("binaries")),std::env::current_exe().ok().and_then(|p|p.parent().map(|p|p.join("binaries")))];
    for dir in dirs.into_iter().flatten(){
        let found:Vec<String>=discover_presets(&dir).into_iter().map(|(name,_,_)|name).collect();
        if !found.is_empty(){return Ok(found);}
    }
    Ok(ZAPRET_PRESETS.iter().map(|p|p.name.to_string()).collect())
}
#[tauri::command]
pub fn get_dpi_versions()->serde_json::Value{serde_json::json!({"zapret":"winws.exe","byedpi":"ciadpi.exe"})}
#[tauri::command]
pub async fn set_system_dns(interface:String, address:Option<String>)->Result<(),String>{
 let mut cmd=Command::new("netsh");cmd.args(["interface","ip","set","dns",&format!("name={interface}"),"static",address.as_deref().unwrap_or("none"),"primary"]);
 if address.is_none(){cmd=Command::new("netsh");cmd.args(["interface","ip","set","dns",&format!("name={interface}"),"dhcp"]);}
 let output=cmd.output().await.map_err(|e|e.to_string())?;if !output.status.success(){return Err(String::from_utf8_lossy(&output.stderr).to_string());}Ok(())
}
#[tauri::command]
pub async fn repair_windivert()->Result<(),String>{let _=Command::new("taskkill").args(["/F","/IM","winws.exe"]).output().await;let o=Command::new("sc").args(["stop","WinDivert"]).output().await.map_err(|e|e.to_string())?;if !o.status.success(){return Err(String::from_utf8_lossy(&o.stderr).to_string());}Ok(())}
#[tauri::command]
pub async fn update_ipset(app:AppHandle)->Result<String,String>{let dir=bin_dir(&app)?.join("lists");std::fs::create_dir_all(&dir).map_err(|e|e.to_string())?;let body=reqwest::get("https://raw.githubusercontent.com/Flowseal/zapret-discord-youtube/master/ipset-all.txt").await.map_err(|e|e.to_string())?.error_for_status().map_err(|e|e.to_string())?.bytes().await.map_err(|e|e.to_string())?;std::fs::write(dir.join("ipset-all.txt"),body).map_err(|e|e.to_string())?;Ok("IPSet updated".into())}
#[tauri::command]
pub fn update_hosts()->Result<(),String>{Err("Обновление системного hosts не настроено".into())}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_zapret_bat() {
        let dir = Path::new("C:\\zapret");
        let text = "@echo off\nchcp 65001 > nul\nset \"BIN=%~dp0bin\\\"\nset \"LISTS=%~dp0lists\\\"\ncd /d %BIN%\nstart \"zapret: %~n0\" /min \"%BIN%winws.exe\" --wf-tcp=80,443,%GameFilterTCP% ^\n--filter-udp=443 --hostlist=\"%LISTS%list-general.txt\" --hostlist-exclude=\"%LISTS%list-exclude.txt\" --dpi-desync=fake --dpi-desync-repeats=11 --dpi-desync-fake-quic=\"%BIN%quic_initial_www_google_com.bin\" ^\n--filter-tcp=%GameFilterTCP% --dpi-desync=fake,multisplit --dpi-desync-fake-tls-mod=rnd,dupsid,sni=www.google.com\n";
        let path = std::env::temp_dir().join("altfy-zapret-test.bat");
        std::fs::write(&path, text).unwrap();
        let args = bat_args(&path, dir).expect("args");
        let _ = std::fs::remove_file(&path);
        assert!(args.iter().any(|arg| arg == "--wf-tcp=80,443,12"));
        assert!(args.iter().any(|arg| arg == "--filter-udp=443"));
        assert!(args.iter().any(|arg| arg == "--hostlist=C:\\zapret\\lists\\list-general.txt"));
        assert!(args.iter().any(|arg| arg == "--dpi-desync-fake-quic=C:\\zapret\\bin\\quic_initial_www_google_com.bin"));
        assert!(args.iter().any(|arg| arg == "--filter-tcp=12"));
        assert!(args.iter().any(|arg| arg == "--dpi-desync-fake-tls-mod=rnd,dupsid,sni=www.google.com"));
        assert!(!args.iter().any(|arg| arg.contains('%')));
    }
}
