//! The plunder dialog's Ammo press, as `_DoPlunderDialog`
//! (@0x93cbd-0x93dd0) runs it. All addresses are in `EV Nova.app`'s
//! `EV Nova` (i386).
//!
//! **The outfit.** The press resets the session (`_ShipStatsToSystemInfo`
//! @0x93cc9, see the `precalced` module), then finds the outfit to take
//! the offered rounds as, afresh: by the engine it scans the 512 records
//! in order for the first whose *first* mod is `ModType` 3 naming the
//! rounds (@0x93cdc-0x93d08), and by the other reading of
//! [`RuleKey::PlunderAmmoOutfit`] the first with such a mod in any slot
//! ([`board::ammo_outfit`](crate::board::ammo_outfit)). When none
//! matches, the press beeps [`SimSound::Refused`](crate::sound::SimSound::Refused)
//! (`snd ` 153, @0x93d0a-0x93d2f) and takes nothing: the offer stays, as
//! only the take path clears it (@0x94000-0x9400a), the self-destruct the
//! press disarmed is not armed again, and the line counts down one tick
//! (see the `plunder_loop` module). The offer itself, and the name the
//! dialog shows for the rounds, read any slot whatever the key (see
//! [`board`](crate::board)).
//!
//! **The take** ([`Session::take_ammo`]). Each round is taken while all
//! three hold, read again for each round:
//!
//! - the free mass, clamped at none as `_ShipFreeMass` clamps it
//!   (@0x93d62, `testl`/`jns`/`xorl` @0xb506-0xb50a), is at least the
//!   outfit's raw `Mass` (`cmpl`/`jl` @0x93d74-0x93d76), not scaled by its
//!   `Flags` 0x0400. So a ship over its free mass still takes rounds of
//!   `Mass` 0 or less;
//! - the rounds taken are below the rounds offered (@0x93d84);
//! - the outfit is not at its maximum (`_HasMaxOfItem` @0x93d9f, see
//!   [`has_max`](crate::has_max)): its ammunition cap, `MaxAmmo` times the
//!   ship's launchers of its weapon, stock weapons included, its `Max` as
//!   `ModType` 27 raises it, and for a plug-in's gun or turret the ship's
//!   gun and turret limits.
//!
//! Each round adds one to the outfit owned and resets the session
//! (@0x93dcb); the ship is refitted once after. `_ShipStatsToSystemInfo`
//! gives the remainder of the rounds to the lowest-index outfit whose
//! first mod names them (@0xcc76-0xccc0), the outfit the scan found, so
//! adding to that outfit matches.

use super::Session;
use crate::catalog::{OutfitId, WeaponId};
use crate::rulebook::RuleKey;

impl Session {
    /// The outfit a boarding's Ammo press takes the rounds of weapon
    /// `slot` as, as [`RuleKey::PlunderAmmoOutfit`] reads it (see the
    /// module docs): none when none matches.
    pub(super) fn ammo_outfit(&self, slot: WeaponId) -> Option<OutfitId> {
        crate::board::ammo_outfit(
            &self.statics.outfits,
            &self.statics.ammo_outfits,
            slot,
            self.setup.rules.source_for(RuleKey::PlunderAmmoOutfit),
        )
    }

    /// Takes up to `rounds` of ammunition outfit `outfit`, one at a time
    /// while the free mass clamped at none covers its raw `Mass` and it is
    /// not at its maximum (see the module docs), and gives how many; the
    /// ship is refitted with them.
    pub(super) fn take_ammo(&mut self, outfit: OutfitId, rounds: u32) -> u32 {
        let Some(record) = self
            .statics
            .outfits
            .iter()
            .find(|record| record.id == outfit)
            .cloned()
        else {
            return 0;
        };
        let unit = i64::from(record.mass);
        let mut count = 0;
        while self
            .ship
            .free_mass(&self.pilot, &self.statics, &self.setup)
            .max(0)
            >= unit
            && count < rounds
            && self
                .ship
                .at_max(&self.pilot, &self.statics, &self.setup, &record)
                .is_none()
        {
            *self.pilot.outfits.entry(outfit).or_default() += 1;
            count += 1;
            // The reset after each round (@0x93dcb).
            self.reset_precalced();
        }
        self.ship.refit(
            &mut self.pilot,
            &self.statics,
            &self.setup,
            true,
            self.setup.script_reserves(),
        );
        count
    }
}

#[cfg(test)]
mod tests {
    use crate::board::{AmmoOffer, Boarding, Take, Taken};
    use crate::catalog::{OutfitId, OutfitRecord, ShipId};
    use crate::combat::armament::MOD_AMMO;
    use crate::comm_line::CommLine;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::session::Session;
    use crate::session::tests::{
        ROCKET, ROCKETS_OFFERED, ammo_aboard, ammo_boarding, ammo_session, take,
    };
    use crate::sound::SimSound;
    use crate::testkit::{FakePilotCatalog, outfit};

    /// The record of `id` in `catalog`.
    fn record(catalog: &mut FakePilotCatalog, id: i16) -> &mut OutfitRecord {
        catalog
            .outfits
            .iter_mut()
            .find(|record| record.id == OutfitId(id))
            .expect("the outfit")
    }

