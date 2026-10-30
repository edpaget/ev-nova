//! The values the flight view's `FlightSim` port (`nova_view`'s
//! `flight::sim_port`) carries across the seam between the view and the
//! session: what a frame shows ([`FlightFrame`]), what the player asks
//! ([`FlightCommand`]) and what the session answers ([`Answer`]), what it
//! raised on the way ([`FlightEvent`]), and a frame's fixed steps' input and
//! report ([`StepInput`], [`StepReport`], its [`StepRun`]s and
//! [`PassReport`]s), and what a screen asks as it opens or its input
//! changes ([`ScreenAsk`]) with the snapshot it draws from
//! ([`ScreenAnswer`], the `screen` module).
//!
//! They live here, not in the view, so the session can build them:
//! [`Session::fill_frame`](crate::Session::fill_frame) fills a frame,
//! [`Session::apply`](crate::Session::apply) answers a command and
//! [`Session::screen`](crate::Session::screen) a screen's ask. Each
//! starts with what the first slice moved across the seam needs, the nav
//! mode and the stellar keys, and grows as later slices move more.

use std::collections::{BTreeMap, BTreeSet};

mod port_screens;
mod screen;

pub use port_screens::{BarNews, PortShops};
pub use screen::{FloatingMap, GateEntry, MapOpen, MapScreen, MarksFrom, ScreenAnswer, ScreenAsk};

use crate::asteroid::Asteroid;
use crate::bay::FighterNote;
use crate::board::{BoardRefusal, Boarding, MissionBoardLine, Take, Taken};
use crate::brackets::{NavBrackets, TargetBrackets};
use crate::cargo_box::CargoBox;
use crate::catalog::{
    DescMovie, Explosion, GovtId, JunkId, OutfitId, ShipId, StellarId, SystemId, WeaponId,
};
use crate::cloak::{Area, CloakShown};
use crate::combat::beam::Beam;
use crate::combat::hull::Condition;
use crate::combat::ion_tint::IonTint;
use crate::combat::projectile::{Shot, ShotId};
use crate::combat::report::SimDiagnostic;
use crate::combat::ship_frames::{Drawn, FrameSpec};
use crate::combat::shot_layer::ShotLayer;
use crate::combat::{CombatEvent, ShipRef};
use crate::escort::{ClassRow, Commanded, EscortClass, EscortCommand, EscortGroup};
use crate::escort_dialog::{EscortButton, EscortDialogView};
use crate::flight::{Controls, ShipState};
use crate::gate::{GateKind, GateRefusal};
use crate::geometry::Vec2;
use crate::grant::Granted;
use crate::hail::{CommNote, Haggle, HailRefusal, HailView};
use crate::hire::PayNote;
use crate::hover::{DrawnBox, DrawnShips, Hover};
use crate::hyperspace::{JumpReadiness, JumpRefusal};
use crate::iff::{RadarGear, ShipIff, StellarIff};
use crate::landing::LandingRefusal;
use crate::market::Good;
use crate::message::SimMessage;
use crate::navigation::NavMode;
use crate::player_fate::{Drawn as ShipDrawn, PlayerCommand, PlayerFate};
use crate::player_info::PlayerInfo;
use crate::red_alert::AlertIcon;
use crate::reserves::{Gauge, Reserves};
use crate::session::{
    Departure, Hailed, HoverPlace, InterfaceLoad, LandPress, MapClicked, MapPress, MissionShown,
    PersonQuote, SavedNav, SessionEvent, Settled, StellarPick, TextBar,
};
use crate::ship_layer::ShipLayer;
use crate::sound::SimSound;
use crate::status_cargo::StatusCargo;
use crate::story::DrawnSheet;
use crate::target_panel::{TargetStatus, TargetTag};
use crate::targeting::TargetPick;
use crate::traffic::npc::Mode;
use crate::traffic::npc::NpcId;
use crate::{
    AbortError, AcceptError, Assigned, Assignment, ComputerRefusal, ComputerRow, HireRefusal,
    Hired, MissionComputer, MissionId, MissionInfoPress, NotOffered, Order, OutfitOrder, OutfitRun,
    RechargeRefusal, RefuseShown, ShipPurchase, ShipRefusal, TradeRefusal, VenueOffer,
};

/// What the flight view draws from, filled by
/// [`Session::fill_frame`](crate::Session::fill_frame): every field is
/// written on each fill, so a frame kept from one fill to the next shows
/// nothing stale. The default is what a session that failed to start
/// shows: nav off, no course, in flight, no jump.
// Each flag is one fact of the flight, filled on its own.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, PartialEq)]
pub struct FlightFrame {
    /// The player's nav mode and its selection
    /// ([`Session::nav_mode`](crate::Session::nav_mode)).
    pub nav_mode: NavMode,
    /// The system the player is in
    /// ([`Session::system`](crate::Session::system)).
    pub system: SystemId,
    /// The course as it is flown
    /// ([`Session::course_shown`](crate::Session::course_shown)).
    pub course_shown: Vec<SystemId>,
    /// Whether a course is plotted, flown or not
    /// ([`Session::course`](crate::Session::course) has a hop).
    pub course_plotted: bool,
    /// The stellar the player is docked at, if any
    /// ([`Session::landed`](crate::Session::landed)); [`FlightFrame::docked`]
    /// says whether it is.
    pub landed: Option<StellarId>,
    /// Whether a jump is under way, the pre-jump stage included: the
    /// original's +0x50 > 0, which its key blocks test live
    /// ([`Session::jumping`](crate::Session::jumping) or
    /// [`Session::preparing_jump`](crate::Session::preparing_jump)).
    pub jump_under_way: bool,
    /// Whether the jump itself has begun, past the pre-jump stage, and the
    /// ship is not yet arrived
    /// ([`Session::jumping`](crate::Session::jumping) is some).
    pub warping: bool,
    /// Whether the warp has ended, so the jump arrives
    /// ([`Session::warp_end_due`](crate::Session::warp_end_due)).
    pub warp_end_due: bool,
    /// Where the player's ship is in its lifecycle
    /// ([`Session::player_fate`](crate::Session::player_fate)).
    pub fate: PlayerFate,
    /// Whether the hold after the player's explosion has run out, now or
    /// before: flight steps no more
    /// ([`Session::hold_expired`](crate::Session::hold_expired)).
    pub hold_expired: bool,
    /// Whether a `Q` run while landed waits for the take-off
    /// ([`Session::leave_due`](crate::Session::leave_due)).
    pub leave_due: bool,
    /// Whether the spaceport's Recharge button is enabled
    /// ([`Session::recharge_enabled`](crate::Session::recharge_enabled)):
    /// a router's fact, read after every spaceport input and as the
    /// spaceport opens, which any command may change. False in flight,
    /// and for a flight that failed to start.
    pub recharge_enabled: bool,
    /// Whether the pilot flying has seen the intro screen
    /// ([`Session::seen_intro`](crate::Session::seen_intro)): a router's
    /// fact, read as the game held is entered from the main menu. False
    /// for a flight that failed to start.
    pub seen_intro: bool,
    /// Whether the ship can jump to the system selected, as the nav area
    /// draws it ([`Session::jump_readiness`](crate::Session::jump_readiness)).
    pub jump_readiness: JumpReadiness,
    /// How many mission texts the session has queued, never reset
    /// ([`Session::texts_queued`](crate::Session::texts_queued)).
    pub texts_queued: u64,
    /// The player's commands the session takes now
    /// ([`Session::takes`](crate::Session::takes)): what the view asks
    /// before it acts on a key, mid-input, reading the frame the last
    /// command or step left.
    pub gates: Gates,
    /// What the course map's drawing is taken from ([`MapKey`]).
    pub map_key: MapKey,
    /// Each ship in the system as the world draws it: the player's first,
    /// then each NPC's in the system's order
    /// ([`Session::npcs`](crate::Session::npcs)).
    pub ships: Vec<FrameShip>,
    /// The ships drawn, in draw order, each with its layer
    /// (`Session::draw_layers`).
    pub drawn: Vec<(ShipRef, ShipLayer)>,
    /// Each shot in flight, in the order it is drawn
    /// (`Session::shots_drawn`).
    pub shots: Vec<FrameShot>,
    /// Each beam being fired, in the fight's order
    /// ([`Session::beams`](crate::Session::beams)).
    pub beams: Vec<FrameBeam>,
    /// Each cargo box in space, in its table's order
    /// ([`Session::cargo_boxes`](crate::Session::cargo_boxes)).
    pub cargo_boxes: Vec<FrameBox>,
    /// Each asteroid in the field, in the field's order
    /// ([`Session::asteroids`](crate::Session::asteroids)).
    pub asteroids: Vec<FrameAsteroid>,
    /// Each stellar the system lists, in its nav order, shown or not, and
    /// how it is drawn ([`FrameStellar`]).
    pub stellars: Vec<FrameStellar>,
    /// The player's effective murk, which the world is murked by
    /// ([`Session::effective_murk`](crate::Session::effective_murk)).
    pub effective_murk: u8,
    /// The system's murk as loaded
    /// ([`Session::system_murk`](crate::Session::system_murk)).
    pub system_murk: i16,
    /// The system's `BkgndColor`, raw `00RRGGBB`
    /// ([`Session::system_background`](crate::Session::system_background)).
    pub system_background: Option<u32>,
    /// Whether double time is on
    /// ([`Session::double_time`](crate::Session::double_time)).
    pub double_time: bool,
    /// The jump's `_hyperGamma`
    /// ([`Session::hyper_gamma`](crate::Session::hyper_gamma)).
    pub hyper_gamma: f32,
    /// The ship types the system's traffic can bring, whose sheets the
    /// view reads ([`Session::traffic_ships`](crate::Session::traffic_ships)).
    pub traffic_ships: Vec<ShipId>,
    /// What the status bar and the target layer show of the player's
    /// ship, its gear and its targets ([`FrameHud`]).
    pub hud: FrameHud,
    /// The ship target as the target panel shows it, if there is one
    /// ([`FrameTarget`]).
    pub target: Option<FrameTarget>,
    /// The `dësc` IDs a sabotaged player sees as it dies, none unsabotaged
    /// ([`Session::sabotage_death_descs`](crate::Session::sabotage_death_descs)):
    /// what the router reads once the hold after the explosion has run
    /// out. They change with the outfits owned and the sabotage flag the
    /// last refresh cached, so they are a frame's, not the opening's.
    pub death_descs: Vec<i16>,
    /// The name of the player's ship, if it has one
    /// ([`Pilot::ship_name`](crate::Pilot::ship_name)), which the messages
    /// that greet or name the player's ship say: a frame's, as buying a
    /// ship renames it.
    pub player_ship_name: Option<String>,
    /// Today's date as the arrival line and the take-off greeting say it
    /// ([`Session::long_date_text`](crate::Session::long_date_text)):
    /// a frame's, as landing and jumping move it on. Empty for a flight
    /// that failed to start.
    pub long_date: String,
    /// Every system the pilot has explored, by ascending ID
    /// ([`Pilot::explored`](crate::Pilot::explored)), which the nav area
    /// asks of the system it names, whichever nav mode it draws: the
    /// current one, or one held or noted before (a take-off's, a text's,
    /// an interface load's). The whole set, not one answer for the
    /// current mode, so a held mode's system reads as the pilot's
    /// exploration stands when it is drawn, as `_DrawStatusNav` reads the
    /// system's explored flag as it draws. Empty for a flight that failed
    /// to start: every system unexplored. [`FlightFrame::has_explored`]
    /// asks it.
    pub explored: Vec<SystemId>,
    /// The floating map's places, as
    /// [`Session::fill_frame`](crate::Session::fill_frame) works them out:
    /// the current system's map position, which its draw centres on every
    /// frame while it is up, and each link its clicks test. Filled in
    /// place, its links cleared and pushed again, so a kept frame's fill
    /// allocates nothing once its list has grown. No places for a flight
    /// that failed to start.
    pub floating: FloatingLinks,
}

