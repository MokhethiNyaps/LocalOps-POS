#[cfg_attr(mobile, tauri::mobile_entry_point)]
use localops_core::bootstrap;
use rusqlite::Connection;
pub fn run() {
    // Determine application data paths (Windows default location)
    let paths = match localops_core::bootstrap::AppPaths::windows_default() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to locate app data directory: {}", e);
            std::process::exit(1);
        }
    };

    // Bootstrap the SQLite database (creates file, runs migrations, verifies integrity)
    let (connection, _health) = match localops_core::bootstrap::bootstrap(&paths) {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Database bootstrap error: {}", e);
            std::process::exit(1);
        }
    };

    // Wrap the connection in a lightweight state object for Tauri
    let db_state = DbState { connection: Mutex::new(connection) };

    tauri::Builder::default()
        .manage(db_state)
        .setup(|app| {
            // Enable verbose logging during development builds
.setup(|_app| Ok(()))
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            crate::commands::create_business,
            crate::commands::list_businesses,
            crate::commands::update_trading_name,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// Simple wrapper exposing the SQLite connection to command handlers.
use std::sync::Mutex;

pub struct DbState {
    pub connection: Mutex<Connection>,
}

mod commands;
