use super::*;

// The nav area while docked.

/// Outfit 201: [`outfitting`]'s fuel tank with no `OnPurchase`.
const BUY_PLAIN_TANK: OutfitOrder = OutfitOrder {
    outfit: OutfitId(201),
    direction: Direction::Buy,
};

/// A view of `catalog` landed on Earth in mode 2, settled, with the
/// HUD's cadence read as `cadence` says.
fn docked_at_earth(catalog: FakeCatalog, cadence: RuleSource) -> View {
    let rules = Rulebook::default().with_override(RuleKey::HudCadence, cadence);
    let mut view = FlightView::new(catalog).with_rules(rules);
    land_now(&mut view);
    view.settle_script();
    assert_eq!(
        view.session().map(Session::nav_mode),
        Ok(NavMode::Stellar(Some(StellarId(128))))
    );
    assert_eq!(port_nav(&view), naming("Earth"));
    view
}

#[test]
fn by_the_bible_the_port_names_a_landed_ms_stellar_outside_the_scene_at_once() {
    let mut view = docked_at_earth(port(), RuleSource::Bible);
    run_set(&mut view, "M131");
    assert_eq!(port_nav(&view), naming("Proxima"), "the session's name");
    assert_eq!(
        view.scene().map(SystemScene::id),
        Some(SystemId(130)),
        "Sol's scene"
    );
    run_set(&mut view, "M132");
    assert_eq!(port_nav(&view), naming(hud::NAV_NO_DESTINATION));
}

/// An accept's and a refusal's redraws are pinned by
/// [`a_dialogs_redraw_shows_what_the_script_run_with_it_wrote`], as a
/// move empties the offers.
#[test]
fn by_the_engine_a_landed_ms_stellar_shows_at_the_ports_next_nav_redraw() {
    type Step = fn(&mut View);
    let redraws: [(&str, Step); 6] = [
        ("a shop closing", FlightView::refresh_status),
        ("a name prompt's Cancel", |view| {
            view.decline_ship(ShipId(129));
        }),
        ("a refused purchase, after its prompt's close", |view| {
            assert!(view.buy_ship(ShipId(999), "Kestrel").is_err());
        }),
        ("a mission text closing", |view| {
            run_set(view, "S131");
            assert_eq!(port_nav(view), naming("Earth"), "queued, not shown");
            assert_eq!(view.take_mission_texts().len(), 1);
            assert_eq!(port_nav(view), naming("Earth"), "shown over the bar");
            view.close_mission_text();
        }),
        ("a ship purchase", |view| {
            view.buy_ship(ShipId(129), "Kestrel").expect("bought");
        }),
        ("an outfitter order changing the ïntf", |view| {
            assert_eq!(view.outfit(BUY_TANK), Ok(()));
            assert_eq!(view.session().map(Session::ship), Ok(ShipId(130)));
        }),
    ];
    let held: [(&str, Step); 3] = [
        ("a trade", |view| {
            assert_eq!(view.trade(BUY_FOOD), Ok(10));
        }),
        ("a recharge", |view| {
            desk_reserve(view, nova_sim::Reserve::Fuel, 100.0);
            assert!(view.recharge().is_ok());
        }),
        ("an outfitter order keeping the ïntf", |view| {
            assert_eq!(view.outfit(BUY_PLAIN_TANK), Ok(()));
        }),
    ];
    for (steps, shown) in [(&redraws[..], "Proxima"), (&held[..], "Earth")] {
        for (what, step) in steps {
            let mut view = docked_at_earth(port(), RuleSource::Engine);
            run_set(&mut view, "M131");
            view.settle_script();
            assert_eq!(
                view.session().map(Session::nav_mode),
                Ok(NavMode::Stellar(Some(StellarId(140))))
            );
            assert_eq!(port_nav(&view), naming("Earth"), "{what}: held");
            step(&mut view);
            assert_eq!(port_nav(&view), naming(shown), "{what}");
        }
    }
}

/// [`docked_at_earth`] by the engine's reading, with `dialog` opened,
/// a landed `M131` run and settled inside it, and `dialog` closed.
fn m_inside(dialog: PortDialog) -> View {
    m_inside_of(port(), dialog)
}

/// [`m_inside`] over `catalog`.
fn m_inside_of(catalog: FakeCatalog, dialog: PortDialog) -> View {
    let mut view = docked_at_earth(catalog, RuleSource::Engine);
    view.open_port_dialog();
    run_set(&mut view, "M131");
    view.settle_script();
    assert_eq!(
        view.session().map(Session::nav_mode),
        Ok(NavMode::Stellar(Some(StellarId(140)))),
        "{dialog:?}: written"
    );
    view.close_port_dialog(dialog);
    assert_eq!(
        view.session().map(Session::nav_mode),
        Ok(NavMode::Stellar(Some(StellarId(128)))),
        "{dialog:?}: restored"
    );
    view
}

#[test]
fn the_bars_map_closing_shows_the_maps_selection_and_puts_back_the_bars_nav() {
    // The planner's click writes the nav in port, which the close shows.
    let mut view = planner(in_the_bar(None));
    view.open_bar_map();
    assert!(view.map_open());
    assert_eq!(view.course_map().mode(), MapMode::Course);
    port_click(&mut view, 131);
    assert!(view.map_open(), "a click doesn't close it");
    assert_eq!(
        view.session().map(Session::nav_mode),
        Ok(NavMode::Hyperspace(Some(SystemId(131)))),
        "the click cues the route"
    );
    view.port_map_input(&key(MAP, true));
    assert!(!view.map_open(), "M closes it");
    assert_eq!(port_nav(&view), cueing_alpha(), "the map's selection drawn");
    assert_eq!(
        view.session().map(Session::nav_mode),
        Ok(NavMode::Stellar(Some(StellarId(128)))),
        "the bar's nav put back"
    );
    assert_eq!(
        view.session().map(|session| session.course().to_vec()),
        Ok(vec![SystemId(131)]),
        "the course kept"
    );
    view.refresh_status();
    assert_eq!(port_nav(&view), naming("Earth"), "the bar's close");
}

