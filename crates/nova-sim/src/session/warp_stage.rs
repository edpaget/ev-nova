//! The player's warp stage: from the end of the brake until the jump
//! ends, `_HandlePlayer`'s run of the original's +0x50 above 1.0.
//!
//! **The stage** (+0x50). The brake holds it at 1.0 (@0x6b6aa); the first
//! frame the ship is slow enough, or at once when it can expedite its
//! jumps, it is set to 2.0 and the clock stamped (@0x6b5e9-0x6b5f5;
//! [`Session::start_warp`]). From then on, each pass of a player not
//! disabled (-0x2cb, @0x6baf7):
//!
//! 1. the ships it leads take its jump ([`Session::make_escorts_hyper`],
//!    `_MakeEscortsHyperWithParent` @0x6bb0a);
//! 2. +0x50 grows by `_gSpeedMult`, 1.0 on either of double time's passes
//!    (@0x6bb0f-0x6bb21);
//! 3. unless it can expedite its jumps (`_ShipCanExpiditeJumps`), its
//!    velocity is multiplied by [`WARP_DRAG`], 0.98 (@0x6bb32-0x6bb59;
//!    [`drag_by`]);
//! 4. the warp end ([`warp::warp_end`], @0x6bb5e-0x6c70b): its natural
//!    end marks the jump's end due ([`Session::warp_end_due`]); its
//!    fallback end halts the warp-ups and plays the warp-out, which
//!    [`Session::arrive`] does, and arms the natural end of the next pass;
//! 5. the turn to the bearing, the player's turn block (@0x6c899), as the
//!    stage sets the auto-turn (@0x6b0d3) with the bearing as +0x68;
//! 6. **the move-off** (@0x6d097-0x6d343): once it faces within
//!    max(`_ShipTurnRate`, [`MOVE_OFF_ARC`]) whole degrees of the bearing
//!    (@0x6d12f-0x6d1bb), while the move-off speed v
//!    ([`warp::move_off_speed`]) is above 0: `_hyperGamma` is set to
//!    (v − 55) × 5 ([`warp::hyper_gamma`]) unless the Hyperspace Effects
//!    preference is off (`_settings`+0x14, @0x6d29c-0x6d2bf); its
//!    position, not its velocity, moves min(v, 50) × `_gSpeedMult` along
//!    its whole-degree heading (`_Accel` on +0x18, @0x6d2c3-0x6d318), so
//!    no speed cap holds it; and its glow rises by 4 to at most 32
//!    (@0x6d320-0x6d33c);
//! 7. an inertialess ship's glow follows its speed and its velocity its
//!    heading (@0x6d343-0x6d43a), then the velocity is capped to the box
//!    and the ship moves on at it (@0x6d43f-0x6d4eb), as every frame.
//!
//! The warp clock is the jump's (see the session's `Jump`), moved on only
//! on the advance pass, as `TickCount` is real time.
//!
//! **The screen's white** (`_hyperGamma`, *0x3b55b0). At the top of each
//! pass of `_HandlePlayer` (@0x683a4-0x68424), unless the Hyperspace
//! Effects preference is off: while +0x50 is above 0 a negative value is
//! raised to 0 and one above 100.0 lowered to it, and once +0x50 is no
//! longer above 0 a value of 0 or more goes back to −1.0
//! ([`Session::hyper_gamma`]). While it is above 0 and no fade is up, the
//! original fades the screen to white (`_FadeWhiteIn` @0x546f:
//! `CGDisplayFade` to white over 1.5 s, not synchronous, so the game runs
//! on under it); the view does it. `_PlayGame` sets it to −1.0 after the
//! priming pass of a game's start (@0x45811). The gate's other two bytes
//! are `_settings`+0xc, Run In A Window (`_LoadPrefs` @0xcee54, key
//! `RunInAWindow`), under which the original fades no display, and the
//! priming flag *0x3b5178: the engine fades its own frame, in a window
//! too, and runs no priming pass.
//!
//! **Ordering against the original.** The warp end is decided in the
//! player's pass, as the original's is, and the view arrives
//! ([`Session::arrive`]) in the same step, before the frame is drawn: no
//! boundary falls between them.
//!
//! Left out: the disabled player's warp-stage branch (@0x6c70d-0x6c723,
//! +0x50 of −999.0 to 0), which no engine state reaches: a disabled
//! player's jump collapses first (see the `jump_collapse` module).

