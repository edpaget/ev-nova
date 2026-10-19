//! The session's side of a ship's refuel of the player
//! (`_HighLevelAIHandler`'s goal 9, @0x8f3e5-0x8f6d4 in the `EV Nova`
//! executable). The traffic flies the refueller and tallies the fuel it
//! gives the player (see `refuel_dock` in the traffic), by the rulebook's
//! [`RuleKey::RefuelCap`](crate::rulebook::RuleKey::RefuelCap) reading,
//! which the session hands the traffic in its [`World`].

#[cfg(test)]
mod tests {
    use super::super::tests::armed;
    use super::super::*;
    use crate::rulebook::{RuleSource, Rulebook};

    #[test]
    fn the_traffic_steps_in_a_world_holding_the_rulebooks_refuel_cap() {
        let catalog = armed();
        for source in [RuleSource::Engine, RuleSource::Bible] {
            let rules = Rulebook::default().with_override(RuleKey::RefuelCap, source);
            let mut session = Session::start(&catalog).expect("starts").with_rules(rules);
            assert_eq!(session.world().refuel_cap, source);
            let mut flown = None;
            session.step_traffic(|_, world| flown = Some(world.refuel_cap));
            assert_eq!(flown, Some(source));
        }
    }
}
