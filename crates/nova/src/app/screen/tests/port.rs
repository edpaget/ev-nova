use super::*;

// Landing.

#[test]
fn l_over_the_planet_shows_its_spaceport() {
    let mut screen = with_dialogs(data());
    assert!(screen.spaceport_view().is_none());
    land(&mut screen);
    let port = spaceport(&screen);
    assert_eq!(port.stellar(), nova_sim::StellarId(128));
    assert_eq!(port.problem(), None);
    assert_eq!(
        port.offered(),
        [nova_sim::Service::Bar, nova_sim::Service::MissionBbs]
    );
    assert_eq!(
        flight(&screen).session().expect("flying").landed(),
        Some(nova_sim::StellarId(128))
    );
    // Drawn alone: no hint.
    assert_eq!(drawn(&screen), drawn(spaceport(&screen)));
}

#[test]
fn landing_lets_go_of_the_flight_keys() {
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    screen.input(&key(Key::Up, true));
    land_twice(&mut screen);
    assert_eq!(screen.showing(), Showing::Spaceport);
    // Up's release goes to the spaceport, and Leave takes off.
    screen.input(&key(Key::Up, false));
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::Flight);
    screen.tick(TICK * 5);
    assert_eq!(ship(&screen), nova_sim::ShipState::default(), "no thrust");
}

#[test]
fn leave_takes_off_back_into_flight_at_the_planet() {
    let mut screen = with_dialogs(data());
    land(&mut screen);
    let leave = spaceport(&screen)
        .dialog()
        .expect("laid out")
        .item_bounds(LEAVE_ITEM)
        .expect("Leave")
        .center();
    for pressed in [true, false] {
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: leave,
        });
    }
    assert_eq!(screen.showing(), Showing::Flight);
    assert!(screen.spaceport_view().is_none());
    assert_eq!(flight(&screen).session().expect("flying").landed(), None);
    assert_eq!(ship(&screen), nova_sim::ShipState::default());
    // It flies again.
    screen.input(&key(Key::Up, true));
    screen.tick(TICK * 3);
    assert!(ship(&screen).position.y < 0.0);
}

#[test]
fn escape_in_the_spaceport_leaves_and_never_quits() {
    let mut screen = with_dialogs(data());
    land(&mut screen);
    assert_eq!(screen.input(&held(Key::Escape)), ScreenAction::None);
    assert_eq!(screen.showing(), Showing::Spaceport);
    assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::None);
    assert_eq!(screen.showing(), Showing::Flight);
    assert_eq!(screen.input(&key(Key::Escape, false)), ScreenAction::None);
    assert_eq!(screen.showing(), Showing::Flight);
}

#[test]
fn tab_f_and_i_do_nothing_in_the_spaceport() {
    let mut screen = with_dialogs(data());
    land(&mut screen);
    for k in [Key::Tab, Key::Char('f'), Key::Char('i'), LAND] {
        assert_eq!(screen.input(&key(k, true)), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::Spaceport, "{k:?}");
    }
    assert!(screen.about().is_none());
}

#[test]
fn without_dialogs_the_spaceport_says_why_and_escape_still_leaves() {
    let mut screen = AppScreen::new(data());
    land(&mut screen);
    assert_eq!(spaceport(&screen).problem(), Some(NO_INTERFACE));
    let texts: Vec<String> = drawn(&screen)
        .iter()
        .filter_map(|c| match c {
            DrawCommand::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        texts,
        [
            "Cannot show the spaceport: no interface file. Press Return or Escape to take off."
                .to_owned()
        ]
    );
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::Flight);
}

#[test]
fn a_missing_spaceport_dialog_says_why() {
    let mut screen = AppScreen::new(data()).with_dialogs(Rc::new(NoDialogs), Rc::new(MonoMetrics));
    land(&mut screen);
    assert_eq!(spaceport(&screen).problem(), Some("no DLOG 1000"));
}

#[test]
fn only_the_spaceport_ticks_and_takes_pointer_and_key_resets() {
    let mut screen = with_dialogs(data());
    land(&mut screen);
    let docked = ship(&screen);
    screen.tick(TICK * 5);
    assert_eq!(ship(&screen), docked);
    let leave = spaceport(&screen)
        .dialog()
        .expect("laid out")
        .item_bounds(LEAVE_ITEM)
        .expect("Leave")
        .center();
    let button = |pressed| Input::PointerButton {
        button: MouseButton::Left,
        pressed,
        at: leave,
    };
    screen.input(&button(true));
    screen.cancel_pointer();
    screen.release_keys();
    screen.input(&button(false));
    assert_eq!(
        screen.showing(),
        Showing::Spaceport,
        "the click was abandoned"
    );
}

#[test]
fn a_refused_landing_stays_in_flight() {
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    screen.input(&key(Key::Up, true));
    screen.tick(TICK * 30);
    land_twice(&mut screen);
    assert_eq!(screen.showing(), Showing::Flight);
    assert!(screen.spaceport_view().is_none());
    assert!(flight(&screen).message().is_some());
}

// The Trade Center.

#[test]
fn the_trade_center_opens_the_stellars_exchange() {
    let store = MemoryPilots::new();
    let mut screen = landed_trader(&store);
    assert!(
        spaceport(&screen)
            .offered()
            .contains(&nova_sim::Service::TradeCenter)
    );
    click_port_item(&mut screen, 7);
    let open = spaceport(&screen).open_trade().expect("trading");
    assert_eq!(open.problem(), None);
    assert_eq!(open.market().row(FOOD).map(|row| row.price), Some(75));
    assert_eq!((open.market().cash, open.market().free), (1000, 10));
    assert_eq!(screen.showing(), Showing::Spaceport);
    assert_eq!(drawn(&screen), drawn(spaceport(&screen)));
}

#[test]
fn an_order_trades_refreshes_the_exchange_and_saves_the_pilot() {
    let store = MemoryPilots::new();
    let mut screen = landed_trader(&store);
    click_port_item(&mut screen, 7);
    let writes = store.writes();
    screen.input(&key(Key::Char('b'), true));
    assert_eq!(pilot(&screen).cash(), 250, "a click buys up to 10 tons");
    assert_eq!(pilot(&screen).held(FOOD), 10);
    let open = spaceport(&screen).open_trade().expect("trading");
    assert_eq!(open.market().row(FOOD).map(|row| row.held), Some(10));
    assert_eq!(open.market().cash, 250);
    assert_eq!(store.writes(), writes + 1, "saved after the input");
    assert_eq!(saved(&store, "Ada").held(FOOD), 10);
    // A max sale and a max buy, the quantity dialog's default each
    // confirmed, then a click on Buy with no space.
    screen.input(&key(Key::Alt, true));
    screen.input(&key(Key::Char('s'), true));
    assert_eq!(pilot(&screen).held(FOOD), 10, "the dialog asks first");
    screen.input(&key(Key::Enter, true));
    assert_eq!(pilot(&screen).held(FOOD), 0, "everything held");
    assert_eq!(pilot(&screen).cash(), 1000);
    screen.input(&key(Key::Char('b'), true));
    screen.input(&key(Key::Enter, true));
    screen.input(&key(Key::Alt, false));
    assert_eq!(pilot(&screen).held(FOOD), 10, "the hold is full");
    assert_eq!(pilot(&screen).cash(), 250);
    let writes = store.writes();
    let buy = spaceport(&screen)
        .open_trade()
        .and_then(|open| open.dialog())
        .and_then(|dialog| dialog.item_bounds(BUY_ITEM))
        .expect("Buy")
        .center();
    for pressed in [true, false] {
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: buy,
        });
    }
    assert_eq!(pilot(&screen).held(FOOD), 10, "greyed");
    assert_eq!(store.writes(), writes, "nothing to save");
    // Escape closes the exchange; the spaceport stays.
    screen.input(&key(Key::Escape, true));
    assert!(spaceport(&screen).open_trade().is_none());
    assert_eq!(screen.showing(), Showing::Spaceport);
}

#[test]
fn alt_b_asks_for_a_count_and_the_count_typed_is_bought_and_saved() {
    let store = MemoryPilots::new();
    let mut screen = landed_trader(&store);
    click_port_item(&mut screen, 7);
    let writes = store.writes();
    screen.input(&key(Key::Alt, true));
    screen.input(&key(Key::Char('b'), true));
    screen.input(&Input::Text('b'));
    screen.input(&key(Key::Alt, false));
    let asking = spaceport(&screen)
        .open_trade()
        .and_then(TradeScreen::quantity)
        .expect("asking");
    assert_eq!(asking.field().text(), "10", "min(1000 / 75, 10 free)");
    assert_eq!(store.writes(), writes, "nothing traded yet");
    screen.input(&key(Key::Char('3'), true));
    screen.input(&Input::Text('3'));
    screen.input(&key(Key::Enter, true));
    assert_eq!(pilot(&screen).held(FOOD), 3);
    assert_eq!(pilot(&screen).cash(), 1000 - 3 * 75);
    assert_eq!(store.writes(), writes + 1, "saved after the input");
    assert_eq!(saved(&store, "Ada").held(FOOD), 3);
    assert!(spaceport(&screen).open_trade().is_some(), "still trading");
}

#[test]
fn by_the_other_count_reading_alt_b_buys_the_most_at_once() {
    let store = MemoryPilots::new();
    let mut screen = landed_trader_by(&store, RuleSource::Bible);
    click_port_item(&mut screen, 7);
    screen.input(&key(Key::Alt, true));
    screen.input(&key(Key::Char('b'), true));
    assert_eq!(pilot(&screen).held(FOOD), 10);
}

#[test]
fn without_the_trade_dialog_the_trade_center_says_why() {
    let store = MemoryPilots::new();
    let mut screen = AppScreen::new(trading_data())
        .with_dialogs(Rc::new(Dialogs), Rc::new(MonoMetrics))
        .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
        .appeared();
    create(&mut screen, "Ada");
    land_twice(&mut screen);
    click_port_item(&mut screen, 7);
    let open = spaceport(&screen).open_trade().expect("trading");
    assert_eq!(open.problem(), Some("no DLOG 1001"));
}

#[test]
fn a_stellar_without_a_trade_center_offers_no_exchange() {
    let mut screen = with_dialogs(data());
    land(&mut screen);
    click_port_item(&mut screen, 7);
    assert!(spaceport(&screen).open_trade().is_none());
    assert!(spaceport(&screen).open_service().is_none());
}

// The Outfitter.

/// The router over [`outfitting_data`] with the outfit dialog, keeping
/// pilots in `store`, flying a new pilot named Ada, landed.
fn landed_outfitter(store: &MemoryPilots) -> AppScreen {
    landed_outfitter_over(store, outfitting_data())
}

const BOOSTER: OutfitId = OutfitId(128);

/// [`data`] with the status bar ([`barred`]), its planet an
/// outfitter selling nothing, as no `oütf` exists.
fn empty_outfitter_data() -> Rc<GameData> {
    let mut built = barred(game_data_fork(false, false, false, 100, b"", b"", None)).build();
    let stellar = built.layout.data_entry(Stellar::TYPE, 128) + 4;
    built.bytes[stellar + 0x09] |= 0x04;
    loaded_bytes(built.bytes)
}

#[test]
fn an_empty_outfitter_shows_its_text_and_restores_nothing() {
    let store = MemoryPilots::new();
    let mut screen = landed_outfitter_over(&store, empty_outfitter_data());
    assert!(
        spaceport(&screen)
            .offered()
            .contains(&nova_sim::Service::Outfitter)
    );
    let bar = bar_drawn(&screen);
    click_port_item(&mut screen, 8);
    assert!(
        spaceport(&screen).open_outfitter().is_none(),
        "never opened"
    );
    assert_eq!(
        spaceport(&screen)
            .mission_text()
            .map(|text| text.lines().join(" ")),
        Some(nova_view::spaceport::outfitter::NO_ITEMS.to_owned())
    );
    assert_eq!(bar_drawn(&screen), bar, "the status bar as it was");
    screen.input(&key(Key::Enter, true));
    assert!(
        spaceport(&screen).mission_text().is_none(),
        "the text closed"
    );
    assert_eq!(screen.showing(), Showing::Spaceport);
}

/// [`data`] with the status bar ([`barred`]), its planet, of tech
/// level 1, a shipyard ([`shipbuying_data`]) and an outfitter selling
/// nothing, as no `oütf` exists. Ship 129 carries nothing, is listed
/// only while `!b7` (`Flags3` 0x0100), and its `OnPurchase` `M129 b7` moves the pilot to
/// Beta Prime and empties the shipyard.
fn empty_shops_after_a_move_data() -> Rc<GameData> {
    let mut built = barred(game_data_fork(true, false, true, 100, b"", b"", None)).build();
    let stellar = built.layout.data_entry(Stellar::TYPE, 128) + 4;
    built.bytes[stellar + 0x09] |= 0x04;
    built.put_u16(stellar + 0x0C, 1);
    let ship = built.layout.data_entry(Ship::TYPE, 129) + 4;
    built.put_u16(ship + 0x4E, 0xFFFF);
    built.put_u16(ship + 0x56, 0);
    built.put_u16(ship + 0x726, 0x0100);
    built.bytes[ship + 0x6C..ship + 0x6F].copy_from_slice(b"!b7");
    built.bytes[ship + 0x26A..ship + 0x271].copy_from_slice(b"M129 b7");
    loaded_bytes(built.bytes)
}

