//! An NPC's warp out of the system: its warp stage, the original's +0x50
//! and +0xa4, the sub-goals that drive them, and the motion `_HandleShip`
//! gives a ship in it.
//!
//! **The stage** ([`Warp`]). +0x50 is above 0 while a ship's jump is
//! under way; +0xa4 stamps its clock, `TickCount`, which here is the
//! advance passes since the stamp ([`Warp::since`]), moved on once a tick
//! ([`Warp::tick_clock`]), as `TickCount` is real time and double time's
//! second pass doesn't move it. `_AIIsShipJumping` (@0x82bf9-0x82c43) is
//! a goal test: goal +0xc8c8 2, 3 or 0xb with sub-goal +0xc8ca 4 or 0xd
//! ([`Npc::jumping`]).
//!
//! **The high level's hold** ([`hold_stage`], `_HighLevelAIHandler`
//! @0x8d513-0x8d56b): on each advance pass a ship whose goal has left
//! its jump ends its stage, and a ship the player leads with its stage
//! under way is held to the player's jump.
//!
//! **Sub-goal 4, its own jump** (`_LowLevelAIHandler` @0x8585b-0x859c5;
//! [`own_jump`]), each advance pass (`_AIDispatch` runs on it alone):
//!
//! - its desired heading +0x68 is outward, `_CalcInterceptAngle` from
//!   (0, 0) to its position (@0x85875-0x8589e);
//! - with +0x50 at 0 or below, +0x50 = 1.0 and the clock is stamped
//!   (@0x858a4-0x858b0);
//! - the clock guard restamps a stamp later than `TickCount`
//!   (@0x858b6-0x858de), which a count of ticks since the stamp never
//!   meets;
//! - +0x50 grows by `_gSpeedMult`, 1.0 (@0x858e4-0x858f9);
//! - once t ≥ d / w on its own clock and its class's warp factor, it is
//!   gone ([`npc_warped_out`]): +0x50 = 0, +0xb8 = 0, +0x74 = −1
//!   (@0x859ae-0x859bf). A mission ship's record +0x30 and +0x6 are
//!   counted there too (@0x85960-0x859ab), which the session's
//!   `mission_ships` module does for [`Outcome::JumpedOut`]. It plays no
//!   sound.
//!
//! A ship enters it where it reaches its jump distance under
//! [`Goal::JumpOut`], or runs out from its attacker beyond its reach
//! (see [`autopilot`](crate::traffic::autopilot)): [`Npc::begin_own_jump`].
//!
//! **Goal 0xb, its leader's jump** (`_HighLevelAIHandler`
//! @0x8de63-0x8df75; [`follow_check`]), each advance pass before the low
//! level: with no leader it jumps on its own (goal 2, sub-goal 4); led by
//! the player, it follows (sub-goal 0xd) while the player's +0x50 is
//! above 0, else it gives up (goal 0); led by an NPC, the leader is put
//! on goal 2, and it follows while the leader's sub-goal is 1 or 4, else
//! gives up. `_MakeEscortsHyperWithParent` (@0x3fd54-0x3fdea) puts the
//! ships the player leads on it each warp-stage frame of the player
//! ([`Npc::follow_player_jump`]); the NPC escorts' entry, `_EscortAI`
//! (@0x839fd-0x83a92), is rdm:phase/escort-fleet-fidelity/phase-8-jump-with-leader's.
//!
//! **Sub-goal 0xd, following** (@0x859c5-0x85c5f; [`follow_jump`]): its
//! +0x68 is its leader's whole-degree heading. While the leader's +0x50
//! is 1.0 or below it takes the leader's velocity and keeps formation
//! (`_AIMaintainFormation` @0x85a28) and copies its glow. Otherwise, after
//! the clock guard: past 30.0, a ship an NPC leads leaves it and jumps on
//! its own (goal 2, sub-goal 4, its +0x68 the leader's, its AI type +0x88
//! its class's, @0x85a8f-0x85af1); while the leader is more than 10° off
//! its own +0x68 (@0x85af8-0x85b33), more than this ship's turn rate
//! off gives up its stage (+0x50 = −1.0) and aims at the leader's +0x68,
//! and within it waits (+0x4c = 180.0, @0x85bc3); within 10° it starts
//! its stage (+0x50 from 0 to 1.0, stamped), its velocity × 0.95, its
//! thrust off, and +0x50 grows by 1.0 (@0x85bcf-0x85c5a).
//!
//! **`_HandleShip`'s warp-out** (@0x346b3-0x34a31; [`warp_block`]), on
//! both passes, for any ship with +0x50 above 0 that is not disabled
//! (`_IsDisabled`) and while *0x3b523c is clear: facing more than its
//! turn rate off its +0x68 (`_AngularDifference` of the whole-degree
//! heading, @0x346f2-0x34741), it restamps its clock (@0x34a31-0x34a41);
//! facing it but not jumping, +0x50 = 0 (@0x34a27); else its velocity ×
//! 0.95 (@0x34762-0x3478c), a led ship takes its leader's +0x68
//! (@0x34796-0x347ba), and it moves its position, not its velocity, by
//! v × `_gSpeedMult` along its whole-degree heading (`_Accel` on +0x18,
//! @0x349ca-0x349f7) while v > 0: [`npc_warp_out_speed`] on the player's
//! clock for a ship the player leads, only while the player's +0x50 is
//! above 1.0 (else v stays 0), and on its own clock otherwise. So the
//! displacement is under no speed cap. *0x3b523c is set only around the
//! priming `_DoPlayGameWork` of a game's start and a system's setup
//! (`_PlayGame` @0x457ff, @0x1a380) and in `_PrepareAllSprites`
//! (@0x3a773), never in play, so it is always clear here. It also skips a
//! ship's position step and turn (@0x34264, @0x34431).
//!
//! The ship flies its warp with no thrust and no fire: `_HandleShip`
//! skips the thrust of a ship with +0x50 above 0 and no burst word
//! (@0x34a96), and sub-goals 4 and 0xd call no `_AIFire*`, so the fire
//! flag the low level clears each frame (@0x851f6) stays clear. It turns
//! to its +0x68 and moves on at its velocity before the warp-out, as
//! `_HandleShip`'s position step (@0x34264) and turn (@0x3443f) come
//! first.
//!
//! **Left out.**
//!
//! - The warp-out's glow, +0xc8d4 += 3 to 32 (@0x349fc-0x34a25), and
//!   sub-goal 0xd's glow copy (@0x85a2d-0x85a4a): NPCs have no glow yet;
//!   rdm:phase/ship-handling-fidelity/phase-10-ship-drawing owns it.
//! - Sub-goal 0xd's nav copy, +0x6c (@0x85bf1-0x85bf6), and goal 0xb's
//!   (@0x8de9d-0x8deb0, @0x8df18-0x8df22): the engine's NPCs keep no
//!   travel stellar for a jump; the system they jump to is never read.
//! - The detach's AI type, +0x88 = class +0x12 (@0x85ada-0x85af1): the
//!   engine keeps one AI type a ship; its own jump flies the same.
//! - Sub-goal 0xd's frame after the detach reads its leader as ship −1
//!   (@0x85af8-0x85b10, +0x9a just cleared), out of the ship table; the
//!   engine ends the frame at the detach.
//! - The thrust skip of a ship whose goal left its jump while its stage
//!   is still above 0 and it does not yet face its +0x68 (@0x34a96): the
//!   autopilot flies it as its new goal says, its thrust included, until
//!   the warp-out's test zeroes its stage.

