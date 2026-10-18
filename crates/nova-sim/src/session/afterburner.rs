//! The player's afterburner (see the [`afterburner`](crate::afterburner)
//! module): `_HandlePlayer`'s engage test before the thrust
//! (@0x6cad4-0x6cb59), its 1.8 times top speed for the thrust while on
//! (@0x6cc43-0x6cc80), and its block after the move (@0x6f970-0x6fd54),
//! which pushes the ship, drains the fuel and writes `_playerMaxVel`, the
//! box the player's velocity is clamped to on the next tick.
//!
//! **Which ticks.** Every `_HandlePlayer` frame reaches the block after
//! the move, intact, disabled or breaking up, the escape-pod eject's four
//! ways out of its branch included (@0x6f239, @0x6f2c5, @0x6f2e0,
//! @0x6f308), but these, which return first: the dead branch (@0x68853,
//! @0x6888c), the escape pod's countdown (@0x68ddd), the landing
//! (@0x692ce) and a jump's arrival (@0x6c625). So the box bleeds through
//! the pre-jump and the warp's run in hyperspace, and is left alone while
//! the player is destroyed or the pod flies. Every frame that reaches the
//! block passes the engage test, which clears `_abOn` first (@0x6cae3);
//! the engine runs the test on a destroyed player's ticks too, which
//! clears it there (a destroyed ship is not intact) where the original
//! leaves it: nothing can tell, as a destroyed ship is never pushed.
//!
//! **What reads it.** `_HandlePlayer` runs before `_HandleShot`
//! (`_DoPlayGameWork` @0x43e19, @0x44385), so `_DamageShip`'s knockback
//! reads this frame's `_abOn`, as the fight here, after
//! [`Session::tick`], reads [`Session::afterburner_on`].
//!
//! **Left out.** `_HasAfterburner`'s cheats (the engine has none); the
//! outfit caches (`_playerHasAfterburner` *0x3b514c and the fuel usage's,
//! reset in `_ResetPlayerPrecalcedValues` @0xc3ac, @0xc419), which the
//! stats, worked out afresh, need no copy of; the status bar's redraw
//! every 4th frame (`_statusRedraw` +4, @0x6fc4d-0x6fc5c), as the bar
//! draws from the state each frame; and an NPC's afterburner, which is
//! `combat-ai-fidelity` phase 7's. The original's box is static data, so
//! it survives into a second pilot loaded in the same run of the app,
//! while the engine's starts at none with each session: no one can see
//! the difference, as a pilot is loaded landed and leaves at rest.
//!
//! None of these blocks plays a sound.

use super::Session;
use crate::afterburner::{afterburner_top, drain, engages, push_speed, push_velocity};
use crate::combat::hull::Condition;
use crate::flight::{Caps, Controls};

impl Session {
    /// Whether any gravity source is in the system, `_gravitySourcesPresent`
    /// (*0x3b5568), which keeps the afterburner from widening the thrust's
    /// top, the box and the knockback's cap. None in the stock data; it is
    /// `ship-handling-fidelity/phase-14-gravity-and-deadly-stellars`' to
    /// work out, in this body alone.
    #[allow(clippy::unused_self)]
    fn gravity_sources_present(&self) -> bool {
        false
    }

    /// Whether the afterburner widens the player's caps this tick: it is on
    /// and no gravity source is present. The thrust's top
    /// ([`Session::player_caps`]) and the knockback's cap
    /// ([`Session::afterburner_knockback`]) both read it.
    fn afterburner_widens(&self) -> bool {
        self.afterburner_on && !self.gravity_sources_present()
    }

    /// Whether the player's afterburner is on this tick, `_abOn`, as
    /// [`Session::tick`] last engaged it: what the fight's knockback reads
    /// (see the module docs).
    #[must_use]
    pub fn afterburner_on(&self) -> bool {
        self.afterburner_on
    }

    /// Whether a push on the player this tick is clamped to 1.8 times its
    /// top speed, `_DamageShip`'s test (@0x3ab19-0x3ab68): its afterburner
    /// is on and no gravity source is present.
    pub(super) fn afterburner_knockback(&self) -> bool {
        self.afterburner_widens()
    }

