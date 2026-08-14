use rand::{distributions::Alphanumeric, Rng};
use serde_json::{json, Value};
use std::{
    fs,
    net::TcpListener,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};
use tauri::{AppHandle, Manager};

const RPC_KEYS: [&str; 13] = [
    "gid",
    "status",
    "totalLength",
    "completedLength",
    "downloadSpeed",
    "uploadSpeed",
    "errorCode",
    "errorMessage",
    "dir",
    "files",
    "bittorrent",
    "infoHash",
    "connections",
];

/* ================= JSON-RPC client ================= */

pub struct RpcClient {
    port: u16,
    secret: String,
    http: reqwest::blocking::Client,
    next_id: AtomicU64,
}

impl RpcClient {
    pub fn new(port: u16, secret: &str) -> Self {
        RpcClient {
            port,
            secret: secret.to_string(),
            http: reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("failed to build HTTP client"),
            next_id: AtomicU64::new(1),
        }
    }

    fn url(&self) -> String {
        format!("http://127.0.0.1:{}/jsonrpc", self.port)
    }

    fn token(&self) -> String {
        format!("token:{}", self.secret)
    }

    fn with_token(&self) -> Vec<Value> {
        vec![Value::String(self.token())]
    }

    fn with_keys(&self) -> Vec<Value> {
        let keys: Vec<Value> = RPC_KEYS.iter().map(|k| Value::String(k.to_string())).collect();
        let mut params = self.with_token();
        params.push(Value::Array(keys));
        params
    }

    /// Perform a JSON-RPC call and return the `result` field.
    pub fn request(&self, method: &str, params: Vec<Value>) -> Result<Value, String> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let payload = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        let res = self
            .http
            .post(self.url())
            .json(&payload)
            .send()
            .map_err(|e| format!("RPC send failed: {e}"))?;

        let body: Value = res.json().map_err(|e| format!("RPC parse failed: {e}"))?;

        if let Some(result) = body.get("result") {
            return Ok(result.clone());
        }
        if let Some(err) = body.get("error") {
            let code = err.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
            let msg = err
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown RPC error")
                .to_string();
            return Err(format!("aria2 error {code}: {msg}"));
        }
        Err("Malformed RPC response".into())
    }

    /* ---- download controls ---- */

    pub fn add_uri(&self, uris: &[String], options: Option<Value>) -> Result<String, String> {
        let mut params = self.with_token();
        params.push(Value::Array(
            uris.iter().map(|u| Value::String(u.clone())).collect(),
        ));
        if let Some(opts) = options {
            params.push(opts);
        }
        let result = self.request("aria2.addUri", params)?;
        result
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "aria2 did not return a GID".into())
    }

    pub fn pause(&self, gid: &str) -> Result<(), String> {
        self.void_gid("aria2.pause", gid)
    }

    pub fn unpause(&self, gid: &str) -> Result<(), String> {
        self.void_gid("aria2.unpause", gid)
    }

    pub fn remove(&self, gid: &str) -> Result<(), String> {
        self.void_gid("aria2.remove", gid)
    }

    pub fn remove_download_result(&self, gid: &str) -> Result<(), String> {
        self.void_gid("aria2.removeDownloadResult", gid)
    }

    pub fn pause_all(&self) -> Result<(), String> {
        self.request("aria2.pauseAll", self.with_token()).map(|_| ())
    }

    pub fn unpause_all(&self) -> Result<(), String> {
        self.request("aria2.unpauseAll", self.with_token()).map(|_| ())
    }

    /* ---- status queries ---- */

    pub fn get_version(&self) -> Result<String, String> {
        let result = self.request("aria2.getVersion", self.with_token())?;
        Ok(result
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("?")
            .to_string())
    }

    pub fn tell_active(&self) -> Result<Vec<Value>, String> {
        self.request("aria2.tellActive", self.with_keys())?
            .as_array()
            .cloned()
            .ok_or_else(|| "bad tellActive response".into())
    }

    pub fn tell_waiting(&self, offset: u64, num: u64) -> Result<Vec<Value>, String> {
        let mut params = self.with_token();
        params.push(Value::from(offset));
        params.push(Value::from(num));
        params.push(Value::Array(
            RPC_KEYS.iter().map(|k| Value::String(k.to_string())).collect(),
        ));
        Ok(self
            .request("aria2.tellWaiting", params)?
            .as_array()
            .cloned()
            .unwrap_or_default())
    }

    pub fn tell_stopped(&self, offset: u64, num: u64) -> Result<Vec<Value>, String> {
        let mut params = self.with_token();
        params.push(Value::from(offset));
        params.push(Value::from(num));
        params.push(Value::Array(
            RPC_KEYS.iter().map(|k| Value::String(k.to_string())).collect(),
        ));
        Ok(self
            .request("aria2.tellStopped", params)?
            .as_array()
            .cloned()
            .unwrap_or_default())
    }

    pub fn get_global_stat(&self) -> Result<Value, String> {
        self.request("aria2.getGlobalStat", self.with_token())
    }

    /* ---- session lifecycle ---- */

    pub fn save_session(&self) -> Result<(), String> {
        self.request("aria2.saveSession", self.with_token()).map(|_| ())
    }

    pub fn shutdown(&self) -> Result<(), String> {
        self.request("aria2.shutdown", self.with_token()).map(|_| ())
    }

    /* ---- internals ---- */

    fn void_gid(&self, method: &str, gid: &str) -> Result<(), String> {
        let mut params = self.with_token();
        params.push(Value::String(gid.to_string()));
        self.request(method, params).map(|_| ())
    }
}

