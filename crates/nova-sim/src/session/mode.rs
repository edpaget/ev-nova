//! The player's mode of play: flying, docked at a stellar, or jumping.
//!
//! The ship is never docked and jumping at once. Every way in to a
//! docking refuses a jump under way
//! ([`Session::land`](super::Session::land) gives
//! [`LandingRefusal::Jumping`](crate::landing::LandingRefusal::Jumping),
//! [`Session::land_at`](super::Session::land_at) gives
//! [`RelocateRefusal::InFlight`](super::RelocateRefusal::InFlight)), and a
//! jump starts only in flight
//! ([`Session::begin_jump`](super::Session::begin_jump) gives
//! [`JumpRefusal::Landed`](crate::hyperspace::JumpRefusal::Landed), and
//! the flight passes, which stage every other jump, stand still while
//! landed). The original keeps the two apart the same way: its L and J
//! keys are taken only while +0x50 <= 0 (@0x6a8df, @0x6b0da-0x6b10d), and
//! its flight loop does not run while docked in `_PlayerLandOnStellar`.
//!
//! The escape pod's countdown and the game's end stay beside the mode,
//! not in it. A jump outlives a deadly stellar's kill, as it does in the
//! original (`_HandleDeadlyStellars` @0x39da4-0x39e1f writes no +0x50),
//! and only [`Session::takes`](super::Session::takes), which the view
//! asks, keeps the pod from docking or jumping. The tests here pin each
//! of those transitions.

use crate::catalog::StellarId;

use super::Jump;

/// Whether the ship flies, is docked at a stellar or is jumping: one at a
/// time (see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) enum Mode {
    /// In flight, with no jump under way.
    #[default]
    Flying,
    /// Docked at the stellar.
    Landed(StellarId),
    /// The jump under way, from J being accepted until the ship arrives.
    Jumping(Jump),
}

impl Mode {
    /// The stellar the ship is docked at, if it has landed.
    pub(super) fn landed(self) -> Option<StellarId> {
        match self {
            Self::Landed(stellar) => Some(stellar),
            Self::Flying | Self::Jumping(_) => None,
        }
    }

    /// The jump under way, if any.
    pub(super) fn jump(self) -> Option<Jump> {
        match self {
            Self::Jumping(jump) => Some(jump),
            Self::Flying | Self::Landed(_) => None,
        }
    }

    /// Lifts off: a landed mode flies, and gives the stellar it was docked
    /// at; any other is left as it is, and gives none.
    pub(super) fn take_landed(&mut self) -> Option<StellarId> {
        let stellar = self.landed()?;
        *self = Self::Flying;
        Some(stellar)
    }