use std::collections::BTreeMap;

use crate::ai::Goal;
use crate::combat::ShipRef;
use crate::combat::hull::Condition;
use crate::flight::{Controls, FlightRules};
use crate::geometry::Vec2;
use crate::handling::Handling;
use crate::traffic::Outcome;
use crate::traffic::World;
use crate::traffic::autopilot::{TopSpeeds, steer, step};
use crate::traffic::inertialess::{self, Inertia};
use crate::traffic::npc::{Mode, Npc, NpcId};
use crate::trig::{accel, angular_difference, intercept_angle, whole};
use crate::warp::{
    WarpOutClock, mac_ticks, npc_warp_out_speed, npc_warped_out, warp_sound_duration,
};

/// The velocity's drag a tick of the warp-out, and of a follow's stage
/// start (0.95, f64 @0xdd618).
pub const WARP_OUT_DRAG: f64 = 0.95;

/// How far, in whole degrees, a leader may face off its own desired
/// heading for its followers to start their stage (`cmpl $0xa`
/// @0x85b30).
pub const FOLLOW_ARC: i32 = 10;

/// The AI wait a follower takes while its leader turns within its own
/// turn (+0x4c = 180.0, @0x85bc3).
pub const FOLLOW_WAIT: u32 = 180;

/// The stage past which a ship an NPC leads leaves it to jump on its
/// own (30.0, f32 @0xdd084).
pub const DETACH_STAGE: f32 = 30.0;

/// Whose jump an NPC's warp follows: its sub-goal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WarpGoal {
    /// Sub-goal 4: its own.
    Own,
    /// Goal 0xb's sub-goal 0xd: its leader's.
    Follow(ShipRef),
}

/// An NPC's warp stage (see the module docs): none as it spawns.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Warp {
    /// The original's +0x50: above 0 while its jump is under way.
    pub stage: f32,
    /// The advance passes since its clock was stamped, +0xa4.
    pub since: u32,
    /// The desired heading its warp sets, +0x68, in whole degrees.
    pub aim: i16,
    /// The sub-goal it jumps by, if any.
    pub goal: Option<WarpGoal>,
}

impl Warp {
    /// Whether its jump is under way: +0x50 above 0.
    #[must_use]
    pub fn under_way(&self) -> bool {
        self.stage > 0.0
    }

    /// Moves its clock on a tick, as `TickCount` moves on between advance
    /// passes.
    pub fn tick_clock(&mut self) {
        self.since = self.since.saturating_add(1);
    }

    /// Stamps its clock, as +0xa4 = `TickCount` does.
    pub fn stamp(&mut self) {
        self.since = 0;
    }
}

impl Npc {
    /// Whether it is jumping, `_AIIsShipJumping` (@0x82bf9-0x82c43): on
    /// its own jump (sub-goal 4) while its goal is still to jump out or to
    /// flee (the original's goals 2 and 3), or following its leader's
    /// (sub-goal 0xd).
    #[must_use]
    pub fn jumping(&self) -> bool {
        match self.warp.goal {
            Some(WarpGoal::Own) => matches!(self.goal, Goal::JumpOut | Goal::Flee(_)),
            Some(WarpGoal::Follow(_)) => true,
            None => false,
        }
    }

