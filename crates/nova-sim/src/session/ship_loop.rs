//! `_DoPlayGameWork`'s ship loop (@0x4445c-0x444c4), as the original runs
//! it: for each slot 1-63 in order, `_HandleShip` (@0x44483) when the slot
//! is active and in the player's system, then `_HandleShipDisplay`
//! (@0x444ad) ([`Session::ship_loop`]).
//!
//! **Each NPC in slot order** flies (its position step @0x34274-0x342aa,
//! after `_AdjustInertialessShipVelocity` @0x3425f; the traffic's
//! [`fly_npc`](crate::traffic::Traffic::fly_npc)), then takes its step:
//! its reloads, regeneration (@0x34424), fire (@0x34ffd) and death timer
//! (@0x35083), then its display, which may make its final explosion
//! ([`Combat::handle_npc`]). So:
//!
//! - **A final explosion's push** (`_DamageShip` @0x2da89) reaches each
//!   ship of a later slot where it still stands, and that ship's own
//!   flight, later in the loop, moves it by the pushed velocity: it is
//!   drawn pushed that frame. A ship of an earlier slot has moved; it
//!   moves by the push on the next pass.
//! - **Each NPC's fire** reads the ships of earlier slots where their own
//!   flight left them, and those of later slots where they stood.
//! - **A ship whose flight ends it** (it lands, jumps, enters a gate or
//!   docks) leaves at once: its slot takes no step, and no later slot's
//!   fire or shockwave sees it.
//!
//! Each flight steers by the other ships where the pass found them, as the
//! original's steering is `_AIDispatch`'s (`_LowLevelAIHandler`), before
//! the loop moves any ship; `_HandleShip` itself reads no other ship's
//! position. Each slot's hits are settled at its step
//! ([`Session::land_phase`]), closer to `_DamageShip` settling each as it
//! lands.
//!
//! [`Combat::handle_npc`]: crate::combat::Combat::handle_npc

use std::collections::BTreeSet;

use super::npc_escape::AtStep;
use super::{Session, TrafficMarks};
use crate::chance::Chance;
use crate::combat::{GamePass, Rules, ShipRef};
use crate::traffic::npc::NpcId;
use crate::traffic::{FlyHalf, FlyPass};

/// How the traffic moves in a pass's ship loop.
pub(super) enum Motion {
    /// The advance pass's fly half, read against the marks its think
    /// left.
    Fly(TrafficMarks),
    /// None: the advance pass populated the system, whose ships move from
    /// the next.
    Populated,
    /// Double time's second pass's fly half.
    Again,
}

