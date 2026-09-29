//! Cashier-safe application services.
//!
//! Every function here derives its scope from the authenticated
//! [`AccessContext`] and returns a deliberately narrow DTO. Sensitive Owner
//! fields (cost, margin, valuation, expected cash, variance, other employees'
//! records) are never part of these structures, so they cannot leak by being
//! serialized and hidden in the UI.

use rusqlite::Connection;
use serde::Serialize;

use crate::{CoreError, Result, access::AccessContext, shift};

/// A sellable item as the till may see it. There is deliberately no cost,
/// margin, valuation or procurement field on this type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CashierSellableItem {
    pub id: String,
    pub department_id: String,
    pub department_name: String,
    pub name: String,
    pub kind: String,
    pub price_minor: i64,
    pub taxable: bool,
    pub sku: Option<String>,
    pub product_code: Option<String>,
    pub barcode: Option<String>,
    pub category_name: Option<String>,
}

const SELLABLE_COLUMNS: &str = "SELECT s.id, d.id, d.name, s.name, s.kind,
            COALESCE(ds.price_override_minor, s.price_minor), s.taxable,
            s.sku, s.product_code, s.barcode, c.name
     FROM department_sellables ds
     JOIN departments d ON d.id = ds.department_id
     JOIN sellable_items s ON s.id = ds.sellable_id
     LEFT JOIN categories c ON c.id = s.category_id
     WHERE d.business_id = ?1 AND d.active = 1 AND s.active = 1 AND ds.active = 1";

fn map_item(row: &rusqlite::Row<'_>) -> rusqlite::Result<CashierSellableItem> {
    Ok(CashierSellableItem {
        id: row.get(0)?,
        department_id: row.get(1)?,
        department_name: row.get(2)?,
        name: row.get(3)?,
        kind: row.get(4)?,
        price_minor: row.get(5)?,
        taxable: row.get(6)?,
        sku: row.get(7)?,
        product_code: row.get(8)?,
        barcode: row.get(9)?,
        category_name: row.get(10)?,
    })
}

/// The POS catalogue for the caller.
///
/// A default Cashier receives only sellables available to the department of the
/// authenticated session terminal. A catalogue manager (`products.manage`) may
/// see the full business catalogue for the Owner POS workflow.
pub fn pos_catalogue(
    connection: &Connection,
    context: &AccessContext,
) -> Result<Vec<CashierSellableItem>> {
    context.require_permission("pos.catalog.view")?;
    if context.has_permission("products.manage") {
        let sql = format!("{SELLABLE_COLUMNS} ORDER BY d.name, s.name");
        let mut statement = connection.prepare(&sql)?;
        let rows = statement
            .query_map([&context.business_id], map_item)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        return Ok(rows);
    }
    let department_id = context.require_department()?;
    let sql = format!("{SELLABLE_COLUMNS} AND d.id = ?2 ORDER BY s.name");
    let mut statement = connection.prepare(&sql)?;
    let rows = statement
        .query_map((&context.business_id, department_id), map_item)?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Operational view of the caller's own current shift. Expected cash and
/// variance are only present when the caller separately holds the management
/// permissions for those fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnShiftView {
    pub id: String,
    pub terminal_id: String,
    pub terminal_name: String,
    pub status: String,
    pub opening_balance_minor: i64,
    pub opened_at: String,
    pub expected_balance_minor: Option<i64>,
    pub variance_minor: Option<i64>,
}

pub fn own_shift_view(context: &AccessContext, value: &shift::Shift) -> OwnShiftView {
    OwnShiftView {
        id: value.id.clone(),
        terminal_id: value.terminal_id.clone(),
        terminal_name: context.terminal_name.clone(),
        status: value.status.clone(),
        opening_balance_minor: value.opening_balance_minor,
        opened_at: value.opened_at.clone(),
        expected_balance_minor: context
            .has_permission("shifts.expected_cash.view")
            .then_some(value.expected_balance_minor),
        variance_minor: context
            .has_permission("shifts.variance.view")
            .then_some(value.variance_minor)
            .flatten(),
    }
}

/// The caller's own current open shift, or `None` when they have not opened one
/// on this terminal. Another employee's open shift is never returned.
pub fn get_own_current_shift(
    connection: &Connection,
    context: &AccessContext,
) -> Result<Option<OwnShiftView>> {
    context.require_permission("shifts.view_own_current")?;
    Ok(crate::access::current_own_shift(connection, context)?
        .map(|value| own_shift_view(context, &value)))
}

