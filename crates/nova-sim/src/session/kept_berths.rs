//! The berths a session's traffic keeps for the next, across the
//! session's own traffic Reset and from one session to the next (see
//! [`kept_berths`](crate::traffic::kept_berths)): each berth's cached
//! odds (+0xc914), search timer (+0xc90c), voice variant (+0xc922),
//! person (+0xc8d0), pick (+0x72) and exit cursors (+0xc8fe-+0xc905), and
//! the player's own exit cursors (slot 0's +0xc8fe-+0xc905), which the
//! original's ship table keeps from `_InitArrays` (@0x1b7a1-0x1b7b5) until
//! the app quits, as no relocation, pilot load or new game writes them.
//!
//! The session hands them on as [`Session::with_slot_sprites`] hands on
//! its ship slots' sprites: [`Session::kept_berths`] gives them, and
//! [`Session::with_kept_berths`] starts a session's traffic and the
//! player's exit cursors from them. The Reset (`TrafficStep::Reset`, the
//! developer edits' move or landing) hands its traffic's on to the new
//! one; the player's ship outlives it. The player's pick (slot 0's +0x72,
//! the selected secondary) is not carried: `_ResetPlayer` writes it -1 on
//! every load and new game (@0x1d4dd).

use super::{PlayerShip, Session, Space};
use crate::combat::exits::ExitCursors;
use crate::rulebook::{RuleKey, RuleSource};
use crate::traffic::kept_berths::KeptBerths;

impl Session {
    /// What its traffic's berths and the player's exit cursors keep, to
    /// hand on to a session started later in the same run
    /// ([`Session::with_kept_berths`]).
    #[must_use]
    pub(crate) fn kept_berths(&self) -> KeptBerths {
        self.space
            .kept_berths()
            .with_player_exits(*self.ship.fit.armament.exit_cursors())
    }

    /// The session, its traffic's berths starting from `kept`, what a
    /// session flown earlier in the same run left
    /// ([`Session::kept_berths`]), in place of a launch's empty table, and
    /// the player's exit cursors from `kept`'s by
    /// [`RuleKey::ReusedSlotExits`]. The traffic keeps the session's
    /// readings, so this runs after [`Session::with_rules`], as
    /// `Session::set_up` orders them.
    #[must_use]
    pub(crate) fn with_kept_berths(mut self, kept: KeptBerths) -> Self {
        self.ship.take_kept_exits(
            kept.player_exits(),
            self.setup.rules.source_for(RuleKey::ReusedSlotExits),
        );
        self.space.traffic.keep_berths(kept);
        self
    }
}

impl PlayerShip {
    /// Takes the exit cursors `kept` from the last session's player by
    /// `reading`: by the engine, as no pilot load or new game writes slot
    /// 0's +0xc8fe-+0xc905; by the other reading the new ship keeps its
    /// own, every `ExitType` at exit 0.
    fn take_kept_exits(&mut self, kept: ExitCursors, reading: RuleSource) {
        if reading == RuleSource::Engine {
            *self.fit.armament.exit_cursors_mut() = kept;
        }
    }
}

