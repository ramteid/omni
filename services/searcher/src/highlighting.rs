use shared::utils::safe_str_slice;
use std::collections::HashSet;
use tracing::debug;

#[derive(Debug, Clone)]
pub struct HighlightConfig {
    pub start_sel: String,
    pub stop_sel: String,
    pub max_words: usize,
    pub min_words: usize,
    pub max_fragments: usize,
    pub fragment_delimiter: String,
    pub include_line_numbers: bool,
}

impl Default for HighlightConfig {
    fn default() -> Self {
        Self {
            start_sel: "**".to_string(),
            stop_sel: "**".to_string(),
            max_words: 20,
            min_words: 10,
            max_fragments: 3,
            fragment_delimiter: "...".to_string(),
            include_line_numbers: false,
        }
    }
}

#[derive(Debug, Clone)]
struct Match {
    start: usize,
    end: usize,
    term_index: usize,
}

#[derive(Debug, Clone)]
struct Fragment {
    start: usize,
    end: usize,
    match_count: usize,
    score: f32,
}

pub fn generate_highlights(content: &str, query: &str, config: &HighlightConfig) -> String {
    if content.is_empty() || query.trim().is_empty() {
        debug!(
            "Empty content or query, returning empty highlights. Content len: {}, query len: {}",
            content.len(),
            query.len()
        );
        return String::new();
    }

    let query_terms = parse_query(query);
    if query_terms.is_empty() {
        debug!(
            "No valid query terms after parsing. Original query: {}",
            query
        );
        return String::new();
    }

    let matches = find_matches(content, &query_terms);
    if matches.is_empty() {
        debug!("No matches found for query: {}", query);
        return String::new();
    }

    debug!("Building fragments for {} matches", matches.len());
    let fragments = build_fragments(content, &matches, config);
    debug!(
        "Built {} fragments from {} matches",
        fragments.len(),
        matches.len()
    );
    format_fragments(content, &fragments, &matches, config)
}

fn parse_query(query: &str) -> Vec<String> {
    query
        .split_whitespace()
        .map(|term| normalize_term(term))
        .filter(|term| !term.is_empty() && term.len() >= 2)
        .collect()
}

fn normalize_term(term: &str) -> String {
    term.to_lowercase()
        .trim_matches(|c: char| !c.is_alphanumeric())
        .to_string()
}

fn find_matches(content: &str, query_terms: &[String]) -> Vec<Match> {
    let content_lower = content.to_lowercase();
    let mut matches = Vec::new();

    for (term_index, term) in query_terms.iter().enumerate() {
        let mut start_pos = 0;
        while let Some(pos) = content_lower[start_pos..].find(term) {
            let abs_pos = start_pos + pos;

            let is_word_boundary = (abs_pos == 0
                || !content_lower.as_bytes()[abs_pos - 1].is_ascii_alphanumeric())
                && (abs_pos + term.len() >= content_lower.len()
                    || !content_lower.as_bytes()[abs_pos + term.len()].is_ascii_alphanumeric());

            if is_word_boundary {
                matches.push(Match {
                    start: abs_pos,
                    end: abs_pos + term.len(),
                    term_index,
                });
            }

            start_pos = abs_pos + 1;
        }
    }

    matches.sort_by_key(|m| m.start);
    matches
}

fn build_fragments(content: &str, matches: &[Match], config: &HighlightConfig) -> Vec<Fragment> {
    if matches.is_empty() {
        return Vec::new();
    }

    let avg_word_len = 6;
    let context_chars = (config.max_words / 2) * avg_word_len;

    let mut fragments = Vec::new();
    let mut covered_ranges: Vec<(usize, usize)> = Vec::new();

    for match_item in matches {
        let match_center = (match_item.start + match_item.end) / 2;

        let mut frag_start = match_center.saturating_sub(context_chars);
        let mut frag_end = (match_center + context_chars).min(content.len());

        frag_start = find_word_boundary_backward(content, frag_start);
        frag_end = find_word_boundary_forward(content, frag_end);

        if is_overlapping(&covered_ranges, frag_start, frag_end) {
            continue;
        }

        let matches_in_fragment: Vec<&Match> = matches
            .iter()
            .filter(|m| m.start >= frag_start && m.end <= frag_end)
            .collect();

        let unique_terms: HashSet<_> = matches_in_fragment.iter().map(|m| m.term_index).collect();

        let score = unique_terms.len() as f32 * 2.0 + matches_in_fragment.len() as f32;

        fragments.push(Fragment {
            start: frag_start,
            end: frag_end,
            match_count: matches_in_fragment.len(),
            score,
        });

        covered_ranges.push((frag_start, frag_end));
    }

    fragments.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    fragments.truncate(config.max_fragments);
    fragments.sort_by_key(|f| f.start);

    fragments
}

