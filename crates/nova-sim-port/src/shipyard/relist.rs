//! Relisting the shipyard over the classes it listed when it opened.
//!
//! The original builds the shipyard's list once each time it opens:
//! `_SetupPortAvailableShipTypes` (@0xbbe3-0xbeda), called only from
//! `_DoShipyardDialog` @0x5e68f, writes the class IDs alone into
//! `*0x3b5014`, from tech, the day's roll, `Require`, `Availability` and the
//! `HideHigher` sweep. Nothing in the dialog's loop builds it again. Every
//! draw (`_ShipyardDialogUpdate` @0x58a10) reads the rest live: each
//! class's price through `_ApplyPriceAndTechnologyFlux` (@0x59387), the
//! trade-in (@0x59433-0x5949a), the cash, the class flown (@0x592cf) and
//! Buy Ship's greying, which `_CalcShipCanBuy` works out from the class
//! flown, the cash, `Require` and `Availability`, but not the roll
//! (`_ShipyardFilter` @0x6047b, @0x60829, @0x60c48).
//!
//! [`Yard::relisted`] is that draw: the classes given, in the order given,
//! each with its price and its `Require` and `Availability` read now. It
//! draws no roll and runs no tech test or sweep.

use super::{ShipOffer, Shipyard, Yard, available, required};
use crate::catalog::ShipId;
use crate::pilot::Pilot;

