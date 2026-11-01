//! The escape pod's rescue on screen, as `_HandlePlayer` shows it
//! (@0x68de3-0x692ce): a fade to black over the pod, the rescue's `dësc`
//! 13999 over a black screen, a second fade to black, the texts the
//! rescue queued shown on the faded screen, and a fade back in from black
//! only when no text has faded the screen in already. A [`RescueScreen`]
//! is that sequence as plain data: its stages ([`RescueStage`]), their
//! times, what each draws, whether the screen is faded and what was
//! pressed under it. Flight runs it (`FlightView`'s `rescue` module).
//!
//! The stages, each with the original's range:
//!
//! 1. [`RescueStage::FadingOut`]: `_FadeScreenOut` (@0x68de3), 1.0 s to
//!    black over the frame the pod showed last. Nothing moves.
//! 2. [`RescueStage::Black`]: the starfield, particles and sprites cleared
//!    and the window painted black (@0x68de8-0x68e32), then
//!    `_FadeScreenIn` (@0x68e37) fades in from black onto the black
//!    window: 1.0 s of black.
//! 3. [`RescueStage::Text`]: `_ShowTheCursor` (@0x68e3c, @0x68e7b), the
//!    window painted black again (@0x68e41-0x68e76), and the rescue's
//!    `dësc` shown unfaded over it (`_ShowTextDialog` @0x68eac). It is
//!    due from the start of the stage, and the stage ends on the next
//!    tick: flight does not tick while a dialog is open over it, so that
//!    tick is the original's return from the dialog, its movies included.
//! 4. [`RescueStage::FadingOutAgain`]: the window painted black
//!    (@0x68eb1-0x68eea) and `_FadeScreenOut` (@0x68eef), 1.0 s of black
//!    to black, which sets `_screenFaded` (*0x3b5198).
//! 5. [`RescueStage::UnderTexts`]: the missions aborted, the player reset
//!    and the days passed under the flag (@0x68efa-0x6912a). The texts
//!    they queued are due, and the first takes `_ShowTextDialog`'s faded
//!    branch: its repaint, `_RefreshSAT` and `_RedrawStatus(1)`, is drawn
//!    under it, then it fades in from black (@0x5af96-0x5b038; an empty
//!    one with no dialog, @0x5a7bf-0x5a874), which clears the flag
//!    ([`RescueScreen::faded_in`]). The repaint reads the state where the
//!    rescue was as that text was queued ([`FirstText`]): the space area
//!    in a system's `BkgndColor`, empty (every sprite hidden since stage
//!    2), the comm line and the floaters over it, and the bar. The stage
//!    ends on the next tick, the original's return from the last text.
//! 6. [`RescueStage::Closing`]: the window painted black, the space area
//!    in the new system's `BkgndColor`, the floaters, and the bar and
//!    comm redrawn (@0x691cb-0x69253), then `_FadeScreenIn` (@0x692c9):
//!    1.0 s from black while the flag is still set, else nothing, the
//!    repaint overdrawn by the next pass's frame at once.
//!
//! Each repaint ([`Repaint`]) is `_RefreshSAT`'s (@0x6b59): the space
//! area, then `_RedrawComm` and `_FloaterDrawCallback` (@0x6c12-0x6c6b,
//! @0x6daf-0x6dc6), which draw the message while `_commCount` is above 0
//! and the escort menu while it shows, then the bar. `_ResetPlayer` sets
//! `_commCount` to 0 (@0x1d904), so only a repaint before it shows a
//! message, and the closing's `_RedrawComm(0)` (@0x69225) draws none.
//! The floating map is in none of them: its level counts `TickCount`
//! (@0x38ba0-0x38bce), 0 from 150 ticks after its last show, and stages
//! 1, 2 and 4 alone take 180.
//!
//! The fades are each the 1.0 s of `_FadeScreenOut` and `_FadeScreenIn`
//! ([`RESCUE_FADE_FOR`]). None of the range plays a sound
//! (no `PlaySound` or `_beepSnd` in @0x68de3-0x692ce, nor in the fades
//! @0x530f-0x5470). The original skips its fades in windowed mode
//! (@0x5338); the engine draws them always, as it does its other fades
//! ([`crate::fade`]).
//!
//! The fades are synchronous, so what is pressed during stages 1, 2 and 4
//! and the closing fade waits in the event queue: it is kept
//! ([`RescueScreen::hold`]) and given back ([`RescueScreen::take_replay`])
//! as the rescue's `dësc`, the texts or flight come next, for whatever
//! shows then.
//!
//! # Ordering against the original
//!
//! The session settles the whole rescue on the pod's pass
//! (`nova_sim::Session::rescue_if_due`): the missions aborted, the ship
//! reset and moved, the days passed and a Strict Play pilot saved. The
//! screen then presents it over the next seconds, and no frame shows
//! state from after the rescue before the original would:
//!
//! - stage 1 draws the frame the pod showed last, not the rescued layout;
//! - stages 2 to 4 draw black;
//! - stage 5 draws the repaint as it stood when its text was queued: the
//!   pod's system, bar and message line for an `OnAbort`'s text, the
//!   pod's system and radar with the reset ship's bar for the
//!   `OnPurchase`'s, the new system for the days';
//! - stage 6 draws the new system.
//!
//! The Strict Play save is made on the rescue's tick, where the original
//! makes it at @0x692c4, after every text. The state saved is the same,
//! and a quit under a text doesn't skip the original's save either (the
//! phase's step 1 findings), so no boundary the player sees or keeps is
//! crossed.