use super::{Jump, Session};
use crate::catalog::SystemId;
use crate::flight::{Controls, Flyer, Turn, advance, drag_by, inertialess_advance};
use crate::glow::{Follow, JumpRaises, ramp_jump_glow};
use crate::player_turn::brake_aim;
use crate::player_turn::turn;
use crate::pre_jump::{self, PreJump};
use crate::traffic::npc::NpcId;
use crate::traffic::warp_out::{LeaderWarp, WarpWorld};
use crate::trig::{self, angular_difference, whole};
use crate::warp::{self, WarpEnd, mac_ticks};

/// What the warp multiplies the velocity by each pass, unless the ship
/// can expedite its jumps (0.98, f64 @0xdd988).
pub const WARP_DRAG: f64 = 0.98;

/// How far off the bearing, in whole degrees, the ship may face and move
/// off, unless its turn rate is wider (30.0, f32 @0xdd084).
pub const MOVE_OFF_ARC: f32 = 30.0;

/// The fastest the move-off moves the ship a pass (50.0, f32 @0xdd064).
const MOVE_OFF_CAP: f32 = 50.0;

/// The player's warp: the original's +0x50 and the globals its warp end
/// and the screen's white read and write (see the module docs). Not
/// saved, as no warp survives a save.
// Each flag is its own, read on its own.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct PlayerWarp {
    /// +0x50 once the warp stage has started: 2.0 as it starts, then 1.0
    /// more each pass ([`Session::warp_stage`] gives 1.0 while it
    /// brakes).
    stage: f32,
    /// `_warpSoundDone` (*0xdee3c).
    sound_done: bool,
    /// Whether the warp end's fallback has halted the warp-ups, for the
    /// natural end's reads; [`Session::arrive`] raises the halt.
    halted: bool,
    /// Whether the jump's end is due.
    end_due: bool,
    /// `_hyperGamma`.
    gamma: f32,
    /// Whether the Hyperspace Effects preference is on, `_settings`+0x14
    /// clear.
    effects: bool,
}

impl Default for PlayerWarp {
    fn default() -> Self {
        Self {
            stage: 0.0,
            sound_done: false,
            halted: false,
            end_due: false,
            gamma: -1.0,
            effects: true,
        }
    }
}

impl Session {
    /// Sets the Hyperspace Effects preference: off, the move-off lights
    /// no white (`_settings`+0x14, see the module docs). On as a session
    /// starts.
    pub fn set_hyperspace_effects(&mut self, on: bool) {
        self.warp.effects = on;
    }

    /// `_hyperGamma`: above 0 while the warp's move-off lights the screen
    /// white, which the view fades to; −1.0 out of a jump (see the module
    /// docs).
    #[must_use]
    pub fn hyper_gamma(&self) -> f32 {
        self.warp.gamma
    }

    /// Whether the warp has ended, so the jump arrives
    /// ([`Session::arrive`]): its natural end, or a pass after its
    /// fallback (see the module docs).
    #[must_use]
    pub fn warp_end_due(&self) -> bool {
        self.warp.end_due && self.jumping().is_some()
    }

    /// The player's +0x50 now: 0 out of a jump, 1.0 while it brakes, the
    /// stage once the warp stage has started.
    pub(super) fn warp_stage(&self) -> f32 {
        match self.jump {
            None => 0.0,
            Some(Jump::PreJump { warp: None, .. }) => 1.0,
            Some(_) => self.warp.stage,
        }
    }

    /// What the traffic's warp-outs read of the player's warp, the warp
    /// sounds and double time ([`WarpWorld`]).
    pub(super) fn warp_world(&self) -> WarpWorld {
        WarpWorld {
            player: LeaderWarp {
                stage: self.warp_stage(),
                since: self.warp_clock().unwrap_or(0),
                factor: self.player_warp_factor(),
                heading: self.player.heading,
                aim: self.aim,
                velocity: self.player.velocity,
                leaving: false,
            },
            durations: self.warp_sounds,
            double_time: self.double_time,
        }
    }

    /// The warp factor of the player's ship class (class +0x44,
    /// [`warp::warp_factor`]).
    pub(super) fn player_warp_factor(&self) -> f32 {
        let flags = self
            .ship_record(self.pilot.ship)
            .map_or(0, |record| record.flags);
        warp::warp_factor(flags)
    }

