//! The Red Alert: the sound and the icon the original gives when the
//! player becomes threatened.
//!
//! Every 60 frames ([`alert_check_due`]) `_HandlePlayer` checks whether any
//! ship threatens the player (@0x68c79-0x68cad), keeping the last check's
//! answer (`0x3b5418 ← 0x3b52b0`, @0x68caf-0x68cc7). Only a check that
//! finds a threat after one that found none goes on (@0x68cc9-0x68cce),
//! so the alert is edge-triggered and re-arms only on a check that finds
//! no threat; a threat that comes and goes between two checks is never
//! seen. While the player is in the escape pod (its countdown +0xc908
//! above 0, @0x68cd3-0x68cdb) nothing more happens, and the edge is spent.
//! A dying player out of the pod is not held back. Else
//! `snd ` 370 plays (@0x68cdd-0x68cff), and, only when the player's
//! `Volume` preference is 1 or less (@0x68d04-0x68d15; see
//! [`RuleKey::AlertBlink`](crate::RuleKey::AlertBlink)), the blink starts
//! with [`ALERT_STEPS`] steps.
//!
//! `_HandleAlertSprite` (@0x2e61d-0x2e6f0), every pass after the check,
//! shows the icon while steps are left, at frame `steps & 1`
//! (@0x2e680-0x2e69c): `cïcn` 18000 for an even count, 18001 for an odd
//! one, so it alternates rather than turning on and off. Then, on a frame
//! that is a multiple of [`ALERT_STEP_FRAMES`], it takes a step
//! (@0x2e6a1-0x2e6ce). From the trigger frame F, a multiple of 60, F
//! shows 18000, F+1 to F+5 18001, F+6 to F+10 18000, and so on to F+141
//! to F+145, 18001; from F+146 it is hidden: 146 ticks visible.
//!
//! Landing (`_HandlePlayerDockRequest` @0x66d84-0x66da0) and hyperspace
//! arrival (`_HandlePlayer` @0x6c024-0x6c046) hide the icon and end the
//! blink ([`RedAlert::clear_icon`]) but keep the last check's answer, so a
//! threat still standing after take-off or arrival does not alert again.
//! A hypergate or wormhole touches neither.
//!
//! The original's double time runs the check twice on one frame-counter
//! value, once on each pass (`_PlayGame` @0x45f20-0x45f31), but the step
//! only once, on the advance pass (`_DoPlayGameWork` @0x44518); see the
//! session's `red_alert` module.

/// The `cïcn` of the alert icon's first frame; its second is the next.
pub const ALERT_CICN: i16 = 18000;

/// How many frames apart the threat checks fall (@0x68c79-0x68cad).
pub const ALERT_CHECK_FRAMES: i16 = 60;

/// How many steps a blink takes (0x1e, @0x68d15).
pub const ALERT_STEPS: u8 = 30;

/// How many frames apart the blink's steps fall (@0x2e6a1-0x2e6ce).
pub const ALERT_STEP_FRAMES: i16 = 5;

/// Whether the threat check falls due on `frame`: a multiple of
/// [`ALERT_CHECK_FRAMES`]. The remainder truncates toward zero, as the
/// original's does, so the frames -14 to -1 after a take-off are not due.
#[must_use]
pub fn alert_check_due(frame: i16) -> bool {
    frame % ALERT_CHECK_FRAMES == 0
}

/// Whether the blink takes a step on `frame`: a multiple of
/// [`ALERT_STEP_FRAMES`], truncating as [`alert_check_due`] does.
#[must_use]
pub fn alert_step_due(frame: i16) -> bool {
    frame % ALERT_STEP_FRAMES == 0
}

/// Which of the alert icon's two frames is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlertIcon {
    /// Frame 0, `cïcn` 18000: shown for an even count of steps left.
    Red,
    /// Frame 1, `cïcn` 18001: shown for an odd count.
    Dark,
}

impl AlertIcon {
    /// The frame shown with `steps` left (`_alertCount & 1`, @0x2e680).
    const fn of(steps: u8) -> Self {
        if steps & 1 == 0 {
            Self::Red
        } else {
            Self::Dark
        }
    }

    /// The `cïcn` this frame draws.
    #[must_use]
    pub const fn cicn(self) -> i16 {
        match self {
            Self::Red => ALERT_CICN,
            Self::Dark => ALERT_CICN + 1,
        }
    }
}

/// The alert's state: the last check's answer, the blink's steps left and
/// the icon frame shown.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RedAlert {
    threatened: bool,
    steps: u8,
    icon: Option<AlertIcon>,
}

impl RedAlert {
    /// One threat check, on a due frame ([`alert_check_due`]): whether the
    /// player is `threatened` now. It alerts, giving `true`, only when the
    /// last check found no threat and the escape pod is not counting down
    /// (`pod_running`); and then starts the blink when it `blinks`.
    pub fn check(&mut self, threatened: bool, pod_running: bool, blinks: bool) -> bool {
        let prev = std::mem::replace(&mut self.threatened, threatened);
        if !threatened || prev || pod_running {
            return false;
        }
        if blinks {
            self.steps = ALERT_STEPS;
        }
        true
    }

