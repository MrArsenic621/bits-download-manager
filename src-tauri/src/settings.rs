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

#[cfg(test)]
mod tests {
    use super::load;

    #[test]
    fn older_settings_files_keep_their_values_and_default_new_fields() {
        let dir = std::env::temp_dir().join("bits_dm_settings_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");
        // Pre-youtube_cookie shape: missing field must not reset the whole config.
        std::fs::write(
            &path,
            r#"{"default_dir":"D:\\Downloads","default_split":8,"max_concurrent_downloads":2,
                "global_speed_limit":0,"notify_on_complete":false,"watch_clipboard":true,
                "auto_categorize":true,"schedule_enabled":false,"schedule_start_time":"01:00",
                "schedule_stop_time":"07:00","shutdown_on_finish":false,"close_to_tray":true,
                "global_proxy":"socks5://127.0.0.1:1080"}"#,
        )
        .unwrap();

        let s = load(&path);
        assert_eq!(s.default_dir, "D:\\Downloads");
        assert_eq!(s.default_split, 8);
        assert_eq!(s.global_proxy, "socks5://127.0.0.1:1080");
        assert_eq!(s.youtube_cookie, "");
        std::fs::remove_dir_all(&dir).ok();
    }
}
