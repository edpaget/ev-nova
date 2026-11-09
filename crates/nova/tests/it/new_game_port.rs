//! The new game's spaceport and screens over the checked-in fixture
//! (`nova_data::new_game::fixture::folder`), with no interface file: the
//! app built as `main` builds it, its dialogs what `nova::interface::wire`
//! gives the new game when the file doesn't open (the placeholder
//! dialogs), with the new game's rulebook and the Ink dialog loader.
//!
//! How each screen lays itself out over a template is pinned in
//! nova-view, and each placeholder's own facts in nova-data; this pins
//! that the two meet: each screen the new game opens is laid out over the
//! placeholders, not on its "Cannot show" line.
//!
//! - **Spaceport.** Landing at Hearth Dock, which has every service the
//!   format has, lays out the spaceport with its description and a button
//!   for each of its services and for Leave. The format has no mission
//!   service, but the Mission BBS is any inhabited stellar's, as the
//!   original's is (`nova_sim::landing::services`), so it has one too.
//! - **Services.** Bar, Trade Center, Outfitter and Shipyard each open
//!   from their button, their buttons labelled on their items, and close
//!   back to the spaceport through their own Leave or Done. The outfitter
//!   lists the fixture's kit; no hull is for sale in the fixture yet, so
//!   the shipyard opens over a variant of it with the cutter for sale.
//!   The Mission BBS, with no mission offered, says so and closes back.
//! - **Recharge.** A pilot short of fuel clicks Recharge: the tank fills,
//!   at `nova_sim::FUEL_PRICE_PER_UNIT` a unit.
//! - **Missions.** In port, I opens the missions log on the dock job.
//! - **The walk.** The main menu, New Pilot, flight and the map, with
//!   Find, warn of nothing (each frame asserts it).
//! - **Every placeholder.** Each screen the walk above doesn't reach is
//!   built from the placeholders and laid out.

use std::rc::Rc;

use nova::app::{Control, Showing, WindowEvent};
use nova::rulebook::ruleset_rulebook;
use nova::ruleset::Ruleset;
use nova::startup::{DataSource, DiskData, NewGameOpened};
use nova_data::new_game::fixture::folder;
use nova_data::new_game::kits::YARD_TECH_LEGAL;
use nova_data::records::ship::Ship;
use nova_data::{AnyRecord, Entry, GameData, InterfaceData, ShipId};
use nova_render::wgpu::GlyphonMetrics;
use nova_render::{Batch, FontFaces, Frame};
use nova_sim::fixture::MemoryPilots;
use nova_sim::mission::OfferLabels;
use nova_sim::storylines::state::MissionState;
use nova_sim::{
    ComputerRefusal, FUEL_PRICE_PER_UNIT, Gauge, InfoRow, MissionComputer, MissionId, MissionInfo,
    News, OfferPrompt, Pilot, PilotStore, RuleSource, Rulebook, Service, ShipFacts, ShipFields,
    ShipRow, ShipSpecs, Shipyard,
};
use nova_view::geometry::Point;
use nova_view::input::{Input, Key};
use nova_view::menu::MenuChoice;
use nova_view::spaceport::bar::LEAVE_ITEM as BAR_LEAVE_ITEM;
use nova_view::spaceport::layout::{LEAVE_ITEM, RECHARGE_ITEM, service_item};
use nova_view::spaceport::missions::{
    COMPUTER_DIALOG, INFO_DIALOG, MissionComputerScreen, MissionInfoScreen,
};
use nova_view::spaceport::news::{NEWS_DIALOG, NewsScreen};
use nova_view::spaceport::offer::{OFFER_DIALOG, OfferDialog};
use nova_view::spaceport::outfitter::{
    BUY_ITEM as OUTFIT_BUY_ITEM, DONE_ITEM as OUTFIT_DONE_ITEM, SELL_ITEM as OUTFIT_SELL_ITEM,
};
use nova_view::spaceport::race::{RACE_DIALOG, RaceScreen};
use nova_view::spaceport::shipyard::{
    BUY_ITEM as SHIPYARD_BUY_ITEM, DONE_ITEM as SHIPYARD_DONE_ITEM,
    INFO_ITEM as SHIPYARD_INFO_ITEM, InfoTemplates, SHIP_INFO_DIALOG, SHIPYARD_DIALOG,
    ShipyardScreen,
};
use nova_view::spaceport::trade::{
    BUY_ITEM as TRADE_BUY_ITEM, DONE_ITEM as TRADE_DONE_ITEM, SELL_ITEM as TRADE_SELL_ITEM,
};
use nova_view::spaceport::{ShipyardCatalog, SpaceportView};
use nova_view::text::TextMetrics;
use nova_view::ui::DialogResources;
use nova_view::ui::button::ButtonStyle;
use nova_view::ui::comm::{HAGGLE_DIALOG, HaggleDialog};
use nova_view::ui::dialog::{Dialog, DialogTemplate};
use nova_view::ui::note::{NOTE_DIALOG, NoteDialog, OK_ITEM as NOTE_OK_ITEM};
use nova_view::ui::quantity::QUANTITY_DIALOG;
use nova_view::ui::text_input::TEXT_INPUT_DIALOG;
use nova_view::ui::yes_no::{NO_ITEM, YES_ITEM};
use nova_view::{MenuCommand, MouseButton, Screen};
use winit::keyboard::KeyCode;

