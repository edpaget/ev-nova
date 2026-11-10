//! The fight's tests of the shape a ship is struck by
//! ([`data_collision`](crate::combat::data_collision)): by its sprite, the
//! engine's reading, and by its data's collision disc, the other reading
//! of [`RuleKey::DataCollision`](crate::RuleKey::DataCollision), at each
//! place a ship is struck: a shot's contact (placed and swept), a
//! proximity fuse (point and swept), a beam (cone and segment), point
//! defence's and the aims' view of the ships, the lineup's rereading, and
//! a deadly stellar.
//!
//! The ships are sized like the sample's light freighter: a `shän` 26
//! wide and 40 long, and a collision radius of 20.

use super::collision_walk::fly;
use super::*;
use crate::catalog::HullRecord;
use crate::combat::contact::{ContactSprites, ShotContact};
use crate::combat::data_collision::CollisionShape;
use crate::combat::lineup::Lineup;
use crate::combat::projectile::FUSE_SPRITE_SHARE;
use crate::combat::stellar_hazards::{DeadlyDisc, DeadlySprite, StellarHazards};
use crate::rulebook::{RuleKey, Rulebook};
use crate::sprite_mask::{MaskRect, PlacedMask};
use crate::sprite_place::{Placement, radius_touch};
use nova_sim_catalog::fixture::hull;

/// The freighter's `shän` sizes.
const SHAN: (i16, i16) = (26, 40);

/// A hull like the sample's light freighter: its `shän` [`SHAN`], its
/// data radius 20.
fn freighter() -> HullSpec {
    HullSpec::new(&HullRecord {
        size: Some(SHAN.0),
        height: Some(SHAN.1),
        collision_radius: Some(20),
        ..hull(140)
    })
}

/// [`freighter`] without its data radius, as a stock ship of its size.
fn stock_sized() -> HullSpec {
    HullSpec {
        collision_radius: None,
        ..freighter()
    }
}

/// The freighter's data radius.
fn radius() -> f32 {
    freighter().collision_radius.expect("a data radius")
}

/// A fight whose ships are struck as `source` reads
/// [`RuleKey::DataCollision`](crate::RuleKey::DataCollision).
fn fight(source: RuleSource) -> Combat {
    let mut combat = Combat::default();
    combat.set_data_collision(source);
    combat
}

/// The readings of the key, the engine's first.
const READINGS: [RuleSource; 2] = [RuleSource::Engine, RuleSource::Bible];

/// A shot two pixels square, with no sheet read.
fn speck() -> WeaponRecord {
    WeaponRecord {
        frame: Some(crate::catalog::FrameSize {
            width: 2,
            height: 2,
        }),
        ..blaster()
    }
}

/// Whether A's shot of `record`, its last move from `from` to `to`,
/// strikes B at the centre of `hull` in the collision passes, the ships
/// struck as `source` reads, the shot touching by `contact` with no sheet
/// lent, `combat`'s other readings set by `set`.
fn struck_by(
    source: RuleSource,
    (record, from, to): (WeaponRecord, Vec2, Vec2),
    (hull, contact): (&HullSpec, ShotContact),
    set: impl FnOnce(&mut Combat),
) -> bool {
    let mut combat = fight(source);
    set(&mut combat);
    fly(&mut combat, (record, A, ShotLayer::Shots), (from, to), None);
    let mut ships = [Ship::at(1, -300.0, 300.0), Ship::at(2, 0.0, 0.0).of(*hull)];
    let (arsenal, govts) = (Arsenal::default(), Governments::default());
    let mut rocks = Rocks::of([]);
    let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
    let arena = Arena {
        pass: GamePass::Advance { double_time: false },
        arsenal: &arsenal,
        govts: &govts,
        rules: Rules::default(),
        field: Field::default(),
    };
    let sprites = ContactSprites {
        reading: contact,
        ..ContactSprites::default()
    };
    let scenery = Scenery {
        asteroids: Some(rocks.lend()),
        contact: Some(&sprites),
        ..Scenery::default()
    };
    combat.test_hits(&mut fighters, arena, scenery, &mut NeverFires);
    combat.take_strikes().iter().any(|strike| strike.ship == B)
}

/// [`struck_by`] the placed rects, the engine's contact.
fn struck(
    source: RuleSource,
    shot: (WeaponRecord, Vec2, Vec2),
    hull: &HullSpec,
    set: impl FnOnce(&mut Combat),
) -> bool {
    struck_by(source, shot, (hull, ShotContact::Placed), set)
}

