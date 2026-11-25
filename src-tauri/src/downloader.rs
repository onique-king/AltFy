use std::{fs, io::{Cursor, Read, Write}, path::{Path, PathBuf}, time::Duration};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager};

const MAX_DOWNLOAD: u64 = 200 * 1024 * 1024;
const MAX_FILE: u64 = 100 * 1024 * 1024;

#[derive(Deserialize)]
struct Release { assets: Vec<Asset> }
#[derive(Deserialize)]
struct Asset { name: String, browser_download_url: String, digest: Option<String>, size: u64 }

fn progress(app: &AppHandle, name: &str, message: &str, done: bool) {
    let _ = app.emit("download-progress", serde_json::json!({ "name": name, "message": message, "done": done }));
}

fn writable_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let local = app.path().app_local_data_dir().map_err(|e| e.to_string())?.join("binaries");
    let program = std::env::current_exe().ok().and_then(|p| p.parent().map(|p| p.join("binaries")));
    for dir in program.into_iter().chain(std::iter::once(local)) {
        if fs::create_dir_all(&dir).is_ok() && tempfile::tempfile_in(&dir).is_ok() { return Ok(dir); }
    }
    Err("Нет доступной для записи папки binaries".into())
}

fn zapret_complete(exe: &Path) -> bool {
    let Some(dir) = exe.parent() else { return false };
    let root = if dir.file_name().is_some_and(|name| name.eq_ignore_ascii_case("bin")) { dir.parent().unwrap_or(dir) } else { dir };
    root.join("bin").join("winws.exe").is_file()
        && root.join("bin").join("WinDivert.dll").is_file()
        && root.join("bin").join("WinDivert64.sys").is_file()
        && root.join("bin").join("cygwin1.dll").is_file()
        && root.join("lists").is_dir()
        && fs::read_dir(root).ok().is_some_and(|entries| entries.flatten().any(|entry| {
            let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            name.starts_with("general") && name.ends_with(".bat") && entry.path().is_file()
        }))
}

fn find_zapret_root(dir: &Path, depth: u8) -> Option<PathBuf> {
    if dir.join("bin").join("winws.exe").is_file() && dir.join("lists").is_dir()
        && fs::read_dir(dir).ok().is_some_and(|entries| entries.flatten().any(|entry| {
            let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            name.starts_with("general") && name.ends_with(".bat") && entry.path().is_file()
        })) { return Some(dir.to_path_buf()); }
    if depth == 0 { return None; }
    for entry in fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_zapret_root(&path, depth - 1) { return Some(found); }
        }
    }
    None
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), String> {
    fs::create_dir_all(to).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(from).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        let Some(file_name) = path.file_name() else { continue };
        let target = to.join(file_name);
        if path.is_dir() { copy_tree(&path, &target)?; }
        else { fs::copy(&path, &target).map_err(|e| e.to_string())?; }
    }
    Ok(())
}

fn extract_zip(archive: &[u8], dir: &Path, wanted: Option<&[&str]>) -> Result<(), String> {
    let mut zip = zip::ZipArchive::new(Cursor::new(archive)).map_err(|e| e.to_string())?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        let Some(path) = entry.enclosed_name() else { continue };
        let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or_default().to_string();
        let keep = match wanted {
            Some(names) => names.iter().any(|name| file_name.eq_ignore_ascii_case(name)),
            None => true,
        };
        if !keep { continue; }
        if entry.size() > MAX_FILE { return Err(format!("{file_name} слишком велик")); }
        let target = match wanted { Some(_) => dir.join(&file_name), None => dir.join(&path) };
        if entry.is_dir() { fs::create_dir_all(&target).map_err(|e| e.to_string())?; continue; }
        if let Some(parent) = target.parent() { fs::create_dir_all(parent).map_err(|e| e.to_string())?; }
        let mut output = fs::File::create(&target).map_err(|e| e.to_string())?;
        std::io::copy(&mut entry.by_ref().take(MAX_FILE + 1), &mut output).map_err(|e| e.to_string())?;
        if output.metadata().map_err(|e| e.to_string())?.len() > MAX_FILE { return Err("Распакованный файл слишком велик".into()); }
    }
    Ok(())
}