    /// Starts its own jump, sub-goal 4 (see the module docs): a fleeing
    /// ship keeps fleeing, and any other's goal is to jump out.
    pub fn begin_own_jump(&mut self) {
        if !matches!(self.goal, Goal::Flee(_)) {
            self.goal = Goal::JumpOut;
        }
        self.warp.goal = Some(WarpGoal::Own);
    }

    /// Puts it on its leader's jump, the player's, as
    /// `_MakeEscortsHyperWithParent` does for each ship the player leads
    /// (@0x3fd89-0x3fdd0): it takes the player's +0x50 `stage` and clock,
    /// `since`, lets go of its target (+0x70 = −2, then −1), and goes to
    /// goal 0xb (`_AIMakeShipJumpWithParent` @0x82bb6: +0x96 = 0, and
    /// with +0x50 at 0 or below, 1.0 and a stamp, which the copy just
    /// made leaves to a player not yet in its warp stage).
    pub fn follow_player_jump(&mut self, stage: f32, since: u32) {
        self.warp.stage = stage;
        self.warp.since = since;
        self.target = None;
        self.provoked = 0.0;
        self.warp.goal = Some(WarpGoal::Follow(ShipRef::Player));
        if self.warp.stage <= 0.0 {
            self.warp.stage = 1.0;
            self.warp.stamp();
        }
    }
}

/// What the followers of a leader read of its warp: the player's or an
/// NPC's.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LeaderWarp {
    /// Its +0x50.
    pub stage: f32,
    /// The advance passes since its clock's stamp.
    pub since: u32,
    /// Its class's warp factor.
    pub factor: f32,
    /// Its heading.
    pub heading: f32,
    /// Its desired heading, +0x68, in whole degrees.
    pub aim: i16,
    /// Its velocity.
    pub velocity: Vec2,
    /// Whether it is on its way out, the original's sub-goal 1 or 4
    /// (goal 0xb's test, @0x8defc-0x8df13): an NPC's goal to jump out.
    pub leaving: bool,
}

/// What the traffic reads of the warps: the player's, the warp sound's
/// duration d and double time.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WarpWorld {
    /// The player's warp.
    pub player: LeaderWarp,
    /// The warp sound's durations, `snd ` 128's and 129's, in Mac ticks.
    pub durations: [u32; 2],
    /// Whether double time is on.
    pub double_time: bool,
}

impl WarpWorld {
    /// The warp sound's duration d now (`_WarpSoundDuration`).
    #[must_use]
    pub fn duration(&self) -> u32 {
        warp_sound_duration(self.durations, self.double_time)
    }
}

/// What goal 0xb's check says of a follower (`_HighLevelAIHandler`
/// @0x8de63-0x8df75).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FollowCheck {
    /// Its leader is jumping: it follows (sub-goal 0xd).
    Follow,
    /// Its leader is not: it gives up (goal 0, sub-goal 0).
    GiveUp,
    /// It has no leader: it jumps on its own (goal 2, sub-goal 4).
    Own,
}

/// Goal 0xb's check of a follower whose leader is `leader`: none when it
/// has none (+0x9a = −1), else whether it is the player and its warp.
#[must_use]
pub fn follow_check(leader: Option<(bool, LeaderWarp)>) -> FollowCheck {
    match leader {
        None => FollowCheck::Own,
        Some((true, player)) if player.stage > 0.0 => FollowCheck::Follow,
        Some((false, npc)) if npc.leaving => FollowCheck::Follow,
        Some(_) => FollowCheck::GiveUp,
    }
}

/// One advance pass of sub-goal 4 for `npc` (see the module docs), its
/// warp sound's duration `duration`: whether it is gone.
pub fn own_jump(npc: &mut Npc, duration: u32) -> bool {
    npc.warp.aim = intercept_angle(Vec2::ZERO, npc.state.position);
    if npc.warp.stage <= 0.0 {
        npc.warp.stage = 1.0;
        npc.warp.stamp();
    }
    npc.warp.stage += 1.0;
    if npc_warped_out(mac_ticks(npc.warp.since), npc.stats.warp_factor, duration) {
        npc.warp = Warp::default();
        return true;
    }
    false
}

/// `_HighLevelAIHandler`'s hold on a ship's warp (@0x8d513-0x8d56b), on
/// each advance pass: a ship whose goal is no longer a jump's (2, 3 or
/// 0xb: [`Goal::JumpOut`], [`Goal::Flee`], or following its leader's)
/// ends its stage, +0x50 = 0; a ship the player leads whose stage is
/// under way is held to the player's jump, goal 0xb. Any other ship
/// under way keeps goal 2 or 3, by whether it holds a target, which the
/// engine's jump-out and flee goals already are.
pub fn hold_stage(npc: &mut Npc) {
    let jumping_goal = matches!(npc.goal, Goal::JumpOut | Goal::Flee(_))
        || matches!(npc.warp.goal, Some(WarpGoal::Follow(_)));
    if !jumping_goal {
        npc.warp.stage = 0.0;
    }
    if npc.warp.under_way() && npc.escorts_player() {
        npc.warp.goal = Some(WarpGoal::Follow(ShipRef::Player));
    }
}

/// What a pass of sub-goal 0xd did (see [`follow_jump`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FollowStep {
    /// Its leader is not yet in its warp stage: it took its velocity and
    /// keeps formation, as the autopilot flies it.
    Formation,
    /// It left its NPC leader to jump on its own.
    Detached,
    /// Its leader still turns: it gave up its stage, or waits.
    Waiting,
    /// Its stage runs.
    Warping,
}