use crate::common::EntryHarness;
use crate::new_game_conversation::{land_at, new_pilot_flying, opened_from, resumed_in};
use crate::new_game_travel::{fixture, nova_of};
use crate::pilots::{Harness, saved, texts};

/// The app over the fixture, with no interface file, wired as `main`
/// wires the new game, keeping pilots in `store`, on the main menu.
fn app(store: &MemoryPilots) -> Harness {
    app_over(opened_from(&folder()), store)
}

/// [`app`] over `data`.
fn app_over(data: Rc<GameData>, store: &MemoryPilots) -> Harness {
    let rules = ruleset_rulebook(None, Ruleset::NewGame.profile().readings).0;
    Harness::wired_without_interface(data, store, |screen| {
        screen
            .with_rulebook(&rules)
            .with_dialog_loader(nova::dialogs::loader(1))
    })
}

/// Ada, a new pilot saved at once and changed by `edit`, resumed in a new
/// app, flying where she was saved.
fn resumed_after(edit: impl FnOnce(&mut Pilot)) -> (Harness, MemoryPilots) {
    resumed_over(&opened_from(&folder()), edit)
}

/// [`resumed_after`] over `data`.
fn resumed_over(data: &Rc<GameData>, edit: impl FnOnce(&mut Pilot)) -> (Harness, MemoryPilots) {
    let store = MemoryPilots::new();
    drop(new_pilot_flying(app_over(Rc::clone(data), &store)));
    let mut pilot = saved(&store, "Ada");
    edit(&mut pilot);
    store
        .write("Ada", &nova_sim::save::encode(&pilot))
        .expect("writes");
    (resumed_in(app_over(Rc::clone(data), &store)), store)
}

/// The fixture, opened as `main` opens it, with the cutter for sale at
/// every legal yard: a price, the legal yards' tech level and always on
/// offer. No hull is for sale in the fixture's own data until hulls gain
/// their prices and yards.
fn selling_the_cutter() -> Rc<GameData> {
    let NewGameOpened::Loaded(loaded) = DiskData.open_new_game(&folder()) else {
        panic!("the fixture opens");
    };
    let id = nova_of(&fixture().ids.ships, "cutter");
    let entry = loaded
        .data
        .get::<Ship>(id)
        .expect("present")
        .expect("decoded");
    let name = entry.name.map(str::to_owned);
    let cutter = Ship {
        cost: 5_000,
        tech_level: YARD_TECH_LEGAL,
        buy_random: 100,
        ..entry.record.clone()
    };
    Rc::new(loaded.data.with_decoded(Entry {
        id,
        name,
        record: AnyRecord::Ship(Box::new(cutter)),
    }))
}

