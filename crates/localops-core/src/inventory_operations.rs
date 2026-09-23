use std::collections::BTreeSet;

use rusqlite::{Connection, OptionalExtension};
use uuid::Uuid;

use crate::{CoreError, Result, audit, inventory};

#[derive(Debug, Clone, Copy)]
pub struct StockItemInput<'a> {
    pub product_id: &'a str,
    pub quantity_micros: i64,
}

#[derive(Debug, Clone, Copy)]
pub struct CompleteTransfer<'a> {
    pub business_id: &'a str,
    pub from_location_id: &'a str,
    pub to_location_id: &'a str,
    pub occurred_at: &'a str,
    pub user_id: &'a str,
    pub terminal_id: Option<&'a str>,
    pub notes: Option<&'a str>,
    pub items: &'a [StockItemInput<'a>],
}

#[derive(Debug, Clone, Copy)]
pub struct RecordWastage<'a> {
    pub business_id: &'a str,
    pub location_id: &'a str,
    pub reason: &'a str,
    pub occurred_at: &'a str,
    pub user_id: &'a str,
    pub terminal_id: Option<&'a str>,
    pub notes: Option<&'a str>,
    pub damage: bool,
    pub items: &'a [StockItemInput<'a>],
}

#[derive(Debug, Clone, Copy)]
pub struct CountedItemInput<'a> {
    pub product_id: &'a str,
    pub counted_micros: i64,
}

#[derive(Debug, Clone, Copy)]
pub struct CompleteStockCount<'a> {
    pub business_id: &'a str,
    pub location_id: &'a str,
    pub counted_at: &'a str,
    pub user_id: &'a str,
    pub terminal_id: Option<&'a str>,
    pub notes: Option<&'a str>,
    pub items: &'a [CountedItemInput<'a>],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StockCountResult {
    pub id: String,
    pub item_count: usize,
    pub variance_item_count: usize,
}

fn validate_product_set<'a>(products: impl Iterator<Item = &'a str>, count: usize) -> Result<()> {
    if count == 0 {
        return Err(CoreError::EmptyInventoryOperation);
    }
    if products.collect::<BTreeSet<_>>().len() != count {
        return Err(CoreError::DuplicateInventoryProduct);
    }
    Ok(())
}

fn validate_location_and_user(
    connection: &Connection,
    business_id: &str,
    location_id: &str,
    user_id: &str,
) -> Result<()> {
    let valid = connection
        .query_row(
            "SELECT 1 FROM locations l
             JOIN users u ON u.business_id = l.business_id
             WHERE l.id = ?2 AND l.business_id = ?1 AND l.active = 1
               AND u.id = ?3 AND u.active = 1",
            (business_id, location_id, user_id),
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if !valid {
        return Err(CoreError::CrossBusinessReference);
    }
    Ok(())
}

pub fn complete_transfer(connection: &Connection, input: CompleteTransfer<'_>) -> Result<String> {
    validate_product_set(
        input.items.iter().map(|item| item.product_id),
        input.items.len(),
    )?;
    if input.from_location_id == input.to_location_id {
        return Err(CoreError::InvalidTransferLocations);
    }
    if input.items.iter().any(|item| item.quantity_micros <= 0) {
        return Err(CoreError::InvalidInventoryQuantity);
    }
    let transaction = connection.unchecked_transaction()?;
    validate_location_and_user(
        &transaction,
        input.business_id,
        input.from_location_id,
        input.user_id,
    )?;
    validate_location_and_user(
        &transaction,
        input.business_id,
        input.to_location_id,
        input.user_id,
    )?;
    let transfer_id = Uuid::now_v7().to_string();
    transaction.execute(
        "INSERT INTO inventory_transfers(
             id, business_id, from_location_id, to_location_id, status,
             completed_at, completed_by, notes
         ) VALUES(?1, ?2, ?3, ?4, 'COMPLETED', ?5, ?6, ?7)",
        (
            &transfer_id,
            input.business_id,
            input.from_location_id,
            input.to_location_id,
            input.occurred_at,
            input.user_id,
            input.notes,
        ),
    )?;
    let correlation_id = Uuid::now_v7().to_string();
    for item in input.items {
        transaction.execute(
            "INSERT INTO inventory_transfer_items(id, transfer_id, product_id, quantity_micros)
             VALUES(?1, ?2, ?3, ?4)",
            (
                Uuid::now_v7().to_string(),
                &transfer_id,
                item.product_id,
                item.quantity_micros,
            ),
        )?;
        inventory::post_movement(
            &transaction,
            inventory::NewMovement {
                business_id: input.business_id,
                product_id: item.product_id,
                location_id: input.from_location_id,
                quantity_micros: -item.quantity_micros,
                movement_type: "TRANSFER_OUT",
                occurred_at: input.occurred_at,
                user_id: input.user_id,
                terminal_id: input.terminal_id,
                reference_type: "inventory_transfer",
                reference_id: &transfer_id,
                correlation_id: &correlation_id,
                notes: input.notes,
            },
        )?;
        inventory::post_movement(
            &transaction,
            inventory::NewMovement {
                business_id: input.business_id,
                product_id: item.product_id,
                location_id: input.to_location_id,
                quantity_micros: item.quantity_micros,
                movement_type: "TRANSFER_IN",
                occurred_at: input.occurred_at,
                user_id: input.user_id,
                terminal_id: input.terminal_id,
                reference_type: "inventory_transfer",
                reference_id: &transfer_id,
                correlation_id: &correlation_id,
                notes: input.notes,
            },
        )?;
    }
    let payload = format!(
        "{{\"fromLocationId\":\"{}\",\"toLocationId\":\"{}\",\"itemCount\":{}}}",
        input.from_location_id,
        input.to_location_id,
        input.items.len()
    );
    audit::record_event(
        &transaction,
        audit::NewAuditEvent {
            business_id: input.business_id,
            user_id: Some(input.user_id),
            terminal_id: input.terminal_id,
            action: "INVENTORY_TRANSFER_COMPLETED",
            entity_type: "inventory_transfer",
            entity_id: &transfer_id,
            old_json: None,
            new_json: Some(&payload),
        },
    )?;
    transaction.commit()?;
    Ok(transfer_id)
}

