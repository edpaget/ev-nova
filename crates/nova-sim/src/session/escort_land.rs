//! Leaving a shipyard with escorts marked in the escort dialog
//! (`_DoEscortLand` @0x4031f-0x40866, which `_DoPortDialog` calls on every
//! leave from the spaceport, @0x601a0): the session's side of
//! [`escort_dialog::settle_marks`].
//!
//! Only a stellar with a shipyard settles the marks (spöb flags +0x30 &
//! 0x08, @0x40328-0x40347); anywhere else they stay. The sales come
//! first, then the upgrades in slot order (the berths the fleet takes,
//! standing in for slots 1 to 63) while the cash covers each; a mark the
//! cash can't cover stays for the next shipyard, and nothing says so.
//!
//! - **A sale** (@0x4035f-0x40427) credits the class's `EscSellValue`
//!   and takes the escort out of the fleet and the system at once. It
//!   takes no share of the cargo and makes no reset: the take-off's own
//!   reset trims the cargo to the holds left, as the original's does.
//! - **An upgrade** (@0x4042d-0x40526) debits the class's
//!   `EscUpgrdCost` and makes the escort its `UpgradeTo` class, its mark
//!   cleared. Its NPC in the system is replaced by one of the new class
//!   on the same berth, keeping its skill, afterburner, aux mark, voice
//!   and cloak, as the original keeps the slot and `_RespawnEscort`
//!   (@0x3d4b8-0x3d62c) writes none of +0x40 (skill), +0xbd
//!   (afterburner), the voice (+0xc922) or the cloak (+0x64, +0xc8d8) as
//!   it respawns the slot from its new type (+0x76) at the take-off. The
//!   take-off restocks it full to its new class, as `_RespawnEscort`'s
//!   restock 1 does (@0x674c2: shield and armour from
//!   `_ShipShieldCapacity`/`_ShipArmorCapacity`, each weapon's count from
//!   class +0x5e8 and ammunition from +0x7e8); the fuel is as recorded.
//!
//! Each escort sold or upgraded adds one to the stay's
//! `escorts_changed`, so leaving passes half as many days
//! ([`Stay::escort_days`](crate::story::day::Stay::escort_days),
//! @0x407ed-0x40851), and a save is due. The report is a text dialog
//! ([`escort_dialog::escort_land_text`]), which the router shows over the
//! spaceport before the leave's days and pay.
//!
//! Left out, as the engine shows the same: the refit of every slot
//! (@0x4052e-0x405a9) and an upgrade's zeroing of the weapon entries
//! (@0x404f6-0x40516), since the take-off places each escort from its
//! record's class and restocks it full; and the sale's `_IsDisabled`
//! skip (@0x403a4), since a disabled escort has already left the fleet
//! (`_DamageShip` @0x3afdd), so none marked can be disabled here.

use super::Session;
use crate::chance::Chance;
use crate::escort_dialog;
use crate::landing::StellarFlags;
use crate::rulebook::RuleKey;

impl Session {
    /// Settles the escorts' marks as the ship leaves the stellar it is
    /// landed at (see the module docs), drawing an upgraded escort's new
    /// NPC's rolls on `chance`, and gives the report to show; none, and
    /// nothing changed, away from a shipyard or when nothing was sold or
    /// upgraded. A second call settles only what a first left: marks the
    /// cash didn't cover, which it still doesn't.
    pub fn land_escorts(&mut self, chance: &mut dyn Chance) -> Option<String> {
        let stellar = self.mode.landed()?;
        let shipyard = self
            .chart
            .sites
            .iter()
            .find(|site| site.id == stellar)
            .is_some_and(|site| site.flags & StellarFlags::SHIPYARD != 0);
        if !shipyard {
            return None;
        }
        let berths = self.space.fleet_berths(&self.pilot);
        let mut order: Vec<usize> = (0..self.pilot.escorts.len()).collect();
        order.sort_by_key(|&index| berths.get(index).copied());
        let statics = &self.statics;
        let settled =
            escort_dialog::settle_marks(&self.pilot.escorts, &order, self.pilot.cash, |ship| {
                statics.ship_record(ship)
            });
        if settled.sold.is_empty() && settled.upgraded.is_empty() {
            return None;
        }
        self.pilot.cash = settled.cash;
        for &(index, ship) in &settled.upgraded {
            self.upgrade_escort(index, ship, chance);
        }
        for &index in settled.sold.iter().rev() {
            self.drop_fleet_escort(index);
        }
        let changed = settled.landing.sold + settled.landing.upgraded;
        self.port.stay.escorts_changed = self.port.stay.escorts_changed.saturating_add(changed);
        self.reform();
        self.out.save_due = true;
        escort_dialog::escort_land_text(
            &settled.landing,
            self.setup.rules.source_for(RuleKey::PayMillions),
        )
    }

