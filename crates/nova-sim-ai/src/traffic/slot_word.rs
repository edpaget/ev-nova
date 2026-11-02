//! The word an NPC's low level leaves `_HandleShip` at the ship's slot of
//! `_AIDispatch`'s loop ([`SlotWord`]), and `_HandleShip`'s flight of it at
//! the ship's step ([`handle_ship`]), on both of double time's passes (in
//! the `EV Nova` executable).
//!
//! **The word.** `_LowLevelAIHandler` runs at each ship's slot (the tail
//! jump @0x901f4) and leaves three words: the heading it wants (+0x68),
//! its thrust (+0x30) and its cap (+0x34). Its head zeroes +0x30
//! (@0x851f1) and +0xba (@0x851f6), writes +0x34 from `_ShipMaxSpeed` at
//! the slot unless it is negative (@0x851ec-0x8520d), gates the subs on
//! `_IsDisabled` (@0x85216), sets +0x68 to the ship's heading truncated
//! (@0x85228-0x8522d), and runs sub 0's `_AIEscortFireUnprovoked`
//! (@0x85236). Then each sub writes its own, each thrust behind its
//! window (the heading's difference against `_ShipTurnRate` at the slot
//! plus a margin):
//!
//! - subs 0x12 (@0x8533d, window @0x852ff), 1 (@0x853fe, @0x8551e,
//!   @0x85552), 0xe (@0x87252), 0x13 (@0x881c4) and 0x15 (@0x88583):
//!   +0x30 = `_ShipAccelRate`, keeping the head's +0x34 ([`Cap::Slot`]);
//! - sub 2 within 500 of its stellar on both axes (@0x85735-0x85773):
//!   +0x30 = `_ShipAccelRate` (@0x85778-0x8577d), +0x34 = 0.25 ×
//!   `_ShipMaxSpeed` at the slot (@0x85783-0x8579e); farther, +0x34 = 0
//!   (@0x857b3, [`Cap::Step`]), both behind the window `_ShipTurnRate` + 5
//!   (@0x856bc-0x856ea);
//! - sub 3: +0x34 = 0 (@0x857d9) and +0x30 = `_ShipAccelRate`
//!   (@0x8584d-0x85858), window @0x85817;
//! - sub 5 (@0x85d52-0x85d59, window @0x85d14), sub 7 (@0x86957-0x8695e)
//!   and sub 8 (@0x87ef8-0x87eff, window @0x87ea9-0x87ee4): +0x34 = 0,
//!   +0x30 = `_ShipAccelRate`;
//! - sub 6: +0x34 = 0 (@0x85f7b), +0x30 = `_ShipAccelRate` (@0x85f82),
//!   then its speed match writes the matched speed to +0x34 (@0x860a6);
//! - sub 0x10: +0x34 = 0 (@0x8645a), +0x30 = 1.5 × `_ShipAccelRate`
//!   (@0x86440-0x86474);
//! - sub 0x11: +0x30 = 2.75 × `_ShipAccelRate` (@0x86594-0x865b2), +0x34
//!   = 1.8 × `_ShipMaxSpeed` at the slot (@0x865ba-0x865e3);
//! - sub 0xc's counter-thrust: +0x34 = 0 (@0x879c1) and +0x30 = 0.66 ×
//!   `_ShipAccelRate` (@0x87a76-0x87a98), window @0x87a40-0x87a6e;
//! - sub 0xe's inertialess brake: +0x30 = −`_ShipAccelRate`
//!   (@0x87121-0x87137), which `_HandleShip` takes off the speed
//!   ([`SlotWord`]'s `speed_off`);
//! - sub 4: +0x30 = 0, +0x34 = −1.0 (@0x85c4c-0x85c53); sub 0xf's
//!   inertialess match: +0x30 = +0x34 = 0 (@0x86ac0-0x86acd); sub 0x14
//!   writes no +0x30 but reads `_ShipAccelRate` for its ease (@0x881db);
//! - the escape pod, in `_AIDispatch` itself (@0x8fb62-0x8fbba): +0x30 =
//!   `_ShipAccelRate` (@0x8fb72), +0x68 = trunc(+0x44) (@0x8fb87), +0x34
//!   = `_ShipMaxSpeed` at the slot (@0x8fb90), with no low level after it.
//!
//! So every read the low level makes is at the slot: `_ShipTurnRate` (24
//! calls inside `_LowLevelAIHandler`, @0x852ff, @0x85492, @0x856bc,
//! @0x85817, @0x85b6c, @0x85d14, @0x85f3d, @0x85fa6, @0x86539, @0x866b4,
//! @0x8671a, @0x868b0, @0x8691c, @0x86bc4, @0x87210, @0x872d0, @0x875ab,
//! @0x878a6, @0x87a40, @0x87eb6, @0x88186, @0x88407, @0x88545, @0x886b9),
//! `_ShipAccelRate` (26) and `_ShipMaxSpeed` (7): each window, each
//! thrust, a sub's multiple of either, and every plan that reads them
//! (stopping distances, the formation's nudge, sub 0x14's ease). The
//! engine's autopilot works it out there too
//! ([`autopilot::low_level`](super::autopilot::low_level)), as the
//! docking's low level does
//! ([`plunder_dock::low_level`](crate::traffic::plunder_dock::low_level)),
//! and its
//! direct writes (a sniper's damping, the formation's nudge and glow copy,
//! a miner's ease, nudge and brake, a docking fighter's pull) are made
//! there, so a higher slot's think reads them that tick and a lower one's
//! as the tick began. Each read of the ship's own handling reads its
//! charge as it stands at the slot: `_HandleShot`'s `_IonizeShip`
//! (@0x3612d) runs in the shot loop (@0x44385) between the slot and the
//! step, and the step's own ion block (@0x342b2-0x34421) sheds it before
//! the turn.
//!
//! **Who writes the word between the slot and the step.** Nothing the
//! shot loop (@0x44385), `_HandleBeams` (@0x4453d) or a dialog runs writes
//! +0x30, +0x34 or +0x68 of a ship in flight. `_HandleShip` itself does:
//! its warp-out writes +0x68 (@0x347ba, a led ship takes its leader's),
//! its burst writes +0x34 (@0x34c63) and its end +0x34 again (@0x34d59),
//! and `_AIMakeShipStopJumping` (called @0x34d32) zeroes +0x50, +0x30 and
//! +0x34 (@0x82eb0-0x82eb6), so a burst that ends on the advance pass
//! thrusts nothing on the second ([`SlotWord::stop_jumping`]). Every spawn
//! zeroes +0x30 and +0x34 (`_GenericRandomShipSpawn` @0x3cb54-0x3cb5a,
//! `_RandomShipSpawn` @0x3c639-0x3c643, `_RespawnEscort` @0x3d4ee-0x3d4f1)
//! and none writes +0x68. `_FlipShip`'s stores at those offsets
//! (@0xa6425, @0xa64a9) are to a sprite's record, not a ship's.
//!
//! **Kept per ship** (`Traffic::words`): the word is written at each slot
//! the AI loop reaches and kept until the next, so a ship the loop doesn't
//! reach flies its last word, as the original's words stand. No pass
//! clears it: double time's second pass runs no `_AIDispatch` (`je
//! 0x44350` @0x4402a), so it flies the advance pass's word, its heading
//! wanted, thrust and cap, and decides nothing. It is cleared as the
//! system is entered or left, and never saved.
//!
//! **A ship with no word since its spawn** ([`SlotWord::spawned`]) thrusts
//! nothing and turns toward the +0x68 its berth's last ship left: a fighter
//! launched below its carrier's slot (`_LaunchFighter` through
//! `_GenericRandomShipSpawn` @0x3d657), one launched into a later slot of
//! the ship loop (an escape ship's @0x354ea), and any ship spawned after
//! its slot's pass on double time's advance pass, flown on the second.
//! `_InitObjects` writes +0x68 = 0 for each of the 64 slots as the game
//! starts (@0x1c315, the loop @0x1c58a), so a berth never used turns
//! toward 0. Arrivals (`_EnterMoreShips` @0x44237) come before the AI loop
//! and are dispatched. The ships `_SetupShipsInSystem` sets up
//! (`_HandlePlayer` @0x69114) are dispatched in the same frame's AI loop
//! (@0x44270-0x442ae) before the ship loop; the engine's populating pass
//! flies none of them (`nova_sim::Session::ship_loop`'s `Populated`), so
//! each flies first after its first low level, as there. Which heading a
//! new ship gets is
//! [`RuleKey::ReusedSlotHeading`](crate::RuleKey::ReusedSlotHeading)'s:
//! the berth's last by the engine, its own whole heading, so no turn, by
//! the other reading.
//!
//! **The step** ([`handle_ship`]): while its AI wait runs (but a count
//! wait) `_HandleShip` skips the turn and thrust (@0x3443f-0x34454,
//! @0x34a54-0x34a72) and only moves it. Otherwise it turns to +0x68 by
//! `_ShipTurnRate` at its step (@0x344b0), after its ion block, and
//! thrusts only if +0x30 isn't 0 (@0x34a7b-0x34a84), by the +0x30 its low
//! level wrote. Its cap is +0x34 as the low level left it ([`Cap`]): where
//! it is 0, `_ShipMaxSpeed` at the step (@0x34a8a-0x34aab), which the
//! binary reads on each pass. An inertialess ship's velocity is pulled
//! toward its heading by `_AdjustInertialessShipVelocity`
//! (@0x3349c-0x33580, called @0x3425f), which reads `_Accel`
//! (@0x334d9) and its own `_ShipAccelRate` (@0x334e1) at the step, before
//! the ion block, apart from the word's +0x30 ([`AtStep::adjust_accel`]).
//! `_ShipIsInertialess` (@0x79c2) reads the hull's flags, +0x86, +0x9a and
//! the sub-goal through `_IsShipInCloseForm` (+0xc8ca == 0xc, @0x82f5a),
//! none of which changes between the slot and the step, so the word
//! carries its inertia from the slot. The glow's rush test (@0x34f57)
//! compares +0x30 with `_ShipAccelRate` at the step
//! ([`ship_display::record`](super::ship_display::record)).
//!
//! None of this plays a sound: neither the head, the subs' word writes,
//! `_HandleShip`'s turn and thrust nor `_AdjustInertialessShipVelocity`
//! calls `PlaySound` or `_beepSnd`. The low level's one `_ST_PlaySound` is
//! the repair's `snd ` 151 ([`dispatch_dock`](super::dispatch_dock)).