impl Default for FlightFrame {
    /// A session that failed to start: nav off, no course, in flight, no
    /// jump, the player flying, no hold and no leave due, no Recharge and
    /// no intro seen, every gate taken
    /// ([`Gates::EVERY`]), and no death `dësc`s; the player alone, drawn in
    /// full in the ships' layer at the default place, with no NPCs, shots,
    /// beams, boxes, asteroids or stellars; no murk and no background; no
    /// ship's name, an empty date and no system explored.
    fn default() -> Self {
        Self {
            nav_mode: NavMode::Off,
            system: SystemId(0),
            course_shown: Vec::new(),
            course_plotted: false,
            landed: None,
            jump_under_way: false,
            warping: false,
            warp_end_due: false,
            fate: PlayerFate::Flying,
            hold_expired: false,
            leave_due: false,
            recharge_enabled: false,
            seen_intro: false,
            jump_readiness: JumpReadiness::Blocked,
            texts_queued: 0,
            gates: Gates::EVERY,
            map_key: MapKey::default(),
            ships: vec![FrameShip::default()],
            drawn: vec![(ShipRef::Player, ShipLayer::Ships)],
            shots: Vec::new(),
            beams: Vec::new(),
            cargo_boxes: Vec::new(),
            asteroids: Vec::new(),
            stellars: Vec::new(),
            effective_murk: 0,
            system_murk: 0,
            system_background: None,
            double_time: false,
            hyper_gamma: 0.0,
            traffic_ships: Vec::new(),
            hud: FrameHud::default(),
            target: None,
            death_descs: Vec::new(),
            player_ship_name: None,
            long_date: String::new(),
            explored: Vec::new(),
            floating: FloatingLinks::default(),
        }
    }
}

impl FlightFrame {
    /// Whether the player is docked ([`FlightFrame::landed`] is some).
    #[must_use]
    pub fn docked(&self) -> bool {
        self.landed.is_some()
    }

    /// The player's ship: [`FlightFrame::ships`]' first record, or the
    /// default one's place when a frame holds none.
    #[must_use]
    pub fn player(&self) -> FrameShip {
        self.ships
            .first()
            .filter(|ship| ship.ship == ShipRef::Player)
            .copied()
            .unwrap_or_default()
    }

    /// The record of `ship`, if it is in the system.
    #[must_use]
    pub fn ship(&self, ship: ShipRef) -> Option<&FrameShip> {
        self.ships.iter().find(|record| record.ship == ship)
    }

    /// The NPCs' records, in the system's order.
    pub fn npcs(&self) -> impl Iterator<Item = &FrameShip> {
        self.ships
            .iter()
            .filter(|record| matches!(record.ship, ShipRef::Npc(_)))
    }

    /// Every ship type a ship here is boxed by ([`FrameShip::framed`]),
    /// such as the stale frames of a hidden ship's slot, other than the
    /// player's own type for the player, whose sheet is read with its
    /// ship: the sheets the view must read to box them, the player's slot
    /// first when it holds another type's frames.
    pub fn framed_types(&self) -> impl Iterator<Item = ShipId> + '_ {
        let player = self
            .ships
            .iter()
            .filter(|record| record.ship == ShipRef::Player)
            .filter_map(|record| record.framed.filter(|&ship| ship != record.class));
        player.chain(self.npcs().filter_map(|npc| npc.framed))
    }

    /// The record of stellar `id`, if the system lists it.
    #[must_use]
    pub fn stellar(&self, id: StellarId) -> Option<&FrameStellar> {
        self.stellars.iter().find(|stellar| stellar.id == id)
    }

    /// Whether the pilot has explored `system` ([`FlightFrame::explored`]
    /// holds it).
    #[must_use]
    pub fn has_explored(&self, system: SystemId) -> bool {
        self.explored.contains(&system)
    }

    /// The stellars shown, in nav order
    /// ([`Session::stellars_shown`](crate::Session::stellars_shown)).
    pub fn stellars_shown(&self) -> impl Iterator<Item = StellarId> + '_ {
        self.stellars
            .iter()
            .filter(|stellar| stellar.shown)
            .map(|stellar| stellar.id)
    }
}

/// What the view is handed as the flight opens, fixed for the flight's
/// life, so none is a frame field: the weapons whose looks it reads, the
/// names it words the flight's messages with, and why the flight could
/// not start, if it could not
/// ([`Session::opening`](crate::Session::opening), or
/// [`FlightOpening::failed`]). A flight that failed fills the default
/// frame, refuses every command but a description's load and a plain
/// text's fill, and drains nothing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FlightOpening {
    /// The weapons the flight can fire, by ascending ID
    /// ([`Session::weapon_ids`](crate::Session::weapon_ids)): the
    /// session's arsenal, read-only for its life.
    pub weapons: Vec<WeaponId>,
    /// The names of the ships, outfits, goods, `jünk`s and stellars the
    /// session read as it started ([`Names`]).
    pub names: Names,
    /// Why the flight could not start, or `None` for one that started.
    pub failure: Option<String>,
}

impl FlightOpening {
    /// The opening of a flight that could not start, for `reason`: no
    /// weapons, and nothing named.
    #[must_use]
    pub fn failed(reason: impl Into<String>) -> Self {
        Self {
            weapons: Vec::new(),
            names: Names::default(),
            failure: Some(reason.into()),
        }
    }
}

/// The names fixed for a flight's life, which the view words its messages
/// with: each ship class's name and long name, each outfit's name and
/// lower-case name and plural, each good's name as the exchange names it,
/// each `jünk`'s lower-case name, and each stellar's `spöb` name, as the
/// session read them when it started
/// ([`Session::opening`](crate::Session::opening)). Plain data: a test
/// fills it from literals. The default names nothing, a flight that could
/// not start.
///
/// The names the flight decides as it runs are not here: the player's
/// ship's name and the date are frame fields
/// ([`FlightFrame::player_ship_name`], [`FlightFrame::long_date`]), and a
/// mission cargo's name rides its board line
/// ([`MissionBoardLine::Retrieved`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Names {
    /// Each ship class's names, by ID.
    pub ships: BTreeMap<ShipId, ShipNames>,
    /// Each outfit's names, by ID.
    pub outfits: BTreeMap<OutfitId, OutfitNames>,
    /// Each good's name as the exchange names it, a `jünk` among them.
    pub goods: BTreeMap<Good, String>,
    /// Each `jünk`'s `LCName`, raw, by ID: what a scan that finds it calls
    /// it.
    pub junk: BTreeMap<JunkId, String>,
    /// Each stellar's `spöb` name, by ID.
    pub stellars: BTreeMap<StellarId, String>,
}

/// A ship class's names, as its `shïp` record gives them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ShipNames {
    /// Its name.
    pub name: String,
    /// Its shipyard long name.
    pub long_name: String,
}

/// An outfit's names, as its `oütf` record gives them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OutfitNames {
    /// Its name.
    pub name: String,
    /// Its `LCName`.
    pub lc_name: String,
    /// Its `LCPlural`.
    pub lc_plural: String,
}

impl Names {
    /// Ship class `ship`'s name.
    #[must_use]
    pub fn ship(&self, ship: ShipId) -> Option<&str> {
        self.ships.get(&ship).map(|names| names.name.as_str())
    }

    /// Ship class `ship`'s shipyard long name.
    #[must_use]
    pub fn ship_long(&self, ship: ShipId) -> Option<&str> {
        self.ships.get(&ship).map(|names| names.long_name.as_str())
    }

    /// Outfit `outfit`'s name.
    #[must_use]
    pub fn outfit(&self, outfit: OutfitId) -> Option<&str> {
        self.outfits.get(&outfit).map(|names| names.name.as_str())
    }

    /// Outfit `outfit`'s lower-case name and plural.
    #[must_use]
    pub fn outfit_lc(&self, outfit: OutfitId) -> Option<(&str, &str)> {
        self.outfits
            .get(&outfit)
            .map(|names| (names.lc_name.as_str(), names.lc_plural.as_str()))
    }

    /// Good `good`'s name, as the exchange names it.
    #[must_use]
    pub fn good(&self, good: Good) -> Option<&str> {
        self.goods.get(&good).map(String::as_str)
    }

    /// `jünk` `junk`'s lower-case name.
    #[must_use]
    pub fn junk_lc(&self, junk: JunkId) -> Option<&str> {
        self.junk.get(&junk).map(String::as_str)
    }

    /// Stellar `stellar`'s `spöb` name.
    #[must_use]
    pub fn stellar(&self, stellar: StellarId) -> Option<&str> {
        self.stellars.get(&stellar).map(String::as_str)
    }
}

/// The player's commands the session takes now, as a set
/// ([`FlightFrame::gates`], filled from
/// [`Session::takes`](crate::Session::takes)): one bit for each
/// [`PlayerCommand`], so a fill writes it without allocating. The default
/// takes every one, as a flight that failed to start has nothing to refuse
/// them with.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Gates(u32);

impl Gates {
    /// Every command taken.
    pub const EVERY: Self = {
        let mut bits = 0;
        let mut i = 0;
        while i < PlayerCommand::ALL.len() {
            let bit = PlayerCommand::ALL[i] as u32;
            assert!(bit < u32::BITS, "a gate's bit fits in the set");
            bits |= 1 << bit;
            i += 1;
        }
        Self(bits)
    };

    /// No command taken.
    pub const NONE: Self = Self(0);

    /// The commands `taken` says are taken.
    #[must_use]
    pub fn from_fn(taken: impl Fn(PlayerCommand) -> bool) -> Self {
        let bits = PlayerCommand::ALL
            .into_iter()
            .filter(|&command| taken(command))
            .fold(0, |bits, command| bits | Self::bit(command));
        Self(bits)
    }

    /// Whether `command` is taken.
    #[must_use]
    pub fn takes(self, command: PlayerCommand) -> bool {
        self.0 & Self::bit(command) != 0
    }

