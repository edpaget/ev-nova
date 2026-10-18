//! The MetalFX spatial upscaler (macOS only, and MetalFX needs macOS 13,
//! the build's minimum).
//!
//! The world is drawn into the world canvas at the logical size (RGBA8
//! unorm; its `RENDER_ATTACHMENT | TEXTURE_BINDING` is Metal's
//! `RenderTarget | ShaderRead`, which covers the `ShaderRead` MetalFX asks
//! of its input). MetalFX scales it, colour mode perceptual, into an RGBA8
//! output texture of the viewport's size: private storage, Metal usage
//! `ShaderRead | RenderTarget`, made on the raw device and wrapped so wgpu
//! never clears it over MetalFX's write, and cleared to black once when it
//! is made (`nova_metalfx` says why). The
//! output is opaque, so a plain copy lands it on the scene at the
//! viewport, before the overlay is drawn over it.
//!
//! A frame is then three command buffers, submitted at once in this order:
//! the world's passes; MetalFX's scale, on an encoder used only through
//! wgpu's raw (`as_hal_mut`) API, since wgpu panics if one encoder mixes
//! the two; and the copy, the overlay and the blit. Metal orders the
//! three by its own hazard tracking.
//!
//! A scaler is fixed to one size pair, so one is kept for the pair last
//! scaled and remade when the pair changes. A pair the core rejects
//! ([`metal_fx_takes`]: a zero side, which aborts inside MetalFX) or
//! MetalFX won't make a scaler for is drawn with sharp-bilinear instead,
//! and logged once ([`RejectedSizes`]).

use super::frame_commands::WgpuCommands;
use super::sharp_bilinear::SharpBilinear;
use super::upscale::Upscale;
use crate::upscale::{RejectedSizes, Upscaler, WorldResolution, metal_fx_takes, resolution_of};
use crate::viewport::PixelRect;

/// A MetalFX scaler for one world size and one output size.
pub(super) trait Scale {
    /// A texture for [`Scale::encode`] to write the scaled world into.
    fn output_texture(&self) -> wgpu::Texture;

    /// A command buffer that scales `world` into `output`.
    fn encode(&self, world: &wgpu::Texture, output: &wgpu::Texture) -> wgpu::CommandBuffer;
}

/// Makes MetalFX scalers: the port [`MetalFxSpatial`] reaches MetalFX
/// through, so a test can stand in one that rejects.
pub(super) trait Scalers {
    /// A scaler from `input` to `output` (each width, height), or `None`
    /// when MetalFX won't make one.
    fn make(&mut self, input: (u32, u32), output: (u32, u32)) -> Option<Box<dyn Scale>>;
}

/// MetalFX's own scalers, through `nova_metalfx`'s safe API, on a device
/// and its queue.
pub(super) struct MetalFxScalers {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl MetalFxScalers {
    /// Scalers on `device`, their output textures cleared through
    /// `queue`.
    pub(super) fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        Self {
            device: device.clone(),
            queue: queue.clone(),
        }
    }
}

impl Scalers for MetalFxScalers {
    fn make(&mut self, input: (u32, u32), output: (u32, u32)) -> Option<Box<dyn Scale>> {
        let scaler = nova_metalfx::SpatialScaler::new(&self.device, input, output)?;
        Some(Box::new(MetalFxScale {
            scaler,
            device: self.device.clone(),
            queue: self.queue.clone(),
        }))
    }
}

