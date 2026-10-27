//! The player's tractor hold (see the
//! [`tractor_hold`](crate::combat::tractor_hold) module): what
//! `_HandlePlayer` does with its lock each frame, the pull's step every
//! held ship reads, and the clears a landing and an arrival make.
//!
//! **The frame's hold** ([`Session::hold_player`], `_HandlePlayer`
//! @0x68436-0x6862c): the very top of the function, before the escape
//! pod's return, on both passes. The release comes first, then the pull
//! toward its holder's velocity (rest when it holds itself) unless it was
//! released, then the age, on the advance pass alone, and the lapse.
//!
//! **The step** ([`PlayerShip::tractor_step`]): a quarter of slot 0's
//! `_ShipAccelRate`, for the player's hold and every NPC's
//! ([`World::tractor_step`](crate::traffic::World::tractor_step)): the
//! player's thrust as it flies, ionized and under the flight's divisor.
//!
//! **The held player is never slowed**: slot 0's branch of
//! `_ShipMaxSpeed`, `_ShipTurnRate` and `_ShipAccelRate` never reads the
//! lock. Its jump is refused ([`Session::held_by_other`],
//! [`hold_jump`](crate::hyperspace::hold_jump)).
//!
//! **The clears.** The original drops the player's lock only in
//! `_ResetPlayer` (@0x1d5d3), on a new pilot or a load, and the engine
//! never saves it. A landing and a jump's arrival leave it to lapse on the
//! wall-clock `TickCount`, and both take far longer than half a second,
//! so the engine drops it as a landing completes and as a jump arrives
//! ([`PlayerShip::clear_tractor`]), as the flight doesn't tick through
//! either.
//!
//! None of this plays a sound.

use super::{Jump, PlayerShip, Session, Setup};
use crate::combat::GamePass;
use crate::combat::ShipRef;
use crate::combat::hull::Condition;
use crate::combat::tractor_hold::{Holder, held_by_other, player_pass, step};
use crate::pilot::Pilot;

impl Session {
    /// Whether a ship other than the player holds it in a tractor beam
    /// ([`held_by_other`]): its jump is refused.
    #[must_use]
    pub fn held_by_other(&self) -> bool {
        self.ship.held_by_other()
    }

    /// The player's frame's hold on `pass` (see the module docs).
    pub(super) fn hold_player(&mut self, pass: GamePass) {
        let step = self
            .ship
            .tractor_step(&self.pilot, &self.setup, self.mode.jump());
        let npcs = self.space.traffic.npcs();
        player_pass(
            &mut self.ship.tractor,
            &mut self.ship.player.velocity,
            |ship| {
                let ShipRef::Npc(id) = ship else {
                    return Holder::Gone;
                };
                npcs.iter()
                    .find(|npc| npc.id == id)
                    .map_or(Holder::Gone, |npc| Holder::There {
                        velocity: npc.state.velocity,
                        intact: npc.condition == Condition::Intact,
                    })
            },
            step,
            pass != GamePass::Again,
        );
    }

    /// Sets the player's lock directly, for
    /// [`fixture::set_player_tractor`](crate::fixture::set_player_tractor).
    #[cfg(any(test, feature = "fixture"))]
    pub(crate) fn set_player_tractor(&mut self, lock: Option<crate::combat::impact::TractorLock>) {
        self.ship.set_player_tractor(lock);
    }
}

impl PlayerShip {
    /// The step a tractor's hold pulls a held ship's velocity by on each
    /// axis, the player's or an NPC's: a quarter of the player's thrust
    /// as it flies ([`step`]).
    pub(super) fn tractor_step(&self, pilot: &Pilot, setup: &Setup, jump: Option<Jump>) -> f32 {
        step(self.flying_handling(pilot, setup, jump).accel)
    }

    /// Whether a ship other than the player holds it in a tractor beam
    /// ([`held_by_other`]): its jump is refused.
    #[must_use]
    fn held_by_other(&self) -> bool {
        held_by_other(self.tractor, ShipRef::Player)
    }