    /// These gates with `command` refused, the rest as they are.
    #[must_use]
    pub fn refusing(self, command: PlayerCommand) -> Self {
        Self(self.0 & !Self::bit(command))
    }

    /// `command`'s bit.
    fn bit(command: PlayerCommand) -> u32 {
        1 << command as u32
    }
}

impl Default for Gates {
    /// [`Gates::EVERY`]: a flight that failed to start refuses nothing.
    fn default() -> Self {
        Self::EVERY
    }
}

impl std::fmt::Debug for Gates {
    /// The commands refused, as the rest are taken.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let refused: Vec<PlayerCommand> = PlayerCommand::ALL
            .into_iter()
            .filter(|&command| !self.takes(command))
            .collect();
        f.debug_struct("Gates").field("refused", &refused).finish()
    }
}

/// What the course map's drawing
/// ([`Session::map_drawing`](crate::Session::map_drawing)) is taken from:
/// the current system, the systems explored and charted, the systems
/// known, the course saved and the nav. Two keys are equal exactly when
/// all six are, so a view that keeps the key it last drew from takes the
/// drawing again when it changes. The view compares keys; it reads
/// nothing in them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapKey {
    /// The current system ([`Session::system`](crate::Session::system)).
    pub system: SystemId,
    /// The systems the pilot has explored, in the pilot's order.
    pub explored: Vec<SystemId>,
    /// The systems the pilot has charted, in the pilot's order.
    pub charted: Vec<SystemId>,
    /// The systems known ([`Session::known`](crate::Session::known)).
    pub known: BTreeSet<SystemId>,
    /// The course saved ([`Session::course`](crate::Session::course)).
    pub course: Vec<SystemId>,
    /// The nav mode ([`Session::nav_mode`](crate::Session::nav_mode)).
    pub nav: NavMode,
}

impl Default for MapKey {
    /// A failed session's: system 0, nothing explored, known or plotted,
    /// nav off.
    fn default() -> Self {
        Self {
            system: SystemId(0),
            explored: Vec::new(),
            charted: Vec::new(),
            known: BTreeSet::new(),
            course: Vec::new(),
            nav: NavMode::Off,
        }
    }
}

/// The floating map's places, as its clicks and its draw read them
/// ([`FlightFrame::floating`]): the current system's map position, and
/// each link its clicks test with its own.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FloatingLinks {
    /// The current system's map position.
    pub current: Vec2,
    /// Each link the clicks test, in their order, with its map position.
    pub links: Vec<(SystemId, Vec2)>,
}

/// What the HUD shows of the player's ship, its gear and its targets,
/// as [`FlightFrame::hud`] carries it: the status bar's gauges, cargo
/// area, secondary weapon line and radar gear, the `ïntf` it is chosen
/// by, and the target layer's brackets, Red Alert icon and escort menu
/// rows, with whether an escort is flying. The default is a failed
/// session's: no brackets, no alert, no escort of any class or flying, and
/// empty reserves and cargo.
#[derive(Clone, Debug, PartialEq)]
pub struct FrameHud {
    /// The ship's shield, armour and fuel
    /// ([`Session::reserves`](crate::Session::reserves)).
    pub reserves: Reserves,
    /// What the status bar's cargo area lists
    /// (`Session::status_cargo`).
    pub cargo: StatusCargo,
    /// The secondary weapon selected
    /// ([`Session::secondary`](crate::Session::secondary)).
    pub secondary: Option<WeaponId>,
    /// The count the secondary weapon's line shows after its name
    /// (`Session::secondary_line_count`).
    pub secondary_line_count: Option<u32>,
    /// What the player's outfits give the radar
    /// (`Session::radar_gear`).
    pub radar_gear: RadarGear,
    /// The interference in the player's system, 0..=100
    /// (`Session::interference`).
    pub interference: u8,
    /// The government whose `Interface` picks the status bar
    /// (`Session::interface_government`).
    pub interface_government: Option<GovtId>,
    /// The brackets round the nav target, if there is one
    /// (`Session::nav_brackets`).
    pub nav_brackets: Option<NavBrackets>,
    /// The brackets round the ship target, if there is one
    /// ([`Session::target_brackets`](crate::Session::target_brackets)).
    pub target_brackets: Option<TargetBrackets>,
    /// The Red Alert icon frame shown, if any
    /// (`Session::red_alert_icon`).
    pub red_alert: Option<AlertIcon>,
    /// The escort menu's class rows, in [`EscortClass::ALL`]'s order
    /// (`Space::escort_rows`).
    pub escort_rows: [ClassRow; 4],
    /// Whether the player has an escort flying, which the escort menu
    /// counts each frame it shows (`_HandlePlayer` @0x6df59-0x6dfa7): a
    /// ship the player leads, hired, captured, a mission escort or a
    /// launched fighter, neither disabled nor breaking up
    /// (`Space::escorts_flying`).
    pub escorts_flying: bool,
}

impl Default for FrameHud {
    /// A session that failed to start: empty reserves and cargo, no
    /// secondary, no radar gear or interference, no `Interface`
    /// government, no brackets, no Red Alert and no escort of any class,
    /// none flying.
    fn default() -> Self {
        Self {
            reserves: Reserves::default(),
            cargo: StatusCargo::default(),
            secondary: None,
            secondary_line_count: None,
            radar_gear: RadarGear::default(),
            interference: 0,
            interface_government: None,
            nav_brackets: None,
            target_brackets: None,
            red_alert: None,
            escort_rows: EscortClass::ALL.map(|class| ClassRow {
                class,
                present: false,
                order: None,
            }),
            escorts_flying: false,
        }
    }
}

/// The ship target as the target panel shows it, as
/// [`FlightFrame::target`] carries it: which NPC, its name and subtitle,
/// its status line and the label against the panel's bottom right. The
/// ship type and the shield the panel's redraws follow are the target's
/// own record's ([`FlightFrame::ship`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameTarget {
    /// The NPC targeted
    /// ([`Session::target`](crate::Session::target)).
    pub npc: NpcId,
    /// Its name, empty when the session has none for it
    /// (`Session::npc_name`).
    pub name: String,
    /// Its subtitle in place of its ship type's, if any
    /// (`Session::npc_subtitle`).
    pub subtitle: Option<String>,
    /// What its status line shows (`Session::target_status`).
    pub status: TargetStatus,
    /// The label against the panel's bottom right, if any
    /// (`Session::target_tag`).
    pub tag: Option<TargetTag>,
}

