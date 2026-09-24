use localops_core::{backup, bootstrap, safety};
use serde::Serialize;
use tauri::State;

use crate::{session_guard::require_active_context, DbState};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupDto {
    filename: String,
    kind: String,
    size_bytes: u64,
    verified: bool,
}

impl From<backup::BackupInfo> for BackupDto {
    fn from(value: backup::BackupInfo) -> Self {
        Self {
            filename: value.filename,
            kind: value.kind,
            size_bytes: value.size_bytes,
            verified: value.verified,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SafetyStatusDto {
    schema_version: i64,
    latest_schema_version: i64,
    integrity: String,
    foreign_key_violations: usize,
    inventory_reconciliation_differences: usize,
    financial_events_without_audit: i64,
    backup_count: usize,
    unverified_backup_count: usize,
    healthy: bool,
    backups: Vec<BackupDto>,
}

#[tauri::command]
pub fn get_safety_status(state: State<'_, DbState>) -> Result<SafetyStatusDto, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_active_context(&connection, &state, Some("business.manage"))?;
    let paths = bootstrap::AppPaths::windows_default().map_err(|error| error.to_string())?;
    let status = safety::inspect(&connection, &context.business_id, &paths.backups)
        .map_err(|error| error.to_string())?;
    let healthy = safety::is_healthy(&status);
    let backups = backup::list_backups(&paths.backups)
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(Into::into)
        .collect();
    Ok(SafetyStatusDto {
        schema_version: status.schema_version,
        latest_schema_version: bootstrap::LATEST_SCHEMA_VERSION,
        integrity: status.integrity,
        foreign_key_violations: status.foreign_key_violations,
        inventory_reconciliation_differences: status.inventory_reconciliation_differences,
        financial_events_without_audit: status.financial_events_without_audit,
        backup_count: status.backup_count,
        unverified_backup_count: status.unverified_backup_count,
        healthy,
        backups,
    })
}

#[tauri::command]
pub fn create_manual_backup(state: State<'_, DbState>) -> Result<BackupDto, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    require_active_context(&connection, &state, Some("business.manage"))?;
    let paths = bootstrap::AppPaths::windows_default().map_err(|error| error.to_string())?;
    backup::create_backup(&connection, &paths.backups, "manual")
        .map(Into::into)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn restore_local_backup(
    state: State<'_, DbState>,
    filename: String,
) -> Result<BackupDto, String> {
    let mut connection = state.connection.lock().map_err(|error| error.to_string())?;
    require_active_context(&connection, &state, Some("business.manage"))?;
    let paths = bootstrap::AppPaths::windows_default().map_err(|error| error.to_string())?;
    let restored = backup::restore_backup(&mut connection, &paths.backups, &filename)
        .map_err(|error| error.to_string())?;
    *state
        .current_session
        .lock()
        .map_err(|error| error.to_string())? = None;
    Ok(restored.into())
}
