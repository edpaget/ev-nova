//! An NPC's drawn state, as the `EV Nova` executable keeps it: its bank
//! and engine glow as `_HandleShip` steps them, the writers of its fold in
//! `_AIDispatch` and `_HighLevelAIHandler`, what its spawn sets, and the
//! frames `_HandleShipDisplay` draws it on (see
//! [`ship_frames`](crate::combat::ship_frames)).
//!
//! **Its drive** ([`Drive`], [`record`]). Each flight step records which
//! way the ship turned and whether it thrust, at twice its own
//! acceleration or more (a rush's 2.75) or less, as `_HandleShip` reads
//! its turn (@0x3444b-0x34586: 1 turning clockwise, -1 the other way, 0
//! not turning or snapping onto its heading) and its thrust word +0x30
//! against `_ShipAccelRate`. A tick it is not flown by a step (an AI wait,
//! its glide in, keeping formation) reads as coasting straight.
//!
//! **The bank and glow** ([`bank_and_glow`], `_HandleShip`
//! @0x34d93-0x34fcb, after its flight):
//!
//! - `shän` `Flags` 0x0001: the bank (+0xc8e4) moves a frame toward ±8
//!   turning, and a frame back toward 0 not turning, 0 within a frame of it
//!   (8.0 @0xdd044, -8.0 @0xdd68c); the flag (+0xc8f8) is 1 past 4, -1
//!   below -4, else 0 (@0xdd680, @0xdd690). A banked ship of 0x0002 too
//!   raises its glow by 2 while it is 23 or less (@0x34eb3-0x34ee5).
//! - Thrusting, a ship of 0x0001 sets its flag to its turn itself
//!   (@0x34f3d-0x34f4a); then a rush raises the glow (+0xc8d4) by 1 while
//!   it is 31 or less, and any other thrust by 1 while 23 or less and lowers
//!   it by 1 from 25. Coasting lowers it by 1 while above 0.
//! - A ship whose AI wait (+0x4c) runs, not a count wait and not an escape
//!   pod, skips all of that (@0x34a57-0x34a72), its glow falling by 1 while
//!   above 0 (@0x3501f-0x35079); its bank stays.
//! - A ship of a derelict government (`_ShipIsDerelict` @0x8225) holds its
//!   glow at 0 every frame (@0x35449-0x3545b).
//!
//! Three copies come before: sub-goal 0xc, close formation, takes its
//! leader's glow (@0x87dce-0x87deb, [`Drive::formation`], the player's
//! for the player's escorts); sub-goal 0xd, following a leader's jump, the
//! leader's while the leader's stage is 1.0 or below (@0x85a2d-0x85a4a,
//! [`warp_out::follow_jump`](crate::traffic::warp_out::follow_jump)); and
//! the warp-out raises it by 3 to 32 as it moves the ship
//! (@0x349fc-0x34a25, [`warp_glow`]). The swarm leader's copies in
//! sub-goals 5, 0x11, 7 and 0xb (@0x85cb7, @0x86632, @0x86810,
//! @0x87394) are rdm:phase/combat-ai-fidelity/phase-14-swarming's.
//!
//! **The fold's writers** ([`dispatch_fold`], each advance pass, and
//! [`landing_unfold`]), each for a ship of 0x0002 with no direction
//! (+0xc8fa):
//!
//! - Gliding in (+0x50 below -900, @0x8fcf5-0x8fd7e), short of its last
//!   set, it folds (+1); a ship of 0x0080 only once its refold is due
//!   ([`REFOLD_TICKS`] after its last shot).
//! - Otherwise, on its own jump or following one (sub-goal 4 or 0xd), above
//!   set 0 and not of 0x0080, it unfolds (-1, @0x901a3-0x901d9); not
//!   jumping, with no AI wait and not landing (goal 1), short of its last
//!   set, it folds as gliding in (@0x90060-0x900bd).
//! - Landing, near its stellar (`_HighLevelAIHandler` state 1, its box
//!   @0x8da0c-0x8da38; here within its landing radius), above set 0 it
//!   unfolds, whatever its direction.
//!
//! **Its spawn** ([`NpcDisplay::spawned`]): its glow at 32
//! (`_RandomShipSpawn` @0x3c664, `_GenericRandomShipSpawn` @0x3cb77), 0
//! for a derelict government's ship (@0x3c766, @0x3cc4d) or a person's
//! (`_SpawnPerson` @0x410d3); its fold at its last set with no direction,
//! never fired (@0x3c4d0, @0x3cab8, @0x3d018, @0x41101). Every spawn but a
//! `düde`'s own ship (`_RandomShipSpawn`) rolls its anim set `Rand(BaseSets)`
//! and its animation timer `Rand(AnimDelay)` (@0x3cc73-0x3ccc2); one
//! jumping in (`_AIMakeShipJumpIn` @0x82ed5-0x82f04) takes set 0 folding
//! up, its timer `-Rand(10)`. The rolls are made as its display first
//! runs, before it is drawn, which is all that reads them.
//!
//! **The display** ([`display`]): each frame after the flight and the
//! fight, [`ShipFrames::step`] on its class's [`FrameSpec`], its fold and
//! whether it is disabled and its key ship aboard. No sound plays in any
//! of it.

