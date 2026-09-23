use std::collections::BTreeMap;

use rusqlite::Connection;

use crate::{CoreError, Result, money};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReportTotals {
    pub gross_sales_minor: i64,
    pub refunds_minor: i64,
    pub net_sales_minor: i64,
    pub gross_profit_minor: i64,
    pub expenses_minor: i64,
    pub shift_variance_minor: i64,
    pub inventory_value_minor: i64,
    pub low_stock_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DepartmentSummary {
    pub department_id: String,
    pub department_name: String,
    pub gross_sales_minor: i64,
    pub refunds_minor: i64,
    pub net_sales_minor: i64,
    pub gross_profit_minor: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaymentSummary {
    pub method_name: String,
    pub method_kind: String,
    pub received_minor: i64,
    pub refunded_minor: i64,
    pub net_minor: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StockSummary {
    pub product_name: String,
    pub location_name: String,
    pub quantity_micros: i64,
    pub minimum_quantity_micros: i64,
    pub value_minor: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DashboardReport {
    pub start_at: String,
    pub end_at: String,
    pub totals: ReportTotals,
    pub departments: Vec<DepartmentSummary>,
    pub payments: Vec<PaymentSummary>,
    pub stock: Vec<StockSummary>,
}

#[derive(Default)]
struct DepartmentAccumulator {
    name: String,
    gross_sales: i64,
    refunds: i64,
    gross_profit: i64,
}

#[derive(Default)]
struct PaymentAccumulator {
    kind: String,
    received: i64,
    refunded: i64,
}

pub fn build_dashboard(
    connection: &Connection,
    business_id: &str,
    start_at: &str,
    end_at: &str,
) -> Result<DashboardReport> {
    let mut totals = ReportTotals::default();
    let mut departments: BTreeMap<String, DepartmentAccumulator> = BTreeMap::new();
    let mut sales_statement = connection.prepare(
        "SELECT si.department_id, d.name, si.line_total_minor, si.cost_minor
         FROM sale_items si
         JOIN sales s ON s.id = si.sale_id
         JOIN departments d ON d.id = si.department_id
         WHERE s.business_id = ?1 AND s.completed_at >= ?2 AND s.completed_at <= ?3
           AND s.status != 'DRAFT'",
    )?;
    for row in sales_statement.query_map((business_id, start_at, end_at), |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, i64>(3)?,
        ))
    })? {
        let (department_id, name, revenue, cost) = row?;
        totals.gross_sales_minor = checked_add(totals.gross_sales_minor, revenue)?;
        totals.gross_profit_minor = checked_add(totals.gross_profit_minor, revenue - cost)?;
        let entry = departments.entry(department_id).or_default();
        entry.name = name;
        entry.gross_sales = checked_add(entry.gross_sales, revenue)?;
        entry.gross_profit = checked_add(entry.gross_profit, revenue - cost)?;
    }

    let mut refund_statement = connection.prepare(
        "SELECT si.department_id, d.name, ri.amount_minor, ri.quantity_micros,
                ri.restore_stock, si.cost_minor, si.quantity_micros
         FROM refund_items ri
         JOIN refunds r ON r.id = ri.refund_id
         JOIN sales s ON s.id = r.sale_id
         JOIN sale_items si ON si.id = ri.sale_item_id
         JOIN departments d ON d.id = si.department_id
         WHERE r.business_id = ?1 AND COALESCE(r.refunded_at, r.created_at) >= ?2
           AND COALESCE(r.refunded_at, r.created_at) <= ?3 AND r.status = 'COMPLETED'",
    )?;
    for row in refund_statement.query_map((business_id, start_at, end_at), |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, bool>(4)?,
            row.get::<_, i64>(5)?,
            row.get::<_, i64>(6)?,
        ))
    })? {
        let (department_id, name, amount, quantity, restored, line_cost, sold_quantity) = row?;
        let restored_cost = if restored {
            money::round_ratio(
                i128::from(line_cost) * i128::from(quantity),
                i128::from(sold_quantity),
            )?
        } else {
            0
        };
        totals.refunds_minor = checked_add(totals.refunds_minor, amount)?;
        totals.gross_profit_minor =
            checked_add(totals.gross_profit_minor, -amount + restored_cost)?;
        let entry = departments.entry(department_id).or_default();
        entry.name = name;
        entry.refunds = checked_add(entry.refunds, amount)?;
        entry.gross_profit = checked_add(entry.gross_profit, -amount + restored_cost)?;
    }
    totals.net_sales_minor = totals
        .gross_sales_minor
        .checked_sub(totals.refunds_minor)
        .ok_or(CoreError::MoneyOverflow)?;
    totals.expenses_minor = connection.query_row(
        "SELECT COALESCE(SUM(amount_minor), 0) FROM expenses
         WHERE business_id = ?1 AND status = 'RECORDED' AND expense_date >= ?2 AND expense_date <= ?3",
        (business_id, date_part(start_at), date_part(end_at)),
        |row| row.get(0),
    )?;
    totals.shift_variance_minor = connection.query_row(
        "SELECT COALESCE(SUM(variance_minor), 0) FROM shifts
         WHERE business_id = ?1 AND status = 'CLOSED' AND closed_at >= ?2 AND closed_at <= ?3",
        (business_id, start_at, end_at),
        |row| row.get(0),
    )?;

    let mut payments: BTreeMap<String, PaymentAccumulator> = BTreeMap::new();
    let mut received_statement = connection.prepare(
        "SELECT pm.name, pm.kind, COALESCE(SUM(p.amount_minor), 0)
         FROM payments p JOIN payment_methods pm ON pm.id = p.method_id
         JOIN sales s ON s.id = p.sale_id
         WHERE p.business_id = ?1 AND s.completed_at >= ?2 AND s.completed_at <= ?3
         GROUP BY pm.id, pm.name, pm.kind",
    )?;
    for row in received_statement.query_map((business_id, start_at, end_at), |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
        ))
    })? {
        let (name, kind, amount) = row?;
        payments.insert(
            name,
            PaymentAccumulator {
                kind,
                received: amount,
                refunded: 0,
            },
        );
    }
    let mut reversed_statement = connection.prepare(
        "SELECT pm.name, pm.kind, COALESCE(SUM(rp.amount_minor), 0)
         FROM refund_payments rp
         JOIN refunds r ON r.id = rp.refund_id
         JOIN payments p ON p.id = rp.payment_id
         JOIN payment_methods pm ON pm.id = p.method_id
         WHERE r.business_id = ?1 AND r.status = 'COMPLETED'
           AND COALESCE(r.refunded_at, r.created_at) >= ?2
           AND COALESCE(r.refunded_at, r.created_at) <= ?3
         GROUP BY pm.id, pm.name, pm.kind",
    )?;
    for row in reversed_statement.query_map((business_id, start_at, end_at), |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
        ))
    })? {
        let (name, kind, amount) = row?;
        let entry = payments.entry(name).or_default();
        entry.kind = kind;
        entry.refunded = amount;
    }

    let mut stock = Vec::new();
    let mut stock_statement = connection.prepare(
        "SELECT s.name, l.name, b.quantity_micros, p.minimum_quantity_micros, p.cost_minor
         FROM inventory_balances b
         JOIN products p ON p.sellable_id = b.product_id
         JOIN sellable_items s ON s.id = p.sellable_id
         JOIN locations l ON l.id = b.location_id
         WHERE b.business_id = ?1 ORDER BY s.name, l.name",
    )?;
    for row in stock_statement.query_map([business_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, i64>(4)?,
        ))
    })? {
        let (product_name, location_name, quantity, minimum, cost) = row?;
        let value = money::multiply_minor_by_quantity(cost, quantity)?;
        totals.inventory_value_minor = checked_add(totals.inventory_value_minor, value)?;
        if quantity < minimum {
            totals.low_stock_count += 1;
        }
        stock.push(StockSummary {
            product_name,
            location_name,
            quantity_micros: quantity,
            minimum_quantity_micros: minimum,
            value_minor: value,
        });
    }
    Ok(DashboardReport {
        start_at: start_at.to_owned(),
        end_at: end_at.to_owned(),
        totals,
        departments: departments
            .into_iter()
            .map(|(department_id, value)| DepartmentSummary {
                department_id,
                department_name: value.name,
                gross_sales_minor: value.gross_sales,
                refunds_minor: value.refunds,
                net_sales_minor: value.gross_sales - value.refunds,
                gross_profit_minor: value.gross_profit,
            })
            .collect(),
        payments: payments
            .into_iter()
            .map(|(method_name, value)| PaymentSummary {
                method_name,
                method_kind: value.kind,
                received_minor: value.received,
                refunded_minor: value.refunded,
                net_minor: value.received - value.refunded,
            })
            .collect(),
        stock,
    })
}

