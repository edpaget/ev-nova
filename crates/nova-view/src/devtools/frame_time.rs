//! The frame-time readout: the last [`FRAME_WINDOW`] intervals between
//! redraws.

use std::collections::VecDeque;
use std::time::Duration;

/// How many frame intervals [`FrameTimes`] averages over.
pub const FRAME_WINDOW: usize = 60;

/// The most recent [`FRAME_WINDOW`] intervals between redraws, oldest
/// first.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FrameTimes {
    samples: VecDeque<Duration>,
}

impl FrameTimes {
    /// No samples yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds one interval, dropping the oldest once the window is full.
    pub fn record(&mut self, interval: Duration) {
        if self.samples.len() == FRAME_WINDOW {
            self.samples.pop_front();
        }
        self.samples.push_back(interval);
    }

    /// How many intervals the window holds.
    #[must_use]
    pub fn count(&self) -> usize {
        self.samples.len()
    }

    /// The newest interval.
    #[must_use]
    pub fn last(&self) -> Option<Duration> {
        self.samples.back().copied()
    }

    /// The mean interval over the window.
    #[must_use]
    pub fn mean(&self) -> Option<Duration> {
        let count = u32::try_from(self.count()).ok().filter(|&n| n > 0)?;
        Some(self.samples.iter().sum::<Duration>() / count)
    }

    /// Frames per second at the mean interval; `None` with no samples or a
    /// zero mean.
    #[must_use]
    pub fn fps(&self) -> Option<f64> {
        self.mean()
            .filter(|mean| !mean.is_zero())
            .map(|mean| 1.0 / mean.as_secs_f64())
    }

    /// One line for the overlay: the newest and mean intervals in
    /// milliseconds, how many frames the mean covers and, unless the mean
    /// is zero, the frames per second.
    #[must_use]
    pub fn readout(&self) -> String {
        let (Some(last), Some(mean)) = (self.last(), self.mean()) else {
            return "frame time: no frames yet".to_owned();
        };
        let line = format!(
            "frame {:.1} ms, mean {:.1} ms over {} frames",
            millis(last),
            millis(mean),
            self.count()
        );
        match self.fps() {
            Some(fps) => format!("{line}, {fps:.1} fps"),
            None => line,
        }
    }
}

fn millis(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    fn times(samples: &[Duration]) -> FrameTimes {
        let mut times = FrameTimes::new();
        for &sample in samples {
            times.record(sample);
        }
        times
    }

    #[test]
    fn empty_has_no_samples() {
        let times = FrameTimes::new();
        assert_eq!(times.count(), 0);
        assert_eq!(times.last(), None);
        assert_eq!(times.mean(), None);
        assert_eq!(times.fps(), None);
        assert_eq!(FrameTimes::default(), times);
    }

    #[test]
    fn last_mean_and_fps_follow_the_samples() {
        let times = times(&[ms(10), ms(20)]);
        assert_eq!(times.count(), 2);
        assert_eq!(times.last(), Some(ms(20)));
        assert_eq!(times.mean(), Some(ms(15)));
        let fps = times.fps().expect("non-zero mean");
        assert!((fps - 1000.0 / 15.0).abs() < 1e-9, "{fps}");
    }

    #[test]
    fn the_window_keeps_only_the_latest_samples() {
        let mut samples = vec![Duration::from_secs(1)];
        samples.extend([ms(10); FRAME_WINDOW]);
        let times = times(&samples);
        assert_eq!(times.count(), FRAME_WINDOW);
        assert_eq!(times.mean(), Some(ms(10)));
    }

    #[test]
    fn last_is_the_newest_sample_after_the_window_wraps() {
        let mut samples = vec![ms(5); FRAME_WINDOW + 3];
        samples.push(ms(7));
        let times = times(&samples);
        assert_eq!(times.last(), Some(ms(7)));
        assert_eq!(times.count(), FRAME_WINDOW);
    }

    #[test]
    fn a_zero_mean_has_no_fps() {
        let times = times(&[Duration::ZERO, Duration::ZERO]);
        assert_eq!(times.mean(), Some(Duration::ZERO));
        assert_eq!(times.fps(), None);
    }

    #[test]
    fn the_readout_says_when_there_are_no_samples() {
        let readout = FrameTimes::new().readout();
        assert!(readout.contains("no frames"), "{readout}");
    }

    #[test]
    fn the_readout_shows_the_last_and_mean_times_the_count_and_fps() {
        let readout = times(&[ms(10), ms(20)]).readout();
        assert_eq!(
            readout,
            "frame 20.0 ms, mean 15.0 ms over 2 frames, 66.7 fps"
        );
    }

    #[test]
    fn the_readout_leaves_out_fps_when_the_mean_is_zero() {
        let readout = times(&[Duration::ZERO]).readout();
        assert_eq!(readout, "frame 0.0 ms, mean 0.0 ms over 1 frames");
        assert!(!readout.contains("fps"), "{readout}");
    }
}
