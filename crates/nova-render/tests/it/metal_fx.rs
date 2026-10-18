//! The MetalFX spatial upscaler through the offscreen adapter: offered
//! only on macOS with a GPU MetalFX supports.
//!
//! Needs a GPU adapter; skips, passing, without one.

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
