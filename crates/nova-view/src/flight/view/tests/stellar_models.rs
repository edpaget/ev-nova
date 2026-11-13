//! Stellars drawn in 3D (rdm:phase/engine-3d/phase-10-sample-in-3d): in
//! the stellars' pass a stellar with a model in the flight's table
//! ([`FlightView::with_world_models`]) draws it unturned at its stand-in's
//! centre, scaled to reach as far as the stand-in, and one with a planet
//! draws the planet at the stand-in disc's radius and the flight clock; any
//! other draws its stand-in, its name and order as before. On
//! [`FakeFlight`], with a hand-built table. The system view draws no
//! models: `system::scene`'s own tests pin its drawing, unchanged.

use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Duration;

use nova_sim_frame::landing::UNKNOWN_STELLAR_RADIUS;

use super::faked_world::{world, world_list};
use super::*;
use crate::flight::world_models::{StellarModel, WorldModels};
use crate::model::{ModelKey, ModelPose, Orientation};
use crate::outline::{Outline, OutlineTriangle, StellarStandIn};
use crate::planet::{PlanetKey, PlanetPose};
use crate::system::catalog::StellarContents;

const HUB: ModelKey = ModelKey(7);
const GIANT: PlanetKey = PlanetKey(3);
/// The hub's footprint's reach, in its own units.
const REACH: f32 = 4.0;
/// The station, the gas giant and a moon, none with a sheet.
const STATION: i16 = 140;
const PLANET: i16 = 141;
const MOON: i16 = 142;

/// `id`, at `at`, with no sheet, standing in as `stand_in`.
fn sheetless(id: i16, name: &str, at: (i16, i16), stand_in: StellarStandIn) -> StellarContents {
    StellarContents {
        sprite: Err(format!("no spïn for {id}")),
        stand_in: Some(stand_in),
        ..crate::flight::fake_catalog::stellar(id, name, at, 1)
    }
}

fn hub_from_above() -> Outline {
    Outline {
        triangles: vec![OutlineTriangle {
            corners: [at(0.0, -4.0), at(3.0, 2.0), at(-3.0, 2.0)],
            color: Color::rgba(200, 200, 200, 255),
        }],
    }
}

/// Sol with the station, the planet and the moon besides Earth and Moon.
fn catalog_3d() -> FakeCatalog {
    FakeCatalog {
        sol_more: vec![
            sheetless(
                STATION,
                "Exchange",
                (200, 100),
                StellarStandIn::Outline(hub_from_above()),
            ),
            sheetless(
                PLANET,
                "Kestrel Reach",
                (-250, 50),
                StellarStandIn::Disc(Color::rgba(201, 163, 107, 255)),
            ),
            sheetless(
                MOON,
                "Pebble",
                (50, 250),
                StellarStandIn::Disc(Color::WHITE),
            ),
        ],
        ..catalog()
    }
}

/// The player at Sol's centre with every stellar shown.
fn frame() -> FlightFrame {
    let mut frame = in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)]);
    frame.stellars.extend(
        [STATION, PLANET, MOON]
            .into_iter()
            .map(|id| record::stellar(StellarId(id))),
    );
    frame
}

fn table() -> WorldModels {
    WorldModels {
        stellars: BTreeMap::from([
            (
                StellarId(STATION),
                StellarModel::Model {
                    key: HUB,
                    reach: REACH,
                },
            ),
            (StellarId(PLANET), StellarModel::Planet { key: GIANT }),
        ]),
        ..WorldModels::default()
    }
}

fn view_of(models: WorldModels) -> Faked {
    world(catalog_3d(), frame()).with_world_models(Rc::new(models))
}

/// Where `view` centres stellar `id`'s stand-in: as a sprite as wide as
/// the landing reach.
fn stand_in_centre(view: &Faked, id: i16) -> Point {
    let stellar = view
        .scene()
        .expect("laid out")
        .stellars()
        .iter()
        .find(|stellar| stellar.id == StellarId(id))
        .expect("held");
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let side = (2.0 * UNKNOWN_STELLAR_RADIUS) as u32;
    view.camera().sprite_centre(stellar.position, (side, side))
}

/// `stand_in` drawn round `centre` as the scene draws it.
fn stand_in_at(stand_in: &StellarStandIn, centre: Point) -> Vec<DrawCommand> {
    let mut list = DrawList::new();
    crate::outline::draw_stellar(&mut list, stand_in, centre, UNKNOWN_STELLAR_RADIUS);
    list.iter().cloned().collect()
}

fn commands(list: &DrawList) -> Vec<DrawCommand> {
    list.iter().cloned().collect()
}

#[test]
fn a_station_with_a_model_draws_it_unturned_at_its_stand_ins_centre_and_reach() {
    let view = view_of(table());
    let list = commands(&world_list(&view));
    let centre = stand_in_centre(&view, STATION);
    let model = DrawCommand::Model {
        model: HUB,
        pose: ModelPose {
            center: centre,
            orientation: Orientation::IDENTITY,
            scale: UNKNOWN_STELLAR_RADIUS / REACH,
        },
    };
    assert_eq!(list.iter().filter(|c| **c == model).count(), 1, "{list:?}");
    let outline = stand_in_at(&StellarStandIn::Outline(hub_from_above()), centre);
    assert!(
        !list.iter().any(|c| outline.contains(c)),
        "no stand-in outline"
    );
}

