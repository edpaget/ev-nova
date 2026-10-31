use super::*;
use crate::spaceport::outfitter::OutfitterScreen;

// Which of the spaceport's layers opened through `FlushEvents(0xff7f, 0)`.

#[test]
fn the_bare_spaceport_has_none() {
    assert_eq!(earth().flushing_layers(), 0);
}

#[test]
fn the_exchange_and_its_quantity_flush_nothing() {
    // `_DoTradeDialog` and `_DoQuantityDialog` never call `FlushEvents`.
    let mut view = trading(Ok(trade_template()));
    click_item(&mut view, 7);
    assert!(view.open_trade().is_some());
    assert_eq!(view.flushing_layers(), 0);
    view.input(&key(Key::Alt));
    view.input(&key(Key::Char('b')));
    assert!(view.open_trade().and_then(TradeScreen::quantity).is_some());
    assert_eq!(view.flushing_layers(), 0);
}

#[test]
fn the_outfitter_flushes_and_its_alert_does_but_not_its_quantity() {
    let mut view = outfitting(Ok(outfit_template()));
    click_item(&mut view, 8);
    assert_eq!(view.flushing_layers(), 1, "_DoOutfitDialog @0x5bd51");
    click_outfit_cell(&mut view, 0);
    view.input(&key(Key::Alt));
    let buy = outfit_item(&view, 7);
    click(&mut view, buy);
    assert!(
        view.open_outfitter()
            .and_then(OutfitterScreen::quantity)
            .is_some()
    );
    assert_eq!(view.flushing_layers(), 1, "_DoQuantityDialog has no flush");
    view.input(&key(Key::Escape));
    view.alert_outfit(&alert(), None);
    assert_eq!(view.flushing_layers(), 2, "_ShowTextDialog @0x5ae67");
}

#[test]
fn the_shipyard_flushes_and_so_do_its_panel_and_its_name_prompt() {
    let view = in_the_shipyards_panel();
    assert_eq!(view.flushing_layers(), 2, "_DoShipyardDescription @0x5b7e3");
    let mut view = shipbuying(Ok(shipyard_template()));
    click_item(&mut view, 9);
    assert_eq!(view.flushing_layers(), 1, "_DoShipyardDialog @0x5e85a");
    view.input(&key(Key::Right));
    let buy = shipyard_item(&view, 1);
    click(&mut view, buy);
    view.open_ship_naming(&naming());
    assert_eq!(view.flushing_layers(), 2, "_EVTextInputDialog @0x55429");
}

#[test]
fn the_bar_flushes_and_so_do_its_news_hire_screen_and_panel() {
    assert_eq!(in_the_bar().flushing_layers(), 1, "_DoBarDialog @0x4908a");
    assert_eq!(in_the_news().flushing_layers(), 2, "_DoNewsDialog @0x4802b");
    assert_eq!(
        hiring(drinking_with_info()).flushing_layers(),
        2,
        "the hire screen is _DoShipyardDialog"
    );
    assert_eq!(in_the_hire_panel().flushing_layers(), 3);
}

#[test]
fn the_mission_computer_mission_info_and_the_player_info_each_flush() {
    let mut view = informed(missioned());
    click_item(&mut view, 10);
    view.open_mission_computer(Ok(offers()));
    assert_eq!(view.flushing_layers(), 1, "_DoMissionDialog @0xa3a1c");
    view.open_mission_info(actives(1));
    assert_eq!(view.flushing_layers(), 2, "_DoMissionInfoDialog @0x9e287");
    view.open_player_info(pilot_info());
    assert_eq!(view.flushing_layers(), 3, "_DoPlayerInfoDialog @0x5a544");
}

#[test]
fn the_player_infos_question_flushes() {
    let mut view = informed(missioned());
    view.open_player_info(pilot_info());
    assert_eq!(view.flushing_layers(), 1);
    let at = |view: &SpaceportView, button| {
        view.player_info()
            .and_then(|screen| screen.button_bounds(button))
            .expect("laid out")
            .center()
    };
    let cargo = at(&view, PlayerInfoButton::Tab(Tab::Cargo));
    click(&mut view, cargo);
    let jettison = at(&view, PlayerInfoButton::Jettison);
    click(&mut view, jettison);
    assert_eq!(view.flushing_layers(), 2, "_EVYesNoDialog @0x55689");
}

#[test]
fn an_offer_flushes_over_the_spaceport_and_over_a_venue() {
    let mut view = venues();
    let _ = view.take_offer_request();
    view.open_offer(offered());
    assert!(view.offer_dialog().is_some());
    assert_eq!(view.flushing_layers(), 1, "_OfferOneMission @0xa2caa");
    view.input(&key(Key::Enter));
    assert_eq!(view.flushing_layers(), 0, "answered, it goes");

    let mut view = drinking_with_offers();
    click_item(&mut view, 11);
    view.open_offer(offered());
    assert_eq!(view.flushing_layers(), 2, "over the bar");
}

#[test]
fn a_mission_text_is_a_stage_and_no_layer() {
    let mut view = missioned();
    let text = |text: &str| MissionText {
        kind: MissionTextKind::Comp,
        mission: MissionId(130),
        text: text.to_owned(),
        graphic: None,
        movie: None,
        map: None,
    };
    view.show_mission_texts(vec![text("One."), text("Two.")]);
    assert_eq!(view.flushing_layers(), 0);
    assert_eq!(view.text_stages(), 2);
    view.input(&key(Key::Enter));
    assert_eq!(view.text_stages(), 1, "the next text in front");
    view.input(&key(Key::Enter));
    assert_eq!(view.text_stages(), 0);
}