use crate::catalog::GovtId;
use crate::chance::Chance;
use crate::combat::armament::Fold;
use crate::combat::ship_frames::{
    BANKS, Drawn, FOLDS, FrameSpec, Moment, REFOLD_TICKS, SPEED_MULT, ShipFrames, UNFOLDS_TO_FIRE,
};
use crate::flight::{Controls, FlightRules, Turn, heading_of, shortest_turn};
use crate::handling::Handling;
use crate::rulebook::RuleSource;
use crate::traffic::Traffic;
use crate::traffic::inertialess::Inertia;
use crate::traffic::npc::{Mode, Npc};
use crate::traffic::spawn::{Entry, NewShip};

/// The most an NPC banks either way (8.0 @0xdd044).
pub const BANK_MOST: f32 = 8.0;
/// The bank past which its flag is set (4.0 @0xdd680).
pub const BANK_FLAG: f32 = 4.0;
/// A spawned ship's glow (`_GenericRandomShipSpawn` @0x3cb77).
pub const GLOW_SPAWN: i16 = 32;
/// The highest glow a plain thrust or the banking glow raises (`cmpw
/// $0x17` @0x34f98, @0x34edc).
pub const GLOW_RISES_TO: i16 = 23;
/// The lowest glow a plain thrust lowers (`cmpw $0x18` @0x34fa1): above
/// cruise, 24.
pub const GLOW_CRUISE: i16 = 24;
/// The highest glow a rush raises (`cmpw $0x1f` @0x34f85).
pub const GLOW_RUSH_RISES_TO: i16 = 31;
/// How far the banking glow raises it.
pub const BANK_GLOW: i16 = 2;
/// How many times its own acceleration a thrust must be to be a rush.
pub const RUSH_RATIO: f32 = 2.0;
/// How far the warp-out raises the glow (@0x34a0c).
pub const WARP_GLOW: i16 = 3;
/// The most the warp-out raises the glow to (@0x34a0f).
pub const WARP_GLOW_TOP: i16 = 32;
/// The spread of a ship jumping in's fold timer, `-Rand(10)` (@0x82ef0).
pub const JUMP_IN_SPREAD: u16 = 10;

/// How a ship thrust on its last step.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Thrust {
    /// It did not.
    #[default]
    None,
    /// At less than [`RUSH_RATIO`] times its own acceleration.
    Plain,
    /// At [`RUSH_RATIO`] times it or more.
    Rush,
}

/// How a ship was flown this tick, which its bank and glow read (see the
/// module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Drive {
    /// Which way it turned: 1 clockwise, -1 the other way, 0 not at all.
    pub turn: i8,
    /// How it thrust.
    pub thrust: Thrust,
    /// Whether it kept close formation (sub-goal 0xc), taking its leader's
    /// glow.
    pub formation: bool,
}

/// What a ship's spawn owes its display, made as it first runs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rolls {
    /// Its anim set `Rand(BaseSets)` and animation timer `Rand(AnimDelay)`.
    pub anim: bool,
    /// Its fold timer `-Rand(10)`, jumping in.
    pub jump_in: bool,
    /// Its glow of 0 if its government is derelict, which the traffic's
    /// spawns cannot read.
    pub derelict_glow: bool,
}

/// An NPC's drawn state (see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NpcDisplay {
    /// Its display state, but for its fold, which is its armament's.
    pub frames: ShipFrames,
    /// Its engine glow's base, +0xc8d4.
    pub glow: i16,
    /// The sets its last display drew it on.
    pub drawn: Drawn,
    /// How it was flown this tick.
    pub drive: Drive,
    /// The rolls its spawn still owes.
    pub rolls: Rolls,
}

/// What a ship's spawn reads (see [`NpcDisplay::spawned`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Spawn {
    /// Whether a person flies it.
    pub person: bool,
    /// Whether it is a `düde`'s own ship, which rolls no animation.
    pub dude_ship: bool,
    /// Whether it jumps in.
    pub jumping_in: bool,
}

impl Spawn {
    /// What `ship`'s spawn reads: whether a person flies it, whether it is
    /// a `düde`'s own ship (its `düde`'s `Booty` carried, of no mission, no
    /// aux slot and no person, `_RandomShipSpawn` @0x3c1d7), and whether
    /// it jumps in.
    #[must_use]
    pub fn of(ship: &NewShip) -> Self {
        Self {
            person: ship.person.is_some(),
            dude_ship: ship.booty.is_some()
                && ship.mission.is_none()
                && ship.aux.is_none()
                && ship.person.is_none(),
            jumping_in: ship.entry == Entry::JumpingIn,
        }
    }
}