/// The stellar named in the status bar's nav area.
fn nav_named(screen: &AppScreen) -> Vec<String> {
    bar_drawn(screen)
        .into_iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, .. } if text.ends_with(" Prime") => Some(text),
            _ => None,
        })
        .collect()
}

/// The shipyard's restore after a landed `M` leaves the nav marked
/// but undrawn; an empty shipyard draws nothing after it, while an
/// empty outfitter's `_DrawStatus` (@0x5ff09-0x5ff10) draws it.
#[test]
fn an_empty_outfitter_draws_the_nav_a_shipyard_left_marked_and_an_empty_shipyard_does_not() {
    let store = MemoryPilots::new();
    let mut screen = landed_shipyard_of(empty_shops_after_a_move_data(), &store);
    assert_eq!(nav_named(&screen), ["Alpha Prime"], "landed");
    click_port_item(&mut screen, 9);
    select_first_ship(&mut screen);
    press_b(&mut screen);
    screen.input(&key(Key::Enter, true));
    assert_eq!(pilot(&screen).ship(), ShipId(129), "bought");
    assert_eq!(pilot(&screen).stellar(), Some(nova_sim::StellarId(129)));
    assert!(
        spaceport(&screen).open_shipyard().is_none(),
        "the purchase closes it"
    );
    assert_eq!(nav_named(&screen), ["Beta Prime"], "the move held");

    click_port_item(&mut screen, 9);
    assert!(spaceport(&screen).open_shipyard().is_none(), "empty");
    assert_eq!(
        spaceport(&screen)
            .mission_text()
            .map(|text| text.lines().join(" ")),
        Some(nova_view::spaceport::shipyard::NO_SHIPS.to_owned())
    );
    screen.input(&key(Key::Enter, true));
    assert_eq!(
        nav_named(&screen),
        ["Beta Prime"],
        "no draw after the shipyard"
    );

    click_port_item(&mut screen, 8);
    assert!(
        spaceport(&screen).open_outfitter().is_none(),
        "never opened"
    );
    assert_eq!(nav_named(&screen), ["Alpha Prime"], "the restore drawn");
}

#[test]
fn the_outfitter_opens_the_stellars_outfitter() {
    let store = MemoryPilots::new();
    let mut screen = landed_outfitter(&store);
    assert!(
        spaceport(&screen)
            .offered()
            .contains(&nova_sim::Service::Outfitter)
    );
    click_port_item(&mut screen, 8);
    let open = spaceport(&screen).open_outfitter().expect("outfitting");
    assert_eq!(open.problem(), None);
    let booster = open.outfitter().row(BOOSTER).expect("listed");
    assert_eq!((booster.price, booster.mass), (500, 2));
    assert_eq!(
        (open.outfitter().cash, open.outfitter().free_mass),
        (1000, 10)
    );
    assert_eq!(screen.showing(), Showing::Spaceport);
    assert_eq!(drawn(&screen), drawn(spaceport(&screen)));
}

/// [`landed_outfitter`], with Nova's rules as `rulebook` chooses.
fn landed_outfitter_by(store: &MemoryPilots, rulebook: &Rulebook) -> AppScreen {
    let mut screen = AppScreen::new(outfitting_data())
        .with_rulebook(rulebook)
        .with_dialogs(Rc::new(OutfitDialogs), Rc::new(MonoMetrics))
        .with_pilots(Some(keeper(store)), Rc::new(MonoMetrics))
        .appeared();
    create(&mut screen, "Ada");
    land_twice(&mut screen);
    assert_eq!(screen.showing(), Showing::Spaceport);
    screen
}

#[test]
fn an_empty_outfitter_cell_clicked_follows_the_grid_selection_rule() {
    let other = Rulebook::default().with_override(RuleKey::GridSelection, RuleSource::Bible);
    for (rulebook, kept) in [(Rulebook::default(), false), (other, true)] {
        let store = MemoryPilots::new();
        let mut screen = landed_outfitter_by(&store, &rulebook);
        click_port_item(&mut screen, 8);
        if !kept {
            // It opens with nothing selected by the engine.
            screen.input(&key(Key::Right, true));
        }
        let open = spaceport(&screen).open_outfitter().expect("outfitting");
        let first = open.selected_outfit();
        assert!(first.is_some(), "the first cell");
        let empty = open
            .cell_bounds(open.outfitter().rows.len())
            .expect("a cell")
            .center();
        click_at(&mut screen, empty);
        let open = spaceport(&screen).open_outfitter().expect("outfitting");
        assert_eq!(
            open.selected_outfit(),
            first.filter(|_| kept),
            "kept: {kept}"
        );
    }
}

#[test]
fn the_outfitter_opens_by_the_grid_selection_rule() {
    let other = Rulebook::default().with_override(RuleKey::GridSelection, RuleSource::Bible);
    for (rulebook, engine) in [(Rulebook::default(), true), (other, false)] {
        let store = MemoryPilots::new();
        let mut screen = landed_outfitter_by(&store, &rulebook);
        click_port_item(&mut screen, 8);
        let open = spaceport(&screen).open_outfitter().expect("outfitting");
        let first = open.outfitter().rows[0].id;
        assert_eq!(
            open.selected_outfit(),
            (!engine).then_some(first),
            "engine: {engine}"
        );
    }
}

/// [`landed_shipyard`], with Nova's rules as `rulebook` chooses.
fn landed_shipyard_by(store: &MemoryPilots, rulebook: &Rulebook) -> AppScreen {
    let mut screen = AppScreen::new(shipbuying_data())
        .with_rulebook(rulebook)
        .with_dialogs(Rc::new(ShipyardDialogs), Rc::new(MonoMetrics))
        .with_pilots(Some(keeper(store)), Rc::new(MonoMetrics))
        .appeared();
    create(&mut screen, "Ada");
    land_twice(&mut screen);
    assert_eq!(screen.showing(), Showing::Spaceport);
    screen
}

/// The rulebook's `dialog_focus_ring` reaches the spaceport and the
/// shops opened over it.
#[test]
fn the_dialog_focus_ring_reaches_the_spaceport_and_its_shipyard() {
    use nova_view::ui::{Dialog, FocusRing};
    for (rulebook, ring) in [
        (Rulebook::default(), FocusRing::Tab),
        (
            Rulebook::default().with_override(RuleKey::DialogFocusRing, RuleSource::Engine),
            FocusRing::Off,
        ),
    ] {
        let store = MemoryPilots::new();
        let mut screen = landed_shipyard_by(&store, &rulebook);
        assert_eq!(
            spaceport(&screen).dialog().map(Dialog::focus_ring),
            Some(ring)
        );
        click_port_item(&mut screen, 9);
        assert_eq!(
            spaceport(&screen)
                .open_shipyard()
                .and_then(|open| open.dialog())
                .map(Dialog::focus_ring),
            Some(ring),
            "{ring:?}"
        );
    }
}

#[test]
fn an_empty_shipyard_cell_clicked_follows_the_grid_selection_rule() {
    let other = Rulebook::default().with_override(RuleKey::GridSelection, RuleSource::Bible);
    for (rulebook, engine) in [(Rulebook::default(), true), (other, false)] {
        let store = MemoryPilots::new();
        let mut screen = landed_shipyard_by(&store, &rulebook);
        click_port_item(&mut screen, 9);
        let open = spaceport(&screen).open_shipyard().expect("shipbuying");
        let opened = (!engine).then_some(0);
        assert_eq!(open.selected(), opened, "as it opens, engine: {engine}");
        let empty = open
            .cell_bounds(open.shipyard().rows.len())
            .expect("a cell")
            .center();
        if engine {
            select_first_ship(&mut screen);
        }
        assert_eq!(
            spaceport(&screen)
                .open_shipyard()
                .and_then(ShipyardScreen::selected),
            Some(0)
        );
        click_at(&mut screen, empty);
        let open = spaceport(&screen).open_shipyard().expect("shipbuying");
        assert_eq!(open.selected(), opened, "an empty cell, engine: {engine}");
    }
}

#[test]
fn the_hire_screen_opens_by_the_grid_selection_rule() {
    let other = Rulebook::default().with_override(RuleKey::GridSelection, RuleSource::Bible);
    for (rulebook, opened) in [(Rulebook::default(), None), (other, Some(0))] {
        // Ship 128 always for hire (`HireRandom` 100).
        let mut built = game_data_fork(false, false, false, 100, b"", b"", None).build();
        let first = built.layout.data_entry(Ship::TYPE, 128) + 4;
        built.put_u16(first + 0x38A, 100);
        let mut screen = AppScreen::new(loaded_bytes(built.bytes))
            .with_rules(rulebook)
            .with_dialogs(Rc::new(PastTheHoldsDialogs), Rc::new(MonoMetrics));
        land(&mut screen);
        click_port_item(&mut screen, 11);
        screen.input(&key(Key::Char('h'), true));
        let hire = spaceport(&screen)
            .open_bar()
            .and_then(BarScreen::hire_screen)
            .expect("the hire screen");
        assert_eq!(hire.selected(), opened, "{rulebook:?}");
    }
}

const LICENSE: OutfitId = OutfitId(130);
const LICENSED: OutfitId = OutfitId(131);

#[test]
fn an_outfit_purchase_that_relists_resets_the_outfitters_selection() {
    let store = MemoryPilots::new();
    let mut screen = landed_outfitter_over(&store, licensing_data());
    click_port_item(&mut screen, 8);
    select_first_outfit(&mut screen);
    let open = spaceport(&screen).open_outfitter().expect("outfitting");
    assert_eq!(open.outfitter().row(LICENSED), None, "hidden");
    screen.input(&key(Key::Right, true));
    let open = spaceport(&screen).open_outfitter().expect("outfitting");
    assert_eq!(open.selected_outfit(), Some(LICENSE));
    screen.input(&key(Key::Char('b'), true));
    assert_eq!(pilot(&screen).owned(LICENSE), 1);
    let open = spaceport(&screen).open_outfitter().expect("outfitting");
    assert!(open.outfitter().row(LICENSED).is_some(), "relisted");
    assert_eq!(open.selected(), None);
    assert_eq!(open.selected_outfit(), Some(BOOSTER));
    screen.input(&key(Key::Char('b'), true));
    assert_eq!(pilot(&screen).owned(BOOSTER), 1, "Buy answers for 128");
    assert_eq!(pilot(&screen).owned(LICENSE), 1);
    assert_eq!(pilot(&screen).cash(), 400);
}

#[test]
fn an_outfit_purchase_that_leaves_nothing_listed_closes_the_outfitter() {
    let store = MemoryPilots::new();
    let mut screen = landed_outfitter_over(&store, self_hiding_data());
    click_port_item(&mut screen, 8);
    select_first_outfit(&mut screen);
    let open = spaceport(&screen).open_outfitter().expect("outfitting");
    assert_eq!(open.outfitter().rows.len(), 1);
    screen.input(&key(Key::Char('b'), true));
    assert_eq!(pilot(&screen).cash(), 500, "bought");
    assert_eq!(pilot(&screen).owned(BOOSTER), 0, "taken away");
    assert!(spaceport(&screen).open_outfitter().is_none(), "closed");
    assert_eq!(screen.showing(), Showing::Spaceport);
    assert!(!spaceport(&screen).left());
}

/// [`outfitting_data`], the booster made a cargo pod: 10 tons more
/// cargo space (`ModType` 2) for 100 credits.
fn cargo_pod_data() -> Rc<GameData> {
    let mut built = game_data_fork(true, true, false, 100, b"", b"", None).build();
    let pod = built.layout.data_entry(Outfit::TYPE, 128) + 4;
    built.put_u16(pod + 0x06, 2);
    built.put_u16(pod + 0x08, 10);
    built.bytes[pod + 0x0E..pod + 0x12].copy_from_slice(&100_i32.to_be_bytes());
    loaded_bytes(built.bytes)
}

/// The router over [`cargo_pod_data`], landed, by the other reading of
/// [`RuleKey::OutfitSaleCargo`]: Ada buys the pod (20 tons), then 12
/// tons of food with the 900 credits left, a lot of 10 and then 2,
/// and sells the pod in the
/// outfitter, which stays open, keeping the 12 tons in her 10.
fn twelve_tons_in_ten(store: &MemoryPilots) -> AppScreen {
    let rulebook = Rulebook::default().with_override(RuleKey::OutfitSaleCargo, RuleSource::Bible);
    let mut screen = AppScreen::new(cargo_pod_data())
        .with_rulebook(&rulebook)
        .with_dialogs(Rc::new(ShopDialogs), Rc::new(MonoMetrics))
        .with_pilots(Some(keeper(store)), Rc::new(MonoMetrics))
        .appeared();
    create(&mut screen, "Ada");
    land_twice(&mut screen);
    click_port_item(&mut screen, 8);
    select_first_outfit(&mut screen);
    screen.input(&key(Key::Char('b'), true));
    screen.input(&key(Key::Escape, true));
    assert_eq!(pilot(&screen).owned(BOOSTER), 1, "the pod bought");
    click_port_item(&mut screen, 7);
    screen.input(&key(Key::Char('b'), true));
    screen.input(&key(Key::Char('b'), true));
    screen.input(&key(Key::Escape, true));
    assert_eq!(pilot(&screen).held(FOOD), 12);
    click_port_item(&mut screen, 8);
    select_first_outfit(&mut screen);
    screen.input(&key(Key::Char('s'), true));
    assert_eq!(pilot(&screen).owned(BOOSTER), 0, "the pod sold");
    assert!(spaceport(&screen).open_outfitter().is_some());
    assert_eq!(pilot(&screen).held(FOOD), 12, "kept past the holds");
    screen
}

