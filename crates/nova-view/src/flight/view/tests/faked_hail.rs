//! The hail's, the comm dialog's and the escort dialog's component tests
//! on [`FakeFlight`]
//! (rdm:phase/view-sim-seam/phase-10-comm-escorts-boarding-on-port): the
//! commands Y and the dialogs' presses send, what the view holds for the
//! router from the flight's answers, and what it says. How a ship
//! answers is nova-sim's (`session::hail`, `session::person_offers`,
//! `session::escort_dialog`). Kept apart from `tests/mod.rs`, which is
//! large.

use super::faked_travel::{faked_lasts, sent_but_lines};
use super::*;
use nova_sim::escort_dialog::{EscortButton, EscortDialogView, UpgradeLine};
use nova_sim::flight_frame::FlightEvent;
use nova_sim::mission::screens::offer_labels;
use nova_sim::planet_comm::PlanetCommView;
use nova_sim::{Haggle, HailView, Hailed, OfferPrompt, SimSound, VenueOffer};

/// A view over a flight in Sol, the player alone.
fn sol() -> Faked {
    faked(in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)]))
}

/// The comm dialog of a hail of NPC 0, a ship 129, saying `reply`.
pub(super) fn hail_view(reply: &str) -> HailView {
    HailView {
        npc: NpcId(0),
        ship: ShipId(129),
        first: None,
        reply: reply.to_owned(),
        comm_name: "Trader".to_owned(),
        govt_name: None,
        hostile: false,
        options: Vec::new(),
        asking: None,
        pay_me: true,
        escort: None,
        portrait: None,
    }
}

/// The escort dialog of a captured ship 130, marked as `mark` says.
fn escort_view(mark: Option<nova_sim::fleet::EscortMark>) -> EscortDialogView {
    EscortDialogView {
        ship: ShipId(130),
        hired: false,
        comm_name: "Escort".to_owned(),
        subtitle: None,
        upgrade: UpgradeLine::Cannot,
        sale: None,
        pay: None,
        can_upgrade: false,
        can_sell: true,
        mark,
    }
}

/// Mission 130's offer, its text `text`.
pub(super) fn prompt(text: &str) -> OfferPrompt {
    OfferPrompt {
        mission: MissionId(130),
        text: text.to_owned(),
        labels: offer_labels("", "", true),
        graphic: None,
        movie: None,
    }
}

/// Holds Up, as the flight keys are held, and presses Y.
fn hail_thrusting(view: &mut Faked) {
    view.held.insert(Key::Up);
    view.hail();
}

// The hail.

#[test]
fn y_sends_the_hail_and_holds_the_comm_dialog_answered_once_letting_go_of_the_keys() {
    let mut view = sol();
    assert_eq!(view.take_hail(), None);
    view.sim
        .queue_answer(Answer::Hailed(Hailed::Comm(hail_view("Channel open."))));
    hail_thrusting(&mut view);
    assert_eq!(
        sent_but_lines(&view),
        [FlightCommand::Hail { option: false }]
    );
    assert_eq!(view.take_hail(), Some(hail_view("Channel open.")));
    assert_eq!(view.take_hail(), None, "once");
    assert_eq!(view.message(), None);
    assert!(view.held.is_empty(), "the flight keys let go");
    assert_eq!(HAIL_KEY, Key::Char('y'));
}

#[test]
fn the_comm_dialogs_contents_are_the_flights_hail_under_way() {
    let mut view = sol();
    assert_eq!(view.hailing(), None, "nothing answered");
    view.sim
        .queue_screen(ScreenAnswer::Hail(Box::new(hail_view("Channel open."))));
    assert_eq!(view.hailing(), Some(hail_view("Channel open.")));
    assert_eq!(view.sim.screens_asked(), [ScreenAsk::Hail, ScreenAsk::Hail]);
}

#[test]
fn alt_y_hails_with_option_held() {
    let mut view = sol();
    view.held.insert(Key::Alt);
    view.hail();
    assert_eq!(
        sent_but_lines(&view),
        [FlightCommand::Hail { option: true }]
    );
}

#[test]
fn a_hail_refused_says_why_in_the_originals_words_and_beeps_as_drained() {
    let mut view = sol();
    view.sim
        .queue_answer(Answer::HailRefused(HailRefusal::NoResponse));
    view.sim.queue_event(FlightEvent::Sound(SimSound::Refused));
    view.hail();
    assert_eq!(view.take_hail(), None);
    assert_eq!(view.drained_sounds(), [Sound::Sim(SimSound::Refused)]);
    faked_lasts(&mut view, NO_RESPONSE, 240);
    view.sim
        .queue_answer(Answer::HailRefused(HailRefusal::InHyperspace));
    view.hail();
    assert_eq!(
        view.message(),
        Some("Unable to send hail - target ship is entering hyperspace.")
    );
    let mut alone = sol();
    alone
        .sim
        .queue_answer(Answer::HailRefused(HailRefusal::NoTarget));
    alone.hail();
    assert_eq!(alone.message(), None, "no target: nothing said");
    assert_eq!(
        hail_refusal_message(HailRefusal::NoResponse),
        Some("No response.")
    );
    assert_eq!(hail_refusal_message(HailRefusal::NoTarget), None);
}

