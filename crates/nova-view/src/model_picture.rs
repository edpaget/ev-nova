//! A ship's or stellar's 3D model drawn in a screen's picture, in place of
//! a `PICT`, a sprite or a stand-in (rdm:phase/rendering-3d/phase-8-models-in-other-screens).
//!
//! The flight draws the new game's ships and stellars as their models
//! (`flight/view`'s `ship_models` and `stellar_models` modules). The other
//! screens that show one draw it here, by the same table
//! ([`WorldModels`]): the target panel, the shipyard's and the hire
//! screen's picture and grid icons, the escort and comm dialogs' item 11,
//! the spaceport's landscape, the planet comm dialog's item 5, the system
//! viewer and the main menu's pilot panel
//! (rdm:phase/rendering-3d/phase-16-pilot-panel-ship-model). The ship
//! browser, a developer's viewer, turns a class's portrait in flight's
//! top-down pose ([`ShipPortrait::draw_posed_in`]) rather than holding it
//! still (rdm:phase/rendering-3d/phase-17-ship-browser-models).
//!
//! - **A ship** draws as its [`ShipPortrait`]: its hull and the outfits
//!   its class's own and as-built kits' mounts show, turrets facing the
//!   nose, in the still [`portrait_orientation`], drawn whole (alpha 1).
//!   The original's pictures are still images that never turn or fade, so
//!   a portrait doesn't either. It is fitted to its rectangle by its
//!   radius ([`ShipModel::radius`]) to reach [`FIT`] of half the shorter
//!   side, so no pose can take it out of the rectangle: the projection is
//!   orthographic, so nothing is drawn further from the centre than its
//!   3D distance. A picture fits its model rather than clipping it
//!   ([`DrawCommand::Clipped`], which would cut it at the edge), so this
//!   fitting is what keeps it inside, whole.
//! - **A stellar** draws as flight draws it ([`draw_stellar_model`]): a
//!   station's model unturned, seen from above as its stand-in is, a
//!   planet's globe turned by a clock, at a radius the screen gives. A
//!   planet's rings and atmosphere reach beyond that radius, so a picture
//!   divides its room by their reach ([`StellarModel::overhang`]) to keep
//!   the whole planet inside, fitted rather than clipped as a model is.
//!
//! Each screen tries its own picture first (a `PICT`, a sheet's frame),
//! so stock data, which has no models and so an empty table, draws as it
//! always did.
//!
//! [`ShipModel::radius`]: crate::flight::world_models::ShipModel::radius

use nova_sim::ShipId;

use crate::draw::{DrawCommand, DrawList};
use crate::flight::mounted_outfits;
use crate::flight::world_models::{StellarModel, WorldModels};
use crate::geometry::{Bounds, Point};
use crate::model::{ModelKey, ModelPose, Orientation};
use crate::planet::PlanetPose;

/// How much of a picture's shorter side a stand-in or a model spans: a
/// margin, so the shape sits inside its box rather than touching its
/// edges.
pub const FIT: f32 = 0.8;

/// How a portrait turns its hull, in degrees about the view axis: its
/// nose, up the screen unturned, points to the lower right.
const PORTRAIT_HEADING: f32 = 135.0;

/// How far a portrait tilts its hull's deck toward the camera, in degrees
/// about the screen's x axis: the camera looks down on the deck from 30
/// degrees above its plane.
const PORTRAIT_TILT: f32 = 60.0;

/// The still pose every ship portrait is drawn in: its hull turned
/// [`PORTRAIT_HEADING`] about the view axis, so its nose points to the
/// lower right, then tilted [`PORTRAIT_TILT`] about the screen's x axis
/// (+y toward +z, the [`crate::model`] axes), so its deck faces the
/// camera and the standard light from up and to the left.
#[must_use]
pub fn portrait_orientation() -> Orientation {
    Orientation::about([0.0, 0.0, 1.0], PORTRAIT_HEADING.to_radians()).then(Orientation::about(
        [1.0, 0.0, 0.0],
        PORTRAIT_TILT.to_radians(),
    ))
}