/// Something the player asks of the flight, which
/// [`Session::apply`](crate::Session::apply) answers.
#[derive(Clone, Debug, PartialEq)]
pub enum FlightCommand {
    /// A stellar key: make the stellar `pick` finds the nav target
    /// ([`Session::select_stellar`](crate::Session::select_stellar)).
    SelectStellar(StellarPick),
    /// Hyper Select: select the next system in mode 3
    /// ([`Session::select_next_system`](crate::Session::select_next_system)).
    SelectNextSystem,
    /// Option-backquote: let go of the ship target
    /// ([`Session::clear_target`](crate::Session::clear_target)).
    ClearTarget,
    /// A target key: pick the ship target as `pick` says
    /// ([`Session::select_target`](crate::Session::select_target)).
    SelectTarget(TargetPick),
    /// One press of the Nav Off / Hyperspace Mode block, with the
    /// backquote (`nav_off`) and H (`h`) down or not
    /// ([`Session::nav_mode_keys`](crate::Session::nav_mode_keys)).
    NavModeKeys {
        /// The backquote, without Option.
        nav_off: bool,
        /// H.
        h: bool,
    },
    /// A press on the course or hypergate map that hit systems
    /// ([`Session::map_click`](crate::Session::map_click)).
    MapClick(MapClick),
    /// The course map's Tab or `\`: cycle the hyperspace selection
    /// ([`Session::cycle_map_selection`](crate::Session::cycle_map_selection)).
    CycleMapSelection,
    /// Clear Route, from its button or C
    /// ([`Session::clear_route`](crate::Session::clear_route)).
    ClearRoute,
    /// A click hit on the floating map: the link, or `None` for the
    /// current system's centre
    /// ([`Session::select_on_floating_map`](crate::Session::select_on_floating_map)).
    SelectOnFloatingMap(Option<SystemId>),
    /// Mark the systems known, as the hypergate map's draw marks the
    /// offers it arrows ([`Session::mark_known`](crate::Session::mark_known)).
    MarkKnown(Vec<SystemId>),
    /// Cue the course's next hop, as the course map's close in flight
    /// does ([`Session::cue_course`](crate::Session::cue_course)).
    CueCourse,
    /// A port dialog opens: save the nav
    /// ([`Session::save_port_nav`](crate::Session::save_port_nav)).
    SavePortNav,
    /// A port dialog closes: restore the nav it saved
    /// ([`Session::restore_port_nav`](crate::Session::restore_port_nav)).
    RestorePortNav,
    /// The bar's map closes: put back the nav the bar opened with
    /// ([`Session::restore_bar_nav`](crate::Session::restore_bar_nav)).
    RestoreBarNav,
    /// The exchange's or the outfitter's map closes: put back the nav the
    /// dialog saved, keeping the save
    /// ([`Session::put_back_port_nav`](crate::Session::put_back_port_nav)).
    PutBackPortNav,
    /// A mission dialog's or port screen's map closes: put back the nav
    /// saved before it ([`Session::restore_map_nav`](crate::Session::restore_map_nav)).
    RestoreMapNav(SavedNav),
    /// The map closes in port: put back the selection it opened with,
    /// keeping the mode
    /// ([`Session::restore_map_selection`](crate::Session::restore_map_selection)).
    RestoreMapSelection(SavedNav),
    /// A left click in space at `at`, in system coordinates, the ships
    /// boxed where the view draws them
    /// ([`Session::click_in_space`](crate::Session::click_in_space)).
    ClickInSpace {
        /// Where the click is, in system coordinates.
        at: Vec2,
        /// The boxes the ships are drawn in.
        drawn: DrawnBoxes,
    },
    /// A press of Weapon Select, `backwards` with Option down: the next
    /// pass's secondary block takes it
    /// ([`Session::press_weapon_select`](crate::Session::press_weapon_select)).
    SelectWeapon {
        /// Whether Option was down at the press.
        backwards: bool,
    },
    /// The jump key: begin a jump
    /// ([`Session::begin_jump`](crate::Session::begin_jump)).
    Jump,
    /// The land key: request clearance, land or enter a gate
    /// ([`Session::land`](crate::Session::land)).
    Land,
    /// Open the hypergate the ship is at, for its map to offer its links
    /// ([`Session::open_hypergate`](crate::Session::open_hypergate)).
    OpenHypergate,
    /// Enter the hypergate opened, for the system picked on its map, or
    /// cancel with none
    /// ([`Session::enter_hypergate`](crate::Session::enter_hypergate)).
    EnterHypergate(Option<SystemId>),
    /// Pass through the wormhole the ship is at
    /// ([`Session::enter_wormhole`](crate::Session::enter_wormhole)).
    EnterWormhole,
    /// End the jump once its warp has ended
    /// ([`Session::arrive`](crate::Session::arrive)).
    Arrive,
    /// The port's Leave: take off
    /// ([`Session::leave_port`](crate::Session::leave_port)).
    TakeOff,
    /// Take off for a held `Q`'s leave, if one is due
    /// ([`Session::leave_if_due`](crate::Session::leave_if_due)).
    LeaveIfDue,
    /// Settle the escorts' marks as the ship leaves the stellar
    /// ([`Session::land_escorts`](crate::Session::land_escorts)).
    LandEscorts,
    /// Settle what the set expressions run since queued
    /// ([`Session::settle_script`](crate::Session::settle_script)).
    SettleScript,
    /// Begin the fade to black after the hold has expired
    /// ([`Session::begin_fade`](crate::Session::begin_fade)).
    BeginFade,
    /// End play once the fade is over
    /// ([`Session::end_play`](crate::Session::end_play)).
    EndPlay,
    /// The view showed a line on the message line for this many ticks:
    /// the session's line follows it
    /// ([`Session::comm_shown`](crate::Session::comm_shown)).
    CommShown(u16),
    /// Return (`ackComm`) cleared the message line: the session's for a
    /// tick while a line shows
    /// ([`Session::comm_acknowledged`](crate::Session::comm_acknowledged)).
    CommAcknowledged,
    /// The plunder dialog's loop counted the message line down a tick
    /// after a take's line, as its `_HandleComm` does
    /// ([`Session::comm_ticked`](crate::Session::comm_ticked)).
    CommTicked,
    /// The comm key: hail the target, or with Option held (`option`) the
    /// navigation stellar
    /// ([`Session::hail_key`](crate::Session::hail_key)).
    Hail {
        /// Whether Option was down at the press.
        option: bool,
    },
    /// Press the comm dialog's `n`th option listed, from 0
    /// ([`Session::answer`](crate::Session::answer)).
    AnswerHail(usize),
    /// A choice in the haggle dialog
    /// ([`Session::haggle`](crate::Session::haggle)).
    Haggle(Haggle),
    /// The comm dialog closes: end the hail under way
    /// ([`Session::hang_up`](crate::Session::hang_up)).
    HangUp,
    /// The Board key: board the target
    /// ([`Session::board`](crate::Session::board)).
    Board,
    /// A press in the plunder dialog
    /// ([`Session::plunder`](crate::Session::plunder)).
    Plunder(Take),
    /// Let the boarding under way go with no dialog shown
    /// ([`Session::let_go_of_boarding`](crate::Session::let_go_of_boarding)).
    LetGoOfBoarding,
    /// An escort key: give the escorts of `group` `command`
    /// ([`Session::command_escorts`](crate::Session::command_escorts)).
    CommandEscorts {
        /// The escort menu's group.
        group: EscortGroup,
        /// The command.
        command: EscortCommand,
    },
    /// A press in the escort dialog
    /// ([`Session::press_escort`](crate::Session::press_escort)).
    PressEscort(EscortButton),
    /// An order in the exchange
    /// ([`Session::trade`](crate::Session::trade)).
    Trade(Order),
    /// The spaceport's Recharge
    /// ([`Session::recharge`](crate::Session::recharge)).
    Recharge,
    /// The outfitter opens
    /// ([`Session::open_outfitter`](crate::Session::open_outfitter)).
    OpenOutfitter,
    /// The outfitter closes
    /// ([`Session::close_outfitter`](crate::Session::close_outfitter)).
    CloseOutfitter,
    /// An outfitter order of up to `count`, as the dialog's Buy or Sell
    /// runs it ([`Session::outfit_order`](crate::Session::outfit_order)).
    Outfit {
        /// The outfit, bought or sold.
        order: OutfitOrder,
        /// How many to buy or sell.
        count: u32,
    },
    /// An outfitter order of up to `count` that counts a Cancel's outfit
    /// day ([`Session::outfit_counted_run`](crate::Session::outfit_counted_run)).
    OutfitCounted {
        /// The outfit, bought or sold.
        order: OutfitOrder,
        /// How many to buy or sell.
        count: u32,
    },
    /// The shipyard opens, its list built and kept for the visit
    /// ([`Session::open_shipyard`](crate::Session::open_shipyard)).
    OpenShipyard,
    /// The name prompt's OK: buy a ship of class `ship` named `name`
    /// ([`Session::buy_ship`](crate::Session::buy_ship)).
    BuyShip {
        /// The class bought.
        ship: ShipId,
        /// The name the prompt gave it.
        name: String,
    },
    /// The name prompt's Cancel for a ship of this class
    /// ([`Session::decline_ship`](crate::Session::decline_ship)).
    DeclineShip(ShipId),
    /// The bar opens, drawing its visit's news
    /// ([`Session::open_bar`](crate::Session::open_bar)).
    OpenBar,
    /// Hire a ship of this class in the bar
    /// ([`Session::hire`](crate::Session::hire)).
    Hire(ShipId),
    /// The bar's hire screen closes
    /// ([`Session::close_hire`](crate::Session::close_hire)).
    CloseHire,
    /// The mission computer opens
    /// ([`Session::open_mission_computer`](crate::Session::open_mission_computer)).
    OpenMissionComputer,
    /// Select the mission computer's row of this mission, loading its
    /// text ([`Session::show_computer_row`](crate::Session::show_computer_row)).
    ShowComputerRow(MissionId),
    /// Select the mission info's row of this slot, loading its text
    /// ([`Session::show_info_row`](crate::Session::show_info_row)).
    ShowInfoRow(u8),
    /// Accept this mission
    /// ([`Session::accept_mission`](crate::Session::accept_mission)).
    AcceptMission(MissionId),
    /// Refuse this mission's offer
    /// ([`Session::refuse_mission`](crate::Session::refuse_mission)).
    RefuseMission(MissionId),
    /// Abort the mission in this slot
    /// ([`Session::abort_mission`](crate::Session::abort_mission)).
    AbortMission(u8),
    /// The next offer at the venue of this `loc`
    /// ([`Session::next_offer`](crate::Session::next_offer)).
    NextOffer(i16),
    /// Enter the venue of this `loc` for its offers
    /// ([`Session::enter_offer_venue`](crate::Session::enter_offer_venue)).
    EnterOfferVenue(i16),
    /// The mission info opened over this mission's offer dialog closes:
    /// load its offer `dësc` again
    /// ([`Session::reload_offer_desc`](crate::Session::reload_offer_desc)).
    ReloadOfferDesc(MissionId),
    /// The mission info key
    /// ([`Session::press_mission_info`](crate::Session::press_mission_info)).
    PressMissionInfo,
    /// The player info key (`Session::press_player_info`).
    PressPlayerInfo,
    /// The player info dialog opens (`Session::open_player_info`).
    OpenPlayerInfo,
    /// Jettison the cargo, `all` of it or all but the mission cargo
    /// ([`Session::jettison_cargo`](crate::Session::jettison_cargo)).
    Jettison {
        /// Whether the mission cargo goes too.
        all: bool,
    },
    /// Assign the ship captured
    /// ([`Session::assign`](crate::Session::assign)).
    Assign(Assignment),
    /// Answer Use As My Ship's rename: the name typed, or none for Cancel
    /// ([`Session::name_capture`](crate::Session::name_capture)).
    NameCapture(Option<String>),
    /// The intro screen was shown
    /// ([`Session::mark_intro_seen`](crate::Session::mark_intro_seen)).
    MarkIntroSeen,
    /// The view's game window is this many logical units wide, which each
    /// system's `Asteroids` is stored by
    /// ([`Session::set_window_width`](crate::Session::set_window_width)):
    /// a setting the view sends as it is built and resized.
    SetWindowWidth(u32),
    /// The double-time key is on or off
    /// ([`Session::set_double_time_key`](crate::Session::set_double_time_key)):
    /// a setting the view sends as the player's preference changes.
    SetDoubleTimeKey(bool),
    /// The player's sound effects are quiet or not, which the Red Alert's
    /// blink reads
    /// ([`Session::set_sound_quiet`](crate::Session::set_sound_quiet)): a
    /// setting the view sends as the sound preferences change.
    SetSoundQuiet(bool),
    /// An ask ([`FlightCommand::is_ask`]): what the pointer at `at`, in
    /// system coordinates, is over among the things the player can
    /// target, the ships boxed as `drawn` says they are drawn
    /// ([`Session::hover`](crate::Session::hover)), answered
    /// [`Answer::Hovered`]. The view's cursor asks it each step it shows,
    /// after the steps, with the boxes drawn from the frame they left.
    Hover {
        /// The pointer, in system coordinates.
        at: Vec2,
        /// Where the ships are drawn.
        drawn: DrawnBoxes,
    },
    /// An ask ([`FlightCommand::is_ask`]): loads `raw`, a `dësc` the view
    /// read that may be missing, naming `movie`, as `_LoadDescription`
    /// loads one ([`Session::load_description`](crate::Session::load_description)),
    /// answered [`Answer::Description`]: taken in for the pilot, its
    /// `l33t` letters drawn on the flight's chance, its movie the last
    /// loaded. It changes the run's rank governments and its last movie,
    /// none of which a frame shows, so it is an ask.
    LoadDescription {
        /// The `dësc`'s text, or `None` when the view could not read it.
        raw: Option<String>,
        /// The movie it names.
        movie: Option<DescMovie>,
    },
}

