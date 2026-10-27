//! The values the flight view's `FlightSim` port (`nova_view`'s
//! `flight::sim_port`) carries across the seam between the view and the
//! session: what a frame shows ([`FlightFrame`]), what the player asks
//! ([`FlightCommand`]) and what the session answers ([`Answer`]), what it
//! raised on the way ([`FlightEvent`]), and a frame's fixed steps' input and
//! report ([`StepInput`], [`StepReport`], its [`StepRun`]s and
//! [`PassReport`]s).
//!
//! They live here, not in the view, so the session can build them:
//! [`Session::fill_frame`](crate::Session::fill_frame) fills a frame and
//! [`Session::apply`](crate::Session::apply) answers a command. Each
//! starts with what the first slice moved across the seam needs, the nav
//! mode and the stellar keys, and grows as later slices move more.

use std::collections::BTreeSet;

use crate::asteroid::Asteroid;
use crate::bay::FighterNote;
use crate::board::MissionBoardLine;
use crate::cargo_box::CargoBox;
use crate::catalog::{Explosion, GovtId, ShipId, StellarId, SystemId, WeaponId};
use crate::cloak::{Area, CloakShown};
use crate::combat::beam::Beam;
use crate::combat::hull::Condition;
use crate::combat::ion_tint::IonTint;
use crate::combat::projectile::{Shot, ShotId};
use crate::combat::report::SimDiagnostic;
use crate::combat::ship_frames::{Drawn, FrameSpec};
use crate::combat::shot_layer::ShotLayer;
use crate::combat::{CombatEvent, ShipRef};
use crate::flight::{Controls, ShipState};
use crate::geometry::Vec2;
use crate::grant::Granted;
use crate::hail::CommNote;
use crate::hire::PayNote;
use crate::hover::{DrawnBox, DrawnShips, Hover};
use crate::hyperspace::JumpReadiness;
use crate::iff::ShipIff;
use crate::message::SimMessage;
use crate::navigation::NavMode;
use crate::player_fate::Drawn as ShipDrawn;
use crate::reserves::Gauge;
use crate::session::{
    HoverPlace, InterfaceLoad, MapClicked, MapPress, MissionShown, PersonQuote, SavedNav,
    SessionEvent, StellarPick, TextBar,
};
use crate::ship_layer::ShipLayer;
use crate::sound::SimSound;
use crate::story::DrawnSheet;
use crate::targeting::TargetPick;
use crate::traffic::npc::Mode;
use crate::traffic::npc::NpcId;

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
    /// Whether the player is docked
    /// ([`Session::landed`](crate::Session::landed) is some).
    pub docked: bool,
    /// Whether a jump is under way, the pre-jump stage included: the
    /// original's +0x50 > 0, which its key blocks test live
    /// ([`Session::jumping`](crate::Session::jumping) or
    /// [`Session::preparing_jump`](crate::Session::preparing_jump)).
    pub jump_under_way: bool,
    /// Whether the ship can jump to the system selected, as the nav area
    /// draws it ([`Session::jump_readiness`](crate::Session::jump_readiness)).
    pub jump_readiness: JumpReadiness,
    /// How many mission texts the session has queued, never reset
    /// ([`Session::texts_queued`](crate::Session::texts_queued)).
    pub texts_queued: u64,
    /// What the course map's drawing is taken from ([`MapKey`]).
    pub map_key: MapKey,
    /// Each ship in the system as the world draws it: the player's first,
    /// then each NPC's in the system's order
    /// ([`Session::npcs`](crate::Session::npcs)).
    pub ships: Vec<FrameShip>,
    /// The ships drawn, in draw order, each with its layer
    /// ([`Session::draw_layers`](crate::Session::draw_layers)).
    pub drawn: Vec<(ShipRef, ShipLayer)>,
    /// Each shot in flight, in the order it is drawn
    /// ([`Session::shots_drawn`](crate::Session::shots_drawn)).
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
}

impl Default for FlightFrame {
    /// A session that failed to start: nav off, no course, in flight, no
    /// jump; the player alone, drawn in full in the ships' layer at the
    /// default place, with no NPCs, shots, beams, boxes, asteroids or
    /// stellars; no murk and no background.
    fn default() -> Self {
        Self {
            nav_mode: NavMode::Off,
            system: SystemId(0),
            course_shown: Vec::new(),
            course_plotted: false,
            docked: false,
            jump_under_way: false,
            jump_readiness: JumpReadiness::Blocked,
            texts_queued: 0,
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
        }
    }
}

impl FlightFrame {
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

    /// The record of stellar `id`, if the system lists it.
    #[must_use]
    pub fn stellar(&self, id: StellarId) -> Option<&FrameStellar> {
        self.stellars.iter().find(|stellar| stellar.id == id)
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
/// ([`Session::floating_map_places`](crate::Session::floating_map_places)):
/// the current system's map position, and each link its clicks test
/// ([`Session::floating_map_links`](crate::Session::floating_map_links))
/// with its own.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FloatingLinks {
    /// The current system's map position.
    pub current: Vec2,
    /// Each link the clicks test, in their order, with its map position.
    pub links: Vec<(SystemId, Vec2)>,
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
    /// ([`Session::cloak_shown`](crate::Session::cloak_shown), the
    /// player's [`Session::player_cloak_shown`](crate::Session::player_cloak_shown)).
    pub cloak: CloakShown,
    /// Its ion tint as its last display left it, if it glows.
    pub ion_tint: Option<IonTint>,
    /// Its fade at a gate it comes out of or enters ([`GateFade`]).
    pub gate: GateFade,
    /// Where its hover and click box is
    /// ([`Session::hover_place`](crate::Session::hover_place)).
    pub hover: HoverPlace,
    /// The ship type whose frames its slot sprite holds, which sizes its
    /// box ([`Session::framed_by`](crate::Session::framed_by)).
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
    /// ([`Session::drawn_sheet`](crate::Session::drawn_sheet)).
    pub drawn: DrawnSheet,
    /// The sheet whose frame count it animates over
    /// ([`Session::counted_sheet`](crate::Session::counted_sheet)).
    pub counted: DrawnSheet,
    /// Whether its frame is frozen
    /// ([`Session::frame_frozen`](crate::Session::frame_frozen)).
    pub frozen: bool,
    /// Whether it animates over no frames
    /// ([`Session::counts_no_frames`](crate::Session::counts_no_frames)).
    pub counts_none: bool,
    /// The frame it draws as a hypergate, or `None` for one that isn't
    /// one or isn't shown
    /// ([`Session::gate_frame`](crate::Session::gate_frame)).
    pub gate_frame: Option<u16>,
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
