//! The collision passes as `_DoPlayGameWork` walks them (rdm
//! weapons-fidelity phase 33): each hit lands as the walk finds it
//! (`_HandleShipHit` @0x36a45-0x36f86), so a turret shot's miss rule
//! (`_ShotCanHitShip` @0x448ad-0x448d5) reads the armour the pass's
//! earlier hits left.

use super::*;

#[test]
fn a_fighter_is_disabled_now_as_is_disabled_reads_it() {
    let rule = NovaDisable;
    let mut under = Ship::at(1, 0.0, 0.0);
    under.reserves.armor.now = 9.0;
    assert!(
        under.fighter().disabled_now(&rule),
        "intact, under the line"
    );
    let mut held = Ship {
        held_disabled: true,
        ..Ship::at(2, 0.0, 0.0)
    };
    assert!(held.fighter().disabled_now(&rule), "held disabled");
    let mut down = Ship {
        condition: Condition::Disabled,
        ..Ship::at(3, 0.0, 0.0)
    };
    assert!(down.fighter().disabled_now(&rule), "not intact");
    let mut above = Ship::at(4, 0.0, 0.0);
    above.reserves.armor.now = 10.0;
    assert!(!above.fighter().disabled_now(&rule), "intact, at the line");
}
