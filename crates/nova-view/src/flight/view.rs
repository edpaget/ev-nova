//! The flight screen: the player's ship flying from its starting system,
//! over the parallax starfield and among the system's stellars, with the
//! camera following the ship, and jumping through hyperspace along the
//! course plotted on the galaxy map.
//!
//! The screen owns a [`nova_sim::Session`], flying a new pilot
//! ([`FlightView::new`]) or a given one ([`FlightView::with_pilot`]), and
//! its catalog. It reads, once
//! when it is built: the session's system, through the [`SystemCatalog`]
//! port, the ship's sprite sheet, through the [`ShipSprites`] port, the
//! HUD's status bar for the player's government, through the
//! [`StatusBars`] port, and the galaxy for its course map, through the
//! [`GalaxyCatalog`] port. After that it reads only when the ship arrives
//! in another system: that system. Drawing and input never read anything.
//!
//! Each day a jump takes rolls the planetary events on the screen's
//! [`SharedChance`] ([`FlightView::with_chance`]); without one, nothing
//! random happens. Landed at a trade center, the session's exchange can be
//! read ([`FlightView::market`]) and traded on ([`FlightView::trade`]);
//! landed at an outfitter, so can its outfitter ([`FlightView::outfitter`],
//! [`FlightView::outfit`]); and landed at a shipyard, a new ship can be
//! bought ([`FlightView::shipyard`], [`FlightView::buy_ship`]), whose
//! sprite sheet is read then.
//!
//! The HUD is drawn last, over everything: the status bar against the
//! right edge, its radar showing the stellars around the ship as drawn,
//! its bars the session's shield, armour and fuel, and its nav area the
//! navigation target or else the course. Without a status bar it says
//! why, and flight goes on.
//!
//! Each [`Screen::tick`] runs the simulation's fixed-step clock: the
//! frame's time becomes whole steps of 1/30 s, each flown with the keys
//! held, and what is left over is how far the display is between the last
//! two steps. The ship, and the camera with it, are drawn that far from
//! the step before the last towards the last, so motion is smooth at any
//! frame rate while the simulation itself never depends on it.
//!
//! The ship is drawn as its sheet's rotation frame for the heading shown,
//! then, at the same centre and on the same frame (modulo the layer's
//! frame count), its engine glow and its running lights. The glow is
//! drawn at the level [`nova_sim::glow_level`] gives for the session's
//! glow base ([`Session::engine_glow`]) at the flight's time in ticks: it
//! fades in over 24 ticks of thrust, flickers, and fades out over 24
//! ticks after Up is released, and landing and beginning a jump put it
//! out. The lights are drawn at the level [`nova_sim::lights_level`]
//! gives for the sheet's blink at the flight's time in ticks. Each is
//! drawn at [`lights_tint`] of its level, or not at all while it is hidden
//! or off. Both are combined by OR with what is beneath
//! ([`Blend::Or`](crate::Blend::Or)) at every level, scaled by level/32,
//! like the original's `_BlitPixieRLETranslucent` (0xc1568) below full and
//! `_BlitPixieRLEAddOver` (0xc24bf) at full: see [`lights_tint`] for the
//! full record. The flight's time
//! stops while the course map is open, so the lights blink and the glow
//! flickers in game time. Random blinking rolls on [`HashedRolls`] from
//! seed 0, and the glow's flicker on [`HashedRolls`] from seed 1. A layer
//! that cannot be shown is left out silently.
//!
//! Input, the original's default keys:
//!
//! - Up thrusts, Left and Right turn, and Down turns to face against the
//!   ship's motion, while held: a press (or its key repeats) holds the key
//!   and its release lets it go. Left and Right together cancel, and
//!   either overrides Down.
//! - L, once a press (its repeats do nothing), requests clearance, then
//!   lands, as [`landing`](nova_sim::landing) says: with no navigation
//!   target it selects the nearest landable stellar and shows the reply
//!   ([`clearance_message`]); with one it lands there. The router takes
//!   the landing ([`FlightView::take_landing`]) and shows the spaceport.
//!   The reply, or why a landing was refused, shows above the help line
//!   in the original's words (`STR#` 2002), for [`MESSAGE_SHOWN_FOR`].
//! - M (a press, not its repeats) opens the course map, a [`GalaxyMap`]
//!   in [`MapMode::Course`](crate::galaxy::MapMode::Course), and lets go
//!   of the flight keys. While it is open flight is paused, as in the
//!   original, only the map is drawn and every input goes to it, except
//!   that M closes it ([`FlightView::close_map`] closes it too, for the
//!   router's Escape). A system clicked on the map becomes the
//!   destination: the session plots the course there and the map shows
//!   it. The map shows the systems the pilot has explored, and the rest
//!   unexplored.
//! - Tab (a press, not its repeats) selects the next of the system's
//!   stellars as the navigation target, as
//!   [`navigation`](nova_sim::navigation) says. The HUD's nav area shows
//!   the target, or else the next system on the course (see
//!   [`hud`](super::hud)).
//! - J (a press) jumps to the next system on the course when the session
//!   allows it, and otherwise says why in the original's words (`STR#`
//!   2002), as a refused landing does. A jump plays its [`JumpEffect`]:
//!   the keys are let go and ignored and the session waits while the stars
//!   streak and the screen fades out; then the ship arrives, the new
//!   system is read and laid out, and it fades in. The HUD stays on top
//!   throughout.
//! - Escape belongs to the app's router, which closes the map or leaves
//!   flight. The screen never quits.

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::time::Duration;

use nova_sim::{
    Chance, Clearance, Controls, FixedStep, HashedRolls, JumpRefusal, LandOutcome, LandingRefusal,
    Market, NeverFires, Order, OutfitOrder, OutfitRefusal, Outfitter, Pilot, PilotCatalog,
    RechargeRefusal, Reserves, Session, ShipId, ShipPurchase, ShipRefusal, ShipState, Shipyard,
    StartError, StellarId, Steps, TradeRefusal, Turn, flight::normalized, flight::shortest_turn,
    glow_level, lights_level,
};

use super::catalog::{ShipSheet, ShipSprites, StatusBars};
use super::hud::{self, HudState, NavDisplay, StatusBar};
use super::jump::{JumpEffect, JumpPhase};
use super::sprite::rotation_frame;
use crate::draw::{crossed_box, lights_tint};
use crate::galaxy::{GalaxyCatalog, GalaxyMap};
use crate::system::camera::Camera;
use crate::system::catalog::SystemCatalog;
use crate::system::scene::{self, PLACEHOLDER, PLACEHOLDER_SIZE, SystemScene};
use crate::system::starfield;
use crate::time::ticks;
use crate::{Color, DrawList, ImageKey, Input, Key, Point, Screen, ScreenAction, Sound};

/// The overlay: the system's title and the help line.
const TITLE: Point = Point::new(16.0, 32.0);
const TITLE_SIZE: f32 = 20.0;
const HELP_AT: Point = Point::new(16.0, 744.0);
const OVERLAY_SIZE: f32 = 14.0;
/// How far below the ship's placeholder the reason goes.
const MESSAGE_GAP: f32 = 22.0;
/// The help line.
pub const HELP: &str = "Up: thrust   Left/Right: turn   Down: reverse   Tab: target   L: land   M: map   J: jump   P: preferences   Esc: leave flight";
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
/// The galaxy map key: the original's documented default (`Keys.nib`'s
/// `mapKey`; `STR#` 2002 #26-28 name a map key).
pub const MAP_KEY: Key = Key::Char('m');
/// The hyperspace jump key: the original's documented default
/// (`Keys.nib`'s `jumpKey`).
pub const JUMP_KEY: Key = Key::Char('j');
/// The key that selects the next stellar as the navigation target: the
/// original's default, Tab.
pub const TARGET_KEY: Key = Key::Tab;

/// `STR#` 2002 #29.
pub const NO_DESTINATION: &str =
    "You have to select a destination before you can start a hyperspace jump.";
/// `STR#` 2002 #42.
pub const TOO_CLOSE: &str =
    "Can't initiate hyperspace jump - not yet far enough away from system center.";
/// `STR#` 2002 #10.
pub const NO_FUEL: &str = "Insufficient energy for hyperspace jump.";
/// Not the original's, which never flies a landed ship: worded after
/// `STR#` 2002 #42 and #73 ("Disengage cloaking device first.").
pub const TAKE_OFF_FIRST: &str = "Can't initiate hyperspace jump - take off first.";

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
/// Not the original's, which takes no keys during a jump: worded after
/// `STR#` 2002 #54 ("Unable to send hail - target ship is entering
/// hyperspace.").
pub const IN_HYPERSPACE: &str = "Unable to land - your ship is in hyperspace.";

/// `STR#` 2002 #76.
pub const DOCKMASTER_READS_YOU: &str = "dockmaster reads you";
/// `STR#` 2002 #78.
pub const TRAFFIC_CONTROL_READS_YOU: &str = "traffic control reads you";
/// `STR#` 2002 #95.
pub const CLEARED_TO_DOCK: &str = "you're cleared to dock.";
/// `STR#` 2002 #96.
pub const YOU_ARE_CLEARED_TO_DOCK: &str = "You are cleared to dock.";
/// `STR#` 2002 #98.
pub const CLEARED_TO_LAND: &str = "you're cleared to land.";
/// `STR#` 2002 #99.
pub const YOU_ARE_CLEARED_TO_LAND: &str = "You are cleared to land.";

/// What the player is told when L requests clearance at the stellar
/// `name`, a station or a planet, and `clearance` is the reply. Only the
/// pieces are the original's (`STR#` 2002); how they are joined is a
/// reconstruction:
///
/// - granted: "{name} traffic control reads you, you're cleared to land."
///   (#78, #98), or at a station "{name} dockmaster reads you, you're
///   cleared to dock." (#76, #95);
/// - uninhabited, with no traffic control to answer: "You are cleared to
///   land." (#99) or "You are cleared to dock." (#96);
/// - denied: "Landing request denied." (#83) or "Docking request denied."
///   (#82).
#[must_use]
pub fn clearance_message(name: &str, station: bool, clearance: Clearance) -> String {
    let (reads_you, cleared, you_are_cleared, denied) = if station {
        (
            DOCKMASTER_READS_YOU,
            CLEARED_TO_DOCK,
            YOU_ARE_CLEARED_TO_DOCK,
            DOCKING_DENIED,
        )
    } else {
        (
            TRAFFIC_CONTROL_READS_YOU,
            CLEARED_TO_LAND,
            YOU_ARE_CLEARED_TO_LAND,
            LANDING_DENIED,
        )
    };
    match clearance {
        Clearance::Granted => format!("{name} {reads_you}, {cleared}"),
        Clearance::NoTrafficControl => you_are_cleared.to_owned(),
        Clearance::Denied => denied.to_owned(),
    }
}

/// What the player is told when `refusal` stops a landing: the original's
/// words for it, for a station or a planet.
#[must_use]
pub fn refusal_message(refusal: &LandingRefusal) -> &'static str {
    let pick = |station: bool, at_station, on_planet| {
        if station { at_station } else { on_planet }
    };
    match *refusal {
        LandingRefusal::Jumping => IN_HYPERSPACE,
        LandingRefusal::NoStellars => NO_STELLARS,
        LandingRefusal::TooFar { station, .. } => pick(station, TOO_FAR_STATION, TOO_FAR_PLANET),
        LandingRefusal::NotLandable { station, .. } => {
            pick(station, HOSTILE_STATION, HOSTILE_PLANET)
        }
        LandingRefusal::Denied { station, .. } => pick(station, DOCKING_DENIED, LANDING_DENIED),
        LandingRefusal::TooFast { station, .. } => pick(station, TOO_FAST_STATION, TOO_FAST_PLANET),
    }
}

/// What the player is told when `refusal` stops a jump: the original's
/// words for it.
#[must_use]
pub fn jump_refusal_message(refusal: &JumpRefusal) -> &'static str {
    match refusal {
        JumpRefusal::NoDestination => NO_DESTINATION,
        JumpRefusal::TooClose { .. } => TOO_CLOSE,
        JumpRefusal::NoFuel { .. } => NO_FUEL,
        JumpRefusal::Landed => TAKE_OFF_FIRST,
    }
}

