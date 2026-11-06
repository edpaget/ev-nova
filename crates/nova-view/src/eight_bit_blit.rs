//! How the original's translucent draw proc draws a frame whose `rlëD`
//! header says depth 8 ([`EightBitBlit::of`]).
//!
//! `_BlitPixieRLETranslucentDrawProc` (@0xbb3fc) tests the **source
//! frame's** depth (+0x20) for 8: the clipped path @0xbb5a4 (to @0xbb7ed),
//! the unclipped path @0xbb792 (to @0xbb8b6). It reads the current sprite's
//! (`0x3b5250`) translucency (+0xa8, a signed word), its red, green and
//! blue channel factors (+0xaa, +0xac, +0xae), its bias (the colorize
//! level, +0xb0) and its colour (+0xb4). Both paths have the same shape:
//!
//! | Sprite state | Unclipped | Clipped |
//! |---|---|---|
//! | +0xa8 ≤ 0, +0xb0 ≤ 0 | `_BlitPixieRLE` (@0xbb9f1-0xbb9f9 → @0xbb9fb → @0xbb9c3) | `_BlitPixieRLEClipped` (@0xbb895 → @0xbb89f → @0xbb751) |
//! | +0xa8 ≤ 0, +0xb0 > 0 | `_BlitPixieRLETranslucent8` (@0xbb9f9 → @0xbb9e7) | `_BlitPixieRLETranslucentClipped8` (@0xbb89d → @0xbb88b) |
//! | 1 ≤ +0xa8 ≤ 0x1f | `_BlitPixieRLETranslucent8` (@0xbb8cd-0xbb8d1 → @0xbb9e7) | `_BlitPixieRLETranslucentClipped8` (@0xbb804-0xbb808 → @0xbb88b) |
//! | +0xa8 = 0x20, all three factors 0x20, +0xb0 = 0 | `_BlitPixieRLEAddOver8` (@0xbb8d7-0xbb907 → @0xbb924) | `_BlitPixieRLEAddOverClipped8` (@0xbb80a-0xbb83a → @0xbb85b) |
//! | +0xa8 ≥ 0x20, any other case | nothing (@0xbb8df, @0xbb8ef, @0xbb8f9, @0xbb907 → @0xbb5e0) | nothing (@0xbb812, @0xbb822, @0xbb82c, @0xbb83a → @0xbb5e0) |
//!
//! Where it differs from the 16-bit dispatch (@0xbb79d-0xbb9c8,
//! @0xbb5af-0xbb757):
//!
//! - At +0xa8 ≤ 0 with no bias, depth 8 ignores the factors. Depth 16
//!   takes `_BlitPixieRLETranslucent` unless all three are 0x20
//!   (@0xbb98e-0xbb9a6).
//! - At 0x20 ≤ +0xa8 ≤ 0x3f, depth 16 draws translucent; only above 0x3f
//!   does it draw nothing (@0xbb7c6, @0xbb5d8).
//! - A non-zero +0xb0 sends depth 16's add-over case to the translucent
//!   blit (@0xbb952). It sends depth 8 to nothing.
//!
//! # The depth-8 routines
//!
//! `_BlitPixieRLETranslucent8` (@0xc1cff-0xc1edf) and its clipped twin
//! (@0xc1ee4-0xc21c0) take a level L = +0xa8 >> 3 when 0 ≤ +0xa8 < 32, and
//! 3 otherwise (@0xc1d14-0xc1d33). At a bias above 2 (@0xc1d4e) each pixel
//! byte s becomes T\[B\]\[C\]\[s\], with B = min(32 − +0xb0, 31) >> 3 (4
//! when that is negative or at least 5, @0xc1d54-0xc1d7d) and C the colour
//! +0xb4 (@0xc1d3b-0xc1d41); the destination is not read
//! (@0xc1e90-0xc1ec0). Otherwise each byte s over the destination byte d
//! becomes T\[L\]\[s\]\[d\] (@0xc1e20-0xc1e4b; clipped @0xc2119, @0xc219e).
//! The token walk is `_BlitPixieRLE`'s; a run (op 4) is a plain
//! `_BlitPixieMemSet` (@0xc1dca-0xc1ddb; clipped @0xc2051).
//!
//! `_BlitPixieRLEAddOver8` (@0xc21c1-0xc22aa) and its clipped twin
//! (@0xc22af-0xc24ba) make each byte T\[4\]\[d\]\[s\] (@0xc2260-0xc2284;
//! clipped @0xc2450-0xc2482), runs a plain `_BlitPixieMemSet` (@0xc221b;
//! clipped @0xc23b5).
//!
//! T is `_alphaTable` (0x3b07a0): 5 × 256 pointers to 256-byte rows,
//! filled from `t√§b` 128 by `_Construct8BitAlphaTable` (@0xc1c05-0xc1cb5).
//! Its only caller, `_SetupSpriteWorld` (@0x221ad-0x221b9), calls it only
//! in an 8-bit world, which only the legacy pipeline makes (`_main`
//! @0xccab8-0xccaca). In the 16-bit world every supported machine runs,
//! the table holds null pointers.
//!
//! # Where the original crashes
//!
//! So both routines read through a null row at the frame's first pixel
//! token: an address in `__PAGEZERO`, which quits the game. The bias copy's
//! row index uses the 16-bit world's +0xb4, an RGB555 colour of at least
//! 0x421 in stock, which lands on a null row or past the table. A crash is
//! not a reading, so the engine draws those cases as a 16-bit frame in the
//! same state is drawn ([`EightBitBlit::AsSixteenBit`]): what the routines
//! mean to draw. Everywhere the original does not crash, the engine
//! follows it.
//!
//! The dispatch reads the header's depth, whichever reading
//! (`eight_bit_sheet`) turned the bytes into pixels, so it takes no
//! reading.

