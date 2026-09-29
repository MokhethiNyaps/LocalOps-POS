pub mod access;
pub mod audit;
pub mod availability;
pub mod backup;
pub mod bootstrap;
pub mod business;
pub mod cashier;
pub mod catalogue;
pub mod category;
pub mod conversion;
pub mod department;
pub mod employee;
pub mod expense;
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
pub mod report;
pub mod role;
pub mod safety;
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
const MIGRATION_6: &str = include_str!("../migrations/0006_shift_expenses.sql");
const MIGRATION_7: &str =
    include_str!("../migrations/0007_owner_cashier_access_control.sql");

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
    #[error("shift is not open")]
    ShiftNotOpen,
    #[error("actual cash balance must not be negative")]
    InvalidActualBalance,
    #[error("expense category name is required")]
    EmptyExpenseCategoryName,
    #[error("expense description is required")]
    EmptyExpenseDescription,
    #[error("expense amount must be greater than zero")]
    InvalidExpenseAmount,
    #[error("expense category not found")]
    ExpenseCategoryNotFound,
    #[error("expense not found")]
    ExpenseNotFound,
    #[error("expense has already been voided")]
    ExpenseAlreadyVoided,
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
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("the requested record is not available for this session")]
    ScopeDenied,
    #[error("this terminal already has an open shift owned by another employee")]
    ShiftOwnedByAnotherUser,
    #[error("a terminal department is required for this operation")]
    TerminalDepartmentRequired,
    #[error("the business must keep at least one active Owner")]
    LastActiveOwnerRequired,
    #[error("initial setup has already been completed")]
    SetupAlreadyCompleted,
    #[error("username is already in use")]
    DuplicateUsername,
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
        version = 5;
    }
    if version < 6 {
        apply_migration(connection, 6, "shift-expenses", "embedded-v6", MIGRATION_6)?;
        version = 6;
    }
    if version < 7 {
        apply_migration(
            connection,
            7,
            "owner-cashier-access-control",
            "embedded-v7",
            MIGRATION_7,
        )?;
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
        assert_eq!(version, 7);
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
    fn migrates_every_supported_schema_version_to_latest() {
        let later_migrations = [
            (2, "identity-alignment", "embedded-v2", MIGRATION_2),
            (3, "inventory-integrity", "embedded-v3", MIGRATION_3),
            (4, "stock-count-zero-variance", "embedded-v4", MIGRATION_4),
            (5, "sale-reversals", "embedded-v5", MIGRATION_5),
            (6, "shift-expenses", "embedded-v6", MIGRATION_6),
            (
                7,
                "owner-cashier-access-control",
                "embedded-v7",
                MIGRATION_7,
            ),
        ];
        for starting_version in 1..=7 {
            let database = Connection::open_in_memory().unwrap();
            configure(&database).unwrap();
            database.execute_batch(MIGRATION_1).unwrap();
            database
                .execute(
                    "INSERT INTO schema_migrations(version,name,checksum)
                     VALUES(1,'foundation','embedded-v1')",
                    [],
                )
                .unwrap();
            database.pragma_update(None, "user_version", 1).unwrap();
            for (version, name, checksum, sql) in later_migrations
                .iter()
                .copied()
                .filter(|(version, _, _, _)| *version <= starting_version)
            {
                apply_migration(&database, version, name, checksum, sql).unwrap();
            }
            migrate(&database).unwrap();
            let final_version: i64 = database
                .query_row("PRAGMA user_version", [], |row| row.get(0))
                .unwrap();
            let integrity: String = database
                .query_row("PRAGMA quick_check", [], |row| row.get(0))
                .unwrap();
            assert_eq!(final_version, 7, "starting at version {starting_version}");
            assert_eq!(integrity, "ok");
        }
    }

    /// Section 5.3 legacy permission compatibility: explicitly granted
    /// management authority survives the split into narrow permissions, while
    /// accidental session-only reads are not converted into new entitlements.
    #[test]
    fn migrates_legacy_roles_into_the_narrow_permission_model() {
        let database = Connection::open_in_memory().unwrap();
        configure(&database).unwrap();
        database.execute_batch(MIGRATION_1).unwrap();
        database
            .execute(
                "INSERT INTO schema_migrations(version,name,checksum)
                 VALUES(1,'foundation','embedded-v1')",
                [],
            )
            .unwrap();
        database.pragma_update(None, "user_version", 1).unwrap();
        let business_id = Uuid::now_v7().to_string();
        database
            .execute(
                "INSERT INTO businesses(id,name) VALUES(?1,'Legacy Business')",
                [&business_id],
            )
            .unwrap();
        let role = |name: &str, system: bool, permissions: &[&str]| {
            let id = Uuid::now_v7().to_string();
            database
                .execute(
                    "INSERT INTO roles(id,business_id,name,system) VALUES(?1,?2,?3,?4)",
                    (&id, &business_id, name, system),
                )
                .unwrap();
            for permission in permissions {
                database
                    .execute(
                        "INSERT INTO role_permissions(role_id,permission_code) VALUES(?1,?2)",
                        (&id, permission),
                    )
                    .unwrap();
            }
            id
        };
        let owner = role("Owner", true, &["business.manage"]);
        let supervisor = role("Supervisor", false, &["shifts.manage", "reports.view"]);
        let stock = role("Stock Controller", false, &["inventory.manage", "products.manage"]);
        let viewer = role("Floor Viewer", false, &[]);

        migrate(&database).unwrap();

        let has = |role_id: &str, permission: &str| -> bool {
            database
                .query_row(
                    "SELECT count(*) FROM role_permissions WHERE role_id = ?1 AND permission_code = ?2",
                    (role_id, permission),
                    |row| row.get::<_, i64>(0),
                )
                .unwrap()
                > 0
        };

        // Owner receives everything, including the newly introduced permissions.
        let permissions: i64 = database
            .query_row("SELECT count(*) FROM permissions", [], |row| row.get(0))
            .unwrap();
        let owner_grants: i64 = database
            .query_row(
                "SELECT count(*) FROM role_permissions WHERE role_id = ?1",
                [&owner],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(owner_grants, permissions);
        let owner_key: Option<String> = database
            .query_row("SELECT role_key FROM roles WHERE id = ?1", [&owner], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(owner_key.as_deref(), Some("OWNER"));

        // shifts.manage keeps its intended split capabilities.
        for permission in [
            "shifts.open_own",
            "shifts.close_own",
            "shifts.close_any",
            "shifts.view_all",
            "shifts.expected_cash.view",
            "shifts.variance.view",
            "reports.export",
        ] {
            assert!(has(&supervisor, permission), "supervisor lost {permission}");
        }
        assert!(!has(&supervisor, "products.cost.view"));
        assert!(!has(&supervisor, "backups.manage"));

        // Inventory/product managers keep the visibility their screens need.
        for permission in [
            "inventory.quantity.view",
            "inventory.value.view",
            "suppliers.manage",
            "purchases.manage",
            "products.cost.view",
            "pos.catalog.view",
        ] {
            assert!(has(&stock, permission), "stock role lost {permission}");
        }
        assert!(!has(&stock, "shifts.close_any"));

        // Accidental session-only reads become nothing.
        let viewer_grants: i64 = database
            .query_row(
                "SELECT count(*) FROM role_permissions WHERE role_id = ?1",
                [&viewer],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(viewer_grants, 0);

        // Every business gains a whitelist Cashier system role.
        let cashier: String = database
            .query_row(
                "SELECT id FROM roles WHERE business_id = ?1 AND role_key = 'CASHIER'",
                [&business_id],
                |row| row.get(0),
            )
            .unwrap();
        let cashier_grants: Vec<String> = database
            .prepare(
                "SELECT permission_code FROM role_permissions WHERE role_id = ?1 ORDER BY permission_code",
            )
            .unwrap()
            .query_map([&cashier], |row| row.get(0))
            .unwrap()
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap();
        let mut expected = role::CASHIER_PERMISSIONS
            .iter()
            .map(|value| (*value).to_owned())
            .collect::<Vec<_>>();
        expected.sort();
        assert_eq!(cashier_grants, expected);
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
        assert_eq!(version, 6);
        assert_eq!(device_key, "TILL-1");
        assert_eq!(assignments, 1);
    }
}