/// Whether the stock placed rects touch: the freighter's rect at the
/// centre and a two-pixel shot's at `at` (`_SWRadiusCollision`, which a
/// ship 26 wide goes by).
fn stock_touch(at: Vec2) -> bool {
    let (width, height) = SHAN;
    let ship = Placement::Standard.rect(
        Vec2::ZERO,
        (width.unsigned_abs().into(), height.unsigned_abs().into()),
    );
    let shot = Placement::Shot.rect(at, (2, 2));
    ship.overlaps(shot) && radius_touch(ship, shot)
}

// A shot's contact, placed.

#[test]
fn a_placed_shot_inside_the_disc_but_off_the_sprite_strikes_only_by_the_disc() {
    let (from, to) = (Vec2::new(0.0, 60.0), Vec2::new(0.0, 17.0));
    assert!(!stock_touch(to), "off the sprite's radius touch");
    assert!(to.length() < radius() && from.length() > radius());
    let shot = (speck(), from, to);
    assert!(!struck(RuleSource::Engine, shot, &freighter(), |_| ()));
    assert!(struck(RuleSource::Bible, shot, &freighter(), |_| ()));
}

#[test]
fn a_placed_shot_on_the_sprite_but_outside_the_disc_strikes_only_by_the_sprite() {
    let (from, to) = (Vec2::new(5.0, -60.0), Vec2::new(5.0, -19.5));
    assert!(stock_touch(to), "within the sprite's radius touch");
    assert!(to.length() > radius(), "its nearest to the centre");
    let shot = (speck(), from, to);
    assert!(struck(RuleSource::Engine, shot, &freighter(), |_| ()));
    assert!(!struck(RuleSource::Bible, shot, &freighter(), |_| ()));
}

#[test]
fn a_ship_without_a_data_radius_is_struck_by_its_sprite_under_both_readings() {
    let inside = (speck(), Vec2::new(0.0, 60.0), Vec2::new(0.0, 17.0));
    let on = (speck(), Vec2::new(5.0, -60.0), Vec2::new(5.0, -19.5));
    for source in READINGS {
        assert!(
            !struck(source, inside, &stock_sized(), |_| ()),
            "{source:?}"
        );
        assert!(struck(source, on, &stock_sized(), |_| ()), "{source:?}");
    }
}

// A shot's contact, swept.

/// A shot falling from above to `y` along the centre line.
fn falling_to(y: f32) -> (WeaponRecord, Vec2, Vec2) {
    (speck(), Vec2::new(0.0, -60.0), Vec2::new(0.0, y))
}

#[test]
fn a_swept_shot_strikes_by_the_data_radius_or_the_hit_radius() {
    let (sized, plain) = (freighter(), stock_sized());
    let swept = |hull| (hull, ShotContact::Swept);
    let between = -f32::midpoint(radius(), sized.hit_radius);
    assert!(sized.hit_radius < -between && -between < radius());
    let shot = falling_to(between);
    assert!(!struck_by(RuleSource::Engine, shot, swept(&sized), |_| ()));
    assert!(struck_by(RuleSource::Bible, shot, swept(&sized), |_| ()));
    assert!(
        !struck_by(RuleSource::Bible, shot, swept(&plain), |_| ()),
        "no data radius: its hit radius"
    );
}

// The target a ship is.

#[test]
fn as_a_target_a_ship_is_its_disc_or_its_sprite() {
    let mut ship = Ship::at(2, 0.0, 0.0).of(freighter());
    let fighter = ship.fighter();
    let disc = fighter.as_target(CollisionShape::DataDisc);
    assert_eq!(
        (disc.radius, disc.sprite_x, disc.sheet),
        (radius(), 2.0 * radius(), true)
    );
    let sprite = fighter.as_target(CollisionShape::Sprite);
    assert_eq!(
        (sprite.radius, sprite.sprite_x, sprite.sheet),
        (freighter().hit_radius, f32::from(SHAN.0), true)
    );
    assert_eq!(
        Target {
            radius: sprite.radius,
            sprite_x: sprite.sprite_x,
            ..disc
        },
        sprite,
        "nothing else differs"
    );
    let mut plain = Ship::at(3, 0.0, 0.0).of(stock_sized());
    let plain = plain.fighter();
    assert_eq!(
        plain.as_target(CollisionShape::DataDisc),
        plain.as_target(CollisionShape::Sprite)
    );
}

#[test]
fn the_fights_targets_and_candidates_are_read_by_its_reading() {
    for (source, shape) in [
        (RuleSource::Engine, CollisionShape::Sprite),
        (RuleSource::Bible, CollisionShape::DataDisc),
    ] {
        let combat = fight(source);
        let mut ships = [Ship::at(2, 0.0, 0.0).of(freighter())];
        let fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
        let expected = fighters[0].as_target(shape);
        assert_eq!(combat.targets(&fighters), [expected], "{source:?}");
        let candidates = combat.candidates(&fighters);
        assert_eq!(candidates[0].target, expected, "{source:?}");
    }
}

