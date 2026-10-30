//! The router over the landing's `dësc` load order
//! (rdm:phase/movies/phase-5-desc-load-order): `_DoPortDialog` loads the
//! port's own `dësc` (@0x5f96b) before the land checks (@0x5fbeb), and
//! the spaceport shows the text it loaded, never loading it again. So a
//! pickup no-space text with no landing text before it plays the port's
//! after-movie, and after a landing the last loaded is the last landing
//! text's, with movies on or off.

use super::*;

/// A `mïsn` the `chär`'s `OnStart` starts: to `travel`, carrying
/// `tons` of food loaded as `pickup` says (none for -1), completed at
/// `travel` with the `CompText` `comp`.
fn held_mission(travel: i16, tons: i16, pickup: i16, comp: i16) -> Vec<u8> {
    let mut bytes = vec![0; Mission::SIZE.expect("fixed")];
    let put = |bytes: &mut Vec<u8>, at: usize, values: &[i16]| {
        for (i, value) in values.iter().enumerate() {
            bytes[at + 2 * i..at + 2 * i + 2].copy_from_slice(&value.to_be_bytes());
        }
    };
    let cargo = if tons > 0 { 0 } else { -1 };
    // AvailStel, -, AvailLoc, AvailRecord, AvailRating, AvailRandom,
    // TravelStel, ReturnStel, CargoType, CargoQty, PickupMode, DropOffMode.
    put(
        &mut bytes,
        0x00,
        &[128, 0, 0, 0, 0, 100, travel, -1, cargo, tons, pickup, -1],
    );
    put(&mut bytes, 0x20, &[0, 0, -1, -1, 0, -1, 0, -1, 0]);
    put(&mut bytes, 0x32, &[-1, -1, -1, -1, -1, comp, -1, 0, 1, -1]);
    put(&mut bytes, 0x58, &[-1]);
    bytes
}

/// What the pilot holds as she lands at Alpha Prime (128).
#[derive(Clone, Copy)]
enum Held {
    /// Mission 130, 5 tons of food loaded at 128 (`PickupMode` 1), and
    /// mission 131, 6 tons loaded as it starts, to Beta Prime (129): 4
    /// tons free in a hull of 10, so 130's pickup finds no space.
    NoSpace,
    /// Mission 130, completed at 128 with the `CompText` `dësc` 5000.
    Completed,
}

/// The router over Alpha Prime (128), whose `dësc` names "Port.mov" as
/// its after-movie, with a 10-ton hull, the missions `held` names started
/// as the pilot begins, and `dësc` 5000 naming "Comp.mov" as its
/// after-movie; movies played through a new [`MockMoviePlayer`] as
/// `prefs` say; Ada flying, landed with two L presses.
fn landed(held: Held, prefs: Prefs) -> (AppScreen, Rc<MockMoviePlayer>) {
    let (on_start, missions): (&[u8], _) = match held {
        Held::NoSpace => (
            b"S130 S131",
            vec![
                (130, held_mission(128, 5, 1, -1)),
                (131, held_mission(129, 6, 0, -1)),
            ],
        ),
        Held::Completed => (b"S130", vec![(130, held_mission(128, 0, -1, 5000))]),
    };
    let fork = missions.iter().fold(
        game_data_fork(true, false, false, 100, on_start, b"", None),
        |fork, (id, mission)| fork.resource(Mission::TYPE, *id, Some(b"Held"), mission),
    );
    let fork = fork
        .resource(
            Desc::TYPE,
            128,
            None,
            &filmed_desc_of("A port.", "Port.mov", 0x0001),
        )
        .resource(
            Desc::TYPE,
            5000,
            None,
            &filmed_desc_of("Done.", "Comp.mov", 0x0001),
        );
    let reel = MockMoviePlayer::playing((160, 120), 2);
    let store = MemoryPilots::new();
    let mut screen = AppScreen::new(loaded(&fork))
        .with_prefs(prefs)
        .with_movie_player(Rc::clone(&reel) as Rc<dyn MoviePlayer>)
        .with_dialogs(Rc::new(InfoOverEveryOpener), Rc::new(MonoMetrics))
        .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
        .appeared();
    create(&mut screen, "Ada");
    assert!(
        active(&screen).contains(&MissionId(130)),
        "started by OnStart"
    );
    land_twice(&mut screen);
    (screen, reel)
}

