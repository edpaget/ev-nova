//! The stock `wëap` that `_HandlePlayer`'s secondary-weapon block reads
//! (rdm play-feel phase 3, see `nova_sim::session`'s `secondary_select`).
//! Skips, passing, when `NOVA_DATA` is unset.
//!
//! - No stock `wëap` sets `Flags` 0x0800, the bit the original's let-go
//!   tests (@0x6abe9), nor `Flags2` 0x0800, the hide-when-empty bit the
//!   other reading of `empty_secondary` tests: under either reading no
//!   stock secondary is let go when it runs out, and the select key skips
//!   none for having no ammunition.
//! - Every one of the 48 stock secondaries has a guidance the select key
//!   takes (raw 8 or less, or 99), so the guidance skip leaves out none.

use crate::common;

use nova_data::GameData;
use nova_sim::CombatCatalog;
use nova_sim::combat::armament::selectable_guidance;
use nova_sim::combat::weapon::{HIDE_WHEN_EMPTY, PERSISTENT_SMOKE, WeaponSpec};

#[test]
fn no_stock_weapon_is_let_go_or_skipped_when_it_runs_out() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let specs: Vec<WeaponSpec> = data.weapons().iter().map(WeaponSpec::new).collect();
    let none: Vec<i16> = Vec::new();
    let flagged = |of: fn(&WeaponSpec) -> bool| -> Vec<i16> {
        specs
            .iter()
            .filter(|spec| of(spec))
            .map(|spec| spec.id.0)
            .collect()
    };
    assert_eq!(flagged(|spec| spec.flags & PERSISTENT_SMOKE != 0), none);
    assert_eq!(flagged(|spec| spec.flags2 & HIDE_WHEN_EMPTY != 0), none);
    let secondaries = flagged(WeaponSpec::secondary);
    assert_eq!(secondaries.len(), 48);
    assert_eq!(
        flagged(|spec| spec.secondary() && !selectable_guidance(spec.guidance)),
        none
    );
}