    /// One pass of the icon on `frame`: shown while steps are left, at the
    /// frame their count gives, then a step taken on a due frame
    /// ([`alert_step_due`]).
    pub fn step(&mut self, frame: i16) {
        if self.steps == 0 {
            self.icon = None;
            return;
        }
        self.icon = Some(AlertIcon::of(self.steps));
        if alert_step_due(frame) {
            self.steps -= 1;
        }
    }

    /// Hides the icon and ends the blink, keeping the last check's answer,
    /// as landing and hyperspace arrival do.
    pub fn clear_icon(&mut self) {
        self.steps = 0;
        self.icon = None;
    }

    /// The icon frame shown, if any.
    #[must_use]
    pub fn icon(&self) -> Option<AlertIcon> {
        self.icon
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The icons shown over `ticks` ticks from frame `from`, each tick
    /// stepping first.
    fn icons(alert: &mut RedAlert, from: i16, ticks: i16) -> Vec<Option<AlertIcon>> {
        (from..from + ticks)
            .map(|frame| {
                alert.step(frame);
                alert.icon()
            })
            .collect()
    }

    #[test]
    fn the_check_is_due_on_multiples_of_60_and_never_on_the_frames_after_a_take_off() {
        for frame in [0, 60, 120, 1020] {
            assert!(alert_check_due(frame), "{frame}");
        }
        for frame in [1, 30, 59, 61, 1024, -14, -1] {
            assert!(!alert_check_due(frame), "{frame}");
        }
    }

    #[test]
    fn the_step_is_due_on_multiples_of_5() {
        for frame in [0, 5, 60, 1020, -10, -5] {
            assert!(alert_step_due(frame), "{frame}");
        }
        for frame in [1, 4, 6, 1024, -14, -1] {
            assert!(!alert_step_due(frame), "{frame}");
        }
    }

    #[test]
    fn the_icon_frames_are_cicn_18000_and_18001() {
        assert_eq!(AlertIcon::Red.cicn(), 18000);
        assert_eq!(AlertIcon::Dark.cicn(), 18001);
    }

    #[test]
    fn a_threat_found_by_a_check_after_none_sounds_and_starts_30_steps() {
        let mut alert = RedAlert::default();
        assert!(alert.check(true, false, true));
        assert_eq!(alert.steps, 30);
        assert_eq!(alert.icon(), None, "shown at the next pass");
    }

    #[test]
    fn the_blink_shows_18000_on_the_trigger_tick_then_alternates_every_5_ticks_for_146_ticks() {
        let mut alert = RedAlert::default();
        assert!(alert.check(true, false, true));
        let shown = icons(&mut alert, 60, 160);
        for (tick, icon) in shown.iter().enumerate() {
            let expected = match tick {
                146.. => None,
                1.. if ((tick - 1) / 5).is_multiple_of(2) => Some(AlertIcon::Dark),
                _ => Some(AlertIcon::Red),
            };
            assert_eq!(*icon, expected, "tick {tick}");
        }
        let red = shown.iter().filter(|&&i| i == Some(AlertIcon::Red)).count();
        let dark = shown
            .iter()
            .filter(|&&i| i == Some(AlertIcon::Dark))
            .count();
        assert_eq!((red, dark), (71, 75));
    }

    #[test]
    fn the_blink_needs_blinks() {
        let mut alert = RedAlert::default();
        assert!(alert.check(true, false, false));
        assert!(icons(&mut alert, 60, 160).iter().all(Option::is_none));
    }

    #[test]
    fn staying_threatened_alerts_once() {
        let mut alert = RedAlert::default();
        assert!(alert.check(true, false, true));
        assert!(!alert.check(true, false, true));
        assert!(!alert.check(true, false, true));
    }

    #[test]
    fn no_threat_no_alert() {
        let mut alert = RedAlert::default();
        assert!(!alert.check(false, false, true));
        assert!(!alert.check(false, false, true));
        assert!(icons(&mut alert, 0, 200).iter().all(Option::is_none));
    }

    #[test]
    fn a_check_without_a_threat_re_arms_it() {
        let mut alert = RedAlert::default();
        assert!(alert.check(true, false, true));
        assert!(!alert.check(false, false, true));
        assert!(alert.check(true, false, true));
    }

    #[test]
    fn a_running_pod_gives_no_alert_and_the_threat_is_spent() {
        let mut alert = RedAlert::default();
        assert!(!alert.check(true, true, true));
        assert_eq!(alert.steps, 0);
        assert!(!alert.check(true, false, true), "the edge is spent");
    }

    #[test]
    fn clearing_the_icon_ends_the_blink_but_keeps_the_threat() {
        let mut alert = RedAlert::default();
        assert!(alert.check(true, false, true));
        alert.step(60);
        assert!(alert.icon().is_some());
        alert.clear_icon();
        assert_eq!(alert.icon(), None);
        assert!(icons(&mut alert, 61, 160).iter().all(Option::is_none));
        assert!(!alert.check(true, false, true), "still threatened");
    }

    #[test]
    fn a_new_blink_restarts_from_30_steps() {
        let mut alert = RedAlert::default();
        assert!(alert.check(true, false, true));
        icons(&mut alert, 60, 60);
        assert!(!alert.check(false, false, true));
        assert!(alert.check(true, false, true));
        assert_eq!(alert.steps, 30);
    }
}
