//! The hyperspace jump's effect: the screen fades to white as the ship
//! warps away, and the new system fades in from white.
//!
//! The session flies the warp (see `nova_sim`'s `warp_stage` module): the
//! ship moves off along its bearing, up to 50 pixels a tick, so the
//! camera and the starfield show the rush: the stars are `_HandleStars`'
//! sprites ([`stars`](super::stars)), each moving with the ship by its
//! parallax and wrapping back in as the ship outruns it. The effect plays on top, on
//! display time, in the phases [`JumpPhase`] names:
//!
//! 1. [`JumpPhase::Warp`]: from the jump's start nothing shows, until the
//!    session's `_hyperGamma` goes above 0 (`Session::hyper_gamma`).
//! 2. [`JumpPhase::FadeOut`], for [`FADE_OUT_FOR`]: the scene fades to
//!    white ([`FADE_COLOR`]), from the first tick it is lit
//!    ([`JumpEffect::light`]), then [`JumpPhase::White`] holds it white.
//! 3. The ship arrives when the session says the warp has ended
//!    (`Session::warp_end_due`), whether or not the fade has finished
//!    ([`JumpEffect::arrive`]).
//! 4. [`JumpPhase::FadeIn`], for [`FADE_IN_FOR`]: the new system fades in
//!    from white.
//!
//! # A jump given up
//!
//! A jump given up before it arrives (the session's jump collapsed under a
//! disabled player, or cut by a sabotage kill outfit) ends the effect
//! there ([`JumpEffect::given_up`]). `_HandlePlayer` @0x6840b-0x6841f
//! calls `_FadeWhiteOut` on its first pass with +0x50 no longer above 0
//! and `_hyperGamma` at 0 or more, which a given-up warp leaves as an
//! arrival does: a screen lit fades back from white over [`FADE_IN_FOR`],
//! as `CGDisplayFade` starts from solid white (blend 1.0) wherever the fade
//! to white had got. An unlit one shows nothing: `_FadeWhiteIn` never
//! acquired a fade reservation (@0x548c), so `_FadeWhiteOut`'s
//! `CGDisplayFade` @0x550c runs on a token already released, or never
//! issued, and does nothing.
//!
//! # Without the fades
//!
//! When the Hyperspace Effects preference is off ([`JumpEffect::warp`]'s
//! `fades` false), the session lights no `_hyperGamma`, so the scene
//! never fades out, and the arrival frame shows solid white once,
//! [`JumpPhase::Flash`] for [`ARRIVAL_FLASH_FOR`]; there is no fade-in.
//!
//! # The original's effect
//!
//! Checked against the original Mac executable (`EV Nova.app`, i386, read
//! with its symbols):
//!
//! - `_HandlePlayer` @0x6d1c1-0x6d318: once it faces the bearing the
//!   ship's position moves along it by the warp speed, and `_hyperGamma`
//!   is `(speed - 55) * 5` (@0x6d29c-0x6d2bf). The stars rush past because
//!   the ship itself moves; `_HandleStars` and `_ScrollStarfield` draw no
//!   streak lines, so neither does the engine.
//! - `_HandlePlayer` @0x683a4-0x68409: once `_hyperGamma` is positive it
//!   calls `_FadeWhiteIn` @0x546f, a `CGDisplayFade` of the whole display
//!   to white (rgb 1,1,1) over 1.5 s, not synchronous, so the game runs on
//!   under it, which [`FADE_OUT_FOR`] matches. It is a display fade, so
//!   the HUD whites out too; the flight view draws the fade over
//!   everything.
//! - @0x6bb5e-0x6c70b: the warp ends on the warp clock, the session's to
//!   decide; @0x6be44-0x6bf3a: the arrival frame is painted solid white.
//! - `_HandlePlayer` @0x6840b-0x6841f: on the next frame `_FadeWhiteOut`
//!   @0x54d1 fades the display back from white over 1.5 s, which
//!   [`FADE_IN_FOR`] matches.
//!
//! # Input
//!
//! The session takes no steering, thrust or firing through the warp (its
//! own gates); the player steers, fires and jumps again under the
//! fade-in, as in the original:
//!
//! - `_FadeWhiteOut` @0x54d1-0x5527 calls `CGDisplayFade` with
//!   `synchronous = 0`, so it returns at once, and clears `_screenFaded`
//!   (@0x551e) before the fade has played.
//! - `_HandlePlayer` @0x683bf-0x6841f calls it on the first frame after
//!   arrival and falls through to @0x68424, the frame's flight and key
//!   handling. Arrival (@0x6c1e3-0x6c20b) leaves the ship's jump state
//!   (`ship+0x50`) at 0, and the key handlers gate only on it being
//!   `<= 0`: the hyperspace key, `_QuickKeyCheck(keyPrefs+0x1c)`
//!   @0x6b0da-0x6b0e6, among them.
//! - `_ignoreKeys` is set only around the warm-up frame on entering play
//!   (`_PlayGame` @0x457ff, `_EnterGameFromMainScreen` @0x1a37d) and
//!   cleared in `_InitObjects` @0x1d3ce; `_QuickKeyCheck` @0xa0f5 masks
//!   keys only for it and for the escort menu. No key handler reads
//!   `_hyperGamma`.
//! - `_PlayerEnterHypergate` @0x63c25-0x63c40 uses the same asynchronous
//!   `_FadeWhiteOut`, so a gate's fade-in takes input too.
//!
//! The preference: `Keys.nib` binds the Hyperspace Effects check box to the
//! `HyperspaceEffects` default through `NSNegateBoolean`, so a set default
//! (settings+0x14) means "skip the effects", and a fresh install has them
//! on. All three engine tests of it skip only the white display fades:
//! `_HandlePlayer` @0x6d2a1 (`_hyperGamma` is not raised, though the ship
//! still accelerates away), `_HandlePlayer` @0x683af (no `_FadeWhiteIn`)
//! and `_PlayerEnterHypergate` @0x63c30 (no `_FadeWhiteOut`). The session
//! holds the preference for the first two (`Session::set_hyperspace_effects`).
//! The arrival frame @0x6be44 is painted white without testing it, which
//! [`JumpPhase::Flash`] stands for. The phase text's other reading,
//! arrival at once, is recorded in task `bible-vs-engine-settings`.
//!
//! # Gates
//!
//! Coming out of a hypergate or wormhole plays only the arrival's half,
//! the ship having arrived at once. `_PlayerEnterHypergate` paints the
//! work area white (`ForeColor(30)`, @0x63b44-0x63b5b), then fades the
//! display back from white with `_FadeWhiteOut` unless the preference
//! skips it (@0x63c25-0x63c40): [`JumpEffect::emerging`].
//! `_PlayerEnterWormhole` paints the window white once (@0x643ea-0x64414)
//! and never fades, whatever the preference: [`JumpEffect::flash`].
//!
//! The black fades (`_FadeScreenOut`, `_FadeScreenIn`) serve death, the
//! intro and dialogs, never the jump. The Help Book and the Bible do not
//! describe the effect.