    /// Escort `index` becomes class `ship`, its mark cleared, and its NPC
    /// in the system, if any, is replaced by one of `ship` on the same
    /// berth, keeping what the slot keeps (see the module docs). With no
    /// record of `ship`, its NPC is gone and none takes its place, as for
    /// any escort whose class has no record.
    fn upgrade_escort(
        &mut self,
        index: usize,
        ship: crate::catalog::ShipId,
        chance: &mut dyn Chance,
    ) {
        let Some(escort) = self.pilot.escorts.get_mut(index) else {
            return;
        };
        escort.ship = ship;
        escort.mark = None;
        let Some(old) = self
            .space
            .escorts
            .fleet
            .get(index)
            .copied()
            .flatten()
            .and_then(|id| self.space.npc(id))
            .cloned()
        else {
            return;
        };
        self.space.traffic.remove(old.id);
        let placed = self.place_escort_at(index, Some(old.berth), Some(old.skill), chance);
        if let Some(npc) = placed.and_then(|id| self.space.npc_mut(id)) {
            npc.afterburner = old.afterburner;
            npc.aux = old.aux;
            npc.voice = old.voice;
            npc.cloak = old.cloak;
            npc.state = old.state;
        }
        self.space.escorts.fleet[index] = placed;
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::{LandingSite, ShipId, ShipRecord, SystemId};
    use crate::chance::NeverFires;
    use crate::fleet::EscortMark;
    use crate::handling::ShipFields;
    use crate::market::Good;
    use crate::rulebook::{RuleSource, Rulebook};
    use crate::testkit::{FAST, FakePilotCatalog, catalog, land_now, planet, ship};
    use crate::traffic::npc::NpcId;

    /// The shipyard's stellar, 128, at the centre, under the ship as it
    /// starts.
    const YARD: crate::catalog::StellarId = crate::catalog::StellarId(128);

    fn yard() -> LandingSite {
        LandingSite {
            flags: StellarFlags::CAN_LAND | StellarFlags::BAR | StellarFlags::SHIPYARD,
            tech_level: 7,
            ..planet(128, 0.0, 0.0)
        }
    }

    /// Class `id`, for hire every day, upgrading to `to` for `cost` and
    /// selling for `value`.
    fn class(id: i16, to: Option<i16>, cost: i64, value: i64) -> ShipRecord {
        ShipRecord {
            hire_random: 100,
            upgrade_to: to.map(ShipId),
            esc_upgrd_cost: cost,
            esc_sell_value: value,
            ..ship(id, FAST)
        }
    }

    /// Planet 128 a shipyard with a bar; class 129 upgrades to 130 for
    /// 6,000, class 131 sells for 499; class 130 is bigger and costs
    /// 30,000.
    fn yards() -> FakePilotCatalog {
        FakePilotCatalog {
            sites: vec![
                (SystemId(130), vec![yard(), planet(129, 2000.0, 0.0)]),
                (SystemId(131), vec![planet(140, 0.0, 0.0)]),
            ],
            ship_records: vec![
                ship(128, FAST),
                class(129, Some(130), 6_000, 700),
                ShipRecord {
                    cost: 30_000,
                    fields: ShipFields { holds: 40, ..FAST },
                    ..class(130, None, 0, 3_000)
                },
                ShipRecord {
                    fields: ShipFields { holds: 20, ..FAST },
                    ..class(131, None, 0, 499)
                },
            ],
            ..catalog()
        }
    }

    /// A session of `catalog` landed at the shipyard with escorts of
    /// `ships` hired there and placed, each captured (not hired) but those
    /// `hired`, holding `cash` credits.
    fn landed_with(
        catalog: &FakePilotCatalog,
        ships: &[i16],
        hired: &[usize],
        cash: i64,
    ) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.pilot.cash = 10_000_000;
        assert_eq!(land_now(&mut session), Ok(YARD));
        for &ship in ships {
            session.hire(ShipId(ship), &mut NeverFires).expect("hires");
        }
        for (index, escort) in session.pilot.escorts.iter_mut().enumerate() {
            if !hired.contains(&index) {
                escort.wage = None;
            }
        }
        session.pilot.cash = cash;
        session.take_save_due();
        session
    }

