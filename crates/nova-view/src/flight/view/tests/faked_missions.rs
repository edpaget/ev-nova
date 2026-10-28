//! The mission computer's, the offers', the mission info's and the
//! mission info key's component tests on [`FakeFlight`]
//! (rdm:phase/view-sim-seam/phase-11-spaceport-on-port): the commands
//! they send, what the view hands back from the flight's answers, what it
//! says, and the bar's redraws around an accept, a refusal and an abort.
//! What the missions offer, accept and run is nova-sim's
//! (`session::apply::missions` and the session's own tests). Kept apart
//! from `tests/mod.rs`, which is large.

use super::faked_shops::{
    EARTH, PROXIMA, UNDER_129, WHOLE_BAR, brief, docked, docked_on, docked_upgrading, fuelled,
    full, leaves, load, purchase, stale, writes_nav,
};
use super::faked_travel::{faked_lasts, sent_but_lines};
use super::*;
use crate::flight::radar::RadarScene;
use nova_sim::flight_frame::FlightEvent;
use nova_sim::mission::screens::{mac_ticks, offer_labels};
use nova_sim::navigation::NavMode;
use nova_sim::{
    ComputerRow, Gauge, InfoRow, MissionComputer, MissionInfo, MissionInfoPress, OfferPrompt,
    RefuseShown, SimMessage,
};

/// The mission computer listing mission 131, "Walk".
fn computer() -> MissionComputer {
    MissionComputer {
        rows: vec![walk()],
        today: "June 23rd, 1177 NC".to_owned(),
    }
}

/// Mission 131's computer row.
fn walk() -> ComputerRow {
    ComputerRow {
        mission: MissionId(131),
        name: "Walk".to_owned(),
        raw_name: "Walk".to_owned(),
        text: "Walk it.".to_owned(),
    }
}

/// The mission info listing mission 130 in slot 0.
fn info() -> MissionInfo {
    MissionInfo {
        rows: vec![InfoRow {
            slot: 0,
            name: "Run".to_owned(),
            failed: false,
            text: "Quick.".to_owned(),
            abortable: true,
        }],
        today: "June 23rd, 1177 NC".to_owned(),
    }
}

/// A refusal of an offer whose dialog was hidden.
const REFUSED: RefuseShown = RefuseShown { offer_hidden: true };

// The mission computer and the venues.

#[test]
fn the_mission_computer_opens_and_selects_its_rows_through_the_flight() {
    let mut view = docked();
    view.sim
        .queue_answer(Answer::MissionComputer(Ok(computer())));
    assert_eq!(view.open_mission_computer(), Ok(computer()));
    view.sim
        .queue_answer(Answer::MissionComputer(Err(ComputerRefusal::SlotsFull)));
    assert_eq!(
        view.open_mission_computer(),
        Err(ComputerRefusal::SlotsFull)
    );
    assert_eq!(
        view.open_mission_computer(),
        Err(ComputerRefusal::NoneAvailable),
        "refused: a flight that never started has nothing available"
    );
    view.sim.queue_answer(Answer::ComputerRow(Some(walk())));
    assert_eq!(view.show_computer_row(MissionId(131)), Some(walk()));
    assert_eq!(view.show_computer_row(MissionId(131)), None, "refused");
    assert_eq!(
        sent(&view),
        [
            FlightCommand::OpenMissionComputer,
            FlightCommand::OpenMissionComputer,
            FlightCommand::OpenMissionComputer,
            FlightCommand::ShowComputerRow(MissionId(131)),
            FlightCommand::ShowComputerRow(MissionId(131)),
        ]
    );
}