use crate::flight::{self, Controls, FlightRules, Turn};
use crate::handling::Handling;
use crate::inertialess::{REVERSE, step_speed};
use crate::rulebook::RuleSource;
use crate::speed_mult::SpeedMult;
use crate::traffic::berth_kept::BerthKept;
use crate::traffic::held_burst::{self, Flight, Word};
#[cfg(doc)]
use crate::traffic::inertialess;
use crate::traffic::inertialess::Inertia;
use crate::traffic::npc::Npc;
use crate::trig;

/// What a ship's low level leaves `_HandleShip` at its slot (see the
/// module docs): the heading it wants (+0x68), its thrust and its cap
/// (+0x30, +0x34), with the inertia it flies them by. `_HandleShip` flies
/// it on both of double time's passes ([`handle_ship`]), turning by its
/// turn rate at its own step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SlotWord {
    /// The heading it wants, +0x68; none keeps its heading as it is, as
    /// the port's earlier formation, which takes its leader's, flies it.
    wanted: Option<i16>,
    /// The controls but the turn: the thrust, and an inertialess ship's
    /// half brake.
    controls: Controls,
    /// The thrust (+0x30's magnitude, a sub's multiple of
    /// `_ShipAccelRate` included) and the top speed (+0x34) the low level
    /// read at its slot. Its turn rate is unread: `_HandleShip` reads its
    /// own.
    flown: Handling,
    /// Where the thrust's top speed is read.
    cap: Cap,
    /// How it flies.
    inertia: Inertia,
    /// What it does to +0x34 (see [`held_burst`]).
    word: Word,
    /// Sub 0xe's inertialess brake: +0x30 = −`_ShipAccelRate`, which
    /// `_HandleShip` takes off its speed (@0x34aef-0x34b68) where it bursts
    /// nothing.
    speed_off: bool,
}