    /// Sets the player's lock directly, for
    /// [`fixture::set_player_tractor`](crate::fixture::set_player_tractor).
    #[cfg(any(test, feature = "fixture"))]
    fn set_player_tractor(&mut self, lock: Option<crate::combat::impact::TractorLock>) {
        self.tractor = lock;
    }

    /// Drops the player's lock, as a landing completes and as a jump
    /// arrives (see the module docs).
    pub(super) fn clear_tractor(&mut self) {
        self.tractor = None;
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::{StellarId, SystemId};
    use crate::combat::Rules;
    use crate::combat::armament::Trigger;
    use crate::combat::impact::TractorLock;
    use crate::flight::Controls;
    use crate::geometry::Vec2;
    use crate::session::tests::{OUT, armed, bound_for_131, facing_an_npc, knocking, tractor_beam};
    use crate::testkit::catalog;
    use crate::testkit::{work, work_with};
    use crate::traffic::npc::NpcId;
    use nova_sim_base::NeverFires;

    const NPC: ShipRef = ShipRef::Npc(NpcId(0));

    /// `armed()`'s session with the player at rest at the centre, held by
    /// NPC 0, which moves at `velocity`.
    fn held(velocity: Vec2) -> Session {
        let mut session = facing_an_npc(&armed(), 180);
        session.ship.settle_box(&session.pilot, &session.setup);
        session.ship.player.position = Vec2::ZERO;
        session.ship.player.velocity = Vec2::ZERO;
        session.space.traffic.npcs_mut()[0].state.velocity = velocity;
        session.ship.tractor = Some(TractorLock::by(NPC));
        session
    }

    #[test]
    fn the_player_is_pulled_toward_its_holders_velocity_by_a_quarter_of_its_thrust() {
        let mut session = held(Vec2::new(-2.0, 2.0));
        let step = session
            .ship
            .tractor_step(&session.pilot, &session.setup, session.mode.jump());
        assert_eq!(
            step,
            session
                .ship
                .flying_handling(&session.pilot, &session.setup, session.mode.jump())
                .accel
                * 0.25
        );
        assert!(step > 0.0);
        work(
            &mut session,
            &crate::testkit::catalog(),
            Controls::default(),
        );
        assert_eq!(session.player().velocity, Vec2::new(-step, step));
        assert_eq!(
            session.tractor().map(|lock| lock.ticks_since_touch),
            Some(1)
        );
    }

    #[test]
    fn both_holds_step_by_the_sessions_step_under_a_divisor_and_ionized() {
        use crate::ai::NovaAi;
        use crate::flight::DISABLED_DRAG;
        use crate::rulebook::{RuleValue, Rulebook};
        let divided = Rulebook::default().with_value(
            RuleValue::ThrustDivisor,
            core::num::NonZeroU32::new(3000).expect("not 0"),
        );
        let catalog = armed();
        let mut session = held(Vec2::new(-5.0, 0.0)).with_rules(divided);
        session.ship.fit.stats.ion.class_capacity = 400.0;
        session.ship.ion.charge = 200.0;
        let step = session
            .ship
            .tractor_step(&session.pilot, &session.setup, session.mode.jump());
        assert_eq!(
            step,
            session
                .ship
                .flying_handling(&session.pilot, &session.setup, session.mode.jump())
                .accel
                * 0.25
        );
        // What the NPCs see of the player's thrust skips the divisor.
        let seen = session.player_side().handling.accel * 0.25;
        assert!(step > seen * 1.5, "{step} against {seen}");
        work(&mut session, &catalog, Controls::default());
        assert_eq!(session.player().velocity, Vec2::new(-step, 0.0));
        // NPC 0, held by the player, disabled so it drifts.
        let npc = &mut session.space.traffic.npcs_mut()[0];
        npc.tractor = Some(TractorLock::by(ShipRef::Player));
        npc.condition = Condition::Disabled;
        work_with(
            &mut session,
            &catalog,
            Controls::default(),
            &NovaAi::default(),
            Rules::default(),
            &mut NeverFires,
        );
        // The step as the pass's ion block (`_HandlePlayer` @0x6d7fd) left
        // the charge, before the ship loop (@0x4445c-0x444c4) moves NPC 0.
        let step = session
            .ship
            .tractor_step(&session.pilot, &session.setup, session.mode.jump());
        assert_eq!(
            session.npcs()[0].state.velocity,
            Vec2::new(-5.0 + step, 0.0) * DISABLED_DRAG
        );
    }

    #[test]
    fn the_second_pass_pulls_but_does_not_age() {
        let mut session = held(Vec2::new(-2.0, 2.0));
        let step = session
            .ship
            .tractor_step(&session.pilot, &session.setup, session.mode.jump());
        session.tick_again(
            Controls::default(),
            &crate::ai::NovaAi::default(),
            Rules::default(),
            &mut NeverFires,
        );
        assert_eq!(session.player().velocity, Vec2::new(-step, step));
        assert_eq!(
            session.tractor().map(|lock| lock.ticks_since_touch),
            Some(0)
        );
    }

    #[test]
    fn the_player_holding_itself_brakes_toward_rest() {
        let mut session = held(Vec2::ZERO);
        session.ship.tractor = Some(TractorLock::by(ShipRef::Player));
        session.ship.player.velocity = Vec2::new(1.0, -1.0);
        let step = session
            .ship
            .tractor_step(&session.pilot, &session.setup, session.mode.jump());
        work(
            &mut session,
            &crate::testkit::catalog(),
            Controls::default(),
        );
        assert_eq!(session.player().velocity, Vec2::new(1.0 - step, step - 1.0));
        assert!(!session.held_by_other());
    }

    #[test]
    fn the_escape_pod_is_pulled_too() {
        let mut session = held(Vec2::new(-2.0, 0.0));
        session.pod = Some(100);
        let step = session
            .ship
            .tractor_step(&session.pilot, &session.setup, session.mode.jump());
        work(
            &mut session,
            &crate::testkit::catalog(),
            Controls::default(),
        );
        assert_eq!(session.pod, Some(99), "the pod flew");
        assert_eq!(session.player().velocity.x, -step);
    }

    #[test]
    fn a_held_player_is_not_slowed() {
        let mut session = held(Vec2::ZERO);
        let handling =
            session
                .ship
                .flying_handling(&session.pilot, &session.setup, session.mode.jump());
        let top = session.top_speed();
        assert!(session.held_by_other());
        session.ship.tractor = None;
        assert_eq!(
            session
                .ship
                .flying_handling(&session.pilot, &session.setup, session.mode.jump()),
            handling
        );
        assert_eq!(session.top_speed(), top);
    }

    #[test]
    fn the_player_is_released_unpulled_when_its_holder_is_disabled_or_dying_or_leaves() {
        let dying = Condition::Dying {
            ticks_left: 100,
            starting: false,
        };
        for condition in [Condition::Disabled, dying] {
            let mut session = held(Vec2::new(-2.0, 2.0));
            session.space.traffic.npcs_mut()[0].condition = condition;
            work(
                &mut session,
                &crate::testkit::catalog(),
                Controls::default(),
            );
            assert_eq!(session.tractor(), None, "{condition:?}");
            assert_eq!(session.player().velocity, Vec2::ZERO, "{condition:?}");
        }
        let mut gone = held(Vec2::new(-2.0, 2.0));
        gone.ship.tractor = Some(TractorLock::by(ShipRef::Npc(NpcId(7))));
        work(&mut gone, &crate::testkit::catalog(), Controls::default());
        assert_eq!(gone.tractor(), None);
        assert_eq!(gone.player().velocity, Vec2::ZERO);
    }

    #[test]
    fn a_real_beams_hold_lapses_15_ticks_after_its_last_touch() {
        let catalog = armed();
        let mut session = knocking(&catalog, (None, 100.0), (Some(tractor_beam()), 200.0));
        session.space.traffic.npcs_mut()[0].trigger = Trigger::default();
        session.space.traffic.npcs_mut()[0].trigger.primary = true;
        let mut last_touch = None;
        let mut lapsed = None;
        for tick in 0..60 {
            if tick == 5 {
                session.space.traffic.npcs_mut()[0].trigger = Trigger::default();
            }
            work(&mut session, &catalog, Controls::default());
            if session
                .tractor()
                .is_some_and(|lock| lock.ticks_since_touch == 0)
            {
                last_touch = Some(tick);
            }
            if last_touch.is_some() && session.tractor().is_none() {
                lapsed = Some(tick);
                break;
            }
        }
        let last_touch = last_touch.expect("touched");
        // `_HandlePlayer`'s top lets the hold go on the 15th pass after
        // the touch's.
        assert_eq!(
            lapsed,
            Some(last_touch + 15),
            "the 15th pass after the touch's"
        );
    }

    #[test]
    fn a_landing_clears_the_lock() {
        let mut session = held(Vec2::ZERO);
        session.dock(StellarId(128));
        assert_eq!(session.tractor(), None);
    }

    #[test]
    fn a_jumps_arrival_clears_the_lock() {
        let catalog = catalog();
        let mut session = bound_for_131(&catalog, OUT, Vec2::ZERO, 90.0);
        session.ship.tractor = Some(TractorLock::by(ShipRef::Player));
        session.begin_jump().expect("jumps");
        while session.jumping().is_none() {
            work(&mut session, &catalog, Controls::default());
        }
        session.ship.tractor = Some(TractorLock::by(ShipRef::Player));
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(131))
        );
        assert_eq!(session.tractor(), None);
    }

    // The jump.

    /// A session on course for 131, out far enough to jump, its lock
    /// `lock`.
    fn bound(catalog: &crate::testkit::FakePilotCatalog, lock: Option<TractorLock>) -> Session {
        let mut session = bound_for_131(catalog, OUT, Vec2::ZERO, 90.0);
        session.ship.tractor = lock;
        session.take_sounds();
        session
    }

    #[test]
    fn a_held_players_jump_is_refused_with_the_refusal_beep() {
        use crate::hyperspace::JumpRefusal;
        use crate::sound::SimSound;
        let catalog = catalog();
        let mut session = bound(&catalog, Some(TractorLock::by(NPC)));
        assert_eq!(session.begin_jump(), Err(JumpRefusal::Tractored));
        assert_eq!(session.take_sounds(), [SimSound::Refused]);
        assert_eq!(session.preparing_jump(), None);
        // Too close is told as the tractor too.
        session.ship.player.position = Vec2::ZERO;
        assert_eq!(session.begin_jump(), Err(JumpRefusal::Tractored));
        assert_eq!(session.take_sounds(), [SimSound::Refused]);
    }

    #[test]
    fn a_player_holding_itself_jumps() {
        let catalog = catalog();
        let mut session = bound(&catalog, Some(TractorLock::by(ShipRef::Player)));
        assert_eq!(session.begin_jump(), Ok(SystemId(131)));
    }

    #[test]
    fn j_again_in_the_pre_jump_after_a_hold_gives_the_same_system() {
        let catalog = catalog();
        let mut session = bound(&catalog, None);
        assert_eq!(session.begin_jump(), Ok(SystemId(131)));
        session.ship.tractor = Some(TractorLock::by(NPC));
        assert_eq!(session.begin_jump(), Ok(SystemId(131)));
    }

    #[test]
    fn a_jump_in_hyperspace_carries_on_whatever_holds_the_ship() {
        use crate::hyperspace::JumpRefusal;
        use crate::sound::SimSound;
        let catalog = catalog();
        let mut session = bound(&catalog, None);
        session.begin_jump().expect("jumps");
        while session.jumping().is_none() {
            work(&mut session, &catalog, Controls::default());
        }
        session.ship.tractor = Some(TractorLock::by(NPC));
        session.take_sounds();
        assert_ne!(session.begin_jump(), Err(JumpRefusal::Tractored));
        assert!(!session.take_sounds().contains(&SimSound::Refused));
    }

    #[test]
    fn the_nav_area_shows_a_held_players_jump_clear() {
        use crate::hyperspace::JumpReadiness;
        let catalog = catalog();
        let session = bound(&catalog, Some(TractorLock::by(NPC)));
        assert_eq!(session.jump_readiness(), JumpReadiness::Clear);
    }
}