/// The table with the hub reaching `reach`.
fn reaching(reach: f32) -> WorldModels {
    let mut models = table();
    models
        .stellars
        .insert(StellarId(STATION), StellarModel::Model { key: HUB, reach });
    models
}

#[test]
fn a_station_whose_model_would_scale_past_finite_draws_its_stand_in() {
    // Reaching nowhere, or so little the scale overflows, or no number.
    for reach in [0.0, -0.0, 1e-40, f32::NAN] {
        let view = view_of(reaching(reach));
        let list = commands(&world_list(&view));
        assert!(
            !list
                .iter()
                .any(|c| matches!(c, DrawCommand::Model { model: HUB, .. })),
            "{reach}: no model"
        );
        let outline = stand_in_at(
            &StellarStandIn::Outline(hub_from_above()),
            stand_in_centre(&view, STATION),
        );
        let at = list
            .iter()
            .position(|c| *c == outline[0])
            .unwrap_or_else(|| panic!("{reach}: the stand-in"));
        assert_eq!(list[at..at + outline.len()], outline[..], "{reach}");
    }
}

#[test]
fn a_station_whose_model_reaches_a_little_draws_it_scaled_up() {
    let reach = 1e-30;
    let view = view_of(reaching(reach));
    let list = commands(&world_list(&view));
    let model = DrawCommand::Model {
        model: HUB,
        pose: ModelPose {
            center: stand_in_centre(&view, STATION),
            orientation: Orientation::IDENTITY,
            scale: UNKNOWN_STELLAR_RADIUS / reach,
        },
    };
    assert_eq!(list.iter().filter(|c| **c == model).count(), 1, "{list:?}");
}

#[test]
fn a_planet_draws_at_the_stand_in_disc_s_radius_on_the_flight_clock() {
    let planet = |view: &Faked| {
        commands(&world_list(view))
            .into_iter()
            .find_map(|c| match c {
                DrawCommand::Planet { planet, pose } => Some((planet, pose)),
                _ => None,
            })
            .expect("the planet")
    };
    let mut view = view_of(table());
    let centre = stand_in_centre(&view, PLANET);
    let (key, early) = planet(&view);
    assert_eq!(key, GIANT);
    assert_eq!(
        early,
        PlanetPose {
            center: centre,
            radius: UNKNOWN_STELLAR_RADIUS,
            time: view.elapsed.as_secs_f64(),
        }
    );
    view.elapsed += Duration::from_millis(2500);
    let (_, late) = planet(&view);
    assert_eq!(
        late,
        PlanetPose {
            time: early.time + 2.5,
            ..early
        },
        "only the time moves"
    );
    let disc = stand_in_at(
        &StellarStandIn::Disc(Color::rgba(201, 163, 107, 255)),
        centre,
    );
    let list = commands(&world_list(&view));
    assert!(!list.iter().any(|c| disc.contains(c)), "no stand-in disc");
}

#[test]
fn a_stellar_with_no_entry_draws_its_stand_in_and_names_and_order_are_unchanged() {
    let before = commands(&world_list(&view_of(WorldModels::default())));
    let after = commands(&world_list(&view_of(table())));
    let view = view_of(table());
    let moon = stand_in_at(
        &StellarStandIn::Disc(Color::WHITE),
        stand_in_centre(&view, MOON),
    );
    // The moon's disc, whole and in place, in both.
    for list in [&before, &after] {
        let at = list
            .iter()
            .position(|c| *c == moon[0])
            .expect("the moon's disc");
        assert_eq!(list[at..at + moon.len()], moon[..]);
    }
    let texts = |list: &[DrawCommand]| -> Vec<DrawCommand> {
        list.iter()
            .filter(|c| matches!(c, DrawCommand::Text { .. }))
            .cloned()
            .collect()
    };
    assert_eq!(texts(&after), texts(&before), "the names, as before");
    // Every command but the two stand-ins drawn over is as before, in order.
    let replaced: Vec<DrawCommand> = stand_in_at(
        &StellarStandIn::Outline(hub_from_above()),
        stand_in_centre(&view, STATION),
    )
    .into_iter()
    .chain(stand_in_at(
        &StellarStandIn::Disc(Color::rgba(201, 163, 107, 255)),
        stand_in_centre(&view, PLANET),
    ))
    .collect();
    let kept: Vec<&DrawCommand> = before.iter().filter(|c| !replaced.contains(c)).collect();
    let others: Vec<&DrawCommand> = after
        .iter()
        .filter(|c| !matches!(c, DrawCommand::Model { .. } | DrawCommand::Planet { .. }))
        .collect();
    assert_eq!(others, kept);
}

#[test]
fn the_3d_stellars_keep_the_stellars_id_order() {
    let list = commands(&world_list(&view_of(table())));
    let model = list
        .iter()
        .position(|c| matches!(c, DrawCommand::Model { model: HUB, .. }))
        .expect("the hub");
    let planet = list
        .iter()
        .position(|c| matches!(c, DrawCommand::Planet { .. }))
        .expect("the planet");
    assert!(model < planet, "140 before 141");
}
