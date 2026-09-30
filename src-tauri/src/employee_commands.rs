//! Owner-facing employee administration commands.
//!
//! Classification: every command here is `OWNER_OR_PERMISSION`. `users.manage`
//! is always required and role changes additionally require `roles.manage`.
//! `user_roles` is the authoritative assignment relationship and any authority
//! change invalidates the affected employee's sessions.

use localops_core::employee::{self, EmployeeRoleView, EmployeeView};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::{session_guard::require_active_context, DbState};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeDirectory {
    employees: Vec<EmployeeView>,
    roles: Vec<EmployeeRoleView>,
}

#[tauri::command]
pub fn list_employees(state: State<'_, DbState>) -> Result<EmployeeDirectory, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_active_context(&connection, &state, Some("users.manage"))?;
    let employees =
        employee::list_employees(&connection, &context).map_err(|error| error.to_string())?;
    let roles =
        employee::list_assignable_roles(&connection, &context).map_err(|error| error.to_string())?;
    Ok(EmployeeDirectory { employees, roles })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateEmployeeInput {
    username: String,
    display_name: String,
    pin: String,
    #[serde(default)]
    role_ids: Vec<String>,
}

#[tauri::command]
pub fn create_employee(
    state: State<'_, DbState>,
    input: CreateEmployeeInput,
) -> Result<EmployeeView, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_active_context(&connection, &state, Some("users.manage"))?;
    employee::create_employee(
        &connection,
        &context,
        employee::NewEmployee {
            username: &input.username,
            display_name: &input.display_name,
            pin: &input.pin,
            role_ids: &input.role_ids,
        },
    )
    .map_err(|error| error.to_string())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateEmployeeInput {
    employee_id: String,
    display_name: Option<String>,
    active: Option<bool>,
}

#[tauri::command]
pub fn update_employee(
    state: State<'_, DbState>,
    input: UpdateEmployeeInput,
) -> Result<EmployeeView, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_active_context(&connection, &state, Some("users.manage"))?;
    employee::update_employee(
        &connection,
        &context,
        &input.employee_id,
        input.display_name.as_deref(),
        input.active,
    )
    .map_err(|error| error.to_string())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetPinInput {
    employee_id: String,
    pin: String,
}

#[tauri::command]
pub fn reset_employee_pin(
    state: State<'_, DbState>,
    input: ResetPinInput,
) -> Result<EmployeeView, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_active_context(&connection, &state, Some("users.manage"))?;
    employee::reset_employee_pin(&connection, &context, &input.employee_id, &input.pin)
        .map_err(|error| error.to_string())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetRolesInput {
    employee_id: String,
    role_ids: Vec<String>,
}

#[tauri::command]
pub fn set_employee_roles(
    state: State<'_, DbState>,
    input: SetRolesInput,
) -> Result<EmployeeView, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_active_context(&connection, &state, Some("users.manage"))?;
    employee::set_employee_roles(&connection, &context, &input.employee_id, &input.role_ids)
        .map_err(|error| error.to_string())
}