use std::time::Duration;

use crate::draw::fill_rect;
use crate::fade::whole_view;
use crate::{Color, DrawList};

/// How long the screen takes to fade to white: the original's 1.5 s, as
/// its `_FadeWhiteIn` fades the display.
pub const FADE_OUT_FOR: Duration = Duration::from_millis(1500);
/// How long the new system takes to fade in: the original's 1.5 s, as its
/// `_FadeWhiteOut` fades the display back from white.
pub const FADE_IN_FOR: Duration = Duration::from_millis(1500);
/// How long the arrival frame shows solid white when the jump plays
/// without its fades: one frame of the original's 30 a second, ours, as
/// its arrival frame is painted white whatever the preference.
pub const ARRIVAL_FLASH_FOR: Duration = Duration::from_millis(33);
/// The colour the screen fades to, at full alpha: white, as the original's
/// `_FadeWhiteIn` and `_FadeWhiteOut` fade the display.
pub const FADE_COLOR: Color = Color::WHITE;

/// Where the effect is, each timed phase with how far through it, in
/// `[0, 1)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum JumpPhase {
    /// The ship warps, the screen not yet lit.
    Warp,
    /// The old system fades to white.
    FadeOut(f32),
    /// The screen is white, until the ship arrives.
    White,
    /// The new system fades in.
    FadeIn(f32),
    /// The arrival frame shows solid white, when the jump plays without
    /// its fades.
    Flash(f32),
    /// It is over.
    Done,
}

/// A hyperspace jump's effect, as it plays.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JumpEffect {
    /// Whether it fades in from white on arrival, as the Hyperspace
    /// Effects preference asks; without, the arrival flashes white once.
    fades: bool,
    /// How long it has faded to white, once lit.
    white: Option<Duration>,
    /// How long since the ship arrived, once it has.
    arrived: Option<Duration>,
}

impl JumpEffect {
    /// The effect of a jump under way, with its fades or without, as the
    /// Hyperspace Effects preference is on or off: nothing shows yet.
    #[must_use]
    pub fn warp(fades: bool) -> Self {
        Self {
            fades,
            white: None,
            arrived: None,
        }
    }