impl NpcDisplay {
    /// The drawn state and fold a ship of class `spec` spawns with as
    /// `spawn` says (see the module docs); a derelict government's ship's
    /// glow of 0 is set as its display first runs ([`display`]).
    #[must_use]
    pub fn spawned(spec: &FrameSpec, spawn: Spawn) -> (Self, Fold) {
        let glow = if spawn.person { 0 } else { GLOW_SPAWN };
        let fold = if spawn.jumping_in {
            Fold {
                dir: 1,
                ..Fold::default()
            }
        } else {
            Fold::default().at(spec.last_set())
        };
        let display = Self {
            glow,
            rolls: Rolls {
                anim: !spawn.dude_ship,
                jump_in: spawn.jumping_in,
                derelict_glow: true,
            },
            ..Self::default()
        };
        (display, fold)
    }
}

/// Records how `npc` is flown this step under `controls` on `handling`,
/// its own acceleration its handling's under `flight`, as `inertia` says
/// (see the module docs).
pub fn record(
    npc: &mut Npc,
    handling: &Handling,
    controls: Controls,
    flight: FlightRules,
    inertia: Inertia,
) {
    let turn = match controls.turn {
        Turn::Right => 1,
        Turn::Left => -1,
        Turn::None
            if inertia == Inertia::Inertial
                && controls.reverse
                && npc.state.velocity.length() > flight.at_rest_speed() =>
        {
            let off = shortest_turn(npc.state.heading, heading_of(npc.state.velocity * -1.0));
            if off.abs() <= handling.turn_rate {
                0
            } else if off > 0.0 {
                1
            } else {
                -1
            }
        }
        Turn::None => 0,
    };
    let own = flight.handling(npc.stats.handling).accel;
    let thrust = if !controls.thrust {
        Thrust::None
    } else if own > 0.0 && handling.accel >= RUSH_RATIO * own {
        Thrust::Rush
    } else {
        Thrust::Plain
    };
    npc.display.drive.turn = turn;
    npc.display.drive.thrust = thrust;
}

/// Whether `npc`'s AI wait holds it from its bank and glow (see the module
/// docs): a wait running that is not a count wait, of any ship but an
/// escape pod. Read as its tick begins, before its flight counts it down.
#[must_use]
pub fn held(npc: &Npc) -> bool {
    npc.wait_running() && !npc.count_wait() && !matches!(npc.mode, Mode::Adrift { .. })
}

/// `_HandleShip`'s bank and glow for `npc` after its flight, `held` by its
/// AI wait as the tick began ([`held`]), its government derelict when
/// `derelict`, its close-formation leader's glow `leader_glow` (see the
/// module docs). Its drive is spent.
pub fn bank_and_glow(npc: &mut Npc, held: bool, derelict: bool, leader_glow: Option<i16>) {
    let spec = npc.hull.frames;
    let drive = std::mem::take(&mut npc.display.drive);
    let display = &mut npc.display;
    if drive.formation
        && let Some(glow) = leader_glow
    {
        display.glow = glow;
    }
    if held {
        fade(&mut display.glow);
    } else {
        if spec.has(BANKS) {
            bank(&mut display.frames, drive.turn);
            if display.frames.flag != 0 && spec.has(FOLDS) && display.glow <= GLOW_RISES_TO {
                display.glow += BANK_GLOW;
            }
        }
        match drive.thrust {
            Thrust::None => fade(&mut display.glow),
            thrust => {
                if spec.has(BANKS) {
                    display.frames.flag = drive.turn;
                }
                let glow = &mut display.glow;
                if thrust == Thrust::Rush {
                    if *glow <= GLOW_RUSH_RISES_TO {
                        *glow += 1;
                    }
                } else if *glow <= GLOW_RISES_TO {
                    *glow += 1;
                } else if *glow > GLOW_CRUISE {
                    *glow -= 1;
                }
            }
        }
    }
    if derelict {
        display.glow = 0;
    }
}

/// `frames`' bank and flag a frame on, turning `turn` (see the module
/// docs).
fn bank(frames: &mut ShipFrames, turn: i8) {
    let bank = &mut frames.bank;
    match turn {
        1.. => {
            if *bank < BANK_MOST {
                *bank += SPEED_MULT;
            }
        }
        ..0 => {
            if *bank > -BANK_MOST {
                *bank -= SPEED_MULT;
            }
        }
        0 => {
            if *bank >= SPEED_MULT {
                *bank -= SPEED_MULT;
            } else if *bank <= -SPEED_MULT {
                *bank += SPEED_MULT;
            } else {
                *bank = 0.0;
            }
        }
    }
    frames.flag = if *bank > BANK_FLAG {
        1
    } else if *bank < -BANK_FLAG {
        -1
    } else {
        0
    };
}

/// `glow` 1 lower while above 0.
fn fade(glow: &mut i16) {
    if *glow > 0 {
        *glow -= 1;
    }
}

/// The warp-out's glow as it moves `npc` (@0x349fc-0x34a25): 3 higher
/// while 31 or less, at most 32.
pub fn warp_glow(npc: &mut Npc) {
    let glow = &mut npc.display.glow;
    if *glow < WARP_GLOW_TOP {
        *glow = (*glow + WARP_GLOW).min(WARP_GLOW_TOP);
    }
}

