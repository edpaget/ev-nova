//! What a fighter holds: the original's AI type 5 scales its shield and
//! armour capacities and its shield recharge rate by
//! [`FIGHTER_CAPACITY`](crate::bay::FIGHTER_CAPACITY) (`_ShipShieldCapacity`
//! @0x2a91-0x2aa7, `_ShipArmorCapacity` @0x2cb9-0x2cd2,
//! `_ShipShieldRechargeRate` @0x2b96-0x2bac), as the rulebook's
//! [`RuleKey::FighterCapacity`](crate::RuleKey::FighterCapacity) reads.
//!
//! Those readers test the word +0x88 alone, no carrier and no parent.
//! The engine's AI type 5 is [`Npc::original_ai_type`] ==
//! [`FIGHTER_AI`]: a ship with a [`carrier`](Npc::carrier), one a bay
//! launched, the player's or an NPC carrier's, or one that rejoined the
//! player's fleet as a fighter, unless a bribe's 1 stands over it
//! ([`Npc::take_bribe`], `_DoCommDialog` @0x961f8, the carrier kept). Its
//! [`stats`](Npc::stats) keep its class's values; the most its shield and
//! armour gauges hold ([`Npc::reserves`]) is what every reader of the
//! capacities reads (the disable threshold, the self-repair, damage, the
//! target panel, the odds, capture and plunder), while its shield
//! recharge ([`Npc::shield_recharge`]) reads the word each tick. So every
//! write of the word holds the capacities again
//! ([`Npc::hold_capacities`]): [`Npc::take_carrier`];
//! [`Npc::leave_carrier`], which `Session::unfleet` (a lost or released
//! escort taking its class's `InherentAI`, `_DamageShip` @0x3b0d7), the
//! releases, a pirate's take and the player's capture (6) call; and
//! [`Npc::take_bribe`]. The original works the capacity out at each read
//! and keeps the shield and armour it holds (+0x54, +0x58) as they are, so
//! a change keeps what the gauges hold now: a fighter that leaves its
//! carrier may hold more than its class's until it is hit, as only the
//! regeneration caps at the capacity. Its armour recharge takes no factor.

use crate::bay::{Carrier, fighter_capacity};
use crate::rulebook::RuleSource;
use crate::traffic::npc::Npc;
use crate::traffic::original_ai_type::FIGHTER_AI;

impl Npc {
    /// Sets the most its shield and armour gauges hold: by `reading` of
    /// [`RuleKey::FighterCapacity`](crate::RuleKey::FighterCapacity) for a
    /// fighter (one whose word is 5), else its class's (see the module
    /// docs). What the gauges hold now is kept.
    pub fn hold_capacities(&mut self, reading: RuleSource) {
        let reading = self.capacity_reading(reading);
        self.reserves.shield.max = fighter_capacity(self.stats.shield, reading);
        self.reserves.armor.max = fighter_capacity(self.stats.armor, reading);
    }

    /// It becomes a fighter of `carrier`, its gauges' most a fighter's by
    /// `reading` of [`RuleKey::FighterCapacity`](crate::RuleKey::FighterCapacity),
    /// what they hold now kept (see the module docs).
    pub fn take_carrier(&mut self, carrier: Carrier, reading: RuleSource) {
        self.carrier = Some(carrier);
        self.hold_capacities(reading);
    }

    /// It is no longer a fighter: it has no carrier, and its gauges' most
    /// is its class's again, what they hold now kept (see the module
    /// docs).
    pub fn leave_carrier(&mut self) {
        self.carrier = None;
        self.hold_capacities(RuleSource::Bible);
    }

    /// The shield points it recharges each tick: its class's, scaled by
    /// `reading` of [`RuleKey::FighterCapacity`](crate::RuleKey::FighterCapacity)
    /// while it is a fighter, its word 5 (see the module docs).
    #[must_use]
    pub fn shield_recharge(&self, reading: RuleSource) -> f32 {
        fighter_capacity(self.stats.shield_regen, self.capacity_reading(reading))
    }

