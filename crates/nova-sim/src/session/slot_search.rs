//! The session's tests of the search timer a ship-table slot keeps
//! (rdm combat-ai-fidelity phase 21): the reading of
//! [`RuleKey::ReusedSlotSearch`] reaches the traffic, and a new traffic
//! (see [`berth_search`](crate::traffic::berth_search)).

use super::tests::{armed, facing_an_npc};
use super::*;
use crate::rulebook::{RuleSource, Rulebook};

#[test]
fn the_reused_slot_search_reading_is_set_by_the_rulebook_and_outlives_a_new_traffic() {
    let catalog = armed();
    for reading in [RuleSource::Engine, RuleSource::Bible] {
        let rules = Rulebook::default().with_override(RuleKey::ReusedSlotSearch, reading);
        let mut session = facing_an_npc(&catalog, 180).with_rules(rules);
        assert_eq!(session.space.traffic.reused_slot_search(), reading);
        let stellar = session.sites[0].id;
        session
            .land_at(&catalog, session.pilot.system, stellar)
            .expect("relocates");
        assert_eq!(session.npcs(), [], "a new traffic");
        assert_eq!(
            session.space.traffic.reused_slot_search(),
            reading,
            "re-applied"
        );
    }
}