    /// The ammunition offered, the dialog open.
    fn offered(session: &Session) -> Option<AmmoOffer> {
        session.boarding().expect("the dialog open").ammo
    }

    #[test]
    fn an_overweight_ship_takes_rounds_of_mass_0_or_less() {
        // `_ShipFreeMass` clamps at none (@0xb506-0xb50a): 0 >= 0.
        for (mass, count) in [(0, 9), (-1, 9), (1, 0)] {
            let mut catalog = ammo_aboard(mass, 20);
            catalog.outfits.push(OutfitRecord {
                mass: 40,
                ..outfit(330, &[])
            });
            let mut session = ammo_session(&catalog);
            session.pilot.outfits.insert(OutfitId(330), 1);
            assert!(
                session
                    .ship
                    .free_mass(&session.pilot, &session.statics, &session.setup)
                    < 0,
                "{mass}"
            );
            assert_eq!(
                take(&mut session, Take::Ammo, &[]).0,
                Taken::Ammo {
                    outfit: OutfitId(310),
                    count
                },
                "{mass}"
            );
        }
    }

    #[test]
    fn taking_ammo_stops_at_max_ammo_times_the_launchers() {
        for (launchers, count) in [(1, 3), (2, 6)] {
            let mut catalog = ammo_aboard(1, 20);
            for weapon in &mut catalog.weapons {
                if weapon.id == ROCKET {
                    weapon.max_ammo = 3;
                }
            }
            for hull in &mut catalog.hulls {
                if hull.id == ShipId(128) {
                    for stock in &mut hull.weapons {
                        if stock.weapon == ROCKET {
                            stock.count = launchers;
                        }
                    }
                }
            }
            let mut session = ammo_session(&catalog);
            assert_eq!(
                take(&mut session, Take::Ammo, &[]).0,
                Taken::Ammo {
                    outfit: OutfitId(310),
                    count
                },
                "{launchers}"
            );
        }
    }

    /// [`ammo_aboard`] with outfit 310 naming the rockets in its second
    /// mod, after a mod of no effect.
    fn named_in_a_later_slot() -> FakePilotCatalog {
        let mut catalog = ammo_aboard(1, 20);
        record(&mut catalog, 310).mods = [(-1, 0), (MOD_AMMO, ROCKET.0), (0, 0), (0, 0)];
        catalog
    }

    /// The comm line shown for 10 ticks, counted down one.
    fn a_tick_down() -> CommLine {
        let mut line = CommLine::default();
        line.show(10);
        line.tick();
        line
    }

    #[test]
    fn an_ammo_press_finds_its_outfit_by_the_first_mod_alone() {
        let catalog = named_in_a_later_slot();
        let mut session = ammo_session(&catalog);
        session.take_sounds();
        session.comm_shown(10);
        assert_eq!(take(&mut session, Take::Ammo, &[]).0, Taken::Missed);
        assert_eq!(session.take_sounds(), [SimSound::Refused]);
        assert_eq!(session.pilot().owned(OutfitId(310)), 0);
        assert_eq!(offered(&session), Some(ROCKETS_OFFERED), "kept");
        assert_eq!(session.comm_line(), a_tick_down());
        assert_eq!(
            take(&mut session, Take::Miss, &[]).1,
            Vec::<u32>::new(),
            "no self-destruct armed"
        );
        let bible =
            Rulebook::default().with_override(RuleKey::PlunderAmmoOutfit, RuleSource::Bible);
        let mut session = ammo_session(&catalog).with_rules(bible);
        assert_eq!(
            take(&mut session, Take::Ammo, &[]).0,
            Taken::Ammo {
                outfit: OutfitId(310),
                count: 9
            }
        );
        let mut both = named_in_a_later_slot();
        both.outfits.push(outfit(312, &[(MOD_AMMO, ROCKET.0)]));
        for (rules, outfit) in [(Rulebook::default(), 312), (bible, 310)] {
            let mut session = ammo_session(&both).with_rules(rules);
            assert_eq!(
                take(&mut session, Take::Ammo, &[]).0,
                Taken::Ammo {
                    outfit: OutfitId(outfit),
                    count: 9
                },
                "{outfit}"
            );
        }
    }

    #[test]
    fn rounds_no_outfit_names_are_offered_and_their_press_refused() {
        let mut catalog = ammo_aboard(1, 20);
        catalog.outfits.retain(|record| record.id != OutfitId(310));
        let unnamed = AmmoOffer {
            named: None,
            ..ROCKETS_OFFERED
        };
        for source in [RuleSource::Engine, RuleSource::Bible] {
            let (session, opened) = ammo_boarding(&catalog);
            let Ok(Boarding::Opened(view)) = opened else {
                panic!("{opened:?}");
            };
            assert_eq!(view.ammo, Some(unnamed), "{source:?}");
            let mut session = session
                .with_rules(Rulebook::default().with_override(RuleKey::PlunderAmmoOutfit, source));
            session.take_sounds();
            assert_eq!(take(&mut session, Take::Ammo, &[]).0, Taken::Missed);
            assert_eq!(session.take_sounds(), [SimSound::Refused], "{source:?}");
            assert_eq!(offered(&session), Some(unnamed), "{source:?}");
        }
    }
}
