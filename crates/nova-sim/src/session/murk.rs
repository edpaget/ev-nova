//! What the murk asks of the session (see [`murk`]): the
//! player's effective murk in its system, which the view murks sprites by,
//! the colour they murk toward, and its system's murk as loaded, which
//! the starfield reads.

use super::{Chart, Session, Setup, Statics};
use crate::murk;
use crate::pilot::Pilot;
use crate::rulebook::RuleKey;

impl Session {
    /// The player's effective murk, 0..=100 ([`murk::effective_murk`]):
    /// its system's `Murk` as loaded ([`murk::loaded_murk`], by the
    /// rulebook's [`RuleKey::LowMurk`] reading), plus what its outfits
    /// add ([`murk::outfit_murk`]), as `_PlayerEffectiveMurk`
    /// (@0x6f8f-0x704b) sums them.
    ///
    /// It is worked out afresh at every call, as
    /// the interference is (`Chart::interference`). The original caches it in
    /// `_playerEffectiveMurk`; when that goes stale is
    /// rdm:task/reset-player-precalced-left-out-branches's question.
    #[must_use]
    pub fn effective_murk(&self) -> u8 {
        self.chart
            .effective_murk(&self.pilot, &self.statics, &self.setup)
    }

    /// The RGB555 colour the player's system's sprites murk toward
    /// ([`murk::murk_colour`]): its `BkgndColor`, as `_LoadObjectData`
    /// builds it. A system off the map reads as a black `BkgndColor`.
    #[must_use]
    pub fn murk_colour(&self) -> u16 {
        murk::murk_colour(self.system_background().unwrap_or(0))
    }

    /// The player's system's `Murk` as loaded ([`murk::loaded_murk`], by
    /// the rulebook's [`RuleKey::LowMurk`] reading), with no outfit's:
    /// the runtime +0xbc that `_LoadObjectData` writes
    /// (@0x77eb7/0x77ece-0x77eff). −1 for a negative `Murk`, which
    /// clears the starfield (`_RandomizeStars` @0x0b168-0x0b18d), and
    /// otherwise 0..=100, which dims it (`_HandleStars` @0x2f8cc-0x2f913).
    #[must_use]
    pub(crate) fn system_murk(&self) -> i16 {
        self.chart.system_murk(&self.pilot, &self.setup)
    }
}

impl Chart {
    /// The player's effective murk, 0..=100 ([`murk::effective_murk`]):
    /// its system's `Murk` as loaded ([`murk::loaded_murk`], by the
    /// rulebook's [`RuleKey::LowMurk`] reading), plus what its outfits
    /// add ([`murk::outfit_murk`]), as `_PlayerEffectiveMurk`
    /// (@0x6f8f-0x704b) sums them.
    ///
    /// It is worked out afresh at every call, as
    /// the interference is (`Chart::interference`). The original caches it in
    /// `_playerEffectiveMurk`; when that goes stale is
    /// rdm:task/reset-player-precalced-left-out-branches's question.
    #[must_use]
    fn effective_murk(&self, pilot: &Pilot, statics: &Statics, setup: &Setup) -> u8 {
        let loaded = self.system_murk(pilot, setup);
        murk::effective_murk(loaded, murk::outfit_murk(&pilot.outfits, &statics.outfits))
    }

