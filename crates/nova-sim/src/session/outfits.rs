//! Granting and removing outfits on the session: the one grant path the
//! outfitter's purchases, boarding grants and the `G` set operator share,
//! and the `G`, `D` and `X` set operators.
//!
//! **The grant path** ([`Session::grant_outfit`]) is the original's
//! `_GrantOutfitItem` (@0x44d4f): an outfit ID outside 128 to 639 does
//! nothing (@0x44d5c), and otherwise the outfit's
//! [`GrantEffect`](crate::outfit_effects::GrantEffect) applies: a map
//! explores the systems [`map_reveals`] gives from the pilot's system,
//! along the session's hyperlinks and as
//! [`OutfitRules::map_explore`] says, read when the grant runs; a paint
//! paints the ship; a clean-record outfit cleans the legal records; and
//! anything else is added, one more owned. An ID in range with no record
//! is added as a plain item, as the original bumps that slot's count.
//! The grant path does not refit the ship; each caller refits once.
//!
//! **Its callers.** A purchase in the outfitter
//! ([`Session::outfit`]) pays, then grants (`_DoOutfitDialog`
//! @0x5c21d); an outfit flagged "remove after purchase" (`Flags` 0x0010)
//! that the grant added is taken away again. (The original takes every
//! such outfit owned away when the outfitter closes, @0x5daeb-0x5dafa,
//! one granted by boarding or `G` too; here only the one just bought
//! goes, at once.) A boarding grant goes through it once for each unit
//! granted (`_DoPlunderDialog` @0x93216). And `G`.
//!
//! **The operators** ([`nova_set_ops`](crate::nova_set_ops) registers
//! them), each run between the original's `_ShipStatsToSystemInfo` and
//! `_SystemInfoToShipStats`, so the ship's stats follow:
//!
//! - `Gxxx` ([`GrantOutfitOp`], `_EvalSetExp` @0x1541d) grants one of the
//!   outfit through the grant path, then refits the ship, gaining as a
//!   purchase does. By the engine it checks neither the outfit's `Max`
//!   nor the free mass; by the other reading of
//!   [`RuleKey::GrantMax`](crate::RuleKey::GrantMax), an outfit the grant
//!   would add is refused, changing nothing, when the player owns its
//!   `Max` already or its mass (as the outfitter weighs it) is more than
//!   the free mass.
//! - `Dxxx` ([`RemoveOutfitOp`], @0x1544b) removes one of an outfit owned
//!   (128 to 639), and with none owned does nothing at all. The ship is
//!   refitted without gaining, so its mass and stats are freed as by a
//!   sale and the reserves are held to the new most. By the engine it
//!   pays nothing; by the other reading of
//!   [`RuleKey::RemoveRefund`](crate::RuleKey::RemoveRefund) it pays what
//!   selling the outfit would.
//! - `Xxxxx` ([`ExploreOp`], @0x15c71) explores the system. The original
//!   writes the explored level of any slot 128 to 2175, a system or not;
//!   here a system not on the star map is not explored, and the explored
//!   systems are one set, where the original keeps `X`'s level 1 apart
//!   from a map's level 2.

use super::Session;
use crate::catalog::{OutfitId, OutfitRecord, SystemId};
use crate::chance::Chance;
use crate::control::{SetOp, SetOpHandler};
use crate::exploration::map_reveals;
use crate::outfit_effects::{GrantEffect, OutfitRules, clean_records};
use crate::outfitter::{self, resale, unit_mass, unit_price};
use crate::pilot::Pilot;
use crate::rulebook::RuleSource;

/// The outfit IDs the grant path and `D` take: an index below 0x200
/// (`_GrantOutfitItem` @0x44d5c, `_EvalSetExp` @0x15427 and @0x15455).
pub const OUTFIT_IDS: std::ops::RangeInclusive<i16> = 128..=639;

/// Adds one of `outfit` to `pilot`'s.
fn add_one(pilot: &mut Pilot, outfit: OutfitId) {
    let owned = pilot.outfits.entry(outfit).or_default();
    *owned = owned.saturating_add(1);
}

