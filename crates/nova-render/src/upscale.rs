//! The frame upscaler: which one draws the world, which ones the device
//! can use, and what that makes of the world layer.
//!
//! An upscaler is chosen as a plain value ([`Upscaler`], `nova_view`'s,
//! which the Preferences dialog lists and the settings keep) and handed to
//! the GPU adapter ([`Gpu::set_upscaler`](crate::Gpu::set_upscaler)). The
//! adapter says how it draws the world layer
//! ([`Gpu::world_resolution`](crate::Gpu::world_resolution)), and the
//! core states each frame's [`WorldSpace`] from that ([`world_space`]).
//!
//! [`Upscaler::Native`] draws the world straight into the target at its
//! own resolution, which changes nothing on screen.
//! [`Upscaler::SharpBilinear`] draws it into a texture of the logical
//! size and enlarges that onto the viewport. [`Upscaler::MetalFxSpatial`]
//! does too, with MetalFX's spatial scaler, on macOS alone.
//!
//! Which upscalers the running device can use is the [`UpscalerProbe`]
//! port's answer; [`listed`] turns it into the list the player picks
//! from, native first and always. A GPU adapter answers from its
//! device's [`Abilities`] ([`offered`]) and draws each choice with what
//! [`effective`] says. A world and output size MetalFX won't scale
//! between ([`metal_fx_takes`], or MetalFX's own refusal) is drawn with
//! sharp-bilinear and logged once ([`RejectedSizes`]).
//!
//! The render scale ([`RenderScale`]) sizes the world texture on top of
//! the Scaling. At [`RenderScale::Auto`], the default, each Scaling draws
//! the world where it did before there was a render scale: Native and
//! Super-xBR (whose upscaler is native) into the target, sharp-bilinear,
//! FSR 1 and MetalFX into a texture of the logical size. At a
//! [`RenderScale::Percent`] every Scaling draws it into a texture that
//! share of the backing size, the viewport's content in target pixels
//! ([`world_space`]), which its upscaler enlarges onto the viewport; a
//! percentage of the backing size, not of the logical one, so a 5K
//! display under FSR 1 or MetalFX can draw the world at, say, 67% where
//! the logical size would be about 27%. Native has no enlarging pass of
//! its own, so below 100% it enlarges with sharp-bilinear
//! ([`enlarging`]); at 100% it draws into the target, as at `Auto`. The
//! overlay is drawn over the enlarged world at the target's resolution
//! whatever the scale, so the HUD stays sharp.

use crate::gpu::WorldSpace;
use crate::render_scale::{RenderScale, scaled_size};
use crate::viewport::Viewport;

pub use nova_view::Upscaler;

/// Says which per-frame upscalers the running device can use.
pub trait UpscalerProbe {
    /// The upscalers it can use, in the order to list them. Native may be
    /// left out: it is always available ([`listed`]).
    fn upscalers(&self) -> Vec<Upscaler>;
}

/// The upscalers to list from a probe's `answer`: native first, whatever
/// the answer says, then the answer's others in its order, each once.
#[must_use]
pub fn listed(answer: &[Upscaler]) -> Vec<Upscaler> {
    let mut list = vec![Upscaler::Native];
    for &upscaler in answer {
        if !list.contains(&upscaler) {
            list.push(upscaler);
        }
    }
    list
}

/// The resolution a GPU draws the world layer at.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WorldResolution {
    /// The target's own: the world is drawn straight into it.
    #[default]
    Native,
    /// The logical space's, one texel a unit, into a texture an upscaler
    /// enlarges onto the viewport.
    Logical,
}

/// The resolution `upscaler` draws the world layer at: native's own, and
/// the logical size for every upscaler that enlarges a world texture. A
/// GPU's [`Gpu::world_resolution`](crate::Gpu::world_resolution) answers
/// this for the upscaler it was set to.
#[must_use]
pub fn resolution_of(upscaler: Upscaler) -> WorldResolution {
    match upscaler {
        Upscaler::Native => WorldResolution::Native,
        Upscaler::SharpBilinear | Upscaler::Fsr1 | Upscaler::MetalFxSpatial => {
            WorldResolution::Logical
        }
    }
}

