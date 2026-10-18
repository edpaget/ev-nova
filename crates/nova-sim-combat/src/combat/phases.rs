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
        let mut source = &mut *chance;
        let chance: &mut dyn Chance = &mut source;
        let targets: Vec<Target> = fighters.iter().map(Fighter::as_target).collect();
        let defence = (arena.rules.defence, arena.govts);
        self.defend(fighters, &targets, defence, chance, Firers::Player);
    }

    /// Step 1's trigger: the player fires the weapons its trigger holds
    /// that are ready, as `_HandlePlayer` calls `_FirePlayerWeapon`
    /// (@0x6ab55, @0x6abc7), drawing on `chance` (see `Combat::fire`).
    pub fn fire_player(
        &mut self,
        fighters: &mut [Fighter],
        arena: Arena<'_>,
        chance: &mut (impl Chance + ?Sized),
    ) {
        self.field = arena.field;
        let mut source = &mut *chance;
        let chance: &mut dyn Chance = &mut source;
        let targets: Vec<Target> = fighters.iter().map(Fighter::as_target).collect();
        self.fire(fighters, &targets, chance, Firers::Player);
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
        let mut source = &mut *chance;
        let chance: &mut dyn Chance = &mut source;
        let terms = (arena.govts, arena.arsenal);
        let begun = self.hit_pass(fighters, terms, arena.rules.disable, scenery, chance);
        self.begun.extend(begun);
    }

    /// Step 4's point defence: every NPC's, as `_AIDispatch` runs it in
    /// its slot loop (@0x4429d, `_AIFireTurret` @0x80b0d), reading the
    /// conditions the collision passes left, drawing on `chance`.
    pub fn defend_npcs(
        &mut self,
        fighters: &mut [Fighter],
        arena: Arena<'_>,
        chance: &mut (impl Chance + ?Sized),
    ) {
        self.field = arena.field;
        let mut source = &mut *chance;
        let chance: &mut dyn Chance = &mut source;
        let targets: Vec<Target> = fighters.iter().map(Fighter::as_target).collect();
        let defence = (arena.rules.defence, arena.govts);
        self.defend(fighters, &targets, defence, chance, Firers::Npcs);
    }

    /// Step 4's picks: each NPC fires the weapons its trigger holds that
    /// are ready, as `_HandleShip` fires its pick (@0x44483), drawing on
    /// `chance`; then every reload counts down a tick (step 5).
    pub fn fire_npcs(
        &mut self,
        fighters: &mut [Fighter],
        arena: Arena<'_>,
        chance: &mut (impl Chance + ?Sized),
    ) {
        self.field = arena.field;
        let mut source = &mut *chance;
        let chance: &mut dyn Chance = &mut source;
        let targets: Vec<Target> = fighters.iter().map(Fighter::as_target).collect();
        self.fire(fighters, &targets, chance, Firers::Npcs);
        for fighter in fighters.iter_mut() {
            fighter.armament.reload();
        }
    }

    /// Step 6's shots, `_HandleShot` (@0x44385): the shots fly among
    /// `fighters`, drawing on `chance`, the tick's missile locks are
    /// found, and each one-at-a-time shot holds its firer's reload.
    pub fn fly_shots(
        &mut self,
        fighters: &mut [Fighter],
        arena: Arena<'_>,
        chance: &mut (impl Chance + ?Sized),
    ) {
        self.field = arena.field;
        let mut source = &mut *chance;
        let chance: &mut dyn Chance = &mut source;
        let targets: Vec<Target> = fighters.iter().map(Fighter::as_target).collect();
        self.fly(fighters, &targets, chance);
        let sides = Sides {
            targets: &targets,
            govts: arena.govts,
            pass_through: self.pass_through,
        };
        let locks: Vec<Landed> = self
            .shots
            .iter()
            .filter_map(|shot| lock(shot, &sides))
            .collect();
        self.locks.extend(locks);
        self.hold_reloads(fighters);
    }

    /// `_HandleBeams` (@0x4453d) and the late steps 7 to 10: the beams
    /// follow their firers among `fighters`, point-defence beams hit
    /// their missiles, the locks and the beams' hits land, conditions
    /// follow the damage, the shockwaves go off, shields and armour
    /// regenerate, and where each ship stands is kept for the next tick's
    /// collision passes.
    pub fn close_tick(
        &mut self,
        fighters: &mut [Fighter],
        arena: Arena<'_>,
        chance: &mut (impl Chance + ?Sized),
    ) {
        self.field = arena.field;
        let mut source = &mut *chance;
        let chance: &mut dyn Chance = &mut source;
        let pass = arena.pass;
        let rule = arena.rules.disable;
        let targets: Vec<Target> = fighters.iter().map(Fighter::as_target).collect();
        let beams_held = self.beams.len();
        self.move_beams(fighters, &targets, pass, chance);
        // Steps 7 and 8.
        self.ended_shots = 0;
        if pass.beams_act() {
            self.hold_on_missiles(pass.beam_damage());
        }
        let sides = Sides {
            targets: &targets,
            govts: arena.govts,
            pass_through: self.pass_through,
        };
        let first_strike = self.strikes.len();
        let mut hits = std::mem::take(&mut self.locks);
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
