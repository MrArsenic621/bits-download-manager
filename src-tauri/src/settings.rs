use crate::model::Settings;
use std::path::PathBuf;

pub fn load(path: &PathBuf) -> Settings {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(path: &PathBuf, settings: &Settings) {
    let _ = std::fs::write(
        path,
        serde_json::to_string_pretty(settings).unwrap_or_default(),
    );
}

pub fn sanitize(mut s: Settings) -> Settings {
    s.default_split = s.default_split.clamp(1, 64);
    s.max_concurrent_downloads = s.max_concurrent_downloads.clamp(1, 16);
    s.default_dir = s.default_dir.trim().to_string();
    s
}
