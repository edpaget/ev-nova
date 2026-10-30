//! The router over the spaceport's `STR#` texts' after-movies
//! (rdm:phase/movies/phase-4-desc-movies-outside-missions): an empty
//! outfitter's #206, an empty shipyard's #223, the bar's nothing for hire
//! (#224), the escorts' landing report and a sale alert each play the
//! last `dësc` loaded's after-movie once they close, as flight's session
//! reads it (`SessionFlight::loaded_after_movie`); by the other reading
//! of `desc_movie_globals`, none.

use super::*;
use nova_sim::{RuleKey, RuleSource, Rulebook};

/// Alpha Prime (128), whose `dësc` names "Port.mov" as its after-movie,
/// an outfitter and a shipyard selling nothing, under `rules`; movies
/// played through a new [`MockMoviePlayer`]; Ada landed there, the
/// landing's movie played out.
fn empty_shops(rules: Rulebook) -> (AppScreen, Rc<MockMoviePlayer>) {
    empty_shops_with(rules, |fork| fork, |_| {})
}

/// [`empty_shops`] over `fork`'s resources too, Ada as `kit` makes her
/// before she enters play.
fn empty_shops_with(
    rules: Rulebook,
    fork: impl FnOnce(ForkBuilder) -> ForkBuilder,
    kit: impl FnOnce(&mut nova_sim::Pilot),
) -> (AppScreen, Rc<MockMoviePlayer>) {
    let mut built = fork(
        game_data_fork(false, false, false, 100, b"", b"", None).resource(
            Desc::TYPE,
            128,
            None,
            &filmed_desc_of("A port.", "Port.mov", 0x0001),
        ),
    )
    .build();
    let stellar = built.layout.data_entry(Stellar::TYPE, 128) + 4;
    built.bytes[stellar + 0x09] |= 0x0C;
    let data = loaded_bytes(built.bytes);
    let store = MemoryPilots::new();
    let mut ada = nova_sim::Pilot::new(data.as_ref(), "Ada").expect("a pilot");
    ada.seen_intro = true;
    kit(&mut ada);
    store.put("Ada", &nova_sim::save::encode(&ada));
    let reel = MockMoviePlayer::playing((160, 120), 2);
    let mut screen = AppScreen::new(data)
        .with_rules(rules)
        .with_movie_player(Rc::clone(&reel) as Rc<dyn MoviePlayer>)
        .with_dialogs(Rc::new(InfoOverEveryOpener), Rc::new(MonoMetrics))
        .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
        .appeared();
    open_held(&mut screen);
    enter_ship(&mut screen);
    land_twice(&mut screen);
    for _ in 0..2 {
        screen.tick(Duration::from_millis(16));
    }
    assert_eq!(screen.showing(), Showing::Spaceport);
    (screen, reel)
}

/// The movies started after the landing's.
fn started_since_landing(reel: &MockMoviePlayer) -> Vec<Call> {
    reeled(reel)
        .into_iter()
        .skip(2)
        .filter(|call| *call != Call::Stop)
        .collect()
}

fn bible() -> Rulebook {
    Rulebook::default().with_override(RuleKey::DescMovieGlobals, RuleSource::Bible)
}

#[test]
fn an_empty_outfitters_text_plays_the_stellars_after_movie_once_it_closes() {
    let (mut screen, reel) = empty_shops(Rulebook::default());
    click_port_item(&mut screen, 8);
    assert_eq!(
        spaceport(&screen)
            .mission_text()
            .map(|text| text.lines().join(" ")),
        Some(nova_view::spaceport::outfitter::NO_ITEMS.to_owned())
    );
    assert_eq!(started_since_landing(&reel), [], "not before it closes");
    screen.input(&key(Key::Enter, true));
    assert_eq!(
        started_since_landing(&reel),
        [Call::Start("Port.mov".into())],
        "the stellar's dësc was the last loaded (@0x5f96b), and #206's \
         after-check reads it (@0x5bbf2, @0x5b40b)"
    );
}

#[test]
fn an_empty_shipyards_text_plays_the_stellars_after_movie_once_it_closes() {
    let (mut screen, reel) = empty_shops(Rulebook::default());
    click_port_item(&mut screen, 9);
    assert_eq!(
        spaceport(&screen)
            .mission_text()
            .map(|text| text.lines().join(" ")),
        Some(nova_view::spaceport::shipyard::NO_SHIPS.to_owned())
    );
    screen.input(&key(Key::Enter, true));
    assert_eq!(
        started_since_landing(&reel),
        [Call::Start("Port.mov".into())],
        "#223 (@0x5e8da)"
    );
}

#[test]
fn by_the_bibles_reading_the_shops_texts_play_no_movie() {
    for item in [8, 9] {
        let (mut screen, reel) = empty_shops(bible());
        click_port_item(&mut screen, item);
        assert!(spaceport(&screen).mission_text().is_some(), "{item}");
        screen.input(&key(Key::Enter, true));
        assert_eq!(started_since_landing(&reel), [], "{item}");
    }
}

