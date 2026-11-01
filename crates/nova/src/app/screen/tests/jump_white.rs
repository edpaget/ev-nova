//! The router over a text shown over flight under the jump's white
//! (rdm:phase/movies/phase-4-desc-movies-outside-missions): a mission
//! text over flight while the warp has lit the screen white, such as a
//! `ShipDoneText` (`_MissionObjectivesCheck` @0x9ed0f), is queued on the
//! faded screen, so it fades in from black (`_FadeScreenIn` @0x5b038)
//! and the white is gone; once it closes, the warp lights the white
//! again on its next pass (@0x683f1-0x68404). The About text's path is
//! pinned through the whole app in `tests/it/hyperspace.rs`.

use super::*;
use nova_view::flight::JumpEffect;

/// The router with dialogs over [`game_data_fork`], where Alpha (128)
/// links Beta (129) and ship 128 is fast (`Accel` 1500, `Speed` 1000,
/// `Maneuver` 30) with `Fuel` 300, flying from Alpha with a course to
/// Beta, flown out past the minimum jump distance, J pressed and ticked
/// until the warp has lit the screen white.
fn lit() -> AppScreen {
    let built = jump_ready(&game_data_fork(false, false, false, 100, b"", b"", None));
    let mut screen = with_dialogs(loaded_bytes(built.bytes));
    fly(&mut screen);
    plot_beta(&mut screen);
    lit_from(screen)
}

/// `fork` built with Alpha (128) linking Beta (129) and ship 128 fast.
fn jump_ready(fork: &ForkBuilder) -> nova_rsrc::fixture::BuiltFork {
    let mut built = fork.build();
    let alpha = built.layout.data_entry(System::TYPE, 128) + 4;
    built.put_u16(alpha + 0x04, 129);
    let fast = built.layout.data_entry(Ship::TYPE, 128) + 4;
    for (at, value) in [(0x04, 1500), (0x06, 1000), (0x08, 30), (0x0A, 300)] {
        built.put_u16(fast + at, value);
    }
    built
}

/// [`lit`], the pilot in Ada's first flight off Alpha Prime with mission
/// 130 active.
fn lit_on_a_mission(store: &MemoryPilots) -> AppScreen {
    let fork = game_data_fork(true, true, true, 100, b"", b"", None)
        .resource(
            Mission::TYPE,
            130,
            Some(b"Venue job"),
            &venue_mission(SPACEPORT),
        )
        .resource(Desc::TYPE, 130 + 3872, Some(b"Offer"), &desc_of("Job 130."));
    let mut screen = landed_at_venues(store, loaded_bytes(jump_ready(&fork).bytes));
    screen.input(&key(Key::Enter, true));
    screen.input(&key(Key::Enter, false));
    assert_eq!(active(&screen), [MissionId(130)]);
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
    for _ in 0..120 {
        screen.tick(TICK);
    }
    plot_beta(&mut screen);
    lit_from(screen)
}

/// `screen` flying with a course to Beta, flown out past the minimum jump
/// distance, J pressed and ticked until the warp has lit the screen white.
fn lit_from(mut screen: AppScreen) -> AppScreen {
    screen.input(&key(Key::Up, true));
    for _ in 0..1800 {
        if ship(&screen).position.length() >= nova_sim::hyperspace::MIN_JUMP_DISTANCE {
            break;
        }
        screen.tick(TICK);
    }
    screen.input(&key(Key::Up, false));
    screen.input(&key(Key::Char('j'), true));
    screen.input(&key(Key::Char('j'), false));
    for _ in 0..1800 {
        if flight(&screen).screen_faded() {
            return screen;
        }
        screen.tick(TICK);
    }
    panic!("the warp never lit the white: {:?}", ship(&screen));
}

/// Plots a course to Beta on flight's map: M, a Shift-click on Beta, M,
/// whose close cues it.
fn plot_beta(screen: &mut AppScreen) {
    screen.input(&key(MAP, true));
    screen.input(&key(MAP, false));
    assert_eq!(screen.showing(), Showing::FlightMap);
    let at = port::scrolled_to(screen, SystemId(129));
    screen.input(&key(Key::Shift, true));
    for pressed in [true, false] {
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        });
    }
    screen.input(&key(Key::Shift, false));
    screen.input(&key(MAP, true));
    screen.input(&key(MAP, false));
    assert_eq!(screen.showing(), Showing::Flight);
    assert_eq!(
        flight(screen).session().expect("flying").course(),
        [nova_sim::SystemId(129)],
        "plotted"
    );
}

/// The white's alpha over flight, if the jump's effect shows one.
fn white(screen: &AppScreen) -> Option<u8> {
    flight(screen).jump_effect().map(JumpEffect::fade_alpha)
}

/// The alpha of the black drawn over the whole screen, last, if any: the
/// text's fade-in from black.
fn black(screen: &AppScreen) -> Option<u8> {
    drawn(screen)
        .iter()
        .rev()
        .find_map(|command| match *command {
            DrawCommand::Line { color, width, .. }
                if width >= 700.0 && (color.r, color.g, color.b) == (0, 0, 0) =>
            {
                Some(color.a)
            }
            _ => None,
        })
}

/// Ticks `screen` until its jump has arrived, or something opens.
fn tick_past_the_arrival(screen: &mut AppScreen) {
    for _ in 0..1800 {
        let under_way = flight(screen).session().expect("flying").jumping();
        if under_way.is_none() || screen.showing() != Showing::Flight {
            return;
        }
        screen.tick(TICK);
    }
    panic!("the jump never arrived");
}

/// A tap of `k` on `screen`.
fn tapped(screen: &mut AppScreen, k: Key) {
    screen.input(&key(k, true));
    screen.input(&key(k, false));
}

