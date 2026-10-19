//! Deadly stellars in flight (see
//! [`deadly_stellar`](crate::deadly_stellar)): `_HandleDeadlyStellars`
//! (@0x39ccd-0x39e47), which `_DoPlayGameWork` calls straight after
//! `_HandleGravity` (@0x4450e), on both passes.
//!
//! **Which stellars.** Each listed stellar, in slot order, that is deadly
//! (`Flags2` 0x0100) and holds a sprite with a frame: a shown one its own,
//! the frame it is drawn with, and a hidden one the sprite of the pool slot
//! it last took, by [`RuleKey::StaleStellarSprite`] (see the
//! `stale_sprites` module). A destroyed stellar shown draws its `DeadType`
//! sheet, or, dead since the last rebuild point with none, graphic type
//! 0's, which kills too.

//! **Which ships.** The player, then each NPC in turn, that is neither
//! destroyed nor breaking up, does not resist ([`resists_deadly`]) and
//! touches the stellar ([`touches`]). A ship killed by one stellar is gone
//! for the later ones.
//!
//! **The kill** (@0x39da4-0x39e1f), made by the fight's tick between its
//! shots and its beams ([`stellar_hazards`](crate::combat::stellar_hazards)):
//! the ship goes inactive with its armour at [`KILLED_ARMOR`] and its
//! death timer at none, and `_CreateExplosion` makes its `Explode2` at
//! size 0 with its sound ([`Combat::explode`]; no explosion when the
//! `Explode2` is out of range, as `_CreateExplosion` refuses it,
//! @0x3f6b1-0x3f6cc). The view plays the bööm's sound, so no `SimSound` is
//! raised here. It never reaches `_HandleShipDisplay`'s death branch, so
//! there is no breakup, shockwave, debris, crime, kill credit or escape
//! pod; and as `_HandleBeams` runs after it (@0x4453d), no beam follows or
//! hits the ship that tick, so a beam on it makes no strike, crime or
//! answer, and one it fired stays where it was. The session settles each
//! kill ([`Session::settle_stellar_kills`]) straight after the tick, before
//! it reads the tick's strikes.
//! - **An NPC** vanishes by [`RuleKey::DeadlyStellarKill`]'s engine
//!   reading: taken out of the system with no fate, no mission count and
//!   its person living on. By the other reading it is destroyed, and
//!   [`Session::take_out_the_destroyed`] raises its fate, counts it lost
//!   when it is a mission ship and loses a person without an escape pod.
//!   The fight's housekeeping that follows lets go of it as the player's
//!   target (the original sets the target to -1 at once, @0x39dd8) and
//!   orphans its fighters.
//! - **An escort of the player** leaves the fleet at once, as one whose
//!   ship is gone ([`Session::lose_escorts`]): by the engine reading it
//!   takes no share of the fleet's cargo, as the original reaches
//!   `_DestroyPartialFleetCargo` only from the death branch (@0x2dc37);
//!   by the other reading, destroyed, it takes its share as any escort
//!   destroyed. How the original's escort roster forgets a slot freed with
//!   no death is not checked: see the phase's follow-up.
//! - **The player** goes destroyed and the hold before the game's end
//!   starts, as `_HandlePlayer`'s dead branch starts it on its next pass
//!   (the "goes inactive" test @0x68700). There is no klaxon and no halted
//!   sound, which belong to the breakup. A player in its escape pod is
//!   killed too, and its pod's countdown ends.
//!
//! - **A mission escort** leaves the formation, and the ships the player
//!   leads re-form without it, by either reading: `_AICalcFormationPositions`
//!   numbers the formation afresh every frame over the active slots whose
//!   parent is the player (the +0xb8 test @0x8515d), so a slot freed with
//!   no death drops out of it on the next frame, though
//!   `_HandleDeadlyStellars` itself calls no re-form. By the engine reading
//!   it takes no share of the fleet's cargo, as `_HandleShip`'s dying
//!   branch (@0x350d9-0x350f4) never runs for it; by the other reading,
//!   destroyed, it takes its share and its AI resets as any mission
//!   escort destroyed ([`Session::lose_dying_mission_escorts`]).
//!
//! **Ordering.** The original's hold starts on the next `_HandlePlayer`,
//! and its killed NPC was laid out by `_HandleShipDisplay` before it went
//! inactive; here the hold starts at the kill and the NPC is taken out
//! before the frame is drawn, by the other reading with its fate raised
//! at once rather than with the fight's other destroyed ships. Neither
//! crosses a drawn frame with different state, beyond the explosion that
//! covers it. A hit on the player earlier in the tick speaks its lines as
//! the hit left the ship, not as the kill did, and the kill sounds no
//! klaxon and halts no sound.

