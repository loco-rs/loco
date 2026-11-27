//! View helpers: the Rails `ActionView` / `ActiveSupport` text helpers.
//!
//! Each helper is a plain function, usable from Rust (views, mailers, JSON
//! responses), and is also registered as a Tera filter of the same name, so
//! templates get the same wording:
//!
//! | Rust | Tera |
//! |---|---|
//! | [`time_ago_in_words`] | `{{ post.created_at \| time_ago_in_words }}` |
//! | [`distance_of_time_in_words`] | `{{ started \| distance_of_time_in_words(to=finished) }}` |
//! | [`pluralize`] | `{{ count \| pluralize(word="comment") }}` |
//! | [`to_sentence`] | `{{ names \| to_sentence }}` |
//! | [`ordinalize`] | `{{ place \| ordinalize }}` |
//! | [`number_with_delimiter`] | `{{ total \| number_with_delimiter }}` |
//! | [`number_to_human`] | `{{ views \| number_to_human }}` |
//!
//! ```rust
//! use loco_rs::controller::views::helpers;
//!
//! assert_eq!(helpers::pluralize(2, "person", None), "2 people");
//! assert_eq!(helpers::to_sentence(&["a", "b", "c"]), "a, b, and c");
//! assert_eq!(helpers::ordinalize(22), "22nd");
//! assert_eq!(helpers::number_with_delimiter(1234567), "1,234,567");
//! assert_eq!(helpers::number_to_human(1234567.0), "1.23 Million");
//! ```

use std::fmt::Display;

use chrono::{DateTime, Datelike, Utc};

const MINUTES_IN_DAY: i64 = 1440;
const MINUTES_IN_MONTH: i64 = 43_200;
const MINUTES_IN_YEAR: i64 = 525_600;
const MINUTES_IN_QUARTER_YEAR: i64 = 131_400;
const MINUTES_IN_THREE_QUARTERS_YEAR: i64 = 394_200;

/// `"1 thing"` or `"n things"`, written with the singular when `n == 1`.
fn counted(n: i64, one: &str, other: &str) -> String {
    if n == 1 {
        one.to_string()
    } else {
        other.replace("%{count}", &n.to_string())
    }
}

/// Approximate distance between two times, in words.
///
/// The wording and the boundaries are those of Rails'
/// `distance_of_time_in_words`: `"less than a minute"`, `"5 minutes"`,
/// `"about 1 hour"`, `"3 days"`, `"about 1 month"`, `"over 1 year"`,
/// `"almost 2 years"`. The order of `from` and `to` does not matter. With
/// `include_seconds`, distances under a minute are split further
/// (`"less than 5 seconds"`, `"half a minute"`).
///
/// ```rust
/// use chrono::{Duration, Utc};
/// use loco_rs::controller::views::helpers::distance_of_time_in_words;
///
/// let from = Utc::now();
/// assert_eq!(distance_of_time_in_words(from, from + Duration::minutes(50), false), "about 1 hour");
/// assert_eq!(distance_of_time_in_words(from, from + Duration::seconds(25), true), "half a minute");
/// ```
#[must_use]
pub fn distance_of_time_in_words(
    from: impl Into<DateTime<Utc>>,
    to: impl Into<DateTime<Utc>>,
    include_seconds: bool,
) -> String {
    let (from, to) = {
        let (a, b) = (from.into(), to.into());
        if a <= b {
            (a, b)
        } else {
            (b, a)
        }
    };
    let millis = (to - from).num_milliseconds();
    // Whole seconds and minutes, rounded half up, as Rails rounds them.
    let seconds = (millis + 500) / 1000;
    let minutes = (millis + 30_000) / 60_000;

    match minutes {
        0..=1 => {
            if !include_seconds {
                return if minutes == 0 {
                    "less than a minute".to_string()
                } else {
                    "1 minute".to_string()
                };
            }
            match seconds {
                0..=4 => "less than 5 seconds".to_string(),
                5..=9 => "less than 10 seconds".to_string(),
                10..=19 => "less than 20 seconds".to_string(),
                20..=39 => "half a minute".to_string(),
                40..=59 => "less than a minute".to_string(),
                _ => "1 minute".to_string(),
            }
        }
        2..=44 => format!("{minutes} minutes"),
        45..=89 => "about 1 hour".to_string(),
        // 90 minutes up to, but not including, 24 hours.
        90..=1439 => format!("about {} hours", (minutes + 30) / 60),
        // 24 hours up to 42 hours.
        1440..=2519 => "1 day".to_string(),
        // 42 hours up to 30 days.
        2520..=43_199 => format!("{} days", (minutes + MINUTES_IN_DAY / 2) / MINUTES_IN_DAY),
        // 30 days up to 60 days, rounded half up as Rails rounds it.
        43_200..=86_399 => counted(
            (minutes + MINUTES_IN_MONTH / 2) / MINUTES_IN_MONTH,
            "about 1 month",
            "about %{count} months",
        ),
        // 60 days up to 365 days.
        86_400..=525_599 => counted(
            (minutes + MINUTES_IN_MONTH / 2) / MINUTES_IN_MONTH,
            "1 month",
            "%{count} months",
        ),
        _ => years_in_words(from, to, minutes),
    }
}

