//! The MetalFX spatial upscaler through the offscreen adapter: offered
//! only on macOS with a GPU MetalFX supports, and there a flight frame's
//! world scaled by MetalFX with the HUD drawn over it exactly as it is
//! over sharp-bilinear's.
//!
//! The frame tests compare the frame drawn with MetalFX (M) with the same
//! frame drawn with sharp-bilinear (S), the world alone drawn with each
//! (Wm, Ws), and the HUD alone drawn natively over black and over white
//! (B0, B1), as `sharp_bilinear` does: where the HUD is opaque (B0 is
//! B1) the frame is the HUD; where it draws nothing (B0 black, B1 white)
//! the frame is the world; elsewhere it is the HUD blended over the world
//! (`over`).
//!
//! Needs a GPU adapter; skips, passing, without one, or (for the frame
//! tests) without MetalFX.

use nova_render::fonts::FontFaces;
use nova_render::{Upscaler, UpscalerProbe};

use super::wgpu_offscreen::gpu_with;

#[cfg(not(target_os = "macos"))]
#[test]
fn off_macos_the_probe_never_offers_metal_fx() {
    let Some(gpu) = gpu_with(8, 8, &FontFaces::bundled()) else {
        return;
    };
    assert!(!gpu.upscalers().contains(&Upscaler::MetalFxSpatial));
}

/// A device on the default adapter, as the offscreen GPU opens one.
#[cfg(target_os = "macos")]
fn default_device() -> Option<wgpu::Device> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        force_fallback_adapter: false,
        compatible_surface: None,
        apply_limit_buckets: false,
    }))
    .ok()?;
    let (device, _queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()?;
    Some(device)
}

#[cfg(target_os = "macos")]
#[test]
fn on_macos_the_probe_offers_metal_fx_exactly_when_the_device_supports_it() {
    let (Some(gpu), Some(device)) = (gpu_with(8, 8, &FontFaces::bundled()), default_device())
    else {
        return;
    };
    assert_eq!(
        gpu.upscalers().contains(&Upscaler::MetalFxSpatial),
        nova_metalfx::supported(&device)
    );
}

#[cfg(target_os = "macos")]
mod frames {
    use nova_render::fonts::FontFaces;
    use nova_render::{Gpu, LogicalSize, PixelRect, Renderer, Upscaler, UpscalerProbe, Viewport};
    use nova_view::DrawList;

    use crate::flight_frame::{
        LOGICAL, Sprites, TOLERANCE, alone, as_world, drawn, fitted, flight, hud, hud_over_white,
        largest_difference, over, world,
    };
    use crate::wgpu_offscreen::gpu_with;

    const METAL_FX: Upscaler = Upscaler::MetalFxSpatial;
    const SHARP: Upscaler = Upscaler::SharpBilinear;

    /// Whether the offscreen GPU offers MetalFX; `false` with no adapter.
    fn metal_fx_offered() -> bool {
        gpu_with(8, 8, &FontFaces::bundled()).is_some_and(|gpu| gpu.upscalers().contains(&METAL_FX))
    }

    fn inside(onto: PixelRect, (x, y): (u32, u32)) -> bool {
        (onto.x..onto.x + onto.w).contains(&x) && (onto.y..onto.y + onto.h).contains(&y)
    }

