use crate::{CoreError, Result};
use rusqlite::Connection;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Unit {
    pub id: String,
    pub business_id: String,
    pub code: String,
    pub name: String,
    pub dimension: String,
    pub scale_num: i64,
    pub scale_den: i64,
    pub decimal_places: i32,
    pub active: bool,
}

pub fn create_unit(
    conn: &Connection,
    business_id: &str,
    code: &str,
    name: &str,
    dimension: &str,
    scale_num: i64,
    scale_den: i64,
    decimal_places: i32,
) -> Result<String> {
    let code = code.trim();
    if code.is_empty() {
        return Err(CoreError::EmptyUnitCode);
    }

    // Verify business exists
    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM businesses WHERE id = ?1",
            [business_id],
            |row| row.get(0),
        )
        .unwrap_or(false);

    if !exists {
        return Err(CoreError::BusinessNotFound);
    }

    // Validate scale
    if scale_num <= 0 || scale_den <= 0 {
        return Err(CoreError::Database(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT_CHECK),
            Some("scale must be positive".to_string()),
        )));
    }

    // Validate decimal places
    if decimal_places < 0 || decimal_places > 6 {
        return Err(CoreError::Database(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT_CHECK),
            Some("decimal places must be 0-6".to_string()),
        )));
    }

    let id = Uuid::now_v7().to_string();
    conn.execute(
        "INSERT INTO units(id, business_id, code, name, dimension, scale_num, scale_den, decimal_places, active) 
         VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1)",
        (&id, business_id, code, name, dimension, scale_num, scale_den, decimal_places),
    )?;

    Ok(id)
}

pub fn get_unit(conn: &Connection, id: &str) -> Result<Option<Unit>> {
    let mut stmt = conn.prepare(
        "SELECT id, business_id, code, name, dimension, scale_num, scale_den, decimal_places, active FROM units WHERE id = ?1",
    )?;
    let unit = stmt.query_row([id], |row| {
        Ok(Unit {
            id: row.get(0)?,
            business_id: row.get(1)?,
            code: row.get(2)?,
            name: row.get(3)?,
            dimension: row.get(4)?,
            scale_num: row.get(5)?,
            scale_den: row.get(6)?,
            decimal_places: row.get(7)?,
            active: row.get::<_, i32>(8)? == 1,
        })
    });

    match unit {
        Ok(u) => Ok(Some(u)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn get_units_by_business(
    conn: &Connection,
    business_id: &str,
    active_only: bool,
) -> Result<Vec<Unit>> {
    let sql = if active_only {
        "SELECT id, business_id, code, name, dimension, scale_num, scale_den, decimal_places, active 
         FROM units WHERE business_id = ?1 AND active = 1 ORDER BY code"
    } else {
        "SELECT id, business_id, code, name, dimension, scale_num, scale_den, decimal_places, active 
         FROM units WHERE business_id = ?1 ORDER BY code"
    };

    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([business_id], |row| {
        Ok(Unit {
            id: row.get(0)?,
            business_id: row.get(1)?,
            code: row.get(2)?,
            name: row.get(3)?,
            dimension: row.get(4)?,
            scale_num: row.get(5)?,
            scale_den: row.get(6)?,
            decimal_places: row.get(7)?,
            active: row.get::<_, i32>(8)? == 1,
        })
    })?;

    let mut units = Vec::new();
    for row in rows {
        units.push(row?);
    }

    Ok(units)
}

pub fn update_unit(
    conn: &Connection,
    id: &str,
    code: Option<&str>,
    name: Option<&str>,
    dimension: Option<&str>,
    scale_num: Option<i64>,
    scale_den: Option<i64>,
    decimal_places: Option<i32>,
) -> Result<()> {
    let mut updates = Vec::new();
    let mut params_vec: Vec<String> = Vec::new();

    if let Some(c) = code {
        let trimmed = c.trim();
        if trimmed.is_empty() {
            return Err(CoreError::EmptyUnitCode);
        }
        updates.push("code = ?");
        params_vec.push(trimmed.to_string());
    }

    if let Some(n) = name {
        updates.push("name = ?");
        params_vec.push(n.to_string());
    }

    if let Some(d) = dimension {
        updates.push("dimension = ?");
        params_vec.push(d.to_string());
    }

    if let Some(sn) = scale_num {
        if sn <= 0 {
            return Err(CoreError::Database(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT_CHECK),
                Some("scale_num must be positive".to_string()),
            )));
        }
        updates.push("scale_num = ?");
        params_vec.push(sn.to_string());
    }

    if let Some(sd) = scale_den {
        if sd <= 0 {
            return Err(CoreError::Database(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT_CHECK),
                Some("scale_den must be positive".to_string()),
            )));
        }
        updates.push("scale_den = ?");
        params_vec.push(sd.to_string());
    }

    if let Some(dp) = decimal_places {
        if dp < 0 || dp > 6 {
            return Err(CoreError::Database(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT_CHECK),
                Some("decimal places must be 0-6".to_string()),
            )));
        }
        updates.push("decimal_places = ?");
        params_vec.push(dp.to_string());
    }

    if updates.is_empty() {
        return Ok(());
    }

    params_vec.push(id.to_string());
    let sql = format!("UPDATE units SET {} WHERE id = ?", updates.join(", "));
    
    let rows = conn.execute(&sql, rusqlite::params_from_iter(params_vec.iter()))?;
    
    if rows == 0 {
        return Err(CoreError::UnitNotFound);
    }

    Ok(())
}

