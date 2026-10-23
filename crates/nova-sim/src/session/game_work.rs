//! An advance pass in `_DoPlayGameWork(1)`'s order ([`Session::game_work`]),
//! and the fight's phases as the session runs them (see [`Combat`]'s
//! phases).
//!
//! **The order** (`_DoPlayGameWork` @0x43de5-0x4453d):
//!
//! 1. `_HandlePlayer` (@0x43e19) up to its move: the pass's head and the
//!    player's work before the move; its self-repair (@0x68b3d), point
//!    defence (@0x68c74), sabotage (@0x692e5), trigger (@0x6ab55) and
//!    reloads' countdown (@0x6ae25), its recoil (@0x62f35) and its bays'
//!    fighters (@0x62c61) settled, and its regeneration (@0x6cf86), before
//!    it moves ([`Session::player_fires`]).
//! 2. The move, the turn, thrust and position step (@0x6c770-0x6d4eb),
//!    or the escape pod's flight; the rescue, which ends the pass
//!    (@0x68ddd); then the player's display step: its death timer's tick
//!    (@0x6d7e6), the klaxon (@0x6f1e4), the eject check (@0x6f22c) and
//!    its display (@0x43e29), which starts its breakup's countdown or
//!    explodes it ([`Session::player_display_step`]).
//! 3. The collision passes (@0x43e70-0x44015), their hits settled and
//!    answered at once, as `_DamageShip`'s reaction block runs in them,
//!    a shot's hit that starts a ship dying queuing the victory of the
//!    escort attacking it (`_HandleShipHit` @0x36c7f-0x36d45).
//! 4. The advance pass's AI (@0x44223-0x442ae): the escorts' voice
//!    channel played (`_HandleVoiceSound` @0x44219, see the `voices`
//!    module), the traffic's think half,
//!    the damage aimed read after the passes (`_CalculateTargetedDamage`
//!    @0x44223), the arrivals (`_EnterMoreShips` @0x44237), the fleets
//!    announced and the refuel landed, then every NPC's point defence and
//!    fighter launches, slot by slot (`_AIFireTurret` @0x80b0d,
//!    `_AILaunchFighter` @0x8e6fa and @0x8732f; [`Combat::defend_npcs`]),
//!    each fighter launched put in the system as its slot's turn ends
//!    (`_LaunchFighter`, see the `fighters` module), a fighter above its
//!    carrier thinking then, and the loop resumed after that slot
//!    ([`Session::dispatch_npcs`]); then the thinks' voice cues answered
//!    in berth order; then the frame's work on the line,
//!    every think's calls, scan, plunders and refuel note, the launched
//!    fighters' among them, settled in slot order
//!    ([`Session::settle_line_queue`], see the `comm_calls` module). All
//!    before `_HandleShot`, so the fighters move on their launch frame.
//! 5. `_HandleShot` (@0x44385): slot by slot, each spent shot's end of
//!    life or each other shot's move and lock, landing at its visit,
//!    their hits settled and answered.
//! 6. The ship loop (@0x4445c-0x444c4): slot by slot, each NPC's
//!    `_HandleShip` (@0x44483), its flight (the traffic's fly half, one
//!    NPC at a time), reloads, regeneration, fire and death timer's tick,
//!    a dying NPC's escape pods and escape-ship roll (see the
//!    `npc_escape` module), then its display (@0x444ad), which starts its
//!    breakup's countdown or explodes it, its shockwave reaching the
//!    later slots before they move; an escape fighter launched into a
//!    later slot is reached in the same loop; then the session's work
//!    after the fly half, the line work its docks and repairs queued
//!    settled ([`Session::ship_loop`]).
//! 7. The asteroids (`_SpawnAsteroid` @0x44243 and `_HandleAsteroids`
//!    @0x444f5, after the collision passes; see the `asteroids` module),
//!    and on the second pass the reinforcement clock
//!    (`_HandleReinforcements` @0x44504); then the stellar hazards,
//!    gravity's pull and the deadly stellars' kills (`_HandleGravity`
//!    @0x44509, `_HandleDeadlyStellars` @0x4450e), then `_HandleBeams`
//!    (@0x4453d), its hits settled and the field lent to it, so a beam
//!    meets each asteroid where this pass's step left it
//!    ([`Session::beams_phase`], see the `asteroid_beams` module), then
//!    what the pass leaves ([`Session::fight_tail`]).
//!
//! Landed, the pass runs the separate parts instead, as the fight stands
//! still. Through a jump the world runs on, in this order (see the
//! `warp_stage` module). Double time's second pass
//! ([`Session::tick_again`]) runs the same order without the AI: from the
//! eject check on, both passes run one routine
//! ([`Session::pass_after_move`]), which branches where `_DoPlayGameWork`
//! branches on its argument (@0x44026-0x4402a).
//!
//! **What remains**, none crossing a boundary the player sees, so none
//! filed:
//!
//! - What the ship loop's departures leave the session, and the fly
//!   half's end, settle after the loop (see [`Session::ship_loop`]).
//! - A final explosion's session work (the fates, a mission ship lost, a
//!   person gone, the take-out) runs in [`Session::fight_tail`], later in
//!   the same pass and before the frame is drawn, where
//!   `_HandleShipDisplay` runs it at the ship's display
//!   (@0x2daa2-0x2dcfc).
//! - Every existing NPC's point defence and launches run after every
//!   existing NPC's decision, where `_AIDispatch` interleaves them slot by
//!   slot; a fighter launched in the loop decides at its launch.
//! - Every think's work on the line settles after the whole loop, in slot
//!   order, where each slot's `_AIDispatch` says its own (see
//!   [`Session::dispatch_npcs`]); a dock or repair the fly half makes
//!   settles after the ship loop, where `_LowLevelAIHandler` makes it in
//!   the AI loop.
//! - The asteroid edge spawn runs with the step after the ship loop,
//!   where `_SpawnAsteroid` (@0x44243) runs before the AI: a new asteroid
//!   is met by no shot until the next pass either way, and the step comes
//!   before the beams in both (see the `asteroids` module).
//! - What the close strikes (the beams, the shockwaves) is answered by the
//!   caller after the pass ([`Session::answer_strikes`]), before the frame
//!   is drawn.
//! - The session's part of an escort a shot's blast or lock takes out of
//!   the player's fleet (its record, AI type and cargo share) settles
//!   after `_HandleShot`'s phase ([`Session::settle_landed`]), where
//!   `_DamageShip` settles it at the landing; the fight clears its leader
//!   at the landing, which is all the later visits read.
//! - The separate parts ([`Session::tick`], [`Session::tick_traffic`],
//!   [`Session::tick_combat`]) keep the old order between the player, the
//!   traffic and the fight, the fight's own phases in its new order, an
//!   NPC's fighters put in the system and every dying NPC's escape pods
//!   and escape-ship roll run at the tick's end; they
//!   are no game path, and the tests that step through them are to step
//!   through [`Session::game_work`] (rdm task
//!   `retire-the-sessions-separate-pass-parts`).
//!
//! **Filed.** An existing NPC in a slot above a carrier thinks before
//! the loop, blind to the fighter launched that frame, and a launched
//! fighter's own launch settles against the room left after every
//! existing claim, where the original's loop reaches them in slot order
//! (rdm:phase/combat-ai-fidelity-cross-roadmap/phase-6-left-out-of-weapons-fidelity-phase-44).
//!
//! **The phases.** Each builds the fight's [`Fighter`]s afresh from the
//! session ([`Session::with_fighters`]): the player, then every NPC, each
//! on the terms the session works out for it. A phase that lands hits is
//! settled at once ([`Session::settle_landed`]): the player's downing
//! lines, the pushes, the NPCs' fates, the strikes' answers, crimes and
//! kills. A target that starts breaking up, emerging or cloaked in a
//! phase stays the target through the pass and is let go at the next
//! pass's head (see the `lost_target` module). What the whole pass leaves
//! is settled once, after its last phase ([`Session::fight_tail`]).

