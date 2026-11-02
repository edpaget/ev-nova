//! A ship held at its gate bursting out once its hold ends, as
//! `_HandleShip` and `_LowLevelAIHandler` have it (addresses in the
//! `EV Nova` executable).
//!
//! - **It owes the burst** (F1): the emerge sets the burst word +0x34 to
//!   -30.0, or -15.0 when the player leads the ship (@0x895aa-0x895bd).
//!   Each frame the low level's preamble zeroes the thrust word +0x30 and
//!   keeps a negative +0x34 (@0x851e9-0x8520d), and while the hold's
//!   +0x4c runs the high level returns at once (@0x8d4b4-0x8d4d8), so
//!   nothing writes +0x34 through the hold. [`Npc::end_wait`] sets
//!   [`Npc::owed_burst`] as the hold ends, by the engine's reading of
//!   [`RuleKey::HeldGateBurst`](crate::RuleKey::HeldGateBurst); by the
//!   other it owes none.
//! - **What drops it** (F8): every substate's thrust but those of subs 1,
//!   0xe and 0x12 (below) and sub 8 (its own word, below) writes +0x34
//!   with +0x30 (subs 2, 3, 5, 6, 7, 9, 0xb, 0xc's
//!   counter-thrust, 0xf, 0x10, 0x11), so the first such thrust drops it
//!   ([`Word::Writes`], [`step`]; what thrusts is [`writes_thrust`]). An
//!   inertialess dogfighter matching its target's speed writes +0x34
//!   whether or not it thrusts (@0x860a6), which the autopilot's fight
//!   drops it for.
//! - **What keeps it** (F5, F6): the swarm's thrust (sub 0x12, @0x8533d),
//!   sub 1's brakes (inertialess -0.5 × `_ShipAccelRate` @0x853e0-0x853fe;
//!   inertial, its full thrust from 1.75 on an axis @0x85510-0x8551e or
//!   half of it @0x85526-0x8558d) and sub 0xe's (inertialess
//!   -`_ShipAccelRate` @0x8710f-0x8713a; inertial, its thrust
//!   @0x87244-0x87252) write +0x30 alone ([`Word::Keeps`]). Subs 1 and
//!   0xe thrust only while the ship moves 0.35 or more on an axis
//!   (@0x853c3, @0x870db, @0x87101), which a held ship does once a hit
//!   pushes it, as it drifts through its hold (F7).
//! - **The burst** (F2, [`step`]): in a frame where +0x30 is not 0 and
//!   +0x34 is below 0 (@0x34a7b-0x34a84, @0x34b00-0x34b03), the ship,
//!   turned first as its controls say (@0x3443f-0x3458e), moves along its
//!   heading at |+0x34| (`_Accel` @0x34bf5-0x34c2a; an inertialess one's
//!   speed +0x48 set to it, @0x34bdf-0x34bf0) in place of its thrust,
//!   and +0x34 rises by |+0x30| (@0x34c2f-0x34c63): it slows by that
//!   frame's thrust, not by the gate burst's 1.165.
//! - **The end** (F2, F3, [`settle`]): in a frame that bursts, however
//!   its word was written, once +0x34 is at or above minus the cap
//!   ([`TopSpeeds::burst_cap`], @0x34c68-0x34d2a),
//!   `_AIMakeShipStopJumping` (@0x82e77-0x82eba) sets goal and sub-goal 0
//!   unless the goal is 9 or 0xf (@0x82e7d-0x82e9f), its travel stellar
//!   +0x6c to -1, its stage +0x50 to 0 and +0x30 = +0x34 = 0, keeping its
//!   target +0x70; an inertialess ship's velocity is zeroed
//!   (@0x34d37-0x34d59); and a ship with no leader waits Rand(30) + 30
//!   (@0x34d5c-0x34d8b), as after a gate's own burst
//!   ([`autopilot::after_burst`]). Its glow falls on the end frame, as
//!   +0x30 is zeroed before the glow reads it (F4, @0x34eec-0x34fc4).
//! - **The last frame fires**: `_HandleShip` tests the wait once, before
//!   the burst block (@0x34a54-0x34a72), and its fire step runs after the
//!   end and reads only the low level's flag and pick (@0x34fcb-0x35018).
//!   So a ship with no leader fires what its low level flagged that frame,
//!   a swarmer's missile (@0x85343-0x85357) say, and nothing through the
//!   wait after ([`Npc::fires_on_burst_end`]).
//! - **Sub 0xd's word**: a follower's stage writes +0x30 = 0 and
//!   +0x34 = -1.0 every frame it gets there, over any word it held
//!   (@0x85bcf-0x85c5a; [`FOLLOW_BURST`], by the engine's reading of
//!   [`RuleKey::HeldGateBurst`](crate::RuleKey::HeldGateBurst)). With
//!   +0x30 at 0 that frame doesn't burst (@0x34a84). No jump given up
//!   writes +0x34, so its next thrust of sub 1, 0xe or 0x12 bursts at 1.0
//!   and, for any ship of top speed above 1.0 less that thrust, ends at
//!   once. Sub 3 writes it inside its thrust window even under way, where
//!   `_HandleShip` then skips the thrust (@0x85845-0x85858, @0x34a96). A
//!   jump completed keeps nothing: a ship gone is gone, and a player's
//!   escort comes back through `_RespawnEscort` (@0x3d4f1), which zeroes
//!   the word, as `Session::place_escort` builds a new ship.
//! - **Sub 8's word** ([`dock_flight`](crate::traffic::dock_flight)): a
//!   docking fighter lowers +0x34 by its pull, 10 × its acceleration,
//!   every frame past its dock test (@0x880c0-0x880c9). Inside its thrust
//!   window it first writes +0x30 = its acceleration and +0x34 = 0
//!   (@0x87ef8), so its thrust always bursts at the pull, over whatever it
//!   owed, and leaves it owing 9 × its acceleration ([`Word::Keeps`]).
//!   The burst ends that frame once that is within its cap, its lead
//!   sparing it the wait. Outside the window the word is lowered with no
//!   burst, which [`settle`] doesn't end. Double time's second pass writes
//!   no word and bursts on what the advance pass left.
//! - **Goal 0 keeps it**: sub 0 writes no +0x30, so a ship that gave up to
//!   goal 0 bursts nothing on its own (@0x34a84). The engine's
//!   [`Goal::Idle`] flies sub 0 itself, the head's word
//!   ([`SlotWord::none`](crate::traffic::slot_word::SlotWord::none)), keeping
//!   what it owes.
//! - **A ship disabled keeps it**: its +0x50 is -1.0, so it takes the
//!   ordinary path and the disabled reset, goal 0 every frame
//!   (@0x8fece-0x8ff13). Subs 1, 0xe and 0x12 all test not disabled
//!   (@0x85378, @0x870b3, @0x85252), so it writes no +0x30, the burst block
//!   is skipped (@0x34a84) and the preamble keeps the word. The autopilot's
//!   drift flies no [`step`], so it drifts dragged, and once repaired its
//!   next such thrust bursts out what is left, with no lead by then as the
//!   reset cleared +0x9a. A ship disabled in a gate's own burst or a glide
//!   is another matter: that block runs on
//!   ([`autopilot::burst_motion`]).
//!
//! Neither the burst nor `_AIMakeShipStopJumping` plays a sound. The
//! mining orders' thrusts are
//! rdm:phase/escort-fleet-fidelity-cross-roadmap/phase-2-mining-orders-burst-an-owed-burst's;
//! sub 0xc's inspection hold is dead code on any data
//! (rdm:phase/escort-fleet-fidelity-followups/phase-2-left-out-of-formation-flight).
//!
//! The engine sets the burst's velocity and moves the ship in the tick
//! its flight thrusts, where the original's move comes first in the next
//! frame's `_HandleShip` (@0x34274-0x342aa): as for the gate's own burst,
//! a one-tick lag no boundary the player sees falls in.

