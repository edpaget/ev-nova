//! The flight's ships drawn as 3D models (rdm:phase/engine-3d/phase-10-sample-in-3d):
//! a ship class the edge registered a hull model for
//! ([`FlightView::with_world_models`], [`WorldModels`]) draws it with its
//! outfits on their hardpoints ([`mounted_outfits()`]) in place of its
//! stand-in outline.
//!
//! A ship's look is tried in this order:
//!
//! 1. the catalog's own sheet;
//! 2. the opt-in stock look's sheet (the player chose it; see the
//!    `ship_looks` module);
//! 3. its class's model;
//! 4. its class's stand-in outline;
//! 5. the crossed box.
//!
//! So a model replaces the stand-in alone. The data picks it, not a
//! rulebook key: only a new game's classes have models, and the stand-in
//! it replaces has no counterpart in the original. Stock data has none,
//! so it draws as it always did.
//!
//! - **Where**: centred where its outline would be centred (the crossed
//!   box's place with none), turned by [`ship_orientation`] at the
//!   heading it is shown at and the frame's
//!   [`hull_turn`](nova_sim::flight_frame::FlightFrame::hull_turn), at the
//!   table's scale.
//! - **Outfits**: the mounts of its class and kits
//!   ([`WorldModels::mounts`]): the player's the kits the frame says it
//!   carries ([`FlightFrame::player_kits`](nova_sim::flight_frame::FlightFrame::player_kits)),
//!   an NPC's its class's as built ([`WorldModels::built_kits`]).
//! - **Turrets**: every turning mount faces the ship's
//!   [`FrameShip::turret_facing`], or the nose with none, swung toward it
//!   on the hull at
//!   [`TURRET_TURN_PER_PASS`](crate::flight::turret_swing::TURRET_TURN_PER_PASS)
//!   a pass shown rather than snapped to it
//!   ([`TurretSwings`](crate::flight::turret_swing::TurretSwings), stepped
//!   by the `stepping` module, let go with the NPCs' displays at a change
//!   of system or a landing).
//! - **Faded ships**: a ship its cloak, a gate or its class's `BaseTransp`
//!   ([`ShipModel::base_transp`]) fades draws its model faded, at the
//!   alpha [`model_alpha`] gives (the new game's reading of the 2D hull's
//!   fade; see the `model_fade` module), its outfits fading with it
//!   ([`DrawCommand::Model`](crate::DrawCommand::Model)), and moved by
//!   the jitter its cloak moves its hull by
//!   ([`CloakDraw::offset`]). A ship whose hull its cloak or gate hides
//!   (a full fade unseen, waiting at the gate it comes out of), or whose
//!   alpha comes out 0, draws nothing, outfits and outline included; a
//!   player not drawn in full draws nothing, as before. Only a class with
//!   no model draws its outline, unfaded, as it always did.
//! - **Colour**: a model takes none of the sprite's colorize (the murk,
//!   the ion tint, the gate's white wash), faded or not
//!   (rdm:phase/rendering-3d/phase-15-model-colorize-like-its-sprite).

use std::rc::Rc;

use nova_sim::OutfitId;
use nova_sim::flight_frame::{FrameShip, GateFade};

use crate::flight::catalog::ShipSprites;
use crate::flight::cloak_fade::CloakDraw;
use crate::flight::model_fade::model_alpha;
use crate::flight::mounted_outfits;
use crate::flight::ship_layers::Place;
use crate::flight::ship_model::ship_orientation;
use crate::flight::sim_port::FlightSim;
use crate::flight::world_models::{ShipModel, WorldModels};
use crate::model::ModelPose;
use crate::outline::{self, Outline};
use crate::{DrawList, Point};

use super::{FlightView, placeholder_size};

impl<C: ShipSprites, S: FlightSim> FlightView<C, S> {
    /// The flight with `models` drawn in place of the new game's
    /// stand-ins: ships here, stellars in the `stellar_models` module (see
    /// the module docs). An empty table draws as none.
    #[must_use]
    pub fn with_world_models(mut self, models: Rc<WorldModels>) -> Self {
        self.world_models = models;
        self
    }
}