#[test]
fn entering_a_venue_and_its_next_offer_go_through_the_flight() {
    let mut view = docked();
    let refused = VenueOffer::Accepted {
        mission: MissionId(150),
        accepted: Err(AcceptError::Refused(
            nova_sim::mission::AcceptRefusal::SlotsFull,
        )),
    };
    view.enter_offer_venue(1);
    view.sim.queue_answer(Answer::Offer(Some(refused.clone())));
    assert_eq!(view.next_offer(1), Some(refused));
    view.sim.queue_answer(Answer::Offer(None));
    assert_eq!(view.next_offer(1), None, "suppressed");
    assert_eq!(view.next_offer(1), None, "refused");
    assert_eq!(
        sent(&view),
        [
            FlightCommand::EnterOfferVenue(1),
            FlightCommand::NextOffer(1),
            FlightCommand::NextOffer(1),
            FlightCommand::NextOffer(1),
        ]
    );
}

#[test]
fn the_wait_before_a_venues_next_offer_is_the_flights_in_mac_ticks() {
    let mut view = docked();
    assert_eq!(
        view.offer_gap(),
        mac_ticks(nova_sim::mission::screens::OFFER_GAP_TICKS)
    );
    view.sim.port_mut().offer_gap = Some(37);
    assert_eq!(view.offer_gap(), mac_ticks(37));
}

#[test]
fn reloading_an_offers_desc_sends_its_mission() {
    let mut view = docked();
    view.reload_offer_desc(MissionId(142));
    assert_eq!(
        sent(&view),
        [FlightCommand::ReloadOfferDesc(MissionId(142))]
    );
}

// The accept and the refusal.

#[test]
fn an_accept_and_a_refusal_send_their_mission_and_hand_back_the_flights_answers() {
    let mut view = docked();
    view.sim.queue_answer(Answer::Accepted(Ok(0)));
    assert_eq!(view.accept_mission(MissionId(130)), Ok(0));
    view.sim.queue_answer(Answer::RefuseShown(Ok(REFUSED)));
    assert_eq!(view.refuse_mission(MissionId(131)), Ok(REFUSED));
    view.sim.queue_answer(Answer::RefuseShown(Err(NotOffered)));
    assert_eq!(
        view.refuse_mission(MissionId(131)),
        Err(NotOffered),
        "answered"
    );
    assert_eq!(
        view.accept_mission(MissionId(130)),
        Err(AcceptError::NotOffered),
        "refused: a flight that never started offers nothing"
    );
    assert_eq!(
        view.refuse_mission(MissionId(131)),
        Err(NotOffered),
        "refused"
    );
    assert_eq!(
        sent(&view),
        [
            FlightCommand::AcceptMission(MissionId(130)),
            FlightCommand::RefuseMission(MissionId(131)),
            FlightCommand::RefuseMission(MissionId(131)),
            FlightCommand::AcceptMission(MissionId(130)),
            FlightCommand::RefuseMission(MissionId(131)),
        ]
    );
}

#[test]
fn closing_a_mission_dialog_or_its_texts_redraws_the_hud() {
    let mut view = faked(in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)]));
    let stale = RadarScene::default();
    let fresh = view.radar_now(&view.frame_now());
    assert_ne!(fresh, stale);
    view.radar_scene = Some(stale.clone());
    view.sim.queue_answer(Answer::Accepted(Ok(0)));
    let _ = view.accept_mission(MissionId(130));
    assert_eq!(view.radar_scene.as_ref(), Some(&fresh), "accepted");
    view.radar_scene = Some(stale.clone());
    let _ = view.refuse_mission(MissionId(130));
    assert_eq!(view.radar_scene.as_ref(), Some(&fresh), "refused");
    view.radar_scene = Some(stale.clone());
    assert_eq!(view.take_mission_texts(), []);
    assert_eq!(view.radar_scene.as_ref(), Some(&stale), "nothing shown");
    // Docked, the held gauges are redrawn too.
    let mut view = docked();
    let held = Reserves::default();
    view.reserves_scene = Some(held);
    assert_eq!(view.reserves_shown(&view.frame_now()), held, "held");
    view.sim.queue_answer(Answer::RefuseShown(Ok(REFUSED)));
    assert!(view.refuse_mission(MissionId(131)).is_ok());
    assert_eq!(
        view.reserves_shown(&view.frame_now()),
        view.reserves(),
        "refused docked"
    );
    view.reserves_scene = Some(held);
    view.radar_scene = Some(stale.clone());
    view.sim.queue_event(brief(1, EARTH, held));
    view.sim.queue_answer(Answer::Accepted(Ok(0)));
    assert_eq!(view.accept_mission(MissionId(130)), Ok(0));
    assert_eq!(view.take_mission_texts().len(), 1);
    assert_eq!(
        view.reserves_shown(&view.frame_now()),
        held,
        "the brief shown over the bar"
    );
    assert_eq!(view.radar_scene.as_ref(), Some(&stale), "the brief shown");
    view.close_mission_text();
    assert_eq!(
        view.radar_scene,
        Some(view.radar_now(&view.frame_now())),
        "the brief closed"
    );
    assert_eq!(
        view.reserves_shown(&view.frame_now()),
        view.reserves(),
        "the brief closed"
    );
}

