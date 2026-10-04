//! The real source of chance: a `SplitMix64` generator, seeded by the
//! caller (from the clock, in `main`), so the same seed always gives the
//! same draws.

use nova_sim::Chance;

/// `SplitMix64`'s increment, the golden ratio's fraction in 64 bits.
const GOLDEN: u64 = 0x9E37_79B9_7F4A_7C15;

/// A `SplitMix64` generator: each draw a well-mixed 64-bit number.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SplitMix(u64);

impl SplitMix {
    /// The generator starting from `seed`.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// The next draw.
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(GOLDEN);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

/// A `percent` % chance fires when a draw, taken modulo 100, is below it;
/// a draw below `n` is a draw taken modulo `n`.
impl Chance for SplitMix {
    fn fires(&mut self, percent: u8) -> bool {
        self.next_u64() % 100 < u64::from(percent)
    }

    fn below(&mut self, n: u32) -> u32 {
        u32::try_from(self.next_u64() % u64::from(n.max(1))).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_seed_gives_splitmix64s_sequence() {
        // `SplitMix64` from seed 0, as its reference implementation gives.
        let mut zero = SplitMix::new(0);
        assert_eq!(
            [zero.next_u64(), zero.next_u64(), zero.next_u64()],
            [
                0xE220_A839_7B1D_CDAF,
                0x6E78_9E6A_A1B9_65F4,
                0x06C4_5D18_8009_454F
            ]
        );
        let draws = |seed| {
            let mut mix = SplitMix::new(seed);
            (0..5).map(|_| mix.next_u64()).collect::<Vec<_>>()
        };
        assert_eq!(draws(42), draws(42));
        assert_ne!(draws(42), draws(43));
    }

    #[test]
    fn nought_percent_never_fires_and_a_hundred_always_does() {
        let mut mix = SplitMix::new(7);
        assert!((0..1000).all(|_| !mix.fires(0)));
        assert!((0..1000).all(|_| mix.fires(100)));
        assert!((0..1000).all(|_| mix.fires(255)), "more than certain");
    }

    #[test]
    fn a_chance_fires_about_as_often_as_it_says() {
        for (seed, percent) in [(1, 30_u8), (2, 35), (3, 75), (4, 1)] {
            let mut mix = SplitMix::new(seed);
            let fired = (0..10_000).filter(|_| mix.fires(percent)).count();
            let expected = usize::from(percent) * 100;
            assert!(fired.abs_diff(expected) < 300, "{percent}%: {fired}");
        }
    }

    #[test]
    fn a_draw_below_n_is_the_next_draw_modulo_n() {
        // Seed 0's draws, modulo 7 and modulo 500.
        let mut zero = SplitMix::new(0);
        assert_eq!(
            zero.below(7),
            u32::try_from(0xE220_A839_7B1D_CDAF_u64 % 7).unwrap()
        );
        assert_eq!(
            zero.below(500),
            u32::try_from(0x6E78_9E6A_A1B9_65F4_u64 % 500).unwrap()
        );
        assert_eq!(SplitMix::new(0).below(1), 0);
        assert_eq!(
            SplitMix::new(0).below(0),
            0,
            "never asked, but never divides by 0"
        );
    }

    #[test]
    fn draws_below_n_stay_in_range_and_spread_evenly() {
        let mut mix = SplitMix::new(11);
        let mut counts = [0_u32; 7];
        for _ in 0..7000 {
            let draw = mix.below(7);
            assert!(draw < 7, "{draw}");
            counts[draw as usize] += 1;
        }
        for count in counts {
            assert!(count.abs_diff(1000) < 120, "{counts:?}");
        }
        let mut wide = SplitMix::new(12);
        assert!((0..1000).all(|_| wide.below(u32::MAX) < u32::MAX));
    }

    #[test]
    fn a_draw_below_the_percent_fires_and_one_at_it_does_not() {
        // Seed 0's first draw is 0xE220A8397B1DCDAF, 35 modulo 100.
        assert_eq!(0xE220_A839_7B1D_CDAF_u64 % 100, 35);
        assert!(SplitMix::new(0).fires(36));
        assert!(!SplitMix::new(0).fires(35));
    }
}
