//! How an NPC fights, frame by frame: `_LowLevelAIHandler`'s substates
//! 5, 6, 0x10, 0x11 and 7 (@0x85c62-0x869ff in the `EV Nova`
//! executable) and the substate `_HighLevelAIHandler` leaves them in
//! (@0x8e3fc-0x8e6dc, @0x8e017-0x8e06b).
//!
//! Each tick a fighting NPC's step ([`step`]) works out its frame
//! ([`Frame`]): the bodies the low level runs, in its block order, each
//! with the heading it wants (the original's +0x68), and the substate it
//! keeps for the next tick ([`Kept`], the original's 0x10 or 0x11 in
//! +0xc8ca). Its passes ([`fire`]) run every body's fire, in order; its
//! flight ([`autopilot`](crate::traffic::autopilot)) is the last body's,
//! as 0x10 and 0x11 always write the thrust +0x30, the cap +0x34 and the
//! heading +0x68 after the bodies before them.
//!
//! - **The high level** ([`high_level`]), every think: attacking, a
//!   stand-off ship keeps its band ([`stand_off::keep`]: back away (5),
//!   approach (7) or snipe (0xe)) and never keeps a pursuit; any other
//!   outside [`DOGFIGHT_BOX`] on either axis approaches (7) unless it is
//!   rushing (0x11, @0x8e68d-0x8e697), and inside dogfights (6) unless it
//!   is breaking off (0x10, @0x8e6cf-0x8e6dc). Sniping, it snipes (0xe).
//!   Plundering an intact ship (goal 0xd, [`Goal::Plunder`],
//!   @0x8ea49-0x8eb3a), it keeps no band: outside [`DOGFIGHT_BOX`] it
//!   snipes when the chase is hopeless, else approaches unless rushing;
//!   inside it dogfights unless breaking off or **rushing**, both kept
//!   (`sub - 0x10 <= 1` @0x8eb28). Plundering a disabled ship it flies no
//!   fight: it docks (@0x8e873-0x8ea44, see
//!   [`plunder_dock`](crate::traffic::plunder_dock)).
//!   Fleeing, within [`FLEE_REACH`] of its attacker on both axes, the
//!   offsets truncated, it flees close (5, @0x8e017-0x8e06b); beyond, it
//!   runs out (substates 1, 3 and 4: past 1000 of the centre and able to
//!   jump it brakes unless it expedites, then jumps, as
//!   [`leave::flee_leg`](crate::ai::leave::flee_leg) says), which run no
//!   pass (no `_AIFireTurret`, so no point defence either). Nothing else clears a
//!   kept pursuit: not a new target, nor a goal change in itself, as
//!   nothing but these rewrites touches +0xc8ca in state 4.
//! - **Flee close** (5, [`Manoeuvre::Flee`]): it wants the table bearing
//!   from its attacker to itself and runs its turret pass; with an
//!   afterburner ([`Npc::afterburner`]) and its attacker within
//!   [`DOGFIGHT_BOX`] on at least one axis (`|d|` < 165,
//!   @0x85d77-0x85e0b) it rushes (0x11) in the same frame.
//! - **Dogfight** (6, [`Manoeuvre::Dogfight`]): its lead
//!   ([`fire::dogfight_lead`]), its turret and gun passes; then the joust
//!   ([`joust`]) may break it off (0x10), and the rush entry
//!   ([`rush_entry`]), tested after it even when it passed, may rush it
//!   (0x11), either in the same frame.
//! - **Break off** (0x10, [`Manoeuvre::BreakOff`]): it wants its break
//!   heading (+0x8e), thrusts every tick at [`BREAK_THRUST`] times its
//!   acceleration, runs its turret pass alone inside [`FIRE_BOX`] on both
//!   axes, and drops back to dogfight once it faces the break heading
//!   within [`GUN_FACING`] turns, strictly (@0x86515-0x86563).
//! - **Rush** (0x11, [`Manoeuvre::Rush`]): it wants the table bearing to
//!   its target, thrusts every tick at [`RUSH_THRUST`] times its
//!   acceleration up to [`RUSH_SPEED`] times its top speed; inside
//!   [`FIRE_BOX`] its turret pass, then its gun pass facing within
//!   [`GUN_FACING`] turns; outside, its missile pass facing within the
//!   same. Then inside [`DOGFIGHT_BOX`] on both axes, truncated, it drops
//!   back to dogfight at once; outside, on `Rand(`[`RUSH_DROP`]`)` == 0
//!   (@0x86755-0x867b2).
//! - **Approach** (7, [`Manoeuvre::Approach`]): its lead
//!   ([`fire::approach_lead`]) and passes, then the rush entry, which runs
//!   on the next tick.
//! - Every facing test is the original's: `_AngularDifference` of the
//!   whole-degree heading and the heading wanted
//!   ([`fire::faces`](crate::ai::fire::faces)).
//! - None of these blocks calls `PlaySound` or `_beepSnd`.
//!
//! # Not modelled
//!
//! - The swarm's `_AIMaintainFormation` and +0xc8d4 copy in subs 5, 0x11
//!   and 7 (@0x85c87-0x85cbf, @0x8660b-0x8663d, @0x867e0-0x86818): rdm
//!   phase combat-ai-fidelity/phase-14-swarming.
//! - Inertialess flight's branches: sub 6's speed match within 100
//!   (@0x85ff3-0x860a6), the joust's skip outside 165 (@0x86143-0x86150)
//!   and 0x10's exit outside 165 (@0x864bd-0x86513). The engine has no
//!   inertialess flight (rdm phase
//!   ship-handling-fidelity/phase-5-inertialess-flight).
//! - The lead of a pick or last gun the ship does not carry, which the
//!   original reads from its global weapon record (@0x85e4c-0x85e59,
//!   @0x2b06a-0x2b075), and a negative `Mass` in the joust: rdm phase
//!   combat-ai-fidelity/phase-17-left-out-of-phase-7.
//! - The order of the joust's draws: the rolls come after its other
//!   gates here, which decides the same.

use crate::ai::afterburner;
use crate::ai::fire::{self, DOGFIGHT_BOX, GUN_FACING, Manoeuvre};
use crate::ai::{Goal, Surroundings, stand_off};
use crate::catalog::WeaponId;
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::combat::hull::Condition;
use crate::flight::ShipState;
use crate::geometry::Vec2;
use crate::rulebook::RuleSource;
use crate::traffic::npc::Npc;
use crate::trig;

