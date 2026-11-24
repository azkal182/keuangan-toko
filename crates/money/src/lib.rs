/// Parses a positive NUMERIC(12,3) quantity into thousandths.
pub fn quantity_milli(input: &str) -> Option<i64> {
    let (whole, fraction) = input.split_once('.').unwrap_or((input, ""));
    if whole.is_empty()
        || whole.len() > 9
        || fraction.len() > 3
        || (input.contains('.') && fraction.is_empty())
        || !whole
            .bytes()
            .chain(fraction.bytes())
            .all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let whole: i64 = whole.parse().ok()?;
    let fraction_value: i64 = if fraction.is_empty() {
        0
    } else {
        fraction.parse().ok()?
    };
    let milli = whole * 1000 + fraction_value * 10_i64.pow(3 - fraction.len() as u32);
    (milli > 0).then_some(milli)
}

pub fn rupiah(input: &str) -> Option<i64> {
    if input.is_empty() || !input.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    input.parse().ok()
}

/// Rounds each positive quantity × price half-up without reducing intermediate precision.
pub fn line_total(milli: i64, price: i64) -> Option<i64> {
    if milli <= 0 || price < 0 {
        return None;
    }
    i64::try_from((i128::from(milli) * i128::from(price) + 500) / 1000).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_rounding_and_overflow() {
        assert_eq!(
            line_total(quantity_milli("1.250").unwrap(), 10001),
            Some(12501)
        );
        assert_eq!(line_total(quantity_milli("0.500").unwrap(), 101), Some(51));
        assert_eq!(line_total(1000, i64::MAX), Some(i64::MAX));
        assert_eq!(line_total(1001, i64::MAX), None);
        assert_eq!(quantity_milli("1.0000"), None);
        assert_eq!(quantity_milli("1000000000"), None);
        assert_eq!(quantity_milli("0.000"), None);
        assert_eq!(rupiah("12.50"), None);
    }
}