/// How an accept is made: from the mission computer, from the offer
/// dialog with a refuse button, or at once with no dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Accepted {
    Computer,
    Offer,
    AtOnce,
}

/// An `OnAccept` of `H130 G201 M131` (`changes`) or `H129 G201 M131`
/// for mission 131, queued on `view`'s flight as the session would raise
/// it, its brief after it: the frame the accept leaves.
fn on_accept(view: &mut Faked, changes: bool) -> FlightFrame {
    let tanked = Reserves {
        fuel: Gauge::full(350.0),
        ..full()
    };
    let government = if changes { UNDER_129 } else { None };
    view.sim.queue_event(load(government, EARTH, full(), 0));
    view.sim.queue_event(brief(1, PROXIMA, tanked));
    leaves(view, |frame| {
        frame.nav_mode = PROXIMA;
        frame.hud.reserves = tanked;
        frame.hud.interface_government = government;
        frame.texts_queued = 1;
    })
}

#[test]
fn a_brief_text_shows_over_the_bar_as_on_accept_left_it() {
    for changes in [true, false] {
        for how in [Accepted::Computer, Accepted::Offer, Accepted::AtOnce] {
            let what = format!("changes {changes}, {how:?}");
            let mut view = docked_upgrading();
            if how == Accepted::Offer {
                view.sim.port_mut().refusable = vec![MissionId(131)];
            }
            let before = view.reserves();
            stale(&mut view);
            let after = on_accept(&mut view, changes);
            if how == Accepted::AtOnce {
                let at_once = VenueOffer::Accepted {
                    mission: MissionId(131),
                    accepted: Ok(0),
                };
                view.sim
                    .queue_answer_showing(Answer::Offer(Some(at_once)), after);
                assert!(view.next_offer(SPACEPORT).is_some(), "{what}");
            } else {
                view.sim
                    .queue_answer_showing(Answer::Accepted(Ok(0)), after);
                assert_eq!(view.accept_mission(MissionId(131)), Ok(0), "{what}");
            }
            assert_eq!(view.take_mission_texts().len(), 1, "{what}: the brief");
            let shown = view.reserves_shown(&view.frame_now());
            match (changes, how) {
                (true, _) => {
                    assert_eq!(shown, full(), "{what}: as at the H, before G201's tank");
                    assert_eq!(port_nav(&view), naming("Earth"), "{what}: before the M");
                }
                (false, Accepted::Offer) => {
                    assert_eq!(shown, before, "{what}: the dialog's redraw before it");
                }
                (false, _) => {
                    assert_eq!(retaken(&view), [false; 4], "{what}: held");
                    assert_eq!(port_nav(&view), naming("Stale"), "{what}");
                }
            }
            view.close_mission_text();
            assert_eq!(retaken(&view), WHOLE_BAR, "{what}: the brief closed");
            assert_eq!(port_nav(&view), naming("Proxima"), "{what}");
        }
    }
}

