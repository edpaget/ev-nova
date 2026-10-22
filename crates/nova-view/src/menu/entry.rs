//! How play is entered from the main screen: `_EnterGameFromMainScreen`
//! @0x19ca7's sequencing, and what it does with the input meanwhile. The
//! router runs an [`Entry`] from Enter Ship until flight's fade-in ends,
//! and gives it every input and the window's close first; the `Entry`
//! decides what is drawn over what, when the intro's text is due, when to
//! hand over to flight, and what it holds or drops.
//!
//! **The order.**
//!
//! 1. The main screen fades to black over a second (`_FadeScreenOut`,
//!    synchronous), drawn but neither ticked nor given input. The music
//!    plays on.
//! 2. A pilot that hasn't seen the intro screen sees it
//!    ([`Intro`](super::intro::Intro), `_ShowIntroScreen` @0x16cf1),
//!    begun with the skip key's state as the fade ends. Its text, when it
//!    is due ([`Entry::take_text_due`]), shows in "Desc Dialog" over
//!    black, which the router opens; it fades in from black over a second
//!    (@0x5b038), and once it closes the screen fades to black over a
//!    second. The fade after the pictures alone is instant, as
//!    `_FadeScreenOut` does nothing on a screen already black
//!    (`_screenFaded` @0x5316-0x531f). The pilot is marked seen once the
//!    intro is over, an empty one included ([`Entry::take_mark_seen`];
//!    @0x19ea2-0x19ebc).
//! 3. The handover ([`Entry::handover_due`]): the router moves the game
//!    into flight, and flight's first frame is a frozen one with keys
//!    ignored (`_ignoreKeys`/`_gameFrozen` around `_DoPlayGameWork(1)`
//!    @0x1a37d-0x1a397, and again in `_PlayGame` @0x457f3-0x45819).
//! 4. Flight fades in from black over a second, not ticked meanwhile
//!    (`_PlayGame`'s synchronous `_FadeScreenIn` @0x4589c). Then play
//!    runs.
//!
//! **The scene** ([`Entry::showing`]): the main menu through the fade
//! out, [`Showing::Intro`] while the pictures show (the music keeps
//! playing), [`Showing::IntroText`] for the text and the fade after it,
//! and flight from the handover.
//!
//! **The input.** The original polls the keys and the button through the
//! whole sequence, and its queued events are drained by the in-game
//! handler just before `_PlayGame` (`_IterateInGameEventLoop(-1)`
//! @0x1a446), unless a flush drops them first. So:
//!
//! - Return, Space, the skip key and the primary button are tracked as
//!   down or up, for the intro ([`IntroKeys`]).
//! - Every plain key, click and text input is otherwise dropped: flight
//!   never sees an input from before its fade-in ends.
//! - Command chords and the window's close are held, in order, for the
//!   handover. A chord is held as the command it names
//!   ([`Deferred::Command`]), never as key presses and releases: in the
//!   original a Command key goes to `MenuKey` and never reaches the key
//!   map flight polls, so it leaves no key down in flight, whether
//!   Command or the window's focus is let go of first. The router takes
//!   the chords it has commands for (Command-Q, Command-comma) and
//!   ignores the rest, Command-Escape and Command-arrow among them.
//!   The intro text's opening drops
//!   everything held (`FlushEvents(0xff7f)` @0x5ae67), and a close is
//!   dropped once the intro has run, as its flush after the text
//!   (`FlushEvents` 3, 5, 1 and 2 @0x1711a-0x17165) drops pointer events.
//! - The text takes every input once its fade-in ends; what arrives
//!   during the fade-in reaches it then ([`Entry::take_for_text`]).
//! - Command chords and the close that arrive during flight's fade-in
//!   are held until it ends.
//! - The window losing focus, or the pointer's cancel, lets go of the
//!   tracked keys and button.
//!
//! After the fade-in, a key held through the entry is taken up by its
//! next OS repeat, which flight counts as a press; a key pressed and
//! released during the entry does nothing. That is the engine's form of
//! the original's polling.

use std::time::Duration;

use super::intro::{Intro, IntroEnd, IntroKeys};
use crate::draw::fill_rect;
use crate::fade::{DEATH_FADE_FOR, ScreenFade, whole_view};
use crate::screen::Showing;
use crate::sound::UiSound;
use crate::{Color, DrawList, Input, Key};

