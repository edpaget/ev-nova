//! The thrust word an NPC's low level leaves `_HandleShip`: the
//! `_ShipAccelRate` it reads at the ship's slot of `_AIDispatch`'s loop
//! ([`Npc::slot_accel`]), which the autopilot thrusts by on both of double
//! time's passes (in the `EV Nova` executable).
//!
//! **The word.** `_HandleShip`'s thrust (@0x34a54-0x34d93) reads no
//! `_ShipAccelRate`. It thrusts by +0x30, capped by +0x34, which the
//! ship's `_LowLevelAIHandler` wrote at its slot: the head zeroes +0x30
//! (@0x851f1) and writes +0x34 from `_ShipMaxSpeed` at the slot when it
//! isn't negative (@0x85200-0x8520d), then each sub that thrusts writes
//! +0x30, each behind its window (the heading's difference against
//! `_ShipTurnRate` at the slot plus a margin):
//!
//! - subs 0x12 (@0x8533d, window @0x852ff), 1 (@0x853fe, @0x8551e,
//!   @0x85552), 0xe (@0x87252), 0x13 (@0x881c4) and 0x15 (@0x88583):
//!   +0x30 = `_ShipAccelRate`, keeping the head's +0x34;
//! - sub 2 within 500 of its stellar on both axes (`_FloatAbs` against
//!   500.0 @0xdd080, @0x85735-0x85773): +0x30 = `_ShipAccelRate`
//!   (@0x85778-0x8577d), +0x34 = 0.25 (@0xdd684) × `_ShipMaxSpeed` at the
//!   slot (@0x85783-0x8579e); farther, +0x34 = 0 (@0x857b3) and +0x30 =
//!   `_ShipAccelRate` (@0x857ab-0x857ba), both behind the window
//!   `_ShipTurnRate` + 5 (@0x856bc-0x856ea);
//! - sub 3: +0x34 = 0 (%esi, zeroed @0x857d9 and kept across the calls)
//!   and +0x30 = `_ShipAccelRate` (@0x8584d-0x85858), window @0x85817;
//! - sub 5 (@0x85d52-0x85d59), sub 7 (@0x86957-0x8695e) and sub 8
//!   (@0x87ef8-0x87eff): +0x34 = 0, +0x30 = `_ShipAccelRate`;
//! - sub 6: +0x34 = 0 (@0x85f7b), +0x30 = `_ShipAccelRate` (@0x85f82),
//!   then its speed match writes the matched speed to +0x34 (@0x860a6);
//! - sub 0x10: +0x34 = 0 (@0x8645a), +0x30 = 1.5 (@0xdd078) ×
//!   `_ShipAccelRate` (@0x86440-0x86474);
//! - sub 0x11: +0x30 = 2.75 (@0xdd96c) × `_ShipAccelRate`
//!   (@0x86594-0x865b2), +0x34 = 1.8 (@0xdd6e0) × `_ShipMaxSpeed` at the
//!   slot (@0x865ba-0x865e3);
//! - sub 0xc's counter-thrust: +0x34 = 0 (%edi, zeroed @0x879c1) and +0x30
//!   = 0.66 (@0xdd5b8) × `_ShipAccelRate` (@0x87a76-0x87a98), window
//!   @0x87a40-0x87a6e;
//! - sub 0xe's inertialess brake: +0x30 = −`_ShipAccelRate`
//!   (@0x87121-0x87137);
//! - sub 4: +0x30 = 0, +0x34 = −1.0 (@0x85c4c-0x85c53); sub 0xf's
//!   inertialess match: +0x30 = +0x34 = 0 (@0x86ac0-0x86acd); sub 0x14
//!   writes no +0x30 but reads `_ShipAccelRate` for its ease (@0x881db);
//! - the escape pod, in `_AIDispatch` itself (@0x8fb62-0x8fbba): +0x30 =
//!   `_ShipAccelRate` (@0x8fb6d-0x8fb72), +0x34 = `_ShipMaxSpeed` at the
//!   slot (@0x8fb78, @0x8fb90), with no low level after it.
//!
//! So every thrust the autopilot flies reads `_ShipAccelRate` at the slot:
//! its whole accel, a sub's multiple of it, and the decisions that read it
//! (stopping distances, the formation's nudge, sub 0x14's ease).
//!
//! **The read** ([`Npc::read_at_slot`]): [`Traffic::leave_in_dispatch`]
//! takes it for each ship the AI loop reaches, at its slot, before its low
//! level's other work: its own accel slowed by its charge as it stands
//! there ([`Npc::flying_handling`]). `_ShipAccelRate` reads the class's
//! accel, the immobile flag, the government, the hold (@0x3643), the
//! person (@0x3674) and the charge (+0x5c @0x3686,
//! `_ShipIonizationFactor` @0x3698). Between a ship's slot (`_AIDispatch`
//! @0x4429d) and its step (`_HandleShip` @0x44483) nothing writes an
//! existing ship's hold, person or parent, but the charge moves:
//! `_HandleShot`'s `_IonizeShip` (@0x3612d) runs in the shot loop
//! (@0x44385) between the two, and the step's own ion block
//! (@0x342b2-0x34421) sheds it before the thrust. So the read is taken at
//! the slot, not as the fly half begins.
//!
//! **Who reads.** The AI loop (@0x44270-0x442ae) dispatches every active
//! ship in the system whose AI type is above 0 (+0x88 @0x4427e), and
//! `_AIDispatch` tail-jumps to the low level (@0x901f4) but for the pod
//! (which writes its own) and while the frozen flag *0x3b523c is set
//! (@0x8fbbf-0x8fbc7, never in play). The engine's loop reaches the ships
//! its runs pick ([`Traffic::think_run`]): on a pass that set a system
//! up, only a fighter launched in it (`nova_sim::Session::dispatch_npcs`'s
//! `thought`), and never a ship a test holds still. A ship the loop
//! doesn't reach keeps its last read, as the original's +0x30 stands until
//! the next low level. No pass clears it: double time's second pass runs
//! no `_AIDispatch` (`je 0x44350` @0x4402a), so it thrusts by the advance
//! pass's read.
//!
//! **A spawn** writes +0x30 = +0x34 = 0 (`_GenericRandomShipSpawn`
//! @0x3cb54-0x3cb5a, %ebx zeroed @0x3c91e) and no +0x68, and so does every
//! launch through it (`_LaunchFighter` @0x3d657, an escape ship's
//! @0x354ea). [`Traffic::add_npc_at`] clears the read. A ship flown with
//! no read since its spawn reads its accel at its step instead; in the
//! original it thrusts nothing until its first low level, which is left
//! out (see
//! rdm:phase/combat-ai-fidelity-cross-roadmap/phase-19-autopilot-low-level-at-the-slot).
//!
//! **The step's own reads.** The turn is `_ShipTurnRate` at the step
//! (@0x344b0), after its ion block. The cap of a sub that leaves +0x34 at
//! 0 is `_ShipMaxSpeed` at the step (@0x34a8a-0x34aab), the engine's
//! [`TopSpeeds::own`](super::autopilot::TopSpeeds::own) as each half
//! begins, which the advance pass's slot read equals and double time's
//! second pass reads after the advance pass's release, lapse and beams.
//! The glow's rush test (@0x34f57) compares +0x30 with `_ShipAccelRate`
//! at the step ([`ship_display::record`](super::ship_display::record)).
//! Left out, all to the same follow-up: the second pass's flight of the
//! advance pass's heading and word, and its cap by the slot's +0x34; the
//! windows' own `_ShipTurnRate` reads at the slot; a ship flown before its
//! first low level; and the inertialess adjustment's own read at the step
//! (`_AdjustInertialessShipVelocity` @0x334e1).
//!
//! None of this plays a sound.