/// A [`Chance`] shared by whoever holds a copy: the app's one source of
/// randomness, handed to each flight. The default never fires.
#[derive(Clone)]
pub struct SharedChance(Rc<RefCell<dyn Chance>>);

impl SharedChance {
    /// The source `chance`, shared.
    #[must_use]
    pub fn new(chance: Rc<RefCell<dyn Chance>>) -> Self {
        Self(chance)
    }
}

impl Default for SharedChance {
    fn default() -> Self {
        Self(Rc::new(RefCell::new(NeverFires)))
    }
}

impl std::fmt::Debug for SharedChance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Chance")
    }
}

impl Chance for SharedChance {
    fn fires(&mut self, percent: u8) -> bool {
        self.0.borrow_mut().fires(percent)
    }
}

/// The player's ship in flight, reading the game data from `C`.
#[derive(Clone, Debug)]
pub struct FlightView<C> {
    /// What the screen reads: when it is built, and on each arrival.
    catalog: C,
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
    /// Time since the view opened, which drives the stellars' animations
    /// and the running lights' blinking. It stops while the map is open.
    elapsed: Duration,
    /// The flight keys held down.
    held: HashSet<Key>,
    /// The stellar landed on, until the router takes it.
    pending_landing: Option<StellarId>,
    /// The message shown, and `elapsed` when it was shown.
    message: Option<(String, Duration)>,
    /// The course map, shown or not.
    map: GalaxyMap,
    /// Whether the course map is shown.
    map_open: bool,
    /// The jump's effect, while it plays.
    jump: Option<JumpEffect>,
    /// What each day's events are rolled on.
    chance: SharedChance,
    /// What random running lights roll on.
    blink_rolls: HashedRolls,
    /// What the engine glow's flicker rolls on: seed 1, so it never
    /// mirrors random-mode lights, which roll on seed 0.
    glow_rolls: HashedRolls,
}

impl<C: PilotCatalog + SystemCatalog + ShipSprites + StatusBars + GalaxyCatalog> FlightView<C> {
    /// A new, unnamed pilot's flight, read from `catalog`, which the
    /// screen keeps.
    pub fn new(catalog: C) -> Self {
        let session = Session::start(&catalog);
        Self::flying(catalog, session)
    }

    /// `pilot`'s flight, read from `catalog`, which the screen keeps. A
    /// pilot docked at a stellar resumes landed there: the landing is
    /// reported once ([`FlightView::take_landing`]), with no sound, so the
    /// router shows the spaceport.
    pub fn with_pilot(catalog: C, pilot: Pilot) -> Self {
        let session = Session::fly(&catalog, pilot);
        Self::flying(catalog, session)
    }

    fn flying(catalog: C, session: Result<Session, StartError>) -> Self {
        let session = session.map_err(|err| err.to_string());
        let (scene, sheet, status_bar) = match &session {
            Ok(session) => (
                Some(SystemScene::load(&catalog, session.system())),
                catalog.ship_sheet(session.ship()),
                hud::choose_status_bar(&catalog, session.government()),
            ),
            Err(reason) => (None, Err(reason.clone()), Err(reason.clone())),
        };
        let previous = session
            .as_ref()
            .map(|session| *session.player())
            .unwrap_or_default();
        let mut map = GalaxyMap::course(&catalog);
        if let Ok(session) = &session {
            map.show_course(session.system(), session.course());
            map.show_explored(session.pilot().explored());
        }
        let pending_landing = session.as_ref().ok().and_then(Session::landed);
        Self {
            catalog,
            map,
            map_open: false,
            jump: None,
            session,
            scene,
            sheet,
            status_bar,
            clock: FixedStep::new(),
            previous,
            alpha: 0.0,
            elapsed: Duration::ZERO,
            held: HashSet::new(),
            pending_landing,
            message: None,
            chance: SharedChance::default(),
            blink_rolls: HashedRolls::new(0),
            glow_rolls: HashedRolls::new(1),
        }
    }

    /// The flight with each day's events rolled on `chance`.
    #[must_use]
    pub fn with_chance(self, chance: SharedChance) -> Self {
        Self { chance, ..self }
    }

    /// The catalog the screen reads.
    #[must_use]
    pub fn catalog(&self) -> &C {
        &self.catalog
    }

    /// Lets go of the flight keys and shows the course map.
    fn open_map(&mut self) {
        self.held.clear();
        self.map_open = true;
    }

    /// The map's input; a destination clicked on it becomes the session's
    /// course, which the map then shows.
    fn map_input(&mut self, input: &Input) {
        self.map.input(input);
        let Some(destination) = self.map.take_destination() else {
            return;
        };
        if let Ok(session) = &mut self.session {
            // A failed plot clears the course, which the map then shows.
            let _ = session.plot_course(destination);
            self.map.show_course(session.system(), session.course());
        }
    }

    /// What the HUD's nav area shows: the selected stellar, by its name in
    /// `scene`; otherwise the next system on the course, by its name on the
    /// map if the pilot has explored it; otherwise nothing.
    fn nav_display(&self, scene: &SystemScene) -> NavDisplay {
        let Ok(session) = &self.session else {
            return NavDisplay::None;
        };
        if let Some(target) = session.nav_target() {
            let name = scene.stellars().iter().find(|stellar| stellar.id == target);
            return NavDisplay::Stellar(name.map(|s| s.name.clone()).unwrap_or_default());
        }
        session.course().first().map_or(NavDisplay::None, |&next| {
            let name = session
                .pilot()
                .has_explored(next)
                .then(|| self.map.model().system(next))
                .flatten()
                .map(|system| system.entry.name.clone());
            NavDisplay::Hyperspace(name)
        })
    }

    /// Begins a jump, or shows why not.
    fn jump(&mut self) {
        let Ok(session) = &mut self.session else {
            return;
        };
        let from = session.system();
        match session.begin_jump() {
            Ok(next) => {
                let position = |id| session.star_map().position(id).unwrap_or_default();
                self.jump = Some(JumpEffect::toward(position(from), position(next)));
                self.held.clear();
                self.message = None;
            }
            Err(refusal) => self.show(jump_refusal_message(&refusal).to_owned()),
        }
    }

    /// Ends the jump: the ship arrives in the next system, which is read
    /// and laid out, and drawn from where the ship arrives.
    fn arrive(&mut self) {
        let Ok(session) = &mut self.session else {
            return;
        };
        let Some(system) = session.arrive(&self.catalog, &mut self.chance) else {
            return;
        };
        self.scene = Some(SystemScene::load(&self.catalog, system));
        self.map.show_course(system, session.course());
        self.map.show_explored(session.pilot().explored());
        self.previous = *session.player();
        self.alpha = 0.0;
        self.message = None;
    }
}

impl<C: ShipSprites> FlightView<C> {
    /// Buys a ship as [`Session::buy_ship`] does, and reads the new ship's
    /// sprite sheet, so the new hull is drawn once it takes off; a session
    /// that failed has no shipyard.
    pub fn buy_ship(&mut self, ship: ShipId) -> Result<ShipPurchase, ShipRefusal> {
        let session = self.session.as_mut().map_err(|_| ShipRefusal::NoShipyard)?;
        let bought = session.buy_ship(ship)?;
        self.sheet = self.catalog.ship_sheet(ship);
        Ok(bought)
    }
}

impl<C> FlightView<C> {
    /// Whether the course map is shown.
    #[must_use]
    pub fn map_open(&self) -> bool {
        self.map_open
    }

    /// The course map, shown or not.
    #[must_use]
    pub fn course_map(&self) -> &GalaxyMap {
        &self.map
    }

    /// Closes the course map, abandoning any gesture on it, and goes back
    /// to flight.
    pub fn close_map(&mut self) {
        self.map.cancel_pointer();
        self.map.release_keys();
        self.map_open = false;
    }

