use crate::{CoreError, Result};
use rusqlite::{Connection, OptionalExtension};
use uuid::Uuid;

/// Location entity representing a physical business location
#[derive(Debug, Clone)]
pub struct Location {
    pub id: String,
    pub business_id: String,
    pub name: String,
    pub code: Option<String>,
    pub address: Option<String>,
    pub contact_phone: Option<String>,
    pub contact_email: Option<String>,
    pub active: bool,
}

/// Create a new location
pub fn create_location(
    connection: &Connection,
    business_id: &str,
    name: &str,
    code: Option<&str>,
) -> Result<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(CoreError::EmptyLocationName);
    }

    // Verify business exists
    let exists: bool = connection
        .query_row(
            "SELECT 1 FROM businesses WHERE id = ?1 AND active = 1",
            [business_id],
            |row| row.get::<_, i32>(0),
        )
        .optional()?
        .is_some();

    if !exists {
        return Err(CoreError::BusinessNotFound);
    }

    let id = Uuid::now_v7().to_string();
    connection.execute(
        "INSERT INTO locations(id, business_id, name, code) VALUES(?1, ?2, ?3, ?4)",
        (&id, business_id, name, code),
    )?;
    Ok(id)
}

/// Get a location by ID
pub fn get_location(connection: &Connection, id: &str) -> Result<Option<Location>> {
    let mut stmt = connection.prepare(
        "SELECT id, business_id, name, code, address, contact_phone, contact_email, active 
         FROM locations WHERE id = ?1",
    )?;
    let location = stmt
        .query_row([&id], |row| {
            Ok(Location {
                id: row.get(0)?,
                business_id: row.get(1)?,
                name: row.get(2)?,
                code: row.get(3)?,
                address: row.get(4)?,
                contact_phone: row.get(5)?,
                contact_email: row.get(6)?,
                active: row.get(7)?,
            })
        })
        .optional()?;
    Ok(location)
}

/// Get location by business and code
pub fn get_location_by_code(
    connection: &Connection,
    business_id: &str,
    code: &str,
) -> Result<Option<Location>> {
    let mut stmt = connection.prepare(
        "SELECT id, business_id, name, code, address, contact_phone, contact_email, active 
         FROM locations WHERE business_id = ?1 AND code = ?2 AND active = 1",
    )?;
    let location = stmt
        .query_row([&business_id, &code], |row| {
            Ok(Location {
                id: row.get(0)?,
                business_id: row.get(1)?,
                name: row.get(2)?,
                code: row.get(3)?,
                address: row.get(4)?,
                contact_phone: row.get(5)?,
                contact_email: row.get(6)?,
                active: row.get(7)?,
            })
        })
        .optional()?;
    Ok(location)
}

/// Update location details
pub fn update_location(
    connection: &Connection,
    id: &str,
    name: Option<&str>,
    code: Option<&str>,
    address: Option<&str>,
    contact_phone: Option<&str>,
    contact_email: Option<&str>,
) -> Result<()> {
    if let Some(n) = name
        && n.trim().is_empty()
    {
        return Err(CoreError::EmptyLocationName);
    }

    connection.execute(
        "UPDATE locations 
         SET name = COALESCE(?1, name),
             code = COALESCE(?2, code),
             address = COALESCE(?3, address),
             contact_phone = COALESCE(?4, contact_phone),
             contact_email = COALESCE(?5, contact_email)
         WHERE id = ?6",
        (name, code, address, contact_phone, contact_email, id),
    )?;
    Ok(())
}

/// Deactivate a location (soft delete)
pub fn deactivate_location(connection: &Connection, id: &str) -> Result<()> {
    connection.execute("UPDATE locations SET active = 0 WHERE id = ?1", [id])?;
    Ok(())
}

