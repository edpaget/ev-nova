//! The flight's stellars drawn in 3D (rdm:phase/engine-3d/phase-10-sample-in-3d):
//! in the stellars' pass, a stellar the edge registered a model or a
//! planet for ([`FlightView::with_world_models`],
//! [`StellarModel`]) draws it in place of its stand-in; every other
//! stellar draws exactly as [`scene::draw_stellars`] draws it, in the
//! same order (stellar ID), its name included.
//!
//! - **A model** (a station's or an outpost's): a [`DrawCommand::Model`]
//!   at the stand-in's centre, unturned, scaled so its footprint reaches
//!   as far as the stand-in's ([`UNKNOWN_STELLAR_RADIUS`] over the table's
//!   reach), in its own colours. One that scale can't size (a reach of
//!   nothing, so little the scale overflows, or no number) draws its
//!   stand-in instead, so no infinite scale reaches the draw list.
//! - **A planet**: a [`DrawCommand::Planet`] at the stand-in's centre, its
//!   globe the stand-in disc's radius, turned by the view's flight clock.
//!
//! Only a stellar that draws its stand-in, having no sheet, is replaced:
//! stock stellars have sheets, and stock data has no models, so it draws
//! as it always did. The data picks the model, not a rulebook key. The
//! system view draws no models.
//!
//! [`DrawCommand::Model`]: crate::DrawCommand::Model
//! [`DrawCommand::Planet`]: crate::DrawCommand::Planet

use nova_sim::landing::UNKNOWN_STELLAR_RADIUS;

use crate::flight::sim_port::FlightSim;
use crate::flight::world_models::StellarModel;
use crate::model::{ModelPose, Orientation};
use crate::planet::PlanetPose;
use crate::system::camera::Camera;
use crate::system::scene::{self, SystemScene};
use crate::{DrawList, Point};

use super::FlightView;

impl<C, S: FlightSim> FlightView<C, S> {
    /// Draws the stellars' pass: each stellar as [`scene::draw_stellars`]
    /// draws it, murked as the player sees it, but one with a model or
    /// planet in the table draws that in its stand-in's place (see the
    /// module docs).
    pub(super) fn draw_stellar_layer(
        &self,
        list: &mut DrawList,
        scene: &SystemScene,
        camera: &Camera,
    ) {
        let time = self.elapsed.as_secs_f64();
        scene::draw_stellars_instead(
            list,
            scene,
            camera,
            Some(self.seen_murk()),
            |list, stellar, centre| {
                self.world_models
                    .stellar(stellar.id)
                    .is_some_and(|model| draw_stellar_model(list, model, centre, time))
            },
        );
    }
}

/// Draws `model` round `centre`, reaching the stand-in's radius, a planet
/// at the flight clock's `time` (see the module docs), and says whether
/// it drew. A model whose scale to that radius isn't finite (it reaches
/// nowhere, or too little, or its reach is no number) draws nothing, so
/// the stand-in draws in its place.
fn draw_stellar_model(list: &mut DrawList, model: StellarModel, centre: Point, time: f64) -> bool {
    match model {
        StellarModel::Model { key, reach } => {
            let scale = UNKNOWN_STELLAR_RADIUS / reach;
            if !scale.is_finite() {
                return false;
            }
            list.model(
                key,
                ModelPose {
                    center: centre,
                    orientation: Orientation::IDENTITY,
                    scale,
                },
            );
        }
        StellarModel::Planet { key } => {
            list.planet(
                key,
                PlanetPose {
                    center: centre,
                    radius: UNKNOWN_STELLAR_RADIUS,
                    time,
                },
            );
        }
    }
    true
}