/// How near, in whole pixels on both axes, a fleeing ship's attacker must
/// be for it to flee close (250, `cmpw $0xfa` @0x8e025, @0x8e047).
pub const FLEE_REACH: i32 = 250;
/// How near, in whole pixels on both axes, a ship breaking off or rushing
/// must be for its turret (and a rush's gun) pass (`trunc(d) + 164`
/// unsigned ≤ 328, @0x86479-0x864ad, @0x86644-0x86689).
pub const FIRE_BOX: i32 = 164;
/// How near, in whole pixels on both axes, a rushing ship drops back to
/// dogfight at once (`trunc(d) + 165` unsigned ≤ 330, @0x86755-0x8679c).
pub const STAY_BOX: i32 = 165;
/// `Rand(100)`: the roll a rushing ship outside [`STAY_BOX`] drops back on
/// at 0 (@0x8679e).
pub const RUSH_DROP: u16 = 100;
/// How many times its acceleration a ship breaking off thrusts (1.5
/// @0xdd078).
pub const BREAK_THRUST: f32 = 1.5;
/// How many times its acceleration a rushing ship thrusts (2.75
/// @0xdd96c).
pub const RUSH_THRUST: f32 = 2.75;
/// How many times its top speed a rushing ship flies up to (the double
/// 1.8 @0xdd6e0).
pub const RUSH_SPEED: f64 = 1.8;
/// How far its break heading turns from its heading, in degrees (135.0
/// @0xdda3c): clockwise from an even slot, anticlockwise from an odd one.
pub const BREAK_TURN: f32 = 135.0;
/// How near, in whole pixels on both axes, a joust's target must be
/// (`cmpl $0x7a` @0x861eb, @0x8620e).
pub const JOUST_RANGE: i32 = 122;
/// How far off, in whole degrees, each ship of a joust may face (`cmpw
/// $0x1e` @0x8626f, @0x862c7), and a ship rushing in may face its target
/// (@0x863f5, @0x869ed).
pub const FACE_OFF: i16 = 30;
/// The `InherentAI` a ship's class must be above to break off (`cmpw $0x2`
/// @0x8619a).
pub const JOUST_AI: i16 = 2;
/// The `Mass`, in tons, a ship's class may be at most to break off (`cmpw
/// $0xc7` @0x861ac).
pub const JOUST_MASS: f32 = 199.0;
/// `Rand(2)`: the roll a joust against an NPC goes on at 0 (@0x86160).
pub const JOUST_ROLL: u16 = 2;
/// How far, in whole pixels on both axes, a ship must be from its target
/// to rush in (`cmpl $0x52`, strictly, @0x863b2, @0x863ce, @0x869a4,
/// @0x869c3).
pub const RUSH_CLEARANCE: i32 = 82;

/// The substate a ship keeps from tick to tick: its pursuit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kept {
    /// Breaking off (0x10) toward this whole-degree break heading (+0x8e).
    BreakOff {
        /// The heading it breaks off along.
        heading: i16,
    },
    /// Rushing on its afterburner (0x11).
    Rush,
}

/// One body a frame runs: how it flies and fires, and the whole-degree
/// heading it wants (+0x68).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Body {
    /// Its manoeuvre.
    pub manoeuvre: Manoeuvre,
    /// The heading it wants.
    pub heading: i16,
}

/// The bodies a fighting ship's low level runs this tick, in block order
/// (up to two), for the goal it was worked out under.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frame {
    /// The goal it was worked out under: a frame for any other is stale.
    pub goal: Goal,
    /// The bodies, in order; none for a ship running out.
    pub bodies: [Option<Body>; 2],
}

impl Frame {
    /// A frame of `goal` running `bodies`, in order: two at most, as no
    /// block chain runs more.
    fn of(goal: Goal, bodies: &[Body]) -> Self {
        debug_assert!(bodies.len() <= 2, "{bodies:?}");
        let mut frame = Self {
            goal,
            bodies: [None; 2],
        };
        for (slot, body) in frame.bodies.iter_mut().zip(bodies) {
            *slot = Some(*body);
        }
        frame
    }

    /// Its bodies, in order.
    pub fn iter(&self) -> impl Iterator<Item = Body> + '_ {
        self.bodies.iter().flatten().copied()
    }

    /// Its last body, whose flight wins: none for a ship running out.
    #[must_use]
    pub fn last(&self) -> Option<Body> {
        self.iter().last()
    }
}

/// What an NPC holds of its pursuit: the substate it keeps and this
/// tick's frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Pursuit {
    /// The pursuit it keeps for the next tick.
    pub kept: Option<Kept>,
    /// This tick's frame, once its step has run.
    pub frame: Option<Frame>,
}

/// The rulebook's entries the step follows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PursuitRules {
    /// Which weapon an approach leads with
    /// ([`RuleKey::ApproachLead`](crate::RuleKey::ApproachLead)).
    pub approach_lead: RuleSource,
}

/// Whether `off` lies within `reach` whole pixels on both axes, each
/// axis truncated (`cvttss2si`) as the original's boxes are.
fn within_whole(off: Vec2, reach: i32) -> bool {
    whole_off(off.x).abs() <= reach && whole_off(off.y).abs() <= reach
}

/// An offset truncated to whole pixels (`cvttss2si`).
#[allow(clippy::cast_possible_truncation)] // Offsets in a system fit an i32.
fn whole_off(offset: f32) -> i32 {
    offset as i32
}

/// The substate `npc`'s high level leaves it in, at a ship at `at`,
/// disabled as `disabled` says (see the module docs): where its frame
/// starts. None for a goal that is no fight.
#[must_use]
pub fn high_level(npc: &Npc, at: &ShipState, disabled: bool) -> Option<Start> {
    let off = at.position - npc.state.position;
    if matches!(npc.goal, Goal::Attack(_) | Goal::Snipe(_))
        && let Some(band) = stand_off::band(npc, disabled)
    {
        return Some(Start::Sub(stand_off::keep(off, band)));
    }
    match npc.goal {
        Goal::Attack(_) => {
            let outside = off.x.abs() > DOGFIGHT_BOX || off.y.abs() > DOGFIGHT_BOX;
            Some(Start::Sub(match (outside, npc.pursuit.kept) {
                (true, Some(Kept::Rush)) => Manoeuvre::Rush,
                (true, _) => Manoeuvre::Approach,
                (false, Some(Kept::BreakOff { heading })) => {
                    return Some(Start::BreakOff { heading });
                }
                (false, _) => Manoeuvre::Dogfight,
            }))
        }
        Goal::Snipe(_) => Some(Start::Sub(Manoeuvre::Snipe)),
        // Goal 0xd against an intact ship (@0x8ea49-0x8eb3a); against a
        // disabled one it docks (@0x8e873-0x8ea44, `plunder_dock`).
        Goal::Plunder { .. } if disabled => None,
        Goal::Plunder { hopeless, .. } => {
            let outside = off.x.abs() > DOGFIGHT_BOX || off.y.abs() > DOGFIGHT_BOX;
            Some(match (outside, npc.pursuit.kept) {
                (true, _) if hopeless => Start::Sub(Manoeuvre::Snipe),
                (_, Some(Kept::Rush)) => Start::Sub(Manoeuvre::Rush),
                (true, _) => Start::Sub(Manoeuvre::Approach),
                (false, Some(Kept::BreakOff { heading })) => Start::BreakOff { heading },
                (false, None) => Start::Sub(Manoeuvre::Dogfight),
            })
        }
        Goal::Flee(_) if within_whole(off, FLEE_REACH) => Some(Start::Sub(Manoeuvre::Flee)),
        Goal::Flee(_) => Some(Start::RunningOut),
        _ => None,
    }
}

/// Where a frame starts (see [`high_level`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Start {
    /// In this substate.
    Sub(Manoeuvre),
    /// Breaking off along its kept break heading.
    BreakOff {
        /// The heading it breaks off along.
        heading: i16,
    },
    /// Running out from its attacker, beyond [`FLEE_REACH`]: no body.
    RunningOut,
}

