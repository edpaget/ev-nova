//! The main menu: New Pilot, Open Pilot and Quit.
//!
//! A plain layout for now: the game's name, and the three buttons in
//! Nova's button style stacked in the middle of the screen. The original's
//! main-menu artwork is still to come.

use std::rc::Rc;
use std::time::Duration;

use crate::color::Color;
use crate::draw::DrawList;
use crate::font::Font;
use crate::geometry::{Bounds, Point};
use crate::input::{Input, Key};
use crate::screen::{Screen, ScreenAction};
use crate::sound::Sound;
use crate::text::TextMetrics;
use crate::ui::{Button, ButtonSkin, ButtonStyle, ButtonTracker};

/// The game's name, over the buttons.
pub const TITLE: &str = "EV Nova";
/// The title's font.
pub const TITLE_FONT: Font = Font::Charcoal;
/// The title's size.
pub const TITLE_SIZE: f32 = 36.0;
/// Where the title's top is.
pub const TITLE_TOP: f32 = 250.0;
/// Each button's size.
const BUTTON_SIZE: (f32, f32) = (200.0, 25.0);
/// Where the first button's top is.
const FIRST_BUTTON_TOP: f32 = 340.0;
/// How far apart the buttons' tops are.
const BUTTON_STEP: f32 = 40.0;
/// The logical screen's width, which the menu is centred across.
const SCREEN_WIDTH: f32 = 1024.0;

/// A button on the main menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuChoice {
    /// Create a pilot.
    NewPilot,
    /// Open a saved pilot.
    OpenPilot,
    /// Quit the game.
    Quit,
}

impl MenuChoice {
    /// Every choice, top to bottom.
    pub const ALL: [Self; 3] = [Self::NewPilot, Self::OpenPilot, Self::Quit];

    fn label(self) -> &'static str {
        match self {
            Self::NewPilot => "New Pilot",
            Self::OpenPilot => "Open Pilot",
            Self::Quit => "Quit",
        }
    }
}

/// The main menu: a button for each [`MenuChoice`].
#[derive(Clone)]
pub struct MainMenu {
    buttons: [Button; 3],
    style: ButtonStyle,
    metrics: Rc<dyn TextMetrics>,
    /// The button a press is held on, by index, and its tracking.
    tracked: Option<usize>,
    tracker: ButtonTracker,
    choice: Option<MenuChoice>,
    sounds: Vec<Sound>,
}

impl std::fmt::Debug for MainMenu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MainMenu")
            .field("tracked", &self.tracked)
            .field("choice", &self.choice)
            .finish_non_exhaustive()
    }
}

impl MainMenu {
    /// The menu, its buttons labelled in `style` and measured by `metrics`.
    #[must_use]
    pub fn new(style: ButtonStyle, metrics: Rc<dyn TextMetrics>) -> Self {
        let (width, height) = BUTTON_SIZE;
        let left = (SCREEN_WIDTH - width) / 2.0;
        let buttons = MenuChoice::ALL.map(|choice| {
            let index = MenuChoice::ALL
                .iter()
                .position(|&c| c == choice)
                .expect("listed");
            let top = (index as f32).mul_add(BUTTON_STEP, FIRST_BUTTON_TOP);
            Button::new(
                Bounds::at(Point::new(left, top), width, height),
                choice.label(),
            )
        });
        Self {
            buttons,
            style,
            metrics,
            tracked: None,
            tracker: ButtonTracker::default(),
            choice: None,
            sounds: Vec::new(),
        }
    }

    /// The button for `choice`.
    #[must_use]
    pub fn button(&self, choice: MenuChoice) -> &Button {
        let index = MenuChoice::ALL
            .iter()
            .position(|&c| c == choice)
            .expect("listed");
        &self.buttons[index]
    }

    /// The button chosen, once.
    pub fn take_choice(&mut self) -> Option<MenuChoice> {
        self.choice.take()
    }
}

