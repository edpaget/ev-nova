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
//!   draws it.
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
//! - **Lights out** ([`lights_out`]): a ship of `shän` `Flags` 0x0040 shows
//!   no lights while disabled, the player too (@0x2c56c-0x2c593).
//!
//! Each layer is added, [`Blend::Or`](crate::draw::Blend::Or) at
//! [`lights_tint`] of its level; the hull and the alt layer are drawn
//! plainly.

use nova_sim::combat::ship_frames::{Drawn, FrameSpec, LIGHTS_OUT_WHILE_DISABLED};
use nova_sim::traffic::npc::NpcId;
use nova_sim::{Condition, HashedRolls};

use super::catalog::{LayerSheet, ShipSheet};
use super::murk::murk_tint;
use super::weapon_glow::npc_glow_level;
use crate::draw::lights_tint;
use crate::{Color, DrawList, ImageKey, Point};

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

/// Whether a ship of `shän` `Flags` `flags` in `condition` shows no lights:
/// of 0x0040 and disabled (`_IsDisabled`).
#[must_use]
pub fn lights_out(flags: u16, condition: Condition) -> bool {
    flags & LIGHTS_OUT_WHILE_DISABLED != 0 && condition.disabled()
}

/// An NPC's lights' murk at a sprite murk of `murk`: trunc(murk × 0.333).
#[must_use]
pub fn lights_murk(murk: u8) -> u8 {
    // Within 0..=10 for a murk of at most 31.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let level = (f64::from(murk) * LIGHTS_MURK_SHARE).trunc() as u8;
    level
}

/// The rolls NPC `id`'s engine glow and running lights flicker on: its
/// own, seeds 2 + 2·id and 3 + 2·id, so no two ships flicker alike, nor
/// any like the player's (seeds 1 and 0).
#[must_use]
pub fn npc_rolls(id: NpcId) -> (HashedRolls, HashedRolls) {
    let seed = 2 + 2 * u64::from(id.0);
    (HashedRolls::new(seed), HashedRolls::new(seed + 1))
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
    /// Whether its hull, and so its alt layer, is drawn.
    pub hull: bool,
    /// Its sprite murk, 0 for the player.
    pub murk: u8,
    /// Whether its glow, lights and weapon glow are held below their murk:
    /// an NPC's.
    pub held: bool,
    /// Its engine glow's level, if shown.
    pub glow: Option<u8>,
    /// Its lights' level, if shown.
    pub lights: Option<u8>,
    /// Its weapon glow's level, if lit.
    pub weapon: Option<u8>,
    /// Its shield flash's level, if shown.
    pub shield: Option<u8>,
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

    /// Draws it at `at`: its hull, glow, lights, weapon glow, alt and shield
    /// layers, in the order `_HandleShipDisplay` puts them in their layer
    /// (@0x2bd58-0x2bdad).
    pub fn draw(&self, list: &mut DrawList, at: Point) {
        let frame = self.frame();
        let hull_tint = murk_tint(self.tint, self.murk);
        if self.hull {
            list.sprite(ImageKey::sprite(self.sheet.image_id, frame), at, hull_tint);
        }
        let held = |level: u8, murk: u8| {
            if self.held {
                npc_glow_level(level, murk)
            } else {
                level
            }
        };
        let add =
            |list: &mut DrawList, layer: Option<LayerSheet>, level: Option<u8>, murk, frame| {
                if let (Some(layer), Some(level)) = (layer, level) {
                    let tint = murk_tint(lights_tint(level), murk);
                    let frame = frame % layer.frames.get();
                    list.or_sprite(ImageKey::sprite(layer.image_id, frame), at, tint);
                }
            };
        let lights_murk = if self.held { lights_murk(self.murk) } else { 0 };
        let glow = self.glow.map(|level| held(level, self.murk));
        add(list, self.sheet.glow, glow, self.murk, frame);
        let lights = self.lights.map(|level| held(level, lights_murk));
        add(list, self.sheet.lights, lights, lights_murk, frame);
        let weapon = self.weapon.map(|level| held(level, self.murk));
        add(list, self.sheet.weapon, weapon, self.murk, frame);
        let spec = turning(self.spec, self.sheet);
        if self.hull
            && let (Some(layer), Some(set)) = (self.sheet.alt, self.drawn.alt_set)
        {
            let alt = spec.index(set, self.heading) % layer.frames.get();
            list.sprite(ImageKey::sprite(layer.image_id, alt), at, hull_tint);
        }
        if let (Some(layer), Some(level)) = (self.sheet.shield, self.shield) {
            let frame = spec.shield_frame(self.drawn.set, self.heading, layer.frames);
            let tint = murk_tint(lights_tint(level), self.murk);
            list.or_sprite(ImageKey::sprite(layer.image_id, frame), at, tint);
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
    fn the_lights_go_out_on_a_disabled_ship_of_0x0040() {
        assert!(lights_out(0x0040, Condition::Disabled));
        assert!(lights_out(0x0058, Condition::Disabled));
        assert!(!lights_out(0x0040, Condition::Intact));
        assert!(!lights_out(0x0018, Condition::Disabled));
        assert!(lights_out(
            0x0040,
            Condition::Dying {
                ticks_left: 3,
                starting: false
            }
        ));
    }

    #[test]
    fn each_npc_rolls_on_its_own_seeds() {
        use nova_sim::BlinkChance;
        let rolls = |id| {
            let (glow, blink) = npc_rolls(NpcId(id));
            (0..8)
                .map(|change| (glow.roll(change, 100), blink.roll(change, 100)))
                .collect::<Vec<_>>()
        };
        assert_ne!(rolls(0), rolls(1));
        let (glow, blink) = npc_rolls(NpcId(4));
        let seeded = |seed| {
            let rolls = HashedRolls::new(seed);
            (0..8).map(|c| rolls.roll(c, 100)).collect::<Vec<_>>()
        };
        let of = |rolls: &HashedRolls| (0..8).map(|c| rolls.roll(c, 100)).collect::<Vec<_>>();
        assert_eq!(
            (of(&glow), of(&blink)),
            (seeded(10), seeded(11)),
            "2 + 2·4, 3 + 2·4"
        );
        let player = HashedRolls::new(1);
        assert_ne!(
            (0..8).map(|c| glow.roll(c, 100)).collect::<Vec<_>>(),
            (0..8).map(|c| player.roll(c, 100)).collect::<Vec<_>>()
        );
        assert_ne!(
            (0..8).map(|c| glow.roll(c, 100)).collect::<Vec<_>>(),
            (0..8).map(|c| blink.roll(c, 100)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn an_npcs_lights_take_a_third_of_its_murk() {
        assert_eq!(lights_murk(0), 0);
        assert_eq!(lights_murk(3), 0, "0.999");
        assert_eq!(lights_murk(4), 1);
        assert_eq!(lights_murk(31), 10);
    }
}
