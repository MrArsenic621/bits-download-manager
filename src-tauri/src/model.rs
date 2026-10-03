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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Empty string means "system Downloads folder".
    pub default_dir: String,
    pub default_split: u32,
    pub max_concurrent_downloads: u32,
    /// Global overall download speed limit in bytes/sec. 0 = unlimited.
    pub global_speed_limit: u64,
    pub notify_on_complete: bool,
    pub watch_clipboard: bool,
    pub auto_categorize: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            default_dir: String::new(),
            default_split: 16,
            max_concurrent_downloads: 3,
            global_speed_limit: 0,
            notify_on_complete: true,
            watch_clipboard: true,
            auto_categorize: true,
        }
    }
}

pub fn detect_category(filename_or_uri: &str) -> Option<&'static str> {
    let clean = filename_or_uri.split('?').next().unwrap_or(filename_or_uri);
    let clean = clean.split('#').next().unwrap_or(clean);
    let ext = std::path::Path::new(clean)
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())?;

    match ext.as_str() {
        "mp4" | "mkv" | "avi" | "mov" | "wmv" | "flv" | "webm" | "m4v" | "ts" | "3gp" => {
            Some("Videos")
        }
        "mp3" | "flac" | "wav" | "aac" | "ogg" | "m4a" | "wma" | "opus" => Some("Audio"),
        "pdf" | "docx" | "doc" | "xlsx" | "xls" | "pptx" | "ppt" | "txt" | "epub" | "csv"
        | "md" | "rtf" => Some("Documents"),
        "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" | "iso" | "dmg" => Some("Archives"),
        "exe" | "msi" | "apk" | "deb" | "rpm" | "appimage" => Some("Programs"),
        _ => None,
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