/// The over-a-year branch: discount a day per leap year crossed, then say
/// "about", "over" or "almost" depending on where in the year it lands.
fn years_in_words(from: DateTime<Utc>, to: DateTime<Utc>, minutes: i64) -> String {
    let mut from_year = from.year();
    if from.month() >= 3 {
        from_year += 1;
    }
    let mut to_year = to.year();
    if to.month() < 3 {
        to_year -= 1;
    }
    let leap_years = if from_year > to_year {
        0
    } else {
        (from_year..=to_year)
            .filter(|y| chrono::NaiveDate::from_ymd_opt(*y, 2, 29).is_some())
            .count()
    };
    let minutes = minutes - i64::try_from(leap_years).unwrap_or(0) * MINUTES_IN_DAY;
    let years = minutes / MINUTES_IN_YEAR;
    let remainder = minutes % MINUTES_IN_YEAR;

    if remainder < MINUTES_IN_QUARTER_YEAR {
        counted(years, "about 1 year", "about %{count} years")
    } else if remainder < MINUTES_IN_THREE_QUARTERS_YEAR {
        counted(years, "over 1 year", "over %{count} years")
    } else {
        counted(years + 1, "almost 1 year", "almost %{count} years")
    }
}

/// Distance between `from` and now, in words: `"3 days"`, `"about 1 hour"`.
///
/// Like Rails, this does not add `"ago"`; write it in the template
/// (`{{ t | time_ago_in_words }} ago`). See [`distance_of_time_in_words`]
/// for the wording.
#[must_use]
pub fn time_ago_in_words(from: impl Into<DateTime<Utc>>, include_seconds: bool) -> String {
    distance_of_time_in_words(from, Utc::now(), include_seconds)
}

/// Irregular plurals Rails knows that the inflector used for table names does
/// not.
const IRREGULAR_PLURALS: &[(&str, &str)] = &[("person", "people"), ("Person", "People")];

/// The plural of an English noun, using the same inflector Loco uses for
/// table names, plus Rails' irregulars (`person` → `people`).
#[must_use]
pub fn pluralize_word(word: &str) -> String {
    IRREGULAR_PLURALS
        .iter()
        .find(|(singular, _)| *singular == word)
        .map_or_else(
            || cruet::to_plural(word),
            |(_, plural)| (*plural).to_string(),
        )
}

/// `count` followed by `singular` or its plural: `"1 person"`, `"2 people"`.
///
/// Rails' `pluralize(count, singular, plural: nil)`: the singular is used only
/// when `count` is exactly 1 (`"1"`, `1`, `1.0`); otherwise `plural` if given,
/// else the inflected plural of `singular`.
///
/// ```rust
/// use loco_rs::controller::views::helpers::pluralize;
///
/// assert_eq!(pluralize(1, "comment", None), "1 comment");
/// assert_eq!(pluralize(0, "comment", None), "0 comments");
/// assert_eq!(pluralize(3, "octopus", Some("octopodes")), "3 octopodes");
/// ```
#[must_use]
pub fn pluralize(count: impl Display, singular: &str, plural: Option<&str>) -> String {
    let count = count.to_string();
    // Rails: `count == 1 || count =~ /^1(\.0+)?$/`.
    let is_one = count == "1"
        || count
            .strip_prefix("1.")
            .is_some_and(|zeros| !zeros.is_empty() && zeros.bytes().all(|b| b == b'0'));
    let word = if is_one {
        singular.to_string()
    } else {
        plural.map_or_else(|| pluralize_word(singular), ToString::to_string)
    };
    format!("{count} {word}")
}