/// Takes one of `outfit` from `pilot`'s, if it owns any: whether it did.
fn take_one(pilot: &mut Pilot, outfit: OutfitId) -> bool {
    match pilot.outfits.get_mut(&outfit) {
        Some(owned) if *owned > 1 => {
            *owned -= 1;
            true
        }
        Some(_) => {
            pilot.outfits.remove(&outfit);
            true
        }
        None => false,
    }
}

impl Session {
    /// This session with granting and removing outfits following `rules`
    /// where the Bible and the engine disagree: the engine's by default.
    #[must_use]
    pub fn with_outfit_rules(mut self, rules: OutfitRules) -> Self {
        self.outfit_rules = rules;
        self
    }

    /// The rules granting and removing outfits follow.
    #[must_use]
    pub fn outfit_rules(&self) -> OutfitRules {
        self.outfit_rules
    }

    /// The record of `outfit`, if there is one.
    fn outfit_record(&self, outfit: OutfitId) -> Option<&OutfitRecord> {
        self.outfits.iter().find(|record| record.id == outfit)
    }

    /// What granting one of `outfit` does: a plain item's when it has no
    /// record.
    fn grant_effect(&self, outfit: OutfitId) -> GrantEffect {
        self.outfit_record(outfit).map_or(
            GrantEffect {
                map: None,
                paint: None,
                clean: Vec::new(),
                added: true,
            },
            |record| GrantEffect::of(record, self.outfit_rules.invalid_map),
        )
    }

    /// The ship's free mass, with the outfits the pilot owns.
    pub(crate) fn free_mass(&self) -> i64 {
        outfitter::free_mass(
            self.fields,
            &self.defaults,
            &self.pilot.outfits,
            &self.outfits,
        )
    }

    /// Grants one of `outfit` through the grant path (see the module
    /// docs), without refitting: whether one more is owned.
    pub(crate) fn grant_outfit(&mut self, outfit: OutfitId) -> bool {
        if !OUTFIT_IDS.contains(&outfit.0) {
            return false;
        }
        let effect = self.grant_effect(outfit);
        if let Some(reach) = effect.map {
            let reached = map_reveals(
                reach,
                self.pilot.system,
                &self.star_map,
                self.hyperlinks,
                &self.govts,
                self.outfit_rules.map_explore,
            );
            for system in reached {
                self.pilot.explore(system);
            }
        }
        if let Some(paint) = effect.paint {
            self.pilot.paint = paint;
        }
        clean_records(&mut self.pilot.legal, &effect.clean);
        if effect.added {
            add_one(&mut self.pilot, outfit);
        }
        effect.added
    }

    /// Whether `G` may grant `outfit` (see the module docs): always by
    /// the engine; by the other reading, unless the grant would add it
    /// while the player owns its `Max` or it outweighs the free mass.
    fn script_may_grant(&self, outfit: OutfitId) -> bool {
        if self.outfit_rules.grant_max == RuleSource::Engine || !self.grant_effect(outfit).added {
            return true;
        }
        self.outfit_record(outfit).is_none_or(|record| {
            i32::from(self.pilot.owned(outfit)) < i32::from(record.max)
                && unit_mass(record, self.fields.mass) <= self.free_mass()
        })
    }

    /// `G`: grants one of `outfit` and refits (see the module docs).
    fn script_grant(&mut self, outfit: OutfitId) {
        if self.script_may_grant(outfit) {
            self.grant_outfit(outfit);
            self.refit(true);
        }
    }

    /// `D`: removes one of `outfit`, if owned, and refits, paying for it
    /// as [`OutfitRules::remove_refund`] says (see the module docs).
    fn script_remove(&mut self, outfit: OutfitId) {
        if !OUTFIT_IDS.contains(&outfit.0) || !take_one(&mut self.pilot, outfit) {
            return;
        }
        if self.outfit_rules.remove_refund == RuleSource::Bible {
            let price = self
                .outfit_record(outfit)
                .map_or(0, |record| unit_price(record, self.fields.mass));
            self.pilot.cash = self.pilot.cash.saturating_add(resale(price));
        }
        self.refit(false);
    }

    /// `X`: explores `system` when it is on the star map.
    fn explore_system(&mut self, system: SystemId) {
        if self.star_map.position(system).is_some() {
            self.pilot.explore(system);
        }
    }

    /// Takes away one of `outfit` just bought, when the outfitter's rules
    /// remove it after purchase (see the module docs).
    pub(crate) fn remove_after_purchase(&mut self, outfit: OutfitId) {
        take_one(&mut self.pilot, outfit);
    }
}

