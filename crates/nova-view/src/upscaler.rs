//! The frame upscaler: how flight's world layer is drawn onto the window,
//! the per-frame half of the player's [`Scaling`](crate::Scaling)
//! ([`Scaling::sites`](crate::Scaling::sites)).
//!
//! It is a plain value, shared by the Scaling the player picks and the
//! renderer that draws with it (`nova_render`'s upscaler port).

/// Which upscaler draws flight's world layer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Upscaler {
    /// No upscaler: the world is drawn at the window's resolution, as the
    /// overlay is.
    #[default]
    Native,
    /// The world is drawn at the logical resolution, one texel a unit,
    /// then enlarged onto the viewport with sharp-bilinear: nearest at
    /// whole scales, a one-pixel blend on the seams between texels at
    /// fractional ones.
    SharpBilinear,
    /// The world is drawn at the logical resolution, then enlarged onto
    /// the viewport with MetalFX's spatial scaler: edge-adaptive, with
    /// built-in sharpening. macOS on Metal only; elsewhere it is never
    /// offered.
    MetalFxSpatial,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_is_native() {
        assert_eq!(Upscaler::default(), Upscaler::Native);
    }
}
