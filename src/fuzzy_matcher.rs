/// Fuzzy Matcher: Suggests similar names for typos using Levenshtein distance
///
/// This module provides fuzzy string matching to suggest corrections for
/// undefined variables, functions, types, etc.
use std::collections::HashMap;

/// Fuzzy matcher for suggesting similar names
pub struct FuzzyMatcher {
    /// Symbol table: maps symbol types to lists of known symbols
    symbols: HashMap<SymbolType, Vec<String>>,
}

/// Type of symbol for categorization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolType {
    Variable,
    Function,
    Type,
    Module,
    Field,
    Method,
}

impl FuzzyMatcher {
    /// Create a new fuzzy matcher
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
        }
    }

    /// Add a symbol to the matcher
    pub fn add_symbol(&mut self, symbol_type: SymbolType, name: String) {
        self.symbols.entry(symbol_type).or_default().push(name);
    }

    /// Add multiple symbols of the same type
    pub fn add_symbols(&mut self, symbol_type: SymbolType, names: Vec<String>) {
        let entry = self.symbols.entry(symbol_type).or_default();
        entry.extend(names);
    }

    /// Find the best match for a typo
    ///
    /// Returns the best matching symbol and its distance score.
    /// Lower distance = better match.
    pub fn find_best_match(&self, symbol_type: SymbolType, typo: &str) -> Option<(String, usize)> {
        let symbols = self.symbols.get(&symbol_type)?;

        let mut best_match: Option<(String, usize)> = None;

        for symbol in symbols {
            let distance = levenshtein_distance(typo, symbol);

            // Only consider matches within reasonable distance
            // (max 3 edits or 30% of the longer string length)
            let max_distance = std::cmp::min(3, std::cmp::max(typo.len(), symbol.len()) * 3 / 10);

            if distance <= max_distance {
                if let Some((_, best_distance)) = best_match {
                    if distance < best_distance {
                        best_match = Some((symbol.clone(), distance));
                    }
                } else {
                    best_match = Some((symbol.clone(), distance));
                }
            }
        }

        best_match
    }

    /// Find multiple suggestions (up to N best matches)
    pub fn find_suggestions(
        &self,
        symbol_type: SymbolType,
        typo: &str,
        max_suggestions: usize,
    ) -> Vec<(String, usize)> {
        let symbols = match self.symbols.get(&symbol_type) {
            Some(s) => s,
            None => return Vec::new(),
        };

        let mut matches: Vec<(String, usize)> = Vec::new();

        for symbol in symbols {
            let distance = levenshtein_distance(typo, symbol);

            // Only consider matches within reasonable distance
            let max_distance = std::cmp::min(3, std::cmp::max(typo.len(), symbol.len()) * 3 / 10);

            if distance <= max_distance {
                matches.push((symbol.clone(), distance));
            }
        }

        // Sort by distance (best matches first)
        matches.sort_by_key(|(_, distance)| *distance);

        // Return top N matches
        matches.truncate(max_suggestions);
        matches
    }

    /// Check if a symbol exists
    pub fn has_symbol(&self, symbol_type: SymbolType, name: &str) -> bool {
        self.symbols
            .get(&symbol_type)
            .map(|symbols| symbols.iter().any(|s| s == name))
            .unwrap_or(false)
    }

    /// Get all symbols of a given type
    pub fn get_symbols(&self, symbol_type: SymbolType) -> Vec<String> {
        self.symbols.get(&symbol_type).cloned().unwrap_or_default()
    }

    /// Clear all symbols
    pub fn clear(&mut self) {
        self.symbols.clear();
    }

    /// Get statistics
    pub fn stats(&self) -> FuzzyMatcherStats {
        let mut total_symbols = 0;
        let mut by_type: HashMap<SymbolType, usize> = HashMap::new();

        for (symbol_type, symbols) in &self.symbols {
            let count = symbols.len();
            total_symbols += count;
            by_type.insert(*symbol_type, count);
        }

        FuzzyMatcherStats {
            total_symbols,
            by_type,
        }
    }
}

impl Default for FuzzyMatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the fuzzy matcher
#[derive(Debug, Clone)]
pub struct FuzzyMatcherStats {
    pub total_symbols: usize,
    pub by_type: HashMap<SymbolType, usize>,
}

