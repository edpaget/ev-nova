//! Which system an arrival enters ([`RuleKey::ArrivalSystem`]). All
//! addresses are in `EV Nova.app`'s `EV Nova` (i386).
//!
//! **A jump** (`_HandlePlayer`'s arrival block). The original reads the
//! raw Con link the jump was selected from: the player's system (+0x74)
//! record's Con slot at +0x6c (@0x6bd20-0x6bd47; `movswl 0x6c(%esi)`
//! @0x6bd3a, the slot read @0x6bd47), and sets +0x94 to the departure
//! (@0x6bd4f). It takes the multi-jump branch only when all three hold:
//! course slot 1 is set (@0x6bd56), its stand-in matches the link's
//! (`_FindActiveCoLocatedSystem` @0x6bd67 and @0x6bd7b, compared
//! @0x6bd80), and `_ShipJumpsPerJump` is above 1 (@0x6bd8f-0x6bd9d).
//! Then each hop is written to +0x74 as `_FindActiveCoLocatedSystem` of
//! course slot 1 (@0x6bdc1-0x6bdd4), explored (@0x6bddb) and the course
//! spooled (@0x6bde0); the last hop's stand-in is written again
//! @0x6be0b. In every other case (no course, @0x6bd5b → 0x6be82; a link
//! standing elsewhere than the course, @0x6bd83 → 0x6be85; a one-jump
//! ship, @0x6bd9d → 0x6be85) the raw Con link is written @0x6be0b,
//! unresolved, and explored (@0x6be12). Only then does the galaxy refresh
//! run (`_PropagateMissionBitEffects` @0x6bf3f), whose move
//! (@0x998f1-0x99a36) acts only on a hidden player's system: a system
//! above 0x7ff is skipped (@0x998fd), a visible one by the +0x1eb test
//! (@0x99918); a hidden one moves to `_FindActiveCoLocatedSystem`
//! (@0x99929), else to `_FindFirstCoLocatedSystem` (@0x9993f) made
//! visible (@0x999a4), written @0x999bf. So a visible system the jump
//! writes stays as it is, whether or not another visible system shadows
//! it at its place, and whether or not the refresh newly shows that one.
//!
//! The engine models +0x6c as the Con slot the mode-3 selection stands in
//! (the `nav` module's `Chart::selected_slot`), and reads its raw Con
//! target ([`Chart::selected_con_target`]) as the arrival begins, before
//! the jump's fuel and days, as the original reads +0x6c (@0x6bd3a) before
//! its days (@0x6c144). The player's nav writes are refused while a jump
//! is under way (the keys, the map's key and the floating map's click),
//! so the selection at arrival is the jump's. The engine falls back on the
//! stand-in J took when the selection stands in no Con slot, which can't
//! be jumped to, or names a system away from J's place, which only
//! [`Session::plot_course`](super::Session::plot_course), a helper for
//! tests and tools, can make during the warp: neither is reached in play.
//!
//! **A wormhole** (`_PlayerEnterWormhole` 0x64005). Its three
//! `_FindActiveCoLocatedSystem` calls (@0x64085, @0x64155, @0x64204) only
//! filter: a candidate needs a visible system at its owner's place. The
//! system written @0x642a7 is the owner (+0x14) as it is: a linked pick
//! (@0x640e5, checked only for an owner other than −1, @0x640ec), an
//! unlinked pick (@0x6420f), or the player's own system when a linked
//! roll stays (@0x64236-0x64245). The `_FindFirstCoLocatedSystem` after
//! it (@0x642ae) only marks the head explored (+0x90). No refresh
//! follows, so a shadowed visible owner, which the rescue of
//! `_HandleStellarSystemVisibility` can make (a player in a shadowed
//! system holding a stellar no system owns), is entered as it is.
//!
//! **A hypergate** (`_PlayerEnterHypergate` 0x637bf) resolves by place
//! before entry: the owner, or `_FindSystemFromStellar`
//! (@0x638cd-0x638e4), then `_FindActiveCoLocatedSystem` (@0x638ed); it
//! writes that stand-in (@0x639fd), and no refresh follows. The engine's
//! hypergate exit is already a stand-in, so both readings enter it.
//!
//! **The other writers of +0x74.** `M` and `N` write the system they
//! name, unresolved (@0x158f2), then run
//! `_HandleStellarSystemVisibility` (@0x159a3), which has no hidden move:
//! a visible shadowed target is entered as it is. The escape pod's
//! rescue writes the found stellar's owner (+0x14) as it is
//! (@0x68f99-0x68f9e), or system 128 with none (@0x68f7f). The owner
//! word includes `_HandleStellarSystemVisibility`'s rescue, kept until
//! the next refresh (the `kept_rescues` module), so it can be a visible
//! system shadowed at its place: the pod enters it as it is by the
//! engine, and by its stand-in by the other reading. The developer tools' relocation and the edit's move have no
//! original and follow the key. No resolution branch makes a sound; the
//! arrival's and the gates' sounds are raised elsewhere.
//!
//! **Here.** Every way in goes through [`Chart::entry_resolution`]. By
//! the engine it is [`OverlaidGalaxy::entered`](crate::story::OverlaidGalaxy::entered):
//! the system as it is, unless it is hidden after any refresh the
//! arrival ran, when the refresh's move takes the player out of it. A
//! jump decides its system before its refresh ([`jump_system`]): the last
//! hop's stand-in on the multi-jump branch, otherwise the raw Con target.
//! By the other reading, story-runtime phase 4's and crons-news-ranks
//! phase 5's, every arrival is resolved by place after the refresh
//! ([`OverlaidGalaxy::arrival`](crate::story::OverlaidGalaxy::arrival)),
//! and a jump enters the stand-in J took. Under `colocated_resolution =
//! bible` a visible system already stands for itself, so the readings
//! differ there only in the jump's raw Con target.
//!
//! Each settles inside the step that makes the arrival, before the frame
//! is drawn, any dialog, the jump's completion or a save: nothing is
//! deferred.

