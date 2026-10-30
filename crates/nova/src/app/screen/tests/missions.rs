use super::*;

// Mission offers at every venue.

#[test]
fn landing_offers_a_spaceport_mission_over_the_spaceport() {
    let store = MemoryPilots::new();
    let mut screen = landed_at_venues(&store, venue_data(SPACEPORT));
    let offer = spaceport(&screen)
        .offer_dialog()
        .expect("offered on landing");
    assert_eq!(offer.prompt().mission, MissionId(130));
    assert_eq!(offer.prompt().text, "Job 130.");
    screen.input(&key(Key::Enter, true));
    assert!(spaceport(&screen).offer_dialog().is_none());
    assert_eq!(active(&screen), [MissionId(130)]);
    assert_eq!(pilot(&screen).cash(), 900, "its pay took 100");
}

#[test]
fn a_pilot_desk_move_offers_the_new_stellars_spaceport_mission() {
    let fork = game_data_fork(true, true, true, 100, b"", b"", None)
        .resource(
            Mission::TYPE,
            131,
            Some(b"Venue job"),
            &venue_mission_at(SPACEPORT, 129),
        )
        .resource(Desc::TYPE, 131 + 3872, Some(b"Offer"), &desc_of("Job 131."));
    let store = MemoryPilots::new();
    let mut screen = landed_at_venues(&store, loaded(&fork));
    assert!(spaceport(&screen).offer_dialog().is_none(), "none at 128");
    edit(
        &mut screen,
        PilotEdit::MoveTo {
            system: SystemId(129),
            stellar: nova_sim::StellarId(129),
        },
    );
    assert_eq!(spaceport(&screen).stellar(), nova_sim::StellarId(129));
    assert_eq!(
        offered_mission(spaceport(&screen).offer_dialog()),
        Some(MissionId(131)),
        "asked before the next frame"
    );
    screen.input(&key(Key::Enter, true));
    assert_eq!(active(&screen), [MissionId(131)]);
}

/// The router landed at Alpha Prime, whose spaceport offers mission
/// 130 of `Flags` `flags`, its offer `dësc` of `text` naming the movie
/// "Offer.mov" played as `movie` says, with movies played through a
/// new [`MockMoviePlayer`] and `prefs`.
pub(super) fn landed_filmed(
    flags: u16,
    text: &str,
    movie: u16,
    prefs: Prefs,
) -> (AppScreen, Rc<MockMoviePlayer>) {
    let (mut screen, reel) = filmed(flags, text, movie, prefs);
    create(&mut screen, "Ada");
    land_twice(&mut screen);
    (screen, reel)
}

/// [`landed_filmed`], before any pilot is made.
pub(super) fn filmed(
    flags: u16,
    text: &str,
    movie: u16,
    prefs: Prefs,
) -> (AppScreen, Rc<MockMoviePlayer>) {
    let mut mission = venue_mission(SPACEPORT);
    mission[0x50..0x52].copy_from_slice(&flags.to_be_bytes());
    let offer = filmed_desc_of(text, "Offer.mov", movie);
    let fork = game_data_fork(true, true, true, 100, b"", b"", None)
        .resource(Mission::TYPE, 130, Some(b"Venue job"), &mission)
        .resource(Desc::TYPE, 130 + 3872, Some(b"Offer"), &offer);
    let reel = MockMoviePlayer::playing((160, 120), 2);
    let store = MemoryPilots::new();
    let screen = AppScreen::new(loaded(&fork))
        .with_prefs(prefs)
        .with_movie_player(Rc::clone(&reel) as Rc<dyn MoviePlayer>)
        .with_dialogs(Rc::new(VenueDialogs), Rc::new(MonoMetrics))
        .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
        .appeared();
    (screen, reel)
}

/// Opens the Preferences, clicks QuickTime Movies and closes them.
fn toggle_movies(screen: &mut AppScreen) {
    open_prefs(screen);
    let was = prefs_dialog(screen).quicktime_movies().on();
    let point = prefs_dialog(screen).quicktime_movies().rect().center();
    click_at(screen, point);
    assert_eq!(prefs_dialog(screen).quicktime_movies().on(), !was);
    screen.input(&key(Key::Enter, true));
    assert!(screen.preferences().is_none(), "closed");
    assert_eq!(screen.prefs().quicktime_movies, !was);
}

#[test]
fn quicktime_movies_turned_off_in_the_dialog_stop_every_movie() {
    for movie in [0, 0x0001] {
        let (mut screen, reel) = filmed(0, "Job.", movie, Prefs::default());
        toggle_movies(&mut screen);
        create(&mut screen, "Ada");
        land_twice(&mut screen);
        assert!(spaceport(&screen).offer_dialog().is_some(), "{movie}");
        screen.input(&key(Key::Enter, true));
        assert_eq!(active(&screen), [MissionId(130)], "{movie}");
        assert!(reeled(&reel).is_empty(), "{movie}: {:?}", reeled(&reel));
    }
}

#[test]
fn quicktime_movies_turned_on_in_the_dialog_play_again() {
    let off = Prefs {
        quicktime_movies: false,
        ..Prefs::default()
    };
    let (mut screen, reel) = filmed(0, "Job.", 0, off);
    toggle_movies(&mut screen);
    create(&mut screen, "Ada");
    land_twice(&mut screen);
    screen.tick(Duration::from_millis(16));
    assert_eq!(
        reeled(&reel),
        [Call::Start("Offer.mov".into())],
        "before the dialog"
    );
    let (mut screen, reel) = filmed(0, "Job.", 0x0001, off);
    toggle_movies(&mut screen);
    create(&mut screen, "Ada");
    land_twice(&mut screen);
    screen.input(&key(Key::Enter, true));
    assert_eq!(
        reeled(&reel),
        [Call::Start("Offer.mov".into())],
        "after the answer"
    );
}

#[test]
fn the_spaceports_offer_plays_its_after_movie_once_answered() {
    let (mut screen, reel) = landed_filmed(0, "Job.", 0x0001, Prefs::default());
    assert!(spaceport(&screen).offer_dialog().is_some());
    assert!(reeled(&reel).is_empty(), "not before the dialog");
    screen.input(&key(Key::Enter, true));
    assert_eq!(active(&screen), [MissionId(130)]);
    assert_eq!(
        reeled(&reel),
        [Call::Start("Offer.mov".into())],
        "after the answer"
    );
    screen.tick(Duration::from_millis(16));
    screen.tick(Duration::from_millis(16));
    assert_eq!(reeled(&reel), [Call::Start("Offer.mov".into()), Call::Stop]);
}

#[test]
fn the_spaceports_offer_plays_its_before_movie_and_a_refusal_after_it_plays_none() {
    let (mut screen, reel) = landed_filmed(0, "Job.", 0, Prefs::default());
    assert!(reeled(&reel).is_empty(), "staged until the spaceport ticks");
    screen.tick(Duration::from_millis(16));
    assert_eq!(reeled(&reel), [Call::Start("Offer.mov".into())]);
    screen.tick(Duration::from_millis(16));
    screen.input(&key(Key::Char('n'), true));
    assert_eq!(active(&screen), []);
    assert_eq!(
        reeled(&reel),
        [Call::Start("Offer.mov".into()), Call::Stop],
        "none after"
    );
}

