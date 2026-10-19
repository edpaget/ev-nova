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
//! Every other figure is [`ShipStats::new`]'s under both readings.

use crate::fuel::OutfitMod;
use crate::handling::ShipFields;
use crate::rulebook::RuleSource;
use crate::stats::ShipStats;

impl ShipStats {
    /// The stats of an NPC of class `fields`, carrying its default items,
    /// `outfits`, as `items`, the
    /// [`RuleKey::NpcDefaultItems`](crate::RuleKey::NpcDefaultItems)
    /// reading, says (see the [module docs](self)).
    #[must_use]
    pub fn for_npc(fields: ShipFields, outfits: &[OutfitMod], items: RuleSource) -> Self {
        let counted = Self::new(fields, outfits);
        match items {
            RuleSource::Bible => counted,
            RuleSource::Engine => {
                let class = Self::new(fields, &[]);
                Self {
                    handling: class.handling,
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
    use crate::handling::Handling;
    use crate::stats::{MORE_ACCEL, MORE_SPEED, TURN_CHANGE};
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
        assert_eq!(stats, ShipStats::new(FAST, &outfits));
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
