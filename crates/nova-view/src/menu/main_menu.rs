//! The main menu: the original's main screen, its six buttons New Pilot,
//! Open Pilot, Quit, Enter Ship, Preferences and About.
//!
//! **Drawing** (`_MainScreenUpdate` @0x17c0d, finished: the appear
//! animation is rdm:phase/main-screen/phase-2-appear-animation-and-fades).
//! The port painted black, `PICT` 8000 placed by
//! [`interface_background`], the logo's frame, each button's `rlëD` frame
//! 0 (frame 1 for the one pressed) at its `cölr` 128 position from
//! `menuTopCorner` (@0x184f8-0x186f7), then the rollover over them
//! (@0x18713). Every position is an offset from `menuTopCorner`.
//!
//! **The pointer** ([`MainScreenTracker`]). A press is hit-tested per
//! pixel on each button's frame-0 mask, the first hit in button order
//! winning (`_SWRLECollisionWithPoint`, @0x19626-0x19648); the tracker
//! plays 600 and 601 as the original's does.
//!
//! **Moving on its own.** The logo picks a new frame every 5 Mac ticks
//! ([`Logo`]) and the rollover fades toward the button under the pointer
//! ([`Rollover`]). While a press is held the logo stops, as
//! `_TrackMainScreenButtons`'s loop runs the rollover's idle
//! (@0x198b0-0x198b5) but not `_MainScreenAnimIdle`; the time still
//! counts, so a pick due meanwhile comes on the next tick after.
//!
//! **Without the art** (plug-in data, or test data that has none), the
//! menu draws a stand-in: the game's name and six Nova-skin rectangle
//! buttons stacked in the middle of the screen, hit-tested by rectangle
//! and tracked the same way.

use std::rc::Rc;
use std::time::Duration;

use super::art::MainScreenArt;
use super::background::{Background, interface_background};
use super::logo::Logo;
use super::rollover::Rollover;
use super::tracking::{MainScreenTracker, Tracked};
use crate::color::Color;
use crate::draw::{DrawCommand, DrawList, fill_rect};
use crate::flight::SharedChance;
use crate::font::Font;
use crate::geometry::{Bounds, Point};
use crate::image::ImageKey;
use crate::input::{Input, Key, MouseButton};
use crate::screen::{Screen, ScreenAction};
use crate::sound::{Sound, UiSound};
use crate::text::TextMetrics;
use crate::ui::{Button, ButtonSkin, ButtonStyle};
use crate::view_shape::CLASSIC_SIZE;

/// The stand-in's title, over its buttons.
pub const TITLE: &str = "EV Nova";
/// The title's font.
pub const TITLE_FONT: Font = Font::Charcoal;
/// The title's size.
pub const TITLE_SIZE: f32 = 36.0;
/// Where the title's top is.
pub const TITLE_TOP: f32 = 250.0;
/// Each stand-in button's size.
const BUTTON_SIZE: (f32, f32) = (200.0, 25.0);
/// Where the first stand-in button's top is.
const FIRST_BUTTON_TOP: f32 = 340.0;
/// How far apart the stand-in buttons' tops are.
const BUTTON_STEP: f32 = 40.0;
/// The logical screen's width, which the stand-in is centred across.
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
    /// Enter play with the pilot loaded.
    EnterShip,
    /// The Preferences.
    Preferences,
    /// The About text.
    About,
}

impl MenuChoice {
    /// Every choice, in `_MainScreenChoice`'s order (0 to 5): the buttons'
    /// `spïn`s 600 to 605.
    pub const ALL: [Self; 6] = [
        Self::NewPilot,
        Self::OpenPilot,
        Self::Quit,
        Self::EnterShip,
        Self::Preferences,
        Self::About,
    ];

    /// The choice's place in [`MenuChoice::ALL`].
    #[must_use]
    pub fn index(self) -> usize {
        match self {
            Self::NewPilot => 0,
            Self::OpenPilot => 1,
            Self::Quit => 2,
            Self::EnterShip => 3,
            Self::Preferences => 4,
            Self::About => 5,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::NewPilot => "New Pilot",
            Self::OpenPilot => "Open Pilot",
            Self::Quit => "Quit",
            Self::EnterShip => "Enter Ship",
            Self::Preferences => "Preferences",
            Self::About => "About",
        }
    }
}

/// How a [`MenuChoice`] was made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChosenBy {
    /// A click on its button, which has already made its sounds
    /// (`_TrackMainScreenButtons` @0x195b9).
    Pointer,
    /// Its typed shortcut, which clicked with [`UiSound::ButtonKeyed`]:
    /// the original runs the choice only once that click has played
    /// (`_PlayMainScreenClickSound` @0x177af).
    Shortcut,
    /// The engine's Escape, which chooses Quit silently.
    Escape,
}

/// A choice made on the main menu, and how.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chosen {
    /// The button chosen.
    pub choice: MenuChoice,
    /// How it was chosen.
    pub by: ChosenBy,
}

/// What the menu draws and hit-tests: the original's art, or the
/// stand-in's rectangles.
#[derive(Clone, Debug)]
enum Layout {
    Art(Box<ArtLayout>),
    StandIn(Box<[Button; 6]>),
}

/// The art, where it goes, and what of it moves.
#[derive(Clone, Debug)]
struct ArtLayout {
    art: MainScreenArt,
    background: Background,
    logo: Logo,
    rollover: Rollover,
}

