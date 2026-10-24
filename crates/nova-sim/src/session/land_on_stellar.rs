//! The parts of `_PlayerLandOnStellar` (@0x631c4) that fill the ship and
//! beep, and `_LoadPilotData`'s matching fill.
//!
//! `_PlayerLandOnStellar` runs the whole stay: the landing's setup, the
//! port dialog (`_DoPortDialog` @0x63381), then the take-off. After the
//! port dialog returns:
//!
//! - At an uninhabited stellar (`spöb` `Flags` 0x0020, tested on the
//!   runtime +0x30 @0x632be) it beeps once, `PlaySound(_beepSnd[1], 1, …)`
//!   @0x632ef, `snd ` 151: [`SimSound::Landed`](crate::sound::SimSound::Landed),
//!   which [`landing_beeps`] decides. At an inhabited one it runs
//!   `_DoAutoRefuel` instead (@0x632c5), which is silent and is left out
//!   here (filed as play-feel-2 phase 20, `landing-auto-refuel`). The
//!   touchdown path makes no other sound: the clearance beeps are on
//!   earlier presses and frames (see the `clearance` module). The engine
//!   raises the beep as it docks, as the stay itself is silent.
//! - After the take-off's placement (@0x633a5-0x63417) it sets the shield
//!   (+0x54) to `_ShipShieldCapacity` and the armour (+0x58) to
//!   `_ShipArmorCapacity` (@0x6341e-0x6343d), before the take-off's day
//!   (`_IncrementGameTime` @0x63440):
//!   [`PlayerShip::fill_shield_and_armour`]. The fuel, ionization and
//!   condition are untouched. So the shield and armour stay as they were
//!   for the whole stay, and are full from the first frame of flight and
//!   in the take-off's save (@0x6354b).
//!
//! `_LoadPilotData` fills both the same way after its outfit loop
//! (@0x754d1-0x754f4), then reads the fuel from the save (@0x75517):
//! `Session::load` fills them after its refit, the fuel kept as saved.

use super::PlayerShip;
use crate::catalog::LandingSite;
use crate::landing::StellarFlags;
use crate::pilot::Pilot;

impl PlayerShip {
    /// Sets the pilot's shield and armour to the ship's capacities, a
    /// surplus over them included, as `_PlayerLandOnStellar`
    /// (@0x6341e-0x6343d) and `_LoadPilotData` (@0x754d1-0x754f4) assign
    /// them. The fuel and each gauge's most are left alone.
    pub(super) fn fill_shield_and_armour(&self, pilot: &mut Pilot) {
        let stats = self.fit.stats;
        pilot.reserves.shield.now = stats.shield;
        pilot.reserves.armor.now = stats.armor;
    }
}

/// Whether landing at `site` beeps (`_beepSnd[1]`, @0x632be-0x632ef): only
/// at an uninhabited stellar; no site, no beep.
pub(super) fn landing_beeps(site: Option<&LandingSite>) -> bool {
    site.is_some_and(|site| site.flags & StellarFlags::UNINHABITED != 0)
}

#[cfg(test)]
#[allow(clippy::float_cmp)] // the fill assigns, so the gauges match exactly
mod tests {
    use super::*;
    use crate::catalog::{OutfitId, StellarId};
    use crate::chance::NeverFires;
    use crate::session::Session;
    use crate::session::tests::{buy, outfitted, outfitting};
    use crate::sound::SimSound;
    use crate::testkit::{catalog, land_now};

    fn site(flags: u32) -> LandingSite {
        LandingSite {
            flags,
            ..crate::testkit::bar()
        }
    }

    #[test]
    fn only_an_uninhabited_stellar_beeps() {
        assert!(!landing_beeps(Some(&site(StellarFlags::CAN_LAND))));
        assert!(landing_beeps(Some(&site(
            StellarFlags::CAN_LAND | StellarFlags::UNINHABITED
        ))));
        assert!(!landing_beeps(None));
    }

