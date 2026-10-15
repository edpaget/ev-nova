# Upscaler comparison and decision

The record of rdm phase `display-modes/phase-4-upscaler-investigation`: which
ways of drawing the original's art bigger were tried on Nova's own sprites,
what they cost, how they looked, and what the operator chose to build.

The side-by-side comparison, with every image, is the published artifact
<https://claude.ai/artifact/FgG9cG58Nytcavaj2Natt7>. The harness and its
outputs were throwaway and are not in the repo.

## Setup

- **Machine.** Apple M5 Max (Mac17,7), macOS 27.0.1 (26A434).
- **Sprites.** Exported from the stock `Nova Files` with `nova-dump`: the
  Valkyrie (144x48), Polaris Dragon (240x80) and Leviathan (432x144) at
  several rotation frames, a planet (Earth, 150x150), an explosion
  animation strip (256x64), and interface art (the status bar 194x400, menu
  buttons 120x122, the comms dialog frame 423x215). Sprites were composited
  onto black before scaling, as they appear over space; alpha handling was
  not compared.
- **Whole frames.** A composed flight scene (black space, two planets, the
  Leviathan, Dragons, Valkyries, an explosion and a blaster bolt) at
  1024x768 and at the widened 1365x768, scaled as a whole frame the way a
  per-frame upscaler would run over the world layer.
- **Scales.** 2x, and the two real display cases, both 1.875x: a 4:3 view
  at 1024x768 → 1920x1440 and the widened flight layer at 1365x768 →
  2560x1440.
- **Integer-only scalers** (xBRZ, ScaleFX, MMPX, hqx) scaled to an integer
  factor and then went down with bilinear, the engine's linear sampler.
  ScaleFX used its libretro preset's fixed 3x, at 2x as well as 1.875x;
  xBRZ, MMPX and hqx (hq2x) used the next integer factor, 2x.

## Candidates and tools

Shader sources are libretro slang-shaders at commit `e1d7563` (8 Oct 2026),
run through librashader 0.12.1's Metal runtime from a small Rust harness.
All licences are compatible with the project's GPL-3.0-or-later.

| Candidate | Path to 1.875x | Platforms | Source and licence |
|---|---|---|---|
| Nearest | direct | any GPU | `stock.slang`, point sampling |
| Bilinear | direct | any GPU | `stock.slang`, linear sampling |
| Sharp-bilinear | direct | any GPU, one fragment shader | `sharp-bilinear-simple.slang`, public domain |
| xBR (lv2) | 3x, then B-spline resample | any GPU, 6 passes | `xbr-lv2.slangp`, Hyllian, MIT |
| Super-xBR | 2x (3 passes), then Jinc2 + deblur | any GPU, 6 passes | `super-xbr.slangp`, Hyllian, MIT |
| xBRZ | 2x, then bilinear | any GPU, or CPU at load | `2xbrz.slang` (MIT); CPU timing with `xbrz-rs` 0.1.0 (GPL-3.0) |
| ScaleFX | 3x, then bilinear | any GPU, 6 passes, two float targets | `scalefx.slangp`, Sp00kyFox, MIT |
| MMPX | 2x, then bilinear | any GPU, or CPU at load | `mmpx.slang` (MIT); CPU timing with `mmpx` 0.2.0 (MIT) |
| hqx (hq2x) | 2x, then bilinear | any GPU (needs its LUT), or CPU | libretro hqx, LGPL-2.1+ |
| FSR 1 (EASU + RCAS) | direct | any GPU | AMD FidelityFX FSR 1 via `fsr.slangp`, MIT; RCAS 0.2 stops, grain off |
| MetalFX spatial | direct; accepted 2x and 1.875x on this GPU | macOS (Apple silicon) only | `MTLFXSpatialScaler`, perceptual colour, RGBA8, own Swift harness |
| Real-ESRGAN x4plus (offline) | 4x neural, then Lanczos down | offline tool on macOS, Windows, Linux | `realesrgan-ncnn-vulkan` 20220424 (v0.2.5.0, BSD-3), model `realesrgan-x4plus`, through MoltenVK |

Every listed candidate ran; none was skipped or substituted.

## Per-frame GPU cost

Milliseconds of GPU time per frame on the whole-frame inputs, for the full
preset chain including the final resample. The 60 Hz budget is 16.7 ms.

| Candidate | 1024x768 → 1920x1440 median / p95 | 1365x768 → 2560x1440 median / p95 |
|---|---|---|
| Nearest | 0.05 / 0.20 | 0.04 / 0.19 |
| Bilinear | 0.04 / 0.18 | 0.03 / 0.17 |
| Sharp-bilinear | 0.05 / 0.19 | 0.03 / 0.18 |
| xBR (lv2) | 0.75 / 0.77 | 1.01 / 1.13 |
| Super-xBR | 0.86 / 0.94 | 1.09 / 1.17 |
| xBRZ | 0.22 / 0.24 | 0.26 / 0.29 |
| ScaleFX | 0.26 / 0.34 | 0.39 / 0.42 |
| MMPX | 0.11 / 0.25 | 0.13 / 0.14 |
| hqx (hq2x) | 0.19 / 0.26 | 0.13 / 0.26 |
| FSR 1 (EASU + RCAS) | 0.19 / 0.21 | 0.26 / 0.28 |
| MetalFX spatial | 0.14 / 0.28 | 0.16 / 0.30 |