#[test]
fn a_refuse_text_shows_over_the_bar_from_before_the_refusal() {
    for government in [None, UNDER_129] {
        let mut view = docked_upgrading();
        view.sim.port_mut().refusable = vec![MissionId(130)];
        let before = view.reserves();
        // The `RefuseText` noting the bar before `OnRefuse`, whose load
        // and `M` follow it.
        view.sim.queue_event(brief(1, EARTH, before));
        view.sim
            .queue_event(load(government, PROXIMA, fuelled(150.0), 1));
        view.sim.queue_answer_showing(
            Answer::RefuseShown(Ok(REFUSED)),
            leaves(&view, |frame| {
                frame.nav_mode = PROXIMA;
                frame.hud.interface_government = government;
                frame.texts_queued = 1;
            }),
        );
        stale(&mut view);
        assert!(view.refuse_mission(MissionId(130)).is_ok());
        assert_eq!(view.take_mission_texts().len(), 1, "{government:?}");
        assert_eq!(
            view.reserves_shown(&view.frame_now()),
            before,
            "{government:?}: before OnRefuse"
        );
        assert_eq!(port_nav(&view), naming("Earth"), "{government:?}");
        view.close_mission_text();
        assert_eq!(retaken(&view), WHOLE_BAR, "{government:?}: the text closed");
        assert_eq!(port_nav(&view), naming("Proxima"), "{government:?}");
    }
}

#[test]
fn a_dialogs_redraw_shows_what_the_script_run_with_it_wrote() {
    type Step = fn(&mut Faked);
    let rows: [(&str, Answer, Step); 3] = [
        ("OnAccept", Answer::Accepted(Ok(0)), |view| {
            assert_eq!(view.accept_mission(MissionId(130)), Ok(0));
        }),
        ("OnRefuse", Answer::RefuseShown(Ok(REFUSED)), |view| {
            assert!(view.refuse_mission(MissionId(130)).is_ok());
        }),
        (
            "a ship's OnPurchase",
            Answer::ShipBought(Ok(purchase())),
            |view| {
                view.buy_ship(ShipId(129), "Kestrel").expect("bought");
            },
        ),
    ];
    for (what, answer, step) in rows {
        let mut view = docked();
        let written = leaves(&view, |frame| frame.nav_mode = PROXIMA);
        view.sim.queue_answer_showing(answer, written);
        step(&mut view);
        assert_eq!(port_nav(&view), naming("Proxima"), "{what}");
    }
}

#[test]
fn a_text_never_shows_over_a_later_redraw() {
    let mut view = docked();
    view.sim.queue_event(brief(1, PROXIMA, full()));
    let written = leaves(&view, |frame| {
        frame.nav_mode = PROXIMA;
        frame.texts_queued = 1;
    });
    view.sim
        .queue_answer_showing(Answer::Accepted(Ok(0)), written);
    assert_eq!(view.accept_mission(MissionId(130)), Ok(0));
    writes_nav(&mut view, NavMode::Stellar(Some(StellarId(132))));
    view.refresh_status();
    assert_eq!(port_nav(&view), naming(hud::NAV_NO_DESTINATION));
    assert_eq!(
        view.take_mission_texts().len(),
        1,
        "the brief, noting Proxima"
    );
    assert_eq!(port_nav(&view), naming(hud::NAV_NO_DESTINATION));
    view.close_mission_text();
    assert_eq!(
        port_nav(&view),
        naming(hud::NAV_NO_DESTINATION),
        "the brief's close dropped with it"
    );
}

#[test]
fn an_accepts_two_texts_close_to_their_own_bars_and_the_dialog_redraws_after_the_last() {
    let mut view = docked();
    let queued = fuelled(200.0);
    view.sim.queue_event(brief(1, PROXIMA, queued));
    view.sim.queue_event(brief(2, PROXIMA, queued));
    let written = leaves(&view, |frame| {
        frame.nav_mode = PROXIMA;
        frame.hud.reserves = queued;
        frame.texts_queued = 2;
    });
    view.sim
        .queue_answer_showing(Answer::Accepted(Ok(0)), written);
    assert_eq!(view.accept_mission(MissionId(131)), Ok(0));
    assert_eq!(
        view.take_mission_texts().len(),
        2,
        "the BriefText, then the LoadCargText"
    );
    view.sim.frame_mut().hud.reserves = fuelled(100.0);
    view.refresh_frame();
    view.close_mission_text();
    assert_eq!(
        view.reserves_shown(&view.frame_now()),
        queued,
        "the brief as it noted the bar, and no redraw of the dialog's yet"
    );
    assert_eq!(port_nav(&view), naming("Proxima"), "OnAccept's M");
    view.close_mission_text();
    assert_eq!(
        retaken(&view),
        WHOLE_BAR,
        "the load's text, then the computer's redraw"
    );
    assert_eq!(view.reserves_shown(&view.frame_now()).fuel.now, 100.0);
    assert!(view.bar_steps.is_empty());
}

