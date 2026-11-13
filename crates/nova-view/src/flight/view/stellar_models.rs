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
//! Both are drawn by [`draw_stellar_model`] ([`FlightView::stellar_model_drawn`]),
//! which the other screens that show a stellar share. The escape pod
//! rescue's closing repaint draws them the same way, each cut to the play
//! area ([`scene::draw_stellars_clipped_instead`]), so it too commands every
//! stellar it shows (below).
//!
//! # Every shown stellar is commanded, on screen or off
//!
//! The pass commands each stellar the frame shows wherever its stand-in
//! falls, as the stand-ins themselves are: nothing here culls one that is
//! off screen. The renderer relies on that. It makes a planet ready (bakes
//! a generated look, builds its mips, uploads it) in the first frame that
//! commands it (`nova_render`'s `planets`), which takes tens of
//! milliseconds a planet. With every stellar commanded each frame, a
//! system's planets are all made ready in the first frame drawn there, and
//! after a jump or a hypergate that frame is drawn under the arrival's
//! solid white ([`JumpPhase::FadeIn`](crate::flight::jump::JumpPhase::FadeIn)
//! at its start, or
//! [`JumpPhase::Flash`](crate::flight::jump::JumpPhase::Flash)), so the
//! pause holds the white and nothing hitches in motion. The view's
//! `first_frame_planets` tests pin both.
//!
//! So a cull added here needs a replacement: culled planets would each be
//! made ready in the frame they scroll into view, mid-flight. The
//! replacement would be a side datum on the draw list naming the planets a
//! screen will draw, for the renderer to make ready from.
//!
//! # The pass names its system
//!
//! The pass names its system on the list ([`DrawList::in_system`]), as
//! the rescue's repaint does. The renderer counts a visit for each new
//! system named, and lets go of the planets not drawn in this visit or the
//! one before, two flight systems back (`nova_render`'s `visits`); going
//! back there makes them ready again in the first frame, as a first visit
//! does. So a cull would need its replacement here too: a planet it left
//! uncommanded through two systems is let go, and made again at its next
//! command.
//!
//! Not every first frame has a white over it: a pilot's first frame in
//! flight, the first after a take-off or a wormhole, and a stellar that
//! starts hidden and shows later pause with nothing covering them. Each is
//! at a change of scene, not in motion.
//!
//! Only a stellar that draws its stand-in, having no sheet, is replaced:
//! stock stellars have sheets, and stock data has no models, so it draws
//! as it always did. The data picks the model, not a rulebook key. The
//! system viewer draws its stellars the same way, by its own clock
//! ([`SystemView::with_world_models`](crate::system::SystemView::with_world_models)).
//!
//! [`DrawCommand::Model`]: crate::DrawCommand::Model
//! [`DrawCommand::Planet`]: crate::DrawCommand::Planet
//! [`StellarModel`]: crate::flight::world_models::StellarModel

use nova_sim::landing::UNKNOWN_STELLAR_RADIUS;

use crate::flight::sim_port::FlightSim;
use crate::model_picture::draw_stellar_model;
use crate::system::camera::Camera;
use crate::system::scene::{self, SceneStellar, SystemScene};
use crate::{DrawList, Point};

use super::FlightView;

impl<C, S: FlightSim> FlightView<C, S> {
    /// Draws the stellars' pass: each stellar as [`scene::draw_stellars`]
    /// draws it, murked as the player sees it, but one with a model or
    /// planet in the table draws that in its stand-in's place (see the
    /// module docs). It names `scene`'s system on the list
    /// ([`DrawList::in_system`]): the frame's flight system.
    pub(super) fn draw_stellar_layer(
        &self,
        list: &mut DrawList,
        scene: &SystemScene,
        camera: &Camera,
    ) {
        list.in_system(scene.id());
        scene::draw_stellars_instead(
            list,
            scene,
            camera,
            Some(self.seen_murk()),
            |list, stellar, centre| self.stellar_model_drawn(list, stellar, centre),
        );
    }

    /// Draws `stellar`'s model or planet round `centre`, its stand-in's
    /// centre, if the table has one it can draw: at
    /// [`UNKNOWN_STELLAR_RADIUS`], a planet by the flight clock. Whether it
    /// drew, so its stand-in is left out (see the module docs).
    pub(super) fn stellar_model_drawn(
        &self,
        list: &mut DrawList,
        stellar: &SceneStellar,
        centre: Point,
    ) -> bool {
        let time = self.elapsed.as_secs_f64();
        self.world_models.stellar(stellar.id).is_some_and(|model| {
            draw_stellar_model(list, model, centre, UNKNOWN_STELLAR_RADIUS, time)
        })
    }
}
