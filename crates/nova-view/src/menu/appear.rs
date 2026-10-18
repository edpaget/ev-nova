//! How the main screen appears: at start-up, and on each return from
//! play. The router shows the [`MainMenu`](super::MainMenu) behind an
//! [`Appear`] until it ends, and gives it every input meanwhile; the
//! `Appear` decides what is dropped, what waits for the screen, and what
//! the screen takes at once.
//!
//! **At start-up** (`_DoMainScreen` @0x1b0b8-0x1b190) the main screen is
//! drawn with no row slid in yet (`_MainScreenAppearStart` @0x1b0d8) and
//! the display fades in from black over it in a second (`_FadeScreenIn`
//! @0x1b126), nothing on it moving. The fade is synchronous, so what the
//! player does meanwhile waits in the queue for the drain after it,
//! `_IterateInGameEventLoop(50)` @0x1b190, which reads it through the
//! in-game handler: a click in the window does nothing, a plain key does
//! nothing, a Command key goes to `MenuKey` (@0x56a6b-0x56a86), which
//! runs its menu command (Quit, Preferences) as the main loop's does, and
//! the close box asks `STR#` 140 #10, `_gameLoaded` being set
//! (@0x569ef-0x56a55, @0x47411). So the start-up drops clicks and plain
//! keys, and holds Command keys and the window's close for when the fade
//! ends.
//!
//! **On a return**, by the engine's reading of
//! [`RuleKey::MainScreenReturnFade`](nova_sim::RuleKey::MainScreenReturnFade)
//! (`_EnterGameFromMainScreen` @0x1a4ff-0x1a6ff): the window is painted
//! black, and it stays black while the primary button is held (the
//! `StillDown` spin @0x1a59a-0x1a5a6) and through the second's fade,
//! which fades in over the window painted black (@0x1a60d-0x1a63b,
//! `_FadeScreenIn` @0x1a673); only then does the main screen appear
//! (`_MainScreenAppearStart` @0x1a678). Its flushes, `FlushEvents` 3, 5,
//! 1 and 2 before the fade and after it (@0x1a5a8-0x1a5f3,
//! @0x1a6af-0x1a6fa), drop null, mouseDown and mouseUp events alone, so
//! the keys typed meanwhile reach the main screen after the fade,
//! Command-Q's quit among them, and every pointer event is dropped, the
//! close box's click included. The return's hold begins at game over:
//! what was typed during flight's fade to black is flushed as `_PlayGame`
//! ends (`FlushEvents(0xff7f)` @0x46618), and the engine gives that to
//! flight. By the other reading the main screen shows at once under the
//! fade from black and takes everything.
//!
//! The pointer stays shown throughout: `_DoMainScreen` shows it before
//! its fade, and `_PlayGame`'s exit before the return (`_ShowTheCursor`
//! @0x465fc).

use std::time::Duration;

use crate::draw::fill_rect;
use crate::fade::{DEATH_FADE_FOR, ScreenFade, whole_view};
use crate::{Color, DrawList, Input, Key, MouseButton};

/// What an [`Appear`] does with an input or a close.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// The screen takes it now, as it would with no appear.
    Pass,
    /// It is dropped.
    Drop,
    /// It waits, kept by the appear, for when the appear ends
    /// ([`Appear::take_held`]).
    Held,
}

/// Something an [`Appear`] kept for when it ends.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Deferred {
    /// An input.
    Input(Input),
    /// The window's close.
    Close,
}

/// Which appear this is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    /// The game's start, fading in over the main screen.
    StartUp,
    /// A return from play, black until the fade ends.
    Return,
    /// A return from play by the other reading, fading in over the main
    /// screen, which takes everything.
    LiveReturn,
}

/// Where an appear is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    /// Waiting for the primary button to be let go of (`StillDown`).
    AwaitingRelease,
    /// The fade from black.
    Fading(ScreenFade),
}

/// The main screen appearing: its fade, and what it does with the input
/// meanwhile.
#[derive(Clone, Debug, PartialEq)]
pub struct Appear {
    kind: Kind,
    stage: Stage,
    held: Vec<Deferred>,
}

impl Appear {
    /// The game's start: a second's fade from black over the main screen.
    #[must_use]
    pub fn start_up() -> Self {
        Self {
            kind: Kind::StartUp,
            stage: fading(),
            held: Vec::new(),
        }
    }

    /// A return from play: by the engine's reading (`menu_live` false),
    /// black while the primary button is down (`button_down`) and then
    /// through the second's fade; by the other, a second's fade over the
    /// main screen, which takes everything.
    #[must_use]
    pub fn returning(menu_live: bool, button_down: bool) -> Self {
        let (kind, stage) = match (menu_live, button_down) {
            (true, _) => (Kind::LiveReturn, fading()),
            (false, true) => (Kind::Return, Stage::AwaitingRelease),
            (false, false) => (Kind::Return, fading()),
        };
        Self {
            kind,
            stage,
            held: Vec::new(),
        }
    }

