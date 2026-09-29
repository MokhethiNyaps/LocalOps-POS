//! Minimal Owner-facing employee administration.
//!
//! `user_roles` is the authoritative role-assignment relationship. The legacy
//! `users.role_id` column is never written or read by this service.
//!
//! Every authority change (deactivation, PIN reset, role change) invalidates
//! all active sessions belonging to the affected employee so a stale frontend
//! can never keep permissions it no longer has.

use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;

use crate::{CoreError, Result, access::AccessContext, audit, role, session, user};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeRoleView {
    pub id: String,
    pub name: String,
    pub role_key: Option<String>,
    pub system: bool,
}

/// Employee data safe for the Owner employee screen. PIN hashes are never part
/// of this DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeView {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub active: bool,
    pub created_at: String,
    pub roles: Vec<EmployeeRoleView>,
    pub is_owner: bool,
}

fn employee_view(connection: &Connection, user: &user::User) -> Result<EmployeeView> {
    let roles = role::list_user_roles(connection, &user.id)?
        .into_iter()
        .map(|value| EmployeeRoleView {
            id: value.id,
            name: value.name,
            role_key: value.role_key,
            system: value.system,
        })
        .collect::<Vec<_>>();
    let is_owner = roles
        .iter()
        .any(|value| value.role_key.as_deref() == Some(role::OWNER_ROLE_KEY));
    Ok(EmployeeView {
        id: user.id.clone(),
        username: user.username.clone(),
        display_name: user.display_name.clone(),
        active: user.active,
        created_at: user.created_at.clone(),
        roles,
        is_owner,
    })
}

fn load_user(connection: &Connection, business_id: &str, user_id: &str) -> Result<user::User> {
    let found = connection
        .query_row(
            "SELECT id, business_id, username, display_name, active, created_at
             FROM users WHERE id = ?1 AND business_id = ?2",
            (user_id, business_id),
            |row| {
                Ok(user::User {
                    id: row.get(0)?,
                    business_id: row.get(1)?,
                    username: row.get(2)?,
                    display_name: row.get(3)?,
                    active: row.get(4)?,
                    created_at: row.get(5)?,
                })
            },
        )
        .optional()?;
    found.ok_or(CoreError::UserNotFound)
}