#[test]
fn a_refused_hail_opens_nothing_and_says_nothing() {
    let mut view = sol();
    view.sim.queue_answer(Answer::Refused);
    hail_thrusting(&mut view);
    assert_eq!(view.take_hail(), None);
    assert_eq!(view.take_offer(), None);
    assert_eq!(view.take_escort_dialog(), None);
    assert_eq!(view.take_planet_comm(), None);
    assert_eq!(view.message(), None);
    assert!(view.held.contains(&Key::Up), "the keys held");
}

// Y's press through the input.

#[test]
fn y_answered_with_a_dialog_leaves_no_key_held_as_the_dialog_takes_its_release() {
    for answer in [
        Hailed::Comm(hail_view("Channel open.")),
        Hailed::Stellar(planet_view(128)),
        Hailed::Escort(escort_view(None)),
        Hailed::Offer(VenueOffer::Prompt(prompt("Help?"))),
    ] {
        let mut view = sol();
        view.sim.queue_answer(Answer::Hailed(answer.clone()));
        view.input(&key(HAIL_KEY, true));
        assert!(!view.holds_keys(), "{answer:?}");
    }
}

#[test]
fn y_answered_with_a_dialog_lets_go_of_the_mouse_button_too() {
    for answer in [
        Hailed::Comm(hail_view("Channel open.")),
        Hailed::Stellar(planet_view(128)),
        Hailed::Escort(escort_view(None)),
        Hailed::Offer(VenueOffer::Prompt(prompt("Help?"))),
    ] {
        let mut view = super::faked_screen::in_flight();
        super::key_fades::button_down(&mut view);
        view.sim.queue_answer(Answer::Hailed(answer));
        view.input(&key(HAIL_KEY, true));
        view.take_hail();
        view.take_escort_dialog();
        view.take_planet_comm();
        view.take_offer();
        view.hang_up();
        super::key_fades::fades_with_no_release(&mut view);
    }
}

#[test]
fn y_refused_stays_held_until_its_release() {
    let mut view = sol();
    view.input(&key(HAIL_KEY, true));
    assert!(view.holds_keys(), "no answer but a refusal");
    view.input(&key(HAIL_KEY, false));
    assert!(!view.holds_keys(), "its release");

    let mut view = sol();
    view.change_sim(|sim| sim.refuse(PlayerCommand::Hail));
    view.input(&key(HAIL_KEY, true));
    assert!(view.holds_keys(), "refused by the gate");
    view.input(&key(HAIL_KEY, false));
    assert!(!view.holds_keys(), "its release");
}

// The planet comm dialog.

#[test]
fn y_on_a_stellar_that_answers_holds_its_planet_comm_dialog_once_letting_go_of_the_keys() {
    let mut view = sol();
    view.sim
        .queue_answer(Answer::Hailed(Hailed::Stellar(planet_view(128))));
    hail_thrusting(&mut view);
    assert_eq!(view.take_planet_comm(), Some(planet_view(128)));
    assert_eq!(view.take_planet_comm(), None, "once");
    assert_eq!(view.take_hail(), None, "no comm dialog");
    assert!(view.held.is_empty(), "the flight keys let go");
}

#[test]
fn the_planet_comm_dialog_draws_the_frame_the_flight_shows_of_its_stellar() {
    let view = sol();
    let scene = view.scene().expect("laid out");
    let first = scene.stellars().first().expect("a stellar").id;
    assert_eq!(view.stellar_frame(first), scene.shown_frame(first));
    assert!(view.stellar_frame(first).is_some(), "drawn");
    assert_eq!(view.stellar_frame(StellarId(999)), None, "not shown");
}