/// Carrying the dock job, so Hearth Dock offers it no more.
fn on_the_dock_job(pilot: &mut Pilot) {
    pilot.story.missions.insert(
        "dock-job".to_owned(),
        MissionState::Active {
            cargo: 10,
            met: false,
        },
    );
    pilot.flags.set("hauling_plate");
}

/// A pilot on the dock job, landed at Hearth Dock.
fn at_hearth_dock() -> Harness {
    let (mut game, _store) = resumed_after(on_the_dock_job);
    land_at(&mut game, "hearth-dock");
    game
}

fn spaceport(game: &Harness) -> &SpaceportView {
    game.app.screen().spaceport_view().expect("landed")
}

/// Clicks the spaceport's item `item`.
fn click_port_item(game: &mut Harness, item: usize) {
    let at = spaceport(game)
        .dialog()
        .expect("laid out")
        .item_bounds(item)
        .expect("an item")
        .center();
    game.click(at);
}

fn pilot(game: &Harness) -> Pilot {
    game.app
        .screen()
        .flight_view()
        .expect("flying")
        .sim()
        .pilot()
        .expect("a pilot")
        .clone()
}

#[test]
fn hearth_dock_s_spaceport_is_laid_out_with_its_description_and_a_button_for_each_service() {
    let mut game = at_hearth_dock();
    let port = spaceport(&game);
    assert_eq!(port.problem(), None, "laid out");
    // The Mission BBS with them: any inhabited stellar has one.
    let offered = [
        Service::Bar,
        Service::TradeCenter,
        Service::Outfitter,
        Service::Shipyard,
        Service::MissionBbs,
    ];
    for service in offered {
        assert!(port.offered().contains(&service), "{service:?}");
    }
    let dialog = port.dialog().expect("laid out");
    for item in offered
        .map(service_item)
        .into_iter()
        .chain([LEAVE_ITEM, RECHARGE_ITEM])
    {
        assert!(dialog.item_shown(item), "item {item}");
    }
    let frame = game.frame();
    let drawn = texts(&frame);
    for label in [
        "Hearth Dock",
        "Bar",
        "Trade Center",
        "Outfitter",
        "Shipyard",
        "Mission BBS",
        "Recharge",
        "Leave",
    ] {
        assert!(drawn.contains(&label.to_owned()), "{label}: {drawn:?}");
    }
    let description = drawn.join(" ");
    assert!(description.contains("The Guild's great dock."), "{drawn:?}");
    assert_labelled(
        &frame,
        spaceport(&game).dialog(),
        &[
            (service_item(Service::Bar), "Bar"),
            (service_item(Service::TradeCenter), "Trade Center"),
            (service_item(Service::Outfitter), "Outfitter"),
            (service_item(Service::Shipyard), "Shipyard"),
            (service_item(Service::MissionBbs), "Mission BBS"),
            (RECHARGE_ITEM, "Recharge"),
            (LEAVE_ITEM, "Leave"),
        ],
    );

    click_port_item(&mut game, LEAVE_ITEM);
    assert_eq!(game.showing(), Showing::Flight, "Leave takes off");
}

/// What the spaceport has open over it, in a word.
fn open_over(game: &Harness) -> &'static str {
    let port = spaceport(game);
    if port.open_bar().is_some() {
        "bar"
    } else if port.open_trade().is_some() {
        "trade"
    } else if port.open_outfitter().is_some() {
        "outfitter"
    } else if port.open_shipyard().is_some() {
        "shipyard"
    } else if port.mission_computer().is_some() {
        "computer"
    } else if port.mission_text().is_some() {
        "text"
    } else if port.open_service().is_some() {
        "service"
    } else {
        "nothing"
    }
}

