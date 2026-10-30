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
//! - the message rect's OR (@0x38b52-0x38b90) is drawn as a fill of its
//!   `BkgndColor` OR-ed over the sprites and the message's text painted
//!   over that, its box staying under the sprites: the text is white,
//!   and white OR-ed with anything is white, so only the engine's
//!   antialiased glyph edges differ from an OR of it;
//! - the escort menu's fade on closing (level 32 − `0x3b51a0`,
//!   @0x38c0f-0x38c5b), filed in rdm:task/escorts-follow-up;
//! - the escort menu showing a change a frame late, as the callback blits
//!   the menu's frame before redrawing it (@0x38c60-0x38c77)
//!   (rdm:phase/play-feel-followups/phase-1-escort-menu-redraw-a-frame-late).

use nova_sim::Vec2;
use nova_sim::sprite_layer::{SpriteLayer, WORLD_PASSES, WorldPass};

use crate::flight::alert;
use crate::flight::asteroids;
use crate::flight::boxes;
use crate::flight::hud::{self, STATUS_BAR_WIDTH};
use crate::flight::sim_port::FlightSim;
use crate::flight::weapons;
use crate::geometry::Bounds;
use crate::system::camera::Camera;
use crate::system::scene::{self, SystemScene};
use crate::{Color, DrawList, Point};

use super::stars;
use super::{FlightView, HELP, HELP_AT, OVERLAY_SIZE, TITLE, TITLE_SIZE};

impl<C, S: FlightSim> FlightView<C, S> {
    /// Draws the sprite world over the scene laid out
    /// ([`FlightView::draw_world`]), then the system's title and the help
    /// line when their overlays are on: whether there was a scene to draw.
    pub(super) fn draw_world_scene(&self, list: &mut DrawList) -> bool {
        let Some(scene) = &self.scene else {
            return false;
        };
        let camera = self.camera();
        list.world_in(self.play_rect(), |list| {
            self.draw_world(list, scene, &camera);
        });
        if self.overlays.title {
            list.text(
                format!("{} (sÿst {})", scene.name(), scene.id().0),
                TITLE,
                TITLE_SIZE,
                None,
                Color::WHITE,
            );
        }
        self.draw_help(list);
        true
    }

    /// Draws the scene and its HUD from the kept frame, the fades left
    /// out: whether there was a scene to draw, or only why flight could
    /// not start (`reason`, asked only then). The sprite world goes
    /// first ([`FlightView::draw_world_scene`]), then the status bar over
    /// it ([`FlightView::draw_bar`]), or why it can't be shown.
    pub(super) fn draw_flight(&self, list: &mut DrawList, reason: impl FnOnce() -> String) -> bool {
        if !self.draw_world_scene(list) {
            list.text(
                format!("Cannot start flight: {}", reason()),
                TITLE,
                TITLE_SIZE,
                None,
                Color::ERROR,
            );
            self.draw_help(list);
            return false;
        }
        match &self.status_bar {
            Ok(bar) => self.draw_bar(list, (bar, self.view.0), &self.frame),
            Err(reason) => hud::draw_unavailable(list, reason, self.view.0),
        }
        true
    }

    /// The message on screen, if any: the slot's until its ticks have
    /// passed.
    #[must_use]
    pub fn message(&self) -> Option<&str> {
        let message = self.message.as_ref()?;
        let shown = crate::time::ticks(self.elapsed) - message.said_at;
        (shown < u128::from(message.ticks)).then_some(message.text.as_str())
    }

    /// Draws the [`HELP`] line when its overlay is on.
    pub(super) fn draw_help(&self, list: &mut DrawList) {
        if self.overlays.help {
            list.text(HELP, HELP_AT, OVERLAY_SIZE, None, Color::DIM);
        }
    }