#[test]
fn the_lineup_reads_its_ships_and_reads_them_again_by_its_reading() {
    for (source, shape) in [
        (RuleSource::Engine, CollisionShape::Sprite),
        (RuleSource::Bible, CollisionShape::DataDisc),
    ] {
        let combat = fight(source);
        let govts = Governments::default();
        let mut ships = [Ship::at(2, 0.0, 0.0).of(freighter())];
        let fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
        let expected = fighters[0].as_target(shape);
        let mut lineup = Lineup::new(&fighters, &[Vec2::ZERO], &govts, combat.hit_readings());
        assert_eq!(lineup.world, [expected], "{source:?}");
        assert_eq!(lineup.seen, [expected], "{source:?}");
        lineup.world[0].radius = -1.0;
        lineup.seen[0].radius = -1.0;
        lineup.refresh(&fighters, &[B], &NovaDisable);
        assert_eq!(lineup.world, [expected], "{source:?}: read again");
        assert_eq!(lineup.seen, [expected], "{source:?}: read again");
    }
}

// A proximity fuse.

/// A blaster of `ProxRadius` 20 two-pixel shots.
fn fused() -> WeaponRecord {
    WeaponRecord {
        prox_radius: 20,
        ..speck()
    }
}

#[test]
fn a_point_fuse_reaches_by_the_disc_s_diameter_for_the_sprite_s_width() {
    let prox = f64::from(fused().prox_radius);
    let reach = |width: f32| (f64::from(width) * FUSE_SPRITE_SHARE + prox).trunc();
    let (sprite, disc) = (reach(f32::from(SHAN.0)), reach(2.0 * radius()));
    assert!(sprite < disc);
    // Still, between the two reaches, and off the contact of both.
    #[allow(clippy::cast_possible_truncation)]
    let at = Vec2::new(0.0, -f64::midpoint(sprite, disc).trunc() as f32);
    assert!(f64::from(-at.y) > sprite && f64::from(-at.y) <= disc);
    assert!(!stock_touch(at) && -at.y > radius());
    let shot = (fused(), at, at);
    assert!(!struck(RuleSource::Engine, shot, &freighter(), |_| ()));
    assert!(struck(RuleSource::Bible, shot, &freighter(), |_| ()));
}

#[test]
fn a_swept_fuse_reaches_by_the_data_radius_or_the_hit_radius() {
    let swept = |combat: &mut Combat| combat.set_proximity_fuse(RuleSource::Bible);
    let prox = f32::from(fused().prox_radius);
    let (sprite, disc) = (freighter().hit_radius + prox, radius() + prox);
    // Its move ends between the two reaches.
    let end = -f32::midpoint(sprite, disc);
    let shot = (fused(), Vec2::new(0.0, -80.0), Vec2::new(0.0, end));
    assert!(!struck(RuleSource::Engine, shot, &freighter(), swept));
    assert!(struck(RuleSource::Bible, shot, &freighter(), swept));
}

// A beam.

/// Whether the laser of A at the centre, facing right, hits B at `at` of
/// `hull` at its first `_HandleBeams`, the ships struck as `source`
/// reads, the beam's other readings set by `set`.
fn beamed_at(source: RuleSource, at: Vec2, hull: &HullSpec, set: impl FnOnce(&mut Combat)) -> bool {
    let mut combat = fight(source);
    set(&mut combat);
    beamed(combat, 90.0, vec![Ship::at(2, at.x, at.y).of(*hull)]) == [B]
}

#[test]
fn a_beam_s_cone_reaches_a_ship_by_its_disc_s_diameter_for_its_width() {
    let length = f32::from(laser().beam_length);
    let reach = |width: f32| length + (beam::cone_radius(width) / 2) as f32;
    let (sprite, disc) = (reach(f32::from(SHAN.0)), reach(2.0 * radius()));
    assert!(sprite < disc);
    let at = Vec2::new(disc, 0.0);
    assert!(!beamed_at(RuleSource::Engine, at, &freighter(), |_| ()));
    assert!(beamed_at(RuleSource::Bible, at, &freighter(), |_| ()));
    assert!(
        !beamed_at(RuleSource::Bible, at, &stock_sized(), |_| ()),
        "no data radius: its sprite's width"
    );
    let near = Vec2::new(sprite, 0.0);
    for source in READINGS {
        assert!(beamed_at(source, near, &freighter(), |_| ()), "{source:?}");
    }
}