/// The router landed at Alpha Prime, whose spaceport offers mission
/// 130 of the `mïsn` bytes `mission`, its offer `dësc` "Job." naming
/// "Offer.mov" played after, with the `dësc`s `descs` too, movies
/// played through a new [`MockMoviePlayer`].
fn landed_offering(mission: &[u8], descs: &[(i16, Vec<u8>)]) -> (AppScreen, Rc<MockMoviePlayer>) {
    let mut fork = game_data_fork(true, true, true, 100, b"", b"", None)
        .resource(Mission::TYPE, 130, Some(b"Venue job"), mission)
        .resource(
            Desc::TYPE,
            130 + 3872,
            Some(b"Offer"),
            &filmed_desc_of("Job.", "Offer.mov", 0x0001),
        );
    for (id, desc) in descs {
        fork = fork.resource(Desc::TYPE, *id, None, desc);
    }
    let reel = MockMoviePlayer::playing((160, 120), 2);
    let store = MemoryPilots::new();
    let mut screen = AppScreen::new(loaded(&fork))
        .with_movie_player(Rc::clone(&reel) as Rc<dyn MoviePlayer>)
        .with_dialogs(Rc::new(VenueDialogs), Rc::new(MonoMetrics))
        .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
        .appeared();
    create(&mut screen, "Ada");
    land_twice(&mut screen);
    (screen, reel)
}

fn ticked(screen: &mut AppScreen, ticks: usize) {
    for _ in 0..ticks {
        screen.tick(Duration::from_millis(16));
    }
}

#[test]
fn a_no_room_text_replays_the_spaceport_offers_after_movie() {
    let mut mission = venue_mission(SPACEPORT);
    mission[0x10..0x12].copy_from_slice(&0_i16.to_be_bytes());
    mission[0x12..0x14].copy_from_slice(&10_000_i16.to_be_bytes());
    let (mut screen, reel) = landed_offering(&mission, &[]);
    assert!(spaceport(&screen).offer_dialog().is_some());
    screen.input(&key(Key::Enter, true));
    assert_eq!(active(&screen), [], "no room for the cargo");
    assert!(
        spaceport(&screen).mission_text().is_some(),
        "the no-room text"
    );
    assert!(reeled(&reel).is_empty(), "after it, not before");
    screen.input(&key(Key::Enter, true));
    ticked(&mut screen, 6);
    assert_eq!(
        reeled(&reel),
        [
            Call::Start("Offer.mov".into()),
            Call::Stop,
            Call::Start("Offer.mov".into()),
            Call::Stop
        ],
        "the STR# text's after-check reads the offer's (@0x5b40b), and the offer's \
         after-check plays it again (@0xa3580)"
    );
}

#[test]
fn a_refuse_texts_after_movie_plays_twice_and_the_offers_not_at_all() {
    let mut mission = venue_mission(SPACEPORT);
    mission[0x58..0x5A].copy_from_slice(&300_i16.to_be_bytes());
    let (mut screen, reel) = landed_offering(
        &mission,
        &[(300, filmed_desc_of("No.", "Refuse.mov", 0x0001))],
    );
    screen.input(&key(Key::Char('n'), true));
    assert!(
        spaceport(&screen).mission_text().is_some(),
        "the RefuseText"
    );
    assert!(
        spaceport(&screen).offer_dialog().is_none(),
        "the offer hidden for it (@0xa2e8e)"
    );
    screen.input(&key(Key::Enter, true));
    ticked(&mut screen, 6);
    assert_eq!(
        reeled(&reel),
        [
            Call::Start("Refuse.mov".into()),
            Call::Stop,
            Call::Start("Refuse.mov".into()),
            Call::Stop
        ],
        "nothing reloads the offer after the refuse text (@0xa2eb5-0xa2f4d)"
    );
}

fn texts_drawn(screen: &AppScreen) -> Vec<String> {
    drawn(screen)
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_refused_spaceport_offer_without_a_refuse_text_stays_drawn_under_its_after_movie() {
    let (mut screen, reel) = landed_offering(&venue_mission(SPACEPORT), &[]);
    assert!(texts_drawn(&screen).contains(&"Job.".to_owned()));
    screen.input(&key(Key::Char('n'), true));
    assert_eq!(reeled(&reel), [Call::Start("Offer.mov".into())]);
    assert!(
        texts_drawn(&screen).contains(&"Job.".to_owned()),
        "not hidden (@0xa2e70-0xa2e78): under its after-movie until @0xa361e"
    );
    ticked(&mut screen, 3);
    assert_eq!(reeled(&reel), [Call::Start("Offer.mov".into()), Call::Stop]);
    assert!(
        !texts_drawn(&screen).contains(&"Job.".to_owned()),
        "disposed"
    );
}

#[test]
fn a_refuse_text_hides_the_spaceport_offer() {
    let mut mission = venue_mission(SPACEPORT);
    mission[0x58..0x5A].copy_from_slice(&300_i16.to_be_bytes());
    let (mut screen, _) = landed_offering(&mission, &[(300, desc_of("No."))]);
    screen.input(&key(Key::Char('n'), true));
    let shown = texts_drawn(&screen);
    assert!(
        !shown.contains(&"Job.".to_owned()),
        "hidden for its RefuseText (@0xa2e8e): {shown:?}"
    );
}

#[test]
fn closing_the_mission_info_over_the_spaceport_offer_reloads_the_offers_after_movie() {
    // 129, accepted on the first landing, is held with a QuickBrief
    // naming "Quick.mov"; its accept shifts 130 out of the list
    // (`RuleKey::OfferShift`), so the second landing offers 131.
    let mut held = venue_mission(SPACEPORT);
    held[0x36..0x38].copy_from_slice(&6001_i16.to_be_bytes());
    let fork = game_data_fork(true, true, true, 100, b"", b"", None)
        .resource(Mission::TYPE, 129, Some(b"Held job"), &held)
        .resource(Desc::TYPE, 129 + 3872, Some(b"Held"), &desc_of("Held."))
        .resource(
            Desc::TYPE,
            6001,
            Some(b"Quick"),
            &filmed_desc_of("Quick.", "Quick.mov", 0x0001),
        )
        .resource(
            Mission::TYPE,
            130,
            Some(b"Shifted job"),
            &venue_mission(SPACEPORT),
        )
        .resource(
            Mission::TYPE,
            131,
            Some(b"Venue job"),
            &venue_mission(SPACEPORT),
        )
        .resource(
            Desc::TYPE,
            131 + 3872,
            Some(b"Offer"),
            &filmed_desc_of("Job.", "Offer.mov", 0x0001),
        );
    let reel = MockMoviePlayer::playing((160, 120), 2);
    let store = MemoryPilots::new();
    let mut screen = AppScreen::new(loaded(&fork))
        .with_movie_player(Rc::clone(&reel) as Rc<dyn MoviePlayer>)
        .with_dialogs(Rc::new(InfoOverEveryOpener), Rc::new(MonoMetrics))
        .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
        .appeared();
    create(&mut screen, "Ada");
    land_twice(&mut screen);
    assert_eq!(
        offered_mission(spaceport(&screen).offer_dialog()),
        Some(MissionId(129))
    );
    screen.input(&key(Key::Enter, true));
    assert_eq!(active(&screen), [MissionId(129)], "held");
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::Flight, "took off");
    land_twice(&mut screen);
    assert_eq!(
        offered_mission(spaceport(&screen).offer_dialog()),
        Some(MissionId(131)),
        "offered on landing again, 129's accept having shifted 130 out"
    );
    screen.input(&key(nova_view::ui::mission_info::MISSION_INFO_KEY, true));
    screen.input(&key(nova_view::ui::mission_info::MISSION_INFO_KEY, false));
    assert!(spaceport(&screen).mission_info().is_some(), "info open");
    screen.input(&key(Key::Escape, true));
    assert!(spaceport(&screen).mission_info().is_none(), "info closed");
    assert!(spaceport(&screen).offer_dialog().is_some(), "still offered");
    screen.input(&key(Key::Char('n'), true));
    assert_eq!(active(&screen), [MissionId(129)], "refused");
    assert!(spaceport(&screen).mission_text().is_none(), "no RefuseText");
    ticked(&mut screen, 3);
    assert_eq!(
        reeled(&reel),
        [Call::Start("Offer.mov".into()), Call::Stop],
        "the info screen loaded the QuickBrief's, and its close loaded the offer's \
         again (@0xa335c)"
    );
}

