use crate::{CoreError, Result, open_database};
use rusqlite::{Connection, OpenFlags};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub const DATABASE_FILENAME: &str = "business.db";
pub const LATEST_SCHEMA_VERSION: i64 = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    pub data_dir: PathBuf,
    pub database: PathBuf,
    pub backups: PathBuf,
}

impl AppPaths {
    pub fn windows_default() -> Result<Self> {
        let root = dirs::data_local_dir()
            .ok_or(CoreError::AppDataDirectoryUnavailable)?
            .join("LocalOps POS");
        Ok(Self::from_data_dir(root))
    }

    pub fn from_data_dir(data_dir: impl Into<PathBuf>) -> Self {
        let data_dir = data_dir.into();
        Self {
            database: data_dir.join(DATABASE_FILENAME),
            backups: data_dir.join("Backups"),
            data_dir,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabaseHealth {
    pub schema_version: i64,
    pub integrity: String,
    pub database_path: PathBuf,
    pub backup_created: Option<PathBuf>,
}

pub fn bootstrap(paths: &AppPaths) -> Result<(Connection, DatabaseHealth)> {
    fs::create_dir_all(&paths.data_dir)?;
    fs::create_dir_all(&paths.backups)?;

    let backup_created = backup_before_upgrade(paths)?;
    let connection = open_database(&paths.database)?;
    let health = verify(&connection, &paths.database, backup_created)?;
    Ok((connection, health))
}

fn backup_before_upgrade(paths: &AppPaths) -> Result<Option<PathBuf>> {
    if !paths.database.exists() {
        return Ok(None);
    }
    let connection =
        Connection::open_with_flags(&paths.database, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let current: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    let integrity: String = connection.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
    drop(connection);
    if integrity != "ok" {
        return Err(CoreError::IntegrityCheckFailed(integrity));
    }
    if current >= LATEST_SCHEMA_VERSION {
        return Ok(None);
    }

    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    let destination = paths.backups.join(format!("pre-migration-{stamp}.db"));
    fs::copy(&paths.database, &destination)?;
    verify_backup(&destination)?;
    Ok(Some(destination))
}

fn verify_backup(path: &Path) -> Result<()> {
    let backup = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let result: String = backup.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
    if result != "ok" {
        return Err(CoreError::BackupVerificationFailed(result));
    }
    Ok(())
}

fn verify(
    connection: &Connection,
    database_path: &Path,
    backup_created: Option<PathBuf>,
) -> Result<DatabaseHealth> {
    let schema_version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    let integrity: String = connection.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
    if integrity != "ok" {
        return Err(CoreError::IntegrityCheckFailed(integrity));
    }
    Ok(DatabaseHealth {
        schema_version,
        integrity,
        database_path: database_path.to_owned(),
        backup_created,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::business::create_business;
    use tempfile::tempdir;

    #[test]
    fn creates_database_in_application_data_directory() {
        let temp = tempdir().unwrap();
        let paths = AppPaths::from_data_dir(temp.path().join("LocalOps POS"));
        let (_db, health) = bootstrap(&paths).unwrap();
        assert!(paths.database.exists());
        assert!(paths.backups.is_dir());
        assert_eq!(health.schema_version, LATEST_SCHEMA_VERSION);
        assert_eq!(health.integrity, "ok");
        assert_eq!(health.backup_created, None);
    }

    #[test]
    fn reopening_is_idempotent_and_preserves_customer_data() {
        let temp = tempdir().unwrap();
        let paths = AppPaths::from_data_dir(temp.path());
        let (db, _) = bootstrap(&paths).unwrap();
        let id = create_business(&db, "Persistent Business").unwrap();
        drop(db);

        let (reopened, health) = bootstrap(&paths).unwrap();
        let name: String = reopened
            .query_row("SELECT name FROM businesses WHERE id=?1", [&id], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(name, "Persistent Business");
        assert_eq!(health.backup_created, None);
    }

    #[test]
    fn verifies_backup_before_upgrading_legacy_database() {
        let temp = tempdir().unwrap();
        let paths = AppPaths::from_data_dir(temp.path());
        fs::create_dir_all(&paths.data_dir).unwrap();
        let legacy = Connection::open(&paths.database).unwrap();
        legacy.execute_batch("CREATE TABLE legacy_marker(value TEXT NOT NULL); INSERT INTO legacy_marker VALUES('safe');").unwrap();
        drop(legacy);

        let (_db, health) = bootstrap(&paths).unwrap();
        let backup_path = health.backup_created.expect("pre-migration backup");
        let backup =
            Connection::open_with_flags(&backup_path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let marker: String = backup
            .query_row("SELECT value FROM legacy_marker", [], |row| row.get(0))
            .unwrap();
        assert_eq!(marker, "safe");
    }
}
