//! A stellar's landing `Fee`, as the session charges it: the press's
//! refusal of a pilot who can't pay is the `landing` rules'
//! ([`LandingRefusal::CannotPayFee`](crate::LandingRefusal::CannotPayFee),
//! from `land_or_select`), and the charge is here ([`Chart::charge_fee`]).
//!
//! `_HandlePlayerDockRequest` charges on its success path only
//! (@0x66db9-0x66df5), after the speed and reach gates pass and before it
//! branches on the stellar's kind, so the one charge serves a landing, a
//! hypergate's entry (`_PlayerEnterHypergate` @0x66ed6-0x66edb) and a
//! wormhole's (`_PlayerEnterWormhole` @0x66f0d-0x66f12). A hypergate map
//! cancelled after it (@0x66ee0-0x66f08) does not refund it. The charge
//! plays no sound.

use super::Chart;
use crate::catalog::StellarId;
use crate::landing::{fee_due, pay_fee};
use crate::pilot::Pilot;

impl Chart {
    /// Charges `pilot` the landing fee of `stellar`, one the system lists,
    /// when one is due ([`fee_due`]): its `Fee` from their cash, never
    /// below 0 ([`pay_fee`], @0x66de3-0x66df5). The press has already
    /// refused a pilot who can't pay.
    pub(super) fn charge_fee(&self, stellar: StellarId, pilot: &mut Pilot) {
        let fee = self
            .listed
            .iter()
            .find(|site| site.id == stellar)
            .and_then(fee_due);
        if let Some(fee) = fee {
            pilot.cash = pay_fee(pilot.cash, fee);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::catalog::{LandingSite, StellarId};
    use crate::chance::NeverFires;
    use crate::clearance::{CLEARED, DockRequest};
    use crate::combat::hull::Condition;
    use crate::gate::{GateKind, GateRefusal};
    use crate::geometry::Vec2;
    use crate::landing::{LandOutcome, LandingRefusal};
    use crate::market::DOMINATED;
    use crate::message::SimMessage;
    use crate::session::tests::{gated, wormholes};
    use crate::session::{LandPress, Session};
    use crate::testkit::{FakePilotCatalog, catalog};

    /// The fee the tests charge.
    const FEE: i32 = 100;

    /// `catalog` with its first stellar of system 130 charging `fee`.
    fn charging(mut catalog: FakePilotCatalog, fee: i32) -> FakePilotCatalog {
        catalog.sites[0].1[0].fee = fee;
        catalog
    }

    /// The session of `catalog`, the pilot holding `cash`, the ship at
    /// rest over its first stellar of system 130 with nothing selected
    /// and that stellar's request at [`CLEARED`], no message or sound
    /// pending.
    fn over(catalog: &FakePilotCatalog, cash: i64) -> Session {
        let site: LandingSite = catalog.sites[0].1[0];
        let mut session = Session::start(catalog).expect("starts");
        session.pilot.cash = cash;
        session.ship.player.position = site.position;
        session.ship.player.velocity = Vec2::ZERO;
        session.port.dock = Some(DockRequest {
            stellar: site.id,
            permission: CLEARED,
        });
        session.take_messages();
        session.take_sounds();
        session
    }

    #[test]
    fn a_pilot_who_cannot_pay_is_refused_silently_and_the_request_dropped() {
        let catalog = charging(catalog(), FEE);
        let mut session = over(&catalog, 99);
        assert_eq!(
            session.land(),
            Err(LandingRefusal::CannotPayFee {
                stellar: StellarId(128),
                station: false,
            })
        );
        assert_eq!(session.take_sounds(), [], "no beep, no traffic control");
        assert_eq!(session.take_messages(), [], "the view words the refusal");
        assert_eq!(session.dock_request(), None);
        assert_eq!(session.pilot.cash, 99);
        assert_eq!(session.nav_target(), Some(StellarId(128)), "selected");
        assert_eq!(session.landed(), None);
    }

    #[test]
    fn a_pilot_who_can_pay_lands_and_pays() {
        let catalog = charging(catalog(), FEE);
        let mut session = over(&catalog, 500);
        assert_eq!(
            session.land(),
            Ok(LandPress::Outcome(LandOutcome::Landed(StellarId(128))))
        );
        assert_eq!(session.pilot.cash, 400);
        assert_eq!(session.landed(), Some(StellarId(128)));
    }

    #[test]
    fn a_stellar_with_no_fee_due_charges_nothing() {
        let dominated = {
            let mut catalog = charging(catalog(), FEE);
            catalog.sites[0].1[0].flags2 |= DOMINATED;
            catalog
        };
        for catalog in [dominated, charging(catalog(), 0), charging(catalog(), -5)] {
            let mut session = over(&catalog, 0);
            assert_eq!(
                session.land(),
                Ok(LandPress::Outcome(LandOutcome::Landed(StellarId(128))))
            );
            assert_eq!(session.pilot.cash, 0);
        }
    }

    #[test]
    fn entering_a_hypergate_charges_its_fee_and_a_cancelled_map_keeps_it() {
        let catalog = charging(gated(), FEE);
        let mut session = over(&catalog, 500);
        assert_eq!(
            session.land(),
            Ok(LandPress::AtGate {
                stellar: StellarId(300),
                kind: GateKind::Hypergate,
            })
        );
        assert_eq!(session.pilot.cash, 400);
        assert_eq!(
            session.enter_hypergate(None, &catalog, &mut NeverFires),
            Err(GateRefusal::Cancelled)
        );
        assert_eq!(session.pilot.cash, 400, "no refund (@0x66ee0-0x66f08)");
    }

    #[test]
    fn entering_a_wormhole_charges_its_fee() {
        let catalog = charging(wormholes(), FEE);
        let mut session = over(&catalog, 500);
        assert_eq!(
            session.land(),
            Ok(LandPress::AtGate {
                stellar: StellarId(400),
                kind: GateKind::Wormhole,
            })
        );
        assert_eq!(session.pilot.cash, 400);
    }

    #[test]
    fn a_disabled_ship_that_cannot_pay_hears_the_fee_refusal() {
        // The fee (@0x669b4) comes before `_IsDisabled` (@0x66af7).
        let catalog = charging(catalog(), FEE);
        let mut session = over(&catalog, 0);
        session.ship.condition = Condition::Disabled;
        assert_eq!(
            session.land(),
            Err(LandingRefusal::CannotPayFee {
                stellar: StellarId(128),
                station: false,
            })
        );
        assert_eq!(session.dock_request(), None, "dropped");
        assert_eq!(session.take_sounds(), []);
        assert_eq!(session.pilot.cash, 0);
    }

    #[test]
    fn the_first_press_is_refused_before_traffic_control_hears_it() {
        let catalog = charging(catalog(), FEE);
        let mut session = over(&catalog, 0);
        session.port.dock = None;
        assert!(matches!(
            session.land(),
            Err(LandingRefusal::CannotPayFee { .. })
        ));
        assert_eq!(session.take_sounds(), []);
        assert!(
            !session
                .take_messages()
                .iter()
                .any(|message| matches!(message, SimMessage::DockRequested { .. })),
            "no request"
        );
        assert_eq!(session.dock_request(), None);
    }
}