    fn mark(session: &mut Session, index: usize, mark: EscortMark) {
        session.pilot.escorts[index].mark = Some(mark);
    }

    fn ids(session: &Session) -> Vec<Option<NpcId>> {
        session.space.escorts.fleet.clone()
    }

    #[test]
    fn leaving_a_shipyard_sells_and_upgrades_the_marked_escorts_and_reports_it() {
        let catalog = yards();
        let mut session = landed_with(&catalog, &[131, 129], &[], 10_000);
        mark(&mut session, 0, EscortMark::Sell);
        mark(&mut session, 1, EscortMark::Upgrade);
        let text = session.land_escorts(&mut NeverFires);
        assert_eq!(
            text.as_deref(),
            Some(
                "One escort was sold for a profit of 499 credits.\n\nOne escort was upgraded \
                 at a cost of 6,000 credits."
            )
        );
        assert_eq!(session.pilot.cash, 10_000 + 499 - 6_000);
        assert_eq!(session.pilot.escorts.len(), 1);
        assert_eq!(session.pilot.escorts[0].ship, ShipId(130));
        assert_eq!(session.pilot.escorts[0].mark, None);
        assert_eq!(session.port.stay.escorts_changed, 2);
        assert_eq!(session.port.stay.escort_days(), 1);
        assert!(session.take_save_due());
        assert_eq!(session.land_escorts(&mut NeverFires), None, "nothing left");
        assert_eq!(session.port.stay.escorts_changed, 2);
    }

    #[test]
    fn the_escorts_days_pass_before_the_take_offs() {
        let catalog = yards();
        let mut changed = landed_with(&catalog, &[131, 131], &[], 0);
        mark(&mut changed, 0, EscortMark::Sell);
        mark(&mut changed, 1, EscortMark::Sell);
        let mut unchanged = landed_with(&catalog, &[131, 131], &[], 0);
        assert_eq!(unchanged.pilot.date, changed.pilot.date);
        changed
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        unchanged
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        assert_eq!(
            changed.pilot.date,
            unchanged.pilot.date.next_day(),
            "two sold pass a day, settled by the take-off itself"
        );
        assert!(changed.pilot.escorts.is_empty());
    }

    #[test]
    fn a_stellar_without_a_shipyard_settles_nothing() {
        let mut catalog = yards();
        catalog.sites[0].1[0].flags = StellarFlags::CAN_LAND | StellarFlags::BAR;
        let mut session = landed_with(&catalog, &[131, 129], &[], 10_000);
        mark(&mut session, 0, EscortMark::Sell);
        mark(&mut session, 1, EscortMark::Upgrade);
        assert_eq!(session.land_escorts(&mut NeverFires), None);
        assert_eq!(session.pilot.escorts.len(), 2);
        assert_eq!(session.pilot.escorts[0].mark, Some(EscortMark::Sell));
        assert_eq!(session.pilot.escorts[1].mark, Some(EscortMark::Upgrade));
        assert_eq!(session.pilot.cash, 10_000);
        assert!(!session.take_save_due());
    }

    #[test]
    fn a_short_purse_keeps_the_upgrade_mark() {
        let catalog = yards();
        let mut session = landed_with(&catalog, &[129], &[], 5_999);
        mark(&mut session, 0, EscortMark::Upgrade);
        assert_eq!(session.land_escorts(&mut NeverFires), None);
        assert_eq!(session.pilot.escorts[0].ship, ShipId(129));
        assert_eq!(session.pilot.escorts[0].mark, Some(EscortMark::Upgrade));
        assert_eq!(session.pilot.cash, 5_999);
    }

    #[test]
    fn a_sold_trader_takes_no_share_of_the_cargo_at_the_sale() {
        let catalog = yards();
        let mut session = landed_with(&catalog, &[131], &[], 0);
        session.pilot.cargo.insert(Good::Commodity(0), 20);
        mark(&mut session, 0, EscortMark::Sell);
        session.land_escorts(&mut NeverFires).expect("sold");
        assert_eq!(session.pilot.cargo.get(&Good::Commodity(0)), Some(&20));
    }