/// Where a word's thrust reads its top speed, as its low level left +0x34
/// (`_HandleShip` @0x34a8a-0x34aab).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cap {
    /// +0x34 above none: the top speed its low level read at its slot,
    /// the head's `_ShipMaxSpeed` (@0x85200-0x8520d) or a sub's multiple
    /// of it (sub 2's quarter near its stellar @0x8579e, sub 0x11's 1.8
    /// @0x865e3, closing's half @0x8765d-0x8767b, a dogfight's match
    /// @0x860a6).
    Slot,
    /// +0x34 none: `_ShipMaxSpeed` at `_HandleShip`'s step, which reads
    /// the hold as it stands then and no charge. Subs 2 (far), 3, 5, 6, 7,
    /// 8, 9, 0xb (outside its box), 0xc's counter-thrust, 0xf's match and
    /// 0x10 write it, and so do the engine's own plans.
    Step,
}

/// What `_HandleShip` reads of a ship at its own step: its turn rate
/// (`_ShipTurnRate`, @0x344b0, @0x34510, @0x34554) after the frame's ion
/// block, its top speed (`_ShipMaxSpeed`, @0x34aab), the
/// `_ShipAccelRate` its inertialess adjustment reads before the ion block
/// (`_AdjustInertialessShipVelocity` @0x334e1), and the frame's gain,
/// `_gSpeedMult`, which its move, turn and thrust read (@0x3427e,
/// @0x344b8, @0x34af9, @0x34b3b, @0x34b85).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AtStep {
    /// Its turn rate.
    pub turn_rate: f32,
    /// Its top speed.
    pub top_speed: f32,
    /// The acceleration its inertialess adjustment pulls by.
    pub adjust_accel: f32,
    /// The frame's gain.
    pub gain: SpeedMult,
}