#[test]
fn the_mission_computers_refusal_plays_the_last_descs_after_movie() {
    let fork = game_data_fork(true, true, true, 100, b"", b"", None).resource(
        Desc::TYPE,
        128,
        None,
        &filmed_desc_of("A port.", "Port.mov", 0x0001),
    );
    let reel = MockMoviePlayer::playing((160, 120), 2);
    let store = MemoryPilots::new();
    let mut screen = AppScreen::new(loaded(&fork))
        .with_movie_player(Rc::clone(&reel) as Rc<dyn MoviePlayer>)
        .with_dialogs(Rc::new(VenueDialogs), Rc::new(MonoMetrics))
        .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
        .appeared();
    create(&mut screen, "Ada");
    land_twice(&mut screen);
    assert_eq!(
        screen.showing(),
        Showing::Movie,
        "the landing plays the stellar's movie (@0x632a7)"
    );
    ticked(&mut screen, 2);
    assert_eq!(screen.showing(), Showing::Spaceport);
    let landing = [Call::Start("Port.mov".into()), Call::Stop];
    click_port_item(&mut screen, 10);
    assert!(
        spaceport(&screen).mission_text().is_some(),
        "nothing available here"
    );
    assert_eq!(reeled(&reel), landing);
    screen.input(&key(Key::Enter, true));
    assert_eq!(
        reeled(&reel)[landing.len()..],
        [Call::Start("Port.mov".into())],
        "the stellar's dësc was the last loaded (@0x5f96b), and the refusal's \
         after-check reads it (@0xa3956, @0x5b40b)"
    );
}

#[test]
fn a_persons_offer_opened_over_a_mission_text_over_flight_waits_for_its_movie() {
    let (mut screen, reel) = filmed(0, "Job.", 0, Prefs::default());
    create(&mut screen, "Ada");
    assert_eq!(screen.showing(), Showing::Flight);
    screen.open_offer(OfferPrompt {
        mission: MissionId(130),
        text: "Job.".to_owned(),
        labels: nova_sim::mission::offer_labels("", "", true),
        graphic: None,
        movie: nova_sim::DescMovie::from_desc("Offer.mov", 0),
    });
    screen.show_over_flight(vec![DescShown {
        text: "Well done.".to_owned(),
        graphic: None,
        movie: nova_sim::DescMovie::from_desc("Text.mov", 0),
    }]);
    ticked(&mut screen, 4);
    assert_eq!(
        reeled(&reel),
        [Call::Start("Text.mov".into()), Call::Stop],
        "`_PlayQTMovie` is modal: the offer's movie waits for the text and its movie"
    );
    assert_eq!(screen.showing(), Showing::MissionText);
    assert_eq!(
        screen
            .flight_mission_text()
            .map(|text| text.lines().join(" "))
            .as_deref(),
        Some("Well done.")
    );
    screen.input(&key(Key::Enter, true));
    assert!(screen.flight_mission_text().is_none(), "Return closes it");
    assert_eq!(reeled(&reel), [Call::Start("Text.mov".into()), Call::Stop]);
    screen.tick(Duration::from_millis(16));
    assert_eq!(
        reeled(&reel),
        [
            Call::Start("Text.mov".into()),
            Call::Stop,
            Call::Start("Offer.mov".into())
        ],
        "then the offer's, as its first tick comes"
    );
    assert_eq!(screen.showing(), Showing::Movie);
}

#[test]
fn a_spaceport_offer_accepted_at_once_plays_its_movie() {
    let (screen, reel) = landed_filmed(0x0004, "", 0x0004, Prefs::default());
    assert_eq!(active(&screen), [MissionId(130)]);
    assert_eq!(reeled(&reel), [Call::Start("Offer.mov".into())]);
    assert!(!screen.hides_pointer(), "during the fade out");
}

#[test]
fn a_cinematic_movie_over_the_spaceport_hides_the_pointer() {
    let (mut screen, _) = landed_filmed(0x0004, "", 0x0004, Prefs::default());
    screen.tick(Duration::from_secs(1));
    assert!(screen.hides_pointer());
}

#[test]
fn command_comma_over_a_spaceport_movie_opens_no_preferences_and_the_movie_plays_on() {
    let (mut screen, reel) = landed_filmed(0, "Job.", 0x0004, Prefs::default());
    screen.tick(Duration::from_secs(1));
    assert!(screen.hides_pointer(), "the cinematic movie plays");
    command_comma(&mut screen);
    assert!(screen.preferences().is_none());
    assert_eq!(screen.showing(), Showing::Spaceport);
    assert!(screen.hides_pointer(), "still hidden");
    for _ in 0..8 {
        screen.tick(Duration::from_secs(1));
    }
    assert_eq!(
        reeled(&reel),
        [Call::Start("Offer.mov".into()), Call::Stop],
        "played on"
    );
    assert!(
        spaceport(&screen)
            .offer_dialog()
            .is_some_and(|offer| !offer.playing()),
        "the offer shows"
    );
}

