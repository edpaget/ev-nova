//! A ship's hull and its layers as `_HandleShipDisplay` (@0x2b514-0x2dfa0)
//! draws them in the `EV Nova` executable, on the frames the session's
//! display picked (`nova_sim::combat::ship_frames`).
//!
//! - **The frames** ([`ShipDraw::draw`]): the hull at its set's frame,
//!   set × `FramesPer` + the rotation of the heading it is drawn at; the
//!   glow, lights and weapon layers at that frame mod their own frames
//!   (`_SWSetCurrentFrameIndex` @0xa90d4-0xa90ee); the shield layer by its
//!   own rule ([`FrameSpec::shield_frame`]); the alt layer at its own set's
//!   frame, with the hull's tint (@0x2caa1-0x2cb0a), while the session
//!   draws it ([`ShipDraw::live_alt`]), or as it was last drawn while the
//!   session holds it ([`held_alt`](super::held_alt)).
//! - **The hull's translucency** ([`base_alpha`]): `BaseTransp` t
//!   (@0x2bdb2-0x2bdce), its channel factors 0 unpainted, draws as
//!   `(src × (32 − t)) >> 5 | (dst × t) >> 5`
//!   (`_BlitPixieRLETranslucent` @0xc1568, `_BlitPixieTranslucentCopy`
//!   @0xc1110), a crossfade the view draws as a normal blend at
//!   (32 − t)/32, as it draws an escape pod's fade. A gate's fade and a
//!   cloak write the hull's translucency after it, and win.
//! - **Murk**: an NPC's hull takes its sprite murk m
//!   ([`sprite_murk`](super::murk::sprite_murk), @0x2be21-0x2be3a), and so
//!   do its glow, weapon glow, alt and shield layers; its lights take
//!   trunc(m × 0.333) ([`lights_murk`], @0x2c5df-0x2c60c). The glow, lights
//!   and weapon glow are held to trunc(32 − 1.5 × their murk)
//!   ([`npc_glow_level`]); the shield is not (@0x2c91b-0x2c939). The
//!   player's murk is 0.
//! - **The lights**: drawn at the level the session's display drew them
//!   ([`Drawn::lights`], `nova_sim::combat::ship_lights`), which is none
//!   while a disabled ship of `shän` `Flags` 0x0040 has them out, the
//!   player too (@0x2c56c-0x2c593).
//! - **The ion tint** ([`ShipDraw::colorize`], see
//!   [`ion_tint`](super::ion_tint)): an ionized ship's hull is colorized
//!   in place of its murk, drawn at its unmurked tint, and the layers that
//!   copy the hull's colorize after the tint block take it in place of
//!   theirs: the alt layer (@0x2caa1-0x2cb0a) and the shield
//!   (@0x2c91b-0x2c939), the player's and an NPC's; an NPC's glow
//!   (@0x2c233-0x2c250) and weapon glow (@0x2c723-0x2c740) at the hull's
//!   level, and its lights at trunc(level × 0.333) (@0x2c5df-0x2c60c),
//!   each held below that level as below a murk
//!   (@0x2c25a-0x2c291, @0x2c616-0x2c64d, @0x2c74a-0x2c781). The
//!   player's glow, lights and weapon glow are never colorized
//!   (@0x2c209, @0x2c5d1, @0x2c715).
//! - **A gate's colorize** ([`gate_colorize`](super::gate_colorize)): an
//!   NPC entering or leaving a gate takes the gate's white in place of
//!   its ion tint and murk, and its layers copy it as they copy the tint.
//! - **A cloak** ([`ShipDraw::cloak`], see [`cloak_fade`](super::cloak_fade)):
//!   its hull and alt take the cloak's level and blend in place of their
//!   tint and murk, they, the glow, the lights and the weapon glow move
//!   by its jitter, and the layers are held to its caps or hidden.
//!
//! Each layer is added, [`Blend::Or`](crate::draw::Blend::Or) at
//! [`lights_tint`] of its level; the hull and the alt layer are drawn
//! plainly. Each one whose sheet holds solid runs draws them raw right
//! after itself ([`solid_runs`](super::solid_runs)), as the original's
//! blits fill a run over every blend and colorize.

