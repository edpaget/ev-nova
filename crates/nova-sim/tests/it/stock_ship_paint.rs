//! Ship paint over the stock data (rdm play-feel-4 phase 5, see the
//! session's `ship_paint` module): two stock persons are painted, 639
//! `ShadeOfBlue` (`Color` 0x001822CD, (3, 4, 25)) and 640 "Zero Wing"
//! (0x00800000, (16, 0, 0)); no stock government's `ShipColor` paints
//! its ships. `ShadeOfBlue`, linked anywhere, flies painted where the
//! person roll brings it. Skips, passing, when `NOVA_DATA` is unset.

use crate::common;

use nova_data::GameData;
use nova_sim::flight_frame::FlightFrame;
use nova_sim::{Chance, CombatCatalog, PersonId, Pilot, Rgb15, Session, ShipRef, TrafficCatalog};

fn opened() -> Option<GameData> {
    let dir = common::nova_data()?;
    Some(GameData::open(&dir, None).expect("the stock data opens"))
}

#[test]
fn two_stock_persons_are_painted_and_no_stock_government() {
    let Some(data) = opened() else {
        return;
    };
    let painted: Vec<_> = TrafficCatalog::persons(&data)
        .into_iter()
        .filter(|person| person.color != 0)
        .map(|person| (person.id.0, person.color, Rgb15::of_rgb(person.color)))
        .collect();
    assert_eq!(
        painted,
        [
            (639, 0x0018_22CD, Some(Rgb15 { r: 3, g: 4, b: 25 })),
            (640, 0x0080_0000, Some(Rgb15 { r: 16, g: 0, b: 0 })),
        ]
    );
    let governments = data.governments();
    assert!(!governments.is_empty());
    assert!(
        governments.iter().all(|govt| govt.ship_color == 0),
        "no stock gövt paints its ships"
    );
}

/// The person roll fires and lands on `përs` 639 (128 + 511); no Person
/// slot below 100 % lists its person, and every other draw is 0.
struct ShadeOfBlue;

impl Chance for ShadeOfBlue {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    fn below(&mut self, n: u32) -> u32 {
        match n {
            1022 => 511,
            100 => 99,
            _ => 0,
        }
    }
}

#[test]
fn shade_of_blue_flies_in_its_paint() {
    let Some(data) = opened() else {
        return;
    };
    let pilot = Pilot::new(&data, "Stock").expect("the stock first chär starts");
    let mut session = Session::fly(&data, pilot).expect("flies");
    session.populate(&data, &mut ShadeOfBlue);
    let shade = session
        .npcs()
        .iter()
        .find(|npc| npc.person.map(|person| person.id) == Some(PersonId(639)))
        .expect("ShadeOfBlue flies")
        .id;
    let mut frame = FlightFrame::default();
    session.fill_frame(&mut frame);
    let record = frame.ship(ShipRef::Npc(shade)).expect("in the frame");
    assert_eq!(record.paint, Some(Rgb15 { r: 3, g: 4, b: 25 }));
    assert_eq!(frame.player().paint, None, "a new pilot is unpainted");
}