    /// The top of a pass of `_HandlePlayer` for the screen's white (see
    /// the module docs).
    pub(super) fn gamma_tick(&mut self) {
        if !self.warp.effects {
            return;
        }
        if self.warp_stage() > 0.0 {
            self.warp.gamma = self.warp.gamma.clamp(0.0, 100.0);
        } else if self.warp.gamma >= 0.0 {
            self.warp.gamma = -1.0;
        }
    }

    /// The warp stage's start (@0x6b5e9-0x6b603): +0x50 = 2.0, and the
    /// pilot is past the tutorial; the warp-up sounds unless it still
    /// plays ([`Session::sound_warp_up`]). The caller stamps the clock.
    pub(super) fn start_warp(&mut self) {
        self.warp.stage = 2.0;
        self.warp.halted = false;
        self.warp.end_due = false;
        // The pilot is past the tutorial (`_HandlePlayer` @0x6b603).
        self.tutorial.jumped();
        self.sound_warp_up();
    }

    /// The warp-up's start clears `_warpSoundDone` (@0x6b695), which a
    /// warp-up still playing skips.
    pub(super) fn clear_warp_sound_done(&mut self) {
        self.warp.sound_done = false;
    }

    /// One pass of the warp stage toward `bearing` under `controls` (see
    /// the module docs, steps 1-7), the aim already set.
    pub(super) fn fly_warp_stage(&mut self, bearing: Option<i16>, controls: Controls) {
        let staged = self.warp.stage > 1.0;
        if staged {
            self.make_escorts_hyper();
            self.warp.stage = grown(self.warp.stage);
            if !self.stats.fast_jump {
                self.player.velocity = drag_by(self.player.velocity, WARP_DRAG);
            }
            self.warp_end_tick();
        }
        let handling = self.flying_handling();
        let flight = self.flight_rules();
        (self.player.heading, _) = turn(
            self.player.heading,
            handling.turn_rate,
            bearing,
            Turn::None,
            flight.player_turn,
        );
        let moved = staged && self.move_off(bearing, handling.turn_rate);
        let caps = self.player_caps();
        if self.player_inertialess() {
            inertialess_advance(
                &mut self.player,
                &mut self.player_speed,
                &handling,
                0.0,
                caps,
                flight,
                Flyer::Player,
            );
        } else {
            advance(
                &mut self.player,
                &handling,
                false,
                caps,
                flight,
                Flyer::Player,
            );
        }
        self.sound_thrust(false);
        self.ramp_jump(
            JumpRaises {
                brake: false,
                move_off: moved,
            },
            controls.thrust,
        );
    }

    /// One pass of a jump's pre-jump stage, bound for `to` on `bearing`,
    /// the warp's clock `warp` (none while braking), under `controls`, on
    /// double time's second pass when `again`: the brake (see
    /// [`pre_jump`]), or the warp stage, started on its first pass.
    pub(super) fn fly_pre_jump(
        &mut self,
        to: SystemId,
        bearing: Option<i16>,
        warp: Option<u32>,
        controls: Controls,
        again: bool,
    ) {
        let handling = self.flying_handling();
        let slow_down = !self.stats.fast_jump;
        let flight = self.flight_rules();
        let inertialess = self.player_inertialess();
        let stage = pre_jump::stage(
            &self.player,
            &handling,
            bearing,
            slow_down,
            flight.player_turn,
            inertialess,
        );
        if stage == PreJump::Brake {
            // An inertialess ship's brake writes no aim (@0x6b6c3).
            if !inertialess {
                self.aim = brake_aim(self.player.velocity);
            }
            let caps = self.player_caps();
            let speed = inertialess.then_some(&mut self.player_speed);
            let thrust = pre_jump::fly(
                &mut self.player,
                &handling,
                bearing,
                slow_down,
                caps,
                flight,
                speed,
            );
            self.sound_thrust(thrust);
            self.ramp_jump(
                JumpRaises {
                    brake: thrust,
                    move_off: false,
                },
                controls.thrust,
            );
            self.burn_afterburner();
            self.stage_jump(to, bearing, None);
        } else {
            self.aim_at_jump(to);
            let warp = match warp {
                None => {
                    // The warp stage starts: the stamp (@0x6b5f5).
                    self.start_warp();
                    0
                }
                Some(ticks) if again => ticks,
                Some(ticks) => ticks.saturating_add(1),
            };
            self.jump = Some(Jump::PreJump {
                to,
                bearing,
                warp: Some(warp),
            });
            self.fly_warp_stage(bearing, controls);
            self.burn_afterburner();
            // Unless its end has already begun the jump.
            if self.preparing_jump().is_some() {
                self.stage_jump(to, bearing, Some(warp));
            }
        }
    }

