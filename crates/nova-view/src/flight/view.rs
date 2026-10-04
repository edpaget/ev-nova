//! The flight screen: the player's ship flying in its starting system,
//! over the parallax starfield and among the system's stellars, with the
//! camera following the ship.
//!
//! The screen owns a [`nova_sim::Session`] and reads everything else once,
//! when it is built: the session's system, through the [`SystemCatalog`]
//! port, the ship's sprite sheet, through the [`ShipSprites`] port, and
//! the HUD's status bar for the player's government, through the
//! [`StatusBars`] port. Drawing and input never read anything.
//!
//! The HUD is drawn last, over everything: the status bar against the
//! right edge, its radar showing the stellars around the ship as drawn,
//! and its bars the session's shield, armour and fuel. Without a status
//! bar it says why, and flight goes on.
//!
//! Each [`Screen::tick`] runs the simulation's fixed-step clock: the
//! frame's time becomes whole steps of 1/30 s, each flown with the keys
//! held, and what is left over is how far the display is between the last
//! two steps. The ship, and the camera with it, are drawn that far from
//! the step before the last towards the last, so motion is smooth at any
//! frame rate while the simulation itself never depends on it.
//!
//! Input, the original's default keys:
//!
//! - Up thrusts, Left and Right turn, and Down turns to face against the
//!   ship's motion, while held: a press (or its key repeats) holds the key
//!   and its release lets it go. Left and Right together cancel, and
//!   either overrides Down.
//! - L lands on the stellar the ship is over, once a press (its repeats
//!   do nothing). The router takes the landing ([`FlightView::take_landing`])
//!   and shows the spaceport. A refused landing says why above the help
//!   line, in the original's words (`STR#` 2002), for
//!   [`MESSAGE_SHOWN_FOR`].
//! - Escape belongs to the app's router, which leaves flight. The screen
//!   never quits.

use std::collections::HashSet;
use std::time::Duration;

use nova_sim::{
    Controls, FixedStep, LandingRefusal, PilotCatalog, Session, ShipState, StellarId, Steps, Turn,
    flight::normalized, flight::shortest_turn,
};

use super::catalog::{ShipSheet, ShipSprites, StatusBars};
use super::hud::{self, HudState, StatusBar};
use super::sprite::rotation_frame;
use crate::draw::crossed_box;
use crate::system::camera::Camera;
use crate::system::catalog::SystemCatalog;
use crate::system::scene::{self, PLACEHOLDER, PLACEHOLDER_SIZE, SystemScene};
use crate::system::starfield;
use crate::{Color, DrawList, ImageKey, Input, Key, Point, Screen, ScreenAction};

/// The overlay: the system's title and the help line.
const TITLE: Point = Point::new(16.0, 32.0);
const TITLE_SIZE: f32 = 20.0;
const HELP_AT: Point = Point::new(16.0, 744.0);
const OVERLAY_SIZE: f32 = 14.0;
/// How far below the ship's placeholder the reason goes.
const MESSAGE_GAP: f32 = 22.0;
/// The help line.
pub const HELP: &str =
    "Up: thrust   Left/Right: turn   Down: reverse   L: land   Esc: leave flight";
/// Where a message, such as why a landing was refused, goes: above the
/// help line.
pub const MESSAGE_AT: Point = Point::new(16.0, 720.0);
/// How long a message stays on screen.
pub const MESSAGE_SHOWN_FOR: Duration = Duration::from_secs(4);

/// The keys flight holds: the original's defaults.
const FLIGHT_KEYS: [Key; 4] = [Key::Up, Key::Left, Key::Right, Key::Down];

/// The land key: the original's default (`STR#` 129, and `STR#` 2002
/// #25).
pub const LAND_KEY: Key = Key::Char('l');

/// `STR#` 2002 #49.
pub const NO_STELLARS: &str = "No stellar objects present.";
/// `STR#` 2002 #67.
pub const TOO_FAR_STATION: &str = "You're too far away to dock at this station.";
/// `STR#` 2002 #68.
pub const TOO_FAR_PLANET: &str = "You're too far away to land on this planet.";
/// `STR#` 2002 #71.
pub const TOO_FAST_STATION: &str = "You're moving too fast to dock at this station.";
/// `STR#` 2002 #72.
pub const TOO_FAST_PLANET: &str = "You're moving too fast to land on this planet.";
/// `STR#` 2002 #82.
pub const DOCKING_DENIED: &str = "Docking request denied.";
/// `STR#` 2002 #83.
pub const LANDING_DENIED: &str = "Landing request denied.";
/// `STR#` 2002 #89: why a ship cannot dock at a station.
pub const HOSTILE_STATION: &str = "The station's hull integrity is too unstable.";
/// `STR#` 2002 #90: why a ship cannot land on a planet.
pub const HOSTILE_PLANET: &str = "The planet's environment is too hostile.";

/// What the player is told when `refusal` stops a landing: the original's
/// words for it, for a station or a planet.
#[must_use]
pub fn refusal_message(refusal: &LandingRefusal) -> &'static str {
    let pick = |station: bool, at_station, on_planet| {
        if station { at_station } else { on_planet }
    };
    match *refusal {
        LandingRefusal::NoStellars => NO_STELLARS,
        LandingRefusal::TooFar { station, .. } => pick(station, TOO_FAR_STATION, TOO_FAR_PLANET),
        LandingRefusal::NotLandable { station, .. } => {
            pick(station, HOSTILE_STATION, HOSTILE_PLANET)
        }
        LandingRefusal::Denied { station, .. } => pick(station, DOCKING_DENIED, LANDING_DENIED),
        LandingRefusal::TooFast { station, .. } => pick(station, TOO_FAST_STATION, TOO_FAST_PLANET),
    }
}

