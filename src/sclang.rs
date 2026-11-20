//! The arithmetic of sclang on numbers, and its reading of a number in a
//! string.

/// The frequency of the MIDI note `note`, in hertz, as `sc_midicps` of
/// SuperCollider computes it for a float64.
pub(crate) fn midicps(note: f64) -> f64 {
    440.0 * 2.0_f64.powf((note - 69.0) / 12.0)
}

/// The amplitude of a level of `db` decibels, as `sc_dbamp` of SuperCollider
/// computes it for a float64.
pub(crate) fn dbamp(db: f64) -> f64 {
    10.0_f64.powf(db * 0.05)
}

/// The base-2 logarithm of the magnitude of `x`, as `sc_log2` of
/// SuperCollider computes it for a float64.
pub(crate) fn log2(x: f64) -> f64 {
    x.abs().log2()
}

/// `a.min(b)` of sclang, as `sc_min` of SuperCollider computes it: `a` if it
/// is below `b`, else `b`.
pub(crate) fn min(a: f64, b: f64) -> f64 {
    if a < b { a } else { b }
}

/// `x.asInteger` of sclang: `x` truncated toward 0 to an `i32`. A value past
/// the range of `i32` saturates, and a NaN is 0. `wrapAt` truncates a Float
/// index the same way.
#[allow(
    clippy::cast_possible_truncation,
    reason = "asInteger of sclang truncates a Float toward 0 to a 32-bit Integer, and saturates on arm64 (measured)"
)]
pub(crate) fn as_integer(x: f64) -> i32 {
    x as i32
}

/// `text.asFloat` of sclang: `atof` of C on the first 254 bytes of `text`,
/// up to a NUL. After white space and a sign, it reads the longest decimal
/// number at the start, with a point and an exponent, or `inf` or `nan` in
/// any case. A text without a number gives 0, and a NaN has the payload of
/// [`f64::NAN`]. `atof` also reads a hexadecimal number after `0x`, where
/// this function reads the decimal 0 before the `x`.
pub(crate) fn as_float(text: &str) -> f64 {
    let bytes = text.as_bytes();
    let bytes = bytes.get(..254).unwrap_or(bytes);
    let bytes = bytes.split(|&byte| byte == 0).next().unwrap_or_default();
    let start = bytes
        .iter()
        .position(|&byte| !matches!(byte, b' ' | b'\t' | b'\n' | 0x0B | 0x0C | b'\r'))
        .unwrap_or(bytes.len());
    let (negative, rest) = match &bytes[start..] {
        [b'-', rest @ ..] => (true, rest),
        [b'+', rest @ ..] | rest => (false, rest),
    };
    let magnitude = infinity_or_nan(rest).or_else(|| decimal(rest));
    match magnitude {
        Some(magnitude) if negative => -magnitude,
        Some(magnitude) => magnitude,
        None => 0.0,
    }
}

/// The count of the decimal digits at the start of `bytes`.
fn digits(bytes: &[u8]) -> usize {
    bytes
        .iter()
        .take_while(|byte| byte.is_ascii_digit())
        .count()
}

/// The decimal number at the start of `bytes`: digits with an optional
/// point, at least one digit, and an exponent if digits follow its `e`.
fn decimal(bytes: &[u8]) -> Option<f64> {
    let whole = digits(bytes);
    let mut end = whole;
    if bytes.get(end) == Some(&b'.') {
        let fraction = digits(&bytes[end + 1..]);
        if whole + fraction == 0 {
            return None;
        }
        end += 1 + fraction;
    }
    if end == 0 {
        return None;
    }
    if let Some(b'e' | b'E') = bytes.get(end) {
        let sign = usize::from(matches!(bytes.get(end + 1), Some(b'+' | b'-')));
        let exponent = bytes.get(end + 1 + sign..).map_or(0, digits);
        if exponent > 0 {
            end += 1 + sign + exponent;
        }
    }
    std::str::from_utf8(&bytes[..end]).ok()?.parse().ok()
}

