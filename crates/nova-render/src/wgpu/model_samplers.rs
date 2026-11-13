//! The wgpu samplers the models pass binds: one for each distinct
//! [`ModelSampler`] a mesh's sections name, made when the mesh is
//! uploaded and kept ([`Samplers`]), never on a frame.
//!
//! [`descriptor`] mirrors [`crate::model_sampler::sample`], value by
//! value. A sampler that reads no mips ([`Mip::None`]) can't simply be
//! given no levels: the texture keeps its whole chain, as another sampler
//! may read it. It is held to the first level by a level-of-detail clamp
//! of 0 to 0.25 with the nearest mip, the Vulkan specification's mapping
//! of OpenGL's `NEAREST` and `LINEAR` minification: the level is always
//! the first, and the minification filter still applies when the texture
//! is drawn small (a clamp of 0 to 0 would make some backends treat every
//! sample as magnified).

use std::collections::HashMap;

use crate::model_sampler::{Filter, Mip, ModelSampler, Wrap};

/// The most a sampler that reads no mips lets the level of detail reach:
/// above 0, so a minified texture is still read through its minification
/// filter, and under a half, so the nearest level is the first.
const NO_MIPS_MAX_LOD: f32 = 0.25;

/// The wgpu sampler that samples as `sampler` says.
pub(super) fn descriptor(sampler: ModelSampler) -> wgpu::SamplerDescriptor<'static> {
    let address = |wrap| match wrap {
        Wrap::Repeat => wgpu::AddressMode::Repeat,
        Wrap::Mirror => wgpu::AddressMode::MirrorRepeat,
        Wrap::Clamp => wgpu::AddressMode::ClampToEdge,
    };
    let filter = |filter| match filter {
        Filter::Nearest => wgpu::FilterMode::Nearest,
        Filter::Linear => wgpu::FilterMode::Linear,
    };
    let unclamped = wgpu::SamplerDescriptor::<'static>::default().lod_max_clamp;
    let (mipmap_filter, lod_max_clamp) = match sampler.mip {
        Mip::None => (wgpu::MipmapFilterMode::Nearest, NO_MIPS_MAX_LOD),
        Mip::Nearest => (wgpu::MipmapFilterMode::Nearest, unclamped),
        Mip::Linear => (wgpu::MipmapFilterMode::Linear, unclamped),
    };
    wgpu::SamplerDescriptor {
        label: Some("nova model texture"),
        address_mode_u: address(sampler.wrap_s),
        address_mode_v: address(sampler.wrap_t),
        mag_filter: filter(sampler.mag),
        min_filter: filter(sampler.min),
        mipmap_filter,
        lod_max_clamp,
        ..Default::default()
    }
}

/// The samplers made so far, by what they do.
#[derive(Default)]
pub(super) struct Samplers {
    made: HashMap<ModelSampler, wgpu::Sampler>,
}

impl Samplers {
    /// The sampler for `sampler`, made on `device` the first time it is
    /// asked for.
    pub(super) fn get(&mut self, device: &wgpu::Device, sampler: ModelSampler) -> &wgpu::Sampler {
        self.made
            .entry(sampler)
            .or_insert_with(|| device.create_sampler(&descriptor(sampler)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with(change: impl FnOnce(&mut ModelSampler)) -> wgpu::SamplerDescriptor<'static> {
        let mut sampler = ModelSampler::DEFAULT;
        change(&mut sampler);
        descriptor(sampler)
    }

    #[test]
    fn the_default_is_the_repeating_trilinear_sampler() {
        assert_eq!(
            descriptor(ModelSampler::DEFAULT),
            wgpu::SamplerDescriptor {
                label: Some("nova model texture"),
                address_mode_u: wgpu::AddressMode::Repeat,
                address_mode_v: wgpu::AddressMode::Repeat,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::MipmapFilterMode::Linear,
                ..Default::default()
            }
        );
    }

    #[test]
    fn each_wrap_is_its_address_mode_on_its_own_axis() {
        for (wrap, mode) in [
            (Wrap::Repeat, wgpu::AddressMode::Repeat),
            (Wrap::Mirror, wgpu::AddressMode::MirrorRepeat),
            (Wrap::Clamp, wgpu::AddressMode::ClampToEdge),
        ] {
            // The other axis clamps when this one doesn't, and repeats
            // when it does.
            let other = if wrap == Wrap::Clamp {
                (Wrap::Repeat, wgpu::AddressMode::Repeat)
            } else {
                (Wrap::Clamp, wgpu::AddressMode::ClampToEdge)
            };
            let along_u = with(|s| (s.wrap_s, s.wrap_t) = (wrap, other.0));
            assert_eq!(along_u.address_mode_u, mode, "{wrap:?} along u");
            assert_eq!(along_u.address_mode_v, other.1, "{wrap:?} along u");
            let along_v = with(|s| (s.wrap_s, s.wrap_t) = (other.0, wrap));
            assert_eq!(along_v.address_mode_u, other.1, "{wrap:?} along v");
            assert_eq!(along_v.address_mode_v, mode, "{wrap:?} along v");
        }
    }

    #[test]
    fn each_filter_is_its_filter_mode_magnified_and_minified_apart() {
        let sharp_near = with(|s| (s.mag, s.min) = (Filter::Nearest, Filter::Linear));
        assert_eq!(sharp_near.mag_filter, wgpu::FilterMode::Nearest);
        assert_eq!(sharp_near.min_filter, wgpu::FilterMode::Linear);
        let sharp_far = with(|s| (s.mag, s.min) = (Filter::Linear, Filter::Nearest));
        assert_eq!(sharp_far.mag_filter, wgpu::FilterMode::Linear);
        assert_eq!(sharp_far.min_filter, wgpu::FilterMode::Nearest);
    }

    #[test]
    #[allow(clippy::float_cmp, reason = "the clamps are constants")]
    fn each_mip_is_its_mipmap_filter_and_no_mips_holds_the_detail_to_the_first_level() {
        let nearest = with(|s| s.mip = Mip::Nearest);
        assert_eq!(nearest.mipmap_filter, wgpu::MipmapFilterMode::Nearest);
        assert_eq!((nearest.lod_min_clamp, nearest.lod_max_clamp), (0.0, 32.0));
        let linear = with(|s| s.mip = Mip::Linear);
        assert_eq!(linear.mipmap_filter, wgpu::MipmapFilterMode::Linear);
        assert_eq!((linear.lod_min_clamp, linear.lod_max_clamp), (0.0, 32.0));
        let none = with(|s| s.mip = Mip::None);
        assert_eq!(none.mipmap_filter, wgpu::MipmapFilterMode::Nearest);
        assert_eq!((none.lod_min_clamp, none.lod_max_clamp), (0.0, 0.25));
    }

    #[test]
    fn every_sampler_keeps_what_the_models_pass_fixes() {
        // No anisotropy, no comparison, and the label.
        for mip in [Mip::None, Mip::Nearest, Mip::Linear] {
            let made = with(|s| (s.mip, s.mag, s.wrap_s) = (mip, Filter::Nearest, Wrap::Mirror));
            assert_eq!(made.label, Some("nova model texture"));
            assert_eq!(made.anisotropy_clamp, 1);
            assert_eq!(made.compare, None);
        }
    }
}
