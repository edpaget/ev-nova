//! The ships' running lights over the stock data (rdm
//! ship-handling-fidelity phase 22): every stock `shän` with a lights
//! image steps its ported lights state from rest exactly as the stateless
//! `lights_level` solves it, and the 0x0040 ships, whose lights go out
//! while disabled, blink steady, square or triangle, never random. Skips,
//! passing, when `NOVA_DATA` is unset.

use crate::common;

use nova_data::GameData;
use nova_sim::combat::hull::HullSpec;
use nova_sim::combat::ship_frames::LIGHTS_OUT_WHILE_DISABLED;
use nova_sim::combat::ship_lights::ShipLights;
use nova_sim::{BlinkChance, CombatCatalog, HashedRolls, lights_level};

/// Every stock class with a lights image, its display.
fn lit_classes(data: &GameData) -> Vec<(i16, nova_sim::combat::ship_frames::FrameSpec)> {
    data.hulls()
        .iter()
        .map(|record| (record.id.0, HullSpec::new(record).frames))
        .filter(|(_, spec)| spec.lights)
        .collect()
}

#[test]
fn every_stock_ships_lights_state_matches_lights_level_from_rest() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let classes = lit_classes(&data);
    assert!(classes.len() > 100, "{}", classes.len());
    let rolls = HashedRolls::new(0);
    for (id, spec) in classes {
        let mut lights = ShipLights::default();
        let mut change = 0;
        for tick in 0..600 {
            lights.step(&spec.blink, |sides| {
                let roll = rolls.roll(change, sides);
                change += 1;
                roll
            });
            assert_eq!(
                lights.level(),
                lights_level(&spec.blink, tick, &rolls),
                "shän {id} at tick {tick}"
            );
        }
    }
}

#[test]
fn the_stock_ships_whose_lights_go_out_blink_steady_square_or_triangle() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let mut modes = [0; 4];
    for (_, spec) in lit_classes(&data) {
        if spec.has(LIGHTS_OUT_WHILE_DISABLED) {
            let mode = match spec.blink.mode {
                1..=3 => usize::try_from(spec.blink.mode).unwrap_or(0),
                _ => 0,
            };
            modes[mode] += 1;
        }
    }
    assert_eq!(modes, [17, 38, 51, 0], "steady, square, triangle, random");
}