#[test]
fn closing_the_outfitter_trims_the_cargo_past_the_holds() {
    let mut screen = twelve_tons_in_ten(&MemoryPilots::new());
    screen.input(&key(Key::Escape, true));
    assert!(spaceport(&screen).open_outfitter().is_none());
    assert_eq!(pilot(&screen).held(FOOD), 10, "@0x5db10");
    click_port_item(&mut screen, 7);
    let open = spaceport(&screen).open_trade().expect("trading");
    assert_eq!(
        open.market().row(FOOD).map(|row| row.held),
        Some(10),
        "the shops shown afresh"
    );
}

// The resets past the holds: the player info opening and the hire
// screen closing.

/// The mission cargo [`past_the_holds`] carries: 9 tons, never cut.
const MISSION_TONS: u32 = 9;

/// [`cargo_pod_data`] whose outfitter offers mission 130 (see
/// [`venue_mission`]) carrying [`MISSION_TONS`] of cargo aboard from
/// its accept, and whose first ship class is for hire in the bar
/// every day (`HireRandom` 100).
fn past_the_holds_data() -> Rc<GameData> {
    let mut mission = venue_mission(OUTFITTER);
    let tons = i16::try_from(MISSION_TONS).expect("a few tons");
    mission[0x10..0x12].copy_from_slice(&0_i16.to_be_bytes());
    mission[0x12..0x14].copy_from_slice(&tons.to_be_bytes());
    let fork = game_data_fork(true, true, false, 100, b"", b"", None)
        .resource(Mission::TYPE, 130, Some(b"Cargo job"), &mission)
        .resource(Desc::TYPE, 130 + 3872, Some(b"Offer"), &desc_of("Job 130."));
    let mut built = fork.build();
    let pod = built.layout.data_entry(Outfit::TYPE, 128) + 4;
    built.put_u16(pod + 0x06, 2);
    built.put_u16(pod + 0x08, 10);
    built.bytes[pod + 0x0E..pod + 0x12].copy_from_slice(&100_i32.to_be_bytes());
    let first = built.layout.data_entry(Ship::TYPE, 128) + 4;
    built.put_u16(first + 0x38A, 100);
    loaded_bytes(built.bytes)
}

/// The router over [`past_the_holds_data`], landed, by the other
/// reading of [`RuleKey::OutfitSaleCargo`]: Ada accepts the
/// outfitter's mission ([`MISSION_TONS`] aboard), buys the pod (20
/// tons), then 10 tons of food, and sells the pod, keeping 19 tons in
/// her 10. The outfitter's close trims the food to trunc(10 x 10 /
/// 19) = 5, leaving 14 tons in 10, as the mission cargo is never cut:
/// each reset after it trims the food again.
fn past_the_holds(store: &MemoryPilots) -> AppScreen {
    let rulebook = Rulebook::default().with_override(RuleKey::OutfitSaleCargo, RuleSource::Bible);
    let mut screen = AppScreen::new(past_the_holds_data())
        .with_rulebook(&rulebook)
        .with_dialogs(Rc::new(PastTheHoldsDialogs), Rc::new(MonoMetrics))
        .with_pilots(Some(keeper(store)), Rc::new(MonoMetrics))
        .appeared();
    create(&mut screen, "Ada");
    land_twice(&mut screen);
    click_port_item(&mut screen, 8);
    let open = spaceport(&screen).open_outfitter().expect("outfitting");
    assert_eq!(offered_mission(open.offer_dialog()), Some(MissionId(130)));
    screen.input(&key(Key::Enter, true));
    assert_eq!(active(&screen), [MissionId(130)]);
    assert_eq!(pilot(&screen).total_cargo(), i64::from(MISSION_TONS));
    select_first_outfit(&mut screen);
    screen.input(&key(Key::Char('b'), true));
    screen.input(&key(Key::Escape, true));
    assert_eq!(pilot(&screen).owned(BOOSTER), 1, "the pod bought");
    click_port_item(&mut screen, 7);
    screen.input(&key(Key::Char('b'), true));
    screen.input(&key(Key::Escape, true));
    assert_eq!(pilot(&screen).held(FOOD), 10);
    click_port_item(&mut screen, 8);
    select_first_outfit(&mut screen);
    assert!(
        spaceport(&screen)
            .open_outfitter()
            .is_some_and(|open| open.offer_dialog().is_none()),
        "no offer over the sale"
    );
    screen.input(&key(Key::Char('s'), true));
    assert_eq!(pilot(&screen).owned(BOOSTER), 0, "the pod sold");
    assert_eq!(pilot(&screen).held(FOOD), 10, "kept past the holds");
    screen.input(&key(Key::Escape, true));
    assert!(spaceport(&screen).open_outfitter().is_none());
    assert_eq!(pilot(&screen).held(FOOD), 5, "the close's trim");
    screen
}

/// The trade center's food row as the spaceport shows it.
fn food_shown(screen: &AppScreen) -> Option<u32> {
    spaceport(screen)
        .open_trade()
        .and_then(|open| open.market().row(FOOD))
        .map(|row| row.held)
}

#[test]
fn p_over_the_trade_center_resets_and_shows_the_cargo_trimmed() {
    let mut screen = past_the_holds(&MemoryPilots::new());
    click_port_item(&mut screen, 7);
    assert_eq!(food_shown(&screen), Some(5));
    screen.input(&key(PLAYER_INFO_KEY, true));
    assert!(spaceport(&screen).player_info().is_some(), "open");
    assert_eq!(pilot(&screen).held(FOOD), 3, "trunc(5 x 10 / 14) @0x5a06a");
    assert_eq!(food_shown(&screen), Some(3), "the shops shown afresh");
    let shown = spaceport(&screen)
        .player_info()
        .and_then(|open| open.info().cargo.clone())
        .expect("cargo shown");
    // The cargo line counts every ton aboard: 3 of food and the
    // mission's 9, not the 14 before the reset.
    assert!(shown.contains("12"), "the info built after: {shown}");
}

#[test]
fn cancelling_the_hire_screen_resets_and_the_shops_show_it() {
    let mut screen = past_the_holds(&MemoryPilots::new());
    click_port_item(&mut screen, 11);
    assert!(spaceport(&screen).open_bar().is_some(), "in the bar");
    screen.input(&key(Key::Char('h'), true));
    assert!(
        spaceport(&screen)
            .open_bar()
            .and_then(BarScreen::hire_screen)
            .is_some(),
        "for hire"
    );
    assert_eq!(pilot(&screen).held(FOOD), 5, "opening resets nothing");
    screen.input(&key(Key::Escape, true));
    assert!(
        spaceport(&screen)
            .open_bar()
            .is_some_and(|bar| bar.hire_screen().is_none()),
        "cancelled back to the bar"
    );
    assert_eq!(pilot(&screen).held(FOOD), 3, "trunc(5 x 10 / 14) @0x49235");
    screen.input(&key(Key::Escape, true));
    assert!(spaceport(&screen).open_bar().is_none(), "left the bar");
    click_port_item(&mut screen, 7);
    assert_eq!(food_shown(&screen), Some(3), "the shops shown afresh");
}

/// [`data`] with system 128 linked to 129, landed at Alpha Prime's
/// bar, the bar open.
fn in_the_bar() -> AppScreen {
    in_the_bar_by(Rulebook::default())
}

/// [`in_the_bar`], its rules read as `rules` says.
pub(super) fn in_the_bar_by(rules: Rulebook) -> AppScreen {
    let mut built = game_data_fork(false, false, false, 100, b"", b"", None).build();
    let alpha = built.layout.data_entry(System::TYPE, 128) + 4;
    built.put_u16(alpha + 0x04, 129);
    let mut screen = AppScreen::new(loaded_bytes(built.bytes))
        .with_rules(rules)
        .with_dialogs(Rc::new(PastTheHoldsDialogs), Rc::new(MonoMetrics));
    land(&mut screen);
    click_port_item(&mut screen, 11);
    assert!(spaceport(&screen).open_bar().is_some(), "in the bar");
    screen
}

#[test]
fn the_bars_map_ticks_its_borders_and_saves_no_change() {
    use nova_sim::map_borders::Next;
    let mut screen = in_the_bar();
    screen.input(&key(MAP, true));
    screen.input(&key(MAP, false));
    assert_eq!(screen.showing(), Showing::FlightMap);
    let passes = || flight(&screen).course_map().border_passes().next();
    assert_eq!(passes(), Next::Unstarted, "the open's coarse pass");
    screen.tick(Duration::from_millis(16));
    let passes = flight(&screen).course_map().border_passes().next();
    assert_eq!(passes, Next::Idle, "the router ticks the flight under it");
    for k in [Key::Char('b'), MAP] {
        screen.input(&key(k, true));
        screen.input(&key(k, false));
    }
    assert!(!flight(&screen).map_open());
    assert!(!flight(&screen).map_borders_shown());
    assert_eq!(screen.take_map_borders_save(), None, "docked");
}

#[test]
fn find_on_the_bars_map_keeps_escape_and_command_comma_from_it() {
    let mut screen = in_the_bar();
    screen.input(&key(MAP, true));
    screen.input(&key(MAP, false));
    for pressed in [true, false] {
        screen.input(&key(Key::Char('f'), pressed));
    }
    assert!(flight(&screen).map_finding());
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::FlightMap, "Escape types nothing");
    command_comma(&mut screen);
    assert!(screen.preferences().is_none(), "no Preferences over Find");
    assert!(flight(&screen).map_finding());
    screen.input(&key(Key::Enter, true));
    assert!(!flight(&screen).map_finding());
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::Spaceport, "closed once Find is");
}

/// Where `system` is on flight's open map, scrolled well into item 3 with
/// the arrows first: the map opens at 0.5625 and shows little of the
/// galaxy.
pub(super) fn scrolled_to(screen: &mut AppScreen, system: SystemId) -> nova_view::Point {
    let shown = nova_view::galaxy::dialog::map_area().grown(-12.0);
    for _ in 0..200 {
        let map = flight(screen).course_map();
        let at = map
            .transform()
            .world_to_screen(map.model().system(system).expect("on the map").position());
        let arrow = if at.x < shown.min.x {
            Key::Left
        } else if at.x > shown.max.x {
            Key::Right
        } else if at.y < shown.min.y {
            Key::Up
        } else if at.y > shown.max.y {
            Key::Down
        } else {
            return at;
        };
        screen.input(&key(arrow, true));
        screen.input(&key(arrow, false));
    }
    panic!("{system:?} never scrolled into item 3");
}

#[test]
fn the_bars_map_opens_over_the_spaceport_and_its_close_puts_back_the_nav() {
    // The planner's click writes the nav in port, which the bar puts back.
    let mut screen =
        in_the_bar_by(Rulebook::default().with_override(RuleKey::MapPlotting, RuleSource::Bible));
    let nav = |screen: &AppScreen| {
        flight(screen)
            .session()
            .ok()
            .map(nova_sim::Session::nav_mode)
    };
    let landed = Some(nova_sim::NavMode::Stellar(Some(nova_sim::StellarId(128))));
    assert_eq!(nav(&screen), landed);
    screen.input(&key(MAP, true));
    assert_eq!(screen.showing(), Showing::FlightMap);
    assert!(spaceport(&screen).open_bar().is_some(), "the bar beneath");
    let mut list = DrawList::new();
    flight(&screen).draw_status_bar(&mut list);
    spaceport(&screen).draw(&mut list);
    flight(&screen).course_map().draw(&mut list);
    assert_eq!(
        drawn(&screen).iter().cloned().collect::<Vec<_>>(),
        list.iter().cloned().collect::<Vec<_>>(),
        "the status bar and the spaceport, the bar open, under the map"
    );
    screen.input(&key(MAP, false));
    let beta = scrolled_to(&mut screen, SystemId(129));
    let empty = nova_view::Point::new(beta.x - 40.0, beta.y);
    let left = |pressed| Input::PointerButton {
        button: MouseButton::Left,
        pressed,
        at: empty,
    };
    screen.input(&left(true));
    screen.cancel_pointer();
    screen.input(&left(false));
    assert_eq!(nav(&screen), landed, "a press on empty space plots nothing");
    for pressed in [true, false] {
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: beta,
        });
    }
    assert_eq!(
        nav(&screen),
        Some(nova_sim::NavMode::Hyperspace(Some(SystemId(129)))),
        "plotted and cued"
    );
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::Spaceport);
    assert!(spaceport(&screen).open_bar().is_some(), "still in the bar");
    assert_eq!(nav(&screen), landed, "the bar's nav put back");
    assert_eq!(
        flight(&screen)
            .session()
            .map(|session| session.course().to_vec()),
        Ok(vec![SystemId(129)]),
        "the course kept"
    );
    screen.input(&key(MAP, true));
    assert_eq!(screen.showing(), Showing::FlightMap, "again");
    screen.input(&key(MAP, false));
    screen.input(&key(MAP, true));
    assert_eq!(screen.showing(), Showing::Spaceport, "M closes it too");
}

