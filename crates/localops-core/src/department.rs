use crate::{CoreError, Result};
use rusqlite::{Connection, OptionalExtension};
use uuid::Uuid;

/// Department within a business
#[derive(Debug, Clone)]
pub struct Department {
    pub id: String,
    pub business_id: String,
    pub name: String,
    pub description: Option<String>,
}

/// Create a new department
pub fn create_department(
    connection: &Connection,
    business_id: &str,
    name: &str,
    description: Option<&str>,
) -> Result<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(CoreError::EmptyDepartmentName);
    }
    let id = Uuid::now_v7().to_string();
    connection.execute(
        "INSERT INTO departments(id, business_id, name, description) VALUES(?1, ?2, ?3, ?4)",
        (&id, business_id, name, description),
    )?;
    Ok(id)
}

/// Get a department by ID
pub fn get_department(connection: &Connection, id: &str) -> Result<Option<Department>> {
    let mut stmt = connection
        .prepare("SELECT id, business_id, name, description FROM departments WHERE id = ?1")?;
    let dept = stmt
        .query_row([&id], |row| {
            Ok(Department {
                id: row.get(0)?,
                business_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
            })
        })
        .optional()?;
    Ok(dept)
}

/// List all departments for a business
pub fn list_departments(connection: &Connection, business_id: &str) -> Result<Vec<Department>> {
    let mut stmt = connection.prepare(
        "SELECT id, business_id, name, description FROM departments WHERE business_id = ?1 AND active = 1 ORDER BY name"
    )?;
    let depts = stmt.query_map([business_id], |row| {
        Ok(Department {
            id: row.get(0)?,
            business_id: row.get(1)?,
            name: row.get(2)?,
            description: row.get(3)?,
        })
    })?;
    depts
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

/// Update department name
pub fn update_department_name(connection: &Connection, id: &str, name: &str) -> Result<()> {
    let name = name.trim();
    if name.is_empty() {
        return Err(CoreError::EmptyDepartmentName);
    }
    connection.execute("UPDATE departments SET name = ?1 WHERE id = ?2", (name, id))?;
    Ok(())
}

/// Deactivate a department
pub fn deactivate_department(connection: &Connection, id: &str) -> Result<()> {
    connection.execute("UPDATE departments SET active = 0 WHERE id = ?1", [id])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{business, open_memory_database};

    #[test]
    fn creates_department() {
        let db = open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();
        let id = create_department(&db, &business_id, "Bar", Some("Beverage department")).unwrap();

        let dept = get_department(&db, &id).unwrap().unwrap();
        assert_eq!(dept.name, "Bar");
        assert_eq!(dept.business_id, business_id);
        assert_eq!(dept.description, Some("Beverage department".to_string()));
    }

    #[test]
    fn rejects_empty_department_name() {
        let db = open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();
        assert!(matches!(
            create_department(&db, &business_id, "   ", None),
            Err(CoreError::EmptyDepartmentName)
        ));
    }

    #[test]
    fn prevents_duplicate_department_names_per_business() {
        let db = open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();
        create_department(&db, &business_id, "Bar", None).unwrap();

        assert!(matches!(
            create_department(&db, &business_id, "Bar", None),
            Err(CoreError::Database(_))
        ));
    }

    #[test]
    fn lists_departments_for_business() {
        let db = open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();
        create_department(&db, &business_id, "Bar", None).unwrap();
        create_department(&db, &business_id, "Kitchen", None).unwrap();

        let depts = list_departments(&db, &business_id).unwrap();
        assert_eq!(depts.len(), 2);
    }

    #[test]
    fn updates_department_name() {
        let db = open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();
        let id = create_department(&db, &business_id, "Old Name", None).unwrap();

        update_department_name(&db, &id, "New Name").unwrap();
        let dept = get_department(&db, &id).unwrap().unwrap();
        assert_eq!(dept.name, "New Name");
    }

    #[test]
    fn deactivates_department() {
        let db = open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();
        let id = create_department(&db, &business_id, "Bar", None).unwrap();

        deactivate_department(&db, &id).unwrap();
        let depts = list_departments(&db, &business_id).unwrap();
        assert_eq!(depts.len(), 0);
    }
}
