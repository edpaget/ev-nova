//! The berths a session's traffic keeps for the next, across the
//! session's own traffic Reset and from one session to the next (see
//! [`kept_berths`](crate::traffic::kept_berths)): each berth's cached
//! odds (+0xc914), search timer (+0xc90c) and person (+0xc8d0), which the
//! original's ship table keeps from `_InitArrays` (@0x1b7a1-0x1b7b5) until
//! the app quits, as no relocation, pilot load or new game writes them.
//!
//! The session hands them on as [`Session::with_slot_sprites`] hands on
//! its ship slots' sprites: [`Session::kept_berths`] gives them, and
//! [`Session::with_kept_berths`] starts a session's traffic from them.
//! The Reset (`TrafficStep::Reset`, the developer edits' move or landing)
//! hands its traffic's on to the new one. The picks and exit cursors
//! start afresh with each traffic (rdm task
//! carry-slot-picks-and-exits-across-app-life).

use super::Session;
use crate::traffic::kept_berths::KeptBerths;

impl Session {
    /// What its traffic's berths keep, to hand on to a session started
    /// later in the same run ([`Session::with_kept_berths`]).
    #[must_use]
    pub fn kept_berths(&self) -> KeptBerths {
        self.traffic.kept_berths()
    }

    /// The session, its traffic's berths starting from `kept`, what a
    /// session flown earlier in the same run left
    /// ([`Session::kept_berths`]), in place of a launch's empty table.
    /// The traffic keeps the session's readings.
    #[must_use]
    pub fn with_kept_berths(mut self, kept: KeptBerths) -> Self {
        self.traffic.keep_berths(kept);
        self
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::lost_sight::SearchTimer;
    use crate::pilot::Pilot;
    use crate::rulebook::Rulebook;
    use crate::session::tests::{armed, facing_an_npc};
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
        session.traffic.refresh_odds(&World::new(&[]));
        session
            .traffic
            .write_berth_search(5, SearchTimer::Searching(40));
        session.traffic.set_berth_person(9, "Ace");
        session
    }

    /// Each berth's odds in `session`'s traffic, as bits.
    fn odds(session: &Session) -> Vec<u32> {
        (0..=LAST_SLOT + 1)
            .map(|berth| session.traffic.berth_odds(berth).to_bits())
            .collect()
    }

    #[test]
    fn the_scene_marks_berth_two_and_holds_a_timer_and_a_person() {
        let catalog = armed();
        let session = marked(&catalog);
        assert_ne!(session.traffic.berth_odds(1).to_bits(), STALE.to_bits());
        assert_eq!(session.traffic.berth_odds(2).to_bits(), STALE.to_bits());
        assert_eq!(session.traffic.berth_person(9), Some("Ace"));
    }

    #[test]
    fn a_relocation_keeps_every_berths_odds_search_timer_and_person() {
        let catalog = armed();
        let mut session = marked(&catalog);
        let before = session.kept_berths();
        let held = odds(&session);
        let stellar = session.sites[0].id;
        session
            .land_at(&catalog, session.pilot.system, stellar)
            .expect("relocates");
        assert_eq!(session.npcs(), [], "a new traffic");
        assert_eq!(odds(&session), held);
        assert_eq!(session.traffic.berth_person(9), Some("Ace"), "for <OSN>");
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
        assert_eq!(next.traffic.berth_odds(spawned).to_bits(), STALE.to_bits());
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
        assert_eq!(next.traffic.berth_person(9), Some("Ace"));
        assert_eq!(next.kept_berths(), last.kept_berths());
    }

    #[test]
    fn a_new_pilots_ship_takes_the_search_timer_its_berth_kept_from_the_last_pilot() {
        let catalog = armed();
        let mut last = facing_an_npc(&catalog, 180);
        last.traffic
            .write_berth_search(1, SearchTimer::Searching(40));
        let mut next = Session::start(&catalog)
            .expect("starts")
            .with_kept_berths(last.kept_berths());
        next.populate(&catalog, &mut Draws::of(&POPULATE));
        assert_eq!(next.npcs()[0].berth, 1);
        assert_eq!(next.npcs()[0].lost_sight, SearchTimer::Carried(40));
    }
}
