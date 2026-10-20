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
//! [`OutfitRules::map_explore`] says, read when the grant runs, each
//! with its place's head by the engine reading of
//! [`RuleKey::ExploredByPlace`](crate::RuleKey::ExploredByPlace)
//! (`_RecursiveAutoExplore` @0xd9b6-0xda10; see the `explored` module),
//! raising [`StoryEvent::Explored`] for each system reached, in order,
//! and none for a head; a paint
//! paints the ship; a clean-record outfit cleans the legal records; and
//! anything else is added, one more owned. An ID in range with no record
//! is added as a plain item, as the original bumps that slot's count.
//! The grant path does not refit the ship; each caller refits once.
//!
//! **Its callers.** A purchase in the outfitter
//! ([`Session::outfit`]) pays, then grants (`_DoOutfitDialog`
//! @0x5c21d), then runs the outfit's `OnPurchase` (see the `hooks`
//! module); an outfit flagged "remove after purchase" (`Flags` 0x0010)
//! that the grant added is taken away again after that. (The original takes every
//! such outfit owned away when the outfitter closes, @0x5daeb-0x5dafa,
//! one granted by boarding or `G` too; here only the one just bought
//! goes, at once.) A boarding grant goes through it once for each unit
//! granted (`_DoPlunderDialog` @0x93216). And `G`.
//!
//! **The refit.** The outfitter refits holding the shield, armour and
//! fuel to their most, as the original clamps all three when its dialog
//! closes (`_DoOutfitDialog` @0x5da4b-0x5dad4), after buying and selling
//! alike. `G`, `D` and a boarding grant (or ammunition take) work the
//! stats out through the original's `_SystemInfoToShipStats` (@0xca33,
//! `_DoPlunderDialog` @0x93226), which clamps no reserve: the fuel is
//! held to its new most, as flight clamps it every frame, and a shield
//! or armour above it is kept by the engine and held to it by the other
//! reading of [`RuleKey::RefitReserves`](crate::RuleKey::RefitReserves).
//!
//! **The operators** (arms of `Session::apply_op`), each run between the
//! original's `_ShipStatsToSystemInfo` and `_SystemInfoToShipStats`, so
//! the ship's stats follow:
//!
//! - `Gxxx` (`_EvalSetExp` @0x1541d) grants one of the
//!   outfit through the grant path, then refits the ship, gaining as a
//!   purchase does. The fuel is held to the new most; a shield or armour
//!   above it is kept by the engine (`_SystemInfoToShipStats` clamps no
//!   reserve) and held to it by the other reading of
//!   [`RuleKey::RefitReserves`](crate::RuleKey::RefitReserves). By the engine it checks neither the outfit's `Max`
//!   nor the free mass; by the other reading of
//!   [`RuleKey::GrantMax`](crate::RuleKey::GrantMax), an outfit the grant
//!   would add is refused, changing nothing, when the player owns its
//!   `Max` already or its mass (as the outfitter weighs it) is more than
//!   the free mass: the check a boarding grant is held to by that
//!   reading too ([`held_to_max`]).
//! - `Dxxx` (@0x1544b) removes one of an outfit owned
//!   (128 to 639), and with none owned does nothing at all. The ship is
//!   refitted without gaining, so its mass and stats are freed as by a
//!   sale. The fuel is held to the new most; a shield or armour above it
//!   is kept by the engine (`_SystemInfoToShipStats` clamps no reserve)
//!   and held to it by the other reading of
//!   [`RuleKey::RefitReserves`](crate::RuleKey::RefitReserves). By the engine it
//!   pays nothing; by the other reading of
//!   [`RuleKey::RemoveRefund`](crate::RuleKey::RemoveRefund) it pays what
//!   selling the outfit would.
//! - `Xxxxx` (@0x15c71) explores the system, and never its place's head
//!   (@0x15c71-0x15ca1). The original
//!   writes the explored level of any slot 128 to 2175, a system or not;
//!   here a system not on the star map is not explored, and the explored
//!   systems are one set, where the original keeps `X`'s level 1 apart
//!   from a map's level 2.