/// A ship class's model as a picture: its hull, how far it and its parts
/// reach, and the parts its class as built shows (see the module docs).
#[derive(Clone, Debug, PartialEq)]
pub struct ShipPortrait {
    /// The hull's model.
    hull: ModelKey,
    /// How far the hull and its parts reach, in model units
    /// ([`ShipModel::radius`](crate::flight::world_models::ShipModel::radius)).
    radius: f32,
    /// A [`DrawCommand::Mounted`] for each part, in the mounts' order. A
    /// mounting is relative to its hull, so the hull's pose doesn't move
    /// them.
    parts: Vec<DrawCommand>,
}

impl ShipPortrait {
    /// The portrait of ship class `ship` from `models`: none for a class
    /// with no model, or with a radius that isn't a positive, finite
    /// number, which nothing could fit.
    #[must_use]
    pub fn of(models: &WorldModels, ship: ShipId) -> Option<Self> {
        let model = models.ship(ship)?;
        if !(model.radius.is_finite() && model.radius > 0.0) {
            return None;
        }
        let mounts = models.mounts(ship, models.built_kits(ship));
        let pose = ModelPose {
            center: Point::new(0.0, 0.0),
            orientation: portrait_orientation(),
            scale: 1.0,
        };
        let commands = mounted_outfits(model.key, pose, 0.0, &mounts, &[], |asset| {
            models.outfit(asset)
        });
        Some(Self {
            hull: model.key,
            radius: model.radius,
            parts: commands.into_iter().skip(1).collect(),
        })
    }

    /// The hull's model.
    #[must_use]
    pub fn hull(&self) -> ModelKey {
        self.hull
    }

    /// A portrait of hull `hull` reaching `radius`, with no parts: the
    /// screens' tests' stand-in for a class's.
    #[cfg(test)]
    pub(crate) fn bare(hull: ModelKey, radius: f32) -> Self {
        Self {
            hull,
            radius,
            parts: Vec::new(),
        }
    }

    /// Draws it into `area`, centred, in [`portrait_orientation`], scaled
    /// so its radius reaches [`FIT`] of half the area's shorter side, its
    /// parts after it; says whether it drew. An area that gives no
    /// positive, finite scale (no size) or no finite centre (a side that
    /// is no number) draws nothing.
    pub fn draw_in(&self, list: &mut DrawList, area: Bounds) -> bool {
        self.draw_posed_in(list, area, portrait_orientation())
    }

    /// Draws it into `area` as [`ShipPortrait::draw_in`] does, fitted and
    /// centred the same way, but with its hull in `orientation` rather
    /// than the still [`portrait_orientation`]; its parts follow the
    /// hull's pose. The ship browser turns a portrait in flight's pose
    /// this way ([`crate::ships`]).
    pub fn draw_posed_in(
        &self,
        list: &mut DrawList,
        area: Bounds,
        orientation: Orientation,
    ) -> bool {
        let scale = FIT * area.width().min(area.height()) / 2.0 / self.radius;
        let centre = area.center();
        if !(scale.is_finite() && scale > 0.0 && centre.x.is_finite() && centre.y.is_finite()) {
            return false;
        }
        list.model(
            self.hull,
            ModelPose {
                center: centre,
                orientation,
                scale,
            },
        );
        for part in &self.parts {
            list.push(part.clone());
        }
        true
    }
}

