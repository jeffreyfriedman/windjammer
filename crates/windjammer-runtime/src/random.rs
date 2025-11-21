//! Random number generation
//!
//! Windjammer's `std::random` module maps to these functions.

use rand::Rng;

/// Generate random integer in range [min, max)
pub fn int_range(min: i64, max: i64) -> i64 {
    rand::thread_rng().gen_range(min..max)
}

/// Generate random float in range [0.0, 1.0)
pub fn float() -> f64 {
    rand::thread_rng().gen()
}

/// Generate random float in range [min, max)
pub fn float_range(min: f64, max: f64) -> f64 {
    rand::thread_rng().gen_range(min..max)
}

/// Generate random boolean
pub fn bool() -> bool {
    rand::thread_rng().gen()
}

/// Choose random element from slice
pub fn choice<T: Clone>(items: &[T]) -> Option<T> {
    use rand::seq::SliceRandom;
    items.choose(&mut rand::thread_rng()).cloned()
}

/// Shuffle a vector
pub fn shuffle<T>(items: &mut [T]) {
    use rand::seq::SliceRandom;
    items.shuffle(&mut rand::thread_rng());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_int_range() {
        for _ in 0..100 {
            let n = int_range(0, 10);
            assert!((0..10).contains(&n));
        }
    }

    #[test]
    fn test_float() {
        for _ in 0..100 {
            let f = float();
            assert!((0.0..1.0).contains(&f));
        }
    }

    #[test]
    fn test_bool() {
        let mut true_count = 0;
        for _ in 0..100 {
            if bool() {
                true_count += 1;
            }
        }
        // Should be roughly 50/50
        assert!(true_count > 20 && true_count < 80);
    }

    #[test]
    fn test_choice() {
        let items = vec![1, 2, 3, 4, 5];
        for _ in 0..10 {
            let chosen = choice(&items);
            assert!(chosen.is_some());
            assert!(items.contains(&chosen.unwrap()));
        }
    }
}