    /// Ends the jump under way, if any: a jumping mode flies; any other is
    /// left as it is.
    pub(super) fn end_jump(&mut self) {
        if let Self::Jumping(_) = self {
            *self = Self::Flying;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ai::NovaAi;
    use crate::catalog::{StellarId, SystemId};
    use crate::chance::NeverFires;
    use crate::combat::Rules;
    use crate::flight::Controls;
    use crate::geometry::Vec2;
    use crate::hyperspace::JumpRefusal;
    use crate::landing::LandingRefusal;
    use crate::session::tests::{OUT, bound_for_131};
    use crate::session::{Jump, RelocateRefusal, Session};
    use crate::testkit::{FakePilotCatalog, begin_jump_now, catalog, land_now};
    use crate::{POD_TICKS, PlayerCommand};

    use super::Mode;

    const DOCK: StellarId = StellarId(128);

    const WARP: Jump = Jump::Hyperspace {
        to: SystemId(131),
        warp: 7,
    };

    #[test]
    fn the_queries_read_each_variant() {
        assert_eq!(Mode::default(), Mode::Flying);
        assert_eq!(Mode::Flying.landed(), None);
        assert_eq!(Mode::Flying.jump(), None);
        assert_eq!(Mode::Landed(DOCK).landed(), Some(DOCK));
        assert_eq!(Mode::Landed(DOCK).jump(), None);
        assert_eq!(Mode::Jumping(WARP).landed(), None);
        assert_eq!(Mode::Jumping(WARP).jump(), Some(WARP));
    }

    #[test]
    fn taking_landed_from_landed_flies() {
        let mut mode = Mode::Landed(DOCK);
        assert_eq!(mode.take_landed(), Some(DOCK));
        assert_eq!(mode, Mode::Flying);
        assert_eq!(mode.take_landed(), None);
        assert_eq!(mode, Mode::Flying);
    }

    #[test]
    fn taking_landed_from_a_jump_leaves_it_jumping() {
        let mut mode = Mode::Jumping(WARP);
        assert_eq!(mode.take_landed(), None);
        assert_eq!(mode, Mode::Jumping(WARP));
    }

    #[test]
    fn ending_a_jump_flies() {
        let mut mode = Mode::Jumping(WARP);
        mode.end_jump();
        assert_eq!(mode, Mode::Flying);
        mode.end_jump();
        assert_eq!(mode, Mode::Flying);
    }

    #[test]
    fn ending_a_jump_leaves_a_landed_mode_landed() {
        let mut mode = Mode::Landed(DOCK);
        mode.end_jump();
        assert_eq!(mode, Mode::Landed(DOCK));
    }

    /// A session in the pre-jump stage, bound for 131: at rest facing up,
    /// it turns before it jumps.
    fn preparing(catalog: &FakePilotCatalog) -> Session {
        let mut session = bound_for_131(catalog, OUT, Vec2::ZERO, 0.0);
        assert_eq!(session.begin_jump(), Ok(SystemId(131)));
        assert!(
            matches!(session.mode.jump(), Some(Jump::PreJump { .. })),
            "{:?}",
            session.mode.jump()
        );
        session
    }

    /// A session in hyperspace, bound for 131.
    fn in_hyperspace(catalog: &FakePilotCatalog) -> Session {
        let mut session = bound_for_131(catalog, OUT, Vec2::ZERO, 0.0);
        assert_eq!(begin_jump_now(&mut session, &catalog), Ok(SystemId(131)));
        assert!(
            matches!(session.mode.jump(), Some(Jump::Hyperspace { .. })),
            "{:?}",
            session.mode.jump()
        );
        session
    }

    /// The two stages of a jump, each a session in it.
    const JUMPING: [fn(&FakePilotCatalog) -> Session; 2] = [preparing, in_hyperspace];

    /// A session docked at planet 128 in system 130.
    fn docked(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        session
    }

    #[test]
    fn a_land_press_in_a_jump_is_refused_and_leaves_it_jumping() {
        let catalog = catalog();
        for start in JUMPING {
            let mut session = start(&catalog);
            let jump = session.mode.jump();
            assert_eq!(session.land(), Err(LandingRefusal::Jumping));
            assert_eq!(session.mode.landed(), None);
            assert_eq!(session.mode.jump(), jump);
        }
    }

    #[test]
    fn land_at_in_a_jump_is_refused_and_leaves_it_jumping() {
        let catalog = catalog();
        for start in JUMPING {
            let mut session = start(&catalog);
            let jump = session.mode.jump();
            assert_eq!(
                session.land_at(&catalog, SystemId(130), StellarId(128)),
                Err(RelocateRefusal::InFlight)
            );
            assert_eq!(session.mode.landed(), None);
            assert_eq!(session.mode.jump(), jump);
        }
    }

    #[test]
    fn j_while_landed_is_refused_and_starts_no_jump() {
        let catalog = catalog();
        let mut session = docked(&catalog);
        session.plot_course(SystemId(131)).expect("a route");
        assert_eq!(session.begin_jump(), Err(JumpRefusal::Landed));
        assert_eq!(session.mode.landed(), Some(StellarId(128)));
        assert_eq!(session.mode.jump(), None);
    }

    #[test]
    fn a_landed_session_through_game_work_and_double_time_starts_no_jump() {
        let catalog = catalog();
        let mut session = docked(&catalog);
        session.plot_course(SystemId(131)).expect("a route");
        let thrust = Controls {
            thrust: true,
            ..Controls::default()
        };
        for pass in 0..5 {
            let rescued = session.game_work(
                thrust,
                &catalog,
                &NovaAi::default(),
                Rules::default(),
                &mut NeverFires,
            );
            assert!(!rescued, "pass {pass}");
            session.tick_again(
                thrust,
                &NovaAi::default(),
                Rules::default(),
                &mut NeverFires,
            );
            assert_eq!(session.mode.landed(), Some(StellarId(128)), "pass {pass}");
            assert_eq!(session.mode.jump(), None, "pass {pass}");
        }
    }

    #[test]
    fn a_landing_docks_with_no_jump() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        assert_eq!(session.mode.landed(), Some(StellarId(128)));
        assert_eq!(session.mode.jump(), None);
    }

    #[test]
    fn a_relocation_docks_with_no_jump() {
        let catalog = catalog();
        let mut moved = docked(&catalog);
        moved
            .relocate(&catalog, SystemId(131), StellarId(140))
            .expect("moves");
        assert_eq!(moved.mode.landed(), Some(StellarId(140)));
        assert_eq!(moved.mode.jump(), None);
        let mut landed = Session::start(&catalog).expect("starts");
        landed
            .land_at(&catalog, SystemId(131), StellarId(140))
            .expect("lands");
        assert_eq!(landed.mode.landed(), Some(StellarId(140)));
        assert_eq!(landed.mode.jump(), None);
    }

    #[test]
    fn a_take_off_flies_with_no_jump() {
        let catalog = catalog();
        let mut session = docked(&catalog);
        assert_eq!(
            session.take_off(&catalog, &mut NeverFires),
            Some(StellarId(128))
        );
        assert_eq!(session.mode.landed(), None);
        assert_eq!(session.mode.jump(), None);
    }

    #[test]
    fn an_arrival_ends_the_jump_and_does_not_dock() {
        let catalog = catalog();
        let mut session = in_hyperspace(&catalog);
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(131))
        );
        assert_eq!(session.mode.jump(), None);
        assert_eq!(session.mode.landed(), None);
    }

    /// Only the view's gate, [`Session::takes`] refusing
    /// [`PlayerCommand::Land`] in the pod, keeps a player from docking it;
    /// the walker's and the developer tools' landing docks it, its
    /// countdown still running.
    #[test]
    fn land_at_in_the_escape_pod_docks_and_the_pod_counts_on() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.pod = Some(POD_TICKS);
        assert!(!session.takes(PlayerCommand::Land), "the view's gate");
        assert_eq!(
            session.land_at(&catalog, SystemId(130), StellarId(128)),
            Ok(())
        );
        assert_eq!(session.mode.landed(), Some(StellarId(128)));
        assert_eq!(session.pod, Some(POD_TICKS));
    }

    /// Only the view's gate, [`Session::takes`] refusing
    /// [`PlayerCommand::Jump`] in the pod, keeps a player from jumping in
    /// it; J pressed past the gate stages a jump beside the pod.
    #[test]
    fn j_in_the_escape_pod_stages_a_jump_beside_it() {
        let mut session = bound_for_131(&catalog(), OUT, Vec2::ZERO, 0.0);
        session.pod = Some(POD_TICKS);
        assert!(!session.takes(PlayerCommand::Jump), "the view's gate");
        assert_eq!(session.begin_jump(), Ok(SystemId(131)));
        assert!(session.mode.jump().is_some());
        assert_eq!(session.pod, Some(POD_TICKS));
    }
}
