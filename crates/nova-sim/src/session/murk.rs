//! What the murk asks of the session (see [`murk`]): the
//! player's effective murk in its system, which the view darkens shots by.

use super::Session;
use crate::murk;
use crate::rulebook::RuleKey;

impl Session {
    /// The player's effective murk, 0..=100 ([`murk::effective_murk`]):
    /// its system's `Murk` as loaded ([`murk::loaded_murk`], by the
    /// rulebook's [`RuleKey::LowMurk`] reading), plus what its outfits
    /// add ([`murk::outfit_murk`]), as `_PlayerEffectiveMurk`
    /// (@0x6f8f-0x704b) sums them.
    ///
    /// It is worked out afresh at every call, as
    /// [`Session::interference`] is. The original caches it in
    /// `_playerEffectiveMurk`; when that goes stale is
    /// rdm:task/reset-player-precalced-left-out-branches's question.
    #[must_use]
    pub fn effective_murk(&self) -> u8 {
        let raw = self.galaxy.map().murk(self.pilot.system);
        let loaded = murk::loaded_murk(raw, self.setup.rules.source_for(RuleKey::LowMurk));
        murk::effective_murk(
            loaded,
            murk::outfit_murk(&self.pilot.outfits, &self.statics.outfits),
        )
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

    /// System 130 of `Murk` `murk`, the others of none; outfit 300 adding
    /// 15 murk in two slots (10 and 5), 301 taking 40 away.
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
}
