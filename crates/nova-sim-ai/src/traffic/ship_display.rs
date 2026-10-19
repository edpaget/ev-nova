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
//! never fired (@0x3c4d0, @0x3cab8, @0x3d018, @0x41101). Its anim set
//! `Rand(BaseSets)` and animation timer `Rand(AnimDelay)` are rolled by
//! its origin ([`AnimRoll`]): a `düde`'s own ship (`_RandomShipSpawn`
//! @0x3c781-0x3c7d6), a mission's (`_MissionDudeSpawn` @0x3d03c-0x3d091)
//! and a fighter (`_LaunchFighter` @0x3da72-0x3dad9) on its own class's;
//! every other `_GenericRandomShipSpawn` caller (a fleet, a person, an
//! aux ship, the nag ship, a capture's ship, a hired escort) on the first
//! `shïp`'s, ID 128's, as it writes the class +0x76 = 0 before its rolls
//! (@0x3c929, @0x3cc56-0x3ccc2), by the engine's reading of
//! [`RuleKey::SpawnAnimRollsFirstClass`](crate::RuleKey::SpawnAnimRollsFirstClass)
//! ([`FirstClass`]), on its own by the other; and the eject's abandoned
//! hull rolls none (`_HandlePlayer` @0x6f3a3, @0x6f3bb). Each roll is made
//! only when it has more than one outcome and its own class reads it. One
//! jumping in (`_AIMakeShipJumpIn` @0x82ed5-0x82f04) takes set 0 folding
//! up, its timer `-Rand(10)`. The rolls are made as its first flight or
//! display begins, before it is drawn, which is all that reads them.
//!
//! **The display** ([`display`]): each frame after the flight and the
//! fight, [`ShipFrames::step`] on its class's [`FrameSpec`], its fold and
//! whether it is disabled and its key ship aboard; then its ion tint
//! ([`ion_tint::display`]), on its class's capacity, its colour cleared
//! once its charge is gone. No sound plays in any of it.

use crate::catalog::GovtId;
use crate::chance::Chance;
use crate::combat::armament::Fold;
use crate::combat::ion_tint::{self, IonTint};
use crate::combat::ship_frames::{
    BANKS, DisplayRules, Drawn, FOLDS, FrameSpec, Moment, REFOLD_TICKS, SPEED_MULT, ShipFrames,
    UNFOLDS_TO_FIRE,
};
use crate::flight::Flyer;
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

/// Which class a spawn rolls its anim set and animation timer on (see the
/// module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AnimRoll {
    /// None: the eject's abandoned hull, or rolled already.
    #[default]
    None,
    /// Its own class's: a `düde`'s own ship, a mission's, a fighter.
    Own,
    /// The first class's, ID 128's, by the engine's reading: every other
    /// `_GenericRandomShipSpawn` caller.
    FirstClass,
}

/// The first class, ID 128, whose `BaseSets` and `AnimDelay` an
/// [`AnimRoll::FirstClass`] spawn rolls on, and the reading of
/// [`RuleKey::SpawnAnimRollsFirstClass`](crate::RuleKey::SpawnAnimRollsFirstClass)
/// it rolls by.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FirstClass {
    /// Class 128's display; [`FrameSpec::NONE`] without one.
    pub spec: FrameSpec,
    /// On class 128's by the engine, on its own by the other reading.
    pub reading: RuleSource,
}

impl FirstClass {
    /// The class a spawn of class `own` rolls `anim` on, if it rolls.
    #[must_use]
    pub fn roll_on(&self, anim: AnimRoll, own: &FrameSpec) -> Option<FrameSpec> {
        match anim {
            AnimRoll::None => None,
            AnimRoll::Own => Some(*own),
            AnimRoll::FirstClass => Some(match self.reading {
                RuleSource::Engine => self.spec,
                RuleSource::Bible => *own,
            }),
        }
    }
}

/// What a ship's spawn owes its display, made as it first runs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rolls {
    /// Its anim set `Rand(BaseSets)` and animation timer `Rand(AnimDelay)`,
    /// on the class its origin says.
    pub anim: AnimRoll,
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
    /// Its ion tint this frame, if it glows
    /// ([`ion_tint`](crate::combat::ion_tint)).
    pub ion_tint: Option<IonTint>,
}

/// What a ship's spawn reads (see [`NpcDisplay::spawned`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Spawn {
    /// Whether a person flies it.
    pub person: bool,
    /// Which class it rolls its animation on.
    pub anim: AnimRoll,
    /// Whether it jumps in.
    pub jumping_in: bool,
}