#[test]
fn a_sale_alert_plays_the_selected_outfits_after_movie_once_it_closes() {
    let fork = game_data_fork(true, true, false, 100, b"", b"", Some(500)).resource(
        Desc::TYPE,
        3000,
        None,
        &filmed_desc_of("A booster.", "Booster.mov", 0x0001),
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
    click_port_item(&mut screen, 8);
    select_first_outfit(&mut screen);
    screen.input(&key(Key::Char('b'), true));
    screen.input(&key(Key::Right, true));
    screen.input(&key(Key::Char('b'), true));
    screen.input(&key(Key::Left, true));
    screen.input(&key(Key::Char('s'), true));
    let open = spaceport(&screen).open_outfitter().expect("outfitting");
    assert!(open.alert_dialog().is_some(), "the sale refused");
    assert_eq!(reeled(&reel), [], "not under the alert");
    screen.input(&key(Key::Escape, true));
    assert_eq!(
        reeled(&reel),
        [Call::Start("Booster.mov".into())],
        "the selection's dësc was the last loaded (`_OutfitFilter` @0x58476), \
         and the alert's after-check reads it (@0x5c7a0, @0x5b40b)"
    );
    assert!(spaceport(&screen).open_outfitter().is_some(), "over it");
}

/// The bar's `dësc` (10000, `_DoBarDialog`'s load @0x48f91), naming
/// "Bar.mov" as its after-movie.
fn filmed_bar(fork: ForkBuilder) -> ForkBuilder {
    fork.resource(
        Desc::TYPE,
        10_000,
        None,
        &filmed_desc_of("A bar.", "Bar.mov", 0x0001),
    )
}

/// The bar's Hire Escort pressed with no ship for hire: `STR#` 2002 #224
/// shows; then whether it is up and the movies started since landing
/// once Done closes it, and once that movie has played out.
fn hire_with_nothing_for_hire(rules: Rulebook) -> Vec<Call> {
    let (mut screen, reel) = empty_shops_with(rules, filmed_bar, |_| {});
    click_port_item(&mut screen, 11);
    assert!(spaceport(&screen).open_bar().is_some(), "the bar");
    screen.input(&key(Key::Char('h'), true));
    assert_eq!(
        spaceport(&screen)
            .mission_text()
            .map(|text| text.lines().join(" ")),
        Some(nova_sim::hire::NONE_FOR_HIRE.to_owned())
    );
    assert_eq!(started_since_landing(&reel), [], "not before it closes");
    screen.input(&key(Key::Enter, true));
    assert!(spaceport(&screen).mission_text().is_none(), "closed");
    started_since_landing(&reel)
}

#[test]
fn nothing_for_hires_text_plays_the_bars_after_movie_once_it_closes() {
    assert_eq!(
        hire_with_nothing_for_hire(Rulebook::default()),
        [Call::Start("Bar.mov".into())],
        "the bar's dësc was the last loaded (@0x48f91), and #224's \
         after-check reads it (@0x5e8da, @0x5b40b)"
    );
}

#[test]
fn by_the_bibles_reading_nothing_for_hires_text_plays_no_movie() {
    assert_eq!(hire_with_nothing_for_hire(bible()), []);
}

/// Ada's one escort, a First she pays no wage, marked for sale.
fn selling_an_escort(ada: &mut nova_sim::Pilot) {
    ada.escorts.push(nova_sim::Escort {
        ship: nova_sim::ShipId(128),
        reserves: nova_sim::Reserves::full(10.0, 10.0, 100.0),
        order: None,
        carried: false,
        wage: None,
        person: None,
        mark: Some(nova_sim::fleet::EscortMark::Sell),
    });
}

/// Leave pressed with Ada's escort marked for sale at the shipyard: the
/// report shows and holds the leave; then the movies started since
/// landing once Done closes it, the router still over the spaceport
/// while one plays.
fn leave_with_a_sale(rules: Rulebook) -> (AppScreen, Vec<Call>) {
    let (mut screen, reel) = empty_shops_with(rules, |fork| fork, selling_an_escort);
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::Spaceport, "held");
    assert!(
        spaceport(&screen).mission_text().is_some(),
        "the report over the spaceport"
    );
    assert_eq!(started_since_landing(&reel), [], "not before it closes");
    screen.input(&key(Key::Enter, true));
    let started = started_since_landing(&reel);
    (screen, started)
}

#[test]
fn the_escorts_landing_report_plays_the_stellars_after_movie_before_the_take_off() {
    let (mut screen, started) = leave_with_a_sale(Rulebook::default());
    assert_eq!(
        started,
        [Call::Start("Port.mov".into())],
        "the stellar's dësc was the last loaded (@0x5f96b), and the \
         report's after-check reads it (@0x407e8, @0x5b40b)"
    );
    assert_eq!(screen.showing(), Showing::Spaceport, "held through it");
    for _ in 0..4 {
        screen.tick(Duration::from_millis(16));
    }
    assert_eq!(screen.showing(), Showing::Flight, "then the take-off");
    assert!(pilot(&screen).escorts().is_empty(), "sold");
}

#[test]
fn by_the_bibles_reading_the_escorts_landing_report_plays_no_movie() {
    let (screen, started) = leave_with_a_sale(bible());
    assert_eq!(started, []);
    assert_eq!(screen.showing(), Showing::Flight, "the leave goes on");
}
