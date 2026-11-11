//! What a passage through a hypergate costs the player in fuel, by
//! [`RuleKey::GateFuel`] (the rule's own home is [`gate_fuel`]).
//!
//! By the engine reading (the default) nothing: the original's
//! `_PlayerEnterHypergate` @0x637bf-0x64005 never reads or writes the
//! ship's fuel (+0x38), and `_HandlePlayerDockRequest` @0x66691-0x67efa
//! reads +0x38 only of the stellar, its `Fee` (@0x669b4, @0x66dce).
//!
//! By the other reading, the new game's:
//!
//! - **The refusal.** The land key's press that would enter a hypergate
//!   ([`Session::gate_fuel_refusal`]), the one the dock request answers
//!   `Landed` for, is refused while the ship holds less fuel than its
//!   hull's fare ([`gate_fuel::affords`], inclusive), with
//!   [`LandingRefusal::GateFuelShort`], which beeps
//!   [`SimSound::Refused`](crate::sound::SimSound::Refused) as J short of
//!   fuel does (`_HandlePlayer` @0x6ba72). It comes before the gate's fee,
//!   so a refused pilot pays none and no entry is pending. The earlier
//!   presses (the clearance request, too far, too fast) are the engine's:
//!   the ship flies on, its selection kept and its dock request as the
//!   press left it. The gate autopilot's own press is refused finally, so
//!   it lets go (see the `gate_pilot` module); J still engages it, as the
//!   refusal belongs to the gate, as the fee's does.
//! - **The spend.** An entry that exits pays the fare, out of the hull
//!   that entered, just before the ship comes out
//!   ([`PlayerShip::pay_gate_fuel`]); an entry refused (`Cancelled`,
//!   `NoLinks`) pays nothing. The press checked the fuel and no tick runs
//!   between the press and the entry, so the floor at none is only a
//!   guard. The spend is silent.
//!
//! A wormhole costs nothing by either reading (see the rule's doc).

use super::{PlayerShip, Session, Setup};
use crate::catalog::StellarId;
use crate::gate::GateKind;
use crate::gate_fuel;
use crate::landing::LandingRefusal;
use crate::pilot::Pilot;
use crate::rulebook::{RuleKey, RuleSource};

impl Session {
    /// The refusal of the land key's press that would enter `stellar`, if
    /// it is a hypergate the ship can't pay its passage through (see the
    /// module docs): `Ok` for any other stellar, and always by the engine
    /// reading.
    pub(super) fn gate_fuel_refusal(&self, stellar: StellarId) -> Result<(), LandingRefusal> {
        let pays = self.gate_kind(stellar) != Some(GateKind::Hypergate)
            || gate_fuel::affords(
                self.pilot.reserves.fuel.now,
                &self.ship.fit.stats,
                self.setup.rules.source_for(RuleKey::GateFuel),
            );
        if pays {
            Ok(())
        } else {
            Err(LandingRefusal::GateFuelShort { stellar })
        }
    }
}