impl Space {
    /// What its traffic's berths keep, to hand on to a session started
    /// later in the same run ([`Session::with_kept_berths`]).
    #[must_use]
    fn kept_berths(&self) -> KeptBerths {
        self.traffic.kept_berths()
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::lost_sight::SearchTimer;
    use crate::catalog::WeaponId;
    use crate::combat::exits::ExitKind;
    use crate::pilot::Pilot;
    use crate::rulebook::Rulebook;
    use crate::session::tests::{armed, facing_an_npc};
    use crate::stats::ShipStats;
    use crate::testkit::FakePilotCatalog;
    use crate::traffic::World;
    use crate::traffic::odds_cache::{LAST_SLOT, STALE};
    use nova_sim_base::fixture::Draws;

    /// The draws `facing_an_npc` populates with: one NPC at berth 1.
    const POPULATE: [u32; 7] = [6, 6, 0, 0, 750, 650, 180];

    /// A session facing an NPC at berth 1, its odds worked out by the
    /// pass, berth 2 marked, berth 5's timer a search of 40 broken off and
    /// berth 9's person Ace.
    fn marked(catalog: &FakePilotCatalog) -> Session {
        let mut session = facing_an_npc(catalog, 180);
        session.space.traffic.refresh_odds(&World::new(&[]));
        session
            .space
            .traffic
            .write_berth_search(5, SearchTimer::Searching(40.0));
        session.space.traffic.set_berth_person(9, "Ace");
        session
    }

    /// Each berth's odds in `session`'s traffic, as bits.
    fn odds(session: &Session) -> Vec<u32> {
        (0..=LAST_SLOT + 1)
            .map(|berth| session.space.traffic.berth_odds(berth).to_bits())
            .collect()
    }

    #[test]
    fn the_scene_marks_berth_two_and_holds_a_timer_and_a_person() {
        let catalog = armed();
        let session = marked(&catalog);
        assert_ne!(
            session.space.traffic.berth_odds(1).to_bits(),
            STALE.to_bits()
        );
        assert_eq!(
            session.space.traffic.berth_odds(2).to_bits(),
            STALE.to_bits()
        );
        assert_eq!(session.space.traffic.berth_person(9), Some("Ace"));
    }

    #[test]
    fn a_relocation_keeps_every_berths_odds_search_timer_and_person() {
        let catalog = armed();
        let mut session = marked(&catalog);
        let before = session.kept_berths();
        let held = odds(&session);
        let stellar = session.chart.sites[0].id;
        session
            .land_at(&catalog, session.pilot.system, stellar)
            .expect("relocates");
        assert_eq!(session.npcs(), [], "a new traffic");
        assert_eq!(odds(&session), held);
        assert_eq!(
            session.space.traffic.berth_person(9),
            Some("Ace"),
            "for <OSN>"
        );
        assert_eq!(session.kept_berths(), before);
    }

    #[test]
    fn a_new_pilots_session_given_the_last_ones_berths_holds_its_odds_and_marker() {
        let catalog = armed();
        let last = marked(&catalog);
        let mut next = Session::start(&catalog)
            .expect("starts")
            .with_kept_berths(last.kept_berths());
        assert_eq!(odds(&next), odds(&last), "before its first populate");
        next.populate(&catalog, &mut Draws::of(&POPULATE));
        let spawned = next.npcs()[0].berth;
        assert_eq!(
            next.space.traffic.berth_odds(spawned).to_bits(),
            STALE.to_bits()
        );
        let (last, next) = (odds(&last), odds(&next));
        for berth in (0..=LAST_SLOT + 1).filter(|&berth| berth != spawned) {
            let at = berth as usize;
            assert_eq!(next[at], last[at], "berth {berth}");
        }
    }

    #[test]
    fn a_loaded_pilots_session_given_the_last_ones_berths_holds_them() {
        let catalog = armed();
        let last = marked(&catalog);
        let pilot = Pilot::new(&catalog, "Loaded").expect("a pilot");
        let next = Session::load(&catalog, pilot, &Rulebook::default())
            .expect("loads")
            .with_kept_berths(last.kept_berths());
        assert_eq!(odds(&next), odds(&last));
        assert_eq!(next.space.traffic.berth_person(9), Some("Ace"));
        assert_eq!(next.kept_berths(), last.kept_berths());
    }

    #[test]
    fn a_new_pilots_ship_takes_the_search_timer_its_berth_kept_from_the_last_pilot() {
        let catalog = armed();
        let mut last = facing_an_npc(&catalog, 180);
        last.space
            .traffic
            .write_berth_search(1, SearchTimer::Searching(40.0));
        let mut next = Session::start(&catalog)
            .expect("starts")
            .with_kept_berths(last.kept_berths());
        next.populate(&catalog, &mut Draws::of(&POPULATE));
        assert_eq!(next.npcs()[0].berth, 1);
        assert_eq!(next.npcs()[0].lost_sight, SearchTimer::Carried(40.0));
    }

    /// The boundary a berth's words cross to the next traffic.
    #[derive(Clone, Copy, Debug)]
    enum Boundary {
        /// The session's own traffic Reset, a landing by the developer
        /// edits.
        Reset,
        /// A new pilot's session after this one.
        NewGame,
        /// A loaded pilot's session after this one.
        Load,
    }

    /// The pick and the gun exit cursor of a ship brought into berth 5,
    /// which no ship holds, across `boundary`, after a ship there picked
    /// weapon 131 and fired two gun shots, the sessions reading
    /// `ReusedSlotPick` by `pick` and `ReusedSlotExits` by `exits`.
    fn carried(
        boundary: Boundary,
        pick: RuleSource,
        exits: RuleSource,
    ) -> (Option<WeaponId>, usize) {
        let catalog = armed();
        let rules = Rulebook::default()
            .with_override(RuleKey::ReusedSlotPick, pick)
            .with_override(RuleKey::ReusedSlotExits, exits);
        let mut last = facing_an_npc(&catalog, 180).with_rules(rules);
        let mut cursors = ExitCursors::default();
        cursors.step(ExitKind::Gun);
        cursors.step(ExitKind::Gun);
        last.space.traffic.write_berth_pick(5, Some(WeaponId(131)));
        last.space.traffic.write_berth_exits(5, cursors);
        let mut next = match boundary {
            Boundary::Reset => {
                let stellar = last.chart.sites[0].id;
                last.land_at(&catalog, last.pilot.system, stellar)
                    .expect("relocates");
                assert_eq!(last.npcs(), [], "a new traffic");
                last
            }
            Boundary::NewGame => Session::start(&catalog)
                .expect("starts")
                .with_rules(rules)
                .with_kept_berths(last.kept_berths()),
            Boundary::Load => {
                let pilot = Pilot::new(&catalog, "Loaded").expect("a pilot");
                Session::load(&catalog, pilot, &rules)
                    .expect("loads")
                    .with_rules(rules)
                    .with_kept_berths(last.kept_berths())
            }
        };
        let id = next
            .space
            .traffic
            .add_npc_at(crate::testkit::npc(0, ShipStats::default()), 5);
        let ship = next.space.npc(id).expect("added");
        (ship.pick, ship.armament.exit_cursors().next(ExitKind::Gun))
    }

    /// Checks that `boundary` carries berth 5's pick and exit cursors by
    /// each reading of each key.
    fn carries_by_their_readings(boundary: Boundary) {
        use RuleSource::{Bible, Engine};
        for pick in [Engine, Bible] {
            for exits in [Engine, Bible] {
                let want_pick = (pick == Engine).then_some(WeaponId(131));
                let want_gun = if exits == Engine { 2 } else { 0 };
                assert_eq!(
                    carried(boundary, pick, exits),
                    (want_pick, want_gun),
                    "{boundary:?}: pick {pick:?}, exits {exits:?}"
                );
            }
        }
    }

    #[test]
    fn a_relocation_carries_each_berths_pick_and_exit_cursors_by_their_readings() {
        carries_by_their_readings(Boundary::Reset);
    }

    #[test]
    fn a_new_pilots_session_carries_each_berths_pick_and_exit_cursors_by_their_readings() {
        carries_by_their_readings(Boundary::NewGame);
    }

    #[test]
    fn a_loaded_pilots_session_carries_each_berths_pick_and_exit_cursors_by_their_readings() {
        carries_by_their_readings(Boundary::Load);
    }
}