use nova_sim::HashedRolls;
use nova_sim::combat::ship_frames::{Alt, Drawn, FrameSpec};
use nova_sim::traffic::npc::NpcId;

use super::catalog::{LayerSheet, ShipSheet};
use super::cloak_fade::CloakDraw;
use super::held_alt::AltLayer;
use super::murk::murk_tint;
use super::solid_runs::solid_overlay;
use super::weapon_glow::npc_glow_level;
use crate::draw::lights_tint;
use crate::{Blend, Color, Colorize, DrawList, ImageKey, Point};

/// The steps of a sprite's translucency: `BaseTransp` 32 or more is not
/// seen.
pub const TRANSLUCENCY_STEPS: i16 = 32;

/// What an NPC's sprite murk is multiplied by for its lights (the double
/// 0.333 @0xdd0c8).
pub const LIGHTS_MURK_SHARE: f64 = 0.333;

/// The alpha a hull of `BaseTransp` `base_transp` is drawn at: full at 0
/// or less, (32 − t)/32 of full, rounded to nearest, for t from 1 to 31,
/// and none from 32.
#[must_use]
pub fn base_alpha(base_transp: i16) -> u8 {
    let left = TRANSLUCENCY_STEPS - base_transp.clamp(0, TRANSLUCENCY_STEPS);
    // At most 255 once scaled.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let alpha = ((i32::from(left) * 255 + 16) / i32::from(TRANSLUCENCY_STEPS)) as u8;
    alpha
}

/// An NPC's lights' murk at a sprite murk of `murk`: trunc(murk × 0.333).
#[must_use]
pub fn lights_murk(murk: u8) -> u8 {
    // Within 0..=10 for a murk of at most 31.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let level = (f64::from(murk) * LIGHTS_MURK_SHARE).trunc() as u8;
    level
}

/// The rolls NPC `id`'s engine glow flickers on: its own, seed 2 + 2·id,
/// so no two ships flicker alike, nor any like the player's (seed 1).
#[must_use]
pub fn npc_glow_rolls(id: NpcId) -> HashedRolls {
    HashedRolls::new(2 + 2 * u64::from(id.0))
}

/// One ship as it is drawn this frame (see the module docs).
#[derive(Clone, Copy, Debug)]
pub struct ShipDraw<'a> {
    /// Its sheet.
    pub sheet: &'a ShipSheet,
    /// Its class's display.
    pub spec: &'a FrameSpec,
    /// The sets its display picked.
    pub drawn: Drawn,
    /// The heading it is drawn at.
    pub heading: f32,
    /// Its hull's tint before its murk: white, at the alpha its fade,
    /// cloak or `BaseTransp` leaves.
    pub tint: Color,
    /// How its cloak changes its drawing, if it does: its hull's tint and
    /// blend, its jitter, and its layers' caps
    /// ([`CloakDraw`], see [`cloak_fade`](super::cloak_fade)).
    pub cloak: Option<CloakDraw>,
    /// Whether its hull, and so its alt layer, is drawn.
    pub hull: bool,
    /// Its sprite murk, 0 for the player.
    pub murk: u8,
    /// Whether its glow, lights and weapon glow are held below their murk:
    /// an NPC's.
    pub held: bool,
    /// Its engine glow's level, if shown.
    pub glow: Option<u8>,
    /// Its lights' level, if shown: its display's ([`Drawn::lights`]).
    pub lights: Option<u8>,
    /// Its weapon glow's level, if lit.
    pub weapon: Option<u8>,
    /// Its shield flash's level, if shown.
    pub shield: Option<u8>,
    /// Its hull's colorize, if it shows one, which takes the place of its
    /// murk: its ion tint's
    /// ([`hull_colorize`](super::ion_tint::hull_colorize)), or, for an NPC
    /// at a gate, the gate's white, written after it
    /// ([`gate_colorize`](super::gate_colorize::gate_colorize)).
    pub colorize: Option<Colorize>,
}