impl Spawn {
    /// What `ship`'s spawn reads: whether a person flies it, its anim
    /// rolls by its origin (its own class's for a mission's ship or a
    /// `düde`'s own, one carrying its `düde`'s `Booty` of no mission, no
    /// aux slot and no person, `_RandomShipSpawn` @0x3c1d7; the first
    /// class's for every other), and whether it jumps in.
    #[must_use]
    pub fn of(ship: &NewShip) -> Self {
        let dude_ship = ship.booty.is_some()
            && ship.mission.is_none()
            && ship.aux.is_none()
            && ship.person.is_none();
        Self {
            person: ship.person.is_some(),
            anim: if dude_ship || ship.mission.is_some() {
                AnimRoll::Own
            } else {
                AnimRoll::FirstClass
            },
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
                anim: spawn.anim,
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
    // Its `_ShipAccelRate`, a sixth while another ship holds it in a
    // tractor beam and slowed by its ion charge (@0x34f57).
    let own = npc
        .ionization()
        .slows(flight.handling(npc.own_handling()))
        .accel;
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
/// when its government is `derelict`, and its rolls on `chance`, its anim
/// rolls on the class `first` says. A roll no reader of its class reads,
/// or of one outcome, is not made: the anim set's but on the 0x0008 path,
/// the animation timer's but there or with an alt layer, and a jump in's
/// but for a ship that banks or folds.
pub fn settle_spawn(
    npc: &mut Npc,
    derelict: bool,
    first: &FirstClass,
    chance: &mut (impl Chance + ?Sized),
) {
    let spec = npc.hull.frames;
    let rolls = std::mem::take(&mut npc.display.rolls);
    if rolls.derelict_glow && derelict {
        npc.display.glow = 0;
    }
    let mut roll = |sides: u16| if sides > 1 { chance.roll(sides) } else { 0 };
    let frames = &mut npc.display.frames;
    let animated = spec.anim_path();
    if let Some(on) = first.roll_on(rolls.anim, &spec) {
        if animated {
            frames.anim_set = i16::try_from(roll(on.base_sets)).unwrap_or(0);
        }
        if animated || spec.alt_sets > 0 {
            frames.anim_timer = f32::from(roll(u16::try_from(on.anim_delay).unwrap_or(0)));
        }
    }
    if rolls.jump_in && (spec.has(BANKS) || spec.has(FOLDS)) {
        frames.bank = -f32::from(roll(JUMP_IN_SPREAD));
    }
}

/// `_HandleShipDisplay`'s frame for `npc` by `rules`, what its spawn owes
/// made first: its glow of 0 when its government is `derelict`, and its
/// rolls on `chance`, its anim rolls on the class `first` says; then its sets and its lights, random lights rolling
/// on `chance` (see the module docs).
pub fn display(
    npc: &mut Npc,
    derelict: bool,
    rules: DisplayRules,
    first: &FirstClass,
    chance: &mut (impl Chance + ?Sized),
) {
    settle_spawn(npc, derelict, first, chance);
    let spec = npc.hull.frames;
    let moment = Moment {
        disabled: npc.condition.disabled(),
        key_aboard: npc
            .armament
            .key_ship_aboard(npc.hull.key_carried, &npc.rounds),
        rules,
    };
    let fold = npc.armament.fold_mut();
    let frames = &mut npc.display.frames;
    let mut drawn = frames.step(&spec, fold, moment);
    drawn.lights = frames.light(&spec, &moment, |sides| chance.roll(sides));
    npc.display.drawn = drawn;
    let capacity = npc.stats.ion.capacity(Flyer::Npc);
    npc.display.ion_tint = ion_tint::display(&mut npc.ion, capacity, chance);
}

impl Traffic {
    /// Rolls the animation of a spawn of most origins on class 128's
    /// display `spec`, its first class (see [`FirstClass`]).
    pub fn set_first_class(&mut self, spec: FrameSpec) {
        self.first_class.spec = spec;
    }

    /// Rolls the animation of a spawn of most origins on the first class
    /// by `reading`
    /// ([`RuleKey::SpawnAnimRollsFirstClass`](crate::RuleKey::SpawnAnimRollsFirstClass)):
    /// by the engine on class 128's, on its own by the other reading.
    pub fn set_spawn_anim_rolls_first_class(&mut self, reading: RuleSource) {
        self.first_class.reading = reading;
    }

    /// The first class spawns roll their animation on, and the reading
    /// they do so by.
    #[must_use]
    pub fn first_class(&self) -> FirstClass {
        self.first_class
    }

    /// Every NPC's display this frame ([`display`]) by `rules`, after the
    /// flight and the fight: a ship of a government `derelict` says is
    /// derelict glowing 0 from its spawn, its spawn's rolls and its random
    /// lights rolled on `chance`; then each blink timer kept as its
    /// berth's ([`Traffic::keep_berth_blink`]).
    pub fn display_ships(
        &mut self,
        derelict: impl Fn(Option<GovtId>) -> bool,
        rules: DisplayRules,
        chance: &mut (impl Chance + ?Sized),
    ) {
        let first = self.first_class;
        for npc in &mut self.npcs {
            let derelict = derelict(npc.govt);
            display(npc, derelict, rules, &first, &mut *chance);
        }
        self.keep_berth_blink();
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
    use crate::traffic::spawn::{Entry, NewShip};
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
                anim: AnimRoll::None,
                jump_in: false,
                derelict_glow: true
            }
        );
        let first = Spawn {
            anim: AnimRoll::FirstClass,
            ..Spawn::default()
        };
        assert_eq!(
            NpcDisplay::spawned(&spec, first).0.rolls.anim,
            AnimRoll::FirstClass
        );
        let person = Spawn {
            person: true,
            ..Spawn::default()
        };
        assert_eq!(NpcDisplay::spawned(&spec, person).0.glow, 0);
        // A derelict government's ship's glow is 0 by its first display.
        for (derelict, glow) in [(true, 0), (false, 32)] {
            let mut ship = folding(0, 1);
            ship.display = spawned;
            display(
                &mut ship,
                derelict,
                DisplayRules::ENGINE,
                &FirstClass::default(),
                &mut Draws::of(&[]),
            );
            assert_eq!(ship.display.glow, glow, "{derelict}");
            ship.display.glow = 9;
            display(
                &mut ship,
                derelict,
                DisplayRules::ENGINE,
                &FirstClass::default(),
                &mut Draws::of(&[]),
            );
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
            anim: AnimRoll::Own,
            jump_in: true,
            derelict_glow: false,
        };
        let mut draws = Draws::of(&[2, 3]);
        display(
            &mut ship,
            false,
            DisplayRules::ENGINE,
            &FirstClass::default(),
            &mut draws,
        );
        assert_eq!(draws.asked, [4, 5], "Rand(BaseSets), Rand(AnimDelay)");
        assert_eq!(ship.display.frames.bank, 0.0, "no bank or fold to roll");
        assert_eq!(ship.display.frames.anim_timer, 4.0, "3, then a frame");
        assert_eq!(ship.display.drawn.set, 2);
        assert_eq!(ship.display.rolls, Rolls::default());
        display(
            &mut ship,
            false,
            DisplayRules::ENGINE,
            &FirstClass::default(),
            &mut draws,
        );
        assert_eq!(draws.asked.len(), 2, "once");
        let owed = Rolls {
            anim: AnimRoll::Own,
            jump_in: true,
            derelict_glow: false,
        };
        // A banking ship jumping in rolls its bank; a plain one nothing.
        let mut banking = folding(BANKS, 3);
        banking.display.rolls = owed;
        let mut draws = Draws::of(&[7]);
        settle_spawn(&mut banking, false, &FirstClass::default(), &mut draws);
        assert_eq!(draws.asked, [10], "Rand(10) alone");
        assert_eq!(banking.display.frames.bank, -7.0);
        let mut plain = folding(0, 1);
        plain.display.rolls = owed;
        let mut draws = Draws::of(&[]);
        settle_spawn(&mut plain, false, &FirstClass::default(), &mut draws);
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
        settle_spawn(&mut alt, false, &FirstClass::default(), &mut draws);
        assert_eq!(draws.asked, [5]);
        assert_eq!(alt.display.frames.anim_timer, 4.0);
        // A delay of 1 or less has one outcome.
        let mut quick = folding(ANIMATED, 4);
        quick.hull.frames.anim_delay = 1;
        quick.display.rolls = owed;
        let mut draws = Draws::of(&[3]);
        settle_spawn(&mut quick, false, &FirstClass::default(), &mut draws);
        assert_eq!(draws.asked, [4], "Rand(BaseSets) alone");
    }

    /// A placed ship of type 200, a `düde`'s own ship when it carries
    /// `booty`.
    fn new_ship(booty: Option<u16>) -> NewShip {
        NewShip {
            ship: ShipId(200),
            govt: None,
            ai_type: crate::traffic::npc::AiType::WimpyTrader,
            lead: None,
            state: ShipState::default(),
            entry: Entry::Placed,
            aggression: 0,
            booty,
            info_types: 0,
            person: None,
            mission: None,
            aux: None,
            derelict: false,
            held: false,
            boarded: false,
            cloaked: false,
            quote: None,
            cargo: None,
        }
    }

    #[test]
    fn each_spawns_origin_picks_the_class_it_rolls_its_animation_on() {
        let of = |ship: NewShip| Spawn::of(&ship).anim;
        assert_eq!(of(new_ship(Some(0))), AnimRoll::Own, "a düde's own ship");
        assert_eq!(of(new_ship(None)), AnimRoll::FirstClass, "a fleet's ship");
        let mark = crate::traffic::npc::MissionMark {
            slot: 0,
            behav: crate::traffic::ShipBehav::Own,
            formation: None,
            order: None,
        };
        let mission = NewShip {
            mission: Some(mark),
            ..new_ship(Some(0))
        };
        assert_eq!(of(mission), AnimRoll::Own, "a mission's ship");
        let aux = NewShip {
            aux: Some(0),
            ..new_ship(Some(0))
        };
        assert_eq!(of(aux), AnimRoll::FirstClass, "an aux ship");
        let person = NewShip {
            person: Some(crate::catalog::PersonId(128)),
            ..new_ship(None)
        };
        assert_eq!(of(person), AnimRoll::FirstClass, "a person");
        let person_dude = NewShip {
            person: Some(crate::catalog::PersonId(128)),
            ..new_ship(Some(0))
        };
        assert_eq!(
            of(person_dude),
            AnimRoll::FirstClass,
            "a person, booty or not"
        );
    }

    /// Class 128 as the stock Shuttle has it: three base sets, no
    /// `AnimDelay`.
    fn shuttle_first(reading: RuleSource) -> FirstClass {
        FirstClass {
            spec: FrameSpec {
                base_sets: 3,
                anim_delay: 0,
                ..FrameSpec::NONE
            },
            reading,
        }
    }

    /// The sides `npc`'s spawn asks of its rolls, owing `anim`.
    fn asked(anim: AnimRoll, first: &FirstClass) -> Vec<u32> {
        let mut ship = folding(ANIMATED, 4);
        ship.display.rolls.anim = anim;
        let mut draws = Draws::of(&[2, 3]);
        settle_spawn(&mut ship, false, first, &mut draws);
        draws.asked
    }

    #[test]
    fn a_first_class_spawn_rolls_on_class_128s_by_the_engine_and_its_own_by_the_other() {
        let engine = shuttle_first(RuleSource::Engine);
        assert_eq!(
            asked(AnimRoll::FirstClass, &engine),
            [3],
            "Rand(3), and no timer roll for an AnimDelay of 0"
        );
        let bible = shuttle_first(RuleSource::Bible);
        assert_eq!(asked(AnimRoll::FirstClass, &bible), [4, 5], "its own");
        assert_eq!(asked(AnimRoll::Own, &engine), [4, 5], "its own");
        assert_eq!(asked(AnimRoll::None, &engine), Vec::<u32>::new());
        // A class that does not animate reads neither, whatever it rolls on.
        let mut plain = folding(0, 1);
        plain.display.rolls.anim = AnimRoll::FirstClass;
        let mut draws = Draws::of(&[]);
        settle_spawn(&mut plain, false, &engine, &mut draws);
        assert_eq!(draws.asked, Vec::<u32>::new());
        assert_eq!(engine.roll_on(AnimRoll::None, &plain.hull.frames), None);
    }

    #[test]
    fn the_traffic_keeps_its_first_class_and_reading() {
        let mut traffic = Traffic::new();
        assert_eq!(traffic.first_class(), FirstClass::default());
        let first = shuttle_first(RuleSource::Bible);
        traffic.set_first_class(first.spec);
        traffic.set_spawn_anim_rolls_first_class(RuleSource::Bible);
        assert_eq!(traffic.first_class(), first);
        // Its display rolls a first-class spawn on it.
        let mut ship = folding(ANIMATED, 4);
        ship.display.rolls.anim = AnimRoll::FirstClass;
        traffic.npcs = vec![ship];
        traffic.set_spawn_anim_rolls_first_class(RuleSource::Engine);
        let mut draws = Draws::of(&[2]);
        traffic.display_ships(|_| false, DisplayRules::ENGINE, &mut draws);
        assert_eq!(draws.asked, [3]);
        assert_eq!(traffic.npcs()[0].display.drawn.set, 2);
    }

    #[test]
    fn the_display_reads_its_condition_and_key_ship() {
        let mut ship = folding(KEY_CARRIED, 2);
        ship.hull.key_carried = Some(ShipId(140));
        display(
            &mut ship,
            false,
            DisplayRules::ENGINE,
            &FirstClass::default(),
            &mut Draws::of(&[]),
        );
        assert_eq!(ship.display.drawn.set, 0, "no bay aboard");
        let mut still = folding(ANIMATED | 0x0010, 3);
        still.condition = Condition::Disabled;
        display(
            &mut still,
            false,
            DisplayRules::ENGINE,
            &FirstClass::default(),
            &mut Draws::of(&[]),
        );
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
