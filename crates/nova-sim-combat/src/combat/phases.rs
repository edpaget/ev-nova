//! The fight's tick as phases of `_DoPlayGameWork`, which its session runs
//! between its own work: the player's point defence, the player's
//! trigger, the collision passes, the NPCs' point defence, `_HandleShot`,
//! the NPCs' picks, then `_HandleBeams` and the late steps (see the
//! [`combat`](super) module docs' steps). [`Combat::tick_on`] runs them in
//! that order with nothing between them.

use super::{
    Combat, Field, Fighter, Firers, GamePass, Landed, Rules, Scenery, Sides, Target, lock,
    regenerate, truncate_downed_player,
};
use crate::chance::Chance;
use crate::combat::armament::Arsenal;
use crate::govt::Governments;

/// What a fight's phase is fought by besides its ships: the pass, each
/// sub-munition's weapon, the relations between governments, the rules,
/// and when and where it is fought.
#[derive(Clone, Copy, Debug)]
pub struct Arena<'a> {
    /// Which of the original's game-work passes it is.
    pub pass: GamePass,
    /// Each sub-munition's weapon.
    pub arsenal: &'a Arsenal,
    /// The relations between governments.
    pub govts: &'a Governments,
    /// The rules the fight is fought by.
    pub rules: Rules<'a>,
    /// When and where it is fought.
    pub field: Field,
}

impl Combat {
    /// Step 1's point defence: the player's, as `_HandlePlayer` runs it
    /// (`_HandleShipPointDefense` @0x68c74) before its trigger, drawing on
    /// `chance` (see `Combat::defend`).
    pub fn defend_player(
        &mut self,
        fighters: &mut [Fighter],
        arena: Arena<'_>,
        chance: &mut (impl Chance + ?Sized),
    ) {
        self.field = arena.field;
        self.note_slots(fighters);
        let mut source = &mut *chance;
        let chance: &mut dyn Chance = &mut source;
        let targets: Vec<Target> = fighters.iter().map(Fighter::as_target).collect();
        let defence = (arena.rules.defence, arena.govts);
        self.defend(fighters, &targets, defence, chance, Firers::Player);
    }

    /// Step 1's trigger: the player fires the weapons its trigger holds
    /// that are ready, as `_HandlePlayer` calls `_FirePlayerWeapon`
    /// (@0x6ab55, @0x6abc7), drawing on `chance` (see `Combat::fire`);
    /// then its reloads count down a tick, as `_HandlePlayer` counts them
    /// (@0x6ae25-0x6ae61) after its trigger and before `_HandleShot` holds
    /// them (`Combat::hold_reloads`).
    pub fn fire_player(
        &mut self,
        fighters: &mut [Fighter],
        arena: Arena<'_>,
        chance: &mut (impl Chance + ?Sized),
    ) {
        self.field = arena.field;
        self.note_slots(fighters);
        let mut source = &mut *chance;
        let chance: &mut dyn Chance = &mut source;
        let targets: Vec<Target> = fighters.iter().map(Fighter::as_target).collect();
        self.fire(fighters, &targets, chance, Firers::Player);
        count_down(fighters, Firers::Player);
    }

    /// Steps 2 and 3, the collision passes (`_SWCollideSpriteLayer`
    /// @0x43e70-0x44010, `_CheckShotProximities` @0x44015): the last
    /// tick's moves tested among `fighters` and `scenery`, the hits landed
    /// and the struck ships' conditions settled (see `Combat::hit_pass`).
    pub fn test_hits(
        &mut self,
        fighters: &mut [Fighter],
        arena: Arena<'_>,
        scenery: Scenery<'_>,
        chance: &mut (impl Chance + ?Sized),
    ) {
        self.field = arena.field;
        self.note_slots(fighters);
        let mut source = &mut *chance;
        let chance: &mut dyn Chance = &mut source;
        let terms = (arena.govts, arena.arsenal);
        self.hit_pass(fighters, terms, arena.rules.disable, scenery, chance);
    }