use std::time::Duration;

use crate::draw::fill_rect;
use crate::fade::{DEATH_FADE_FOR, ScreenFade, whole_view};
use crate::geometry::Bounds;
use crate::input::Input;
use crate::{Color, DrawList, Point};
use nova_sim::flight_frame::RescueTexts;

/// How long each of the rescue's fades, and the black between the first
/// two, lasts: the 1.0 s of `_FadeScreenOut` (@0x5399) and
/// `_FadeScreenIn` (@0x542d).
pub const RESCUE_FADE_FOR: Duration = DEATH_FADE_FOR;

/// The rescue's stages, in order (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RescueStage {
    /// The fade to black over the pod's last frame.
    FadingOut,
    /// The black window, as the screen fades in onto it.
    Black,
    /// The rescue's `dësc` over the black window.
    Text,
    /// The second fade to black.
    FadingOutAgain,
    /// The texts the rescue queued, on the faded screen.
    UnderTexts,
    /// The new system's repaint, and its fade in while still faded.
    Closing,
    /// Over: flight goes on.
    Done,
}

/// What a repaint of the rescue draws, `_RefreshSAT` (@0x6b59) then
/// `_RedrawStatus(1)`: the space area's colour, the comm line and the
/// floaters over it, and the bar.
#[derive(Clone, Debug, PartialEq)]
pub struct Repaint {
    /// The space area's colour: a system's `BkgndColor`.
    pub background: Color,
    /// The comm line and the floaters, as `_RefreshSAT`'s `_RedrawComm`
    /// and `_FloaterDrawCallback` draw them (@0x6c12-0x6c6b,
    /// @0x6daf-0x6dc6): the message while one shows, and the escort menu
    /// while it does.
    pub floaters: DrawList,
    /// The status bar, drawn over the black window.
    pub bar: DrawList,
}

impl Repaint {
    /// Paints the window black, the space area `space` in the background
    /// colour, the floaters, then the bar, over a view `view` in size.
    fn draw(&self, list: &mut DrawList, view: (f32, f32), space: Bounds) {
        fill_rect(list, whole_view(view), Color::BLACK);
        fill_rect(list, space, self.background);
        list.extend_translated(&self.floaters, Point::new(0.0, 0.0));
        list.extend_translated(&self.bar, Point::new(0.0, 0.0));
    }
}