use super::ship_loop::Motion;
use super::{FightTerms, NpcBefore, Session, Space, TrafficMarks};
use crate::ai::Behaviour;
use crate::catalog::{PilotCatalog, TrafficCatalog};
use crate::chance::Chance;
use crate::combat::armament::{OutfitRounds, Trigger};
use crate::combat::asteroid_walk::AsteroidField;
use crate::combat::hull::Condition;
use crate::combat::{Arena, Combat, Fighter, GamePass, Rules, Scenery, ShipRef};
use crate::flight::Controls;
use crate::traffic::npc::NpcId;

/// What a pass's fight reads of the session as it began: every NPC, and
/// the player as `_HandlePlayer` read it at its top.
pub(super) struct PassStart {
    /// Every NPC.
    npcs: Vec<NpcBefore>,
    /// The player as the pass began.
    player: PlayerHead,
}

/// What `_HandlePlayer` computes of the player at its top
/// (@0x68aba-0x68ac6), before the pass's display settles a new
/// condition.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct PlayerHead {
    /// Whether the player was dying or destroyed, `_IsDying` (-0x2ca),
    /// which the self-destruct reads (see the `self_destruct` module).
    pub(super) dying: bool,
    /// Whether the player was disabled or breaking up, `_IsDisabled`
    /// (-0x2cb), which the cloak key and upkeep read (see the
    /// `player_cloak` module).
    pub(super) disabled: bool,
}

impl PassStart {
    /// `session` as its pass's fight begins.
    pub(super) fn of(session: &Session) -> Self {
        let condition = session.ship.condition;
        Self {
            npcs: session.space.npcs_before(),
            player: PlayerHead {
                dying: matches!(condition, Condition::Dying { .. } | Condition::Destroyed),
                disabled: matches!(condition, Condition::Disabled | Condition::Dying { .. }),
            },
        }
    }

    /// The player as the pass began.
    pub(super) fn player_head(&self) -> PlayerHead {
        self.player
    }

    /// Every NPC as the pass's fight began.
    pub(super) fn npcs(&self) -> &[NpcBefore] {
        &self.npcs
    }

    /// NPC `id`'s condition as the pass's fight began, if it was there.
    pub(super) fn condition_of(&self, id: NpcId) -> Option<Condition> {
        self.npcs
            .iter()
            .find(|before| before.id == id)
            .map(|before| before.condition)
    }
}

impl Session {
    /// One advance pass of the game's work under the player's `controls`,
    /// in `_DoPlayGameWork(1)`'s order (see the module docs), the traffic
    /// drawn from `catalog` and deciding as `behaviour` says, the fight by
    /// `rules`, drawing on `chance`. Whether the escape pod's rescue
    /// settled on it, which ends the pass there (`_HandlePlayer` @0x68ddd;
    /// see [`Session::rescue_if_due`]).
    ///
    /// Landed, it runs the separate parts instead ([`Session::tick`],
    /// [`Session::rescue_if_due`], [`Session::tick_traffic`] and
    /// [`Session::tick_combat`]), as the fight stands still then. Through
    /// a jump it runs on in its own order, as the world does (see the
    /// `warp_stage` module).
    pub fn game_work(
        &mut self,
        controls: Controls,
        catalog: &(impl PilotCatalog + TrafficCatalog),
        behaviour: &(impl Behaviour + ?Sized),
        rules: Rules,
        chance: &mut (impl Chance + ?Sized),
    ) -> bool {
        if self.mode.landed().is_some() {
            self.tick(controls);
            if self.rescue_if_due(catalog, chance) {
                return true;
            }
            self.tick_traffic(catalog, behaviour, chance);
            self.tick_combat(rules, chance);
            return false;
        }
        let mut source = &mut *chance;
        let chance: &mut dyn Chance = &mut source;
        // `_HandlePlayer` (@0x43e19), up to its move.
        self.ship.warp.gate = None;
        let frame = self.pass_head();
        let pass = GamePass::Advance {
            double_time: self.ship.warp.double_time,
        };
        self.before_move(frame, pass);
        self.ship.fate.player_exploded_this_tick = false;
        let start = PassStart::of(self);
        self.player_fires(pass, rules, chance);
        self.player_move(controls, pass);
        // `_HandlePlayer`'s warning on a new jump (@0x6b3de-0x6b565).
        self.warn_fighters_left_behind(chance);
        if self.rescue_if_due(catalog, chance) {
            return true;
        }
        self.pass_after_move(&start, pass, Some(catalog), behaviour, rules, chance);
        false
    }