    #[test]
    fn selling_one_keeps_the_fleet_and_its_ships_in_step() {
        let catalog = yards();
        let mut session = landed_with(&catalog, &[131, 131, 131], &[], 0);
        let [a, b, c] = ids(&session)[..] else {
            panic!("three placed");
        };
        let (a, b, c) = (a.expect("A"), b.expect("B"), c.expect("C"));
        let c_berth = session.space.npc(c).expect("C").berth;
        let ships = |session: &Session, id| {
            session
                .space
                .npc(id)
                .and_then(|npc| npc.escort)
                .map(|duty| duty.formation.ships)
        };
        assert_eq!(ships(&session, c), Some(4));
        mark(&mut session, 1, EscortMark::Sell);
        session.land_escorts(&mut NeverFires).expect("sold");
        assert_eq!(session.pilot.escorts.len(), 2);
        assert_eq!(ids(&session), [Some(a), Some(c)]);
        assert_eq!(session.space.npc(c).expect("C").berth, c_berth);
        assert_eq!(session.space.fleet_berths(&session.pilot)[1], c_berth);
        assert!(session.space.npc(b).is_none(), "B's NPC is gone");
        assert_eq!(ships(&session, c), Some(3), "re-formed");
        assert!(session.take_save_due());
    }

    #[test]
    fn selling_the_first_or_the_last_keeps_them_in_step() {
        let catalog = yards();
        for sold in [0, 2] {
            let mut session = landed_with(&catalog, &[131, 131, 131], &[], 0);
            let before = ids(&session);
            mark(&mut session, sold, EscortMark::Sell);
            session.land_escorts(&mut NeverFires).expect("sold");
            let mut kept = before.clone();
            kept.remove(sold);
            assert_eq!(ids(&session), kept, "{sold}");
            assert_eq!(session.pilot.escorts.len(), 2, "{sold}");
        }
    }

    #[test]
    fn an_escort_sold_with_no_npc_placed_leaves_the_fleet_all_the_same() {
        let catalog = yards();
        let mut session = landed_with(&catalog, &[131], &[], 0);
        session.space.escorts.fleet.clear();
        mark(&mut session, 0, EscortMark::Sell);
        session.land_escorts(&mut NeverFires).expect("sold");
        assert!(session.pilot.escorts.is_empty());
        assert!(session.space.escorts.fleet.is_empty());
    }

    #[test]
    fn an_upgraded_escort_flies_its_new_class_after_the_take_off() {
        let catalog = yards();
        let mut session = landed_with(&catalog, &[129], &[], 6_000);
        let old = ids(&session)[0].expect("placed");
        let (berth, skill) = {
            let npc = session.space.npc(old).expect("in the system");
            (npc.berth, npc.skill)
        };
        mark(&mut session, 0, EscortMark::Upgrade);
        session.land_escorts(&mut NeverFires).expect("upgraded");
        assert!(
            session.space.npc(old).is_none(),
            "the old class's NPC is gone"
        );
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        session.tick_traffic(&catalog, &crate::ai::Peaceful, &mut NeverFires);
        let id = ids(&session)[0].expect("placed");
        let npc = session.space.npc(id).expect("in the system");
        let record = session.statics.ship_record(ShipId(130)).expect("a record");
        let kind = session.setup.ship_kind(&session.statics, record);
        assert_eq!(npc.ship, ShipId(130));
        assert_eq!(npc.stats, kind.stats);
        assert_eq!(npc.hull, kind.hull);
        assert_eq!(
            npc.reserves.armor.now, npc.reserves.armor.max,
            "restocked full"
        );
        assert_eq!((npc.berth, npc.skill), (berth, skill));
        assert!(
            session
                .space
                .traffic
                .npcs()
                .iter()
                .all(|npc| npc.ship != ShipId(129)),
            "no NPC of the old class is left"
        );
    }