    /// The afterburner's engage test for this tick under `controls`
    /// (@0x6cad4-0x6cb59; [`engages`]): the key held, the ship intact,
    /// carrying an afterburner, out of a jump and with the fuel for it.
    /// Its press counts as a flight key whatever the outcome, which the
    /// view's cursor reads (*0x3b5550, @0x6cb03).
    pub(super) fn engage_afterburner(&mut self, controls: Controls) {
        self.afterburner_on = engages(
            controls.afterburner,
            self.condition == Condition::Intact,
            self.stats.afterburner,
            self.jump.is_some(),
            self.pilot.reserves.fuel.now,
        );
    }

    /// What the player's tick is capped by ([`Caps`]): its thrust tops out
    /// at 1.8 times its top speed while the afterburner is on and no
    /// gravity source is present (@0x6cc43-0x6cc80), and its velocity is
    /// clamped to the box the last tick left (@0x6d43f-0x6d4aa).
    pub(super) fn player_caps(&self) -> Caps {
        let top = self.top_speed();
        Caps {
            thrust_top: if self.afterburner_widens() {
                afterburner_top(top)
            } else {
                top
            },
            max_vel: self.player_max_vel.get(),
        }
    }

    /// The afterburner's block after the move (@0x6f970-0x6fd3c; see the
    /// [`afterburner`](crate::afterburner) module): while it is on, the box
    /// widens, the ship is pushed (its speed if it is inertialess, else its
    /// velocity) and the fuel drains; off, the box bleeds back. The glow's
    /// step is ramped with the thrust's (see the [`glow`](crate::glow)
    /// module).
    pub(super) fn burn_afterburner(&mut self) {
        let handling = self.flying_handling();
        let (top, accel) = (handling.max_speed, handling.accel);
        if !self.afterburner_on {
            self.player_max_vel.bleed(top, accel);
            return;
        }
        let gravity = self.gravity_sources_present();
        self.player_max_vel.burn(top, gravity);
        if self.player_inertialess() {
            self.player_speed = push_speed(self.player_speed, accel, top);
        } else {
            self.player.velocity =
                push_velocity(self.player.velocity, self.player.heading, accel, top);
        }
        let usage = self.stats.afterburner.unwrap_or_default();
        let fuel = &mut self.pilot.reserves.fuel;
        fuel.now = drain(fuel.now, usage);
    }
    /// Settles the box at the top speed, as a session's first tick does:
    /// for a test that flies a new session's first tick as any other.
    #[cfg(test)]
    pub(crate) fn settle_box(&mut self) {
        self.player_max_vel.bleed(self.top_speed(), 0.0);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::{OutfitId, ShipId, SystemId};
    use crate::flight::{Face, Turn, facing};
    use crate::geometry::Vec2;
    use crate::glow::GLOW_AFTERBURNER;
    use crate::handling::ShipFields;
    use crate::stats::{AFTERBURNER, INERTIALESS_HULL};
    use crate::testkit::{FAST, FakePilotCatalog, catalog, outfit};
    use crate::{GLOW_CRUISE, RuleKey, RuleSource, Rulebook};

    /// The afterburner's `ModVal`: 0.2997 fuel a tick, so the fake ship's
    /// 300 last 1000 ticks.
    const MOD_VAL: i16 = 9;

    /// The fake catalog with ship 128 carrying an afterburner of
    /// [`MOD_VAL`] as a default item.
    fn burner() -> FakePilotCatalog {
        FakePilotCatalog {
            defaults: vec![(ShipId(128), vec![(OutfitId(197), 1)])],
            outfits: vec![outfit(197, &[(AFTERBURNER, MOD_VAL)])],
            ..catalog()
        }
    }

    /// [`burner`] with ship 128's hull inertialess.
    fn inertialess() -> FakePilotCatalog {
        FakePilotCatalog {
            ships: vec![(
                ShipId(128),
                Ok(ShipFields {
                    flags2: INERTIALESS_HULL,
                    ..FAST
                }),
            )],
            ..burner()
        }
    }

    /// The fuel the afterburner uses a tick.
    fn usage() -> f32 {
        (f64::from(MOD_VAL) * 0.0333) as f32
    }

    /// A session of `catalog` at rest at the centre facing `heading`,
    /// past its first tick, whose box of none holds it at rest.
    fn flying(catalog: &FakePilotCatalog, heading: f32) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.tick(Controls::default());
        session.player.position = Vec2::ZERO;
        session.player.velocity = Vec2::ZERO;
        session.player.heading = heading;
        session
    }

