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
//!    fighters (@0x62c61) settled before it moves
//!    ([`Session::player_fires`]).
//! 2. The move, the turn, thrust and position step (@0x6c770-0x6d4eb),
//!    or the escape pod's flight; the rescue, which ends the pass
//!    (@0x68ddd); then the eject check (@0x6f22c).
//! 3. The collision passes (@0x43e70-0x44015), their hits settled and
//!    answered at once, as `_DamageShip`'s reaction block runs in them.
//! 4. The advance pass's AI (@0x44223-0x442ae): the traffic's think half,
//!    the damage aimed read after the passes (`_CalculateTargetedDamage`
//!    @0x44223), the arrivals (`_EnterMoreShips` @0x44237), the scans,
//!    then every NPC's point defence (`_AIFireTurret` @0x80b0d).
//! 5. `_HandleShot` (@0x44385): the ends of life, the moves and the
//!    locks, their hits settled and answered.
//! 6. `_HandleShip` (@0x44483): the traffic's fly half and the session's
//!    work after it, the assistance, then each NPC's fire.
//! 7. `_HandleBeams` (@0x4453d) and the fight's late steps, then what the
//!    pass leaves ([`Session::fight_tail`]), then the asteroids
//!    (`_SpawnAsteroid` @0x44243 and `_HandleAsteroids` @0x444f5, after
//!    the passes).
//!
//! Landed, or with the jump under way as the pass begins or once its move
//! begins it, the pass runs the separate parts instead, as the fight
//! stands still. Double time's second pass ([`Session::tick_again`]) runs
//! the same order without the AI: from the eject check on, both passes
//! run one routine ([`Session::pass_after_move`]), which branches where
//! `_DoPlayGameWork` branches on its argument (@0x44026-0x4402a).
//!
//! **What remains**, none crossing a boundary the player sees, so none
//! filed:
//!
//! - Every NPC's point defence runs after every NPC's decision, where
//!   `_AIDispatch` interleaves them slot by slot.
//! - What the close strikes (the beams, the shockwaves) is answered by the
//!   caller after the pass ([`Session::answer_strikes`]), before the frame
//!   is drawn.
//! - The separate parts ([`Session::tick`], [`Session::tick_traffic`],
//!   [`Session::tick_combat`]) keep the old order between the player, the
//!   traffic and the fight, the fight's own phases in its new order; they
//!   are no game path, and the tests that step through them are to step
//!   through [`Session::game_work`] (rdm task
//!   `retire-the-sessions-separate-pass-parts`).
//!
//! **The phases.** Each builds the fight's [`Fighter`]s afresh from the
//! session ([`Session::with_fighters`]): the player, then every NPC, each
//! on the terms the session works out for it. A phase that lands hits is
//! settled at once ([`Session::settle_landed`]): the player's downing
//! lines, the pushes, the NPCs' fates, the strikes' answers, crimes and
//! kills, and a target lost. What the whole pass leaves is settled once,
//! after its last phase ([`Session::fight_tail`]).

use super::{FightTerms, NpcBefore, Session, TrafficMarks};
use crate::ai::Behaviour;
use crate::catalog::{PilotCatalog, TrafficCatalog};
use crate::chance::Chance;
use crate::combat::armament::{OutfitRounds, Trigger};
use crate::combat::hull::Condition;
use crate::combat::{Arena, Combat, Fighter, GamePass, Rules, Scenery, ShipRef};
use crate::flight::Controls;

/// What a pass's fight reads of the session as it began: whether the
/// player was destroyed or dying, and every NPC.
pub(super) struct PassStart {
    /// Whether the player's ship was destroyed.
    was_destroyed: bool,
    /// Whether the player's ship was breaking up.
    was_dying: bool,
    /// Every NPC.
    npcs: Vec<NpcBefore>,
}

