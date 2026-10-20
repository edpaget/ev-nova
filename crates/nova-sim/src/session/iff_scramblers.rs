//! The IFF scramblers in a session (`oütf` `ModType` 48): the governments
//! an owned one marks, their byte +0x83 (`_ResetPlayerPrecalcedValues`
//! @0xc69c-0xc6f1 in the `EV Nova` executable), and what the session hands
//! the AI of the player's ship for target selection.
//!
//! **The marks.** Each reset ([`Session::reset_precalced`]) marks every
//! government whose `Class1-4` holds the `ModVal` of an owned `ModType` 48
//! outfit; a `ModVal` of -1 marks none (@0xc6ce). By the engine's reading
//! of [`RuleKey::IffScrambler`] a mark is never cleared, as only
//! `_InitObjects` clears the byte (@0x1d00d), so it outlives the outfit's
//! sale, as the `ModType` 44 inhibitor does (see the `reinforcements`
//! module); by the other it is worked out afresh at each reset. Nothing is
//! saved: the original keeps the byte for the application's life and
//! never saves it.
//!
//! **Where it is read.** The traffic's [`World`](crate::traffic::World)
//! carries the marks into the AI's
//! [`Surroundings::iff_marked`](crate::ai::Surroundings::iff_marked): a
//! marked government's warship takes no player candidate from its hunt
//! (`_SelectWarshipTarget` @0x8aa13, see `nova_sim::ai::target`). The
//! other readers (`_AIDoesShipLikePlayer` @0x82424, `_IsThreatToStellar`
//! @0xa062, `_DoPlanetCommDialog` @0x96c4d) are rdm:task/iff-scrambler-other-readers's
//! and rdm:phase/stellar-fidelity/phase-13-stellar-defence-guns's.
//!
//! **The player's ship.** [`Session::player_side`] gives the AI its ship
//! type, so a xenophobe passes over the escape pod (class 0x2ff), and the
//! government it fights as (class +0x9f4, [`ShipRecord::combat_govt`](crate::catalog::ShipRecord::combat_govt)),
//! which the enemy-class roll and the player's friend strength read.

use super::Session;
use crate::catalog::{GovtId, outfit_mods};
use crate::rulebook::{RuleKey, RuleSource};

/// An owned outfit's `ModType` that marks governments as the IFF
/// scrambler (0x30 @0xc69c).
pub const IFF_SCRAMBLER: i16 = 48;

/// The scrambler's `ModVal` that marks no government: no class is -1
/// (@0xc6ce).
pub const NO_CLASS: i16 = -1;