/// Open the caller's own shift on the authenticated terminal.
pub fn open_own_shift(
    connection: &Connection,
    context: &AccessContext,
    opening_balance_minor: i64,
    opened_at: &str,
) -> Result<OwnShiftView> {
    context.require_permission("shifts.open_own")?;
    let opened = shift::open_own_shift(
        connection,
        &context.business_id,
        &context.terminal_id,
        &context.user_id,
        opening_balance_minor,
        opened_at,
    )?;
    Ok(own_shift_view(context, &opened))
}

/// Result of a blind shift close.
///
/// The Cashier submits a counted amount and is told only what they submitted.
/// Expected cash and variance are persisted but are withheld from the response
/// unless the caller holds the corresponding management permissions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShiftCloseResult {
    pub shift_id: String,
    pub status: String,
    pub closed_at: String,
    pub actual_cash_submitted_minor: i64,
    pub shift_owner_user_id: String,
    pub closed_by_user_id: String,
    pub expected_balance_minor: Option<i64>,
    pub variance_minor: Option<i64>,
}

fn close_result(context: &AccessContext, closed: shift::Shift) -> ShiftCloseResult {
    ShiftCloseResult {
        shift_id: closed.id,
        status: closed.status,
        closed_at: closed.closed_at.unwrap_or_default(),
        actual_cash_submitted_minor: closed.actual_balance_minor.unwrap_or_default(),
        shift_owner_user_id: closed.user_id,
        closed_by_user_id: closed.closed_by.unwrap_or_else(|| context.user_id.clone()),
        expected_balance_minor: context
            .has_permission("shifts.expected_cash.view")
            .then_some(closed.expected_balance_minor),
        variance_minor: context
            .has_permission("shifts.variance.view")
            .then_some(closed.variance_minor)
            .flatten(),
    }
}

/// Blind close of the caller's own open shift.
pub fn close_own_shift(
    connection: &Connection,
    context: &AccessContext,
    actual_balance_minor: i64,
    closed_at: &str,
    notes: Option<&str>,
) -> Result<ShiftCloseResult> {
    context.require_permission("shifts.close_own")?;
    let open = crate::access::require_current_own_shift(connection, context)?;
    let closed = shift::close_shift(
        connection,
        &context.business_id,
        &open.id,
        &context.user_id,
        actual_balance_minor,
        closed_at,
        notes,
    )?;
    Ok(close_result(context, closed))
}

/// Supervisory close of any open shift in the business.
///
/// The original shift owner is preserved; `closed_by` records the authenticated
/// supervisor, and the full reconciliation is returned when authorized.
pub fn close_any_shift(
    connection: &Connection,
    context: &AccessContext,
    shift_id: &str,
    actual_balance_minor: i64,
    closed_at: &str,
    notes: Option<&str>,
) -> Result<ShiftCloseResult> {
    context.require_permission("shifts.close_any")?;
    let target =
        shift::get_shift(connection, &context.business_id, shift_id)?.ok_or(CoreError::ScopeDenied)?;
    let closed = shift::close_shift(
        connection,
        &context.business_id,
        &target.id,
        &context.user_id,
        actual_balance_minor,
        closed_at,
        notes,
    )?;
    Ok(close_result(context, closed))
}

/// A sale row as the till may see it: no cost, profit, margin or valuation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CashierSaleSummary {
    pub id: String,
    pub sale_number: String,
    pub status: String,
    pub total_minor: i64,
    pub completed_at: String,
}

fn map_sale(row: &rusqlite::Row<'_>) -> rusqlite::Result<CashierSaleSummary> {
    Ok(CashierSaleSummary {
        id: row.get(0)?,
        sale_number: row.get(1)?,
        status: row.get(2)?,
        total_minor: row.get(3)?,
        completed_at: row.get(4)?,
    })
}

