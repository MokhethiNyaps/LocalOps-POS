use crate::{CoreError, Result};
use rusqlite::Connection;

#[derive(Debug, Clone)]
pub struct Product {
    pub sellable_id: String,
    pub base_unit_id: String,
    pub cost_minor: i64,
    pub track_stock: bool,
    pub minimum_quantity_micros: i64,
}

pub(crate) fn create_product(
    conn: &Connection,
    sellable_id: &str,
    base_unit_id: &str,
    cost_minor: i64,
    track_stock: bool,
    minimum_quantity_micros: i64,
) -> Result<()> {
    // Verify sellable exists and is a PRODUCT
    let kind: String = conn
        .query_row(
            "SELECT kind FROM sellable_items WHERE id = ?1",
            [sellable_id],
            |row| row.get(0),
        )
        .map_err(|_| CoreError::SellableNotFound)?;

    if kind != "PRODUCT" {
        return Err(CoreError::InvalidSellableKind);
    }

    // Verify the base unit belongs to the same business as the sellable.
    let unit_exists: bool = conn
        .query_row(
            "SELECT 1
             FROM units u JOIN sellable_items s ON s.business_id = u.business_id
             WHERE u.id = ?1 AND s.id = ?2",
            (base_unit_id, sellable_id),
            |row| row.get(0),
        )
        .unwrap_or(false);

    if !unit_exists {
        return Err(CoreError::UnitNotFound);
    }

    conn.execute(
        "INSERT INTO products(sellable_id, base_unit_id, cost_minor, track_stock, minimum_quantity_micros) 
         VALUES(?1, ?2, ?3, ?4, ?5)",
        (sellable_id, base_unit_id, &cost_minor, &track_stock, &minimum_quantity_micros),
    )?;

    Ok(())
}

