//! The stock jump's arrival by the engine's reading of `jump_arrival`
//! (`_HandlePlayer` @0x6bc0d-0x6bd20; see `nova_sim::jump_arrival`): the
//! stock starting ship jumps to a linked system and comes in 1350 from the
//! centre on the side it left, at its top speed. The no-jump zone is
//! 1000 plus each owned `ModType` 23 outfit's `ModVal` × count
//! (`_ShipHyperSafeDist` @0x4485-0x4511); stock's one such outfit, the
//! Horizontal Booster (`oütf` 202), lowers it by 500, so every stock
//! arrival is outside the zone. Skips, passing, when `NOVA_DATA` is unset.

use crate::common;

use nova_data::GameData;
use nova_sim::hyperspace::MIN_JUMP_DISTANCE;
use nova_sim::jump_arrival::ARRIVAL_RADIUS;
use nova_sim::stats::HYPERSPACE_DISTANCE;
use nova_sim::{Controls, NeverFires, OutfitId, Pilot, PilotCatalog, Session};

#[test]
fn a_stock_jump_arrives_1350_out_on_the_side_it_left_at_top_speed() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let pilot = Pilot::new(&data, "Stock").expect("the stock first chär starts");
    let mut session = Session::fly(&data, pilot).expect("flies");
    let departure = session.system();
    let next = session
        .star_map()
        .listed_links(departure)
        .first()
        .copied()
        .expect("a hyperlink");
    session.plot_course(next).expect("a route");
    let thrust = Controls {
        thrust: true,
        ..Controls::default()
    };
    for _ in 0..10_000 {
        if session.player().position.length() >= session.stats().jump_distance {
            break;
        }
        session.tick(thrust);
    }
    session.begin_jump().expect("jumps");
    for _ in 0..10_000 {
        if session.warp_end_due() {
            break;
        }
        session.tick(Controls::default());
    }
    assert!(session.warp_end_due(), "the warp ends");
    let heading = session.player().heading;
    assert_eq!(session.arrive(&data, &mut NeverFires), Some(next));

    let player = *session.player();
    assert!(
        (player.position.length() - ARRIVAL_RADIUS).abs() < 1e-3,
        "{player:?}"
    );
    let map = session.star_map();
    let travel =
        map.position(next).expect("on the map") - map.position(departure).expect("on the map");
    assert!(
        player.position.x * travel.x + player.position.y * travel.y < 0.0,
        "on the side it left: {player:?}, travelling {travel:?}"
    );
    assert_eq!(
        player.heading.to_bits(),
        heading.to_bits(),
        "the warp's heading kept"
    );
    let top = session.top_speed();
    assert!(top > 0.0);
    assert!(
        (player.velocity.length() - top).abs() < 1e-3,
        "{player:?} at {top}"
    );
    assert!(player.position.length() > session.stats().jump_distance);
}

#[test]
fn no_stock_outfit_raises_the_no_jump_zone_past_the_arrival() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let mut movers = Vec::new();
    let mut widest = MIN_JUMP_DISTANCE;
    for outfit in data.outfits() {
        for &(mod_type, mod_val) in &outfit.mods {
            if mod_type == HYPERSPACE_DISTANCE {
                movers.push((outfit.id, mod_val, outfit.max));
                widest += f32::from(mod_val.max(0)) * f32::from(outfit.max.max(1));
            }
        }
    }
    assert_eq!(movers, [(OutfitId(202), -500, 1)]);
    assert!(widest <= MIN_JUMP_DISTANCE, "{widest}");
    assert!(widest < ARRIVAL_RADIUS);
}
