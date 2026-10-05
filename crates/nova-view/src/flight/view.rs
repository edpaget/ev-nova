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
//! [`StatusBars`] port, the galaxy for its course map, through the
//! [`GalaxyCatalog`] port, and the look of every weapon and explosion type,
//! through the [`CombatLooks`] port. After that it reads only when the
//! ship arrives in another system: that system, and when the system's NPC
//! traffic is populated: its traffic, through the [`TrafficCatalog`]
//! port, and the target cards and codes of the ship types and governments
//! it brings. Drawing and input never read anything.
//!
//! The session populates the system's NPC traffic when it says to (see
//! [`Session::tick_traffic`]), rolled on the screen's [`SharedChance`].
//! Each time, the sprite sheet of each ship type the traffic can spawn is
//! read, once per type for the life of the screen. After each step of the
//! player's ship, the traffic takes a step,
//! its NPCs deciding as the screen's [`Behaviour`] says
//! ([`FlightView::with_behaviour`]; [`Peaceful`] by default), and then the
//! fight ([`Session::tick_combat`]), its ships disabled as the screen's
//! [`DisableRule`] says ([`FlightView::with_disable_rule`];
//! [`NovaDisable`] by default), and their point defence engaging the
//! missiles its [`PointDefenceRule`] calls hostile
//! ([`FlightView::with_point_defence_rule`]; [`Allegiance`] by default). What could not be read of the looks
//! (each once, when the screen is built), then the session's diagnostics
//! about game data it does not handle yet, pass through
//! [`Screen::take_diagnostics`] for the app to write out. Each NPC is
//! drawn with its own ship's sprite, after the stellars and before the
//! player, smoothed between its last two steps as the player's ship is
//! (a crossed box when its sheet cannot be read), and as a dim blip on the
//! radar.
//!
//! Each step's fight events are drained into the [`Effects`]: its
//! explosions, debris and sounds, rolled on their own [`SharedChance`]
//! ([`FlightView::with_effects_chance`]), never on the simulation's. The
//! beams live before the fight's step are passed along, so a looped
//! weapon is heard once a beam. The fight is drawn over the stellars:
//! beams that go under the ships, then the ships, then the shots (each
//! smoothed as the ships are) and the other beams, then the explosions and
//! debris, then the brackets round the target ([`weapons`],
//! [`effects`](super::effects), [`target`]). The HUD shows the target
//! panel and the secondary weapon's line, its text laid out by the
//! screen's [`TextMetrics`] ([`FlightView::with_metrics`]). Landing and
//! arriving clear the effects.
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
//! - M (a press, not its repeats) opens the course map, a [`GalaxyMap`]
//!   in [`MapMode::Course`](crate::galaxy::MapMode::Course), and lets go
//!   of the flight keys. While it is open flight is paused, as in the
//!   original, only the map is drawn and every input goes to it, except
//!   that M closes it ([`FlightView::close_map`] closes it too, for the
//!   router's Escape). A system clicked on the map becomes the
//!   destination: the session plots the course there and the map shows
//!   it. The map shows the systems the pilot has explored, and the rest
//!   unexplored.
//! - J (a press) jumps to the next system on the course when the session
//!   allows it, and otherwise says why in the original's words (`STR#`
//!   2002), as a refused landing does. A jump plays its [`JumpEffect`]:
//!   the keys are let go and ignored and the session waits while the stars
//!   streak and the screen fades out; then the ship arrives, the new
//!   system is read and laid out, and it fades in. The HUD stays on top
//!   throughout.
//! - Space fires the primary weapons and Control the secondary selected,
//!   while held. W (a press) selects the next secondary weapon, and with
//!   Alt (Option) the one before. Tab (a press) targets the next ship in
//!   turn, and R the nearest ([`TargetPick`]). The original's Shift-Tab,
//!   back through the ships, is not bound: there is no Shift key yet.
//! - Escape belongs to the app's router, which closes the map or leaves
//!   flight. The screen never quits.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashSet};
use std::rc::Rc;
use std::time::Duration;

use nova_sim::{
    Allegiance, Behaviour, Chance, CombatCatalog, Condition, Controls, DisableRule, FixedStep,
    GovtId, JumpRefusal, LandingRefusal, Market, NeverFires, NovaDisable, Npc, NpcId, Order,
    OutfitOrder, OutfitRefusal, Outfitter, Peaceful, Pilot, PilotCatalog, PointDefenceRule,
    RechargeRefusal, Reserves, Rules, Session, ShipId, ShipPurchase, ShipRef, ShipRefusal,
    ShipState, Shipyard, StartError, StellarId, Steps, TargetPick, TradeRefusal, TrafficCatalog,
    Turn, Vec2, flight::normalized, flight::shortest_turn,
};

use super::catalog::{CombatLooks, Looks, ShipSheet, ShipSprites, StatusBars, TargetCard};
use super::effects::{Dying, Effects, Scene};
use super::hud::{self, HudState, StatusBar};
use super::jump::{JumpEffect, JumpPhase};
use super::sprite::rotation_frame;
use super::target::{self, TargetShown};
use super::weapons::{self, BeamShown, ShotShown};
use crate::draw::crossed_box;
use crate::galaxy::{GalaxyCatalog, GalaxyMap};
use crate::system::camera::Camera;
use crate::system::catalog::SystemCatalog;
use crate::system::scene::{self, PLACEHOLDER, PLACEHOLDER_SIZE, SystemScene};
use crate::system::starfield;
use crate::text::TextMetrics;
use crate::{
    Color, Diagnostic, DrawList, ImageKey, Input, Key, Point, Screen, ScreenAction, Sound,
};

/// The overlay: the system's title and the help line.
const TITLE: Point = Point::new(16.0, 32.0);
const TITLE_SIZE: f32 = 20.0;
const HELP_AT: Point = Point::new(16.0, 744.0);
const OVERLAY_SIZE: f32 = 14.0;
/// How far below the ship's placeholder the reason goes.
const MESSAGE_GAP: f32 = 22.0;
/// The help line.
pub const HELP: &str = "Arrows: fly   Space: fire   Ctrl: secondary   W: weapon   Tab: next target   R: nearest   L: land   M: map   J: jump   P: preferences   Esc: leave";
/// Where a message, such as why a landing was refused, goes: above the
/// help line.
pub const MESSAGE_AT: Point = Point::new(16.0, 720.0);
/// How long a message stays on screen.
pub const MESSAGE_SHOWN_FOR: Duration = Duration::from_secs(4);

/// The keys flight holds: the original's defaults, and Alt, which turns
/// the weapon select key back.
const FLIGHT_KEYS: [Key; 7] = [
    Key::Up,
    Key::Left,
    Key::Right,
    Key::Down,
    FIRE_KEY,
    SECONDARY_KEY,
    Key::Alt,
];

/// The primary fire key: the original's default (`fireKey0`, `_loadKeys`
/// @0xcd5f8).
pub const FIRE_KEY: Key = Key::Space;
/// The secondary fire key: the original's default (`fireKey1`).
pub const SECONDARY_KEY: Key = Key::Control;
/// The secondary weapon select key: the original's default
/// (`weapSelect`); with Alt (Option) it selects the one before.
pub const SELECT_KEY: Key = Key::Char('w');
/// The target select key: the original's default (`targSel`), which
/// picks the next ship in turn.
pub const TARGET_KEY: Key = Key::Tab;
/// The closest target key: the original's default (`closeTarg`), which
/// picks the nearest ship.
pub const NEAREST_KEY: Key = Key::Char('r');

/// The land key: the original's default (`STR#` 129, and `STR#` 2002
/// #25).
pub const LAND_KEY: Key = Key::Char('l');
/// The galaxy map key: the original's documented default (`Keys.nib`'s
/// `mapKey`; `STR#` 2002 #26-28 name a map key).
pub const MAP_KEY: Key = Key::Char('m');
/// The hyperspace jump key: the original's documented default
/// (`Keys.nib`'s `jumpKey`).
pub const JUMP_KEY: Key = Key::Char('j');

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
/// Not the original's, whose disabled ship takes no keys: worded after
/// `STR#` 2002 #42.
pub const JUMP_DISABLED: &str = "Can't initiate hyperspace jump - your ship is disabled.";

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
/// Not the original's, whose disabled ship takes no keys: worded as
/// [`IN_HYPERSPACE`] is.
pub const LAND_DISABLED: &str = "Unable to land - your ship is disabled.";

/// What the player is told when `refusal` stops a landing: the original's
/// words for it, for a station or a planet.
#[must_use]
pub fn refusal_message(refusal: &LandingRefusal) -> &'static str {
    let pick = |station: bool, at_station, on_planet| {
        if station { at_station } else { on_planet }
    };
    match *refusal {
        LandingRefusal::Jumping => IN_HYPERSPACE,
        LandingRefusal::Disabled => LAND_DISABLED,
        LandingRefusal::NoStellars => NO_STELLARS,
        LandingRefusal::TooFar { station, .. } => pick(station, TOO_FAR_STATION, TOO_FAR_PLANET),
        LandingRefusal::NotLandable { station, .. } => {
            pick(station, HOSTILE_STATION, HOSTILE_PLANET)
        }
        LandingRefusal::Denied { station, .. } => pick(station, DOCKING_DENIED, LANDING_DENIED),
        LandingRefusal::TooFast { station, .. } => pick(station, TOO_FAST_STATION, TOO_FAST_PLANET),
    }
}

/// Where a ship is drawn `alpha` of the way from `from` to `to`.
fn shown_position(from: &ShipState, to: &ShipState, alpha: f32) -> Point {
    let (from, to) = (from.position, to.position);
    Point::new(
        (to.x - from.x).mul_add(alpha, from.x),
        (to.y - from.y).mul_add(alpha, from.y),
    )
}