#[test]
fn m_i_p_and_y_under_the_warps_white_leave_the_game_showing_flight() {
    let mut screen = lit();
    for k in [
        MAP,
        nova_view::ui::mission_info::MISSION_INFO_KEY,
        PLAYER_INFO_KEY,
        Key::Char('y'),
    ] {
        tapped(&mut screen, k);
        assert_eq!(screen.showing(), Showing::Flight, "{k:?}");
        screen.tick(TICK);
        screen.tick(TICK);
        assert_eq!(screen.showing(), Showing::Flight, "{k:?} ticked");
    }
}

#[test]
fn m_on_a_collapse_under_the_white_opens_nothing_and_a_tap_after_it_opens_the_map() {
    let mut screen = lit();
    screen.input(&key(MAP, true));
    flight_mut(&mut screen).change_sim(|sim| {
        nova_sim::fixture::set_player_condition(
            sim.session_mut().expect("flying"),
            nova_sim::Condition::Disabled,
        );
    });
    screen.take_sounds();
    screen.tick(TICK);
    let session = flight(&screen).session().expect("flying");
    assert_eq!(session.jumping(), None, "collapsed");
    assert_eq!(screen.showing(), Showing::Flight, "the map refused");
    let beeps: Vec<UiSound> = screen
        .take_sounds()
        .into_iter()
        .filter_map(|sound| match sound {
            Sound::Ui(sound) => Some(sound),
            _ => None,
        })
        .collect();
    assert_eq!(beeps, [UiSound::MapOpened, UiSound::MapClosed]);
    screen.input(&key(MAP, false));
    tapped(&mut screen, MAP);
    assert_eq!(screen.showing(), Showing::FlightMap, "a plain press");
}

#[test]
fn m_after_the_arrival_opens_the_map_and_held_through_it_does_not() {
    let mut screen = lit();
    screen.input(&key(MAP, true));
    tick_past_the_arrival(&mut screen);
    screen.tick(TICK);
    assert_eq!(screen.showing(), Showing::Flight, "held, it lost its read");
    screen.input(&key(MAP, false));
    tapped(&mut screen, MAP);
    assert_eq!(screen.showing(), Showing::FlightMap);
}

#[test]
fn p_pressed_on_the_arrivals_pass_and_held_opens_its_dialog_from_the_tick() {
    // Pressed afresh on every pass, as one down earlier latched.
    let mut screen = lit();
    while flight(&screen)
        .session()
        .expect("flying")
        .jumping()
        .is_some()
    {
        screen.input(&key(PLAYER_INFO_KEY, false));
        screen.input(&key(PLAYER_INFO_KEY, true));
        screen.tick(TICK);
    }
    assert_eq!(screen.showing(), Showing::Flight, "the arrival's pass");
    screen.tick(TICK);
    assert_eq!(screen.showing(), Showing::PlayerInfo);
}

#[test]
fn i_held_through_the_arrival_opens_mission_info_from_the_tick() {
    let store = MemoryPilots::new();
    let mut screen = lit_on_a_mission(&store);
    screen.input(&key(nova_view::ui::mission_info::MISSION_INFO_KEY, true));
    tick_past_the_arrival(&mut screen);
    screen.tick(TICK);
    assert_eq!(screen.showing(), Showing::MissionInfo);
}

#[test]
fn a_mission_text_under_the_warps_white_fades_in_from_black_and_the_white_is_lit_again_after_it() {
    let mut screen = lit();
    assert!(white(&screen).is_some_and(|alpha| alpha > 0), "lit");
    screen.show_over_flight(vec![DescShown {
        text: "Done.".to_owned(),
        graphic: None,
        movie: None,
    }]);
    assert_eq!(screen.showing(), Showing::MissionText);
    assert!(
        !flight(&screen).screen_faded(),
        "_FadeScreenIn @0x5b038 jumps the display to black and fades it in"
    );
    assert_eq!(white(&screen), Some(0), "the white is gone");
    assert_eq!(black(&screen), Some(255), "from black");
    screen.input(&key(Key::Enter, true));
    screen.input(&key(Key::Enter, false));
    assert_eq!(
        screen.showing(),
        Showing::MissionText,
        "held through its fade-in"
    );
    let frame = Duration::from_nanos(1_000_000_000 / 60);
    // The fade-in's second, at 60 frames a second, and the frame after.
    for _ in 0..61 {
        screen.tick(frame);
    }
    assert_eq!(screen.showing(), Showing::Flight, "the Return reached it");
    // Flight's next pass: a tick, two frames, at most.
    for _ in 0..2 {
        if flight(&screen).screen_faded() {
            break;
        }
        screen.tick(frame);
    }
    assert!(flight(&screen).screen_faded(), "lit again");
    let relit = white(&screen).expect("still warping");
    assert!(
        relit < 128,
        "the white fades in again over 1.5 s on the next pass: {relit}"
    );
}

#[test]
fn a_mission_text_fading_in_under_the_warps_white_covers_the_widened_view() {
    use super::widescreen::{WIDE, black_across};
    let mut screen = lit();
    screen.set_widened_size(WIDE);
    screen.show_over_flight(vec![DescShown {
        text: "Done.".to_owned(),
        graphic: None,
        movie: None,
    }]);
    assert_eq!(
        black_across(&screen, WIDE.0).last(),
        Some(&255),
        "from black, over the whole widened view"
    );
    for _ in 0..30 {
        screen.tick(Duration::from_nanos(1_000_000_000 / 60));
    }
    assert!(
        black_across(&screen, WIDE.0)
            .last()
            .is_some_and(|&alpha| alpha > 0 && alpha < 255),
        "half way in, still over the whole view"
    );
}