    /// The primary button was let go of, wherever: a wait for it ends and
    /// the fade begins.
    pub fn released(&mut self) {
        if self.stage == Stage::AwaitingRelease {
            self.stage = fading();
        }
    }

    /// `dt` passes: the fade moves on, unless the appear waits for the
    /// button. Whether the appear has ended.
    pub fn tick(&mut self, dt: Duration) -> bool {
        match &mut self.stage {
            Stage::AwaitingRelease => false,
            Stage::Fading(fade) => {
                fade.advance(dt);
                fade.done()
            }
        }
    }

    /// What becomes of `input`, Command being held as `command_held`
    /// says. A held one is kept, in order. A primary release while the
    /// appear waits for it ends the wait, and is dropped.
    pub fn input(&mut self, input: &Input, command_held: bool) -> Verdict {
        let verdict = match (self.kind, input) {
            (Kind::LiveReturn, _) => Verdict::Pass,
            (Kind::StartUp, Input::Key { key, .. }) if *key == Key::Command || command_held => {
                Verdict::Held
            }
            (Kind::Return, Input::Key { .. } | Input::Text(_)) => Verdict::Held,
            (
                _,
                &Input::PointerButton {
                    button: MouseButton::Left,
                    pressed: false,
                    ..
                },
            ) => {
                self.released();
                Verdict::Drop
            }
            _ => Verdict::Drop,
        };
        if verdict == Verdict::Held {
            self.held.push(Deferred::Input(*input));
        }
        verdict
    }

    /// What becomes of the window's close: held at start-up, dropped on a
    /// return, and passed by the other reading.
    pub fn close(&mut self) -> Verdict {
        match self.kind {
            Kind::StartUp => {
                self.held.push(Deferred::Close);
                Verdict::Held
            }
            Kind::Return => Verdict::Drop,
            Kind::LiveReturn => Verdict::Pass,
        }
    }

    /// What was held, in order, once.
    pub fn take_held(&mut self) -> Vec<Deferred> {
        std::mem::take(&mut self.held)
    }

    /// Whether the main screen runs under the appear, ticking and taking
    /// input, as by the other reading; otherwise it waits unseen.
    #[must_use]
    pub fn menu_live(&self) -> bool {
        self.kind == Kind::LiveReturn
    }

    /// Covers a view `view` in size as the appear shows: its fade over
    /// whatever is drawn under it, or solid black while a return waits
    /// or fades.
    pub fn draw(&self, list: &mut DrawList, view: (f32, f32)) {
        match (self.kind, &self.stage) {
            (Kind::StartUp | Kind::LiveReturn, Stage::Fading(fade)) => fade.draw(list, view),
            _ => fill_rect(list, whole_view(view), Color::BLACK),
        }
    }
}