**Method.** For each candidate and mapping: 1.5 s of back-to-back frames to
settle the GPU clocks, one discarded run, then 2,000 frames encoded as one
command buffer each and committed together. A frame's cost is the gap
between consecutive command buffers' `GPUEndTime`. Three rounds, interleaved
across candidates; the table gives the median of the round medians and of
the round p95s. Below about 0.1 ms the numbers sit at the timer's noise
floor: nearest, bilinear and sharp-bilinear are effectively free. Frames
committed and waited on one at a time gave noisier numbers (the GPU drops
its clocks between them) and are on the artifact for reference only.

## Load-time and offline cost

CPU, single thread, release build, median of 5 runs, over every stock rlëD
sheet the dump produced (282 sheets, 160 Mpx of source):

| Scaler | Factor | Seconds, all sheets |
|---|---|---|
| MMPX (`mmpx` 0.2.0) | 2x | 2.93 |
| xBRZ (`xbrz-rs` 0.1.0) | 2x | 2.34 |
| xBRZ (`xbrz-rs` 0.1.0) | 3x | 3.83 |

A GPU shader form of these scalers would do the same work at load in a
fraction of that; these CPU numbers are the upper bound.

Real-ESRGAN x4plus took 0.4–0.9 s per sprite (about 0.4 s of each is process
start and model load) and 6–8 s per whole 1024x768 or 1365x768 frame on this
GPU. At the 1365x768 frame's rate (6.0 µs per source pixel) the full 160 Mpx
of stock sheets would take roughly 16 minutes on this machine. That is an
extrapolation, not a measurement, and a weaker GPU will take longer.

## Visual findings

- **Nearest.** The baseline: stair-steps on every anti-aliased edge. At
  1.875x source pixels land as a mix of 1- and 2-pixel columns, so edges
  wobble as ships turn and drift.
- **Bilinear.** Soft and even, no artifacts, but hull plating, planet cloud
  and interface text lose crispness.
- **Sharp-bilinear.** Identical to nearest at 2x. At 1.875x it is nearest
  with a 1-pixel blend on the uneven seams, so pixel widths look even. Keeps
  the original look and adds no smoothing.
- **xBR (lv2).** Rounds diagonal stair-steps on hulls and the planet ring
  while keeping interior texture; the B-spline resample leaves it slightly
  soft.
- **Super-xBR.** The cleanest edges of the classic scalers on this art: thin
  lines and hull outlines come out crisp and texture survives. A faint halo
  on the brightest high-contrast edges. The most expensive per frame.
- **xBRZ.** Painterly: rounds edges well but merges fine texture into flat
  blobs (planet clouds, hull panels, the stone dialog border).
- **ScaleFX.** Like xBRZ with cleaner line reconstruction; still flattens
  texture on rendered art.
- **MMPX** and **hqx.** Almost indistinguishable from nearest. Both key on
  equal or near-equal colours, which anti-aliased 3D renders rarely have, so
  they find few edges to rebuild.
- **FSR 1.** Crisp, faithful geometry with a sharpened, higher-contrast look;
  hull edges look the most like a higher-resolution render among the shaders.
  RCAS amplifies fine texture, so planet cloud, explosions and stone borders
  turn grainy.
- **MetalFX spatial.** Very close to FSR 1, with a touch less grain, and the
  cheapest of the sharpening upscalers here.
- **Real-ESRGAN x4plus.** Repaints the art. Ships come out clean and sharp,
  like higher-resolution renders, with invented panel detail. Textured art
  breaks: explosions turn to speckle, stone interface borders become streaks,
  the planet ring becomes a bright hard line.

Nova's ship and planet sprites are pre-rendered 3D with anti-aliasing, so
the scalers built for flat-palette pixel art (MMPX, hqx, xBRZ, ScaleFX) do
little or harm texture; the edge-directed and sharpening ones do best.

## Decision (2026-10-15)

The operator's answers, verbatim:

- Scalers: "MetalFX spatial, Sharp-bilinear, Super-xBR" (FSR 1 was not
  chosen).
- Site: "Both".
- HD pack: "Yes, but can this ship as a first load step? we obviously can't
  prebuild and ship these assets but it's definitely the best looking"

How the roadmap reads that (the orchestrator's interpretation):

- **Per frame**, over the world layer: sharp-bilinear on every platform and
  MetalFX spatial on macOS only.
- **At sprite load**: Super-xBR, run once when sprites load, and the
  Real-ESRGAN-style HD pack.
- **HD pack**: built locally on the player's machine from their own
  `Nova Files` (and plug-ins) on first load or when enabled, cached on disk,
  never shipped prebuilt. It covers ship sprites by default, with other
  sprite kinds opt-in, because explosions and textured interface art came out
  damaged.
- Nearest at native resolution stays the default at both sites.

The phases written from this decision are phases 5 to 10 of the
`display-modes` roadmap (`rdm phase list --roadmap display-modes`).
