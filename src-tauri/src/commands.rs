// src-tauri/src/commands.rs

use tauri::State;
use localops_core::business::{self};
/// Create a new business and return its generated UUID.
#[tauri::command]
pub fn create_business(state: State<'_, super::DbState>, name: String) -> Result<String, String> {
    let conn = state.connection.lock().map_err(|e| e.to_string())?;
    business::create_business(&*conn, &name).map_err(|e| e.to_string())
}

/// List all active businesses as a vector of (id, name) tuples.
#[tauri::command]
pub fn list_businesses(state: State<'_, super::DbState>) -> Result<Vec<(String, String)>, String> {
    let conn = state.connection.lock().map_err(|e| e.to_string())?;
    let businesses = business::list_businesses(&*conn).map_err(|e| e.to_string())?;
    Ok(businesses
        .into_iter()
        .map(|b| (b.id, b.name))
        .collect())
}

/// Update the trading name for a given business.
#[tauri::command]
pub fn update_trading_name(
    state: State<'_, super::DbState>,
    id: String,
    trading_name: Option<String>,
) -> Result<(), String> {
    let conn = state.connection.lock().map_err(|e| e.to_string())?;
    business::update_business_trading_name(
        &*conn,
        &id,
        trading_name.as_deref(),
    )
    .map_err(|e| e.to_string())
}