#[cfg(doc)]
use super::Traffic;
use super::World;
use super::npc::Npc;

impl Npc {
    /// Its low level's read of its `_ShipAccelRate` at its slot of the AI
    /// loop in `world`, kept as [`Npc::slot_accel`]: its accel as it flies
    /// now ([`Npc::flying_handling`]), slowed by its charge as it stands
    /// (see the module docs).
    pub fn read_at_slot(&mut self, world: &World) {
        let top = self.top_speed(world.led_speed);
        self.slot_accel = Some(self.flying_handling(top, world.flight).accel);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::collections::BTreeSet;

    use crate::ai::fixture::{PIRATES, TRADERS, ship};
    use crate::ai::{Behaviour, Goal, Surroundings};
    use crate::chance::{Chance, NeverFires};
    use crate::combat::ShipRef;
    use crate::combat::impact::TractorLock;
    use crate::flight::{FlightRules, ShipState};
    use crate::geometry::Vec2;
    use crate::handling::{Handling, ShipFields};
    use crate::stats::ShipStats;
    use crate::traffic::npc::{AiType, Mode, Npc, NpcId};
    use crate::traffic::ship_display::GLOW_CRUISE;
    use crate::traffic::{Traffic, World};
    use nova_sim_base::fixture::FAST;

    /// Every ship keeps its goal.
    #[derive(Debug)]
    struct Keeps;

    impl Behaviour for Keeps {
        fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            npc.goal
        }
    }

