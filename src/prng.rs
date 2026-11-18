//! Deterministic seeded pseudo-random number generator (SplitMix64 + XorShift64*).

#[derive(Clone, Debug)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        let mut s = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        s = (s ^ (s >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        s = (s ^ (s >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        s ^= s >> 31;
        if s == 0 {
            s = 0xD1B5_4A32_D192_ED03;
        }
        Self { state: s }
    }

    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Uniform float in [0.0, 1.0).
    #[inline]
    pub fn next_f32(&mut self) -> f32 {
        let v = (self.next_u32() >> 8) as f32;
        v * (1.0 / 16_777_216.0)
    }

    /// Uniform float in [0.0, 1.0).
    #[inline]
    pub fn next_f64(&mut self) -> f64 {
        let v = (self.next_u64() >> 11) as f64;
        v * (1.0 / 9_007_199_254_740_992.0)
    }

    #[inline]
    pub fn range_f32(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.next_f32()
    }

    #[inline]
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        if lo >= hi {
            return lo;
        }
        let span = (i64::from(hi) - i64::from(lo) + 1) as u64;
        let offset = (self.next_u64() % span) as i64;
        (i64::from(lo) + offset) as i32
    }

    #[inline]
    pub fn range_usize(&mut self, upper_exclusive: usize) -> usize {
        if upper_exclusive <= 1 {
            return 0;
        }
        (self.next_u64() % (upper_exclusive as u64)) as usize
    }
}