    /// The rest of a pass of the game's work after the player's move, from
    /// the player's display step ([`Session::player_display_step`]) on, on
    /// `pass` by `rules`, against the session as the pass `start`ed, the
    /// NPCs answering its hits as `behaviour` says, drawing on `chance`:
    /// both passes run it, [`Session::game_work`] and
    /// [`Session::tick_again`], as the original runs one routine,
    /// `_DoPlayGameWork(pass)`, for both (see the module docs).
    ///
    /// `ai` is the advance pass's AI, its traffic drawn from the catalog
    /// it holds; the second pass has none, as `_DoPlayGameWork(0)` jumps
    /// past it (@0x4402a to @0x44350). Where the original branches on its
    /// argument, this branches on `ai`: with it, the traffic thinks and
    /// its fleets are said, every NPC's point defence fires and its
    /// fighters launch, each put in the system as its slot's turn ends
    /// ([`Session::dispatch_npcs`]), then the frame's work on the line
    /// settles in slot order ([`Session::settle_line_queue`]), all before
    /// the shots, and the ship loop's traffic flies its fly half, its
    /// reinforcement clock included ([`Session::traffic_tail`]); without
    /// it, the traffic only moves, and the reinforcement clock
    /// (`_HandleReinforcements` @0x44504) ticks after the asteroids and
    /// before the beams (see [`Session::ship_loop`]).
    pub(super) fn pass_after_move(
        &mut self,
        start: &PassStart,
        pass: GamePass,
        ai: Option<&dyn TrafficCatalog>,
        behaviour: &(impl Behaviour + ?Sized),
        rules: Rules,
        chance: &mut dyn Chance,
    ) {
        // The end of `_HandlePlayer` and its display (@0x43e29).
        self.player_display_step(pass, rules, start.player_head(), chance);
        // The collision passes (@0x43e70-0x44015).
        self.collision_passes(pass, rules, chance);
        self.answer_strikes(behaviour);
        // The advance pass's AI (@0x44223-0x442ae).
        let thought = if let Some(catalog) = ai {
            // `_HandleVoiceSound` (@0x44219), before `_AIDispatch`
            // (@0x4429d; see the `voices` module).
            self.space
                .handle_voice(&self.statics, &mut self.out, &mut *chance);
            let marks = self.traffic_think(catalog, behaviour, chance);
            // The arrivals of a Derelict government come in disabled.
            self.space.hold_derelicts(&self.statics);
            // `_LaunchFighter` marks each fighter's slot active in
            // `_AIDispatch` (@0x3c90b, @0x3d680), so the rest of the loop,
            // `_HandleShot` and its own `_HandleShip` see it that frame.
            self.dispatch_npcs(pass, behaviour, rules, chance);
            // `_EscortAI`'s variants and target calls, in berth order.
            self.space.answer_voice_cues(&self.statics, &mut *chance);
            // Every think's work on the line, the launched fighters'
            // among it, in slot order (see `Session::dispatch_npcs`).
            self.settle_line_queue(rules.law, chance);
            Some(marks)
        } else {
            None
        };
        // `_HandleShot` (@0x44385), its guidance reading the field
        // before `_HandleAsteroids` moves it.
        let spots = self.space.asteroid_spots();
        self.land_phase(pass, rules, chance, |combat, fighters, arena, chance| {
            combat.fly_shots_in(fighters, arena, &spots, chance);
        });
        self.answer_strikes(behaviour);
        // The ship loop: `_HandleShip` (@0x44483) and `_HandleShipDisplay`
        // (@0x444ad), slot by slot.
        let motion = match thought {
            Some(Some(marks)) => Motion::Fly(marks),
            Some(None) => Motion::Populated,
            None => Motion::Again,
        };
        self.ship_loop(start, motion, pass, rules, chance);
        // `_HandleAsteroids` (@0x444f5) and the second pass's
        // `_HandleReinforcements` (@0x44504), after the collision passes
        // and before the beams, which meet each asteroid where this step
        // left it (see the `asteroids` module).
        self.asteroid_pass(pass, chance);
        if ai.is_none() {
            self.tick_reinforcements(chance);
        }
        // The stellar hazards and `_HandleBeams` (@0x44509-0x4453d), and
        // what the pass leaves.
        self.beams_phase(pass, rules, chance);
        self.fight_tail(rules, chance);
    }

    /// `_HandlePlayer`'s fight before its move, on `pass` by `rules`,
    /// drawing on `chance`: the boarded flag's clear unless the player is
    /// disabled (@0x68b02; see the `pirate_plunder` module), the repair
    /// inhibit's countdown and the
    /// disabled player's self-repair (@0x68b3d-0x68c6c; see the
    /// `player_repair` module), its point defence (@0x68c74,
    /// [`Combat::defend_player`]), the sabotage clock's tick (@0x692e5;
    /// see the `sabotage` module), its trigger and its reloads' countdown
    /// (@0x6ab55, @0x6ae25, [`Combat::fire_player`]); then its recoil set
    /// in its velocity (`_AdjustedAccel` @0x62f35) and the fighters its bays
    /// launched put in the system (`_LaunchFighter` @0x62c61), all before
    /// it moves. So it fires from where it stood, along the heading and at
    /// the velocity the last pass left, and a sabotage that downs it still
    /// lets its point defence fire that pass. Last, its shields and armour
    /// regenerate (@0x6cf86-0x6d093, [`Combat::regenerate_player`]), after
    /// its trigger and before its position step (@0x6d4b0).
    pub(super) fn player_fires(&mut self, pass: GamePass, rules: Rules, chance: &mut dyn Chance) {
        self.ship.unboard_unless_disabled();
        self.player_repair_tick(rules, chance);
        self.with_fighters(pass, rules, chance, |combat, fighters, arena, chance| {
            combat.defend_player(fighters, arena, chance);
        });
        self.sabotage_tick(rules, chance);
        self.space.count_player_berths(&self.setup);
        self.with_fighters(pass, rules, chance, |combat, fighters, arena, chance| {
            combat.fire_player(fighters, arena, chance);
        });
        self.take_shoves();
        let sorties = self.space.combat.take_sorties();
        self.launch_fighters(&sorties, chance);
        self.with_fighters(pass, rules, chance, |combat, fighters, arena, _| {
            combat.regenerate_player(fighters, arena);
        });
    }

    /// `_HandleShip`'s fight and `_HandleShipDisplay` for each NPC on
    /// `pass` by `rules`, drawing on `chance`, for the separate parts
    /// ([`Session::tick_combat`]), whose traffic has flown already (the
    /// game's passes run [`Session::ship_loop`]): every NPC's stray
    /// provocation eases
    /// (@0x35374-0x353b2), after the collision passes and the shots and
    /// before the beams; then, slot by slot, each NPC's reloads count
    /// down, it regenerates, it fires its pick, its death timer ticks and
    /// it is displayed ([`Combat::handle_npcs`]), a final explosion's
    /// shockwave striking at once, its strikes settled
    /// ([`Session::land_phase`]), and laid out
    /// ([`Session::lay_out_displayed`]); every NPC's cloak fade steps
    /// ([`Traffic::step_fades`](crate::traffic::Traffic::step_fades)); and
    /// the pick fired is cleared or kept
    /// ([`Traffic::after_firing`](crate::traffic::Traffic::after_firing)).
    pub(super) fn handle_npcs(&mut self, pass: GamePass, rules: Rules, chance: &mut dyn Chance) {
        self.space.traffic.ease_stray();
        self.land_phase(pass, rules, chance, |combat, fighters, arena, chance| {
            combat.handle_npcs(fighters, arena, chance);
        });
        self.lay_out_displayed();
        self.space.traffic.step_fades();
        self.space.traffic.after_firing(&self.statics.arsenal);
    }

