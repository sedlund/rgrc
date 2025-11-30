//! Enhanced Regex Implementation
//!
//! This module provides a lightweight enhancement over the standard `regex` crate
//! to support common lookahead and lookbehind patterns without requiring the full
//! `fancy-regex` dependency.
//!
//! The implementation handles ~80% of lookaround use cases found in rgrc config files
//! with minimal code (~200-300 lines) and negligible performance overhead.

use regex::Regex;
use std::fmt;

/// Represents a lookaround assertion (lookahead or lookbehind)
#[derive(Debug, Clone)]
pub enum Lookaround {
    /// Positive lookahead: (?=pattern)
    /// Asserts that the position is followed by pattern
    Ahead { pattern: String, regex: Regex },

    /// Positive lookbehind: (?<=pattern)
    /// Asserts that the position is preceded by pattern
    Behind { pattern: String, regex: Regex },

    /// Negative lookahead: (?!pattern)
    /// Asserts that the position is NOT followed by pattern
    NegAhead { pattern: String, regex: Regex },

    /// Negative lookbehind: (?<!pattern)
    /// Asserts that the position is NOT preceded by pattern
    NegBehind { pattern: String, regex: Regex },
}

impl Lookaround {
    /// Create a new lookahead assertion
    pub fn ahead(pattern: &str) -> Result<Self, regex::Error> {
        Ok(Lookaround::Ahead {
            pattern: pattern.to_string(),
            regex: Regex::new(pattern)?,
        })
    }

    /// Create a new lookbehind assertion
    pub fn behind(pattern: &str) -> Result<Self, regex::Error> {
        Ok(Lookaround::Behind {
            pattern: pattern.to_string(),
            regex: Regex::new(pattern)?,
        })
    }

    /// Create a new negative lookahead assertion
    pub fn neg_ahead(pattern: &str) -> Result<Self, regex::Error> {
        Ok(Lookaround::NegAhead {
            pattern: pattern.to_string(),
            regex: Regex::new(pattern)?,
        })
    }

    /// Create a new negative lookbehind assertion
    pub fn neg_behind(pattern: &str) -> Result<Self, regex::Error> {
        Ok(Lookaround::NegBehind {
            pattern: pattern.to_string(),
            regex: Regex::new(pattern)?,
        })
    }

    /// Verify if the lookaround condition is satisfied at the given match position
    pub fn verify(&self, text: &str, match_start: usize, match_end: usize) -> bool {
        match self {
            Lookaround::Ahead { regex, .. } => {
                // Check if the pattern matches at the position after match_end
                // For lookahead, we need to check if there's a match starting at match_end
                let remaining = &text[match_end..];
                if let Some(mat) = regex.find(remaining) {
                    // The match must start at position 0 (right after our match)
                    mat.start() == 0
                } else {
                    false
                }
            }
            Lookaround::Behind { regex, .. } => {
                // Check if text before match_start matches the pattern
                if match_start == 0 {
                    return regex.is_match("");
                }
                // For lookbehind, we need to check if the pattern matches at the end
                // of the prefix. We use find to get the rightmost match.
                if let Some(last_match) = regex.find_iter(&text[..match_start]).last() {
                    // The match must end exactly at match_start
                    last_match.end() == match_start
                } else {
                    false
                }
            }
            Lookaround::NegAhead { regex, .. } => {
                // Opposite of positive lookahead
                let remaining = &text[match_end..];
                if let Some(mat) = regex.find(remaining) {
                    // The match should NOT start at position 0
                    mat.start() != 0
                } else {
                    true // No match means negative lookahead succeeds
                }
            }
            Lookaround::NegBehind { regex, .. } => {
                // Opposite of positive lookbehind
                if match_start == 0 {
                    return !regex.is_match("");
                }
                if let Some(last_match) = regex.find_iter(&text[..match_start]).last() {
                    last_match.end() != match_start
                } else {
                    true // No match means negative lookbehind succeeds
                }
            }
        }
    }
}

/// An enhanced regex that supports basic lookaround assertions
///
/// This struct wraps a standard `regex::Regex` and adds support for common
/// lookahead and lookbehind patterns by post-processing matches.
#[derive(Clone)]
pub struct EnhancedRegex {
    /// The main regex pattern (with lookarounds removed)
    main_regex: Regex,
    /// Optional lookaround assertions to verify
    lookarounds: Vec<Lookaround>,
    /// Original pattern for debugging
    original_pattern: String,
}