/// Where the rescue was as the first text it shows was queued: what the
/// repaint under it reads, as `_ShowTextDialog`'s faded branch repaints
/// the live state (`_RefreshSAT`, `_RedrawStatus(1)`, @0x5af96-0x5b038).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FirstText {
    /// Queued by an `OnAbort` (@0x68f0d), before `_ResetPlayer`: the
    /// pod's system, its bar and its message line as the pod last drew
    /// them.
    BeforeReset,
    /// Queued by ship 128's `OnPurchase` (@0x68f55), after `_ResetPlayer`
    /// (@0x68f2f) and before the move (`_DeleteAllComputerShips`
    /// @0x68f69): the pod's system and its radar, the reset ship's bar,
    /// and no message, `_commCount` being 0 (@0x1d904).
    AfterReset,
    /// Queued by the days (@0x6912a), after the move: the new system,
    /// its ships on the radar (their sprites hidden still), the reset
    /// ship's bar, and no message.
    AfterMove,
}

impl FirstText {
    /// Where a first text whose place among the texts is `texts`
    /// ([`TextBar::texts`](nova_sim::TextBar::texts)) was queued, by the
    /// counts `at` the rescue noted; none at all reads as before the
    /// reset, as nothing is shown over the repaint then.
    #[must_use]
    pub fn of(texts: Option<u64>, at: RescueTexts) -> Self {
        match texts {
            Some(texts) if texts > at.moved => Self::AfterMove,
            Some(texts) if texts > at.reset => Self::AfterReset,
            _ => Self::BeforeReset,
        }
    }
}

/// The rescue on screen (see the module docs).
#[derive(Clone, Debug, PartialEq)]
pub struct RescueScreen {
    /// The stage it is in, and how far into it, for those that last.
    stage: RescueStage,
    /// How far into the stage it is.
    elapsed: Duration,
    /// The frame the pod showed last: its scene and its HUD.
    pod: DrawList,
    /// The repaint under the first faded text: the pod's system's
    /// background and the bar as the pod last drew it.
    under: Repaint,
    /// Whether the screen is faded (`_screenFaded`).
    faded: bool,
    /// What was pressed since the last hand-over.
    held: Vec<Input>,
    /// What was pressed, given back for what shows next.
    replay: Vec<Input>,
}

impl RescueScreen {
    /// The rescue begins, fading out over `pod`, the frame the pod showed
    /// last, with `under` the repaint under the first faded text.
    #[must_use]
    pub fn new(pod: DrawList, under: Repaint) -> Self {
        Self {
            stage: RescueStage::FadingOut,
            elapsed: Duration::ZERO,
            pod,
            under,
            faded: false,
            held: Vec::new(),
            replay: Vec::new(),
        }
    }

    /// `under` is the repaint under the first faded text instead: the
    /// one a text queued after `_ResetPlayer` reads ([`FirstText`]).
    pub fn repaint_under(&mut self, under: Repaint) {
        self.under = under;
    }

    /// The stage it is in.
    #[must_use]
    pub fn stage(&self) -> RescueStage {
        self.stage
    }

