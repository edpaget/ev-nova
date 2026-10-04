//! The fixed-step clock and a flight session together: one control script
//! flown at 30, 60 and 120 display frames a second ends in exactly the same
//! state.

use std::time::Duration;

use nova_sim::{
    CharacterStart, CommodityStrings, Controls, DisasterRecord, FixedStep, JunkRecord, LandingSite,
    OutfitMod, PilotCatalog, Session, ShipFields, ShipId, ShipState, StarSystem, StartDate,
    StartError, SystemId, Turn,
};

/// One `chär` flying an agile ship from system 130.
struct Pilot;

impl PilotCatalog for Pilot {
    fn first_character(&self) -> Result<CharacterStart, StartError> {
        Ok(CharacterStart {
            ship: Some(ShipId(128)),
            systems: [Some(SystemId(130)), None, None, None],
            start: StartDate::default(),
            ..CharacterStart::default()
        })
    }

    fn ship_fields(&self, _id: ShipId) -> Result<ShipFields, String> {
        Ok(ShipFields {
            speed: 450,
            accel: 600,
            maneuver: 25,
            ..ShipFields::default()
        })
    }

    fn default_outfits(&self, _id: ShipId) -> Vec<OutfitMod> {
        Vec::new()
    }

    fn system_exists(&self, _id: SystemId) -> bool {
        true
    }

    fn landing_sites(&self, _system: SystemId) -> Vec<LandingSite> {
        Vec::new()
    }

    fn star_map(&self) -> Vec<StarSystem> {
        Vec::new()
    }

    fn commodity_strings(&self) -> CommodityStrings {
        CommodityStrings::default()
    }

    fn junk(&self) -> Vec<JunkRecord> {
        Vec::new()
    }

    fn disasters(&self) -> Vec<DisasterRecord> {
        Vec::new()
    }
}

/// The script, by half-seconds: thrust for a second, turn right for half
/// a second, reverse for a second, then coast to the end of the third.
fn controls(half_second: u64) -> Controls {
    match half_second {
        0 | 1 => Controls {
            thrust: true,
            ..Controls::default()
        },
        2 => Controls {
            turn: Turn::Right,
            ..Controls::default()
        },
        3 | 4 => Controls {
            reverse: true,
            ..Controls::default()
        },
        _ => Controls::default(),
    }
}

/// Flies the script for three seconds of frames at `fps`, each redraw at
/// `n / fps` seconds to the nanosecond, as a display clock reads.
fn fly(fps: u64) -> ShipState {
    let mut session = Session::start(&Pilot).expect("starts");
    let mut clock = FixedStep::new();
    let mut last = 0;
    for frame in 1..=3 * fps {
        // The keys held through this frame: those of the half-second it
        // starts in.
        let held = controls((frame - 1) * 2 / fps);
        let now = frame * 1_000_000_000 / fps;
        for _ in 0..clock.advance(Duration::from_nanos(now - last)).steps {
            session.tick(held);
        }
        last = now;
    }
    *session.player()
}

#[test]
fn the_same_script_ends_in_the_same_state_at_30_60_and_120_fps() {
    let at_30 = fly(30);
    assert_ne!(at_30.position, ShipState::default().position, "it flew");
    assert!(at_30.heading > 0.0, "it turned: {at_30:?}");
    assert_eq!(fly(60), at_30);
    assert_eq!(fly(120), at_30);
}