    /// The glow's step through a jump, with `raises` and the thrust `key`
    /// held or not ([`ramp_jump_glow`]); an inertialess ship's follows its
    /// speed.
    pub(super) fn ramp_jump(&mut self, raises: JumpRaises, key: bool) {
        let follow = self.player_inertialess().then(|| Follow::Speed {
            speed: self.player_speed,
            max_speed: self.flying_handling().max_speed,
        });
        self.engine_glow =
            ramp_jump_glow(self.engine_glow, raises, follow, self.afterburner_on, key);
    }

    /// `_MakeEscortsHyperWithParent` (@0x3fd54-0x3fdea): each ship the
    /// player leads takes its +0x50 and clock, lets go of its target and
    /// follows its jump ([`Npc::follow_player_jump`]). Its test of +0x8c,
    /// the stellar a ship defends, always holds: the engine has no
    /// defence ships.
    ///
    /// [`Npc::follow_player_jump`]: crate::traffic::npc::Npc::follow_player_jump
    fn make_escorts_hyper(&mut self) {
        let stage = self.warp.stage;
        let since = self.warp_clock().unwrap_or(0);
        let led: Vec<NpcId> = self.player_led().map(|npc| npc.id).collect();
        for id in led {
            if let Some(npc) = self.npc_mut(id) {
                npc.follow_player_jump(stage, since);
            }
        }
    }

    /// One pass of the warp end ([`warp::warp_end`]).
    fn warp_end_tick(&mut self) {
        let since = mac_ticks(self.warp_clock().unwrap_or(0));
        let playing =
            !self.warp.halted && (self.warp_ups.playing(false) || self.warp_ups.playing(true));
        match warp::warp_end(
            self.warp.stage,
            since,
            self.player_warp_factor(),
            self.warp_sound(),
            playing,
            &mut self.warp.sound_done,
        ) {
            WarpEnd::Running => {}
            WarpEnd::Natural => {
                self.warp.end_due = true;
                // The warp ends whichever way the ship faces: one still
                // turning has begun its jump all the same.
                if let Some(Jump::PreJump {
                    to,
                    warp: Some(warp),
                    ..
                }) = self.jump
                {
                    self.jump = Some(Jump::Hyperspace { to, warp });
                }
            }
            WarpEnd::Fallback => self.warp.halted = true,
        }
    }

    /// The move-off toward `bearing`, at a turn rate of `turn_rate` (see
    /// the module docs, step 6): whether the ship moved off.
    fn move_off(&mut self, bearing: Option<i16>, turn_rate: f32) -> bool {
        let Some(bearing) = bearing else {
            return false;
        };
        let off = f32::from(angular_difference(whole(self.player.heading), bearing).abs());
        if turn_rate.max(MOVE_OFF_ARC) < off {
            return false;
        }
        let v = warp::move_off_speed(
            mac_ticks(self.warp_clock().unwrap_or(0)),
            self.player_warp_factor(),
            self.warp_sound(),
            self.double_time,
        );
        if v.is_nan() || v <= 0.0 {
            return false;
        }
        if self.warp.effects {
            self.warp.gamma = warp::hyper_gamma(v);
        }
        let push = trig::accel(whole(self.player.heading), v.min(MOVE_OFF_CAP));
        self.player.position = self.player.position + push;
        true
    }
}

