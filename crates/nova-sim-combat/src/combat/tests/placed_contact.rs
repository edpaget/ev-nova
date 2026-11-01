//! A shot's contact in the collision passes by the two sprites' placed
//! rects (see the `contact` module): `_ShipCollisionHandler`'s radius and
//! mask tests and `_AsteroidCollisionHandler`'s mask test, each after the
//! strict rect pre-test, the hit at the shot's position; and the swept
//! test the other reading and the fallback keep.

use std::rc::Rc;

use super::collision_walk::fly;
use super::*;
use crate::asteroid::KINDS;
use crate::combat::contact::{ContactSprites, ShipSprite, ShotContact};
use crate::player_wrap::{PlayerWrap, WRAP_EDGE, WRAP_SHIFT};
use crate::sprite_mask::{MaskWalk, RleSheet};
use nova_sim_base::fixture::rle::sheet;

/// The weapon `Graphic` the shots here show.
const GRAPHIC: i16 = 5;

/// A blaster that explodes where it hits, showing [`GRAPHIC`].
fn gun() -> WeaponRecord {
    WeaponRecord {
        graphic: GRAPHIC,
        ..rock_gun()
    }
}

/// [`gun`] whose shots are `width` by `height` with no sheet read.
fn sized(width: u32, height: u32) -> WeaponRecord {
    WeaponRecord {
        frame: Some(crate::catalog::FrameSize { width, height }),
        ..gun()
    }
}

/// A frame `side` square drawn where `drawn` says.
fn square(side: usize, drawn: impl Fn(usize, usize) -> bool) -> Rc<RleSheet> {
    let rows: Vec<String> = (0..side)
        .map(|y| {
            (0..side)
                .map(|x| if drawn(x, y) { '#' } else { '.' })
                .collect()
        })
        .collect();
    let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
    Rc::new(sheet(&[&rows]))
}

/// A 40-square ship frame drawn only in its bottom-right quarter.
fn quartered() -> Rc<RleSheet> {
    square(40, |x, y| x >= 20 && y >= 20)
}

/// B's sprite of `sheet`, and a 2-square solid shot sheet for
/// [`GRAPHIC`], by `reading`.
fn lending(ship: Option<Rc<RleSheet>>, reading: ShotContact) -> ContactSprites {
    let mut sprites = ContactSprites {
        reading,
        ..ContactSprites::default()
    };
    if let Some(sheet) = ship {
        sprites.ships.push((B, ShipSprite { sheet, frame: 0 }));
    }
    sprites.shots.insert(GRAPHIC, square(2, |_, _| true));
    sprites
}

/// B, at the centre, of `hull`.
fn b_of(hull: &HullSpec) -> Ship {
    Ship::at(2, 0.0, 0.0).of(*hull)
}

/// A hull 0.33 × 40 across, as a 40-wide `shän` gives it.
fn forty() -> HullSpec {
    HullSpec {
        hit_radius: 40.0 * 0.33,
        ..HullSpec::default()
    }
}

/// The collision passes among `ships` and `rocks`, `sprites` lent: the
/// ships struck and where each hit exploded.
fn passes(
    combat: &mut Combat,
    ships: &mut [Ship],
    rocks: &mut Rocks,
    sprites: &ContactSprites,
) -> (Vec<ShipRef>, Vec<Vec2>) {
    let (arsenal, govts) = (Arsenal::default(), Governments::default());
    let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
    let arena = Arena {
        pass: GamePass::Advance { double_time: false },
        arsenal: &arsenal,
        govts: &govts,
        rules: Rules::default(),
        field: Field::default(),
    };
    let scenery = Scenery {
        asteroids: Some(rocks.lend()),
        contact: Some(sprites),
        ..Scenery::default()
    };
    combat.test_hits(&mut fighters, arena, scenery, &mut NeverFires);
    let struck = combat.take_strikes().iter().map(|s| s.ship).collect();
    let at = combat
        .take_events()
        .iter()
        .filter_map(|event| match *event {
            CombatEvent::Exploded { at, .. } => Some(at),
            _ => None,
        })
        .collect();
    (struck, at)
}

/// A's shot of `record` whose last move ran from `from` to `to`, met by
/// `ships` with `sprites` lent.
fn shot_among(
    record: WeaponRecord,
    (from, to): (Vec2, Vec2),
    ships: &mut [Ship],
    sprites: &ContactSprites,
) -> (Vec<ShipRef>, Vec<Vec2>) {
    let mut combat = Combat::default();
    fly(&mut combat, (record, A, ShotLayer::Shots), (from, to), None);
    passes(&mut combat, ships, &mut Rocks::of([]), sprites)
}

