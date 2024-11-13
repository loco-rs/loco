#![allow(clippy::missing_panics_doc)]
use regex::Regex;

pub fn assert_line_regex(content: &str, expected: &str) {
    let re = Regex::new(expected).unwrap();

    // Use assert! to check the regex match and panic if it fails
    assert!(
        re.is_match(content),
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
