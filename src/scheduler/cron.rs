//! The cron expressions the scheduler runs on: seconds first and an optional
//! year last, evaluated in UTC.
//!
//! ```text
//! sec  min  hour  day-of-month  month  day-of-week  [year]
//! ```
//!
//! Every field takes `*`, a number, a range `a-b`, a step (`*/n`, `a/n`,
//! `a-b/n`) and comma-separated lists of those. Months and weekdays also take
//! names (`JAN`, `MON`), and Sunday is either `0` or `7`. In the two day
//! fields `?` means `*`. Day of month adds `L` (the last day) and `nW` (the
//! weekday nearest the nth). Day of week adds `nL` (the last such weekday of
//! the month) and `n#k` (the kth such weekday). A day has to match both day
//! fields.

use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, Timelike, Utc};

const MONTHS: [&str; 12] = [
    "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
];
const WEEKDAYS: [&str; 7] = ["SUN", "MON", "TUE", "WED", "THU", "FRI", "SAT"];
const YEARS: (u32, u32) = (1, 5000);

/// A parsed cron expression.
#[derive(Debug, Clone)]
pub struct Cron {
    seconds: u64,
    minutes: u64,
    hours: u64,
    months: u64,
    /// Days of month, by bit.
    days: u64,
    last_day: bool,
    /// Days of month whose nearest weekday matches (`nW`).
    nearest_weekdays: u64,
    /// Weekdays, by bit from Sunday.
    weekdays: u64,
    /// Weekdays matching only as the last of their month (`nL`).
    last_weekdays: u64,
    /// Weekdays matching only as the kth of their month (`n#k`), by k - 1.
    nth_weekdays: [u64; 5],
    years: Vec<(u32, u32, u32)>,
}

impl Cron {
    /// Parses a 6- or 7-field expression.
    ///
    /// # Errors
    ///
    /// When the expression has the wrong number of fields, or a field holds
    /// a value, range or character it does not allow.
    pub fn parse(expression: &str) -> Result<Self, String> {
        let expression = expression.trim().to_uppercase();
        let mut fields = expression.split_whitespace().collect::<Vec<_>>();
        match fields.len() {
            6 => fields.push("*"),
            7 => {}
            n => return Err(format!("expected 6 or 7 fields, found {n}")),
        }
        let day_field = |field: &str| {
            if field == "?" {
                "*".to_string()
            } else {
                field.to_string()
            }
        };

        let mut cron = Self {
            seconds: bits(&parse_field(fields[0], 0, 59)?),
            minutes: bits(&parse_field(fields[1], 0, 59)?),
            hours: bits(&parse_field(fields[2], 0, 23)?),
            months: bits(&parse_field(&with_names(fields[4], &MONTHS, 1), 1, 12)?),
            days: 0,
            last_day: false,
            nearest_weekdays: 0,
            weekdays: 0,
            last_weekdays: 0,
            nth_weekdays: [0; 5],
            years: parse_field(fields[6], YEARS.0, YEARS.1)?,
        };

        let days = day_field(fields[3]);
        if parts(&days).next().is_none() {
            return Err(format!("empty field `{days}`"));
        }
        for part in parts(&days) {
            if part == "L" {
                cron.last_day = true;
            } else if let Some(day) = part.strip_suffix('W') {
                cron.nearest_weekdays |= 1 << number(day, 1, 31)?;
            } else {
                cron.days |= bits(&[range(part, 1, 31)?]);
            }
        }

        // `MON-SUN` reads Sunday as 7, the end of the week.
        let weekdays = with_names(&day_field(fields[5]).replace("-SUN", "-7"), &WEEKDAYS, 0);
        if parts(&weekdays).next().is_none() {
            return Err(format!("empty field `{weekdays}`"));
        }
        for part in parts(&weekdays) {
            if let Some(day) = part.strip_suffix('L') {
                cron.last_weekdays |= weekday_bits(&[(number(day, 0, 7)?, number(day, 0, 7)?, 1)]);
            } else if let Some((days, nth)) = part.split_once('#') {
                let nth = number(nth, 1, 5)?;
                cron.nth_weekdays[nth as usize - 1] |= weekday_bits(&[range(days, 0, 7)?]);
            } else {
                cron.weekdays |= weekday_bits(&[range(part, 0, 7)?]);
            }
        }
        Ok(cron)
    }

