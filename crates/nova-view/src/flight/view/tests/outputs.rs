//! The one-shot outputs the app and the router take, over a scripted
//! flight: each comes from what the flight drained, once.

use super::*;
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
