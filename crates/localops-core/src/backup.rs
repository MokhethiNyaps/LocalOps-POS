use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use chrono::{Datelike, Utc};
use rusqlite::{Connection, OpenFlags, backup::Backup};

use crate::{CoreError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupInfo {
    pub filename: String,
    pub kind: String,
    pub size_bytes: u64,
    pub verified: bool,
}

pub fn ensure_automatic_backups(
    connection: &Connection,
    backup_dir: &Path,
) -> Result<Vec<BackupInfo>> {
    fs::create_dir_all(backup_dir)?;
    let now = Utc::now();
    let daily_prefix = format!("daily-{}", now.format("%Y-%m-%d"));
    if !has_prefix(backup_dir, &daily_prefix)? {
        create_backup(connection, backup_dir, "daily")?;
    }
    let weekly_prefix = format!("weekly-{}-W{:02}", now.year(), now.iso_week().week());
    if !has_prefix(backup_dir, &weekly_prefix)? {
        create_named_backup(connection, backup_dir, &weekly_prefix)?;
    }
    let monthly_prefix = format!("monthly-{}", now.format("%Y-%m"));
    if !has_prefix(backup_dir, &monthly_prefix)? {
        create_named_backup(connection, backup_dir, &monthly_prefix)?;
    }
    prune_kind(backup_dir, "daily-", 7)?;
    prune_kind(backup_dir, "weekly-", 4)?;
    prune_kind(backup_dir, "monthly-", 12)?;
    list_backups(backup_dir)
}

pub fn create_backup(connection: &Connection, backup_dir: &Path, kind: &str) -> Result<BackupInfo> {
    if !matches!(kind, "manual" | "daily" | "pre-restore") {
        return Err(CoreError::BackupVerificationFailed(
            "invalid backup kind".to_owned(),
        ));
    }
    let prefix = format!("{kind}-{}", Utc::now().format("%Y-%m-%d"));
    create_named_backup(connection, backup_dir, &prefix)
}

fn create_named_backup(
    connection: &Connection,
    backup_dir: &Path,
    prefix: &str,
) -> Result<BackupInfo> {
    fs::create_dir_all(backup_dir)?;
    let filename = format!("{}-{}.db", prefix, Utc::now().format("%H%M%S%.3f"));
    let destination_path = backup_dir.join(&filename);
    let mut destination = Connection::open(&destination_path)?;
    let backup = Backup::new(connection, &mut destination)?;
    backup.run_to_completion(32, Duration::from_millis(10), None)?;
    drop(backup);
    drop(destination);
    verify_backup(&destination_path)?;
    Ok(BackupInfo {
        filename,
        kind: prefix.split('-').next().unwrap_or("backup").to_owned(),
        size_bytes: fs::metadata(&destination_path)?.len(),
        verified: true,
    })
}

pub fn list_backups(backup_dir: &Path) -> Result<Vec<BackupInfo>> {
    if !backup_dir.exists() {
        return Ok(Vec::new());
    }
    let mut backups = Vec::new();
    for entry in fs::read_dir(backup_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("db") {
            continue;
        }
        let filename = entry.file_name().to_string_lossy().into_owned();
        backups.push(BackupInfo {
            kind: filename.split('-').next().unwrap_or("backup").to_owned(),
            size_bytes: entry.metadata()?.len(),
            verified: verify_backup(&path).is_ok(),
            filename,
        });
    }
    backups.sort_by(|left, right| right.filename.cmp(&left.filename));
    Ok(backups)
}

pub fn restore_backup(
    destination: &mut Connection,
    backup_dir: &Path,
    filename: &str,
) -> Result<BackupInfo> {
    let source_path = safe_backup_path(backup_dir, filename)?;
    verify_backup(&source_path)?;
    let source = Connection::open_with_flags(&source_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    create_backup(destination, backup_dir, "pre-restore")?;
    let backup = Backup::new(&source, destination)?;
    backup.run_to_completion(32, Duration::from_millis(10), None)?;
    drop(backup);
    let integrity: String = destination.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
    if integrity != "ok" {
        return Err(CoreError::IntegrityCheckFailed(integrity));
    }
    let version: i64 = destination.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version > crate::bootstrap::LATEST_SCHEMA_VERSION {
        return Err(CoreError::BackupVerificationFailed(
            "backup was created by a newer application version".to_owned(),
        ));
    }
    crate::migrate(destination)?;
    Ok(BackupInfo {
        filename: filename.to_owned(),
        kind: filename.split('-').next().unwrap_or("backup").to_owned(),
        size_bytes: fs::metadata(source_path)?.len(),
        verified: true,
    })
}

pub fn verify_backup(path: &Path) -> Result<()> {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let integrity: String = connection.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
    if integrity != "ok" {
        return Err(CoreError::BackupVerificationFailed(integrity));
    }
    let foreign_key_errors = connection
        .prepare("PRAGMA foreign_key_check")?
        .query([])?
        .mapped(|_| Ok(()))
        .count();
    if foreign_key_errors > 0 {
        return Err(CoreError::BackupVerificationFailed(format!(
            "{foreign_key_errors} foreign-key violations"
        )));
    }
    Ok(())
}

fn safe_backup_path(backup_dir: &Path, filename: &str) -> Result<PathBuf> {
    if filename.is_empty()
        || Path::new(filename)
            .file_name()
            .and_then(|value| value.to_str())
            != Some(filename)
        || !filename.ends_with(".db")
    {
        return Err(CoreError::BackupVerificationFailed(
            "invalid backup filename".to_owned(),
        ));
    }
    let path = backup_dir.join(filename);
    if !path.is_file() {
        return Err(CoreError::BackupVerificationFailed(
            "backup does not exist".to_owned(),
        ));
    }
    Ok(path)
}

fn has_prefix(backup_dir: &Path, prefix: &str) -> Result<bool> {
    Ok(fs::read_dir(backup_dir)?
        .filter_map(|entry| entry.ok())
        .any(|entry| {
            entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.starts_with(prefix) && name.ends_with(".db"))
        }))
}

fn prune_kind(backup_dir: &Path, prefix: &str, retain: usize) -> Result<()> {
    let mut paths = fs::read_dir(backup_dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|name| name.starts_with(prefix) && name.ends_with(".db"))
        })
        .collect::<Vec<_>>();
    paths.sort_by(|left, right| right.file_name().cmp(&left.file_name()));
    for path in paths.into_iter().skip(retain) {
        fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::business;
    use tempfile::tempdir;

    #[test]
    fn creates_verified_backup_and_restores_customer_data() {
        let temp = tempdir().unwrap();
        let backup_dir = temp.path().join("backups");
        let mut database = crate::open_memory_database().unwrap();
        let business_id = business::create_business(&database, "Before").unwrap();
        let backup = create_backup(&database, &backup_dir, "manual").unwrap();
        database
            .execute(
                "UPDATE businesses SET name = 'After' WHERE id = ?1",
                [&business_id],
            )
            .unwrap();
        restore_backup(&mut database, &backup_dir, &backup.filename).unwrap();
        let name: String = database
            .query_row(
                "SELECT name FROM businesses WHERE id = ?1",
                [&business_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(name, "Before");
        assert!(list_backups(&backup_dir).unwrap().len() >= 2);
    }

    #[test]
    fn rejects_paths_and_corrupt_backups_without_touching_database() {
        let temp = tempdir().unwrap();
        let backup_dir = temp.path().join("backups");
        fs::create_dir_all(&backup_dir).unwrap();
        fs::write(backup_dir.join("corrupt.db"), b"not sqlite").unwrap();
        let mut database = crate::open_memory_database().unwrap();
        business::create_business(&database, "Safe").unwrap();
        assert!(restore_backup(&mut database, &backup_dir, "../corrupt.db").is_err());
        assert!(restore_backup(&mut database, &backup_dir, "corrupt.db").is_err());
        let count: i64 = database
            .query_row("SELECT COUNT(*) FROM businesses", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }
}
