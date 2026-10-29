//! The session's side of the miners (see
//! [`miner`](crate::ai::miner)): whether miners are active, the
//! original's `_miningShipsActive` (`*0x3b5558`), which the asteroid step
//! reads to refill asteroid slot 0
//! ([`Asteroids::refill_slot_0`](crate::asteroid::Asteroids::refill_slot_0)).
//! How a miner mines and scoops is the traffic's and the fight's (see
//! [`mine_orders`](crate::ai::mine_orders)).
//!
//! **The flag** ([`Space::miners_active`]). The tail of `_AIDispatch`
//! (@0x9003c-0x9005d) sets it for every ship whose dispatch reaches it
//! with `shïp` `Flags3` & 3 and +0x9a == -1, and `_DoPlayGameWork` clears
//! it each advance pass (@0x44250). Every dispatch that passes the
//! dispatch's gates reaches the tail
//! ([`dispatch_gate::reaches_think`](crate::ai::dispatch_gate::reaches_think)):
//! the miner's own think, every other type's, a tick off the AI timer, the
//! regroup pause, a ship waiting in its gate (goal 0x15), and a disabled
//! ship's reset. +0x9a == -1 is the engine's routing test,
//! [`miner::thinks_for`](crate::ai::miner::thinks_for): no leader, not the
//! player's escort or a mission escort, no carrier. So a player's escort
//! of the miner bits alone sets nothing.
//!
//! *Disabled ships count.* The disabled reset writes +0x9a = -1 just
//! before it jumps to the tail (@0x8feda → @0x8ff13 → @0x90032), so a
//! disabled or breaking-up ship of the miner bits sets the flag whatever
//! leader it had: here, any ship of the bits that reaches its think and
//! that `_IsDisabled` holds for ([`Npc::is_disabled`](crate::traffic::npc::Npc::is_disabled)).
//!
//! *Defence ships.* `_AIDispatch` routes a stellar's defence ship to
//! `_DefenseShipAI` (@0x8ff4d-0x8ff5a) before the miner (@0x8ff64), so one
//! of the miner bits sets nothing, breaking up included, as `_IsDisabled`
//! never holds for it then (@0x2d6f). Held disabled, the reset clears its
//! stellar (@0x8fef8) and it takes the tail, setting the flag.
//!
//! *Ordering against the original.* The flag is derived live as the
//! asteroid step runs ([`Session::tick_asteroids`]), not set by this
//! pass's thinks and kept. Between the two only the shots and the ship
//! loop run, before the beams and what the pass leaves, as
//! `_HandleAsteroids` (@0x444f5) runs there, so a miner destroyed this
//! pass is still in the traffic as the step reads the flag. The one
//! difference is a miner that leaves the system in this pass's fly half:
//! the original's flag still counts its think, and the engine's doesn't.
//! Slot 0 then isn't refilled that pass; no asteroid moves or changes,
//! only which slot holds one, which no frame shows. The flag settles
//! before any frame is drawn.

use super::Space;
use crate::ai::dispatch_gate::reaches_think;
use crate::ai::miner::{MINER_BITS, thinks_for};
#[cfg(doc)]
use crate::session::Session;

impl Space {
    /// Whether miners are active, `_miningShipsActive` (see the module
    /// docs): some NPC that reaches its think, of the miner bits, is
    /// routed to the miner or disabled.
    pub(super) fn miners_active(&self) -> bool {
        self.traffic.npcs().iter().any(|npc| {
            reaches_think(npc)
                && npc.miner & MINER_BITS != 0
                && ((thinks_for(npc) && npc.defends.is_none()) || npc.is_disabled())
        })
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use crate::ai::miner::{MINER_BITS, MINES, SCOOPS};
    use crate::ai::{Goal, NovaAi};
    use crate::asteroid::Asteroid;
    use crate::catalog::WeaponRecord;
    use crate::chance::NeverFires;
    use crate::combat::armament::Armament;
    use crate::combat::hull::Condition;
    use crate::combat::weapon::WeaponSpec;
    use crate::combat::{Rules, ShipRef};
    use crate::escort::EscortDuty;
    use crate::flight::Controls;
    use crate::geometry::Vec2;
    use crate::session::Session;
    use crate::stats::ShipStats;
    use crate::testkit::{FAST, FakePilotCatalog, catalog, npc, weapon};
    use crate::traffic::npc::{Mode, Npc, NpcId};

    /// An asteroid of kind `kind` at (`x`, `y`), moving (1, -0.5) a tick.
    fn rock(kind: u8, x: f32, y: f32) -> Asteroid {
        Asteroid {
            position: Vec2::new(x, y),
            velocity: Vec2::new(1.0, -0.5),
            frame: 0.0,
            spin: 0.0,
            strength: 30,
            kind,
        }
    }

    /// `catalog`'s session in flight with `ships` in the system and slot
    /// 0 of its field emptied, rocks left in slots 1 and 2.
    fn slot_0_gone(catalog: &FakePilotCatalog, ships: Vec<Npc>) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        assert_eq!(session.landed(), None);
        for ship in ships {
            session.space.traffic.add_npc(ship);
        }
        let field = &mut session.space.asteroids;
        for (kind, x) in [(0, -100.0), (1, 100.0), (2, 150.0)] {
            field.place(rock(kind, x, 50.0));
        }
        let types = session.statics.asteroid_types.clone();
        let _ = session.space.asteroids.destroy(0, &types, &mut NeverFires);
        assert_eq!(session.space.asteroids.get(0), None);
        session
    }