    /// `reading` for a fighter (one whose word is 5); the class's own (the
    /// other reading) for any other ship.
    fn capacity_reading(&self, reading: RuleSource) -> RuleSource {
        if self.original_ai_type() == FIGHTER_AI {
            reading
        } else {
            RuleSource::Bible
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::combat::ShipRef;
    use crate::stats::ShipStats;
    use crate::traffic::fixture::npc;
    use nova_sim_base::fixture::FAST;

    /// A ship of `FAST` (shield 30, armour 45), its shield recharging
    /// 0.125 a tick and its armour 0.02, its gauges holding 10 and 20.
    fn ship() -> Npc {
        let mut stats = ShipStats::new(FAST, &[]);
        stats.shield_regen = 0.125;
        stats.armor_regen = 0.02;
        let mut ship = npc(1, stats);
        ship.reserves.shield.now = 10.0;
        ship.reserves.armor.now = 20.0;
        ship
    }

    /// [`ship`] as a fighter of the player.
    fn fighter() -> Npc {
        Npc {
            carrier: Some(Carrier {
                ship: ShipRef::Player,
                window: 100.0,
                reach: 40.0,
            }),
            ..ship()
        }
    }

    /// Its gauges' (now, most) for the shield and the armour.
    fn gauges(ship: &Npc) -> [(f32, f32); 2] {
        let reserves = ship.reserves;
        [
            (reserves.shield.now, reserves.shield.max),
            (reserves.armor.now, reserves.armor.max),
        ]
    }

    #[test]
    fn a_fighter_holds_1_333_times_its_classs_shield_and_armour_keeping_what_it_has() {
        let mut fighter = fighter();
        assert_eq!((fighter.stats.shield, fighter.stats.armor), (30.0, 45.0));
        fighter.hold_capacities(RuleSource::Engine);
        assert_eq!(gauges(&fighter), [(10.0, 39.99), (20.0, 59.985)]);
        assert_eq!(
            (fighter.stats.shield, fighter.stats.armor),
            (30.0, 45.0),
            "its stats keep its class's"
        );
        fighter.hold_capacities(RuleSource::Bible);
        assert_eq!(gauges(&fighter), [(10.0, 30.0), (20.0, 45.0)]);
    }

    #[test]
    fn a_ship_with_no_carrier_holds_its_classs_by_either_reading() {
        for reading in RuleSource::ALL {
            let mut ship = ship();
            ship.reserves.shield.max = 39.99;
            ship.reserves.armor.max = 59.985;
            ship.reserves.shield.now = 39.0;
            ship.hold_capacities(reading);
            assert_eq!(gauges(&ship), [(39.0, 30.0), (20.0, 45.0)], "{reading:?}");
        }
    }

    #[test]
    fn taking_a_carrier_a_ship_holds_a_fighters_by_the_reading_keeping_what_it_has() {
        let carrier = fighter().carrier.expect("carried");
        for (reading, held, recharge) in [
            (
                RuleSource::Engine,
                [(10.0, 39.99), (20.0, 59.985)],
                0.166_625,
            ),
            (RuleSource::Bible, [(10.0, 30.0), (20.0, 45.0)], 0.125),
        ] {
            let mut ship = ship();
            ship.take_carrier(carrier, reading);
            assert_eq!(ship.carrier, Some(carrier), "{reading:?}");
            assert_eq!(gauges(&ship), held, "{reading:?}");
            assert_eq!(ship.shield_recharge(reading), recharge, "{reading:?}");
        }
    }

    #[test]
    fn leaving_its_carrier_a_fighter_holds_its_classs_again_keeping_what_it_has() {
        let mut fighter = fighter();
        fighter.hold_capacities(RuleSource::Engine);
        fighter.reserves.shield.now = 39.0;
        fighter.leave_carrier();
        assert_eq!(fighter.carrier, None);
        assert_eq!(gauges(&fighter), [(39.0, 30.0), (20.0, 45.0)]);
        assert_eq!(fighter.shield_recharge(RuleSource::Engine), 0.125);
    }

    #[test]
    fn a_bribed_fighter_holds_its_classs_capacities_and_recharge() {
        // The bribe writes 1 over +0x88 keeping the carrier (`_DoCommDialog`
        // @0x961f8), and the readers test the word alone (@0x2a91,
        // @0x2b96, @0x2cbc).
        for reading in RuleSource::ALL {
            let mut fighter = Npc {
                carrier: Some(Carrier {
                    ship: ShipRef::Npc(crate::traffic::npc::NpcId(7)),
                    window: 100.0,
                    reach: 40.0,
                }),
                ..ship()
            };
            fighter.hold_capacities(reading);
            fighter.reserves.shield.now = 35.0;
            fighter.take_bribe();
            assert!(fighter.carrier.is_some(), "{reading:?}: kept");
            assert_eq!(
                gauges(&fighter),
                [(35.0, 30.0), (20.0, 45.0)],
                "{reading:?}"
            );
            assert_eq!(fighter.shield_recharge(reading), 0.125, "{reading:?}");
        }
    }

    #[test]
    fn a_fighters_shield_recharges_1_333_times_as_fast_by_the_engine() {
        // 0.125 × 1.333 in doubles: 0.166_625.
        assert_eq!(fighter().shield_recharge(RuleSource::Engine), 0.166_625);
        assert_eq!(fighter().shield_recharge(RuleSource::Bible), 0.125);
        for reading in RuleSource::ALL {
            assert_eq!(ship().shield_recharge(reading), 0.125, "{reading:?}");
        }
    }
}