impl AtStep {
    /// What `_HandleShip` reads of `npc` at its step under `flight`, the
    /// ships the player leads at `led_speed`, its adjustment's
    /// acceleration `adjust_accel` read before the frame's ion block, at
    /// the frame's `gain`: its turn rate after the ion block
    /// ([`Npc::flying_handling`], whose top speed it doesn't read) and its
    /// `_ShipMaxSpeed` with the hold as it stands then and no charge
    /// ([`Npc::top_speed`]).
    #[must_use]
    pub fn of_npc(
        npc: &Npc,
        led_speed: f32,
        flight: FlightRules,
        adjust_accel: f32,
        gain: SpeedMult,
    ) -> Self {
        let top_speed = npc.top_speed(led_speed);
        Self {
            turn_rate: npc.flying_handling(top_speed, flight).turn_rate,
            top_speed,
            adjust_accel,
            gain,
        }
    }

    /// The turn rate, top speed and acceleration of `handling`, at a full
    /// frame's gain: a step that reads what its slot read, as the
    /// modules' tests compose a low level and its step with nothing
    /// between (`autopilot::fly`, `fly_at_once`). A flight of the
    /// traffic's reads the frame's gain ([`AtStep::of_npc`]).
    #[must_use]
    pub fn of(handling: &Handling) -> Self {
        Self {
            turn_rate: handling.turn_rate,
            top_speed: handling.max_speed,
            adjust_accel: handling.accel,
            gain: SpeedMult::FULL,
        }
    }

    /// This step at the frame's `gain`.
    #[must_use]
    pub fn at_gain(self, gain: SpeedMult) -> Self {
        Self { gain, ..self }
    }

    /// The handling a step that thrusts nothing flies by: its turn rate,
    /// its top speed, and the acceleration its inertialess adjustment
    /// reads ([`inertialess::drift`]).
    #[must_use]
    pub fn unthrust(self) -> Handling {
        Handling {
            max_speed: self.top_speed,
            accel: self.adjust_accel,
            turn_rate: self.turn_rate,
        }
    }
}

impl SlotWord {
    /// The word of a ship whose low level set no heading and no thrust,
    /// the head's (see the module docs): its own whole heading
    /// (@0x8522d), no thrust (@0x851f1), +0x34 its slot's top speed
    /// (@0x851ec-0x8520d), on `handling`, as `inertia` says.
    #[must_use]
    pub fn none(npc: &Npc, handling: &Handling, inertia: Inertia) -> Self {
        Self::of(
            Flight::writes(
                Controls::default(),
                trig::whole(npc.state.heading),
                Cap::Slot,
            ),
            handling,
            inertia,
        )
    }

