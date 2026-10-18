//! The Scaling the player picks in the Preferences dialog: how the game is
//! enlarged onto the window, one choice among alternatives that work at
//! two sites.
//!
//! The per-frame site enlarges flight's world as each frame is drawn (an
//! [`Upscaler`]); the load-time site enlarges sprites and pictures once,
//! as they load into the atlas (an [`Enlargement`]). The player picks one
//! entry of one list, and [`Scaling::sites`] says what each site does for
//! it. The choice is a plain value, shared by the dialog, the settings that
//! keep it, and the app that applies both halves (`nova_render`'s
//! upscaler and enlarger ports).

use crate::upscaler::Upscaler;

/// How the game is enlarged onto the window: the player's one choice.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Scaling {
    /// Not at all: the world is drawn at the window's resolution, and
    /// images as they decode.
    #[default]
    Native,
    /// Flight's world drawn at the logical resolution, then enlarged with
    /// sharp-bilinear ([`Upscaler::SharpBilinear`]).
    SharpBilinear,
    /// Flight's world drawn at the logical resolution, then enlarged with
    /// MetalFX's spatial scaler ([`Upscaler::MetalFxSpatial`]). Listed only
    /// where the device has it.
    MetalFxSpatial,
    /// Sprites and pictures doubled with Hyllian's Super-xBR as they load
    /// ([`Enlargement::SuperXbr`]), the world drawn natively. Every device
    /// has it: it runs on the CPU.
    SuperXbr,
}

/// How the load-time site enlarges images as they load into the atlas.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Enlargement {
    /// Not at all: the atlas holds each image as it decodes.
    #[default]
    Native,
    /// Each image but a pattern doubled with Super-xBR, drawn at its own
    /// logical size.
    SuperXbr,
}

impl Scaling {
    /// Every scaling, in the order the Preferences dialog lists them.
    pub const ALL: [Self; 4] = [
        Self::Native,
        Self::SharpBilinear,
        Self::MetalFxSpatial,
        Self::SuperXbr,
    ];

    /// What each site does for it: the per-frame upscaler drawing flight's
    /// world, and the load-time enlargement of images. Each per-frame
    /// scaling loads images natively, and Super-xBR draws the world
    /// natively.
    #[must_use]
    pub fn sites(self) -> (Upscaler, Enlargement) {
        match self {
            Self::Native => (Upscaler::Native, Enlargement::Native),
            Self::SharpBilinear => (Upscaler::SharpBilinear, Enlargement::Native),
            Self::MetalFxSpatial => (Upscaler::MetalFxSpatial, Enlargement::Native),
            Self::SuperXbr => (Upscaler::Native, Enlargement::SuperXbr),
        }
    }

    /// The Preferences dialog's list: `upscalers`, the per-frame ones the
    /// device can use as `nova_render::listed` orders them (native
    /// first), then Super-xBR, which every device has.
    #[must_use]
    pub fn list(upscalers: &[Upscaler]) -> Vec<Self> {
        upscalers
            .iter()
            .map(|&upscaler| Self::from(upscaler))
            .chain([Self::SuperXbr])
            .collect()
    }

    /// Its entry in the Preferences dialog's list. The list's row has no
    /// caption of its own, so each entry names what it chooses.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Native => "Scaling: Native",
            Self::SharpBilinear => "Scaling: Sharp-bilinear",
            Self::MetalFxSpatial => "Scaling: MetalFX",
            Self::SuperXbr => "Scaling: Super-xBR",
        }
    }
}

impl From<Upscaler> for Scaling {
    /// The scaling `upscaler` draws alone, images loading natively.
    fn from(upscaler: Upscaler) -> Self {
        match upscaler {
            Upscaler::Native => Self::Native,
            Upscaler::SharpBilinear => Self::SharpBilinear,
            Upscaler::MetalFxSpatial => Self::MetalFxSpatial,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_native() {
        assert_eq!(Scaling::default(), Scaling::Native);
        assert_eq!(Enlargement::default(), Enlargement::Native);
    }

    #[test]
    fn every_scaling_is_listed_in_list_order() {
        assert_eq!(
            Scaling::ALL,
            [
                Scaling::Native,
                Scaling::SharpBilinear,
                Scaling::MetalFxSpatial,
                Scaling::SuperXbr
            ]
        );
    }

    #[test]
    fn each_label_names_the_scaling_it_chooses() {
        assert_eq!(Scaling::Native.label(), "Scaling: Native");
        assert_eq!(Scaling::SharpBilinear.label(), "Scaling: Sharp-bilinear");
        assert_eq!(Scaling::MetalFxSpatial.label(), "Scaling: MetalFX");
        assert_eq!(Scaling::SuperXbr.label(), "Scaling: Super-xBR");
    }

    #[test]
    fn each_scaling_works_at_its_own_site() {
        assert_eq!(
            Scaling::Native.sites(),
            (Upscaler::Native, Enlargement::Native)
        );
        assert_eq!(
            Scaling::SharpBilinear.sites(),
            (Upscaler::SharpBilinear, Enlargement::Native)
        );
        assert_eq!(
            Scaling::MetalFxSpatial.sites(),
            (Upscaler::MetalFxSpatial, Enlargement::Native)
        );
        assert_eq!(
            Scaling::SuperXbr.sites(),
            (Upscaler::Native, Enlargement::SuperXbr)
        );
    }

    #[test]
    fn each_upscaler_is_the_scaling_drawn_by_it_alone() {
        for upscaler in [
            Upscaler::Native,
            Upscaler::SharpBilinear,
            Upscaler::MetalFxSpatial,
        ] {
            assert_eq!(
                Scaling::from(upscaler).sites(),
                (upscaler, Enlargement::Native),
                "{upscaler:?}"
            );
        }
    }

    #[test]
    fn the_list_is_the_upscalers_listed_then_super_xbr() {
        use Scaling::{MetalFxSpatial, Native, SharpBilinear, SuperXbr};
        assert_eq!(Scaling::list(&[Upscaler::Native]), [Native, SuperXbr]);
        assert_eq!(
            Scaling::list(&[Upscaler::Native, Upscaler::SharpBilinear]),
            [Native, SharpBilinear, SuperXbr]
        );
        assert_eq!(
            Scaling::list(&[
                Upscaler::Native,
                Upscaler::SharpBilinear,
                Upscaler::MetalFxSpatial
            ]),
            [Native, SharpBilinear, MetalFxSpatial, SuperXbr]
        );
    }

    #[test]
    fn super_xbr_alone_enlarges_as_images_load() {
        let enlarging: Vec<Scaling> = Scaling::ALL
            .into_iter()
            .filter(|scaling| scaling.sites().1 != Enlargement::Native)
            .collect();
        assert_eq!(enlarging, [Scaling::SuperXbr]);
    }
}
