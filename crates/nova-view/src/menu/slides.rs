//! The main screen's three slides: the rows of buttons sliding in as the
//! screen appears, `_LoadMainScreenAnim` setting them off and
//! `_MainScreenAnimIdle` stepping them.
//!
//! **The start** (`_LoadMainScreenAnim` @0x178a5-0x1799e). Slide 0 starts
//! at -1 (@0x17953), and slide i at `index[i-1] + count[i-1] × -0.5`,
//! worked in doubles and truncated toward zero (@0x1795f-0x17987): -1, -6
//! and -11 for the stock counts of 11, 10 and 11. The slides' last step is
//! set to now (`_lastSlideAnimTime` = `TickCount`, @0x17a59).
//!
//! **The steps** (`_MainScreenAnimIdle` @0x1889f-0x18953). Once
//! `TickCount` is at least one past the last step, the last step is set to
//! now and each slide below its count, in order 0, 1, 2, steps: at index
//! 0 it plays 602 ([`SlideSound::Start`]), its index goes up by 1, and on
//! reaching its count it plays 603 ([`SlideSound::End`]). A slide shows
//! frame `index` while it is in `[0, count)` (`_MainScreenAnimDraw`
//! @0x16146), and its row of buttons once it has reached its count
//! (`_MainScreenUpdate` @0x184f8-0x1854a, a signed compare).
//!
//! The engine steps once for each whole Mac tick that passes, carrying
//! the rest, as the original's idle, polled far more often than a tick,
//! steps each tick. While the menu waits unseen (behind the appear's
//! fade), the time counts, but at most one step comes due: the original
//! sets the last step to now at each step, so the first idle after a gap
//! steps once and doesn't catch up. The time behind a dialog over the
//! menu isn't counted at all, as the menu isn't ticked then; the
//! original's first step after one comes at once, the engine's within a
//! tick, which no frame tells apart.
//!
//! **The skip** (`_MainScreenMouse` @0x1ae3a-0x1ae99): a mouseDown sets
//! every slide at or below 0x7cff to 0x7d00, with no sound, so every row
//! shows at once.

use std::time::Duration;

use crate::time::{MAC_TICK_NANOS, sixtieths};

/// What a slide's step plays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlideSound {
    /// Slide `.0` stepped from frame 0: snd 602 (@0x188fc).
    Start(usize),
    /// Slide `.0` reached its count: snd 603 (@0x18932).
    End(usize),
}

/// The three slides' indices, their frame counts and the time since their
/// last step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slides {
    index: [i32; 3],
    counts: [u16; 3],
    since: Duration,
}

impl Slides {
    /// Slides of `counts` frames, every one finished: every row shows.
    #[must_use]
    pub fn finished(counts: [u16; 3]) -> Self {
        Self {
            index: counts.map(i32::from),
            counts,
            since: Duration::ZERO,
        }
    }

    /// Sets the slides off, as `_LoadMainScreenAnim` does: each at its
    /// start index, the last step now.
    pub fn start(&mut self) {
        let mut previous: Option<(i32, u16)> = None;
        for (index, &count) in self.index.iter_mut().zip(&self.counts) {
            *index = match previous {
                None => -1,
                Some((before, count)) => {
                    let start = f64::from(before) + f64::from(count) * -0.5;
                    // Within i32: an index above -32768 less half a u16.
                    #[allow(clippy::cast_possible_truncation)]
                    let start = start.trunc() as i32;
                    start
                }
            };
            previous = Some((*index, count));
        }
        self.since = Duration::ZERO;
    }

    /// `dt` passes while the menu waits unseen: it counts toward the next
    /// step, but at most one step comes due.
    pub fn wait(&mut self, dt: Duration) {
        self.since = self.since.saturating_add(dt).min(one_tick());
    }

    /// `dt` passes: a step for each whole Mac tick, carrying the rest,
    /// until every slide has finished. The sounds the steps play, in
    /// order.
    pub fn tick(&mut self, dt: Duration) -> Vec<SlideSound> {
        self.since = self.since.saturating_add(dt);
        let steps = sixtieths(self.since);
        self.since = Duration::from_nanos((self.since.as_nanos() % MAC_TICK_NANOS) as u64);
        let mut sounds = Vec::new();
        for _ in 0..steps {
            if !self.running() {
                break;
            }
            self.step(&mut sounds);
        }
        sounds
    }

