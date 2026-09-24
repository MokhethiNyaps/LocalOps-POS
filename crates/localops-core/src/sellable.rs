use crate::{CoreError, Result};
use rusqlite::Connection;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct SellableItem {
    pub id: String,
    pub business_id: String,
    pub kind: String, // "PRODUCT" or "SERVICE"
    pub name: String,
    pub category_id: Option<String>,
    pub price_minor: i64,
    pub taxable: bool,
    pub barcode: Option<String>,
    pub sku: Option<String>,
    pub product_code: Option<String>,
    pub active: bool,
}

pub(crate) fn create_sellable_item(
    conn: &Connection,
    business_id: &str,
    name: &str,
    kind: &str,
    price_minor: i64,
    category_id: Option<&str>,
    taxable: bool,
) -> Result<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(CoreError::EmptySellableName);
    }

    if kind != "PRODUCT" && kind != "SERVICE" {
        return Err(CoreError::InvalidSellableKind);
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

    // Verify category exists if provided
    if let Some(cat_id) = category_id {
        let cat_exists: bool = conn
            .query_row(
                "SELECT 1 FROM categories WHERE id = ?1 AND business_id = ?2",
                [cat_id, business_id],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !cat_exists {
            return Err(CoreError::CategoryNotFound);
        }
    }

    let id = Uuid::now_v7().to_string();
    conn.execute(
        "INSERT INTO sellable_items(id, business_id, kind, name, category_id, price_minor, taxable, active) 
         VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, 1)",
        (&id, business_id, kind, name, category_id, &price_minor, &taxable),
    )?;

    Ok(id)
}