/// Calculate Levenshtein distance between two strings
///
/// This is the minimum number of single-character edits (insertions, deletions, or substitutions)
/// required to change one string into the other.
///
/// Time complexity: O(m * n) where m and n are the lengths of the strings
/// Space complexity: O(min(m, n))
pub fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let len1 = s1.len();
    let len2 = s2.len();

    // Early returns for edge cases
    if len1 == 0 {
        return len2;
    }
    if len2 == 0 {
        return len1;
    }
    if s1 == s2 {
        return 0;
    }

    // Use the shorter string for the columns to save memory
    let (s1, s2, len1, len2) = if len1 > len2 {
        (s2, s1, len2, len1)
    } else {
        (s1, s2, len1, len2)
    };

    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();

    // We only need two rows of the matrix at a time
    let mut prev_row: Vec<usize> = (0..=len1).collect();
    let mut curr_row: Vec<usize> = vec![0; len1 + 1];

    for i in 1..=len2 {
        curr_row[0] = i;

        for j in 1..=len1 {
            let cost = if s2_chars[i - 1] == s1_chars[j - 1] {
                0
            } else {
                1
            };

            curr_row[j] = std::cmp::min(
                std::cmp::min(
                    curr_row[j - 1] + 1, // insertion
                    prev_row[j] + 1,     // deletion
                ),
                prev_row[j - 1] + cost, // substitution
            );
        }

        std::mem::swap(&mut prev_row, &mut curr_row);
    }

    prev_row[len1]
}

/// Calculate similarity ratio between two strings (0.0 to 1.0)
///
/// 1.0 = identical strings
/// 0.0 = completely different
pub fn similarity_ratio(s1: &str, s2: &str) -> f64 {
    let max_len = std::cmp::max(s1.len(), s2.len());
    if max_len == 0 {
        return 1.0;
    }

    let distance = levenshtein_distance(s1, s2);
    1.0 - (distance as f64 / max_len as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("", ""), 0);
        assert_eq!(levenshtein_distance("abc", "abc"), 0);
        assert_eq!(levenshtein_distance("", "abc"), 3);
        assert_eq!(levenshtein_distance("abc", ""), 3);
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
        assert_eq!(levenshtein_distance("saturday", "sunday"), 3);
        assert_eq!(levenshtein_distance("abc", "def"), 3);
    }

    #[test]
    fn test_similarity_ratio() {
        assert_eq!(similarity_ratio("abc", "abc"), 1.0);
        assert!((similarity_ratio("kitten", "sitting") - 0.571).abs() < 0.01);
        assert_eq!(similarity_ratio("", ""), 1.0);
    }

    #[test]
    fn test_fuzzy_matcher_basic() {
        let mut matcher = FuzzyMatcher::new();
        matcher.add_symbol(SymbolType::Variable, "count".to_string());
        matcher.add_symbol(SymbolType::Variable, "counter".to_string());
        matcher.add_symbol(SymbolType::Variable, "index".to_string());

        // Test exact match
        assert!(matcher.has_symbol(SymbolType::Variable, "count"));

        // Test typo matching
        let result = matcher.find_best_match(SymbolType::Variable, "cont");
        assert!(result.is_some());
        let (suggestion, distance) = result.unwrap();
        assert_eq!(suggestion, "count");
        assert_eq!(distance, 1); // Edit distance from "cont" to "count"
    }

    #[test]
    fn test_fuzzy_matcher_suggestions() {
        let mut matcher = FuzzyMatcher::new();
        matcher.add_symbol(SymbolType::Function, "print".to_string());
        matcher.add_symbol(SymbolType::Function, "println".to_string());
        matcher.add_symbol(SymbolType::Function, "printf".to_string());

        let suggestions = matcher.find_suggestions(SymbolType::Function, "pring", 3);
        assert!(!suggestions.is_empty());
        assert_eq!(suggestions[0].0, "print"); // Best match
    }

    #[test]
    fn test_fuzzy_matcher_no_match() {
        let mut matcher = FuzzyMatcher::new();
        matcher.add_symbol(SymbolType::Variable, "foo".to_string());

        // Very different string should not match
        let result = matcher.find_best_match(SymbolType::Variable, "completely_different");
        assert!(result.is_none());
    }

    #[test]
    fn test_fuzzy_matcher_stats() {
        let mut matcher = FuzzyMatcher::new();
        matcher.add_symbol(SymbolType::Variable, "x".to_string());
        matcher.add_symbol(SymbolType::Variable, "y".to_string());
        matcher.add_symbol(SymbolType::Function, "foo".to_string());

        let stats = matcher.stats();
        assert_eq!(stats.total_symbols, 3);
        assert_eq!(*stats.by_type.get(&SymbolType::Variable).unwrap(), 2);
        assert_eq!(*stats.by_type.get(&SymbolType::Function).unwrap(), 1);
    }
}