/// What an [`Entry`] does with an input or a close.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// It is dropped.
    Drop,
    /// It waits, kept by the entry, for the handover or the fade-in's end
    /// ([`Entry::handed_over`], [`Entry::take_held`]).
    Held,
    /// The intro's text takes it now.
    Text,
    /// It waits for the intro's text, until its fade-in ends
    /// ([`Entry::take_for_text`]).
    HeldForText,
}

/// Something an [`Entry`] kept for the handover or the fade-in's end.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Deferred {
    /// A Command chord: the key pressed with Command down, for the router
    /// to take as a menu command (Command-Q, Command-comma) or ignore.
    /// It is never replayed as a key press, so it leaves no key down.
    Command(Key),
    /// The window's close.
    Close,
}

/// Where an entry is.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Stage {
    /// The main screen fades to black.
    FadingOut(ScreenFade),
    /// The intro's pictures.
    Intro,
    /// The intro's text, `dësc` `id`, waits for the router to open it.
    TextDue(i16),
    /// The intro's text shows, fading in from black while the fade is
    /// there.
    Text(Option<ScreenFade>),
    /// The screen fades to black after the text.
    FadingToBlack(ScreenFade),
    /// Flight is due to take over.
    HandoverDue,
    /// Flight fades in from black.
    FadingIn(ScreenFade),
    /// Play runs.
    Done,
}

/// Play being entered from the main screen.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    stage: Stage,
    intro: Option<Intro>,
    keys: IntroKeys,
    held: Vec<Deferred>,
    for_text: Vec<Input>,
    mark_seen: bool,
}

impl Entry {
    /// Enter Ship's entry: the main screen's fade out, then `intro` for a
    /// pilot that hasn't seen the intro screen (`None` for one that has),
    /// then the handover and flight's fade-in.
    #[must_use]
    pub fn new(intro: Option<Intro>) -> Self {
        Self {
            stage: Stage::FadingOut(fade_out()),
            intro,
            keys: IntroKeys::default(),
            held: Vec::new(),
            for_text: Vec::new(),
            mark_seen: false,
        }
    }

    /// `dt` passes: the fade or the intro moves on.
    pub fn tick(&mut self, dt: Duration) {
        let faded = match &mut self.stage {
            Stage::FadingOut(fade)
            | Stage::Text(Some(fade))
            | Stage::FadingToBlack(fade)
            | Stage::FadingIn(fade) => {
                fade.advance(dt);
                fade.done()
            }
            Stage::Intro => {
                if let Some(intro) = &mut self.intro {
                    intro.tick(dt);
                }
                self.poll_intro();
                return;
            }
            Stage::TextDue(_) | Stage::Text(None) | Stage::HandoverDue | Stage::Done => return,
        };
        if !faded {
            return;
        }
        self.stage = match self.stage {
            Stage::FadingOut(_) => match &mut self.intro {
                Some(intro) => {
                    intro.begin(self.keys);
                    self.stage = Stage::Intro;
                    self.poll_intro();
                    return;
                }
                None => Stage::HandoverDue,
            },
            Stage::Text(_) => Stage::Text(None),
            Stage::FadingToBlack(_) => Stage::HandoverDue,
            _ => Stage::Done,
        };
    }

    /// Moves on once the intro's pictures have ended: to its text, when
    /// one is due, or else past the intro, to the handover at once, the
    /// screen being black already.
    fn poll_intro(&mut self) {
        match self.intro.as_ref().and_then(Intro::end) {
            Some(IntroEnd::Done { text: Some(text) }) => self.stage = Stage::TextDue(text),
            Some(IntroEnd::Done { text: None } | IntroEnd::Skipped) => {
                self.intro_over();
                self.stage = Stage::HandoverDue;
            }
            None => {}
        }
    }

    /// The intro is over: the pilot is to be marked seen, and a close held
    /// is dropped with the pointer events its flush drops.
    fn intro_over(&mut self) {
        self.mark_seen = true;
        self.held.retain(|deferred| *deferred != Deferred::Close);
    }

    /// The intro's text, once it is due, until the router says it opened
    /// ([`Entry::text_opened`]).
    #[must_use]
    pub fn take_text_due(&self) -> Option<i16> {
        match self.stage {
            Stage::TextDue(text) => Some(text),
            _ => None,
        }
    }