pub fn get_product(conn: &Connection, sellable_id: &str) -> Result<Option<Product>> {
    let mut stmt = conn.prepare(
        "SELECT sellable_id, base_unit_id, cost_minor, track_stock, minimum_quantity_micros 
         FROM products WHERE sellable_id = ?1",
    )?;
    let product = stmt.query_row([sellable_id], |row| {
        Ok(Product {
            sellable_id: row.get(0)?,
            base_unit_id: row.get(1)?,
            cost_minor: row.get(2)?,
            track_stock: row.get::<_, i32>(3)? == 1,
            minimum_quantity_micros: row.get(4)?,
        })
    });

    match product {
        Ok(prod) => Ok(Some(prod)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn update_product(
    conn: &Connection,
    sellable_id: &str,
    base_unit_id: Option<&str>,
    cost_minor: Option<i64>,
    track_stock: Option<bool>,
    minimum_quantity_micros: Option<i64>,
) -> Result<()> {
    let mut updates = Vec::new();
    let mut params_vec: Vec<String> = Vec::new();

    if let Some(uid) = base_unit_id {
        // Verify the unit belongs to the product's business.
        let unit_exists: bool = conn
            .query_row(
                "SELECT 1
                 FROM units u
                 JOIN sellable_items s ON s.business_id = u.business_id
                 JOIN products p ON p.sellable_id = s.id
                 WHERE u.id = ?1 AND p.sellable_id = ?2",
                (uid, sellable_id),
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !unit_exists {
            return Err(CoreError::UnitNotFound);
        }

        updates.push("base_unit_id = ?");
        params_vec.push(uid.to_string());
    }

    if let Some(c) = cost_minor {
        updates.push("cost_minor = ?");
        params_vec.push(c.to_string());
    }

    if let Some(t) = track_stock {
        updates.push("track_stock = ?");
        params_vec.push(if t { "1".to_string() } else { "0".to_string() });
    }

    if let Some(m) = minimum_quantity_micros {
        updates.push("minimum_quantity_micros = ?");
        params_vec.push(m.to_string());
    }

    if updates.is_empty() {
        return Ok(());
    }

    params_vec.push(sellable_id.to_string());
    let sql = format!(
        "UPDATE products SET {} WHERE sellable_id = ?",
        updates.join(", ")
    );

    let rows = conn.execute(&sql, rusqlite::params_from_iter(params_vec.iter()))?;

    if rows == 0 {
        return Err(CoreError::SellableNotFound);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{business, sellable, unit};

    #[test]
    fn creates_product_with_required_fields() {
        let db = crate::open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();

        let unit_id = unit::create_unit(&db, &business_id, "EA", "Each", "COUNT", 1, 1, 0).unwrap();
        let sellable_id = sellable::create_sellable_item(
            &db,
            &business_id,
            "Coffee",
            "PRODUCT",
            2500,
            None,
            true,
        )
        .unwrap();

        create_product(&db, &sellable_id, &unit_id, 1500, true, 1000000).unwrap();

        let product = get_product(&db, &sellable_id).unwrap().unwrap();
        assert_eq!(product.sellable_id, sellable_id);
        assert_eq!(product.base_unit_id, unit_id);
        assert_eq!(product.cost_minor, 1500);
        assert!(product.track_stock);
        assert_eq!(product.minimum_quantity_micros, 1000000);
    }

    #[test]
    fn rejects_creation_for_service() {
        let db = crate::open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();

        let unit_id = unit::create_unit(&db, &business_id, "HR", "Hour", "TIME", 60, 1, 2).unwrap();
        let service_id = sellable::create_sellable_item(
            &db,
            &business_id,
            "Consultation",
            "SERVICE",
            50000,
            None,
            false,
        )
        .unwrap();

        assert!(matches!(
            create_product(&db, &service_id, &unit_id, 0, false, 0),
            Err(CoreError::InvalidSellableKind)
        ));
    }

    #[test]
    fn rejects_creation_for_nonexistent_sellable() {
        let db = crate::open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();

        let unit_id = unit::create_unit(&db, &business_id, "EA", "Each", "COUNT", 1, 1, 0).unwrap();

        assert!(matches!(
            create_product(
                &db,
                &uuid::Uuid::now_v7().to_string(),
                &unit_id,
                100,
                true,
                0
            ),
            Err(CoreError::SellableNotFound)
        ));
    }

    #[test]
    fn rejects_creation_with_nonexistent_unit() {
        let db = crate::open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();

        let sellable_id = sellable::create_sellable_item(
            &db,
            &business_id,
            "Product",
            "PRODUCT",
            100,
            None,
            true,
        )
        .unwrap();

        assert!(matches!(
            create_product(
                &db,
                &sellable_id,
                &uuid::Uuid::now_v7().to_string(),
                100,
                true,
                0
            ),
            Err(CoreError::UnitNotFound)
        ));
    }

    #[test]
    fn rejects_unit_from_another_business() {
        let db = crate::open_memory_database().unwrap();
        let business_a = business::create_business(&db, "Business A").unwrap();
        let business_b = business::create_business(&db, "Business B").unwrap();
        let foreign_unit =
            unit::create_unit(&db, &business_b, "EA", "Each", "COUNT", 1, 1, 0).unwrap();
        let sellable_id =
            sellable::create_sellable_item(&db, &business_a, "Product", "PRODUCT", 100, None, true)
                .unwrap();

        assert!(matches!(
            create_product(&db, &sellable_id, &foreign_unit, 50, true, 0),
            Err(CoreError::UnitNotFound)
        ));
    }

    #[test]
    fn updates_product_details() {
        let db = crate::open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();

        let unit1_id =
            unit::create_unit(&db, &business_id, "EA", "Each", "COUNT", 1, 1, 0).unwrap();
        let unit2_id =
            unit::create_unit(&db, &business_id, "BOX", "Box", "COUNT", 12, 1, 0).unwrap();
        let sellable_id = sellable::create_sellable_item(
            &db,
            &business_id,
            "Widget",
            "PRODUCT",
            5000,
            None,
            true,
        )
        .unwrap();

        create_product(&db, &sellable_id, &unit1_id, 3000, true, 500000).unwrap();

        update_product(
            &db,
            &sellable_id,
            Some(&unit2_id),
            Some(4000),
            Some(false),
            Some(1000000),
        )
        .unwrap();

        let product = get_product(&db, &sellable_id).unwrap().unwrap();
        assert_eq!(product.base_unit_id, unit2_id);
        assert_eq!(product.cost_minor, 4000);
        assert!(!product.track_stock);
        assert_eq!(product.minimum_quantity_micros, 1000000);
    }

    #[test]
    fn returns_none_for_nonexistent_product() {
        let db = crate::open_memory_database().unwrap();

        let result = get_product(&db, &uuid::Uuid::now_v7().to_string()).unwrap();
        assert!(result.is_none());
    }
}