impl Session {
    /// `_DoPlayGameWork`'s ship loop on `pass` by `rules`, the traffic
    /// moving as `motion` says, drawing on `chance` (see the module docs):
    /// every NPC's stray provocation eases (@0x35374-0x353b2); then, slot
    /// by slot, each NPC flies and takes its step; then what the fly half
    /// leaves: its end, and the session's work after it; the ships laid
    /// out ([`Session::lay_out_displayed`]); every NPC's cloak fade steps
    /// ([`Traffic::step_fades`](crate::traffic::Traffic::step_fades)); and
    /// the pick fired is cleared or kept
    /// ([`Traffic::after_firing`](crate::traffic::Traffic::after_firing)).
    /// Gives each NPC as its own step found it, in slot order, for its
    /// escape-ship roll (see the `npc_escape` module).
    ///
    /// What the loop's departures leave the session (the pirates answered,
    /// each mission ship that left raising its fate, the persons' offers,
    /// the fighters docked and orphaned, the watched ships observed), the
    /// fly half's end (the docks started, the waits counted down, the
    /// followers released, the fuel), the reinforcement clock, the gates'
    /// sprites and the assistance all settle after the loop, where the
    /// original runs them in each NPC's own `_HandleShip` or `_AIDispatch`
    /// (`_HandleReinforcements` itself comes after the loop, @0x44504).
    /// No step in the loop reads them, and all settle before the frame is
    /// drawn.
    pub(super) fn ship_loop(
        &mut self,
        motion: Motion,
        pass: GamePass,
        rules: Rules,
        chance: &mut dyn Chance,
    ) -> Vec<AtStep> {
        self.traffic.ease_stray();
        let again_marks = matches!(motion, Motion::Again)
            .then(|| self.mission_marks())
            .filter(|_| !self.traffic_due);
        let mut fly = match &motion {
            Motion::Fly(marks) => Some(self.begin_fly(&marks.still, FlyHalf::Advance)),
            Motion::Again if again_marks.is_some() => {
                Some(self.begin_fly(&BTreeSet::new(), FlyHalf::Again))
            }
            Motion::Again | Motion::Populated => None,
        };
        let mut order: Vec<(u32, NpcId)> = self
            .traffic
            .npcs()
            .iter()
            .map(|npc| (npc.berth, npc.id))
            .collect();
        order.sort_unstable();
        let mut shown = vec![ShipRef::Player];
        let mut found = Vec::new();
        for (_, id) in order {
            if let Some(fly) = fly.as_mut() {
                self.step_traffic(|traffic, world| {
                    traffic.fly_npc(fly, id, &world, &mut *chance);
                });
            }
            let Some(npc) = self.npc(id) else {
                continue;
            };
            found.push(AtStep::of(npc));
            self.land_phase(pass, rules, chance, |combat, fighters, arena, chance| {
                combat.handle_npc(fighters, arena, ShipRef::Npc(id), &mut shown, chance);
            });
        }
        if let Some(fly) = fly {
            self.traffic.end_fly(fly);
        }
        match motion {
            Motion::Fly(marks) => {
                self.after_traffic_motion(&marks, &mut &mut *chance);
                self.traffic_tail(&mut &mut *chance);
                self.tick_assistance(rules.disable);
            }
            Motion::Populated => {
                self.traffic_tail(&mut &mut *chance);
                self.tick_assistance(rules.disable);
            }
            Motion::Again => {
                if let Some(marks) = again_marks {
                    self.mission_ships_left(&marks);
                    self.dock_fighters();
                }
            }
        }
        self.lay_out_displayed();
        self.traffic.step_fades();
        self.traffic.after_firing(&self.arsenal);
        found
    }

    /// Begins the traffic's fly half of `half`, the NPCs in `still`
    /// keeping their place, in the world it flies in.
    fn begin_fly(&mut self, still: &BTreeSet<NpcId>, half: FlyHalf) -> FlyPass {
        self.step_traffic(|traffic, world| traffic.begin_fly(&world, still, half))
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use crate::ai::{Behaviour, Goal, Surroundings};
    use crate::catalog::WeaponRecord;
    use crate::chance::{Chance, NeverFires};
    use crate::combat::armament::Trigger;
    use crate::combat::hull::Condition;
    use crate::combat::{CombatEvent, Rules, ShipRef};
    use crate::flight::Controls;
    use crate::geometry::Vec2;
    use crate::session::Session;
    use crate::session::tests::{FIRE, armed, blaster, facing_an_npc};
    use crate::stats::ShipStats;
    use crate::testkit::{FAST, FakePilotCatalog, npc};
    use crate::traffic::autopilot::Outcome;
    use crate::traffic::npc::{Npc, NpcId};

    /// Every NPC keeps its goal and holds no trigger.
    #[derive(Debug)]
    struct Keep;

    impl Behaviour for Keep {
        fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            npc.goal
        }
    }

    /// An advance pass of `session`, its NPCs as `behaviour` says.
    fn pass(session: &mut Session, catalog: &FakePilotCatalog, behaviour: &impl Behaviour) {
        let rescued = session.game_work(
            Controls::default(),
            catalog,
            behaviour,
            Rules::default(),
            &mut NeverFires,
        );
        assert!(!rescued);
    }

