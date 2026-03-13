#[inline]
fn rotl(x: u32, k: u32) -> u32 {
    (x << k) | (x >> (32 - k))
}

#[derive(Clone, Debug, Default)]
pub struct Xoshiro128Plus {
    s: [u32; 4],
}

impl Xoshiro128Plus {
    pub fn new(seed: u32) -> Self {
        let mut rng = Self::default();
        rng.s[0] = seed;
        rng.s[1] = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        rng.s[2] = seed ^ 0x9E3779B9;
        rng.s[3] = 1;
        rng
    }

    pub fn from_state(s: [u32; 4]) -> Self {
        Self { s }
    }

    pub fn next_u32(&mut self) -> u32 {
        let result = self.s[0].wrapping_add(self.s[3]);

        let t = self.s[1] << 9;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = rotl(self.s[3], 11);

        result
    }

    pub fn next_f32(&mut self) -> f32 {
        let result = self.s[0].wrapping_add(self.s[3]);

        let t = self.s[1] << 9;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = rotl(self.s[3], 11);

        (result & 0x7FFFFFFF) as f32 / 2147483648.0
    }

    pub fn next_range(&mut self, min: u32, max: u32) -> u32 {
        let range = max - min + 1;
        min + (self.next_f32() * range as f32).floor() as u32
    }

    pub fn next_u64(&mut self) -> u64 {
        let hi = self.next_u32() as u64;
        let lo = self.next_u32() as u64;
        (hi << 32) | lo
    }

    pub fn next_u64_range(&mut self, min_bits: u32, max_bits: u32) -> (u32, u32) {
        let _ = self.next_f32();

        let n = self.next_range(min_bits, max_bits);
        let k = n - 32;

        let high_min = 1u32 << (k - 1);
        let high_max = (1u32 << k) - 1;
        let high = self.next_range(high_min, high_max);
        let low = self.next_u32();

        (high, low)
    }

    pub fn seed_from_global_id(global_id: (u32, u32), seed: u32) -> Self {
        let hash = global_id.0 ^ (global_id.1 << 16) ^ seed;
        Self::new(hash)
    }

    pub fn state(&self) -> [u32; 4] {
        self.s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rng_basic() {
        let mut rng = Xoshiro128Plus::new(42);
        let a = rng.next_u32();
        let b = rng.next_u32();
        assert_ne!(a, b);
    }

    #[test]
    fn test_rng_f32_range() {
        let mut rng = Xoshiro128Plus::new(12345);
        for _ in 0..100 {
            let f = rng.next_f32();
            assert!(f >= 0.0 && f < 1.0);
        }
    }

    #[test]
    fn test_rng_range() {
        let mut rng = Xoshiro128Plus::new(999);
        for _ in 0..100 {
            let v = rng.next_range(10, 20);
            assert!(v >= 10 && v <= 20);
        }
    }

    #[test]
    fn test_rng_reproducible() {
        let mut rng1 = Xoshiro128Plus::new(42);
        let mut rng2 = Xoshiro128Plus::new(42);

        for _ in 0..10 {
            assert_eq!(rng1.next_u32(), rng2.next_u32());
        }
    }
}