fn is_overlapping(ranges: &[(usize, usize)], start: usize, end: usize) -> bool {
    ranges
        .iter()
        .any(|(r_start, r_end)| !(end <= *r_start || start >= *r_end))
}

fn find_word_boundary_backward(content: &str, pos: usize) -> usize {
    if pos == 0 {
        return 0;
    }

    let search_start = pos.saturating_sub(50);
    let slice = safe_str_slice(content, search_start, pos);

    if let Some(last_space) = slice.rfind(|c: char| c.is_whitespace()) {
        search_start + last_space + 1
    } else {
        search_start
    }
}

fn find_word_boundary_forward(content: &str, pos: usize) -> usize {
    if pos >= content.len() {
        return content.len();
    }

    let search_end = (pos + 50).min(content.len());
    let slice = safe_str_slice(content, pos, search_end);

    if let Some(first_space) = slice.find(|c: char| c.is_whitespace()) {
        pos + first_space
    } else {
        search_end
    }
}

fn build_line_offset_map(content: &str) -> Vec<usize> {
    let mut line_starts = vec![0];
    for (i, byte) in content.bytes().enumerate() {
        if byte == b'\n' {
            line_starts.push(i + 1);
        }
    }
    line_starts
}

fn get_line_number(byte_pos: usize, line_map: &[usize]) -> usize {
    line_map
        .binary_search(&byte_pos)
        .unwrap_or_else(|insert_pos| insert_pos)
}