#[test]
fn an_overlay_over_a_spaceport_movie_shows_the_pointer() {
    let (mut screen, _) = landed_filmed(0, "Job.", 0x0004, Prefs::default());
    screen.tick(Duration::from_secs(1));
    assert!(screen.hides_pointer());
    screen.open_preferences();
    assert_eq!(screen.showing(), Showing::Preferences);
    assert!(!screen.hides_pointer());
}

#[test]
fn with_movies_off_in_the_preferences_none_plays() {
    let off = Prefs {
        quicktime_movies: false,
        ..Prefs::default()
    };
    let (mut screen, reel) = landed_filmed(0, "Job.", 0x0001, off);
    screen.input(&key(Key::Enter, true));
    assert_eq!(active(&screen), [MissionId(130)]);
    assert!(reeled(&reel).is_empty());
}

/// Opens shop `item` at a stellar offering mission 130 at `loc`, and
/// gives the offer open over it.
fn shop_offer(loc: i16, item: usize) -> Option<MissionId> {
    let store = MemoryPilots::new();
    let mut screen = landed_at_venues(&store, venue_data(loc));
    assert!(
        spaceport(&screen).offer_dialog().is_none(),
        "not on landing"
    );
    click_port_item(&mut screen, item);
    let spaceport = spaceport(&screen);
    let opened = match loc {
        TRADE_CENTER => spaceport.open_trade().and_then(TradeScreen::offer_dialog),
        OUTFITTER => spaceport
            .open_outfitter()
            .and_then(OutfitterScreen::offer_dialog),
        _ => spaceport
            .open_shipyard()
            .and_then(ShipyardScreen::offer_dialog),
    };
    offered_mission(opened)
}

#[test]
fn entering_the_trade_center_offers_its_mission_at_once() {
    assert_eq!(shop_offer(TRADE_CENTER, 7), Some(MissionId(130)));
}

#[test]
fn entering_the_outfitter_offers_its_mission_at_once() {
    assert_eq!(shop_offer(OUTFITTER, 8), Some(MissionId(130)));
}

/// [`shipbuying_data`] with an outfitter selling nothing, as no `oütf`
/// exists, and mission 130 at the outfitter ([`venue_mission`]),
/// unrefusable and with no offer text, so an offer of it is its
/// accept at once.
fn empty_outfitter_offering_data() -> Rc<GameData> {
    let mut mission = venue_mission(OUTFITTER);
    mission[0x50..0x52].copy_from_slice(&0x0004_u16.to_be_bytes());
    let mut built = game_data_fork(true, false, true, 100, b"", b"", None)
        .resource(Mission::TYPE, 130, Some(b"Venue job"), &mission)
        .build();
    let stellar = built.layout.data_entry(Stellar::TYPE, 128) + 4;
    built.bytes[stellar + 0x09] |= 0x04;
    loaded_bytes(built.bytes)
}

/// An outfitter listing nothing never opens, so it offers nothing
/// (`_DoOutfitDialog` @0x5bbf7 → @0x5dbd6 skips its offer): its
/// mission, accepted at once on any offer, stays unaccepted.
#[test]
fn an_empty_outfitter_offers_nothing() {
    let store = MemoryPilots::new();
    let mut screen = landed_at_venues(&store, empty_outfitter_offering_data());
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
    screen.input(&key(Key::Enter, true));
    screen.tick(Duration::from_secs(5));
    assert!(spaceport(&screen).offer_dialog().is_none());
    assert_eq!(active(&screen), [], "nothing offered, nothing accepted");
    assert_eq!(pilot(&screen).cash(), 1000);
}

#[test]
fn entering_the_shipyard_offers_its_mission_at_once() {
    assert_eq!(shop_offer(SHIPYARD, 9), Some(MissionId(130)));
}

#[test]
fn an_accept_at_the_outfitter_hands_the_outfitter_back() {
    let store = MemoryPilots::new();
    let mut screen = landed_at_venues(&store, venue_data(OUTFITTER));
    click_port_item(&mut screen, 8);
    let open = spaceport(&screen).open_outfitter().expect("outfitting");
    assert_eq!(offered_mission(open.offer_dialog()), Some(MissionId(130)));
    assert_eq!(open.outfitter().cash, 1000);
    screen.input(&key(Key::Enter, true));
    assert_eq!(active(&screen), [MissionId(130)]);
    let open = spaceport(&screen)
        .open_outfitter()
        .expect("still outfitting");
    assert!(open.offer_dialog().is_none());
    assert_eq!(open.outfitter().cash, 900, "the cash shown changes");
}

/// The trade center's offer at a stellar offering missions 130-132
/// there, with the offer timer on `clock`: 130 refused, 131 left open
/// 5 s and refused, and a moment after: the offer open then.
fn offer_a_moment_after_a_long_offer(clock: RuleSource) -> Option<MissionId> {
    let store = MemoryPilots::new();
    let rulebook = Rulebook::default().with_override(RuleKey::OfferClock, clock);
    let mut screen = AppScreen::new(venue_data_of(TRADE_CENTER, &[130, 131, 132]))
        .with_rulebook(&rulebook)
        .with_dialogs(Rc::new(VenueDialogs), Rc::new(MonoMetrics))
        .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
        .appeared();
    create(&mut screen, "Ada");
    land_twice(&mut screen);
    click_port_item(&mut screen, 7);
    let offered = |screen: &AppScreen| {
        offered_mission(
            spaceport(screen)
                .open_trade()
                .and_then(TradeScreen::offer_dialog),
        )
    };
    assert_eq!(offered(&screen), Some(MissionId(130)));
    screen.input(&key(Key::Char('n'), true));
    screen.tick(Duration::from_secs(2));
    screen.tick(Duration::from_secs(2));
    assert_eq!(offered(&screen), Some(MissionId(131)), "{clock:?}");
    screen.tick(Duration::from_secs(5));
    screen.input(&key(Key::Char('n'), true));
    screen.tick(Duration::from_millis(1));
    screen.tick(Duration::from_millis(1));
    offered(&screen)
}

/// A chance that never fires and draws the first outcome of every
/// range.
struct FirstOutcomes;

impl Chance for FirstOutcomes {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    fn below(&mut self, _n: u32) -> u32 {
        0
    }
}

/// The trade center's offer at a stellar offering missions 130 and 131,
/// the router rolling on `chance` if given: 130 refused, then whether 131
/// is offered 700 ms later (42 of the Mac's ticks), and 1.1 s later.
fn offered_after_a_refusal(chance: Option<SharedChance>) -> [Option<MissionId>; 2] {
    let store = MemoryPilots::new();
    let mut screen = AppScreen::new(venue_data_of(TRADE_CENTER, &[130, 131]));
    if let Some(chance) = chance {
        screen = screen.with_chance(chance);
    }
    let mut screen = screen
        .with_dialogs(Rc::new(VenueDialogs), Rc::new(MonoMetrics))
        .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
        .appeared();
    create(&mut screen, "Ada");
    land_twice(&mut screen);
    click_port_item(&mut screen, 7);
    let offered = |screen: &AppScreen| {
        offered_mission(
            spaceport(screen)
                .open_trade()
                .and_then(TradeScreen::offer_dialog),
        )
    };
    assert_eq!(offered(&screen), Some(MissionId(130)), "precondition");
    screen.input(&key(Key::Char('n'), true));
    screen.tick(Duration::from_millis(700));
    let early = offered(&screen);
    screen.tick(Duration::from_millis(400));
    [early, offered(&screen)]
}

