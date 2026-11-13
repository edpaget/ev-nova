//! The stock cloaks as the player's own (rdm cloaking-fidelity phase 8,
//! see `nova_sim::cloak` and the session's `player_cloak`). Skips,
//! passing, when `NOVA_DATA` is unset.
//!
//! - The stock `ModType` 17 outfits and their `ModVal`s: 211 0xe, 234
//!   0x6, 266 0x2, 268 0x80b, 269 0x409 and 347 0x6. None has a fuel
//!   nibble; only the Polaris organs 268 and 269 have a shield one.
//! - By the engine's reading of `cloak_reserves` each can cloak on an
//!   empty shield and none on an empty tank; by the Bible's, 268 and 269
//!   can't on an empty shield.
//! - Three ship types carry `Flags2` 0x0400, which keeps the player's
//!   cloak through a jump.

use crate::common;

use std::collections::BTreeMap;

use nova_data::GameData;
use nova_sim::cloak::{CLOAK_JUMPING, CloakCheck, CloakGear, MOD_CLOAK};
use nova_sim::rulebook::RuleSource;
use nova_sim::{OutfitId, PilotCatalog};

#[test]
fn the_stock_cloaks_give_the_player_their_usages_and_reserves_by_each_reading() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let outfits = data.outfits();
    let cloaks: Vec<(i16, i16)> = outfits
        .iter()
        .filter_map(|record| {
            record
                .mods
                .iter()
                .find(|&&(kind, _)| kind == MOD_CLOAK)
                .map(|&(_, mod_val)| (record.id.0, mod_val))
        })
        .collect();
    assert_eq!(
        cloaks,
        [
            (211, 0xe),
            (234, 0x6),
            (266, 0x2),
            (268, 0x80b),
            (269, 0x409),
            (347, 0x6)
        ]
    );
    let ready = CloakCheck {
        disabled: false,
        fuel: 100.0,
        shield: 100.0,
        jumping: false,
    };
    let no_shield = CloakCheck {
        shield: 0.0,
        ..ready
    };
    let no_fuel = CloakCheck { fuel: 0.0, ..ready };
    // (outfit, shield usage, zeroes, decloaks, by the Bible on an empty
    // shield)
    for (id, shield_usage, zeroes, decloaks, bible_bare) in [
        (211, 0, true, true, true),
        (234, 0, true, false, true),
        (266, 0, false, false, true),
        (268, 8, false, true, false),
        (269, 4, false, true, false),
        (347, 0, true, false, true),
    ] {
        let owned: BTreeMap<OutfitId, u16> = [(OutfitId(id), 1)].into();
        let gear = CloakGear::player(0, &owned, &outfits);
        assert!(gear.device, "{id}");
        assert_eq!(
            (
                gear.shield_usage,
                gear.fuel_usage,
                gear.zeroes_shields,
                gear.decloaks_when_hit
            ),
            (shield_usage, 0, zeroes, decloaks),
            "{id}"
        );
        assert!(gear.can_cloak(no_shield, RuleSource::Engine), "{id}");
        assert!(!gear.can_cloak(no_fuel, RuleSource::Engine), "{id}");
        assert_eq!(
            gear.can_cloak(no_shield, RuleSource::Bible),
            bible_bare,
            "{id}"
        );
        assert!(
            gear.can_cloak(no_fuel, RuleSource::Bible),
            "{id}: no fuel usage"
        );
    }
    let jumping = data
        .ships()
        .into_iter()
        .filter(|ship| ship.fields.flags2 & CLOAK_JUMPING != 0)
        .count();
    assert_eq!(jumping, 3, "Flags2 0x0400");
}