#[test]
fn a_shot_over_a_wide_ships_clear_corner_misses_though_within_its_old_disc() {
    let sprites = lending(Some(quartered()), ShotContact::Placed);
    let corner = (Vec2::new(-9.0, -30.0), Vec2::new(-9.0, -9.0));
    let (struck, _) = shot_among(gun(), corner, &mut [b_of(&forty())], &sprites);
    assert_eq!(struck, [], "its pixels miss the clear quarter");
    let swept = lending(Some(quartered()), ShotContact::Swept);
    let (struck, _) = shot_among(gun(), corner, &mut [b_of(&forty())], &swept);
    assert_eq!(struck, [B], "the disc, 13.2 across, takes in (-9, -9)");
}

#[test]
fn a_shot_over_a_wide_ships_pixels_hits_where_the_shot_is() {
    let sprites = lending(Some(quartered()), ShotContact::Placed);
    let into = (Vec2::new(9.0, -30.0), Vec2::new(9.0, 9.0));
    let (struck, at) = shot_among(gun(), into, &mut [b_of(&forty())], &sprites);
    assert_eq!((struck, at), (vec![B], vec![Vec2::new(9.0, 9.0)]));
    // By the other reading, where the move entered the disc.
    let swept = lending(Some(quartered()), ShotContact::Swept);
    let (_, at) = shot_among(gun(), into, &mut [b_of(&forty())], &swept);
    assert!(at[0].y < -8.0, "{at:?}");
}

#[test]
fn a_shot_that_crosses_a_ship_in_one_move_misses_by_the_engine_and_hits_by_the_other() {
    let solid = square(40, |_, _| true);
    let through = (Vec2::new(0.0, -60.0), Vec2::new(0.0, 60.0));
    let placed = lending(Some(Rc::clone(&solid)), ShotContact::Placed);
    let (struck, _) = shot_among(gun(), through, &mut [b_of(&forty())], &placed);
    assert_eq!(struck, []);
    let swept = lending(Some(solid), ShotContact::Swept);
    let (struck, at) = shot_among(gun(), through, &mut [b_of(&forty())], &swept);
    assert_eq!(struck, [B]);
    assert!((at[0] - Vec2::new(0.0, -13.2)).length() < 1e-3, "{at:?}");
}

#[test]
fn a_shot_over_a_ships_run_misses_by_the_original_walk_and_hits_by_the_other() {
    let runs: Vec<String> = (0..40).map(|_| "r".repeat(40)).collect();
    let rows: Vec<&str> = runs.iter().map(String::as_str).collect();
    let run = Rc::new(sheet(&[&rows]));
    let on = (Vec2::new(9.0, -30.0), Vec2::new(9.0, 9.0));
    for (walk, struck) in [(MaskWalk::Original, vec![]), (MaskWalk::Drawn, vec![B])] {
        let sprites = ContactSprites {
            walk,
            ..lending(Some(Rc::clone(&run)), ShotContact::Placed)
        };
        let (hit, _) = shot_among(gun(), on, &mut [b_of(&forty())], &sprites);
        assert_eq!(hit, struck, "{walk:?}");
    }
}

/// A hull 30 square by its `shän`, with no sheet read.
fn thirty() -> HullSpec {
    HullSpec {
        sprite_size: Some(30.0),
        sprite_height: Some(30.0),
        ..HullSpec::default()
    }
}

#[test]
fn a_small_ship_goes_by_radius_at_its_shan_sizes_with_no_sheet() {
    let none = ContactSprites::default();
    let hit = |to: Vec2| {
        let (struck, at) = shot_among(sized(4, 4), (to, to), &mut [b_of(&thirty())], &none);
        (!struck.is_empty()).then(|| at[0])
    };
    // Rects (-15, 15) and the shot's, 4 across: centres 17 or nearer.
    assert_eq!(hit(Vec2::new(14.0, 0.0)), Some(Vec2::new(14.0, 0.0)));
    assert_eq!(hit(Vec2::new(18.0, 0.0)), None, "the rects apart");
    // Within both rects off both axes, but 18.4 apart.
    assert_eq!(hit(Vec2::new(13.0, 13.0)), None);
}

#[test]
fn a_tall_shot_is_tested_at_its_width_only_rect() {
    // 4 by 12, 14 above: its centre by radius is its point, 14 from the
    // ship's, within 17; the standard rule's would sit at 18, past it.
    let none = ContactSprites::default();
    let at = Vec2::new(0.0, -14.0);
    let (struck, _) = shot_among(sized(4, 12), (at, at), &mut [b_of(&thirty())], &none);
    assert_eq!(struck, [B]);
    // 19 below: its rect starts below the ship's, where the standard
    // rule's would overlap it within reach.
    let below = Vec2::new(0.0, 19.0);
    let (struck, _) = shot_among(sized(4, 12), (below, below), &mut [b_of(&thirty())], &none);
    assert_eq!(struck, []);
}