    /// The jump's effect, while it plays.
    #[must_use]
    pub fn jump_effect(&self) -> Option<&JumpEffect> {
        self.jump.as_ref()
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
    pub fn message(&self) -> Option<&str> {
        let (text, shown_at) = self.message.as_ref()?;
        (self.elapsed < *shown_at + MESSAGE_SHOWN_FOR).then_some(text.as_str())
    }

    /// Shows `text` from now.
    fn show(&mut self, text: String) {
        self.message = Some((text, self.elapsed));
    }

    /// Presses the land key: requests clearance and shows the reply,
    /// lands, or shows why not.
    fn land(&mut self) {
        let Ok(session) = &mut self.session else {
            return;
        };
        match session.land() {
            Ok(LandOutcome::Selected {
                stellar,
                station,
                clearance,
            }) => {
                let name = self.scene.as_ref().and_then(|scene| {
                    let named = scene.stellars().iter().find(|named| named.id == stellar);
                    named.map(|named| named.name.as_str())
                });
                self.show(clearance_message(
                    name.unwrap_or_default(),
                    station,
                    clearance,
                ));
            }
            Ok(LandOutcome::Landed(stellar)) => {
                self.pending_landing = Some(stellar);
                self.message = None;
            }
            Err(refusal) => self.show(refusal_message(&refusal).to_owned()),
        }
    }

    /// The flight, or why it could not start.
    pub fn session(&self) -> Result<&Session, &str> {
        self.session.as_ref().map_err(String::as_str)
    }

    /// The pilot flying, if the flight started.
    #[must_use]
    pub fn pilot(&self) -> Option<&Pilot> {
        self.session.as_ref().ok().map(Session::pilot)
    }

    /// Changes the pilot with `change` while landed, as
    /// [`Session::transact`] does, and says whether it did.
    pub fn transact(&mut self, change: impl FnOnce(&mut Pilot)) -> bool {
        self.session
            .as_mut()
            .is_ok_and(|session| session.transact(change))
    }

    /// The exchange of the stellar landed on, as [`Session::market`] gives
    /// it; none for a session that failed.
    #[must_use]
    pub fn market(&self) -> Option<Market> {
        self.session.as_ref().ok()?.market()
    }

    /// Trades as [`Session::trade`] does; a session that failed has no
    /// exchange.
    pub fn trade(&mut self, order: Order) -> Result<u32, TradeRefusal> {
        match &mut self.session {
            Ok(session) => session.trade(order),
            Err(_) => Err(TradeRefusal::NoMarket),
        }
    }

    /// The outfitter of the stellar landed on, as [`Session::outfitter`]
    /// gives it; none for a session that failed.
    #[must_use]
    pub fn outfitter(&self) -> Option<Outfitter> {
        self.session.as_ref().ok()?.outfitter()
    }

    /// Buys or sells an outfit as [`Session::outfit`] does; a session that
    /// failed has no outfitter.
    pub fn outfit(&mut self, order: OutfitOrder) -> Result<(), OutfitRefusal> {
        match &mut self.session {
            Ok(session) => session.outfit(order),
            Err(_) => Err(OutfitRefusal::NoOutfitter),
        }
    }

    /// Recharges as [`Session::recharge`] does; a session that failed
    /// sells no fuel.
    pub fn recharge(&mut self) -> Result<i64, RechargeRefusal> {
        match &mut self.session {
            Ok(session) => session.recharge(),
            Err(_) => Err(RechargeRefusal::NoFuel),
        }
    }

    /// The shipyard of the stellar landed on, as [`Session::shipyard`]
    /// gives it; none for a session that failed.
    #[must_use]
    pub fn shipyard(&self) -> Option<Shipyard> {
        self.session.as_ref().ok()?.shipyard()
    }

    /// Whether the pilot should be saved, as [`Session::take_save_due`]
    /// says; taking it clears it.
    pub fn take_save_due(&mut self) -> bool {
        self.session.as_mut().is_ok_and(Session::take_save_due)
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

    /// Today's date as the HUD shows it, or none for a session that never
    /// started.
    fn date_text(&self) -> String {
        self.session
            .as_ref()
            .map(Session::date_text)
            .unwrap_or_default()
    }

    /// The ship's shield, armour and fuel, or none for a session that
    /// never started.
    fn reserves(&self) -> Reserves {
        self.session
            .as_ref()
            .map(Session::reserves)
            .unwrap_or_default()
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
                let tick = u64::try_from(ticks(self.elapsed)).unwrap_or(u64::MAX);
                let base = self.session.as_ref().map_or(0, Session::engine_glow);
                let glow = sheet
                    .glow
                    .zip(glow_level(base, tick, &self.glow_rolls).map(lights_tint));
                let level = lights_level(&sheet.blink, tick, &self.blink_rolls);
                let lights = sheet.lights.zip(level.map(lights_tint));
                for (layer, tint) in [glow, lights].into_iter().flatten() {
                    let layer_frame = frame % layer.frames.get();
                    list.or_sprite(ImageKey::sprite(layer.image_id, layer_frame), at, tint);
                }
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

impl<C: PilotCatalog + SystemCatalog + ShipSprites + StatusBars + GalaxyCatalog> Screen
    for FlightView<C>
{
    /// Never quits: Escape is the router's.
    fn input(&mut self, input: &Input) -> ScreenAction {
        if self.jump.is_some() {
            return ScreenAction::None;
        }
        let press = match *input {
            Input::Key {
                key,
                pressed: true,
                repeat: false,
            } => Some(key),
            _ => None,
        };
        if self.map_open {
            if press == Some(MAP_KEY) {
                self.close_map();
            } else {
                self.map_input(input);
            }
            return ScreenAction::None;
        }
        match press {
            Some(LAND_KEY) => self.land(),
            Some(MAP_KEY) => {
                self.open_map();
                return ScreenAction::None;
            }
            Some(JUMP_KEY) => {
                self.jump();
                return ScreenAction::None;
            }
            Some(TARGET_KEY) => {
                if let Ok(session) = &mut self.session {
                    session.select_next_stellar();
                }
                return ScreenAction::None;
            }
            _ => {}
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
    /// the stellars' animations. While the map is open nothing moves; while
    /// a jump plays only its effect does, and the ship arrives when the
    /// effect says.
    fn tick(&mut self, dt: Duration) {
        if self.map_open {
            return;
        }
        if let Some(effect) = &mut self.jump {
            self.elapsed += dt;
            let arrived = effect.advance(dt);
            let done = effect.done();
            if arrived {
                self.arrive();
            }
            if done {
                self.jump = None;
            }
            return;
        }
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
        if self.map_open {
            self.map.draw(list);
            return;
        }
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
        match self.jump.map(|effect| (effect, effect.phase())) {
            Some((effect, JumpPhase::Streak(_) | JumpPhase::FadeOut(_))) => {
                starfield::draw_streaked(list, &camera, effect.direction(), effect.streak_length());
            }
            _ => starfield::draw(list, &camera),
        }
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
        if let Some(effect) = &self.jump {
            effect.draw_fade(list);
        }
        match &self.status_bar {
            Ok(bar) => {
                let stellars: Vec<Point> = scene.stellars().iter().map(|s| s.position).collect();
                let date = self.date_text();
                let state = HudState {
                    position: self.shown_position(),
                    stellars: &stellars,
                    reserves: self.reserves(),
                    nav: self.nav_display(scene),
                    date: &date,
                };
                hud::draw(list, bar, &state);
            }
            Err(reason) => hud::draw_unavailable(list, reason),
        }
    }

    /// Abandons any gesture on the course map.
    fn cancel_pointer(&mut self) {
        self.map.cancel_pointer();
    }

    /// Lets go of every flight key: the ship stops thrusting and turning,
    /// and coasts.
    fn release_keys(&mut self) {
        self.held.clear();
        self.map.release_keys();
    }

    /// The session's sounds: thrust, landing, taking off and jumping. The
    /// course map has no buttons that sound.
    fn take_sounds(&mut self) -> Vec<Sound> {
        self.session
            .as_mut()
            .map(|session| session.take_sounds().into_iter().map(Sound::Sim).collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::RefCell;
    use std::num::NonZeroU16;

    use nova_sim::landing::{LandingRefusal, StellarFlags};
    use nova_sim::{
        CharacterStart, CommodityStrings, DateAffixes, DisasterRecord, Handling, JunkRecord,
        LandingSite, OutfitId, OutfitRecord, Reserves, ShipFields, ShipId, ShipRecord, ShipStats,
        SimSound, SoundId, StarSystem, StartDate, StartError, SystemId, TICK, Vec2, step,
    };

    use super::*;
    use crate::draw::lights_tint;
    use crate::flight::catalog::{Blink, GovtId, LayerSheet, StatusBarLayout};
    use crate::flight::hud::{self, HudState, NavDisplay, StatusBar};
    use crate::galaxy::{Galaxy, MapMode, SystemEntry};
    use crate::sound::Sound;
    use crate::system::camera::VIEW_CENTER;
    use crate::system::catalog::{
        AnimationData, StellarContents, StellarId, StellarSheet, SystemContents,
    };
    use crate::{Blend, DrawCommand, Font};
    use nova_sim::hyperspace::{JumpRefusal, MIN_JUMP_DISTANCE};
    use nova_sim::{BlinkChance, GLOW_CRUISE, HashedRolls, glow_level};

    /// The first `chär` flies ship 128 (an average ship that turns 3° a
    /// tick, with a 36-rotation, 40 x 40 sheet, `rlëD` 2000) from system
    /// 130, Sol: Earth at (0, -600) and Moon at (300, -200), which animates
    /// a frame a tick. On the map Sol is at (0, 0), linked to Alpha
    /// Centauri (131) at (600, 0), which holds Proxima at its centre;
    /// Barnard (132) at (0, 600) is linked to nothing. Records the systems
    /// read.
    struct FakeCatalog {
        character: Result<CharacterStart, StartError>,
        fields: ShipFields,
        sheet: Result<ShipSheet, String>,
        /// `ïntf` 128: stock-like, or why it cannot be read.
        bar: Result<StatusBarLayout, String>,
        /// System 130's landing sites: Earth and Moon, by default.
        sites: Vec<LandingSite>,
        systems_read: RefCell<Vec<SystemId>>,
        /// The commodities: none, by default.
        commodities: CommodityStrings,
        /// The planetary events: none, by default.
        disasters: Vec<DisasterRecord>,
        /// The outfits: none, by default.
        outfits: Vec<OutfitRecord>,
        /// The ship classes the shipyard reads: none, by default.
        ships: Vec<ShipRecord>,
        /// The ships whose sheets were asked for.
        sheets_asked: RefCell<Vec<ShipId>>,
    }

    type View = FlightView<FakeCatalog>;

    const FIELDS: ShipFields = ShipFields {
        speed: 300,
        accel: 300,
        maneuver: 30,
        shield: 40,
        armor: 60,
        fuel: 250,
        fuel_regen: 0,
        holds: 0,
        mass: 15,
        free_mass: 8,
        contribute: 1,
    };

    fn sheet() -> ShipSheet {
        ShipSheet {
            image_id: 2000,
            rotations: NonZeroU16::new(36).expect("non-zero"),
            frame_width: 40,
            frame_height: 40,
            glow: None,
            lights: None,
            blink: Blink::STEADY,
        }
    }

    fn catalog() -> FakeCatalog {
        FakeCatalog {
            character: Ok(CharacterStart {
                ship: Some(ShipId(128)),
                systems: [None, Some(SystemId(130)), None, None],
                start: StartDate {
                    day: 23,
                    month: 6,
                    year: 1177,
                },
                ..CharacterStart::default()
            }),
            fields: FIELDS,
            sheet: Ok(sheet()),
            bar: Ok(layout()),
            sites: vec![
                site(128, (0.0, -600.0), StellarFlags::CAN_LAND),
                site(129, (300.0, -200.0), StellarFlags::CAN_LAND),
            ],
            systems_read: RefCell::default(),
            commodities: CommodityStrings::default(),
            disasters: Vec::new(),
            outfits: Vec::new(),
            ships: Vec::new(),
            sheets_asked: RefCell::default(),
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
            landing_sound: None,
            tech_level: 1,
            special_tech: [0; 8],
            govt: None,
        }
    }

    /// The start's date, with stock's affixes.
    const DATE: &str = "June 23, 1177 NC";

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
            cargo: rect(8.0, 458.0, 184.0, 552.0),
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
            Ok(self.fields)
        }

        fn default_outfits(&self, id: ShipId) -> Vec<(OutfitId, u16)> {
            assert_eq!(id, ShipId(128));
            Vec::new()
        }

        fn outfits(&self) -> Vec<OutfitRecord> {
            self.outfits.clone()
        }

        fn ships(&self) -> Vec<ShipRecord> {
            self.ships.clone()
        }

        fn system_exists(&self, id: SystemId) -> bool {
            id == SystemId(130)
        }

        fn landing_sites(&self, system: SystemId) -> Vec<LandingSite> {
            match system.0 {
                130 => self.sites.clone(),
                131 => vec![site(140, (0.0, 0.0), StellarFlags::CAN_LAND)],
                other => panic!("asked for sÿst {other}'s sites"),
            }
        }

        fn star_map(&self) -> Vec<StarSystem> {
            let star = |id, (x, y), links: &[i16]| StarSystem {
                id: SystemId(id),
                position: Vec2::new(x, y),
                links: links.iter().copied().map(SystemId).collect(),
                govt: None,
            };
            vec![
                star(130, (0.0, 0.0), &[131]),
                star(131, (600.0, 0.0), &[]),
                star(132, (0.0, 600.0), &[]),
            ]
        }

        fn commodity_strings(&self) -> CommodityStrings {
            self.commodities.clone()
        }

        fn junk(&self) -> Vec<JunkRecord> {
            Vec::new()
        }

        fn disasters(&self) -> Vec<DisasterRecord> {
            self.disasters.clone()
        }

        /// Stock's: no prefix, and " NC".
        fn date_affixes(&self) -> DateAffixes {
            DateAffixes {
                prefix: String::new(),
                suffix: " NC".to_owned(),
            }
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
            self.systems_read.borrow_mut().push(id);
            let (name, stellars) = match id.0 {
                130 => (
                    "Sol",
                    vec![
                        stellar(128, "Earth", (0, -600), 1),
                        stellar(129, "Moon", (300, -200), 4),
                    ],
                ),
                131 => ("Alpha Centauri", vec![stellar(140, "Proxima", (0, 0), 1)]),
                other => panic!("asked for sÿst {other}"),
            };
            SystemContents {
                id,
                name: name.to_owned(),
                stellars,
                problems: Vec::new(),
            }
        }
    }

    impl GalaxyCatalog for FakeCatalog {
        fn galaxy(&self) -> Galaxy {
            let entry = |id, name: &str, (x, y), links: &[i16]| SystemEntry {
                id: SystemId(id),
                name: name.to_owned(),
                x,
                y,
                links: links.iter().copied().map(SystemId).collect(),
                govt: None,
                stellars: Vec::new(),
            };
            Galaxy {
                systems: vec![
                    entry(130, "Sol", (0, 0), &[131]),
                    entry(131, "Alpha Centauri", (600, 0), &[]),
                    entry(132, "Barnard", (0, 600), &[]),
                ],
                ..Galaxy::default()
            }
        }
    }

    impl ShipSprites for FakeCatalog {
        /// Ship 128's sheet is [`FakeCatalog::sheet`]; ship 129's is
        /// `rlëD` 2001's, with 72 rotations, glow `rlëD` 2101 and lights
        /// `rlëD` 2201.
        fn ship_sheet(&self, id: ShipId) -> Result<ShipSheet, String> {
            self.sheets_asked.borrow_mut().push(id);
            match id.0 {
                128 => self.sheet.clone(),
                129 => Ok(ShipSheet {
                    image_id: 2001,
                    rotations: NonZeroU16::new(72).expect("non-zero"),
                    glow: Some(layer(2101, 72)),
                    lights: Some(layer(2201, 72)),
                    ..sheet()
                }),
                other => panic!("asked for shïp {other}'s sheet"),
            }
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

    fn flight() -> View {
        FlightView::new(catalog())
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

    fn player(view: &View) -> ShipState {
        *view.session().expect("flying").player()
    }

    fn handling() -> Handling {
        ShipStats::new(FIELDS, &[]).handling
    }

    /// The ship as it starts: at rest at the centre, facing up.
    fn start() -> ShipState {
        ShipState::default()
    }

    fn reserves(view: &View) -> Reserves {
        view.session().expect("flying").reserves()
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

    fn ticks(view: &mut View, n: u32) {
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
        let mut view = FlightView::new(failed);
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

    fn drawn(view: &View) -> DrawList {
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
                reserves: ShipStats::new(FIELDS, &[]).full(),
                nav: NavDisplay::None,
                date: DATE,
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
            "Up: thrust   Left/Right: turn   Down: reverse   Tab: target   L: land   M: map   J: jump   P: preferences   Esc: leave flight"
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
        let view = FlightView::new(sheetless);
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

    // The engine glow and running lights.

    fn layer(image_id: i16, frames: u16) -> LayerSheet {
        LayerSheet {
            image_id,
            frames: NonZeroU16::new(frames).expect("non-zero"),
        }
    }

    /// [`sheet`] with a 36-frame glow, `rlëD` 2100, and 12-frame lights,
    /// `rlëD` 2200.
    fn layered() -> FakeCatalog {
        FakeCatalog {
            sheet: Ok(ShipSheet {
                glow: Some(layer(2100, 36)),
                lights: Some(layer(2200, 12)),
                ..sheet()
            }),
            ..catalog()
        }
    }

    /// The ship's sprites drawn: those from `rlëD`s 2000 to 2299.
    fn ship_sprites(view: &View) -> Vec<(ImageKey, Point)> {
        sprites(&drawn(view))
            .into_iter()
            .filter(|(image, _)| (2000..2300).contains(&image.id))
            .collect()
    }

    #[test]
    fn a_coasting_ship_draws_its_lights_over_it_and_no_glow() {
        let mut view = FlightView::new(layered());
        assert_eq!(
            ship_sprites(&view),
            [
                (ImageKey::sprite(2000, 0), VIEW_CENTER),
                (ImageKey::sprite(2200, 0), VIEW_CENTER),
            ]
        );
        view.input(&key(Key::Up, true));
        assert_eq!(ship_sprites(&view).len(), 2, "held, but not yet flown");
        ticks(&mut view, 3);
        view.input(&key(Key::Up, false));
        // As many ticks coasting as thrusting: the glow has faded out.
        ticks(&mut view, 3);
        let session = view.session().expect("flying");
        assert!(!session.thrusting());
        assert_eq!(session.engine_glow(), 0);
        assert_eq!(
            ship_sprites(&view)
                .iter()
                .map(|(image, _)| image.id)
                .collect::<Vec<_>>(),
            [2000, 2200],
            "coasting"
        );
    }

    #[test]
    fn a_thrusting_ship_draws_its_glow_between_it_and_its_lights() {
        let mut view = FlightView::new(layered());
        view.input(&key(Key::Up, true));
        // A base of 6: drawn at every roll.
        ticks(&mut view, 6);
        let centre = view.camera().world_to_screen(view.shown_position());
        assert_eq!(
            ship_sprites(&view),
            [
                (ImageKey::sprite(2000, 0), centre),
                (ImageKey::sprite(2100, 0), centre),
                (ImageKey::sprite(2200, 0), centre),
            ]
        );
    }

    /// The ship's sprites' image ids and blends, as [`ship_sprites`].
    fn ship_blends(view: &View) -> Vec<(i16, Blend)> {
        drawn(view)
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Sprite { image, blend, .. } if (2000..2300).contains(&image.id) => {
                    Some((image.id, *blend))
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_thrusting_ships_glow_and_lights_or_onto_its_normal_sprite() {
        let mut view = FlightView::new(layered());
        view.input(&key(Key::Up, true));
        ticks(&mut view, 6);
        assert_eq!(
            ship_blends(&view),
            [(2000, Blend::Normal), (2100, Blend::Or), (2200, Blend::Or),]
        );
        assert!(
            !drawn(&view).iter().any(|command| matches!(
                command,
                DrawCommand::Sprite {
                    blend: Blend::Additive,
                    ..
                }
            )),
            "nothing in a thrusting, lit frame is added"
        );
        view.input(&key(Key::Up, false));
        ticks(&mut view, 6);
        assert_eq!(view.session().expect("flying").engine_glow(), 0);
        assert_eq!(
            ship_blends(&view),
            [(2000, Blend::Normal), (2200, Blend::Or)],
            "coasting"
        );
    }

    #[test]
    fn the_layers_turn_with_the_ship_between_steps() {
        let mut view = FlightView::new(layered());
        view.input(&key(Key::Up, true));
        view.input(&key(Key::Right, true));
        // 50 steps and a half: the display is between two steps.
        ticks(&mut view, 50);
        view.tick(TICK / 2);
        assert_ne!(view.shown_heading(), player(&view).heading, "between");
        let frame = view.frame().expect("a sheet");
        assert_eq!(frame, 15, "{}", view.shown_heading());
        let centre = view.camera().world_to_screen(view.shown_position());
        assert_eq!(
            ship_sprites(&view),
            [
                (ImageKey::sprite(2000, frame), centre),
                (ImageKey::sprite(2100, frame), centre),
                (ImageKey::sprite(2200, frame % 12), centre),
            ]
        );
    }

    #[test]
    fn landing_while_thrusting_puts_the_glow_out() {
        let mut view = FlightView::new(FakeCatalog {
            sites: vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)],
            ..layered()
        });
        view.input(&key(Key::Up, true));
        ticks(&mut view, 6);
        assert_eq!(ship_sprites(&view).len(), 3, "glowing");
        land_now(&mut view);
        assert_eq!(view.take_landing(), Some(StellarId(128)));
        assert_eq!(view.session().expect("flying").engine_glow(), 0);
        let ids: Vec<_> = ship_sprites(&view).iter().map(|(i, _)| i.id).collect();
        assert_eq!(ids, [2000, 2200], "landed");
    }

    #[test]
    fn beginning_a_jump_while_thrusting_puts_the_glow_out() {
        let mut view = FlightView::new(layered());
        plot(&mut view, 131);
        fly_out(&mut view);
        view.input(&key(Key::Up, true));
        ticks(&mut view, 6);
        assert_eq!(ship_sprites(&view).len(), 3, "glowing");
        view.input(&key(JUMP_KEY, true));
        assert!(view.jump_effect().is_some(), "jumping");
        assert_eq!(view.session().expect("flying").engine_glow(), 0);
        let ids: Vec<_> = ship_sprites(&view).iter().map(|(i, _)| i.id).collect();
        assert_eq!(ids, [2000, 2200], "jumping");
    }

    #[test]
    fn a_ship_without_layers_draws_only_its_sprite() {
        let mut view = flight();
        view.input(&key(Key::Up, true));
        ticks(&mut view, 3);
        assert!(view.session().expect("flying").thrusting());
        let ids: Vec<_> = ship_sprites(&view).iter().map(|(i, _)| i.id).collect();
        assert_eq!(ids, [2000]);
    }

    #[test]
    fn a_ship_without_a_sheet_draws_no_layers() {
        let mut view = FlightView::new(FakeCatalog {
            sheet: Err("no shän 128 for shïp 128".to_owned()),
            ..layered()
        });
        view.input(&key(Key::Up, true));
        ticks(&mut view, 3);
        assert_eq!(ship_sprites(&view), []);
    }

    /// Every command drawing the glow, `rlëD` 2100, whatever its blend.
    fn glow_commands(view: &View) -> Vec<DrawCommand> {
        drawn(view)
            .iter()
            .filter(
                |command| matches!(command, DrawCommand::Sprite { image, .. } if image.id == 2100),
            )
            .cloned()
            .collect()
    }

    /// The glow commands for a base of `base` at `tick`: the glow drawn by
    /// OR at the level [`glow_level`] gives over the ship's centre, on its
    /// frame, or none when it is hidden.
    fn glow_for(view: &View, base: u8, tick: u64) -> Vec<DrawCommand> {
        let frame = view.frame().expect("a sheet") % 36;
        let center = view.camera().world_to_screen(view.shown_position());
        glow_level(base, tick, &view.glow_rolls)
            .map(|level| DrawCommand::Sprite {
                image: ImageKey::sprite(2100, frame),
                center,
                tint: lights_tint(level),
                blend: Blend::Or,
            })
            .into_iter()
            .collect()
    }

    #[test]
    fn the_glow_ramps_in_and_flickers_at_the_glow_function_level() {
        let mut view = FlightView::new(layered());
        view.input(&key(Key::Up, true));
        let mut cruising = Vec::new();
        for n in 1..=40_u8 {
            ticks(&mut view, 1);
            let base = n.min(GLOW_CRUISE);
            assert_eq!(view.session().expect("flying").engine_glow(), base);
            let want = glow_for(&view, base, u64::from(n));
            assert_eq!(glow_commands(&view), want, "tick {n}");
            if n >= 6 {
                assert_eq!(want.len(), 1, "a base of 6 or more always shows");
            }
            if n >= 24 {
                cruising.extend(want);
            }
        }
        cruising.dedup();
        assert!(cruising.len() >= 2, "it flickers: {cruising:?}");
    }

    #[test]
    fn releasing_up_fades_the_glow_out() {
        let mut view = FlightView::new(layered());
        view.input(&key(Key::Up, true));
        ticks(&mut view, 30);
        assert_eq!(view.session().expect("flying").engine_glow(), GLOW_CRUISE);
        view.input(&key(Key::Up, false));
        for k in 1..=40_u8 {
            ticks(&mut view, 1);
            let base = GLOW_CRUISE.saturating_sub(k);
            assert_eq!(view.session().expect("flying").engine_glow(), base);
            let want = glow_for(&view, base, 30 + u64::from(k));
            assert_eq!(glow_commands(&view), want, "{k} ticks after");
            if k == 3 {
                let level = glow_level(21, 33, &view.glow_rolls).expect("still drawn");
                let alpha = lights_tint(level).a;
                assert!(alpha < 255 && alpha >= lights_tint(17).a, "{alpha}");
                assert_eq!(want.len(), 1, "fading, not out");
            }
            if k >= 24 {
                assert_eq!(want, [], "out {k} ticks after");
            }
        }
    }

    // The running lights' blinking.

    /// The Shuttle's blink: a double flash every 40 ticks, lit at ticks
    /// 1-3 and 10-12.
    const SHUTTLE: Blink = Blink {
        mode: 1,
        a: 4,
        b: 1,
        c: 2,
        d: 20,
    };

    /// [`layered`], with the lights blinking by `blink`.
    fn blinking(blink: Blink) -> FakeCatalog {
        FakeCatalog {
            sheet: Ok(ShipSheet {
                glow: Some(layer(2100, 36)),
                lights: Some(layer(2200, 12)),
                blink,
                ..sheet()
            }),
            ..catalog()
        }
    }

    /// The tints the lights, `rlëD` 2200, are drawn by OR with.
    fn lights_tints(view: &View) -> Vec<Color> {
        drawn(view)
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Sprite {
                    image,
                    tint,
                    blend: Blend::Or,
                    ..
                } if image.id == 2200 => Some(*tint),
                _ => None,
            })
            .collect()
    }

    /// The lights' alphas drawn: one, or none when they are off.
    fn lights_alphas(view: &View) -> Vec<u8> {
        lights_tints(view).iter().map(|tint| tint.a).collect()
    }

    #[test]
    fn the_shuttles_lights_blink_with_game_time() {
        let mut view = FlightView::new(blinking(SHUTTLE));
        assert_eq!(lights_alphas(&view), [0_u8; 0], "tick 0: off");
        assert_eq!(ship_sprites(&view)[0].0.id, 2000, "the ship is drawn");
        ticks(&mut view, 1);
        assert_eq!(lights_tints(&view), [Color::WHITE], "tick 1: on");
        ticks(&mut view, 3);
        assert_eq!(lights_alphas(&view), [0_u8; 0], "tick 4: off");
        ticks(&mut view, 6);
        assert_eq!(lights_alphas(&view), [255], "tick 10: on again");
        ticks(&mut view, 3);
        assert_eq!(lights_alphas(&view), [0_u8; 0], "tick 13: the gap");
    }

    #[test]
    fn a_pulsing_light_is_ored_at_its_level() {
        // The stock triangle: 10 to 31.
        let triangle = Blink {
            mode: 2,
            a: 10,
            b: 75,
            c: 32,
            d: 75,
        };
        let mut view = FlightView::new(blinking(triangle));
        assert_eq!(lights_alphas(&view), [0_u8; 0], "0.75: not drawn");
        ticks(&mut view, 1);
        assert_eq!(lights_alphas(&view), [8], "level 1");
        ticks(&mut view, 40);
        assert_eq!(lights_alphas(&view), [247], "level 31");
    }

    /// Every command drawing the lights, `rlëD` 2200, whatever its blend.
    fn lights_commands(view: &View) -> Vec<DrawCommand> {
        drawn(view)
            .iter()
            .filter(
                |command| matches!(command, DrawCommand::Sprite { image, .. } if image.id == 2200),
            )
            .cloned()
            .collect()
    }

    /// The lights drawn by OR with `tint` over the coasting ship's centre.
    fn lights_at(tint: Color) -> DrawCommand {
        DrawCommand::Sprite {
            image: ImageKey::sprite(2200, 0),
            center: VIEW_CENTER,
            tint,
            blend: Blend::Or,
        }
    }

    #[test]
    fn lights_are_ored_scaled_by_level_below_full_and_whole_at_full() {
        // The stock triangle, 10 to 31, never full: the original draws
        // every level with its translucent blit, which ORs in the lights
        // scaled by level/32 (see `lights_tint`).
        let triangle = Blink {
            mode: 2,
            a: 10,
            b: 75,
            c: 32,
            d: 75,
        };
        let rolls = HashedRolls::new(0);
        let mut view = FlightView::new(blinking(triangle));
        let mut levels = Vec::new();
        for tick in 0..=101 {
            let level = lights_level(&triangle, tick, &rolls);
            let want: Vec<_> = level
                .map(|n| lights_at(lights_tint(n)))
                .into_iter()
                .collect();
            assert_eq!(lights_commands(&view), want, "tick {tick}");
            levels.extend(level);
            ticks(&mut view, 1);
        }
        assert!(levels.contains(&10), "the lowest level is drawn");
        assert!(levels.contains(&31), "the highest level is drawn");

        // A steady light is full: ORed whole.
        let view = FlightView::new(blinking(Blink::STEADY));
        assert_eq!(lights_commands(&view), [lights_at(Color::WHITE)]);
    }

    #[test]
    fn steady_lights_are_ored_at_full_from_the_start() {
        let view = FlightView::new(blinking(Blink::STEADY));
        assert_eq!(lights_tints(&view), [Color::WHITE]);
    }

    #[test]
    fn the_glow_still_shows_while_the_lights_are_off() {
        let mut view = FlightView::new(blinking(SHUTTLE));
        view.input(&key(Key::Up, true));
        // Tick 13: the Shuttle's lights are dark, and a base of 13 always
        // shows the glow.
        ticks(&mut view, 13);
        assert!(view.session().expect("flying").thrusting());
        assert_eq!(
            ship_blends(&view),
            [(2000, Blend::Normal), (2100, Blend::Or)]
        );
        let glow = drawn(&view).iter().find_map(|command| match command {
            DrawCommand::Sprite { image, tint, .. } if image.id == 2100 => Some(*tint),
            _ => None,
        });
        let level = glow_level(13, 13, &view.glow_rolls).expect("drawn");
        assert_eq!(glow, Some(lights_tint(level)));
    }

    #[test]
    fn the_blink_pauses_while_the_map_is_open() {
        let mut view = FlightView::new(blinking(SHUTTLE));
        ticks(&mut view, 3);
        assert_eq!(lights_alphas(&view), [255], "tick 3");
        view.input(&key(MAP_KEY, true));
        assert!(view.map_open());
        // Tick 103 would be dark.
        ticks(&mut view, 100);
        view.close_map();
        assert_eq!(lights_alphas(&view), [255], "still tick 3");
        ticks(&mut view, 1);
        assert_eq!(lights_alphas(&view), [0_u8; 0], "tick 4");
    }

    #[test]
    fn random_lights_roll_on_hashed_rolls() {
        // A=5 B=12 C=3: one of 8 levels from 5, every 4 ticks.
        let random = Blink {
            mode: 3,
            a: 5,
            b: 12,
            c: 3,
            d: 0,
        };
        let rolls = HashedRolls::new(0);
        let expected = |change| lights_tint(5 + rolls.roll(change, 8) as u8).a;
        let mut view = FlightView::new(blinking(random));
        assert_eq!(lights_alphas(&view), [expected(0)]);
        ticks(&mut view, 4);
        assert_eq!(lights_alphas(&view), [expected(1)]);
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
        // No system name in the nav area (the dev title still names it).
        assert!(!texts(&list).contains(&"Sol".to_owned()), "{list:?}");
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
        let reserves = reserves(&view);
        assert_eq!(reserves, ShipStats::new(FIELDS, &[]).full());
        let mut expected = DrawList::new();
        hud::draw(
            &mut expected,
            view.status_bar().expect("a status bar"),
            &HudState {
                position: at(0.0, 0.0),
                stellars: &[],
                reserves,
                nav: NavDisplay::None,
                date: DATE,
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
        let mut view = FlightView::new(barless);
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
        let view = FlightView::new(failed);
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
    fn flight_among(sites: Vec<LandingSite>) -> View {
        FlightView::new(FakeCatalog { sites, ..catalog() })
    }

    const LAND: Key = Key::Char('l');

    /// The message drawn at its place, if any.
    fn message(view: &View) -> Option<DrawCommand> {
        drawn(view)
            .iter()
            .find(|c| matches!(c, DrawCommand::Text { origin, .. } if *origin == MESSAGE_AT))
            .cloned()
    }

    /// L pressed twice: clearance, then landing.
    fn land_now(view: &mut View) {
        tap(view, LAND);
        tap(view, LAND);
    }

    #[test]
    fn l_requests_clearance_and_a_second_l_lands() {
        // The Moon (129), in the scene, over the ship.
        let mut view = flight_among(vec![site(129, (6.0, -8.0), StellarFlags::CAN_LAND)]);
        assert_eq!(view.take_landing(), None);
        assert_eq!(view.input(&key(LAND, true)), ScreenAction::None);
        assert_eq!(view.take_landing(), None, "cleared, not landed");
        assert_eq!(view.session().expect("flying").landed(), None);
        assert_eq!(nav_target(&view), Some(StellarId(129)));
        let cleared = "Moon traffic control reads you, you're cleared to land.";
        assert_eq!(view.message(), Some(cleared));
        assert_eq!(
            message(&view),
            Some(overlay(cleared, MESSAGE_AT, OVERLAY_SIZE, Color::WHITE))
        );
        assert_eq!(nav(&view), [hud::NAV_STELLAR, "Moon"], "the HUD shows it");
        assert_eq!(view.take_sounds(), [], "no sound for clearance");

        view.input(&key(LAND, false));
        assert_eq!(view.input(&key(LAND, true)), ScreenAction::None);
        assert_eq!(
            view.session().expect("flying").landed(),
            Some(StellarId(129))
        );
        assert_eq!(view.take_landing(), Some(StellarId(129)));
        assert_eq!(view.take_landing(), None, "taken");
        assert_eq!(view.message(), None);
        assert_eq!(message(&view), None);
        assert_eq!(LAND_KEY, LAND);
    }

    #[test]
    fn l_lands_at_once_on_a_stellar_selected_by_tab() {
        let mut view = flight_among(vec![site(129, (6.0, -8.0), StellarFlags::CAN_LAND)]);
        tap(&mut view, TARGET_KEY);
        tap(&mut view, LAND);
        assert_eq!(view.take_landing(), Some(StellarId(129)));
    }

    #[test]
    fn every_clearance_has_its_message() {
        use Clearance::{Denied, Granted, NoTrafficControl};
        let cases = [
            (
                false,
                Granted,
                "Earth traffic control reads you, you're cleared to land.",
            ),
            (
                true,
                Granted,
                "Earth dockmaster reads you, you're cleared to dock.",
            ),
            (false, NoTrafficControl, "You are cleared to land."),
            (true, NoTrafficControl, "You are cleared to dock."),
            (false, Denied, "Landing request denied."),
            (true, Denied, "Docking request denied."),
        ];
        for (station, clearance, text) in cases {
            assert_eq!(
                clearance_message("Earth", station, clearance),
                text,
                "{station} {clearance:?}"
            );
        }
        assert_eq!(
            [
                TRAFFIC_CONTROL_READS_YOU,
                DOCKMASTER_READS_YOU,
                CLEARED_TO_LAND,
                CLEARED_TO_DOCK,
                YOU_ARE_CLEARED_TO_LAND,
                YOU_ARE_CLEARED_TO_DOCK,
            ],
            [
                "traffic control reads you",
                "dockmaster reads you",
                "you're cleared to land.",
                "you're cleared to dock.",
                "You are cleared to land.",
                "You are cleared to dock.",
            ]
        );
    }

    #[test]
    fn the_clearance_names_the_stellar_selected_and_says_how_it_answered() {
        let uninhabited = StellarFlags::CAN_LAND | StellarFlags::UNINHABITED;
        let mut view = flight_among(vec![site(128, (0.0, 0.0), uninhabited)]);
        tap(&mut view, LAND);
        assert_eq!(view.message(), Some("You are cleared to land."));
        let strict = LandingSite {
            min_status: 1,
            ..site(
                129,
                (0.0, 0.0),
                StellarFlags::CAN_LAND | StellarFlags::STATION,
            )
        };
        let mut view = flight_among(vec![strict]);
        tap(&mut view, LAND);
        assert_eq!(view.message(), Some("Docking request denied."));
        assert_eq!(nav_target(&view), Some(StellarId(129)), "selected anyway");
        let mut view = flight();
        tap(&mut view, LAND);
        assert_eq!(
            view.message(),
            Some("Moon traffic control reads you, you're cleared to land."),
            "the nearer of Sol's two"
        );
    }

    #[test]
    fn a_repeat_or_release_of_l_does_nothing() {
        let mut view = flight_among(vec![site(140, (0.0, 0.0), StellarFlags::CAN_LAND)]);
        view.input(&held(LAND));
        view.input(&key(LAND, false));
        assert_eq!(view.take_landing(), None);
        assert_eq!(view.session().expect("flying").landed(), None);
        assert_eq!(nav_target(&view), None, "nothing selected");
        let mut far = flight();
        far.input(&held(LAND));
        assert_eq!(far.message(), None, "no refusal either");
    }

    #[test]
    fn each_refusal_says_why_in_the_originals_words() {
        // L, then L again: the first requests clearance where it can.
        let refused = |sites: Vec<LandingSite>| {
            let mut view = flight_among(sites);
            tap(&mut view, LAND);
            tap(&mut view, LAND);
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
            land_now(&mut view);
            assert_eq!(view.take_landing(), None);
            assert_eq!(view.message(), Some(expected));
            assert_eq!(nav_target(&view), Some(StellarId(140)), "kept: try again");
        }
    }

    #[test]
    fn every_refusal_has_its_string() {
        let stellar = StellarId(128);
        let cases = [
            (LandingRefusal::Jumping, IN_HYPERSPACE),
            (LandingRefusal::NoStellars, NO_STELLARS),
            (
                LandingRefusal::TooFar {
                    stellar,
                    station: true,
                },
                TOO_FAR_STATION,
            ),
            (
                LandingRefusal::TooFar {
                    stellar,
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
                IN_HYPERSPACE,
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
                "Unable to land - your ship is in hyperspace.",
            ]
        );
    }

    #[test]
    fn the_refusal_is_drawn_above_the_help_line_until_it_has_been_shown_long_enough() {
        let mut view = flight();
        ticks(&mut view, 3);
        land_now(&mut view);
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
        land_now(&mut view);
        assert_eq!(view.message(), Some(TOO_FAR_PLANET));
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
        land_now(&mut view);
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
        land_now(&mut view);
        assert_eq!(view.message(), Some(TOO_FAR_PLANET));
        // Nudge the ship and drift over the planet.
        view.input(&key(Key::Up, true));
        ticks(&mut view, 1);
        view.input(&key(Key::Up, false));
        for _ in 0..100 {
            if player(&view).position.y <= -4.0 {
                break;
            }
            ticks(&mut view, 1);
        }
        assert!(player(&view).position.y <= -4.0, "{:?}", player(&view));
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
        let mut view = FlightView::new(FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..catalog()
        });
        view.input(&key(LAND, true));
        assert_eq!(view.take_landing(), None);
        assert_eq!(view.message(), None);
        assert_eq!(view.take_off(), None);
    }

    #[test]
    fn l_over_a_stellar_sounds_the_landing_with_its_own_sound() {
        let mut view = flight_among(vec![LandingSite {
            landing_sound: Some(SoundId(10_032)),
            ..site(140, (6.0, -8.0), StellarFlags::CAN_LAND)
        }]);
        assert_eq!(view.take_sounds(), []);
        tap(&mut view, LAND);
        assert_eq!(view.take_sounds(), [], "clearance is silent");
        tap(&mut view, LAND);
        assert_eq!(
            view.take_sounds(),
            [Sound::Sim(SimSound::Landed {
                stellar_sound: Some(SoundId(10_032))
            })]
        );
        assert_eq!(view.take_sounds(), [], "taken");
        view.take_off();
        assert_eq!(view.take_sounds(), [Sound::Sim(SimSound::TookOff)]);
    }

    #[test]
    fn thrust_sounds_as_it_starts_and_stops() {
        let mut view = flight();
        view.input(&key(Key::Up, true));
        ticks(&mut view, 3);
        assert_eq!(view.take_sounds(), [Sound::Sim(SimSound::ThrustStarted)]);
        view.input(&key(Key::Up, false));
        ticks(&mut view, 1);
        assert_eq!(view.take_sounds(), [Sound::Sim(SimSound::ThrustStopped)]);
    }

    #[test]
    fn a_flight_that_never_started_makes_no_sounds() {
        let mut view = FlightView::new(FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..catalog()
        });
        view.input(&key(Key::Up, true));
        ticks(&mut view, 3);
        assert_eq!(view.take_sounds(), []);
    }

    // Hyperspace.

    const MAP: Key = Key::Char('m');
    const JUMP: Key = Key::Char('j');

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    /// Presses and releases `k`.
    fn tap(view: &mut View, k: Key) {
        view.input(&key(k, true));
        view.input(&key(k, false));
    }

    /// Where system `id` is on the course map.
    fn on_map(view: &View, id: i16) -> Point {
        let map = view.course_map();
        let system = map.model().system(SystemId(id)).expect("on the map");
        map.view().world_to_screen(system.position())
    }

    /// Clicks the left button at `at`.
    fn click(view: &mut View, at: Point) {
        for pressed in [true, false] {
            view.input(&Input::PointerButton {
                button: crate::MouseButton::Left,
                pressed,
                at,
            });
        }
    }

    /// Opens the map, picks system `id` and closes the map again.
    fn plot(view: &mut View, id: i16) {
        tap(view, MAP);
        let at = on_map(view, id);
        click(view, at);
        tap(view, MAP);
        assert!(!view.map_open());
    }

    /// Thrusts straight up until the ship is at least the minimum jump
    /// distance from the centre, then lets go of Up.
    fn fly_out(view: &mut View) {
        view.input(&key(Key::Up, true));
        for _ in 0..2000 {
            if player(view).position.length() >= MIN_JUMP_DISTANCE {
                break;
            }
            view.tick(TICK);
        }
        view.input(&key(Key::Up, false));
        assert!(
            player(view).position.length() >= MIN_JUMP_DISTANCE,
            "{:?}",
            player(view)
        );
    }

    #[test]
    fn the_course_map_starts_on_the_current_system_with_no_course() {
        let view = flight();
        assert!(!view.map_open());
        assert_eq!(view.course_map().mode(), MapMode::Course);
        assert_eq!(view.course_map().current(), Some(SystemId(130)));
        assert_eq!(view.course_map().route(), []);
        assert_eq!(view.jump_effect(), None);
        assert_eq!((MAP_KEY, JUMP_KEY), (MAP, JUMP));
    }

    #[test]
    fn m_opens_the_map_over_everything_and_lets_go_of_the_flight_keys() {
        let mut view = flight();
        view.input(&key(Key::Up, true));
        assert_eq!(view.input(&key(MAP, true)), ScreenAction::None);
        assert!(view.map_open());
        let mut map = DrawList::new();
        view.course_map().draw(&mut map);
        assert_eq!(drawn(&view), map, "only the map");
        // Holding M, or letting it go, keeps it open.
        view.input(&held(MAP));
        view.input(&key(MAP, false));
        assert!(view.map_open());
        view.input(&key(MAP, true));
        assert!(!view.map_open());
        ticks(&mut view, 5);
        assert_eq!(player(&view), start(), "Up was let go");
    }

    #[test]
    fn while_the_map_is_open_flight_is_paused_and_keys_go_to_the_map() {
        let mut view = flight();
        view.input(&key(Key::Up, true));
        ticks(&mut view, 3);
        let flying = player(&view);
        tap(&mut view, MAP);
        let fitted = *view.course_map().view();
        ticks(&mut view, 30);
        assert_eq!(player(&view), flying, "paused");
        view.input(&key(Key::Left, true));
        assert_ne!(*view.course_map().view(), fitted, "Left pans the map");
        view.input(&key(LAND, true));
        view.input(&key(JUMP, true));
        assert_eq!(view.message(), None);
        assert_eq!(view.take_landing(), None);
        assert_eq!(view.jump_effect(), None);
        tap(&mut view, MAP);
        ticks(&mut view, 1);
        assert_eq!(player(&view).heading, 0.0, "Left went to the map");
    }

    #[test]
    fn a_click_on_a_system_plots_the_course_there_and_the_map_shows_it() {
        let mut view = flight();
        tap(&mut view, MAP);
        let alpha = on_map(&view, 131);
        click(&mut view, alpha);
        assert!(view.map_open());
        let course = [SystemId(131)];
        assert_eq!(view.session().expect("flying").course(), course);
        assert_eq!(view.course_map().route(), course);
        assert_eq!(view.course_map().current(), Some(SystemId(130)));
        let barnard = on_map(&view, 132);
        click(&mut view, barnard);
        assert_eq!(view.session().expect("flying").course(), []);
        assert_eq!(view.course_map().route(), []);
        assert!(
            texts(&drawn(&view)).contains(&"No hyperspace route".to_owned()),
            "{:?}",
            texts(&drawn(&view))
        );
    }

    #[test]
    fn closing_the_map_abandons_its_drag() {
        let mut view = flight();
        tap(&mut view, MAP);
        let alpha = on_map(&view, 131);
        view.input(&Input::PointerButton {
            button: crate::MouseButton::Left,
            pressed: true,
            at: alpha,
        });
        view.close_map();
        assert!(!view.map_open());
        tap(&mut view, MAP);
        view.input(&Input::PointerButton {
            button: crate::MouseButton::Left,
            pressed: false,
            at: alpha,
        });
        assert_eq!(view.course_map().selected(), None, "no click");
        assert_eq!(view.session().expect("flying").course(), []);
    }

    #[test]
    fn j_says_why_a_jump_is_refused_in_the_originals_words() {
        let mut view = flight();
        view.input(&key(JUMP, true));
        assert_eq!(view.message(), Some(NO_DESTINATION));
        assert_eq!(
            message(&view),
            Some(overlay(
                NO_DESTINATION,
                MESSAGE_AT,
                OVERLAY_SIZE,
                Color::WHITE
            ))
        );
        plot(&mut view, 131);
        view.input(&key(JUMP, true));
        assert_eq!(view.message(), Some(TOO_CLOSE));
        assert_eq!(view.jump_effect(), None);

        let mut dry = FlightView::new(FakeCatalog {
            fields: ShipFields { fuel: 50, ..FIELDS },
            ..catalog()
        });
        plot(&mut dry, 131);
        fly_out(&mut dry);
        dry.input(&key(JUMP, true));
        assert_eq!(dry.message(), Some(NO_FUEL));
        assert_eq!(dry.jump_effect(), None);
    }

    #[test]
    fn a_repeat_or_release_of_j_does_nothing() {
        let mut view = flight();
        plot(&mut view, 131);
        fly_out(&mut view);
        view.input(&held(JUMP));
        view.input(&key(JUMP, false));
        assert_eq!(view.jump_effect(), None);
        assert_eq!(view.message(), None);
    }

    #[test]
    fn every_jump_refusal_has_its_string() {
        assert_eq!(
            jump_refusal_message(&JumpRefusal::NoDestination),
            NO_DESTINATION
        );
        assert_eq!(
            jump_refusal_message(&JumpRefusal::TooClose { distance: 1.0 }),
            TOO_CLOSE
        );
        assert_eq!(
            jump_refusal_message(&JumpRefusal::NoFuel { fuel: 1.0 }),
            NO_FUEL
        );
        assert_eq!(jump_refusal_message(&JumpRefusal::Landed), TAKE_OFF_FIRST);
        assert_eq!(
            [NO_DESTINATION, TOO_CLOSE, NO_FUEL, TAKE_OFF_FIRST],
            [
                "You have to select a destination before you can start a hyperspace jump.",
                "Can't initiate hyperspace jump - not yet far enough away from system center.",
                "Insufficient energy for hyperspace jump.",
                "Can't initiate hyperspace jump - take off first.",
            ]
        );
    }

    #[test]
    fn j_while_landed_is_refused_until_take_off() {
        let mut view = flight_among(vec![site(140, (0.0, 0.0), StellarFlags::CAN_LAND)]);
        plot(&mut view, 131);
        land_now(&mut view);
        assert_eq!(view.take_landing(), Some(StellarId(140)));
        view.input(&key(JUMP, true));
        assert_eq!(view.jump_effect(), None);
        assert_eq!(view.session().expect("flying").jumping(), None);
        assert_eq!(view.message(), Some(TAKE_OFF_FIRST));
        view.take_off();
        view.input(&key(JUMP, true));
        assert_eq!(view.message(), Some(TOO_CLOSE), "flying again");
    }

    /// The fade quad's place in the list and its colour, if drawn.
    fn fade(list: &DrawList) -> Option<(usize, Color)> {
        list.iter().enumerate().find_map(|(at, c)| match *c {
            DrawCommand::Line {
                from, width, color, ..
            } if from == Point::new(0.0, 384.0) && width == 768.0 => Some((at, color)),
            _ => None,
        })
    }

    /// Where the HUD starts: its status bar picture.
    fn hud_at(list: &DrawList) -> usize {
        list.iter()
            .position(|c| matches!(c, DrawCommand::Picture { image, .. } if *image == ImageKey::picture(700)))
            .expect("the HUD")
    }

    /// Where the ship's sprite is drawn.
    fn ship_at(list: &DrawList) -> usize {
        list.iter()
            .position(|c| matches!(c, DrawCommand::Sprite { image, .. } if image.id == 2000))
            .expect("the ship")
    }

    #[test]
    fn a_jump_streaks_the_stars_then_fades_out_arrives_and_fades_in() {
        let mut view = flight();
        plot(&mut view, 131);
        fly_out(&mut view);
        let leaving = player(&view);
        let fuel_leaving = reserves(&view).fuel.now;
        assert_eq!(view.input(&key(JUMP, true)), ScreenAction::None);
        assert_eq!(view.message(), None);
        let effect = view.jump_effect().expect("jumping");
        assert_eq!(effect.direction(), at(1.0, 0.0), "east, to Alpha Centauri");
        assert_eq!(
            view.session().expect("flying").jumping(),
            Some(SystemId(131))
        );

        // The stars streak; nothing fades.
        view.tick(ms(500));
        assert_eq!(player(&view), leaving, "frozen");
        let list = drawn(&view);
        let mut streaks = DrawList::new();
        starfield::draw_streaked(&mut streaks, &view.camera(), at(1.0, 0.0), 256.0);
        assert!(!streaks.is_empty());
        assert!(list.iter().take(streaks.len()).eq(streaks.iter()));
        assert_eq!(fade(&list), None);
        assert!(texts(&list).contains(&"Sol (sÿst 130)".to_owned()));

        // The old system fades out, under the HUD.
        view.tick(ms(625));
        let list = drawn(&view);
        let (at_fade, color) = fade(&list).expect("a fade");
        assert_eq!(color, Color::rgba(0, 0, 0, 64));
        assert!(ship_at(&list) < at_fade && at_fade < hud_at(&list));
        assert!(texts(&list).contains(&"Sol (sÿst 130)".to_owned()));
        view.tick(ms(125));
        assert_eq!(fade(&drawn(&view)).map(|f| f.1.a), Some(128), "growing");
        assert_eq!(view.session().expect("flying").system(), SystemId(130));

        // Past 1.5 s it arrives, and the new system fades in.
        view.tick(ms(300));
        let session = view.session().expect("flying");
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(session.jumping(), None);
        assert_eq!(
            *view.catalog().systems_read.borrow(),
            [SystemId(130), SystemId(131)]
        );
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
        assert_eq!(view.course_map().current(), Some(SystemId(131)));
        assert_eq!(view.course_map().route(), []);
        let arrived = player(&view);
        assert_eq!(arrived.position, Vec2::new(-1000.0, 0.0));
        assert_eq!(reserves(&view).fuel.now, fuel_leaving - 100.0);
        assert_eq!(view.shown_position(), at(-1000.0, 0.0));
        let list = drawn(&view);
        let (at_fade, color) = fade(&list).expect("a fade");
        assert_eq!(color.a, 230);
        assert!(texts(&list).contains(&"Alpha Centauri (sÿst 131)".to_owned()));
        assert!(at_fade < hud_at(&list));
        let mut hud = DrawList::new();
        hud::draw(
            &mut hud,
            view.status_bar().expect("a status bar"),
            &HudState {
                position: at(-1000.0, 0.0),
                stellars: &[at(0.0, 0.0)],
                reserves: reserves(&view),
                nav: NavDisplay::None,
                date: "June 24, 1177 NC",
            },
        );
        assert!(
            list.iter().skip(hud_at(&list)).eq(hud.iter()),
            "the HUD, with a jump's fuel less and a day on, is last"
        );
        view.tick(ms(100));
        assert_eq!(fade(&drawn(&view)).map(|f| f.1.a), Some(179), "shrinking");
        assert_eq!(player(&view), arrived, "still frozen");

        // Then plain flight.
        view.tick(ms(500));
        assert_eq!(view.jump_effect(), None);
        let list = drawn(&view);
        assert_eq!(fade(&list), None);
        let mut stars = DrawList::new();
        starfield::draw(&mut stars, &view.camera());
        assert!(list.iter().take(stars.len()).eq(stars.iter()));
        ticks(&mut view, 3);
        assert_eq!(player(&view), stepped(arrived, Controls::default(), 3));
        assert_ne!(player(&view), arrived, "it flies again");
    }

    #[test]
    fn keys_are_ignored_while_the_jump_plays() {
        let mut view = flight();
        plot(&mut view, 131);
        fly_out(&mut view);
        view.input(&key(Key::Up, true));
        view.input(&key(JUMP, true));
        view.tick(ms(300));
        view.input(&key(Key::Up, false));
        view.input(&key(Key::Left, true));
        view.input(&key(MAP, true));
        view.input(&key(LAND, true));
        view.input(&key(JUMP, true));
        assert!(!view.map_open());
        assert_eq!(view.message(), None);
        view.tick(ms(2000));
        assert_eq!(view.jump_effect(), None);
        assert_eq!(view.take_landing(), None);
        let arrived = player(&view);
        ticks(&mut view, 5);
        assert_eq!(
            player(&view),
            stepped(arrived, Controls::default(), 5),
            "neither thrusting nor turning"
        );
    }

    #[test]
    fn a_refusal_shown_before_the_jump_is_gone_after_it() {
        let mut view = flight();
        fly_out(&mut view);
        view.input(&key(JUMP, true));
        view.input(&key(JUMP, false));
        assert_eq!(view.message(), Some(NO_DESTINATION));
        plot(&mut view, 131);
        assert_eq!(view.message(), Some(NO_DESTINATION), "still on screen");
        view.input(&key(JUMP, true));
        assert_eq!(view.message(), None);
    }

    #[test]
    fn map_and_jump_in_a_flight_that_never_started_do_nothing_much() {
        let mut view = FlightView::new(FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..catalog()
        });
        assert_eq!(view.course_map().current(), None);
        view.input(&key(JUMP, true));
        assert_eq!(view.jump_effect(), None);
        assert_eq!(view.message(), None);
        tap(&mut view, MAP);
        assert!(view.map_open());
        let alpha = on_map(&view, 131);
        click(&mut view, alpha);
        assert_eq!(view.course_map().route(), []);
    }

    #[test]
    fn the_stellars_go_on_animating_while_the_jump_plays() {
        let mut view = flight();
        plot(&mut view, 131);
        fly_out(&mut view);
        view.input(&key(JUMP, true));
        let moon = |view: &View| sprites(&drawn(view))[1].0.frame;
        let before = moon(&view);
        view.tick(TICK);
        assert_eq!(moon(&view), (before + 1) % 4, "a frame a tick");
        view.tick(TICK * 2);
        assert_eq!(moon(&view), (before + 3) % 4);
        assert!(view.jump_effect().is_some());
    }

    #[test]
    fn cancelling_the_pointer_abandons_a_click_on_the_open_map() {
        let mut view = flight();
        tap(&mut view, MAP);
        let alpha = on_map(&view, 131);
        let left = |pressed| Input::PointerButton {
            button: crate::MouseButton::Left,
            pressed,
            at: alpha,
        };
        view.input(&left(true));
        view.cancel_pointer();
        view.input(&left(false));
        assert_eq!(view.course_map().selected(), None, "no click");
        assert_eq!(view.session().expect("flying").course(), []);
        assert!(view.map_open());
    }

    // The pilot.

    /// A pilot named `name` docked at Earth: landed over a planet 128 at
    /// the centre, then flown in the stock catalog, where Earth is at
    /// (0, -600).
    fn docked_pilot(name: &str) -> Pilot {
        let centred = FakeCatalog {
            sites: vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)],
            ..catalog()
        };
        let pilot = Pilot::new(&centred, name).expect("starts");
        let mut view = FlightView::with_pilot(centred, pilot);
        land_now(&mut view);
        assert_eq!(view.take_landing(), Some(StellarId(128)));
        view.pilot().expect("flying").clone()
    }

    #[test]
    fn a_pilot_docked_at_a_stellar_resumes_landed_there_silently() {
        let pilot = docked_pilot("Ada");
        assert_eq!(pilot.stellar(), Some(StellarId(128)));
        let mut view = FlightView::with_pilot(catalog(), pilot.clone());
        assert_eq!(view.pilot(), Some(&pilot));
        assert_eq!(
            view.take_landing(),
            Some(StellarId(128)),
            "the router lands"
        );
        assert_eq!(view.take_landing(), None, "once");
        assert_eq!(player(&view).position, Vec2::new(0.0, -600.0));
        assert_eq!(view.take_sounds(), [], "no landing sound");
        assert!(!view.take_save_due());
        assert_eq!(view.take_off(), Some(StellarId(128)));
    }

    #[test]
    fn a_pilot_in_flight_resumes_at_the_centre() {
        let pilot = Pilot::new(&catalog(), "Bob").expect("starts");
        let mut view = FlightView::with_pilot(catalog(), pilot);
        assert_eq!(view.take_landing(), None);
        assert_eq!(player(&view), start());
        assert_eq!(view.pilot().map(Pilot::name), Some("Bob"));
    }

    #[test]
    fn a_pilot_whose_system_is_gone_cannot_fly() {
        let mut pilot = docked_pilot("Ada");
        pilot.explore(SystemId(131));
        let text = nova_sim::save::encode(&pilot).replace("\"system\": 130", "\"system\": 131");
        let moved = nova_sim::save::decode(&text).expect("a pilot");
        let view = FlightView::with_pilot(catalog(), moved);
        assert_eq!(
            view.session().err(),
            Some("the pilot's system, sÿst 131, does not exist")
        );
        assert_eq!(view.pilot(), None);
    }

    #[test]
    fn landing_taking_off_and_transactions_make_a_save_due() {
        let centred = FakeCatalog {
            sites: vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)],
            ..catalog()
        };
        let mut view = FlightView::new(centred);
        assert!(!view.take_save_due());
        assert!(!view.transact(|pilot| pilot.set_cash(5)), "in flight");
        land_now(&mut view);
        assert!(view.take_save_due(), "landed");
        assert!(view.transact(|pilot| pilot.set_cash(5)));
        assert_eq!(view.pilot().map(Pilot::cash), Some(5));
        assert!(view.take_save_due(), "a transaction");
        assert!(!view.take_save_due(), "taken");
        view.take_off();
        assert!(view.take_save_due(), "took off");
    }

    #[test]
    fn a_view_that_cannot_fly_has_no_pilot_and_saves_nothing() {
        let broken = FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..catalog()
        };
        let mut view = FlightView::new(broken);
        assert_eq!(view.pilot(), None);
        assert!(!view.transact(|pilot| pilot.set_cash(5)));
        assert!(!view.take_save_due());
    }

    #[test]
    fn the_course_map_shows_the_explored_systems_and_arriving_explores() {
        let mut view = flight();
        let explored = |view: &View| {
            view.course_map()
                .explored()
                .map(|set| set.iter().copied().collect::<Vec<_>>())
        };
        assert_eq!(explored(&view), Some(vec![SystemId(130)]));
        plot(&mut view, 131);
        fly_out(&mut view);
        view.input(&key(JUMP, true));
        for _ in 0..90 {
            view.tick(TICK);
        }
        assert_eq!(view.session().expect("flying").system(), SystemId(131));
        assert_eq!(explored(&view), Some(vec![SystemId(130), SystemId(131)]));
    }

    // The exchange and the day's chances.

    use nova_sim::{Chance, Direction, DisasterId, Good, Lot, Order, TradeRefusal};

    /// Earth at the centre, a trade center trading food at 75, the first
    /// `chär` holding 1000 credits, and its ship 10 tons.
    fn trading() -> FakeCatalog {
        let flags = StellarFlags::CAN_LAND | StellarFlags::TRADE_CENTER | 2 << 28;
        FakeCatalog {
            character: Ok(CharacterStart {
                cash: 1000,
                ..catalog().character.expect("a chär")
            }),
            fields: ShipFields {
                holds: 10,
                ..FIELDS
            },
            sites: vec![site(128, (0.0, 0.0), flags)],
            commodities: CommodityStrings {
                names: vec!["Food".to_owned()],
                base_prices: vec!["75".to_owned()],
            },
            ..catalog()
        }
    }

    const BUY_FOOD: Order = Order {
        good: Good::Commodity(0),
        direction: Direction::Buy,
        lot: Lot::One,
    };

    #[test]
    fn the_exchange_is_the_sessions_and_a_trade_goes_through_it() {
        let mut view = FlightView::new(trading());
        assert_eq!(view.market(), None, "in flight");
        assert_eq!(view.trade(BUY_FOOD), Err(TradeRefusal::NoMarket));
        land_now(&mut view);
        view.take_save_due();
        let market = view.market().expect("landed at a trade center");
        assert_eq!(
            market.row(Good::Commodity(0)).map(|row| row.price),
            Some(75)
        );
        assert_eq!(view.trade(BUY_FOOD), Ok(1));
        assert_eq!(view.pilot().map(Pilot::cash), Some(925));
        assert!(view.take_save_due(), "a trade");
        assert_eq!(
            view.market()
                .and_then(|m| m.row(Good::Commodity(0)).map(|row| row.held)),
            Some(1)
        );
        let broken = FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..trading()
        };
        let mut broken = FlightView::new(broken);
        assert_eq!(broken.market(), None);
        assert_eq!(broken.trade(BUY_FOOD), Err(TradeRefusal::NoMarket));
    }

    use nova_sim::{OutfitOrder, OutfitRefusal};

    /// Earth at the centre, an outfitter selling a fuel tank (+100 fuel,
    /// a ton, 1000 credits), the first `chär` holding 1000 credits.
    fn outfitting() -> FakeCatalog {
        FakeCatalog {
            character: Ok(CharacterStart {
                cash: 1000,
                ..catalog().character.expect("a chär")
            }),
            sites: vec![site(
                128,
                (0.0, 0.0),
                StellarFlags::CAN_LAND | StellarFlags::OUTFITTER,
            )],
            outfits: vec![OutfitRecord {
                id: OutfitId(200),
                name: "Fuel Tank".to_owned(),
                short_name: "Fuel Tank".to_owned(),
                disp_weight: 0,
                mass: 1,
                tech_level: 1,
                max: 5,
                flags: 0,
                cost: 1000,
                mods: [(12, 100), (0, 0), (0, 0), (0, 0)],
                contribute: 0,
                require: 0,
                require_govt: -1,
                availability: String::new(),
            }],
            ..catalog()
        }
    }

    const BUY_TANK: OutfitOrder = OutfitOrder {
        outfit: OutfitId(200),
        direction: Direction::Buy,
    };

    #[test]
    fn the_outfitter_is_the_sessions_and_an_order_goes_through_it() {
        let mut view = FlightView::new(outfitting());
        assert_eq!(view.outfitter(), None, "in flight");
        assert_eq!(view.outfit(BUY_TANK), Err(OutfitRefusal::NoOutfitter));
        land_now(&mut view);
        view.take_save_due();
        let outfitter = view.outfitter().expect("landed at an outfitter");
        assert_eq!(
            outfitter.row(OutfitId(200)).map(|row| row.price),
            Some(1000)
        );
        assert_eq!(view.outfit(BUY_TANK), Ok(()));
        assert_eq!(view.pilot().map(Pilot::cash), Some(0));
        assert!(view.take_save_due(), "a purchase");
        assert_eq!(
            view.outfitter()
                .and_then(|o| o.row(OutfitId(200)).map(|row| row.owned)),
            Some(1)
        );
        assert_eq!(view.reserves().fuel.max, 350.0, "the tank");
        let broken = FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..outfitting()
        };
        let mut broken = FlightView::new(broken);
        assert_eq!(broken.outfitter(), None);
        assert_eq!(broken.outfit(BUY_TANK), Err(OutfitRefusal::NoOutfitter));
    }

    #[test]
    fn recharging_goes_through_the_session() {
        use nova_sim::RechargeRefusal;
        let mut view = FlightView::new(outfitting());
        assert_eq!(view.recharge(), Err(RechargeRefusal::NoFuel), "in flight");
        land_now(&mut view);
        view.take_save_due();
        assert_eq!(view.recharge(), Err(RechargeRefusal::Full));
        assert!(!view.take_save_due());
        let broken = FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..outfitting()
        };
        let mut broken = FlightView::new(broken);
        assert_eq!(broken.recharge(), Err(RechargeRefusal::NoFuel));
    }

    /// Fires every time, and records each percent it is asked.
    #[derive(Default)]
    struct Always {
        asked: Vec<u8>,
    }

    impl Chance for Always {
        fn fires(&mut self, percent: u8) -> bool {
            self.asked.push(percent);
            true
        }
    }

    /// A food surplus at Proxima, 35 % a day.
    fn eventful() -> FakeCatalog {
        FakeCatalog {
            disasters: vec![DisasterRecord {
                id: DisasterId(128),
                name: "An enormous food surplus".to_owned(),
                stellar: 140,
                commodity: 0,
                price_delta: -15,
                duration: 30,
                freq: 35,
                activate_on: String::new(),
            }],
            ..catalog()
        }
    }

    /// Jumps to Alpha Centauri and arrives.
    fn jump_to_alpha(view: &mut View) {
        plot(view, 131);
        fly_out(view);
        view.input(&key(JUMP, true));
        for _ in 0..90 {
            view.tick(TICK);
        }
        assert_eq!(view.session().expect("flying").system(), SystemId(131));
    }

    #[test]
    fn a_jump_rolls_the_days_events_on_the_chance_given() {
        let always = Rc::new(RefCell::new(Always::default()));
        let shared: Rc<RefCell<dyn Chance>> = always.clone();
        let mut view = FlightView::new(eventful()).with_chance(SharedChance::new(shared));
        jump_to_alpha(&mut view);
        assert_eq!(always.borrow().asked, [35]);
        let events: Vec<_> = view.pilot().expect("a pilot").events().collect();
        assert_eq!(events, [(DisasterId(128), 30)]);
        assert_eq!(format!("{:?}", SharedChance::default()), "Chance");
    }

    #[test]
    fn without_a_chance_given_nothing_random_happens() {
        let mut view = FlightView::new(eventful());
        jump_to_alpha(&mut view);
        assert_eq!(view.pilot().expect("a pilot").events().count(), 0);
        let mut shared = SharedChance::default();
        assert!(!shared.fires(100));
    }

    use nova_sim::ShipRefusal;

    /// [`outfitting`], where Earth is a shipyard too, selling ship 129 for
    /// 900 credits: twice as fast, with a fuel tank.
    fn shipbuying() -> FakeCatalog {
        let mut earth = site(
            128,
            (0.0, 0.0),
            StellarFlags::CAN_LAND | StellarFlags::OUTFITTER | StellarFlags::SHIPYARD,
        );
        earth.tech_level = 1;
        FakeCatalog {
            sites: vec![earth],
            ships: vec![ShipRecord {
                id: ShipId(129),
                name: "Fast".to_owned(),
                short_name: "Fast".to_owned(),
                long_name: String::new(),
                fields: ShipFields {
                    speed: 600,
                    ..FIELDS
                },
                defaults: vec![(OutfitId(200), 1)],
                cost: 900,
                tech_level: 1,
                buy_random: 100,
                require: 0,
                availability: String::new(),
                flags3: 0,
                disp_weight: 0,
                max_gun: 0,
                max_tur: 0,
                length: 0,
                crew: 0,
            }],
            ..outfitting()
        }
    }

    #[test]
    fn the_shipyard_is_the_sessions_and_a_purchase_reloads_the_ships_sheet() {
        let mut view = FlightView::new(shipbuying());
        assert_eq!(view.shipyard(), None, "in flight");
        assert_eq!(view.buy_ship(ShipId(129)), Err(ShipRefusal::NoShipyard));
        land_now(&mut view);
        view.take_save_due();
        let shipyard = view.shipyard().expect("landed at a shipyard");
        assert_eq!(shipyard.row(ShipId(129)).map(|row| row.price), Some(900));
        assert_eq!(*view.catalog().sheets_asked.borrow(), [ShipId(128)]);
        assert_eq!(
            view.buy_ship(ShipId(999)),
            Err(ShipRefusal::NotListed),
            "refused"
        );
        assert_eq!(*view.catalog().sheets_asked.borrow(), [ShipId(128)]);
        let bought = view.buy_ship(ShipId(129)).expect("bought");
        assert_eq!(bought.price, 900);
        assert!(view.take_save_due(), "a purchase");
        assert_eq!(view.pilot().map(Pilot::ship), Some(ShipId(129)));
        assert_eq!(view.pilot().map(Pilot::cash), Some(1000 - 900));
        assert_eq!(
            *view.catalog().sheets_asked.borrow(),
            [ShipId(128), ShipId(129)]
        );
        assert_eq!(view.reserves().fuel.max, 350.0, "its tank");
        // Off again, the new hull is drawn, at the new speed.
        view.take_off().expect("took off");
        let mut list = DrawList::new();
        view.draw(&mut list);
        assert!(
            list.iter().any(|command| matches!(
                command,
                DrawCommand::Sprite { image, .. } if image.id == 2001
            )),
            "the new sheet"
        );
        let ids: Vec<_> = ship_sprites(&view).iter().map(|(i, _)| i.id).collect();
        assert_eq!(ids, [2001, 2201], "and its lights");
        assert_eq!(
            view.session().map(|session| session.handling().max_speed),
            Ok(6.0)
        );
        let broken = FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..shipbuying()
        };
        let mut broken = FlightView::new(broken);
        assert_eq!(broken.shipyard(), None);
        assert_eq!(broken.buy_ship(ShipId(129)), Err(ShipRefusal::NoShipyard));
    }

    // The navigation target.

    fn nav_target(view: &View) -> Option<StellarId> {
        view.session().expect("flying").nav_target()
    }

    /// The texts drawn in the HUD's nav area, the stock bar's at (830, 0).
    fn nav(view: &View) -> Vec<String> {
        let area = layout().nav.offset(at(830.0, 0.0));
        drawn(view)
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, origin, .. } if area.contains(*origin) => {
                    Some(text.clone())
                }
                _ => None,
            })
            .collect()
    }

    /// The texts drawn in the cargo area, at the stock bar's (830, 0),
    /// with where each starts.
    fn cargo(view: &View) -> Vec<(String, Point)> {
        let area = layout().cargo.offset(at(830.0, 0.0));
        drawn(view)
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, origin, .. } if area.contains(*origin) => {
                    Some((text.clone(), *origin))
                }
                _ => None,
            })
            .collect()
    }

    /// The last line of the cargo area, (8, 458)-(184, 552) at (830, 0):
    /// one line of Geneva 12 (1.2 x 12) above its bottom.
    fn last_cargo_line() -> Point {
        at(838.0, 552.0 - 1.2 * 12.0)
    }

    #[test]
    fn the_hud_shows_the_date_on_the_cargo_areas_last_line() {
        assert_eq!(
            layout().cargo,
            crate::geometry::Bounds {
                min: at(8.0, 458.0),
                max: at(184.0, 552.0),
            }
        );
        let mut view = flight();
        assert_eq!(cargo(&view), [(DATE.to_owned(), last_cargo_line())]);
        jump_to_alpha(&mut view);
        assert_eq!(
            cargo(&view),
            [("June 24, 1177 NC".to_owned(), last_cargo_line())]
        );
    }

    #[test]
    fn tab_cycles_the_target_through_the_systems_stellars_in_nav_order() {
        let mut view = flight();
        assert_eq!(TARGET_KEY, Key::Tab);
        assert_eq!(nav_target(&view), None);
        // Earth, 128, comes first in the system's order, though the Moon
        // is nearer.
        for expected in [128, 129, 128] {
            assert_eq!(view.input(&key(Key::Tab, true)), ScreenAction::None);
            assert_eq!(nav_target(&view), Some(StellarId(expected)));
        }
    }

    #[test]
    fn a_repeat_or_release_of_tab_does_nothing() {
        let mut view = flight();
        view.input(&held(Key::Tab));
        view.input(&key(Key::Tab, false));
        assert_eq!(nav_target(&view), None);
        tap(&mut view, Key::Tab);
        view.input(&held(Key::Tab));
        view.input(&key(Key::Tab, false));
        assert_eq!(nav_target(&view), Some(StellarId(128)));
    }

    #[test]
    fn with_nothing_selected_and_no_course_the_hud_says_no_destination() {
        assert_eq!(nav(&flight()), [hud::NAV_NO_DESTINATION]);
    }

    #[test]
    fn the_hud_shows_the_selected_stellar_by_name() {
        let mut view = flight();
        tap(&mut view, Key::Tab);
        assert_eq!(nav(&view), [hud::NAV_STELLAR, "Earth"]);
        tap(&mut view, Key::Tab);
        assert_eq!(nav(&view), [hud::NAV_STELLAR, "Moon"]);
    }

    #[test]
    fn a_plotted_jump_shows_hyperspace_and_the_next_system_if_explored() {
        let mut view = flight();
        plot(&mut view, 131);
        assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, hud::NAV_UNEXPLORED]);
        jump_to_alpha(&mut view);
        assert_eq!(nav(&view), [hud::NAV_NO_DESTINATION], "arrived");
        plot(&mut view, 130);
        assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, "Sol"]);
    }

    #[test]
    fn with_a_target_and_a_course_the_hud_shows_the_target() {
        let mut view = flight();
        plot(&mut view, 131);
        tap(&mut view, Key::Tab);
        assert_eq!(nav(&view), [hud::NAV_STELLAR, "Earth"]);
        assert_eq!(
            view.session().expect("flying").course(),
            [SystemId(131)],
            "the course is kept"
        );
    }

    #[test]
    fn arriving_clears_the_target_and_tab_selects_in_the_new_system() {
        let mut view = flight();
        tap(&mut view, Key::Tab);
        jump_to_alpha(&mut view);
        assert_eq!(nav_target(&view), None);
        assert_eq!(nav(&view), [hud::NAV_NO_DESTINATION]);
        tap(&mut view, Key::Tab);
        assert_eq!(nav(&view), [hud::NAV_STELLAR, "Proxima"]);
    }

    #[test]
    fn tab_with_the_map_open_or_during_a_jump_changes_nothing() {
        let mut view = flight();
        tap(&mut view, MAP);
        tap(&mut view, Key::Tab);
        assert!(view.map_open());
        assert_eq!(nav_target(&view), None);
        tap(&mut view, MAP);
        tap(&mut view, Key::Tab);
        plot(&mut view, 131);
        fly_out(&mut view);
        view.input(&key(JUMP, true));
        view.tick(ms(300));
        tap(&mut view, Key::Tab);
        assert_eq!(nav_target(&view), Some(StellarId(128)));
    }

    #[test]
    fn tab_in_a_flight_that_never_started_does_nothing() {
        let mut view = FlightView::new(FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..catalog()
        });
        assert_eq!(view.input(&key(Key::Tab, true)), ScreenAction::None);
        assert!(view.session().is_err());
    }
}