/// The connectors [`to_sentence_with`] joins words with. The default is
/// Rails': `", "`, `" and "`, `", and "`.
#[derive(Debug, Clone)]
pub struct SentenceConnectors<'a> {
    /// Between all but the last two words. Default `", "`.
    pub words_connector: &'a str,
    /// Between the words of a two-word list. Default `" and "`.
    pub two_words_connector: &'a str,
    /// Before the last word of a list of three or more. Default `", and "`.
    pub last_word_connector: &'a str,
}

impl Default for SentenceConnectors<'_> {
    fn default() -> Self {
        Self {
            words_connector: ", ",
            two_words_connector: " and ",
            last_word_connector: ", and ",
        }
    }
}

/// Joins words into a sentence: `"a"`, `"a and b"`, `"a, b, and c"`.
///
/// Rails' `Array#to_sentence`, with its default connectors. Use
/// [`to_sentence_with`] to change them (for example, to drop the serial
/// comma).
#[must_use]
pub fn to_sentence<S: AsRef<str>>(words: &[S]) -> String {
    to_sentence_with(words, &SentenceConnectors::default())
}

/// [`to_sentence`] with custom connectors.
///
/// ```rust
/// use loco_rs::controller::views::helpers::{to_sentence_with, SentenceConnectors};
///
/// let connectors = SentenceConnectors { last_word_connector: " and ", ..Default::default() };
/// assert_eq!(to_sentence_with(&["a", "b", "c"], &connectors), "a, b and c");
/// ```
#[must_use]
pub fn to_sentence_with<S: AsRef<str>>(words: &[S], connectors: &SentenceConnectors<'_>) -> String {
    match words {
        [] => String::new(),
        [one] => one.as_ref().to_string(),
        [a, b] => format!(
            "{}{}{}",
            a.as_ref(),
            connectors.two_words_connector,
            b.as_ref()
        ),
        [rest @ .., last] => {
            let head: Vec<&str> = rest.iter().map(AsRef::as_ref).collect();
            format!(
                "{}{}{}",
                head.join(connectors.words_connector),
                connectors.last_word_connector,
                last.as_ref()
            )
        }
    }
}

/// The suffix [`ordinalize`] adds: `"st"`, `"nd"`, `"rd"` or `"th"`.
#[must_use]
pub const fn ordinal(number: i64) -> &'static str {
    let n = number.unsigned_abs();
    if matches!(n % 100, 11..=13) {
        return "th";
    }
    match n % 10 {
        1 => "st",
        2 => "nd",
        3 => "rd",
        _ => "th",
    }
}

/// A number as an ordinal: `"1st"`, `"2nd"`, `"11th"`, `"23rd"`, `"-1st"`.
#[must_use]
pub fn ordinalize(number: i64) -> String {
    format!("{number}{}", ordinal(number))
}

/// A number with `,` between thousands: `1234567.89` → `"1,234,567.89"`.
///
/// Anything that displays as a number works (integers, floats, numeric
/// strings); other text is returned unchanged.
#[must_use]
pub fn number_with_delimiter(number: impl Display) -> String {
    number_with_delimiter_using(number, ",", ".")
}

/// [`number_with_delimiter`] with a custom delimiter and decimal separator,
/// as with Rails' `delimiter:` and `separator:` options.
///
/// ```rust
/// use loco_rs::controller::views::helpers::number_with_delimiter_using;
///
/// assert_eq!(number_with_delimiter_using(1234567.5, ".", ","), "1.234.567,5");
/// ```
#[must_use]
pub fn number_with_delimiter_using(
    number: impl Display,
    delimiter: &str,
    separator: &str,
) -> String {
    let text = number.to_string();
    let (sign, unsigned) = text
        .strip_prefix('-')
        .map_or(("", text.as_str()), |rest| ("-", rest));
    let (integer, fraction) = unsigned
        .split_once('.')
        .map_or((unsigned, None), |(i, f)| (i, Some(f)));
    if integer.is_empty()
        || !integer.bytes().all(|b| b.is_ascii_digit())
        || !fraction.is_none_or(|f| f.bytes().all(|b| b.is_ascii_digit()))
    {
        return text;
    }

    let mut out = String::with_capacity(text.len() + integer.len() / 3 * delimiter.len());
    out.push_str(sign);
    for (i, c) in integer.chars().enumerate() {
        if i > 0 && (integer.len() - i) % 3 == 0 {
            out.push_str(delimiter);
        }
        out.push(c);
    }
    if let Some(fraction) = fraction {
        out.push_str(separator);
        out.push_str(fraction);
    }
    out
}

