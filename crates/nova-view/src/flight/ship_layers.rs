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
use crate::draw::lights_tint;
use crate::eight_bit_blit::{DrawProcState, EightBitBlit, Look};
use crate::{Blend, Color, Colorize, DrawList, ImageKey, Point};

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
            image: ImageKey::sprite(layer.image_id, frame),
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
            let hull = ImageKey::sprite(self.sheet.image_id, frame);
            list.mixed_sprite(hull, at, tint, blend, hull_colorize);
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
                let image = ImageKey::sprite(layer.image_id, frame);
                let at = self.moved(place(layer.size()));
                list.mixed_sprite(image, at, tint, blend, colorize);
                if layer.solid && level > 0 {
                    solid_overlay(list, layer.image_id, frame, at);
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
            list.mixed_sprite(alt.image, alt.at, alt.tint, alt.blend, alt.colorize);
            let alt_sheet = self.sheet.alt;
            if alt_sheet.is_some_and(|layer| layer.solid && layer.image_id == alt.image.id) {
                solid_overlay(list, alt.image.id, alt.image.frame, alt.at);
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
            let image = ImageKey::sprite(layer.image_id, frame);
            list.mixed_sprite(image, at, tint, blend, colorize);
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

    mod placement {
        use std::num::NonZeroU16;

        use nova_sim::combat::ship_frames::{Alt, Drawn, FrameSpec};

        use super::super::*;
        use crate::DrawCommand;
        use crate::flight::cloak_fade::CloakDraw;

        /// A layer `rlëD` `image_id` of `width` x `height` frames.
        fn layer(image_id: i16, (frame_width, frame_height): (u32, u32)) -> LayerSheet {
            LayerSheet {
                image_id,
                frames: NonZeroU16::new(36).expect("non-zero"),
                frame_width,
                frame_height,
                solid: false,
                eight_bit: false,
            }
        }

        /// A 7 x 6 hull 1000 whose glow 1100, lights 1200, weapon glow
        /// 1300, alt 1400 and shield 1500 each have a frame of its own.
        fn sheet() -> ShipSheet {
            ShipSheet {
                image_id: 1000,
                rotations: NonZeroU16::new(36).expect("non-zero"),
                frame_width: 7,
                frame_height: 6,
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
        use crate::DrawCommand;
        use crate::flight::cloak_fade::CloakDraw;
        use crate::flight::held_alt::HeldAlts;

        const AT: Point = Point::new(100.0, 50.0);

        const PAINT: Option<Rgb15> = Some(Rgb15 { r: 31, g: 16, b: 4 });

        /// A 4 x 4 layer `rlëD` `image_id` of 36 frames.
        fn layer(image_id: i16, eight_bit: bool, solid: bool) -> LayerSheet {
            LayerSheet {
                image_id,
                frames: NonZeroU16::new(36).expect("non-zero"),
                frame_width: 4,
                frame_height: 4,
                solid,
                eight_bit,
            }
        }

        /// A hull 1000 with a glow 1100, alt 1400 and shield 1500, every
        /// sheet 8-bit or not and holding solid runs or not.
        fn sheet(eight_bit: bool, solid: bool) -> ShipSheet {
            ShipSheet {
                image_id: 1000,
                rotations: NonZeroU16::new(36).expect("non-zero"),
                frame_width: 10,
                frame_height: 10,
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
    }

    #[test]
    fn an_npcs_lights_take_a_third_of_its_murk() {
        assert_eq!(lights_murk(0), 0);
        assert_eq!(lights_murk(3), 0, "0.999");
        assert_eq!(lights_murk(4), 1);
        assert_eq!(lights_murk(31), 10);
    }
}