    /// A leaderless ship of [`FAST`] of the miner bits `bits`.
    fn miner(id: u32, bits: u16) -> Npc {
        Npc {
            miner: bits,
            ..npc(id, ShipStats::new(FAST, &[]))
        }
    }

    /// Whether a pass's asteroid step refilled slot 0 with slot 1's rock,
    /// moved on by its velocity, freeing slot 1.
    fn refills(ships: Vec<Npc>) -> bool {
        let catalog = catalog();
        let mut session = slot_0_gone(&catalog, ships);
        session.tick_asteroids(&mut NeverFires);
        let moved = rock(1, 101.0, 49.5);
        let Some(&slot0) = session.space.asteroids.get(0) else {
            assert_eq!(session.space.asteroids.get(1), Some(&moved));
            return false;
        };
        assert_eq!(slot0, moved, "slot 1's, moved on");
        assert_eq!(session.space.asteroids.get(1), None, "slot 1 freed");
        true
    }

    #[test]
    fn a_defence_ship_of_the_miner_bits_sets_no_miners_active_unless_disabled() {
        // `_AIDispatch` routes it to `_DefenseShipAI` (@0x8ff4d-0x8ff5a)
        // before the miner (@0x8ff64); held disabled, the reset clears its
        // stellar and it takes the tail (@0x8fef8, @0x8ff13), but breaking
        // up it is not disabled (@0x2d6f).
        let guard = |condition| Npc {
            defends: Some(crate::catalog::StellarId(128)),
            condition,
            ..miner(1, MINES)
        };
        assert!(!refills(vec![guard(Condition::Intact)]));
        let dying = Condition::Dying {
            ticks_left: 3,
            starting: false,
        };
        assert!(!refills(vec![guard(dying)]));
        assert!(refills(vec![guard(Condition::Disabled)]));
    }

    #[test]
    fn with_a_leaderless_miner_present_the_first_live_asteroid_moves_into_slot_0() {
        assert!(refills(vec![miner(1, MINES)]));
        assert!(refills(vec![miner(1, SCOOPS)]));
        assert!(refills(vec![npc(2, ShipStats::default()), miner(1, MINES)]));
    }

    #[test]
    fn with_no_miner_slot_0_stays_empty() {
        assert!(!refills(vec![]));
        assert!(!refills(vec![npc(1, ShipStats::default())]));
    }

    #[test]
    fn a_players_escort_or_a_led_miner_alone_refills_nothing() {
        let escort = Npc {
            escort: Some(EscortDuty {
                formation: crate::escort::FormationSlot::default(),
                order: None,
            }),
            ..miner(1, MINES)
        };
        assert!(!refills(vec![escort]), "the player's escort");
        let led = Npc {
            leader: Some(NpcId(2)),
            ..miner(1, MINES)
        };
        assert!(!refills(vec![led]), "a fleet's escort");
        let mut carried = miner(1, MINES);
        carried.take_carrier(
            crate::bay::Carrier {
                ship: crate::combat::ShipRef::Npc(NpcId(2)),
                window: 100.0,
                reach: 40.0,
            },
            crate::rulebook::RuleSource::Engine,
        );
        carried.leader = None;
        assert!(!refills(vec![carried]), "a carrier's fighter");
    }

