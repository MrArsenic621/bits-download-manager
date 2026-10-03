use crate::aria2::RpcClient;
use crate::model;
use crate::sync::{self, SyncContext};
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Deserialize)]
struct BridgeDownloadPayload {
    url: String,
    dir: Option<String>,
    out: Option<String>,
    split: Option<u32>,
    referer: Option<String>,
    #[serde(rename = "userAgent")]
    user_agent: Option<String>,
    cookies: Option<String>,
}

pub fn start_bridge_server(
    client: Arc<RpcClient>,
    sync: Arc<SyncContext>,
    app_data: PathBuf,
    default_dir: String,
    default_split: u32,
    auto_categorize: bool,
) {
    std::thread::spawn(move || {
        let listener = match TcpListener::bind("127.0.0.1:6801") {
            Ok(l) => l,
            Err(_) => return, // Port already in use or bridge disabled
        };

        for stream in listener.incoming() {
            if let Ok(mut stream) = stream {
                let client = client.clone();
                let sync = sync.clone();
                let app_data = app_data.clone();
                let default_dir = default_dir.clone();

                std::thread::spawn(move || {
                    handle_client(
                        &mut stream,
                        client,
                        sync,
                        app_data,
                        default_dir,
                        default_split,
                        auto_categorize,
                    );
                });
            }
        }
    });
}

fn handle_client(
    stream: &mut TcpStream,
    client: Arc<RpcClient>,
    sync: Arc<SyncContext>,
    app_data: PathBuf,
    default_dir: String,
    default_split: u32,
    auto_categorize: bool,
) {
    let mut buffer = [0u8; 8192];
    let n = match stream.read(&mut buffer) {
        Ok(n) if n > 0 => n,
        _ => return,
    };

    let req = String::from_utf8_lossy(&buffer[..n]);
    let lines: Vec<&str> = req.lines().collect();
    if lines.is_empty() {
        return;
    }

    let first_line = lines[0];
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() < 2 {
        return;
    }

    let method = parts[0];
    let path = parts[1];

    if method == "OPTIONS" {
        let response = "HTTP/1.1 204 No Content\r\n\
                        Access-Control-Allow-Origin: *\r\n\
                        Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
                        Access-Control-Allow-Headers: Content-Type, Authorization\r\n\
                        Connection: close\r\n\r\n";
        let _ = stream.write_all(response.as_bytes());
        return;
    }

    if method == "GET" && (path == "/ping" || path == "/status") {
        let body = json!({
            "status": "ok",
            "app": "Bits Download Manager",
            "version": "2.0.0"
        })
        .to_string();

        let response = format!(
            "HTTP/1.1 200 OK\r\n\
             Content-Type: application/json\r\n\
             Access-Control-Allow-Origin: *\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = stream.write_all(response.as_bytes());
        return;
    }

    if method == "POST" && (path == "/add" || path == "/download") {
        // Find body after double newline
        let body_str = if let Some(idx) = req.find("\r\n\r\n") {
            &req[idx + 4..]
        } else if let Some(idx) = req.find("\n\n") {
            &req[idx + 2..]
        } else {
            ""
        };

        if let Ok(payload) = serde_json::from_str::<BridgeDownloadPayload>(body_str) {
            let uri = payload.url.trim().to_string();
            if !uri.is_empty() {
                let resolved_dir = if let Some(custom_dir) = payload.dir.filter(|d| !d.trim().is_empty()) {
                    custom_dir
                } else {
                    let base = if !default_dir.is_empty() {
                        PathBuf::from(&default_dir)
                    } else {
                        crate::aria2::default_download_dir(&app_data)
                    };
                    if auto_categorize {
                        let target_name = payload.out.as_deref().unwrap_or(&uri);
                        if let Some(category) = model::detect_category(target_name) {
                            base.join(category).to_string_lossy().into_owned()
                        } else {
                            base.to_string_lossy().into_owned()
                        }
                    } else {
                        base.to_string_lossy().into_owned()
                    }
                };

                let _ = std::fs::create_dir_all(&resolved_dir);

                let mut options = serde_json::Map::new();
                options.insert("dir".into(), Value::String(resolved_dir));

                if let Some(out) = payload.out.filter(|s| !s.is_empty()) {
                    options.insert("out".into(), Value::String(out));
                }

                let split = payload.split.unwrap_or(default_split).clamp(1, 64);
                options.insert("split".into(), Value::String(split.to_string()));
                options.insert("max-connection-per-server".into(), Value::String(split.to_string()));
                options.insert("continue".into(), Value::String("true".into()));

                if let Some(r) = payload.referer.filter(|s| !s.trim().is_empty()) {
                    options.insert("referer".into(), Value::String(r));
                }
                if let Some(ua) = payload.user_agent.filter(|s| !s.trim().is_empty()) {
                    options.insert("user-agent".into(), Value::String(ua));
                }
                if let Some(c) = payload.cookies.filter(|s| !s.trim().is_empty()) {
                    options.insert(
                        "header".into(),
                        Value::Array(vec![Value::String(format!("Cookie: {}", c.trim()))]),
                    );
                }

                match client.add_uri(&[uri], Some(Value::Object(options))) {
                    Ok(gid) => {
                        sync.created.lock().unwrap().insert(gid.clone(), sync::now());
                        let res_body = json!({ "success": true, "gid": gid }).to_string();
                        let response = format!(
                            "HTTP/1.1 200 OK\r\n\
                             Content-Type: application/json\r\n\
                             Access-Control-Allow-Origin: *\r\n\
                             Content-Length: {}\r\n\
                             Connection: close\r\n\r\n{}",
                            res_body.len(),
                            res_body
                        );
                        let _ = stream.write_all(response.as_bytes());
                        return;
                    }
                    Err(e) => {
                        let res_body = json!({ "success": false, "error": e }).to_string();
                        let response = format!(
                            "HTTP/1.1 500 Internal Server Error\r\n\
                             Content-Type: application/json\r\n\
                             Access-Control-Allow-Origin: *\r\n\
                             Content-Length: {}\r\n\
                             Connection: close\r\n\r\n{}",
                            res_body.len(),
                            res_body
                        );
                        let _ = stream.write_all(response.as_bytes());
                        return;
                    }
                }
            }
        }

        let res_body = json!({ "error": "Invalid request payload" }).to_string();
        let response = format!(
            "HTTP/1.1 400 Bad Request\r\n\
             Content-Type: application/json\r\n\
             Access-Control-Allow-Origin: *\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\r\n{}",
            res_body.len(),
            res_body
        );
        let _ = stream.write_all(response.as_bytes());
        return;
    }

    let response = "HTTP/1.1 404 Not Found\r\n\
                    Content-Length: 0\r\n\
                    Connection: close\r\n\r\n";
    let _ = stream.write_all(response.as_bytes());
}