/// The player's ship in flight.
#[derive(Clone, Debug)]
pub struct FlightView {
    /// The flight, or why it could not start.
    session: Result<Session, String>,
    /// The session's system, laid out; `None` when the session failed.
    scene: Option<SystemScene>,
    /// The player ship's sheet, or why it cannot be shown.
    sheet: Result<ShipSheet, String>,
    /// The HUD's status bar, or why it cannot be shown.
    status_bar: Result<StatusBar, String>,
    /// Turns frame times into simulation steps.
    clock: FixedStep,
    /// The player's ship as it was a step before the session's.
    previous: ShipState,
    /// How far the display is from `previous` to the session's ship.
    alpha: f32,
    /// Time since the view opened, which drives the stellars' animations.
    elapsed: Duration,
    /// The flight keys held down.
    held: HashSet<Key>,
    /// The stellar landed on, until the router takes it.
    pending_landing: Option<StellarId>,
    /// The message shown, and `elapsed` when it was shown.
    message: Option<(&'static str, Duration)>,
}

impl FlightView {
    /// A new pilot's flight, read from `catalog` once.
    pub fn new(catalog: &(impl PilotCatalog + SystemCatalog + ShipSprites + StatusBars)) -> Self {
        let session = Session::start(catalog).map_err(|err| err.to_string());
        let (scene, sheet, status_bar) = match &session {
            Ok(session) => (
                Some(SystemScene::load(catalog, session.system())),
                catalog.ship_sheet(session.ship()),
                hud::choose_status_bar(catalog, session.government()),
            ),
            Err(reason) => (None, Err(reason.clone()), Err(reason.clone())),
        };
        let previous = session
            .as_ref()
            .map(|session| *session.player())
            .unwrap_or_default();
        Self {
            session,
            scene,
            sheet,
            status_bar,
            clock: FixedStep::new(),
            previous,
            alpha: 0.0,
            elapsed: Duration::ZERO,
            held: HashSet::new(),
            pending_landing: None,
            message: None,
        }
    }

    /// The stellar the ship has just landed on, once: the router takes it
    /// to show the spaceport.
    pub fn take_landing(&mut self) -> Option<StellarId> {
        self.pending_landing.take()
    }

    /// Takes off from the stellar landed on, and gives it; `None` when the
    /// ship has not landed. The next frame draws the ship where it is, at
    /// the stellar, not on its way from where it was, and shows no message
    /// from before the landing.
    pub fn take_off(&mut self) -> Option<StellarId> {
        let stellar = self.session.as_mut().ok()?.take_off()?;
        self.previous = self.current();
        self.alpha = 0.0;
        self.message = None;
        Some(stellar)
    }

    /// The message on screen, if any.
    #[must_use]
    pub fn message(&self) -> Option<&'static str> {
        let (text, shown_at) = self.message?;
        (self.elapsed < shown_at + MESSAGE_SHOWN_FOR).then_some(text)
    }

    /// Lands, or shows why not.
    fn land(&mut self) {
        let Ok(session) = &mut self.session else {
            return;
        };
        match session.land() {
            Ok(stellar) => {
                self.pending_landing = Some(stellar);
                self.message = None;
            }
            Err(refusal) => self.message = Some((refusal_message(&refusal), self.elapsed)),
        }
    }

    /// The flight, or why it could not start.
    pub fn session(&self) -> Result<&Session, &str> {
        self.session.as_ref().map_err(String::as_str)
    }

    /// The session's system, as laid out, if the session started.
    #[must_use]
    pub fn scene(&self) -> Option<&SystemScene> {
        self.scene.as_ref()
    }

    /// The HUD's status bar, or why it cannot be shown.
    pub fn status_bar(&self) -> Result<&StatusBar, &str> {
        self.status_bar.as_ref().map_err(String::as_str)
    }

    /// How far the display is between the last two steps, in `[0, 1)`.
    #[must_use]
    pub fn alpha(&self) -> f32 {
        self.alpha
    }

    /// Where the ship is drawn: `alpha` of the way from where it was a step
    /// ago to where it is.
    #[must_use]
    pub fn shown_position(&self) -> Point {
        let (from, to) = (self.previous.position, self.current().position);
        Point::new(
            (to.x - from.x).mul_add(self.alpha, from.x),
            (to.y - from.y).mul_add(self.alpha, from.y),
        )
    }

    /// Which way the ship is drawn facing: `alpha` of the way, the short way
    /// round, from its heading a step ago to its heading now.
    #[must_use]
    pub fn shown_heading(&self) -> f32 {
        let (from, to) = (self.previous.heading, self.current().heading);
        normalized(shortest_turn(from, to).mul_add(self.alpha, from))
    }

    /// The camera, on the ship as drawn.
    #[must_use]
    pub fn camera(&self) -> Camera {
        Camera::centred_on(self.shown_position())
    }

    /// The frame of the ship's sheet drawn, or `None` without a sheet.
    #[must_use]
    pub fn frame(&self) -> Option<u16> {
        let sheet = self.sheet.as_ref().ok()?;
        Some(rotation_frame(self.shown_heading(), sheet.rotations))
    }

    /// The player's ship now, or where it was for a session that never
    /// started.
    fn current(&self) -> ShipState {
        self.session
            .as_ref()
            .map_or(self.previous, |session| *session.player())
    }

    /// The controls for the keys held.
    fn controls(&self) -> Controls {
        let holding = |key| self.held.contains(&key);
        let turn = match (holding(Key::Left), holding(Key::Right)) {
            (true, false) => Turn::Left,
            (false, true) => Turn::Right,
            _ => Turn::None,
        };
        Controls {
            thrust: holding(Key::Up),
            turn,
            reverse: holding(Key::Down),
        }
    }

    fn draw_ship(&self, list: &mut DrawList, at: Point) {
        match &self.sheet {
            Ok(sheet) => {
                let frame = rotation_frame(self.shown_heading(), sheet.rotations);
                list.sprite(ImageKey::sprite(sheet.image_id, frame), at, Color::WHITE);
            }
            Err(reason) => {
                crossed_box(list, at, PLACEHOLDER_SIZE, PLACEHOLDER);
                let below = Point::new(
                    at.x - PLACEHOLDER_SIZE / 2.0,
                    at.y + PLACEHOLDER_SIZE / 2.0 + MESSAGE_GAP,
                );
                list.text(
                    format!("Sprite unavailable: {reason}"),
                    below,
                    OVERLAY_SIZE,
                    None,
                    Color::ERROR,
                );
            }
        }
    }
}

impl Screen for FlightView {
    /// Never quits: Escape is the router's.
    fn input(&mut self, input: &Input) -> ScreenAction {
        if let Input::Key {
            key: LAND_KEY,
            pressed: true,
            repeat: false,
        } = *input
        {
            self.land();
        }
        if let Input::Key { key, pressed, .. } = *input
            && FLIGHT_KEYS.contains(&key)
        {
            if pressed {
                self.held.insert(key);
            } else {
                self.held.remove(&key);
            }
        }
        ScreenAction::None
    }

    /// Runs the simulation's steps for `dt` with the keys held, and advances
    /// the stellars' animations.
    fn tick(&mut self, dt: Duration) {
        self.elapsed += dt;
        let Steps { steps, alpha } = self.clock.advance(dt);
        let controls = self.controls();
        if let Ok(session) = &mut self.session {
            for _ in 0..steps {
                self.previous = *session.player();
                session.tick(controls);
            }
        }
        self.alpha = alpha;
    }

