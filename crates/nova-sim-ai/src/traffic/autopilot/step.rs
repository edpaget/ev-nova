//! The autopilot's step half: `_HandleShip`'s own work at the ship's step
//! of the ship loop (@0x44483), which flies the word its low level left
//! at its slot ([`SlotWord`]) and decides nothing (see
//! [`slot_word`](crate::traffic::slot_word)). It runs the immobile zero
//! and undo (@0x34209-0x3423b), the flights a ship has no say in (the
//! gate's waits, glide in and burst out, the coast after them, a
//! disabled ship's drift, [`unsteered`]), which ignore the word, the AI
//! wait's skip of the turn and thrust (@0x34a54-0x34a72), and otherwise
//! the turn and thrust of the word ([`slot_word::handle_ship`]).

use super::{Outcome, POST_BURST_SPREAD, POST_BURST_WAIT, TopSpeeds};
use crate::catalog::StellarId;
use crate::chance::Chance;
use crate::combat::armament::Trigger;
use crate::combat::hull::Condition;
use crate::flight::{self, Controls, FlightRules, Flyer, Turn};
use crate::geometry::Vec2;
use crate::handling::Handling;
use crate::rulebook::RuleSource;
use crate::traffic::emerge::exit_speed;
use crate::traffic::inertialess::{self, Inertia};
use crate::traffic::npc::{Mode, Npc};
use crate::traffic::pod_flight;
use crate::traffic::ship_display;
use crate::traffic::slot_word::{self, AtStep, SlotWord};
use crate::traffic::spawn::glide_speed;

/// `_HandleShip`'s flight of `npc` at its step by the `word` its low level
/// left at its slot (see the module docs), with what it reads `at_step`,
/// at `speeds`, under the flight rules, a hold at its gate that runs out
/// owing its burst as the reading of
/// [`RuleKey::HeldGateBurst`](crate::RuleKey::HeldGateBurst) says
/// (`flight`, `held_burst`), the wait a leaderless ship takes after its
/// glide or burst rolled on `chance`. What became of it.
///
/// An [`immobile`](Npc::immobile) ship's velocity is set to none before
/// it flies and its move is undone after, as `_HandleShip` zeroes it and
/// skips the move (@0x34209-0x3423b).
pub fn step(
    npc: &mut Npc,
    word: &SlotWord,
    at_step: AtStep,
    speeds: TopSpeeds,
    (flight, held_burst): (FlightRules, RuleSource),
    chance: &mut (impl Chance + ?Sized),
) -> Outcome {
    let by = (flight, held_burst);
    if !npc.immobile {
        return step_mobile(npc, word, at_step, speeds, by, chance);
    }
    let at = npc.state.position;
    npc.state.velocity = Vec2::ZERO;
    let outcome = step_mobile(npc, word, at_step, speeds, by, chance);
    npc.state.position = at;
    outcome
}

/// [`step`] for a ship that may move.
fn step_mobile(
    npc: &mut Npc,
    word: &SlotWord,
    at_step: AtStep,
    speeds: TopSpeeds,
    (flight, held_burst): (FlightRules, RuleSource),
    chance: &mut (impl Chance + ?Sized),
) -> Outcome {
    let inertia = word.inertia();
    if pod_flight::is_pod(npc) {
        // The word `_AIDispatch` wrote at its slot (@0x8fb62-0x8fbba), its
        // turn skipped while its wait runs (@0x3443f-0x34446).
        let turn = match word.wanted() {
            Some(wanted) if pod_flight::turns(npc) => {
                flight::turn_to(&mut npc.state, wanted, at_step.turn_rate)
            }
            _ => Turn::None,
        };
        slot_word::fly_word(npc, word, turn, at_step, flight);
        return Outcome::Flying;
    }
    let unthrust = at_step.unthrust();
    let unsteered_by = (speeds, flight, held_burst);
    if let Some(outcome) = unsteered(npc, &unthrust, unsteered_by, inertia, chance) {
        return outcome;
    }
    slot_word::handle_ship(npc, word, at_step, flight);
    Outcome::Flying
}