pub fn get_sellable_item(conn: &Connection, id: &str) -> Result<Option<SellableItem>> {
    let mut stmt = conn.prepare(
        "SELECT id, business_id, kind, name, category_id, price_minor, taxable, barcode, sku, product_code, active 
         FROM sellable_items WHERE id = ?1",
    )?;
    let item = stmt.query_row([id], |row| {
        Ok(SellableItem {
            id: row.get(0)?,
            business_id: row.get(1)?,
            kind: row.get(2)?,
            name: row.get(3)?,
            category_id: row.get(4)?,
            price_minor: row.get(5)?,
            taxable: row.get::<_, i32>(6)? == 1,
            barcode: row.get(7)?,
            sku: row.get(8)?,
            product_code: row.get(9)?,
            active: row.get::<_, i32>(10)? == 1,
        })
    });

    match item {
        Ok(it) => Ok(Some(it)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn get_sellable_items_by_business(
    conn: &Connection,
    business_id: &str,
    active_only: bool,
) -> Result<Vec<SellableItem>> {
    let sql = if active_only {
        "SELECT id, business_id, kind, name, category_id, price_minor, taxable, barcode, sku, product_code, active 
         FROM sellable_items WHERE business_id = ?1 AND active = 1 ORDER BY name"
    } else {
        "SELECT id, business_id, kind, name, category_id, price_minor, taxable, barcode, sku, product_code, active 
         FROM sellable_items WHERE business_id = ?1 ORDER BY name"
    };

    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([business_id], |row| {
        Ok(SellableItem {
            id: row.get(0)?,
            business_id: row.get(1)?,
            kind: row.get(2)?,
            name: row.get(3)?,
            category_id: row.get(4)?,
            price_minor: row.get(5)?,
            taxable: row.get::<_, i32>(6)? == 1,
            barcode: row.get(7)?,
            sku: row.get(8)?,
            product_code: row.get(9)?,
            active: row.get::<_, i32>(10)? == 1,
        })
    })?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row?);
    }

    Ok(items)
}

/// Assigns the local lookup identifiers used by keyboards and USB barcode scanners.
/// Empty values are stored as NULL so the partial unique indexes remain useful.
pub fn set_sellable_identifiers(
    conn: &Connection,
    id: &str,
    barcode: Option<&str>,
    sku: Option<&str>,
    product_code: Option<&str>,
) -> Result<()> {
    let normalize = |value: Option<&str>| {
        value
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    };
    let rows = conn.execute(
        "UPDATE sellable_items
         SET barcode = ?2, sku = ?3, product_code = ?4
         WHERE id = ?1",
        (
            id,
            normalize(barcode),
            normalize(sku),
            normalize(product_code),
        ),
    )?;
    if rows == 0 {
        return Err(CoreError::SellableNotFound);
    }
    Ok(())
}

pub fn update_sellable_item(
    conn: &Connection,
    id: &str,
    name: Option<&str>,
    price_minor: Option<i64>,
    taxable: Option<bool>,
    category_id: Option<Option<&str>>,
) -> Result<()> {
    use rusqlite::types::Value;

    let mut updates = Vec::new();
    let mut params_vec: Vec<Value> = Vec::new();

    if let Some(n) = name {
        let trimmed = n.trim();
        if trimmed.is_empty() {
            return Err(CoreError::EmptySellableName);
        }
        updates.push("name = ?");
        params_vec.push(Value::Text(trimmed.to_string()));
    }

    if let Some(p) = price_minor {
        updates.push("price_minor = ?");
        params_vec.push(Value::Integer(p));
    }

    if let Some(t) = taxable {
        updates.push("taxable = ?");
        params_vec.push(Value::Integer(i64::from(t)));
    }

    if let Some(cid) = category_id {
        updates.push("category_id = ?");
        match cid {
            Some(actual_cid) => {
                let same_business = conn
                    .query_row(
                        "SELECT 1
                         FROM sellable_items s
                         JOIN categories c ON c.business_id = s.business_id
                         WHERE s.id = ?1 AND c.id = ?2",
                        (id, actual_cid),
                        |_| Ok(()),
                    )
                    .is_ok();
                if !same_business {
                    return Err(CoreError::CrossBusinessReference);
                }
                params_vec.push(Value::Text(actual_cid.to_string()));
            }
            None => params_vec.push(Value::Null),
        }
    }

    if updates.is_empty() {
        return Ok(());
    }

    params_vec.push(Value::Text(id.to_string()));
    let sql = format!(
        "UPDATE sellable_items SET {} WHERE id = ?",
        updates.join(", ")
    );

    let rows = conn.execute(&sql, rusqlite::params_from_iter(params_vec.iter()))?;

    if rows == 0 {
        return Err(CoreError::SellableNotFound);
    }

    Ok(())
}

pub fn deactivate_sellable_item(conn: &Connection, id: &str) -> Result<()> {
    let result = conn.execute("UPDATE sellable_items SET active = 0 WHERE id = ?1", [id]);

    match result {
        Ok(0) => Err(CoreError::SellableNotFound),
        Ok(_) => Ok(()),
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{business, category};

    #[test]
    fn creates_product_with_required_fields() {
        let db = crate::open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();

        let id =
            create_sellable_item(&db, &business_id, "Coffee", "PRODUCT", 2500, None, true).unwrap();
        assert_eq!(Uuid::parse_str(&id).unwrap().get_version_num(), 7);

        let item = get_sellable_item(&db, &id).unwrap().unwrap();
        assert_eq!(item.name, "Coffee");
        assert_eq!(item.kind, "PRODUCT");
        assert_eq!(item.price_minor, 2500);
        assert!(item.taxable);
        assert!(item.active);
    }

    #[test]
    fn assigns_and_normalizes_local_lookup_identifiers() {
        let db = crate::open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();
        let id =
            create_sellable_item(&db, &business_id, "Coffee", "PRODUCT", 2500, None, true).unwrap();

        set_sellable_identifiers(
            &db,
            &id,
            Some(" 6001000000012 "),
            Some(" SKU-001 "),
            Some("   "),
        )
        .unwrap();

        let item = get_sellable_item(&db, &id).unwrap().unwrap();
        assert_eq!(item.barcode.as_deref(), Some("6001000000012"));
        assert_eq!(item.sku.as_deref(), Some("SKU-001"));
        assert_eq!(item.product_code, None);
    }

    #[test]
    fn creates_service_with_required_fields() {
        let db = crate::open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();

        let id = create_sellable_item(
            &db,
            &business_id,
            "Consultation",
            "SERVICE",
            50000,
            None,
            false,
        )
        .unwrap();

        let item = get_sellable_item(&db, &id).unwrap().unwrap();
        assert_eq!(item.kind, "SERVICE");
        assert_eq!(item.price_minor, 50000);
        assert!(!item.taxable);
    }

    #[test]
    fn rejects_empty_sellable_name() {
        let db = crate::open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();

        assert!(matches!(
            create_sellable_item(&db, &business_id, "  ", "PRODUCT", 100, None, true),
            Err(CoreError::EmptySellableName)
        ));
    }

    #[test]
    fn rejects_invalid_kind() {
        let db = crate::open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();

        assert!(matches!(
            create_sellable_item(&db, &business_id, "Test", "INVALID", 100, None, true),
            Err(CoreError::InvalidSellableKind)
        ));
    }

    #[test]
    fn validates_business_exists() {
        let db = crate::open_memory_database().unwrap();

        assert!(matches!(
            create_sellable_item(
                &db,
                &Uuid::now_v7().to_string(),
                "Test",
                "PRODUCT",
                100,
                None,
                true
            ),
            Err(CoreError::BusinessNotFound)
        ));
    }

    #[test]
    fn creates_sellable_with_category() {
        let db = crate::open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();
        let cat_id = category::create_category(&db, &business_id, "Beverages", None).unwrap();

        let id = create_sellable_item(
            &db,
            &business_id,
            "Tea",
            "PRODUCT",
            1500,
            Some(&cat_id),
            true,
        )
        .unwrap();

        let item = get_sellable_item(&db, &id).unwrap().unwrap();
        assert_eq!(item.category_id, Some(cat_id));
    }

    #[test]
    fn lists_sellable_items_ordered_by_name() {
        let db = crate::open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();

        create_sellable_item(&db, &business_id, "Zebra", "PRODUCT", 100, None, true).unwrap();
        create_sellable_item(&db, &business_id, "Apple", "PRODUCT", 200, None, true).unwrap();
        create_sellable_item(&db, &business_id, "Mango", "SERVICE", 300, None, false).unwrap();

        let items = get_sellable_items_by_business(&db, &business_id, true).unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].name, "Apple");
        assert_eq!(items[1].name, "Mango");
        assert_eq!(items[2].name, "Zebra");
    }

    #[test]
    fn filters_inactive_sellable_items() {
        let db = crate::open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();

        let id1 =
            create_sellable_item(&db, &business_id, "Active", "PRODUCT", 100, None, true).unwrap();
        let id2 = create_sellable_item(&db, &business_id, "Inactive", "PRODUCT", 200, None, true)
            .unwrap();
        deactivate_sellable_item(&db, &id2).unwrap();

        let active = get_sellable_items_by_business(&db, &business_id, true).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, id1);

        let all = get_sellable_items_by_business(&db, &business_id, false).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn updates_sellable_item_details() {
        let db = crate::open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();

        let id = create_sellable_item(&db, &business_id, "Old Name", "PRODUCT", 100, None, true)
            .unwrap();

        update_sellable_item(&db, &id, Some("New Name"), Some(250), Some(false), None).unwrap();

        let item = get_sellable_item(&db, &id).unwrap().unwrap();
        assert_eq!(item.name, "New Name");
        assert_eq!(item.price_minor, 250);
        assert!(!item.taxable);
    }

    #[test]
    fn clears_category_and_rejects_foreign_category() {
        let db = crate::open_memory_database().unwrap();
        let business_a = business::create_business(&db, "Business A").unwrap();
        let business_b = business::create_business(&db, "Business B").unwrap();
        let category_a = category::create_category(&db, &business_a, "Local", None).unwrap();
        let category_b = category::create_category(&db, &business_b, "Foreign", None).unwrap();
        let id = create_sellable_item(
            &db,
            &business_a,
            "Item",
            "PRODUCT",
            100,
            Some(&category_a),
            true,
        )
        .unwrap();

        update_sellable_item(&db, &id, None, None, None, Some(None)).unwrap();
        assert!(
            get_sellable_item(&db, &id)
                .unwrap()
                .unwrap()
                .category_id
                .is_none()
        );
        assert!(matches!(
            update_sellable_item(&db, &id, None, None, None, Some(Some(&category_b))),
            Err(CoreError::CrossBusinessReference)
        ));
    }

    #[test]
    fn isolates_sellable_items_by_business() {
        let db = crate::open_memory_database().unwrap();
        let biz1 = business::create_business(&db, "Business 1").unwrap();
        let biz2 = business::create_business(&db, "Business 2").unwrap();

        let id1 = create_sellable_item(&db, &biz1, "Item1", "PRODUCT", 100, None, true).unwrap();
        let id2 = create_sellable_item(&db, &biz2, "Item2", "PRODUCT", 200, None, true).unwrap();

        let items1 = get_sellable_items_by_business(&db, &biz1, false).unwrap();
        assert_eq!(items1.len(), 1);
        assert_eq!(items1[0].id, id1);

        let items2 = get_sellable_items_by_business(&db, &biz2, false).unwrap();
        assert_eq!(items2.len(), 1);
        assert_eq!(items2[0].id, id2);
    }
}
