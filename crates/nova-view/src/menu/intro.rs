//! The intro screen a pilot sees as it enters play for the first time:
//! `_ShowIntroScreen` @0x16cf1's decisions, in the core's terms. The
//! entry ([`menu::entry`](super::entry)) runs it between the main screen's
//! fade out and the handover to flight, and the router shows its text.
//!
//! **The pictures.** A `chär` names four intro `PICT`s, each with its
//! delay in seconds, and a text (`IntroPict1-4`, `PictDelay1-4`,
//! `IntroTextID`; [`CharacterIntro`]). For each picture in turn:
//!
//! - A picture whose ID is 0 or less is passed over, with no fade, no
//!   wait and no sound (@0x16e91-0x16e95).
//! - It is drawn centred on black over the whole view, halving with a
//!   truncation toward zero as `_CenterPict` @0x8652 does; a `PICT` that
//!   can't be found leaves the black, and the wait is kept
//!   (@0x16d4e-0x16d9b).
//! - snd 30003 ([`INTRO_SOUND`]) is raised as it starts, when the data
//!   has it ([`UiSound::IntroPicture`]): `_ShowIntroScreen` loads it and,
//!   when it is there, cancels the music and plays it at priority 10
//!   (@0x16e9b-0x16ed1).
//! - It fades in from black over a second (`_FadeScreenIn`).
//! - The timer then waits until the primary button, Return, Space and the
//!   skip key are all up.
//! - It ends once its delay has passed (`delay × 60` ticks), on a Return
//!   or Space press, or on a primary press in the window
//!   (`SectRect`/`PtInRect` @0x16f3e-0x16f98, on the press). The wait
//!   compares ticks unsigned (`jae` @0x17027), so a negative delay never
//!   times out, and a delay of 0 ends as soon as the keys are up.
//! - At its end its snd is halted (@0x17064, [`UiSound::IntroPictureHalted`])
//!   and it fades out to black over a second (@0x17074).
//!
//! **The skip key** is the player's Pause key, `keyPrefs`+0x2e
//! (`pauseKey`, `_loadKeys` @0xce1ec), whose default is Escape (0x35,
//! @0xce1c9): the intro reads [`PauseCommand::Pause`]. A skip is latched
//! if the key is down as the main screen's fade ends (@0x16d05-0x16d16),
//! or at any poll of a picture's waits (@0x16fdc-0x16fed,
//! @0x16ffc-0x17012). It ends the current picture, once the keys are up,
//! and passes over the rest of the pictures and the text.
//!
//! **Return and Space** are the intro's own commands
//! ([`IntroCommand::Confirm`] and [`IntroCommand::Activate`]), read with
//! the Pause key through the bindings the keys are given
//! ([`IntroKeys::with_bindings`]; [`contexts::INTRO`]). The intro polls
//! Return's key code alone, so the keypad's Enter neither ends a picture
//! nor is waited for, and it reads none of the shared UI context's keys.
//!
//! **The text** comes after the pictures, unless skipped, when the `chär`
//! names one ([`IntroEnd::Done`]); the router loads, munges and shows it.
//!
//! The pointer is hidden while the pictures show; the original keeps it
//! shown when it runs in a window (`_settings`+0xc `RunInAWindow`
//! @0x16d1d), which the engine doesn't know here
//! (rdm:phase/front-end/phase-5-entry-left-outs).
//!
//! The original polls the keys' and the button's state. The engine tracks
//! them as down or up from the inputs ([`IntroKeys`]), and a press that
//! comes during a picture's timed wait ends it at once.

use std::time::Duration;

use crate::draw::fill_rect;
use crate::fade::{DEATH_FADE_FOR, ScreenFade, whole_view};
use crate::keymap::{
    Command, CommandInput, CommandReader, IntroCommand, PauseCommand, SharedBindings, contexts,
};
use crate::sound::{SoundId, UiSound};
use crate::{Color, DrawList, ImageKey, Input, MouseButton, Point};

/// The intro pictures' sound, snd 30003 (@0x16e9b).
pub const INTRO_SOUND: SoundId = SoundId(30_003);

/// How long each picture fades in and out: `_FadeScreenIn`'s and
/// `_FadeScreenOut`'s second.
pub const PICTURE_FADE_FOR: Duration = DEATH_FADE_FOR;

/// One of a `chär`'s intro pictures, as its fields hold it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IntroPicture {
    /// The `PICT`'s ID (`IntroPict`n); 0 or less is none.
    pub picture: i16,
    /// How many seconds it shows for (`PictDelay`n); negative never times
    /// out.
    pub delay: i16,
}

impl IntroPicture {
    /// No picture.
    pub const NONE: Self = Self {
        picture: -1,
        delay: 0,
    };
}

/// A `chär`'s intro: its four pictures and its text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CharacterIntro {
    /// The pictures, in order.
    pub pictures: [IntroPicture; 4],
    /// The text's `dësc` (`IntroTextID`), `None` for -1.
    pub text: Option<i16>,
}

