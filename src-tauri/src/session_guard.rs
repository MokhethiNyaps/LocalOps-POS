//! Centralised command guard.
//!
//! Every exposed Tauri command is deliberately classified (see
//! `docs/localops-pos/AUTHORIZATION-INVENTORY.md`):
//!
//! * `PUBLIC_PREAUTH` — bootstrap, login and setup-required initial setup only.
//! * `AUTHENTICATED_GENERAL` — any valid session, no business capability.
//! * `CASHIER_SCOPED` — permission plus user/terminal/department/shift scope.
//! * `OWNER_OR_PERMISSION` — explicit permission, never the role display name.
//! * `OWNER_ONLY` — permission plus the `OWNER` system role in V1.
//!
//! Authorization itself lives in `localops_core::access`; this module only
//! bridges the Tauri session state to that application boundary.

use localops_core::{access::AccessContext, role};
use rusqlite::Connection;

use crate::DbState;

/// Resolve the authenticated access context, optionally requiring a permission.
pub fn require_active_context(
    connection: &Connection,
    state: &DbState,
    permission: Option<&str>,
) -> Result<AccessContext, String> {
    let session_id = state
        .current_session
        .lock()
        .map_err(|error| error.to_string())?
        .clone()
        .ok_or_else(|| "Sign in is required".to_owned())?;
    let context = localops_core::access::resolve(connection, &session_id)
        .map_err(|_| "Your session expired. Please sign in again".to_owned())?;
    if let Some(permission_code) = permission {
        context
            .require_permission(permission_code)
            .map_err(|error| error.to_string())?;
    }
    Ok(context)
}

/// `OWNER_ONLY`: the permission plus the Owner system role. Used for the
/// deliberately undelegated V1 capabilities such as restoring a backup.
pub fn require_owner(
    connection: &Connection,
    state: &DbState,
    permission: &str,
) -> Result<AccessContext, String> {
    let context = require_active_context(connection, state, Some(permission))?;
    if !context.is_system_role(role::OWNER_ROLE_KEY) {
        return Err("Permission denied: this action is restricted to the Owner".to_owned());
    }
    Ok(context)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use localops_core::{access::fixtures, session, setup, terminal, user};

    use super::*;

    fn owner_state() -> (DbState, String) {
        let connection = localops_core::open_memory_database().unwrap();
        let owner = setup::complete_initial_setup(
            &connection,
            setup::InitialSetup {
                business_name: "LocalOps",
                department_name: "Shop",
                location_name: "Store",
                terminal_name: "Till",
                owner_username: "owner",
                owner_display_name: "Owner",
                owner_pin: "1234",
            },
        )
        .unwrap();
        let business_id = owner.business_id.clone();
        (
            DbState {
                connection: Mutex::new(connection),
                current_session: Mutex::new(Some(owner.session_id.clone())),
            },
            business_id,
        )
    }

    #[test]
    fn active_context_uses_session_business_and_enforces_permissions() {
        let (state, business_id) = owner_state();
        let connection = state.connection.lock().unwrap();
        let context = require_active_context(&connection, &state, Some("products.manage")).unwrap();
        assert_eq!(context.business_id, business_id);
        assert!(context.department_id.is_some());
        assert!(context.is_owner());
    }

    #[test]
    fn rejects_an_authenticated_user_without_catalogue_permission() {
        let connection = localops_core::open_memory_database().unwrap();
        let owner = setup::complete_initial_setup(
            &connection,
            setup::InitialSetup {
                business_name: "LocalOps",
                department_name: "Shop",
                location_name: "Store",
                terminal_name: "Till",
                owner_username: "owner",
                owner_display_name: "Owner",
                owner_pin: "1234",
            },
        )
        .unwrap();
        let cashier =
            user::create_user(&connection, &owner.business_id, "cashier", "Cashier", "5678")
                .unwrap();
        let terminal = terminal::get_terminal(&connection, &owner.terminal_id)
            .unwrap()
            .unwrap();
        let cashier_session =
            session::start_session(&connection, &owner.business_id, &cashier, &terminal.id)
                .unwrap();
        let state = DbState {
            connection: Mutex::new(connection),
            current_session: Mutex::new(Some(cashier_session.id)),
        };
        let connection = state.connection.lock().unwrap();
        let error = require_active_context(&connection, &state, Some("products.manage"))
            .expect_err("cashier should not manage the catalogue");
        assert!(error.contains("Permission denied"));
    }

    #[test]
    fn owner_only_commands_reject_a_permissioned_non_owner() {
        let connection = localops_core::open_memory_database().unwrap();
        let fixture = fixtures::business(&connection);
        let (cashier_id, cashier_session) =
            fixtures::cashier(&connection, &fixture.business_id, &fixture.terminal_id, "thabo")
                .unwrap();
        // Deliberately grant the backup permission through a custom role.
        let custom = role::create_role(&connection, &fixture.business_id, "Backup Operator", false)
            .unwrap();
        role::grant_permission(&connection, &custom, "backups.manage", None).unwrap();
        role::assign_role(&connection, &cashier_id, &custom, None).unwrap();
        let state = DbState {
            connection: Mutex::new(connection),
            current_session: Mutex::new(Some(cashier_session)),
        };
        let connection = state.connection.lock().unwrap();
        assert!(require_active_context(&connection, &state, Some("backups.manage")).is_ok());
        assert!(require_owner(&connection, &state, "backups.manage").is_err());
    }

    #[test]
    fn an_invalidated_session_can_no_longer_run_protected_commands() {
        let (state, _) = owner_state();
        {
            let connection = state.connection.lock().unwrap();
            let session_id = state.current_session.lock().unwrap().clone().unwrap();
            session::end_session(&connection, &session_id, "LOGOUT").unwrap();
        }
        let connection = state.connection.lock().unwrap();
        assert!(require_active_context(&connection, &state, None).is_err());
    }
}
