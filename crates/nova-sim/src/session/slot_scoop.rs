//! The session's tests of the can-scoop flag a ship-table slot keeps
//! (see [`berth_scoops`](crate::traffic::berth_scoops)): how
//! [`RuleKey::ReusedSlotScoop`] reaches the traffic, and the eject's hulk,
//! the one ship the session brings in with no `_ShipCanScoop` asked.

use super::eject::tests::{AUTO, POD, ejecting, flying, hulk, in_the_pod, to_the_first_check};
use super::tests::{armed, facing_an_npc};
use super::*;
use crate::rulebook::{RuleSource, Rulebook};

/// The rulebook with `reading` of [`RuleKey::ReusedSlotScoop`].
fn reading(reading: RuleSource) -> Rulebook {
    Rulebook::default().with_override(RuleKey::ReusedSlotScoop, reading)
}

#[test]
fn the_reused_slot_scoop_reading_is_set_by_the_rulebook_and_outlives_a_new_traffic() {
    let catalog = armed();
    for source in [RuleSource::Engine, RuleSource::Bible] {
        let mut session = facing_an_npc(&catalog, 180).with_rules(reading(source));
        assert_eq!(session.space.traffic.reused_slot_scoop(), source);
        let stellar = session.chart.sites[0].id;
        session
            .land_at(&catalog, session.pilot.system, stellar)
            .expect("relocates");
        assert_eq!(session.npcs(), [], "a new traffic");
        assert_eq!(
            session.space.traffic.reused_slot_scoop(),
            source,
            "re-applied"
        );
    }
}

#[test]
fn the_ejects_hulk_holds_its_berths_word_by_the_engine_and_leaves_it() {
    // The player's class carries no scoop; every berth was left scooping.
    let catalog = ejecting(100);
    for (source, held) in [(RuleSource::Engine, true), (RuleSource::Bible, false)] {
        let mut session = flying(&catalog, &[POD, AUTO]).with_rules(reading(source));
        for berth in 0..16 {
            session.space.traffic.write_berth_scoop(berth, true);
        }
        to_the_first_check(&mut session, &catalog, false);
        assert!(in_the_pod(&session), "{source:?}");
        let hulk = hulk(&session);
        assert_eq!(hulk.can_scoop, held, "{source:?}");
        assert_eq!(
            session.space.traffic.berth_scoop(hulk.berth),
            Some(true),
            "{source:?}: left as it was"
        );
    }
}
