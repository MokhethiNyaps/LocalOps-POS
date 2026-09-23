use crate::{CoreError, Result, conversion};
use rusqlite::{Connection, OptionalExtension};
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub struct NewRecipeItem<'a> {
    pub ingredient_product_id: &'a str,
    pub quantity_micros: i64,
    pub unit_id: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecipeConsumption {
    pub ingredient_product_id: String,
    pub base_unit_id: String,
    pub quantity_micros: i64,
}

pub fn replace_recipe(
    connection: &Connection,
    business_id: &str,
    owner_sellable_id: &str,
    yield_quantity_micros: i64,
    items: &[NewRecipeItem<'_>],
) -> Result<String> {
    if yield_quantity_micros <= 0 || items.is_empty() {
        return Err(CoreError::InvalidRecipeQuantity);
    }
    if !connection.is_autocommit() {
        return replace_recipe_rows(
            connection,
            business_id,
            owner_sellable_id,
            yield_quantity_micros,
            items,
        );
    }
    let transaction = connection.unchecked_transaction()?;
    let recipe_id = replace_recipe_rows(
        &transaction,
        business_id,
        owner_sellable_id,
        yield_quantity_micros,
        items,
    )?;
    transaction.commit()?;
    Ok(recipe_id)
}

fn replace_recipe_rows(
    connection: &Connection,
    business_id: &str,
    owner_sellable_id: &str,
    yield_quantity_micros: i64,
    items: &[NewRecipeItem<'_>],
) -> Result<String> {
    let owner_exists = connection
        .query_row(
            "SELECT 1 FROM sellable_items
             WHERE id = ?1 AND business_id = ?2 AND active = 1",
            (owner_sellable_id, business_id),
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if !owner_exists {
        return Err(CoreError::CrossBusinessReference);
    }

    connection.execute(
        "UPDATE recipes
         SET active = 0, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
         WHERE owner_sellable_id = ?1 AND active = 1",
        [owner_sellable_id],
    )?;
    let recipe_id = Uuid::now_v7().to_string();
    connection.execute(
        "INSERT INTO recipes(id, business_id, owner_sellable_id, yield_quantity_micros)
         VALUES(?1, ?2, ?3, ?4)",
        (
            &recipe_id,
            business_id,
            owner_sellable_id,
            yield_quantity_micros,
        ),
    )?;

    for item in items {
        if item.quantity_micros <= 0 {
            return Err(CoreError::InvalidRecipeQuantity);
        }
        let base_unit_id = connection
            .query_row(
                "SELECT p.base_unit_id
                 FROM products p
                 JOIN sellable_items s ON s.id = p.sellable_id
                 WHERE p.sellable_id = ?1 AND s.business_id = ?2 AND s.active = 1",
                (item.ingredient_product_id, business_id),
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or(CoreError::CrossBusinessReference)?;
        conversion::convert_quantity_micros(
            connection,
            business_id,
            item.unit_id,
            &base_unit_id,
            item.quantity_micros,
        )?;
        connection.execute(
            "INSERT INTO recipe_items(
                 id, recipe_id, ingredient_product_id, quantity_micros, unit_id
             ) VALUES(?1, ?2, ?3, ?4, ?5)",
            (
                Uuid::now_v7().to_string(),
                &recipe_id,
                item.ingredient_product_id,
                item.quantity_micros,
                item.unit_id,
            ),
        )?;
    }
    Ok(recipe_id)
}

pub fn calculate_consumption(
    connection: &Connection,
    owner_sellable_id: &str,
    sold_quantity_micros: i64,
) -> Result<Vec<RecipeConsumption>> {
    if sold_quantity_micros <= 0 {
        return Err(CoreError::InvalidRecipeQuantity);
    }
    let recipe = connection
        .query_row(
            "SELECT id, business_id, yield_quantity_micros
             FROM recipes WHERE owner_sellable_id = ?1 AND active = 1",
            [owner_sellable_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .optional()?
        .ok_or(CoreError::RecipeNotFound)?;

    let mut statement = connection.prepare(
        "SELECT ri.ingredient_product_id, ri.quantity_micros, ri.unit_id, p.base_unit_id
         FROM recipe_items ri
         JOIN products p ON p.sellable_id = ri.ingredient_product_id
         WHERE ri.recipe_id = ?1
         ORDER BY ri.ingredient_product_id",
    )?;
    let rows = statement.query_map([&recipe.0], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;
    let mut consumption = Vec::new();
    for row in rows {
        let (product_id, per_yield_micros, unit_id, base_unit_id) = row?;
        let required_in_recipe_unit = conversion::checked_ratio(
            sold_quantity_micros,
            i128::from(per_yield_micros),
            i128::from(recipe.2),
        )?;
        let base_quantity = conversion::convert_quantity_micros(
            connection,
            &recipe.1,
            &unit_id,
            &base_unit_id,
            required_in_recipe_unit,
        )?;
        consumption.push(RecipeConsumption {
            ingredient_product_id: product_id,
            base_unit_id,
            quantity_micros: base_quantity,
        });
    }
    Ok(consumption)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{business, catalogue, open_memory_database, unit};

    struct Fixture {
        database: Connection,
        business_id: String,
        gram: String,
        each: String,
        chicken: String,
        bun: String,
    }

    fn fixture() -> Fixture {
        let database = open_memory_database().unwrap();
        let business_id = business::create_business(&database, "Restaurant").unwrap();
        let gram =
            unit::create_unit(&database, &business_id, "G", "Gram", "MASS", 1, 1, 3).unwrap();
        let each =
            unit::create_unit(&database, &business_id, "EA", "Each", "COUNT", 1, 1, 0).unwrap();
        let chicken = catalogue::create_product(
            &database,
            catalogue::NewProduct {
                business_id: &business_id,
                name: "Chicken",
                price_minor: 0,
                category_id: None,
                taxable: true,
                base_unit_id: &gram,
                cost_minor: 0,
                track_stock: true,
                minimum_quantity_micros: 0,
            },
        )
        .unwrap();
        let bun = catalogue::create_product(
            &database,
            catalogue::NewProduct {
                business_id: &business_id,
                name: "Bun",
                price_minor: 0,
                category_id: None,
                taxable: true,
                base_unit_id: &each,
                cost_minor: 0,
                track_stock: true,
                minimum_quantity_micros: 0,
            },
        )
        .unwrap();
        Fixture {
            database,
            business_id,
            gram,
            each,
            chicken,
            bun,
        }
    }

    #[test]
    fn calculates_product_recipe_consumption_for_multiple_sales() {
        let fixture = fixture();
        let burger = catalogue::create_product(
            &fixture.database,
            catalogue::NewProduct {
                business_id: &fixture.business_id,
                name: "Chicken Burger",
                price_minor: 8500,
                category_id: None,
                taxable: true,
                base_unit_id: &fixture.each,
                cost_minor: 0,
                track_stock: false,
                minimum_quantity_micros: 0,
            },
        )
        .unwrap();
        replace_recipe(
            &fixture.database,
            &fixture.business_id,
            &burger,
            1_000_000,
            &[
                NewRecipeItem {
                    ingredient_product_id: &fixture.chicken,
                    quantity_micros: 150_000_000,
                    unit_id: &fixture.gram,
                },
                NewRecipeItem {
                    ingredient_product_id: &fixture.bun,
                    quantity_micros: 1_000_000,
                    unit_id: &fixture.each,
                },
            ],
        )
        .unwrap();
        let consumption = calculate_consumption(&fixture.database, &burger, 3_000_000).unwrap();
        assert_eq!(
            consumption
                .iter()
                .find(|item| item.ingredient_product_id == fixture.bun)
                .unwrap()
                .quantity_micros,
            3_000_000
        );
        assert_eq!(
            consumption
                .iter()
                .find(|item| item.ingredient_product_id == fixture.chicken)
                .unwrap()
                .quantity_micros,
            450_000_000
        );
    }

    #[test]
    fn supports_service_consumables_and_preserves_replaced_recipe() {
        let fixture = fixture();
        let service = catalogue::create_service(
            &fixture.database,
            catalogue::NewService {
                business_id: &fixture.business_id,
                name: "Premium Wash",
                price_minor: 10_000,
                category_id: None,
                taxable: true,
                duration_minutes: Some(45),
            },
        )
        .unwrap();
        let first = replace_recipe(
            &fixture.database,
            &fixture.business_id,
            &service,
            1_000_000,
            &[NewRecipeItem {
                ingredient_product_id: &fixture.chicken,
                quantity_micros: 10_000_000,
                unit_id: &fixture.gram,
            }],
        )
        .unwrap();
        replace_recipe(
            &fixture.database,
            &fixture.business_id,
            &service,
            1_000_000,
            &[NewRecipeItem {
                ingredient_product_id: &fixture.chicken,
                quantity_micros: 20_000_000,
                unit_id: &fixture.gram,
            }],
        )
        .unwrap();
        let old_active: bool = fixture
            .database
            .query_row(
                "SELECT active FROM recipes WHERE id = ?1",
                [&first],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!old_active);
        assert_eq!(
            calculate_consumption(&fixture.database, &service, 1_000_000).unwrap()[0]
                .quantity_micros,
            20_000_000
        );
    }
}
