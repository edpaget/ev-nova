//! The FFI: MetalFX's spatial scaler over wgpu's Metal backend.
//!
//! Every `as_hal` guard holds a read lock on the device's destruction
//! lock, and wgpu panics if a thread takes a second while holding one, so
//! each raw Metal object is retained and its guard dropped at once.

#![allow(unsafe_code)]

use objc2::Message;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::{
    MTLDevice, MTLPixelFormat, MTLStorageMode, MTLTexture, MTLTextureDescriptor, MTLTextureType,
    MTLTextureUsage,
};
use objc2_metal_fx::{
    MTLFXSpatialScaler, MTLFXSpatialScalerBase, MTLFXSpatialScalerColorProcessingMode,
    MTLFXSpatialScalerDescriptor,
};
use wgpu::hal::api::Metal;

/// The one texture format the scaler reads and writes, in wgpu's terms.
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// [`FORMAT`] in Metal's terms.
const METAL_FORMAT: MTLPixelFormat = MTLPixelFormat::RGBA8Unorm;

/// The raw Metal device under `device`, or `None` off the Metal backend.
fn raw_device(device: &wgpu::Device) -> Option<Retained<ProtocolObject<dyn MTLDevice>>> {
    // SAFETY: `Device::as_hal`'s contract: the raw device is never
    // destroyed here (it is only retained, which keeps it alive), and the
    // guard is dropped before this returns.
    let hal = unsafe { device.as_hal::<Metal>() }?;
    Some(hal.raw_device().clone())
}

/// The raw Metal texture under `texture`.
///
/// # Panics
///
/// If `texture` is not on the Metal backend.
fn raw_texture(texture: &wgpu::Texture) -> Retained<ProtocolObject<dyn MTLTexture>> {
    // SAFETY: `Texture::as_hal`'s contract: the raw texture is never
    // destroyed here (it is only retained), and the guard is dropped
    // before this returns.
    let hal = unsafe { texture.as_hal::<Metal>() }.expect("a texture on the Metal backend");
    hal.raw_handle().retain()
}

/// Whether MetalFX's spatial scaler runs on `device`: it is on the Metal
/// backend and MetalFX says it supports its GPU. Sizes aren't asked: the
/// scaler takes any size but a zero side ([`SpatialScaler::new`]).
#[must_use]
pub fn supported(device: &wgpu::Device) -> bool {
    raw_device(device).is_some_and(|raw| {
        // SAFETY: a plain query of a live device.
        unsafe { MTLFXSpatialScalerDescriptor::supportsDevice(&raw) }
    })
}

/// A MetalFX spatial scaler for one input size and one output size.
pub struct SpatialScaler {
    raw: Retained<ProtocolObject<dyn MTLFXSpatialScaler>>,
    device: Retained<ProtocolObject<dyn MTLDevice>>,
    input: (u32, u32),
    output: (u32, u32),
}

impl SpatialScaler {
    /// A scaler on `device` from `input` to `output` (width, height), or
    /// `None` off the Metal backend, for a zero side, or when MetalFX
    /// makes none.
    ///
    /// A zero side is refused before MetalFX is asked: making a scaler
    /// with a zero output side aborts the process.
    #[must_use]
    pub fn new(device: &wgpu::Device, input: (u32, u32), output: (u32, u32)) -> Option<Self> {
        if [input.0, input.1, output.0, output.1].contains(&0) {
            return None;
        }
        let raw_device = raw_device(device)?;
        // SAFETY: plain property setters on a fresh descriptor, then a
        // constructor that answers nil on failure. No side is zero.
        let raw = unsafe {
            let descriptor = MTLFXSpatialScalerDescriptor::new();
            descriptor.setColorTextureFormat(METAL_FORMAT);
            descriptor.setOutputTextureFormat(METAL_FORMAT);
            descriptor.setInputWidth(input.0 as usize);
            descriptor.setInputHeight(input.1 as usize);
            descriptor.setOutputWidth(output.0 as usize);
            descriptor.setOutputHeight(output.1 as usize);
            descriptor.setColorProcessingMode(MTLFXSpatialScalerColorProcessingMode::Perceptual);
            descriptor.newSpatialScalerWithDevice(&raw_device)
        }?;
        Some(Self {
            raw,
            device: raw_device,
            input,
            output,
        })
    }