    /// The router opened the intro's text (`opened`), or couldn't, as
    /// without dialogs. Opened, it drops everything held, and fades in
    /// from black; not, the text is passed over, to the handover at once.
    pub fn text_opened(&mut self, opened: bool) {
        if !matches!(self.stage, Stage::TextDue(_)) {
            return;
        }
        if opened {
            self.held.clear();
            self.stage = Stage::Text(Some(fade_in()));
        } else {
            self.intro_over();
            self.stage = Stage::HandoverDue;
        }
    }

    /// The intro's text closed: the intro is over, and the screen fades
    /// to black.
    pub fn text_closed(&mut self) {
        if matches!(self.stage, Stage::Text(_)) {
            self.intro_over();
            self.stage = Stage::FadingToBlack(fade_out());
        }
    }

    /// The inputs held for the intro's text, once its fade-in has ended.
    pub fn take_for_text(&mut self) -> Vec<Input> {
        if self.stage == Stage::Text(None) {
            std::mem::take(&mut self.for_text)
        } else {
            Vec::new()
        }
    }

    /// Whether flight is due to take over.
    #[must_use]
    pub fn handover_due(&self) -> bool {
        self.stage == Stage::HandoverDue
    }

    /// The router has handed over to flight: its fade-in begins, and what
    /// was held is given back, in order, for the router to replay.
    pub fn handed_over(&mut self) -> Vec<Deferred> {
        if self.stage != Stage::HandoverDue {
            return Vec::new();
        }
        self.stage = Stage::FadingIn(fade_in());
        std::mem::take(&mut self.held)
    }

    /// Whether play runs: flight's fade-in has ended.
    #[must_use]
    pub fn done(&self) -> bool {
        self.stage == Stage::Done
    }

    /// What was held during flight's fade-in, in order, once it has
    /// ended.
    pub fn take_held(&mut self) -> Vec<Deferred> {
        if self.done() {
            std::mem::take(&mut self.held)
        } else {
            Vec::new()
        }
    }

    /// Whether the pilot is to be marked as having seen the intro screen,
    /// once.
    pub fn take_mark_seen(&mut self) -> bool {
        std::mem::take(&mut self.mark_seen)
    }

    /// The intro's sounds since they were last taken.
    pub fn take_sounds(&mut self) -> Vec<UiSound> {
        self.intro
            .as_mut()
            .map(Intro::take_sounds)
            .unwrap_or_default()
    }

    /// The scene: the main menu through the fade out, the intro's
    /// pictures, its text and the fade after it, then flight.
    #[must_use]
    pub fn showing(&self) -> Showing {
        match self.stage {
            Stage::FadingOut(_) => Showing::MainMenu,
            Stage::Intro => Showing::Intro,
            Stage::TextDue(_) | Stage::Text(_) | Stage::FadingToBlack(_) => Showing::IntroText,
            Stage::HandoverDue | Stage::FadingIn(_) | Stage::Done => Showing::Flight,
        }
    }

    /// Whether a Command chord's press made now is held for the handover
    /// ([`Deferred::Command`]): outside the intro's text, until play runs.
    /// The text, due or showing, takes it instead.
    #[must_use]
    pub fn holds_commands(&self) -> bool {
        !matches!(self.stage, Stage::TextDue(_) | Stage::Text(_) | Stage::Done)
    }

    /// Whether the main screen is drawn under the entry: through its fade
    /// out.
    #[must_use]
    pub fn menu_drawn(&self) -> bool {
        matches!(self.stage, Stage::FadingOut(_))
    }

    /// Whether the intro hides the pointer: while its pictures show,
    /// which is the intro stage alone (the intro hides it neither before
    /// it begins nor once it has ended).
    #[must_use]
    pub fn hides_pointer(&self) -> bool {
        self.intro.as_ref().is_some_and(Intro::hides_pointer)
    }

    /// What becomes of `input`, Command being held as `command_held`
    /// says. The intro's keys are noted either way. Outside the intro's
    /// text, a key's first press with Command down is held as a
    /// [`Deferred::Command`]; Command itself, repeats and every release
    /// are dropped.
    pub fn input(&mut self, input: &Input, command_held: bool) -> Verdict {
        self.keys.note(input);
        match self.stage {
            Stage::Intro => {
                if let Some(intro) = &mut self.intro {
                    intro.input(input);
                }
                self.poll_intro();
            }
            Stage::TextDue(_) | Stage::Text(Some(_)) => {
                self.for_text.push(*input);
                return Verdict::HeldForText;
            }
            Stage::Text(None) => return Verdict::Text,
            Stage::Done => return Verdict::Drop,
            Stage::FadingOut(_)
            | Stage::FadingToBlack(_)
            | Stage::HandoverDue
            | Stage::FadingIn(_) => {}
        }
        match *input {
            Input::Key {
                key,
                pressed: true,
                repeat: false,
            } if command_held && key != Key::Command => {
                self.held.push(Deferred::Command(key));
                Verdict::Held
            }
            _ => Verdict::Drop,
        }
    }