impl<C, S: FlightSim> FlightView<C, S> {
    /// The player's ship's class's model, when it has one. Asked only once
    /// its sheet can't be read and it is drawn in full.
    pub(super) fn player_model(&self) -> Option<ShipModel> {
        self.world_models.ship(self.drawn().player().class)
    }

    /// NPC `npc`'s class's model, when it has one. Asked only once its
    /// sheet can't be read and its hull is drawn at all.
    pub(super) fn npc_model(&self, npc: &FrameShip) -> Option<ShipModel> {
        self.world_models.ship(npc.class)
    }

    /// Draws the player's ship as `model`, where `place` centres its
    /// outline, faded and jittered as its cloak `cloak` draws its hull
    /// ([`FlightView::draw_ship_model`]). A player's ship takes no gate's
    /// fade, as its sprite takes none.
    pub(super) fn draw_player_model(
        &self,
        list: &mut DrawList,
        place: Place<'_>,
        (model, cloak): (ShipModel, Option<CloakDraw>),
    ) {
        let frame = self.drawn();
        let player = frame.player();
        let centre = model_centre(place, self.outline.as_ref());
        let shown = (&player, centre, self.shown_heading());
        let fade = (GateFade::None, cloak);
        self.draw_ship_model(list, shown, (model, fade), &frame.player_kits);
    }

    /// Draws NPC `npc` as `model`, facing `heading`, where `place` centres
    /// its outline, faded and jittered as its cloak `cloak` and its gate
    /// draw its hull ([`FlightView::draw_ship_model`]).
    pub(super) fn draw_npc_model(
        &self,
        list: &mut DrawList,
        npc: &FrameShip,
        place: Place<'_>,
        (model, heading, cloak): (ShipModel, f32, Option<CloakDraw>),
    ) {
        let centre = model_centre(place, self.npc_outlines.get(&npc.class));
        let kits = self.world_models.built_kits(npc.class);
        let fade = (npc.gate, cloak);
        self.draw_ship_model(list, (npc, centre, heading), (model, fade), kits);
    }

    /// Draws `ship`, shown at `centre` facing `heading`, as its class's
    /// `model` with the outfits of its class and `kits` on their mounts,
    /// each turret facing its record's turret facing as its swing has
    /// reached it ([`TurretSwings::facing`](crate::flight::turret_swing::TurretSwings::facing)), at the alpha its
    /// cloak, `gate` and the model's `BaseTransp` give it ([`model_alpha`])
    /// and moved by its cloak's jitter (see the module docs); nothing at
    /// all, its outfits included, when that draws nothing.
    fn draw_ship_model(
        &self,
        list: &mut DrawList,
        (ship, centre, heading): (&FrameShip, Point, f32),
        (model, (gate, cloak)): (ShipModel, (GateFade, Option<CloakDraw>)),
        kits: &[OutfitId],
    ) {
        let Some(alpha) = model_alpha(ship.cloak, ship.paint, gate, model.base_transp) else {
            return;
        };
        let jitter = cloak.map_or(Point::new(0.0, 0.0), |cloak| cloak.offset);
        let turn = self.drawn().hull_turn;
        let pose = ModelPose {
            center: Point::new(centre.x + jitter.x, centre.y + jitter.y),
            orientation: ship_orientation(&ship.frames, heading, turn),
            scale: model.scale,
        };
        let angle = ship.frames.hull_angle(heading, turn);
        let mounts = self.world_models.mounts(ship.class, kits);
        let facings = vec![self.turret_swings.facing(ship, angle); mounts.len()];
        let commands = mounted_outfits(model.key, pose, angle, &mounts, &facings, |asset| {
            self.world_models.outfit(asset)
        });
        for command in commands {
            list.push(command.at_alpha(alpha));
        }
    }
}

/// Where a ship's model is centred: where `place` centres the frame of
/// its `outline`, which holds it at every heading ([`outline::extent`]),
/// or the crossed box's with none.
fn model_centre(place: Place<'_>, outline: Option<&Outline>) -> Point {
    place(outline.map_or_else(placeholder_size, |outline| outline::extent(outline, 1.0)))
}
