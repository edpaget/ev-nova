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
//! - **What drops it** (F8): every substate's thrust but three writes
//!   +0x34 with +0x30 (subs 2, 3, 5, 6, 7, 8, 9, 0xb, 0xc's
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
//! - **The end** (F2, F3, [`settle`]): once +0x34 is at or above minus
//!   the cap ([`TopSpeeds::burst_cap`], @0x34c68-0x34d2a),
//!   `_AIMakeShipStopJumping` (@0x82e77-0x82eba) sets goal and sub-goal 0
//!   unless the goal is 9 or 0xf (@0x82e7d-0x82e9f), its travel stellar
//!   +0x6c to -1 and +0x30 = +0x34 = 0, keeping its target +0x70; an
//!   inertialess ship's velocity is zeroed (@0x34d37-0x34d59); and a ship
//!   with no leader waits Rand(30) + 30 (@0x34d5c-0x34d8b), as after a
//!   gate's own burst ([`autopilot::after_burst`]). Its glow falls on the
//!   end frame, as +0x30 is zeroed before the glow reads it (F4,
//!   @0x34eec-0x34fc4).
//!
//! Neither the burst nor `_AIMakeShipStopJumping` plays a sound. The
//! fire on the burst's last frame (F4), sub 0xd's -1.0, the inspection
//! hold's `_Accel` and a ship disabled during its burst are
//! rdm:phase/escort-fleet-fidelity-followups/phase-13-left-out-of-held-ship-burst's;
//! the mining orders' thrusts are
//! rdm:phase/escort-fleet-fidelity-cross-roadmap/phase-2-mining-orders-burst-an-owed-burst's.
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
/// reverse ([`flight::inertialess_step`]). An inertial reverse turns
/// alone.
pub(crate) fn writes_thrust(controls: Controls, inertia: Inertia) -> bool {
    controls.thrust || (matches!(inertia, Inertia::Inertialess { .. }) && controls.reverse)
}

/// What a flight flies this tick: its controls, and its [`Word`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Flight {
    /// The controls.
    pub controls: Controls,
    /// What it does to +0x34.
    pub word: Word,
}

impl Flight {
    /// Sub 1's flight under `controls`, flown on `flown` as `inertia` says
    /// ([`Word::stop`]).
    pub(crate) fn stop(controls: Controls, flown: &Handling, inertia: Inertia) -> Self {
        Self {
            controls,
            word: Word::stop(controls, flown, inertia),
        }
    }
}

/// A flight that writes +0x34 as it thrusts ([`Word::Writes`]).
impl From<Controls> for Flight {
    fn from(controls: Controls) -> Self {
        Self {
            controls,
            word: Word::Writes,
        }
    }
}

/// Flies `npc` one tick under `controls` on `handling` under `flight`, as
/// `inertia` says, its substate doing `word` to +0x34 (see the module
/// docs): owing a burst under a [`Word::Keeps`] that thrusts, it bursts
/// ([`burst`]); under one that doesn't, it flies keeping what it owes
/// ([`autopilot::plain_step`]); under [`Word::Writes`] it flies as ever,
/// its thrust ([`writes_thrust`]) dropping what it owes (F8).
pub(crate) fn step(
    npc: &mut Npc,
    handling: &Handling,
    controls: Controls,
    flight: FlightRules,
    inertia: Inertia,
    word: Word,
) {
    match (word, npc.owed_burst) {
        (Word::Keeps { thrust }, Some(speed)) if thrust > 0.0 => {
            burst(npc, handling, controls, flight, inertia, (speed, thrust));
        }
        (Word::Keeps { .. }, _) => autopilot::plain_step(npc, handling, controls, flight, inertia),
        (Word::Writes, _) => {
            if writes_thrust(controls, inertia) {
                npc.owed_burst = None;
            }
            autopilot::plain_step(npc, handling, controls, flight, inertia);
        }
    }
}

