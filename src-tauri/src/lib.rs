#[cfg_attr(mobile, tauri::mobile_entry_point)]
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
    let db_state = DbState {
        connection: Mutex::new(connection),
        current_session: Mutex::new(None),
    };

    tauri::Builder::default()
        .manage(db_state)
        .setup(|_app| Ok(()))
        .invoke_handler(tauri::generate_handler![
            crate::commands::create_business,
            crate::commands::list_businesses,
            crate::commands::update_trading_name,
            crate::commands::complete_initial_setup,
            crate::commands::login,
            crate::commands::logout,
            crate::commands::get_app_bootstrap,
            crate::catalogue_commands::get_catalogue_snapshot,
            crate::catalogue_commands::create_catalogue_category,
            crate::catalogue_commands::create_catalogue_unit,
            crate::catalogue_commands::create_catalogue_product,
            crate::catalogue_commands::create_catalogue_service,
            crate::catalogue_commands::create_catalogue_packaging,
            crate::catalogue_commands::replace_catalogue_recipe,
            crate::catalogue_commands::set_catalogue_availability,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// Simple wrapper exposing the SQLite connection to command handlers.
use std::sync::Mutex;

pub struct DbState {
    pub connection: Mutex<Connection>,
    pub current_session: Mutex<Option<String>>,
}

mod catalogue_commands;
mod commands;
mod session_guard;
