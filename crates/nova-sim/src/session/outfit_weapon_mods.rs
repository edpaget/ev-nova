//! Which of an outfit's four mods make it a weapon (`ModType` 1) or
//! ammunition (`ModType` 3) across the session, as
//! [`RuleKey::OutfitWeaponMods`](crate::rulebook::RuleKey::OutfitWeaponMods)
//! reads it. All addresses are in `EV
//! Nova.app`'s `EV Nova` (i386).
//!
//! By the engine, the first mod alone. `_SystemInfoToShipStats` builds the
//! weapon array and the rounds array from each owned outfit's first
//! `ModType`/`ModVal` (@0xca8d, @0xcab1), and `_LoadObjectData` picks a
//! class's stock weapons' holders by it (@0x7aab8, @0x7ac29). So an outfit
//! naming a weapon or rounds only in a later mod mounts nothing, holds no
//! rounds and holds no stock weapon. By the other reading, any of the four.
//!
//! The session reads the key from its live rulebook (`setup.rules`), never
//! freezing it at load:
//!
//! - the refit mounts the armament by it
//!   ([`Arsenal::player`](crate::combat::armament::Arsenal::player)), and
//!   [`Session::with_rules`](super::Session::with_rules) refits, so the armament and the rounds never
//!   read the mods differently;
//! - every reader of the player's rounds (firing, the status bar's count,
//!   the secondary pick, a fighter stowed or put back, the escape ship,
//!   the bays' room and the control-bit facts) takes
//!   [`Statics::ammo_outfits`](super::statics::Statics::ammo_outfits) by
//!   it;
//! - a load's pending stock, a ship bought and the eject's pod pick their
//!   stock holders by it
//!   ([`Arsenal::stock_fits`](crate::combat::armament::Arsenal::stock_fits));
//! - the plunder Ammo press takes its outfit by it (see the
//!   `plunder_ammo` module).
//!
//! The original's all-slot scans read every mod under both readings,
//! through
//! [`Statics::named_ammo_outfits`](super::statics::Statics::named_ammo_outfits):
//! the plunder dialog's name for the rounds (`_PlunderDialogUpdate`
//! @0x92c59-0x92c81) and `_CanBuyFighter`'s bay outfit (@0x5b95-0x5bd8).
//! The outfitter's bay room (`_CanBuyOutfitItem` @0x4e8f6) reads the first
//! mod alone, so it is keyed.

#[cfg(test)]
mod tests {
    use crate::catalog::{OutfitId, StockWeapon, WeaponId};
    use crate::chance::NeverFires;
    use crate::combat::armament::{MOD_AMMO, MOD_WEAPON};
    use crate::flight::Controls;
    use crate::pilot::Pilot;
    use crate::rulebook::RuleKey;
    use crate::rulebook::{RuleSource, Rulebook};
    use crate::session::Session;
    use crate::session::fighters::tests::{VIPER, VIPERS, carrying, pilot};
    use crate::session::tests::{NEW, ROCKET, fired_by_the_player, shipbuying, with_secondaries};
    use crate::testkit::{FakePilotCatalog, catalog, land_now, outfit, work};

    /// Rockets' launcher (outfit 300) and their rounds (outfit 301) named
    /// only in a later mod, after a mod of no effect, below the first-mod
    /// outfits of [`with_secondaries`] (312 and 310).
    const LATER_LAUNCHER: OutfitId = OutfitId(300);
    const LATER_ROCKETS: OutfitId = OutfitId(301);
    const FIRST_LAUNCHER: OutfitId = OutfitId(312);

    fn later() -> FakePilotCatalog {
        let mut catalog = with_secondaries(catalog());
        catalog
            .outfits
            .push(outfit(LATER_LAUNCHER.0, &[(-1, 0), (MOD_WEAPON, ROCKET.0)]));
        catalog
            .outfits
            .push(outfit(LATER_ROCKETS.0, &[(-1, 0), (MOD_AMMO, ROCKET.0)]));
        catalog
    }

    fn rules(mods: RuleSource) -> Rulebook {
        Rulebook::default().with_override(RuleKey::OutfitWeaponMods, mods)
    }

    /// [`later`]'s pilot owning `owned` alone, loaded by `mods`.
    fn loaded(owned: &[(OutfitId, u16)], mods: RuleSource) -> Session {
        let catalog = later();
        let mut pilot = Pilot::new(&catalog, "").expect("starts");
        pilot.outfits = owned.iter().copied().collect();
        Session::load(&catalog, pilot, &rules(mods)).expect("loads")
    }

    /// The rockets the session's armament mounts.
    fn rockets_mounted(session: &Session) -> u32 {
        session
            .ship
            .fit
            .armament
            .mounts()
            .iter()
            .filter(|mount| mount.spec.id == ROCKET)
            .map(|mount| mount.count)
            .sum()
    }

    #[test]
    fn a_later_mod_launcher_mounts_nothing_by_the_engine() {
        let owned = [(LATER_LAUNCHER, 2)];
        assert_eq!(rockets_mounted(&loaded(&owned, RuleSource::Engine)), 0);
        assert_eq!(rockets_mounted(&loaded(&owned, RuleSource::Bible)), 2);
    }