use super::{PlayerShip, Session, Setup, Statics, clamp_i16};
use crate::catalog::{OutfitId, OutfitRecord, SystemId, WeaponId};
use crate::combat::armament::MOD_WEAPON;
use crate::exploration::map_reveals;
use crate::grant::{GrantStock, held_to_max};
use crate::outfit_effects::{GrantEffect, OutfitRules, clean_records, cleaned_records};
use crate::outfitter::{self, resale, unit_mass, unit_price};
use crate::pilot::{ExploredLevel, Pilot};
use crate::rulebook::RuleSource;
use crate::story::StoryEvent;

/// The outfit IDs the grant path and `D` take: an index below 0x200
/// (`_GrantOutfitItem` @0x44d5c, `_EvalSetExp` @0x15427 and @0x15455).
pub const OUTFIT_IDS: std::ops::RangeInclusive<i16> = 128..=639;

/// Adds one of `outfit` to `pilot`'s.
fn add_one(pilot: &mut Pilot, outfit: OutfitId) {
    let owned = pilot.outfits.entry(outfit).or_default();
    *owned = owned.saturating_add(1);
}

/// Whether `record` mounts the secondary `selected`: whether one of its
/// mod slots is a weapon (`ModType` 1) whose `ModVal` names it, as the
/// outfitter's sale checks by the engine once the outfit's count falls
/// to 0 (`_DoOutfitDialog` @0x5cf73-0x5cf8c;
/// [`RuleKey::SecondarySale`](crate::RuleKey::SecondarySale)). None
/// selected is mounted by nothing.
pub(super) fn launches(record: &OutfitRecord, selected: Option<WeaponId>) -> bool {
    selected.is_some_and(|weapon| {
        record
            .mods
            .iter()
            .any(|&(mod_type, mod_val)| mod_type == MOD_WEAPON && mod_val == weapon.0)
    })
}

/// Whether the ship carries the secondary `selected`: whether an outfit
/// among `outfits` that `pilot` owns, at a count above 0, mounts it (see
/// [`launches`]). This reads the outfits, not the armament, so it holds
/// in the outfitter's sale before the refit. None selected is carried by
/// nothing. The other readings of
/// [`RuleKey::StaleSecondary`](crate::RuleKey::StaleSecondary) and
/// [`RuleKey::SecondarySale`](crate::RuleKey::SecondarySale) let go of a
/// selection not carried.
pub(super) fn carries(pilot: &Pilot, outfits: &[OutfitRecord], selected: Option<WeaponId>) -> bool {
    outfits
        .iter()
        .any(|record| pilot.owned(record.id) > 0 && launches(record, selected))
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
    /// Grants one of `outfit` through the grant path (see the module
    /// docs), without refitting: whether one more is owned.
    pub(crate) fn grant_outfit(&mut self, outfit: OutfitId) -> bool {
        if !OUTFIT_IDS.contains(&outfit.0) {
            return false;
        }
        let effect = self.setup.grant_effect(&self.statics, outfit);
        if let Some(reach) = effect.map {
            let reached = map_reveals(
                reach,
                self.pilot.system,
                self.chart.galaxy.map(),
                self.setup.hyperlinks,
                &self.statics.govts,
                self.setup.outfit_rules().map_explore,
            );
            for system in reached {
                // At level 2, with its place's head, by the engine
                // (`_RecursiveAutoExplore` @0xd9a6-0xd9af, @0xda07-0xda10).
                self.explore_place(system, ExploredLevel::Charted);
                self.raise(StoryEvent::Explored { system });
            }
            // Its known pass, with no clear (`_AutoSetExploration`
            // @0xdb06-0xdbd4).
            self.add_known();
        }
        if let Some(paint) = effect.paint {
            self.pilot.paint = paint;
        }
        clean_records(&mut self.pilot.legal, &effect.clean);
        // The original cleans the visible systems' records (@0x44f4e).
        let visible = self
            .chart
            .galaxy
            .governed()
            .filter(|&(system, _)| self.chart.galaxy.visible(system));
        let cleaned = cleaned_records(
            visible,
            |system| self.pilot.system_record(system),
            &effect.clean,
        );
        for system in cleaned {
            self.pilot.set_system_record(system, 0);
        }
        if effect.added {
            add_one(&mut self.pilot, outfit);
        }
        effect.added
    }

    /// `G`: grants one of `outfit` and refits (see the module docs).
    pub(super) fn script_grant(&mut self, outfit: OutfitId) {
        if self
            .ship
            .script_may_grant(&self.pilot, &self.statics, &self.setup, outfit)
        {
            self.grant_outfit(outfit);
            self.ship.refit(
                &mut self.pilot,
                &self.statics,
                &self.setup,
                true,
                self.setup.script_reserves(),
            );
        }
    }

    /// `X`: explores `system` when it is in the catalog's galaxy, hidden
    /// or not: the original writes the explored level itself (@0x15c71),
    /// whatever the system's visibility.
    pub(super) fn explore_system(&mut self, system: SystemId) {
        if self.chart.galaxy.exists(system) {
            self.pilot.explore(system);
        }
    }

    /// Takes away one of `outfit` just bought, when the outfitter's rules
    /// remove it after purchase (see the module docs).
    pub(crate) fn remove_after_purchase(&mut self, outfit: OutfitId) {
        take_one(&mut self.pilot, outfit);
    }
}

