//! How long the new game's 3D start-up takes
//! (rdm:phase/rendering-3d/phase-11-new-game-world-load-time): the loader
//! (`nova_data::new_game::load`), the world builder ([`world_assets`]),
//! and the first and second frames that draw one system's planets.
//!
//! The one test here is `#[ignore]`d, so CI never runs it; it only reads
//! the clock and prints, and asserts nothing about time. Run it by hand,
//! in both builds:
//!
//! ```text
//! cargo nextest run -p nova-render --release --test it --run-ignored only --no-capture load_cost::
//! cargo nextest run -p nova-render --test it --run-ignored only --no-capture load_cost::
//! ```
//!
//! It times three data sets:
//!
//! - the 3D sample (`nova_data::new_game::fixture::modelled_sample_3d`):
//!   three systems, one generated planet;
//! - the scaled galaxy ([`scaled`]): the 3D sample with [`ADDED_SYSTEMS`]
//!   more systems of two generated planets each, the seven types by
//!   turns, each with clouds, rings and an atmosphere;
//! - the folder `GAME_DATA` names, the full sample, skipping without it.
//!
//! The frames are drawn on the `recording` Gpu, so no GPU adapter is
//! needed: a first frame's time is what the renderer does on the CPU
//! before it submits (a planet's mips, and its bake when it is added
//! unbaked), not the GPU's upload. It is hermetic but for `GAME_DATA`:
//! in-memory sources, with no other files or environment.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Instant;

use nova_data::new_game::fixture::modelled_sample_3d;
use nova_data::new_game::{NewGame, load};
use nova_data::wire::id::StellarId;
use nova_galaxy_format::source::{DirSource, MemSource, Source};
use nova_render::recording::{NoImages, RecordingGpu};
use nova_render::{LOGICAL, LoadedPlanet, Renderer, Viewport, WorldAssets, world_assets};
use nova_view::flight::StellarModel;
use nova_view::{DrawList, PlanetKey, PlanetPose, Point};

use super::common::game_data;

/// The systems [`scaled`] adds to the 3D sample, two planets each.
const ADDED_SYSTEMS: u32 = 49;

/// The most runs of one data set; each number reported is the least of
/// its runs.
const RUNS: u32 = 5;

/// A data set stops running again once its runs have taken this long, so
/// a slow one (the scaled galaxy in a debug build) runs once.
const ENOUGH_MS: f64 = 10_000.0;

/// The seven planet types, as a look's `type` names them.
const TYPES: [&str; 7] = [
    "rocky",
    "desert",
    "ocean",
    "garden",
    "ice",
    "lava",
    "gas-giant",
];

/// The 3D sample with [`ADDED_SYSTEMS`] more systems, each of two
/// generated planets: the seven types by turns, each with clouds, rings
/// and an atmosphere. It has no ID table, so no record is unpinned.
fn scaled() -> MemSource {
    let mut source = modelled_sample_3d().without("nova-ids.json");
    for system in 0..ADDED_SYSTEMS {
        source = source.with(
            &format!("systems/added-{system}.md"),
            &format!(
                "---\nname: Added {system}\nregion: commonwealth-rim\n\
                 map: [{}, {}]\n---\n",
                1000 + system * 20,
                1000 + (system % 7) * 20
            ),
        );
        for planet in 0..2 {
            let n = system * 2 + planet;
            let kind = TYPES[n as usize % TYPES.len()];
            source = source.with(
                &format!("stellars/added-{system}-{planet}.md"),
                &format!(
                    "---\nname: Added {system} {planet}\nkind: planet\nsystem: added-{system}\n\
                     position: [{}, 0]\nvisual:\n  generated:\n    type: {kind}\n\
                     \x20   palette: [\"#c9a36b\", \"#8a5a3c\", \"#e8d8b0\"]\n    clouds: 0.6\n\
                     \x20   rings: {{inner: 1.4, outer: 2.3, color: \"#d8c8a8\"}}\n\
                     \x20   atmosphere: {{color: \"#f0e0c0\", thickness: 0.04}}\n\
                     \x20   seed: {}\n---\n",
                    planet * 600,
                    4417 + n
                ),
            );
        }
    }
    source
}

