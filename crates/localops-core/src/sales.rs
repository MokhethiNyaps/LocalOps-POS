use std::collections::{BTreeMap, BTreeSet};

use rusqlite::{Connection, OptionalExtension, params};
use uuid::Uuid;

use crate::{CoreError, Result, audit, inventory, money, recipe};

#[derive(Debug, Clone, Copy)]
pub struct SaleLineInput<'a> {
    pub sellable_id: &'a str,
    pub department_id: &'a str,
    pub quantity_micros: i64,
    pub discount_minor: i64,
}

#[derive(Debug, Clone, Copy)]
pub struct PaymentInput<'a> {
    pub method_id: &'a str,
    pub amount_minor: i64,
    pub tendered_minor: Option<i64>,
    pub reference: Option<&'a str>,
}

#[derive(Debug, Clone, Copy)]
pub struct CompleteSale<'a> {
    pub business_id: &'a str,
    pub idempotency_key: &'a str,
    pub sale_number: Option<&'a str>,
    pub terminal_id: &'a str,
    pub shift_id: &'a str,
    pub cashier_id: &'a str,
    pub customer_id: Option<&'a str>,
    pub completed_at: &'a str,
    pub notes: Option<&'a str>,
    pub lines: &'a [SaleLineInput<'a>],
    pub payments: &'a [PaymentInput<'a>],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiptLine {
    pub id: String,
    pub description: String,
    pub kind: String,
    pub quantity_micros: i64,
    pub unit_price_minor: i64,
    pub discount_minor: i64,
    pub tax_minor: i64,
    pub line_total_minor: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiptPayment {
    pub id: String,
    pub method_name: String,
    pub method_kind: String,
    pub amount_minor: i64,
    pub tendered_minor: Option<i64>,
    pub change_minor: i64,
    pub reference: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletedSale {
    pub id: String,
    pub sale_number: String,
    pub currency: String,
    pub subtotal_minor: i64,
    pub discount_minor: i64,
    pub tax_minor: i64,
    pub total_minor: i64,
    pub amount_paid_minor: i64,
    pub change_due_minor: i64,
    pub completed_at: String,
    pub lines: Vec<ReceiptLine>,
    pub payments: Vec<ReceiptPayment>,
    pub idempotent_replay: bool,
}

#[derive(Debug)]
struct PreparedLine {
    id: String,
    sellable_id: String,
    department_id: String,
    description: String,
    kind: String,
    quantity_micros: i64,
    unit_price_minor: i64,
    discount_minor: i64,
    tax_rate_ppm: i64,
    tax_minor: i64,
    line_total_minor: i64,
    cost_minor: i64,
    consumptions: Vec<PreparedConsumption>,
}

#[derive(Debug)]
struct PreparedConsumption {
    product_id: String,
    location_id: String,
    quantity_micros: i64,
}

pub fn complete_sale(connection: &Connection, input: CompleteSale<'_>) -> Result<CompletedSale> {
    if let Some(existing_id) = connection
        .query_row(
            "SELECT id FROM sales WHERE business_id = ?1 AND idempotency_key = ?2",
            (input.business_id, input.idempotency_key),
            |row| row.get::<_, String>(0),
        )
        .optional()?
    {
        let mut existing = load_completed_sale(connection, &existing_id)?;
        existing.idempotent_replay = true;
        return Ok(existing);
    }
    if input.lines.is_empty() {
        return Err(CoreError::EmptySale);
    }
    let line_keys = input
        .lines
        .iter()
        .map(|line| (line.sellable_id, line.department_id))
        .collect::<BTreeSet<_>>();
    if line_keys.len() != input.lines.len() {
        return Err(CoreError::DuplicateSaleItem);
    }
    let transaction = connection.unchecked_transaction()?;
    let (currency, tax_enabled, business_tax_rate_ppm, prices_include_tax): (
        String,
        bool,
        i64,
        bool,
    ) = transaction
        .query_row(
            "SELECT currency, tax_enabled, tax_rate_ppm, prices_include_tax
             FROM businesses WHERE id = ?1 AND active = 1",
            [input.business_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()?
        .ok_or(CoreError::BusinessNotFound)?;
    let shift_valid = transaction
        .query_row(
            "SELECT 1 FROM shifts sh
             JOIN terminals t ON t.id = sh.terminal_id AND t.business_id = sh.business_id
             JOIN users u ON u.id = sh.user_id AND u.business_id = sh.business_id
             WHERE sh.id = ?2 AND sh.business_id = ?1 AND sh.terminal_id = ?3
               AND sh.user_id = ?4 AND sh.status = 'OPEN'
               AND t.active = 1 AND u.active = 1",
            (
                input.business_id,
                input.shift_id,
                input.terminal_id,
                input.cashier_id,
            ),
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if !shift_valid {
        return Err(CoreError::OpenShiftNotFound);
    }

    let sale_id = Uuid::now_v7().to_string();
    let sale_number = input
        .sale_number
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| format!("S-{}", &sale_id[..8]));
    let correlation_id = Uuid::now_v7().to_string();
    let mut prepared = Vec::with_capacity(input.lines.len());
    let mut effects: BTreeMap<(String, String, String), i64> = BTreeMap::new();
    let mut subtotal_minor = 0_i64;
    let mut discount_minor = 0_i64;
    let mut tax_minor = 0_i64;
    let mut total_minor = 0_i64;

    for line in input.lines {
        if line.quantity_micros <= 0 || line.discount_minor < 0 {
            return Err(CoreError::InvalidSaleDiscount);
        }
        let available = transaction
            .query_row(
                "SELECT s.name, s.kind, COALESCE(ds.price_override_minor, s.price_minor),
                        s.taxable, p.cost_minor, p.track_stock
                 FROM department_sellables ds
                 JOIN departments d ON d.id = ds.department_id
                 JOIN sellable_items s ON s.id = ds.sellable_id AND s.business_id = d.business_id
                 LEFT JOIN products p ON p.sellable_id = s.id
                 WHERE ds.department_id = ?2 AND ds.sellable_id = ?3
                   AND d.business_id = ?1 AND d.active = 1 AND s.active = 1 AND ds.active = 1",
                (input.business_id, line.department_id, line.sellable_id),
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, bool>(3)?,
                        row.get::<_, Option<i64>>(4)?,
                        row.get::<_, Option<bool>>(5)?,
                    ))
                },
            )
            .optional()?
            .ok_or(CoreError::SaleItemNotAvailable)?;
        let gross_minor = money::multiply_minor_by_quantity(available.2, line.quantity_micros)?;
        if line.discount_minor > gross_minor {
            return Err(CoreError::InvalidSaleDiscount);
        }
        let after_discount = gross_minor - line.discount_minor;
        let rate_ppm = if tax_enabled && available.3 {
            business_tax_rate_ppm
        } else {
            0
        };
        let line_tax = if rate_ppm == 0 {
            0
        } else if prices_include_tax {
            money::round_ratio(
                i128::from(after_discount) * i128::from(rate_ppm),
                i128::from(1_000_000 + rate_ppm),
            )?
        } else {
            money::round_ratio(i128::from(after_discount) * i128::from(rate_ppm), 1_000_000)?
        };
        let line_total = if prices_include_tax {
            after_discount
        } else {
            after_discount
                .checked_add(line_tax)
                .ok_or(CoreError::MoneyOverflow)?
        };
        let location_id: Option<String> = transaction.query_row(
            "SELECT COALESCE(
                    (SELECT dl.location_id FROM department_locations dl
                     WHERE dl.department_id = ?1 AND dl.is_default = 1),
                    (SELECT t.location_id FROM terminals t WHERE t.id = ?2)
                 )",
            (line.department_id, input.terminal_id),
            |row| row.get(0),
        )?;
        let has_recipe = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM recipes WHERE owner_sellable_id = ?1 AND active = 1)",
            [line.sellable_id],
            |row| row.get::<_, bool>(0),
        )?;
        let mut line_cost = 0_i64;
        let mut consumptions = Vec::new();
        if has_recipe {
            let location_id = location_id
                .as_deref()
                .ok_or(CoreError::InventoryLocationRequired)?;
            for consumed in
                recipe::calculate_consumption(&transaction, line.sellable_id, line.quantity_micros)?
            {
                let ingredient_cost: i64 = transaction.query_row(
                    "SELECT cost_minor FROM products WHERE sellable_id = ?1",
                    [&consumed.ingredient_product_id],
                    |row| row.get(0),
                )?;
                line_cost = line_cost
                    .checked_add(money::multiply_minor_by_quantity(
                        ingredient_cost,
                        consumed.quantity_micros,
                    )?)
                    .ok_or(CoreError::MoneyOverflow)?;
                *effects
                    .entry((
                        consumed.ingredient_product_id.clone(),
                        location_id.to_owned(),
                        "CONSUMPTION".to_owned(),
                    ))
                    .or_default() -= consumed.quantity_micros;
                consumptions.push(PreparedConsumption {
                    product_id: consumed.ingredient_product_id,
                    location_id: location_id.to_owned(),
                    quantity_micros: consumed.quantity_micros,
                });
            }
        } else if available.1 == "PRODUCT" {
            line_cost =
                money::multiply_minor_by_quantity(available.4.unwrap_or(0), line.quantity_micros)?;
            if available.5.unwrap_or(false) {
                let location_id = location_id
                    .as_deref()
                    .ok_or(CoreError::InventoryLocationRequired)?;
                *effects
                    .entry((
                        line.sellable_id.to_owned(),
                        location_id.to_owned(),
                        "SALE".to_owned(),
                    ))
                    .or_default() -= line.quantity_micros;
                consumptions.push(PreparedConsumption {
                    product_id: line.sellable_id.to_owned(),
                    location_id: location_id.to_owned(),
                    quantity_micros: line.quantity_micros,
                });
            }
        }
        subtotal_minor = subtotal_minor
            .checked_add(gross_minor)
            .ok_or(CoreError::MoneyOverflow)?;
        discount_minor = discount_minor
            .checked_add(line.discount_minor)
            .ok_or(CoreError::MoneyOverflow)?;
        tax_minor = tax_minor
            .checked_add(line_tax)
            .ok_or(CoreError::MoneyOverflow)?;
        total_minor = total_minor
            .checked_add(line_total)
            .ok_or(CoreError::MoneyOverflow)?;
        prepared.push(PreparedLine {
            id: Uuid::now_v7().to_string(),
            sellable_id: line.sellable_id.to_owned(),
            department_id: line.department_id.to_owned(),
            description: available.0,
            kind: available.1,
            quantity_micros: line.quantity_micros,
            unit_price_minor: available.2,
            discount_minor: line.discount_minor,
            tax_rate_ppm: rate_ppm,
            tax_minor: line_tax,
            line_total_minor: line_total,
            cost_minor: line_cost,
            consumptions,
        });
    }

    let mut payment_total = 0_i64;
    let mut total_change = 0_i64;
    let mut cash_allocated = 0_i64;
    let mut prepared_payments = Vec::with_capacity(input.payments.len());
    for payment in input.payments {
        if payment.amount_minor <= 0 {
            return Err(CoreError::PaymentMismatch);
        }
        let method = transaction
            .query_row(
                "SELECT name, kind FROM payment_methods
                 WHERE id = ?1 AND business_id = ?2 AND active = 1",
                (payment.method_id, input.business_id),
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?
            .ok_or(CoreError::PaymentMethodNotFound)?;
        let (tendered, change) = if method.1 == "CASH" {
            let tendered = payment.tendered_minor.unwrap_or(payment.amount_minor);
            if tendered < payment.amount_minor {
                return Err(CoreError::InvalidCashTender);
            }
            cash_allocated = cash_allocated
                .checked_add(payment.amount_minor)
                .ok_or(CoreError::MoneyOverflow)?;
            (Some(tendered), tendered - payment.amount_minor)
        } else {
            if payment
                .tendered_minor
                .is_some_and(|value| value != payment.amount_minor)
            {
                return Err(CoreError::NonCashOverpayment);
            }
            (payment.tendered_minor, 0)
        };
        payment_total = payment_total
            .checked_add(payment.amount_minor)
            .ok_or(CoreError::MoneyOverflow)?;
        total_change = total_change
            .checked_add(change)
            .ok_or(CoreError::MoneyOverflow)?;
        prepared_payments.push((payment, method.0, method.1, tendered, change));
    }
    if payment_total != total_minor {
        return Err(CoreError::PaymentMismatch);
    }

    let overall_department = prepared
        .iter()
        .map(|line| line.department_id.as_str())
        .collect::<BTreeSet<_>>();
    let overall_department =
        (overall_department.len() == 1).then(|| prepared[0].department_id.as_str());
    transaction.execute(
        "INSERT INTO sales(
             id, business_id, sale_number, idempotency_key, department_id, terminal_id,
             shift_id, customer_id, cashier_id, status, subtotal_minor, discount_minor,
             tax_minor, total_minor, amount_paid_minor, change_due_minor, currency,
             completed_at, notes
         ) VALUES(
             ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'COMPLETED', ?10, ?11,
             ?12, ?13, ?13, ?14, ?15, ?16, ?17
         )",
        params![
            &sale_id,
            input.business_id,
            &sale_number,
            input.idempotency_key,
            overall_department,
            input.terminal_id,
            input.shift_id,
            input.customer_id,
            input.cashier_id,
            subtotal_minor,
            discount_minor,
            tax_minor,
            total_minor,
            total_change,
            &currency,
            input.completed_at,
            input.notes,
        ],
    )?;
    for line in &prepared {
        transaction.execute(
            "INSERT INTO sale_items(
                 id, sale_id, sellable_id, department_id, description, kind,
                 quantity_micros, unit_price_minor, discount_minor, tax_rate_ppm,
                 tax_minor, line_total_minor, cost_minor
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            (
                &line.id,
                &sale_id,
                &line.sellable_id,
                &line.department_id,
                &line.description,
                &line.kind,
                line.quantity_micros,
                line.unit_price_minor,
                line.discount_minor,
                line.tax_rate_ppm,
                line.tax_minor,
                line.line_total_minor,
                line.cost_minor,
            ),
        )?;
        for consumption in &line.consumptions {
            transaction.execute(
                "INSERT INTO sale_item_consumptions(
                     sale_item_id, product_id, location_id, quantity_micros
                 ) VALUES(?1, ?2, ?3, ?4)",
                (
                    &line.id,
                    &consumption.product_id,
                    &consumption.location_id,
                    consumption.quantity_micros,
                ),
            )?;
        }
    }
    let mut receipt_payments = Vec::with_capacity(prepared_payments.len());
    for (payment, method_name, method_kind, tendered, change) in prepared_payments {
        let payment_id = Uuid::now_v7().to_string();
        transaction.execute(
            "INSERT INTO payments(
                 id, business_id, sale_id, method_id, amount_minor, tendered_minor,
                 change_minor, reference, created_by
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            (
                &payment_id,
                input.business_id,
                &sale_id,
                payment.method_id,
                payment.amount_minor,
                tendered,
                change,
                payment.reference,
                input.cashier_id,
            ),
        )?;
        receipt_payments.push(ReceiptPayment {
            id: payment_id,
            method_name,
            method_kind,
            amount_minor: payment.amount_minor,
            tendered_minor: tendered,
            change_minor: change,
            reference: payment.reference.map(str::to_owned),
            status: "RECORDED".to_owned(),
        });
    }
    for ((product_id, location_id, movement_type), quantity_micros) in effects {
        inventory::post_movement(
            &transaction,
            inventory::NewMovement {
                business_id: input.business_id,
                product_id: &product_id,
                location_id: &location_id,
                quantity_micros,
                movement_type: &movement_type,
                occurred_at: input.completed_at,
                user_id: input.cashier_id,
                terminal_id: Some(input.terminal_id),
                reference_type: "sale",
                reference_id: &sale_id,
                correlation_id: &correlation_id,
                notes: input.notes,
            },
        )?;
    }
    if cash_allocated > 0 {
        transaction.execute(
            "UPDATE shifts SET expected_balance_minor = expected_balance_minor + ?2
             WHERE id = ?1 AND status = 'OPEN'",
            (input.shift_id, cash_allocated),
        )?;
    }
    let payload = format!(
        "{{\"saleNumber\":\"{}\",\"totalMinor\":{},\"paymentCount\":{}}}",
        sale_number,
        total_minor,
        input.payments.len()
    );
    audit::record_event(
        &transaction,
        audit::NewAuditEvent {
            business_id: input.business_id,
            user_id: Some(input.cashier_id),
            terminal_id: Some(input.terminal_id),
            action: "SALE_COMPLETED",
            entity_type: "sale",
            entity_id: &sale_id,
            old_json: None,
            new_json: Some(&payload),
        },
    )?;
    transaction.commit()?;

    Ok(CompletedSale {
        id: sale_id,
        sale_number,
        currency,
        subtotal_minor,
        discount_minor,
        tax_minor,
        total_minor,
        amount_paid_minor: payment_total,
        change_due_minor: total_change,
        completed_at: input.completed_at.to_owned(),
        lines: prepared
            .into_iter()
            .map(|line| ReceiptLine {
                id: line.id,
                description: line.description,
                kind: line.kind,
                quantity_micros: line.quantity_micros,
                unit_price_minor: line.unit_price_minor,
                discount_minor: line.discount_minor,
                tax_minor: line.tax_minor,
                line_total_minor: line.line_total_minor,
            })
            .collect(),
        payments: receipt_payments,
        idempotent_replay: false,
    })
}

