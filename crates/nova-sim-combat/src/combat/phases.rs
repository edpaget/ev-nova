//! The fight's tick as phases of `_DoPlayGameWork`, which its session runs
//! between its own work: the player's point defence, the player's
//! trigger and its regeneration, the collision passes, the NPCs' point
//! defence and launches, `_HandleShot`, each NPC's `_HandleShip` by slot,
//! then `_HandleBeams` and the late steps
//! (see the [`combat`](super) module docs' steps). [`Combat::tick_on`]
//! runs them in that order with nothing between them.

use super::collision::reread;
use super::{
    Combat, Field, Fighter, Firers, GamePass, Landed, Rules, Scenery, ShipRef, Sides, Target, lock,
    regenerate, regenerate_player,
};
use crate::chance::Chance;
use crate::combat::armament::Arsenal;
use crate::govt::Governments;
use crate::ship_layer::Slot;

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

    /// Step 4's point defence and fighter launches: every NPC's, as
    /// `_AIDispatch` runs them in its slot loop (@0x44270-0x442ae): by
    /// [`Fighter::slot`], each NPC's goal-4 launch (`_AILaunchFighter`
    /// @0x8e6fa, [`Trigger::bays`](super::armament::Trigger::bays)), its
    /// point defence (`_AIFireTurret` @0x80b0d), then the snipe's launch
    /// (@0x8732f, [`Trigger::bays_after`](super::armament::Trigger::bays_after)),
    /// reading the conditions the collision passes left, drawing on
    /// `chance` (see `ai_dispatch`). Each launch leaves a
    /// [`Sortie`](super::Sortie) for the session to put in the system
    /// before step 5. None on double time's second pass
    /// ([`GamePass::Again`]), as `_AIDispatch` runs on the advance pass
    /// alone (@0x4402a).
    ///
    /// The loop is resumable: it takes only the slots above `after`, and
    /// with `stop` set it stops right after the first slot whose turn
    /// launched a fighter, returning that slot; it returns none once it
    /// reaches the end. `_LaunchFighter` marks the fighter's slot active
    /// at once (`_GenericRandomShipSpawn` @0x3c90b, @0x3d680) and the loop
    /// rereads each slot as it comes, so the session puts the fighter in
    /// the system and resumes after that slot with it among `fighters`, a
    /// candidate of every later slot's point defence and, in a slot above
    /// its carrier's, taking its own turn
    /// (`nova_sim::Session::dispatch_npcs`).
    ///
    /// The original interleaves each NPC's point defence and launches with
    /// its decision, slot by slot; the session runs every existing NPC's
    /// decision first and a launched fighter's at its launch
    /// (`nova_sim::Session::dispatch_npcs`). Both settle before the frame
    /// is drawn.
    pub fn defend_npcs(
        &mut self,
        fighters: &mut [Fighter],
        arena: Arena<'_>,
        after: Option<Slot>,
        stop: bool,
        chance: &mut (impl Chance + ?Sized),
    ) -> Option<Slot> {
        if arena.pass == GamePass::Again {
            return None;
        }
        self.field = arena.field;
        self.note_slots(fighters);
        let mut source = &mut *chance;
        let chance: &mut dyn Chance = &mut source;
        let targets: Vec<Target> = fighters.iter().map(Fighter::as_target).collect();
        let defence = (arena.rules.defence, arena.govts);
        self.dispatch_npcs(fighters, &targets, defence, (after, stop), chance)
    }

    /// The player's shields and armour regenerate, as `_HandlePlayer` runs
    /// it after its trigger and before its move (@0x6cf86-0x6d093), by
    /// `arena`'s rules: so before that frame's `_HandleBeams` (see
    /// `regenerate_player`).
    pub fn regenerate_player(&mut self, fighters: &mut [Fighter], arena: Arena<'_>) {
        let rule = arena.rules.disable;
        for fighter in fighters
            .iter_mut()
            .filter(|fighter| fighter.ship == ShipRef::Player)
        {
            regenerate_player(fighter, rule);
        }
    }

    /// Step 6, `_HandleShip` for each NPC, as `_DoPlayGameWork`'s ship
    /// loop runs it (@0x4445c-0x444c4), by [`Fighter::slot`]: its reloads
    /// count down a tick (@0x33896-0x338ce), after `_HandleShot` has held
    /// them (`Combat::hold_reloads`); its shields and armour regenerate
    /// (@0x34424-0x34677, `regenerate`); then it fires the weapons its
    /// trigger holds that are ready (`_FireAIShipWeapon` @0x34ffd,
    /// @0x3500a), at the ships as the slots before it left them, drawing
    /// on `chance`; then its death timer ticks (@0x35083,
    /// [`Condition::count_down`](super::hull::Condition::count_down)), and
    /// its display step runs (@0x444ad, see the `display` module), its
    /// wreck's shockwave striking at once.
    pub fn handle_npcs(
        &mut self,
        fighters: &mut [Fighter],
        arena: Arena<'_>,
        chance: &mut (impl Chance + ?Sized),
    ) {
        self.field = arena.field;
        self.note_slots(fighters);
        let mut source = &mut *chance;
        let chance: &mut dyn Chance = &mut source;
        let mut order: Vec<usize> = (0..fighters.len())
            .filter(|&index| fighters[index].ship != ShipRef::Player)
            .collect();
        order.sort_by_key(|&index| fighters[index].slot);
        let rule = arena.rules.disable;
        // The player's display ran before the collision passes.
        let mut shown = vec![ShipRef::Player];
        for index in order {
            let fighter = &mut fighters[index];
            fighter.armament.reload();
            regenerate(fighter, rule);
            let targets: Vec<Target> = fighters.iter().map(Fighter::as_target).collect();
            self.fire_one(&mut fighters[index], &targets, chance);
            fighters[index].condition.count_down();
            self.display(fighters, index, rule, &shown);
            shown.push(fighters[index].ship);
        }
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
    /// 9: the beams follow their firers among `fighters`, drawing on
    /// `chance`, point-defence beams hit their missiles, the beams' hits
    /// land, each ship they struck settles its condition (truncated first
    /// for a newly downed player), and where each ship stands is kept for
    /// the next tick's collision passes. Every ship has regenerated and
    /// been displayed already, at its own step ([`Combat::display_player`],
    /// [`Combat::handle_npcs`]): a breakup a beam begins starts at the
    /// ship's next display step.
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
        let mut targets: Vec<Target> = fighters.iter().map(Fighter::as_target).collect();
        let beams_held = self.beams.len();
        self.move_beams(fighters, &targets, pass, chance);
        // Steps 7 and 8.
        if pass.beams_act() {
            self.hold_on_missiles(pass.beam_damage());
        }
        let first_strike = self.strikes.len();
        if pass.beams_act() {
            // Each beam's hit lands before the next beam is tested
            // (`_HandleBeams` @0x316a1), so a later beam's filter reads the
            // escort an earlier one took out of the player's fleet.
            for index in 0..self.beams.len() {
                let sides = Sides {
                    targets: &targets,
                    govts: arena.govts,
                    pass_through: self.pass_through,
                };
                let Some(hit) = self.beam_hit(index, fighters, &sides, pass.beam_damage()) else {
                    continue;
                };
                let ship = hit.ship;
                self.land(fighters, vec![hit], rule);
                if let Some(struck) = fighters.iter().position(|fighter| fighter.ship == ship) {
                    reread(&fighters[struck], &mut targets[struck], rule);
                }
            }
        }
        // Step 9: each ship struck settles; a breakup begun here starts at
        // the ship's next display step.
        self.settle_struck(fighters, first_strike, rule);
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