#[test]
fn a_venues_next_offer_waits_the_gap_drawn_on_the_routers_chance() {
    // `OFFER_GAP_TICKS` (30) plus a draw below 30: never firing, the
    // draw is its last outcome, 29, so 59 ticks; drawing first outcomes,
    // 0, so 30 ticks.
    assert_eq!(
        offered_after_a_refusal(None),
        [None, Some(MissionId(131))],
        "the default chance's long gap"
    );
    let first = SharedChance::new(Rc::new(RefCell::new(FirstOutcomes)));
    assert_eq!(
        offered_after_a_refusal(Some(first)),
        [Some(MissionId(131)), Some(MissionId(131))],
        "the short gap the router's chance drew"
    );
}

#[test]
fn the_routers_offer_clock_reaches_the_venues() {
    assert_eq!(
        offer_a_moment_after_a_long_offer(RuleSource::Engine),
        Some(MissionId(132)),
        "the gap ran from the ask"
    );
    assert_eq!(
        offer_a_moment_after_a_long_offer(RuleSource::Bible),
        None,
        "the gap runs from the answer"
    );
}

#[test]
fn the_bar_offers_again_after_its_hire_screen_restarts_its_wait() {
    let store = MemoryPilots::new();
    let fork = [130, 131].into_iter().fold(
        game_data_fork(true, true, false, 100, b"", b"", None),
        |fork, id: i16| {
            fork.resource(Mission::TYPE, id, Some(b"Bar job"), &venue_mission(BAR))
                .resource(
                    Desc::TYPE,
                    id + 3872,
                    Some(b"Offer"),
                    &desc_of(&format!("Job {id}.")),
                )
        },
    );
    let mut built = fork.build();
    // The first ship is for hire, as in `past_the_holds_data`.
    let first = built.layout.data_entry(Ship::TYPE, 128) + 4;
    built.put_u16(first + 0x38A, 100);
    let mut screen = AppScreen::new(loaded_bytes(built.bytes))
        .with_dialogs(Rc::new(PastTheHoldsDialogs), Rc::new(MonoMetrics))
        .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
        .appeared();
    create(&mut screen, "Ada");
    land_twice(&mut screen);
    click_port_item(&mut screen, 11);
    let offered = |screen: &AppScreen| {
        offered_mission(
            spaceport(screen)
                .open_bar()
                .and_then(BarScreen::offer_dialog),
        )
    };
    screen.tick(Duration::from_millis(300));
    screen.tick(Duration::from_millis(300));
    assert_eq!(offered(&screen), Some(MissionId(130)));
    screen.input(&key(Key::Char('n'), true));
    screen.input(&key(Key::Char('h'), true));
    assert!(
        spaceport(&screen)
            .open_bar()
            .and_then(BarScreen::hire_screen)
            .is_some(),
        "for hire"
    );
    screen.tick(Duration::from_millis(1));
    screen.input(&key(Key::Escape, true));
    screen.tick(Duration::from_secs(2));
    screen.tick(Duration::from_secs(2));
    assert_eq!(offered(&screen), Some(MissionId(131)), "rearmed");
}

#[test]
fn the_spaceport_does_not_offer_again_on_returning_from_a_service() {
    let store = MemoryPilots::new();
    let mut screen = landed_at_venues(&store, venue_data_of(SPACEPORT, &[130, 131]));
    assert_eq!(
        offered_mission(spaceport(&screen).offer_dialog()),
        Some(MissionId(130)),
        "one offer as it lands"
    );
    screen.input(&key(Key::Char('n'), true));
    assert!(spaceport(&screen).offer_dialog().is_none(), "refused");
    screen.tick(Duration::from_secs(5));
    assert!(spaceport(&screen).offer_dialog().is_none(), "131 waits");
    click_port_item(&mut screen, 7);
    screen.input(&key(Key::Escape, true));
    assert!(spaceport(&screen).open_trade().is_none(), "back");
    screen.tick(Duration::from_secs(5));
    assert!(
        spaceport(&screen).offer_dialog().is_none(),
        "nothing on returning"
    );
}

// The picture variants, as the router lays them out.

/// A `dësc` `Graphic` above 127, so the picture variants show.
const SHOWN_GRAPHIC: i16 = 5008;

/// The picture variant of [`offer_template`], told apart by its size:
/// 400 x 250, its graphic (8) at (290, 10).
fn picture_offer_template() -> DialogTemplate {
    let mut template = offer_template();
    template.bounds = Bounds::at(Point::new(0.0, 0.0), 400.0, 250.0);
    template.items.resize(8, template.items[3].clone());
    template.items[7].bounds = Bounds::at(Point::new(290.0, 10.0), 100.0, 100.0);
    template
}

/// The picture variant of [`bar_template`], told apart by its size:
/// 266 x 306, its picture (8) above the text.
fn picture_bar_template() -> DialogTemplate {
    let mut template = bar_template();
    template.bounds = Bounds::at(Point::new(0.0, 0.0), 266.0, 306.0);
    template.items[7].bounds = Bounds::at(Point::new(16.0, 120.0), 230.0, 100.0);
    template
}

/// [`PastTheHoldsDialogs`], with the picture variants of the offer
/// (1020) and the bar (1021) too.
struct PictureDialogs;

impl DialogResources for PictureDialogs {
    fn dialog_template(&self, id: i16) -> Result<DialogTemplate, String> {
        match id {
            PICTURE_OFFER_DIALOG => Ok(picture_offer_template()),
            PICTURE_BAR_DIALOG => Ok(picture_bar_template()),
            _ => PastTheHoldsDialogs.dialog_template(id),
        }
    }
}

/// A 2 x 2 `PICT` of one colour.
fn small_pict() -> Vec<u8> {
    let frame = [0, 0, 2, 2];
    PictBuilder::new(frame)
        .direct_bits(&DirectBits::rgb888(frame, &[[90, 90, 90]; 4]))
        .end()
        .build()
}

/// The first `chär` flies ship 128 in system 128, whose one Person
/// slot, at 100 %, holds `përs` 128, independent, in ship 129, of
/// `Flags` 0x0040, so hailing it offers its `LinkMission` 128 (offered
/// from a ship on every visit) in place of the comm dialog; the
/// mission's offer text, `dësc` 4000, has `Graphic` [`SHOWN_GRAPHIC`].
fn person_offer_data() -> Rc<GameData> {
    loaded(&person_offer_fork(|_| {}))
}

/// [`person_offer_data`]'s fork, its `mïsn` 128 fitted by `fit`.
fn person_offer_fork(fit: impl FnOnce(&mut Vec<u8>)) -> ForkBuilder {
    person_offer_fork_of(fit, 1)
}

