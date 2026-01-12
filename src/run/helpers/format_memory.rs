const BASE: f64 = 1024.0;
const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];

fn get_unit_index(bytes: f64) -> usize {
    if bytes == 0.0 {
        return 0;
    }
    let index = (bytes.ln() / BASE.ln()).floor() as usize;
    index.min(UNITS.len() - 1)
}

fn format_shifted_value(value: f64, fraction_digits: usize) -> String {
    let formatted_value = format!("{value:.fraction_digits$}");

    if fraction_digits > 0 {
        formatted_value
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_owned()
    } else {
        formatted_value
    }
}

pub(crate) fn format_memory(bytes: f64, fraction_digits: Option<usize>) -> String {
    let fraction_digits = fraction_digits.unwrap_or(1); // Default to 1 decimal place

    if bytes == 0.0 {
        return "0 B".to_string();
    }

    let unit_index = get_unit_index(bytes);
    let unit = UNITS[unit_index];
    let shifted_value = bytes / BASE.powi(unit_index as i32);

    format!(
        "{} {}",
        format_shifted_value(shifted_value, fraction_digits),
        unit
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_memory_bytes() {
        assert_eq!(format_memory(100.0, None), "100 B");
        assert_eq!(format_memory(100.0, Some(0)), "100 B");
        assert_eq!(format_memory(123.45, Some(2)), "123.45 B");
    }

    #[test]
    fn test_format_memory_kilobytes() {
        assert_eq!(format_memory(1024.0, None), "1 KB");
        assert_eq!(format_memory(1024.0, Some(0)), "1 KB");
        assert_eq!(format_memory(1536.0, Some(1)), "1.5 KB");
        assert_eq!(format_memory(1263.0, Some(2)), "1.23 KB");
    }

    #[test]
    fn test_format_memory_megabytes() {
        assert_eq!(format_memory(1048576.0, None), "1 MB"); // 1024^2
        assert_eq!(format_memory(1258291.0, Some(2)), "1.2 MB");
    }

    #[test]
    fn test_format_memory_gigabytes() {
        assert_eq!(format_memory(1073741824.0, None), "1 GB"); // 1024^3
        assert_eq!(format_memory(1288490188.8, Some(2)), "1.2 GB");
    }

    #[test]
    fn test_format_memory_zero() {
        assert_eq!(format_memory(0.0, None), "0 B");
    }

    #[test]
    fn test_format_memory_no_trailing_zeros() {
        assert_eq!(format_memory(2048.0, None), "2 KB"); // Should be "2 KB" not "2.0 KB"
        assert_eq!(format_memory(3072.0, None), "3 KB"); // Should be "3 KB" not "3.0 KB"
    }
}