    /// Moves it on by `dt`, carrying what is left of a stage that ends
    /// into the next. A tick that reaches the rescue's `dësc` or its texts
    /// stops there, and the next ends that stage (see the module docs).
    /// Gives the stages it entered, in order.
    pub fn tick(&mut self, dt: Duration) -> Vec<RescueStage> {
        let mut entered = Vec::new();
        let mut left = dt;
        loop {
            match self.stage {
                RescueStage::FadingOut | RescueStage::Black | RescueStage::FadingOutAgain => {
                    let need = RESCUE_FADE_FOR.saturating_sub(self.elapsed);
                    if left < need {
                        self.elapsed += left;
                        break;
                    }
                    left -= need;
                    let next = match self.stage {
                        RescueStage::FadingOut => RescueStage::Black,
                        RescueStage::Black => RescueStage::Text,
                        _ => RescueStage::UnderTexts,
                    };
                    self.enter(next, &mut entered);
                    if next >= RescueStage::Text {
                        break;
                    }
                }
                RescueStage::Text => {
                    self.faded = true;
                    self.enter(RescueStage::FadingOutAgain, &mut entered);
                }
                RescueStage::UnderTexts => {
                    self.enter(RescueStage::Closing, &mut entered);
                    if !self.faded {
                        self.enter(RescueStage::Done, &mut entered);
                        break;
                    }
                }
                RescueStage::Closing => {
                    let need = RESCUE_FADE_FOR.saturating_sub(self.elapsed);
                    if left < need {
                        self.elapsed += left;
                        break;
                    }
                    self.faded = false;
                    self.enter(RescueStage::Done, &mut entered);
                    break;
                }
                RescueStage::Done => break,
            }
        }
        entered
    }

    /// Enters `stage` from its start, noting it in `entered`; what was
    /// pressed is given back as the rescue's `dësc`, its texts or flight
    /// come next.
    fn enter(&mut self, stage: RescueStage, entered: &mut Vec<RescueStage>) {
        self.stage = stage;
        self.elapsed = Duration::ZERO;
        if matches!(
            stage,
            RescueStage::Text | RescueStage::UnderTexts | RescueStage::Done
        ) {
            self.replay.append(&mut self.held);
        }
        entered.push(stage);
    }

    /// Whether the rescue's `dësc` is due: from the black window on.
    #[must_use]
    pub fn desc_due(&self) -> bool {
        self.stage >= RescueStage::Text
    }

    /// Whether the texts the rescue queued are due: once the second fade
    /// is over.
    #[must_use]
    pub fn texts_due(&self) -> bool {
        self.stage >= RescueStage::UnderTexts
    }

    /// Whether the screen is faded (`_screenFaded`): from the second fade
    /// until a text fades it in or the closing fade ends.
    #[must_use]
    pub fn faded(&self) -> bool {
        self.faded
    }

    /// A text has faded the screen in: no closing fade follows.
    pub fn faded_in(&mut self) {
        self.faded = false;
    }

    /// Whether the system's pointer shows: from `_ShowTheCursor`
    /// (@0x68e3c) on.
    #[must_use]
    pub fn shows_pointer(&self) -> bool {
        self.stage >= RescueStage::Text
    }

    /// Keeps `input`, pressed under the rescue, for what shows next.
    pub fn hold(&mut self, input: &Input) {
        self.held.push(*input);
    }

    /// What was pressed under the rescue, given back once its `dësc`, its
    /// texts or flight come next, taken once.
    pub fn take_replay(&mut self) -> Vec<Input> {
        std::mem::take(&mut self.replay)
    }

    /// Draws the stage over a view `view` in size whose space area is
    /// `space`: the pod's frame fading out, black, the repaint under the
    /// texts (black over it while faded), or `closing`'s repaint fading
    /// in. Nothing once done.
    pub fn draw(
        &self,
        list: &mut DrawList,
        (view, space): ((f32, f32), Bounds),
        closing: impl FnOnce() -> Repaint,
    ) {
        match self.stage {
            RescueStage::FadingOut => {
                list.extend_translated(&self.pod, Point::new(0.0, 0.0));
                self.fade(ScreenFade::to_black(RESCUE_FADE_FOR))
                    .draw(list, view);
            }
            RescueStage::Black | RescueStage::Text | RescueStage::FadingOutAgain => {
                fill_rect(list, whole_view(view), Color::BLACK);
            }
            RescueStage::UnderTexts => {
                self.under.draw(list, view, space);
                if self.faded {
                    fill_rect(list, whole_view(view), Color::BLACK);
                }
            }
            RescueStage::Closing => {
                closing().draw(list, view, space);
                self.fade(ScreenFade::from_black(RESCUE_FADE_FOR))
                    .draw(list, view);
            }
            RescueStage::Done => {}
        }
    }

