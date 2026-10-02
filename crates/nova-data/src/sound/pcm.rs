//! Decoded sounds: 16-bit linear PCM.

/// A sample rate exactly as a sound header stores it: an unsigned 16.16
/// fixed-point number of samples per second.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SampleRate(u32);

impl SampleRate {
    /// Wraps a 16.16 fixed-point rate.
    pub(crate) fn from_fixed(fixed: u32) -> Self {
        Self(fixed)
    }

    /// The raw 16.16 fixed-point value.
    #[must_use]
    pub fn fixed(self) -> u32 {
        self.0
    }

    /// Samples per second, exactly.
    #[must_use]
    pub fn hz(self) -> f64 {
        f64::from(self.0) / 65_536.0
    }

    /// Samples per second, rounded to the nearest whole number (halves up).
    #[must_use]
    pub fn nearest_hz(self) -> u32 {
        ((u64::from(self.0) + 0x8000) >> 16) as u32
    }
}

/// A decoded sound: signed 16-bit linear PCM samples, interleaved by frame
/// (left, right, left, ... for stereo).
///
/// Every source format maps to `i16`:
///
/// - 8-bit samples are unsigned offset binary with silence at `0x80`; each
///   becomes `(byte - 128) << 8`, so `0x00` is -32768, `0x80` is 0 and
///   `0xFF` is 32512.
/// - 16-bit samples are big-endian two's complement and are kept as they
///   are.
/// - IMA4 decodes directly to 16-bit samples.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pcm {
    sample_rate: SampleRate,
    channels: u16,
    samples: Vec<i16>,
    loop_points: (u32, u32),
    base_note: u8,
}

impl Pcm {
    /// Wraps decoded samples. Callers pass 1 or 2 channels and a whole
    /// number of frames.
    pub(crate) fn new(
        sample_rate: SampleRate,
        channels: u16,
        samples: Vec<i16>,
        loop_points: (u32, u32),
        base_note: u8,
    ) -> Self {
        Self {
            sample_rate,
            channels,
            samples,
            loop_points,
            base_note,
        }
    }

    /// The header's sample rate.
    #[must_use]
    pub fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    /// The number of channels: 1 (mono) or 2 (stereo).
    #[must_use]
    pub fn channels(&self) -> u16 {
        self.channels
    }

    /// Every sample, interleaved by frame.
    #[must_use]
    pub fn samples(&self) -> &[i16] {
        &self.samples
    }

    /// The number of frames: samples per channel.
    #[must_use]
    pub fn frames(&self) -> usize {
        self.samples.len() / usize::from(self.channels)
    }

    /// The header's loop start and end, exactly as stored, or `None` when
    /// both are 0. They are not checked against the sample count: stock
    /// sounds have loop points past their end.
    #[must_use]
    pub fn loop_points(&self) -> Option<(u32, u32)> {
        (self.loop_points != (0, 0)).then_some(self.loop_points)
    }

    /// The header's `baseFrequency`: the MIDI note the sound plays at its
    /// own rate (60 is middle C).
    #[must_use]
    pub fn base_note(&self) -> u8 {
        self.base_note
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rate_keeps_its_fixed_value_and_converts_to_hertz() {
        let rate = SampleRate::from_fixed(0x2B77_45D1);
        assert_eq!(rate.fixed(), 0x2B77_45D1);
        assert!((rate.hz() - 11_127.272_720_336_914).abs() < 1e-9);
        assert!((SampleRate::from_fixed(0x5622_0000).hz() - 22_050.0).abs() < f64::EPSILON);
        assert!((SampleRate::from_fixed(0x0000_8000).hz() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn nearest_hz_rounds_halves_up() {
        let nearest = |fixed| SampleRate::from_fixed(fixed).nearest_hz();
        assert_eq!(nearest(0x2B77_45D1), 11_127);
        assert_eq!(nearest(0x2B77_7FFF), 11_127);
        assert_eq!(nearest(0x2B77_8000), 11_128);
        assert_eq!(nearest(0x5622_0000), 22_050);
        assert_eq!(nearest(u32::MAX), 65_536);
    }

    #[test]
    fn pcm_reports_what_it_was_built_from() {
        let rate = SampleRate::from_fixed(0x5622_0000);
        let pcm = Pcm::new(rate, 2, vec![1, -1, 2, -2, 3, -3], (4, 9), 61);
        assert_eq!(pcm.sample_rate(), rate);
        assert_eq!(pcm.channels(), 2);
        assert_eq!(pcm.samples(), [1, -1, 2, -2, 3, -3]);
        assert_eq!(pcm.frames(), 3);
        assert_eq!(pcm.loop_points(), Some((4, 9)));
        assert_eq!(pcm.base_note(), 61);
        let mono = Pcm::new(rate, 1, vec![5; 7], (0, 0), 60);
        assert_eq!(mono.frames(), 7);
    }

    #[test]
    fn loop_points_are_none_only_when_both_are_zero() {
        let rate = SampleRate::from_fixed(1);
        let loops = |points| Pcm::new(rate, 1, Vec::new(), points, 60).loop_points();
        assert_eq!(loops((0, 0)), None);
        assert_eq!(loops((0, 5)), Some((0, 5)));
        assert_eq!(loops((5, 0)), Some((5, 0)));
        assert_eq!(loops((1742, 1743)), Some((1742, 1743)));
    }
}