impl FlightCommand {
    /// The player's command the session takes this command under
    /// ([`Session::takes`](crate::Session::takes)): `None` for the commands
    /// no key gates, the map's own input and the sequencing the view runs
    /// at its own points. The view's one mapping from a command to its
    /// gate: it asks it before it sends a travel command, and before J's
    /// latch moves (`nova_view`'s `FlightView::takes_command` and
    /// `FlightView::send_gated`). Keys it asks mid-input, before any
    /// command exists, it gates by the key (`FlightView::command_of`).
    ///
    /// - The nav's keys and clicks ([`FlightCommand::SelectStellar`],
    ///   [`FlightCommand::NavModeKeys`], Option-backquote's
    ///   [`FlightCommand::ClearTarget`], a [`FlightCommand::ClickInSpace`]
    ///   and a [`FlightCommand::SelectOnFloatingMap`]) are
    ///   [`PlayerCommand::Nav`]; Hyper Select is
    ///   [`PlayerCommand::SelectSystem`], a target key
    ///   [`PlayerCommand::Target`] and Weapon Select
    ///   [`PlayerCommand::SelectWeapon`].
    /// - [`FlightCommand::Jump`] is [`PlayerCommand::Jump`]; the land key's
    ///   [`FlightCommand::Land`], and the gates it opens or passes
    ///   through ([`FlightCommand::OpenHypergate`],
    ///   [`FlightCommand::EnterWormhole`]), are [`PlayerCommand::Land`].
    /// - The comm key's [`FlightCommand::Hail`] is [`PlayerCommand::Hail`],
    ///   the Board key's [`FlightCommand::Board`] [`PlayerCommand::Board`],
    ///   an escort key's [`FlightCommand::CommandEscorts`]
    ///   [`PlayerCommand::Escort`], and the mission info key's
    ///   [`FlightCommand::PressMissionInfo`] [`PlayerCommand::MissionInfo`],
    ///   as the session asks it itself.
    /// - [`FlightCommand::EnterHypergate`] comes with the hypergate map's
    ///   close, which no key gates once the map is open, and is `None`, as
    ///   are the map's clicks, cycle and Clear Route, its marks and cue,
    ///   the nav's saves and restores, the arrival, the take-offs, the
    ///   escorts' landing, the settling, the fade, the end of play, the
    ///   message line's ([`FlightCommand::CommShown`],
    ///   [`FlightCommand::CommAcknowledged`], [`FlightCommand::CommTicked`])
    ///   the dialogs' presses: the comm dialog's answers, haggles and
    ///   close, the plunder dialog's takes and its letting go, and the
    ///   escort dialog's buttons; and the spaceport's: the shops', the
    ///   bar's and the mission screens', which run docked where no key
    ///   gates them. [`FlightCommand::Jettison`] is `None` too: its key's
    ///   press is gated by the key, and the player info's Jettison by no
    ///   gate. So are the settings ([`FlightCommand::SetWindowWidth`],
    ///   [`FlightCommand::SetDoubleTimeKey`],
    ///   [`FlightCommand::SetSoundQuiet`]), which no key sends.
    #[must_use]
    pub fn gate(&self) -> Option<PlayerCommand> {
        match self {
            Self::SelectStellar(_)
            | Self::NavModeKeys { .. }
            | Self::ClearTarget
            | Self::ClickInSpace { .. }
            | Self::SelectOnFloatingMap(_) => Some(PlayerCommand::Nav),
            Self::SelectNextSystem => Some(PlayerCommand::SelectSystem),
            Self::SelectTarget(_) => Some(PlayerCommand::Target),
            Self::SelectWeapon { .. } => Some(PlayerCommand::SelectWeapon),
            Self::Jump => Some(PlayerCommand::Jump),
            Self::Land | Self::OpenHypergate | Self::EnterWormhole => Some(PlayerCommand::Land),
            Self::Hail { .. } => Some(PlayerCommand::Hail),
            Self::Board => Some(PlayerCommand::Board),
            Self::CommandEscorts { .. } => Some(PlayerCommand::Escort),
            Self::PressMissionInfo => Some(PlayerCommand::MissionInfo),
            Self::PressPlayerInfo => Some(PlayerCommand::PlayerInfo),
            Self::MapClick(_)
            | Self::CycleMapSelection
            | Self::ClearRoute
            | Self::MarkKnown(_)
            | Self::CueCourse
            | Self::SavePortNav
            | Self::RestorePortNav
            | Self::RestoreBarNav
            | Self::PutBackPortNav
            | Self::RestoreMapNav(_)
            | Self::RestoreMapSelection(_)
            | Self::EnterHypergate(_)
            | Self::Arrive
            | Self::TakeOff
            | Self::LeaveIfDue
            | Self::LandEscorts
            | Self::SettleScript
            | Self::BeginFade
            | Self::EndPlay
            | Self::CommShown(_)
            | Self::CommAcknowledged
            | Self::CommTicked
            | Self::AnswerHail(_)
            | Self::Haggle(_)
            | Self::HangUp
            | Self::Plunder(_)
            | Self::LetGoOfBoarding
            | Self::PressEscort(_)
            | Self::Trade(_)
            | Self::Recharge
            | Self::OpenOutfitter
            | Self::CloseOutfitter
            | Self::Outfit { .. }
            | Self::OutfitCounted { .. }
            | Self::OpenShipyard
            | Self::BuyShip { .. }
            | Self::DeclineShip(_)
            | Self::OpenBar
            | Self::Hire(_)
            | Self::CloseHire
            | Self::OpenMissionComputer
            | Self::ShowComputerRow(_)
            | Self::ShowInfoRow(_)
            | Self::AcceptMission(_)
            | Self::RefuseMission(_)
            | Self::AbortMission(_)
            | Self::NextOffer(_)
            | Self::EnterOfferVenue(_)
            | Self::ReloadOfferDesc(_)
            | Self::Jettison { .. }
            | Self::Assign(_)
            | Self::NameCapture(_)
            | Self::MarkIntroSeen
            | Self::OpenPlayerInfo
            | Self::SetWindowWidth(_)
            | Self::SetDoubleTimeKey(_)
            | Self::SetSoundQuiet(_)
            | Self::Hover { .. }
            | Self::LoadDescription { .. } => None,
        }
    }

    /// Whether the command only asks: [`FlightCommand::Hover`] and
    /// [`FlightCommand::LoadDescription`]. An ask changes nothing a frame
    /// shows (`Session::fill_frame` fills the same frame after it, pinned
    /// in `session/apply/asks.rs`), so the view doesn't refill the frame
    /// it keeps after one (`nova_view`'s `FlightView::send`). The hover is
    /// asked after each tick's steps with the boxes the view draws, a
    /// per-frame question, and a description's load writes the run's
    /// rank governments and its last movie and draws on the chance, so
    /// neither is a screen's ask ([`ScreenAsk`]); a question a screen asks
    /// as it opens is.
    #[must_use]
    pub fn is_ask(&self) -> bool {
        matches!(self, Self::Hover { .. } | Self::LoadDescription { .. })
    }
}

/// A press on an in-flight map, as [`FlightCommand::MapClick`] carries it:
/// [`MapPress`]'s fields, owned.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapClick {
    /// The visible systems whose hit box holds the press, by ascending ID.
    pub hits: Vec<SystemId>,
    /// Whether Shift was held at the press.
    pub shift: bool,
    /// On the hypergate map, the systems the gate offers; `None` on the
    /// course map.
    pub hypergate: Option<Vec<SystemId>>,
    /// The mission systems taken as the map opened.
    pub missions: Vec<SystemId>,
    /// The green mission system the map's opener marked.
    pub green: Option<SystemId>,
}

impl MapClick {
    /// The press as [`Session::map_click`](crate::Session::map_click)
    /// reads it.
    #[must_use]
    pub fn press(&self) -> MapPress<'_> {
        MapPress {
            hits: &self.hits,
            shift: self.shift,
            hypergate: self.hypergate.as_deref(),
            missions: &self.missions,
            green: self.green,
        }
    }
}

/// What [`Session::apply`](crate::Session::apply) answered a
/// [`FlightCommand`].
#[derive(Clone, Debug, PartialEq)]
pub enum Answer {
    /// The command did nothing: nothing changed, and no sound was raised.
    Refused,
    /// The command ran, with nothing more to say.
    Done,
    /// The stellar a [`FlightCommand::SelectStellar`] selected.
    Stellar(StellarId),
    /// The system a [`FlightCommand::SelectNextSystem`] selected, or
    /// `None` when there was none to offer and the selection was let go.
    System(Option<SystemId>),
    /// The ship target a [`FlightCommand::SelectTarget`] left, `None` for
    /// none.
    Target(Option<NpcId>),
    /// What an accepted [`FlightCommand::MapClick`] did.
    Clicked(MapClicked),
    /// What a [`FlightCommand::ClickInSpace`] took: the ship it
    /// targeted, the player's own as it let go of the target, or the
    /// stellar it selected.
    InSpace(Hover),
    /// The system a [`FlightCommand::Jump`] is bound for: the jump has
    /// begun, or was already being prepared.
    Jumped(SystemId),
    /// Why a [`FlightCommand::Jump`] was refused, and whether the player
    /// is told ([`Session::jump_refusal_told`](crate::Session::jump_refusal_told)).
    /// Not [`Answer::Refused`]: a refusal told raises its beep.
    JumpRefused {
        /// Why.
        refusal: JumpRefusal,
        /// Whether the player is told.
        told: bool,
    },
    /// What a [`FlightCommand::Land`] did, when it was not refused.
    LandPress(LandPress),
    /// Why a [`FlightCommand::Land`] was refused, and what gate the
    /// stellar it names is, if any
    /// ([`Session::gate_kind`](crate::Session::gate_kind)). Not
    /// [`Answer::Refused`]: a refusal may raise its beep.
    LandRefused {
        /// Why.
        refusal: LandingRefusal,
        /// The gate the refusal's stellar is, if it names one and it is a
        /// gate.
        gate: Option<GateKind>,
    },
    /// The links a [`FlightCommand::OpenHypergate`] offers.
    Offered(Vec<SystemId>),
    /// Why entering or opening a gate took the ship nowhere. Not
    /// [`Answer::Refused`]: the hypergate's entry is consumed.
    GateRefused(GateRefusal),
    /// The system an arrival ([`FlightCommand::Arrive`],
    /// [`FlightCommand::EnterHypergate`], [`FlightCommand::EnterWormhole`])
    /// arrived in, and what happened in the fight up to the arrival that
    /// no step took: the explosions the jump left pending, a crön's `Y` on
    /// its days among them, for the view to show on the arrival's first
    /// frame.
    Arrived {
        /// The system arrived in.
        system: SystemId,
        /// The fight's events still pending, in order.
        combat: Vec<CombatEvent>,
    },
    /// What a take-off ([`FlightCommand::TakeOff`],
    /// [`FlightCommand::LeaveIfDue`]) did.
    Departed(Departure),
    /// What a [`FlightCommand::LandEscorts`] has to report over the
    /// spaceport; `None` when nothing was sold or upgraded.
    EscortReport(Option<String>),
    /// What a [`FlightCommand::SettleScript`] settled.
    Settled(Settled),
    /// What a [`FlightCommand::Hail`] opened, or the stellar that answered.
    Hailed(Hailed),
    /// Why a [`FlightCommand::Hail`] was refused. Not [`Answer::Refused`]:
    /// a refusal beeps, and any hail under way ends.
    HailRefused(HailRefusal),
    /// The comm dialog's contents after a [`FlightCommand::AnswerHail`] or
    /// a [`FlightCommand::Haggle`]; `None` when no hail is under way.
    Hail(Option<HailView>),
    /// What a [`FlightCommand::Board`] did, when it passed the checks.
    Boarded(Boarding),
    /// Why a [`FlightCommand::Board`] was refused. Not [`Answer::Refused`]:
    /// a refusal said beeps, and the line the last board left is let go.
    BoardRefused(BoardRefusal),
    /// What a [`FlightCommand::Plunder`] took, and whether the plunder
    /// dialog is still open after it.
    Plundered {
        /// What the press did.
        taken: Taken,
        /// Whether a boarding is still under way
        /// ([`Session::boarding`](crate::Session::boarding) is some).
        open: bool,
    },
    /// What a [`FlightCommand::CommandEscorts`] changed.
    Commanded(Commanded),
    /// What the escort dialog shows after a [`FlightCommand::PressEscort`]:
    /// `None` once it closes, or when it was not open.
    EscortDialog(Option<EscortDialogView>),
    /// What a [`FlightCommand::Trade`] traded, or why it was refused.
    Traded(Result<u32, TradeRefusal>),
    /// What a [`FlightCommand::Recharge`] cost, or why it was refused.
    Recharged(Result<i64, RechargeRefusal>),
    /// The run of a [`FlightCommand::Outfit`] or a
    /// [`FlightCommand::OutfitCounted`], refused at its first pass or not.
    Outfitted(OutfitRun),
    /// What a [`FlightCommand::BuyShip`] bought, or why it was refused.
    ShipBought(Result<ShipPurchase, ShipRefusal>),
    /// What a [`FlightCommand::Hire`] hired, or why it was refused.
    Hired(Result<Hired, HireRefusal>),
    /// The mission computer a [`FlightCommand::OpenMissionComputer`]
    /// opened, or why it has nothing.
    MissionComputer(Result<MissionComputer, ComputerRefusal>),
    /// The row a [`FlightCommand::ShowComputerRow`] selected; `None` for a
    /// mission not offered there.
    ComputerRow(Option<ComputerRow>),
    /// The text a [`FlightCommand::ShowInfoRow`] loaded; `None` for a free
    /// slot.
    InfoRow(Option<String>),
    /// The slot a [`FlightCommand::AcceptMission`] put the mission in, or
    /// why it was refused.
    Accepted(Result<u8, AcceptError>),
    /// What a [`FlightCommand::RefuseMission`] showed, or that the mission
    /// was not offered.
    RefuseShown(Result<RefuseShown, NotOffered>),
    /// Whether a [`FlightCommand::AbortMission`] aborted, or why not.
    Aborted(Result<(), AbortError>),
    /// The offer a [`FlightCommand::NextOffer`] made; `None` with nothing
    /// to offer.
    Offer(Option<VenueOffer>),
    /// What a [`FlightCommand::PressMissionInfo`] did, its own refusal
    /// among them: not [`Answer::Refused`], which is left to a flight
    /// with no session, as the view says #354 for that.
    MissionInfo(MissionInfoPress),
    /// Whether the session took a [`FlightCommand::PressPlayerInfo`]: its
    /// own refusal, which beeps, rides it, not [`Answer::Refused`], which
    /// is left to a flight with no session.
    PlayerInfoPressed(bool),
    /// What a [`FlightCommand::OpenPlayerInfo`] opened: the screen, or
    /// none in a jump or while the ship breaks up.
    PlayerInfo(Option<Box<PlayerInfo>>),
    /// What a [`FlightCommand::Assign`] did with the ship captured.
    Assigned(Assigned),
    /// What a [`FlightCommand::Hover`] found under the pointer, `None` for
    /// nothing.
    Hovered(Option<Hover>),
    /// The text a [`FlightCommand::LoadDescription`] loaded, empty for a
    /// missing `dësc`.
    Description(String),
}