    #[test]
    fn the_fill_sets_shield_and_armour_to_their_capacities_and_keeps_the_fuel() {
        let session = Session::start(&catalog()).expect("starts");
        let stats = session.ship.fit.stats;
        for (shield, armor) in [(10.0, 5.0), (stats.shield + 7.0, stats.armor + 3.0)] {
            let mut pilot = session.pilot.clone();
            pilot.reserves.shield.now = shield;
            pilot.reserves.armor.now = armor;
            pilot.reserves.fuel.now = 42.0;
            let before = pilot.reserves;
            session.ship.fill_shield_and_armour(&mut pilot);
            assert_eq!(pilot.reserves.shield.now, stats.shield);
            assert_eq!(pilot.reserves.armor.now, stats.armor);
            assert_eq!(pilot.reserves.fuel, before.fuel);
            assert_eq!(pilot.reserves.shield.max, before.shield.max);
            assert_eq!(pilot.reserves.armor.max, before.armor.max);
        }
    }

    /// A session at the start, its shield and armour damaged and its fuel
    /// down.
    fn damaged() -> Session {
        let mut session = Session::start(&catalog()).expect("starts");
        damage(&mut session);
        session
    }

    fn damage(session: &mut Session) {
        session.pilot.reserves.shield.now = 1.0;
        session.pilot.reserves.armor.now = 2.0;
        session.pilot.reserves.fuel.now = 50.0;
    }

    #[test]
    fn the_stay_keeps_the_damage_and_the_take_off_fills_shield_and_armour() {
        let mut session = damaged();
        land_now(&mut session).expect("lands");
        assert_eq!(session.reserves().shield.now, 1.0, "docked: as landed");
        assert_eq!(session.reserves().armor.now, 2.0, "docked: as landed");
        let fuel = session.reserves().fuel;
        session
            .take_off(&catalog(), &mut NeverFires)
            .expect("took off");
        let stats = session.stats();
        assert_eq!(session.reserves().shield.now, stats.shield);
        assert_eq!(session.reserves().armor.now, stats.armor);
        assert_eq!(session.reserves().fuel, fuel, "the fuel untouched");
    }

    #[test]
    fn the_take_off_fills_to_a_capacity_raised_in_the_port() {
        let catalog = outfitting();
        let mut session = outfitted(&catalog);
        let before = session.stats().shield;
        session
            .outfit(buy(OutfitId(302)), &mut NeverFires)
            .expect("bought");
        assert!(session.stats().shield > before);
        damage(&mut session);
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("took off");
        assert_eq!(session.reserves().shield.now, session.stats().shield);
        assert_eq!(session.reserves().armor.now, session.stats().armor);
    }

    #[test]
    fn a_loaded_pilot_resumes_with_shield_and_armour_full_and_its_fuel_as_saved() {
        let session = damaged();
        let pilot = session.pilot().clone();
        let resumed = Session::fly(&catalog(), pilot).expect("flies");
        let stats = resumed.stats();
        assert_eq!(resumed.reserves().shield.now, stats.shield);
        assert_eq!(resumed.reserves().armor.now, stats.armor);
        assert_eq!(resumed.reserves().fuel.now, 50.0, "the fuel as saved");
    }

    #[test]
    fn landing_at_an_inhabited_stellar_is_silent() {
        let mut session = Session::start(&catalog()).expect("starts");
        let _ = session.take_sounds();
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        assert!(
            !session.take_sounds().contains(&SimSound::Landed),
            "_DoAutoRefuel's branch: no beep"
        );
    }

    #[test]
    fn landing_at_an_uninhabited_stellar_beeps_once() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.chart.sites[0].flags |= StellarFlags::UNINHABITED;
        let _ = session.take_sounds();
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        let sounds = session.take_sounds();
        assert_eq!(
            sounds
                .iter()
                .filter(|sound| **sound == SimSound::Landed)
                .count(),
            1,
            "{sounds:?}"
        );
    }

    #[test]
    fn a_requested_landing_at_an_inhabited_stellar_beeps_traffic_control_once() {
        let mut session = Session::start(&catalog()).expect("starts");
        let _ = session.take_sounds();
        let mut sounds = Vec::new();
        for press in 0..3 {
            if press > 0 {
                session.dock_approach();
            }
            let landed = session.land();
            sounds.extend(session.take_sounds());
            if session.landed().is_some() {
                break;
            }
            landed.expect("requests");
        }
        assert_eq!(session.landed(), Some(StellarId(128)));
        let count = |wanted: SimSound| sounds.iter().filter(|sound| **sound == wanted).count();
        assert_eq!(count(SimSound::TrafficControl), 1, "{sounds:?}");
        assert_eq!(count(SimSound::Landed), 0, "{sounds:?}");
    }
}
