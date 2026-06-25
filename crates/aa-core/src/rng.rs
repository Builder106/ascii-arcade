//! A small, deterministic PRNG (SplitMix64).
//!
//! Ported from `SceneSetting.swift`'s `SeededGenerator`. Stateful scenes (Matrix
//! rain, fire, Life, pipes) seed from this so a fixed "Seed" setting reproduces
//! the same animation and unit tests stay stable across platforms.

pub struct SeededRng {
    state: u64,
}

impl SeededRng {
    pub fn new(seed: u64) -> Self {
        SeededRng {
            state: if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed },
        }
    }

    /// Next raw 64-bit value.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `0.0..1.0`.
    pub fn next_f64(&mut self) -> f64 {
        // 53-bit mantissa for a uniform double.
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniform integer in `0..n` (n must be > 0).
    pub fn next_below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }

    /// Uniform integer in `lo..=hi` (inclusive), mirroring Swift's
    /// `Int.random(in: lo...hi)`. `hi` must be `>= lo`.
    pub fn next_range_inclusive(&mut self, lo: i64, hi: i64) -> i64 {
        debug_assert!(hi >= lo);
        let span = (hi - lo) as u64 + 1;
        lo + (self.next_u64() % span) as i64
    }

    /// Uniform float in `lo..=hi`, mirroring Swift's `Double.random(in: lo...hi)`.
    pub fn next_range_f64(&mut self, lo: f64, hi: f64) -> f64 {
        lo + self.next_f64() * (hi - lo)
    }

    /// A coin flip, mirroring Swift's `Bool.random()`.
    pub fn next_bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    /// Pick a uniformly-random element from a non-empty slice, mirroring
    /// `Array.randomElement(using:)`.
    pub fn choose<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.next_below(items.len())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_for_same_seed() {
        let mut a = SeededRng::new(42);
        let mut b = SeededRng::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn f64_in_unit_range() {
        let mut r = SeededRng::new(7);
        for _ in 0..1000 {
            let v = r.next_f64();
            assert!((0.0..1.0).contains(&v));
        }
    }
}
