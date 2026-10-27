//! The set operators that move the player and play a sound: `M` and `N`
//! move it to another system, `Q` makes it leave the stellar it is landed
//! on, and `P` plays a sound.
//!
//! **The original.** `_EvalSetExp` (@0x150fc) dispatches on the letter
//! through the table at 0xdd2fc (@0x15cc2):
//!
//! | Operator | Where | What |
//! |---|---|---|
//! | `M` | @0x1570e-0x158d4 | the player's system is set (ship +0x74) and its stellar cleared (+0x6c = -1); the first of the new system's nav slots that is not -1 is found, with no check of landability or gates. Landed, that stellar becomes +0x6c and nothing else changes; in flight, the ship is put at its x/y at rest. With no stellar, nothing more: in flight the position and velocity are kept |
//! | `N` | @0x158d9-0x159ad | the system set and +0x6c = -1, landed or not (@0x158fd), with no stellar looked for; the position and velocity are never touched, and `_dontMovePlayerAfterLanding` (0x316951) is set |
//! | both | @0x15807-0x1583e | every active ship whose parent is the player (+0x9a 0: its escorts and fighters out) takes the new system; the others keep the old one, and `_DoPlayGameWork` runs only the ships in the player's system, so they stay behind |
//! | both, landed | | `_dockedPortMissions` is filled with -1, and that is all |
//! | both, in flight | @0x158a4 | the shots, cargo boxes (here the entry table's `Reset::Boxes`), explosions, smoke and asteroids (here `Reset::Asteroids`) are killed, the stellar sprites rebuilt, and `_MissionHandlePlayerEnteredNewSystem` runs |
//! | `Q` | @0x15bee-0x15c6c | `_absquatulateStr` (0x1635a0) becomes one of `STR#` `xxx`'s strings, the raw ID, with no range check, drawn as `T` draws (`_GetRandomIndString`: `Rand(count) + 1`); only inside a mission's set expression (`_currentMissionIndexForSetParser` 0-15, set by `_EvalCurrentMissionBitSetString` @0x99a6b alone, and -1 after it @0x99a79) is it run through `_MungeBriefing(0, slot)` (@0x15c37-0x15c4b), so a hook's text, and `OnRefuse`'s, is shown as it is. A missing or empty list, or an empty pick, leaves it empty (strcpy over an earlier one). Landed, it is held until the port's filter next runs, after every text and dialog over the port has closed (`_PortFilter` @0x4f047-0x4f063), and the take-off shows it (@0x63588-0x635e3); here [`Session::leave_due`] holds it for the take-off |
//!
//! Neither calls `_SetupShipsInSystem` or `_AutoSetExploration`, whose
//! only callers are a new pilot, the main screen, the take-off and
//! `_HandlePlayer`'s death (@0x69114) and hyperspace arrival (@0x6c1f9)
//! paths, so in flight the new system is neither explored nor populated
//! at once. Neither touches `_hyperRoute`, and `_CueNextHyperRouteDest`
//! (@0xe2dc) targets the route's next system only when it is linked from
//! the current one, so a stale route cannot be jumped along until it is
//! replotted.
//!
//! The take-off (`_PlayerLandOnStellar` @0x633ad-0x63400) zeroes the
//! velocity, then: with `_dontMovePlayerAfterLanding` set, clears it and
//! keeps the position, which the landing never moved; otherwise puts the
//! ship at +0x6c's x/y, or at the centre with none. It explores the
//! system (`_AutoSetExploration` @0x634c8), and the spaceport sets it up
//! afterwards (`_SetupShipsInSystem`). A landed move never touches
//! `_dockedPort`, which `_DoPortDialog` stores as it opens (@0x5f93b) and
//! its sub-dialogs read, so the open spaceport serves the stellar it
//! opened on until it is left.
//!
//! **Here.** `Mxxx` and `Nxxx` apply at their operator, as `_EvalSetExp`
//! applies them, with no catalog: a set expression runs in the middle of
//! a purchase, a capture or a hook (see the `operator_moves` module). The
//! operator writes the player's system and places the ship, landed or in
//! flight; landed it empties the offers and writes the nav selection,
//! from each system's first stellar read as the session starts (see the
//! `nav` module), so a dialog's redraw after the script shows it; in
//! flight it lets go of the selection, rebuilds the chart and dispatches
//! the enter-system hook. A later operator of the same script reads the
//! system moved to. A move to a system in range that does not exist runs
//! the move's tail with no move, as an `N` out of range does below, by
//! [`ScriptEffectRules::missing_system_move`]'s engine reading, and does
//! nothing by the other. Each move makes a save due. From a jump's end to
//! its entry an in-flight move is the jump's own (see the `jump_moves`
//! module), and from the ship leaving the spaceport to the take-off's
//! last nebula pass a move is the take-off's landed move (see the
//! `take_off_moves` module).
//!
//! What needs the catalog, the move's entry, waits for
//! [`Session::settle_script`], which whoever runs set expressions calls
//! after each input and tick, before saving: the story dispatcher's one
//! dispatch point. It completes the entry the moves owe, dispatches the
//! story events raised, then applies a `Q`'s leave in flight (a landed
//! one is held for the take-off), until nothing is left. In flight the
//! entry gives up a jump being prepared. The scripts the dispatcher runs
//! nest, by [`RuleKey::StoryReentry`]'s engine reading: after each
//! operator, their moves' entry completes and the events raised dispatch
//! before the next operator. The events a deferred script raises
//! dispatch from the caller's next `settle_script` (see the
//! [`story`](crate::story) module's "Dispatch").
//!
//! A move enters the system as every way in does, by its rows of the
//! table in the `entry` module ([`Session::enter_system`]), with the ship
//! where the operator put it, as below.
//!
//! In flight, the ship is in the new system: its stellars read, the
//! navigation target, a gate's pending entry, the shots and beams, the
//! strikes, the player's target, the boarding and the hail let go; a
//! held `P` sound is kept, and no arrival sounds or is raised. `M`
//! puts it at rest on the system's first stellar, whatever that is (a
//! gate too); in a system with no stellar it stays where it is, moving
//! as it was, unless [`ScriptEffectRules::starless`] puts it at rest at
//! the centre. `N` keeps the position and velocity.
//!
//! Landed, only where the pilot is changes: its system, and by `M` its
//! stellar, with the ship at rest at the first stellar's position (at the
//! centre with none); `N` leaves the pilot on no stellar, the ship where
//! it is. The open spaceport keeps serving the stellar landed on, as
//! [`Session::market`] and the rest read it, until the take-off swaps the
//! new system's stellars in. There is no other reading of that: an `N`
//! leaves no stellar to rebuild the spaceport for. By `M` the pilot's
//! stellar is the first one only when it has the can-land flag and is not
//! a hypergate or wormhole, otherwise none: the pilot's stellar is what a
//! save keeps as the stellar last landed on, through the flight after
//! the take-off too, and [`Session::fly`] docks a reloaded pilot there,
//! so the original's choice would dock it on a gate or a stellar that
//! cannot be landed on, which nothing else in the engine allows (landing
//! on a gate enters it). Such a pilot reloads in flight at the centre;
//! one saved after a landed `N` does too.
//!
//! Every take-off now explores the system, as the original's does, which
//! changes nothing in a system explored already. It keeps the ship where
//! it touched down, instead of the stellar's centre, after an `N` run
//! while docked: by [`ScriptEffectRules::keep_flag`]'s engine reading only
//! then, as the original clears its flag at every landing
//! (`_PlayerLandOnStellar` @0x631f1-0x631f6) before the take-off reads it
//! (@0x633ad); by the other, after any `N` since the last take-off, landed
//! or in flight. The flag is not saved.
//!
//! What else a move does follows [`ScriptEffectRules::arrival`]. By the
//! engine, nothing: the course is kept, and J refuses while its next
//! system is not linked from the new one
//! ([`JumpRefusal::NoDestination`](crate::JumpRefusal)); the system is
//! explored by the next take-off; and in flight, in another system, only
//! every ship the player leads (hired and mission escorts, fighters out)
//! follows, as it is, the new
//! system's ships coming only with its arrivals over time (see
//! [`Traffic::retarget`](crate::Traffic::retarget)); the rest stay in
//! the ship table through the next traffic tick, or are dropped at once,
//! as [`RuleKey::MoveLeftBehind`] says. By the other
//! reading the move is an arrival: the system is explored and the course
//! cleared at once, and in flight the fighters out are kept or abandoned
//! as the fighter rules say and the system is populated
//! ([`Session::populate`]).
//!
//! **An `N` out of range** (outside 128-2175) runs the move's tail with
//! no move, by [`ScriptEffectRules::out_of_range_move`]'s engine reading,
//! as the original's range check (@0x158e3) jumps to that tail
//! (@0x1593f) rather than past the operator. Landed, the offers are
//! emptied (@0x15949-0x15967) and nothing else changes: no enter-system
//! hook runs. In flight, the five scene flags are set, and
//! `_HandleStellarSystemVisibility` and
//! `_MissionHandlePlayerEnteredNewSystem` run (@0x1596e-0x159ad). Here
//! the player's own system is entered again as a move within the system,
//! the ship kept as it is, the pilot's stellar kept and no take-off hold
//! set. That move is never an arrival, whatever
//! [`ScriptEffectRules::arrival`] says, as it moves nowhere. It applies
//! at its operator and owes its entry as a move does. By the other reading it does nothing, as
//! every other operator out of range does (see the `control` module).
//!
//! **An `M` or `N` to a system with no `sÿst`** (in range) runs that same
//! tail, by [`ScriptEffectRules::missing_system_move`]'s engine reading:
//! neither operator checks that the system exists (@0x15723, @0x158ea),
//! and both tails branch only on the landed flag (@0x157fb-0x158d4,
//! @0x1593f-0x159ad). The original moves the player into the system
//! table's blank row first; a session holds no pilot in a system with no
//! record, so here the move itself, `M`'s placement on that row's unset
//! nav slots, `N`'s take-off hold and the escorts' move are left out, and
//! the ship, the pilot's stellar and the escorts stay as they are. By the
//! other reading the move does nothing.
//!
//! **`Qxxx`** draws its message when it runs, from
//! the session's string lists ([`Session::with_strings`]; none by
//! default), with a roll of as many sides as the list holds, as `T`
//! draws, so the order of the draws is kept. Nothing moves until it is
//! settled; whoever sees the original's string next acts on it:
//!
//! - Landed, every port dialog's filter (`_BarFilter` @0x47a4b,
//!   `_PortFilter` @0x4f041 and the others) closes on its next event, so
//!   `Q` does not wait for the spaceport to close, it forces it closed;
//!   but the texts and dialogs shown over the port (`_ShowTextDialog`,
//!   an offer, the mission info screen) never read the string, so the
//!   leave waits for them. The take-off then shows the message
//!   (`_DisplayComm` @0x635cf) in place of its usual greeting, with no
//!   beep, and clears it (@0x635e0). The landing clears it too, before
//!   the port opens (@0x6332a-0x63335). Here [`Session::settle_script`]
//!   holds a landed `Q` ([`Session::leave_due`]), and
//!   [`Session::leave_if_due`] carries it out: whoever shows the
//!   spaceport calls it once nothing shown over it holds the leave, and
//!   a driver with nothing shown (the storyline walker) after each
//!   settling. It leaves as the port's Leave does
//!   ([`Session::leave_port`]): [`Session::take_off`] consumes the leave,
//!   giving its message once ([`Session::take_leave_message`]), and the
//!   session settles again at once itself, so the take-off's
//!   `DayPassed`, `Explored` and `TookOff` dispatch before its save and
//!   its first frame, as the original runs them inside the take-off. The
//!   landing drops a leave still
//!   pending. A held leave is not saved, as the original's pilot file
//!   holds no such string, so a pilot saved at the landing reloads docked
//!   with no `Q`.
//! - In flight, the next frame beeps `_beepSnd[4]` (`_PlayGame`
//!   @0x4654c) and shows the message: here the message, and
//!   [`SimSound::ScriptMessage`].
//!
//! The text is filled only inside a mission's set expression, as the
//! original munges it only there: drawn while a mission's script runs,
//! it goes through the one substitution path for that mission's slot
//! ([`Session::mission_text`]; see [`text_tags`](crate::text_tags)).
//! The scripts that count are `OnAccept`, `OnAbort` (the player's, `A`'s
//! and an auto-abort's), `OnFailure` (at the return and at a missed
//! deadline) and `OnSuccess`; `OnSuccess` runs once its slot is freed, so
//! its mission tags read `"[Error]"`, as the original's do. After each,
//! the script's mission is none, not the one before (@0x99a79), so the
//! rest of a mission's script after a nested one's (an `S` in an
//! `OnAccept`) is shown as written. Anywhere else, `OnRefuse` and every
//! hook that is not a mission's, the text is the string as it is. A
//! blank `Q` (a missing
//! or empty list, or an empty pick) does nothing and cancels an earlier
//! one, unless [`ScriptEffectRules::blank_leave`] makes the player leave
//! all the same, with no message.
//!
//! **`Pxxx`** (@0x15a24) sets `_missionSoundID` to the
//! raw `snd ` ID, with no range check, and only `_PlayGame` reads it,
//! once a flight frame (@0x462fa): when it is not -1 and no mission sound
//! is held (`_missionSnd`), it loads and plays it at the effects volume;
//! it lets the held one go once it has stopped (@0x46358-0x46375); and
//! every frame resets the ID (@0x463a9), as do `_ResetPlayer` (@0x1d909)
//! and the hyperspace arrival (`_HandlePlayer` @0x6c4ff), but not a
//! hypergate or wormhole exit (`_PlayerEnterHypergate` @0x637bf,
//! `_PlayerEnterWormhole` @0x64005, which bypass that arrival). So only
//! the last `P` before a flight frame counts, one while a mission sound
//! still plays is dropped, and the spaceport never reads it, so a landed
//! `P` plays on the first flight frame after the take-off.
//!
//! Here, by [`ScriptEffectRules::sound`]'s engine reading, `P` holds its
//! sound, a later one replacing it, and the session's next flight pass
//! ([`Session::game_work`]) sounds it as [`SimSound::Script`], exclusive: the
//! audio side plays it on its one mission channel, unless the last sound
//! played there is still playing. A jump's arrival drops a held sound;
//! a hypergate or wormhole exit does not, so it sounds on the first tick
//! after. None is saved. By the other reading every `P` sounds at once,
//! landed or not, over whatever plays. The session never calls audio: the
//! sound leaves it as an event, drained with [`Session::take_sounds`].
//!
//! The first stellar a landed move's entry docks at is the first of
//! [`PilotCatalog::landing_sites`], destroyed or not, as the original
//! reads neither landability nor the destroyed state there. A landed `M`
//! docks the pilot at it when it has the can-land flag and is no gate,
//! destroyed or not and landable only once destroyed or not
//! ([`is_dockable_destroyed_or_not`](crate::landing::is_dockable_destroyed_or_not)),
//! as the original docks there; only
//! a gate or a stellar without the can-land flag leaves it on none.