/// What a GPU device can draw the world with beyond what every device
/// can (native, sharp-bilinear and FSR 1's two plain fragment passes),
/// found once when its adapter is made.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Abilities {
    /// MetalFX's spatial scaler runs on it: a macOS build, the Metal
    /// backend, and a GPU MetalFX supports. Never on Windows or Linux.
    pub metal_fx_spatial: bool,
}

/// The upscalers a device with `abilities` offers, in the order to list
/// them: the portable ones always (native, sharp-bilinear and FSR 1),
/// then MetalFX, the platform one, last when it can.
#[must_use]
pub fn offered(abilities: Abilities) -> Vec<Upscaler> {
    let mut upscalers = vec![Upscaler::Native, Upscaler::SharpBilinear, Upscaler::Fsr1];
    if abilities.metal_fx_spatial {
        upscalers.push(Upscaler::MetalFxSpatial);
    }
    upscalers
}

/// The upscaler that draws `choice` on a device with `abilities`: MetalFX
/// on a device without it is drawn with sharp-bilinear, the nearest it
/// has; every other choice (FSR 1 among them, which every device runs)
/// as it is. MetalFX is never offered there
/// ([`offered`]), so only a direct
/// [`Gpu::set_upscaler`](crate::Gpu::set_upscaler) can ask for it.
#[must_use]
pub fn effective(choice: Upscaler, abilities: Abilities) -> Upscaler {
    match choice {
        Upscaler::MetalFxSpatial if !abilities.metal_fx_spatial => Upscaler::SharpBilinear,
        _ => choice,
    }
}

/// Whether MetalFX's spatial scaler can scale an `input` world onto an
/// `output` rectangle (each width, height): any size but one with a zero
/// side, which aborts the process inside MetalFX rather than failing
/// (phase 7's spike). Shrinking and 1x are taken.
#[must_use]
pub fn metal_fx_takes(input: (u32, u32), output: (u32, u32)) -> bool {
    ![input.0, input.1, output.0, output.1].contains(&0)
}

/// The world and output sizes MetalFX would not scale between, each
/// drawn with sharp-bilinear instead and logged once.
#[derive(Clone, Debug, Default)]
pub struct RejectedSizes {
    pairs: Vec<((u32, u32), (u32, u32))>,
}

impl RejectedSizes {
    /// Whether scaling `input` onto `output` was rejected before.
    #[must_use]
    pub fn rejected(&self, input: (u32, u32), output: (u32, u32)) -> bool {
        self.pairs.contains(&(input, output))
    }

    /// Records that MetalFX rejected scaling `input` onto `output`: the
    /// line to log the first time for the pair, `None` after.
    pub fn reject(&mut self, input: (u32, u32), output: (u32, u32)) -> Option<String> {
        if self.rejected(input, output) {
            return None;
        }
        self.pairs.push((input, output));
        let ((in_w, in_h), (out_w, out_h)) = (input, output);
        Some(format!(
            "nova: MetalFX can't scale the world from {in_w}x{in_h} to {out_w}x{out_h}; \
             drawing it with sharp-bilinear"
        ))
    }
}

/// Where a frame drawn into `viewport` draws its world, at `resolution`
/// and `scale`.
///
/// At [`RenderScale::Auto`] the resolution alone decides: straight into
/// the target at the native resolution, and into a texture of the logical
/// size at the logical one. At [`RenderScale::Percent`] (clamped as
/// [`RenderScale::percent`] clamps) the world is drawn into a texture of
/// that share of the viewport's content ([`scaled_size`]), whatever the
/// resolution, except a native world at 100%, which is the target itself.
/// A zero-area viewport has no content to take a share of, so it draws
/// where [`RenderScale::Auto`] would.
#[must_use]
pub fn world_space(
    resolution: WorldResolution,
    scale: RenderScale,
    viewport: &Viewport,
) -> WorldSpace {
    let percent = match scale {
        RenderScale::Auto => None,
        RenderScale::Percent(p) => Some(p.clamp(RenderScale::MIN, RenderScale::MAX)),
    };
    let native_whole = |&p: &u8| resolution == WorldResolution::Native && p == RenderScale::MAX;
    let percent = percent.filter(|p| !native_whole(p));
    let content = viewport.content().map(|(rect, _)| (rect.w, rect.h));
    if let (Some(percent), Some(backing)) = (percent, content) {
        return WorldSpace::Texture {
            size: scaled_size(backing, percent),
        };
    }
    match resolution {
        WorldResolution::Native => WorldSpace::Target,
        WorldResolution::Logical => {
            let logical = viewport.logical();
            WorldSpace::Texture {
                size: (logical.w, logical.h),
            }
        }
    }
}