use crate::ai::Goal;
use crate::chance::Chance;
use crate::flight::{self, Controls, FlightRules};
use crate::handling::Handling;
use crate::inertialess::BRAKE;
use crate::traffic::autopilot::{self, TopSpeeds};
use crate::traffic::inertialess::Inertia;
use crate::traffic::npc::Npc;
use crate::traffic::ship_display::{self, Thrust};
use crate::traffic::slot_word::Cap;
#[cfg(doc)]
use crate::traffic::warp_out::FOLLOW_BURST;

/// What a flight's substate does this tick to the burst word +0x34 beside
/// its thrust word +0x30 (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Word {
    /// It writes +0x34 when it thrusts: every substate but 1, 0xe and
    /// 0x12, whose thrust drops an owed burst (F8).
    Writes,
    /// It leaves +0x34 alone (subs 1, 0xe and 0x12), writing +0x30 alone:
    /// `thrust` is |+0x30| in pixels a tick, 0 when it doesn't thrust.
    Keeps {
        /// |+0x30|.
        thrust: f32,
    },
}

impl Word {
    /// Sub 0x12's word, and an inertial sniper's (sub 0xe): its thrust,
    /// `flown`'s acceleration, while `controls` thrust.
    pub(crate) fn thrusting(controls: Controls, flown: &Handling) -> Self {
        Self::Keeps {
            thrust: if controls.thrust { flown.accel } else { 0.0 },
        }
    }