/// `Gxxx`: grants one of the outfit (see the module docs).
#[derive(Clone, Copy, Debug, Default)]
pub struct GrantOutfitOp;

impl SetOpHandler<Session> for GrantOutfitOp {
    fn apply(&self, op: &SetOp, session: &mut Session, _chance: &mut dyn Chance) {
        if let SetOp::GrantOutfit(outfit) = op {
            session.script_grant(*outfit);
        }
    }
}

/// `Dxxx`: removes one of the outfit (see the module docs).
#[derive(Clone, Copy, Debug, Default)]
pub struct RemoveOutfitOp;

impl SetOpHandler<Session> for RemoveOutfitOp {
    fn apply(&self, op: &SetOp, session: &mut Session, _chance: &mut dyn Chance) {
        if let SetOp::RemoveOutfit(outfit) = op {
            session.script_remove(*outfit);
        }
    }
}

/// `Xxxxx`: explores the system (see the module docs).
#[derive(Clone, Copy, Debug, Default)]
pub struct ExploreOp;

impl SetOpHandler<Session> for ExploreOp {
    fn apply(&self, op: &SetOp, session: &mut Session, _chance: &mut dyn Chance) {
        if let SetOp::Explore(system) = op {
            session.explore_system(*system);
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::catalog::{GovtId, GovtRecord, StarSystem};
    use crate::control::{ScriptNote, SetExpr};
    use crate::outfit_effects::{CLEAN_RECORD, MAP, PAINT, Rgb15};
    use crate::stats::MORE_SHIELD;
    use crate::testkit::{FakePilotCatalog, Scripted, catalog, govt, outfit, star};

    #[test]
    fn the_outfit_rules_are_the_engines_until_others_are_given() {
        let session = session(&mapped());
        assert_eq!(session.outfit_rules(), OutfitRules::default());
        let rules = OutfitRules {
            map_explore: RuleSource::Bible,
            ..OutfitRules::default()
        };
        assert_eq!(session.with_outfit_rules(rules).outfit_rules(), rules);
    }

    /// A map of 2 jumps.
    const JUMPS_2: OutfitId = OutfitId(300);
    /// A map of the inhabited independent systems.
    const INDEPENDENTS: OutfitId = OutfitId(301);
    /// A map of government class 3.
    const CLASS_3: OutfitId = OutfitId(302);
    /// Cleans the record with government 140.
    const CLEAN_140: OutfitId = OutfitId(303);
    /// Paints the ship (3, 5, 7).
    const PAINTER: OutfitId = OutfitId(304);
    /// A shield of 10, a ton, up to 2, 3001 credits.
    const SHIELD: OutfitId = OutfitId(305);
    /// A map of `ModVal` 0, which explores nothing.
    const BLANK_MAP: OutfitId = OutfitId(306);
    /// A shield of 10 weighing 100 tons, far past the 30 free.
    const HEAVY: OutfitId = OutfitId(307);

    fn lived_in(system: StarSystem, stellars: &[Option<(u32, u16)>]) -> StarSystem {
        StarSystem {
            stellars: stellars.to_vec(),
            ..system
        }
    }

    fn governed(system: StarSystem, govt: i16) -> StarSystem {
        StarSystem {
            govt: Some(GovtId(govt)),
            ..system
        }
    }

    /// The pilot starts in 130, of a diamond: 130 lists 131 then 132, 131
    /// lists 130 and 132, 132 lists 130, 131 and 133, and 133 lists 132.
    /// Off it, 134 is independent with a stellar lived in, 136 independent
    /// with one only in its fifth slot, 135 of government 140 (class 3)
    /// and 137 of government 141 (class 4). The outfits are the constants
    /// above.
    fn mapped() -> FakePilotCatalog {
        let home = Some((0x0001, 0));
        FakePilotCatalog {
            star_map: vec![
                star(130, (0.0, 0.0), &[131, 132]),
                star(131, (1.0, 0.0), &[130, 132]),
                star(132, (1.0, 1.0), &[130, 131, 133]),
                star(133, (2.0, 1.0), &[132]),
                lived_in(star(134, (5.0, 0.0), &[]), &[home]),
                governed(lived_in(star(135, (6.0, 0.0), &[]), &[home]), 140),
                lived_in(
                    star(136, (7.0, 0.0), &[]),
                    &[None, None, None, Some((0x20, 0)), home],
                ),
                governed(star(137, (8.0, 0.0), &[]), 141),
            ],
            govts: vec![
                GovtRecord {
                    classes: [3, -1, -1, -1],
                    ..govt(140)
                },
                GovtRecord {
                    classes: [4, -1, -1, -1],
                    ..govt(141)
                },
            ],
            outfits: vec![
                outfit(300, &[(MAP, 2)]),
                outfit(301, &[(MAP, -1)]),
                outfit(302, &[(MAP, -1003)]),
                outfit(303, &[(CLEAN_RECORD, 140)]),
                outfit(304, &[(PAINT, (3 << 10) | (5 << 5) | 7)]),
                OutfitRecord {
                    max: 2,
                    cost: 3001,
                    ..outfit(305, &[(MORE_SHIELD, 10)])
                },
                outfit(306, &[(MAP, 0)]),
                OutfitRecord {
                    mass: 100,
                    ..outfit(307, &[(MORE_SHIELD, 10)])
                },
            ],
            ..catalog()
        }
    }

    fn session(catalog: &FakePilotCatalog) -> Session {
        Session::fly(catalog, Pilot::new(catalog, "Ada").expect("starts")).expect("flies")
    }

    fn explored(session: &Session) -> BTreeSet<i16> {
        session.pilot().explored().map(|system| system.0).collect()
    }

    fn ids(list: &[i16]) -> BTreeSet<i16> {
        list.iter().copied().collect()
    }

    #[test]
    fn a_map_explores_and_adds_nothing() {
        let mut session = session(&mapped());
        assert!(!session.grant_outfit(JUMPS_2));
        assert_eq!(explored(&session), ids(&[130, 131, 132]), "D is missed");
        assert_eq!(session.pilot().owned(JUMPS_2), 0);
        let mut session = self::session(&mapped());
        session.grant_outfit(CLASS_3);
        assert_eq!(explored(&session), ids(&[130, 135]));
    }

    #[test]
    fn the_bibles_map_reaches_every_system_within_its_jumps() {
        let rules = OutfitRules {
            map_explore: RuleSource::Bible,
            ..OutfitRules::default()
        };
        let mut session = session(&mapped()).with_outfit_rules(rules);
        session.grant_outfit(JUMPS_2);
        assert_eq!(explored(&session), ids(&[130, 131, 132, 133]));
    }

    #[test]
    fn the_inhabited_independent_map_reads_the_stellars_as_the_rule_says_when_granted() {
        let mut session = session(&mapped());
        session.grant_outfit(INDEPENDENTS);
        assert_eq!(explored(&session), ids(&[130, 134]));
        // The rule given after the session started is the one the grant
        // reads.
        let rules = OutfitRules {
            map_explore: RuleSource::Bible,
            ..OutfitRules::default()
        };
        let mut session = self::session(&mapped()).with_outfit_rules(rules);
        session.grant_outfit(INDEPENDENTS);
        assert_eq!(explored(&session), ids(&[130, 134, 136]));
    }

    #[test]
    fn the_maps_follow_the_sessions_hyperlinks() {
        // 133 lists 132, but 132 does not list it back.
        let mut catalog = mapped();
        catalog.star_map[2] = star(132, (1.0, 1.0), &[130, 131]);
        let rules = OutfitRules {
            map_explore: RuleSource::Bible,
            ..OutfitRules::default()
        };
        let mut session = session(&catalog).with_outfit_rules(rules);
        session.grant_outfit(JUMPS_2);
        assert_eq!(explored(&session), ids(&[130, 131, 132]));
        let mut both = self::session(&catalog)
            .with_outfit_rules(rules)
            .with_hyperlinks(crate::HyperlinkRule::BothWays);
        both.grant_outfit(JUMPS_2);
        assert_eq!(explored(&both), ids(&[130, 131, 132, 133]));
    }

    #[test]
    fn a_clean_record_outfit_clears_a_bad_record_and_keeps_a_good_one() {
        let mut session = session(&mapped());
        session.pilot.legal.insert(GovtId(140), -50);
        session.pilot.legal.insert(GovtId(141), -20);
        assert!(!session.grant_outfit(CLEAN_140));
        assert_eq!(session.pilot().legal_record(GovtId(140)), 0);
        assert_eq!(session.pilot().legal_record(GovtId(141)), -20);
        assert_eq!(session.pilot().owned(CLEAN_140), 0);
        session.pilot.legal.insert(GovtId(140), 30);
        session.grant_outfit(CLEAN_140);
        assert_eq!(session.pilot().legal_record(GovtId(140)), 30);
    }

    #[test]
    fn a_paint_paints_the_ship() {
        let mut session = session(&mapped());
        assert!(!session.grant_outfit(PAINTER));
        assert_eq!(session.pilot().paint(), Some(Rgb15 { r: 3, g: 5, b: 7 }));
        assert_eq!(session.pilot().owned(PAINTER), 0);
    }

    #[test]
    fn a_plain_outfit_is_added_and_an_id_out_of_range_does_nothing() {
        let mut session = session(&mapped());
        assert!(session.grant_outfit(SHIELD));
        assert!(session.grant_outfit(SHIELD));
        assert_eq!(session.pilot().owned(SHIELD), 2);
        let before = session.pilot().clone();
        for id in [127, 640, -1, 0] {
            assert!(!session.grant_outfit(OutfitId(id)), "{id}");
        }
        assert_eq!(*session.pilot(), before);
        assert!(session.grant_outfit(OutfitId(639)), "no record: plain");
        assert!(session.grant_outfit(OutfitId(128)));
        assert_eq!(session.pilot().owned(OutfitId(639)), 1);
        assert_eq!(session.pilot().owned(OutfitId(128)), 1);
    }

    #[test]
    fn a_map_that_explores_nothing_is_used_up_or_kept_as_the_rule_says() {
        let mut session = session(&mapped());
        assert!(!session.grant_outfit(BLANK_MAP));
        assert_eq!(session.pilot().owned(BLANK_MAP), 0);
        assert_eq!(explored(&session), ids(&[130]));
        let rules = OutfitRules {
            invalid_map: RuleSource::Bible,
            ..OutfitRules::default()
        };
        let mut session = self::session(&mapped()).with_outfit_rules(rules);
        assert!(session.grant_outfit(BLANK_MAP));
        assert_eq!(session.pilot().owned(BLANK_MAP), 1);
    }

    fn set(text: &str) -> SetExpr {
        SetExpr::parse(text).expect("parses")
    }

    /// Runs `text` on `session`: whether a save fell due.
    fn run(session: &mut Session, text: &str) -> bool {
        session.take_save_due();
        session.run_set(&set(text), &mut Scripted::default());
        session.take_save_due()
    }

    /// A session of `catalog` with Nova's set operators.
    fn scripted(catalog: &FakePilotCatalog) -> Session {
        session(catalog).with_set_ops(std::rc::Rc::new(crate::nova_set_ops()))
    }

    #[test]
    fn g_grants_a_plain_outfit_and_refits() {
        let mut session = scripted(&mapped());
        let shield = session.stats().shield;
        assert!(run(&mut session, "G305"));
        assert_eq!(session.pilot().owned(SHIELD), 1);
        assert_eq!(session.stats().shield, shield + 10.0, "refitted");
        assert_eq!(session.reserves().shield.now, shield + 10.0, "gained");
        assert_eq!(session.take_script_notes(), []);
    }

    #[test]
    fn g_of_a_map_explores_and_adds_nothing() {
        let mut session = scripted(&mapped());
        assert!(run(&mut session, "G300"));
        assert_eq!(explored(&session), ids(&[130, 131, 132]));
        assert_eq!(session.pilot().owned(JUMPS_2), 0);
    }

    #[test]
    fn g_ignores_the_max_and_the_free_mass_by_the_engine() {
        let mut session = scripted(&mapped());
        assert!(run(&mut session, "G305 G305 G305 G307"));
        assert_eq!(session.pilot().owned(SHIELD), 3, "past its Max of 2");
        assert_eq!(session.pilot().owned(HEAVY), 1, "100 tons of 30 free");
        assert!(session.free_mass() < 0);
    }

    #[test]
    fn g_is_held_to_the_max_and_the_free_mass_by_the_other_reading() {
        let rules = OutfitRules {
            grant_max: RuleSource::Bible,
            ..OutfitRules::default()
        };
        let mut session = scripted(&mapped()).with_outfit_rules(rules);
        run(&mut session, "G305 G305");
        assert_eq!(session.pilot().owned(SHIELD), 2);
        let before = session.clone();
        assert!(!run(&mut session, "G305 G307"), "nothing changed");
        assert_eq!(session, before);
        // A map is never added, so it is never held.
        run(&mut session, "G300");
        assert_eq!(explored(&session), ids(&[130, 131, 132]));
        // An outfit that just fits the free mass is granted.
        let mut catalog = mapped();
        catalog.outfits[7].mass = 30;
        let mut session = scripted(&catalog).with_outfit_rules(rules);
        run(&mut session, "G307 G307");
        assert_eq!(session.pilot().owned(HEAVY), 1);
        assert_eq!(session.free_mass(), 0);
    }

    #[test]
    fn d_removes_one_freeing_its_mass_and_stats_and_paying_nothing() {
        let mut session = scripted(&mapped());
        run(&mut session, "G305 G305");
        let (cash, free, shield) = (
            session.pilot().cash(),
            session.free_mass(),
            session.stats().shield,
        );
        assert_eq!(session.reserves().shield.now, shield);
        assert!(run(&mut session, "D305"));
        assert_eq!(session.pilot().owned(SHIELD), 1);
        assert_eq!(session.free_mass(), free + 1);
        assert_eq!(session.stats().shield, shield - 10.0);
        assert_eq!(session.reserves().shield.now, shield - 10.0, "held to it");
        assert_eq!(session.pilot().cash(), cash, "no refund");
        run(&mut session, "D305");
        assert_eq!(
            session.pilot().outfits().count(),
            0,
            "none left is none listed"
        );
    }

    #[test]
    fn d_pays_what_selling_would_by_the_other_reading() {
        let rules = OutfitRules {
            remove_refund: RuleSource::Bible,
            ..OutfitRules::default()
        };
        let mut session = scripted(&mapped()).with_outfit_rules(rules);
        run(&mut session, "G305");
        let cash = session.pilot().cash();
        run(&mut session, "D305");
        assert_eq!(session.pilot().cash(), cash + 1500, "half of 3001");
        assert_eq!(session.pilot().owned(SHIELD), 0);
        run(&mut session, "D305");
        assert_eq!(session.pilot().cash(), cash + 1500, "none owned, none paid");
        session.pilot.outfits.insert(OutfitId(600), 1);
        run(&mut session, "D600");
        assert_eq!(session.pilot().owned(OutfitId(600)), 0);
        assert_eq!(session.pilot().cash(), cash + 1500, "no record, no price");
    }

    #[test]
    fn d_of_an_outfit_not_owned_changes_nothing() {
        let mut session = scripted(&mapped());
        let before = session.clone();
        assert!(!run(&mut session, "D305"), "no save due");
        assert_eq!(session, before);
        assert_eq!(session.take_script_notes(), [], "nothing noted");
        session.pilot.outfits.insert(OutfitId(127), 1);
        session.pilot.outfits.insert(OutfitId(640), 1);
        // The parser takes 128 to 639 only; a handler given another ID
        // ignores it.
        for id in [127, 640] {
            RemoveOutfitOp.apply(
                &SetOp::RemoveOutfit(OutfitId(id)),
                &mut session,
                &mut Scripted::default(),
            );
        }
        assert_eq!(session.pilot().owned(OutfitId(127)), 1, "out of range");
        assert_eq!(session.pilot().owned(OutfitId(640)), 1, "out of range");
    }

    #[test]
    fn x_explores_a_system_on_the_map_and_ignores_one_that_is_not() {
        let mut session = scripted(&mapped());
        assert!(run(&mut session, "X133"));
        assert_eq!(explored(&session), ids(&[130, 133]));
        assert!(!run(&mut session, "X999 X133"), "nothing new");
        assert_eq!(explored(&session), ids(&[130, 133]));
        assert_eq!(session.take_script_notes(), []);
    }

    #[test]
    fn the_operators_note_nothing_unhandled() {
        let mut session = scripted(&mapped());
        run(&mut session, "G305 D305 X131");
        assert_eq!(session.take_script_notes(), Vec::<ScriptNote>::new());
    }
}
