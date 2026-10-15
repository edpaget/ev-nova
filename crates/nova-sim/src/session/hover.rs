//! The session's hover test (see the [`hover`](crate::hover) module): the
//! ship under the pointer, then, with none and no jump under way, the
//! stellar.

use super::Session;
use crate::combat::hull::Condition;
use crate::geometry::Vec2;
use crate::hover::{DrawnShips, Hover, ship_under};
use crate::navigation::stellar_under;

impl Session {
    /// What the pointer at `at`, in system coordinates, is over among the
    /// things the player can target, the ships boxed as `drawn` draws them
    /// (`_HandleMouse` @0x380e3-0x3828d): a ship ([`ship_under`], the
    /// player's own only while it has a target and, past the `_IsDying`
    /// skip @0x380f7 that comes before the slot-0 target rule
    /// @0x381c3-0x381d5, is intact or disabled), or else, unless a jump is
    /// under way, the pre-jump stage included (the player's +0x50 > 0,
    /// @0x381f2), the stellar a click there would pick ([`stellar_under`]).
    #[must_use]
    pub fn hover(&self, at: Vec2, drawn: &impl DrawnShips) -> Option<Hover> {
        ship_under(
            self.npcs(),
            self.scanner(),
            self.target.is_some()
                && matches!(self.condition, Condition::Intact | Condition::Disabled),
            at,
            drawn,
        )
        .or_else(|| {
            self.jump.is_none().then(|| {
                stellar_under(&self.sites, at, |site| self.drawn_frame_size(site))
                    .map(Hover::Stellar)
            })?
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{StellarId, SystemId};
    use crate::hover::DrawnBox;
    use crate::session::Jump;
    use crate::stats::ShipStats;
    use crate::testkit::{catalog, npc};
    use crate::traffic::npc::NpcId;

    /// Every ship drawn in a 60-pixel square at `at`.
    struct AllAt(Vec2);

    impl DrawnShips for AllAt {
        fn npc(&self, _id: NpcId) -> Option<DrawnBox> {
            Some(self.square())
        }

        fn player(&self) -> Option<DrawnBox> {
            Some(self.square())
        }
    }

    impl AllAt {
        fn square(&self) -> DrawnBox {
            DrawnBox {
                centre: self.0,
                width: 60,
                height: 60,
            }
        }
    }

    /// Planet 128, at (30, -40), 100 square.
    const OVER_128: Vec2 = Vec2::new(30.0, -40.0);

    #[test]
    fn with_no_ship_under_the_pointer_a_stellar_is_hovered_but_not_during_a_jump() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        let elsewhere = AllAt(Vec2::new(5000.0, 5000.0));
        assert_eq!(
            session.hover(OVER_128, &elsewhere),
            Some(Hover::Stellar(StellarId(128)))
        );
        assert_eq!(session.hover(Vec2::new(500.0, 500.0), &elsewhere), None);
        session.jump = Some(Jump::PreJump {
            to: SystemId(131),
            bearing: None,
            warp: None,
        });
        assert_eq!(session.hover(OVER_128, &elsewhere), None, "pre-jump");
        session.jump = Some(Jump::Hyperspace {
            to: SystemId(131),
            warp: 0,
        });
        assert_eq!(session.hover(OVER_128, &elsewhere), None, "jumping");
    }

    #[test]
    fn a_ship_under_the_pointer_comes_before_the_stellar() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        let id = session.traffic.add_npc(npc(0, ShipStats::default()));
        assert_eq!(
            session.hover(OVER_128, &AllAt(OVER_128)),
            Some(Hover::Ship(id))
        );
    }

    #[test]
    fn the_players_ship_is_hovered_while_it_holds_a_target() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        let far = Vec2::new(-5000.0, 0.0);
        assert_eq!(session.hover(far, &AllAt(far)), None);
        session.target = Some(NpcId(99));
        assert_eq!(session.hover(far, &AllAt(far)), Some(Hover::Player));
    }

    #[test]
    fn a_dying_or_destroyed_players_ship_is_not_hovered_though_it_holds_a_target() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        let far = Vec2::new(-5000.0, 0.0);
        session.target = Some(NpcId(99));
        session.condition = Condition::Disabled;
        assert_eq!(session.hover(far, &AllAt(far)), Some(Hover::Player));
        session.condition = Condition::Dying {
            ticks_left: 3,
            starting: false,
        };
        assert_eq!(session.hover(far, &AllAt(far)), None, "dying");
        session.condition = Condition::Destroyed;
        assert_eq!(session.hover(far, &AllAt(far)), None, "destroyed");
    }
}