/// The nav mode of `screen`'s session.
fn port_nav_mode(screen: &AppScreen) -> Option<nova_sim::NavMode> {
    flight(screen)
        .session()
        .ok()
        .map(nova_sim::Session::nav_mode)
}

/// The router over [`shipbuying_data`]'s shops, Alpha (128) linking
/// Beta (129) and back, the booster's `OnPurchase` `outfit_hook` and ship 129's
/// `ship_hook`, the planner's click writing the nav in port
/// (`MapPlotting`'s Bible reading), flying a new pilot named Ada,
/// landed.
fn landed_planning(outfit_hook: &[u8], ship_hook: &[u8]) -> AppScreen {
    let mut built = game_data_fork(true, true, true, 100, b"", outfit_hook, None).build();
    let alpha = built.layout.data_entry(System::TYPE, 128) + 4;
    built.put_u16(alpha + 0x04, 129);
    let beta = built.layout.data_entry(System::TYPE, 129) + 4;
    built.put_u16(beta + 0x04, 128);
    // `OnPurchase`, at 0x26A.
    let second = built.layout.data_entry(Ship::TYPE, 129) + 4 + 0x26A;
    built.bytes[second..second + ship_hook.len()].copy_from_slice(ship_hook);
    let mut screen = AppScreen::new(loaded_bytes(built.bytes))
        .with_rules(Rulebook::default().with_override(RuleKey::MapPlotting, RuleSource::Bible))
        .with_dialogs(Rc::new(ShipyardDialogs), Rc::new(MonoMetrics))
        .with_pilots(Some(keeper(&MemoryPilots::new())), Rc::new(MonoMetrics))
        .appeared();
    create(&mut screen, "Ada");
    land_twice(&mut screen);
    assert_eq!(screen.showing(), Showing::Spaceport);
    assert_eq!(port_nav_mode(&screen), Some(LANDED_NAV));
    screen
}

/// The nav a landing at Earth (128) shows.
const LANDED_NAV: nova_sim::NavMode = nova_sim::NavMode::Stellar(Some(nova_sim::StellarId(128)));

/// Opens the map from what `screen` shows with M, plots `system` on it
/// with the planner's click, which cues it, and closes it with
/// Escape, back to the spaceport: the nav after the close. A map that
/// put back no nav would keep the cue.
fn plot_on_a_port_screens_map(
    screen: &mut AppScreen,
    system: SystemId,
    what: &str,
) -> Option<nova_sim::NavMode> {
    screen.input(&key(MAP, true));
    screen.input(&key(MAP, false));
    assert_eq!(screen.showing(), Showing::FlightMap, "{what}");
    let at = scrolled_to(screen, system);
    for pressed in [true, false] {
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        });
    }
    assert_eq!(
        port_nav_mode(screen),
        Some(nova_sim::NavMode::Hyperspace(Some(system))),
        "{what}: plotted and cued"
    );
    screen.input(&key(Key::Escape, true));
    screen.input(&key(Key::Escape, false));
    assert_eq!(screen.showing(), Showing::Spaceport, "{what}: closed");
    port_nav_mode(screen)
}

#[test]
fn the_map_key_opens_the_map_over_the_spaceport_and_the_exchange_which_stay_open() {
    // The bare spaceport has no port dialog's save, so the exchange's
    // put-back would keep the cue, as would the bar's and none. Nothing
    // in the exchange writes the nav, so its save is the nav as its map
    // opens: the outfitter's test tells that put-back from the
    // spaceport's.
    let mut screen = landed_planning(b"", b"");
    assert_eq!(
        plot_on_a_port_screens_map(&mut screen, SystemId(129), "the spaceport"),
        Some(LANDED_NAV),
        "the nav as the map opened"
    );
    assert!(spaceport(&screen).open_trade().is_none());
    assert!(!spaceport(&screen).left(), "still docked");
    click_port_item(&mut screen, 7);
    assert!(spaceport(&screen).open_trade().is_some(), "trading");
    assert_eq!(
        plot_on_a_port_screens_map(&mut screen, SystemId(129), "the exchange"),
        Some(LANDED_NAV),
        "the exchange's save put back"
    );
    assert!(spaceport(&screen).open_trade().is_some(), "still trading");
}

#[test]
fn the_outfitters_map_puts_back_the_nav_the_outfitter_opened_with() {
    // A purchase's landed `M` writes the nav after the outfitter's save,
    // so the nav as the map opens isn't the save the outfitter puts
    // back.
    let mut screen = landed_planning(b"M129", b"");
    click_port_item(&mut screen, 8);
    select_first_outfit(&mut screen);
    assert!(spaceport(&screen).open_outfitter().is_some(), "outfitting");
    screen.input(&key(Key::Char('b'), true));
    screen.input(&key(Key::Char('b'), false));
    assert_ne!(
        port_nav_mode(&screen),
        Some(LANDED_NAV),
        "the purchase's M writes the nav"
    );
    assert_eq!(
        plot_on_a_port_screens_map(&mut screen, SystemId(128), "the outfitter"),
        Some(LANDED_NAV),
        "the outfitter's save put back"
    );
    assert!(
        spaceport(&screen).open_outfitter().is_some(),
        "still outfitting"
    );
}

#[test]
fn the_map_key_opens_the_map_over_the_shipyards_info_panel_which_stays_open() {
    // A purchase closes the shipyard, so its landed `M` can no longer
    // write the nav after the shipyard's save: the nav as each map opens
    // is the save the shipyard took as it opened.
    let mut screen = landed_planning(b"", b"");
    click_port_item(&mut screen, 9);
    select_first_ship(&mut screen);
    let bought = port_nav_mode(&screen);
    assert_eq!(bought, Some(LANDED_NAV));
    assert_eq!(
        plot_on_a_port_screens_map(&mut screen, SystemId(129), "the shipyard"),
        bought,
        "the shipyard: the nav as the map opened"
    );
    screen.input(&key(nova_view::spaceport::shipyard::INFO_KEY, true));
    assert_eq!(
        plot_on_a_port_screens_map(&mut screen, SystemId(129), "its Info panel"),
        bought,
        "the Info panel: the nav as the map opened"
    );
    assert!(
        spaceport(&screen)
            .open_shipyard()
            .and_then(ShipyardScreen::info_panel)
            .is_some(),
        "the panel still open"
    );
}

#[test]
fn a_purchase_closes_the_shipyard_restoring_its_nav_in_the_input_that_bought() {
    // The purchase's landed `M` writes the nav after the shipyard's save;
    // the close that follows restores the save before the next frame.
    let mut screen = landed_planning(b"", b"M129");
    click_port_item(&mut screen, 9);
    select_first_ship(&mut screen);
    press_b(&mut screen);
    screen.input(&key(Key::Enter, true));
    assert_eq!(pilot(&screen).ship(), ShipId(129), "bought");
    assert!(spaceport(&screen).open_shipyard().is_none(), "closed");
    assert_eq!(
        port_nav_mode(&screen),
        Some(LANDED_NAV),
        "the shipyard's close restored its save"
    );
}

#[test]
fn tab_on_the_bars_map_changes_nothing() {
    let mut screen = in_the_bar();
    let nav = |screen: &AppScreen| {
        flight(screen)
            .session()
            .ok()
            .map(nova_sim::Session::nav_mode)
    };
    let landed = nav(&screen);
    screen.input(&key(MAP, true));
    screen.input(&key(MAP, false));
    assert_eq!(screen.showing(), Showing::FlightMap);
    for k in [Key::Tab, Key::Char('\\')] {
        assert_eq!(screen.input(&key(k, true)), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::FlightMap, "{k:?}");
        assert_eq!(nav(&screen), landed, "{k:?}");
    }
    assert!(spaceport(&screen).open_bar().is_some(), "still in the bar");
}

#[test]
fn p_in_flight_resets_before_the_info_is_built() {
    let mut screen = past_the_holds(&MemoryPilots::new());
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::Flight, "taken off");
    past_the_takeoff_grace(&mut screen);
    let held = pilot(&screen).held(FOOD);
    let trimmed = held * 10 / (held + MISSION_TONS);
    assert!(trimmed < held, "{held} still past the holds");
    screen.input(&key(PLAYER_INFO_KEY, true));
    let shown = screen
        .flight_player_info()
        .and_then(|open| open.info().cargo.clone())
        .expect("open, showing the cargo");
    assert_eq!(pilot(&screen).held(FOOD), trimmed, "@0x5a06a");
    let total = (trimmed + MISSION_TONS).to_string();
    assert!(shown.contains(&total), "the info built after: {shown}");
}

#[test]
fn an_outfit_order_changes_the_pilot_refreshes_the_outfitter_and_saves() {
    let store = MemoryPilots::new();
    let mut screen = landed_outfitter(&store);
    click_port_item(&mut screen, 8);
    select_first_outfit(&mut screen);
    let speed = flight(&screen)
        .session()
        .expect("flying")
        .handling()
        .max_speed;
    let writes = store.writes();
    screen.input(&key(Key::Char('b'), true));
    assert_eq!(pilot(&screen).owned(BOOSTER), 1);
    assert_eq!(pilot(&screen).cash(), 500);
    let open = spaceport(&screen).open_outfitter().expect("outfitting");
    assert_eq!(open.outfitter().row(BOOSTER).map(|row| row.owned), Some(1));
    assert_eq!(open.outfitter().free_mass, 8);
    assert_eq!(store.writes(), writes + 1, "saved after the input");
    assert_eq!(saved(&store, "Ada").owned(BOOSTER), 1);
    let faster = flight(&screen)
        .session()
        .expect("flying")
        .handling()
        .max_speed;
    assert!((faster - speed - 3.0).abs() < 1e-6, "{speed} to {faster}");
    // Another, then a click on Buy with too little left to pay.
    screen.input(&key(Key::Char('b'), true));
    assert_eq!(pilot(&screen).cash(), 0);
    let writes = store.writes();
    let buy = spaceport(&screen)
        .open_outfitter()
        .and_then(|open| open.dialog())
        .and_then(|dialog| dialog.item_bounds(OUTFIT_BUY_ITEM))
        .expect("Buy")
        .center();
    for pressed in [true, false] {
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: buy,
        });
    }
    assert_eq!(pilot(&screen).owned(BOOSTER), 2, "greyed");
    assert_eq!(store.writes(), writes, "nothing to save");
    // S sells one back, bought since the outfitter opened: in full.
    screen.input(&key(Key::Char('s'), true));
    assert_eq!(pilot(&screen).owned(BOOSTER), 1);
    assert_eq!(pilot(&screen).cash(), 500);
    // Escape closes the outfitter; the spaceport stays.
    screen.input(&key(Key::Escape, true));
    assert!(spaceport(&screen).open_outfitter().is_none());
    assert_eq!(screen.showing(), Showing::Spaceport);
}

#[test]
fn a_sale_refused_in_the_loop_opens_its_words_over_the_outfitter() {
    let store = MemoryPilots::new();
    let mut screen = landed_outfitter_over(&store, pod_data());
    click_port_item(&mut screen, 8);
    select_first_outfit(&mut screen);
    screen.input(&key(Key::Char('b'), true));
    screen.input(&key(Key::Right, true));
    screen.input(&key(Key::Char('b'), true));
    screen.input(&key(Key::Left, true));
    assert_eq!(pilot(&screen).owned(BOOSTER), 1);
    assert_eq!(pilot(&screen).owned(OutfitId(129)), 1);
    let open = spaceport(&screen).open_outfitter().expect("outfitting");
    assert_eq!(open.outfitter().free_mass, 5);
    assert!(open.alert_dialog().is_none());
    let (cash, writes) = (pilot(&screen).cash(), store.writes());
    screen.input(&key(Key::Char('s'), true));
    assert_eq!(pilot(&screen).owned(BOOSTER), 1, "nothing sold");
    assert_eq!(pilot(&screen).cash(), cash);
    assert_eq!(store.writes(), writes, "nothing to save");
    let open = spaceport(&screen).open_outfitter().expect("outfitting");
    let alert = open.alert_dialog().expect("the words in DLOG 3003");
    assert_eq!(alert.lines().join(" "), NEGATIVE_FREE_MASS);
    screen.input(&key(Key::Escape, true));
    let open = spaceport(&screen)
        .open_outfitter()
        .expect("still outfitting");
    assert!(open.alert_dialog().is_none(), "Escape closed the words");
    screen.input(&key(Key::Escape, true));
    assert!(spaceport(&screen).open_outfitter().is_none());
    assert_eq!(screen.showing(), Showing::Spaceport);
}