    fn held(thrust: bool, afterburner: bool) -> Controls {
        Controls {
            thrust,
            turn: Turn::None,
            reverse: false,
            face: Face::Off,
            afterburner,
        }
    }

    fn near(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    fn fuel(session: &Session) -> f32 {
        session.pilot.reserves.fuel.now
    }

    /// How far the ship moves in one tick under `controls`, from the
    /// centre, where the position keeps its precision.
    fn moved(session: &mut Session, controls: Controls) -> Vec2 {
        session.player.position = Vec2::ZERO;
        session.tick(controls);
        session.player.position
    }

    /// `ticks` ticks under `controls`.
    fn run(session: &mut Session, controls: Controls, ticks: usize) {
        for _ in 0..ticks {
            session.tick(controls);
        }
    }

    // Engaging.

    #[test]
    fn the_afterburner_is_on_while_its_key_is_held_with_one_carried() {
        let mut session = flying(&burner(), 90.0);
        assert!(!session.afterburner_on());
        session.tick(held(false, true));
        assert!(session.afterburner_on());
        session.tick(held(false, false));
        assert!(!session.afterburner_on(), "let go");
        let mut none = flying(&catalog(), 90.0);
        none.tick(held(true, true));
        assert!(!none.afterburner_on(), "none carried");
    }

    #[test]
    fn no_afterburner_without_fuel_for_it_disabled_dying_or_jumping() {
        let mut empty = flying(&burner(), 90.0);
        empty.pilot.reserves.fuel.now = 0.0;
        empty.tick(held(true, true));
        assert!(!empty.afterburner_on(), "no fuel");
        let mut short = flying(&burner(), 90.0);
        short.pilot.reserves.fuel.now = usage() * 0.5;
        short.tick(held(true, true));
        assert!(!short.afterburner_on(), "below the usage");
        assert_eq!(fuel(&short), usage() * 0.5, "none spent");
        let dying = Condition::Dying {
            ticks_left: 100,
            starting: false,
        };
        for condition in [Condition::Disabled, dying] {
            let mut downed = flying(&burner(), 90.0);
            downed.condition = condition;
            downed.tick(held(true, true));
            assert!(!downed.afterburner_on(), "{condition:?}");
        }
        let mut jumping = super::super::tests::bound_for_131(
            &burner(),
            super::super::tests::OUT,
            Vec2::ZERO,
            0.0,
        );
        jumping.tick(Controls::default());
        assert_eq!(jumping.begin_jump(), Ok(SystemId(131)));
        let full = fuel(&jumping);
        jumping.tick(held(true, true));
        assert!(!jumping.afterburner_on(), "in the pre-jump");
        assert!(fuel(&jumping) <= full, "{}", fuel(&jumping));
    }

    #[test]
    fn a_destroyed_players_afterburner_is_off() {
        let mut session = flying(&burner(), 90.0);
        session.tick(held(false, true));
        assert!(session.afterburner_on());
        session.condition = Condition::Destroyed;
        session.tick(held(false, true));
        assert!(!session.afterburner_on());
    }

    #[test]
    fn the_fuel_drains_to_none_and_the_afterburner_stops_the_tick_after() {
        let mut session = flying(&burner(), 90.0);
        let full = fuel(&session);
        session.tick(held(false, true));
        assert!(near(fuel(&session), full - usage()), "{}", fuel(&session));
        session.pilot.reserves.fuel.now = usage() * 1.5;
        session.tick(held(false, true));
        assert!(session.afterburner_on());
        assert!(near(fuel(&session), usage() * 0.5));
        session.tick(held(false, true));
        assert!(!session.afterburner_on(), "below the usage");
        session.pilot.reserves.fuel.now = usage();
        session.tick(held(false, true));
        assert!(session.afterburner_on(), "at the usage");
        assert_eq!(fuel(&session), 0.0);
        session.tick(held(false, true));
        assert!(!session.afterburner_on());
    }

    // The speed.

    #[test]
    fn thrust_and_afterburner_reach_1_8_times_top_speed_on_an_axis_a_tick_behind_the_box() {
        let mut session = flying(&burner(), 90.0);
        let top = session.top_speed();
        session.player.velocity = Vec2::new(top, 0.0);
        session.tick(held(true, true));
        assert!(
            near(
                session.player.velocity.x,
                top + session.flying_handling().accel * 2.75
            ),
            "clamped to the old box, then pushed: {:?}",
            session.player
        );
        let first = session.player.position.x;
        assert!(near(first, top), "moved at the old box: {first}");
        run(&mut session, held(true, true), 100);
        let burnt = afterburner_top(top);
        let step = moved(&mut session, held(true, true));
        assert!(near(step.x, burnt), "{step:?}");
        assert!(step.y.abs() < 1e-3);
        // Pushed past it after the move, it is damped for the next tick,
        // whose thrust takes it back to the box.
        assert!(
            near(session.player.velocity.x, burnt * 0.99),
            "{:?}",
            session.player
        );
        assert!(near(moved(&mut session, held(true, true)).x, burnt));
    }

    #[test]
    fn on_a_diagonal_both_axes_reach_1_8_times_their_share() {
        let mut session = flying(&burner(), 45.0);
        run(&mut session, held(true, true), 150);
        // Each axis's share of the afterburner's top is below the box, so
        // nothing clamps it: the push damps it once past its share, and
        // the thrust lifts it back over, a tick's thrust at most.
        let along = facing(45.0) * afterburner_top(session.top_speed());
        let thrust = facing(45.0).x * session.flying_handling().accel;
        for _ in 0..4 {
            let step = moved(&mut session, held(true, true));
            assert_eq!(step.x, -step.y, "{step:?}");
            assert!(
                step.x >= along.x * 0.99 && step.x <= along.x + thrust + 1e-4,
                "{step:?}"
            );
        }
    }

    #[test]
    fn the_afterburner_alone_pushes_by_accel_x_2_75_past_top_speed() {
        let mut session = flying(&burner(), 90.0);
        let push = session.flying_handling().accel * 2.75;
        session.tick(held(false, true));
        assert!(
            near(session.player.velocity.x, push),
            "{:?}",
            session.player
        );
        session.tick(held(false, true));
        assert!(near(session.player.velocity.x, 2.0 * push));
        run(&mut session, held(false, true), 100);
        let burnt = afterburner_top(session.top_speed());
        // It meets the box, is damped, and climbs back to it: a tick at the
        // box, then one at 0.99 of it.
        let steps = [0; 4].map(|_| moved(&mut session, held(false, true)).x);
        assert!(
            steps
                .iter()
                .all(|&x| x <= burnt + 1e-4 && x >= burnt * 0.99 - 1e-4),
            "{steps:?}"
        );
        assert!(steps.iter().any(|&x| near(x, burnt)), "{steps:?}");
    }

    #[test]
    fn released_the_box_bleeds_at_accel_x_0_4_a_tick_holding_the_velocity_to_it() {
        let mut session = flying(&burner(), 90.0);
        run(&mut session, held(true, true), 100);
        let top = session.top_speed();
        let bleed = session.flying_handling().accel * 0.4;
        let mut box_ = session.player_max_vel.get();
        assert!(near(box_, afterburner_top(top)));
        let mut ticks = 0;
        let mut held_to_it = 0;
        while session.player_max_vel.get() > top {
            let was = session.player.velocity.x;
            let step = moved(&mut session, held(true, false));
            ticks += 1;
            assert!(
                near(step.x, was.min(box_)),
                "held to the last box: {step:?}"
            );
            held_to_it += usize::from(was > box_);
            let now = session.player_max_vel.get();
            assert!(now == top || near(box_ - now, bleed), "{box_} to {now}");
            box_ = now;
            assert!(ticks < 200);
        }
        assert!(held_to_it > 50, "{held_to_it} of {ticks}");
        assert!(
            near(moved(&mut session, held(true, false)).x, top),
            "back at top speed"
        );
    }

    #[test]
    fn the_box_bleeds_with_no_afterburner_carried_and_in_the_pre_jump() {
        let mut session = flying(&catalog(), 90.0);
        let top = session.top_speed();
        session.player_max_vel = crate::afterburner::MaxVel::START;
        session.player_max_vel.burn(top, false);
        session.tick(Controls::default());
        assert!(near(
            session.player_max_vel.get(),
            afterburner_top(top) - session.flying_handling().accel * 0.4
        ));
        let mut jumping = super::super::tests::bound_for_131(
            &burner(),
            super::super::tests::OUT,
            Vec2::ZERO,
            0.0,
        );
        jumping.player_max_vel.burn(top, false);
        assert_eq!(jumping.begin_jump(), Ok(SystemId(131)));
        jumping.tick(Controls::default());
        assert!(jumping.player_max_vel.get() < afterburner_top(top));
    }

    /// Widens `session`'s box to 1.8 times its top speed, as an afterburn
    /// leaves it: the box's width.
    fn widened(session: &mut Session) -> f32 {
        session.player_max_vel.burn(session.top_speed(), false);
        let box_ = session.player_max_vel.get();
        assert!(near(box_, afterburner_top(session.top_speed())));
        box_
    }

    #[test]
    fn in_hyperspace_the_box_bleeds_by_accel_x_0_4_a_tick() {
        let mut session = flying(&burner(), 90.0);
        let box_ = widened(&mut session);
        session.jump = Some(super::super::Jump::Hyperspace {
            to: SystemId(131),
            warp: 1,
        });
        session.tick(Controls::default());
        assert!(
            matches!(session.jump, Some(super::super::Jump::Hyperspace { .. })),
            "still in the warp's run"
        );
        let bleed = session.flying_handling().accel * 0.4;
        assert!(
            near(session.player_max_vel.get(), box_ - bleed),
            "{box_} to {}",
            session.player_max_vel.get()
        );
    }

    #[test]
    fn a_destroyed_players_tick_leaves_the_box_alone() {
        let mut session = flying(&burner(), 90.0);
        let box_ = widened(&mut session);
        session.condition = Condition::Destroyed;
        session.tick(Controls::default());
        assert_eq!(session.player_max_vel.get(), box_);
    }

    #[test]
    fn the_escape_pods_tick_leaves_the_box_alone() {
        let mut session = flying(&burner(), 90.0);
        let box_ = widened(&mut session);
        session.pod = Some(100);
        session.tick(Controls::default());
        assert_eq!(session.pod, Some(99), "the pod flew");
        assert_eq!(session.player_max_vel.get(), box_);
    }

    #[test]
    fn the_first_pre_jump_tick_after_an_afterburn_is_held_to_the_box_not_top_speed() {
        let mut session = super::super::tests::bound_for_131(
            &burner(),
            super::super::tests::OUT,
            Vec2::ZERO,
            90.0,
        );
        run(&mut session, held(true, true), 100);
        let top = session.top_speed();
        let box_ = session.player_max_vel.get();
        assert!(near(box_, afterburner_top(top)), "{box_}");
        assert_eq!(session.begin_jump(), Ok(SystemId(131)));
        session.tick(Controls::default());
        assert!(session.preparing_jump().is_some(), "in the pre-jump");
        let speed = session.player.velocity.length();
        assert!(speed > top + 1.0, "held to the box: {speed}");
        assert!(speed <= box_ + 1e-4, "{speed}");
    }

    #[test]
    fn a_disabled_inertialess_player_keeps_a_speed_past_top_speed_in_a_widened_box() {
        let mut session = flying(&inertialess(), 90.0);
        let box_ = widened(&mut session);
        let top = session.top_speed();
        let speed = box_ - 0.5;
        assert!(speed > top + 1.0);
        session.player_speed = speed;
        session.player.velocity = Vec2::new(speed, 0.0);
        session.condition = Condition::Disabled;
        session.tick(Controls::default());
        assert!(
            near(
                session.player_speed(),
                speed * crate::inertialess::PLAYER_DISABLED_DECAY
            ),
            "{}",
            session.player_speed()
        );
    }

    #[test]
    fn a_new_sessions_box_of_none_holds_the_first_tick_at_rest() {
        let mut session = Session::start(&burner()).expect("starts");
        session.player.velocity = Vec2::new(3.0, 0.0);
        let at = session.player.position;
        session.tick(held(true, false));
        assert_eq!(session.player.velocity, Vec2::ZERO);
        assert_eq!(session.player.position, at);
        assert_eq!(session.player_max_vel.get(), session.top_speed());
        session.tick(held(true, false));
        assert!(session.player.velocity != Vec2::ZERO, "moves on the second");
    }

    #[test]
    fn double_times_second_pass_burns_again() {
        let mut session = flying(&burner(), 90.0);
        let full = fuel(&session);
        session.tick(held(false, true));
        session.tick_again(
            held(false, true),
            &crate::ai::NovaAi::default(),
            crate::combat::Rules::default(),
            &mut crate::chance::NeverFires,
        );
        assert!(
            near(fuel(&session), full - 2.0 * usage()),
            "{}",
            fuel(&session)
        );
    }

    #[test]
    fn under_the_circle_the_afterburning_players_speed_is_held_to_the_boxs_circle() {
        let rules = Rulebook::default().with_override(RuleKey::SpeedCap, RuleSource::Bible);
        let mut session = flying(&burner(), 45.0).with_rules(rules);
        run(&mut session, held(true, true), 300);
        let burnt = afterburner_top(session.top_speed());
        for _ in 0..4 {
            let speed = moved(&mut session, held(true, true)).length();
            assert!(speed > session.top_speed(), "{speed}");
            assert!(speed <= burnt + 1e-3, "{speed}");
        }
    }

    // Knockback.

    #[test]
    fn a_hit_on_the_afterburning_player_keeps_it_past_its_top_speed() {
        use super::super::tests::{armed, fight_until, knocker, knocking};
        let catalog = armed();
        let hit = |burning: bool| {
            let mut session = knocking(&catalog, (None, 100.0), (Some(knocker()), 100.0));
            session.player.velocity = Vec2::new(20.0, 0.0);
            session.afterburner_on = burning;
            fight_until(&mut session, |session| {
                session.reserves().shield.now < 1000.0
            });
            (session.top_speed(), session.player.velocity)
        };
        let (top, plain) = hit(false);
        assert!(top < 20.0 && afterburner_top(top) > 20.0, "{top}");
        assert!(
            near(plain.x, top) && near(plain.y, 1.0),
            "clamped: {plain:?}"
        );
        let (_, burning) = hit(true);
        assert!(
            near(burning.x, 20.0) && near(burning.y, 1.0),
            "kept: {burning:?}"
        );
    }

    #[test]
    fn a_shockwave_on_the_afterburning_player_keeps_it_past_its_top_speed() {
        use super::super::tests::{beside_a_heavy_npc, on_its_last_tick, shockwave_catalog};
        let catalog = shockwave_catalog();
        let shoved = |burning: bool| {
            let mut session = beside_a_heavy_npc(&catalog);
            session.pilot.reserves.shield = crate::reserves::Gauge::full(100.0);
            session.pilot.reserves.armor = crate::reserves::Gauge::full(300.0);
            let top = session.top_speed();
            session.player.velocity = Vec2::new(afterburner_top(top) - 0.5, 0.0);
            session.afterburner_on = burning;
            let npc = &mut session.traffic.npcs_mut()[0];
            on_its_last_tick(&mut npc.reserves, &mut npc.condition);
            session.tick_combat(
                crate::combat::Rules::default(),
                &mut crate::chance::NeverFires,
            );
            (top, session.player.velocity)
        };
        let (top, plain) = shoved(false);
        assert!(near(plain.x, top), "clamped: {plain:?}");
        let (_, burning) = shoved(true);
        assert!(
            near(burning.x, afterburner_top(top) - 0.5),
            "kept: {burning:?}"
        );
        assert!(burning.y > 0.0, "pushed: {burning:?}");
    }

    // Inertialess.

    #[test]
    fn an_inertialess_afterburner_pushes_the_speed_to_1_8_times_top_speed_coasting() {
        let mut session = flying(&inertialess(), 90.0);
        let push = session.flying_handling().accel * 2.75;
        session.tick(held(false, true));
        assert!(
            near(session.player_speed(), push),
            "{}",
            session.player_speed()
        );
        run(&mut session, held(false, true), 200);
        let burnt = afterburner_top(session.top_speed());
        assert!(
            near(session.player_speed(), burnt),
            "{}",
            session.player_speed()
        );
        assert!(
            near(moved(&mut session, held(false, true)).x, burnt),
            "{:?}",
            session.player
        );
    }

    #[test]
    fn with_thrust_too_an_inertialess_speed_sits_at_top_speed_plus_the_push() {
        let mut session = flying(&inertialess(), 90.0);
        run(&mut session, held(true, true), 200);
        let top = session.top_speed();
        let push = session.flying_handling().accel * 2.75;
        assert!(
            near(session.player_speed(), top + push),
            "{}",
            session.player_speed()
        );
    }

    // The glow.

    #[test]
    fn coasting_on_the_afterburner_the_glow_climbs_1_a_tick_to_32() {
        let mut session = flying(&burner(), 90.0);
        session.engine_glow = i16::from(GLOW_CRUISE);
        session.tick(held(false, true));
        assert_eq!(session.engine_glow, 25);
        for _ in 0..20 {
            session.tick(held(false, true));
        }
        assert_eq!(session.engine_glow, GLOW_AFTERBURNER);
    }

    #[test]
    fn thrusting_on_the_afterburner_the_glow_climbs_2_a_tick_to_cruise_then_1_to_32() {
        let mut session = flying(&burner(), 90.0);
        for k in 1..=12 {
            session.tick(held(true, true));
            assert_eq!(session.engine_glow, 2 * k, "{k}");
        }
        session.tick(held(true, true));
        assert_eq!(session.engine_glow, 25);
        for _ in 0..7 {
            session.tick(held(true, true));
        }
        assert_eq!(session.engine_glow, GLOW_AFTERBURNER);
        session.tick(held(true, true));
        assert_eq!(session.engine_glow, GLOW_AFTERBURNER);
    }

    /// The inertialess player's glow base after one tick under
    /// `controls` from `base`, flown first to past its top speed under
    /// the same keys, so its glow follows a speed target of cruise.
    fn inertialess_glow_from(base: i16, controls: Controls) -> i16 {
        let mut session = flying(&inertialess(), 90.0);
        run(&mut session, controls, 200);
        assert!(session.player_speed() > session.top_speed());
        session.engine_glow = base;
        session.tick(controls);
        session.engine_glow
    }

    #[test]
    fn past_top_speed_the_afterburner_holds_an_inertialess_glow_at_30_and_lifts_one_below() {
        // Coasting: down 1, toward cruise 1, then the afterburner's 2 up.
        assert_eq!(inertialess_glow_from(30, held(false, true)), 30);
        assert_eq!(inertialess_glow_from(10, held(false, true)), 12);
        // Thrusting: up 1 below cruise, toward cruise 1, then 1 up.
        assert_eq!(inertialess_glow_from(30, held(true, true)), 30);
        assert_eq!(inertialess_glow_from(10, held(true, true)), 13);
    }

    #[test]
    fn released_at_32_the_glow_falls_1_a_tick_to_cruise_under_thrust() {
        let mut session = flying(&burner(), 90.0);
        session.engine_glow = GLOW_AFTERBURNER;
        for k in 1..=8 {
            session.tick(held(true, false));
            assert_eq!(session.engine_glow, GLOW_AFTERBURNER - k);
        }
        session.tick(held(true, false));
        assert_eq!(session.engine_glow, i16::from(GLOW_CRUISE));
    }
}