/// `_AIDispatch`'s writers of `npc`'s fold, each advance pass (see the
/// module docs).
pub fn dispatch_fold(npc: &mut Npc) {
    let spec = npc.hull.frames;
    let waiting = npc.wait_running();
    let landing = matches!(npc.goal, crate::ai::Goal::Land(_));
    let jumping = npc.warp.goal.is_some();
    let gliding = matches!(npc.mode, Mode::JumpingIn { .. });
    let fold = npc.armament.fold_mut();
    if !spec.has(FOLDS) || fold.dir != 0 {
        return;
    }
    let short = fold.set < spec.last_set();
    let due = !spec.has(UNFOLDS_TO_FIRE) || fold.since_fire >= REFOLD_TICKS;
    if gliding || !(jumping || waiting || landing) {
        if short && due {
            fold.dir = 1;
        }
    } else if jumping && fold.set > 0 && !spec.has(UNFOLDS_TO_FIRE) {
        fold.dir = -1;
    }
}

/// `_HighLevelAIHandler`'s unfold of a landing `npc` near its stellar
/// (@0x8da0c-0x8da38): above set 0, a ship of 0x0002 unfolds.
pub fn landing_unfold(npc: &mut Npc) {
    let folds = npc.hull.frames.has(FOLDS);
    let fold = npc.armament.fold_mut();
    if folds && fold.set > 0 {
        fold.dir = -1;
    }
}

/// What `npc`'s spawn owes it, made before anything reads it, as its
/// first flight or display begins (see the module docs): its glow of 0
/// when its government is `derelict`, and its rolls on `chance`. A roll no
/// reader of its class reads, or of one outcome, is not made: the anim
/// set's but on the 0x0008 path, the animation timer's but there or with
/// an alt layer, and a jump in's but for a ship that banks or folds.
pub fn settle_spawn(npc: &mut Npc, derelict: bool, chance: &mut (impl Chance + ?Sized)) {
    let spec = npc.hull.frames;
    let rolls = std::mem::take(&mut npc.display.rolls);
    if rolls.derelict_glow && derelict {
        npc.display.glow = 0;
    }
    let mut roll = |sides: u16| if sides > 1 { chance.roll(sides) } else { 0 };
    let frames = &mut npc.display.frames;
    let animated = spec.anim_path();
    if rolls.anim && animated {
        frames.anim_set = i16::try_from(roll(spec.base_sets)).unwrap_or(0);
    }
    if rolls.anim && (animated || spec.alt_sets > 0) {
        frames.anim_timer = f32::from(roll(u16::try_from(spec.anim_delay).unwrap_or(0)));
    }
    if rolls.jump_in && (spec.has(BANKS) || spec.has(FOLDS)) {
        frames.bank = -f32::from(roll(JUMP_IN_SPREAD));
    }
}

/// `_HandleShipDisplay`'s frame for `npc`, its alt layer stepping as
/// `double_step` says, what its spawn owes made first: its glow of 0 when
/// its government is `derelict`, and its rolls on `chance` (see the module
/// docs).
pub fn display(
    npc: &mut Npc,
    derelict: bool,
    double_step: RuleSource,
    chance: &mut (impl Chance + ?Sized),
) {
    settle_spawn(npc, derelict, chance);
    let spec = npc.hull.frames;
    let moment = Moment {
        disabled: npc.condition.disabled(),
        key_aboard: npc
            .armament
            .key_ship_aboard(npc.hull.key_carried, &npc.rounds),
        double_step,
    };
    let fold = npc.armament.fold_mut();
    npc.display.drawn = npc.display.frames.step(&spec, fold, moment);
}

