//! The player's ship target let go once it is lost, where the original
//! lets go of it.
//!
//! - **At `_HandlePlayer`'s head** (@0x68655-0x686fd, right after
//!   `_WrapPlayerPosition` @0x6864d and before the dead branch, the
//!   trigger and the move), on both passes (`_DoPlayGameWork` @0x43e19):
//!   with a target set (+0x70 != -1), the player lets go of it when its
//!   slot is inactive (+0xb8 == 0, @0x6867d), it is dying (`_IsDying`
//!   @0x68689), it is emerging from a hypergate
//!   (`_AIShipIsEmergingFromHypergate` @0x686a4), or it is cloaked
//!   (`_ShipIsCloaked` @0x686bf), not the player's escort (+0x9a != 0,
//!   @0x686d4) and the player can't target cloaked ships
//!   (`_ShipCanTargetCloakedShips` @0x686e2)
//!   ([`Session::let_go_of_lost_target`]). The test behind +0x6e == 1
//!   always holds (see [`Session::select_target`]).
//! - **At an inactive slot's display** (`_HandleShipDisplay`
//!   @0x2b532-0x2b551): a target whose slot went inactive in its own
//!   `_HandleShip` or display (it landed, jumped, docked or exploded) is
//!   let go before the frame's brackets are placed
//!   ([`Session::let_go_of_gone_target`]).
//!
//! So a target that starts breaking up, starts emerging or cloaks in a
//! pass stays the target through that pass: the frame drawn after it
//! shows it in the target panel (`_DrawStatus` @0x44041) and brackets it
//! (`_HandleWeapTargetSprites` @0x444ff), grey once it is dying
//! ([`brackets::standing`](crate::brackets::standing)), and the next
//! pass's head lets go of it. Both checks also set the status bar's
//! redraw flag (`0x3b506c`+6), which the view needs not: it redraws every
//! frame.

use super::Session;
use crate::targeting;

impl Session {
    /// `_HandlePlayer`'s lost-target check, at the head of each pass (see
    /// the module docs): lets go of the target once it is no longer in
    /// the system or no longer targetable ([`targeting::targetable`]: it
    /// is breaking up, emerging from a gate, or cloaked and neither the
    /// player can target cloaked ships nor it escorts the player). A
    /// target of an untargetable type is kept: the check never reads the
    /// bit.
    pub(super) fn let_go_of_lost_target(&mut self) {
        let cloaked_ok = self.scanner().targets;
        if self
            .target()
            .is_none_or(|npc| !targeting::targetable(npc, cloaked_ok))
        {
            self.target = None;
        }
    }

    /// `_HandleShipDisplay`'s inactive-slot branch for the target's slot
    /// (see the module docs): lets go of the target once it is no longer
    /// in the system (it was destroyed, landed, jumped out or docked, or
    /// the system was populated afresh).
    ///
    /// The original lets go at the slot's own display in the ship loop,
    /// before the brackets; the engine lets go where the NPC leaves the
    /// traffic, after the ship loop (`Session::traffic_tail`), after the
    /// take-out (`Session::fight_tail`) or as it leaves
    /// (`Session::leave`). Both settle in the same pass, before the frame
    /// is drawn.
    pub(super) fn let_go_of_gone_target(&mut self) {
        if self.target().is_none() {
            self.target = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ai::{Behaviour, Goal, Surroundings};
    use crate::brackets::Standing;
    use crate::chance::{Chance, NeverFires};
    use crate::combat::{CombatEvent, Rules};
    use crate::flight::Controls;
    use crate::session::Session;
    use crate::session::tests::{FIRE, armed, facing_an_npc};
    use crate::targeting::TargetPick;
    use crate::testkit::FakePilotCatalog;
    use crate::traffic::npc::{Npc, NpcId};

    /// Every NPC idles.
    #[derive(Debug)]
    struct Idling;

    impl Behaviour for Idling {
        fn decide(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            Goal::Idle
        }
    }

    /// An advance pass of `session` in flight.
    fn pass(session: &mut Session, catalog: &FakePilotCatalog) {
        let rescued = session.game_work(
            Controls::default(),
            catalog,
            &Idling,
            Rules::default(),
            &mut NeverFires,
        );
        assert!(!rescued);
    }

    /// Runs advance passes of `session`, the player firing at its target,
    /// NPC 0, until a pass breaks NPC 0 up.
    fn break_up_the_target(session: &mut Session, catalog: &FakePilotCatalog) {
        session.select_target(TargetPick::Next);
        assert_eq!(session.target, Some(NpcId(0)));
        session.hold_trigger(FIRE);
        for _ in 0..200 {
            pass(session, catalog);
            let broke = session
                .take_combat_events()
                .iter()
                .any(|event| matches!(event, CombatEvent::BreakingUp { .. }));
            if broke {
                return;
            }
            assert_eq!(session.target, Some(NpcId(0)), "intact");
        }
        panic!("never broke up: {:?}", session.npcs());
    }

    #[test]
    fn a_target_broken_up_by_a_shot_is_held_until_the_next_pass_head() {
        // `_HandlePlayer`'s check (@0x68655-0x686fd) lets go of a dying
        // target at the head of the next frame's `_HandlePlayer`, so the
        // frame that breaks it up still draws it, its brackets grey.
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        break_up_the_target(&mut session, &catalog);
        assert_eq!(session.target, Some(NpcId(0)), "held through the pass");
        let brackets = session.target_brackets().expect("bracketed");
        assert_eq!(brackets.standing, Standing::Disabled);
        // The head of the next pass, before its trigger and its move.
        session.tick(Controls::default());
        assert_eq!(session.target, None);
    }

    #[test]
    fn a_target_that_cloaks_is_let_go_at_the_next_pass_head() {
        // Its display steps its fade into its cloak (@0x2d088), after the
        // pass's head; the player can't target cloaked ships.
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        session.select_target(TargetPick::Next);
        session.traffic.npcs_mut()[0].cloak = crate::cloak::Cloak {
            fade: 15.5,
            direction: 1,
        };
        pass(&mut session, &catalog);
        assert!(session.npcs()[0].cloak.is_cloaked(), "cloaked");
        assert_eq!(session.target, Some(NpcId(0)), "held through the pass");
        session.tick(Controls::default());
        assert_eq!(session.target, None);
    }

    #[test]
    fn a_target_broken_up_on_the_advance_pass_is_let_go_at_the_second_pass_head() {
        // `_HandlePlayer` runs on both passes (`_DoPlayGameWork` @0x43e19).
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        break_up_the_target(&mut session, &catalog);
        assert_eq!(session.target, Some(NpcId(0)));
        session.tick_again(
            Controls::default(),
            &Idling,
            Rules::default(),
            &mut NeverFires,
        );
        assert_eq!(session.target, None);
    }

    #[test]
    fn a_target_taken_out_is_let_go_in_the_same_pass() {
        // Its slot goes inactive in its own display, which lets go of it
        // (@0x2b532-0x2b551) before the frame is drawn.
        let mut catalog = armed();
        catalog.hulls[1].death_delay = 0;
        let mut session = facing_an_npc(&catalog, 180);
        session.select_target(TargetPick::Next);
        session.traffic.npcs_mut()[0].reserves.armor.now = 0.0;
        pass(&mut session, &catalog);
        assert!(session.npcs().is_empty(), "destroyed and taken out");
        assert_eq!(session.target, None);
    }
}