    /// The word a spawn leaves `npc` until its first low level, on
    /// `handling` under `flight` (see the module docs): no thrust, +0x34
    /// none, and the heading `wanted`, its berth's last, or with none its
    /// own whole heading, as
    /// [`RuleKey::ReusedSlotHeading`](crate::RuleKey::ReusedSlotHeading)'s
    /// reading says ([`spawn_heading`]).
    #[must_use]
    pub fn spawned(
        npc: &Npc,
        wanted: Option<i16>,
        handling: &Handling,
        flight: FlightRules,
    ) -> Self {
        let wanted = wanted.unwrap_or_else(|| trig::whole(npc.state.heading));
        let inertia = Inertia::of(npc, None, true, flight);
        Self::of(
            Flight::writes(Controls::default(), wanted, Cap::Step),
            handling,
            inertia,
        )
    }

    /// The word `flight`, a low level's, leaves on `handling`, the ship's
    /// own at its slot, as `inertia` says.
    #[must_use]
    pub(crate) fn of(flight: Flight, handling: &Handling, inertia: Inertia) -> Self {
        Self {
            wanted: flight.wanted,
            controls: flight.controls,
            flown: flight.flown.unwrap_or(*handling),
            cap: flight.cap,
            inertia,
            word: flight.word,
            speed_off: flight.speed_off,
        }
    }

    /// The heading it wants (+0x68), none to keep its heading.
    #[must_use]
    pub fn wanted(&self) -> Option<i16> {
        self.wanted
    }

    /// Whether it thrusts (+0x30 not 0), or brakes an inertialess speed.
    #[must_use]
    pub fn thrusts(&self) -> bool {
        self.controls.thrust || self.controls.reverse || self.speed_off
    }

    /// Where its thrust reads its top speed.
    #[must_use]
    pub fn cap(&self) -> Cap {
        self.cap
    }

    /// The handling its low level read at its slot: its thrust and its
    /// top speed.
    #[must_use]
    pub fn flown(&self) -> Handling {
        self.flown
    }

    /// How it flies.
    #[must_use]
    pub fn inertia(&self) -> Inertia {
        self.inertia
    }

    /// `_AIMakeShipStopJumping`'s writes (@0x82eb0-0x82eb6): +0x30 = +0x34 =
    /// 0, so the word thrusts nothing until the next low level writes it.
    pub fn stop_jumping(&mut self) {
        self.controls = Controls::default();
        self.speed_off = false;
        self.cap = Cap::Step;
        self.word = Word::Keeps { thrust: 0.0 };
    }
}

/// The heading a ship new to `berth` wants until its first low level, by
/// `headings`' reading of
/// [`RuleKey::ReusedSlotHeading`](crate::RuleKey::ReusedSlotHeading): by
/// the engine the +0x68 its last ship left, 0 for a berth no ship has held
/// since the game began (`_InitObjects` @0x1c315); none by the other, its
/// own whole heading.
#[must_use]
pub fn spawn_heading(headings: &BerthKept<i16>, berth: u32) -> Option<i16> {
    match headings.reading() {
        RuleSource::Engine => Some(headings.left(berth).unwrap_or(0)),
        RuleSource::Bible => None,
    }
}

/// `_HandleShip`'s turn and thrust of `npc` by the `word` its low level
/// left, with what it reads `at_step`, under `flight` (see the module
/// docs): while its AI wait runs (but a count wait) it only moves on at
/// its velocity (inertialess, by its speed along its heading), as
/// `_HandleShip` skips its turn and thrust (@0x3443f-0x34454,
/// @0x34a54-0x34a72); otherwise it turns to the heading wanted
/// ([`flight::turn_to`]) by its turn rate at its step, and thrusts as the
/// low level said, by the thrust its low level read at its slot, capped
/// as the low level left the cap ([`Cap`]), an inertialess ship's
/// velocity pulled by its adjustment's own acceleration at its step.
/// Each moves, turns and thrusts at the frame's gain ([`AtStep::gain`]).
pub fn handle_ship(npc: &mut Npc, word: &SlotWord, at_step: AtStep, flight: FlightRules) {
    if npc.ai_wait > 0 && !npc.count_wait() {
        let unthrust = at_step.unthrust();
        super::autopilot::position_step(npc, &unthrust, word.inertia, at_step.gain);
        return;
    }
    let turn = word.wanted.map_or(Turn::None, |wanted| {
        flight::turn_to(&mut npc.state, wanted, at_step.turn_rate, at_step.gain)
    });
    fly_word(npc, word, turn, at_step, flight);
}

