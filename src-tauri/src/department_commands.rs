use localops_core::{audit, department};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::{session_guard::require_owner, DbState};

#[derive(Serialize)]
pub struct StockLocationDto {
    id: String,
    name: String,
}

#[tauri::command]
pub fn get_department_stock_locations(
    state: State<'_, DbState>,
) -> Result<Vec<StockLocationDto>, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_owner(&connection, &state, "departments.manage")?;
    let mut statement = connection
        .prepare(
            "SELECT id, name FROM locations WHERE business_id = ?1 AND active = 1 ORDER BY name",
        )
        .map_err(|error| error.to_string())?;
    let locations = statement
        .query_map([&context.business_id], |row| {
            Ok(StockLocationDto {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        })
        .map_err(|error| error.to_string())?;
    locations
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewDepartmentTerminal {
    name: String,
    location_id: String,
    terminal_name: String,
}

#[tauri::command]
pub fn create_department_terminal(
    state: State<'_, DbState>,
    input: NewDepartmentTerminal,
) -> Result<(), String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_owner(&connection, &state, "departments.manage")?;
    context
        .require_permission("terminals.manage")
        .map_err(|error| error.to_string())?;
    let (department_id, terminal_id) = department::create_department_terminal(
        &connection,
        &context.business_id,
        &input.name,
        &input.location_id,
        &input.terminal_name,
    )
    .map_err(|error| error.to_string())?;
    let details = serde_json::json!({ "name": input.name, "locationId": input.location_id, "terminalId": terminal_id, "terminalName": input.terminal_name }).to_string();
    audit::record_event(
        &connection,
        audit::NewAuditEvent {
            business_id: &context.business_id,
            user_id: Some(&context.user_id),
            terminal_id: Some(&context.terminal_id),
            action: "DEPARTMENT_TERMINAL_CREATED",
            entity_type: "department",
            entity_id: &department_id,
            old_json: None,
            new_json: Some(&details),
        },
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}