    /// Step 4's point defence: every NPC's, as `_AIDispatch` runs it in
    /// its slot loop (@0x4429d, `_AIFireTurret` @0x80b0d), reading the
    /// conditions the collision passes left, drawing on `chance`. None on
    /// double time's second pass ([`GamePass::Again`]), as `_AIDispatch`
    /// runs on the advance pass alone (@0x4402a).
    ///
    /// The original interleaves each NPC's point defence with its
    /// decision, slot by slot; the session runs every decision first
    /// (`nova_sim::Session::game_work`), and the simulation has no slots.
    /// Both settle before the frame is drawn.
    pub fn defend_npcs(
        &mut self,
        fighters: &mut [Fighter],
        arena: Arena<'_>,
        chance: &mut (impl Chance + ?Sized),
    ) {
        if arena.pass == GamePass::Again {
            return;
        }
        self.field = arena.field;
        self.note_slots(fighters);
        let mut source = &mut *chance;
        let chance: &mut dyn Chance = &mut source;
        let targets: Vec<Target> = fighters.iter().map(Fighter::as_target).collect();
        let defence = (arena.rules.defence, arena.govts);
        self.defend(fighters, &targets, defence, chance, Firers::Npcs);
    }

    /// Step 6's picks, `_HandleShip`'s (@0x44483): each NPC's reloads
    /// count down a tick (@0x33896-0x338ce), after `_HandleShot` has held
    /// them (`Combat::hold_reloads`), then it fires the weapons its trigger
    /// holds that are ready (`_FireAIShipWeapon` @0x34ffd, @0x3500a),
    /// drawing on `chance`.
    pub fn fire_npcs(
        &mut self,
        fighters: &mut [Fighter],
        arena: Arena<'_>,
        chance: &mut (impl Chance + ?Sized),
    ) {
        self.field = arena.field;
        self.note_slots(fighters);
        let mut source = &mut *chance;
        let chance: &mut dyn Chance = &mut source;
        count_down(fighters, Firers::Npcs);
        let targets: Vec<Target> = fighters.iter().map(Fighter::as_target).collect();
        self.fire(fighters, &targets, chance, Firers::Npcs);
    }

    /// Step 5, `_HandleShot` (@0x44385): first its loop visits the slots
    /// in order ([`Combat::visit_slots`]): each held slot is freed, and
    /// each shot its last move spent comes to its end of life (@0x36195), its
    /// blast meeting each NPC where it stood as the last tick ran and the
    /// player where it is, as the collision passes do
    /// (`Combat::hit_pass`); its hits land and each ship struck settles
    /// its condition as in the collision passes. Then the shots fly among
    /// `fighters`, drawing on `chance` ([`Combat::fly`]), all but the
    /// sub-munitions released into a slot the loop had passed; the tick's
    /// missile locks land ([`Strike::lock`](super::Strike::lock),
    /// @0x35855), and each one-at-a-time shot holds its firer's reload
    /// (`Combat::hold_reloads`, @0x35c09).
    pub fn fly_shots(
        &mut self,
        fighters: &mut [Fighter],
        arena: Arena<'_>,
        chance: &mut (impl Chance + ?Sized),
    ) {
        self.field = arena.field;
        self.note_slots(fighters);
        let mut source = &mut *chance;
        let chance: &mut dyn Chance = &mut source;
        let rule = arena.rules.disable;
        let now = self.stand_where_they_stood(fighters);
        let stood: Vec<Target> = fighters.iter().map(Fighter::as_target).collect();
        let sides = Sides {
            targets: &stood,
            govts: arena.govts,
            pass_through: self.pass_through,
        };
        let first_strike = self.strikes.len();
        // Each held slot is freed at its visit, a shot point defence ended,
        // hidden at life 0, with no end of life (@0x36195 → @0x35f0a →
        // @0x3617c): the collision passes have already tested and let it
        // go (`Combat::test_hidden`).
        let visited = self.visit_slots(fighters, &sides, arena.arsenal, chance);
        let expiries = visited.expiries;
        self.land(fighters, expiries, rule);
        for (fighter, position) in fighters.iter_mut().zip(now) {
            fighter.state.position = position;
        }
        self.settle_struck(fighters, first_strike, rule);
        let targets: Vec<Target> = fighters.iter().map(Fighter::as_target).collect();
        self.fly(fighters, &targets, &visited.sitting_out, chance);
        let sides = Sides {
            targets: &targets,
            ..sides
        };
        let locks: Vec<Landed> = self
            .shots
            .iter()
            .filter_map(|shot| lock(shot, &sides))
            .collect();
        self.land(fighters, locks, rule);
        self.hold_reloads(fighters);
    }

