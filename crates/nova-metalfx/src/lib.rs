//! MetalFX's spatial scaler, driven inside wgpu's Metal backend, behind a
//! safe API in wgpu's types: the macOS world upscaler's FFI.
//!
//! The workspace forbids unsafe code; this crate is the one place it
//! lives (its FFI module allows it, every block with a `SAFETY` comment
//! citing the wgpu `as_hal` contract it meets), so every other crate,
//! `nova-render` included, keeps the ban. On any target but macOS the
//! crate is empty, and none of its dependencies are built.
//!
//! - [`supported`]: whether a wgpu device is on Metal and MetalFX runs on
//!   its GPU.
//! - [`SpatialScaler`]: a scaler for one input and one output size, the
//!   texture it writes ([`SpatialScaler::output_texture`]) and a command
//!   buffer that scales into it ([`SpatialScaler::encode`]).
//!
//! What the scaler is made with, found by phase 7's spike on Apple
//! silicon:
//!
//! - **Formats:** RGBA8 unorm in and out (`MTLPixelFormatRGBA8Unorm`,
//!   wgpu's `Rgba8Unorm`). The renderer's world canvas holds sRGB-encoded
//!   bytes without conversion, so the colour mode is `Perceptual`.
//! - **Usages:** MetalFX asks `ShaderRead` of the world texture (which the
//!   world canvas's `RenderTarget | ShaderRead` covers) and
//!   `ShaderRead | RenderTarget` of the output.
//! - **Storage:** the output must be private: MetalFX asserts it under
//!   Metal's validation layer. The output is made on the raw device and
//!   wrapped for wgpu, which then takes it as initialised and never clears
//!   it over MetalFX's write. So the crate clears it to opaque black itself,
//!   once, before handing it out ([`SpatialScaler::output_texture`]): no
//!   safe caller can read the memory it was made in, whether or not it
//!   encodes first.
//! - **Alpha:** the output is opaque (255) wherever the input is.
//! - **Sizes:** any size from 1x1 up, enlarging, shrinking or neither. A
//!   zero side aborts the process inside MetalFX (it is not an
//!   Objective-C exception, so nothing can catch it), so
//!   [`SpatialScaler::new`] refuses one before asking.
//!
//! MetalFX first shipped in macOS 13, and this crate links it strongly, so
//! the build's minimum is macOS 13 (`.cargo/config.toml`). [`supported`]
//! still asks the GPU: a macOS 13 machine can have one MetalFX doesn't
//! support, such as a VM's paravirtual Metal device.

// Off macOS the crate is empty, so the links above have nothing to name.
#![cfg_attr(not(target_os = "macos"), allow(rustdoc::broken_intra_doc_links))]
#![cfg(target_os = "macos")]

mod scaler;

pub use scaler::{SpatialScaler, supported};