    /// The first time strictly after `after`, on a whole second, or `None`
    /// when no time up to the year 5000 matches.
    #[must_use]
    pub fn next_after(&self, after: DateTime<Utc>) -> Option<DateTime<Utc>> {
        let after = after.naive_utc();
        let mut t = after.with_nanosecond(0)? + Duration::seconds(1);
        loop {
            let year = u32::try_from(t.year()).ok()?;
            if year > YEARS.1 {
                return None;
            }
            if !self
                .years
                .iter()
                .any(|&(lo, hi, step)| matches(year, lo, hi, step))
            {
                t = start_of(t.year() + 1, 1, 1)?;
            } else if !has(self.months, t.month()) {
                t = if t.month() == 12 {
                    start_of(t.year() + 1, 1, 1)?
                } else {
                    start_of(t.year(), t.month() + 1, 1)?
                };
            } else if !self.day_matches(t.date()) {
                t = t.date().succ_opt()?.and_hms_opt(0, 0, 0)?;
            } else if !has(self.hours, t.hour()) {
                t = t.with_minute(0)?.with_second(0)? + Duration::hours(1);
            } else if !has(self.minutes, t.minute()) {
                t = t.with_second(0)? + Duration::minutes(1);
            } else if !has(self.seconds, t.second()) {
                t += Duration::seconds(1);
            } else {
                return Some(t.and_utc());
            }
        }
    }

    fn day_matches(&self, date: NaiveDate) -> bool {
        let day = date.day();
        let last = last_day_of_month(date);
        let by_month = has(self.days, day)
            || (self.last_day && day == last)
            || (1..=last).any(|n| has(self.nearest_weekdays, n) && nearest_weekday(date, n) == day);

        let weekday = date.weekday().num_days_from_sunday();
        let nth = (day - 1) / 7;
        let by_week = has(self.weekdays, weekday)
            || (has(self.last_weekdays, weekday) && day + 7 > last)
            || self
                .nth_weekdays
                .get(nth as usize)
                .is_some_and(|set| has(*set, weekday));
        by_month && by_week
    }
}

/// The day the `nW` rule picks in `date`'s month: the nth itself on a
/// weekday, otherwise the closest weekday that stays inside the month.
fn nearest_weekday(date: NaiveDate, n: u32) -> u32 {
    let Some(target) = date.with_day(n) else {
        return 0;
    };
    match target.weekday().num_days_from_sunday() {
        6 if n == 1 => n + 2,
        6 => n - 1,
        0 if n == last_day_of_month(date) => n - 2,
        0 => n + 1,
        _ => n,
    }
}

fn last_day_of_month(date: NaiveDate) -> u32 {
    let (year, month) = if date.month() == 12 {
        (date.year() + 1, 1)
    } else {
        (date.year(), date.month() + 1)
    };
    NaiveDate::from_ymd_opt(year, month, 1)
        .and_then(|first| first.pred_opt())
        .map_or(31, |last| last.day())
}

fn start_of(year: i32, month: u32, day: u32) -> Option<NaiveDateTime> {
    NaiveDate::from_ymd_opt(year, month, day)?.and_hms_opt(0, 0, 0)
}

/// The comma-separated parts of a field; empty parts are skipped.
fn parts(field: &str) -> impl Iterator<Item = &str> {
    field
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
}

/// Spells the names in `field` as numbers, the first name being `first`.
fn with_names(field: &str, names: &[&str], first: usize) -> String {
    names
        .iter()
        .enumerate()
        .fold(field.to_string(), |field, (i, name)| {
            field.replace(name, &(i + first).to_string())
        })
}

fn parse_field(field: &str, min: u32, max: u32) -> Result<Vec<(u32, u32, u32)>, String> {
    let ranges = parts(field)
        .map(|part| range(part, min, max))
        .collect::<Result<Vec<_>, _>>()?;
    if ranges.is_empty() {
        return Err(format!("empty field `{field}`"));
    }
    Ok(ranges)
}

/// One part of a field as `(first, last, step)`.
fn range(part: &str, min: u32, max: u32) -> Result<(u32, u32, u32), String> {
    let (span, step) = match part.split_once('/') {
        Some((span, step)) => (span, number(step, 1, u32::from(u16::MAX))?),
        None => (part, 1),
    };
    let (lo, hi) = if span == "*" {
        (min, max)
    } else if let Some((lo, hi)) = span.split_once('-') {
        (number(lo, min, max)?, number(hi, min, max)?)
    } else {
        let lo = number(span, min, max)?;
        // `a/n` runs from a to the end of the field; a bare `a` is just a.
        (
            lo,
            if step > 1 || part.contains('/') {
                max
            } else {
                lo
            },
        )
    };
    if lo > hi {
        return Err(format!("range `{part}` runs backwards"));
    }
    Ok((lo, hi, step))
}