use std::collections::BTreeSet;

use super::Session;
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::combat::hull::Condition;
use crate::deadly_stellar::DeadlyStellarKill;
use crate::player_fate::{Ending, HOLD_TICKS};
use crate::rulebook::RuleKey;
use crate::story::{Fate, StoryEvent};
use crate::traffic::npc::NpcId;
#[cfg(doc)]
use crate::{
    combat::Combat,
    deadly_stellar::{KILLED_ARMOR, resists_deadly, touches},
};

impl Session {
    /// Settles the fight's kills by deadly stellars
    /// ([`Combat::take_stellar_kills`]; see the module docs), drawing on
    /// `chance`: the player's hold starts and its pod
    /// ends; each NPC vanishes or is destroyed and taken out, by
    /// [`RuleKey::DeadlyStellarKill`]; the escorts it was leave the fleet
    /// and the rest re-form. The player's condition just before its kill,
    /// when it was killed.
    pub(super) fn settle_stellar_kills(&mut self, chance: &mut dyn Chance) -> Option<Condition> {
        let kills = self.combat.take_stellar_kills();
        let rule = DeadlyStellarKill::from(self.rules.source_for(RuleKey::DeadlyStellarKill));
        let led: BTreeSet<NpcId> = self.player_led().map(|npc| npc.id).collect();
        let mut player = None;
        let mut killed = BTreeSet::new();
        for kill in &kills {
            match kill.ship {
                ShipRef::Player => {
                    self.kill_player_by_stellar();
                    player = Some(kill.was);
                }
                ShipRef::Npc(id) => {
                    if rule == DeadlyStellarKill::Vanishes {
                        self.traffic.remove(id);
                    }
                    killed.insert(id);
                }
            }
        }
        if killed.is_empty() {
            return player;
        }
        self.lose_escorts(&[], &killed);
        if rule == DeadlyStellarKill::Destroyed {
            self.lose_dying_mission_escorts();
        }
        if !killed.is_disjoint(&led) {
            self.reform();
        }
        if rule == DeadlyStellarKill::Destroyed {
            for &id in &killed {
                self.take_out_stellar_kill(id, chance);
            }
        }
        player
    }

    /// The player's ship, destroyed by a deadly stellar, ends its pod and
    /// starts the hold (see the module docs).
    fn kill_player_by_stellar(&mut self) {
        self.pod = None;
        self.player_exploded_this_tick = true;
        self.player_end = Some(Ending::Holding {
            ticks_left: HOLD_TICKS,
        });
    }