/// Flies `npc` one tick under `controls` as `inertia` says, an inertialess
/// one's velocity pulled by `adjust` (`_AdjustInertialessShipVelocity`'s
/// own `_ShipAccelRate`, @0x334e1): an inertial ship's [`flight::step`],
/// an inertialess one's [`flight::inertialess_step_with`] (see
/// [`inertialess`]), its drive recorded ([`ship_display::record`]). What it
/// owes of a gate burst is left: a flight whose substate writes the burst
/// word +0x34 flies by [`held_burst::step`](crate::traffic::held_burst::step)
/// under `Word::Writes`, which drops it.
pub(crate) fn plain_step(
    npc: &mut Npc,
    handling: &Handling,
    controls: Controls,
    flight: FlightRules,
    (inertia, adjust): (Inertia, f32),
) {
    ship_display::record(npc, handling, controls, flight);
    match inertia {
        Inertia::Inertial => flight::step(&mut npc.state, handling, controls, flight, Flyer::Npc),
        Inertia::Inertialess { .. } => {
            flight::inertialess_step_with(
                &mut npc.state,
                &mut npc.speed,
                handling,
                controls,
                adjust,
                flight,
            );
        }
    }
}

/// Tick `tick` of `npc`'s glide in from hyperspace: along its heading at
/// [`glide_speed`], ending as [`burst`] says.
///
/// `_HandleShip` runs the glide (+0x34 set to -50.0 and +0x30 to -1.165 by
/// low mode 10, @0x88114-0x88136) through the same negative-speed path as
/// the burst out of a gate (@0x34bc0-0x34d32): it ends against the same
/// cap, the lower of its own and its lead's top speed (@0x34c68-0x34d0d),
/// once the next speed would reach it (@0x34d10-0x34d2a), and leaves the
/// velocity at the last speed, which lies within 1.165 above the cap.
fn glide(
    npc: &mut Npc,
    tick: u32,
    speeds: TopSpeeds,
    flown: Flown,
    chance: &mut (impl Chance + ?Sized),
) {
    if !burst(
        npc,
        glide_speed(tick),
        glide_speed(tick + 1),
        speeds,
        flown,
        chance,
    ) {
        npc.mode = Mode::JumpingIn { tick: tick + 1 };
    }
}

/// Flies `npc` one tick when it has no say in its flight: entering a
/// gate or waiting at one, drifting while `_IsDisabled` holds or it is
/// destroyed (`_HandleShip` reads `_IsDisabled` once, @0x33846, for its
/// drag @0x33950 and its drift @0x34424, so a stellar's defence ship
/// breaking up still steers, [`Npc::active_not_disabled`]), gliding in from
/// hyperspace (intact or disabled), bursting out of a gate, or holding
/// its course after either (see the module docs of
/// [`autopilot`](super)), on `handling` (its turn rate and top speed at
/// its step, and the acceleration its inertialess adjustment reads) under
/// `flight`, as `inertia` says, its burst ending at `speeds`'
/// [`burst_cap`](TopSpeeds::burst_cap) and its wait rolled on `chance`; a
/// hold at its gate that runs out owes its burst as `held_burst` reads
/// ([`Npc::end_wait`]).
/// An intact glide or burst has its heading truncated
/// ([`flight::turn_to_own`]) before it moves, as sub 0xa leaves the low
/// level's `cvttss2si(+0x44)` aim (@0x85228-0x8522d) and `_HandleShip`
/// turns onto it ahead of the move (@0x34424 → @0x34bc0); a disabled one,
/// which skips that turn, and every wait keep theirs.
/// What became of it, if it did. A disabled or breaking-up ship's speed
/// along its heading slows with its velocity, inertialess or not
/// (@0x3398f-0x339a1).
pub(super) fn unsteered(
    npc: &mut Npc,
    handling: &Handling,
    (speeds, flight, held_burst): (TopSpeeds, FlightRules, RuleSource),
    inertia: Inertia,
    chance: &mut (impl Chance + ?Sized),
) -> Option<Outcome> {
    let flown = Flown { handling, inertia };
    match (npc.mode, npc.active_not_disabled()) {
        (Mode::EnteringGate { gate, ticks_left }, _) => {
            return Some(enter_gate(npc, gate, ticks_left, flown));
        }
        (Mode::Emerging { ticks_left }, _) => wait_at_gate(npc, ticks_left, flown),
        (Mode::HeldAtGate { ticks_left }, _) => {
            hold_at_gate(npc, ticks_left, flown, held_burst);
        }
        (Mode::JumpingIn { tick }, false) if npc.condition == Condition::Disabled => {
            glide(npc, tick, speeds, flown, chance);
        }
        (mode, false) => {
            match mode {
                Mode::LeavingGate { .. } => {
                    npc.mode = Mode::Flying;
                    npc.emerges_from = None;
                }
                // `_HandleShip` counts +0x4c down for a disabled ship too
                // (@0x34424 → @0x34679 → @0x34a54 → @0x35026).
                Mode::Coasting { ticks_left } => count_down(npc, ticks_left),
                _ => {}
            }
            disabled_drag(npc);
            // No thrust: it writes neither word.
            plain_step(
                npc,
                handling,
                Controls::default(),
                flight,
                (inertia, handling.accel),
            );
        }
        (Mode::JumpingIn { tick }, true) => {
            flight::turn_to_own(&mut npc.state);
            glide(npc, tick, speeds, flown, chance);
        }
        (Mode::LeavingGate { tick }, true) => {
            flight::turn_to_own(&mut npc.state);
            leave_gate(npc, tick, speeds, flown, chance);
        }
        (Mode::Coasting { ticks_left }, true) => coast(npc, ticks_left, flown),
        (Mode::Flying | Mode::Adrift { .. }, true) => return None,
    }
    Some(Outcome::Flying)
}

