//! The readers of the outfit counts outside the shipyard, and whether
//! each leaves a purchase's held stock out.
//!
//! The original keeps two stores: the outfit counts (`*0x3b51a8`) and
//! the weapon array (the player's slot +0xc8, rounds +0xd0). A purchase
//! tops the array up to its stock weapons and `AmmoLoad`, and only the
//! next `_ShipStatsToSystemInfo` folds that top-up into the counts (see
//! the `stock_fold` module). In the port both stores are one map,
//! `Pilot::outfits`, which holds the top-up at once; by the engine reading
//! of [`RuleKey::PurchaseStockFold`](crate::rulebook::RuleKey::PurchaseStockFold)
//! `PlayerShip::held_stock` records the part still held in the array
//! alone. A site that models a read of the counts takes
//! `PlayerShip::counted_outfits`, the held part left out; a site that
//! models a read of the array keeps `Pilot::outfits`.

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::collections::BTreeMap;

    use crate::catalog::{CronId, CronRecord, OutfitId, ShipId, ShipRecord};
    use crate::chance::NeverFires;
    use crate::control::{PilotFacts, Test};
    use crate::cron::CronRefusal;
    use crate::landing::StellarFlags;
    use crate::rulebook::Rulebook;
    use crate::rules::Verdict;
    use crate::session::Session;
    use crate::session::tests::{GUN, LAUNCHER, ROCKETS, TANK};
    use crate::shipyard::ShipFlags3;
    use crate::testkit::{FAST, FakePilotCatalog, cron, land_now, ship};

    use super::super::stock_fold::fixture::{at_once, bought, stocked};

    /// The three answers every reader is pinned by: (a) by the engine,
    /// after the purchase; (b) by the engine, after `fold` runs; (c) by
    /// the other reading, after the purchase.
    fn readings<T>(
        catalog: &FakePilotCatalog,
        setup: impl Fn(&mut Session),
        fold: impl Fn(&mut Session),
        read: impl Fn(&mut Session) -> T,
    ) -> [T; 3] {
        let mut held = bought(catalog, Rulebook::default());
        setup(&mut held);
        let a = read(&mut held);
        fold(&mut held);
        let b = read(&mut held);
        let mut at_once = bought(catalog, at_once());
        setup(&mut at_once);
        let c = read(&mut at_once);
        [a, b, c]
    }

    /// The outfitter's fold (@0x5bb57), for the readers of a stay.
    fn outfitter(session: &mut Session) {
        session.open_outfitter();
        session.close_outfitter();
    }

    /// Nothing between the purchase and the read.
    fn stay(_: &mut Session) {}

    /// The take-off, for the readers in flight: the held part survives
    /// it, as no in-flight code folds.
    fn take_off(catalog: &FakePilotCatalog) -> impl Fn(&mut Session) + '_ {
        |session| {
            session.take_off(catalog, &mut NeverFires);
        }
    }

    /// A landing's fold (@0x63211), for the readers in flight.
    fn land(session: &mut Session) {
        land_now(session).expect("lands");
    }

    /// A landing's fold and a take-off again, for the readers in flight.
    fn relaunch(catalog: &FakePilotCatalog) -> impl Fn(&mut Session) + '_ {
        move |session| {
            land(session);
            take_off(catalog)(session);
        }
    }

    /// `catalog` with `edit` made to outfit `id`'s record.
    fn with_outfit(
        mut catalog: FakePilotCatalog,
        id: OutfitId,
        edit: impl FnOnce(&mut crate::catalog::OutfitRecord),
    ) -> FakePilotCatalog {
        let record = catalog
            .outfits
            .iter_mut()
            .find(|record| record.id == id)
            .expect("listed");
        edit(record);
        catalog
    }

    #[test]
    fn the_held_stock_survives_a_take_off() {
        let catalog = stocked();
        let mut session = bought(&catalog, Rulebook::default());
        let held = session.ship.held_stock.clone();
        take_off(&catalog)(&mut session);
        assert_eq!(session.ship.held_stock, held);
        assert_ne!(held, BTreeMap::new());
    }

    #[test]
    fn the_counted_outfits_leave_the_held_stock_out_by_the_engine() {
        let catalog = stocked();
        let session = bought(&catalog, Rulebook::default());
        let counted = session.ship.counted_outfits(&session.pilot);
        assert!(matches!(counted, Cow::Owned(_)));
        assert_eq!(*counted, BTreeMap::from([(TANK, 1)]));
        let mut session = session;
        session.ship.held_stock = BTreeMap::from([(GUN, 5), (LAUNCHER, 0), (ROCKETS, 19)]);
        assert_eq!(
            *session.ship.counted_outfits(&session.pilot),
            BTreeMap::from([(TANK, 1), (LAUNCHER, 1), (ROCKETS, 1)]),
            "the held part out, saturating at none"
        );
    }

    #[test]
    fn the_counted_outfits_are_the_pilots_own_with_nothing_held() {
        let catalog = stocked();
        let session = bought(&catalog, at_once());
        let counted = session.ship.counted_outfits(&session.pilot);
        assert!(matches!(counted, Cow::Borrowed(_)));
        assert_eq!(*counted, session.pilot.outfits);
        assert_eq!(counted.get(&GUN), Some(&1));
    }

    #[test]
    fn an_o_test_leaves_a_held_outfit_out_until_the_fold() {
        let catalog = stocked();
        assert_eq!(
            readings(&catalog, stay, outfitter, |session| session.has_outfit(GUN)),
            [false, true, true]
        );
    }

    /// A for-hire class whose `Availability` is `o` of the held gun.
    const GUNNED: ShipId = ShipId(131);

    /// [`stocked`] with planet 128 a bar too, hiring out [`GUNNED`] every
    /// day.
    fn hiring() -> FakePilotCatalog {
        let mut catalog = stocked();
        catalog.sites[0].1[0].flags |= StellarFlags::BAR;
        catalog.ship_records.push(ShipRecord {
            availability: Test::parse("o305"),
            hire_random: 100,
            ..ship(GUNNED.0, FAST)
        });
        catalog
    }

    /// Whether the bar lets [`GUNNED`] be hired.
    fn gunned_for_hire(session: &mut Session) -> bool {
        session
            .escorts_for_hire(&mut NeverFires)
            .expect("a bar")
            .row(GUNNED)
            .is_some_and(|row| row.hire.is_ok())
    }

    #[test]
    fn the_hire_screens_availability_leaves_a_held_outfit_out_until_it_closes() {
        let catalog = hiring();
        assert_eq!(
            readings(&catalog, stay, Session::close_hire, gunned_for_hire),
            [false, true, true],
            "_DoBarDialog folds as the hire screen closes (@0x49235)"
        );
    }

    /// The `Contribute` bit [`licensed`] gives the held gun.
    const GUN_BIT: u64 = 0x8;

    /// [`stocked`] with the gun contributing [`GUN_BIT`].
    fn licensed() -> FakePilotCatalog {
        with_outfit(stocked(), GUN, |record| record.contribute = GUN_BIT)
    }

    #[test]
    fn the_contributed_bits_leave_a_held_outfits_out_until_the_fold() {
        let catalog = licensed();
        let contributes = |session: &mut Session| {
            session.ship.contributed(&session.pilot, &session.statics) & GUN_BIT != 0
        };
        assert_eq!(
            readings(&catalog, stay, outfitter, contributes),
            [false, true, true],
            "_GetPlayerContributeBits @0x7730-0x775f reads the counts"
        );
    }

    #[test]
    fn the_hire_screens_require_leaves_a_held_outfits_bits_out_until_it_closes() {
        let mut catalog = licensed();
        catalog.sites[0].1[0].flags |= StellarFlags::BAR;
        catalog.ship_records.push(ShipRecord {
            require: GUN_BIT,
            flags3: ShipFlags3::HIDE_UNLESS_REQUIRED,
            hire_random: 100,
            ..ship(GUNNED.0, FAST)
        });
        let listed = |session: &mut Session| {
            session
                .escorts_for_hire(&mut NeverFires)
                .expect("a bar")
                .row(GUNNED)
                .is_some()
        };
        assert_eq!(
            readings(&catalog, stay, Session::close_hire, listed),
            [false, true, true],
            "_SetupPortAvailableShipTypes @0xbd09, before @0x49235 folds"
        );
    }

    #[test]
    fn a_crons_require_in_flight_leaves_a_held_outfits_bits_out_until_the_landing() {
        let mut catalog = licensed();
        catalog.crons.push(CronRecord {
            require: GUN_BIT,
            ..cron(128)
        });
        let required = |session: &mut Session| {
            session
                .cron_refusals(CronId(128))
                .expect("a crön")
                .iter()
                .any(|(_, verdict)| *verdict == Verdict::Refuses(CronRefusal::Require))
        };
        assert_eq!(
            readings(&catalog, take_off(&catalog), land, required),
            [true, false, false],
            "_ActivateCron @0x38681 on a jump's days"
        );
    }

    // The scan.

    use crate::SimMessage;
    use crate::catalog::{GovtId, GovtRecord, Penalties};
    use crate::legal::NovaLaw;
    use crate::testkit::{Draws, govt, npc};
    use crate::traffic::npc::AiType;

    /// [`stocked`] with the gun masked 0x0004 and the planet's
    /// government, 128, scanning for it without a fine.
    fn smuggling() -> FakePilotCatalog {
        let mut catalog = with_outfit(stocked(), GUN, |record| record.scan_mask = 0x0004);
        catalog.govts = vec![GovtRecord {
            penalties: Penalties {
                smuggle: 5,
                ..Penalties::default()
            },
            scan_mask: 0x0004,
            news_pic: -1,
            ..govt(128)
        }];
        catalog
    }

    /// Whether an interceptor of government 128 beside the player finds
    /// the gun: `Some(one)` when it does.
    fn scan(session: &mut Session) -> Option<bool> {
        let mut police = npc(0, session.stats());
        police.govt = Some(GovtId(128));
        police.ai_type = AiType::Interceptor;
        police.ship = ShipId(129);
        police.state.position = session.ship.player.position;
        let id = session.space.traffic.add_npc(police);
        let _ = session.take_messages();
        session.scanned_by(id, &NovaLaw::default(), &mut Draws::of(&[0]));
        session
            .take_messages()
            .into_iter()
            .find_map(|message| match message {
                SimMessage::IllegalOutfit { outfit, one, .. } if outfit == GUN => Some(one),
                _ => None,
            })
    }

    #[test]
    fn a_scan_in_flight_leaves_a_held_outfit_out_until_the_landing() {
        let catalog = smuggling();
        assert_eq!(
            readings(&catalog, take_off(&catalog), relaunch(&catalog), scan),
            [None, Some(true), Some(true)],
            "_ScanPlayer @0x7efd1-0x7eff6 reads the counts"
        );
    }

    #[test]
    fn a_scan_counts_one_of_an_outfit_owned_once_beside_a_held_one() {
        let catalog = smuggling();
        let owned_one_more = |session: &mut Session| {
            take_off(&catalog)(session);
            *session.pilot.outfits.entry(GUN).or_default() += 1;
        };
        let [engine, _, other] = readings(&catalog, owned_one_more, stay, scan);
        assert_eq!(engine, Some(true), "one in the counts (@0x7f10e)");
        assert_eq!(other, Some(false), "two owned");
    }
}
