//! The main menu: the original's main screen, its six buttons New Pilot,
//! Open Pilot, Quit, Enter Ship, Preferences and About.
//!
//! **Drawing** (`_MainScreenUpdate` @0x17c0d). The port painted black,
//! `PICT` 8000 placed by [`interface_background`], the logo's frame, each
//! slide's frame while it runs (`_MainScreenAnimDraw` @0x16146), each
//! the pilot panel, centred on `mainMenuCenter` ([`PilotPanelArea`],
//! @0x17dcc-0x184ea), each shown row's buttons' `rlëD` frame 0 (frame 1
//! for the one pressed) at its `cölr` 128 position from `menuTopCorner`
//! (@0x184f8-0x186f7), then the rollover over them (@0x18713). Every
//! position is an offset from `menuTopCorner`. The router tells the menu
//! which pilot the panel is about ([`MainMenu::show_pilot`]).
//!
//! **The appear** ([`MainMenu::appear`], `_LoadMainScreenAnim`). The
//! three slides set off ([`Slides`]), and each row of buttons, 0 and 3,
//! 1 and 4, 2 and 5, shows once its slide has finished, with 602 as a
//! slide leaves its first frame and 603 as it finishes. While any slide
//! runs the rollover is held clear ([`Rollover::clear`]), and the step
//! after the last finishes comes at once. A primary press finishes every
//! slide at once, silently, and is tracked as a press as well
//! (`_MainScreenMouse` @0x1ae3a-0x1ae99). A menu made new shows its
//! slides finished; the router has it appear.
//!
//! **The pointer** ([`MainScreenTracker`]). A press is hit-tested per
//! pixel on each button's frame-0 mask, the first hit in button order
//! winning (`_SWRLECollisionWithPoint`, @0x19626-0x19648); the tracker
//! plays 600 and 601 as the original's does.
//!
//! **Moving on its own**, in `_MainScreenAnimIdle`'s order: the logo
//! picks a new frame every 5 Mac ticks ([`Logo`]), the slides step, and
//! the rollover fades toward the button under the pointer ([`Rollover`]).
//! While a press is held the logo stops, as `_TrackMainScreenButtons`'s
//! loop runs the rollover's idle (@0x198b0-0x198b5) but not
//! `_MainScreenAnimIdle`; the time still counts, so a pick due meanwhile
//! comes on the next tick after. While the menu waits unseen behind the
//! router's fade ([`MainMenu::wait`]), nothing moves, but the logo's and
//! the slides' time counts, as their timers were set before the fade.
//! While the app is in the background ([`MainMenu::idle_in_background`]),
//! the logo and the slides wait the same way and the rollover shows its
//! none frame in full.
//!
//! **Without the art** (plug-in data, or test data that has none), the
//! menu draws a stand-in: the game's name, the pilot panel centred on the
//! classic centre (512, 384), and six Nova-skin rectangle buttons stacked
//! in the middle of the screen, hit-tested by rectangle and tracked the
//! same way.

use std::rc::Rc;
use std::time::Duration;

use super::art::{MainScreenArt, PictFrames};
use super::background::{Background, interface_background};
use super::logo::Logo;
use super::pilot_panel::{PanelLabels, PanelLooks, PanelPilot, PilotPanelArea, PilotPanelSource};
use super::rollover::Rollover;
use super::slides::{SlideSound, Slides};
use super::tracking::{MainScreenTracker, Tracked};
use crate::color::Color;
use crate::draw::{DrawCommand, DrawList, fill_rect};
use crate::flight::SharedChance;
use crate::font::Font;
use crate::geometry::{Bounds, Point};
use crate::image::ImageKey;
use crate::input::{Input, MouseButton};
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
    slides: Slides,
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

    fn draw(&self, list: &mut DrawList, pressed: Option<usize>, panel: &PanelDraw<'_>) {
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
        draw_frame(list, &self.art.logo, self.logo.frame(), corner);
        for (slide, art) in self.art.slides.iter().enumerate() {
            if let Some(frame) = self.slides.frame(slide) {
                draw_frame(list, art, frame, corner);
            }
        }
        panel.area.draw(
            list,
            self.background.center,
            self.slides.row_shown(2),
            panel.metrics,
        );
        for (index, button) in self.art.buttons.iter().enumerate() {
            if !self.slides.row_shown(row_of(index)) {
                continue;
            }
            let frame = u16::from(pressed == Some(index));
            list.picture(
                ImageKey::sprite(button.sprite, frame),
                self.button_origin(index),
            );
        }
        self.rollover.draw(list, &self.art.rollover, corner);
    }
}