    /// What becomes of the window's close: held, but dropped while the
    /// intro's text shows, whose own flush after it would drop it.
    pub fn close(&mut self) -> Verdict {
        match self.stage {
            Stage::TextDue(_) | Stage::Text(_) | Stage::Done => Verdict::Drop,
            _ => {
                self.held.push(Deferred::Close);
                Verdict::Held
            }
        }
    }

    /// The keys and the button were let go of outside the game (the
    /// window losing focus).
    pub fn release_keys(&mut self) {
        self.keys = IntroKeys::default();
        if let Some(intro) = &mut self.intro
            && self.stage == Stage::Intro
        {
            intro.release_keys();
            self.poll_intro();
        }
    }

    /// The pointer was cancelled: the primary button counts as up.
    pub fn cancel_pointer(&mut self) {
        self.keys.primary = false;
        if let Some(intro) = &mut self.intro
            && self.stage == Stage::Intro
        {
            intro.cancel_pointer();
            self.poll_intro();
        }
    }

    /// Draws what goes under the intro's text in a view `view` in size:
    /// the intro's pictures, or black from the intro's end to the
    /// handover.
    pub fn draw_under(&self, list: &mut DrawList, view: (f32, f32)) {
        match self.stage {
            Stage::Intro => {
                if let Some(intro) = &self.intro {
                    intro.draw(list, view);
                }
            }
            Stage::TextDue(_) | Stage::Text(_) | Stage::FadingToBlack(_) | Stage::HandoverDue => {
                fill_rect(list, whole_view(view), Color::BLACK);
            }
            Stage::FadingOut(_) | Stage::FadingIn(_) | Stage::Done => {}
        }
    }

    /// Draws the fades over everything, the intro's text included, in a
    /// view `view` in size.
    pub fn draw_over(&self, list: &mut DrawList, view: (f32, f32)) {
        match self.stage {
            Stage::FadingOut(fade)
            | Stage::Text(Some(fade))
            | Stage::FadingToBlack(fade)
            | Stage::FadingIn(fade) => fade.draw(list, view),
            _ => {}
        }
    }
}

/// The second's fade to black, `_FadeScreenOut`'s (@0x5399).
fn fade_out() -> ScreenFade {
    ScreenFade::to_black(DEATH_FADE_FOR)
}