use crate::{Blend, Color, Colorize};

/// How a sprite is drawn: its tint, colorize and blend.
pub type Look = (Color, Option<Colorize>, Blend);

/// The sprite fields the draw proc's depth-8 dispatch reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrawProcState {
    /// +0xa8: the translucency, a signed word. 0x20 and up draws nothing
    /// unless it is the add-over case.
    pub translucency: i16,
    /// Whether all three channel factors (+0xaa, +0xac, +0xae) are 0x20.
    pub full_factors: bool,
    /// +0xb0: the bias, the colorize level; 0 when not colorized.
    pub bias: i16,
}

impl DrawProcState {
    /// The bias a sprite drawn with `colorize` carries: its level, or 0
    /// when it is not colorized.
    #[must_use]
    pub fn bias_of(colorize: Option<Colorize>) -> i16 {
        colorize.map_or(0, |colorize| i16::from(colorize.level))
    }
}

/// What the draw proc does with a depth-8 frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EightBitBlit {
    /// `_BlitPixieRLE`: the frame as decoded, with no tint, blend or
    /// colorize, whatever the channel factors say.
    Plain,
    /// One of the depth-8 translucent or add-over routines, which crash in
    /// the 16-bit world: drawn as a 16-bit frame in the same state is.
    AsSixteenBit,
    /// No blit at all: not the frame, not its runs.
    Nothing,
}

impl EightBitBlit {
    /// What the draw proc does with a frame in `state` of a sheet that is
    /// `eight_bit` or not: [`of`](Self::of) for an 8-bit one, and a
    /// 16-bit frame's draw for any other.
    #[must_use]
    pub fn of_sheet(eight_bit: bool, state: DrawProcState) -> Self {
        if eight_bit {
            Self::of(state)
        } else {
            Self::AsSixteenBit
        }
    }

    /// How a sprite whose 16-bit frame would draw as `look` is drawn: at
    /// `look`; plainly, white, uncolorized and normal; or not at all.
    #[must_use]
    pub fn applied(self, look: Look) -> Option<Look> {
        match self {
            Self::Plain => Some((Color::WHITE, None, Blend::Normal)),
            Self::AsSixteenBit => Some(look),
            Self::Nothing => None,
        }
    }

