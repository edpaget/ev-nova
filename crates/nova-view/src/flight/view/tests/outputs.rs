//! The one-shot outputs the app and the router take, over a scripted
//! flight: each comes from what the flight drained, once.

use super::*;
use crate::flight::escorts::fighters_abandoned_message;
use nova_sim::flight_frame::FlightEvent;

#[test]
fn the_save_due_the_holds_expiry_and_the_rescues_desc_are_taken_once() {
    let mut view = faked(in_sol());
    assert!(!view.take_save_due());
    assert!(!view.take_hold_expired());
    assert_eq!(view.take_rescue_desc(), None);
    for event in [
        FlightEvent::SaveDue,
        FlightEvent::HoldExpired,
        FlightEvent::RescueDesc(13999),
    ] {
        view.sim.queue_event(event);
    }
    assert!(view.take_save_due());
    assert!(
        view.take_hold_expired(),
        "kept in the inbox for its own take"
    );
    assert_eq!(view.take_rescue_desc(), Some(13999));
    assert!(!view.take_save_due(), "once");
    assert!(!view.take_hold_expired(), "once");
    assert_eq!(view.take_rescue_desc(), None, "once");
}

#[test]
fn the_flights_sounds_come_before_the_interfaces() {
    let mut view = faked(in_sol());
    view.ui_sounds.push(UiSound::MapOpened);
    view.sim.queue_event(FlightEvent::Sound(SimSound::Refused));
    view.sim
        .queue_event(FlightEvent::Sound(SimSound::NavSelected));
    assert_eq!(
        view.drained_sounds(),
        [
            Sound::Sim(SimSound::Refused),
            Sound::Sim(SimSound::NavSelected),
            Sound::Ui(UiSound::MapOpened),
        ]
    );
    assert_eq!(view.drained_sounds(), [], "taken");
}

#[test]
fn the_flights_diagnostics_follow_the_looks_unread() {
    let mut view = faked(in_sol());
    let unread = Diagnostic::Sim(nova_sim::SimDiagnostic::UnimplementedGuidance {
        weapon: nova_sim::WeaponId(200),
        guidance: 7,
    });
    view.unread_looks.push(unread.clone());
    let sim = nova_sim::SimDiagnostic::UnimplementedGuidance {
        weapon: nova_sim::WeaponId(128),
        guidance: 42,
    };
    view.sim.queue_event(FlightEvent::Diagnostic(sim));
    assert_eq!(view.drained_diagnostics(), [unread, Diagnostic::Sim(sim)]);
    assert_eq!(view.drained_diagnostics(), [], "each once");
}

#[test]
fn the_fighters_abandoned_are_said_each_for_a_message_slot() {
    let mut view = faked(in_sol());
    view.sim
        .queue_event(FlightEvent::FighterNote(FighterNote::Abandoned(2)));
    view.sim
        .queue_event(FlightEvent::FighterNote(FighterNote::Abandoned(1)));
    assert_eq!(
        view.fighters_said(),
        [
            (fighters_abandoned_message(2), MESSAGE_TICKS),
            (fighters_abandoned_message(1), MESSAGE_TICKS),
        ]
    );
    assert_eq!(view.fighters_said(), [], "once");
}

#[test]
fn an_arrival_takes_its_fighters_pay_and_messages_and_leaves_the_rest() {
    let mut view = faked(in_sol());
    let message = SimMessage::Buoy("Welcome.".to_owned());
    for event in [
        FlightEvent::FighterNote(FighterNote::Abandoned(3)),
        FlightEvent::PayNote(nova_sim::PayNote::Defected(1)),
        FlightEvent::Message(message.clone()),
        FlightEvent::TakeoffNav(NavMode::Off),
    ] {
        view.sim.queue_event(event);
    }
    let (fighters, pay, messages) = view.arrival_outputs();
    assert_eq!(fighters, [(fighters_abandoned_message(3), MESSAGE_TICKS)]);
    assert_eq!(pay, pay_said(&[nova_sim::PayNote::Defected(1)]));
    assert!(!pay.is_empty(), "the defection is said");
    assert_eq!(messages, [message]);
    let (_, _, held) = view.takeoff_outputs();
    assert_eq!(held, Some(NavMode::Off), "another kind, kept for its take");
}