fn checked_add(left: i64, right: i64) -> Result<i64> {
    left.checked_add(right).ok_or(CoreError::MoneyOverflow)
}

fn date_part(value: &str) -> &str {
    value.get(..10).unwrap_or(value)
}

pub fn to_csv(report: &DashboardReport) -> String {
    let mut csv = String::from("section,name,gross,refunds,net,gross_profit\n");
    csv.push_str(&format!(
        "business,All departments,{},{},{},{}\n",
        report.totals.gross_sales_minor,
        report.totals.refunds_minor,
        report.totals.net_sales_minor,
        report.totals.gross_profit_minor
    ));
    for department in &report.departments {
        csv.push_str(&format!(
            "department,\"{}\",{},{},{},{}\n",
            department.department_name.replace('"', "\"\""),
            department.gross_sales_minor,
            department.refunds_minor,
            department.net_sales_minor,
            department.gross_profit_minor
        ));
    }
    csv.push_str("\nsection,name,received,refunded,net\n");
    for payment in &report.payments {
        csv.push_str(&format!(
            "payment,\"{}\",{},{},{}\n",
            payment.method_name.replace('"', "\"\""),
            payment.received_minor,
            payment.refunded_minor,
            payment.net_minor
        ));
    }
    csv
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setup;

    #[test]
    fn empty_dashboard_is_stable_and_exports_headers() {
        let database = crate::open_memory_database().unwrap();
        let setup = setup::complete_initial_setup(
            &database,
            setup::InitialSetup {
                business_name: "Report Business",
                department_name: "Main",
                location_name: "Store",
                terminal_name: "Till",
                owner_username: "owner",
                owner_display_name: "Owner",
                owner_pin: "1234",
            },
        )
        .unwrap();
        let report = build_dashboard(
            &database,
            &setup.business_id,
            "2026-09-01T00:00:00.000Z",
            "2026-09-30T23:59:59.999Z",
        )
        .unwrap();
        assert_eq!(report.totals, ReportTotals::default());
        assert!(to_csv(&report).contains("business,All departments"));
    }
}