    /// The advance pass's AI, reading `catalog`, deciding as `behaviour`
    /// says and drawing on `chance`: on the first pass in a system not
    /// yet set up, its population ([`Session::populate`]), which moves
    /// from the next; otherwise the session's work before the AI and the
    /// traffic's think half ([`Traffic::think_holding`]), and the fleets
    /// it brings said and the refuel it gave landed
    /// (`Space::after_traffic_ai`); the work it queues on the line
    /// settles after the launch frame's loop. What its motion is read
    /// against, when it thought.
    ///
    /// [`Traffic::think_holding`]: crate::traffic::Traffic::think_holding
    fn traffic_think(
        &mut self,
        catalog: &(impl TrafficCatalog + ?Sized),
        behaviour: &(impl Behaviour + ?Sized),
        chance: &mut dyn Chance,
    ) -> Option<TrafficMarks> {
        if self.space.traffic_due {
            // No system is set up while the jump is under way.
            if self.jumping().is_none() {
                self.populate(catalog, chance);
            }
            return None;
        }
        let (strikes, marks) = self.before_traffic_ai(catalog, chance);
        self.step_traffic(|traffic, world| {
            traffic.think_holding(behaviour, world, &strikes, chance);
        });
        self.space
            .after_traffic_ai(&mut self.pilot, &self.setup, &mut self.out, chance);
        Some(marks)
    }