use super::entry::{Placement, ShipPlacement};
use super::{MissionRun, Outbox, Session, Setup};
use crate::catalog::{PilotCatalog, SoundId, StellarId, SystemId, TrafficCatalog};
use crate::chance::Chance;
use crate::geometry::Vec2;
use crate::rulebook::{RuleKey, RuleSource, Rulebook};
use crate::sound::SimSound;

/// The disputed rules of the moving set operators that the session
/// follows, projected from its rule set (see [`Session::with_rules`]):
/// the engine's by default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScriptEffectRules {
    /// Where `M` puts a ship in flight in a system with no stellar
    /// ([`RuleKey::MoveStarless`]).
    pub starless: RuleSource,
    /// Whether a move is an arrival ([`RuleKey::MoveArrival`]).
    pub arrival: RuleSource,
    /// Whether an `N` in flight, or after a take-off's placement, keeps
    /// the next take-off's landing position ([`RuleKey::MoveKeepFlag`]):
    /// by the engine reading it doesn't, as every landing clears the flag.
    pub keep_flag: RuleSource,
    /// Whether a blank `Q` makes the player leave ([`RuleKey::BlankLeave`]).
    pub blank_leave: RuleSource,
    /// When `P`'s sound plays ([`RuleKey::ScriptSound`]).
    pub sound: RuleSource,
    /// Whether an `N` out of range runs the move's tail
    /// ([`RuleKey::OutOfRangeMove`]).
    pub out_of_range_move: RuleSource,
    /// Whether an `M` or `N` to a system with no record runs the move's
    /// tail ([`RuleKey::MissingSystemMove`]).
    pub missing_system_move: RuleSource,
}

impl ScriptEffectRules {
    /// The rules `rulebook` chooses: its [`RuleKey::MoveStarless`],
    /// [`RuleKey::MoveArrival`], [`RuleKey::MoveKeepFlag`],
    /// [`RuleKey::BlankLeave`], [`RuleKey::ScriptSound`],
    /// [`RuleKey::OutOfRangeMove`] and [`RuleKey::MissingSystemMove`]
    /// entries.
    #[must_use]
    pub fn from_rulebook(rulebook: &Rulebook) -> Self {
        Self {
            starless: rulebook.source_for(RuleKey::MoveStarless),
            arrival: rulebook.source_for(RuleKey::MoveArrival),
            keep_flag: rulebook.source_for(RuleKey::MoveKeepFlag),
            blank_leave: rulebook.source_for(RuleKey::BlankLeave),
            sound: rulebook.source_for(RuleKey::ScriptSound),
            out_of_range_move: rulebook.source_for(RuleKey::OutOfRangeMove),
            missing_system_move: rulebook.source_for(RuleKey::MissingSystemMove),
        }
    }
}

/// A move a set expression queued, for [`Session::settle_script`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ScriptMove {
    /// `M`: to the system, at its first stellar.
    To(SystemId),
    /// `N`: to the system, keeping the position.
    KeepPosition(SystemId),
    /// An `N` out of range, by [`ScriptEffectRules::out_of_range_move`]'s
    /// engine reading: the move's tail with no move. A move to a system
    /// with no record settles as this too, by
    /// [`ScriptEffectRules::missing_system_move`]'s engine reading.
    Stay,
}

/// What the set expressions left for later: the moves and the leave for
/// [`Session::settle_script`], and the sound for the next flight tick.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Queued {
    /// The moves, in order.
    pub(super) moves: Vec<ScriptMove>,
    /// The message a `Q` leaves on: none for no `Q` pending. Landed, it
    /// is held to the take-off, as `_absquatulateStr` is.
    leave: Option<String>,
    /// The message the last take-off left on, a held `Q`'s, until taken.
    left_with: Option<String>,
    /// The mission sound a `P` holds to the next flight tick.
    pub(super) sound: Option<SoundId>,
}

/// What [`Session::settle_script`] did.
#[must_use]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Settled {
    /// The system the last move applied went to, if any did.
    pub moved: Option<SystemId>,
    /// The message a `Q` in flight shows, if it has one. A landed `Q` is
    /// held for the take-off instead ([`Session::leave_due`]).
    pub message: Option<String>,
}

/// What [`Session::leave_port`] did: the stellar taken off from, and what
/// settling the take-off did.
#[must_use]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Departure {
    /// The stellar the ship took off from.
    pub stellar: StellarId,
    /// What settling the take-off's events did.
    pub settled: Settled,
}

impl Session {
    /// Applies what the set expressions run since the last settling
    /// queued (see the module docs): first a person's swap for its
    /// mission's ship that an accept on its hail made due (see the
    /// `mission_ships` module), then the moves in order, each read from
    /// `catalog`, populating on `chance` where the rules say, then a `Q`'s
    /// leave. Before the leave, and after the moves and the leave, the
    /// story events raised dispatch, each running the subscriptions it
    /// matches (see the [`story`](crate::story) module's "Dispatch").
    /// Once nothing is left, the ship goals of each mission accepted in
    /// flight are checked, and what that runs settles in turn (see the
    /// `mission_ships` module). Last, by the other reading of
    /// [`RuleKey::VisibilityRefresh`], when a set expression changed the
    /// pilot since the last settling, the galaxy is refreshed (see the `story` module's
    /// "Visibility and governments"); an idle settling evaluates nothing.
    /// Whoever runs set expressions calls it after each input and tick,
    /// before saving.
    pub fn settle_script(
        &mut self,
        catalog: &(impl PilotCatalog + TrafficCatalog + ?Sized),
        chance: &mut (impl Chance + ?Sized),
    ) -> Settled {
        // A new run of docked scripts (see [`Session::take_interface_loads`]).
        self.out.begin_docked_run();
        let mut settled = Settled::default();
        if let Some(stellar) = self.run.offer_roll_due.take()
            && self.mode.landed() == Some(stellar)
        {
            self.roll_landing_offers(stellar, catalog, &mut &mut *chance);
        }
        loop {
            if let Some((npc, slot)) = self.run.swap_due.take() {
                self.swap_in_mission_ship(catalog, npc, slot, chance);
            }
            self.settle_story(catalog, &mut &mut *chance, &mut settled);
            let due = std::mem::take(&mut self.run.goal_checks_due);
            if due.is_empty() {
                if std::mem::take(&mut self.chart.galaxy_stale)
                    && self.setup.rules.source_for(RuleKey::VisibilityRefresh) == RuleSource::Bible
                {
                    self.refresh_galaxy(catalog);
                }
                return settled;
            }
            for slot in due {
                self.check_ship_goals(slot, false, &mut &mut *chance);
            }
        }
    }

    /// Completes the entry the moves applied at their operators owe
    /// ([`Session::settle_pending_move`]; see the `operator_moves`
    /// module), reading `catalog`, populating on `chance` where the rules
    /// say, and notes the system moved to on `settled`.
    pub(super) fn settle_moves(
        &mut self,
        catalog: &(impl PilotCatalog + TrafficCatalog + ?Sized),
        chance: &mut (impl Chance + ?Sized),
        settled: &mut Settled,
    ) {
        if let Some(system) = self.settle_pending_move(catalog, catalog, chance) {
            settled.moved = Some(system);
        }
    }

    /// Whether a `Q` run while landed waits for the take-off: every port
    /// filter of the original leaves on its next call while
    /// `_absquatulateStr` is set (`_PortFilter` @0x4f047-0x4f063 and the
    /// others; see the module docs). The leave waits for the texts and
    /// dialogs shown over the spaceport, which never read it.
    #[must_use]
    pub fn leave_due(&self) -> bool {
        self.run.leave_due(self.mode.landed())
    }

    /// Carries out a held `Q`'s leave ([`Session::leave_due`]) as the
    /// port's Leave does ([`Session::leave_port`]), and what it did; `None`,
    /// and nothing changes, when no leave is due. Whoever shows the
    /// spaceport calls it once nothing shown over the port holds the
    /// leave; a driver with nothing shown, after each settling.
    pub(crate) fn leave_if_due(
        &mut self,
        catalog: &(impl PilotCatalog + TrafficCatalog + ?Sized),
        chance: &mut (impl Chance + ?Sized),
    ) -> Option<Departure> {
        if !self.leave_due() {
            return None;
        }
        self.leave_port(catalog, chance)
    }

    /// Leaves the spaceport, the one path for the port's Leave and a
    /// `Q`'s, as `_DoPortDialog` has one (@0x5fcd3, @0x601a0-0x6025e):
    /// takes off ([`Session::take_off`]), consuming a held leave, then
    /// settles the take-off's `DayPassed`, `Explored` and `TookOff`
    /// ([`Session::settle_script`]) at once, so they dispatch before the
    /// take-off's save and its first frame, as `_PlayerLandOnStellar` runs
    /// them inside the take-off. `None`, and nothing changes, when the
    /// ship has not landed.
    pub(crate) fn leave_port(
        &mut self,
        catalog: &(impl PilotCatalog + TrafficCatalog + ?Sized),
        chance: &mut (impl Chance + ?Sized),
    ) -> Option<Departure> {
        let stellar = self.take_off(catalog, chance)?;
        let settled = self.settle_script(catalog, chance);
        Some(Departure { stellar, settled })
    }

    /// The message the last take-off left on, a held `Q`'s, once: the
    /// take-off line, shown in place of the take-off's greeting, with no
    /// beep (`_DisplayComm` @0x635cf).
    pub(crate) fn take_leave_message(&mut self) -> Option<String> {
        self.run.take_leave_message()
    }