    /// NPC `id`, destroyed by a deadly stellar by the other reading, is
    /// taken out at once as a ship destroyed in the fight: its fate, its
    /// mission's count and its person, drawing on `chance`.
    fn take_out_stellar_kill(&mut self, id: NpcId, chance: &mut dyn Chance) {
        if self.npc(id).is_none() {
            return;
        }
        self.raise(StoryEvent::ShipFate {
            ship: id,
            fate: Fate::Destroyed,
        });
        self.count_lost_mission_ship(id, chance);
        self.lose_destroyed(id);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::{LandingSite, OutfitId, ShipId};
    use crate::chance::NeverFires;
    use crate::combat::{CombatEvent, Rules};
    use crate::deadly_stellar::{DEADLY_RESISTANCE, DEADLY_STELLAR};
    use crate::flight::Controls;
    use crate::geometry::Vec2;
    use crate::rulebook::{RuleSource, Rulebook};
    use crate::session::tests::{armed, facing_an_npc};
    use crate::sound::SimSound;
    use crate::story::{Fate, StoryEvent};
    use crate::testkit::{FakePilotCatalog, outfit, person, planet};
    use crate::traffic::npc::{Npc, NpcPerson};

    /// A deadly stellar `id` at (`x`, `y`), 100 x 100 (radius 50).
    fn deadly_at(id: i16, x: f32, y: f32) -> LandingSite {
        LandingSite {
            flags2: DEADLY_STELLAR,
            ..planet(id, x, y)
        }
    }

    /// A deadly stellar whose disc takes in NPC 0, 100 above the centre,
    /// and leaves the player at the centre clear.
    fn on_the_npc() -> LandingSite {
        deadly_at(140, 0.0, -150.0)
    }

    /// `catalog`'s session with the player at rest at the centre and its
    /// NPC at rest 100 above, the system listing `listed`, of which
    /// `shown` are shown, under `rules`.
    fn among(
        catalog: &FakePilotCatalog,
        listed: &[LandingSite],
        shown: usize,
        rules: Rulebook,
    ) -> Session {
        let mut session = facing_an_npc(catalog, 180).with_rules(rules);
        session.player.position = Vec2::ZERO;
        session.player.velocity = Vec2::ZERO;
        session.listed = listed.to_vec();
        session.sites = listed[..shown].to_vec();
        session
    }

    fn fight(session: &mut Session) {
        session.tick_combat(Rules::default(), &mut NeverFires);
    }

    fn npc_mut(session: &mut Session) -> &mut Npc {
        &mut session.traffic.npcs_mut()[0]
    }

    fn exploded(events: &[CombatEvent]) -> Vec<CombatEvent> {
        events
            .iter()
            .filter(|event| matches!(event, CombatEvent::Exploded { .. }))
            .copied()
            .collect()
    }

    fn fates(session: &Session) -> Vec<StoryEvent> {
        session
            .story_raised()
            .into_iter()
            .filter(|event| matches!(event, StoryEvent::ShipFate { .. }))
            .collect()
    }

    /// Each reading of [`RuleKey::DeadlyStellarKill`] and
    /// [`RuleKey::GravityAfterburner`], the engine's first.
    fn readings() -> Vec<Rulebook> {
        let mut readings = Vec::new();
        for kill in [RuleSource::Engine, RuleSource::Bible] {
            for afterburner in [RuleSource::Engine, RuleSource::Bible] {
                readings.push(
                    Rulebook::default()
                        .with_override(RuleKey::DeadlyStellarKill, kill)
                        .with_override(RuleKey::GravityAfterburner, afterburner),
                );
            }
        }
        readings
    }

    /// The other reading of [`RuleKey::DeadlyStellarKill`].
    fn destroyed_reading() -> Rulebook {
        Rulebook::default().with_override(RuleKey::DeadlyStellarKill, RuleSource::Bible)
    }

    /// [`among`] with NPC 0 flown by person 600, of `Flags` `flags`.
    fn with_a_person(flags: u16, rules: Rulebook) -> Session {
        let catalog = armed();
        let mut session = among(&catalog, &[on_the_npc()], 1, rules);
        let record = crate::catalog::PersonRecord {
            flags,
            ..person(600, 129)
        };
        npc_mut(&mut session).person = Some(NpcPerson::of(&record, false));
        session
    }

    #[test]
    fn an_npc_touching_a_deadly_stellar_vanishes_in_its_explode2_with_nothing_else() {
        let catalog = armed();
        let mut session = among(&catalog, &[on_the_npc()], 1, Rulebook::default());
        let npc = session.npcs()[0].clone();
        let rating = session.pilot.combat_rating;
        let record = session.pilot.clone();
        fight(&mut session);
        assert_eq!(session.npcs(), [], "gone");
        let events = session.take_combat_events();
        assert_eq!(
            exploded(&events),
            [CombatEvent::Exploded {
                at: npc.state.position,
                explosion: npc.hull.explosion.expect("an Explode2"),
                size: 0.0,
            }]
        );
        assert!(
            !events.iter().any(|event| matches!(
                event,
                CombatEvent::BreakingUp { .. } | CombatEvent::Destroyed { .. }
            )),
            "{events:?}"
        );
        assert_eq!(fates(&session), [], "no fate");
        assert_eq!(session.pilot.combat_rating, rating, "no kill credit");
        assert_eq!(session.pilot.legal, record.legal, "no crime");
        assert_eq!(session.condition, Condition::Intact, "the player is spared");
    }

    #[test]
    fn by_the_engine_the_vanished_npcs_person_lives_on_and_by_the_other_reading_is_gone() {
        let mut vanished = with_a_person(0, Rulebook::default());
        fight(&mut vanished);
        assert_eq!(vanished.npcs(), []);
        assert!(!vanished.pilot().gone(crate::catalog::PersonId(600)));
        let mut destroyed = with_a_person(0, destroyed_reading());
        fight(&mut destroyed);
        assert_eq!(destroyed.npcs(), []);
        assert!(destroyed.pilot().gone(crate::catalog::PersonId(600)));
        let mut podded = with_a_person(crate::person::ESCAPE_POD, destroyed_reading());
        fight(&mut podded);
        assert!(
            !podded.pilot().gone(crate::catalog::PersonId(600)),
            "its pod"
        );
    }

    #[test]
    fn by_the_other_reading_the_npc_is_destroyed_with_its_fate_and_the_same_explosion() {
        let catalog = armed();
        let mut session = among(&catalog, &[on_the_npc()], 1, destroyed_reading());
        fight(&mut session);
        assert_eq!(session.npcs(), [], "taken out");
        assert_eq!(
            fates(&session),
            [StoryEvent::ShipFate {
                ship: NpcId(0),
                fate: Fate::Destroyed,
            }]
        );
        let events = session.take_combat_events();
        assert_eq!(exploded(&events).len(), 1, "{events:?}");
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, CombatEvent::BreakingUp { .. })),
            "{events:?}"
        );
    }

    #[test]
    fn a_dying_npc_or_a_stellar_immune_one_is_spared() {
        for rules in readings() {
            let catalog = armed();
            let dying = |session: &mut Session| {
                npc_mut(session).condition = Condition::Dying {
                    ticks_left: 100,
                    starting: false,
                };
            };
            let immune = |session: &mut Session| npc_mut(session).hull.stellar_immune = true;
            for spare in [dying, immune] {
                let mut session = among(&catalog, &[on_the_npc()], 1, rules);
                spare(&mut session);
                fight(&mut session);
                assert_eq!(session.npcs().len(), 1);
            }
            let mut disabled = among(&catalog, &[on_the_npc()], 1, rules);
            npc_mut(&mut disabled).condition = Condition::Disabled;
            fight(&mut disabled);
            assert_eq!(disabled.npcs(), [], "a disabled NPC is killed");
        }
    }

    #[test]
    fn an_npcs_resistance_outfit_does_not_count_nor_does_inertialess_flight() {
        for rules in readings() {
            let catalog = armed();
            let mut session = among(&catalog, &[on_the_npc()], 1, rules);
            npc_mut(&mut session).stats.deadly_resistance = true;
            npc_mut(&mut session).stats.inertialess_hull = true;
            fight(&mut session);
            assert_eq!(session.npcs(), []);
        }
    }

    #[test]
    fn a_ship_clear_of_the_stellar_is_spared() {
        for rules in readings() {
            let catalog = armed();
            let clear = deadly_at(140, 0.0, -161.0);
            let mut session = among(&catalog, &[clear], 1, rules);
            fight(&mut session);
            assert_eq!(session.npcs().len(), 1);
            assert_eq!(exploded(&session.take_combat_events()), []);
        }
    }

    #[test]
    fn a_hidden_deadly_stellar_or_one_with_no_frame_kills_nothing() {
        for rules in readings() {
            let catalog = armed();
            let shown = planet(139, 3000.0, 0.0);
            let mut hidden = among(&catalog, &[shown, on_the_npc()], 1, rules);
            fight(&mut hidden);
            assert_eq!(hidden.npcs().len(), 1, "hidden, never shown");
            let frameless = LandingSite {
                frame_size: None,
                ..on_the_npc()
            };
            let mut session = among(&catalog, &[frameless], 1, rules);
            fight(&mut session);
            assert_eq!(session.npcs().len(), 1, "no frame");
        }
    }

    #[test]
    fn a_destroyed_deadly_stellar_kills_by_its_dead_frame() {
        for rules in readings() {
            let catalog = armed();
            let dead = |dead_frame_size| LandingSite {
                destroyed: true,
                frame_size: Some((10, 10)),
                dead_frame_size,
                ..on_the_npc()
            };
            let mut session = among(&catalog, &[dead(Some((100, 100)))], 1, rules);
            fight(&mut session);
            assert_eq!(session.npcs(), [], "its dead frame reaches");
            let mut none = among(&catalog, &[dead(None)], 1, rules);
            fight(&mut none);
            assert_eq!(
                none.npcs().len(),
                1,
                "no dead frame: its Type frame, 10 across"
            );
        }
    }

    #[test]
    fn the_players_target_killed_is_let_go() {
        for rules in readings() {
            let catalog = armed();
            let mut session = among(&catalog, &[on_the_npc()], 1, rules);
            session.target = Some(NpcId(0));
            fight(&mut session);
            assert_eq!(session.target, None);
        }
    }

    #[test]
    fn an_explode2_out_of_range_shows_nothing_but_still_kills() {
        for rules in readings() {
            let catalog = armed();
            let mut session = among(&catalog, &[on_the_npc()], 1, rules);
            npc_mut(&mut session).hull.explosion = None;
            fight(&mut session);
            assert_eq!(session.npcs(), []);
            assert_eq!(exploded(&session.take_combat_events()), []);
        }
    }

    #[test]
    fn the_player_touching_is_destroyed_with_the_hold_started_and_no_klaxon() {
        for rules in readings() {
            let catalog = armed();
            let on_the_player = deadly_at(140, 0.0, 40.0);
            let mut session = among(&catalog, &[on_the_player], 1, rules);
            session.take_sounds();
            fight(&mut session);
            assert_eq!(session.condition, Condition::Destroyed);
            assert_eq!(session.pilot.reserves.armor.now, -1000.0);
            assert_eq!(
                session.player_end,
                Some(Ending::Holding {
                    ticks_left: HOLD_TICKS
                })
            );
            assert_eq!(session.player_fate(), crate::PlayerFate::Exploded);
            let sounds = session.take_sounds();
            assert!(
                !sounds
                    .iter()
                    .any(|sound| matches!(sound, SimSound::Klaxon | SimSound::Halt(_))),
                "{sounds:?}"
            );
            let events = session.take_combat_events();
            assert_eq!(
                exploded(&events),
                [CombatEvent::Exploded {
                    at: Vec2::ZERO,
                    explosion: session.hull.explosion.expect("an Explode2"),
                    size: 0.0,
                }]
            );
            assert_eq!(session.npcs().len(), 1, "the NPC is clear");
        }
    }

    #[test]
    fn the_player_carrying_a_modtype_42_outfit_or_stellar_immune_is_spared() {
        for rules in readings() {
            let mut catalog = armed();
            catalog.outfits.push(outfit(197, &[(DEADLY_RESISTANCE, 0)]));
            catalog.defaults = vec![(ShipId(128), vec![(OutfitId(197), 1)])];
            let on_the_player = deadly_at(140, 0.0, 40.0);
            let mut outfitted = among(&catalog, &[on_the_player], 1, rules);
            assert!(outfitted.stats.deadly_resistance);
            fight(&mut outfitted);
            assert_eq!(outfitted.condition, Condition::Intact);
            let plain = armed();
            let mut immune = among(&plain, &[on_the_player], 1, rules);
            immune.hull.stellar_immune = true;
            fight(&mut immune);
            assert_eq!(immune.condition, Condition::Intact);
            let mut dying = among(&plain, &[on_the_player], 1, rules);
            dying.condition = Condition::Dying {
                ticks_left: 100,
                starting: false,
            };
            fight(&mut dying);
            assert!(matches!(dying.condition, Condition::Dying { .. }), "spared");
        }
    }

    #[test]
    fn a_destroyed_player_is_not_killed_again_on_later_passes() {
        for rules in readings() {
            let catalog = armed();
            let on_the_player = deadly_at(140, 0.0, 40.0);
            let mut session = among(&catalog, &[on_the_player], 1, rules);
            fight(&mut session);
            assert_eq!(session.condition, Condition::Destroyed);
            let mut events = session.take_combat_events();
            session.tick(Controls::default());
            fight(&mut session);
            session.tick_again(Controls::default(), Rules::default(), &mut NeverFires);
            events.extend(session.take_combat_events());
            assert_eq!(exploded(&events).len(), 1, "{events:?}");
            let Some(Ending::Holding { ticks_left }) = session.player_end else {
                panic!("holding: {:?}", session.player_end);
            };
            assert!(ticks_left < HOLD_TICKS, "counting down: {ticks_left}");
        }
    }

    #[test]
    fn by_the_other_reading_an_npc_one_stellar_destroys_is_not_exploded_by_another() {
        let catalog = armed();
        let overlapping = [on_the_npc(), deadly_at(141, 0.0, -140.0)];
        let mut session = among(&catalog, &overlapping, 2, destroyed_reading());
        fight(&mut session);
        assert_eq!(session.npcs(), []);
        let events = session.take_combat_events();
        assert_eq!(exploded(&events).len(), 1, "{events:?}");
        assert_eq!(fates(&session).len(), 1, "one fate");
    }

    #[test]
    fn the_player_in_its_escape_pod_is_killed_and_never_rescued() {
        for rules in readings() {
            let catalog = armed();
            let on_the_player = deadly_at(140, 0.0, 40.0);
            let mut session = among(&catalog, &[on_the_player], 1, rules);
            session.pod = Some(crate::player_fate::POD_TICKS);
            fight(&mut session);
            assert_eq!(session.pod, None);
            assert_eq!(session.player_fate(), crate::PlayerFate::Exploded);
            assert!(matches!(session.player_end, Some(Ending::Holding { .. })));
            for _ in 0..400 {
                session.tick(Controls::default());
                fight(&mut session);
                assert!(!session.rescue_due, "never rescued");
            }
        }
    }

    #[test]
    fn the_second_pass_kills_too() {
        for rules in readings() {
            let catalog = armed();
            let mut session = among(&catalog, &[planet(140, 0.0, -150.0)], 1, rules);
            fight(&mut session);
            assert_eq!(session.npcs().len(), 1);
            let at = session.npcs()[0].state.position;
            session.sites = vec![deadly_at(140, at.x, at.y)];
            session.listed = session.sites.clone();
            session.tick_again(Controls::default(), Rules::default(), &mut NeverFires);
            assert_eq!(session.npcs(), []);
        }
    }

    /// `armed`'s catalog with the player's gun a 300-pixel beam lasting 3
    /// ticks, doing 5 energy damage.
    fn beaming() -> FakePilotCatalog {
        let mut catalog = armed();
        catalog.weapons = vec![crate::catalog::WeaponRecord {
            guidance: 0,
            count: 3,
            reload: 10,
            beam_length: 300,
            energy_dmg: 5,
            ..crate::session::tests::blaster()
        }];
        catalog
    }

    #[test]
    fn the_players_beam_on_an_npc_a_deadly_stellar_kills_that_tick_strikes_nothing() {
        for rules in readings() {
            let catalog = beaming();
            let struck = |listed: &[LandingSite]| {
                let mut session = among(&catalog, listed, listed.len(), rules);
                session.hold_trigger(crate::session::tests::FIRE);
                let legal = session.pilot.legal.clone();
                fight(&mut session);
                let strikes: Vec<_> = session
                    .strikes
                    .iter()
                    .filter(|strike| strike.ship == ShipRef::Npc(NpcId(0)))
                    .copied()
                    .collect();
                (strikes, session.pilot.legal == legal, session.npcs().len())
            };
            let (strikes, _, npcs) = struck(&[planet(140, 0.0, -150.0)]);
            assert_eq!(strikes.len(), 1, "the beam hits it with no deadly stellar");
            assert_eq!(npcs, 1);
            let (strikes, lawful, npcs) = struck(&[on_the_npc()]);
            assert_eq!(strikes, [], "killed before the beams");
            assert!(lawful, "no crime");
            assert_eq!(npcs, 0);
        }
    }

    /// The tick on which the NPC's first shot lands on the player, firing
    /// at it from 100 above, from the start: with the player at the
    /// centre, its armour held at `armor`.
    fn first_hit_on_the_player(catalog: &FakePilotCatalog, armor: f32) -> usize {
        let mut session = among(catalog, &[planet(140, 3000.0, 0.0)], 1, Rulebook::default());
        session.pilot.reserves.armor.now = armor;
        for tick in 0..60 {
            fire_at_the_player(&mut session, catalog);
            if session
                .strikes
                .iter()
                .any(|strike| strike.ship == ShipRef::Player)
            {
                return tick;
            }
        }
        panic!("never hit");
    }

    fn fire_at_the_player(session: &mut Session, catalog: &FakePilotCatalog) {
        session.tick_traffic(catalog, &crate::session::tests::Firing, &mut NeverFires);
        fight(session);
    }

    #[test]
    fn a_hit_on_the_player_the_tick_a_deadly_stellar_kills_it_says_no_line_of_its_own() {
        let catalog = armed();
        // A hit that downs nothing: the player full.
        let on = first_hit_on_the_player(&catalog, 100_000.0);
        let mut session = among(
            &catalog,
            &[planet(140, 3000.0, 0.0)],
            1,
            Rulebook::default(),
        );
        session.pilot.reserves.armor.now = 100_000.0;
        for _ in 0..on {
            fire_at_the_player(&mut session, &catalog);
        }
        session.take_messages();
        session.take_sounds();
        session.sites = vec![deadly_at(140, 0.0, 40.0)];
        session.listed = session.sites.clone();
        fire_at_the_player(&mut session, &catalog);
        assert!(
            session
                .strikes
                .iter()
                .any(|strike| strike.ship == ShipRef::Player),
            "hit that tick"
        );
        assert_eq!(session.condition, Condition::Destroyed, "and killed");
        let messages = session.take_messages();
        assert!(
            !messages.iter().any(|message| matches!(
                message,
                crate::message::SimMessage::ShipDisabled
                    | crate::message::SimMessage::ShipDestroyed
            )),
            "{messages:?}"
        );
        let sounds = session.take_sounds();
        assert!(
            !sounds.iter().any(|sound| matches!(
                sound,
                SimSound::ShipDowned | SimSound::Klaxon | SimSound::Halt(_)
            )),
            "{sounds:?}"
        );
    }
}