/// [`docked_upgrading`], its bar held stale, with an `OnAbort` or an
/// `OnFailure` of `H130 M131` (`changes`) or `H129 M131` queued on its
/// flight as the session would raise it: the view, and the frame the
/// command leaves.
fn scripted_docked(changes: bool) -> (Faked, FlightFrame) {
    let mut view = docked_upgrading();
    let government = if changes { UNDER_129 } else { None };
    view.sim.queue_event(load(government, EARTH, full(), 0));
    let after = leaves(&view, |frame| {
        frame.nav_mode = PROXIMA;
        frame.hud.interface_government = government;
    });
    stale(&mut view);
    (view, after)
}

#[test]
fn a_docked_on_abort_intf_change_redraws_the_whole_bar_at_the_operator() {
    for changes in [true, false] {
        let (mut view, after) = scripted_docked(changes);
        view.sim
            .queue_answer_showing(Answer::Aborted(Ok(())), after);
        assert_eq!(view.abort_mission(0), Ok(()));
        assert_eq!(sent(&view), [FlightCommand::AbortMission(0)]);
        if changes {
            assert_eq!(
                retaken(&view),
                [true, false, true, true],
                "the whole bar, the nav from before the M"
            );
            assert_eq!(port_nav(&view), naming("Earth"));
        } else {
            assert_eq!(retaken(&view), [false; 4], "held");
        }
    }
}

// The mission info and its key.

#[test]
fn the_mission_info_its_rows_and_an_abort_go_through_the_flight() {
    let mut view = docked();
    assert_eq!(view.mission_info().rows, []);
    view.sim.port_mut().mission_info = Some(info());
    assert_eq!(view.mission_info(), info());
    view.sim
        .queue_answer(Answer::InfoRow(Some("Quick.".to_owned())));
    assert_eq!(view.show_info_row(0).as_deref(), Some("Quick."));
    view.sim.queue_answer(Answer::InfoRow(None));
    assert_eq!(view.show_info_row(1), None, "a free slot");
    assert_eq!(view.show_info_row(0), None, "refused");
    view.sim.queue_answer(Answer::Aborted(Ok(())));
    assert_eq!(view.abort_mission(0), Ok(()));
    assert_eq!(
        view.abort_mission(0),
        Err(AbortError::NotActive),
        "refused: a flight that never started has none active"
    );
    assert_eq!(
        sent(&view),
        [
            FlightCommand::ShowInfoRow(0),
            FlightCommand::ShowInfoRow(1),
            FlightCommand::ShowInfoRow(0),
            FlightCommand::AbortMission(0),
            FlightCommand::AbortMission(0),
        ]
    );
}

#[test]
fn i_with_no_mission_says_what_the_flight_said_after_its_beep() {
    let mut view = faked(in_sol());
    view.sim
        .queue_event(FlightEvent::Sound(nova_sim::SimSound::Refused));
    view.sim
        .queue_event(FlightEvent::Message(SimMessage::NoActiveMissions));
    view.sim
        .queue_answer(Answer::MissionInfo(MissionInfoPress::NoActiveMissions));
    view.ask_mission_info();
    assert_eq!(sent_but_lines(&view), [FlightCommand::PressMissionInfo]);
    assert_eq!(view.take_mission_info_request(), None);
    assert_eq!(
        view.drained_sounds(),
        [Sound::Sim(nova_sim::SimSound::Refused)],
        "_beepSnd[3], snd 153"
    );
    faked_lasts(
        &mut view,
        NO_ACTIVE_MISSIONS,
        SimMessage::NoActiveMissions.ticks(),
    );
}

