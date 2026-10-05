use crate::model::Vault;
use std::path::PathBuf;

pub fn load(path: &PathBuf) -> Vault {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(path: &PathBuf, vault: &Vault) {
    let _ = std::fs::write(
        path,
        serde_json::to_string_pretty(vault).unwrap_or_default(),
    );
}