/// The frame `npc` flies at a ship at `at` before its step has run this
/// tick (see [`step`]): its stored frame while it is for its goal, else
/// the high level's substate alone, its lead from the pick it holds and
/// its approach's by the engine. None for a goal that is no fight.
#[must_use]
pub fn frame_of(npc: &Npc, at: &ShipState, disabled: bool) -> Option<Frame> {
    if let Some(frame) = npc.pursuit.frame
        && frame.goal == npc.goal
    {
        return Some(frame);
    }
    let start = high_level(npc, at, disabled)?;
    let looks = Looks::of(npc, at, npc.pick, RuleSource::Engine);
    Some(match start {
        Start::Sub(manoeuvre) => Frame::of(npc.goal, &[looks.body(manoeuvre)]),
        Start::BreakOff { heading } => Frame::of(
            npc.goal,
            &[Body {
                manoeuvre: Manoeuvre::BreakOff,
                heading,
            }],
        ),
        Start::RunningOut => Frame::of(npc.goal, &[]),
    })
}

/// The headings each body wants, worked out once at the frame's start.
struct Looks {
    away: i16,
    bearing: i16,
    dogfight: i16,
    approach: i16,
}

impl Looks {
    /// `npc`'s headings at a ship at `at`, leading with `pick` as its
    /// pick and its approach by `approach_lead`.
    fn of(npc: &Npc, at: &ShipState, pick: Option<WeaponId>, approach_lead: RuleSource) -> Self {
        Self {
            away: trig::intercept_angle(at.position, npc.state.position),
            bearing: trig::intercept_angle(npc.state.position, at.position),
            dogfight: fire::dogfight_lead(npc, at, pick),
            approach: fire::approach_lead(npc, at, pick, approach_lead),
        }
    }

    /// The body of `manoeuvre`, with the heading it wants. A break-off's
    /// is its kept heading, given apart.
    fn body(&self, manoeuvre: Manoeuvre) -> Body {
        let heading = match manoeuvre {
            Manoeuvre::Flee => self.away,
            Manoeuvre::Dogfight => self.dogfight,
            Manoeuvre::Approach => self.approach,
            Manoeuvre::Rush | Manoeuvre::Snipe | Manoeuvre::BreakOff => self.bearing,
        };
        Body { manoeuvre, heading }
    }
}

/// What the joust and the rush entry read of a ship and its target.
struct Duel {
    /// Its offset from its target (self − target).
    off: Vec2,
    /// Its whole-degree heading.
    heading: i16,
    /// The table bearing to its target.
    bearing: i16,
    /// Its target's whole-degree heading.
    target_heading: i16,
    /// Whether its target is the player.
    at_player: bool,
    /// Its target's ship-table slot: the player's is 0.
    target_slot: u32,
    /// Its target's class's mass.
    target_mass: f32,
    /// The player's combat rating.
    rating: i64,
}

/// Whether `npc` jousting `duel` breaks off, rolling on `chance` (see the
/// module docs; @0x860c6-0x86242): its class's hull, its target within
/// [`JOUST_RANGE`], the slot and mass tiebreak, both facing each other
/// within [`FACE_OFF`], and the roll.
fn joust(npc: &Npc, duel: &Duel, chance: &mut (impl Chance + ?Sized)) -> bool {
    let hull = npc.hull.inherent_ai > JOUST_AI && npc.hull.mass <= JOUST_MASS;
    let near =
        whole_off(duel.off.x).abs() <= JOUST_RANGE && whole_off(duel.off.y).abs() <= JOUST_RANGE;
    let tiebreak = duel.target_slot < npc.berth
        || (duel.target_slot > npc.berth && npc.hull.mass < duel.target_mass);
    let reverse = (i32::from(duel.bearing) + 180).rem_euclid(360);
    #[allow(clippy::cast_possible_truncation)] // In 0..360.
    let reverse = reverse as i16;
    let face_off = trig::angular_difference(duel.heading, duel.bearing) <= FACE_OFF
        && trig::angular_difference(duel.target_heading, reverse) <= FACE_OFF;
    if !(hull && near && tiebreak && face_off) {
        return false;
    }
    // Against an NPC, `Rand(2)` == 0 goes on, or the complex roll; against
    // the player, the complex roll alone (@0x86156-0x8617e).
    (!duel.at_player && chance.roll(JOUST_ROLL) == 0)
        || afterburner::performs_complex_manoeuvres(duel.rating, chance)
}

/// The break heading of a ship heading `heading` in slot `berth`:
/// `trunc(heading ± 135)`, wrapped once into 0..360 (@0x862d1-0x86338).
#[must_use]
pub fn break_heading(heading: f32, berth: u32) -> i16 {
    let turned = if berth.is_multiple_of(2) {
        heading + BREAK_TURN
    } else {
        heading - BREAK_TURN
    };
    let mut whole = trig::whole(turned);
    if whole < 0 {
        whole += 360;
    }
    if whole > 359 {
        whole -= 360;
    }
    whole
}

/// Whether `npc` rushes in on `duel` (see the module docs): an afterburner,
/// more than [`RUSH_CLEARANCE`] off on both axes, and facing its target
/// within [`FACE_OFF`] (@0x86371-0x86404, @0x86969-0x869f6).
fn rush_entry(npc: &Npc, duel: &Duel) -> bool {
    npc.afterburner
        && whole_off(duel.off.x).abs() > RUSH_CLEARANCE
        && whole_off(duel.off.y).abs() > RUSH_CLEARANCE
        && trig::angular_difference(duel.heading, duel.bearing) <= FACE_OFF
}

/// `npc`'s pursuit this tick among `around`, by `rules`, rolling on
/// `chance` (see the module docs): its frame at the ship its goal is
/// about, and the pursuit it keeps. None of either for a goal that is no
/// fight or a ship not in the system.
pub fn step(
    npc: &Npc,
    around: &Surroundings,
    rules: PursuitRules,
    chance: &mut (impl Chance + ?Sized),
) -> Pursuit {
    let Some(ship) = npc.goal.quarry() else {
        return Pursuit::default();
    };
    let Some(at) = around.state_of(ship) else {
        return Pursuit::default();
    };
    let disabled = around.condition_of(ship) == Some(Condition::Disabled);
    let Some(start) = high_level(npc, &at, disabled) else {
        return Pursuit::default();
    };
    let looks = Looks::of(npc, &at, fire::lead_pick(npc, around), rules.approach_lead);
    let (target_slot, target_mass) = match ship {
        ShipRef::Player => (0, around.player.map_or(0.0, |player| player.hull.mass)),
        ShipRef::Npc(id) => around
            .npc(id)
            .map_or((0, 0.0), |other| (other.berth, other.hull.mass)),
    };
    let duel = Duel {
        off: npc.state.position - at.position,
        heading: trig::whole(npc.state.heading),
        bearing: looks.bearing,
        target_heading: trig::whole(at.heading),
        at_player: ship == ShipRef::Player,
        target_slot,
        target_mass,
        rating: around.player.map_or(0, |player| player.rating),
    };
    let mut sub = match start {
        Start::Sub(manoeuvre) => Sub::Body(manoeuvre),
        Start::BreakOff { heading } => Sub::BreakOff(heading),
        Start::RunningOut => {
            return Pursuit {
                kept: None,
                frame: Some(Frame::of(npc.goal, &[])),
            };
        }
    };
    let mut bodies: Vec<Body> = Vec::with_capacity(2);
    // Sub 5 (@0x85c62-0x85e0b).
    if sub == Sub::Body(Manoeuvre::Flee) {
        bodies.push(looks.body(Manoeuvre::Flee));
        if npc.afterburner && (duel.off.x.abs() < DOGFIGHT_BOX || duel.off.y.abs() < DOGFIGHT_BOX) {
            sub = Sub::Body(Manoeuvre::Rush);
        }
    }
    // Sub 6 (@0x85e14-0x8640d): the joust, then the rush entry over it.
    if sub == Sub::Body(Manoeuvre::Dogfight) {
        bodies.push(looks.body(Manoeuvre::Dogfight));
        if joust(npc, &duel, chance) {
            sub = Sub::BreakOff(break_heading(npc.state.heading, npc.berth));
        }
        if rush_entry(npc, &duel) {
            sub = Sub::Body(Manoeuvre::Rush);
        }
    }
    // Sub 0x10 (@0x86410-0x86563).
    if let Sub::BreakOff(heading) = sub {
        bodies.push(Body {
            manoeuvre: Manoeuvre::BreakOff,
            heading,
        });
        if fire::faces(npc, heading, GUN_FACING) {
            sub = Sub::Body(Manoeuvre::Dogfight);
        }
    }
    // Sub 0x11 (@0x8656c-0x867b2).
    if sub == Sub::Body(Manoeuvre::Rush) {
        bodies.push(looks.body(Manoeuvre::Rush));
        if within_whole(duel.off, STAY_BOX) || chance.roll(RUSH_DROP) == 0 {
            sub = Sub::Body(Manoeuvre::Dogfight);
        }
    }
    // Sub 7 (@0x867bb-0x869ff): its rush entry runs on the next tick.
    if sub == Sub::Body(Manoeuvre::Approach) {
        bodies.push(looks.body(Manoeuvre::Approach));
        if rush_entry(npc, &duel) {
            sub = Sub::Body(Manoeuvre::Rush);
        }
    }
    // A snipe (0xe) is a block of its own.
    if sub == Sub::Body(Manoeuvre::Snipe) {
        bodies.push(looks.body(Manoeuvre::Snipe));
    }
    Pursuit {
        kept: match sub {
            Sub::BreakOff(heading) => Some(Kept::BreakOff { heading }),
            Sub::Body(Manoeuvre::Rush) => Some(Kept::Rush),
            Sub::Body(_) => None,
        },
        frame: Some(Frame::of(npc.goal, &bodies)),
    }
}