/// The second's fade from black, `_FadeScreenIn`'s (@0x542d).
fn fading() -> Stage {
    Stage::Fading(ScreenFade::from_black(DEATH_FADE_FOR))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Point};

    const VIEW: (f32, f32) = (1024.0, 768.0);

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    /// The black's alpha the appear draws, if it draws one.
    fn black(appear: &Appear) -> Option<u8> {
        let mut list = DrawList::new();
        appear.draw(&mut list, VIEW);
        let commands: Vec<&DrawCommand> = list.iter().collect();
        match commands[..] {
            [] => None,
            [DrawCommand::Line { color, .. }] => {
                assert_eq!((color.r, color.g, color.b), (0, 0, 0));
                Some(color.a)
            }
            _ => panic!("{commands:?}"),
        }
    }

    /// The alphas the appear draws as it is ticked by `steps`, and
    /// whether each tick ended it.
    fn alphas(appear: &mut Appear, steps: &[u64]) -> Vec<(Option<u8>, bool)> {
        steps
            .iter()
            .map(|&step| {
                let done = appear.tick(ms(step));
                (black(appear), done)
            })
            .collect()
    }

    fn key(key: Key, pressed: bool) -> Input {
        Input::Key {
            key,
            pressed,
            repeat: false,
        }
    }

    fn button(button: MouseButton, pressed: bool) -> Input {
        Input::PointerButton {
            button,
            pressed,
            at: Point::new(5.0, 5.0),
        }
    }

    #[test]
    fn the_start_up_fade_falls_from_black_over_a_second() {
        let mut appear = Appear::start_up();
        assert_eq!(black(&appear), Some(255));
        assert!(!appear.menu_live());
        assert_eq!(
            alphas(&mut appear, &[500, 499, 1]),
            [(Some(128), false), (None, false), (None, true)]
        );
    }

    #[test]
    fn a_return_is_solid_black_until_its_fade_ends() {
        let mut appear = Appear::returning(false, false);
        assert_eq!(black(&appear), Some(255));
        assert!(!appear.menu_live());
        assert_eq!(
            alphas(&mut appear, &[500, 499, 1]),
            [(Some(255), false), (Some(255), false), (Some(255), true)]
        );
    }

    #[test]
    fn with_a_live_menu_the_return_fades_over_it() {
        let mut appear = Appear::returning(true, true);
        assert!(appear.menu_live());
        assert_eq!(black(&appear), Some(255));
        assert_eq!(
            alphas(&mut appear, &[500, 500]),
            [(Some(128), false), (None, true)],
            "no wait for the button"
        );
    }

    #[test]
    fn awaiting_the_release_holds_the_fade_at_black() {
        let mut appear = Appear::returning(false, true);
        assert_eq!(
            alphas(&mut appear, &[5000, 5000]),
            [(Some(255), false), (Some(255), false)]
        );
    }

    #[test]
    fn the_release_starts_the_fade() {
        let mut appear = Appear::returning(false, true);
        appear.tick(ms(700));
        for other in [MouseButton::Right, MouseButton::Middle, MouseButton::Other] {
            assert_eq!(appear.input(&button(other, false), false), Verdict::Drop);
        }
        assert_eq!(
            appear.input(&button(MouseButton::Left, true), false),
            Verdict::Drop
        );
        assert!(!appear.tick(ms(5000)), "still waiting");
        assert_eq!(
            appear.input(&button(MouseButton::Left, false), false),
            Verdict::Drop
        );
        assert!(!appear.tick(ms(999)), "the whole second from the release");
        assert!(appear.tick(ms(1)));

        let mut appear = Appear::returning(false, true);
        appear.released();
        assert!(!appear.tick(ms(999)));
        assert!(appear.tick(ms(1)), "a release outside the game");
        assert_eq!(appear.take_held(), [], "a release is never held");
    }

    #[test]
    fn a_release_once_fading_changes_nothing() {
        let mut appear = Appear::returning(false, false);
        appear.tick(ms(600));
        appear.released();
        assert_eq!(
            appear.input(&button(MouseButton::Left, false), false),
            Verdict::Drop
        );
        assert!(appear.tick(ms(400)), "the fade goes on from where it was");
    }

    /// Every kind of input, a Command key's included.
    fn every_input() -> Vec<(Input, bool)> {
        vec![
            (key(Key::Char('n'), true), false),
            (key(Key::Char('n'), false), false),
            (Input::Text('n'), false),
            (key(Key::Escape, true), false),
            (key(Key::Command, true), false),
            (key(Key::Char('q'), true), true),
            (key(Key::Char('q'), false), true),
            (key(Key::Command, false), true),
            (Input::Text('q'), true),
            (button(MouseButton::Left, true), false),
            (button(MouseButton::Right, true), false),
            (Input::PointerMoved(Point::new(1.0, 2.0)), false),
            (Input::PointerLeft, false),
        ]
    }

    fn verdicts(appear: &mut Appear) -> Vec<Verdict> {
        every_input()
            .iter()
            .map(|(input, command)| appear.input(input, *command))
            .collect()
    }

    #[test]
    fn the_verdicts_follow_the_table() {
        use Verdict::{Drop, Held, Pass};
        assert_eq!(
            verdicts(&mut Appear::start_up()),
            [
                Drop, Drop, Drop, Drop, Held, Held, Held, Held, Drop, Drop, Drop, Drop, Drop
            ],
            "start-up: plain keys and the pointer dropped, Command keys held"
        );
        for button_down in [false, true] {
            assert_eq!(
                verdicts(&mut Appear::returning(false, button_down)),
                [
                    Held, Held, Held, Held, Held, Held, Held, Held, Held, Drop, Drop, Drop, Drop
                ],
                "a return: every key held, the pointer dropped"
            );
        }
        assert_eq!(
            verdicts(&mut Appear::returning(true, false)),
            [Pass; 13],
            "the other reading's return takes everything"
        );
    }

    #[test]
    fn the_close_is_held_at_start_up_dropped_on_a_return_and_passed_live() {
        let mut start_up = Appear::start_up();
        assert_eq!(start_up.close(), Verdict::Held);
        assert_eq!(start_up.take_held(), [Deferred::Close]);
        let mut back = Appear::returning(false, true);
        assert_eq!(back.close(), Verdict::Drop);
        assert_eq!(back.take_held(), []);
        let mut live = Appear::returning(true, false);
        assert_eq!(live.close(), Verdict::Pass);
        assert_eq!(live.take_held(), []);
    }

    #[test]
    fn held_inputs_come_back_in_order() {
        let mut appear = Appear::start_up();
        let command = key(Key::Command, true);
        let quit = key(Key::Char('q'), true);
        appear.input(&command, false);
        appear.input(&Input::Text('x'), true);
        appear.close();
        appear.input(&quit, true);
        assert_eq!(
            appear.take_held(),
            [
                Deferred::Input(command),
                Deferred::Close,
                Deferred::Input(quit)
            ]
        );
        assert_eq!(appear.take_held(), [], "once");

        let mut appear = Appear::returning(false, false);
        let typed = [
            key(Key::Char('n'), true),
            Input::Text('n'),
            key(Key::Char('n'), false),
        ];
        for input in &typed {
            appear.input(input, false);
        }
        appear.input(&button(MouseButton::Left, true), false);
        assert_eq!(appear.take_held(), typed.map(Deferred::Input));
    }
}