impl Yard<'_> {
    /// The shipyard relisting exactly `listing`, the classes its opening
    /// listed, in that order, for `pilot` as it is now: each class's
    /// price, the trade-in, the cash and the class flown read afresh, and
    /// each class buyable when its `Require` is met and its `Availability`
    /// holds now (see the module docs). A class with no `shïp` record is
    /// skipped. `None` when the stellar has no shipyard.
    #[must_use]
    pub fn relisted(&self, pilot: &Pilot, listing: &[ShipId]) -> Option<Shipyard> {
        let contributed = self.contributed(pilot)?;
        let rows = listing
            .iter()
            .filter_map(|&id| self.ships.iter().find(|ship| ship.id == id))
            .map(|ship| {
                let offer = ShipOffer {
                    yard: self,
                    ship,
                    contributed,
                };
                offer.row(required().of(&offer) && available().of(&offer))
            })
            .collect();
        Some(self.listing(pilot, rows))
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{
        buying, cheap, costing, gated, listed, pilot, port, requiring, row, yard,
    };
    use super::*;
    use crate::catalog::{LandingSite, OutfitRecord, ShipRecord};
    use crate::chance::NeverFires;
    use crate::control::Gate;
    use crate::landing::StellarFlags;
    use crate::shipyard::ShipRefusal;
    use crate::wares::DayRolls;
    use nova_sim_catalog::fixture::{AllowAll, RefuseBits, outfit};

    fn ids(ids: &[i16]) -> Vec<ShipId> {
        ids.iter().map(|&id| ShipId(id)).collect()
    }

    fn relist(ships: &[ShipRecord], pilot: &Pilot, listing: &[i16]) -> Shipyard {
        let site = port();
        yard(ships, &[], &site)
            .relisted(pilot, &ids(listing))
            .expect("a shipyard")
    }

    #[test]
    fn it_lists_exactly_the_classes_given_in_the_order_given() {
        // 130's roll is never, and 131's tech is above the port's: a
        // fresh build drops both.
        let ships = [
            cheap(129),
            buying(130, 0),
            costing(131, 1000, 9),
            cheap(132),
        ];
        let site = port();
        let fresh = yard(&ships, &[], &site)
            .shipyard(&pilot(), &mut DayRolls::default(), &mut NeverFires)
            .expect("a shipyard");
        assert_eq!(listed(&fresh), [129, 132], "a fresh build drops them");
        assert_eq!(
            listed(&relist(&ships, &pilot(), &[131, 129, 130])),
            [131, 129, 130]
        );
    }

    #[test]
    fn a_class_with_no_record_is_skipped() {
        assert_eq!(listed(&relist(&[cheap(129)], &pilot(), &[200, 129])), [129]);
    }

    #[test]
    fn a_stellar_without_the_shipyard_flag_has_none() {
        let site = LandingSite {
            flags: StellarFlags::CAN_LAND | StellarFlags::OUTFITTER,
            ..port()
        };
        let ships = [cheap(129)];
        assert_eq!(
            yard(&ships, &[], &site).relisted(&pilot(), &ids(&[129])),
            None
        );
    }

    #[test]
    fn its_values_are_the_pilots_now() {
        let ships = [cheap(129), costing(130, 5000, 4)];
        let mut flying = pilot();
        let before = relist(&ships, &flying, &[129, 130]);
        assert_eq!(before.current, ShipId(128));
        flying.ship = ShipId(129);
        flying.cash = 1234;
        let after = relist(&ships, &flying, &[129, 130]);
        assert_eq!(after.current, ShipId(129), "the class flown now");
        assert_eq!(after.cash, 1234, "the cash now");
        assert_eq!(row(&after, 130).price, 5000, "the price, read afresh");
        let site = port();
        let fresh = yard(&ships, &[], &site)
            .shipyard(&flying, &mut DayRolls::default(), &mut NeverFires)
            .expect("a shipyard");
        assert_eq!(after.trade_in, fresh.trade_in, "the trade-in now");
        assert_ne!(after.trade_in, before.trade_in, "another class's trade-in");
        assert_eq!(after.can(ShipId(129)), Err(ShipRefusal::AlreadyFlown));
        assert_eq!(after.can(ShipId(130)), Err(ShipRefusal::CannotAfford));
    }

    #[test]
    fn require_is_read_now_both_ways() {
        let licence = OutfitRecord {
            contribute: 0x0800_0000_0000,
            ..outfit(140, &[])
        };
        let outfits = [licence];
        let ships = [requiring(129, 0x0800_0000_0000, 0)];
        let site = port();
        let relisted = |pilot: &Pilot| {
            yard(&ships, &outfits, &site)
                .relisted(pilot, &ids(&[129]))
                .expect("a shipyard")
        };
        let mut licensed = pilot();
        licensed.outfits.insert(crate::catalog::OutfitId(140), 1);
        assert_eq!(relisted(&licensed).can(ShipId(129)), Ok(()));
        assert_eq!(
            relisted(&pilot()).can(ShipId(129)),
            Err(ShipRefusal::NotForSale),
            "listed, but greyed once the bit is gone"
        );
    }

    #[test]
    fn availability_is_read_now_both_ways() {
        let ships = [gated(129, 0, "b7")];
        let site = port();
        let allowing = Yard {
            gate: Gate {
                control_bits: &AllowAll,
                ..Gate::FRESH
            },
            ..yard(&ships, &[], &site)
        }
        .relisted(&pilot(), &ids(&[129]))
        .expect("a shipyard");
        assert_eq!(allowing.can(ShipId(129)), Ok(()));
        let refusing = Yard {
            gate: Gate {
                control_bits: &RefuseBits(&[7]),
                ..Gate::FRESH
            },
            ..yard(&ships, &[], &site)
        }
        .relisted(&pilot(), &ids(&[129]))
        .expect("a shipyard");
        assert_eq!(listed(&refusing), [129], "still listed");
        assert_eq!(refusing.can(ShipId(129)), Err(ShipRefusal::NotForSale));
    }

    #[test]
    fn a_relisted_row_is_the_row_a_fresh_build_makes() {
        let ships = [cheap(129), costing(130, 5000, 2)];
        let site = port();
        let fresh = yard(&ships, &[], &site)
            .shipyard(&pilot(), &mut DayRolls::default(), &mut NeverFires)
            .expect("a shipyard");
        let relisted = relist(&ships, &pilot(), &[129, 130]);
        assert_eq!(relisted, fresh);
    }
}
