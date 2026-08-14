use crate::aria2::RpcClient;
use crate::history;
use crate::model::{Download, GlobalStat, Snapshot};
use serde_json::Value;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
use tauri::{AppHandle, Emitter};

pub struct SyncContext {
    pub snapshot: Arc<Mutex<Snapshot>>,
    pub history: Arc<Mutex<HashMap<String, Download>>>,
    pub created: Arc<Mutex<HashMap<String, u64>>>,
    pub history_path: PathBuf,
    pub last_persisted: Arc<Mutex<String>>,
}

impl SyncContext {
    pub fn persist_history(&self) {
        let history = self.history.lock().unwrap();
        let mut last = self.last_persisted.lock().unwrap();
        history::persist_if_changed(&self.history_path, &history, &mut last);
    }
}

pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn as_u64(v: Option<&Value>) -> u64 {
    v.and_then(|x| x.as_str())
        .and_then(|s| s.parse().ok())
        .or_else(|| v.and_then(|x| x.as_u64()))
        .unwrap_or(0)
}

fn value_to_download(raw: &Value, created_at: u64) -> Download {
    let gid = raw
        .get("gid")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let dir = raw
        .get("dir")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let files = raw.get("files").and_then(|f| f.as_array()).and_then(|f| f.first());
    let uri = files
        .and_then(|f| f.get("uris"))
        .and_then(|u| u.as_array())
        .and_then(|u| u.first())
        .and_then(|x| x.get("uri"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let path = files
        .and_then(|f| f.get("path"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let bittorrent_name = raw
        .get("bittorrent")
        .and_then(|b| b.get("info"))
        .and_then(|i| i.get("name"))
        .and_then(|v| v.as_str());

    let filename = bittorrent_name
        .map(|s| s.to_string())
        .or_else(|| {
            std::path::Path::new(&path)
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| uri_to_filename(&uri));

    let status = raw
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let total = as_u64(raw.get("totalLength"));
    let completed = as_u64(raw.get("completedLength"));
    let speed = as_u64(raw.get("downloadSpeed"));
    let upload = as_u64(raw.get("uploadSpeed"));
    let error_message = raw
        .get("errorMessage")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let progress = if total > 0 {
        (completed as f64 / total as f64) * 100.0
    } else {
        0.0
    };
    let eta_secs = if status == "active" && speed > 0 && total > completed {
        Some(((total - completed) as f64 / speed as f64) as u64)
    } else {
        None
    };

    Download {
        gid,
        uri,
        filename,
        dir,
        status,
        total_length: total,
        completed_length: completed,
        download_speed: speed,
        upload_speed: upload,
        progress,
        eta_secs,
        error_message,
        created_at,
    }
}

fn uri_to_filename(uri: &str) -> String {
    uri.rsplit('/')
        .find(|seg| !seg.is_empty())
        .unwrap_or(uri)
        .split('?')
        .next()
        .unwrap_or("")
        .to_string()
}

fn created_for(ctx: &SyncContext, gid: &str, default: u64) -> u64 {
    let mut created = ctx.created.lock().unwrap();
    if let Some(c) = created.get(gid) {
        return *c;
    }
    created.insert(gid.to_string(), default);
    default
}

/// Query aria2 and assemble a fresh snapshot.
fn collect(client: &RpcClient, ctx: &SyncContext) -> Result<Snapshot, String> {
    let active = client.tell_active()?;
    let waiting = client.tell_waiting(0, 1000)?;
    let stopped = client.tell_stopped(0, 500)?;
    let gstat = client.get_global_stat()?;

    let now = now();
    let mut downloads: Vec<Download> = Vec::new();
    let mut seen: Vec<String> = Vec::new();

    for raw in active.iter().chain(waiting.iter()) {
        let d = value_to_download(raw, created_for(ctx, gid_of(raw), now));
        seen.push(d.gid.clone());
        downloads.push(d);
    }

    {
        let mut history = ctx.history.lock().unwrap();
        for raw in stopped.iter() {
            let status = raw
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let gid = gid_of(raw);

            // "removed" entries are cleanup leftovers; purge them.
            if status == "removed" {
                let _ = client.remove_download_result(gid);
                history.remove(gid);
                continue;
            }

            let d = value_to_download(raw, created_for(ctx, gid, now));
            seen.push(d.gid.clone());
            if status == "complete" || status == "error" {
                history.insert(d.gid.clone(), d.clone());
            }
            downloads.push(d);
        }

        // Fold in history entries aria2 no longer reports (survives restarts).
        for d in history.values() {
            if !seen.contains(&d.gid) {
                downloads.push(d.clone());
            }
        }
    }

    ctx.persist_history();

    let global = GlobalStat {
        download_speed: as_u64(gstat.get("downloadSpeed")),
        upload_speed: as_u64(gstat.get("uploadSpeed")),
        num_active: as_u64(gstat.get("numActive")),
        num_waiting: as_u64(gstat.get("numWaiting")),
        num_stopped: as_u64(gstat.get("numStopped")),
    };

    Ok(Snapshot {
        downloads,
        global,
        aria2_version: None,
        startup_error: None,
    })
}

fn gid_of(raw: &Value) -> &str {
    raw.get("gid").and_then(|v| v.as_str()).unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_aria2_payload_to_download() {
        let raw = serde_json::json!({
            "gid": "abc123",
            "status": "active",
            "totalLength": "1048576",
            "completedLength": "524288",
            "downloadSpeed": "131072",
            "uploadSpeed": "0",
            "errorCode": "0",
            "errorMessage": "",
            "dir": "C:\\Users\\test\\Downloads",
            "files": [{
                "path": "C:\\Users\\test\\Downloads\\file.bin",
                "uris": [{"uri": "https://example.com/file.bin"}]
            }]
        });

        let d = value_to_download(&raw, 42);

        assert_eq!(d.gid, "abc123");
        assert_eq!(d.status, "active");
        assert_eq!(d.filename, "file.bin");
        assert_eq!(d.uri, "https://example.com/file.bin");
        assert_eq!(d.total_length, 1048576);
        assert_eq!(d.completed_length, 524288);
        assert_eq!(d.download_speed, 131072);
        assert!((d.progress - 50.0).abs() < 0.001);
        assert_eq!(d.eta_secs, Some(4));
        assert_eq!(d.created_at, 42);
        assert_eq!(d.file_path(), "C:\\Users\\test\\Downloads\\file.bin");
    }

    #[test]
    fn falls_back_to_uri_for_filename() {
        let raw = serde_json::json!({
            "gid": "x",
            "status": "error",
            "files": [{ "path": "", "uris": [{"uri": "https://example.com/dir/archive.zip?v=1"}] }],
            "errorMessage": "Connection reset"
        });
        let d = value_to_download(&raw, 0);
        assert_eq!(d.filename, "archive.zip");
        assert_eq!(d.error_message.as_deref(), Some("Connection reset"));
        assert_eq!(d.status, "error");
    }
}

/// Spawn the background poller that keeps the shared snapshot fresh and
/// pushes updates to the frontend over a Tauri event.
pub fn run_sync(app: AppHandle, client: Arc<RpcClient>, ctx: Arc<SyncContext>) {
    std::thread::spawn(move || {
        let mut version: Option<String> = None;
        loop {
            if let Ok(v) = client.get_version() {
                version = Some(v);
            }

            match collect(&client, &ctx) {
                Ok(mut snap) => {
                    snap.aria2_version = version.clone();
                    let payload = snap.clone();
                    if let Ok(mut guard) = ctx.snapshot.lock() {
                        *guard = snap;
                    }
                    let _ = app.emit("downloads://update", payload);
                }
                Err(e) => {
                    let payload = serde_json::json!({ "error": e });
                    let _ = app.emit("downloads://update", payload);
                }
            }

            std::thread::sleep(Duration::from_secs(1));
        }
    });
}