    /// What the draw proc does with a depth-8 frame in `state` (see the
    /// module docs).
    #[must_use]
    pub fn of(state: DrawProcState) -> Self {
        let DrawProcState {
            translucency,
            full_factors,
            bias,
        } = state;
        match translucency {
            ..=0 if bias <= 0 => Self::Plain,
            ..=0x1f => Self::AsSixteenBit,
            0x20 if full_factors && bias == 0 => Self::AsSixteenBit,
            _ => Self::Nothing,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blit(translucency: i16, full_factors: bool, bias: i16) -> EightBitBlit {
        EightBitBlit::of(DrawProcState {
            translucency,
            full_factors,
            bias,
        })
    }

    #[test]
    fn only_an_8_bit_sheet_takes_the_depth_8_dispatch() {
        let state = DrawProcState {
            translucency: 0x30,
            full_factors: false,
            bias: 0,
        };
        assert_eq!(EightBitBlit::of_sheet(true, state), EightBitBlit::Nothing);
        assert_eq!(
            EightBitBlit::of_sheet(false, state),
            EightBitBlit::AsSixteenBit
        );
    }

    #[test]
    fn each_dispatch_draws_its_look() {
        let colorize = Some(Colorize::toward_pixel(24, 0x1081));
        let look = (Color::rgba(10, 20, 30, 40), colorize, Blend::Or);
        assert_eq!(
            EightBitBlit::Plain.applied(look),
            Some((Color::WHITE, None, Blend::Normal))
        );
        assert_eq!(EightBitBlit::AsSixteenBit.applied(look), Some(look));
        assert_eq!(EightBitBlit::Nothing.applied(look), None);
    }

    #[test]
    fn the_bias_is_the_colorize_level_or_none() {
        assert_eq!(DrawProcState::bias_of(None), 0);
        let colorize = Colorize::toward_pixel(24, 0x1081);
        assert_eq!(DrawProcState::bias_of(Some(colorize)), 24);
    }

    #[test]
    fn opaque_and_unbiased_draws_plain_whatever_the_factors() {
        for full in [false, true] {
            assert_eq!(blit(0, full, 0), EightBitBlit::Plain);
            assert_eq!(blit(-5, full, 0), EightBitBlit::Plain);
            assert_eq!(blit(0, full, -3), EightBitBlit::Plain);
        }
    }

    #[test]
    fn opaque_and_biased_takes_the_translucent_routine() {
        assert_eq!(blit(-5, false, 1), EightBitBlit::AsSixteenBit);
        assert_eq!(blit(0, false, 1), EightBitBlit::AsSixteenBit);
        assert_eq!(blit(0, true, 32), EightBitBlit::AsSixteenBit);
    }

    #[test]
    fn levels_below_0x20_take_the_translucent_routine() {
        for translucency in [1, 0x10, 0x1f] {
            for (full, bias) in [(false, 0), (true, 0), (false, 5), (true, 5)] {
                assert_eq!(
                    blit(translucency, full, bias),
                    EightBitBlit::AsSixteenBit,
                    "{translucency} {full} {bias}"
                );
            }
        }
    }

    #[test]
    fn the_plain_add_over_takes_the_add_over_routine() {
        assert_eq!(blit(0x20, true, 0), EightBitBlit::AsSixteenBit);
    }

    #[test]
    fn level_0x20_other_than_the_plain_add_over_draws_nothing() {
        assert_eq!(blit(0x20, true, 1), EightBitBlit::Nothing);
        assert_eq!(blit(0x20, true, -1), EightBitBlit::Nothing);
        assert_eq!(blit(0x20, false, 0), EightBitBlit::Nothing);
    }

    #[test]
    fn levels_above_0x20_draw_nothing() {
        // A 16-bit frame still draws translucent from 0x20 to 0x3f.
        for translucency in [0x21, 0x3f, 0x40, i16::MAX] {
            for (full, bias) in [(true, 0), (false, 0), (true, 4)] {
                assert_eq!(
                    blit(translucency, full, bias),
                    EightBitBlit::Nothing,
                    "{translucency} {full} {bias}"
                );
            }
        }
    }
}