    /// Sub 1's word, as `inertia` says, while `controls` thrust
    /// ([`writes_thrust`]): an inertial ship's thrust, `flown`'s
    /// acceleration (half of it on the soft brake, which `flown` carries);
    /// an inertialess one's brake, |[`BRAKE`]| of it (@0x853f6).
    pub(crate) fn stop(controls: Controls, flown: &Handling, inertia: Inertia) -> Self {
        let thrust = match inertia {
            _ if !writes_thrust(controls, inertia) => 0.0,
            Inertia::Inertial => flown.accel,
            Inertia::Inertialess { .. } => flown.accel * -BRAKE,
        };
        Self::Keeps { thrust }
    }
}

/// Whether `controls` write the thrust word +0x30, as `inertia` says: an
/// inertial ship's thrust; an inertialess one's thrust or its brake,
/// reverse ([`flight::inertialess_step`]). No inertial NPC reverses.
pub(crate) fn writes_thrust(controls: Controls, inertia: Inertia) -> bool {
    controls.thrust || (matches!(inertia, Inertia::Inertialess { .. }) && controls.reverse)
}

/// What a low level leaves its ship this tick (see
/// [`slot_word`](crate::traffic::slot_word)): its controls but their
/// turn, its [`Word`], the handling it flies by when that isn't its
/// goal's, the heading it wants (+0x68) and where its thrust's top speed
/// is read.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Flight {
    /// The controls but their turn.
    pub controls: Controls,
    /// What it does to +0x34.
    pub word: Word,
    /// The handling it flies by, sub 1's (half thrust on its soft brake,
    /// [`leave_flight::stop`](crate::traffic::leave_flight::stop)) or a
    /// sub's multiple of its thrust or top speed; none flies by the
    /// goal's.
    pub flown: Option<Handling>,
    /// The heading it wants, +0x68: none keeps its heading.
    pub wanted: Option<i16>,
    /// Where its thrust's top speed is read.
    pub cap: Cap,
    /// Sub 0xe's inertialess brake (see [`SlotWord`](crate::traffic::slot_word::SlotWord)).
    pub speed_off: bool,
}

impl Flight {
    /// A flight of `controls` wanting `wanted`, its thrust capped as `cap`
    /// says, writing +0x34 as it thrusts ([`Word::Writes`]).
    pub(crate) fn writes(controls: Controls, wanted: i16, cap: Cap) -> Self {
        Self {
            controls,
            word: Word::Writes,
            flown: None,
            wanted: Some(wanted),
            cap,
            speed_off: false,
        }
    }

    /// Sub 1's flight under `controls` wanting `wanted`, flown on `flown`
    /// as `inertia` says ([`Word::stop`]), keeping the head's +0x34.
    pub(crate) fn stop(
        controls: Controls,
        flown: &Handling,
        inertia: Inertia,
        wanted: i16,
    ) -> Self {
        Self {
            controls,
            word: Word::stop(controls, flown, inertia),
            flown: Some(*flown),
            wanted: Some(wanted),
            cap: Cap::Slot,
            speed_off: false,
        }
    }

    /// This flight flown by `flown`.
    #[must_use]
    pub(crate) fn by(self, flown: Handling) -> Self {
        Self {
            flown: Some(flown),
            ..self
        }
    }

    /// This flight doing `word` to +0x34.
    #[must_use]
    pub(crate) fn doing(self, word: Word) -> Self {
        Self { word, ..self }
    }
}

/// Flies `npc` one tick under `controls` on `handling` under `flight`, as
/// `inertia` says, its substate doing `word` to +0x34 (see the module
/// docs), an inertialess ship's velocity pulled by `adjust`
/// ([`AtStep::adjust_accel`](crate::traffic::slot_word::AtStep::adjust_accel)):
/// owing a burst under a [`Word::Keeps`] that thrusts, it bursts
/// ([`burst`]), marking that it did
/// ([`Drive::burst`](ship_display::Drive::burst)); under one that doesn't, it flies keeping what it owes
/// ([`autopilot::plain_step`]); under [`Word::Writes`] it flies as ever,
/// its thrust ([`writes_thrust`]) dropping what it owes (F8).
pub(crate) fn step(
    npc: &mut Npc,
    handling: &Handling,
    controls: Controls,
    flight: FlightRules,
    (inertia, word, adjust): (Inertia, Word, f32),
) {
    match (word, npc.owed_burst) {
        (Word::Keeps { thrust }, Some(speed)) if thrust > 0.0 => {
            burst(
                npc,
                handling,
                controls,
                flight,
                (inertia, adjust),
                (speed, thrust),
            );
        }
        (Word::Keeps { .. }, _) => {
            autopilot::plain_step(npc, handling, controls, flight, (inertia, adjust));
        }
        (Word::Writes, _) => {
            if writes_thrust(controls, inertia) {
                npc.owed_burst = None;
            }
            autopilot::plain_step(npc, handling, controls, flight, (inertia, adjust));
        }
    }
}

