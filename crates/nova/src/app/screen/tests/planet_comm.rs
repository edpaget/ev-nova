//! The planet comm dialog over flight (see the screen's `planet_comm`
//! module).

use super::*;
use nova_sim::planet_comm::{PlanetCommButton, PlanetCommItem};
use nova_view::ui::planet_comm::PLANET_COMM_DIALOG;

/// Every draw its last outcome, no percentage firing: a bribe roll of
/// 99, and a haggle roll of 99, which loses.
struct Last;

impl Chance for Last {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    fn below(&mut self, n: u32) -> u32 {
        n - 1
    }
}

/// The first `chär` holds 1,000,000 credits and flies ship 128 in system
/// 128, which holds Alpha Prime (128), independent and of `MinStatus`
/// `min_status`; `STR#` 3002 holds "s<n>" for each n up to 60.
fn planet_data(min_status: i16) -> Rc<GameData> {
    let anim = ship_anim();
    let sheet = (0..4)
        .fold(RledBuilder::new(1, 1), |sheet, _| {
            sheet.frame(|f| f.line().pixels(&[0x7C00]))
        })
        .build();
    let mut character = vec![0; Character::SIZE.expect("fixed")];
    character[0x00..0x04].copy_from_slice(&1_000_000_i32.to_be_bytes());
    character[0x04..0x06].copy_from_slice(&128_i16.to_be_bytes());
    character[0x06..0x08].copy_from_slice(&128_i16.to_be_bytes());
    for slot in 1..4 {
        let at = 0x06 + 2 * slot;
        character[at..at + 2].copy_from_slice(&(-1_i16).to_be_bytes());
    }
    let mut stellar = landable();
    stellar[0x14..0x18].copy_from_slice(&[0xFF, 0xFF, 0, 0]);
    stellar[0x16..0x18].copy_from_slice(&min_status.to_be_bytes());
    let mut spin = vec![0; Spin::SIZE.expect("fixed")];
    spin[0..2].copy_from_slice(&1000_i16.to_be_bytes());
    let replies: Vec<String> = (1..=60).map(|n| format!("s{n}")).collect();
    let fork = with_main_screen(ForkBuilder::new())
        .resource(
            Ship::TYPE,
            128,
            Some(b"First"),
            &ShipBuilder::new()
                .accel(300)
                .speed(300)
                .maneuver(10)
                .build(),
        )
        .resource(Character::TYPE, 128, Some(b"Pilot"), &character)
        .resource(ShipAnim::TYPE, 128, None, &anim)
        .resource(RLED, 1000, None, &sheet)
        .resource(System::TYPE, 128, Some(b"Alpha"), &system(0, &[128]))
        .resource(Stellar::TYPE, 128, Some(b"Alpha Prime"), &stellar)
        .resource(Spin::TYPE, 1000, None, &spin)
        .resource(StrList::TYPE, 3002, None, &str_list(&as_strs(&replies)))
        .build()
        .bytes;
    Rc::new(one_file(fork).load())
}

fn as_strs(strings: &[String]) -> Vec<&str> {
    strings.iter().map(String::as_str).collect()
}

fn tap(screen: &mut AppScreen, k: Key) {
    screen.input(&key(k, true));
    screen.input(&key(k, false));
}

/// Flies `screen`, then selects and hails Alpha Prime, taking the sounds.
fn hail_alpha(screen: &mut AppScreen) {
    fly(screen);
    tap(screen, Key::Char('1'));
    screen.tick(TICK);
    tap(screen, Key::Char('y'));
    let _ = screen.take_sounds();
}

/// The router over [`planet_data`] with dialogs, in flight, Alpha Prime
/// selected and hailed, its sounds taken.
fn hailing(min_status: i16) -> AppScreen {
    let shared: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(Last));
    let mut screen = with_dialogs(planet_data(min_status)).with_chance(SharedChance::new(shared));
    hail_alpha(&mut screen);
    screen
}