/// One advance pass of sub-goal 0xd for `npc`, following `leader`, the
/// player when `player` (see the module docs).
pub fn follow_jump(npc: &mut Npc, leader: LeaderWarp, player: bool) -> FollowStep {
    npc.warp.aim = whole(leader.heading);
    if leader.stage <= 1.0 {
        npc.state.velocity = leader.velocity;
        return FollowStep::Formation;
    }
    if npc.warp.stage > DETACH_STAGE && !player {
        npc.leader = None;
        npc.warp.aim = leader.aim;
        npc.begin_own_jump();
        return FollowStep::Detached;
    }
    let off = i32::from(angular_difference(whole(leader.heading), leader.aim)).abs();
    if off > FOLLOW_ARC {
        #[allow(clippy::cast_precision_loss)]
        if off as f32 > npc.stats.handling.turn_rate {
            npc.warp.stage = -1.0;
            npc.warp.aim = leader.aim;
        } else {
            npc.ai_wait = FOLLOW_WAIT;
        }
        return FollowStep::Waiting;
    }
    if npc.warp.stage == 0.0 {
        npc.warp.stage = 1.0;
        npc.warp.stamp();
    }
    npc.state.velocity = drag(npc.state.velocity);
    npc.warp.stage += 1.0;
    FollowStep::Warping
}

/// `velocity` × [`WARP_OUT_DRAG`] a component, worked in f64 and stored
/// as an f32 (@0x34762-0x3478c, @0x85c10-0x85c42).
#[allow(clippy::cast_possible_truncation)]
fn drag(velocity: Vec2) -> Vec2 {
    let scale = |component: f32| (f64::from(component) * WARP_OUT_DRAG) as f32;
    Vec2::new(scale(velocity.x), scale(velocity.y))
}

/// Whose clock and desired heading a ship's warp-out reads (see
/// [`warp_block`]): the player's, for a ship the player leads; an NPC
/// leader's +0x68, for a ship an NPC leads; or its own alone.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Led {
    /// The player leads it.
    ByPlayer,
    /// An NPC leads it, whose desired heading is this.
    ByNpc(i16),
    /// No one does.
    No,
}

/// One pass of `_HandleShip`'s warp-out for `npc`, led as `led` says, in
/// `world` (see the module docs): nothing unless its stage is under way
/// and it is intact.
pub fn warp_block(npc: &mut Npc, led: Led, world: &WarpWorld) {
    if !npc.warp.under_way() || npc.condition != Condition::Intact {
        return;
    }
    let off = f32::from(angular_difference(whole(npc.state.heading), npc.warp.aim).abs());
    if npc.stats.handling.turn_rate < off {
        npc.warp.stamp();
        return;
    }
    if !npc.jumping() {
        npc.warp.stage = 0.0;
        return;
    }
    npc.state.velocity = drag(npc.state.velocity);
    let duration = world.duration();
    let v = match led {
        Led::ByPlayer => {
            npc.warp.aim = world.player.aim;
            if world.player.stage > 1.0 {
                npc_warp_out_speed(
                    WarpOutClock::Player,
                    mac_ticks(world.player.since),
                    world.player.factor,
                    duration,
                    world.double_time,
                )
            } else {
                0.0
            }
        }
        Led::ByNpc(_) | Led::No => {
            if let Led::ByNpc(aim) = led {
                npc.warp.aim = aim;
            }
            npc_warp_out_speed(
                WarpOutClock::Own,
                mac_ticks(npc.warp.since),
                npc.stats.warp_factor,
                duration,
                world.double_time,
            )
        }
    };
    if v > 0.0 {
        npc.state.position = npc.state.position + accel(whole(npc.state.heading), v);
    }
}

/// One pass of a jumping ship's flight before its warp-out: it moves on
/// at its velocity and turns toward its desired heading, with no thrust
/// and no fire (see the module docs).
pub fn fly(npc: &mut Npc, handling: &Handling, flight: FlightRules, inertia: Inertia) {
    let controls = Controls {
        turn: steer(npc.state.heading, f32::from(npc.warp.aim), handling),
        ..Controls::default()
    };
    npc.trigger = crate::combat::armament::Trigger::default();
    match inertia {
        Inertia::Inertial => step(npc, handling, controls, flight, inertia),
        Inertia::Inertialess { .. } => {
            npc.state.heading = turned(npc.state.heading, controls, handling);
            inertialess::drift(npc, handling);
        }
    }
}

/// `heading` turned a tick as `controls` say at `handling`'s turn rate,
/// kept within 0-360.
fn turned(heading: f32, controls: Controls, handling: &Handling) -> f32 {
    let by = match controls.turn {
        crate::flight::Turn::Left => -handling.turn_rate,
        crate::flight::Turn::Right => handling.turn_rate,
        crate::flight::Turn::None => 0.0,
    };
    (heading + by).rem_euclid(360.0)
}

/// Whether `outcome` is the departure of an own jump.
#[must_use]
pub fn departed(gone: bool) -> Option<Outcome> {
    gone.then_some(Outcome::JumpedOut)
}

/// What each of `npcs` shows its followers of its warp, as the tick's
/// passes begin.
#[must_use]
pub fn leaders(npcs: &[Npc]) -> BTreeMap<NpcId, LeaderWarp> {
    npcs.iter()
        .map(|npc| {
            let view = LeaderWarp {
                stage: npc.warp.stage,
                since: npc.warp.since,
                factor: npc.stats.warp_factor,
                heading: npc.state.heading,
                aim: npc.warp.aim,
                velocity: npc.state.velocity,
                leaving: npc.goal == Goal::JumpOut,
            };
            (npc.id, view)
        })
        .collect()
}

