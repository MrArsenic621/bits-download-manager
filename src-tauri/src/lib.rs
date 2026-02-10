mod aria2;

use aria2::Aria2;
use std::sync::Mutex;
use tauri::Manager;

/* ================= App State ================= */

struct AppState {
    aria2: Mutex<Option<Aria2>>,
}

/* ================= Commands ================= */

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn aria2_info(state: tauri::State<AppState>) -> Result<(u16, String), String> {
    let guard = state.aria2.lock().map_err(|_| "State poisoned")?;
    let aria2 = guard.as_ref().ok_or("aria2 not running")?;

    Ok((aria2.port(), aria2.secret().to_string()))
}

/* ================= App Entry ================= */

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let aria2 = Aria2::start(app.handle());

            app.manage(AppState {
                aria2: Mutex::new(Some(aria2)),
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            aria2_info
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