    /// The effect of coming out of a hypergate: the new system fades in
    /// from white ([`JumpPhase::FadeIn`], for [`FADE_IN_FOR`]) when `fades`,
    /// as the Hyperspace Effects preference is on, and otherwise the screen
    /// shows solid white once ([`JumpPhase::Flash`]): the arrival's half,
    /// the ship having arrived already. See "Gates" in the module docs.
    #[must_use]
    pub fn emerging(fades: bool) -> Self {
        Self {
            fades,
            white: None,
            arrived: Some(Duration::ZERO),
        }
    }

    /// The effect of passing through a wormhole: the screen shows solid
    /// white once ([`JumpPhase::Flash`]), whatever the preference.
    #[must_use]
    pub fn flash() -> Self {
        Self::emerging(false)
    }

    /// Starts the fade to white, as `_FadeWhiteIn` does once `_hyperGamma`
    /// is above 0, unless it has started or the ship has arrived.
    pub fn light(&mut self) {
        if self.white.is_none() && self.arrived.is_none() {
            self.white = Some(Duration::ZERO);
        }
    }

    /// The jump is given up before the ship arrives: once lit, the screen
    /// fades back from white ([`JumpPhase::FadeIn`], for [`FADE_IN_FOR`]),
    /// whatever the fade to white had reached; unlit, nothing more shows.
    /// See "A jump given up" in the module docs.
    #[must_use]
    pub fn given_up(self) -> Option<Self> {
        self.white.map(|_| Self {
            fades: true,
            white: None,
            arrived: Some(Duration::ZERO),
        })
    }

    /// The ship arrives: the arrival's half plays from now, unless it is
    /// playing already.
    pub fn arrive(&mut self) {
        self.arrived.get_or_insert(Duration::ZERO);
    }

    /// Plays `dt` more of the effect: of its arrival's half once the ship
    /// has arrived, else of its fade to white once lit.
    pub fn advance(&mut self, dt: Duration) {
        if let Some(arrived) = &mut self.arrived {
            *arrived = arrived.saturating_add(dt);
        } else if let Some(white) = &mut self.white {
            *white = white.saturating_add(dt);
        }
    }

    /// Where the effect is.
    #[must_use]
    pub fn phase(&self) -> JumpPhase {
        let fraction = |into: Duration, of: Duration| into.as_secs_f32() / of.as_secs_f32();
        match (self.arrived, self.white) {
            (Some(at), _) if self.fades => {
                if at < FADE_IN_FOR {
                    JumpPhase::FadeIn(fraction(at, FADE_IN_FOR))
                } else {
                    JumpPhase::Done
                }
            }
            (Some(at), _) => {
                if at < ARRIVAL_FLASH_FOR {
                    JumpPhase::Flash(fraction(at, ARRIVAL_FLASH_FOR))
                } else {
                    JumpPhase::Done
                }
            }
            (None, None) => JumpPhase::Warp,
            (None, Some(at)) if at < FADE_OUT_FOR => JumpPhase::FadeOut(fraction(at, FADE_OUT_FOR)),
            (None, Some(_)) => JumpPhase::White,
        }
    }

    /// Whether the screen is faded (`_screenFaded`, *0x3b5198): from the
    /// light, which `_FadeWhiteIn` sets at once (@0x5475), until the first
    /// tick after the arrival, whose `_FadeWhiteOut` clears it
    /// (@0x6841f). A text shown meanwhile shows on the faded screen (see
    /// [`faded_texts`](crate::ui::faded_texts)).
    #[must_use]
    pub fn faded(&self) -> bool {
        self.white.is_some() && self.arrived.is_none_or(|at| at.is_zero())
    }

    /// The effect once a text on the faded screen has faded it in from
    /// black (`_FadeScreenIn` @0x5b038, which takes the white's
    /// reservation, @0x53fa-0x5401, and releases it): the white is gone.
    /// Before the arrival the warp drops back to unlit, so the next
    /// [`light`](Self::light) fades to white over [`FADE_OUT_FOR`] again
    /// (`_FadeWhiteIn` on the next pass with the flag clear,
    /// @0x683f1-0x68404); on the arrival's own pass the effect ends, as the
    /// next pass's `_FadeWhiteOut` finds the reservation released and shows
    /// nothing. Unlit, nothing changes.
    #[must_use]
    pub fn faded_in(self) -> Option<Self> {
        if self.white.is_none() {
            return Some(self);
        }
        if self.arrived.is_some() {
            return None;
        }
        Some(Self::warp(self.fades))
    }