    /// `Q`: draws the message to leave on from `STR#` `list` on `chance`
    /// (see the module docs); a blank one cancels an earlier `Q`, or
    /// leaves with no message by [`ScriptEffectRules::blank_leave`]'s
    /// Bible reading.
    pub(super) fn leave_stellar(&mut self, list: i16, chance: &mut dyn Chance) {
        let picked = self
            .setup
            .pick_string(list, chance)
            .map(|text| match self.run.script_slot {
                Some(slot) => self.mission_text(slot, &text),
                None => text,
            });
        self.run.queued.leave = picked.or_else(|| self.setup.blank_leave());
    }
}

/// Where a move puts the ship (see the module docs), the system's first
/// stellar at `first`, with the stellar to dock at landed, if any, kept
/// when `keep` (`N`), `landed` or not, by `rules`: `M` stops it there,
/// its speed along its heading kept (`_EvalSetExp` @0x157a6-0x157da),
/// which landed is none already (`_PlayerLandOnStellar` @0x63409, and
/// nothing writes it while docked); with no stellar it
/// stays as it is in flight, unless [`ScriptEffectRules::starless`] stops
/// it at the centre, and landed it rests at the centre. `N` keeps the
/// ship, and holds the next take-off's landing position landed, or in
/// flight by [`ScriptEffectRules::keep_flag`]'s other reading.
pub(super) fn move_placement(
    first: Option<(Vec2, Option<StellarId>)>,
    keep: bool,
    landed: bool,
    rules: ScriptEffectRules,
) -> Placement {
    let ship = match first.map(|(position, _)| position) {
        _ if keep => ShipPlacement::Keep,
        Some(position) => ShipPlacement::Stopped(position),
        None if landed => ShipPlacement::AtRest(Vec2::ZERO),
        None if rules.starless == RuleSource::Bible => ShipPlacement::Stopped(Vec2::ZERO),
        None => ShipPlacement::Keep,
    };
    let stellar = first.and_then(|(_, dock)| dock).filter(|_| landed && !keep);
    Placement {
        ship,
        stellar,
        hold: keep && (landed || rules.keep_flag == RuleSource::Bible),
    }
}

impl Setup {
    /// The rules the moving set operators follow, projected from the
    /// session's rule set ([`Session::with_rules`]).
    pub(super) fn script_effect_rules(&self) -> ScriptEffectRules {
        ScriptEffectRules::from_rulebook(&self.rules)
    }

    /// A blank `Q`'s leave: none, cancelling an earlier `Q`, or one with
    /// no message by [`ScriptEffectRules::blank_leave`]'s Bible reading.
    fn blank_leave(&self) -> Option<String> {
        (self.script_effect_rules().blank_leave == RuleSource::Bible).then(String::new)
    }
}

impl MissionRun {
    /// Applies a `Q`'s leave in flight, if one is pending, noting its
    /// message on `settled`: whether one was. Landed, the leave is held,
    /// as `_absquatulateStr` is ([`Session::leave_due`]), for
    /// [`Session::leave_if_due`] to carry out, and nothing is applied.
    pub(super) fn settle_leave(
        &mut self,
        out: &mut Outbox,
        landed: Option<StellarId>,
        settled: &mut Settled,
    ) -> bool {
        if landed.is_some() {
            return false;
        }
        let Some(text) = self.queued.leave.take() else {
            return false;
        };
        settled.message = Some(text).filter(|text| !text.is_empty());
        if settled.message.is_some() {
            out.sounds.push(SimSound::ScriptMessage);
        }
        true
    }

    /// Whether a `Q` run while landed waits for the take-off: every port
    /// filter of the original leaves on its next call while
    /// `_absquatulateStr` is set (`_PortFilter` @0x4f047-0x4f063 and the
    /// others; see the module docs). The leave waits for the texts and
    /// dialogs shown over the spaceport, which never read it.
    #[must_use]
    pub(super) fn leave_due(&self, landed: Option<StellarId>) -> bool {
        landed.is_some() && self.queued.leave.is_some()
    }

    /// The take-off's part of a held `Q`: it consumes the leave, keeping
    /// its message, if it has one, for [`Session::take_leave_message`]
    /// (`_PlayerLandOnStellar` @0x63588-0x635e3). A take-off with no `Q`
    /// keeps none.
    pub(super) fn depart(&mut self) {
        self.queued.left_with = self.queued.leave.take().filter(|text| !text.is_empty());
    }

    /// The message the last take-off left on, a held `Q`'s, once: the
    /// take-off line, shown in place of the take-off's greeting, with no
    /// beep (`_DisplayComm` @0x635cf).
    fn take_leave_message(&mut self) -> Option<String> {
        self.queued.left_with.take()
    }

    /// Whether the last take-off left on a held `Q`'s message, not yet
    /// taken: the take-off's greeting is not said, as the message shows
    /// in its place and leaves the tutorial as it was
    /// (`_PlayerLandOnStellar` @0x63593-0x635e3).
    pub(super) fn left_with_message(&self) -> bool {
        self.queued.left_with.is_some()
    }

    /// Leaves `text` as the message the last take-off left on, for a
    /// test that needs one without a `Q` and a take-off.
    #[cfg(test)]
    pub(super) fn leave_with(&mut self, text: &str) {
        self.queued.left_with = Some(text.to_owned());
    }

    /// The landing's part: a leave still pending is dropped
    /// (`_PlayerLandOnStellar` clears `_absquatulateStr` before the port
    /// opens, @0x6332a-0x63335).
    pub(super) fn drop_leave(&mut self) {
        self.queued.leave = None;
    }

    /// `P`: plays `sound`, held to the next flight tick or at once, as
    /// [`ScriptEffectRules::sound`] says (see the module docs).
    pub(super) fn play_script_sound(&mut self, setup: &Setup, out: &mut Outbox, sound: SoundId) {
        match setup.script_effect_rules().sound {
            RuleSource::Engine => self.queued.sound = Some(sound),
            RuleSource::Bible => out.sounds.push(SimSound::Script {
                sound,
                exclusive: false,
            }),
        }
    }

    /// Sounds the mission sound a `P` holds, if any, as the flight's
    /// tick does.
    pub(super) fn sound_script(&mut self, out: &mut Outbox) {
        if let Some(sound) = self.queued.sound.take() {
            out.sounds.push(SimSound::Script {
                sound,
                exclusive: true,
            });
        }
    }

    /// A `Q` whose list is past 32767, the engine's widest ID, so names
    /// none: a blank `Q`, as `_GetRandomIndString` (@0x72ef7) leaves
    /// `_absquatulateStr` empty for a missing list (@0x72f0f).
    pub(super) fn leave_on_no_list(&mut self, setup: &Setup) {
        self.queued.leave = setup.blank_leave();
    }