/* ================= Process management ================= */

pub struct Aria2Process {
    child: Option<Child>,
    rpc_path: PathBuf,
}

impl Aria2Process {
    /// Start (or reuse) the aria2 daemon and return an RPC client bound to it.
    pub fn start(app: &AppHandle) -> Result<(Arc<RpcClient>, Self), String> {
        let app_data = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("failed to resolve app data dir: {e}"))?;

        let rpc_path = app_data.join("rpc.json");

        // Reuse an already-running daemon from a previous launch.
        if let Ok(cfg) = fs::read_to_string(&rpc_path) {
            if let Ok(json) = serde_json::from_str::<Value>(&cfg) {
                if let (Some(port), Some(secret)) = (
                    json.get("port").and_then(|v| v.as_u64()),
                    json.get("secret").and_then(|v| v.as_str()),
                ) {
                    let client = Arc::new(RpcClient::new(port as u16, secret));
                    if client.get_version().is_ok() {
                        return Ok((
                            client,
                            Aria2Process {
                                child: None,
                                rpc_path,
                            },
                        ));
                    }
                }
            }
        }

        // Fresh start.
        let binary = Self::ensure_binary(app)?;
        let port = Self::find_free_port();
        let secret: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();

        let downloads_dir = default_download_dir(&app_data);
        fs::create_dir_all(&downloads_dir).map_err(|e| e.to_string())?;

        let session_path = app_data.join("session.txt");
        if !session_path.exists() {
            fs::File::create(&session_path).map_err(|e| e.to_string())?;
        }

