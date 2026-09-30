//! Central authorization and scoping for the Owner/Cashier security model.
//!
//! Every authenticated command resolves an [`AccessContext`] from the session
//! and then asks this module the questions required by the specification:
//! who are you, what may you do, whose records may you read, on which terminal,
//! in which department and during which shift.
//!
//! Frontend hiding is UX only — this module is the authoritative boundary.

use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;

use crate::{CoreError, Result, role, session, shift};

/// Everything the application layer may trust about the caller.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessContext {
    pub session_id: String,
    pub user_id: String,
    pub business_id: String,
    pub terminal_id: String,
    pub department_id: Option<String>,
    pub display_name: String,
    pub username: String,
    pub terminal_name: String,
    pub department_name: Option<String>,
    pub business_name: String,
    pub permissions: Vec<String>,
    pub system_role_keys: Vec<String>,
}

impl AccessContext {
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.iter().any(|value| value == permission)
    }

    pub fn has_any_permission(&self, permissions: &[&str]) -> bool {
        permissions
            .iter()
            .any(|permission| self.has_permission(permission))
    }

    pub fn is_system_role(&self, role_key: &str) -> bool {
        self.system_role_keys.iter().any(|value| value == role_key)
    }

    pub fn is_owner(&self) -> bool {
        self.is_system_role(role::OWNER_ROLE_KEY)
    }

    pub fn require_permission(&self, permission: &str) -> Result<()> {
        if self.has_permission(permission) {
            return Ok(());
        }
        Err(CoreError::PermissionDenied(permission.to_owned()))
    }

    pub fn require_any_permission(&self, permissions: &[&str]) -> Result<()> {
        if self.has_any_permission(permissions) {
            return Ok(());
        }
        Err(CoreError::PermissionDenied(permissions.join(" or ")))
    }

    /// The department implied by the authenticated session terminal. Fails
    /// closed when the terminal has no active department.
    pub fn require_department(&self) -> Result<&str> {
        self.department_id
            .as_deref()
            .ok_or(CoreError::TerminalDepartmentRequired)
    }

    /// Reject any client-supplied identifier that tries to widen the session
    /// scope. Owner functionality may still pass the session's own values.
    pub fn require_same_business(&self, business_id: &str) -> Result<()> {
        if business_id == self.business_id {
            return Ok(());
        }
        Err(CoreError::ScopeDenied)
    }

    pub fn require_same_terminal(&self, terminal_id: &str) -> Result<()> {
        if terminal_id == self.terminal_id {
            return Ok(());
        }
        Err(CoreError::ScopeDenied)
    }

    pub fn require_same_department(&self, department_id: &str) -> Result<()> {
        if Some(department_id) == self.department_id.as_deref() {
            return Ok(());
        }
        Err(CoreError::ScopeDenied)
    }

    pub fn require_same_user(&self, user_id: &str) -> Result<()> {
        if user_id == self.user_id {
            return Ok(());
        }
        Err(CoreError::ScopeDenied)
    }
}

/// Resolve the access context for an active session, refreshing its activity
/// timestamp. Returns [`CoreError::SessionNotFound`] for expired, ended or
/// invalidated sessions.
pub fn resolve(connection: &Connection, session_id: &str) -> Result<AccessContext> {
    let active = session::resume_session(connection, session_id)?.ok_or(CoreError::SessionNotFound)?;
    session::touch_session(connection, session_id)?;
    let (display_name, username) = connection
        .query_row(
            "SELECT display_name, username FROM users WHERE id = ?1 AND active = 1",
            [&active.user_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?
        .ok_or(CoreError::UserNotFound)?;
    let (terminal_name, department_id, department_name) = connection
        .query_row(
            "SELECT t.name, d.id, d.name
             FROM terminals t
             LEFT JOIN departments d ON d.id = t.department_id AND d.active = 1
             WHERE t.id = ?1 AND t.business_id = ?2 AND t.active = 1",
            (&active.terminal_id, &active.business_id),
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            },
        )
        .optional()?
        .ok_or(CoreError::TerminalNotFound)?;
    let business_name: String = connection.query_row(
        "SELECT name FROM businesses WHERE id = ?1",
        [&active.business_id],
        |row| row.get(0),
    )?;
    let permissions = role::list_user_permissions(connection, &active.user_id)?
        .into_iter()
        .map(|permission| permission.code)
        .collect();
    let system_role_keys = role::user_system_role_keys(connection, &active.user_id)?;
    Ok(AccessContext {
        session_id: active.id,
        user_id: active.user_id,
        business_id: active.business_id,
        terminal_id: active.terminal_id,
        department_id,
        display_name,
        username,
        terminal_name,
        department_name,
        business_name,
        permissions,
        system_role_keys,
    })
}

/// The open shift owned by the authenticated user on the authenticated terminal.
/// Another employee's open shift is never the caller's shift.
pub fn current_own_shift(
    connection: &Connection,
    context: &AccessContext,
) -> Result<Option<shift::Shift>> {
    let open = shift::get_open_shift(connection, &context.business_id, &context.terminal_id)?;
    Ok(open.filter(|value| value.user_id == context.user_id))
}

/// Fail closed when the caller has no own open shift on this terminal.
pub fn require_current_own_shift(
    connection: &Connection,
    context: &AccessContext,
) -> Result<shift::Shift> {
    current_own_shift(connection, context)?.ok_or(CoreError::OpenShiftNotFound)
}

/// Scope facts about a sale, resolved with the scope inside the query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaleScope {
    pub sale_id: String,
    pub business_id: String,
    pub terminal_id: String,
    pub shift_id: String,
    pub cashier_id: String,
}