/// Each opens over its placeholder, its buttons labelled on their items,
/// and closes back to the spaceport through its own Leave or Done.
#[test]
fn the_bar_and_the_trade_center_open_from_their_buttons_and_close_back_through_their_own() {
    let mut game = at_hearth_dock();
    click_port_item(&mut game, service_item(Service::Bar));
    let frame = game.frame();
    assert_eq!(open_over(&game), "bar");
    let bar = spaceport(&game).open_bar().expect("open");
    assert_eq!(bar.problem(), None, "laid out");
    assert_labelled(&frame, bar.dialog(), &[(BAR_LEAVE_ITEM, "Leave")]);
    let leave = centre_of(bar.dialog(), BAR_LEAVE_ITEM);
    game.click(leave);
    game.frame();
    assert_eq!(open_over(&game), "nothing", "the bar closed");
    assert_eq!(game.showing(), Showing::Spaceport);

    let mut game = at_hearth_dock();
    click_port_item(&mut game, service_item(Service::TradeCenter));
    let frame = game.frame();
    assert_eq!(open_over(&game), "trade");
    let trade = spaceport(&game).open_trade().expect("open");
    assert_eq!(trade.problem(), None, "laid out");
    assert_labelled(
        &frame,
        trade.dialog(),
        &[
            (TRADE_DONE_ITEM, "Done"),
            (TRADE_BUY_ITEM, "Buy"),
            (TRADE_SELL_ITEM, "Sell"),
        ],
    );
    let done = centre_of(trade.dialog(), TRADE_DONE_ITEM);
    game.click(done);
    game.frame();
    assert_eq!(open_over(&game), "nothing", "the trade center closed");
    assert_eq!(game.showing(), Showing::Spaceport);
}

/// The centre of `dialog`'s item `item`.
fn centre_of(dialog: Option<&Dialog>, item: usize) -> Point {
    dialog
        .expect("laid out")
        .item_bounds(item)
        .expect("an item")
        .center()
}

/// Asserts that `frame` draws each label of `labels` on its item of
/// `dialog`: the label's run starts inside the item's bounds.
fn assert_labelled(frame: &Frame, dialog: Option<&Dialog>, labels: &[(usize, &str)]) {
    let dialog = dialog.expect("laid out");
    let runs: Vec<_> = frame
        .batches
        .iter()
        .flat_map(|batch| match batch {
            Batch::Text(runs) => runs.clone(),
            _ => Vec::new(),
        })
        .collect();
    for &(item, label) in labels {
        let bounds = dialog.item_bounds(item).expect("an item");
        let on_it = runs.iter().any(|run| {
            let (x, y) = run.origin_px;
            run.text == label && bounds.contains(Point::new(x, y))
        });
        assert!(on_it, "{label:?} on item {item} at {bounds:?}");
    }
}

/// Over the fixture Hearth Dock's outfitter lists its one kit, which fits
/// the hauler (`mods/cargo-racks.md`), so the button opens the outfitter
/// over its placeholder, its buttons labelled on their items, and Done
/// closes it back to the spaceport.
#[test]
fn the_outfitter_opens_from_its_button_and_its_done_closes_it_back() {
    let mut game = at_hearth_dock();
    click_port_item(&mut game, service_item(Service::Outfitter));
    let frame = game.frame();
    assert_eq!(open_over(&game), "outfitter");
    let outfitter = spaceport(&game).open_outfitter().expect("open");
    assert_eq!(outfitter.problem(), None, "laid out");
    assert_labelled(
        &frame,
        outfitter.dialog(),
        &[
            (OUTFIT_DONE_ITEM, "Done"),
            (OUTFIT_BUY_ITEM, "Buy"),
            (OUTFIT_SELL_ITEM, "Sell"),
        ],
    );

    let done = centre_of(outfitter.dialog(), OUTFIT_DONE_ITEM);
    game.click(done);
    game.frame();
    assert_eq!(open_over(&game), "nothing", "closed");
    assert_eq!(game.showing(), Showing::Spaceport);
}