impl CharacterIntro {
    /// No picture and no text: the original's intro globals as
    /// `_InitMainScreen` resets them at launch (@0x16101-0x1612d).
    pub const EMPTY: Self = Self {
        pictures: [IntroPicture::NONE; 4],
        text: None,
    };
}

/// What the intro screen needs of the game data.
pub trait IntroSource {
    /// The first `chär`'s intro, by ascending ID, or `None` with no `chär`
    /// that can be read.
    fn first_character_intro(&self) -> Option<CharacterIntro>;

    /// `PICT` `picture`'s width and height, or `None` when it can't be
    /// found or read.
    fn intro_picture_size(&self, picture: i16) -> Option<(u32, u32)>;

    /// Whether the data has `snd ` `sound`.
    fn has_sound(&self, sound: SoundId) -> bool;
}

/// The keys and button the intro waits on, down or up, as the inputs put
/// them: each key as its command is held, read through the keys'
/// bindings ([`IntroKeys::with_bindings`]).
#[derive(Clone, Debug)]
#[allow(clippy::struct_excessive_bools)] // One for each key or button, down or up.
pub struct IntroKeys {
    /// The primary button.
    pub primary: bool,
    /// Return ([`IntroCommand::Confirm`]).
    pub enter: bool,
    /// Space ([`IntroCommand::Activate`]).
    pub space: bool,
    /// The skip key ([`PauseCommand::Pause`]).
    pub skip: bool,
    /// The keys, as the intro's commands ([`contexts::INTRO`]).
    reader: CommandReader,
}

impl Default for IntroKeys {
    /// Everything up, read through the default bindings.
    fn default() -> Self {
        Self {
            primary: false,
            enter: false,
            space: false,
            skip: false,
            reader: CommandReader::new(contexts::INTRO),
        }
    }
}

impl PartialEq for IntroKeys {
    /// The same keys and button down, whatever they are read through.
    fn eq(&self, other: &Self) -> bool {
        (self.primary, self.enter, self.space, self.skip)
            == (other.primary, other.enter, other.space, other.skip)
    }
}

impl Eq for IntroKeys {}

impl IntroKeys {
    /// The keys read through `bindings`.
    #[must_use]
    pub fn with_bindings(mut self, bindings: SharedBindings) -> Self {
        self.reader.set_bindings(bindings);
        self
    }

    /// Notes what `input` does to the keys and the button, and says which
    /// command a key's press, repeat or release meant, if any. A key
    /// changing sets the three keys to their commands as held.
    pub fn note(&mut self, input: &Input) -> Option<CommandInput> {
        match *input {
            Input::Key { .. } => {
                let meant = self.reader.follow(input);
                self.enter = self.reader.held(IntroCommand::Confirm);
                self.space = self.reader.held(IntroCommand::Activate);
                self.skip = self.reader.held(PauseCommand::Pause);
                return meant;
            }
            Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                ..
            } => self.primary = pressed,
            _ => {}
        }
        None
    }

    /// Lets go of the keys and the button, keeping the bindings.
    pub fn let_go(&mut self) {
        self.reader.let_go();
        self.primary = false;
        self.enter = false;
        self.space = false;
        self.skip = false;
    }

    /// Whether any of them is down.
    #[must_use]
    pub fn any_down(&self) -> bool {
        self.primary || self.enter || self.space || self.skip
    }
}

/// How the intro ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntroEnd {
    /// The skip key passed over the rest, the text included.
    Skipped,
    /// Every picture showed; the text follows, if the `chär` names one.
    Done {
        /// The text's `dësc`.
        text: Option<i16>,
    },
}

/// A picture to show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Shown {
    picture: i16,
    /// How long it shows once the keys are up; `None` never times out.
    delay: Option<Duration>,
    /// Its size, `None` when it can't be found.
    size: Option<(u32, u32)>,
}

/// Where the intro is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    /// Not begun: the main screen still fades out.
    Waiting,
    /// A picture fades in.
    FadingIn(ScreenFade),
    /// A picture waits for the keys and the button to be up.
    AwaitingRelease,
    /// A picture's timed wait, this far in.
    Showing(Duration),
    /// A picture fades out.
    FadingOut(ScreenFade),
    /// Every picture has shown, or the rest were skipped.
    Finished,
}

/// The intro screen, running.
#[derive(Clone, Debug, PartialEq)]
pub struct Intro {
    shown: Vec<Shown>,
    /// The picture showing, an index into `shown`.
    at: usize,
    stage: Stage,
    keys: IntroKeys,
    skip: bool,
    /// The sound the picture showing raised, to halt at its end.
    sound: Option<SoundId>,
    /// Whether the data has [`INTRO_SOUND`].
    has_sound: bool,
    sounds: Vec<UiSound>,
    text: Option<i16>,
}