/// One tick of `npc`'s burst at `speed`, its thrust word `thrust` (F2):
/// it records its drive and turns as [`autopilot::plain_step`] would under
/// `controls` on `handling` under `flight`, as `inertia` says, then moves
/// along its new heading at `speed` in place of its thrust
/// ([`autopilot::burst_motion`], an inertialess one's velocity pulled by
/// `adjust`), and owes `thrust` less.
fn burst(
    npc: &mut Npc,
    handling: &Handling,
    controls: Controls,
    flight: FlightRules,
    (inertia, adjust): (Inertia, f32),
    (speed, thrust): (f32, f32),
) {
    ship_display::record(npc, handling, controls, flight);
    npc.state.heading = flight::turned(npc.state.heading, controls.turn, handling.turn_rate);
    let pulled = Handling {
        accel: adjust,
        ..*handling
    };
    autopilot::burst_motion(npc, speed, &pulled, inertia);
    npc.owed_burst = Some(speed - thrust);
    npc.display.drive.burst = true;
}

/// Ends `npc`'s burst after its flight this tick, `burst` whether a burst
/// ran in it ([`Drive::burst`](ship_display::Drive::burst)), as
/// `_HandleShip` tests the end inside its burst block (@0x34d10-0x34d2a),
/// however the word it burst on was written (F2). A word changed with no
/// burst, as sub 8 lowers it outside its window or sub 0xd writes its 1.0,
/// doesn't end. Once a burst has left what it owes at or
/// below its cap at `speeds` ([`TopSpeeds::burst_cap`]), it stops jumping
/// ([`stop_jumping`]), its glow falls (F4), and, as `inertia` says, it
/// ends as a gate's own burst does ([`autopilot::after_burst`]: an
/// inertialess ship's velocity zeroed, and a ship with no lead waiting
/// its Rand(30) + 30, rolled on `chance`). This tick's fire step still
/// runs, as it tests no wait: a ship now waiting keeps the trigger
/// [`autopilot::after_burst`] released, marked
/// ([`Npc::fires_on_burst_end`]). A ship with a lead takes no wait.
/// Whether it stopped jumping, so the caller zeroes the word it flies by
/// ([`SlotWord::stop_jumping`](crate::traffic::slot_word::SlotWord::stop_jumping)):
/// `_AIMakeShipStopJumping` zeroes +0x30 and +0x34 (@0x82eb3-0x82eb6), so
/// double time's second pass, which runs no low level, thrusts nothing.
pub(crate) fn settle(
    npc: &mut Npc,
    burst: bool,
    speeds: TopSpeeds,
    inertia: Inertia,
    chance: &mut (impl Chance + ?Sized),
) -> bool {
    let Some(now) = npc.owed_burst.filter(|_| burst) else {
        return false;
    };
    if now > speeds.burst_cap() {
        return false;
    }
    stop_jumping(npc);
    npc.display.drive.thrust = Thrust::None;
    let trigger = npc.trigger;
    autopilot::after_burst(npc, speeds, inertia, chance);
    if npc.wait_running() {
        npc.trigger = trigger;
        npc.fires_on_burst_end = true;
    }
    true
}