        let mut cmd = Command::new(&binary);
        cmd.args([
            "--enable-rpc",
            "--rpc-listen-all=false",
            &format!("--rpc-listen-port={port}"),
            &format!("--rpc-secret={secret}"),
            &format!("--dir={}", downloads_dir.display()),
            &format!("--input-file={}", session_path.display()),
            &format!("--save-session={}", session_path.display()),
            "--save-session-interval=10",
            "--continue=true",
            "--auto-file-renaming=false",
            "--allow-overwrite=false",
            "--max-concurrent-downloads=3",
            "--split=16",
            "--max-connection-per-server=16",
            "--summary-interval=1",
            "--console-log-level=warn",
            &format!("--stop-with-process={}", std::process::id()),
        ]);
        cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());

        let child = cmd
            .spawn()
            .map_err(|e| format!("failed to start aria2: {e}"))?;

        let client = Arc::new(RpcClient::new(port, &secret));

        // Wait for the RPC endpoint to come up (up to ~5s).
        let mut up = false;
        for _ in 0..25 {
            std::thread::sleep(Duration::from_millis(200));
            if client.get_version().is_ok() {
                up = true;
                break;
            }
        }
        if !up {
            let mut child = child;
            let _ = child.kill();
            return Err("aria2 RPC endpoint did not come up".into());
        }

        let cfg = json!({ "port": port, "secret": secret });
        fs::write(&rpc_path, serde_json::to_string_pretty(&cfg).unwrap_or_default())
            .map_err(|e| e.to_string())?;

        Ok((
            client,
            Aria2Process {
                child: Some(child),
                rpc_path,
            },
        ))
    }

    /// Gracefully save the session and shut the daemon down.
    pub fn stop(self, client: &RpcClient) {
        let _ = client.save_session();
        let _ = client.shutdown();
        std::thread::sleep(Duration::from_millis(400));
        if let Some(mut child) = self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = fs::remove_file(&self.rpc_path);
    }

    fn find_free_port() -> u16 {
        TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    }

    fn ensure_binary(app: &AppHandle) -> Result<PathBuf, String> {
        let app_data = app
            .path()
            .app_data_dir()
            .map_err(|e| e.to_string())?;
        let bin_dir = app_data.join("bin");
        let target = bin_dir.join("aria2c.exe");

        if target.exists() {
            return Ok(target);
        }

        // Candidate source locations, in order.
        let mut candidates: Vec<PathBuf> = Vec::new();
        candidates.push(PathBuf::from("bin/windows/aria2c.exe")); // dev cwd = src-tauri
        if let Ok(res) = app.path().resource_dir() {
            candidates.push(res.join("bin").join("windows").join("aria2c.exe"));
            candidates.push(res.join("aria2c.exe"));
        }
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                // Tauri external binaries get a target-triple suffix in release.
                candidates.push(dir.join("aria2c-x86_64-pc-windows-msvc.exe"));
                candidates.push(dir.join("aria2c.exe"));
            }
        }

        for candidate in candidates {
            if candidate.exists() {
                fs::create_dir_all(&bin_dir).map_err(|e| e.to_string())?;
                fs::copy(&candidate, &target).map_err(|e| e.to_string())?;
                return Ok(target);
            }
        }

        Err("aria2c binary not found — bundled binary is missing".into())
    }
}

/// Prefer the user's Downloads folder; fall back to the app data dir.
fn default_download_dir(app_data: &PathBuf) -> PathBuf {
    if let Ok(profile) = std::env::var("USERPROFILE") {
        let downloads = PathBuf::from(profile).join("Downloads");
        if downloads.exists() {
            return downloads;
        }
    }
    app_data.join("downloads")
}