/// One tick of `npc`'s burst at `speed`, its thrust word `thrust` (F2):
/// it records its drive and turns as [`autopilot::plain_step`] would under
/// `controls` on `handling` under `flight`, as `inertia` says, then moves
/// along its new heading at `speed` in place of its thrust
/// ([`autopilot::burst_motion`]), and owes `thrust` less.
fn burst(
    npc: &mut Npc,
    handling: &Handling,
    controls: Controls,
    flight: FlightRules,
    inertia: Inertia,
    (speed, thrust): (f32, f32),
) {
    ship_display::record(npc, handling, controls, flight, inertia);
    npc.state.heading = match inertia {
        Inertia::Inertial => flight::npc_turn(&npc.state, handling, controls, flight),
        Inertia::Inertialess { .. } => {
            flight::turned(npc.state.heading, controls.turn, handling.turn_rate)
        }
    };
    autopilot::burst_motion(npc, speed, handling, inertia);
    npc.owed_burst = Some(speed - thrust);
}

/// Ends `npc`'s burst after its flight this tick, `owed` what it owed
/// before the flight (F2): once a burst has lowered what it owes to at or
/// below its cap at `speeds` ([`TopSpeeds::burst_cap`]), it stops jumping
/// ([`stop_jumping`]), its glow falls (F4), and, as `inertia` says, it
/// ends as a gate's own burst does ([`autopilot::after_burst`]: an
/// inertialess ship's velocity zeroed, and a ship with no lead waiting
/// its Rand(30) + 30, rolled on `chance`).
pub(crate) fn settle(
    npc: &mut Npc,
    owed: Option<f32>,
    speeds: TopSpeeds,
    inertia: Inertia,
    chance: &mut (impl Chance + ?Sized),
) {
    let (Some(before), Some(now)) = (owed, npc.owed_burst) else {
        return;
    };
    if now >= before || now > speeds.burst_cap() {
        return;
    }
    stop_jumping(npc);
    npc.display.drive.thrust = Thrust::None;
    autopilot::after_burst(npc, speeds, inertia, chance);
}