    /// One step of every slide below its count, in order.
    fn step(&mut self, sounds: &mut Vec<SlideSound>) {
        for (slide, (index, &count)) in self.index.iter_mut().zip(&self.counts).enumerate() {
            let count = i32::from(count);
            if *index >= count {
                continue;
            }
            if *index == 0 {
                sounds.push(SlideSound::Start(slide));
            }
            *index += 1;
            if *index == count {
                sounds.push(SlideSound::End(slide));
            }
        }
    }

    /// Finishes every slide at once, silently, as a click does.
    pub fn skip(&mut self) {
        for (index, &count) in self.index.iter_mut().zip(&self.counts) {
            *index = (*index).max(i32::from(count));
        }
    }

    /// Whether any slide is still below its count.
    #[must_use]
    pub fn running(&self) -> bool {
        !(0..self.index.len()).all(|slide| self.row_shown(slide))
    }

    /// Whether slide `row` has finished, so its row of buttons shows; a
    /// row past the last shows.
    #[must_use]
    pub fn row_shown(&self, row: usize) -> bool {
        self.index
            .get(row)
            .zip(self.counts.get(row))
            .is_none_or(|(&index, &count)| index >= i32::from(count))
    }

    /// The frame slide `slide` shows, while it is within its frames.
    #[must_use]
    pub fn frame(&self, slide: usize) -> Option<u16> {
        let (&index, &count) = self.index.get(slide).zip(self.counts.get(slide))?;
        u16::try_from(index).ok().filter(|&frame| frame < count)
    }
}

/// One Mac tick.
fn one_tick() -> Duration {
    Duration::from_nanos(MAC_TICK_NANOS as u64)
}

#[cfg(test)]
mod tests {
    use nova_sim::mission::screens::mac_ticks;

    use super::*;

    const STOCK: [u16; 3] = [11, 10, 11];

    fn started(counts: [u16; 3]) -> Slides {
        let mut slides = Slides::finished(counts);
        slides.start();
        slides
    }

    #[test]
    fn finished_slides_show_every_row_and_no_frame() {
        let slides = Slides::finished(STOCK);
        assert!(!slides.running());
        assert!((0..3).all(|row| slides.row_shown(row)));
        assert!((0..3).all(|slide| slides.frame(slide).is_none()));
    }

    #[test]
    fn stock_counts_start_at_minus_1_minus_6_and_minus_11() {
        let slides = started(STOCK);
        assert_eq!(slides.index, [-1, -6, -11]);
        assert!(slides.running());
        assert!((0..3).all(|row| !slides.row_shown(row)));
    }

    #[test]
    fn the_start_truncates_toward_zero() {
        // -1 - 1.5 = -2.5 is -2, and -2 - 1 = -3.
        assert_eq!(started([3, 2, 3]).index, [-1, -2, -3]);
        // -1 - 2.5 = -3.5 is -3, and -3 - 0.5 = -3.5 is -3.
        assert_eq!(started([5, 1, 4]).index, [-1, -3, -3]);
    }

    #[test]
    fn each_slide_moves_one_frame_a_tick() {
        let mut slides = started(STOCK);
        let mut frames = Vec::new();
        for _ in 0..3 {
            slides.tick(mac_ticks(1));
            frames.push((slides.frame(0), slides.frame(1)));
        }
        assert_eq!(
            frames,
            [(Some(0), None), (Some(1), None), (Some(2), None)],
            "slide 1 is still below 0"
        );
        for _ in 0..3 {
            slides.tick(mac_ticks(1));
        }
        assert_eq!((slides.frame(0), slides.frame(1)), (Some(5), Some(0)));
    }

    /// The sounds of each of `ticks` ticks, by the tick they came in.
    fn sounds_by_tick(slides: &mut Slides, ticks: u32) -> Vec<(u32, SlideSound)> {
        (1..=ticks)
            .flat_map(|tick| {
                slides
                    .tick(mac_ticks(1))
                    .into_iter()
                    .map(move |sound| (tick, sound))
            })
            .collect()
    }