/// Resolve a sale only when it belongs to the authenticated business.
pub fn sale_scope(connection: &Connection, context: &AccessContext, sale_id: &str) -> Result<SaleScope> {
    connection
        .query_row(
            "SELECT id, business_id, terminal_id, shift_id, cashier_id
             FROM sales WHERE id = ?1 AND business_id = ?2",
            (sale_id, &context.business_id),
            |row| {
                Ok(SaleScope {
                    sale_id: row.get(0)?,
                    business_id: row.get(1)?,
                    terminal_id: row.get(2)?,
                    shift_id: row.get(3)?,
                    cashier_id: row.get(4)?,
                })
            },
        )
        .optional()?
        .ok_or(CoreError::ScopeDenied)
}

/// Authorize a later receipt lookup (a reprint).
///
/// * `receipts.reprint_all` — any sale in the authenticated business.
/// * `receipts.reprint_own_current_shift` — only a sale created by the caller,
///   on the session terminal, inside the caller's current open shift.
pub fn authorize_receipt_reprint(
    connection: &Connection,
    context: &AccessContext,
    sale_id: &str,
) -> Result<SaleScope> {
    if context.has_permission("receipts.reprint_all") {
        return sale_scope(connection, context, sale_id);
    }
    context.require_permission("receipts.reprint_own_current_shift")?;
    let shift = require_current_own_shift(connection, context)?;
    let scope = sale_scope(connection, context, sale_id)?;
    if scope.cashier_id != context.user_id
        || scope.terminal_id != context.terminal_id
        || scope.shift_id != shift.id
    {
        return Err(CoreError::ScopeDenied);
    }
    Ok(scope)
}

#[cfg(any(test, feature = "test-support"))]
pub mod fixtures {
    use rusqlite::Connection;

    use crate::{Result, role, session, setup, terminal, user};

    pub struct Fixture {
        pub business_id: String,
        pub department_id: String,
        pub terminal_id: String,
        pub owner_id: String,
        pub owner_session: String,
    }

    pub fn business(connection: &Connection) -> Fixture {
        let result = setup::complete_initial_setup(
            connection,
            setup::InitialSetup {
                business_name: "Moko's Lifestyle Centre",
                department_name: "Bar",
                location_name: "Main Store",
                terminal_name: "Main Bar Till",
                owner_username: "owner",
                owner_display_name: "Moko",
                owner_pin: "1234",
            },
        )
        .unwrap();
        Fixture {
            business_id: result.business_id,
            department_id: result.department_id,
            terminal_id: result.terminal_id,
            owner_id: result.user_id,
            owner_session: result.session_id,
        }
    }

    /// Create a default Cashier and sign them in at a terminal.
    pub fn cashier(
        connection: &Connection,
        business_id: &str,
        terminal_id: &str,
        username: &str,
    ) -> Result<(String, String)> {
        let user_id = user::create_user(connection, business_id, username, "Thabo", "5678")?;
        let cashier_role = role::find_role_by_key(connection, business_id, role::CASHIER_ROLE_KEY)?
            .expect("cashier system role");
        role::assign_role(connection, &user_id, &cashier_role.id, None)?;
        let active = session::start_session(connection, business_id, &user_id, terminal_id)?;
        Ok((user_id, active.id))
    }