/// Draws `image` at `at` at `tint`, combined by `blend`, colorized by
/// `colorize` when it mixes.
fn draw_layer(
    list: &mut DrawList,
    (image, at, tint, blend): (ImageKey, Point, Color, Blend),
    colorize: Option<Colorize>,
) {
    match colorize.filter(|colorize| colorize.mixes()) {
        Some(colorize) => list.colorized_sprite(image, at, tint, blend, colorize),
        None => list.push(crate::DrawCommand::Sprite {
            image,
            center: at,
            tint,
            blend,
        }),
    };
}

/// Class `spec`'s display turning by `sheet`'s rotations, its `shän`'s
/// `FramesPer` as its sheet is laid out.
#[must_use]
pub fn turning(spec: &FrameSpec, sheet: &ShipSheet) -> FrameSpec {
    FrameSpec {
        frames: sheet.rotations,
        ..*spec
    }
}

impl ShipDraw<'_> {
    /// The frame its hull is drawn on.
    #[must_use]
    pub fn frame(&self) -> u16 {
        turning(self.spec, self.sheet).index(self.drawn.set, self.heading)
    }

    /// The hull's tint, colorize and blend: its unmurked tint colorized by
    /// its ion tint, or its murk, drawn plainly; or, cloaked, its cloak's
    /// tint and blend in place of its tint and murk, as the cloak writes
    /// the channel factors after them.
    fn hull_tint(&self) -> (Color, Option<Colorize>, Blend) {
        if let Some((tint, blend)) = self.cloak.and_then(|cloak| cloak.hull) {
            return (tint, self.colorize, blend);
        }
        match self.colorize {
            Some(colorize) => (self.tint, Some(colorize), Blend::Normal),
            None => (murk_tint(self.tint, self.murk), None, Blend::Normal),
        }
    }

    /// How far its cloak moves its hull and the layers that move with it.
    fn offset(&self) -> Point {
        self.cloak
            .map_or(Point::new(0.0, 0.0), |cloak| cloak.offset)
    }

    /// Its alt layer as drawn at `at` this frame, when its hull is drawn
    /// and the session draws the layer on a set: its own set's frame, with
    /// the hull's tint and colorize.
    #[must_use]
    pub fn live_alt(&self, at: Point) -> Option<AltLayer> {
        let (Some(layer), Alt::Drawn(set), true) = (self.sheet.alt, self.drawn.alt, self.hull)
        else {
            return None;
        };
        let frame = turning(self.spec, self.sheet).index(set, self.heading) % layer.frames.get();
        let (tint, colorize, blend) = self.hull_tint();
        let offset = self.offset();
        Some(AltLayer {
            image: ImageKey::sprite(layer.image_id, frame),
            at: Point::new(at.x + offset.x, at.y + offset.y),
            tint,
            colorize,
            blend,
        })
    }

    /// Draws it at `at`: its hull, glow, lights, weapon glow, alt layer
    /// `alt` (live or held, see [`ShipDraw::live_alt`]) and shield layer,
    /// in the order `_HandleShipDisplay` puts them in their layer
    /// (@0x2bd58-0x2bdad).
    pub fn draw(&self, list: &mut DrawList, at: Point, alt: Option<AltLayer>) {
        let frame = self.frame();
        let (hull_tint, colorize, hull_blend) = self.hull_tint();
        let shield_at = at;
        let offset = self.offset();
        let at = Point::new(at.x + offset.x, at.y + offset.y);
        if self.hull {
            let hull = ImageKey::sprite(self.sheet.image_id, frame);
            draw_layer(list, (hull, at, hull_tint, hull_blend), colorize);
            if self.sheet.solid {
                solid_overlay(list, self.sheet.image_id, frame, at);
            }
        }
        // A cloak holds the glow and lights to its cap, or hides them, and
        // the weapon glow to its own cap.
        let layers = |level: u8| match self.cloak {
            Some(cloak) => cloak.layers.map(|cap| level.min(cap)),
            None => Some(level),
        };
        let weapon_cap = self.cloak.map_or(u8::MAX, |cloak| cloak.weapon);
        // The level an NPC's layers are held below and colorized at: the
        // hull's ion tint's, or its murk. The player's layers take only its
        // murk (0), never its ion level, as the original draws the player's
        // glow, lights and weapon glow at level 0 (@0x2c209, @0x2c5d1,
        // @0x2c715).
        let murk = if self.held {
            colorize.map_or(self.murk, |colorize| colorize.level)
        } else {
            self.murk
        };
        let held = |level: u8, murk: u8| {
            if self.held {
                npc_glow_level(level, murk)
            } else {
                level
            }
        };
        // An NPC's glow, lights and weapon glow copy the hull's colorize at
        // `murk`; the player's never do.
        let copied = |murk: u8| {
            colorize.filter(|_| self.held).map(|colorize| Colorize {
                level: murk,
                ..colorize
            })
        };
        let add =
            |list: &mut DrawList, layer: Option<LayerSheet>, level: Option<u8>, murk, frame| {
                if let (Some(layer), Some(level)) = (layer, level) {
                    let frame = frame % layer.frames.get();
                    let image = ImageKey::sprite(layer.image_id, frame);
                    let (tint, colorize) = match copied(murk) {
                        Some(colorize) => (lights_tint(level), Some(colorize)),
                        None => (murk_tint(lights_tint(level), murk), None),
                    };
                    draw_layer(list, (image, at, tint, Blend::Or), colorize);
                    if layer.solid && level > 0 {
                        solid_overlay(list, layer.image_id, frame, at);
                    }
                }
            };
        let lights_murk = if self.held { lights_murk(murk) } else { 0 };
        let glow = self.glow.and_then(|level| layers(held(level, murk)));
        add(list, self.sheet.glow, glow, murk, frame);
        let lights = self
            .lights
            .and_then(|level| layers(held(level, lights_murk)));
        add(list, self.sheet.lights, lights, lights_murk, frame);
        let weapon = self.weapon.map(|level| held(level, murk).min(weapon_cap));
        add(list, self.sheet.weapon, weapon, murk, frame);
        let spec = turning(self.spec, self.sheet);
        if let Some(alt) = alt {
            draw_layer(list, (alt.image, alt.at, alt.tint, alt.blend), alt.colorize);
            let alt_sheet = self.sheet.alt;
            if alt_sheet.is_some_and(|layer| layer.solid && layer.image_id == alt.image.id) {
                solid_overlay(list, alt.image.id, alt.image.frame, alt.at);
            }
        }
        let shield = self
            .shield
            .filter(|_| self.cloak.is_none_or(|cloak| cloak.shield));
        if let (Some(layer), Some(level)) = (self.sheet.shield, shield) {
            let at = shield_at;
            let frame = spec.shield_frame(self.drawn.set, self.heading, layer.frames);
            let image = ImageKey::sprite(layer.image_id, frame);
            let tint = match colorize {
                Some(_) => lights_tint(level),
                None => murk_tint(lights_tint(level), self.murk),
            };
            draw_layer(list, (image, at, tint, Blend::Or), colorize);
            if layer.solid && level > 0 {
                solid_overlay(list, layer.image_id, frame, at);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hulls_base_transp_draws_it_at_32_less_it_32nds() {
        assert_eq!(base_alpha(0), 255);
        assert_eq!(base_alpha(-3), 255);
        assert_eq!(base_alpha(10), 175, "22/32");
        assert_eq!(base_alpha(16), 128);
        assert_eq!(base_alpha(31), 8);
        assert_eq!(base_alpha(32), 0);
        assert_eq!(base_alpha(40), 0);
    }

    #[test]
    fn each_npcs_glow_rolls_on_its_own_seed() {
        use nova_sim::BlinkChance;
        let of = |rolls: &HashedRolls| (0..8).map(|c| rolls.roll(c, 100)).collect::<Vec<_>>();
        assert_ne!(of(&npc_glow_rolls(NpcId(0))), of(&npc_glow_rolls(NpcId(1))));
        assert_eq!(
            of(&npc_glow_rolls(NpcId(4))),
            of(&HashedRolls::new(10)),
            "2 + 2·4"
        );
        assert_ne!(of(&npc_glow_rolls(NpcId(0))), of(&HashedRolls::new(1)));
    }

    #[test]
    fn an_npcs_lights_take_a_third_of_its_murk() {
        assert_eq!(lights_murk(0), 0);
        assert_eq!(lights_murk(3), 0, "0.999");
        assert_eq!(lights_murk(4), 1);
        assert_eq!(lights_murk(31), 10);
    }
}