pub async fn ensure_binary(app: &AppHandle, name: &str) -> Result<PathBuf, String> {
    ensure_binary_inner(app, name, false).await
}

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
pub async fn ensure_wintun(exe: &Path) -> Result<(), String> {
    let dir = exe.parent().ok_or("Не удалось определить папку sing-box")?;
    let target = dir.join("wintun.dll");
    if target.is_file() { return Ok(()); }

    let client = reqwest::Client::builder().user_agent("AltFy-client").timeout(Duration::from_secs(120)).build().map_err(|error| error.to_string())?;
    let archive_url = "https://www.wintun.net/builds/wintun-0.14.1.zip";
    let archive = client.get(archive_url).send().await.map_err(|error| error.to_string())?.error_for_status().map_err(|error| error.to_string())?.bytes().await.map_err(|error| error.to_string())?;
    if archive.len() as u64 > MAX_DOWNLOAD { return Err("Архив Wintun слишком велик".into()); }
    let mut zip = zip::ZipArchive::new(Cursor::new(archive)).map_err(|error| error.to_string())?;
    let index = (0..zip.len()).find_map(|index| {
        let entry = zip.by_index(index).ok()?;
        let path = entry.enclosed_name()?.to_string_lossy().replace('\\', "/").to_ascii_lowercase();
        (path.ends_with("wintun/bin/amd64/wintun.dll")).then_some(index)
    }).ok_or("В архиве Wintun отсутствует wintun/bin/amd64/wintun.dll")?;
    let mut file = zip.by_index(index).map_err(|error| error.to_string())?;
    if file.size() > MAX_FILE { return Err("wintun.dll слишком велик".into()); }
    let mut output = fs::File::create(&target).map_err(|error| error.to_string())?;
    std::io::copy(&mut file.by_ref().take(MAX_FILE + 1), &mut output).map_err(|error| error.to_string())?;
    if output.metadata().map_err(|error| error.to_string())?.len() > MAX_FILE { return Err("wintun.dll слишком велик".into()); }
    Ok(())
}

#[cfg(not(all(target_os = "windows", target_arch = "x86_64")))]
pub async fn ensure_wintun(_exe: &Path) -> Result<(), String> { Err("Загрузка Wintun поддерживается только в Windows x64".into()) }

