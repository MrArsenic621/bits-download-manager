mod aria2;
mod history;
mod model;
mod settings;
mod sync;

use aria2::{Aria2Process, RpcClient};
use model::{Download, Settings, Snapshot};
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
) -> Result<String, String> {
    let uri = uri.trim().to_string();
    if uri.is_empty() {
        return Err("URL is empty".into());
    }

    let client = state.client()?;
    let sync = state.sync.clone();
    let defaults = state.settings.lock().unwrap().clone();

    tauri::async_runtime::spawn_blocking(move || {
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
            }

            app.manage(AppState {
                client,
                process: Mutex::new(process),
                sync,
                settings: Arc::new(Mutex::new(settings)),
                settings_path,
            });

            Ok(())
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