/// The plan review's correction 2 (2026-10-27-1918-581e): a flight that
/// refuses the press, one that never started, says #354 itself, with no
/// session to beep, as a failed session did.
#[test]
fn i_in_a_flight_that_never_started_says_no_active_missions_without_a_beep() {
    let mut view = faked(FlightFrame::default());
    view.ask_mission_info();
    assert_eq!(view.take_mission_info_request(), None);
    assert_eq!(view.drained_sounds(), []);
    faked_lasts(&mut view, NO_ACTIVE_MISSIONS, MESSAGE_TICKS);
}

#[test]
fn i_the_flight_refuses_itself_does_nothing_and_says_nothing() {
    let mut view = faked(in_sol());
    view.sim
        .queue_answer(Answer::MissionInfo(MissionInfoPress::Refused));
    view.held.insert(Key::Up);
    view.ask_mission_info();
    assert_eq!(view.take_mission_info_request(), None);
    assert_eq!(view.message(), None);
    assert!(view.held.contains(&Key::Up), "the flight keys kept");
}

#[test]
fn i_with_an_active_mission_asks_for_the_info_screen_once_letting_go_of_the_keys() {
    let mut view = faked(in_sol());
    view.sim
        .queue_answer(Answer::MissionInfo(MissionInfoPress::Opens(info())));
    view.held.insert(Key::Up);
    view.ask_mission_info();
    assert_eq!(view.take_mission_info_request(), Some(info()));
    assert_eq!(view.take_mission_info_request(), None, "once");
    assert_eq!(view.message(), None);
    assert!(view.held.is_empty(), "the flight keys let go");
}

// A person's offer answered over flight.

/// A person's offer of mission 130, its text "Help?".
fn offer() -> OfferPrompt {
    OfferPrompt {
        mission: MissionId(130),
        text: "Help?".to_owned(),
        labels: offer_labels("", "", true),
        graphic: None,
        movie: None,
    }
}

#[test]
fn a_persons_offer_is_taken_once() {
    let mut view = faked(in_sol());
    assert_eq!(view.take_offer(), None);
    view.pending_offer = Some(offer());
    assert_eq!(view.take_offer(), Some(offer()));
    assert_eq!(view.take_offer(), None, "once");
}

/// How a person's offer is answered over flight: accepted or refused in
/// the dialog with a refuse button, or accepted in the one-button
/// dialog.
const FLIGHT_ANSWERS: [(&str, bool, bool); 3] = [
    ("an accept", false, false),
    ("a refusal", false, true),
    ("a one-button accept", true, false),
];

/// A view over a flight in Sol, the HUD's cadence read as `cadence`
/// says, its bar drawn, with an offer of mission 130 (refusable unless
/// `unrefusable`) taken, and the answer the flight gives queued: its
/// script's tank and its text, and the Moon (129) selected after it.
fn offered_in_flight(cadence: RuleSource, unrefusable: bool, refuse: bool) -> Faked {
    let frame = FlightFrame {
        nav_mode: NavMode::Stellar(Some(StellarId(129))),
        ..in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)])
    };
    let mut frame = frame;
    frame.hud.reserves = full();
    let mut view = docked_on(catalog(), frame, cadence);
    if !unrefusable {
        view.sim.port_mut().refusable = vec![MissionId(130)];
    }
    view.pending_offer = Some(offer());
    view.take_offer().expect("offered");
    let tanked = Reserves {
        fuel: Gauge::full(350.0),
        ..full()
    };
    view.sim.queue_event(brief(1, view.nav_mode(), full()));
    let after = leaves(&view, |frame| {
        frame.hud.reserves = tanked;
        frame.texts_queued = 1;
    });
    let answer = if refuse {
        Answer::RefuseShown(Ok(REFUSED))
    } else {
        Answer::Accepted(Ok(0))
    };
    view.sim.queue_answer_showing(answer, after);
    view
}

