use crate::{Result, product, sellable, service};
use rusqlite::Connection;

#[derive(Debug, Clone, Copy)]
pub struct NewProduct<'a> {
    pub business_id: &'a str,
    pub name: &'a str,
    pub price_minor: i64,
    pub category_id: Option<&'a str>,
    pub taxable: bool,
    pub base_unit_id: &'a str,
    pub cost_minor: i64,
    pub track_stock: bool,
    pub minimum_quantity_micros: i64,
}

#[derive(Debug, Clone, Copy)]
pub struct NewService<'a> {
    pub business_id: &'a str,
    pub name: &'a str,
    pub price_minor: i64,
    pub category_id: Option<&'a str>,
    pub taxable: bool,
    pub duration_minutes: Option<i32>,
}

/// Creates the common sellable row and its product subtype atomically.
pub fn create_product(connection: &Connection, input: NewProduct<'_>) -> Result<String> {
    let transaction = connection.unchecked_transaction()?;
    let sellable_id = sellable::create_sellable_item(
        &transaction,
        input.business_id,
        input.name,
        "PRODUCT",
        input.price_minor,
        input.category_id,
        input.taxable,
    )?;
    product::create_product(
        &transaction,
        &sellable_id,
        input.base_unit_id,
        input.cost_minor,
        input.track_stock,
        input.minimum_quantity_micros,
    )?;
    transaction.commit()?;
    Ok(sellable_id)
}

/// Creates the common sellable row and its service subtype atomically.
pub fn create_service(connection: &Connection, input: NewService<'_>) -> Result<String> {
    let transaction = connection.unchecked_transaction()?;
    let sellable_id = sellable::create_sellable_item(
        &transaction,
        input.business_id,
        input.name,
        "SERVICE",
        input.price_minor,
        input.category_id,
        input.taxable,
    )?;
    service::create_service(&transaction, &sellable_id, input.duration_minutes)?;
    transaction.commit()?;
    Ok(sellable_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CoreError, business, unit};

    #[test]
    fn creates_product_aggregate_atomically() {
        let database = crate::open_memory_database().unwrap();
        let business_id = business::create_business(&database, "Test Business").unwrap();
        let unit_id =
            unit::create_unit(&database, &business_id, "EA", "Each", "COUNT", 1, 1, 0).unwrap();
        let id = create_product(
            &database,
            NewProduct {
                business_id: &business_id,
                name: "Coffee",
                price_minor: 2500,
                category_id: None,
                taxable: true,
                base_unit_id: &unit_id,
                cost_minor: 1400,
                track_stock: true,
                minimum_quantity_micros: 5_000_000,
            },
        )
        .unwrap();
        assert!(
            sellable::get_sellable_item(&database, &id)
                .unwrap()
                .is_some()
        );
        assert!(product::get_product(&database, &id).unwrap().is_some());
    }

    #[test]
    fn rolls_back_sellable_when_product_details_fail() {
        let database = crate::open_memory_database().unwrap();
        let business_id = business::create_business(&database, "Test Business").unwrap();
        let result = create_product(
            &database,
            NewProduct {
                business_id: &business_id,
                name: "Broken product",
                price_minor: 100,
                category_id: None,
                taxable: true,
                base_unit_id: "missing-unit",
                cost_minor: 50,
                track_stock: true,
                minimum_quantity_micros: 0,
            },
        );
        assert!(matches!(result, Err(CoreError::UnitNotFound)));
        let count: i64 = database
            .query_row(
                "SELECT count(*) FROM sellable_items WHERE name = 'Broken product'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn creates_service_aggregate_atomically() {
        let database = crate::open_memory_database().unwrap();
        let business_id = business::create_business(&database, "Test Business").unwrap();
        let id = create_service(
            &database,
            NewService {
                business_id: &business_id,
                name: "Premium Wash",
                price_minor: 10_000,
                category_id: None,
                taxable: true,
                duration_minutes: Some(45),
            },
        )
        .unwrap();
        assert!(service::get_service(&database, &id).unwrap().is_some());
    }
}
