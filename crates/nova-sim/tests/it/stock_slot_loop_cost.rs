//! How long a pass of `Session::game_work` takes over the stock data under
//! each reading of `RuleKey::LaunchFrameAi`: the per-slot loop
//! (`RuleSource::Engine`, `_AIDispatch`'s loop @0x44270-0x442ae, see
//! `Session::dispatch_npcs`) and the one batch (`RuleSource::Bible`).
//!
//! Two crowds are timed: the worst stock crowd, the (system, seed) whose
//! populate gives the most NPCs over every stock system with traffic and
//! 16 seeds, and a 54-NPC crowd in that system, its `AvgShips` raised by a
//! `TrafficCatalog` wrapper until the populate gives 54 (the setup holds at
//! most 55: 64 slots less the spawn reserve of 8, less the player's).
//!
//! The tests here are `#[ignore]`d, so CI never runs them; they only read
//! the clock and print, asserting the flow but never a time. Run them by
//! hand in release, in a target directory of its own outside the worktree,
//! with no other build running. The scan and timings take about 40 seconds
//! on a quiet machine, and a loaded one can push them past nextest's
//! 120-second terminate (`.config/nextest.toml`), so run them through
//! `cargo test`:
//!
//! ```text
//! NOVA_DATA=".../Nova Files" CARGO_PROFILE_RELEASE_DEBUG=line-tables-only CARGO_TARGET_DIR=<a directory outside the worktree> cargo test --release -j 6 -p nova-sim --test it -- --ignored --nocapture --test-threads 1 stock_slot_loop_cost::
//! ```
//!
//! It skips, passing, without `NOVA_DATA`. To profile one crowd, run the
//! built `it` binary under a profiler with `--ignored --exact` and one of
//! the `the_54_npc_crowd_*` tests, which time only that crowd and reading.
//!
//! # What it prints
//!
//! - the worst stock crowd's system, seed and NPC count, and the
//!   `AvgShips` that gives the 54-NPC crowd;
//! - for each crowd, reading and seed: the NPC count's min, max and mean
//!   over the timed passes, and the pass's mean, p50, p99 and max in µs;
//! - for each crowd, the loop's added cost (the per-slot mean less the
//!   batch mean, over both seeds) and its share of a 16.7 ms frame.

use std::time::{Duration, Instant};

use nova_data::GameData;
use nova_sim::ai::NovaAi;
use nova_sim::catalog::SystemReinforcements;
use nova_sim::{
    Chance, Controls, DudeId, DudeRecord, FleetRecord, PersonRecord, Pilot, PilotCatalog, RuleKey,
    RuleSource, Rulebook, Rules, Session, SystemId, SystemTraffic, TrafficCatalog,
};

use crate::common;

/// The passes worked before the timing starts.
const WARM_UP: usize = 60;
/// The passes timed.
const TIMED: usize = 1_000;
/// The seeds the scan for the worst stock crowd tries in each system.
const SCAN_SEEDS: u64 = 16;
/// The crowd the second timing is of.
const CROWD: usize = 54;
/// A 60 Hz display frame, in µs.
const FRAME_US: f64 = 16_700.0;

/// A seeded source of draws: a 64-bit linear congruential generator.
struct Seeded(u64);

impl Chance for Seeded {
    fn fires(&mut self, percent: u8) -> bool {
        self.below(100) < u32::from(percent)
    }

    fn below(&mut self, n: u32) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        u32::try_from((self.0 >> 33) % u64::from(n)).unwrap_or(0)
    }
}

/// The stock traffic, one system's `AvgShips` raised to `avg_ships`.
struct Crowded<'a> {
    data: &'a GameData,
    system: SystemId,
    avg_ships: i16,
}

impl TrafficCatalog for Crowded<'_> {
    fn system_traffic(&self, id: SystemId) -> Option<SystemTraffic> {
        let traffic = self.data.system_traffic(id)?;
        Some(if id == self.system {
            SystemTraffic {
                avg_ships: self.avg_ships,
                ..traffic
            }
        } else {
            traffic
        })
    }

    fn system_reinforcements(&self, id: SystemId) -> Option<SystemReinforcements> {
        self.data.system_reinforcements(id)
    }

    fn dude(&self, id: DudeId) -> Option<DudeRecord> {
        self.data.dude(id)
    }

    fn fleets(&self) -> Vec<FleetRecord> {
        self.data.fleets()
    }

    fn persons(&self) -> Vec<PersonRecord> {
        self.data.persons()
    }
}