pub fn record_wastage(connection: &Connection, input: RecordWastage<'_>) -> Result<String> {
    validate_product_set(
        input.items.iter().map(|item| item.product_id),
        input.items.len(),
    )?;
    if input.reason.trim().is_empty() {
        return Err(CoreError::EmptyWastageReason);
    }
    if input.items.iter().any(|item| item.quantity_micros <= 0) {
        return Err(CoreError::InvalidInventoryQuantity);
    }
    let transaction = connection.unchecked_transaction()?;
    validate_location_and_user(
        &transaction,
        input.business_id,
        input.location_id,
        input.user_id,
    )?;
    let wastage_id = Uuid::now_v7().to_string();
    transaction.execute(
        "INSERT INTO wastage(id, business_id, location_id, reason, occurred_at, user_id, notes)
         VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        (
            &wastage_id,
            input.business_id,
            input.location_id,
            input.reason.trim(),
            input.occurred_at,
            input.user_id,
            input.notes,
        ),
    )?;
    let correlation_id = Uuid::now_v7().to_string();
    let movement_type = if input.damage { "DAMAGE" } else { "WASTE" };
    for item in input.items {
        let posted = inventory::post_movement(
            &transaction,
            inventory::NewMovement {
                business_id: input.business_id,
                product_id: item.product_id,
                location_id: input.location_id,
                quantity_micros: -item.quantity_micros,
                movement_type,
                occurred_at: input.occurred_at,
                user_id: input.user_id,
                terminal_id: input.terminal_id,
                reference_type: "wastage",
                reference_id: &wastage_id,
                correlation_id: &correlation_id,
                notes: input.notes,
            },
        )?;
        transaction.execute(
            "INSERT INTO wastage_items(id, wastage_id, product_id, quantity_micros, movement_id)
             VALUES(?1, ?2, ?3, ?4, ?5)",
            (
                Uuid::now_v7().to_string(),
                &wastage_id,
                item.product_id,
                item.quantity_micros,
                &posted.movement_id,
            ),
        )?;
    }
    let payload = format!(
        "{{\"reason\":\"{}\",\"type\":\"{}\",\"itemCount\":{}}}",
        input.reason.trim().replace('"', "\\\""),
        movement_type,
        input.items.len()
    );
    audit::record_event(
        &transaction,
        audit::NewAuditEvent {
            business_id: input.business_id,
            user_id: Some(input.user_id),
            terminal_id: input.terminal_id,
            action: "WASTAGE_RECORDED",
            entity_type: "wastage",
            entity_id: &wastage_id,
            old_json: None,
            new_json: Some(&payload),
        },
    )?;
    transaction.commit()?;
    Ok(wastage_id)
}

