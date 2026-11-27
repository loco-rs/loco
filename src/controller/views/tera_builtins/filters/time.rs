//! Time filters: `time_ago_in_words`, `distance_of_time_in_words`.
//!
//! The wording lives in [`crate::controller::views::helpers`]; these are thin
//! Tera wrappers over it.

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use tera::{Error, Kwargs, State, TeraResult, Value};

use crate::controller::views::helpers;

/// Reads a time from a template value.
///
/// Accepts what a model serializes to: an RFC 3339 string (`DateTime<Utc>`,
/// `DateTimeWithTimeZone`), a naive date-time string (`DateTime`, read as
/// UTC), a date (`Date`, midnight UTC), or an integer of Unix seconds.
fn parse_time(value: &Value, filter: &str, arg: &str) -> TeraResult<DateTime<Utc>> {
    let parsed = value.as_i64().map_or_else(
        || value.as_str().and_then(parse_time_str),
        |secs| DateTime::from_timestamp(secs, 0),
    );
    parsed.ok_or_else(|| {
        Error::message(format!(
            "{filter}: `{arg}` must be an RFC 3339 date-time, a date or Unix seconds, got `{value}`"
        ))
    })
}

fn parse_time_str(text: &str) -> Option<DateTime<Utc>> {
    let text = text.trim();
    DateTime::parse_from_rfc3339(text)
        .map(|t| t.with_timezone(&Utc))
        .ok()
        .or_else(|| {
            ["%Y-%m-%dT%H:%M:%S%.f", "%Y-%m-%d %H:%M:%S%.f"]
                .iter()
                .find_map(|fmt| NaiveDateTime::parse_from_str(text, fmt).ok())
                .map(|t| t.and_utc())
        })
        .or_else(|| {
            NaiveDate::parse_from_str(text, "%Y-%m-%d")
                .ok()
                .and_then(|d| d.and_hms_opt(0, 0, 0))
                .map(|t| t.and_utc())
        })
}

fn include_seconds(kwargs: &Kwargs) -> TeraResult<bool> {
    Ok(kwargs.get::<bool>("include_seconds")?.unwrap_or(false))
}

/// `{{ post.created_at | time_ago_in_words }}` renders `"3 days"`.
///
/// Keyword arguments: `include_seconds=true` for finer wording under a
/// minute, and `now=` to measure from a fixed time instead of the clock
/// (useful in tests and in emails rendered ahead of time).
///
/// # Errors
///
/// Returns an error if the value or `now` is not a time.
// `Kwargs` by value is Tera's `Filter` signature, not a choice.
#[allow(clippy::needless_pass_by_value)]
pub fn time_ago_in_words(value: &Value, kwargs: Kwargs, _: &State<'_>) -> TeraResult<Value> {
    let from = parse_time(value, "time_ago_in_words", "value")?;
    let now = match kwargs.get::<&Value>("now")? {
        Some(now) => parse_time(now, "time_ago_in_words", "now")?,
        None => Utc::now(),
    };
    Ok(Value::from(helpers::distance_of_time_in_words(
        from,
        now,
        include_seconds(&kwargs)?,
    )))
}

/// `{{ started_at | distance_of_time_in_words(to=finished_at) }}` renders
/// `"about 2 hours"`.
///
/// Takes `to=` (required) and `include_seconds=true`.
///
/// # Errors
///
/// Returns an error if the value or `to` is not a time, or `to` is missing.
#[allow(clippy::needless_pass_by_value)]
pub fn distance_of_time_in_words(
    value: &Value,
    kwargs: Kwargs,
    _: &State<'_>,
) -> TeraResult<Value> {
    let from = parse_time(value, "distance_of_time_in_words", "value")?;
    let to = parse_time(
        kwargs.must_get::<&Value>("to")?,
        "distance_of_time_in_words",
        "to",
    )?;
    Ok(Value::from(helpers::distance_of_time_in_words(
        from,
        to,
        include_seconds(&kwargs)?,
    )))
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(Value::from("2024-06-01T12:00:00Z"))]
    #[case(Value::from("2024-06-01T14:00:00+02:00"))]
    #[case(Value::from("2024-06-01T12:00:00"))]
    #[case(Value::from("2024-06-01 12:00:00.123"))]
    #[case(Value::from(1_717_243_200))]
    fn parses_model_times(#[case] value: Value) {
        let expected = Utc.with_ymd_and_hms(2024, 6, 1, 12, 0, 0).unwrap();
        let parsed = parse_time(&value, "f", "value").unwrap();
        assert_eq!(parsed.timestamp(), expected.timestamp());
    }

    #[test]
    fn parses_dates_as_midnight() {
        let parsed = parse_time(&Value::from("2024-06-01"), "f", "value").unwrap();
        assert_eq!(parsed, Utc.with_ymd_and_hms(2024, 6, 1, 0, 0, 0).unwrap());
    }

    #[test]
    fn rejects_non_times() {
        let err = parse_time(&Value::from("yesterday"), "time_ago_in_words", "value")
            .unwrap_err()
            .to_string();
        assert!(err.contains("time_ago_in_words"), "{err}");
        assert!(parse_time(&Value::from(true), "f", "value").is_err());
    }
}