    #[test]
    fn a_disabled_ship_of_the_miner_bits_refills_whatever_its_leader() {
        // The disabled reset clears +0x9a before the tail (@0x8feda).
        let escort = Npc {
            escort: Some(EscortDuty {
                formation: crate::escort::FormationSlot::default(),
                order: None,
            }),
            condition: Condition::Disabled,
            ..miner(1, MINES)
        };
        assert!(refills(vec![escort]));
        let not_a_miner = Npc {
            condition: Condition::Disabled,
            ..npc(1, ShipStats::default())
        };
        assert!(!refills(vec![not_a_miner]), "no miner bits");
    }

    #[test]
    fn a_miner_gliding_in_or_without_ai_refills_nothing() {
        let gliding = Npc {
            mode: Mode::JumpingIn { tick: 0 },
            ..miner(1, MINES)
        };
        assert!(!refills(vec![gliding]));
        let hulk = Npc {
            no_ai: true,
            ..miner(1, MINES)
        };
        assert!(!refills(vec![hulk]));
    }

    /// A session of [`mining_past_a_ship`]'s: the miner and the ship
    /// between it and slot 0.
    struct Scene {
        session: Session,
        miner: NpcId,
        between: NpcId,
    }

    /// Slot 0's strength: more than any test's shots take.
    const SOLID: i16 = 30_000;

    /// `catalog`'s session in flight with slot 0 at rest at (0, 300), a
    /// `Flags3` 0x0001 miner 100 below it facing it, armed with `weapon`,
    /// and an immobile ship halfway between, its shield and armour full.
    fn mining_past_a_ship(catalog: &FakePilotCatalog, weapon: WeaponRecord) -> Scene {
        let mut session = Session::start(catalog).expect("starts");
        // The system's own traffic is no part of the scene.
        session.space.traffic_due = false;
        session.space.asteroids.place(Asteroid {
            velocity: Vec2::ZERO,
            strength: SOLID,
            ..rock(0, 0.0, 300.0)
        });
        let mut mining = miner(1, MINES);
        mining.state.position = Vec2::new(0.0, 400.0);
        mining.armament = Armament::new([(WeaponSpec::new(&weapon), 1)]);
        let miner = session.space.traffic.add_npc(mining);
        let mut ship = npc(2, ShipStats::new(FAST, &[]));
        ship.state.position = Vec2::new(0.0, 350.0);
        ship.immobile = true;
        let between = session.space.traffic.add_npc(ship);
        Scene {
            session,
            miner,
            between,
        }
    }

    /// One advance pass of `session`'s game work, its traffic `NovaAi`'s.
    fn pass(session: &mut Session, catalog: &FakePilotCatalog) {
        session.game_work(
            Controls::default(),
            catalog,
            &NovaAi::default(),
            Rules::default(),
            &mut NeverFires,
        );
    }

    /// The shield and armour of `scene`'s ship between.
    fn standing(scene: &Scene) -> (f32, f32) {
        let reserves = scene
            .session
            .space
            .npc(scene.between)
            .expect("there")
            .reserves;
        (reserves.shield.now, reserves.armor.now)
    }

    /// A gun of 10 pixels a tick for 30 ticks.
    fn gun() -> WeaponRecord {
        WeaponRecord {
            speed: 1000,
            count: 30,
            mass_dmg: 5,
            energy_dmg: 5,
            reload: 60,
            ..weapon(128)
        }
    }

    /// A beam 200 long, lasting 20 ticks.
    fn beam() -> WeaponRecord {
        WeaponRecord {
            guidance: 0,
            count: 20,
            beam_length: 200,
            beam_width: 2,
            mass_dmg: 5,
            energy_dmg: 5,
            reload: 60,
            ..weapon(128)
        }
    }

    #[test]
    fn a_mining_ships_shot_passes_through_a_ship_into_slot_0() {
        let catalog = catalog();
        let mut scene = mining_past_a_ship(&catalog, gun());
        let before = standing(&scene);
        for _ in 0..30 {
            pass(&mut scene.session, &catalog);
            let miner = scene.session.space.npc(scene.miner).expect("there");
            assert_eq!(miner.goal, Goal::Mine);
        }
        assert_eq!(standing(&scene), before, "nothing hit the ship");
        let slot0 = scene.session.space.asteroids.get(0).expect("live");
        assert!(slot0.strength < SOLID, "the shot struck slot 0");
    }