/// A fresh pilot flying in `system`, off any stellar, under `source`'s
/// reading of the per-slot loop, one pass worked.
fn flying(data: &GameData, system: SystemId, source: RuleSource) -> Session {
    let mut pilot = Pilot::new(data, "").expect("the stock first chär starts");
    pilot.system = system;
    pilot.stellar = None;
    let mut session = Session::fly(data, pilot)
        .expect("flies")
        .with_rules(Rulebook::default().with_override(RuleKey::LaunchFrameAi, source));
    work(&mut session, data, &mut Seeded(0));
    session
}

/// One pass of the game's work, Nova's AI deciding, the events drained.
fn work(session: &mut Session, data: &GameData, chance: &mut Seeded) {
    let rescued = session.game_work(
        Controls::default(),
        data,
        &NovaAi::default(),
        Rules::default(),
        chance,
    );
    assert!(!rescued, "no rescue ends the pass");
    let _ = common::drained(session);
}

/// How many NPCs `catalog`'s populate on `seed` puts in `flown`'s system,
/// on a copy of `flown`.
fn populated(flown: &Session, catalog: &(impl TrafficCatalog + ?Sized), seed: u64) -> usize {
    let mut session = flown.clone();
    session.populate(catalog, &mut Seeded(seed));
    session.npcs().len()
}

/// The (system, seed) of the most NPCs at populate, with the count.
fn worst_stock_crowd(data: &GameData) -> (SystemId, u64, usize) {
    let mut worst = (SystemId(128), 0, 0);
    for id in 128..2176 {
        let system = SystemId(id);
        if !data.system_exists(system) || data.system_traffic(system).is_none() {
            continue;
        }
        let flown = flying(data, system, RuleSource::Engine);
        for seed in 0..SCAN_SEEDS {
            let count = populated(&flown, data, seed);
            if count > worst.2 {
                worst = (system, seed, count);
            }
        }
    }
    worst
}

/// The `AvgShips` from 54 up whose populate on `seed` gives exactly
/// [`CROWD`] NPCs, or else the first that gives more, with its count.
fn crowd_of_54(data: &GameData, system: SystemId, seed: u64) -> (i16, usize) {
    let mut first_over = None;
    let flown = flying(data, system, RuleSource::Engine);
    for avg_ships in 54..=200 {
        let catalog = Crowded {
            data,
            system,
            avg_ships,
        };
        let count = populated(&flown, &catalog, seed);
        if count == CROWD {
            return (avg_ships, count);
        }
        if count > CROWD && first_over.is_none() {
            first_over = Some((avg_ships, count));
        }
    }
    first_over.expect("some AvgShips fills the system to 54 or more")
}

/// One timing: the NPC counts and the passes' times.
struct Timing {
    npcs: Vec<usize>,
    passes: Vec<Duration>,
}

/// Times `passes` passes in `system` under `source`, the crowd populated
/// from `catalog` on `seed`, after [`WARM_UP`] passes.
fn timed(
    data: &GameData,
    (system, catalog): (SystemId, &(impl TrafficCatalog + ?Sized)),
    seed: u64,
    source: RuleSource,
    passes: usize,
) -> Timing {
    let mut session = flying(data, system, source);
    let mut chance = Seeded(seed);
    session.populate(catalog, &mut chance);
    for _ in 0..WARM_UP {
        work(&mut session, data, &mut chance);
    }
    let mut timing = Timing {
        npcs: Vec::with_capacity(passes),
        passes: Vec::with_capacity(passes),
    };
    for _ in 0..passes {
        timing.npcs.push(session.npcs().len());
        let start = Instant::now();
        let rescued = session.game_work(
            Controls::default(),
            data,
            &NovaAi::default(),
            Rules::default(),
            &mut chance,
        );
        timing.passes.push(start.elapsed());
        assert!(!rescued, "no rescue ends the pass");
        let _ = common::drained(&mut session);
    }
    timing
}

/// `us` as µs, to one decimal.
fn micros(took: Duration) -> f64 {
    took.as_secs_f64() * 1e6
}

