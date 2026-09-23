use crate::{Result, business, department, location, role, session, terminal, user};
use rusqlite::Connection;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct InitialSetup<'a> {
    pub business_name: &'a str,
    pub department_name: &'a str,
    pub location_name: &'a str,
    pub terminal_name: &'a str,
    pub owner_username: &'a str,
    pub owner_display_name: &'a str,
    pub owner_pin: &'a str,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SetupResult {
    pub business_id: String,
    pub department_id: String,
    pub location_id: String,
    pub terminal_id: String,
    pub user_id: String,
    pub role_id: String,
    pub session_id: String,
}

pub fn complete_initial_setup(
    connection: &Connection,
    input: InitialSetup<'_>,
) -> Result<SetupResult> {
    let transaction = connection.unchecked_transaction()?;
    let business_id = business::create_business(&transaction, input.business_name)?;
    let location_id = location::create_location(
        &transaction,
        &business_id,
        input.location_name,
        Some("MAIN"),
    )?;
    let department_id =
        department::create_department(&transaction, &business_id, input.department_name, None)?;
    transaction.execute(
        "INSERT INTO department_locations(department_id, location_id, is_default)
         VALUES(?1, ?2, 1)",
        (&department_id, &location_id),
    )?;
    let device_key = Uuid::now_v7().to_string();
    let terminal_id = terminal::create_terminal_with_identity(
        &transaction,
        &business_id,
        Some(&department_id),
        Some(&location_id),
        input.terminal_name,
        Some("MAIN-TILL"),
        &device_key,
    )?;
    let user_id = user::create_user(
        &transaction,
        &business_id,
        input.owner_username,
        input.owner_display_name,
        input.owner_pin,
    )?;
    let role_id = role::create_role(&transaction, &business_id, "Owner", true)?;
    for permission in role::list_permissions(&transaction)? {
        role::grant_permission(&transaction, &role_id, &permission.code, Some(&user_id))?;
    }
    role::assign_role(&transaction, &user_id, &role_id, Some(&user_id))?;
    let active_session =
        session::start_session(&transaction, &business_id, &user_id, &terminal_id)?;

    transaction.execute(
        "INSERT INTO audit_logs(
             id, business_id, user_id, terminal_id, action, entity_type,
             entity_id, occurred_at, new_json, correlation_id
         ) VALUES(
             ?1, ?2, ?3, ?4, 'INITIAL_SETUP_COMPLETED', 'business', ?2,
             strftime('%Y-%m-%dT%H:%M:%fZ','now'),
             json_object('businessName', ?5, 'departmentName', ?6, 'locationName', ?7),
             ?8
         )",
        (
            Uuid::now_v7().to_string(),
            &business_id,
            &user_id,
            &terminal_id,
            input.business_name.trim(),
            input.department_name.trim(),
            input.location_name.trim(),
            Uuid::now_v7().to_string(),
        ),
    )?;
    transaction.commit()?;

    Ok(SetupResult {
        business_id,
        department_id,
        location_id,
        terminal_id,
        user_id,
        role_id,
        session_id: active_session.id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{open_memory_database, role};

    fn setup_input<'a>(pin: &'a str) -> InitialSetup<'a> {
        InitialSetup {
            business_name: "Moko's Lifestyle Centre",
            department_name: "Bar",
            location_name: "Main Store",
            terminal_name: "Main Till",
            owner_username: "owner",
            owner_display_name: "Moko",
            owner_pin: pin,
        }
    }

    #[test]
    fn creates_complete_audited_first_run_atomically() {
        let database = open_memory_database().unwrap();
        let result = complete_initial_setup(&database, setup_input("1234")).unwrap();
        assert!(role::user_has_permission(&database, &result.user_id, "business.manage").unwrap());
        assert!(role::user_has_permission(&database, &result.user_id, "sales.create").unwrap());

        let facts: (i64, i64, i64) = database
            .query_row(
                "SELECT
                    (SELECT count(*) FROM department_locations WHERE department_id = ?1 AND is_default = 1),
                    (SELECT count(*) FROM sessions WHERE id = ?2 AND ended_at IS NULL),
                    (SELECT count(*) FROM audit_logs WHERE action = 'INITIAL_SETUP_COMPLETED')",
                (&result.department_id, &result.session_id),
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(facts, (1, 1, 1));
    }

    #[test]
    fn rolls_back_everything_when_owner_pin_is_invalid() {
        let database = open_memory_database().unwrap();
        assert!(complete_initial_setup(&database, setup_input("12ab")).is_err());
        let businesses: i64 = database
            .query_row("SELECT count(*) FROM businesses", [], |row| row.get(0))
            .unwrap();
        let terminals: i64 = database
            .query_row("SELECT count(*) FROM terminals", [], |row| row.get(0))
            .unwrap();
        assert_eq!((businesses, terminals), (0, 0));
    }
}