    /// Whether `scene`'s miner has a shot in flight.
    fn fired(scene: &Scene) -> bool {
        let miner = ShipRef::Npc(scene.miner);
        scene
            .session
            .space
            .combat
            .shots()
            .iter()
            .any(|shot| shot.firer == miner)
    }

    /// Whether `scene`'s miner has a beam lit.
    fn beaming(scene: &Scene) -> bool {
        let miner = ShipRef::Npc(scene.miner);
        scene
            .session
            .space
            .combat
            .beams()
            .iter()
            .any(|beam| beam.firer == miner)
    }

    /// Takes `scene`'s miner off state 16, as a provocation would.
    fn stop_mining(scene: &mut Scene) {
        let miner = scene.session.space.npc_mut(scene.miner).expect("there");
        miner.goal = Goal::Wait;
    }

    #[test]
    fn a_shot_in_flight_hits_the_ship_once_its_firer_no_longer_mines() {
        let catalog = catalog();
        let mut scene = mining_past_a_ship(&catalog, gun());
        let before = standing(&scene);
        for _ in 0..100 {
            if fired(&scene) {
                break;
            }
            pass(&mut scene.session, &catalog);
        }
        assert!(fired(&scene));
        for _ in 0..10 {
            // Its goal leaves state 16 as each pass begins, before the
            // collision passes read it; its think puts it back after.
            stop_mining(&mut scene);
            pass(&mut scene.session, &catalog);
        }
        assert_ne!(standing(&scene), before, "the shot hit the ship");
    }

    #[test]
    fn a_mining_ships_beam_passes_through_a_ship_and_hits_it_once_it_no_longer_mines() {
        let catalog = catalog();
        let mut scene = mining_past_a_ship(&catalog, beam());
        let before = standing(&scene);
        for _ in 0..100 {
            if beaming(&scene) {
                break;
            }
            pass(&mut scene.session, &catalog);
        }
        assert!(beaming(&scene));
        for _ in 0..5 {
            pass(&mut scene.session, &catalog);
            assert!(beaming(&scene));
        }
        assert_eq!(standing(&scene), before, "nothing hit the ship");
        // The beam's hits come after the pass's thinks, so its firer leaves
        // state 16 for good: no longer a miner, it waits.
        let miner = scene.session.space.npc_mut(scene.miner).expect("there");
        miner.miner = 0;
        miner.goal = Goal::Wait;
        pass(&mut scene.session, &catalog);
        assert_ne!(
            scene.session.space.npc(scene.miner).expect("there").goal,
            Goal::Mine
        );
        assert_ne!(standing(&scene), before, "the beam hit the ship");
    }

    #[test]
    fn a_scooping_miner_takes_a_ton_of_its_box_then_leaves_on_its_full_hold() {
        let mut catalog = catalog();
        catalog.box_frames[1] = Some((40, 40));
        let mut session = Session::start(&catalog).expect("starts");
        session.space.traffic_due = false;
        // Slot 0 is live, so it would mine (state 16) but for the box.
        session.space.asteroids.place(Asteroid {
            velocity: Vec2::ZERO,
            strength: SOLID,
            ..rock(0, 300.0, -300.0)
        });
        let mut scooper = miner(1, MINER_BITS);
        scooper.can_scoop = true;
        scooper.holds = 20;
        scooper.cargo.add(0, 19);
        scooper.hull.hit_radius = 10.0;
        scooper.state.position = Vec2::new(-300.0, 300.0);
        let scooper = session.space.traffic.add_npc(scooper);
        session
            .space
            .boxes
            .spawn_scoopable(Vec2::new(-300.0, 100.0), Some(2), 1, &mut NeverFires);
        let goal = |session: &Session| session.space.npc(scooper).expect("there").goal;
        pass(&mut session, &catalog);
        assert_eq!(goal(&session), Goal::Scoop, "state 17");
        for _ in 0..300 {
            if session.cargo_boxes().is_empty() {
                break;
            }
            assert_eq!(goal(&session), Goal::Scoop);
            pass(&mut session, &catalog);
        }
        assert!(session.cargo_boxes().is_empty(), "scooped");
        let hold = session.space.npc(scooper).expect("there").cargo;
        assert_eq!((hold.tons(0), hold.tons(2), hold.total()), (19, 1, 20));
        for _ in 0..20 {
            pass(&mut session, &catalog);
            assert_eq!(goal(&session), Goal::JumpOut, "state 2, and it stays");
        }
    }
}
