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
//!   (@0x2bdb2-0x2bdce), its channel factors `paint_c − 32`, 0 unpainted,
//!   draws as `(src × (32 − t)) >> 5 | (dst × t) >> 5` unpainted
//!   (`_BlitPixieRLETranslucent` @0xc1568, `_BlitPixieTranslucentCopy`
//!   @0xc1110), a crossfade the view draws as a normal blend at
//!   (32 − t)/32, as it draws an escape pod's fade. A gate's fade and a
//!   cloak write the hull's translucency after it, and win.
//! - **The paint** ([`paint`](super::paint)): a painted hull, and its alt
//!   layer, draw `(src_c × (paint_c − t)) >> 5 | (dst_c × t) >> 5` in
//!   each channel, [`Blend::OrTranslucent`] at the paint's tint, its
//!   colorize mixing the texel first, as an unpainted one's. The other layers are
//!   never painted (@0x2be41-0x2beb2, @0x2caa1-0x2cb0a).
//! - **Murk**: an NPC's hull is colorized at its sprite murk m toward the
//!   system's colour ([`SeenMurk::at`](super::murk::SeenMurk::at),
//!   @0x2be21-0x2be3a), which fills the sprite's one colorize slot
//!   ([`ShipDraw::colorize`]), and its glow, weapon glow, alt and shield
//!   layers copy it; its lights copy it at trunc(m × 0.333)
//!   ([`lights_murk`], @0x2c5df-0x2c60c). The glow, lights and weapon glow
//!   are held to trunc(32 − 1.5 × their murk) ([`npc_glow_level`]); the
//!   shield is not (@0x2c91b-0x2c939). The player's murk is 0.
//! - **The lights**: drawn at the level the session's display drew them
//!   ([`Drawn::lights`], `nova_sim::combat::ship_lights`), which is none
//!   while a disabled ship of `shän` `Flags` 0x0040 has them out, the
//!   player too (@0x2c56c-0x2c593).
//! - **The ion tint** ([`ShipDraw::colorize`], see
//!   [`ion_tint`](super::ion_tint)): an ionized ship's hull is colorized
//!   in place of its murk, and the layers that copy the hull's colorize
//!   after the tint block take it in place of theirs: the alt layer (@0x2caa1-0x2cb0a) and the shield
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
//!   tint and keep their colorize, as the cloak writes +0xa8 and the
//!   channel factors and leaves +0xb0 (@0x2d1d6-0x2d22a). They, the glow,
//!   the lights and the weapon glow move by its jitter, and the layers
//!   are held to its caps or hidden.
//!
//! - **An 8-bit sheet** ([`EightBitBlit`]): a hull, alt or layer whose
//!   `rlëD` is 8-bit goes through the draw proc's depth-8 dispatch with
//!   its sprite's fields: the hull's translucency (its cloak's
//!   [`CloakDraw::hull_translucency`], or `BaseTransp`'s or a gate fade's)
//!   and colorize level, its factors never all 0x20; the alt copies the
//!   hull's state, its result kept while held ([`AltLayer::shown`]); the
//!   glow, lights, weapon glow and shield at 0x20, their factors at their
//!   level and their bias their colorize's. It draws plainly, as a 16-bit
//!   sheet would, or not at all, its solid runs included.
//!
//! - **The world's edge** ([`BoxCut`]): the hull and each layer are
//!   sprites of their own, so each is cut at the world's edge by its own
//!   box and sheet, as `_BlitPixieRLETranslucentDrawProc`
//!   (@0xbb3fc-0xbba11) clips each sprite's box to the world and takes the
//!   clipped blit for any cut (@0xbb56d-0xbb579, @0xbb770-0xbb789): an
//!   overhanging sheet cut to its box, a sheet with clip quirks drawing the
//!   clipped blit's frame. Its solid runs take its decision. A held alt
//!   of a sheet the ship no longer has is drawn whole.
//!
//! - **Its images** ([`SheetImage`](crate::SheetImage)): the hull and each
//!   layer draw their `rlëD`'s frames, or, for an image cut from its `PICT`
//!   pair (`nova_data::ShipImage`), the ship's frames of that image
//!   ([`ImageKey::ship_sheet`](crate::ImageKey::ship_sheet)). A pair has no
//!   clipped frame and no runs. A held alt takes its sheet's cut only when
//!   the whole key matches.
//!
//! Each layer is added, [`Blend::Or`](crate::draw::Blend::Or) at
//! [`lights_tint`] of its level; the hull and the alt layer are drawn
//! plainly, or in their paint. Each one whose sheet holds solid runs draws them raw right
//! after itself ([`solid_runs`](super::solid_runs)), as the original's
//! blits fill a run over every blend and colorize.

use nova_sim::combat::ship_frames::{Alt, Drawn, FrameSpec};
use nova_sim::traffic::npc::NpcId;
use nova_sim::{HashedRolls, Rgb15};

use super::catalog::{LayerSheet, ShipSheet};
use super::cloak_fade::CloakDraw;
use super::held_alt::AltLayer;
use super::paint::{painted_hull, translucency};
use super::solid_runs::solid_overlay;
use super::weapon_glow::npc_glow_level;
use crate::box_cut::BoxCut;
use crate::draw::lights_tint;
use crate::eight_bit_blit::{DrawProcState, EightBitBlit, Look};
use crate::{Blend, Color, Colorize, DrawList, Point};

