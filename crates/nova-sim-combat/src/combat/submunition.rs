//! Sub-munitions: the shots a shot releases when its proximity fuse sets
//! it off by a ship or it comes to the end of its life.
//!
//! The values are the original's (`_SpawnShotSubmunitions`, called from
//! `_HandleShot` and `_HandleShipHit` in the `EV Nova` executable):
//!
//! - A shot releases its weapon's [`Submunitions`] on a proximity hit
//!   ([`Fuse::Proximity`](super::projectile::Fuse::Proximity)), aimed at
//!   the ship it hit, and at the end of its life, aimed at the ship its
//!   damage is aimed at ([`Shot::aimed_at`], none once it is lost),
//!   unless its weapon releases none then (`Flags2` 0x0020). A shot that
//!   touches a ship releases none: the sprite collision passes call
//!   `_HandleShipHit` with `spawnSubs` 0 (@0x37095), the proximity pass
//!   with 1 (@0x37678). It releases
//!   them only while its generation is below the `SubLimit`, when there is
//!   one. A proximity hit releases them after its hit and blast land
//!   (`_HandleShipHit` @0x36f12-0x36f47), so they seek among the ships as
//!   that hit left them; an end of life releases them before its own blast
//!   lands (`_HandleShot` @0x35f49, the blast @0x36106). Either way they
//!   read what every shot handled before it landed, never a later one.
//! - Each of the `SubCount` shots starts where its parent is, a
//!   generation on, fired by its parent's firer and carrying its parent's
//!   [`Source`](super::projectile::Source) ([`release`]). The loop seats
//!   each in turn (`_SpawnShot(…, 1)` @0x3f328) and only then aims it
//!   ([`aim`], @0x3f398-0x3f5a5), so each sub-munition is aimed on its
//!   own. A point-defence (9) parent's take its own target. With `Flags2`
//!   0x0010 each takes the nearest ship it can hit, as
//!   `_FindNearestValidShotTarget` (@0x6a3a-0x6af3) asks `_ShotCanHitShip`
//!   of every active ship, the player's included, of the seated shot
//!   itself with no target of its own (@0x3f3c3): so it reads the shot's
//!   leftover miss roll (below) and its interference jam
//!   ([`Jam::Interfered`](super::projectile::Jam::Interfered), rolled as
//!   it is seated, @0x3eab2-0x3eb43), with which it can hit no ship. Two
//!   sub-munitions of one release can seek different ships. The nearest
//!   is measured in whole pixels, each coordinate truncated toward zero
//!   (@0x6a7e-0x6ad4), and of two equally near the lower ship slot wins
//!   ([`Target::slot`](super::projectile::Target::slot), @0x6abd-0x6ace).
//!   The original's 16-bit sum, which wraps past about 181 pixels, is
//!   wide here. With none it takes the given target, and heads at the
//!   table's whole-degree bearing to it (`_CalcInterceptAngle`
//!   @0x3f421-0x3f42b); otherwise it takes the given target and the
//!   parent's heading.
//! - A positive `SubTheta` turns each by `θ - below(2θ + 1)`, within θ
//!   either way; a negative one spreads them in a starburst |θ| apart,
//!   centred on the heading less half the spread in whole degrees
//!   (`|θ|·(count − 1) / 2` in integers, @0x3f49c-0x3f4da), so an odd
//!   spread leans half a degree clockwise. The heading is then taken
//!   round once each way (@0x3f50f-0x3f54f).
//! - Each takes its `Speed` along its truncated heading from rest
//!   (`_Accel(trunc(heading))` @0x3f597), a bomb's (5) too,
//!   whoever fired the parent; a rocket's (6) starts with its parent's
//!   velocity instead, with no speed added (@0x3f55c-0x3f5a5).
//! - None rolls a turret miss ([`Shot::miss_roll`]): the original's
//!   `_SpawnShot(…, 1)` never writes the roll (+0x42, written only under
//!   -0x42 @0x3e89e), so a sub-munition carries whatever its shot slot's
//!   last ship-fired shot left there: -1, 0 for a slot never used, or a
//!   turret's or point defence's roll. The fight gives it that leftover
//!   as it takes its slot
//!   ([`shot_slots`](super::shot_slots), by
//!   [`RuleKey::ReusedShotSlotRoll`](crate::RuleKey::ReusedShotSlotRoll)'s
//!   engine reading; none by the other), so it can miss an intact ship of
//!   a lower class, and its seek, made once it is seated, passes such a
//!   ship over.
//! - Each takes its sprite layer by its own guidance, as `_SpawnShot`
//!   places it ([`ShotLayer::spawned`], @0x3f328): a turret (4) over the
//!   ships when its parent's source is a ship whose class has `Flags3`
//!   0x0040, read of whatever ship its parent's source slot
//!   ([`Shot::firer_slot`]) holds as it spawns, or that slot's last ship
//!   (see the fight's `source_slot` module): the caller gives the bit
//!   (`turret_over`).
//! - The release plays the sub-munition's `Sound` once at the parent, at
//!   [`SUBMUNITION_PRIORITY`](super::SUBMUNITION_PRIORITY) (see
//!   [`Combat`](super::Combat)).
//!
//! [`Submunitions`]: super::weapon::Submunitions