    #[test]
    fn start_sounds_on_the_step_from_frame_0_and_end_on_reaching_the_count() {
        let mut slides = started(STOCK);
        assert_eq!(
            sounds_by_tick(&mut slides, 30),
            [
                (2, SlideSound::Start(0)),
                (7, SlideSound::Start(1)),
                (12, SlideSound::End(0)),
                (12, SlideSound::Start(2)),
                (16, SlideSound::End(1)),
                (22, SlideSound::End(2)),
            ]
        );
        assert!(!slides.running());
    }

    #[test]
    fn a_row_shows_in_the_tick_its_slide_ends() {
        let mut slides = started(STOCK);
        for _ in 0..11 {
            slides.tick(mac_ticks(1));
        }
        assert_eq!(slides.frame(0), Some(10), "the last frame");
        assert!(!slides.row_shown(0));
        slides.tick(mac_ticks(1));
        assert!(slides.row_shown(0));
        assert_eq!(slides.frame(0), None, "past its frames");
        assert!(!slides.row_shown(1));
        assert!(slides.running());
    }

    #[test]
    fn the_first_step_comes_a_tick_after_the_start_and_part_ticks_carry() {
        let mut slides = started(STOCK);
        assert_eq!(slides.tick(Duration::from_millis(10)), []);
        assert_eq!(slides.frame(0), None, "not a whole tick");
        slides.tick(Duration::from_millis(7));
        assert_eq!(slides.frame(0), Some(0), "17 ms make a tick");
        slides.tick(mac_ticks(3));
        assert_eq!(slides.frame(0), Some(3), "three steps");
    }

    #[test]
    fn waiting_unseen_makes_at_most_one_step_due() {
        let mut slides = started(STOCK);
        slides.wait(Duration::from_secs(1));
        assert_eq!(slides.frame(0), None, "nothing steps while it waits");
        slides.tick(Duration::ZERO);
        assert_eq!(slides.frame(0), Some(0), "one step at once");
        slides.tick(Duration::from_millis(10));
        assert_eq!(slides.frame(0), Some(0), "and no catching up");
        let mut slides = started(STOCK);
        slides.wait(Duration::from_millis(10));
        slides.tick(Duration::from_millis(7));
        assert_eq!(slides.frame(0), Some(0), "a part tick waited counts");
    }

    #[test]
    fn a_long_frame_settles_and_steps_no_further() {
        let mut slides = started(STOCK);
        let sounds = slides.tick(mac_ticks(1000));
        assert_eq!(sounds.len(), 6, "each slide's start and end once");
        assert!(!slides.running());
        assert!((0..3).all(|row| slides.row_shown(row)));
    }

    #[test]
    fn finished_slides_step_nothing() {
        let mut slides = Slides::finished(STOCK);
        assert_eq!(slides.tick(mac_ticks(50)), []);
        assert_eq!(slides.index, [11, 10, 11]);
    }

    #[test]
    fn a_skip_finishes_every_slide_silently() {
        let mut slides = started(STOCK);
        for _ in 0..3 {
            slides.tick(mac_ticks(1));
        }
        slides.skip();
        assert!(!slides.running());
        assert!((0..3).all(|row| slides.row_shown(row)));
        assert!((0..3).all(|slide| slides.frame(slide).is_none()));
        assert_eq!(slides.tick(mac_ticks(30)), [], "no 602 or 603 after");
    }

    #[test]
    fn a_slide_of_no_frames_shows_its_row_and_never_a_frame() {
        let mut slides = started([0, 2, 3]);
        assert_eq!(slides.index, [-1, -1, -2]);
        assert_eq!(
            sounds_by_tick(&mut slides, 1),
            [(1, SlideSound::End(0))],
            "-1 steps to 0, its count"
        );
        assert!(slides.row_shown(0));
        assert_eq!(slides.frame(0), None);
    }

    #[test]
    fn a_row_past_the_last_shows() {
        assert!(started(STOCK).row_shown(3));
        assert_eq!(started(STOCK).frame(3), None);
    }
}