    pub fn second_terminal(
        connection: &Connection,
        business_id: &str,
        department_name: &str,
        terminal_name: &str,
    ) -> Result<(String, String)> {
        let department_id =
            crate::department::create_department(connection, business_id, department_name, None)?;
        let terminal_id = terminal::create_terminal_with_identity(
            connection,
            business_id,
            Some(&department_id),
            None,
            terminal_name,
            None,
            &uuid::Uuid::now_v7().to_string(),
        )?;
        Ok((department_id, terminal_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_memory_database;

    #[test]
    fn owner_session_resolves_full_authority_and_department() {
        let database = open_memory_database().unwrap();
        let fixture = fixtures::business(&database);
        let context = resolve(&database, &fixture.owner_session).unwrap();
        assert!(context.is_owner());
        assert_eq!(context.business_id, fixture.business_id);
        assert_eq!(context.department_id.as_deref(), Some(fixture.department_id.as_str()));
        for permission in [
            "business.manage",
            "reports.export",
            "backups.manage",
            "shifts.close_any",
            "products.cost.view",
            "sales.discount",
        ] {
            assert!(context.has_permission(permission), "owner missing {permission}");
        }
    }

    #[test]
    fn cashier_session_receives_only_the_whitelist() {
        let database = open_memory_database().unwrap();
        let fixture = fixtures::business(&database);
        let (_, session_id) =
            fixtures::cashier(&database, &fixture.business_id, &fixture.terminal_id, "thabo")
                .unwrap();
        let context = resolve(&database, &session_id).unwrap();
        assert!(context.is_system_role(role::CASHIER_ROLE_KEY));
        assert!(!context.is_owner());
        let mut granted = context.permissions.clone();
        granted.sort();
        let mut expected = role::CASHIER_PERMISSIONS
            .iter()
            .map(|value| (*value).to_owned())
            .collect::<Vec<_>>();
        expected.sort();
        assert_eq!(granted, expected);
        for denied in [
            "products.cost.view",
            "sales.view_all",
            "sales.discount",
            "sales.refund",
            "sales.void",
            "shifts.close_any",
            "shifts.view_all",
            "shifts.expected_cash.view",
            "shifts.variance.view",
            "reports.view",
            "reports.export",
            "expenses.view",
            "users.manage",
            "roles.manage",
            "backups.manage",
            "inventory.quantity.view",
            "inventory.value.view",
            "suppliers.manage",
            "purchases.manage",
            "receipts.reprint_all",
            "audit.view",
        ] {
            assert!(!context.has_permission(denied), "cashier must not have {denied}");
            assert!(context.require_permission(denied).is_err());
        }
    }

    #[test]
    fn cashier_cannot_widen_scope_with_supplied_identifiers() {
        let database = open_memory_database().unwrap();
        let fixture = fixtures::business(&database);
        let (restaurant_department, restaurant_terminal) = fixtures::second_terminal(
            &database,
            &fixture.business_id,
            "Restaurant",
            "Restaurant Till",
        )
        .unwrap();
        let (_, session_id) =
            fixtures::cashier(&database, &fixture.business_id, &fixture.terminal_id, "thabo")
                .unwrap();
        let context = resolve(&database, &session_id).unwrap();
        assert!(context.require_same_business("another-business").is_err());
        assert!(context.require_same_terminal(&restaurant_terminal).is_err());
        assert!(context.require_same_department(&restaurant_department).is_err());
        assert!(context.require_same_user(&fixture.owner_id).is_err());
        assert!(context.require_same_terminal(&fixture.terminal_id).is_ok());
    }

    #[test]
    fn another_users_open_shift_is_not_the_callers_shift() {
        let database = open_memory_database().unwrap();
        let fixture = fixtures::business(&database);
        let (_, session_id) =
            fixtures::cashier(&database, &fixture.business_id, &fixture.terminal_id, "thabo")
                .unwrap();
        let cashier_context = resolve(&database, &session_id).unwrap();
        shift::open_shift(
            &database,
            &fixture.business_id,
            &fixture.terminal_id,
            &fixture.owner_id,
            1_000,
            "2026-09-23T08:00:00.000Z",
        )
        .unwrap();
        assert!(current_own_shift(&database, &cashier_context).unwrap().is_none());
        assert!(matches!(
            require_current_own_shift(&database, &cashier_context),
            Err(CoreError::OpenShiftNotFound)
        ));
    }

    #[test]
    fn expired_session_cannot_resolve_an_access_context() {
        let database = open_memory_database().unwrap();
        let fixture = fixtures::business(&database);
        database
            .execute(
                "UPDATE sessions SET last_active_at = '2000-01-01T00:00:00.000Z' WHERE id = ?1",
                [&fixture.owner_session],
            )
            .unwrap();
        assert!(matches!(
            resolve(&database, &fixture.owner_session),
            Err(CoreError::SessionNotFound)
        ));
    }

    #[test]
    fn receipt_reprint_is_limited_to_the_cashiers_own_current_shift() {
        let database = open_memory_database().unwrap();
        let fixture = fixtures::business(&database);
        let (cashier_id, cashier_session) =
            fixtures::cashier(&database, &fixture.business_id, &fixture.terminal_id, "thabo")
                .unwrap();
        let cashier = resolve(&database, &cashier_session).unwrap();
        let owner = resolve(&database, &fixture.owner_session).unwrap();
        let shift = crate::shift::open_own_shift(
            &database,
            &fixture.business_id,
            &fixture.terminal_id,
            &cashier_id,
            0,
            "2026-01-01T08:00:00.000Z",
        )
        .unwrap();
        let sale_id = seed_sale(
            &database,
            &fixture.business_id,
            &fixture.department_id,
            &fixture.terminal_id,
            &shift.id,
            &cashier_id,
            "S-1",
        );
        let owner_sale = seed_sale(
            &database,
            &fixture.business_id,
            &fixture.department_id,
            &fixture.terminal_id,
            &shift.id,
            &fixture.owner_id,
            "S-2",
        );

        // Own sale in the own current shift reprints.
        assert_eq!(
            authorize_receipt_reprint(&database, &cashier, &sale_id)
                .unwrap()
                .sale_id,
            sale_id
        );
        // Another user's sale in the same shift does not.
        assert!(matches!(
            authorize_receipt_reprint(&database, &cashier, &owner_sale),
            Err(CoreError::ScopeDenied)
        ));
        // The Owner may reprint any sale in the business.
        assert!(authorize_receipt_reprint(&database, &owner, &owner_sale).is_ok());

        // Once the shift is closed the Cashier can no longer reprint it.
        crate::shift::close_shift(
            &database,
            &fixture.business_id,
            &shift.id,
            &cashier_id,
            0,
            "2026-01-01T17:00:00.000Z",
            None,
        )
        .unwrap();
        assert!(authorize_receipt_reprint(&database, &cashier, &sale_id).is_err());
        assert!(authorize_receipt_reprint(&database, &owner, &sale_id).is_ok());
    }

    #[test]
    fn the_session_terminal_determines_the_department_scope() {
        let database = open_memory_database().unwrap();
        let fixture = fixtures::business(&database);
        let (restaurant_department, restaurant_terminal) =
            fixtures::second_terminal(&database, &fixture.business_id, "Restaurant", "Kitchen Till")
                .unwrap();
        let (cashier_id, bar_session) =
            fixtures::cashier(&database, &fixture.business_id, &fixture.terminal_id, "thabo")
                .unwrap();
        let bar = resolve(&database, &bar_session).unwrap();
        assert_eq!(bar.department_id.as_deref(), Some(fixture.department_id.as_str()));
        assert!(bar.require_same_terminal(&restaurant_terminal).is_err());
        assert!(bar.require_same_department(&restaurant_department).is_err());

        // Signing in again at another active terminal produces the new scope.
        crate::session::end_session(&database, &bar_session, "LOGOUT").unwrap();
        let next = crate::session::start_session(
            &database,
            &fixture.business_id,
            &cashier_id,
            &restaurant_terminal,
        )
        .unwrap();
        let restaurant = resolve(&database, &next.id).unwrap();
        assert_eq!(restaurant.terminal_id, restaurant_terminal);
        assert_eq!(
            restaurant.department_id.as_deref(),
            Some(restaurant_department.as_str())
        );
    }

    /// Insert a completed sale directly: these tests exercise authorization,
    /// not the sale pipeline, which has its own tests.
    fn seed_sale(
        connection: &Connection,
        business_id: &str,
        department_id: &str,
        terminal_id: &str,
        shift_id: &str,
        cashier_id: &str,
        sale_number: &str,
    ) -> String {
        let id = uuid::Uuid::now_v7().to_string();
        connection
            .execute(
                "INSERT INTO sales(id, business_id, department_id, terminal_id, shift_id,
                    cashier_id, sale_number, status, subtotal_minor, discount_minor, tax_minor,
                    total_minor, currency, completed_at, idempotency_key)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, 'COMPLETED', 1000, 0, 0, 1000, 'ZAR',
                    '2026-01-01T09:00:00.000Z', ?8)",
                (
                    &id,
                    business_id,
                    department_id,
                    terminal_id,
                    shift_id,
                    cashier_id,
                    sale_number,
                    &id,
                ),
            )
            .unwrap();
        id
    }
}
