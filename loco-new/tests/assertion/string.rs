#![allow(clippy::missing_panics_doc)]
use std::path::PathBuf;

use regex::Regex;

#[must_use]
pub fn load(path: PathBuf) -> String {
    std::fs::read_to_string(path).expect("could not read file")
}

pub fn assert_line_regex(content: &str, expected: &str) {
    let re = Regex::new(expected).unwrap();

    // Use assert! to check the regex match and panic if it fails
    assert!(
        // sanitize windows crlf
        re.is_match(&content.replace('\r', "")),
        "Assertion failed: The content did not match the expected string. Expected: '{expected}', \
         content:\n{content}"
    );
}

pub fn assert_str_not_exists(content: &str, expected: &str) {
    // Use assert! to check the regex match and panic if it fails
    assert!(
        !content.contains(expected),
        "Assertion failed: The content matched the unexpected string. Expected string to not \
         exist: '{expected}', content in:\n{content}",
    );
}

pub fn assert_contains(content: &str, expected: &str) {
    let content_sanitized = content.replace('\r', "");
    let expected_sanitized = expected.replace('\r', "");

    assert!(
        content_sanitized.contains(&expected_sanitized),
        "Assertion failed: The content did not contain the expected string. Expected: \
         '{expected_sanitized}', content:\n{content_sanitized}"
    );
}

pub fn assert_not_contains(content: &str, unexpected: &str) {
    let content_sanitized = content.replace('\r', "");
    let unexpected_sanitized = unexpected.replace('\r', "");

    assert!(
        !content_sanitized.contains(&unexpected_sanitized),
        "Assertion failed: The content unexpectedly contained the string. Unexpected: \
         '{unexpected_sanitized}', content:\n{content_sanitized}"
    );
}