use super::ShipRef;
use super::projectile::{Shot, ShotLayer, Source};
use super::sides::Sides;
use super::weapon::{Guidance, WeaponSpec};
use crate::chance::Chance;
use crate::flight::ShipState;
use crate::geometry::Vec2;
use crate::trig;

/// The sub-munitions of `sub` that `parent` releases, each at `target`
/// (the ship it hit, or its own target) with its parent's heading, in its
/// sprite layer by its own guidance and `turret_over`, whether its
/// firer's class draws turret shots over the ships: what
/// `_SpawnShot(…, 1)` seats (@0x3f328), unaimed. Each is aimed only once
/// it has its slot ([`aim`]; see the module docs).
#[must_use]
pub fn release(
    parent: &Shot,
    sub: &WeaponSpec,
    (target, turret_over): (Option<ShipRef>, bool),
) -> Vec<Shot> {
    let Some(subs) = parent
        .weapon
        .submunitions
        .filter(|subs| subs.releases_at(parent.generation))
    else {
        return Vec::new();
    };
    let launcher = ShipState {
        position: parent.position,
        velocity: Vec2::ZERO,
        heading: parent.heading,
    };
    // `_SpawnShot(parent +0x2c, ..)` reads the parent's source slot
    // (@0x3f328), a ship's while it is one.
    let ship_source = matches!(parent.source, Source::Ship(_));
    let unaimed = Shot {
        generation: parent.generation + 1,
        source: parent.source,
        firer_slot: parent.firer_slot,
        target,
        aimed_at: target,
        layer: ShotLayer::spawned(sub.guidance, ship_source, turret_over),
        turret_over,
        ..Shot::launch(*sub, parent.firer, parent.fleet, &launcher, parent.heading)
    };
    vec![unaimed; subs.count as usize]
}

/// Sub-munition `n` of those `parent` releases ([`release`]), `shot`, as
/// it is aimed once seated, carrying its slot's leftover roll and its
/// interference jam, among the ships of `sides`, its spread drawn on
/// `chance` (@0x3f398-0x3f5a5; see the module docs).
pub fn aim(parent: &Shot, shot: Shot, n: u32, sides: &Sides, chance: &mut dyn Chance) -> Shot {
    let targets = sides.targets;
    let given = shot.target;
    let (target, heading) = if parent.weapon.guidance == Guidance::PointDefence {
        (parent.target, parent.heading)
    } else if parent.weapon.seeks_with_subs() {
        // The seated shot itself, its target -1 (@0x3f3c3). Clearing the
        // target changes nothing it finds: it is the given one, which the
        // seek falls back to, and a homing shot that may hit only it finds
        // it or nothing. A shot no ship fired reads `aimed_at`.
        let mut seeking = shot;
        seeking.target = None;
        seeking.aimed_at = None;
        let nearest = targets
            .iter()
            .filter(|ship| seeking.can_hit(ship, sides))
            .min_by_key(|ship| (whole_pixels_apart(shot.position, ship.position), ship.slot))
            .map(|ship| ship.ship);
        let target = nearest.or(given);
        let heading = target
            .and_then(|target| targets.iter().find(|ship| ship.ship == target))
            .map_or(parent.heading, |ship| {
                f32::from(trig::intercept_angle(parent.position, ship.position))
            });
        (target, heading)
    } else {
        (given, parent.heading)
    };
    let (theta, count) = parent
        .weapon
        .submunitions
        .map_or((0, 1), |subs| (subs.theta, subs.count));
    let spread = u32::from(theta.unsigned_abs());
    let offset = if theta > 0 {
        f32::from(theta) - chance.below(spread * 2 + 1) as f32
    } else {
        // Halved in whole degrees (@0x3f49c-0x3f4da), as `count - 1` is
        // never below none.
        let centred = spread * count.saturating_sub(1) / 2;
        (spread * n) as f32 - centred as f32
    };
    // Wrapped once (@0x3f50f-0x3f54f).
    let heading = trig::wrap_once(heading + offset);
    // A rocket keeps its parent's velocity; any other, a bomb too, takes
    // its `Speed` along its truncated heading from rest (@0x3f55c-0x3f5a5).
    let velocity = if shot.weapon.guidance == Guidance::Rocket {
        parent.velocity
    } else {
        trig::accel(trig::whole(heading), shot.weapon.speed)
    };
    Shot {
        target,
        aimed_at: target,
        heading,
        velocity,
        ..shot
    }
}