    /// Double time's second pass of `session`.
    fn again(session: &mut Session) {
        session.tick_again(
            Controls::default(),
            &Keep,
            Rules::default(),
            &mut NeverFires,
        );
    }

    /// `catalog`'s session in flight, its traffic set up and emptied.
    fn empty(catalog: &FakePilotCatalog) -> Session {
        let mut session = facing_an_npc(catalog, 180);
        session.traffic.remove(NpcId(0));
        session
    }

    /// An idle NPC of mass 40 at berth `berth`, at `at` moving at
    /// `velocity`, in `session`.
    fn place(session: &mut Session, berth: u32, at: Vec2, velocity: Vec2) -> NpcId {
        let mut ship = npc(0, ShipStats::new(FAST, &[]));
        ship.hull.mass = 40.0;
        ship.state.position = at;
        ship.state.velocity = velocity;
        session.traffic.add_npc_at(ship, berth)
    }

    /// A ship of mass 200 at berth `berth` at `at`, at rest, breaking up
    /// with its timer at 2: its own step takes it to 1, and its display
    /// makes its final explosion, a shockwave of 32 over 65 pixels.
    fn wreck(session: &mut Session, berth: u32, at: Vec2) -> NpcId {
        let mut ship = npc(0, ShipStats::new(FAST, &[]));
        ship.hull.mass = 200.0;
        ship.hull.death_delay = 5;
        ship.reserves.armor.now = 0.0;
        ship.condition = Condition::Dying {
            ticks_left: 2,
            starting: false,
        };
        ship.state.position = at;
        session.traffic.add_npc_at(ship, berth)
    }

    fn state(session: &Session, id: NpcId) -> crate::flight::ShipState {
        session.npc(id).expect("there").state
    }

    const WRECKED: Vec2 = Vec2::new(600.0, 0.0);

    #[test]
    fn a_later_slot_is_moved_by_a_wrecks_push_in_the_explosion_pass() {
        // Slot 1's display pushes slot 2 (`_DamageShip` @0x2da89) before
        // slot 2's own `_HandleShip` moves it (@0x34274-0x342aa), on either
        // pass.
        let catalog = armed();
        for second_pass in [false, true] {
            let mut session = empty(&catalog);
            let gone = wreck(&mut session, 1, WRECKED);
            let at = WRECKED + Vec2::new(40.0, 0.0);
            let pushed = place(&mut session, 2, at, Vec2::ZERO);
            if second_pass {
                again(&mut session);
            } else {
                pass(&mut session, &catalog, &Keep);
            }
            assert!(session.npc(gone).is_none(), "{second_pass}: exploded");
            let after = state(&session, pushed);
            assert!(after.velocity.x > 0.0, "{second_pass}: {after:?}");
            assert_eq!(after.position, at + after.velocity, "{second_pass}");
        }
    }

    #[test]
    fn an_earlier_slot_is_pushed_and_moves_by_it_on_the_next_pass() {
        // Slot 1 has moved already when slot 2's display pushes it.
        let catalog = armed();
        let mut session = empty(&catalog);
        let at = WRECKED + Vec2::new(40.0, 0.0);
        let pushed = place(&mut session, 1, at, Vec2::ZERO);
        wreck(&mut session, 2, WRECKED);
        pass(&mut session, &catalog, &Keep);
        let after = state(&session, pushed);
        assert_eq!(after.position, at, "not yet");
        assert!(after.velocity.x > 0.0, "{after:?}");
        pass(&mut session, &catalog, &Keep);
        assert!(state(&session, pushed).position.x > at.x, "the next pass");
    }

    /// Whether NPC `id` was hit by the pass's shockwave.
    fn hit(events: &[CombatEvent], id: NpcId) -> bool {
        events.iter().any(|event| {
            matches!(event, CombatEvent::Hit { ship, weapon: None, .. } if *ship == ShipRef::Npc(id))
        })
    }

