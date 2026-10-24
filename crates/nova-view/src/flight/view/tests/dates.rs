//! The arrival messages and the take-off greeting write the long date,
//! as the original's `_DateToLongString` (@0x8ef8) does: the day with
//! its English ordinal.

use super::*;

/// [`catalog`] with the first `chär` starting on `day` of `month`, 1177,
/// over `base`'s systems.
fn starting(base: FakeCatalog, month: i16, day: i16) -> FakeCatalog {
    let Ok(start) = base.character.clone() else {
        panic!("a start");
    };
    FakeCatalog {
        character: Ok(CharacterStart {
            start: StartDate {
                day,
                month,
                year: 1177,
            },
            ..start
        }),
        ..base
    }
}

/// How a message reaches the player, from a session that starts on the
/// given day.
#[derive(Clone, Copy, Debug)]
enum Way {
    Jump,
    Hypergate,
    Wormhole,
    TakeOff,
}

/// The message `way` shows from a session started on `day` of `month`.
fn message_from(way: Way, month: i16, day: i16) -> String {
    let view = match way {
        Way::Jump => {
            let mut view = FlightView::new(starting(catalog(), month, day));
            jump_to_alpha(&mut view);
            view
        }
        Way::Hypergate => {
            let mut view = FlightView::new(starting(gated(), month, day));
            land_now(&mut view);
            let alpha = on_map(&mut view, 131);
            click(&mut view, alpha);
            tap(&mut view, MAP);
            view
        }
        Way::Wormhole => {
            let mut view = FlightView::new(starting(holed(), month, day));
            land_now(&mut view);
            view
        }
        Way::TakeOff => {
            let centred = FakeCatalog {
                sites: vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)],
                ..starting(catalog(), month, day)
            };
            let mut view = FlightView::new(centred);
            land_now(&mut view);
            assert_eq!(view.take_landing(), Some(StellarId(128)));
            assert_eq!(view.take_off(), Some(StellarId(128)));
            view
        }
    };
    view.message().expect("a message").to_owned()
}

#[test]
fn each_message_writes_the_long_date_with_its_ordinal() {
    let cases = [
        (
            Way::Jump,
            6,
            30,
            "Jumping into the Alpha Centauri system on July 1st, 1177 NC.",
        ),
        (
            Way::Jump,
            6,
            3,
            "Jumping into the Alpha Centauri system on June 4th, 1177 NC.",
        ),
        (
            Way::Jump,
            6,
            10,
            "Jumping into the Alpha Centauri system on June 11th, 1177 NC.",
        ),
        (
            Way::Jump,
            6,
            11,
            "Jumping into the Alpha Centauri system on June 12th, 1177 NC.",
        ),
        (
            Way::Jump,
            6,
            12,
            "Jumping into the Alpha Centauri system on June 13th, 1177 NC.",
        ),
        (
            Way::Jump,
            6,
            20,
            "Jumping into the Alpha Centauri system on June 21st, 1177 NC.",
        ),
        (
            Way::Jump,
            7,
            30,
            "Jumping into the Alpha Centauri system on July 31st, 1177 NC.",
        ),
        (
            Way::Hypergate,
            6,
            22,
            "Exiting hypergate in the Alpha Centauri system on June 22nd, 1177 NC.",
        ),
        (
            Way::Wormhole,
            6,
            23,
            "Passing through a wormhole into the Alpha Centauri system on June 23rd, 1177 NC.",
        ),
        (
            Way::TakeOff,
            6,
            1,
            "Launching from Earth on June 2nd, 1177 NC.",
        ),
    ];
    for (way, month, day, said) in cases {
        assert_eq!(
            message_from(way, month, day),
            said,
            "{way:?} from {month}/{day}"
        );
    }
}
