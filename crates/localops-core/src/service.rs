use crate::{CoreError, Result};
use rusqlite::Connection;

#[derive(Debug, Clone)]
pub struct Service {
    pub sellable_id: String,
    pub duration_minutes: Option<i32>,
}

pub fn create_service(
    conn: &Connection,
    sellable_id: &str,
    duration_minutes: Option<i32>,
) -> Result<()> {
    // Verify sellable exists and is a SERVICE
    let kind: String = conn
        .query_row(
            "SELECT kind FROM sellable_items WHERE id = ?1",
            [sellable_id],
            |row| row.get(0),
        )
        .map_err(|_| CoreError::SellableNotFound)?;

    if kind != "SERVICE" {
        return Err(CoreError::InvalidSellableKind);
    }

    // Use 0 as default for NULL duration_minutes since DB column is NOT NULL with DEFAULT 0
    let duration = duration_minutes.unwrap_or(0);
    
    conn.execute(
        "INSERT INTO services(sellable_id, duration_minutes) VALUES(?1, ?2)",
        (sellable_id, duration),
    )?;

    Ok(())
}

pub fn get_service(conn: &Connection, sellable_id: &str) -> Result<Option<Service>> {
    let mut stmt = conn.prepare(
        "SELECT sellable_id, duration_minutes FROM services WHERE sellable_id = ?1",
    )?;
    let service = stmt.query_row([sellable_id], |row| {
        Ok(Service {
            sellable_id: row.get(0)?,
            duration_minutes: row.get(1)?,
        })
    });

    match service {
        Ok(svc) => Ok(Some(svc)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn update_service(
    conn: &Connection,
    sellable_id: &str,
    duration_minutes: Option<i32>,
) -> Result<()> {
    let sql = "UPDATE services SET duration_minutes = ? WHERE sellable_id = ?";
    
    let rows = conn.execute(sql, rusqlite::params![duration_minutes, sellable_id])?;
    
    if rows == 0 {
        return Err(CoreError::SellableNotFound);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{business, sellable};

    #[test]
    fn creates_service_with_duration() {
        let db = crate::open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();
        
        let sellable_id = sellable::create_sellable_item(&db, &business_id, "Consultation", "SERVICE", 50000, None, false).unwrap();
        
        create_service(&db, &sellable_id, Some(60)).unwrap();
        
        let service = get_service(&db, &sellable_id).unwrap().unwrap();
        assert_eq!(service.sellable_id, sellable_id);
        assert_eq!(service.duration_minutes, Some(60));
    }

    #[test]
    fn creates_service_without_duration() {
        let db = crate::open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();
        
        let sellable_id = sellable::create_sellable_item(&db, &business_id, "Support", "SERVICE", 30000, None, false).unwrap();
        
        create_service(&db, &sellable_id, None).unwrap();
        
        let service = get_service(&db, &sellable_id).unwrap().unwrap();
        // Duration defaults to 0 when not specified (DB column is NOT NULL with DEFAULT 0)
        assert_eq!(service.duration_minutes, Some(0));
    }

    #[test]
    fn rejects_creation_for_product() {
        let db = crate::open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();
        
        let sellable_id = sellable::create_sellable_item(&db, &business_id, "Coffee", "PRODUCT", 2500, None, true).unwrap();
        
        assert!(matches!(
            create_service(&db, &sellable_id, Some(5)),
            Err(CoreError::InvalidSellableKind)
        ));
    }

    #[test]
    fn rejects_creation_for_nonexistent_sellable() {
        let db = crate::open_memory_database().unwrap();
        
        assert!(matches!(
            create_service(&db, &uuid::Uuid::now_v7().to_string(), None),
            Err(CoreError::SellableNotFound)
        ));
    }

    #[test]
    fn updates_service_duration() {
        let db = crate::open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();
        
        let sellable_id = sellable::create_sellable_item(&db, &business_id, "Training", "SERVICE", 100000, None, false).unwrap();
        
        create_service(&db, &sellable_id, Some(30)).unwrap();
        update_service(&db, &sellable_id, Some(120)).unwrap();
        
        let service = get_service(&db, &sellable_id).unwrap().unwrap();
        assert_eq!(service.duration_minutes, Some(120));
    }

    #[test]
    fn returns_none_for_nonexistent_service() {
        let db = crate::open_memory_database().unwrap();
        
        let result = get_service(&db, &uuid::Uuid::now_v7().to_string()).unwrap();
        assert!(result.is_none());
    }
}