    /// Whether the effect is over.
    #[must_use]
    pub fn done(&self) -> bool {
        self.phase() == JumpPhase::Done
    }

    /// The fade's alpha: none while the ship warps unlit, rising to
    /// opaque through the fade-out, opaque until and at the arrival, and
    /// falling back to none through the fade-in.
    #[must_use]
    pub fn fade_alpha(&self) -> u8 {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let alpha = |p: f32| (p * 255.0).round() as u8;
        match self.phase() {
            JumpPhase::Warp | JumpPhase::Done => 0,
            JumpPhase::FadeOut(p) => alpha(p),
            JumpPhase::FadeIn(p) => alpha(1.0 - p),
            JumpPhase::White | JumpPhase::Flash(_) => u8::MAX,
        }
    }

    /// Covers the whole of flight's view, `view` in size, in
    /// [`FADE_COLOR`] at the fade's alpha, if it shows at all, as the
    /// original's white fades cover the whole display.
    pub fn draw_fade(&self, list: &mut DrawList, view: (f32, f32)) {
        let alpha = self.fade_alpha();
        if alpha > 0 {
            fill_rect(
                list,
                whole_view(view),
                Color {
                    a: alpha,
                    ..FADE_COLOR
                },
            );
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::system::camera::VIEW_SIZE as CLASSIC;
    use crate::{DrawCommand, Point};

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    /// A jump lit and played `at`.
    fn lit(at: Duration) -> JumpEffect {
        let mut effect = JumpEffect::warp(true);
        effect.light();
        effect.advance(at);
        effect
    }

    #[test]
    fn a_warp_shows_nothing_until_it_is_lit_then_fades_to_white_and_holds() {
        let mut effect = JumpEffect::warp(true);
        effect.advance(ms(5000));
        assert_eq!(effect.phase(), JumpPhase::Warp, "unlit, time does not run");
        assert_eq!(effect.fade_alpha(), 0);
        assert_eq!(lit(Duration::ZERO).phase(), JumpPhase::FadeOut(0.0));
        assert_eq!(lit(ms(750)).phase(), JumpPhase::FadeOut(0.5));
        assert_eq!(lit(ms(1500)).phase(), JumpPhase::White);
        assert_eq!(lit(ms(60_000)).phase(), JumpPhase::White, "until arrival");
        assert_eq!((FADE_OUT_FOR, FADE_IN_FOR), (ms(1500), ms(1500)));
    }

    #[test]
    fn lighting_again_does_not_restart_the_fade() {
        let mut effect = lit(ms(750));
        effect.light();
        assert_eq!(effect.phase(), JumpPhase::FadeOut(0.5));
    }

    #[test]
    fn the_arrival_fades_in_from_white_whether_or_not_the_fade_out_ended() {
        for at in [Duration::ZERO, ms(300), ms(3000)] {
            let mut effect = lit(at);
            effect.arrive();
            assert_eq!(effect.phase(), JumpPhase::FadeIn(0.0), "{at:?}");
            assert_eq!(effect.fade_alpha(), 255);
            effect.advance(ms(750));
            assert_eq!(effect.phase(), JumpPhase::FadeIn(0.5));
            effect.advance(ms(750));
            assert!(effect.done());
        }
        let mut unlit = JumpEffect::warp(true);
        unlit.arrive();
        assert_eq!(unlit.phase(), JumpPhase::FadeIn(0.0), "white at arrival");
        unlit.light();
        assert_eq!(unlit.phase(), JumpPhase::FadeIn(0.0), "lit too late");
    }

    #[test]
    fn a_jump_given_up_lit_fades_back_from_white_and_unlit_shows_nothing() {
        for at in [Duration::ZERO, ms(300), ms(3000)] {
            let mut effect = lit(at).given_up().expect("lit");
            assert_eq!(effect.phase(), JumpPhase::FadeIn(0.0), "{at:?}");
            assert_eq!(effect.fade_alpha(), 255, "from white");
            effect.advance(ms(750));
            assert_eq!(effect.phase(), JumpPhase::FadeIn(0.5));
            effect.advance(ms(750));
            assert!(effect.done());
        }
        assert_eq!(JumpEffect::warp(true).given_up(), None);
        assert_eq!(JumpEffect::warp(false).given_up(), None);
    }

    #[test]
    fn the_fade_rises_from_nothing_to_opaque_and_falls_back() {
        let alphas: Vec<u8> = [0, 750, 1499, 1500, 9000]
            .into_iter()
            .map(|at| lit(ms(at)).fade_alpha())
            .collect();
        assert_eq!(alphas, [0, 128, 255, 255, 255]);
        let mut arriving = lit(ms(9000));
        arriving.arrive();
        arriving.advance(ms(750));
        assert_eq!(arriving.fade_alpha(), 128);
        arriving.advance(ms(744));
        assert_eq!(arriving.fade_alpha(), 1);
        arriving.advance(ms(6));
        assert_eq!(arriving.fade_alpha(), 0);
    }

    #[test]
    fn the_fade_covers_the_screen_in_white() {
        let mut list = DrawList::new();
        lit(ms(750)).draw_fade(&mut list, CLASSIC);
        lit(ms(750)).draw_fade(&mut list, (1365.0, 768.0));
        let line = |width| DrawCommand::Line {
            from: Point::new(0.0, 384.0),
            to: Point::new(width, 384.0),
            width: 768.0,
            color: Color::rgba(255, 255, 255, 128),
        };
        assert_eq!(
            list.iter().cloned().collect::<Vec<_>>(),
            [line(1024.0), line(1365.0)],
            "the whole view, widened or not"
        );
        assert_eq!(FADE_COLOR, Color::WHITE);
        let mut none = DrawList::new();
        JumpEffect::warp(true).draw_fade(&mut none, CLASSIC);
        assert!(none.is_empty());
    }

    #[test]
    fn without_fades_the_arrival_flashes_white_once() {
        let mut effect = JumpEffect::warp(false);
        assert_eq!(effect.phase(), JumpPhase::Warp);
        effect.arrive();
        assert_eq!(effect.phase(), JumpPhase::Flash(0.0));
        assert_eq!(effect.fade_alpha(), 255);
        effect.advance(ms(32));
        assert!(!effect.done());
        effect.advance(ms(1));
        assert!(effect.done());
        assert_eq!(ARRIVAL_FLASH_FOR, ms(33));
    }

    #[test]
    fn out_of_a_hypergate_the_new_system_fades_in_from_white() {
        let mut gate = JumpEffect::emerging(true);
        assert_eq!(gate.phase(), JumpPhase::FadeIn(0.0));
        assert_eq!(gate.fade_alpha(), 255);
        gate.advance(ms(750));
        assert_eq!(gate.phase(), JumpPhase::FadeIn(0.5));
        gate.advance(ms(750));
        assert!(gate.done());
    }

    #[test]
    fn out_of_a_hypergate_without_fades_and_out_of_a_wormhole_the_screen_flashes_white() {
        for mut effect in [JumpEffect::emerging(false), JumpEffect::flash()] {
            assert_eq!(effect.phase(), JumpPhase::Flash(0.0));
            assert_eq!(effect.fade_alpha(), 255);
            effect.advance(ARRIVAL_FLASH_FOR);
            assert!(effect.done());
        }
    }

    // The faded screen (`_screenFaded`, *0x3b5198).

    #[test]
    fn the_screen_is_faded_from_the_light_to_the_first_tick_after_arrival() {
        let mut effect = JumpEffect::warp(true);
        assert!(!effect.faded(), "unlit");
        effect.light();
        assert!(effect.faded(), "_FadeWhiteIn sets it at once (@0x5475)");
        effect.advance(ms(60_000));
        assert!(effect.faded());
        effect.arrive();
        assert!(effect.faded(), "on the arrival's own pass");
        effect.advance(ms(16));
        assert!(!effect.faded(), "the next pass's _FadeWhiteOut (@0x6841f)");
        assert!(!JumpEffect::emerging(true).faded(), "a gate lights nothing");
        assert!(!JumpEffect::flash().faded());
        assert!(
            !lit(ms(100)).given_up().expect("lit").faded(),
            "given up, faded out"
        );
    }

    #[test]
    fn a_text_fading_the_screen_in_drops_the_warp_back_to_unlit_and_the_next_light_fades_again() {
        let effect = lit(ms(60_000)).faded_in().expect("still warping");
        assert_eq!(effect.phase(), JumpPhase::Warp, "the white is gone");
        assert_eq!(effect.fade_alpha(), 0);
        assert!(!effect.faded());
        let mut relit = effect;
        relit.light();
        assert_eq!(relit.phase(), JumpPhase::FadeOut(0.0), "1.5 s again");
        let mut arrived = lit(ms(60_000));
        arrived.arrive();
        assert_eq!(
            arrived.faded_in(),
            None,
            "on the arrival's pass it ends the effect: the next _FadeWhiteOut \
             finds its reservation released"
        );
        let unlit = JumpEffect::warp(false);
        assert_eq!(unlit.faded_in(), Some(unlit), "nothing to fade in");
    }
}
