//! The NPC autopilot: each tick it turns an NPC's goal into [`Controls`]
//! and flies them with the player's [`flight::step`], on the NPC's own
//! handling at its top speed [`TopSpeeds::own`], its `_ShipMaxSpeed`
//! ([`Npc::top_speed`]). It decides nothing (the [`ai`](crate::ai) does); it only
//! flies the goal it is given.
//!
//! - [`Goal::Land`]: it heads for the stellar at full speed, steering out
//!   any sideways drift, and brakes (Down and thrust) once its stopping
//!   distance reaches the distance left. It lands within the stellar's
//!   [`landing_radius`] at or below [`LANDING_SPEED`]. That circle is not
//!   the player's landing distance
//!   ([`RuleKey::LandingDistance`](crate::RuleKey::LandingDistance)):
//!   the original lands an NPC within a quarter of the `Type` width on
//!   each axis, inclusive, with no speed gate (`_HighLevelAIHandler`
//!   @0x8dbdf-0x8dc5a), story-runtime phase 16 (npc-landing-reach).
//! - [`Goal::JumpOut`]: it heads straight out from the centre at full
//!   speed, and jumps once it is its stats' jump distance out.
//! - [`Goal::Follow`]: it keeps within [`FOLLOW_DISTANCE`] of its lead,
//!   matching its lead's velocity there.
//! - [`Goal::Assist`]: outside its reach of the player (see
//!   [`assist`]) it follows the player as an escort its lead; within it,
//!   it brakes.
//! - [`Goal::Formation`]: the player's escort, hired or a mission escort,
//!   keeps its slot beside the player ([`Npc::formation`],
//!   [`slot_position`](crate::escort::slot_position)); with no slot it brakes. Within
//!   [`KEEP_FORMATION`] of it on each axis, it takes the player's
//!   velocity and heading and moves with it, nudged back towards the slot
//!   on each axis more than [`FORMATION_SLACK`] off by at most
//!   [`FORMATION_NUDGE`] ticks' acceleration; within
//!   [`APPROACH_FORMATION`], it closes on the slot at the speed it can
//!   still brake from; farther off, it flies at full speed towards it.
//! - [`Goal::Dock`]: a fighter flies back to its
//!   [`Carrier`](crate::bay::Carrier)
//!   (`_LowLevelAIHandler` sub 8 and 0xb, see [`bay`](crate::bay)).
//!   Outside its dock window on either axis it closes on the carrier as
//!   an escort on its slot, at full speed beyond [`APPROACH_FORMATION`];
//!   inside, it turns at the carrier, thrusting within its turn rate and
//!   [`DOCK_TURN_SLACK`] of it, and each axis more than
//!   [`FORMATION_NUDGE`] ticks' acceleration off is pulled that much
//!   towards it. Within the carrier's reach on both axes it docks, and
//!   leaves the system ([`Outcome::Docked`]).
//! - [`Goal::Idle`], or a goal it cannot fly (a stellar, lead, player or
//!   carrier that is not there): it brakes to a stop.
//! - While it jumps in ([`Mode::JumpingIn`]) it glides towards the centre,
//!   [`glide_speed`] a tick, with no steering, and flies on at that speed
//!   once the next tick's would be at or below its cap
//!   ([`TopSpeeds::burst_cap`]), as it does bursting out of a gate (see
//!   below). A disabled ship glides on too: `_AIDispatch`
//!   cancels a disabled ship's jump out but lets its jump in run
//!   (@0x8fccb-0x8fd08), so a rescue mission's ship held disabled as it
//!   comes in ends its glide in the system.
//! - While it waits at the gate it comes out of ([`Mode::Emerging`]) it
//!   holds still where it was put, at rest, its wait counting down
//!   whatever its condition, as `_HandleShip` counts the timer down for
//!   every ship (@0x3501f-0x35048) and the original's goal 21 holds it
//!   until its +0x4c runs out (`_HighLevelAIHandler` @0x8dd61-0x8dd93).
//!   Then ([`Mode::LeavingGate`]) it bursts out along its heading at
//!   [`exit_speed`] (from [`LED_BURST_TICK`] for a ship
//!   [`led_out`](Npc::led_out) of the gate by the player, so from 15),
//!   with no steering, and flies on at that speed once
//!   the next tick's would be at or below its cap
//!   ([`TopSpeeds::burst_cap`]: the lower of its own top speed and its
//!   lead's, the player's for a ship the player leads; see
//!   [`emerge`](crate::traffic::emerge)), then flies its goal. A ship not
//!   intact by then drifts instead of bursting, flying from there on.
//! - A ship with no lead, its glide or burst over, holds its course
//!   ([`Mode::Coasting`]) for [`POST_BURST_WAIT`] +
//!   `Rand(`[`POST_BURST_SPREAD`]`)` ticks (`_HandleShip`
//!   @0x34d5c-0x34d8b): it moves on at the speed the glide or burst left
//!   it, above its top speed, with no steering, thrust or fire, and its
//!   trigger is released. Then it flies its goal, [`flight::step`]
//!   bringing it down to its top speed. A ship with a lead flies at once.
//!   One not intact while it waits drifts instead.
//! - Otherwise a ship that is not [`Condition::Intact`] (disabled,
//!   breaking up or destroyed) drifts: it flies on with no controls, and
//!   never lands or jumps.

use crate::ai::{Goal, fire};
use crate::bay::DOCK_TURN_SLACK;
use crate::catalog::{LandingSite, ShipId, StellarId};
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::combat::aim::bearing;
use crate::combat::armament::Trigger;
use crate::combat::hull::Condition;
use crate::escort::{APPROACH_FORMATION, FORMATION_NUDGE, FORMATION_SLACK, KEEP_FORMATION};
use crate::flight::{self, AT_REST_SPEED, Controls, ShipState, Turn, heading_of, shortest_turn};
use crate::geometry::Vec2;
use crate::hail::assist;
use crate::handling::Handling;
use crate::hyperspace::JUMP_FUEL;
use crate::landing::{LANDING_SPEED, landing_radius};
use crate::traffic::emerge::{LED_BURST_TICK, exit_speed};
use crate::traffic::npc::{Mode, Npc};
use crate::traffic::spawn::glide_speed;

/// How far from its lead, in pixels, an escort keeps.
pub const FOLLOW_DISTANCE: f32 = 100.0;
/// How far off its course, in degrees, a ship still thrusts.
pub const ALIGNED: f32 = 5.0;
/// How many turns' worth off the heading it wants an attacker approaching
/// still thrusts (4.0 @0xdd680).
pub const APPROACH_THRUST: f32 = 4.0;
/// How far beyond a turn, in degrees, a dogfighter still thrusts (15.0
/// @0xdda30).
pub const DOGFIGHT_THRUST: f32 = 15.0;
/// How far beyond a turn, in degrees, a ship fleeing close still thrusts
/// (20.0 @0xdd668).
pub const FLEE_THRUST: f32 = 20.0;
/// How near, in pixels on either axis, an attacker must be for a ship
/// fleeing to turn straight away from it (low mode 5, @0x85c62).
pub const FLEE_CLOSE: f32 = 250.0;
/// The least a leaderless ship holds its course after its glide in or
/// burst out of a gate, in ticks (`Rand(30) + 30`, @0x34d74-0x34d81).
pub const POST_BURST_WAIT: u32 = 30;
/// `Rand(30)`: the spread of that wait.
pub const POST_BURST_SPREAD: u32 = 30;

/// What became of an NPC this tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// It flies on.
    Flying,
    /// It landed on this stellar, and leaves the system.
    Landed(StellarId),
    /// It jumped out of the system.
    JumpedOut,
    /// A fighter of ship type `ship` docked with `carrier`, and leaves
    /// the system for its bay.
    Docked {
        /// The ship it docked with.
        carrier: ShipRef,
        /// Its ship type.
        ship: ShipId,
    },
}

/// The top speeds a ship flies by this tick: its own `_ShipMaxSpeed`
/// ([`Npc::top_speed`]), and its lead's, while it has a lead in the
/// system, which caps its burst out of a gate (`_HandleShip`
/// @0x34c68-0x34ce6).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TopSpeeds {
    /// Its own top speed.
    pub own: f32,
    /// Its lead's top speed: the player's for a ship the player leads, the
    /// fleet lead's for a fleet escort; `None` without a lead, or with
    /// one gone from the system, as `_AICheckParentExists` clears it.
    pub lead: Option<f32>,
}

impl TopSpeeds {
    /// `npc`'s own handling's top speed, with no lead.
    #[must_use]
    pub fn own_of(npc: &Npc) -> Self {
        Self {
            own: npc.stats.handling.max_speed,
            lead: None,
        }
    }

    /// The speed its burst out of a gate ends at: the lower of its own top
    /// speed and its lead's, the lead's on a tie (@0x34c68-0x34ce6), or its
    /// own without a lead. `_HandleShip` raises a cap below -1.0 to -1.0
    /// (@0x34cf3-0x34d0d), which never binds, as no top speed is negative.
    #[must_use]
    pub fn burst_cap(self) -> f32 {
        self.lead.map_or(self.own, |lead| lead.min(self.own))
    }
}

/// Flies `npc` one tick towards its goal, among `sites`, the ship its
/// goal is about (the lead it follows, the ship it fights or inspects)
/// at `other`, when it is there, at its top speed `speeds.own`, its burst
/// out of a gate or glide in capped at [`TopSpeeds::burst_cap`], the wait
/// a leaderless ship takes after it rolled on `chance`.
pub fn fly(
    npc: &mut Npc,
    sites: &[LandingSite],
    other: Option<&ShipState>,
    speeds: TopSpeeds,
    chance: &mut (impl Chance + ?Sized),
) -> Outcome {
    let handling = Handling {
        max_speed: speeds.own,
        ..npc.stats.handling
    };
    if unsteered(npc, &handling, speeds, chance) {
        return Outcome::Flying;
    }
    let state = npc.state;
    let controls = match npc.goal {
        Goal::Land(stellar) => match sites.iter().find(|site| site.id == stellar) {
            Some(site) if over(&state, site) => return Outcome::Landed(stellar),
            Some(site) => approach(&state, &handling, site.position),
            None => brake(&state, &handling),
        },
        Goal::JumpOut => {
            if state.position.length() >= npc.stats.jump_distance {
                return Outcome::JumpedOut;
            }
            run_out(&state, &handling)
        }
        Goal::Follow(_) | Goal::Inspect(_) => match other {
            Some(other) => follow(&state, &handling, other),
            None => brake(&state, &handling),
        },
        Goal::Attack(_) => match other {
            Some(target) => attack(npc, target, &handling),
            None => brake(&state, &handling),
        },
        Goal::Snipe(_) => match other {
            Some(target) if state.velocity.length() <= AT_REST_SPEED => Controls {
                turn: steer(state.heading, fire::wanted_heading(npc, target), &handling),
                ..Controls::default()
            },
            _ => brake(&state, &handling),
        },
        Goal::Flee(_) => match other {
            Some(attacker) => {
                let off = state.position - attacker.position;
                if off.x.abs() <= FLEE_CLOSE && off.y.abs() <= FLEE_CLOSE {
                    let away = heading_of(off);
                    Controls {
                        thrust: shortest_turn(state.heading, away).abs()
                            <= handling.turn_rate + FLEE_THRUST,
                        turn: steer(state.heading, away, &handling),
                        reverse: false,
                    }
                } else {
                    let out = state.position.length();
                    if out >= npc.stats.jump_distance && npc.reserves.fuel.now >= JUMP_FUEL {
                        return Outcome::JumpedOut;
                    }
                    run_out(&state, &handling)
                }
            }
            None => brake(&state, &handling),
        },
        Goal::Assist(help) => match other {
            Some(player)
                if !assist::within(&state, player, assist::reach(help, handling.turn_rate)) =>
            {
                follow(&state, &handling, player)
            }
            _ => brake(&state, &handling),
        },
        Goal::Formation { .. } => match (other, npc.formation()) {
            (Some(leader), Some(place)) => {
                let slot = place.position(leader);
                let off = slot - state.position;
                let within = |reach: f32| off.x.abs() <= reach && off.y.abs() <= reach;
                if within(KEEP_FORMATION) {
                    keep_formation(npc, leader, slot);
                    return Outcome::Flying;
                }
                rejoin(&state, &handling, slot, leader.velocity)
            }
            _ => brake(&state, &handling),
        },
        Goal::Dock(_) => match dock(npc, other, &handling) {
            Ok(controls) => controls,
            Err(outcome) => return outcome,
        },
        Goal::Idle => brake(&state, &handling),
    };
    flight::step(&mut npc.state, &handling, controls);
    Outcome::Flying
}