impl Screen for MainMenu {
    /// A click on a button (pressed and let go of on it) chooses it, and
    /// an Escape press chooses Quit. The router acts on the choice; the
    /// menu itself never quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        if let Input::Key {
            key: Key::Escape,
            pressed: true,
            repeat: false,
        } = *input
        {
            self.choice = Some(MenuChoice::Quit);
            return ScreenAction::None;
        }
        if let Input::PointerButton {
            pressed: true, at, ..
        } = *input
        {
            self.tracker.cancel_pointer();
            self.tracked = self.buttons.iter().position(|button| button.contains(at));
        }
        let Some(index) = self.tracked else {
            return ScreenAction::None;
        };
        if self.tracker.input(&self.buttons[index], input) {
            self.choice = Some(MenuChoice::ALL[index]);
        }
        if let Some(sound) = self.tracker.take_sound() {
            self.sounds.push(Sound::Ui(sound));
        }
        if !self.tracker.armed() {
            self.tracked = None;
        }
        ScreenAction::None
    }

    /// Nothing moves on its own.
    fn tick(&mut self, _dt: Duration) {}

    /// The title centred across the screen, then each button, pressed
    /// while a press on it is held with the pointer on it.
    fn draw(&self, list: &mut DrawList) {
        let width = self.metrics.width(TITLE_FONT, TITLE_SIZE, TITLE);
        list.text_in(
            TITLE_FONT,
            TITLE,
            Point::new(SCREEN_WIDTH / 2.0 - width / 2.0, TITLE_TOP),
            TITLE_SIZE,
            None,
            Color::WHITE,
        );
        for (index, button) in self.buttons.iter().enumerate() {
            let pressed = self.tracked == Some(index) && self.tracker.pressed();
            button.draw(pressed, &ButtonSkin::NOVA, &self.style, &self.metrics, list);
        }
    }

    fn cancel_pointer(&mut self) {
        self.tracker.cancel_pointer();
        self.tracked = None;
    }

    /// The buttons' sounds as they are clicked.
    fn take_sounds(&mut self) -> Vec<Sound> {
        std::mem::take(&mut self.sounds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::DrawCommand;
    use crate::input::MouseButton;
    use crate::sound::UiSound;
    use crate::text::fixture::MonoMetrics;

    fn menu() -> MainMenu {
        MainMenu::new(ButtonStyle::STOCK, Rc::new(MonoMetrics))
    }

    fn button(pressed: bool, at: Point) -> Input {
        Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        }
    }

    fn click(menu: &mut MainMenu, at: Point) -> Vec<Sound> {
        let mut sounds = Vec::new();
        for pressed in [true, false] {
            assert_eq!(menu.input(&button(pressed, at)), ScreenAction::None);
            sounds.extend(menu.take_sounds());
        }
        sounds
    }

    fn key(key: Key, pressed: bool, repeat: bool) -> Input {
        Input::Key {
            key,
            pressed,
            repeat,
        }
    }

    #[test]
    fn its_three_buttons_are_stacked_in_the_middle_of_the_screen() {
        let menu = menu();
        let rects: Vec<Bounds> = MenuChoice::ALL
            .iter()
            .map(|&choice| menu.button(choice).rect)
            .collect();
        assert_eq!(
            rects,
            [
                Bounds::at(Point::new(412.0, 340.0), 200.0, 25.0),
                Bounds::at(Point::new(412.0, 380.0), 200.0, 25.0),
                Bounds::at(Point::new(412.0, 420.0), 200.0, 25.0),
            ]
        );
        let labels: Vec<&str> = MenuChoice::ALL
            .iter()
            .map(|&choice| menu.button(choice).label.as_str())
            .collect();
        assert_eq!(labels, ["New Pilot", "Open Pilot", "Quit"]);
    }

    #[test]
    fn a_click_on_a_button_chooses_it_once_with_its_sounds() {
        for choice in MenuChoice::ALL {
            let mut menu = menu();
            let at = menu.button(choice).rect.center();
            assert_eq!(
                click(&mut menu, at),
                [Sound::Ui(UiSound::ButtonDown), Sound::Ui(UiSound::ButtonUp)]
            );
            assert_eq!(menu.take_choice(), Some(choice));
            assert_eq!(menu.take_choice(), None, "once");
        }
    }

    #[test]
    fn a_press_let_go_of_elsewhere_or_cancelled_chooses_nothing() {
        let mut menu = menu();
        let new = menu.button(MenuChoice::NewPilot).rect.center();
        menu.input(&button(true, new));
        menu.input(&Input::PointerMoved(Point::new(10.0, 10.0)));
        menu.input(&button(false, Point::new(10.0, 10.0)));
        assert_eq!(menu.take_choice(), None);
        menu.input(&button(true, new));
        menu.cancel_pointer();
        menu.input(&button(false, new));
        assert_eq!(menu.take_choice(), None);
        assert_eq!(
            menu.take_sounds(),
            [
                Sound::Ui(UiSound::ButtonDown),
                Sound::Ui(UiSound::ButtonUp),
                Sound::Ui(UiSound::ButtonDown),
            ],
            "a cancelled press is let go of silently"
        );
        assert_eq!(click(&mut menu, Point::new(10.0, 10.0)), []);
        assert_eq!(menu.take_choice(), None, "a click on nothing");
    }

    #[test]
    fn a_press_on_one_button_let_go_of_on_another_chooses_nothing() {
        let mut menu = menu();
        let new = menu.button(MenuChoice::NewPilot).rect.center();
        let quit = menu.button(MenuChoice::Quit).rect.center();
        menu.input(&button(true, new));
        menu.input(&button(false, quit));
        assert_eq!(menu.take_choice(), None);
    }

    #[test]
    fn escape_chooses_quit_but_its_repeats_and_release_do_not() {
        let mut menu = menu();
        menu.input(&key(Key::Escape, true, true));
        menu.input(&key(Key::Escape, false, false));
        assert_eq!(menu.take_choice(), None);
        menu.input(&key(Key::Escape, true, false));
        assert_eq!(menu.take_choice(), Some(MenuChoice::Quit));
        for other in [key(Key::Enter, true, false), Input::Text('q')] {
            menu.input(&other);
            assert_eq!(menu.take_choice(), None, "{other:?}");
        }
    }

    #[test]
    fn it_draws_the_title_then_each_button_pressed_while_held() {
        let mut menu = menu();
        let mut expected = DrawList::new();
        let title_width = MonoMetrics.width(TITLE_FONT, TITLE_SIZE, TITLE);
        expected.text_in(
            TITLE_FONT,
            TITLE,
            Point::new(512.0 - title_width / 2.0, TITLE_TOP),
            TITLE_SIZE,
            None,
            Color::WHITE,
        );
        for choice in MenuChoice::ALL {
            menu.button(choice).draw(
                false,
                &ButtonSkin::NOVA,
                &ButtonStyle::STOCK,
                &MonoMetrics,
                &mut expected,
            );
        }
        let mut list = DrawList::new();
        menu.draw(&mut list);
        assert_eq!(list, expected);

        let open = menu.button(MenuChoice::OpenPilot).rect.center();
        menu.input(&button(true, open));
        let mut list = DrawList::new();
        menu.draw(&mut list);
        let mut pressed = DrawList::new();
        menu.button(MenuChoice::OpenPilot).draw(
            true,
            &ButtonSkin::NOVA,
            &ButtonStyle::STOCK,
            &MonoMetrics,
            &mut pressed,
        );
        let commands: Vec<&DrawCommand> = list.iter().collect();
        let pressed: Vec<&DrawCommand> = pressed.iter().collect();
        assert!(
            commands.windows(pressed.len()).any(|run| run == pressed),
            "Open Pilot drawn pressed"
        );
    }
}