pub fn complete_stock_count(
    connection: &Connection,
    input: CompleteStockCount<'_>,
) -> Result<StockCountResult> {
    validate_product_set(
        input.items.iter().map(|item| item.product_id),
        input.items.len(),
    )?;
    if input.items.iter().any(|item| item.counted_micros < 0) {
        return Err(CoreError::InvalidStockCountQuantity);
    }
    let transaction = connection.unchecked_transaction()?;
    validate_location_and_user(
        &transaction,
        input.business_id,
        input.location_id,
        input.user_id,
    )?;
    let count_id = Uuid::now_v7().to_string();
    transaction.execute(
        "INSERT INTO stock_counts(
             id, business_id, location_id, status, counted_at, completed_by, notes
         ) VALUES(?1, ?2, ?3, 'COMPLETED', ?4, ?5, ?6)",
        (
            &count_id,
            input.business_id,
            input.location_id,
            input.counted_at,
            input.user_id,
            input.notes,
        ),
    )?;
    let correlation_id = Uuid::now_v7().to_string();
    let mut variance_item_count = 0_usize;
    for item in input.items {
        let expected_micros = inventory::get_balance(
            &transaction,
            input.business_id,
            item.product_id,
            input.location_id,
        )?;
        let variance_micros = item
            .counted_micros
            .checked_sub(expected_micros)
            .ok_or(CoreError::QuantityOverflow)?;
        let movement_id = if variance_micros == 0 {
            None
        } else {
            variance_item_count += 1;
            Some(
                inventory::post_movement(
                    &transaction,
                    inventory::NewMovement {
                        business_id: input.business_id,
                        product_id: item.product_id,
                        location_id: input.location_id,
                        quantity_micros: variance_micros,
                        movement_type: "COUNT",
                        occurred_at: input.counted_at,
                        user_id: input.user_id,
                        terminal_id: input.terminal_id,
                        reference_type: "stock_count",
                        reference_id: &count_id,
                        correlation_id: &correlation_id,
                        notes: input.notes,
                    },
                )?
                .movement_id,
            )
        };
        transaction.execute(
            "INSERT INTO stock_count_items(
                 id, stock_count_id, product_id, expected_micros, counted_micros,
                 variance_micros, movement_id
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            (
                Uuid::now_v7().to_string(),
                &count_id,
                item.product_id,
                expected_micros,
                item.counted_micros,
                variance_micros,
                movement_id.as_deref(),
            ),
        )?;
    }
    let payload = format!(
        "{{\"itemCount\":{},\"varianceItemCount\":{}}}",
        input.items.len(),
        variance_item_count
    );
    audit::record_event(
        &transaction,
        audit::NewAuditEvent {
            business_id: input.business_id,
            user_id: Some(input.user_id),
            terminal_id: input.terminal_id,
            action: "STOCK_COUNT_COMPLETED",
            entity_type: "stock_count",
            entity_id: &count_id,
            old_json: None,
            new_json: Some(&payload),
        },
    )?;
    transaction.commit()?;
    Ok(StockCountResult {
        id: count_id,
        item_count: input.items.len(),
        variance_item_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{catalogue, location, setup, unit};

    struct Fixture {
        database: Connection,
        business_id: String,
        from_location: String,
        to_location: String,
        user_id: String,
        terminal_id: String,
        product_id: String,
    }

    fn fixture(opening_micros: i64) -> Fixture {
        let database = crate::open_memory_database().unwrap();
        let setup = setup::complete_initial_setup(
            &database,
            setup::InitialSetup {
                business_name: "Test",
                department_name: "Shop",
                location_name: "Main",
                terminal_name: "Till",
                owner_username: "owner",
                owner_display_name: "Owner",
                owner_pin: "1234",
            },
        )
        .unwrap();
        let to_location =
            location::create_location(&database, &setup.business_id, "Secondary", None).unwrap();
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
                name: "Stock",
                price_minor: 100,
                category_id: None,
                taxable: true,
                base_unit_id: &each,
                cost_minor: 50,
                track_stock: true,
                minimum_quantity_micros: 0,
            },
        )
        .unwrap();
        let fixture = Fixture {
            database,
            business_id: setup.business_id,
            from_location: setup.location_id,
            to_location,
            user_id: setup.user_id,
            terminal_id: setup.terminal_id,
            product_id,
        };
        if opening_micros > 0 {
            inventory::post_movement(
                &fixture.database,
                inventory::NewMovement {
                    business_id: &fixture.business_id,
                    product_id: &fixture.product_id,
                    location_id: &fixture.from_location,
                    quantity_micros: opening_micros,
                    movement_type: "OPENING",
                    occurred_at: "2026-09-23T09:00:00.000Z",
                    user_id: &fixture.user_id,
                    terminal_id: Some(&fixture.terminal_id),
                    reference_type: "opening",
                    reference_id: "opening-1",
                    correlation_id: "opening-1",
                    notes: None,
                },
            )
            .unwrap();
        }
        fixture
    }

    #[test]
    fn transfer_posts_paired_correlated_movements() {
        let fixture = fixture(10_000_000);
        let items = [StockItemInput {
            product_id: &fixture.product_id,
            quantity_micros: 4_000_000,
        }];
        let transfer_id = complete_transfer(
            &fixture.database,
            CompleteTransfer {
                business_id: &fixture.business_id,
                from_location_id: &fixture.from_location,
                to_location_id: &fixture.to_location,
                occurred_at: "2026-09-23T10:00:00.000Z",
                user_id: &fixture.user_id,
                terminal_id: Some(&fixture.terminal_id),
                notes: None,
                items: &items,
            },
        )
        .unwrap();
        assert_eq!(
            inventory::get_balance(
                &fixture.database,
                &fixture.business_id,
                &fixture.product_id,
                &fixture.from_location
            )
            .unwrap(),
            6_000_000
        );
        assert_eq!(
            inventory::get_balance(
                &fixture.database,
                &fixture.business_id,
                &fixture.product_id,
                &fixture.to_location
            )
            .unwrap(),
            4_000_000
        );
        let facts: (i64, i64) = fixture
            .database
            .query_row(
                "SELECT count(*), count(DISTINCT correlation_id)
                 FROM inventory_movements WHERE reference_id = ?1",
                [&transfer_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(facts, (2, 1));
    }

    #[test]
    fn insufficient_transfer_rolls_back_document_and_both_sides() {
        let fixture = fixture(2_000_000);
        let items = [StockItemInput {
            product_id: &fixture.product_id,
            quantity_micros: 3_000_000,
        }];
        let result = complete_transfer(
            &fixture.database,
            CompleteTransfer {
                business_id: &fixture.business_id,
                from_location_id: &fixture.from_location,
                to_location_id: &fixture.to_location,
                occurred_at: "2026-09-23T10:00:00.000Z",
                user_id: &fixture.user_id,
                terminal_id: Some(&fixture.terminal_id),
                notes: None,
                items: &items,
            },
        );
        assert!(matches!(result, Err(CoreError::NegativeStock)));
        let transfers: i64 = fixture
            .database
            .query_row("SELECT count(*) FROM inventory_transfers", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(transfers, 0);
        assert_eq!(
            inventory::get_balance(
                &fixture.database,
                &fixture.business_id,
                &fixture.product_id,
                &fixture.to_location
            )
            .unwrap(),
            0
        );
    }

    #[test]
    fn wastage_and_counts_preserve_expected_and_variance() {
        let fixture = fixture(10_000_000);
        let wasted = [StockItemInput {
            product_id: &fixture.product_id,
            quantity_micros: 2_000_000,
        }];
        record_wastage(
            &fixture.database,
            RecordWastage {
                business_id: &fixture.business_id,
                location_id: &fixture.from_location,
                reason: "Expired",
                occurred_at: "2026-09-23T10:00:00.000Z",
                user_id: &fixture.user_id,
                terminal_id: Some(&fixture.terminal_id),
                notes: None,
                damage: false,
                items: &wasted,
            },
        )
        .unwrap();
        let counted = [CountedItemInput {
            product_id: &fixture.product_id,
            counted_micros: 7_000_000,
        }];
        let count = complete_stock_count(
            &fixture.database,
            CompleteStockCount {
                business_id: &fixture.business_id,
                location_id: &fixture.from_location,
                counted_at: "2026-09-23T11:00:00.000Z",
                user_id: &fixture.user_id,
                terminal_id: Some(&fixture.terminal_id),
                notes: None,
                items: &counted,
            },
        )
        .unwrap();
        assert_eq!(count.variance_item_count, 1);
        let stored: (i64, i64, i64) = fixture
            .database
            .query_row(
                "SELECT expected_micros, counted_micros, variance_micros
                 FROM stock_count_items WHERE stock_count_id = ?1",
                [&count.id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(stored, (8_000_000, 7_000_000, -1_000_000));

        let zero_count = complete_stock_count(
            &fixture.database,
            CompleteStockCount {
                business_id: &fixture.business_id,
                location_id: &fixture.from_location,
                counted_at: "2026-09-23T12:00:00.000Z",
                user_id: &fixture.user_id,
                terminal_id: Some(&fixture.terminal_id),
                notes: None,
                items: &counted,
            },
        )
        .unwrap();
        assert_eq!(zero_count.variance_item_count, 0);
        let movement_id: Option<String> = fixture
            .database
            .query_row(
                "SELECT movement_id FROM stock_count_items WHERE stock_count_id = ?1",
                [&zero_count.id],
                |row| row.get(0),
            )
            .unwrap();
        assert!(movement_id.is_none());
    }
}