    fn draw(&self, list: &mut DrawList) {
        let Some(scene) = &self.scene else {
            let reason = self.session().err().unwrap_or_default();
            list.text(
                format!("Cannot start flight: {reason}"),
                TITLE,
                TITLE_SIZE,
                None,
                Color::ERROR,
            );
            list.text(HELP, HELP_AT, OVERLAY_SIZE, None, Color::DIM);
            return;
        };
        let camera = self.camera();
        starfield::draw(list, &camera);
        scene::draw_stellars(list, scene, &camera, self.elapsed);
        self.draw_ship(list, camera.world_to_screen(self.shown_position()));
        list.text(
            format!("{} (sÿst {})", scene.name(), scene.id().0),
            TITLE,
            TITLE_SIZE,
            None,
            Color::WHITE,
        );
        list.text(HELP, HELP_AT, OVERLAY_SIZE, None, Color::DIM);
        if let Some(message) = self.message() {
            list.text(message, MESSAGE_AT, OVERLAY_SIZE, None, Color::WHITE);
        }
        match &self.status_bar {
            Ok(bar) => {
                let stellars: Vec<Point> = scene.stellars().iter().map(|s| s.position).collect();
                let state = HudState {
                    position: self.shown_position(),
                    stellars: &stellars,
                    reserves: self.current().reserves,
                    system: scene.name(),
                };
                hud::draw(list, bar, &state);
            }
            Err(reason) => hud::draw_unavailable(list, reason),
        }
    }

    /// Lets go of every flight key: the ship stops thrusting and turning,
    /// and coasts.
    fn release_keys(&mut self) {
        self.held.clear();
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::num::NonZeroU16;

    use nova_sim::landing::{LandingRefusal, StellarFlags};
    use nova_sim::{
        CharacterStart, Handling, LandingSite, Reserves, ShipFields, ShipId, StartError, SystemId,
        TICK, Vec2, step,
    };

    use super::*;
    use crate::flight::catalog::{GovtId, StatusBarLayout};
    use crate::flight::hud::{self, HudState, StatusBar};
    use crate::system::camera::VIEW_CENTER;
    use crate::system::catalog::{
        AnimationData, StellarContents, StellarId, StellarSheet, SystemContents,
    };
    use crate::{DrawCommand, Font};

    /// The first `chär` flies ship 128 (an average ship that turns 3° a
    /// tick, with a 36-rotation, 40 x 40 sheet, `rlëD` 2000) from system
    /// 130, Sol: Earth at (0, -600) and Moon at (300, -200), which animates
    /// a frame a tick.
    struct FakeCatalog {
        character: Result<CharacterStart, StartError>,
        sheet: Result<ShipSheet, String>,
        /// `ïntf` 128: stock-like, or why it cannot be read.
        bar: Result<StatusBarLayout, String>,
        /// System 130's landing sites: Earth and Moon, by default.
        sites: Vec<LandingSite>,
    }

    const FIELDS: ShipFields = ShipFields {
        speed: 300,
        accel: 300,
        maneuver: 30,
        shield: 40,
        armor: 60,
        fuel: 250,
    };

    fn sheet() -> ShipSheet {
        ShipSheet {
            image_id: 2000,
            rotations: NonZeroU16::new(36).expect("non-zero"),
            frame_width: 40,
            frame_height: 40,
        }
    }

    fn catalog() -> FakeCatalog {
        FakeCatalog {
            character: Ok(CharacterStart {
                ship: Some(ShipId(128)),
                systems: [None, Some(SystemId(130)), None, None],
            }),
            sheet: Ok(sheet()),
            bar: Ok(layout()),
            sites: vec![
                site(128, (0.0, -600.0), StellarFlags::CAN_LAND),
                site(129, (300.0, -200.0), StellarFlags::CAN_LAND),
            ],
        }
    }

    /// Stellar `id` at `(x, y)`, 20 x 20 (radius 10), with `flags`.
    fn site(id: i16, (x, y): (f32, f32), flags: u32) -> LandingSite {
        LandingSite {
            id: StellarId(id),
            position: Vec2::new(x, y),
            frame_size: Some((20, 20)),
            flags,
            min_status: 0,
        }
    }

    /// Stock `ïntf` 128's areas, with background `PICT` 700.
    fn layout() -> StatusBarLayout {
        let rect = |left, top, right, bottom| crate::geometry::Bounds {
            min: at(left, top),
            max: at(right, bottom),
        };
        StatusBarLayout {
            radar: rect(8.0, 8.0, 184.0, 184.0),
            shield: rect(35.0, 199.0, 184.0, 206.0),
            armor: rect(35.0, 216.0, 184.0, 223.0),
            fuel: rect(35.0, 234.0, 184.0, 241.0),
            nav: rect(8.0, 254.0, 184.0, 286.0),
            bright_text: Color::WHITE,
            dim_text: Color::DIM,
            bright_radar: Color::rgba(0, 255, 0, 255),
            dim_radar: Color::rgba(0, 128, 0, 255),
            shield_color: Color::rgba(0, 0, 255, 255),
            armor_color: Color::rgba(255, 0, 0, 255),
            fuel_full: Color::rgba(255, 255, 0, 255),
            fuel_partial: Color::rgba(128, 128, 0, 255),
            font: Font::Geneva,
            font_size: 12.0,
            status_bkgnd: 700,
        }
    }

    impl PilotCatalog for FakeCatalog {
        fn first_character(&self) -> Result<CharacterStart, StartError> {
            self.character.clone()
        }

        fn ship_fields(&self, id: ShipId) -> Result<ShipFields, String> {
            assert_eq!(id, ShipId(128));
            Ok(FIELDS)
        }

        fn system_exists(&self, id: SystemId) -> bool {
            id == SystemId(130)
        }

        fn landing_sites(&self, system: SystemId) -> Vec<LandingSite> {
            assert_eq!(system, SystemId(130));
            self.sites.clone()
        }
    }

    fn stellar(id: i16, name: &str, (x, y): (i16, i16), frames: u16) -> StellarContents {
        StellarContents {
            id: StellarId(id),
            name: name.to_owned(),
            x,
            y,
            sprite: Ok(StellarSheet {
                image_id: 1000 + id,
                frames: NonZeroU16::new(frames).expect("non-zero"),
                frame_width: 20,
                frame_height: 20,
            }),
            animation: AnimationData {
                delay: 1,
                frame0_bias: 0,
                only_when_destroyed: false,
            },
        }
    }

    impl SystemCatalog for FakeCatalog {
        fn system(&self, id: SystemId) -> SystemContents {
            assert_eq!(id, SystemId(130));
            SystemContents {
                id,
                name: "Sol".to_owned(),
                stellars: vec![
                    stellar(128, "Earth", (0, -600), 1),
                    stellar(129, "Moon", (300, -200), 4),
                ],
                problems: Vec::new(),
            }
        }
    }

    impl ShipSprites for FakeCatalog {
        fn ship_sheet(&self, id: ShipId) -> Result<ShipSheet, String> {
            assert_eq!(id, ShipId(128));
            self.sheet.clone()
        }
    }

    /// A new pilot has no government, so only `ïntf` 128 and its picture
    /// are ever asked for.
    impl StatusBars for FakeCatalog {
        fn government_interface(&self, id: GovtId) -> Result<i16, String> {
            panic!("asked for gövt {}", id.0)
        }

        fn status_bar(&self, id: i16) -> Result<StatusBarLayout, String> {
            assert_eq!(id, 128);
            self.bar.clone()
        }

        fn picture_size(&self, id: i16) -> Option<(u32, u32)> {
            assert_eq!(id, 700);
            Some((194, 767))
        }
    }

    fn flight() -> FlightView {
        FlightView::new(&catalog())
    }

    fn at(x: f32, y: f32) -> Point {
        Point::new(x, y)
    }

    fn key(key: Key, pressed: bool) -> Input {
        Input::Key {
            key,
            pressed,
            repeat: false,
        }
    }

    fn held(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: true,
        }
    }