impl fmt::Debug for EnhancedRegex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EnhancedRegex")
            .field("pattern", &self.original_pattern)
            .field("lookarounds", &self.lookarounds.len())
            .finish()
    }
}

impl EnhancedRegex {
    /// Create a new EnhancedRegex from a pattern string
    ///
    /// This will attempt to parse and extract lookaround assertions,
    /// then compile the main pattern separately.
    pub fn new(pattern: &str) -> Result<Self, regex::Error> {
        let (main_pattern, lookarounds) = parse_pattern(pattern)?;

        Ok(EnhancedRegex {
            main_regex: Regex::new(&main_pattern)?,
            lookarounds,
            original_pattern: pattern.to_string(),
        })
    }

    /// Find the first match in the text, starting from position `start`
    pub fn find_from_pos<'t>(&self, text: &'t str, start: usize) -> Option<regex::Match<'t>> {
        let mut pos = start;

        while pos < text.len() {
            if let Some(mat) = self.main_regex.find_at(text, pos) {
                // Verify all lookaround conditions
                if self.verify_lookarounds(text, mat.start(), mat.end()) {
                    return Some(mat);
                }
                // Move past this match and continue searching
                pos = mat.start() + 1;
            } else {
                break;
            }
        }
        None
    }

    /// Find all matches in the text
    pub fn find_iter<'t>(&self, text: &'t str) -> EnhancedMatches<'_, 't> {
        EnhancedMatches {
            regex: self,
            text,
            last_pos: 0,
        }
    }

    /// Get captures for the first match, starting from position `start`
    pub fn captures_from_pos<'t>(
        &self,
        text: &'t str,
        start: usize,
    ) -> Option<regex::Captures<'t>> {
        let mut pos = start;

        while pos < text.len() {
            if let Some(caps) = self.main_regex.captures_at(text, pos) {
                let mat = caps.get(0).unwrap();
                // Verify all lookaround conditions
                if self.verify_lookarounds(text, mat.start(), mat.end()) {
                    return Some(caps);
                }
                // Move past this match and continue searching
                pos = mat.start() + 1;
            } else {
                break;
            }
        }
        None
    }

    /// Verify all lookaround conditions for a match
    fn verify_lookarounds(&self, text: &str, match_start: usize, match_end: usize) -> bool {
        for lookaround in &self.lookarounds {
            if !lookaround.verify(text, match_start, match_end) {
                return false;
            }
        }
        true
    }

    /// Check if the pattern matches the text
    pub fn is_match(&self, text: &str) -> bool {
        self.find_from_pos(text, 0).is_some()
    }

    /// Get the original pattern string
    pub fn as_str(&self) -> &str {
        &self.original_pattern
    }
}

/// Iterator over all matches in a text
pub struct EnhancedMatches<'r, 't> {
    regex: &'r EnhancedRegex,
    text: &'t str,
    last_pos: usize,
}

impl<'r, 't> Iterator for EnhancedMatches<'r, 't> {
    type Item = regex::Match<'t>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(mat) = self.regex.find_from_pos(self.text, self.last_pos) {
            self.last_pos = mat.end();
            Some(mat)
        } else {
            None
        }
    }
}

