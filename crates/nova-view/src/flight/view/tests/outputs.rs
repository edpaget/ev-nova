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

#[test]
fn the_steps_comm_then_quotes_are_relayed_the_last_staying_and_nothing_sent() {
    let mut view = faked(in_sol());
    let note = nova_sim::CommNote {
        from: nova_sim::NpcId(3),
        comm_name: "Shuttle".to_owned(),
        ship_name: "Kestrel".to_owned(),
        done: nova_sim::Help::Refuel,
    };
    view.relay_comm_and_quotes(std::slice::from_ref(&note), Vec::new());
    let shown = view.message.clone().expect("relayed");
    assert_eq!(
        (shown.text, shown.ticks),
        (comm_message(&note), note.ticks())
    );
    let quote = nova_sim::PersonQuote {
        npc: nova_sim::NpcId(4),
        text: "Prepare to die.".to_owned(),
    };
    let ticks = quote.ticks();
    view.relay_comm_and_quotes(&[note], vec![quote]);
    let shown = view.message.clone().expect("relayed");
    assert_eq!(
        (shown.text.as_str(), shown.ticks),
        ("Prepare to die.", ticks)
    );
    assert_eq!(sent(&view), [], "the session's line left alone");
}
