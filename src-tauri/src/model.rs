use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Download {
    pub gid: String,
    pub uri: String,
    pub filename: String,
    pub dir: String,
    /// one of: active, waiting, paused, complete, error, removed
    pub status: String,
    pub total_length: u64,
    pub completed_length: u64,
    pub download_speed: u64,
    pub upload_speed: u64,
    pub progress: f64,
    pub eta_secs: Option<u64>,
    pub error_message: Option<String>,
    pub created_at: u64,
}

impl Download {
    pub fn file_path(&self) -> String {
        if self.dir.is_empty() {
            return self.filename.clone();
        }
        std::path::Path::new(&self.dir)
            .join(&self.filename)
            .to_string_lossy()
            .into_owned()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalStat {
    pub download_speed: u64,
    pub upload_speed: u64,
    pub num_active: u64,
    pub num_waiting: u64,
    pub num_stopped: u64,
}

impl Default for GlobalStat {
    fn default() -> Self {
        Self {
            download_speed: 0,
            upload_speed: 0,
            num_active: 0,
            num_waiting: 0,
            num_stopped: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub downloads: Vec<Download>,
    pub global: GlobalStat,
    pub aria2_version: Option<String>,
    /// Set when the aria2 engine failed to start; cleared once it reports.
    pub startup_error: Option<String>,
}

impl Default for Snapshot {
    fn default() -> Self {
        Self {
            downloads: Vec::new(),
            global: GlobalStat::default(),
            aria2_version: None,
            startup_error: None,
        }
    }
}