/// The upscaler that enlarges a texture world when `upscaler` is the one
/// chosen: native has no enlarging pass of its own, so a native world
/// drawn below 100% ([`world_space`]) is enlarged with sharp-bilinear;
/// every other upscaler enlarges with itself.
#[must_use]
pub fn enlarging(upscaler: Upscaler) -> Upscaler {
    match upscaler {
        Upscaler::Native => Upscaler::SharpBilinear,
        Upscaler::SharpBilinear | Upscaler::Fsr1 | Upscaler::MetalFxSpatial => upscaler,
    }
}

#[cfg(test)]
mod tests {
    use nova_view::{Scaling, ViewShape};

    use super::*;
    use crate::viewport::LOGICAL;

    #[test]
    fn native_draws_the_world_natively_and_the_enlargers_at_the_logical_size() {
        assert_eq!(resolution_of(Upscaler::Native), WorldResolution::Native);
        assert_eq!(
            resolution_of(Upscaler::SharpBilinear),
            WorldResolution::Logical
        );
        assert_eq!(resolution_of(Upscaler::Fsr1), WorldResolution::Logical);
        assert_eq!(
            resolution_of(Upscaler::MetalFxSpatial),
            WorldResolution::Logical
        );
    }

    const WITH_METAL_FX: Abilities = Abilities {
        metal_fx_spatial: true,
    };
    const WITHOUT: Abilities = Abilities {
        metal_fx_spatial: false,
    };

    #[test]
    fn every_device_offers_native_sharp_bilinear_and_fsr1() {
        assert_eq!(
            offered(WITHOUT),
            [Upscaler::Native, Upscaler::SharpBilinear, Upscaler::Fsr1]
        );
        assert_eq!(Abilities::default(), WITHOUT);
    }

    #[test]
    fn a_device_with_metal_fx_offers_it_last_after_the_portable_ones() {
        assert_eq!(
            offered(WITH_METAL_FX),
            [
                Upscaler::Native,
                Upscaler::SharpBilinear,
                Upscaler::Fsr1,
                Upscaler::MetalFxSpatial
            ]
        );
    }

    #[test]
    fn metal_fx_without_the_ability_is_drawn_with_sharp_bilinear() {
        assert_eq!(
            effective(Upscaler::MetalFxSpatial, WITHOUT),
            Upscaler::SharpBilinear
        );
        assert_eq!(
            effective(Upscaler::MetalFxSpatial, WITH_METAL_FX),
            Upscaler::MetalFxSpatial
        );
    }

    #[test]
    fn every_scaling_listed_is_drawn_as_itself_and_super_xbr_draws_the_world_natively() {
        for abilities in [WITHOUT, WITH_METAL_FX] {
            let list = Scaling::list(&offered(abilities));
            assert_eq!(list.last(), Some(&Scaling::SuperXbr));
            for entry in list {
                let upscaler = entry.sites().0;
                assert_eq!(effective(upscaler, abilities), upscaler, "{entry:?}");
            }
        }
        assert_eq!(
            resolution_of(Scaling::SuperXbr.sites().0),
            WorldResolution::Native
        );
    }

    #[test]
    fn every_other_choice_is_drawn_as_chosen() {
        for abilities in [WITHOUT, WITH_METAL_FX] {
            for choice in [Upscaler::Native, Upscaler::SharpBilinear, Upscaler::Fsr1] {
                assert_eq!(effective(choice, abilities), choice);
            }
        }
    }

    #[test]
    fn metal_fx_takes_any_size_without_a_zero_side() {
        assert!(metal_fx_takes((1024, 768), (1920, 1440)));
        assert!(metal_fx_takes((1, 1), (2, 2)));
        assert!(metal_fx_takes((64, 48), (32, 24)), "a shrink");
        assert!(metal_fx_takes((64, 48), (64, 48)), "1x");
        assert!(!metal_fx_takes((0, 768), (1920, 1440)));
        assert!(!metal_fx_takes((1024, 0), (1920, 1440)));
        assert!(!metal_fx_takes((1024, 768), (0, 1440)));
        assert!(!metal_fx_takes((1024, 768), (1920, 0)));
    }