    /// A `P` whose sound is past 32767, the engine's widest ID, so names
    /// none. By [`ScriptEffectRules::sound`]'s engine reading it replaces
    /// a held sound with none, as `_missionSoundID` is overwritten
    /// (@0x15a30) and `_PlayGame` plays nothing for a sound that does not
    /// load (`_ST_LoadSndResource` @0x4631b-0x46348); by the other, there
    /// is nothing to sound.
    pub(super) fn drop_script_sound(&mut self, setup: &Setup) {
        if setup.script_effect_rules().sound == RuleSource::Engine {
            self.queued.sound = None;
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
pub(super) mod tests {
    use super::*;
    use crate::catalog::{
        DudeId, DudeRecord, GovtId, HullRecord, LandingSite, ShipId, SoundId, StellarId,
        StockWeapon, SystemTraffic, WeaponId, WeaponRecord,
    };
    use crate::flight::Controls;
    use crate::testkit::{work, work_with};
    use std::rc::Rc;

    use crate::catalog::CommCatalog;
    use crate::chance::NeverFires;
    use crate::combat::Rules;
    use crate::control::SetExpr;
    use crate::gate::HYPERGATE;
    use crate::handling::ShipFields;
    use crate::hyperspace::JumpRefusal;
    use crate::message::SimMessage;
    use crate::pilot::{Escort, Pilot};
    use crate::reserves::Reserves;
    use crate::save;
    use crate::session::SessionEvent;
    use crate::testkit::{
        FAST, FakePilotCatalog, Scripted, begin_jump_now, catalog, fly_out, hull, land_now, planet,
        ship, star, weapon,
    };
    use crate::traffic::npc::NpcId;
    use crate::tutorial::Tutorial;

    /// A system of `avg` ships of düde 128, ship 129, for govt 140.
    fn traffic(system: i16, avg: i16) -> (SystemId, SystemTraffic) {
        let mut dude_types = [(-1, 0); 8];
        dude_types[0] = (128, 100);
        (
            SystemId(system),
            SystemTraffic {
                dude_types,
                avg_ships: avg,
                persons: Default::default(),
            },
        )
    }

    /// [`catalog`] with four systems: 130, the start (planets 128 at
    /// (30, -40) and 129), linked to 131; 131, linked to 132, its first
    /// stellar planet 140 at (100, 200), then 141; 132, with no stellar;
    /// and 133, linked to 130 alone, its first stellar the hypergate 150
    /// at (300, 0), then planet 151. Two ships fly in 130 and three in
    /// 131. Ship 129 has a record; ship 130, a fighter of no fuel, too.
    /// The player's ship carries a blaster, outfit 250 holding it.
    pub(in crate::session) fn moving() -> FakePilotCatalog {
        let base = catalog();
        let gate = LandingSite {
            flags2: HYPERGATE,
            ..planet(150, 300.0, 0.0)
        };
        let dry = ShipFields { fuel: 0, ..FAST };
        FakePilotCatalog {
            systems: [130, 131, 132, 133].map(SystemId).to_vec(),
            sites: vec![
                (SystemId(130), base.sites[0].1.clone()),
                (
                    SystemId(131),
                    vec![planet(140, 100.0, 200.0), planet(141, -50.0, 0.0)],
                ),
                (SystemId(133), vec![gate, planet(151, 0.0, 300.0)]),
            ],
            star_map: vec![
                star(130, (0.0, 0.0), &[131]),
                star(131, (600.0, 0.0), &[132]),
                star(132, (600.0, 600.0), &[]),
                star(133, (-600.0, 0.0), &[130]),
            ],
            traffic: vec![traffic(130, 2), traffic(131, 3)],
            dudes: vec![(
                DudeId(128),
                DudeRecord {
                    ai_type: 1,
                    govt: Some(GovtId(140)),
                    ships: vec![(ShipId(129), 1)],
                    booty: 0,
                    info_types: 0,
                },
            )],
            ships: vec![(ShipId(128), Ok(FAST)), (ShipId(130), Ok(dry))],
            ship_records: vec![ship(129, FAST), ship(130, dry)],
            weapons: vec![WeaponRecord {
                reload: 2,
                count: 30,
                speed: 2000,
                ..weapon(128)
            }],
            hulls: vec![HullRecord {
                weapons: vec![StockWeapon {
                    weapon: WeaponId(128),
                    count: 1,
                    ammo: 0,
                }],
                ..hull(128)
            }],
            // The outfit that holds the blaster, so the pilot owns it.
            outfits: [
                base.outfits.clone(),
                vec![crate::testkit::outfit(
                    250,
                    &[(crate::combat::armament::MOD_WEAPON, 128)],
                )],
            ]
            .concat(),
            ..base
        }
    }

    /// An escort of ship class `ship`, a fighter out when `carried`.
    fn escort(ship: i16, carried: bool) -> Escort {
        Escort {
            ship: ShipId(ship),
            reserves: Reserves::full(30.0, 45.0, 300.0),
            order: None,
            carried,
            wage: None,
            person: None,
            mark: None,
        }
    }

    /// `pilot`'s session in flight in 130 among its traffic, no save due.
    fn flying_with(catalog: &FakePilotCatalog, pilot: Pilot) -> Session {
        let mut session = Session::fly(catalog, pilot).expect("flies");
        session.populate(catalog, &mut NeverFires);
        session.take_save_due();
        session
    }

    fn flying(catalog: &FakePilotCatalog) -> Session {
        flying_with(catalog, Pilot::new(catalog, "Ada").expect("starts"))
    }

    /// [`flying`], landed on planet 128 from the centre.
    fn landed(catalog: &FakePilotCatalog) -> Session {
        let mut session = flying(catalog);
        land_now(&mut session).expect("lands");
        session.take_save_due();
        session.take_sounds();
        session
    }

    fn run(session: &mut Session, text: &str) {
        session.run_set(
            &SetExpr::parse(text).expect("parses"),
            &mut Scripted::default(),
        );
    }

    fn settle(session: &mut Session, catalog: &FakePilotCatalog) -> Settled {
        session.settle_script(catalog, &mut NeverFires)
    }

    /// Runs `text` and settles it.
    fn moved(session: &mut Session, catalog: &FakePilotCatalog, text: &str) {
        run(session, text);
        let _ = settle(session, catalog);
        session
            .chart
            .assert_docked_among_shown(session.mode.landed());
    }

    fn ids(sites: &[LandingSite]) -> Vec<i16> {
        sites.iter().map(|site| site.id.0).collect()
    }

    // The rules.

    #[test]
    fn each_rule_follows_its_own_rulebook_entry() {
        let engine = ScriptEffectRules {
            starless: RuleSource::Engine,
            arrival: RuleSource::Engine,
            keep_flag: RuleSource::Engine,
            blank_leave: RuleSource::Engine,
            sound: RuleSource::Engine,
            out_of_range_move: RuleSource::Engine,
            missing_system_move: RuleSource::Engine,
        };
        assert_eq!(ScriptEffectRules::default(), engine);
        assert_eq!(
            ScriptEffectRules::from_rulebook(&Rulebook::default()),
            engine
        );
        let bible = |key| {
            ScriptEffectRules::from_rulebook(
                &Rulebook::default().with_override(key, RuleSource::Bible),
            )
        };
        assert_eq!(
            bible(RuleKey::MoveStarless),
            ScriptEffectRules {
                starless: RuleSource::Bible,
                ..engine
            }
        );
        assert_eq!(
            bible(RuleKey::MoveArrival),
            ScriptEffectRules {
                arrival: RuleSource::Bible,
                ..engine
            }
        );
        assert_eq!(
            bible(RuleKey::MoveKeepFlag),
            ScriptEffectRules {
                keep_flag: RuleSource::Bible,
                ..engine
            }
        );
        assert_eq!(
            bible(RuleKey::BlankLeave),
            ScriptEffectRules {
                blank_leave: RuleSource::Bible,
                ..engine
            }
        );
        assert_eq!(
            bible(RuleKey::ScriptSound),
            ScriptEffectRules {
                sound: RuleSource::Bible,
                ..engine
            }
        );
        assert_eq!(
            bible(RuleKey::OutOfRangeMove),
            ScriptEffectRules {
                out_of_range_move: RuleSource::Bible,
                ..engine
            }
        );
        assert_eq!(
            bible(RuleKey::MissingSystemMove),
            ScriptEffectRules {
                missing_system_move: RuleSource::Bible,
                ..engine
            }
        );
    }

    // Through run_set.

    #[test]
    fn m_and_n_write_the_system_at_their_operators_and_owe_one_entry() {
        let catalog = moving();
        let mut session = flying(&catalog);
        run(&mut session, "M131 N132");
        assert_eq!(session.system(), SystemId(132), "at the operators");
        assert_eq!(ids(&session.chart.sites), [128, 129], "shown at the entry");
        assert_eq!(session.take_script_notes(), []);
        assert!(session.take_save_due());
        assert_eq!(settle(&mut session, &catalog).moved, Some(SystemId(132)));
        assert_eq!(session.system(), SystemId(132));
        assert!(session.take_save_due());
        assert_eq!(settle(&mut session, &catalog), Settled::default(), "once");
        assert!(!session.take_save_due());
    }

    #[test]
    fn the_moves_apply_in_order() {
        let catalog = moving();
        let mut session = flying(&catalog);
        moved(&mut session, &catalog, "M131 N133");
        assert_eq!(session.system(), SystemId(133));
        assert_eq!(session.player().position, Vec2::new(100.0, 200.0));
        assert_eq!(ids(&session.chart.sites), [150, 151]);
    }

    /// The rules with `MissingSystemMove` read as `reading`.
    fn missing_system(reading: RuleSource) -> Rulebook {
        Rulebook::default().with_override(RuleKey::MissingSystemMove, reading)
    }

    #[test]
    fn by_the_other_reading_a_move_to_a_missing_system_does_nothing() {
        let catalog = moving();
        for start in [flying, landed] {
            let mut session = start(&catalog).with_rules(missing_system(RuleSource::Bible));
            let _ = settle(&mut session, &catalog);
            session.take_events();
            let before = session.clone();
            run(&mut session, "M150 N999");
            assert_eq!(settle(&mut session, &catalog), Settled::default());
            assert_eq!(session, before);
            assert!(!session.take_save_due());
        }
    }

    // In flight.

    #[test]
    fn m_in_flight_puts_the_ship_at_rest_on_the_first_stellar() {
        let catalog = moving();
        let mut session = flying(&catalog);
        session.ship.player.position = Vec2::new(10.0, 10.0);
        session.ship.player.velocity = Vec2::new(5.0, -1.0);
        session.ship.player.heading = 1.5;
        run(&mut session, "M131");
        assert_eq!(settle(&mut session, &catalog).moved, Some(SystemId(131)));
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(session.player().position, Vec2::new(100.0, 200.0));
        assert_eq!(session.player().velocity, Vec2::ZERO);
        assert_eq!(session.player().heading, 1.5, "heading kept");
        assert_eq!(ids(&session.chart.sites), [140, 141]);
        assert_eq!(session.landed(), None);
        assert_eq!(session.pilot().stellar(), None);
        assert!(session.take_save_due());
        moved(&mut session, &catalog, "M133");
        assert_eq!(
            session.player().position,
            Vec2::new(300.0, 0.0),
            "a gate as any stellar"
        );
    }

    #[test]
    fn m_in_flight_keeps_the_speed_along_the_heading() {
        // `_EvalSetExp` zeroes +0x20/+0x24 (@0x157d1, @0x157da) and never
        // writes +0x48, which an inertialess ship's velocity follows from
        // the next frame (`_AdjustInertialessShipVelocity` @0x6d43a).
        let catalog = moving();
        let mut session = flying(&catalog);
        session.ship.player_speed = 4.0;
        moved(&mut session, &catalog, "M131");
        assert_eq!(session.player().velocity, Vec2::ZERO);
        assert_eq!(session.player_speed(), 4.0, "kept");
    }

    /// `pilot` saved and loaded, then flown.
    fn reloaded(catalog: &FakePilotCatalog, pilot: &Pilot) -> Session {
        let saved = save::decode(&save::encode(pilot)).expect("loads");
        Session::fly(catalog, saved).expect("flies")
    }

    #[test]
    fn a_move_in_flight_to_another_system_forgets_the_stellar_last_landed_on() {
        let catalog = moving();
        let mut session = landed(&catalog);
        assert_eq!(
            session.take_off(&catalog, &mut NeverFires),
            Some(StellarId(128))
        );
        assert_eq!(session.pilot().stellar(), Some(StellarId(128)), "kept");
        moved(&mut session, &catalog, "M131");
        assert_eq!(session.pilot().stellar(), None);
        let flown = reloaded(&catalog, session.pilot());
        assert_eq!(flown.system(), SystemId(131));
        assert_eq!(flown.landed(), None, "in flight");
        assert_eq!(flown.player().position, Vec2::ZERO, "at the centre");
        moved(&mut session, &catalog, "N130");
        assert_eq!(session.pilot().stellar(), None, "not 128 again");
        let flown = reloaded(&catalog, session.pilot());
        assert_eq!(flown.system(), SystemId(130));
        assert_eq!(flown.landed(), None, "not docked at 128");
        assert_eq!(flown.player().position, Vec2::ZERO);
    }

    #[test]
    fn a_move_in_flight_within_the_system_keeps_the_stellar_last_landed_on() {
        let catalog = moving();
        let mut session = landed(&catalog);
        session.take_off(&catalog, &mut NeverFires);
        moved(&mut session, &catalog, "N130");
        assert_eq!(session.system(), SystemId(130));
        assert_eq!(session.pilot().stellar(), Some(StellarId(128)));
        let flown = reloaded(&catalog, session.pilot());
        assert_eq!(
            crate::testkit::resumed_on(&flown),
            Some(StellarId(128)),
            "on it again"
        );
    }

    #[test]
    fn n_in_flight_keeps_the_position_and_velocity() {
        let catalog = moving();
        let mut session = flying(&catalog);
        session.ship.player.position = Vec2::new(10.0, 10.0);
        session.ship.player.velocity = Vec2::new(5.0, -1.0);
        moved(&mut session, &catalog, "N131");
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(session.player().position, Vec2::new(10.0, 10.0));
        assert_eq!(session.player().velocity, Vec2::new(5.0, -1.0));
        assert_eq!(ids(&session.chart.sites), [140, 141]);
    }

    #[test]
    fn m_into_a_system_with_no_stellar_keeps_the_ship_moving_by_the_engine() {
        let catalog = moving();
        let mut session = flying(&catalog);
        session.ship.player.position = Vec2::new(10.0, 10.0);
        session.ship.player.velocity = Vec2::new(5.0, -1.0);
        moved(&mut session, &catalog, "M132");
        assert_eq!(session.system(), SystemId(132));
        assert_eq!(session.player().position, Vec2::new(10.0, 10.0));
        assert_eq!(session.player().velocity, Vec2::new(5.0, -1.0));
        assert_eq!(session.chart.sites, []);
    }

    #[test]
    fn m_into_a_system_with_no_stellar_stops_at_the_centre_by_the_bible() {
        let catalog = moving();
        let rules = Rulebook::default().with_override(RuleKey::MoveStarless, RuleSource::Bible);
        let mut session = flying(&catalog).with_rules(rules);
        session.ship.player.position = Vec2::new(10.0, 10.0);
        session.ship.player.velocity = Vec2::new(5.0, -1.0);
        moved(&mut session, &catalog, "M132");
        assert_eq!(session.player().position, Vec2::ZERO);
        assert_eq!(session.player().velocity, Vec2::ZERO);
        session.ship.player.velocity = Vec2::new(5.0, -1.0);
        moved(&mut session, &catalog, "N131");
        assert_eq!(session.player().velocity, Vec2::new(5.0, -1.0), "N keeps");
    }

    #[test]
    fn a_move_in_flight_lets_go_of_the_scene() {
        let catalog = moving();
        for text in ["M131", "N130"] {
            let mut session = flying(&catalog);
            session.hold_fire(true, false);
            work(&mut session, &catalog, Controls::default());
            assert_eq!(session.shots().len(), 1, "{text}");
            session.select_stellar(crate::session::StellarPick::Slot(0));
            session.chart.target = Some(NpcId(0));
            session.ship.warp.gate = Some(StellarId(128));
            session.space.strikes = session.space.combat.take_strikes();
            moved(&mut session, &catalog, text);
            assert_eq!(session.shots(), [], "{text}");
            assert_eq!(session.nav_target(), None, "{text}");
            assert_eq!(session.chart.target, None, "{text}");
            assert_eq!(session.ship.warp.gate, None, "{text}");
            assert_eq!(session.space.strikes, [], "{text}");
        }
    }

    #[test]
    fn a_move_in_flight_gives_up_a_jump_being_prepared() {
        let catalog = moving();
        let mut session = flying(&catalog);
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session, &catalog);
        session.ship.player.velocity = Vec2::new(0.0, 5.0);
        session.begin_jump().expect("accepted");
        assert!(session.preparing_jump().is_some());
        work(&mut session, &catalog, crate::flight::Controls::default());
        assert!(session.thrusting(), "braking");
        moved(&mut session, &catalog, "N133");
        assert_eq!(session.preparing_jump(), None);
        assert!(!session.thrusting());
        assert_eq!(session.engine_glow(), 0);
    }

    #[test]
    fn a_move_in_flight_with_no_jump_to_give_up_leaves_the_thrust_on() {
        let catalog = moving();
        let mut session = flying(&catalog);
        let thrust = crate::flight::Controls {
            thrust: true,
            ..crate::flight::Controls::default()
        };
        work(&mut session, &catalog, thrust);
        session.take_sounds();
        moved(&mut session, &catalog, "N131");
        assert!(session.thrusting());
        assert_eq!(session.take_sounds(), [], "no stop sounded");
    }

    #[test]
    fn by_the_engine_only_the_escorts_and_fighters_out_follow_into_another_system() {
        let catalog = moving();
        let mut pilot = Pilot::new(&catalog, "Ada").expect("starts");
        pilot.escorts = vec![escort(129, false), escort(130, true)];
        let mut session = flying_with(&catalog, pilot);
        assert_eq!(session.npcs().len(), 4);
        let fleet: Vec<NpcId> = session
            .space
            .escorts
            .fleet
            .iter()
            .flatten()
            .copied()
            .collect();
        assert_eq!(fleet.len(), 2);
        moved(&mut session, &catalog, "M131");
        let left: Vec<NpcId> = session.npcs().iter().map(|npc| npc.id).collect();
        assert_eq!(left, fleet, "the others stay behind");
        assert_eq!(session.pilot().escorts().len(), 2, "the dry fighter too");
        assert_eq!(session.take_fighter_notes(), []);
        assert_eq!(
            session.space.traffic.ship_types(),
            [ShipId(129)],
            "131's table"
        );
        for _ in 0..3 {
            work_with(
                &mut session,
                &catalog,
                Controls::default(),
                &crate::ai::Peaceful,
                Rules::default(),
                &mut NeverFires,
            );
        }
        assert!(
            session.npcs().iter().all(|npc| fleet.contains(&npc.id)),
            "no initial population"
        );
    }

    /// The rules with `MoveLeftBehind` read as `reading`.
    fn left_behind(reading: RuleSource) -> Rulebook {
        Rulebook::default().with_override(RuleKey::MoveLeftBehind, reading)
    }

    /// The berths of `session`'s NPCs, in order.
    fn berths(session: &Session) -> Vec<u32> {
        session.npcs().iter().map(|npc| npc.berth).collect()
    }

    #[test]
    fn a_move_in_flight_holds_the_berths_left_behind_through_the_next_traffic_tick() {
        let catalog = moving();
        for (reading, berth) in [(RuleSource::Engine, 3), (RuleSource::Bible, 1)] {
            let mut session = flying(&catalog).with_rules(left_behind(reading));
            assert_eq!(berths(&session), [1, 2], "{reading:?}");
            moved(&mut session, &catalog, "M131");
            assert_eq!(session.npcs(), [], "{reading:?}");
            let mut chance = crate::testkit::Draws::of(&[0, 6, 6, 0, 0, 0]);
            work_with(
                &mut session,
                &catalog,
                Controls::default(),
                &crate::ai::Peaceful,
                Rules::default(),
                &mut chance,
            );
            assert_eq!(berths(&session), [berth], "{reading:?}");
            assert_eq!(
                session.space.traffic.berths_taken(),
                1,
                "{reading:?}: freed by the tick's end"
            );
        }
    }

    #[test]
    fn two_moves_in_one_settle_bring_the_ships_left_behind_back() {
        // The moves out and back owe one entry, from 130 into 130, so the
        // traffic is never left behind, under either reading.
        let catalog = moving();
        for reading in RuleSource::ALL {
            let mut session = flying(&catalog).with_rules(left_behind(reading));
            let before = session.npcs().to_vec();
            assert_eq!(before.len(), 2);
            moved(&mut session, &catalog, "M131 M130");
            assert_eq!(session.system(), SystemId(130), "{reading:?}");
            assert_eq!(session.npcs(), before, "{reading:?}");
        }
        // Settled in between, the move out leaves them behind first.
        for (reading, back) in [(RuleSource::Engine, true), (RuleSource::Bible, false)] {
            let mut session = flying(&catalog).with_rules(left_behind(reading));
            let before = session.npcs().to_vec();
            moved(&mut session, &catalog, "M131");
            moved(&mut session, &catalog, "M130");
            let expected = if back { before } else { Vec::new() };
            assert_eq!(session.npcs(), expected, "{reading:?}");
        }
    }

    #[test]
    fn a_move_back_after_a_traffic_tick_brings_none_back() {
        let catalog = moving();
        let mut session = flying(&catalog);
        moved(&mut session, &catalog, "M131");
        work_with(
            &mut session,
            &catalog,
            Controls::default(),
            &crate::ai::Peaceful,
            Rules::default(),
            &mut NeverFires,
        );
        moved(&mut session, &catalog, "M130");
        assert_eq!(session.npcs(), []);
    }

    #[test]
    fn as_an_arrival_a_move_in_flight_holds_nothing_behind() {
        let catalog = moving();
        let mut session = flying(&catalog).with_rules(arrival(RuleSource::Bible));
        moved(&mut session, &catalog, "M131");
        assert_eq!(session.npcs().len(), 3, "populated");
        assert_eq!(session.space.traffic.berths_taken(), 3, "none held");
        assert_eq!(berths(&session), [1, 2, 3]);
    }

    #[test]
    fn by_the_engine_a_move_within_the_system_keeps_its_traffic() {
        let catalog = moving();
        let mut session = flying(&catalog);
        let npcs = session.npcs().to_vec();
        moved(&mut session, &catalog, "N130");
        assert_eq!(session.npcs(), npcs);
    }

    #[test]
    fn by_the_engine_the_system_is_not_explored_and_the_course_kept() {
        let catalog = moving();
        let mut session = flying(&catalog);
        session.plot_course(SystemId(132)).expect("a route");
        moved(&mut session, &catalog, "M133");
        assert!(!session.pilot().has_explored(SystemId(133)));
        assert_eq!(session.course(), [SystemId(131), SystemId(132)]);
    }

    #[test]
    fn j_refuses_while_the_courses_next_system_is_not_linked_from_here() {
        let catalog = moving();
        let mut session = flying(&catalog);
        session.plot_course(SystemId(131)).expect("a route");
        moved(&mut session, &catalog, "M133");
        fly_out(&mut session, &catalog);
        assert_eq!(session.begin_jump(), Err(JumpRefusal::NoDestination));
        session.plot_course(SystemId(131)).expect("a route by 130");
        assert_eq!(session.course(), [SystemId(130), SystemId(131)]);
        assert_eq!(begin_jump_now(&mut session, &catalog), Ok(SystemId(130)));
    }

    #[test]
    fn as_an_arrival_the_move_explores_clears_the_course_and_populates() {
        let catalog = moving();
        let rules = Rulebook::default().with_override(RuleKey::MoveArrival, RuleSource::Bible);
        let mut pilot = Pilot::new(&catalog, "Ada").expect("starts");
        pilot.escorts = vec![escort(129, false), escort(130, true)];
        let mut session = flying_with(&catalog, pilot).with_rules(rules);
        session.plot_course(SystemId(132)).expect("a route");
        assert!(session.pilot().course_start().is_some());
        moved(&mut session, &catalog, "M131");
        assert!(session.pilot().has_explored(SystemId(131)));
        assert_eq!(session.course(), []);
        assert_eq!(session.pilot().course_start(), None, "the start cleared");
        assert_eq!(session.pilot().escorts().len(), 1, "the dry fighter left");
        assert_eq!(
            session.take_fighter_notes(),
            [crate::bay::FighterNote::Abandoned(1)]
        );
        assert_eq!(session.npcs().len(), 4, "three ships and the escort");
        assert_eq!(session.space.escorts.fleet.iter().flatten().count(), 1);
    }

    // What a move lets go of and keeps, by either reading of
    // `MoveArrival`.

    const READINGS: [RuleSource; 2] = [RuleSource::Engine, RuleSource::Bible];

    /// The rules with `MoveArrival` read as `reading`.
    fn arrival(reading: RuleSource) -> Rulebook {
        Rulebook::default().with_override(RuleKey::MoveArrival, reading)
    }

    #[test]
    fn a_move_never_arms_the_escort_strength_timer_by_either_reading() {
        let catalog = moving();
        for reading in READINGS {
            for text in ["M131", "N130"] {
                for start in [flying as fn(&FakePilotCatalog) -> Session, landed] {
                    let mut session = start(&catalog);
                    session = session.with_rules(arrival(reading));
                    run(&mut session, text);
                    let mut chance = crate::testkit::Draws::of(&[]);
                    let _ = session.settle_script(&catalog, &mut chance);
                    assert!(
                        !chance.asked.contains(&30),
                        "{reading:?} {text}: {:?}",
                        chance.asked
                    );
                }
            }
        }
    }

    #[test]
    fn a_move_in_flight_lets_go_of_the_boarding_and_the_hail_by_either_reading() {
        let catalog = moving();
        for reading in READINGS {
            for text in ["M131", "N130"] {
                let mut session = flying(&catalog).with_rules(arrival(reading));
                session.stage_scene(NpcId(0));
                moved(&mut session, &catalog, text);
                session.assert_scene_left(&format!("{text} {reading:?}"));
            }
        }
    }

    #[test]
    fn a_move_keeps_a_held_sound_and_sounds_and_raises_no_arrival() {
        let catalog = moving();
        for reading in READINGS {
            for text in ["M131", "N131"] {
                let case = format!("{text} {reading:?}");
                let mut session = flying(&catalog).with_rules(arrival(reading));
                session.take_sounds();
                session.take_messages();
                run(&mut session, "P300");
                moved(&mut session, &catalog, text);
                assert_eq!(session.take_sounds(), [], "in flight: {case}");
                assert_eq!(session.take_messages(), [], "in flight: {case}");
                work(&mut session, &catalog, crate::flight::Controls::default());
                assert_eq!(
                    session.take_sounds(),
                    [mission_sound(300, true)],
                    "in flight: {case}"
                );
                let mut session = landed(&catalog).with_rules(arrival(reading));
                session.take_messages();
                run(&mut session, "P300");
                moved(&mut session, &catalog, text);
                assert_eq!(session.take_sounds(), [], "landed: {case}");
                assert_eq!(session.take_messages(), [], "landed: {case}");
                session.take_off(&catalog, &mut NeverFires);
                assert_eq!(session.take_sounds(), [SimSound::TookOff], "{case}");
                work(&mut session, &catalog, crate::flight::Controls::default());
                assert_eq!(
                    session.take_sounds(),
                    [mission_sound(300, true)],
                    "landed: {case}"
                );
            }
        }
    }

    #[test]
    fn as_an_arrival_a_move_within_the_system_populates_it_afresh_on_the_chance_given() {
        let catalog = moving();
        let mut session = flying(&catalog).with_rules(arrival(RuleSource::Bible));
        let before: Vec<NpcId> = session.npcs().iter().map(|npc| npc.id).collect();
        run(&mut session, "N130");
        let mut chance = crate::testkit::Draws::default();
        let _ = session.settle_script(&catalog, &mut chance);
        let after: Vec<NpcId> = session.npcs().iter().map(|npc| npc.id).collect();
        assert_eq!(after.len(), 2);
        assert!(
            after.iter().all(|id| !before.contains(id)),
            "{before:?} {after:?}"
        );
        assert!(!chance.asked.is_empty(), "drawn on the chance given");
    }

    #[test]
    fn a_landed_move_leaves_the_traffic_and_the_fighters_out_by_either_reading() {
        let catalog = moving();
        for reading in READINGS {
            let mut pilot = Pilot::new(&catalog, "Ada").expect("starts");
            pilot.escorts = vec![escort(129, false), escort(130, true)];
            let mut session = flying_with(&catalog, pilot).with_rules(arrival(reading));
            land_now(&mut session).expect("lands");
            let npcs = session.npcs().to_vec();
            moved(&mut session, &catalog, "M131");
            assert_eq!(session.npcs(), npcs, "{reading:?}");
            assert_eq!(session.pilot().escorts().len(), 2, "{reading:?}");
            assert_eq!(session.take_fighter_notes(), [], "{reading:?}");
            assert_eq!(
                ids(&session.chart.sites),
                [128, 129],
                "the port's until the take-off: {reading:?}"
            );
            session.take_off(&catalog, &mut NeverFires);
            assert_eq!(ids(&session.chart.sites), [140, 141], "{reading:?}");
        }
    }

    // Landed.

    #[test]
    fn landed_m_moves_the_pilot_and_leaves_the_spaceport_as_it_is() {
        let catalog = moving();
        let mut session = landed(&catalog);
        assert_eq!(session.landed(), Some(StellarId(128)));
        session.plot_course(SystemId(131)).expect("a route");
        moved(&mut session, &catalog, "M131");
        assert_eq!(session.pilot().system(), SystemId(131));
        assert_eq!(session.pilot().stellar(), Some(StellarId(140)));
        assert_eq!(session.landed(), Some(StellarId(128)), "still docked");
        assert_eq!(ids(&session.chart.sites), [128, 129], "the port's stellars");
        assert_eq!(session.player().position, Vec2::new(100.0, 200.0));
        assert!(!session.pilot().has_explored(SystemId(131)));
        assert_eq!(session.course(), [SystemId(131)], "kept");
        assert!(session.take_save_due());
    }

    #[test]
    fn landed_m_onto_a_gate_or_into_a_system_with_no_stellar_leaves_the_pilot_on_none() {
        let catalog = moving();
        let mut session = landed(&catalog);
        moved(&mut session, &catalog, "M133");
        assert_eq!(session.pilot().stellar(), None, "no docking at a gate");
        assert_eq!(session.player().position, Vec2::new(300.0, 0.0));
        moved(&mut session, &catalog, "M131 M132");
        assert_eq!(session.pilot().stellar(), None);
        assert_eq!(session.player().position, Vec2::ZERO);
        assert_eq!(session.landed(), Some(StellarId(128)));
    }

    #[test]
    fn a_pilot_saved_after_a_landed_m_flies_again_on_the_new_stellar() {
        let catalog = moving();
        let mut session = landed(&catalog);
        moved(&mut session, &catalog, "M131");
        let saved = save::decode(&save::encode(session.pilot())).expect("loads");
        let flown = Session::fly(&catalog, saved).expect("flies");
        assert_eq!(flown.system(), SystemId(131));
        assert_eq!(crate::testkit::resumed_on(&flown), Some(StellarId(140)));
        assert_eq!(flown.player().position, Vec2::new(100.0, 200.0));
    }

    #[test]
    fn the_take_off_after_a_landed_m_is_in_the_new_system_at_its_first_stellar() {
        let catalog = moving();
        let mut session = landed(&catalog);
        moved(&mut session, &catalog, "M131");
        assert_eq!(
            session.take_off(&catalog, &mut NeverFires),
            Some(StellarId(128))
        );
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(ids(&session.chart.sites), [140, 141]);
        assert_eq!(session.player().position, Vec2::new(100.0, 200.0));
        assert!(session.pilot().has_explored(SystemId(131)), "explored");
        work_with(
            &mut session,
            &catalog,
            Controls::default(),
            &crate::ai::Peaceful,
            Rules::default(),
            &mut NeverFires,
        );
        assert_eq!(session.npcs().len(), 3, "131 populated");
        let mut session = landed(&catalog);
        moved(&mut session, &catalog, "M132");
        session.take_off(&catalog, &mut NeverFires);
        assert_eq!(session.player().position, Vec2::ZERO, "the centre");
        assert_eq!(session.chart.sites, []);
    }

    #[test]
    fn the_take_off_after_a_landed_n_is_from_where_the_ship_touched_down() {
        let catalog = moving();
        let mut session = flying(&catalog);
        session.ship.player.position = Vec2::new(10.0, -10.0);
        land_now(&mut session).expect("lands");
        assert_eq!(session.player().position, Vec2::new(30.0, -40.0), "docked");
        moved(&mut session, &catalog, "N131");
        assert_eq!(session.pilot().stellar(), None);
        assert_eq!(session.landed(), Some(StellarId(128)));
        session.take_off(&catalog, &mut NeverFires);
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(session.player().position, Vec2::new(10.0, -10.0));
        assert_eq!(ids(&session.chart.sites), [140, 141]);
        session.ship.player.position = Vec2::new(-45.0, 0.0);
        land_now(&mut session).expect("lands on 141");
        session.take_off(&catalog, &mut NeverFires);
        assert_eq!(
            session.player().position,
            Vec2::new(-50.0, 0.0),
            "the flag spent"
        );
    }

    #[test]
    fn a_relocation_after_a_landed_n_takes_off_from_the_stellar_moved_to() {
        let catalog = moving();
        let mut session = flying(&catalog);
        session.ship.player.position = Vec2::new(10.0, -10.0);
        land_now(&mut session).expect("lands");
        moved(&mut session, &catalog, "N131");
        session
            .relocate(&catalog, SystemId(131), StellarId(141))
            .expect("moves");
        session
            .chart
            .assert_docked_among_shown(session.mode.landed());
        session.take_off(&catalog, &mut NeverFires);
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(
            session.player().position,
            Vec2::new(-50.0, 0.0),
            "141's, not where the ship touched down"
        );
    }

    #[test]
    fn as_an_arrival_a_landed_move_explores_and_clears_the_course_at_once() {
        let catalog = moving();
        let rules = Rulebook::default().with_override(RuleKey::MoveArrival, RuleSource::Bible);
        let mut session = landed(&catalog).with_rules(rules);
        session.plot_course(SystemId(131)).expect("a route");
        assert!(session.pilot().course_start().is_some());
        moved(&mut session, &catalog, "N131");
        assert!(session.pilot().has_explored(SystemId(131)));
        assert_eq!(session.course(), []);
        assert_eq!(session.pilot().course_start(), None, "the start cleared");
        assert_eq!(session.landed(), Some(StellarId(128)));
    }

    #[test]
    fn a_relocation_after_a_landed_move_takes_off_where_it_docked() {
        let catalog = moving();
        let mut session = landed(&catalog);
        moved(&mut session, &catalog, "M133");
        session
            .relocate(&catalog, SystemId(131), StellarId(141))
            .expect("moves");
        session
            .chart
            .assert_docked_among_shown(session.mode.landed());
        session.take_off(&catalog, &mut NeverFires);
        assert_eq!(ids(&session.chart.sites), [140, 141]);
        assert_eq!(session.player().position, Vec2::new(-50.0, 0.0));
    }

    // The take-off's flag.

    /// The ship lands from (10, -10) on planet 128, after `text` ran in
    /// flight under `keep_flag`, and takes off: where it is then.
    fn taken_off_after(text: &str, keep_flag: RuleSource) -> Vec2 {
        taken_off_under(
            text,
            Rulebook::default().with_override(RuleKey::MoveKeepFlag, keep_flag),
        )
    }

    /// [`taken_off_after`] under the whole of `rules`.
    fn taken_off_under(text: &str, rules: Rulebook) -> Vec2 {
        let catalog = moving();
        let mut session = flying(&catalog).with_rules(rules);
        moved(&mut session, &catalog, text);
        session.ship.player.position = Vec2::new(10.0, -10.0);
        land_now(&mut session).expect("lands");
        session.take_off(&catalog, &mut NeverFires);
        session.player().position
    }

    #[test]
    fn by_the_engine_an_n_in_flight_leaves_the_take_off_as_any_other() {
        // `_PlayerLandOnStellar` clears the flag first (@0x631f1-0x631f6).
        assert_eq!(
            taken_off_after("N130", RuleSource::Engine),
            Vec2::new(30.0, -40.0)
        );
        assert_eq!(
            taken_off_under("N130", Rulebook::default()),
            Vec2::new(30.0, -40.0),
            "the default"
        );
    }

    #[test]
    fn by_the_other_reading_an_n_in_flight_keeps_the_next_take_offs_landing_position() {
        assert_eq!(
            taken_off_after("N130", RuleSource::Bible),
            Vec2::new(10.0, -10.0)
        );
        assert_eq!(
            taken_off_after("M130", RuleSource::Bible),
            Vec2::new(30.0, -40.0),
            "only N"
        );
    }

    #[test]
    fn a_landed_n_keeps_the_landing_position_by_either_reading() {
        for source in RuleSource::ALL {
            let catalog = moving();
            let rules = Rulebook::default().with_override(RuleKey::MoveKeepFlag, source);
            let mut session = flying(&catalog).with_rules(rules);
            session.ship.player.position = Vec2::new(10.0, -10.0);
            land_now(&mut session).expect("lands");
            moved(&mut session, &catalog, "N130");
            session.take_off(&catalog, &mut NeverFires);
            assert_eq!(
                session.player().position,
                Vec2::new(10.0, -10.0),
                "{source:?}"
            );
        }
    }

    // P.

    fn mission_sound(id: i16, exclusive: bool) -> SimSound {
        SimSound::Script {
            sound: SoundId(id),
            exclusive,
        }
    }

    #[test]
    fn by_the_engine_p_sounds_on_the_next_flight_tick() {
        let catalog = moving();
        let mut session = flying(&catalog);
        session.take_sounds();
        run(&mut session, "P300");
        assert_eq!(session.take_sounds(), [], "held");
        assert!(!session.take_save_due());
        work(&mut session, &catalog, crate::flight::Controls::default());
        assert_eq!(session.take_sounds(), [mission_sound(300, true)]);
        work(&mut session, &catalog, crate::flight::Controls::default());
        assert_eq!(session.take_sounds(), [], "once");
    }

    #[test]
    fn by_the_engine_p_waits_for_the_advance_pass_in_double_time() {
        let catalog = moving();
        let mut session = flying(&catalog);
        work(&mut session, &catalog, crate::flight::Controls::default());
        session.take_sounds();
        run(&mut session, "P300");
        session.tick_again(
            crate::flight::Controls::default(),
            &crate::ai::NovaAi::default(),
            crate::Rules::default(),
            &mut NeverFires,
        );
        assert_eq!(session.take_sounds(), [], "the second pass plays none");
        work(&mut session, &catalog, crate::flight::Controls::default());
        assert_eq!(session.take_sounds(), [mission_sound(300, true)]);
    }

    #[test]
    fn by_the_engine_only_the_last_p_before_the_tick_sounds() {
        let catalog = moving();
        let mut session = flying(&catalog);
        session.take_sounds();
        run(&mut session, "P300 P301");
        run(&mut session, "P302");
        work(&mut session, &catalog, crate::flight::Controls::default());
        assert_eq!(session.take_sounds(), [mission_sound(302, true)]);
    }

    #[test]
    fn by_the_engine_a_landed_p_sounds_on_the_first_tick_after_the_take_off() {
        let catalog = moving();
        let mut session = landed(&catalog);
        run(&mut session, "P300");
        work(&mut session, &catalog, crate::flight::Controls::default());
        assert_eq!(session.take_sounds(), [], "landed");
        session.take_off(&catalog, &mut NeverFires);
        assert_eq!(session.take_sounds(), [SimSound::TookOff]);
        work(&mut session, &catalog, crate::flight::Controls::default());
        assert_eq!(session.take_sounds(), [mission_sound(300, true)]);
    }

    #[test]
    fn by_the_engine_a_jumps_arrival_drops_a_held_sound() {
        let catalog = moving();
        let mut session = flying(&catalog);
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session, &catalog);
        begin_jump_now(&mut session, &catalog).expect("jumps");
        run(&mut session, "P300");
        session.arrive(&catalog, &mut NeverFires).expect("arrives");
        session.take_sounds();
        work(&mut session, &catalog, crate::flight::Controls::default());
        assert_eq!(session.take_sounds(), []);
    }

    #[test]
    fn by_the_other_reading_every_p_sounds_at_once_landed_or_not() {
        let catalog = moving();
        let rules = Rulebook::default().with_override(RuleKey::ScriptSound, RuleSource::Bible);
        let mut session = landed(&catalog).with_rules(rules);
        run(&mut session, "P300 P301");
        assert_eq!(
            session.take_sounds(),
            [mission_sound(300, false), mission_sound(301, false)]
        );
        session.take_off(&catalog, &mut NeverFires);
        session.take_sounds();
        work(&mut session, &catalog, crate::flight::Controls::default());
        assert_eq!(session.take_sounds(), [], "nothing held");
    }

    #[test]
    fn a_p_past_32767_drops_the_held_sound_by_the_engine() {
        let catalog = moving();
        let mut session = flying(&catalog);
        session.take_sounds();
        run(&mut session, "P200 P40000");
        work(&mut session, &catalog, crate::flight::Controls::default());
        assert_eq!(session.take_sounds(), [], "names no sound");
        let rules = Rulebook::default().with_override(RuleKey::ScriptSound, RuleSource::Bible);
        let mut session = flying(&catalog).with_rules(rules);
        session.take_sounds();
        run(&mut session, "P200 P40000");
        assert_eq!(session.take_sounds(), [mission_sound(200, false)]);
        run(&mut session, "P40000");
        work(&mut session, &catalog, crate::flight::Controls::default());
        assert_eq!(session.take_sounds(), [], "the Bible: nothing to sound");
    }

    // Q.

    /// String lists by ID.
    #[derive(Debug, Default)]
    struct Lists(Vec<(i16, Vec<&'static str>)>);

    impl CommCatalog for Lists {
        fn string_list(&self, id: i16) -> Vec<String> {
            self.0
                .iter()
                .find(|(list, _)| *list == id)
                .map_or_else(Vec::new, |(_, strings)| {
                    strings.iter().map(|&string| string.to_owned()).collect()
                })
        }
    }

    /// 25048 holds three messages, the last naming the player; 25049 is
    /// empty; 25050 holds an empty string and one that is not.
    fn told(session: Session) -> Session {
        session.with_strings(Rc::new(Lists(vec![
            (25048, vec!["Go.", "Leave now.", "Off you go, <PSN>."]),
            (25049, vec![]),
            (25050, vec!["", "Shoo."]),
        ])))
    }

    /// Runs `text`, rolling `rolls`, and gives the sides asked.
    fn run_rolling(session: &mut Session, text: &str, rolls: &[u16]) -> Vec<u16> {
        let mut chance = Scripted::rolling(rolls);
        session.run_set(&SetExpr::parse(text).expect("parses"), &mut chance);
        chance.sides_asked
    }

    /// A session landed on planet 128 of [`moving`]'s `catalog`, a `Q`
    /// run there holding its leave for the take-off: "Leave now.".
    pub(in crate::session) fn leave_held(catalog: &FakePilotCatalog) -> Session {
        let mut session = told(landed(catalog));
        run_rolling(&mut session, "Q25048", &[2]);
        assert!(session.leave_due(), "due once run");
        session
    }

    /// Takes off from wherever `session` is landed, on a chance that
    /// never fires.
    fn take_off(session: &mut Session, catalog: &FakePilotCatalog) -> Option<StellarId> {
        session.take_off(catalog, &mut NeverFires)
    }

    #[test]
    fn a_landed_q_is_held_for_the_take_off_and_passes_its_day_there() {
        let catalog = moving();
        let mut session = told(landed(&catalog));
        let landed_on = session.date();
        run_rolling(&mut session, "Q25048", &[2]);
        assert!(session.leave_due(), "due once run");
        assert_eq!(settle(&mut session, &catalog), Settled::default());
        assert_eq!(session.landed(), Some(StellarId(128)), "held");
        assert!(session.leave_due(), "still due");
        assert_eq!(session.date(), landed_on, "nothing yet");
        assert_eq!(session.take_sounds(), [], "no beep, no take-off");
        assert!(!session.take_save_due());
        assert_eq!(take_off(&mut session, &catalog), Some(StellarId(128)));
        assert_eq!(session.date(), landed_on.next_day());
        assert!(!session.leave_due());
    }

    #[test]
    fn the_take_off_gives_a_held_qs_message_once_with_no_beep() {
        let catalog = moving();
        let mut session = told(landed(&catalog));
        assert_eq!(run_rolling(&mut session, "Q25048", &[2]), [3]);
        let _ = settle(&mut session, &catalog);
        assert_eq!(
            session.take_leave_message(),
            None,
            "not before the take-off"
        );
        assert_eq!(take_off(&mut session, &catalog), Some(StellarId(128)));
        assert_eq!(
            session.take_leave_message().as_deref(),
            Some("Off you go, <PSN>.")
        );
        assert_eq!(session.take_leave_message(), None, "taken");
        assert_eq!(session.take_sounds(), [SimSound::TookOff], "no beep");
        assert!(session.take_save_due());
        assert_eq!(
            settle(&mut session, &catalog),
            Settled::default(),
            "consumed"
        );
        assert_eq!(session.take_sounds(), [], "nothing left for flight");
    }

    #[test]
    fn a_due_leave_takes_off_and_settles_the_take_offs_events_at_once() {
        let catalog = moving();
        let mut session = told(landed(&catalog));
        let landed_on = session.date();
        run_rolling(&mut session, "Q25048", &[2]);
        let _ = settle(&mut session, &catalog);
        session.run.drop_raised();
        assert_eq!(
            session.leave_if_due(&catalog, &mut NeverFires),
            Some(Departure {
                stellar: StellarId(128),
                settled: Settled::default(),
            })
        );
        assert_eq!(session.landed(), None, "in flight");
        assert_eq!(session.date(), landed_on.next_day(), "its day passed");
        assert_eq!(session.story_raised(), [], "DayPassed, TookOff dispatched");
        assert!(!session.leave_due());
        assert_eq!(
            session.take_leave_message().as_deref(),
            Some("Off you go, <PSN>.")
        );
        assert_eq!(
            session.leave_if_due(&catalog, &mut NeverFires),
            None,
            "once"
        );
    }

    #[test]
    fn with_no_leave_due_nothing_leaves() {
        let catalog = moving();
        let mut session = told(landed(&catalog));
        assert_eq!(session.leave_if_due(&catalog, &mut NeverFires), None);
        assert_eq!(session.landed(), Some(StellarId(128)), "docked");
        let mut session = told(flying(&catalog));
        run_rolling(&mut session, "Q25048", &[0]);
        assert_eq!(session.leave_if_due(&catalog, &mut NeverFires), None);
    }

    #[test]
    fn the_ports_leave_takes_off_and_settles_as_a_qs_does() {
        let catalog = moving();
        let mut session = told(landed(&catalog));
        session.run.drop_raised();
        let departure = session
            .leave_port(&catalog, &mut NeverFires)
            .expect("leaves");
        assert_eq!(departure.stellar, StellarId(128));
        assert_eq!(session.landed(), None);
        assert_eq!(session.story_raised(), [], "settled");
        assert_eq!(
            session.leave_port(&catalog, &mut NeverFires),
            None,
            "in flight"
        );
    }

    #[test]
    fn a_take_off_with_no_q_gives_no_message() {
        let catalog = moving();
        let mut session = told(landed(&catalog));
        assert_eq!(take_off(&mut session, &catalog), Some(StellarId(128)));
        assert_eq!(session.take_leave_message(), None);
    }

    #[test]
    fn a_landing_drops_a_leave_still_pending_from_flight() {
        let catalog = moving();
        let mut session = told(flying(&catalog));
        run_rolling(&mut session, "Q25048", &[0]);
        land_now(&mut session).expect("lands");
        assert!(!session.leave_due(), "the landing clears it");
        assert_eq!(settle(&mut session, &catalog).message, None);
        assert_eq!(take_off(&mut session, &catalog), Some(StellarId(128)));
        assert_eq!(session.take_leave_message(), None);
    }

    #[test]
    fn a_q_in_flight_shows_its_message_with_a_beep() {
        let catalog = moving();
        let mut session = told(flying(&catalog));
        session.take_sounds();
        run_rolling(&mut session, "Q25048", &[0]);
        assert!(!session.leave_due(), "in flight nothing is held");
        let settled = settle(&mut session, &catalog);
        assert_eq!(settled.message.as_deref(), Some("Go."));
        assert_eq!(session.take_sounds(), [SimSound::ScriptMessage]);
        assert_eq!(session.landed(), None);
        assert_eq!(settle(&mut session, &catalog), Settled::default(), "once");
    }

    #[test]
    fn by_the_engine_a_blank_q_does_nothing_and_cancels_an_earlier_one() {
        let catalog = moving();
        for (text, rolls, sides) in [
            ("Q25051", &[][..], &[][..]),
            ("Q25049", &[], &[]),
            ("Q25050", &[0], &[2]),
            ("Q25048 Q25049", &[1], &[3]),
        ] {
            let mut session = told(landed(&catalog));
            assert_eq!(run_rolling(&mut session, text, rolls), sides, "{text}");
            assert!(!session.leave_due(), "{text}");
            assert_eq!(settle(&mut session, &catalog), Settled::default(), "{text}");
            assert_eq!(session.landed(), Some(StellarId(128)), "{text}");
            assert_eq!(session.take_sounds(), [], "{text}");
            let mut session = told(flying(&catalog));
            session.take_sounds();
            run_rolling(&mut session, text, rolls);
            assert_eq!(settle(&mut session, &catalog), Settled::default(), "{text}");
            assert_eq!(session.take_sounds(), [], "{text}");
        }
    }

    #[test]
    fn a_later_blank_q_cancels_a_held_leave() {
        let catalog = moving();
        let mut session = told(landed(&catalog));
        run_rolling(&mut session, "Q25048", &[0]);
        let _ = settle(&mut session, &catalog);
        assert!(session.leave_due());
        run_rolling(&mut session, "Q25049", &[]);
        let _ = settle(&mut session, &catalog);
        assert!(!session.leave_due(), "cancelled");
        assert_eq!(take_off(&mut session, &catalog), Some(StellarId(128)));
        assert_eq!(session.take_leave_message(), None);
    }

    #[test]
    fn by_the_bible_a_blank_q_leaves_all_the_same_with_no_message() {
        let catalog = moving();
        let rules = Rulebook::default().with_override(RuleKey::BlankLeave, RuleSource::Bible);
        let mut session = told(landed(&catalog)).with_rules(rules);
        run_rolling(&mut session, "Q25049", &[]);
        assert_eq!(settle(&mut session, &catalog), Settled::default());
        assert!(session.leave_due(), "held all the same");
        assert_eq!(take_off(&mut session, &catalog), Some(StellarId(128)));
        assert_eq!(session.take_leave_message(), None, "no message");
        let mut session = told(flying(&catalog)).with_rules(rules);
        session.take_sounds();
        run_rolling(&mut session, "Q25050", &[0]);
        assert_eq!(settle(&mut session, &catalog), Settled::default());
        assert_eq!(session.take_sounds(), [], "nothing to show");
        let mut session = told(landed(&catalog)).with_rules(rules);
        run_rolling(&mut session, "Q25048", &[1]);
        let _ = settle(&mut session, &catalog);
        let _ = take_off(&mut session, &catalog);
        assert_eq!(
            session.take_leave_message().as_deref(),
            Some("Leave now."),
            "as usual"
        );
    }

    #[test]
    fn a_q_past_32767_is_a_blank_q() {
        let catalog = moving();
        for (reading, due) in [(RuleSource::Engine, false), (RuleSource::Bible, true)] {
            let rules = Rulebook::default().with_override(RuleKey::BlankLeave, reading);
            let mut session = told(landed(&catalog)).with_rules(rules);
            assert_eq!(run_rolling(&mut session, "Q25048 Q40000", &[0]), [3]);
            let _ = settle(&mut session, &catalog);
            assert_eq!(session.leave_due(), due, "{reading:?}");
            if due {
                assert_eq!(take_off(&mut session, &catalog), Some(StellarId(128)));
                assert_eq!(session.take_leave_message(), None, "{reading:?}");
            }
            let mut session = told(flying(&catalog)).with_rules(rules);
            session.take_sounds();
            run_rolling(&mut session, "Q25048 Q40000", &[0]);
            assert_eq!(settle(&mut session, &catalog), Settled::default());
            assert_eq!(session.take_sounds(), [], "{reading:?}: no message");
        }
    }

    #[test]
    fn a_q_take_off_with_a_message_neither_greets_nor_moves_the_tutorial() {
        let catalog = moving();
        for tutorial in [Tutorial::SEASONED, Tutorial::NEW_PILOT] {
            let mut session = told(landed(&catalog));
            session.tutorial = tutorial;
            run_rolling(&mut session, "Q25048", &[0]);
            let _ = settle(&mut session, &catalog);
            assert!(session.leave_due(), "held for the take-off");
            assert_eq!(take_off(&mut session, &catalog), Some(StellarId(128)));
            assert_eq!(session.take_messages(), [], "{tutorial:?}");
            assert_eq!(session.tutorial(), tutorial);
            assert!(session.take_leave_message().is_some(), "{tutorial:?}");
        }
    }

    /// By the engine a blank `Q` never takes off (see
    /// [`by_the_engine_a_blank_q_does_nothing_and_cancels_an_earlier_one`]),
    /// so only by [`RuleKey::BlankLeave`]'s other reading does a `Q`'s
    /// take-off with no message reach the greeting.
    #[test]
    fn a_q_take_off_with_an_empty_message_greets() {
        let catalog = moving();
        let rules = Rulebook::default().with_override(RuleKey::BlankLeave, RuleSource::Bible);
        let mut session = told(landed(&catalog)).with_rules(rules);
        run_rolling(&mut session, "Q25049", &[]);
        let _ = settle(&mut session, &catalog);
        assert!(session.leave_due(), "held for the take-off");
        assert_eq!(take_off(&mut session, &catalog), Some(StellarId(128)));
        assert_eq!(session.take_leave_message(), None);
        assert_eq!(
            session.take_messages(),
            [SimMessage::TookOff {
                stellar: StellarId(128),
                lead: 0
            }]
        );
        let mut session = told(landed(&catalog)).with_rules(rules);
        session.tutorial = Tutorial::NEW_PILOT;
        run_rolling(&mut session, "Q25049", &[]);
        let _ = settle(&mut session, &catalog);
        assert_eq!(take_off(&mut session, &catalog), Some(StellarId(128)));
        assert_eq!(session.take_messages(), [], "a newbie is not greeted");
        assert_eq!(session.tutorial().flag(), -1);
    }

    #[test]
    fn a_landed_move_and_q_take_off_in_the_system_moved_to() {
        let catalog = moving();
        let mut session = told(landed(&catalog));
        run_rolling(&mut session, "M131 Q25048", &[0]);
        let settled = settle(&mut session, &catalog);
        assert_eq!(settled.moved, Some(SystemId(131)));
        assert!(session.leave_due());
        assert_eq!(take_off(&mut session, &catalog), Some(StellarId(128)));
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(ids(&session.chart.sites), [140, 141]);
        assert_eq!(session.player().position, Vec2::new(100.0, 200.0));
        assert_eq!(session.npcs().len(), 3, "131's ships");
    }

    // An out-of-range `N`, and every other out-of-range operator.

    /// The rules with `OutOfRangeMove` read as `reading`.
    fn out_of_range(reading: RuleSource) -> Rulebook {
        Rulebook::default().with_override(RuleKey::OutOfRangeMove, reading)
    }

    /// Runs `text` and settles its moves alone, the story events raised
    /// left undispatched: those raised.
    fn stayed(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        text: &str,
    ) -> Vec<crate::story::StoryEvent> {
        session.run.drop_raised();
        run(session, text);
        session.settle_moves(catalog, &mut NeverFires, &mut Settled::default());
        session
            .chart
            .assert_docked_among_shown(session.mode.landed());
        session.story_raised()
    }

    #[test]
    fn by_the_engine_a_landed_out_of_range_n_only_empties_the_offers() {
        let catalog = moving();
        for text in ["N5", "N2176", "n70000"] {
            let mut session = landed(&catalog);
            session.plot_course(SystemId(131)).expect("a route");
            let _ = settle(&mut session, &catalog);
            session.take_events();
            let explored = session.pilot().explored.clone();
            let course = session.course().to_vec();
            let raised = stayed(&mut session, &catalog, text);
            assert_eq!(raised, [], "{text}: no enter-system hook");
            assert_eq!(session.system(), SystemId(130), "{text}");
            assert_eq!(session.landed(), Some(StellarId(128)), "{text}");
            assert_eq!(session.pilot().stellar, Some(StellarId(128)), "{text}");
            assert_eq!(session.pilot().explored, explored, "{text}");
            assert_eq!(session.course(), course, "{text}");
            assert_eq!(session.take_events(), [], "{text}");
            assert!(!session.port.hold_position, "{text}");
        }
    }

    #[test]
    fn by_the_engine_an_out_of_range_n_in_flight_enters_the_system_again_where_it_is() {
        let catalog = moving();
        let mut session = flying(&catalog);
        session.pilot.stellar = Some(StellarId(129));
        session.hold_fire(true, false);
        work(&mut session, &catalog, Controls::default());
        assert_eq!(session.shots().len(), 1);
        session.ship.player.position = Vec2::new(10.0, 10.0);
        session.ship.player.velocity = Vec2::new(5.0, -1.0);
        session.select_stellar(crate::session::StellarPick::Slot(0));
        session.chart.target = Some(NpcId(0));
        session.ship.warp.gate = Some(StellarId(128));
        session.space.strikes = session.space.combat.take_strikes();
        session.stage_scene(NpcId(0));
        session.take_events();
        let npcs = session.npcs().to_vec();
        let position = session.player().position;
        let velocity = session.player().velocity;
        let raised = stayed(&mut session, &catalog, "N9999");
        assert_eq!(raised, [], "its hook dispatched at the operator (@0x159a8)");
        assert_eq!(session.system(), SystemId(130));
        assert_eq!(session.player().position, position);
        assert_eq!(session.player().velocity, velocity);
        assert!(!session.port.hold_position, "no take-off hold");
        assert_eq!(session.pilot().stellar, Some(StellarId(129)), "kept");
        assert_eq!(session.shots(), []);
        assert_eq!(session.nav_target(), None);
        assert_eq!(session.chart.target, None);
        assert_eq!(session.ship.warp.gate, None);
        assert_eq!(session.space.strikes, []);
        session.assert_scene_left("N9999");
        assert_eq!(session.npcs(), npcs, "the traffic kept");
        assert_eq!(session.take_events(), [SessionEvent::SystemChanged]);
        assert!(session.take_save_due());
    }

    #[test]
    fn by_the_engine_a_move_to_a_missing_system_in_flight_enters_the_system_again_where_it_is() {
        let catalog = moving();
        for text in ["M150", "N999"] {
            let mut session = flying(&catalog);
            session.pilot.stellar = Some(StellarId(129));
            session.hold_fire(true, false);
            work(&mut session, &catalog, Controls::default());
            assert_eq!(session.shots().len(), 1, "{text}");
            session.ship.player.position = Vec2::new(10.0, 10.0);
            session.ship.player.velocity = Vec2::new(5.0, -1.0);
            session.stage_scene(NpcId(0));
            session.take_events();
            session.run.drop_raised();
            run(&mut session, text);
            let mut settled = Settled::default();
            session.settle_moves(&catalog, &mut NeverFires, &mut settled);
            session
                .chart
                .assert_docked_among_shown(session.mode.landed());
            assert_eq!(
                session.story_raised(),
                [],
                "{text}: its hook dispatched at the operator (@0x159a8)"
            );
            assert_eq!(settled.moved, None, "{text}: moved nowhere");
            assert_eq!(session.system(), SystemId(130), "{text}");
            assert_eq!(session.player().position, Vec2::new(10.0, 10.0), "{text}");
            assert_eq!(session.player().velocity, Vec2::new(5.0, -1.0), "{text}");
            assert!(!session.port.hold_position, "{text}: no take-off hold");
            assert_eq!(session.pilot().stellar, Some(StellarId(129)), "{text}");
            assert_eq!(session.shots(), [], "{text}");
            session.assert_scene_left(text);
            assert_eq!(
                session.take_events(),
                [SessionEvent::SystemChanged],
                "{text}"
            );
            assert!(session.take_save_due(), "{text}");
        }
    }

    #[test]
    fn by_the_engine_a_landed_move_to_a_missing_system_changes_only_the_offers() {
        let catalog = moving();
        for text in ["M1500", "N1500"] {
            let mut session = landed(&catalog);
            let _ = settle(&mut session, &catalog);
            session.take_events();
            let raised = stayed(&mut session, &catalog, text);
            assert_eq!(raised, [], "{text}: no enter-system hook");
            assert_eq!(session.system(), SystemId(130), "{text}");
            assert_eq!(session.landed(), Some(StellarId(128)), "{text}");
            assert_eq!(session.pilot().stellar, Some(StellarId(128)), "{text}");
            assert_eq!(session.take_events(), [], "{text}");
            assert!(!session.port.hold_position, "{text}");
        }
    }

    #[test]
    fn as_an_arrival_an_out_of_range_n_in_flight_is_still_no_arrival() {
        let catalog = moving();
        let mut pilot = Pilot::new(&catalog, "Ada").expect("starts");
        pilot.escorts = vec![escort(129, false), escort(130, true)];
        let mut session = flying_with(&catalog, pilot).with_rules(arrival(RuleSource::Bible));
        session.plot_course(SystemId(132)).expect("a route");
        let explored = session.pilot().explored.clone();
        let course = session.course().to_vec();
        let npcs: Vec<NpcId> = session.npcs().iter().map(|npc| npc.id).collect();
        session.run.drop_raised();
        run(&mut session, "N9999");
        let mut chance = crate::testkit::Draws::default();
        session.settle_moves(&catalog, &mut chance, &mut Settled::default());
        assert_eq!(session.pilot().explored, explored, "not explored");
        assert_eq!(session.course(), course, "the course kept");
        assert_eq!(session.pilot().escorts().len(), 2, "the dry fighter kept");
        assert_eq!(session.take_fighter_notes(), []);
        let after: Vec<NpcId> = session.npcs().iter().map(|npc| npc.id).collect();
        assert_eq!(after, npcs, "not populated");
        assert_eq!(chance.asked, Vec::<u32>::new(), "nothing drawn");
        assert!(
            !session
                .story_raised()
                .iter()
                .any(|event| matches!(event, crate::story::StoryEvent::Explored { .. })),
        );
    }

    #[test]
    fn by_the_other_reading_an_out_of_range_n_does_nothing() {
        let catalog = moving();
        for start in [flying, landed] {
            let mut session = start(&catalog);
            session = session.with_rules(out_of_range(RuleSource::Bible));
            let _ = settle(&mut session, &catalog);
            session.take_events();
            let before = session.clone();
            run(&mut session, "N9999");
            assert_eq!(settle(&mut session, &catalog), Settled::default());
            assert_eq!(session, before);
            assert!(!session.take_save_due());
        }
    }

    #[test]
    fn every_other_out_of_range_operator_does_nothing_and_the_rest_runs() {
        let catalog = moving();
        for reading in READINGS {
            for start in [flying, landed] {
                let mut session = start(&catalog);
                session = session.with_rules(out_of_range(reading));
                let _ = settle(&mut session, &catalog);
                session.take_events();
                let mut expected = session.clone();
                run(&mut expected, "b7");
                let _ = settle(&mut expected, &catalog);
                run(
                    &mut session,
                    "M5 M2176 A5 F5 S5 G5 D5 C5 E5 H5 K5 L5 Y5 U5 X5 b10000 !b10000 ^b10000 b7",
                );
                assert_eq!(settle(&mut session, &catalog), Settled::default());
                assert_eq!(session, expected, "{reading:?}");
            }
        }
    }

    // Events.

    #[test]
    fn a_move_in_flight_tells_the_system_changed_even_within_the_system() {
        let catalog = moving();
        for text in ["M131", "N130", "M130"] {
            let mut session = flying(&catalog);
            moved(&mut session, &catalog, text);
            assert_eq!(
                session.take_events(),
                [SessionEvent::SystemChanged],
                "{text}"
            );
        }
        let mut session = flying(&catalog);
        moved(&mut session, &catalog, "M999");
        assert_eq!(
            session.take_events(),
            [SessionEvent::SystemChanged],
            "no such system: entered again by the engine"
        );
        let mut session = flying(&catalog).with_rules(missing_system(RuleSource::Bible));
        moved(&mut session, &catalog, "M999");
        assert_eq!(
            session.take_events(),
            [],
            "no such system: the other reading"
        );
    }

    #[test]
    fn a_landed_move_to_another_system_tells_it_at_the_take_off() {
        let catalog = moving();
        let mut session = landed(&catalog);
        moved(&mut session, &catalog, "M131");
        assert_eq!(session.take_events(), [], "the stellars wait");
        session.take_off(&catalog, &mut NeverFires);
        assert_eq!(session.take_events(), [SessionEvent::SystemChanged]);
    }

    #[test]
    fn a_landed_move_within_the_system_or_back_to_it_tells_nothing() {
        let catalog = moving();
        for text in ["N130", "M130", "M131 M130"] {
            let mut session = landed(&catalog);
            moved(&mut session, &catalog, text);
            assert_eq!(session.take_events(), [], "{text}: settled");
            session.take_off(&catalog, &mut NeverFires);
            assert_eq!(session.take_events(), [], "{text}: taken off");
        }
        let mut session = landed(&catalog);
        session.take_off(&catalog, &mut NeverFires);
        assert_eq!(session.take_events(), [], "a plain take-off");
    }

    #[test]
    fn a_landed_move_and_q_tell_the_system_changed_at_the_take_off() {
        let catalog = moving();
        let mut session = told(landed(&catalog));
        run_rolling(&mut session, "M131 Q25048", &[0]);
        let _ = settle(&mut session, &catalog);
        assert_eq!(session.take_events(), [], "held");
        let _ = take_off(&mut session, &catalog);
        assert_eq!(session.take_events(), [SessionEvent::SystemChanged]);
    }

    #[test]
    fn without_string_lists_a_q_does_nothing() {
        let catalog = moving();
        let mut session = landed(&catalog);
        assert_eq!(run_rolling(&mut session, "Q25048", &[]), Vec::<u16>::new());
        assert_eq!(settle(&mut session, &catalog), Settled::default());
        assert_eq!(session.landed(), Some(StellarId(128)));
    }
}