impl ArtLayout {
    /// Where button `index`'s top-left is.
    fn button_origin(&self, index: usize) -> Point {
        let corner = self.background.top_corner;
        let position = self.art.buttons[index].position;
        Point::new(corner.x + position.x, corner.y + position.y)
    }

    /// The first button whose frame-0 mask is opaque at `at`.
    fn hit(&self, at: Point) -> Option<usize> {
        (0..self.art.buttons.len()).find(|&index| {
            let origin = self.button_origin(index);
            let x = (at.x - origin.x).floor() as i64;
            let y = (at.y - origin.y).floor() as i64;
            self.art.buttons[index].mask.opaque_at(x, y)
        })
    }

    fn draw(&self, list: &mut DrawList, pressed: Option<usize>) {
        let corner = self.background.top_corner;
        let (width, height) = CLASSIC_SIZE;
        fill_rect(
            list,
            Bounds::at(Point::new(0.0, 0.0), width as f32, height as f32),
            Color::BLACK,
        );
        list.picture(
            ImageKey::picture(self.art.background),
            self.background.picture_top_left,
        );
        let logo = &self.art.logo;
        let frame = Bounds::at(
            Point::new(corner.x + logo.position.x, corner.y + logo.position.y),
            logo.frame_size.0,
            logo.frame_size.1,
        );
        let origin = logo.frame_origin(self.logo.frame());
        list.clipped(
            frame,
            DrawCommand::Picture {
                image: ImageKey::picture(logo.picture),
                top_left: Point::new(frame.min.x - origin.x, frame.min.y - origin.y),
            },
        );
        for (index, button) in self.art.buttons.iter().enumerate() {
            let frame = u16::from(pressed == Some(index));
            list.picture(
                ImageKey::sprite(button.sprite, frame),
                self.button_origin(index),
            );
        }
        self.rollover.draw(list, &self.art.rollover, corner);
    }
}

/// The main menu: a button for each [`MenuChoice`].
#[derive(Clone)]
pub struct MainMenu {
    layout: Layout,
    style: ButtonStyle,
    metrics: Rc<dyn TextMetrics>,
    tracker: MainScreenTracker,
    /// Where the pointer is, while it is over the window.
    pointer: Option<Point>,
    /// What the logo's picks are drawn on.
    chance: SharedChance,
    choice: Option<Chosen>,
    sounds: Vec<Sound>,
}

impl std::fmt::Debug for MainMenu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MainMenu")
            .field("pressed", &self.tracker.pressed())
            .field("choice", &self.choice)
            .finish_non_exhaustive()
    }
}

impl MainMenu {
    /// The menu drawn from `art`, or the stand-in without it, its
    /// stand-in buttons labelled in `style` and measured by `metrics`. The
    /// logo picks on a source that never fires until one is given
    /// ([`MainMenu::with_chance`]).
    #[must_use]
    pub fn new(
        art: Option<MainScreenArt>,
        style: ButtonStyle,
        metrics: Rc<dyn TextMetrics>,
    ) -> Self {
        let layout = match art {
            Some(art) => Layout::Art(Box::new(ArtLayout {
                background: interface_background(CLASSIC_SIZE, art.background_size),
                logo: Logo::new(art.logo.frames),
                rollover: Rollover::new(art.rollover.frames),
                art,
            })),
            None => Layout::StandIn(Box::new(stand_in_buttons())),
        };
        Self {
            layout,
            style,
            metrics,
            tracker: MainScreenTracker::default(),
            pointer: None,
            chance: SharedChance::default(),
            choice: None,
            sounds: Vec::new(),
        }
    }

    /// The menu with the logo's picks drawn on `chance`.
    #[must_use]
    pub fn with_chance(mut self, chance: SharedChance) -> Self {
        self.set_chance(chance);
        self
    }

    /// Draws the logo's picks on `chance` from now on.
    pub fn set_chance(&mut self, chance: SharedChance) {
        self.chance = chance;
    }

    /// Whether the menu draws the original's art, rather than the
    /// stand-in.
    #[must_use]
    pub fn draws_art(&self) -> bool {
        matches!(self.layout, Layout::Art(_))
    }

    /// `choice`'s button's rectangle: its frame's, with the art.
    #[must_use]
    pub fn button_bounds(&self, choice: MenuChoice) -> Bounds {
        let index = choice.index();
        match &self.layout {
            Layout::Art(layout) => {
                let mask = &layout.art.buttons[index].mask;
                Bounds::at(
                    layout.button_origin(index),
                    mask.width() as f32,
                    mask.height() as f32,
                )
            }
            Layout::StandIn(buttons) => buttons[index].rect,
        }
    }

    /// A point that presses `choice`'s button: the middle of one of its
    /// opaque pixels with the art (the centre's, if it is opaque), and its
    /// rectangle's centre for the stand-in. A button with no opaque pixel
    /// gives its rectangle's centre, which misses.
    #[must_use]
    pub fn press_point(&self, choice: MenuChoice) -> Point {
        let index = choice.index();
        if let Layout::Art(layout) = &self.layout
            && let Some((x, y)) = layout.art.buttons[index].mask.opaque_pixel()
        {
            let origin = layout.button_origin(index);
            return Point::new(origin.x + x as f32 + 0.5, origin.y + y as f32 + 0.5);
        }
        self.button_bounds(choice).center()
    }

    /// The logo's frame shown, with the art.
    #[must_use]
    pub fn logo_frame(&self) -> Option<u16> {
        match &self.layout {
            Layout::Art(layout) => Some(layout.logo.frame()),
            Layout::StandIn(_) => None,
        }
    }