    const CLASSIC: (u32, u32) = (1024, 768);

    #[test]
    fn a_rejected_size_is_logged_once() {
        let mut rejected = RejectedSizes::default();
        assert!(!rejected.rejected(CLASSIC, (1920, 1440)));
        assert_eq!(
            rejected.reject(CLASSIC, (1920, 1440)).as_deref(),
            Some(
                "nova: MetalFX can't scale the world from 1024x768 to 1920x1440; \
                 drawing it with sharp-bilinear"
            )
        );
        assert!(rejected.rejected(CLASSIC, (1920, 1440)));
        assert_eq!(rejected.reject(CLASSIC, (1920, 1440)), None, "once");
    }

    #[test]
    fn each_rejected_size_is_logged_once_of_its_own() {
        let mut rejected = RejectedSizes::default();
        assert!(rejected.reject(CLASSIC, (1920, 1440)).is_some());
        let other = rejected.reject((1365, 768), (2560, 1440));
        assert_eq!(
            other.as_deref(),
            Some(
                "nova: MetalFX can't scale the world from 1365x768 to 2560x1440; \
                 drawing it with sharp-bilinear"
            )
        );
        assert!(!rejected.rejected(CLASSIC, (2560, 1440)), "only the pairs");
        assert!(!rejected.rejected((1365, 768), (1920, 1440)));
        assert_eq!(rejected.reject(CLASSIC, (1920, 1440)), None, "back again");
        assert_eq!(rejected.reject((1365, 768), (2560, 1440)), None);
    }

    #[test]
    fn a_native_world_is_drawn_into_the_target() {
        let viewport = Viewport::new(LOGICAL, (2048, 1536), 2.0);
        assert_eq!(
            world_space(WorldResolution::Native, RenderScale::Auto, &viewport),
            WorldSpace::Target
        );
        assert_eq!(Upscaler::default(), Upscaler::Native);
        assert_eq!(WorldResolution::default(), WorldResolution::Native);
    }

    #[test]
    fn a_logical_world_is_drawn_into_a_texture_of_the_logical_size() {
        let classic = Viewport::new(LOGICAL, (2048, 1536), 2.0);
        assert_eq!(
            world_space(WorldResolution::Logical, RenderScale::Auto, &classic),
            WorldSpace::Texture { size: (1024, 768) }
        );
        let widened = Viewport::for_shape(ViewShape::Widened, (1920, 1080), 1.0);
        let logical = widened.logical();
        assert_eq!((logical.w, logical.h), (1365, 768));
        assert_eq!(
            world_space(WorldResolution::Logical, RenderScale::Auto, &widened),
            WorldSpace::Texture { size: (1365, 768) }
        );
    }

    #[test]
    fn auto_draws_the_world_where_each_resolution_did() {
        let widened = Viewport::for_shape(ViewShape::Widened, (1920, 1080), 1.0);
        for viewport in [Viewport::new(LOGICAL, (2048, 1536), 2.0), widened] {
            let logical = viewport.logical();
            assert_eq!(
                world_space(WorldResolution::Native, RenderScale::Auto, &viewport),
                WorldSpace::Target
            );
            assert_eq!(
                world_space(WorldResolution::Logical, RenderScale::Auto, &viewport),
                WorldSpace::Texture {
                    size: (logical.w, logical.h)
                }
            );
        }
    }

    #[test]
    fn a_native_world_at_a_hundred_percent_stays_in_the_target() {
        let viewport = Viewport::new(LOGICAL, (2048, 1536), 2.0);
        assert_eq!(
            world_space(
                WorldResolution::Native,
                RenderScale::percent(100),
                &viewport
            ),
            WorldSpace::Target
        );
    }