    fn of(traffic: &Traffic, id: NpcId) -> &Npc {
        (traffic.npcs().iter())
            .find(|npc| npc.id == id)
            .expect("there")
    }

    fn of_mut(traffic: &mut Traffic, id: NpcId) -> &mut Npc {
        (traffic.npcs_mut().iter_mut())
            .find(|npc| npc.id == id)
            .expect("there")
    }

    /// A trader 2000 up the screen in slot 5, and an attacker of `fields`
    /// at the centre in slot 3, at rest facing it (sub 7, which thrusts
    /// leaving +0x34 at 0), of `IonizeMax` 400 and `Deionize` 100 a tick,
    /// charged `charge`: the attacker.
    fn attacking_of(traffic: &mut Traffic, fields: ShipFields, charge: f32) -> NpcId {
        let target = traffic.add_npc_at(ship(0, TRADERS, AiType::WimpyTrader, 0.0, -2000.0), 5);
        let mut attacker = ship(0, PIRATES, AiType::Warship, 0.0, 0.0);
        attacker.stats = ShipStats::new(fields, &[]);
        attacker.goal = Goal::Attack(ShipRef::Npc(target));
        attacker.target = Some(ShipRef::Npc(target));
        attacker.stats.ion.class_capacity = 400.0;
        attacker.stats.ion.class_rate = 100.0;
        attacker.ion.charge = charge;
        traffic.add_npc_at(attacker, 3)
    }

    /// [`attacking_of`] `FAST`.
    fn attacking(traffic: &mut Traffic, charge: f32) -> NpcId {
        attacking_of(traffic, FAST, charge)
    }

    /// `npc` as it flies charged `charge`, unheld.
    fn charged(npc: &Npc, charge: f32) -> Handling {
        let mut ship = npc.clone();
        ship.ion.charge = charge;
        ship.tractor = None;
        ship.flying_handling(ship.stats.handling.max_speed, FlightRules::default())
    }

    fn speed(traffic: &Traffic, id: NpcId) -> f32 {
        -of(traffic, id).state.velocity.y
    }

    #[test]
    fn an_ionised_npcs_thrust_is_the_accel_its_low_level_read_at_its_slot_before_the_shed() {
        // Sub 7 writes +0x30 = `_ShipAccelRate` at its slot (@0x8695e),
        // before `_HandleShip`'s ion block sheds 300 to 200
        // (@0x342b2-0x34421).
        let mut traffic = Traffic::with_interval(1);
        let attacker = attacking(&mut traffic, 300.0);
        let slot = charged(of(&traffic, attacker), 300.0).accel;
        assert_ne!(slot, charged(of(&traffic, attacker), 200.0).accel);
        traffic.tick_in(&Keeps, World::new(&[]), &[], &mut NeverFires);
        assert_eq!(of(&traffic, attacker).ion.charge, 200.0);
        assert_eq!(speed(&traffic, attacker), slot);
    }

    #[test]
    fn on_double_times_second_pass_it_thrusts_by_the_advance_passs_slot_accel() {
        // The second pass runs no `_AIDispatch` (@0x4402a), so `_HandleShip`
        // thrusts by the advance pass's +0x30 after its own shed to 100.
        let mut traffic = Traffic::with_interval(1);
        let attacker = attacking(&mut traffic, 300.0);
        let slot = charged(of(&traffic, attacker), 300.0).accel;
        traffic.tick_in(&Keeps, World::new(&[]), &[], &mut NeverFires);
        traffic.move_in(World::new(&[]), &mut NeverFires);
        assert_eq!(of(&traffic, attacker).ion.charge, 100.0);
        assert_eq!(speed(&traffic, attacker), slot + slot);
    }