/// `_AIMakeShipStopJumping` (@0x82e77-0x82eba, F3): goal and sub-goal 0
/// ([`Goal::Idle`], its kept pursuit, docking and any sub 4 or 0xd
/// dropped) unless it assists the player (goal 9 or 0xf, @0x82e96); its
/// travel stellar cleared (+0x6c = -1), its stage ended (+0x50 = 0,
/// @0x82eb0) and its burst gone (+0x34 = 0). Its target is kept.
fn stop_jumping(npc: &mut Npc) {
    if !matches!(npc.goal, Goal::Assist(_)) {
        npc.goal = Goal::Idle;
        npc.pursuit.kept = None;
        npc.docking = None;
        npc.warp.goal = None;
    }
    npc.travel = None;
    npc.warp.stage = 0.0;
    npc.owed_burst = None;
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::pursuit::Kept;
    use crate::catalog::WeaponId;
    use crate::combat::ShipRef;
    use crate::combat::armament::Trigger;
    use crate::flight::{ShipState, Turn, facing};
    use crate::geometry::Vec2;
    use crate::hail::Help;
    use crate::stats::ShipStats;
    use crate::traffic::npc::Mode;
    use crate::traffic::plunder_dock::{DockSub, Docking};
    use crate::traffic::warp_out::WarpGoal;
    use nova_sim_base::fixture::{Draws, FAST};

    const INERTIALESS: Inertia = Inertia::Inertialess { speed: 2.0 };

    /// A ship of [`FAST`] at (100, 200) moving (2, 0), heading 90°, owing
    /// a burst of `owed`.
    fn owing(owed: Option<f32>) -> Npc {
        let mut npc = crate::traffic::fixture::npc(1, ShipStats::new(FAST, &[]));
        npc.state = ShipState {
            position: Vec2::new(100.0, 200.0),
            velocity: Vec2::new(2.0, 0.0),
            heading: 90.0,
        };
        npc.speed = 2.0;
        npc.owed_burst = owed;
        npc
    }

    fn handling(npc: &Npc) -> Handling {
        npc.stats.handling
    }

    fn close(a: Vec2, b: Vec2) -> bool {
        (a - b).length() < 1e-4
    }

    #[test]
    fn bursting_it_turns_as_its_controls_say_then_moves_along_its_heading_at_what_it_owes() {
        let mut npc = owing(Some(30.0));
        let flown = handling(&npc);
        let controls = Controls {
            thrust: true,
            turn: Turn::Right,
            ..Controls::default()
        };
        step(
            &mut npc,
            &flown,
            controls,
            FlightRules::default(),
            (Inertia::Inertial, Word::Keeps { thrust: 0.25 }, flown.accel),
        );
        let heading = 90.0 + flown.turn_rate;
        assert_eq!(npc.state.heading, heading);
        assert!(close(npc.state.velocity, facing(heading) * 30.0));
        assert!(close(
            npc.state.position,
            Vec2::new(100.0, 200.0) + facing(heading) * 30.0
        ));
        assert_eq!(npc.owed_burst, Some(29.75), "less this tick's thrust");
        assert_eq!(
            npc.display.drive.thrust,
            Thrust::Plain,
            "its drive recorded"
        );
    }

    #[test]
    fn an_inertialess_ship_bursting_takes_what_it_owes_as_its_speed_and_ignores_reverse() {
        let mut npc = owing(Some(30.0));
        let flown = handling(&npc);
        let controls = Controls {
            reverse: true,
            turn: Turn::Left,
            ..Controls::default()
        };
        let mut drifted = npc.clone();
        drifted.state.heading = 90.0 - flown.turn_rate;
        crate::traffic::inertialess::drift(&mut drifted, &flown);
        step(
            &mut npc,
            &flown,
            controls,
            FlightRules::default(),
            (INERTIALESS, Word::Keeps { thrust: 0.5 }, flown.accel),
        );
        assert_eq!(npc.state.heading, 90.0 - flown.turn_rate);
        assert_eq!(npc.speed, 30.0, "+0x48 = |+0x34|");
        assert!(close(npc.state.velocity, drifted.state.velocity));
        assert!(close(npc.state.position, drifted.state.position));
        assert_eq!(npc.owed_burst, Some(29.5));
    }

    #[test]
    fn a_keeping_tick_that_does_not_thrust_flies_as_ever_and_keeps_what_it_owes() {
        let controls = Controls {
            reverse: true,
            ..Controls::default()
        };
        for inertia in [Inertia::Inertial, INERTIALESS] {
            let mut npc = owing(Some(30.0));
            let flown = handling(&npc);
            let mut plain = npc.clone();
            autopilot::plain_step(
                &mut plain,
                &flown,
                controls,
                FlightRules::default(),
                (inertia, flown.accel),
            );
            step(
                &mut npc,
                &flown,
                controls,
                FlightRules::default(),
                (inertia, Word::Keeps { thrust: 0.0 }, flown.accel),
            );
            assert_eq!(npc.owed_burst, Some(30.0), "{inertia:?}: kept");
            assert_eq!(npc.state, plain.state, "{inertia:?}");
            assert_eq!(npc.speed, plain.speed, "{inertia:?}");
        }
    }

    #[test]
    fn with_nothing_owed_a_keeping_thrust_flies_as_ever() {
        let mut npc = owing(None);
        let flown = handling(&npc);
        let controls = Controls {
            thrust: true,
            ..Controls::default()
        };
        let mut plain = npc.clone();
        autopilot::plain_step(
            &mut plain,
            &flown,
            controls,
            FlightRules::default(),
            (Inertia::Inertial, flown.accel),
        );
        step(
            &mut npc,
            &flown,
            controls,
            FlightRules::default(),
            (Inertia::Inertial, Word::Keeps { thrust: 0.3 }, flown.accel),
        );
        assert_eq!(npc.state, plain.state);
        assert_eq!(npc.owed_burst, None);
    }

    #[test]
    fn a_writing_thrust_drops_what_it_owes_and_flies_as_ever() {
        let thrust = Controls {
            thrust: true,
            ..Controls::default()
        };
        let reverse = Controls {
            reverse: true,
            ..Controls::default()
        };
        for (inertia, controls) in [
            (Inertia::Inertial, thrust),
            (INERTIALESS, thrust),
            (INERTIALESS, reverse),
        ] {
            let mut npc = owing(Some(30.0));
            let flown = handling(&npc);
            let mut plain = npc.clone();
            autopilot::plain_step(
                &mut plain,
                &flown,
                controls,
                FlightRules::default(),
                (inertia, flown.accel),
            );
            step(
                &mut npc,
                &flown,
                controls,
                FlightRules::default(),
                (inertia, Word::Writes, flown.accel),
            );
            assert_eq!(npc.owed_burst, None, "{inertia:?} {controls:?}");
            assert_eq!(npc.state, plain.state, "{inertia:?} {controls:?}");
        }
        // No thrust writes nothing.
        for (inertia, controls) in [
            (Inertia::Inertial, Controls::default()),
            (INERTIALESS, Controls::default()),
        ] {
            let mut npc = owing(Some(30.0));
            let flown = handling(&npc);
            step(
                &mut npc,
                &flown,
                controls,
                FlightRules::default(),
                (inertia, Word::Writes, flown.accel),
            );
            assert_eq!(npc.owed_burst, Some(30.0), "{inertia:?} {controls:?}");
        }
    }

    #[test]
    fn sub_1s_word_is_its_full_soft_or_inertialess_brake_and_the_swarms_its_thrust() {
        let flown = Handling {
            accel: 0.4,
            ..owing(None).stats.handling
        };
        let soft = Handling {
            accel: 0.2,
            ..flown
        };
        let thrust = Controls {
            thrust: true,
            ..Controls::default()
        };
        let reverse = Controls {
            reverse: true,
            ..Controls::default()
        };
        let keeps = |thrust| Word::Keeps { thrust };
        assert_eq!(Word::stop(thrust, &flown, Inertia::Inertial), keeps(0.4));
        assert_eq!(Word::stop(thrust, &soft, Inertia::Inertial), keeps(0.2));
        assert_eq!(Word::stop(reverse, &flown, Inertia::Inertial), keeps(0.0));
        assert_eq!(
            Word::stop(reverse, &flown, INERTIALESS),
            keeps(0.2),
            "-0.5 × accel"
        );
        assert_eq!(
            Word::stop(Controls::default(), &flown, INERTIALESS),
            keeps(0.0)
        );
        assert_eq!(Word::thrusting(thrust, &flown), keeps(0.4));
        assert_eq!(Word::thrusting(reverse, &flown), keeps(0.0));
        assert_eq!(
            Flight::writes(thrust, 90, Cap::Step),
            Flight {
                controls: thrust,
                word: Word::Writes,
                flown: None,
                wanted: Some(90),
                cap: Cap::Step,
                speed_off: false,
            }
        );
        assert_eq!(
            Flight::stop(thrust, &soft, Inertia::Inertial, 45),
            Flight {
                controls: thrust,
                word: keeps(0.2),
                flown: Some(soft),
                wanted: Some(45),
                cap: Cap::Slot,
                speed_off: false,
            },
            "sub 1 keeps the head's +0x34"
        );
    }

    /// A ship that now owes `now`, having burst this tick as `burst` says,
    /// attacking the player with a pursuit kept and a docking, its travel
    /// stellar set and its drive thrusting.
    fn settled(
        now: f32,
        burst: bool,
        speeds: TopSpeeds,
        inertia: Inertia,
        draws: &mut Draws,
    ) -> Npc {
        let mut npc = owing(Some(now));
        npc.goal = Goal::Attack(ShipRef::Player);
        npc.target = Some(ShipRef::Player);
        npc.pursuit.kept = Some(Kept::Rush);
        npc.docking = Some(Docking {
            ship: ShipRef::Player,
            sub: DockSub::Close,
        });
        npc.travel = Some(crate::catalog::StellarId(150));
        npc.display.drive.thrust = Thrust::Plain;
        settle(&mut npc, burst, speeds, inertia, draws);
        npc
    }

    #[test]
    fn a_burst_lowered_to_its_cap_stops_jumping_and_waits_30_to_59_with_no_lead() {
        let own = TopSpeeds {
            own: 6.0,
            lead: None,
        };
        let mut draws = Draws::of(&[7]);
        let ended = settled(6.0, true, own, Inertia::Inertial, &mut draws);
        assert_eq!(ended.goal, Goal::Idle, "goal 0");
        assert_eq!(
            (ended.pursuit.kept, ended.docking),
            (None, None),
            "sub-goal 0"
        );
        assert_eq!(ended.target, Some(ShipRef::Player), "+0x70 kept");
        assert!(!ended.travel_set(), "+0x6c = -1");
        assert_eq!(ended.owed_burst, None);
        assert_eq!(ended.display.drive.thrust, Thrust::None, "+0x30 = 0");
        assert_eq!(ended.mode, Mode::Coasting { ticks_left: 37 });
        assert_eq!(draws.asked, [30], "Rand(30)");
        assert!(close(ended.state.velocity, Vec2::new(2.0, 0.0)), "kept");
        // An inertialess ship's velocity is zeroed.
        let ended = settled(6.0, true, own, INERTIALESS, &mut Draws::of(&[0]));
        assert_eq!(ended.state.velocity, Vec2::ZERO);
        // With a lead it flies on, its cap the lower top speed.
        let led = TopSpeeds {
            own: 9.0,
            lead: Some(6.0),
        };
        let mut draws = Draws::of(&[]);
        let ended = settled(5.9, true, led, Inertia::Inertial, &mut draws);
        assert_eq!((ended.mode, ended.owed_burst), (Mode::Flying, None));
        assert_eq!(draws.asked, Vec::<u32>::new());
    }

    #[test]
    fn a_burst_ending_in_a_wait_keeps_its_trigger_for_this_ticks_fire_and_is_marked() {
        // `_HandleShip`'s fire step comes after the end and tests no wait
        // (@0x34d32-0x34d8b, @0x34fcb-0x35018).
        let held = Trigger {
            only: Some(WeaponId(131)),
            keep: Some(WeaponId(131)),
            point_defence: true,
            ..Trigger::default()
        };
        let own = TopSpeeds {
            own: 6.0,
            lead: None,
        };
        let mut npc = owing(Some(6.0));
        npc.trigger = held;
        settle(&mut npc, true, own, Inertia::Inertial, &mut Draws::of(&[0]));
        assert!(matches!(npc.mode, Mode::Coasting { .. }));
        assert_eq!(npc.trigger, held);
        assert!(npc.fires_on_burst_end);
    }

    #[test]
    fn a_burst_ending_with_a_lead_takes_no_wait_and_needs_no_mark() {
        let held = Trigger {
            only: Some(WeaponId(131)),
            ..Trigger::default()
        };
        let led = TopSpeeds {
            own: 6.0,
            lead: Some(6.0),
        };
        let mut npc = owing(Some(6.0));
        npc.trigger = held;
        settle(&mut npc, true, led, Inertia::Inertial, &mut Draws::of(&[]));
        assert_eq!(npc.mode, Mode::Flying);
        assert_eq!(npc.trigger, held);
        assert!(!npc.fires_on_burst_end);
    }

    #[test]
    fn a_burst_going_on_is_not_marked() {
        let own = TopSpeeds {
            own: 6.0,
            lead: None,
        };
        let mut npc = owing(Some(7.0));
        settle(&mut npc, true, own, Inertia::Inertial, &mut Draws::of(&[]));
        assert!(!npc.fires_on_burst_end);
    }

    #[test]
    fn a_burst_above_its_cap_or_a_word_changed_with_no_burst_goes_on() {
        let own = TopSpeeds {
            own: 6.0,
            lead: None,
        };
        // Above its cap after a burst; at or below it with no burst this
        // tick, as sub 8 lowers the word outside its window or sub 0xd
        // writes its 1.0 (@0x34a84).
        for (now, burst) in [(6.1, true), (5.0, false), (1.0, false)] {
            let mut draws = Draws::of(&[]);
            let going = settled(now, burst, own, Inertia::Inertial, &mut draws);
            assert_eq!(going.owed_burst, Some(now), "{now} {burst}");
            assert_eq!(going.goal, Goal::Attack(ShipRef::Player));
            assert_eq!(going.mode, Mode::Flying);
            assert!(going.travel_set());
            assert!(draws.asked.is_empty());
        }
        // Nothing owed now: no burst to end.
        let mut npc = owing(None);
        npc.goal = Goal::Attack(ShipRef::Player);
        settle(&mut npc, true, own, Inertia::Inertial, &mut Draws::of(&[]));
        assert_eq!(npc.goal, Goal::Attack(ShipRef::Player));
    }

    #[test]
    fn a_burst_on_a_word_written_this_tick_ends_at_its_cap_whatever_it_owed_before() {
        // Sub 8 writes +0x34 = 0 less its pull (@0x87ef8, @0x880c0) over
        // nothing owed, and `_HandleShip`'s end test runs inside the burst
        // block (@0x34d10-0x34d2a).
        let led = TopSpeeds {
            own: 6.0,
            lead: Some(6.0),
        };
        let mut npc = owing(None);
        npc.goal = Goal::Dock(ShipRef::Player);
        let flown = handling(&npc);
        npc.owed_burst = Some(1.8);
        let controls = Controls {
            thrust: true,
            ..Controls::default()
        };
        let keeps = Word::Keeps { thrust: 0.18 };
        step(
            &mut npc,
            &flown,
            controls,
            FlightRules::default(),
            (Inertia::Inertial, keeps, flown.accel),
        );
        assert!(npc.display.drive.burst, "it burst");
        let burst = npc.display.drive.burst;
        settle(&mut npc, burst, led, Inertia::Inertial, &mut Draws::of(&[]));
        assert_eq!((npc.goal, npc.owed_burst), (Goal::Idle, None));
        assert_eq!(npc.mode, Mode::Flying, "with a lead, no wait");
    }

    #[test]
    fn only_a_burst_marks_that_it_burst() {
        let thrust = Controls {
            thrust: true,
            ..Controls::default()
        };
        for (owed, word) in [
            (None, Word::Keeps { thrust: 0.18 }),
            (Some(30.0), Word::Keeps { thrust: 0.0 }),
            (Some(30.0), Word::Writes),
        ] {
            let mut npc = owing(owed);
            let flown = handling(&npc);
            step(
                &mut npc,
                &flown,
                thrust,
                FlightRules::default(),
                (Inertia::Inertial, word, flown.accel),
            );
            assert!(!npc.display.drive.burst, "{owed:?} {word:?}");
        }
    }

    #[test]
    fn the_end_of_a_burst_zeroes_its_stage_and_drops_its_jump_with_its_sub() {
        // +0x50 = 0 in every case (@0x82eb0); sub 0 where goal 0 is written
        // (@0x82e9f).
        let own = TopSpeeds {
            own: 6.0,
            lead: None,
        };
        for (goal, sub) in [
            (Goal::Idle, None),
            (Goal::Assist(Help::Refuel), Some(WarpGoal::Own)),
        ] {
            for stage in [-1.0, 5.0] {
                let mut npc = owing(Some(6.0));
                npc.goal = goal;
                npc.warp.stage = stage;
                npc.warp.goal = Some(WarpGoal::Own);
                settle(&mut npc, true, own, Inertia::Inertial, &mut Draws::of(&[0]));
                assert_eq!(npc.warp.stage, 0.0, "{goal:?} {stage}");
                assert_eq!(npc.warp.goal, sub, "{goal:?} {stage}");
            }
        }
    }

    #[test]
    fn an_assisting_ship_keeps_its_goal_and_docking_as_its_burst_ends() {
        // `_AIMakeShipStopJumping` skips goals 9 and 0xf (@0x82e96).
        let mut npc = owing(Some(5.0));
        npc.goal = Goal::Assist(Help::Refuel);
        let docking = Some(Docking {
            ship: ShipRef::Player,
            sub: DockSub::Stop,
        });
        npc.docking = docking;
        npc.travel = Some(crate::catalog::StellarId(150));
        let own = TopSpeeds {
            own: 6.0,
            lead: None,
        };
        settle(&mut npc, true, own, Inertia::Inertial, &mut Draws::of(&[0]));
        assert_eq!(
            (npc.goal, npc.docking),
            (Goal::Assist(Help::Refuel), docking)
        );
        assert!(!npc.travel_set());
        assert_eq!(npc.owed_burst, None);
        assert_eq!(npc.mode, Mode::Coasting { ticks_left: 30 });
    }

    #[test]
    fn an_inertialess_burst_pulls_its_velocity_by_its_steps_adjust_accel() {
        let mut npc = owing(Some(30.0));
        // Off heading × speed, so the pull shows.
        npc.state.velocity = Vec2::new(0.0, 1.0);
        let flown = handling(&npc);
        let mut drifted = npc.clone();
        let pulled = Handling {
            accel: 0.01,
            ..flown
        };
        crate::traffic::inertialess::drift(&mut drifted, &pulled);
        step(
            &mut npc,
            &flown,
            Controls::default(),
            FlightRules::default(),
            (INERTIALESS, Word::Keeps { thrust: 0.5 }, 0.01),
        );
        assert!(close(npc.state.velocity, drifted.state.velocity));
        assert!(
            !close(npc.state.velocity, Vec2::new(2.0, 0.0)),
            "pulled a little"
        );
    }
}