impl Setup {
    /// The rules granting and removing outfits follow, projected from
    /// the session's rule set ([`Session::with_rules`]).
    pub(super) fn outfit_rules(&self) -> OutfitRules {
        OutfitRules::from_rulebook(&self.rules)
    }

    /// What granting one of `outfit` does: a plain item's when it has no
    /// record.
    fn grant_effect(&self, statics: &Statics, outfit: OutfitId) -> GrantEffect {
        statics.outfit_record(outfit).map_or(
            GrantEffect {
                map: None,
                paint: None,
                clean: Vec::new(),
                added: true,
            },
            |record| GrantEffect::of(record, self.outfit_rules().invalid_map),
        )
    }
}

impl Statics {
    /// The record of `outfit`, if there is one.
    fn outfit_record(&self, outfit: OutfitId) -> Option<&OutfitRecord> {
        self.outfits.iter().find(|record| record.id == outfit)
    }
}

impl PlayerShip {
    /// `record` as a grant sees it on the pilot's ship, its `Max` as the
    /// `ModType` 27 outfits owned raise it.
    pub(super) fn grant_stock(
        &self,
        pilot: &Pilot,
        statics: &Statics,
        setup: &Setup,
        record: &OutfitRecord,
    ) -> GrantStock {
        GrantStock {
            outfit: record.id,
            item_class: record.item_class,
            mass: record.mass,
            unit_mass: unit_mass(record, self.fit.fields.mass, &setup.rules),
            owned: pilot.owned(record.id),
            max: clamp_i16(super::raised_max_of(pilot, statics, setup, record)),
        }
    }

    /// The ship's free mass, with the outfits the pilot owns, its
    /// standard equipment left out.
    pub(super) fn free_mass(&self, pilot: &Pilot, statics: &Statics, setup: &Setup) -> i64 {
        outfitter::free_mass(
            self.fit.fields,
            &self.standard(),
            &pilot.outfits,
            &statics.outfits,
            &setup.rules,
        )
    }