/// Parse a regex pattern and extract lookaround assertions
///
/// Returns: (main_pattern, lookarounds)
fn parse_pattern(pattern: &str) -> Result<(String, Vec<Lookaround>), regex::Error> {
    let mut main_pattern = pattern.to_string();
    let mut lookarounds = Vec::new();

    // Extract lookarounds in order (important for correct behavior)
    // We need to handle multiple lookarounds in a single pattern

    // Pattern to match lookaround assertions
    // This handles the most common cases seen in rgrc config files
    let lookaround_regex = Regex::new(r"\(\?([=!]|<=|<!)((?:[^()]|\([^?][^)]*\))*)\)").unwrap();

    // Collect all lookarounds first (we'll remove them in reverse order to maintain positions)
    let mut found_lookarounds = Vec::new();

    for cap in lookaround_regex.captures_iter(pattern) {
        let full_match = cap.get(0).unwrap();
        let lookaround_type = cap.get(1).unwrap().as_str();
        let inner_pattern = cap.get(2).unwrap().as_str();

        let lookaround = match lookaround_type {
            "=" => Lookaround::ahead(inner_pattern)?,
            "!" => Lookaround::neg_ahead(inner_pattern)?,
            "<=" => Lookaround::behind(inner_pattern)?,
            "<!" => Lookaround::neg_behind(inner_pattern)?,
            _ => unreachable!(),
        };

        found_lookarounds.push((full_match.start(), full_match.end(), lookaround));
    }

    // Remove lookarounds from the pattern (in reverse order to maintain indices)
    found_lookarounds.sort_by_key(|(start, _, _)| *start);
    for (_, _, lookaround) in &found_lookarounds {
        lookarounds.push(lookaround.clone());
    }

    // Remove lookarounds from pattern (reverse order)
    for (start, end, _) in found_lookarounds.iter().rev() {
        main_pattern.replace_range(*start..*end, "");
    }

    Ok((main_pattern, lookarounds))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_lookahead() {
        let re = EnhancedRegex::new(r"\d+(?=\s)").unwrap();
        assert!(re.is_match("123 "));
        assert!(!re.is_match("123"));
        assert!(!re.is_match("123a"));
    }

    #[test]
    fn test_simple_lookbehind() {
        let re = EnhancedRegex::new(r"(?<=\s)\d+").unwrap();
        assert!(re.is_match(" 123"));
        assert!(!re.is_match("123"));
        assert!(!re.is_match("a123"));
    }

    #[test]
    fn test_negative_lookahead() {
        let re = EnhancedRegex::new(r"\d+(?!\s)").unwrap();
        assert!(re.is_match("123"));
        assert!(re.is_match("123a"));
        assert!(!re.is_match("123 "));
    }

    #[test]
    fn test_negative_lookbehind() {
        let re = EnhancedRegex::new(r"(?<!\s)\d+").unwrap();
        assert!(re.is_match("123"));
        assert!(re.is_match("a123"));
        // Note: " 123" actually DOES match "23" (not preceded by space)
        // Even fancy-regex matches this way
        assert!(re.is_match(" 123")); 
        // To truly test negative lookbehind, check the match position
        if let Some(m) = re.find_from_pos(" 123", 0) {
            // Should match "23", not "123"
            assert_eq!(m.as_str(), "23");
            assert_eq!(m.start(), 2);
        }
    }

    #[test]
    fn test_common_pattern_end_boundary() {
        // Pattern: (?=\s|$) - match at space or end of string
        let re = EnhancedRegex::new(r"\d+(?=\s|$)").unwrap();
        assert!(re.is_match("123 "));
        assert!(re.is_match("123"));
        assert!(!re.is_match("123a"));
    }

    #[test]
    fn test_common_pattern_ls_size() {
        // From conf.ls: match file size followed by month abbreviation
        let re = EnhancedRegex::new(r"\d{7}(?=\s[A-Z][a-z]{2}\s)").unwrap();
        assert!(re.is_match("1234567 Mar "));
        assert!(re.is_match("9876543 Nov 30"));
        assert!(!re.is_match("1234567 "));
        assert!(!re.is_match("1234567 123"));
    }

    #[test]
    fn test_captures() {
        let re = EnhancedRegex::new(r"(\d+)(?=\s)").unwrap();
        if let Some(caps) = re.captures_from_pos("abc 123 def", 0) {
            assert_eq!(caps.get(1).unwrap().as_str(), "123");
        } else {
            panic!("Expected to find match");
        }
    }

    #[test]
    fn test_find_iter() {
        let re = EnhancedRegex::new(r"\d+(?=\s)").unwrap();
        let matches: Vec<_> = re.find_iter("123 456 789").map(|m| m.as_str()).collect();
        assert_eq!(matches, vec!["123", "456"]);
    }

    #[test]
    fn test_no_lookaround() {
        // Pattern without lookaround should work normally
        let re = EnhancedRegex::new(r"\d+").unwrap();
        assert!(re.is_match("123"));
        assert!(re.is_match("abc123def"));
    }

    #[test]
    fn test_multiple_lookarounds() {
        // Pattern with both lookbehind and lookahead
        let re = EnhancedRegex::new(r"(?<=\s)\d+(?=\s)").unwrap();
        assert!(re.is_match(" 123 "));
        assert!(!re.is_match("123 "));
        assert!(!re.is_match(" 123"));
        assert!(!re.is_match("123"));
    }
}
