use std::sync::OnceLock;

static CLEANUP_USER_MODEL: OnceLock<Vec<(&'static str, &'static str)>> = OnceLock::new();
static CLEANUP_DATE: OnceLock<Vec<(&'static str, &'static str)>> = OnceLock::new();
static CLEANUP_MODEL: OnceLock<Vec<(&'static str, &'static str)>> = OnceLock::new();
static CLEANUP_MAIL: OnceLock<Vec<(&'static str, &'static str)>> = OnceLock::new();

pub fn get_cleanup_user_model() -> &'static Vec<(&'static str, &'static str)> {
    CLEANUP_USER_MODEL.get_or_init(|| {
        vec![
            (
                r"([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})",
                "PID",
            ),
            (r#"password: "[^"]*""#, "password: \"PASSWORD\""),
            (r"([A-Za-z0-9-_]*\.[A-Za-z0-9-_]*\.[A-Za-z0-9-_]*)", "TOKEN"),
        ]
    })
}

pub fn get_cleanup_date() -> &'static Vec<(&'static str, &'static str)> {
    CLEANUP_DATE.get_or_init(|| {
        vec![
            (
                r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?[+-]\d{2}:\d{2}",
                "DATE",
            ), // with tz
            (r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+", "DATE"),
            (r"(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})", "DATE"),
        ]
    })
}

pub fn get_cleanup_model() -> &'static Vec<(&'static str, &'static str)> {
    CLEANUP_MODEL.get_or_init(|| vec![(r"id: \d+,", "id: ID")])
}

pub fn get_cleanup_mail() -> &'static Vec<(&'static str, &'static str)> {
    CLEANUP_MAIL.get_or_init(|| {
        vec![
            (r"[0-9A-Za-z]+{40}", "IDENTIFIER"),
            (
                r"\w+, \d{1,2} \w+ \d{4} \d{2}:\d{2}:\d{2} [+-]\d{4}",
                "DATE",
            ),
            (
                r"([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})",
                "RANDOM_ID",
            ),
            (
                r"([0-9a-fA-F]{8}-[0-9a-fA-F]{4})-[0-9a-fA-F]{4}-.*[0-9a-fA-F]{2}",
                "RANDOM_ID",
            ),
        ]
    })
}

/// Combines cleanup filters from various categories (user model, date, and
/// model) into one list. This is used for data cleaning and pattern
/// replacement.
///
/// # Example
///
/// The provided example demonstrates how to efficiently clean up a user model.
/// This process is particularly valuable when you need to capture a snapshot of
/// user model data that includes dynamic elements such as incrementing IDs,
/// automatically generated PIDs, creation/update timestamps, and similar
/// attributes.
///
/// ```rust,ignore
/// use myapp::app::App;
/// use loco_rs::testing::prelude::*;
///
/// #[tokio::test]
/// async fn test_create_user() {
///     let boot = boot_test::<App>().await;
///
///     // Create a user and save into the database.
///
///     // capture the snapshot and cleanup the data.
///     with_settings!({
///         filters => cleanup_user_model()
///     }, {
///         assert_debug_snapshot!(saved_user);
///     });
/// }
/// ```
#[must_use]
pub fn cleanup_user_model() -> Vec<(&'static str, &'static str)> {
    let mut combined_filters = get_cleanup_user_model().clone();
    combined_filters.extend(get_cleanup_date().iter().copied());
    combined_filters.extend(get_cleanup_model().iter().copied());
    combined_filters
}

/// Combines cleanup filters from emails  that can be dynamic
#[must_use]
pub fn cleanup_email() -> Vec<(&'static str, &'static str)> {
    let mut combined_filters = get_cleanup_mail().clone();
    combined_filters.extend(get_cleanup_date().iter().copied());
    combined_filters
}

#[cfg(test)]
mod tests {
    use regex::Regex;

    use super::*;

    /// A realistic pretty-printed record, mimicking `assert_debug_snapshot!`
    /// output for a user model: a password hash (which itself contains a
    /// comma) followed by another field on the same line.
    const SAMPLE_RECORD: &str = r#"password: "$argon2id$v=19$m=19456,t=2,p=1$ETQBx4rTgNAZhSaeYZKOZg$eYTdH26CRT6nUJtacLDEboP0li6xUwUF/q5nSlQ8uuc", api_key: "lo-95ec80d7-cb60-4b70-9b4b-9ef74cb88758","#;