/// Whether `npc` has a say in its flight: flying or adrift, active and not
/// disabled. Any other ship's flight is `_HandleShip`'s own
/// ([`unsteered`]), which reads no word.
#[must_use]
pub(super) fn steered(npc: &Npc) -> bool {
    matches!(npc.mode, Mode::Flying | Mode::Adrift { .. }) && npc.active_not_disabled()
}

/// One tick of `npc` entering `gate`, `ticks_left` of its wait left
/// (see [`gate_entry`](crate::traffic::gate_entry)): still, whatever its
/// condition and goal, as `_HandleShip` counts +0x4c down for every ship
/// and the fade reads only the sub-goal; and gone once the wait is out. An
/// inertialess ship's velocity follows its speed meanwhile, as `flown`
/// says ([`inertialess::drift`]).
fn enter_gate(npc: &mut Npc, gate: StellarId, ticks_left: u32, flown: Flown) -> Outcome {
    flown.hold(npc);
    let left = ticks_left.saturating_sub(1);
    if left == 0 {
        return Outcome::EnteredGate(gate);
    }
    npc.mode = Mode::EnteringGate {
        gate,
        ticks_left: left,
    };
    Outcome::Flying
}

/// A disabled or breaking-up `npc`'s drag before it moves: its velocity,
/// and its speed along its heading with it, inertialess or not, slowed by
/// [`flight::DISABLED_DRAG`] (@0x33950-0x339a1, @0x3398f-0x339a1). Any
/// other is left, a stellar's defence ship breaking up included, as
/// `_IsDisabled` never calls it disabled ([`Npc::is_disabled`], @0x2d6f).
pub(super) fn disabled_drag(npc: &mut Npc) {
    if npc.is_disabled() {
        flight::drag(&mut npc.state);
        npc.speed *= flight::DISABLED_DRAG;
    }
}

/// `_HandleShip`'s move of `npc` while it waits in or at its gate, as
/// `flown` says: dragged when disabled ([`disabled_drag`]), then moved on
/// its velocity ([`position_step`], @0x34274-0x342aa). Nothing in the
/// wait zeroes the velocity, so a hit's push (`_DamageShip`
/// @0x3aa7d, its +0x50 -1.0 there) sets it drifting.
fn drift_in_wait(npc: &mut Npc, flown: Flown) {
    disabled_drag(npc);
    position_step(npc, flown.handling, flown.inertia);
}