const HUMAN_UNITS: [(i32, &str); 6] = [
    (0, ""),
    (3, "Thousand"),
    (6, "Million"),
    (9, "Billion"),
    (12, "Trillion"),
    (15, "Quadrillion"),
];

/// The most digits [`number_to_human_with`] prints. 15 significant digits is
/// all an `f64` carries through decimal and back, so asking for more would
/// only print rounding noise (or overflow); larger precisions are treated as
/// this.
const MAX_PRECISION: u32 = 15;

/// `number` rounded half away from zero to `decimals` places (negative
/// `decimals` round to tens, hundreds, ...). Where the scaling would overflow,
/// the number already has no digits to drop and is returned as is.
fn round_to(number: f64, decimals: i32) -> f64 {
    // Scale by a power of ten at least 1, which is exact up to 10^22, so the
    // result is the nearest `f64` to the rounded decimal.
    let factor = 10f64.powi(decimals.abs());
    let rounded = if decimals >= 0 {
        (number * factor).round() / factor
    } else {
        (number / factor).round() * factor
    };
    if factor.is_finite() && rounded.is_finite() {
        rounded
    } else {
        number
    }
}

/// Rounds to `precision` significant digits (or decimal places when
/// `significant` is false), returning the value and the decimals to print.
///
/// As in Rails, `precision: 0` with `significant: true` rounds to a whole
/// number, since zero significant digits has no meaning.
#[allow(clippy::cast_possible_truncation)]
fn round_number(number: f64, precision: u32, significant: bool) -> (f64, usize) {
    // Fits in an i32: precision is at most MAX_PRECISION.
    let precision = precision.min(MAX_PRECISION).cast_signed();
    if !significant || precision == 0 {
        // Keep the whole and decimal digits together within MAX_PRECISION.
        let whole_digits = if number == 0.0 {
            1
        } else {
            (number.abs().log10().floor() as i32 + 1).max(1)
        };
        let decimals = precision.min((MAX_PRECISION.cast_signed() - whole_digits).max(0));
        return (round_to(number, decimals), decimals.unsigned_abs() as usize);
    }
    if number == 0.0 {
        return (0.0, (precision - 1).unsigned_abs() as usize);
    }
    let digits = number.abs().log10().floor() as i32 + 1;
    let rounded = round_to(number, precision - digits);
    if rounded == 0.0 {
        return (0.0, 0);
    }
    // Rounding can add a digit (999.6 -> 1000): print one decimal fewer.
    let rounded_digits = rounded.abs().log10().floor() as i32 + 1;
    (
        rounded,
        usize::try_from(precision - rounded_digits).unwrap_or(0),
    )
}

/// Formats with `decimals` places, then strips trailing zeros (Rails'
/// `strip_insignificant_zeros: true`).
fn format_stripped(number: f64, decimals: usize) -> String {
    // With no decimals, `Display` gives the shortest digits that read back as
    // the same `f64` (`3.4e23` is `340000000000000000000000`), where `{:.0}`
    // would print the binary value's every digit.
    let text = if decimals == 0 {
        number.to_string()
    } else {
        format!("{number:.decimals$}")
    };
    let text = if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        text
    };
    if text == "-0" {
        "0".to_string()
    } else {
        text
    }
}

/// A number in words for people: `1234` → `"1.23 Thousand"`,
/// `1_500_000` → `"1.5 Million"`, `123` → `"123"`.
///
/// Rails' `number_to_human` with its defaults: three significant digits,
/// insignificant zeros stripped, units up to `"Quadrillion"`. See
/// [`number_to_human_with`] to change the precision.
#[must_use]
pub fn number_to_human(number: f64) -> String {
    number_to_human_with(number, 3, true)
}