/// Over the fixture with the cutter for sale, the button opens the
/// shipyard over its placeholder, its buttons labelled on their items,
/// and Done closes it back to the spaceport.
#[test]
fn the_shipyard_opens_from_its_button_and_its_done_closes_it_back() {
    let (mut game, _store) = resumed_over(&selling_the_cutter(), on_the_dock_job);
    land_at(&mut game, "hearth-dock");
    click_port_item(&mut game, service_item(Service::Shipyard));
    let frame = game.frame();
    assert_eq!(open_over(&game), "shipyard");
    let shipyard = spaceport(&game).open_shipyard().expect("open");
    assert_eq!(shipyard.problem(), None, "laid out");
    assert_labelled(
        &frame,
        shipyard.dialog(),
        &[
            (SHIPYARD_DONE_ITEM, "Done"),
            (SHIPYARD_BUY_ITEM, "Buy Ship"),
            (SHIPYARD_INFO_ITEM, "Info"),
        ],
    );

    let done = centre_of(shipyard.dialog(), SHIPYARD_DONE_ITEM);
    game.click(done);
    game.frame();
    assert_eq!(open_over(&game), "nothing", "closed");
    assert_eq!(game.showing(), Showing::Spaceport);
}

/// Over the fixture no mission is offered on the computer, so the Mission
/// BBS says so in a text over the spaceport, laid out by the placeholder
/// text dialog, rather than opening; the text closes back to the
/// spaceport. The computer's own placeholder is built in
/// [`every_placeholder_the_walk_does_not_reach_lays_its_screen_out`].
#[test]
fn the_mission_bbs_says_there_is_nothing_here_and_closes_back() {
    let none_here = ComputerRefusal::NoneAvailable.message();
    let mut game = at_hearth_dock();
    click_port_item(&mut game, service_item(Service::MissionBbs));
    let drawn = texts(&game.frame());
    assert_eq!(open_over(&game), "text");
    assert!(drawn.contains(&none_here), "{drawn:?}");

    game.press(KeyCode::Enter);
    game.frame();
    assert_eq!(open_over(&game), "nothing", "closed");
    assert_eq!(game.showing(), Showing::Spaceport);
}

#[test]
fn recharge_fills_the_tank_at_the_price_of_a_unit() {
    let (mut game, store) = resumed_after(|pilot| {
        on_the_dock_job(pilot);
        pilot.reserves.fuel.now = pilot.reserves.fuel.max - 100.0;
    });
    land_at(&mut game, "hearth-dock");
    let before = pilot(&game);
    assert!(before.reserves.fuel.now < before.reserves.fuel.max);

    click_port_item(&mut game, RECHARGE_ITEM);
    let after = pilot(&game);
    let full = Gauge {
        now: before.reserves.fuel.max,
        ..before.reserves.fuel
    };
    assert_eq!(after.reserves.fuel, full, "filled");
    assert_eq!(after.cash, before.cash - 100 * FUEL_PRICE_PER_UNIT);
    assert_eq!(saved(&store, "Ada"), after, "saved");
    assert_eq!(game.showing(), Showing::Spaceport);
}

#[test]
fn i_in_port_opens_the_missions_log_and_it_closes_back_to_the_spaceport() {
    let mut game = at_hearth_dock();
    game.press(KeyCode::KeyI);
    let drawn = texts(&game.frame());
    for line in ["Missions", "Hull plate for Verge Camp"] {
        assert!(drawn.contains(&line.to_owned()), "{line}: {drawn:?}");
    }
    game.press(KeyCode::Enter);
    let drawn = texts(&game.frame());
    assert!(
        !drawn.contains(&"Hull plate for Verge Camp".to_owned()),
        "closed: {drawn:?}"
    );
    assert_eq!(game.showing(), Showing::Spaceport);
}

/// Presses M until the course map opens, as a take-off holds the key a
/// while.
fn open_the_map(game: &mut Harness) {
    for _ in 0..600 {
        if game.app.screen().flight_view().expect("flying").map_open() {
            return;
        }
        game.press(KeyCode::KeyM);
        game.frame();
    }
    panic!("the map never opens");
}