#[test]
fn a_take_off_says_its_held_q_message_for_a_scripts_slot_and_takes_its_nav() {
    let mut view = faked(in_sol());
    assert_eq!(view.leave_said(), []);
    let message = SimMessage::Buoy("Hello.".to_owned());
    for event in [
        FlightEvent::LeaveMessage("Off you go.".to_owned()),
        FlightEvent::PayNote(nova_sim::PayNote::Defected(2)),
        FlightEvent::Message(message.clone()),
        FlightEvent::TakeoffNav(NavMode::Stellar(Some(StellarId(128)))),
    ] {
        view.sim.queue_event(event);
    }
    assert_eq!(
        view.leave_said(),
        [("Off you go.".to_owned(), SCRIPT_MESSAGE_TICKS)]
    );
    let (pay, messages, held) = view.takeoff_outputs();
    assert_eq!(pay, pay_said(&[nova_sim::PayNote::Defected(2)]));
    assert_eq!(messages, [message]);
    assert_eq!(held, Some(NavMode::Stellar(Some(StellarId(128)))));
    assert_eq!(
        view.takeoff_outputs(),
        (Vec::new(), Vec::new(), None),
        "once"
    );
}

/// A mission text of `mission`, its bar noted at the `texts`th queued
/// with the nav `nav`.
fn queued_text(mission: i16, texts: u64, nav: NavMode) -> FlightEvent {
    FlightEvent::MissionShown {
        shown: MissionShown::Text(nova_sim::MissionText {
            kind: nova_sim::MissionTextKind::Brief,
            mission: MissionId(mission),
            text: format!("text {mission}"),
            graphic: None,
            movie: None,
            map: None,
        }),
        bar: Some(TextBar {
            texts,
            nav,
            reserves: nova_sim::Reserves::default(),
        }),
    }
}

#[test]
fn a_text_queued_since_the_nav_was_taken_notes_its_nav_and_an_older_one_none() {
    let nav = NavMode::Stellar(Some(StellarId(128)));
    let mut view = faked(FlightFrame {
        texts_queued: 2,
        ..in_sol()
    });
    view.nav_texts = 1;
    view.sim.queue_event(queued_text(128, 2, nav));
    let taken = view.flight_texts();
    assert_eq!(taken.texts.len(), 1);
    assert_eq!(taken.noted, Some(nav), "queued since");
    assert!(!taken.staged, "not staged");
    view.nav_texts = 2;
    view.sim.queue_event(queued_text(129, 2, nav));
    assert_eq!(view.flight_texts().noted, None, "none since");
    assert_eq!(view.flight_texts().texts, [], "taken");
}

#[test]
fn texts_whose_bars_were_all_staged_are_taken_as_staged() {
    let mut view = faked(in_sol());
    view.bars_staged = 3;
    view.sim.queue_event(queued_text(128, 2, NavMode::Off));
    view.sim.queue_event(queued_text(129, 3, NavMode::Off));
    assert!(view.flight_texts().staged);
    view.sim.queue_event(queued_text(130, 3, NavMode::Off));
    view.sim.queue_event(queued_text(131, 4, NavMode::Off));
    assert!(!view.flight_texts().staged, "one newer");
}

#[test]
fn the_bars_not_yet_staged_are_those_queued_after_the_last_staged() {
    let mut view = faked(in_sol());
    view.bars_staged = 1;
    view.sim.queue_event(queued_text(128, 1, NavMode::Off));
    view.sim.queue_event(queued_text(129, 2, NavMode::Off));
    let texts: Vec<u64> = view.unstaged_bars().iter().map(|bar| bar.texts).collect();
    assert_eq!(texts, [2]);
    assert_eq!(view.docked_texts().len(), 2, "the bars left the texts be");
    assert_eq!(view.unstaged_bars(), [], "taken with them");
}

