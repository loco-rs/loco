//! Byte sizes as config and templates spell them: `2mb`, `1.5 GiB`, `512k`,
//! `100`. The unit is case-insensitive. `K`/`KB` and the rest are powers of
//! 1000, `Ki`/`KiB` powers of 1024, a bare number is bytes, and a `bit` or
//! `bits` suffix (`10mbit`) counts bits. Fractions round up to a whole byte.

const PREFIXES: &[u8] = b"KMGTPE";
const DECIMAL_UNITS: [&str; 6] = ["KB", "MB", "GB", "TB", "PB", "EB"];
/// The most fraction digits a size may carry.
const MAX_SCALE: u32 = 28;

/// Parses a size into bytes.
///
/// # Errors
///
/// When the text is not a non-negative number followed by a known unit, or
/// the size does not fit in a `u64`.
pub fn parse(text: &str) -> Result<u64, String> {
    let text = text.trim();
    let invalid = || format!("`{text}` is not a byte size");
    let digits = text.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return Err(invalid());
    }
    let (whole, rest) = text.split_at(digits);
    let (fraction, rest) = match rest.strip_prefix('.') {
        Some(rest) => {
            let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
            if digits == 0 {
                return Err(invalid());
            }
            rest.split_at(digits)
        }
        None => ("", rest),
    };
    let scale = u32::try_from(fraction.len()).map_err(|_| invalid())?;
    if scale > MAX_SCALE {
        return Err(invalid());
    }
    // The number as an integer over 10^scale.
    let mantissa = format!("{whole}{fraction}")
        .parse::<u128>()
        .map_err(|_| invalid())?;
    let (per_unit, bits) = unit(rest.trim_start_matches(' ')).ok_or_else(invalid)?;

    let mut numerator = mantissa;
    let mut denominator = 10u128.pow(scale);
    // A bit is an eighth of a byte.
    let mut per_unit = per_unit;
    if bits {
        denominator *= 8;
    }
    // Cancel common factors first, so a long fraction of a large unit does
    // not overflow on the way to a size that fits.
    let common = gcd(per_unit, denominator);
    per_unit /= common;
    denominator /= common;
    numerator = numerator.checked_mul(per_unit).ok_or_else(invalid)?;
    u64::try_from(numerator.div_ceil(denominator)).map_err(|_| invalid())
}

/// Formats bytes in the largest power-of-1000 unit they reach, to two
/// decimals: `1.23 KB`, `70.69 MB`. Below a kilobyte it is `512 B`.
#[must_use]
pub fn to_human(bytes: u64) -> String {
    let mut size = 1_000_000_000_000_000_000u64;
    for name in DECIMAL_UNITS.iter().rev() {
        if bytes >= size {
            #[allow(clippy::cast_precision_loss)]
            return format!("{:.2} {name}", bytes as f64 / size as f64);
        }
        size /= 1000;
    }
    format!("{bytes} B")
}

/// The bytes in one `unit`, and whether it counts bits.
fn unit(unit: &str) -> Option<(u128, bool)> {
    let unit = unit.to_ascii_uppercase();
    let (per_unit, rest) = match unit.as_bytes().first() {
        None => return Some((1, false)),
        Some(b'B') => (1, &unit[..]),
        Some(prefix) => {
            let power = PREFIXES.iter().position(|p| p == prefix)? + 1;
            let power = u32::try_from(power).ok()?;
            match unit[1..].strip_prefix('I') {
                // `Ki` alone is still bytes.
                Some("") => return Some((1024u128.pow(power), false)),
                Some(rest) => (1024u128.pow(power), rest),
                None if unit.len() == 1 => return Some((1000u128.pow(power), false)),
                None => (1000u128.pow(power), &unit[1..]),
            }
        }
    };
    match rest {
        "B" => Some((per_unit, false)),
        "BIT" | "BITS" => Some((per_unit, true)),
        _ => None,
    }
}

const fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case("100", 100)]
    #[case("2mb", 2_000_000)]
    #[case("2MB", 2_000_000)]
    #[case("512k", 512_000)]
    #[case("1.5 GiB", 1_610_612_736)]
    #[case("1ki", 1024)]
    #[case("10mbit", 1_250_000)]
    #[case("1bit", 1)]
    #[case("0.1b", 1)]
    #[case("  3   kb ", 3000)]
    fn parses(#[case] text: &str, #[case] bytes: u64) {
        assert_eq!(parse(text), Ok(bytes));
    }

    #[rstest]
    #[case("")]
    #[case("mb")]
    #[case("-1")]
    #[case("1.")]
    #[case("1.5.5")]
    #[case("5 xb")]
    #[case("5 kbyte")]
    #[case("20 EB")]
    fn rejects(#[case] text: &str) {
        assert!(parse(text).is_err(), "{text}");
    }

    #[test]
    fn formats() {
        assert_eq!(to_human(512), "512 B");
        assert_eq!(to_human(1234), "1.23 KB");
        assert_eq!(to_human(70_691_577), "70.69 MB");
        assert_eq!(to_human(u64::MAX), "18.45 EB");
    }
}
