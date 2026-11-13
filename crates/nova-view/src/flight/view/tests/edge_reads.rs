//! The reads the edge's checks over a real flight make of the view
//! (`edge_reads.rs`, rdm:phase/faster-builds/phase-6-nova-view-off-nova-sim),
//! pinned here over a scripted flight, so this crate's own tests catch a
//! read that stops reading what the view holds. nova-flight's
//! `edge::edge_checks` use them over the real flight.

use super::*;

#[test]
fn worded_says_each_message_in_order_with_its_ticks_and_holds_back_an_arrivals() {
    let mut view = faked(in_sol());
    let first = SimMessage::FleetArrivalQuote("Hail 9".to_owned());
    let second = SimMessage::FleetArrivalQuote("Hail 10".to_owned());
    let said = view.worded(vec![
        first.clone(),
        SimMessage::Arrived(SystemId(130)),
        second.clone(),
    ]);
    assert_eq!(
        said,
        [
            ("Hail 9".to_owned(), first.ticks()),
            ("Hail 10".to_owned(), second.ticks()),
        ]
    );
    assert_eq!(
        view.pending_leads,
        [SimMessage::Arrived(SystemId(130))],
        "held for its arrival"
    );
}

#[test]
fn words_chance_is_the_chance_the_view_words_on() {
    let mut view = faked(in_sol());
    let chance = SharedChance::default();
    view.words_chance = chance.clone();
    assert!(view.words_chance().shares(&chance), "the view's");
    assert!(
        !view.words_chance().shares(&SharedChance::default()),
        "not another"
    );
}

#[test]
fn stars_due_says_whether_a_scatter_waits_and_clearing_it_lets_it_go() {
    let mut view = faked(in_sol());
    assert!(view.stars_due(), "due as the view opens");
    view.clear_stars_due();
    assert!(!view.stars_due(), "let go");
    view.enter_game(|_| true);
    assert!(view.stars_due(), "due again once play is entered");
}

#[test]
fn kept_frame_is_the_frame_the_view_last_filled() {
    let at = |x: f32| FlightFrame {
        ships: vec![record::player(128, Vec2::new(x, 0.0), 0.0)],
        ..in_sol()
    };
    let mut view = faked(at(5.0));
    assert_eq!(view.kept_frame(), &at(5.0), "filled as it opens");
    view.sim.set_frame(at(9.0));
    assert_eq!(view.kept_frame(), &at(5.0), "kept between fills");
    view.refresh_frame();
    assert_eq!(view.kept_frame(), &at(9.0));
}

#[test]
fn reads_npc_sheet_says_which_ship_types_sheets_the_view_has_read() {
    let mut frame = in_sol();
    frame.traffic_ships = vec![ShipId(129)];
    let mut view = faked(frame);
    assert!(!view.reads_npc_sheet(ShipId(129)), "none read yet");
    view.show_set_up(true);
    assert!(view.reads_npc_sheet(ShipId(129)), "the traffic's, read");
    assert!(!view.reads_npc_sheet(ShipId(130)), "one no frame asked for");
}

#[test]
fn open_gate_map_opens_the_hypergate_map_offering_the_gates() {
    let mut view = faked(in_sol());
    let opening = map_opening(&view.frame, Some(130));
    view.sim.queue_screen(opening);
    view.open_gate_map(vec![SystemId(132), SystemId(133)]);
    assert!(view.map_open(), "opened");
    assert_eq!(view.map.mode(), MapMode::Hypergate);
    assert_eq!(view.map.offered(), [SystemId(132), SystemId(133)]);
}
