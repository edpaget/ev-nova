//! Sprite murk: how far a sprite has faded into a murky system's space,
//! as the original's `_HandleSpriteMurkiness(x, y, sprite)`
//! (@0x2b42f-0x2b513) works it out for a shot on every live tick
//! (`_HandleShot` @0x35c48-0x35c61), and how the view darkens the sprite
//! by it.
//!
//! - The level ([`sprite_murk`]): the shot's position truncated to whole
//!   pixels (`cvttss2si`, `cwtl` @0x35c4f-0x35c5d), each axis' distance
//!   from the player's ship (slot 0 +0x18/+0x1c) truncated (`cvttss2si`
//!   @0x2b451, @0x2b474), squared and summed, times the player's
//!   effective murk (`nova_sim::Session::effective_murk`), times
//!   [`MURK_SCALE`] (0xdd560), truncated, and clamped to 0..=31
//!   (@0x2b4cc-0x2b4e5). The original's sum and product are 32-bit
//!   (`imull` @0x2b469-0x2b48e), its positions 16-bit and its level
//!   stored in 16 bits, which would wrap far out; ours are wide and never
//!   wrap.
//! - The tint ([`murk_tint`]): the original mixes a murky sprite toward
//!   its system's `BkgndColor`, `src × (32 − m) >> 5 | colour × m >> 5`
//!   per 5-bit channel (`_BlitPixieTranslucentCopy` @0xc1140-0xc11ca),
//!   the colour the `sÿst` +0x1f8 RGB555 (built @0x77d85-0x77e99).
//!
//! Left out:
//!
//! - **The murk colour.** The sprite tint can only multiply, so the view
//!   darkens toward black, `rgb × (32 − m)/32`, and never adds the
//!   colour. The RGB555 rounding gives even a black `BkgndColor` 1 of 31
//!   a channel, and the stock systems that murk 1 to 4 of 31, so at full
//!   murk the view is within 4/31 a channel of the original, and nearer
//!   below it. Loading `BkgndColor` and an additive sprite colour in the
//!   renderer is rdm:task/murk-mixes-toward-the-background-colour.
//! - **The 8-bit depth's cap of 24** (@0x2b4b1-0x2b4c1): the engine draws
//!   at 16-bit only.

use nova_sim::Vec2;

use crate::Color;

/// What the squared distance times the murk is multiplied by (the double
/// @0xdd560).
pub const MURK_SCALE: f64 = 1.2e-5;

/// The most a sprite murks, out of 32 (@0x2b4cc-0x2b4d2).
pub const MAX_SPRITE_MURK: u8 = 31;

/// The steps of the murk's mix: a level of 32 would be all colour.
const MURK_STEPS: u16 = 32;

/// How murky a sprite at `at` is, seen from the player's ship at
/// `viewer`, in a system of effective murk `murk` (0..=100): 0 to
/// [`MAX_SPRITE_MURK`] out of 32 (see the module docs).
#[must_use]
pub fn sprite_murk(at: Vec2, viewer: Vec2, murk: u8) -> u8 {
    let axis = |sprite: f32, player: f32| {
        // A whole-pixel position, as the original passes it.
        let whole = sprite.trunc();
        i64::from((player - whole).trunc() as i32).abs()
    };
    let (dx, dy) = (axis(at.x, viewer.x), axis(at.y, viewer.y));
    let product = (dx * dx + dy * dy).saturating_mul(i64::from(murk));
    // A product of whole pixels within an `f64`'s exact range for any
    // distance a fight spans.
    #[allow(clippy::cast_precision_loss)]
    let level = (product as f64 * MURK_SCALE).trunc();
    // Clamped to 0..=31 before the cast.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let level = level.clamp(0.0, f64::from(MAX_SPRITE_MURK)) as u8;
    level
}

/// `base` darkened by a murk of `level` out of 32 ([`sprite_murk`]): its
/// red, green and blue times (32 − `level`)/32, rounded down as the
/// original's `>> 5` does, its alpha kept. Toward black, not the
/// system's background colour (see the module docs).
#[must_use]
pub fn murk_tint(base: Color, level: u8) -> Color {
    let keep = MURK_STEPS - u16::from(level).min(MURK_STEPS);
    // At most 255 × 32 / 32: within a `u8`.
    #[allow(clippy::cast_possible_truncation)]
    let scale = |channel: u8| (u16::from(channel) * keep / MURK_STEPS) as u8;
    Color::rgba(scale(base.r), scale(base.g), scale(base.b), base.a)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sprite_at_the_players_ship_never_murks() {
        let at = Vec2::new(120.5, -40.25);
        assert_eq!(sprite_murk(at, at, 100), 0);
    }

    #[test]
    fn the_level_is_the_truncated_distance_squared_times_the_murk_times_1_2e_minus_5() {
        // 300 and 400 off: 250,000 × 40 × 1.2e-5 = 120, clamped to 31.
        assert_eq!(sprite_murk(Vec2::new(300.0, 400.0), Vec2::ZERO, 40), 31);
        // 200 off on x: 40,000 × 50 × 1.2e-5 = 24.
        assert_eq!(sprite_murk(Vec2::new(200.0, 0.0), Vec2::ZERO, 50), 24);
        // (-150, 99) off: 22,500 + 9,801 = 32,301; × 60 × 1.2e-5 =
        // 23.26 → 23.
        let level = sprite_murk(Vec2::new(-150.0, 99.0), Vec2::ZERO, 60);
        assert_eq!(level, 23);
        // At murk 100 a level is 1,200ths of the squared distance: 129
        // off gives 19.97, 130 off 20.28.
        let at_129 =
            |shot: f32, player: f32| sprite_murk(Vec2::new(0.0, shot), Vec2::new(0.0, player), 100);
        assert_eq!(at_129(129.0, 0.0), 19);
        assert_eq!(at_129(130.0, 0.0), 20);
        assert_eq!(
            at_129(129.6, -0.5),
            19,
            "the shot truncated to 129 first: 129.5 off, truncated to 129"
        );
        assert_eq!(
            at_129(129.0, -0.9),
            19,
            "129.9 off truncates to 129, never rounds to 130"
        );
        assert_eq!(at_129(-129.6, 0.5), 19, "and the other way");
    }

    #[test]
    fn the_level_is_clamped_at_31() {
        assert_eq!(sprite_murk(Vec2::new(0.0, 162.0), Vec2::ZERO, 100), 31);
        assert_eq!(sprite_murk(Vec2::new(0.0, 160.0), Vec2::ZERO, 100), 30);
        assert_eq!(
            sprite_murk(Vec2::new(1.0e9, 1.0e9), Vec2::ZERO, 100),
            31,
            "never wraps"
        );
    }

    #[test]
    fn no_murk_murks_nothing() {
        assert_eq!(sprite_murk(Vec2::new(5000.0, 5000.0), Vec2::ZERO, 0), 0);
    }

    #[test]
    fn the_tint_darkens_toward_black_and_keeps_alpha() {
        let base = Color::rgba(255, 128, 31, 200);
        assert_eq!(murk_tint(base, 0), base);
        assert_eq!(murk_tint(base, 16), Color::rgba(127, 64, 15, 200));
        assert_eq!(murk_tint(base, 31), Color::rgba(7, 4, 0, 200));
        assert_eq!(murk_tint(base, 32), Color::rgba(0, 0, 0, 200));
        assert_eq!(murk_tint(base, 255), Color::rgba(0, 0, 0, 200));
    }
}