    #[test]
    fn later_mod_rounds_are_none_of_the_players_by_the_engine() {
        let owned = [(FIRST_LAUNCHER, 1), (LATER_ROCKETS, 5)];
        let catalog = later();
        for (mods, count, fired) in [
            (RuleSource::Engine, 0, vec![]),
            (RuleSource::Bible, 5, vec![ROCKET]),
        ] {
            let mut session = loaded(&owned, mods);
            session.ship.secondary = Some(ROCKET);
            assert_eq!(session.secondary_line_count(), Some(count), "{mods:?}");
            session.hold_fire(false, true);
            work(&mut session, &catalog, Controls::default());
            assert_eq!(
                fired_by_the_player(&session.take_combat_events()),
                fired,
                "{mods:?}"
            );
        }
    }

    #[test]
    fn a_rule_given_after_the_load_remounts_the_armament_and_reads_the_rounds() {
        let owned = [(LATER_LAUNCHER, 1), (LATER_ROCKETS, 5)];
        let mut session = loaded(&owned, RuleSource::Engine);
        assert_eq!(rockets_mounted(&session), 0);
        session = session.with_rules(rules(RuleSource::Bible));
        assert_eq!(rockets_mounted(&session), 1, "no other refit between");
        session.ship.secondary = Some(ROCKET);
        assert_eq!(session.secondary_line_count(), Some(5));
        session = session.with_rules(Rulebook::default());
        assert_eq!(rockets_mounted(&session), 0, "and back");
    }

    #[test]
    fn a_loads_pending_stock_goes_to_the_first_mod_holder_by_the_engine() {
        let catalog = later();
        for (mods, holder) in [
            (RuleSource::Engine, FIRST_LAUNCHER),
            (RuleSource::Bible, LATER_LAUNCHER),
        ] {
            let mut pilot = Pilot::new(&catalog, "").expect("starts");
            pilot.outfits.clear();
            pilot.stock_weapons_pending = true;
            let session = Session::load(&catalog, pilot, &rules(mods)).expect("loads");
            assert_eq!(session.pilot().owned(holder), 1, "{mods:?}");
            assert_eq!(rockets_mounted(&session), 1, "{mods:?}");
        }
    }

    #[test]
    fn a_new_pilot_started_by_a_rulebook_takes_its_stock_by_it() {
        let catalog = later();
        for (mods, holder) in [
            (RuleSource::Engine, FIRST_LAUNCHER),
            (RuleSource::Bible, LATER_LAUNCHER),
        ] {
            let session = Session::start_ruled(&catalog, &rules(mods)).expect("starts");
            assert_eq!(session.pilot().owned(holder), 1, "{mods:?}");
        }
    }

    #[test]
    fn a_ship_bought_takes_its_stock_by_the_rulebook() {
        let mut catalog = shipbuying();
        catalog.weapons.push(crate::testkit::weapon(130));
        catalog
            .outfits
            .push(outfit(300, &[(-1, 0), (MOD_WEAPON, 130)]));
        catalog.outfits.push(outfit(310, &[(MOD_WEAPON, 130)]));
        catalog.hulls.retain(|hull| hull.id != NEW);
        catalog.hulls.push(crate::catalog::HullRecord {
            weapons: vec![StockWeapon {
                weapon: WeaponId(130),
                count: 1,
                ammo: 0,
            }],
            ..crate::testkit::hull(NEW.0)
        });
        for (mods, holder) in [(RuleSource::Engine, 310), (RuleSource::Bible, 300)] {
            let mut session = Session::start_ruled(&catalog, &rules(mods)).expect("starts");
            land_now(&mut session).expect("lands");
            session
                .buy_ship(NEW, "Kestrel", &mut NeverFires)
                .expect("bought");
            assert_eq!(session.pilot().owned(OutfitId(holder)), 1, "{mods:?}");
        }
    }

    #[test]
    fn a_fighter_stowed_goes_to_the_first_mod_outfit_by_the_engine() {
        let mut catalog = carrying();
        // 156 names the bay's fighters in its second mod alone, below 158.
        catalog
            .outfits
            .push(outfit(156, &[(-1, 0), (MOD_AMMO, 149)]));
        for (mods, holder) in [
            (RuleSource::Engine, VIPERS),
            (RuleSource::Bible, OutfitId(156)),
        ] {
            let mut session = Session::fly(&catalog, pilot(&catalog, 0, Vec::new()))
                .expect("flies")
                .with_rules(rules(mods));
            assert!(
                session.ship.stow_fighter(
                    &mut session.pilot,
                    &session.statics,
                    &session.setup,
                    VIPER,
                    VIPER
                ),
                "{mods:?}"
            );
            assert_eq!(session.pilot().owned(holder), 1, "{mods:?}");
        }
    }

    #[test]
    fn a_boarded_fighters_room_reads_the_bays_last_outfit_in_any_slot() {
        // `_CanBuyFighter`'s scan (@0x5b95-0x5bd8) takes the bay's last
        // outfit by ID among all four mods: 159, of `Max` 0, so a bay of
        // `MaxAmmo` 0 has no room, whatever the key says.
        let mut catalog = carrying();
        catalog.weapons[0].max_ammo = 0;
        let room = |catalog: &FakePilotCatalog, mods| {
            Session::fly(catalog, pilot(catalog, 0, Vec::new()))
                .expect("flies")
                .with_rules(rules(mods))
                .can_take_fighter(VIPER)
        };
        for mods in [RuleSource::Engine, RuleSource::Bible] {
            assert!(room(&catalog, mods), "{mods:?}: 158's Max of 9999");
        }
        catalog.outfits.push(crate::catalog::OutfitRecord {
            max: 0,
            ..outfit(159, &[(-1, 0), (MOD_AMMO, 149)])
        });
        for mods in [RuleSource::Engine, RuleSource::Bible] {
            assert!(!room(&catalog, mods), "{mods:?}: 159's Max of 0");
        }
    }
}