/// List all active locations for a business
pub fn list_locations(connection: &Connection, business_id: &str) -> Result<Vec<Location>> {
    let mut stmt = connection.prepare(
        "SELECT id, business_id, name, code, address, contact_phone, contact_email, active 
         FROM locations 
         WHERE business_id = ?1 AND active = 1 
         ORDER BY name",
    )?;
    let locations = stmt.query_map([business_id], |row| {
        Ok(Location {
            id: row.get(0)?,
            business_id: row.get(1)?,
            name: row.get(2)?,
            code: row.get(3)?,
            address: row.get(4)?,
            contact_phone: row.get(5)?,
            contact_email: row.get(6)?,
            active: row.get(7)?,
        })
    })?;
    locations
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{business, open_memory_database};

    #[test]
    fn creates_location_with_required_fields() {
        let db = open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();

        let id = create_location(&db, &business_id, "Main Store", Some("MAIN")).unwrap();

        let location = get_location(&db, &id).unwrap().unwrap();
        assert_eq!(location.name, "Main Store");
        assert_eq!(location.code, Some("MAIN".to_string()));
        assert_eq!(location.business_id, business_id);
        assert!(location.active);
        assert!(location.address.is_none());
    }

    #[test]
    fn rejects_empty_location_name() {
        let db = open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();

        assert!(matches!(
            create_location(&db, &business_id, "   ", None),
            Err(CoreError::EmptyLocationName)
        ));
    }

    #[test]
    fn rejects_location_for_nonexistent_business() {
        let db = open_memory_database().unwrap();

        assert!(matches!(
            create_location(&db, "nonexistent", "Store", None),
            Err(CoreError::BusinessNotFound)
        ));
    }

    #[test]
    fn updates_location_details() {
        let db = open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();
        let id = create_location(&db, &business_id, "Original Name", Some("ORIG")).unwrap();

        update_location(
            &db,
            &id,
            Some("Updated Name"),
            Some("UPDT"),
            Some("123 Main St"),
            Some("+27 12 345 6789"),
            Some("store@example.com"),
        )
        .unwrap();

        let location = get_location(&db, &id).unwrap().unwrap();
        assert_eq!(location.name, "Updated Name");
        assert_eq!(location.code, Some("UPDT".to_string()));
        assert_eq!(location.address, Some("123 Main St".to_string()));
        assert_eq!(location.contact_phone, Some("+27 12 345 6789".to_string()));
        assert_eq!(
            location.contact_email,
            Some("store@example.com".to_string())
        );
    }

    #[test]
    fn deactivates_location() {
        let db = open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();
        let id = create_location(&db, &business_id, "Active Store", None).unwrap();

        deactivate_location(&db, &id).unwrap();

        let location = get_location(&db, &id).unwrap().unwrap();
        assert!(!location.active);

        // Should not appear in active list
        let locations = list_locations(&db, &business_id).unwrap();
        assert!(!locations.iter().any(|l| l.id == id));
    }

    #[test]
    fn lists_active_locations_ordered_by_name() {
        let db = open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();

        create_location(&db, &business_id, "Store C", Some("C")).unwrap();
        create_location(&db, &business_id, "Store A", Some("A")).unwrap();
        create_location(&db, &business_id, "Store B", Some("B")).unwrap();

        let locations = list_locations(&db, &business_id).unwrap();
        assert_eq!(locations.len(), 3);
        assert_eq!(locations[0].name, "Store A");
        assert_eq!(locations[1].name, "Store B");
        assert_eq!(locations[2].name, "Store C");
    }

    #[test]
    fn gets_location_by_code() {
        let db = open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();
        create_location(&db, &business_id, "Main Store", Some("MAIN")).unwrap();

        let location = get_location_by_code(&db, &business_id, "MAIN")
            .unwrap()
            .unwrap();
        assert_eq!(location.name, "Main Store");
        assert_eq!(location.code, Some("MAIN".to_string()));

        // Should not find inactive location
        deactivate_location(&db, &location.id).unwrap();
        let inactive = get_location_by_code(&db, &business_id, "MAIN").unwrap();
        assert!(inactive.is_none());
    }

    #[test]
    fn filters_locations_by_business() {
        let db = open_memory_database().unwrap();
        let business_a = business::create_business(&db, "Business A").unwrap();
        let business_b = business::create_business(&db, "Business B").unwrap();

        create_location(&db, &business_a, "Store A1", None).unwrap();
        create_location(&db, &business_a, "Store A2", None).unwrap();
        create_location(&db, &business_b, "Store B1", None).unwrap();

        let locations_a = list_locations(&db, &business_a).unwrap();
        let locations_b = list_locations(&db, &business_b).unwrap();

        assert_eq!(locations_a.len(), 2);
        assert_eq!(locations_b.len(), 1);
        assert!(locations_a.iter().all(|l| l.business_id == business_a));
        assert!(locations_b.iter().all(|l| l.business_id == business_b));
    }
}
