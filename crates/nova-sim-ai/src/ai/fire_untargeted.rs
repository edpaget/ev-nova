//! What a mining NPC fires: `_AIFireGunUntargeted` (@0x80425-0x80535 in
//! the `EV Nova` executable), which order 0x14 runs while the miner faces
//! the heading it wants at asteroid slot 0 (@0x8843a-0x88443; see
//! [`mine_orders`](crate::ai::mine_orders)), and the trigger it leaves
//! ([`trigger`]), which [`fire::trigger`](crate::ai::fire::trigger) gives
//! for a ship in [`Goal::Mine`](crate::ai::Goal::Mine).
//!
//! **The walk** ([`find`]). It walks the weapon slots by weapon index
//! (stride 0xc8, @0x804f8-0x80505) for one with a count above 0
//! (@0x80443), a guidance it takes, not planet-type, with its ammunition
//! and reloaded:
//!
//! - guidance -1 and 6 pass (@0x80463, @0x80493);
//! - 0, a beam, passes unless its `Seeker` has
//!   [`IGNORES_ASTEROIDS`] while the ship's class has `Flags3`
//!   [`MINES`] (@0x8046e-0x80491), as such a beam would pass through the
//!   asteroid;
//! - 7, a front-quadrant turret, passes only with the ship's target +0x70
//!   at -1 (@0x8049f-0x804a7), which order 0x14 has just written
//!   (@0x8843a), so it always passes here;
//! - every other guidance fails (@0x8049d);
//! - a planet-type weapon (`Flags2` 0x0400, byte +0x23 bit 0x04) fails
//!   (@0x804a9);
//! - `_WeaponHasAmmo` ([`fire::has_ammo`](crate::ai::fire::has_ammo),
//!   @0x804b9) and a reload at or below none (+0xf8, @0x804c2-0x804cc).
//!
//! Of those, the one of the most `MassDmg`, a value of none or less
//! counting as 1, strictly more than the best so far from none, so a tie
//! goes to the lower weapon index (`jle` @0x804ea): the lowest
//! [`WeaponId`]. There is no range test, no sight test and no sound of its
//! own.
//!
//! **The pick and the flag** ([`trigger`], @0x8050b-0x80527): a find is
//! the pick (+0x72); with none the pick is left as it was. Then any pick,
//! found or held, raises the fire flag (+0xba), as the gun pass does
//! ([`fire`](crate::ai::fire)): a held pick fires even when the walk finds
//! nothing. Its ammunition and reload aren't tested again.

#[cfg(doc)]
use crate::ai::Goal;
use crate::ai::Surroundings;
use crate::ai::fire::has_ammo;
use crate::ai::mine_orders::{self, MineOrder};
use crate::ai::miner::MINES;
use crate::catalog::WeaponId;
use crate::combat::armament::{Mount, Trigger};
use crate::combat::weapon::Guidance;
#[cfg(doc)]
use crate::combat::weapon::IGNORES_ASTEROIDS;
use crate::traffic::npc::Npc;

/// The weapon `npc`'s walk finds, if any (see the module docs).
#[must_use]
pub fn find(npc: &Npc) -> Option<WeaponId> {
    let mut taken: Vec<&Mount> = npc
        .armament
        .mounts()
        .iter()
        .filter(|mount| takes(npc, mount))
        .collect();
    // The walk is by weapon index, whatever order the ship mounted them.
    taken.sort_by_key(|mount| mount.spec.id);
    let mut best: Option<(WeaponId, f32)> = None;
    for mount in taken {
        let damage = mount.spec.mass_damage;
        let worth = if damage <= 0.0 { 1.0 } else { damage };
        if best.is_none_or(|(_, most)| worth > most) {
            best = Some((mount.spec.id, worth));
        }
    }
    best.map(|(id, _)| id)
}

