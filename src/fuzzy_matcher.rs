//! Levenshtein distance, used to suggest corrections for misspelled names
//! (undefined variables, functions, types, methods).

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
}