#[test]
fn a_beam_s_segment_meets_a_ship_by_its_data_radius_or_its_hit_radius() {
    let segment = |combat: &mut Combat| combat.set_beam_hit_test(RuleSource::Bible);
    let off = f32::midpoint(radius(), freighter().hit_radius);
    let at = Vec2::new(100.0, off);
    assert!(!beamed_at(RuleSource::Engine, at, &freighter(), segment));
    assert!(beamed_at(RuleSource::Bible, at, &freighter(), segment));
}

// A deadly stellar.

/// The stellar hazards of one deadly disc of radius 5 at the centre, with
/// `masks` lent, among `ships`, as `source` reads: the ships killed.
fn killed(source: RuleSource, ships: &mut [Ship], hazards: &StellarHazards) -> Vec<ShipRef> {
    let mut combat = fight(source);
    phase::run(
        &mut combat,
        ships,
        &NovaDisable,
        |combat, fighters, _, _| {
            combat.hazards(fighters, hazards);
        },
    );
    combat
        .take_stellar_kills()
        .iter()
        .map(|kill| kill.ship)
        .collect()
}

/// A deadly disc of `radius` at the centre, with no mask.
fn deadly_disc(radius: f32) -> DeadlySprite {
    DeadlySprite {
        disc: DeadlyDisc {
            at: Vec2::ZERO,
            radius,
        },
        mask: None,
    }
}

#[test]
fn a_deadly_stellar_s_disc_reaches_a_ship_by_its_data_radius_or_its_hit_radius() {
    let stellar = 5.0;
    let off = stellar + f32::midpoint(radius(), freighter().hit_radius);
    let hazards = StellarHazards {
        deadly: vec![deadly_disc(stellar)],
        ..StellarHazards::default()
    };
    let ship = || [Ship::at(2, 0.0, off).of(freighter())];
    assert_eq!(killed(RuleSource::Engine, &mut ship(), &hazards), []);
    assert_eq!(killed(RuleSource::Bible, &mut ship(), &hazards), [B]);
    let plain = || [Ship::at(2, 0.0, off).of(stock_sized())];
    assert_eq!(
        killed(RuleSource::Bible, &mut plain(), &hazards),
        [],
        "no data radius: its hit radius"
    );
}

/// A one-pixel mask at `top_left`.
fn speck_mask(top_left: (i32, i32)) -> PlacedMask {
    let sheet = nova_sim_base::fixture::rle::sheet(&[&["#"]]);
    PlacedMask {
        sheet: std::rc::Rc::new(sheet),
        frame: 0,
        rect: MaskRect::placed(top_left, (1, 1)),
    }
}

#[test]
fn a_ship_lending_a_mask_goes_by_its_disc_under_the_data_reading() {
    // The stellar's one pixel and the ship's lie apart, so their masks
    // miss; the discs overlap.
    let hazards = StellarHazards {
        deadly: vec![DeadlySprite {
            mask: Some(speck_mask((0, 0))),
            ..deadly_disc(5.0)
        }],
        ships: vec![(B, speck_mask((3, 3)))],
        ..StellarHazards::default()
    };
    let ship = || [Ship::at(2, 0.0, 0.0).of(freighter())];
    assert_eq!(
        killed(RuleSource::Engine, &mut ship(), &hazards),
        [],
        "the masks"
    );
    assert_eq!(
        killed(RuleSource::Bible, &mut ship(), &hazards),
        [B],
        "its disc"
    );
    let plain = || [Ship::at(2, 0.0, 0.0).of(stock_sized())];
    assert_eq!(
        killed(RuleSource::Bible, &mut plain(), &hazards),
        [],
        "no data radius: the masks"
    );
}

#[test]
fn the_hull_rules_set_the_hull_turn_and_the_collision_shape_from_the_rulebook() {
    for headings in READINGS {
        for collision in READINGS {
            let rules = Rulebook::default()
                .with_override(RuleKey::ContinuousHeadings, headings)
                .with_override(RuleKey::DataCollision, collision);
            let mut set = Combat::default();
            set.set_hull_rules(&rules);
            let mut each = Combat::default();
            each.set_continuous_headings(headings);
            each.set_data_collision(collision);
            assert_eq!(set, each, "{headings:?} {collision:?}");
        }
    }
}

#[test]
fn the_data_collision_key_sets_the_fight_s_reading_and_its_engine_reading_is_the_default() {
    assert_eq!(fight(RuleSource::Engine), Combat::default());
    assert_ne!(fight(RuleSource::Bible), Combat::default());
}
