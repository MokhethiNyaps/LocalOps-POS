pub mod audit;
pub mod availability;
pub mod bootstrap;
pub mod business;
pub mod catalogue;
pub mod category;
pub mod conversion;
pub mod department;
pub mod inventory;
pub mod inventory_operations;
pub mod location;
pub mod money;
pub mod packaging;
pub mod payment;
pub mod product;
pub mod purchasing;
pub mod recipe;
pub mod refund;
pub mod role;
pub mod sales;
pub mod sellable;
pub mod service;
pub mod session;
pub mod setup;
pub mod shift;
pub mod terminal;
pub mod unit;
pub mod user;

use rusqlite::{Connection, OpenFlags};
use std::{io, path::Path, time::SystemTimeError};
use thiserror::Error;

#[cfg(test)]
use uuid::Uuid;

const MIGRATION_1: &str = include_str!("../migrations/0001_foundation.sql");
const MIGRATION_2: &str = include_str!("../migrations/0002_identity_alignment.sql");
const MIGRATION_3: &str = include_str!("../migrations/0003_inventory_integrity.sql");
const MIGRATION_4: &str = include_str!("../migrations/0004_stock_count_zero_variance.sql");
const MIGRATION_5: &str = include_str!("../migrations/0005_sale_reversals.sql");

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("business name is required")]
    EmptyBusinessName,
    #[error("department name is required")]
    EmptyDepartmentName,
    #[error("department not found")]
    DepartmentNotFound,
    #[error("category name is required")]
    EmptyCategoryName,
    #[error("category not found")]
    CategoryNotFound,
    #[error("unit code is required")]
    EmptyUnitCode,
    #[error("unit not found")]
    UnitNotFound,
    #[error("units use incompatible dimensions")]
    IncompatibleUnits,
    #[error("quantity conversion is not exact at six-decimal precision")]
    InexactQuantity,
    #[error("quantity calculation overflowed the supported range")]
    QuantityOverflow,
    #[error("product packaging not found")]
    PackagingNotFound,
    #[error("recipe not found")]
    RecipeNotFound,
    #[error("recipe quantity must be greater than zero")]
    InvalidRecipeQuantity,
    #[error("inventory movement quantity must not be zero")]
    InvalidInventoryQuantity,
    #[error("invalid inventory movement type")]
    InvalidInventoryMovementType,
    #[error("inventory movement quantity has the wrong sign for its type")]
    InvalidInventoryMovementSign,
    #[error("product is not configured for stock tracking")]
    StockNotTracked,
    #[error("inventory movement has already been posted for this reference")]
    DuplicateInventoryMovement,
    #[error("inventory movement would make stock negative")]
    NegativeStock,
    #[error("money calculation overflowed the supported range")]
    MoneyOverflow,
    #[error("supplier name is required")]
    EmptySupplierName,
    #[error("supplier not found")]
    SupplierNotFound,
    #[error("purchase must contain at least one item")]
    EmptyPurchase,
    #[error("purchase contains the same product more than once")]
    DuplicatePurchaseProduct,
    #[error("purchase total is invalid")]
    InvalidPurchaseTotal,
    #[error("inventory operation must contain at least one item")]
    EmptyInventoryOperation,
    #[error("inventory operation contains the same product more than once")]
    DuplicateInventoryProduct,
    #[error("inventory transfer locations must be different")]
    InvalidTransferLocations,
    #[error("wastage reason is required")]
    EmptyWastageReason,
    #[error("stock count quantity must not be negative")]
    InvalidStockCountQuantity,
    #[error("sale must contain at least one item")]
    EmptySale,
    #[error("sale contains a duplicate item for the same department")]
    DuplicateSaleItem,
    #[error("sellable item is not available in the selected department")]
    SaleItemNotAvailable,
    #[error("sale not found")]
    SaleNotFound,
    #[error("sale item discount exceeds its value")]
    InvalidSaleDiscount,
    #[error("completed sale payments must equal the amount due")]
    PaymentMismatch,
    #[error("cash tender is less than the allocated payment")]
    InvalidCashTender,
    #[error("non-cash payments cannot be over-tendered")]
    NonCashOverpayment,
    #[error("payment method not found")]
    PaymentMethodNotFound,
    #[error("an open shift is required")]
    OpenShiftNotFound,
    #[error("an inventory location is required for stock consumption")]
    InventoryLocationRequired,
    #[error("refund must contain at least one item")]
    EmptyRefund,
    #[error("refund reason is required")]
    EmptyRefundReason,
    #[error("refund contains a duplicate sale item or payment")]
    DuplicateRefundAllocation,
    #[error("refund quantity exceeds the remaining sold quantity")]
    RefundQuantityExceeded,
    #[error("refund payment allocation exceeds the original payment")]
    RefundPaymentExceeded,
    #[error("refund payment allocations must equal the refund total")]
    RefundPaymentMismatch,
    #[error("a void must reverse the entire unrefunded sale")]
    IncompleteVoid,
    #[error("sale has already been voided")]
    SaleAlreadyVoided,
    #[error("refund not found")]
    RefundNotFound,
    #[error("username is required")]
    EmptyUsername,
    #[error("display name is required")]
    EmptyDisplayName,
    #[error("PIN must contain 4 to 12 digits")]
    InvalidPin,
    #[error("PIN hashing failed: {0}")]
    PinHashingFailed(String),
    #[error("role name is required")]
    EmptyRoleName,
    #[error("role not found")]
    RoleNotFound,
    #[error("permission not found")]
    PermissionNotFound,
    #[error("user not found")]
    UserNotFound,
    #[error("session not found or no longer active")]
    SessionNotFound,
    #[error("related records must belong to the same business")]
    CrossBusinessReference,
    #[error("location name is required")]
    EmptyLocationName,
    #[error("terminal name is required")]
    EmptyTerminalName,
    #[error("terminal device key is required")]
    EmptyDeviceKey,
    #[error("terminal not found")]
    TerminalNotFound,
    #[error("location not found")]
    LocationNotFound,
    #[error("business not found")]
    BusinessNotFound,
    #[error("sellable item name is required")]
    EmptySellableName,
    #[error("invalid sellable item kind")]
    InvalidSellableKind,
    #[error("sellable item not found")]
    SellableNotFound,
    #[error("application data directory is unavailable")]
    AppDataDirectoryUnavailable,
    #[error("filesystem error: {0}")]
    Io(#[from] io::Error),
    #[error("system clock error: {0}")]
    Clock(#[from] SystemTimeError),
    #[error("database integrity check failed: {0}")]
    IntegrityCheckFailed(String),
    #[error("backup verification failed: {0}")]
    BackupVerificationFailed(String),
}

pub type Result<T> = std::result::Result<T, CoreError>;

pub fn open_database(path: impl AsRef<Path>) -> Result<Connection> {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
    )?;
    configure(&connection)?;
    migrate(&connection)?;
    Ok(connection)
}

