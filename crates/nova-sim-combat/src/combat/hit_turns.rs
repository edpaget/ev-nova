//! The port through which a fight's phase has each direct hit answered
//! before the next lands (rdm play-feel-9 phase 10): the phase runs in
//! stretches, each stopping just after a direct hit, and its keeper settles
//! and answers that stretch's hits and lends the ships afresh before the
//! next ([`HitTurns`], [`run_in_turn`]).
//!
//! # The original
//!
//! `_DamageShip`'s prologue (@0x3a807-0x3a9a3) reads the attacker's and
//! its leader's records live at every call, through the ship table
//! (`*0x3b5138 + slot * 0xc940`, @0x3a8cb-0x3a8e5), with no copy taken at
//! the shot's or beam's spawn. For a real hit (argument +0x20, @0x3a895) by
//! an NPC (@0x3a89e-0x3a8c1) it tests the attacker's target (+0x70) against
//! the victim's slot (@0x3a95b), then `_AIShipIsPlundering` of the
//! attacker (goal 0xd, @0x3a964, @0x89847), or else of its NPC leader
//! (+0x9a in 1-63, @0x3a97a-0x3a99a), the leader's own target not read.
//! Either sets the can't-kill byte -0x1b2 (@0x3a9a3), which the caller's
//! disable-only argument seeds (+0x28, @0x3a846) and the shield and armour
//! damage reads (@0x3aba9, @0x3ac3a) ([`plunder`](super::plunder)).
//!
//! The reaction block of an earlier hit in the same loop writes both: the
//! attacker's +0x70 (@0x3bcf6, @0x3bd79, @0x3be5d; `_AIMakeShipAttackPlayer`
//! @0x89d4d; `_DoGoodSamaritan` @0x82b97) and its goal, 0xd to 4
//! (@0x89d3e, @0x82b82). `_HandleShipHit` calls `_DamageShip` for the
//! direct hit (@0x36c7a), whose reaction block finishes inside that call;
//! only then come its blast's `_DamageShip` calls (@0x36ec0) and its
//! sub-munitions (`_SpawnShotSubmunitions` @0x36f47). So every hit after
//! a direct hit, its own blast included, reads what the direct hit's
//! reaction wrote. The prologue plays no sound.
//!
//! @0x3a8e7-0x3a954 is the shareware nag ship's damage scaling, which the
//! engine has no nag ship for (recorded with `nova_sim_ai`'s `answer_flag`
//! module).
//!
//! # The port
//!
//! The answer needs the traffic, which the fight's [`Fighter`]s borrow
//! from for the whole of a lend, so it can't run inside one. Instead a
//! phase that lands direct hits runs in stretches ([`Stretch`]): each runs
//! on the fight and its ships as its keeper lends them, and stops just
//! after a direct hit lands ([`Stretch::Answer`]), its walk kept on the
//! [`Combat`] to resume, or runs to the phase's end ([`Stretch::Done`]).
//! Between two stretches the keeper:
//!
//! 1. settles and answers the hits the stretch landed, in the order they
//!    landed ([`Combat::take_landed`], [`HitTurns::answer`]), so the
//!    next lend's fight holds none of them;
//! 2. lends the same ships, in the same order, afresh from wherever it
//!    keeps them ([`HitTurns::lend`]): what the stretch wrote through the
//!    borrows kept, and what the answers wrote read.
//!
//! `check_contract` states both, behind the `fixture` feature. Every
//! field a reaction writes and a [`Fighter`] carries is read afresh at the
//! next stretch: the target, attacking, plundering, docked, the threat to
//! the player, mining, the cloak and the leader and fleet.
//!
//! **Where the phases stop.** The collision passes
//! (`Combat::test_hits_in_turn`) stop after each direct hit of the contact
//! walk and of the proximity pass, before its blast and its sub-munitions;
//! the asteroid passes, point defence's layer and the hidden shots' turns
//! land only blasts, which are never answered, and run through.
//! `_HandleBeams` (`Combat::close_tick_in_turn`) stops after each beam's
//! hit. `_HandleShot` doesn't stop: its hits are missile locks, which
//! can't kill, and end-of-life blasts, which are no real hit (its other
//! reads are rdm:phase/play-feel-9/phase-11-fight-reads-targets-in-turn).
//!
//! *Ordering against the original.* A hit that is never answered (a blast,
//! a shockwave) settles at the next stop or at the phase's end, where
//! `_DamageShip` settles it as it lands. No answer runs between them, and
//! both come before the next answer and the phase's end, so no boundary
//! is crossed.

use super::phases::Arena;
use super::{Combat, Fighter};
use crate::chance::Chance;

/// How a stretch of a fight's phase ended.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stretch {
    /// It stopped just after a direct hit landed: the hits so far are to be
    /// settled and answered, and the phase resumed among the ships lent
    /// afresh.
    Answer,
    /// It ran to the phase's end.
    Done,
}

