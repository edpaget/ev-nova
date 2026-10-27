//! [`FlightView`]'s scene as the kept frame holds its stellars, over the
//! [`FlightSim`] port: which are shown, how each looks
//! ([`FrameStellar`](nova_sim::flight_frame::FrameStellar)) and where its
//! animation stands, and the starfield scattered about the frame's ship.
//! Any flight drives them, a scripted one in the component tests.

use std::time::Duration;

use crate::flight::sim_port::FlightSim;
use crate::system::catalog::SystemCatalog;

use super::stars;
use super::{FlightView, look, show_destroyed, show_gate_frames, shown_scene};

impl<C, S: FlightSim> FlightView<C, S> {
    /// Draws each stellar of the scene shown as the frame, filled afresh,
    /// holds it after a rebuild point
    /// ([`SystemScene::rebuild`](crate::system::scene::SystemScene::rebuild)):
    /// one whose look changed shows its current frame. The scene is not
    /// laid out again.
    pub(super) fn rebuild_stellars(&mut self) {
        self.refresh_frame();
        if let Some(scene) = &mut self.scene {
            scene.rebuild(|id| look(&self.frame, id));
        }
    }

    /// Draws each stellar of the scene shown as the frame, filled afresh,
    /// holds it ([`show_destroyed`]). The scene is not laid out again.
    pub(super) fn show_destroyed(&mut self) {
        self.refresh_frame();
        if let Some(scene) = &mut self.scene {
            show_destroyed(scene, &self.frame);
        }
    }

    /// Moves the clock on by `dt` and steps the stellars' animations by
    /// the whole ticks of 1/30 s it crossed, as the original's sprite tick
    /// runs every play frame, the jump's included. A random next frame
    /// (`Flags2` 0x0002) is drawn on the effects' chance, never the
    /// session's: the frame a stellar shows is drawn state only.
    pub(super) fn step_stellars(&mut self, dt: Duration) {
        let before = crate::time::ticks(self.elapsed);
        self.elapsed += dt;
        if let Some(scene) = &mut self.scene {
            scene.step(
                crate::time::ticks(self.elapsed) - before,
                &mut self.effects_chance,
            );
        }
    }

    /// Scatters the stars afresh about the frame's ship and places them
    /// once, on the effects' chance, when the flight draws the original's
    /// starfield (see the `stars` module): never for a flight that failed,
    /// whose looks hold no stars' sheet.
    pub(super) fn reset_stars(&mut self) {
        if stars::original(self.presentation.starfield, &self.looks) {
            let at = stars::surroundings(&self.frame, self.view, self.anchor(), &self.looks);
            stars::reset(
                &mut self.stars,
                &at,
                self.parallax_starfield,
                &mut self.effects_chance,
            );
        }
    }
}

impl<C: SystemCatalog, S: FlightSim> FlightView<C, S> {
    /// Lets go of the scene shown, keeping each of its stellars'
    /// animation state
    /// ([`KeptFrames::keep`](crate::system::kept_frames::KeptFrames::keep)),
    /// and shows the frame's in its place, read from the catalog with the
    /// stellars the frame, filled afresh, shows ([`shown_scene`]): each
    /// stellar looking as the frame holds it ([`show_destroyed`]), then each
    /// the flight has shown before resumed where it was left, on the sheet
    /// it now draws
    /// ([`SystemScene::resume`](crate::system::scene::SystemScene::resume)),
    /// and each hypergate on its frame's frame. The look is set before the
    /// frames resume, so the rebuild or redraw that follows finds it
    /// unchanged and leaves them alone.
    pub(super) fn show_scene(&mut self) {
        self.refresh_frame();
        if let Some(left) = &self.scene {
            self.kept_frames.keep(left);
        }
        let mut scene = shown_scene(&*self.catalog, &self.frame);
        show_destroyed(&mut scene, &self.frame);
        scene.resume(&self.kept_frames);
        show_gate_frames(&mut scene, &self.frame);
        self.scene = Some(scene);
    }
}