impl PlayerShip {
    /// Pays a hypergate passage's fare out of `pilot`'s fuel by `setup`'s
    /// reading of [`RuleKey::GateFuel`]: this hull's
    /// [`fare`](gate_fuel::fare) by the other reading, none below none;
    /// nothing by the engine's.
    pub(super) fn pay_gate_fuel(&self, pilot: &mut Pilot, setup: &Setup) {
        if setup.rules.source_for(RuleKey::GateFuel) == RuleSource::Bible {
            let fuel = &mut pilot.reserves.fuel;
            fuel.now = (fuel.now - gate_fuel::fare(&self.fit.stats)).max(0.0);
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::{ShipId, SystemId};
    use crate::chance::NeverFires;
    use crate::gate::GateRefusal;
    use crate::handling::ShipFields;
    use crate::landing::LandOutcome;
    use crate::rulebook::Rulebook;
    use crate::session::LandPress;
    use crate::session::tests::gated;
    use crate::sound::SimSound;
    use crate::testkit::{FAST, FakePilotCatalog};

    /// The hull's gate fuel in these tests.
    const FARE: i16 = 150;

    /// [`gated`], the player's hull's gate fuel [`FARE`].
    fn fared() -> FakePilotCatalog {
        FakePilotCatalog {
            ships: vec![(
                ShipId(128),
                Ok(ShipFields {
                    gate_fuel: Some(FARE),
                    ..FAST
                }),
            )],
            ..gated()
        }
    }

    /// `catalog`'s session over hypergate 300 by `reading` of
    /// [`RuleKey::GateFuel`], holding `fuel`, L pressed once and the dock
    /// approach run, its sounds taken: the next L is the one that would
    /// enter the gate.
    fn cleared(catalog: &FakePilotCatalog, reading: RuleSource, fuel: f32) -> Session {
        cleared_holding(catalog, reading, fuel, None)
    }

    /// [`cleared`], the pilot holding `cash` when given.
    fn cleared_holding(
        catalog: &FakePilotCatalog,
        reading: RuleSource,
        fuel: f32,
        cash: Option<i64>,
    ) -> Session {
        let rules = Rulebook::default().with_override(RuleKey::GateFuel, reading);
        let mut session = Session::start(catalog).expect("starts").with_rules(rules);
        session.pilot.reserves.fuel.now = fuel;
        if let Some(cash) = cash {
            session.pilot.cash = cash;
        }
        assert!(matches!(
            session.land(),
            Ok(LandPress::Outcome(LandOutcome::Requested { .. }))
        ));
        session.dock_approach();
        session.take_sounds();
        session
    }

    const AT_GATE: Result<LandPress, LandingRefusal> = Ok(LandPress::AtGate {
        stellar: StellarId(300),
        kind: GateKind::Hypergate,
    });

    #[test]
    fn a_passage_leaves_the_fuel_by_the_engine_and_takes_the_hulls_fare_by_the_other() {
        let catalog = fared();
        for (reading, left) in [
            (RuleSource::Engine, 300.0),
            (RuleSource::Bible, 300.0 - f32::from(FARE)),
        ] {
            let mut session = cleared(&catalog, reading, 300.0);
            assert_eq!(session.land(), AT_GATE, "{reading:?}");
            assert_eq!(session.reserves().fuel.now, 300.0, "the press pays nothing");
            assert_eq!(
                session.enter_hypergate(Some(SystemId(131)), &catalog, &mut NeverFires),
                Ok(SystemId(131))
            );
            assert_eq!(session.reserves().fuel.now, left, "{reading:?}");
            assert_eq!(session.reserves().fuel.max, 300.0, "{reading:?}");
        }
    }

    #[test]
    fn a_hull_with_no_gate_fuel_pays_one_jumps_worth() {
        let catalog = gated();
        let mut session = cleared(&catalog, RuleSource::Bible, 300.0);
        assert_eq!(session.land(), AT_GATE);
        assert_eq!(
            session.enter_hypergate(Some(SystemId(131)), &catalog, &mut NeverFires),
            Ok(SystemId(131))
        );
        assert_eq!(
            session.reserves().fuel.now,
            300.0 - crate::hyperspace::JUMP_FUEL
        );
    }

    #[test]
    fn an_entry_refused_spends_nothing() {
        let catalog = fared();
        let mut session = cleared(&catalog, RuleSource::Bible, 300.0);
        assert_eq!(session.land(), AT_GATE);
        assert_eq!(
            session.enter_hypergate(None, &catalog, &mut NeverFires),
            Err(GateRefusal::Cancelled)
        );
        assert_eq!(session.reserves().fuel.now, 300.0, "cancelled");
        let mut linkless = fared();
        linkless.gates[0].links = [None; 8];
        let mut session = cleared(&linkless, RuleSource::Bible, 300.0);
        assert_eq!(session.land(), AT_GATE);
        assert_eq!(
            session.enter_hypergate(Some(SystemId(131)), &linkless, &mut NeverFires),
            Err(GateRefusal::NoLinks)
        );
        assert_eq!(session.reserves().fuel.now, 300.0, "no links");
        assert_eq!(session.system(), SystemId(130));
    }

    #[test]
    fn short_of_the_fare_the_press_that_would_enter_is_refused_with_a_beep() {
        let catalog = fared();
        let mut session = cleared(&catalog, RuleSource::Bible, f32::from(FARE) - 1.0);
        let selected = session.nav_mode();
        assert_eq!(
            session.land(),
            Err(LandingRefusal::GateFuelShort {
                stellar: StellarId(300)
            })
        );
        let sounds = session.take_sounds();
        assert_eq!(
            sounds
                .iter()
                .filter(|&&sound| sound == SimSound::Refused)
                .count(),
            1,
            "{sounds:?}"
        );
        assert_eq!(
            session.open_hypergate(&catalog),
            Err(GateRefusal::NotAtGate),
            "no entry pending"
        );
        assert_eq!(session.system(), SystemId(130));
        assert_eq!(session.nav_mode(), selected, "the selection kept");
        assert_eq!(session.reserves().fuel.now, f32::from(FARE) - 1.0);
    }

    #[test]
    fn with_the_fare_or_by_the_engine_the_press_enters() {
        let catalog = fared();
        let mut session = cleared(&catalog, RuleSource::Bible, f32::from(FARE));
        assert_eq!(session.land(), AT_GATE, "the fare exactly");
        let mut session = cleared(&catalog, RuleSource::Engine, f32::from(FARE) - 1.0);
        assert_eq!(session.land(), AT_GATE, "the engine asks for none");
        assert!(!session.take_sounds().contains(&SimSound::Refused));
    }

    #[test]
    fn a_pilot_refused_for_fuel_pays_no_fee_and_one_entering_pays_it() {
        let mut catalog = fared();
        catalog.sites[0].1[0].fee = 100;
        let short = f32::from(FARE) - 1.0;
        let mut short = cleared_holding(&catalog, RuleSource::Bible, short, Some(500));
        assert_eq!(
            short.land(),
            Err(LandingRefusal::GateFuelShort {
                stellar: StellarId(300)
            })
        );
        assert_eq!(short.pilot.cash, 500, "no fee");
        let fare = f32::from(FARE);
        let mut paying = cleared_holding(&catalog, RuleSource::Bible, fare, Some(500));
        assert_eq!(paying.land(), AT_GATE);
        assert_eq!(paying.pilot.cash, 400, "the fee, at the entering press");
    }

    #[test]
    fn a_plain_stellar_is_never_refused_for_gate_fuel() {
        let session = cleared(&fared(), RuleSource::Bible, 0.0);
        assert_eq!(session.gate_fuel_refusal(StellarId(140)), Ok(()));
        assert_eq!(
            session.gate_fuel_refusal(StellarId(300)),
            Err(LandingRefusal::GateFuelShort {
                stellar: StellarId(300)
            })
        );
    }
}