    fn player(view: &FlightView) -> ShipState {
        *view.session().expect("flying").player()
    }

    fn handling() -> Handling {
        Handling::from_fields(FIELDS)
    }

    /// The ship as it starts: at rest at the centre, facing up, full.
    fn start() -> ShipState {
        ShipState {
            reserves: Reserves::from_fields(FIELDS),
            ..ShipState::default()
        }
    }

    /// `state` after `ticks` steps under `controls`.
    fn stepped(mut state: ShipState, controls: Controls, ticks: u32) -> ShipState {
        for _ in 0..ticks {
            step(&mut state, &handling(), controls);
        }
        state
    }

    /// The second half of a tick: `TICK / 2` rounds down, so this is a
    /// nanosecond longer.
    const REST_OF_TICK: Duration = Duration::from_nanos(16_666_667);

    const THRUST: Controls = Controls {
        thrust: true,
        turn: Turn::None,
        reverse: false,
    };

    fn ticks(view: &mut FlightView, n: u32) {
        for _ in 0..n {
            view.tick(TICK);
        }
    }

    // Building.

    #[test]
    fn it_starts_the_session_in_its_system_at_rest() {
        let view = flight();
        let session = view.session().expect("flying");
        assert_eq!(session.ship(), ShipId(128));
        assert_eq!(session.system(), SystemId(130));
        assert_eq!(session.handling(), handling());
        assert_eq!(*session.player(), start());
        let scene = view.scene().expect("a scene");
        assert_eq!(scene.id(), SystemId(130));
        assert_eq!(scene.stellars().len(), 2);
        assert_eq!(view.camera().center(), at(0.0, 0.0));
        assert_eq!(view.shown_position(), at(0.0, 0.0));
        assert_eq!(view.frame(), Some(0));
    }