/// Only the bar's own map puts back the bar's nav: a map opened in
/// port some other way (the port screens' have restores of their own,
/// [`FlightView::open_port_screen_map`]) leaves the nav its click cued, with the bar's save still held,
/// even after the bar's own map has opened and closed.
#[test]
fn a_map_in_port_the_bar_did_not_open_leaves_the_bars_nav_saved() {
    // The planner's click writes the nav in port, which the close shows.
    let mut view = planner(in_the_bar(None));
    view.open_bar_map();
    view.port_map_input(&key(MAP, true));
    assert!(!view.map_open(), "the bar's map closed");
    view.open_map();
    port_click(&mut view, 131);
    view.port_map_input(&key(MAP, true));
    assert!(!view.map_open());
    assert_eq!(port_nav(&view), cueing_alpha(), "the map's selection drawn");
    assert_eq!(
        view.session().map(Session::nav_mode),
        Ok(NavMode::Hyperspace(Some(SystemId(131)))),
        "the bar's nav not put back"
    );
    assert_eq!(
        view.session().map(|session| session.course().to_vec()),
        Ok(vec![SystemId(131)]),
        "the course kept"
    );
}

#[test]
fn the_bars_map_key_repeat_or_release_leaves_it_open() {
    let mut view = in_the_bar(None);
    view.open_bar_map();
    view.port_map_input(&Input::Key {
        key: MAP,
        pressed: true,
        repeat: true,
    });
    view.port_map_input(&key(MAP, false));
    assert!(view.map_open());
}

/// The map's beeps are `_HandlePlayer`'s, around the map M opens in
/// flight (@0x6a809, @0x6a85a): the map opened in port makes neither.
#[test]
fn the_map_in_port_beeps_neither_as_it_opens_nor_as_it_closes() {
    let mut view = in_the_bar(None);
    view.take_sounds();
    view.open_bar_map();
    assert_eq!(map_sounds(&mut view), [], "the bar's map opened");
    view.port_map_input(&key(MAP, true));
    assert!(!view.map_open());
    assert_eq!(map_sounds(&mut view), [], "the bar's map closed");
    view.open_map();
    view.close_map();
    assert_eq!(map_sounds(&mut view), [], "another map in port");
}

/// The port close skips the cue (`_DoSystemMap` @0x13be2-0x13c0a): a
/// course plotted before the map opened, whose cue the land key put
/// mode 2 over, stays uncued, as does one Shift-clicked in port.
#[test]
fn in_port_the_maps_close_cues_no_course() {
    let mut view = in_the_bar(Some(131));
    view.open_bar_map();
    view.port_map_input(&key(MAP, true));
    assert!(!view.map_open());
    assert_eq!(port_nav(&view), naming("Earth"), "no cue drawn");
    assert_eq!(
        view.session().map(Session::nav_mode),
        Ok(NavMode::Stellar(Some(StellarId(128))))
    );
    assert_eq!(
        view.session().map(|session| session.course().to_vec()),
        Ok(vec![SystemId(131)]),
        "the course kept"
    );
}

#[test]
fn an_empty_outfitter_draws_a_nav_the_shipyard_left_marked() {
    let mut view = m_inside_of(port(), PortDialog::Shipyard);
    assert_eq!(port_nav(&view), naming("Proxima"), "held after the close");
    view.empty_outfitter_returned();
    assert_eq!(port_nav(&view), naming("Earth"), "the restore drawn");
    run_set(&mut view, "M131");
    view.settle_script();
    view.empty_outfitter_returned();
    assert_eq!(port_nav(&view), naming("Earth"), "nothing marked");
}

#[test]
fn a_landed_m_in_the_outfitter_exchange_or_computer_is_undone_and_shown_at_once() {
    for dialog in [
        PortDialog::Outfitter,
        PortDialog::Exchange,
        PortDialog::MissionComputer,
    ] {
        let view = m_inside(dialog);
        assert_eq!(port_nav(&view), naming("Earth"), "{dialog:?}");
    }
}

#[test]
fn the_mission_computers_close_redraws_the_whole_bar() {
    let mut view = docked_at_earth(port(), RuleSource::Engine);
    run_set(&mut view, "M131");
    view.settle_script();
    desk_reserve(&mut view, nova_sim::Reserve::Fuel, 100.0);
    assert_eq!(port_nav(&view), naming("Earth"), "held");
    assert_ne!(
        view.reserves_shown(&view.frame_now()).fuel.now,
        100.0,
        "held"
    );
    view.open_port_dialog();
    view.close_port_dialog(PortDialog::MissionComputer);
    assert_eq!(
        port_nav(&view),
        naming("Proxima"),
        "redrawn, though the restore changed nothing"
    );
    assert_eq!(
        view.reserves_shown(&view.frame_now()).fuel.now,
        100.0,
        "redrawn"
    );
}

#[test]
fn by_the_engine_a_landed_n_shows_no_destination_at_the_next_redraw() {
    let mut view = docked_at_earth(port(), RuleSource::Engine);
    run_set(&mut view, "N131");
    view.settle_script();
    assert_eq!(port_nav(&view), naming("Earth"), "held");
    view.refresh_status();
    assert_eq!(port_nav(&view), naming(hud::NAV_NO_DESTINATION));
}

#[test]
fn a_comp_text_shows_the_landed_stellar_and_the_next_redraw_its_on_successs_move() {
    let mut catalog = port();
    catalog.missions = vec![MissionRecord {
        travel_stel: 128,
        on_success: nova_sim::Script::parse("M131"),
        ..errand(130)
    }];
    catalog.mission_texts = vec![MissionTexts {
        comp: Some("Done.".to_owned()),
        ..MissionTexts::named(MissionId(130), "Run")
    }];
    let mut view = docked_at_earth(catalog, RuleSource::Engine);
    run_set(&mut view, "S130");
    view.take_off().expect("takes off");
    view.settle_script();
    land_now(&mut view);
    view.settle_script();
    assert_eq!(
        view.session().map(Session::nav_mode),
        Ok(NavMode::Stellar(Some(StellarId(140)))),
        "OnSuccess ran"
    );
    assert_eq!(view.take_mission_texts().len(), 1);
    assert_eq!(port_nav(&view), naming("Earth"), "as the text noted it");
    view.refresh_status();
    assert_eq!(port_nav(&view), naming("Proxima"));
}

