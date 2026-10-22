//! The fight's tick as phases of `_DoPlayGameWork`, which its session runs
//! between its own work: the player's point defence, the player's
//! trigger and its regeneration, the collision passes, the NPCs' point
//! defence and launches, `_HandleShot`, each NPC's `_HandleShip` by slot,
//! then `_HandleBeams` and the late steps
//! (see the [`combat`](super) module docs' steps). [`Combat::tick_on`]
//! runs them in that order with nothing between them.

use super::asteroid_beam::BeamField;
use super::ion::IonLockout;
use super::lineup::reread;
use super::stellar_hazards::StellarHazards;
use super::{
    Combat, Field, Fighter, Firers, GamePass, Rules, Scenery, ShipRef, Sides, Target, regenerate,
    regenerate_player,
};
use crate::chance::Chance;
use crate::combat::armament::Arsenal;
use crate::geometry::Vec2;
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
    /// them (`Combat::hold_reloads`), each weapon it can't fire ionized
    /// held after them (@0x6ae73-0x6ae9c, [`IonLockout`](super::ion::IonLockout)).
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
        let lockout = self.ion_lockout;
        for fighter in fighters
            .iter_mut()
            .filter(|fighter| fighter.ship == ShipRef::Player)
        {
            lockout.reload_one(fighter);
        }
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
    /// them (`Combat::hold_reloads`), each weapon it can't fire ionized
    /// held after them (@0x338d6-0x338f0, [`IonLockout`](super::ion::IonLockout)); its shields and armour regenerate
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
        let mut order: Vec<(Slot, ShipRef)> = fighters
            .iter()
            .filter(|fighter| fighter.ship != ShipRef::Player)
            .map(|fighter| (fighter.slot, fighter.ship))
            .collect();
        order.sort_by_key(|&(slot, _)| slot);
        // The player's display ran before the collision passes.
        let mut shown = vec![ShipRef::Player];
        for (_, ship) in order {
            self.handle_npc(fighters, arena, ship, &mut shown, chance);
        }
    }

    /// Step 6 for one NPC, `ship` among `fighters`, as
    /// [`Combat::handle_npcs`] runs it at its slot, the ships in `shown`
    /// displayed before it this pass (the player first), to which it is
    /// added once displayed: its reloads, regeneration and fire, its death
    /// timer's tick, and its display, its wreck's shockwave striking at
    /// once. The session runs it at each NPC's slot, right after that
    /// NPC's flight (`nova_sim::Session::ship_loop`), keeping `shown`
    /// across the calls. Nothing happens for a ship not among `fighters`.
    /// It is [`Combat::step_npc`] then [`Combat::display_npc`].
    pub fn handle_npc(
        &mut self,
        fighters: &mut [Fighter],
        arena: Arena<'_>,
        ship: ShipRef,
        shown: &mut Vec<ShipRef>,
        chance: &mut (impl Chance + ?Sized),
    ) {
        self.step_npc(fighters, arena, ship, chance);
        self.display_npc(fighters, arena, ship, shown);
    }

    /// The first part of [`Combat::handle_npc`], `_HandleShip`'s fight
    /// for `ship` among `fighters`: its reloads, regeneration and fire,
    /// drawing on `chance`, and its death timer's tick (@0x35083). The
    /// session runs its dying block between this and its display
    /// (`nova_sim::Session::ship_loop`). Nothing happens for a ship not
    /// among `fighters`.
    pub fn step_npc(
        &mut self,
        fighters: &mut [Fighter],
        arena: Arena<'_>,
        ship: ShipRef,
        chance: &mut (impl Chance + ?Sized),
    ) {
        self.field = arena.field;
        self.note_slots(fighters);
        let Some(index) = fighters.iter().position(|fighter| fighter.ship == ship) else {
            return;
        };
        let mut source = &mut *chance;
        let chance: &mut dyn Chance = &mut source;
        let idle = fighters[index]
            .target
            .is_none_or(|target| fighters.iter().all(|other| other.ship != target));
        let fighter = &mut fighters[index];
        self.ion_lockout.reload_one(fighter);
        if idle {
            fighter.armament.hold_idle_bays();
        }
        regenerate(fighter, arena.rules.disable);
        let targets: Vec<Target> = fighters.iter().map(Fighter::as_target).collect();
        self.fire_one(&mut fighters[index], &targets, chance);
        fighters[index].condition.count_down();
    }

    /// The second part of [`Combat::handle_npc`], `_HandleShipDisplay`
    /// for `ship` among `fighters` (@0x444ad), the ships in `shown`
    /// displayed before it this pass, to which it is added: its display,
    /// its wreck's shockwave striking at once. Nothing happens for a ship
    /// not among `fighters`.
    pub fn display_npc(
        &mut self,
        fighters: &mut [Fighter],
        arena: Arena<'_>,
        ship: ShipRef,
        shown: &mut Vec<ShipRef>,
    ) {
        self.field = arena.field;
        self.note_slots(fighters);
        let Some(index) = fighters.iter().position(|fighter| fighter.ship == ship) else {
            return;
        };
        self.display(fighters, index, arena.rules.disable, shown);
        shown.push(ship);
    }

    /// Step 5, `_HandleShot` (@0x44385) for each shot slot in turn
    /// ([`Combat::handle_shots`]), among `fighters`, drawing on `chance`:
    /// at each slot's visit a held slot is freed, a shot its last move
    /// spent comes to its end of life (@0x36195), its sub-munitions
    /// released and then its blast landed, and any other shot flies a tick
    /// and lands its missile lock ([`Strike::lock`](super::Strike::lock),
    /// @0x35855). Each ship struck settles its condition as in the
    /// collision passes, so every later visit reads what the earlier ones
    /// landed, an escort's cleared leader included. Then each
    /// one-at-a-time shot holds its firer's reload (`Combat::hold_reloads`,
    /// @0x35c09).
    ///
    /// A held slot, a shot point defence ended and hid at life 0, has no
    /// end of life (@0x36195 → @0x35f0a → @0x3617c): the collision passes
    /// have already tested and let it go (`Combat::proximity_pass`).
    ///
    /// It lends the guidance no asteroids: it is [`Combat::fly_shots_in`]
    /// with none.
    pub fn fly_shots(
        &mut self,
        fighters: &mut [Fighter],
        arena: Arena<'_>,
        chance: &mut (impl Chance + ?Sized),
    ) {
        self.fly_shots_in(fighters, arena, &[], chance);
    }

    /// [`Combat::fly_shots`] with the guidance lent where each asteroid
    /// slot's asteroid is, by slot, as `_HandleShot` meets them: a homing
    /// `Seeker` 0x0002 shot may lock onto one, and a locked one steers at
    /// its slot's ([`asteroid_seek`](super::asteroid_seek)).
    pub fn fly_shots_in(
        &mut self,
        fighters: &mut [Fighter],
        arena: Arena<'_>,
        asteroids: &[Option<Vec2>],
        chance: &mut (impl Chance + ?Sized),
    ) {
        self.field = arena.field;
        self.note_slots(fighters);
        let mut source = &mut *chance;
        let chance: &mut dyn Chance = &mut source;
        let terms = (arena.govts, arena.arsenal);
        self.handle_shots(fighters, terms, arena.rules.disable, asteroids, chance);
        self.hold_reloads(fighters);
    }

    /// `_HandleBeams` (@0x4453d), step 6's beams and the late steps 7 to
    /// 9: the beams follow their firers among `fighters`, drawing on
    /// `chance`, each live beam stamping its firer's last shot on the
    /// advance pass, point-defence beams hit their missiles, the beams'
    /// hits land, each ship they struck settles its condition (truncated
    /// first for a newly downed player), and where each ship stands is
    /// kept for the next tick's collision passes. As the fight's tick
    /// ends, what each ship's ion block shed is cleared
    /// ([`Ion::shed`](super::ion::Ion::shed)). Every ship has regenerated and
    /// been displayed already, at its own step ([`Combat::display_player`],
    /// [`Combat::handle_npcs`]): a breakup a beam begins starts at the
    /// ship's next display step.
    ///
    /// It lends the beams no asteroid field: it is [`Combat::close_tick_in`]
    /// with none.
    pub fn close_tick(
        &mut self,
        fighters: &mut [Fighter],
        arena: Arena<'_>,
        chance: &mut (impl Chance + ?Sized),
    ) {
        self.close_tick_in(fighters, arena, None, chance);
    }

    /// [`Combat::close_tick`] with `field` lent to the beams: each beam
    /// whose ship pass hit nothing runs `_HandleBeams`' asteroid pass on
    /// it as its turn comes, before the next beam is tested
    /// ([`asteroid_beam`](super::asteroid_beam)), so a later beam meets
    /// what an earlier one left, its fragments included.
    pub fn close_tick_in(
        &mut self,
        fighters: &mut [Fighter],
        arena: Arena<'_>,
        mut field: Option<BeamField<'_>>,
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
        if pass.beams_act() {
            self.stamp_beam_firers(fighters);
        }
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
                    if let Some(field) = field.as_mut() {
                        self.cut_asteroids(index, fighters, field, pass, chance);
                    }
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
        IonLockout::clear_shed(fighters);
    }

    /// The stellar hazards among `fighters`: gravity's pull, then the
    /// deadly stellars' kills (`_HandleGravity` @0x44509,
    /// `_HandleDeadlyStellars` @0x4450e; see [`stellar_hazards`]), which
    /// `_DoPlayGameWork` runs after the ship loop and before
    /// `_HandleBeams` (@0x4453d, [`Combat::close_tick`]), on either pass.
    /// So a ship a deadly stellar kills is out of the beams.
    ///
    /// [`stellar_hazards`]: super::stellar_hazards
    pub fn hazards(&mut self, fighters: &mut [Fighter], hazards: &StellarHazards) {
        self.note_slots(fighters);
        self.stellar_hazards(fighters, hazards);
    }
}
