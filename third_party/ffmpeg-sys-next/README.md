# ffmpeg-sys-next 9.0.1, as EV Nova vendors it

This is [ffmpeg-sys-next](https://crates.io/crates/ffmpeg-sys-next) 9.0.1
(WTFPL), copied from crates.io and patched into the workspace
(`[patch.crates-io]` in the root `Cargo.toml`). It is not a workspace member,
so fmt, clippy, rustdoc, the tests and cargo-mutants leave it alone. Its
`build` feature always builds FFmpeg from source; the upstream build script
can't be used as is, because it clones a moving branch of FFmpeg at build
time (impossible offline, as in the Flatpak), passes a configure line with
every decoder on, and doesn't finish an MSVC build.

## Changes against upstream

Everything else is upstream's, byte for byte.

- `build.rs`:
  - **Fetch.** Builds the release tarball `../ffmpeg-source.toml` records
    (version, URL, SHA-256) instead of `git clone`. It takes the tarball
    from `NOVA_FFMPEG_TARBALL` when that is set, else a copy cached in
    `$CARGO_HOME/nova-ffmpeg/`, downloaded with `curl` on a miss. The
    SHA-256 is checked every time, and the tarball is unpacked with `tar`
    (MSYS2's on Windows) into `OUT_DIR`. The record's version must be in the
    crate's FFmpeg series (9.0).
  - **Configure.** A fixed line replaces the feature-driven one:
    `--disable-everything`, then only the mov demuxer, the file protocol,
    the h264 parser, the h264, cinepak, svq1, svq3, qtrle, rpza and smc
    decoders, and avcodec, avformat and swscale; no programs, docs,
    network, hardware acceleration, autodetected libraries or x86 assembly
    (`--disable-x86asm`, so no nasm). No GPL component, no `--enable-debug`,
    and no `-march=native` (the build is shipped). On MSVC it adds
    `--toolchain=msvc` and `-MD`, Rust's C runtime.
  - **Windows.** configure and make run with MSYS2's `sh` and `make` (under
    `MSYS2_ROOT`, else `C:\msys64`) in the environment
    `cc::windows_registry` finds for `cl.exe`, MSVC's directories ahead of
    MSYS2's `usr/bin` on `PATH`, so no developer prompt is needed.
  - **A finished build** is marked by a stamp in `dist/` holding the
    tarball's SHA-256 and the configure line, rather than by
    `libavutil.a` (which MSVC names `avutil.lib`); a build from anything
    else is redone.
  - **Linking.** `EXTRALIBS` entries spelled `name.lib` (MSVC's) are
    linked, and so are its `-framework` entries; the fixed list of Apple
    frameworks is gone (it linked QTKit, OpenCL and others nothing here
    uses, and the linker warned that QTKit has no arm64 slice).
  - **Native builds only**: a build whose target isn't the host fails, so
    the cross-compiling and sysroot code is gone.
  - **No installed FFmpeg**: the `FFMPEG_DIR`, vcpkg and pkg-config
    branches are gone, and building without the `build` feature fails.
- `Cargo.toml`: the `pkg-config` and `vcpkg` build-dependencies are gone,
  and `sha2` is added.
- The crate's `.github/` workflows and the registry's `.cargo-ok` marker
  are left out, and this section is added.

---

[![ffmpeg-sys-next on crates.io](https://img.shields.io/crates/v/ffmpeg-sys-next?cacheSeconds=3600)](https://crates.io/crates/ffmpeg-sys-next)
[![build](https://github.com/zmwangx/rust-ffmpeg-sys/actions/workflows/build.yml/badge.svg?branch=master)](https://github.com/zmwangx/rust-ffmpeg-sys/actions)

This is a fork of the abandoned [ffmpeg-sys](https://github.com/meh/rust-ffmpeg-sys) crate. You can find this crate as [ffmpeg-sys-next](https://crates.io/crates/ffmpeg-sys-next) on crates.io.

This crate contains low level bindings to FFmpeg. You're probably interested in the high level bindings instead: [ffmpeg-next](https://github.com/zmwangx/rust-ffmpeg).

A word on versioning: major and minor versions track major and minor versions of FFmpeg, e.g. 4.2.x of this crate has been updated to support the 4.2.x series of FFmpeg. Patch level is reserved for bug fixes of this crate and does not track FFmpeg patch versions.

## Feature flags

In addition to feature flags declared in `Cargo.toml`, this crate performs various compile-time version and feature detections and exposes the results in additional flags. These flags are briefly documented below; run `cargo build -vv` to view more details.

- `ffmpeg_<x>_<y>` flags (new in v4.3.2), e.g. `ffmpeg_4_4`, indicating the FFmpeg installation being compiled against is at least version `<x>.<y>`. Currently available:

  - `ffmpeg_3_0`
  - `ffmpeg_3_1`
  - `ffmpeg_3_2`
  - `ffmpeg_3_3`
  - `ffmpeg_3_4`
  - `ffmpeg_4_0`
  - `ffmpeg_4_1`
  - `ffmpeg_4_2`
  - `ffmpeg_4_3`
  - `ffmpeg_4_4`
  - `ffmpeg_5_0`
  - `ffmpeg_5_1`
  - `ffmpeg_6_0`
  - `ffmpeg_6_1`
  - `ffmpeg_7_0`
  - `ffmpeg_7_1`
  - `ffmpeg_8_0`
  - `ffmpeg_8_1`
  - `ffmpeg_9_0`

- `avcodec_version_greater_than_<x>_<y>` (new in v4.3.2), e.g., `avcodec_version_greater_than_58_90`. The name should be self-explanatory.

- `ff_api_<feature>`, e.g. `ff_api_vaapi`, corresponding to whether their respective uppercase deprecation guards evaluate to true.

- `ff_api_<feature>_is_defined`, e.g. `ff_api_vappi_is_defined`, similar to above except these are enabled as long as the corresponding deprecation guards are defined.