/// `_HandleShip`'s flight of `word` after its turn `turn` (see
/// [`handle_ship`]): sub 0xe's inertialess brake, where it bursts nothing,
/// then the thrust, capped as the word says.
pub(crate) fn fly_word(
    npc: &mut Npc,
    word: &SlotWord,
    turn: Turn,
    at_step: AtStep,
    flight: FlightRules,
) {
    let handling = Handling {
        turn_rate: at_step.turn_rate,
        max_speed: match word.cap {
            Cap::Slot => word.flown.max_speed,
            Cap::Step => at_step.top_speed,
        },
        ..word.flown
    };
    if word.speed_off && npc.owed_burst.is_none() {
        // +0x30 = −`_ShipAccelRate`, capped at its top speed and floored at
        // none, × the frame's gain (@0x34aef-0x34b68, read @0x34af9).
        npc.speed = step_speed(
            npc.speed,
            REVERSE,
            handling.accel,
            handling.max_speed,
            at_step.gain,
        );
    }
    let controls = Controls {
        turn,
        ..word.controls
    };
    let by = (word.inertia, word.word, at_step.adjust_accel);
    held_burst::step(npc, &handling, controls, flight, by, at_step.gain);
}

/// `flight`, a low level's on `handling` as `inertia` says, flown at once
/// by `_HandleShip` on the same handling ([`handle_ship`]): a module's
/// tests of a low level with no slot and step apart.
#[cfg(test)]
pub(crate) fn fly_at_once(
    npc: &mut Npc,
    flight: Flight,
    handling: &Handling,
    rules: FlightRules,
    inertia: Inertia,
) {
    let word = SlotWord::of(flight, handling, inertia);
    handle_ship(npc, &word, AtStep::of(handling), rules);
}

