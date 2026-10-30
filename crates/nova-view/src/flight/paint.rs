//! A painted ship's hull, as `_HandleShipDisplay` (@0x2be41-0x2beb2 in
//! `EV Nova.app`'s `EV Nova`, i386) draws it from the paint the session
//! resolves (`nova_sim::flight_frame::FrameShip::paint`,
//! `_GetShipPaintColor` @0x7d0c).
//!
//! For every ship, the player too, the hull's channel factors +0xaa,
//! +0xac and +0xae are set to −32 and the paint's channel added:
//! `paint_c − 32`, unpainted (32, 32, 32) giving 0. That follows `BaseTransp`'s
//! translucency t (+0xa8, @0x2bdb2-0x2bdce) and the murk, and precedes
//! the ion tint and a gate's colorize, which write only +0xa8, +0xb0 and
//! +0xb4, so a gate's fade changes t and leaves the paint. The draw proc
//! takes the plain blit only with all three factors at 32
//! (@0xbb71e-0xbb736), never so here, so a hull always goes through
//! `_BlitPixieRLETranslucent` (@0xc1568), whose source factor is
//! `factor_c − t + 32 = paint_c − t` and destination factor t
//! (@0xc1578-0xc1648):
//!
//! ```text
//! out_c = ((src_c × (paint_c − t)) >> 5) | ((dst_c × t) >> 5)
//! ```
//!
//! per 5-bit channel. Unpainted it is the crossfade at (32 − t) the view
//! draws plainly ([`base_alpha`](super::ship_layers::base_alpha)), so an
//! unpainted hull keeps that path ([`painted_hull`] gives `None`); painted
//! and opaque it is the multiply `(src_c × paint_c) >> 5`. The view draws
//! a painted hull as [`Blend::OrTranslucent`](crate::Blend::OrTranslucent)
//! at [`translucent_tint_by_channel`]. The alt layer copies the hull's
//! factors (@0x2caa1-0x2cb0a), so it is painted alike; the glow, lights,
//! weapon glow and shield write their own, and the solid runs fill raw,
//! so the paint never reaches them. A cloak writes the factors from the
//! paint after all of this (see [`cloak_fade`](super::cloak_fade)).
//!
//! Left out: a channel painted below t gives a negative source factor,
//! which the blit's unsigned test (@0xc1645) sends down its brighten
//! branch (@0xc164c). That is an integer wrap, which the engine does not
//! replicate: the factor is held to 0. The 8-bit depth's blits
//! (@0xbb7ed-0xbb8b6) are not modelled, as the view draws at 16 bits.

use nova_sim::Rgb15;

use crate::Color;
use crate::draw::translucent_tint_by_channel;

/// An unpainted ship's paint in each channel, in 32nds
/// (`_GetShipPaintColor` @0x7d9d-0x7da7).
pub const UNPAINTED: u8 = 32;

/// The steps of a hull's translucency, and of each channel factor.
const STEPS: u8 = 32;

/// `paint`'s red, green and blue, each 0 to 31, or [`UNPAINTED`] in each
/// for none.
#[must_use]
pub fn paint_channels(paint: Option<Rgb15>) -> [u8; 3] {
    paint.map_or([UNPAINTED; 3], |paint| [paint.r, paint.g, paint.b])
}

/// The translucency, in 32nds, of a hull drawn at `alpha`: 32 less its
/// opacity in 32nds, as [`base_alpha`](super::ship_layers::base_alpha)
/// and a gate's fade write it.
#[must_use]
pub fn translucency(alpha: u8) -> u8 {
    // At most 32.
    #[allow(clippy::cast_possible_truncation)]
    let opacity = ((u16::from(alpha) * u16::from(STEPS) + 127) / 255) as u8;
    STEPS - opacity
}