    /// Timestamps as `assert_debug_snapshot!` renders a
    /// `DateTimeWithTimeZone` (chrono `DateTime<FixedOffset>`): RFC 3339 with
    /// an explicit numeric UTC offset whose sign follows the local timezone.
    const DATES_WITH_OFFSET: &[&str] = &[
        "2026-08-07T03:33:44.123456789-03:00", // west of UTC
        "2026-08-07T03:33:44.123456789+02:00", // east of UTC
        "2026-08-07T03:33:44.123456789+00:00", // UTC itself
        "2026-08-07T03:33:44-03:00",           // no fractional seconds
    ];

    fn redact_date(input: &str) -> String {
        let mut redacted = input.to_string();
        for (pattern, replacement) in get_cleanup_date() {
            let re = Regex::new(pattern).unwrap();
            redacted = re.replace_all(&redacted, *replacement).to_string();
        }
        redacted
    }

    fn password_filter() -> (&'static str, &'static str) {
        get_cleanup_user_model()
            .iter()
            .copied()
            .find(|(pattern, _)| pattern.starts_with("password"))
            .expect("a password redaction rule must be present")
    }

    #[test]
    fn old_password_regex_was_inert() {
        // Regression guard for #14: `password: (.*{60}),` is a degenerate
        // quantifier (`(.*){60}` collapses to `.*`), so it has no length
        // constraint at all. It greedily matches through to the *last* comma
        // on the line, silently swallowing whatever field comes right after
        // the password (here, `api_key`) instead of stopping at the hash.
        let old_pattern = Regex::new(r"password: (.*{60}),").unwrap();
        let redacted = old_pattern.replace(SAMPLE_RECORD, "password: \"PASSWORD\",");

        // the neighboring field got eaten by the over-broad match
        assert!(!redacted.contains("api_key"));
    }

    #[test]
    fn password_field_is_redacted_precisely() {
        let (pattern, replacement) = password_filter();
        let re = Regex::new(pattern).unwrap();
        let redacted = re.replace(SAMPLE_RECORD, replacement);

        // Exact match: only the quoted value is replaced, the trailing comma
        // and the neighboring field are left byte-for-byte intact. Asserting
        // the whole string guards against re-introducing punctuation artifacts
        // (e.g. a doubled `,,` if the replacement re-adds the comma the regex
        // no longer consumes).
        assert_eq!(
            redacted,
            r#"password: "PASSWORD", api_key: "lo-95ec80d7-cb60-4b70-9b4b-9ef74cb88758","#
        );
        assert!(
            !redacted.contains(",,"),
            "redaction must not double the comma"
        );
    }

    #[test]
    fn cleanup_user_model_redacts_full_record() {
        let filters = cleanup_user_model();
        let mut redacted = SAMPLE_RECORD.to_string();
        for (pattern, replacement) in filters {
            let re = Regex::new(pattern).unwrap();
            redacted = re.replace_all(&redacted, replacement).to_string();
        }

        // The uuid-looking part of the api_key also gets redacted by the PID
        // filter (expected/pre-existing behavior); what matters here is that
        // the `api_key` field is not swallowed by the password filter and no
        // punctuation artifact is introduced.
        assert_eq!(redacted, r#"password: "PASSWORD", api_key: "lo-PID","#);
        assert!(
            !redacted.contains(",,"),
            "redaction must not double the comma"
        );
    }

    #[test]
    fn old_date_regex_only_matched_positive_offsets() {
        // Regression guard: the "with tz" rule used to end in `\+\d{2}:\d{2}`,
        // which only matches a *positive* UTC offset. It is the sole rule that
        // consumes the offset, so on a machine west of UTC the timestamp fell
        // through to the offset-less rules — those stop at the seconds and
        // leave the offset stranded, redacting to `DATE-03:00` instead of
        // `DATE`. That desync fails every snapshot carrying a timestamp.
        let old_pattern =
            Regex::new(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?\+\d{2}:\d{2}").unwrap();

        assert!(!old_pattern.is_match("2026-08-07T03:33:44.123456789-03:00"));
        assert!(old_pattern.is_match("2026-08-07T03:33:44.123456789+02:00"));
    }

    #[test]
    fn cleanup_date_redacts_either_offset_sign() {
        for sample in DATES_WITH_OFFSET {
            assert_eq!(
                redact_date(sample),
                "DATE",
                "`{sample}` must redact whole, leaving no offset behind"
            );
        }
    }
}