/* ================= Tests ================= */

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};

    struct TestServer {
        port: u16,
    }

    impl TestServer {
        fn start() -> TestServer {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let port = listener.local_addr().unwrap().port();
            std::thread::spawn(move || {
                for stream in listener.incoming() {
                    if let Ok(mut s) = stream {
                        std::thread::spawn(move || handle(&mut s));
                    }
                }
            });
            TestServer { port }
        }
    }

    fn handle(s: &mut TcpStream) {
        let mut buf = [0u8; 8192];
        let n = match s.read(&mut buf) {
            Ok(n) if n > 0 => n,
            _ => return,
        };
        let request = String::from_utf8_lossy(&buf[..n]);
        let path = request.split_whitespace().nth(1).unwrap_or("/");

        if path.starts_with("/slow/") {
            let total: usize = path.trim_start_matches("/slow/").parse().unwrap_or(1024);
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {total}\r\nConnection: close\r\n\r\n"
            );
            if s.write_all(header.as_bytes()).is_err() {
                return;
            }
            let chunk = vec![0u8; 64 * 1024];
            let mut sent = 0usize;
            while sent < total {
                let take = chunk.len().min(total - sent);
                if s.write_all(&chunk[..take]).is_err() {
                    return;
                }
                sent += take;
                std::thread::sleep(Duration::from_millis(50));
            }
        } else if path == "/big.bin" {
            let data = vec![0xABu8; 1024 * 1024];
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                data.len()
            );
            if s.write_all(header.as_bytes()).is_err() {
                return;
            }
            let _ = s.write_all(&data);
        } else {
            let _ = s.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        }
    }

    fn spawn_aria2() -> (RpcClient, Child) {
        let port = {
            TcpListener::bind("127.0.0.1:0")
                .unwrap()
                .local_addr()
                .unwrap()
                .port()
        };
        let secret = "test-secret";
        let dir = std::env::temp_dir().join(format!("aria2-rpc-test-{port}"));
        std::fs::create_dir_all(&dir).unwrap();
        let session = dir.join("session.txt");
        std::fs::write(&session, "").unwrap();

        let child = Command::new("bin/windows/aria2c.exe")
            .args([
                "--enable-rpc",
                "--rpc-listen-all=false",
                &format!("--rpc-listen-port={port}"),
                &format!("--rpc-secret={secret}"),
                &format!("--dir={}", dir.display()),
                &format!("--input-file={}", session.display()),
                &format!("--save-session={}", session.display()),
                "--continue=true",
                "--console-log-level=warn",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();

        (RpcClient::new(port, secret), child)
    }

    #[test]
    fn rpc_lifecycle() {
        let server = TestServer::start();
        let (client, mut child) = spawn_aria2();

        let mut up = false;
        for _ in 0..25 {
            std::thread::sleep(Duration::from_millis(200));
            if client.get_version().is_ok() {
                up = true;
                break;
            }
        }
        assert!(up, "aria2 RPC endpoint did not come up");

        let one = json!({ "split": "1", "max-connection-per-server": "1" });

        // Slow download: pause then resume.
        let url = format!("http://127.0.0.1:{}/slow/2097152", server.port);
        let gid = client.add_uri(&[url], Some(one.clone())).unwrap();
        assert!(!gid.is_empty());

        std::thread::sleep(Duration::from_millis(400));
        client.pause(&gid).unwrap();
        let waiting = client.tell_waiting(0, 100).unwrap();
        let status = waiting
            .iter()
            .find(|w| w.get("gid").and_then(|g| g.as_str()) == Some(gid.as_str()))
            .and_then(|w| w.get("status").and_then(|s| s.as_str()))
            .unwrap_or("");
        assert_eq!(status, "paused", "expected paused status");

        client.unpause(&gid).unwrap();
        client.remove(&gid).unwrap();
        client.remove_download_result(&gid).unwrap();

        // Fast download: wait for completion.
        let url2 = format!("http://127.0.0.1:{}/big.bin", server.port);
        let gid2 = client.add_uri(&[url2], Some(one)).unwrap();

        let mut complete = false;
        for _ in 0..100 {
            std::thread::sleep(Duration::from_millis(200));
            let stopped = client.tell_stopped(0, 20).unwrap();
            if stopped.iter().any(|s| {
                s.get("gid").and_then(|g| g.as_str()) == Some(gid2.as_str())
                    && s.get("status").and_then(|x| x.as_str()) == Some("complete")
            }) {
                complete = true;
                break;
            }
        }
        assert!(complete, "download did not complete in time");

        let gs = client.get_global_stat().unwrap();
        let stopped_count: u64 = gs
            .get("numStopped")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        assert!(stopped_count > 0, "expected stopped downloads to be reported");

        let _ = client.shutdown();
        std::thread::sleep(Duration::from_millis(300));
        let _ = child.kill();
        let _ = child.wait();
    }
}