/// The second's fade from black, `_FadeScreenIn`'s (@0x542d).
fn fade_in() -> ScreenFade {
    ScreenFade::from_black(DEATH_FADE_FOR)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::menu::intro::{CharacterIntro, INTRO_SOUND, IntroPicture, IntroSource, SKIP_KEY};
    use crate::sound::SoundId;
    use crate::{DrawCommand, MouseButton, Point};

    const VIEW: (f32, f32) = (1024.0, 768.0);
    const SECOND: Duration = Duration::from_secs(1);

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    /// A source with 640 x 480 `PICT`s 8200 and 8201, and snd 30003 when
    /// `sound`.
    struct Source {
        sizes: BTreeMap<i16, (u32, u32)>,
        sound: bool,
    }

    impl IntroSource for Source {
        fn first_character_intro(&self) -> Option<CharacterIntro> {
            None
        }

        fn intro_picture_size(&self, picture: i16) -> Option<(u32, u32)> {
            self.sizes.get(&picture).copied()
        }

        fn has_sound(&self, sound: SoundId) -> bool {
            self.sound && sound == INTRO_SOUND
        }
    }

    fn source() -> Source {
        Source {
            sizes: [(8200, (640, 480)), (8201, (640, 480))].into(),
            sound: true,
        }
    }

    /// Two pictures of 2 s, and `text`.
    fn pictures(text: Option<i16>) -> CharacterIntro {
        CharacterIntro {
            pictures: [
                IntroPicture {
                    picture: 8200,
                    delay: 2,
                },
                IntroPicture {
                    picture: 8201,
                    delay: 2,
                },
                IntroPicture::NONE,
                IntroPicture::NONE,
            ],
            text,
        }
    }

    fn unseen(intro: &CharacterIntro) -> Entry {
        Entry::new(Some(Intro::new(intro, &source())))
    }

    fn key(key: Key, pressed: bool) -> Input {
        Input::Key {
            key,
            pressed,
            repeat: false,
        }
    }

    /// An OS repeat of `key`, held down.
    fn held(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: true,
        }
    }

    fn click(pressed: bool) -> Input {
        Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: Point::new(5.0, 5.0),
        }
    }

    /// The alphas of the black quads `draw` draws, in order.
    fn blacks(entry: &Entry, over: bool) -> Vec<u8> {
        let mut list = DrawList::new();
        if over {
            entry.draw_over(&mut list, VIEW);
        } else {
            entry.draw_under(&mut list, VIEW);
        }
        list.iter()
            .filter_map(|command| match command {
                DrawCommand::Line { color, .. } => Some(color.a),
                _ => None,
            })
            .collect()
    }

    /// The picture `draw_under` draws, if any.
    fn picture(entry: &Entry) -> Option<i16> {
        let mut list = DrawList::new();
        entry.draw_under(&mut list, VIEW);
        list.iter().find_map(|command| match command {
            DrawCommand::Picture { image, .. } => Some(image.id),
            _ => None,
        })
    }

    /// Ticks `entry` through both pictures, untouched.
    fn through_pictures(entry: &mut Entry) {
        for _ in 0..2 {
            entry.tick(SECOND);
            entry.tick(Duration::from_secs(2));
            entry.tick(SECOND);
        }
    }

    #[test]
    fn a_seen_pilot_fades_out_over_the_menu_then_hands_over_then_fades_flight_in() {
        let mut entry = Entry::new(None);
        assert!(entry.menu_drawn());
        assert_eq!(entry.showing(), Showing::MainMenu);
        assert_eq!(blacks(&entry, true), [] as [u8; 0], "clear at first");
        entry.tick(ms(500));
        assert_eq!(blacks(&entry, true), [128]);
        assert!(!entry.handover_due());
        entry.tick(ms(500));
        assert!(entry.handover_due());
        assert!(!entry.menu_drawn());
        assert_eq!(entry.showing(), Showing::Flight);
        assert_eq!(blacks(&entry, false), [255], "black until the handover");
        assert_eq!(entry.handed_over(), []);
        assert!(!entry.handover_due());
        assert_eq!(blacks(&entry, true), [255]);
        assert_eq!(blacks(&entry, false), [] as [u8; 0], "flight under");
        entry.tick(ms(500));
        assert_eq!(blacks(&entry, true), [128]);
        assert!(!entry.done());
        entry.tick(ms(500));
        assert!(entry.done());
        assert!(!entry.take_mark_seen(), "no intro, nothing to mark");
        assert_eq!(entry.handed_over(), [], "once");
    }

    #[test]
    fn an_unseen_pilot_sees_its_pictures_then_its_text_then_a_fade_then_flight() {
        let mut entry = unseen(&pictures(Some(128)));
        entry.tick(SECOND);
        assert_eq!(entry.showing(), Showing::Intro);
        assert_eq!(picture(&entry), Some(8200));
        assert!(entry.hides_pointer());
        assert_eq!(entry.take_text_due(), None);
        entry.tick(SECOND);
        entry.tick(Duration::from_secs(2));
        entry.tick(SECOND);
        assert_eq!(picture(&entry), Some(8201));
        entry.tick(SECOND);
        entry.tick(Duration::from_secs(2));
        entry.tick(SECOND);
        assert_eq!(entry.take_text_due(), Some(128));
        assert_eq!(entry.showing(), Showing::IntroText);
        assert!(!entry.hides_pointer());
        assert!(!entry.take_mark_seen(), "not before the text");
        entry.text_opened(true);
        assert_eq!(entry.take_text_due(), None);
        assert_eq!(blacks(&entry, false), [255], "black under the text");
        assert_eq!(blacks(&entry, true), [255], "fading in from black");
        entry.tick(ms(500));
        assert_eq!(blacks(&entry, true), [128]);
        entry.tick(ms(500));
        assert_eq!(blacks(&entry, true), [] as [u8; 0]);
        entry.tick(Duration::from_secs(60));
        assert!(!entry.handover_due(), "the text waits");
        entry.text_closed();
        assert!(entry.take_mark_seen());
        assert!(!entry.take_mark_seen(), "once");
        assert_eq!(entry.showing(), Showing::IntroText);
        entry.tick(ms(999));
        assert!(!entry.handover_due());
        entry.tick(ms(1));
        assert!(entry.handover_due());
    }

    #[test]
    fn an_empty_intro_hands_over_at_the_fades_end_and_marks_the_pilot_seen() {
        let mut entry = unseen(&CharacterIntro::EMPTY);
        entry.tick(ms(999));
        assert!(!entry.handover_due());
        entry.tick(ms(1));
        assert!(entry.handover_due(), "no second fade");
        assert!(entry.take_mark_seen());
    }

    #[test]
    fn a_skipped_intro_hands_over_with_no_text() {
        let mut entry = unseen(&pictures(Some(128)));
        entry.tick(SECOND);
        entry.tick(SECOND);
        entry.input(&key(SKIP_KEY, true), false);
        entry.input(&key(SKIP_KEY, false), false);
        entry.tick(SECOND);
        assert!(entry.handover_due());
        assert_eq!(entry.take_text_due(), None);
        assert!(entry.take_mark_seen());
    }

    #[test]
    fn the_skip_key_down_as_the_fade_ends_skips_after_the_first_picture() {
        let mut entry = unseen(&pictures(Some(128)));
        entry.input(&key(SKIP_KEY, true), false);
        entry.tick(SECOND);
        assert_eq!(picture(&entry), Some(8200));
        entry.tick(SECOND);
        entry.tick(Duration::from_secs(10));
        entry.input(&key(SKIP_KEY, false), false);
        entry.tick(SECOND);
        assert!(entry.handover_due());
    }

    #[test]
    fn pictures_with_no_text_hand_over_as_they_end() {
        let mut entry = unseen(&pictures(None));
        entry.tick(SECOND);
        through_pictures(&mut entry);
        assert!(entry.handover_due());
        assert!(entry.take_mark_seen());
    }

    #[test]
    fn a_text_that_cant_be_opened_is_passed_over() {
        let mut entry = unseen(&CharacterIntro {
            text: Some(5),
            ..CharacterIntro::EMPTY
        });
        entry.tick(SECOND);
        assert_eq!(entry.take_text_due(), Some(5));
        entry.text_opened(false);
        assert!(entry.handover_due());
        assert!(entry.take_mark_seen());
    }

    #[test]
    fn the_scene_follows_each_stage() {
        let mut entry = unseen(&pictures(Some(128)));
        let mut seen = vec![entry.showing()];
        entry.tick(SECOND);
        seen.push(entry.showing());
        through_pictures(&mut entry);
        seen.push(entry.showing());
        entry.text_opened(true);
        entry.tick(SECOND);
        entry.text_closed();
        seen.push(entry.showing());
        entry.tick(SECOND);
        entry.handed_over();
        seen.push(entry.showing());
        assert_eq!(
            seen,
            [
                Showing::MainMenu,
                Showing::Intro,
                Showing::IntroText,
                Showing::IntroText,
                Showing::Flight
            ]
        );
    }

    #[test]
    fn plain_keys_and_clicks_are_dropped_and_their_state_still_read() {
        let mut entry = unseen(&pictures(None));
        for input in [
            key(Key::Up, true),
            key(Key::Space, true),
            Input::Text(' '),
            click(true),
            click(false),
            Input::PointerMoved(Point::new(1.0, 1.0)),
        ] {
            assert_eq!(entry.input(&input, false), Verdict::Drop, "{input:?}");
        }
        entry.tick(SECOND);
        entry.tick(SECOND);
        entry.tick(Duration::from_secs(10));
        assert_eq!(picture(&entry), Some(8200), "Space still down");
        assert_eq!(entry.input(&key(Key::Space, false), false), Verdict::Drop);
        entry.tick(Duration::from_secs(2));
        entry.tick(SECOND);
        assert_eq!(picture(&entry), Some(8201));
    }

    #[test]
    fn command_chords_and_the_close_are_held_for_the_handover() {
        let mut entry = Entry::new(None);
        assert_eq!(entry.input(&key(Key::Command, true), false), Verdict::Drop);
        assert_eq!(entry.input(&key(Key::Char('q'), true), true), Verdict::Held);
        assert_eq!(entry.input(&Input::Text('q'), true), Verdict::Drop);
        assert_eq!(entry.close(), Verdict::Held);
        assert_eq!(
            entry.input(&key(Key::Char('q'), false), true),
            Verdict::Drop
        );
        assert_eq!(entry.input(&key(Key::Command, false), true), Verdict::Drop);
        entry.tick(SECOND);
        assert_eq!(
            entry.handed_over(),
            [Deferred::Command(Key::Char('q')), Deferred::Close]
        );
    }

    #[test]
    fn a_chords_repeats_and_releases_are_dropped_command_up_first_or_not() {
        let mut entry = Entry::new(None);
        assert_eq!(entry.input(&key(Key::Up, true), true), Verdict::Held);
        assert_eq!(entry.input(&held(Key::Up), true), Verdict::Drop);
        assert_eq!(entry.input(&key(Key::Up, false), true), Verdict::Drop);
        assert_eq!(entry.input(&key(Key::Escape, true), true), Verdict::Held);
        assert_eq!(entry.input(&key(Key::Command, false), false), Verdict::Drop);
        assert_eq!(entry.input(&key(Key::Escape, false), false), Verdict::Drop);
        entry.tick(SECOND);
        assert_eq!(
            entry.handed_over(),
            [Deferred::Command(Key::Up), Deferred::Command(Key::Escape)],
            "as commands, never as keys down or up"
        );
    }

    /// Whether a Command chord typed into `entry` now is held.
    fn chord_held(entry: &Entry) -> bool {
        entry.clone().input(&key(Key::Char(','), true), true) == Verdict::Held
    }

    #[test]
    fn it_says_it_holds_a_command_chord_in_each_stage_that_holds_one() {
        let mut entry = unseen(&pictures(Some(128)));
        let mut stages = Vec::new();
        for _ in 0..12 {
            stages.push((entry.showing(), entry.holds_commands()));
            assert_eq!(entry.holds_commands(), chord_held(&entry), "{stages:?}");
            if entry.take_text_due().is_some() {
                stages.push((entry.showing(), entry.holds_commands()));
                entry.text_opened(true);
                stages.push((entry.showing(), entry.holds_commands()));
                assert!(!entry.holds_commands(), "the text takes it");
                assert_eq!(entry.holds_commands(), chord_held(&entry));
                entry.text_closed();
            }
            entry.handed_over();
            entry.tick(Duration::from_secs(2));
        }
        assert!(entry.done());
        assert!(!entry.holds_commands(), "play runs");
        assert!(!chord_held(&entry));
        assert!(stages.contains(&(Showing::MainMenu, true)), "{stages:?}");
        assert!(stages.contains(&(Showing::Intro, true)), "{stages:?}");
        assert!(stages.contains(&(Showing::IntroText, false)), "{stages:?}");
    }

    #[test]
    fn the_texts_opening_drops_what_was_held() {
        let mut entry = unseen(&CharacterIntro {
            text: Some(5),
            ..CharacterIntro::EMPTY
        });
        assert_eq!(entry.input(&key(Key::Char('q'), true), true), Verdict::Held);
        entry.tick(SECOND);
        entry.text_opened(true);
        entry.tick(SECOND);
        entry.text_closed();
        entry.tick(SECOND);
        assert_eq!(entry.handed_over(), []);
    }

    #[test]
    fn a_close_is_dropped_once_an_intro_ran_and_while_its_text_shows() {
        let mut entry = unseen(&pictures(None));
        assert_eq!(entry.close(), Verdict::Held);
        assert_eq!(entry.input(&key(Key::Char(','), true), true), Verdict::Held);
        entry.tick(SECOND);
        assert_eq!(entry.close(), Verdict::Held, "during the pictures");
        through_pictures(&mut entry);
        assert_eq!(
            entry.handed_over(),
            [Deferred::Command(Key::Char(','))],
            "the closes dropped, the Command chord kept"
        );
        let mut texted = unseen(&CharacterIntro {
            text: Some(5),
            ..CharacterIntro::EMPTY
        });
        texted.tick(SECOND);
        texted.text_opened(true);
        assert_eq!(texted.close(), Verdict::Drop);
        texted.tick(SECOND);
        assert_eq!(texted.close(), Verdict::Drop);
        texted.text_closed();
        assert_eq!(texted.close(), Verdict::Held, "after the text");
        texted.tick(SECOND);
        assert_eq!(texted.handed_over(), [Deferred::Close]);
        let mut seen = Entry::new(None);
        seen.close();
        seen.tick(SECOND);
        assert_eq!(seen.handed_over(), [Deferred::Close], "no intro ran");
    }

    #[test]
    fn inputs_during_the_texts_fade_in_wait_for_it() {
        let mut entry = unseen(&CharacterIntro {
            text: Some(5),
            ..CharacterIntro::EMPTY
        });
        entry.tick(SECOND);
        assert_eq!(
            entry.input(&key(Key::Enter, true), false),
            Verdict::HeldForText
        );
        entry.text_opened(true);
        let typed = key(Key::Enter, false);
        assert_eq!(entry.input(&typed, false), Verdict::HeldForText);
        assert_eq!(entry.take_for_text(), [], "not before the fade ends");
        entry.tick(ms(999));
        assert_eq!(entry.take_for_text(), []);
        entry.tick(ms(1));
        assert_eq!(entry.take_for_text(), [key(Key::Enter, true), typed]);
        assert_eq!(entry.take_for_text(), [], "once");
        assert_eq!(entry.input(&key(Key::Char('q'), true), true), Verdict::Text);
    }

    #[test]
    fn command_keys_during_flights_fade_in_are_held_until_it_ends() {
        let mut entry = Entry::new(None);
        entry.tick(SECOND);
        entry.handed_over();
        let command_q = key(Key::Char('q'), true);
        assert_eq!(entry.input(&command_q, true), Verdict::Held);
        assert_eq!(entry.input(&key(Key::Up, true), false), Verdict::Drop);
        assert_eq!(entry.close(), Verdict::Held);
        entry.tick(ms(999));
        assert_eq!(entry.take_held(), []);
        entry.tick(ms(1));
        assert_eq!(
            entry.take_held(),
            [Deferred::Command(Key::Char('q')), Deferred::Close]
        );
        assert_eq!(entry.input(&command_q, true), Verdict::Drop, "done");
        assert_eq!(entry.close(), Verdict::Drop);
    }

    #[test]
    fn letting_go_or_cancelling_the_pointer_releases_the_tracked_keys() {
        let mut entry = unseen(&pictures(None));
        entry.input(&key(Key::Space, true), false);
        entry.input(&click(true), false);
        entry.release_keys();
        entry.tick(SECOND);
        entry.tick(SECOND);
        entry.tick(Duration::from_secs(2));
        entry.tick(SECOND);
        assert_eq!(
            picture(&entry),
            Some(8201),
            "nothing held at the fade's end"
        );
        entry.input(&click(true), false);
        entry.tick(SECOND);
        entry.tick(Duration::from_secs(5));
        assert_eq!(picture(&entry), Some(8201), "the button holds it");
        entry.cancel_pointer();
        entry.tick(Duration::from_secs(2));
        entry.tick(SECOND);
        assert!(entry.handover_due());
        let mut down = unseen(&pictures(None));
        down.input(&click(true), false);
        down.cancel_pointer();
        down.tick(SECOND);
        down.tick(SECOND);
        down.tick(Duration::from_secs(2));
        down.tick(SECOND);
        assert_eq!(picture(&down), Some(8201), "cancelled before the intro");
        let mut focus = unseen(&pictures(None));
        focus.tick(SECOND);
        focus.input(&key(Key::Enter, true), false);
        focus.tick(SECOND);
        focus.tick(Duration::from_secs(5));
        focus.release_keys();
        focus.tick(Duration::from_secs(2));
        focus.tick(SECOND);
        assert_eq!(picture(&focus), Some(8201), "let go during a picture");
    }

    #[test]
    fn the_intros_sounds_come_through() {
        let mut entry = unseen(&pictures(None));
        assert_eq!(entry.take_sounds(), []);
        entry.tick(SECOND);
        assert_eq!(entry.take_sounds(), [UiSound::IntroPicture(INTRO_SOUND)]);
        assert_eq!(Entry::new(None).take_sounds(), []);
    }

    #[test]
    fn the_picture_is_drawn_and_the_fades_are_the_second() {
        let mut entry = unseen(&pictures(None));
        entry.tick(SECOND);
        assert_eq!(
            blacks(&entry, false),
            [255, 255],
            "black, then the picture's fade"
        );
        assert_eq!(blacks(&entry, true), [] as [u8; 0]);
        assert_eq!(DEATH_FADE_FOR, SECOND);
    }

    #[test]
    fn calls_out_of_turn_change_nothing() {
        let mut entry = Entry::new(None);
        entry.text_opened(true);
        entry.text_closed();
        assert_eq!(entry.handed_over(), []);
        assert_eq!(entry.take_held(), []);
        assert!(entry.menu_drawn(), "still fading out");
        entry.tick(SECOND);
        entry.text_closed();
        assert!(entry.handover_due());
    }
}