fn cash(screen: &AppScreen) -> i64 {
    flight(screen).session().expect("flying").pilot().cash
}

#[test]
fn y_on_a_stellar_that_answers_opens_the_planet_comm_dialog_over_paused_flight() {
    let screen = &mut hailing(0);
    assert_eq!(screen.showing(), Showing::Comm, "as the comm dialog shows");
    let dialog = screen.planet_comm().expect("open");
    assert_eq!(dialog.view().name, "Alpha Prime");
    assert_eq!(dialog.view().reply, "s5Alpha Prime.", "STR# 3002 #(4 + 1)");
    assert_eq!(dialog.view().second, PlanetCommButton::Greetings);
    let alpha = flight(screen).alpha();
    screen.tick(TICK / 2);
    assert_eq!(
        flight(screen).alpha().to_bits(),
        alpha.to_bits(),
        "flight is paused"
    );
}

#[test]
fn close_channel_closes_the_planet_comm_dialog_and_resumes_flight() {
    let screen = &mut hailing(0);
    tap(screen, Key::Escape);
    assert!(screen.planet_comm().is_none());
    assert_eq!(screen.showing(), Showing::Flight);
    assert!(
        screen
            .take_sounds()
            .contains(&Sound::Sim(SimSound::PlanetCommClosed)),
        "_beepSnd[2] @0x97aa4"
    );
}

#[test]
fn offer_bribe_opens_the_haggle_dialog_and_accepting_pays_closing_both() {
    let screen = &mut hailing(1);
    let dialog = screen.planet_comm().expect("open");
    assert_eq!(dialog.view().second, PlanetCommButton::OfferBribe);
    tap(screen, Key::Char('o'));
    assert_eq!(screen.showing(), Showing::Haggle);
    assert_eq!(
        screen.haggle().expect("open").lines(),
        ["Pay us 3,000 credits."],
        "of a planet"
    );
    tap(screen, Key::Enter);
    assert!(screen.haggle().is_none());
    assert!(screen.planet_comm().is_none(), "a bribe paid closes it");
    assert_eq!(screen.showing(), Showing::Flight);
    assert_eq!(cash(screen), 1_000_000 - 3000);
}

#[test]
fn a_lost_bribe_haggle_returns_to_the_planet_comm_dialog() {
    let screen = &mut hailing(1);
    tap(screen, Key::Char('b'));
    assert_eq!(screen.showing(), Showing::Haggle);
    tap(screen, Key::Char('l'));
    assert!(screen.haggle().is_none(), "a lost roll declines");
    assert_eq!(screen.showing(), Showing::Comm);
    assert_eq!(
        screen.planet_comm().expect("open").view().reply,
        "s35",
        "STR# 3002 #(4 + 31)"
    );
    assert_eq!(cash(screen), 1_000_000, "nothing paid");
}

#[test]
fn without_the_interface_files_dialog_the_built_in_one_opens_with_a_warning() {
    let screen = &mut hailing(0);
    assert_eq!(screen.showing(), Showing::Comm);
    assert_eq!(
        screen.planet_comm().expect("open").view().name,
        "Alpha Prime"
    );
    assert_eq!(
        screen.take_warnings(),
        [format!(
            "nova: using the built-in planet comm dialog: no DLOG {PLANET_COMM_DIALOG}"
        )],
        "the test dialogs hold no planet comm DLOG"
    );
}

#[test]
fn without_any_metrics_a_planet_hail_is_closed() {
    let screen = &mut AppScreen::new(planet_data(0));
    hail_alpha(screen);
    assert_eq!(screen.showing(), Showing::Flight);
    assert!(screen.planet_comm().is_none());
    assert_eq!(
        flight_mut(screen).press_planet_comm(PlanetCommItem::Second),
        None,
        "the flight's planet comm dialog is closed"
    );
    assert_eq!(
        screen.take_warnings(),
        ["nova: cannot show the planet comm dialog: no metrics to lay it out"]
    );
}