/// The substate +0xc8ca as the low level's blocks pass it on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Sub {
    /// Any substate but a break-off.
    Body(Manoeuvre),
    /// Breaking off (0x10) along this heading (+0x8e).
    BreakOff(i16),
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::PlayerSide;
    use crate::ai::fixture::player;
    use crate::combat::ShipRef;
    use crate::stats::ShipStats;
    use crate::traffic::npc::NpcId;
    use nova_sim_base::fixture::{FAST, Ranged};

    /// Where the fighter is: off the centre, so every offset is measured
    /// from it.
    const AT: Vec2 = Vec2::new(300.0, 200.0);
    /// An NPC as a ship.
    const OTHER: ShipRef = ShipRef::Npc(NpcId(2));

    /// A fighter at [`AT`] facing `heading` (3 degrees a tick), in slot 2,
    /// of a class that may break off (`InherentAI` 3, 100 tons), with
    /// `goal`, no afterburner.
    fn fighter(goal: Goal, heading: f32) -> Npc {
        let mut npc = crate::traffic::fixture::npc(1, ShipStats::new(FAST, &[]));
        npc.goal = goal;
        npc.berth = 2;
        npc.state.position = AT;
        npc.state.heading = heading;
        npc.hull.inherent_ai = 3;
        npc.hull.mass = 100.0;
        npc
    }

    /// The player `off` from [`AT`], facing `heading`.
    fn player_off(off: Vec2, heading: f32) -> PlayerSide {
        let mut side = player(AT.x + off.x, AT.y + off.y);
        side.state.heading = heading;
        side
    }

    /// `npc`'s step at the player `side` of combat rating `rating`, on
    /// `chance`.
    fn stepped_at_player(
        npc: &Npc,
        side: &PlayerSide,
        rating: i64,
        chance: &mut Ranged,
    ) -> Pursuit {
        let npcs = [npc.clone()];
        let around = Surroundings {
            player: Some(PlayerSide { rating, ..*side }),
            ..Surroundings::new(&[], &npcs)
        };
        step(npc, &around, PursuitRules::default(), chance)
    }

    /// `npc`'s step at NPC 2, `off` from [`AT`] facing `heading`, in
    /// `slot`, of `mass` tons, with the player's rating `rating`, on
    /// `chance`.
    fn stepped_at_npc(
        npc: &Npc,
        (off, heading): (Vec2, f32),
        (slot, mass): (u32, f32),
        rating: i64,
        chance: &mut Ranged,
    ) -> Pursuit {
        let mut other = crate::traffic::fixture::npc(2, ShipStats::new(FAST, &[]));
        other.state.position = AT + off;
        other.state.heading = heading;
        other.berth = slot;
        other.hull.mass = mass;
        let npcs = [npc.clone(), other];
        let around = Surroundings {
            player: Some(PlayerSide {
                rating,
                ..player(0.0, 0.0)
            }),
            ..Surroundings::new(&[], &npcs)
        };
        step(npc, &around, PursuitRules::default(), chance)
    }

    /// The manoeuvres of `pursuit`'s frame, in order.
    fn bodies(pursuit: &Pursuit) -> Vec<Manoeuvre> {
        pursuit
            .frame
            .map(|frame| frame.iter().map(|body| body.manoeuvre).collect())
            .unwrap_or_default()
    }

    const AT_PLAYER: Goal = Goal::Attack(ShipRef::Player);
    /// Straight up from [`AT`], 100 off.
    const ABOVE: Vec2 = Vec2::new(0.0, -100.0);

    /// A joust at the player above, facing down at the fighter facing up,
    /// the complex roll passing (a rating of 1599 against `Rand(1344)`'s
    /// highest outcome).
    fn jousted(npc: &Npc, off: Vec2, target_heading: f32) -> Pursuit {
        stepped_at_player(
            npc,
            &player_off(off, target_heading),
            1599,
            &mut Ranged::of(&[]),
        )
    }

    #[test]
    fn the_named_values() {
        assert_eq!(
            (FLEE_REACH, FIRE_BOX, STAY_BOX, JOUST_RANGE, RUSH_CLEARANCE),
            (250, 164, 165, 122, 82)
        );
        assert_eq!((RUSH_DROP, JOUST_ROLL, FACE_OFF, JOUST_AI), (100, 2, 30, 2));
        assert_eq!(
            (BREAK_THRUST, RUSH_THRUST, BREAK_TURN, JOUST_MASS),
            (1.5, 2.75, 135.0, 199.0)
        );
        assert_eq!(RUSH_SPEED, 1.8);
    }

    #[test]
    fn a_dogfighter_meeting_its_target_head_on_breaks_off_clockwise_from_an_even_slot() {
        let npc = fighter(AT_PLAYER, 0.0);
        let pursuit = jousted(&npc, ABOVE, 180.0);
        assert_eq!(bodies(&pursuit), [Manoeuvre::Dogfight, Manoeuvre::BreakOff]);
        assert_eq!(pursuit.kept, Some(Kept::BreakOff { heading: 135 }));
        assert_eq!(
            pursuit
                .frame
                .and_then(|frame| frame.last())
                .map(|body| body.heading),
            Some(135)
        );
    }

    #[test]
    fn the_break_heading_turns_anticlockwise_from_an_odd_slot_and_is_whole_and_wrapped() {
        assert_eq!(break_heading(0.0, 2), 135);
        assert_eq!(break_heading(0.0, 3), 225);
        assert_eq!(break_heading(10.7, 2), 145);
        assert_eq!(break_heading(300.5, 2), 75);
        // 359 stays; only 360 and over wrap.
        assert_eq!(break_heading(224.5, 2), 359);
        // Truncated toward zero: -34.1 is -34, so 326.
        assert_eq!(break_heading(100.9, 1), 326);
        let mut npc = fighter(AT_PLAYER, 0.0);
        npc.berth = 3;
        assert_eq!(
            jousted(&npc, ABOVE, 180.0).kept,
            Some(Kept::BreakOff { heading: 225 })
        );
    }

    #[test]
    fn a_class_of_inherent_ai_2_or_over_199_tons_never_breaks_off() {
        let mut npc = fighter(AT_PLAYER, 0.0);
        npc.hull.inherent_ai = 2;
        assert_eq!(jousted(&npc, ABOVE, 180.0).kept, None);
        npc.hull.inherent_ai = 3;
        npc.hull.mass = 200.0;
        assert_eq!(jousted(&npc, ABOVE, 180.0).kept, None);
        npc.hull.mass = 199.0;
        assert!(jousted(&npc, ABOVE, 180.0).kept.is_some());
    }

    #[test]
    fn a_joust_needs_its_target_within_122_whole_pixels_on_both_axes() {
        let npc = fighter(AT_PLAYER, 0.0);
        assert!(jousted(&npc, Vec2::new(0.0, -122.9), 180.0).kept.is_some());
        assert_eq!(jousted(&npc, Vec2::new(0.0, -123.0), 180.0).kept, None);
        // 30 degrees off up the screen, 122 across: the bearing still
        // within 30 of its heading of 330.
        let npc = fighter(AT_PLAYER, 315.0);
        assert!(
            jousted(&npc, Vec2::new(-100.0, -100.0), 135.0)
                .kept
                .is_some()
        );
        assert_eq!(jousted(&npc, Vec2::new(-123.0, -100.0), 135.0).kept, None);
    }

    #[test]
    fn a_joust_needs_each_ship_within_30_whole_degrees_of_facing_the_other() {
        // Its own heading off the bearing (0): 30 goes on, 31 does not.
        assert!(
            jousted(&fighter(AT_PLAYER, 30.9), ABOVE, 180.0)
                .kept
                .is_some()
        );
        assert_eq!(jousted(&fighter(AT_PLAYER, 31.0), ABOVE, 180.0).kept, None);
        assert!(
            jousted(&fighter(AT_PLAYER, 330.0), ABOVE, 180.0)
                .kept
                .is_some()
        );
        assert_eq!(jousted(&fighter(AT_PLAYER, 329.9), ABOVE, 180.0).kept, None);
        // The target's off the bearing turned about (180).
        let npc = fighter(AT_PLAYER, 0.0);
        assert!(jousted(&npc, ABOVE, 210.5).kept.is_some());
        assert_eq!(jousted(&npc, ABOVE, 211.0).kept, None);
        assert!(jousted(&npc, ABOVE, 150.0).kept.is_some());
        assert_eq!(jousted(&npc, ABOVE, 149.9).kept, None);
    }

    #[test]
    fn against_an_npc_a_lower_slot_goes_on_the_same_never_and_a_higher_only_for_a_lighter_ship() {
        let npc = fighter(Goal::Attack(OTHER), 0.0);
        // A roll of 0 on `Rand(2)` goes on with no complex roll.
        let jousted_npc = |slot: u32, mass: f32| {
            stepped_at_npc(
                &npc,
                (ABOVE, 180.0),
                (slot, mass),
                0,
                &mut Ranged::of(&[(2, 0)]),
            )
            .kept
            .is_some()
        };
        assert!(jousted_npc(1, 50.0));
        assert!(!jousted_npc(2, 500.0));
        assert!(jousted_npc(3, 100.1));
        assert!(!jousted_npc(3, 100.0));
    }

    #[test]
    fn against_an_npc_a_roll_of_0_breaks_off_and_a_1_only_with_the_complex_roll() {
        let npc = fighter(Goal::Attack(OTHER), 0.0);
        let jousted_npc = |chance: &mut Ranged, rating: i64| {
            stepped_at_npc(&npc, (ABOVE, 180.0), (1, 100.0), rating, chance)
                .kept
                .is_some()
        };
        let mut chance = Ranged::of(&[(2, 0)]);
        assert!(jousted_npc(&mut chance, 0));
        assert!(chance.was_asked(2));
        let mut chance = Ranged::of(&[(2, 1), (1344, 100)]);
        assert!(!jousted_npc(&mut chance, 355));
        assert_eq!(chance.asked, [2, 1344]);
        let mut chance = Ranged::of(&[(2, 1), (1344, 100)]);
        assert!(jousted_npc(&mut chance, 356));
    }

    #[test]
    fn against_the_player_only_the_complex_roll_decides() {
        let npc = fighter(AT_PLAYER, 0.0);
        let at_rating = |rating: i64| {
            let mut chance = Ranged::of(&[(2, 0), (1344, 44)]);
            let kept = stepped_at_player(&npc, &player_off(ABOVE, 180.0), rating, &mut chance).kept;
            assert!(!chance.was_asked(2), "no `Rand(2)` against the player");
            assert_eq!(chance.asked, [1344]);
            kept.is_some()
        };
        assert!(at_rating(300));
        assert!(!at_rating(299));
    }

    #[test]
    fn a_ship_breaking_off_keeps_it_inside_165_until_it_faces_its_break_heading_within_3_turns() {
        let mut npc = fighter(AT_PLAYER, 0.0);
        npc.pursuit.kept = Some(Kept::BreakOff { heading: 135 });
        let mut chance = Ranged::of(&[]);
        // Far off its break heading: it breaks off on, its turret alone.
        let pursuit = stepped_at_player(&npc, &player_off(ABOVE, 180.0), 0, &mut chance);
        assert_eq!(bodies(&pursuit), [Manoeuvre::BreakOff]);
        assert_eq!(pursuit.kept, Some(Kept::BreakOff { heading: 135 }));
        // 9 off (3 turns of 3): still breaking off, strictly. 126.9 is
        // 8.1 off as a float but 126, 9 off, in whole degrees: still. 127
        // is 8 off, and it drops back to dogfight.
        for heading in [126.0, 126.9] {
            npc.state.heading = heading;
            let pursuit = stepped_at_player(&npc, &player_off(ABOVE, 180.0), 0, &mut chance);
            assert_eq!(
                pursuit.kept,
                Some(Kept::BreakOff { heading: 135 }),
                "{heading}"
            );
        }
        npc.state.heading = 127.0;
        let pursuit = stepped_at_player(&npc, &player_off(ABOVE, 180.0), 0, &mut chance);
        assert_eq!(bodies(&pursuit), [Manoeuvre::BreakOff]);
        assert_eq!(pursuit.kept, None);
        assert!(chance.asked.is_empty());
    }

    #[test]
    fn outside_165_the_high_level_replaces_a_break_off_with_the_approach() {
        let mut npc = fighter(AT_PLAYER, 0.0);
        npc.pursuit.kept = Some(Kept::BreakOff { heading: 135 });
        let pursuit = stepped_at_player(
            &npc,
            &player_off(Vec2::new(0.0, -165.1), 180.0),
            0,
            &mut Ranged::of(&[]),
        );
        assert_eq!(bodies(&pursuit), [Manoeuvre::Approach]);
        assert_eq!(pursuit.kept, None);
    }

    /// `npc` with an afterburner.
    fn burning(mut npc: Npc) -> Npc {
        npc.afterburner = true;
        npc
    }

    #[test]
    fn an_approaching_afterburner_ship_more_than_82_off_on_both_axes_and_facing_rushes_from_the_next_tick()
     {
        let off = Vec2::new(-83.0, -300.0);
        let bearing = trig::intercept_angle(AT, AT + off);
        let npc = burning(fighter(AT_PLAYER, f32::from(bearing)));
        let mut chance = Ranged::of(&[]);
        let pursuit = stepped_at_player(&npc, &player_off(off, 0.0), 0, &mut chance);
        assert_eq!(bodies(&pursuit), [Manoeuvre::Approach], "this tick");
        assert_eq!(pursuit.kept, Some(Kept::Rush));
        let next = Npc {
            pursuit,
            ..npc.clone()
        };
        let pursuit = stepped_at_player(&next, &player_off(off, 0.0), 0, &mut chance);
        assert_eq!(bodies(&pursuit), [Manoeuvre::Rush], "the next");
        // 82 whole pixels across, or up, is not enough.
        for off in [Vec2::new(-82.9, -300.0), Vec2::new(-300.0, -82.9)] {
            let npc = burning(fighter(
                AT_PLAYER,
                f32::from(trig::intercept_angle(AT, AT + off)),
            ));
            assert_eq!(
                stepped_at_player(&npc, &player_off(off, 0.0), 0, &mut chance).kept,
                None,
                "{off:?}"
            );
        }
    }

    #[test]
    fn a_ship_rushes_in_only_with_an_afterburner_and_facing_within_30_whole_degrees() {
        let off = Vec2::new(-200.0, -200.0);
        let mut chance = Ranged::of(&[]);
        let kept = |npc: &Npc, chance: &mut Ranged| {
            stepped_at_player(npc, &player_off(off, 0.0), 0, chance).kept
        };
        // The table's bearing up and left is 314.
        let bearing = f32::from(trig::intercept_angle(AT, AT + off));
        assert_eq!(bearing, 314.0);
        assert_eq!(kept(&fighter(AT_PLAYER, bearing), &mut chance), None);
        assert_eq!(
            kept(&burning(fighter(AT_PLAYER, bearing + 30.9)), &mut chance),
            Some(Kept::Rush)
        );
        assert_eq!(
            kept(&burning(fighter(AT_PLAYER, bearing + 31.0)), &mut chance),
            None
        );
        assert_eq!(
            kept(&burning(fighter(AT_PLAYER, bearing - 30.0)), &mut chance),
            Some(Kept::Rush)
        );
    }

    #[test]
    fn a_dogfighting_afterburner_ship_rushes_in_the_same_tick_and_drops_back_inside_165() {
        let off = Vec2::new(-100.0, -100.0);
        let npc = burning(fighter(AT_PLAYER, 315.0));
        let mut chance = Ranged::of(&[]);
        let pursuit = stepped_at_player(&npc, &player_off(off, 0.0), 0, &mut chance);
        assert_eq!(bodies(&pursuit), [Manoeuvre::Dogfight, Manoeuvre::Rush]);
        assert_eq!(pursuit.kept, None, "inside 165 on both axes");
        assert!(!chance.was_asked(100));
    }

    #[test]
    fn a_ship_that_both_jousts_and_rushes_in_rushes() {
        // 100 off on both axes, head on: the joust's 0x10, then the rush
        // entry's 0x11 over it.
        let off = Vec2::new(-100.0, -100.0);
        let npc = burning(fighter(AT_PLAYER, 315.0));
        let pursuit = stepped_at_player(&npc, &player_off(off, 135.0), 1599, &mut Ranged::of(&[]));
        assert_eq!(bodies(&pursuit), [Manoeuvre::Dogfight, Manoeuvre::Rush]);
        assert_eq!(pursuit.kept, None);
    }

    #[test]
    fn a_rushing_ship_wants_the_plain_bearing_and_outside_165_drops_back_on_a_roll_of_0_in_100() {
        let off = Vec2::new(-200.0, -300.0);
        let mut npc = burning(fighter(AT_PLAYER, 0.0));
        npc.pursuit.kept = Some(Kept::Rush);
        npc.state.velocity = Vec2::new(3.0, 0.0);
        let rushed = |outcome: u32| {
            let mut chance = Ranged::of(&[(100, outcome)]);
            let pursuit = stepped_at_player(&npc, &player_off(off, 0.0), 0, &mut chance);
            assert_eq!(chance.asked, [100]);
            pursuit
        };
        let kept = rushed(99);
        assert_eq!(bodies(&kept), [Manoeuvre::Rush]);
        assert_eq!(kept.kept, Some(Kept::Rush));
        assert_eq!(
            kept.frame
                .and_then(|frame| frame.last())
                .map(|body| body.heading),
            Some(trig::intercept_angle(AT, AT + off)),
            "the bearing, never a lead"
        );
        assert_eq!(rushed(0).kept, None);
    }

    #[test]
    fn a_fleeing_afterburner_ship_within_250_and_165_on_an_axis_rushes_and_at_166_by_166_flees() {
        let fleeing = Goal::Flee(ShipRef::Player);
        let fled = |npc: &Npc, off: Vec2| {
            stepped_at_player(npc, &player_off(off, 0.0), 0, &mut Ranged::of(&[]))
        };
        let npc = burning(fighter(fleeing, 0.0));
        // Within 165 on one axis, 249 on the other: it rushes.
        let pursuit = fled(&npc, Vec2::new(164.9, 249.0));
        assert_eq!(bodies(&pursuit), [Manoeuvre::Flee, Manoeuvre::Rush]);
        assert_eq!(
            bodies(&fled(&npc, Vec2::new(-200.0, 100.0))),
            [Manoeuvre::Flee, Manoeuvre::Rush]
        );
        // Inside the 250 box but beyond 165 on both axes: it flees.
        assert_eq!(
            bodies(&fled(&npc, Vec2::new(166.0, 166.0))),
            [Manoeuvre::Flee]
        );
        assert_eq!(
            bodies(&fled(&npc, Vec2::new(165.0, -165.0))),
            [Manoeuvre::Flee]
        );
        // With no afterburner it flees as before.
        let plain = fighter(fleeing, 0.0);
        assert_eq!(
            bodies(&fled(&plain, Vec2::new(100.0, 100.0))),
            [Manoeuvre::Flee]
        );
    }

    #[test]
    fn a_fleeing_ship_runs_out_beyond_250_whole_pixels_with_no_body() {
        let npc = burning(fighter(Goal::Flee(ShipRef::Player), 0.0));
        let fled =
            |off: Vec2| stepped_at_player(&npc, &player_off(off, 0.0), 0, &mut Ranged::of(&[]));
        assert_eq!(
            bodies(&fled(Vec2::new(250.9, 0.0))),
            [Manoeuvre::Flee, Manoeuvre::Rush]
        );
        let out = fled(Vec2::new(251.0, 0.0));
        assert!(out.frame.is_some_and(|frame| frame.last().is_none()));
        assert_eq!(out.kept, None);
    }

    #[test]
    fn a_stand_off_afterburner_ship_backing_away_rushes_in_the_same_tick() {
        let mut npc = burning(fighter(AT_PLAYER, 0.0));
        npc.hull.stands_off = true;
        let pursuit = stepped_at_player(
            &npc,
            &player_off(Vec2::new(0.0, -100.0), 0.0),
            0,
            &mut Ranged::of(&[]),
        );
        assert_eq!(bodies(&pursuit), [Manoeuvre::Flee, Manoeuvre::Rush]);
        assert_eq!(pursuit.kept, None);
    }

    #[test]
    fn a_stand_off_ship_never_keeps_a_rush() {
        let mut npc = burning(fighter(AT_PLAYER, 0.0));
        npc.hull.stands_off = true;
        npc.pursuit.kept = Some(Kept::Rush);
        // No weapons: a band of none, so beyond the box it approaches.
        let pursuit = stepped_at_player(
            &npc,
            &player_off(Vec2::new(0.0, -400.0), 0.0),
            0,
            &mut Ranged::of(&[]),
        );
        assert_eq!(bodies(&pursuit), [Manoeuvre::Approach]);
    }

    #[test]
    fn a_new_target_keeps_the_pursuit_and_a_goal_that_is_no_attack_drops_it() {
        let off = Vec2::new(-200.0, -300.0);
        let mut npc = burning(fighter(Goal::Attack(OTHER), 0.0));
        npc.pursuit.kept = Some(Kept::Rush);
        npc.goal = AT_PLAYER;
        let pursuit = stepped_at_player(&npc, &player_off(off, 0.0), 0, &mut Ranged::of(&[]));
        assert_eq!(bodies(&pursuit), [Manoeuvre::Rush]);
        assert_eq!(pursuit.kept, Some(Kept::Rush));
        npc.goal = Goal::Idle;
        assert_eq!(
            stepped_at_player(&npc, &player_off(off, 0.0), 0, &mut Ranged::of(&[])),
            Pursuit::default()
        );
        npc.goal = Goal::Snipe(ShipRef::Player);
        let sniped = stepped_at_player(&npc, &player_off(off, 0.0), 0, &mut Ranged::of(&[]));
        assert_eq!(bodies(&sniped), [Manoeuvre::Snipe]);
        assert_eq!(sniped.kept, None);
    }

    const PLUNDER: Goal = Goal::Plunder {
        ship: ShipRef::Player,
        hopeless: false,
    };
    const HOPELESS: Goal = Goal::Plunder {
        ship: ShipRef::Player,
        hopeless: true,
    };

    /// Where `npc`'s high level starts its frame at a ship `off` from
    /// [`AT`], disabled as `disabled` says.
    fn start_at(npc: &Npc, off: Vec2, disabled: bool) -> Option<Start> {
        let at = ShipState {
            position: AT + off,
            ..ShipState::default()
        };
        high_level(npc, &at, disabled)
    }

    #[test]
    fn a_plunderer_inside_165_dogfights_keeping_a_break_off_and_a_rush() {
        let inside = Vec2::new(165.0, -165.0);
        let mut npc = fighter(PLUNDER, 0.0);
        assert_eq!(
            start_at(&npc, inside, false),
            Some(Start::Sub(Manoeuvre::Dogfight))
        );
        npc.pursuit.kept = Some(Kept::BreakOff { heading: 135 });
        assert_eq!(
            start_at(&npc, inside, false),
            Some(Start::BreakOff { heading: 135 })
        );
        npc.pursuit.kept = Some(Kept::Rush);
        assert_eq!(
            start_at(&npc, inside, false),
            Some(Start::Sub(Manoeuvre::Rush)),
            "`sub - 0x10 <= 1` keeps 0x11 too (@0x8eb28)"
        );
        let attacker = Npc {
            goal: AT_PLAYER,
            ..npc.clone()
        };
        assert_eq!(
            start_at(&attacker, inside, false),
            Some(Start::Sub(Manoeuvre::Dogfight)),
            "an attacker drops its rush inside"
        );
        let hopeless = Npc {
            goal: HOPELESS,
            pursuit: Pursuit::default(),
            ..npc
        };
        assert_eq!(
            start_at(&hopeless, inside, false),
            Some(Start::Sub(Manoeuvre::Dogfight)),
            "never a snipe inside"
        );
    }

    #[test]
    fn outside_165_a_plunderer_approaches_unless_rushing_and_snipes_when_hopeless() {
        for outside in [Vec2::new(165.1, 0.0), Vec2::new(0.0, -165.1)] {
            let mut npc = fighter(PLUNDER, 0.0);
            assert_eq!(
                start_at(&npc, outside, false),
                Some(Start::Sub(Manoeuvre::Approach)),
                "{outside:?}"
            );
            npc.pursuit.kept = Some(Kept::BreakOff { heading: 135 });
            assert_eq!(
                start_at(&npc, outside, false),
                Some(Start::Sub(Manoeuvre::Approach))
            );
            npc.pursuit.kept = Some(Kept::Rush);
            assert_eq!(
                start_at(&npc, outside, false),
                Some(Start::Sub(Manoeuvre::Rush))
            );
            npc.goal = HOPELESS;
            assert_eq!(
                start_at(&npc, outside, false),
                Some(Start::Sub(Manoeuvre::Snipe)),
                "the hopeless test comes first (@0x8eace)"
            );
        }
    }

    #[test]
    fn a_plunderer_keeps_no_stand_off_band() {
        let mut npc = fighter(PLUNDER, 0.0);
        npc.hull.stands_off = true;
        npc.armament = Armament::new([(
            WeaponSpec::new(&WeaponRecord {
                guidance: 0,
                beam_length: 1000,
                ..nova_sim_catalog::fixture::weapon(128)
            }),
            1,
        )]);
        let attacker = Npc {
            goal: AT_PLAYER,
            ..npc.clone()
        };
        let off = Vec2::new(0.0, -600.0);
        assert_eq!(
            start_at(&attacker, off, false),
            Some(Start::Sub(Manoeuvre::Snipe)),
            "within its band, an attacker snipes"
        );
        assert_eq!(
            start_at(&npc, off, false),
            Some(Start::Sub(Manoeuvre::Approach))
        );
        assert_eq!(
            start_at(&npc, Vec2::new(0.0, -100.0), false),
            Some(Start::Sub(Manoeuvre::Dogfight)),
            "no backing away"
        );
    }

    #[test]
    fn a_plunderer_of_a_disabled_ship_flies_no_fight_as_it_docks() {
        for goal in [PLUNDER, HOPELESS] {
            let npc = burning(fighter(goal, 0.0));
            assert_eq!(start_at(&npc, Vec2::new(0.0, -100.0), true), None);
            assert_eq!(start_at(&npc, Vec2::new(0.0, -600.0), true), None);
        }
    }

    #[test]
    fn a_frame_is_read_only_for_the_goal_it_was_worked_out_under() {
        let npc = burning(fighter(AT_PLAYER, 315.0));
        let off = Vec2::new(-100.0, -100.0);
        let at = player_off(off, 0.0).state;
        let mut stepped = npc.clone();
        stepped.pursuit = stepped_at_player(&npc, &player_off(off, 0.0), 0, &mut Ranged::of(&[]));
        assert_eq!(
            frame_of(&stepped, &at, false).map(|frame| frame.iter().count()),
            Some(2)
        );
        stepped.goal = Goal::Attack(OTHER);
        assert_eq!(
            frame_of(&stepped, &at, false).map(|frame| frame.iter().count()),
            Some(1),
            "the high level's substate alone"
        );
    }

    // The passes and the flight of a frame.

    use crate::ai::fire::{FireRules, trigger};
    use crate::catalog::{WeaponId, WeaponRecord};
    use crate::combat::armament::{Armament, Trigger};
    use crate::combat::weapon::WeaponSpec;
    use crate::traffic::autopilot::{self, TopSpeeds};
    use nova_sim_catalog::fixture::weapon;

    /// A gun (128), a turret (130) and a missile (131), each reaching 300.
    fn armed(mut npc: Npc, ids: &[i16]) -> Npc {
        let of = |id: i16| {
            let guidance = match id {
                130 => 4,
                131 => 1,
                _ => -1,
            };
            WeaponSpec::new(&WeaponRecord {
                guidance,
                speed: 1000,
                count: 30,
                ..weapon(id)
            })
        };
        npc.armament = Armament::new(ids.iter().map(|&id| (of(id), 1)));
        npc
    }

    /// `npc` stepped at the player `off` from [`AT`], then the trigger it
    /// holds.
    fn triggered(npc: &Npc, off: Vec2) -> Trigger {
        let npcs = [npc.clone()];
        let around = Surroundings {
            player: Some(player_off(off, 0.0)),
            ..Surroundings::new(&[], &npcs)
        };
        let mut stepped = npc.clone();
        stepped.pursuit = step(npc, &around, PursuitRules::default(), &mut Ranged::of(&[]));
        trigger(&stepped, &around, FireRules::default())
    }

    #[allow(clippy::unnecessary_wraps)]
    fn id(id: i16) -> Option<WeaponId> {
        Some(WeaponId(id))
    }

    #[test]
    fn a_ship_breaking_off_fires_its_turret_and_point_defence_only_in_the_164_box_and_never_a_gun()
    {
        let mut npc = armed(fighter(AT_PLAYER, 0.0), &[128, 130]);
        npc.pursuit.kept = Some(Kept::BreakOff { heading: 135 });
        let near = triggered(&npc, Vec2::new(0.0, -164.9));
        assert_eq!((near.only, near.point_defence), (id(130), true));
        let edge = triggered(&npc, Vec2::new(0.0, -165.0));
        assert_eq!((edge.only, edge.point_defence), (None, false), "past 164");
        let gunner = Npc {
            armament: armed(fighter(AT_PLAYER, 0.0), &[128]).armament,
            ..npc
        };
        assert_eq!(triggered(&gunner, Vec2::new(0.0, -100.0)).only, None);
    }

    #[test]
    fn a_rushing_ship_outside_the_box_fires_its_missile_facing_within_3_turns_and_no_gun() {
        let mut npc = burning(armed(fighter(AT_PLAYER, 0.0), &[128, 131]));
        npc.pursuit.kept = Some(Kept::Rush);
        let off = Vec2::new(0.0, -290.0);
        let rushing = triggered(&npc, off);
        assert_eq!((rushing.only, rushing.point_defence), (id(131), false));
        npc.state.heading = 9.0;
        assert_eq!(triggered(&npc, off).only, None, "9 off, strictly");
        let gunner = Npc {
            armament: armed(fighter(AT_PLAYER, 0.0), &[128]).armament,
            ..npc
        };
        assert_eq!(triggered(&gunner, off).only, None);
    }

    #[test]
    fn a_rushing_ship_in_the_box_runs_its_turret_then_its_gun_facing_within_3_turns() {
        // Fleeing close, it rushes in the same frame: the flee's turret
        // pass, then the rush's turret and gun passes.
        let flee = Goal::Flee(ShipRef::Player);
        let gunner = burning(armed(fighter(flee, 0.0), &[128]));
        let rushing = triggered(&gunner, Vec2::new(0.0, -100.0));
        assert_eq!((rushing.only, rushing.point_defence), (id(128), true));
        let off_by_9 = Npc {
            state: ShipState {
                heading: 9.0,
                ..gunner.state
            },
            ..gunner.clone()
        };
        assert_eq!(triggered(&off_by_9, Vec2::new(0.0, -100.0)).only, None);
        // Without the afterburner it flees, its turret pass alone.
        let plain = Npc {
            afterburner: false,
            ..gunner
        };
        assert_eq!(triggered(&plain, Vec2::new(0.0, -100.0)).only, None);
    }

    /// `npc` stepped at the player `off` from [`AT`], then flown a tick.
    fn flown(npc: &Npc, off: Vec2) -> Npc {
        let side = player_off(off, 0.0);
        let npcs = [npc.clone()];
        let around = Surroundings {
            player: Some(side),
            ..Surroundings::new(&[], &npcs)
        };
        let mut ship = npc.clone();
        ship.pursuit = step(npc, &around, PursuitRules::default(), &mut Ranged::of(&[]));
        autopilot::fly(
            &mut ship,
            &[],
            &std::collections::BTreeMap::new(),
            Some(&side.state),
            false,
            TopSpeeds::own_of(npc),
            crate::flight::FlightRules::default(),
            crate::traffic::autopilot::Readings::default(),
            &mut Ranged::of(&[]),
        );
        ship
    }

    #[test]
    fn a_ship_breaking_off_thrusts_every_tick_at_1_5_times_its_acceleration() {
        // FAST thrusts 900 / 5000 = 0.18 a tick; facing 135 off its break
        // heading, it turns 3 toward it and thrusts at 0.27 all the same.
        let mut npc = fighter(AT_PLAYER, 0.0);
        npc.pursuit.kept = Some(Kept::BreakOff { heading: 135 });
        let ship = flown(&npc, ABOVE);
        assert_eq!(ship.state.heading, 3.0);
        assert!(
            (ship.state.velocity.length() - 0.27).abs() < 1e-6,
            "{:?}",
            ship.state
        );
    }

    #[test]
    fn a_rushing_ship_thrusts_at_2_75_times_its_acceleration_up_to_1_8_times_its_top_speed() {
        let mut npc = burning(fighter(AT_PLAYER, 0.0));
        npc.pursuit.kept = Some(Kept::Rush);
        npc.state.velocity = Vec2::new(0.0, -6.0);
        // 2000 above, so it stays outside the box; a roll of 99 keeps it.
        let off = Vec2::new(0.0, -2000.0);
        let ship = flown(&npc, off);
        // 2.75 × 0.18 = 0.495 a tick.
        assert!(
            (ship.state.velocity.length() - 6.495).abs() < 1e-5,
            "{:?}",
            ship.state
        );
        // Each push lands while it is short of 1.8 × 6 = 10.8, the tenth
        // carrying it to 10.95, past it, and then no more: `_AdjustedAccel`
        // stops adding at the cap but clamps nothing.
        let mut ship = npc;
        for _ in 0..12 {
            let relative = off - (ship.state.position - AT);
            ship = flown(&ship, relative);
        }
        assert!(
            (ship.state.velocity.length() - 10.95).abs() < 1e-4,
            "{:?}",
            ship.state
        );
    }

    #[test]
    fn a_fleeing_ship_that_rushes_flies_the_rush_at_its_attacker() {
        // The frame's last body wins: it turns toward its attacker, not
        // away, and thrusts at the rush's 2.75 × 0.18 = 0.495.
        let npc = burning(fighter(Goal::Flee(ShipRef::Player), 10.0));
        let ship = flown(&npc, ABOVE);
        assert_eq!(ship.state.heading, 7.0, "toward the bearing of 0");
        assert!(
            (ship.state.velocity.length() - 0.495).abs() < 1e-6,
            "{:?}",
            ship.state
        );
    }
}