    /// The rollover's frame shown and its alpha, with the art.
    #[must_use]
    pub fn rollover(&self) -> Option<(u16, u8)> {
        match &self.layout {
            Layout::Art(layout) => Some((layout.rollover.shown(), layout.rollover.alpha())),
            Layout::StandIn(_) => None,
        }
    }

    /// Tells the menu where the pointer is now (`None` off the window),
    /// for the rollover's next tick, as `_MainScreenRolloverIdle` reads
    /// `GetMouse` each idle (@0x16500-0x16528) wherever the pointer's
    /// moves went meanwhile. A press being tracked is left alone: only a
    /// pointer input drags it.
    pub fn set_pointer(&mut self, at: Option<Point>) {
        self.pointer = at;
    }

    /// The button chosen, and how, once.
    pub fn take_choice(&mut self) -> Option<Chosen> {
        self.choice.take()
    }

    /// The button whose art or rectangle is under `at`.
    fn hit(&self, at: Point) -> Option<usize> {
        match &self.layout {
            Layout::Art(layout) => layout.hit(at),
            Layout::StandIn(buttons) => buttons.iter().position(|button| button.contains(at)),
        }
    }

    /// Keeps what a tracking step did: its sounds, and its choice.
    fn tracked(&mut self, tracked: Tracked) {
        self.sounds.extend(tracked.sounds().map(Sound::Ui));
        if let Some(index) = tracked.chosen {
            self.choice = Some(Chosen {
                choice: MenuChoice::ALL[index],
                by: ChosenBy::Pointer,
            });
        }
    }
}

/// The stand-in's buttons, stacked in the middle of the screen.
fn stand_in_buttons() -> [Button; 6] {
    let (width, height) = BUTTON_SIZE;
    let left = (SCREEN_WIDTH - width) / 2.0;
    MenuChoice::ALL.map(|choice| {
        let top = (choice.index() as f32).mul_add(BUTTON_STEP, FIRST_BUTTON_TOP);
        Button::new(
            Bounds::at(Point::new(left, top), width, height),
            choice.label(),
        )
    })
}

