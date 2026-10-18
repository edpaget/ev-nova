//! The frame upscaler the player picks in the Preferences dialog: how
//! flight's world layer is drawn onto the window.
//!
//! The choice is a plain value, shared by the dialog (which lists the ones
//! the running device can use), the settings that keep it, and the
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

impl Upscaler {
    /// Its entry in the Preferences dialog's list. The list's row has no
    /// caption of its own, so each entry names what it chooses.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Native => "Scaling: Native",
            Self::SharpBilinear => "Scaling: Sharp-bilinear",
            Self::MetalFxSpatial => "Scaling: MetalFX",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_is_native() {
        assert_eq!(Upscaler::default(), Upscaler::Native);
    }

    #[test]
    fn each_label_names_the_scaling_it_chooses() {
        assert_eq!(Upscaler::Native.label(), "Scaling: Native");
        assert_eq!(Upscaler::SharpBilinear.label(), "Scaling: Sharp-bilinear");
        assert_eq!(Upscaler::MetalFxSpatial.label(), "Scaling: MetalFX");
    }
}