/// `_AIMakeShipStopJumping` (@0x82e77-0x82eba, F3): goal and sub-goal 0
/// ([`Goal::Idle`], its kept pursuit and docking dropped) unless it
/// assists the player (goal 9 or 0xf, @0x82e96); its travel stellar
/// cleared (+0x6c = -1) and its burst gone (+0x34 = 0). Its target is
/// kept.
fn stop_jumping(npc: &mut Npc) {
    if !matches!(npc.goal, Goal::Assist(_)) {
        npc.goal = Goal::Idle;
        npc.pursuit.kept = None;
        npc.docking = None;
    }
    npc.travel_set = false;
    npc.owed_burst = None;
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::pursuit::Kept;
    use crate::combat::ShipRef;
    use crate::flight::{ShipState, Turn, facing};
    use crate::geometry::Vec2;
    use crate::hail::Help;
    use crate::stats::ShipStats;
    use crate::traffic::npc::Mode;
    use crate::traffic::plunder_dock::{DockSub, Docking};
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
            Inertia::Inertial,
            Word::Keeps { thrust: 0.25 },
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
        // Reverse with no turn key turns it toward its retro heading, as
        // the inertial step does.
        let mut braking = owing(Some(30.0));
        braking.state.heading = 0.0;
        let controls = Controls {
            reverse: true,
            ..Controls::default()
        };
        step(
            &mut braking,
            &flown,
            controls,
            FlightRules::default(),
            Inertia::Inertial,
            Word::Keeps { thrust: 0.25 },
        );
        assert_eq!(braking.state.heading, 360.0 - flown.turn_rate);
        assert!(close(
            braking.state.velocity,
            facing(braking.state.heading) * 30.0
        ));
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
            INERTIALESS,
            Word::Keeps { thrust: 0.5 },
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
                inertia,
            );
            step(
                &mut npc,
                &flown,
                controls,
                FlightRules::default(),
                inertia,
                Word::Keeps { thrust: 0.0 },
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
            Inertia::Inertial,
        );
        step(
            &mut npc,
            &flown,
            controls,
            FlightRules::default(),
            Inertia::Inertial,
            Word::Keeps { thrust: 0.3 },
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
                inertia,
            );
            step(
                &mut npc,
                &flown,
                controls,
                FlightRules::default(),
                inertia,
                Word::Writes,
            );
            assert_eq!(npc.owed_burst, None, "{inertia:?} {controls:?}");
            assert_eq!(npc.state, plain.state, "{inertia:?} {controls:?}");
        }
        // No thrust writes nothing: an inertial reverse turns alone.
        for (inertia, controls) in [
            (Inertia::Inertial, reverse),
            (INERTIALESS, Controls::default()),
        ] {
            let mut npc = owing(Some(30.0));
            let flown = handling(&npc);
            step(
                &mut npc,
                &flown,
                controls,
                FlightRules::default(),
                inertia,
                Word::Writes,
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
            Flight::from(thrust),
            Flight {
                controls: thrust,
                word: Word::Writes
            }
        );
        assert_eq!(
            Flight::stop(thrust, &soft, Inertia::Inertial),
            Flight {
                controls: thrust,
                word: keeps(0.2)
            }
        );
    }

    /// A ship that owed `before` and now owes `now`, attacking the player
    /// with a pursuit kept and a docking, its travel stellar set and its
    /// drive thrusting.
    fn settled(
        before: f32,
        now: f32,
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
        npc.travel_set = true;
        npc.display.drive.thrust = Thrust::Plain;
        settle(&mut npc, Some(before), speeds, inertia, draws);
        npc
    }

    #[test]
    fn a_burst_lowered_to_its_cap_stops_jumping_and_waits_30_to_59_with_no_lead() {
        let own = TopSpeeds {
            own: 6.0,
            lead: None,
        };
        let mut draws = Draws::of(&[7]);
        let ended = settled(6.2, 6.0, own, Inertia::Inertial, &mut draws);
        assert_eq!(ended.goal, Goal::Idle, "goal 0");
        assert_eq!(
            (ended.pursuit.kept, ended.docking),
            (None, None),
            "sub-goal 0"
        );
        assert_eq!(ended.target, Some(ShipRef::Player), "+0x70 kept");
        assert!(!ended.travel_set, "+0x6c = -1");
        assert_eq!(ended.owed_burst, None);
        assert_eq!(ended.display.drive.thrust, Thrust::None, "+0x30 = 0");
        assert_eq!(ended.mode, Mode::Coasting { ticks_left: 37 });
        assert_eq!(draws.asked, [30], "Rand(30)");
        assert!(close(ended.state.velocity, Vec2::new(2.0, 0.0)), "kept");
        // An inertialess ship's velocity is zeroed.
        let ended = settled(6.2, 6.0, own, INERTIALESS, &mut Draws::of(&[0]));
        assert_eq!(ended.state.velocity, Vec2::ZERO);
        // With a lead it flies on, its cap the lower top speed.
        let led = TopSpeeds {
            own: 9.0,
            lead: Some(6.0),
        };
        let mut draws = Draws::of(&[]);
        let ended = settled(6.2, 5.9, led, Inertia::Inertial, &mut draws);
        assert_eq!((ended.mode, ended.owed_burst), (Mode::Flying, None));
        assert_eq!(draws.asked, Vec::<u32>::new());
    }

    #[test]
    fn a_burst_above_its_cap_or_not_lowered_this_tick_goes_on() {
        let own = TopSpeeds {
            own: 6.0,
            lead: None,
        };
        for (before, now) in [(6.3, 6.1), (5.0, 5.0)] {
            let mut draws = Draws::of(&[]);
            let going = settled(before, now, own, Inertia::Inertial, &mut draws);
            assert_eq!(going.owed_burst, Some(now), "{before} {now}");
            assert_eq!(going.goal, Goal::Attack(ShipRef::Player));
            assert_eq!(going.mode, Mode::Flying);
            assert!(going.travel_set);
            assert!(draws.asked.is_empty());
        }
        // Nothing owed before, or nothing now: no burst ran.
        let mut npc = owing(Some(3.0));
        settle(&mut npc, None, own, Inertia::Inertial, &mut Draws::of(&[]));
        assert_eq!(npc.owed_burst, Some(3.0));
        let mut npc = owing(None);
        npc.goal = Goal::Attack(ShipRef::Player);
        settle(
            &mut npc,
            Some(9.0),
            own,
            Inertia::Inertial,
            &mut Draws::of(&[]),
        );
        assert_eq!(npc.goal, Goal::Attack(ShipRef::Player));
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
        npc.travel_set = true;
        let own = TopSpeeds {
            own: 6.0,
            lead: None,
        };
        settle(
            &mut npc,
            Some(6.5),
            own,
            Inertia::Inertial,
            &mut Draws::of(&[0]),
        );
        assert_eq!(
            (npc.goal, npc.docking),
            (Goal::Assist(Help::Refuel), docking)
        );
        assert!(!npc.travel_set);
        assert_eq!(npc.owed_burst, None);
        assert_eq!(npc.mode, Mode::Coasting { ticks_left: 30 });
    }
}
