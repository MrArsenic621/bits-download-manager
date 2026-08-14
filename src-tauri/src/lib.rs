mod aria2;
mod history;
mod model;
mod sync;

use aria2::{Aria2Process, RpcClient};
use model::{Download, Snapshot};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use sync::SyncContext;
use tauri::Manager;

/* ================= App State ================= */

struct AppState {
    client: Arc<RpcClient>,
    process: Mutex<Option<Aria2Process>>,
    sync: Arc<SyncContext>,
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

    let client = state.client.clone();
    let sync = state.sync.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let mut options = serde_json::Map::new();

        if let Some(dir) = dir.as_ref() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            options.insert("dir".into(), Value::String(dir.clone()));
        }
        if let Some(out) = out.as_ref() {
            if !out.is_empty() {
                options.insert("out".into(), Value::String(out.clone()));
            }
        }
        let split = split.unwrap_or(16).clamp(1, 64);
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
    let client = state.client.clone();
    tauri::async_runtime::spawn_blocking(move || client.pause(&gid))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn resume_download(gid: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let client = state.client.clone();
    tauri::async_runtime::spawn_blocking(move || client.unpause(&gid))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn pause_all(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let client = state.client.clone();
    tauri::async_runtime::spawn_blocking(move || client.pause_all())
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn resume_all(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let client = state.client.clone();
    tauri::async_runtime::spawn_blocking(move || client.unpause_all())
        .await
        .map_err(|e| e.to_string())?
}

/// Remove from the queue and drop it from the UI/history (files are kept).
#[tauri::command]
async fn remove_download(gid: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let client = state.client.clone();
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
    let client = state.client.clone();
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
    let client = state.client.clone();
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
        .setup(|app| {
            let handle = app.handle().clone();
            let (client, process) = Aria2Process::start(&handle).expect("failed to start aria2");

            let app_data = handle
                .path()
                .app_data_dir()
                .expect("failed to resolve app data dir");
            let history_path = app_data.join("history.json");

            let sync = Arc::new(SyncContext {
                snapshot: Arc::new(Mutex::new(Snapshot::default())),
                history: Arc::new(Mutex::new(history::load(&history_path))),
                created: Arc::new(Mutex::new(Default::default())),
                history_path,
                last_persisted: Arc::new(Mutex::new(String::new())),
            });

            sync::run_sync(handle.clone(), client.clone(), sync.clone());

            app.manage(AppState {
                client,
                process: Mutex::new(Some(process)),
                sync,
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            add_download,
            pause_download,
            resume_download,
            pause_all,
            resume_all,
            remove_download,
            delete_download,
            clear_finished,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                if let Some(state) = app_handle.try_state::<AppState>() {
                    let client = state.client.clone();
                    if let Some(process) = state.process.lock().unwrap().take() {
                        process.stop(&client);
                    }
                }
            }
        });
}