impl Intro {
    /// `intro`'s screen, its pictures' sizes and its sound read from
    /// `source`, waiting to begin.
    #[must_use]
    pub fn new(intro: &CharacterIntro, source: &(impl IntroSource + ?Sized)) -> Self {
        let shown = intro
            .pictures
            .iter()
            .filter(|picture| picture.picture > 0)
            .map(|picture| Shown {
                picture: picture.picture,
                delay: u64::try_from(picture.delay).ok().map(Duration::from_secs),
                size: source.intro_picture_size(picture.picture),
            })
            .collect();
        Self {
            shown,
            at: 0,
            stage: Stage::Waiting,
            keys: IntroKeys::default(),
            skip: false,
            sound: None,
            has_sound: source.has_sound(INTRO_SOUND),
            sounds: Vec::new(),
            text: intro.text,
        }
    }

    /// Whether it shows nothing at all: no picture and no text.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.shown.is_empty() && self.text.is_none()
    }

    /// The main screen's fade has ended with `keys` as they are: a skip
    /// is latched if the skip key is down, and the first picture starts.
    pub fn begin(&mut self, keys: IntroKeys) {
        if self.stage != Stage::Waiting {
            return;
        }
        self.skip = keys.skip;
        self.keys = keys;
        self.at = 0;
        self.start_picture();
    }

    /// Starts the picture at `at`, raising its sound, or finishes when
    /// there is none left.
    fn start_picture(&mut self) {
        if self.at >= self.shown.len() {
            self.stage = Stage::Finished;
            return;
        }
        if self.has_sound {
            self.sounds.push(UiSound::IntroPicture(INTRO_SOUND));
            self.sound = Some(INTRO_SOUND);
        }
        self.stage = Stage::FadingIn(ScreenFade::from_black(PICTURE_FADE_FOR));
    }

    /// Ends the picture showing: its sound halted, and its fade out
    /// begun.
    fn end_picture(&mut self) {
        if let Some(sound) = self.sound.take() {
            self.sounds.push(UiSound::IntroPictureHalted(sound));
        }
        self.stage = Stage::FadingOut(ScreenFade::to_black(PICTURE_FADE_FOR));
    }

    /// The release wait's poll: a skip latched if the skip key is down,
    /// and the timed wait begun once every key is up.
    fn poll_release(&mut self) {
        if self.keys.skip {
            self.skip = true;
        }
        if !self.keys.any_down() {
            self.stage = Stage::Showing(Duration::ZERO);
            self.poll_showing();
        }
    }

    /// The timed wait's poll: the picture ends once a skip is latched or
    /// its delay has passed.
    fn poll_showing(&mut self) {
        let Stage::Showing(elapsed) = self.stage else {
            return;
        };
        let timed_out = self.shown[self.at]
            .delay
            .is_some_and(|delay| elapsed >= delay);
        if self.skip || timed_out {
            self.end_picture();
        }
    }

    /// Takes in `input`: the keys and the button noted, and, during a
    /// picture's timed wait, a Return, Space or primary press ends it,
    /// and a skip key press latches a skip and ends it.
    pub fn input(&mut self, input: &Input) {
        let meant = self.keys.note(input);
        match self.stage {
            Stage::AwaitingRelease => self.poll_release(),
            Stage::Showing(_) => {
                let ends = match *input {
                    Input::Key { pressed: true, .. } => match meant.map(|meant| meant.command) {
                        Some(Command::Pause(PauseCommand::Pause)) => {
                            self.skip = true;
                            true
                        }
                        Some(Command::Intro(IntroCommand::Confirm | IntroCommand::Activate)) => {
                            true
                        }
                        _ => false,
                    },
                    Input::PointerButton {
                        button: MouseButton::Left,
                        pressed: true,
                        ..
                    } => true,
                    _ => false,
                };
                if ends {
                    self.end_picture();
                }
            }
            _ => {}
        }
    }

    /// The keys and the button were let go of outside the game (the
    /// window losing focus): each counts as up.
    pub fn release_keys(&mut self) {
        self.keys.let_go();
        if self.stage == Stage::AwaitingRelease {
            self.poll_release();
        }
    }

    /// The primary button was let go of outside the game (the pointer
    /// cancelled): it counts as up.
    pub fn cancel_pointer(&mut self) {
        self.keys.primary = false;
        if self.stage == Stage::AwaitingRelease {
            self.poll_release();
        }
    }

    /// `dt` passes.
    pub fn tick(&mut self, dt: Duration) {
        match &mut self.stage {
            Stage::FadingIn(fade) => {
                fade.advance(dt);
                if fade.done() {
                    self.stage = Stage::AwaitingRelease;
                    self.poll_release();
                }
            }
            Stage::AwaitingRelease => self.poll_release(),
            Stage::Showing(elapsed) => {
                *elapsed = elapsed.saturating_add(dt);
                self.poll_showing();
            }
            Stage::FadingOut(fade) => {
                fade.advance(dt);
                if fade.done() {
                    self.at += 1;
                    if self.skip {
                        self.stage = Stage::Finished;
                    } else {
                        self.start_picture();
                    }
                }
            }
            Stage::Waiting | Stage::Finished => {}
        }
    }

    /// How it ended, once it has.
    #[must_use]
    pub fn end(&self) -> Option<IntroEnd> {
        (self.stage == Stage::Finished).then_some(if self.skip {
            IntroEnd::Skipped
        } else {
            IntroEnd::Done { text: self.text }
        })
    }

    /// Whether the pointer is hidden: while a picture shows.
    #[must_use]
    pub fn hides_pointer(&self) -> bool {
        !matches!(self.stage, Stage::Waiting | Stage::Finished)
    }

    /// The sounds raised since they were last taken.
    pub fn take_sounds(&mut self) -> Vec<UiSound> {
        std::mem::take(&mut self.sounds)
    }

    /// Covers a view `view` in size in black, with the picture showing
    /// centred on it under its fade.
    pub fn draw(&self, list: &mut DrawList, view: (f32, f32)) {
        fill_rect(list, whole_view(view), Color::BLACK);
        let fade = match self.stage {
            Stage::FadingIn(fade) | Stage::FadingOut(fade) => Some(fade),
            Stage::AwaitingRelease | Stage::Showing(_) => None,
            Stage::Waiting | Stage::Finished => return,
        };
        if let Some(size) = self.shown[self.at].size {
            list.picture(
                ImageKey::picture(self.shown[self.at].picture),
                centred(view, size),
            );
        }
        if let Some(fade) = fade {
            fade.draw(list, view);
        }
    }
}