/// Which way a ship is drawn facing `alpha` of the way, the short way
/// round, from `from` to `to`.
fn shown_heading(from: &ShipState, to: &ShipState, alpha: f32) -> f32 {
    normalized(shortest_turn(from.heading, to.heading).mul_add(alpha, from.heading))
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
        JumpRefusal::Disabled => JUMP_DISABLED,
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

    fn below(&mut self, n: u32) -> u32 {
        self.0.borrow_mut().below(n)
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
    /// Time since the view opened, which drives the stellars' animations.
    elapsed: Duration,
    /// The flight keys held down.
    held: HashSet<Key>,
    /// The stellar landed on, until the router takes it.
    pending_landing: Option<StellarId>,
    /// The message shown, and `elapsed` when it was shown.
    message: Option<(&'static str, Duration)>,
    /// The course map, shown or not.
    map: GalaxyMap,
    /// Whether the course map is shown.
    map_open: bool,
    /// The jump's effect, while it plays.
    jump: Option<JumpEffect>,
    /// What each day's events and the traffic are rolled on.
    chance: SharedChance,
    /// How the NPCs decide.
    behaviour: Rc<dyn Behaviour>,
    /// When a ship in the fight is disabled.
    disable_rule: Rc<dyn DisableRule>,
    /// Which missiles the ships' point defence engages.
    defence_rule: Rc<dyn PointDefenceRule>,
    /// Each NPC ship type's sheet, or why it cannot be shown, read once.
    npc_sheets: BTreeMap<ShipId, Result<ShipSheet, String>>,
    /// Each NPC as it was a step before the session's.
    npc_previous: BTreeMap<NpcId, ShipState>,
    /// The weapons' and explosions' looks, read once.
    looks: Looks,
    /// What could not be read of the looks, until it is taken.
    unread_looks: Vec<Diagnostic>,
    /// Each NPC ship type's target card, read once.
    cards: BTreeMap<ShipId, TargetCard>,
    /// Each NPC government's target code, read once.
    codes: BTreeMap<GovtId, Option<String>>,
    /// The fight's explosions, debris and sounds.
    effects: Effects,
    /// What the effects are rolled on.
    effects_chance: SharedChance,
    /// What the HUD's text is measured by, if anything.
    metrics: Option<Metrics>,
}

/// Text metrics, shared.
#[derive(Clone)]
struct Metrics(Rc<dyn TextMetrics>);

impl std::fmt::Debug for Metrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Metrics")
    }
}

impl<
    C: PilotCatalog
        + TrafficCatalog
        + CombatCatalog
        + CombatLooks
        + SystemCatalog
        + ShipSprites
        + StatusBars
        + GalaxyCatalog,