impl Screen for MainMenu {
    /// A primary-button press on a button, held and let go of over one,
    /// chooses the button let go of over, with the original's sounds
    /// ([`MainScreenTracker`]); the other mouse buttons do nothing. An Escape press chooses Quit, silently: an
    /// engine key, as the original's main screen ignores Escape
    /// (`_MainScreenKey` @0x1ac2d). Typing N, O or Q, in either case,
    /// chooses New Pilot, Open Pilot or Quit with [`UiSound::ButtonKeyed`],
    /// as `_MainScreenKey` @0x1aba0 does through
    /// `_PlayMainScreenClickSound`. The router takes its other keys (E, P,
    /// A and X) and O's held-key guard (@0x1abd3-0x1abf5), since it sees
    /// every key. The router acts on the choice, holding a shortcut's for
    /// its click; the menu itself never quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        match *input {
            Input::Text(typed) => {
                let choice = match typed.to_ascii_lowercase() {
                    'n' => MenuChoice::NewPilot,
                    'o' => MenuChoice::OpenPilot,
                    'q' => MenuChoice::Quit,
                    _ => return ScreenAction::None,
                };
                self.sounds.push(Sound::Ui(UiSound::ButtonKeyed));
                self.choice = Some(Chosen {
                    choice,
                    by: ChosenBy::Shortcut,
                });
            }
            Input::Key {
                key: Key::Escape,
                pressed: true,
                repeat: false,
            } => {
                self.choice = Some(Chosen {
                    choice: MenuChoice::Quit,
                    by: ChosenBy::Escape,
                });
            }
            Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at,
            } => {
                self.pointer = Some(at);
                let hit = self.hit(at);
                let tracked = if pressed {
                    self.tracker.cancel();
                    self.tracker.press(hit)
                } else {
                    self.tracker.release(hit)
                };
                self.tracked(tracked);
            }
            // Other keys do nothing. `_MainScreenMouse` runs only on the
            // primary button's mouseDown, so the other mouse buttons
            // neither press nor let go.
            Input::Key { .. } | Input::PointerButton { .. } => {}
            Input::PointerMoved(at) => {
                self.pointer = Some(at);
                let hit = self.hit(at);
                let tracked = self.tracker.moved(hit);
                self.tracked(tracked);
            }
            Input::PointerLeft => {
                self.pointer = None;
                let tracked = self.tracker.moved(None);
                self.tracked(tracked);
            }
        }
        ScreenAction::None
    }

    /// The logo and the rollover move on, with the art: the rollover
    /// toward the button under the pointer, the logo only while no press
    /// is held.
    fn tick(&mut self, dt: Duration) {
        let target = self.pointer.and_then(|at| self.hit(at));
        let held = self.tracker.tracking();
        if let Layout::Art(layout) = &mut self.layout {
            if held {
                layout.logo.wait(dt);
            } else {
                layout.logo.tick(dt, &mut self.chance);
            }
            layout.rollover.tick(dt, target);
        }
    }

    /// The main screen as `_MainScreenUpdate` draws it, or the stand-in:
    /// the title centred across the screen, then each button, pressed
    /// while a press on it is held with the pointer on it.
    fn draw(&self, list: &mut DrawList) {
        let pressed = self.tracker.pressed();
        let buttons = match &self.layout {
            Layout::Art(layout) => {
                layout.draw(list, pressed);
                return;
            }
            Layout::StandIn(buttons) => buttons,
        };
        let width = self.metrics.width(TITLE_FONT, TITLE_SIZE, TITLE);
        list.text_in(
            TITLE_FONT,
            TITLE,
            Point::new(SCREEN_WIDTH / 2.0 - width / 2.0, TITLE_TOP),
            TITLE_SIZE,
            None,
            Color::WHITE,
        );
        for (index, button) in buttons.iter().enumerate() {
            button.draw(
                pressed == Some(index),
                &ButtonSkin::NOVA,
                &self.style,
                &self.metrics,
                list,
            );
        }
    }

    fn cancel_pointer(&mut self) {
        self.tracker.cancel();
    }

    /// The buttons' sounds as they are clicked or keyed.
    fn take_sounds(&mut self) -> Vec<Sound> {
        std::mem::take(&mut self.sounds)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::RefCell;

    use nova_sim::Chance;
    use nova_sim::mission::screens::mac_ticks;

    use super::*;
    use crate::draw::{Blend, lights_tint};
    use crate::menu::fixture::{
        BACKGROUND_SIZE, BUTTON_POSITIONS, BUTTON_SPRITES, CLEAR_CORNER, LOGO_FRAME, LOGO_PICT,
        LOGO_POSITION, ROLLOVER_POSITION, ROLLOVER_SIZE, ROLLOVER_SPRITE, main_screen_art,
    };
    use crate::text::fixture::MonoMetrics;

    fn stand_in() -> MainMenu {
        MainMenu::new(None, ButtonStyle::STOCK, Rc::new(MonoMetrics))
    }

    fn with_art() -> MainMenu {
        MainMenu::new(
            Some(main_screen_art()),
            ButtonStyle::STOCK,
            Rc::new(MonoMetrics),
        )
    }

    fn button(pressed: bool, at: Point) -> Input {
        Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        }
    }

    fn click(menu: &mut MainMenu, at: Point) -> Vec<Sound> {
        for pressed in [true, false] {
            assert_eq!(menu.input(&button(pressed, at)), ScreenAction::None);
        }
        menu.take_sounds()
    }

    fn key(key: Key, pressed: bool, repeat: bool) -> Input {
        Input::Key {
            key,
            pressed,
            repeat,
        }
    }

    const DOWN: Sound = Sound::Ui(UiSound::ButtonDown);
    const UP: Sound = Sound::Ui(UiSound::ButtonUp);

    /// Where fixture button `index`'s top-left is: its `cölr` position,
    /// `menuTopCorner` being (0, 0) in the classic port.
    fn origin(index: usize) -> Point {
        let (x, y) = BUTTON_POSITIONS[index];
        Point::new(f32::from(x), f32::from(y))
    }

    /// The fixture button `index`'s clear corner.
    fn clear_corner(index: usize) -> Point {
        let at = origin(index);
        Point::new(
            at.x + f32::from(CLEAR_CORNER.0) + 0.5,
            at.y + f32::from(CLEAR_CORNER.1) + 0.5,
        )
    }

    /// The `rlëD` frames drawn as pictures, by sprite.
    fn button_frames(menu: &MainMenu) -> Vec<(i16, u16, Point)> {
        let mut list = DrawList::new();
        menu.draw(&mut list);
        list.iter()
            .filter_map(|command| match command {
                DrawCommand::Picture { image, top_left } if image.kind == ImageKind::Rled => {
                    Some((image.id, image.frame, *top_left))
                }
                _ => None,
            })
            .collect()
    }

    /// The rollover's frame and alpha as drawn, if it is.
    fn drawn_rollover(menu: &MainMenu) -> Option<(u16, u8)> {
        let mut list = DrawList::new();
        menu.draw(&mut list);
        list.iter().find_map(|command| match command {
            DrawCommand::Sprite {
                image, tint, blend, ..
            } if image.id == ROLLOVER_SPRITE => {
                assert_eq!(*blend, Blend::Or);
                let alpha = (0..=32).find(|&alpha| lights_tint(alpha) == *tint)?;
                Some((image.frame, alpha))
            }
            _ => None,
        })
    }

    /// The logo's frame as drawn: its picture's offset from the clip.
    fn drawn_logo_frame(menu: &MainMenu) -> u16 {
        let mut list = DrawList::new();
        menu.draw(&mut list);
        list.iter()
            .find_map(|command| match command {
                DrawCommand::Clipped { clip, command } => match **command {
                    DrawCommand::Picture { image, top_left } if image.id == LOGO_PICT => {
                        Some(((clip.min.y - top_left.y) / f32::from(LOGO_FRAME.1)) as u16)
                    }
                    _ => None,
                },
                _ => None,
            })
            .expect("the logo")
    }

    use crate::image::ImageKind;

    // The art.

    #[test]
    fn it_draws_the_black_port_the_picture_the_logo_and_the_buttons_frame_0() {
        let menu = with_art();
        assert!(menu.draws_art());
        let mut list = DrawList::new();
        menu.draw(&mut list);
        let mut expected = DrawList::new();
        fill_rect(
            &mut expected,
            Bounds::at(Point::new(0.0, 0.0), 1024.0, 768.0),
            Color::BLACK,
        );
        let (width, height) = BACKGROUND_SIZE;
        expected.picture(
            ImageKey::picture(8000),
            Point::new(
                (1024.0 - f32::from(width)) / 2.0,
                (768.0 - f32::from(height)) / 2.0,
            ),
        );
        let logo = Point::new(f32::from(LOGO_POSITION.0), f32::from(LOGO_POSITION.1));
        expected.clipped(
            Bounds::at(logo, f32::from(LOGO_FRAME.0), f32::from(LOGO_FRAME.1)),
            DrawCommand::Picture {
                image: ImageKey::picture(LOGO_PICT),
                top_left: logo,
            },
        );
        for (index, sprite) in BUTTON_SPRITES.into_iter().enumerate() {
            expected.picture(ImageKey::sprite(sprite, 0), origin(index));
        }
        assert_eq!(list, expected, "the rollover clear at first");
    }

    #[test]
    fn a_press_on_an_opaque_pixel_draws_it_pressed_and_plays_600() {
        for choice in MenuChoice::ALL {
            let mut menu = with_art();
            let index = choice.index();
            menu.input(&button(true, menu.press_point(choice)));
            assert_eq!(menu.take_sounds(), [DOWN]);
            let frames: Vec<u16> = button_frames(&menu)
                .iter()
                .map(|&(_, frame, _)| frame)
                .collect();
            let mut expected = [0; 6];
            expected[index] = 1;
            assert_eq!(frames, expected, "{choice:?}");
        }
    }

    #[test]
    fn a_press_on_a_clear_pixel_in_the_rectangle_is_a_silent_miss() {
        for choice in MenuChoice::ALL {
            let mut menu = with_art();
            let corner = clear_corner(choice.index());
            assert!(menu.button_bounds(choice).contains(corner));
            assert_eq!(click(&mut menu, corner), []);
            assert_eq!(menu.take_choice(), None);
            assert!(button_frames(&menu).iter().all(|&(_, frame, _)| frame == 0));
        }
    }

    #[test]
    fn a_click_chooses_the_button_with_its_sounds() {
        for choice in MenuChoice::ALL {
            let mut menu = with_art();
            let at = menu.press_point(choice);
            assert_eq!(click(&mut menu, at), [DOWN, UP]);
            assert_eq!(
                menu.take_choice(),
                Some(Chosen {
                    choice,
                    by: ChosenBy::Pointer
                })
            );
            assert_eq!(menu.take_choice(), None, "once");
        }
    }

    #[test]
    fn dragging_off_on_and_letting_go_over_a_button_chooses_it() {
        let mut menu = with_art();
        let open = menu.press_point(MenuChoice::OpenPilot);
        menu.input(&button(true, open));
        menu.input(&Input::PointerMoved(Point::new(5.0, 5.0)));
        assert!(button_frames(&menu).iter().all(|&(_, frame, _)| frame == 0));
        menu.input(&Input::PointerMoved(open));
        menu.input(&button(false, open));
        assert_eq!(menu.take_sounds(), [DOWN, UP, DOWN, UP]);
        assert_eq!(
            menu.take_choice(),
            Some(Chosen {
                choice: MenuChoice::OpenPilot,
                by: ChosenBy::Pointer
            })
        );
    }

    #[test]
    fn letting_go_off_every_button_is_silent_and_chooses_nothing() {
        let mut menu = with_art();
        let open = menu.press_point(MenuChoice::OpenPilot);
        menu.input(&button(true, open));
        menu.input(&Input::PointerMoved(Point::new(5.0, 5.0)));
        menu.input(&button(false, Point::new(5.0, 5.0)));
        assert_eq!(menu.take_sounds(), [DOWN, UP]);
        assert_eq!(menu.take_choice(), None);
        // The pointer leaving the window drags off too.
        menu.input(&button(true, open));
        menu.input(&Input::PointerLeft);
        assert_eq!(menu.take_sounds(), [DOWN, UP]);
    }

    #[test]
    fn dragging_from_one_button_onto_another_chooses_the_other() {
        let mut menu = with_art();
        let new = menu.press_point(MenuChoice::NewPilot);
        let quit = menu.press_point(MenuChoice::Quit);
        menu.input(&button(true, new));
        menu.input(&Input::PointerMoved(quit));
        let frames: Vec<u16> = button_frames(&menu)
            .iter()
            .map(|&(_, frame, _)| frame)
            .collect();
        assert_eq!(frames, [0, 0, 1, 0, 0, 0]);
        menu.input(&button(false, quit));
        assert_eq!(menu.take_sounds(), [DOWN, DOWN, UP]);
        assert_eq!(
            menu.take_choice().map(|chosen| chosen.choice),
            Some(MenuChoice::Quit)
        );
    }

    #[test]
    fn a_cancelled_press_is_let_go_of_silently() {
        let mut menu = with_art();
        let new = menu.press_point(MenuChoice::NewPilot);
        menu.input(&button(true, new));
        menu.cancel_pointer();
        menu.input(&button(false, new));
        assert_eq!(menu.take_sounds(), [DOWN]);
        assert_eq!(menu.take_choice(), None);
    }

    #[test]
    fn only_the_primary_button_presses_a_button() {
        let other = |button, pressed, at| Input::PointerButton {
            button,
            pressed,
            at,
        };
        let mut menu = with_art();
        let new = menu.press_point(MenuChoice::NewPilot);
        let quit = menu.press_point(MenuChoice::Quit);
        for mouse in [MouseButton::Right, MouseButton::Middle, MouseButton::Other] {
            for pressed in [true, false] {
                menu.input(&other(mouse, pressed, new));
            }
            assert_eq!(menu.take_sounds(), [], "{mouse:?} is silent");
            assert_eq!(menu.take_choice(), None, "{mouse:?} chooses nothing");
            assert!(!menu.tracker.tracking(), "{mouse:?} tracks nothing");
        }
        // A secondary press and release while the primary is held leave
        // its press alone.
        menu.input(&button(true, new));
        menu.input(&other(MouseButton::Right, true, quit));
        menu.input(&other(MouseButton::Right, false, quit));
        assert_eq!(menu.take_sounds(), [DOWN]);
        assert_eq!(menu.take_choice(), None);
        assert_eq!(menu.tracker.pressed(), Some(MenuChoice::NewPilot.index()));
        menu.input(&button(false, new));
        assert_eq!(menu.take_sounds(), [UP]);
        assert_eq!(
            menu.take_choice().map(|chosen| chosen.choice),
            Some(MenuChoice::NewPilot)
        );
    }

    #[test]
    fn a_buttons_rectangle_and_press_point_come_from_its_art() {
        let menu = with_art();
        for choice in MenuChoice::ALL {
            let at = origin(choice.index());
            assert_eq!(menu.button_bounds(choice), Bounds::at(at, 4.0, 3.0));
            // The centre (2, 1) is opaque.
            assert_eq!(menu.press_point(choice), Point::new(at.x + 2.5, at.y + 1.5));
        }
    }

    /// A source whose rolls are scripted, shared so the test can read it.
    #[derive(Default)]
    struct Rolls {
        outcomes: Vec<u16>,
        sides: Vec<u16>,
    }

    impl Chance for Rolls {
        fn fires(&mut self, _percent: u8) -> bool {
            false
        }

        fn below(&mut self, n: u32) -> u32 {
            n - 1
        }

        fn roll(&mut self, sides: u16) -> u16 {
            self.sides.push(sides);
            self.outcomes.pop().unwrap_or(0)
        }
    }

    fn rolling(outcomes: &[u16]) -> (MainMenu, Rc<RefCell<Rolls>>) {
        let rolls = Rc::new(RefCell::new(Rolls {
            outcomes: outcomes.iter().rev().copied().collect(),
            sides: Vec::new(),
        }));
        let chance: Rc<RefCell<dyn Chance>> = Rc::clone(&rolls) as Rc<RefCell<dyn Chance>>;
        (with_art().with_chance(SharedChance::new(chance)), rolls)
    }

    #[test]
    fn the_logo_shows_a_new_frame_every_5_ticks_on_the_chance_given() {
        let (mut menu, rolls) = rolling(&[3, 0]);
        assert_eq!(drawn_logo_frame(&menu), 0);
        menu.tick(mac_ticks(4));
        assert_eq!(drawn_logo_frame(&menu), 0, "not before the fifth tick");
        menu.tick(mac_ticks(1));
        assert_eq!(drawn_logo_frame(&menu), 4);
        assert_eq!(menu.logo_frame(), Some(4));
        menu.tick(mac_ticks(5));
        assert_eq!(drawn_logo_frame(&menu), 0);
        assert_eq!(rolls.borrow().sides, [6, 6], "among the six others");
    }

    #[test]
    fn a_logo_frame_across_the_picture_is_cut_from_its_column() {
        let mut art = main_screen_art();
        art.logo.columns = 2;
        art.logo.frames = 4;
        let menu = MainMenu::new(Some(art), ButtonStyle::STOCK, Rc::new(MonoMetrics));
        let mut menu = menu.with_chance(SharedChance::new(Rc::new(RefCell::new(Rolls {
            outcomes: vec![0],
            sides: Vec::new(),
        }))));
        menu.tick(mac_ticks(5));
        assert_eq!(menu.logo_frame(), Some(1), "the second column");
        let mut list = DrawList::new();
        menu.draw(&mut list);
        let picture = list.iter().find_map(|command| match command {
            DrawCommand::Clipped { command, .. } => match **command {
                DrawCommand::Picture { top_left, .. } => Some(top_left),
                _ => None,
            },
            _ => None,
        });
        let logo = Point::new(f32::from(LOGO_POSITION.0), f32::from(LOGO_POSITION.1));
        assert_eq!(
            picture,
            Some(Point::new(logo.x - f32::from(LOGO_FRAME.0), logo.y))
        );
    }

    #[test]
    fn a_chance_set_later_is_the_one_rolled() {
        let rolls = Rc::new(RefCell::new(Rolls::default()));
        let mut menu = with_art();
        menu.set_chance(SharedChance::new(
            Rc::clone(&rolls) as Rc<RefCell<dyn Chance>>
        ));
        menu.tick(mac_ticks(5));
        assert_eq!(rolls.borrow().sides, [6]);
    }

    #[test]
    fn the_logo_stops_while_a_press_is_held_and_picks_once_let_go() {
        let (mut menu, _rolls) = rolling(&[3]);
        menu.input(&button(true, Point::new(5.0, 5.0)));
        menu.input(&button(true, menu.press_point(MenuChoice::NewPilot)));
        menu.tick(mac_ticks(8));
        assert_eq!(drawn_logo_frame(&menu), 0, "held");
        menu.input(&Input::PointerMoved(Point::new(5.0, 5.0)));
        menu.tick(mac_ticks(1));
        assert_eq!(drawn_logo_frame(&menu), 0, "held off the buttons too");
        menu.input(&button(false, Point::new(5.0, 5.0)));
        menu.tick(Duration::ZERO);
        assert_eq!(drawn_logo_frame(&menu), 4, "the pick due comes at once");
    }

    #[test]
    fn a_miss_holds_no_press_and_the_logo_runs_on() {
        let (mut menu, _rolls) = rolling(&[3]);
        menu.input(&button(true, Point::new(5.0, 5.0)));
        menu.tick(mac_ticks(5));
        assert_eq!(drawn_logo_frame(&menu), 4);
    }

    #[test]
    fn the_rollover_fades_in_over_the_button_under_the_pointer() {
        let mut menu = with_art();
        menu.input(&Input::PointerMoved(
            menu.press_point(MenuChoice::Preferences),
        ));
        menu.tick(Duration::ZERO);
        assert_eq!(drawn_rollover(&menu), None, "switched, still clear");
        assert_eq!(menu.rollover(), Some((4, 0)));
        let mut seen = Vec::new();
        for _ in 0..9 {
            menu.tick(mac_ticks(1));
            seen.push(drawn_rollover(&menu).expect("shown"));
        }
        assert_eq!(
            seen,
            [4, 8, 12, 16, 20, 24, 28, 32, 32].map(|alpha| (4, alpha))
        );
        let mut list = DrawList::new();
        menu.draw(&mut list);
        let center = list.iter().find_map(|command| match command {
            DrawCommand::Sprite { center, .. } => Some(*center),
            _ => None,
        });
        assert_eq!(
            center,
            Some(Point::new(
                f32::from(ROLLOVER_POSITION.0) + f32::from(ROLLOVER_SIZE.0) / 2.0,
                f32::from(ROLLOVER_POSITION.1) + f32::from(ROLLOVER_SIZE.1) / 2.0,
            ))
        );
        assert!(
            matches!(list.iter().last(), Some(DrawCommand::Sprite { .. })),
            "drawn over the buttons"
        );
    }

    #[test]
    fn the_rollover_fades_out_before_switching_and_shows_frame_6_over_none() {
        let mut menu = with_art();
        menu.input(&Input::PointerMoved(menu.press_point(MenuChoice::NewPilot)));
        menu.tick(mac_ticks(20));
        assert_eq!(menu.rollover(), Some((0, 32)));
        menu.input(&Input::PointerMoved(menu.press_point(MenuChoice::About)));
        menu.tick(mac_ticks(1));
        assert_eq!(drawn_rollover(&menu), Some((0, 28)), "fading out");
        menu.tick(mac_ticks(7));
        assert_eq!(menu.rollover(), Some((0, 0)));
        menu.tick(mac_ticks(1));
        assert_eq!(menu.rollover(), Some((5, 0)), "switched once clear");
        menu.input(&Input::PointerLeft);
        menu.tick(mac_ticks(2));
        assert_eq!(menu.rollover(), Some((6, 4)), "the none frame");
        menu.input(&Input::PointerMoved(clear_corner(1)));
        menu.tick(mac_ticks(1));
        assert_eq!(menu.rollover(), Some((6, 8)), "a clear pixel is none");
    }

    #[test]
    fn the_rollover_aims_where_it_is_told_the_pointer_is() {
        let mut menu = with_art();
        menu.input(&Input::PointerMoved(menu.press_point(MenuChoice::NewPilot)));
        menu.set_pointer(Some(menu.press_point(MenuChoice::Quit)));
        menu.tick(mac_ticks(1));
        assert_eq!(menu.rollover(), Some((2, 4)), "over Quit");
        menu.set_pointer(None);
        menu.tick(mac_ticks(3));
        assert_eq!(menu.rollover(), Some((6, 4)), "off the window: none");
    }

    #[test]
    fn being_told_the_pointer_neither_presses_nor_drags() {
        let mut menu = with_art();
        let quit = menu.press_point(MenuChoice::Quit);
        menu.set_pointer(Some(quit));
        assert_eq!(menu.take_sounds(), [], "no press");
        menu.input(&button(true, quit));
        menu.take_sounds();
        menu.set_pointer(Some(Point::new(5.0, 5.0)));
        assert_eq!(menu.take_sounds(), [], "not dragged off");
        assert_eq!(click_release(&mut menu, quit), Some(MenuChoice::Quit));
    }

    /// Lets go of a press at `at`: the choice it makes.
    fn click_release(menu: &mut MainMenu, at: Point) -> Option<MenuChoice> {
        menu.input(&button(false, at));
        menu.take_choice().map(|chosen| chosen.choice)
    }

    #[test]
    fn the_rollover_runs_on_while_a_press_is_held() {
        let mut menu = with_art();
        menu.input(&button(true, menu.press_point(MenuChoice::Quit)));
        menu.tick(Duration::ZERO);
        menu.tick(mac_ticks(1));
        assert_eq!(menu.rollover(), Some((2, 4)));
    }

    // The stand-in.

    #[test]
    fn the_stand_in_stacks_six_labelled_buttons_in_the_middle_of_the_screen() {
        let menu = stand_in();
        assert!(!menu.draws_art());
        let rects: Vec<Bounds> = MenuChoice::ALL
            .iter()
            .map(|&choice| menu.button_bounds(choice))
            .collect();
        let expected: Vec<Bounds> = (0..6)
            .map(|index| Bounds::at(Point::new(412.0, 340.0 + 40.0 * index as f32), 200.0, 25.0))
            .collect();
        assert_eq!(rects, expected);
        for choice in MenuChoice::ALL {
            assert_eq!(
                menu.press_point(choice),
                menu.button_bounds(choice).center()
            );
        }
        assert_eq!(menu.logo_frame(), None);
        assert_eq!(menu.rollover(), None);
    }

    #[test]
    fn the_stand_in_draws_the_title_then_each_button_pressed_while_held() {
        let mut menu = stand_in();
        let labels = [
            "New Pilot",
            "Open Pilot",
            "Quit",
            "Enter Ship",
            "Preferences",
            "About",
        ];
        let expected_list = |pressed: Option<usize>| {
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
            for (index, label) in labels.into_iter().enumerate() {
                Button::new(menu_rect(index), label).draw(
                    pressed == Some(index),
                    &ButtonSkin::NOVA,
                    &ButtonStyle::STOCK,
                    &MonoMetrics,
                    &mut expected,
                );
            }
            expected
        };
        let mut list = DrawList::new();
        menu.draw(&mut list);
        assert_eq!(list, expected_list(None));
        menu.input(&button(true, menu.press_point(MenuChoice::Preferences)));
        let mut list = DrawList::new();
        menu.draw(&mut list);
        assert_eq!(list, expected_list(Some(4)));
        menu.input(&Input::PointerMoved(Point::new(10.0, 10.0)));
        let mut list = DrawList::new();
        menu.draw(&mut list);
        assert_eq!(list, expected_list(None), "dragged off");
    }

    fn menu_rect(index: usize) -> Bounds {
        Bounds::at(Point::new(412.0, 340.0 + 40.0 * index as f32), 200.0, 25.0)
    }

    #[test]
    fn the_stand_in_tracks_as_the_original_does() {
        let mut menu = stand_in();
        let about = menu.press_point(MenuChoice::About);
        assert_eq!(click(&mut menu, about), [DOWN, UP]);
        assert_eq!(
            menu.take_choice(),
            Some(Chosen {
                choice: MenuChoice::About,
                by: ChosenBy::Pointer
            })
        );
        menu.input(&button(true, about));
        menu.input(&Input::PointerMoved(Point::new(10.0, 10.0)));
        menu.input(&button(false, Point::new(10.0, 10.0)));
        assert_eq!(menu.take_sounds(), [DOWN, UP]);
        assert_eq!(menu.take_choice(), None);
        assert_eq!(click(&mut menu, Point::new(10.0, 10.0)), []);
        // Nothing moves on its own.
        let before = drawn(&menu);
        menu.tick(mac_ticks(50));
        assert_eq!(drawn(&menu), before);
    }

    fn drawn(menu: &MainMenu) -> DrawList {
        let mut list = DrawList::new();
        menu.draw(&mut list);
        list
    }

    // The keys.

    #[test]
    fn escape_chooses_quit_but_its_repeats_and_release_do_not() {
        let mut menu = with_art();
        menu.input(&key(Key::Escape, true, true));
        menu.input(&key(Key::Escape, false, false));
        assert_eq!(menu.take_choice(), None);
        menu.input(&key(Key::Escape, true, false));
        assert_eq!(
            menu.take_choice(),
            Some(Chosen {
                choice: MenuChoice::Quit,
                by: ChosenBy::Escape
            })
        );
        assert_eq!(menu.take_sounds(), [], "silently");
    }

    #[test]
    fn n_o_and_q_choose_their_button_with_a_keyed_click_in_either_case() {
        for (typed, choice) in [
            ('n', MenuChoice::NewPilot),
            ('N', MenuChoice::NewPilot),
            ('o', MenuChoice::OpenPilot),
            ('O', MenuChoice::OpenPilot),
            ('q', MenuChoice::Quit),
            ('Q', MenuChoice::Quit),
        ] {
            let mut menu = with_art();
            assert_eq!(menu.input(&Input::Text(typed)), ScreenAction::None);
            assert_eq!(
                menu.take_sounds(),
                [Sound::Ui(UiSound::ButtonKeyed)],
                "{typed}"
            );
            assert_eq!(
                menu.take_choice(),
                Some(Chosen {
                    choice,
                    by: ChosenBy::Shortcut
                }),
                "{typed}"
            );
            assert_eq!(menu.take_choice(), None, "once");
        }
    }

    #[test]
    fn other_keys_choose_nothing_and_make_no_sound() {
        let mut menu = with_art();
        for other in [
            Input::Text('x'),
            Input::Text('a'),
            Input::Text('e'),
            Input::Text(' '),
            Input::Text('\r'),
            key(Key::Enter, true, false),
            key(Key::Char(' '), true, false),
            key(Key::Char('n'), true, false),
        ] {
            menu.input(&other);
            assert_eq!(menu.take_choice(), None, "{other:?}");
            assert_eq!(menu.take_sounds(), [], "{other:?}");
        }
    }

    #[test]
    fn each_choice_has_its_place_in_all() {
        for (index, choice) in MenuChoice::ALL.into_iter().enumerate() {
            assert_eq!(choice.index(), index);
        }
    }

    #[test]
    fn debug_shows_the_press_and_the_choice() {
        let mut menu = with_art();
        menu.input(&button(true, menu.press_point(MenuChoice::Quit)));
        menu.input(&key(Key::Escape, true, false));
        assert_eq!(
            format!("{menu:?}"),
            "MainMenu { pressed: Some(2), choice: Some(Chosen { choice: Quit, by: Escape }), .. }"
        );
    }
}
