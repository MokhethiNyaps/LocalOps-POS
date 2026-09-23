use crate::{CoreError, Result};
use rusqlite::Connection;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Category {
    pub id: String,
    pub business_id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub active: bool,
}

pub fn create_category(
    conn: &Connection,
    business_id: &str,
    name: &str,
    parent_id: Option<&str>,
) -> Result<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(CoreError::EmptyCategoryName);
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

    // Verify parent exists if provided
    if let Some(pid) = parent_id {
        let parent_exists: bool = conn
            .query_row(
                "SELECT 1 FROM categories WHERE id = ?1 AND business_id = ?2",
                [pid, business_id],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !parent_exists {
            return Err(CoreError::CategoryNotFound);
        }
    }

    let id = Uuid::now_v7().to_string();
    conn.execute(
        "INSERT INTO categories(id, business_id, name, parent_id, active) VALUES(?1, ?2, ?3, ?4, 1)",
        (&id, business_id, name, parent_id),
    )?;

    Ok(id)
}

pub fn get_category(conn: &Connection, id: &str) -> Result<Option<Category>> {
    let mut stmt = conn
        .prepare("SELECT id, business_id, name, parent_id, active FROM categories WHERE id = ?1")?;
    let category = stmt.query_row([id], |row| {
        Ok(Category {
            id: row.get(0)?,
            business_id: row.get(1)?,
            name: row.get(2)?,
            parent_id: row.get(3)?,
            active: row.get::<_, i32>(4)? == 1,
        })
    });

    match category {
        Ok(cat) => Ok(Some(cat)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn get_categories_by_business(
    conn: &Connection,
    business_id: &str,
    active_only: bool,
) -> Result<Vec<Category>> {
    let sql = if active_only {
        "SELECT id, business_id, name, parent_id, active FROM categories 
         WHERE business_id = ?1 AND active = 1 ORDER BY name"
    } else {
        "SELECT id, business_id, name, parent_id, active FROM categories 
         WHERE business_id = ?1 ORDER BY name"
    };

    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([business_id], |row| {
        Ok(Category {
            id: row.get(0)?,
            business_id: row.get(1)?,
            name: row.get(2)?,
            parent_id: row.get(3)?,
            active: row.get::<_, i32>(4)? == 1,
        })
    })?;

    let mut categories = Vec::new();
    for row in rows {
        categories.push(row?);
    }

    Ok(categories)
}

pub fn update_category(
    conn: &Connection,
    id: &str,
    name: Option<&str>,
    parent_id: Option<Option<&str>>,
) -> Result<()> {
    let mut updates = Vec::new();
    let mut params_vec = Vec::new();

    if let Some(n) = name {
        let trimmed = n.trim();
        if trimmed.is_empty() {
            return Err(CoreError::EmptyCategoryName);
        }
        updates.push("name = ?");
        params_vec.push(trimmed.to_string());
    }

    if let Some(pid) = parent_id {
        updates.push("parent_id = ?");
        match pid {
            Some(actual_pid) => params_vec.push(actual_pid.to_string()),
            None => params_vec.push(String::new()),
        }
    }

    if updates.is_empty() {
        return Ok(());
    }

    params_vec.push(id.to_string());
    let sql = format!("UPDATE categories SET {} WHERE id = ?", updates.join(", "));

    let rows = conn.execute(&sql, rusqlite::params_from_iter(params_vec.iter()))?;

    if rows == 0 {
        return Err(CoreError::CategoryNotFound);
    }

    Ok(())
}

pub fn deactivate_category(conn: &Connection, id: &str) -> Result<()> {
    let result = conn.execute("UPDATE categories SET active = 0 WHERE id = ?1", [id]);

    match result {
        Ok(0) => Err(CoreError::CategoryNotFound),
        Ok(_) => Ok(()),
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::business;

    #[test]
    fn creates_category_with_required_fields() {
        let db = crate::open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();

        let id = create_category(&db, &business_id, "Beverages", None).unwrap();
        assert_eq!(Uuid::parse_str(&id).unwrap().get_version_num(), 7);

        let cat = get_category(&db, &id).unwrap().unwrap();
        assert_eq!(cat.name, "Beverages");
        assert!(cat.active);
        assert!(cat.parent_id.is_none());
    }

    #[test]
    fn rejects_empty_category_name() {
        let db = crate::open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();

        assert!(matches!(
            create_category(&db, &business_id, "  ", None),
            Err(CoreError::EmptyCategoryName)
        ));
    }

    #[test]
    fn validates_business_exists() {
        let db = crate::open_memory_database().unwrap();

        assert!(matches!(
            create_category(&db, &Uuid::now_v7().to_string(), "Test", None),
            Err(CoreError::BusinessNotFound)
        ));
    }

    #[test]
    fn creates_category_with_parent() {
        let db = crate::open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();

        let parent_id = create_category(&db, &business_id, "Drinks", None).unwrap();
        let child_id = create_category(&db, &business_id, "Soft Drinks", Some(&parent_id)).unwrap();

        let child = get_category(&db, &child_id).unwrap().unwrap();
        assert_eq!(child.parent_id, Some(parent_id));
    }

    #[test]
    fn lists_categories_ordered_by_name() {
        let db = crate::open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();

        create_category(&db, &business_id, "Zebra", None).unwrap();
        create_category(&db, &business_id, "Apple", None).unwrap();
        create_category(&db, &business_id, "Mango", None).unwrap();

        let cats = get_categories_by_business(&db, &business_id, true).unwrap();
        assert_eq!(cats.len(), 3);
        assert_eq!(cats[0].name, "Apple");
        assert_eq!(cats[1].name, "Mango");
        assert_eq!(cats[2].name, "Zebra");
    }

    #[test]
    fn filters_inactive_categories() {
        let db = crate::open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();

        let id1 = create_category(&db, &business_id, "Active", None).unwrap();
        let id2 = create_category(&db, &business_id, "Inactive", None).unwrap();
        deactivate_category(&db, &id2).unwrap();

        let active = get_categories_by_business(&db, &business_id, true).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, id1);

        let all = get_categories_by_business(&db, &business_id, false).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn updates_category_details() {
        let db = crate::open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();

        let parent_id = create_category(&db, &business_id, "Parent", None).unwrap();
        let id = create_category(&db, &business_id, "Old Name", None).unwrap();

        update_category(&db, &id, Some("New Name"), Some(Some(&parent_id))).unwrap();

        let cat = get_category(&db, &id).unwrap().unwrap();
        assert_eq!(cat.name, "New Name");
        assert_eq!(cat.parent_id, Some(parent_id));
    }

    #[test]
    fn isolates_categories_by_business() {
        let db = crate::open_memory_database().unwrap();
        let biz1 = business::create_business(&db, "Business 1").unwrap();
        let biz2 = business::create_business(&db, "Business 2").unwrap();

        let id1 = create_category(&db, &biz1, "Cat1", None).unwrap();
        let id2 = create_category(&db, &biz2, "Cat2", None).unwrap();

        let cats1 = get_categories_by_business(&db, &biz1, false).unwrap();
        assert_eq!(cats1.len(), 1);
        assert_eq!(cats1[0].id, id1);

        let cats2 = get_categories_by_business(&db, &biz2, false).unwrap();
        assert_eq!(cats2.len(), 1);
        assert_eq!(cats2[0].id, id2);
    }
}