    /// Draws the flight frame into `viewport` with MetalFX and checks it
    /// against sharp-bilinear's and the parts it is made of.
    fn assert_metal_fx_frame(viewport: &Viewport) {
        if !metal_fx_offered() {
            return;
        }
        let target = viewport.window_px();
        let (onto, _) = viewport.content().expect("an area");
        let drawn =
            |list: &DrawList, upscaler| drawn(list, viewport, upscaler).expect("an adapter");
        let m = drawn(&flight(), METAL_FX);
        let s = drawn(&flight(), SHARP);
        let wm = drawn(&as_world(world), METAL_FX);
        let ws = drawn(&as_world(world), SHARP);
        let b0 = drawn(&alone(hud), Upscaler::Native);
        let b1 = drawn(&hud_over_white(LOGICAL), Upscaler::Native);

        // (a) The target's size, opaque, the bars exactly black.
        assert_eq!(m.len(), (target.0 * target.1 * 4) as usize);
        for (at, pixel) in m.chunks(4).enumerate() {
            let xy = (at as u32 % target.0, at as u32 / target.0);
            assert_eq!(pixel[3], 255, "alpha at {xy:?}");
            if !inside(onto, xy) {
                assert_eq!(pixel, [0, 0, 0, 255], "the bar at {xy:?}");
            }
        }

        // (b) MetalFX's world is not sharp-bilinear's.
        let (most, at) = largest_difference(&wm, &ws, target.0);
        assert!(
            most > 8 * TOLERANCE,
            "MetalFX's world is sharp-bilinear's within {most} (at {at:?})"
        );

        // (c) The HUD is drawn over each world the same way.
        let hud_over_metal_fx = over(&b0, &b1, &wm);
        let hud_over_sharp = over(&b0, &b1, &ws);
        let frames = m.chunks(4).zip(s.chunks(4)).zip(wm.chunks(4));
        let huds = b0.chunks(4).zip(b1.chunks(4));
        let references = hud_over_metal_fx.chunks(4).zip(hud_over_sharp.chunks(4));
        for (at, (((m, s), wm), ((b0, b1), (blend_m, blend_s)))) in
            frames.zip(huds.zip(references)).enumerate()
        {
            let xy = (at as u32 % target.0, at as u32 / target.0);
            if b0 == b1 {
                assert_eq!((m, s), (b0, b0), "the opaque HUD at {xy:?}");
            } else if b0[..3] == [0, 0, 0] && b1[..3] == [255, 255, 255] {
                assert_eq!(m[..3], wm[..3], "the world alone at {xy:?}");
            } else {
                let near =
                    |got: &[u8], want: &[u8]| (0..3).all(|c| got[c].abs_diff(want[c]) <= TOLERANCE);
                assert!(near(m, blend_m), "{xy:?}: {m:?}, want {blend_m:?}");
                assert!(near(s, blend_s), "{xy:?}: {s:?}, want {blend_s:?}");
            }
        }
    }

    #[test]
    fn a_flight_frame_at_2x_is_scaled_by_metal_fx_under_the_same_hud() {
        assert_metal_fx_frame(&fitted(LOGICAL, (64, 48)));
    }

    #[test]
    fn a_flight_frame_at_1_875_is_scaled_by_metal_fx_under_the_same_hud() {
        assert_metal_fx_frame(&fitted(LOGICAL, (60, 45)));
    }

    #[test]
    fn one_gpu_scales_one_logical_size_then_another_as_fresh_gpus_do() {
        // A new size pair makes a new MetalFX scaler (and drops the old
        // one) on the same GPU; each frame is what a fresh GPU draws.
        if !metal_fx_offered() {
            return;
        }
        let mut gpu = gpu_with(64, 48, &FontFaces::bundled()).expect("an adapter");
        gpu.set_upscaler(METAL_FX);
        let small = LogicalSize { w: 16, h: 12 };
        let mut renderer = Renderer::new(Sprites);
        for logical in [LOGICAL, small, LOGICAL] {
            let viewport = fitted(logical, (64, 48));
            renderer.render(&flight(), &viewport, &mut gpu);
            let reused = gpu.read_pixels().expect("read back");
            let fresh = drawn(&flight(), &viewport, METAL_FX).expect("an adapter");
            let (most, at) = largest_difference(&reused, &fresh, 64);
            assert_eq!(most, 0, "{logical:?} at {at:?}");
        }
        assert_eq!(gpu.take_warnings(), Vec::<String>::new());
    }

    #[test]
    fn a_letterboxed_flight_frame_at_1_875_is_scaled_onto_its_viewport() {
        let viewport = fitted(LOGICAL, (60, 51));
        let (onto, _) = viewport.content().expect("an area");
        assert_eq!(
            onto,
            PixelRect {
                x: 0,
                y: 3,
                w: 60,
                h: 45
            }
        );
        assert_metal_fx_frame(&viewport);
    }
}