impl Answer {
    /// Whether the command was refused.
    #[must_use]
    pub fn is_refused(&self) -> bool {
        matches!(self, Self::Refused)
    }
}

/// Something the flight raised for the view to act on once: one of the
/// session's one-shot outputs, drained in one stream by
/// [`Session::drain_events`](crate::Session::drain_events), in the order
/// its docs give. Each wraps the output as the session raised it; the
/// fight's own events are not among them (they come with each step).
#[derive(Clone, Debug, PartialEq)]
pub enum FlightEvent {
    /// A sound to play.
    Sound(SimSound),
    /// A message for the message line, in the session's terms.
    Message(SimMessage),
    /// A change to what the view draws of the session ([`SessionEvent`]).
    Session(SessionEvent),
    /// The pilot changed in a way that should be saved.
    SaveDue,
    /// The hold after the player's explosion ran out.
    HoldExpired,
    /// Game data the simulation does not handle yet, told once a session.
    Diagnostic(SimDiagnostic),
    /// What a boarding granted.
    Grant(Granted),
    /// The comm line a failed mission's ship said as a board plundered it.
    BoardLine(MissionBoardLine),
    /// What an NPC assisting the player did.
    Comm(CommNote),
    /// What the player's fighters met as it left a system.
    FighterNote(FighterNote),
    /// What paying the escorts did.
    PayNote(PayNote),
    /// A hail quote said.
    Quote(PersonQuote),
    /// A mission text or offer movie to show, with the status bar as a
    /// text was queued ([`TextBar`]; none for a movie).
    MissionShown {
        /// The text or movie.
        shown: MissionShown,
        /// The bar as the text was queued; `None` for a movie.
        bar: Option<TextBar>,
    },
    /// A run of docked scripts began: the `C`, `E` and `H` loads drained
    /// before it and not yet taken are let go, as the session lets go of
    /// its own at each run.
    InterfaceLoadsCleared,
    /// A `C`, `E` or `H` run while docked ([`InterfaceLoad`]).
    InterfaceLoad(InterfaceLoad),
    /// The nav mode as the last take-off began, before its deselect.
    TakeoffNav(NavMode),
    /// The escape pod's rescue's `dësc`.
    RescueDesc(i16),
    /// The message the last take-off left on, a held `Q`'s.
    LeaveMessage(String),
}

/// What a frame's fixed steps are given
/// ([`Session::step`](crate::Session::step)): the player's controls, and
/// the holds and settings the view takes from its keys, preferences and
/// window, which the session holds before the frame's first step, steps
/// or none.
// Each is its own key, chord or preference, held on its own.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StepInput {
    /// The player's steering, thrust and afterburner.
    pub controls: Controls,
    /// The play area about the ship, in world units
    /// ([`Session::set_play_area`](crate::Session::set_play_area)).
    pub play_area: Area,
    /// The game window's width
    /// ([`Session::set_window_width`](crate::Session::set_window_width)).
    pub window_width: u32,
    /// The primary and secondary fire keys
    /// ([`Session::hold_fire`](crate::Session::hold_fire)).
    pub fire: (bool, bool),
    /// The eject chord
    /// ([`Session::hold_eject`](crate::Session::hold_eject)).
    pub eject: bool,
    /// The self-destruct chord
    /// ([`Session::hold_self_destruct`](crate::Session::hold_self_destruct)).
    pub self_destruct: bool,
    /// The cloak key
    /// ([`Session::hold_cloak`](crate::Session::hold_cloak)).
    pub cloak: bool,
    /// The weapon safety key
    /// ([`Session::hold_weapon_safety`](crate::Session::hold_weapon_safety)).
    pub weapon_safety: bool,
    /// The double-time key
    /// ([`Session::set_double_time_key`](crate::Session::set_double_time_key)).
    pub double_time_key: bool,
    /// The Hyperspace Effects preference
    /// ([`Session::set_hyperspace_effects`](crate::Session::set_hyperspace_effects)).
    pub hyperspace_effects: bool,
    /// The land key
    /// ([`Session::set_land_key`](crate::Session::set_land_key)).
    pub land_key: bool,
}

/// What a frame's fixed steps reported
/// ([`Session::step`](crate::Session::step)): each step that ran, in
/// order, and whether the stepping stopped short. Filled into a report
/// the view keeps, as a frame is: each fill writes it afresh.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StepReport {
    /// Each step that ran, in order.
    pub steps: Vec<StepRun>,
    /// Whether the stepping stopped before a step it was asked for: the
    /// warp's end or the hold's expiry before it, or the escape pod's
    /// rescue on the step before.
    pub stopped: bool,
}

impl StepReport {
    /// Empties it for the next fill, keeping its room.
    pub fn clear(&mut self) {
        self.steps.clear();
        self.stopped = false;
    }
}

/// One fixed step that ran, as [`StepReport`] reports it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StepRun {
    /// Each ship's place before the step: the player's first, then each
    /// NPC's in the system's order.
    pub before: Vec<(ShipRef, ShipPlace)>,
    /// The comm notes taken at the advance pass's end: what the step
    /// before raised after its own take, then this step's.
    pub comm: Vec<CommNote>,
    /// Each pass that ran to its end: the advance pass, then double
    /// time's second; none when the rescue settled on the advance pass.
    pub passes: Vec<PassReport>,
    /// The player's ship where the escape pod's rescue put it, when the
    /// rescue settled on this step and ended the stepping.
    pub rescued: Option<ShipPlace>,
}

/// One pass of a step as it stood at the pass's end, after the NPCs
/// answered its strikes and the persons said their quotes: what the view
/// steps its effects, flashes, glows, shot looks and stars from.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PassReport {
    /// Each ship in the system: the player's first, then each NPC's.
    pub ships: Vec<FrameShip>,
    /// Each shot in flight, in the fight's order.
    pub shots: Vec<FrameShot>,
    /// Each beam's weapon, in the fight's order.
    pub beam_weapons: Vec<WeaponId>,
    /// What happened in the fight since the last pass's take, in order.
    pub combat: Vec<CombatEvent>,
    /// Whether the pass ran on a smoke frame
    /// ([`Session::smoke_frame`](crate::Session::smoke_frame)).
    pub smoke_frame: bool,
    /// The system's murk as loaded
    /// ([`Session::system_murk`](crate::Session::system_murk)).
    pub system_murk: i16,
    /// Where the player's ship is.
    pub player_position: Vec2,
    /// Whether double time is on
    /// ([`Session::double_time`](crate::Session::double_time)).
    pub double_time: bool,
    /// The quotes the pass said, in order.
    pub quotes: Vec<PersonQuote>,
}

