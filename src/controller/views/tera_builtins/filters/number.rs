#![allow(clippy::implicit_hasher)]
use byte_unit::Byte;
use serde_json::value::Value;
use std::collections::HashMap;
use tera::Result;
use thousands::Separable;

/// Formats a numeric value by adding commas as thousands separators.
///
/// # Errors
///
/// If the `value` is not a numeric value, the function will return the original
/// value as a string without any error.
pub fn number_with_delimiter<S: ::std::hash::BuildHasher>(
    value: &Value,
    _: &HashMap<String, Value, S>,
) -> Result<Value> {
    match value {
        Value::Number(number) => Ok(Value::String(number.separate_with_commas())),
        _ => Ok(value.clone()),
    }
}

/// Converts a numeric value (in bytes) into a human-readable size string with appropriate units.
///
/// # Errors
///
/// If the `value` is not a numeric value, the function will return the original
/// value as a string without any error.
pub fn number_to_human_size<S: ::std::hash::BuildHasher>(
    value: &Value,
    _: &HashMap<String, Value, S>,
) -> Result<Value> {
    Byte::from_str(value.to_string()).map_or_else(
        |_| Ok(value.clone()),
        |byte_unit| {
            Ok(Value::String(
                byte_unit.get_appropriate_unit(false).to_string(),
            ))
        },
    )
}

/// Converts a numeric value into a formatted percentage string.
///
/// # Errors
///
/// If the `value` is not a numeric value, the function will return the original
/// value as a string without any error.
pub fn number_to_percentage(value: &Value, options: &HashMap<String, Value>) -> Result<Value> {
    match value {
        Value::Number(number) => {
            let format = options
                .get("format")
                .and_then(|v| v.as_str())
                .unwrap_or("%n%");

            Ok(Value::String(format.replace("%n", &number.to_string())))
        }
        _ => Ok(value.clone()),
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use serde_json::json;
    use std::collections::HashMap;

    use super::*;

    #[rstest]
    #[case(json!(100), "100")]
    #[case(json!(100.2), "100.2")]
    #[case(json!(1000), "1,000")]
    #[case(json!(10000), "10,000")]
    #[case(json!(10000.1234), "10,000.1234")]
    #[case(json!(-100), "-100")]
    #[case(json!(-100.2), "-100.2")]
    #[case(json!(-1000), "-1,000")]
    #[case(json!(-10000), "-10,000")]
    #[case(json!(-10000.12345), "-10,000.12345")]
    #[case(json!("invalid"), "invalid")]
    fn test_number_with_delimiter(#[case] input: Value, #[case] expected: &str) {
        let result = number_with_delimiter(&input, &HashMap::new()).unwrap();
        assert_eq!(result, Value::String(expected.to_string()));
    }

    #[rstest]
    #[case(json!(1234), "1.23 KB")]
    #[case(json!(70_691_577), "70.69 MB")]
    #[case(json!("invalid"), "invalid")]
    fn test_number_to_human_size(#[case] input: Value, #[case] expected: &str) {
        let result = number_to_human_size(&input, &HashMap::new()).unwrap();
        assert_eq!(result, Value::String(expected.to_string()));
    }

    #[rstest]
    #[case(json!(100), HashMap::new(), "100%")]
    #[case(json!(100), HashMap::from([("format".to_string(), Value::String("%n %".to_string()))]), "100 %")]
    #[case(json!("invalid"), HashMap::new(), "invalid")]
    fn test_number_to_percentage(
        #[case] value: Value,
        #[case] options: HashMap<String, Value>,
        #[case] expected: &str,
    ) {
        assert_eq!(
            number_to_percentage(&value, &options).unwrap(),
            Value::String(expected.to_string())
        );
    }
}