#[test]
fn the_planet_comm_dialogs_presses_and_haggle_send_their_commands() {
    use nova_sim::planet_comm::PlanetCommItem;
    let mut view = sol();
    let asked = PlanetCommView {
        asking: Some(3000),
        ..planet_view(128)
    };
    view.sim
        .queue_answer(Answer::PlanetComm(Some(asked.clone())));
    assert_eq!(view.press_planet_comm(PlanetCommItem::Second), Some(asked));
    view.sim
        .queue_answer(Answer::PlanetComm(Some(planet_view(128))));
    assert_eq!(
        view.haggle_bribe(Haggle::LowerPrice),
        Some(planet_view(128))
    );
    view.sim.queue_answer(Answer::PlanetComm(None));
    assert_eq!(view.haggle_bribe(Haggle::Accept), None, "paid: closed");
    assert_eq!(
        view.press_planet_comm(PlanetCommItem::Close),
        None,
        "refused"
    );
    assert_eq!(
        sent_but_lines(&view),
        [
            FlightCommand::PressPlanetComm(PlanetCommItem::Second),
            FlightCommand::HaggleBribe(Haggle::LowerPrice),
            FlightCommand::HaggleBribe(Haggle::Accept),
            FlightCommand::PressPlanetComm(PlanetCommItem::Close),
        ]
    );
}

#[test]
fn the_planet_comm_dialogs_close_redraws_the_radar_and_a_press_that_keeps_it_does_not() {
    use nova_sim::planet_comm::PlanetCommItem;
    let mut view = super::faked_hud::barred(catalog(), among(Vec::new()));
    super::faked_hud::hud_tick(&mut view);
    let stale = RadarScene::default();
    view.radar_scene = Some(stale.clone());
    view.sim
        .queue_answer(Answer::PlanetComm(Some(planet_view(128))));
    view.press_planet_comm(PlanetCommItem::Second);
    view.press_planet_comm(PlanetCommItem::Close);
    assert_eq!(*view.radar_shown(&view.frame), stale, "open, then refused");
    view.sim.queue_answer(Answer::PlanetComm(None));
    view.press_planet_comm(PlanetCommItem::Close);
    assert_eq!(
        *view.radar_shown(&view.frame),
        view.radar_now(&view.frame),
        "closed: as _RedrawStatus(1) @0x97b3d"
    );
    view.radar_scene = Some(stale.clone());
    view.sim
        .queue_answer(Answer::PlanetComm(Some(planet_view(128))));
    view.haggle_bribe(Haggle::LowerPrice);
    assert_eq!(*view.radar_shown(&view.frame), stale, "the haggle goes on");
    view.sim.queue_answer(Answer::PlanetComm(None));
    view.haggle_bribe(Haggle::Accept);
    assert_eq!(
        *view.radar_shown(&view.frame),
        view.radar_now(&view.frame),
        "a bribe paid closes it"
    );
}

#[test]
fn a_hail_the_flight_does_not_take_sends_nothing() {
    let mut view = sol();
    view.change_sim(|sim| sim.refuse(PlayerCommand::Hail));
    view.sim
        .queue_answer(Answer::Hailed(Hailed::Comm(hail_view("Channel open."))));
    view.hail();
    assert_eq!(sent(&view), [], "the gate refused it");
    assert_eq!(view.take_hail(), None);
}

#[test]
fn a_hail_dialog_keeps_the_message() {
    let mut view = sol();
    view.say_for(MISSION_FAILED, 240);
    view.sim
        .queue_answer(Answer::Hailed(Hailed::Comm(hail_view("Channel open."))));
    view.hail();
    view.take_hail().expect("hailed");
    assert_eq!(view.message(), Some(MISSION_FAILED), "under the dialog");
    view.hang_up();
    faked_lasts(&mut view, MISSION_FAILED, 240);
}

#[test]
fn answers_haggles_and_hanging_up_send_their_commands_and_give_back_the_answers() {
    let mut view = sol();
    view.sim
        .queue_answer(Answer::Hail(Some(hail_view("Thank you."))));
    assert_eq!(view.answer(1), Some(hail_view("Thank you.")));
    view.sim.queue_answer(Answer::Hail(None));
    assert_eq!(view.haggle(Haggle::Accept), None, "no price asked");
    view.sim
        .queue_answer(Answer::Hail(Some(hail_view("Lowered."))));
    assert_eq!(view.haggle(Haggle::LowerPrice), Some(hail_view("Lowered.")));
    view.hang_up();
    assert_eq!(view.answer(0), None, "refused");
    assert_eq!(view.haggle(Haggle::Accept), None, "refused");
    assert_eq!(
        sent_but_lines(&view),
        [
            FlightCommand::AnswerHail(1),
            FlightCommand::Haggle(Haggle::Accept),
            FlightCommand::Haggle(Haggle::LowerPrice),
            FlightCommand::HangUp,
            FlightCommand::AnswerHail(0),
            FlightCommand::Haggle(Haggle::Accept),
        ]
    );
}

