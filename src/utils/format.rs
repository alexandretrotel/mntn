/// Converts a byte count into a human-readable string with units (bytes, KB, MB, GB).
///
/// Uses base 1024 for unit conversion.
///
/// # Arguments
///
/// * `bytes` - The number of bytes to convert.
///
/// # Returns
///
/// A formatted string representing the size in an appropriate unit with two decimal places.
///
/// # Examples
///
/// ```
/// use mntn::utils::format::bytes_to_human_readable;
///
/// assert_eq!(bytes_to_human_readable(1024), "1.00 KB");
/// assert_eq!(bytes_to_human_readable(500), "500 bytes");
/// ```
pub fn bytes_to_human_readable(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}
