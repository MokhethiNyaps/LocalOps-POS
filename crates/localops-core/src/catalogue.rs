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
    if !connection.is_autocommit() {
        return create_product_rows(connection, input);
    }
    let transaction = connection.unchecked_transaction()?;
    let sellable_id = create_product_rows(&transaction, input)?;
    transaction.commit()?;
    Ok(sellable_id)
}

fn create_product_rows(connection: &Connection, input: NewProduct<'_>) -> Result<String> {
    let sellable_id = sellable::create_sellable_item(
        connection,
        input.business_id,
        input.name,
        "PRODUCT",
        input.price_minor,
        input.category_id,
        input.taxable,
    )?;
    product::create_product(
        connection,
        &sellable_id,
        input.base_unit_id,
        input.cost_minor,
        input.track_stock,
        input.minimum_quantity_micros,
    )?;
    Ok(sellable_id)
}

/// Creates the common sellable row and its service subtype atomically.
pub fn create_service(connection: &Connection, input: NewService<'_>) -> Result<String> {
    if !connection.is_autocommit() {
        return create_service_rows(connection, input);
    }
    let transaction = connection.unchecked_transaction()?;
    let sellable_id = create_service_rows(&transaction, input)?;
    transaction.commit()?;
    Ok(sellable_id)
}

fn create_service_rows(connection: &Connection, input: NewService<'_>) -> Result<String> {
    let sellable_id = sellable::create_sellable_item(
        connection,
        input.business_id,
        input.name,
        "SERVICE",
        input.price_minor,
        input.category_id,
        input.taxable,
    )?;
    service::create_service(connection, &sellable_id, input.duration_minutes)?;
    Ok(sellable_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CoreError, availability, business, category, department, packaging, recipe, unit};

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

    #[test]
    fn configures_a_complete_mixed_operation_catalogue() {
        let database = crate::open_memory_database().unwrap();
        let business_id = business::create_business(&database, "Mixed Operation").unwrap();
        let department_id =
            department::create_department(&database, &business_id, "Restaurant", None).unwrap();
        let food = category::create_category(&database, &business_id, "Food", None).unwrap();
        let each =
            unit::create_unit(&database, &business_id, "EA", "Each", "COUNT", 1, 1, 0).unwrap();
        let gram =
            unit::create_unit(&database, &business_id, "G", "Gram", "WEIGHT", 1, 1, 3).unwrap();
        let chicken = create_product(
            &database,
            NewProduct {
                business_id: &business_id,
                name: "Chicken",
                price_minor: 0,
                category_id: Some(&food),
                taxable: false,
                base_unit_id: &gram,
                cost_minor: 8,
                track_stock: true,
                minimum_quantity_micros: 2_000_000_000,
            },
        )
        .unwrap();
        let burger = create_product(
            &database,
            NewProduct {
                business_id: &business_id,
                name: "Chicken Burger",
                price_minor: 7_500,
                category_id: Some(&food),
                taxable: true,
                base_unit_id: &each,
                cost_minor: 3_000,
                track_stock: false,
                minimum_quantity_micros: 0,
            },
        )
        .unwrap();
        let delivery = create_service(
            &database,
            NewService {
                business_id: &business_id,
                name: "Delivery",
                price_minor: 1_500,
                category_id: None,
                taxable: true,
                duration_minutes: Some(20),
            },
        )
        .unwrap();
        packaging::create_packaging(
            &database,
            &business_id,
            &chicken,
            &gram,
            "1 kg bag",
            1_000,
            1,
            true,
            false,
        )
        .unwrap();
        recipe::replace_recipe(
            &database,
            &business_id,
            &burger,
            1_000_000,
            &[recipe::NewRecipeItem {
                ingredient_product_id: &chicken,
                quantity_micros: 150_000_000,
                unit_id: &gram,
            }],
        )
        .unwrap();
        availability::set_department_availability(
            &database,
            &department_id,
            &burger,
            Some(7_000),
            true,
        )
        .unwrap();
        availability::set_department_availability(&database, &department_id, &delivery, None, true)
            .unwrap();

        let department_items =
            availability::list_department_sellables(&database, &department_id).unwrap();
        let consumption = recipe::calculate_consumption(&database, &burger, 2_000_000).unwrap();
        assert_eq!(department_items.len(), 2);
        assert_eq!(
            department_items
                .iter()
                .find(|item| item.sellable_id == burger)
                .unwrap()
                .effective_price_minor,
            7_000
        );
        assert_eq!(consumption[0].quantity_micros, 300_000_000);
        assert_eq!(
            packaging::list_product_packaging(&database, &chicken)
                .unwrap()
                .len(),
            1
        );
    }
}
