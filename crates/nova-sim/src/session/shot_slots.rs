//! The session's view of the original's shot slots (rdm weapons-fidelity
//! phase 34): the order the shots are drawn in, each shot layer as its
//! slots entered it (see `nova_sim_combat::combat::shot_slots`), and the
//! rules that read the slots.

use super::Session;
use crate::combat::projectile::Shot;

impl Session {
    /// The shots in flight in the order they are drawn, bottom first: by
    /// layer, and within a layer by
    /// [`RuleKey::ShotOverlap`](crate::rulebook::RuleKey::ShotOverlap), the
    /// order their slots entered it by the engine, firing order by the
    /// other reading
    /// ([`Combat::shots_drawn`](crate::combat::Combat::shots_drawn)).
    #[must_use]
    pub fn shots_drawn(&self) -> Vec<&Shot> {
        self.combat.shots_drawn()
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{FIRE, armed};
    use super::super::*;
    use crate::chance::NeverFires;
    use crate::combat::Rules;
    use crate::combat::shot_layer::ShotLayer;
    use crate::rulebook::{RuleSource, Rulebook};

    #[test]
    fn the_shots_are_drawn_in_the_fights_order() {
        let catalog = armed();
        let mut session = Session::start(&catalog).expect("starts");
        session.hold_trigger(FIRE);
        while session.shots().len() < 2 {
            session.tick_combat(Rules::default(), &mut NeverFires);
        }
        session.combat.shots_mut()[0].layer = ShotLayer::OverShips;
        let ids = |shots: Vec<&Shot>| shots.iter().map(|shot| shot.id).collect::<Vec<_>>();
        let first = session.shots()[0].id;
        assert_eq!(
            ids(session.shots_drawn()),
            ids(session.combat.shots_drawn())
        );
        assert_eq!(ids(session.shots_drawn())[1], first, "over the ships");
    }

    #[test]
    fn the_rulebook_says_how_the_shot_layers_overlap() {
        let catalog = armed();
        let session = |rules: Rulebook| Session::start(&catalog).expect("starts").with_rules(rules);
        let mut firing = Combat::default();
        firing.set_shot_overlap(RuleSource::Bible);
        assert_ne!(firing, Combat::default());
        let other = Rulebook::default().with_override(RuleKey::ShotOverlap, RuleSource::Bible);
        assert_eq!(session(other).combat, firing);
        assert_eq!(session(Rulebook::default()).combat, Combat::default());
    }
}