    /// `fade` as far into it as the stage is.
    fn fade(&self, mut fade: ScreenFade) -> ScreenFade {
        fade.advance(self.elapsed);
        fade
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Key;

    const SECOND: Duration = RESCUE_FADE_FOR;

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    fn press(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: false,
        }
    }

    fn pod() -> DrawList {
        let mut list = DrawList::new();
        list.dot(Point::new(10.0, 10.0), 2.0, Color::WHITE);
        list
    }

    fn under() -> Repaint {
        let mut floaters = DrawList::new();
        floaters.dot(Point::new(20.0, 700.0), 2.0, Color::WHITE);
        let mut bar = DrawList::new();
        bar.dot(Point::new(900.0, 10.0), 2.0, Color::WHITE);
        Repaint {
            background: Color::rgba(0, 0, 0x10, 255),
            floaters,
            bar,
        }
    }

    fn screen() -> RescueScreen {
        RescueScreen::new(pod(), under())
    }

    /// A fresh screen moved on to `stage` ([`advance`]).
    fn at(stage: RescueStage) -> RescueScreen {
        let mut screen = screen();
        advance(&mut screen, stage);
        screen
    }

    /// Moves `screen` on to `stage` by whole seconds, and by an empty
    /// tick out of the stages that wait for one.
    fn advance(screen: &mut RescueScreen, stage: RescueStage) {
        while screen.stage() < stage {
            let dt = if matches!(screen.stage(), RescueStage::Text | RescueStage::UnderTexts) {
                Duration::ZERO
            } else {
                SECOND
            };
            screen.tick(dt);
        }
        assert_eq!(screen.stage(), stage);
    }

    const VIEW: (f32, f32) = (1365.0, 768.0);

    fn space() -> Bounds {
        Bounds::at(Point::new(0.0, 0.0), 1171.0, 768.0)
    }

    fn drawn(screen: &RescueScreen, closing: Repaint) -> DrawList {
        let mut list = DrawList::new();
        screen.draw(&mut list, (VIEW, space()), || closing);
        list
    }

    fn black(alpha: u8) -> DrawList {
        let mut list = DrawList::new();
        fill_rect(
            &mut list,
            whole_view(VIEW),
            Color {
                a: alpha,
                ..Color::BLACK
            },
        );
        list
    }

    /// `repaint` as drawn: the window black, the space area in its
    /// background, its floaters, then its bar.
    fn painted(repaint: &Repaint) -> DrawList {
        let mut list = DrawList::new();
        fill_rect(&mut list, whole_view(VIEW), Color::BLACK);
        fill_rect(&mut list, space(), repaint.background);
        list.extend_translated(&repaint.floaters, Point::new(0.0, 0.0));
        list.extend_translated(&repaint.bar, Point::new(0.0, 0.0));
        list
    }

    fn then(mut first: DrawList, second: &DrawList) -> DrawList {
        first.extend_translated(second, Point::new(0.0, 0.0));
        first
    }

    fn new_system() -> Repaint {
        Repaint {
            background: Color::rgba(0x10, 0, 0, 255),
            floaters: DrawList::new(),
            bar: DrawList::new(),
        }
    }

    #[test]
    fn each_fade_and_the_black_last_a_second_and_a_tick_across_carries_on() {
        let mut screen = screen();
        assert_eq!(screen.tick(ms(999)), []);
        assert_eq!(screen.stage(), RescueStage::FadingOut);
        assert_eq!(screen.tick(ms(1)), [RescueStage::Black]);
        assert_eq!(screen.tick(ms(500)), []);
        assert_eq!(screen.tick(ms(600)), [RescueStage::Text], "stops there");
        assert_eq!(screen.stage(), RescueStage::Text);
        assert_eq!(screen.tick(ms(10)), [RescueStage::FadingOutAgain]);
        assert_eq!(screen.tick(ms(989)), []);
        assert_eq!(screen.tick(ms(5)), [RescueStage::UnderTexts]);
        assert_eq!(
            screen.tick(ms(400)),
            [RescueStage::Closing],
            "the next tick"
        );
        assert_eq!(screen.tick(ms(599)), []);
        assert_eq!(screen.tick(ms(1)), [RescueStage::Done]);
        assert_eq!(screen.tick(SECOND), [], "over");
    }