/// [`number_to_human`] with Rails' `precision:` and `significant:` options.
/// With `significant: false`, `precision` counts decimal places instead of
/// significant digits.
///
/// `precision: 0` rounds to a whole number either way,
/// as in Rails, and precisions above 15 (all an `f64` carries through
/// decimal and back) are treated as 15.
///
/// ```rust
/// use loco_rs::controller::views::helpers::number_to_human_with;
///
/// assert_eq!(number_to_human_with(1234567.0, 2, true), "1.2 Million");
/// assert_eq!(number_to_human_with(1234567.0, 1, false), "1.2 Million");
/// ```
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn number_to_human_with(number: f64, precision: u32, significant: bool) -> String {
    if !number.is_finite() {
        return number.to_string();
    }
    // Round first, so 999_999 reads "1 Million", not "1000 Thousand".
    let (rounded, _) = round_number(number, precision, significant);
    let exponent = if rounded == 0.0 {
        0
    } else {
        rounded.abs().log10().floor() as i32
    };
    let (unit_exponent, unit) = HUMAN_UNITS
        .iter()
        .rev()
        .find(|(e, _)| *e <= exponent)
        .copied()
        .unwrap_or(HUMAN_UNITS[0]);
    let scaled = number / 10f64.powi(unit_exponent);
    let (value, decimals) = round_number(scaled, precision, significant);
    let text = format_stripped(value, decimals);
    if unit.is_empty() {
        text
    } else {
        format!("{text} {unit}")
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone};
    use rstest::rstest;

    use super::*;

    fn at(secs: i64) -> String {
        let from = Utc.with_ymd_and_hms(2024, 6, 1, 12, 0, 0).unwrap();
        distance_of_time_in_words(from, from + Duration::seconds(secs), false)
    }

    fn at_with_seconds(secs: i64) -> String {
        let from = Utc.with_ymd_and_hms(2024, 6, 1, 12, 0, 0).unwrap();
        distance_of_time_in_words(from, from + Duration::seconds(secs), true)
    }

    // Boundaries from Rails' date_helper_test.rb.
    #[rstest]
    #[case(0, "less than a minute")]
    #[case(29, "less than a minute")]
    #[case(30, "1 minute")]
    #[case(89, "1 minute")]
    #[case(90, "2 minutes")]
    #[case(44 * 60 + 29, "44 minutes")]
    #[case(44 * 60 + 30, "about 1 hour")]
    #[case(89 * 60 + 29, "about 1 hour")]
    #[case(89 * 60 + 30, "about 2 hours")]
    #[case(23 * 3600 + 59 * 60 + 29, "about 24 hours")]
    #[case(23 * 3600 + 59 * 60 + 30, "1 day")]
    #[case(41 * 3600 + 59 * 60 + 29, "1 day")]
    #[case(41 * 3600 + 59 * 60 + 30, "2 days")]
    #[case(2 * 86_400 + 12 * 3600, "3 days")]
    #[case(29 * 86_400 + 23 * 3600 + 59 * 60 + 29, "30 days")]
    #[case(29 * 86_400 + 23 * 3600 + 59 * 60 + 30, "about 1 month")]
    #[case(44 * 86_400 + 23 * 3600 + 59 * 60 + 29, "about 1 month")]
    #[case(44 * 86_400 + 23 * 3600 + 59 * 60 + 30, "about 2 months")]
    #[case(59 * 86_400 + 23 * 3600 + 59 * 60 + 29, "about 2 months")]
    #[case(59 * 86_400 + 23 * 3600 + 59 * 60 + 30, "2 months")]
    #[case(364 * 86_400, "12 months")]
    #[case(365 * 86_400 + 60, "about 1 year")]
    #[case(365 * 86_400 + 92 * 86_400, "over 1 year")]
    #[case(365 * 86_400 + 274 * 86_400, "almost 2 years")]
    #[case(2 * 365 * 86_400 + 2 * 86_400, "about 2 years")]
    #[case(10 * 365 * 86_400 + 3 * 86_400, "about 10 years")]
    fn distance_matches_rails(#[case] secs: i64, #[case] expected: &str) {
        assert_eq!(at(secs), expected);
        // Order does not matter.
        let from = Utc.with_ymd_and_hms(2024, 6, 1, 12, 0, 0).unwrap();
        assert_eq!(
            distance_of_time_in_words(from + Duration::seconds(secs), from, false),
            expected
        );
    }

    #[rstest]
    #[case(0, "less than 5 seconds")]
    #[case(4, "less than 5 seconds")]
    #[case(5, "less than 10 seconds")]
    #[case(9, "less than 10 seconds")]
    #[case(10, "less than 20 seconds")]
    #[case(19, "less than 20 seconds")]
    #[case(20, "half a minute")]
    #[case(39, "half a minute")]
    #[case(40, "less than a minute")]
    #[case(59, "less than a minute")]
    #[case(60, "1 minute")]
    #[case(89, "1 minute")]
    #[case(90, "2 minutes")]
    fn distance_with_seconds(#[case] secs: i64, #[case] expected: &str) {
        assert_eq!(at_with_seconds(secs), expected);
    }

    #[test]
    fn time_ago_uses_now() {
        assert_eq!(
            time_ago_in_words(Utc::now() - Duration::hours(3), false),
            "about 3 hours"
        );
        assert_eq!(
            time_ago_in_words(Utc::now().fixed_offset() - Duration::days(4), false),
            "4 days"
        );
    }

    #[rstest]
    #[case("1", "person", None, "1 person")]
    #[case("2", "person", None, "2 people")]
    #[case("0", "comment", None, "0 comments")]
    #[case("1.0", "comment", None, "1.0 comment")]
    #[case("1.5", "comment", None, "1.5 comments")]
    #[case("-1", "comment", None, "-1 comments")]
    #[case("10", "comment", None, "10 comments")]
    #[case("2", "child", None, "2 children")]
    #[case("2", "category", None, "2 categories")]
    #[case("2", "person", Some("users"), "2 users")]
    #[case("1", "person", Some("users"), "1 person")]
    fn pluralize_matches_rails(
        #[case] count: &str,
        #[case] singular: &str,
        #[case] plural: Option<&str>,
        #[case] expected: &str,
    ) {
        assert_eq!(pluralize(count, singular, plural), expected);
    }

    #[test]
    fn pluralize_takes_numbers() {
        assert_eq!(pluralize(1, "post", None), "1 post");
        assert_eq!(pluralize(3_u64, "post", None), "3 posts");
    }

    #[rstest]
    #[case(&[], "")]
    #[case(&["one"], "one")]
    #[case(&["one", "two"], "one and two")]
    #[case(&["one", "two", "three"], "one, two, and three")]
    #[case(&["a", "b", "c", "d"], "a, b, c, and d")]
    fn to_sentence_matches_rails(#[case] words: &[&str], #[case] expected: &str) {
        assert_eq!(to_sentence(words), expected);
    }

    #[test]
    fn to_sentence_custom_connectors() {
        let connectors = SentenceConnectors {
            words_connector: " ",
            two_words_connector: " & ",
            last_word_connector: " or ",
        };
        assert_eq!(to_sentence_with(&["a", "b"], &connectors), "a & b");
        assert_eq!(
            to_sentence_with(&["a", "b", "c", "d"], &connectors),
            "a b c or d"
        );
        assert_eq!(to_sentence(&[String::from("x")]), "x");
    }

    #[rstest]
    #[case(0, "0th")]
    #[case(1, "1st")]
    #[case(2, "2nd")]
    #[case(3, "3rd")]
    #[case(4, "4th")]
    #[case(11, "11th")]
    #[case(12, "12th")]
    #[case(13, "13th")]
    #[case(21, "21st")]
    #[case(102, "102nd")]
    #[case(111, "111th")]
    #[case(1003, "1003rd")]
    #[case(-1, "-1st")]
    #[case(-11, "-11th")]
    #[case(-22, "-22nd")]
    fn ordinalize_matches_rails(#[case] n: i64, #[case] expected: &str) {
        assert_eq!(ordinalize(n), expected);
    }

    #[rstest]
    #[case("12345678", "12,345,678")]
    #[case("0", "0")]
    #[case("123", "123")]
    #[case("123456", "123,456")]
    #[case("123456.78", "123,456.78")]
    #[case("-123456.78", "-123,456.78")]
    #[case("1234567.0001", "1,234,567.0001")]
    #[case("abc", "abc")]
    #[case("12a3", "12a3")]
    #[case("", "")]
    fn delimiter_matches_rails(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(number_with_delimiter(input), expected);
    }

    #[test]
    fn delimiter_options() {
        assert_eq!(number_with_delimiter(1_234_567), "1,234,567");
        assert_eq!(
            number_with_delimiter_using(12_345_678, ".", ","),
            "12.345.678"
        );
        assert_eq!(
            number_with_delimiter_using(12_345_678.05, " ", ","),
            "12 345 678,05"
        );
    }

    // Cases from Rails' number_helper_test.rb.
    #[rstest]
    #[case(-123.0, "-123")]
    #[case(-0.5, "-0.5")]
    #[case(0.0, "0")]
    #[case(0.5, "0.5")]
    #[case(123.0, "123")]
    #[case(1234.0, "1.23 Thousand")]
    #[case(12345.0, "12.3 Thousand")]
    #[case(1_234_567.0, "1.23 Million")]
    #[case(1_234_567_890.0, "1.23 Billion")]
    #[case(1_234_567_890_123.0, "1.23 Trillion")]
    #[case(1_234_567_890_123_456.0, "1.23 Quadrillion")]
    #[case(1_234_567_890_123_456_789.0, "1230 Quadrillion")]
    #[case(489_939.0, "490 Thousand")]
    #[case(489_939_000.0, "490 Million")]
    #[case(999_999.0, "1 Million")]
    #[case(1000.0, "1 Thousand")]
    #[case(1_500_000.0, "1.5 Million")]
    #[case(-1_234_567.0, "-1.23 Million")]
    fn number_to_human_matches_rails(#[case] n: f64, #[case] expected: &str) {
        assert_eq!(number_to_human(n), expected);
    }

    #[test]
    fn number_to_human_precision() {
        assert_eq!(number_to_human_with(489_939.0, 2, true), "490 Thousand");
        assert_eq!(number_to_human_with(489_939.0, 4, true), "489.9 Thousand");
        assert_eq!(number_to_human_with(1_234_567.0, 1, false), "1.2 Million");
        assert_eq!(number_to_human_with(1_234_567.0, 0, false), "1 Million");
        assert_eq!(number_to_human_with(12.346, 2, false), "12.35");
    }

    // `precision: 0` used to round to 0 and panic formatting ~2^31 decimals.
    #[rstest]
    #[case(1_234_567.0, true, "1 Million")]
    #[case(1_234_567.0, false, "1 Million")]
    #[case(0.4, true, "0")]
    #[case(0.0, true, "0")]
    #[case(-0.4, true, "0")]
    #[case(123.0, true, "123")]
    #[case(1500.0, true, "2 Thousand")]
    fn number_to_human_precision_zero(
        #[case] n: f64,
        #[case] significant: bool,
        #[case] expected: &str,
    ) {
        assert_eq!(number_to_human_with(n, 0, significant), expected);
    }

    // Huge precisions used to panic (over 65535 decimals) or print "NaN".
    #[rstest]
    #[case(1_234_567.0, true, "1.234567 Million")]
    #[case(1_234_567.0, false, "1.234567 Million")]
    #[case(0.5, true, "0.5")]
    #[case(0.1, false, "0.1")]
    #[case(1e-300, false, "0")]
    fn number_to_human_large_precision(
        #[case] n: f64,
        #[case] significant: bool,
        #[case] expected: &str,
    ) {
        for precision in [15, 17, 309, 400, 65_535, 65_536, u32::MAX] {
            assert_eq!(number_to_human_with(n, precision, significant), expected);
        }
    }

    // The extremes of `f64` print digits that read back as the same number.
    #[test]
    fn number_to_human_extremes() {
        for precision in [3, 15, 400, u32::MAX] {
            let tiny = number_to_human_with(1e-300, precision, true);
            assert_eq!(tiny.parse::<f64>().ok(), Some(1e-300), "{tiny}");
            let huge = number_to_human_with(f64::MAX, precision, true);
            let digits = huge.strip_suffix(" Quadrillion").expect(&huge);
            assert!(digits.bytes().all(|b| b.is_ascii_digit()), "{huge}");
        }
    }
}
