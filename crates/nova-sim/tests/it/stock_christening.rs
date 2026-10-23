//! A new stock pilot's ship christening (rdm play-feel phase 10): the
//! prompt `_DoNewPilot` builds from `STR#` 2002 #121 and the first
//! `chär`'s ship's `LongName`, and the three `STR#` 128 defaults its roll
//! picks among. Skips, passing, when `NOVA_DATA` is unset.

use crate::common;

use nova_sim::christening::christening;
use nova_sim::pilot::Pilot;
use nova_sim::{Chance, PilotCatalog};

/// Every roll is `.0`; nothing fires.
struct Rolls(u16);

impl Chance for Rolls {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    fn below(&mut self, n: u32) -> u32 {
        n - 1
    }

    fn roll(&mut self, _sides: u16) -> u16 {
        self.0
    }
}

#[test]
fn a_new_stock_pilot_is_asked_to_christen_its_ship_with_a_stock_default() {
    let Some(data) = common::opened() else {
        return;
    };
    let pilot = Pilot::new(&data, "Stock").expect("the stock first chär starts");
    let ships = data.ships();
    for (roll, name) in [
        (0, "Ring of Glory"),
        (1, "Snowy Owl"),
        (2, "Cardinal Virtue"),
    ] {
        let asked = christening(&data, &ships, pilot.ship(), &mut Rolls(roll));
        assert!(
            asked
                .prompt
                .starts_with("Now, please christen your brand-new "),
            "{asked:?}"
        );
        assert!(asked.prompt.ends_with(": "), "{asked:?}");
        assert!(asked.prompt.len() > "Now, please christen your brand-new : ".len());
        assert_eq!(asked.default, name, "roll {roll}");
    }
}