/// A ship as a [`FlightFrame`] and a [`PassReport`] carry it: one record
/// per ship, built by the same code for both
/// (`Session::frame_ship_player`, `Session::frame_ship_npc`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameShip {
    /// Which ship.
    pub ship: ShipRef,
    /// Its class.
    pub class: ShipId,
    /// Where it is.
    pub position: Vec2,
    /// How fast it moves, per step.
    pub velocity: Vec2,
    /// Which way it faces, in degrees.
    pub heading: f32,
    /// How it is holding up.
    pub condition: Condition,
    /// The explosion its breakup plays, if any (its class's hull's).
    pub breakup: Option<Explosion>,
    /// Its shield, now and at most.
    pub shield: Gauge,
    /// Its class's own loaded `Shield`
    /// ([`Session::class_shield`](crate::Session::class_shield)).
    pub class_shield: Option<f32>,
    /// Its hull's frames: how its sheet's sets turn, bank and animate.
    pub frames: FrameSpec,
    /// The sets its display last drew it on, and its lights' level.
    pub drawn: Drawn,
    /// How much of it is drawn: the player's as its fate says
    /// ([`Session::player_fate`](crate::Session::player_fate)'s row), an
    /// NPC's in full.
    pub shown: ShipDrawn,
    /// Its engine glow's base, as drawn
    /// ([`drawn_base`](crate::glow::drawn_base); the player's
    /// [`Session::engine_glow`](crate::Session::engine_glow)).
    pub glow: u8,
    /// How its cloak lets the screen show it
    /// (`Session::cloak_shown`, the
    /// player's `Session::player_cloak_shown`).
    pub cloak: CloakShown,
    /// Its ion tint as its last display left it, if it glows.
    pub ion_tint: Option<IonTint>,
    /// Its fade at a gate it comes out of or enters ([`GateFade`]).
    pub gate: GateFade,
    /// Where its hover and click box is
    /// (`Session::hover_place`).
    pub hover: HoverPlace,
    /// The ship type whose frames its slot sprite holds, which sizes its
    /// box (`Session::framed_by`).
    pub framed: Option<ShipId>,
    /// Its government, whose target code the view reads; none for the
    /// player and for an independent.
    pub govt: Option<GovtId>,
    /// Whether it shows on the player's radar
    /// ([`Session::on_radar`](crate::Session::on_radar)); the player's
    /// never does, as the radar draws it apart.
    pub on_radar: bool,
    /// What IFF shows it as
    /// ([`Session::ship_iff`](crate::Session::ship_iff)); the player's
    /// [`ShipIff::Other`].
    pub iff: ShipIff,
    /// Its class's `Mass`, which a density scanner squares it by.
    pub mass: f32,
}

impl Default for FrameShip {
    /// The player's ship as a failed session draws it: at the default
    /// place, intact and in full, with no frames, glow, cloak or tint.
    fn default() -> Self {
        Self {
            ship: ShipRef::Player,
            class: ShipId(0),
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
            heading: 0.0,
            condition: Condition::default(),
            breakup: None,
            shield: Gauge::default(),
            class_shield: None,
            frames: FrameSpec::NONE,
            drawn: Drawn::default(),
            shown: ShipDrawn::Full,
            glow: 0,
            cloak: CloakShown::Opaque,
            ion_tint: None,
            gate: GateFade::None,
            hover: HoverPlace::Own,
            framed: None,
            govt: None,
            on_radar: false,
            iff: ShipIff::Other,
            mass: 0.0,
        }
    }
}

impl FrameShip {
    /// Where it is and which way it faces.
    #[must_use]
    pub fn place(&self) -> ShipPlace {
        ShipPlace {
            position: self.position,
            heading: self.heading,
        }
    }
}

/// Where a ship is and which way it faces: what the view draws a ship
/// between two steps from ([`StepRun::before`]).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ShipPlace {
    /// Where it is.
    pub position: Vec2,
    /// Which way it faces, in degrees.
    pub heading: f32,
}

impl From<ShipState> for ShipPlace {
    fn from(state: ShipState) -> Self {
        Self {
            position: state.position,
            heading: state.heading,
        }
    }
}

/// An NPC's fade at a gate: what tints and washes it as it comes out of
/// a hypergate or wormhole, or enters one to leave (the NPC's
/// `traffic::npc::Mode`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GateFade {
    /// At no gate.
    None,
    /// Waiting at the gate it comes out of, for this many more ticks.
    Emerging {
        /// The ticks of the wait left.
        ticks_left: u32,
    },
    /// Entering a gate to leave, for this many more ticks.
    Entering {
        /// The ticks of the entry left.
        ticks_left: u32,
    },
}

impl GateFade {
    /// The fade an NPC in `mode` shows.
    #[must_use]
    pub fn of(mode: &Mode) -> Self {
        match *mode {
            Mode::Emerging { ticks_left } => Self::Emerging { ticks_left },
            Mode::EnteringGate { ticks_left, .. } => Self::Entering { ticks_left },
            _ => Self::None,
        }
    }

    /// Whether it is at a gate, coming out or going in.
    #[must_use]
    pub fn at_gate(self) -> bool {
        self != Self::None
    }
}

/// A shot in flight as a [`FlightFrame`] and a [`PassReport`] carry it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameShot {
    /// Its number in the fight.
    pub id: ShotId,
    /// Its weapon.
    pub weapon: WeaponId,
    /// Where it is.
    pub position: Vec2,
    /// How fast it moves, per step.
    pub velocity: Vec2,
    /// Which way it faces, in degrees.
    pub heading: f32,
    /// How many steps it has flown.
    pub age: u32,
    /// Its spin's timer (`Shot::spin_timer`).
    pub spin_timer: u32,
    /// The sprite layer it is drawn in.
    pub layer: ShotLayer,
}

impl FrameShot {
    /// `shot`'s record.
    #[must_use]
    pub fn of(shot: &Shot) -> Self {
        Self {
            id: shot.id,
            weapon: shot.weapon.id,
            position: shot.position,
            velocity: shot.velocity,
            heading: shot.heading,
            age: shot.age,
            spin_timer: shot.spin_timer,
            layer: shot.layer,
        }
    }
}

/// A beam being fired as a [`FlightFrame`] carries it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameBeam {
    /// The ship firing it, whose drawn place moves it.
    pub firer: ShipRef,
    /// Where it starts.
    pub start: Vec2,
    /// Where it ends.
    pub end: Vec2,
    /// Its weapon.
    pub weapon: WeaponId,
    /// Its fade.
    pub fade: u32,
}

impl FrameBeam {
    /// `beam`'s record.
    #[must_use]
    pub fn of(beam: &Beam) -> Self {
        Self {
            firer: beam.firer,
            start: beam.start,
            end: beam.end,
            weapon: beam.weapon.id,
            fade: beam.fade,
        }
    }
}

/// A cargo box in space as a [`FlightFrame`] carries it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameBox {
    /// Where it is.
    pub position: Vec2,
    /// How fast it moves, per step.
    pub velocity: Vec2,
    /// The frame it shows.
    pub frame: u16,
    /// How far it has faded ([`CargoBox::fade`]).
    pub fade: u8,
    /// Its graphic.
    pub graphic: u16,
}

impl FrameBox {
    /// `cargo_box`'s record.
    #[must_use]
    pub fn of(cargo_box: &CargoBox) -> Self {
        Self {
            position: cargo_box.position,
            velocity: cargo_box.velocity,
            frame: cargo_box.frame,
            fade: cargo_box.fade(),
            graphic: cargo_box.graphic,
        }
    }
}

/// An asteroid as a [`FlightFrame`] carries it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameAsteroid {
    /// Where it is.
    pub position: Vec2,
    /// How fast it moves, per step.
    pub velocity: Vec2,
    /// The frame it is on, whole part drawn.
    pub frame: f32,
    /// Its kind.
    pub kind: u8,
}

impl FrameAsteroid {
    /// `asteroid`'s record.
    #[must_use]
    pub fn of(asteroid: &Asteroid) -> Self {
        Self {
            position: asteroid.position,
            velocity: asteroid.velocity,
            frame: asteroid.frame,
            kind: asteroid.kind,
        }
    }
}

/// A stellar its system lists, as a [`FlightFrame`] carries it: whether
/// it is shown, and how it is drawn.
// Each flag is one of the session's facts about the stellar.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameStellar {
    /// Which stellar.
    pub id: StellarId,
    /// Whether it is among the stellars shown
    /// ([`Session::stellars_shown`](crate::Session::stellars_shown)).
    pub shown: bool,
    /// Whether its galaxy holds it destroyed
    /// ([`Session::galaxy`](crate::Session::galaxy)).
    pub destroyed: bool,
    /// The sheet it draws
    /// (`Session::drawn_sheet`).
    pub drawn: DrawnSheet,
    /// The sheet whose frame count it animates over
    /// (`Session::counted_sheet`).
    pub counted: DrawnSheet,
    /// Whether its frame is frozen
    /// (`Session::frame_frozen`).
    pub frozen: bool,
    /// Whether it animates over no frames
    /// (`Session::counts_no_frames`).
    pub counts_none: bool,
    /// The frame it draws as a hypergate, or `None` for one that isn't
    /// one or isn't shown
    /// (`Session::gate_frame`).
    pub gate_frame: Option<u16>,
    /// What IFF shows it as on the radar, or `None` for one that isn't
    /// shown (`Session::stellar_iff`).
    pub iff: Option<StellarIff>,
}

/// The boxes the ships are drawn in, for the hover test and a click in
/// space ([`DrawnShips`]): the player's, and each NPC's that has one, in
/// the system's order. The view builds it from where it draws them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DrawnBoxes {
    /// The player's box, or `None` when it has none.
    pub player: Option<DrawnBox>,
    /// Each NPC's box, by id, in the system's order.
    pub npcs: Vec<(NpcId, DrawnBox)>,
}

impl DrawnBoxes {
    /// Whether it holds no box.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.player.is_none() && self.npcs.is_empty()
    }
}

impl DrawnShips for DrawnBoxes {
    fn npc(&self, id: NpcId) -> Option<DrawnBox> {
        self.npcs
            .iter()
            .find(|&&(npc, _)| npc == id)
            .map(|&(_, drawn)| drawn)
    }