/// Prints one timing's line, and gives its mean in µs.
fn report(crowd: &str, source: RuleSource, seed: u64, timing: &Timing) -> f64 {
    let mut sorted = timing.passes.clone();
    sorted.sort();
    let mean = sorted.iter().map(|took| micros(*took)).sum::<f64>() / sorted.len() as f64;
    let at = |share: f64| micros(sorted[((sorted.len() - 1) as f64 * share) as usize]);
    let (min, max) = (
        timing.npcs.iter().min().copied().unwrap_or(0),
        timing.npcs.iter().max().copied().unwrap_or(0),
    );
    let npcs = timing.npcs.iter().sum::<usize>() as f64 / timing.npcs.len() as f64;
    println!(
        "{crowd} {source:?} seed {seed}: npcs {min}-{max} mean {npcs:.1}; pass mean {mean:.1} p50 {:.1} p99 {:.1} max {:.1} µs",
        at(0.5),
        at(0.99),
        micros(sorted[sorted.len() - 1]),
    );
    mean
}

/// Times one crowd under both readings and two seeds, and prints the
/// loop's added cost.
fn time_crowd(
    data: &GameData,
    crowd: &str,
    system: SystemId,
    catalog: &(impl TrafficCatalog + ?Sized),
    seeds: [u64; 2],
) {
    let mut means = [0.0; 2];
    for (at, source) in [RuleSource::Engine, RuleSource::Bible]
        .into_iter()
        .enumerate()
    {
        for seed in seeds {
            let timing = timed(data, (system, catalog), seed, source, TIMED);
            assert_eq!(timing.passes.len(), TIMED, "every pass ran");
            means[at] += report(crowd, source, seed, &timing) / seeds.len() as f64;
        }
    }
    let added = means[0] - means[1];
    println!(
        "{crowd}: the per-slot loop adds {added:.1} µs a pass, {:.1}% of a 16.7 ms frame",
        added / FRAME_US * 100.0
    );
}

#[test]
#[ignore = "a timing to run by hand in release; see the module docs"]
fn the_per_slot_loops_cost_at_the_worst_stock_crowd_and_at_54_npcs() {
    let Some(data) = common::opened() else {
        return;
    };
    let (system, seed, count) = worst_stock_crowd(&data);
    println!(
        "worst stock crowd: system {} seed {seed}: {count} NPCs",
        system.0
    );
    assert!(count > 0, "some stock system has traffic");
    let (avg_ships, crowd) = crowd_of_54(&data, system, seed);
    println!(
        "54-NPC crowd: system {} seed {seed} AvgShips {avg_ships}: {crowd} NPCs",
        system.0
    );
    assert!(crowd >= CROWD, "the crowd reached 54");

    time_crowd(&data, "stock", system, &data, [seed, seed + 1]);
    let catalog = Crowded {
        data: &data,
        system,
        avg_ships,
    };
    time_crowd(&data, "54", system, &catalog, [seed, seed + 1]);
}

/// The 54-NPC crowd the scan above finds in the stock data (2026-11-01):
/// system 312 on seed 0, its `AvgShips` raised to 64.
const PROFILED: (SystemId, u64, i16) = (SystemId(312), 0, 64);
/// The passes a profile's run times.
const PROFILED_PASSES: usize = 3_000;

/// Works [`PROFILED_PASSES`] passes of the [`PROFILED`] crowd under
/// `source`, for a profiler to sample, and prints their timing.
fn profile(source: RuleSource) {
    let Some(data) = common::opened() else {
        return;
    };
    let (system, seed, avg_ships) = PROFILED;
    let catalog = Crowded {
        data: &data,
        system,
        avg_ships,
    };
    let timing = timed(&data, (system, &catalog), seed, source, PROFILED_PASSES);
    assert_eq!(timing.passes.len(), PROFILED_PASSES, "every pass ran");
    let _ = report("54", source, seed, &timing);
}

#[test]
#[ignore = "a run for a profiler, by hand in release; see the module docs"]
fn the_54_npc_crowd_under_the_per_slot_loop() {
    profile(RuleSource::Engine);
}

#[test]
#[ignore = "a run for a profiler, by hand in release; see the module docs"]
fn the_54_npc_crowd_in_one_batch() {
    profile(RuleSource::Bible);
}
