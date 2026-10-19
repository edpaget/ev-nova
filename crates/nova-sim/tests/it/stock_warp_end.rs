//! The stock warp-up's length against the original's warp end
//! (`_HandlePlayer` @0x6bb5e-0x6c697; see `nova_sim::warp::warp_end` and the
//! session's `warp_stage` module). Skips, passing, when `NOVA_DATA` is unset.
//!
//! - `snd ` 128 lasts 364 Mac ticks, 129 (double time) 252 and the
//!   warp-out, 130, 151 (`_LoadSounds` @0x1c0b5-0x1c12d).
//! - The warp-up plays pitched by the class's warp factor w at
//!   trunc(65536 / w) (`_AdjustWarpSoundSpeed` @0x5ecc), so it lasts
//!   d × 65536 / rate. The warp ends naturally once both warp-ups are over,
//!   else a pass after t > d / w (@0x6c637-0x6c697). Of the stock factors
//!   only w 0.91 (Enterprise) reaches its natural end; 1.3 (Shuttle), 1.69
//!   (Star Liner) and 2.08 (Cargo Drone) end by the fallback with the
//!   warp-up still playing: the fallback's pass halts it and plays the
//!   warp-out (@0x6c6a8-0x6c706), and the arrival a pass later, the
//!   natural end on the flag the fallback set, halts nothing and finds the
//!   warp-out still playing (@0x6bbe1), so it raises neither again.
//! - The fallback's end comes "a pass after" by the passes `_DoPlayGameWork`
//!   runs (`_PlayGame` @0x45f20-0x45f31): in single time the next frame's,
//!   but in double time the same frame's second pass, on the same warp
//!   clock, so the double-time fallbacks end one advance pass sooner than
//!   counting frames would give (Shuttle 97, not 98).
//! - The natural end's warp-out skip (@0x6bbe1-0x6bc08) is reached with
//!   stock sounds only by the arrival after a fallback, a pass after the
//!   fallback started 130: on a warp that ends naturally, the shortest
//!   stock pitched warp-up (229) outlasts the warp-out (151), and it starts
//!   after the arrival that started 130.

use crate::common;

use nova_data::GameData;
use nova_data::records::ship::Ship;
use nova_data::records::system::System;
use nova_sim::warp::{
    pitched_length, sound_duration, warp_factor, warp_out_duration, warp_up_rate,
};
use nova_sim::{
    Controls, NeverFires, Pilot, PilotCatalog, Rules, Session, ShipId, SimSound, SystemId,
};

const SHUTTLE: ShipId = ShipId(128);
const ENTERPRISE: ShipId = ShipId(139);
const STAR_LINER: ShipId = ShipId(134);
const CARGO_DRONE: ShipId = ShipId(130);

/// A whole stock jump's warp, by ship and time: the pitched length, and
/// the advance passes from the warp-up's pass to the warp end.
struct Expected {
    ship: ShipId,
    double: bool,
    pitched: u32,
    passes: u32,
    natural: bool,
}

const EXPECTED: [Expected; 8] = [
    Expected {
        ship: SHUTTLE,
        double: false,
        pitched: 473,
        passes: 142,
        natural: false,
    },
    Expected {
        ship: SHUTTLE,
        double: true,
        pitched: 327,
        passes: 97,
        natural: false,
    },
    Expected {
        ship: ENTERPRISE,
        double: false,
        pitched: 331,
        passes: 166,
        natural: true,
    },
    Expected {
        ship: ENTERPRISE,
        double: true,
        pitched: 229,
        passes: 115,
        natural: true,
    },
    Expected {
        ship: STAR_LINER,
        double: false,
        pitched: 615,
        passes: 109,
        natural: false,
    },
    Expected {
        ship: STAR_LINER,
        double: true,
        pitched: 425,
        passes: 75,
        natural: false,
    },
    Expected {
        ship: CARGO_DRONE,
        double: false,
        pitched: 757,
        passes: 89,
        natural: false,
    },
    Expected {
        ship: CARGO_DRONE,
        double: true,
        pitched: 524,
        passes: 61,
        natural: false,
    },
];

const HALT: SimSound = SimSound::Halt(&nova_sim::sound::WARP_UP_HALTS);

/// A stock pilot flying `ship`, its reserves full.
fn flying_in(data: &GameData, ship: ShipId) -> Session {
    let pilot = Pilot::new(data, "Stock").expect("the stock first chär starts");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["ship"] = serde_json::json!(ship.0);
    save["stellar"] = serde_json::Value::Null;
    // The Shuttle's 30 armour would leave a bigger hull disabled: every
    // stock hull's full, instead.
    for reserve in ["shield", "armor"] {
        save["reserves"][reserve] = serde_json::json!({ "now": 10_000.0, "max": 10_000.0 });
    }
    let flown = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    let session = Session::fly(data, flown).expect("flies");
    assert_eq!(session.ship(), ship);
    session
}

/// The first hyperlink of `system`.
fn first_link(data: &GameData, system: SystemId) -> SystemId {
    let record = data
        .get::<System>(system.0)
        .expect("present")
        .expect("decodes")
        .record;
    record
        .con
        .into_iter()
        .flatten()
        .next()
        .expect("a hyperlink")
}

/// One advance pass, and double time's second pass while it is on.
fn pass(session: &mut Session, controls: Controls) {
    session.tick(controls);
    if session.double_time() {
        session.tick_again(controls, Rules::default(), &mut NeverFires);
    }
}