/// The milliseconds since `start`.
fn ms(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

/// The megabytes `planet` holds on the CPU as the builder hands it over:
/// its surface, its clouds and its ring profile, or nothing to speak of
/// for a look not baked yet.
fn megabytes(planet: &LoadedPlanet) -> f64 {
    let Some(surface) = planet.baked() else {
        return 0.0;
    };
    let bytes = surface.surface.rgba.len()
        + surface.clouds.as_ref().map_or(0, |c| c.rgba.len())
        + surface.rings.as_ref().map_or(0, |r| r.profile.len());
    bytes as f64 / (1024.0 * 1024.0)
}

/// The planet keys of the system of `game` that draws the most planets
/// (the first by id among equals), with its id; none when no system draws
/// one.
fn busiest_system(game: &NewGame, assets: &WorldAssets) -> Option<(String, Vec<PlanetKey>)> {
    let mut systems: BTreeMap<String, Vec<PlanetKey>> = BTreeMap::new();
    for (id, &nova) in &game.ids.stellars {
        if let Some(StellarModel::Planet { key, .. }) = assets.models.stellar(StellarId(nova)) {
            let system = game.galaxy.stellars[id].data.system.to_string();
            systems.entry(system).or_default().push(key);
        }
    }
    let most = systems.values().map(Vec::len).max()?;
    systems.into_iter().find(|(_, keys)| keys.len() == most)
}

/// One run's numbers.
struct Run {
    load_ms: f64,
    world_ms: f64,
    /// The planets handed over holding their surface, and those unbaked.
    planets: (usize, usize),
    planet_mb: f64,
    system: Option<(String, usize)>,
    first_frame_ms: f64,
    second_frame_ms: f64,
    problems: usize,
}

/// Loads and builds `source` once, then draws its busiest system's
/// planets twice.
fn run(source: &impl Source, dir: &Path) -> Run {
    let start = Instant::now();
    let game = load(source, dir);
    let load_ms = ms(start);
    let start = Instant::now();
    let assets = world_assets(&game, source);
    let world_ms = ms(start);

    let baked = assets
        .planets
        .iter()
        .filter(|(_, planet)| planet.baked().is_some())
        .count();
    let planets = (baked, assets.planets.len() - baked);
    let planet_mb = assets.planets.iter().map(|(_, p)| megabytes(p)).sum();
    let problems = game.problems.len() + assets.problems.len();
    let system = busiest_system(&game, &assets);
    let mut list = DrawList::new();
    for (n, key) in system.iter().flat_map(|(_, keys)| keys).enumerate() {
        list.planet(
            *key,
            PlanetPose {
                center: Point::new(100.0 + 40.0 * n as f32, 200.0),
                radius: 30.0,
                time: 0.0,
            },
        );
    }
    let mut renderer = Renderer::new(NoImages);
    for (key, mesh) in assets.meshes {
        renderer.add_model(key, mesh);
    }
    for (key, planet) in assets.planets {
        renderer.add_planet(key, planet);
    }
    let viewport = Viewport::new(LOGICAL, (LOGICAL.w * 2, LOGICAL.h * 2), 2.0);
    let mut gpu = RecordingGpu::new();
    let start = Instant::now();
    renderer.render(&list, &viewport, &mut gpu);
    let first_frame_ms = ms(start);
    let start = Instant::now();
    renderer.render(&list, &viewport, &mut gpu);
    let second_frame_ms = ms(start);
    Run {
        load_ms,
        world_ms,
        planets,
        planet_mb,
        system: system.map(|(id, keys)| (id, keys.len())),
        first_frame_ms,
        second_frame_ms,
        problems,
    }
}

/// Runs `source` up to [`RUNS`] times and prints the least of each
/// number, under `name`.
fn report(name: &str, source: &impl Source, dir: &Path) {
    let started = Instant::now();
    let mut runs = Vec::new();
    while runs.len() < RUNS as usize && (runs.is_empty() || ms(started) < ENOUGH_MS) {
        runs.push(run(source, dir));
    }
    let least = |of: fn(&Run) -> f64| runs.iter().map(of).fold(f64::INFINITY, f64::min);
    let shown = runs.last().expect("a run");
    let build = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    println!("{name} ({build}, least of {} runs):", runs.len());
    println!("  new_game::load  {:10.2} ms", least(|r| r.load_ms));
    println!("  world_assets    {:10.2} ms", least(|r| r.world_ms));
    println!(
        "  planets         {} baked or read holding {:.1} MB, {} unbaked",
        shown.planets.0, shown.planet_mb, shown.planets.1
    );
    match &shown.system {
        Some((system, drawn)) => {
            println!(
                "  first frame     {:10.2} ms ({drawn} planets of `{system}`)",
                least(|r| r.first_frame_ms)
            );
            println!("  second frame    {:10.2} ms", least(|r| r.second_frame_ms));
        }
        None => println!("  no system draws a planet"),
    }
    println!("  problems        {}", shown.problems);
}

#[test]
#[ignore = "reads the clock; run by hand (see the module doc)"]
fn the_new_game_s_start_up_costs() {
    report(
        "3D sample: 3 systems, 1 generated planet",
        &modelled_sample_3d(),
        Path::new("sample"),
    );
    report(
        &format!(
            "scaled galaxy: {} systems, {} generated planets",
            3 + ADDED_SYSTEMS,
            1 + 2 * ADDED_SYSTEMS
        ),
        &scaled(),
        Path::new("scaled"),
    );
    if let Some(dir) = game_data() {
        report("GAME_DATA folder", &DirSource::new(&dir), &dir);
    }
}