/// How `npc` flies back to its carrier at `carrier` (see the module
/// docs): the controls to fly outside its dock window, or braking with
/// no carrier there; or, inside it, what became of it, having flown.
fn dock(
    npc: &mut Npc,
    carrier: Option<&ShipState>,
    handling: &Handling,
) -> Result<Controls, Outcome> {
    let (Some(carrier), Some(dock)) = (carrier, npc.carrier) else {
        return Ok(brake(&npc.state, handling));
    };
    let off = carrier.position - npc.state.position;
    let within = |reach: f32| off.x.abs() <= reach && off.y.abs() <= reach;
    if !within(dock.window) {
        return Ok(rejoin(
            &npc.state,
            handling,
            carrier.position,
            carrier.velocity,
        ));
    }
    if within(dock.reach) {
        return Err(Outcome::Docked {
            carrier: dock.ship,
            ship: npc.ship,
        });
    }
    dock_in(npc, carrier, handling);
    Err(Outcome::Flying)
}

/// Flies back to `to`, which moves at `velocity`: within
/// [`APPROACH_FORMATION`] of it on each axis, closing at the speed it can
/// still brake from; farther off, at full speed towards it.
fn rejoin(state: &ShipState, handling: &Handling, to: Vec2, velocity: Vec2) -> Controls {
    let off = to - state.position;
    if off.x.abs() <= APPROACH_FORMATION && off.y.abs() <= APPROACH_FORMATION {
        close_on(state, handling, to, velocity, 0.0)
    } else {
        let distance = off.length();
        match_velocity(state, handling, off * (handling.max_speed / distance))
    }
}

/// One tick of `npc` docking with a carrier at `carrier`, inside its dock
/// window (see the module docs): it turns at the carrier, thrusting
/// within its turn rate and [`DOCK_TURN_SLACK`] of it, flies, and is
/// pulled towards it on each axis farther off than
/// [`FORMATION_NUDGE`] ticks' acceleration.
fn dock_in(npc: &mut Npc, carrier: &ShipState, handling: &Handling) {
    let wanted = bearing(npc.state.position, carrier.position);
    let controls = Controls {
        thrust: shortest_turn(npc.state.heading, wanted).abs()
            <= handling.turn_rate + DOCK_TURN_SLACK,
        turn: steer(npc.state.heading, wanted, handling),
        reverse: false,
    };
    flight::step(&mut npc.state, handling, controls);
    let pull = handling.accel * FORMATION_NUDGE;
    let towards = |at: f32, to: f32| {
        let off = to - at;
        if off.abs() > pull {
            at + off.signum() * pull
        } else {
            at
        }
    };
    let at = npc.state.position;
    npc.state.position = Vec2::new(
        towards(at.x, carrier.position.x),
        towards(at.y, carrier.position.y),
    );
}

/// Heads straight out from the centre at full speed; from the centre
/// itself, ahead.
fn run_out(state: &ShipState, handling: &Handling) -> Controls {
    let away = if state.position.length() > 0.0 {
        heading_of(state.position)
    } else {
        state.heading
    };
    match_velocity(state, handling, flight::facing(away) * handling.max_speed)
}

/// Turns from `heading` towards `wanted`, not at all within half a tick's
/// turn, where turning would only overshoot.
fn steer(heading: f32, wanted: f32, handling: &Handling) -> Turn {
    let off = shortest_turn(heading, wanted);
    let half_turn = handling.turn_rate / 2.0;
    if off > half_turn {
        Turn::Right
    } else if off < -half_turn {
        Turn::Left
    } else {
        Turn::None
    }
}