    #[test]
    fn a_native_world_below_a_hundred_percent_is_a_texture_of_that_share_of_the_viewport() {
        let viewport = Viewport::new(LOGICAL, (2048, 1536), 2.0);
        assert_eq!(
            world_space(WorldResolution::Native, RenderScale::percent(50), &viewport),
            WorldSpace::Texture { size: (1024, 768) }
        );
        assert_eq!(
            world_space(WorldResolution::Native, RenderScale::percent(99), &viewport),
            WorldSpace::Texture { size: (2028, 1521) }
        );
    }

    #[test]
    fn an_enlarging_upscalers_world_at_a_percent_is_sized_from_the_backing_not_the_logical_size() {
        let widened = Viewport::for_shape(ViewShape::Widened, (1920, 1080), 1.0);
        assert_eq!(
            world_space(WorldResolution::Logical, RenderScale::percent(50), &widened),
            WorldSpace::Texture { size: (960, 540) }
        );
        let retina = Viewport::new(LOGICAL, (2048, 1536), 2.0);
        assert_eq!(
            world_space(WorldResolution::Logical, RenderScale::percent(100), &retina),
            WorldSpace::Texture { size: (2048, 1536) },
            "the backing size, enlarged 1x"
        );
    }

    #[test]
    fn a_letterboxed_viewport_scales_its_content_not_the_window() {
        // 2560 wide fits 1024 at 2x, 2048 wide, with bars either side.
        let viewport = Viewport::new(LOGICAL, (2560, 1536), 2.0);
        for resolution in [WorldResolution::Native, WorldResolution::Logical] {
            assert_eq!(
                world_space(resolution, RenderScale::percent(50), &viewport),
                WorldSpace::Texture { size: (1024, 768) },
                "{resolution:?}"
            );
        }
    }

    #[test]
    fn a_percent_built_out_of_range_is_drawn_clamped() {
        let viewport = Viewport::new(LOGICAL, (2048, 1536), 2.0);
        assert_eq!(
            world_space(
                WorldResolution::Native,
                RenderScale::Percent(200),
                &viewport
            ),
            WorldSpace::Target
        );
        assert_eq!(
            world_space(WorldResolution::Logical, RenderScale::Percent(0), &viewport),
            WorldSpace::Texture { size: (512, 384) }
        );
    }

    #[test]
    fn a_zero_area_viewport_at_a_percent_draws_where_auto_would() {
        let viewport = Viewport::new(LOGICAL, (0, 0), 1.0);
        for resolution in [WorldResolution::Native, WorldResolution::Logical] {
            assert_eq!(
                world_space(resolution, RenderScale::percent(50), &viewport),
                world_space(resolution, RenderScale::Auto, &viewport),
                "{resolution:?}"
            );
        }
    }

    #[test]
    fn native_is_enlarged_with_sharp_bilinear_and_every_other_upscaler_with_itself() {
        assert_eq!(enlarging(Upscaler::Native), Upscaler::SharpBilinear);
        for upscaler in [
            Upscaler::SharpBilinear,
            Upscaler::Fsr1,
            Upscaler::MetalFxSpatial,
        ] {
            assert_eq!(enlarging(upscaler), upscaler);
        }
    }

    /// Answers what it is given.
    struct Probe(Vec<Upscaler>);

    impl UpscalerProbe for Probe {
        fn upscalers(&self) -> Vec<Upscaler> {
            self.0.clone()
        }
    }

    fn listed_from(answer: &[Upscaler]) -> Vec<Upscaler> {
        listed(&Probe(answer.to_vec()).upscalers())
    }

    const NATIVE: Upscaler = Upscaler::Native;
    const SHARP: Upscaler = Upscaler::SharpBilinear;

    #[test]
    fn native_is_listed_first_whatever_the_probe_says() {
        assert_eq!(listed_from(&[SHARP]), [NATIVE, SHARP]);
        assert_eq!(listed_from(&[SHARP, NATIVE]), [NATIVE, SHARP]);
        assert_eq!(listed_from(&[NATIVE, SHARP]), [NATIVE, SHARP]);
        assert_eq!(listed_from(&[]), [NATIVE]);
    }

    #[test]
    fn an_upscaler_answered_twice_is_listed_once() {
        assert_eq!(listed_from(&[SHARP, SHARP]), [NATIVE, SHARP]);
        assert_eq!(listed_from(&[NATIVE, NATIVE]), [NATIVE]);
    }
}