/// The pilot panel to draw, and what measures its text.
struct PanelDraw<'a> {
    area: &'a PilotPanelArea,
    metrics: &'a dyn TextMetrics,
}

/// The classic centre, which the stand-in's pilot panel is centred on.
const CLASSIC_CENTER: Point = Point::new(512.0, 384.0);

/// Draws frame `frame` of `art`, clipped to its place from `corner`.
fn draw_frame(list: &mut DrawList, art: &PictFrames, frame: u16, corner: Point) {
    let clip = Bounds::at(
        Point::new(corner.x + art.position.x, corner.y + art.position.y),
        art.frame_size.0,
        art.frame_size.1,
    );
    let origin = art.frame_origin(frame);
    list.clipped(
        clip,
        DrawCommand::Picture {
            image: ImageKey::picture(art.picture),
            top_left: Point::new(clip.min.x - origin.x, clip.min.y - origin.y),
        },
    );
}

/// The row of slides button `index` comes in with: 0 and 3 with the
/// first, 1 and 4 with the second, 2 and 5 with the third
/// (`_MainScreenUpdate` @0x184f8-0x186f7).
fn row_of(index: usize) -> usize {
    index % 3
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
    /// The panel about the pilot loaded.
    panel: PilotPanelArea,
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
                slides: Slides::finished(art.slides.map(|slide| slide.frames)),
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
            panel: PilotPanelArea::new(PanelLooks::stock(PanelLabels::default())),
        }
    }

    /// The menu with its pilot panel in `looks`, showing no pilot. A menu
    /// made without them draws the panel in the stock font and colours
    /// with empty labels.
    #[must_use]
    pub fn with_pilot_panel(mut self, looks: PanelLooks) -> Self {
        self.panel = PilotPanelArea::new(looks);
        self
    }

    /// Shows `pilot` in the pilot panel, reading its ship from `source`
    /// when its type changes ([`PilotPanelArea::show`]).
    pub fn show_pilot(&mut self, pilot: PanelPilot, source: &dyn PilotPanelSource) {
        self.panel.show(pilot, source);
    }

    /// The pilot the panel shows.
    #[must_use]
    pub fn pilot_shown(&self) -> &PanelPilot {
        self.panel.pilot()
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

    /// Sets the screen appearing, as `_LoadMainScreenAnim` does each time
    /// the main screen comes up: the slides set off, the rollover cleared
    /// with its frame kept and its first step due, and the logo's 5 ticks
    /// started again on the frame it shows (@0x179d8-0x17a73). The
    /// stand-in has nothing to set off.
    pub fn appear(&mut self) {
        if let Layout::Art(layout) = &mut self.layout {
            layout.slides.start();
            layout.rollover.clear();
            layout.logo.restart();
        }
    }

    /// `dt` passes while the menu waits unseen, behind the router's fade:
    /// nothing moves, but the time counts toward the logo's next pick and
    /// the slides' next step, as `TickCount` runs on.
    pub fn wait(&mut self, dt: Duration) {
        if let Layout::Art(layout) = &mut self.layout {
            layout.logo.wait(dt);
            layout.slides.wait(dt);
        }
    }

    /// `dt` passes with the app in the background, as
    /// `_MainScreenAnimIdle` runs while `_inBackground` is set: the logo
    /// doesn't pick (@0x18845) and the slides don't step (@0x18891), but
    /// their time counts, as their timers aren't touched, so what came due
    /// comes on the first tick after ([`MainMenu::wait`]); the rollover
    /// shows the none frame in full ([`Rollover::switched_out`]). The
    /// stand-in has nothing to idle.
    pub fn idle_in_background(&mut self, dt: Duration) {
        if let Layout::Art(layout) = &mut self.layout {
            layout.rollover.switched_out();
        }
        self.wait(dt);
    }

    /// Whether row `row`'s buttons show: its slide has finished; always
    /// for the stand-in.
    #[must_use]
    pub fn row_shown(&self, row: usize) -> bool {
        match &self.layout {
            Layout::Art(layout) => layout.slides.row_shown(row),
            Layout::StandIn(_) => true,
        }
    }

    /// Whether any slide still runs.
    #[must_use]
    pub fn slides_running(&self) -> bool {
        match &self.layout {
            Layout::Art(layout) => layout.slides.running(),
            Layout::StandIn(_) => false,
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
    /// ([`MainScreenTracker`]); the other mouse buttons do nothing. Escape
    /// does nothing, as `_MainScreenKey` @0x1aba0 has no branch for it
    /// (@0x1ac2d). Typing N, O or Q, in either case,
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
            Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at,
            } => {
                self.pointer = Some(at);
                if pressed && let Layout::Art(layout) = &mut self.layout {
                    layout.slides.skip();
                }
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
            Input::Key { .. } | Input::PointerButton { .. } | Input::Scroll { .. } => {}
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

    /// The logo, the slides and the rollover move on, with the art, in
    /// `_MainScreenAnimIdle`'s order: the logo and the slides only while
    /// no press is held, then the rollover toward the button under the
    /// pointer, held clear while any slide runs and stepping at once in
    /// the tick the last one finishes (`_MainScreenRolloverIdle`
    /// @0x1647f-0x164b1).
    fn tick(&mut self, dt: Duration) {
        let target = self.pointer.and_then(|at| self.hit(at));
        let held = self.tracker.tracking();
        let Layout::Art(layout) = &mut self.layout else {
            return;
        };
        let ran = layout.slides.running();
        if held {
            layout.logo.wait(dt);
            layout.slides.wait(dt);
        } else {
            layout.logo.tick(dt, &mut self.chance);
            let sounds = layout.slides.tick(dt).into_iter().map(|sound| {
                Sound::Ui(match sound {
                    SlideSound::Start(_) => UiSound::RowSlideStart,
                    SlideSound::End(_) => UiSound::RowSlideEnd,
                })
            });
            self.sounds.extend(sounds);
        }
        if layout.slides.running() {
            layout.rollover.clear();
        } else if ran {
            layout.rollover.tick(Duration::ZERO, target);
        } else {
            layout.rollover.tick(dt, target);
        }
    }

    /// The main screen as `_MainScreenUpdate` draws it, or the stand-in:
    /// the title centred across the screen, the pilot panel, then each
    /// button, pressed while a press on it is held with the pointer on it.
    fn draw(&self, list: &mut DrawList) {
        let pressed = self.tracker.pressed();
        let panel = PanelDraw {
            area: &self.panel,
            metrics: self.metrics.as_ref(),
        };
        let buttons = match &self.layout {
            Layout::Art(layout) => {
                layout.draw(list, pressed, &panel);
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
        // The stand-in shows every row, so the message never waits.
        panel.area.draw(list, CLASSIC_CENTER, true, panel.metrics);
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

    /// The buttons' sounds as they are clicked or keyed, and the slides'.
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
    use crate::input::Key;
    use crate::menu::fixture::{
        BACKGROUND_SIZE, BUTTON_POSITIONS, BUTTON_SPRITES, CLEAR_CORNER, LOGO_FRAME, LOGO_PICT,
        LOGO_POSITION, ROLLOVER_POSITION, ROLLOVER_SIZE, ROLLOVER_SPRITE, SLIDE_FRAME, SLIDE_PICTS,
        SLIDE_POSITIONS, main_screen_art,
    };
    use crate::menu::pilot_panel::PanelShip;
    use crate::target_picture::ShipPicture;
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

    // The appear: the slides.

    /// The fixture's menu, appearing: its slides start at -1, -2 and -3
    /// (counts 3, 2 and 3).
    fn appearing() -> MainMenu {
        let mut menu = with_art();
        menu.appear();
        menu
    }

    /// The slides' frames as drawn: each slide's picture and the frame its
    /// offset from the clip shows, with the clip.
    fn drawn_slides(menu: &MainMenu) -> Vec<(i16, u16, Bounds)> {
        let mut list = DrawList::new();
        menu.draw(&mut list);
        list.iter()
            .filter_map(|command| match command {
                DrawCommand::Clipped { clip, command } => match **command {
                    DrawCommand::Picture { image, top_left } if SLIDE_PICTS.contains(&image.id) => {
                        assert_eq!(top_left.x, clip.min.x, "one frame across");
                        let frame = (clip.min.y - top_left.y) / f32::from(SLIDE_FRAME.1);
                        Some((image.id, frame as u16, *clip))
                    }
                    _ => None,
                },
                _ => None,
            })
            .collect()
    }

    /// Where slide `slide`'s frame is drawn.
    fn slide_clip(slide: usize) -> Bounds {
        let (x, y) = SLIDE_POSITIONS[slide];
        Bounds::at(
            Point::new(f32::from(x), f32::from(y)),
            f32::from(SLIDE_FRAME.0),
            f32::from(SLIDE_FRAME.1),
        )
    }

    /// The buttons drawn, by index.
    fn buttons_drawn(menu: &MainMenu) -> Vec<usize> {
        button_frames(menu)
            .iter()
            .map(|&(sprite, ..)| {
                BUTTON_SPRITES
                    .iter()
                    .position(|&id| id == sprite)
                    .expect("a button")
            })
            .collect()
    }

    const START: Sound = Sound::Ui(UiSound::RowSlideStart);
    const END: Sound = Sound::Ui(UiSound::RowSlideEnd);

    #[test]
    fn a_menu_made_shows_its_slides_finished() {
        let menu = with_art();
        assert!(!menu.slides_running());
        assert_eq!(drawn_slides(&menu), []);
        assert_eq!(buttons_drawn(&menu), [0, 1, 2, 3, 4, 5]);
        assert!((0..3).all(|row| menu.row_shown(row)));
    }

    #[test]
    fn appearing_draws_each_slides_frame_in_its_rectangle_from_its_strip() {
        let mut menu = appearing();
        assert!(menu.slides_running());
        assert_eq!(drawn_slides(&menu), [], "every slide below frame 0");
        assert_eq!(buttons_drawn(&menu), [0_usize; 0], "no row shown");
        menu.tick(mac_ticks(1));
        assert_eq!(drawn_slides(&menu), [(8030, 0, slide_clip(0))]);
        menu.tick(mac_ticks(1));
        assert_eq!(
            drawn_slides(&menu),
            [(8030, 1, slide_clip(0)), (8031, 0, slide_clip(1))]
        );
        menu.tick(mac_ticks(1));
        assert_eq!(
            drawn_slides(&menu),
            [
                (8030, 2, slide_clip(0)),
                (8031, 1, slide_clip(1)),
                (8032, 0, slide_clip(2))
            ]
        );
    }

    #[test]
    fn no_frame_is_drawn_past_the_count() {
        let mut menu = appearing();
        menu.tick(mac_ticks(4));
        assert_eq!(drawn_slides(&menu), [(8032, 1, slide_clip(2))]);
        menu.tick(mac_ticks(2));
        assert_eq!(drawn_slides(&menu), []);
    }

    #[test]
    fn the_slides_are_drawn_over_the_logo_and_under_the_buttons() {
        let mut menu = appearing();
        menu.tick(mac_ticks(4));
        let mut list = DrawList::new();
        menu.draw(&mut list);
        let place =
            |wanted: &dyn Fn(&DrawCommand) -> bool| list.iter().position(wanted).expect("drawn");
        let logo = place(&|command| {
            matches!(command, DrawCommand::Clipped { command, .. }
                if matches!(**command, DrawCommand::Picture { image, .. } if image.id == LOGO_PICT))
        });
        let slide = place(&|command| {
            matches!(command, DrawCommand::Clipped { command, .. }
                if matches!(**command, DrawCommand::Picture { image, .. } if image.id == 8032))
        });
        let button = place(
            &|command| matches!(command, DrawCommand::Picture { image, .. } if image.kind == ImageKind::Rled),
        );
        assert!(logo < slide && slide < button, "{logo} {slide} {button}");
    }

    #[test]
    fn the_menus_ticks_raise_602_and_603() {
        let mut menu = appearing();
        let mut heard = Vec::new();
        for _ in 0..7 {
            menu.tick(mac_ticks(1));
            heard.push(menu.take_sounds());
        }
        assert_eq!(
            heard,
            [
                vec![],
                vec![START],
                vec![START],
                vec![END, END, START],
                vec![],
                vec![END],
                vec![],
            ]
        );
    }

    #[test]
    fn each_rows_buttons_show_once_its_slide_finishes() {
        let mut menu = appearing();
        let mut shown = Vec::new();
        for _ in 0..6 {
            menu.tick(mac_ticks(1));
            shown.push(buttons_drawn(&menu));
        }
        assert_eq!(
            shown,
            [
                vec![],
                vec![],
                vec![],
                vec![0, 1, 3, 4],
                vec![0, 1, 3, 4],
                vec![0, 1, 2, 3, 4, 5],
            ]
        );
    }

    #[test]
    fn row_shown_follows_the_slides() {
        let mut menu = appearing();
        assert!((0..3).all(|row| !menu.row_shown(row)));
        menu.tick(mac_ticks(4));
        assert!(menu.row_shown(0) && menu.row_shown(1));
        assert!(!menu.row_shown(2));
        assert!(menu.slides_running());
        menu.tick(mac_ticks(2));
        assert!(menu.row_shown(2));
        assert!(!menu.slides_running());
    }

    // The pilot panel.

    /// A source naming every ship "Variant", with a 4 x 2 `PICT` 3000.
    struct Ships;

    impl PilotPanelSource for Ships {
        fn pilot_panel_looks(&self) -> PanelLooks {
            panel_looks()
        }

        fn panel_ship(&self, _ship: nova_sim::catalog::ShipId) -> PanelShip {
            PanelShip {
                subtitle: "Variant".to_owned(),
                picture: Some(ShipPicture {
                    picture: 3000,
                    width: 4,
                    height: 2,
                }),
            }
        }
    }

    fn panel_looks() -> PanelLooks {
        PanelLooks::stock(PanelLabels {
            pilot_name: "Pilot Name:".to_owned(),
            no_pilot: "No Pilot File Loaded".to_owned(),
            ..PanelLabels::default()
        })
    }

    fn alive() -> PanelPilot {
        PanelPilot::Alive(Box::new(nova_sim::pilot_panel::PilotPanel {
            pilot_name: "Ada".to_owned(),
            ship_name: String::new(),
            ship_class: String::new(),
            ship: nova_sim::catalog::ShipId(128),
            legal_status: String::new(),
            combat_rating: String::new(),
            date: String::new(),
        }))
    }

    /// Where `text` is drawn, if it is: its top-left.
    fn drawn_text(menu: &MainMenu, wanted: &str) -> Option<Point> {
        drawn(menu).iter().find_map(|command| match command {
            DrawCommand::Text { text, origin, .. } if text == wanted => Some(*origin),
            _ => None,
        })
    }

    #[test]
    fn the_panel_is_drawn_after_the_slides_and_before_the_buttons_and_the_rollover() {
        let mut menu = appearing().with_pilot_panel(panel_looks());
        menu.show_pilot(alive(), &Ships);
        menu.tick(mac_ticks(4));
        let list = drawn(&menu);
        let place =
            |wanted: &dyn Fn(&DrawCommand) -> bool| list.iter().position(wanted).expect("drawn");
        let slide = place(&|command| {
            matches!(command, DrawCommand::Clipped { command, .. }
                if matches!(**command, DrawCommand::Picture { image, .. } if image.id == 8032))
        });
        let panel =
            place(&|command| matches!(command, DrawCommand::Text { text, .. } if text == "Ada"));
        let picture = place(&|command| matches!(command, DrawCommand::BlendedPicture { .. }));
        let button = place(
            &|command| matches!(command, DrawCommand::Picture { image, .. } if image.kind == ImageKind::Rled),
        );
        assert!(
            slide < panel && picture < button,
            "{slide} {panel} {picture} {button}"
        );
        let mut menu = with_art().with_pilot_panel(panel_looks());
        menu.show_pilot(alive(), &Ships);
        menu.input(&Input::PointerMoved(menu.press_point(MenuChoice::About)));
        menu.tick(Duration::ZERO);
        menu.tick(mac_ticks(1));
        let list = drawn(&menu);
        let picture = list
            .iter()
            .position(|command| matches!(command, DrawCommand::BlendedPicture { .. }))
            .expect("the picture");
        let rollover = list
            .iter()
            .position(|command| matches!(command, DrawCommand::Sprite { .. }))
            .expect("the rollover");
        assert!(picture < rollover, "{picture} {rollover}");
    }

    #[test]
    fn the_panel_is_centred_on_main_menu_center() {
        let mut menu = with_art().with_pilot_panel(panel_looks());
        menu.show_pilot(alive(), &Ships);
        // The fixture's 4 x 2 picture centres `mainMenuCenter` at (512, 384).
        let center = interface_background(CLASSIC_SIZE, (4, 2)).center;
        assert_eq!(center, Point::new(512.0, 384.0));
        assert_eq!(
            drawn_text(&menu, "Pilot Name:"),
            Some(Point::new(center.x - 190.0, center.y + 250.0 - 9.0))
        );
        assert_eq!(
            drawn_text(&menu, "Variant"),
            Some(Point::new(center.x - 185.0, center.y + 346.0 - 9.0)),
            "the subtitle from the source"
        );
        assert_eq!(menu.pilot_shown(), &alive());
    }

    #[test]
    fn no_pilot_file_loaded_waits_for_slide_2_as_the_screen_appears() {
        let mut menu = appearing().with_pilot_panel(panel_looks());
        menu.appear();
        assert_eq!(menu.pilot_shown(), &PanelPilot::None);
        let mut seen = Vec::new();
        for _ in 0..6 {
            menu.tick(mac_ticks(1));
            seen.push(drawn_text(&menu, "No Pilot File Loaded").is_some());
        }
        assert_eq!(seen, [false, false, false, false, false, true]);
        // "No Pilot File Loaded" is 20 characters, 4.5 each in Geneva 9.
        assert_eq!(
            drawn_text(&menu, "No Pilot File Loaded"),
            Some(Point::new(512.0 - 45.0, 384.0 + 310.0 - 9.0))
        );
    }

    #[test]
    fn the_stand_in_draws_the_panel_at_the_classic_centre_and_its_message_at_once() {
        let mut menu = stand_in().with_pilot_panel(panel_looks());
        menu.appear();
        assert_eq!(
            drawn_text(&menu, "No Pilot File Loaded"),
            Some(Point::new(512.0 - 45.0, 384.0 + 310.0 - 9.0))
        );
        menu.show_pilot(alive(), &Ships);
        assert_eq!(
            drawn_text(&menu, "Pilot Name:"),
            Some(Point::new(512.0 - 190.0, 384.0 + 250.0 - 9.0))
        );
        let list = drawn(&menu);
        let title = list
            .iter()
            .position(|command| matches!(command, DrawCommand::Text { text, .. } if text == TITLE))
            .expect("the title");
        let panel = list
            .iter()
            .position(|command| matches!(command, DrawCommand::Text { text, .. } if text == "Ada"))
            .expect("the panel");
        let button = list
            .iter()
            .position(
                |command| matches!(command, DrawCommand::Text { text, .. } if text == "New Pilot"),
            )
            .expect("a button");
        assert!(title < panel && panel < button, "{title} {panel} {button}");
    }

    #[test]
    fn a_menu_made_without_looks_draws_no_empty_message() {
        let menu = stand_in();
        assert_eq!(menu.pilot_shown(), &PanelPilot::None);
        assert!(
            !drawn(&menu).iter().any(
                |command| matches!(command, DrawCommand::Text { text, .. } if text.is_empty())
            )
        );
    }

    #[test]
    fn the_stand_in_shows_every_row_and_appears_as_it_is() {
        let mut menu = stand_in();
        menu.appear();
        assert!((0..3).all(|row| menu.row_shown(row)));
        assert!(!menu.slides_running());
        let before = drawn(&menu);
        menu.wait(mac_ticks(10));
        menu.tick(mac_ticks(10));
        assert_eq!(drawn(&menu), before);
        assert_eq!(menu.take_sounds(), []);
    }

    #[test]
    fn the_rollover_stays_clear_while_any_slide_runs_and_steps_once_they_finish() {
        let mut menu = appearing();
        menu.input(&Input::PointerMoved(
            menu.press_point(MenuChoice::Preferences),
        ));
        let mut seen = Vec::new();
        for _ in 0..8 {
            menu.tick(mac_ticks(1));
            seen.push(menu.rollover().expect("the art"));
        }
        assert_eq!(
            seen,
            [
                (6, 0),
                (6, 0),
                (6, 0),
                (6, 0),
                (6, 0),
                (4, 0),
                (4, 4),
                (4, 8)
            ],
            "held on the none frame, then one step in the finishing tick"
        );
    }

    #[test]
    fn a_click_during_the_slides_finishes_them_without_603_and_presses_the_button() {
        let mut menu = appearing();
        menu.tick(mac_ticks(2));
        assert_eq!(menu.take_sounds(), [START]);
        let quit = menu.press_point(MenuChoice::Quit);
        menu.input(&button(true, quit));
        assert_eq!(menu.take_sounds(), [DOWN], "no 603");
        assert!(!menu.slides_running());
        assert_eq!(drawn_slides(&menu), []);
        assert_eq!(buttons_drawn(&menu), [0, 1, 2, 3, 4, 5]);
        assert_eq!(menu.tracker.pressed(), Some(MenuChoice::Quit.index()));
        menu.tick(mac_ticks(10));
        assert_eq!(menu.take_sounds(), [], "nothing more from the slides");
        assert_eq!(click_release(&mut menu, quit), Some(MenuChoice::Quit));
    }

    #[test]
    fn a_miss_during_the_slides_finishes_them_silently() {
        let mut menu = appearing();
        menu.tick(mac_ticks(1));
        assert_eq!(click(&mut menu, Point::new(5.0, 5.0)), []);
        assert!(!menu.slides_running());
        assert_eq!(buttons_drawn(&menu), [0, 1, 2, 3, 4, 5]);
        assert_eq!(menu.take_choice(), None);
    }

    #[test]
    fn a_slide_skipped_before_frame_0_never_plays_602() {
        let mut menu = appearing();
        menu.input(&button(true, Point::new(5.0, 5.0)));
        menu.tick(mac_ticks(20));
        assert_eq!(menu.take_sounds(), []);
    }

    #[test]
    fn a_secondary_click_leaves_the_slides_running() {
        let mut menu = appearing();
        for pressed in [true, false] {
            menu.input(&Input::PointerButton {
                button: MouseButton::Right,
                pressed,
                at: Point::new(5.0, 5.0),
            });
        }
        assert!(menu.slides_running());
        menu.tick(mac_ticks(2));
        assert_eq!(menu.take_sounds(), [START]);
    }

    #[test]
    fn waiting_unseen_moves_nothing_and_leaves_the_logo_and_one_step_due() {
        let (mut menu, rolls) = rolling(&[3]);
        menu.appear();
        menu.wait(Duration::from_secs(1));
        assert_eq!(drawn_slides(&menu), []);
        assert_eq!(drawn_logo_frame(&menu), 0);
        assert!(rolls.borrow().sides.is_empty(), "no pick while it waits");
        menu.tick(Duration::ZERO);
        assert_eq!(drawn_logo_frame(&menu), 4, "the pick due comes at once");
        assert_eq!(drawn_slides(&menu), [(8030, 0, slide_clip(0))], "one step");
        menu.tick(Duration::from_millis(10));
        assert_eq!(drawn_slides(&menu), [(8030, 0, slide_clip(0))], "no more");
    }

    #[test]
    fn appearing_again_keeps_the_logo_and_rollover_frames_and_clears_the_rollover() {
        let (mut menu, _rolls) = rolling(&[3]);
        menu.input(&Input::PointerMoved(menu.press_point(MenuChoice::NewPilot)));
        menu.tick(mac_ticks(5));
        assert_eq!(menu.rollover(), Some((0, 20)));
        assert_eq!(menu.logo_frame(), Some(4));
        menu.tick(mac_ticks(4));
        menu.appear();
        assert_eq!(menu.rollover(), Some((0, 0)), "clear, its frame kept");
        assert_eq!(menu.logo_frame(), Some(4), "kept");
        menu.wait(mac_ticks(4));
        menu.tick(Duration::ZERO);
        assert_eq!(menu.logo_frame(), Some(4), "the logo's 5 ticks start again");
    }

    // In the background.

    #[test]
    fn in_the_background_the_rollover_over_a_button_shows_the_none_frame_in_full() {
        let mut menu = with_art();
        menu.input(&Input::PointerMoved(menu.press_point(MenuChoice::Quit)));
        menu.tick(mac_ticks(3));
        assert_eq!(menu.rollover(), Some((2, 12)));
        menu.idle_in_background(mac_ticks(1));
        assert_eq!(menu.rollover(), Some((6, 32)));
        assert_eq!(drawn_rollover(&menu), Some((6, 32)), "drawn so");
    }

    #[test]
    fn in_the_background_the_slides_dont_step_and_play_nothing() {
        let mut menu = appearing();
        menu.idle_in_background(mac_ticks(10));
        assert_eq!(drawn_slides(&menu), [], "still below frame 0");
        assert_eq!(menu.take_sounds(), [], "no 602 or 603");
        assert_eq!(menu.rollover(), Some((6, 32)), "not held clear");
        menu.tick(Duration::ZERO);
        assert_eq!(
            drawn_slides(&menu),
            [(8030, 0, slide_clip(0))],
            "one step due after"
        );
    }

    #[test]
    fn in_the_background_the_logo_doesnt_pick_but_its_time_counts() {
        let (mut menu, rolls) = rolling(&[3]);
        menu.idle_in_background(mac_ticks(5));
        assert_eq!(drawn_logo_frame(&menu), 0);
        assert!(rolls.borrow().sides.is_empty(), "no pick while away");
        menu.tick(Duration::from_millis(1));
        assert_eq!(drawn_logo_frame(&menu), 4, "the pick due comes at once");
    }

    #[test]
    fn in_the_background_the_stand_in_is_unchanged() {
        let mut menu = stand_in();
        let mut before = DrawList::new();
        menu.draw(&mut before);
        menu.idle_in_background(mac_ticks(30));
        let mut after = DrawList::new();
        menu.draw(&mut after);
        assert_eq!(after, before);
        assert_eq!(menu.rollover(), None);
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
    fn escape_chooses_nothing_and_is_silent() {
        let mut menu = with_art();
        for (pressed, repeat) in [(true, false), (true, true), (false, false)] {
            assert_eq!(
                menu.input(&key(Key::Escape, pressed, repeat)),
                ScreenAction::None
            );
            assert_eq!(menu.take_choice(), None, "{pressed} {repeat}");
            assert_eq!(menu.take_sounds(), [], "{pressed} {repeat}");
        }
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
        menu.input(&Input::Text('q'));
        assert_eq!(
            format!("{menu:?}"),
            "MainMenu { pressed: Some(2), choice: Some(Chosen { choice: Quit, by: Shortcut }), .. }"
        );
    }
}
