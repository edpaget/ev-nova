//! The flight's sprite world, the scene under the HUD, drawn in the order
//! `_SetupSpriteWorld` builds it ([`WORLD_PASSES`]).
//!
//! Bottom first: the stars (and the message line), the stellars, the
//! beams under the ships (`Flags2` 0x2000, the stellar layer's
//! `_BeamDrawOffscreenCallback`), the asteroids, the disabled ships, the
//! cargo boxes and then the escape pods, the smoke, the point-defence,
//! homing and other shots, the ships the player leads, the other ships and
//! the player, the turret shots over the ships, the explosions, the target
//! layer (the brackets, the cursor, the red-alert and double-time icons),
//! the other beams (the target layer's `_BeamDrawCallback`), the floaters
//! (`_PostDrawCallback`'s `_FloaterDrawCallback`: the message line's text,
//! the floating map and the escort menu), and last the particles
//! (`_PostDrawCallback`'s `_DrawParticlesOffscreen`). See
//! [`nova_sim::sprite_layer`] for the addresses.
//!
//! Left out:
//!
//! - the order within the boxes' layer: the engine draws the cargo boxes
//!   in their table's order (`Session::cargo_boxes`), the original in
//!   slot order (rdm:task/cargo-box-slot-order); the explosions, smoke
//!   puffs and escape pods draw in slot order;
//! - `_FloaterDrawCallback`'s guards (@0x38b34-0x38b4c): it draws nothing
//!   while switched out (`0x3b50e0`), which the engine's flight has no
//!   state for, or while the world's byte +0xe8 is 0, `SpriteWorld`
//!   bookkeeping the engine has no need of, drawing every frame whole;
//! - the message rect's OR (@0x38b52-0x38b90) is drawn as the message's
//!   text over the sprites, its box staying under them: a system
//!   `BkgndColor` other than black, which the OR lays over the sprites in
//!   the rect, is left out
//!   (rdm:phase/play-feel-followups/phase-3-message-rect-background-ored-over-sprites);
//! - the escort menu's fade on closing (level 32 − `0x3b51a0`,
//!   @0x38c0f-0x38c5b), filed in rdm:task/escorts-follow-up;
//! - the escort menu showing a change a frame late, as the callback blits
//!   the menu's frame before redrawing it (@0x38c60-0x38c77)
//!   (rdm:phase/play-feel-followups/phase-1-escort-menu-redraw-a-frame-late);
//! - every world sprite but the cursor drawing past the play area into
//!   the status bar's column, where the sprite world ends 194 short of
//!   the window (@0x21b54-0x21ba4)
//!   (rdm:phase/play-feel-followups/phase-2-flight-world-clipped-to-play-area).

use nova_sim::sprite_layer::{SpriteLayer, WORLD_PASSES, WorldPass};
use nova_sim::{RuleKey, RuleSource, Session, Vec2};

use crate::flight::alert;
use crate::flight::asteroids;
use crate::flight::boxes;
use crate::flight::hud::{self, STATUS_BAR_WIDTH};
use crate::flight::weapons;
use crate::geometry::Bounds;
use crate::system::camera::Camera;
use crate::system::scene::{self, SystemScene};
use crate::{Color, DrawList, Point};

use super::FlightView;
use super::stars;

impl<C> FlightView<C> {
    /// Draws the sprite world, pass by pass in [`WORLD_PASSES`]' order
    /// (see the module docs). The message line's box is drawn over the
    /// stars and its text with the floaters; the text, the floating map's
    /// and the escort menu's included, is overlay among the world
    /// ([`DrawList::world`]).
    pub(super) fn draw_world(&self, list: &mut DrawList, scene: &SystemScene, camera: &Camera) {
        let beams = self.shown_beams(camera);
        let zig_zag = self.session.as_ref().map_or(RuleSource::Engine, |session| {
            session.rules().source_for(RuleKey::LightningZigZag)
        });
        let lightning = (zig_zag, self.shot_looks.lightning());
        let stacked = self.sprites_stacked(camera);
        let npcs = self.shown_npcs();
        for pass in WORLD_PASSES {
            match pass {
                WorldPass::Layer(SpriteLayer::Stars) => self.draw_stars_and_message(list, camera),
                WorldPass::Layer(SpriteLayer::Stellars) => {
                    scene::draw_stellars(list, scene, camera);
                }
                WorldPass::BeamsUnderShips => {
                    weapons::draw_beams(list, &beams, &self.looks, true, lightning);
                }
                WorldPass::Layer(SpriteLayer::Asteroids) => {
                    asteroids::draw_asteroids(list, &self.shown_asteroids(camera), &self.looks);
                }
                WorldPass::Layer(SpriteLayer::Boxes) => {
                    boxes::draw_boxes(list, &self.shown_boxes(camera), &self.looks);
                    self.effects.draw_pods(list, camera, &self.looks);
                }
                WorldPass::Layer(SpriteLayer::Smoke) => self.draw_smoke(list, camera),
                WorldPass::Layer(SpriteLayer::Explosions) => self.draw_explosions(list, camera),
                WorldPass::Layer(SpriteLayer::Targets) => self.draw_targets(list, scene, camera),
                WorldPass::Floaters => self.draw_floaters(list),
                WorldPass::BeamsOverShips => {
                    weapons::draw_beams(list, &beams, &self.looks, false, lightning);
                }
                WorldPass::Particles => self.effects.draw_particles(list, camera),
                WorldPass::Layer(
                    layer @ (SpriteLayer::Disabled
                    | SpriteLayer::PointDefence
                    | SpriteLayer::Homing
                    | SpriteLayer::Shots
                    | SpriteLayer::Led
                    | SpriteLayer::Ships
                    | SpriteLayer::OverShips),
                ) => self.draw_sprite_layer(list, camera, &stacked, &npcs, layer),
            }
        }
    }