    #[test]
    fn an_ion_hit_between_its_slot_and_its_step_doesnt_slow_its_thrust() {
        // `_HandleShot`'s `_IonizeShip` (@0x3612d) runs in the shot loop
        // (@0x44385), between the AI loop and the ship loop.
        let mut traffic = Traffic::with_interval(1);
        let attacker = attacking(&mut traffic, 0.0);
        let slot = charged(of(&traffic, attacker), 0.0).accel;
        traffic.think_holding(&Keeps, World::new(&[]), &[], &mut NeverFires);
        of_mut(&mut traffic, attacker).ion.charge = 400.0;
        traffic.fly_holding(World::new(&[]), &BTreeSet::new(), &mut NeverFires);
        assert_eq!(of(&traffic, attacker).ion.charge, 300.0);
        assert_eq!(speed(&traffic, attacker), slot);
    }

    #[test]
    fn each_advance_pass_reads_it_afresh() {
        // The second tick's slot reads the charge the first tick's step
        // left: 200.
        let mut traffic = Traffic::with_interval(1);
        let attacker = attacking(&mut traffic, 300.0);
        let first = charged(of(&traffic, attacker), 300.0).accel;
        let second = charged(of(&traffic, attacker), 200.0).accel;
        traffic.tick_in(&Keeps, World::new(&[]), &[], &mut NeverFires);
        traffic.tick_in(&Keeps, World::new(&[]), &[], &mut NeverFires);
        assert_eq!(of(&traffic, attacker).slot_accel, Some(second));
        assert_eq!(speed(&traffic, attacker), first + second);
    }

