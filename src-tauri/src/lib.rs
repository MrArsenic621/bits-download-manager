mod aria2;
mod bridge;
mod history;
mod model;
mod settings;
mod sync;
mod vault;

use aria2::{Aria2Process, RpcClient};
use model::{Download, Settings, Snapshot, Vault};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use sync::SyncContext;
use tauri::Manager;

/* ================= App State ================= */

struct AppState {
    client: Option<Arc<RpcClient>>,
    process: Mutex<Option<Aria2Process>>,
    sync: Arc<SyncContext>,
    settings: Arc<Mutex<Settings>>,
    settings_path: PathBuf,
    vault: Arc<Mutex<Vault>>,
    vault_path: PathBuf,
}

impl AppState {
    fn client(&self) -> Result<Arc<RpcClient>, String> {
        self.client
            .clone()
            .ok_or_else(|| "aria2 engine is not running".into())
    }
}

/* ================= Commands ================= */

#[tauri::command]
fn get_snapshot(state: tauri::State<AppState>) -> Snapshot {
    state.sync.snapshot.lock().unwrap().clone()
}

#[tauri::command]
async fn add_download(
    state: tauri::State<'_, AppState>,
    uri: String,
    dir: Option<String>,
    out: Option<String>,
    split: Option<u32>,
    referer: Option<String>,
    user_agent: Option<String>,
    cookie: Option<String>,
    auth_user: Option<String>,
    auth_pass: Option<String>,
) -> Result<String, String> {
    let uri = uri.trim().to_string();
    if uri.is_empty() {
        return Err("URL is empty".into());
    }

    let client = state.client()?;
    let sync = state.sync.clone();
    let defaults = state.settings.lock().unwrap().clone();
    let app_data = state
        .settings_path
        .parent()
        .map(PathBuf::from)
        .unwrap_or_default();
    
    let host = url::Url::parse(&uri)
        .ok()
        .and_then(|u| u.domain().map(|d| d.to_string()));
    
    let mut auth_user = auth_user;
    let mut auth_pass = auth_pass;
    let mut cookies = cookie;

    if let Some(host) = host {
        let vault = state.vault.lock().unwrap();
        if let Some(entry) = vault.0.iter().find(|e| host.ends_with(&e.domain)) {
            if auth_user.is_none() { auth_user = entry.auth_user.clone(); }
            if auth_pass.is_none() { auth_pass = entry.auth_pass.clone(); }
            if cookies.is_none() { cookies = entry.cookies.clone(); }
        }
    }

    tauri::async_runtime::spawn_blocking(move || {
        let mut options = serde_json::Map::new();

        let resolved_dir = if let Some(custom_dir) = dir.filter(|d| !d.trim().is_empty()) {
            custom_dir
        } else {
            let base = if !defaults.default_dir.is_empty() {
                PathBuf::from(&defaults.default_dir)
            } else {
                aria2::default_download_dir(&app_data)
            };
            if defaults.auto_categorize {
                let target_name = out.as_deref().unwrap_or(&uri);
                if let Some(category) = model::detect_category(target_name) {
                    base.join(category).to_string_lossy().into_owned()
                } else {
                    base.to_string_lossy().into_owned()
                }
            } else {
                base.to_string_lossy().into_owned()
            }
        };

        std::fs::create_dir_all(&resolved_dir).map_err(|e| e.to_string())?;
        options.insert("dir".into(), Value::String(resolved_dir));

        if let Some(out) = out.as_ref() {
            if !out.is_empty() {
                options.insert("out".into(), Value::String(out.clone()));
            }
        }
        let split = split.unwrap_or(defaults.default_split).clamp(1, 64);
        options.insert("split".into(), Value::String(split.to_string()));
        options.insert(
            "max-connection-per-server".into(),
            Value::String(split.to_string()),
        );
        options.insert("continue".into(), Value::String("true".into()));

        if let Some(r) = referer.filter(|s| !s.trim().is_empty()) {
            options.insert("referer".into(), Value::String(r.trim().to_string()));
        }
        if let Some(ua) = user_agent.filter(|s| !s.trim().is_empty()) {
            options.insert("user-agent".into(), Value::String(ua.trim().to_string()));
        }
        if let Some(c) = cookies.filter(|s| !s.trim().is_empty()) {
            options.insert(
                "header".into(),
                Value::Array(vec![Value::String(format!("Cookie: {}", c.trim()))]),
            );
        }
        if let Some(u) = auth_user.filter(|s| !s.trim().is_empty()) {
            options.insert("http-user".into(), Value::String(u.trim().to_string()));
        }
        if let Some(p) = auth_pass.filter(|s| !s.trim().is_empty()) {
            options.insert("http-passwd".into(), Value::String(p.trim().to_string()));
        }

        let gid = client.add_uri(&[uri], Some(Value::Object(options)))?;
        sync.created
            .lock()
            .unwrap()
            .insert(gid.clone(), sync::now());

        Ok::<String, String>(gid)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn pause_download(gid: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let client = state.client()?;
    tauri::async_runtime::spawn_blocking(move || client.pause(&gid))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn resume_download(gid: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let client = state.client()?;
    tauri::async_runtime::spawn_blocking(move || client.unpause(&gid))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn pause_all(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let client = state.client()?;
    tauri::async_runtime::spawn_blocking(move || client.pause_all())
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn resume_all(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let client = state.client()?;
    tauri::async_runtime::spawn_blocking(move || client.unpause_all())
        .await
        .map_err(|e| e.to_string())?
}

/// Remove from the queue and drop it from the UI/history (files are kept).
#[tauri::command]
async fn remove_download(gid: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let client = state.client()?;
    let sync = state.sync.clone();
    tauri::async_runtime::spawn_blocking(move || {
        // remove() only works on active/waiting downloads; ignore "not found".
        if let Err(e) = client.remove(&gid) {
            if !e.contains("not found") {
                return Err(e);
            }
        }
        let _ = client.remove_download_result(&gid);
        sync.history.lock().unwrap().remove(&gid);
        sync.created.lock().unwrap().remove(&gid);
        sync.persist_history();
        Ok::<(), String>(())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Remove from queue and delete the file(s) from disk.
#[tauri::command]
async fn delete_download(gid: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let client = state.client()?;
    let sync = state.sync.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let target = lookup_download(&sync, &gid);

        if let Err(e) = client.remove(&gid) {
            if !e.contains("not found") {
                return Err(e);
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
        let _ = client.remove_download_result(&gid);

        if let Some(d) = target {
            delete_file(&d.file_path());
        }
        sync.history.lock().unwrap().remove(&gid);
        sync.created.lock().unwrap().remove(&gid);
        sync.persist_history();
        Ok::<(), String>(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn clear_finished(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let client = state.client()?;
    let sync = state.sync.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut history = sync.history.lock().unwrap();
        let gids: Vec<String> = history.keys().cloned().collect();
        for gid in &gids {
            let _ = client.remove_download_result(gid);
            history.remove(gid);
        }
        sync.created.lock().unwrap().clear();
        drop(history);
        sync.persist_history();
        Ok::<(), String>(())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Add a .torrent file from disk. Magnet links work via add_download.
#[tauri::command]
async fn add_torrent(
    path: String,
    dir: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let client = state.client()?;
    let sync = state.sync.clone();
    let defaults = state.settings.lock().unwrap().clone();

    tauri::async_runtime::spawn_blocking(move || {
        use base64::Engine as _;
        let bytes = std::fs::read(&path)
            .map_err(|e| format!("failed to read torrent file: {e}"))?;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);

        let mut options = serde_json::Map::new();
        let dir = dir.or(if defaults.default_dir.is_empty() {
            None
        } else {
            Some(defaults.default_dir.clone())
        });
        if let Some(dir) = dir.as_ref() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            options.insert("dir".into(), Value::String(dir.clone()));
        }
        options.insert("split".into(), Value::String(defaults.default_split.to_string()));

        let opts = if options.is_empty() {
            None
        } else {
            Some(Value::Object(options))
        };

        let gid = client.add_torrent(&b64, opts)?;
        sync.created
            .lock()
            .unwrap()
            .insert(gid.clone(), sync::now());

        Ok::<String, String>(gid)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn get_settings(state: tauri::State<AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
async fn update_settings(
    state: tauri::State<'_, AppState>,
    settings: Settings,
) -> Result<(), String> {
    let settings = settings::sanitize(settings);

    let client = state.client.clone();
    let settings_arc = state.settings.clone();
    let path = state.settings_path.clone();

    tauri::async_runtime::spawn_blocking(move || {
        // Apply the settings that aria2 supports at runtime.
        if let Some(client) = client.as_ref() {
            let options = serde_json::json!({
                "max-concurrent-downloads": settings.max_concurrent_downloads.to_string(),
                "max-overall-download-limit": settings.global_speed_limit.to_string(),
            });
            if let Err(e) = client.change_global_option(options) {
                return Err(e);
            }
        }
        if let Ok(mut guard) = settings_arc.lock() {
            *guard = settings.clone();
        }
        settings::save(&path, &settings);
        Ok::<(), String>(())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Set a per-download download speed limit in bytes/sec (0 = unlimited).
#[tauri::command]
async fn set_speed_limit(
    gid: String,
    limit: u64,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let client = state.client()?;
    tauri::async_runtime::spawn_blocking(move || {
        client.change_option(&gid, serde_json::json!({ "max-download-limit": limit.to_string() }))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn get_vault(state: tauri::State<AppState>) -> Vault {
    state.vault.lock().unwrap().clone()
}

#[tauri::command]
fn update_vault(state: tauri::State<AppState>, vault: Vault) -> Result<(), String> {
    let path = state.vault_path.clone();
    if let Ok(mut guard) = state.vault.lock() {
        *guard = vault.clone();
    }
    vault::save(&path, &vault);
    Ok(())
}
#[tauri::command]
async fn calculate_checksum(path: String, algorithm: String) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;

    tauri::async_runtime::spawn_blocking(move || {
        let algo_upper = algorithm.to_uppercase();
        if algo_upper == "SHA-256" || algo_upper == "SHA256" {
            let mut file = std::fs::File::open(&path)
                .map_err(|e| format!("failed to open file {path}: {e}"))?;
            let mut buffer = [0u8; 64 * 1024];
            let mut hasher = Sha256::new();
            loop {
                let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
                if n == 0 {
                    break;
                }
                hasher.update(&buffer[..n]);
            }
            Ok(hex::encode(hasher.finalize()))
        } else if algo_upper == "SHA-1" || algo_upper == "SHA1" || algo_upper == "MD5" {
            let cert_algo = if algo_upper.starts_with("SHA") { "SHA1" } else { "MD5" };
            let mut cmd = std::process::Command::new("certutil");
            cmd.args(["-hashfile", &path, cert_algo]);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x08000000);
            }
            let output = cmd.output().map_err(|e| format!("failed to run hash utility: {e}"))?;
            let text = String::from_utf8_lossy(&output.stdout);
            let lines: Vec<&str> = text.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
            if lines.len() >= 2 {
                let raw_hash = lines[1].replace(' ', "").to_lowercase();
                if !raw_hash.is_empty() {
                    return Ok(raw_hash);
                }
            }
            Err("failed to parse hash output".into())
        } else {
            Err(format!("Unsupported algorithm: {algorithm}"))
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Trigger Windows shutdown, sleep, or hibernate.
#[tauri::command]
async fn shutdown_pc(action: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut cmd = match action.as_str() {
            "sleep" => {
                let mut c = std::process::Command::new("powershell");
                c.args(["-Command", "rundll32.exe powrprof.dll,SetSuspendState 0,1,0"]);
                c
            }
            "hibernate" => {
                let mut c = std::process::Command::new("shutdown");
                c.args(["/h"]);
                c
            }
            _ => {
                let mut c = std::process::Command::new("shutdown");
                c.args(["/s", "/t", "30"]);
                c
            }
        };
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }
        cmd.spawn().map_err(|e| format!("failed to trigger shutdown: {e}"))?;
        Ok::<(), String>(())
    })
    .await
    .map_err(|e| e.to_string())?
}

fn lookup_download(sync: &SyncContext, gid: &str) -> Option<Download> {
    let snapshot = sync.snapshot.lock().unwrap();
    snapshot
        .downloads
        .iter()
        .find(|d| d.gid == gid)
        .cloned()
        .or_else(|| sync.history.lock().unwrap().get(gid).cloned())
}

fn delete_file(path: &str) {
    let p = std::path::Path::new(path);
    if p.exists() {
        let _ = std::fs::remove_file(p);
    }
    let ctrl = format!("{path}.aria2");
    let ctrl_path = std::path::Path::new(&ctrl);
    if ctrl_path.exists() {
        let _ = std::fs::remove_file(ctrl_path);
    }
}

/* ================= App Entry ================= */

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let handle = app.handle().clone();

            let app_data = handle
                .path()
                .app_data_dir()
                .expect("failed to resolve app data dir");
            let settings_path = app_data.join("settings.json");
            let settings = settings::load(&settings_path);
            let settings = settings::sanitize(settings);

            let (client, process, startup_error) = match Aria2Process::start(&handle, &settings) {
                Ok((client, process)) => (Some(client), Some(process), None),
                Err(e) => {
                    eprintln!("failed to start aria2: {e}");
                    (None, None, Some(e))
                }
            };

            let history_path = app_data.join("history.json");
            let vault_path = app_data.join("vault.json");

            let sync = Arc::new(SyncContext {
                snapshot: Arc::new(Mutex::new(Snapshot::default())),
                history: Arc::new(Mutex::new(history::load(&history_path))),
                created: Arc::new(Mutex::new(Default::default())),
                history_path,
                last_persisted: Arc::new(Mutex::new(String::new())),
            });

            {
                let mut snap = sync.snapshot.lock().unwrap();
                snap.startup_error = startup_error;
            }

            if let Some(client) = client.as_ref() {
                sync::run_sync(handle.clone(), client.clone(), sync.clone());
                bridge::start_bridge_server(
                    client.clone(),
                    sync.clone(),
                    app_data.clone(),
                    settings.default_dir.clone(),
                    settings.default_split,
                    settings.auto_categorize,
                );
            }

            app.manage(AppState {
                client,
                process: Mutex::new(process),
                sync,
                settings: Arc::new(Mutex::new(settings)),
                settings_path,
                vault: Arc::new(Mutex::new(vault::load(&vault_path))),
                vault_path,
            });

            use tauri::menu::{Menu, MenuItem};
            use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

            let show_i = MenuItem::with_id(app, "show", "Show / Hide Bits", true, None::<&str>)?;
            let resume_i = MenuItem::with_id(app, "resume_all", "Resume All", true, None::<&str>)?;
            let pause_i = MenuItem::with_id(app, "pause_all", "Pause All", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit Bits", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &resume_i, &pause_i, &quit_i])?;

            let mut builder = TrayIconBuilder::new()
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app: &tauri::AppHandle, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            if let Ok(visible) = window.is_visible() {
                                if visible {
                                    let _ = window.hide();
                                } else {
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                }
                            }
                        }
                    }
                    "resume_all" => {
                        if let Some(state) = app.try_state::<AppState>() {
                            if let Ok(client) = state.client() {
                                let _ = client.unpause_all();
                            }
                        }
                    }
                    "pause_all" => {
                        if let Some(state) = app.try_state::<AppState>() {
                            if let Ok(client) = state.client() {
                                let _ = client.pause_all();
                            }
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray: &tauri::tray::TrayIcon, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            if let Ok(visible) = window.is_visible() {
                                if visible {
                                    let _ = window.hide();
                                } else {
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                }
                            }
                        }
                    }
                });

            if let Some(icon) = app.default_window_icon() {
                builder = builder.icon(icon.clone());
            }

            let _ = builder.build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle();
                if let Some(state) = app.try_state::<AppState>() {
                    let close_to_tray = state
                        .settings
                        .lock()
                        .map(|s| s.close_to_tray)
                        .unwrap_or(true);
                    if close_to_tray {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            add_download,
            add_torrent,
            pause_download,
            resume_download,
            pause_all,
            resume_all,
            remove_download,
            delete_download,
            clear_finished,
            get_settings,
            update_settings,
            set_speed_limit,
            calculate_checksum,
            shutdown_pc,
            get_vault,
            update_vault,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                if let Some(state) = app_handle.try_state::<AppState>() {
                    if let Some(process) = state.process.lock().unwrap().take() {
                        if let Some(client) = state.client.as_ref() {
                            process.stop(client);
                        }
                    }
                }
            }
        });
}
