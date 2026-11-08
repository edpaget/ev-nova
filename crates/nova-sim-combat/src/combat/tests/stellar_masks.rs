//! A planet-type shot's contact with a destroyable stellar in the
//! proximity pass by the two sprites' masks, the shot first (see the
//! `stellar_contact` module): `_SWRLECollision(shot, stellar)` @0x372a8,
//! the hit at the shot's position, and the swept disc the other reading
//! and the fallback keep.

use std::rc::Rc;

use super::collision_walk::fly;
use super::*;
use crate::catalog::StellarId;
use crate::combat::contact::{ContactSprites, ShotContact, StellarSprite};
use crate::combat::stellar::{StellarStrike, StellarTarget};
use crate::player_wrap::{PlayerWrap, WRAP_EDGE, WRAP_SHIFT};
use crate::sprite_mask::{MaskWalk, RleSheet};
use nova_sim_base::fixture::rle::sheet;

/// The weapon `Graphic` the shots here show.
const GRAPHIC: i16 = 5;

/// The stellar struck here.
const STELLAR: StellarId = StellarId(128);

/// A planet-type gun showing [`GRAPHIC`], with no fuse.
fn gun() -> WeaponRecord {
    WeaponRecord {
        graphic: GRAPHIC,
        ..planet_gun()
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

/// A 40-square stellar frame drawn only in its bottom-right quarter.
fn quartered() -> Rc<RleSheet> {
    square(40, |x, y| x >= 20 && y >= 20)
}

/// `sheet` showing `frame` for a stellar at `at`, placed at its size.
fn sprite(sheet: Rc<RleSheet>, frame: u16, at: Vec2) -> StellarSprite {
    let size = (sheet.width, sheet.height);
    StellarSprite {
        sheet,
        frame,
        at,
        size,
        placed_as: size,
    }
}

/// The shot's sheet `shot` for [`GRAPHIC`], and each of `stellars`'
/// sprites, by `reading`.
fn lending(
    shot: Option<Rc<RleSheet>>,
    stellars: Vec<(StellarId, StellarSprite)>,
    reading: ShotContact,
) -> ContactSprites {
    let mut sprites = ContactSprites {
        stellars,
        reading,
        ..ContactSprites::default()
    };
    if let Some(sheet) = shot {
        sprites.shots.insert(GRAPHIC, sheet);
    }
    sprites
}

/// A 2-square solid shot sheet, and [`STELLAR`]'s sprite of [`quartered`]
/// at the centre, by `reading`.
fn quarter_lent(reading: ShotContact) -> ContactSprites {
    let stellar = sprite(quartered(), 0, Vec2::ZERO);
    lending(
        Some(square(2, |_, _| true)),
        vec![(STELLAR, stellar)],
        reading,
    )
}

/// Stellar `id` at `at`, its disc 30 across, standing at `strength`.
fn target(id: StellarId, at: Vec2, strength: i64) -> StellarTarget {
    StellarTarget {
        stellar: id,
        position: at,
        radius: 30.0,
        strength,
    }
}

/// What one proximity pass made of `stellars`: the strikes, where each
/// hit exploded, and whether a shot is still in flight.
struct Pass {
    strikes: Vec<StellarStrike>,
    at: Vec<Vec2>,
    flying: bool,
}

/// The collision passes of `combat` among `stellars`, `sprites` lent.
fn passes(combat: &mut Combat, stellars: &mut [StellarTarget], sprites: &ContactSprites) -> Pass {
    let (arsenal, govts) = (Arsenal::default(), Governments::default());
    let arena = Arena {
        pass: GamePass::Advance { double_time: false },
        arsenal: &arsenal,
        govts: &govts,
        rules: Rules::default(),
        field: Field::default(),
    };
    let scenery = Scenery {
        stellars,
        contact: Some(sprites),
        ..Scenery::default()
    };
    combat.test_hits(&mut [], arena, scenery, &mut NeverFires);
    let at = combat
        .take_events()
        .iter()
        .filter_map(|event| match *event {
            CombatEvent::Exploded { at, .. } => Some(at),
            _ => None,
        })
        .collect();
    Pass {
        strikes: combat.take_stellar_strikes(),
        at,
        flying: !combat.shots().is_empty(),
    }
}

/// A's shot of `record`, its last move from `from` to `to`, among
/// `stellars`, `sprites` lent.
fn shot_into(
    record: WeaponRecord,
    (from, to): (Vec2, Vec2),
    stellars: &mut [StellarTarget],
    sprites: &ContactSprites,
) -> Pass {
    let mut combat = Combat::default();
    fly(&mut combat, (record, A, ShotLayer::Shots), (from, to), None);
    passes(&mut combat, stellars, sprites)
}

/// Into the drawn quarter from well above it.
const INTO: (Vec2, Vec2) = (Vec2::new(9.0, -60.0), Vec2::new(9.0, 9.0));

/// Into the clear quarter, within the disc, from well above it.
const CORNER: (Vec2, Vec2) = (Vec2::new(-9.0, -60.0), Vec2::new(-9.0, -9.0));

/// Where [`INTO`]'s move entered the disc.
fn into_the_disc() -> Vec2 {
    Vec2::new(9.0, -(30.0_f32 * 30.0 - 81.0).sqrt())
}

fn near(a: Vec2, b: Vec2) -> bool {
    (a - b).length() < 1e-3
}

#[test]
fn a_shot_over_a_stellars_pixels_strikes_it_where_the_shot_is() {
    let sprites = quarter_lent(ShotContact::Placed);
    let mut stellars = [target(STELLAR, Vec2::ZERO, 100)];
    let pass = shot_into(gun(), INTO, &mut stellars, &sprites);
    assert_eq!(
        pass.strikes,
        [StellarStrike {
            stellar: STELLAR,
            by: Some(A),
            damage: 14,
            destroyed: false,
        }],
        "4 mass and 10 energy"
    );
    assert_eq!(stellars[0].strength, 86);
    assert_eq!(
        pass.at,
        [Vec2::new(9.0, 9.0)],
        "not where it entered the disc"
    );
    assert!(!pass.flying, "spent");
}

#[test]
fn a_shot_over_a_stellars_clear_pixels_misses_though_within_its_disc() {
    let sprites = quarter_lent(ShotContact::Placed);
    let mut stellars = [target(STELLAR, Vec2::ZERO, 100)];
    let pass = shot_into(gun(), CORNER, &mut stellars, &sprites);
    assert_eq!(pass.strikes, []);
    assert_eq!(stellars[0].strength, 100);
    assert!(pass.flying, "it flies on");
}

/// Whether A's shot of `shot`'s one row, at (0, 1), strikes a stellar of
/// `stellar`'s one row at the centre, the two rects the same, by `walk`.
fn struck_by_rows(shot: &str, stellar: &str, walk: MaskWalk) -> bool {
    let stellar = sprite(Rc::new(sheet(&[&[stellar]])), 0, Vec2::ZERO);
    let sprites = ContactSprites {
        walk,
        ..lending(
            Some(Rc::new(sheet(&[&[shot]]))),
            vec![(STELLAR, stellar)],
            ShotContact::Placed,
        )
    };
    let mut stellars = [target(STELLAR, Vec2::ZERO, 100)];
    let at = Vec2::new(0.0, 1.0);
    !shot_into(gun(), (at, at), &mut stellars, &sprites)
        .strikes
        .is_empty()
}

#[test]
fn by_the_original_walk_the_shots_pixel_touches_the_stellars_just_right_of_it() {
    assert!(
        struck_by_rows("#.", ".#", MaskWalk::Original),
        "the shot first"
    );
    assert!(
        !struck_by_rows(".#", "#.", MaskWalk::Original),
        "not the mirror"
    );
    assert!(!struck_by_rows("#.", ".#", MaskWalk::Drawn));
    assert!(!struck_by_rows(".#", "#.", MaskWalk::Drawn));
}

#[test]
fn a_shot_that_crosses_a_stellar_in_one_move_misses_by_the_masks_and_strikes_by_the_disc() {
    let through = (Vec2::new(9.0, -60.0), Vec2::new(9.0, 60.0));
    let mut stellars = [target(STELLAR, Vec2::ZERO, 100)];
    let placed = quarter_lent(ShotContact::Placed);
    assert_eq!(
        shot_into(gun(), through, &mut stellars, &placed).strikes,
        []
    );
    let swept = quarter_lent(ShotContact::Swept);
    let pass = shot_into(gun(), through, &mut stellars, &swept);
    assert_eq!(pass.strikes.len(), 1);
    assert!(near(pass.at[0], into_the_disc()), "{:?}", pass.at);
}

#[test]
fn the_first_standing_in_the_list_whose_mask_touches_is_struck() {
    let other = StellarId(129);
    let mut sprites = quarter_lent(ShotContact::Placed);
    sprites
        .stellars
        .push((other, sprite(quartered(), 0, Vec2::ZERO)));
    let struck = |first: i64| {
        let mut stellars = [
            target(other, Vec2::ZERO, first),
            target(STELLAR, Vec2::ZERO, 100),
        ];
        let pass = shot_into(gun(), INTO, &mut stellars, &sprites);
        pass.strikes.iter().map(|s| s.stellar).collect::<Vec<_>>()
    };
    assert_eq!(struck(100), [other], "the first in the list");
    assert_eq!(struck(-1), [STELLAR], "the first shot down is passed over");
}

#[test]
fn a_shot_point_defence_ended_is_tested_by_its_mask_where_it_is() {
    let sprites = quarter_lent(ShotContact::Placed);
    let struck = |at: Vec2| {
        let mut combat = Combat::default();
        fly(&mut combat, (gun(), A, ShotLayer::Shots), (at, at), None);
        let ended = combat.shots.remove(0);
        combat.hide(ended);
        let mut stellars = [target(STELLAR, Vec2::ZERO, 100)];
        let pass = passes(&mut combat, &mut stellars, &sprites);
        (pass.strikes.len(), pass.at)
    };
    let drawn = Vec2::new(9.0, 9.0);
    assert_eq!(struck(drawn), (1, vec![drawn]));
    assert_eq!(
        struck(Vec2::new(-9.0, -9.0)),
        (0, vec![]),
        "its clear quarter"
    );
}

#[test]
fn on_a_wraps_pass_a_stellar_is_met_by_its_mask_where_its_sprite_stood() {
    let stood = Vec2::new(WRAP_EDGE + 10.0, 0.0);
    let shift = Vec2::new(-WRAP_SHIFT, 0.0);
    let before = Vec2::new(stood.x - 100.0, 0.0);
    let struck = |stellar_at: Vec2| {
        let mut combat = Combat::default();
        fly(
            &mut combat,
            (gun(), A, ShotLayer::Shots),
            (before, before),
            None,
        );
        combat.carry(&PlayerWrap::of(stood).expect("past the edge"), Vec::new());
        let solid = sprite(square(4, |_, _| true), 0, stellar_at);
        let sprites = lending(
            Some(square(2, |_, _| true)),
            vec![(STELLAR, solid)],
            ShotContact::Placed,
        );
        let mut stellars = [StellarTarget {
            radius: 2.0,
            ..target(STELLAR, stellar_at, 100)
        }];
        passes(&mut combat, &mut stellars, &sprites).strikes.len()
    };
    assert_eq!(struck(before), 1, "where the two sprites stood");
    assert_eq!(struck(before + shift), 0, "not in the world");
}

// The disc decides.

#[test]
fn a_stellar_lent_no_sprite_is_met_by_its_disc() {
    let sprites = lending(
        Some(square(2, |_, _| true)),
        Vec::new(),
        ShotContact::Placed,
    );
    let mut stellars = [target(STELLAR, Vec2::ZERO, 100)];
    let pass = shot_into(gun(), INTO, &mut stellars, &sprites);
    assert_eq!(pass.strikes.len(), 1);
    assert!(near(pass.at[0], into_the_disc()), "{:?}", pass.at);
    let pass = shot_into(gun(), CORNER, &mut stellars, &sprites);
    assert_eq!(pass.strikes.len(), 1, "over its clear quarter");
}

#[test]
fn a_shot_whose_sheet_was_not_read_meets_a_stellar_by_its_disc() {
    let stellar = sprite(quartered(), 0, Vec2::ZERO);
    let sprites = lending(None, vec![(STELLAR, stellar)], ShotContact::Placed);
    let mut stellars = [target(STELLAR, Vec2::ZERO, 100)];
    let pass = shot_into(gun(), CORNER, &mut stellars, &sprites);
    assert_eq!(pass.strikes.len(), 1);
    assert!(
        pass.at[0].y < -9.0,
        "where it entered the disc: {:?}",
        pass.at
    );
}

#[test]
fn a_stellar_whose_sheet_has_no_frame_meets_a_shot_by_its_disc() {
    // A frame past the frames reads modulo them; only a sheet of none
    // leaves `_SWRLECollision` no frame to read.
    let none: &[&[&str]] = &[];
    let empty = StellarSprite {
        size: (40, 40),
        placed_as: (40, 40),
        ..sprite(Rc::new(sheet(none)), 0, Vec2::ZERO)
    };
    let sprites = lending(
        Some(square(2, |_, _| true)),
        vec![(STELLAR, empty)],
        ShotContact::Placed,
    );
    let mut stellars = [target(STELLAR, Vec2::ZERO, 100)];
    let pass = shot_into(gun(), CORNER, &mut stellars, &sprites);
    assert_eq!(pass.strikes.len(), 1);
    assert!(
        pass.at[0].y < -9.0,
        "where it entered the disc: {:?}",
        pass.at
    );
}

#[test]
fn by_the_other_reading_a_stellar_is_its_disc_though_both_masks_are_lent() {
    let sprites = quarter_lent(ShotContact::Swept);
    let mut stellars = [target(STELLAR, Vec2::ZERO, 100)];
    let pass = shot_into(gun(), CORNER, &mut stellars, &sprites);
    assert_eq!(pass.strikes.len(), 1, "over its clear quarter");
    assert!(
        pass.at[0].y < -9.0,
        "where it entered the disc: {:?}",
        pass.at
    );
}

#[test]
fn a_shot_of_another_weapon_passes_over_a_stellars_pixels() {
    let sprites = quarter_lent(ShotContact::Placed);
    let mut stellars = [target(STELLAR, Vec2::ZERO, 100)];
    let ordinary = WeaponRecord { flags2: 0, ..gun() };
    assert_eq!(
        shot_into(ordinary, INTO, &mut stellars, &sprites).strikes,
        []
    );
}