use crate::catalog::SystemId;
use crate::navigation::NavMode;
use crate::pilot::Pilot;
use crate::rulebook::{RuleKey, RuleSource};
use crate::story::HiddenMove;

use super::{Chart, Setup};

impl Chart {
    /// Where entering `asked` takes the player, after any refresh the
    /// entry ran, as [`RuleKey::ArrivalSystem`] reads (see the module
    /// docs).
    pub(super) fn entry_resolution(&self, setup: &Setup, asked: SystemId) -> HiddenMove {
        match setup.rules.source_for(RuleKey::ArrivalSystem) {
            RuleSource::Engine => self.galaxy.entered(asked),
            RuleSource::Bible => self.galaxy.arrival(asked),
        }
    }

    /// The raw Con target of the slot the mode-3 selection stands in, in
    /// `pilot`'s system, for a jump to `to`: the system the original's
    /// jump writes off its multi-jump branch (+0x6c, @0x6bd3a-0x6bd47).
    /// `None` outside mode 3, with nothing selected, when no Con slot
    /// stands for the selection, or when its target is not at `to`'s
    /// place: a selection the engine let change during the warp, which
    /// the original's keys never can (`_HandlePlayer` gates them on +0x50,
    /// see the flight view), so the jump enters the system it was bound
    /// for.
    pub(super) fn selected_con_target(&self, pilot: &Pilot, to: SystemId) -> Option<SystemId> {
        let NavMode::Hyperspace(Some(selected)) = self.nav else {
            return None;
        };
        let slot = self.selected_slot(pilot, selected)?;
        let target = *self.galaxy.con_slots(pilot.system).get(slot)?;
        self.galaxy
            .map()
            .places()
            .together(target, to)
            .then_some(target)
    }

    /// The system a jump's arrival explores before its days (@0x6be12),
    /// `written` the system it writes ([`jump_system`]): by the engine,
    /// `written` as it is, at its stand-in's place, so the same nebulae;
    /// by the other reading, the system the arrival resolves it to by the
    /// flags as they stood before the refresh.
    pub(super) fn jump_explored(&self, setup: &Setup, written: SystemId) -> SystemId {
        match setup.rules.source_for(RuleKey::ArrivalSystem) {
            RuleSource::Engine => written,
            RuleSource::Bible => self.galaxy.arrival(written).to,
        }
    }
}

