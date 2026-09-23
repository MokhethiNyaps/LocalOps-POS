use localops_core::{role, session};
use rusqlite::{Connection, OptionalExtension};

use crate::DbState;

#[derive(Debug, Clone)]
pub struct ActiveContext {
    pub business_id: String,
    pub user_id: String,
    pub terminal_id: String,
    pub department_id: Option<String>,
}

pub fn require_active_context(
    connection: &Connection,
    state: &DbState,
    permission: Option<&str>,
) -> Result<ActiveContext, String> {
    let session_id = state
        .current_session
        .lock()
        .map_err(|error| error.to_string())?
        .clone()
        .ok_or_else(|| "Sign in is required".to_owned())?;
    let active = session::resume_session(connection, &session_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Your session expired. Please sign in again".to_owned())?;
    if let Some(permission_code) = permission {
        if !role::user_has_permission(connection, &active.user_id, permission_code)
            .map_err(|error| error.to_string())?
        {
            return Err(format!("Permission denied: {permission_code}"));
        }
    }
    session::touch_session(connection, &session_id).map_err(|error| error.to_string())?;
    let department_id = connection
        .query_row(
            "SELECT department_id FROM terminals WHERE id = ?1",
            [&active.terminal_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .flatten();
    Ok(ActiveContext {
        business_id: active.business_id,
        user_id: active.user_id,
        terminal_id: active.terminal_id,
        department_id,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use localops_core::{session, setup, terminal, user};

    use super::*;

    #[test]
    fn active_context_uses_session_business_and_enforces_permissions() {
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
        let state = DbState {
            connection: Mutex::new(connection),
            current_session: Mutex::new(Some(owner.session_id.clone())),
        };
        let connection = state.connection.lock().unwrap();
        let context = require_active_context(&connection, &state, Some("products.manage")).unwrap();
        assert_eq!(context.business_id, owner.business_id);
        assert_eq!(
            context.department_id.as_deref(),
            Some(owner.department_id.as_str())
        );
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
        let cashier = user::create_user(
            &connection,
            &owner.business_id,
            "cashier",
            "Cashier",
            "5678",
        )
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
}