/// Every employee in the authenticated business, active and inactive.
pub fn list_employees(
    connection: &Connection,
    context: &AccessContext,
) -> Result<Vec<EmployeeView>> {
    context.require_permission("users.manage")?;
    let mut statement = connection.prepare(
        "SELECT id, business_id, username, display_name, active, created_at
         FROM users WHERE business_id = ?1 ORDER BY active DESC, display_name",
    )?;
    let users = statement
        .query_map([&context.business_id], |row| {
            Ok(user::User {
                id: row.get(0)?,
                business_id: row.get(1)?,
                username: row.get(2)?,
                display_name: row.get(3)?,
                active: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    users
        .iter()
        .map(|value| employee_view(connection, value))
        .collect()
}

/// Roles that the employee screen may assign in this release: the deterministic
/// system roles plus any existing custom roles already used by the business.
pub fn list_assignable_roles(
    connection: &Connection,
    context: &AccessContext,
) -> Result<Vec<EmployeeRoleView>> {
    context.require_permission("users.manage")?;
    role::ensure_system_roles(connection, &context.business_id)?;
    Ok(role::list_roles(connection, &context.business_id)?
        .into_iter()
        .map(|value| EmployeeRoleView {
            id: value.id,
            name: value.name,
            role_key: value.role_key,
            system: value.system,
        })
        .collect())
}

fn active_owner_count(connection: &Connection, business_id: &str) -> Result<i64> {
    let count = connection.query_row(
        "SELECT count(DISTINCT u.id)
         FROM users u
         JOIN user_roles ur ON ur.user_id = u.id
         JOIN roles r ON r.id = ur.role_id
         WHERE u.business_id = ?1 AND u.active = 1 AND r.active = 1
           AND r.role_key = ?2",
        (business_id, role::OWNER_ROLE_KEY),
        |row| row.get(0),
    )?;
    Ok(count)
}

/// Enforced in the application layer, not only in the UI: a business must keep
/// at least one usable Owner.
fn require_remaining_owner(connection: &Connection, business_id: &str) -> Result<()> {
    if active_owner_count(connection, business_id)? == 0 {
        return Err(CoreError::LastActiveOwnerRequired);
    }
    Ok(())
}

fn audit_employee(
    connection: &Connection,
    context: &AccessContext,
    action: &str,
    employee_id: &str,
    payload: Option<&str>,
) -> Result<()> {
    audit::record_event(
        connection,
        audit::NewAuditEvent {
            business_id: &context.business_id,
            user_id: Some(&context.user_id),
            terminal_id: Some(&context.terminal_id),
            action,
            entity_type: "user",
            entity_id: employee_id,
            old_json: None,
            new_json: payload,
        },
    )?;
    Ok(())
}

#[derive(Debug, Clone)]
pub struct NewEmployee<'a> {
    pub username: &'a str,
    pub display_name: &'a str,
    pub pin: &'a str,
    pub role_ids: &'a [String],
}

/// Create an employee. Role assignment is written through `user_roles`; when no
/// role is supplied the deterministic Cashier system role is used, so an Owner
/// never has to understand permission codes to create a standard till user.
pub fn create_employee(
    connection: &Connection,
    context: &AccessContext,
    input: NewEmployee<'_>,
) -> Result<EmployeeView> {
    context.require_permission("users.manage")?;
    context.require_permission("roles.manage")?;
    role::ensure_system_roles(connection, &context.business_id)?;
    let transaction = connection.unchecked_transaction()?;
    let duplicate = transaction
        .query_row(
            "SELECT 1 FROM users WHERE business_id = ?1 AND username = ?2",
            (&context.business_id, input.username.trim()),
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if duplicate {
        return Err(CoreError::DuplicateUsername);
    }
    let user_id = user::create_user(
        &transaction,
        &context.business_id,
        input.username,
        input.display_name,
        input.pin,
    )?;
    let role_ids = if input.role_ids.is_empty() {
        vec![
            role::find_role_by_key(&transaction, &context.business_id, role::CASHIER_ROLE_KEY)?
                .ok_or(CoreError::RoleNotFound)?
                .id,
        ]
    } else {
        input.role_ids.to_vec()
    };
    for role_id in &role_ids {
        assert_role_in_business(&transaction, &context.business_id, role_id)?;
        role::assign_role(&transaction, &user_id, role_id, Some(&context.user_id))?;
    }
    audit_employee(
        &transaction,
        context,
        "USER_CREATED",
        &user_id,
        Some(&format!(
            "{{\"username\":\"{}\",\"roleCount\":{}}}",
            input.username.trim().replace('"', ""),
            role_ids.len()
        )),
    )?;
    let created = load_user(&transaction, &context.business_id, &user_id)?;
    let view = employee_view(&transaction, &created)?;
    transaction.commit()?;
    Ok(view)
}

fn assert_role_in_business(connection: &Connection, business_id: &str, role_id: &str) -> Result<()> {
    let ok = connection
        .query_row(
            "SELECT 1 FROM roles WHERE id = ?1 AND business_id = ?2 AND active = 1",
            (role_id, business_id),
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if ok { Ok(()) } else { Err(CoreError::RoleNotFound) }
}

/// Update an employee's display name and/or active state.
pub fn update_employee(
    connection: &Connection,
    context: &AccessContext,
    employee_id: &str,
    display_name: Option<&str>,
    active: Option<bool>,
) -> Result<EmployeeView> {
    context.require_permission("users.manage")?;
    let existing = load_user(connection, &context.business_id, employee_id)?;
    let transaction = connection.unchecked_transaction()?;
    if let Some(name) = display_name {
        user::update_user(&transaction, employee_id, name)?;
    }
    let mut invalidate = false;
    if let Some(next_active) = active {
        if next_active != existing.active {
            transaction.execute(
                "UPDATE users
                 SET active = ?2, failed_attempts = 0, locked_until = NULL,
                     updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
                 WHERE id = ?1",
                (employee_id, next_active),
            )?;
            invalidate = true;
            require_remaining_owner(&transaction, &context.business_id)?;
            audit_employee(
                &transaction,
                context,
                if next_active {
                    "USER_ACTIVATED"
                } else {
                    "USER_DEACTIVATED"
                },
                employee_id,
                None,
            )?;
        }
    }
    if invalidate {
        session::end_sessions_for_user(&transaction, employee_id, "AUTHORITY_CHANGED")?;
    }
    let updated = load_user(&transaction, &context.business_id, employee_id)?;
    let view = employee_view(&transaction, &updated)?;
    transaction.commit()?;
    Ok(view)
}

/// Reset an employee PIN using the existing Argon2id hashing. The plaintext PIN
/// is never audited and all of that user's sessions are invalidated.
pub fn reset_employee_pin(
    connection: &Connection,
    context: &AccessContext,
    employee_id: &str,
    new_pin: &str,
) -> Result<EmployeeView> {
    context.require_permission("users.manage")?;
    let existing = load_user(connection, &context.business_id, employee_id)?;
    let transaction = connection.unchecked_transaction()?;
    user::change_user_pin(&transaction, &existing.id, new_pin)?;
    session::end_sessions_for_user(&transaction, &existing.id, "AUTHORITY_CHANGED")?;
    audit_employee(&transaction, context, "USER_PIN_RESET", &existing.id, None)?;
    let view = employee_view(&transaction, &existing)?;
    transaction.commit()?;
    Ok(view)
}

/// Replace an employee's role assignments. `user_roles` is authoritative.
pub fn set_employee_roles(
    connection: &Connection,
    context: &AccessContext,
    employee_id: &str,
    role_ids: &[String],
) -> Result<EmployeeView> {
    context.require_permission("users.manage")?;
    context.require_permission("roles.manage")?;
    let existing = load_user(connection, &context.business_id, employee_id)?;
    let transaction = connection.unchecked_transaction()?;
    transaction.execute("DELETE FROM user_roles WHERE user_id = ?1", [&existing.id])?;
    for role_id in role_ids {
        assert_role_in_business(&transaction, &context.business_id, role_id)?;
        role::assign_role(&transaction, &existing.id, role_id, Some(&context.user_id))?;
    }
    require_remaining_owner(&transaction, &context.business_id)?;
    session::end_sessions_for_user(&transaction, &existing.id, "AUTHORITY_CHANGED")?;
    audit_employee(
        &transaction,
        context,
        "USER_ROLES_CHANGED",
        &existing.id,
        Some(&format!("{{\"roleCount\":{}}}", role_ids.len())),
    )?;
    let view = employee_view(&transaction, &existing)?;
    transaction.commit()?;
    Ok(view)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{access, open_memory_database};

    #[test]
    fn owner_creates_a_cashier_whose_role_comes_from_user_roles() {
        let database = open_memory_database().unwrap();
        let fixture = access::fixtures::business(&database);
        let owner = access::resolve(&database, &fixture.owner_session).unwrap();
        let created = create_employee(
            &database,
            &owner,
            NewEmployee {
                username: "thabo",
                display_name: "Thabo",
                pin: "5678",
                role_ids: &[],
            },
        )
        .unwrap();
        assert!(!created.is_owner);
        assert_eq!(created.roles.len(), 1);
        assert_eq!(created.roles[0].role_key.as_deref(), Some("CASHIER"));
        let assignments: i64 = database
            .query_row(
                "SELECT count(*) FROM user_roles WHERE user_id = ?1",
                [&created.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(assignments, 1);
        let hash: String = database
            .query_row("SELECT pin_hash FROM users WHERE id = ?1", [&created.id], |row| {
                row.get(0)
            })
            .unwrap();
        assert!(hash.starts_with("$argon2id$"));
        assert!(!hash.contains("5678"));
    }

    #[test]
    fn cashier_cannot_use_employee_administration() {
        let database = open_memory_database().unwrap();
        let fixture = access::fixtures::business(&database);
        let (cashier_id, session_id) = access::fixtures::cashier(
            &database,
            &fixture.business_id,
            &fixture.terminal_id,
            "thabo",
        )
        .unwrap();
        let cashier = access::resolve(&database, &session_id).unwrap();
        assert!(list_employees(&database, &cashier).is_err());
        assert!(
            create_employee(
                &database,
                &cashier,
                NewEmployee {
                    username: "sneaky",
                    display_name: "Sneaky",
                    pin: "1111",
                    role_ids: &[],
                }
            )
            .is_err()
        );
        assert!(update_employee(&database, &cashier, &fixture.owner_id, None, Some(false)).is_err());
        assert!(reset_employee_pin(&database, &cashier, &fixture.owner_id, "9999").is_err());
        let owner_role =
            role::find_role_by_key(&database, &fixture.business_id, role::OWNER_ROLE_KEY)
                .unwrap()
                .unwrap();
        assert!(set_employee_roles(&database, &cashier, &cashier_id, &[owner_role.id]).is_err());
    }

    #[test]
    fn deactivation_pin_reset_and_role_change_invalidate_sessions() {
        let database = open_memory_database().unwrap();
        let fixture = access::fixtures::business(&database);
        let owner = access::resolve(&database, &fixture.owner_session).unwrap();
        let (cashier_id, cashier_session) = access::fixtures::cashier(
            &database,
            &fixture.business_id,
            &fixture.terminal_id,
            "thabo",
        )
        .unwrap();
        assert!(access::resolve(&database, &cashier_session).is_ok());

        reset_employee_pin(&database, &owner, &cashier_id, "4321").unwrap();
        assert!(matches!(
            access::resolve(&database, &cashier_session),
            Err(CoreError::SessionNotFound)
        ));

        let second = session::start_session(
            &database,
            &fixture.business_id,
            &cashier_id,
            &fixture.terminal_id,
        )
        .unwrap();
        let cashier_role =
            role::find_role_by_key(&database, &fixture.business_id, role::CASHIER_ROLE_KEY)
                .unwrap()
                .unwrap();
        set_employee_roles(&database, &owner, &cashier_id, &[cashier_role.id]).unwrap();
        assert!(matches!(
            access::resolve(&database, &second.id),
            Err(CoreError::SessionNotFound)
        ));

        let third = session::start_session(
            &database,
            &fixture.business_id,
            &cashier_id,
            &fixture.terminal_id,
        )
        .unwrap();
        update_employee(&database, &owner, &cashier_id, None, Some(false)).unwrap();
        assert!(matches!(
            access::resolve(&database, &third.id),
            Err(CoreError::SessionNotFound)
        ));
        // A deactivated employee can no longer authenticate.
        assert!(
            user::authenticate_user(&database, &fixture.business_id, "thabo", "4321")
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn the_last_active_owner_cannot_be_removed() {
        let database = open_memory_database().unwrap();
        let fixture = access::fixtures::business(&database);
        let owner = access::resolve(&database, &fixture.owner_session).unwrap();
        assert!(matches!(
            update_employee(&database, &owner, &fixture.owner_id, None, Some(false)),
            Err(CoreError::LastActiveOwnerRequired)
        ));
        let cashier_role =
            role::find_role_by_key(&database, &fixture.business_id, role::CASHIER_ROLE_KEY)
                .unwrap()
                .unwrap();
        assert!(matches!(
            set_employee_roles(&database, &owner, &fixture.owner_id, &[cashier_role.id]),
            Err(CoreError::LastActiveOwnerRequired)
        ));
        // The Owner is still usable after both rejected operations.
        let still_owner = load_user(&database, &fixture.business_id, &fixture.owner_id).unwrap();
        assert!(still_owner.active);
        assert!(access::resolve(&database, &fixture.owner_session)
            .unwrap()
            .is_owner());
    }

    #[test]
    fn a_second_owner_allows_deactivating_the_first() {
        let database = open_memory_database().unwrap();
        let fixture = access::fixtures::business(&database);
        let owner = access::resolve(&database, &fixture.owner_session).unwrap();
        let owner_role =
            role::find_role_by_key(&database, &fixture.business_id, role::OWNER_ROLE_KEY)
                .unwrap()
                .unwrap();
        create_employee(
            &database,
            &owner,
            NewEmployee {
                username: "second",
                display_name: "Second Owner",
                pin: "2468",
                role_ids: &[owner_role.id],
            },
        )
        .unwrap();
        let updated =
            update_employee(&database, &owner, &fixture.owner_id, None, Some(false)).unwrap();
        assert!(!updated.active);
    }

    #[test]
    fn multiple_roles_combine_and_cashier_does_not_cancel_authority() {
        let database = open_memory_database().unwrap();
        let fixture = access::fixtures::business(&database);
        let owner = access::resolve(&database, &fixture.owner_session).unwrap();
        let reporting = role::create_role(&database, &fixture.business_id, "Reporting", false).unwrap();
        role::grant_permission(&database, &reporting, "reports.view", None).unwrap();
        let cashier_role =
            role::find_role_by_key(&database, &fixture.business_id, role::CASHIER_ROLE_KEY)
                .unwrap()
                .unwrap();
        let created = create_employee(
            &database,
            &owner,
            NewEmployee {
                username: "lerato",
                display_name: "Lerato",
                pin: "3333",
                role_ids: &[cashier_role.id.clone(), reporting.clone()],
            },
        )
        .unwrap();
        let active = session::start_session(
            &database,
            &fixture.business_id,
            &created.id,
            &fixture.terminal_id,
        )
        .unwrap();
        let context = access::resolve(&database, &active.id).unwrap();
        assert!(context.has_permission("sales.create"));
        assert!(context.has_permission("reports.view"));
        assert!(!context.has_permission("reports.export"));

        // An inactive role grants nothing.
        role::deactivate_role(&database, &reporting).unwrap();
        let refreshed = access::resolve(&database, &active.id).unwrap();
        assert!(!refreshed.has_permission("reports.view"));
        assert!(refreshed.has_permission("sales.create"));
    }
}
