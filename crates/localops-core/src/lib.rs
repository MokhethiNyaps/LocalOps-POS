pub mod availability;
pub mod bootstrap;
pub mod business;
pub mod catalogue;
pub mod category;
pub mod department;
pub mod location;
pub mod product;
pub mod role;
pub mod sellable;
pub mod service;
pub mod terminal;
pub mod unit;
pub mod user;

use rusqlite::{Connection, OpenFlags};
use std::{io, path::Path, time::SystemTimeError};
use thiserror::Error;

#[cfg(test)]
use uuid::Uuid;

const MIGRATION_1: &str = include_str!("../migrations/0001_foundation.sql");

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("business name is required")]
    EmptyBusinessName,
    #[error("department name is required")]
    EmptyDepartmentName,
    #[error("category name is required")]
    EmptyCategoryName,
    #[error("category not found")]
    CategoryNotFound,
    #[error("unit code is required")]
    EmptyUnitCode,
    #[error("unit not found")]
    UnitNotFound,
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
    #[error("related records must belong to the same business")]
    CrossBusinessReference,
    #[error("location name is required")]
    EmptyLocationName,
    #[error("terminal name is required")]
    EmptyTerminalName,
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
    let version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version >= 1 {
        return Ok(());
    }
    connection.execute_batch("BEGIN IMMEDIATE")?;
    let result = (|| -> rusqlite::Result<()> {
        connection.execute_batch(MIGRATION_1)?;
        connection.execute(
            "INSERT INTO schema_migrations(version,name,checksum) VALUES(1,'foundation','embedded-v1')",
            [],
        )?;
        connection.pragma_update(None, "user_version", 1)?;
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
        assert_eq!(version, 1);
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
}