    /// Whether `G` may grant `outfit` (see the module docs): unless the
    /// grant would add it and [`held_to_max`] holds a grant of one to
    /// none, as a boarding grant is held.
    fn script_may_grant(
        &self,
        pilot: &Pilot,
        statics: &Statics,
        setup: &Setup,
        outfit: OutfitId,
    ) -> bool {
        if !setup.grant_effect(statics, outfit).added {
            return true;
        }
        statics.outfit_record(outfit).is_none_or(|record| {
            let stock = self.grant_stock(pilot, statics, setup, record);
            held_to_max(
                setup.outfit_rules().grant_max,
                1,
                &stock,
                self.free_mass(pilot, statics, setup),
            )
            .is_none_or(|count| count > 0)
        })
    }

    /// `D`: removes one of `outfit`, if owned, and refits, paying for it
    /// as [`OutfitRules::remove_refund`] says (see the module docs).
    pub(super) fn script_remove(
        &mut self,
        pilot: &mut Pilot,
        statics: &Statics,
        setup: &Setup,
        outfit: OutfitId,
    ) {
        if !OUTFIT_IDS.contains(&outfit.0) || !take_one(pilot, outfit) {
            return;
        }
        if setup.outfit_rules().remove_refund == RuleSource::Bible {
            let price = statics.outfit_record(outfit).map_or(0, |record| {
                unit_price(record, self.fit.fields.mass, &setup.rules)
            });
            pilot.cash = pilot.cash.saturating_add(resale(price));
        }
        self.refit(pilot, statics, setup, false, setup.script_reserves());
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::catalog::{GovtId, GovtRecord, StarSystem};
    use crate::control::{ScriptNote, SetExpr, SetOp};
    use crate::outfit_effects::{CLEAN_RECORD, MAP, PAINT, Rgb15};
    use crate::rulebook::{RuleKey, Rulebook};
    use crate::stats::MORE_SHIELD;
    use crate::testkit::{FakePilotCatalog, Scripted, catalog, govt, outfit, star};

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
    fn a_map_grant_adds_its_known_pass_and_clears_nothing() {
        let mut session = session(&mapped());
        let known = |session: &Session| -> BTreeSet<i16> {
            session.known().iter().map(|system| system.0).collect()
        };
        assert_eq!(known(&session), ids(&[130, 131, 132]));
        session.mark_known([SystemId(137)]);
        session.grant_outfit(JUMPS_2);
        assert_eq!(
            known(&session),
            ids(&[130, 131, 132, 133, 137]),
            "132's link 133 learnt, 137 kept"
        );
    }

    #[test]
    fn the_bibles_map_reaches_every_system_within_its_jumps() {
        let rules = Rulebook::default().with_override(RuleKey::MapExplore, RuleSource::Bible);
        let mut session = session(&mapped()).with_rules(rules);
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
        let rules = Rulebook::default().with_override(RuleKey::MapExplore, RuleSource::Bible);
        let mut session = self::session(&mapped()).with_rules(rules);
        session.grant_outfit(INDEPENDENTS);
        assert_eq!(explored(&session), ids(&[130, 134, 136]));
    }

    #[test]
    fn the_maps_follow_the_sessions_hyperlinks() {
        // 133 lists 132, but 132 does not list it back.
        let mut catalog = mapped();
        catalog.star_map[2] = star(132, (1.0, 1.0), &[130, 131]);
        let rules = Rulebook::default().with_override(RuleKey::MapExplore, RuleSource::Bible);
        let mut session = session(&catalog).with_rules(rules);
        session.grant_outfit(JUMPS_2);
        assert_eq!(explored(&session), ids(&[130, 131, 132]));
        let mut both = self::session(&catalog)
            .with_rules(rules)
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
    fn a_clean_record_outfit_cleans_the_records_in_its_governments_visible_systems() {
        let mut catalog = mapped();
        catalog.star_map[6].govt = Some(GovtId(140));
        crate::testkit::hideable(&mut catalog);
        let mut pilot = Pilot::new(&catalog, "Ada").expect("starts");
        // Saved while 136 is hidden by its `Visibility`: the load's refresh
        // keeps it hidden.
        pilot.bits_mut().set(crate::testkit::hiding_bit(136));
        pilot.galaxy.set_visible(SystemId(136), Some(false));
        for (system, record) in [(134, -7), (135, -5), (136, -8), (137, -6)] {
            pilot.set_system_record(SystemId(system), record);
        }
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        session.grant_outfit(CLEAN_140);
        let records: Vec<i16> = (134..=137)
            .map(|system| session.pilot().system_record(SystemId(system)))
            .collect();
        assert_eq!(
            records,
            [-7, 0, -8, -6],
            "140's visible 135 only: not its hidden 136, the independent 134 \
             or 141's 137"
        );
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
        let rules = Rulebook::default().with_override(RuleKey::InvalidMap, RuleSource::Bible);
        let mut session = self::session(&mapped()).with_rules(rules);
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
        session(catalog)
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
        assert!(
            session
                .ship
                .free_mass(&session.pilot, &session.statics, &session.setup)
                < 0
        );
    }

    #[test]
    fn g_is_held_to_the_max_and_the_free_mass_by_the_other_reading() {
        let rules = Rulebook::default().with_override(RuleKey::GrantMax, RuleSource::Bible);
        let mut session = scripted(&mapped()).with_rules(rules);
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
        let mut session = scripted(&catalog).with_rules(rules);
        run(&mut session, "G307 G307");
        assert_eq!(session.pilot().owned(HEAVY), 1);
        assert_eq!(
            session
                .ship
                .free_mass(&session.pilot, &session.statics, &session.setup),
            0
        );
    }

    #[test]
    fn the_sessions_free_mass_and_grant_stock_weigh_a_scaled_outfit_as_its_rules_say() {
        // HEAVY made 20 tons scaled by the ship's 40: 8 by the bare
        // product, but never below its own 20 by the engine.
        let mut catalog = mapped();
        catalog.outfits[7].mass = 20;
        catalog.outfits[7].flags = outfitter::OutfitFlags::MASS_BY_MASS;
        let record = catalog.outfits[7].clone();
        let weighed = |source| {
            let rules = Rulebook::default().with_override(RuleKey::ScaledMass, source);
            let mut session = session(&catalog).with_rules(rules);
            let free = session
                .ship
                .free_mass(&session.pilot, &session.statics, &session.setup);
            session.pilot.outfits.insert(HEAVY, 1);
            (
                free - session
                    .ship
                    .free_mass(&session.pilot, &session.statics, &session.setup),
                session
                    .ship
                    .grant_stock(&session.pilot, &session.statics, &session.setup, &record)
                    .unit_mass,
            )
        };
        assert_eq!(weighed(RuleSource::Engine), (20, 20));
        assert_eq!(weighed(RuleSource::Bible), (8, 8));
    }

    #[test]
    fn d_removes_one_freeing_its_mass_and_stats_and_paying_nothing() {
        let mut session = scripted(&mapped());
        run(&mut session, "G305 G305");
        let (cash, free, shield) = (
            session.pilot().cash(),
            session
                .ship
                .free_mass(&session.pilot, &session.statics, &session.setup),
            session.stats().shield,
        );
        assert_eq!(session.reserves().shield.now, shield);
        assert!(run(&mut session, "D305"));
        assert_eq!(session.pilot().owned(SHIELD), 1);
        assert_eq!(
            session
                .ship
                .free_mass(&session.pilot, &session.statics, &session.setup),
            free + 1
        );
        assert_eq!(session.stats().shield, shield - 10.0);
        assert_eq!(
            session.reserves().shield.now,
            shield,
            "by the engine the surplus is kept"
        );
        assert_eq!(session.pilot().cash(), cash, "no refund");
        run(&mut session, "D305");
        assert_eq!(
            session.pilot().outfits().count(),
            0,
            "none left is none listed"
        );
    }

    #[test]
    fn d_keeps_a_surplus_shield_and_armour_by_the_engine_and_holds_them_by_the_other_reading() {
        for (source, held) in [(RuleSource::Engine, false), (RuleSource::Bible, true)] {
            let rules = Rulebook::default().with_override(RuleKey::RefitReserves, source);
            let mut session = scripted(&mapped()).with_rules(rules);
            run(&mut session, "G305");
            let (shield, armor, fuel) = (
                session.stats().shield,
                session.stats().armor,
                session.stats().fuel,
            );
            let reserves = &mut session.pilot.reserves;
            reserves.shield.now = shield + 5.0;
            reserves.armor.now = armor + 7.0;
            reserves.fuel.now = fuel + 9.0;
            run(&mut session, "D305");
            let (shield, armor, fuel) = (
                session.stats().shield,
                session.stats().armor,
                session.stats().fuel,
            );
            let reserves = session.reserves();
            assert_eq!(
                (reserves.shield.max, reserves.armor.max, reserves.fuel.max),
                (shield, armor, fuel),
                "{source:?}: the mosts follow the stats"
            );
            if held {
                assert_eq!(reserves.shield.now, shield, "{source:?}");
                assert_eq!(reserves.armor.now, armor, "{source:?}");
            } else {
                assert_eq!(reserves.shield.now, shield + 15.0, "{source:?}");
                assert_eq!(reserves.armor.now, armor + 7.0, "{source:?}");
            }
            assert_eq!(reserves.fuel.now, fuel, "{source:?}: the fuel is held");
        }
    }

    #[test]
    fn d_pays_what_selling_would_by_the_other_reading() {
        let rules = Rulebook::default().with_override(RuleKey::RemoveRefund, RuleSource::Bible);
        let mut session = scripted(&mapped()).with_rules(rules);
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
    fn d_refunds_half_the_cost_of_an_outfit_priced_by_mass_on_a_massless_ship() {
        let mut catalog = mapped();
        catalog.outfits.push(OutfitRecord {
            cost: 100,
            flags: outfitter::OutfitFlags::PRICE_BY_MASS,
            ..outfit(310, &[])
        });
        let refund = |mass_price| {
            let rules = Rulebook::default()
                .with_override(RuleKey::RemoveRefund, RuleSource::Bible)
                .with_override(RuleKey::MassPrice, mass_price);
            let mut session = scripted(&catalog).with_rules(rules);
            run(&mut session, "G310");
            assert_eq!(session.pilot().owned(OutfitId(310)), 1);
            session.ship.fit.fields.mass = 0;
            let cash = session.pilot().cash();
            run(&mut session, "D310");
            assert_eq!(session.pilot().owned(OutfitId(310)), 0);
            session.pilot().cash() - cash
        };
        assert_eq!(refund(RuleSource::Engine), 50, "half the cost");
        assert_eq!(refund(RuleSource::Bible), 0, "half of 100 x 0");
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
            session.apply_op(&SetOp::RemoveOutfit(OutfitId(id)), &mut Scripted::default());
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

    /// Changes `session`'s galaxy as `change` says, read from `catalog`.
    fn overlay(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        change: impl FnOnce(&mut crate::story::GalaxyOverlay),
    ) {
        assert!(session.change_galaxy(catalog, change));
    }

    #[test]
    fn a_map_reaches_no_hidden_system() {
        let catalog = mapped();
        let mut session = session(&catalog);
        overlay(&mut session, &catalog, |galaxy| {
            galaxy.set_visible(SystemId(131), Some(false));
        });
        session.grant_outfit(JUMPS_2);
        assert_eq!(explored(&session), ids(&[130, 132, 133]), "131 hidden");
    }

    #[test]
    fn the_inhabited_independent_map_skips_a_hidden_system() {
        // @0x44de3 tests the visible flag.
        let catalog = mapped();
        let mut session = session(&catalog);
        overlay(&mut session, &catalog, |galaxy| {
            galaxy.set_visible(SystemId(134), Some(false));
        });
        session.grant_outfit(INDEPENDENTS);
        assert_eq!(explored(&session), ids(&[130]), "134 hidden");
    }

    #[test]
    fn a_government_class_map_skips_a_hidden_system() {
        // @0x44e45 tests the visible flag.
        let catalog = mapped();
        let mut session = session(&catalog);
        overlay(&mut session, &catalog, |galaxy| {
            galaxy.set_visible(SystemId(135), Some(false));
        });
        session.grant_outfit(CLASS_3);
        assert_eq!(explored(&session), ids(&[130]), "135 hidden");
    }

    /// [`mapped`] with 129 at 131's place and 128 at 135's, each the head
    /// of its place, both hidden, sharing exploration as `source` says.
    fn shadowed(source: RuleSource) -> (FakePilotCatalog, Session) {
        let mut catalog = mapped();
        catalog.star_map.push(star(129, (1.0, 0.0), &[]));
        catalog.star_map.push(star(128, (6.0, 0.0), &[]));
        let mut session = session(&catalog)
            .with_rules(Rulebook::default().with_override(RuleKey::ExploredByPlace, source));
        overlay(&mut session, &catalog, |galaxy| {
            galaxy.set_visible(SystemId(129), Some(false));
            galaxy.set_visible(SystemId(128), Some(false));
        });
        (catalog, session)
    }

    #[test]
    fn a_map_grant_explores_each_reached_systems_head_and_raises_explored_for_the_reached_only() {
        for source in RuleSource::ALL {
            let (_, mut session) = shadowed(source);
            session.drop_raised();
            session.grant_outfit(JUMPS_2);
            let heads: &[i16] = match source {
                RuleSource::Engine => &[129],
                RuleSource::Bible => &[],
            };
            let mut expected = ids(&[130, 131, 132]);
            expected.extend(heads);
            assert_eq!(explored(&session), expected, "{source:?}");
            assert_eq!(
                session.story_raised(),
                [130, 131, 132].map(|system| StoryEvent::Explored {
                    system: SystemId(system)
                }),
                "{source:?}"
            );
            session.grant_outfit(CLASS_3);
            assert_eq!(
                session.pilot().has_explored(SystemId(128)),
                source == RuleSource::Engine,
                "{source:?}: a government-class map"
            );
        }
    }

    #[test]
    fn x_explores_a_member_without_its_head_under_both_readings() {
        // `_EvalSetExp` @0x15c71-0x15ca1 raises only the system.
        for source in RuleSource::ALL {
            let (_, mut session) = shadowed(source);
            assert!(run(&mut session, "X131"));
            assert!(session.pilot().has_explored(SystemId(131)));
            assert!(!session.pilot().has_explored(SystemId(129)), "{source:?}");
        }
    }

    fn charted(session: &Session) -> BTreeSet<i16> {
        session.pilot().charted().map(|system| system.0).collect()
    }

    #[test]
    fn each_map_grant_charts_what_it_reaches_and_by_the_engine_their_heads() {
        // `_AutoSetExploration` at level 2 (@0x44dcb, @0x44e12, @0x44e86);
        // `_RecursiveAutoExplore` raises the system (@0xd9a6-0xd9af) and
        // its head (@0xda07-0xda10).
        for source in RuleSource::ALL {
            let heads = |engine: &[i16]| -> Vec<i16> {
                if source == RuleSource::Engine {
                    engine.to_vec()
                } else {
                    Vec::new()
                }
            };
            let (_, mut session) = shadowed(source);
            session.grant_outfit(JUMPS_2);
            let mut expected = ids(&[130, 131, 132]);
            expected.extend(heads(&[129]));
            assert_eq!(charted(&session), expected, "{source:?}: N jumps");
            let (_, mut session) = shadowed(source);
            session.grant_outfit(CLASS_3);
            let mut expected = ids(&[135]);
            expected.extend(heads(&[128]));
            assert_eq!(charted(&session), expected, "{source:?}: a class");
            // 127 at 134's place, hidden, its head.
            let mut catalog = mapped();
            catalog.star_map.push(star(127, (5.0, 0.0), &[]));
            let mut session = session_with(&catalog, source);
            overlay(&mut session, &catalog, |galaxy| {
                galaxy.set_visible(SystemId(127), Some(false));
            });
            session.grant_outfit(INDEPENDENTS);
            let mut expected = ids(&[134]);
            expected.extend(heads(&[127]));
            assert_eq!(charted(&session), expected, "{source:?}: independents");
        }
    }

    /// [`session`] sharing exploration as `source` says.
    fn session_with(catalog: &FakePilotCatalog, source: RuleSource) -> Session {
        session(catalog)
            .with_rules(Rulebook::default().with_override(RuleKey::ExploredByPlace, source))
    }

    #[test]
    fn x_explores_at_level_1_and_leaves_a_charted_system_charted() {
        // `_EvalSetExp` @0x15c93-0x15ca1 raises a level of 0 to 1.
        let mut session = scripted(&mapped());
        session.pilot.chart(SystemId(132));
        assert!(run(&mut session, "X133 X132"));
        assert_eq!(charted(&session), ids(&[132]));
        assert_eq!(explored(&session), ids(&[130, 132, 133]));
    }

    #[test]
    fn a_government_class_map_reads_the_overlays_governments() {
        let catalog = mapped();
        let mut session = session(&catalog);
        overlay(&mut session, &catalog, |galaxy| {
            galaxy.set_govt(SystemId(137), Some(Some(GovtId(140))));
            galaxy.set_govt(SystemId(135), Some(None));
        });
        session.grant_outfit(CLASS_3);
        assert_eq!(explored(&session), ids(&[130, 137]));
    }

    #[test]
    fn x_explores_a_hidden_system_too() {
        let catalog = mapped();
        let mut session = scripted(&catalog);
        overlay(&mut session, &catalog, |galaxy| {
            galaxy.set_visible(SystemId(133), Some(false));
        });
        assert!(run(&mut session, "X133"));
        assert_eq!(explored(&session), ids(&[130, 133]));
    }

    #[test]
    fn the_operators_tell_nothing() {
        let mut session = scripted(&mapped());
        run(&mut session, "G305 D305 X131");
        assert_eq!(session.take_script_notes(), Vec::<ScriptNote>::new());
    }

    /// `session` with the story events raised so far let go of.
    fn quiet(mut session: Session) -> Session {
        session.drop_raised();
        session
    }

    fn explored_events(systems: &[i16]) -> Vec<StoryEvent> {
        systems
            .iter()
            .map(|&system| StoryEvent::Explored {
                system: SystemId(system),
            })
            .collect()
    }

    #[test]
    fn a_map_raises_explored_for_each_system_it_reaches_explored_already_or_not() {
        let mut session = quiet(scripted(&mapped()));
        run(&mut session, "G300");
        assert_eq!(session.story_raised(), explored_events(&[130, 131, 132]));
        let mut session = quiet(session);
        session.grant_outfit(JUMPS_2);
        assert_eq!(
            session.story_raised(),
            explored_events(&[130, 131, 132]),
            "again"
        );
        let mut session = quiet(session);
        session.grant_outfit(BLANK_MAP);
        session.grant_outfit(SHIELD);
        assert_eq!(session.story_raised(), [], "a map of nothing, a shield");
    }

    #[test]
    fn x_raises_nothing() {
        let mut session = quiet(scripted(&mapped()));
        run(&mut session, "X133 X130");
        assert_eq!(session.story_raised(), []);
    }
}
