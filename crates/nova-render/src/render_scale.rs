//! The render scale's sizing: the world texture a [`RenderScale`] of
//! [`RenderScale::Percent`] draws the world layer into, a share of the
//! backing size (the viewport's content in target pixels).
//! [`world_space`](crate::world_space) chooses with it.

pub use nova_view::RenderScale;

/// The world texture's size at `percent` of a `backing` content of that
/// many pixels across and down: each side `round(side * percent / 100)`,
/// never below one texel, so the content's aspect is kept within that
/// rounding.
#[must_use]
pub fn scaled_size(backing: (u32, u32), percent: u8) -> (u32, u32) {
    let side = |px: u32| {
        let scaled = (u64::from(px) * u64::from(percent) + 50) / 100;
        u32::try_from(scaled).unwrap_or(u32::MAX).max(1)
    };
    (side(backing.0), side(backing.1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_percent_of_the_backing_size_scales_each_side() {
        assert_eq!(scaled_size((2048, 1536), 50), (1024, 768));
        assert_eq!(scaled_size((5120, 2880), 67), (3430, 1930));
    }

    #[test]
    fn a_hundred_percent_is_the_backing_size() {
        for backing in [(2048, 1536), (5120, 2880), (1, 1), (1365, 768)] {
            assert_eq!(scaled_size(backing, 100), backing);
        }
    }

    #[test]
    fn a_tiny_viewport_never_gives_a_zero_side() {
        let (w, h) = scaled_size((3, 1), 25);
        assert!(w >= 1 && h >= 1, "{w}x{h}");
        assert_eq!(scaled_size((1, 1), 25), (1, 1));
    }

    #[test]
    fn rounding_keeps_the_aspect_within_a_texel() {
        for backing in [(2048, 1536), (5120, 2880), (1920, 1080), (1366, 767)] {
            for percent in RenderScale::MIN..=RenderScale::MAX {
                let (w, h) = scaled_size(backing, percent);
                let aspect = f64::from(backing.0) / f64::from(backing.1);
                let off = (f64::from(w) - f64::from(h) * aspect).abs();
                // Each side is off by at most half a texel, the height's
                // half scaled by the aspect.
                let most = f64::midpoint(1.0, aspect) + 1e-9;
                assert!(off <= most, "{backing:?} at {percent}%: {w}x{h}");
            }
        }
    }
}