    /// Draws the target layer's sprites in `_InitSprites`' slot order: the
    /// nav brackets and then the target brackets (@0x20093-0x20277), the
    /// cursor (@0x2027d-0x202f9) unless the course map is open, which
    /// hides it, clipped to the play area ([`FlightView::play_rect`]),
    /// the red-alert icon (@0x202fe-0x2038b) and the double-time icon
    /// (@0x20390-0x203fd).
    fn draw_targets(&self, list: &mut DrawList, scene: &SystemScene, camera: &Camera) {
        self.draw_brackets(list, scene, camera);
        if !self.map_open {
            self.cursor.draw(list, &self.looks, self.play_rect());
        }
        self.draw_red_alert(list);
        if self.session.as_ref().is_ok_and(Session::double_time) {
            alert::draw_double_time(list);
        }
    }

    /// The play area on screen: the view less the status bar's
    /// [`STATUS_BAR_WIDTH`] at the right, where `_SetupSpriteWorld`'s
    /// sprite world ends (`subw $0xc2` @0x21b66, passed as the world's and
    /// the offscreen rect @0x21b7c-0x21ba4).
    pub(super) fn play_rect(&self) -> Bounds {
        Bounds {
            min: Point::new(0.0, 0.0),
            max: Point::new(self.view.0 - STATUS_BAR_WIDTH, self.view.1),
        }
    }

    /// Draws `_FloaterDrawCallback`'s floaters (0x38b2d), which
    /// `_PostDrawCallback` draws after every layer and before the
    /// particles (@0x38c8d-0x38c9a): the message line's text, as the
    /// callback ORs the background frame's message rect over the sprites
    /// (@0x38b52-0x38b90; its box stays under the sprites, in
    /// [`FlightView::draw_stars_and_message`]), then the floating map
    /// (@0x38b95-0x38c0a), then the escort menu (@0x38c0f-0x38c5b).
    ///
    /// The text of all three (the message, the floating map's names, the
    /// escort menu's title, rows and orders) is overlay
    /// ([`DrawList::world`]). Drawn at the window's resolution every
    /// command draws in list order, so the text stays under the
    /// particles; drawn into a world texture under a frame upscaler, the
    /// world is laid out before the overlay and the text draws over the
    /// particles (rdm:phase/play-feel-cross-roadmap/phase-3-floater-text-under-the-particles-upscaled).
    fn draw_floaters(&self, list: &mut DrawList) {
        let metrics = self.metrics.as_ref().map(|metrics| &*metrics.0);
        if let Some(message) = self.message() {
            hud::draw_message_text(list, message, self.view, metrics);
        }
        self.draw_floating_map(list);
        self.escort_menu
            .draw(list, &self.escort_rows(), self.escort_colors, metrics);
    }

    /// Draws the stars, then the message line's box over them, erased in
    /// the system's background ([`message_background`]), as `_DisplayComm`
    /// erases the background frame's message rect. Its text is drawn
    /// with the floaters ([`FlightView::draw_floaters`]).
    fn draw_stars_and_message(&self, list: &mut DrawList, camera: &Camera) {
        let source = self.session.as_ref().map_or(RuleSource::Engine, |session| {
            session.rules().source_for(RuleKey::FlightStarfield)
        });
        stars::draw(list, (&self.stars, &self.looks), source, camera, self.alpha);
        if self.message().is_some() {
            let background = message_background(
                self.session
                    .as_ref()
                    .ok()
                    .and_then(Session::system_background),
            );
            hud::draw_message_box(list, self.view, background);
        }
    }

    /// Draws the smoke puffs, murked as the player sees them; none without
    /// a session.
    fn draw_smoke(&self, list: &mut DrawList, camera: &Camera) {
        if let Ok(session) = &self.session {
            let viewer = session.player().position;
            let murk = session.effective_murk();
            self.shot_looks.draw_smoke(list, camera, viewer, murk);
        }
    }

    /// Draws the explosions, murked as the player sees them; unmurked, as
    /// seen from the camera's centre, without a session.
    fn draw_explosions(&self, list: &mut DrawList, camera: &Camera) {
        let (viewer, murk) = self.session.as_ref().map_or_else(
            |_| {
                let centre = camera.center();
                (Vec2::new(centre.x, centre.y), 0)
            },
            |session| (session.player().position, session.effective_murk()),
        );
        self.effects
            .draw_explosions(list, camera, &self.looks, viewer, murk);
    }
}

/// The colour the flight message's box is erased in: the player's
/// system's `BkgndColor` (`background`, raw `00RRGGBB`) as the original's
/// thousands-of-colours display shows it, or black with no system or no
/// session (`_SetSystemBackgroundBackColor` @0x6e06-0x6e46, `BackColor`
/// 33 @0x6e1c).
fn message_background(background: Option<u32>) -> Color {
    background.map_or(Color::BLACK, Color::from_rgb24_as_rgb555)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_message_background_is_the_systems_at_rgb555_or_black_without_one() {
        assert_eq!(
            message_background(Some(0x0007_0F12)),
            Color::rgba(0x00, 0x08, 0x10, 255)
        );
        assert_eq!(message_background(None), Color::BLACK);
    }
}
