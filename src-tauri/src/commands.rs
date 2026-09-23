// src-tauri/src/commands.rs

use localops_core::{
    business::{self},
    session,
    setup::{self, SetupResult},
    user,
};
use serde::{Deserialize, Serialize};
use tauri::State;
/// Create a new business and return its generated UUID.
#[tauri::command]
pub fn create_business(state: State<'_, super::DbState>, name: String) -> Result<String, String> {
    let conn = state.connection.lock().map_err(|e| e.to_string())?;
    business::create_business(&conn, &name).map_err(|e| e.to_string())
}

/// List all active businesses as a vector of (id, name) tuples.
#[tauri::command]
pub fn list_businesses(state: State<'_, super::DbState>) -> Result<Vec<(String, String)>, String> {
    let conn = state.connection.lock().map_err(|e| e.to_string())?;
    let businesses = business::list_businesses(&conn).map_err(|e| e.to_string())?;
    Ok(businesses.into_iter().map(|b| (b.id, b.name)).collect())
}

/// Update the trading name for a given business.
#[tauri::command]
pub fn update_trading_name(
    state: State<'_, super::DbState>,
    id: String,
    trading_name: Option<String>,
) -> Result<(), String> {
    let conn = state.connection.lock().map_err(|e| e.to_string())?;
    business::update_business_trading_name(&conn, &id, trading_name.as_deref())
        .map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitialSetupInput {
    business_name: String,
    department_name: String,
    location_name: String,
    terminal_name: String,
    owner_username: String,
    owner_display_name: String,
    owner_pin: String,
}

#[tauri::command]
pub fn complete_initial_setup(
    state: State<'_, super::DbState>,
    input: InitialSetupInput,
) -> Result<SetupResult, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let result = setup::complete_initial_setup(
        &connection,
        setup::InitialSetup {
            business_name: &input.business_name,
            department_name: &input.department_name,
            location_name: &input.location_name,
            terminal_name: &input.terminal_name,
            owner_username: &input.owner_username,
            owner_display_name: &input.owner_display_name,
            owner_pin: &input.owner_pin,
        },
    )
    .map_err(|error| error.to_string())?;
    *state
        .current_session
        .lock()
        .map_err(|error| error.to_string())? = Some(result.session_id.clone());
    Ok(result)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResult {
    session_id: String,
    user_id: String,
    display_name: String,
}

#[tauri::command]
pub fn login(
    state: State<'_, super::DbState>,
    business_id: String,
    terminal_id: String,
    username: String,
    pin: String,
) -> Result<LoginResult, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let authenticated = user::authenticate_user(&connection, &business_id, &username, &pin)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| {
            "Invalid username or PIN, or the account is temporarily locked".to_owned()
        })?;
    let active = session::start_session(&connection, &business_id, &authenticated.id, &terminal_id)
        .map_err(|error| error.to_string())?;
    let mut current = state
        .current_session
        .lock()
        .map_err(|error| error.to_string())?;
    if let Some(previous) = current.replace(active.id.clone()) {
        let _ = session::end_session(&connection, &previous, "REPLACED");
    }
    Ok(LoginResult {
        session_id: active.id,
        user_id: authenticated.id,
        display_name: authenticated.display_name,
    })
}

#[tauri::command]
pub fn logout(state: State<'_, super::DbState>) -> Result<(), String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let session_id = state
        .current_session
        .lock()
        .map_err(|error| error.to_string())?
        .take()
        .ok_or_else(|| "No active session".to_owned())?;
    session::end_session(&connection, &session_id, "LOGOUT").map_err(|error| error.to_string())
}
