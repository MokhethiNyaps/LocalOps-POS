use crate::{CoreError, Result, conversion};
use rusqlite::{Connection, OptionalExtension};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductPackaging {
    pub id: String,
    pub business_id: String,
    pub product_id: String,
    pub unit_id: String,
    pub name: String,
    pub factor_num: i64,
    pub factor_den: i64,
    pub can_purchase: bool,
    pub can_sell: bool,
}

#[allow(clippy::too_many_arguments)]
pub fn create_packaging(
    connection: &Connection,
    business_id: &str,
    product_id: &str,
    unit_id: &str,
    name: &str,
    factor_num: i64,
    factor_den: i64,
    can_purchase: bool,
    can_sell: bool,
) -> Result<String> {
    if factor_num <= 0 || factor_den <= 0 || name.trim().is_empty() {
        return Err(CoreError::IncompatibleUnits);
    }
    let valid = connection
        .query_row(
            "SELECT 1
             FROM products p
             JOIN sellable_items s ON s.id = p.sellable_id
             JOIN units u ON u.business_id = s.business_id
             WHERE p.sellable_id = ?1 AND u.id = ?2 AND s.business_id = ?3",
            (product_id, unit_id, business_id),
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if !valid {
        return Err(CoreError::CrossBusinessReference);
    }

    let id = Uuid::now_v7().to_string();
    connection.execute(
        "INSERT INTO product_packaging(
             id, business_id, product_id, unit_id, name, factor_num, factor_den,
             can_purchase, can_sell
         ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        (
            &id,
            business_id,
            product_id,
            unit_id,
            name.trim(),
            factor_num,
            factor_den,
            can_purchase,
            can_sell,
        ),
    )?;
    Ok(id)
}

pub fn list_product_packaging(
    connection: &Connection,
    product_id: &str,
) -> Result<Vec<ProductPackaging>> {
    let mut statement = connection.prepare(
        "SELECT id, business_id, product_id, unit_id, name, factor_num, factor_den,
                can_purchase, can_sell
         FROM product_packaging WHERE product_id = ?1 ORDER BY name",
    )?;
    statement
        .query_map([product_id], map_packaging)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn packaging_to_base_micros(
    connection: &Connection,
    packaging_id: &str,
    quantity_micros: i64,
) -> Result<i64> {
    let packaging = connection
        .query_row(
            "SELECT id, business_id, product_id, unit_id, name, factor_num, factor_den,
                    can_purchase, can_sell
             FROM product_packaging WHERE id = ?1",
            [packaging_id],
            map_packaging,
        )
        .optional()?
        .ok_or(CoreError::PackagingNotFound)?;
    conversion::checked_ratio(
        quantity_micros,
        i128::from(packaging.factor_num),
        i128::from(packaging.factor_den),
    )
}

pub fn set_packaging_usage(
    connection: &Connection,
    packaging_id: &str,
    can_purchase: bool,
    can_sell: bool,
) -> Result<()> {
    let changed = connection.execute(
        "UPDATE product_packaging
         SET can_purchase = ?2, can_sell = ?3,
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
         WHERE id = ?1",
        (packaging_id, can_purchase, can_sell),
    )?;
    if changed == 0 {
        return Err(CoreError::PackagingNotFound);
    }
    Ok(())
}

fn map_packaging(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProductPackaging> {
    Ok(ProductPackaging {
        id: row.get(0)?,
        business_id: row.get(1)?,
        product_id: row.get(2)?,
        unit_id: row.get(3)?,
        name: row.get(4)?,
        factor_num: row.get(5)?,
        factor_den: row.get(6)?,
        can_purchase: row.get(7)?,
        can_sell: row.get(8)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{business, catalogue, open_memory_database, unit};

    #[test]
    fn converts_product_specific_crates_to_bottles_exactly() {
        let database = open_memory_database().unwrap();
        let business_id = business::create_business(&database, "Test").unwrap();
        let bottle =
            unit::create_unit(&database, &business_id, "BTL", "Bottle", "COUNT", 1, 1, 0).unwrap();
        let crate_unit =
            unit::create_unit(&database, &business_id, "CRT", "Crate", "COUNT", 1, 1, 0).unwrap();
        let product_id = catalogue::create_product(
            &database,
            catalogue::NewProduct {
                business_id: &business_id,
                name: "Castle Lager",
                price_minor: 2500,
                category_id: None,
                taxable: true,
                base_unit_id: &bottle,
                cost_minor: 1600,
                track_stock: true,
                minimum_quantity_micros: 0,
            },
        )
        .unwrap();
        let packaging_id = create_packaging(
            &database,
            &business_id,
            &product_id,
            &crate_unit,
            "Crate of 12",
            12,
            1,
            true,
            false,
        )
        .unwrap();
        assert_eq!(
            packaging_to_base_micros(&database, &packaging_id, 5_000_000).unwrap(),
            60_000_000
        );
    }

    #[test]
    fn rejects_cross_business_packaging() {
        let database = open_memory_database().unwrap();
        let business_a = business::create_business(&database, "A").unwrap();
        let business_b = business::create_business(&database, "B").unwrap();
        let each_a =
            unit::create_unit(&database, &business_a, "EA", "Each", "COUNT", 1, 1, 0).unwrap();
        let each_b =
            unit::create_unit(&database, &business_b, "EA", "Each", "COUNT", 1, 1, 0).unwrap();
        let product_id = catalogue::create_product(
            &database,
            catalogue::NewProduct {
                business_id: &business_a,
                name: "Item",
                price_minor: 100,
                category_id: None,
                taxable: true,
                base_unit_id: &each_a,
                cost_minor: 50,
                track_stock: true,
                minimum_quantity_micros: 0,
            },
        )
        .unwrap();
        assert!(matches!(
            create_packaging(
                &database,
                &business_a,
                &product_id,
                &each_b,
                "Foreign",
                1,
                1,
                true,
                true
            ),
            Err(CoreError::CrossBusinessReference)
        ));
    }
}
