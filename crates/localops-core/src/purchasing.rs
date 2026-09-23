use std::collections::BTreeSet;

use rusqlite::{Connection, OptionalExtension};
use uuid::Uuid;

use crate::{CoreError, Result, audit, conversion, inventory, money, packaging};

#[derive(Debug, Clone, Copy)]
pub struct NewSupplier<'a> {
    pub business_id: &'a str,
    pub name: &'a str,
    pub contact_name: Option<&'a str>,
    pub email: Option<&'a str>,
    pub phone: Option<&'a str>,
    pub user_id: &'a str,
    pub terminal_id: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Supplier {
    pub id: String,
    pub name: String,
    pub contact_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct PurchaseItemInput<'a> {
    pub product_id: &'a str,
    pub description: &'a str,
    pub quantity_micros: i64,
    pub unit_id: &'a str,
    pub packaging_id: Option<&'a str>,
    pub unit_cost_minor: i64,
}

#[derive(Debug, Clone, Copy)]
pub struct ReceivePurchase<'a> {
    pub business_id: &'a str,
    pub purchase_number: &'a str,
    pub supplier_id: Option<&'a str>,
    pub location_id: &'a str,
    pub invoice_reference: Option<&'a str>,
    pub purchased_at: &'a str,
    pub tax_minor: i64,
    pub discount_minor: i64,
    pub payment_status: &'a str,
    pub notes: Option<&'a str>,
    pub user_id: &'a str,
    pub terminal_id: Option<&'a str>,
    pub items: &'a [PurchaseItemInput<'a>],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceivedPurchase {
    pub id: String,
    pub purchase_number: String,
    pub subtotal_minor: i64,
    pub total_minor: i64,
    pub item_count: usize,
}

pub fn create_supplier(connection: &Connection, input: NewSupplier<'_>) -> Result<String> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(CoreError::EmptySupplierName);
    }
    let transaction = connection.unchecked_transaction()?;
    let valid_user = transaction
        .query_row(
            "SELECT 1 FROM users WHERE id = ?1 AND business_id = ?2 AND active = 1",
            (input.user_id, input.business_id),
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if !valid_user {
        return Err(CoreError::CrossBusinessReference);
    }
    let id = Uuid::now_v7().to_string();
    transaction.execute(
        "INSERT INTO suppliers(id, business_id, name, contact_name, email, phone)
         VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
        (
            &id,
            input.business_id,
            name,
            input.contact_name,
            input.email,
            input.phone,
        ),
    )?;
    let payload = format!("{{\"name\":\"{}\"}}", name.replace('"', "\\\""));
    audit::record_event(
        &transaction,
        audit::NewAuditEvent {
            business_id: input.business_id,
            user_id: Some(input.user_id),
            terminal_id: input.terminal_id,
            action: "SUPPLIER_CREATED",
            entity_type: "supplier",
            entity_id: &id,
            old_json: None,
            new_json: Some(&payload),
        },
    )?;
    transaction.commit()?;
    Ok(id)
}

