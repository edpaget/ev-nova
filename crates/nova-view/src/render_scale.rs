//! The render scale: the size flight's world layer is drawn at, before the
//! Scaling's [`Upscaler`](crate::Upscaler) enlarges it onto the viewport.
//!
//! It is a plain value, shared by the settings file, a future Preferences
//! control and the renderer that sizes the world texture with it
//! (`nova_render::world_space`).

/// The size the world layer is drawn at.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RenderScale {
    /// The size the Scaling draws the world at on its own: the window's
    /// resolution under Native (and Super-xBR), the logical resolution
    /// under the enlarging upscalers. It changes nothing.
    #[default]
    Auto,
    /// This percentage of the backing size (the viewport's content in
    /// target pixels), for every Scaling, from [`RenderScale::MIN`] to
    /// [`RenderScale::MAX`]. Build it with [`RenderScale::percent`], which
    /// clamps.
    Percent(u8),
}

impl RenderScale {
    /// The smallest percentage.
    pub const MIN: u8 = 25;
    /// The largest percentage: the backing size itself.
    pub const MAX: u8 = 100;

    /// `p` percent of the backing size, clamped to
    /// [`MIN`](Self::MIN)..=[`MAX`](Self::MAX).
    #[must_use]
    pub fn percent(p: u8) -> Self {
        Self::Percent(p.clamp(Self::MIN, Self::MAX))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_render_scale_is_auto() {
        assert_eq!(RenderScale::default(), RenderScale::Auto);
    }

    #[test]
    fn a_percent_is_clamped_to_25_through_100() {
        for (asked, kept) in [
            (0, 25),
            (24, 25),
            (25, 25),
            (67, 67),
            (100, 100),
            (101, 100),
            (255, 100),
        ] {
            assert_eq!(
                RenderScale::percent(asked),
                RenderScale::Percent(kept),
                "{asked}%"
            );
        }
    }
}