#[test]
fn a_counted_sale_refused_part_way_keeps_its_sales_and_opens_its_words() {
    let store = MemoryPilots::new();
    let mut screen = landed_outfitter_over(&store, two_pod_data());
    click_port_item(&mut screen, 8);
    select_first_outfit(&mut screen);
    screen.input(&key(Key::Char('b'), true));
    screen.input(&key(Key::Char('b'), true));
    screen.input(&key(Key::Right, true));
    screen.input(&key(Key::Char('b'), true));
    screen.input(&key(Key::Left, true));
    assert_eq!(pilot(&screen).owned(BOOSTER), 2);
    assert_eq!(pilot(&screen).owned(OutfitId(129)), 1);
    let open = spaceport(&screen).open_outfitter().expect("outfitting");
    assert_eq!(open.outfitter().free_mass, 25);
    let refund = open.outfitter().refund(BOOSTER).expect("a refund");
    let (cash, writes) = (pilot(&screen).cash(), store.writes());
    // Alt-S, as a Mac sends Option-S, then a count of 2.
    screen.input(&key(Key::Alt, true));
    screen.input(&key(Key::Char('s'), true));
    screen.input(&Input::Text('ß'));
    screen.input(&key(Key::Alt, false));
    screen.input(&key(Key::Char('2'), true));
    screen.input(&Input::Text('2'));
    screen.input(&key(Key::Enter, true));
    assert_eq!(pilot(&screen).owned(BOOSTER), 1, "one sold, then refused");
    assert_eq!(pilot(&screen).cash(), cash + refund);
    assert_eq!(store.writes(), writes + 1, "saved after the input");
    let open = spaceport(&screen).open_outfitter().expect("outfitting");
    let alert = open.alert_dialog().expect("the words in DLOG 3003");
    assert_eq!(alert.lines().join(" "), NEGATIVE_FREE_MASS);
    screen.input(&key(Key::Escape, true));
    let open = spaceport(&screen)
        .open_outfitter()
        .expect("still outfitting");
    assert!(open.alert_dialog().is_none(), "Escape closed the words");
}

#[test]
fn alt_on_buy_asks_for_a_count_and_buys_the_count_typed() {
    let store = MemoryPilots::new();
    let mut screen = landed_outfitter(&store);
    click_port_item(&mut screen, 8);
    select_first_outfit(&mut screen);
    let writes = store.writes();
    screen.input(&key(Key::Alt, true));
    let buy = spaceport(&screen)
        .open_outfitter()
        .and_then(|open| open.dialog())
        .and_then(|dialog| dialog.item_bounds(OUTFIT_BUY_ITEM))
        .expect("Buy")
        .center();
    for pressed in [true, false] {
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: buy,
        });
    }
    screen.input(&key(Key::Alt, false));
    let open = spaceport(&screen).open_outfitter().expect("outfitting");
    let asking = open.quantity().expect("asking");
    assert_eq!(asking.field().text(), "2", "1000 / 500");
    assert_eq!(pilot(&screen).owned(BOOSTER), 0, "the dialog asks first");
    screen.input(&key(Key::Char('2'), true));
    screen.input(&Input::Text('2'));
    screen.input(&key(Key::Enter, true));
    assert_eq!(pilot(&screen).owned(BOOSTER), 2);
    assert_eq!(pilot(&screen).cash(), 0, "two prices");
    let open = spaceport(&screen).open_outfitter().expect("outfitting");
    assert!(open.quantity().is_none(), "closed");
    assert_eq!(open.outfitter().row(BOOSTER).map(|row| row.owned), Some(2));
    assert_eq!(store.writes(), writes + 1, "saved after the input");
}

#[test]
fn a_move_by_a_purchase_hook_keeps_the_spaceport_open_and_saves_the_pilot_moved() {
    let store = MemoryPilots::new();
    let mut screen = landed_outfitter_over(&store, hooked_outfitting_data(b"M129"));
    click_port_item(&mut screen, 8);
    select_first_outfit(&mut screen);
    let writes = store.writes();
    screen.input(&key(Key::Char('b'), true));
    assert_eq!(screen.showing(), Showing::Spaceport, "still landed");
    assert_eq!(spaceport(&screen).stellar(), nova_sim::StellarId(128));
    let open = spaceport(&screen).open_outfitter().expect("outfitting");
    assert_eq!(open.outfitter().row(BOOSTER).map(|row| row.owned), Some(1));
    let session = flight(&screen).session().expect("flying");
    assert_eq!(session.landed(), Some(nova_sim::StellarId(128)));
    assert!(session.market().is_some(), "the port's exchange still");
    assert_eq!(pilot(&screen).system(), nova_sim::SystemId(129));
    assert_eq!(pilot(&screen).stellar(), Some(nova_sim::StellarId(129)));
    assert_eq!(store.writes(), writes + 1, "saved once, after the move");
    assert_eq!(saved(&store, "Ada").system(), nova_sim::SystemId(129));
    screen.input(&key(Key::Escape, true));
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::Flight);
    let scene = flight(&screen)
        .scene()
        .map(nova_view::system::SystemScene::id);
    assert_eq!(scene, Some(nova_sim::SystemId(129)), "taken off there");
}

#[test]
fn a_q_by_a_purchase_hook_closes_the_spaceport_takes_off_and_saves() {
    let store = MemoryPilots::new();
    let mut screen = landed_outfitter_over(&store, hooked_outfitting_data(b"Q25048"));
    click_port_item(&mut screen, 8);
    select_first_outfit(&mut screen);
    let writes = store.writes();
    screen.input(&key(Key::Char('b'), true));
    assert_eq!(screen.showing(), Showing::Flight);
    assert!(screen.spaceport_view().is_none());
    assert_eq!(pilot(&screen).owned(BOOSTER), 1, "bought first");
    let session = flight(&screen).session().expect("flying");
    assert_eq!(session.landed(), None);
    assert_eq!(flight(&screen).message(), Some("Off you go, <PSN>."));
    assert_eq!(store.writes(), writes + 1, "saved once, after the input");
    assert_eq!(saved(&store, "Ada").owned(BOOSTER), 1);
    let sounds = screen.take_sounds();
    let took_off = sounds
        .iter()
        .position(|&sound| sound == Sound::Sim(nova_sim::SimSound::TookOff));
    assert!(took_off.is_some(), "{sounds:?}");
    assert!(
        sounds
            .iter()
            .position(|&sound| sound == halted())
            .is_some_and(|halt| Some(halt) < took_off),
        "the ambient sound halts with the spaceport: {sounds:?}"
    );
    assert!(
        !sounds.contains(&Sound::Sim(nova_sim::SimSound::ScriptMessage)),
        "no beep on the take-off"
    );
    screen.input(&key(Key::Char('l'), true));
    assert_eq!(screen.showing(), Showing::Flight, "flight takes the keys");
}

#[test]
fn a_sound_by_a_purchase_hook_plays_after_the_take_off() {
    let store = MemoryPilots::new();
    let mut screen = landed_outfitter_over(&store, hooked_outfitting_data(b"P300"));
    click_port_item(&mut screen, 8);
    select_first_outfit(&mut screen);
    screen.input(&key(Key::Char('b'), true));
    screen.tick(Duration::from_millis(100));
    let mission = Sound::Sim(SimSound::Script {
        sound: nova_sim::SoundId(300),
        exclusive: true,
    });
    assert!(!screen.take_sounds().contains(&mission), "landed");
    screen.input(&key(Key::Escape, true));
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::Flight);
    screen.take_sounds();
    screen.tick(Duration::from_millis(100));
    assert_eq!(
        screen
            .take_sounds()
            .into_iter()
            .filter(|&sound| sound == mission)
            .count(),
        1
    );
}

#[test]
fn a_new_pilot_moved_by_its_on_start_flies_there_and_is_saved_there() {
    let store = MemoryPilots::new();
    let mut screen = AppScreen::new(starting_data(b"M129"))
        .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
        .appeared();
    create(&mut screen, "Ada");
    assert_eq!(screen.showing(), Showing::Flight);
    assert_eq!(pilot(&screen).system(), nova_sim::SystemId(129));
    let scene = flight(&screen)
        .scene()
        .map(nova_view::system::SystemScene::id);
    assert_eq!(scene, Some(nova_sim::SystemId(129)));
    assert_eq!(saved(&store, "Ada").system(), nova_sim::SystemId(129));
}

#[test]
fn without_the_outfit_dialog_the_outfitter_says_why() {
    let store = MemoryPilots::new();
    let mut screen = AppScreen::new(outfitting_data())
        .with_dialogs(Rc::new(Dialogs), Rc::new(MonoMetrics))
        .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
        .appeared();
    create(&mut screen, "Ada");
    land_twice(&mut screen);
    click_port_item(&mut screen, 8);
    let open = spaceport(&screen).open_outfitter().expect("outfitting");
    assert_eq!(open.problem(), Some("no DLOG 1002"));
}

#[test]
fn a_stellar_without_an_outfitter_offers_none() {
    let store = MemoryPilots::new();
    let mut screen = landed_trader(&store);
    click_port_item(&mut screen, 8);
    assert!(spaceport(&screen).open_outfitter().is_none());
    assert!(spaceport(&screen).open_service().is_none());
}

#[test]
fn an_order_in_one_refreshes_the_other_for_when_it_opens() {
    let store = MemoryPilots::new();
    let mut screen = AppScreen::new(outfitting_data())
        .with_dialogs(Rc::new(EveryDialog), Rc::new(MonoMetrics))
        .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
        .appeared();
    create(&mut screen, "Ada");
    land_twice(&mut screen);
    click_port_item(&mut screen, 8);
    select_first_outfit(&mut screen);
    screen.input(&key(Key::Char('b'), true));
    screen.input(&key(Key::Escape, true));
    click_port_item(&mut screen, 7);
    let open = spaceport(&screen).open_trade().expect("trading");
    assert_eq!(open.market().cash, 500, "the outfit was paid for");
    screen.input(&key(Key::Char('b'), true));
    assert_eq!(pilot(&screen).held(FOOD), 6, "500 at 75");
    screen.input(&key(Key::Escape, true));
    click_port_item(&mut screen, 8);
    select_first_outfit(&mut screen);
    let open = spaceport(&screen).open_outfitter().expect("outfitting");
    assert_eq!(open.outfitter().cash, 50, "the food was paid for");
}

// The Shipyard.

#[test]
fn the_shipyard_opens_the_stellars_shipyard() {
    let store = MemoryPilots::new();
    let mut screen = landed_shipyard(&store);
    assert!(
        spaceport(&screen)
            .offered()
            .contains(&nova_sim::Service::Shipyard)
    );
    click_port_item(&mut screen, 9);
    let open = spaceport(&screen).open_shipyard().expect("shipbuying");
    assert_eq!(open.problem(), None);
    let second = open.shipyard().row(ShipId(129)).expect("listed");
    assert_eq!(second.price, 800);
    assert_eq!(open.shipyard().row(ShipId(128)), None, "BuyRandom 0");
    assert_eq!((open.shipyard().cash, open.shipyard().trade_in), (1000, 0));
    assert_eq!(screen.showing(), Showing::Spaceport);
    assert_eq!(drawn(&screen), drawn(spaceport(&screen)));
}

/// Presses B, as the platform sends it: the key, then its character.
fn press_b(screen: &mut AppScreen) {
    screen.input(&key(Key::Char('b'), true));
    screen.input(&Input::Text('b'));
    screen.input(&key(Key::Char('b'), false));
}

/// The name prompt open over the shipyard, if it is.
fn name_prompt(screen: &AppScreen) -> Option<&nova_view::ui::TextInputDialog> {
    spaceport(screen)
        .open_shipyard()
        .and_then(nova_view::spaceport::ShipyardScreen::naming)
}

#[test]
fn a_ship_order_changes_the_pilots_ship_refreshes_the_shops_and_saves() {
    let store = MemoryPilots::new();
    let mut screen = landed_shipyard(&store);
    click_port_item(&mut screen, 9);
    select_first_ship(&mut screen);
    let writes = store.writes();
    press_b(&mut screen);
    assert_eq!(pilot(&screen).ship(), ShipId(128), "named first");
    screen.input(&key(Key::Enter, true));
    assert_eq!(pilot(&screen).ship(), ShipId(129));
    assert_eq!(pilot(&screen).cash(), 200);
    assert_eq!(pilot(&screen).owned(BOOSTER), 1, "its default booster");
    assert_eq!(store.writes(), writes + 1, "saved after the input");
    assert_eq!(saved(&store, "Ada").ship(), ShipId(129));
    assert!(
        spaceport(&screen).open_shipyard().is_none(),
        "the purchase closes it"
    );
    click_port_item(&mut screen, 9);
    let open = spaceport(&screen).open_shipyard().expect("shipbuying");
    assert_eq!(open.shipyard().current, ShipId(129));
    assert_eq!(open.shipyard().cash, 200);
    let speed = flight(&screen)
        .session()
        .expect("flying")
        .handling()
        .max_speed;
    assert!(
        (speed - 9.0).abs() < 1e-6,
        "600 and the booster's 300: {speed}"
    );
    // The outfitter and the exchange read the new ship.
    screen.input(&key(Key::Escape, true));
    click_port_item(&mut screen, 8);
    let open = spaceport(&screen).open_outfitter().expect("outfitting");
    assert_eq!(open.outfitter().cash, 200);
    assert_eq!(open.outfitter().free_mass, 12);
    screen.input(&key(Key::Escape, true));
    click_port_item(&mut screen, 7);
    let open = spaceport(&screen).open_trade().expect("trading");
    assert_eq!((open.market().cash, open.market().capacity), (200, 20));
    assert_eq!(screen.showing(), Showing::Spaceport);
}

