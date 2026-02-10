use ulid::Ulid;

pub fn generate_ulid() -> String {
    Ulid::new().to_string()
}

/// Safely slices a string at the given byte positions, adjusting to char boundaries.
///
/// # Arguments
/// * `content` - The string to slice
/// * `start` - Start byte position (will be adjusted to char boundary if needed)
/// * `end` - End byte position (will be adjusted to char boundary if needed)
///
/// # Returns
/// A string slice from the adjusted start to adjusted end
pub fn safe_str_slice(content: &str, start: usize, end: usize) -> &str {
    if start >= content.len() {
        panic!(
            "safe_str_slice: start ({}) >= content length ({})",
            start,
            content.len()
        );
    }
    if start == end {
        return "";
    }

    if start > end {
        panic!("safe_str_slice: start ({}) >= end ({})", start, end);
    }

    let mut adjusted_start = start.min(content.len());
    let mut adjusted_end = end.min(content.len());

    while adjusted_start > 0 && !content.is_char_boundary(adjusted_start) {
        adjusted_start -= 1;
    }

    while adjusted_end < content.len() && !content.is_char_boundary(adjusted_end) {
        adjusted_end += 1;
    }

    if adjusted_start >= adjusted_end {
        panic!(
            "safe_str_slice: adjusted bounds invalid - start ({}) >= end ({})",
            adjusted_start, adjusted_end
        );
    }

    &content[adjusted_start..adjusted_end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_str_slice_ascii() {
        let content = "hello world";
        assert_eq!(safe_str_slice(content, 0, 5), "hello");
        assert_eq!(safe_str_slice(content, 6, 11), "world");
    }

    #[test]
    fn test_safe_str_slice_multibyte_adjusted_to_boundary() {
        // \u{1F600} is 4 bytes: bytes 0..4
        let content = "\u{1F600}abc";
        // Slicing at byte 2 (mid-emoji) should adjust start back to 0
        let result = safe_str_slice(content, 0, 4);
        assert_eq!(result, "\u{1F600}");
    }

    #[test]
    fn test_safe_str_slice_end_mid_char_adjusts_forward() {
        // \u{00E9} (é) is 2 bytes
        let content = "caf\u{00E9}!";
        // byte 4 is mid-char for é (which spans bytes 3..5), end should adjust to 5
        let result = safe_str_slice(content, 0, 4);
        assert_eq!(result, "caf\u{00E9}");
    }

    #[test]
    fn test_safe_str_slice_start_equals_end() {
        let content = "hello";
        assert_eq!(safe_str_slice(content, 2, 2), "");
    }

    #[test]
    fn test_safe_str_slice_end_clamped_to_length() {
        let content = "short";
        assert_eq!(safe_str_slice(content, 0, 100), "short");
    }

    #[test]
    #[should_panic(expected = "start")]
    fn test_safe_str_slice_start_past_end_panics() {
        safe_str_slice("hello", 10, 15);
    }

    #[test]
    #[should_panic(expected = "start")]
    fn test_safe_str_slice_start_greater_than_end_panics() {
        safe_str_slice("hello", 3, 1);
    }
}