pub fn open_memory_database() -> Result<Connection> {
    let connection = Connection::open_in_memory()?;
    configure(&connection)?;
    migrate(&connection)?;
    Ok(connection)
}

fn configure(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA busy_timeout = 5000;
         PRAGMA journal_mode = WAL;
         PRAGMA synchronous = FULL;",
    )
}

fn migrate(connection: &Connection) -> Result<()> {
    let mut version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version < 1 {
        apply_migration(connection, 1, "foundation", "embedded-v1", MIGRATION_1)?;
        version = 1;
    }
    if version < 2 {
        apply_migration(
            connection,
            2,
            "identity-alignment",
            "embedded-v2",
            MIGRATION_2,
        )?;
        version = 2;
    }
    if version < 3 {
        apply_migration(
            connection,
            3,
            "inventory-integrity",
            "embedded-v3",
            MIGRATION_3,
        )?;
        version = 3;
    }
    if version < 4 {
        apply_migration(
            connection,
            4,
            "stock-count-zero-variance",
            "embedded-v4",
            MIGRATION_4,
        )?;
        version = 4;
    }
    if version < 5 {
        apply_migration(connection, 5, "sale-reversals", "embedded-v5", MIGRATION_5)?;
    }
    Ok(())
}

fn apply_migration(
    connection: &Connection,
    version: i64,
    name: &str,
    checksum: &str,
    sql: &str,
) -> Result<()> {
    connection.execute_batch("BEGIN IMMEDIATE")?;
    let result = (|| -> rusqlite::Result<()> {
        connection.execute_batch(sql)?;
        connection.execute(
            "INSERT INTO schema_migrations(version,name,checksum) VALUES(?1,?2,?3)",
            (version, name, checksum),
        )?;
        connection.pragma_update(None, "user_version", version)?;
        Ok(())
    })();
    match result {
        Ok(()) => {
            connection.execute_batch("COMMIT")?;
            Ok(())
        }
        Err(error) => {
            let _ = connection.execute_batch("ROLLBACK");
            Err(error.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_enables_foreign_keys_and_records_version() {
        let db = open_memory_database().unwrap();
        let enabled: i64 = db
            .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
            .unwrap();
        let version: i64 = db
            .query_row("SELECT max(version) FROM schema_migrations", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(enabled, 1);
        assert_eq!(version, 5);
    }

    #[test]
    fn business_uses_defaults_and_uuid_v7() {
        let db = open_memory_database().unwrap();
        let id = business::create_business(&db, "Moko's Lifestyle Centre").unwrap();
        assert_eq!(Uuid::parse_str(&id).unwrap().get_version_num(), 7);
        let values: (String, String) = db
            .query_row(
                "SELECT currency, timezone FROM businesses WHERE id=?1",
                [&id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(values, ("ZAR".into(), "Africa/Johannesburg".into()));
    }

    #[test]
    fn rejects_empty_business_and_duplicate_department_name() {
        let db = open_memory_database().unwrap();
        assert!(matches!(
            business::create_business(&db, "  "),
            Err(CoreError::EmptyBusinessName)
        ));
        let business_id = business::create_business(&db, "LocalOps Test").unwrap();
        let first = Uuid::now_v7().to_string();
        db.execute(
            "INSERT INTO departments(id,business_id,name) VALUES(?1,?2,'Bar')",
            (&first, &business_id),
        )
        .unwrap();
        let second = Uuid::now_v7().to_string();
        assert!(
            db.execute(
                "INSERT INTO departments(id,business_id,name) VALUES(?1,?2,'Bar')",
                (&second, &business_id)
            )
            .is_err()
        );
    }

    #[test]
    fn database_blocks_cross_reference_to_missing_business() {
        let db = open_memory_database().unwrap();
        let result = db.execute(
            "INSERT INTO locations(id,business_id,name) VALUES(?1,?2,'Store')",
            (Uuid::now_v7().to_string(), Uuid::now_v7().to_string()),
        );
        assert!(result.is_err());
    }

    #[test]
    fn migrates_v1_terminal_identity_and_legacy_role_assignment() {
        let db = Connection::open_in_memory().unwrap();
        configure(&db).unwrap();
        db.execute_batch(MIGRATION_1).unwrap();
        db.execute(
            "INSERT INTO schema_migrations(version,name,checksum)
             VALUES(1,'foundation','embedded-v1')",
            [],
        )
        .unwrap();
        db.pragma_update(None, "user_version", 1).unwrap();

        let business_id = Uuid::now_v7().to_string();
        let role_id = Uuid::now_v7().to_string();
        let user_id = Uuid::now_v7().to_string();
        let terminal_id = Uuid::now_v7().to_string();
        db.execute(
            "INSERT INTO businesses(id,name) VALUES(?1,'Legacy Business')",
            [&business_id],
        )
        .unwrap();
        db.execute(
            "INSERT INTO roles(id,business_id,name) VALUES(?1,?2,'Owner')",
            (&role_id, &business_id),
        )
        .unwrap();
        db.execute(
            "INSERT INTO users(id,business_id,username,display_name,pin_hash,role_id)
             VALUES(?1,?2,'owner','Owner','legacy',?3)",
            (&user_id, &business_id, &role_id),
        )
        .unwrap();
        db.execute(
            "INSERT INTO terminals(id,business_id,name,code)
             VALUES(?1,?2,'Main Till','TILL-1')",
            (&terminal_id, &business_id),
        )
        .unwrap();

        migrate(&db).unwrap();

        let version: i64 = db
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        let device_key: String = db
            .query_row(
                "SELECT device_key FROM terminals WHERE id = ?1",
                [&terminal_id],
                |row| row.get(0),
            )
            .unwrap();
        let assignments: i64 = db
            .query_row(
                "SELECT count(*) FROM user_roles WHERE user_id = ?1 AND role_id = ?2",
                (&user_id, &role_id),
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(version, 5);
        assert_eq!(device_key, "TILL-1");
        assert_eq!(assignments, 1);
    }
}
