use std::path::Path;

use rusqlite::Connection;

use crate::{Result, backup, bootstrap, inventory};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafetyStatus {
    pub schema_version: i64,
    pub integrity: String,
    pub foreign_key_violations: usize,
    pub inventory_reconciliation_differences: usize,
    pub financial_events_without_audit: i64,
    pub backup_count: usize,
    pub unverified_backup_count: usize,
}

pub fn inspect(
    connection: &Connection,
    business_id: &str,
    backup_dir: &Path,
) -> Result<SafetyStatus> {
    let schema_version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    let integrity: String = connection.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
    let foreign_key_violations = connection
        .prepare("PRAGMA foreign_key_check")?
        .query([])?
        .mapped(|_| Ok(()))
        .count();
    let inventory_reconciliation_differences =
        inventory::reconcile_balances(connection, business_id)?.len();
    let financial_events_without_audit: i64 = connection.query_row(
        "SELECT
           (SELECT COUNT(*) FROM sales s WHERE s.business_id = ?1 AND s.status != 'DRAFT'
              AND NOT EXISTS(SELECT 1 FROM audit_logs a WHERE a.entity_type = 'sale' AND a.entity_id = s.id))
         + (SELECT COUNT(*) FROM refunds r WHERE r.business_id = ?1
              AND NOT EXISTS(SELECT 1 FROM audit_logs a WHERE a.entity_type = 'refund' AND a.entity_id = r.id))
         + (SELECT COUNT(*) FROM expenses e WHERE e.business_id = ?1
              AND NOT EXISTS(SELECT 1 FROM audit_logs a WHERE a.entity_type = 'expense' AND a.entity_id = e.id))
         + (SELECT COUNT(*) FROM shifts sh WHERE sh.business_id = ?1 AND sh.status = 'CLOSED'
              AND NOT EXISTS(SELECT 1 FROM audit_logs a WHERE a.entity_type = 'shift' AND a.entity_id = sh.id AND a.action = 'SHIFT_CLOSED'))",
        [business_id],
        |row| row.get(0),
    )?;
    let backups = backup::list_backups(backup_dir)?;
    Ok(SafetyStatus {
        schema_version,
        integrity,
        foreign_key_violations,
        inventory_reconciliation_differences,
        financial_events_without_audit,
        backup_count: backups.len(),
        unverified_backup_count: backups.iter().filter(|item| !item.verified).count(),
    })
}

pub fn is_healthy(status: &SafetyStatus) -> bool {
    status.schema_version == bootstrap::LATEST_SCHEMA_VERSION
        && status.integrity == "ok"
        && status.foreign_key_violations == 0
        && status.inventory_reconciliation_differences == 0
        && status.financial_events_without_audit == 0
        && status.unverified_backup_count == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setup;
    use tempfile::tempdir;

    #[test]
    fn healthy_offline_database_passes_all_reconciliation_checks() {
        let database = crate::open_memory_database().unwrap();
        let setup = setup::complete_initial_setup(
            &database,
            setup::InitialSetup {
                business_name: "Safe Business",
                department_name: "Main",
                location_name: "Store",
                terminal_name: "Till",
                owner_username: "owner",
                owner_display_name: "Owner",
                owner_pin: "1234",
            },
        )
        .unwrap();
        let temp = tempdir().unwrap();
        crate::backup::create_backup(&database, temp.path(), "manual").unwrap();
        let status = inspect(&database, &setup.business_id, temp.path()).unwrap();
        assert!(is_healthy(&status));
        assert_eq!(status.backup_count, 1);
    }
}