/// Goal 0xb's push of an NPC leader onto its own jump (goal 2,
/// @0x8dedf-0x8defa): each leader of a ship in `npcs` that follows its
/// jump and is intact is set to jump out.
pub fn send_leaders_out(npcs: &mut [Npc]) {
    let sent: Vec<NpcId> = npcs
        .iter()
        .filter(|npc| npc.condition == Condition::Intact)
        .filter_map(|npc| match npc.warp.goal {
            Some(WarpGoal::Follow(ShipRef::Npc(leader))) => Some(leader),
            _ => None,
        })
        .collect();
    for npc in npcs.iter_mut().filter(|npc| sent.contains(&npc.id)) {
        npc.goal = Goal::JumpOut;
    }
}

/// How `npc` is led, for its warp-out, among `leaders`.
fn led(npc: &Npc, leaders: &BTreeMap<NpcId, LeaderWarp>) -> Led {
    if npc.escorts_player() {
        Led::ByPlayer
    } else {
        npc.leader
            .and_then(|leader| leaders.get(&leader))
            .map_or(Led::No, |leader| Led::ByNpc(leader.aim))
    }
}

/// `_HandleShip`'s warp-out for `npc` after the autopilot flew it
/// ([`warp_block`]), among `leaders`, in `world`: for a ship whose stage is
/// under way though it is not jumping, or not yet.
pub fn settle(npc: &mut Npc, leaders: &BTreeMap<NpcId, LeaderWarp>, world: &WarpWorld) {
    if npc.warp.under_way() {
        warp_block(npc, led(npc, leaders), world);
    }
}

