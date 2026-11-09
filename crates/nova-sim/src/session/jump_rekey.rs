//! J pressed again once the jump has begun (`Jump::Hyperspace`): the jump
//! is committed, so [`Session::begin_jump`](super::Session::begin_jump) gives the system bound for and
//! changes nothing, as it does during the pre-jump stage.

#[cfg(test)]
mod tests {
    use crate::catalog::SystemId;
    use crate::flight::Controls;
    use crate::geometry::Vec2;
    use crate::session::Jump;
    use crate::session::tests::{OUT, bound_for_131};
    use crate::testkit::{begin_jump_now, catalog, work};

    #[test]
    fn j_in_hyperspace_gives_the_system_and_changes_nothing() {
        let catalog = catalog();
        let mut session = bound_for_131(&catalog, OUT, Vec2::ZERO, 0.0);
        begin_jump_now(&mut session, &catalog).expect("accepted");
        assert!(matches!(session.mode.jump(), Some(Jump::Hyperspace { .. })));
        let mut ticks = 0;
        while session
            .ship
            .warp
            .warp_ups
            .playing(session.ship.warp.double_time)
            && ticks < 1000
        {
            work(&mut session, &catalog, Controls::default());
            ticks += 1;
        }
        if ticks == 0 {
            for _ in 0..3 {
                work(&mut session, &catalog, Controls::default());
            }
        }
        assert_eq!(session.jumping(), Some(SystemId(131)), "still in the jump");
        session.take_sounds();
        let before = session.mode.jump();
        let clock = session.warp_clock();
        let warp = session.ship.warp.warp;
        let accepted = session.ship.warp.jump_accepted;

        assert_eq!(session.begin_jump(), Ok(SystemId(131)));

        assert_eq!(session.mode.jump(), before);
        assert_eq!(session.warp_clock(), clock);
        assert_eq!(session.ship.warp.warp, warp);
        assert_eq!(session.ship.warp.jump_accepted, accepted);
        assert_eq!(session.jumping(), Some(SystemId(131)));
        assert_eq!(session.preparing_jump(), None);
        assert_eq!(session.take_sounds(), vec![]);
    }
}