/// One of MetalFX's spatial scalers, with the device it was made on and
/// its queue.
struct MetalFxScale {
    scaler: nova_metalfx::SpatialScaler,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl Scale for MetalFxScale {
    fn output_texture(&self) -> wgpu::Texture {
        self.scaler.output_texture(&self.device, &self.queue)
    }

    fn encode(&self, world: &wgpu::Texture, output: &wgpu::Texture) -> wgpu::CommandBuffer {
        self.scaler.encode(&self.device, world, output)
    }
}

/// The MetalFX spatial upscaler: the world is drawn at the logical
/// resolution, then scaled onto the viewport by MetalFX. A size MetalFX
/// won't scale between is drawn with sharp-bilinear instead, and logged
/// once ([`RejectedSizes`]).
pub(super) struct MetalFxSpatial {
    scalers: Box<dyn Scalers>,
    sharp: SharpBilinear,
    rejected: RejectedSizes,
    /// The scaler for the size pair last scaled, and its output texture:
    /// a scaler is fixed to one pair, so a new pair makes a new one.
    current: Option<Current>,
}

/// A scaler made for one size pair, and the texture it writes.
struct Current {
    input: (u32, u32),
    output: (u32, u32),
    scale: Box<dyn Scale>,
    texture: wgpu::Texture,
}

impl MetalFxSpatial {
    /// The upscaler, its scalers made by `scalers`, falling back to a
    /// sharp-bilinear upscaler on `device`, written through `queue`.
    pub(super) fn new(
        scalers: Box<dyn Scalers>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Self {
        Self {
            scalers,
            sharp: SharpBilinear::new(device, queue),
            rejected: RejectedSizes::default(),
            current: None,
        }
    }

    /// The scaler from `input` to `output`, made if the one kept is for
    /// another pair, or `None` when the core or MetalFX rejects the pair.
    fn scaler(&mut self, input: (u32, u32), output: (u32, u32)) -> Option<&Current> {
        if !metal_fx_takes(input, output) || self.rejected.rejected(input, output) {
            return None;
        }
        let kept = self
            .current
            .as_ref()
            .is_some_and(|current| (current.input, current.output) == (input, output));
        if !kept {
            self.current = None;
            let scale = self.scalers.make(input, output)?;
            let texture = scale.output_texture();
            self.current = Some(Current {
                input,
                output,
                scale,
                texture,
            });
        }
        self.current.as_ref()
    }
}

impl Upscale for MetalFxSpatial {
    fn resolution(&self) -> WorldResolution {
        resolution_of(Upscaler::MetalFxSpatial)
    }