/// How far apart `a` and `b` are, squared, as
/// `_FindNearestValidShotTarget` measures it (@0x6a7e-0x6ad4): each
/// coordinate truncated toward zero (`cvttss2si`) before the difference.
/// The original forms it in 16 bits, wrapping past about 181 pixels; here
/// it is wide.
fn whole_pixels_apart(a: Vec2, b: Vec2) -> i64 {
    let whole = |value: f32| value.trunc() as i64;
    let dx = whole(b.x) - whole(a.x);
    let dy = whole(b.y) - whole(a.y);
    dx * dx + dy * dy
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::{ShipId, WeaponId, WeaponRecord};
    use crate::combat::NpcId;
    use crate::combat::fixture::among;
    use crate::combat::hull::Condition;
    use crate::combat::projectile::{Jam, Source, Target};
    use crate::combat::sides::Ties;
    use crate::flight::{ShipState, facing};
    use crate::geometry::Vec2;
    use crate::ship_layer::Slot;
    use crate::trig;
    use nova_sim_base::fixture::Draws;
    use nova_sim_catalog::fixture::weapon;

    const FIRER: ShipRef = ShipRef::Npc(NpcId(1));
    const HIT: ShipRef = ShipRef::Npc(NpcId(2));
    const NEAR: ShipRef = ShipRef::Npc(NpcId(3));

    /// A parent of `guidance` releasing `count` of weapon 148 at `theta`,
    /// up to `limit` generations, with `flags2`, at (100, 50) heading
    /// right at 5 pixels a tick down, at `target`.
    fn parent(guidance: i16, count: i16, theta: i16, limit: i16, flags2: u16) -> Shot {
        let spec = WeaponSpec::new(&WeaponRecord {
            guidance,
            speed: 500,
            sub_count: count,
            sub_type: Some(WeaponId(148)),
            sub_theta: theta,
            sub_limit: limit,
            flags2,
            ..weapon(182)
        });
        Shot {
            target: Some(HIT),
            ..Shot::launch(
                spec,
                FIRER,
                FIRER,
                &ShipState {
                    position: Vec2::new(100.0, 50.0),
                    velocity: Vec2::new(0.0, 5.0),
                    heading: 90.0,
                },
                90.0,
            )
        }
    }

    /// Sub-munition 148 of `guidance`, 13 pixels a tick.
    fn sub(guidance: i16) -> WeaponSpec {
        WeaponSpec::new(&WeaponRecord {
            guidance,
            speed: 1300,
            count: 250,
            durability: 2,
            ..weapon(148)
        })
    }

    fn ship(ship: ShipRef, x: f32, y: f32) -> Target {
        Target {
            ship,
            ship_type: ShipId(128),
            fleet: ship,
            ties: Ties::default(),
            position: Vec2::new(x, y),
            velocity: Vec2::ZERO,
            radius: 10.0,
            sprite_x: 30.0,
            sheet: true,
            condition: Condition::Intact,
            disabled: false,
            miss_class: 100,
            slot: crate::ship_layer::Slot::Berth(0),
        }
    }

    /// The sub-munitions of `sub` that `parent` releases at `target`,
    /// each aimed among the ships of `sides` as it is seated, carrying
    /// nothing from its slot.
    fn released(
        parent: &Shot,
        sub: &WeaponSpec,
        given: (Option<ShipRef>, bool),
        sides: &Sides,
        chance: &mut dyn Chance,
    ) -> Vec<Shot> {
        release(parent, sub, given)
            .into_iter()
            .zip(0..)
            .map(|(shot, n)| aim(parent, shot, n, sides, chance))
            .collect()
    }

    /// The one sub-munition a seeking parent at `at` releases at `given`,
    /// seated carrying `roll` and `jam`, aimed among `targets`.
    fn seated(
        at: Vec2,
        (roll, jam): (Option<u32>, Jam),
        given: Option<ShipRef>,
        targets: &[Target],
    ) -> Shot {
        let seeking = Shot {
            position: at,
            ..parent(-1, 1, 0, 0, 0x0010)
        };
        let [shot] = release(&seeking, &sub(-1), (given, false))[..] else {
            panic!("one sub-munition");
        };
        let shot = Shot {
            miss_roll: roll,
            jam,
            ..shot
        };
        aim(&seeking, shot, 0, &among(targets), &mut Draws::of(&[]))
    }

    #[test]
    fn a_released_sub_munition_waits_unaimed_at_the_given_target() {
        // `_SpawnShot(…, 1)` gives it the target it is given (@0x3f328);
        // the seek comes after its slot (@0x3f3cf).
        let seeking = parent(-1, 2, 45, 0, 0x0010);
        let subs = release(&seeking, &sub(-1), (Some(HIT), false));
        assert_eq!(subs.len(), 2);
        for shot in subs {
            assert_eq!((shot.target, shot.aimed_at), (Some(HIT), Some(HIT)));
            assert_eq!(shot.heading, 90.0, "the parent's");
        }
    }

    #[test]
    fn a_seated_sub_munition_seeks_past_an_intact_ship_its_leftover_roll_is_over() {
        // Its slot's leftover roll of 90 is over the near fighter's class
        // of 80 (`_ShotCanHitShip` @0x448bd-0x448d5): it heads at the far
        // one, of class 100. With no roll, or the near one disabled, it
        // takes the near one.
        let near = Target {
            miss_class: 80,
            ..ship(NEAR, 100.0, 100.0)
        };
        let far = ship(HIT, 300.0, 50.0);
        let at = Vec2::new(100.0, 50.0);
        let shot = seated(at, (Some(90), Jam::Clear), None, &[near, far]);
        assert_eq!((shot.target, shot.heading), (Some(HIT), 90.0));
        assert_eq!(shot.aimed_at, Some(HIT));
        let shot = seated(at, (None, Jam::Clear), None, &[near, far]);
        assert_eq!((shot.target, shot.heading), (Some(NEAR), 180.0));
        let disabled = Target {
            disabled: true,
            ..near
        };
        let shot = seated(at, (Some(90), Jam::Clear), None, &[disabled, far]);
        assert_eq!(shot.target, Some(NEAR), "disabled, never missed");
    }

    #[test]
    fn an_interfered_sub_munition_seeks_nothing_and_takes_the_given_target() {
        // Jammed at its spawn (+0x30 = 999, @0x3eb43), it can hit no ship
        // (@0x447f7): it falls back to the given target (@0x3f3ec-0x3f3f0)
        // and heads at it.
        let targets = [ship(NEAR, 100.0, 60.0), ship(HIT, 100.0, 450.0)];
        let at = Vec2::new(100.0, 50.0);
        let shot = seated(at, (None, Jam::Interfered), Some(HIT), &targets);
        assert_eq!((shot.target, shot.heading), (Some(HIT), 180.0));
        let shot = seated(at, (None, Jam::Clear), Some(HIT), &targets);
        assert_eq!(shot.target, Some(NEAR), "unjammed, the nearest");
    }

    #[test]
    fn the_seek_measures_whole_pixels_and_ties_go_to_the_lower_slot() {
        // From x = 0.9 (0 whole), A at 10.0 is 9.1 off and B at -9.5 is
        // 10.4, but in whole pixels (`cvttss2si`, @0x6a7e-0x6ad4) A is 10
        // and B 9.
        let at = Vec2::new(0.9, 0.0);
        let a = Target {
            slot: Slot::Berth(5),
            ..ship(NEAR, 10.0, 0.0)
        };
        let b = Target {
            slot: Slot::Berth(2),
            ..ship(HIT, -9.5, 0.0)
        };
        let shot = seated(at, (None, Jam::Clear), None, &[a, b]);
        assert_eq!(shot.target, Some(HIT), "nearer in whole pixels");
        // At -10.7, 10 whole pixels off as A is: the lower slot, though A
        // is first and nearer by float (@0x6abd-0x6ace).
        let tied = Target {
            position: Vec2::new(-10.7, 0.0),
            ..b
        };
        let shot = seated(at, (None, Jam::Clear), None, &[a, tied]);
        assert_eq!(shot.target, Some(HIT), "berth 2 under berth 5");
        let shot = seated(at, (None, Jam::Clear), None, &[tied, a]);
        assert_eq!(shot.target, Some(HIT), "in any order");
        let player = Target {
            slot: Slot::Player,
            ..ship(ShipRef::Player, 10.0, 0.0)
        };
        let shot = seated(at, (None, Jam::Clear), None, &[tied, player]);
        assert_eq!(shot.target, Some(ShipRef::Player), "slot 0");
    }

    /// The sub-munitions of `sub` that `parent` releases at nothing, its
    /// firer's class drawing turret shots over the ships or not.
    fn release_over(parent: &Shot, sub: &WeaponSpec, turret_over: bool) -> Vec<Shot> {
        released(
            parent,
            sub,
            (None, turret_over),
            &among(&[]),
            &mut Draws::of(&[]),
        )
    }

    fn headings(subs: &[Shot]) -> Vec<f32> {
        subs.iter().map(|shot| shot.heading).collect()
    }

    #[test]
    fn each_sub_munition_starts_where_its_parent_is_a_generation_on() {
        let parent = Shot {
            generation: 2,
            source: Source::Ship(Ties {
                govt: Some(crate::catalog::GovtId(140)),
                ..Ties::default()
            }),
            ..parent(-1, 3, 0, 0, 0)
        };
        let mut chance = Draws::of(&[]);
        let subs = released(
            &parent,
            &sub(-1),
            (Some(HIT), false),
            &among(&[]),
            &mut chance,
        );
        assert!(chance.asked.is_empty(), "no spread, nothing drawn");
        assert_eq!(subs.len(), 3);
        for shot in &subs {
            assert_eq!(shot.position, Vec2::new(100.0, 50.0));
            assert_eq!(shot.generation, 3);
            assert_eq!((shot.firer, shot.fleet), (FIRER, FIRER));
            assert_eq!(shot.source, parent.source, "its firer's ties");
            assert_eq!(shot.weapon.id, WeaponId(148));
            assert_eq!(shot.target, Some(HIT));
            assert_eq!(shot.heading, 90.0, "the parent's heading");
            assert_eq!(shot.durability, 2.0, "its own weapon's");
            assert_eq!(shot.age, 0);
        }
    }

    #[test]
    fn a_sub_munition_is_disable_only_as_its_own_weapon_is() {
        let disabling = WeaponSpec::new(&WeaponRecord {
            flags2: 0x1000,
            ..weapon(148)
        });
        let mut chance = Draws::of(&[]);
        for (parent_flags2, sub, disable_only) in [
            (0x1000, sub(-1), false),
            (0, disabling, true),
            (0x1000, disabling, true),
        ] {
            let parent = parent(-1, 2, 0, 0, parent_flags2);
            let subs = released(&parent, &sub, (None, false), &among(&[]), &mut chance);
            assert_eq!(subs.len(), 2);
            for shot in &subs {
                assert_eq!(shot.disable_only, disable_only, "{parent_flags2:#x}");
            }
        }
    }

    #[test]
    fn a_positive_theta_turns_each_within_it_either_way() {
        let mut chance = Draws::of(&[0, 45, 90]);
        let subs = released(
            &parent(-1, 3, 45, 0, 0),
            &sub(-1),
            (None, false),
            &among(&[]),
            &mut chance,
        );
        assert_eq!(headings(&subs), [135.0, 90.0, 45.0]);
        assert_eq!(chance.asked, [91, 91, 91]);
    }

    #[test]
    fn a_negative_theta_spreads_them_in_a_starburst() {
        let mut chance = Draws::of(&[]);
        let subs = released(
            &parent(-1, 5, -10, 0, 0),
            &sub(-1),
            (None, false),
            &among(&[]),
            &mut chance,
        );
        assert_eq!(headings(&subs), [70.0, 80.0, 90.0, 100.0, 110.0]);
        let pair = released(
            &parent(-1, 2, -10, 0, 0),
            &sub(-1),
            (None, false),
            &among(&[]),
            &mut chance,
        );
        assert_eq!(headings(&pair), [85.0, 95.0]);
        assert!(chance.asked.is_empty(), "nothing drawn");
    }

    #[test]
    fn seeking_the_nearest_takes_the_nearest_ship_they_can_hit_and_heads_at_it() {
        let targets = [
            ship(HIT, 100.0, 450.0),
            ship(NEAR, 100.0, -50.0),
            ship(FIRER, 100.0, 60.0),
            Target {
                condition: Condition::Destroyed,
                ..ship(ShipRef::Npc(NpcId(4)), 110.0, 50.0)
            },
        ];
        let seeking = parent(-1, 2, 0, 0, 0x0010);
        let subs = released(
            &seeking,
            &sub(-1),
            (Some(HIT), false),
            &among(&targets),
            &mut Draws::of(&[]),
        );
        assert_eq!(
            subs.iter().map(|s| s.target).collect::<Vec<_>>(),
            [Some(NEAR); 2]
        );
        assert_eq!(headings(&subs), [0.0, 0.0], "at its bearing");
        let alone = released(
            &seeking,
            &sub(-1),
            (Some(HIT), false),
            &among(&targets[..1]),
            &mut Draws::of(&[]),
        );
        assert_eq!(alone[0].target, Some(HIT), "the given target, the only one");
        assert_eq!(alone[0].heading, 180.0, "at its bearing");
        // Nearer the parent at (100, 50), though nearer the centre is the
        // other.
        let measured = [ship(NEAR, 100.0, -50.0), ship(HIT, -100.0, -40.0)];
        let subs = released(
            &seeking,
            &sub(-1),
            (None, false),
            &among(&measured),
            &mut Draws::of(&[]),
        );
        assert_eq!(subs[0].target, Some(NEAR), "from the parent");
        let nothing = released(
            &seeking,
            &sub(-1),
            (Some(HIT), false),
            &among(&[]),
            &mut Draws::of(&[]),
        );
        assert_eq!(nothing[0].target, Some(HIT), "the given target");
        assert_eq!(nothing[0].heading, 90.0, "nowhere to head");
        let unseeking = released(
            &parent(-1, 1, 0, 0, 0),
            &sub(-1),
            (Some(HIT), false),
            &among(&targets),
            &mut Draws::of(&[]),
        );
        assert_eq!(unseeking[0].target, Some(HIT));
        assert_eq!(unseeking[0].heading, 90.0, "the parent's heading");
    }

    #[test]
    fn seeking_sub_munitions_can_pick_the_player_or_a_breaking_up_ship() {
        let seeking = parent(-1, 1, 0, 0, 0x0010);
        let targets = [ship(HIT, 100.0, 450.0), ship(ShipRef::Player, 100.0, 80.0)];
        let subs = released(
            &seeking,
            &sub(-1),
            (Some(HIT), false),
            &among(&targets),
            &mut Draws::of(&[]),
        );
        assert_eq!(subs[0].target, Some(ShipRef::Player));
        let dying = Target {
            condition: Condition::Dying {
                ticks_left: 3,
                starting: false,
            },
            ..ship(NEAR, 100.0, 60.0)
        };
        let subs = released(
            &seeking,
            &sub(-1),
            (Some(HIT), false),
            &among(&[targets[0], targets[1], dying]),
            &mut Draws::of(&[]),
        );
        assert_eq!(subs[0].target, Some(NEAR), "a hulk is nearer");
    }

    #[test]
    fn seeking_sub_munitions_skip_their_firers_government() {
        let of_140 = Ties {
            govt: Some(crate::catalog::GovtId(140)),
            ..Ties::default()
        };
        let seeking = Shot {
            source: Source::Ship(of_140),
            ..parent(-1, 1, 0, 0, 0x0010)
        };
        let targets = [
            ship(HIT, 100.0, 450.0),
            Target {
                ties: of_140,
                ..ship(NEAR, 100.0, 60.0)
            },
        ];
        let subs = released(
            &seeking,
            &sub(-1),
            (None, false),
            &among(&targets),
            &mut Draws::of(&[]),
        );
        assert_eq!(subs[0].target, Some(HIT), "its own government's passed by");
    }

    #[test]
    fn a_seeking_sub_munition_no_ship_fired_seeks_past_the_given_target() {
        // A shot no ship fired hits only the ship it is aimed at
        // (@0x44a71-0x44a88), but the seek asks with no target of its own
        // (@0x3f3c3): it takes the nearer ship, not the given one.
        let unowned = Shot {
            source: Source::Other,
            ..parent(-1, 1, 0, 0, 0x0010)
        };
        let targets = [ship(HIT, 100.0, 450.0), ship(NEAR, 100.0, 60.0)];
        let subs = released(
            &unowned,
            &sub(-1),
            (Some(HIT), false),
            &among(&targets),
            &mut Draws::of(&[]),
        );
        assert_eq!(subs[0].target, Some(NEAR));
    }

    #[test]
    fn sub_munitions_carry_their_parents_side() {
        for source in [
            Source::Other,
            Source::Ship(Ties {
                leader: Some(ShipRef::Player),
                govt: None,
                booty: Some(0x0100),
                mining: true,
            }),
        ] {
            let parent = Shot {
                source,
                ..parent(-1, 2, 0, 0, 0)
            };
            let subs = released(
                &parent,
                &sub(-1),
                (Some(HIT), false),
                &among(&[]),
                &mut Draws::of(&[]),
            );
            assert!(subs.iter().all(|shot| shot.source == source), "{source:?}");
        }
    }

    #[test]
    fn a_guided_sub_munition_with_no_target_can_hit_nothing_and_takes_the_given_target() {
        let targets = [ship(NEAR, 100.0, -50.0), ship(HIT, 100.0, 450.0)];
        let seeking = parent(4, 1, 0, 0, 0x0010);
        let subs = released(
            &seeking,
            &sub(1),
            (Some(HIT), false),
            &among(&targets),
            &mut Draws::of(&[]),
        );
        assert_eq!(subs[0].target, Some(HIT));
        assert_eq!(subs[0].heading, 180.0);
    }

    #[test]
    fn a_sub_munition_carries_no_miss_roll_whatever_its_parents() {
        let turret = Shot {
            miss_roll: Some(100),
            ..parent(4, 2, 0, 0, 0)
        };
        let subs = released(
            &turret,
            &sub(4),
            (None, false),
            &among(&[]),
            &mut Draws::of(&[]),
        );
        assert_eq!(subs.len(), 2);
        assert!(subs.iter().all(|shot| shot.miss_roll.is_none()), "{subs:?}");
    }

    #[test]
    fn a_seeking_sub_munition_of_a_turret_takes_the_nearest_fighter_class_ship() {
        let turret = Shot {
            miss_roll: Some(100),
            ..parent(4, 1, 0, 0, 0x0010)
        };
        let targets = [
            ship(HIT, 100.0, 450.0),
            Target {
                miss_class: 80,
                ..ship(NEAR, 100.0, 60.0)
            },
        ];
        let subs = released(
            &turret,
            &sub(4),
            (None, false),
            &among(&targets),
            &mut Draws::of(&[]),
        );
        assert_eq!(subs[0].target, Some(NEAR), "never missed, at -1");
    }

    #[test]
    fn a_point_defence_parent_passes_on_its_own_target() {
        let defence = Shot {
            target: Some(NEAR),
            ..parent(9, 1, 0, 0, 0)
        };
        let subs = released(
            &defence,
            &sub(-1),
            (Some(HIT), false),
            &among(&[]),
            &mut Draws::of(&[]),
        );
        assert_eq!(subs[0].target, Some(NEAR));
    }

    #[test]
    fn a_seeking_sub_munition_heads_at_the_tables_bearing() {
        // 135 degrees by float, 134 by the table.
        let targets = [ship(NEAR, 200.0, 150.0)];
        let subs = released(
            &parent(-1, 1, 0, 0, 0x0010),
            &sub(-1),
            (None, false),
            &among(&targets),
            &mut Draws::of(&[]),
        );
        assert_eq!(headings(&subs), [134.0]);
        assert_eq!(subs[0].velocity, trig::accel(134, 13.0));
    }

    #[test]
    fn a_sub_munition_flies_along_its_truncated_heading_wrapped_once() {
        let turned = Shot {
            heading: 359.5,
            ..parent(-1, 1, 5, 0, 0)
        };
        // θ − below(11) with a draw of 0: 359.5 + 5, taken round once.
        let subs = released(
            &turned,
            &sub(-1),
            (None, false),
            &among(&[]),
            &mut Draws::of(&[0]),
        );
        assert_eq!(headings(&subs), [4.5]);
        assert_eq!(subs[0].velocity, trig::accel(4, 13.0));
        let bomb = released(
            &turned,
            &sub(5),
            (None, false),
            &among(&[]),
            &mut Draws::of(&[0]),
        );
        assert_eq!(
            bomb[0].velocity,
            trig::accel(4, 13.0),
            "a bomb from rest too"
        );
    }

    #[test]
    fn a_sub_munition_takes_its_layer_by_its_own_guidance_and_its_firers_class() {
        let parent = parent(-1, 1, 0, 0, 0);
        let layer = |sub: &WeaponSpec, parent: &Shot, turret_over: bool| {
            release_over(parent, sub, turret_over)[0].layer
        };
        assert_eq!(layer(&sub(4), &parent, true), ShotLayer::OverShips);
        assert!(
            release_over(&parent, &sub(4), true)[0].turret_over,
            "carried on"
        );
        assert_eq!(layer(&sub(4), &parent, false), ShotLayer::Shots);
        assert_eq!(layer(&sub(1), &parent, true), ShotLayer::Homing);
        assert_eq!(layer(&sub(9), &parent, false), ShotLayer::PointDefence);
        assert_eq!(layer(&sub(-1), &parent, true), ShotLayer::Shots);
        let unowned = Shot {
            source: Source::Other,
            ..parent
        };
        assert_eq!(
            layer(&sub(4), &unowned, true),
            ShotLayer::Shots,
            "no ship's"
        );
    }

    #[test]
    fn a_sub_munition_of_an_unwrapped_bomb_takes_its_heading_round_once() {
        let unwrapped = Shot {
            heading: -1.5,
            ..parent(5, 1, 0, 0, 0)
        };
        let subs = released(
            &unwrapped,
            &sub(-1),
            (None, false),
            &among(&[]),
            &mut Draws::of(&[]),
        );
        assert_eq!(headings(&subs), [358.5]);
        assert_eq!(subs[0].velocity, trig::accel(358, 13.0));
    }

    #[test]
    fn rockets_keep_their_parents_velocity_and_others_start_from_rest() {
        let parent = parent(-1, 1, 0, 0, 0);
        assert_eq!(parent.velocity, Vec2::new(0.0, 5.0) + trig::accel(90, 5.0));
        let subs = released(
            &parent,
            &sub(6),
            (None, false),
            &among(&[]),
            &mut Draws::of(&[]),
        );
        assert_eq!(subs[0].velocity, parent.velocity);
        let unguided = released(
            &parent,
            &sub(-1),
            (None, false),
            &among(&[]),
            &mut Draws::of(&[]),
        );
        assert_eq!(unguided[0].velocity, trig::accel(90, 13.0));
    }

    #[test]
    fn a_bomb_sub_munition_leaves_at_its_speed_from_rest_whoever_fired_the_parent() {
        for firer in [FIRER, ShipRef::Player] {
            let parent = Shot {
                firer,
                fleet: firer,
                ..parent(-1, 1, 0, 0, 0)
            };
            let subs = released(
                &parent,
                &sub(5),
                (None, false),
                &among(&[]),
                &mut Draws::of(&[]),
            );
            let along = facing(90.0) * 13.0;
            assert!(
                (subs[0].velocity - along).length() < 1e-4,
                "{firer:?}: {subs:?}"
            );
        }
    }

    #[test]
    fn an_npcs_rocket_sub_munition_takes_its_parents_velocity_and_no_speed() {
        let parent = parent(-1, 1, 0, 0, 0);
        assert_eq!(parent.firer, FIRER, "an NPC's");
        let subs = released(
            &parent,
            &sub(6),
            (None, false),
            &among(&[]),
            &mut Draws::of(&[]),
        );
        assert_eq!(subs[0].velocity, parent.velocity);
    }

    #[test]
    fn a_starburst_halves_its_spread_in_whole_degrees() {
        let pair = released(
            &parent(-1, 2, -15, 0, 0),
            &sub(-1),
            (None, false),
            &among(&[]),
            &mut Draws::of(&[]),
        );
        assert_eq!(headings(&pair), [83.0, 98.0], "15 / 2 is 7");
        let three = released(
            &parent(-1, 3, -15, 0, 0),
            &sub(-1),
            (None, false),
            &among(&[]),
            &mut Draws::of(&[]),
        );
        assert_eq!(headings(&three), [75.0, 90.0, 105.0], "30 / 2 is 15");
    }

    #[test]
    fn sub_munitions_are_released_only_below_the_limit_and_only_with_a_count() {
        for (generation, limit, released) in [(0, 0, 1), (5, 0, 1), (0, 2, 1), (1, 2, 1), (2, 2, 0)]
        {
            let parent = Shot {
                generation,
                ..parent(-1, 1, 0, limit, 0)
            };
            let subs = release(&parent, &sub(-1), (None, false));
            assert_eq!(subs.len(), released, "{generation} of {limit}");
        }
        let none = release(&parent(-1, 0, 0, 0, 0), &sub(-1), (None, false));
        assert_eq!(none, []);
    }
}
