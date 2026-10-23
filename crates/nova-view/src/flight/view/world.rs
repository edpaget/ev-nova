//! The flight's sprite world, the scene under the HUD, drawn in the order
//! `_SetupSpriteWorld` builds it ([`WORLD_PASSES`]).
//!
//! Bottom first: the stars (and the message line), the stellars, the
//! beams under the ships (`Flags2` 0x2000, the stellar layer's
//! `_BeamDrawOffscreenCallback`), the asteroids, the disabled ships, the
//! cargo boxes and then the escape pods, the smoke, the point-defence,
//! homing and other shots, the ships the player leads, the other ships and
//! the player, the turret shots over the ships, the explosions, the target
//! layer, the other beams (the target layer's `_BeamDrawCallback`), and
//! last the particles (`_PostDrawCallback`'s `_DrawParticlesOffscreen`).
//! See [`nova_sim::sprite_layer`] for the addresses.
//!
//! Left out: the target layer's brackets, cursor, red-alert and
//! double-time icons, which the view draws in the overlay after the
//! world, over the beams and the particles, and the floating map and the
//! escort menu, which `_PostDrawCallback` draws before the particles
//! (rdm:phase/play-feel/phase-13-target-layer-and-floaters-in-the-world-order);
//! and the order within the explosions', smoke's, pods' and boxes' layers,
//! their slot order in the original
//! (rdm:phase/play-feel/phase-14-slot-order-within-layers).

use nova_sim::sprite_layer::{SpriteLayer, WORLD_PASSES, WorldPass};
use nova_sim::{RuleKey, RuleSource, Session};

use crate::flight::asteroids;
use crate::flight::boxes;
use crate::flight::hud;
use crate::flight::weapons;
use crate::system::camera::Camera;
use crate::system::scene::{self, SystemScene};
use crate::{Color, DrawList};

use super::FlightView;
use super::stars;

impl<C> FlightView<C> {
    /// Draws the sprite world, pass by pass in [`WORLD_PASSES`]' order
    /// (see the module docs). The message line, drawn over the stars, has
    /// its box filled in the system's background ([`message_background`]),
    /// in the world; its text is overlay among the world
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
                    scene::draw_stellars(list, scene, camera, self.elapsed);
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
                WorldPass::Layer(SpriteLayer::Explosions) => {
                    self.effects.draw_explosions(list, camera, &self.looks);
                }
                // The brackets, the cursor, the icons, the floating map
                // and the escort menu are drawn in the overlay, after the
                // world (see the module docs).
                WorldPass::Layer(SpriteLayer::Targets) | WorldPass::Floaters => {}
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

    /// Draws the stars, then the message line over them.
    fn draw_stars_and_message(&self, list: &mut DrawList, camera: &Camera) {
        let source = self.session.as_ref().map_or(RuleSource::Engine, |session| {
            session.rules().source_for(RuleKey::FlightStarfield)
        });
        stars::draw(list, (&self.stars, &self.looks), source, camera, self.alpha);
        let metrics = self.metrics.as_ref().map(|metrics| &*metrics.0);
        if let Some(message) = self.message() {
            let background = message_background(
                self.session
                    .as_ref()
                    .ok()
                    .and_then(Session::system_background),
            );
            hud::draw_message(list, message, self.view, background, metrics);
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
