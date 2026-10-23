//! Tests of the rating stretch in [`Armament::fire_pick`]: an NPC's pick
//! fired at the player reloads stretched by the player's combat rating
//! ([`PlayerRating::stretch`], `_FireAIShipWeapon` @0x88fb3-0x8901b), before
//! the burst and the exclusive hold.

#![allow(clippy::float_cmp)]

use super::{Armament, LauncherRounds, Shooter};
use crate::catalog::{WeaponId, WeaponRecord};
use crate::chance::NeverFires;
use crate::combat::ai_fire::PlayerRating;
use crate::combat::aim::Aim;
use crate::combat::hull::Condition;
use crate::combat::report::Reports;
use crate::combat::weapon::{EXCLUSIVE, EXCLUSIVE_HOLD, SIMULTANEOUS, WeaponSpec};
use crate::reserves::Gauge;
use crate::rulebook::RuleSource;
use nova_sim_catalog::fixture::weapon;

/// A rating that stretches by x1.75: 0 against a yardstick of 1.
const NEW_PILOT: Option<PlayerRating> = Some(PlayerRating {
    rating: 0,
    yardstick: 1,
});

/// Weapon `id` of `Reload` `reload`.
fn gun(id: i16, reload: i16) -> WeaponRecord {
    WeaponRecord {
        reload,
        ..weapon(id)
    }
}

/// Fires `pick` from `armament` straight ahead, at the player as
/// `at_player` says; whether it fired.
fn fire(armament: &mut Armament, pick: i16, at_player: Option<PlayerRating>) -> bool {
    armament
        .fire_pick(
            WeaponId(pick),
            Condition::Intact,
            Shooter {
                npc: true,
                ..Shooter::PLAYER
            },
            &mut LauncherRounds::default(),
            &mut Gauge::default(),
            &mut NeverFires,
            &mut Reports::default(),
            RuleSource::Engine,
            &mut |_spec: &WeaponSpec| Some(Aim::along(0.0, None)),
            at_player,
        )
        .is_some()
}

fn armed(weapons: &[(WeaponRecord, u32)]) -> Armament {
    Armament::new(
        weapons
            .iter()
            .map(|(record, count)| (WeaponSpec::new(record), *count)),
    )
}

#[test]
fn a_pick_fired_at_the_player_reloads_stretched_by_the_rating() {
    let mut armament = armed(&[(gun(130, 20), 1)]);
    assert!(fire(&mut armament, 130, NEW_PILOT));
    assert_eq!(armament.mounts[0].reload, 35.0);
}

#[test]
fn a_pick_fired_at_anyone_else_reloads_unstretched() {
    let mut armament = armed(&[(gun(130, 20), 1)]);
    assert!(fire(&mut armament, 130, None));
    assert_eq!(armament.mounts[0].reload, 20.0);
}

#[test]
fn a_split_reload_is_stretched_after_the_split() {
    let mut armament = armed(&[(gun(130, 20), 2)]);
    assert_eq!(armament.mounts[0].spec.flags & SIMULTANEOUS, 0);
    assert!(fire(&mut armament, 130, NEW_PILOT));
    assert_eq!(armament.mounts[0].reload, 10.0 * 1.75);
}

#[test]
fn a_bursts_last_shot_takes_the_burst_reload_unstretched() {
    let bursting = WeaponRecord {
        burst_count: 3,
        burst_reload: 50,
        ..gun(146, 20)
    };
    let mut armament = armed(&[(bursting, 1)]);
    let mut timers = Vec::new();
    for _ in 0..3 {
        armament.mounts[0].reload = 0.0;
        assert!(fire(&mut armament, 146, NEW_PILOT));
        timers.push(armament.mounts[0].reload);
    }
    assert!(timers.contains(&50.0), "{timers:?}");
    assert!(
        timers.iter().all(|&timer| timer == 35.0 || timer == 50.0),
        "{timers:?}"
    );
    assert!(timers.contains(&35.0), "{timers:?}");
}

/// An exclusive weapon 150 of `Reload` 20 (and `burst`, `BurstReload`
/// 50), beside a plain gun 130.
fn exclusive(burst_count: i16) -> Armament {
    // The original's exclusive bit, `Flags2` 0x20 at +0x24 of its weapon
    // table (@0x891d4), is the engine's `EXCLUSIVE` on `flags3`.
    let exclusive = WeaponRecord {
        flags3: EXCLUSIVE,
        burst_count,
        burst_reload: 50,
        ..gun(150, 20)
    };
    armed(&[(gun(130, 5), 1), (exclusive, 1)])
}

#[test]
fn an_exclusive_pick_at_the_player_holds_the_others_at_its_stretched_reload_plus_2() {
    let mut armament = exclusive(0);
    assert!(fire(&mut armament, 150, NEW_PILOT));
    assert_eq!(armament.mounts[1].reload, 35.0);
    assert_eq!(armament.mounts[0].reload, 35.0 + EXCLUSIVE_HOLD);
    assert_eq!(armament.mounts[0].reload, 37.0);
}

#[test]
fn an_exclusive_bursts_last_shot_holds_the_others_at_the_burst_reload_plus_2() {
    let mut armament = exclusive(1);
    assert!(fire(&mut armament, 150, NEW_PILOT));
    assert_eq!(armament.mounts[1].reload, 50.0);
    assert_eq!(armament.mounts[0].reload, 52.0);
}