/// Plots a course to `next`, flies out past the jump distance in single
/// time, then begins the jump, in double time if `double`.
fn out_and_jump(session: &mut Session, next: SystemId, double: bool) {
    session.set_double_time_key(false);
    session.plot_course(next).expect("a route");
    let thrust = Controls {
        thrust: true,
        ..Controls::default()
    };
    let mut tries = 0;
    while session.player().position.length() < session.stats().jump_distance {
        session.tick(thrust);
        tries += 1;
        assert!(tries < 10_000, "never got out");
    }
    session.set_double_time_key(double);
    session.begin_jump().expect("jumps");
}

/// The warp of one stock jump, from its warp-up's pass to its end.
struct Warp {
    /// The warp-up's rate.
    rate: u32,
    /// The advance passes from the warp-up's to the one the warp is due
    /// to end on.
    passes: u32,
    /// The sounds each of those passes raised, the first pass's first
    /// (both halves of a double-time frame together).
    sounds: Vec<Vec<SimSound>>,
}

/// Passes `session` until its warp-up sounds, then counts the advance
/// passes until the warp is due to end, keeping the sounds of each.
fn passes_to_end(session: &mut Session, double: bool) -> Warp {
    let mut rate = None;
    for _ in 0..10_000 {
        let sounds = session.take_sounds();
        if let Some(raised) = sounds.iter().find_map(|sound| match (sound, double) {
            (SimSound::JumpBegan { rate }, false)
            | (SimSound::JumpBeganInDoubleTime { rate }, true) => Some(*rate),
            _ => None,
        }) {
            rate = Some(raised);
            break;
        }
        pass(session, Controls::default());
    }
    let rate = rate.expect("the warp-up sounds");
    let mut sounds = Vec::new();
    for passes in 1..10_000 {
        pass(session, Controls::default());
        sounds.push(session.take_sounds());
        if session.warp_end_due() {
            return Warp {
                rate,
                passes,
                sounds,
            };
        }
    }
    panic!("the warp never ended");
}

#[test]
fn the_stock_warp_up_lasts_its_pitched_length_against_the_warp_end() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let single = sound_duration(data.warp_sound());
    let double = sound_duration(data.warp_sound_double_time());
    assert_eq!((single, double), (364, 252));
    assert_eq!(warp_out_duration(data.warp_out_sound()), 151);
    for (ship, bit) in [
        (SHUTTLE, 0),
        (ENTERPRISE, 0x1),
        (STAR_LINER, 0x2),
        (CARGO_DRONE, 0x4),
    ] {
        let flags = data
            .get::<Ship>(ship.0)
            .expect("present")
            .expect("decodes")
            .record
            .flags
            .bits();
        assert_eq!(
            warp_factor(flags).to_bits(),
            warp_factor(bit).to_bits(),
            "{ship:?}"
        );
    }
    assert_eq!(warp_up_rate(warp_factor(0)), 50412);

    for expected in &EXPECTED {
        let label = format!("{:?} double {}", expected.ship, expected.double);
        let mut session = flying_in(&data, expected.ship);
        let next = first_link(&data, session.system());
        out_and_jump(&mut session, next, expected.double);
        let Warp {
            rate,
            passes,
            sounds: raised,
        } = passes_to_end(&mut session, expected.double);
        let flags = data
            .get::<Ship>(expected.ship.0)
            .expect("present")
            .expect("decodes")
            .record
            .flags
            .bits();
        assert_eq!(rate, warp_up_rate(warp_factor(flags)), "{label}");
        let d = if expected.double { double } else { single };
        assert_eq!(pitched_length(d, rate), expected.pitched, "{label}");
        assert_eq!(passes, expected.passes, "{label}");
        if expected.natural {
            assert!(passes * 2 >= expected.pitched, "{label}: over");
            assert!((passes - 1) * 2 < expected.pitched, "{label}: its last");
        } else {
            assert!(passes * 2 < expected.pitched, "{label}: still playing");
        }
        let warp_end = [HALT, SimSound::Arrived];
        // The fallback's pass: the one before the end in single time, the
        // same frame's advance half in double time.
        let fallback = if expected.double { passes } else { passes - 1 };
        for (pass, sounds) in (1..).zip(&raised) {
            if !expected.natural && pass == fallback {
                assert_eq!(sounds[..], warp_end, "{label}: the fallback's pass");
            } else {
                assert!(
                    !sounds.contains(&HALT) && !sounds.contains(&SimSound::Arrived),
                    "{label}: pass {pass}: {sounds:?}"
                );
            }
        }
        assert!(session.arrive(&data, &mut NeverFires).is_some(), "{label}");
        let sounds = session.take_sounds();
        if expected.natural {
            assert!(sounds.contains(&SimSound::Arrived), "{label}: {sounds:?}");
        } else {
            assert!(
                !sounds.contains(&HALT) && !sounds.contains(&SimSound::Arrived),
                "{label}: 130 still plays: {sounds:?}"
            );
        }
    }
}

#[test]
fn back_to_back_stock_jumps_never_skip_the_warp_out() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    for double in [false, true] {
        let mut session = flying_in(&data, ENTERPRISE);
        let start = session.system();
        out_and_jump(&mut session, first_link(&data, start), double);
        let _ = passes_to_end(&mut session, double);
        assert!(session.arrive(&data, &mut NeverFires).is_some());
        assert!(
            session.take_sounds().contains(&SimSound::Arrived),
            "{double}"
        );

        // The next jump, back where it came from, begins on the arrival's
        // tick.
        out_and_jump(&mut session, start, double);
        let mut since = 0;
        while !session.warp_end_due() {
            pass(&mut session, Controls::default());
            since += 1;
            assert!(since < 10_000, "the warp never ended");
        }
        assert!(since * 2 > 151, "{double}: {since} passes");
        let _ = session.take_sounds();
        assert!(session.arrive(&data, &mut NeverFires).is_some());
        assert!(
            session.take_sounds().contains(&SimSound::Arrived),
            "{double}"
        );
    }
}
