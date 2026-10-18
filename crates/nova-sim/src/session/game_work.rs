//! The fight's phases as the session runs them, and what each settles
//! (see [`Combat`]'s phases).
//!
//! Each phase builds the fight's [`Fighter`]s afresh from the session
//! ([`Session::with_fighters`]): the player, then every NPC, each on the
//! terms the session works out for it. A phase that lands hits is settled
//! at once ([`Session::settle_landed`]): the player's downing lines, the
//! pushes, the eject on a breakup's first tick, the NPCs' fates, the
//! strikes' answers, crimes and kills, and a target lost. What the whole
//! pass leaves is settled once, after its last phase
//! ([`Session::fight_tail`]).

use super::{FightTerms, NpcBefore, Session};
use crate::chance::Chance;
use crate::combat::armament::{OutfitRounds, Trigger};
use crate::combat::hull::Condition;
use crate::combat::{Arena, Combat, Fighter, GamePass, Rules, Scenery, ShipRef};

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
    use crate::chance::NeverFires;
    use crate::combat::Rules;
    use crate::combat::armament::Trigger;
    use crate::combat::hull::Condition;
    use crate::session::tests::{FIRE, armed, facing_an_npc};
    use crate::story::{Fate, StoryEvent};
    use crate::traffic::npc::NpcId;

    /// The ship fates raised and not yet settled.
    fn fates(session: &crate::session::Session) -> Vec<StoryEvent> {
        session
            .story_raised()
            .into_iter()
            .filter(|event| matches!(event, StoryEvent::ShipFate { .. }))
            .collect()
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