/// [`person_offer_fork`] with the system's first `persons` Person slots
/// at 100 %, slot n holding Person 128 + n.
fn person_offer_fork_of(fit: impl FnOnce(&mut Vec<u8>), persons: usize) -> ForkBuilder {
    let put = |bytes: &mut [u8], at: usize, values: &[i16]| {
        for (i, value) in values.iter().enumerate() {
            bytes[at + 2 * i..at + 2 * i + 2].copy_from_slice(&value.to_be_bytes());
        }
    };
    let ship = |crew| {
        ShipBuilder::new()
            .accel(300)
            .speed(300)
            .maneuver(10)
            .fuel(300)
            .armor(100)
            .weap_types(&[-1; 8])
            .inherent_ai(1)
            .crew(crew)
            .default_items(&[-1; 8])
            .build()
    };
    let anim = ShipAnimBuilder::new()
        .base_image_id(1000)
        .base_set_count(1)
        .frames_per(4)
        .build();
    let sheet = (0..4)
        .fold(RledBuilder::new(1, 1), |sheet, _| {
            sheet.frame(|f| f.line().pixels(&[0x7C00]))
        })
        .build();
    let mut character = vec![0; Character::SIZE.expect("fixed")];
    put(&mut character, 0x04, &[128, 128, -1, -1, -1]);
    put(&mut character, 0x0E, &[-1; 4]);
    let mut system = vec![0; System::SIZE.expect("fixed")];
    put(&mut system, 0x04, &[-1; 32]);
    put(&mut system, 0x44, &[-1; 8]);
    put(&mut system, 0x64, &[0, -1]);
    let slots: Vec<i16> = (0..8)
        .map(|n| if n < persons { 128 + n as i16 } else { -1 })
        .collect();
    put(&mut system, 0x6E, &slots);
    put(&mut system, 0x7E, &vec![100; persons]);
    let mut person = vec![0; Person::SIZE.expect("fixed")];
    put(&mut person, 0x00, &[-2, -1, 3, 2, 0, 129]);
    put(&mut person, 0x0C, &[-1; 4]);
    put(&mut person, 0x2A, &[-1, -1, -1, 128]);
    person[0x32..0x34].copy_from_slice(&0x0040_u16.to_be_bytes());
    let mut mission = vec![0; Mission::SIZE.expect("fixed")];
    put(&mut mission, 0x04, &[2]);
    put(&mut mission, 0x0A, &[100]);
    put(&mut mission, 0x34, &[-1]);
    put(&mut mission, 0x58, &[-1]);
    fit(&mut mission);
    ForkBuilder::new()
        .resource(Ship::TYPE, 128, Some(b"First"), &ship(10))
        .resource(Ship::TYPE, 129, Some(b"Hauler"), &ship(3))
        .resource(Character::TYPE, 128, Some(b"Pilot"), &character)
        .resource(ShipAnim::TYPE, 128, None, &anim)
        .resource(ShipAnim::TYPE, 129, None, &anim)
        .resource(RLED, 1000, None, &sheet)
        .resource(System::TYPE, 128, Some(b"Alpha"), &system)
        .resource(Person::TYPE, 128, Some(b"Merchant"), &person)
        .resource(Mission::TYPE, 128, Some(b"Ride"), &mission)
        .resource(
            Desc::TYPE,
            128 + 3872,
            Some(b"Offer"),
            &pictured_desc_of("Ride along.", SHOWN_GRAPHIC),
        )
}

/// [`person_offer_data`] under the status bar ([`barred`]), where
/// `mïsn` 128's `BriefText` is `dësc` 5000, "Go.", and its `OnAccept`
/// is `G133`, a fuel tank of 100.
fn briefing_person_offer_data() -> Rc<GameData> {
    let mut tank = booster_record(b"");
    put_i16s(&mut tank, 0x02, &[0]);
    put_i16s(&mut tank, 0x06, &[12, 100]);
    let fork = person_offer_fork(|mission| {
        put_i16s(mission, 0x34, &[5000]);
        mission[0x15B..0x15F].copy_from_slice(b"G133");
    })
    .resource(Outfit::TYPE, 133, Some(b"Tank"), &tank)
    .resource(Desc::TYPE, 5000, Some(b"Brief"), &desc_of("Go."));
    loaded(&barred(fork))
}

/// [`PictureDialogs`] without "Desc Dialog" (3003, 3004): mission
/// texts are dropped as they come.
struct NoTextDialogs;

impl DialogResources for NoTextDialogs {
    fn dialog_template(&self, id: i16) -> Result<DialogTemplate, String> {
        if id == DESC_DIALOG || id == PICTURE_DESC_DIALOG {
            return Err(format!("no DLOG {id}"));
        }
        PictureDialogs.dialog_template(id)
    }
}

/// The router over [`briefing_person_offer_data`] with `dialogs`, in
/// the developer's flight, the tank half full, hailing the merchant:
/// its offer open.
fn hailing_briefer(dialogs: Rc<dyn DialogResources>) -> AppScreen {
    let mut screen = AppScreen::new(briefing_person_offer_data())
        .with_dialogs(dialogs, Rc::new(MonoMetrics))
        .with_behaviour(Rc::new(Idle));
    fly(&mut screen);
    edit(
        &mut screen,
        PilotEdit::Reserve(nova_sim::Reserve::Fuel, 150.0),
    );
    hail(&mut screen);
    assert_eq!(screen.showing(), Showing::Offer);
    screen
}

#[test]
fn a_persons_brief_shows_over_flights_pre_accept_bar_until_it_closes() {
    let mut screen = hailing_briefer(Rc::new(PictureDialogs));
    let half = fuel_width(&drawn(&screen)).expect("some fuel");
    screen.input(&key(Key::Enter, true));
    assert_eq!(screen.showing(), Showing::MissionText, "the brief");
    assert_eq!(
        fuel_width(&drawn(&screen)),
        Some(half),
        "the bar from before OnAccept's tank"
    );
    screen.input(&key(Key::Enter, true));
    assert_eq!(screen.showing(), Showing::Flight, "closed");
    let width = fuel_width(&drawn(&screen)).expect("some fuel");
    assert!(width > half, "live again, the tank's fuel too: {width}");
}

#[test]
fn a_persons_brief_dropped_over_flight_lets_go_of_the_bar() {
    let mut screen = hailing_briefer(Rc::new(NoTextDialogs));
    let half = fuel_width(&drawn(&screen)).expect("some fuel");
    screen.input(&key(Key::Enter, true));
    assert_eq!(screen.showing(), Showing::Flight, "the brief dropped");
    let width = fuel_width(&drawn(&screen)).expect("some fuel");
    assert!(width > half, "live: {width}");
}

/// Whether `screen` draws `text`.
fn draws_text(screen: &AppScreen, text: &str) -> bool {
    drawn(screen)
        .iter()
        .any(|command| matches!(command, DrawCommand::Text { text: t, .. } if t == text))
}