/// Each frame of the walk asserts that nothing failed but a missing
/// picture, which the placeholders have none of, and that nothing was
/// warned of ([`Harness::frame`]).
#[test]
fn the_main_menu_new_pilot_flight_and_the_map_with_find_warn_of_nothing() {
    let mut game = app(&MemoryPilots::new());
    game.frame();
    assert_eq!(game.showing(), Showing::MainMenu);

    let new_pilot = game.menu_button(MenuChoice::NewPilot);
    game.click(new_pilot);
    game.frame();
    assert!(game.app.screen().new_pilot().is_some(), "New Pilot opens");
    game.type_name("Ada");
    game.press(KeyCode::Enter);
    game.frame();
    assert!(game.app.screen().christening().is_some(), "the christening");
    game.press(KeyCode::Enter);
    game.enter_ship();
    game.frame();
    assert_eq!(game.showing(), Showing::Flight);

    open_the_map(&mut game);
    game.press(KeyCode::KeyF);
    game.frame();
    let map = game
        .app
        .screen()
        .flight_view()
        .expect("flying")
        .course_map();
    assert!(map.find_dialog().is_some(), "Find opens");
    game.press(KeyCode::Escape);
    game.frame();
}

/// The placeholders, as `main` gives the new game them.
fn placeholders() -> Rc<dyn DialogResources> {
    let no_file: Result<InterfaceData, _> = Err("no interface file");
    nova::interface::wire(Ruleset::NewGame, no_file)
        .dialogs
        .expect("the placeholders")
}

fn metrics() -> Rc<dyn TextMetrics> {
    Rc::new(GlyphonMetrics::new(&FontFaces::bundled()))
}

/// Placeholder `id`, laid out by the real metrics.
fn layout(id: i16) -> Result<(DialogTemplate, Rc<dyn TextMetrics>), String> {
    Ok((placeholders().dialog_template(id)?, metrics()))
}

fn template(id: i16) -> DialogTemplate {
    placeholders().dialog_template(id).expect("a placeholder")
}

/// The fixture's data, for the screens that read pictures and names.
fn fixture_data() -> Rc<GameData> {
    opened_from(&folder())
}

/// A shipyard listing the hauler alone, with nothing to pay.
fn one_ship_shipyard() -> Shipyard {
    Shipyard {
        rows: vec![ShipRow {
            id: ShipId(128),
            name: "Hauler".to_owned(),
            short_name: "Hauler".to_owned(),
            price: 0,
            specs: ShipSpecs {
                fields: ShipFields::default(),
                max_gun: 0,
                max_tur: 0,
                length: 0,
                crew: 0,
                space: 0,
                rounding: RuleSource::Engine,
                weapons: Vec::new(),
                long_name: "Hauler".to_owned(),
            },
            facts: ShipFacts::default(),
        }],
        trade_in: 0,
        cash: 0,
        current: ShipId(128),
        rules: Rulebook::default(),
    }
}

fn key(key: Key, pressed: bool) -> Input {
    Input::Key {
        key,
        pressed,
        repeat: false,
    }
}

fn click_on(screen: &mut impl Screen, at: Point) {
    for pressed in [true, false] {
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        });
    }
}