fn movies_off() -> Prefs {
    Prefs {
        quicktime_movies: false,
        ..Prefs::default()
    }
}

/// The last `dësc` loaded's after-movie, as flight's session reads it.
fn after_movie(screen: &AppScreen) -> Option<String> {
    flight(screen)
        .sim()
        .loaded_after_movie()
        .map(|movie| movie.file)
}

/// The text shown over the spaceport, on one line.
fn text_shown(screen: &AppScreen) -> Option<String> {
    spaceport(screen)
        .mission_text()
        .map(|text| text.lines().join(" "))
}

/// The spaceport's own text, on one line.
fn port_text(screen: &AppScreen) -> Option<String> {
    spaceport(screen)
        .dialog()?
        .text(nova_view::spaceport::layout::TEXT_ITEM)
        .map(|text| text.lines().join(" "))
}

#[test]
fn with_movies_off_a_no_space_text_reads_the_ports_after_movie_and_starts_none() {
    let (screen, reel) = landed(Held::NoSpace, movies_off());
    assert_eq!(screen.showing(), Showing::Spaceport);
    assert!(text_shown(&screen).is_some(), "the no-space text");
    assert_eq!(
        after_movie(&screen).as_deref(),
        Some("Port.mov"),
        "the port's dësc loaded before the land checks (@0x5f96b), and the \
         STR# text loads none"
    );
    assert!(reeled(&reel).is_empty(), "movies off: none starts");
}

#[test]
fn with_movies_off_the_last_loaded_after_a_landing_is_its_comp_text() {
    let (screen, _) = landed(Held::Completed, movies_off());
    assert_eq!(text_shown(&screen).as_deref(), Some("Done."));
    assert_eq!(after_movie(&screen).as_deref(), Some("Comp.mov"));
}

/// Closes each text shown with Enter and ticks every movie out, until the
/// spaceport shows.
fn through_to_the_spaceport(screen: &mut AppScreen) {
    for _ in 0..8 {
        match screen.showing() {
            Showing::Spaceport if text_shown(screen).is_none() => return,
            Showing::MissionText | Showing::Spaceport => {
                screen.input(&key(Key::Enter, true));
            }
            _ => screen.tick(Duration::from_millis(16)),
        }
    }
    panic!("never reached the spaceport: {:?}", screen.showing());
}

// With movies on, the landing's texts show over its movie, where the
// original plays the movie (@0x632a7) before `_DoPortDialog` (@0x63381):
// rdm:phase/movies/phase-9-landing-movie-before-land-texts. These tests
// pin only what the load order decides.

#[test]
fn with_movies_on_the_no_space_text_plays_the_ports_after_movie() {
    let (mut screen, reel) = landed(Held::NoSpace, Prefs::default());
    assert_eq!(reeled(&reel), [Call::Start("Port.mov".into())], "landing");
    through_to_the_spaceport(&mut screen);
    let started: Vec<Call> = reeled(&reel)
        .into_iter()
        .filter(|call| *call != Call::Stop)
        .collect();
    assert_eq!(
        started,
        [
            Call::Start("Port.mov".into()),
            Call::Start("Port.mov".into())
        ],
        "the landing's movie, then the no-space text's after-movie, the \
         port's dësc the last loaded"
    );
}

#[test]
fn with_movies_on_the_last_loaded_after_a_landing_is_its_comp_text_not_the_ports() {
    let (mut screen, _) = landed(Held::Completed, Prefs::default());
    through_to_the_spaceport(&mut screen);
    assert_eq!(
        port_text(&screen).as_deref(),
        Some("A port."),
        "the port shows the text loaded before the land checks"
    );
    assert_eq!(
        after_movie(&screen).as_deref(),
        Some("Comp.mov"),
        "the spaceport opening after the movie loads the port's dësc no \
         second time"
    );
}