#[test]
fn without_dialogs_a_persons_brief_dropped_over_flight_lets_go_of_the_bar() {
    let mut screen = AppScreen::new(briefing_person_offer_data()).with_behaviour(Rc::new(Idle));
    fly(&mut screen);
    edit(
        &mut screen,
        PilotEdit::Reserve(nova_sim::Reserve::Fuel, 150.0),
    );
    hail(&mut screen);
    assert_eq!(screen.showing(), Showing::Offer, "its fallback layout");
    let half = fuel_width(&drawn(&screen)).expect("some fuel");
    screen.input(&key(Key::Enter, true));
    assert_eq!(screen.showing(), Showing::Flight, "the brief dropped");
    assert_eq!(active(&screen), [MissionId(128)], "accepted");
    let width = fuel_width(&drawn(&screen)).expect("some fuel");
    assert!(width > half, "the gauges live: {width}");
    let hyperspace = nova_view::flight::hud::NAV_HYPERSPACE;
    assert!(!draws_text(&screen, hyperspace), "not yet in mode 3");
    // H, flight's hyperspace mode key.
    screen.input(&key(Key::Char('h'), true));
    screen.input(&key(Key::Char('h'), false));
    assert!(draws_text(&screen, hyperspace), "the nav live");
}

#[test]
fn a_persons_offer_with_a_graphic_above_127_opens_over_flight_in_1020() {
    let mut screen = AppScreen::new(person_offer_data())
        .with_dialogs(Rc::new(PictureDialogs), Rc::new(MonoMetrics))
        .with_behaviour(Rc::new(Idle));
    fly(&mut screen);
    hail(&mut screen);
    assert_eq!(screen.showing(), Showing::Offer, "over the paused flight");
    let dialog = screen.offer().expect("open");
    assert_eq!(dialog.prompt().mission, MissionId(128));
    assert_eq!(dialog.prompt().graphic, Some(SHOWN_GRAPHIC));
    assert_eq!(dialog.problem(), None);
    assert_eq!(
        dialog.dialog().map(Dialog::bounds),
        Some(picture_offer_template().bounds),
        "laid out by DLOG 1020"
    );
    assert!(
        screen
            .take_sounds()
            .contains(&Sound::Sim(SimSound::HailOffer)),
        "the hail's sound (@0x6223d)"
    );
}

#[test]
fn a_bar_desc_whose_graphic_is_above_127_and_whose_pict_exists_opens_in_1021() {
    let fork = game_data_fork(true, true, true, 100, b"", b"", None)
        .resource(
            Desc::TYPE,
            10_000,
            None,
            &pictured_desc_of("A smoky bar.", SHOWN_GRAPHIC),
        )
        .resource(PICT, SHOWN_GRAPHIC, None, &small_pict());
    let store = MemoryPilots::new();
    let mut screen = AppScreen::new(loaded(&fork))
        .with_dialogs(Rc::new(PictureDialogs), Rc::new(MonoMetrics))
        .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
        .appeared();
    create(&mut screen, "Ada");
    land_twice(&mut screen);
    click_port_item(&mut screen, 11);
    let bar = spaceport(&screen).open_bar().expect("in the bar");
    assert_eq!(bar.problem(), None);
    assert_eq!(
        bar.dialog().map(Dialog::bounds),
        Some(picture_bar_template().bounds),
        "laid out by DLOG 1021"
    );
}

/// A person's offer and a mission text over flight take the rulebook's
/// `dialog_focus_ring` as they open.
#[test]
fn an_offer_and_a_mission_text_open_with_the_rulebooks_focus_ring() {
    for (rules, ring) in focus_ring_readings() {
        let mut screen = AppScreen::new(person_offer_data())
            .with_dialogs(Rc::new(PictureDialogs), Rc::new(MonoMetrics))
            .with_behaviour(Rc::new(Idle))
            .with_rules(rules);
        fly(&mut screen);
        hail(&mut screen);
        assert_eq!(
            screen
                .offer()
                .and_then(OfferDialog::dialog)
                .map(nova_view::ui::Dialog::focus_ring),
            Some(ring),
            "the offer"
        );
        let (screen, _) = filmed(0, "Job.", 0, Prefs::default());
        let mut screen = screen.with_rules(rules);
        create(&mut screen, "Ada");
        screen.show_over_flight(vec![DescShown {
            text: "Well done.".to_owned(),
            graphic: None,
            movie: None,
        }]);
        assert_eq!(
            screen
                .flight_mission_text()
                .map(|text| text.dialog().focus_ring()),
            Some(ring),
            "the mission text"
        );
    }
}

/// A router in flight holding the merchant's mission 128, whose
/// `QuickBrief` names "Quick.mov", hailing a second merchant, Person 129:
/// its offer of mission 130 open over flight, whose `dësc` names
/// "Offer.mov" as its after-movie; and the reel the movies play on.
fn holding_and_offered_in_flight() -> (AppScreen, Rc<MockMoviePlayer>) {
    let mut person = vec![0; Person::SIZE.expect("fixed")];
    put_i16s(&mut person, 0x00, &[-2, -1, 3, 2, 0, 129]);
    put_i16s(&mut person, 0x0C, &[-1; 4]);
    put_i16s(&mut person, 0x2A, &[-1, -1, -1, 130]);
    person[0x32..0x34].copy_from_slice(&0x0040_u16.to_be_bytes());
    let mut offered = vec![0; Mission::SIZE.expect("fixed")];
    put_i16s(&mut offered, 0x04, &[2]);
    put_i16s(&mut offered, 0x0A, &[100]);
    put_i16s(&mut offered, 0x34, &[-1]);
    put_i16s(&mut offered, 0x58, &[-1]);
    let fork = person_offer_fork_of(|held| put_i16s(held, 0x36, &[6001]), 2)
        .resource(Person::TYPE, 129, Some(b"Second"), &person)
        .resource(Mission::TYPE, 130, Some(b"Second job"), &offered)
        .resource(
            Desc::TYPE,
            130 + 3872,
            Some(b"Offer"),
            &filmed_desc_of("Job.", "Offer.mov", 0x0001),
        )
        .resource(
            Desc::TYPE,
            6001,
            Some(b"Quick"),
            &filmed_desc_of("Quick.", "Quick.mov", 0x0001),
        );
    let reel = MockMoviePlayer::playing((160, 120), 2);
    let mut screen = AppScreen::new(loaded(&fork))
        .with_movie_player(Rc::clone(&reel) as Rc<dyn MoviePlayer>)
        .with_dialogs(Rc::new(InfoOverEveryOpener), Rc::new(MonoMetrics))
        .with_behaviour(Rc::new(Idle));
    fly(&mut screen);
    hail_person(&mut screen, 128);
    assert_eq!(
        screen.offer().map(|o| o.prompt().mission),
        Some(MissionId(128))
    );
    screen.input(&key(Key::Enter, true));
    screen.input(&key(Key::Enter, false));
    assert_eq!(active(&screen), [MissionId(128)], "held");
    assert_eq!(screen.showing(), Showing::Flight);
    hail_person(&mut screen, 129);
    assert_eq!(
        screen.offer().map(|o| o.prompt().mission),
        Some(MissionId(130)),
        "the second merchant's offer"
    );
    reel.log.borrow_mut().clear();
    (screen, reel)
}

