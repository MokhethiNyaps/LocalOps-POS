use crate::{CoreError, Result};
use rusqlite::Connection;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DepartmentSellable {
    pub department_id: String,
    pub sellable_id: String,
    pub name: String,
    pub kind: String,
    pub base_price_minor: i64,
    pub price_override_minor: Option<i64>,
    pub effective_price_minor: i64,
    pub active: bool,
}

/// Makes a product or service available in a department. The INSERT...SELECT
/// enforces shared business ownership in the same database statement.
pub fn set_department_availability(
    connection: &Connection,
    department_id: &str,
    sellable_id: &str,
    price_override_minor: Option<i64>,
    active: bool,
) -> Result<()> {
    if price_override_minor.is_some_and(|price| price < 0) {
        return Err(CoreError::Database(rusqlite::Error::InvalidParameterName(
            "price_override_minor must be non-negative".to_owned(),
        )));
    }

    let changed = connection.execute(
        "INSERT INTO department_sellables(
             department_id, sellable_id, price_override_minor, active
         )
         SELECT d.id, s.id, ?3, ?4
         FROM departments d
         JOIN sellable_items s ON s.business_id = d.business_id
         WHERE d.id = ?1 AND s.id = ?2 AND d.active = 1 AND s.active = 1
         ON CONFLICT(department_id, sellable_id) DO UPDATE SET
             price_override_minor = excluded.price_override_minor,
             active = excluded.active",
        (department_id, sellable_id, price_override_minor, active),
    )?;
    if changed == 0 {
        return Err(CoreError::CrossBusinessReference);
    }
    Ok(())
}

pub fn list_department_sellables(
    connection: &Connection,
    department_id: &str,
) -> Result<Vec<DepartmentSellable>> {
    let mut statement = connection.prepare(
        "SELECT ds.department_id, ds.sellable_id, s.name, s.kind, s.price_minor,
                ds.price_override_minor, COALESCE(ds.price_override_minor, s.price_minor), ds.active
         FROM department_sellables ds
         JOIN sellable_items s ON s.id = ds.sellable_id
         WHERE ds.department_id = ?1 AND ds.active = 1 AND s.active = 1
         ORDER BY s.name",
    )?;
    statement
        .query_map([department_id], |row| {
            Ok(DepartmentSellable {
                department_id: row.get(0)?,
                sellable_id: row.get(1)?,
                name: row.get(2)?,
                kind: row.get(3)?,
                base_price_minor: row.get(4)?,
                price_override_minor: row.get(5)?,
                effective_price_minor: row.get(6)?,
                active: row.get(7)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{business, department, sellable};

    #[test]
    fn makes_products_and_services_available_with_effective_prices() {
        let database = crate::open_memory_database().unwrap();
        let business_id = business::create_business(&database, "Test Business").unwrap();
        let department_id =
            department::create_department(&database, &business_id, "Bar", None).unwrap();
        let product_id = sellable::create_sellable_item(
            &database,
            &business_id,
            "Castle Lager",
            "PRODUCT",
            2500,
            None,
            true,
        )
        .unwrap();
        let service_id = sellable::create_sellable_item(
            &database,
            &business_id,
            "Delivery",
            "SERVICE",
            500,
            None,
            false,
        )
        .unwrap();

        set_department_availability(&database, &department_id, &product_id, Some(2200), true)
            .unwrap();
        set_department_availability(&database, &department_id, &service_id, None, true).unwrap();

        let available = list_department_sellables(&database, &department_id).unwrap();
        assert_eq!(available.len(), 2);
        assert_eq!(available[0].effective_price_minor, 2200);
        assert_eq!(available[1].effective_price_minor, 500);
    }

    #[test]
    fn blocks_cross_business_availability() {
        let database = crate::open_memory_database().unwrap();
        let business_a = business::create_business(&database, "Business A").unwrap();
        let business_b = business::create_business(&database, "Business B").unwrap();
        let department_id =
            department::create_department(&database, &business_a, "Bar", None).unwrap();
        let sellable_id = sellable::create_sellable_item(
            &database,
            &business_b,
            "Foreign item",
            "PRODUCT",
            100,
            None,
            true,
        )
        .unwrap();
        assert!(matches!(
            set_department_availability(&database, &department_id, &sellable_id, None, true),
            Err(CoreError::CrossBusinessReference)
        ));
    }

    #[test]
    fn deactivates_availability_without_deleting_history() {
        let database = crate::open_memory_database().unwrap();
        let business_id = business::create_business(&database, "Test Business").unwrap();
        let department_id =
            department::create_department(&database, &business_id, "Bar", None).unwrap();
        let sellable_id = sellable::create_sellable_item(
            &database,
            &business_id,
            "Item",
            "PRODUCT",
            100,
            None,
            true,
        )
        .unwrap();
        set_department_availability(&database, &department_id, &sellable_id, None, true).unwrap();
        set_department_availability(&database, &department_id, &sellable_id, None, false).unwrap();

        assert!(
            list_department_sellables(&database, &department_id)
                .unwrap()
                .is_empty()
        );
        let stored: i64 = database
            .query_row(
                "SELECT active FROM department_sellables
                 WHERE department_id = ?1 AND sellable_id = ?2",
                (&department_id, &sellable_id),
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stored, 0);
    }
}
