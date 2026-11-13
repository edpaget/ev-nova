//! The shipyard's list, kept from its opening for as long as it stays open.
//!
//! The original builds the shipyard's list once each time it opens
//! (`_SetupPortAvailableShipTypes` @0xbbe3-0xbeda, called only from
//! `_DoShipyardDialog` @0x5e68f), into an array of class IDs at
//! `*0x3b5014`, and nothing in the dialog's loop builds it again: a decline,
//! an offer, mission info and player info only redraw it. Each draw reads
//! the rest live (see `nova_sim::shipyard`'s relisting).
//!
//! Here [`Session::open_shipyard`] applies the redraws due and builds the
//! shipyard afresh, and the port keeps the classes it listed
//! (`Port::shipyard_listing`). While it is kept, [`Session::shipyard`] and
//! the purchase relist those classes, in that order, with the prices, the
//! trade-in, the cash, the class flown and Buy Ship's greying read now.
//! The next opening replaces it, and a landing, a relocation and a
//! take-off forget it. With none kept (a session that never opened the
//! shipyard), the shipyard is built afresh at each ask.

use super::Session;
use super::port::Port;
use crate::catalog::ShipId;
use crate::chance::Chance;
use crate::shipyard::Shipyard;

impl Session {
    /// The shipyard dialog opens, as `_DoShipyardDialog` builds its list on
    /// entry (@0x5e68f): the roll of each class bought or declined since
    /// the last opening or landing is drawn again, the rolls not drawn yet
    /// are drawn on `chance`, and the classes listed are kept for the
    /// visit (see the module docs). `None` when the ship is not landed at a
    /// shipyard, and then nothing is kept.
    ///
    /// The original draws a class's roll again at once, after the purchase
    /// or the decline (@0x5f0d5-0x5f0f8). Here it is drawn at the next
    /// opening or landing: nothing reads the rolls in between but the
    /// opening's build, and they are never saved, so no frame, dialog or
    /// save can tell the two apart.
    pub(crate) fn open_shipyard(&mut self, chance: &mut dyn Chance) -> Option<Shipyard> {
        self.port.redraw_ships();
        self.port.forget_shipyard_listing();
        let shipyard = self.shipyard(chance);
        self.port.shipyard_listing = shipyard
            .as_ref()
            .map(|shipyard| shipyard.rows.iter().map(|row| row.id).collect());
        shipyard
    }
}

impl Port {
    /// Draws again the roll of each class bought or declined since it was
    /// last drawn: the next ask draws it afresh.
    pub(super) fn redraw_ships(&mut self) {
        for ship in std::mem::take(&mut self.ship_redraws) {
            self.ship_rolls.redraw(&ship);
        }
    }

    /// Forgets the classes the shipyard listed when it last opened, so the
    /// next ask builds it afresh.
    pub(super) fn forget_shipyard_listing(&mut self) {
        self.shipyard_listing = None;
    }

