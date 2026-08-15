use byte_unit::Byte;
use tera::{Kwargs, State, TeraResult, Value};

/// Helper function to add commas as thousands separators
fn separate_with_commas(num_str: &str) -> String {
    if let Some((integer_part, decimal_part)) = num_str.split_once('.') {
        // Handle decimal numbers
        let formatted_integer = separate_integer_part(integer_part);
        format!("{formatted_integer}.{decimal_part}")
    } else {
        // Handle integers
        separate_integer_part(num_str)
    }
}

fn separate_integer_part(num_str: &str) -> String {
    let is_negative = num_str.starts_with('-');
    let num_str = if is_negative { &num_str[1..] } else { num_str };

    let len = num_str.len();
    let mut result = String::with_capacity(len + (len - 1) / 3);

    for (i, c) in num_str.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }

    if is_negative {
        format!("-{result}")
    } else {
        result
    }
}

// The `apply_*` functions below hold the actual formatting logic and take only
// a value, so they stay unit-testable. Tera 2 filters receive a `&State` that
// cannot be constructed outside the engine, so the registered filters are thin
// wrappers over these.

/// Formatting behind the `number_with_delimiter` filter.
fn apply_with_delimiter(value: &Value) -> Value {
    if value.is_number() {
        Value::from(separate_with_commas(&value.to_string()))
    } else {
        value.clone()
    }
}

/// Formatting behind the `number_to_human_size` filter.
fn apply_to_human_size(value: &Value) -> Value {
    Byte::parse_str(value.to_string(), true).map_or_else(
        |_| value.clone(),
        |byte| {
            // byte-unit 5 renders full precision by default; `{:.2}` preserves
            // the 2-decimal formatting that byte-unit 4 produced.
            Value::from(format!(
                "{:.2}",
                byte.get_appropriate_unit(byte_unit::UnitType::Decimal)
            ))
        },
    )
}

/// Formatting behind the `number_to_percentage` filter.
fn apply_to_percentage(value: &Value, format: Option<&str>) -> Value {
    if value.is_number() {
        Value::from(format.unwrap_or("%n%").replace("%n", &value.to_string()))
    } else {
        value.clone()
    }
}

/// Formats a numeric value by adding commas as thousands separators.
///
/// # Examples:
///
/// ```text
/// {{1000 | number_with_delimiter}}
/// ```
///
/// # Errors
///
/// If the `value` is not a numeric value, the function will return the original
/// value as a string without any error.
pub fn number_with_delimiter(value: &Value, _: Kwargs, _: &State<'_>) -> TeraResult<Value> {
    Ok(apply_with_delimiter(value))
}

/// Converts a numeric value (in bytes) into a human-readable size string with
/// appropriate units.
///
/// # Examples:
///
/// ```text
/// {{70691577 | number_to_human_size}}
/// ```
///
/// # Errors
///
/// If the `value` is not a numeric value, the function will return the original
/// value as a string without any error.
pub fn number_to_human_size(value: &Value, _: Kwargs, _: &State<'_>) -> TeraResult<Value> {
    Ok(apply_to_human_size(value))
}

/// Converts a numeric value into a formatted percentage string.
///
/// # Examples:
///
/// ```text
/// {{100 | number_to_percentage}}
/// {{100 | number_to_percentage(format='%n %')}}
/// ```
///
/// # Errors
///
/// Returns an error if the `format` argument is present but not a string.
// `Kwargs` by value is Tera's `Filter` signature, not a choice.
#[allow(clippy::needless_pass_by_value)]
pub fn number_to_percentage(value: &Value, kwargs: Kwargs, _: &State<'_>) -> TeraResult<Value> {
    let format: Option<String> = kwargs.get("format")?;
    Ok(apply_to_percentage(value, format.as_deref()))
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(Value::from(100), "100")]
    #[case(Value::from(100.2), "100.2")]
    #[case(Value::from(1000), "1,000")]
    #[case(Value::from(10000), "10,000")]
    #[case(Value::from(10000.1234), "10,000.1234")]
    #[case(Value::from(-100), "-100")]
    #[case(Value::from(-100.2), "-100.2")]
    #[case(Value::from(-1000), "-1,000")]
    #[case(Value::from(-10000), "-10,000")]
    #[case(Value::from(-10000.12345), "-10,000.12345")]
    #[case(Value::from("invalid"), "invalid")]
    #[case(Value::from(0), "0")]
    #[case(Value::from(0.123), "0.123")]
    #[case(Value::from(1_000_000), "1,000,000")]
    #[case(Value::from(1_000_000_000), "1,000,000,000")]
    #[case(Value::from(-0.123), "-0.123")]
    // Restored from the Tera 1 suite to prove no behaviour drift:
    #[case(Value::from(10000.1234), "10,000.1234")]
    #[case(Value::from("0.0"), "0.0")]
    #[case(Value::from(1_234_567_890.123_456), "1,234,567,890.123456")]
    #[case(Value::from(0.000_123), "0.000123")]
    #[case(Value::from("100.00"), "100.00")]
    #[case(Value::from(-1_234_567.89), "-1,234,567.89")]
    #[case(Value::from("100.00230"), "100.00230")]
    #[case(Value::from("0100.00230"), "0100.00230")]
    #[case(Value::from(""), "")]
    fn test_number_with_delimiter(#[case] input: Value, #[case] expected: &str) {
        assert_eq!(apply_with_delimiter(&input), Value::from(expected));
    }

    #[rstest]
    #[case(Value::from(1234), "1.23 KB")]
    #[case(Value::from(70_691_577), "70.69 MB")]
    #[case(Value::from("invalid"), "invalid")]
    fn test_number_to_human_size(#[case] input: Value, #[case] expected: &str) {
        assert_eq!(apply_to_human_size(&input), Value::from(expected));
    }

    #[rstest]
    #[case(Value::from(100), None, "100%")]
    #[case(Value::from(100), Some("%n %"), "100 %")]
    #[case(Value::from("invalid"), None, "invalid")]
    fn test_number_to_percentage(
        #[case] value: Value,
        #[case] format: Option<&str>,
        #[case] expected: &str,
    ) {
        assert_eq!(apply_to_percentage(&value, format), Value::from(expected));
    }
}