/// One tick of `npc`'s wait at the gate it comes out of, `ticks_left` of
/// it left: moving on its velocity ([`drift_in_wait`]), and bursting out
/// once it is over, its trigger (the point defence it may hold while it
/// waits) released, as the burst's sub-goal 0xa makes no
/// `_AIEscortFireUnprovoked` call (@0x8f3d9).
fn wait_at_gate(npc: &mut Npc, ticks_left: u32, flown: Flown) {
    drift_in_wait(npc, flown);
    let left = ticks_left.saturating_sub(1);
    npc.mode = if left == 0 {
        npc.trigger = Trigger::default();
        Mode::LeavingGate { tick: 0 }
    } else {
        Mode::Emerging { ticks_left: left }
    };
}

/// One tick of `npc` held at its gate ([`Mode::HeldAtGate`]),
/// `ticks_left` of its wait left: moving on its velocity
/// ([`drift_in_wait`]), whatever its condition and goal, as `_HandleShip`
/// counts +0x4c down for every ship (@0x3501f-0x35048) and skips its
/// steering, thrust and fire while it runs (@0x34a54-0x34a72); and
/// flying its goal once the wait is out, from the next tick, owing its
/// gate burst as `burst` reads ([`Npc::end_wait`]), as the original flies
/// it on the frame after +0x4c reaches 0.
fn hold_at_gate(npc: &mut Npc, ticks_left: u32, flown: Flown, burst: RuleSource) {
    drift_in_wait(npc, flown);
    let left = ticks_left.saturating_sub(1);
    if left == 0 {
        npc.end_wait(burst);
    } else {
        npc.mode = Mode::HeldAtGate { ticks_left: left };
    }
}

/// Tick `tick` of `npc`'s burst out of a gate: along its heading at
/// [`exit_speed`], ending as [`burst`] says. On its first tick it is out
/// of its gate ([`Npc::emerges_from`] cleared, @0x8dd8a-0x8dd93).
fn leave_gate(
    npc: &mut Npc,
    tick: u32,
    speeds: TopSpeeds,
    flown: Flown,
    chance: &mut (impl Chance + ?Sized),
) {
    if tick == 0 {
        // The pass that starts the burst sets goal 8 and +0x6c = -1
        // (`_HighLevelAIHandler` @0x8dd8a-0x8dd93).
        npc.emerges_from = None;
    }
    if !burst(
        npc,
        exit_speed(npc.led_out, tick),
        exit_speed(npc.led_out, tick + 1),
        speeds,
        flown,
        chance,
    ) {
        npc.mode = Mode::LeavingGate { tick: tick + 1 };
    }
}

/// One tick of a glide in or a burst out of a gate (`_HandleShip`
/// @0x34bc0-0x34d8b): `npc` moves along its heading at `speed`. Once the
/// `next` tick's speed would be at or below its cap
/// ([`TopSpeeds::burst_cap`]) the burst is over, and whether it is, the
/// caller stepping the mode on while it is not. It keeps this tick's
/// speed. With a lead (`speeds.lead`: a fleet escort's, or the player
/// for a ship the player leads) it flies on; with none it holds its
/// course for [`POST_BURST_WAIT`] + `Rand(`[`POST_BURST_SPREAD`]`)`
/// ticks first ([`Mode::Coasting`], @0x34d5c-0x34d8b), its trigger
/// released.
///
/// An inertialess ship (as `flown` says) takes `speed` as its speed along
/// its heading instead (@0x34bdf-0x34bf0), after its velocity has
/// followed the last tick's and it has moved (@0x34250-0x342aa), and its
/// velocity is zeroed as the burst ends (@0x34d46-0x34d59).
fn burst(
    npc: &mut Npc,
    speed: f32,
    next: f32,
    speeds: TopSpeeds,
    flown: Flown,
    chance: &mut (impl Chance + ?Sized),
) -> bool {
    burst_motion(npc, speed, flown.handling, flown.inertia);
    if next > speeds.burst_cap() {
        return false;
    }
    after_burst(npc, speeds, flown.inertia, chance);
    true
}