impl Traffic {
    /// Every NPC's display this frame ([`display`]), after the flight and
    /// the fight: its alt layer stepping as `double_step` says, a ship of
    /// a government `derelict` says is derelict glowing 0 from its spawn,
    /// its spawn's rolls made on `chance`.
    pub fn display_ships(
        &mut self,
        derelict: impl Fn(Option<GovtId>) -> bool,
        double_step: RuleSource,
        chance: &mut (impl Chance + ?Sized),
    ) {
        for npc in &mut self.npcs {
            let derelict = derelict(npc.govt);
            display(npc, derelict, double_step, &mut *chance);
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::{ShipId, StellarId};
    use crate::combat::armament::NEVER_FIRED;
    use crate::combat::hull::Condition;
    use crate::combat::ship_frames::{ANIMATED, KEY_CARRIED};
    use crate::flight::ShipState;
    use crate::geometry::Vec2;
    use crate::stats::ShipStats;
    use crate::traffic::fixture::npc;
    use crate::traffic::warp_out::WarpGoal;
    use nova_sim_base::fixture::Draws;

    /// 3° a tick, 0.5 acceleration.
    fn stats() -> ShipStats {
        ShipStats {
            handling: Handling {
                turn_rate: 3.0,
                accel: 0.5,
                max_speed: 5.0,
            },
            ..ShipStats::default()
        }
    }

    /// An NPC of `shän` `Flags` `flags` and `sets` base sets.
    fn folding(flags: u16, sets: u16) -> Npc {
        let mut ship = npc(1, stats());
        ship.hull.frames = FrameSpec {
            flags,
            base_sets: sets,
            anim_delay: 5,
            ..FrameSpec::NONE
        };
        ship
    }

    fn flown(ship: &mut Npc, turn: i8, thrust: Thrust) {
        ship.display.drive = Drive {
            turn,
            thrust,
            formation: false,
        };
        bank_and_glow(ship, false, false, None);
    }

    #[test]
    fn a_banking_ship_banks_a_frame_a_tick_to_8_and_flags_past_4() {
        let mut ship = folding(BANKS, 3);
        let mut seen = Vec::new();
        for _ in 0..10 {
            flown(&mut ship, 1, Thrust::None);
            seen.push((ship.display.frames.bank, ship.display.frames.flag));
        }
        assert_eq!(
            seen,
            [
                (1.0, 0),
                (2.0, 0),
                (3.0, 0),
                (4.0, 0),
                (5.0, 1),
                (6.0, 1),
                (7.0, 1),
                (8.0, 1),
                (8.0, 1),
                (8.0, 1)
            ]
        );
        for _ in 0..4 {
            flown(&mut ship, 0, Thrust::None);
        }
        assert_eq!(
            (ship.display.frames.bank, ship.display.frames.flag),
            (4.0, 0)
        );
        for _ in 0..13 {
            flown(&mut ship, -1, Thrust::None);
        }
        assert_eq!(
            (ship.display.frames.bank, ship.display.frames.flag),
            (-8.0, -1)
        );
        ship.display.frames.bank = 0.5;
        flown(&mut ship, 0, Thrust::None);
        assert_eq!(ship.display.frames.bank, 0.0, "within a frame: 0");
        ship.display.frames.bank = -0.5;
        flown(&mut ship, 0, Thrust::None);
        assert_eq!(ship.display.frames.bank, 0.0);
        ship.display.frames.bank = -3.0;
        flown(&mut ship, 0, Thrust::None);
        assert_eq!(ship.display.frames.bank, -2.0, "back up a frame");
        ship.display.frames.bank = -5.0;
        flown(&mut ship, 0, Thrust::None);
        let shown = (ship.display.frames.bank, ship.display.frames.flag);
        assert_eq!(shown, (-4.0, 0), "-4: level");
    }

    #[test]
    fn a_ship_that_does_not_bank_keeps_no_bank() {
        let mut ship = folding(0, 3);
        flown(&mut ship, 1, Thrust::Plain);
        assert_eq!(ship.display.frames, ShipFrames::default());
    }

    #[test]
    fn thrusting_a_banking_ship_flags_its_turn_itself() {
        let mut ship = folding(BANKS, 3);
        flown(&mut ship, 1, Thrust::Plain);
        assert_eq!(
            (ship.display.frames.bank, ship.display.frames.flag),
            (1.0, 1)
        );
        ship.display.frames.bank = 7.0;
        flown(&mut ship, 0, Thrust::Plain);
        assert_eq!(ship.display.frames.flag, 0, "banked, thrusting straight");
        flown(&mut ship, -1, Thrust::Rush);
        assert_eq!(ship.display.frames.flag, -1);
    }

    #[test]
    fn a_wait_holds_the_bank_and_lowers_the_glow() {
        let mut ship = folding(BANKS, 3);
        ship.display.frames.bank = 6.0;
        ship.display.frames.flag = 1;
        ship.display.glow = 10;
        ship.display.drive = Drive {
            turn: -1,
            thrust: Thrust::Rush,
            formation: false,
        };
        bank_and_glow(&mut ship, true, false, None);
        assert_eq!(
            (ship.display.frames.bank, ship.display.frames.flag),
            (6.0, 1)
        );
        assert_eq!(ship.display.glow, 9);
        assert_eq!(ship.display.drive, Drive::default(), "spent");
        ship.display.glow = 0;
        bank_and_glow(&mut ship, true, false, None);
        assert_eq!(ship.display.glow, 0);
    }

    #[test]
    fn the_wait_that_holds_it_is_not_a_count_wait_nor_a_pods() {
        let mut ship = folding(0, 1);
        assert!(!held(&ship));
        ship.ai_wait = 5;
        assert!(held(&ship));
        ship.ai_wait = 0;
        ship.goal = crate::ai::Goal::CountWait { ticks_left: 9 };
        assert!(!held(&ship), "a count wait thrusts on");
        ship.goal = crate::ai::Goal::Idle;
        ship.mode = Mode::Adrift { ticks_left: 50 };
        assert!(!held(&ship), "an escape pod");
        ship.mode = Mode::Coasting { ticks_left: 3 };
        assert!(held(&ship));
    }

    #[test]
    fn thrust_raises_the_glow_to_24_and_a_rush_to_32() {
        let mut ship = folding(0, 1);
        for _ in 0..30 {
            flown(&mut ship, 0, Thrust::Plain);
        }
        assert_eq!(ship.display.glow, 24);
        for _ in 0..30 {
            flown(&mut ship, 0, Thrust::Rush);
        }
        assert_eq!(ship.display.glow, 32);
        flown(&mut ship, 0, Thrust::Plain);
        assert_eq!(
            ship.display.glow, 31,
            "a plain thrust above cruise lowers it"
        );
        ship.display.glow = 25;
        flown(&mut ship, 0, Thrust::Plain);
        assert_eq!(ship.display.glow, 24);
        flown(&mut ship, 0, Thrust::Plain);
        assert_eq!(ship.display.glow, 24, "held at cruise");
        for _ in 0..30 {
            flown(&mut ship, 0, Thrust::None);
        }
        assert_eq!(ship.display.glow, 0, "coasting brings it to 0");
    }

    #[test]
    fn a_banked_folding_ship_raises_its_glow_by_2() {
        let mut ship = folding(BANKS | FOLDS, 3);
        ship.display.frames.bank = 7.0;
        ship.display.glow = 20;
        flown(&mut ship, 1, Thrust::None);
        assert_eq!(ship.display.glow, 21, "+2, then coasting -1");
        ship.display.glow = 24;
        flown(&mut ship, 1, Thrust::None);
        assert_eq!(ship.display.glow, 23, "above 23 no raise");
        let mut level = ship_of_glow(BANKS | FOLDS, 20);
        flown(&mut level, 1, Thrust::None);
        assert_eq!(level.display.glow, 19, "level: no raise");
        let mut plain = ship_of_glow(BANKS, 20);
        plain.display.frames.bank = 7.0;
        flown(&mut plain, 1, Thrust::None);
        assert_eq!(plain.display.glow, 19, "no 0x0002");
    }

    fn ship_of_glow(flags: u16, glow: i16) -> Npc {
        let mut ship = folding(flags, 3);
        ship.display.glow = glow;
        ship
    }

    #[test]
    fn a_derelict_governments_ship_holds_its_glow_at_0() {
        let mut ship = ship_of_glow(0, 20);
        ship.display.drive.thrust = Thrust::Rush;
        bank_and_glow(&mut ship, false, true, None);
        assert_eq!(ship.display.glow, 0);
    }

    #[test]
    fn a_ship_in_close_formation_takes_its_leaders_glow_before_its_own_step() {
        let mut ship = ship_of_glow(0, 3);
        ship.display.drive.formation = true;
        bank_and_glow(&mut ship, false, false, Some(20));
        assert_eq!(ship.display.glow, 19, "the leader's, then coasting");
        ship.display.glow = 3;
        bank_and_glow(&mut ship, false, false, Some(20));
        assert_eq!(ship.display.glow, 2, "out of formation: its own");
    }

    #[test]
    fn the_warp_out_raises_the_glow_by_3_to_32() {
        let mut ship = ship_of_glow(0, 28);
        warp_glow(&mut ship);
        assert_eq!(ship.display.glow, 31);
        warp_glow(&mut ship);
        assert_eq!(ship.display.glow, 32);
        warp_glow(&mut ship);
        assert_eq!(ship.display.glow, 32);
        ship.display.glow = 40;
        warp_glow(&mut ship);
        assert_eq!(ship.display.glow, 40, "above 31 left alone");
    }

    #[test]
    fn a_step_records_its_turn_and_thrust() {
        let flight = FlightRules::default();
        let handling = flight.handling(stats().handling);
        let mut ship = folding(0, 1);
        let controls = |turn, thrust| Controls {
            turn,
            thrust,
            ..Controls::default()
        };
        record(
            &mut ship,
            &handling,
            controls(Turn::Right, true),
            flight,
            Inertia::Inertial,
        );
        assert_eq!(
            ship.display.drive,
            Drive {
                turn: 1,
                thrust: Thrust::Plain,
                formation: false
            }
        );
        let rushing = Handling {
            accel: handling.accel * 2.75,
            ..handling
        };
        record(
            &mut ship,
            &rushing,
            controls(Turn::Left, true),
            flight,
            Inertia::Inertial,
        );
        assert_eq!(
            (ship.display.drive.turn, ship.display.drive.thrust),
            (-1, Thrust::Rush)
        );
        let breaking = Handling {
            accel: handling.accel * 1.5,
            ..handling
        };
        record(
            &mut ship,
            &breaking,
            controls(Turn::None, true),
            flight,
            Inertia::Inertial,
        );
        assert_eq!(
            (ship.display.drive.turn, ship.display.drive.thrust),
            (0, Thrust::Plain)
        );
        record(
            &mut ship,
            &handling,
            controls(Turn::None, false),
            flight,
            Inertia::Inertial,
        );
        assert_eq!(ship.display.drive.thrust, Thrust::None);
        // A ship of no acceleration of its own never rushes.
        ship.stats.handling.accel = 0.0;
        let still = Handling {
            accel: 0.0,
            ..handling
        };
        let thrust = controls(Turn::None, true);
        record(&mut ship, &still, thrust, flight, Inertia::Inertial);
        assert_eq!(ship.display.drive.thrust, Thrust::Plain);
    }

    #[test]
    fn reversing_records_the_turn_toward_its_retro_heading_unless_it_snaps() {
        let flight = FlightRules::default();
        let handling = flight.handling(stats().handling);
        let mut ship = folding(0, 1);
        // Moving east, facing north: the retro heading, west, is to the
        // left.
        ship.state = ShipState {
            velocity: Vec2::new(2.0, 0.0),
            ..ShipState::default()
        };
        let reverse = Controls {
            reverse: true,
            ..Controls::default()
        };
        record(&mut ship, &handling, reverse, flight, Inertia::Inertial);
        assert_eq!(ship.display.drive.turn, -1);
        ship.state.heading = 268.0;
        record(&mut ship, &handling, reverse, flight, Inertia::Inertial);
        assert_eq!(ship.display.drive.turn, 0, "within a turn it snaps");
        ship.state.heading = 200.0;
        record(&mut ship, &handling, reverse, flight, Inertia::Inertial);
        assert_eq!(ship.display.drive.turn, 1);
        ship.state.velocity = Vec2::ZERO;
        record(&mut ship, &handling, reverse, flight, Inertia::Inertial);
        assert_eq!(ship.display.drive.turn, 0, "at rest: no turn");
        ship.state.velocity = Vec2::new(flight.at_rest_speed(), 0.0);
        record(&mut ship, &handling, reverse, flight, Inertia::Inertial);
        assert_eq!(ship.display.drive.turn, 0, "at the rest speed: no turn");
        ship.state.velocity = Vec2::new(2.0, 0.0);
        let inertialess = Inertia::Inertialess { speed: 2.0 };
        record(&mut ship, &handling, reverse, flight, inertialess);
        assert_eq!(
            ship.display.drive.turn, 0,
            "an inertialess ship's reverse turns it not"
        );
    }

    #[test]
    fn a_spawn_glows_at_32_but_a_persons_or_a_derelict_governments() {
        let spec = FrameSpec {
            flags: FOLDS,
            base_sets: 6,
            ..FrameSpec::NONE
        };
        let (spawned, fold) = NpcDisplay::spawned(&spec, Spawn::default());
        assert_eq!(spawned.glow, 32);
        assert_eq!(
            fold,
            Fold {
                set: 5,
                dir: 0,
                since_fire: NEVER_FIRED
            },
            "folded"
        );
        assert_eq!(
            spawned.rolls,
            Rolls {
                anim: true,
                jump_in: false,
                derelict_glow: true
            }
        );
        let person = Spawn {
            person: true,
            ..Spawn::default()
        };
        assert_eq!(NpcDisplay::spawned(&spec, person).0.glow, 0);
        let dude = Spawn {
            dude_ship: true,
            ..Spawn::default()
        };
        assert!(!NpcDisplay::spawned(&spec, dude).0.rolls.anim);
        // A derelict government's ship's glow is 0 by its first display.
        for (derelict, glow) in [(true, 0), (false, 32)] {
            let mut ship = folding(0, 1);
            ship.display = spawned;
            display(&mut ship, derelict, RuleSource::Engine, &mut Draws::of(&[]));
            assert_eq!(ship.display.glow, glow, "{derelict}");
            ship.display.glow = 9;
            display(&mut ship, derelict, RuleSource::Engine, &mut Draws::of(&[]));
            assert_eq!(ship.display.glow, 9, "{derelict}: once");
        }
        let jumping = Spawn {
            jumping_in: true,
            ..Spawn::default()
        };
        let (spawned, fold) = NpcDisplay::spawned(&spec, jumping);
        assert_eq!(
            (fold.set, fold.dir),
            (0, 1),
            "jumping in: set 0, folding up"
        );
        assert!(spawned.rolls.jump_in);
    }

    #[test]
    fn the_first_display_makes_the_spawns_rolls() {
        let mut ship = folding(ANIMATED, 4);
        ship.display.rolls = Rolls {
            anim: true,
            jump_in: true,
            derelict_glow: false,
        };
        let mut draws = Draws::of(&[2, 3]);
        display(&mut ship, false, RuleSource::Engine, &mut draws);
        assert_eq!(draws.asked, [4, 5], "Rand(BaseSets), Rand(AnimDelay)");
        assert_eq!(ship.display.frames.bank, 0.0, "no bank or fold to roll");
        assert_eq!(ship.display.frames.anim_timer, 4.0, "3, then a frame");
        assert_eq!(ship.display.drawn.set, 2);
        assert_eq!(ship.display.rolls, Rolls::default());
        display(&mut ship, false, RuleSource::Engine, &mut draws);
        assert_eq!(draws.asked.len(), 2, "once");
        let owed = Rolls {
            anim: true,
            jump_in: true,
            derelict_glow: false,
        };
        // A banking ship jumping in rolls its bank; a plain one nothing.
        let mut banking = folding(BANKS, 3);
        banking.display.rolls = owed;
        let mut draws = Draws::of(&[7]);
        settle_spawn(&mut banking, false, &mut draws);
        assert_eq!(draws.asked, [10], "Rand(10) alone");
        assert_eq!(banking.display.frames.bank, -7.0);
        let mut plain = folding(0, 1);
        plain.display.rolls = owed;
        let mut draws = Draws::of(&[]);
        settle_spawn(&mut plain, false, &mut draws);
        assert_eq!(
            draws.asked,
            Vec::<u32>::new(),
            "nothing read: nothing drawn"
        );
        // An alt layer reads the timer alone.
        let mut alt = folding(0, 1);
        alt.hull.frames.alt_sets = 6;
        alt.display.rolls = owed;
        let mut draws = Draws::of(&[4]);
        settle_spawn(&mut alt, false, &mut draws);
        assert_eq!(draws.asked, [5]);
        assert_eq!(alt.display.frames.anim_timer, 4.0);
        // A delay of 1 or less has one outcome.
        let mut quick = folding(ANIMATED, 4);
        quick.hull.frames.anim_delay = 1;
        quick.display.rolls = owed;
        let mut draws = Draws::of(&[3]);
        settle_spawn(&mut quick, false, &mut draws);
        assert_eq!(draws.asked, [4], "Rand(BaseSets) alone");
    }

    #[test]
    fn the_display_reads_its_condition_and_key_ship() {
        let mut ship = folding(KEY_CARRIED, 2);
        ship.hull.key_carried = Some(ShipId(140));
        display(&mut ship, false, RuleSource::Engine, &mut Draws::of(&[]));
        assert_eq!(ship.display.drawn.set, 0, "no bay aboard");
        let mut still = folding(ANIMATED | 0x0010, 3);
        still.condition = Condition::Disabled;
        display(&mut still, false, RuleSource::Engine, &mut Draws::of(&[]));
        assert_eq!(still.display.frames.anim_timer, 0.0, "disabled, still");
    }

    #[test]
    fn a_folding_ship_folds_flying_unfolds_jumping_and_landing_near() {
        let mut ship = folding(FOLDS, 6);
        dispatch_fold(&mut ship);
        assert_eq!(ship.armament.fold().dir, 1, "flying: fold");
        ship.armament.fold_mut().dir = 0;
        ship.armament.fold_mut().set = 5;
        dispatch_fold(&mut ship);
        assert_eq!(ship.armament.fold().dir, 0, "folded already");
        ship.warp.goal = Some(WarpGoal::Own);
        dispatch_fold(&mut ship);
        assert_eq!(ship.armament.fold().dir, -1, "jumping: unfold");
        ship.armament.fold_mut().dir = 0;
        ship.armament.fold_mut().set = 0;
        dispatch_fold(&mut ship);
        assert_eq!(ship.armament.fold().dir, 0, "jumping, unfolded");
        ship.warp.goal = None;
        ship.goal = crate::ai::Goal::Land(StellarId(128));
        dispatch_fold(&mut ship);
        assert_eq!(ship.armament.fold().dir, 0, "landing: no fold");
        landing_unfold(&mut ship);
        assert_eq!(ship.armament.fold().dir, 0, "near, unfolded already");
        ship.armament.fold_mut().set = 3;
        landing_unfold(&mut ship);
        assert_eq!(ship.armament.fold().dir, -1, "near: unfold");
        ship.goal = crate::ai::Goal::Idle;
        ship.armament.fold_mut().dir = 0;
        ship.ai_wait = 5;
        dispatch_fold(&mut ship);
        assert_eq!(ship.armament.fold().dir, 0, "waiting: no fold");
        ship.ai_wait = 0;
        ship.armament.fold_mut().dir = -1;
        dispatch_fold(&mut ship);
        assert_eq!(ship.armament.fold().dir, -1, "a direction kept");
    }

    #[test]
    fn a_ship_gliding_in_folds_and_a_miner_only_once_its_refold_is_due() {
        let mut ship = folding(FOLDS | UNFOLDS_TO_FIRE, 6);
        ship.mode = Mode::JumpingIn { tick: 0 };
        ship.armament.fold_mut().since_fire = REFOLD_TICKS - 1;
        dispatch_fold(&mut ship);
        assert_eq!(ship.armament.fold().dir, 0, "not yet due");
        ship.armament.fold_mut().since_fire = REFOLD_TICKS;
        dispatch_fold(&mut ship);
        assert_eq!(ship.armament.fold().dir, 1);
        // A miner never unfolds to jump.
        let mut miner = folding(FOLDS | UNFOLDS_TO_FIRE, 6);
        miner.armament.fold_mut().set = 5;
        miner.warp.goal = Some(WarpGoal::Own);
        dispatch_fold(&mut miner);
        assert_eq!(miner.armament.fold().dir, 0);
        // A ship that does not fold has none of it.
        let mut plain = folding(0, 6);
        dispatch_fold(&mut plain);
        landing_unfold(&mut plain);
        plain.armament.fold_mut().set = 3;
        landing_unfold(&mut plain);
        assert_eq!(plain.armament.fold().dir, 0);
    }
}