/// Recent sales for the caller.
///
/// `sales.view_all` returns the business-wide list used by Owner workflows.
/// Otherwise the caller must hold `sales.view_own_current_shift` and receives
/// only the sales they created during their own current open shift on the
/// session terminal. With no own open shift the list is empty by design.
pub fn recent_sales(
    connection: &Connection,
    context: &AccessContext,
    limit: i64,
) -> Result<Vec<CashierSaleSummary>> {
    if context.has_permission("sales.view_all") {
        let mut statement = connection.prepare(
            "SELECT id, sale_number, status, total_minor, COALESCE(completed_at, created_at)
             FROM sales WHERE business_id = ?1
             ORDER BY COALESCE(completed_at, created_at) DESC, created_at DESC
             LIMIT ?2",
        )?;
        let rows = statement
            .query_map((&context.business_id, limit), map_sale)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        return Ok(rows);
    }
    context.require_permission("sales.view_own_current_shift")?;
    let Some(open) = crate::access::current_own_shift(connection, context)? else {
        return Ok(Vec::new());
    };
    let mut statement = connection.prepare(
        "SELECT id, sale_number, status, total_minor, COALESCE(completed_at, created_at)
         FROM sales
         WHERE business_id = ?1 AND terminal_id = ?2 AND cashier_id = ?3 AND shift_id = ?4
         ORDER BY COALESCE(completed_at, created_at) DESC, created_at DESC
         LIMIT ?5",
    )?;
    let rows = statement
        .query_map(
            (
                &context.business_id,
                &context.terminal_id,
                &context.user_id,
                &open.id,
                limit,
            ),
            map_sale,
        )?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{access, availability, catalogue, open_memory_database, sales, unit};

    fn seed_item(
        connection: &Connection,
        business_id: &str,
        department_id: &str,
        name: &str,
        price_minor: i64,
    ) -> String {
        let unit_id = unit::create_unit(
            connection,
            business_id,
            &format!("EA-{name}"),
            "Each",
            "COUNT",
            1,
            1,
            0,
        )
        .unwrap();
        let sellable_id = catalogue::create_product(
            connection,
            catalogue::NewProduct {
                business_id,
                name,
                price_minor,
                category_id: None,
                taxable: true,
                base_unit_id: &unit_id,
                cost_minor: price_minor / 2,
                track_stock: false,
                minimum_quantity_micros: 0,
            },
        )
        .unwrap();
        availability::set_department_availability(
            connection,
            department_id,
            &sellable_id,
            None,
            true,
        )
        .unwrap();
        sellable_id
    }

    #[test]
    fn cashier_catalogue_is_limited_to_the_session_terminal_department() {
        let database = open_memory_database().unwrap();
        let fixture = access::fixtures::business(&database);
        let (restaurant_department, _restaurant_terminal) = access::fixtures::second_terminal(
            &database,
            &fixture.business_id,
            "Restaurant",
            "Restaurant Till",
        )
        .unwrap();
        seed_item(
            &database,
            &fixture.business_id,
            &fixture.department_id,
            "Castle Lager",
            2_500,
        );
        seed_item(
            &database,
            &fixture.business_id,
            &restaurant_department,
            "Beef Burger",
            9_500,
        );
        let (_, session_id) = access::fixtures::cashier(
            &database,
            &fixture.business_id,
            &fixture.terminal_id,
            "thabo",
        )
        .unwrap();
        let context = access::resolve(&database, &session_id).unwrap();
        let items = pos_catalogue(&database, &context).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "Castle Lager");

        let owner = access::resolve(&database, &fixture.owner_session).unwrap();
        assert_eq!(pos_catalogue(&database, &owner).unwrap().len(), 2);
    }

    #[test]
    fn cashier_sellable_payload_never_serializes_sensitive_fields() {
        let item = CashierSellableItem {
            id: "s1".into(),
            department_id: "d1".into(),
            department_name: "Bar".into(),
            name: "Castle Lager".into(),
            kind: "PRODUCT".into(),
            price_minor: 2_500,
            taxable: true,
            sku: None,
            product_code: None,
            barcode: None,
            category_name: None,
        };
        let json = format!("{:?}", item);
        for forbidden in ["cost", "margin", "profit", "valuation", "supplier"] {
            assert!(!json.to_lowercase().contains(forbidden));
        }
    }

    #[test]
    fn blind_close_hides_expected_and_variance_from_the_cashier() {
        let database = open_memory_database().unwrap();
        let fixture = access::fixtures::business(&database);
        let (_, session_id) = access::fixtures::cashier(
            &database,
            &fixture.business_id,
            &fixture.terminal_id,
            "thabo",
        )
        .unwrap();
        let context = access::resolve(&database, &session_id).unwrap();
        open_own_shift(&database, &context, 5_030, "2026-09-23T08:00:00.000Z").unwrap();
        let closed = close_own_shift(
            &database,
            &context,
            4_820,
            "2026-09-23T18:00:00.000Z",
            Some("Counted twice"),
        )
        .unwrap();
        assert_eq!(closed.status, "CLOSED");
        assert_eq!(closed.actual_cash_submitted_minor, 4_820);
        assert!(closed.expected_balance_minor.is_none());
        assert!(closed.variance_minor.is_none());

        // The reconciliation is still persisted for the Owner.
        let stored = shift::get_shift(&database, &fixture.business_id, &closed.shift_id)
            .unwrap()
            .unwrap();
        assert_eq!(stored.expected_balance_minor, 5_030);
        assert_eq!(stored.actual_balance_minor, Some(4_820));
        assert_eq!(stored.variance_minor, Some(-210));
    }

    #[test]
    fn owner_can_close_an_abandoned_cashier_shift_and_see_reconciliation() {
        let database = open_memory_database().unwrap();
        let fixture = access::fixtures::business(&database);
        let (cashier_id, cashier_session) = access::fixtures::cashier(
            &database,
            &fixture.business_id,
            &fixture.terminal_id,
            "thabo",
        )
        .unwrap();
        let cashier = access::resolve(&database, &cashier_session).unwrap();
        let opened = open_own_shift(&database, &cashier, 1_000, "2026-09-23T08:00:00.000Z").unwrap();
        let owner = access::resolve(&database, &fixture.owner_session).unwrap();

        assert!(matches!(
            close_any_shift(
                &database,
                &cashier,
                &opened.id,
                900,
                "2026-09-23T18:00:00.000Z",
                None
            ),
            Err(CoreError::PermissionDenied(_))
        ));

        let closed = close_any_shift(
            &database,
            &owner,
            &opened.id,
            900,
            "2026-09-23T18:00:00.000Z",
            None,
        )
        .unwrap();
        assert_eq!(closed.shift_owner_user_id, cashier_id);
        assert_eq!(closed.closed_by_user_id, fixture.owner_id);
        assert_eq!(closed.expected_balance_minor, Some(1_000));
        assert_eq!(closed.variance_minor, Some(-100));
    }

    #[test]
    fn recent_sales_are_scoped_to_the_cashiers_own_current_shift() {
        let database = open_memory_database().unwrap();
        let fixture = access::fixtures::business(&database);
        let sellable = seed_item(
            &database,
            &fixture.business_id,
            &fixture.department_id,
            "Castle Lager",
            2_500,
        );
        let (_, cashier_session) = access::fixtures::cashier(
            &database,
            &fixture.business_id,
            &fixture.terminal_id,
            "thabo",
        )
        .unwrap();
        let cashier = access::resolve(&database, &cashier_session).unwrap();
        let opened = open_own_shift(&database, &cashier, 0, "2026-09-23T08:00:00.000Z").unwrap();
        let method = crate::payment::ensure_default_methods(
            &database,
            &fixture.business_id,
            &fixture.owner_id,
            None,
        )
        .unwrap()
        .into_iter()
        .find(|value| value.kind == "CASH")
        .unwrap();
        sales::complete_sale(
            &database,
            sales::CompleteSale {
                business_id: &fixture.business_id,
                idempotency_key: "sale-1",
                sale_number: None,
                terminal_id: &fixture.terminal_id,
                shift_id: &opened.id,
                cashier_id: &cashier.user_id,
                customer_id: None,
                completed_at: "2026-09-23T09:00:00.000Z",
                notes: None,
                lines: &[sales::SaleLineInput {
                    sellable_id: &sellable,
                    department_id: &fixture.department_id,
                    quantity_micros: 1_000_000,
                    discount_minor: 0,
                }],
                payments: &[sales::PaymentInput {
                    method_id: &method.id,
                    amount_minor: 2_500,
                    tendered_minor: Some(2_500),
                    reference: None,
                }],
            },
        )
        .unwrap();

        let own = recent_sales(&database, &cashier, 25).unwrap();
        assert_eq!(own.len(), 1);

        // After the shift closes there is no current open shift, so the default
        // Cashier scope yields nothing.
        close_own_shift(&database, &cashier, 2_500, "2026-09-23T18:00:00.000Z", None).unwrap();
        assert!(recent_sales(&database, &cashier, 25).unwrap().is_empty());

        // The Owner keeps historical access.
        let owner = access::resolve(&database, &fixture.owner_session).unwrap();
        assert_eq!(recent_sales(&database, &owner, 25).unwrap().len(), 1);
    }
}
