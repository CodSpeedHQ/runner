fn get_nearest_exponent(val: f64) -> i32 {
    match val {
        0.0 => 0,
        x if x >= 10f64.powi(0) => 0,
        x if x >= 10f64.powi(-3) => 3,
        x if x >= 10f64.powi(-6) => 6,
        _ => 9,
    }
}

fn format_shifted_value(value: f64, fraction_digits: usize) -> String {
    if fraction_digits == 0 && value.fract() == 0.0 {
        format!("{:.0}", value)
    } else {
        format!("{:.1$}", value, fraction_digits)
    }
}

fn format_duration_to_exponent(val: f64, exponent: i32, fraction_digits: usize) -> String {
    if val < 10f64.powi(-9) {
        return "< 1 ns".to_string();
    }

    match exponent {
        0 => format!("{} s", format_shifted_value(val, fraction_digits)),
        3 => format!(
            "{} ms",
            format_shifted_value(val * 10f64.powi(3), fraction_digits)
        ),
        6 => format!(
            "{} µs",
            format_shifted_value(val * 10f64.powi(6), fraction_digits)
        ),
        9 => format!(
            "{} ns",
            format_shifted_value(val * 10f64.powi(9), fraction_digits)
        ),
        _ => format!("{} s", val),
    }
}

pub(crate) fn format_duration(val: f64, fraction_digits: Option<usize>) -> String {
    let fraction_digits = fraction_digits.unwrap_or(1); // Default to 1 decimal place
    let exponent = get_nearest_exponent(val);
    format_duration_to_exponent(val, exponent, fraction_digits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration_seconds() {
        assert_eq!(format_duration(1.1, None), "1.1 s");
        assert_eq!(format_duration(1.1, Some(0)), "1 s");
        assert_eq!(format_duration(1.234, Some(2)), "1.23 s");
    }

    #[test]
    fn test_format_duration_milliseconds() {
        assert_eq!(format_duration(0.001, None), "1.0 ms");
        assert_eq!(format_duration(0.001, Some(0)), "1 ms");
        assert_eq!(format_duration(0.001234, Some(2)), "1.23 ms");
    }

    #[test]
    fn test_format_duration_microseconds() {
        assert_eq!(format_duration(0.000001, None), "1.0 µs");
        assert_eq!(format_duration(0.000001234, Some(2)), "1.23 µs");
    }

    #[test]
    fn test_format_duration_nanoseconds() {
        assert_eq!(format_duration(0.000000001, None), "1.0 ns");
        assert_eq!(format_duration(0.000000001234, Some(2)), "1.23 ns");
    }

    #[test]
    fn test_format_duration_less_than_nanosecond() {
        assert_eq!(format_duration(0.0000000001, None), "< 1 ns");
    }

    #[test]
    fn test_format_duration_zero() {
        assert_eq!(format_duration(0.0, None), "< 1 ns");
    }
}