#[test]
fn a_refused_ship_order_changes_nothing_and_saves_nothing() {
    let store = MemoryPilots::new();
    let mut screen = landed_shipyard_of(dear_shipbuying_data(), &store);
    click_port_item(&mut screen, 9);
    select_first_ship(&mut screen);
    press_b(&mut screen);
    screen.input(&key(Key::Enter, true));
    let writes = store.writes();
    let before = pilot(&screen).clone();
    // Ship 130: 200 and a 400 trade-in do not cover 5,000.
    click_port_item(&mut screen, 9);
    select_first_ship(&mut screen);
    screen.input(&key(Key::Right, true));
    let open = spaceport(&screen).open_shipyard().expect("shipbuying");
    assert_eq!(
        open.shipyard().can(ShipId(130)),
        Err(nova_sim::ShipRefusal::CannotAfford)
    );
    press_b(&mut screen);
    assert!(name_prompt(&screen).is_none(), "nothing to name");
    screen.input(&key(Key::Enter, true));
    assert_eq!(*pilot(&screen), before);
    assert_eq!(store.writes(), writes, "nothing to save");
}

#[test]
fn buying_a_ship_asks_its_name_and_keeps_the_name_confirmed() {
    let store = MemoryPilots::new();
    let mut screen = landed_shipyard(&store);
    click_port_item(&mut screen, 9);
    select_first_ship(&mut screen);
    assert_eq!(pilot(&screen).ship_name(), Some(""), "no STR# 128 default");
    let writes = store.writes();
    press_b(&mut screen);
    let prompt = name_prompt(&screen).expect("the name prompt");
    let default = prompt.field().text().to_owned();
    let digits = default.strip_prefix("Second ").expect("the class's name");
    assert_eq!(digits.len(), 3, "{default}");
    assert!(
        digits.chars().all(|digit| ('1'..='9').contains(&digit)),
        "{default}"
    );
    assert!(prompt.field().selected(), "the B typed nothing");
    let texts: Vec<String> = drawn(&screen)
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert!(
        texts
            .iter()
            .any(|text| text.contains("Please name your new Second Shipyard:")),
        "{texts:?}"
    );
    assert_eq!(store.writes(), writes, "nothing bought yet");
    let message = flight(&screen).message().map(str::to_owned);
    type_text(&mut screen, "The Kestrel");
    screen.input(&key(Key::Enter, true));
    assert!(name_prompt(&screen).is_none(), "closed");
    assert_eq!(pilot(&screen).ship(), ShipId(129));
    assert_eq!(pilot(&screen).ship_name(), Some("Kestrel"));
    assert_eq!(store.writes(), writes + 1, "saved after the input");
    assert_eq!(saved(&store, "Ada").ship_name(), Some("Kestrel"));
    assert_eq!(
        flight(&screen).message().map(str::to_owned),
        message,
        "no message follows"
    );
    assert!(
        spaceport(&screen).open_shipyard().is_none(),
        "the purchase closes it"
    );
}

#[test]
fn typing_a_ship_name_never_opens_the_dialogs_under_it() {
    let store = MemoryPilots::new();
    let mut screen = landed_shipyard(&store);
    click_port_item(&mut screen, 9);
    select_first_ship(&mut screen);
    press_b(&mut screen);
    for c in ['p', 'i', 'f'] {
        screen.input(&key(Key::Char(c), true));
        screen.input(&Input::Text(c));
        screen.input(&key(Key::Char(c), false));
    }
    assert!(screen.preferences().is_none(), "no Preferences");
    assert_eq!(screen.showing(), Showing::Spaceport);
    let prompt = name_prompt(&screen).expect("the name prompt");
    assert_eq!(prompt.field().text(), "pif");
    // Once it is closed, P opens the player info.
    click_cancel(&mut screen);
    assert!(name_prompt(&screen).is_none(), "declined");
    screen.input(&key(Key::Char('p'), true));
    assert!(screen.preferences().is_none());
    assert!(
        spaceport(&screen).player_info().is_some(),
        "P is the player info"
    );
}

#[test]
fn cancelling_the_name_prompt_buys_nothing_and_saves_nothing() {
    let store = MemoryPilots::new();
    let mut screen = landed_shipyard(&store);
    click_port_item(&mut screen, 9);
    select_first_ship(&mut screen);
    let writes = store.writes();
    let before = pilot(&screen).clone();
    press_b(&mut screen);
    click_cancel(&mut screen);
    assert!(name_prompt(&screen).is_none(), "closed");
    assert_eq!(*pilot(&screen), before);
    assert_eq!(store.writes(), writes, "nothing to save");
    assert!(spaceport(&screen).open_shipyard().is_some());
}

/// Clicks the open name prompt's Cancel.
fn click_cancel(screen: &mut AppScreen) {
    let cancel = name_prompt(screen)
        .expect("the name prompt")
        .dialog()
        .item_bounds(6)
        .expect("Cancel")
        .center();
    for pressed in [true, false] {
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: cancel,
        });
    }
}

/// The `BuyRandom` ship 129 sells on in [`rolling_shipyard`]: a
/// chance no other draw asks.
const ROLLED: u8 = 37;

/// Answers each [`ROLLED`] % draw from its script, then misses, and
/// counts them; every other chance misses, as [`nova_sim::NeverFires`]
/// does.
#[derive(Default)]
struct BuyRolls {
    script: VecDeque<bool>,
    asked: usize,
}

impl Chance for BuyRolls {
    fn fires(&mut self, percent: u8) -> bool {
        if percent != ROLLED {
            return false;
        }
        self.asked += 1;
        self.script.pop_front().unwrap_or(false)
    }

    fn below(&mut self, n: u32) -> u32 {
        n.saturating_sub(1)
    }

    fn roll(&mut self, _sides: u16) -> u16 {
        0
    }
}

/// [`landed_shipyard`], ship 129 for sale on a chance of [`ROLLED`] %
/// drawn from `script`, and the draws.
fn rolling_shipyard(store: &MemoryPilots, script: &[bool]) -> (AppScreen, Rc<RefCell<BuyRolls>>) {
    let rolls = Rc::new(RefCell::new(BuyRolls {
        script: script.iter().copied().collect(),
        asked: 0,
    }));
    let shared: Rc<RefCell<dyn Chance>> = rolls.clone();
    let mut screen = AppScreen::new(game_data_selling(true, true, true, i16::from(ROLLED)))
        .with_dialogs(Rc::new(ShipyardDialogs), Rc::new(MonoMetrics))
        .with_pilots(Some(keeper(store)), Rc::new(MonoMetrics))
        .appeared()
        .with_chance(SharedChance::new(shared));
    create(&mut screen, "Ada");
    land_twice(&mut screen);
    assert_eq!(screen.showing(), Showing::Spaceport);
    (screen, rolls)
}

fn lists_second(screen: &AppScreen) -> bool {
    let open = spaceport(screen).open_shipyard().expect("shipbuying");
    open.shipyard().row(ShipId(129)).is_some()
}

#[test]
fn a_ship_declined_is_named_again_on_the_list_built_at_opening() {
    let store = MemoryPilots::new();
    let (mut screen, rolls) = rolling_shipyard(&store, &[true, false]);
    click_port_item(&mut screen, 9);
    select_first_ship(&mut screen);
    assert!(lists_second(&screen));
    assert_eq!(rolls.borrow().asked, 1, "drawn as the list was built");
    press_b(&mut screen);
    click_cancel(&mut screen);
    assert!(name_prompt(&screen).is_none(), "closed");
    press_b(&mut screen);
    assert!(name_prompt(&screen).is_some(), "the prompt opens again");
    assert_eq!(rolls.borrow().asked, 1, "nothing drawn again yet");
    screen.input(&key(Key::Enter, true));
    assert_eq!(pilot(&screen).ship(), ShipId(129), "bought");
}

#[test]
fn a_ship_declined_stays_listed_while_open_and_draws_its_roll_again_at_the_next_opening() {
    let store = MemoryPilots::new();
    let (mut screen, rolls) = rolling_shipyard(&store, &[true, false]);
    click_port_item(&mut screen, 9);
    select_first_ship(&mut screen);
    press_b(&mut screen);
    click_cancel(&mut screen);
    assert!(lists_second(&screen), "the list is kept while open");
    assert_eq!(rolls.borrow().asked, 1);
    // Player info over the shipyard hands the shops back: the list
    // built as it opened is relisted, and nothing is drawn.
    screen.input(&key(Key::Char('p'), true));
    screen.input(&key(Key::Char('p'), false));
    assert!(spaceport(&screen).player_info().is_some(), "player info");
    screen.input(&key(Key::Escape, true));
    screen.input(&key(Key::Escape, false));
    assert!(spaceport(&screen).player_info().is_none());
    assert!(lists_second(&screen), "still listed");
    assert_eq!(rolls.borrow().asked, 1, "nothing drawn while open");
    // An order elsewhere draws nothing either.
    screen.input(&key(Key::Escape, true));
    click_port_item(&mut screen, 8);
    select_first_outfit(&mut screen);
    screen.input(&key(Key::Char('b'), true));
    assert_eq!(pilot(&screen).owned(BOOSTER), 1, "the outfit was bought");
    assert_eq!(rolls.borrow().asked, 1, "not drawn by the outfitter");
    screen.input(&key(Key::Escape, true));
    click_port_item(&mut screen, 9);
    assert_eq!(rolls.borrow().asked, 2, "drawn again as it opens");
    assert!(
        spaceport(&screen).open_shipyard().is_none(),
        "the new roll missed: nothing listed, it never opens"
    );
}

#[test]
fn a_purchase_closes_the_shipyard_and_the_next_opening_lists_by_the_new_rolls() {
    let store = MemoryPilots::new();
    let (mut screen, rolls) = rolling_shipyard(&store, &[true, true]);
    click_port_item(&mut screen, 9);
    select_first_ship(&mut screen);
    press_b(&mut screen);
    screen.input(&key(Key::Enter, true));
    assert_eq!(pilot(&screen).ship(), ShipId(129), "bought");
    assert!(spaceport(&screen).open_shipyard().is_none(), "closed");
    assert_eq!(rolls.borrow().asked, 1, "not drawn again yet");
    click_port_item(&mut screen, 9);
    assert_eq!(rolls.borrow().asked, 2, "drawn again as it opens");
    let open = spaceport(&screen).open_shipyard().expect("listed again");
    assert!(lists_second(&screen), "the new roll holds");
    assert_eq!(open.shipyard().current, ShipId(129), "marked (current)");
}

#[test]
fn a_stellar_without_a_shipyard_offers_none() {
    let store = MemoryPilots::new();
    let mut screen = landed_outfitter(&store);
    click_port_item(&mut screen, 9);
    assert!(spaceport(&screen).open_shipyard().is_none());
    assert!(spaceport(&screen).open_service().is_none());
}

#[test]
fn without_the_shipyard_dialog_the_shipyard_says_why() {
    let store = MemoryPilots::new();
    let mut screen = AppScreen::new(shipbuying_data())
        .with_dialogs(Rc::new(EveryDialog), Rc::new(MonoMetrics))
        .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
        .appeared();
    create(&mut screen, "Ada");
    land_twice(&mut screen);
    click_port_item(&mut screen, 9);
    let open = spaceport(&screen).open_shipyard().expect("shipbuying");
    assert_eq!(open.problem(), Some("no DLOG 1004"));
}

// The status bar in port.

/// [`outfitting_data`] with the status bar ([`barred`]), ship 128
/// carrying 300 fuel, and the outfitter also selling `oütf` 132, IFF
/// (`ModType` 14), and `oütf` 133, a fuel tank (`ModType` 12) of 100,
/// each 100 credits and weighing nothing.
fn barred_outfitting_data() -> Rc<GameData> {
    barred_data(false)
}

/// [`barred_outfitting_data`], where the planet is also a shipyard
/// ([`shipbuying_data`]) whose ship 129 carries IFF (`oütf` 132)
/// beside its booster.
fn barred_shipbuying_data() -> Rc<GameData> {
    barred_data(true)
}

/// [`barred_outfitting_data`], or [`barred_shipbuying_data`] when
/// `shipbuying`.
fn barred_data(shipbuying: bool) -> Rc<GameData> {
    let sold = |mod_type: i16, mod_val: i16| {
        let mut outfit = booster_record(b"");
        put_i16s(&mut outfit, 0x02, &[0]);
        put_i16s(&mut outfit, 0x06, &[mod_type, mod_val]);
        outfit[0x0E..0x12].copy_from_slice(&100_i32.to_be_bytes());
        outfit
    };
    let fork = barred(game_data_fork(true, true, shipbuying, 100, b"", b"", None))
        .resource(Outfit::TYPE, 132, Some(b"IFF"), &sold(14, 0))
        .resource(Outfit::TYPE, 133, Some(b"Tank"), &sold(12, 100));
    let mut built = fork.build();
    let first = built.layout.data_entry(Ship::TYPE, 128) + 4;
    built.put_u16(first + 0x0A, 300);
    if shipbuying {
        let second = built.layout.data_entry(Ship::TYPE, 129) + 4;
        built.put_u16(second + 0x50, 132);
        built.put_u16(second + 0x58, 1);
    }
    loaded_bytes(built.bytes)
}