pub fn list_suppliers(connection: &Connection, business_id: &str) -> Result<Vec<Supplier>> {
    let mut statement = connection.prepare(
        "SELECT id, name, contact_name, email, phone
         FROM suppliers WHERE business_id = ?1 AND active = 1 ORDER BY name",
    )?;
    statement
        .query_map([business_id], |row| {
            Ok(Supplier {
                id: row.get(0)?,
                name: row.get(1)?,
                contact_name: row.get(2)?,
                email: row.get(3)?,
                phone: row.get(4)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn receive_purchase(
    connection: &Connection,
    input: ReceivePurchase<'_>,
) -> Result<ReceivedPurchase> {
    if input.items.is_empty() {
        return Err(CoreError::EmptyPurchase);
    }
    if input.tax_minor < 0 || input.discount_minor < 0 {
        return Err(CoreError::InvalidPurchaseTotal);
    }
    let unique_products = input
        .items
        .iter()
        .map(|item| item.product_id)
        .collect::<BTreeSet<_>>();
    if unique_products.len() != input.items.len() {
        return Err(CoreError::DuplicatePurchaseProduct);
    }
    let transaction = connection.unchecked_transaction()?;
    let context_valid = transaction
        .query_row(
            "SELECT 1 FROM locations l
             JOIN users u ON u.business_id = l.business_id
             WHERE l.id = ?2 AND l.business_id = ?1 AND l.active = 1
               AND u.id = ?3 AND u.active = 1",
            (input.business_id, input.location_id, input.user_id),
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if !context_valid {
        return Err(CoreError::CrossBusinessReference);
    }
    if let Some(supplier_id) = input.supplier_id {
        let supplier_valid = transaction
            .query_row(
                "SELECT 1 FROM suppliers WHERE id = ?1 AND business_id = ?2 AND active = 1",
                (supplier_id, input.business_id),
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if !supplier_valid {
            return Err(CoreError::SupplierNotFound);
        }
    }

    let purchase_id = Uuid::now_v7().to_string();
    transaction.execute(
        "INSERT INTO purchases(
             id, business_id, purchase_number, supplier_id, location_id, status,
             invoice_reference, purchased_at, total_minor, tax_minor, discount_minor,
             notes, created_by, payment_status
         ) VALUES(?1, ?2, ?3, ?4, ?5, 'RECEIVED', ?6, ?7, 0, ?8, ?9, ?10, ?11, ?12)",
        (
            &purchase_id,
            input.business_id,
            input.purchase_number,
            input.supplier_id,
            input.location_id,
            input.invoice_reference,
            input.purchased_at,
            input.tax_minor,
            input.discount_minor,
            input.notes,
            input.user_id,
            input.payment_status,
        ),
    )?;
    let correlation_id = Uuid::now_v7().to_string();
    let mut subtotal_minor = 0_i64;
    for item in input.items {
        if item.quantity_micros <= 0 || item.unit_cost_minor < 0 {
            return Err(CoreError::InvalidPurchaseTotal);
        }
        let base_unit_id = transaction
            .query_row(
                "SELECT p.base_unit_id
                 FROM products p JOIN sellable_items s ON s.id = p.sellable_id
                 WHERE p.sellable_id = ?1 AND s.business_id = ?2 AND p.track_stock = 1",
                (item.product_id, input.business_id),
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or(CoreError::CrossBusinessReference)?;
        let base_quantity_micros = if let Some(packaging_id) = item.packaging_id {
            let packaging_matches = transaction
                .query_row(
                    "SELECT 1 FROM product_packaging
                     WHERE id = ?1 AND product_id = ?2 AND unit_id = ?3 AND can_purchase = 1",
                    (packaging_id, item.product_id, item.unit_id),
                    |_| Ok(()),
                )
                .optional()?
                .is_some();
            if !packaging_matches {
                return Err(CoreError::PackagingNotFound);
            }
            packaging::packaging_to_base_micros(&transaction, packaging_id, item.quantity_micros)?
        } else {
            conversion::convert_quantity_micros(
                &transaction,
                input.business_id,
                item.unit_id,
                &base_unit_id,
                item.quantity_micros,
            )?
        };
        let line_total_minor =
            money::multiply_minor_by_quantity(item.unit_cost_minor, item.quantity_micros)?;
        subtotal_minor = subtotal_minor
            .checked_add(line_total_minor)
            .ok_or(CoreError::MoneyOverflow)?;
        let purchase_item_id = Uuid::now_v7().to_string();
        transaction.execute(
            "INSERT INTO purchase_items(
                 id, purchase_id, product_id, description, quantity_micros, unit_id,
                 base_quantity_micros, unit_cost_minor, line_total_minor
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            (
                &purchase_item_id,
                &purchase_id,
                item.product_id,
                item.description,
                item.quantity_micros,
                item.unit_id,
                base_quantity_micros,
                item.unit_cost_minor,
                line_total_minor,
            ),
        )?;
        inventory::post_movement(
            &transaction,
            inventory::NewMovement {
                business_id: input.business_id,
                product_id: item.product_id,
                location_id: input.location_id,
                quantity_micros: base_quantity_micros,
                movement_type: "PURCHASE",
                occurred_at: input.purchased_at,
                user_id: input.user_id,
                terminal_id: input.terminal_id,
                reference_type: "purchase",
                reference_id: &purchase_id,
                correlation_id: &correlation_id,
                notes: input.notes,
            },
        )?;
        let latest_base_cost = money::round_ratio(
            i128::from(line_total_minor) * 1_000_000,
            i128::from(base_quantity_micros),
        )?;
        transaction.execute(
            "UPDATE products SET cost_minor = ?2 WHERE sellable_id = ?1",
            (item.product_id, latest_base_cost),
        )?;
    }
    let total_minor = subtotal_minor
        .checked_add(input.tax_minor)
        .and_then(|value| value.checked_sub(input.discount_minor))
        .filter(|value| *value >= 0)
        .ok_or(CoreError::InvalidPurchaseTotal)?;
    transaction.execute(
        "UPDATE purchases SET total_minor = ?2 WHERE id = ?1",
        (&purchase_id, total_minor),
    )?;
    let payload = format!(
        "{{\"purchaseNumber\":\"{}\",\"totalMinor\":{},\"itemCount\":{}}}",
        input.purchase_number.replace('"', "\\\""),
        total_minor,
        input.items.len()
    );
    audit::record_event(
        &transaction,
        audit::NewAuditEvent {
            business_id: input.business_id,
            user_id: Some(input.user_id),
            terminal_id: input.terminal_id,
            action: "PURCHASE_RECEIVED",
            entity_type: "purchase",
            entity_id: &purchase_id,
            old_json: None,
            new_json: Some(&payload),
        },
    )?;
    transaction.commit()?;
    Ok(ReceivedPurchase {
        id: purchase_id,
        purchase_number: input.purchase_number.to_owned(),
        subtotal_minor,
        total_minor,
        item_count: input.items.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{catalogue, inventory, setup, unit};

    struct Fixture {
        database: Connection,
        business_id: String,
        location_id: String,
        user_id: String,
        terminal_id: String,
        each: String,
        crate_unit: String,
        product_id: String,
    }

    fn fixture() -> Fixture {
        let database = crate::open_memory_database().unwrap();
        let setup = setup::complete_initial_setup(
            &database,
            setup::InitialSetup {
                business_name: "Bottle Store",
                department_name: "Shop",
                location_name: "Main Store",
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
            "BTL",
            "Bottle",
            "COUNT",
            1,
            1,
            0,
        )
        .unwrap();
        let crate_unit = unit::create_unit(
            &database,
            &setup.business_id,
            "CRT",
            "Crate",
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
                name: "Lager",
                price_minor: 2_500,
                category_id: None,
                taxable: true,
                base_unit_id: &each,
                cost_minor: 0,
                track_stock: true,
                minimum_quantity_micros: 0,
            },
        )
        .unwrap();
        Fixture {
            database,
            business_id: setup.business_id,
            location_id: setup.location_id,
            user_id: setup.user_id,
            terminal_id: setup.terminal_id,
            each,
            crate_unit,
            product_id,
        }
    }

    #[test]
    fn receives_packaged_purchase_and_posts_stock_atomically() {
        let fixture = fixture();
        let supplier_id = create_supplier(
            &fixture.database,
            NewSupplier {
                business_id: &fixture.business_id,
                name: "ABC Distributors",
                contact_name: None,
                email: None,
                phone: None,
                user_id: &fixture.user_id,
                terminal_id: Some(&fixture.terminal_id),
            },
        )
        .unwrap();
        let packaging_id = packaging::create_packaging(
            &fixture.database,
            &fixture.business_id,
            &fixture.product_id,
            &fixture.crate_unit,
            "Crate of 12",
            12,
            1,
            true,
            false,
        )
        .unwrap();
        let items = [PurchaseItemInput {
            product_id: &fixture.product_id,
            description: "Lager crate",
            quantity_micros: 2_000_000,
            unit_id: &fixture.crate_unit,
            packaging_id: Some(&packaging_id),
            unit_cost_minor: 12_000,
        }];
        let received = receive_purchase(
            &fixture.database,
            ReceivePurchase {
                business_id: &fixture.business_id,
                purchase_number: "PO-001",
                supplier_id: Some(&supplier_id),
                location_id: &fixture.location_id,
                invoice_reference: Some("INV-1"),
                purchased_at: "2026-09-23T12:00:00.000Z",
                tax_minor: 0,
                discount_minor: 0,
                payment_status: "PAID",
                notes: None,
                user_id: &fixture.user_id,
                terminal_id: Some(&fixture.terminal_id),
                items: &items,
            },
        )
        .unwrap();
        assert_eq!(received.total_minor, 24_000);
        assert_eq!(
            inventory::get_balance(
                &fixture.database,
                &fixture.business_id,
                &fixture.product_id,
                &fixture.location_id
            )
            .unwrap(),
            24_000_000
        );
        let stored: (i64, String, i64) = fixture.database.query_row("SELECT p.total_minor, p.payment_status, pr.cost_minor FROM purchases p JOIN purchase_items pi ON pi.purchase_id = p.id JOIN products pr ON pr.sellable_id = pi.product_id WHERE p.id = ?1", [&received.id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).unwrap();
        assert_eq!(stored, (24_000, "PAID".to_owned(), 1_000));
    }

    #[test]
    fn failed_item_rolls_back_purchase_and_all_movements() {
        let fixture = fixture();
        let items = [PurchaseItemInput {
            product_id: &fixture.product_id,
            description: "Invalid",
            quantity_micros: 1_000_000,
            unit_id: "missing",
            packaging_id: None,
            unit_cost_minor: 100,
        }];
        assert!(
            receive_purchase(
                &fixture.database,
                ReceivePurchase {
                    business_id: &fixture.business_id,
                    purchase_number: "PO-BAD",
                    supplier_id: None,
                    location_id: &fixture.location_id,
                    invoice_reference: None,
                    purchased_at: "2026-09-23T12:00:00.000Z",
                    tax_minor: 0,
                    discount_minor: 0,
                    payment_status: "UNPAID",
                    notes: None,
                    user_id: &fixture.user_id,
                    terminal_id: Some(&fixture.terminal_id),
                    items: &items
                }
            )
            .is_err()
        );
        let facts: (i64, i64) = fixture.database.query_row("SELECT (SELECT count(*) FROM purchases), (SELECT count(*) FROM inventory_movements)", [], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
        assert_eq!(facts, (0, 0));
    }

    #[test]
    fn rejects_duplicate_product_lines_before_writing() {
        let fixture = fixture();
        let item = PurchaseItemInput {
            product_id: &fixture.product_id,
            description: "Lager",
            quantity_micros: 1_000_000,
            unit_id: &fixture.each,
            packaging_id: None,
            unit_cost_minor: 1_000,
        };
        let items = [item, item];
        let result = receive_purchase(
            &fixture.database,
            ReceivePurchase {
                business_id: &fixture.business_id,
                purchase_number: "PO-DUP",
                supplier_id: None,
                location_id: &fixture.location_id,
                invoice_reference: None,
                purchased_at: "2026-09-23T12:00:00.000Z",
                tax_minor: 0,
                discount_minor: 0,
                payment_status: "UNPAID",
                notes: None,
                user_id: &fixture.user_id,
                terminal_id: Some(&fixture.terminal_id),
                items: &items,
            },
        );
        assert!(matches!(result, Err(CoreError::DuplicatePurchaseProduct)));
    }
}
