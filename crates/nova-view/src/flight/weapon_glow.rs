//! The weapon glow: a ship's `shän` `WeapImageID` sprite, lit while a
//! `Flags2` 0x0200 weapon it fires glows (ship +0xc8e8), as
//! `_HandleShipDisplay` draws it (@0x2c6ae-0x2c82c).
//!
//! - The fight lights it at 32.0 ([`CombatEvent::WeaponGlow`](nova_sim::CombatEvent::WeaponGlow)):
//!   a call that fired the weapon, before its ship's display, so it shows
//!   that frame (the player's fighter bay's only once the session puts a
//!   fighter it launched in the system, the same frame); and each live
//!   beam of it for its firer, after every
//!   display (`_HandleBeams` @0x4453d), so it shows the next frame
//!   (`late`).
//! - While the glow is above 0.0 the sprite shows at level
//!   n = trunc(min(glow, 32)) ([`WeaponGlow::shown`]), added,
//!   [`Blend::Or`](crate::draw::Blend::Or) at
//!   [`lights_tint`](crate::draw::lights_tint)`(n)`. An NPC's takes its
//!   hull's murk m, and n is held to trunc(32 − 1.5 × m), at least 0
//!   ([`npc_glow_level`], −1.5 @0xdd570, 32 @0xdd578); the player's has no
//!   murk.
//! - Then the glow falls by its class's decay, `WeapDecay` × 0.333
//!   ([`ShipSheet::weapon_decay`](super::catalog::ShipSheet::weapon_decay)),
//!   to −1.0 at the least ([`WeaponGlow::step`]).
//!
//! The spawns zero it (@0x3c4ff, @0x3ca71, @0x3d949, @0x82ef3, @0x1c4d4)
//! and the player's resets write it (@0x1d58b, @0x6c587, @0x67002,
//! @0x7638d): the view clears the glows where it clears the shield
//! flashes.
//!
//! The Weapon Effects preference, under which alone the weapon sprites
//! are loaded (`_LoadExtendedShipSprites` @0x2343c-0x2344b), gates the
//! drawing: off, the flight view draws no ship's glow layer, while the
//! glows still light and fall, as +0xc8e8 still does without the
//! sprites. The original reads it only at launch; the router keeps the
//! value the game launched with unless
//! `nova_sim::RuleKey::WeaponEffectsNextLaunch` says otherwise.
//!
//! Left out: the 8-bit
//! depth's translucency 32 − n (@0x2c78b-0x2c79d), as the engine draws at
//! 16-bit only; and the cloak's effect on the layer, the cloak roadmaps'.

use std::collections::BTreeMap;

use nova_sim::ShipRef;

/// The glow a firing lights (32.0 @0x62c98, @0x88de4, @0x30b37).
pub const GLOW_LIT: f32 = 32.0;

/// The least the glow falls to (-1.0).
pub const GLOW_FLOOR: f32 = -1.0;

/// What an NPC's sprite murk holds its weapon glow's level below:
/// 32 − [`GLOW_MURK_SCALE`] × m (@0xdd570, @0xdd578).
pub const GLOW_MURK_SCALE: f64 = 1.5;

/// One ship's weapon glow.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WeaponGlow {
    /// The glow, out of 32.
    glow: f32,
    /// The level its frame's display showed, if any.
    drawn: Option<u8>,
}

impl Default for WeaponGlow {
    /// Unlit: below none.
    fn default() -> Self {
        Self {
            glow: GLOW_FLOOR,
            drawn: None,
        }
    }
}

impl WeaponGlow {
    /// Lights it at [`GLOW_LIT`].
    pub fn light(&mut self) {
        self.glow = GLOW_LIT;
    }

    /// The level its sprite shows now: trunc(min(glow, 32)) while the
    /// glow is above none.
    #[must_use]
    pub fn shown(&self) -> Option<u8> {
        // At most 32 when positive.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        (self.glow > 0.0).then(|| self.glow.min(GLOW_LIT).trunc() as u8)
    }

    /// Its frame's display: the level it shows is kept ([`WeaponGlow::drawn`]),
    /// then the glow falls by `decay`, to [`GLOW_FLOOR`] at the least.
    pub fn step(&mut self, decay: f32) {
        self.drawn = self.shown();
        self.glow = (self.glow - decay).max(GLOW_FLOOR);
    }

    /// The level its last display showed, if it showed.
    #[must_use]
    pub fn drawn(&self) -> Option<u8> {
        self.drawn
    }
}

/// The level an NPC's weapon glow of level `n` shows at a sprite murk of
/// `murk` 32nds: n, held to trunc(32 − 1.5 × murk), at least 0.
#[must_use]
pub fn npc_glow_level(n: u8, murk: u8) -> u8 {
    let cap = (f64::from(GLOW_LIT) - GLOW_MURK_SCALE * f64::from(murk)).trunc();
    // Within 0..=32 once clamped.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let cap = cap.clamp(0.0, f64::from(GLOW_LIT)) as u8;
    n.min(cap)
}

/// Each ship's weapon glow.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WeaponGlows {
    glows: BTreeMap<ShipRef, WeaponGlow>,
}

