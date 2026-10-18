//! Tests of the pursuit step as the traffic's tick runs it (see
//! [`pursuit`](crate::ai::pursuit)): after the goals are taken and before
//! the fire commands, for each NPC whose decision is due, its frame then
//! flown.

use super::{Traffic, World};
use crate::ai::fire::Manoeuvre;
use crate::ai::pursuit::{Kept, frame_of};
use crate::ai::{Behaviour, Goal, PlayerSide, Surroundings};
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::geometry::Vec2;
use crate::rulebook::RuleSource;
use crate::stats::ShipStats;
use crate::traffic::fixture::npc;
use crate::traffic::npc::Npc;
use nova_sim_base::fixture::{FAST, Ranged};

/// Every NPC attacks the player.
#[derive(Debug)]
struct Attacking;

impl Behaviour for Attacking {
    fn decide(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        Goal::Attack(ShipRef::Player)
    }
}

/// The player at `at`, at rest facing down.
fn player_at(at: Vec2) -> PlayerSide {
    let mut side = crate::ai::fixture::player(at.x, at.y);
    side.state.heading = 180.0;
    side
}

/// Traffic of one afterburner attacker at the centre facing up, deciding
/// every tick.
fn traffic() -> Traffic {
    let mut traffic = Traffic::with_interval(1);
    let mut ship = npc(1, ShipStats::new(FAST, &[]));
    ship.afterburner = true;
    ship.goal = Goal::Attack(ShipRef::Player);
    traffic.npcs = vec![ship];
    traffic
}

#[test]
fn an_afterburner_attacker_approaching_rushes_in_from_the_tick_after_and_flies_the_rush() {
    let mut traffic = traffic();
    let player = player_at(Vec2::new(-100.0, -400.0));
    let world = World {
        player: Some(player),
        ..World::new(&[])
    };
    // The bearing up and a little left, within 30 of its heading of 0.
    traffic.tick_in(&Attacking, world, &[], &mut Ranged::of(&[]));
    let ship = &traffic.npcs()[0];
    assert_eq!(ship.pursuit.kept, Some(Kept::Rush));
    let frame = frame_of(ship, &player.state, false).expect("a fight");
    assert_eq!(
        frame.iter().map(|body| body.manoeuvre).collect::<Vec<_>>(),
        [Manoeuvre::Approach]
    );
    let speed = ship.state.velocity.length();
    traffic.tick_in(&Attacking, world, &[], &mut Ranged::of(&[]));
    let ship = &traffic.npcs()[0];
    let frame = frame_of(ship, &player.state, false).expect("a fight");
    assert_eq!(
        frame.iter().map(|body| body.manoeuvre).collect::<Vec<_>>(),
        [Manoeuvre::Rush]
    );
    // FAST thrusts 900 / 5000 = 0.18; rushing, 2.75 times that.
    assert!(
        (ship.state.velocity.length() - speed - 0.495).abs() < 0.05,
        "{speed} {:?}",
        ship.state
    );
}

#[test]
fn the_step_leads_an_approach_as_the_worlds_approach_lead_says() {
    use crate::catalog::{WeaponId, WeaponRecord};
    use crate::combat::armament::{Armament, LauncherRounds, Shooter};
    use crate::combat::hull::Condition;
    use crate::combat::weapon::WeaponSpec;
    // A gun of 10 a tick, fired once (its last gun), and a turret of 20 a
    // tick held as its pick, at the player 400 above crossing right.
    let spec = |id: i16, guidance: i16, speed: i16| {
        WeaponSpec::new(&WeaponRecord {
            guidance,
            speed,
            count: 50,
            ..nova_sim_catalog::fixture::weapon(id)
        })
    };
    let mut traffic = traffic();
    let ship = &mut traffic.npcs[0];
    ship.afterburner = false;
    ship.armament = Armament::new([(spec(128, -1, 1000), 1), (spec(130, 4, 2000), 1)]);
    let mut fuel = ship.reserves.fuel;
    ship.armament.fire_pick(
        WeaponId(128),
        Condition::Intact,
        Shooter::PLAYER,
        &mut LauncherRounds::default(),
        &mut fuel,
        &mut crate::chance::NeverFires,
        &mut crate::combat::report::Reports::default(),
        RuleSource::Engine,
        &mut |_| Some(crate::combat::aim::Aim::along(0.0, None)),
    );
    ship.pick = Some(WeaponId(130));
    let mut player = player_at(Vec2::new(0.0, -400.0));
    player.state.velocity = Vec2::new(5.0, 0.0);
    let heading = |reading: RuleSource| {
        let mut traffic = traffic.clone();
        let world = World {
            player: Some(player),
            approach_lead: reading,
            ..World::new(&[])
        };
        traffic.tick_in(&Attacking, world, &[], &mut Ranged::of(&[]));
        traffic.npcs()[0]
            .pursuit
            .frame
            .and_then(|frame| frame.last())
            .map(|body| body.heading)
    };
    let (by_pick, by_last_gun) = (heading(RuleSource::Engine), heading(RuleSource::Bible));
    assert!(by_pick.is_some_and(|heading| heading > 0));
    assert!(by_last_gun > by_pick, "{by_last_gun:?} {by_pick:?}");
}