    /// The player's system's `Murk` as loaded (see
    /// [`Session::system_murk`]).
    fn system_murk(&self, pilot: &Pilot, setup: &Setup) -> i16 {
        let raw = self.galaxy.map().murk(pilot.system);
        murk::loaded_murk(raw, setup.rules.source_for(RuleKey::LowMurk))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{OutfitId, StarSystem, SystemId};
    use crate::murk::MOD_MURK;
    use crate::pilot::Pilot;
    use crate::rulebook::{RuleSource, Rulebook};
    use crate::testkit::{FakePilotCatalog, catalog, outfit};

    /// The `BkgndColor` of the murky fixture's system 130.
    const BACKGROUND: u32 = 0x0019_090f;

    /// System 130 of `Murk` `murk` and [`BACKGROUND`], the others of none
    /// and black; outfit 300 adding 15 murk in two slots (10 and 5), 301
    /// taking 40 away.
    fn murky(murk: i16) -> FakePilotCatalog {
        let base = catalog();
        FakePilotCatalog {
            outfits: vec![
                outfit(300, &[(MOD_MURK, 10), (1, 99), (MOD_MURK, 5)]),
                outfit(301, &[(MOD_MURK, -40)]),
            ],
            star_map: base
                .star_map
                .iter()
                .map(|system| StarSystem {
                    murk: if system.id == SystemId(130) { murk } else { 0 },
                    background: if system.id == SystemId(130) {
                        BACKGROUND
                    } else {
                        0
                    },
                    ..system.clone()
                })
                .collect(),
            ..base
        }
    }

    fn session(catalog: &FakePilotCatalog) -> Session {
        Session::fly(catalog, Pilot::new(catalog, "Ada").expect("starts")).expect("flies")
    }

    #[test]
    fn the_effective_murk_is_the_players_systems() {
        let session = session(&murky(40));
        assert_eq!(session.pilot.system, SystemId(130));
        assert_eq!(session.effective_murk(), 40);
    }

    #[test]
    fn the_murk_colour_is_the_players_systems() {
        let catalog = murky(40);
        let mut session = session(&catalog);
        assert_eq!(session.pilot.system, SystemId(130));
        assert_eq!(session.murk_colour(), murk::murk_colour(BACKGROUND));

        let other = catalog
            .star_map
            .iter()
            .find(|system| system.id != SystemId(130))
            .expect("a second system");
        session.pilot.system = other.id;
        assert_eq!(session.murk_colour(), murk::murk_colour(0), "black");

        session.pilot.system = SystemId(1999);
        assert_eq!(
            session.murk_colour(),
            murk::murk_colour(0),
            "off the map, as a black BkgndColor"
        );
    }

    #[test]
    fn an_outfit_adds_count_times_its_mod_val_over_its_slots() {
        let mut session = session(&murky(40));
        session.pilot.outfits.insert(OutfitId(300), 2);
        assert_eq!(session.effective_murk(), 70, "40 + 2 × (10 + 5)");
        session.pilot.outfits.insert(OutfitId(301), 1);
        assert_eq!(session.effective_murk(), 30);
        session.pilot.outfits.insert(OutfitId(301), 3);
        assert_eq!(session.effective_murk(), 0, "never below none");
    }

    #[test]
    fn the_effective_murk_is_clamped_at_100() {
        let mut murkiest = session(&murky(90));
        murkiest.pilot.outfits.insert(OutfitId(300), 1);
        assert_eq!(murkiest.effective_murk(), 100);
        assert_eq!(session(&murky(250)).effective_murk(), 100);
    }

    #[test]
    fn a_murk_of_5_is_none_by_the_engine_and_5_by_the_bible() {
        let catalog = murky(5);
        assert_eq!(session(&catalog).effective_murk(), 0);
        let bible = Rulebook::default().with_override(RuleKey::LowMurk, RuleSource::Bible);
        assert_eq!(session(&catalog).with_rules(bible).effective_murk(), 5);
    }

    #[test]
    fn a_negative_murk_counts_as_none_before_the_outfits() {
        let mut session = session(&murky(-20));
        assert_eq!(session.effective_murk(), 0);
        session.pilot.outfits.insert(OutfitId(300), 1);
        assert_eq!(session.effective_murk(), 15, "-1 floored at none, plus 15");
    }

    #[test]
    fn the_system_murk_is_the_players_systems_as_loaded_without_its_outfits() {
        let mut outfitted = session(&murky(40));
        outfitted.pilot.outfits.insert(OutfitId(300), 2);
        assert_eq!(outfitted.system_murk(), 40, "the outfits left out");
        assert_eq!(session(&murky(250)).system_murk(), 100);
        assert_eq!(session(&murky(-20)).system_murk(), -1, "kept negative");
        assert_eq!(session(&murky(0)).system_murk(), 0);
    }

    #[test]
    fn a_system_murk_of_5_is_none_by_the_engine_and_5_by_the_bible() {
        let catalog = murky(5);
        assert_eq!(session(&catalog).system_murk(), 0);
        let bible = Rulebook::default().with_override(RuleKey::LowMurk, RuleSource::Bible);
        assert_eq!(session(&catalog).with_rules(bible).system_murk(), 5);
    }
}