#[test]
fn the_interface_changes_are_the_loads_since_the_last_run_that_each_load_another_intf() {
    let catalog = FakeCatalog {
        interfaces: vec![(150, 129)],
        ..catalog()
    };
    let mut view = faked_in(catalog, in_sol());
    let load = |government, texts| InterfaceLoad {
        government,
        nav: NavMode::Off,
        reserves: nova_sim::Reserves::default(),
        texts,
    };
    view.sim
        .queue_event(FlightEvent::InterfaceLoad(load(None, 0)));
    view.sim.queue_event(FlightEvent::InterfaceLoadsCleared);
    let before = hud::interface_for(&*view.catalog, None);
    for event in [
        load(None, 1),
        load(Some(GovtId(150)), 2),
        load(Some(GovtId(150)), 3),
    ] {
        view.sim.queue_event(FlightEvent::InterfaceLoad(event));
    }
    let (changes, after) = view.interface_changes(Some(&before));
    let texts: Vec<u64> = changes.iter().map(|load| load.texts).collect();
    assert_ne!(
        hud::interface_for(&*view.catalog, Some(GovtId(150))),
        before,
        "the fixture's 150 flies another bar"
    );
    assert_eq!(texts, [2], "the cleared one let go, the same intf skipped");
    assert_eq!(
        after,
        Some(hud::interface_for(&*view.catalog, Some(GovtId(150))))
    );
    assert_eq!(view.interface_changes(None), (Vec::new(), None), "taken");
}

#[test]
fn a_drained_board_line_and_grant_are_each_taken_once_and_leave_the_rest() {
    let mut view = faked(in_sol());
    assert_eq!(view.board_line_drained(), None);
    assert_eq!(view.grant_drained(), None);
    let granted = nova_sim::Granted {
        outfit: nova_sim::OutfitId(128),
        count: 2,
    };
    let refuelled = MissionBoardLine::Refuelled { ship: ShipId(129) };
    for event in [
        FlightEvent::BoardLine(MissionBoardLine::Boarded),
        FlightEvent::Grant(granted),
        FlightEvent::BoardLine(refuelled.clone()),
        FlightEvent::SaveDue,
    ] {
        view.sim.queue_event(event);
    }
    assert_eq!(
        view.board_line_drained(),
        Some(refuelled),
        "the last board's"
    );
    assert_eq!(view.grant_drained(), Some(granted));
    assert_eq!(view.board_line_drained(), None, "once");
    assert_eq!(view.grant_drained(), None, "once");
    assert!(view.take_save_due(), "another kind, kept for its take");
}

/// Drains `view`'s flight as a frame's steps take from it: the comm
/// notes on the advance pass, then the quotes, then relays them.
fn relay_the_steps(view: &mut Faked) {
    let notes = view.drain().take_comm();
    let quotes = view.drain().take_quotes();
    view.relay_comm_and_quotes(&notes, quotes);
}

#[test]
fn a_drained_comm_note_is_relayed_in_its_words_for_its_ticks() {
    let mut view = faked(in_sol());
    let note = nova_sim::CommNote {
        from: nova_sim::NpcId(3),
        comm_name: "Shuttle".to_owned(),
        ship_name: "Kestrel".to_owned(),
        done: nova_sim::Help::Refuel,
    };
    view.sim.queue_event(FlightEvent::Comm(note.clone()));
    relay_the_steps(&mut view);
    let shown = view.message.clone().expect("relayed");
    assert_eq!(
        (shown.text, shown.ticks),
        (comm_message(&note), note.ticks())
    );
    assert_eq!(sent(&view), [], "the session's line left alone");
    view.message = None;
    relay_the_steps(&mut view);
    assert_eq!(view.message, None, "relayed once");
}

#[test]
fn a_drained_quote_is_relayed_after_the_comm_and_stays() {
    let mut view = faked(in_sol());
    let note = nova_sim::CommNote {
        from: nova_sim::NpcId(3),
        comm_name: "Shuttle".to_owned(),
        ship_name: "Kestrel".to_owned(),
        done: nova_sim::Help::Repair,
    };
    let quote = nova_sim::PersonQuote {
        npc: nova_sim::NpcId(4),
        text: "Prepare to die.".to_owned(),
    };
    let ticks = quote.ticks();
    view.sim.queue_event(FlightEvent::Quote(quote));
    view.sim.queue_event(FlightEvent::Comm(note));
    relay_the_steps(&mut view);
    let shown = view.message.clone().expect("relayed");
    assert_eq!(
        (shown.text.as_str(), shown.ticks),
        ("Prepare to die.", ticks),
        "the quote over the comm, whatever the drain's order"
    );
}