impl WeaponGlows {
    /// Steps the glows a frame, as the fight's `lit` glows say (each
    /// ship's, and whether it is late): the glows lit before their ship's
    /// display are lit, then each ship's display shows its glow and lets
    /// it fall by its decay, `decay(ship)`, a ship of none being gone and
    /// forgotten; then the late ones are lit, to show the next frame.
    pub fn step(&mut self, lit: &[(ShipRef, bool)], decay: impl Fn(ShipRef) -> Option<f32>) {
        for &(ship, _) in lit.iter().filter(|(_, late)| !late) {
            self.glows.entry(ship).or_default().light();
        }
        self.glows.retain(|&ship, glow| {
            let Some(decay) = decay(ship) else {
                return false;
            };
            glow.step(decay);
            // One not shown is at or below none: as good as out.
            glow.drawn().is_some()
        });
        for &(ship, _) in lit.iter().filter(|(_, late)| *late) {
            self.glows.entry(ship).or_default().light();
        }
    }

    /// The level `ship`'s weapon glow showed at its last display, if it
    /// showed.
    #[must_use]
    pub fn drawn(&self, ship: ShipRef) -> Option<u8> {
        self.glows.get(&ship)?.drawn()
    }

    /// Puts every glow out.
    pub fn clear(&mut self) {
        self.glows.clear();
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use nova_sim::NpcId;

    use super::*;

    const NPC: ShipRef = ShipRef::Npc(NpcId(4));

    #[test]
    fn a_lit_glow_falls_by_its_decay_a_frame_to_minus_1() {
        let mut glow = WeaponGlow::default();
        assert_eq!(glow.shown(), None, "unlit");
        glow.light();
        assert_eq!(glow.shown(), Some(32));
        let decay = (5.0_f64 * 0.333) as f32;
        let mut shown = Vec::new();
        for _ in 0..22 {
            glow.step(decay);
            shown.push(glow.drawn());
        }
        let mut level = 32.0_f32;
        let expected: Vec<Option<u8>> = (0..22)
            .map(|_| {
                let drawn = (level > 0.0).then(|| level.min(32.0).trunc() as u8);
                level = (level - decay).max(-1.0);
                drawn
            })
            .collect();
        assert_eq!(shown, expected);
        assert_eq!(&shown[..4], [Some(32), Some(30), Some(28), Some(27)]);
        assert_eq!(glow.glow, -1.0, "floored");
    }

    #[test]
    fn a_glow_at_or_below_none_is_hidden() {
        let mut glow = WeaponGlow::default();
        glow.light();
        glow.step(32.0);
        assert_eq!(glow.drawn(), Some(32));
        assert_eq!(glow.shown(), None, "at 0.0");
        glow.step(1.0);
        assert_eq!(glow.drawn(), None);
        let mut faint = WeaponGlow::default();
        faint.light();
        faint.step(31.5);
        assert_eq!(faint.shown(), Some(0), "above none: level 0");
    }

    #[test]
    fn an_npcs_glow_is_held_below_its_murk() {
        assert_eq!(npc_glow_level(32, 0), 32);
        assert_eq!(npc_glow_level(32, 10), 17, "32 − 15");
        assert_eq!(npc_glow_level(12, 10), 12);
        assert_eq!(npc_glow_level(32, 21), 0, "32 − 31.5 truncates to 0");
        assert_eq!(npc_glow_level(32, 22), 0, "floored at 0");
        assert_eq!(npc_glow_level(32, 31), 0);
        assert_eq!(npc_glow_level(30, 1), 30, "32 − 1.5 = 30.5 → 30");
        assert_eq!(npc_glow_level(31, 1), 30);
    }

    #[test]
    fn a_firing_shows_that_frame_and_a_beams_glow_the_next() {
        let mut glows = WeaponGlows::default();
        let decay = |_| Some(4.0);
        glows.step(&[(ShipRef::Player, false)], decay);
        assert_eq!(glows.drawn(ShipRef::Player), Some(32), "that frame");
        glows.step(&[], decay);
        assert_eq!(glows.drawn(ShipRef::Player), Some(28));
        glows.step(&[(NPC, true)], decay);
        assert_eq!(glows.drawn(NPC), None, "a beam's: not this frame");
        glows.step(&[(NPC, true)], decay);
        assert_eq!(glows.drawn(NPC), Some(32), "the next");
        glows.step(&[], decay);
        assert_eq!(
            glows.drawn(NPC),
            Some(32),
            "once after the beam's last tick"
        );
        glows.step(&[], decay);
        assert_eq!(glows.drawn(NPC), Some(28));
    }

    #[test]
    fn a_gone_ships_glow_is_forgotten_and_a_cleared_one_out() {
        let mut glows = WeaponGlows::default();
        glows.step(&[(NPC, false), (ShipRef::Player, false)], |_| Some(1.0));
        glows.step(&[], |ship| (ship == ShipRef::Player).then_some(1.0));
        assert_eq!(glows.drawn(NPC), None, "gone");
        assert_eq!(glows.drawn(ShipRef::Player), Some(31));
        glows.clear();
        assert_eq!(glows.drawn(ShipRef::Player), None);
    }

    #[test]
    fn a_glow_run_out_is_dropped() {
        let mut glows = WeaponGlows::default();
        glows.step(&[(NPC, false)], |_| Some(40.0));
        assert_eq!(glows.drawn(NPC), Some(32));
        glows.step(&[], |_| Some(40.0));
        assert_eq!(glows.drawn(NPC), None);
        assert!(glows.glows.is_empty(), "dropped");
    }
}