impl Session {
    /// The reset's IFF marks (@0xc69c-0xc6f1): set by each owned
    /// `ModType` 48 outfit, on top of those set before by the engine's
    /// reading of [`RuleKey::IffScrambler`], in place of them by the other.
    pub(super) fn reset_iff_scramblers(&mut self) {
        if self.setup.rules.source_for(RuleKey::IffScrambler) == RuleSource::Bible {
            self.space.iff_marked.clear();
        }
        for owned in outfit_mods(&self.pilot.outfits, &self.statics.outfits) {
            if owned.mod_type != IFF_SCRAMBLER || owned.mod_val == NO_CLASS {
                continue;
            }
            let classed: Vec<GovtId> = self
                .statics
                .govts
                .ids()
                .filter(|&govt| {
                    self.statics
                        .govts
                        .get(govt)
                        .is_some_and(|record| record.classes.contains(&owned.mod_val))
                })
                .collect();
            self.space.iff_marked.extend(classed);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::catalog::{GovtRecord, OutfitId, ShipId};
    use crate::pilot::Pilot;
    use crate::rulebook::Rulebook;
    use crate::testkit::{FAST, FakePilotCatalog, catalog, govt, outfit, ship};

    /// A scrambler of class 1.
    const CLASS_1: OutfitId = OutfitId(442);
    /// A scrambler of `ModVal` -1.
    const NO_CLASS_SCRAMBLER: OutfitId = OutfitId(444);

    /// Governments 128 and 130 of class 1 and 129 of class 2, the
    /// scramblers, and ship 128 fighting as government 129.
    fn scrambled() -> FakePilotCatalog {
        FakePilotCatalog {
            ship_records: vec![crate::catalog::ShipRecord {
                combat_govt: Some(GovtId(129)),
                ..ship(128, FAST)
            }],
            govts: vec![
                GovtRecord {
                    classes: [1, -1, -1, -1],
                    ..govt(128)
                },
                GovtRecord {
                    classes: [2, -1, -1, -1],
                    ..govt(129)
                },
                GovtRecord {
                    classes: [4, 1, -1, -1],
                    ..govt(130)
                },
            ],
            outfits: vec![
                outfit(442, &[(IFF_SCRAMBLER, 1)]),
                outfit(444, &[(IFF_SCRAMBLER, NO_CLASS)]),
                outfit(445, &[(44, 2)]),
            ],
            ..catalog()
        }
    }

    /// A flight of `catalog` by `rules`.
    fn flying(catalog: &FakePilotCatalog, rules: Rulebook) -> Session {
        let mut session =
            Session::fly(catalog, Pilot::new(catalog, "Ada").expect("a pilot")).expect("flies");
        session.setup.rules = rules;
        session
    }

    /// The marks after a reset with `owned`.
    fn marked_by(owned: &[OutfitId]) -> BTreeSet<GovtId> {
        let catalog = scrambled();
        let mut session = flying(&catalog, Rulebook::default());
        for &outfit in owned {
            session.pilot.outfits.insert(outfit, 1);
        }
        session.reset_precalced();
        session.space.iff_marked
    }

    #[test]
    fn a_mod_type_48_outfit_marks_every_government_of_its_class() {
        assert_eq!(
            marked_by(&[CLASS_1]),
            BTreeSet::from([GovtId(128), GovtId(130)])
        );
        assert_eq!(
            marked_by(&[NO_CLASS_SCRAMBLER]),
            BTreeSet::new(),
            "-1 marks none"
        );
        assert_eq!(
            marked_by(&[OutfitId(445)]),
            BTreeSet::new(),
            "an inhibitor is no scrambler"
        );
        assert_eq!(marked_by(&[]), BTreeSet::new());
    }

    /// Whether government 128 is marked after a reset with the class-1
    /// scrambler, its sale and another reset, by `source`.
    fn marked_after_the_sale(source: RuleSource) -> bool {
        let catalog = scrambled();
        let mut session = flying(
            &catalog,
            Rulebook::default().with_override(RuleKey::IffScrambler, source),
        );
        session.pilot.outfits.insert(CLASS_1, 1);
        session.reset_precalced();
        assert!(session.space.iff_marked.contains(&GovtId(128)), "marked");
        session.pilot.outfits.remove(&CLASS_1);
        session.reset_precalced();
        session.space.iff_marked.contains(&GovtId(128))
    }

    #[test]
    fn by_the_engine_a_mark_outlives_its_sale_and_by_the_other_reading_it_does_not() {
        assert!(
            marked_after_the_sale(RuleSource::Engine),
            "only _InitObjects clears it"
        );
        assert!(
            !marked_after_the_sale(RuleSource::Bible),
            "worked out afresh"
        );
    }

    #[test]
    fn the_traffic_sees_the_marks_and_the_players_ship_type_and_combat_government() {
        let catalog = scrambled();
        let mut session = flying(&catalog, Rulebook::default());
        session.pilot.outfits.insert(CLASS_1, 1);
        session.reset_precalced();
        let world = session.world();
        assert_eq!(
            world.iff_marked,
            &BTreeSet::from([GovtId(128), GovtId(130)])
        );
        let around = world.around(&[]);
        assert!(around.iff_marked(Some(GovtId(130))));
        assert!(!around.iff_marked(Some(GovtId(129))));
        assert!(!around.iff_marked(None));
        let player = session.player_side();
        assert_eq!(player.combat_govt, Some(GovtId(129)));
        assert_eq!(player.ship, ShipId(128));
    }
}
