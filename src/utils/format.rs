/// Converts a byte count into a human-readable string with IEC units (bytes, KiB, MiB, GiB, ...).
///
/// Uses base 1024 for unit conversion (e.g., 1 KiB = 1024 bytes).
///
/// Examples:
/// - `500` -> `"500 bytes"`
/// - `2048` -> `"2 KiB"`
/// - `1048576` -> `"1 MiB"`
pub fn bytes_to_human_readable(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["bytes", "KiB", "MiB", "GiB", "TiB", "PiB"];

    if bytes < 1024 {
        return format!("{} bytes", bytes);
    }

    let mut size = bytes as f64;
    let mut unit = 0;

    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }

    let formatted_size = format_decimal(size);
    format!("{} {}", formatted_size, UNITS[unit])
}

/// Formats a float with up to 2 decimal places, removing trailing zeros and the decimal point if not needed.
///
/// Examples:
/// - `2.00` -> `"2"`
/// - `2.50` -> `"2.5"`
/// - `2.75` -> `"2.75"`
fn format_decimal(value: f64) -> String {
    let mut s = format!("{:.2}", value);
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_bytes() {
        assert_eq!(bytes_to_human_readable(0), "0 bytes");
    }

    #[test]
    fn test_one_byte() {
        assert_eq!(bytes_to_human_readable(1), "1 bytes");
    }

    #[test]
    fn test_bytes_below_kib() {
        assert_eq!(bytes_to_human_readable(512), "512 bytes");
        assert_eq!(bytes_to_human_readable(1023), "1023 bytes");
    }

    #[test]
    fn test_exactly_one_kib() {
        assert_eq!(bytes_to_human_readable(1024), "1 KiB");
    }

    #[test]
    fn test_kib_with_fraction() {
        // 1.5 KiB = 1536 bytes
        assert_eq!(bytes_to_human_readable(1536), "1.5 KiB");
        // 2.5 KiB = 2560 bytes
        assert_eq!(bytes_to_human_readable(2560), "2.5 KiB");
    }

    #[test]
    fn test_exactly_one_mib() {
        // 1 MiB = 1024 * 1024 = 1048576 bytes
        assert_eq!(bytes_to_human_readable(1048576), "1 MiB");
    }

    #[test]
    fn test_mib_with_fraction() {
        // 1.5 MiB = 1572864 bytes
        assert_eq!(bytes_to_human_readable(1572864), "1.5 MiB");
    }

    #[test]
    fn test_exactly_one_gib() {
        // 1 GiB = 1024^3 = 1073741824 bytes
        assert_eq!(bytes_to_human_readable(1073741824), "1 GiB");
    }

    #[test]
    fn test_gib_with_fraction() {
        // 2.25 GiB
        assert_eq!(bytes_to_human_readable(2415919104), "2.25 GiB");
    }

    #[test]
    fn test_exactly_one_tib() {
        // 1 TiB = 1024^4 = 1099511627776 bytes
        assert_eq!(bytes_to_human_readable(1099511627776), "1 TiB");
    }

    #[test]
    fn test_exactly_one_pib() {
        // 1 PiB = 1024^5 = 1125899906842624 bytes
        assert_eq!(bytes_to_human_readable(1125899906842624), "1 PiB");
    }

    #[test]
    fn test_large_pib_value() {
        // 10 PiB
        assert_eq!(bytes_to_human_readable(11258999068426240), "10 PiB");
    }

    #[test]
    fn test_max_u64() {
        // u64::MAX should produce a PiB result without panicking
        let result = bytes_to_human_readable(u64::MAX);
        assert!(result.contains("PiB"));
    }

    #[test]
    fn test_format_decimal_whole_number() {
        assert_eq!(format_decimal(2.0), "2");
        assert_eq!(format_decimal(100.0), "100");
    }

    #[test]
    fn test_format_decimal_single_decimal() {
        assert_eq!(format_decimal(2.5), "2.5");
        assert_eq!(format_decimal(1.1), "1.1");
    }

    #[test]
    fn test_format_decimal_two_decimals() {
        assert_eq!(format_decimal(2.75), "2.75");
        assert_eq!(format_decimal(3.14), "3.14");
    }

    #[test]
    fn test_format_decimal_rounds_to_two_places() {
        // 2.999 should round to 3.00 -> "3"
        assert_eq!(format_decimal(2.999), "3");
        // 2.555 should round to 2.56
        assert_eq!(format_decimal(2.555), "2.56");
    }

    #[test]
    fn test_format_decimal_trailing_zero_removal() {
        assert_eq!(format_decimal(1.10), "1.1");
        assert_eq!(format_decimal(1.00), "1");
    }
}