#[test]
fn with_no_size_or_sheet_contact_keeps_the_swept_radius() {
    let none = ContactSprites::default();
    let into = (Vec2::new(-9.0, -30.0), Vec2::new(-9.0, -9.0));
    let (struck, at) = shot_among(gun(), into, &mut [b_of(&forty())], &none);
    assert_eq!(struck, [B]);
    assert!(at[0].y < -9.0, "where it entered the disc: {at:?}");
    // A wide ship's sizes but no sheet: the mask test falls back too.
    let wide = HullSpec {
        sprite_size: Some(40.0),
        sprite_height: Some(40.0),
        ..forty()
    };
    let shot_sheet = lending(None, ShotContact::Placed);
    let (struck, _) = shot_among(gun(), into, &mut [b_of(&wide)], &shot_sheet);
    assert_eq!(struck, [B]);
}

#[test]
fn on_a_wraps_pass_a_shot_left_behind_meets_by_its_rect_a_delta_back() {
    let stood = Vec2::new(WRAP_EDGE + 10.0, 0.0);
    let shift = Vec2::new(-WRAP_SHIFT, 0.0);
    let off = |x: f32| Vec2::new(stood.x + x, 0.0);
    let (shot, ship) = (off(-WRAP_EDGE - 5.0), off(-WRAP_EDGE + 5.0) + shift);
    let mut combat = Combat::default();
    fly(
        &mut combat,
        (sized(4, 4), A, ShotLayer::Shots),
        (shot, shot),
        None,
    );
    combat.carry(&PlayerWrap::of(stood).expect("past the edge"), Vec::new());
    let mut ships = [Ship::at(2, ship.x, ship.y).of(thirty())];
    let (struck, at) = passes(
        &mut combat,
        &mut ships,
        &mut Rocks::of([]),
        &ContactSprites::default(),
    );
    assert_eq!(struck, [B], "their sprites 10 apart");
    assert_eq!(at, [shot], "the hit where the shot is");
}

/// Asteroid kind 0's sheet `sheet`, and the shot's.
fn rocky(rock: Rc<RleSheet>, reading: ShotContact) -> ContactSprites {
    let mut sprites = lending(None, reading);
    let mut asteroids: [Option<Rc<RleSheet>>; KINDS] = std::array::from_fn(|_| None);
    asteroids[0] = Some(rock);
    sprites.asteroids = asteroids;
    sprites
}

/// Whether A's shot of `record`, its last move from `from` to `to`, wears
/// an asteroid at the centre, `sprites` lent.
fn rock_hit(record: WeaponRecord, (from, to): (Vec2, Vec2), sprites: &ContactSprites) -> bool {
    let mut combat = Combat::default();
    fly(&mut combat, (record, A, ShotLayer::Shots), (from, to), None);
    let mut rocks = Rocks::of([rock(0, 0.0, 0.0)]);
    passes(&mut combat, &mut [], &mut rocks, sprites);
    rocks.at(0).strength < 10
}

#[test]
fn a_shot_touches_an_asteroid_by_its_frames_pixels() {
    let sprites = rocky(quartered(), ShotContact::Placed);
    let corner = (Vec2::new(-5.0, -5.0), Vec2::new(-5.0, -5.0));
    assert!(!rock_hit(gun(), corner, &sprites), "the clear quarter");
    let pixels = (Vec2::new(9.0, 9.0), Vec2::new(9.0, 9.0));
    assert!(rock_hit(gun(), pixels, &sprites));
    let seeker = WeaponRecord {
        seeker: crate::combat::weapon::IGNORES_ASTEROIDS,
        ..gun()
    };
    assert!(!rock_hit(seeker, pixels, &sprites), "Seeker 0x0001");
    // With no sheet, its disc, 10 in radius.
    let none = ContactSprites::default();
    assert!(rock_hit(gun(), corner, &none));
    // By the other reading too.
    assert!(rock_hit(
        gun(),
        corner,
        &rocky(quartered(), ShotContact::Swept)
    ));
}

#[test]
fn an_asteroid_touches_by_the_frame_it_shows() {
    // Frame 1 solid, frame 0 clear.
    let two = Rc::new(sheet(&[&["..", ".."], &["##", "##"]]));
    let sprites = rocky(two, ShotContact::Placed);
    let on = |frame: f32| {
        let mut combat = Combat::default();
        let at = Vec2::ZERO;
        fly(&mut combat, (gun(), A, ShotLayer::Shots), (at, at), None);
        let (slot, mut asteroid) = rock(0, 0.0, 0.0);
        asteroid.frame = frame;
        let mut rocks = Rocks::of([(slot, asteroid)]);
        passes(&mut combat, &mut [], &mut rocks, &sprites);
        rocks.at(0).strength < 10
    };
    assert!(!on(0.0));
    assert!(on(1.7), "its whole part, 1");
}
