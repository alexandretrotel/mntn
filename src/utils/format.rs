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