    #[test]
    fn an_upgraded_escort_keeps_its_afterburner_aux_voice_cloak_and_state() {
        use crate::cloak::Cloak;
        use crate::geometry::Vec2;
        use crate::voice::VoiceVariant;
        let catalog = yards();
        let mut session = landed_with(&catalog, &[129], &[], 6_000);
        let old = ids(&session)[0].expect("placed");
        let state = crate::ShipState {
            position: Vec2::new(123.0, -45.0),
            velocity: Vec2::new(1.5, -0.5),
            heading: 77.0,
        };
        let cloak = Cloak {
            fade: 3.0,
            direction: 1,
        };
        {
            let npc = session.space.npc_mut(old).expect("in the system");
            npc.afterburner = true;
            npc.aux = Some(4);
            npc.voice = Some(VoiceVariant::Odd);
            npc.cloak = cloak;
            npc.state = state;
        }
        mark(&mut session, 0, EscortMark::Upgrade);
        session.land_escorts(&mut NeverFires).expect("upgraded");
        let id = ids(&session)[0].expect("placed");
        assert_ne!(id, old, "a new NPC");
        let npc = session.space.npc(id).expect("in the system");
        assert_eq!(npc.ship, ShipId(130));
        assert!(npc.afterburner, "afterburner");
        assert_eq!(npc.aux, Some(4), "aux");
        assert_eq!(npc.voice, Some(VoiceVariant::Odd), "voice");
        assert_eq!(npc.cloak, cloak, "cloak");
        assert_eq!(npc.state, state, "state");
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        // The take-off places its escorts afresh around the ship, so the
        // state is the take-off's own, and the cloak has faded on; the
        // rest carries over.
        let id = ids(&session)[0].expect("placed");
        let npc = session.space.npc(id).expect("in the system");
        assert_eq!(npc.ship, ShipId(130));
        assert!(npc.afterburner, "afterburner after the take-off");
        assert_eq!(npc.aux, Some(4), "aux after the take-off");
        assert_eq!(
            npc.voice,
            Some(VoiceVariant::Odd),
            "voice after the take-off"
        );
        assert_eq!(
            npc.cloak.direction, cloak.direction,
            "still fading in after the take-off"
        );
        assert!(npc.cloak.fade >= cloak.fade, "cloak after the take-off");
    }

    #[test]
    fn upgrades_are_paid_in_berth_order_not_fleet_order() {
        let catalog = yards();
        let mut session = landed_with(&catalog, &[131, 129, 131], &[], 10_000_000);
        // The first leaves the fleet, and a new hire takes its berth at the
        // fleet's end.
        let first = ids(&session)[0].expect("placed");
        let first_berth = session.space.npc(first).expect("placed").berth;
        session.pilot.escorts.remove(0);
        session.space.escorts.fleet.remove(0);
        session.space.traffic.remove(first);
        session.hire(ShipId(129), &mut NeverFires).expect("hires");
        session.pilot.escorts[2].wage = None;
        let berths = session.space.fleet_berths(&session.pilot);
        assert_eq!(berths[2], first_berth, "the lowest berth, at the end");
        assert!(berths[0] > berths[2], "{berths:?}");
        mark(&mut session, 0, EscortMark::Upgrade);
        mark(&mut session, 2, EscortMark::Upgrade);
        session.pilot.cash = 6_000;
        session.land_escorts(&mut NeverFires).expect("upgraded one");
        assert_eq!(
            session.pilot.escorts[2].ship,
            ShipId(130),
            "the lower berth"
        );
        assert_eq!(session.pilot.escorts[2].mark, None);
        assert_eq!(session.pilot.escorts[0].ship, ShipId(129));
        assert_eq!(session.pilot.escorts[0].mark, Some(EscortMark::Upgrade));
        assert_eq!(session.pilot.cash, 0);
    }

    #[test]
    fn an_upgrade_above_a_sale_keeps_its_new_npc_as_the_fleet_shifts() {
        let catalog = yards();
        let mut session = landed_with(&catalog, &[131, 129], &[], 6_000);
        mark(&mut session, 0, EscortMark::Sell);
        mark(&mut session, 1, EscortMark::Upgrade);
        session.land_escorts(&mut NeverFires).expect("settled");
        let id = ids(&session)[0].expect("the upgraded escort's NPC");
        assert_eq!(
            session.space.npc(id).expect("in the system").ship,
            ShipId(130)
        );
        assert_eq!(session.pilot.escorts[0].ship, ShipId(130));
    }

    #[test]
    fn an_upgraded_hired_escort_is_paid_by_its_new_class_or_its_kept_wage() {
        let catalog = yards();
        for (rule, wage) in [(RuleSource::Engine, 300), (RuleSource::Bible, 100)] {
            let mut session = landed_with(&catalog, &[129], &[0], 6_000);
            session.setup.rules = Rulebook::default().with_override(RuleKey::EscortWage, rule);
            session.pilot.escorts[0].wage = Some(100);
            mark(&mut session, 0, EscortMark::Upgrade);
            session.land_escorts(&mut NeverFires).expect("upgraded");
            let escort = session.pilot.escorts[0];
            assert_eq!(
                session.setup.paid_wage(&session.statics, &escort),
                wage,
                "{rule:?}"
            );
        }
    }
}
