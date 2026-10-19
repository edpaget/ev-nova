//! What the original's ship table keeps from one traffic to the next: each
//! slot's cached odds (+0xc914), search timer (+0xc90c) and person
//! (+0xc8d0).
//!
//! **The original.** The ship table is one `NewPtr(0x325000)` that
//! `_InitArrays` takes once (@0x1b7a1-0x1b7b5, called from `_InitEVNova`
//! @0x466fa) and keeps until the app quits. `_InitObjects` (@0x1c2e5,
//! called once from `_InitEVNova` @0x47252) sets some of each slot's words
//! at launch, none of these three, and the fresh pages the table is
//! served from read 0. After that, only the spawns, the odds pass and the
//! AI write +0xc914 (see [`odds_cache`](super::odds_cache)), only
//! `_HighLevelAIHandler` writes +0xc90c (see
//! [`berth_search`](super::berth_search)), and only the spawns and the
//! captures write an NPC slot's +0xc8d0 (`_ResetPlayer` @0x1d4fb writes
//! the player's slot 0 alone). No relocation, pilot load or new game
//! (`_OpenPilotFile`, `_LoadPilotData`, `_ResetPlayer`, `_DoNewPilot`,
//! `_EnterGameFromMainScreen`, `_PlayGame`) writes them, so each slot's
//! words outlast its ship, its system, its pilot and its game.
//!
//! **Here.** A [`Traffic`] lasts until a traffic Reset (the developer
//! edits' move or landing, `nova_sim::Session::relocate`) or a new
//! session. [`KeptBerths`] is what it hands on to the next:
//! [`Traffic::kept_berths`] takes it from one and [`Traffic::keep_berths`]
//! gives it to the next, which keeps its own readings. The session hands
//! it on across its own Reset, and the app from each session to the next
//! (`nova_sim::Session::with_kept_berths`), so it lasts as long as the app.
//!
//! What the carry changes: the search timers reach a new ship by
//! [`RuleKey::ReusedSlotSearch`](crate::RuleKey::ReusedSlotSearch), and
//! `<OSN>` reads the person of the slot a hail named after a Reset, while
//! no new ship has come into it (a new pilot or a load names no slot,
//! `_InitMissions` @0x97bbf). The odds carry changes no read: every
//! active slot was written -1 by its spawn, and a carried marker only
//! joins the walk the new ships' marks make.
//!
//! The picks (+0x72) and the exit cursors (+0xc8fe-+0xc905) the original's
//! table keeps too are not handed on: a new traffic starts them empty (see
//! [`berth_kept`](super::berth_kept)).

use std::collections::BTreeMap;

use super::Traffic;
use super::berth_search::BerthSearch;
use super::odds_cache::OddsCache;

/// The words each berth keeps from one traffic to the next (see the
/// module docs).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct KeptBerths {
    /// Each berth's last search timer.
    search: BerthSearch,
    /// Each berth's cached odds, and the pass's marker.
    odds: OddsCache,
    /// The person of the ship last brought in at each berth.
    persons: BTreeMap<u32, String>,
}

impl Traffic {
    /// What its berths keep, to hand on to the next traffic (see
    /// [`kept_berths`](self)).
    #[must_use]
    pub fn kept_berths(&self) -> KeptBerths {
        KeptBerths {
            search: self.berth_search.clone(),
            odds: self.odds.clone(),
            persons: self.berth_persons.clone(),
        }
    }

    /// Takes the berths `kept` from an earlier traffic, keeping its own
    /// readings, before any ship comes in (see
    /// [`kept_berths`](self)).
    pub fn keep_berths(&mut self, kept: KeptBerths) {
        self.berth_search.take_timers_of(kept.search);
        self.odds = kept.odds;
        self.berth_persons = kept.persons;
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::Goal;
    use crate::ai::lost_sight::SearchTimer;
    use crate::catalog::GovtId;
    use crate::combat::ShipRef;
    use crate::rulebook::RuleSource;
    use crate::stats::ShipStats;
    use crate::traffic::World;
    use crate::traffic::fixture::npc;
    use crate::traffic::npc::NpcId;
    use crate::traffic::odds_cache::LAST_SLOT;
    use nova_sim_base::fixture::FAST;

    /// A traffic of three ships at berths 1-3, the third attacking the
    /// second, whose odds pass has run twice: berths 1 and 2 at their odds
    /// and berth 3 marked.
    fn passed() -> Traffic {
        let mut traffic = Traffic::with_interval(1);
        for (strength, govt) in [(100.0, 140), (50.0, 141), (25.0, 142)] {
            let mut ship = npc(0, ShipStats::new(FAST, &[]));
            ship.hull.strength = strength;
            ship.govt = Some(GovtId(govt));
            traffic.add_npc(ship);
        }
        traffic.npcs[2].goal = Goal::Attack(ShipRef::Npc(NpcId(1)));
        traffic.refresh_odds(&World::new(&[]));
        traffic.refresh_odds(&World::new(&[]));
        traffic
    }

    /// Each berth's odds in `traffic`, as bits.
    fn odds(traffic: &Traffic) -> Vec<u32> {
        (0..=LAST_SLOT + 1)
            .map(|berth| traffic.berth_odds(berth).to_bits())
            .collect()
    }

    #[test]
    fn a_new_traffic_given_the_kept_berths_reads_the_odds_and_marker_they_held() {
        let old = passed();
        assert_eq!(old.berth_odds(2), 0.5, "the scene");
        let mut new = Traffic::new();
        new.keep_berths(old.kept_berths());
        assert_eq!(odds(&new), odds(&old));
    }

    #[test]
    fn a_ship_brought_into_a_new_traffic_takes_its_berths_search_timer() {
        let mut old = passed();
        old.write_berth_search(1, SearchTimer::Searching(40));
        let mut new = Traffic::new();
        new.keep_berths(old.kept_berths());
        let id = new.add_npc_at(npc(0, ShipStats::new(FAST, &[])), 1);
        let ship = new.npcs().iter().find(|npc| npc.id == id).expect("added");
        assert_eq!(ship.lost_sight, SearchTimer::Carried(40));
    }

    #[test]
    fn keeping_berths_keeps_the_traffics_own_reading() {
        let mut old = passed();
        old.write_berth_search(1, SearchTimer::Searching(40));
        let mut new = Traffic::new();
        new.set_reused_slot_search(RuleSource::Bible);
        new.keep_berths(old.kept_berths());
        assert_eq!(new.reused_slot_search(), RuleSource::Bible);
        let id = new.add_npc_at(npc(0, ShipStats::new(FAST, &[])), 1);
        let ship = new.npcs().iter().find(|npc| npc.id == id).expect("added");
        assert_eq!(ship.lost_sight, SearchTimer::Unset, "no timer taken");
    }

    #[test]
    fn a_new_traffic_given_the_kept_berths_names_each_berths_last_person() {
        let mut old = passed();
        old.set_berth_person(9, "Ace");
        let mut new = Traffic::new();
        new.keep_berths(old.kept_berths());
        assert_eq!(new.berth_person(9), Some("Ace"));
        assert_eq!(new.berth_person(1), None);
    }
}
