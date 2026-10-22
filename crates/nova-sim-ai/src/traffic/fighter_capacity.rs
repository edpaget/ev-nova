//! What a fighter holds: the original's AI type 5 scales its shield and
//! armour capacities and its shield recharge rate by
//! [`FIGHTER_CAPACITY`](crate::bay::FIGHTER_CAPACITY) (`_ShipShieldCapacity`
//! @0x2a91-0x2aa7, `_ShipArmorCapacity` @0x2cb9-0x2cd2,
//! `_ShipShieldRechargeRate` @0x2b96-0x2bac), as the rulebook's
//! [`RuleKey::FighterCapacity`](crate::RuleKey::FighterCapacity) reads.
//!
//! The engine's stand-in for AI type 5 is a ship with a
//! [`carrier`](Npc::carrier): one a bay launched, the player's or an NPC
//! carrier's, or one that rejoined the player's fleet as a fighter. Its
//! [`stats`](Npc::stats) keep its class's values; the most its shield and
//! armour gauges hold ([`Npc::reserves`]) is what every reader of the
//! capacities reads (the disable threshold, the self-repair, damage, the
//! target panel, the odds, capture and plunder), so
//! [`Npc::hold_capacities`] sets it wherever a ship becomes a fighter or
//! stops being one. The original works the capacity out at each read
//! and keeps the shield and armour it holds (+0x54, +0x58) as they are, so
//! a change keeps what the gauges hold now: a fighter that leaves its
//! carrier may hold more than its class's until it is hit, as only the
//! regeneration caps at the capacity. Its armour recharge takes no factor.

use crate::bay::fighter_capacity;
use crate::rulebook::RuleSource;
use crate::traffic::npc::Npc;

impl Npc {
    /// Sets the most its shield and armour gauges hold: by `reading` of
    /// [`RuleKey::FighterCapacity`](crate::RuleKey::FighterCapacity) for a
    /// fighter (one with a carrier), else its class's (see the module
    /// docs). What the gauges hold now is kept.
    pub fn hold_capacities(&mut self, reading: RuleSource) {
        let reading = self.capacity_reading(reading);
        self.reserves.shield.max = fighter_capacity(self.stats.shield, reading);
        self.reserves.armor.max = fighter_capacity(self.stats.armor, reading);
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
    /// while it is a fighter (see the module docs).
    #[must_use]
    pub fn shield_recharge(&self, reading: RuleSource) -> f32 {
        fighter_capacity(self.stats.shield_regen, self.capacity_reading(reading))
    }

    /// `reading` for a fighter; the class's own (the other reading) for
    /// any other ship.
    fn capacity_reading(&self, reading: RuleSource) -> RuleSource {
        if self.carrier.is_some() {
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
    use crate::bay::Carrier;
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
    fn a_fighters_shield_recharges_1_333_times_as_fast_by_the_engine() {
        // 0.125 × 1.333 in doubles: 0.166_625.
        assert_eq!(fighter().shield_recharge(RuleSource::Engine), 0.166_625);
        assert_eq!(fighter().shield_recharge(RuleSource::Bible), 0.125);
        for reading in RuleSource::ALL {
            assert_eq!(ship().shield_recharge(reading), 0.125, "{reading:?}");
        }
    }
}
