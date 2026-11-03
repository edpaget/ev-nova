//! Wiring: the spaceport's readout of the keys a held press holds back
//! ([`Screen::holds_back_keys`]) reads its mission texts, the offer over
//! the spaceport, and the screen open over it, whose own readouts are
//! pinned in their modules.

use super::*;
use crate::spaceport::offer::fixture::accept_click_holding;

fn escape() -> Input {
    key(Key::Escape)
}

fn left(pressed: bool, at: Point) -> Input {
    Input::PointerButton {
        button: MouseButton::Left,
        pressed,
        at,
    }
}

#[test]
fn a_click_held_on_the_spaceports_offer_holds_back_keys_until_the_answers_leftover_runs() {
    let mut view = venues();
    assert_eq!(view.take_offer_request(), Some(SPACEPORT));
    view.open_offer(offered());
    assert!(!view.holds_back_keys(), "before the press");
    let [press, held, release] = accept_click_holding(view.offer_dialog().expect("open"), escape());
    view.input(&press);
    assert!(view.holds_back_keys(), "during the press");
    view.input(&held);
    view.input(&release);
    assert!(view.take_offer_choice().is_some(), "accepted");
    assert!(view.holds_back_keys(), "the Escape waits for the next tick");
    view.tick(Duration::ZERO);
    assert!(!view.holds_back_keys(), "it has run");
}

#[test]
fn a_click_held_on_an_offer_over_each_venue_holds_back_keys() {
    for (venue, item) in [(BAR, 11), (TRADE_CENTER, 7), (OUTFITTER, 8), (SHIPYARD, 9)] {
        let mut view = venues();
        view.take_offer_request();
        click_item(&mut view, item);
        serve_openings(&mut view);
        view.tick(FIRST_OFFER_DELAY);
        assert_eq!(view.take_offer_request(), Some(venue));
        view.open_offer(offered());
        let [press, ..] = accept_click_holding(view.offer_dialog().expect("open"), escape());
        assert!(!view.holds_back_keys(), "{venue}: before the press");
        view.input(&press);
        assert!(view.holds_back_keys(), "{venue}: during the press");
    }
}

#[test]
fn a_click_held_on_a_mission_texts_done_holds_back_keys() {
    let mut view = missioned();
    view.show_mission_texts(vec![MissionText {
        kind: MissionTextKind::Brief,
        mission: MissionId(130),
        text: "Go.".to_owned(),
        graphic: None,
        movie: None,
        map: None,
    }]);
    let done = view
        .mission_text()
        .and_then(|text| text.dialog().item_bounds(crate::ui::desc::DONE_ITEM))
        .expect("Done")
        .center();
    assert!(!view.holds_back_keys(), "before the press");
    view.input(&left(true, done));
    assert!(view.holds_back_keys(), "during the press");
    view.input(&left(false, done));
    assert!(!view.holds_back_keys(), "after the release");
}

#[test]
fn a_click_held_on_the_outfitters_alert_holds_back_keys() {
    let mut view = outfitting(Ok(outfit_template()));
    click_item(&mut view, 8);
    view.alert_outfit(&alert(), None);
    let done = view
        .open_outfitter()
        .and_then(OutfitterScreen::alert_dialog)
        .and_then(|alert| alert.dialog().item_bounds(crate::ui::desc::DONE_ITEM))
        .expect("Done")
        .center();
    assert!(!view.holds_back_keys(), "before the press");
    view.input(&left(true, done));
    assert!(view.holds_back_keys(), "during the press");
}