    /// Draws the sprite world, pass by pass in [`WORLD_PASSES`]' order
    /// (see the module docs), from the kept frame, its brackets, icon and
    /// escort rows from the frame's HUD
    /// ([`FlightFrame::hud`](nova_sim::flight_frame::FlightFrame::hud)).
    /// The message line's box is drawn over the stars and its text with
    /// the floaters; the text, the floating map's and the escort menu's
    /// included, is overlay among the world ([`DrawList::world`]).
    pub(super) fn draw_world(&self, list: &mut DrawList, scene: &SystemScene, camera: &Camera) {
        let beams = self.shown_beams(camera);
        let lightning = (self.presentation.zig_zag, self.shot_looks.lightning());
        let stacked = self.sprites_stacked(camera);
        let npcs = self.shown_npcs();
        for pass in WORLD_PASSES {
            match pass {
                WorldPass::Layer(SpriteLayer::Stars) => self.draw_stars_and_message(list, camera),
                WorldPass::Layer(SpriteLayer::Stellars) => {
                    let seen = (self.frame.player().position, self.frame.effective_murk);
                    scene::draw_stellars(list, scene, camera, Some(seen));
                }
                WorldPass::BeamsUnderShips => {
                    weapons::draw_beams(list, &beams, &self.looks, true, lightning);
                }
                WorldPass::Layer(SpriteLayer::Asteroids) => {
                    asteroids::draw_asteroids(list, &self.shown_asteroids(camera), &self.looks);
                }
                WorldPass::Layer(SpriteLayer::Boxes) => {
                    boxes::draw_boxes(list, &self.shown_boxes(camera), &self.looks);
                    self.draw_pods(list, camera);
                }
                WorldPass::Layer(SpriteLayer::Smoke) => self.draw_smoke(list, camera),
                WorldPass::Layer(SpriteLayer::Explosions) => self.draw_explosions(list, camera),
                WorldPass::Layer(SpriteLayer::Targets) => {
                    self.draw_targets(list, (scene, camera));
                }
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
    /// (@0x20390-0x203fd), while the frame says double time is on: the
    /// brackets and the icon as the frame's HUD holds them.
    fn draw_targets(&self, list: &mut DrawList, at: (&SystemScene, &Camera)) {
        let hud = &self.frame.hud;
        self.draw_brackets(list, at, hud.nav_brackets, hud.target_brackets);
        if !self.map_open {
            self.cursor.draw(list, &self.looks, self.play_rect());
        }
        if let (Some(icon), Some(width)) = (hud.red_alert, self.alert_width) {
            alert::draw_red_alert(list, icon, width, self.view.0);
        }
        if self.frame.double_time {
            alert::draw_double_time(list);
        }
    }

    /// The play area on screen: the view less the status bar's
    /// [`STATUS_BAR_WIDTH`] at the right, where `_SetupSpriteWorld`'s
    /// sprite world ends (`subw $0xc2` @0x21b66, passed as the world's and
    /// the offscreen rect @0x21b7c-0x21ba4). The whole world is clipped
    /// to it ([`DrawList::world_in`]), as the sprite world's frame ends
    /// there: no sprite, line or dot of the world draws into the column.
    pub(super) fn play_rect(&self) -> Bounds {
        Bounds {
            min: Point::new(0.0, 0.0),
            max: Point::new(self.view.0 - STATUS_BAR_WIDTH, self.view.1),
        }
    }

    /// Draws `_FloaterDrawCallback`'s floaters (0x38b2d), which
    /// `_PostDrawCallback` draws after every layer and before the
    /// particles (@0x38c8d-0x38c9a): the message line, as the callback
    /// ORs the background frame's message rect over the sprites
    /// (@0x38b52-0x38b90): its background OR-ed in
    /// ([`hud::or_message_background`]) and its text over that (its box
    /// stays under the sprites, in
    /// [`FlightView::draw_stars_and_message`]), then the floating map
    /// (@0x38b95-0x38c0a), then the escort menu with the frame's rows
    /// (@0x38c0f-0x38c5b).
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
            let background = message_background(self.frame.system_background);
            hud::or_message_background(list, self.view, background);
            hud::draw_message_text(list, message, self.view, metrics);
        }
        self.draw_floating_map(list);
        self.escort_menu.draw(
            list,
            &self.frame.hud.escort_rows,
            self.escort_colors,
            metrics,
        );
    }

    /// Draws the stars, by the flight's reading of
    /// [`RuleKey::FlightStarfield`](nova_sim::RuleKey::FlightStarfield),
    /// then the message line's box over them, erased in the system's
    /// background ([`message_background`]), as `_DisplayComm` erases the
    /// background frame's message rect. Its text is drawn with the
    /// floaters ([`FlightView::draw_floaters`]).
    fn draw_stars_and_message(&self, list: &mut DrawList, camera: &Camera) {
        let source = self.presentation.starfield;
        stars::draw(list, (&self.stars, &self.looks), source, camera, self.alpha);
        if self.message().is_some() {
            let background = message_background(self.frame.system_background);
            hud::draw_message_box(list, self.view, background);
        }
    }

    /// Draws the smoke puffs, murked as the player sees them where the
    /// frame has its ship.
    fn draw_smoke(&self, list: &mut DrawList, camera: &Camera) {
        let viewer = self.frame.player().position;
        let murk = self.frame.effective_murk;
        self.shot_looks.draw_smoke(list, camera, viewer, murk);
    }

    /// Draws the escape pods, murked as the player sees them where the
    /// frame has its ship.
    fn draw_pods(&self, list: &mut DrawList, camera: &Camera) {
        let viewer = self.frame.player().position;
        let murk = self.frame.effective_murk;
        self.effects
            .draw_pods(list, camera, &self.looks, viewer, murk);
    }

    /// Draws the explosions, murked as the player sees them where the
    /// frame has its ship.
    fn draw_explosions(&self, list: &mut DrawList, camera: &Camera) {
        let viewer: Vec2 = self.frame.player().position;
        let murk = self.frame.effective_murk;
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
