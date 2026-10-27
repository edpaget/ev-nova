//! Embeds `nova.exe`'s Windows resources: its icon (made from the macOS
//! `.icns`), its version information and its application manifest
//! (`packaging/windows/nova.exe.manifest`). For any other target it does
//! nothing. Every decision is `nova_winres`'s; this only reads and writes
//! the files and runs the resource compiler.
//!
//! On a Windows host the resources must compile, so a Windows build
//! always carries them. Cross-checking from another host (`mise run
//! lint-windows`) usually finds no resource compiler, which is fine: a
//! check links nothing.

use std::path::{Path, PathBuf};
use std::{env, fs};

use nova_winres::{Resources, ico_from_icns, resource_script};

/// The product's name, as `[package.metadata.bundle] name` gives it to
/// the macOS bundle.
const PRODUCT_NAME: &str = "EV Nova";

fn main() {
    let crate_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let packaging = crate_dir.join("..").join("..").join("packaging");
    let icns = packaging.join("macos").join("Nova.icns");
    let manifest = packaging.join("windows").join("nova.exe.manifest");
    for input in [&icns, &manifest] {
        println!("cargo:rerun-if-changed={}", input.display());
    }
    // The target's, not `cfg!(windows)`: the script runs on the host.
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    let ico = out.join("nova.ico");
    let icns_bytes = fs::read(&icns).unwrap_or_else(|error| panic!("{}: {error}", icns.display()));
    let icon =
        ico_from_icns(&icns_bytes).unwrap_or_else(|error| panic!("{}: {error}", icns.display()));
    write(&ico, &icon);
    let script = resource_script(&Resources {
        icon: &ico,
        manifest: &manifest,
        product_name: PRODUCT_NAME,
        description: env!("CARGO_PKG_DESCRIPTION"),
        version: env!("CARGO_PKG_VERSION"),
        original_filename: "nova.exe",
        copyright: concat!("Licensed under ", env!("CARGO_PKG_LICENSE")),
    })
    .unwrap_or_else(|error| panic!("the package version: {error}"));
    let rc = out.join("nova.rc");
    write(&rc, script.as_bytes());
    let compiled = embed_resource::compile_for(&rc, ["nova"], embed_resource::NONE);
    let compiled = if cfg!(windows) {
        compiled.manifest_required()
    } else {
        compiled.manifest_optional()
    };
    compiled.unwrap_or_else(|error| panic!("{}: {error}", rc.display()));
    // Only the manifest above: MSVC's linker makes none of its own, so
    // there are never two RT_MANIFEST resources to collide.
    if env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!("cargo:rustc-link-arg-bin=nova=/MANIFEST:NO");
    }
}

fn write(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
}