#[test]
fn a_dialogs_redraw_shows_what_the_script_run_with_it_wrote() {
    type Fit = fn(&mut FakeCatalog);
    type Step = fn(&mut View);
    let rows: [(&str, Fit, Step); 5] = [
        (
            "OnAccept",
            |catalog| catalog.missions[0].on_accept = nova_sim::Script::parse("M131"),
            |view| assert_eq!(view.accept_mission(MissionId(130)), Ok(0)),
        ),
        (
            "OnRefuse",
            |catalog| catalog.missions[0].on_refuse = nova_sim::Script::parse("M131"),
            |view| assert_eq!(view.refuse_mission(MissionId(130)).map(|_| ()), Ok(())),
        ),
        (
            "a RefuseText before OnRefuse",
            |catalog| {
                catalog.missions[0].on_refuse = nova_sim::Script::parse("M131");
                catalog.mission_texts = vec![MissionTexts {
                    refuse: Some("No.".to_owned()),
                    ..MissionTexts::named(MissionId(130), "Run")
                }];
            },
            |view| {
                assert_eq!(view.refuse_mission(MissionId(130)).map(|_| ()), Ok(()));
                assert_eq!(view.take_mission_texts().len(), 1);
                view.close_mission_text();
            },
        ),
        (
            "a BriefText after OnAccept",
            |catalog| {
                catalog.missions[0].on_accept = nova_sim::Script::parse("M131");
                catalog.mission_texts = vec![MissionTexts {
                    brief: Some("Go.".to_owned()),
                    ..MissionTexts::named(MissionId(130), "Run")
                }];
            },
            |view| {
                assert_eq!(view.accept_mission(MissionId(130)), Ok(0));
                assert_eq!(view.take_mission_texts().len(), 1);
                view.close_mission_text();
            },
        ),
        (
            "a ship's OnPurchase",
            |catalog| {
                let ship = catalog
                    .ships
                    .iter_mut()
                    .find(|ship| ship.id == ShipId(129))
                    .expect("ship 129");
                ship.on_purchase = nova_sim::Script::parse("M131");
            },
            |view| {
                view.buy_ship(ShipId(129), "Kestrel").expect("bought");
            },
        ),
    ];
    for (what, fit, step) in rows {
        let mut catalog = port();
        fit(&mut catalog);
        let mut view = docked_at_earth(catalog, RuleSource::Engine);
        step(&mut view);
        assert_eq!(port_nav(&view), naming("Proxima"), "{what}");
    }
}