    #[test]
    fn a_long_tick_crosses_the_fade_and_the_black_together() {
        let mut screen = screen();
        assert_eq!(
            screen.tick(ms(2500)),
            [RescueStage::Black, RescueStage::Text]
        );
        let mut screen = at(RescueStage::FadingOutAgain);
        assert_eq!(screen.tick(ms(1500)), [RescueStage::UnderTexts]);
    }

    #[test]
    fn the_desc_is_due_after_the_black_and_the_texts_after_the_second_fade() {
        for (stage, desc, texts) in [
            (RescueStage::FadingOut, false, false),
            (RescueStage::Black, false, false),
            (RescueStage::Text, true, false),
            (RescueStage::FadingOutAgain, true, false),
            (RescueStage::UnderTexts, true, true),
            (RescueStage::Closing, true, true),
        ] {
            let screen = at(stage);
            assert_eq!(screen.desc_due(), desc, "{stage:?}");
            assert_eq!(screen.texts_due(), texts, "{stage:?}");
        }
    }

    #[test]
    fn the_screen_is_faded_from_the_second_fade_until_a_text_fades_it_in() {
        for stage in [
            RescueStage::FadingOut,
            RescueStage::Black,
            RescueStage::Text,
        ] {
            assert!(!at(stage).faded(), "{stage:?}");
        }
        assert!(at(RescueStage::FadingOutAgain).faded());
        let mut screen = at(RescueStage::UnderTexts);
        assert!(screen.faded());
        screen.faded_in();
        assert!(!screen.faded());
    }

    #[test]
    fn still_faded_the_closing_fades_in_over_a_second() {
        let mut screen = at(RescueStage::UnderTexts);
        assert_eq!(screen.tick(ms(1)), [RescueStage::Closing]);
        assert!(screen.faded(), "until the fade ends");
        screen.tick(SECOND.saturating_sub(ms(1)));
        assert_eq!(screen.stage(), RescueStage::Done);
        assert!(!screen.faded());
    }

    #[test]
    fn once_a_text_faded_in_the_closing_is_over_at_once() {
        let mut screen = at(RescueStage::UnderTexts);
        screen.faded_in();
        assert_eq!(
            screen.tick(ms(1)),
            [RescueStage::Closing, RescueStage::Done]
        );
    }

    #[test]
    fn the_pointer_shows_from_the_desc_on() {
        assert!(!at(RescueStage::FadingOut).shows_pointer());
        assert!(!at(RescueStage::Black).shows_pointer());
        for stage in [
            RescueStage::Text,
            RescueStage::FadingOutAgain,
            RescueStage::UnderTexts,
            RescueStage::Closing,
        ] {
            assert!(at(stage).shows_pointer(), "{stage:?}");
        }
    }

    #[test]
    fn what_is_pressed_comes_back_as_the_desc_the_texts_or_flight_come_next() {
        let mut screen = screen();
        screen.hold(&press(Key::Enter));
        screen.tick(SECOND);
        assert_eq!(screen.take_replay(), [], "still black");
        screen.tick(SECOND);
        assert_eq!(screen.take_replay(), [press(Key::Enter)], "the desc");
        assert_eq!(screen.take_replay(), [], "once");
        screen.tick(ms(1));
        screen.hold(&press(Key::Space));
        screen.tick(SECOND);
        assert_eq!(screen.take_replay(), [press(Key::Space)], "the texts");
        screen.tick(ms(1));
        screen.hold(&press(Key::Tab));
        screen.tick(SECOND);
        assert_eq!(screen.stage(), RescueStage::Done);
        assert_eq!(screen.take_replay(), [press(Key::Tab)], "flight");
    }

