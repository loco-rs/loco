#![allow(clippy::missing_panics_doc)]
use std::{fs::File, io::BufReader, path::PathBuf};

use serde_yaml::Value;

#[must_use]
pub fn load(path: PathBuf) -> serde_yaml::Value {
    let file = File::open(path).expect("could not open file");
    let reader = BufReader::new(file);
    serde_yaml::from_reader(reader).expect("invalid yaml content")
}

pub fn assert_path_value_eq_string(yml: &Value, path: &[&str], expected: &str) {
    let expected_value = Value::String(expected.to_string());
    assert_path_value_eq(yml, path, &expected_value);
}

/// Asserts that the YAML value at the specified path is equal to the expected
/// boolean value.
pub fn assert_path_value_eq_bool(yml: &Value, path: &[&str], expected: bool) {
    let expected_value = Value::Bool(expected);
    assert_path_value_eq(yml, path, &expected_value);
}

/// Asserts that the YAML value at the specified path is equal to the expected
/// number value.
pub fn assert_path_value_eq_int(yml: &Value, path: &[&str], expected: i64) {
    let expected_value = Value::Number(serde_yaml::Number::from(expected));
    assert_path_value_eq(yml, path, &expected_value);
}

pub fn assert_path_value_eq_float(yml: &Value, path: &[&str], expected: f64) {
    let expected_value = Value::Number(serde_yaml::Number::from(expected));
    assert_path_value_eq(yml, path, &expected_value);
}

/// Asserts that the YAML mapping at the specified path contains the expected
/// number of keys.
pub fn assert_path_key_count(yml: &Value, path: &[&str], expected_count: usize) {
    let actual = get_value_at_path(yml, path).expect("Path not found in YAML structure");
    assert!(
        matches!(actual, Value::Mapping(map) if map.len() == expected_count),
        "Assertion failed: Path {:?} does not contain the expected number of keys. Expected: {}, \
         Actual: {}",
        path,
        expected_count,
        match actual {
            Value::Mapping(map) => map.len(),
            _ => 0,
        }
    );
}

/// Assert that a YAML value contains a specific key path and that it matches
/// the expected value.
pub fn assert_path_value_eq(yml: &Value, path: &[&str], expected: &Value) {
    let actual = get_value_at_path(yml, path);
    assert!(
        actual == Some(expected),
        "Assertion failed: Path {path:?} does not match expected value. Expected: {expected:?}, \
         Actual: {actual:?}"
    );
}

/// Assert that a YAML value contains a specific key path and that it matches
/// the expected value, but excludes
pub fn assert_path_value_eq_excluded(
    yml: &Value,
    path: &[&str],
    excluded: &[&str],
    expected: &Value,
) {
    let actual = get_value_at_path(yml, path);
    assert!(
        actual.is_some(),
        "Path {path:?} not found in YAML structure"
    );
    let mut actual = actual.unwrap().clone();
    let actual = remove_value_at_path(&mut actual, excluded);
    assert_ne!(
        actual,
        Some(expected.clone()),
        "Assertion failed: Path {path:?} does not match expected value. Expected: {expected:?}, \
         Actual: {actual:?}"
    );
}

// pub fn assert_path_value_eq_mapping(yml: &Value, path: &[&str], expected:
// &serde_yaml::Mapping) {     let actual = get_value_at_path(yml,
// path).unwrap();     assert!(
//         matches!(actual, Value::Mapping(map) if map == expected),
//         "Assertion failed: Path {path:?} does not match expected mapping.
// Expected: {expected:?}, Actual: {actual:?}"     );
// }

/// Assert that a YAML value contains a specific path, and that the value is an
/// object.
pub fn assert_path_is_object(yml: &Value, path: &[&str]) {
    let actual = get_value_at_path(yml, path).unwrap();
    assert!(
        matches!(actual, Value::Mapping(_)),
        "Assertion failed: Path {path:?} is not an object. Actual value: {actual:?}"
    );
}

/// Helper function to concatenate keys of a nested mapping to form a string.
#[must_use]
pub fn get_keys_concatenated_as_string(yml: &Value, path: &[&str]) -> Option<String> {
    let value_at_path = get_value_at_path(yml, path)?;
    if let Value::Mapping(map) = value_at_path {
        let mut concatenated_string = String::new();
        for key in map.keys() {
            if let Value::String(key_str) = key {
                concatenated_string.push_str(key_str);
            }
        }
        Some(concatenated_string)
    } else {
        None
    }
}

/// Assert that the YAML value at the given path is empty (either an empty
/// object or sequence).
pub fn assert_path_is_empty(yml: &Value, path: &[&str]) {
    let actual = get_value_at_path(yml, path);

    assert!(
        match actual {
            Some(Value::Mapping(map)) => map.is_empty(),
            Some(Value::Sequence(seq)) => seq.is_empty(),
            Some(Value::Null) | None => true,
            _ => {
                false
            }
        },
        "Assertion failed: Path {path:?} is not empty. Actual value: {actual:?}"
    );
}

pub fn assert_path_value_eq_mapping(yml: &Value, path: &[&str], expected: &serde_yaml::Mapping) {
    let actual = get_value_at_path(yml, path).expect("Path not found in YAML structure");
    assert!(
        matches!(actual, Value::Mapping(map) if map == expected),
        "Assertion failed: Path {path:?} does not match expected mapping. Expected: \
         {expected:#?}, Actual: {actual:#?}"
    );
}

pub fn assert_path_value_not_exists(yml: &Value, path: &[&str]) {
    let actual = get_value_at_path(yml, path);
    assert!(
        actual.is_none(),
        "Assertion failed: Path {path:?} exists. Actual value: {actual:?}"
    );
}

/// Internal helper function to remove a specific path from a YAML structure.
pub fn remove_value_at_path(yml: &mut Value, path: &[&str]) -> Option<Value> {
    // If there is no path, there's nothing to remove.
    if path.is_empty() {
        return None;
    }

    let mut current = yml;
    for &key in &path[..path.len() - 1] {
        match current {
            Value::Mapping(map) => {
                current = map.get_mut(&Value::String(key.to_string()))?;
            }
            Value::Sequence(seq) => {
                let idx = key.parse::<usize>().ok()?;
                current = seq.get_mut(idx)?;
            }
            // If we reach a non-Map or non-Sequence type, we can't proceed.
            _ => return None,
        }
    }

    let last_key = path[path.len() - 1];
    match current {
        Value::Mapping(map) => map.remove(&Value::String(last_key.to_string())),
        Value::Sequence(seq) => {
            let idx = last_key.parse::<usize>().ok()?;
            if idx < seq.len() {
                Some(seq.remove(idx))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Internal helper function to traverse a YAML structure and get the value at a
/// specific path.
#[must_use]
pub fn get_value_at_path<'a>(yml: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = yml;
    for &key in path {
        match current {
            Value::Mapping(map) => {
                current = map.get(Value::String(key.to_string()))?;
            }
            Value::Sequence(seq) => match key.parse::<usize>() {
                Ok(index) => current = seq.get(index)?,
                Err(_) => return None,
            },
            _ => return None,
        }
    }
    Some(current)
}