impl PassStart {
    /// `session` as its pass's fight begins.
    pub(super) fn of(session: &Session) -> Self {
        Self {
            was_destroyed: session.condition == Condition::Destroyed,
            was_dying: matches!(session.condition, Condition::Dying { .. }),
            npcs: session.npcs_before(),
        }
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
    /// Landed, or with the jump under way as the pass begins or once the
    /// move has begun it, it runs the separate parts instead
    /// ([`Session::tick`], [`Session::rescue_if_due`],
    /// [`Session::tick_traffic`], [`Session::tick_assistance`] and
    /// [`Session::tick_combat`]), as the fight stands still then and the
    /// warp's lead-in traffic is the traffic tick's.
    pub fn game_work(
        &mut self,
        controls: Controls,
        catalog: &(impl PilotCatalog + TrafficCatalog),
        behaviour: &(impl Behaviour + ?Sized),
        rules: Rules,
        chance: &mut (impl Chance + ?Sized),
    ) -> bool {
        if self.landed.is_some() || self.jumping().is_some() {
            self.tick(controls);
            if self.rescue_if_due(catalog, chance) {
                return true;
            }
            self.tick_traffic(catalog, behaviour, chance);
            self.tick_assistance(rules.disable);
            self.tick_combat(rules, chance);
            return false;
        }
        let mut source = &mut *chance;
        let chance: &mut dyn Chance = &mut source;
        // `_HandlePlayer` (@0x43e19), up to its move.
        self.gate = None;
        let frame = self.pass_head();
        let pass = GamePass::Advance {
            double_time: self.double_time,
        };
        self.before_move(frame, pass);
        self.player_exploded_this_tick = false;
        let start = PassStart::of(self);
        self.player_fires(pass, rules, chance);
        self.player_move(controls, pass);
        if self.jumping().is_some() {
            // The jump began in the move: the rest stands still.
            self.tick_traffic(catalog, behaviour, chance);
            self.tick_assistance(rules.disable);
            self.tick_combat(rules, chance);
            return false;
        }
        if self.rescue_if_due(catalog, chance) {
            return true;
        }
        self.pass_after_move(&start, pass, Some(catalog), behaviour, rules, chance);
        false
    }

    /// The rest of a pass of the game's work after the player's move, on
    /// `pass` by `rules`, against the session as the pass `start`ed, the
    /// NPCs answering its hits as `behaviour` says, drawing on `chance`:
    /// both passes run it, [`Session::game_work`] and
    /// [`Session::tick_again`], as the original runs one routine,
    /// `_DoPlayGameWork(pass)`, for both (see the module docs).
    ///
    /// `ai` is the advance pass's AI, its traffic drawn from the catalog
    /// it holds; the second pass has none, as `_DoPlayGameWork(0)` jumps
    /// past it (@0x4402a to @0x44350). Where the original branches on its
    /// argument, this branches on `ai`: with it, the traffic thinks, the
    /// scans settle and every NPC's point defence fires before the shots,
    /// and the traffic's motion is its fly half, its reinforcement clock
    /// included ([`Session::traffic_tail`]), then the assistance; without
    /// it, the traffic only moves, and the reinforcement clock
    /// (`_HandleReinforcements` @0x44504) ticks after the asteroids.
    pub(super) fn pass_after_move(
        &mut self,
        start: &PassStart,
        pass: GamePass,
        ai: Option<&dyn TrafficCatalog>,
        behaviour: &(impl Behaviour + ?Sized),
        rules: Rules,
        chance: &mut dyn Chance,
    ) {
        self.eject_before_fight(chance);
        // The collision passes (@0x43e70-0x44015).
        self.collision_passes(pass, rules, chance);
        self.answer_strikes(behaviour);
        // The advance pass's AI (@0x44223-0x442ae).
        let thought = if let Some(catalog) = ai {
            let marks = self.traffic_think(catalog, behaviour, chance);
            self.take_scans(rules.law, chance);
            self.with_fighters(pass, rules, chance, |combat, fighters, arena, chance| {
                combat.defend_npcs(fighters, arena, chance);
            });
            Some(marks)
        } else {
            None
        };
        // `_HandleShot` (@0x44385).
        self.land_phase(pass, rules, chance, |combat, fighters, arena, chance| {
            combat.fly_shots(fighters, arena, chance);
        });
        self.answer_strikes(behaviour);
        // `_HandleShip` (@0x44483).
        if let Some(marks) = thought {
            self.traffic_motion(marks, chance);
            self.tick_assistance(rules.disable);
        } else {
            self.move_traffic_again(chance);
        }
        self.npcs_fire(pass, rules, chance);
        // `_HandleBeams` (@0x4453d), and what the pass leaves.
        self.land_phase(pass, rules, chance, |combat, fighters, arena, chance| {
            combat.close_tick(fighters, arena, chance);
        });
        self.fight_tail(start, pass, rules, chance);
        // After the shots, as `_SpawnAsteroid` and `_HandleAsteroids`
        // come after the collision passes (see the `asteroids` module).
        self.tick_asteroids(chance);
        if ai.is_none() {
            self.tick_reinforcements(chance);
        }
    }

    /// `_HandlePlayer`'s fight before its move, on `pass` by `rules`,
    /// drawing on `chance`: the repair inhibit's countdown and the
    /// disabled player's self-repair (@0x68b3d-0x68c6c; see the
    /// `player_repair` module), its point defence (@0x68c74,
    /// [`Combat::defend_player`]), the sabotage clock's tick (@0x692e5;
    /// see the `sabotage` module), its trigger and its reloads' countdown
    /// (@0x6ab55, @0x6ae25, [`Combat::fire_player`]); then its recoil set
    /// in its velocity (`_AdjustedAccel` @0x62f35) and the fighters its bays
    /// launched put in the system (`_LaunchFighter` @0x62c61), all before
    /// it moves. So it fires from where it stood, along the heading and at
    /// the velocity the last pass left, and a sabotage that downs it still
    /// lets its point defence fire that pass.
    pub(super) fn player_fires(&mut self, pass: GamePass, rules: Rules, chance: &mut dyn Chance) {
        self.player_repair_tick(rules, chance);
        self.with_fighters(pass, rules, chance, |combat, fighters, arena, chance| {
            combat.defend_player(fighters, arena, chance);
        });
        self.sabotage_tick(rules, chance);
        self.with_fighters(pass, rules, chance, |combat, fighters, arena, chance| {
            combat.fire_player(fighters, arena, chance);
        });
        self.take_shoves();
        let sorties = self.combat.take_sorties();
        self.launch_fighters(&sorties, chance);
    }

    /// `_HandleShip`'s fire on `pass` by `rules`, drawing on `chance`:
    /// every NPC's stray provocation eases (@0x35374-0x353b2), after the
    /// collision passes and the shots and before the beams, then each
    /// NPC's reloads count down and it fires its pick
    /// ([`Combat::fire_npcs`]), and the pick fired is cleared or kept
    /// ([`Traffic::after_firing`](crate::traffic::Traffic::after_firing)).
    pub(super) fn npcs_fire(&mut self, pass: GamePass, rules: Rules, chance: &mut dyn Chance) {
        self.traffic.ease_stray();
        self.with_fighters(pass, rules, chance, |combat, fighters, arena, chance| {
            combat.fire_npcs(fighters, arena, chance);
        });
        self.traffic.after_firing(&self.arsenal);
    }

    /// The advance pass's AI, reading `catalog`, deciding as `behaviour`
    /// says and drawing on `chance`: on the first pass in a system not
    /// yet set up, its population ([`Session::populate`]), which moves
    /// from the next; otherwise the session's work before the AI, the
    /// traffic's think half ([`Traffic::think_holding`]), and the fleets
    /// and comm calls it brings. What its motion is read against, when it
    /// thought.
    ///
    /// [`Traffic::think_holding`]: crate::traffic::Traffic::think_holding
    fn traffic_think(
        &mut self,
        catalog: &(impl TrafficCatalog + ?Sized),
        behaviour: &(impl Behaviour + ?Sized),
        chance: &mut dyn Chance,
    ) -> Option<TrafficMarks> {
        if self.traffic_due {
            self.populate(catalog, chance);
            return None;
        }
        let (strikes, marks) = self.before_traffic_ai(catalog, chance);
        self.step_traffic(|traffic, world| {
            traffic.think_holding(behaviour, world, &strikes, chance);
        });
        self.after_traffic_ai(chance);
        Some(marks)
    }

    /// The traffic's motion, read against the `marks` its think left,
    /// drawing on `chance`: its fly half
    /// ([`Traffic::fly_holding`](crate::traffic::Traffic::fly_holding)) and
    /// the session's work after it, when it thought; then the end of the
    /// traffic tick ([`Session::traffic_tail`]).
    fn traffic_motion(&mut self, marks: Option<TrafficMarks>, chance: &mut dyn Chance) {
        if let Some(marks) = marks {
            self.step_traffic(|traffic, world| traffic.fly_holding(world, &marks.still, chance));
            self.after_traffic_motion(&marks, chance);
        }
        self.traffic_tail(chance);
    }

    /// Every NPC as it is now, for the strikes that follow to be read
    /// against.
    fn npcs_before(&self) -> Vec<NpcBefore> {
        self.traffic.npcs().iter().map(NpcBefore::of).collect()
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
        let armor_regen = self.player_armor_regen();
        // This tick's `_abOn`, as `_HandlePlayer` runs before the shots
        // (see the `afterburner` module).
        let afterburner = self.afterburner_knockback();
        let pilot = &mut self.pilot;
        let mut rounds = OutfitRounds {
            owned: &mut pilot.outfits,
            sources: &self.ammo_outfits,
        };
        let mut fighters = vec![Fighter {
            ship: ShipRef::Player,
            ship_type: pilot.ship,
            fleet: ShipRef::Player,
            govt: None,
            leader: None,
            booty: None,
            state: self.player,
            hull: self.hull,
            shield_regen: self.stats.shield_regen,
            armor_regen,
            trigger: if self.pod.is_some() {
                Trigger::default()
            } else {
                super::player_trigger(self.trigger, self.jump.is_some())
            },
            target: self.target.map(ShipRef::Npc),
            reserves: &mut pilot.reserves,
            condition: &mut self.condition,
            held_disabled: false,
            cloaked: false,
            attacking: None,
            threatens_player: false,
            plundering: false,
            emerging: false,
            cloak_scanner: scanner.on_screen,
            has_cloak: false,
            jamming: player_terms.jamming,
            class_speed: player_terms.class_speed,
            top_speed: player_terms.top_speed,
            jumping: self.jump.is_some(),
            afterburner,
            tractor: &mut self.tractor,
            ion: &mut self.ion,
            armament: &mut self.armament,
            rounds: &mut rounds,
            drawn_heading: player_terms.drawn_heading,
            docked: false,
            sprite: player_terms.sprite,
        }];
        let npcs = self.traffic.npcs_mut().iter_mut().zip(terms);
        fighters.extend(npcs.map(|(npc, terms)| terms.fighter(npc)));
        let arena = Arena {
            pass,
            arsenal: &self.arsenal,
            govts: &self.govts,
            rules,
            field,
        };
        phase(&mut self.combat, &mut fighters, arena, chance);
    }

    /// Settles the hits a phase of the fight on `pass` landed, against
    /// what each NPC `was` and the player's condition `player_was` before
    /// it, by `rules`, drawing on `chance`: the player's "Ship disabled."
    /// and "Ship destroyed." lines, said as the hits land, before the
    /// eject on a breakup's first tick can take the player into a new
    /// ship (see the `player_downed` module); the pushes; each NPC newly
    /// downed raising its fate; the strikes weighed, answered for crimes
    /// and kills ([`Session::settle_strikes`]) and kept for the NPCs'
    /// answers; and a target lost let go.
    ///
    /// The original's `_DamageShip` settles each hit as it lands; here a
    /// phase's hits settle together after it, before the next phase or
    /// the frame.
    fn settle_landed(
        &mut self,
        was: &[NpcBefore],
        player_was: Condition,
        rules: Rules,
        chance: &mut dyn Chance,
    ) {
        let strikes = self.combat.take_strikes();
        self.player_downed(player_was, &strikes, chance);
        self.take_shoves();
        self.raise_disabled(was, chance);
        self.settle_strikes(strikes, was, rules.law, chance);
        self.clear_lost_target();
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
        let was = self.npcs_before();
        let player_was = self.condition;
        self.with_fighters(pass, rules, chance, phase);
        self.settle_landed(&was, player_was, rules, chance);
    }

    /// The fight's collision passes on `pass` by `rules`, drawing on
    /// `chance` ([`Combat::test_hits`]), among the stellars shown that
    /// shots can hit and the asteroids, then their settling: the hits'
    /// ([`Session::settle_landed`]), each stellar shot down running its
    /// `OnDestroy` (see the `stellars` module), and each asteroid broken
    /// (see the `asteroids` module).
    pub(super) fn collision_passes(
        &mut self,
        pass: GamePass,
        rules: Rules,
        chance: &mut dyn Chance,
    ) {
        let mut stellars = self.stellar_targets();
        let mut rocks = self.asteroid_targets();
        self.land_phase(pass, rules, chance, |combat, fighters, arena, chance| {
            let scenery = Scenery {
                stellars: &mut stellars,
                asteroids: &mut rocks,
            };
            combat.test_hits(fighters, arena, scenery, chance);
        });
        self.strike_stellars(&stellars, rules.law, chance);
        self.settle_asteroid_hits(&rocks, chance);
    }

    /// What a pass's fight on `pass` leaves, settled once after its last
    /// phase, by `rules`, drawing on `chance`, against the session as the
    /// pass `start`ed: each NPC's exit cursors kept for its berth; the
    /// eject on a breakup's first tick, which runs the next pass's eject
    /// early, so after the whole fight (see the `eject` module); the
    /// player's explosion, and the klaxon of its breakup
    /// ([`Session::settle_player_explosion`]); the fighters launched from
    /// bays put in the system (see the `fighters` module); each dying
    /// NPC's escape pods (see the `npc_escape` module); the fleet kept in
    /// step; the escorts lost; the NPCs' self-repair; each NPC destroyed
    /// taken out; the fighters orphaned; a target lost let go; the
    /// stellars seen; the ships laid out; and the heading each ship is
    /// drawn at noted, which its next tick's shots leave from.
    pub(super) fn fight_tail(
        &mut self,
        start: &PassStart,
        pass: GamePass,
        rules: Rules,
        chance: &mut dyn Chance,
    ) {
        self.traffic.keep_berth_exits();
        self.eject_on_breakup(pass, chance);
        self.settle_player_explosion(start.was_destroyed, start.was_dying);
        let sorties = self.combat.take_sorties();
        self.launch_fighters(&sorties, chance);
        // The words the pass's stray beams stepped, before the NPCs'
        // escape pods read the crew pods left. The original steps at the
        // beam's spawn; settling here, in the same tick before the frame
        // is drawn, crosses no boundary (see the `stray_words` module).
        self.step_stray_words(chance);
        self.npc_escapes(&start.npcs, chance);
        self.sync_fleet();
        self.lose_escorts();
        self.lose_dying_mission_escorts();
        self.self_repair(rules, &mut *chance);
        self.take_out_the_destroyed(chance);
        self.orphan_fighters();
        self.clear_lost_target();
        self.see_stellars();
        self.lay_out_ships();
        self.note_drawn();
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
        session.target = Some(NpcId(0));
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
        let npc = &mut session.traffic.npcs_mut()[0];
        npc.mode = Mode::Emerging { ticks_left: 60 };
        npc.ai_type = crate::traffic::npc::AiType::Warship;
        session.target = Some(NpcId(0));
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
            session.traffic.npcs_mut()[0].hull.mass = 40.0;
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
        session.target = Some(NpcId(0));
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
        session.player.velocity = Vec2::new(3.0, 0.0);
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
        session.hull.mass = 40.0;
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
            by_parts.tick_assistance(Rules::default().disable);
            by_parts.tick_combat(Rules::default(), &mut NeverFires);
            assert_eq!(session, by_parts);
        }
    }

    #[test]
    fn a_landed_or_jumping_pass_runs_the_parts_as_before() {
        let catalog = armed();
        let mut landed = facing_an_npc(&catalog, 180);
        crate::testkit::land_now(&mut landed).expect("lands");
        runs_the_parts(landed, &catalog);
        let catalog = crate::testkit::catalog();
        let mut jumping = crate::session::tests::bound_for_131(
            &catalog,
            crate::session::tests::OUT,
            Vec2::ZERO,
            90.0,
        );
        jumping.begin_jump().expect("jumps");
        assert!(jumping.in_warp_lead_in());
        runs_the_parts(jumping, &catalog);
    }

    #[test]
    fn a_ship_disabled_by_the_hit_pass_raises_its_fate_once() {
        // Each phase settles its own hits, against the NPCs as the phase
        // found them: the disabling raises its fate after the collision
        // passes, and no later phase raises it again.
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        session.traffic.npcs_mut()[0].reserves.shield.now = 0.0;
        session.traffic.npcs_mut()[0].reserves.armor.now = 16.0;
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
}
