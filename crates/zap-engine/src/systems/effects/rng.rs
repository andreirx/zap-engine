//! Seedable pseudo-random number generator (xorshift64).
//! Deterministic, fast, no-std compatible.

/// Seedable pseudo-random number generator (xorshift64).
/// Deterministic, fast, no-std compatible.
#[derive(Debug, Clone)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng {
            state: if seed == 0 { 1 } else { seed },
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// Generate a random number in [0, upper_bound).
    pub fn next_int(&mut self, upper_bound: u32) -> u32 {
        (self.next_u64() % upper_bound as u64) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rng_deterministic() {
        let mut rng1 = Rng::new(42);
        let mut rng2 = Rng::new(42);
        for _ in 0..10 {
            assert_eq!(rng1.next_int(1000), rng2.next_int(1000));
        }
    }

    #[test]
    fn rng_zero_seed_handled() {
        let mut rng = Rng::new(0);
        // Should not panic or loop forever
        let _ = rng.next_int(100);
    }
}