#[tauri::command]
pub async fn update_binary(app: AppHandle, name: String) -> Result<String, String> {
    ensure_binary_inner(&app, &name, true).await.map(|path| path.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn get_binary_status(app: AppHandle) -> Result<serde_json::Value, String> {
    let mut roots = Vec::new();
    if let Ok(path) = app.path().resource_dir() { roots.push(path.join("binaries")); }
    if let Ok(path) = std::env::current_exe() { if let Some(parent) = path.parent() { roots.push(parent.join("binaries")); } }
    if let Ok(path) = app.path().app_local_data_dir() { roots.push(path.join("binaries")); }
    #[cfg(debug_assertions)]
    roots.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries"));
    let installed = |name: &str| roots.iter().any(|root| {
        let path = if name == "winws.exe" { root.join("zapret").join("bin").join(name) } else { root.join(name) };
        let alternate = if name == "winws.exe" { root.join("bin").join(name) } else { path.clone() };
        [path, alternate].iter().any(|path| path.is_file() && (name != "winws.exe" || zapret_complete(path)))
    });
    Ok(serde_json::json!({
        "sing-box.exe": installed("sing-box.exe"),
        "winws.exe": installed("winws.exe"),
        "ciadpi.exe": installed("ciadpi.exe"),
        "tg-ws-proxy.exe": installed("tg-ws-proxy.exe")
    }))
}

#[tauri::command]
pub async fn check_app_updates() -> Result<String, String> {
    let client = reqwest::Client::builder().user_agent("AltFy-client").timeout(Duration::from_secs(8)).build().map_err(|e| e.to_string())?;
    client.get("https://github.com").send().await.map_err(|e| e.to_string())?.error_for_status().map_err(|e| e.to_string())?;
    let response: serde_json::Value = client.get("https://api.github.com/repos/onique-king/AltFy/releases/latest").send().await.map_err(|e| e.to_string())?.error_for_status().map_err(|e| e.to_string())?.json().await.map_err(|e| e.to_string())?;
    response["tag_name"].as_str().map(|tag| tag.trim_start_matches('v').to_string()).ok_or_else(|| "GitHub не вернул версию релиза".into())
}

async fn ensure_binary_inner(app: &AppHandle, name: &str, force: bool) -> Result<PathBuf, String> {
    let name = if name.eq_ignore_ascii_case("byedpi") { "ciadpi.exe" } else { name };
    let (repo, label) = match name {
        "tg-ws-proxy.exe" => ("Flowseal/tg-ws-proxy", "TG-WS Proxy"),
        "sing-box.exe" => ("SagerNet/sing-box", "sing-box"),
        "winws.exe" => ("Flowseal/zapret-discord-youtube", "WinWS"),
        "ciadpi.exe" => ("hufrea/byedpi", "ByeDPI"),
        _ => return Err("Неизвестный модуль".into()),
    };
    let mut roots = Vec::new();
    if let Ok(p) = app.path().resource_dir() { roots.push(p.join("binaries")); }
    if let Ok(p) = std::env::current_exe() { if let Some(p) = p.parent() { roots.push(p.join("binaries")); } }
    if let Ok(p) = app.path().app_local_data_dir() { roots.push(p.join("binaries")); }
    #[cfg(debug_assertions)]
    roots.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries"));
    let paths: Vec<PathBuf> = if name == "winws.exe" {
        roots.iter().map(|root| root.join("zapret").join("bin").join(name)).collect()
    } else {
        roots.iter().map(|root| root.join(name)).collect()
    };
    let existing = paths.into_iter().find(|p| p.is_file() && (name != "winws.exe" || zapret_complete(p)));
    if !force {
        if let Some(path) = existing.clone() { return Ok(path); }
    }
    #[cfg(not(all(target_os = "windows", target_arch = "x86_64")))]
    return Err("Автозагрузка поддерживается только в Windows x64".into());
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        let result = download(app, name, repo, label, force).await;
        match result {
            Ok(path) => Ok(path),
            Err(error) => {
                progress(app, name, &format!("Ошибка загрузки {label}: {error}"), true);
                if force { Err(error) } else { match existing { Some(path) => Ok(path), None => Err(error) } }
            }
        }
    }
    #[cfg(not(all(target_os = "windows", target_arch = "x86_64")))]
    let _ = (repo, label, force);
}

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
async fn download(app: &AppHandle, name: &str, repo: &str, label: &str, force: bool) -> Result<PathBuf, String> {
    progress(app, name, &format!("Загрузка модуля {label}…"), false);
    let client = reqwest::Client::builder().user_agent("AltFy-client").timeout(Duration::from_secs(120)).build().map_err(|e| e.to_string())?;
    let release: Release = client.get(format!("https://api.github.com/repos/{repo}/releases/latest")).send().await.map_err(|e| e.to_string())?.error_for_status().map_err(|e| e.to_string())?.json().await.map_err(|e| e.to_string())?;
    let asset = release.assets.iter().find(|asset| match name {
        "tg-ws-proxy.exe" => asset.name == "TgWsProxy_windows.exe",
        "sing-box.exe" => asset.name.starts_with("sing-box-") && asset.name.ends_with("-windows-amd64.zip"),
        "winws.exe" => asset.name.starts_with("zapret-discord-youtube-") && asset.name.ends_with(".zip"),
        "ciadpi.exe" => asset.name.ends_with(".zip") && (asset.name.contains("windows-x86_64") || asset.name.contains("x86_64-w64")) && !asset.name.contains("i686"),
        _ => false,
    }).ok_or("Подходящий Windows x64 релиз не найден")?;
    if asset.size == 0 || asset.size > MAX_DOWNLOAD { return Err("Недопустимый размер загрузки".into()); }
    let digest = asset.digest.as_deref().and_then(|s| s.strip_prefix("sha256:")).ok_or("Релиз не содержит SHA-256; загрузка отменена")?;
    let mut response = client.get(&asset.browser_download_url).send().await.map_err(|e| e.to_string())?.error_for_status().map_err(|e| e.to_string())?;
    let mut data = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if data.len() as u64 + chunk.len() as u64 > MAX_DOWNLOAD || data.len() as u64 + chunk.len() as u64 > asset.size { return Err("Превышен размер загрузки".into()); }
        data.extend_from_slice(&chunk);
        progress(app, name, &format!("Загрузка {label}: {}%", data.len() as u64 * 100 / asset.size), false);
    }
    if data.len() as u64 != asset.size || format!("{:x}", Sha256::digest(&data)) != digest.to_ascii_lowercase() { return Err("Ошибка проверки SHA-256 или размера".into()); }
    let dir = writable_dir(app)?;
    let staging = tempfile::tempdir_in(&dir).map_err(|e| e.to_string())?;
    if !asset.name.ends_with(".zip") {
        let staged = staging.path().join(name);
        fs::File::create(&staged).and_then(|mut file| file.write_all(&data)).map_err(|e| e.to_string())?;
        if force && dir.join(name).exists() { fs::remove_file(dir.join(name)).map_err(|e| e.to_string())?; }
        fs::rename(&staged, dir.join(name)).map_err(|e| e.to_string())?;
        progress(app, name, &format!("Модуль {label} загружен"), true);
        return Ok(dir.join(name));
    }
    if name == "winws.exe" {
        extract_zip(&data, staging.path(), None)?;
        let root = find_zapret_root(staging.path(), 3).ok_or("В архиве Zapret нет папок bin/ и lists/")?;
        let _ = std::process::Command::new("taskkill").args(["/F", "/IM", "winws.exe"]).output();
        let zapret_dir = dir.join("zapret");
        if force && zapret_dir.exists() { fs::remove_dir_all(&zapret_dir).map_err(|e| e.to_string())?; }
        fs::create_dir_all(&zapret_dir).map_err(|e| e.to_string())?;
        copy_tree(&root, &zapret_dir)?;
        progress(app, name, &format!("Сборка {label} установлена (bin, lists, BAT-стратегии)"), true);
        return Ok(zapret_dir.join("bin").join(name));
    }
    let wanted: &[&str] = match name {
        "sing-box.exe" => &["sing-box.exe"],
        "ciadpi.exe" => &["ciadpi.exe"],
        _ => return Err("Неизвестный архив".into()),
    };
    extract_zip(&data, staging.path(), Some(wanted))?;
    for file in wanted {
        let target = dir.join(file);
        if target.exists() && !force { return Err(format!("Файл {} уже существует; установка отменена", target.display())); }
        if target.exists() { fs::remove_file(&target).map_err(|e| e.to_string())?; }
        fs::rename(staging.path().join(file), target).map_err(|e| e.to_string())?;
    }
    progress(app, name, &format!("Модуль {label} загружен"), true);
    Ok(dir.join(name))
}