/// The [`Blend::OrTranslucent`](crate::Blend::OrTranslucent) tint of a
/// hull painted `paint` at translucency `translucency` (see the module
/// docs): source factor `paint_c − t` in each channel, held to 0, and
/// destination factor t; `None` unpainted, so the hull draws as before.
#[must_use]
pub fn painted_hull(paint: Option<Rgb15>, translucency: u8) -> Option<Color> {
    let t = translucency.min(STEPS);
    let source = paint_channels(Some(paint?)).map(|channel| channel.saturating_sub(t));
    Some(translucent_tint_by_channel(source, t))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flight::ship_layers::base_alpha;

    /// A paint of `r`, `g` and `b`.
    fn paint(r: u8, g: u8, b: u8) -> Rgb15 {
        Rgb15 { r, g, b }
    }

    /// One 5-bit channel as `Blend::OrTranslucent` documents its draw at
    /// `tint`: `n_c = round(c × 32 / 255)`, `m = round(a × 32 / 255)`, then
    /// `((src × n_c) >> 5) | ((dst × m) >> 5)`.
    fn or_translucent(tint: Color, channel: usize, src: u16, dst: u16) -> u16 {
        let back = |value: u8| (u16::from(value) * 32 + 127) / 255;
        let n = back([tint.r, tint.g, tint.b][channel]);
        ((src * n) >> 5) | ((dst * back(tint.a)) >> 5)
    }

    /// One 5-bit channel as `_BlitPixieRLETranslucent` (@0xc1578-0xc1648)
    /// draws a hull painted `paint_c` at translucency `t`, for `paint_c ≥ t`.
    fn original(paint_c: u16, t: u16, src: u16, dst: u16) -> u16 {
        ((src * (paint_c - t)) >> 5) | ((dst * t) >> 5)
    }

    #[test]
    fn a_painted_hull_pixel_draws_the_originals_colour() {
        for t in 0..=31u8 {
            for paint_c in t..=31 {
                let tint = painted_hull(Some(paint(paint_c, 31, 0)), t).expect("painted");
                for (src, dst) in [(31, 0), (17, 9), (31, 31), (1, 30), (0, 31)] {
                    assert_eq!(
                        or_translucent(tint, 0, src, dst),
                        original(u16::from(paint_c), u16::from(t), src, dst),
                        "paint {paint_c} at {t}: {src} over {dst}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_painted_opaque_hull_multiplies_each_channel_by_its_paint() {
        let tint = painted_hull(Some(paint(31, 16, 4)), 0).expect("painted");
        assert_eq!(tint, translucent_tint_by_channel([31, 16, 4], 0));
        // A white hull pixel takes the paint; a mid grey half of it.
        let drawn = |src| [0, 1, 2].map(|c| or_translucent(tint, c, src, 25));
        assert_eq!(drawn(31), [30, 15, 3]);
        assert_eq!(drawn(16), [15, 8, 2]);
    }

    #[test]
    fn an_unpainted_hull_keeps_its_own_colour() {
        assert_eq!(painted_hull(None, 0), None);
        assert_eq!(painted_hull(None, 10), None);
    }

    #[test]
    fn a_channel_painted_below_the_translucency_is_held_to_0() {
        // The blit's unsigned test would brighten it (@0xc1645): a wrap,
        // not replicated.
        let tint = painted_hull(Some(paint(4, 20, 31)), 10).expect("painted");
        assert_eq!(tint, translucent_tint_by_channel([0, 10, 21], 10));
        assert_eq!(
            painted_hull(Some(paint(31, 31, 31)), 40),
            Some(translucent_tint_by_channel([0; 3], 32)),
            "a translucency past 32 is 32"
        );
    }

    #[test]
    fn a_hulls_translucency_is_32_less_its_opacity_in_32nds() {
        for t in 0..=32 {
            assert_eq!(translucency(base_alpha(t.into())), t, "{t}");
        }
        assert_eq!(translucency(255), 0);
        assert_eq!(translucency(0), 32);
    }

    #[test]
    fn a_paints_channels_are_its_own_or_32_unpainted() {
        assert_eq!(paint_channels(Some(paint(1, 2, 3))), [1, 2, 3]);
        assert_eq!(paint_channels(None), [32, 32, 32]);
    }
}
