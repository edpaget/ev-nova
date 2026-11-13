//! The rescue's closing repaint of a system with stellar models
//! (`rescue` module docs, "The sprites"): a station with a model and a
//! planet in the flight's table ([`FlightView::with_world_models`]) are
//! drawn where their stand-ins would be, as flight draws them, each cut
//! to the play area; with an empty table the repaint draws the stand-ins
//! as before; and the repaints under the texts draw none. On
//! [`FakeFlight`], with a hand-built table.

use std::collections::BTreeMap;
use std::rc::Rc;

use nova_sim::landing::UNKNOWN_STELLAR_RADIUS;

use super::rescue::{
    NEW_BACKGROUND, briefed, closing_over, pod_over, rescue_into, rescued_frame, space_painted,
    stellars_drawn, under_the_first_text,
};
use super::stellar_models::{
    GIANT, HUB, REACH, hub_from_above, sheetless, stand_in_at, stand_in_centre,
};
use super::*;
use crate::flight::world_models::{StellarModel, WorldModels};
use crate::model::{ModelPose, Orientation};
use crate::outline::StellarStandIn;
use crate::planet::PlanetPose;

/// A station and a planet in Alpha Centauri (131), beside Proxima (140),
/// neither with a sheet.
const STATION: i16 = 142;
const PLANET: i16 = 143;

/// Sol, and Alpha Centauri with the station and the planet.
fn catalog_3d() -> FakeCatalog {
    FakeCatalog {
        alpha_more: vec![
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
        ],
        ..catalog()
    }
}

/// The station's hub, the planet's ringed globe.
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
            (StellarId(PLANET), StellarModel::planet(GIANT, 2.3)),
        ]),
        ..WorldModels::default()
    }
}

/// [`rescued_frame`], the station and the planet shown besides Proxima.
fn rescued_3d() -> FlightFrame {
    let mut frame = rescued_frame();
    frame.stellars.extend(
        [STATION, PLANET]
            .into_iter()
            .map(|id| record::stellar(StellarId(id))),
    );
    frame
}

/// A pod in Sol over the 3D catalog, its flight given `models`.
fn pod_with(models: WorldModels) -> Faked {
    pod_over(catalog_3d()).with_world_models(Rc::new(models))
}

fn clipped(view: &Faked, command: DrawCommand) -> DrawCommand {
    DrawCommand::Clipped {
        clip: view.play_rect(),
        command: Box::new(command),
    }
}

/// Whether `command` is a model or a planet, clipped or not.
fn is_3d(command: &DrawCommand) -> bool {
    match command {
        DrawCommand::Model { .. } | DrawCommand::Planet { .. } => true,
        DrawCommand::Clipped { command, .. } => is_3d(command),
        _ => false,
    }
}

/// Where `run` starts in `list` at or after `from`, if it is there whole.
fn run_at(list: &[DrawCommand], run: &[DrawCommand], from: usize) -> Option<usize> {
    (from..=list.len().saturating_sub(run.len())).find(|&at| list[at..at + run.len()] == *run)
}

#[test]
fn the_closing_repaint_draws_a_modelled_station_and_planet_clipped_to_the_play_area() {
    let view = closing_over(pod_with(table()), rescued_3d());
    assert!(view.rescuing(), "the closing");
    let list: Vec<DrawCommand> = drawn(&view).iter().cloned().collect();

    let station = stand_in_centre(&view, STATION);
    let model = DrawCommand::model(
        HUB,
        ModelPose {
            center: station,
            orientation: Orientation::IDENTITY,
            scale: UNKNOWN_STELLAR_RADIUS / REACH,
        },
    );
    assert!(list.contains(&clipped(&view, model)), "{list:?}");
    let planet = DrawCommand::Planet {
        planet: GIANT,
        pose: PlanetPose {
            center: stand_in_centre(&view, PLANET),
            radius: UNKNOWN_STELLAR_RADIUS,
            time: view.elapsed.as_secs_f64(),
        },
    };
    assert!(list.contains(&clipped(&view, planet)), "{list:?}");
    let outline = stand_in_at(&StellarStandIn::Outline(hub_from_above()), station);
    assert!(
        !outline
            .into_iter()
            .any(|triangle| list.contains(&clipped(&view, triangle))),
        "no stand-in outline"
    );

    // The whole run is live flight's stellar layer, clipped, over the
    // space.
    let scene = view.scene().expect("laid out");
    let mut live = DrawList::new();
    live.world_in(view.play_rect(), |live| {
        view.draw_stellar_layer(live, scene, &view.camera());
    });
    let run: Vec<DrawCommand> = live
        .iter()
        .map(|command| clipped(&view, command.clone()))
        .collect();
    let space = first(&drawn(&view), |command| {
        *command == space_painted(&view, NEW_BACKGROUND)
    });
    assert!(run_at(&list, &run, space).is_some(), "{list:?}");
}

#[test]
fn with_an_empty_table_the_closing_repaint_draws_the_stand_ins_as_before() {
    let view = closing_over(pod_with(WorldModels::default()), rescued_3d());
    let list: Vec<DrawCommand> = drawn(&view).iter().cloned().collect();
    let stellars = stellars_drawn(&view);
    let station = stand_in_at(
        &StellarStandIn::Outline(hub_from_above()),
        stand_in_centre(&view, STATION),
    );
    assert!(
        stellars.contains(&clipped(&view, station[0].clone())),
        "the station's stand-in"
    );
    let space = first(&drawn(&view), |command| {
        *command == space_painted(&view, NEW_BACKGROUND)
    });
    assert!(run_at(&list, &stellars, space).is_some(), "{list:?}");
    assert!(!list.iter().any(is_3d), "{list:?}");
}

#[test]
fn the_repaint_under_a_text_draws_no_stellar_model() {
    let mut view = pod_with(table());
    rescue_into(&mut view, (vec![(briefed(), None)], None), rescued_3d());
    let list = under_the_first_text(&mut view);
    assert!(!list.iter().any(is_3d), "hidden (@0x68e04): {list:?}");
}
