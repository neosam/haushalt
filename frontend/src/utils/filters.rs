//! Utilities for filtering tasks by text.

/// Checks if a task title matches all words in the filter text.
///
/// # Arguments
/// * `title` - The task title to check
/// * `filter_text` - The filter string (case-insensitive, space-separated words)
///
/// # Returns
/// * `true` if all words in `filter_text` appear in `title` (case-insensitive)
/// * `true` if `filter_text` is empty (no filter applied)
/// * `false` otherwise
///
/// # Example
/// ```
/// use frontend::utils::matches_text_filter;
/// assert!(matches_text_filter("Clean Kitchen", "clean"));
/// assert!(matches_text_filter("Clean Kitchen", "clean kitchen"));
/// assert!(matches_text_filter("Kitchen Cleaning", "kitchen clean"));
/// assert!(!matches_text_filter("Bedroom Cleaning", "kitchen"));
/// assert!(matches_text_filter("Any Task", "")); // Empty filter matches all
/// ```
pub fn matches_text_filter(title: &str, filter_text: &str) -> bool {
    if filter_text.is_empty() {
        return true;
    }

    let title_lower = title.to_lowercase();
    let filter_lower = filter_text.to_lowercase();
    let words: Vec<&str> = filter_lower.split_whitespace().collect();

    words.iter().all(|word| title_lower.contains(word))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_filter_matches_all() {
        assert!(matches_text_filter("Any Task", ""));
    }

    #[test]
    fn test_single_word_match() {
        assert!(matches_text_filter("Clean Kitchen", "clean"));
        assert!(matches_text_filter("Clean Kitchen", "kitchen"));
    }

    #[test]
    fn test_single_word_no_match() {
        assert!(!matches_text_filter("Clean Kitchen", "bedroom"));
    }

    #[test]
    fn test_multiple_words_match() {
        assert!(matches_text_filter("Clean Kitchen", "clean kitchen"));
        assert!(matches_text_filter("Kitchen Cleaning", "kitchen clean"));
    }

    #[test]
    fn test_multiple_words_no_match() {
        assert!(!matches_text_filter("Clean Kitchen", "bedroom clean"));
    }

    #[test]
    fn test_case_insensitive() {
        assert!(matches_text_filter("Clean Kitchen", "CLEAN"));
        assert!(matches_text_filter("clean kitchen", "Clean"));
        assert!(matches_text_filter("ClEaN KiTcHeN", "cLeAn KiTcHeN"));
    }

    #[test]
    fn test_word_order_independent() {
        assert!(matches_text_filter("Clean Kitchen", "kitchen clean"));
        assert!(matches_text_filter("Clean Kitchen", "clean kitchen"));
    }

    #[test]
    fn test_partial_word_match() {
        assert!(matches_text_filter("Cleaning", "clean"));
        assert!(matches_text_filter("Kitchen", "kit"));
    }

    #[test]
    fn test_whitespace_handling() {
        assert!(matches_text_filter("Clean Kitchen", "  clean   kitchen  "));
    }
}