    #[test]
    fn a_later_slot_flying_into_the_blast_this_pass_is_not_reached() {
        // 66 pixels out, beyond the 65 the shockwave reaches, flying in at
        // 2 a tick: a later slot still stands outside as the wreck's
        // display strikes (@0x2d8ef-0x2da9c); an earlier one has flown in.
        let catalog = armed();
        for (mover_berth, wreck_berth, reached) in [(2, 1, false), (1, 2, true)] {
            let mut session = empty(&catalog);
            let mover = place(
                &mut session,
                mover_berth,
                WRECKED + Vec2::new(66.0, 0.0),
                Vec2::new(-2.0, 0.0),
            );
            wreck(&mut session, wreck_berth, WRECKED);
            session.take_combat_events();
            pass(&mut session, &catalog, &Keep);
            assert!(
                state(&session, mover).position.x < WRECKED.x + 65.0,
                "flown in"
            );
            let events = session.take_combat_events();
            assert_eq!(hit(&events, mover), reached, "berth {mover_berth}");
        }
    }

    /// Fires its primary at NPC `at`.
    #[derive(Debug)]
    struct FiringAt(NpcId);

    impl Behaviour for FiringAt {
        fn decide(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            Goal::Idle
        }

        fn trigger(&self, npc: &Npc, _around: &Surroundings) -> Trigger {
            if npc.id == self.0 {
                Trigger::default()
            } else {
                FIRE
            }
        }

        fn target(&self, npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
            (npc.id != self.0).then_some(ShipRef::Npc(self.0))
        }
    }

    #[test]
    fn an_npcs_fire_reads_a_later_slot_where_it_stood() {
        // `_FireAIShipWeapon` (@0x34ffd) aims at the target's slot as the
        // loop left it: an earlier slot moved, a later one not yet. Both
        // ships move alike, so the turret's lead reads only where the
        // target is.
        let catalog = FakePilotCatalog {
            weapons: vec![WeaponRecord {
                guidance: 4,
                ..blaster()
            }],
            ..armed()
        };
        let heading = |shooter_berth: u32, target_berth: u32, moving: bool| {
            let mut session = facing_an_npc(&catalog, 180);
            let velocity = if moving {
                Vec2::new(2.0, 0.0)
            } else {
                Vec2::ZERO
            };
            let shooter = &mut session.traffic.npcs_mut()[0];
            shooter.berth = shooter_berth;
            shooter.state.position = WRECKED;
            shooter.state.velocity = velocity;
            let shooter = shooter.id;
            let target = place(
                &mut session,
                target_berth,
                WRECKED + Vec2::new(0.0, -40.0),
                velocity,
            );
            pass(&mut session, &catalog, &FiringAt(target));
            session
                .shots()
                .iter()
                .find(|shot| shot.firer == ShipRef::Npc(shooter))
                .map(|shot| shot.heading)
                .expect("fired")
        };
        let still = heading(1, 2, false);
        assert_eq!(heading(2, 1, true), still, "both moved");
        assert_ne!(heading(1, 2, true), still, "where it stood");
    }

    #[test]
    fn an_npc_whose_flight_lands_takes_no_step_and_no_later_blast() {
        // Its slot is inactive from its own `_HandleShip`: the loop skips
        // its display, and a later slot's shockwave reaches it no more.
        let catalog = armed();
        let mut session = empty(&catalog);
        let site = session.sites[0];
        let lander = place(&mut session, 1, site.position, Vec2::ZERO);
        session.traffic.npcs_mut()[0].goal = Goal::Land(site.id);
        wreck(&mut session, 2, site.position + Vec2::new(20.0, 0.0));
        session.take_combat_events();
        pass(&mut session, &catalog, &Keep);
        assert_eq!(session.departed(), [(lander, Outcome::Landed(site.id))]);
        assert!(!hit(&session.take_combat_events(), lander));
    }
}
