//! LIAN desktop shell. Deliberately thin: all domain logic lives in
//! `lian-core`. The shell owns the SQLite connection, the tray, the window
//! lifecycle, and the reminder delivery loop.

use std::path::PathBuf;
use std::sync::Mutex;

use serde_json::Value;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Manager, WindowEvent};
use tauri_plugin_notification::NotificationExt;

/// The live connection is optional so restore/purge can close it, swap the
/// database file, and reopen.
pub struct Db {
    pub conn: Mutex<Option<rusqlite::Connection>>,
    pub path: PathBuf,
}

use lian_core::rusqlite;

fn with_conn<T>(
    db: &Db,
    f: impl FnOnce(&rusqlite::Connection) -> lian_core::Result<T>,
) -> Result<T, String> {
    let guard = db.conn.lock().map_err(|_| "database lock poisoned".to_string())?;
    let conn = guard.as_ref().ok_or_else(|| "database is not open".to_string())?;
    f(conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn api(db: tauri::State<'_, Db>, method: String, payload: Value) -> Result<Value, String> {
    with_conn(&db, |conn| lian_core::api::dispatch(conn, &method, payload))
}

#[tauri::command]
fn data_location(db: tauri::State<'_, Db>) -> String {
    db.path.to_string_lossy().to_string()
}

/// Restore from a backup: verify + safety-copy via core, close the live
/// connection, replace the database file, reopen and re-migrate.
#[tauri::command]
fn restore_backup(db: tauri::State<'_, Db>, backup_path: String) -> Result<Value, String> {
    let live = db.path.to_string_lossy().to_string();
    let prep = with_conn(&db, |conn| {
        lian_core::backup::prepare_restore(conn, &live, &backup_path)
    })?;
    let mut guard = db.conn.lock().map_err(|_| "database lock poisoned".to_string())?;
    *guard = None; // close current connection
    for suffix in ["-wal", "-shm"] {
        let p = PathBuf::from(format!("{live}{suffix}"));
        if p.exists() {
            let _ = std::fs::remove_file(&p);
        }
    }
    std::fs::copy(&backup_path, &db.path).map_err(|e| format!("restore copy failed: {e}"))?;
    let conn = lian_core::db::open(&db.path).map_err(|e| e.to_string())?;
    *guard = Some(conn);
    Ok(prep)
}

/// Permanently delete all local data, then start from an empty database.
#[tauri::command]
fn purge_all_data(db: tauri::State<'_, Db>) -> Result<Value, String> {
    let live = db.path.to_string_lossy().to_string();
    let mut guard = db.conn.lock().map_err(|_| "database lock poisoned".to_string())?;
    *guard = None;
    let result = lian_core::backup::purge_all_data(&live).map_err(|e| e.to_string())?;
    let conn = lian_core::db::open(&db.path).map_err(|e| e.to_string())?;
    *guard = Some(conn);
    Ok(result)
}

fn deliver_due_notifications(app: &tauri::AppHandle) {
    let db = app.state::<Db>();
    let due = match with_conn(&db, lian_core::reminders::due_notifications) {
        Ok(d) => d,
        Err(_) => return,
    };
    for n in due {
        let title = n["title"].as_str().unwrap_or("LIAN").to_string();
        let body = n["body"].as_str().unwrap_or("").to_string();
        let shown = app
            .notification()
            .builder()
            .title(&title)
            .body(&body)
            .show()
            .is_ok();
        if shown {
            let _ = with_conn(&db, |conn| lian_core::reminders::record_fired(conn, &n));
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Durable application-data directory, outside the install dir.
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let db_path = data_dir.join("lian.sqlite3");
            let conn = lian_core::db::open(&db_path)?;
            app.manage(Db { conn: Mutex::new(Some(conn)), path: db_path });

            // Tray: LIAN stays available after the window closes.
            let show = MenuItem::with_id(app, "show", "Open LIAN", true, None::<&str>)?;
            let pause = MenuItem::with_id(app, "pause", "Pause reminders 2h", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit LIAN", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &pause, &quit])?;
            TrayIconBuilder::with_id("lian-tray")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "pause" => {
                        let db = app.state::<Db>();
                        let until = (chrono_now_plus_minutes(120)).to_string();
                        let _ = with_conn(&db, |conn| {
                            lian_core::reminders::set_pause(conn, true, Some(until.clone()))
                        });
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            // Reminder loop: check every 30 seconds. All safeguards (quiet
            // hours, pause, snooze, dedupe, burst suppression) live in core.
            let handle = app.handle().clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_secs(30));
                deliver_due_notifications(&handle);
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // Close-to-tray keeps reminders alive; setting is user-controlled.
                let db = window.app_handle().state::<Db>();
                let close_to_tray = with_conn(&db, |conn| {
                    lian_core::settings::get(conn, "close_to_tray")
                })
                .map(|v| v.as_bool().unwrap_or(true))
                .unwrap_or(true);
                if close_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            api,
            data_location,
            restore_backup,
            purge_all_data
        ])
        .run(tauri::generate_context!())
        .expect("error while running LIAN");
}

fn chrono_now_plus_minutes(minutes: i64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
        + minutes * 60;
    // RFC3339 UTC from epoch seconds without pulling chrono into the shell.
    lian_core::util::epoch_to_rfc3339(now)
}