    fn player(&self) -> Option<DrawnBox> {
        self.player
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A ship record of `ship`, of class `class`, boxed by `framed`.
    fn boxed(ship: ShipRef, class: i16, framed: Option<i16>) -> FrameShip {
        FrameShip {
            ship,
            class: ShipId(class),
            framed: framed.map(ShipId),
            ..FrameShip::default()
        }
    }

    #[test]
    fn the_types_framed_are_each_npcs_and_the_players_only_when_not_its_own() {
        let npcs = [
            boxed(ShipRef::Npc(NpcId(1)), 129, Some(129)),
            boxed(ShipRef::Npc(NpcId(2)), 130, Some(128)),
            boxed(ShipRef::Npc(NpcId(3)), 131, None),
        ];
        let own = FlightFrame {
            ships: [vec![boxed(ShipRef::Player, 140, Some(140))], npcs.to_vec()].concat(),
            ..FlightFrame::default()
        };
        assert_eq!(
            own.framed_types().collect::<Vec<_>>(),
            [ShipId(129), ShipId(128)],
            "the player's own type is read with its ship"
        );
        let stale = FlightFrame {
            ships: [vec![boxed(ShipRef::Player, 140, Some(150))], npcs.to_vec()].concat(),
            ..FlightFrame::default()
        };
        assert_eq!(
            stale.framed_types().collect::<Vec<_>>(),
            [ShipId(150), ShipId(129), ShipId(128)],
            "the player's slot first when it holds another type's frames"
        );
    }

    #[test]
    fn every_gate_is_taken_by_every_and_none_by_none() {
        for command in PlayerCommand::ALL {
            assert!(Gates::EVERY.takes(command), "{command:?}");
            assert!(!Gates::NONE.takes(command), "{command:?}");
        }
        assert_eq!(Gates::default(), Gates::EVERY);
        assert_eq!(Gates::from_fn(|_| true), Gates::EVERY);
        assert_eq!(Gates::from_fn(|_| false), Gates::NONE);
    }

    #[test]
    fn refusing_a_gate_refuses_it_alone() {
        for refused in PlayerCommand::ALL {
            let gates = Gates::EVERY.refusing(refused);
            for command in PlayerCommand::ALL {
                assert_eq!(gates.takes(command), command != refused, "{command:?}");
            }
            assert_eq!(gates, Gates::from_fn(|command| command != refused));
            assert_eq!(gates.refusing(refused), gates, "refused once is enough");
        }
        assert_eq!(
            format!("{:?}", Gates::EVERY.refusing(PlayerCommand::Land)),
            "Gates { refused: [Land] }"
        );
    }

    #[test]
    fn a_cleared_report_holds_no_step_and_no_stop() {
        let mut report = StepReport {
            steps: vec![StepRun::default(); 3],
            stopped: true,
        };
        report.clear();
        assert_eq!(report, StepReport::default());
    }

    #[test]
    fn each_command_is_gated_by_the_key_that_sends_it_or_by_none() {
        use crate::flight_frame::FlightCommand as C;
        let saved = SavedNav::of(NavMode::Off);
        let gated = [
            (
                C::SelectStellar(StellarPick::Slot(0)),
                Some(PlayerCommand::Nav),
            ),
            (
                C::NavModeKeys {
                    nav_off: true,
                    h: false,
                },
                Some(PlayerCommand::Nav),
            ),
            (C::ClearTarget, Some(PlayerCommand::Nav)),
            (
                C::ClickInSpace {
                    at: Vec2::ZERO,
                    drawn: DrawnBoxes::default(),
                },
                Some(PlayerCommand::Nav),
            ),
            (C::SelectOnFloatingMap(None), Some(PlayerCommand::Nav)),
            (C::SelectNextSystem, Some(PlayerCommand::SelectSystem)),
            (
                C::SelectTarget(TargetPick::Next),
                Some(PlayerCommand::Target),
            ),
            (
                C::SelectWeapon { backwards: false },
                Some(PlayerCommand::SelectWeapon),
            ),
            (C::Jump, Some(PlayerCommand::Jump)),
            (C::Land, Some(PlayerCommand::Land)),
            (C::OpenHypergate, Some(PlayerCommand::Land)),
            (C::EnterWormhole, Some(PlayerCommand::Land)),
            (
                C::MapClick(MapClick {
                    hits: Vec::new(),
                    shift: false,
                    hypergate: None,
                    missions: Vec::new(),
                    green: None,
                }),
                None,
            ),
            (C::CycleMapSelection, None),
            (C::ClearRoute, None),
            (C::MarkKnown(Vec::new()), None),
            (C::CueCourse, None),
            (C::SavePortNav, None),
            (C::RestorePortNav, None),
            (C::RestoreBarNav, None),
            (C::PutBackPortNav, None),
            (C::RestoreMapNav(saved), None),
            (C::RestoreMapSelection(saved), None),
            (C::EnterHypergate(Some(SystemId(131))), None),
            (C::Arrive, None),
            (C::TakeOff, None),
            (C::LeaveIfDue, None),
            (C::LandEscorts, None),
            (C::SettleScript, None),
            (C::BeginFade, None),
            (C::EndPlay, None),
            (C::CommShown(240), None),
            (C::Hail { option: false }, Some(PlayerCommand::Hail)),
            (C::Hail { option: true }, Some(PlayerCommand::Hail)),
            (C::Board, Some(PlayerCommand::Board)),
            (
                C::CommandEscorts {
                    group: crate::escort::EscortGroup::All,
                    command: crate::escort::EscortCommand::Hold,
                },
                Some(PlayerCommand::Escort),
            ),
            (C::AnswerHail(0), None),
            (C::Haggle(crate::hail::Haggle::Accept), None),
            (C::HangUp, None),
            (C::Plunder(crate::board::Take::Cargo), None),
            (C::LetGoOfBoarding, None),
            (
                C::PressEscort(crate::escort_dialog::EscortButton::Close),
                None,
            ),
            (C::CommAcknowledged, None),
            (C::CommTicked, None),
            (C::PressMissionInfo, Some(PlayerCommand::MissionInfo)),
            (C::PressPlayerInfo, Some(PlayerCommand::PlayerInfo)),
            (C::OpenPlayerInfo, None),
            (C::SetWindowWidth(1024), None),
            (C::SetDoubleTimeKey(true), None),
            (C::SetSoundQuiet(true), None),
        ];
        for (command, gate) in gated {
            assert_eq!(command.gate(), gate, "{command:?}");
        }
    }

    #[test]
    fn the_spaceports_and_the_mission_screens_commands_are_gated_by_none() {
        use crate::flight_frame::FlightCommand as C;
        let order = crate::OutfitOrder {
            outfit: crate::catalog::OutfitId(128),
            direction: crate::market::Direction::Buy,
        };
        let port = [
            C::Trade(crate::Order {
                row: 0,
                good: crate::market::Good::Commodity(0),
                direction: crate::market::Direction::Buy,
                lot: crate::market::Lot::Click,
            }),
            C::Recharge,
            C::OpenOutfitter,
            C::CloseOutfitter,
            C::Outfit { order, count: 1 },
            C::OutfitCounted { order, count: 1 },
            C::OpenShipyard,
            C::BuyShip {
                ship: ShipId(129),
                name: "Kestrel".to_owned(),
            },
            C::DeclineShip(ShipId(129)),
            C::OpenBar,
            C::Hire(ShipId(129)),
            C::CloseHire,
            C::OpenMissionComputer,
            C::ShowComputerRow(crate::MissionId(130)),
            C::ShowInfoRow(0),
            C::AcceptMission(crate::MissionId(130)),
            C::RefuseMission(crate::MissionId(130)),
            C::AbortMission(0),
            C::NextOffer(1),
            C::EnterOfferVenue(1),
            C::ReloadOfferDesc(crate::MissionId(130)),
            C::Jettison { all: true },
            C::Assign(crate::Assignment::Escort),
            C::NameCapture(None),
            C::MarkIntroSeen,
        ];
        for command in port {
            assert_eq!(command.gate(), None, "{command:?}");
        }
    }

    #[test]
    fn a_frame_is_docked_while_it_names_a_stellar_landed_on() {
        assert!(!FlightFrame::default().docked());
        let landed = FlightFrame {
            landed: Some(StellarId(128)),
            ..FlightFrame::default()
        };
        assert!(landed.docked());
    }

    #[test]
    fn only_a_refusal_is_refused() {
        assert!(Answer::Refused.is_refused());
        assert!(!Answer::Stellar(StellarId(128)).is_refused());
    }

    #[test]
    fn an_npcs_gate_fade_follows_its_mode() {
        assert_eq!(
            GateFade::of(&Mode::Emerging { ticks_left: 7 }),
            GateFade::Emerging { ticks_left: 7 }
        );
        let entering = Mode::EnteringGate {
            gate: StellarId(130),
            ticks_left: 5,
        };
        assert_eq!(
            GateFade::of(&entering),
            GateFade::Entering { ticks_left: 5 }
        );
        assert_eq!(GateFade::of(&Mode::Flying), GateFade::None);
        assert!(GateFade::Entering { ticks_left: 0 }.at_gate());
        assert!(GateFade::Emerging { ticks_left: 0 }.at_gate());
        assert!(!GateFade::None.at_gate());
    }

    /// A ship record of `ship` at `x`.
    fn at(ship: ShipRef, x: f32) -> FrameShip {
        FrameShip {
            ship,
            position: Vec2::new(x, 0.0),
            heading: x,
            ..FrameShip::default()
        }
    }

    #[test]
    fn a_frames_player_is_its_first_record_or_the_default_place() {
        let npc = ShipRef::Npc(NpcId(3));
        let frame = FlightFrame {
            ships: vec![at(ShipRef::Player, 5.0), at(npc, 9.0)],
            ..FlightFrame::default()
        };
        assert_eq!(frame.player(), at(ShipRef::Player, 5.0));
        assert_eq!(frame.ship(npc), Some(&at(npc, 9.0)));
        assert_eq!(frame.ship(ShipRef::Npc(NpcId(4))), None);
        assert_eq!(frame.npcs().copied().collect::<Vec<_>>(), [at(npc, 9.0)]);
        let empty = FlightFrame {
            ships: vec![at(npc, 9.0)],
            ..FlightFrame::default()
        };
        assert_eq!(empty.player(), FrameShip::default());
        assert_eq!(
            at(ShipRef::Player, 5.0).place(),
            ShipPlace {
                position: Vec2::new(5.0, 0.0),
                heading: 5.0,
            }
        );
    }

    #[test]
    fn a_ships_place_is_its_states_position_and_heading() {
        let state = ShipState {
            position: Vec2::new(1.0, 2.0),
            velocity: Vec2::new(3.0, 4.0),
            heading: 90.0,
        };
        assert_eq!(
            ShipPlace::from(state),
            ShipPlace {
                position: Vec2::new(1.0, 2.0),
                heading: 90.0,
            }
        );
    }

    #[test]
    fn a_frames_stellars_shown_are_those_flagged_in_their_order() {
        let stellar = |id, shown| FrameStellar {
            id: StellarId(id),
            shown,
            destroyed: false,
            drawn: DrawnSheet::Type,
            counted: DrawnSheet::Type,
            frozen: false,
            counts_none: false,
            gate_frame: None,
            iff: None,
        };
        let frame = FlightFrame {
            stellars: vec![stellar(131, true), stellar(128, true), stellar(129, false)],
            ..FlightFrame::default()
        };
        assert!(frame.stellars_shown().eq([StellarId(131), StellarId(128)]));
        assert_eq!(frame.stellar(StellarId(129)), Some(&stellar(129, false)));
        assert_eq!(frame.stellar(StellarId(130)), None);
    }

    #[test]
    fn drawn_boxes_give_each_ships_box_and_none_for_a_ship_without() {
        let square = |x| DrawnBox {
            centre: Vec2::new(x, 0.0),
            width: 10,
            height: 10,
        };
        let boxes = DrawnBoxes {
            player: Some(square(0.0)),
            npcs: vec![(NpcId(1), square(1.0)), (NpcId(2), square(2.0))],
        };
        assert_eq!(boxes.player(), Some(square(0.0)));
        assert_eq!(boxes.npc(NpcId(2)), Some(square(2.0)));
        assert_eq!(boxes.npc(NpcId(3)), None);
        assert!(!boxes.is_empty());
        assert!(DrawnBoxes::default().is_empty());
        let player_only = DrawnBoxes {
            player: Some(square(0.0)),
            npcs: Vec::new(),
        };
        assert!(!player_only.is_empty());
        let npc_only = DrawnBoxes {
            player: None,
            npcs: vec![(NpcId(1), square(1.0))],
        };
        assert!(!npc_only.is_empty());
    }
}