    /// `_HandleBeams` (@0x4453d), step 6's beams and the late steps 7 to
    /// 10: the beams follow their firers among `fighters`, drawing on
    /// `chance`, point-defence beams hit their missiles, the beams' hits
    /// land, conditions follow the damage, the shockwaves go off, shields
    /// and armour regenerate, and where each ship stands is kept for the
    /// next tick's collision passes.
    ///
    /// The original starts a breakup's countdown in `_HandleShipDisplay`,
    /// and regenerates in `_HandlePlayer` and `_HandleShip`, before
    /// `_HandleBeams`; here both come after the beams' hits (filed as
    /// weapons-fidelity's phase 39).
    pub fn close_tick(
        &mut self,
        fighters: &mut [Fighter],
        arena: Arena<'_>,
        chance: &mut (impl Chance + ?Sized),
    ) {
        self.field = arena.field;
        self.note_slots(fighters);
        let mut source = &mut *chance;
        let chance: &mut dyn Chance = &mut source;
        let pass = arena.pass;
        let rule = arena.rules.disable;
        let targets: Vec<Target> = fighters.iter().map(Fighter::as_target).collect();
        let beams_held = self.beams.len();
        self.move_beams(fighters, &targets, pass, chance);
        // Steps 7 and 8.
        if pass.beams_act() {
            self.hold_on_missiles(pass.beam_damage());
        }
        let sides = Sides {
            targets: &targets,
            govts: arena.govts,
            pass_through: self.pass_through,
        };
        let first_strike = self.strikes.len();
        let mut hits = Vec::new();
        if pass.beams_act() {
            hits.extend(self.beam_hits(fighters, &sides, pass.beam_damage()));
        }
        self.land(fighters, hits, rule);
        // Steps 9 to 10.
        let begun = std::mem::take(&mut self.begun);
        for fighter in fighters.iter_mut() {
            let struck = self.strikes[first_strike..]
                .iter()
                .any(|strike| strike.ship == fighter.ship);
            if struck {
                truncate_downed_player(fighter, rule);
            }
            // A breakup the collision passes began takes no tick off on
            // its first tick (@0x2d834-0x2d850).
            if begun.contains(&fighter.ship) {
                continue;
            }
            let downed = self.update_condition(fighter, rule);
            self.mark_downed(first_strike, fighter.ship, downed);
        }
        self.shock(fighters, rule);
        for fighter in fighters.iter_mut() {
            regenerate(fighter);
        }
        self.ended_beams = beams_held.saturating_sub(self.beams.len());
        self.stood = fighters
            .iter()
            .map(|fighter| (fighter.ship, fighter.state.position))
            .collect();
    }
}

/// Counts the reloads of each of the `firers` among `fighters` down a
/// tick.
fn count_down(fighters: &mut [Fighter], firers: Firers) {
    for fighter in fighters
        .iter_mut()
        .filter(|fighter| firers.take(fighter.ship))
    {
        fighter.armament.reload();
    }
}