    fn enlarge(
        &mut self,
        commands: &mut WgpuCommands<'_>,
        world: &wgpu::TextureView,
        target: &wgpu::TextureView,
        onto: PixelRect,
        warnings: &mut Vec<String>,
    ) {
        let input = (world.texture().width(), world.texture().height());
        let output = (onto.w, onto.h);
        if let Some(current) = self.scaler(input, output) {
            // MetalFX scales in a command buffer of its own (wgpu panics if
            // an encoder mixes its API with the raw one), then a plain copy
            // lands the opaque result on the viewport.
            commands.insert(current.scale.encode(world.texture(), &current.texture));
            commands.encoder().copy_texture_to_texture(
                current.texture.as_image_copy(),
                wgpu::TexelCopyTextureInfo {
                    texture: target.texture(),
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: onto.x,
                        y: onto.y,
                        z: 0,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                current.texture.size(),
            );
            return;
        }
        warnings.extend(self.rejected.reject(input, output));
        self.sharp
            .enlarge_into(commands.encoder(), world, target, onto);
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use nova_data::graphics::Image;
    use nova_view::{Color, DrawList, ImageKind, Point};

    use super::*;
    use crate::batch::Renderer;
    use crate::fonts::FontFaces;
    use crate::gpu::Gpu;
    use crate::images::{ImageError, ImageSource};
    use crate::viewport::{LogicalSize, Viewport};
    use crate::wgpu::OffscreenGpu;
    use crate::wgpu::sharp_bilinear::SharpBilinear;

    /// No images: the frames below draw solids alone.
    struct NoImages;

    impl ImageSource for NoImages {
        fn frames(&self, _kind: ImageKind, _id: i16) -> Result<Vec<Image>, ImageError> {
            Err(ImageError::Missing)
        }
    }

    /// The size pairs a stand-in was asked to make a scaler for, shared
    /// with the test.
    type Asked = Rc<RefCell<Vec<((u32, u32), (u32, u32))>>>;

    /// Makes no scaler, whatever the size: MetalFX rejecting every pair.
    struct Rejecting(Asked);

    impl Scalers for Rejecting {
        fn make(&mut self, input: (u32, u32), output: (u32, u32)) -> Option<Box<dyn Scale>> {
            self.0.borrow_mut().push((input, output));
            None
        }
    }

    /// Makes a scaler for every pair, on `device`, and keeps each pair
    /// asked for and each encode's pair.
    struct Accepting {
        device: wgpu::Device,
        asked: Asked,
        encodes: Asked,
    }

    impl Scalers for Accepting {
        fn make(&mut self, input: (u32, u32), output: (u32, u32)) -> Option<Box<dyn Scale>> {
            self.asked.borrow_mut().push((input, output));
            Some(Box::new(Counting {
                device: self.device.clone(),
                pair: (input, output),
                encodes: self.encodes.clone(),
            }))
        }
    }

    /// Scales nothing: it keeps each encode and hands back an empty
    /// command buffer.
    struct Counting {
        device: wgpu::Device,
        pair: ((u32, u32), (u32, u32)),
        encodes: Asked,
    }

    impl Scale for Counting {
        fn output_texture(&self) -> wgpu::Texture {
            let (width, height) = self.pair.1;
            self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("stand-in output"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
        }

        fn encode(&self, world: &wgpu::Texture, output: &wgpu::Texture) -> wgpu::CommandBuffer {
            assert_eq!((world.width(), world.height()), self.pair.0);
            assert_eq!((output.width(), output.height()), self.pair.1);
            self.encodes.borrow_mut().push(self.pair);
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor::default())
                .finish()
        }
    }

    /// A GPU whose MetalFX scalers are [`Accepting`]'s, and the pairs they
    /// were made for and encoded at.
    fn accepting_gpu() -> Option<(OffscreenGpu, Asked, Asked)> {
        let (asked, encodes) = (Asked::default(), Asked::default());
        let (makes_kept, encodes_kept) = (asked.clone(), encodes.clone());
        let gpu = gpu_drawing_with(move |device, queue| {
            let scalers = Accepting {
                device: device.clone(),
                asked: makes_kept,
                encodes: encodes_kept,
            };
            Box::new(MetalFxSpatial::new(Box::new(scalers), device, queue))
        })?;
        Some((gpu, asked, encodes))
    }

    #[test]
    fn a_scaler_is_made_once_for_a_size_and_used_every_frame() {
        let Some((mut gpu, asked, encodes)) = accepting_gpu() else {
            return;
        };
        draw(&mut gpu, CLASSIC);
        draw(&mut gpu, CLASSIC);
        let pair = ((32, 24), (64, 48));
        assert_eq!(*asked.borrow(), [pair]);
        assert_eq!(*encodes.borrow(), [pair, pair]);
        assert_eq!(gpu.take_warnings(), Vec::<String>::new());
    }

    #[test]
    fn a_new_size_makes_a_new_scaler_and_only_the_current_one_is_kept() {
        let Some((mut gpu, asked, _)) = accepting_gpu() else {
            return;
        };
        draw(&mut gpu, CLASSIC);
        draw(&mut gpu, SMALL);
        draw(&mut gpu, SMALL);
        draw(&mut gpu, CLASSIC);
        let (classic, small) = (((32, 24), (64, 48)), ((16, 12), (64, 48)));
        assert_eq!(*asked.borrow(), [classic, small, classic]);
    }

    /// A 64 x 48 offscreen GPU drawing the world with `upscale`, made on
    /// its device; `None` with no adapter.
    fn gpu_drawing_with(
        upscale: impl FnOnce(&wgpu::Device, &wgpu::Queue) -> Box<dyn Upscale>,
    ) -> Option<OffscreenGpu> {
        OffscreenGpu::with_upscale(64, 48, &FontFaces::bundled(), upscale).ok()
    }

    /// A MetalFX upscaler whose scalers are `scalers`.
    fn metal_fx(
        scalers: impl Scalers + 'static,
    ) -> impl FnOnce(&wgpu::Device, &wgpu::Queue) -> Box<dyn Upscale> {
        move |device, queue| Box::new(MetalFxSpatial::new(Box::new(scalers), device, queue))
    }

    /// A world of a line and a dot, then an overlay line over it, in
    /// `logical` fitted to the 64 x 48 target.
    fn frame_list() -> DrawList {
        let mut list = DrawList::new();
        list.world(|list| {
            list.line(
                Point::new(2.0, 4.0),
                Point::new(14.0, 9.0),
                2.0,
                Color::rgba(0, 0, 255, 255),
            )
            .dot(Point::new(8.0, 8.0), 2.0, Color::rgba(255, 255, 0, 255));
        });
        list.line(
            Point::new(1.0, 1.0),
            Point::new(10.0, 2.0),
            1.0,
            Color::rgba(255, 0, 0, 128),
        );
        list
    }

    fn viewport(logical: LogicalSize) -> Viewport {
        Viewport::new(logical, (64, 48), 1.0)
    }

    const CLASSIC: LogicalSize = LogicalSize { w: 32, h: 24 };
    const SMALL: LogicalSize = LogicalSize { w: 16, h: 12 };

    /// Draws the frame list into `logical` on `gpu`, read back.
    fn draw(gpu: &mut OffscreenGpu, logical: LogicalSize) -> Vec<u8> {
        let report = Renderer::new(NoImages).render(&frame_list(), &viewport(logical), gpu);
        assert_eq!(report.new_failures, vec![]);
        gpu.read_pixels().expect("read back")
    }

    /// The frame list drawn into `logical` with sharp-bilinear by a fresh
    /// GPU.
    fn sharp_bilinear(logical: LogicalSize) -> Vec<u8> {
        let mut gpu = gpu_drawing_with(|device, queue| Box::new(SharpBilinear::new(device, queue)))
            .expect("an adapter");
        draw(&mut gpu, logical)
    }

    #[test]
    fn a_rejected_size_is_drawn_with_sharp_bilinear_and_logged_once() {
        let asked = Asked::default();
        let Some(mut gpu) = gpu_drawing_with(metal_fx(Rejecting(asked.clone()))) else {
            return;
        };
        assert_eq!(gpu.world_resolution(), crate::WorldResolution::Logical);
        assert_eq!(draw(&mut gpu, CLASSIC), sharp_bilinear(CLASSIC));
        assert_eq!(
            gpu.take_warnings(),
            [
                "nova: MetalFX can't scale the world from 32x24 to 64x48; drawing it with \
              sharp-bilinear"
            ]
        );
        assert_eq!(draw(&mut gpu, CLASSIC), sharp_bilinear(CLASSIC));
        assert_eq!(gpu.take_warnings(), Vec::<String>::new(), "once");
        assert_eq!(*asked.borrow(), [((32, 24), (64, 48))], "asked once");
    }

    #[test]
    fn a_new_rejected_size_is_logged_once_of_its_own() {
        let Some(mut gpu) = gpu_drawing_with(metal_fx(Rejecting(Asked::default()))) else {
            return;
        };
        draw(&mut gpu, CLASSIC);
        assert_eq!(gpu.take_warnings().len(), 1);
        assert_eq!(draw(&mut gpu, SMALL), sharp_bilinear(SMALL));
        assert_eq!(
            gpu.take_warnings(),
            [
                "nova: MetalFX can't scale the world from 16x12 to 64x48; drawing it with \
              sharp-bilinear"
            ]
        );
        draw(&mut gpu, CLASSIC);
        draw(&mut gpu, SMALL);
        assert_eq!(gpu.take_warnings(), Vec::<String>::new(), "back again");
    }
}