/// Attacks a target at `target`: faces the heading it wants to fire along
/// ([`fire::wanted_heading`]), thrusting within [`APPROACH_THRUST`] turns
/// of it approaching from beyond the dogfight box, and within a turn and
/// [`DOGFIGHT_THRUST`] dogfighting inside it.
fn attack(npc: &Npc, target: &ShipState, handling: &Handling) -> Controls {
    let wanted = fire::wanted_heading(npc, target);
    let window = match fire::manoeuvre(npc.goal, &npc.state, target) {
        Some(fire::Manoeuvre::Dogfight) => handling.turn_rate + DOGFIGHT_THRUST,
        _ => APPROACH_THRUST * handling.turn_rate,
    };
    Controls {
        thrust: shortest_turn(npc.state.heading, wanted).abs() <= window,
        turn: steer(npc.state.heading, wanted, handling),
        reverse: false,
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
fn glide(npc: &mut Npc, tick: u32, speeds: TopSpeeds, chance: &mut (impl Chance + ?Sized)) {
    if !burst(
        npc,
        glide_speed(tick),
        glide_speed(tick + 1),
        speeds,
        chance,
    ) {
        npc.mode = Mode::JumpingIn { tick: tick + 1 };
    }
}

/// Flies `npc` one tick when it has no say in its flight: waiting at a
/// gate, drifting while not intact, gliding in from hyperspace (intact
/// or disabled), bursting out of a gate, or holding its course after
/// either (see the module docs), on `handling`, its burst ending at
/// `speeds`' [`burst_cap`](TopSpeeds::burst_cap) and its wait rolled on
/// `chance`. Whether it did.
fn unsteered(
    npc: &mut Npc,
    handling: &Handling,
    speeds: TopSpeeds,
    chance: &mut (impl Chance + ?Sized),
) -> bool {
    match (npc.mode, npc.condition == Condition::Intact) {
        (Mode::Emerging { ticks_left }, _) => wait_at_gate(npc, ticks_left),
        (Mode::JumpingIn { tick }, false) if npc.condition == Condition::Disabled => {
            glide(npc, tick, speeds, chance);
        }
        (mode, false) => {
            if let Mode::LeavingGate { .. } | Mode::Coasting { .. } = mode {
                npc.mode = Mode::Flying;
            }
            flight::step(&mut npc.state, handling, Controls::default());
        }
        (Mode::JumpingIn { tick }, true) => glide(npc, tick, speeds, chance),
        (Mode::LeavingGate { tick }, true) => leave_gate(npc, tick, speeds, chance),
        (Mode::Coasting { ticks_left }, true) => coast(npc, ticks_left),
        (Mode::Flying, true) => return false,
    }
    true
}

/// One tick of `npc`'s wait at the gate it comes out of, `ticks_left` of
/// it left: still, and bursting out once it is over.
fn wait_at_gate(npc: &mut Npc, ticks_left: u32) {
    npc.state.velocity = Vec2::ZERO;
    let left = ticks_left.saturating_sub(1);
    npc.mode = if left == 0 {
        Mode::LeavingGate {
            tick: if npc.led_out { LED_BURST_TICK } else { 0 },
        }
    } else {
        Mode::Emerging { ticks_left: left }
    };
}

/// Tick `tick` of `npc`'s burst out of a gate: along its heading at
/// [`exit_speed`], ending as [`burst`] says.
fn leave_gate(npc: &mut Npc, tick: u32, speeds: TopSpeeds, chance: &mut (impl Chance + ?Sized)) {
    if !burst(npc, exit_speed(tick), exit_speed(tick + 1), speeds, chance) {
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
fn burst(
    npc: &mut Npc,
    speed: f32,
    next: f32,
    speeds: TopSpeeds,
    chance: &mut (impl Chance + ?Sized),
) -> bool {
    npc.state.velocity = flight::facing(npc.state.heading) * speed;
    npc.state.position = npc.state.position + npc.state.velocity;
    if next > speeds.burst_cap() {
        return false;
    }
    npc.mode = if speeds.lead.is_some() {
        Mode::Flying
    } else {
        npc.trigger = Trigger::default();
        Mode::Coasting {
            ticks_left: POST_BURST_WAIT + chance.below(POST_BURST_SPREAD),
        }
    };
    true
}

/// One tick of `npc`'s wait after its burst, `ticks_left` of it left: it
/// moves on at its velocity, neither turning, thrusting, slowing nor
/// firing, as `_HandleShip` skips its steering, thrust, over-speed and
/// fire while +0x4c runs (@0x34a54-0x34a72 to @0x3501f) but moves it
/// first (@0x34274-0x342aa), and flies once the wait is over
/// (@0x3501f-0x35048).
fn coast(npc: &mut Npc, ticks_left: u32) {
    npc.state.position = npc.state.position + npc.state.velocity;
    let left = ticks_left.saturating_sub(1);
    npc.mode = if left == 0 {
        Mode::Flying
    } else {
        Mode::Coasting { ticks_left: left }
    };
}

/// Whether a ship at `state` can land on `site`: over it, and slow enough.
fn over(state: &ShipState, site: &LandingSite) -> bool {
    (site.position - state.position).length() <= landing_radius(site)
        && state.velocity.length() <= LANDING_SPEED
}

/// Heads for `target` at full speed, and brakes once the ship's stopping
/// distance reaches the distance left.
fn approach(state: &ShipState, handling: &Handling, target: Vec2) -> Controls {
    let to = target - state.position;
    let distance = to.length();
    if distance <= stopping_distance(state, handling) {
        return brake(state, handling);
    }
    match_velocity(state, handling, to * (handling.max_speed / distance))
}

/// How far a ship at `state` travels while it turns to face against its
/// motion and brakes to a stop; none at rest, and for ever when it cannot
/// turn or thrust.
fn stopping_distance(state: &ShipState, handling: &Handling) -> f32 {
    let speed = state.velocity.length();
    if speed <= AT_REST_SPEED {
        return 0.0;
    }
    if handling.turn_rate <= 0.0 || handling.accel <= 0.0 {
        return f32::INFINITY;
    }
    let behind = heading_of(state.velocity * -1.0);
    let turning = shortest_turn(state.heading, behind).abs() / handling.turn_rate;
    // Braking a tick at a time covers about half a tick's speed more
    // than the continuous v² / 2a; a whole tick's is the margin.
    speed * (turning.ceil() + 1.0) + speed * speed / (2.0 * handling.accel)
}

/// Turns to face against the ship's motion (Down) and thrusts once it
/// does, until thrust would only push it the other way.
fn brake(state: &ShipState, handling: &Handling) -> Controls {
    let speed = state.velocity.length();
    if speed <= AT_REST_SPEED {
        return Controls::default();
    }
    let behind = heading_of(state.velocity * -1.0);
    // Down lands exactly on the heading behind within one tick's turn,
    // before this tick's thrust.
    let facing_back = shortest_turn(state.heading, behind).abs() <= handling.turn_rate;
    Controls {
        thrust: facing_back && speed > handling.accel / 2.0,
        turn: Turn::None,
        reverse: true,
    }
}

/// Keeps within [`FOLLOW_DISTANCE`] of a lead at `lead`: closing at the
/// speed it can still brake from in the distance beyond it, on top of the
/// lead's velocity, and matching the lead's velocity within it.
fn follow(state: &ShipState, handling: &Handling, lead: &ShipState) -> Controls {
    close_on(
        state,
        handling,
        lead.position,
        lead.velocity,
        FOLLOW_DISTANCE,
    )
}

/// Keeps within `margin` of `target`, which moves at `velocity`: closing
/// at the speed it can still brake from in the distance beyond the
/// margin, on top of that velocity, and matching it within the margin.
fn close_on(
    state: &ShipState,
    handling: &Handling,
    target: Vec2,
    velocity: Vec2,
    margin: f32,
) -> Controls {
    let to = target - state.position;
    let distance = to.length();
    let beyond = (distance - margin).max(0.0);
    let speed = braking_speed(beyond, handling).min(handling.max_speed);
    // None within the margin, where `to` may be no direction at all.
    let closing = if distance > margin {
        to * (speed / distance)
    } else {
        Vec2::ZERO
    };
    match_velocity(state, handling, velocity + closing)
}

/// Keeps formation on `slot` beside a leader at `leader`
/// (`_AIMaintainFormation` @0x84f04): `npc` takes the leader's velocity
/// and heading and moves with it, then each axis more than
/// [`FORMATION_SLACK`] off the slot is nudged towards it by
/// [`FORMATION_NUDGE`] ticks' acceleration at most, never past it.
fn keep_formation(npc: &mut Npc, leader: &ShipState, slot: Vec2) {
    let state = &mut npc.state;
    state.velocity = leader.velocity;
    state.heading = leader.heading;
    state.position = state.position + state.velocity;
    let nudge = npc.stats.handling.accel * FORMATION_NUDGE;
    let towards = |at: f32, to: f32| {
        let off = to - at;
        if off.abs() > FORMATION_SLACK {
            at + off.signum() * nudge.min(off.abs())
        } else {
            at
        }
    };
    state.position = Vec2::new(
        towards(state.position.x, slot.x),
        towards(state.position.y, slot.y),
    );
}

/// The fastest a ship can go and still turn round and stop within
/// `distance`: the `v` with `v T + v² / 2a = distance`, `T` the ticks a
/// half turn takes. None when it cannot turn or thrust.
fn braking_speed(distance: f32, handling: &Handling) -> f32 {
    if handling.turn_rate <= 0.0 || handling.accel <= 0.0 {
        return 0.0;
    }
    let half_turn = 180.0 / handling.turn_rate;
    let a = handling.accel;
    a * (half_turn.mul_add(half_turn, 2.0 * distance / a).sqrt() - half_turn)
}

/// Turns towards the change that takes the ship's velocity to `desired`,
/// and thrusts once it faces it; coasts once the velocity is within half
/// a tick's thrust of it.
fn match_velocity(state: &ShipState, handling: &Handling, desired: Vec2) -> Controls {
    let change = desired - state.velocity;
    if change.length() <= handling.accel / 2.0 {
        return Controls::default();
    }
    let wanted = heading_of(change);
    let off = shortest_turn(state.heading, wanted);
    Controls {
        thrust: off.abs() <= ALIGNED.max(handling.turn_rate),
        turn: steer(state.heading, wanted, handling),
        reverse: false,
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::escort::{EscortDuty, FormationSlot};
    use crate::hail::{Help, assist};
    use crate::handling::ShipFields;
    use crate::stats::ShipStats;
    use crate::testkit::{Draws, FAST, planet};
    use crate::traffic::emerge::{EMERGE_TICKS, LED_BURST_TICK};
    use crate::traffic::npc::NpcId;
    use crate::traffic::spawn::{hyperspace_entry, jump_in_distance};

    /// Flies `npc` at its own top speed, with no lead: the autopilot as
    /// most tests see it.
    fn fly(npc: &mut Npc, sites: &[LandingSite], other: Option<&ShipState>) -> Outcome {
        fly_at(npc, sites, other, TopSpeeds::own_of(npc))
    }

    /// Flies `npc` at `speeds`, any wait it draws its longest.
    fn fly_at(
        npc: &mut Npc,
        sites: &[LandingSite],
        other: Option<&ShipState>,
        speeds: TopSpeeds,
    ) -> Outcome {
        super::fly(npc, sites, other, speeds, &mut Draws::of(&[]))
    }

    /// An average ship: 3 pixels a tick at most, 0.1 more a tick, 1° a
    /// tick.
    const AVERAGE: ShipFields = ShipFields {
        speed: 300,
        accel: 300,
        maneuver: 10,
        ..FAST
    };

    fn npc(fields: ShipFields, goal: Goal, start: ShipState) -> Npc {
        Npc {
            state: start,
            goal,
            ..crate::testkit::npc(1, ShipStats::new(fields, &[]))
        }
    }

    fn at(x: f32, y: f32, vx: f32, vy: f32, heading: f32) -> ShipState {
        ShipState {
            position: Vec2::new(x, y),
            velocity: Vec2::new(vx, vy),
            heading,
        }
    }

    /// Flies `npc` until it lands or jumps, or `limit` ticks pass, and
    /// gives how it ended and after how many ticks.
    fn until_gone(npc: &mut Npc, sites: &[LandingSite], limit: u32) -> (Outcome, u32) {
        for tick in 0..limit {
            let outcome = fly(npc, sites, None);
            if outcome != Outcome::Flying {
                return (outcome, tick);
            }
        }
        (Outcome::Flying, limit)
    }

    /// An escort in slot 2 of 3, spaced 30 apart, flying `fields` from
    /// `start`, keeping formation.
    fn escort(fields: ShipFields, start: ShipState) -> Npc {
        Npc {
            escort: Some(EscortDuty {
                formation: FormationSlot {
                    slot: 2,
                    ships: 3,
                    spacing: 30.0,
                },
                order: None,
            }),
            ..npc(fields, Goal::Formation { guard: None }, start)
        }
    }

    /// Slot 2 of 3, spaced 30 apart, behind `leader`.
    fn slot_of(leader: &ShipState) -> Vec2 {
        crate::escort::slot_position(leader, 2, 3, 30.0)
    }

    #[test]
    fn an_escort_on_its_slot_matches_the_leaders_velocity_and_heading() {
        let leader = at(400.0, -200.0, 1.0, -2.0, 30.0);
        let slot = slot_of(&leader);
        let mut ship = escort(FAST, at(slot.x, slot.y, 0.0, 0.0, 200.0));
        assert_eq!(fly(&mut ship, &[], Some(&leader)), Outcome::Flying);
        assert_eq!(ship.state.velocity, leader.velocity);
        assert_eq!(ship.state.heading, 30.0);
        assert_eq!(
            ship.state.position,
            slot + leader.velocity,
            "moving with it"
        );
    }

    #[test]
    fn an_escort_keeping_formation_is_nudged_back_beyond_8_pixels_off_by_at_most_ten_ticks_thrust()
    {
        let leader = at(0.0, 0.0, 0.0, 0.0, 0.0);
        let slot = slot_of(&leader);
        let off = |x: f32, y: f32| at(slot.x + x, slot.y + y, 0.0, 0.0, 0.0);
        let nudged = |fields: ShipFields, x: f32, y: f32| {
            let mut ship = escort(fields, off(x, y));
            fly(&mut ship, &[], Some(&leader));
            ship.state.position - slot
        };
        assert_eq!(
            nudged(FAST, 8.0, -8.0),
            Vec2::new(8.0, -8.0),
            "within the slack"
        );
        assert_eq!(
            nudged(FAST, 9.0, -9.0),
            Vec2::new(6.0, -6.0),
            "0.3 x 10 a tick"
        );
        assert_eq!(
            nudged(FAST, 9.0, 2.0),
            Vec2::new(6.0, 2.0),
            "each axis on its own"
        );
        assert_eq!(nudged(AVERAGE, -9.0, 0.0), Vec2::new(-8.0, 0.0), "0.1 x 10");
        assert_eq!(nudged(FAST, 299.0, -300.0), Vec2::new(296.0, -297.0));
        let strong = ShipFields {
            accel: 30_000,
            ..FAST
        };
        assert_eq!(nudged(strong, 9.0, -20.0), Vec2::ZERO, "never past it");
        assert_eq!(
            (
                KEEP_FORMATION,
                APPROACH_FORMATION,
                FORMATION_SLACK,
                FORMATION_NUDGE
            ),
            (300.0, 600.0, 8.0, 10.0)
        );
    }

    #[test]
    fn an_escort_off_its_slot_approaches_it_and_far_off_flies_at_full_speed() {
        let leader = at(0.0, 0.0, 0.0, 0.0, 0.0);
        let slot = slot_of(&leader);
        // At full speed (3) straight down at the slot from above.
        let mut near = escort(AVERAGE, at(slot.x, slot.y - 400.0, 0.0, 3.0, 180.0));
        let mut far = escort(AVERAGE, at(slot.x, slot.y - 700.0, 0.0, 3.0, 180.0));
        let before = (near.state, far.state);
        fly(&mut near, &[], Some(&leader));
        fly(&mut far, &[], Some(&leader));
        assert_ne!(
            near.state.heading, before.0.heading,
            "400 off, it turns to slow down"
        );
        assert_eq!(
            far.state.heading, before.1.heading,
            "700 off, it keeps going"
        );
        assert_eq!(far.state.velocity, before.1.velocity);
        assert_eq!(far.state.position, before.1.position + before.1.velocity);
        // From rest, facing the slot, either thrusts towards it.
        for distance in [301.0, 400.0, 700.0] {
            let mut ship = escort(AVERAGE, at(slot.x, slot.y - distance, 0.0, 0.0, 180.0));
            fly(&mut ship, &[], Some(&leader));
            assert!(ship.state.velocity.y > 0.0, "{distance}: {:?}", ship.state);
            assert_eq!(ship.state.heading, 180.0, "{distance}");
        }
        // Beyond 300 on one axis only, it flies rather than keeps formation.
        let mut aside = escort(AVERAGE, at(slot.x + 301.0, slot.y, 0.0, 0.0, 270.0));
        fly(&mut aside, &[], Some(&leader));
        assert!(aside.state.velocity.x < 0.0, "{:?}", aside.state);
        assert_eq!(aside.state.heading, 270.0, "not the leader's");
    }

    #[test]
    fn an_escort_whose_leader_is_gone_or_with_no_duty_brakes() {
        // Moving down and facing down, it turns to face back to brake.
        let mut lost = escort(AVERAGE, at(0.0, 0.0, 0.0, 2.0, 180.0));
        fly(&mut lost, &[], None);
        assert_ne!(lost.state.heading, 180.0, "{:?}", lost.state);
        let leader = at(0.0, 0.0, 0.0, 0.0, 0.0);
        let mut dutiless = escort(AVERAGE, at(0.0, 0.0, 0.0, 2.0, 180.0));
        dutiless.escort = None;
        fly(&mut dutiless, &[], Some(&leader));
        assert_ne!(dutiless.state.heading, 180.0, "{:?}", dutiless.state);
        let mut unjoined = escort(AVERAGE, at(0.0, 0.0, 0.0, 2.0, 180.0));
        unjoined.escort = None;
        unjoined.mission = Some(mark(None));
        fly(&mut unjoined, &[], Some(&leader));
        assert_ne!(unjoined.state.heading, 180.0, "{:?}", unjoined.state);
    }

    /// A mark of slot 0's escorting mission, holding `formation`.
    fn mark(formation: Option<FormationSlot>) -> crate::traffic::npc::MissionMark {
        crate::traffic::npc::MissionMark {
            order: None,
            slot: 0,
            behav: crate::mission::ships::ShipBehav::EscortPlayer,
            formation,
        }
    }

    #[test]
    fn a_mission_escort_flies_its_formation_slot_with_no_escort_duty() {
        let leader = at(400.0, -200.0, 1.0, -2.0, 30.0);
        let slot = slot_of(&leader);
        let mut ship = escort(FAST, at(slot.x, slot.y, 0.0, 0.0, 200.0));
        ship.escort = None;
        ship.mission = Some(mark(Some(FormationSlot {
            slot: 2,
            ships: 3,
            spacing: 30.0,
        })));
        assert_eq!(fly(&mut ship, &[], Some(&leader)), Outcome::Flying);
        assert_eq!(ship.state.velocity, leader.velocity);
        assert_eq!(ship.state.position, slot + leader.velocity);
        assert_eq!(ship.escort, None);
    }

    #[test]
    fn the_named_values() {
        assert_eq!((FOLLOW_DISTANCE, ALIGNED), (100.0, 5.0));
    }

    #[test]
    fn a_ship_at_rest_flies_to_a_stellar_and_lands_on_it() {
        let sites = [planet(140, 500.0, 300.0), planet(141, -800.0, 0.0)];
        for fields in [FAST, AVERAGE] {
            for heading in [0.0, 90.0, 225.0] {
                let mut ship = npc(
                    fields,
                    Goal::Land(StellarId(140)),
                    at(0.0, 0.0, 0.0, 0.0, heading),
                );
                let (outcome, ticks) = until_gone(&mut ship, &sites, 2000);
                assert_eq!(
                    outcome,
                    Outcome::Landed(StellarId(140)),
                    "{fields:?} {heading}"
                );
                assert!(ticks < 1200, "{ticks} ticks");
                let off = (ship.state.position - sites[0].position).length();
                assert!(off <= 50.0, "{off}");
                assert!(ship.state.velocity.length() <= LANDING_SPEED);
            }
        }
    }

    #[test]
    fn a_ship_crossing_past_a_stellar_turns_back_and_lands_on_it() {
        let sites = [planet(140, 0.0, 0.0)];
        for (fields, start) in [
            (FAST, at(-400.0, 200.0, 6.0, 0.0, 90.0)),
            (FAST, at(0.0, -100.0, 0.0, 6.0, 180.0)),
            (AVERAGE, at(300.0, 300.0, 0.0, -3.0, 0.0)),
            (AVERAGE, at(-20.0, 0.0, 3.0, 0.0, 90.0)),
        ] {
            let mut ship = npc(fields, Goal::Land(StellarId(140)), start);
            let (outcome, ticks) = until_gone(&mut ship, &sites, 3000);
            assert_eq!(outcome, Outcome::Landed(StellarId(140)), "{start:?}");
            assert!(ticks < 2000, "{start:?}: {ticks} ticks");
        }
    }

    #[test]
    fn a_ship_over_a_stellar_slow_enough_lands_at_once() {
        let sites = [planet(140, 10.0, 0.0)];
        let mut ship = npc(
            FAST,
            Goal::Land(StellarId(140)),
            at(0.0, 0.0, 1.0, 0.0, 0.0),
        );
        assert_eq!(
            fly(&mut ship, &sites, None),
            Outcome::Landed(StellarId(140))
        );
        assert_eq!(ship.state.position, Vec2::ZERO, "where it was");
        let mut fast = npc(
            FAST,
            Goal::Land(StellarId(140)),
            at(0.0, 0.0, 1.5, 0.0, 0.0),
        );
        assert_eq!(fly(&mut fast, &sites, None), Outcome::Flying, "too fast");
        let mut far = npc(
            FAST,
            Goal::Land(StellarId(140)),
            at(61.0, 0.0, 0.0, 0.0, 0.0),
        );
        assert_eq!(fly(&mut far, &sites, None), Outcome::Flying, "too far");
    }

    #[test]
    fn a_ship_not_intact_drifts_and_never_lands_or_jumps() {
        let sites = [planet(140, 0.0, 0.0)];
        for condition in [
            Condition::Disabled,
            Condition::Dying { ticks_left: 4 },
            Condition::Destroyed,
        ] {
            let mut lander = npc(
                AVERAGE,
                Goal::Land(StellarId(140)),
                at(0.0, 0.0, 0.0, 0.0, 0.0),
            );
            lander.condition = condition;
            assert_eq!(
                fly(&mut lander, &sites, None),
                Outcome::Flying,
                "{condition:?}"
            );
            assert_eq!(
                lander.state,
                at(0.0, 0.0, 0.0, 0.0, 0.0),
                "at rest, it stays"
            );
            let mut jumper = npc(AVERAGE, Goal::JumpOut, at(0.0, -2000.0, 1.0, -2.0, 90.0));
            jumper.condition = condition;
            assert_eq!(fly(&mut jumper, &sites, None), Outcome::Flying);
            assert_eq!(jumper.state, at(1.0, -2002.0, 1.0, -2.0, 90.0), "drifting");
        }
    }

    #[test]
    fn a_ship_breaking_up_or_destroyed_jumping_in_drifts_with_no_glide() {
        for condition in [Condition::Dying { ticks_left: 4 }, Condition::Destroyed] {
            let mut jumper = npc(AVERAGE, Goal::JumpOut, at(0.0, -2000.0, 1.0, -2.0, 90.0));
            jumper.condition = condition;
            jumper.mode = Mode::JumpingIn { tick: 3 };
            assert_eq!(fly(&mut jumper, &[], None), Outcome::Flying);
            assert_eq!(jumper.state, at(1.0, -2002.0, 1.0, -2.0, 90.0), "drifting");
            assert_eq!(jumper.mode, Mode::JumpingIn { tick: 3 }, "no glide");
        }
    }

    /// A ship of `fields` coming out of a gate at (100, -200), heading 90°,
    /// `ticks_left` of its wait to go, bent on landing.
    fn emerging(fields: ShipFields, ticks_left: u32) -> Npc {
        let mut ship = npc(
            fields,
            Goal::Land(StellarId(140)),
            at(100.0, -200.0, 0.0, 0.0, 90.0),
        );
        ship.mode = Mode::Emerging { ticks_left };
        ship
    }

    #[test]
    fn a_ship_coming_out_of_a_gate_waits_there_still_then_leaves_it() {
        let sites = [planet(140, 0.0, 0.0)];
        let mut ship = emerging(AVERAGE, EMERGE_TICKS);
        assert!(ship.emerging());
        for left in (1..EMERGE_TICKS).rev() {
            assert_eq!(fly(&mut ship, &sites, None), Outcome::Flying);
            assert_eq!(ship.mode, Mode::Emerging { ticks_left: left });
            assert_eq!(ship.state, at(100.0, -200.0, 0.0, 0.0, 90.0), "still");
        }
        fly(&mut ship, &sites, None);
        assert_eq!(ship.mode, Mode::LeavingGate { tick: 0 });
        assert!(!ship.emerging());
        assert_eq!(
            ship.state,
            at(100.0, -200.0, 0.0, 0.0, 90.0),
            "not yet moving"
        );
    }

    /// The last tick of a burst out of a gate, flown at `speeds`.
    fn burst_ends(speeds: TopSpeeds) -> u32 {
        let sites = [planet(140, 0.0, 0.0)];
        let mut ship = emerging(AVERAGE, 1);
        fly(&mut ship, &sites, None);
        for _ in 0..20 {
            let Mode::LeavingGate { tick } = ship.mode else {
                panic!("not bursting: {:?}", ship.mode);
            };
            fly_at(&mut ship, &sites, None, speeds);
            if !matches!(ship.mode, Mode::LeavingGate { .. }) {
                return tick;
            }
        }
        panic!("never ended")
    }

    #[test]
    fn a_burst_ends_at_the_lower_of_its_own_and_its_leads_top_speed() {
        let speeds = |own, lead| TopSpeeds { own, lead };
        // 30 - 3t: the burst ends once the next tick's is at or below the
        // cap.
        assert_eq!(exit_speed(9), 3.0);
        assert_eq!(burst_ends(speeds(6.0, Some(3.0))), 8, "the lead's 3");
        assert_eq!(burst_ends(speeds(6.0, Some(9.0))), 7, "its own 6");
        assert_eq!(burst_ends(speeds(6.0, Some(6.0))), 7, "a tie: 6");
        assert_eq!(burst_ends(speeds(6.0, None)), 7, "no lead: its own");
        assert_eq!(burst_ends(speeds(3.0, None)), 8);
        assert_eq!(speeds(6.0, Some(3.0)).burst_cap(), 3.0);
        assert_eq!(speeds(3.0, Some(6.0)).burst_cap(), 3.0);
        assert_eq!(speeds(6.0, None).burst_cap(), 6.0);
    }

    #[test]
    fn flying_at_a_raised_top_speed_it_reaches_it_and_drifting_never_passes_it() {
        let raised = TopSpeeds {
            own: 4.5,
            lead: None,
        };
        let leader = at(20_000.0, 0.0, 0.0, 0.0, 90.0);
        let mut ship = escort(AVERAGE, at(0.0, 0.0, 0.0, 0.0, 90.0));
        let mut fastest: f32 = 0.0;
        for _ in 0..300 {
            fly_at(&mut ship, &[], Some(&leader), raised);
            fastest = fastest.max(ship.state.velocity.length());
        }
        assert!((fastest - 4.5).abs() < 1e-3, "{fastest}");
        let mut own = escort(AVERAGE, at(0.0, 0.0, 0.0, 0.0, 90.0));
        for _ in 0..300 {
            fly(&mut own, &[], Some(&leader));
        }
        assert!(own.state.velocity.length() <= 3.0 + 1e-4);
        let drifting = |speed: f32| {
            let mut wreck = npc(AVERAGE, Goal::Idle, at(0.0, 0.0, speed, 0.0, 90.0));
            wreck.condition = Condition::Disabled;
            fly_at(&mut wreck, &[], None, raised);
            wreck.state.velocity.length()
        };
        assert!((drifting(4.0) - 4.0).abs() < 1e-4, "under 4.5: kept");
        assert!((drifting(10.0) - 4.5).abs() < 1e-4, "capped at 4.5");
    }

    #[test]
    fn a_ship_led_out_of_a_gate_bursts_from_15() {
        let sites = [planet(140, 0.0, 0.0)];
        // Top speed 3: 15, 12, 9, then on at 6. Led by the player, whose
        // top speed is its lead's.
        let speeds = TopSpeeds {
            own: 3.0,
            lead: Some(3.0),
        };
        let mut ship = emerging(AVERAGE, 1);
        ship.led_out = true;
        fly(&mut ship, &sites, None);
        let mut x = 100.0;
        for (tick, speed) in (LED_BURST_TICK..).zip([15.0, 12.0, 9.0, 6.0]) {
            assert_eq!(ship.mode, Mode::LeavingGate { tick });
            fly_at(&mut ship, &sites, None, speeds);
            x += speed;
            assert!((ship.state.velocity - Vec2::new(speed, 0.0)).length() < 1e-4);
            assert!((ship.state.position - Vec2::new(x, -200.0)).length() < 1e-3);
        }
        assert_eq!(ship.mode, Mode::Flying);
    }

    #[test]
    fn leaving_a_gate_it_bursts_out_on_its_heading_until_it_is_down_to_its_top_speed() {
        let sites = [planet(140, 0.0, 0.0)];
        // Top speed 3: 30, 27, ... 6, then on at 6 once 3 is next.
        let mut ship = emerging(AVERAGE, 1);
        fly(&mut ship, &sites, None);
        let mut x = 100.0;
        for (tick, speed) in (0..9).zip([30.0, 27.0, 24.0, 21.0, 18.0, 15.0, 12.0, 9.0, 6.0]) {
            assert_eq!(ship.mode, Mode::LeavingGate { tick });
            assert_eq!(fly(&mut ship, &sites, None), Outcome::Flying);
            x += speed;
            assert!(
                (ship.state.velocity - Vec2::new(speed, 0.0)).length() < 1e-4,
                "{tick}: {:?}",
                ship.state
            );
            assert!((ship.state.position - Vec2::new(x, -200.0)).length() < 1e-3);
            assert_eq!(ship.state.heading, 90.0, "no steering");
        }
        assert_eq!(ship.mode, Mode::Coasting { ticks_left: 59 }, "no lead");
        // A ship of top speed 30 or more is out after the first tick.
        let mut fast = emerging(
            ShipFields {
                speed: 3000,
                ..AVERAGE
            },
            1,
        );
        fly(&mut fast, &sites, None);
        fly(&mut fast, &sites, None);
        assert_eq!(fast.mode, Mode::Coasting { ticks_left: 59 });
        assert!((fast.state.velocity - Vec2::new(30.0, 0.0)).length() < 1e-4);
        // One of no speed bursts out until its speed would reach 0.
        let mut still = emerging(
            ShipFields {
                speed: 0,
                ..AVERAGE
            },
            1,
        );
        fly(&mut still, &sites, None);
        let ticks = (1..20)
            .find(|_| {
                fly(&mut still, &sites, None);
                !matches!(still.mode, Mode::LeavingGate { .. })
            })
            .expect("out");
        assert_eq!(ticks, 10, "30 down to 3");
    }

    #[test]
    fn a_ship_not_intact_still_waits_out_its_gate_but_drifts_instead_of_bursting() {
        let mut ship = emerging(AVERAGE, 2);
        ship.condition = Condition::Disabled;
        fly(&mut ship, &[], None);
        assert_eq!(ship.mode, Mode::Emerging { ticks_left: 1 });
        fly(&mut ship, &[], None);
        assert_eq!(ship.mode, Mode::LeavingGate { tick: 0 });
        fly(&mut ship, &[], None);
        assert_eq!(ship.mode, Mode::Flying, "no burst");
        assert_eq!(ship.state, at(100.0, -200.0, 0.0, 0.0, 90.0), "drifting");
    }

    #[test]
    fn a_ship_emerging_from_a_gate_holds_at_rest_for_its_ticks_then_flies_its_goal() {
        let start = at(100.0, 200.0, 3.0, -4.0, 45.0);
        let mut ship = npc(FAST, Goal::Attack(ShipRef::Player), start);
        ship.mode = Mode::Emerging { ticks_left: 3 };
        let player = at(600.0, 200.0, 0.0, 0.0, 0.0);
        for left in [2, 1, 0] {
            assert_eq!(fly(&mut ship, &[], Some(&player)), Outcome::Flying);
            assert_eq!(ship.state, at(100.0, 200.0, 0.0, 0.0, 45.0), "{left}");
            let mode = if left == 0 {
                Mode::LeavingGate { tick: 0 }
            } else {
                Mode::Emerging { ticks_left: left }
            };
            assert_eq!(ship.mode, mode);
        }
        while ship.mode != Mode::Flying {
            fly(&mut ship, &[], Some(&player));
            assert_eq!(ship.state.heading, 45.0, "bursting out, unsteered");
        }
        fly(&mut ship, &[], Some(&player));
        assert_ne!(ship.state.heading, 45.0, "turns on the player");
        let mut wreck = npc(FAST, Goal::Idle, start);
        wreck.condition = Condition::Disabled;
        wreck.mode = Mode::Emerging { ticks_left: 2 };
        fly(&mut wreck, &[], None);
        assert_eq!(wreck.state, at(100.0, 200.0, 0.0, 0.0, 45.0), "held too");
        assert_eq!(wreck.mode, Mode::Emerging { ticks_left: 1 });
    }

    /// A ship of `fields` jumping in, at (0, -2000) heading 180°, on the
    /// glide's first tick.
    fn jumping_in(fields: ShipFields) -> Npc {
        let mut ship = npc(fields, Goal::Idle, at(0.0, -2000.0, 0.0, 0.0, 180.0));
        ship.mode = Mode::JumpingIn { tick: 0 };
        ship
    }

    /// The last tick of `ship`'s glide in, flown at `speeds`, and its
    /// speed after it.
    fn glide_ends(ship: &mut Npc, speeds: TopSpeeds) -> (u32, f32) {
        for _ in 0..50 {
            let Mode::JumpingIn { tick } = ship.mode else {
                panic!("not gliding: {:?}", ship.mode);
            };
            fly_at(ship, &[], None, speeds);
            if !matches!(ship.mode, Mode::JumpingIn { .. }) {
                return (tick, ship.state.velocity.length());
            }
            assert_eq!(ship.mode, Mode::JumpingIn { tick: tick + 1 });
        }
        panic!("never ended")
    }

    #[test]
    fn an_arriving_ship_ends_its_glide_on_the_first_tick_its_next_speed_reaches_the_cap_keeping_its_last_speed()
     {
        // 50 - 1.165 x 38 = 5.73 is at or below 6; 50 - 1.165 x 37 = 6.895
        // is not.
        let speeds = TopSpeeds {
            own: 6.0,
            lead: Some(9.0),
        };
        let mut ship = jumping_in(AVERAGE);
        let (last, speed) = glide_ends(&mut ship, speeds);
        assert_eq!(last, 37);
        assert_eq!(ship.mode, Mode::Flying);
        assert!((speed - glide_speed(37)).abs() < 1e-4, "{speed}");
        assert!((speed - 6.895).abs() < 1e-3, "not snapped to 6: {speed}");
        let along = flight::facing(180.0) * glide_speed(37);
        assert!((ship.state.velocity - along).length() < 1e-4);
    }

    #[test]
    fn a_jumping_in_escort_whose_lead_is_slower_ends_its_glide_at_its_leads_top_speed() {
        // A cap of 9 would end it on tick 35 (50 - 1.165 x 36 = 8.06); the
        // lead's 3 ends it on tick 40 (50 - 1.165 x 41 = 2.235).
        let speeds = TopSpeeds {
            own: 9.0,
            lead: Some(3.0),
        };
        let mut ship = jumping_in(AVERAGE);
        let (last, speed) = glide_ends(&mut ship, speeds);
        assert_eq!(last, 40);
        assert!((speed - glide_speed(40)).abs() < 1e-4, "{speed}");
        assert!((speed - 3.4).abs() < 1e-3, "{speed}");
    }

    #[test]
    fn a_ship_whose_cap_is_the_glides_second_speed_glides_one_tick_at_50() {
        for cap in [glide_speed(1), 100.0] {
            let speeds = TopSpeeds {
                own: cap,
                lead: Some(cap + 1.0),
            };
            let mut ship = jumping_in(AVERAGE);
            assert_eq!(glide_ends(&mut ship, speeds), (0, 50.0), "{cap}");
            assert!((ship.state.position - Vec2::new(0.0, -1950.0)).length() < 1e-3);
        }
    }

    #[test]
    fn a_disabled_ship_jumping_in_glides_in_as_an_intact_one_does() {
        let speeds = TopSpeeds {
            own: 6.0,
            lead: Some(9.0),
        };
        let mut intact = jumping_in(AVERAGE);
        let mut disabled = intact.clone();
        disabled.condition = Condition::Disabled;
        while intact.mode != Mode::Flying {
            assert_eq!(fly_at(&mut disabled, &[], None, speeds), Outcome::Flying);
            fly_at(&mut intact, &[], None, speeds);
            assert_eq!(disabled.state, intact.state);
            assert_eq!(disabled.mode, intact.mode);
        }
        assert_eq!(disabled.condition, Condition::Disabled);
    }

    #[test]
    fn a_glide_in_ends_at_the_burst_cap_keeping_its_speed_intact_or_disabled() {
        // A lead's 3 caps it below its raised 4.5: 50 - 1.165 x 41 = 2.235
        // is the first at or below 3, so it keeps tick 40's 3.4.
        let speeds = TopSpeeds {
            own: 4.5,
            lead: Some(3.0),
        };
        for condition in [Condition::Intact, Condition::Disabled] {
            let mut ship = jumping_in(AVERAGE);
            ship.condition = condition;
            ship.mode = Mode::JumpingIn { tick: 40 };
            fly_at(&mut ship, &[], None, speeds);
            assert_eq!(ship.mode, Mode::Flying, "{condition:?}");
            let along = flight::facing(180.0) * glide_speed(40);
            assert!(
                (ship.state.velocity - along).length() < 1e-4,
                "{condition:?}: {:?}",
                ship.state.velocity
            );
        }
    }

    /// The leaderless ship of `fields`, bent on landing on planet 140 at
    /// the centre, out of a gate at (100, -200) heading 90° once its burst
    /// is over, drawing `draws`: its burst's last tick done.
    fn burst_out(fields: ShipFields, speeds: TopSpeeds, draws: &mut Draws) -> Npc {
        let sites = [planet(140, 0.0, 0.0)];
        let mut ship = emerging(fields, 1);
        ship.trigger = Trigger {
            primary: true,
            ..Trigger::default()
        };
        super::fly(&mut ship, &sites, None, speeds, draws);
        while let Mode::LeavingGate { .. } = ship.mode {
            super::fly(&mut ship, &sites, None, speeds, draws);
        }
        ship
    }

    #[test]
    fn a_leaderless_ship_leaving_a_gate_holds_its_course_for_30_to_59_ticks_before_its_ai_acts() {
        let sites = [planet(140, 0.0, 0.0)];
        let speeds = TopSpeeds {
            own: 3.0,
            lead: None,
        };
        assert_eq!((POST_BURST_WAIT, POST_BURST_SPREAD), (30, 30));
        for (draw, wait) in [(0, 30), (29, 59)] {
            let mut draws = Draws::of(&[draw]);
            let mut ship = burst_out(AVERAGE, speeds, &mut draws);
            assert_eq!(ship.mode, Mode::Coasting { ticks_left: wait });
            assert_eq!(draws.asked, [30], "one Rand(30)");
            assert_eq!(ship.trigger, Trigger::default(), "holding fire");
            // Top speed 3: on at 6, its last burst speed.
            let velocity = ship.state.velocity;
            assert!((velocity - Vec2::new(6.0, 0.0)).length() < 1e-4);
            for left in (0..wait).rev() {
                let before = ship.state;
                assert_eq!(
                    super::fly(&mut ship, &sites, None, speeds, &mut draws),
                    Outcome::Flying
                );
                assert_eq!(ship.state.velocity, velocity, "{left}: not slowed");
                assert_eq!(ship.state.heading, 90.0, "{left}: not turned");
                assert_eq!(ship.state.position, before.position + velocity);
                let mode = if left == 0 {
                    Mode::Flying
                } else {
                    Mode::Coasting { ticks_left: left }
                };
                assert_eq!(ship.mode, mode);
            }
            assert_eq!(draws.asked, [30], "no draw while it waits");
            super::fly(&mut ship, &sites, None, speeds, &mut draws);
            assert_ne!(ship.state.heading, 90.0, "turns for the planet");
            assert!(
                ship.state.velocity.length() <= 3.0 + 1e-4,
                "at its top speed"
            );
        }
    }

    #[test]
    fn a_ship_with_a_lead_does_not_wait_after_its_burst() {
        // A fleet escort's lead, or the player for a ship the player leads:
        // both are a lead's top speed here.
        for lead in [3.0, 9.0] {
            let speeds = TopSpeeds {
                own: 3.0,
                lead: Some(lead),
            };
            let mut draws = Draws::of(&[]);
            let ship = burst_out(AVERAGE, speeds, &mut draws);
            assert_eq!(ship.mode, Mode::Flying, "{lead}");
            assert!(draws.asked.is_empty(), "{lead}: no draw");
            let mut arriving = jumping_in(AVERAGE);
            glide_ends(&mut arriving, speeds);
            assert_eq!(arriving.mode, Mode::Flying, "{lead}");
        }
    }

    #[test]
    fn a_leaderless_arriving_ship_waits_after_its_glide() {
        let speeds = TopSpeeds {
            own: 6.0,
            lead: None,
        };
        for (draw, wait) in [(0, 30), (29, 59)] {
            let mut ship = jumping_in(AVERAGE);
            let mut draws = Draws::of(&[draw]);
            while let Mode::JumpingIn { .. } = ship.mode {
                super::fly(&mut ship, &[], None, speeds, &mut draws);
            }
            assert_eq!(ship.mode, Mode::Coasting { ticks_left: wait });
            assert_eq!(draws.asked, [30]);
            let along = flight::facing(180.0) * glide_speed(37);
            assert!((ship.state.velocity - along).length() < 1e-4);
            let before = ship.state;
            super::fly(&mut ship, &[], None, speeds, &mut draws);
            assert_eq!(ship.state.velocity, before.velocity, "coasting at 6.895");
            assert_eq!(ship.state.position, before.position + before.velocity);
        }
    }

    #[test]
    fn a_ship_not_intact_while_it_waits_drifts_instead() {
        let speeds = TopSpeeds {
            own: 3.0,
            lead: None,
        };
        let mut ship = burst_out(AVERAGE, speeds, &mut Draws::of(&[0]));
        ship.condition = Condition::Disabled;
        fly_at(&mut ship, &[], None, speeds);
        assert_eq!(ship.mode, Mode::Flying);
        assert!(
            ship.state.velocity.length() <= 3.0 + 1e-4,
            "drifting at its top speed"
        );
    }

    #[test]
    fn a_ship_jumping_out_heads_out_and_jumps_at_its_jump_distance() {
        for start in [
            at(100.0, -50.0, 0.0, 0.0, 180.0),
            at(0.0, 0.0, 0.0, 0.0, 30.0),
        ] {
            let mut ship = npc(FAST, Goal::JumpOut, start);
            let mut farthest = 0.0_f32;
            let (outcome, ticks) = (0..2000)
                .find_map(|tick| {
                    let outcome = fly(&mut ship, &[], None);
                    farthest = farthest.max(ship.state.position.length());
                    (outcome != Outcome::Flying).then_some((outcome, tick))
                })
                .expect("jumps");
            assert_eq!(outcome, Outcome::JumpedOut);
            assert!(ticks < 400, "{ticks}");
            assert!(farthest >= ship.stats.jump_distance, "{farthest}");
            assert!(farthest < ship.stats.jump_distance + 10.0, "{farthest}");
        }
    }

    #[test]
    fn a_ship_at_its_jump_distance_jumps_without_moving() {
        let mut ship = npc(FAST, Goal::JumpOut, at(0.0, -1000.0, 0.0, 0.0, 0.0));
        assert_eq!(fly(&mut ship, &[], None), Outcome::JumpedOut);
        assert_eq!(ship.state.position, Vec2::new(0.0, -1000.0));
        let mut short = npc(FAST, Goal::JumpOut, at(0.0, -999.0, 0.0, 0.0, 0.0));
        assert_eq!(fly(&mut short, &[], None), Outcome::Flying);
    }

    #[test]
    fn a_jump_in_glides_from_about_2098_out_without_steering_until_its_cap() {
        let start = hyperspace_entry(&mut crate::testkit::Draws::of(&[60]));
        assert!((start.position.length() - jump_in_distance()).abs() < 1e-2);
        let mut ship = npc(FAST, Goal::Land(StellarId(140)), start);
        ship.mode = Mode::JumpingIn { tick: 0 };
        let speeds = TopSpeeds {
            own: 6.0,
            lead: Some(9.0),
        };
        let sites = [planet(140, 0.0, 0.0)];
        let mut last = start.position.length();
        for tick in 0..=37 {
            assert_eq!(ship.mode, Mode::JumpingIn { tick });
            assert_eq!(fly_at(&mut ship, &sites, None, speeds), Outcome::Flying);
            let now = ship.state.position.length();
            assert!((last - now - glide_speed(tick)).abs() < 1e-2, "{tick}");
            assert_eq!(ship.state.heading, start.heading, "no steering");
            last = now;
        }
        assert_eq!(ship.mode, Mode::Flying);
        // Five ticks short of the 43 that would bring it 1000 out.
        let short: f32 = (38..43).map(glide_speed).sum();
        assert!((last - (1000.0 + short)).abs() < 0.1, "{last}");
        let inward = start.position * (-1.0 / jump_in_distance());
        assert!(
            (ship.state.velocity - inward * glide_speed(37)).length() < 1e-3,
            "{:?}",
            ship.state
        );
    }

    #[test]
    fn a_ship_with_no_speed_never_moves() {
        let still = ShipFields { speed: 0, ..FAST };
        let sites = [planet(140, 300.0, 0.0)];
        let lead = at(500.0, 0.0, 2.0, 0.0, 90.0);
        for goal in [
            Goal::Land(StellarId(140)),
            Goal::JumpOut,
            Goal::Follow(NpcId(0)),
            Goal::Idle,
        ] {
            let mut ship = npc(still, goal, at(0.0, 0.0, 0.0, 0.0, 45.0));
            for _ in 0..300 {
                assert_eq!(fly(&mut ship, &sites, Some(&lead)), Outcome::Flying);
            }
            assert_eq!(ship.state.position, Vec2::ZERO, "{goal:?}");
        }
    }

    #[test]
    fn an_idle_ship_or_one_whose_target_is_gone_brakes_to_a_stop() {
        for goal in [
            Goal::Idle,
            Goal::Land(StellarId(999)),
            Goal::Follow(NpcId(9)),
        ] {
            let mut ship = npc(FAST, goal, at(0.0, 0.0, 4.0, -3.0, 10.0));
            for _ in 0..200 {
                assert_eq!(fly(&mut ship, &[], None), Outcome::Flying);
            }
            assert!(
                ship.state.velocity.length() < 0.5,
                "{goal:?}: {:?}",
                ship.state
            );
        }
    }

    #[test]
    fn an_assisting_ship_comes_to_the_player_and_waits_docked_within_its_reach() {
        for help in [Help::Refuel, Help::Repair] {
            let player = at(0.0, 0.0, 0.0, 0.0, 0.0);
            let mut ship = npc(FAST, Goal::Assist(help), at(-600.0, 400.0, 0.0, 0.0, 0.0));
            for _ in 0..600 {
                assert_eq!(fly(&mut ship, &[], Some(&player)), Outcome::Flying);
            }
            let reach = assist::reach(help, ship.stats.handling.turn_rate);
            assert!(
                assist::within(&ship.state, &player, reach),
                "{help:?} {:?}",
                ship.state
            );
            assert!(assist::docked(&ship.state), "{help:?} {:?}", ship.state);
        }
    }

    #[test]
    fn outside_its_reach_an_assisting_ship_follows_the_player() {
        let player = at(0.0, 0.0, 1.0, 0.0, 0.0);
        // FAST turns 3° a tick: a refuel's reach is 105.
        for start in [
            at(-106.0, 0.0, 0.0, 0.0, 0.0),
            at(0.0, 300.0, 2.0, 1.0, 45.0),
        ] {
            let mut ship = npc(FAST, Goal::Assist(Help::Refuel), start);
            let mut escort = npc(FAST, Goal::Follow(NpcId(0)), start);
            fly(&mut ship, &[], Some(&player));
            fly(&mut escort, &[], Some(&player));
            assert_eq!(ship.state, escort.state, "{start:?}");
        }
    }

    #[test]
    fn within_its_reach_or_with_the_player_gone_an_assisting_ship_brakes() {
        let player = at(0.0, 0.0, 0.0, 0.0, 0.0);
        let start = at(-105.0, 105.0, 2.0, -1.0, 30.0);
        for (help, other) in [
            (Help::Refuel, Some(&player)),
            (Help::Repair, Some(&player)),
            (Help::Refuel, None),
        ] {
            let mut ship = npc(FAST, Goal::Assist(help), start);
            let mut idle = npc(FAST, Goal::Idle, start);
            for _ in 0..30 {
                fly(&mut ship, &[], other);
                fly(&mut idle, &[], other);
                assert_eq!(ship.state, idle.state, "{help:?} {other:?}");
            }
        }
        let moving = at(0.0, 0.0, 1.0, 0.0, 0.0);
        let mut ship = npc(
            FAST,
            Goal::Assist(Help::Refuel),
            at(-100.0, 0.0, 0.0, 0.0, 90.0),
        );
        for _ in 0..30 {
            fly(&mut ship, &[], Some(&moving));
        }
        assert_eq!(
            ship.state.position,
            Vec2::new(-100.0, 0.0),
            "within reach of a moving player it does not chase"
        );
        let far = at(-1000.0, 0.0, 2.0, 0.0, 30.0);
        let mut ship = npc(FAST, Goal::Assist(Help::Repair), far);
        let mut idle = npc(FAST, Goal::Idle, far);
        fly(&mut ship, &[], None);
        fly(&mut idle, &[], None);
        assert_eq!(ship.state, idle.state, "gone, from afar");
    }

    #[test]
    fn an_escort_catches_up_with_its_lead_and_keeps_with_it() {
        let mut lead = at(0.0, 0.0, 2.0, 0.0, 90.0);
        let mut ship = npc(
            FAST,
            Goal::Follow(NpcId(0)),
            at(-600.0, 400.0, 0.0, 0.0, 0.0),
        );
        for _ in 0..600 {
            lead.position = lead.position + lead.velocity;
            assert_eq!(fly(&mut ship, &[], Some(&lead)), Outcome::Flying);
        }
        let apart = (ship.state.position - lead.position).length();
        assert!(apart <= FOLLOW_DISTANCE * 1.5, "{apart}");
        assert!(
            (ship.state.velocity - lead.velocity).length() < 1.0,
            "{:?}",
            ship.state
        );
    }
    // The helpers, exactly.

    /// 6 pixels a tick at most, 0.3 more a tick, 3° a tick.
    fn fast() -> Handling {
        ShipStats::new(FAST, &[]).handling
    }

    #[test]
    fn the_stopping_distance_is_the_turn_then_the_braking() {
        let handling = Handling {
            max_speed: 3.0,
            accel: 0.1,
            turn_rate: 3.0,
        };
        // Moving up at 3, facing up: a half turn, 60 ticks, and one more,
        // then 3² / 0.2.
        let ahead = at(0.0, 0.0, 0.0, -3.0, 0.0);
        assert!((stopping_distance(&ahead, &handling) - (3.0 * 61.0 + 45.0)).abs() < 1e-3);
        // Facing 100° round: 80° to go, 27 ticks (26.7 rounded up).
        let partway = at(0.0, 0.0, 0.0, -3.0, 100.0);
        assert!((stopping_distance(&partway, &handling) - (3.0 * 28.0 + 45.0)).abs() < 1e-3);
        // Already facing back: no turn, but the tick's margin.
        let back = at(0.0, 0.0, 0.0, -2.0, 180.0);
        assert!((stopping_distance(&back, &handling) - (2.0 + 20.0)).abs() < 1e-3);
        assert_eq!(
            stopping_distance(&at(0.0, 0.0, 0.0, 0.0, 0.0), &handling),
            0.0
        );
    }

    #[test]
    fn a_ship_that_cannot_turn_or_thrust_never_stops() {
        let back = at(0.0, 0.0, 0.0, -2.0, 180.0);
        for handling in [
            Handling {
                turn_rate: 0.0,
                ..fast()
            },
            Handling {
                accel: 0.0,
                ..fast()
            },
        ] {
            assert_eq!(
                stopping_distance(&back, &handling),
                f32::INFINITY,
                "{handling:?}"
            );
            assert_eq!(braking_speed(500.0, &handling), 0.0, "{handling:?}");
        }
    }

    #[test]
    fn the_braking_speed_turns_and_stops_within_the_distance() {
        let handling = fast();
        let half_turn = 180.0 / handling.turn_rate;
        for distance in [0.0, 50.0, 250.0, 1000.0] {
            let speed = braking_speed(distance, &handling);
            let needed = speed.mul_add(half_turn, speed * speed / (2.0 * handling.accel));
            assert!((needed - distance).abs() < 1e-2, "{distance}: {speed}");
        }
        assert!(braking_speed(250.0, &handling) > braking_speed(50.0, &handling));
    }

    #[test]
    fn braking_turns_back_then_thrusts_while_thrust_slows_the_ship() {
        let handling = fast();
        let a = handling.accel;
        let brake_at =
            |speed: f32, heading: f32| brake(&at(0.0, 0.0, 0.0, -speed, heading), &handling);
        let turning = brake_at(3.0, 0.0);
        assert_eq!(
            (turning.reverse, turning.thrust, turning.turn),
            (true, false, Turn::None)
        );
        assert!(
            brake_at(3.0, 177.0).thrust,
            "within one turn of facing back"
        );
        assert!(!brake_at(3.0, 176.9).thrust);
        assert!(brake_at(a * 0.6, 180.0).thrust);
        assert!(!brake_at(a * 0.5, 180.0).thrust, "would only push it back");
        assert_eq!(brake_at(0.0, 90.0), Controls::default(), "at rest");
    }

    #[test]
    fn matching_a_velocity_coasts_within_half_a_ticks_thrust() {
        let handling = fast();
        let a = handling.accel;
        let still = at(0.0, 0.0, 0.0, 0.0, 0.0);
        let up = |speed: f32| Vec2::new(0.0, -speed);
        assert_eq!(
            match_velocity(&still, &handling, up(a * 0.4)),
            Controls::default()
        );
        let thrusting = match_velocity(&still, &handling, up(a * 0.6));
        assert_eq!((thrusting.thrust, thrusting.turn), (true, Turn::None));
    }

    #[test]
    fn matching_a_velocity_turns_only_beyond_half_a_ticks_turn() {
        let handling = fast();
        let facing = |heading: f32| {
            match_velocity(
                &at(0.0, 0.0, 0.0, 0.0, heading),
                &handling,
                Vec2::new(0.0, -6.0),
            )
        };
        // Wanting heading 0, from either side.
        assert_eq!(facing(358.5).turn, Turn::None, "1.5° short: half a turn");
        assert_eq!(facing(358.4).turn, Turn::Right);
        assert_eq!(facing(1.5).turn, Turn::None);
        assert_eq!(facing(1.6).turn, Turn::Left);
        assert_eq!(facing(358.8).turn, Turn::None);
        // Thrust within the aligned cone or a tick's turn, whichever is
        // wider.
        assert!(facing(5.0).thrust && !facing(5.1).thrust);
        let nimble = Handling {
            turn_rate: 8.0,
            ..handling
        };
        let wide = match_velocity(&at(0.0, 0.0, 0.0, 0.0, 7.9), &nimble, Vec2::new(0.0, -6.0));
        assert!(wide.thrust);
    }

    #[test]
    fn an_escort_beyond_its_distance_closes_no_faster_than_it_can_brake() {
        let handling = fast();
        let lead = at(0.0, 0.0, 0.0, 0.0, 0.0);
        // 150 below the lead, moving up at 2 and facing it: 50 beyond its
        // distance it can close at about 0.8, so it turns to slow down.
        let closing = at(0.0, 150.0, 0.0, -2.0, 0.0);
        let controls = follow(&closing, &handling, &lead);
        assert!(!controls.thrust, "{controls:?}");
        assert_ne!(controls.turn, Turn::None);
        // From 350 below it can close faster than 2: it thrusts on.
        let far = at(0.0, 350.0, 0.0, -2.0, 0.0);
        assert!(follow(&far, &handling, &lead).thrust);
    }

    #[test]
    fn an_escort_within_its_distance_matches_its_leads_velocity() {
        let handling = fast();
        let lead = at(10.0, 0.0, 1.0, 0.0, 90.0);
        let alongside = at(0.0, 0.0, 1.0, 0.0, 90.0);
        assert_eq!(follow(&alongside, &handling, &lead), Controls::default());
        let on_top = at(10.0, 0.0, 1.0, 0.0, 90.0);
        assert_eq!(follow(&on_top, &handling, &lead), Controls::default());
        let at_the_edge = at(-90.0, 0.0, 1.0, 0.0, 90.0);
        assert_eq!(follow(&at_the_edge, &handling, &lead), Controls::default());
    }

    #[test]
    fn a_ship_jumps_out_straight_away_from_the_centre() {
        let start = at(300.0, -400.0, 0.0, 0.0, 200.0);
        let mut ship = npc(FAST, Goal::JumpOut, start);
        let mut last = start.position;
        while fly(&mut ship, &[], None) == Outcome::Flying {
            last = ship.state.position;
        }
        let off = shortest_turn(heading_of(start.position), heading_of(last));
        assert!(off.abs() < 3.0, "{off}: {last:?}");
        // From the centre itself, straight ahead.
        let mut ship = npc(FAST, Goal::JumpOut, at(0.0, 0.0, 0.0, 0.0, 30.0));
        let mut last = Vec2::ZERO;
        while fly(&mut ship, &[], None) == Outcome::Flying {
            last = ship.state.position;
        }
        let off = shortest_turn(30.0, heading_of(last));
        assert!(off.abs() < 1e-3, "{off}: {last:?}");
    }

    // Fights.

    use crate::combat::ShipRef;

    const FOE: ShipRef = ShipRef::Player;

    /// A ship at rest at the centre facing `heading`, 3 degrees a tick,
    /// with `goal`.
    fn fighter(goal: Goal, heading: f32) -> Npc {
        npc(FAST, goal, at(0.0, 0.0, 0.0, 0.0, heading))
    }

    fn controls_of(ship: &Npc, other: &ShipState) -> Controls {
        let mut flown = ship.clone();
        let before = flown.state;
        fly(&mut flown, &[], Some(other));
        // Undo the step to read the controls from the change it made.
        let turned = shortest_turn(before.heading, flown.state.heading);
        let speed_change = (flown.state.velocity - before.velocity).length();
        Controls {
            thrust: speed_change > 0.0,
            turn: if turned > 0.0 {
                Turn::Right
            } else if turned < 0.0 {
                Turn::Left
            } else {
                Turn::None
            },
            reverse: false,
        }
    }

    #[test]
    fn attacking_from_afar_it_turns_to_the_lead_and_thrusts_within_four_turns() {
        // 3 degrees a tick: thrust within 12 of the target 1000 above.
        let target = at(0.0, -1000.0, 0.0, 0.0, 0.0);
        let thrusting = controls_of(&fighter(Goal::Attack(FOE), 12.0), &target);
        assert_eq!((thrusting.thrust, thrusting.turn), (true, Turn::Left));
        let turning = controls_of(&fighter(Goal::Attack(FOE), 12.1), &target);
        assert_eq!((turning.thrust, turning.turn), (false, Turn::Left));
        let other_way = controls_of(&fighter(Goal::Attack(FOE), 300.0), &target);
        assert_eq!(other_way.turn, Turn::Right);
    }

    #[test]
    fn dogfighting_within_165_it_thrusts_within_a_turn_and_15_degrees() {
        // 3 + 15: within 18 of the target 165 above.
        let target = at(165.0, -165.0, 0.0, 0.0, 0.0);
        let bearing = 45.0;
        let thrusting = controls_of(&fighter(Goal::Attack(FOE), bearing + 18.0), &target);
        assert!(thrusting.thrust);
        let turning = controls_of(&fighter(Goal::Attack(FOE), bearing + 18.1), &target);
        assert!(!turning.thrust);
        let afar = at(165.1, -165.0, 0.0, 0.0, 0.0);
        let approaching = controls_of(&fighter(Goal::Attack(FOE), bearing + 13.0), &afar);
        assert!(!approaching.thrust, "outside the box, within 12 only");
    }

    #[test]
    fn an_attacker_leads_a_moving_target_with_its_gun() {
        use crate::catalog::WeaponRecord;
        use crate::combat::armament::Armament;
        use crate::combat::weapon::WeaponSpec;
        let mut ship = fighter(Goal::Attack(FOE), 0.0);
        ship.armament = Armament::new([(
            WeaponSpec::new(&WeaponRecord {
                speed: 1000,
                count: 50,
                ..crate::testkit::weapon(128)
            }),
            1,
        )]);
        // 1000 above crossing right at 10 a tick: led 1000 to the right,
        // 45 degrees.
        let crossing = at(0.0, -1000.0, 10.0, 0.0, 0.0);
        let mut flown = ship.clone();
        for _ in 0..30 {
            fly(&mut flown, &[], Some(&crossing));
        }
        assert!(
            (flown.state.heading - 45.0).abs() < 3.0,
            "{:?}",
            flown.state
        );
    }

    #[test]
    fn sniping_it_brakes_to_a_stop_then_faces_its_target() {
        let target = at(-300.0, 0.0, 0.0, 0.0, 0.0);
        let mut moving = npc(FAST, Goal::Snipe(FOE), at(0.0, 0.0, 0.0, -3.0, 0.0));
        for _ in 0..200 {
            fly(&mut moving, &[], Some(&target));
        }
        assert!(moving.state.velocity.length() <= crate::flight::AT_REST_SPEED);
        let bearing = crate::combat::aim::bearing(moving.state.position, target.position);
        assert!(
            shortest_turn(moving.state.heading, bearing).abs() < 3.0,
            "{:?}",
            moving.state
        );
        let still = fighter(Goal::Snipe(FOE), 0.0);
        let controls = controls_of(&still, &target);
        assert_eq!((controls.thrust, controls.turn), (false, Turn::Left));
    }

    #[test]
    fn fleeing_close_it_turns_straight_away_and_thrusts_within_a_turn_and_20() {
        // The attacker 250 below: away is up, 0; thrust within 23.
        let attacker = at(0.0, 250.0, 0.0, 0.0, 0.0);
        let thrusting = controls_of(&fighter(Goal::Flee(FOE), 23.0), &attacker);
        assert_eq!((thrusting.thrust, thrusting.turn), (true, Turn::Left));
        let turning = controls_of(&fighter(Goal::Flee(FOE), 23.1), &attacker);
        assert!(!turning.thrust);
    }

    #[test]
    fn fleeing_from_afar_it_runs_out_from_the_centre_and_jumps_with_the_fuel() {
        // The attacker 251 right: run straight out, up from (0, -10).
        let attacker = at(251.0, -10.0, 0.0, 0.0, 0.0);
        let mut runner = npc(FAST, Goal::Flee(FOE), at(0.0, -10.0, 0.0, 0.0, 0.0));
        let (outcome, _) = (0..1000)
            .find_map(|tick| {
                let outcome = fly(&mut runner, &[planet(140, 0.0, -10.0)], Some(&attacker));
                (outcome != Outcome::Flying).then_some((outcome, tick))
            })
            .expect("leaves");
        assert_eq!(outcome, Outcome::JumpedOut, "never lands");
        assert!(runner.state.position.x.abs() < 1.0, "{:?}", runner.state);
        assert!(
            runner.state.position.length() >= runner.stats.jump_distance,
            "only past its jump distance: {:?}",
            runner.state
        );
        let mut dry = npc(FAST, Goal::Flee(FOE), at(0.0, -1000.0, 0.0, -6.0, 0.0));
        dry.reserves.fuel.now = 99.0;
        for _ in 0..30 {
            assert_eq!(fly(&mut dry, &[], Some(&attacker)), Outcome::Flying);
        }
        assert!(dry.state.position.y < -1100.0, "runs on: {:?}", dry.state);
    }

    #[test]
    fn inspecting_it_closes_in_and_keeps_alongside() {
        let target = at(400.0, 0.0, 0.0, 0.0, 0.0);
        let mut inspector = fighter(Goal::Inspect(FOE), 0.0);
        for _ in 0..600 {
            fly(&mut inspector, &[], Some(&target));
        }
        let apart = (inspector.state.position - target.position).length();
        assert!(
            apart <= FOLLOW_DISTANCE * 1.1,
            "{apart}: {:?}",
            inspector.state
        );
        assert!(inspector.state.velocity.length() < 0.5);
    }

    #[test]
    fn a_fight_whose_ship_is_gone_brakes() {
        for goal in [
            Goal::Attack(FOE),
            Goal::Snipe(FOE),
            Goal::Flee(FOE),
            Goal::Inspect(FOE),
        ] {
            let mut ship = npc(FAST, goal, at(0.0, 0.0, 4.0, -3.0, 10.0));
            for _ in 0..200 {
                assert_eq!(fly(&mut ship, &[], None), Outcome::Flying);
            }
            assert!(
                ship.state.velocity.length() < 0.5,
                "{goal:?}: {:?}",
                ship.state
            );
        }
    }

    // Docking.

    /// A fighter of `fields` returning from `start` to the player, its
    /// dock window 100 and the player's sprite 40 across.
    fn docking(fields: ShipFields, start: ShipState) -> Npc {
        Npc {
            ship: crate::catalog::ShipId(144),
            carrier: Some(crate::bay::Carrier {
                ship: ShipRef::Player,
                window: 100.0,
                reach: 40.0,
            }),
            ..npc(fields, Goal::Dock(ShipRef::Player), start)
        }
    }

    #[test]
    fn a_fighter_outside_its_dock_window_closes_as_on_a_slot_and_far_off_at_full_speed() {
        let carrier = at(0.0, 0.0, 0.0, 0.0, 0.0);
        // At full speed (3) straight down at the carrier from above.
        let mut near = docking(AVERAGE, at(0.0, -400.0, 0.0, 3.0, 180.0));
        let mut far = docking(AVERAGE, at(0.0, -700.0, 0.0, 3.0, 180.0));
        let before = (near.state, far.state);
        assert_eq!(fly(&mut near, &[], Some(&carrier)), Outcome::Flying);
        fly(&mut far, &[], Some(&carrier));
        assert_ne!(
            near.state.heading, before.0.heading,
            "400 off, it turns to slow down"
        );
        assert_eq!(
            far.state.heading, before.1.heading,
            "700 off, it keeps going"
        );
        assert_eq!(far.state.velocity, before.1.velocity);
        for distance in [101.0, 400.0, 700.0] {
            let mut ship = docking(AVERAGE, at(0.0, -distance, 0.0, 0.0, 180.0));
            fly(&mut ship, &[], Some(&carrier));
            assert!(ship.state.velocity.y > 0.0, "{distance}: {:?}", ship.state);
        }
    }

    #[test]
    fn inside_its_window_a_fighter_turns_at_the_carrier_and_thrusts_within_a_turn_and_a_degree() {
        let carrier = at(0.0, 0.0, 0.0, 0.0, 0.0);
        // 60 above the carrier, facing away: it turns, and only turns.
        let mut away = docking(AVERAGE, at(0.0, -60.0, 0.0, 0.0, 0.0));
        fly(&mut away, &[], Some(&carrier));
        assert_ne!(away.state.heading, 0.0, "turning");
        assert_eq!(away.state.velocity, Vec2::ZERO, "no thrust");
        // A turn (1) and 1.0 off the bearing (180): it thrusts.
        for heading in [178.0, 182.0, 180.0] {
            let mut aimed = docking(AVERAGE, at(0.0, -60.0, 0.0, 0.0, heading));
            fly(&mut aimed, &[], Some(&carrier));
            assert!(aimed.state.velocity.length() > 0.0, "{heading}");
        }
        for heading in [177.0, 183.0] {
            let mut aside = docking(AVERAGE, at(0.0, -60.0, 0.0, 0.0, heading));
            fly(&mut aside, &[], Some(&carrier));
            assert_eq!(aside.state.velocity, Vec2::ZERO, "{heading}");
        }
        assert_eq!(DOCK_TURN_SLACK, 1.0);
    }

    #[test]
    fn inside_its_window_a_fighter_is_pulled_by_ten_ticks_thrust_on_each_axis_beyond_it() {
        let carrier = at(0.0, 0.0, 0.0, 0.0, 0.0);
        // At rest, facing away from the carrier at (-70, 2.5): FAST pulls
        // 0.3 x 10 = 3 a tick, and y is within that.
        let mut ship = docking(FAST, at(-70.0, 2.5, 0.0, 0.0, 270.0));
        fly(&mut ship, &[], Some(&carrier));
        assert_eq!(ship.state.position, Vec2::new(-67.0, 2.5));
        let mut both = docking(FAST, at(50.0, 60.0, 0.0, 0.0, 135.0));
        fly(&mut both, &[], Some(&carrier));
        assert_eq!(both.state.position, Vec2::new(47.0, 57.0));
        // An axis exactly the pull off is within it, and stays.
        let pull = ShipStats::new(FAST, &[]).handling.accel * FORMATION_NUDGE;
        let mut edge = docking(FAST, at(-pull, 60.0, 0.0, 0.0, 135.0));
        fly(&mut edge, &[], Some(&carrier));
        assert_eq!(edge.state.position.x, -pull, "not farther than the pull");
        assert_eq!(edge.state.position.y, 60.0 - pull);
    }

    #[test]
    fn a_sluggish_fighter_far_off_on_one_axis_flies_at_full_speed() {
        // 0.01 of acceleration and half a degree a tick: it could not
        // brake from its top speed within 700.
        const SLUGGISH: ShipFields = ShipFields {
            accel: 30,
            maneuver: 5,
            ..AVERAGE
        };
        let carrier = at(0.0, 0.0, 0.0, 0.0, 0.0);
        let start = at(0.0, -700.0, 0.0, 3.0, 180.0);
        let mut far = docking(SLUGGISH, start);
        fly(&mut far, &[], Some(&carrier));
        assert_eq!(far.state.heading, 180.0, "700 off on y, it keeps going");
        assert_eq!(far.state.velocity, start.velocity);
        let mut near = docking(SLUGGISH, at(0.0, -500.0, 0.0, 3.0, 180.0));
        fly(&mut near, &[], Some(&carrier));
        assert_ne!(near.state.heading, 180.0, "500 off, it turns to brake");
    }

    #[test]
    fn within_the_carriers_reach_on_both_axes_a_fighter_docks() {
        let carrier = at(500.0, 500.0, 1.0, 1.0, 0.0);
        for (x, y) in [(40.0, 40.0), (-40.0, 0.0), (0.0, -40.0), (0.0, 0.0)] {
            let mut ship = docking(FAST, at(500.0 + x, 500.0 + y, 0.0, 0.0, 0.0));
            assert_eq!(
                fly(&mut ship, &[], Some(&carrier)),
                Outcome::Docked {
                    carrier: ShipRef::Player,
                    ship: crate::catalog::ShipId(144)
                },
                "({x}, {y})"
            );
        }
        for (x, y) in [(41.0, 0.0), (0.0, -41.0), (60.0, 30.0)] {
            let mut ship = docking(FAST, at(500.0 + x, 500.0 + y, 0.0, 0.0, 0.0));
            assert_eq!(
                fly(&mut ship, &[], Some(&carrier)),
                Outcome::Flying,
                "({x}, {y})"
            );
        }
    }

    #[test]
    fn a_fighter_whose_carrier_is_gone_or_with_none_brakes() {
        let mut lost = docking(AVERAGE, at(0.0, 0.0, 0.0, 2.0, 180.0));
        assert_eq!(fly(&mut lost, &[], None), Outcome::Flying);
        assert_ne!(lost.state.heading, 180.0, "{:?}", lost.state);
        let carrier = at(0.0, 0.0, 0.0, 0.0, 0.0);
        let mut carrierless = docking(AVERAGE, at(0.0, 0.0, 0.0, 2.0, 180.0));
        carrierless.carrier = None;
        assert_eq!(fly(&mut carrierless, &[], Some(&carrier)), Outcome::Flying);
        assert_ne!(carrierless.state.heading, 180.0, "{:?}", carrierless.state);
    }
}