    #[test]
    fn a_session_that_cannot_start_says_why_and_draws_nothing_else() {
        let failed = FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..catalog()
        };
        let mut view = FlightView::new(&failed);
        assert_eq!(view.session().err(), Some("no chär to start from"));
        assert!(view.scene().is_none());
        assert_eq!(view.frame(), None);
        view.input(&key(Key::Up, true));
        ticks(&mut view, 3);
        assert_eq!(
            drawn(&view).iter().cloned().collect::<Vec<_>>(),
            [
                DrawCommand::Text {
                    text: "Cannot start flight: no chär to start from".to_owned(),
                    font: Font::Geneva,
                    origin: TITLE,
                    size: TITLE_SIZE,
                    wrap_width: None,
                    color: Color::ERROR,
                },
                overlay(HELP, HELP_AT, OVERLAY_SIZE, Color::DIM),
            ]
        );
    }

    // Flying.

    #[test]
    fn holding_up_thrusts_the_ship_up_and_the_camera_follows() {
        let mut view = flight();
        assert_eq!(view.input(&key(Key::Up, true)), ScreenAction::None);
        view.tick(Duration::from_secs(1));
        let expected = stepped(start(), THRUST, 30);
        assert_eq!(player(&view), expected);
        assert!(expected.position.y < -40.0, "{expected:?}");
        assert_eq!(expected.position.x, 0.0);
        assert_eq!(view.camera().center(), view.shown_position());
        let shown = view.shown_position();
        // Drawn at most a step behind.
        let behind = shown.y - expected.position.y;
        assert!((0.0..=3.0 + 1e-4).contains(&behind), "{shown:?}");
    }

    #[test]
    fn releasing_up_coasts() {
        let mut view = flight();
        view.input(&key(Key::Up, true));
        ticks(&mut view, 10);
        view.input(&key(Key::Up, false));
        let coasting = player(&view);
        ticks(&mut view, 10);
        let later = player(&view);
        assert_eq!(later.velocity, coasting.velocity);
        assert_eq!(later, stepped(coasting, Controls::default(), 10));
    }

    #[test]
    fn left_and_right_turn_the_ship_and_its_frame() {
        let mut view = flight();
        view.input(&key(Key::Right, true));
        ticks(&mut view, 11);
        // 30° after 10 steps, drawn a step behind.
        assert_eq!(player(&view).heading, 33.0);
        assert_eq!(view.shown_heading(), 30.0);
        assert_eq!(view.frame(), Some(3));
        view.input(&key(Key::Right, false));
        view.input(&key(Key::Left, true));
        ticks(&mut view, 21);
        assert_eq!(player(&view).heading, 330.0);
        assert_eq!(view.frame(), Some(33));
        assert_eq!(player(&view).velocity, Vec2::ZERO, "turning alone");
    }

    #[test]
    fn left_and_right_together_cancel() {
        let mut view = flight();
        view.input(&key(Key::Left, true));
        view.input(&key(Key::Right, true));
        ticks(&mut view, 5);
        assert_eq!(player(&view).heading, 0.0);
    }

    #[test]
    fn down_turns_the_ship_against_its_motion() {
        let mut view = flight();
        view.input(&key(Key::Up, true));
        ticks(&mut view, 30);
        view.input(&key(Key::Up, false));
        view.input(&key(Key::Down, true));
        ticks(&mut view, 70);
        assert!(
            (player(&view).heading - 180.0).abs() < 1e-3,
            "{:?}",
            player(&view)
        );
        assert_eq!(view.frame(), Some(18));
        // Left or Right overrides it.
        view.input(&key(Key::Left, true));
        ticks(&mut view, 1);
        assert!((player(&view).heading - 177.0).abs() < 1e-3);
    }

    #[test]
    fn repeats_hold_a_key_and_releasing_the_keys_stops_the_thrust() {
        let mut view = flight();
        view.input(&held(Key::Up));
        ticks(&mut view, 5);
        assert_eq!(player(&view), stepped(start(), THRUST, 5));
        view.release_keys();
        let coasting = player(&view);
        ticks(&mut view, 5);
        assert_eq!(player(&view), stepped(coasting, Controls::default(), 5));
    }

    #[test]
    fn a_release_without_a_press_does_nothing() {
        let mut view = flight();
        view.input(&key(Key::Up, false));
        ticks(&mut view, 3);
        assert_eq!(player(&view), start());
    }

    #[test]
    fn other_input_changes_nothing_and_never_quits() {
        let mut view = flight();
        let others = [
            key(Key::Escape, true),
            key(Key::Tab, true),
            key(Key::Space, true),
            key(Key::Enter, true),
            key(Key::Char('w'), true),
            key(Key::Other, true),
            Input::PointerMoved(at(1.0, 2.0)),
            Input::PointerButton {
                button: crate::MouseButton::Left,
                pressed: true,
                at: at(1.0, 2.0),
            },
        ];
        for input in others {
            assert_eq!(view.input(&input), ScreenAction::None, "{input:?}");
        }
        ticks(&mut view, 5);
        assert_eq!(player(&view), start());
    }

    // Interpolation.

    #[test]
    fn the_ship_is_drawn_alpha_of_the_way_from_the_last_step_to_this_one() {
        let mut view = flight();
        view.input(&key(Key::Up, true));
        view.tick(TICK);
        let first = stepped(start(), THRUST, 1);
        assert_eq!(player(&view), first);
        assert_eq!(view.alpha(), 0.0);
        assert_eq!(view.shown_position(), at(0.0, 0.0), "a step behind");
        view.tick(TICK / 2);
        assert!((view.alpha() - 0.5).abs() < 1e-6, "{}", view.alpha());
        assert_eq!(player(&view), first, "no step yet");
        let half = view.shown_position();
        assert!((half.y - first.position.y / 2.0).abs() < 1e-6, "{half:?}");
        assert_eq!(half.x, 0.0);
        assert_eq!(view.camera().center(), half);
        // The rest of the tick: one more step, and back to its start.
        view.tick(REST_OF_TICK);
        let second = stepped(first, THRUST, 1);
        assert_eq!(player(&view), second);
        assert_eq!(view.alpha(), 0.0);
        assert_eq!(
            view.shown_position(),
            at(first.position.x, first.position.y)
        );
    }

    #[test]
    fn mid_flight_the_ship_is_drawn_between_its_last_two_positions() {
        let mut view = flight();
        view.input(&key(Key::Right, true));
        ticks(&mut view, 15);
        view.input(&key(Key::Right, false));
        view.input(&key(Key::Up, true));
        ticks(&mut view, 20);
        let before = player(&view);
        ticks(&mut view, 1);
        let after = player(&view);
        view.tick(TICK / 4);
        let shown = view.shown_position();
        let expected = |from: f32, to: f32| from + (to - from) * view.alpha();
        assert!(
            before.position.x > 1.0 && before.position.y < -1.0,
            "{before:?}"
        );
        assert!((shown.x - expected(before.position.x, after.position.x)).abs() < 1e-4);
        assert!((shown.y - expected(before.position.y, after.position.y)).abs() < 1e-4);
    }

    #[test]
    fn the_heading_is_drawn_the_short_way_round_across_0() {
        let mut view = flight();
        view.input(&key(Key::Left, true));
        view.tick(TICK);
        assert_eq!(player(&view).heading, 357.0);
        view.tick(TICK / 2);
        assert!(
            (view.shown_heading() - 358.5).abs() < 1e-3,
            "{}",
            view.shown_heading()
        );
        view.input(&key(Key::Left, false));
        view.input(&key(Key::Right, true));
        view.tick(REST_OF_TICK);
        view.tick(TICK / 2);
        // From 357 to 0: half way is 358.5 again, not 178.5.
        assert_eq!(player(&view).heading, 0.0);
        assert!(
            (view.shown_heading() - 358.5).abs() < 1e-3,
            "{}",
            view.shown_heading()
        );
    }

    // Drawing.

    fn drawn(view: &FlightView) -> DrawList {
        let mut list = DrawList::new();
        view.draw(&mut list);
        list
    }

    fn overlay(text: &str, origin: Point, size: f32, color: Color) -> DrawCommand {
        DrawCommand::Text {
            text: text.to_owned(),
            font: Font::Geneva,
            origin,
            size,
            wrap_width: None,
            color,
        }
    }

    fn sprites(list: &DrawList) -> Vec<(ImageKey, Point)> {
        list.iter()
            .filter_map(|command| match command {
                DrawCommand::Sprite { image, center, .. } => Some((*image, *center)),
                _ => None,
            })
            .collect()
    }

    fn lines(list: &DrawList) -> Vec<DrawCommand> {
        list.iter()
            .filter(|c| matches!(c, DrawCommand::Line { .. }))
            .cloned()
            .collect()
    }

    /// The radar dots drawn: the dots in the radar's colour.
    fn dots_on_radar(list: &DrawList) -> Vec<Point> {
        let radar = layout().bright_radar;
        list.iter()
            .filter_map(|command| match *command {
                DrawCommand::Dot { center, color, .. } if color == radar => Some(center),
                _ => None,
            })
            .collect()
    }

    fn texts(list: &DrawList) -> Vec<String> {
        list.iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn it_draws_the_stars_the_stellars_the_ship_the_overlay_then_the_hud() {
        let view = flight();
        let list = drawn(&view);
        let mut expected = DrawList::new();
        starfield::draw(&mut expected, &view.camera());
        scene::draw_stellars(
            &mut expected,
            view.scene().expect("a scene"),
            &view.camera(),
            Duration::ZERO,
        );
        expected.sprite(ImageKey::sprite(2000, 0), VIEW_CENTER, Color::WHITE);
        expected.push(overlay("Sol (sÿst 130)", TITLE, TITLE_SIZE, Color::WHITE));
        expected.push(overlay(HELP, HELP_AT, OVERLAY_SIZE, Color::DIM));
        hud::draw(
            &mut expected,
            view.status_bar().expect("a status bar"),
            &HudState {
                position: at(0.0, 0.0),
                stellars: &[at(0.0, -600.0), at(300.0, -200.0)],
                reserves: start().reserves,
                system: "Sol",
            },
        );
        assert_eq!(list, expected);
        assert!(matches!(list.iter().next(), Some(DrawCommand::Dot { .. })));
        assert_eq!(
            sprites(&list),
            [
                (ImageKey::sprite(1128, 0), at(512.0, -216.0)),
                (ImageKey::sprite(1129, 0), at(812.0, 184.0)),
                (ImageKey::sprite(2000, 0), VIEW_CENTER),
            ]
        );
        assert_eq!(
            HELP,
            "Up: thrust   Left/Right: turn   Down: reverse   L: land   Esc: leave flight"
        );
        assert_eq!((TITLE, HELP_AT), (at(16.0, 32.0), at(16.0, 744.0)));
    }

    #[test]
    fn the_ship_stays_at_the_screens_centre_as_the_stellars_go_by() {
        let mut view = flight();
        view.input(&key(Key::Up, true));
        view.input(&key(Key::Right, true));
        view.tick(Duration::from_millis(1500));
        let list = drawn(&view);
        let shown = sprites(&list);
        let camera = view.camera();
        assert_eq!(
            shown[2],
            (
                ImageKey::sprite(2000, view.frame().expect("a sheet")),
                VIEW_CENTER
            )
        );
        assert_ne!(view.frame(), Some(0));
        assert_eq!(shown[0].1, camera.world_to_screen(at(0.0, -600.0)));
        assert_ne!(shown[0].1, at(512.0, -216.0), "the stellars moved");
        let mut stars = DrawList::new();
        starfield::draw(&mut stars, &camera);
        assert!(
            list.iter().take(stars.len()).eq(stars.iter()),
            "stars under the camera"
        );
    }

    #[test]
    fn the_stellars_animate_with_time() {
        let mut view = flight();
        ticks(&mut view, 1);
        assert_eq!(sprites(&drawn(&view))[1].0, ImageKey::sprite(1129, 1));
        ticks(&mut view, 2);
        assert_eq!(sprites(&drawn(&view))[1].0, ImageKey::sprite(1129, 3));
    }

    #[test]
    fn a_ship_without_a_sheet_is_a_placeholder_with_the_reason() {
        let sheetless = FakeCatalog {
            sheet: Err("no shän 128 for shïp 128".to_owned()),
            ..catalog()
        };
        let view = FlightView::new(&sheetless);
        assert_eq!(view.frame(), None);
        let list = drawn(&view);
        assert_eq!(sprites(&list).len(), 2, "the stellars only");
        let mut expected = DrawList::new();
        crossed_box(&mut expected, VIEW_CENTER, PLACEHOLDER_SIZE, PLACEHOLDER);
        assert_eq!(
            lines(&list)[..expected.len()],
            *lines(&expected),
            "then the HUD's"
        );
        let reason = list
            .iter()
            .find(|c| matches!(c, DrawCommand::Text { text, .. } if text.starts_with("Sprite")))
            .cloned();
        assert_eq!(
            reason,
            Some(overlay(
                "Sprite unavailable: no shän 128 for shïp 128",
                at(
                    VIEW_CENTER.x - PLACEHOLDER_SIZE / 2.0,
                    VIEW_CENTER.y + PLACEHOLDER_SIZE / 2.0 + MESSAGE_GAP
                ),
                OVERLAY_SIZE,
                Color::ERROR
            ))
        );
        assert!(texts(&list).contains(&"Sol (sÿst 130)".to_owned()));
    }

    // The HUD.

    #[test]
    fn a_new_pilot_shows_the_default_status_bar() {
        let view = flight();
        assert_eq!(
            view.status_bar(),
            Ok(&StatusBar {
                layout: layout(),
                background: Some(hud::Background {
                    id: 700,
                    width: 194.0,
                    height: 767.0,
                }),
            })
        );
    }

    #[test]
    fn the_hud_shows_the_status_bar_full_bars_and_the_stellars_on_radar() {
        let view = flight();
        let list = drawn(&view);
        assert!(
            list.iter().any(|c| *c
                == DrawCommand::Picture {
                    image: ImageKey::picture(700),
                    top_left: at(830.0, 0.0),
                }),
            "{list:?}"
        );
        // The shïp's shield 40, armour 60 and fuel 250: two whole jumps,
        // then half of one.
        let width = 149.0;
        let bar = |top: f32, from: f32, to: f32, color| DrawCommand::Line {
            from: at(865.0 + from, top + 3.5),
            to: at(865.0 + to, top + 3.5),
            width: 7.0,
            color,
        };
        let layout = layout();
        assert_eq!(
            lines(&list),
            [
                bar(199.0, 0.0, width, layout.shield_color),
                bar(216.0, 0.0, width, layout.armor_color),
                bar(234.0, 0.0, width * 0.8, layout.fuel_full),
                bar(234.0, width * 0.8, width, layout.fuel_partial),
            ]
        );
        let radar = layout.radar.offset(at(830.0, 0.0));
        let on_radar = |stellar| hud::radar_point(radar, at(0.0, 0.0), stellar);
        assert_eq!(
            dots_on_radar(&list),
            [
                on_radar(at(0.0, -600.0)).expect("in range"),
                on_radar(at(300.0, -200.0)).expect("in range"),
            ]
        );
        assert_eq!(texts(&list).last().map(String::as_str), Some("Sol"));
    }

    #[test]
    fn the_radar_follows_the_ship_as_drawn() {
        let mut view = flight();
        let before = dots_on_radar(&drawn(&view));
        view.input(&key(Key::Up, true));
        view.tick(Duration::from_millis(1500));
        view.tick(TICK / 2);
        let shown = view.shown_position();
        assert!(shown.y < -10.0, "{shown:?}");
        let after = dots_on_radar(&drawn(&view));
        let radar = layout().radar.offset(at(830.0, 0.0));
        assert_eq!(
            after,
            [
                hud::radar_point(radar, shown, at(0.0, -600.0)).expect("in range"),
                hud::radar_point(radar, shown, at(300.0, -200.0)).expect("in range"),
            ]
        );
        assert!(after[0].y > before[0].y, "{before:?} {after:?}");
    }

    #[test]
    fn the_bars_show_the_sessions_reserves() {
        let view = flight();
        let reserves = player(&view).reserves;
        assert_eq!(reserves, Reserves::from_fields(FIELDS));
        let mut expected = DrawList::new();
        hud::draw(
            &mut expected,
            view.status_bar().expect("a status bar"),
            &HudState {
                position: at(0.0, 0.0),
                stellars: &[],
                reserves,
                system: "Sol",
            },
        );
        assert_eq!(lines(&drawn(&view)), lines(&expected));
    }

    #[test]
    fn an_unreadable_status_bar_says_why_and_flight_goes_on() {
        let barless = FakeCatalog {
            bar: Err("no ïntf 128".to_owned()),
            ..catalog()
        };
        let mut view = FlightView::new(&barless);
        assert_eq!(view.status_bar(), Err("no ïntf 128"));
        view.input(&key(Key::Up, true));
        ticks(&mut view, 5);
        assert_eq!(player(&view), stepped(start(), THRUST, 5));
        let list = drawn(&view);
        let mut reason = DrawList::new();
        hud::draw_unavailable(&mut reason, "no ïntf 128");
        assert_eq!(list.iter().last(), reason.iter().next());
        assert_eq!(lines(&list), [], "no bars");
        assert_eq!(dots_on_radar(&list), [], "no radar");
        assert_eq!(sprites(&list).len(), 3, "stellars and ship");
    }

    #[test]
    fn a_session_that_cannot_start_has_no_status_bar() {
        let failed = FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..catalog()
        };
        let view = FlightView::new(&failed);
        assert_eq!(view.status_bar(), Err("no chär to start from"));
    }

    // Frame rate.

    /// Flies a script for three seconds of frames at `fps`, each redraw at
    /// `n / fps` seconds to the nanosecond: thrust for a second, turn
    /// right for half a second, reverse for a second, then coast.
    fn fly(fps: u64) -> ShipState {
        let mut view = flight();
        let script = [
            (0, Key::Up, true),
            (2, Key::Up, false),
            (2, Key::Right, true),
            (3, Key::Right, false),
            (3, Key::Down, true),
            (5, Key::Down, false),
        ];
        let mut last = 0;
        for frame in 1..=3 * fps {
            let half_second = (frame - 1) * 2 / fps;
            if (frame - 1) * 2 % fps == 0 {
                for &(_, k, pressed) in script.iter().filter(|(at, ..)| *at == half_second) {
                    view.input(&key(k, pressed));
                }
            }
            let now = frame * 1_000_000_000 / fps;
            view.tick(Duration::from_nanos(now - last));
            last = now;
        }
        player(&view)
    }

    #[test]
    fn the_same_keys_fly_the_same_at_30_60_and_120_fps() {
        let at_30 = fly(30);
        assert!(at_30.heading > 0.0, "{at_30:?}");
        assert!(at_30.position.y < 0.0, "{at_30:?}");
        assert_eq!(fly(60), at_30);
        assert_eq!(fly(120), at_30);
    }

    // Landing.

    /// The fake catalog with system 130 holding just `sites`.
    fn flight_among(sites: Vec<LandingSite>) -> FlightView {
        FlightView::new(&FakeCatalog { sites, ..catalog() })
    }

    const LAND: Key = Key::Char('l');

    /// The message drawn at its place, if any.
    fn message(view: &FlightView) -> Option<DrawCommand> {
        drawn(view)
            .iter()
            .find(|c| matches!(c, DrawCommand::Text { origin, .. } if *origin == MESSAGE_AT))
            .cloned()
    }

    #[test]
    fn l_over_a_landable_stellar_lands_once() {
        let mut view = flight_among(vec![site(140, (6.0, -8.0), StellarFlags::CAN_LAND)]);
        assert_eq!(view.take_landing(), None);
        assert_eq!(view.input(&key(LAND, true)), ScreenAction::None);
        assert_eq!(
            view.session().expect("flying").landed(),
            Some(StellarId(140))
        );
        assert_eq!(view.take_landing(), Some(StellarId(140)));
        assert_eq!(view.take_landing(), None, "taken");
        assert_eq!(view.message(), None);
        assert_eq!(message(&view), None);
        assert_eq!(LAND_KEY, LAND);
    }

    #[test]
    fn a_repeat_or_release_of_l_does_nothing() {
        let mut view = flight_among(vec![site(140, (0.0, 0.0), StellarFlags::CAN_LAND)]);
        view.input(&held(LAND));
        view.input(&key(LAND, false));
        assert_eq!(view.take_landing(), None);
        assert_eq!(view.session().expect("flying").landed(), None);
        let mut far = flight();
        far.input(&held(LAND));
        assert_eq!(far.message(), None, "no refusal either");
    }

    #[test]
    fn each_refusal_says_why_in_the_originals_words() {
        let refused = |sites: Vec<LandingSite>| {
            let mut view = flight_among(sites);
            view.input(&key(LAND, true));
            assert_eq!(view.take_landing(), None);
            view.message().map(str::to_owned)
        };
        let centre = |flags, min_status| LandingSite {
            min_status,
            ..site(140, (0.0, 0.0), flags)
        };
        let station = StellarFlags::CAN_LAND | StellarFlags::STATION;
        assert_eq!(
            refused(Vec::new()).as_deref(),
            Some("No stellar objects present.")
        );
        assert_eq!(
            refused(vec![site(140, (0.0, -600.0), StellarFlags::CAN_LAND)]).as_deref(),
            Some("You're too far away to land on this planet.")
        );
        assert_eq!(
            refused(vec![site(140, (0.0, -600.0), station)]).as_deref(),
            Some("You're too far away to dock at this station.")
        );
        assert_eq!(
            refused(vec![centre(0, 0)]).as_deref(),
            Some("The planet's environment is too hostile.")
        );
        assert_eq!(
            refused(vec![centre(StellarFlags::CAN_LAND, 1)]).as_deref(),
            Some("Landing request denied.")
        );
        assert_eq!(
            refused(vec![centre(station, 32767)]).as_deref(),
            Some("Docking request denied.")
        );
    }

    #[test]
    fn a_ship_moving_too_fast_is_told_so() {
        let big = |flags| LandingSite {
            frame_size: Some((400, 400)),
            ..site(140, (0.0, 0.0), flags)
        };
        for (flags, expected) in [
            (
                StellarFlags::CAN_LAND,
                "You're moving too fast to land on this planet.",
            ),
            (
                StellarFlags::CAN_LAND | StellarFlags::STATION,
                "You're moving too fast to dock at this station.",
            ),
        ] {
            let mut view = flight_among(vec![big(flags)]);
            view.input(&key(Key::Up, true));
            ticks(&mut view, 15);
            view.input(&key(LAND, true));
            assert_eq!(view.take_landing(), None);
            assert_eq!(view.message(), Some(expected));
        }
    }

    #[test]
    fn every_refusal_has_its_string() {
        let stellar = StellarId(128);
        let cases = [
            (LandingRefusal::NoStellars, NO_STELLARS),
            (
                LandingRefusal::TooFar {
                    nearest: stellar,
                    station: true,
                },
                TOO_FAR_STATION,
            ),
            (
                LandingRefusal::TooFar {
                    nearest: stellar,
                    station: false,
                },
                TOO_FAR_PLANET,
            ),
            (
                LandingRefusal::NotLandable {
                    stellar,
                    station: true,
                },
                HOSTILE_STATION,
            ),
            (
                LandingRefusal::NotLandable {
                    stellar,
                    station: false,
                },
                HOSTILE_PLANET,
            ),
            (
                LandingRefusal::Denied {
                    stellar,
                    station: true,
                    min_status: 1,
                },
                DOCKING_DENIED,
            ),
            (
                LandingRefusal::Denied {
                    stellar,
                    station: false,
                    min_status: 1,
                },
                LANDING_DENIED,
            ),
            (
                LandingRefusal::TooFast {
                    stellar,
                    station: true,
                    speed: 2.0,
                },
                TOO_FAST_STATION,
            ),
            (
                LandingRefusal::TooFast {
                    stellar,
                    station: false,
                    speed: 2.0,
                },
                TOO_FAST_PLANET,
            ),
        ];
        for (refusal, text) in cases {
            assert_eq!(refusal_message(&refusal), text, "{refusal:?}");
        }
        assert_eq!(
            [
                NO_STELLARS,
                TOO_FAR_STATION,
                TOO_FAR_PLANET,
                TOO_FAST_STATION,
                TOO_FAST_PLANET,
                DOCKING_DENIED,
                LANDING_DENIED,
                HOSTILE_STATION,
                HOSTILE_PLANET,
            ],
            [
                "No stellar objects present.",
                "You're too far away to dock at this station.",
                "You're too far away to land on this planet.",
                "You're moving too fast to dock at this station.",
                "You're moving too fast to land on this planet.",
                "Docking request denied.",
                "Landing request denied.",
                "The station's hull integrity is too unstable.",
                "The planet's environment is too hostile.",
            ]
        );
    }

    #[test]
    fn the_refusal_is_drawn_above_the_help_line_until_it_has_been_shown_long_enough() {
        let mut view = flight();
        ticks(&mut view, 3);
        view.input(&key(LAND, true));
        let shown = Some(overlay(
            TOO_FAR_PLANET,
            MESSAGE_AT,
            OVERLAY_SIZE,
            Color::WHITE,
        ));
        assert_eq!(message(&view), shown);
        // After the help line.
        let list = drawn(&view);
        let commands: Vec<&DrawCommand> = list.iter().collect();
        let help = commands
            .iter()
            .position(|c| **c == overlay(HELP, HELP_AT, OVERLAY_SIZE, Color::DIM))
            .expect("the help line");
        assert_eq!(Some(commands[help + 1]), shown.as_ref());
        view.tick(Duration::from_millis(3999));
        assert_eq!(message(&view), shown);
        assert_eq!(view.message(), Some(TOO_FAR_PLANET));
        view.tick(Duration::from_millis(1));
        assert_eq!(message(&view), None);
        assert_eq!(view.message(), None);
        assert_eq!(MESSAGE_SHOWN_FOR, Duration::from_secs(4));
        assert_eq!(MESSAGE_AT, at(16.0, 720.0));
    }

    #[test]
    fn a_new_refusal_shows_afresh() {
        let mut view = flight();
        view.input(&key(LAND, true));
        view.tick(Duration::from_secs(3));
        view.input(&key(LAND, false));
        view.input(&key(LAND, true));
        view.tick(Duration::from_secs(2));
        assert_eq!(view.message(), Some(TOO_FAR_PLANET));
    }

    #[test]
    fn taking_off_draws_the_ship_at_the_stellar_on_the_next_frame() {
        let mut view = flight_among(vec![site(140, (6.0, -8.0), StellarFlags::CAN_LAND)]);
        view.input(&key(LAND, true));
        assert_eq!(view.take_landing(), Some(StellarId(140)));
        assert_eq!(view.take_off(), Some(StellarId(140)));
        assert_eq!(view.session().expect("flying").landed(), None);
        assert_eq!(view.alpha(), 0.0);
        assert_eq!(view.shown_position(), at(6.0, -8.0));
        assert_eq!(view.camera().center(), at(6.0, -8.0));
        assert_eq!(view.take_off(), None, "not landed");
        // It flies on from there.
        view.input(&key(Key::Up, true));
        ticks(&mut view, 2);
        assert!(player(&view).position.y < -8.0, "{:?}", player(&view));
    }

    #[test]
    fn a_refusal_is_forgotten_once_the_ship_lands() {
        let mut view = flight_among(vec![site(140, (0.0, -12.0), StellarFlags::CAN_LAND)]);
        view.input(&key(LAND, true));
        view.input(&key(LAND, false));
        assert_eq!(view.message(), Some(TOO_FAR_PLANET));
        // Nudge the ship and drift over the planet.
        view.input(&key(Key::Up, true));
        ticks(&mut view, 1);
        view.input(&key(Key::Up, false));
        while player(&view).position.y > -4.0 {
            ticks(&mut view, 1);
        }
        assert!(view.elapsed < MESSAGE_SHOWN_FOR, "still on screen");
        view.input(&key(LAND, true));
        assert_eq!(view.take_landing(), Some(StellarId(140)));
        assert_eq!(view.message(), None);
        assert_eq!(message(&view), None);
        assert_eq!(view.take_off(), Some(StellarId(140)));
        assert_eq!(view.message(), None, "not back after take-off");
    }

    #[test]
    fn landing_in_a_flight_that_never_started_does_nothing() {
        let mut view = FlightView::new(&FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..catalog()
        });
        view.input(&key(LAND, true));
        assert_eq!(view.take_landing(), None);
        assert_eq!(view.message(), None);
        assert_eq!(view.take_off(), None);
    }
}
