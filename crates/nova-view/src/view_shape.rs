//! Each screen's logical shape: the classic 1024x768, or flight's view
//! widened to the window's aspect ratio.
//!
//! Every screen draws in Nova's own 1024x768 except flight, which asks for
//! [`ViewShape::Widened`]: a height of 768 and a width set by the window,
//! never narrower than 1024 and with no upper cap ([`widened_size`]). The
//! original's flight is simply the window's size, its HUD placed from the
//! window's edges and the play area left of the status bar (`_SetupWind`
//! @0x1b720, `_InitSprites`, `_SetupSpriteWorld` @0x21b54 in `EV Nova.app`);
//! ours keeps the height at 768. The dialogs and maps over a widened view
//! keep their 1024x768 frame, centred in it ([`classic_offset`]), which is
//! where `_CenterWindow` (@0x220e-0x2359) puts a centred dialog in a window
//! that wide.
//!
//! `nova_render::Viewport` fits a shape into the window.

/// The classic logical size every screen but flight draws in: Nova's
/// largest original screen mode.
pub const CLASSIC_SIZE: (u32, u32) = (1024, 768);

/// The logical shape a screen asks for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ViewShape {
    /// Nova's 1024x768, letterboxed into the window.
    #[default]
    Classic,
    /// 768 high and as wide as the window's aspect ratio makes it, never
    /// narrower than 1024 ([`widened_size`]).
    Widened,
}

/// The widened logical size for a window of `window_px` pixels:
/// (max(1024, round(768 × w / h)), 768). A window taller than 4:3 keeps
/// 1024, as does one with no area. Width has no upper cap.
#[must_use]
pub fn widened_size(window_px: (u32, u32)) -> (u32, u32) {
    let (w, h) = window_px;
    if w == 0 || h == 0 {
        return CLASSIC_SIZE;
    }
    let (classic_w, classic_h) = (u64::from(CLASSIC_SIZE.0), u64::from(CLASSIC_SIZE.1));
    let (w, h) = (u64::from(w), u64::from(h));
    let widened = (classic_h * w + h / 2) / h;
    let width = u32::try_from(widened.max(classic_w)).unwrap_or(u32::MAX);
    (width, CLASSIC_SIZE.1)
}

/// How far right a 1024-wide frame sits when centred in a view `view_w`
/// wide: floor((`view_w` − 1024) / 2), 0 in the classic view. A dialog the
/// original centres on a window that wide (`_CenterWindow`, trunc(W/2) −
/// trunc(size/2)) lands exactly this far right of where it does at 1024.
#[must_use]
pub fn classic_offset(view_w: f32) -> f32 {
    ((view_w - CLASSIC_SIZE.0 as f32) / 2.0).floor()
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn a_4_by_3_window_keeps_the_classic_size() {
        for window in [(1024, 768), (2048, 1536), (800, 600), (1440, 1080)] {
            assert_eq!(widened_size(window), (1024, 768), "{window:?}");
        }
    }

    #[test]
    fn a_wider_window_widens_to_its_aspect_ratio_with_no_cap() {
        for (window, size) in [
            ((1920, 1080), (1365, 768)),
            ((1366, 768), (1366, 768)),
            ((2560, 1440), (1365, 768)),
            ((3440, 1440), (1835, 768)),
            ((5120, 1440), (2731, 768)),
        ] {
            assert_eq!(widened_size(window), size, "{window:?}");
        }
    }

    #[test]
    fn a_taller_window_or_one_with_no_area_keeps_the_classic_size() {
        assert_eq!(widened_size((1024, 1280)), (1024, 768));
        assert_eq!(widened_size((0, 0)), (1024, 768));
        assert_eq!(widened_size((1920, 0)), (1024, 768));
        assert_eq!(widened_size((0, 1080)), (1024, 768));
    }

    #[test]
    fn the_classic_frame_is_centred_floored() {
        assert_eq!(classic_offset(1024.0), 0.0);
        assert_eq!(classic_offset(1365.0), 170.0);
        assert_eq!(classic_offset(1366.0), 171.0);
    }

    #[test]
    fn a_screen_is_classic_by_default() {
        assert_eq!(ViewShape::default(), ViewShape::Classic);
    }
}
