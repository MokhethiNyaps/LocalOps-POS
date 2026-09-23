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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DepartmentLocation {
    pub department_id: String,
    pub location_id: String,
    pub location_name: String,
    pub is_default: bool,
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

pub fn assign_location(
    connection: &Connection,
    department_id: &str,
    location_id: &str,
    is_default: bool,
) -> Result<()> {
    let transaction = connection.unchecked_transaction()?;
    let same_business = transaction
        .query_row(
            "SELECT 1
             FROM departments d JOIN locations l ON l.business_id = d.business_id
             WHERE d.id = ?1 AND l.id = ?2 AND d.active = 1 AND l.active = 1",
            (department_id, location_id),
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if !same_business {
        return Err(CoreError::CrossBusinessReference);
    }

    if is_default {
        transaction.execute(
            "UPDATE department_locations SET is_default = 0 WHERE department_id = ?1",
            [department_id],
        )?;
    }
    transaction.execute(
        "INSERT INTO department_locations(department_id, location_id, is_default)
         VALUES(?1, ?2, ?3)
         ON CONFLICT(department_id, location_id) DO UPDATE SET is_default = excluded.is_default",
        (department_id, location_id, is_default),
    )?;
    transaction.commit()?;
    Ok(())
}

pub fn list_department_locations(
    connection: &Connection,
    department_id: &str,
) -> Result<Vec<DepartmentLocation>> {
    let mut statement = connection.prepare(
        "SELECT dl.department_id, dl.location_id, l.name, dl.is_default
         FROM department_locations dl
         JOIN locations l ON l.id = dl.location_id
         WHERE dl.department_id = ?1 AND l.active = 1
         ORDER BY dl.is_default DESC, l.name",
    )?;
    statement
        .query_map([department_id], |row| {
            Ok(DepartmentLocation {
                department_id: row.get(0)?,
                location_id: row.get(1)?,
                location_name: row.get(2)?,
                is_default: row.get(3)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{business, location, open_memory_database};

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

    #[test]
    fn assigns_locations_and_changes_the_default_atomically() {
        let db = open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();
        let department_id = create_department(&db, &business_id, "Bar", None).unwrap();
        let main =
            location::create_location(&db, &business_id, "Main Store", Some("MAIN")).unwrap();
        let bar = location::create_location(&db, &business_id, "Bar Store", Some("BAR")).unwrap();

        assign_location(&db, &department_id, &main, true).unwrap();
        assign_location(&db, &department_id, &bar, true).unwrap();
        let locations = list_department_locations(&db, &department_id).unwrap();
        assert_eq!(locations.len(), 2);
        assert_eq!(locations.iter().filter(|item| item.is_default).count(), 1);
        assert_eq!(locations[0].location_id, bar);
    }

    #[test]
    fn blocks_location_assignment_across_businesses() {
        let db = open_memory_database().unwrap();
        let business_a = business::create_business(&db, "Business A").unwrap();
        let business_b = business::create_business(&db, "Business B").unwrap();
        let department_id = create_department(&db, &business_a, "Bar", None).unwrap();
        let location_id = location::create_location(&db, &business_b, "Store", None).unwrap();
        assert!(matches!(
            assign_location(&db, &department_id, &location_id, true),
            Err(CoreError::CrossBusinessReference)
        ));
    }
}
