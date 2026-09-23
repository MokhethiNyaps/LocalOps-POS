use crate::{CoreError, Result};
use rusqlite::{Connection, OptionalExtension};
use uuid::Uuid;

/// Terminal entity representing a POS terminal/register
#[derive(Debug, Clone)]
pub struct Terminal {
    pub id: String,
    pub business_id: String,
    pub department_id: Option<String>,
    pub location_id: Option<String>,
    pub device_key: String,
    pub name: String,
    pub code: Option<String>,
    pub description: Option<String>,
    pub active: bool,
}

/// Create a new terminal
pub fn create_terminal(
    connection: &Connection,
    business_id: &str,
    location_id: Option<&str>,
    name: &str,
    code: Option<&str>,
) -> Result<String> {
    let device_key = Uuid::now_v7().to_string();
    create_terminal_with_identity(
        connection,
        business_id,
        None,
        location_id,
        name,
        code,
        &device_key,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn create_terminal_with_identity(
    connection: &Connection,
    business_id: &str,
    department_id: Option<&str>,
    location_id: Option<&str>,
    name: &str,
    code: Option<&str>,
    device_key: &str,
) -> Result<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(CoreError::EmptyTerminalName);
    }
    let device_key = device_key.trim();
    if device_key.is_empty() {
        return Err(CoreError::EmptyDeviceKey);
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

    // Verify location exists if provided
    if let Some(loc_id) = location_id {
        let loc_exists: bool = connection
            .query_row(
                "SELECT 1 FROM locations WHERE id = ?1 AND business_id = ?2 AND active = 1",
                (loc_id, business_id),
                |row| row.get::<_, i32>(0),
            )
            .optional()?
            .is_some();

        if !loc_exists {
            return Err(CoreError::LocationNotFound);
        }
    }

    if let Some(dept_id) = department_id {
        let department_exists = connection
            .query_row(
                "SELECT 1 FROM departments WHERE id = ?1 AND business_id = ?2 AND active = 1",
                (dept_id, business_id),
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if !department_exists {
            return Err(CoreError::DepartmentNotFound);
        }
    }

    let id = Uuid::now_v7().to_string();
    connection.execute(
        "INSERT INTO terminals(
             id, business_id, department_id, location_id, device_key, name, code
         ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        (
            &id,
            business_id,
            department_id,
            location_id,
            device_key,
            name,
            code,
        ),
    )?;
    Ok(id)
}

/// Get a terminal by ID
pub fn get_terminal(connection: &Connection, id: &str) -> Result<Option<Terminal>> {
    let mut stmt = connection.prepare(
        "SELECT id, business_id, department_id, location_id, device_key, name, code, description, active 
         FROM terminals WHERE id = ?1",
    )?;
    let terminal = stmt
        .query_row([&id], |row| {
            Ok(Terminal {
                id: row.get(0)?,
                business_id: row.get(1)?,
                department_id: row.get(2)?,
                location_id: row.get(3)?,
                device_key: row.get(4)?,
                name: row.get(5)?,
                code: row.get(6)?,
                description: row.get(7)?,
                active: row.get(8)?,
            })
        })
        .optional()?;
    Ok(terminal)
}

/// Get terminal by business and code
pub fn get_terminal_by_code(
    connection: &Connection,
    business_id: &str,
    code: &str,
) -> Result<Option<Terminal>> {
    let mut stmt = connection.prepare(
        "SELECT id, business_id, department_id, location_id, device_key, name, code, description, active 
         FROM terminals WHERE business_id = ?1 AND code = ?2 AND active = 1",
    )?;
    let terminal = stmt
        .query_row([&business_id, &code], |row| {
            Ok(Terminal {
                id: row.get(0)?,
                business_id: row.get(1)?,
                department_id: row.get(2)?,
                location_id: row.get(3)?,
                device_key: row.get(4)?,
                name: row.get(5)?,
                code: row.get(6)?,
                description: row.get(7)?,
                active: row.get(8)?,
            })
        })
        .optional()?;
    Ok(terminal)
}

/// Update terminal details
pub fn update_terminal(
    connection: &Connection,
    id: &str,
    name: Option<&str>,
    code: Option<&str>,
    location_id: Option<&str>,
    description: Option<&str>,
) -> Result<()> {
    if let Some(n) = name
        && n.trim().is_empty()
    {
        return Err(CoreError::EmptyTerminalName);
    }

    connection.execute(
        "UPDATE terminals 
         SET name = COALESCE(?1, name),
             code = COALESCE(?2, code),
             location_id = COALESCE(?3, location_id),
             description = COALESCE(?4, description)
         WHERE id = ?5",
        (name, code, location_id, description, id),
    )?;
    Ok(())
}

/// Deactivate a terminal (soft delete)
pub fn deactivate_terminal(connection: &Connection, id: &str) -> Result<()> {
    connection.execute("UPDATE terminals SET active = 0 WHERE id = ?1", [id])?;
    Ok(())
}

/// List all active terminals for a business
pub fn list_terminals(connection: &Connection, business_id: &str) -> Result<Vec<Terminal>> {
    let mut stmt = connection.prepare(
        "SELECT id, business_id, department_id, location_id, device_key, name, code, description, active 
         FROM terminals 
         WHERE business_id = ?1 AND active = 1 
         ORDER BY name",
    )?;
    let terminals = stmt.query_map([business_id], |row| {
        Ok(Terminal {
            id: row.get(0)?,
            business_id: row.get(1)?,
            department_id: row.get(2)?,
            location_id: row.get(3)?,
            device_key: row.get(4)?,
            name: row.get(5)?,
            code: row.get(6)?,
            description: row.get(7)?,
            active: row.get(8)?,
        })
    })?;
    terminals
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

/// List terminals for a specific location
pub fn list_terminals_by_location(
    connection: &Connection,
    location_id: &str,
) -> Result<Vec<Terminal>> {
    let mut stmt = connection.prepare(
        "SELECT id, business_id, department_id, location_id, device_key, name, code, description, active 
         FROM terminals 
         WHERE location_id = ?1 AND active = 1 
         ORDER BY name",
    )?;
    let terminals = stmt.query_map([location_id], |row| {
        Ok(Terminal {
            id: row.get(0)?,
            business_id: row.get(1)?,
            department_id: row.get(2)?,
            location_id: row.get(3)?,
            device_key: row.get(4)?,
            name: row.get(5)?,
            code: row.get(6)?,
            description: row.get(7)?,
            active: row.get(8)?,
        })
    })?;
    terminals
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{business, location, open_memory_database};

    #[test]
    fn creates_terminal_with_required_fields() {
        let db = open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();

        let id = create_terminal(&db, &business_id, None, "Main Register", Some("REG01")).unwrap();

        let terminal = get_terminal(&db, &id).unwrap().unwrap();
        assert_eq!(terminal.name, "Main Register");
        assert_eq!(terminal.code, Some("REG01".to_string()));
        assert_eq!(terminal.business_id, business_id);
        assert!(terminal.active);
        assert!(terminal.location_id.is_none());
    }

    #[test]
    fn creates_terminal_with_location() {
        let db = open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();
        let location_id =
            location::create_location(&db, &business_id, "Main Store", Some("MAIN")).unwrap();

        let id = create_terminal(
            &db,
            &business_id,
            Some(&location_id),
            "Store Register",
            Some("STORE01"),
        )
        .unwrap();

        let terminal = get_terminal(&db, &id).unwrap().unwrap();
        assert_eq!(terminal.location_id, Some(location_id));
        assert_eq!(terminal.name, "Store Register");
    }

    #[test]
    fn rejects_empty_terminal_name() {
        let db = open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();

        assert!(matches!(
            create_terminal(&db, &business_id, None, "   ", None),
            Err(CoreError::EmptyTerminalName)
        ));
    }

    #[test]
    fn rejects_terminal_for_nonexistent_business() {
        let db = open_memory_database().unwrap();

        assert!(matches!(
            create_terminal(&db, "nonexistent", None, "Register", None),
            Err(CoreError::BusinessNotFound)
        ));
    }

    #[test]
    fn rejects_terminal_for_nonexistent_location() {
        let db = open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();

        assert!(matches!(
            create_terminal(&db, &business_id, Some("nonexistent"), "Register", None),
            Err(CoreError::LocationNotFound)
        ));
    }

    #[test]
    fn updates_terminal_details() {
        let db = open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();
        let location_id =
            location::create_location(&db, &business_id, "Main Store", Some("MAIN")).unwrap();
        let id = create_terminal(&db, &business_id, None, "Original Name", Some("ORIG")).unwrap();

        update_terminal(
            &db,
            &id,
            Some("Updated Name"),
            Some("UPDT"),
            Some(&location_id),
            Some("Updated description"),
        )
        .unwrap();

        let terminal = get_terminal(&db, &id).unwrap().unwrap();
        assert_eq!(terminal.name, "Updated Name");
        assert_eq!(terminal.code, Some("UPDT".to_string()));
        assert_eq!(terminal.location_id, Some(location_id));
        assert_eq!(
            terminal.description,
            Some("Updated description".to_string())
        );
    }

    #[test]
    fn deactivates_terminal() {
        let db = open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();
        let id = create_terminal(&db, &business_id, None, "Active Register", None).unwrap();

        deactivate_terminal(&db, &id).unwrap();

        let terminal = get_terminal(&db, &id).unwrap().unwrap();
        assert!(!terminal.active);

        // Should not appear in active list
        let terminals = list_terminals(&db, &business_id).unwrap();
        assert!(!terminals.iter().any(|t| t.id == id));
    }

    #[test]
    fn lists_active_terminals_ordered_by_name() {
        let db = open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();

        create_terminal(&db, &business_id, None, "Register C", Some("C")).unwrap();
        create_terminal(&db, &business_id, None, "Register A", Some("A")).unwrap();
        create_terminal(&db, &business_id, None, "Register B", Some("B")).unwrap();

        let terminals = list_terminals(&db, &business_id).unwrap();
        assert_eq!(terminals.len(), 3);
        assert_eq!(terminals[0].name, "Register A");
        assert_eq!(terminals[1].name, "Register B");
        assert_eq!(terminals[2].name, "Register C");
    }

    #[test]
    fn gets_terminal_by_code() {
        let db = open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();
        create_terminal(&db, &business_id, None, "Main Register", Some("MAIN")).unwrap();

        let terminal = get_terminal_by_code(&db, &business_id, "MAIN")
            .unwrap()
            .unwrap();
        assert_eq!(terminal.name, "Main Register");
        assert_eq!(terminal.code, Some("MAIN".to_string()));

        // Should not find inactive terminal
        deactivate_terminal(&db, &terminal.id).unwrap();
        let inactive = get_terminal_by_code(&db, &business_id, "MAIN").unwrap();
        assert!(inactive.is_none());
    }

    #[test]
    fn lists_terminals_by_location() {
        let db = open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();
        let location_a =
            location::create_location(&db, &business_id, "Location A", Some("LOCA")).unwrap();
        let location_b =
            location::create_location(&db, &business_id, "Location B", Some("LOCB")).unwrap();

        create_terminal(&db, &business_id, Some(&location_a), "Terminal A1", None).unwrap();
        create_terminal(&db, &business_id, Some(&location_a), "Terminal A2", None).unwrap();
        create_terminal(&db, &business_id, Some(&location_b), "Terminal B1", None).unwrap();
        create_terminal(&db, &business_id, None, "No Location Terminal", None).unwrap();

        let terminals_a = list_terminals_by_location(&db, &location_a).unwrap();
        let terminals_b = list_terminals_by_location(&db, &location_b).unwrap();

        assert_eq!(terminals_a.len(), 2);
        assert_eq!(terminals_b.len(), 1);
        assert!(
            terminals_a
                .iter()
                .all(|t| t.location_id == Some(location_a.clone()))
        );
        assert!(
            terminals_b
                .iter()
                .all(|t| t.location_id == Some(location_b.clone()))
        );
    }

    #[test]
    fn filters_terminals_by_business() {
        let db = open_memory_database().unwrap();
        let business_a = business::create_business(&db, "Business A").unwrap();
        let business_b = business::create_business(&db, "Business B").unwrap();

        create_terminal(&db, &business_a, None, "Terminal A1", None).unwrap();
        create_terminal(&db, &business_a, None, "Terminal A2", None).unwrap();
        create_terminal(&db, &business_b, None, "Terminal B1", None).unwrap();

        let terminals_a = list_terminals(&db, &business_a).unwrap();
        let terminals_b = list_terminals(&db, &business_b).unwrap();

        assert_eq!(terminals_a.len(), 2);
        assert_eq!(terminals_b.len(), 1);
        assert!(terminals_a.iter().all(|t| t.business_id == business_a));
        assert!(terminals_b.iter().all(|t| t.business_id == business_b));
    }
}