    #[test]
    fn the_pods_frame_fades_to_black() {
        let mut screen = screen();
        assert_eq!(drawn(&screen, new_system()), pod(), "not yet dark");
        screen.tick(ms(500));
        assert_eq!(drawn(&screen, new_system()), then(pod(), &black(128)));
    }

    #[test]
    fn the_whole_view_is_black_from_the_first_fades_end_to_the_seconds() {
        for stage in [
            RescueStage::Black,
            RescueStage::Text,
            RescueStage::FadingOutAgain,
        ] {
            assert_eq!(drawn(&at(stage), new_system()), black(255), "{stage:?}");
        }
    }

    #[test]
    fn under_the_texts_the_pods_system_is_repainted_unseen_until_faded_in() {
        let mut screen = at(RescueStage::UnderTexts);
        assert_eq!(
            drawn(&screen, new_system()),
            then(painted(&under()), &black(255))
        );
        screen.faded_in();
        assert_eq!(drawn(&screen, new_system()), painted(&under()));
    }

    #[test]
    fn a_repaint_draws_its_floaters_over_the_space_area_and_under_the_bar() {
        let mut screen = at(RescueStage::UnderTexts);
        screen.faded_in();
        let repaint = under();
        let mut expected = DrawList::new();
        fill_rect(&mut expected, whole_view(VIEW), Color::BLACK);
        fill_rect(&mut expected, space(), repaint.background);
        expected.dot(Point::new(20.0, 700.0), 2.0, Color::WHITE);
        expected.dot(Point::new(900.0, 10.0), 2.0, Color::WHITE);
        assert_eq!(drawn(&screen, new_system()), expected);
    }

    #[test]
    fn a_repaint_given_for_the_texts_replaces_the_pods() {
        let mut screen = at(RescueStage::Black);
        screen.repaint_under(new_system());
        advance(&mut screen, RescueStage::UnderTexts);
        screen.faded_in();
        assert_eq!(drawn(&screen, under()), painted(&new_system()));
    }

    fn marks() -> RescueTexts {
        RescueTexts { reset: 3, moved: 5 }
    }

    #[test]
    fn a_text_queued_by_the_abort_scripts_comes_before_the_reset() {
        assert_eq!(FirstText::of(Some(1), marks()), FirstText::BeforeReset);
        assert_eq!(FirstText::of(Some(3), marks()), FirstText::BeforeReset);
        assert_eq!(FirstText::of(None, marks()), FirstText::BeforeReset, "none");
    }

    #[test]
    fn a_text_queued_by_the_on_purchase_comes_after_the_reset_before_the_move() {
        assert_eq!(FirstText::of(Some(4), marks()), FirstText::AfterReset);
        assert_eq!(FirstText::of(Some(5), marks()), FirstText::AfterReset);
    }

    #[test]
    fn a_text_queued_by_the_days_comes_after_the_move() {
        assert_eq!(FirstText::of(Some(6), marks()), FirstText::AfterMove);
    }

    #[test]
    fn the_closing_repaints_the_new_system_under_a_fade_from_black() {
        let mut screen = at(RescueStage::UnderTexts);
        screen.tick(ms(1));
        screen.tick(ms(499));
        assert_eq!(
            drawn(&screen, new_system()),
            then(painted(&new_system()), &black(128))
        );
    }

    #[test]
    fn nothing_is_drawn_once_done() {
        let mut screen = at(RescueStage::UnderTexts);
        screen.faded_in();
        screen.tick(ms(1));
        assert!(drawn(&screen, new_system()).is_empty());
    }

    #[test]
    fn each_fade_takes_a_second() {
        assert_eq!(RESCUE_FADE_FOR, Duration::from_secs(1));
    }
}