/// `stage` grown by `_gSpeedMult`, 1.0, in the original's widths: the f64
/// sum stored as an f32 (@0x6bb0f-0x6bb21).
#[allow(clippy::cast_possible_truncation)]
fn grown(stage: f32) -> f32 {
    (f64::from(stage) + 1.0) as f32
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::{ShipId, SystemId};
    use crate::chance::NeverFires;
    use crate::geometry::Vec2;
    use crate::session::tests::{OUT, bound_for_131};
    use crate::sound::{SimSound, WARP_UP_HALTS};
    use crate::testkit::{FAST, FakePilotCatalog, catalog};

    /// `catalog`'s ship at rest, facing east, the bearing to 131, out far
    /// enough, its warp sounds 364 and 252 Mac ticks long: J begins the
    /// jump at once.
    fn jumping(catalog: &FakePilotCatalog) -> Session {
        let mut session = bound_for_131(catalog, OUT, Vec2::ZERO, 90.0);
        session.warp_sounds = [364, 252];
        session.begin_jump().expect("jumps");
        assert_eq!(session.jumping(), Some(SystemId(131)));
        session
    }

    /// A catalog whose ship's class, 128, has `Flags` `flags` in its
    /// `shïp` record.
    fn flagged(flags: u16) -> FakePilotCatalog {
        let mut catalog = catalog();
        catalog.ship_records = vec![crate::catalog::ShipRecord {
            flags,
            ..crate::testkit::ship(128, FAST)
        }];
        catalog
    }

    fn tick(session: &mut Session) {
        session.tick(Controls::default());
    }

    // The stage.

    #[test]
    fn the_stage_starts_at_2_and_grows_by_1_a_pass_on_either_pass() {
        let mut session = jumping(&catalog());
        assert_eq!(session.warp_stage(), 2.0, "@0x6b5e9");
        tick(&mut session);
        assert_eq!(session.warp_stage(), 3.0);
        assert_eq!(session.warp_clock(), Some(1));
        session.tick_again(
            Controls::default(),
            crate::Rules::default(),
            &mut NeverFires,
        );
        assert_eq!(session.warp_stage(), 4.0, "double time's second pass too");
        assert_eq!(session.warp_clock(), Some(1), "the clock is real time");
    }

    #[test]
    fn a_stage_of_1_runs_no_warp_stage_block() {
        // +0x50 > 1.0 (@0x6bae2-0x6baf1): the brake's 1.0 is not in it.
        let mut session = jumping(&catalog());
        session.warp.stage = 1.0;
        session.player.velocity = Vec2::new(0.5, 0.0);
        tick(&mut session);
        assert_eq!(session.warp_stage(), 1.0, "no growth");
        assert_eq!(session.player.velocity, Vec2::new(0.5, 0.0), "no drag");
    }

    #[test]
    fn the_brake_holds_the_stage_at_1() {
        let mut session = bound_for_131(&catalog(), OUT, Vec2::new(0.0, -5.0), 0.0);
        session.begin_jump().expect("jumps");
        tick(&mut session);
        assert_eq!(session.warp_clock(), None, "braking");
        assert_eq!(session.warp_stage(), 1.0);
    }

    // The drag.

    #[test]
    fn the_warp_drags_the_velocity_by_0_98_a_pass_unless_the_ship_expedites_its_jumps() {
        let mut session = jumping(&catalog());
        let velocity = Vec2::new(0.5, -0.25);
        session.player.velocity = velocity;
        tick(&mut session);
        assert_eq!(session.player.velocity, drag_by(velocity, 0.98));
        assert_eq!(session.player.velocity.x.to_bits(), 0.49_f32.to_bits());
        assert_eq!(session.player.position, OUT + drag_by(velocity, 0.98));
        let mut fast = jumping(&catalog());
        fast.stats.fast_jump = true;
        fast.player.velocity = velocity;
        tick(&mut fast);
        assert_eq!(fast.player.velocity, velocity, "`_ShipCanExpiditeJumps`");
    }

    // An inertialess ship.

    #[test]
    fn an_inertialess_ships_velocity_follows_its_heading_and_its_glow_its_speed_past_the_move_off()
    {
        // Step 7 (@0x6d343-0x6d43a): after the move-off, the velocity is
        // pulled toward heading × speed and the glow follows the speed.
        let catalog = FakePilotCatalog {
            ships: vec![(
                ShipId(128),
                Ok(crate::handling::ShipFields {
                    flags2: crate::stats::INERTIALESS_HULL,
                    ..FAST
                }),
            )],
            ..catalog()
        };
        let mut session = jumping(&catalog);
        assert!(session.player_inertialess());
        tick_to(&mut session, 100);
        assert_eq!(session.player.heading, 90.0, "facing the bearing");
        let speed = session.top_speed() / 2.0;
        session.player_speed = speed;
        session.player.velocity = Vec2::new(0.0, -2.0);
        session.engine_glow = 10;
        let handling = session.flying_handling();
        let caps = session.player_caps();
        let flight = session.flight_rules();
        let mut dragged = session.player;
        dragged.velocity = drag_by(dragged.velocity, WARP_DRAG);
        let mut inertialess = dragged;
        let mut followed = speed;
        inertialess_advance(
            &mut inertialess,
            &mut followed,
            &handling,
            0.0,
            caps,
            flight,
            Flyer::Player,
        );
        let mut plain = dragged;
        advance(&mut plain, &handling, false, caps, flight, Flyer::Player);
        assert_ne!(inertialess.velocity, plain.velocity);
        let raises = JumpRaises {
            brake: false,
            move_off: true,
        };
        let follow = Follow::Speed {
            speed: followed,
            max_speed: handling.max_speed,
        };
        let glow = ramp_jump_glow(10, raises, Some(follow), false, false);
        assert_ne!(glow, ramp_jump_glow(10, raises, None, false, false));
        let at = session.player.position;
        tick(&mut session);
        let v = warp::move_off_speed(mac_ticks(101), warp::warp_factor(0), 364, false);
        assert_eq!(session.player.velocity, inertialess.velocity);
        assert_eq!(session.player_speed(), followed);
        assert_eq!(
            session.player.position,
            at + trig::accel(90, v) + inertialess.velocity,
            "the move-off, then the move at the velocity"
        );
        assert_eq!(session.engine_glow, glow);
    }

    // The move-off.

    /// Ticks `session` until its warp clock reads `clock`.
    fn tick_to(session: &mut Session, clock: u32) {
        while session.warp_clock().is_some_and(|now| now < clock) {
            tick(session);
        }
        assert_eq!(session.warp_clock(), Some(clock));
    }

    #[test]
    fn the_move_off_moves_the_position_by_v_along_the_whole_degree_heading() {
        // 0.35 × 364 / 1.69 = 75.4 Mac ticks: v first above 0 at clock 38.
        let w = warp::warp_factor(0);
        assert!(warp::move_off_speed(74, w, 364, false) <= 0.0);
        let v = warp::move_off_speed(76, w, 364, false);
        assert!(v > 0.0);
        let mut session = jumping(&catalog());
        tick_to(&mut session, 37);
        assert_eq!(session.player.position, OUT, "still");
        session.player.heading = 90.9;
        tick(&mut session);
        assert_eq!(
            session.player.position,
            OUT + trig::accel(90, v),
            "along 90, its whole degree, by v; the velocity untouched"
        );
        assert_eq!(session.player.velocity, Vec2::ZERO);
        assert_eq!(session.hyper_gamma(), warp::hyper_gamma(v));
    }

    #[test]
    fn the_move_off_is_capped_at_50_a_pass_and_held_by_no_speed_cap() {
        let mut session = jumping(&catalog());
        tick_to(&mut session, 300);
        let at = session.player.position;
        tick(&mut session);
        assert_eq!(session.player.position, at + trig::accel(90, 50.0));
        assert!(50.0 > session.flying_handling().max_speed);
    }

    #[test]
    fn the_ship_moves_off_only_facing_within_30_whole_degrees_of_the_bearing() {
        let mut session = jumping(&catalog());
        tick_to(&mut session, 100);
        // 124 turns 3 to 121, 31 off: no move.
        session.player.heading = 124.0;
        let at = session.player.position;
        tick(&mut session);
        assert_eq!(session.player.heading, 121.0);
        assert_eq!(session.player.position, at);
        // 118, 28 off: it moves.
        tick(&mut session);
        assert_eq!(session.player.heading, 118.0);
        assert_ne!(session.player.position, at);
    }

    #[test]
    fn the_ship_moves_off_at_30_whole_degrees_off_the_bearing() {
        let mut session = jumping(&catalog());
        tick_to(&mut session, 100);
        // 123 turns 3 to 120, 30 off: within.
        session.player.heading = 123.0;
        let at = session.player.position;
        tick(&mut session);
        assert_eq!(session.player.heading, 120.0);
        assert_ne!(session.player.position, at);
    }

    #[test]
    fn a_ship_turning_wider_than_30_moves_off_within_its_turn() {
        let mut catalog = catalog();
        catalog.ships = vec![(
            ShipId(128),
            Ok(crate::handling::ShipFields {
                maneuver: 400,
                ..FAST
            }),
        )];
        let mut session = jumping(&catalog);
        let rate = session.flying_handling().turn_rate;
        assert!(rate > 30.0, "{rate}");
        assert_eq!(MOVE_OFF_ARC, 30.0);
        tick_to(&mut session, 100);
        let at = session.player.position;
        session.player.heading = 90.0;
        tick(&mut session);
        assert_ne!(session.player.position, at);
    }

    // The screen's white.

    #[test]
    fn the_hyper_gamma_lights_through_the_move_off_and_goes_out_on_arrival() {
        let catalog = catalog();
        let mut session = jumping(&catalog);
        assert_eq!(session.hyper_gamma(), -1.0, "out of a warp");
        tick(&mut session);
        assert_eq!(session.hyper_gamma(), 0.0, "raised to 0 in the stage");
        tick_to(&mut session, 200);
        let v = warp::move_off_speed(400, warp::warp_factor(0), 364, false);
        assert!(warp::hyper_gamma(v) > 100.0, "{v}");
        assert_eq!(session.hyper_gamma(), warp::hyper_gamma(v));
        tick(&mut session);
        let v = warp::move_off_speed(402, warp::warp_factor(0), 364, false);
        assert_eq!(
            session.hyper_gamma(),
            warp::hyper_gamma(v),
            "set after the clamp"
        );
        session.arrive(&catalog, &mut NeverFires).expect("arrives");
        tick(&mut session);
        assert_eq!(session.hyper_gamma(), -1.0, "out of the stage");
    }

    #[test]
    fn the_hyper_gamma_is_lowered_to_100_at_the_top_of_a_pass() {
        let mut session = jumping(&catalog());
        session.warp.gamma = 250.0;
        session.gamma_tick();
        assert_eq!(session.hyper_gamma(), 100.0);
        session.warp.gamma = -3.0;
        session.gamma_tick();
        assert_eq!(session.hyper_gamma(), 0.0);
    }

    #[test]
    fn with_hyperspace_effects_off_the_hyper_gamma_never_lights() {
        let mut session = jumping(&catalog());
        session.set_hyperspace_effects(false);
        for _ in 0..200 {
            tick(&mut session);
            assert_eq!(session.hyper_gamma(), -1.0);
        }
    }

    // The glow.

    #[test]
    fn the_move_off_runs_the_glow_up_to_31() {
        let mut session = jumping(&catalog());
        tick_to(&mut session, 37);
        assert_eq!(session.engine_glow(), 0, "coasting");
        tick(&mut session);
        assert_eq!(session.engine_glow(), 4, "-1 at none, +4");
        tick_to(&mut session, 60);
        assert_eq!(
            session.engine_glow(),
            31,
            "+4 to 32, less the fall above 24"
        );
    }

    #[test]
    fn a_braking_thrust_raises_the_glow_by_3_while_it_is_at_most_23() {
        // Moving up, facing down, against its motion: it thrusts.
        let mut session = bound_for_131(&catalog(), OUT, Vec2::new(0.0, -5.0), 180.0);
        session.begin_jump().expect("jumps");
        tick(&mut session);
        assert!(session.thrusting, "the brake's thrust");
        assert_eq!(session.engine_glow(), 2, "+3, then the coast's -1");
        session.engine_glow = 23;
        tick(&mut session);
        assert_eq!(session.engine_glow(), 24, "26, -1, -1 above 24");
        session.engine_glow = 24;
        tick(&mut session);
        assert_eq!(session.engine_glow(), 23, "no raise above 23");
    }

    // The warp-up's pitch.

    #[test]
    fn the_warp_up_plays_pitched_by_w_for_its_pitched_length() {
        let mut session = jumping(&catalog());
        let rate = warp::warp_up_rate(warp::warp_factor(0));
        assert_eq!(rate, 50_412);
        assert_eq!(session.take_sounds(), [SimSound::JumpBegan { rate }]);
        // 364 × 65536 / 50412 = 473 Mac ticks: 237 ticks.
        for _ in 0..236 {
            tick(&mut session);
        }
        assert!(session.warp_ups.playing(false));
        tick(&mut session);
        assert!(!session.warp_ups.playing(false));
    }

    // The warp's end.

    #[test]
    fn a_shuttle_jump_ends_a_pass_after_its_fallback_past_d_over_w() {
        // w 1.3, d 364: the fallback at t > 280 Mac ticks, clock 141, the
        // warp-up playing on to 473; the end on the pass after.
        let catalog = catalog();
        let mut session = jumping(&catalog);
        tick_to(&mut session, 141);
        assert!(!session.warp_end_due(), "the fallback's pass");
        assert!(session.warp_ups.playing(false));
        tick(&mut session);
        assert!(session.warp_end_due(), "the pass after");
        assert!(session.warp_stage() > 30.0);
        session.take_sounds();
        session.arrive(&catalog, &mut NeverFires).expect("arrives");
        assert_eq!(
            session.take_sounds()[..2],
            [SimSound::Halt(&WARP_UP_HALTS), SimSound::Arrived],
            "the fallback's halt and warp-out"
        );
        assert!(!session.warp_end_due());
    }

    #[test]
    fn a_slow_jumpers_warp_ends_naturally_as_its_pitched_warp_up_ends() {
        // w 0.91: the warp-up plays at 72017, 364 × 65536 / 72017 = 331
        // Mac ticks, over after 166 ticks, before d / w = 400.
        let catalog = flagged(0x0001);
        let mut session = jumping(&catalog);
        assert_eq!(session.player_warp_factor(), warp::warp_factor(0x0001));
        tick_to(&mut session, 165);
        assert!(session.warp_ups.playing(false));
        assert!(!session.warp_end_due());
        tick(&mut session);
        assert!(!session.warp_ups.playing(false));
        assert!(session.warp_end_due(), "the natural end");
        session.take_sounds();
        session.arrive(&catalog, &mut NeverFires).expect("arrives");
        assert_eq!(
            session.take_sounds()[..2],
            [SimSound::Halt(&WARP_UP_HALTS), SimSound::Arrived],
            "the warp-out, the warp-ups already over"
        );
    }

    #[test]
    fn a_warp_up_played_clears_the_warp_sound_flag_a_cancelled_warp_left() {
        // Left set by a warp given up a pass after its fallback, the flag
        // would end the next warp at a stage of 30 though its warp-up
        // plays (@0x6b695 clears it as the warp-up plays).
        let mut session = bound_for_131(&catalog(), OUT, Vec2::ZERO, 90.0);
        session.warp_sounds = [364, 252];
        session.warp.sound_done = true;
        session.begin_jump().expect("jumps");
        for _ in 0..40 {
            tick(&mut session);
            assert!(!session.warp_end_due(), "{}", session.warp_stage());
        }
    }

    #[test]
    fn a_warp_ends_only_past_a_stage_of_30() {
        // d 0: t > d / w at once, and no warp-up plays; the stage reaches
        // 31 on the 29th pass.
        let mut session = bound_for_131(&catalog(), OUT, Vec2::ZERO, 90.0);
        session.warp_sounds = [0, 0];
        session.begin_jump().expect("jumps");
        for _ in 0..28 {
            tick(&mut session);
            assert!(!session.warp_end_due(), "{}", session.warp_stage());
        }
        assert_eq!(session.warp_stage(), 30.0);
        tick(&mut session);
        assert!(session.warp_end_due());
    }

    #[test]
    fn a_warp_ending_while_the_ship_still_turns_begins_the_jump() {
        // 1° a tick, 180° off the bearing: still turning when the warp
        // ends at a stage past 30, its 29th pass.
        let mut catalog = catalog();
        catalog.ships = vec![(
            ShipId(128),
            Ok(crate::handling::ShipFields {
                maneuver: 10,
                ..FAST
            }),
        )];
        let mut session = bound_for_131(&catalog, OUT, Vec2::ZERO, 270.0);
        session.warp_sounds = [0, 0];
        session.begin_jump().expect("jumps");
        for _ in 0..28 {
            tick(&mut session);
        }
        assert_eq!(session.preparing_jump(), Some(SystemId(131)), "turning");
        assert!(!session.warp_end_due());
        tick(&mut session);
        assert_eq!(session.jumping(), Some(SystemId(131)));
        assert!(session.warp_end_due());
    }
}
