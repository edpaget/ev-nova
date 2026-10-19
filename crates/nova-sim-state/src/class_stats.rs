//! An NPC's stats under the
//! [`RuleKey::NpcDefaultItems`](crate::RuleKey::NpcDefaultItems) reading
//! ([`ShipStats::for_npc`]).
//!
//! The original's `_Ship*` functions count a ship's outfits for the
//! player alone. An NPC's top speed and thrust start from its class's
//! (`_ShipMaxSpeed` @0x37c5, class +0x40, the bare `Speed` / 100;
//! `_ShipAccelRate` @0x361e, class +0x3c, `Accel` / 10000, doubled), and
//! its turn rate is its class's (`_ShipTurnRate` @0x33f4-0x341c, class
//! +0x38, `Maneuver` × 0.1), floored as an NPC's
//! ([`floored_turn`](crate::handling::floored_turn)). So by the engine an
//! NPC's default items change none of them: 23 stock classes carry a
//! reactor (`ModType` 8 and 7) that the original's NPC never flies with.
//! By the other reading, the port's earlier one, they count as any
//! outfit's do ([`ShipStats::new`]).
//!
//! The reserves follow the same rule. `_ShipShieldCapacity`
//! (@0x2995-0x2aba, class +0x5c), `_ShipArmorCapacity` (@0x2bc0-0x2ce5,
//! class +0x60), `_ShipShieldRechargeRate` (@0x2abb-0x2bbf, class +0x50)
//! and `_ShipArmorRechargeRate` (@0x2e0e-0x2f20, class +0x54) add the
//! outfits' `ModType` 4, 6, 5 and 29 for the player alone, and
//! `_ShipFuelCapacity` returns an NPC's class `Fuel` at once
//! (@0x2f47-0x2f4f), where the player adds `ModType` 12. By the engine
//! an NPC's are its class's (stock: Carbon Fiber on 4 classes, a Shield
//! Recharger on 1, a Battery Pack on 3). So are its ion capacity and
//! deionization rate (`_ShipIonizationFactor` @0x3260,
//! `_ShipDeionizationRate` @0x6ee9, [`IonStats`]); by the other reading
//! its absorbers and dissipaters count as the player's do.
//!
//! Its fuel regeneration counts its fuel scoops by either reading:
//! `_ShipFuelGenRate`'s NPC branch (@0x3130-0x31e3) sums its class's
//! default items' `ModType` 18. Every other figure is [`ShipStats::new`]'s
//! under both readings.

use crate::fuel::OutfitMod;
use crate::handling::ShipFields;
use crate::rulebook::RuleSource;
use crate::stats::{IonStats, ShipStats};