> FlightView<C>
{
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
        let looks = match &session {
            Ok(_) => Looks::read(&catalog, catalog.weapons().iter().map(|weapon| weapon.id)),
            Err(_) => Looks::default(),
        };
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
            behaviour: Rc::new(Peaceful),
            disable_rule: Rc::new(NovaDisable),
            defence_rule: Rc::new(Allegiance),
            npc_sheets: BTreeMap::new(),
            npc_previous: BTreeMap::new(),
            unread_looks: looks
                .problems()
                .into_iter()
                .map(Diagnostic::Unreadable)
                .collect(),
            looks,
            cards: BTreeMap::new(),
            codes: BTreeMap::new(),
            effects: Effects::default(),
            effects_chance: SharedChance::default(),
            metrics: None,
        }
    }

    /// The flight with its explosions and debris rolled on `chance`, apart
    /// from the simulation's.
    #[must_use]
    pub fn with_effects_chance(self, effects_chance: SharedChance) -> Self {
        Self {
            effects_chance,
            ..self
        }
    }

    /// The flight with the HUD's text measured by `metrics`.
    #[must_use]
    pub fn with_metrics(self, metrics: Rc<dyn TextMetrics>) -> Self {
        Self {
            metrics: Some(Metrics(metrics)),
            ..self
        }
    }

    /// The flight with each day's events and the traffic rolled on
    /// `chance`.
    #[must_use]
    pub fn with_chance(self, chance: SharedChance) -> Self {
        Self { chance, ..self }
    }

    /// The flight with its NPCs deciding as `behaviour` says.
    #[must_use]
    pub fn with_behaviour(self, behaviour: Rc<dyn Behaviour>) -> Self {
        Self { behaviour, ..self }
    }

    /// The flight with its ships disabled as `rule` says.
    #[must_use]
    pub fn with_disable_rule(self, disable_rule: Rc<dyn DisableRule>) -> Self {
        Self {
            disable_rule,
            ..self
        }
    }

    /// The flight with its ships' point defence engaging the missiles
    /// `rule` calls hostile.
    #[must_use]
    pub fn with_point_defence_rule(self, defence_rule: Rc<dyn PointDefenceRule>) -> Self {
        Self {
            defence_rule,
            ..self
        }
    }

    /// Reads the sheet of each ship type the traffic can spawn that has
    /// not been read yet.
    fn read_npc_sheets(&mut self) {
        let Ok(session) = &self.session else {
            return;
        };
        for ship in session.traffic_ships() {
            self.npc_sheets
                .entry(ship)
                .or_insert_with(|| self.catalog.ship_sheet(ship));
            self.cards
                .entry(ship)
                .or_insert_with(|| self.catalog.target_card(ship));
        }
        for govt in session.npcs().iter().filter_map(|npc| npc.govt) {
            self.codes
                .entry(govt)
                .or_insert_with(|| self.catalog.target_code(govt));
        }
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
            Err(refusal) => self.message = Some((jump_refusal_message(&refusal), self.elapsed)),
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
        self.npc_previous.clear();
        self.effects.clear();
        self.read_npc_sheets();
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
    /// from before the landing; the session populates the system's
    /// traffic afresh on its next tick ([`Session::take_off`]).
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
                self.effects.clear();
            }
            Err(refusal) => self.message = Some((refusal_message(&refusal), self.elapsed)),
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
        shown_position(&self.previous, &self.current(), self.alpha)
    }

    /// Which way the ship is drawn facing: `alpha` of the way, the short way
    /// round, from its heading a step ago to its heading now.
    #[must_use]
    pub fn shown_heading(&self) -> f32 {
        shown_heading(&self.previous, &self.current(), self.alpha)
    }

    /// Where `npc` is drawn and which way it faces: `alpha` of the way from
    /// how it was a step ago, as the player's ship; where it is, when it
    /// was not there a step ago.
    fn shown_npc(&self, npc: &Npc) -> (Point, f32) {
        let from = self.npc_previous.get(&npc.id).unwrap_or(&npc.state);
        (
            shown_position(from, &npc.state, self.alpha),
            shown_heading(from, &npc.state, self.alpha),
        )
    }

    /// The NPCs, each with where it is drawn and which way it faces.
    fn shown_npcs(&self) -> Vec<(&Npc, Point, f32)> {
        let Ok(session) = &self.session else {
            return Vec::new();
        };
        session
            .npcs()
            .iter()
            .map(|npc| {
                let (at, heading) = self.shown_npc(npc);
                (npc, at, heading)
            })
            .collect()
    }

    /// Draws each NPC with its own ship's sprite, or a crossed box when its
    /// sheet cannot be read.
    fn draw_npcs(&self, list: &mut DrawList, camera: &Camera) {
        for (npc, at, heading) in self.shown_npcs() {
            let at = camera.world_to_screen(at);
            match self.npc_sheets.get(&npc.ship) {
                Some(Ok(sheet)) => {
                    let frame = rotation_frame(heading, sheet.rotations);
                    list.sprite(ImageKey::sprite(sheet.image_id, frame), at, Color::WHITE);
                }
                _ => crossed_box(list, at, PLACEHOLDER_SIZE, PLACEHOLDER),
            }
        }
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

    /// Picks the target as `pick` says.
    fn select_target(&mut self, pick: TargetPick) {
        if let Ok(session) = &mut self.session {
            session.select_target(pick);
        }
    }

    /// The fight's explosions, debris and sounds.
    #[must_use]
    pub fn effects(&self) -> &Effects {
        &self.effects
    }

    /// The shots in flight, each where it is drawn: `alpha` of the way
    /// from where it was a step ago, as the ships are.
    fn shown_shots(&self, camera: &Camera) -> Vec<ShotShown> {
        let Ok(session) = &self.session else {
            return Vec::new();
        };
        let behind = 1.0 - self.alpha;
        session
            .shots()
            .iter()
            .map(|shot| ShotShown {
                at: camera.world_to_screen(Point::new(
                    shot.velocity.x.mul_add(-behind, shot.position.x),
                    shot.velocity.y.mul_add(-behind, shot.position.y),
                )),
                heading: shot.heading,
                age: shot.age,
                weapon: shot.weapon.id,
            })
            .collect()
    }

    /// The beams being fired, each moved with its firer as it is drawn.
    fn shown_beams(&self, camera: &Camera) -> Vec<BeamShown> {
        let Ok(session) = &self.session else {
            return Vec::new();
        };
        session
            .beams()
            .iter()
            .map(|beam| {
                let (dx, dy) = self.drawn_off(session, beam.firer);
                let shown = |at: Vec2| camera.world_to_screen(Point::new(at.x + dx, at.y + dy));
                BeamShown {
                    start: shown(beam.start),
                    end: shown(beam.end),
                    weapon: beam.weapon.id,
                }
            })
            .collect()
    }

    /// How far `ship` is drawn from where it is.
    fn drawn_off(&self, session: &Session, ship: ShipRef) -> (f32, f32) {
        let (shown, now) = match ship {
            ShipRef::Player => (self.shown_position(), session.player().position),
            ShipRef::Npc(id) => match session.npcs().iter().find(|npc| npc.id == id) {
                Some(npc) => (self.shown_npc(npc).0, npc.state.position),
                None => return (0.0, 0.0),
            },
        };
        (shown.x - now.x, shown.y - now.y)
    }

    /// Draws the brackets round the target, where it is drawn.
    fn draw_brackets(&self, list: &mut DrawList, camera: &Camera) {
        let Some(npc) = self.session.as_ref().ok().and_then(Session::target) else {
            return;
        };
        let (at, _) = self.shown_npc(npc);
        let size = match self.npc_sheets.get(&npc.ship) {
            Some(Ok(sheet)) => sheet.frame_width.max(sheet.frame_height) as f32,
            _ => PLACEHOLDER_SIZE,
        };
        let disabled = npc.condition == Condition::Disabled;
        target::draw_brackets(list, camera.world_to_screen(at), size, disabled);
    }

    /// Draws the status bar's target panel and secondary weapon line.
    fn draw_combat_hud(&self, list: &mut DrawList, bar: &StatusBar) {
        let Ok(session) = &self.session else {
            return;
        };
        let origin = hud::bar_origin(bar);
        let metrics = self.metrics.as_ref().map(|metrics| &*metrics.0);
        let unread = TargetCard::default();
        let shown = session.target().map(|npc| TargetShown {
            name: session.ship_name(npc.ship).unwrap_or_default(),
            card: self.cards.get(&npc.ship).unwrap_or(&unread),
            code: npc.govt.and_then(|govt| self.codes.get(&govt)?.as_deref()),
            reserves: npc.reserves,
            disabled: npc.condition == Condition::Disabled,
        });
        target::draw_target_panel(list, &bar.layout, origin, shown.as_ref(), metrics);
        let line = session.secondary().map(|id| {
            let look = self.looks.weapon(id);
            let name = look.map_or_else(|| format!("wëap {}", id.0), |look| look.name.clone());
            let flags2 = look.map_or(0, |look| look.flags2);
            target::secondary_text(&name, session.secondary_rounds(), flags2)
        });
        target::draw_secondary(list, &bar.layout, origin, line.as_deref(), metrics);
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

impl<
    C: PilotCatalog
        + TrafficCatalog
        + CombatCatalog
        + CombatLooks
        + SystemCatalog
        + ShipSprites
        + StatusBars
        + GalaxyCatalog,
> Screen for FlightView<C>
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
            Some(TARGET_KEY) => self.select_target(TargetPick::Next),
            Some(NEAREST_KEY) => self.select_target(TargetPick::Nearest),
            Some(SELECT_KEY) => {
                let backwards = self.held.contains(&Key::Alt);
                if let Ok(session) = &mut self.session {
                    session.select_secondary(backwards);
                }
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
        let fire = (
            self.held.contains(&FIRE_KEY),
            self.held.contains(&SECONDARY_KEY),
        );
        if let Ok(session) = &mut self.session {
            session.hold_fire(fire.0, fire.1);
            for _ in 0..steps {
                self.previous = *session.player();
                self.npc_previous = session
                    .npcs()
                    .iter()
                    .map(|npc| (npc.id, npc.state))
                    .collect();
                session.tick(controls);
                session.tick_traffic(&self.catalog, &*self.behaviour, &mut self.chance);
                let rules = Rules {
                    disable: &*self.disable_rule,
                    defence: &*self.defence_rule,
                    law: &nova_sim::NovaLaw,
                };
                session.tick_combat(rules, &mut self.chance);
                let player = point(session.player().position);
                let dying = dying(session, &self.sheet, &self.npc_sheets);
                let chance = &mut self.effects_chance;
                self.effects.step(&dying, player, &self.looks, chance);
                let scene = Scene { player };
                for event in session.take_combat_events() {
                    self.effects.apply(&event, &scene, &self.looks, chance);
                }
            }
        }
        // The session may have populated its system afresh.
        self.read_npc_sheets();
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
        let beams = self.shown_beams(&camera);
        weapons::draw_beams(list, &beams, &self.looks, true);
        self.draw_npcs(list, &camera);
        self.draw_ship(list, camera.world_to_screen(self.shown_position()));
        weapons::draw_shots(list, &self.shown_shots(&camera), &self.looks);
        weapons::draw_beams(list, &beams, &self.looks, false);
        self.effects.draw(list, &camera, &self.looks);
        self.draw_brackets(list, &camera);
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
                let ships: Vec<Point> = self.shown_npcs().iter().map(|&(_, at, _)| at).collect();
                let state = HudState {
                    position: self.shown_position(),
                    stellars: &stellars,
                    ships: &ships,
                    reserves: self.reserves(),
                    system: scene.name(),
                };
                hud::draw(list, bar, &state);
                self.draw_combat_hud(list, bar);
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

    /// The session's sounds (thrust, landing, taking off and jumping),
    /// then the fight's. The course map has no buttons that sound.
    fn take_sounds(&mut self) -> Vec<Sound> {
        let mut sounds: Vec<Sound> = self
            .session
            .as_mut()
            .map(|session| session.take_sounds().into_iter().map(Sound::Sim).collect())
            .unwrap_or_default();
        sounds.extend(self.effects.take_sounds().into_iter().map(Sound::Combat));
        sounds
    }

    /// What could not be read of the looks, then the session's
    /// diagnostics, each once.
    fn take_diagnostics(&mut self) -> Vec<Diagnostic> {
        let mut diagnostics = std::mem::take(&mut self.unread_looks);
        if let Ok(session) = &mut self.session {
            let sim = session.take_diagnostics().into_iter();
            diagnostics.extend(sim.map(Diagnostic::Sim));
        }
        diagnostics
    }
}

/// The simulation's `v` as a view point.
fn point(v: Vec2) -> Point {
    Point::new(v.x, v.y)
}

/// The ships in `session` breaking up, each with its sprite's width from
/// `player`'s sheet or the NPCs' `sheets` (a placeholder's without one).
fn dying(
    session: &Session,
    player: &Result<ShipSheet, String>,
    sheets: &BTreeMap<ShipId, Result<ShipSheet, String>>,
) -> Vec<Dying> {
    let width = |sheet: Option<&Result<ShipSheet, String>>| match sheet {
        Some(Ok(sheet)) => sheet.frame_width as f32,
        _ => PLACEHOLDER_SIZE,
    };
    let breaking = |condition, at: Vec2, sprite_width, explosion| match condition {
        Condition::Dying { ticks_left } => Some(Dying {
            at: point(at),
            sprite_width,
            explosion,
            ticks_left,
        }),
        _ => None,
    };
    let me = breaking(
        session.player_condition(),
        session.player().position,
        width(Some(player)),
        session.hull().breakup,
    );
    let npcs = session.npcs().iter().filter_map(|npc| {
        breaking(
            npc.condition,
            npc.state.position,
            width(sheets.get(&npc.ship)),
            npc.hull.breakup,
        )
    });
    me.into_iter().chain(npcs).collect()
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::RefCell;
    use std::num::NonZeroU16;

    use nova_sim::landing::{LandingRefusal, StellarFlags};
    use nova_sim::{
        CharacterStart, CommodityStrings, DisasterRecord, Handling, JunkRecord, LandingSite,
        OutfitId, OutfitRecord, Reserves, ShipFields, ShipId, ShipRecord, ShipStats, SimDiagnostic,
        SimSound, SoundId, StarSystem, StartDate, StartError, SystemId, TICK, Vec2, step,
    };

    use super::*;
    use crate::flight::catalog::{BoomLook, GovtId, StatusBarLayout, TargetCard, WeaponLook};
    use crate::flight::hud::{self, HudState, StatusBar};
    use crate::galaxy::{Galaxy, MapMode, SystemEntry};
    use crate::sound::Sound;
    use crate::system::camera::VIEW_CENTER;
    use crate::system::catalog::{
        AnimationData, StellarContents, StellarId, StellarSheet, SystemContents,
    };
    use crate::{DrawCommand, Font};
    use nova_sim::hyperspace::{JumpRefusal, MIN_JUMP_DISTANCE};

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
        /// Each system's traffic: none, by default.
        traffic: Vec<(SystemId, nova_sim::SystemTraffic)>,
        /// The düdes: none, by default.
        dudes: Vec<(nova_sim::DudeId, nova_sim::DudeRecord)>,
        /// The weapons: none, by default.
        weapons: Vec<nova_sim::WeaponRecord>,
        /// The ship types' combat fields: none, by default.
        hulls: Vec<nova_sim::HullRecord>,
        /// The governments: none, by default.
        govts: Vec<nova_sim::GovtRecord>,
        /// The weapons' looks: none, by default.
        looks: Vec<(i16, Result<WeaponLook, String>)>,
        /// The explosions' looks: none, by default.
        booms: Vec<(i16, Result<BoomLook, String>)>,
        /// The ship types' target cards: none, by default.
        cards: Vec<(i16, TargetCard)>,
        /// The governments' target codes: none, by default.
        codes: Vec<(i16, String)>,
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
        shield_rech: 0,
        armor_rech: 0,
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
                start: StartDate::default(),
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
            traffic: Vec::new(),
            dudes: Vec::new(),
            weapons: Vec::new(),
            hulls: Vec::new(),
            govts: Vec::new(),
            looks: Vec::new(),
            booms: Vec::new(),
            cards: Vec::new(),
            codes: Vec::new(),
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
            weap: rect(8.0, 300.0, 184.0, 315.0),
            targ: rect(8.0, 330.0, 184.0, 442.0),
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
            subtitle_size: 10.0,
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
    }

    /// The weapons, ship types' combat fields and governments given.
    impl CombatCatalog for FakeCatalog {
        fn weapons(&self) -> Vec<nova_sim::WeaponRecord> {
            self.weapons.clone()
        }

        fn hulls(&self) -> Vec<nova_sim::HullRecord> {
            self.hulls.clone()
        }

        fn governments(&self) -> Vec<nova_sim::GovtRecord> {
            self.govts.clone()
        }
    }

    /// The traffic and düdes given; no fleets.
    impl TrafficCatalog for FakeCatalog {
        fn system_traffic(&self, id: SystemId) -> Option<nova_sim::SystemTraffic> {
            self.traffic
                .iter()
                .find(|(system, _)| *system == id)
                .map(|(_, traffic)| *traffic)
        }

        fn dude(&self, id: nova_sim::DudeId) -> Option<nova_sim::DudeRecord> {
            self.dudes
                .iter()
                .find(|(dude, _)| *dude == id)
                .map(|(_, record)| record.clone())
        }

        fn fleets(&self) -> Vec<nova_sim::FleetRecord> {
            Vec::new()
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
        /// `rlëD` 2001's, with 72 rotations; ship 130's cannot be read.
        fn ship_sheet(&self, id: ShipId) -> Result<ShipSheet, String> {
            self.sheets_asked.borrow_mut().push(id);
            match id.0 {
                128 => self.sheet.clone(),
                129 => Ok(ShipSheet {
                    image_id: 2001,
                    rotations: NonZeroU16::new(72).expect("non-zero"),
                    ..sheet()
                }),
                130 => Err("no shän 130".to_owned()),
                other => panic!("asked for shïp {other}'s sheet"),
            }
        }
    }

    /// The looks, cards and codes given; any other weapon cannot be read,
    /// and there is no other explosion type.
    impl CombatLooks for FakeCatalog {
        fn weapon_look(&self, id: nova_sim::WeaponId) -> Result<WeaponLook, String> {
            self.looks
                .iter()
                .find(|(weapon, _)| *weapon == id.0)
                .map_or_else(
                    || Err(format!("no wëap {}", id.0)),
                    |(_, look)| look.clone(),
                )
        }

        fn boom_look(&self, id: nova_sim::BoomId) -> Option<Result<BoomLook, String>> {
            self.booms
                .iter()
                .find(|(boom, _)| *boom == id.0)
                .map(|(_, look)| look.clone())
        }

        fn target_card(&self, ship: ShipId) -> TargetCard {
            self.cards
                .iter()
                .find(|(id, _)| *id == ship.0)
                .map(|(_, card)| card.clone())
                .unwrap_or_default()
        }

        fn target_code(&self, govt: GovtId) -> Option<String> {
            self.codes
                .iter()
                .find(|(id, _)| *id == govt.0)
                .map(|(_, code)| code.clone())
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
            key(Key::Enter, true),
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
                ships: &[],
                reserves: ShipStats::new(FIELDS, &[]).full(),
                system: "Sol",
            },
        );
        let bar = view.status_bar().expect("a status bar");
        let origin = hud::bar_origin(bar);
        target::draw_target_panel(&mut expected, &bar.layout, origin, None, None);
        target::draw_secondary(&mut expected, &bar.layout, origin, None, None);
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
            "Arrows: fly   Space: fire   Ctrl: secondary   W: weapon   Tab: next target   R: nearest   L: land   M: map   J: jump   P: preferences   Esc: leave"
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
        let names: Vec<String> = texts(&list).into_iter().rev().take(3).collect();
        assert_eq!(names, [NO_SECONDARY, NO_TARGET, "Sol"]);
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
                ships: &[],
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
            (LandingRefusal::Jumping, IN_HYPERSPACE),
            (LandingRefusal::Disabled, LAND_DISABLED),
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
        view.input(&key(LAND, true));
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
        assert_eq!(jump_refusal_message(&JumpRefusal::Disabled), JUMP_DISABLED);
        assert_eq!(
            [LAND_DISABLED, JUMP_DISABLED],
            [
                "Unable to land - your ship is disabled.",
                "Can't initiate hyperspace jump - your ship is disabled."
            ]
        );
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
        view.input(&key(LAND, true));
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
                ships: &[],
                reserves: reserves(&view),
                system: "Alpha Centauri",
            },
        );
        assert!(
            list.iter()
                .skip(hud_at(&list))
                .take(hud.len())
                .eq(hud.iter()),
            "the HUD, with a jump's fuel less, is last"
        );
        assert_eq!(
            list.len(),
            hud_at(&list) + hud.len() + 2,
            "then the target and weapon"
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
        tap(&mut view, LAND_KEY);
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
        tap(&mut view, LAND_KEY);
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
        tap(&mut view, LAND_KEY);
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
        tap(&mut view, LAND_KEY);
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
        tap(&mut view, LAND_KEY);
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

    /// Fires every time, and records each percent it is asked; each draw
    /// is 0, and records its `n`.
    #[derive(Default)]
    struct Always {
        asked: Vec<u8>,
        drawn: Vec<u32>,
    }

    impl Chance for Always {
        fn fires(&mut self, percent: u8) -> bool {
            self.asked.push(percent);
            true
        }

        fn below(&mut self, n: u32) -> u32 {
            self.drawn.push(n);
            0
        }
    }

    #[test]
    fn a_shared_chance_draws_from_its_source() {
        let always = Rc::new(RefCell::new(Always::default()));
        let shared: Rc<RefCell<dyn Chance>> = always.clone();
        let mut chance = SharedChance::new(shared);
        assert_eq!(chance.below(7), 0);
        assert_eq!(chance.below(500), 0);
        assert_eq!(always.borrow().drawn, [7, 500]);
        assert_eq!(SharedChance::default().below(256), 255, "never fires");
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
                inherent_ai: 1,
            }],
            ..outfitting()
        }
    }

    #[test]
    fn the_shipyard_is_the_sessions_and_a_purchase_reloads_the_ships_sheet() {
        let mut view = FlightView::new(shipbuying());
        assert_eq!(view.shipyard(), None, "in flight");
        assert_eq!(view.buy_ship(ShipId(129)), Err(ShipRefusal::NoShipyard));
        tap(&mut view, LAND_KEY);
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
    // Traffic.

    use std::collections::VecDeque;

    use nova_sim::{Goal, NpcId};

    /// Draws its script, then the last outcome, so no roll fires;
    /// records each `n` asked.
    #[derive(Default)]
    struct Script {
        draws: VecDeque<u32>,
        asked: Vec<u32>,
    }

    impl Chance for Script {
        fn fires(&mut self, _percent: u8) -> bool {
            false
        }

        fn below(&mut self, n: u32) -> u32 {
            self.asked.push(n);
            self.draws.pop_front().unwrap_or(n - 1)
        }
    }

    fn scripted(draws: &[u32]) -> (Rc<RefCell<Script>>, SharedChance) {
        let script = Rc::new(RefCell::new(Script {
            draws: draws.iter().copied().collect(),
            asked: Vec::new(),
        }));
        let shared: Rc<RefCell<dyn Chance>> = script.clone();
        (script, SharedChance::new(shared))
    }

    /// One setup pass placing the düde's ship at (`x` - 750, `y` - 750)
    /// facing `heading`.
    fn placed(x: u32, y: u32, heading: u32) -> [u32; 7] {
        [6, 6, 0, 0, x, y, heading]
    }

    /// Decides nothing: every NPC idles.
    #[derive(Debug)]
    struct Still;

    impl Behaviour for Still {
        fn decide(
            &self,
            _npc: &nova_sim::Npc,
            _around: &nova_sim::Surroundings,
            _chance: &mut dyn Chance,
        ) -> Goal {
            Goal::Idle
        }
    }

    /// [`catalog`] with `avg` ships on average in each of `systems`, all of
    /// düde 128's ship `ship` with AI `ai_type`; ships 129 and 130 are
    /// records the traffic can fly.
    fn trafficked(systems: &[i16], avg: i16, ship: i16, ai_type: i16) -> FakeCatalog {
        let mut dude_types = [(-1, 0); 8];
        dude_types[0] = (128, 100);
        let record = |id| ShipRecord {
            id: ShipId(id),
            name: format!("Ship {id}"),
            short_name: String::new(),
            long_name: String::new(),
            fields: ShipFields {
                speed: 600,
                ..FIELDS
            },
            defaults: Vec::new(),
            cost: 1,
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
            inherent_ai: 1,
        };
        FakeCatalog {
            traffic: systems
                .iter()
                .map(|&id| {
                    (
                        SystemId(id),
                        nova_sim::SystemTraffic {
                            dude_types,
                            avg_ships: avg,
                        },
                    )
                })
                .collect(),
            dudes: vec![(
                nova_sim::DudeId(128),
                nova_sim::DudeRecord {
                    ai_type,
                    govt: None,
                    ships: vec![(ShipId(ship), 1)],
                },
            )],
            ships: vec![record(129), record(130)],
            ..catalog()
        }
    }

    fn npc_ids(view: &View) -> Vec<NpcId> {
        let session = view.session().expect("flying");
        session.npcs().iter().map(|npc| npc.id).collect()
    }

    fn npc_states(view: &View) -> Vec<ShipState> {
        let session = view.session().expect("flying");
        session.npcs().iter().map(|npc| npc.state).collect()
    }

    #[test]
    fn the_traffic_is_populated_when_the_flight_starts() {
        let (script, chance) = scripted(&placed(850, 650, 90));
        let mut view = FlightView::new(trafficked(&[130], 1, 129, 3))
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        assert_eq!(npc_ids(&view), [], "not until the first tick");
        view.tick(TICK);
        let session = view.session().expect("flying");
        let npc = &session.npcs()[0];
        assert_eq!((npc.ship, npc.goal), (ShipId(129), Goal::Idle));
        assert_eq!(npc.state.position, Vec2::new(100.0, -100.0));
        assert_eq!(&script.borrow().asked[..7], [7, 7, 100, 1, 1500, 1500, 360]);
        ticks(&mut view, 3);
        assert_eq!(npc_ids(&view), [NpcId(0)], "populated once");
        let npc = &view.session().expect("flying").npcs()[0];
        assert_eq!(npc.goal, Goal::Idle, "deciding as the behaviour given says");
        assert_eq!(npc.state.position, Vec2::new(100.0, -100.0));
    }

    #[test]
    fn an_npc_is_drawn_with_its_own_sprite_where_it_is_facing_its_heading() {
        let (_, chance) = scripted(&placed(850, 650, 90));
        let mut view = FlightView::new(trafficked(&[130], 1, 129, 3))
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        let list = drawn(&view);
        let frame = rotation_frame(90.0, NonZeroU16::new(72).expect("non-zero"));
        assert_eq!(
            sprites(&list),
            [
                (ImageKey::sprite(1128, 0), at(512.0, -216.0)),
                (ImageKey::sprite(1129, 1), at(812.0, 184.0)),
                (ImageKey::sprite(2001, frame), at(612.0, 284.0)),
                (ImageKey::sprite(2000, 0), VIEW_CENTER),
            ],
            "after the stellars, before the player"
        );
    }

    #[test]
    fn an_npc_is_a_dim_blip_on_the_radar() {
        let (_, chance) = scripted(&placed(850, 650, 90));
        let mut view = FlightView::new(trafficked(&[130], 1, 129, 3))
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        let bar = view.status_bar().expect("a status bar");
        let radar = bar.layout.radar.offset(hud::bar_origin(bar));
        let blip = hud::radar_point(radar, at(0.0, 0.0), at(100.0, -100.0)).expect("in range");
        let list = drawn(&view);
        let dim: Vec<Point> = list
            .iter()
            .filter_map(|command| match *command {
                DrawCommand::Dot { center, color, .. } if color == layout().dim_radar => {
                    Some(center)
                }
                _ => None,
            })
            .collect();
        assert_eq!(dim, [blip]);
    }

    #[test]
    fn an_npc_is_drawn_between_its_last_two_steps() {
        // At (0, 100) facing up, it heads for Earth, straight up.
        let mut draws = placed(750, 850, 0).to_vec();
        draws.push(0);
        let (_, chance) = scripted(&draws);
        let mut view = FlightView::new(trafficked(&[130], 1, 129, 1)).with_chance(chance);
        // The first step sets the system up; the NPC moves from the next.
        view.tick(TICK);
        assert_eq!(npc_states(&view)[0].position, Vec2::new(0.0, 100.0));
        view.tick(TICK + TICK / 2);
        let npc = &view.session().expect("flying").npcs()[0];
        assert_eq!(npc.goal, Goal::Land(StellarId(128)), "Peaceful, by default");
        let moved = npc.state.position.y - 100.0;
        assert!(moved < 0.0, "{npc:?}");
        let alpha = view.alpha();
        assert!(alpha > 0.4, "{alpha}");
        let shown = sprites(&drawn(&view))[2].1;
        let expected = view
            .camera()
            .world_to_screen(at(0.0, moved.mul_add(alpha, 100.0)));
        assert!(
            (shown.y - expected.y).abs() < 1e-3,
            "{shown:?} {expected:?}"
        );
        assert_eq!(shown.x, expected.x);
    }

    #[test]
    fn an_npc_whose_sheet_cannot_be_read_is_a_crossed_box() {
        let (_, chance) = scripted(&placed(850, 650, 90));
        let mut view = FlightView::new(trafficked(&[130], 1, 130, 3))
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        let list = drawn(&view);
        let mut boxed = DrawList::new();
        crossed_box(&mut boxed, at(612.0, 284.0), PLACEHOLDER_SIZE, PLACEHOLDER);
        let commands: Vec<_> = list.iter().cloned().collect();
        let expected: Vec<_> = boxed.iter().cloned().collect();
        assert!(
            commands.windows(expected.len()).any(|run| run == expected),
            "{commands:?}"
        );
        assert_eq!(sprites(&list).len(), 3, "two stellars and the player");
    }

    #[test]
    fn each_ship_types_sheet_is_read_once() {
        let mut view = FlightView::new(trafficked(&[130, 131], 3, 129, 1));
        assert_eq!(*view.catalog().sheets_asked.borrow(), [ShipId(128)]);
        ticks(&mut view, 5);
        assert_eq!(npc_ids(&view).len(), 3);
        assert_eq!(
            *view.catalog().sheets_asked.borrow(),
            [ShipId(128), ShipId(129)]
        );
        jump_to_alpha(&mut view);
        assert!(!npc_ids(&view).is_empty());
        assert_eq!(
            *view.catalog().sheets_asked.borrow(),
            [ShipId(128), ShipId(129)],
            "not again"
        );
    }

    #[test]
    fn arriving_repopulates_from_the_new_system() {
        let mut view = FlightView::new(trafficked(&[131], 2, 129, 1));
        view.tick(TICK);
        assert_eq!(npc_ids(&view), []);
        jump_to_alpha(&mut view);
        assert_eq!(npc_ids(&view).len(), 2);
        assert_eq!(
            *view.catalog().sheets_asked.borrow(),
            [ShipId(128), ShipId(129)],
            "read on arrival"
        );
    }

    #[test]
    fn npcs_stand_still_while_the_map_is_open_or_the_ship_is_landed() {
        let catalog = FakeCatalog {
            sites: vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)],
            ..trafficked(&[130], 2, 129, 1)
        };
        let mut view = FlightView::new(catalog);
        ticks(&mut view, 3);
        let before = npc_states(&view);
        assert_eq!(before.len(), 2);
        tap(&mut view, MAP);
        ticks(&mut view, 10);
        assert_eq!(npc_states(&view), before, "the map is open");
        tap(&mut view, MAP);
        tap(&mut view, LAND_KEY);
        assert!(view.take_landing().is_some());
        let before = npc_states(&view);
        ticks(&mut view, 10);
        assert_eq!(npc_states(&view), before, "landed");
    }

    #[test]
    fn taking_off_repopulates_the_system() {
        let catalog = FakeCatalog {
            sites: vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)],
            ..trafficked(&[130], 2, 129, 1)
        };
        let mut view = FlightView::new(catalog);
        view.tick(TICK);
        assert_eq!(npc_ids(&view), [NpcId(0), NpcId(1)]);
        tap(&mut view, LAND_KEY);
        view.take_off().expect("took off");
        view.tick(TICK);
        assert_eq!(npc_ids(&view), [NpcId(2), NpcId(3)]);
    }

    // Combat.

    /// Says no ship is disabled, counting the times it is asked.
    #[derive(Debug, Default)]
    struct Counting {
        asked: std::cell::Cell<usize>,
    }

    impl DisableRule for Counting {
        fn disabled(&self, _armor: nova_sim::Gauge, _hull: &nova_sim::HullSpec) -> bool {
            self.asked.set(self.asked.get() + 1);
            false
        }
    }

    #[test]
    fn the_fight_asks_the_disable_rule_given_each_step_in_flight() {
        let rule = Rc::new(Counting::default());
        let mut view = flight_among(vec![site(140, (0.0, 0.0), StellarFlags::CAN_LAND)])
            .with_disable_rule(rule.clone());
        view.tick(TICK);
        assert_eq!(rule.asked.get(), 1, "the player, the only ship");
        ticks(&mut view, 2);
        assert_eq!(rule.asked.get(), 3);
        view.input(&key(MAP_KEY, true));
        ticks(&mut view, 2);
        assert_eq!(rule.asked.get(), 3, "not while the map is open");
        view.input(&key(MAP_KEY, true));
        view.input(&key(LAND_KEY, true));
        assert!(view.take_landing().is_some());
        ticks(&mut view, 2);
        assert_eq!(rule.asked.get(), 3, "not while landed");
    }

    /// Every NPC idles and holds its trigger.
    #[derive(Debug)]
    struct Firing;

    impl Behaviour for Firing {
        fn decide(
            &self,
            _npc: &nova_sim::Npc,
            _around: &nova_sim::Surroundings,
            _chance: &mut dyn Chance,
        ) -> Goal {
            Goal::Idle
        }

        fn trigger(
            &self,
            _npc: &nova_sim::Npc,
            _around: &nova_sim::Surroundings,
        ) -> nova_sim::Trigger {
            nova_sim::Trigger {
                primary: true,
                secondary: None,
            }
        }
    }

    #[test]
    fn the_sessions_diagnostics_come_through_the_screen() {
        let mining = nova_sim::WeaponRecord {
            id: nova_sim::WeaponId(181),
            reload: 20,
            count: 12,
            mass_dmg: 2,
            energy_dmg: 2,
            guidance: -1,
            speed: 1000,
            ammo_type: -1,
            inaccuracy: 0,
            impact: 5,
            explod_type: -1,
            prox_radius: 5,
            blast_radius: 6,
            flags: 0,
            seeker: 0,
            flags2: 0x8000,
            flags3: 0,
            decay: 0,
            beam_length: 0,
            burst_count: 0,
            burst_reload: 0,
            guided_turn: 0,
            durability: 0,
            sub_count: 0,
            sub_type: None,
            sub_theta: 0,
            sub_limit: 0,
        };
        let hull = nova_sim::HullRecord {
            id: ShipId(129),
            flags: 0,
            death_delay: 0,
            explode1: -1,
            explode2: -1,
            mass: 0,
            weapons: vec![nova_sim::StockWeapon {
                weapon: nova_sim::WeaponId(181),
                count: 1,
                ammo: 0,
            }],
            size: None,
            strength: 0,
        };
        let (_, chance) = scripted(&placed(850, 650, 90));
        let catalog = FakeCatalog {
            weapons: vec![mining],
            hulls: vec![hull],
            looks: vec![(181, Ok(WeaponLook::default()))],
            ..trafficked(&[130], 1, 129, 3)
        };
        let mut view = FlightView::new(catalog)
            .with_chance(chance)
            .with_behaviour(Rc::new(Firing));
        assert_eq!(view.take_diagnostics(), []);
        ticks(&mut view, 3);
        assert_eq!(
            view.take_diagnostics(),
            [Diagnostic::Sim(SimDiagnostic::UnimplementedWeaponFlag {
                weapon: nova_sim::WeaponId(181),
                field: nova_sim::combat::flags::FlagField::Flags2,
                bit: 0x8000
            })]
        );
        ticks(&mut view, 30);
        assert_eq!(view.take_diagnostics(), [], "once");
    }

    #[test]
    fn a_look_whose_sheet_cannot_be_read_still_sounds_and_is_reported_once() {
        let mut catalog = armed(0, &[BLASTER]);
        catalog.looks[0].1 = Ok(WeaponLook {
            sheet: Some(Err("no spïn 3005".to_owned())),
            ..look("Blaster", 208, None)
        });
        catalog.booms.push((
            129,
            Ok(BoomLook {
                sheet: Err("no spïn 401".to_owned()),
                advance: 1.0,
                sound: None,
            }),
        ));
        let mut view = fighting(catalog, &[]);
        assert_eq!(
            view.take_diagnostics(),
            [
                Diagnostic::Unreadable("wëap 128: no spïn 3005".to_owned()),
                Diagnostic::Unreadable("no wëap 150".to_owned()),
                Diagnostic::Unreadable("bööm 129: no spïn 401".to_owned()),
            ],
            "and the unseen weapon, which has no look at all"
        );
        view.input(&key(FIRE_KEY, true));
        view.tick(TICK);
        let sounds: Vec<_> = combat_sounds(&mut view).iter().map(|s| s.sound).collect();
        assert_eq!(sounds, [nova_sim::SoundId(208)], "the blaster is heard");
        ticks(&mut view, 3);
        assert_eq!(view.take_diagnostics(), [], "once");
    }

    // Combat controls and display.

    use std::num::NonZeroU16 as Frames;

    use nova_sim::{BoomId, Condition, WeaponId};

    use crate::flight::catalog::EffectSheet;
    use crate::flight::effects::{DEBRIS_COLOR, Effects};
    use crate::flight::target::{self, BRACKETS, DISABLED_BRACKETS, NO_SECONDARY, NO_TARGET};
    use crate::flight::weapons::{BEAM_UNDER_SHIPS, translucent};
    use crate::sound::CombatSound;

    const BLASTER: WeaponId = WeaponId(128);
    const ROCKET: WeaponId = WeaponId(140);
    const MISSILE: WeaponId = WeaponId(141);
    const TORCH: WeaponId = WeaponId(142);
    const UNDER: WeaponId = WeaponId(146);
    const OVER: WeaponId = WeaponId(147);
    const UNSEEN: WeaponId = WeaponId(150);

    /// `wëap` `id` firing every tick, 20 pixels a tick for 30 ticks, doing
    /// no damage, with `flags`, `guidance` and `explod_type`.
    fn gun(id: WeaponId, flags: u16, guidance: i16, explod_type: i16) -> nova_sim::WeaponRecord {
        nova_sim::WeaponRecord {
            id,
            reload: 0,
            count: 30,
            mass_dmg: 0,
            energy_dmg: 0,
            guidance,
            speed: 2000,
            ammo_type: -1,
            inaccuracy: 0,
            impact: 0,
            explod_type,
            prox_radius: 0,
            blast_radius: 0,
            flags,
            seeker: 0,
            flags2: 0,
            flags3: 0,
            decay: 0,
            beam_length: 50,
            burst_count: 0,
            burst_reload: 0,
            guided_turn: 0,
            durability: 0,
            sub_count: 0,
            sub_type: None,
            sub_theta: 0,
            sub_limit: 0,
        }
    }

    /// Ship `id` carrying one of each of `weapons`, gone at once in
    /// `bööm` 128.
    fn hull_of(id: i16, weapons: &[WeaponId]) -> nova_sim::HullRecord {
        nova_sim::HullRecord {
            id: ShipId(id),
            flags: 0,
            death_delay: 0,
            explode1: -1,
            explode2: 0,
            mass: 0,
            weapons: weapons
                .iter()
                .map(|&weapon| nova_sim::StockWeapon {
                    weapon,
                    count: 1,
                    ammo: 0,
                })
                .collect(),
            size: None,
            strength: 0,
        }
    }

    fn effect_sheet(image_id: i16, frames: u16) -> EffectSheet {
        EffectSheet {
            image_id,
            frames: Frames::new(frames).expect("non-zero"),
        }
    }

    /// A look named `name` sounding `snd ` `sound`, its shots on `rlëD`
    /// `image` of 36 frames, if any.
    fn look(name: &str, sound: i16, image: Option<i16>) -> WeaponLook {
        WeaponLook {
            name: name.to_owned(),
            sheet: image.map(|image| Ok(effect_sheet(image, 36))),
            sound: Some(nova_sim::SoundId(sound)),
            ..WeaponLook::default()
        }
    }

    /// [`trafficked`] (ships of type 129, warships) where the player's
    /// ship carries `weapons`; the blaster, rockets, missiles and torch
    /// have looks, and `bööm` 128 shows `rlëD` 400's 3 frames at a frame
    /// a step, sounding `snd ` 302. Ship 129 is a "Light Transport" with
    /// picture 3001, and govt 140's code is "Fed.".
    fn armed(avg: i16, weapons: &[WeaponId]) -> FakeCatalog {
        let mut catalog = trafficked(&[130], avg, 129, 3);
        catalog.dudes[0].1.govt = Some(GovtId(140));
        FakeCatalog {
            weapons: vec![
                gun(BLASTER, 0, -1, 0),
                gun(ROCKET, 0x0002, -1, -1),
                gun(MISSILE, 0x0002, -1, -1),
                gun(TORCH, 0x0002, -1, -1),
                gun(UNDER, 0, 0, -1),
                gun(OVER, 0, 0, -1),
                gun(UNSEEN, 0, -1, -1),
            ],
            hulls: vec![hull_of(128, weapons), hull_of(129, &[])],
            looks: vec![
                (128, Ok(look("Blaster", 208, Some(3500)))),
                (140, Ok(look("Rocket", 209, Some(3501)))),
                (141, Ok(look("Missile", 210, None))),
                (142, Ok(look("Torch", 211, None))),
                (
                    146,
                    Ok(WeaponLook {
                        flags2: BEAM_UNDER_SHIPS,
                        beam_width: 1,
                        beam_color: 0x0000_00FF,
                        ..WeaponLook::default()
                    }),
                ),
                (
                    147,
                    Ok(WeaponLook {
                        beam_width: 1,
                        beam_color: 0x0000_FF00,
                        ..WeaponLook::default()
                    }),
                ),
            ],
            booms: vec![(
                128,
                Ok(BoomLook {
                    sheet: Ok(effect_sheet(400, 3)),
                    advance: 1.0,
                    sound: Some(nova_sim::SoundId(302)),
                }),
            )],
            cards: vec![(
                129,
                TargetCard {
                    subtitle: "Light Transport".to_owned(),
                    picture: Some(3001),
                },
            )],
            codes: vec![(140, "Fed.".to_owned())],
            ..catalog
        }
    }

    /// `catalog`'s flight with its NPCs idling, placed by `draws`, after
    /// the step that populates the system.
    fn fighting(catalog: FakeCatalog, draws: &[u32]) -> View {
        let (_, chance) = scripted(draws);
        let mut view = FlightView::new(catalog)
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        view
    }

    /// NPC 0 at (100, -100) and NPC 1 at (-50, 0), nearer the player.
    fn two_npcs() -> Vec<u32> {
        [placed(850, 650, 90), placed(700, 750, 0)].concat()
    }

    fn target_of(view: &View) -> Option<NpcId> {
        view.session().expect("flying").target().map(|npc| npc.id)
    }

    #[test]
    fn tab_targets_the_next_npc_and_r_the_nearest() {
        let mut view = fighting(armed(2, &[]), &two_npcs());
        assert_eq!(target_of(&view), None);
        tap(&mut view, Key::Tab);
        assert_eq!(target_of(&view), Some(NpcId(0)));
        view.input(&held(Key::Tab));
        assert_eq!(target_of(&view), Some(NpcId(0)), "a repeat does nothing");
        tap(&mut view, Key::Tab);
        assert_eq!(target_of(&view), Some(NpcId(1)));
        tap(&mut view, Key::Tab);
        assert_eq!(target_of(&view), None, "past the last");
        tap(&mut view, NEAREST_KEY);
        assert_eq!(target_of(&view), Some(NpcId(1)));
        tap(&mut view, Key::Tab);
        view.input(&held(NEAREST_KEY));
        assert_eq!(target_of(&view), None, "a repeat does nothing");
        assert_eq!((TARGET_KEY, NEAREST_KEY), (Key::Tab, Key::Char('r')));
    }

    /// How many of the shots in flight are of `weapon`.
    fn shots_of(view: &View, weapon: WeaponId) -> usize {
        let session = view.session().expect("flying");
        session
            .shots()
            .iter()
            .filter(|shot| shot.weapon.id == weapon)
            .count()
    }

    #[test]
    fn space_fires_the_primaries_and_control_the_secondary_while_held() {
        let mut view = fighting(armed(0, &[BLASTER, ROCKET]), &[]);
        view.input(&key(FIRE_KEY, true));
        ticks(&mut view, 2);
        view.input(&held(FIRE_KEY));
        ticks(&mut view, 1);
        assert_eq!((shots_of(&view, BLASTER), shots_of(&view, ROCKET)), (3, 0));
        view.input(&key(FIRE_KEY, false));
        ticks(&mut view, 2);
        assert_eq!(shots_of(&view, BLASTER), 3, "let go");
        view.input(&key(SECONDARY_KEY, true));
        ticks(&mut view, 2);
        assert_eq!((shots_of(&view, BLASTER), shots_of(&view, ROCKET)), (3, 2));
        view.input(&key(SECONDARY_KEY, false));
        ticks(&mut view, 1);
        assert_eq!(shots_of(&view, ROCKET), 2, "let go");
        assert_eq!((FIRE_KEY, SECONDARY_KEY), (Key::Space, Key::Control));
    }

    #[test]
    fn letting_go_of_the_keys_lets_go_of_the_fire_keys() {
        let mut view = fighting(armed(0, &[BLASTER, ROCKET]), &[]);
        view.input(&key(FIRE_KEY, true));
        view.input(&key(SECONDARY_KEY, true));
        ticks(&mut view, 1);
        view.release_keys();
        ticks(&mut view, 2);
        assert_eq!((shots_of(&view, BLASTER), shots_of(&view, ROCKET)), (1, 1));
    }

    fn secondary_of(view: &View) -> Option<WeaponId> {
        view.session().expect("flying").secondary()
    }

    #[test]
    fn w_selects_the_next_secondary_and_alt_w_the_one_before() {
        let mut view = fighting(armed(0, &[BLASTER, ROCKET, MISSILE, TORCH]), &[]);
        assert_eq!(secondary_of(&view), Some(ROCKET));
        tap(&mut view, SELECT_KEY);
        assert_eq!(secondary_of(&view), Some(MISSILE));
        view.input(&held(SELECT_KEY));
        assert_eq!(secondary_of(&view), Some(MISSILE), "a repeat does nothing");
        view.input(&key(Key::Alt, true));
        tap(&mut view, SELECT_KEY);
        assert_eq!(secondary_of(&view), Some(ROCKET));
        tap(&mut view, SELECT_KEY);
        assert_eq!(secondary_of(&view), Some(TORCH), "wrapping");
        view.input(&key(Key::Alt, false));
        tap(&mut view, SELECT_KEY);
        assert_eq!(secondary_of(&view), Some(ROCKET));
        view.input(&key(Key::Alt, true));
        view.release_keys();
        tap(&mut view, SELECT_KEY);
        assert_eq!(secondary_of(&view), Some(MISSILE), "Alt let go");
        assert_eq!(SELECT_KEY, Key::Char('w'));
    }

    #[test]
    fn the_combat_keys_do_nothing_while_the_map_is_open() {
        let mut view = fighting(armed(2, &[BLASTER, ROCKET, MISSILE]), &two_npcs());
        tap(&mut view, MAP);
        for k in [Key::Tab, NEAREST_KEY, SELECT_KEY, FIRE_KEY, SECONDARY_KEY] {
            view.input(&key(k, true));
        }
        tap(&mut view, MAP);
        ticks(&mut view, 2);
        assert_eq!(target_of(&view), None);
        assert_eq!(secondary_of(&view), Some(ROCKET));
        assert_eq!(view.session().expect("flying").shots(), []);
    }

    #[test]
    fn the_combat_keys_do_nothing_while_a_jump_plays() {
        let mut view = fighting(armed(0, &[BLASTER, ROCKET, MISSILE]), &[]);
        plot(&mut view, 131);
        fly_out(&mut view);
        view.input(&key(JUMP, true));
        for k in [NEAREST_KEY, SELECT_KEY, FIRE_KEY] {
            view.input(&key(k, true));
        }
        view.tick(ms(2000));
        assert_eq!(view.jump_effect(), None);
        ticks(&mut view, 2);
        assert_eq!(secondary_of(&view), Some(ROCKET));
        assert_eq!(view.session().expect("flying").shots(), []);
    }

    /// The combat sounds taken.
    fn combat_sounds(view: &mut View) -> Vec<CombatSound> {
        view.take_sounds()
            .into_iter()
            .filter_map(|sound| match sound {
                Sound::Combat(sound) => Some(sound),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn every_steps_fight_reaches_the_effects() {
        let mut view = fighting(armed(0, &[BLASTER]), &[]);
        view.input(&key(FIRE_KEY, true));
        view.tick(TICK * 3);
        assert_eq!(shots_of(&view, BLASTER), 3, "three steps");
        let fired = CombatSound {
            sound: nova_sim::SoundId(208),
            offset: (0, 0),
        };
        assert_eq!(combat_sounds(&mut view), [fired; 3]);
        assert_eq!(combat_sounds(&mut view), [], "taken");
    }

    #[test]
    fn the_sounds_are_the_sessions_then_the_fights() {
        let mut view = fighting(armed(0, &[BLASTER]), &[]);
        view.input(&key(Key::Up, true));
        view.input(&key(FIRE_KEY, true));
        view.tick(TICK);
        assert_eq!(
            view.take_sounds(),
            [
                Sound::Sim(SimSound::ThrustStarted),
                Sound::Combat(CombatSound {
                    sound: nova_sim::SoundId(208),
                    offset: (0, 0),
                })
            ]
        );
    }

    /// The first command matching `wanted`'s position in `list`.
    fn first(list: &DrawList, wanted: impl Fn(&DrawCommand) -> bool) -> usize {
        list.iter()
            .position(wanted)
            .unwrap_or_else(|| panic!("not drawn: {list:?}"))
    }

    fn sprite_of(id: i16) -> impl Fn(&DrawCommand) -> bool {
        move |command| matches!(command, DrawCommand::Sprite { image, .. } if image.id == id)
    }

    fn line_in(color: Color) -> impl Fn(&DrawCommand) -> bool {
        move |command| matches!(command, DrawCommand::Line { color: c, .. } if *c == color)
    }

    /// The flight with an NPC 100 pixels above the player, the player
    /// firing its blaster (which explodes where it hits) and two beams, one
    /// drawn under the ships and one over them, at it.
    fn firing_at_an_npc() -> View {
        let catalog = armed(1, &[BLASTER, UNDER, OVER]);
        let mut view = fighting(catalog, &placed(750, 650, 180));
        tap(&mut view, Key::Tab);
        view.input(&key(FIRE_KEY, true));
        ticks(&mut view, 5);
        view
    }

    #[test]
    fn the_fight_is_drawn_between_the_stellars_and_the_hud_in_order() {
        let view = firing_at_an_npc();
        let list = drawn(&view);
        let stellar = first(&list, sprite_of(1128));
        let under = first(&list, line_in(Color::rgba(0, 0, 255, 255)));
        let npc = first(&list, sprite_of(2001));
        let ship = first(&list, sprite_of(2000));
        let shot = first(&list, sprite_of(3500));
        let over = first(&list, line_in(Color::rgba(0, 255, 0, 255)));
        let explosion = first(&list, sprite_of(400));
        let brackets = first(&list, line_in(BRACKETS));
        let title = first(
            &list,
            |c| matches!(c, DrawCommand::Text { origin, .. } if *origin == TITLE),
        );
        let hud = hud_at(&list);
        let order = [
            stellar, under, npc, ship, shot, over, explosion, brackets, title, hud,
        ];
        assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "{order:?}");
        let DrawCommand::Sprite { tint, .. } = list.iter().nth(explosion).cloned().expect("drawn")
        else {
            panic!("a sprite")
        };
        assert_eq!(tint, translucent());
    }

    #[test]
    fn a_shot_is_drawn_with_its_frame_where_it_flies() {
        let mut view = fighting(armed(0, &[BLASTER]), &[]);
        // Turned 45 degrees right, so the shot flies across and up.
        view.input(&key(Key::Right, true));
        ticks(&mut view, 15);
        view.input(&key(Key::Right, false));
        view.input(&key(FIRE_KEY, true));
        view.tick(TICK);
        view.input(&key(FIRE_KEY, false));
        view.tick(TICK / 2);
        let session = view.session().expect("flying");
        let shot = session.shots()[0];
        let alpha = view.alpha();
        let shown = at(
            shot.position.x - shot.velocity.x * (1.0 - alpha),
            shot.position.y - shot.velocity.y * (1.0 - alpha),
        );
        let list = drawn(&view);
        let drawn_at = sprites(&list)
            .into_iter()
            .find(|(image, _)| image.id == 3500)
            .expect("the shot");
        assert_eq!(drawn_at.0, ImageKey::sprite(3500, 4), "heading 45");
        let expected = view.camera().world_to_screen(shown);
        assert!(
            (drawn_at.1.x - expected.x).abs() < 1e-3 && (drawn_at.1.y - expected.y).abs() < 1e-3,
            "{drawn_at:?} {expected:?}"
        );
    }

    /// Every NPC heads for planet 128 and holds its trigger.
    #[derive(Debug)]
    struct LandingFiring;

    impl Behaviour for LandingFiring {
        fn decide(
            &self,
            _npc: &nova_sim::Npc,
            _around: &nova_sim::Surroundings,
            _chance: &mut dyn Chance,
        ) -> Goal {
            Goal::Land(StellarId(128))
        }

        fn trigger(
            &self,
            _npc: &nova_sim::Npc,
            _around: &nova_sim::Surroundings,
        ) -> nova_sim::Trigger {
            nova_sim::Trigger {
                primary: true,
                secondary: None,
            }
        }
    }

    /// Where the beams in `color` start, on screen: the lines a pixel wide,
    /// not the HUD's bars.
    fn beam_starts(list: &DrawList, color: Color) -> Vec<Point> {
        list.iter()
            .filter_map(|command| match *command {
                DrawCommand::Line {
                    from,
                    color: c,
                    width,
                    ..
                } if c == color && width == 1.0 => Some(from),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_beam_is_drawn_from_its_firer_where_the_firer_is_drawn() {
        let mut catalog = armed(1, &[OVER]);
        catalog.hulls[1] = hull_of(129, &[UNDER]);
        let (_, chance) = scripted(&placed(850, 650, 0));
        let mut view = FlightView::new(catalog)
            .with_chance(chance)
            .with_behaviour(Rc::new(LandingFiring));
        view.tick(TICK);
        view.input(&key(Key::Up, true));
        view.input(&key(FIRE_KEY, true));
        ticks(&mut view, 5);
        view.tick(TICK / 2);
        let session = view.session().expect("flying");
        let npc = &session.npcs()[0];
        assert_ne!(
            view.npc_previous[&npc.id].position, npc.state.position,
            "moving"
        );
        assert_ne!(view.previous.position, session.player().position, "moving");
        let list = drawn(&view);
        let camera = view.camera();
        let player = camera.world_to_screen(view.shown_position());
        let over = beam_starts(&list, Color::rgba(0, 255, 0, 255));
        assert!(!over.is_empty());
        assert!(
            over.iter().all(|&start| start == player),
            "the player's, from the player as drawn: {over:?}"
        );
        let (npc_shown, _) = view.shown_npc(npc);
        let under = beam_starts(&list, Color::rgba(0, 0, 255, 255));
        assert!(!under.is_empty());
        let expected = camera.world_to_screen(npc_shown);
        assert!(
            under
                .iter()
                .all(|start| (start.x - expected.x).abs() < 1e-3
                    && (start.y - expected.y).abs() < 1e-3),
            "the NPC's, from the NPC as drawn: {under:?} {expected:?}"
        );
    }

    #[test]
    fn a_ship_breaking_up_goes_off_about_where_it_is() {
        let mut catalog = armed(1, &[BLASTER]);
        catalog.weapons[0].mass_dmg = 1000;
        catalog.weapons[0].flags = 0x0020;
        catalog.weapons[0].explod_type = -1;
        catalog.hulls[1].death_delay = 30;
        catalog.hulls[1].explode1 = 0;
        let mut view = fighting(catalog, &placed(750, 650, 180));
        view.input(&key(FIRE_KEY, true));
        view.tick(TICK);
        view.input(&key(FIRE_KEY, false));
        let mut breaking = None;
        for _ in 0..40 {
            view.tick(TICK);
            let session = view.session().expect("flying");
            if let Some(npc) = session.npcs().first()
                && let Condition::Dying { ticks_left } = npc.condition
                && ticks_left < 19
                && !view.effects().explosions().is_empty()
            {
                breaking = Some(npc.state.position);
                break;
            }
        }
        let at = breaking.expect("an explosion as it breaks up");
        // Never firing, the effects' chance draws the last outcome: a
        // quarter of its 40-pixel sprite less one out, each way.
        assert_eq!(
            view.effects().explosions()[0].at,
            Point::new(at.x + 9.0, at.y + 9.0)
        );
    }

    #[test]
    fn the_players_ship_breaking_up_goes_off_about_where_it_is() {
        // The NPC, straight above the player and facing it, fires a
        // blaster that kills at a hit; the player's ship, 80 pixels wide
        // (the NPC's 40), takes 30 ticks to break up.
        let mut catalog = armed(1, &[]);
        catalog.sheet = Ok(ShipSheet {
            frame_width: 80,
            ..sheet()
        });
        catalog.weapons[0].mass_dmg = 1000;
        catalog.weapons[0].flags = 0x0020;
        catalog.weapons[0].explod_type = -1;
        catalog.hulls[0].death_delay = 30;
        catalog.hulls[0].explode1 = 0;
        catalog.hulls[1] = hull_of(129, &[BLASTER]);
        let mut view = fighting(catalog, &placed(750, 650, 180)).with_behaviour(Rc::new(Firing));
        let mut breaking = None;
        for _ in 0..40 {
            view.tick(TICK);
            let session = view.session().expect("flying");
            if let Condition::Dying { ticks_left } = session.player_condition()
                && ticks_left < 19
                && !view.effects().explosions().is_empty()
            {
                breaking = Some(session.player().position);
                break;
            }
        }
        let at = breaking.expect("an explosion as the player's ship breaks up");
        // Never firing, the effects' chance draws the last outcome: a
        // quarter of the player's 80-pixel sprite less one out, each way.
        assert_eq!(
            view.effects().explosions()[0].at,
            Point::new(at.x + 19.0, at.y + 19.0)
        );
    }

    #[test]
    fn a_shot_whose_look_cannot_be_read_is_a_crossed_box() {
        let mut view = fighting(armed(0, &[UNSEEN]), &[]);
        view.input(&key(FIRE_KEY, true));
        view.tick(TICK);
        let list = drawn(&view);
        let shot = view.session().expect("flying").shots()[0];
        let mut boxed = DrawList::new();
        // A whole step on, it is drawn a step behind, where it left.
        assert_eq!(view.alpha(), 0.0);
        let left = shot.position - shot.velocity;
        let centre = view.camera().world_to_screen(at(left.x, left.y));
        crossed_box(
            &mut boxed,
            centre,
            weapons::SHOT_PLACEHOLDER_SIZE,
            PLACEHOLDER,
        );
        let commands: Vec<_> = list.iter().cloned().collect();
        let expected: Vec<_> = boxed.iter().cloned().collect();
        assert!(
            commands.windows(expected.len()).any(|run| run == expected),
            "{commands:?}"
        );
    }

    #[test]
    fn a_destroyed_npc_explodes_and_scatters_debris_on_the_frames_after() {
        let mut catalog = armed(1, &[BLASTER]);
        catalog.weapons[0].mass_dmg = 1000;
        catalog.weapons[0].flags = 0x0020;
        catalog.weapons[0].explod_type = -1;
        let mut view = fighting(catalog, &placed(750, 650, 180));
        tap(&mut view, Key::Tab);
        // Drifting up, so the explosion is heard from where the player is.
        view.input(&key(Key::Up, true));
        view.tick(TICK);
        view.input(&key(Key::Up, false));
        view.input(&key(FIRE_KEY, true));
        let mut destroyed = false;
        for _ in 0..10 {
            view.tick(TICK);
            if view.session().expect("flying").npcs().is_empty() {
                destroyed = true;
                break;
            }
        }
        assert!(destroyed, "{:?}", view.session().expect("flying").npcs());
        assert_eq!(target_of(&view), None);
        let effects = view.effects();
        assert_eq!(effects.explosions().len(), 1);
        assert_eq!(effects.explosions()[0].boom, BoomId(128));
        assert_eq!(effects.debris().len(), 16);
        let list = drawn(&view);
        let explosion = first(&list, sprite_of(400));
        assert_eq!(
            list.iter()
                .skip(explosion)
                .filter(|c| matches!(c, DrawCommand::Dot { color, .. } if *color == DEBRIS_COLOR))
                .count(),
            16,
            "drawn after the explosion"
        );
        let player = view.session().expect("flying").player().position;
        assert!(player.y < 0.0, "{player:?}");
        let boom = view.effects().explosions()[0].at;
        let heard = CombatSound {
            sound: nova_sim::SoundId(302),
            offset: (
                (boom.x - player.x).round() as i32,
                (boom.y - player.y).round() as i32,
            ),
        };
        assert!(
            combat_sounds(&mut view).contains(&heard),
            "the explosion is heard: {heard:?}"
        );
    }

    /// [`armed`] with a blaster whose shots detonate after a tick.
    fn detonating() -> FakeCatalog {
        let mut catalog = armed(0, &[BLASTER]);
        catalog.weapons[0].flags = 0x8000;
        catalog.weapons[0].count = 1;
        catalog
    }

    /// Fires one detonating shot, which explodes.
    fn detonate(view: &mut View) {
        view.input(&key(FIRE_KEY, true));
        view.tick(TICK);
        view.input(&key(FIRE_KEY, false));
        assert_eq!(view.effects().explosions().len(), 1);
    }

    #[test]
    fn landing_clears_the_effects() {
        let mut catalog = detonating();
        catalog.sites = vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)];
        let mut view = fighting(catalog, &[]);
        detonate(&mut view);
        tap(&mut view, LAND_KEY);
        assert!(view.take_landing().is_some());
        assert_eq!(*view.effects(), Effects::default());
    }

    #[test]
    fn arriving_clears_the_effects() {
        let mut view = fighting(detonating(), &[]);
        plot(&mut view, 131);
        fly_out(&mut view);
        detonate(&mut view);
        view.input(&key(JUMP, true));
        view.tick(ms(1000));
        assert_eq!(view.effects().explosions().len(), 1, "frozen in the jump");
        view.tick(ms(1000));
        assert_eq!(view.session().expect("flying").system(), SystemId(131));
        assert_eq!(*view.effects(), Effects::default());
    }

    #[test]
    fn the_target_is_bracketed_and_its_panel_shows_it() {
        let view = firing_at_an_npc();
        let session = view.session().expect("flying");
        let npc = &session.npcs()[0];
        let list = drawn(&view);
        let (shown, _) = view.shown_npc(npc);
        let mut brackets = DrawList::new();
        target::draw_brackets(
            &mut brackets,
            view.camera().world_to_screen(shown),
            40.0,
            false,
        );
        let commands: Vec<_> = list.iter().cloned().collect();
        let expected: Vec<_> = brackets.iter().cloned().collect();
        assert!(
            commands.windows(8).any(|run| run == expected),
            "{commands:?}"
        );
        let bar = view.status_bar().expect("a status bar");
        let card = TargetCard {
            subtitle: "Light Transport".to_owned(),
            picture: Some(3001),
        };
        let mut panel = DrawList::new();
        target::draw_target_panel(
            &mut panel,
            &bar.layout,
            hud::bar_origin(bar),
            Some(&target::TargetShown {
                name: "Ship 129",
                card: &card,
                code: Some("Fed."),
                reserves: npc.reserves,
                disabled: false,
            }),
            None,
        );
        let expected: Vec<_> = panel.iter().cloned().collect();
        assert!(
            commands.windows(expected.len()).any(|run| run == expected),
            "{commands:?}"
        );
    }

    #[test]
    fn a_disabled_targets_brackets_are_grey() {
        let catalog = armed(1, &[]);
        let mut view =
            fighting(catalog, &placed(750, 650, 180)).with_disable_rule(Rc::new(Disabling));
        tap(&mut view, Key::Tab);
        view.tick(TICK);
        let session = view.session().expect("flying");
        assert_eq!(session.npcs()[0].condition, Condition::Disabled);
        let list = drawn(&view);
        assert!(list.iter().any(line_in(DISABLED_BRACKETS)));
        assert!(!list.iter().any(line_in(BRACKETS)));
        assert!(texts(&list).contains(&target::DISABLED.to_owned()));
    }

    /// Disables every ship.
    #[derive(Debug)]
    struct Disabling;

    impl DisableRule for Disabling {
        fn disabled(&self, _armor: nova_sim::Gauge, _hull: &nova_sim::HullSpec) -> bool {
            true
        }
    }

    #[test]
    fn the_hud_shows_no_target_and_the_secondary_weapon() {
        let view = fighting(armed(0, &[BLASTER, ROCKET]), &[]);
        let list = texts(&drawn(&view));
        assert!(list.contains(&NO_TARGET.to_owned()), "{list:?}");
        assert!(list.contains(&"Rocket".to_owned()), "{list:?}");
        let unarmed = texts(&drawn(&fighting(armed(0, &[BLASTER]), &[])));
        assert!(unarmed.contains(&NO_SECONDARY.to_owned()), "{unarmed:?}");
    }

    #[test]
    fn the_panel_is_laid_out_by_the_metrics_given() {
        let view = fighting(armed(0, &[BLASTER]), &[])
            .with_metrics(Rc::new(crate::text::fixture::MonoMetrics));
        let list = drawn(&view);
        let origin = list
            .iter()
            .find_map(|c| match c {
                DrawCommand::Text { text, origin, .. } if text == NO_TARGET => Some(*origin),
                _ => None,
            })
            .expect("No Target");
        // Centred: 9 characters of 6 in the 176 across from 838.
        assert_eq!(origin.x, 899.0);
    }

    #[test]
    fn the_effects_roll_on_their_own_chance() {
        let mut catalog = armed(1, &[BLASTER]);
        catalog.weapons[0].mass_dmg = 1000;
        catalog.weapons[0].flags = 0x0020;
        catalog.weapons[0].explod_type = -1;
        let (_, chance) = scripted(&placed(750, 650, 180));
        let (effects, effects_chance) = scripted(&[]);
        let mut view = FlightView::new(catalog)
            .with_chance(chance)
            .with_behaviour(Rc::new(Still))
            .with_effects_chance(effects_chance);
        view.tick(TICK);
        view.input(&key(FIRE_KEY, true));
        ticks(&mut view, 10);
        assert!(view.session().expect("flying").npcs().is_empty());
        assert_eq!(
            effects.borrow().asked,
            [360, 101, 31].repeat(16),
            "the debris, drawn on the effects' own chance"
        );
    }

    #[test]
    fn a_looped_weapon_is_heard_again_only_once_its_sound_has_played_out() {
        let mut catalog = armed(0, &[UNDER]);
        catalog.weapons[4].reload = 0;
        catalog.weapons[4].count = 5;
        catalog.looks[4].1 = Ok(WeaponLook {
            flags: weapons::LOOPED_SOUND,
            sound: Some(nova_sim::SoundId(220)),
            sound_ticks: Some(3),
            ..WeaponLook::default()
        });
        let mut view = fighting(catalog, &[]);
        view.input(&key(FIRE_KEY, true));
        ticks(&mut view, 4);
        assert_eq!(
            view.session().expect("flying").beams().len(),
            4,
            "a beam fired each step"
        );
        assert_eq!(combat_sounds(&mut view).len(), 2, "on the first and fourth");
        view.input(&key(FIRE_KEY, false));
        ticks(&mut view, 1);
        view.input(&key(FIRE_KEY, true));
        ticks(&mut view, 1);
        assert_eq!(combat_sounds(&mut view), [], "still playing");
        view.input(&key(FIRE_KEY, false));
        ticks(&mut view, 3);
        view.input(&key(FIRE_KEY, true));
        ticks(&mut view, 1);
        assert_eq!(combat_sounds(&mut view).len(), 1, "played out");
    }

    #[test]
    fn the_metrics_debug_as_their_name() {
        let metrics = Metrics(Rc::new(crate::text::fixture::MonoMetrics));
        assert_eq!(format!("{metrics:?}"), "Metrics");
    }

    #[test]
    fn the_help_line_names_the_combat_keys() {
        for name in ["Space", "Ctrl", "W", "Tab", "R:"] {
            assert!(HELP.contains(name), "{name}: {HELP}");
        }
    }

    // Point defence.

    /// Says no missile is hostile, counting the times it is asked.
    #[derive(Debug, Default)]
    struct Unalarmed {
        asked: std::cell::Cell<usize>,
    }

    impl nova_sim::PointDefenceRule for Unalarmed {
        fn hostile(
            &self,
            _defender: nova_sim::combat::defence::Side,
            _firer: nova_sim::combat::defence::Side,
            _govts: &nova_sim::Governments,
        ) -> bool {
            self.asked.set(self.asked.get() + 1);
            false
        }
    }

    /// Every NPC idles, targets the player and holds its trigger.
    #[derive(Debug)]
    struct Attacking;

    impl Behaviour for Attacking {
        fn decide(
            &self,
            _npc: &nova_sim::Npc,
            _around: &nova_sim::Surroundings,
            _chance: &mut dyn Chance,
        ) -> Goal {
            Goal::Idle
        }

        fn trigger(
            &self,
            _npc: &nova_sim::Npc,
            _around: &nova_sim::Surroundings,
        ) -> nova_sim::Trigger {
            nova_sim::Trigger {
                primary: true,
                secondary: None,
            }
        }

        fn target(
            &self,
            _npc: &nova_sim::Npc,
            _around: &nova_sim::Surroundings,
        ) -> Option<ShipRef> {
            Some(ShipRef::Player)
        }
    }

    const QUAD: WeaponId = WeaponId(133);
    const IR: WeaponId = WeaponId(134);

    /// [`armed`] with the player, over a planet at the centre, carrying a
    /// point-defence turret, and its traffic a missile it fires once, 5
    /// pixels a tick.
    fn defending() -> FakeCatalog {
        let mut catalog = armed(1, &[]);
        catalog.sites = vec![site(140, (0.0, 0.0), StellarFlags::CAN_LAND)];
        catalog.weapons.push(nova_sim::WeaponRecord {
            reload: 5,
            count: 12,
            mass_dmg: 1,
            energy_dmg: 4,
            ..gun(QUAD, 0, 9, -1)
        });
        catalog.weapons.push(nova_sim::WeaponRecord {
            reload: 1000,
            count: 200,
            speed: 500,
            ..gun(IR, 0, 1, -1)
        });
        catalog.hulls = vec![hull_of(128, &[QUAD]), hull_of(129, &[IR])];
        catalog
    }

    /// `catalog`'s flight with its NPC attacking from 200 pixels above
    /// the player, after the step that populates the system.
    fn attacked(catalog: FakeCatalog) -> View {
        let (_, chance) = scripted(&placed(750, 550, 180));
        let mut view = FlightView::new(catalog)
            .with_chance(chance)
            .with_behaviour(Rc::new(Attacking));
        view.tick(TICK);
        view
    }

    #[test]
    fn the_fight_asks_the_point_defence_rule_given_each_step_in_flight() {
        let rule = Rc::new(Unalarmed::default());
        let mut view = attacked(defending()).with_point_defence_rule(rule.clone());
        view.tick(TICK);
        assert_eq!(shots_of(&view, IR), 1, "fired at the player");
        assert_eq!(rule.asked.get(), 1, "of the one missile");
        ticks(&mut view, 2);
        assert_eq!(rule.asked.get(), 3);
        assert_eq!(shots_of(&view, QUAD), 0, "never hostile");
        view.input(&key(MAP_KEY, true));
        ticks(&mut view, 2);
        assert_eq!(rule.asked.get(), 3, "not while the map is open");
        view.input(&key(MAP_KEY, true));
        view.input(&key(LAND_KEY, true));
        assert!(view.take_landing().is_some());
        ticks(&mut view, 2);
        assert_eq!(rule.asked.get(), 3, "not while landed");
    }

    #[test]
    fn a_missile_shot_down_is_drawn_exploding_and_unheard() {
        let mut view = attacked(defending());
        let mut sounds = Vec::new();
        let mut exploded = false;
        for _ in 0..12 {
            view.tick(TICK);
            sounds.extend(combat_sounds(&mut view));
            let explosion = drawn(&view).iter().any(
                |command| matches!(command, DrawCommand::Sprite { image, .. } if image.id == 400),
            );
            if explosion {
                exploded = true;
                break;
            }
        }
        assert!(exploded, "bööm 128 drawn");
        assert_eq!(shots_of(&view, IR), 0, "shot down");
        assert!(
            sounds
                .iter()
                .all(|sound| sound.sound != nova_sim::SoundId(302)),
            "{sounds:?}"
        );
    }
}
