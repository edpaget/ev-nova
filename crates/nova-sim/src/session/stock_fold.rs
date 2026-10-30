//! A bought ship's stock top-up, held out of the outfit counts until the
//! next `_ShipStatsToSystemInfo`.
//!
//! `_DoShipyardDialog` tops a bought ship's weapon array up to its stock
//! weapons and `AmmoLoad` (@0x5eebc-0x5ef92) and never calls
//! `_ShipStatsToSystemInfo` (@0xcaee-0xcce7), which folds what the array
//! holds beyond the outfit counts into them. Until it runs, the outfit
//! counts leave the top-up out, so the shipyard's trade-in, the next
//! purchase's carry and `Require` don't count it, and a second purchase
//! clears the array and drops it unpaid (see `nova_sim::shipyard`'s Stock
//! weapons).
//!
//! In the port the outfit counts and the weapon array are one map
//! (`Pilot::outfits`), which the armament and the status bar read as the
//! array. By the engine reading of
//! [`RuleKey::PurchaseStockFold`](crate::rulebook::RuleKey::PurchaseStockFold)
//! `PlayerShip::held_stock` records the part of it a purchase's top-up
//! still holds in the array alone ([`shipyard::Purchase::held`]), and
//! `PlayerShip::counted` takes it out for the shipyard's readers
//! ([`shipyard::outfit_counts`]). The fold, [`Session::fold_stock`],
//! forgets it. By the other reading nothing is held. The held part is
//! never saved: the original saves only at the new pilot (@0x1935f), a
//! landing after its fold (@0x6354b) and the rescue (@0x692c4), so a load
//! treats the top-up as owned, as a landing's fold would. The port's save
//! after a purchase is its own boundary.
//!
//! # Fold sites
//!
//! Every caller of `_ShipStatsToSystemInfo` a landed stay can reach
//! between two purchases folds here, at the original's point:
//!
//! | Site | Original | Here |
//! |---|---|---|
//! | the outfitter opening | @0x5bb57 | `Session::open_outfitter` |
//! | the player info, in flight or landed | @0x5a06a | `Session::open_player_info` |
//! | the hire screen closing | `_DoBarDialog` @0x49235 | `Session::close_hire` |
//! | `G` and `D` of an ID of 128-639 | `_EvalSetExp` @0x15424/@0x15439, @0x15452/@0x15466 | `Session::apply_op` |
//! | `C`, `E` and `H` of an ID of 128-895 | `_EvalSetExp` @0x1549d/@0x154b3 (`E`/`H` again @0x15622) | `Session::apply_op` |
//! | a landing | `_PlayerLandOnStellar` @0x63211 | the landing's reset |
//! | any change of class | the weapon array rewritten | `Session::become_class` |
//!
//! A set operator folds whether or not it changes anything, and `C`, `E`
//! and `H` fold even for a class the session has no record of. A
//! purchase's own `OnPurchase` runs after the top-up (@0x5f00d), so one
//! holding such an operator folds it.
//!
//! The rest need no code: the new pilot (@0x18f48) holds nothing yet, and
//! the ship display (@0x2dc3c), `_HandleShip` (@0x33a28), Use As My Ship
//! (@0x419fd), the eject (@0x6f8bb), the pod's ship (@0x6f077) and the
//! plunder dialog (@0x931d9, @0x93cc9, @0x93dcb) run in flight. Those
//! either change the class, which folds, or are followed by a landing,
//! which folds before any shipyard can read the held part, so folding
//! later changes nothing the player can see.
//!
//! Readers outside the shipyard (the hire screen's `Require`, an `o`
//! test, `_ShipFreeMass`, a pirate's plunder, `_ScanPlayer` and the
//! `_Ship*` stat readers past a held outfit's first mod) still read the
//! counts with the top-up: they are
//! rdm:phase/play-feel-5-followups/phase-17-held-stock-outside-shipyard.

use std::ops::RangeInclusive;

use super::Session;
use super::outfits::OUTFIT_IDS;
use crate::control::SetOp;
#[cfg(doc)]
use crate::shipyard;

/// The ship IDs `C`, `E` and `H` fold for: an index below 0x300
/// (`_EvalSetExp` @0x1549d).
const SHIP_IDS: RangeInclusive<i16> = 128..=895;