impl ShipStats {
    /// The stats of an NPC of class `fields`, carrying its default items,
    /// `outfits`, as `items`, the
    /// [`RuleKey::NpcDefaultItems`](crate::RuleKey::NpcDefaultItems)
    /// reading, says (see the [module docs](self)).
    #[must_use]
    pub fn for_npc(fields: ShipFields, outfits: &[OutfitMod], items: RuleSource) -> Self {
        let counted = Self::new(fields, outfits);
        match items {
            RuleSource::Bible => Self {
                ion: IonStats {
                    npc_counts: true,
                    ..counted.ion
                },
                ..counted
            },
            RuleSource::Engine => {
                let class = Self::new(fields, &[]);
                Self {
                    handling: class.handling,
                    shield: class.shield,
                    armor: class.armor,
                    fuel: class.fuel,
                    shield_regen: class.shield_regen,
                    armor_regen: class.armor_regen,
                    ..counted
                }
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::flight::Flyer;
    use crate::fuel::FUEL_SCOOP;
    use crate::handling::Handling;
    use crate::stats::{
        ION_ABSORBER, ION_DISSIPATER, MORE_ACCEL, MORE_ARMOR, MORE_ARMOR_RECHARGE, MORE_FUEL,
        MORE_SHIELD, MORE_SHIELD_RECHARGE, MORE_SPEED, TURN_CHANGE,
    };
    use nova_sim_base::fixture::FAST;

    fn outfit(mod_type: i16, mod_val: i16, count: u16) -> OutfitMod {
        OutfitMod {
            mod_type,
            mod_val,
            count,
        }
    }

    /// A reactor (+200 `Speed`, +300 `Accel`) and a steering outfit
    /// (+100 to the turn, 1° a tick), each carried once.
    fn reactor_and_steering() -> Vec<OutfitMod> {
        vec![
            outfit(MORE_SPEED, 200, 1),
            outfit(MORE_ACCEL, 300, 1),
            outfit(TURN_CHANGE, 100, 1),
        ]
    }

    #[test]
    fn by_the_engine_an_npc_flies_and_turns_at_its_classs_alone() {
        let stats = ShipStats::for_npc(FAST, &reactor_and_steering(), RuleSource::Engine);
        assert_eq!(
            stats.handling,
            Handling::from_totals(600, 900, 30.0),
            "FAST's own: 6, 0.18 and 3"
        );
        assert_eq!(stats.handling, ShipStats::new(FAST, &[]).handling);
    }

    #[test]
    fn by_the_other_reading_its_default_items_count() {
        let outfits = reactor_and_steering();
        let stats = ShipStats::for_npc(FAST, &outfits, RuleSource::Bible);
        assert_eq!(stats.handling, Handling::from_totals(800, 1200, 40.0));
        let counted = ShipStats::new(FAST, &outfits);
        assert_eq!(
            ShipStats {
                ion: counted.ion,
                ..stats
            },
            counted,
            "every figure counted, the ion stats counting for an NPC too"
        );
        assert!(stats.ion.npc_counts);
    }

    /// A ship of shield 30, armour 45, fuel 300, regenerating 200
    /// shield and 100 armour ×1000 a tick, `IonizeMax` 400 and
    /// `Deionize` 50, fuel every 8 ticks.
    const RESERVED: ShipFields = ShipFields {
        shield_rech: 200,
        armor_rech: 100,
        ionize_max: 400,
        deionize: 50,
        fuel_regen: 8,
        ..FAST
    };

    /// One of each outfit that adds to a reserve, or to ionization, and a
    /// fuel scoop regenerating a unit every 4 ticks.
    fn reserve_items() -> Vec<OutfitMod> {
        vec![
            outfit(MORE_SHIELD, 50, 1),
            outfit(MORE_ARMOR, 25, 2),
            outfit(MORE_SHIELD_RECHARGE, 300, 1),
            outfit(MORE_ARMOR_RECHARGE, 400, 1),
            outfit(MORE_FUEL, 100, 1),
            outfit(ION_ABSORBER, 200, 1),
            outfit(ION_DISSIPATER, 150, 1),
            outfit(FUEL_SCOOP, 4, 1),
        ]
    }

    #[test]
    fn by_the_engine_an_npcs_reserves_and_ionization_are_its_classs_alone() {
        let stats = ShipStats::for_npc(RESERVED, &reserve_items(), RuleSource::Engine);
        assert_eq!((stats.shield, stats.armor, stats.fuel), (30.0, 45.0, 300.0));
        assert_eq!((stats.shield_regen, stats.armor_regen), (0.2, 0.1));
        assert_eq!(stats.ion.capacity(Flyer::Npc), 400.0);
        assert_eq!(stats.ion.rate(Flyer::Npc), 0.5);
        assert_eq!(stats.full(), ShipStats::new(RESERVED, &[]).full());
    }

    #[test]
    fn by_the_other_reading_its_default_items_add_to_its_reserves_and_ionization() {
        let stats = ShipStats::for_npc(RESERVED, &reserve_items(), RuleSource::Bible);
        assert_eq!((stats.shield, stats.armor, stats.fuel), (80.0, 95.0, 400.0));
        assert_eq!((stats.shield_regen, stats.armor_regen), (0.5, 0.5));
        assert_eq!(stats.ion.capacity(Flyer::Npc), 600.0);
        assert_eq!(stats.ion.rate(Flyer::Npc), (0.5_f64 + 150.0 * 0.01) as f32);
    }

    #[test]
    fn its_fuel_scoops_count_by_either_reading() {
        for items in [RuleSource::Engine, RuleSource::Bible] {
            let stats = ShipStats::for_npc(RESERVED, &reserve_items(), items);
            assert_eq!(
                stats.fuel_regen,
                ShipStats::new(RESERVED, &reserve_items()).fuel_regen,
                "{items:?}"
            );
            assert!(
                stats.fuel_regen > ShipStats::new(RESERVED, &[]).fuel_regen,
                "the scoop counts: {items:?}"
            );
        }
    }

    #[test]
    fn the_players_ion_stats_count_its_outfits_by_either_reading() {
        for items in [RuleSource::Engine, RuleSource::Bible] {
            let stats = ShipStats::for_npc(RESERVED, &reserve_items(), items);
            assert_eq!(stats.ion.capacity(Flyer::Player), 600.0, "{items:?}");
        }
    }

    #[test]
    fn by_the_engine_the_class_turn_is_floored_as_an_npcs() {
        let slow = ShipFields {
            maneuver: 12,
            ..FAST
        };
        let stats = ShipStats::for_npc(slow, &[outfit(TURN_CHANGE, -200, 1)], RuleSource::Engine);
        assert_eq!(stats.handling.turn_rate, 1.2);
        let turning = ShipStats::for_npc(slow, &[outfit(TURN_CHANGE, -200, 1)], RuleSource::Bible);
        assert_eq!(turning.handling.turn_rate, 1.0, "1.0 by the floor");
    }
}
