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

use crate::bay::FighterNote;
use crate::board::MissionBoardLine;
use crate::catalog::{Explosion, ShipId, StellarId, SystemId, WeaponId};
use crate::cloak::Area;
use crate::combat::hull::Condition;
use crate::combat::projectile::ShotId;
use crate::combat::report::SimDiagnostic;
use crate::combat::{CombatEvent, ShipRef};
use crate::flight::{Controls, ShipState};
use crate::geometry::Vec2;
use crate::grant::Granted;
use crate::hail::CommNote;
use crate::hire::PayNote;
use crate::hyperspace::JumpReadiness;
use crate::message::SimMessage;
use crate::navigation::NavMode;
use crate::reserves::Gauge;
use crate::session::{
    InterfaceLoad, MapClicked, MapPress, MissionShown, PersonQuote, SavedNav, SessionEvent,
    StellarPick, TextBar,
};
use crate::sound::SimSound;
use crate::targeting::TargetPick;
use crate::traffic::npc::NpcId;

/// What the flight view draws from, filled by
/// [`Session::fill_frame`](crate::Session::fill_frame): every field is
/// written on each fill, so a frame kept from one fill to the next shows
/// nothing stale. The default is what a session that failed to start
/// shows: nav off, no course, in flight, no jump.
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
}

impl Default for FlightFrame {
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
        }
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
    /// Each ship's state before the step: the player's first, then each
    /// NPC's in the system's order.
    pub before: Vec<(ShipRef, ShipState)>,
    /// The comm notes taken at the advance pass's end: what the step
    /// before raised after its own take, then this step's.
    pub comm: Vec<CommNote>,
    /// Each pass that ran to its end: the advance pass, then double
    /// time's second; none when the rescue settled on the advance pass.
    pub passes: Vec<PassReport>,
    /// The player's ship where the escape pod's rescue put it, when the
    /// rescue settled on this step and ended the stepping.
    pub rescued: Option<ShipState>,
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

/// A ship as a [`PassReport`] carries it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameShip {
    /// Which ship.
    pub ship: ShipRef,
    /// Its class.
    pub class: ShipId,
    /// Where it is.
    pub position: Vec2,
    /// How it is holding up.
    pub condition: Condition,
    /// The explosion its breakup plays, if any (its class's hull's).
    pub breakup: Option<Explosion>,
    /// Its shield, now and at most.
    pub shield: Gauge,
    /// Its class's own loaded `Shield`
    /// ([`Session::class_shield`](crate::Session::class_shield)).
    pub class_shield: Option<f32>,
}

/// A shot in flight as a [`PassReport`] carries it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameShot {
    /// Its number in the fight.
    pub id: ShotId,
    /// Its weapon.
    pub weapon: WeaponId,
    /// Where it is.
    pub position: Vec2,
    /// Which way it faces, in degrees.
    pub heading: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_refusal_is_refused() {
        assert!(Answer::Refused.is_refused());
        assert!(!Answer::Stellar(StellarId(128)).is_refused());
    }
}