#[test]
fn a_text_never_shows_over_a_later_redraw() {
    let mut catalog = port();
    catalog.missions[0].on_accept = nova_sim::Script::parse("M131");
    catalog.mission_texts = vec![MissionTexts {
        brief: Some("Go.".to_owned()),
        ..MissionTexts::named(MissionId(130), "Run")
    }];
    let mut view = docked_at_earth(catalog, RuleSource::Engine);
    assert_eq!(view.accept_mission(MissionId(130)), Ok(0));
    run_set(&mut view, "M132");
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

// A docked mission script's `C`, `E` and `H`, and the bar under its
// texts.

/// [`port`] fitted by `fit`, docked at Earth by the engine's cadence.
fn docked_scripted(fit: impl FnOnce(&mut FakeCatalog)) -> View {
    let mut catalog = port();
    fit(&mut catalog);
    docked_at_earth(catalog, RuleSource::Engine)
}

/// An `H` that puts the ship under another `ïntf` (ship 130, under
/// 129), and one that keeps it (ship 129, under 128 as ship 128 is).
const INTF_ROWS: [(&str, bool); 2] = [("H130", true), ("H129", false)];

#[test]
fn a_docked_on_abort_intf_change_redraws_the_whole_bar_at_the_operator() {
    for (h, changes) in INTF_ROWS {
        let mut view = docked_scripted(|catalog| {
            catalog.missions[0].on_abort = nova_sim::Script::parse(&format!("{h} M131"));
        });
        assert_eq!(view.accept_mission(MissionId(130)), Ok(0));
        stale_bar(&mut view);
        view.abort_mission(0).expect("aborted");
        if changes {
            assert_eq!(
                retaken(&view),
                [true, false, true, true],
                "{h}: the whole bar, the nav from before the M"
            );
            assert_eq!(port_nav(&view), naming("Earth"), "{h}");
        } else {
            assert_eq!(retaken(&view), [false; 4], "{h}: held");
        }
    }
}

#[test]
fn a_docked_jettisons_on_failure_intf_change_redraws_the_whole_bar_at_the_operator() {
    for (h, changes) in INTF_ROWS {
        let mut view = docked_scripted(|catalog| {
            let mission = &mut catalog.missions[0];
            mission.cargo_type = 0;
            mission.cargo_qty = 1;
            mission.pickup_mode = 0;
            mission.on_failure = nova_sim::Script::parse(&format!("{h} M131"));
        });
        assert_eq!(view.accept_mission(MissionId(130)), Ok(0));
        stale_bar(&mut view);
        view.jettison_all();
        assert_eq!(
            view.session()
                .map(|session| session.pilot().missions().missions().len()),
            Ok(0),
            "{h}: failed"
        );
        if changes {
            assert_eq!(retaken(&view), [true, false, true, true], "{h}");
        } else {
            assert_eq!(retaken(&view), [false; 4], "{h}: held");
        }
    }
}

#[test]
fn a_brief_text_shows_over_the_bar_as_on_accept_left_it() {
    for (h, changes) in INTF_ROWS {
        for how in [Accepted::Computer, Accepted::Offer, Accepted::AtOnce] {
            let what = format!("{h}, {how:?}");
            let mut view = docked_scripted(|catalog| {
                let mission = &mut catalog.missions[1];
                mission.on_accept = nova_sim::Script::parse(&format!("{h} G201 M131"));
                if how != Accepted::Computer {
                    mission.avail_loc = SPACEPORT;
                }
                if how == Accepted::AtOnce {
                    mission.flags = nova_sim::mission::screens::UNREFUSABLE;
                }
            });
            assert_eq!(
                view.session()
                    .map(|session| session.refusable_offer_dialog(MissionId(131))),
                Ok(how == Accepted::Offer),
                "{what}"
            );
            let before = view.reserves();
            stale_bar(&mut view);
            if how == Accepted::AtOnce {
                assert!(
                    matches!(
                        view.next_offer(SPACEPORT),
                        Some(VenueOffer::Accepted {
                            accepted: Ok(0),
                            ..
                        })
                    ),
                    "{what}"
                );
            } else {
                assert_eq!(view.accept_mission(MissionId(131)), Ok(0), "{what}");
            }
            assert_eq!(view.take_mission_texts().len(), 1, "{what}: the brief");
            let shown = view.reserves_shown(&view.frame_now());
            match (changes, how) {
                (true, _) => {
                    assert_eq!(
                        shown.fuel.max,
                        view.reserves().fuel.max - 100.0,
                        "{what}: as at the H, before G201's tank"
                    );
                    assert_eq!(port_nav(&view), naming("Earth"), "{what}: before the M");
                }
                (false, Accepted::Offer) => {
                    assert_eq!(shown, before, "{what}: the dialog's redraw before it");
                    assert_eq!(port_nav(&view), naming("Earth"), "{what}");
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
    for script in ["M131 G201", "H130 M131 G201"] {
        let mut view = docked_scripted(|catalog| {
            catalog.missions[0].avail_loc = SPACEPORT;
            catalog.missions[0].on_refuse = nova_sim::Script::parse(script);
            catalog.mission_texts.push(MissionTexts {
                refuse: Some("No.".to_owned()),
                ..MissionTexts::named(MissionId(130), "Run")
            });
        });
        let before = view.reserves();
        stale_bar(&mut view);
        assert_eq!(view.refuse_mission(MissionId(130)).map(|_| ()), Ok(()));
        assert_eq!(view.take_mission_texts().len(), 1, "{script}");
        assert_eq!(
            view.reserves_shown(&view.frame_now()),
            before,
            "{script}: before OnRefuse"
        );
        assert_eq!(port_nav(&view), naming("Earth"), "{script}");
        view.close_mission_text();
        assert_eq!(retaken(&view), WHOLE_BAR, "{script}: the text closed");
        assert_eq!(port_nav(&view), naming("Proxima"), "{script}");
    }
}

/// [`port`] where mission 130 goes to Earth, its `CompText` "Done."
/// and its `FailText` "Failed.", with `on_success` and `on_failure`,
/// accepted by `S130` (and failed by `F130` when `fail`), then landed
/// on again: the landing's gauges, and the view before its landing
/// settles.
fn relanded(on_success: &str, on_failure: &str, fail: bool) -> (Reserves, View) {
    relanded_by(RuleSource::Engine, on_success, on_failure, fail)
}

/// [`relanded`], the HUD's cadence read as `cadence` says.
fn relanded_by(
    cadence: RuleSource,
    on_success: &str,
    on_failure: &str,
    fail: bool,
) -> (Reserves, View) {
    let mut catalog = port();
    catalog.missions = vec![MissionRecord {
        travel_stel: 128,
        on_success: nova_sim::Script::parse(on_success),
        on_failure: nova_sim::Script::parse(on_failure),
        ..errand(130)
    }];
    catalog.mission_texts = vec![MissionTexts {
        comp: Some("Done.".to_owned()),
        fail: Some("Failed.".to_owned()),
        ..MissionTexts::named(MissionId(130), "Run")
    }];
    let mut view = docked_at_earth(catalog, cadence);
    run_set(&mut view, if fail { "S130 F130" } else { "S130" });
    view.take_off().expect("takes off");
    view.settle_script();
    land_now(&mut view);
    (view.reserves_shown(&view.frame_now()), view)
}

/// A text queued before an `H` onto ship 130 (under `ïntf` 129, whose
/// background is `PICT` 705): a refusal's `RefuseText` before its
/// `OnRefuse` (@0xa2f04 → @0xa2f41), a landing's `CompText` before
/// its `OnSuccess` (@0xa048c → @0xa04f8), and a `ShipDoneText` before
/// its `OnShipDone` (@0x9ed0f → @0x9ed70), of a mission to destroy
/// no ships that an `S` in a landing's `OnSuccess` starts, whose goals
/// the landing's trailing check meets. Each gives the view, docked
/// with the text taken and a frame's catch-up made.
fn texts_before_an_h(cadence: RuleSource) -> Vec<(&'static str, View)> {
    let refusal = {
        let mut catalog = port();
        catalog.missions[0].avail_loc = SPACEPORT;
        catalog.missions[0].on_refuse = nova_sim::Script::parse("H130");
        catalog.mission_texts.push(MissionTexts {
            refuse: Some("No.".to_owned()),
            ..MissionTexts::named(MissionId(130), "Run")
        });
        let mut view = docked_at_earth(catalog, cadence);
        assert_eq!(bar_shown(&view), Some(700), "ship 128's");
        assert_eq!(view.refuse_mission(MissionId(130)).map(|_| ()), Ok(()));
        view
    };
    let landing = {
        let (_, mut view) = relanded_by(cadence, "H130", "", false);
        assert_eq!(bar_shown(&view), Some(700), "ship 128's");
        view.settle_script();
        view
    };
    let ship_done = {
        let mut catalog = port();
        catalog.missions = vec![
            MissionRecord {
                travel_stel: 128,
                on_success: nova_sim::Script::parse("S131"),
                ..errand(130)
            },
            MissionRecord {
                on_ship_done: nova_sim::Script::parse("H130"),
                ships: MissionShip {
                    goal: 0,
                    count: 0,
                    ..MissionShip::NONE
                },
                ..errand(131)
            },
        ];
        catalog.mission_texts = vec![
            MissionTexts::named(MissionId(130), "Run"),
            MissionTexts {
                ship_done: Some("Met.".to_owned()),
                ..MissionTexts::named(MissionId(131), "Escort")
            },
        ];
        let mut view = docked_at_earth(catalog, cadence);
        run_set(&mut view, "S130");
        view.take_off().expect("takes off");
        view.settle_script();
        land_now(&mut view);
        assert_eq!(bar_shown(&view), Some(700), "ship 128's");
        view.settle_script();
        view
    };
    let views = vec![
        ("a RefuseText", refusal),
        ("a CompText", landing),
        ("a ShipDoneText", ship_done),
    ];
    views
        .into_iter()
        .map(|(what, mut view)| {
            assert_eq!(view.take_mission_texts().len(), 1, "{what}");
            view.catch_up();
            assert_eq!(
                view.session().map(Session::ship),
                Ok(ShipId(130)),
                "{what}: the H ran"
            );
            (what, view)
        })
        .collect()
}

#[test]
fn a_text_queued_before_an_h_shows_over_the_old_graphic_until_it_closes() {
    for (what, mut view) in texts_before_an_h(RuleSource::Engine) {
        assert_eq!(bar_shown(&view), Some(700), "{what}: the old graphic");
        view.tick(TICK);
        assert_eq!(bar_shown(&view), Some(700), "{what}: a frame on");
        view.close_mission_text();
        assert_eq!(bar_shown(&view), Some(705), "{what}: at the H, once closed");
        view.tick(TICK);
        assert_eq!(bar_shown(&view), Some(705), "{what}: kept");
    }
}

#[test]
fn under_the_bible_cadence_the_graphic_follows_at_once() {
    for (what, mut view) in texts_before_an_h(RuleSource::Bible) {
        assert_eq!(bar_shown(&view), Some(705), "{what}: at once");
        view.close_mission_text();
        assert_eq!(bar_shown(&view), Some(705), "{what}");
    }
}

#[test]
fn a_comp_text_shows_the_gauges_from_before_on_success() {
    for (script, at_h) in [("G201 M131", false), ("H130 G201 M131", true)] {
        let (landed, mut view) = relanded(script, "", false);
        view.settle_script();
        assert_eq!(view.take_mission_texts().len(), 1, "{script}");
        assert_eq!(
            view.reserves_shown(&view.frame_now()),
            landed,
            "{script}: the landing's"
        );
        assert_eq!(port_nav(&view), naming("Earth"), "{script}");
        view.close_mission_text();
        let shown = view.reserves_shown(&view.frame_now());
        if at_h {
            assert_eq!(
                shown.fuel.max,
                view.reserves().fuel.max - 100.0,
                "{script}: as at the H"
            );
        } else {
            assert_eq!(shown, landed, "{script}: as at the text");
        }
        assert_eq!(port_nav(&view), naming("Earth"), "{script}: before the M");
        view.refresh_status();
        assert_eq!(retaken(&view), WHOLE_BAR, "{script}: the next redraw");
        assert_eq!(port_nav(&view), naming("Proxima"), "{script}");
    }
}

#[test]
fn a_fail_text_shows_over_the_bar_as_on_failure_left_it() {
    for (h, changes) in INTF_ROWS {
        let (landed, mut view) = relanded("", &format!("{h} G201 M131"), true);
        view.settle_script();
        assert_eq!(view.take_mission_texts().len(), 1, "{h}: the fail text");
        let shown = view.reserves_shown(&view.frame_now());
        if changes {
            assert_eq!(
                shown.fuel.max,
                view.reserves().fuel.max - 100.0,
                "{h}: as at the H"
            );
        } else {
            assert_eq!(shown, landed, "{h}: the landing's");
        }
        assert_eq!(port_nav(&view), naming("Earth"), "{h}");
        view.close_mission_text();
        assert_eq!(retaken(&view), WHOLE_BAR, "{h}: as the text noted it");
        assert_eq!(port_nav(&view), naming("Proxima"), "{h}");
    }
}

#[test]
fn a_landings_two_texts_each_close_to_their_own_bar_and_the_loads_before_the_next() {
    let mut catalog = port();
    catalog.missions = vec![
        MissionRecord {
            travel_stel: 128,
            on_success: nova_sim::Script::parse("H130 G201"),
            ..errand(130)
        },
        MissionRecord {
            travel_stel: 128,
            on_success: nova_sim::Script::parse("M131 H129"),
            ..errand(131)
        },
    ];
    catalog.mission_texts = vec![
        MissionTexts {
            comp: Some("A done.".to_owned()),
            ..MissionTexts::named(MissionId(130), "A")
        },
        MissionTexts {
            comp: Some("B done.".to_owned()),
            ..MissionTexts::named(MissionId(131), "B")
        },
    ];
    let mut view = docked_at_earth(catalog, RuleSource::Engine);
    run_set(&mut view, "S130 S131");
    view.take_off().expect("takes off");
    view.settle_script();
    land_now(&mut view);
    let landed = view.reserves_shown(&view.frame_now());
    view.settle_script();
    assert_eq!(
        view.session().map(Session::ship),
        Ok(ShipId(129)),
        "both OnSuccess ran"
    );
    assert_eq!(
        view.take_mission_texts().len(),
        2,
        "A's CompText, then B's after A's OnSuccess"
    );
    assert_eq!(
        view.reserves_shown(&view.frame_now()),
        landed,
        "the landing's"
    );
    assert_eq!(landed.fuel.max, 250.0, "the landing's ship");
    assert_eq!(port_nav(&view), naming("Earth"));
    view.close_mission_text();
    assert_eq!(
        view.reserves_shown(&view.frame_now()).fuel.max,
        350.0,
        "A's text as it noted the bar, then A's H130 as at its operator, \
         before G201's tank"
    );
    assert_eq!(
        port_nav(&view),
        naming("Earth"),
        "B's M and H come after B's text, and no dialog redraws"
    );
    view.close_mission_text();
    assert_eq!(
        retaken(&view),
        WHOLE_BAR,
        "B's text, then B's H129 after its M"
    );
    assert_eq!(port_nav(&view), naming("Proxima"));
    assert!(view.bar_steps.is_empty());
}

#[test]
fn an_accepts_two_texts_close_to_their_own_bars_and_the_dialog_redraws_after_the_last() {
    let mut view = docked_scripted(|catalog| {
        let mission = &mut catalog.missions[1];
        mission.cargo_type = 0;
        mission.cargo_qty = 1;
        mission.pickup_mode = 0;
        mission.on_accept = nova_sim::Script::parse("M131");
        catalog.mission_texts[0].load_cargo = Some("Loaded.".to_owned());
    });
    assert_eq!(view.accept_mission(MissionId(131)), Ok(0));
    let queued = view.reserves();
    assert_eq!(
        view.take_mission_texts().len(),
        2,
        "the BriefText, then the LoadCargText"
    );
    desk_reserve(&mut view, nova_sim::Reserve::Fuel, 100.0);
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

#[test]
fn leaving_the_dock_drops_the_texts_pending_redraws() {
    let mut view = docked_scripted(|catalog| {
        catalog.missions[1].on_accept = nova_sim::Script::parse("M131");
    });
    assert_eq!(view.accept_mission(MissionId(131)), Ok(0));
    assert_eq!(view.take_mission_texts().len(), 1);
    assert_eq!(view.bar_steps.len(), 1, "the brief's close");
    view.take_off().expect("takes off");
    assert!(view.bar_steps.is_empty(), "the take-off's redraw");
}

// The gauges while docked.

/// Sets `reserve` through the pilot desk, which is no redraw point.
fn desk_reserve(view: &mut View, reserve: nova_sim::Reserve, value: f32) {
    let mut desk = view.pilot_desk().expect("flying");
    desk.edit(PilotEdit::Reserve(reserve, value)).expect("set");
    // As the app does after a desk edit.
    view.resync();
}

/// The fuel gauge's full-colour fill the HUD draws, if any.
fn fuel_drawn(view: &View) -> Option<DrawCommand> {
    drawn(view)
        .iter()
        .find(|c| matches!(c, DrawCommand::Line { color, .. } if *color == layout().fuel_full))
        .cloned()
}

#[test]
fn by_the_engine_the_gauges_hold_while_docked_until_a_shop_closes() {
    let mut view = FlightView::new(outfitting());
    land_now(&mut view);
    let landed = fuel_drawn(&view);
    assert!(landed.is_some());
    let held = view.reserves();
    desk_reserve(&mut view, nova_sim::Reserve::Fuel, 100.0);
    assert_eq!(view.outfit(BUY_TANK), Ok(()));
    assert_eq!(view.reserves().fuel.max, 350.0, "the tank");
    assert_eq!(view.reserves_shown(&view.frame_now()), held, "landing's");
    assert_eq!(fuel_drawn(&view), landed, "drawn held");
    view.refresh_status();
    assert_eq!(
        view.reserves_shown(&view.frame_now()),
        view.reserves(),
        "the shop closed"
    );
    assert_ne!(fuel_drawn(&view), landed);
}

#[test]
fn by_the_other_reading_the_gauges_follow_at_once_while_docked() {
    let rules = Rulebook::default().with_override(RuleKey::HudCadence, RuleSource::Bible);
    let mut view = FlightView::new(outfitting()).with_rules(rules);
    land_now(&mut view);
    let landed = fuel_drawn(&view);
    desk_reserve(&mut view, nova_sim::Reserve::Fuel, 100.0);
    assert_eq!(view.outfit(BUY_TANK), Ok(()));
    assert_eq!(
        view.reserves_shown(&view.frame_now()),
        view.reserves(),
        "at once"
    );
    assert_ne!(fuel_drawn(&view), landed);
}

// An outfitter order whose script changes the ship's `ïntf`.

/// Holds a stale snapshot of each of the status bar's parts: no
/// gauges, nav naming "Stale", a target panel for "Stale", and a
/// radar with the player off its centre.
fn stale_bar(view: &mut View) {
    view.reserves_scene = Some(Reserves::default());
    view.nav_scene = Some((
        view.nav_mode(),
        NavDisplay::Stellar(Some("Stale".to_owned())),
    ));
    view.target_scene = Some((
        None,
        Some(TargetPanel {
            name: "Stale".to_owned(),
            subtitle: String::new(),
            picture: None,
            status: nova_sim::TargetStatus::Hidden,
            tag: None,
        }),
    ));
    view.radar_scene = Some(RadarScene {
        player: at(-1.0, -1.0),
        ..RadarScene::default()
    });
    assert_eq!(retaken(view), [false; 4], "stale");
}

/// The whole bar redrawn, or the radar alone.
const WHOLE_BAR: [bool; 4] = [true; 4];
// The take-off's redraw before its deselect.

#[test]
fn the_take_off_after_a_landed_m_names_its_stellar_for_one_frame() {
    type TakeOff = fn(&mut View);
    let take_offs: [(&str, TakeOff); 2] = [
        ("the take-off", |view| {
            run_set(view, "M131");
            view.settle_script();
            assert_eq!(view.take_off(), Some(StellarId(128)));
        }),
        ("a Q", |view| {
            run_set(view, "M131 Q25048");
            view.settle_script();
            assert_eq!(view.leave_if_due(), Some(StellarId(128)));
        }),
    ];
    for (what, take_off) in take_offs {
        let mut view = told(docked_at_earth(port(), RuleSource::Engine));
        take_off(&mut view);
        assert_eq!(
            view.session().map(Session::nav_mode),
            Ok(NavMode::Stellar(None)),
            "{what}: let go"
        );
        assert_eq!(
            view.nav_shown(&view.frame_now()),
            flashing("Proxima"),
            "{what}: held"
        );
        view.tick(Duration::from_millis(1));
        assert_eq!(
            view.nav_shown(&view.frame_now()),
            flashing("Proxima"),
            "{what}: a frame within the tick"
        );
        view.tick(TICK);
        assert_eq!(
            view.nav_shown(&view.frame_now()),
            NavDisplay::Stellar(None),
            "{what}: a tick on"
        );
        let mut view = told(docked_at_earth(port(), RuleSource::Bible));
        take_off(&mut view);
        assert_eq!(
            view.nav_shown(&view.frame_now()),
            NavDisplay::Stellar(None),
            "{what}: by the bible"
        );
    }
}

#[test]
fn the_take_offs_held_nav_shows_in_the_first_frame_drawn_after_it_whatever_the_rate() {
    for fps in [120, 60, 30, 20] {
        let frame = Duration::from_secs(1) / fps;
        let mut view = told(docked_at_earth(port(), RuleSource::Engine));
        run_set(&mut view, "M131");
        view.settle_script();
        view.elapsed = Duration::from_nanos(333_333_329); // ten ticks less 1 ns
        assert_eq!(
            crate::time::ticks(view.elapsed + Duration::from_nanos(1)),
            10
        );
        assert_eq!(view.take_off(), Some(StellarId(128)));
        view.tick(frame);
        assert_eq!(
            view.nav_shown(&view.frame_now()),
            flashing("Proxima"),
            "{fps} fps: the first frame drawn, though it crosses a tick"
        );
        view.tick(TICK);
        assert_eq!(
            view.nav_shown(&view.frame_now()),
            NavDisplay::Stellar(None),
            "{fps} fps: a tick on"
        );
    }
}

#[test]
fn a_take_off_with_nothing_let_go_holds_nothing() {
    let mut view = docked_at_earth(port(), RuleSource::Engine);
    run_set(&mut view, "M132");
    view.settle_script();
    assert_eq!(view.take_off(), Some(StellarId(128)));
    assert_eq!(view.nav_shown(&view.frame_now()), NavDisplay::Stellar(None));
    assert_eq!(view.nav_flash, None, "no flash to end");
}

#[test]
fn a_plain_take_off_names_the_landed_stellar_for_one_frame() {
    let mut view = docked_at_earth(port(), RuleSource::Engine);
    assert_eq!(view.take_off(), Some(StellarId(128)));
    assert_eq!(
        view.session().map(Session::nav_mode),
        Ok(NavMode::Stellar(None)),
        "let go"
    );
    assert_eq!(view.nav_shown(&view.frame_now()), flashing("Earth"), "held");
    view.tick(TICK);
    assert_eq!(
        view.nav_shown(&view.frame_now()),
        flashing("Earth"),
        "the first frame"
    );
    view.tick(TICK);
    assert_eq!(
        view.nav_shown(&view.frame_now()),
        NavDisplay::Stellar(None),
        "a tick on"
    );
    let mut view = docked_at_earth(port(), RuleSource::Bible);
    view.take_off();
    assert_eq!(
        view.nav_shown(&view.frame_now()),
        NavDisplay::Stellar(None),
        "by the bible"
    );
}

#[test]
fn a_take_off_with_a_course_names_the_landed_stellar_over_the_cue_for_one_frame() {
    let rules = Rulebook::default().with_override(RuleKey::HudCadence, RuleSource::Engine);
    let mut view = FlightView::new(port()).with_rules(rules);
    view.sim
        .session
        .as_mut()
        .expect("flying")
        .plot_course(SystemId(131))
        .expect("a route");
    tap(&mut view, HYPERSPACE_MODE_KEY);
    land_now(&mut view);
    view.settle_script();
    assert_eq!(
        view.session().map(Session::nav_mode),
        Ok(NavMode::Stellar(Some(StellarId(128)))),
        "the land key put mode 2 over the cue"
    );
    assert_eq!(view.take_off(), Some(StellarId(128)));
    assert_eq!(
        view.session().map(Session::nav_mode),
        Ok(NavMode::Hyperspace(Some(SystemId(131)))),
        "cued"
    );
    assert_eq!(
        view.nav_shown(&view.frame_now()),
        flashing("Earth"),
        "held over the cue"
    );
    view.tick(TICK);
    assert_eq!(
        view.nav_shown(&view.frame_now()),
        flashing("Earth"),
        "the first frame"
    );
    view.tick(TICK);
    assert!(
        matches!(
            view.nav_shown(&view.frame_now()),
            NavDisplay::Hyperspace { .. }
        ),
        "a tick on: {:?}",
        view.nav_shown(&view.frame_now())
    );
}

#[test]
fn a_text_queued_in_the_take_offs_days_shows_over_its_held_nav() {
    let mut catalog = port();
    catalog.crons = vec![nova_sim::catalog::CronRecord {
        id: nova_sim::catalog::CronId(128),
        first_day: 0,
        first_month: 0,
        first_year: 0,
        last_day: 0,
        last_month: 0,
        last_year: 0,
        random: 100,
        duration: 0,
        pre_holdoff: 0,
        post_holdoff: 0,
        flags: 0,
        enable_on: Test::default(),
        on_start: nova_sim::Script::parse("S131"),
        on_end: nova_sim::Script::default(),
        contribute: 0,
        require: 0,
        news_govt: [-1; 4],
        govt_news: [-1; 4],
        ind_news: -1,
    }];
    let mut view = docked_at_earth(catalog, RuleSource::Engine);
    run_set(&mut view, "M131");
    view.settle_script();
    assert_eq!(view.take_off(), Some(StellarId(128)));
    assert_eq!(view.take_mission_texts().len(), 1, "the cron's mission");
    assert_eq!(
        view.nav_shown(&view.frame_now()),
        flashing("Proxima"),
        "under its text"
    );
    view.tick(TICK);
    assert_eq!(view.nav_shown(&view.frame_now()), NavDisplay::Stellar(None));
}

#[test]
fn a_pilot_resumed_and_landed_holds_the_gauges_from_its_landing() {
    let mut view = over_earth(outfitting());
    assert_eq!(view.take_landing(), None, "in flight");
    land_now(&mut view);
    assert_eq!(view.outfit(BUY_TANK), Ok(()));
    assert_eq!(view.reserves().fuel.max, 350.0, "the tank");
    assert_eq!(
        view.reserves_shown(&view.frame_now()).fuel.max,
        250.0,
        "the landing's"
    );
}

#[test]
fn landing_redraws_the_held_gauges() {
    let mut view = FlightView::new(outfitting());
    view.reserves_scene = Some(Reserves::default());
    desk_reserve(&mut view, nova_sim::Reserve::Fuel, 100.0);
    land_now(&mut view);
    assert_eq!(
        view.reserves_shown(&view.frame_now()),
        view.reserves(),
        "landing's"
    );
    assert_eq!(view.reserves_shown(&view.frame_now()).fuel.now, 100.0);
}

#[test]
fn in_flight_the_gauges_stay_live_by_the_engine() {
    let mut view = flight();
    assert!(view.hud_waits());
    view.redraw_status();
    let before = fuel_drawn(&view);
    desk_reserve(&mut view, nova_sim::Reserve::Fuel, 100.0);
    assert_eq!(
        view.reserves_shown(&view.frame_now()).fuel.now,
        100.0,
        "at once"
    );
    assert_ne!(fuel_drawn(&view), before);
}

#[test]
fn beginning_runs_on_start_once_and_explores_what_it_sets() {
    let catalog = starting_with("R(b5 b6) ^b7 X131");
    let mut view = flying_as(catalog, "Ada").with_chance(rolling_one());
    assert!(!bit_set(&view, 7), "flying runs nothing");
    view.begin();
    assert!(bit_set(&view, 5), "on the flight's chance");
    assert!(bit_set(&view, 7), "once");
    let explored: Vec<SystemId> = view.session().expect("flying").pilot().explored().collect();
    assert_eq!(explored, [SystemId(130), SystemId(131)]);
    let broken = FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..starting_with("b5")
    };
    let mut broken = FlightView::new(broken);
    broken.begin();
    assert_eq!(broken.pilot(), None);
}

#[test]
fn resuming_faces_the_heading_drawn_on_the_flights_chance() {
    let (script, chance) = scripted(&[123]);
    let mut view = over_earth(catalog()).with_chance(chance);
    assert_eq!(player(&view).heading, 0.0, "flying alone faces up");
    view.resume();
    assert_eq!(player(&view).heading, 123.0);
    let asked = &script.borrow().asked;
    assert_eq!(asked[0], 360, "the heading first");
    assert_eq!(
        asked[1..],
        [100; nova_sim::mission::ROLLED],
        "then the offers' roll table, on the same chance"
    );
}

#[test]
fn beginning_draws_the_offer_roll_table_on_the_flights_chance() {
    let (script, chance) = scripted(&[]);
    let catalog = catalog();
    let mut view = flying_as(catalog, "Ada").with_chance(chance);
    view.begin();
    let asked = &script.borrow().asked;
    assert!(asked.len() >= nova_sim::mission::ROLLED);
    assert_eq!(
        asked[asked.len() - nova_sim::mission::ROLLED..],
        [100; nova_sim::mission::ROLLED]
    );
}

#[test]
fn beginning_a_new_pilot_leaves_it_facing_up() {
    let (_, chance) = scripted(&[123]);
    let catalog = catalog();
    let mut view = flying_as(catalog, "Ada").with_chance(chance);
    view.begin();
    assert_eq!(player(&view).heading, 0.0);
}

#[test]
fn a_failed_flight_resumes_nothing() {
    let (script, chance) = scripted(&[123]);
    let broken = FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..catalog()
    };
    let mut broken = FlightView::new(broken).with_chance(chance);
    broken.resume();
    assert_eq!(broken.pilot(), None);
    assert!(script.borrow().asked.is_empty(), "nothing drawn");
}

/// A catalog of mission 130 with two auxiliary ships, and the mission
/// as its accept leaves it.
fn aux_errand() -> (impl Fn() -> FakeCatalog, nova_sim::mission::ActiveMission) {
    let errands = || {
        let mut errands = catalog();
        errands.sites = vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)];
        errands.missions = vec![MissionRecord {
            aux: nova_sim::catalog::AuxShip {
                count: 2,
                dude: Some(nova_sim::catalog::DudeId(200)),
                syst: -1,
            },
            ..errand(130)
        }];
        errands
    };
    let accepted = {
        let mut view = FlightView::new(errands());
        land_now(&mut view);
        view.settle_script();
        assert_eq!(view.accept_mission(MissionId(130)), Ok(0));
        let pilot = view.pilot().expect("flying");
        pilot.missions().get(0).expect("held").clone()
    };
    (errands, accepted)
}

/// A flight over Earth holding [`aux_errand`]'s mission in slot 0, on
/// `chance`.
fn aux_flight(chance: SharedChance) -> View {
    let (errands, accepted) = aux_errand();
    let catalog = errands();
    let session = SessionBuilder::flying(&catalog)
        .over(StellarId(128))
        .with_mission(0, accepted)
        .build();
    FlightView::from_session(catalog, session.into()).with_chance(chance)
}

/// Mission 130's auxiliary ships in `view`.
fn aux_of(view: &View) -> nova_sim::mission::aux_ships::AuxShips {
    view.pilot()
        .expect("flying")
        .missions()
        .get(0)
        .expect("held")
        .aux
}

/// A flight that failed to start, on `chance`.
fn broken_flight(chance: SharedChance) -> View {
    let broken = FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..catalog()
    };
    FlightView::new(broken).with_chance(chance)
}

#[test]
fn opening_on_the_main_screen_restarts_the_aux_countdown_on_the_flights_chance() {
    let (script, chance) = scripted(&[9]);
    let mut view = aux_flight(chance);
    assert_eq!(aux_of(&view).countdown, 139, "the accept's draw");
    view.open_on_main_screen();
    assert_eq!(
        aux_of(&view),
        nova_sim::mission::aux_ships::AuxShips {
            countdown: 79,
            arrived: 0,
            left: 2
        }
    );
    assert_eq!(script.borrow().asked, [70]);
    let (script, chance) = scripted(&[]);
    broken_flight(chance).open_on_main_screen();
    assert!(
        script.borrow().asked.is_empty(),
        "a failed flight draws nothing"
    );
}

#[test]
fn opening_at_launch_restarts_the_aux_countdown_on_the_flights_chance() {
    let (script, chance) = scripted(&[4]);
    let mut view = aux_flight(chance);
    view.open_at_launch();
    assert_eq!(
        aux_of(&view),
        nova_sim::mission::aux_ships::AuxShips {
            countdown: 74,
            arrived: 0,
            left: 2
        }
    );
    assert_eq!(script.borrow().asked, [70]);
    let (script, chance) = scripted(&[]);
    broken_flight(chance).open_at_launch();
    assert!(
        script.borrow().asked.is_empty(),
        "a failed flight draws nothing"
    );
}

#[test]
fn in_port_a_plain_click_on_the_map_selects_only() {
    let mut view = in_the_bar(None);
    view.open_bar_map();
    port_click(&mut view, 131);
    assert_eq!(view.course_map().selected(), Some(SystemId(131)));
    assert_eq!(
        view.session().map(Session::nav_mode),
        Ok(NavMode::Stellar(Some(StellarId(128)))),
        "no nav written docked (@0x136ec)"
    );
    assert_eq!(view.session().map(|session| session.course().len()), Ok(0));
}

#[test]
fn in_port_a_shift_click_builds_the_course_and_the_close_cues_nothing() {
    let mut view = in_the_bar(None);
    view.open_bar_map();
    view.port_map_input(&key(Key::Shift, true));
    port_click(&mut view, 131);
    view.port_map_input(&key(Key::Shift, false));
    assert_eq!(
        view.session().map(|session| session.course().to_vec()),
        Ok(vec![SystemId(131)]),
        "the Shift-click never tests _onPlanet"
    );
    assert!(clear_route_enabled(&view));
    view.port_map_input(&key(MAP, true));
    assert!(!view.map_open());
    assert_eq!(port_nav(&view), naming("Earth"), "no cue drawn");
    assert_eq!(
        view.session().map(Session::nav_mode),
        Ok(NavMode::Stellar(Some(StellarId(128))))
    );
}

#[test]
fn in_port_c_clears_the_route_and_the_close_puts_back_the_bars_nav() {
    let mut view = in_the_bar(Some(131));
    view.open_bar_map();
    assert!(clear_route_enabled(&view), "a course");
    view.port_map_input(&key(Key::Char('c'), true));
    assert_eq!(view.session().map(|session| session.course().len()), Ok(0));
    assert_eq!(
        view.session().map(Session::nav_mode),
        Ok(NavMode::Stellar(None)),
        "let go, the mode kept"
    );
    view.port_map_input(&key(MAP, true));
    assert!(!view.map_open());
    assert_eq!(
        view.session().map(Session::nav_mode),
        Ok(NavMode::Stellar(Some(StellarId(128)))),
        "the bar's nav put back"
    );
}

/// A text queued before the nav area's last redraw is older than it: its
/// nav doesn't replace the nav area as the texts are taken in flight.
#[test]
fn a_text_queued_before_the_last_nav_redraw_leaves_the_nav_area_as_redrawn() {
    let mut catalog = catalog();
    catalog.missions = vec![errand(130)];
    catalog.mission_texts = vec![MissionTexts {
        brief: Some("Off.".to_owned()),
        ..MissionTexts::named(MissionId(130), "Run")
    }];
    let mut view = FlightView::new(catalog);
    run_set(&mut view, "S130");
    view.settle_script();
    view.redraw_status();
    view.sim
        .session
        .as_mut()
        .expect("flying")
        .select_stellar(StellarPick::Slot(0));
    assert_eq!(view.take_mission_texts().len(), 1, "the brief");
    assert_eq!(
        view.nav_scene.as_ref().map(|(mode, _)| *mode),
        Some(NavMode::Stellar(Some(StellarId(128)))),
        "as the take's redraw drew it, not the text's nav off"
    );
}