/// Infinity for `inf`, and NaN for `nan`, at the start of `bytes` in any
/// case.
fn infinity_or_nan(bytes: &[u8]) -> Option<f64> {
    let starts = |word: &[u8]| {
        bytes
            .get(..word.len())
            .is_some_and(|start| start.eq_ignore_ascii_case(word))
    };
    if starts(b"inf") {
        Some(f64::INFINITY)
    } else if starts(b"nan") {
        Some(f64::NAN)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{as_float, as_integer, dbamp, log2, midicps, min};

    /// The bits of `x`.
    fn bits(x: f64) -> u64 {
        x.to_bits()
    }

    #[test]
    fn midicps_of_60_is_261_6255653005986_and_of_69_is_440() {
        assert_eq!(midicps(60.0).to_bits(), 261.625_565_300_598_6_f64.to_bits());
        assert_eq!(midicps(69.0).to_bits(), 440.0_f64.to_bits());
    }

    #[test]
    fn dbamp_of_0_is_1_and_log2_of_8_and_of_minus_8_is_3() {
        assert_eq!(dbamp(0.0).to_bits(), 1.0_f64.to_bits());
        assert_eq!(log2(8.0).to_bits(), 3.0_f64.to_bits());
        assert_eq!(log2(-8.0).to_bits(), 3.0_f64.to_bits());
    }

    #[test]
    fn min_gives_its_second_number_unless_the_first_is_below_it_as_measured() {
        assert_eq!(bits(min(0.0, -0.0)), bits(-0.0));
        assert_eq!(bits(min(-0.0, 0.0)), bits(0.0));
        assert_eq!(bits(min(f64::NAN, 1.0)), bits(1.0));
        assert!(min(1.0, f64::NAN).is_nan());
        assert_eq!(bits(min(0.25, 0.5)), bits(0.25));
        assert_eq!(bits(min(0.5, 0.25)), bits(0.25));
    }

    #[test]
    fn as_integer_truncates_toward_0_and_saturates_as_measured() {
        for (x, integer) in [
            (1.7, 1),
            (-1.5, -1),
            (-0.5, 0),
            (-0.0, 0),
            (f64::NAN, 0),
            (f64::INFINITY, i32::MAX),
            (f64::NEG_INFINITY, i32::MIN),
            (3e9, i32::MAX),
            (-3e9, i32::MIN),
            (2_147_483_647.5, i32::MAX),
            (-2_147_483_648.5, i32::MIN),
            (1e300, i32::MAX),
        ] {
            assert_eq!(as_integer(x), integer, "{x}");
        }
    }

    #[test]
    fn as_float_reads_the_decimal_number_at_the_start_as_measured() {
        for (text, number) in [
            ("3", 3.0),
            ("+2", 2.0),
            ("-2", -2.0),
            ("1e3", 1000.0),
            ("1e", 1.0),
            ("1e+", 1.0),
            ("1e+x", 1.0),
            ("1E2", 100.0),
            (".5", 0.5),
            ("5.", 5.0),
            ("-.5", -0.5),
            ("1.e2", 100.0),
            ("1.5e-3x", 0.0015),
            ("1_000", 1.0),
            ("1,5", 1.0),
            ("1f", 1.0),
            ("0.5d", 0.5),
            ("3 ", 3.0),
            ("1x", 1.0),
            ("2.5", 2.5),
            ("3.7z", 3.7),
            ("00000000000000000000000000000000000000001", 1.0),
            (
                "0.000000000000000000000000000000000000000000000000001e51",
                1.0,
            ),
            ("2.5000000000000000000000000000000000000000000000001", 2.5),
            ("1.7976931348623157e308", f64::MAX),
            ("1.7976931348623158e308", f64::MAX),
            ("1.7976931348623159e308", f64::INFINITY),
            ("1e400", f64::INFINITY),
            ("-1e400", f64::NEG_INFINITY),
            ("1e99999999999", f64::INFINITY),
            ("1e-400", 0.0),
            ("1e-99999999999", 0.0),
            ("4.9e-324", f64::from_bits(1)),
            ("2.4703282292062328e-324", f64::from_bits(1)),
        ] {
            assert_eq!(bits(as_float(text)), bits(number), "{text:?}");
        }
    }

    #[test]
    fn as_float_skips_white_space_and_gives_0_without_a_number_as_measured() {
        for (text, number) in [
            (" 3", 3.0),
            ("\t3", 3.0),
            ("\n3", 3.0),
            ("\u{b}3", 3.0),
            ("\u{c}3", 3.0),
            ("\r3", 3.0),
            (" -3", -3.0),
            ("", 0.0),
            ("x", 0.0),
            (".", 0.0),
            ("-", 0.0),
            ("+", 0.0),
            ("e5", 0.0),
            (".e2", 0.0),
            ("- 3", 0.0),
            ("--1", 0.0),
            ("+-1", 0.0),
            ("0b101", 0.0),
        ] {
            assert_eq!(bits(as_float(text)), bits(number), "{text:?}");
        }
    }

    #[test]
    fn as_float_stops_at_a_nul_as_atof_does_on_the_c_string_of_its_primitive() {
        assert_eq!(bits(as_float("\u{0}3")), bits(0.0));
        assert_eq!(bits(as_float("1\u{0}2")), bits(1.0));
    }

    #[test]
    fn as_float_reads_a_hexadecimal_number_as_its_decimal_start_unlike_atof() {
        // "0x2".asFloat is 2.0 in sclang 3.14.1 (measured).
        assert_eq!(bits(as_float("0x2")), bits(0.0));
    }

    #[test]
    fn as_float_reads_inf_and_nan_in_any_case_as_measured() {
        for text in [
            "inf", "INF", "Inf", "infinity", "Infinity", "iNfInItY", "infx", "infinit",
        ] {
            assert_eq!(bits(as_float(text)), bits(f64::INFINITY), "{text:?}");
        }
        assert_eq!(bits(as_float("-inf")), bits(f64::NEG_INFINITY));
        for text in [
            "nan", "NAN", "NaN", "nAn", "nan(123)", "nan()", "nanx", "nan(",
        ] {
            let number = as_float(text);
            assert!(number.is_nan() && number.is_sign_positive(), "{text:?}");
        }
        let number = as_float("-nan");
        assert!(number.is_nan() && number.is_sign_negative());
    }

    #[test]
    fn as_float_reads_the_first_254_bytes_alone_as_measured() {
        assert_eq!(bits(as_float(&format!("{}1", "0".repeat(260)))), 0);
        let number = as_float(&format!("1{}9", "0".repeat(253)));
        assert_eq!(bits(number), 0x7475_D2CE_5574_7A18);
        assert_eq!(bits(as_float(&"9".repeat(254))), 0x74AB_4781_EAD1_989E);
        assert_eq!(bits(as_float(&"9".repeat(255))), 0x74AB_4781_EAD1_989E);
    }
}
