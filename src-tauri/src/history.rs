use crate::model::Download;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

const MAX_HISTORY: usize = 500;

#[derive(Serialize, Deserialize)]
struct HistoryFile {
    downloads: Vec<Download>,
}

/// Load persisted history from disk, keyed by GID.
pub fn load(path: &PathBuf) -> HashMap<String, Download> {
    match fs_read_json(path) {
        Some(f) => f
            .downloads
            .into_iter()
            .map(|d| (d.gid.clone(), d))
            .collect(),
        None => HashMap::new(),
    }
}

/// Write history to disk, only rewriting when the content actually changed.
pub fn persist_if_changed(
    path: &PathBuf,
    history: &HashMap<String, Download>,
    last: &mut String,
) {
    let mut v: Vec<Download> = history.values().cloned().collect();
    v.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    v.truncate(MAX_HISTORY);
    let f = HistoryFile { downloads: v };

    let serialized = serde_json::to_string(&f).unwrap_or_default();
    if *last == serialized {
        return;
    }
    let _ = std::fs::write(path, &serialized);
    *last = serialized;
}

fn fs_read_json(path: &PathBuf) -> Option<HistoryFile> {
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}