/// Whether the walk takes `mount` of `npc`'s (see the module docs).
fn takes(npc: &Npc, mount: &Mount) -> bool {
    let spec = &mount.spec;
    let guided = match spec.guidance {
        Guidance::Unguided | Guidance::Rocket | Guidance::FrontTurret => true,
        Guidance::Beam => !(spec.ignores_asteroids() && npc.miner & MINES != 0),
        _ => false,
    };
    // The count above 0 (@0x80443) holds for every mount: an armament
    // mounts none of a weapon carried none of (`Armament::new`).
    guided && !spec.planet_type() && has_ammo(npc, mount) && mount.reload <= 0.0
}

/// The trigger `npc` holds after `_AIFireGunUntargeted` (see the module
/// docs): the weapon its walk finds, else the pick it holds, fired and
/// kept; nothing with neither. It launches no fighter and asks no point
/// defence.
#[must_use]
pub fn trigger(npc: &Npc) -> Trigger {
    let pick = find(npc).or(npc.pick);
    Trigger {
        only: pick,
        keep: pick,
        ..Trigger::default()
    }
}

/// The trigger of `npc`, in [`Goal::Mine`], among `around`: order 0x14
/// with asteroid slot 0 ([`Surroundings::mining_rock`]) inside its
/// mining box ([`mine_orders::order`]) and facing the heading it wants
/// within its turn and [`FIRE_SLACK`](mine_orders::FIRE_SLACK)
/// ([`mine_orders::fires`]) gives [`trigger`]; any other tick of the goal
/// fires nothing and keeps its pick, as orders 1 and 0x13 run no fire.
///
/// The heading is read where order 0x14 reads it, after its hold
/// ([`mine_orders::hold`]) has eased its velocity and nudged its position
/// by a step of its `_ShipAccelRate` under [`Surroundings::flight`]: the
/// lead for the pick it holds, by that weapon's global record
/// ([`Surroundings::weapons`], @0x883c5), or the bearing with none. The
/// traffic flies the same hold later in the tick
/// ([`mine_flight`](crate::traffic::mine_flight)); the fire picks here
/// from the state the flight starts from, as the original's low level
/// picks before `_HandleShip` moves the ship.
#[must_use]
pub fn mining(npc: &Npc, around: &Surroundings) -> Trigger {
    let Some(rock) = around.mining_rock.as_ref() else {
        return Trigger::default();
    };
    if mine_orders::order(npc, Some(rock)) != MineOrder::Kill {
        return Trigger::default();
    }
    let accel = npc.flying_handling(0.0, around.flight).accel;
    let mut held = npc.state;
    mine_orders::hold(&mut held, rock, mine_orders::kill_step(accel));
    let pick = npc.pick.and_then(|pick| around.weapons.weapon(pick));
    let wanted = mine_orders::kill_heading(&held, rock, pick);
    if mine_orders::fires(npc.state.heading, wanted, npc.turn_rate()) {
        trigger(npc)
    } else {
        Trigger::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::NO_WEAPONS;
    use crate::ai::miner::SCOOPS;
    use crate::catalog::WeaponRecord;
    use crate::combat::armament::{Armament, Arsenal};
    use crate::combat::weapon::{IGNORES_ASTEROIDS, WeaponSpec};
    use crate::stats::ShipStats;
    use nova_sim_base::fixture::FAST;
    use nova_sim_catalog::fixture::weapon;
    use nova_sim_combat::asteroid::Asteroid;

    /// A gun of `id` doing `mass` mass damage.
    fn gun(id: i16, mass: i16) -> WeaponRecord {
        WeaponRecord {
            speed: 1000,
            count: 20,
            mass_dmg: mass,
            reload: 10,
            ..weapon(id)
        }
    }

    /// A weapon of `id` and `guidance` doing 5 mass damage.
    fn guided(id: i16, guidance: i16) -> WeaponRecord {
        WeaponRecord {
            guidance,
            ..gun(id, 5)
        }
    }

    /// A `Flags3` 0x0001 miner carrying one of each of `weapons`.
    fn miner(weapons: &[WeaponRecord]) -> Npc {
        let mut npc = crate::traffic::fixture::npc(1, ShipStats::new(FAST, &[]));
        npc.armament = Armament::new(weapons.iter().map(|record| (WeaponSpec::new(record), 1)));
        npc.miner = MINES;
        npc
    }

    #[allow(clippy::unnecessary_wraps)]
    fn id(id: i16) -> Option<WeaponId> {
        Some(WeaponId(id))
    }

    #[test]
    fn the_most_mass_damage_wins_none_or_less_counting_as_1() {
        assert_eq!(
            find(&miner(&[gun(128, 3), gun(129, 7), gun(130, 5)])),
            id(129)
        );
        // -4 and 0 both count as 1: a tie, so the lower index.
        assert_eq!(find(&miner(&[gun(131, -4), gun(130, 0)])), id(130));
        assert_eq!(find(&miner(&[gun(130, 0), gun(131, 2)])), id(131));
        assert_eq!(find(&miner(&[gun(131, 1), gun(130, -4)])), id(130));
    }

    #[test]
    fn a_tie_goes_to_the_lower_weapon_index_whatever_the_mount_order() {
        assert_eq!(find(&miner(&[gun(140, 5), gun(129, 5)])), id(129));
        assert_eq!(find(&miner(&[gun(129, 5), gun(140, 5)])), id(129));
    }

    #[test]
    fn a_planet_type_weapon_is_never_found() {
        let planet = WeaponRecord {
            flags2: 0x0400,
            ..gun(129, 50)
        };
        assert_eq!(find(&miner(&[gun(128, 1), planet])), id(128));
    }

    #[test]
    fn a_beam_missing_asteroids_is_left_out_for_a_ship_that_mines() {
        let missing = WeaponRecord {
            seeker: IGNORES_ASTEROIDS,
            ..guided(129, 0)
        };
        let beam = guided(130, 0);
        assert_eq!(find(&miner(&[missing, beam])), id(130));
        assert_eq!(find(&miner(&[missing])), None);
        let mut scooper = miner(&[missing]);
        scooper.miner = SCOOPS;
        assert_eq!(find(&scooper), id(129), "Flags3 0x0002 alone");
    }

    #[test]
    fn guns_rockets_beams_and_front_turrets_pass_and_no_other_guidance() {
        for guidance in [-1, 0, 6, 7] {
            assert_eq!(
                find(&miner(&[guided(129, guidance)])),
                id(129),
                "{guidance}"
            );
        }
        for guidance in [1, 3, 4, 5, 8, 9, 10, 99] {
            assert_eq!(find(&miner(&[guided(129, guidance)])), None, "{guidance}");
        }
    }

    #[test]
    fn a_reloading_mount_or_one_out_of_ammunition_is_passed_over() {
        let mut reloading = miner(&[gun(128, 1), gun(129, 9)]);
        reloading.armament.hold(WeaponId(129));
        assert_eq!(find(&reloading), id(128));
        let empty = WeaponRecord {
            ammo_type: 10,
            ..gun(129, 9)
        };
        assert_eq!(find(&miner(&[gun(128, 1), empty])), id(128));
    }

    #[test]
    fn a_mount_of_no_count_is_passed_over() {
        let mut npc = miner(&[]);
        npc.armament = Armament::new([
            (WeaponSpec::new(&gun(128, 1)), 1),
            (WeaponSpec::new(&gun(129, 9)), 0),
        ]);
        assert_eq!(find(&npc), id(128));
    }

    #[test]
    fn a_find_is_fired_and_kept() {
        let npc = miner(&[gun(128, 1)]);
        let fired = trigger(&npc);
        assert_eq!((fired.only, fired.keep), (id(128), id(128)));
        assert!(!fired.point_defence && !fired.bays && !fired.bays_after);
        let mut holding = miner(&[gun(128, 1)]);
        holding.pick = id(140);
        assert_eq!(trigger(&holding).only, id(128), "the find over the pick");
    }

    /// A live asteroid at (`x`, `y`), at rest.
    fn rock(x: f32, y: f32) -> nova_sim_combat::asteroid::Asteroid {
        nova_sim_combat::asteroid::Asteroid {
            position: crate::geometry::Vec2::new(x, y),
            velocity: crate::geometry::Vec2::ZERO,
            frame: 0.0,
            spin: 0.0,
            strength: 30,
            kind: 0,
        }
    }

    /// A gun-armed miner in [`Goal::Mine`] at (`x`, `y`) heading
    /// `heading`, its class turn 2 (a mining box of 120).
    fn mining_at(x: f32, y: f32, heading: f32) -> Npc {
        let mut npc = miner(&[gun(128, 1)]);
        npc.goal = crate::ai::Goal::Mine;
        npc.hull.class_turn = 2.0;
        npc.state.position = crate::geometry::Vec2::new(x, y);
        npc.state.heading = heading;
        npc
    }

    /// What `npc` fires by [`crate::ai::fire::trigger`] with `slot0` in
    /// slot 0.
    fn fired_at(npc: &Npc, slot0: Option<nova_sim_combat::asteroid::Asteroid>) -> Trigger {
        let npcs = [npc.clone()];
        let around = Surroundings {
            mining_rock: slot0,
            ..Surroundings::new(&[], &npcs)
        };
        crate::ai::fire::trigger(npc, &around, crate::ai::fire::FireRules::default())
    }

    /// The degrees beyond the bearing a miner of [`mining_at`]'s turn may
    /// face and fire: its turn and 10, less a whole degree.
    fn window(npc: &Npc) -> f32 {
        (npc.turn_rate() + mine_orders::FIRE_SLACK).ceil() - 1.0
    }

    #[test]
    fn a_miner_inside_the_box_facing_slot_0_fires_its_untargeted_gun() {
        // 100 off on each axis, in the band: the bearing to slot 0 is 314.
        let facing = mining_at(100.0, 100.0, 314.0);
        let fired = fired_at(&facing, Some(rock(0.0, 0.0)));
        assert_eq!((fired.only, fired.keep), (id(128), id(128)));
        let edge = mining_at(100.0, 100.0, 314.0 - window(&facing));
        assert_eq!(fired_at(&edge, Some(rock(0.0, 0.0))).only, id(128));
        let edge = mining_at(100.0, 100.0, 314.0 + window(&facing));
        assert_eq!(fired_at(&edge, Some(rock(0.0, 0.0))).only, id(128));
    }

    #[test]
    fn a_miner_just_outside_its_facing_window_fires_nothing() {
        let facing = mining_at(100.0, 100.0, 314.0);
        let off = window(&facing) + 1.0;
        for heading in [314.0 + off, 314.0 - off] {
            let away = mining_at(100.0, 100.0, heading);
            assert_eq!(
                fired_at(&away, Some(rock(0.0, 0.0))),
                Trigger::default(),
                "{heading}"
            );
        }
    }

    #[test]
    fn a_miner_chasing_or_with_slot_0_empty_fires_nothing() {
        // 121 off on x: outside the box, order 0x13, though it faces slot 0.
        let chasing = mining_at(121.0, 0.0, 270.0);
        assert_eq!(fired_at(&chasing, Some(rock(0.0, 0.0))), Trigger::default());
        let inside = mining_at(120.0, 0.0, 270.0);
        assert_eq!(fired_at(&inside, Some(rock(0.0, 0.0))).only, id(128));
        let mut holding = mining_at(100.0, 100.0, 314.0);
        holding.pick = id(140);
        assert_eq!(fired_at(&holding, None), Trigger::default(), "order 1");
    }

    #[test]
    fn a_miner_whose_ai_wait_runs_holds_its_fire() {
        let mut waiting = mining_at(100.0, 100.0, 314.0);
        waiting.trigger = fired_at(&waiting, Some(rock(0.0, 0.0)));
        assert_eq!(waiting.trigger.only, id(128));
        waiting.ai_wait = 5;
        assert_eq!(crate::ai::fire::gated(&waiting).only, None);
    }

    /// A miner of [`mining_at`] at (100, 100) heading `heading`, carrying
    /// and holding the guidance -1 gun 128 of `speed`, and the global
    /// records holding that gun.
    fn picking(speed: i16, heading: f32) -> (Npc, Arsenal) {
        let record = WeaponRecord {
            guidance: -1,
            speed,
            ..gun(128, 1)
        };
        let mut npc = mining_at(100.0, 100.0, heading);
        npc.armament = Armament::new([(WeaponSpec::new(&record), 1)]);
        npc.pick = id(128);
        (npc, Arsenal::of_specs([WeaponSpec::new(&record)]))
    }

    /// What `npc` fires by [`crate::ai::fire::trigger`] with `slot0` in
    /// slot 0 and the global weapon records `weapons`.
    fn fired_among(npc: &Npc, slot0: Asteroid, weapons: &Arsenal) -> Trigger {
        let npcs = [npc.clone()];
        let around = Surroundings {
            mining_rock: Some(slot0),
            weapons,
            ..Surroundings::new(&[], &npcs)
        };
        crate::ai::fire::trigger(npc, &around, crate::ai::fire::FireRules::default())
    }

    /// Slot 0 at the origin moving by (`v`, −`v`): across the line of
    /// fire of a miner at (100, 100), whose bearing to it is 314.
    fn crossing(v: f32) -> Asteroid {
        Asteroid {
            velocity: crate::geometry::Vec2::new(v, -v),
            ..rock(0.0, 0.0)
        }
    }

    #[test]
    fn a_miner_fires_facing_the_lead_for_its_pick_and_not_the_bare_bearing() {
        // Gun 128 at 10 px a tick: 141 px off, slot 0 moving by (3, −3)
        // and the miner eased to (0.27, −0.27), the lead is 337, 23
        // degrees off the bearing, beyond a turn of 3 and 10.
        let (lead, weapons) = picking(1000, 337.0);
        assert_eq!(
            fired_among(&lead, crossing(3.0), &weapons).only,
            id(128),
            "facing the lead"
        );
        let (bearing, weapons) = picking(1000, 314.0);
        assert_eq!(
            fired_among(&bearing, crossing(3.0), &weapons),
            Trigger::default(),
            "facing the bearing"
        );
        assert_eq!(
            fired_among(&bearing, crossing(3.0), &NO_WEAPONS).only,
            id(128),
            "the bearing, with no record of its pick"
        );
    }

    #[test]
    fn a_miner_leads_from_the_velocity_its_hold_has_eased() {
        // Gun 128 at 1 px a tick, slot 0 moving by (0.5, −0.5): from the
        // miner at rest the lead would be 351; the hold first eases it to
        // (0.27, −0.27), and from there the lead is 334.
        let (held, weapons) = picking(100, 334.0);
        assert_eq!(
            fired_among(&held, crossing(0.5), &weapons).only,
            id(128),
            "facing the lead after the hold"
        );
        let (unheld, weapons) = picking(100, 351.0);
        assert_eq!(
            fired_among(&unheld, crossing(0.5), &weapons),
            Trigger::default(),
            "facing the lead before the hold"
        );
    }

    #[test]
    fn with_nothing_found_a_held_pick_still_fires_and_with_none_nothing_does() {
        let mut holding = miner(&[guided(128, 4)]);
        holding.pick = id(150);
        let fired = trigger(&holding);
        assert_eq!((fired.only, fired.keep), (id(150), id(150)));
        assert_eq!(trigger(&miner(&[guided(128, 4)])), Trigger::default());
    }
}