/// `low`, a low level's flight or how it ended, flown at once as
/// [`fly_at_once`] flies it: what became of the ship.
#[cfg(test)]
pub(crate) fn fly_low(
    npc: &mut Npc,
    low: Result<Flight, crate::traffic::autopilot::Outcome>,
    handling: &Handling,
    rules: FlightRules,
    inertia: Inertia,
) -> crate::traffic::autopilot::Outcome {
    match low {
        Ok(flight) => {
            fly_at_once(npc, flight, handling, rules, inertia);
            crate::traffic::autopilot::Outcome::Flying
        }
        Err(outcome) => outcome,
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::flight::ShipState;
    use crate::geometry::Vec2;
    use crate::stats::ShipStats;
    use crate::traffic::fixture::npc;
    use nova_sim_base::fixture::FAST;

    /// Top speed 4, thrust 0.5, 3 degrees a tick.
    const HANDLING: Handling = Handling {
        max_speed: 4.0,
        accel: 0.5,
        turn_rate: 3.0,
    };

    /// A ship of [`FAST`] at the centre moving `vy` down the screen,
    /// heading `heading`.
    fn ship(vy: f32, heading: f32) -> Npc {
        let mut ship = npc(1, ShipStats::new(FAST, &[]));
        ship.state = ShipState {
            position: Vec2::ZERO,
            velocity: Vec2::new(0.0, vy),
            heading,
        };
        ship
    }

    /// A word wanting `wanted`, thrusting as `thrust` says, capped as
    /// `cap` says, on [`HANDLING`].
    fn word(wanted: i16, thrust: bool, cap: Cap) -> SlotWord {
        let controls = Controls {
            thrust,
            ..Controls::default()
        };
        SlotWord::of(
            Flight::writes(controls, wanted, cap),
            &HANDLING,
            Inertia::Inertial,
        )
    }

    #[test]
    fn handle_ship_thrusts_nothing_on_a_word_whose_thrust_is_none() {
        // `_HandleShip` thrusts only if +0x30 isn't 0 (@0x34a84); it turns
        // to +0x68 either way.
        let mut flown = ship(1.0, 0.0);
        handle_ship(
            &mut flown,
            &word(90, false, Cap::Slot),
            AtStep::of(&HANDLING),
            FlightRules::default(),
        );
        assert_eq!(flown.state.velocity, Vec2::new(0.0, 1.0), "no thrust");
        assert_eq!(flown.state.heading, 3.0, "turned by its turn rate");
        assert!(!word(90, false, Cap::Slot).thrusts());
    }

    #[test]
    fn a_slot_cap_is_the_slots_top_speed_and_a_step_cap_the_steps() {
        // +0x34 above none caps at the slot's `_ShipMaxSpeed`; at none,
        // `_HandleShip` reads its own (@0x34a8a-0x34b03). Moving 3.8 up
        // the screen, below the slot's 4 it gains its whole thrust, and
        // past the step's 3 none.
        let thrust = |cap: Cap| {
            let mut flown = ship(-3.8, 0.0);
            let at_step = AtStep {
                top_speed: 3.0,
                ..AtStep::of(&HANDLING)
            };
            handle_ship(
                &mut flown,
                &word(0, true, cap),
                at_step,
                FlightRules::default(),
            );
            flown.state.velocity.y
        };
        assert_eq!(thrust(Cap::Slot), -4.3, "below its slot's 4");
        assert_eq!(thrust(Cap::Step), -3.8, "at its step's 3, held");
    }

    #[test]
    fn the_rush_test_reads_the_words_thrust_against_the_steps_accel() {
        // The rush test (@0x34f57) compares +0x30, the slot's, with
        // `_ShipAccelRate` at the step, after a charge to 400 read there:
        // the plain thrust is a rush of the slowed accel.
        use crate::traffic::ship_display::Thrust;
        let mut flown = ship(0.0, 0.0);
        let own = flown.flying_handling(flown.stats.handling.max_speed, FlightRules::default());
        let word = SlotWord::of(
            Flight::writes(
                Controls {
                    thrust: true,
                    ..Controls::default()
                },
                0,
                Cap::Step,
            ),
            &own,
            Inertia::Inertial,
        );
        handle_ship(&mut flown, &word, AtStep::of(&own), FlightRules::default());
        assert_eq!(flown.display.drive.thrust, Thrust::Plain, "uncharged");
        let mut charged = ship(0.0, 0.0);
        charged.stats.ion.class_capacity = 400.0;
        charged.ion.charge = 400.0;
        handle_ship(
            &mut charged,
            &word,
            AtStep::of(&own),
            FlightRules::default(),
        );
        assert_eq!(charged.display.drive.thrust, Thrust::Rush, "charged");
    }

    #[test]
    fn a_word_with_no_heading_keeps_the_ships() {
        let mut flown = ship(0.0, 12.5);
        let word = SlotWord {
            wanted: None,
            ..word(0, false, Cap::Slot)
        };
        handle_ship(
            &mut flown,
            &word,
            AtStep::of(&HANDLING),
            FlightRules::default(),
        );
        assert_eq!(flown.state.heading, 12.5);
        assert_eq!(word.wanted(), None);
    }

    #[test]
    fn in_its_ai_wait_it_moves_on_with_no_turn_or_thrust_but_in_a_count_wait() {
        let mut waiting = ship(1.0, 0.0);
        waiting.ai_wait = 5;
        handle_ship(
            &mut waiting,
            &word(90, true, Cap::Slot),
            AtStep::of(&HANDLING),
            FlightRules::default(),
        );
        assert_eq!(waiting.state.heading, 0.0);
        assert_eq!(waiting.state.velocity, Vec2::new(0.0, 1.0));
        assert_eq!(waiting.state.position, Vec2::new(0.0, 1.0));
        waiting.goal = crate::ai::Goal::CountWait;
        handle_ship(
            &mut waiting,
            &word(90, false, Cap::Slot),
            AtStep::of(&HANDLING),
            FlightRules::default(),
        );
        assert_eq!(waiting.state.heading, 3.0, "a count wait turns");
    }

    #[test]
    fn an_inertialess_ship_in_its_wait_drifts_by_the_steps_adjust_accel() {
        let mut waiting = ship(1.0, 90.0);
        waiting.ai_wait = 5;
        waiting.speed = 2.0;
        let inertialess = SlotWord {
            inertia: Inertia::Inertialess { speed: 2.0 },
            ..word(0, false, Cap::Slot)
        };
        let at_step = AtStep {
            adjust_accel: 0.1,
            ..AtStep::of(&HANDLING)
        };
        handle_ship(&mut waiting, &inertialess, at_step, FlightRules::default());
        // Toward (2, 0) by 0.1 × 4 on each axis.
        assert!((waiting.state.velocity - Vec2::new(0.4, 0.6)).length() < 1e-5);
    }

    #[test]
    fn sub_0xes_inertialess_brake_takes_a_whole_ticks_acceleration_off_its_speed_at_its_step() {
        // +0x30 = −`_ShipAccelRate` (@0x8710f-0x8713a): `_HandleShip` takes
        // a whole tick's acceleration off the speed, capped at its top speed
        // and floored at none (@0x34aef-0x34b68), and it wants its own
        // whole heading.
        let handling = ShipStats::new(FAST, &[]).handling;
        assert_eq!((handling.accel, handling.max_speed), (0.18, 6.0));
        let braked = |speed: f32| {
            let mut sniper = ship(-3.0, 40.6);
            sniper.speed = speed;
            let word = SlotWord::of(
                Flight {
                    speed_off: true,
                    ..Flight::writes(Controls::default(), 40, Cap::Slot).doing(Word::Keeps {
                        thrust: handling.accel,
                    })
                },
                &handling,
                Inertia::Inertialess { speed },
            );
            assert!(word.thrusts());
            handle_ship(
                &mut sniper,
                &word,
                AtStep::of(&handling),
                FlightRules::default(),
            );
            assert_eq!(sniper.state.heading, 40.0, "+0x68's default");
            sniper.speed
        };
        assert!((braked(3.0) - 2.82).abs() < 1e-6, "{}", braked(3.0));
        assert_eq!(braked(0.1), 0.0, "floored");
        assert_eq!(braked(7.0), 6.0, "capped at its top speed");
        // Owing a burst, it bursts instead (@0x34b00-0x34b03).
        let mut owing = ship(-3.0, 40.6);
        owing.speed = 3.0;
        owing.owed_burst = Some(30.0);
        let word = SlotWord::of(
            Flight {
                speed_off: true,
                ..Flight::writes(Controls::default(), 40, Cap::Slot).doing(Word::Keeps {
                    thrust: handling.accel,
                })
            },
            &handling,
            Inertia::Inertialess { speed: 3.0 },
        );
        handle_ship(
            &mut owing,
            &word,
            AtStep::of(&handling),
            FlightRules::default(),
        );
        assert_eq!(owing.speed, 30.0, "+0x48 = |+0x34|");
    }

    #[test]
    fn a_stopped_jump_thrusts_nothing_and_caps_at_its_step() {
        let mut stopped = word(0, true, Cap::Slot);
        stopped.stop_jumping();
        assert!(!stopped.thrusts());
        assert_eq!(stopped.cap(), Cap::Step);
        assert_eq!(stopped.word, Word::Keeps { thrust: 0.0 });
    }

    #[test]
    fn a_spawns_word_turns_toward_its_berths_heading_or_its_own_with_none() {
        let fresh = ship(0.0, 41.7);
        let flight = FlightRules::default();
        let word = SlotWord::spawned(&fresh, Some(90), &HANDLING, flight);
        assert_eq!(word.wanted(), Some(90));
        assert!(!word.thrusts());
        assert_eq!(word.cap(), Cap::Step);
        let own = SlotWord::spawned(&fresh, None, &HANDLING, flight);
        assert_eq!(own.wanted(), Some(41));
    }

    #[test]
    fn a_new_ship_takes_its_berths_last_heading_or_0_by_the_engine_and_none_by_the_other() {
        let mut headings = BerthKept::<i16>::default();
        assert_eq!(spawn_heading(&headings, 4), Some(0), "never used");
        headings.write(4, Some(90));
        assert_eq!(spawn_heading(&headings, 4), Some(90));
        headings.set_reading(RuleSource::Bible);
        assert_eq!(spawn_heading(&headings, 4), None);
    }
}