#[test]
fn hanging_up_redraws_the_radar_at_once_leaving_the_timer_and_blink() {
    let mut view = super::faked_hud::barred(catalog(), among(Vec::new()));
    for _ in 0..3 {
        super::faked_hud::hud_tick(&mut view);
    }
    let stale = RadarScene::default();
    assert_ne!(view.radar_now(&view.frame), stale);
    view.radar_scene = Some(stale);
    let cadence = view.hud_cadence;
    view.hang_up();
    assert_eq!(view.hud_cadence, cadence, "neither timed nor flipped");
    assert_eq!(
        *view.radar_shown(&view.frame),
        view.radar_now(&view.frame),
        "as _RedrawStatus"
    );
}

// The offer a person makes in place of the comm dialog: held for the
// router, which takes it (`FlightView::take_offer`, phase 11's, still the
// session's view's), here read off the view.

#[test]
fn y_on_a_person_offering_its_mission_holds_the_offer_once_letting_go_of_the_keys() {
    let mut view = sol();
    assert_eq!(view.take_offer(), None);
    view.sim
        .queue_answer(Answer::Hailed(Hailed::Offer(VenueOffer::Prompt(prompt(
            "Ace needs help.",
        )))));
    hail_thrusting(&mut view);
    assert_eq!(view.take_hail(), None, "no comm dialog");
    assert_eq!(view.take_offer(), Some(prompt("Ace needs help.")));
    assert_eq!(view.take_offer(), None, "once");
    assert!(view.held.is_empty(), "the flight keys let go");
}

#[test]
fn an_offer_keeps_the_message() {
    let mut view = sol();
    view.say_for(MISSION_FAILED, 240);
    view.sim
        .queue_answer(Answer::Hailed(Hailed::Offer(VenueOffer::Prompt(prompt(
            "Help?",
        )))));
    view.hail();
    view.take_offer().expect("offered");
    assert_eq!(view.message(), Some(MISSION_FAILED), "under the dialog");
}

#[test]
fn an_offer_accepted_at_once_shows_nothing() {
    let mut view = sol();
    view.sim
        .queue_answer(Answer::Hailed(Hailed::Offer(VenueOffer::Accepted {
            mission: MissionId(130),
            accepted: Ok(0),
        })));
    hail_thrusting(&mut view);
    assert_eq!(view.take_offer(), None);
    assert_eq!(view.take_hail(), None);
    assert!(view.held.contains(&Key::Up), "nothing to let go for");
}

// The escort dialog.

#[test]
fn y_on_a_fleet_escort_holds_its_dialog_once_and_its_presses_send_their_buttons() {
    let mut view = sol();
    assert_eq!(view.take_escort_dialog(), None);
    view.sim
        .queue_answer(Answer::Hailed(Hailed::Escort(escort_view(None))));
    hail_thrusting(&mut view);
    assert_eq!(view.take_escort_dialog(), Some(escort_view(None)));
    assert_eq!(view.take_escort_dialog(), None, "once");
    assert_eq!(view.take_hail(), None, "no comm dialog");
    assert!(view.held.is_empty(), "the flight keys let go");
    let sold = escort_view(Some(nova_sim::fleet::EscortMark::Sell));
    view.sim
        .queue_answer(Answer::EscortDialog(Some(sold.clone())));
    assert_eq!(view.press_escort(EscortButton::Sell), Some(sold));
    view.sim.queue_answer(Answer::EscortDialog(None));
    assert_eq!(view.press_escort(EscortButton::Close), None);
    assert_eq!(view.press_escort(EscortButton::Sell), None, "refused");
    assert_eq!(
        sent_but_lines(&view)[1..],
        [
            FlightCommand::PressEscort(EscortButton::Sell),
            FlightCommand::PressEscort(EscortButton::Close),
            FlightCommand::PressEscort(EscortButton::Sell),
        ]
    );
}

#[test]
fn the_escort_dialogs_close_redraws_the_radar_and_a_refused_press_does_not() {
    let mut view = super::faked_hud::barred(catalog(), among(Vec::new()));
    super::faked_hud::hud_tick(&mut view);
    let stale = RadarScene::default();
    view.radar_scene = Some(stale.clone());
    view.sim
        .queue_answer(Answer::EscortDialog(Some(escort_view(None))));
    view.press_escort(EscortButton::Sell);
    assert_eq!(*view.radar_shown(&view.frame), stale, "still open");
    view.press_escort(EscortButton::Close);
    assert_eq!(*view.radar_shown(&view.frame), stale, "refused: no close");
    view.sim.queue_answer(Answer::EscortDialog(None));
    view.press_escort(EscortButton::Close);
    assert_eq!(
        *view.radar_shown(&view.frame),
        view.radar_now(&view.frame),
        "closed: as _RedrawStatus"
    );
}
