//! [`Session::key_fades`]: the facts [`KeyFades::of`] decides what fades
//! the cursor from, gathered for [`Session::fill_frame`]. It reads the
//! ship, the mode and the port, through [`Session::takes`], so it stays a
//! `Session` method. The decision and its tests are in
//! [`key_fade`](crate::key_fade); these say each fact reaches it.

use super::super::Session;
use super::super::game_work::PlayerHead;
use crate::key_fade::{KeyFacts, KeyFades};
use crate::player_fate::PlayerCommand;

impl Session {
    /// What fades the cursor now ([`KeyFades`]). `handlers` is
    /// [`PlayerCommand::FrameRate`] taken, which is taken wherever a key
    /// block runs. `_IsDisabled` and `_IsDying` are the next pass's
    /// [`PlayerHead`], as the countdown and the cloak key read them.
    pub(super) fn key_fades(&self) -> KeyFades {
        let head = PlayerHead::of(self.ship.condition);
        KeyFades::of(KeyFacts {
            handlers: self.takes(PlayerCommand::FrameRate),
            disabled: head.disabled,
            jump_under_way: self.mode.jump().is_some(),
            secondary: self.secondary().is_some(),
            land: self.takes(PlayerCommand::Land),
            mission_info: self.takes(PlayerCommand::MissionInfo),
            self_destruct: self.ship.fate.self_destruct.chord_fades(head.dying),
            auto_eject: self.ship.auto_eject_fires(&self.pilot, &self.statics),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::catalog::{OutfitId, SystemId, WeaponId};
    use crate::combat::hull::Condition;
    use crate::escape::MOD_AUTO_EJECT;
    use crate::key_fade::{FadeKey, KeyFades};
    use crate::player_fate::POD_TICKS;
    use crate::self_destruct::{END, SelfDestruct};
    use crate::session::Session;
    use crate::session::frame::tests::filled;
    use crate::testkit::{catalog, fly_out, outfit};

    /// A session flying.
    fn flying() -> Session {
        Session::start(&catalog()).expect("starts")
    }

    /// `session`'s fades, filled over a stale frame.
    fn fades(session: &Session) -> KeyFades {
        filled(session).key_fades
    }

    #[test]
    fn flying_the_helm_fades_the_eject_doesnt_and_nothing_fades_unbidden() {
        let fades = fades(&flying());
        assert!(fades.fades(FadeKey::Helm));
        assert!(!fades.fades(FadeKey::Eject));
        assert!(!fades.fades(FadeKey::WeaponSafety));
        assert!(!fades.fades(FadeKey::Secondary));
        assert!(!fades.unbidden());
    }

    #[test]
    fn a_secondary_selected_opens_s_and_ctrl() {
        let mut session = flying();
        session.ship.secondary = Some(WeaponId(128));
        let fades = fades(&session);
        assert!(fades.fades(FadeKey::WeaponSafety));
        assert!(fades.fades(FadeKey::Secondary));
    }

    #[test]
    fn disabled_refuses_the_helm_and_opens_the_eject() {
        let mut session = flying();
        session.set_player_condition(Condition::Disabled);
        let fades = fades(&session);
        assert!(!fades.fades(FadeKey::Helm));
        assert!(fades.fades(FadeKey::Eject));
    }

    #[test]
    fn breaking_up_refuses_the_helm_landing_and_the_mission_info_and_leaves_the_free_keys() {
        let mut session = flying();
        session.set_player_condition(Condition::Dying {
            ticks_left: 100,
            starting: false,
        });
        let fades = fades(&session);
        for key in [FadeKey::Helm, FadeKey::Land, FadeKey::MissionInfo] {
            assert!(!fades.fades(key), "{key:?}");
        }
        assert!(fades.fades(FadeKey::Free));
        assert!(fades.fades(FadeKey::Eject));
    }

    #[test]
    fn a_dying_start_of_the_countdown_does_not_fade() {
        let mut session = flying();
        session.set_player_condition(Condition::Disabled);
        assert!(fades(&session).fades(FadeKey::SelfDestruct), "disabled");
        session.set_player_condition(Condition::Dying {
            ticks_left: 100,
            starting: false,
        });
        assert!(!fades(&session).fades(FadeKey::SelfDestruct), "dying");
    }

    #[test]
    fn in_the_pod_and_exploded_nothing_fades() {
        let mut pod = flying();
        pod.pod = Some(POD_TICKS);
        assert_eq!(fades(&pod), KeyFades::NONE);
        let mut exploded = flying();
        exploded.set_player_condition(Condition::Destroyed);
        assert_eq!(fades(&exploded), KeyFades::NONE);
    }

    #[test]
    fn a_pre_jump_fades_unbidden_until_it_is_given_up() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session, &catalog);
        assert!(!fades(&session).unbidden());
        session.begin_jump().expect("accepted");
        assert!(fades(&session).unbidden());
        session.collapse_jump(false);
        assert!(!fades(&session).unbidden());
    }

    #[test]
    fn the_take_off_grace_refuses_the_mission_info_and_leaves_the_free_keys() {
        let mut session = flying();
        session.start_takeoff_grace();
        let fades = fades(&session);
        assert!(!fades.fades(FadeKey::MissionInfo));
        assert!(fades.fades(FadeKey::Free));
    }

    #[test]
    fn auto_eject_armed_on_a_ship_carrying_it_fades_unbidden() {
        const AUTO: i16 = 187;
        let mut stock = catalog();
        stock.outfits.push(outfit(AUTO, &[(MOD_AUTO_EJECT, 0)]));
        let mut session = Session::start(&stock).expect("starts");
        let armed = Condition::Dying {
            ticks_left: 1,
            starting: false,
        };
        session.set_player_condition(armed);
        assert!(!fades(&session).unbidden(), "not carried");
        session.pilot.outfits.insert(OutfitId(AUTO), 1);
        assert!(fades(&session).unbidden(), "armed");
        session.set_player_condition(Condition::Disabled);
        assert!(!fades(&session).unbidden(), "not breaking up");
    }

    #[test]
    fn the_countdown_fades_only_at_its_last_pass() {
        let mut session = flying();
        session.ship.fate.self_destruct = SelfDestruct::at(100);
        assert!(!fades(&session).fades(FadeKey::SelfDestruct), "running");
        session.ship.fate.self_destruct = SelfDestruct::at(END + 1);
        assert!(fades(&session).fades(FadeKey::SelfDestruct), "last pass");
    }
}