    /// Runs `phase` of the fight on `pass` by `rules`, drawing on
    /// `chance`, among the player and every NPC as the session holds them
    /// now, each a [`Fighter`] on its terms ([`Session::fight_terms`]);
    /// the player's trigger is none in the escape pod, and only its point
    /// defence while its jump is under way (see `player_trigger`).
    pub(super) fn with_fighters(
        &mut self,
        pass: GamePass,
        rules: Rules,
        chance: &mut dyn Chance,
        phase: impl FnOnce(&mut Combat, &mut [Fighter], Arena<'_>, &mut dyn Chance),
    ) {
        let (player_terms, terms): (FightTerms, Vec<FightTerms>) = self.fight_terms();
        let field = self.field();
        let scanner = self.scanner();
        let armor_regen = self.ship.player_armor_regen();
        // This tick's `_abOn`, as `_HandlePlayer` runs before the shots
        // (see the `afterburner` module).
        let knockback = self.afterburner_knockback();
        let has_cloak = self.ship.cloak_gear(&self.pilot, &self.statics).device;
        let pilot = &mut self.pilot;
        let mut rounds = OutfitRounds {
            owned: &mut pilot.outfits,
            sources: &self.statics.ammo_outfits,
        };
        let mut fighters = vec![Fighter {
            ship: ShipRef::Player,
            ship_type: pilot.ship,
            fleet: ShipRef::Player,
            govt: None,
            leader: None,
            fleet_escort: false,
            booty: None,
            state: self.ship.player,
            hull: self.ship.fit.hull,
            shield_regen: self.ship.fit.stats.shield_regen,
            armor_regen,
            trigger: if self.pod.is_some() {
                Trigger::default()
            } else {
                super::player_trigger(self.ship.trigger, self.mode.jump().is_some())
            },
            target: self.chart.target.map(ShipRef::Npc),
            reserves: &mut pilot.reserves,
            condition: &mut self.ship.condition,
            held_disabled: false,
            // A cloaked player fires only its weapons that fire cloaked
            // (`_FirePlayerWeapon` @0x627f5-0x62814, point defence
            // @0x3931c-0x3932f), and NPCs' point defence and homing shots
            // see it as they see any cloaked ship (@0x3955f, @0x322d8).
            cloaked: self.ship.cloak.cloak.is_cloaked(),
            attacking: None,
            threatens_player: false,
            plundering: false,
            mining: false,
            emerging: false,
            cloak_scanner: scanner.on_screen,
            has_cloak,
            jamming: player_terms.jamming,
            class_speed: player_terms.class_speed,
            top_speed: player_terms.top_speed,
            jumping: self.mode.jump().is_some(),
            afterburner: knockback.shots,
            afterburner_beam: knockback.beams,
            tractor: &mut self.ship.tractor,
            ion_capacity: self
                .ship
                .fit
                .stats
                .ion
                .capacity(crate::flight::Flyer::Player),
            ion: &mut self.ship.ion,
            armament: &mut self.ship.fit.armament,
            rounds: &mut rounds,
            drawn_heading: player_terms.drawn_heading,
            docked: false,
            resists: player_terms.resists,
            sprite: player_terms.sprite,
            slot: crate::ship_layer::Slot::Player,
        }];
        let npcs = self.space.traffic.npcs_mut().iter_mut().zip(terms);
        fighters.extend(npcs.map(|(npc, terms)| terms.fighter(npc)));
        let arena = Arena {
            pass,
            arsenal: &self.statics.arsenal,
            govts: &self.statics.govts,
            rules,
            field,
        };
        phase(&mut self.space.combat, &mut fighters, arena, chance);
    }

    /// Settles the hits a phase of the fight on `pass` landed, against
    /// what each NPC `was` and the player's condition `player_was` before
    /// it, by `rules`, drawing on `chance`: the player's "Ship disabled."
    /// and "Ship destroyed." lines, said as the hits land, before the
    /// eject on a breakup's first tick can take the player into a new
    /// ship, a deadly stellar's kill in the phase, `killed`, read as the
    /// condition the hits left (see the `player_downed` module); the
    /// pushes; each NPC newly
    /// downed raising its fate; the strikes weighed, answered for crimes
    /// and kills ([`Session::settle_strikes`]) and kept for the NPCs'
    /// answers. A target the phase left dying or cloaked is kept: the next
    /// pass's head lets go of it (see the `lost_target` module).
    ///
    /// Between the fates and the strikes, every escort the phase left
    /// disabled leaves the fleet ([`Session::lose_escorts`]): its record,
    /// AI type and cargo share, as `_DamageShip`'s escort block
    /// (@0x3af8f-0x3b0d7) runs before its reaction block (@0x3b408), so
    /// the answers read it unled, and so does everything after the phase.
    /// A kill credits the player's combat rating, which the pilot's save
    /// keeps, by the shooter's leader at the kill, as `_DamageShip` reads
    /// it as each hit lands (@0x3ad72): the fight notes each escort lost
    /// with the strike that lost it ([`Combat::take_landed`]), so a kill an
    /// escort scored up to that strike in the phase still counts and one
    /// after it does not ([`Space::credit_kills`]).
    ///
    /// The original's `_DamageShip` settles each hit as it lands; here a
    /// phase's hits settle together after it, before the next phase or
    /// the frame. The fight itself already clears a lost escort's leader
    /// as its hit lands (see the fight's `escort_loss` module), so the
    /// rest of the phase reads it unled; only the session's part of the
    /// loss waits for the phase's end.
    fn settle_landed(
        &mut self,
        was: &[NpcBefore],
        (player_was, killed): (Condition, Option<Condition>),
        rules: Rules,
        chance: &mut dyn Chance,
    ) {
        let (strikes, losses, finished) = self.space.combat.take_landed();
        // `_HandleShipHit`'s victories, as the hits landed, before the
        // answers retarget anyone (see the `voices` module).
        self.space
            .speak_victories(&self.statics, &finished, &losses);
        let struck = super::struck_npcs(&strikes);
        self.player_downed(player_was, killed, &strikes, chance);
        self.take_shoves();
        self.raise_disabled(was, chance);
        self.space.sync_fleet(&mut self.pilot);
        self.lose_escorts(was, &struck);
        self.settle_strikes((strikes, losses), was, rules.law, chance);
    }

    /// Runs `phase`, one that lands hits, as [`Session::with_fighters`]
    /// does, then settles them ([`Session::settle_landed`]).
    pub(super) fn land_phase(
        &mut self,
        pass: GamePass,
        rules: Rules,
        chance: &mut dyn Chance,
        phase: impl FnOnce(&mut Combat, &mut [Fighter], Arena<'_>, &mut dyn Chance),
    ) {
        let was = self.space.npcs_before();
        let player_was = self.ship.condition;
        self.with_fighters(pass, rules, chance, phase);
        self.settle_landed(&was, (player_was, None), rules, chance);
    }

    /// The pass's stellar hazards and its `_HandleBeams` on `pass` by
    /// `rules`, drawing on `chance`: gravity pulls and the deadly stellars
    /// kill (`_HandleGravity` @0x44509, `_HandleDeadlyStellars` @0x4450e;
    /// [`Combat::hazards`]), then the beams' step and the late steps
    /// ([`Combat::close_tick`]), among one set of fighters, so a ship a
    /// deadly stellar kills is out of the beams. The kills' hold starts
    /// before the beams' hits settle (see the `deadly_stellars` module),
    /// as a phase that lands hits settles ([`Session::settle_landed`]).
    ///
    /// The asteroid field is lent to the beams for the phase
    /// ([`Combat::close_tick_in`]), which cut, wear down, break, push and
    /// tractor its asteroids, destroying each at once; the boxes of each
    /// one destroyed are spawned after it (see the `asteroid_beams`
    /// module).
    pub(super) fn beams_phase(&mut self, pass: GamePass, rules: Rules, chance: &mut dyn Chance) {
        let hazards = self.stellar_hazards();
        let was = self.space.npcs_before();
        let player_was = self.ship.condition;
        let types = std::rc::Rc::clone(&self.statics.asteroid_types);
        let mut asteroids = std::mem::take(&mut self.space.asteroids);
        self.with_fighters(pass, rules, chance, |combat, fighters, arena, chance| {
            combat.hazards(fighters, &hazards);
            let field = AsteroidField {
                asteroids: &mut asteroids,
                types: &types,
            };
            combat.close_tick_in(fighters, arena, Some(field), chance);
        });
        self.space.asteroids = asteroids;
        self.space.settle_asteroid_remains(&mut *chance);
        let killed = self.hold_stellar_kills();
        self.settle_landed(&was, (player_was, killed), rules, chance);
    }

    /// The fight's collision passes on `pass` by `rules`, drawing on
    /// `chance` ([`Combat::test_hits`]), among the stellars shown that
    /// shots can hit and the asteroid field, lent whole, then their
    /// settling: the hits' ([`Session::settle_landed`]), each stellar shot
    /// down running its `OnDestroy` (see the `stellars` module), and the
    /// boxes of each asteroid the shots destroyed (see the `asteroid_hits`
    /// module).
    pub(super) fn collision_passes(
        &mut self,
        pass: GamePass,
        rules: Rules,
        chance: &mut dyn Chance,
    ) {
        let mut stellars = self.stellar_targets();
        let was = self.space.npcs_before();
        let player_was = self.ship.condition;
        let types = std::rc::Rc::clone(&self.statics.asteroid_types);
        let mut asteroids = std::mem::take(&mut self.space.asteroids);
        self.with_fighters(pass, rules, chance, |combat, fighters, arena, chance| {
            let scenery = Scenery {
                stellars: &mut stellars,
                asteroids: Some(AsteroidField {
                    asteroids: &mut asteroids,
                    types: &types,
                }),
                hazards: None,
            };
            combat.test_hits(fighters, arena, scenery, chance);
        });
        self.space.asteroids = asteroids;
        self.settle_landed(&was, (player_was, None), rules, chance);
        self.strike_stellars(&stellars, rules.law, chance);
        self.space.settle_asteroid_remains(&mut *chance);
    }

    /// What a pass's fight leaves, settled once after its last phase, by
    /// `rules`, drawing on `chance`: each NPC's exit cursors kept for its
    /// berth; the fighters launched from bays put in the system (see the
    /// `fighters` module); the words the pass's stray beams stepped; the
    /// fleet kept in step; the escorts whose ship is gone lost; the NPCs'
    /// self-repair; each NPC a deadly stellar killed vanished or destroyed
    /// (see the `deadly_stellars` module), then each NPC destroyed taken
    /// out; the
    /// fighters orphaned; a target gone let go; the stellars seen; the
    /// ships laid out and their frames displayed (see the `ship_display`
    /// module); and the heading each ship is drawn at noted, which its
    /// next tick's shots leave from.
    /// Each dying NPC's escape pods and escape-ship roll ran before it, in
    /// its own turn of the ship loop ([`Session::ship_loop`]) or, in the
    /// separate parts, after the beams (`Session::fight`; see the
    /// `npc_escape` module).
    pub(super) fn fight_tail(&mut self, rules: Rules, chance: &mut dyn Chance) {
        self.space.traffic.keep_berth_exits();
        let sorties = self.space.combat.take_sorties();
        self.launch_fighters(&sorties, chance);
        // The words the pass's stray beams stepped (`_HandleBeams`' among
        // them, after the ship loop). The original steps at the beam's
        // spawn; settling here, in the same tick before the frame is
        // drawn, crosses no boundary (see the `stray_words` module). None
        // is left when nothing stepped one since the ship loop.
        self.step_stray_words(chance);
        self.space.sync_fleet(&mut self.pilot);
        // Each phase lost the escorts its hits downed; what is left is
        // those whose ship is gone.
        self.lose_escorts(&[], &std::collections::BTreeSet::new());
        self.lose_dying_mission_escorts();
        self.self_repair(rules, &mut *chance);
        self.take_out_the_killed(chance);
        self.space
            .lose_leads(self.setup.rules.source_for(crate::RuleKey::NpcEscortAi));
        self.let_go_of_gone_target();
        self.see_stellars();
        self.space.hold_derelicts(&self.statics);
        self.lay_out_ships();
        self.display_ships(&mut *chance);
        self.note_drawn();
    }
}

impl Space {
    /// Every NPC as it is now, for the strikes that follow to be read
    /// against.
    fn npcs_before(&self) -> Vec<NpcBefore> {
        self.traffic.npcs().iter().map(NpcBefore::of).collect()
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::RefCell;

    use crate::ai::{Behaviour, Decision, Goal, NovaAi, Reaction, Surroundings};
    use crate::catalog::WeaponRecord;
    use crate::chance::{Chance, NeverFires};
    use crate::combat::armament::Trigger;
    use crate::combat::hull::Condition;
    use crate::combat::{Rules, ShipRef, Strike};
    use crate::flight::{Controls, Turn};
    use crate::geometry::Vec2;
    use crate::session::Session;
    use crate::session::tests::{FIRE, armed, blaster, facing_an_npc};
    use crate::story::{Fate, StoryEvent};
    use crate::testkit::FakePilotCatalog;
    use crate::traffic::npc::{Mode, Npc, NpcId};

    /// What a decision saw of an NPC: whether it was provoked, the ship it
    /// targeted, and the damage aimed at it.
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Seen {
        id: NpcId,
        provoked: bool,
        target: Option<ShipRef>,
        targeted: Option<i32>,
    }

    /// Nova's AI, recording what each decision saw.
    #[derive(Debug, Default)]
    struct Recording {
        ai: NovaAi,
        seen: RefCell<Vec<Seen>>,
    }

    impl Recording {
        fn take(&self) -> Vec<Seen> {
            std::mem::take(&mut self.seen.borrow_mut())
        }
    }

    impl Behaviour for Recording {
        fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
            self.ai.decide(npc, around, chance)
        }

        fn decision(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Decision {
            self.seen.borrow_mut().push(Seen {
                id: npc.id,
                provoked: npc.provoked > 0.0,
                target: npc.target,
                targeted: around.targeted.get(&ShipRef::Npc(npc.id)).copied(),
            });
            self.ai.decision(npc, around, chance)
        }

        fn trigger(&self, npc: &Npc, around: &Surroundings) -> Trigger {
            self.ai.trigger(npc, around)
        }

        fn target(&self, npc: &Npc, around: &Surroundings) -> Option<ShipRef> {
            self.ai.target(npc, around)
        }

        fn react(&self, npc: &Npc, strike: &Strike, around: &Surroundings) -> Reaction {
            self.ai.react(npc, strike, around)
        }
    }

    /// A pass of `session` in flight under `controls`, by Nova's rules,
    /// its NPCs deciding as `behaviour` says.
    fn pass(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        controls: Controls,
        behaviour: &impl Behaviour,
    ) {
        let rescued = session.game_work(
            controls,
            catalog,
            behaviour,
            Rules::default(),
            &mut NeverFires,
        );
        assert!(!rescued);
    }

    /// [`facing_an_npc`], after a pass that sets the player's speed box
    /// (see the `afterburner` module), the player at rest.
    fn warmed(catalog: &FakePilotCatalog) -> Session {
        let mut session = facing_an_npc(catalog, 180);
        pass(
            &mut session,
            catalog,
            Controls::default(),
            &Recording::default(),
        );
        assert_eq!(session.player().velocity, Vec2::ZERO);
        session
    }

    /// Runs passes of `session`, the player's trigger held on the first
    /// alone, until NPC 0's shield first drops: what its decisions saw on
    /// that pass, and how many passes ran.
    fn until_hit(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        behaviour: &Recording,
    ) -> (Vec<Seen>, u32) {
        session.hold_trigger(FIRE);
        for passes in 1..=40 {
            pass(session, catalog, Controls::default(), behaviour);
            session.hold_trigger(Trigger::default());
            let seen = behaviour.take();
            if session.npcs()[0].reserves.shield.now < 30.0 {
                return (seen, passes);
            }
        }
        panic!("never hit");
    }

    #[test]
    fn an_npc_struck_by_the_players_shot_has_answered_it_when_that_ticks_decision_runs() {
        // The collision passes (@0x43e70-0x44015) and `_DamageShip`'s
        // reaction block come before `_AIDispatch` (@0x4429d).
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        // Aimed at it: a stray hit the answer flag forgives.
        session.chart.target = Some(NpcId(0));
        let behaviour = Recording::default();
        let (seen, _) = until_hit(&mut session, &catalog, &behaviour);
        let npc = seen
            .iter()
            .find(|seen| seen.id == NpcId(0))
            .expect("decided");
        assert!(npc.provoked, "{npc:?}");
        assert_eq!(npc.target, Some(ShipRef::Player));
    }

    #[test]
    fn a_ship_the_player_hits_in_its_gate_is_held_there_before_that_ticks_motion() {
        // The hit's answer holds it at the gate for 20 ticks
        // (`_DamageShip` @0x3be6e-0x3be7c), and that pass's `_HandleShip`
        // (@0x44483) counts the hold down.
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        let npc = &mut session.space.traffic.npcs_mut()[0];
        npc.mode = Mode::Emerging { ticks_left: 60 };
        npc.ai_type = crate::traffic::npc::AiType::Warship;
        // Emerging, it is no target the pass's head keeps (`_HandlePlayer`
        // @0x686a4), so the hit is a stray one: provoked past forgiving
        // through the passes before it.
        npc.stray = 2.0 * crate::ai::answer_flag::STRAY_THRESHOLD;
        let behaviour = Recording::default();
        until_hit(&mut session, &catalog, &behaviour);
        assert_eq!(session.npcs()[0].mode, Mode::HeldAtGate { ticks_left: 19 });
    }

    #[test]
    fn an_npc_pushed_by_a_shot_moves_by_the_push_in_the_tick_it_lands() {
        // The push lands in its velocity (`_DamageShip` @0x3a9b8-0x3ab84)
        // before `_HandleShip`'s position step (@0x34274-0x342aa).
        for (impact, pushed) in [(0, false), (2000, true)] {
            let catalog = FakePilotCatalog {
                weapons: vec![WeaponRecord {
                    impact,
                    ..blaster()
                }],
                ..armed()
            };
            let mut session = facing_an_npc(&catalog, 180);
            // Its class, of no mass in the fixture, given one.
            session.space.traffic.npcs_mut()[0].hull.mass = 40.0;
            let start = session.npcs()[0].state.position;
            let behaviour = Recording::default();
            until_hit(&mut session, &catalog, &behaviour);
            let moved = session.npcs()[0].state.position - start;
            assert_eq!(moved.y < 0.0, pushed, "{impact}: {moved:?}");
            if !pushed {
                assert_eq!(moved, Vec2::ZERO, "at rest");
            }
        }
    }

    #[test]
    fn the_damage_the_ai_reads_as_aimed_counts_no_shot_the_hit_pass_took() {
        // `_CalculateTargetedDamage` (@0x44223) runs after the collision
        // passes: the shot that hit, life -1 (@0x36f4f), counts for none.
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        session.chart.target = Some(NpcId(0));
        let behaviour = Recording::default();
        session.hold_trigger(FIRE);
        let mut flying = Vec::new();
        let seen = loop {
            pass(&mut session, &catalog, Controls::default(), &behaviour);
            session.hold_trigger(Trigger::default());
            let seen = behaviour.take();
            if session.npcs()[0].reserves.shield.now < 30.0 {
                break seen;
            }
            flying.extend(seen);
            assert!(flying.len() < 40, "never hit");
        };
        assert!(
            flying.iter().all(|seen| seen.targeted.is_some()),
            "in flight: {flying:?}"
        );
        assert_eq!(seen[0].targeted, None, "the shot the pass took");
    }

    #[test]
    fn the_players_shot_leaves_from_where_it_stood_before_the_ticks_move() {
        // `_HandlePlayer` fires (@0x6ab55) before its position step
        // (@0x6d4b0-0x6d4eb).
        let catalog = armed();
        let mut session = warmed(&catalog);
        session.ship.player.velocity = Vec2::new(3.0, 0.0);
        let stood = session.player().position;
        session.hold_trigger(FIRE);
        pass(
            &mut session,
            &catalog,
            Controls::default(),
            &Recording::default(),
        );
        assert_eq!(session.player().position, stood + Vec2::new(3.0, 0.0));
        let shot = session
            .shots()
            .iter()
            .find(|shot| shot.firer == ShipRef::Player)
            .expect("a shot");
        assert_eq!(shot.moved_from, Some(stood));
    }

    #[test]
    fn the_players_recoil_moves_it_in_the_tick_it_fires() {
        // The recoil goes into the velocity (`_AdjustedAccel` @0x62f35)
        // before the turn, thrust and position step (@0x6c770-0x6d4eb).
        let catalog = FakePilotCatalog {
            weapons: vec![WeaponRecord {
                recoil: 100,
                ..blaster()
            }],
            ..armed()
        };
        let mut session = warmed(&catalog);
        // The player's class, of no mass in the fixture, given one.
        session.ship.fit.hull.mass = 40.0;
        let stood = session.player().position;
        session.hold_trigger(FIRE);
        pass(
            &mut session,
            &catalog,
            Controls::default(),
            &Recording::default(),
        );
        let velocity = session.player().velocity;
        assert!(velocity.y > 0.0, "pushed back, down: {velocity:?}");
        assert_eq!(session.player().position, stood + velocity);
    }

    #[test]
    fn the_player_fires_along_its_heading_before_the_ticks_turn() {
        // The trigger (@0x6ab55) comes before the turn (@0x6c770-0x6c8d3).
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        let heading = session.player().heading;
        session.hold_trigger(FIRE);
        let turning = Controls {
            turn: Turn::Left,
            ..Controls::default()
        };
        pass(&mut session, &catalog, turning, &Recording::default());
        assert_ne!(session.player().heading, heading, "turned");
        let shot = session
            .shots()
            .iter()
            .find(|shot| shot.firer == ShipRef::Player)
            .expect("a shot");
        assert_eq!(shot.heading, heading);
    }

    /// The ship fates raised and not yet settled.
    fn fates(session: &crate::session::Session) -> Vec<StoryEvent> {
        session
            .story_raised()
            .into_iter()
            .filter(|event| matches!(event, StoryEvent::ShipFate { .. }))
            .collect()
    }

    /// Asserts three passes of `session` match the separate parts.
    fn runs_the_parts(mut session: Session, catalog: &FakePilotCatalog) {
        let mut by_parts = session.clone();
        for _ in 0..3 {
            pass(
                &mut session,
                catalog,
                Controls::default(),
                &NovaAi::default(),
            );
            by_parts.tick(Controls::default());
            assert!(!by_parts.rescue_if_due(catalog, &mut NeverFires));
            by_parts.tick_traffic(catalog, &NovaAi::default(), &mut NeverFires);
            by_parts.tick_combat(Rules::default(), &mut NeverFires);
            assert_eq!(session, by_parts);
        }
    }

    #[test]
    fn a_landed_pass_runs_the_parts_as_before() {
        // Landed, the fight stands still; through a jump the world runs
        // on in the pass's own order (see the `warp_stage` module).
        let catalog = armed();
        let mut landed = facing_an_npc(&catalog, 180);
        crate::testkit::land_now(&mut landed).expect("lands");
        runs_the_parts(landed, &catalog);
    }

    /// [`armed`] with weapon 128 a 200-pixel beam lasting 3 ticks, firing
    /// every 10, doing 5 energy damage.
    fn beam_armed() -> FakePilotCatalog {
        FakePilotCatalog {
            weapons: vec![WeaponRecord {
                guidance: 0,
                count: 3,
                reload: 10,
                speed: 0,
                beam_length: 200,
                mass_dmg: 0,
                energy_dmg: 5,
                ..blaster()
            }],
            ..armed()
        }
    }

    #[test]
    fn each_ship_regenerates_before_the_passs_beams() {
        // `_HandlePlayer` regenerates the player's shield (@0x6cf86) and
        // each NPC's own `_HandleShip` its (@0x34424, the ship loop
        // @0x4445c), both before `_HandleBeams` (@0x4453d) lands the
        // beams fired this pass: 0.5 short, up to full, less 5.
        let catalog = beam_armed();
        let mut session = facing_an_npc(&catalog, 180);
        session.ship.fit.stats.shield_regen = 1.0;
        let full = session.reserves().shield.max;
        session.pilot.reserves.shield.now = full - 0.5;
        let npc = &mut session.space.traffic.npcs_mut()[0];
        npc.stats.shield_regen = 1.0;
        let npc_full = npc.reserves.shield.max;
        npc.reserves.shield.now = npc_full - 0.5;
        session.hold_trigger(FIRE);
        pass(
            &mut session,
            &catalog,
            Controls::default(),
            &crate::session::tests::Firing,
        );
        assert_eq!(session.npcs()[0].reserves.shield.now, npc_full - 5.0);
        assert_eq!(session.reserves().shield.now, full - 5.0);
    }

    #[test]
    fn an_npc_whose_fade_crosses_into_its_cloak_this_pass_still_fires_its_pick() {
        // `_HandleShipDisplay` steps the fade (@0x2d088-0x2d164) after the
        // NPC's `_HandleShip` has fired its pick (@0x34ffd, @0x444ad): at
        // 15.5 rising by 0.75, its blaster (no `Flags2` 0x0100) fires, and
        // the ship reads cloaked after.
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        session.space.traffic.npcs_mut()[0].cloak = crate::cloak::Cloak {
            fade: 15.5,
            direction: 1,
        };
        session.take_combat_events();
        pass(
            &mut session,
            &catalog,
            Controls::default(),
            &crate::session::tests::Firing,
        );
        let fired = session.take_combat_events().iter().any(|event| {
            matches!(
                event,
                crate::combat::CombatEvent::Fired {
                    ship: ShipRef::Npc(NpcId(0)),
                    ..
                }
            )
        });
        assert!(fired, "its pick");
        let cloak = session.npcs()[0].cloak;
        assert_eq!(cloak.fade, 16.25);
        assert!(cloak.is_cloaked(), "cloaked after");
    }

    #[test]
    fn a_landed_fight_steps_no_fade() {
        // Landed, no ship loop runs, and no display steps a fade.
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        crate::testkit::land_now(&mut session).expect("lands");
        let mut npc = crate::testkit::npc(0, crate::stats::ShipStats::default());
        npc.cloak = crate::cloak::Cloak {
            fade: 4.0,
            direction: 1,
        };
        let id = session.space.traffic.add_npc(npc);
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(session.space.npc(id).expect("there").cloak.fade, 4.0);
    }

    #[test]
    fn the_warps_lead_in_still_steps_each_npcs_fade() {
        // The traffic flies the lead-in, and each NPC's display steps its
        // fade (@0x2d088), though the fight stands still.
        let catalog = crate::testkit::catalog();
        let mut session = crate::session::tests::bound_for_131(
            &catalog,
            crate::session::tests::OUT,
            Vec2::ZERO,
            90.0,
        );
        let mut npc = crate::testkit::npc(0, crate::stats::ShipStats::default());
        npc.cloak = crate::cloak::Cloak {
            fade: 4.0,
            direction: 1,
        };
        let id = session.space.traffic.add_npc(npc);
        session.begin_jump().expect("jumps");
        assert!(session.in_warp_lead_in());
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(session.space.npc(id).expect("there").cloak.fade, 4.75);
    }

    #[test]
    fn a_ship_disabled_by_the_hit_pass_raises_its_fate_once() {
        // Each phase settles its own hits, against the NPCs as the phase
        // found them: the disabling raises its fate after the collision
        // passes, and no later phase raises it again.
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        session.space.traffic.npcs_mut()[0].reserves.shield.now = 0.0;
        session.space.traffic.npcs_mut()[0].reserves.armor.now = 16.0;
        session.hold_trigger(FIRE);
        let mut ticks = 0;
        while session.npcs()[0].condition == Condition::Intact {
            session.tick_combat(Rules::default(), &mut NeverFires);
            session.hold_trigger(Trigger::default());
            ticks += 1;
            assert!(ticks < 20, "never disabled");
        }
        let disabled = StoryEvent::ShipFate {
            ship: NpcId(0),
            fate: Fate::Disabled,
        };
        assert_eq!(fates(&session), [disabled]);
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(fates(&session), [disabled], "once");
    }

    /// Every NPC keeps its goal.
    #[derive(Debug)]
    struct Keeping;

    impl Behaviour for Keeping {
        fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            npc.goal
        }
    }

    /// [`facing_an_npc`] by `rules`, NPC 0 a lead drifting right at 0.3
    /// holding its course, and a fleet escort of it 250 to its right at
    /// rest, after a pass: the lead and the escort.
    fn fleet_after_a_pass(rules: crate::rulebook::Rulebook) -> (Npc, Npc) {
        let catalog = armed();
        let mut session = warmed(&catalog).with_rules(rules);
        let lead = NpcId(0);
        let npc = &mut session.space.traffic.npcs_mut()[0];
        npc.goal = Goal::Hold;
        npc.state.velocity = Vec2::new(0.3, 0.0);
        let mut escort = npc.clone();
        escort.leader = Some(lead);
        escort.goal = Goal::Follow { lead, guard: None };
        escort.state.position.x += 250.0;
        escort.state.velocity = Vec2::ZERO;
        let escort = session.space.traffic.add_npc(escort);
        pass(&mut session, &catalog, Controls::default(), &Keeping);
        let ship = |id| session.space.npc(id).expect("there").clone();
        (ship(lead), ship(escort))
    }

    #[test]
    fn a_pass_gives_a_fleet_escort_its_slot_and_flies_it_in_formation_by_the_rulebook() {
        use crate::rulebook::{RuleKey, RuleSource, Rulebook};
        // Slot 2 of its lead's formation, by either reading.
        let slot = |escort: &Npc| escort.fleet_slot.map(|slot| (slot.slot, slot.ships));
        // Within 300 of its lead and within 0.525 of its velocity: matched,
        // it takes the lead's velocity (sub 0xc).
        let (lead, escort) = fleet_after_a_pass(Rulebook::default());
        assert_eq!(slot(&escort), Some((2, 2)));
        assert_eq!(escort.state.velocity, lead.state.velocity);
        // By the earlier follow it closes on its lead from 250 off.
        let earlier = Rulebook::default().with_override(RuleKey::NpcFormation, RuleSource::Bible);
        let (lead, escort) = fleet_after_a_pass(earlier);
        assert_eq!(slot(&escort), Some((2, 2)));
        assert_ne!(escort.state.velocity, lead.state.velocity);
    }
}