/// The status bar's background in port: `PICT` 700 against the right
/// edge.
fn bar_background() -> DrawCommand {
    DrawCommand::Picture {
        image: nova_view::ImageKey::picture(700),
        top_left: Point::new(830.0, 0.0),
    }
}

/// What flight's status bar alone draws ([`FlightView::draw_status_bar`]).
fn bar_drawn(screen: &AppScreen) -> Vec<DrawCommand> {
    let mut list = DrawList::new();
    flight(screen).draw_status_bar(&mut list);
    list.iter().cloned().collect()
}

/// The 1-pixel dots inside the radar, (838, 8) to (1014, 184), as
/// their pixels.
fn radar_dots(list: &DrawList) -> Vec<(f32, f32)> {
    list.iter()
        .filter_map(|command| match *command {
            DrawCommand::Dot {
                center, size: 1.0, ..
            } if (838.0..1014.0).contains(&center.x) && (8.0..184.0).contains(&center.y) => {
                Some((center.x - 0.5, center.y - 0.5))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn in_port_the_status_bar_shows_beside_the_spaceport_with_a_blank_radar() {
    let store = MemoryPilots::new();
    let mut screen = AppScreen::new(barred_outfitting_data())
        .with_dialogs(Rc::new(OutfitDialogs), Rc::new(MonoMetrics))
        .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
        .appeared();
    create(&mut screen, "Ada");
    assert!(
        radar_dots(&drawn(&screen)).len() > 1,
        "in flight the planet is on radar too"
    );
    land_twice(&mut screen);
    assert_eq!(screen.showing(), Showing::Spaceport);
    let list = drawn(&screen);
    let bar = bar_drawn(&screen);
    assert_eq!(bar.first(), Some(&bar_background()));
    let mut port = drawn(spaceport(&screen))
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(
        port.first(),
        Some(&DrawCommand::Picture {
            image: nova_view::spaceport::layout::BACKGROUND,
            top_left: spaceport(&screen).dialog().expect("laid out").bounds().min,
        })
    );
    let mut under = bar.clone();
    under.append(&mut port);
    assert_eq!(list.iter().cloned().collect::<Vec<_>>(), under);
    assert_eq!(radar_dots(&list), [(926.0, 96.0)], "the player alone");
}

#[test]
fn the_outfitter_draws_over_the_status_bar() {
    let store = MemoryPilots::new();
    let mut screen = landed_outfitter_over(&store, barred_outfitting_data());
    click_port_item(&mut screen, 8);
    assert!(spaceport(&screen).open_outfitter().is_some());
    let mut under = bar_drawn(&screen);
    assert_eq!(under.first(), Some(&bar_background()));
    under.extend(drawn(spaceport(&screen)).iter().cloned());
    assert_eq!(drawn(&screen).iter().cloned().collect::<Vec<_>>(), under);
}

#[test]
fn the_status_bar_is_drawn_in_port_and_flight_only() {
    let mut screen = AppScreen::new(barred_outfitting_data());
    let background = bar_background();
    fly(&mut screen);
    assert!(drawn(&screen).iter().any(|c| *c == background), "flight");
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::ShipBrowser);
    assert!(!drawn(&screen).iter().any(|c| *c == background), "ships");
    screen.input(&key(Key::Tab, true));
    assert_eq!(screen.showing(), Showing::GalaxyMap);
    assert!(!drawn(&screen).iter().any(|c| *c == background), "galaxy");
}

#[test]
fn a_port_without_a_bar_draws_no_reason() {
    let store = MemoryPilots::new();
    let screen = landed_outfitter(&store);
    assert!(flight(&screen).status_bar().is_err(), "no ïntf");
    let texts: Vec<String> = drawn(&screen)
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert!(
        !texts.iter().any(|text| text.starts_with("Status bar")),
        "{texts:?}"
    );
    assert_eq!(drawn(&screen), drawn(spaceport(&screen)));
}

const IFF: OutfitId = OutfitId(132);

/// Clicks at `at` with the left button.
fn click_on(screen: &mut AppScreen, at: Point) {
    for pressed in [true, false] {
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        });
    }
}

/// Selects `outfit`'s cell in the outfitter open, then clicks its
/// item `item`.
fn click_for_outfit(screen: &mut AppScreen, outfit: OutfitId, item: usize) {
    let rows = spaceport(screen)
        .open_outfitter()
        .expect("outfitting")
        .outfitter()
        .rows
        .len();
    for _ in 0..rows {
        let open = spaceport(screen).open_outfitter().expect("outfitting");
        if open.selected_outfit() == Some(outfit) {
            break;
        }
        screen.input(&key(Key::Right, true));
    }
    let open = spaceport(screen).open_outfitter().expect("outfitting");
    assert_eq!(open.selected_outfit(), Some(outfit));
    let at = open
        .dialog()
        .and_then(|dialog| dialog.item_bounds(item))
        .expect("the item")
        .center();
    click_on(screen, at);
}

/// Whether `list` paints the radar, (838, 8) to (1014, 184), black.
fn radar_black(list: &DrawList) -> bool {
    list.iter().any(|command| {
        matches!(*command, DrawCommand::Line { from, to, color, .. }
            if color == Color::rgba(0, 0, 0, 255)
                && from.x >= 838.0 && to.x <= 1014.0
                && (8.0..184.0).contains(&from.y))
    })
}

/// Whether `list` draws the player cyan at the radar's centre.
fn player_cyan(list: &DrawList) -> bool {
    list.iter().any(|command| {
        *command
            == DrawCommand::Dot {
                center: Point::new(926.5, 96.5),
                size: 1.0,
                color: Color::rgba(0, 255, 255, 255),
            }
    })
}

#[test]
fn an_iff_bought_in_port_turns_the_bars_radar_black_at_once_and_one_sold_clears_it() {
    let store = MemoryPilots::new();
    let mut screen = landed_outfitter_over(&store, barred_outfitting_data());
    click_port_item(&mut screen, 8);
    assert!(!radar_black(&drawn(&screen)), "without IFF");
    click_for_outfit(&mut screen, IFF, OUTFIT_BUY_ITEM);
    assert_eq!(pilot(&screen).owned(IFF), 1);
    let list = drawn(&screen);
    assert!(radar_black(&list), "bought");
    assert!(player_cyan(&list), "bought");
    click_for_outfit(&mut screen, IFF, OUTFIT_SELL_ITEM);
    assert_eq!(pilot(&screen).owned(IFF), 0);
    let list = drawn(&screen);
    assert!(!radar_black(&list), "sold");
    assert!(!player_cyan(&list), "sold");
    assert!(spaceport(&screen).open_outfitter().is_some(), "still open");
}

#[test]
fn a_ship_bought_carrying_iff_turns_the_bars_radar_black_at_once() {
    let store = MemoryPilots::new();
    let mut screen = landed_shipyard_of(barred_shipbuying_data(), &store);
    click_port_item(&mut screen, 9);
    select_first_ship(&mut screen);
    assert!(!radar_black(&drawn(&screen)), "without IFF");
    press_b(&mut screen);
    screen.input(&key(Key::Enter, true));
    assert_eq!(pilot(&screen).ship(), ShipId(129));
    assert_eq!(pilot(&screen).owned(IFF), 1, "its default IFF");
    assert!(spaceport(&screen).open_shipyard().is_none(), "closed");
    let list = drawn(&screen);
    assert!(radar_black(&list));
    assert!(player_cyan(&list));
}

/// Flies `screen` up, the thrust held, until the radar, refreshed
/// every 8 ticks, has just refreshed, then 7 ticks more: it holds the
/// ship as it was 7 ticks back.
fn stale_radar(screen: &mut AppScreen) {
    screen.input(&key(Key::Up, true));
    for _ in 0..200 {
        screen.tick(TICK);
    }
    let mut before = radar_dots(&drawn(screen));
    for _ in 0..8 {
        screen.tick(TICK);
        let after = radar_dots(&drawn(screen));
        if after != before {
            break;
        }
        before = after;
    }
    for _ in 0..7 {
        screen.tick(TICK);
    }
}

#[test]
fn player_infos_and_mission_infos_close_over_flight_redraw_the_whole_bar() {
    type Close = fn(&mut AppScreen);
    let rows: [(&str, Close); 2] = [
        ("player info (@0x5a725)", |screen| {
            screen.input(&key(PLAYER_INFO_KEY, true));
            screen.input(&key(PLAYER_INFO_KEY, false));
            assert!(screen.flight_player_info().is_some(), "open");
            screen.input(&key(Key::Escape, true));
            assert!(screen.flight_player_info().is_none(), "closed");
        }),
        ("mission info (@0x9e769)", |screen| {
            let mut missions = MissionDialogs::new(
                DescTemplates {
                    plain: Err("no DLOG 3003".to_owned()),
                    picture: Err("no DLOG 3004".to_owned()),
                },
                Ok(mission_info_template()),
                Rc::new(MonoMetrics),
                screen.data.button_style(),
            );
            missions.open_info(nova_sim::MissionInfo {
                rows: vec![nova_sim::InfoRow {
                    slot: 0,
                    name: "Run".to_owned(),
                    failed: false,
                    text: "Go.".to_owned(),
                    abortable: true,
                }],
                today: String::new(),
            });
            assert!(missions.info().is_some(), "open");
            screen.missions = Some(missions);
            screen.input(&key(Key::Escape, true));
            assert!(screen.missions.is_none(), "closed");
        }),
    ];
    for (what, close) in rows {
        let mut screen = AppScreen::new(barred_outfitting_data())
            .with_dialogs(Rc::new(Dialogs), Rc::new(MonoMetrics));
        fly(&mut screen);
        stale_radar(&mut screen);
        let held = radar_dots(&drawn(&screen));
        close(&mut screen);
        let shown = radar_dots(&drawn(&screen));
        screen.flight.as_mut().expect("flying").refresh_status();
        let now = radar_dots(&drawn(&screen));
        assert_ne!(held, now, "{what}: the radar held what it drew before");
        assert_eq!(shown, now, "{what}: redrawn as it closed");
    }
}

#[test]
fn player_infos_close_over_flight_without_its_dialog_redraws_nothing() {
    /// [`Dialogs`] without "Player Info" (1017): the original, failing
    /// to get it, neither shows the dialog nor redraws (@0x5a0a5).
    struct NoPlayerInfo;

    impl DialogResources for NoPlayerInfo {
        fn dialog_template(&self, id: i16) -> Result<DialogTemplate, String> {
            if id == PLAYER_INFO_DIALOG {
                return Err(format!("no DLOG {id}"));
            }
            Dialogs.dialog_template(id)
        }
    }

    let mut screen = AppScreen::new(barred_outfitting_data())
        .with_dialogs(Rc::new(NoPlayerInfo), Rc::new(MonoMetrics));
    fly(&mut screen);
    stale_radar(&mut screen);
    let held = radar_dots(&drawn(&screen));
    screen.input(&key(PLAYER_INFO_KEY, true));
    screen.input(&key(PLAYER_INFO_KEY, false));
    let info = screen.flight_player_info().expect("open");
    assert!(info.problem().is_some(), "not laid out");
    screen.input(&key(Key::Escape, true));
    assert!(screen.flight_player_info().is_none(), "closed");
    assert_eq!(radar_dots(&drawn(&screen)), held, "not redrawn");
    screen.flight.as_mut().expect("flying").refresh_status();
    assert_ne!(radar_dots(&drawn(&screen)), held, "stale as it closed");
}

const TANK: OutfitId = OutfitId(133);

/// [`barred_outfitting_data`]'s ship and tank, where Alpha Prime's
/// spaceport offers mission 130 ([`venue_mission`]), whose `OnAccept`
/// is `G133`, the tank, and whose `BriefText` is `dësc` 5000, "Go.".
fn barred_briefing_data() -> Rc<GameData> {
    let mut tank = booster_record(b"");
    put_i16s(&mut tank, 0x02, &[0]);
    put_i16s(&mut tank, 0x06, &[12, 100]);
    tank[0x0E..0x12].copy_from_slice(&100_i32.to_be_bytes());
    let mut mission = venue_mission(SPACEPORT);
    put_i16s(&mut mission, 0x34, &[5000]);
    mission[0x15B..0x15F].copy_from_slice(b"G133");
    let fork = barred(game_data_fork(true, true, true, 100, b"", b"", None))
        .resource(Outfit::TYPE, 133, Some(b"Tank"), &tank)
        .resource(Mission::TYPE, 130, Some(b"Venue job"), &mission)
        .resource(Desc::TYPE, 130 + 3872, Some(b"Offer"), &desc_of("Job."))
        .resource(Desc::TYPE, 5000, Some(b"Brief"), &desc_of("Go."));
    let mut built = fork.build();
    let first = built.layout.data_entry(Ship::TYPE, 128) + 4;
    built.put_u16(first + 0x0A, 300);
    loaded_bytes(built.bytes)
}

#[test]
fn a_spaceport_brief_text_shows_over_the_pre_accept_bar_and_its_done_redraws_it() {
    let store = MemoryPilots::new();
    let mut screen = landed_at_venues(&store, barred_briefing_data());
    assert!(spaceport(&screen).offer_dialog().is_some(), "offered");
    let full = fuel_width(&drawn(&screen)).expect("some fuel");
    edit(
        &mut screen,
        PilotEdit::Reserve(nova_sim::Reserve::Fuel, 150.0),
    );
    assert_eq!(fuel_width(&drawn(&screen)), Some(full), "held");
    screen.input(&key(Key::Enter, true));
    assert_eq!(active(&screen), [MissionId(130)]);
    assert!(
        spaceport(&screen).mission_text().is_some(),
        "the brief shows"
    );
    let half = fuel_width(&drawn(&screen)).expect("some fuel");
    assert!(
        half < full,
        "the dialog's redraw before the accept: {half} of {full}"
    );
    screen.input(&key(Key::Enter, true));
    assert!(spaceport(&screen).mission_text().is_none(), "closed");
    let width = fuel_width(&drawn(&screen)).expect("some fuel");
    assert!(
        width > half,
        "its close redraws OnAccept's tank, fuel and all: {width}"
    );
    screen.flight.as_mut().expect("flying").refresh_status();
    assert_eq!(fuel_width(&drawn(&screen)), Some(width), "as it is now");
}

#[test]
fn mission_info_closing_over_the_outfitter_redraws_the_bar_held_under_it() {
    /// [`VenueDialogs`] with the mission info screen.
    struct InfoDialogs;

    impl DialogResources for InfoDialogs {
        fn dialog_template(&self, id: i16) -> Result<DialogTemplate, String> {
            if id == INFO_DIALOG {
                return Ok(mission_info_template());
            }
            VenueDialogs.dialog_template(id)
        }
    }

    let store = MemoryPilots::new();
    let mut screen = AppScreen::new(barred_briefing_data())
        .with_dialogs(Rc::new(InfoDialogs), Rc::new(MonoMetrics))
        .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
        .appeared();
    create(&mut screen, "Ada");
    land_twice(&mut screen);
    screen.input(&key(Key::Enter, true));
    screen.input(&key(Key::Enter, true));
    assert_eq!(active(&screen), [MissionId(130)], "accepted");
    assert!(spaceport(&screen).mission_text().is_none(), "brief closed");
    let full = fuel_width(&drawn(&screen)).expect("some fuel");
    edit(
        &mut screen,
        PilotEdit::Reserve(nova_sim::Reserve::Fuel, 150.0),
    );
    click_port_item(&mut screen, 8);
    assert!(spaceport(&screen).open_outfitter().is_some(), "open");
    assert_eq!(fuel_width(&drawn(&screen)), Some(full), "held");
    screen.input(&key(nova_view::ui::mission_info::MISSION_INFO_KEY, true));
    screen.input(&key(nova_view::ui::mission_info::MISSION_INFO_KEY, false));
    assert!(spaceport(&screen).mission_info().is_some(), "info open");
    assert_eq!(fuel_width(&drawn(&screen)), Some(full), "not as it opens");
    screen.input(&key(Key::Escape, true));
    assert!(spaceport(&screen).mission_info().is_none(), "info closed");
    assert!(spaceport(&screen).open_outfitter().is_some(), "still open");
    let width = fuel_width(&drawn(&screen)).expect("some fuel");
    assert!(width < full, "redrawn as it closes (@0x9e769): {width}");
}

/// [`barred`] [`shipbuying_data`], its first ship for hire, where
/// Alpha Prime's spaceport offers mission 130, whose `OnAccept`,
/// `S131`, starts mission 131 too, each one's `OnAbort` `M129`, a
/// landed move that selects Beta Prime; and its bar offers mission
/// 129, listed before 130 so the accept's shift (`RuleKey::OfferShift`)
/// leaves it.
fn aborting_to_beta_data() -> Rc<GameData> {
    let mut job = venue_mission(SPACEPORT);
    job[0x15B..0x15F].copy_from_slice(b"S131");
    job[0x557..0x55B].copy_from_slice(b"M129");
    let mut next = venue_mission(SPACEPORT);
    next[0x5C..0x5E].copy_from_slice(b"b9");
    next[0x557..0x55B].copy_from_slice(b"M129");
    let fork = barred(game_data_fork(true, true, true, 100, b"", b"", None))
        .resource(Mission::TYPE, 130, Some(b"Venue job"), &job)
        .resource(Mission::TYPE, 131, Some(b"Next job"), &next)
        .resource(Mission::TYPE, 129, Some(b"Bar job"), &venue_mission(BAR))
        .resource(Desc::TYPE, 130 + 3872, Some(b"Offer"), &desc_of("Job."))
        .resource(Desc::TYPE, 129 + 3872, Some(b"Offer"), &desc_of("Bar job."));
    let mut built = fork.build();
    let first = built.layout.data_entry(Ship::TYPE, 128) + 4;
    built.put_u16(first + 0x38A, 100);
    loaded_bytes(built.bytes)
}

/// Whether `screen` draws "Beta Prime", which only the nav area can.
fn names_beta(screen: &AppScreen) -> bool {
    drawn(screen)
        .iter()
        .any(|command| matches!(command, DrawCommand::Text { text, .. } if text == "Beta Prime"))
}

/// A screen mission info opens over: what it is, how to open it on
/// the spaceport, and whether it is open.
type InfoOpener = (&'static str, fn(&mut AppScreen), fn(&SpaceportView) -> bool);

/// Each screen mission info opens over that redraws the bar as it
/// closes (@0x9e769), as [`aborting_to_beta_data`] lays them out.
const INFO_OPENERS: [InfoOpener; 7] = [
    (
        "the exchange",
        |screen| click_port_item(screen, 7),
        |port| port.open_trade().is_some(),
    ),
    (
        "the outfitter",
        |screen| click_port_item(screen, 8),
        |port| port.open_outfitter().is_some(),
    ),
    (
        "the shipyard",
        |screen| click_port_item(screen, 9),
        |port| port.open_shipyard().is_some(),
    ),
    (
        "the shipyard's description",
        |screen| {
            click_port_item(screen, 9);
            select_first_ship(screen);
            screen.input(&key(nova_view::spaceport::shipyard::INFO_KEY, true));
        },
        |port| {
            port.open_shipyard()
                .and_then(ShipyardScreen::info_panel)
                .is_some()
        },
    ),
    (
        "the hire screen",
        |screen| {
            click_port_item(screen, 11);
            screen.input(&key(Key::Char('h'), true));
        },
        |port| port.open_bar().and_then(BarScreen::hire_screen).is_some(),
    ),
    (
        "the hire screen's description",
        |screen| {
            click_port_item(screen, 11);
            screen.input(&key(Key::Char('h'), true));
            select_first_ship(screen);
            screen.input(&key(nova_view::spaceport::shipyard::INFO_KEY, true));
        },
        |port| {
            port.open_bar()
                .and_then(BarScreen::hire_screen)
                .and_then(nova_view::spaceport::HireScreen::info_panel)
                .is_some()
        },
    ),
    (
        "an offer over the bar",
        |screen| {
            click_port_item(screen, 11);
            screen.tick(Duration::from_millis(300));
            screen.tick(Duration::from_millis(300));
            assert_eq!(
                offered_mission(
                    spaceport(screen)
                        .open_bar()
                        .and_then(BarScreen::offer_dialog)
                ),
                Some(MissionId(129))
            );
        },
        |port| port.open_bar().and_then(BarScreen::offer_dialog).is_some(),
    ),
];

#[test]
fn mission_info_closing_over_each_opener_shows_the_nav_its_abort_wrote() {
    for (what, open, still_open) in INFO_OPENERS {
        let store = MemoryPilots::new();
        let mut screen = AppScreen::new(aborting_to_beta_data())
            .with_dialogs(Rc::new(InfoOverEveryOpener), Rc::new(MonoMetrics))
            .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
            .appeared();
        create(&mut screen, "Ada");
        land_twice(&mut screen);
        screen.input(&key(Key::Enter, true));
        assert_eq!(active(&screen).len(), 2, "{what}: both accepted");
        open(&mut screen);
        assert!(still_open(spaceport(&screen)), "{what}: open");
        screen.input(&key(nova_view::ui::mission_info::MISSION_INFO_KEY, true));
        screen.input(&key(nova_view::ui::mission_info::MISSION_INFO_KEY, false));
        let abort = spaceport(&screen)
            .mission_info()
            .and_then(MissionInfoScreen::dialog)
            .and_then(|dialog| dialog.item_bounds(nova_view::spaceport::missions::ABORT_ITEM))
            .expect("Abort")
            .center();
        click_on(&mut screen, abort);
        assert_eq!(active(&screen).len(), 1, "{what}: one aborted");
        assert_eq!(
            flight(&screen).session().map(Session::nav_mode),
            Ok(nova_sim::NavMode::Stellar(Some(nova_sim::StellarId(129)))),
            "{what}: its M129 selected Beta Prime"
        );
        assert!(
            spaceport(&screen).mission_info().is_some(),
            "{what}: info open"
        );
        assert!(!names_beta(&screen), "{what}: held under it");
        screen.input(&key(Key::Escape, true));
        assert!(
            spaceport(&screen).mission_info().is_none(),
            "{what}: closed"
        );
        assert!(still_open(spaceport(&screen)), "{what}: still open");
        assert!(
            names_beta(&screen),
            "{what}: redrawn as it closes (@0x9e769)"
        );
    }
}

#[test]
fn a_fuel_tank_bought_in_port_leaves_the_bars_fuel_gauge_until_the_outfitter_closes() {
    let store = MemoryPilots::new();
    let mut screen = landed_outfitter_over(&store, barred_outfitting_data());
    edit(
        &mut screen,
        PilotEdit::Reserve(nova_sim::Reserve::Fuel, 150.0),
    );
    assert_eq!(fuel_width(&drawn(&screen)), Some(149.0), "landing's");
    click_port_item(&mut screen, 8);
    click_for_outfit(&mut screen, TANK, OUTFIT_BUY_ITEM);
    assert_eq!(pilot(&screen).owned(TANK), 1);
    assert_eq!(
        fuel_width(&drawn(&screen)),
        Some(149.0),
        "held while the outfitter is open"
    );
    let done = spaceport(&screen)
        .open_outfitter()
        .and_then(|open| open.dialog())
        .and_then(|dialog| dialog.item_bounds(1))
        .expect("Done")
        .center();
    click_on(&mut screen, done);
    assert!(spaceport(&screen).open_outfitter().is_none(), "closed");
    let width = fuel_width(&drawn(&screen)).expect("some fuel");
    assert!(width < 149.0, "redrawn as it closes: {width}");
}

const UPGRADE: OutfitId = OutfitId(134);

/// [`barred_outfitting_data`], where the outfitter also sells `oütf`
/// 134, an upgrade of no mod for 100 credits whose `OnPurchase` is
/// `H129`, and ship 129, carrying 300 fuel, is under gövt 128, whose
/// `Interface` is `ïntf` 129: [`bar_interface`] again.
fn barred_upgrading_data() -> Rc<GameData> {
    let mut upgrade = booster_record(b"H129");
    put_i16s(&mut upgrade, 0x02, &[0]);
    put_i16s(&mut upgrade, 0x06, &[0, 0]);
    upgrade[0x0E..0x12].copy_from_slice(&100_i32.to_be_bytes());
    let mut govt = vec![0; Govt::SIZE.expect("fixed")];
    put_i16s(&mut govt, 0xAC, &[129]);
    let fork = barred(game_data_fork(true, true, false, 100, b"", b"", None))
        .resource(Outfit::TYPE, 134, Some(b"Upgrade"), &upgrade)
        .resource(Govt::TYPE, 128, Some(b"Rebels"), &govt)
        .resource(Interface::TYPE, 129, None, &bar_interface());
    let mut built = fork.build();
    let first = built.layout.data_entry(Ship::TYPE, 128) + 4;
    built.put_u16(first + 0x0A, 300);
    let second = built.layout.data_entry(Ship::TYPE, 129) + 4;
    built.put_u16(second + 0x0A, 300);
    built.put_u16(second + 0x48, 128);
    loaded_bytes(built.bytes)
}

#[test]
fn an_upgrade_bought_in_port_onto_another_bar_redraws_its_fuel_gauge_at_once() {
    let store = MemoryPilots::new();
    let mut screen = landed_outfitter_over(&store, barred_upgrading_data());
    edit(
        &mut screen,
        PilotEdit::Reserve(nova_sim::Reserve::Fuel, 150.0),
    );
    assert_eq!(fuel_width(&drawn(&screen)), Some(149.0), "landing's");
    click_port_item(&mut screen, 8);
    click_for_outfit(&mut screen, UPGRADE, OUTFIT_BUY_ITEM);
    assert_eq!(pilot(&screen).ship(), ShipId(129), "upgraded");
    assert!(spaceport(&screen).open_outfitter().is_some(), "still open");
    let width = fuel_width(&drawn(&screen)).expect("some fuel");
    assert!(width < 149.0, "redrawn at once: {width}");
}