    #[test]
    fn a_spawn_starts_with_no_slot_read() {
        // Every spawn zeroes +0x30 (`_GenericRandomShipSpawn` @0x3cb54).
        let mut traffic = Traffic::with_interval(1);
        let mut read = ship(0, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
        read.slot_accel = Some(1.5);
        let id = traffic.add_npc_at(read, 4);
        assert_eq!(of(&traffic, id).slot_accel, None);
    }

    #[test]
    fn with_no_slot_read_it_thrusts_by_its_accel_at_its_step() {
        // A pass with no AI loop before it, its read none since its spawn:
        // it reads its accel at its step, after the shed to 200.
        let mut traffic = Traffic::with_interval(1);
        let attacker = attacking(&mut traffic, 300.0);
        let step = charged(of(&traffic, attacker), 200.0).accel;
        traffic.move_in(World::new(&[]), &mut NeverFires);
        assert_eq!(of(&traffic, attacker).slot_accel, None);
        assert_eq!(speed(&traffic, attacker), step);
    }

    /// `FAST` of accel `accel` and top speed `speed`.
    fn fields(accel: i16, speed: i16) -> ShipFields {
        ShipFields {
            accel,
            speed,
            ..FAST
        }
    }

    /// The speed of an attacker of accel 900 and top speed 90, uncharged,
    /// moving `speed` toward its target, before and after double time's
    /// second pass, held from the advance pass's end (`_HandleBeams`
    /// @0x4453d) by the player moving at its velocity, so the pull does
    /// nothing; with its unheld and held handling.
    fn second_pass_held(speed_at: f32) -> ([f32; 2], Handling, Handling) {
        let mut traffic = Traffic::with_interval(1);
        let attacker = attacking_of(&mut traffic, fields(900, 90), 0.0);
        of_mut(&mut traffic, attacker).state.velocity = Vec2::new(0.0, -speed_at);
        let unheld = charged(of(&traffic, attacker), 0.0);
        traffic.tick_in(&Keeps, World::new(&[]), &[], &mut NeverFires);
        let before = speed(&traffic, attacker);
        let held = {
            let npc = of_mut(&mut traffic, attacker);
            npc.tractor = Some(TractorLock::by(ShipRef::Player));
            npc.own_handling()
        };
        let mut player = crate::ai::fixture::player(0.0, 500.0);
        player.state = ShipState {
            velocity: of(&traffic, attacker).state.velocity,
            ..player.state
        };
        let world = World {
            player: Some(player),
            ..World::new(&[])
        };
        traffic.move_in(world, &mut NeverFires);
        assert!(of(&traffic, attacker).tractor.is_some(), "still held");
        let after = speed(&traffic, attacker);
        ([before, after], unheld, held)
    }

    #[test]
    fn on_the_second_pass_a_thrust_that_leaves_0x34_at_0_caps_at_its_step_top_speed() {
        // Sub 7 leaves +0x34 at 0 (@0x86957), so the second pass's step
        // caps its thrust at its held `_ShipMaxSpeed` (@0x34aab), a third,
        // while the thrust is the advance slot's unheld +0x30. Short of the
        // held cap it gains the unheld accel, not the held third.
        let ([before, after], unheld, held) = second_pass_held(0.0);
        assert_eq!(before, unheld.accel);
        assert!(before < held.max_speed && held.accel < unheld.accel);
        assert_eq!(after, before + unheld.accel);
        // Past the held cap and short of the unheld one, it gains nothing.
        let ([before, after], unheld, held) = second_pass_held(0.2);
        assert!(held.max_speed < before && before < unheld.max_speed);
        assert_eq!(after, before);
    }

    #[test]
    fn an_escape_pods_thrust_is_its_slot_accel() {
        // `_AIDispatch` writes a pod's +0x30 = `_ShipAccelRate`
        // (@0x8fb6d-0x8fb72) at its slot.
        let mut traffic = Traffic::with_interval(1);
        let mut pod = ship(0, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
        pod.ship = crate::combat::hull::ESCAPE_POD_SHIP;
        pod.mode = Mode::Adrift { ticks_left: 10 };
        pod.stats.ion.class_capacity = 400.0;
        pod.stats.ion.class_rate = 100.0;
        pod.ion.charge = 300.0;
        let pod = traffic.add_npc_at(pod, 2);
        let slot = charged(of(&traffic, pod), 300.0).accel;
        traffic.tick_in(&Keeps, World::new(&[]), &[], &mut NeverFires);
        assert_eq!(of(&traffic, pod).ion.charge, 200.0);
        assert_eq!(speed(&traffic, pod), slot);
    }

    #[test]
    fn the_glow_reads_a_rush_by_its_slot_thrust_against_its_step_accel() {
        // The rush test (@0x34f57) compares +0x30, read uncharged at the
        // slot, with `_ShipAccelRate` at the step, after an ion hit to 400
        // and the shed to 300: 0.3 of it, so the plain thrust is a rush.
        // A rush raises a glow of 24 by 1 (@0x34f85), where a plain thrust
        // leaves it (@0x34fa1).
        let mut traffic = Traffic::with_interval(1);
        let attacker = attacking(&mut traffic, 0.0);
        traffic.think_holding(&Keeps, World::new(&[]), &[], &mut NeverFires);
        let npc = of_mut(&mut traffic, attacker);
        npc.ion.charge = 400.0;
        npc.display.glow = GLOW_CRUISE;
        traffic.fly_holding(World::new(&[]), &BTreeSet::new(), &mut NeverFires);
        assert_eq!(of(&traffic, attacker).display.glow, GLOW_CRUISE + 1);
        let mut uncharged = Traffic::with_interval(1);
        let attacker = attacking(&mut uncharged, 0.0);
        uncharged.think_holding(&Keeps, World::new(&[]), &[], &mut NeverFires);
        of_mut(&mut uncharged, attacker).display.glow = GLOW_CRUISE;
        uncharged.fly_holding(World::new(&[]), &BTreeSet::new(), &mut NeverFires);
        assert_eq!(of(&uncharged, attacker).display.glow, GLOW_CRUISE, "plain");
    }

    #[test]
    fn the_read_is_its_accel_as_it_flies_at_the_slot() {
        let mut npc = ship(0, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
        npc.stats.ion.class_capacity = 400.0;
        npc.ion.charge = 200.0;
        npc.read_at_slot(&World::new(&[]));
        assert_eq!(npc.slot_accel, Some(charged(&npc, 200.0).accel));
        assert!(npc.slot_accel < Some(charged(&npc, 0.0).accel));
    }
}
