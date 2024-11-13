#![allow(clippy::missing_panics_doc)]
use std::path::PathBuf;

use toml::Value;

#[must_use]
pub fn load(path: PathBuf) -> toml::Value {
    let s = std::fs::read_to_string(path).expect("could not open file");
    toml::from_str(&s).expect("invalid toml content")
}

pub fn assert_path_value_eq_string(toml: &Value, path: &[&str], expected: &str) {
    let expected_value = Value::String(expected.to_string());
    assert_path_value_eq(toml, path, &expected_value);
}

pub fn eq_path_value_eq_bool(toml: &Value, path: &[&str], expected: bool) {
    let expected_value = Value::Boolean(expected);
    assert_path_value_eq(toml, path, &expected_value);
}

pub fn assert_path_is_empty_array(toml: &Value, path: &[&str]) {
    let actual = get_value_at_path(toml, path);

    assert!(
        match actual {
            Some(Value::Array(arr)) => arr.is_empty(),
            None => true,
            _ => false,
        },
        "Assertion failed: Path {path:?} is not an empty array. Actual value: {actual:?}"
    );
}

/// Assert that the value at the specified path is an array and matches the
/// expected array.
pub fn assert_path_value_eq_array(toml: &Value, path: &[&str], expected: &[Value]) {
    let expected_value = Value::Array(expected.to_vec());
    assert_path_value_eq(toml, path, &expected_value);
}

/// Assert that a TOML value contains a specific key path and that it matches
/// the expected value.
pub fn assert_path_value_eq(toml: &Value, path: &[&str], expected: &Value) {
    let actual = get_value_at_path(toml, path);
    assert!(
        actual == Some(expected),
        "Assertion failed: Path {path:?} does not match expected value. Expected: {expected:?}, \
         Actual: {actual:?}"
    );
}

/// Assert that a TOML value contains a specific path, and that the value is an
/// object (table).
pub fn assert_path_is_object(toml: &Value, path: &[&str]) {
    let actual = get_value_at_path(toml, path).unwrap();
    assert!(
        matches!(actual, Value::Table(_)),
        "Assertion failed: Path {path:?} is not an object. Actual value: {actual:?}"
    );
}

/// Helper function to concatenate keys of a nested table to form a string.
#[must_use]
pub fn get_keys_concatenated_as_string(toml: &Value, path: &[&str]) -> Option<String> {
    let value_at_path = get_value_at_path(toml, path)?;
    if let Value::Table(table) = value_at_path {
        let mut concatenated_string = String::new();
        for key in table.keys() {
            concatenated_string.push_str(key);
        }
        Some(concatenated_string)
    } else {
        None
    }
}

/// Assert that the TOML value at the given path is empty (either an empty table
/// or array).
pub fn assert_path_is_empty(toml: &Value, path: &[&str]) {
    let actual = get_value_at_path(toml, path);

    assert!(
        match actual {
            Some(Value::Table(table)) => table.is_empty(),
            Some(Value::Array(arr)) => arr.is_empty(),
            None => true,
            _ => false,
        },
        "Assertion failed: Path {path:?} is not empty. Actual value: {actual:?}"
    );
}

pub fn assert_path_exists(toml: &Value, path: &[&str]) {
    let actual = get_value_at_path(toml, path);

    assert!(
        actual.is_some(),
        "Assertion failed: Path {path:?} does not exist. Actual value: {actual:?}"
    );
}

/// Internal helper function to traverse a TOML structure and get the value at a
/// specific path.
#[must_use]
pub fn get_value_at_path<'a>(toml: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = toml;
    for &key in path {
        match current {
            Value::Table(table) => {
                current = table.get(key)?;
            }
            Value::Array(arr) => match key.parse::<usize>() {
                Ok(index) => current = arr.get(index)?,
                Err(_) => return None,
            },
            _ => return None,
        }
    }
    Some(current)
}