/// One stretch of a fight's phase, run on the fight and its ships as they
/// are lent, with the phase's arena and chance.
pub type StretchFn<'a> =
    dyn FnMut(&mut Combat, &mut [Fighter<'_>], Arena<'_>, &mut dyn Chance) -> Stretch + 'a;

/// Who keeps a fight's ships between the stretches of a phase: it lends
/// them to each stretch, and settles and answers the hits each landed
/// (see the module docs).
pub trait HitTurns {
    /// Runs `stretch` on the fight and its ships as they stand now, lent
    /// afresh, so they read what the answers so far wrote; gives what
    /// `stretch` gave.
    fn lend(&mut self, stretch: &mut StretchFn<'_>) -> Stretch;

    /// Settles and answers the hits the last stretch landed
    /// ([`Combat::take_landed`]), in the order they landed.
    fn answer(&mut self);
}

/// Runs `stretch` through `turns` until a stretch runs to the phase's end,
/// the hits of each that stopped answered before the next. The last
/// stretch's hits are the caller's to settle, with the phase's own tail.
pub fn run_in_turn(
    turns: &mut (impl HitTurns + ?Sized),
    mut stretch: impl FnMut(&mut Combat, &mut [Fighter<'_>], Arena<'_>, &mut dyn Chance) -> Stretch,
) {
    while turns.lend(&mut stretch) == Stretch::Answer {
        turns.answer();
    }
}

/// Checks the port's contract against `turns`:
///
/// 1. [`HitTurns::lend`] gives what its stretch gave.
/// 2. After [`HitTurns::answer`], the next lend's fight holds no landed
///    hit ([`Combat::take_landed`]): a strike pushed in one stretch
///    ([`Combat::push_strike`]) is gone by the next.
/// 3. The next lend, after an answer, lends the same ships in the same
///    order, keeping what the last stretch wrote through the borrows (a
///    shield point taken off the first ship, given back after).
///
/// The strike it pushes is a blast's on the first ship lent, of no damage
/// and by no ship, which settles to nothing.
///
/// It runs against this crate's fake keeper and against the session's
/// (`nova_sim::Session`'s `hit_turns` module).
///
/// # Errors
///
/// A message naming the first clause `turns` breaks.
#[cfg(any(test, feature = "fixture"))]
pub fn check_contract(turns: &mut dyn HitTurns) -> Result<(), String> {
    use super::{Nominal, ShipRef, Strike};
    for want in [Stretch::Answer, Stretch::Done] {
        let got = turns.lend(&mut |_, _, _, _| want);
        if got != want {
            return Err(format!(
                "lend gave {got:?} for a stretch that gave {want:?}"
            ));
        }
        if got == Stretch::Answer {
            turns.answer();
        }
    }
    let _ = turns.lend(&mut |combat, fighters, _, _| {
        let ship = fighters
            .first()
            .map_or(ShipRef::Player, |fighter| fighter.ship);
        combat.push_strike(Strike {
            ship,
            by: None,
            damage: 0.0,
            downed: None,
            direct: false,
            intended: false,
            lock: false,
            nominal: Nominal::default(),
            weight: 0.0,
            answered: false,
        });
        Stretch::Answer
    });
    turns.answer();
    let mut left = 0;
    let _ = turns.lend(&mut |combat, _, _, _| {
        left = combat.take_landed().0.len();
        Stretch::Done
    });
    if left != 0 {
        return Err(format!("{left} strike(s) left after the answer"));
    }
    let mut before: Vec<(ShipRef, f32)> = Vec::new();
    let _ = turns.lend(&mut |_, fighters, _, _| {
        before = fighters
            .iter()
            .map(|fighter| (fighter.ship, fighter.reserves.shield.now))
            .collect();
        if let Some(first) = fighters.first_mut() {
            first.reserves.shield.now -= 1.0;
        }
        Stretch::Answer
    });
    turns.answer();
    let mut after: Vec<(ShipRef, f32)> = Vec::new();
    let _ = turns.lend(&mut |_, fighters, _, _| {
        after = fighters
            .iter()
            .map(|fighter| (fighter.ship, fighter.reserves.shield.now))
            .collect();
        if let Some(first) = fighters.first_mut() {
            first.reserves.shield.now += 1.0;
        }
        Stretch::Done
    });
    let ships = |lent: &[(ShipRef, f32)]| lent.iter().map(|&(ship, _)| ship).collect::<Vec<_>>();
    if ships(&before) != ships(&after) {
        return Err(format!(
            "the next lend lent {:?}, not {:?}",
            ships(&after),
            ships(&before)
        ));
    }
    let kept = before
        .iter()
        .zip(&after)
        .enumerate()
        .all(|(index, (&(_, was), &(_, now)))| {
            let wrote = if index == 0 { 1.0 } else { 0.0 };
            (was - wrote - now).abs() < f32::EPSILON
        });
    if !kept {
        return Err(format!(
            "the next lend's shields {after:?} lost what the last stretch wrote to {before:?}"
        ));
    }
    Ok(())
}
