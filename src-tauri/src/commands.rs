// src-tauri/src/commands.rs
//
// Pre-authentication surface and session lifecycle.
//
// Classification (see docs/localops-pos/AUTHORIZATION-INVENTORY.md):
//   get_app_bootstrap        PUBLIC_PREAUTH (safe bootstrap data only)
//   login                    PUBLIC_PREAUTH
//   complete_initial_setup   PUBLIC_PREAUTH only while setup is required
//   logout                   AUTHENTICATED_GENERAL
//   get_session_capabilities AUTHENTICATED_GENERAL
//   update_trading_name      OWNER_OR_PERMISSION (business.manage)
//
// The legacy public `create_business` and `list_businesses` commands were
// removed from the runtime surface: first-run setup is the supported creation
// workflow and the bootstrap contract replaces the management list.

use localops_core::{
    access::AccessContext,
    business, role, session, setup, terminal, user,
};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::session_guard::require_active_context;

/// The canonical authenticated session contract.
///
/// Normal login and first-run setup both return this shape so the frontend
/// never has two incompatible session models.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionPayload {
    pub session_id: String,
    pub business_id: String,
    pub business_name: String,
    pub user_id: String,
    pub username: String,
    pub display_name: String,
    pub terminal_id: String,
    pub terminal_name: String,
    pub department_id: Option<String>,
    pub department_name: Option<String>,
    pub permissions: Vec<String>,
    pub system_roles: Vec<String>,
    pub role_label: String,
}

impl From<AccessContext> for SessionPayload {
    fn from(value: AccessContext) -> Self {
        Self {
            session_id: value.session_id,
            business_id: value.business_id,
            business_name: value.business_name,
            user_id: value.user_id,
            username: value.username,
            display_name: value.display_name,
            terminal_id: value.terminal_id,
            terminal_name: value.terminal_name,
            department_id: value.department_id,
            department_name: value.department_name,
            role_label: system_role_label(&value.system_role_keys).to_owned(),
            permissions: value.permissions,
            system_roles: value.system_role_keys,
        }
    }
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

/// `PUBLIC_PREAUTH`, and only while first-run setup is genuinely required.
#[tauri::command]
pub fn complete_initial_setup(
    state: State<'_, super::DbState>,
    input: InitialSetupInput,
) -> Result<SessionPayload, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    if !setup::setup_required(&connection).map_err(|error| error.to_string())? {
        return Err("Initial setup has already been completed. Please sign in.".to_owned());
    }
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
    localops_core::access::resolve(&connection, &result.session_id)
        .map(SessionPayload::from)
        .map_err(|error| error.to_string())
}

/// `PUBLIC_PREAUTH`. Authenticates and establishes the session scope. The
/// selected terminal must be active and belong to the selected business; after
/// this point the session terminal cannot be changed by payload tampering.
#[tauri::command]
pub fn login(
    state: State<'_, super::DbState>,
    business_id: String,
    terminal_id: String,
    username: String,
    pin: String,
) -> Result<SessionPayload, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let authenticated = user::authenticate_user(&connection, &business_id, &username, &pin)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| {
            "Invalid username or PIN, or the account is temporarily locked".to_owned()
        })?;
    let active = session::start_session(&connection, &business_id, &authenticated.id, &terminal_id)
        .map_err(|_| "That terminal is not available for this business".to_owned())?;
    let mut current = state
        .current_session
        .lock()
        .map_err(|error| error.to_string())?;
    if let Some(previous) = current.replace(active.id.clone()) {
        let _ = session::end_session(&connection, &previous, "REPLACED");
    }
    localops_core::access::resolve(&connection, &active.id)
        .map(SessionPayload::from)
        .map_err(|error| error.to_string())
}

/// `AUTHENTICATED_GENERAL`. Lets the frontend rebuild its capability model.
#[tauri::command]
pub fn get_session_capabilities(
    state: State<'_, super::DbState>,
) -> Result<SessionPayload, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    require_active_context(&connection, &state, None).map(SessionPayload::from)
}

/// `OWNER_OR_PERMISSION` (`business.manage`). The session business is used; a
/// supplied identifier can never target another business.
#[tauri::command]
pub fn update_trading_name(
    state: State<'_, super::DbState>,
    trading_name: Option<String>,
) -> Result<(), String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_active_context(&connection, &state, Some("business.manage"))?;
    business::update_business_trading_name(
        &connection,
        &context.business_id,
        trading_name.as_deref(),
    )
    .map_err(|error| error.to_string())
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

/// `PUBLIC_PREAUTH`. Returns only what the sign-in screen needs: the business
/// name and its active terminals. No management data is exposed.
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
        setup_required: setup::setup_required(&connection).map_err(|error| error.to_string())?,
        businesses,
    })
}

/// `AUTHENTICATED_GENERAL`.
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

/// Roles that may be shown in the Owner employee screen.
pub fn system_role_label(keys: &[String]) -> &'static str {
    if keys.iter().any(|value| value == role::OWNER_ROLE_KEY) {
        "Owner"
    } else if keys.iter().any(|value| value == role::CASHIER_ROLE_KEY) {
        "Cashier"
    } else {
        "Employee"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_label_is_derived_from_system_roles() {
        assert_eq!(system_role_label(&["OWNER".to_owned()]), "Owner");
        assert_eq!(system_role_label(&["CASHIER".to_owned()]), "Cashier");
        assert_eq!(
            system_role_label(&["CASHIER".to_owned(), "OWNER".to_owned()]),
            "Owner"
        );
        assert_eq!(system_role_label(&[]), "Employee");
    }
}
