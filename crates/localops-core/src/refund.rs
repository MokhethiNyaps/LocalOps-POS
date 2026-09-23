use std::collections::{BTreeMap, BTreeSet};

use rusqlite::{Connection, OptionalExtension, params};
use uuid::Uuid;

use crate::{CoreError, Result, audit, inventory, money};

#[derive(Debug, Clone, Copy)]
pub struct RefundItemInput<'a> {
    pub sale_item_id: &'a str,
    pub quantity_micros: i64,
    pub restore_stock: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct RefundPaymentInput<'a> {
    pub payment_id: &'a str,
    pub amount_minor: i64,
    pub reference: Option<&'a str>,
}

#[derive(Debug, Clone, Copy)]
pub struct CreateRefund<'a> {
    pub business_id: &'a str,
    pub sale_id: &'a str,
    pub idempotency_key: &'a str,
    pub refund_number: Option<&'a str>,
    pub shift_id: &'a str,
    pub terminal_id: &'a str,
    pub user_id: &'a str,
    pub reason: &'a str,
    pub occurred_at: &'a str,
    pub void_sale: bool,
    pub items: &'a [RefundItemInput<'a>],
    pub payments: &'a [RefundPaymentInput<'a>],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefundedItem {
    pub sale_item_id: String,
    pub description: String,
    pub quantity_micros: i64,
    pub amount_minor: i64,
    pub restore_stock: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefundedPayment {
    pub payment_id: String,
    pub method_name: String,
    pub method_kind: String,
    pub amount_minor: i64,
    pub reference: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletedRefund {
    pub id: String,
    pub refund_number: String,
    pub sale_id: String,
    pub reason: String,
    pub total_minor: i64,
    pub sale_status: String,
    pub occurred_at: String,
    pub items: Vec<RefundedItem>,
    pub payments: Vec<RefundedPayment>,
    pub idempotent_replay: bool,
}

#[derive(Debug)]
struct StockRestoration {
    refund_item_id: String,
    product_id: String,
    location_id: String,
    quantity_micros: i64,
}

pub fn create_refund(connection: &Connection, input: CreateRefund<'_>) -> Result<CompletedRefund> {
    if let Some(existing_id) = connection
        .query_row(
            "SELECT id FROM refunds WHERE business_id = ?1 AND idempotency_key = ?2",
            (input.business_id, input.idempotency_key),
            |row| row.get::<_, String>(0),
        )
        .optional()?
    {
        let mut existing = load_refund(connection, &existing_id)?;
        existing.idempotent_replay = true;
        return Ok(existing);
    }
    if input.items.is_empty() {
        return Err(CoreError::EmptyRefund);
    }
    if input.reason.trim().is_empty() {
        return Err(CoreError::EmptyRefundReason);
    }
    let item_keys = input
        .items
        .iter()
        .map(|item| item.sale_item_id)
        .collect::<BTreeSet<_>>();
    let payment_keys = input
        .payments
        .iter()
        .map(|payment| payment.payment_id)
        .collect::<BTreeSet<_>>();
    if item_keys.len() != input.items.len() || payment_keys.len() != input.payments.len() {
        return Err(CoreError::DuplicateRefundAllocation);
    }

    let transaction = connection.unchecked_transaction()?;
    let shift_valid = transaction
        .query_row(
            "SELECT 1 FROM shifts sh
             JOIN users u ON u.id = sh.user_id AND u.business_id = sh.business_id
             JOIN terminals t ON t.id = sh.terminal_id AND t.business_id = sh.business_id
             WHERE sh.id = ?2 AND sh.business_id = ?1 AND sh.terminal_id = ?3
               AND sh.user_id = ?4 AND sh.status = 'OPEN' AND u.active = 1 AND t.active = 1",
            (
                input.business_id,
                input.shift_id,
                input.terminal_id,
                input.user_id,
            ),
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if !shift_valid {
        return Err(CoreError::OpenShiftNotFound);
    }
    let (sale_status, sale_total): (String, i64) = transaction
        .query_row(
            "SELECT status, total_minor FROM sales WHERE id = ?2 AND business_id = ?1",
            (input.business_id, input.sale_id),
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?
        .ok_or(CoreError::SaleNotFound)?;
    if sale_status == "VOID" {
        return Err(CoreError::SaleAlreadyVoided);
    }
    if input.void_sale && sale_status != "COMPLETED" {
        return Err(CoreError::IncompleteVoid);
    }

    let refund_id = Uuid::now_v7().to_string();
    let refund_number = input
        .refund_number
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| {
            let prefix = if input.void_sale { "V" } else { "R" };
            format!("{prefix}-{}", &refund_id[..8])
        });
    let correlation_id = Uuid::now_v7().to_string();
    let mut total_minor = 0_i64;
    let mut prepared_items = Vec::with_capacity(input.items.len());
    let mut restorations = Vec::new();

    for item in input.items {
        if item.quantity_micros <= 0 {
            return Err(CoreError::RefundQuantityExceeded);
        }
        let (description, sold_quantity, line_total): (String, i64, i64) = transaction
            .query_row(
                "SELECT description, quantity_micros, line_total_minor
                 FROM sale_items WHERE id = ?1 AND sale_id = ?2",
                (item.sale_item_id, input.sale_id),
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?
            .ok_or(CoreError::SaleNotFound)?;
        let (already_quantity, already_amount): (i64, i64) = transaction.query_row(
            "SELECT COALESCE(SUM(ri.quantity_micros), 0), COALESCE(SUM(ri.amount_minor), 0)
             FROM refund_items ri
             JOIN refunds r ON r.id = ri.refund_id
             WHERE ri.sale_item_id = ?1 AND r.status = 'COMPLETED'",
            [item.sale_item_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let remaining_quantity = sold_quantity - already_quantity;
        if item.quantity_micros > remaining_quantity {
            return Err(CoreError::RefundQuantityExceeded);
        }
        let remaining_amount = line_total - already_amount;
        let amount_minor = if item.quantity_micros == remaining_quantity {
            remaining_amount
        } else {
            money::round_ratio(
                i128::from(line_total) * i128::from(item.quantity_micros),
                i128::from(sold_quantity),
            )?
            .min(remaining_amount)
        };
        if amount_minor <= 0 {
            return Err(CoreError::RefundQuantityExceeded);
        }
        let refund_item_id = Uuid::now_v7().to_string();
        if item.restore_stock {
            let mut statement = transaction.prepare(
                "SELECT product_id, location_id, quantity_micros
                 FROM sale_item_consumptions WHERE sale_item_id = ?1",
            )?;
            let consumptions = statement
                .query_map([item.sale_item_id], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                    ))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            drop(statement);
            for (product_id, location_id, consumed_quantity) in consumptions {
                let already_restored: i64 = transaction.query_row(
                    "SELECT COALESCE(SUM(ric.quantity_micros), 0)
                     FROM refund_item_consumptions ric
                     JOIN refund_items ri ON ri.id = ric.refund_item_id
                     JOIN refunds r ON r.id = ri.refund_id
                     WHERE ri.sale_item_id = ?1 AND ric.product_id = ?2
                       AND ric.location_id = ?3 AND r.status = 'COMPLETED'",
                    (item.sale_item_id, &product_id, &location_id),
                    |row| row.get(0),
                )?;
                let quantity_micros = money::round_ratio(
                    i128::from(consumed_quantity) * i128::from(item.quantity_micros),
                    i128::from(sold_quantity),
                )?
                .min(consumed_quantity - already_restored);
                if quantity_micros > 0 {
                    restorations.push(StockRestoration {
                        refund_item_id: refund_item_id.clone(),
                        product_id,
                        location_id,
                        quantity_micros,
                    });
                }
            }
        }
        total_minor = total_minor
            .checked_add(amount_minor)
            .ok_or(CoreError::MoneyOverflow)?;
        prepared_items.push((refund_item_id, item, description, amount_minor));
    }

    let mut payment_total = 0_i64;
    let mut cash_refund = 0_i64;
    let mut prepared_payments = Vec::with_capacity(input.payments.len());
    for payment in input.payments {
        if payment.amount_minor <= 0 {
            return Err(CoreError::RefundPaymentMismatch);
        }
        let (method_name, method_kind, original_amount): (String, String, i64) = transaction
            .query_row(
                "SELECT pm.name, pm.kind, p.amount_minor
                 FROM payments p JOIN payment_methods pm ON pm.id = p.method_id
                 WHERE p.id = ?1 AND p.sale_id = ?2",
                (payment.payment_id, input.sale_id),
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?
            .ok_or(CoreError::RefundPaymentExceeded)?;
        let already_refunded: i64 = transaction.query_row(
            "SELECT COALESCE(SUM(rp.amount_minor), 0)
             FROM refund_payments rp JOIN refunds r ON r.id = rp.refund_id
             WHERE rp.payment_id = ?1 AND r.status = 'COMPLETED'",
            [payment.payment_id],
            |row| row.get(0),
        )?;
        if payment.amount_minor > original_amount - already_refunded {
            return Err(CoreError::RefundPaymentExceeded);
        }
        payment_total = payment_total
            .checked_add(payment.amount_minor)
            .ok_or(CoreError::MoneyOverflow)?;
        if method_kind == "CASH" {
            cash_refund = cash_refund
                .checked_add(payment.amount_minor)
                .ok_or(CoreError::MoneyOverflow)?;
        }
        prepared_payments.push((payment, method_name, method_kind));
    }
    if payment_total != total_minor {
        return Err(CoreError::RefundPaymentMismatch);
    }
    let previous_refunds: i64 = transaction.query_row(
        "SELECT COALESCE(SUM(total_minor), 0) FROM refunds
         WHERE sale_id = ?1 AND status = 'COMPLETED'",
        [input.sale_id],
        |row| row.get(0),
    )?;
    let cumulative_refund = previous_refunds
        .checked_add(total_minor)
        .ok_or(CoreError::MoneyOverflow)?;
    if cumulative_refund > sale_total {
        return Err(CoreError::RefundQuantityExceeded);
    }
    if input.void_sale && cumulative_refund != sale_total {
        return Err(CoreError::IncompleteVoid);
    }

    transaction.execute(
        "INSERT INTO refunds(
             id, business_id, sale_id, refund_number, reason, total_minor, created_by,
             idempotency_key, shift_id, terminal_id, refunded_at
         ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            &refund_id,
            input.business_id,
            input.sale_id,
            &refund_number,
            input.reason.trim(),
            total_minor,
            input.user_id,
            input.idempotency_key,
            input.shift_id,
            input.terminal_id,
            input.occurred_at,
        ],
    )?;
    let mut receipt_items = Vec::with_capacity(prepared_items.len());
    for (refund_item_id, item, description, amount_minor) in prepared_items {
        transaction.execute(
            "INSERT INTO refund_items(
                 id, refund_id, sale_item_id, quantity_micros, amount_minor, restore_stock
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
            (
                &refund_item_id,
                &refund_id,
                item.sale_item_id,
                item.quantity_micros,
                amount_minor,
                item.restore_stock,
            ),
        )?;
        receipt_items.push(RefundedItem {
            sale_item_id: item.sale_item_id.to_owned(),
            description,
            quantity_micros: item.quantity_micros,
            amount_minor,
            restore_stock: item.restore_stock,
        });
    }
    let mut receipt_payments = Vec::with_capacity(prepared_payments.len());
    for (payment, method_name, method_kind) in prepared_payments {
        transaction.execute(
            "INSERT INTO refund_payments(id, refund_id, payment_id, amount_minor, reference)
             VALUES(?1, ?2, ?3, ?4, ?5)",
            (
                Uuid::now_v7().to_string(),
                &refund_id,
                payment.payment_id,
                payment.amount_minor,
                payment.reference,
            ),
        )?;
        receipt_payments.push(RefundedPayment {
            payment_id: payment.payment_id.to_owned(),
            method_name,
            method_kind,
            amount_minor: payment.amount_minor,
            reference: payment.reference.map(str::to_owned),
        });
    }

    let mut movement_totals: BTreeMap<(String, String), i64> = BTreeMap::new();
    for restoration in &restorations {
        *movement_totals
            .entry((
                restoration.product_id.clone(),
                restoration.location_id.clone(),
            ))
            .or_default() += restoration.quantity_micros;
    }
    let mut movement_ids = BTreeMap::new();
    for ((product_id, location_id), quantity_micros) in movement_totals {
        let movement = inventory::post_movement(
            &transaction,
            inventory::NewMovement {
                business_id: input.business_id,
                product_id: &product_id,
                location_id: &location_id,
                quantity_micros,
                movement_type: "ADJUSTMENT",
                occurred_at: input.occurred_at,
                user_id: input.user_id,
                terminal_id: Some(input.terminal_id),
                reference_type: "refund",
                reference_id: &refund_id,
                correlation_id: &correlation_id,
                notes: Some("Refund stock restoration"),
            },
        )?;
        movement_ids.insert((product_id, location_id), movement.movement_id);
    }
    for restoration in restorations {
        let movement_id = &movement_ids[&(
            restoration.product_id.clone(),
            restoration.location_id.clone(),
        )];
        transaction.execute(
            "INSERT INTO refund_item_consumptions(
                 id, refund_item_id, product_id, location_id, quantity_micros, movement_id
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
            (
                Uuid::now_v7().to_string(),
                restoration.refund_item_id,
                restoration.product_id,
                restoration.location_id,
                restoration.quantity_micros,
                movement_id,
            ),
        )?;
    }
    if cash_refund > 0 {
        let updated = transaction.execute(
            "UPDATE shifts SET expected_balance_minor = expected_balance_minor - ?2
             WHERE id = ?1 AND status = 'OPEN' AND expected_balance_minor >= ?2",
            (input.shift_id, cash_refund),
        )?;
        if updated != 1 {
            return Err(CoreError::RefundPaymentExceeded);
        }
    }
    let new_status = if input.void_sale {
        transaction.execute(
            "UPDATE sales SET status = 'VOID', voided_at = ?2, void_reason = ?3,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = ?1",
            (input.sale_id, input.occurred_at, input.reason.trim()),
        )?;
        transaction.execute(
            "UPDATE payments SET status = 'REVERSED' WHERE sale_id = ?1",
            [input.sale_id],
        )?;
        "VOID"
    } else if cumulative_refund == sale_total {
        transaction.execute(
            "UPDATE sales SET status = 'REFUNDED',
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = ?1",
            [input.sale_id],
        )?;
        "REFUNDED"
    } else {
        transaction.execute(
            "UPDATE sales SET status = 'PART_REFUNDED',
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = ?1",
            [input.sale_id],
        )?;
        "PART_REFUNDED"
    };
    let payload = format!(
        "{{\"saleId\":\"{}\",\"totalMinor\":{},\"status\":\"{}\"}}",
        input.sale_id, total_minor, new_status
    );
    audit::record_event(
        &transaction,
        audit::NewAuditEvent {
            business_id: input.business_id,
            user_id: Some(input.user_id),
            terminal_id: Some(input.terminal_id),
            action: if input.void_sale {
                "SALE_VOIDED"
            } else {
                "SALE_REFUNDED"
            },
            entity_type: "refund",
            entity_id: &refund_id,
            old_json: None,
            new_json: Some(&payload),
        },
    )?;
    transaction.commit()?;

    Ok(CompletedRefund {
        id: refund_id,
        refund_number,
        sale_id: input.sale_id.to_owned(),
        reason: input.reason.trim().to_owned(),
        total_minor,
        sale_status: new_status.to_owned(),
        occurred_at: input.occurred_at.to_owned(),
        items: receipt_items,
        payments: receipt_payments,
        idempotent_replay: false,
    })
}

pub fn load_refund(connection: &Connection, refund_id: &str) -> Result<CompletedRefund> {
    let header = connection
        .query_row(
            "SELECT r.refund_number, r.sale_id, r.reason, r.total_minor,
                    COALESCE(r.refunded_at, r.created_at), s.status
             FROM refunds r JOIN sales s ON s.id = r.sale_id
             WHERE r.id = ?1 AND r.status = 'COMPLETED'",
            [refund_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        )
        .optional()?
        .ok_or(CoreError::RefundNotFound)?;
    let mut item_statement = connection.prepare(
        "SELECT ri.sale_item_id, si.description, ri.quantity_micros,
                ri.amount_minor, ri.restore_stock
         FROM refund_items ri JOIN sale_items si ON si.id = ri.sale_item_id
         WHERE ri.refund_id = ?1 ORDER BY ri.created_at, ri.id",
    )?;
    let items = item_statement
        .query_map([refund_id], |row| {
            Ok(RefundedItem {
                sale_item_id: row.get(0)?,
                description: row.get(1)?,
                quantity_micros: row.get(2)?,
                amount_minor: row.get(3)?,
                restore_stock: row.get(4)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut payment_statement = connection.prepare(
        "SELECT rp.payment_id, pm.name, pm.kind, rp.amount_minor, rp.reference
         FROM refund_payments rp
         JOIN payments p ON p.id = rp.payment_id
         JOIN payment_methods pm ON pm.id = p.method_id
         WHERE rp.refund_id = ?1 ORDER BY rp.created_at, rp.id",
    )?;
    let payments = payment_statement
        .query_map([refund_id], |row| {
            Ok(RefundedPayment {
                payment_id: row.get(0)?,
                method_name: row.get(1)?,
                method_kind: row.get(2)?,
                amount_minor: row.get(3)?,
                reference: row.get(4)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(CompletedRefund {
        id: refund_id.to_owned(),
        refund_number: header.0,
        sale_id: header.1,
        reason: header.2,
        total_minor: header.3,
        occurred_at: header.4,
        sale_status: header.5,
        items,
        payments,
        idempotent_replay: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{availability, catalogue, inventory, payment, sales, setup, shift, unit};

    struct Fixture {
        database: Connection,
        business_id: String,
        location_id: String,
        terminal_id: String,
        user_id: String,
        shift_id: String,
        product_id: String,
        sale: sales::CompletedSale,
    }

    fn fixture() -> Fixture {
        let database = crate::open_memory_database().unwrap();
        let setup = setup::complete_initial_setup(
            &database,
            setup::InitialSetup {
                business_name: "Refund Business",
                department_name: "Main",
                location_name: "Store",
                terminal_name: "Till",
                owner_username: "owner",
                owner_display_name: "Owner",
                owner_pin: "1234",
            },
        )
        .unwrap();
        let each = unit::create_unit(
            &database,
            &setup.business_id,
            "EA",
            "Each",
            "COUNT",
            1,
            1,
            0,
        )
        .unwrap();
        let product_id = catalogue::create_product(
            &database,
            catalogue::NewProduct {
                business_id: &setup.business_id,
                name: "Returnable Product",
                price_minor: 2_500,
                category_id: None,
                taxable: false,
                base_unit_id: &each,
                cost_minor: 1_000,
                track_stock: true,
                minimum_quantity_micros: 0,
            },
        )
        .unwrap();
        availability::set_department_availability(
            &database,
            &setup.department_id,
            &product_id,
            None,
            true,
        )
        .unwrap();
        inventory::post_movement(
            &database,
            inventory::NewMovement {
                business_id: &setup.business_id,
                product_id: &product_id,
                location_id: &setup.location_id,
                quantity_micros: 10_000_000,
                movement_type: "OPENING",
                occurred_at: "2026-09-23T08:00:00.000Z",
                user_id: &setup.user_id,
                terminal_id: Some(&setup.terminal_id),
                reference_type: "opening",
                reference_id: "refund-test-opening",
                correlation_id: "refund-test-opening",
                notes: None,
            },
        )
        .unwrap();
        let methods = payment::ensure_default_methods(
            &database,
            &setup.business_id,
            &setup.user_id,
            Some(&setup.terminal_id),
        )
        .unwrap();
        let cash = methods.iter().find(|method| method.kind == "CASH").unwrap();
        let card = methods.iter().find(|method| method.kind == "CARD").unwrap();
        let shift_id = shift::open_shift(
            &database,
            &setup.business_id,
            &setup.terminal_id,
            &setup.user_id,
            1_000,
            "2026-09-23T08:00:00.000Z",
        )
        .unwrap()
        .id;
        let lines = [sales::SaleLineInput {
            sellable_id: &product_id,
            department_id: &setup.department_id,
            quantity_micros: 4_000_000,
            discount_minor: 0,
        }];
        let payments = [
            sales::PaymentInput {
                method_id: &cash.id,
                amount_minor: 4_000,
                tendered_minor: Some(4_000),
                reference: None,
            },
            sales::PaymentInput {
                method_id: &card.id,
                amount_minor: 6_000,
                tendered_minor: Some(6_000),
                reference: Some("CARD-SALE"),
            },
        ];
        let sale = sales::complete_sale(
            &database,
            sales::CompleteSale {
                business_id: &setup.business_id,
                idempotency_key: "refund-test-sale",
                sale_number: Some("SALE-REFUND-1"),
                terminal_id: &setup.terminal_id,
                shift_id: &shift_id,
                cashier_id: &setup.user_id,
                customer_id: None,
                completed_at: "2026-09-23T09:00:00.000Z",
                notes: None,
                lines: &lines,
                payments: &payments,
            },
        )
        .unwrap();
        Fixture {
            database,
            business_id: setup.business_id,
            location_id: setup.location_id,
            terminal_id: setup.terminal_id,
            user_id: setup.user_id,
            shift_id,
            product_id,
            sale,
        }
    }

    #[test]
    fn partially_refunds_split_payments_and_explicitly_restores_stock() {
        let fixture = fixture();
        let items = [RefundItemInput {
            sale_item_id: &fixture.sale.lines[0].id,
            quantity_micros: 1_000_000,
            restore_stock: true,
        }];
        let payments = [
            RefundPaymentInput {
                payment_id: &fixture.sale.payments[0].id,
                amount_minor: 2_000,
                reference: Some("CASH-RETURN"),
            },
            RefundPaymentInput {
                payment_id: &fixture.sale.payments[1].id,
                amount_minor: 500,
                reference: Some("CARD-RETURN"),
            },
        ];
        let input = CreateRefund {
            business_id: &fixture.business_id,
            sale_id: &fixture.sale.id,
            idempotency_key: "refund-key-1",
            refund_number: Some("REF-1"),
            shift_id: &fixture.shift_id,
            terminal_id: &fixture.terminal_id,
            user_id: &fixture.user_id,
            reason: "Customer return",
            occurred_at: "2026-09-23T10:00:00.000Z",
            void_sale: false,
            items: &items,
            payments: &payments,
        };
        let completed = create_refund(&fixture.database, input).unwrap();
        assert_eq!(completed.total_minor, 2_500);
        assert_eq!(completed.sale_status, "PART_REFUNDED");
        assert_eq!(completed.payments.len(), 2);
        assert_eq!(
            inventory::get_balance(
                &fixture.database,
                &fixture.business_id,
                &fixture.product_id,
                &fixture.location_id,
            )
            .unwrap(),
            7_000_000
        );
        let expected_cash: i64 = fixture
            .database
            .query_row(
                "SELECT expected_balance_minor FROM shifts WHERE id = ?1",
                [&fixture.shift_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(expected_cash, 3_000);
        let replay = create_refund(&fixture.database, input).unwrap();
        assert_eq!(replay.id, completed.id);
        assert!(replay.idempotent_replay);
    }

    #[test]
    fn rejects_over_refund_without_partial_records() {
        let fixture = fixture();
        let items = [RefundItemInput {
            sale_item_id: &fixture.sale.lines[0].id,
            quantity_micros: 5_000_000,
            restore_stock: true,
        }];
        let payments = [RefundPaymentInput {
            payment_id: &fixture.sale.payments[0].id,
            amount_minor: 4_000,
            reference: None,
        }];
        let result = create_refund(
            &fixture.database,
            CreateRefund {
                business_id: &fixture.business_id,
                sale_id: &fixture.sale.id,
                idempotency_key: "over-refund",
                refund_number: None,
                shift_id: &fixture.shift_id,
                terminal_id: &fixture.terminal_id,
                user_id: &fixture.user_id,
                reason: "Invalid",
                occurred_at: "2026-09-23T10:00:00.000Z",
                void_sale: false,
                items: &items,
                payments: &payments,
            },
        );
        assert!(matches!(result, Err(CoreError::RefundQuantityExceeded)));
        let count: i64 = fixture
            .database
            .query_row("SELECT COUNT(*) FROM refunds", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn void_reverses_full_sale_payments_stock_and_cash() {
        let fixture = fixture();
        let items = [RefundItemInput {
            sale_item_id: &fixture.sale.lines[0].id,
            quantity_micros: 4_000_000,
            restore_stock: true,
        }];
        let payments = [
            RefundPaymentInput {
                payment_id: &fixture.sale.payments[0].id,
                amount_minor: 4_000,
                reference: None,
            },
            RefundPaymentInput {
                payment_id: &fixture.sale.payments[1].id,
                amount_minor: 6_000,
                reference: Some("CARD-VOID"),
            },
        ];
        let completed = create_refund(
            &fixture.database,
            CreateRefund {
                business_id: &fixture.business_id,
                sale_id: &fixture.sale.id,
                idempotency_key: "void-key-1",
                refund_number: Some("VOID-1"),
                shift_id: &fixture.shift_id,
                terminal_id: &fixture.terminal_id,
                user_id: &fixture.user_id,
                reason: "Operator correction",
                occurred_at: "2026-09-23T10:00:00.000Z",
                void_sale: true,
                items: &items,
                payments: &payments,
            },
        )
        .unwrap();
        assert_eq!(completed.sale_status, "VOID");
        assert_eq!(
            inventory::get_balance(
                &fixture.database,
                &fixture.business_id,
                &fixture.product_id,
                &fixture.location_id,
            )
            .unwrap(),
            10_000_000
        );
        let reversed: i64 = fixture
            .database
            .query_row(
                "SELECT COUNT(*) FROM payments WHERE sale_id = ?1 AND status = 'REVERSED'",
                [&fixture.sale.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(reversed, 2);
        let expected_cash: i64 = fixture
            .database
            .query_row(
                "SELECT expected_balance_minor FROM shifts WHERE id = ?1",
                [&fixture.shift_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(expected_cash, 1_000);
    }
}