/// Tabs to Person `id` and hails it.
fn hail_person(screen: &mut AppScreen, id: i16) {
    screen.tick(TICK);
    for _ in 0..4 {
        screen.input(&key(Key::Tab, true));
        screen.input(&key(Key::Tab, false));
        let targeted = flight(screen)
            .session()
            .ok()
            .and_then(|session| session.target())
            .and_then(|npc| npc.person)
            .map(|person| person.id.0);
        if targeted == Some(id) {
            screen.take_sounds();
            screen.input(&key(Key::Char('y'), true));
            screen.input(&key(Key::Char('y'), false));
            return;
        }
    }
    panic!("never targeted {id}");
}

fn press_mission_info(screen: &mut AppScreen) {
    screen.input(&key(nova_view::ui::mission_info::MISSION_INFO_KEY, true));
    screen.input(&key(nova_view::ui::mission_info::MISSION_INFO_KEY, false));
}

#[test]
fn i_on_a_persons_offer_over_flight_opens_mission_info_over_it_with_no_map() {
    let (mut screen, _reel) = holding_and_offered_in_flight();
    let still = ship(&screen);
    press_mission_info(&mut screen);
    assert!(
        screen.flight_mission_info().is_some(),
        "info open (@0xa3344)"
    );
    assert!(screen.offer().is_some(), "over the offer");
    assert_eq!(screen.showing(), Showing::MissionInfo);
    ticked(&mut screen, 5);
    assert_eq!(ship(&screen), still, "flight paused");
    screen.take_sounds();
    screen.input(&key(Key::Char('m'), true));
    screen.input(&key(Key::Char('m'), false));
    assert!(!flight(&screen).dialog_map_open(), "no map (@0x9e3c6)");
    assert!(screen.flight_mission_info().is_some(), "info still open");
    assert!(
        !screen
            .take_sounds()
            .iter()
            .any(|sound| matches!(sound, Sound::Ui(UiSound::MapOpened))),
        "no map sound"
    );
}

#[test]
fn closing_the_mission_info_over_a_flight_offer_reloads_the_offers_after_movie() {
    let (mut screen, reel) = holding_and_offered_in_flight();
    press_mission_info(&mut screen);
    assert!(screen.flight_mission_info().is_some(), "info open");
    screen.input(&key(Key::Escape, true));
    screen.input(&key(Key::Escape, false));
    assert!(screen.flight_mission_info().is_none(), "info closed");
    assert_eq!(screen.showing(), Showing::Offer, "back to the offer");
    assert!(
        screen.offer().is_some_and(|offer| !offer.answered()),
        "unanswered"
    );
    screen.input(&key(Key::Char('n'), true));
    ticked(&mut screen, 3);
    assert_eq!(
        reeled(&reel),
        [Call::Start("Offer.mov".into()), Call::Stop],
        "the info screen loaded the QuickBrief's, and its close loaded the offer's \
         again (@0xa335c)"
    );
}

#[test]
fn i_on_a_persons_offer_over_flight_with_no_mission_held_does_nothing() {
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    screen.open_offer(nova_sim::OfferPrompt {
        mission: MissionId(130),
        text: "Job.".to_owned(),
        labels: nova_sim::mission::offer_labels("", "", true),
        graphic: None,
        movie: None,
    });
    screen.take_sounds();
    screen.input(&key(Key::Command, true));
    press_mission_info(&mut screen);
    assert!(screen.flight_mission_info().is_none(), "nothing (@0xa32ee)");
    assert!(screen.missions.is_none(), "no dialogs built");
    assert_eq!(screen.showing(), Showing::Offer);
    assert_eq!(screen.take_sounds(), [], "no sound");
    // No flush either: Command is still held, so Command-Q quits.
    assert_eq!(
        screen.input(&key(Key::Char('q'), true)),
        ScreenAction::Quit,
        "Command still held"
    );
    screen.input(&key(Key::Char('q'), false));
    screen.input(&key(Key::Command, false));
    screen.input(&key(Key::Enter, true));
    screen.input(&key(Key::Enter, false));
    assert!(screen.offer().is_none(), "answered");
}

#[test]
fn the_mission_info_opening_over_a_flight_offer_lets_go_of_the_keys_held() {
    // Command held as I opens the info is let go (`FlushEvents`
    // @0xa32f4-0xa333f), so a Q alone after doesn't quit.
    let (mut screen, _reel) = holding_and_offered_in_flight();
    screen.input(&key(Key::Command, true));
    press_mission_info(&mut screen);
    assert!(screen.flight_mission_info().is_some(), "info open");
    assert_ne!(
        screen.input(&key(Key::Char('q'), true)),
        ScreenAction::Quit,
        "Command let go"
    );
}

#[test]
fn the_mission_info_closing_over_a_flight_offer_lets_go_of_the_keys_held() {
    // As the port's close does (`note_info_closed`).
    let (mut screen, _reel) = holding_and_offered_in_flight();
    press_mission_info(&mut screen);
    screen.input(&key(Key::Command, true));
    screen.input(&key(Key::Escape, true));
    assert!(screen.flight_mission_info().is_none(), "info closed");
    assert_ne!(
        screen.input(&key(Key::Char('q'), true)),
        ScreenAction::Quit,
        "Command let go"
    );
}

#[test]
fn the_mission_info_opening_over_a_flight_offer_lets_go_of_the_pointer_held() {
    // A press on the offer's Refuse held as I opens the info is let go
    // (`FlushEvents` @0xa32f4-0xa333f), so its release after the close
    // clicks nothing.
    let (mut screen, _reel) = holding_and_offered_in_flight();
    let at = screen
        .offer()
        .and_then(|offer| offer.dialog())
        .and_then(|dialog| dialog.item_bounds(nova_view::spaceport::offer::REFUSE_ITEM))
        .expect("the Refuse button")
        .center();
    let pointer = |pressed| Input::PointerButton {
        button: MouseButton::Left,
        pressed,
        at,
    };
    screen.input(&pointer(true));
    press_mission_info(&mut screen);
    assert!(screen.flight_mission_info().is_some(), "info open");
    screen.input(&key(Key::Escape, true));
    screen.input(&key(Key::Escape, false));
    assert!(screen.flight_mission_info().is_none(), "info closed");
    screen.take_sounds();
    screen.input(&pointer(false));
    assert!(
        screen.offer().is_some_and(|offer| !offer.answered()),
        "the offer still open and unanswered"
    );
    assert_eq!(screen.take_sounds(), [], "no click or button sound");
}
