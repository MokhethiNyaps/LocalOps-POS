// src-tauri/src/commands.rs

use localops_core::{
    business::{self},
    role, session,
    setup::{self, SetupResult},
    terminal, user,
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
    business_id: String,
    user_id: String,
    terminal_id: String,
    department_id: Option<String>,
    display_name: String,
    permissions: Vec<String>,
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
    let permissions = role::list_user_permissions(&connection, &authenticated.id)
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|permission| permission.code)
        .collect();
    Ok(LoginResult {
        session_id: active.id,
        business_id: active.business_id,
        user_id: authenticated.id,
        terminal_id: active.terminal_id,
        department_id: connection
            .query_row(
                "SELECT department_id FROM terminals WHERE id = ?1",
                [&terminal_id],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?,
        display_name: authenticated.display_name,
        permissions,
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapTerminal {
    id: String,
    name: String,
    department_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapBusiness {
    id: String,
    name: String,
    terminals: Vec<BootstrapTerminal>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppBootstrap {
    setup_required: bool,
    businesses: Vec<BootstrapBusiness>,
}

#[tauri::command]
pub fn get_app_bootstrap(state: State<'_, super::DbState>) -> Result<AppBootstrap, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let businesses = business::list_businesses(&connection)
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|entry| {
            let terminals = terminal::list_terminals(&connection, &entry.id)
                .map_err(|error| error.to_string())?
                .into_iter()
                .map(|value| BootstrapTerminal {
                    id: value.id,
                    name: value.name,
                    department_id: value.department_id,
                })
                .collect();
            Ok(BootstrapBusiness {
                id: entry.id,
                name: entry.name,
                terminals,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(AppBootstrap {
        setup_required: businesses.is_empty(),
        businesses,
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