/// Each placeholder screen the walks above don't lay out, built from the
/// placeholders `main` gives the new game, as the router builds it: laid
/// out, not on its "Cannot show" line, and the shipyard's ship info, which
/// its Info opens. The quantity dialog (1003) and the
/// text input (3001) are the view's own layouts; the haggle dialog (1008)
/// stands for the built-in flight dialogs.
#[test]
fn every_placeholder_the_walk_does_not_reach_lays_its_screen_out() {
    let style = ButtonStyle::STOCK;
    let data = fixture_data();

    let computer = MissionComputer {
        rows: Vec::new(),
        today: "1 January".to_owned(),
    };
    let computer = MissionComputerScreen::new(layout(COMPUTER_DIALOG), computer, style);
    assert_eq!(computer.problem(), None, "the mission computer");
    assert!(computer.dialog().is_some());

    let row = InfoRow {
        slot: 0,
        name: "A job".to_owned(),
        failed: false,
        text: "Do it.".to_owned(),
        abortable: true,
    };
    let info = MissionInfo::shown(vec![row], "1 January");
    let info = MissionInfoScreen::new(layout(INFO_DIALOG), info, style);
    assert_eq!(info.problem(), None, "the missions log");

    let prompt = OfferPrompt {
        mission: MissionId(128),
        text: "Take the job?".to_owned(),
        labels: OfferLabels {
            accept: "Accept".to_owned(),
            refuse: Some("Refuse".to_owned()),
        },
        graphic: None,
        movie: None,
    };
    let offer = OfferDialog::new(layout(OFFER_DIALOG), prompt, style);
    assert_eq!(offer.problem(), None, "the offer");

    let news = NewsScreen::new(layout(NEWS_DIALOG), &News::default(), None);
    assert_eq!(news.problem(), None, "the news");

    let race = RaceScreen::new(
        layout(RACE_DIALOG),
        Some(template(QUANTITY_DIALOG)),
        style,
        100,
    );
    assert_eq!(race.problem(), None, "the race");

    let note = NoteDialog::new(&template(NOTE_DIALOG), "A note.", style, metrics());
    assert!(note.dialog().item_shown(NOTE_OK_ITEM), "the note");

    // The shipyard is walked above; built here for its ship info (1005).
    let mut shipyard = ShipyardScreen::new(
        layout(SHIPYARD_DIALOG),
        InfoTemplates {
            plain: placeholders().dialog_template(SHIP_INFO_DIALOG),
            picture: Err("no art".to_owned()),
        },
        placeholders().dialog_template(TEXT_INPUT_DIALOG),
        one_ship_shipyard(),
        Rc::clone(&data) as Rc<dyn ShipyardCatalog>,
        style,
    );
    assert_eq!(shipyard.problem(), None, "the shipyard");
    shipyard.input(&key(Key::ArrowRight, true));
    shipyard.input(&key(Key::ArrowRight, false));
    let info_button = shipyard
        .dialog()
        .expect("laid out")
        .item_bounds(SHIPYARD_INFO_ITEM)
        .expect("Info")
        .center();
    click_on(&mut shipyard, info_button);
    assert!(shipyard.info_panel().is_some(), "the ship info");

    let haggle = HaggleDialog::new(&template(HAGGLE_DIALOG), 100, true, style, metrics());
    assert!(haggle.is_ok(), "the haggle dialog");
}

/// The router's own placeholder dialogs, opened through the app: the
/// main menu's close question, Preferences and the player info. (The text
/// dialog is laid out by the spaceport's texts above.)
#[test]
fn the_question_preferences_and_player_info_open_through_the_app() {
    let store = MemoryPilots::new();
    let mut game = app(&store);
    game.frame();
    assert_eq!(game.handle(WindowEvent::CloseRequested), Control::Continue);
    game.frame();
    let question = game.app.screen().question().expect("the close question");
    for item in [YES_ITEM, NO_ITEM] {
        assert!(question.dialog().item_shown(item), "item {item}");
    }
    game.press(KeyCode::Escape);
    game.frame();

    let mut game = new_pilot_flying(app(&store));
    assert_eq!(
        game.handle(WindowEvent::Menu(MenuCommand::Preferences)),
        Control::Continue
    );
    game.frame();
    assert!(game.app.screen().preferences().is_some(), "Preferences");
    game.press(KeyCode::Enter);
    game.frame();

    game.press(KeyCode::KeyP);
    game.frame();
    let info = game
        .app
        .screen()
        .flight_player_info()
        .expect("the player info");
    assert_eq!(info.problem(), None);
}
