use crate::{CoreError, Result};
use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use uuid::Uuid;

/// Business entity with default settings
#[derive(Debug, Clone, Serialize)]
pub struct Business {
    pub id: String,
    pub name: String,
    pub trading_name: Option<String>,
    pub currency: String,
    pub timezone: String,
}

/// Create a new business with default settings
pub fn create_business(connection: &Connection, name: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(CoreError::EmptyBusinessName);
    }
    let id = Uuid::now_v7().to_string();
    connection.execute(
        "INSERT INTO businesses(id, name) VALUES(?1, ?2)",
        (&id, name),
    )?;
    Ok(id)
}

/// Get a business by ID
pub fn get_business(connection: &Connection, id: &str) -> Result<Option<Business>> {
    let mut stmt = connection.prepare(
        "SELECT id, name, trading_name, currency, timezone FROM businesses WHERE id = ?1",
    )?;
    let business = stmt
        .query_row([&id], |row| {
            Ok(Business {
                id: row.get(0)?,
                name: row.get(1)?,
                trading_name: row.get(2)?,
                currency: row.get(3)?,
                timezone: row.get(4)?,
            })
        })
        .optional()?;
    Ok(business)
}

/// Update business trading name
pub fn update_business_trading_name(
    connection: &Connection,
    id: &str,
    trading_name: Option<&str>,
) -> Result<()> {
    connection.execute(
        "UPDATE businesses SET trading_name = ?1 WHERE id = ?2",
        (trading_name, id),
    )?;
    Ok(())
}

/// List all active businesses
pub fn list_businesses(connection: &Connection) -> Result<Vec<Business>> {
    let mut stmt = connection.prepare(
        "SELECT id, name, trading_name, currency, timezone FROM businesses WHERE active = 1 ORDER BY name"
    )?;
    let businesses = stmt.query_map([], |row| {
        Ok(Business {
            id: row.get(0)?,
            name: row.get(1)?,
            trading_name: row.get(2)?,
            currency: row.get(3)?,
            timezone: row.get(4)?,
        })
    })?;
    businesses
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_memory_database;

    #[test]
    fn creates_business_with_defaults() {
        let db = open_memory_database().unwrap();
        let id = create_business(&db, "Test Business").unwrap();

        let business = get_business(&db, &id).unwrap().unwrap();
        assert_eq!(business.name, "Test Business");
        assert_eq!(business.currency, "ZAR");
        assert_eq!(business.timezone, "Africa/Johannesburg");
        assert!(business.trading_name.is_none());
    }

    #[test]
    fn rejects_empty_business_name() {
        let db = open_memory_database().unwrap();
        assert!(matches!(
            create_business(&db, "   "),
            Err(CoreError::EmptyBusinessName)
        ));
    }

    #[test]
    fn updates_trading_name() {
        let db = open_memory_database().unwrap();
        let id = create_business(&db, "Original Name").unwrap();

        update_business_trading_name(&db, &id, Some("Trading As")).unwrap();
        let business = get_business(&db, &id).unwrap().unwrap();
        assert_eq!(business.trading_name, Some("Trading As".to_string()));

        update_business_trading_name(&db, &id, None).unwrap();
        let business = get_business(&db, &id).unwrap().unwrap();
        assert!(business.trading_name.is_none());
    }

    #[test]
    fn lists_active_businesses() {
        let db = open_memory_database().unwrap();
        create_business(&db, "Business A").unwrap();
        create_business(&db, "Business B").unwrap();

        let businesses = list_businesses(&db).unwrap();
        assert_eq!(businesses.len(), 2);
        assert_eq!(businesses[0].name, "Business A");
        assert_eq!(businesses[1].name, "Business B");
    }
}
