use super::*;

// Escorts.

#[test]
fn the_help_line_names_the_escort_keys() {
    assert!(
        HELP.contains("F/D/V/C: escorts   Alt-C: dock   E: escort menu"),
        "{HELP}"
    );
}

// Hiring escorts and paying them.

/// The adapter's own check (rdm:phase/view-sim-seam/phase-11-spaceport-on-port):
/// it hands the flight's chance to a rolled read, the hire list, and to
/// a command, the hire. What the view does with the answers runs on
/// `FakeFlight` (`tests/faked_shops.rs`).
#[test]
fn hiring_goes_through_the_session_on_the_flights_chance() {
    let always = Rc::new(RefCell::new(Always::default()));
    let shared: Rc<RefCell<dyn Chance>> = always.clone();
    let mut view = test_flight::flown(
        hiring(),
        FlightStart::New,
        Collaborators {
            chance: SharedChance::new(shared),
            ..Collaborators::default()
        },
    );
    assert_eq!(view.escorts_for_hire(), None, "in flight");
    assert_eq!(view.hire(ShipId(129)), Err(HireRefusal::NoBar));
    land_now(&mut view);
    view.take_save_due();
    let list = view.escorts_for_hire().expect("a bar");
    let row = list.row(ShipId(129)).expect("for hire");
    assert_eq!((row.fee, row.wage), (90, 9));
    assert_eq!(always.borrow().asked, [50]);
    let hired = view.hire(ShipId(129)).expect("hired");
    assert_eq!((hired.fee, hired.wage), (90, 9));
    assert_eq!(view.sim().pilot().map(Pilot::cash), Some(1000 - 90));
    assert_eq!(
        view.sim().pilot().expect("flying").escorts()[0].wage,
        Some(9)
    );
    assert!(view.take_save_due());
}

/// The adapter's own check: it hands the flight's chance to the bar's
/// opening, whose news it reads back.
#[test]
fn the_bars_news_is_drawn_through_the_session_on_the_flights_chance() {
    let always = Rc::new(RefCell::new(Always::default()));
    let shared: Rc<RefCell<dyn Chance>> = always.clone();
    let strings = FakeCatalog {
        strings: vec![
            (8100, vec!["C1".to_owned(), "C2".to_owned()]),
            (8101, vec!["G1".to_owned(), "G2".to_owned()]),
        ],
        ..catalog()
    };
    let mut view = open_flight(
        hiring(),
        FlightStart::New,
        SessionSetup {
            strings: Some(Rc::new(strings)),
            ..SessionSetup::default()
        },
        Collaborators {
            chance: SharedChance::new(shared),
            ..Collaborators::default()
        },
    );
    assert!(!view.open_bar(), "in flight");
    assert_eq!(view.news(), None);
    assert_eq!(view.news_picture(), None, "in flight");
    land_now(&mut view);
    assert_eq!(view.news_picture(), Some(9000), "no NewsPic");
    assert!(view.open_bar());
    assert_eq!(
        view.news(),
        Some(&nova_sim::News {
            commercial: "C2".to_owned(),
            story: "G2".to_owned(),
        }),
        "each the last string, as the flight's chance rolls"
    );
}