/// Where a picture `size` goes centred in a view `view` in size, halving
/// with a truncation toward zero, as `_CenterPict` @0x8652 does.
fn centred(view: (f32, f32), size: (u32, u32)) -> Point {
    // Whole logical units: the view's size truncates as the port's does.
    #[allow(clippy::cast_possible_truncation)]
    let port = (view.0 as i64, view.1 as i64);
    let left = (port.0 - i64::from(size.0)) / 2;
    let top = (port.1 - i64::from(size.1)) / 2;
    // Within a screen's size, so exact in an f32.
    #[allow(clippy::cast_precision_loss)]
    Point::new(left as f32, top as f32)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::RefCell;
    use std::collections::BTreeMap;

    use super::*;
    use crate::{DrawCommand, Key};

    const VIEW: (f32, f32) = (1024.0, 768.0);

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    fn secs(secs: u64) -> Duration {
        Duration::from_secs(secs)
    }

    /// A source with `PICT`s of the sizes given and, or not, snd 30003,
    /// recording the `PICT`s whose size it was asked.
    #[derive(Default)]
    struct Source {
        sizes: BTreeMap<i16, (u32, u32)>,
        sound: bool,
        asked: RefCell<Vec<i16>>,
    }

    impl IntroSource for Source {
        fn first_character_intro(&self) -> Option<CharacterIntro> {
            None
        }

        fn intro_picture_size(&self, picture: i16) -> Option<(u32, u32)> {
            self.asked.borrow_mut().push(picture);
            self.sizes.get(&picture).copied()
        }

        fn has_sound(&self, sound: SoundId) -> bool {
            self.sound && sound == INTRO_SOUND
        }
    }

    fn source(sizes: &[(i16, (u32, u32))], sound: bool) -> Source {
        Source {
            sizes: sizes.iter().copied().collect(),
            sound,
            asked: RefCell::new(Vec::new()),
        }
    }

    fn picture(picture: i16, delay: i16) -> IntroPicture {
        IntroPicture { picture, delay }
    }

    fn intro(pictures: [IntroPicture; 4], text: Option<i16>) -> CharacterIntro {
        CharacterIntro { pictures, text }
    }

    /// Two 640 x 480 pictures, 8200 for 2 s and 8201 for 3 s, and text 128.
    fn two_pictures() -> CharacterIntro {
        intro(
            [
                picture(8200, 2),
                picture(-1, 9),
                picture(8201, 3),
                IntroPicture::NONE,
            ],
            Some(128),
        )
    }

    fn sized() -> Source {
        source(&[(8200, (640, 480)), (8201, (640, 480))], false)
    }

    fn begun(intro: &CharacterIntro, source: &Source, keys: IntroKeys) -> Intro {
        let mut screen = Intro::new(intro, source);
        screen.begin(keys);
        screen
    }

    fn key(key: Key, pressed: bool) -> Input {
        Input::Key {
            key,
            pressed,
            repeat: false,
        }
    }

    fn click(pressed: bool) -> Input {
        Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: Point::new(10.0, 10.0),
        }
    }

    /// The picture drawn and the fade's alpha over it, if any.
    fn drawn(screen: &Intro) -> (Option<(i16, Point)>, Option<u8>) {
        let mut list = DrawList::new();
        screen.draw(&mut list, VIEW);
        let commands: Vec<&DrawCommand> = list.iter().collect();
        let mut picture = None;
        let mut fade = None;
        for (i, command) in commands.iter().enumerate() {
            match command {
                DrawCommand::Line { color, width, .. } => {
                    assert_eq!((color.r, color.g, color.b), (0, 0, 0));
                    assert_eq!(*width, VIEW.1, "the whole view");
                    if i == 0 {
                        assert_eq!(color.a, 255, "black under everything");
                    } else {
                        fade = Some(color.a);
                    }
                }
                DrawCommand::Picture { image, top_left } => {
                    picture = Some((image.id, *top_left));
                }
                other => panic!("{other:?}"),
            }
        }
        (picture, fade)
    }

    /// Ticks `screen` through a picture's fade in.
    fn faded_in(screen: &mut Intro) {
        screen.tick(PICTURE_FADE_FOR);
    }

    #[test]
    fn pictures_of_id_0_or_less_are_passed_over_and_the_rest_come_in_order() {
        let data = sized();
        let mut screen = begun(&two_pictures(), &data, IntroKeys::default());
        assert_eq!(*data.asked.borrow(), [8200, 8201], "only those shown");
        assert_eq!(drawn(&screen).0.map(|(id, _)| id), Some(8200));
        faded_in(&mut screen);
        screen.tick(secs(2));
        screen.tick(PICTURE_FADE_FOR);
        assert_eq!(drawn(&screen).0.map(|(id, _)| id), Some(8201));
        faded_in(&mut screen);
        screen.tick(secs(3));
        assert_eq!(screen.end(), None, "fading out");
        screen.tick(PICTURE_FADE_FOR);
        assert_eq!(screen.end(), Some(IntroEnd::Done { text: Some(128) }));
    }

    #[test]
    fn no_picture_finishes_at_once_with_its_text() {
        let none = intro(
            [
                picture(0, 5),
                picture(-3, 5),
                IntroPicture::NONE,
                picture(-1, 1),
            ],
            Some(7),
        );
        let data = source(&[], true);
        let mut screen = begun(&none, &data, IntroKeys::default());
        assert_eq!(screen.end(), Some(IntroEnd::Done { text: Some(7) }));
        assert_eq!(screen.take_sounds(), [], "no sound");
        assert!(!screen.hides_pointer());
        assert!(!Intro::new(&none, &data).is_empty(), "it has a text");
        assert!(Intro::new(&CharacterIntro::EMPTY, &data).is_empty());
        assert!(!Intro::new(&two_pictures(), &data).is_empty());
    }

    #[test]
    fn a_picture_is_centred_on_black_truncating_and_a_missing_one_is_black_with_its_wait() {
        let data = source(&[(8200, (641, 481))], false);
        let mut screen = begun(&two_pictures(), &data, IntroKeys::default());
        assert_eq!(drawn(&screen).0, Some((8200, Point::new(191.0, 143.0))));
        faded_in(&mut screen);
        screen.tick(secs(2));
        screen.tick(PICTURE_FADE_FOR);
        assert_eq!(drawn(&screen), (None, Some(255)), "8201 is missing");
        faded_in(&mut screen);
        assert_eq!(drawn(&screen), (None, None));
        screen.tick(ms(2999));
        assert_eq!(screen.end(), None, "its wait is kept");
        screen.tick(ms(1));
        screen.tick(PICTURE_FADE_FOR);
        assert!(screen.end().is_some());
        let big = source(&[(8200, (1027, 771))], false);
        let screen = begun(&two_pictures(), &big, IntroKeys::default());
        assert_eq!(drawn(&screen).0, Some((8200, Point::new(-1.0, -1.0))));
    }

    #[test]
    fn each_picture_fades_in_over_a_second_and_out_back_to_black() {
        let data = sized();
        let mut screen = begun(&two_pictures(), &data, IntroKeys::default());
        assert_eq!(drawn(&screen).1, Some(255));
        screen.tick(ms(500));
        assert_eq!(drawn(&screen).1, Some(128));
        screen.tick(ms(500));
        assert_eq!(drawn(&screen).1, None, "shown in full");
        screen.tick(secs(2));
        assert_eq!(drawn(&screen).1, None, "its fade out has just begun");
        screen.tick(ms(500));
        assert_eq!(
            drawn(&screen),
            (Some((8200, Point::new(192.0, 144.0))), Some(128))
        );
        screen.tick(ms(500));
        assert_eq!(drawn(&screen).0.map(|(id, _)| id), Some(8201));
        assert_eq!(drawn(&screen).1, Some(255), "the next fades in from black");
    }

    #[test]
    fn a_picture_ends_at_its_delay_not_a_tick_before() {
        let data = sized();
        let mut screen = begun(&two_pictures(), &data, IntroKeys::default());
        faded_in(&mut screen);
        screen.tick(ms(1999));
        assert_eq!(drawn(&screen).1, None, "still showing");
        screen.tick(ms(1));
        screen.tick(ms(500));
        assert_eq!(drawn(&screen).1, Some(128), "fading out");
    }

    #[test]
    fn the_timer_waits_for_every_key_and_the_button_to_be_up() {
        // The skip key's hold is
        // `the_skip_key_down_as_it_begins_shows_the_first_picture_until_its_release`.
        for held in [click(true), key(Key::Enter, true), key(Key::Space, true)] {
            let data = sized();
            let mut screen = begun(&two_pictures(), &data, IntroKeys::default());
            screen.input(&held);
            faded_in(&mut screen);
            screen.tick(secs(10));
            assert_eq!(drawn(&screen).1, None, "{held:?}: no timer while held");
            let released = match held {
                Input::Key { key, .. } => self::key(key, false),
                _ => click(false),
            };
            screen.input(&released);
            screen.tick(ms(1999));
            assert_eq!(drawn(&screen).1, None, "{held:?}: timed from the release");
            screen.tick(ms(1));
            screen.tick(ms(500));
            assert_eq!(drawn(&screen).1, Some(128), "{held:?}: then ends");
        }
    }

    #[test]
    fn a_press_after_the_release_wait_ends_the_picture_early() {
        for press in [key(Key::Enter, true), key(Key::Space, true), click(true)] {
            let data = sized();
            let mut screen = begun(&two_pictures(), &data, IntroKeys::default());
            faded_in(&mut screen);
            screen.tick(ms(100));
            screen.input(&press);
            screen.tick(ms(500));
            assert_eq!(drawn(&screen).1, Some(128), "{press:?} ends it");
            screen.tick(ms(500));
            assert_eq!(drawn(&screen).0.map(|(id, _)| id), Some(8201));
            assert_eq!(screen.end(), None, "{press:?} skips nothing");
        }
    }

    /// `_ShowIntroScreen` polls Return's key code alone (`_KeyCheck(0x24)`
    /// @0x16ef2, @0x16fc3, @0x17034), so the keypad's Enter neither ends a
    /// picture nor is waited for.
    #[test]
    fn the_keypads_enter_neither_ends_a_picture_nor_is_waited_for() {
        let data = sized();
        let mut screen = begun(&two_pictures(), &data, IntroKeys::default());
        faded_in(&mut screen);
        screen.tick(ms(100));
        screen.input(&key(Key::NumpadEnter, true));
        screen.tick(ms(500));
        assert_eq!(drawn(&screen).1, None, "it doesn't end the picture");
        let mut keys = IntroKeys::default();
        keys.note(&key(Key::NumpadEnter, true));
        assert_eq!(keys, IntroKeys::default(), "nothing to wait for");
    }

    #[test]
    fn a_key_held_through_the_fade_in_only_delays_the_timer() {
        let data = sized();
        let held = IntroKeys {
            enter: true,
            ..IntroKeys::default()
        };
        let mut screen = begun(&two_pictures(), &data, held);
        faded_in(&mut screen);
        screen.input(&key(Key::Enter, true));
        screen.tick(secs(30));
        assert_eq!(drawn(&screen).1, None, "never ends by itself");
        screen.input(&key(Key::Enter, false));
        screen.tick(ms(1999));
        assert_eq!(drawn(&screen).1, None);
    }

    #[test]
    fn a_negative_delay_never_times_out_but_a_key_advances_it() {
        let forever = intro(
            [
                picture(8200, -1),
                IntroPicture::NONE,
                IntroPicture::NONE,
                IntroPicture::NONE,
            ],
            None,
        );
        let data = sized();
        let mut screen = begun(&forever, &data, IntroKeys::default());
        faded_in(&mut screen);
        screen.tick(secs(100_000));
        assert_eq!(drawn(&screen).1, None);
        screen.input(&key(Key::Space, true));
        screen.tick(PICTURE_FADE_FOR);
        assert_eq!(screen.end(), Some(IntroEnd::Done { text: None }));
    }

    #[test]
    fn a_delay_of_0_ends_as_soon_as_the_keys_are_up() {
        let at_once = intro(
            [
                picture(8200, 0),
                IntroPicture::NONE,
                IntroPicture::NONE,
                IntroPicture::NONE,
            ],
            None,
        );
        let data = sized();
        let mut screen = begun(&at_once, &data, IntroKeys::default());
        faded_in(&mut screen);
        screen.tick(ms(500));
        assert_eq!(drawn(&screen).1, Some(128), "fading out at once");
        let mut held = begun(&at_once, &data, IntroKeys::default());
        held.input(&click(true));
        faded_in(&mut held);
        held.tick(secs(5));
        assert_eq!(drawn(&held).1, None);
        held.input(&click(false));
        held.tick(ms(500));
        assert_eq!(drawn(&held).1, Some(128), "on the release");
    }

    #[test]
    fn the_skip_key_pressed_during_a_wait_ends_the_picture_and_skips_the_rest() {
        let data = sized();
        let mut screen = begun(&two_pictures(), &data, IntroKeys::default());
        faded_in(&mut screen);
        screen.tick(ms(100));
        screen.input(&key(Key::Escape, true));
        screen.tick(ms(500));
        assert_eq!(drawn(&screen).1, Some(128), "fading out");
        screen.tick(ms(500));
        assert_eq!(screen.end(), Some(IntroEnd::Skipped), "no 8201, no text");
    }

    #[test]
    fn a_rebound_pause_and_confirm_act_on_their_new_keys_alone() {
        use crate::keymap::{Bindings, IntroCommand, KeyCombo, PauseCommand, SharedBindings};
        let bindings = SharedBindings::new(
            Bindings::default()
                .rebound(PauseCommand::Pause, KeyCombo::key(Key::KeyQ))
                .rebound(IntroCommand::Confirm, KeyCombo::key(Key::KeyJ)),
        );
        let data = sized();
        let keys = IntroKeys::default().with_bindings(bindings);
        let mut screen = begun(&two_pictures(), &data, keys.clone());
        faded_in(&mut screen);
        screen.tick(ms(100));
        screen.input(&key(Key::Escape, true));
        screen.input(&key(Key::Enter, true));
        screen.tick(ms(500));
        assert_eq!(drawn(&screen).1, None, "neither Escape nor Return ends it");
        screen.input(&key(Key::KeyJ, true));
        screen.tick(ms(500));
        assert_eq!(drawn(&screen).1, Some(128), "J ends it");
        screen.tick(ms(500));
        assert_eq!(
            drawn(&screen).0.map(|(id, _)| id),
            Some(8201),
            "J skips nothing"
        );
        let mut screen = begun(&two_pictures(), &data, keys);
        faded_in(&mut screen);
        screen.input(&key(Key::KeyQ, true));
        screen.tick(PICTURE_FADE_FOR);
        assert_eq!(screen.end(), Some(IntroEnd::Skipped), "Q skips");
    }

    #[test]
    fn the_skip_key_down_as_it_begins_shows_the_first_picture_until_its_release() {
        let data = sized();
        let held = IntroKeys {
            skip: true,
            ..IntroKeys::default()
        };
        let mut screen = begun(&two_pictures(), &data, held);
        assert_eq!(drawn(&screen).0.map(|(id, _)| id), Some(8200));
        faded_in(&mut screen);
        screen.tick(secs(10));
        assert_eq!(drawn(&screen).1, None, "waits for the release");
        screen.input(&key(Key::Escape, false));
        screen.tick(ms(500));
        assert_eq!(drawn(&screen).1, Some(128));
        screen.tick(ms(500));
        assert_eq!(screen.end(), Some(IntroEnd::Skipped));
    }

    #[test]
    fn the_skip_key_held_into_the_release_wait_latches_a_skip() {
        let data = sized();
        let mut screen = begun(&two_pictures(), &data, IntroKeys::default());
        screen.input(&key(Key::Escape, true));
        faded_in(&mut screen);
        screen.input(&key(Key::Escape, false));
        screen.tick(PICTURE_FADE_FOR);
        assert_eq!(screen.end(), Some(IntroEnd::Skipped));
    }

    #[test]
    fn the_skip_key_down_with_no_picture_skips_the_text() {
        let text_only = intro([IntroPicture::NONE; 4], Some(128));
        let data = sized();
        let held = IntroKeys {
            skip: true,
            ..IntroKeys::default()
        };
        assert_eq!(
            begun(&text_only, &data, held).end(),
            Some(IntroEnd::Skipped)
        );
    }

    #[test]
    fn the_sound_starts_with_each_picture_and_halts_at_its_end_when_the_data_has_it() {
        let data = source(&[(8200, (640, 480))], true);
        let mut screen = Intro::new(&two_pictures(), &data);
        assert_eq!(screen.take_sounds(), [], "not before it begins");
        screen.begin(IntroKeys::default());
        assert_eq!(screen.take_sounds(), [UiSound::IntroPicture(INTRO_SOUND)]);
        faded_in(&mut screen);
        screen.tick(secs(2));
        assert_eq!(
            screen.take_sounds(),
            [UiSound::IntroPictureHalted(INTRO_SOUND)]
        );
        screen.tick(PICTURE_FADE_FOR);
        assert_eq!(screen.take_sounds(), [UiSound::IntroPicture(INTRO_SOUND)]);
        let quiet = sized();
        let mut screen = begun(&two_pictures(), &quiet, IntroKeys::default());
        faded_in(&mut screen);
        screen.tick(secs(2));
        assert_eq!(screen.take_sounds(), [], "none without snd 30003");
    }

    #[test]
    fn it_finishes_with_the_text_only_when_not_skipped_and_named() {
        let data = sized();
        let no_text = intro(two_pictures().pictures, None);
        let mut screen = begun(&no_text, &data, IntroKeys::default());
        for _ in 0..2 {
            faded_in(&mut screen);
            screen.tick(secs(3));
            screen.tick(PICTURE_FADE_FOR);
        }
        assert_eq!(screen.end(), Some(IntroEnd::Done { text: None }));
    }

    #[test]
    fn it_hides_the_pointer_while_the_pictures_show() {
        let data = sized();
        let mut screen = Intro::new(&two_pictures(), &data);
        assert!(!screen.hides_pointer(), "not begun");
        screen.begin(IntroKeys::default());
        assert!(screen.hides_pointer());
        faded_in(&mut screen);
        screen.input(&key(Key::Escape, true));
        assert!(screen.hides_pointer(), "fading out");
        screen.tick(PICTURE_FADE_FOR);
        assert!(!screen.hides_pointer(), "finished");
    }

    #[test]
    fn letting_go_of_the_keys_counts_as_their_release() {
        let data = sized();
        let mut screen = begun(&two_pictures(), &data, IntroKeys::default());
        screen.input(&key(Key::Space, true));
        screen.input(&click(true));
        faded_in(&mut screen);
        screen.tick(secs(5));
        screen.release_keys();
        screen.tick(ms(1999));
        assert_eq!(drawn(&screen).1, None, "timed from the letting go");
        screen.tick(ms(1));
        screen.tick(ms(500));
        assert_eq!(drawn(&screen).1, Some(128));
    }

    #[test]
    fn cancelling_the_pointer_lets_go_of_the_button_alone() {
        let data = sized();
        let mut screen = begun(&two_pictures(), &data, IntroKeys::default());
        screen.input(&click(true));
        faded_in(&mut screen);
        screen.tick(secs(5));
        screen.cancel_pointer();
        screen.tick(ms(1999));
        assert_eq!(drawn(&screen).1, None, "timed from the cancel");
        screen.tick(ms(1));
        screen.tick(ms(500));
        assert_eq!(drawn(&screen).1, Some(128));
        let mut space = begun(&two_pictures(), &data, IntroKeys::default());
        space.input(&click(true));
        space.input(&key(Key::Space, true));
        faded_in(&mut space);
        space.cancel_pointer();
        space.tick(secs(5));
        assert_eq!(drawn(&space).1, None, "Space is still down");
    }

    #[test]
    fn begin_runs_once_and_nothing_moves_before_it() {
        let data = sized();
        let mut screen = Intro::new(&two_pictures(), &data);
        screen.tick(secs(100));
        screen.input(&key(Key::Enter, true));
        let mut list = DrawList::new();
        screen.draw(&mut list, VIEW);
        assert_eq!(list.len(), 1, "black alone before it begins");
        assert_eq!(screen.end(), None);
        screen.begin(IntroKeys::default());
        faded_in(&mut screen);
        screen.tick(ms(100));
        screen.begin(IntroKeys {
            skip: true,
            ..IntroKeys::default()
        });
        screen.tick(ms(1900));
        screen.tick(PICTURE_FADE_FOR);
        assert_eq!(
            drawn(&screen).0.map(|(id, _)| id),
            Some(8201),
            "the second begin changed nothing"
        );
    }

    #[test]
    fn the_keys_follow_their_presses_and_releases() {
        let mut keys = IntroKeys::default();
        assert!(!keys.any_down());
        for (input, want) in [
            (
                key(Key::Enter, true),
                IntroKeys {
                    enter: true,
                    ..IntroKeys::default()
                },
            ),
            (
                key(Key::Space, true),
                IntroKeys {
                    space: true,
                    ..IntroKeys::default()
                },
            ),
            (
                key(Key::Escape, true),
                IntroKeys {
                    skip: true,
                    ..IntroKeys::default()
                },
            ),
            (
                click(true),
                IntroKeys {
                    primary: true,
                    ..IntroKeys::default()
                },
            ),
        ] {
            let mut noted = IntroKeys::default();
            noted.note(&input);
            assert_eq!(noted, want, "{input:?}");
            assert!(noted.any_down());
            keys.note(&input);
        }
        keys.note(&key(Key::ArrowUp, true));
        keys.note(&Input::PointerButton {
            button: MouseButton::Right,
            pressed: false,
            at: Point::new(0.0, 0.0),
        });
        assert!(keys.primary && keys.enter && keys.space && keys.skip);
        for input in [
            key(Key::Enter, false),
            key(Key::Space, false),
            key(Key::Escape, false),
            click(false),
        ] {
            keys.note(&input);
        }
        assert!(!keys.any_down());
    }

    #[test]
    fn the_skip_key_is_escape_the_sound_30003_and_the_fades_a_second() {
        let skip = crate::keymap::KeyMap::default().combo(PauseCommand::Pause.into());
        assert_eq!(skip.map(|combo| combo.key), Some(Key::Escape));
        assert_eq!(INTRO_SOUND, SoundId(30_003));
        assert_eq!(PICTURE_FADE_FOR, secs(1));
        assert_eq!(
            CharacterIntro::EMPTY,
            CharacterIntro {
                pictures: [IntroPicture {
                    picture: -1,
                    delay: 0
                }; 4],
                text: None
            }
        );
    }
}
