use crate::{CoreError, Result};
use rusqlite::{Connection, OptionalExtension};

#[derive(Debug, Clone, PartialEq, Eq)]
struct UnitFactor {
    dimension: String,
    numerator: i64,
    denominator: i64,
}

pub fn convert_quantity_micros(
    connection: &Connection,
    business_id: &str,
    from_unit_id: &str,
    to_unit_id: &str,
    quantity_micros: i64,
) -> Result<i64> {
    let from = load_factor(connection, business_id, from_unit_id)?;
    let to = load_factor(connection, business_id, to_unit_id)?;
    if from.dimension != to.dimension {
        return Err(CoreError::IncompatibleUnits);
    }
    checked_ratio(
        quantity_micros,
        i128::from(from.numerator) * i128::from(to.denominator),
        i128::from(from.denominator) * i128::from(to.numerator),
    )
}

pub fn checked_ratio(value: i64, numerator: i128, denominator: i128) -> Result<i64> {
    if denominator <= 0 || numerator <= 0 {
        return Err(CoreError::IncompatibleUnits);
    }
    let scaled = i128::from(value)
        .checked_mul(numerator)
        .ok_or(CoreError::QuantityOverflow)?;
    if scaled % denominator != 0 {
        return Err(CoreError::InexactQuantity);
    }
    i64::try_from(scaled / denominator).map_err(|_| CoreError::QuantityOverflow)
}

fn load_factor(connection: &Connection, business_id: &str, unit_id: &str) -> Result<UnitFactor> {
    connection
        .query_row(
            "SELECT dimension, scale_num, scale_den
             FROM units WHERE id = ?1 AND business_id = ?2 AND active = 1",
            (unit_id, business_id),
            |row| {
                Ok(UnitFactor {
                    dimension: row.get(0)?,
                    numerator: row.get(1)?,
                    denominator: row.get(2)?,
                })
            },
        )
        .optional()?
        .ok_or(CoreError::UnitNotFound)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{business, open_memory_database, unit};

    #[test]
    fn converts_exact_metric_quantities() {
        let database = open_memory_database().unwrap();
        let business_id = business::create_business(&database, "Test").unwrap();
        let gram =
            unit::create_unit(&database, &business_id, "G", "Gram", "MASS", 1, 1, 3).unwrap();
        let kilogram = unit::create_unit(
            &database,
            &business_id,
            "KG",
            "Kilogram",
            "MASS",
            1000,
            1,
            3,
        )
        .unwrap();
        assert_eq!(
            convert_quantity_micros(&database, &business_id, &kilogram, &gram, 1_500_000).unwrap(),
            1_500_000_000
        );
    }

    #[test]
    fn rejects_incompatible_and_inexact_conversions() {
        let database = open_memory_database().unwrap();
        let business_id = business::create_business(&database, "Test").unwrap();
        let each =
            unit::create_unit(&database, &business_id, "EA", "Each", "COUNT", 1, 1, 0).unwrap();
        let litre =
            unit::create_unit(&database, &business_id, "L", "Litre", "VOLUME", 1, 1, 3).unwrap();
        assert!(matches!(
            convert_quantity_micros(&database, &business_id, &each, &litre, 1_000_000),
            Err(CoreError::IncompatibleUnits)
        ));
        assert!(matches!(
            checked_ratio(1, 1, 3),
            Err(CoreError::InexactQuantity)
        ));
    }
}
