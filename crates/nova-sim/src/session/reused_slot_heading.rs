//! The session's test of the reused slot's heading
//! (rdm:phase/combat-ai-fidelity-cross-roadmap/phase-19-autopilot-low-level-at-the-slot):
//! a ship brought into a ship-table slot turns toward the heading its
//! slot's last ship wanted until its first low level, by the rulebook's
//! [`RuleKey::ReusedSlotHeading`]. The session sets the reading on its
//! traffic, and again on each new traffic it makes as the player lands
//! (see [`slot_word`](crate::traffic::slot_word)).

use super::tests::{armed, facing_an_npc};
use super::*;
use crate::rulebook::{RuleSource, Rulebook};

#[test]
fn the_reused_slot_heading_reading_is_set_by_the_rulebook_and_outlives_a_new_traffic() {
    let catalog = armed();
    for reading in [RuleSource::Engine, RuleSource::Bible] {
        let rules = Rulebook::default().with_override(RuleKey::ReusedSlotHeading, reading);
        let mut session = facing_an_npc(&catalog, 180).with_rules(rules);
        assert_eq!(session.space.traffic.reused_slot_heading(), reading);
        let stellar = session.chart.sites[0].id;
        session
            .land_at(&catalog, session.pilot.system, stellar)
            .expect("relocates");
        assert_eq!(session.npcs(), [], "a new traffic");
        assert_eq!(
            session.space.traffic.reused_slot_heading(),
            reading,
            "re-applied"
        );
    }
}
