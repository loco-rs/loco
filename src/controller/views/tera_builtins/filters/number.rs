use tera::{Kwargs, State, TeraResult, Value};

use crate::controller::views::helpers;

// The `apply_*` functions below hold the actual formatting logic and take only
// a value, so they stay unit-testable. Tera 2 filters receive a `&State` that
// cannot be constructed outside the engine, so the registered filters are thin
// wrappers over these.

/// Formatting behind the `number_with_delimiter` filter.
fn apply_with_delimiter(value: &Value, delimiter: &str, separator: &str) -> Value {
    if value.is_number() {
        Value::from(helpers::number_with_delimiter_using(
            value, delimiter, separator,
        ))
    } else {
        value.clone()
    }
}

/// Formatting behind the `number_to_human` filter.
fn apply_to_human(value: &Value, precision: u32, significant: bool) -> Value {
    let number = value
        .as_f64()
        .or_else(|| value.as_str().and_then(|s| s.trim().parse::<f64>().ok()));
    number.map_or_else(
        || value.clone(),
        |n| Value::from(helpers::number_to_human_with(n, precision, significant)),
    )
}

/// Formatting behind the `number_to_human_size` filter.
fn apply_to_human_size(value: &Value) -> Value {
    crate::byte_size::parse(&value.to_string()).map_or_else(
        |_| value.clone(),
        |bytes| Value::from(crate::byte_size::to_human(bytes)),
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
/// Takes Rails' `delimiter` (default `","`) and `separator` (default `"."`)
/// keyword arguments.
///
/// # Examples:
///
/// ```text
/// {{1000 | number_with_delimiter}}
/// {{1234567.5 | number_with_delimiter(delimiter=".", separator=",")}}
/// ```
///
/// # Errors
///
/// Returns an error if `delimiter` or `separator` is not a string. If the
/// `value` is not a numeric value, it is returned unchanged.
// `Kwargs` by value is Tera's `Filter` signature, not a choice.
#[allow(clippy::needless_pass_by_value)]
pub fn number_with_delimiter(value: &Value, kwargs: Kwargs, _: &State<'_>) -> TeraResult<Value> {
    let delimiter: &str = kwargs.get("delimiter")?.unwrap_or(",");
    let separator: &str = kwargs.get("separator")?.unwrap_or(".");
    Ok(apply_with_delimiter(value, delimiter, separator))
}

/// Writes a number in words for people: `1234` → `"1.23 Thousand"`,
/// `1500000` → `"1.5 Million"`.
///
/// Takes Rails' `precision` (default 3) and `significant` (default `true`)
/// keyword arguments.
///
/// # Examples:
///
/// ```text
/// {{ views | number_to_human }}
/// {{ views | number_to_human(precision=2) }}
/// ```
///
/// # Errors
///
/// Returns an error if `precision` or `significant` has the wrong type. If the
/// `value` is not a numeric value, it is returned unchanged.
#[allow(clippy::needless_pass_by_value)]
pub fn number_to_human(value: &Value, kwargs: Kwargs, _: &State<'_>) -> TeraResult<Value> {
    let precision: u32 = kwargs.get("precision")?.unwrap_or(3);
    let significant: bool = kwargs.get("significant")?.unwrap_or(true);
    Ok(apply_to_human(value, precision, significant))
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
        assert_eq!(
            apply_with_delimiter(&input, ",", "."),
            Value::from(expected)
        );
    }

    #[test]
    fn test_number_with_delimiter_options() {
        assert_eq!(
            apply_with_delimiter(&Value::from(1_234_567.5), ".", ","),
            Value::from("1.234.567,5")
        );
    }

    #[rstest]
    #[case(Value::from(123), 3, "123")]
    #[case(Value::from(1234), 3, "1.23 Thousand")]
    #[case(Value::from(1_234_567), 3, "1.23 Million")]
    #[case(Value::from(1_234_567), 2, "1.2 Million")]
    #[case(Value::from("1500000"), 3, "1.5 Million")]
    #[case(Value::from("invalid"), 3, "invalid")]
    fn test_number_to_human(#[case] input: Value, #[case] precision: u32, #[case] expected: &str) {
        assert_eq!(
            apply_to_human(&input, precision, true),
            Value::from(expected)
        );
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