pub fn load_completed_sale(connection: &Connection, sale_id: &str) -> Result<CompletedSale> {
    let header = connection
        .query_row(
            "SELECT sale_number, currency, subtotal_minor, discount_minor, tax_minor,
                    total_minor, amount_paid_minor, change_due_minor, completed_at
             FROM sales WHERE id = ?1 AND status IN ('COMPLETED','PART_REFUNDED','REFUNDED','VOID')",
            [sale_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, String>(8)?,
                ))
            },
        )
        .optional()?
        .ok_or(CoreError::SaleNotFound)?;
    let mut line_statement = connection.prepare(
        "SELECT id, description, kind, quantity_micros, unit_price_minor,
                discount_minor, tax_minor, line_total_minor
         FROM sale_items WHERE sale_id = ?1 ORDER BY created_at, id",
    )?;
    let lines = line_statement
        .query_map([sale_id], |row| {
            Ok(ReceiptLine {
                id: row.get(0)?,
                description: row.get(1)?,
                kind: row.get(2)?,
                quantity_micros: row.get(3)?,
                unit_price_minor: row.get(4)?,
                discount_minor: row.get(5)?,
                tax_minor: row.get(6)?,
                line_total_minor: row.get(7)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut payment_statement = connection.prepare(
        "SELECT p.id, pm.name, pm.kind, p.amount_minor, p.tendered_minor, p.change_minor,
                p.reference, p.status
         FROM payments p JOIN payment_methods pm ON pm.id = p.method_id
         WHERE p.sale_id = ?1 ORDER BY p.created_at, p.id",
    )?;
    let payments = payment_statement
        .query_map([sale_id], |row| {
            Ok(ReceiptPayment {
                id: row.get(0)?,
                method_name: row.get(1)?,
                method_kind: row.get(2)?,
                amount_minor: row.get(3)?,
                tendered_minor: row.get(4)?,
                change_minor: row.get(5)?,
                reference: row.get(6)?,
                status: row.get(7)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(CompletedSale {
        id: sale_id.to_owned(),
        sale_number: header.0,
        currency: header.1,
        subtotal_minor: header.2,
        discount_minor: header.3,
        tax_minor: header.4,
        total_minor: header.5,
        amount_paid_minor: header.6,
        change_due_minor: header.7,
        completed_at: header.8,
        lines,
        payments,
        idempotent_replay: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{availability, catalogue, inventory, payment, recipe, setup, shift, unit};

    struct Fixture {
        database: Connection,
        business_id: String,
        department_id: String,
        location_id: String,
        terminal_id: String,
        user_id: String,
        shift_id: String,
        lager: String,
        wash: String,
        shampoo: String,
        cash: String,
        card: String,
    }

    fn fixture() -> Fixture {
        let database = crate::open_memory_database().unwrap();
        let setup = setup::complete_initial_setup(
            &database,
            setup::InitialSetup {
                business_name: "Mixed Business",
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
        let ml = unit::create_unit(
            &database,
            &setup.business_id,
            "ML",
            "Millilitre",
            "VOLUME",
            1,
            1,
            3,
        )
        .unwrap();
        let lager = catalogue::create_product(
            &database,
            catalogue::NewProduct {
                business_id: &setup.business_id,
                name: "Lager",
                price_minor: 2_500,
                category_id: None,
                taxable: true,
                base_unit_id: &each,
                cost_minor: 1_000,
                track_stock: true,
                minimum_quantity_micros: 0,
            },
        )
        .unwrap();
        let shampoo = catalogue::create_product(
            &database,
            catalogue::NewProduct {
                business_id: &setup.business_id,
                name: "Shampoo",
                price_minor: 0,
                category_id: None,
                taxable: false,
                base_unit_id: &ml,
                cost_minor: 2,
                track_stock: true,
                minimum_quantity_micros: 0,
            },
        )
        .unwrap();
        let wash = catalogue::create_service(
            &database,
            catalogue::NewService {
                business_id: &setup.business_id,
                name: "Premium Wash",
                price_minor: 10_000,
                category_id: None,
                taxable: true,
                duration_minutes: Some(30),
            },
        )
        .unwrap();
        availability::set_department_availability(
            &database,
            &setup.department_id,
            &lager,
            None,
            true,
        )
        .unwrap();
        availability::set_department_availability(
            &database,
            &setup.department_id,
            &wash,
            None,
            true,
        )
        .unwrap();
        recipe::replace_recipe(
            &database,
            &setup.business_id,
            &wash,
            1_000_000,
            &[recipe::NewRecipeItem {
                ingredient_product_id: &shampoo,
                quantity_micros: 150_000_000,
                unit_id: &ml,
            }],
        )
        .unwrap();
        for (product_id, quantity, reference) in [
            (&lager, 10_000_000, "opening-lager"),
            (&shampoo, 1_000_000_000, "opening-shampoo"),
        ] {
            inventory::post_movement(
                &database,
                inventory::NewMovement {
                    business_id: &setup.business_id,
                    product_id,
                    location_id: &setup.location_id,
                    quantity_micros: quantity,
                    movement_type: "OPENING",
                    occurred_at: "2026-09-23T08:00:00.000Z",
                    user_id: &setup.user_id,
                    terminal_id: Some(&setup.terminal_id),
                    reference_type: "opening",
                    reference_id: reference,
                    correlation_id: reference,
                    notes: None,
                },
            )
            .unwrap();
        }
        let methods = payment::ensure_default_methods(
            &database,
            &setup.business_id,
            &setup.user_id,
            Some(&setup.terminal_id),
        )
        .unwrap();
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
        let cash = methods
            .iter()
            .find(|item| item.kind == "CASH")
            .unwrap()
            .id
            .clone();
        let card = methods
            .iter()
            .find(|item| item.kind == "CARD")
            .unwrap()
            .id
            .clone();
        Fixture {
            database,
            business_id: setup.business_id,
            department_id: setup.department_id,
            location_id: setup.location_id,
            terminal_id: setup.terminal_id,
            user_id: setup.user_id,
            shift_id,
            lager,
            wash,
            shampoo,
            cash,
            card,
        }
    }

    #[test]
    fn completes_split_payment_sale_with_direct_and_recipe_consumption() {
        let fixture = fixture();
        let lines = [
            SaleLineInput {
                sellable_id: &fixture.lager,
                department_id: &fixture.department_id,
                quantity_micros: 2_000_000,
                discount_minor: 0,
            },
            SaleLineInput {
                sellable_id: &fixture.wash,
                department_id: &fixture.department_id,
                quantity_micros: 1_000_000,
                discount_minor: 0,
            },
        ];
        let payments = [
            PaymentInput {
                method_id: &fixture.cash,
                amount_minor: 5_000,
                tendered_minor: Some(10_000),
                reference: None,
            },
            PaymentInput {
                method_id: &fixture.card,
                amount_minor: 10_000,
                tendered_minor: Some(10_000),
                reference: Some("CARD-OK"),
            },
        ];
        let input = CompleteSale {
            business_id: &fixture.business_id,
            idempotency_key: "sale-key-1",
            sale_number: Some("SALE-1"),
            terminal_id: &fixture.terminal_id,
            shift_id: &fixture.shift_id,
            cashier_id: &fixture.user_id,
            customer_id: None,
            completed_at: "2026-09-23T10:00:00.000Z",
            notes: None,
            lines: &lines,
            payments: &payments,
        };
        let completed = complete_sale(&fixture.database, input).unwrap();
        assert_eq!(completed.total_minor, 15_000);
        assert_eq!(completed.change_due_minor, 5_000);
        assert_eq!(completed.lines.len(), 2);
        assert_eq!(completed.payments.len(), 2);
        assert_eq!(
            inventory::get_balance(
                &fixture.database,
                &fixture.business_id,
                &fixture.lager,
                &fixture.location_id
            )
            .unwrap(),
            8_000_000
        );
        assert_eq!(
            inventory::get_balance(
                &fixture.database,
                &fixture.business_id,
                &fixture.shampoo,
                &fixture.location_id
            )
            .unwrap(),
            850_000_000
        );
        let replay = complete_sale(&fixture.database, input).unwrap();
        assert!(replay.idempotent_replay);
        assert_eq!(replay.id, completed.id);
        let sale_count: i64 = fixture
            .database
            .query_row("SELECT count(*) FROM sales", [], |row| row.get(0))
            .unwrap();
        assert_eq!(sale_count, 1);
    }

    #[test]
    fn payment_mismatch_rolls_back_everything() {
        let fixture = fixture();
        let lines = [SaleLineInput {
            sellable_id: &fixture.lager,
            department_id: &fixture.department_id,
            quantity_micros: 1_000_000,
            discount_minor: 0,
        }];
        let payments = [PaymentInput {
            method_id: &fixture.card,
            amount_minor: 2_400,
            tendered_minor: None,
            reference: None,
        }];
        let result = complete_sale(
            &fixture.database,
            CompleteSale {
                business_id: &fixture.business_id,
                idempotency_key: "mismatch",
                sale_number: None,
                terminal_id: &fixture.terminal_id,
                shift_id: &fixture.shift_id,
                cashier_id: &fixture.user_id,
                customer_id: None,
                completed_at: "2026-09-23T10:00:00.000Z",
                notes: None,
                lines: &lines,
                payments: &payments,
            },
        );
        assert!(matches!(result, Err(CoreError::PaymentMismatch)));
        let facts: (i64, i64) = fixture
            .database
            .query_row(
                "SELECT (SELECT count(*) FROM sales),
                        (SELECT count(*) FROM payments)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(facts, (0, 0));
    }

    #[test]
    fn insufficient_stock_rolls_back_sale_and_payments() {
        let fixture = fixture();
        let lines = [SaleLineInput {
            sellable_id: &fixture.lager,
            department_id: &fixture.department_id,
            quantity_micros: 11_000_000,
            discount_minor: 0,
        }];
        let payments = [PaymentInput {
            method_id: &fixture.card,
            amount_minor: 27_500,
            tendered_minor: None,
            reference: None,
        }];
        let result = complete_sale(
            &fixture.database,
            CompleteSale {
                business_id: &fixture.business_id,
                idempotency_key: "no-stock",
                sale_number: None,
                terminal_id: &fixture.terminal_id,
                shift_id: &fixture.shift_id,
                cashier_id: &fixture.user_id,
                customer_id: None,
                completed_at: "2026-09-23T10:00:00.000Z",
                notes: None,
                lines: &lines,
                payments: &payments,
            },
        );
        assert!(matches!(result, Err(CoreError::NegativeStock)));
        let sales: i64 = fixture
            .database
            .query_row("SELECT count(*) FROM sales", [], |row| row.get(0))
            .unwrap();
        assert_eq!(sales, 0);
        assert_eq!(
            inventory::get_balance(
                &fixture.database,
                &fixture.business_id,
                &fixture.lager,
                &fixture.location_id
            )
            .unwrap(),
            10_000_000
        );
    }

    #[test]
    fn rejects_non_cash_over_tender() {
        let fixture = fixture();
        let lines = [SaleLineInput {
            sellable_id: &fixture.lager,
            department_id: &fixture.department_id,
            quantity_micros: 1_000_000,
            discount_minor: 0,
        }];
        let payments = [PaymentInput {
            method_id: &fixture.card,
            amount_minor: 2_500,
            tendered_minor: Some(3_000),
            reference: None,
        }];
        let result = complete_sale(
            &fixture.database,
            CompleteSale {
                business_id: &fixture.business_id,
                idempotency_key: "card-overage",
                sale_number: None,
                terminal_id: &fixture.terminal_id,
                shift_id: &fixture.shift_id,
                cashier_id: &fixture.user_id,
                customer_id: None,
                completed_at: "2026-09-23T10:00:00.000Z",
                notes: None,
                lines: &lines,
                payments: &payments,
            },
        );
        assert!(matches!(result, Err(CoreError::NonCashOverpayment)));
    }

    #[test]
    fn snapshots_inclusive_tax_using_central_rounding() {
        let fixture = fixture();
        fixture
            .database
            .execute(
                "UPDATE businesses
                 SET tax_enabled = 1, tax_rate_ppm = 150000, prices_include_tax = 1
                 WHERE id = ?1",
                [&fixture.business_id],
            )
            .unwrap();
        let lines = [SaleLineInput {
            sellable_id: &fixture.lager,
            department_id: &fixture.department_id,
            quantity_micros: 1_000_000,
            discount_minor: 0,
        }];
        let payments = [PaymentInput {
            method_id: &fixture.card,
            amount_minor: 2_500,
            tendered_minor: None,
            reference: None,
        }];
        let completed = complete_sale(
            &fixture.database,
            CompleteSale {
                business_id: &fixture.business_id,
                idempotency_key: "taxed-sale",
                sale_number: None,
                terminal_id: &fixture.terminal_id,
                shift_id: &fixture.shift_id,
                cashier_id: &fixture.user_id,
                customer_id: None,
                completed_at: "2026-09-23T10:00:00.000Z",
                notes: None,
                lines: &lines,
                payments: &payments,
            },
        )
        .unwrap();
        assert_eq!(completed.total_minor, 2_500);
        assert_eq!(completed.tax_minor, 326);
        assert_eq!(completed.lines[0].tax_minor, 326);
    }
}