fn format_fragments(
    content: &str,
    fragments: &[Fragment],
    matches: &[Match],
    config: &HighlightConfig,
) -> String {
    debug!("Formatting {} fragments", fragments.len());
    let mut result = String::new();

    let line_map = if config.include_line_numbers {
        Some(build_line_offset_map(content))
    } else {
        None
    };

    for (i, fragment) in fragments.iter().enumerate() {
        if i > 0 {
            result.push('\n');
            result.push_str(&config.fragment_delimiter);
            result.push('\n');
        }

        if fragment.start > 0 && i == 0 {
            result.push_str(&config.fragment_delimiter);
            result.push('\n');
        }

        let fragment_text = safe_str_slice(content, fragment.start, fragment.end);
        let matches_in_fragment: Vec<&Match> = matches
            .iter()
            .filter(|m| m.start >= fragment.start && m.end <= fragment.end)
            .collect();

        // Build highlighted text first
        let mut highlighted = String::new();
        let mut last_pos = 0;
        for match_item in matches_in_fragment {
            let match_start = match_item.start - fragment.start;
            let match_end = match_item.end - fragment.start;

            // Skip overlapping matches (can happen with multiple query terms)
            if match_start < last_pos {
                continue;
            }

            highlighted.push_str(safe_str_slice(fragment_text, last_pos, match_start));
            highlighted.push_str(&config.start_sel);
            highlighted.push_str(safe_str_slice(fragment_text, match_start, match_end));
            highlighted.push_str(&config.stop_sel);

            last_pos = match_end;
        }
        highlighted.push_str(&fragment_text[last_pos..]);

        // Add line numbers if configured
        if let Some(ref map) = line_map {
            let start_line = get_line_number(fragment.start, map) + 1;
            let mut current_line = start_line;

            for line in highlighted.lines() {
                if current_line > start_line {
                    result.push('\n');
                }
                result.push_str(&format!("{} | {}", current_line, line));
                current_line += 1;
            }
        } else {
            result.push_str(&highlighted);
        }

        if fragment.end < content.len() {
            result.push('\n');
            result.push_str(&config.fragment_delimiter);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_query() {
        let terms = parse_query("hello world");
        assert_eq!(terms, vec!["hello", "world"]);

        let terms = parse_query("  Hello   World  ");
        assert_eq!(terms, vec!["hello", "world"]);

        let terms = parse_query("hello, world!");
        assert_eq!(terms, vec!["hello", "world"]);
    }

    #[test]
    fn test_normalize_term() {
        assert_eq!(normalize_term("Hello"), "hello");
        assert_eq!(normalize_term("world!"), "world");
        assert_eq!(normalize_term("(test)"), "test");
    }

    #[test]
    fn test_generate_highlights_empty() {
        let config = HighlightConfig::default();
        assert_eq!(generate_highlights("", "query", &config), "");
        assert_eq!(generate_highlights("content", "", &config), "");
    }

    #[test]
    fn test_generate_highlights_simple() {
        let config = HighlightConfig::default();
        let content = "The quick brown fox jumps over the lazy dog";
        let result = generate_highlights(content, "fox", &config);
        assert!(result.contains("**fox**"));
    }

    #[test]
    fn test_generate_highlights_multiple_matches() {
        let config = HighlightConfig::default();
        let content = "The fox and the fox are friends. The fox is happy.";
        let result = generate_highlights(content, "fox", &config);
        assert!(result.contains("**fox**"));
    }

    #[test]
    fn test_generate_highlights_case_insensitive() {
        let config = HighlightConfig::default();
        let content = "The Fox and the FOX are friends";
        let result = generate_highlights(content, "fox", &config);
        assert!(result.contains("**Fox**"));
        assert!(result.contains("**FOX**"));
    }

    #[test]
    fn test_generate_highlights_no_match() {
        let config = HighlightConfig::default();
        let content = "The quick brown fox";
        let result = generate_highlights(content, "elephant", &config);
        assert_eq!(result, "");
    }

    #[test]
    fn test_generate_highlights_word_boundary() {
        let config = HighlightConfig::default();
        let content = "testing tests test tested";
        let result = generate_highlights(content, "test", &config);
        assert!(result.contains("**test**"));
        assert!(!result.contains("**testing**"));
    }

    #[test]
    fn test_generate_highlights_multiple_terms() {
        let config = HighlightConfig::default();
        let content = "The quick brown fox jumps over the lazy dog";
        let result = generate_highlights(content, "fox dog", &config);
        assert!(result.contains("**fox**"));
        assert!(result.contains("**dog**"));
    }

    #[test]
    fn test_generate_highlights_with_line_numbers() {
        let mut config = HighlightConfig::default();
        config.include_line_numbers = true;
        config.max_words = 5; // Small window to create separate fragments
        config.max_fragments = 2;
        let content = "Line one content here with extra words.\nLine two has a fox here with more words.\nLine three is here with content.\nLine four more text.\nLine five has another fox in it with text.";
        let result = generate_highlights(content, "fox", &config);

        // Verify exact format with line numbers and highlights
        assert!(result.contains("2 | Line two has a **fox** here with"));

        // Second fragment may be on a different line depending on word boundaries
        // Just verify it contains line number prefix and fox highlight
        assert!(result.contains("**fox**"));
        assert!(result.matches("| ").count() >= 2); // At least 2 lines with line numbers

        // Verify delimiters with newlines around fragments
        assert!(result.starts_with("...\n"));
        assert!(result.contains("\n...\n"));
        assert!(result.ends_with("\n..."));
    }

    #[test]
    fn test_generate_highlights_multiline_fragment_with_line_numbers() {
        let mut config = HighlightConfig::default();
        config.include_line_numbers = true;
        config.max_words = 100;
        config.max_fragments = 1;

        let content = "First line with some content here\nSecond line with test keyword\nThird line also has test keyword\nFourth line with more content";
        let result = generate_highlights(content, "test", &config);

        // Should have exact line numbers and highlights for the fragment containing both matches
        assert!(result.contains("2 | Second line with **test** keyword"));
        assert!(result.contains("3 | Third line also has **test** keyword"));

        // Fragment should have line breaks between lines
        assert!(result.contains("\n"));
    }

    #[test]
    fn test_fragment_delimiter_with_newlines() {
        let mut config = HighlightConfig::default();
        config.max_words = 10;
        config.max_fragments = 3;
        let content = "Start content here with lots of words. Middle section has fox in it with more words around. End section has another fox reference with more text. Final content at the end.";
        let result = generate_highlights(content, "fox", &config);

        // Should start with delimiter and newline if first fragment doesn't start at position 0
        if result.starts_with("...") {
            assert!(result.starts_with("...\n"));
        }

        // Should have newlines around delimiters between fragments
        if result.matches("...").count() > 2 {
            assert!(result.contains("\n...\n"));
        }

        // Should end with newline and delimiter if last fragment doesn't reach the end
        if result.ends_with("...") {
            assert!(result.ends_with("\n..."));
        }
    }
}