    /// The input size (width, height) it scales from.
    #[must_use]
    pub fn input(&self) -> (u32, u32) {
        self.input
    }

    /// The output size (width, height) it scales to.
    #[must_use]
    pub fn output(&self) -> (u32, u32) {
        self.output
    }

    /// The Metal usage MetalFX needs of the world texture.
    fn color_usage(&self) -> MTLTextureUsage {
        // SAFETY: a plain property read.
        unsafe { self.raw.colorTextureUsage() }
    }

    /// A texture for [`encode`](Self::encode) to write into: the output
    /// size, RGBA8, private storage (MetalFX's assertion demands it), with
    /// the Metal usage MetalFX asks for plus shader reads, and wgpu's
    /// `COPY_SRC | TEXTURE_BINDING`.
    ///
    /// It is made on the raw device and wrapped, so wgpu takes it as
    /// initialised and never clears it: a clear would wipe MetalFX's
    /// write, which wgpu can't see.
    ///
    /// # Panics
    ///
    /// If `device` is not the one the scaler was made on, or Metal can't
    /// make the texture.
    #[must_use]
    pub fn output_texture(&self, device: &wgpu::Device) -> wgpu::Texture {
        let raw_device = self.same_device(device);
        let (width, height) = self.output;
        // SAFETY: a plain descriptor constructor and property reads.
        let (descriptor, usage) = unsafe {
            (
                MTLTextureDescriptor::texture2DDescriptorWithPixelFormat_width_height_mipmapped(
                    METAL_FORMAT,
                    width as usize,
                    height as usize,
                    false,
                ),
                self.raw.outputTextureUsage(),
            )
        };
        descriptor.setUsage(usage | MTLTextureUsage::ShaderRead);
        descriptor.setStorageMode(MTLStorageMode::Private);
        let raw = raw_device
            .newTextureWithDescriptor(&descriptor)
            .expect("Metal makes the output texture");
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        // SAFETY: `texture_from_raw` wraps a 2D, one-layer, one-level
        // RGBA8 texture of exactly `size`, which is what `raw` is; and
        // `create_texture_from_hal`'s contract holds: the hal texture was
        // made on `device`'s own Metal device (`same_device`), it matches
        // the descriptor below (size, format, one level, one sample, 2D;
        // `COPY_SRC` and `TEXTURE_BINDING` are blits and shader reads,
        // which `ShaderRead` covers), and it is initialised as far as wgpu
        // can tell: it is only ever read after `encode` has written all
        // of it, earlier in the same submission.
        unsafe {
            let hal = wgpu::hal::metal::Device::texture_from_raw(
                raw,
                FORMAT,
                MTLTextureType::Type2D,
                1,
                1,
                wgpu::hal::CopyExtent {
                    width,
                    height,
                    depth: 1,
                },
                None,
            );
            device.create_texture_from_hal::<Metal>(
                hal,
                &wgpu::TextureDescriptor {
                    label: Some("nova MetalFX output"),
                    size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: FORMAT,
                    usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                },
                wgpu::TextureUses::UNINITIALIZED,
            )
        }
    }