/// Flies `npc` one pass of its jump, the advance pass when `think`, among
/// `leaders`, in `world` at `speeds`: goal 0xb's check and its low level,
/// sub-goal 4 or 0xd ([`follow_check`], [`own_jump`], [`follow_jump`]) on
/// the advance pass, then its flight ([`fly`]) and its warp-out
/// ([`warp_block`]). None when it is not jumping, or follows a leader not
/// yet in its warp stage: the autopilot flies it as its goal says.
pub fn fly_jump(
    npc: &mut Npc,
    leaders: &BTreeMap<NpcId, LeaderWarp>,
    world: &World,
    speeds: TopSpeeds,
    think: bool,
) -> Option<Outcome> {
    if npc.mode != Mode::Flying || npc.condition != Condition::Intact {
        return None;
    }
    let goal = npc.warp.goal?;
    let leader = match goal {
        WarpGoal::Follow(ShipRef::Player) => Some((true, world.warp.player)),
        WarpGoal::Follow(ShipRef::Npc(id)) => leaders.get(&id).map(|&leader| (false, leader)),
        WarpGoal::Own => None,
    };
    if think {
        match goal {
            WarpGoal::Follow(_) => match follow_check(leader) {
                FollowCheck::Own => npc.begin_own_jump(),
                FollowCheck::GiveUp => {
                    npc.warp.goal = None;
                    return None;
                }
                FollowCheck::Follow => {
                    if let Some((player, leader)) = leader
                        && follow_jump(npc, leader, player) == FollowStep::Formation
                    {
                        return None;
                    }
                }
            },
            WarpGoal::Own => {
                if npc.jumping() && own_jump(npc, world.warp.duration()) {
                    return Some(Outcome::JumpedOut);
                }
            }
        }
    } else if leader.is_some_and(|(_, leader)| leader.stage <= 1.0) {
        return None;
    }
    if !npc.jumping() {
        return None;
    }
    let handling = Handling {
        max_speed: speeds.own,
        ..world.flight.handling(npc.stats.handling)
    };
    let inertia = Inertia::of(npc, None, world.flight);
    fly(npc, &handling, world.flight, inertia);
    warp_block(npc, led(npc, leaders), &world.warp);
    Some(Outcome::Flying)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::flight::ShipState;
    use crate::stats::ShipStats;
    use crate::traffic::fixture::npc;
    use crate::warp::warp_factor;

    /// 3° a tick, w 1.3.
    fn stats() -> ShipStats {
        ShipStats {
            handling: Handling {
                max_speed: 5.0,
                accel: 0.2,
                turn_rate: 3.0,
            },
            warp_factor: warp_factor(0),
            ..ShipStats::default()
        }
    }

    /// An NPC 2000 out east of the centre, facing east, at rest.
    fn ship() -> Npc {
        let mut ship = npc(1, stats());
        ship.state = ShipState {
            position: Vec2::new(2000.0, 0.0),
            velocity: Vec2::ZERO,
            heading: 90.0,
        };
        ship
    }

    /// d 364 out of double time, the player at rest in no warp.
    fn world() -> WarpWorld {
        WarpWorld {
            player: LeaderWarp::default(),
            durations: [364, 252],
            double_time: false,
        }
    }

    // `_AIIsShipJumping`.

    #[test]
    fn a_ship_is_jumping_on_its_own_jump_or_its_leaders() {
        let mut ship = ship();
        assert!(!ship.jumping());
        ship.begin_own_jump();
        assert_eq!(ship.goal, Goal::JumpOut);
        assert!(ship.jumping());
        ship.goal = Goal::Flee(ShipRef::Player);
        assert!(ship.jumping(), "goal 3, fleeing");
        ship.goal = Goal::Idle;
        assert!(!ship.jumping(), "its goal left the jump");
        ship.warp.goal = Some(WarpGoal::Follow(ShipRef::Player));
        assert!(ship.jumping());
    }

    // The high level's hold on a warp.

    #[test]
    fn a_ship_whose_goal_left_its_jump_ends_its_stage_at_the_high_level() {
        let mut ship = ship();
        ship.begin_own_jump();
        ship.warp.stage = 5.0;
        hold_stage(&mut ship);
        assert_eq!(ship.warp.stage, 5.0, "goal 2 keeps it");
        ship.goal = Goal::Flee(ShipRef::Player);
        hold_stage(&mut ship);
        assert_eq!(ship.warp.stage, 5.0, "goal 3 keeps it");
        ship.goal = Goal::Attack(ShipRef::Player);
        hold_stage(&mut ship);
        assert_eq!(ship.warp.stage, 0.0, "+0x50 = 0 (@0x8d52e)");
        let mut follower = self::ship();
        follower.goal = Goal::Idle;
        follower.warp.goal = Some(WarpGoal::Follow(ShipRef::Player));
        follower.warp.stage = -1.0;
        hold_stage(&mut follower);
        assert_eq!(follower.warp.stage, -1.0, "goal 0xb keeps it");
    }

    #[test]
    fn a_ship_the_player_leads_with_its_stage_under_way_is_held_to_the_players_jump() {
        let mut ship = ship();
        ship.escort = Some(crate::escort::EscortDuty {
            formation: crate::escort::FormationSlot::default(),
            order: None,
        });
        ship.goal = Goal::JumpOut;
        ship.warp.stage = 3.0;
        hold_stage(&mut ship);
        assert_eq!(
            ship.warp.goal,
            Some(WarpGoal::Follow(ShipRef::Player)),
            "goal 0xb (@0x8d54c)"
        );
    }

    // Sub-goal 4.

    #[test]
    fn its_own_jump_aims_outward_starts_its_stage_and_grows_it() {
        let mut ship = ship();
        ship.state.position = Vec2::new(0.0, -2000.0);
        ship.begin_own_jump();
        assert!(!own_jump(&mut ship, 364));
        assert_eq!(ship.warp.aim, 0, "straight up, out");
        assert_eq!(ship.warp.stage, 2.0, "1.0, then 1.0 more");
        assert_eq!(ship.warp.since, 0, "stamped");
        ship.warp.tick_clock();
        assert!(!own_jump(&mut ship, 364));
        assert_eq!(ship.warp.stage, 3.0);
        assert_eq!(ship.warp.since, 1, "not restamped");
    }

    #[test]
    fn its_own_jump_is_gone_once_t_reaches_d_over_w() {
        let mut ship = ship();
        ship.begin_own_jump();
        own_jump(&mut ship, 364);
        // 364 / 1.3 = 280 Mac ticks: 140 ticks.
        ship.warp.since = 139;
        assert!(!own_jump(&mut ship, 364), "278 Mac ticks");
        ship.warp.since = 140;
        assert!(own_jump(&mut ship, 364), "280");
        assert_eq!(ship.warp, Warp::default(), "+0x50 = 0");
        assert_eq!(departed(true), Some(Outcome::JumpedOut));
        assert_eq!(departed(false), None);
    }

    // Goal 0xb.

    #[test]
    fn goal_0xb_follows_a_jumping_leader_and_gives_up_on_one_that_is_not() {
        let jumping = LeaderWarp {
            stage: 0.5,
            ..LeaderWarp::default()
        };
        assert_eq!(follow_check(None), FollowCheck::Own);
        assert_eq!(follow_check(Some((true, jumping))), FollowCheck::Follow);
        assert_eq!(
            follow_check(Some((true, LeaderWarp::default()))),
            FollowCheck::GiveUp
        );
        let leaving = LeaderWarp {
            leaving: true,
            ..LeaderWarp::default()
        };
        assert_eq!(follow_check(Some((false, leaving))), FollowCheck::Follow);
        assert_eq!(follow_check(Some((false, jumping))), FollowCheck::GiveUp);
    }

    // `_MakeEscortsHyperWithParent`.

    #[test]
    fn a_ship_the_player_leads_takes_the_players_stage_and_clock_and_lets_go_of_its_target() {
        let mut ship = ship();
        ship.target = Some(ShipRef::Npc(crate::traffic::npc::NpcId(9)));
        ship.provoked = 12.0;
        ship.follow_player_jump(5.0, 7);
        assert_eq!(ship.warp.stage, 5.0);
        assert_eq!(ship.warp.since, 7);
        assert_eq!(ship.target, None);
        assert_eq!(ship.provoked, 0.0);
        assert_eq!(ship.warp.goal, Some(WarpGoal::Follow(ShipRef::Player)));
        ship.follow_player_jump(0.0, 7);
        assert_eq!(ship.warp.stage, 1.0, "a stage of none starts at 1.0");
        assert_eq!(ship.warp.since, 0, "stamped");
    }

    // Sub-goal 0xd.

    /// A leader heading east toward east, in its warp stage.
    fn leader() -> LeaderWarp {
        LeaderWarp {
            stage: 5.0,
            since: 3,
            factor: warp_factor(0),
            heading: 90.4,
            aim: 90,
            velocity: Vec2::new(1.0, 0.5),
            leaving: true,
        }
    }

    #[test]
    fn a_follower_of_a_leader_not_yet_warping_takes_its_velocity() {
        let mut ship = ship();
        let slow = LeaderWarp {
            stage: 1.0,
            ..leader()
        };
        assert_eq!(follow_jump(&mut ship, slow, true), FollowStep::Formation);
        assert_eq!(ship.state.velocity, slow.velocity);
        assert_eq!(ship.warp.aim, 90, "its leader's whole-degree heading");
    }

    #[test]
    fn a_follower_starts_its_stage_once_its_leader_faces_within_10() {
        let mut ship = ship();
        ship.state.velocity = Vec2::new(2.0, 0.0);
        assert_eq!(follow_jump(&mut ship, leader(), true), FollowStep::Warping);
        assert_eq!(ship.warp.stage, 2.0, "0 to 1.0, then 1.0 more");
        assert_eq!(ship.warp.since, 0);
        assert_eq!(ship.state.velocity, Vec2::new(1.9, 0.0));
        let turning = LeaderWarp {
            aim: 100,
            ..leader()
        };
        assert_eq!(
            follow_jump(&mut ship, turning, true),
            FollowStep::Warping,
            "10 off is within"
        );
        assert_eq!(ship.warp.stage, 3.0, "not restarted");
    }

    #[test]
    fn a_follower_of_a_turning_leader_gives_up_or_waits_by_its_own_turn() {
        let mut ship = ship();
        ship.warp.stage = 4.0;
        let turning = LeaderWarp {
            aim: 101,
            ..leader()
        };
        // 11 off, more than its 3° a tick: it gives up its stage.
        assert_eq!(follow_jump(&mut ship, turning, true), FollowStep::Waiting);
        assert_eq!(ship.warp.stage, -1.0);
        assert_eq!(ship.warp.aim, 101, "its leader's +0x68");
        assert_eq!(ship.ai_wait, 0);
        let mut nimble = ship.clone();
        nimble.stats.handling.turn_rate = 11.0;
        nimble.warp.stage = 4.0;
        assert_eq!(follow_jump(&mut nimble, turning, true), FollowStep::Waiting);
        assert_eq!(nimble.warp.stage, 4.0);
        assert_eq!(nimble.ai_wait, FOLLOW_WAIT, "within its turn, it waits");
        assert!(nimble.wait_running());
    }

    #[test]
    fn past_30_a_follower_leaves_its_npc_leader_but_never_the_player() {
        let mut ship = ship();
        ship.leader = Some(crate::traffic::npc::NpcId(4));
        ship.warp.goal = Some(WarpGoal::Follow(ShipRef::Npc(crate::traffic::npc::NpcId(
            4,
        ))));
        ship.warp.stage = 30.5;
        let lead = LeaderWarp {
            aim: 77,
            ..leader()
        };
        assert_eq!(follow_jump(&mut ship, lead, false), FollowStep::Detached);
        assert_eq!(ship.leader, None);
        assert_eq!(ship.warp.aim, 77);
        assert_eq!(ship.warp.goal, Some(WarpGoal::Own));
        assert_eq!(ship.goal, Goal::JumpOut);
        let mut by_player = self::ship();
        by_player.warp.stage = 30.5;
        assert_eq!(
            follow_jump(&mut by_player, leader(), true),
            FollowStep::Warping
        );
        let mut at = self::ship();
        at.warp.stage = 30.0;
        assert_eq!(follow_jump(&mut at, leader(), false), FollowStep::Warping);
    }

    // `_HandleShip`'s warp-out.

    /// A ship on its own jump, facing its aim, east, `stage` in and `since`
    /// ticks into its clock, moving east at 2.
    fn warping(stage: f32, since: u32) -> Npc {
        let mut ship = ship();
        ship.begin_own_jump();
        ship.warp.stage = stage;
        ship.warp.since = since;
        ship.warp.aim = 90;
        ship.state.velocity = Vec2::new(2.0, 0.0);
        ship
    }

    #[test]
    fn the_warp_out_drags_the_velocity_and_displaces_the_position_on_its_own_clock() {
        let world = world();
        // Own clock: 0.35 × 364 / 1.69 = 75.4 Mac ticks before it moves.
        let mut early = warping(3.0, 37);
        warp_block(&mut early, Led::No, &world);
        assert_eq!(early.state.velocity, Vec2::new(1.9, 0.0), "× 0.95");
        assert_eq!(early.state.position, Vec2::new(2000.0, 0.0), "v ≤ 0");
        let mut moving = warping(3.0, 100);
        warp_block(&mut moving, Led::No, &world);
        let v = npc_warp_out_speed(WarpOutClock::Own, 200, warp_factor(0), 364, false);
        assert!(v > 0.0 && v < 50.0, "{v}");
        assert_eq!(
            moving.state.position,
            Vec2::new(2000.0, 0.0) + accel(90, v),
            "along its whole-degree heading, by v"
        );
        assert_eq!(
            moving.state.velocity,
            Vec2::new(1.9, 0.0),
            "the velocity only drags"
        );
    }

    #[test]
    fn the_warp_out_is_capped_at_50_a_pass() {
        let mut far = warping(3.0, 100_000);
        warp_block(&mut far, Led::No, &world());
        assert_eq!(far.state.position, Vec2::new(2000.0, 0.0) + accel(90, 50.0));
    }

    #[test]
    fn the_warp_out_runs_along_the_whole_degree_heading() {
        let mut ship = warping(3.0, 100_000);
        ship.state.heading = 91.9;
        ship.warp.aim = 91;
        warp_block(&mut ship, Led::No, &world());
        assert_eq!(
            ship.state.position,
            Vec2::new(2000.0, 0.0) + accel(91, 50.0)
        );
    }

    #[test]
    fn a_ship_not_facing_its_aim_restamps_its_clock_and_does_not_move() {
        let mut ship = warping(3.0, 100);
        ship.warp.aim = 95;
        warp_block(&mut ship, Led::No, &world());
        assert_eq!(ship.warp.since, 0, "restamped");
        assert_eq!(ship.state.position, Vec2::new(2000.0, 0.0));
        assert_eq!(ship.state.velocity, Vec2::new(2.0, 0.0), "no drag");
        let mut at_rate = warping(3.0, 100);
        at_rate.warp.aim = 93;
        warp_block(&mut at_rate, Led::No, &world());
        assert_eq!(at_rate.warp.since, 100, "3 off is within its 3° turn");
    }

    #[test]
    fn a_ship_facing_its_aim_but_no_longer_jumping_ends_its_stage() {
        let mut ship = warping(3.0, 100);
        ship.goal = Goal::Idle;
        warp_block(&mut ship, Led::No, &world());
        assert_eq!(ship.warp.stage, 0.0);
        assert_eq!(ship.state.position, Vec2::new(2000.0, 0.0));
    }

    #[test]
    fn a_disabled_ship_or_one_not_under_way_has_no_warp_out() {
        let mut disabled = warping(3.0, 100);
        disabled.condition = Condition::Disabled;
        warp_block(&mut disabled, Led::No, &world());
        assert_eq!(disabled.state.velocity, Vec2::new(2.0, 0.0));
        let mut still = warping(0.0, 100);
        warp_block(&mut still, Led::No, &world());
        assert_eq!(still.state.velocity, Vec2::new(2.0, 0.0));
    }

    #[test]
    fn a_ship_the_player_leads_warps_out_on_the_players_clock_and_aim() {
        let mut world = world();
        world.player = LeaderWarp {
            stage: 3.0,
            since: 100,
            factor: warp_factor(0x0004),
            aim: 45,
            ..LeaderWarp::default()
        };
        let mut ship = warping(3.0, 0);
        ship.warp.goal = Some(WarpGoal::Follow(ShipRef::Player));
        warp_block(&mut ship, Led::ByPlayer, &world);
        assert_eq!(ship.warp.aim, 45, "the player's +0x68");
        let v = npc_warp_out_speed(WarpOutClock::Player, 200, warp_factor(0x0004), 364, false);
        assert!(v > 0.0);
        assert_eq!(ship.state.position, Vec2::new(2000.0, 0.0) + accel(90, v));
        // While the player's stage is at 1.0, v stays at none.
        world.player.stage = 1.0;
        let mut held = warping(3.0, 0);
        held.warp.goal = Some(WarpGoal::Follow(ShipRef::Player));
        warp_block(&mut held, Led::ByPlayer, &world);
        assert_eq!(held.state.position, Vec2::new(2000.0, 0.0));
        assert_eq!(held.state.velocity, Vec2::new(1.9, 0.0), "it still drags");
    }

    #[test]
    fn the_led_clock_runs_at_0_667_and_its_own_at_0_5_in_double_time() {
        let mut world = world();
        world.double_time = true;
        world.player = LeaderWarp {
            stage: 3.0,
            since: 100,
            factor: warp_factor(0),
            aim: 90,
            ..LeaderWarp::default()
        };
        let mut led = warping(3.0, 100);
        led.warp.goal = Some(WarpGoal::Follow(ShipRef::Player));
        warp_block(&mut led, Led::ByPlayer, &world);
        let player_v = npc_warp_out_speed(WarpOutClock::Player, 200, warp_factor(0), 252, true);
        assert_eq!(
            led.state.position,
            Vec2::new(2000.0, 0.0) + accel(90, player_v)
        );
        let mut own = warping(3.0, 100);
        warp_block(&mut own, Led::No, &world);
        let own_v = npc_warp_out_speed(WarpOutClock::Own, 200, warp_factor(0), 252, true);
        assert_eq!(
            own.state.position,
            Vec2::new(2000.0, 0.0) + accel(90, own_v)
        );
        assert!(
            player_v > own_v && player_v < 50.0,
            "0.667 less 45/w against 0.5 less 35/w"
        );
    }

    #[test]
    fn a_ship_an_npc_leads_takes_its_leaders_aim_on_its_own_clock() {
        let mut ship = warping(3.0, 100);
        warp_block(&mut ship, Led::ByNpc(270), &world());
        assert_eq!(ship.warp.aim, 270);
        let v = npc_warp_out_speed(WarpOutClock::Own, 200, warp_factor(0), 364, false);
        assert_eq!(ship.state.position, Vec2::new(2000.0, 0.0) + accel(90, v));
    }

    // The flight.

    #[test]
    fn a_jumping_ship_turns_to_its_aim_and_moves_on_without_thrust_or_fire() {
        let mut ship = warping(3.0, 0);
        ship.warp.aim = 180;
        ship.trigger.primary = true;
        let handling = ship.stats.handling;
        fly(
            &mut ship,
            &handling,
            FlightRules::default(),
            Inertia::Inertial,
        );
        assert_eq!(ship.state.heading, 93.0, "a turn toward 180");
        assert_eq!(ship.state.velocity, Vec2::new(2.0, 0.0), "no thrust");
        assert_eq!(ship.state.position, Vec2::new(2002.0, 0.0));
        assert_eq!(ship.trigger, crate::combat::armament::Trigger::default());
    }
}