/// The system a jump writes before its refresh, as
/// [`RuleKey::ArrivalSystem`] reads: by the engine, `at`, the last hop's
/// stand-in, on the multi-jump branch (`multi_jump`), else the raw Con
/// target `raw` the jump was selected from, else `at`; by the other
/// reading, `at`.
pub(super) fn jump_system(
    setup: &Setup,
    multi_jump: bool,
    at: SystemId,
    raw: Option<SystemId>,
) -> SystemId {
    match setup.rules.source_for(RuleKey::ArrivalSystem) {
        RuleSource::Engine if !multi_jump => raw.unwrap_or(at),
        _ => at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::StellarId;
    use crate::chance::NeverFires;
    use crate::control::SetExpr;
    use crate::gate::{GateKind, WORMHOLE};
    use crate::hyperspace::MultiJumpRule;
    use crate::rulebook::Rulebook;
    use crate::session::jump_exploration::tests::watching_explored;
    use crate::session::tests::{
        alternates, at_gate, chained, gate_landing, gate_site, gated, hide, with_alternate,
        wormholes,
    };
    use crate::session::{LandPress, Session};
    use crate::story::{GalaxyOverlay, OverlaidGalaxy};
    use crate::testkit::Scripted;
    use crate::testkit::{
        FakePilotCatalog, begin_jump_now, fly_out, hiding_bit, land_now, planet, star,
    };

    fn ids(list: &[i16]) -> Vec<SystemId> {
        list.iter().copied().map(SystemId).collect()
    }

    /// The setup with `arrival_system` reading `source`.
    fn setup(source: RuleSource) -> Setup {
        Setup {
            rules: Rulebook::default().with_override(RuleKey::ArrivalSystem, source),
            ..Setup::default()
        }
    }

    /// What `engine` and `other` give by `source`.
    fn by<T>(source: RuleSource, engine: T, other: T) -> T {
        match source {
            RuleSource::Engine => engine,
            RuleSource::Bible => other,
        }
    }

    // The chart's readings, over a hand-built galaxy.

    /// 1 at (0, 0) lists 3, 2 and 4; 2 and 3 share (50, 50), 4 is at
    /// (90, 0); each of `hidden` hidden.
    fn chart(hidden: &[i16]) -> Chart {
        let mut overlay = GalaxyOverlay::default();
        for &id in hidden {
            overlay.set_visible(SystemId(id), Some(false));
        }
        let galaxy = OverlaidGalaxy::new(
            vec![
                star(1, (0.0, 0.0), &[3, 2, 4]),
                star(2, (50.0, 50.0), &[]),
                star(3, (50.0, 50.0), &[]),
                star(4, (90.0, 0.0), &[]),
            ],
            overlay,
        );
        Chart::new(galaxy, SystemId(1))
    }

    /// A pilot in system 1.
    fn pilot() -> Pilot {
        let mut pilot = Pilot::new(&crate::testkit::catalog(), "").expect("starts");
        pilot.system = SystemId(1);
        pilot
    }

    #[test]
    fn an_entry_keeps_a_shadowed_visible_system_by_the_engine_and_resolves_it_by_the_other() {
        let chart = chart(&[]);
        let stay = |to| HiddenMove {
            to: SystemId(to),
            make_visible: false,
        };
        for source in RuleSource::ALL {
            let setup = setup(source);
            assert_eq!(
                chart.entry_resolution(&setup, SystemId(3)),
                by(source, stay(3), stay(2)),
                "{source:?}"
            );
            assert_eq!(chart.entry_resolution(&setup, SystemId(2)), stay(2));
        }
    }

    #[test]
    fn an_entry_into_a_hidden_system_moves_out_of_it_by_either_reading() {
        let shown = |to| HiddenMove {
            to: SystemId(to),
            make_visible: true,
        };
        for source in RuleSource::ALL {
            let setup = setup(source);
            assert_eq!(
                chart(&[3]).entry_resolution(&setup, SystemId(3)).to,
                SystemId(2),
                "{source:?}"
            );
            assert_eq!(
                chart(&[2, 3]).entry_resolution(&setup, SystemId(3)),
                shown(2),
                "{source:?}: the head made visible"
            );
        }
    }

    #[test]
    fn the_selected_con_target_is_the_raw_system_of_the_slot_the_selection_stands_in() {
        let pilot = pilot();
        let mut chart = chart(&[]);
        let target = |chart: &Chart| chart.selected_con_target(&pilot, SystemId(2));
        assert_eq!(target(&chart), None, "nav off");
        chart.set_nav(NavMode::Stellar(None));
        assert_eq!(target(&chart), None, "mode 2");
        chart.set_nav(NavMode::Hyperspace(None));
        assert_eq!(target(&chart), None, "none selected");
        chart.set_nav(NavMode::Hyperspace(Some(SystemId(3))));
        assert_eq!(target(&chart), Some(SystemId(3)));
        chart.select_at_place(&pilot, SystemId(2));
        assert_eq!(
            target(&chart),
            Some(SystemId(3)),
            "2 picked by place: the first slot standing there names 3"
        );
        chart.set_nav(NavMode::Hyperspace(Some(SystemId(2))));
        assert_eq!(target(&chart), Some(SystemId(2)));
        chart.set_nav(NavMode::Hyperspace(Some(SystemId(1))));
        assert_eq!(target(&chart), None, "no slot stands for 1");
    }

    #[test]
    fn a_con_target_away_from_the_jumps_place_is_none() {
        let pilot = pilot();
        let mut chart = chart(&[]);
        chart.set_nav(NavMode::Hyperspace(Some(SystemId(4))));
        assert_eq!(
            chart.selected_con_target(&pilot, SystemId(4)),
            Some(SystemId(4))
        );
        assert_eq!(
            chart.selected_con_target(&pilot, SystemId(2)),
            None,
            "the selection changed during the warp"
        );
    }

    #[test]
    fn a_jump_writes_the_raw_target_off_the_multi_jump_branch_by_the_engine() {
        let (at, raw) = (SystemId(2), Some(SystemId(3)));
        for source in RuleSource::ALL {
            let setup = setup(source);
            assert_eq!(jump_system(&setup, true, at, raw), at, "{source:?}");
            assert_eq!(
                jump_system(&setup, false, at, raw),
                by(source, SystemId(3), at),
                "{source:?}"
            );
            assert_eq!(jump_system(&setup, false, at, None), at, "{source:?}");
        }
    }

    // Jumps: a refresh at the arrival newly shows 131, the lower system at
    // the destination's place.

    /// [`alternates`] reading `arrival_system` as `source`, with 131
    /// hidden: 134 stands for its place.
    fn shadowing(catalog: &FakePilotCatalog, source: RuleSource) -> Session {
        let mut session = Session::start(catalog)
            .expect("starts")
            .with_rules(Rulebook::default().with_override(RuleKey::ArrivalSystem, source));
        assert!(hide(&mut session, catalog, &[131]));
        session
    }

    /// Plots a course to 134, flies out and begins the jump, then clears
    /// 131's hiding bit, so the arrival's refresh shows it again, and
    /// arrives.
    fn jump_as_131_shows(session: &mut Session, catalog: &FakePilotCatalog) -> Option<SystemId> {
        session.plot_course(SystemId(134)).expect("a route");
        fly_out(session);
        assert_eq!(begin_jump_now(session), Ok(SystemId(134)));
        session.pilot.bits_mut().clear(hiding_bit(131));
        session.arrive(catalog, &mut NeverFires)
    }

    /// The system the session is in, the stellars it shows, and its course.
    fn landed_in(session: &Session) -> (SystemId, Vec<StellarId>, Vec<SystemId>) {
        (
            session.system(),
            session.stellars_shown().collect(),
            session.course().to_vec(),
        )
    }

    #[test]
    fn a_multi_jump_enters_its_last_hops_stand_in_as_it_was_before_the_refresh() {
        let base = alternates();
        let catalog = FakePilotCatalog {
            star_map: base.star_map.clone(),
            ..chained(base, 2)
        };
        for source in RuleSource::ALL {
            let mut session = shadowing(&catalog, source).with_multi_jump(MultiJumpRule::default());
            let arrived = jump_as_131_shows(&mut session, &catalog);
            let (system, stellar) = by(source, (134, 141), (131, 140));
            assert_eq!(arrived, Some(SystemId(system)), "{source:?}");
            assert_eq!(
                landed_in(&session),
                (SystemId(system), vec![StellarId(stellar)], vec![]),
                "{source:?}"
            );
            assert!(session.galaxy().visible(SystemId(131)), "{source:?}");
        }
    }

    #[test]
    fn a_single_jump_from_a_slot_naming_the_shadowed_system_enters_it_as_it_is() {
        let mut catalog = alternates();
        catalog.star_map[0] = star(130, (0.0, 0.0), &[134, 131]);
        for source in RuleSource::ALL {
            let mut session = shadowing(&catalog, source);
            let arrived = jump_as_131_shows(&mut session, &catalog);
            let (system, stellar) = by(source, (134, 141), (131, 140));
            assert_eq!(arrived, Some(SystemId(system)), "{source:?}");
            assert_eq!(
                landed_in(&session),
                (SystemId(system), vec![StellarId(stellar)], vec![]),
                "{source:?}"
            );
        }
    }

    #[test]
    fn a_multi_jump_ship_off_any_course_enters_the_raw_target() {
        // No course: the multi-jump branch needs one (@0x6bd5b), so even a
        // ship of two hops a jump writes the raw Con link.
        let base = alternates();
        let catalog = FakePilotCatalog {
            star_map: base.star_map.clone(),
            ..chained(base, 2)
        };
        for source in RuleSource::ALL {
            let mut session = shadowing(&catalog, source).with_multi_jump(MultiJumpRule::default());
            assert_eq!(
                session.select_next_system(),
                Some(SystemId(134)),
                "standing in 130's one slot, which names 131"
            );
            fly_out(&mut session);
            assert_eq!(begin_jump_now(&mut session), Ok(SystemId(134)));
            session.pilot.bits_mut().clear(hiding_bit(131));
            // By the other reading, 134 resolved by place once 131 shows.
            assert_eq!(
                session.arrive(&catalog, &mut NeverFires),
                Some(SystemId(131)),
                "{source:?}"
            );
        }
    }

    #[test]
    fn a_single_jump_from_a_slot_naming_the_system_shown_again_enters_it() {
        // 130's one slot names 131, hidden at J: 134 stood for it.
        let catalog = alternates();
        for source in RuleSource::ALL {
            let mut session = shadowing(&catalog, source);
            assert_eq!(
                jump_as_131_shows(&mut session, &catalog),
                Some(SystemId(131)),
                "{source:?}"
            );
            assert_eq!(
                landed_in(&session),
                (SystemId(131), vec![StellarId(140)], vec![]),
                "{source:?}"
            );
            assert!(session.galaxy().visible(SystemId(131)), "{source:?}");
        }
    }

    #[test]
    fn a_jump_explores_the_system_it_writes_before_the_refresh() {
        // 130's one slot names 131, hidden at J: by the engine the jump
        // writes and explores it (@0x6be12), at 134's place, so the same
        // nebulae; by the other reading the arrival resolved by the flags
        // as they stood.
        let catalog = alternates();
        for source in RuleSource::ALL {
            let (mut session, seen) = watching_explored(shadowing(&catalog, source), &[131, 134]);
            jump_as_131_shows(&mut session, &catalog);
            // Then the system entered, explored again after the refresh
            // (@0x6c1db).
            assert_eq!(
                seen.systems(),
                by(source, ids(&[131, 131]), ids(&[134, 131])),
                "{source:?}"
            );
        }
    }

    // A wormhole's exit into a shadowed visible owner.

    /// [`wormholes`] over [`alternates`]' 134, the player flying in 134
    /// while 131 is visible, over wormhole 400 linked to wormhole 430 at
    /// (70, -90), which 134 alone lists: 134 is shadowed, so 430 has no
    /// owner and the rescue holds it in the player's 134.
    fn in_shadowed_134(source: RuleSource) -> (FakePilotCatalog, Session) {
        let mut catalog = wormholes();
        with_alternate(&mut catalog);
        catalog.sites = vec![
            (SystemId(130), vec![planet(128, 30.0, -40.0)]),
            (SystemId(131), vec![planet(140, 0.0, 0.0)]),
            (SystemId(134), vec![gate_landing(400, 0x2200)]),
        ];
        catalog.gates[0] = gate_site(400, (0.0, 0.0), 0x2200, &[430], 120);
        catalog
            .gates
            .push(gate_site(430, (70.0, -90.0), WORMHOLE, &[], 0));
        catalog.gate_listers = vec![
            (SystemId(130), StellarId(410)),
            (SystemId(132), StellarId(420)),
            (SystemId(134), StellarId(430)),
        ];
        let mut pilot = Pilot::new(&catalog, "").expect("starts");
        pilot.system = SystemId(134);
        let mut session = Session::fly(&catalog, pilot)
            .expect("flies")
            .with_rules(Rulebook::default().with_override(RuleKey::ArrivalSystem, source));
        assert_eq!(session.system(), SystemId(134), "shadowed, not hidden");
        assert!(session.galaxy().visible(SystemId(131)));
        assert_eq!(session.galaxy().owner(&[SystemId(134)]), None);
        session.land().expect("clearance");
        session.dock_approach();
        assert_eq!(
            session.land(),
            Ok(LandPress::AtGate {
                stellar: StellarId(400),
                kind: GateKind::Wormhole,
            })
        );
        (catalog, session)
    }

    #[test]
    fn a_wormhole_comes_out_in_a_shadowed_owner_as_it_is_by_the_engine() {
        for source in RuleSource::ALL {
            let (catalog, mut session) = in_shadowed_134(source);
            let system = by(source, SystemId(134), SystemId(131));
            assert_eq!(
                session.enter_wormhole(&catalog, &mut Scripted::rolling(&[0])),
                Ok(system),
                "{source:?}"
            );
            assert_eq!(session.system(), system, "{source:?}");
        }
    }

    #[test]
    fn a_hypergate_comes_out_in_the_stand_in_by_either_reading() {
        let mut catalog = gated();
        with_alternate(&mut catalog);
        catalog.gate_listers = vec![
            (SystemId(134), StellarId(310)),
            (SystemId(132), StellarId(320)),
        ];
        for source in RuleSource::ALL {
            let mut session = at_gate(&catalog)
                .with_rules(Rulebook::default().with_override(RuleKey::ArrivalSystem, source));
            assert_eq!(
                session.enter_hypergate(Some(SystemId(131)), &catalog, &mut NeverFires),
                Ok(SystemId(131)),
                "{source:?}: 310 is 134's, which 131 stands for"
            );
            assert_eq!(session.system(), SystemId(131), "{source:?}");
        }
    }

    // The other ways in.

    #[test]
    fn m_into_a_shadowed_visible_system_enters_it_as_it_is_by_the_engine() {
        let catalog = alternates();
        for source in RuleSource::ALL {
            let mut session = Session::start(&catalog)
                .expect("starts")
                .with_rules(Rulebook::default().with_override(RuleKey::ArrivalSystem, source));
            session.run_set(&SetExpr::parse("M134").expect("parses"), &mut NeverFires);
            let settled = session.settle_script(&catalog, &mut NeverFires);
            let (system, stellar) = by(source, (134, 141), (131, 140));
            assert_eq!(settled.moved, Some(SystemId(system)), "{source:?}");
            assert_eq!(session.system(), SystemId(system), "{source:?}");
            assert_eq!(
                session.stellars_shown().collect::<Vec<_>>(),
                [StellarId(stellar)],
                "{source:?}"
            );
        }
    }

    #[test]
    fn a_relocation_into_a_shadowed_visible_system_enters_it_as_it_is_by_the_engine() {
        let catalog = alternates();
        for source in RuleSource::ALL {
            let mut session = Session::start(&catalog)
                .expect("starts")
                .with_rules(Rulebook::default().with_override(RuleKey::ArrivalSystem, source));
            land_now(&mut session).expect("lands");
            session
                .relocate(&catalog, SystemId(134), StellarId(141))
                .expect("relocates");
            assert_eq!(
                session.system(),
                by(source, SystemId(134), SystemId(131)),
                "{source:?}"
            );
            assert!(session.galaxy().visible(SystemId(134)), "{source:?}");
        }
    }
}