fn number(text: &str, min: u32, max: u32) -> Result<u32, String> {
    let value = text
        .parse::<u32>()
        .map_err(|_| format!("`{text}` is not a number"))?;
    if (min..=max).contains(&value) {
        Ok(value)
    } else {
        Err(format!("{value} is outside {min}-{max}"))
    }
}

const fn matches(value: u32, lo: u32, hi: u32, step: u32) -> bool {
    value >= lo && value <= hi && (value - lo).is_multiple_of(step)
}

fn bits(ranges: &[(u32, u32, u32)]) -> u64 {
    ranges.iter().fold(0, |set, &(lo, hi, step)| {
        (lo..=hi)
            .step_by(step as usize)
            .fold(set, |set, value| set | (1 << value))
    })
}

/// Weekday bits with 7 folded onto Sunday.
fn weekday_bits(ranges: &[(u32, u32, u32)]) -> u64 {
    let set = bits(ranges);
    (set & 0x7f) | ((set >> 7) & 1)
}

const fn has(set: u64, value: u32) -> bool {
    set & (1 << value) != 0
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn next(expression: &str, after: DateTime<Utc>) -> Option<String> {
        Cron::parse(expression)
            .unwrap()
            .next_after(after)
            .map(|t| t.format("%Y-%m-%d %a %H:%M:%S").to_string())
    }

    fn at(y: i32, m: u32, d: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, 12, 0, 0).unwrap()
    }

    #[test]
    fn fires_on_the_next_whole_second() {
        let after = at(2026, 10, 18) + Duration::milliseconds(640);
        assert_eq!(
            next("*/5 * * * * *", after).unwrap(),
            "2026-10-18 Sun 12:00:05"
        );
    }

    #[test]
    fn day_rules() {
        let oct = at(2026, 10, 1);
        assert_eq!(
            next("0 0 9 L * ? *", oct).unwrap(),
            "2026-10-31 Sat 09:00:00"
        );
        // The 31st is a Saturday, so 31W is Friday the 30th.
        assert_eq!(
            next("0 0 9 31W * ? *", oct).unwrap(),
            "2026-10-30 Fri 09:00:00"
        );
        // Nov 1st is a Sunday; 1W moves forward to Monday the 2nd.
        assert_eq!(
            next("0 0 9 1W 11 ? *", oct).unwrap(),
            "2026-11-02 Mon 09:00:00"
        );
        assert_eq!(
            next("0 0 9 ? * FRI#2 *", oct).unwrap(),
            "2026-10-09 Fri 09:00:00"
        );
        assert_eq!(
            next("0 0 9 ? * 5L *", oct).unwrap(),
            "2026-10-30 Fri 09:00:00"
        );
        assert_eq!(
            next("0 0 9 ? * MON-SUN *", oct).unwrap(),
            "2026-10-02 Fri 09:00:00"
        );
        assert_eq!(
            next("0 0 9 * * 7 *", oct).unwrap(),
            "2026-10-04 Sun 09:00:00"
        );
        // Both day fields have to match: a Friday the 13th.
        assert_eq!(
            next("0 0 0 13 * FRI *", oct).unwrap(),
            "2026-11-13 Fri 00:00:00"
        );
    }

    #[test]
    fn years_and_impossible_dates() {
        assert_eq!(
            next("0 0 0 29 FEB ? *", at(2026, 3, 1)).unwrap(),
            "2028-02-29 Tue 00:00:00"
        );
        assert_eq!(next("0 0 0 1 1 ? 2020,2024", at(2026, 1, 1)), None);
        assert_eq!(next("0 0 0 30 FEB ? *", at(2026, 1, 1)), None);
    }

    #[test]
    fn rejects_malformed_fields() {
        for expression in [
            "* * * * *",
            "60 * * * * *",
            "* * * 0 * *",
            "* * * * * 8",
            "*/0 * * * * *",
            "* * 5-2 * * *",
            "* * * L-2 * *",
            "* * * * * L",
            "* * * * * 1#6",
            "* * * * * ,",
        ] {
            assert!(Cron::parse(expression).is_err(), "{expression}");
        }
    }
}