pub fn deactivate_unit(conn: &Connection, id: &str) -> Result<()> {
    let result = conn.execute("UPDATE units SET active = 0 WHERE id = ?1", [id]);
    
    match result {
        Ok(rows) if rows == 0 => Err(CoreError::UnitNotFound),
        Ok(_) => Ok(()),
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::business;

    #[test]
    fn creates_unit_with_required_fields() {
        let db = crate::open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();
        
        let id = create_unit(&db, &business_id, "PCS", "Piece", "quantity", 1, 1, 0).unwrap();
        assert_eq!(Uuid::parse_str(&id).unwrap().get_version_num(), 7);
        
        let unit = get_unit(&db, &id).unwrap().unwrap();
        assert_eq!(unit.code, "PCS");
        assert_eq!(unit.name, "Piece");
        assert_eq!(unit.dimension, "quantity");
        assert_eq!(unit.scale_num, 1);
        assert_eq!(unit.scale_den, 1);
        assert_eq!(unit.decimal_places, 0);
        assert!(unit.active);
    }

    #[test]
    fn rejects_empty_unit_code() {
        let db = crate::open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();
        
        assert!(matches!(
            create_unit(&db, &business_id, "  ", "Piece", "quantity", 1, 1, 0),
            Err(CoreError::EmptyUnitCode)
        ));
    }

    #[test]
    fn validates_business_exists() {
        let db = crate::open_memory_database().unwrap();
        
        assert!(matches!(
            create_unit(&db, &Uuid::now_v7().to_string(), "PCS", "Piece", "quantity", 1, 1, 0),
            Err(CoreError::BusinessNotFound)
        ));
    }

    #[test]
    fn lists_units_ordered_by_code() {
        let db = crate::open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();
        
        create_unit(&db, &business_id, "ZEBRA", "Zebra", "quantity", 1, 1, 0).unwrap();
        create_unit(&db, &business_id, "APPLE", "Apple", "quantity", 1, 1, 0).unwrap();
        create_unit(&db, &business_id, "MANGO", "Mango", "quantity", 1, 1, 0).unwrap();
        
        let units = get_units_by_business(&db, &business_id, true).unwrap();
        assert_eq!(units.len(), 3);
        assert_eq!(units[0].code, "APPLE");
        assert_eq!(units[1].code, "MANGO");
        assert_eq!(units[2].code, "ZEBRA");
    }

    #[test]
    fn filters_inactive_units() {
        let db = crate::open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();
        
        let id1 = create_unit(&db, &business_id, "ACTIVE", "Active", "quantity", 1, 1, 0).unwrap();
        let id2 = create_unit(&db, &business_id, "INACTIVE", "Inactive", "quantity", 1, 1, 0).unwrap();
        deactivate_unit(&db, &id2).unwrap();
        
        let active = get_units_by_business(&db, &business_id, true).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, id1);
        
        let all = get_units_by_business(&db, &business_id, false).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn updates_unit_details() {
        let db = crate::open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();
        
        let id = create_unit(&db, &business_id, "OLD", "Old Name", "quantity", 1, 1, 0).unwrap();
        
        update_unit(&db, &id, Some("NEW"), Some("New Name"), Some("weight"), Some(1000), Some(1), Some(3)).unwrap();
        
        let unit = get_unit(&db, &id).unwrap().unwrap();
        assert_eq!(unit.code, "NEW");
        assert_eq!(unit.name, "New Name");
        assert_eq!(unit.dimension, "weight");
        assert_eq!(unit.scale_num, 1000);
        assert_eq!(unit.scale_den, 1);
        assert_eq!(unit.decimal_places, 3);
    }

    #[test]
    fn isolates_units_by_business() {
        let db = crate::open_memory_database().unwrap();
        let biz1 = business::create_business(&db, "Business 1").unwrap();
        let biz2 = business::create_business(&db, "Business 2").unwrap();
        
        let id1 = create_unit(&db, &biz1, "PCS1", "Piece 1", "quantity", 1, 1, 0).unwrap();
        let id2 = create_unit(&db, &biz2, "PCS2", "Piece 2", "quantity", 1, 1, 0).unwrap();
        
        let units1 = get_units_by_business(&db, &biz1, false).unwrap();
        assert_eq!(units1.len(), 1);
        assert_eq!(units1[0].id, id1);
        
        let units2 = get_units_by_business(&db, &biz2, false).unwrap();
        assert_eq!(units2.len(), 1);
        assert_eq!(units2[0].id, id2);
    }

    #[test]
    fn creates_unit_with_fractional_scale() {
        let db = crate::open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();
        
        // Create a unit where 1 base unit = 0.001 of this unit (e.g., milligrams)
        let id = create_unit(&db, &business_id, "MG", "Milligram", "weight", 1, 1000, 3).unwrap();
        
        let unit = get_unit(&db, &id).unwrap().unwrap();
        assert_eq!(unit.code, "MG");
        assert_eq!(unit.scale_num, 1);
        assert_eq!(unit.scale_den, 1000);
        assert_eq!(unit.decimal_places, 3);
    }
}
