//! The explosions a set expression makes and kills (`_EvalSetExp`, in
//! `EV Nova.app`'s `EV Nova`, i386).
//!
//! **`Y`'s explosion.** `Y` explodes a stellar with
//! `_CreateExplosion(x, y, type, 0, 1)` (@0x15a99-0x15ab4), which plays
//! the explosion's sound at once (@0x3f8ca-0x3f91a). The session raises
//! that sound itself, docked or not, so the explosion it reports here,
//! [`CombatEvent::ExplodedHeard`] ([`Combat::explode_heard`]), is drawn
//! with no sound of its own.
//!
//! **The kill.** An in-flight `M` or `N`, or `N`'s tail out of range, sets
//! `_killExplods` (@0x158ba, @0x1597e), which lasts to the end of the
//! `_PlayGame` frame: that frame's `_HandleExplods` hides and frees every
//! explosion live (@0x2f5ff-0x2f607, @0x2f844-0x2f85e) before the sprite
//! world is drawn. [`Combat::kill_explosions`] drops the explosions the
//! fight has reported and the view has not taken yet, and a ship's
//! destruction keeps its other effects without its explosion. What the
//! view has drawn already it clears as it lays out the scene the move
//! enters.

use super::{Combat, CombatEvent};
use crate::catalog::Explosion;
use crate::geometry::Vec2;

impl Combat {
    /// Reports `explosion` at `at`, of size 0, its sound raised already by
    /// the session: a `Y`'s (see the module docs).
    pub fn explode_heard(&mut self, at: Vec2, explosion: Explosion) {
        self.events
            .push(CombatEvent::ExplodedHeard { at, explosion });
    }

    /// Drops every explosion reported and not yet taken (see the module
    /// docs): each [`CombatEvent::Exploded`], [`CombatEvent::ExplodedHeard`],
    /// [`CombatEvent::ShotDown`] and [`CombatEvent::Fizzled`], each type-0
    /// explosion too, and a [`CombatEvent::Destroyed`]'s explosion, the
    /// ship's destruction kept.
    pub fn kill_explosions(&mut self) {
        self.events.retain_mut(|event| match event {
            CombatEvent::Exploded { .. }
            | CombatEvent::ExplodedHeard { .. }
            | CombatEvent::ShotDown { .. }
            | CombatEvent::Fizzled { .. } => false,
            CombatEvent::Destroyed { explosion, .. } => {
                *explosion = None;
                true
            }
            _ => true,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{ShipId, WeaponId};
    use crate::combat::ShipRef;

    fn boom(kind: i16) -> Explosion {
        Explosion::decode(kind).expect("a bööm")
    }

    #[test]
    fn a_ys_explosion_is_reported_heard_already_of_size_0() {
        let mut combat = Combat::default();
        combat.explode_heard(Vec2::new(30.0, -40.0), boom(4));
        assert_eq!(
            combat.take_events(),
            [CombatEvent::ExplodedHeard {
                at: Vec2::new(30.0, -40.0),
                explosion: boom(4),
            }]
        );
    }

    #[test]
    fn a_kill_drops_every_explosion_not_yet_taken_and_keeps_the_rest() {
        let mut combat = Combat::default();
        let destroyed = |explosion| CombatEvent::Destroyed {
            ship: ShipRef::Player,
            ship_type: ShipId(128),
            at: Vec2::ZERO,
            velocity: Vec2::ZERO,
            explosion,
            size: 10.0,
        };
        let fired = CombatEvent::Fired {
            ship: ShipRef::Player,
            weapon: WeaponId(128),
            at: Vec2::ZERO,
            sound: None,
        };
        combat.events = vec![
            CombatEvent::Exploded {
                at: Vec2::ZERO,
                explosion: boom(2),
                size: 5.0,
            },
            fired,
            CombatEvent::ExplodedHeard {
                at: Vec2::ZERO,
                explosion: boom(4),
            },
            CombatEvent::ShotDown { at: Vec2::ZERO },
            CombatEvent::Fizzled { at: Vec2::ZERO },
            destroyed(Some(boom(3))),
        ];
        combat.kill_explosions();
        assert_eq!(combat.take_events(), [fired, destroyed(None)]);
        combat.kill_explosions();
        assert_eq!(combat.take_events(), [], "nothing left");
    }
}
