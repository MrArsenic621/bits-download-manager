use rand::{distributions::Alphanumeric, Rng};
use std::{
    fs,
    net::TcpListener,
    path::PathBuf,
    process::{Child, Command},
};
use tauri::{AppHandle, Manager};

pub struct Aria2 {
    port: u16,
    secret: String,
    process: Child,
}

impl Aria2 {
    /* ========= public API ========= */

    pub fn start(app: &AppHandle) -> Self {
        let port = Self::find_free_port();
        let secret = Self::generate_secret();
        let aria2_path = Self::ensure_binary(app);

        let process = Command::new(aria2_path)
            .args([
                "--enable-rpc",
                "--rpc-listen-all=false",
                &format!("--rpc-listen-port={}", port),
                &format!("--rpc-secret={}", secret),
                "--continue=true",
                "--summary-interval=1",
            ])
            .spawn()
            .expect("failed to start aria2");

        Self {
            port,
            secret,
            process,
        }
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn secret(&self) -> &str {
        &self.secret
    }

    pub fn is_running(&mut self) -> bool {
        self.process.try_wait().unwrap().is_none()
    }

    pub fn stop(mut self) {
        let _ = self.process.kill();
    }

    /* ========= internals ========= */

    fn find_free_port() -> u16 {
        TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    }

    fn generate_secret() -> String {
        rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect()
    }

    fn ensure_binary(app: &AppHandle) -> PathBuf {
        let app_dir = app
            .path()
            .app_data_dir()
            .expect("failed to get app data dir");

        let bin_dir = app_dir.join("bin");
        let target = bin_dir.join("aria2c.exe");

        if !target.exists() {
            fs::create_dir_all(&bin_dir).unwrap();

            // This path is relative to src-tauri/
            let bundled = PathBuf::from("bin/windows/aria2c.exe");
            fs::copy(&bundled, &target)
                .expect("failed to copy aria2c binary");
        }
        println!("Using aria2 binary at: {:?}", target);
        target
    }
}