/// Whether `op` runs `_ShipStatsToSystemInfo` before it acts: `G` and `D`
/// of an outfit ID of 128-639 (@0x15424, @0x15452), and `C`, `E` and `H`
/// of a ship ID of 128-895 (@0x1549d), whether or not they change
/// anything.
pub(super) fn folds_stock(op: &SetOp) -> bool {
    match op {
        SetOp::GrantOutfit(outfit) | SetOp::RemoveOutfit(outfit) => OUTFIT_IDS.contains(&outfit.0),
        SetOp::ChangeShip(ship)
        | SetOp::ChangeShipWithDefaults(ship)
        | SetOp::ReplaceShip(ship) => SHIP_IDS.contains(&ship.0),
        _ => false,
    }
}

impl Session {
    /// `_ShipStatsToSystemInfo`'s fold of a bought ship's stock top-up
    /// into the outfit counts: nothing is held out of them afterwards (see
    /// the module docs). Each fold site's reset stays where it is.
    pub(super) fn fold_stock(&mut self) {
        self.ship.fold_stock();
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::catalog::{OutfitId, OutfitRecord, ShipId, ShipRecord};
    use crate::chance::NeverFires;
    use crate::control::{Script, SetOp};
    use crate::outfitter::OutfitFlags;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::session::tests::{
        GUN, LAUNCHER, NEW, NameLists, ROCKETS, TANK, mounted, outfitted, shipbuying, stock_weapons,
    };
    use crate::shipyard::ShipRefusal;
    use crate::testkit::{FAST, FakePilotCatalog, land_now, outfit, ship};

    /// The first ship, which [`stock_weapons`] stocks with two guns.
    const OLD: ShipId = ShipId(128);

    /// A persistent outfit, which the engine's trade-in leaves out, so
    /// granting it adds nothing to the trade-in.
    const LICENCE: OutfitId = OutfitId(340);

    /// The trade-in of ship 129 (17,500 credits) with its tank (1000)
    /// alone: a quarter of the hull and half the tank, 4375 + 500 = 4875,
    /// rounded down to ten by the flux.
    const WITHOUT_STOCK: i64 = 4870;

    /// The trade-in of ship 129 with its tank, gun (1000), launcher (2000)
    /// and 20 rockets (20 each) owned: 4375 + 500 + 500 + 1000 + 200 =
    /// 6575, rounded down to ten by the flux.
    const WITH_STOCK: i64 = 6570;

    /// [`stock_weapons`] over [`shipbuying`], with [`LICENCE`].
    fn stocked() -> FakePilotCatalog {
        let mut catalog = stock_weapons(shipbuying());
        catalog.outfits.push(OutfitRecord {
            flags: OutfitFlags::PERSISTENT,
            ..outfit(LICENCE.0, &[])
        });
        catalog
    }

    /// A session landed at `catalog`'s shipyard by `rules`, rich enough
    /// for two purchases, having bought ship 129.
    fn bought(catalog: &FakePilotCatalog, rules: Rulebook) -> Session {
        let mut session = outfitted(catalog).with_rules(rules);
        session.pilot.cash = 100_000;
        session
            .buy_ship(NEW, "Kestrel", &mut NeverFires)
            .expect("bought");
        session
    }

    /// The trade-in `session`'s shipyard shows.
    fn trade_in(session: &mut Session) -> i64 {
        session
            .shipyard(&mut NeverFires)
            .expect("a shipyard")
            .trade_in
    }

    /// A named step a stay can take after a purchase.
    type Step<'a> = (&'static str, &'a dyn Fn(&mut Session));

    /// The default rules with the other reading of `purchase_stock_fold`.
    fn at_once() -> Rulebook {
        Rulebook::default().with_override(RuleKey::PurchaseStockFold, RuleSource::Bible)
    }

    #[test]
    fn the_trade_in_counts_stock_weapons_and_rounds_as_the_original_does() {
        let catalog = stocked();
        let mut session = outfitted(&catalog);
        assert_eq!(trade_in(&mut session), 2500 + 2 * 500, "two guns");
        let mut session = bought(&catalog, Rulebook::default());
        assert_eq!(trade_in(&mut session), WITHOUT_STOCK, "held in the array");
        session.open_outfitter();
        session.close_outfitter();
        assert_eq!(trade_in(&mut session), WITH_STOCK, "folded");
    }

    #[test]
    fn by_the_engine_a_second_purchase_in_one_stay_trades_in_without_the_first_ships_stock() {
        let catalog = stocked();
        let mut session = bought(&catalog, Rulebook::default());
        assert_eq!(
            session.ship.held_stock,
            BTreeMap::from([(GUN, 1), (LAUNCHER, 1), (ROCKETS, 20)]),
            "the top-up; the tank is a default item"
        );
        assert_eq!(trade_in(&mut session), WITHOUT_STOCK);
        let before = session.pilot().cash();
        let report = session
            .buy_ship(OLD, "Kestrel", &mut NeverFires)
            .expect("bought again");
        assert_eq!(report.trade_in, WITHOUT_STOCK);
        assert_eq!(report.price, 10_000);
        assert_eq!(session.pilot().cash(), before - 10_000 + WITHOUT_STOCK);
        assert_eq!(
            session.pilot().outfits().collect::<Vec<_>>(),
            [(GUN, 2)],
            "ship 128's own top-up; the launcher and rockets are gone"
        );
        assert_eq!(session.ship.held_stock, BTreeMap::from([(GUN, 2)]));
    }

    #[test]
    fn the_stock_is_folded_by_the_outfitter_player_info_hire_screen_a_set_operator_or_a_landing() {
        let catalog = stocked();
        let script = |session: &mut Session, text: &str| {
            session.run_script(&Script::parse(text), &mut NeverFires);
        };
        let folds: [Step; 9] = [
            ("the outfitter", &|session| {
                session.open_outfitter();
                session.close_outfitter();
            }),
            ("the player info", &|session| {
                session.open_player_info(&catalog).expect("shown");
            }),
            ("the hire screen", &|session| session.close_hire()),
            ("G", &|session| script(session, "G340")),
            ("D of none owned", &|session| script(session, "D340")),
            ("C of no class", &|session| script(session, "C200")),
            ("E of no class", &|session| script(session, "E895")),
            ("H of no class", &|session| script(session, "H200")),
            ("a landing", &|session| {
                session.take_off(&catalog, &mut NeverFires);
                land_now(session).expect("lands");
            }),
        ];
        for (fold, run) in folds {
            let mut session = bought(&catalog, Rulebook::default());
            run(&mut session);
            assert_eq!(session.ship.held_stock, BTreeMap::new(), "{fold}");
            assert_eq!(trade_in(&mut session), WITH_STOCK, "{fold}");
            let report = session
                .buy_ship(OLD, "Kestrel", &mut NeverFires)
                .expect("bought again");
            assert_eq!(report.trade_in, WITH_STOCK, "{fold}");
        }
        // The parser keeps `G`/`D` to 128-639 and `C`/`E`/`H` to 128-895;
        // past them `apply_op` folds nothing, as `_EvalSetExp` skips them.
        let op = |session: &mut Session, op: SetOp| session.apply_op(&op, &mut NeverFires);
        let stays: [Step; 9] = [
            ("the shipyard opening", &|session| {
                session.open_shipyard(&mut NeverFires).expect("a shipyard");
            }),
            ("a decline", &|session| session.decline_ship(OLD)),
            ("the name prompt", &|session| {
                session
                    .ship_naming(OLD, &NameLists, &mut NeverFires)
                    .expect("named");
            }),
            ("G past the outfits", &|session| {
                op(session, SetOp::GrantOutfit(OutfitId(640)));
            }),
            ("D below the outfits", &|session| {
                op(session, SetOp::RemoveOutfit(OutfitId(127)));
            }),
            ("C past the ships", &|session| {
                op(session, SetOp::ChangeShip(ShipId(896)));
            }),
            ("E below the ships", &|session| {
                op(session, SetOp::ChangeShipWithDefaults(ShipId(127)));
            }),
            ("H past the ships", &|session| {
                op(session, SetOp::ReplaceShip(ShipId(896)));
            }),
            ("a bit write", &|session| script(session, "b1")),
        ];
        for (stay, run) in stays {
            let mut session = bought(&catalog, Rulebook::default());
            run(&mut session);
            assert_eq!(trade_in(&mut session), WITHOUT_STOCK, "{stay}");
        }
    }

    #[test]
    fn an_on_purchase_set_operator_folds_the_stock() {
        let mut catalog = stocked();
        catalog.ship_records[1].on_purchase = Script::parse("G340");
        let mut session = bought(&catalog, Rulebook::default());
        assert_eq!(session.pilot().owned(LICENCE), 1, "granted");
        assert_eq!(trade_in(&mut session), WITH_STOCK);
    }

    #[test]
    fn by_the_engine_a_held_persistent_stock_outfit_is_not_carried() {
        let mut catalog = stocked();
        for record in &mut catalog.outfits {
            if record.id == LAUNCHER {
                record.flags = OutfitFlags::PERSISTENT;
            }
        }
        let carried = |rules| {
            let mut session = bought(&catalog, rules);
            session
                .buy_ship(OLD, "Kestrel", &mut NeverFires)
                .expect("bought again");
            session.pilot().owned(LAUNCHER)
        };
        assert_eq!(carried(Rulebook::default()), 0, "cleared with the array");
        assert_eq!(carried(at_once()), 1, "owned, so carried");
    }

    /// A class (130, tech 5) whose `Require` needs bit 0x4.
    const LICENSED: ShipId = ShipId(130);

    /// [`stocked`] with [`LAUNCHER`] contributing bit 0x4 and
    /// [`LICENSED`] on sale, requiring it.
    fn licensing() -> FakePilotCatalog {
        let mut catalog = stocked();
        for record in &mut catalog.outfits {
            if record.id == LAUNCHER {
                record.contribute = 0x4;
            }
        }
        catalog.ship_records.push(ShipRecord {
            require: 0x4,
            tech_level: 5,
            ..ship(LICENSED.0, FAST)
        });
        catalog
    }

    /// Whether `session`'s shipyard lists [`LICENSED`] as buyable.
    fn licensed(session: &mut Session) -> bool {
        session
            .shipyard(&mut NeverFires)
            .expect("a shipyard")
            .row(LICENSED)
            .is_some_and(|row| row.facts.buyable)
    }

    #[test]
    fn by_the_engine_require_leaves_out_a_held_stock_outfits_contribute_bits() {
        let catalog = licensing();
        let mut session = bought(&catalog, Rulebook::default());
        assert!(!licensed(&mut session), "held in the array");
        assert_eq!(
            session.buy_ship(LICENSED, "Kestrel", &mut NeverFires).err(),
            Some(ShipRefusal::NotForSale),
            "refused"
        );
        session.open_outfitter();
        session.close_outfitter();
        assert!(licensed(&mut session), "folded");
        let mut session = bought(&catalog, at_once());
        assert!(licensed(&mut session), "owned at once");
    }

    #[test]
    fn by_the_other_reading_the_stock_is_owned_at_once() {
        let catalog = stocked();
        let mut session = bought(&catalog, at_once());
        assert_eq!(session.ship.held_stock, BTreeMap::new());
        assert_eq!(trade_in(&mut session), WITH_STOCK);
        let report = session
            .buy_ship(OLD, "Kestrel", &mut NeverFires)
            .expect("bought again");
        assert_eq!(report.trade_in, WITH_STOCK);
    }

    #[test]
    fn the_armament_and_outfits_read_the_top_up_by_either_reading() {
        let catalog = stocked();
        let engine = bought(&catalog, Rulebook::default());
        let other = bought(&catalog, at_once());
        assert_eq!(mounted(&engine), [(150, 1), (138, 1)]);
        assert_eq!(mounted(&engine), mounted(&other));
        assert_eq!(
            engine.pilot().outfits().collect::<Vec<_>>(),
            [(TANK, 1), (GUN, 1), (LAUNCHER, 1), (ROCKETS, 20)]
        );
        assert_eq!(
            engine.pilot().outfits().collect::<Vec<_>>(),
            other.pilot().outfits().collect::<Vec<_>>()
        );
    }
}