    /// A command buffer that scales `world` into `output`, to submit after
    /// the commands that draw `world` and before those that read `output`,
    /// in one `queue.submit`.
    ///
    /// It is recorded on an encoder of its own, used only through wgpu's
    /// raw (`as_hal_mut`) API: wgpu panics if one encoder mixes the two.
    /// Metal orders the three by its own hazard tracking, as wgpu-hal
    /// makes every texture with tracked hazards.
    ///
    /// # Panics
    ///
    /// If `device` is not the one the scaler was made on, or `world` and
    /// `output` are not RGBA8 textures of the input and output sizes.
    #[must_use]
    pub fn encode(
        &self,
        device: &wgpu::Device,
        world: &wgpu::Texture,
        output: &wgpu::Texture,
    ) -> wgpu::CommandBuffer {
        self.same_device(device);
        assert_eq!(
            (world.width(), world.height()),
            self.input,
            "the world's size"
        );
        assert_eq!(
            (output.width(), output.height()),
            self.output,
            "the output's size"
        );
        assert_eq!(world.format(), FORMAT, "the world's format");
        assert_eq!(output.format(), FORMAT, "the output's format");
        let world = raw_texture(world);
        assert!(
            world.usage().contains(self.color_usage()),
            "the world's Metal usage covers MetalFX's"
        );
        let output = raw_texture(output);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("nova MetalFX"),
        });
        // SAFETY: `CommandEncoder::as_hal_mut`'s contract: the raw command
        // buffer is neither destroyed nor ended here (wgpu ends it in
        // `finish`), and the wgpu encoder isn't touched while the closure
        // records. The textures are live (retained above, and the command
        // buffer retains what it uses), on this device and of the sizes
        // and format the scaler was made for (checked above).
        let recorded = unsafe {
            encoder.as_hal_mut::<Metal, _, _>(|hal| {
                let buffer = hal?.raw_command_buffer()?;
                self.raw.setColorTexture(Some(&world));
                self.raw.setOutputTexture(Some(&output));
                self.raw.setInputContentWidth(self.input.0 as usize);
                self.raw.setInputContentHeight(self.input.1 as usize);
                self.raw.encodeToCommandBuffer(buffer);
                self.raw.setColorTexture(None);
                self.raw.setOutputTexture(None);
                Some(())
            })
        };
        recorded.expect("an open Metal command buffer");
        encoder.finish()
    }

    /// `device`'s raw Metal device, which must be the one the scaler was
    /// made on.
    fn same_device(&self, device: &wgpu::Device) -> Retained<ProtocolObject<dyn MTLDevice>> {
        let raw = raw_device(device).expect("a device on the Metal backend");
        assert!(
            Retained::as_ptr(&raw) == Retained::as_ptr(&self.device),
            "the device the scaler was made on"
        );
        raw
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A device on the default adapter, when there is one.
    fn gpu() -> Option<(wgpu::Device, wgpu::Queue)> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface: None,
            apply_limit_buckets: false,
        }))
        .ok()?;
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()
    }

    /// A device MetalFX can scale on, or `None` (the test then skips,
    /// passing): no adapter, or one MetalFX doesn't support (a CI VM's
    /// paravirtual Metal device, say).
    fn metal_fx_gpu() -> Option<(wgpu::Device, wgpu::Queue)> {
        gpu().filter(|(device, _)| supported(device))
    }

    /// A high-contrast pattern: 4-pixel checks crossed by a diagonal.
    fn pattern((w, h): (u32, u32)) -> Vec<u8> {
        let mut bytes = Vec::new();
        for y in 0..h {
            for x in 0..w {
                let level = if ((x / 4) + (y / 4)) % 2 == 0 {
                    240
                } else {
                    16
                };
                let green = if x * h > y * w { 200 } else { 30 };
                bytes.extend_from_slice(&[level, green, 255 - level, 255]);
            }
        }
        bytes
    }

    /// A world texture as the renderer's world canvas is made (plus
    /// `COPY_DST`, to write the pattern in), holding `pixels`.
    fn world(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        (w, h): (u32, u32),
        pixels: &[u8],
    ) -> wgpu::Texture {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("test world"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            texture.as_image_copy(),
            pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(w * 4),
                rows_per_image: Some(h),
            },
            texture.size(),
        );
        texture
    }

    /// `texture`'s RGBA8 pixels, row-major, top row first.
    fn read(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture) -> Vec<u8> {
        let (w, h) = (texture.width(), texture.height());
        let row = (w * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::from(row) * u64::from(h),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(h),
                },
            },
            texture.size(),
        );
        queue.submit(Some(encoder.finish()));
        buffer.map_async(wgpu::MapMode::Read, .., |result| {
            result.expect("the read-back buffer maps");
        });
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("the GPU finishes");
        let mapped = buffer.get_mapped_range(..).expect("mapped");
        mapped
            .chunks(row as usize)
            .flat_map(|line| &line[..(w * 4) as usize])
            .copied()
            .collect()
    }

    /// `pixels`, `input` in size, enlarged to `output` by nearest
    /// neighbour.
    fn nearest(pixels: &[u8], (w, h): (u32, u32), (ow, oh): (u32, u32)) -> Vec<u8> {
        let mut out = Vec::new();
        for y in 0..oh {
            for x in 0..ow {
                let at = (((y * h / oh) * w + x * w / ow) * 4) as usize;
                out.extend_from_slice(&pixels[at..at + 4]);
            }
        }
        out
    }

    /// The pattern at `input`, scaled to `output` by MetalFX through the
    /// safe API, read back.
    fn scaled(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        input: (u32, u32),
        output: (u32, u32),
    ) -> Vec<u8> {
        let world = world(device, queue, input, &pattern(input));
        let scaler = SpatialScaler::new(device, input, output).expect("a scaler at this size");
        let out = scaler.output_texture(device);
        queue.submit(Some(scaler.encode(device, &world, &out)));
        read(device, queue, &out)
    }

    /// The contract: the output is the requested size, written (not
    /// lazily cleared), opaque, MetalFX's own (not a nearest enlargement)
    /// and the same every run.
    fn assert_scales(input: (u32, u32), output: (u32, u32)) {
        let Some((device, queue)) = metal_fx_gpu() else {
            return;
        };
        let first = scaled(&device, &queue, input, output);
        assert_eq!(first.len(), (output.0 * output.1 * 4) as usize);
        assert!(
            first.iter().any(|&byte| byte != 0),
            "the output is all zero"
        );
        assert!(
            first.chunks(4).all(|pixel| pixel[3] == 255),
            "the output is opaque"
        );
        let enlarged = nearest(&pattern(input), input, output);
        assert_ne!(first, enlarged, "MetalFX gave a nearest enlargement");
        assert_eq!(
            first,
            scaled(&device, &queue, input, output),
            "deterministic"
        );
    }

    #[test]
    fn it_scales_the_phase_6_harness_world_by_2x() {
        assert_scales((32, 24), (64, 48));
    }

    #[test]
    fn it_scales_the_phase_6_harness_world_by_1_875x() {
        assert_scales((32, 24), (60, 45));
    }

    #[test]
    fn it_scales_the_classic_world_to_1440p() {
        assert_scales((1024, 768), (1920, 1440));
    }

    #[test]
    fn it_scales_the_widened_world_to_1440p() {
        assert_scales((1365, 768), (2560, 1440));
    }

    #[test]
    fn an_apple_silicon_gpu_supports_it() {
        let Some((device, _queue)) = gpu() else {
            return;
        };
        // Every Apple silicon GPU ("Apple M1" and later) runs MetalFX; a
        // CI VM's "Apple Paravirtual device" need not, so it isn't asked.
        let info = device.adapter_info();
        if info.backend == wgpu::Backend::Metal && info.name.starts_with("Apple M") {
            assert!(supported(&device));
        }
    }

    #[test]
    fn a_zero_side_makes_no_scaler() {
        // MetalFX aborts the process making a scaler with a zero output
        // side (the phase 7 spike), so `new` must refuse before asking.
        let Some((device, _queue)) = metal_fx_gpu() else {
            return;
        };
        assert!(SpatialScaler::new(&device, (32, 24), (0, 48)).is_none());
        assert!(SpatialScaler::new(&device, (32, 24), (64, 0)).is_none());
        assert!(SpatialScaler::new(&device, (0, 24), (64, 48)).is_none());
        assert!(SpatialScaler::new(&device, (32, 0), (64, 48)).is_none());
        assert!(SpatialScaler::new(&device, (32, 24), (64, 48)).is_some());
    }

    #[test]
    fn the_world_canvas_usage_covers_the_colour_texture_usage() {
        let Some((device, _queue)) = metal_fx_gpu() else {
            return;
        };
        let scaler = SpatialScaler::new(&device, (32, 24), (64, 48)).expect("a scaler");
        // The renderer's world canvas is `RENDER_ATTACHMENT |
        // TEXTURE_BINDING`, which wgpu-hal makes `RenderTarget |
        // ShaderRead`.
        let canvas = MTLTextureUsage::RenderTarget | MTLTextureUsage::ShaderRead;
        assert!(!scaler.color_usage().is_empty());
        assert!(canvas.contains(scaler.color_usage()));
    }

    #[test]
    fn the_output_texture_has_the_output_size_and_is_copyable() {
        let Some((device, _queue)) = metal_fx_gpu() else {
            return;
        };
        let scaler = SpatialScaler::new(&device, (32, 24), (60, 45)).expect("a scaler");
        let out = scaler.output_texture(&device);
        assert_eq!((out.width(), out.height()), (60, 45));
        assert_eq!(out.format(), wgpu::TextureFormat::Rgba8Unorm);
        assert!(out.usage().contains(wgpu::TextureUsages::COPY_SRC));
        assert_eq!(scaler.input(), (32, 24));
        assert_eq!(scaler.output(), (60, 45));
    }
}