/// Where a ship's sprite of a frame's size (width, height) is centred on
/// screen: placed at whole pixels from the player, as the original places
/// each of a ship's sprites (see [`ShipDraw::draw`]).
pub type Place<'a> = &'a dyn Fn((u32, u32)) -> Point;

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

/// How a glow, lights, weapon glow or shield `layer` at `level`, whose
/// 16-bit frame draws as `look`, is drawn: its sprite's +0xa8 is 0x20 and
/// each channel factor its level (glow @0x2c2b6-0x2c2d6, lights
/// @0x2c672-0x2c692, weapon glow @0x2c7a6-0x2c7c6, shield
/// @0x2c87c-0x2c914), its bias `look`'s colorize's level, so an 8-bit
/// sheet draws only at level 32 uncolorized, the add-over
/// ([`EightBitBlit`]).
fn layer_look(layer: LayerSheet, level: u8, look: Look) -> Option<Look> {
    let state = DrawProcState {
        translucency: LAYER_TRANSLUCENCY,
        full_factors: i16::from(level) == LAYER_TRANSLUCENCY,
        bias: DrawProcState::bias_of(look.1),
    };
    EightBitBlit::of_sheet(layer.eight_bit, state).applied(look)
}

/// The translucency the glow, lights, weapon glow and shield write to their
/// sprites in the 16-bit world (+0xa8 = 0x20, @0x2c2b6, @0x2c672, @0x2c7a6,
/// @0x2c87c).
const LAYER_TRANSLUCENCY: i16 = 0x20;

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
    /// Its hull's tint, unmurked: white, at the alpha its fade,
    /// cloak or `BaseTransp` leaves.
    pub tint: Color,
    /// Its paint, `None` unpainted: the hull and alt layer are drawn in it
    /// at the translucency [`tint`](Self::tint)'s alpha gives
    /// ([`painted_hull`]), unless a cloak writes their factors.
    pub paint: Option<Rgb15>,
    /// How its cloak changes its drawing, if it does: its hull's tint and
    /// blend, its jitter, and its layers' caps
    /// ([`CloakDraw`], see [`cloak_fade`](super::cloak_fade)).
    pub cloak: Option<CloakDraw>,
    /// Whether its hull, and so its alt layer, is drawn.
    pub hull: bool,
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
    /// Its hull's colorize, the sprite's one slot (+0xb0/+0xb4): for an
    /// NPC, its murk's ([`SeenMurk::at`](super::murk::SeenMurk::at)), which
    /// its ion tint's replaces
    /// ([`hull_colorize`](super::ion_tint::hull_colorize)), or, at a gate,
    /// the gate's white, written after it
    /// ([`gate_colorize`](super::gate_colorize::gate_colorize)). The
    /// player's hull never murks, so it shows only its ion tint's.
    pub colorize: Option<Colorize>,
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

    /// The hull's tint, colorize and blend as a 16-bit sheet draws it: its
    /// unmurked tint drawn plainly, or, painted, in its paint at its
    /// translucency ([`painted_hull`]); or, cloaked, its cloak's tint and
    /// blend in place of its tint, as the cloak writes the channel factors
    /// and leaves the colorize (+0xb0 is not touched after @0x2cad1). Its
    /// colorize is [`ShipDraw::colorize`] in every case.
    fn hull_tint(&self) -> Look {
        if let Some((tint, blend)) = self.cloak.and_then(|cloak| cloak.hull) {
            return (tint, self.colorize, blend);
        }
        let (tint, blend) = match painted_hull(self.paint, translucency(self.tint.a)) {
            Some(painted) => (painted, Blend::OrTranslucent),
            None => (self.tint, Blend::Normal),
        };
        (tint, self.colorize, blend)
    }

    /// The hull's sprite fields that the draw proc's depth-8 dispatch
    /// reads, which the alt layer copies (@0x2caa6-0x2cb0a): its cloak's
    /// translucency, or else `BaseTransp`'s or a gate's fade's; and its
    /// colorize's level. Its channel factors are never all 0x20: they are
    /// paint − 32 (@0x2be41-0x2beb2), at most −1 for a paint of 31 or
    /// less and 0 unpainted, and a cloak's are at most 31 fading
    /// (@0x2d35f-0x2d440) and 2 fully faded.
    fn hull_state(&self) -> DrawProcState {
        let translucency = self.cloak.map_or_else(
            || i16::from(translucency(self.tint.a)),
            |cloak| cloak.hull_translucency,
        );
        DrawProcState {
            translucency,
            full_factors: false,
            bias: DrawProcState::bias_of(self.colorize),
        }
    }

    /// How a sheet that is `eight_bit` or not draws in the hull's state
    /// ([`ShipDraw::hull_state`]): at [`ShipDraw::hull_tint`]'s look,
    /// plainly, or not at all ([`EightBitBlit`]).
    fn hull_look(&self, eight_bit: bool) -> Option<Look> {
        EightBitBlit::of_sheet(eight_bit, self.hull_state()).applied(self.hull_tint())
    }

    /// `at` moved by its cloak's jitter, as its hull and the layers that
    /// move with it are.
    fn moved(&self, at: Point) -> Point {
        let offset = self
            .cloak
            .map_or(Point::new(0.0, 0.0), |cloak| cloak.offset);
        Point::new(at.x + offset.x, at.y + offset.y)
    }

    /// Its alt layer as drawn this frame, placed by `place` (see
    /// [`ShipDraw::draw`]) by its own frame's size, when its hull is drawn
    /// and the session draws the layer on a set: its own set's frame, with
    /// the hull's tint and colorize, through the depth-8 dispatch when its
    /// own sheet is 8-bit, in the hull's state.
    #[must_use]
    pub fn live_alt(&self, place: Place<'_>) -> Option<AltLayer> {
        let (Some(layer), Alt::Drawn(set), true) = (self.sheet.alt, self.drawn.alt, self.hull)
        else {
            return None;
        };
        let frame = turning(self.spec, self.sheet).index(set, self.heading) % layer.frames.get();
        let look = self.hull_look(layer.eight_bit);
        let (tint, colorize, blend) = look.unwrap_or_else(|| self.hull_tint());
        let at = self.moved(place(layer.size()));
        Some(AltLayer {
            image: layer.image.frame(frame),
            at,
            shown: look.is_some(),
            tint,
            colorize,
            blend,
        })
    }

    /// Draws it: its hull, glow, lights, weapon glow, alt layer `alt`
    /// (live or held, see [`ShipDraw::live_alt`]) and shield layer, in the
    /// order `_HandleShipDisplay` puts them in their layer
    /// (@0x2bd58-0x2bdad). Each is centred where `place` puts a sprite of
    /// its own frame's size, as the original places each sprite by its own
    /// `_GetSpriteXSize`/`_GetSpriteYSize` (@0x2cd17-0x2d083); all but the
    /// shield are then moved by the cloak's jitter, which `_OffsetRect`
    /// applies after the placement (@0x2d31e, @0x2d46b, @0x2d4e8,
    /// @0x2d5ae, @0x2d69e).
    pub fn draw(&self, list: &mut DrawList, place: Place<'_>, alt: Option<AltLayer>) {
        let frame = self.frame();
        let colorize = self.colorize;
        if let (true, Some((tint, hull_colorize, blend))) =
            (self.hull, self.hull_look(self.sheet.eight_bit))
        {
            let at = self.moved(place(self.sheet.size()));
            let cut = BoxCut::of(list.world_clip(), at, self.sheet.edge());
            let hull = self.sheet.image.frame(frame);
            cut.mixed_sprite(list, hull, at, (tint, blend, hull_colorize));
            if self.sheet.solid {
                solid_overlay(list, cut, self.sheet.image, frame, at);
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
        // hull's colorize's. The player's layers never take it, as the
        // original draws the player's glow, lights and weapon glow at level
        // 0 (@0x2c209, @0x2c5d1, @0x2c715).
        let murk = if self.held {
            colorize.map_or(0, |colorize| colorize.level)
        } else {
            0
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
                let (Some(layer), Some(level)) = (layer, level) else {
                    return;
                };
                let colorize = copied(murk);
                let Some((tint, colorize, blend)) =
                    layer_look(layer, level, (lights_tint(level), colorize, Blend::Or))
                else {
                    return;
                };
                let frame = frame % layer.frames.get();
                let image = layer.image.frame(frame);
                let at = self.moved(place(layer.size()));
                let cut = BoxCut::of(list.world_clip(), at, layer.edge());
                cut.mixed_sprite(list, image, at, (tint, blend, colorize));
                if layer.solid && level > 0 {
                    solid_overlay(list, cut, layer.image, frame, at);
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
        if let Some(alt) = alt.filter(|alt| alt.shown) {
            // A held alt of a sheet the ship no longer has is drawn whole:
            // its sheet isn't known. The sheet is matched by its whole key,
            // so an `rlëD` and a `PICT` pair of one ID never match.
            let alt_sheet = self
                .sheet
                .alt
                .filter(|layer| layer.image.frame(alt.image.frame) == alt.image);
            let cut = alt_sheet.map_or(BoxCut::WHOLE, |layer| {
                BoxCut::of(list.world_clip(), alt.at, layer.edge())
            });
            cut.mixed_sprite(list, alt.image, alt.at, (alt.tint, alt.blend, alt.colorize));
            if let Some(layer) = alt_sheet.filter(|layer| layer.solid) {
                solid_overlay(list, cut, layer.image, alt.image.frame, alt.at);
            }
        }
        let shield = self
            .shield
            .filter(|_| self.cloak.is_none_or(|cloak| cloak.shield));
        if let (Some(layer), Some(level)) = (self.sheet.shield, shield)
            && let Some((tint, colorize, blend)) =
                layer_look(layer, level, (lights_tint(level), colorize, Blend::Or))
        {
            let at = place(layer.size());
            let frame = spec.shield_frame(self.drawn.set, self.heading, layer.frames);
            let image = layer.image.frame(frame);
            let cut = BoxCut::of(list.world_clip(), at, layer.edge());
            cut.mixed_sprite(list, image, at, (tint, blend, colorize));
            if layer.solid && level > 0 {
                solid_overlay(list, cut, layer.image, frame, at);
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

    mod placement {
        use std::num::NonZeroU16;

        use nova_sim::combat::ship_frames::{Alt, Drawn, FrameSpec};

        use super::super::*;
        use crate::DrawCommand;
        use crate::flight::cloak_fade::CloakDraw;

        /// A layer `rlëD` `image_id` of `width` x `height` frames.
        fn layer(image_id: i16, (frame_width, frame_height): (u32, u32)) -> LayerSheet {
            LayerSheet {
                image: crate::SheetImage::Rled(image_id),
                frames: NonZeroU16::new(36).expect("non-zero"),
                frame_width,
                frame_height,
                image_size: (frame_width, frame_height),
                clip_quirks: false,
                solid: false,
                eight_bit: false,
            }
        }

        /// A 7 x 6 hull 1000 whose glow 1100, lights 1200, weapon glow
        /// 1300, alt 1400 and shield 1500 each have a frame of its own.
        fn sheet() -> ShipSheet {
            ShipSheet {
                image: crate::SheetImage::Rled(1000),
                rotations: NonZeroU16::new(36).expect("non-zero"),
                frame_width: 7,
                frame_height: 6,
                image_size: (7, 6),
                clip_quirks: false,
                glow: Some(layer(1100, (5, 4))),
                lights: Some(layer(1200, (6, 3))),
                weapon: Some(layer(1300, (7, 2))),
                weap_decay: 0,
                alt: Some(layer(1400, (4, 5))),
                shield: Some(layer(1500, (9, 8))),
                base_transp: 0,
                solid: false,
                eight_bit: false,
            }
        }

        /// A placement that tells sizes apart: a sprite of `(w, h)` is
        /// centred at (w, 100 h).
        #[allow(clippy::cast_precision_loss)]
        fn by_size((width, height): (u32, u32)) -> Point {
            Point::new(width as f32, 100.0 * height as f32)
        }

        /// Where each sprite of the ship, every layer lit, is centred,
        /// moved by `cloak`'s jitter.
        fn centres(cloak: Option<CloakDraw>) -> Vec<(i16, Point)> {
            let sheet = sheet();
            let spec = FrameSpec::NONE;
            let ship = ShipDraw {
                sheet: &sheet,
                spec: &spec,
                drawn: Drawn {
                    alt: Alt::Drawn(0),
                    ..Drawn::default()
                },
                heading: 0.0,
                tint: Color::WHITE,
                paint: None,
                cloak,
                hull: true,
                held: false,
                glow: Some(20),
                lights: Some(20),
                weapon: Some(20),
                shield: Some(20),
                colorize: None,
            };
            let mut list = DrawList::new();
            ship.draw(&mut list, &by_size, ship.live_alt(&by_size));
            list.iter()
                .filter_map(|command| match *command {
                    DrawCommand::Sprite { image, center, .. } => Some((image.id, center)),
                    _ => None,
                })
                .collect()
        }

        #[test]
        fn the_hull_and_each_layer_are_placed_by_their_own_frames() {
            assert_eq!(
                centres(None),
                [
                    (1000, Point::new(7.0, 600.0)),
                    (1100, Point::new(5.0, 400.0)),
                    (1200, Point::new(6.0, 300.0)),
                    (1300, Point::new(7.0, 200.0)),
                    (1400, Point::new(4.0, 500.0)),
                    (1500, Point::new(9.0, 800.0)),
                ]
            );
        }

        #[test]
        fn a_jitter_moves_the_hull_and_layers_from_their_places_but_not_the_shield() {
            let cloak = CloakDraw {
                hull: None,
                hull_translucency: 0,
                offset: Point::new(2.0, -3.0),
                layers: Some(32),
                weapon: 32,
                shield: true,
            };
            assert_eq!(
                centres(Some(cloak))
                    .into_iter()
                    .map(|(id, at)| (id, at.x))
                    .collect::<Vec<_>>(),
                [
                    (1000, 9.0),
                    (1100, 7.0),
                    (1200, 8.0),
                    (1300, 9.0),
                    (1400, 6.0),
                    (1500, 9.0),
                ]
            );
        }
    }

    /// A ship whose sheets are 8-bit, through the draw proc's depth-8
    /// dispatch ([`EightBitBlit`](crate::eight_bit_blit::EightBitBlit)),
    /// beside the same ship's 16-bit sheets.
    mod eight_bit {
        use std::num::NonZeroU16;

        use nova_sim::ShipRef;
        use nova_sim::cloak::CloakShown;
        use nova_sim::combat::ship_frames::{Alt, Drawn, FrameSpec};

        use super::super::*;
        use crate::flight::cloak_fade::CloakDraw;
        use crate::flight::held_alt::HeldAlts;
        use crate::{DrawCommand, ImageKey};

        const AT: Point = Point::new(100.0, 50.0);

        const PAINT: Option<Rgb15> = Some(Rgb15 { r: 31, g: 16, b: 4 });

        /// A 4 x 4 layer `rlëD` `image_id` of 36 frames.
        fn layer(image_id: i16, eight_bit: bool, solid: bool) -> LayerSheet {
            LayerSheet {
                image: crate::SheetImage::Rled(image_id),
                frames: NonZeroU16::new(36).expect("non-zero"),
                frame_width: 4,
                frame_height: 4,
                image_size: (4, 4),
                clip_quirks: false,
                solid,
                eight_bit,
            }
        }

        /// A hull 1000 with a glow 1100, alt 1400 and shield 1500, every
        /// sheet 8-bit or not and holding solid runs or not.
        fn sheet(eight_bit: bool, solid: bool) -> ShipSheet {
            ShipSheet {
                image: crate::SheetImage::Rled(1000),
                rotations: NonZeroU16::new(36).expect("non-zero"),
                frame_width: 10,
                frame_height: 10,
                image_size: (10, 10),
                clip_quirks: false,
                glow: Some(layer(1100, eight_bit, solid)),
                lights: None,
                weapon: None,
                weap_decay: 0,
                alt: Some(layer(1400, eight_bit, solid)),
                shield: Some(layer(1500, eight_bit, solid)),
                base_transp: 0,
                solid,
                eight_bit,
            }
        }

        /// The player's ship of `sheet`, unpainted, opaque, uncolorized,
        /// its alt drawn on set 0 and no layer lit.
        fn ship<'a>(sheet: &'a ShipSheet, spec: &'a FrameSpec) -> ShipDraw<'a> {
            ShipDraw {
                sheet,
                spec,
                drawn: Drawn {
                    alt: Alt::Drawn(0),
                    ..Drawn::default()
                },
                heading: 0.0,
                tint: Color::WHITE,
                paint: None,
                cloak: None,
                hull: true,
                held: false,
                glow: None,
                lights: None,
                weapon: None,
                shield: None,
                colorize: None,
            }
        }

        /// `sheet`'s ship cloaked `shown`.
        fn cloaked<'a>(
            sheet: &'a ShipSheet,
            spec: &'a FrameSpec,
            shown: CloakShown,
        ) -> ShipDraw<'a> {
            let cloak = CloakDraw::of(shown, None, 0, &HashedRolls::new(1));
            ShipDraw {
                cloak,
                hull: cloak.is_none_or(|cloak| cloak.hull.is_some()),
                ..ship(sheet, spec)
            }
        }

        /// A hull drawn at `BaseTransp` `base_transp`'s alpha.
        fn at_base_transp(base_transp: i16) -> Color {
            Color {
                a: base_alpha(base_transp),
                ..Color::WHITE
            }
        }

        /// One sprite drawn: its image, tint, blend and colorize.
        type Sprite = (ImageKey, Color, Blend, Option<Colorize>);

        /// What `ship` draws with its alt `alt`.
        fn drawn_with(ship: &ShipDraw<'_>, alt: Option<AltLayer>) -> Vec<Sprite> {
            let mut list = DrawList::new();
            ship.draw(&mut list, &|_| AT, alt);
            list.iter()
                .filter_map(|command| match *command {
                    DrawCommand::Sprite {
                        image, tint, blend, ..
                    } => Some((image, tint, blend, None)),
                    DrawCommand::Colorized {
                        image,
                        tint,
                        blend,
                        colorize,
                        ..
                    } => Some((image, tint, blend, Some(colorize))),
                    _ => None,
                })
                .collect()
        }

        /// What `ship` draws, its alt live.
        fn drawn(ship: &ShipDraw<'_>) -> Vec<Sprite> {
            drawn_with(ship, ship.live_alt(&|_| AT))
        }

        /// The sheets of what `ship` draws.
        fn ids(ship: &ShipDraw<'_>) -> Vec<i16> {
            drawn(ship).iter().map(|(image, ..)| image.id).collect()
        }

        const NONE: [i16; 0] = [];

        #[test]
        fn a_painted_opaque_8_bit_hull_draws_plainly_unpainted() {
            let spec = FrameSpec::NONE;
            let (eight, sixteen) = (sheet(true, false), sheet(false, false));
            let painted = |sheet| ShipDraw {
                paint: PAINT,
                ..ship(sheet, &spec)
            };
            let look = |sprite: Sprite| (sprite.0.id, sprite.1, sprite.2, sprite.3);
            // +0xa8 0 and no bias: `_BlitPixieRLE`, the factors ignored.
            let eight = drawn(&painted(&eight));
            assert_eq!(look(eight[0]), (1000, Color::WHITE, Blend::Normal, None));
            // Its alt copies the hull's state, and draws plainly too.
            assert_eq!(look(eight[1]), (1400, Color::WHITE, Blend::Normal, None));
            assert_eq!(drawn(&painted(&sixteen))[0].2, Blend::OrTranslucent);
        }

        #[test]
        fn a_translucent_or_colorized_8_bit_hull_draws_as_a_16_bit_one_does() {
            let spec = FrameSpec::NONE;
            let (eight, sixteen) = (sheet(true, false), sheet(false, false));
            let faint = |sheet, paint| ShipDraw {
                tint: at_base_transp(10),
                paint,
                ..ship(sheet, &spec)
            };
            for paint in [None, PAINT] {
                let drawn8 = drawn(&faint(&eight, paint));
                assert_eq!(drawn8, drawn(&faint(&sixteen, paint)), "{paint:?}");
                assert_eq!(drawn8.len(), 2, "the hull and its alt");
            }
            // Colorized, an opaque 8-bit hull is biased: drawn as 16-bit.
            let colorize = Some(Colorize::toward_pixel(5, 0x1081));
            let biased = |sheet| ShipDraw {
                colorize,
                ..ship(sheet, &spec)
            };
            assert_eq!(drawn(&biased(&eight)), drawn(&biased(&sixteen)));
            assert_eq!(drawn(&biased(&eight))[0].3, colorize);
        }

        #[test]
        fn an_8_bit_hull_at_base_transp_32_or_more_draws_nothing() {
            let spec = FrameSpec::NONE;
            let eight = sheet(true, false);
            for base_transp in [32, 40] {
                let gone = ShipDraw {
                    tint: at_base_transp(base_transp),
                    ..ship(&eight, &spec)
                };
                assert_eq!(ids(&gone), NONE, "{base_transp}");
            }
        }

        #[test]
        fn a_fading_cloak_hides_an_8_bit_hull_and_its_alt() {
            let spec = FrameSpec::NONE;
            let fading = CloakShown::Fading(21.5);
            assert_eq!(ids(&cloaked(&sheet(true, false), &spec, fading)), NONE);
            let sixteen: Vec<_> = drawn(&cloaked(&sheet(false, false), &spec, fading))
                .iter()
                .map(|(image, _, blend, _)| (image.id, *blend))
                .collect();
            assert_eq!(sixteen, [(1000, Blend::Or), (1400, Blend::Or)]);
        }

        #[test]
        fn a_full_fade_seen_draws_an_8_bit_hull_as_a_16_bit_one() {
            let spec = FrameSpec::NONE;
            let seen = CloakShown::Translucent(0x1e);
            let eight = drawn(&cloaked(&sheet(true, false), &spec, seen));
            assert_eq!(eight, drawn(&cloaked(&sheet(false, false), &spec, seen)));
            assert_eq!(eight[0].2, Blend::OrTranslucent);
        }

        #[test]
        fn an_8_bit_glow_draws_only_at_level_32_uncolorized() {
            let spec = FrameSpec::NONE;
            let (eight, sixteen) = (sheet(true, false), sheet(false, false));
            let glowing = |sheet, glow, held, colorize| ShipDraw {
                hull: false,
                glow: Some(glow),
                held,
                colorize,
                ..ship(sheet, &spec)
            };
            // The add-over: drawn as a 16-bit layer is.
            let full = drawn(&glowing(&eight, 32, false, None));
            assert_eq!(full, drawn(&glowing(&sixteen, 32, false, None)));
            assert_eq!(full[0].1, lights_tint(32));
            assert_eq!(full[0].2, Blend::Or);
            // Any other level draws nothing.
            assert_eq!(ids(&glowing(&eight, 31, false, None)), NONE);
            assert_eq!(ids(&glowing(&sixteen, 31, false, None)), [1100]);
            // An NPC's glow copies its murk, and draws nothing.
            let murk = Some(Colorize::toward_pixel(4, 0x1081));
            assert_eq!(ids(&glowing(&eight, 32, true, murk)), NONE);
            // The player's glow takes none of its hull's colorize, so it
            // draws at 32.
            assert_eq!(ids(&glowing(&eight, 32, false, murk)), [1100]);
        }

        #[test]
        fn an_8_bit_shield_at_32_draws_only_when_its_hull_is_uncolorized() {
            let spec = FrameSpec::NONE;
            let eight = sheet(true, false);
            let flashed = |colorize| ShipDraw {
                hull: false,
                shield: Some(32),
                colorize,
                ..ship(&eight, &spec)
            };
            assert_eq!(ids(&flashed(None)), [1500]);
            let ionized = Some(Colorize::toward_pixel(10, 0x7C00));
            assert_eq!(ids(&flashed(ionized)), NONE);
        }

        #[test]
        fn an_8_bit_sprite_that_draws_nothing_draws_no_runs() {
            let spec = FrameSpec::NONE;
            let (eight, sixteen) = (sheet(true, true), sheet(false, true));
            let lit = |sheet| ShipDraw {
                glow: Some(10),
                shield: Some(10),
                ..cloaked(sheet, &spec, CloakShown::Fading(21.5))
            };
            assert_eq!(ids(&lit(&eight)), NONE);
            // The 16-bit ship draws each sprite and its runs.
            let runs: Vec<_> = drawn(&lit(&sixteen))
                .iter()
                .filter(|(image, ..)| image.kind == crate::ImageKind::RledSolid)
                .map(|(image, ..)| image.id)
                .collect();
            assert_eq!(runs, [1000, 1100, 1400, 1500]);
        }

        #[test]
        fn a_held_alt_keeps_drawing_nothing() {
            let spec = FrameSpec::NONE;
            let eight = sheet(true, false);
            let fading = cloaked(&eight, &spec, CloakShown::Fading(21.5));
            let mut held = HeldAlts::default();
            let live = fading.live_alt(&|_| AT);
            let recorded = held.shown(ShipRef::Player, Alt::Drawn(0), live, true);
            assert!(recorded.is_some_and(|alt| !alt.shown), "{recorded:?}");
            // Held, the alt keeps the state it was last drawn in, though
            // the hull has since stopped fading.
            let kept = held.shown(ShipRef::Player, Alt::Held, None, true);
            assert_eq!(kept, recorded);
            let opaque = ship(&eight, &spec);
            let ids: Vec<_> = drawn_with(&opaque, kept)
                .iter()
                .map(|(image, ..)| image.id)
                .collect();
            assert_eq!(ids, [1000]);
        }

        /// A hull 1000 that is `hull_eight_bit` or not, whose glow 1100,
        /// alt 1400 and shield 1500 are `layers_eight_bit` or not.
        fn mixed(hull_eight_bit: bool, layers_eight_bit: bool) -> ShipSheet {
            ShipSheet {
                eight_bit: hull_eight_bit,
                ..sheet(layers_eight_bit, false)
            }
        }

        #[test]
        fn mid_fade_each_sprite_reads_its_own_sheets_flag() {
            let spec = FrameSpec::NONE;
            let (hull16, hull8) = (mixed(false, true), mixed(true, false));
            let lit = |sheet| ShipDraw {
                glow: Some(10),
                shield: Some(10),
                ..cloaked(sheet, &spec, CloakShown::Fading(21.5))
            };
            // A 16-bit hull draws; its 8-bit alt, in the hull's state, and
            // its 8-bit glow and shield draw nothing.
            assert_eq!(ids(&lit(&hull16)), [1000]);
            // An 8-bit hull draws nothing; its 16-bit glow, alt and shield
            // still draw.
            assert_eq!(ids(&lit(&hull8)), [1100, 1400, 1500]);
        }

        #[test]
        fn an_8_bit_glow_at_31_is_dropped_beside_a_16_bit_hull() {
            let spec = FrameSpec::NONE;
            let (hull16, hull8) = (mixed(false, true), mixed(true, false));
            let glowing = |sheet| ShipDraw {
                glow: Some(31),
                ..ship(sheet, &spec)
            };
            // The opaque hull and its alt draw plainly at either depth.
            assert_eq!(ids(&glowing(&hull16)), [1000, 1400]);
            assert_eq!(ids(&glowing(&hull8)), [1000, 1100, 1400]);
        }
    }

    /// A ship cut to its box at the world's edge (`box_cut`): each sprite
    /// by its own box and sheet, its solid runs with it.
    mod at_the_worlds_edge {
        use std::num::NonZeroU16;

        use nova_sim::combat::ship_frames::{Alt, Drawn, FrameSpec};

        use super::super::*;
        use crate::ImageKey;
        use crate::box_cut::drawn_cuts;
        use crate::flight::held_alt::AltLayer;
        use crate::geometry::Bounds;

        /// The world: 200 x 100 from the origin.
        const WORLD: Bounds = Bounds {
            min: Point { x: 0.0, y: 0.0 },
            max: Point { x: 200.0, y: 100.0 },
        };

        /// A 4 x 4 layer `rlëD` `image_id` of 36 frames, its image `image`.
        fn layer(image_id: i16, image: (u32, u32)) -> LayerSheet {
            LayerSheet {
                image: crate::SheetImage::Rled(image_id),
                frames: NonZeroU16::new(36).expect("non-zero"),
                frame_width: 4,
                frame_height: 4,
                image_size: image,
                clip_quirks: false,
                solid: false,
                eight_bit: false,
            }
        }

        /// A 10 x 10 hull 1000 whose image is `image`, with a box-sized
        /// glow 1100.
        fn sheet(image: (u32, u32)) -> ShipSheet {
            ShipSheet {
                image: crate::SheetImage::Rled(1000),
                rotations: NonZeroU16::new(36).expect("non-zero"),
                frame_width: 10,
                frame_height: 10,
                image_size: image,
                clip_quirks: false,
                glow: Some(layer(1100, (4, 4))),
                lights: None,
                weapon: None,
                weap_decay: 0,
                alt: None,
                shield: None,
                base_transp: 0,
                solid: false,
                eight_bit: false,
            }
        }

        /// The player's ship of `sheet`, opaque, its glow lit at 32.
        fn ship<'a>(sheet: &'a ShipSheet, spec: &'a FrameSpec) -> ShipDraw<'a> {
            ShipDraw {
                sheet,
                spec,
                drawn: Drawn {
                    alt: Alt::Drawn(0),
                    ..Drawn::default()
                },
                heading: 0.0,
                tint: Color::WHITE,
                paint: None,
                cloak: None,
                hull: true,
                held: false,
                glow: Some(32),
                lights: None,
                weapon: None,
                shield: None,
                colorize: None,
            }
        }

        /// What `ship` draws centred on `at` in the world: each command's
        /// image and the box it is cut to, if it is.
        fn drawn_at(ship: &ShipDraw<'_>, at: Point) -> Vec<(ImageKey, Option<Bounds>)> {
            let mut list = DrawList::new();
            list.world_in(WORLD, |list| ship.draw(list, &|_| at, None));
            drawn_cuts(&list)
        }

        /// The box of a `side`-square frame centred on `at`.
        fn box_of(at: Point, side: f32) -> Bounds {
            crate::box_cut::box_of(at, (side, side))
        }

        /// Straddling the world's left edge: the hull's box and the glow's.
        const LEFT: Point = Point::new(1.0, 50.0);

        #[test]
        fn an_overhanging_hull_straddling_the_worlds_edge_is_cut_to_its_box() {
            let (sheet, spec) = (sheet((14, 10)), FrameSpec::NONE);
            assert_eq!(
                drawn_at(&ship(&sheet, &spec), LEFT),
                [
                    (ImageKey::sprite(1000, 0), Some(box_of(LEFT, 10.0))),
                    (ImageKey::sprite(1100, 0), None),
                ]
            );
        }

        #[test]
        fn an_overhanging_hull_wholly_inside_the_world_draws_whole() {
            let (sheet, spec) = (sheet((14, 10)), FrameSpec::NONE);
            let inside = Point::new(100.0, 50.0);
            assert_eq!(
                drawn_at(&ship(&sheet, &spec), inside),
                [
                    (ImageKey::sprite(1000, 0), None),
                    (ImageKey::sprite(1100, 0), None),
                ]
            );
        }

        #[test]
        fn a_quirky_hull_cut_at_the_left_draws_its_clipped_frame_and_runs() {
            let sheet = ShipSheet {
                clip_quirks: true,
                solid: true,
                ..sheet((10, 10))
            };
            let spec = FrameSpec::NONE;
            let hull = box_of(LEFT, 10.0);
            assert_eq!(
                drawn_at(&ship(&sheet, &spec), LEFT),
                [
                    (ImageKey::clipped_sprite(1000, 0, (0, 4)), Some(hull)),
                    (ImageKey::clipped_solid_runs(1000, 0, (0, 4)), Some(hull)),
                    (ImageKey::sprite(1100, 0), None),
                ]
            );
        }

        #[test]
        fn an_overhanging_hulls_solid_runs_are_cut_to_its_box_with_it() {
            let sheet = ShipSheet {
                solid: true,
                ..sheet((14, 10))
            };
            let spec = FrameSpec::NONE;
            let hull = box_of(LEFT, 10.0);
            assert_eq!(
                drawn_at(&ship(&sheet, &spec), LEFT)[..2],
                [
                    (ImageKey::sprite(1000, 0), Some(hull)),
                    (ImageKey::solid_runs(1000, 0), Some(hull)),
                ]
            );
        }

        #[test]
        fn an_overhanging_glow_is_cut_by_its_own_box_not_the_hulls() {
            let sheet = ShipSheet {
                glow: Some(layer(1100, (8, 4))),
                ..sheet((10, 10))
            };
            let spec = FrameSpec::NONE;
            // The hull's box straddles the left edge; the glow's, 4 wide,
            // is inside.
            let at = Point::new(3.0, 50.0);
            assert_eq!(
                drawn_at(&ship(&sheet, &spec), at),
                [
                    (ImageKey::sprite(1000, 0), None),
                    (ImageKey::sprite(1100, 0), None),
                ]
            );
            // Both straddle it: the glow is cut to its own box.
            assert_eq!(
                drawn_at(&ship(&sheet, &spec), LEFT),
                [
                    (ImageKey::sprite(1000, 0), None),
                    (ImageKey::sprite(1100, 0), Some(box_of(LEFT, 4.0))),
                ]
            );
        }

        #[test]
        fn a_pict_pair_hull_and_layer_at_the_edge_draw_their_frames_cut_to_their_boxes() {
            use nova_data::ShipImage;
            let pair = |image| crate::SheetImage::Ship { ship: 128, image };
            // Even marked quirky and solid, a pair has no clipped frame and
            // no runs: its 16-bit RLE is cut to its box.
            let sheet = ShipSheet {
                image: pair(ShipImage::Base),
                clip_quirks: true,
                solid: true,
                glow: Some(LayerSheet {
                    image: pair(ShipImage::Glow),
                    clip_quirks: true,
                    solid: true,
                    ..layer(1100, (4, 4))
                }),
                ..sheet((10, 10))
            };
            let spec = FrameSpec::NONE;
            assert_eq!(
                drawn_at(&ship(&sheet, &spec), LEFT),
                [
                    (
                        ImageKey::ship_sheet(128, ShipImage::Base, 0),
                        Some(box_of(LEFT, 10.0))
                    ),
                    (
                        ImageKey::ship_sheet(128, ShipImage::Glow, 0),
                        Some(box_of(LEFT, 4.0))
                    ),
                ]
            );
        }

        #[test]
        fn a_held_alt_takes_its_sheets_cut_only_when_its_whole_key_matches() {
            use nova_data::ShipImage;
            let pair = crate::SheetImage::Ship {
                ship: 1400,
                image: ShipImage::Alt,
            };
            let sheet = ShipSheet {
                glow: None,
                // Overhanging its box, so a cut shows.
                alt: Some(LayerSheet {
                    image: pair,
                    ..layer(1400, (8, 4))
                }),
                ..sheet((10, 10))
            };
            let spec = FrameSpec::NONE;
            let held = |image| AltLayer {
                image,
                at: LEFT,
                shown: true,
                tint: Color::WHITE,
                colorize: None,
                blend: Blend::Normal,
            };
            let alt_of = |alt: AltLayer| {
                let mut list = DrawList::new();
                list.world_in(WORLD, |list| {
                    ship(&sheet, &spec).draw(list, &|_| LEFT, Some(alt));
                });
                drawn_cuts(&list)[1]
            };
            // Its own sheet's key: cut by that sheet's box.
            let own = pair.frame(2);
            assert_eq!(alt_of(held(own)), (own, Some(box_of(LEFT, 4.0))));
            // An `rlëD` of the same ID is another sheet: drawn whole.
            let other = ImageKey::sprite(1400, 2);
            assert_eq!(alt_of(held(other)), (other, None));
        }

        #[test]
        fn every_layer_is_decided_by_its_own_sheet() {
            let edged = |image_id| LayerSheet {
                clip_quirks: true,
                solid: true,
                ..layer(image_id, (4, 4))
            };
            let sheet = ShipSheet {
                glow: Some(edged(1100)),
                lights: Some(edged(1200)),
                weapon: Some(edged(1300)),
                alt: Some(edged(1400)),
                shield: Some(edged(1500)),
                ..sheet((10, 10))
            };
            let spec = FrameSpec::NONE;
            let every_layer = ShipDraw {
                lights: Some(32),
                weapon: Some(32),
                shield: Some(32),
                ..ship(&sheet, &spec)
            };
            let mut list = DrawList::new();
            list.world_in(WORLD, |list| {
                every_layer.draw(list, &|_| LEFT, every_layer.live_alt(&|_| LEFT));
            });
            let layer_box = box_of(LEFT, 4.0);
            let cut = (0, 1);
            let layers: Vec<_> = drawn_cuts(&list)
                .into_iter()
                .filter(|(image, _)| image.id != 1000)
                .collect();
            let expected: Vec<_> = [1100, 1200, 1300, 1400, 1500]
                .into_iter()
                .flat_map(|id| {
                    [
                        (ImageKey::clipped_sprite(id, 0, cut), Some(layer_box)),
                        (ImageKey::clipped_solid_runs(id, 0, cut), Some(layer_box)),
                    ]
                })
                .collect();
            assert_eq!(layers, expected);
        }
    }

    #[test]
    fn an_npcs_lights_take_a_third_of_its_murk() {
        assert_eq!(lights_murk(0), 0);
        assert_eq!(lights_murk(3), 0, "0.999");
        assert_eq!(lights_murk(4), 1);
        assert_eq!(lights_murk(31), 10);
    }
}