    /// The classes the shipyard listed when it opened, while it is kept.
    pub(super) fn shipyard_listing(&self) -> Option<&[ShipId]> {
        self.shipyard_listing.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{LandingSite, StellarId, SystemId};
    use crate::chance::NeverFires;
    use crate::control::{Bit, Test};
    use crate::landing::StellarFlags;
    use crate::session::tests::{NEW, NameLists, outfitted, rolling, shipbuying};
    use crate::shipyard::{ShipFlags3, ShipRefusal};
    use crate::testkit::{FakePilotCatalog, Scripted, catalog, land_now, planet};

    fn lists_new(session: &mut Session, chance: &mut dyn Chance) -> bool {
        let shipyard = session.shipyard(chance).expect("a shipyard");
        shipyard.row(NEW).is_some()
    }

    /// A session landed at [`rolling`]'s shipyard, ship 129 on a
    /// `BuyRandom` of 50, opened on a roll that lists it.
    fn opened() -> Session {
        let mut session = outfitted(&rolling(100, 50));
        let mut firing = Scripted::answering(&[true]);
        let shipyard = session.open_shipyard(&mut firing).expect("a shipyard");
        assert!(shipyard.row(NEW).is_some(), "listed on opening");
        assert_eq!(firing.asked, [50]);
        session
    }

    #[test]
    fn a_declined_class_stays_listed_until_the_shipyard_opens_again() {
        let mut session = opened();
        session.decline_ship(NEW);
        let mut missing = Scripted::answering(&[false]);
        assert!(lists_new(&mut session, &mut missing), "still listed");
        assert!(missing.asked.is_empty(), "no roll drawn while open");
        let reopened = session.open_shipyard(&mut missing).expect("a shipyard");
        assert_eq!(reopened.row(NEW), None, "its new roll misses");
        assert_eq!(missing.asked, [50], "drawn again at the opening");
    }

    #[test]
    fn a_bought_class_stays_listed_with_the_values_of_now_until_the_shipyard_opens_again() {
        let mut session = opened();
        let before = session.shipyard(&mut NeverFires).expect("a shipyard");
        session
            .buy_ship(NEW, "Kestrel", &mut NeverFires)
            .expect("bought");
        let mut missing = Scripted::answering(&[false]);
        let after = session.shipyard(&mut missing).expect("a shipyard");
        assert!(missing.asked.is_empty(), "no roll drawn while open");
        assert_eq!(
            after.rows.iter().map(|row| row.id).collect::<Vec<_>>(),
            before.rows.iter().map(|row| row.id).collect::<Vec<_>>(),
            "the opening's list"
        );
        assert_eq!(after.current, NEW, "the class flown now");
        assert_eq!(after.cash, session.pilot().cash, "the cash now");
        assert_ne!(after.cash, before.cash);
        let mut fresh = session.clone();
        fresh.port.forget_shipyard_listing();
        let fresh = fresh.shipyard(&mut NeverFires).expect("a shipyard");
        assert_eq!(after.trade_in, fresh.trade_in, "the trade-in now");
        assert_ne!(after.trade_in, before.trade_in);
        assert_eq!(after.can(NEW), Err(ShipRefusal::AlreadyFlown));
        let reopened = session.open_shipyard(&mut missing).expect("a shipyard");
        assert_eq!(reopened.row(NEW), None, "its new roll misses");
        assert_eq!(missing.asked, [50]);
    }

    /// [`shipbuying`] with ship 129 sold only on control bit 7: hidden
    /// while it is refused when `hidden`, greyed otherwise.
    fn on_bit_7(hidden: bool) -> FakePilotCatalog {
        let mut catalog = shipbuying();
        let new = &mut catalog.ship_records[1];
        new.availability = Test::parse("b7");
        if hidden {
            new.flags3 |= ShipFlags3::HIDE_UNLESS_AVAILABLE;
        }
        catalog
    }

    fn bit_7() -> Bit {
        Bit::new(7).expect("a bit")
    }

    #[test]
    fn a_class_its_availability_adds_is_listed_only_at_the_next_opening() {
        let mut session = outfitted(&on_bit_7(true));
        let shipyard = session.open_shipyard(&mut NeverFires).expect("a shipyard");
        assert_eq!(shipyard.row(NEW), None);
        session.set_control_bit(bit_7(), true);
        assert!(!lists_new(&mut session, &mut NeverFires), "not while open");
        assert_eq!(
            session.ship_naming(NEW, &NameLists, &mut NeverFires),
            Err(ShipRefusal::NotListed)
        );
        let reopened = session.open_shipyard(&mut NeverFires).expect("a shipyard");
        assert!(reopened.row(NEW).is_some(), "listed at the next opening");
    }

    #[test]
    fn buy_ships_greying_reads_availability_now() {
        let mut session = outfitted(&on_bit_7(false));
        let shipyard = session.open_shipyard(&mut NeverFires).expect("a shipyard");
        assert_eq!(shipyard.can(NEW), Err(ShipRefusal::NotForSale));
        session.set_control_bit(bit_7(), true);
        let relisted = session.shipyard(&mut NeverFires).expect("a shipyard");
        assert_eq!(relisted.can(NEW), Ok(()), "buyable now");
        session.set_control_bit(bit_7(), false);
        let relisted = session.shipyard(&mut NeverFires).expect("a shipyard");
        assert_eq!(relisted.can(NEW), Err(ShipRefusal::NotForSale));
    }

    #[test]
    fn the_next_opening_replaces_the_listing() {
        let mut session = opened();
        session.port.shipyard_listing = Some(vec![ShipId(200)]);
        let shipyard = session.open_shipyard(&mut NeverFires).expect("a shipyard");
        let listed: Vec<ShipId> = shipyard.rows.iter().map(|row| row.id).collect();
        assert_eq!(listed, [ShipId(128), NEW]);
        assert_eq!(session.port.shipyard_listing(), Some(&listed[..]));
    }

    #[test]
    fn a_fresh_session_keeps_no_listing() {
        let session = Session::start(&shipbuying()).expect("starts");
        assert_eq!(session.port.shipyard_listing(), None);
    }

    #[test]
    fn an_opening_with_no_shipyard_keeps_nothing() {
        let mut session = Session::start(&shipbuying()).expect("starts");
        session.port.shipyard_listing = Some(vec![NEW]);
        assert_eq!(session.open_shipyard(&mut NeverFires), None, "in flight");
        assert_eq!(session.port.shipyard_listing(), None);
    }

    #[test]
    fn a_take_off_forgets_the_listing() {
        let mut session = opened();
        session
            .take_off(&catalog(), &mut NeverFires)
            .expect("takes off");
        assert_eq!(session.port.shipyard_listing(), None);
    }

    #[test]
    fn a_landing_forgets_the_listing() {
        let mut session = opened();
        session
            .take_off(&catalog(), &mut NeverFires)
            .expect("takes off");
        session.port.shipyard_listing = Some(vec![NEW]);
        land_now(&mut session).expect("lands again");
        assert_eq!(session.port.shipyard_listing(), None);
    }

    #[test]
    fn a_relocation_forgets_the_listing() {
        let mut catalog = shipbuying();
        catalog.sites[0].1.push(LandingSite {
            flags: StellarFlags::CAN_LAND | StellarFlags::SHIPYARD,
            ..planet(129, 400.0, 0.0)
        });
        let mut session = outfitted(&catalog);
        session.open_shipyard(&mut NeverFires).expect("a shipyard");
        assert!(session.port.shipyard_listing().is_some());
        session
            .relocate(&catalog, SystemId(130), StellarId(129))
            .expect("relocated");
        assert_eq!(session.port.shipyard_listing(), None);
    }

    #[test]
    fn without_an_opening_the_shipyard_is_built_afresh_and_draws_no_redraw() {
        let mut session = outfitted(&rolling(100, 50));
        let mut firing = Scripted::answering(&[true]);
        assert!(lists_new(&mut session, &mut firing));
        session.decline_ship(NEW);
        let mut missing = Scripted::answering(&[false]);
        assert!(lists_new(&mut session, &mut missing), "the roll kept");
        assert!(missing.asked.is_empty());
        assert_eq!(session.port.shipyard_listing(), None, "nothing kept");
    }
}