/// Draws stellar `model` round `centre`, reaching `radius` logical units,
/// a planet at the clock's `time`, and says whether it drew: a station's
/// model unturned, scaled so its footprint (its stand-in outline's reach)
/// reaches `radius`, or a planet's globe of `radius` (its rings and
/// atmosphere reach [`StellarModel::overhang`] times it, which a picture
/// divides its room by). A model whose scale to that radius isn't
/// finite (it reaches nowhere, or too little, or its reach is no number)
/// draws nothing, so the caller draws its stand-in in its place.
pub fn draw_stellar_model(
    list: &mut DrawList,
    model: StellarModel,
    centre: Point,
    radius: f32,
    time: f64,
) -> bool {
    match model {
        StellarModel::Model { key, reach } => {
            let scale = radius / reach;
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
        StellarModel::Planet { key, .. } => {
            list.planet(
                key,
                PlanetPose {
                    center: centre,
                    radius,
                    time,
                },
            );
        }
    }
    true
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::collections::BTreeMap;

    use nova_data::store::{Footing, Mount, Mounts, Shown};
    use nova_sim::OutfitId;

    use super::*;
    use crate::flight::world_models::ShipModel;
    use crate::model::Mounting;
    use crate::planet::PlanetKey;

    const HULL: ModelKey = ModelKey(1);
    const TURRET: ModelKey = ModelKey(2);
    const GUN: ModelKey = ModelKey(3);
    const HUB: ModelKey = ModelKey(4);
    const BED: OutfitId = OutfitId(130);
    const SHIP: ShipId = ShipId(128);

    fn mount(hardpoint: &str, at: [f64; 3], turns: bool, asset: Option<&str>) -> Mount {
        Mount {
            hardpoint: hardpoint.to_owned(),
            at,
            footing: Footing::Surface,
            turns,
            shows: asset.map(|asset| Shown {
                asset: asset.to_owned(),
                lift: 0.25,
            }),
        }
    }

    /// Ship 128 at `radius`, a fixed gun on its nose, a bare hardpoint and
    /// a turret on its as-built [`BED`] kit.
    fn table(radius: f32) -> WorldModels {
        WorldModels {
            ships: BTreeMap::from([(
                SHIP,
                ShipModel {
                    key: HULL,
                    scale: 3.0,
                    base_transp: 12,
                    radius,
                },
            )]),
            built_kits: BTreeMap::from([(SHIP, vec![BED])]),
            mounts: Mounts {
                hulls: BTreeMap::from([(
                    SHIP,
                    vec![
                        mount("nose", [0.0, 2.0, 0.5], false, Some("gun")),
                        mount("bare", [1.0, 0.0, 0.0], false, None),
                    ],
                )]),
                kits: BTreeMap::from([(
                    (SHIP, BED),
                    vec![mount("bed", [0.0, -1.0, 0.5], true, Some("turret-mount"))],
                )]),
            },
            outfits: BTreeMap::from([("turret-mount".to_owned(), TURRET), ("gun".to_owned(), GUN)]),
            stellars: BTreeMap::new(),
        }
    }

    fn portrait() -> ShipPortrait {
        ShipPortrait::of(&table(4.0), SHIP).expect("a portrait")
    }

    fn drawn_in(portrait: &ShipPortrait, area: Bounds) -> (bool, Vec<DrawCommand>) {
        let mut list = DrawList::new();
        let drew = portrait.draw_in(&mut list, area);
        (drew, commands(&list))
    }

    fn commands(list: &DrawList) -> Vec<DrawCommand> {
        list.iter().cloned().collect()
    }

    #[track_caller]
    fn assert_near(a: [f32; 3], b: [f32; 3]) {
        for (p, q) in a.iter().zip(&b) {
            assert!((p - q).abs() < 1e-5, "{a:?} is not {b:?}");
        }
    }

    #[test]
    fn a_class_with_no_model_or_no_radius_to_fit_has_no_portrait() {
        assert_eq!(ShipPortrait::of(&WorldModels::default(), SHIP), None);
        assert_eq!(ShipPortrait::of(&table(4.0), ShipId(129)), None);
        for radius in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert_eq!(ShipPortrait::of(&table(radius), SHIP), None, "{radius}");
        }
        assert!(ShipPortrait::of(&table(f32::MIN_POSITIVE), SHIP).is_some());
    }

    #[test]
    fn its_parts_are_the_class_s_as_built_mounts_turrets_facing_the_nose() {
        let models = table(4.0);
        let portrait = portrait();
        assert_eq!(portrait.hull(), HULL);
        let pose = ModelPose {
            center: Point::new(9.0, 9.0),
            orientation: Orientation::IDENTITY,
            scale: 2.0,
        };
        let want = mounted_outfits(
            HULL,
            pose,
            0.0,
            &models.mounts(SHIP, &[BED]),
            &[],
            |asset| models.outfit(asset),
        );
        assert_eq!(portrait.parts, want[1..]);
        assert_eq!(
            portrait.parts,
            [
                DrawCommand::Mounted {
                    model: GUN,
                    mounting: Mounting {
                        at: [0.0, -2.0, 0.75],
                        turn: 0.0
                    },
                },
                DrawCommand::Mounted {
                    model: TURRET,
                    mounting: Mounting {
                        at: [0.0, 1.0, 0.75],
                        turn: 0.0
                    },
                },
            ]
        );
    }

    #[test]
    fn it_draws_its_hull_whole_at_the_area_s_centre_fitted_then_its_parts() {
        let portrait = portrait();
        let area = Bounds::at(Point::new(10.0, 20.0), 128.0, 64.0);
        let (drew, commands) = drawn_in(&portrait, area);
        assert!(drew);
        let mut want = vec![DrawCommand::Model {
            model: HULL,
            pose: ModelPose {
                center: Point::new(74.0, 52.0),
                orientation: portrait_orientation(),
                scale: FIT * 64.0 / 2.0 / 4.0,
            },
            alpha: 1.0,
            colorize: None,
        }];
        want.extend(portrait.parts.iter().cloned());
        assert_eq!(commands, want);
        assert_eq!(commands.len(), 3);
    }

    #[test]
    fn its_radius_at_its_scale_stays_within_fit_of_half_the_shorter_side() {
        let portrait = portrait();
        for (w, h) in [(64.0, 64.0), (128.0, 64.0), (32.0, 90.0)] {
            let area = Bounds::at(Point::new(-5.0, 7.0), w, h);
            let (_, commands) = drawn_in(&portrait, area);
            let DrawCommand::Model { pose, .. } = commands[0] else {
                panic!("a model first");
            };
            let reach = portrait.radius * pose.scale;
            let room = FIT * f32::min(w, h) / 2.0;
            assert!(reach <= room + 1e-4, "{w}x{h}: {reach} > {room}");
            assert!(reach >= room - 1e-4, "{w}x{h}: fills the room");
        }
    }

    #[test]
    fn an_area_with_no_scale_to_give_draws_nothing() {
        let portrait = portrait();
        for area in [
            Bounds::at(Point::new(0.0, 0.0), 0.0, 64.0),
            Bounds::at(Point::new(0.0, 0.0), f32::NAN, 64.0),
            Bounds::at(Point::new(0.0, 0.0), 64.0, f32::NAN),
            Bounds::at(Point::new(0.0, 0.0), 64.0, -3.0),
        ] {
            let (drew, commands) = drawn_in(&portrait, area);
            assert!(!drew, "{area:?}");
            assert_eq!(commands, [], "{area:?}");
        }
        let huge = ShipPortrait::of(&table(f32::MIN_POSITIVE), SHIP).expect("a portrait");
        let (drew, commands) = drawn_in(&huge, Bounds::at(Point::new(0.0, 0.0), 64.0, 64.0));
        assert!(!drew, "an infinite scale");
        assert_eq!(commands, []);
        for (portrait, area) in [
            (&portrait, Bounds::at(Point::new(0.0, 0.0), 0.0, 64.0)),
            (&portrait, Bounds::at(Point::new(0.0, 0.0), f32::NAN, 64.0)),
            (&portrait, Bounds::at(Point::new(0.0, 0.0), 64.0, f32::NAN)),
            (&portrait, Bounds::at(Point::new(0.0, 0.0), 64.0, -3.0)),
            (&portrait, Bounds::at(Point::new(f32::NAN, 0.0), 64.0, 64.0)),
            (&portrait, Bounds::at(Point::new(0.0, f32::NAN), 64.0, 64.0)),
            (&huge, Bounds::at(Point::new(0.0, 0.0), 64.0, 64.0)),
        ] {
            let mut list = DrawList::new();
            let drew = portrait.draw_posed_in(&mut list, area, Orientation::IDENTITY);
            assert!(!drew, "{area:?}");
            assert_eq!(list.iter().count(), 0, "{area:?}");
        }
    }

    #[test]
    fn drawn_posed_it_fits_and_centres_as_draw_in_and_takes_the_pose_given() {
        let portrait = portrait();
        let area = Bounds::at(Point::new(10.0, 20.0), 128.0, 64.0);
        let pose = Orientation::about([0.0, 0.0, 1.0], 0.7);
        let mut list = DrawList::new();
        assert!(portrait.draw_posed_in(&mut list, area, pose));
        let mut want = vec![DrawCommand::Model {
            model: HULL,
            pose: ModelPose {
                center: Point::new(74.0, 52.0),
                orientation: pose,
                scale: FIT * 64.0 / 2.0 / 4.0,
            },
            alpha: 1.0,
            colorize: None,
        }];
        want.extend(portrait.parts.iter().cloned());
        assert_eq!(commands(&list), want);
    }

    #[test]
    fn draw_in_is_drawn_posed_in_the_portrait_orientation() {
        let portrait = portrait();
        let area = Bounds::at(Point::new(10.0, 20.0), 128.0, 64.0);
        let mut posed = DrawList::new();
        assert!(portrait.draw_posed_in(&mut posed, area, portrait_orientation()));
        let (drew, still) = drawn_in(&portrait, area);
        assert!(drew);
        assert_eq!(still, commands(&posed));
    }

    #[test]
    fn the_pose_points_the_nose_to_the_lower_right_and_the_deck_up_to_the_camera() {
        let pose = portrait_orientation();
        let nose = pose.rotate([0.0, -1.0, 0.0]).expect("a turn");
        assert!(nose[0] > 0.0 && nose[1] > 0.0, "{nose:?}");
        let deck = pose.rotate([0.0, 0.0, 1.0]).expect("a turn");
        assert!(deck[2] > 0.0 && deck[1] < 0.0, "{deck:?}");
        let half = std::f32::consts::FRAC_1_SQRT_2;
        assert_near(nose, [half, half * 0.5, half * 3f32.sqrt() / 2.0]);
        assert_near(deck, [0.0, -(3f32.sqrt()) / 2.0, 0.5]);
    }

    #[test]
    fn a_station_scales_to_the_radius_over_its_reach_unturned_and_a_planet_takes_it() {
        let centre = Point::new(30.0, 40.0);
        let mut list = DrawList::new();
        let station = StellarModel::Model {
            key: HUB,
            reach: 4.0,
        };
        assert!(draw_stellar_model(&mut list, station, centre, 10.0, 5.0));
        // Its rings reach beyond the globe, which takes the radius.
        let giant = StellarModel::planet(PlanetKey(1), 2.3);
        assert!(draw_stellar_model(&mut list, giant, centre, 10.0, 5.0));
        assert_eq!(
            commands(&list),
            [
                DrawCommand::model(
                    HUB,
                    ModelPose {
                        center: centre,
                        orientation: Orientation::IDENTITY,
                        scale: 2.5,
                    }
                ),
                DrawCommand::Planet {
                    planet: PlanetKey(1),
                    pose: PlanetPose {
                        center: centre,
                        radius: 10.0,
                        time: 5.0,
                    },
                },
            ]
        );
    }

    #[test]
    fn a_station_no_radius_can_size_draws_nothing() {
        for reach in [0.0, f32::NAN, f32::MIN_POSITIVE] {
            let mut list = DrawList::new();
            let station = StellarModel::Model { key: HUB, reach };
            assert!(!draw_stellar_model(
                &mut list,
                station,
                Point::new(0.0, 0.0),
                10.0,
                0.0
            ));
            assert_eq!(commands(&list), [], "{reach}");
        }
    }
}