/// A burst's move (`_HandleShip` @0x34bc0-0x34c2a): `npc` moves along its
/// heading at `speed`; an inertialess one, as `inertia` says, takes
/// `speed` as its speed along its heading after its velocity has followed
/// the last tick's at `handling`'s rate and it has moved
/// ([`inertialess::drift`], @0x34250-0x342aa, @0x34bdf-0x34bf0).
pub(crate) fn burst_motion(npc: &mut Npc, speed: f32, handling: &Handling, inertia: Inertia) {
    if let Inertia::Inertialess { .. } = inertia {
        inertialess::drift(npc, handling);
        npc.speed = speed;
    } else {
        npc.state.velocity = flight::facing(npc.state.heading) * speed;
        npc.state.position = npc.state.position + npc.state.velocity;
    }
}

/// The end of a burst (`_HandleShip` @0x34d37-0x34d8b): an inertialess
/// `npc`'s velocity zeroed, as `inertia` says; with a lead (`speeds.lead`)
/// it flies on, and with none it holds its course for
/// [`POST_BURST_WAIT`] + `Rand(`[`POST_BURST_SPREAD`]`)` ticks, rolled on
/// `chance` ([`Mode::Coasting`]), its trigger released.
pub(crate) fn after_burst(
    npc: &mut Npc,
    speeds: TopSpeeds,
    inertia: Inertia,
    chance: &mut (impl Chance + ?Sized),
) {
    if let Inertia::Inertialess { .. } = inertia {
        npc.state.velocity = Vec2::ZERO;
    }
    npc.mode = if speeds.lead.is_some() {
        Mode::Flying
    } else {
        npc.trigger = Trigger::default();
        Mode::Coasting {
            ticks_left: POST_BURST_WAIT + chance.below(POST_BURST_SPREAD),
        }
    };
}

/// One tick of `npc`'s wait after its burst, `ticks_left` of it left: it
/// moves on at its velocity, neither turning, thrusting, slowing nor
/// firing, as `_HandleShip` skips its steering, thrust, over-speed and
/// fire while +0x4c runs (@0x34a54-0x34a72 to @0x3501f) but moves it
/// first (@0x34274-0x342aa), and flies once the wait is over
/// (@0x3501f-0x35048). An inertialess ship's velocity follows its speed
/// as it moves (@0x3425f; [`inertialess::drift`]).
fn coast(npc: &mut Npc, ticks_left: u32, flown: Flown) {
    position_step(npc, flown.handling, flown.inertia);
    count_down(npc, ticks_left);
}

/// `_HandleShip`'s move alone (@0x34274-0x342aa), with neither turn nor
/// thrust: `npc` moves by its velocity, an inertialess one's velocity
/// following its speed at `handling`'s rate first (@0x3425f,
/// [`inertialess::drift`]): the acceleration its adjustment reads at its
/// step ([`AtStep::unthrust`]).
pub(crate) fn position_step(npc: &mut Npc, handling: &Handling, inertia: Inertia) {
    match inertia {
        Inertia::Inertial => npc.state.position = npc.state.position + npc.state.velocity,
        Inertia::Inertialess { .. } => inertialess::drift(npc, handling),
    }
}

/// What a ship with no say in its flight flies on: its handling, and
/// whether it is inertialess.
#[derive(Clone, Copy)]
struct Flown<'a> {
    /// Its handling.
    handling: &'a Handling,
    /// Whether it is inertialess.
    inertia: Inertia,
}

impl Flown<'_> {
    /// Holds `npc` where it is, as a ship entering a gate is held: still
    /// with inertia; inertialess, its velocity following its speed
    /// ([`inertialess::drift`]).
    fn hold(self, npc: &mut Npc) {
        match self.inertia {
            Inertia::Inertial => npc.state.velocity = Vec2::ZERO,
            Inertia::Inertialess { .. } => inertialess::drift(npc, self.handling),
        }
    }
}

/// Counts `npc`'s wait on its course down a tick from `ticks_left`
/// (@0x35026-0x35048): it flies once the wait is out.
fn count_down(npc: &mut Npc, ticks_left: u32) {
    let left = ticks_left.saturating_sub(1);
    npc.mode = if left == 0 {
        Mode::Flying
    } else {
        Mode::Coasting { ticks_left: left }
    };
}