/// Answers `view`'s offer, refusing it if `refuse`.
fn answer_offer(view: &mut Faked, refuse: bool, what: &str) {
    if refuse {
        assert!(view.refuse_mission(MissionId(130)).is_ok(), "{what}");
    } else {
        assert_eq!(view.accept_mission(MissionId(130)), Ok(0), "{what}");
    }
}

#[test]
fn an_offer_answered_over_flight_shows_its_text_over_the_pre_answer_bar() {
    for (what, unrefusable, refuse) in FLIGHT_ANSWERS {
        let mut view = offered_in_flight(RuleSource::Engine, unrefusable, refuse);
        let before = view.reserves();
        let nav = view.nav_shown(&view.frame_now());
        answer_offer(&mut view, refuse, what);
        assert_eq!(
            view.reserves().fuel.max,
            before.fuel.max + 100.0,
            "{what}: the flight's tank"
        );
        // A selection the bar hasn't drawn.
        view.sim.frame_mut().nav_mode = EARTH;
        view.refresh_frame();
        assert_ne!(view.nav_now(), nav, "{what}: Earth selected");
        assert_eq!(view.take_mission_texts().len(), 1, "{what}: its text");
        assert_eq!(
            view.reserves_shown(&view.frame_now()),
            before,
            "{what}: under the text"
        );
        assert_eq!(
            view.nav_shown(&view.frame_now()),
            nav,
            "{what}: the nav held too"
        );
        view.close_mission_text();
        assert_eq!(
            view.reserves_shown(&view.frame_now()),
            view.reserves(),
            "{what}: live once it closed"
        );
        assert_eq!(
            view.nav_shown(&view.frame_now()),
            view.nav_now(),
            "{what}: nav live too"
        );
        assert!(view.bar_steps.is_empty(), "{what}: let go");
    }
}

#[test]
fn under_the_bible_cadence_flights_gauges_stay_live_under_an_offers_text() {
    for (what, unrefusable, refuse) in FLIGHT_ANSWERS {
        let mut view = offered_in_flight(RuleSource::Bible, unrefusable, refuse);
        answer_offer(&mut view, refuse, what);
        assert_eq!(view.take_mission_texts().len(), 1, "{what}: its text");
        assert_eq!(
            view.reserves_shown(&view.frame_now()),
            view.reserves(),
            "{what}: live"
        );
    }
}

#[test]
fn a_flight_that_never_started_closes_and_reloads_nothing_but_sends_each() {
    let mut view = faked(FlightFrame::default());
    view.close_outfitter();
    view.close_hire();
    view.reload_offer_desc(MissionId(142));
    assert_eq!(
        sent(&view),
        [
            FlightCommand::CloseOutfitter,
            FlightCommand::CloseHire,
            FlightCommand::ReloadOfferDesc(MissionId(142)),
        ]
    );
}
#[test]
fn a_docked_jettisons_on_failure_intf_change_redraws_the_whole_bar_at_the_operator() {
    for changes in [true, false] {
        let (mut view, after) = scripted_docked(changes);
        view.sim.queue_answer_showing(Answer::Done, after);
        view.jettison_all();
        assert_eq!(
            sent_but_lines(&view),
            [FlightCommand::Jettison { all: true }]
        );
        if changes {
            assert_eq!(retaken(&view), [true, false, true, true]);
        } else {
            assert_eq!(retaken(&view), [false; 4], "held");
        }
    }
}

#[test]
fn the_offer_names_are_the_flights() {
    use crate::spaceport::OfferNames;
    let mut view = faked(in_sol());
    assert_eq!(
        OfferNames::munged(&view, MissionId(130), "Run <PAY>"),
        None,
        "nothing offered"
    );
    view.sim.port_mut().offer_texts = vec![(MissionId(130), "Run 1,000".to_owned())];
    assert_eq!(
        OfferNames::munged(&view, MissionId(130), "Run <PAY>").as_deref(),
        Some("Run 1,000"),
        "the flight's munge"
    );
    assert_eq!(
        OfferNames::munged(&view, MissionId(999), "Run"),
        None,
        "not offered"
    );
}
