use crate::{CoreError, Result};

pub fn round_ratio(numerator: i128, denominator: i128) -> Result<i64> {
    if denominator <= 0 {
        return Err(CoreError::MoneyOverflow);
    }
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    let rounded = if remainder.abs() * 2 >= denominator {
        quotient + numerator.signum()
    } else {
        quotient
    };
    i64::try_from(rounded).map_err(|_| CoreError::MoneyOverflow)
}

pub fn multiply_minor_by_quantity(unit_amount_minor: i64, quantity_micros: i64) -> Result<i64> {
    round_ratio(
        i128::from(unit_amount_minor) * i128::from(quantity_micros),
        1_000_000,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounds_half_away_from_zero() {
        assert_eq!(round_ratio(1, 2).unwrap(), 1);
        assert_eq!(round_ratio(-1, 2).unwrap(), -1);
        assert_eq!(round_ratio(4, 3).unwrap(), 1);
        assert_eq!(round_ratio(5, 3).unwrap(), 2);
    }

    #[test]
    fn calculates_fractional_quantity_line_total() {
        assert_eq!(multiply_minor_by_quantity(1_000, 1_500_000).unwrap(), 1_500);
    }
}
