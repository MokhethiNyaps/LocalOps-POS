use std::collections::{BTreeMap, BTreeSet};

use rusqlite::{Connection, OptionalExtension};
use uuid::Uuid;

use crate::{CoreError, Result, audit};

const MOVEMENT_TYPES: &[&str] = &[
    "PURCHASE",
    "SALE",
    "WASTE",
    "DAMAGE",
    "ADJUSTMENT",
    "TRANSFER_OUT",
    "TRANSFER_IN",
    "COUNT",
    "OPENING",
    "CONSUMPTION",
];
const POSITIVE_TYPES: &[&str] = &["PURCHASE", "TRANSFER_IN", "OPENING"];
const NEGATIVE_TYPES: &[&str] = &["SALE", "WASTE", "DAMAGE", "TRANSFER_OUT", "CONSUMPTION"];

#[derive(Debug, Clone, Copy)]
pub struct NewMovement<'a> {
    pub business_id: &'a str,
    pub product_id: &'a str,
    pub location_id: &'a str,
    pub quantity_micros: i64,
    pub movement_type: &'a str,
    pub occurred_at: &'a str,
    pub user_id: &'a str,
    pub terminal_id: Option<&'a str>,
    pub reference_type: &'a str,
    pub reference_id: &'a str,
    pub correlation_id: &'a str,
    pub notes: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostedMovement {
    pub movement_id: String,
    pub balance_micros: i64,
    pub balance_version: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryBalance {
    pub business_id: String,
    pub product_id: String,
    pub product_name: String,
    pub location_id: String,
    pub location_name: String,
    pub quantity_micros: i64,
    pub version: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconciliationDifference {
    pub business_id: String,
    pub product_id: String,
    pub location_id: String,
    pub cached_micros: i64,
    pub ledger_micros: i64,
}

pub fn post_movement(connection: &Connection, input: NewMovement<'_>) -> Result<PostedMovement> {
    validate_movement(input.movement_type, input.quantity_micros)?;
    if !connection.is_autocommit() {
        return post_movement_rows(connection, input);
    }
    let transaction = connection.unchecked_transaction()?;
    let result = post_movement_rows(&transaction, input)?;
    transaction.commit()?;
    Ok(result)
}

fn post_movement_rows(connection: &Connection, input: NewMovement<'_>) -> Result<PostedMovement> {
    let valid_context = connection
        .query_row(
            "SELECT p.track_stock
             FROM products p
             JOIN sellable_items s ON s.id = p.sellable_id
             JOIN locations l ON l.business_id = s.business_id AND l.id = ?3 AND l.active = 1
             JOIN users u ON u.business_id = s.business_id AND u.id = ?4 AND u.active = 1
             WHERE p.sellable_id = ?2 AND s.business_id = ?1 AND s.active = 1",
            (
                input.business_id,
                input.product_id,
                input.location_id,
                input.user_id,
            ),
            |row| row.get::<_, bool>(0),
        )
        .optional()?
        .ok_or(CoreError::CrossBusinessReference)?;
    if !valid_context {
        return Err(CoreError::StockNotTracked);
    }
    let duplicate = connection
        .query_row(
            "SELECT 1 FROM inventory_movements
             WHERE business_id = ?1 AND product_id = ?2 AND location_id = ?3
               AND type = ?4 AND reference_type = ?5 AND reference_id = ?6",
            (
                input.business_id,
                input.product_id,
                input.location_id,
                input.movement_type,
                input.reference_type,
                input.reference_id,
            ),
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if duplicate {
        return Err(CoreError::DuplicateInventoryMovement);
    }

    let balance_changed = connection.execute(
        "UPDATE inventory_balances
         SET quantity_micros = quantity_micros + ?4,
             version = version + 1,
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
         WHERE business_id = ?1 AND product_id = ?2 AND location_id = ?3
           AND quantity_micros + ?4 >= 0",
        (
            input.business_id,
            input.product_id,
            input.location_id,
            input.quantity_micros,
        ),
    )?;
    if balance_changed == 0 {
        if input.quantity_micros < 0 {
            return Err(CoreError::NegativeStock);
        }
        connection.execute(
            "INSERT INTO inventory_balances(
                 business_id, product_id, location_id, quantity_micros, version
             ) VALUES(?1, ?2, ?3, ?4, 1)",
            (
                input.business_id,
                input.product_id,
                input.location_id,
                input.quantity_micros,
            ),
        )?;
    }

    let movement_id = Uuid::now_v7().to_string();
    connection.execute(
        "INSERT INTO inventory_movements(
             id, business_id, product_id, location_id, quantity_micros, type,
             occurred_at, user_id, reference_type, reference_id, correlation_id, notes
         ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        (
            &movement_id,
            input.business_id,
            input.product_id,
            input.location_id,
            input.quantity_micros,
            input.movement_type,
            input.occurred_at,
            input.user_id,
            input.reference_type,
            input.reference_id,
            input.correlation_id,
            input.notes,
        ),
    )?;
    let new_json = format!(
        "{{\"quantityMicros\":{},\"type\":\"{}\",\"locationId\":\"{}\"}}",
        input.quantity_micros, input.movement_type, input.location_id
    );
    audit::record_event(
        connection,
        audit::NewAuditEvent {
            business_id: input.business_id,
            user_id: Some(input.user_id),
            terminal_id: input.terminal_id,
            action: "INVENTORY_MOVEMENT_POSTED",
            entity_type: "inventory_movement",
            entity_id: &movement_id,
            old_json: None,
            new_json: Some(&new_json),
        },
    )?;
    let (balance_micros, balance_version) = connection.query_row(
        "SELECT quantity_micros, version FROM inventory_balances
         WHERE business_id = ?1 AND product_id = ?2 AND location_id = ?3",
        (input.business_id, input.product_id, input.location_id),
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    Ok(PostedMovement {
        movement_id,
        balance_micros,
        balance_version,
    })
}

fn validate_movement(movement_type: &str, quantity_micros: i64) -> Result<()> {
    if quantity_micros == 0 {
        return Err(CoreError::InvalidInventoryQuantity);
    }
    if !MOVEMENT_TYPES.contains(&movement_type) {
        return Err(CoreError::InvalidInventoryMovementType);
    }
    if (POSITIVE_TYPES.contains(&movement_type) && quantity_micros < 0)
        || (NEGATIVE_TYPES.contains(&movement_type) && quantity_micros > 0)
    {
        return Err(CoreError::InvalidInventoryMovementSign);
    }
    Ok(())
}

pub fn get_balance(
    connection: &Connection,
    business_id: &str,
    product_id: &str,
    location_id: &str,
) -> Result<i64> {
    Ok(connection
        .query_row(
            "SELECT quantity_micros FROM inventory_balances
             WHERE business_id = ?1 AND product_id = ?2 AND location_id = ?3",
            (business_id, product_id, location_id),
            |row| row.get(0),
        )
        .optional()?
        .unwrap_or(0))
}

pub fn list_balances(connection: &Connection, business_id: &str) -> Result<Vec<InventoryBalance>> {
    let mut statement = connection.prepare(
        "SELECT b.business_id, b.product_id, s.name, b.location_id, l.name,
                b.quantity_micros, b.version
         FROM inventory_balances b
         JOIN sellable_items s ON s.id = b.product_id
         JOIN locations l ON l.id = b.location_id
         WHERE b.business_id = ?1
         ORDER BY s.name, l.name",
    )?;
    statement
        .query_map([business_id], |row| {
            Ok(InventoryBalance {
                business_id: row.get(0)?,
                product_id: row.get(1)?,
                product_name: row.get(2)?,
                location_id: row.get(3)?,
                location_name: row.get(4)?,
                quantity_micros: row.get(5)?,
                version: row.get(6)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn reconcile_balances(
    connection: &Connection,
    business_id: &str,
) -> Result<Vec<ReconciliationDifference>> {
    type Key = (String, String, String);
    let mut cached: BTreeMap<Key, i64> = BTreeMap::new();
    let mut statement = connection.prepare(
        "SELECT business_id, product_id, location_id, quantity_micros
         FROM inventory_balances WHERE business_id = ?1",
    )?;
    for row in statement.query_map([business_id], |row| {
        Ok(((row.get(0)?, row.get(1)?, row.get(2)?), row.get(3)?))
    })? {
        let (key, quantity) = row?;
        cached.insert(key, quantity);
    }
    let mut ledger: BTreeMap<Key, i64> = BTreeMap::new();
    let mut statement = connection.prepare(
        "SELECT business_id, product_id, location_id, sum(quantity_micros)
         FROM inventory_movements WHERE business_id = ?1
         GROUP BY business_id, product_id, location_id",
    )?;
    for row in statement.query_map([business_id], |row| {
        Ok(((row.get(0)?, row.get(1)?, row.get(2)?), row.get(3)?))
    })? {
        let (key, quantity) = row?;
        ledger.insert(key, quantity);
    }
    let keys = cached
        .keys()
        .chain(ledger.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    Ok(keys
        .into_iter()
        .filter_map(|(business_id, product_id, location_id)| {
            let key = (business_id.clone(), product_id.clone(), location_id.clone());
            let cached_micros = cached.get(&key).copied().unwrap_or(0);
            let ledger_micros = ledger.get(&key).copied().unwrap_or(0);
            (cached_micros != ledger_micros).then_some(ReconciliationDifference {
                business_id,
                product_id,
                location_id,
                cached_micros,
                ledger_micros,
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{catalogue, setup, unit};

    struct Fixture {
        database: Connection,
        business_id: String,
        location_id: String,
        user_id: String,
        terminal_id: String,
        product_id: String,
    }

    fn fixture(track_stock: bool) -> Fixture {
        let database = crate::open_memory_database().unwrap();
        let setup = setup::complete_initial_setup(
            &database,
            setup::InitialSetup {
                business_name: "Test",
                department_name: "Shop",
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
                name: "Stock item",
                price_minor: 100,
                category_id: None,
                taxable: true,
                base_unit_id: &each,
                cost_minor: 50,
                track_stock,
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
            product_id,
        }
    }

    fn movement<'a>(
        fixture: &'a Fixture,
        quantity: i64,
        kind: &'a str,
        reference: &'a str,
    ) -> NewMovement<'a> {
        NewMovement {
            business_id: &fixture.business_id,
            product_id: &fixture.product_id,
            location_id: &fixture.location_id,
            quantity_micros: quantity,
            movement_type: kind,
            occurred_at: "2026-09-23T10:00:00.000Z",
            user_id: &fixture.user_id,
            terminal_id: Some(&fixture.terminal_id),
            reference_type: "test",
            reference_id: reference,
            correlation_id: reference,
            notes: None,
        }
    }

    #[test]
    fn movement_ledger_and_cached_balance_advance_atomically() {
        let fixture = fixture(true);
        let opening = post_movement(
            &fixture.database,
            movement(&fixture, 10_000_000, "OPENING", "open"),
        )
        .unwrap();
        let sale = post_movement(
            &fixture.database,
            movement(&fixture, -3_000_000, "SALE", "sale"),
        )
        .unwrap();
        assert_eq!(opening.balance_version, 1);
        assert_eq!(sale.balance_micros, 7_000_000);
        assert_eq!(sale.balance_version, 2);
        assert!(
            reconcile_balances(&fixture.database, &fixture.business_id)
                .unwrap()
                .is_empty()
        );
        let facts: (i64, i64) = fixture
            .database
            .query_row(
                "SELECT (SELECT count(*) FROM inventory_movements),
                    (SELECT count(*) FROM audit_logs WHERE action = 'INVENTORY_MOVEMENT_POSTED')",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(facts, (2, 2));
    }

    #[test]
    fn negative_stock_failure_leaves_no_partial_records() {
        let fixture = fixture(true);
        let result = post_movement(
            &fixture.database,
            movement(&fixture, -1_000_000, "SALE", "sale"),
        );
        assert!(matches!(result, Err(CoreError::NegativeStock)));
        assert_eq!(
            get_balance(
                &fixture.database,
                &fixture.business_id,
                &fixture.product_id,
                &fixture.location_id
            )
            .unwrap(),
            0
        );
        let movement_count: i64 = fixture
            .database
            .query_row("SELECT count(*) FROM inventory_movements", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(movement_count, 0);
    }

    #[test]
    fn rejects_duplicate_references_and_untracked_products() {
        let tracked = fixture(true);
        post_movement(
            &tracked.database,
            movement(&tracked, 1_000_000, "OPENING", "same"),
        )
        .unwrap();
        assert!(matches!(
            post_movement(
                &tracked.database,
                movement(&tracked, 1_000_000, "OPENING", "same")
            ),
            Err(CoreError::DuplicateInventoryMovement)
        ));
        let untracked = fixture(false);
        assert!(matches!(
            post_movement(
                &untracked.database,
                movement(&untracked, 1_000_000, "OPENING", "open")
            ),
            Err(CoreError::StockNotTracked)
        ));
    }

    #[test]
    fn reconciliation_detects_cache_corruption() {
        let fixture = fixture(true);
        post_movement(
            &fixture.database,
            movement(&fixture, 4_000_000, "OPENING", "open"),
        )
        .unwrap();
        fixture
            .database
            .execute("UPDATE inventory_balances SET quantity_micros = 99", [])
            .unwrap();
        let differences = reconcile_balances(&fixture.database, &fixture.business_id).unwrap();
        assert_eq!(differences.len(), 1);
        assert_eq!(differences[0].cached_micros, 99);
        assert_eq!(differences[0].ledger_micros, 4_000_000);
    }
}
