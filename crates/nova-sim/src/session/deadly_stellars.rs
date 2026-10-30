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
//! answer, and one it fired stays where it was. Straight after the tick,
//! before it reads the tick's strikes, the session starts the player's
//! hold ([`Session::hold_stellar_kills`]). An NPC killed reads as the
//! tick's hits left it through the fight's housekeeping, so a hit that
//! disabled it before the kill raises its disabled fate, counts a mission
//! ship disabled and loses an escort with its share of the cargo, as the
//! hit's `_DamageShip` did before `_HandleDeadlyStellars` ran. It then
//! vanishes or is destroyed ([`Session::take_out_the_killed`]) before the
//! destroyed ships are taken out.
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
//!   destroyed. It is forgotten for good: the original has no roster
//!   apart from the ship table, an escort being an active slot (+0xb8)
//!   the player leads, and every reader skips the slot it frees: the
//!   save's escort loops (`_WritePilotData` @0x72604, @0x7265e,
//!   @0x72728), the next system's setup (`_SetupShipsInSystem`
//!   @0x42baf), the daily pay (`_DoEscortPayment` @0x40148), the hire
//!   limit (`_CanHireEscorts` @0x57ad) and the formation
//!   (`_AICalcFormationPositions` @0x8515d). `_DeleteAllComputerShips(0)`
//!   keeps the slot's stale fields (@0x3bf7e-0x3bfa9), which the next
//!   spawn into it overwrites. So it is not saved, placed again, paid or
//!   counted, as the engine's dropped record is not.
//! - **The player** goes destroyed and the hold before the game's end
//!   starts, as `_HandlePlayer`'s dead branch starts it on its next pass
//!   (the "goes inactive" test @0x68700). There is no klaxon and no halted
//!   sound, which belong to the breakup. A player in its escape pod is
//!   killed too, and its pod's countdown ends: the dead branch is taken
//!   before anything else on the ship (@0x68700, jumping to the active
//!   path @0x68891 only for an active ship), and the pod's countdown
//!   (+0xc908, @0x68d95-0x68ddd) and rescue (@0x68de3) are on the active
//!   path, so a killed pod is never rescued and the hold runs from 0. A
//!   pod whose countdown runs out on a pass is rescued inside
//!   `_HandlePlayer`, before `_HandleDeadlyStellars` that pass, so it is
//!   never killed: the view rescues ([`Session::rescue_if_due`]) before the
//!   fight's tick, and double time's second pass
//!   ([`Session::tick_again`]) stops before its fight once a rescue is
//!   due.
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
//! at the end of the fight's housekeeping, before the frame is drawn.
//! Neither
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
use crate::traffic::npc::NpcId;
#[cfg(doc)]
use crate::{
    combat::Combat,
    deadly_stellar::{KILLED_ARMOR, resists_deadly, touches},
};

impl Session {
    /// The part of the fight's kills by deadly stellars
    /// ([`Combat::stellar_kills`]) that settles straight after the tick
    /// (see the module docs): the player's hold starts and its pod ends,
    /// and each NPC killed reads, until [`Session::take_out_the_killed`],
    /// as the tick's hits left it. The player's condition just before its
    /// kill, when it was killed.
    pub(super) fn hold_stellar_kills(&mut self) -> Option<Condition> {
        let kills = self.space.combat.stellar_kills().to_vec();
        let mut player = None;
        for kill in kills {
            match kill.ship {
                ShipRef::Player => {
                    self.kill_player_by_stellar();
                    player = Some(kill.was);
                }
                ShipRef::Npc(id) => {
                    if let Some(npc) = self.space.npc_mut(id) {
                        npc.condition = kill.was;
                    }
                }
            }
        }
        player
    }

    /// The fight's housekeeping's end (see the module docs), drawing on
    /// `chance`: each NPC a deadly stellar killed this tick vanishes or is
    /// destroyed, by [`RuleKey::DeadlyStellarKill`], the escorts it was
    /// leave the fleet and the rest re-form; then every NPC destroyed is
    /// taken out ([`Session::take_out_the_destroyed`]).
    pub(super) fn take_out_the_killed(&mut self, chance: &mut dyn Chance) {
        self.settle_stellar_kills();
        self.take_out_the_destroyed(chance);
    }

    /// Each NPC among the fight's kills by deadly stellars
    /// ([`Combat::take_stellar_kills`]) still here vanishes or is
    /// destroyed, as [`Session::take_out_the_killed`] says.
    fn settle_stellar_kills(&mut self) {
        let rule = DeadlyStellarKill::from(self.setup.rules.source_for(RuleKey::DeadlyStellarKill));
        let led: BTreeSet<NpcId> = self.player_led().map(|npc| npc.id).collect();
        let killed: BTreeSet<NpcId> = self
            .space
            .combat
            .take_stellar_kills()
            .into_iter()
            .filter_map(|kill| match kill.ship {
                ShipRef::Npc(id) => Some(id),
                ShipRef::Player => None,
            })
            .filter(|&id| self.space.npc(id).is_some())
            .collect();
        if killed.is_empty() {
            return;
        }
        for &id in &killed {
            match rule {
                DeadlyStellarKill::Vanishes => self.space.traffic.remove(id),
                DeadlyStellarKill::Destroyed => {
                    if let Some(npc) = self.space.npc_mut(id) {
                        npc.condition = Condition::Destroyed;
                    }
                }
            }
        }
        self.lose_escorts(&[], &killed);
        if rule == DeadlyStellarKill::Destroyed {
            self.lose_dying_mission_escorts();
        }
        if !killed.is_disjoint(&led) {
            self.reform();
        }
    }

    /// The player's ship, destroyed by a deadly stellar, ends its pod and
    /// starts the hold (see the module docs).
    fn kill_player_by_stellar(&mut self) {
        self.pod = None;
        self.ship.fate.player_exploded_this_tick = true;
        self.player_end = Some(Ending::Holding {
            ticks_left: HOLD_TICKS,
        });
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
pub(super) mod tests {
    use super::*;
    use crate::catalog::{LandingSite, OutfitId, ShipId};
    use crate::chance::NeverFires;
    use crate::combat::{CombatEvent, Rules};
    use crate::deadly_stellar::{DEADLY_RESISTANCE, DEADLY_STELLAR};
    use crate::flight::Controls;
    use crate::geometry::Vec2;
    use crate::message::SimMessage;
    use crate::rulebook::{RuleSource, Rulebook};
    use crate::session::tests::{armed, facing_an_npc};
    use crate::sound::SimSound;
    use crate::story::{Fate, StoryEvent};
    use crate::testkit::{FakePilotCatalog, outfit, person, planet};
    use crate::testkit::{work, work_with};
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
        session.ship.player.position = Vec2::ZERO;
        session.ship.player.velocity = Vec2::ZERO;
        session.chart.listed = listed.to_vec();
        session.chart.sites = listed[..shown].to_vec();
        session
    }

    fn fight(session: &mut Session) {
        work(session, &crate::testkit::catalog(), Controls::default());
    }

    fn npc_mut(session: &mut Session) -> &mut Npc {
        &mut session.space.traffic.npcs_mut()[0]
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
        assert_eq!(
            session.ship.condition,
            Condition::Intact,
            "the player is spared"
        );
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
            session.chart.target = Some(NpcId(0));
            fight(&mut session);
            assert_eq!(session.chart.target, None);
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
            assert_eq!(session.ship.condition, Condition::Destroyed);
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
                    explosion: session.ship.fit.hull.explosion.expect("an Explode2"),
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
            assert!(outfitted.ship.fit.stats.deadly_resistance);
            fight(&mut outfitted);
            assert_eq!(outfitted.ship.condition, Condition::Intact);
            let plain = armed();
            let mut immune = among(&plain, &[on_the_player], 1, rules);
            immune.ship.fit.hull.stellar_immune = true;
            fight(&mut immune);
            assert_eq!(immune.ship.condition, Condition::Intact);
            let mut dying = among(&plain, &[on_the_player], 1, rules);
            dying.ship.condition = Condition::Dying {
                ticks_left: 100,
                starting: false,
            };
            fight(&mut dying);
            assert!(
                matches!(dying.ship.condition, Condition::Dying { .. }),
                "spared"
            );
        }
    }

    #[test]
    fn a_destroyed_player_is_not_killed_again_on_later_passes() {
        for rules in readings() {
            let catalog = armed();
            let on_the_player = deadly_at(140, 0.0, 40.0);
            let mut session = among(&catalog, &[on_the_player], 1, rules);
            fight(&mut session);
            assert_eq!(session.ship.condition, Condition::Destroyed);
            let mut events = session.take_combat_events();
            work(&mut session, &catalog, Controls::default());
            fight(&mut session);
            session.tick_again(
                Controls::default(),
                &crate::ai::NovaAi::default(),
                Rules::default(),
                &mut NeverFires,
            );
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
                work(&mut session, &catalog, Controls::default());
                fight(&mut session);
                assert!(!session.ship.fate.rescue_due, "never rescued");
            }
        }
    }

    /// A jump outlives the kill: the deadly stellar takes no account of
    /// one (`_HandleDeadlyStellars` tests only +0xb8, @0x39d3f), its kill
    /// writes no +0x50 (@0x39da4-0x39e1f), and the dead branch returns
    /// before the jump's match. In play, the warp-out run or the
    /// pre-jump's drift carries the ship into the stellar; here the jump is
    /// set directly. So the game's end sits beside the mode, not in it.
    #[test]
    fn a_deadly_stellar_kills_a_jumping_player_and_the_jump_stays() {
        use crate::catalog::SystemId;
        use crate::session::Jump;
        let to = SystemId(131);
        for jump in [
            Jump::PreJump {
                to,
                bearing: Some(90),
                warp: Some(3),
            },
            Jump::Hyperspace { to, warp: 3 },
        ] {
            let catalog = armed();
            let on_the_player = deadly_at(140, 0.0, 40.0);
            let mut session = among(&catalog, &[on_the_player], 1, Rulebook::default());
            session.mode = crate::session::Mode::Jumping(jump);
            fight(&mut session);
            assert_eq!(session.ship.condition, Condition::Destroyed, "{jump:?}");
            assert!(
                matches!(session.player_end, Some(Ending::Holding { .. })),
                "{jump:?}"
            );
            // The pass's move ran the jump's frame, its warp clock on a
            // tick, before `_HandleDeadlyStellars` (@0x4450e) killed it.
            let jump = match jump {
                Jump::PreJump { to, bearing, .. } => Jump::PreJump {
                    to,
                    bearing,
                    warp: Some(4),
                },
                Jump::Hyperspace { to, .. } => Jump::Hyperspace { to, warp: 4 },
            };
            assert_eq!(session.mode.jump(), Some(jump), "the jump stays");
            for tick in 0..5 {
                fight(&mut session);
                assert!(
                    matches!(session.player_end, Some(Ending::Holding { .. })),
                    "{jump:?} tick {tick}"
                );
                assert_eq!(session.mode.jump(), Some(jump), "{jump:?} tick {tick}");
            }
        }
    }

    /// The pod's countdown runs out on the tick a deadly stellar covers
    /// it: `_HandlePlayer` rescues it (@0x68de3) before
    /// `_HandleDeadlyStellars` runs that pass, so it is never killed: the
    /// rescue ends the pass ([`Session::game_work`]).
    #[test]
    fn a_pod_rescued_on_the_tick_a_deadly_stellar_covers_it_is_not_killed() {
        for rules in readings() {
            let catalog = armed();
            let on_the_player = deadly_at(140, 0.0, 40.0);
            let mut session = among(&catalog, &[on_the_player], 1, rules);
            session.pilot.ship = crate::combat::hull::ESCAPE_POD_SHIP;
            session.pod = Some(1);
            // Its countdown runs out, and the rescue ends the pass
            // (`_HandlePlayer` @0x68ddd) before `_HandleDeadlyStellars`.
            assert!(session.game_work(
                Controls::default(),
                &catalog,
                &crate::testkit::Passive,
                Rules::default(),
                &mut NeverFires
            ));
            assert_ne!(session.ship.condition, Condition::Destroyed);
            assert_eq!(session.player_end, None);
            assert_eq!(exploded(&session.take_combat_events()), []);
        }
    }

    #[test]
    fn a_pod_whose_countdown_runs_out_on_the_second_pass_is_not_killed() {
        for rules in readings() {
            let catalog = armed();
            let on_the_player = deadly_at(140, 0.0, 40.0);
            let mut session = among(&catalog, &[on_the_player], 1, rules);
            session.pilot.ship = crate::combat::hull::ESCAPE_POD_SHIP;
            session.pod = Some(1);
            session.tick_again(
                Controls::default(),
                &crate::ai::NovaAi::default(),
                Rules::default(),
                &mut NeverFires,
            );
            assert!(session.ship.fate.rescue_due, "its countdown ran out");
            assert_eq!(session.pod, Some(0), "not killed");
            assert_ne!(session.ship.condition, Condition::Destroyed);
            assert_eq!(exploded(&session.take_combat_events()), []);
            assert!(session.rescue_if_due(&catalog, &mut NeverFires));
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
            session.chart.sites = vec![deadly_at(140, at.x, at.y)];
            session.chart.listed = session.chart.sites.clone();
            session.tick_again(
                Controls::default(),
                &crate::ai::NovaAi::default(),
                Rules::default(),
                &mut NeverFires,
            );
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
                let shield = session.npcs()[0].reserves.shield.now;
                fight(&mut session);
                // The beams' strikes are answered as they settle, so none
                // is kept: what the beam struck shows in the NPC's shield.
                let hit = session
                    .npcs()
                    .iter()
                    .find(|npc| npc.id == NpcId(0))
                    .is_some_and(|npc| npc.reserves.shield.now < shield);
                (hit, session.pilot.legal == legal, session.npcs().len())
            };
            let (hit, _, npcs) = struck(&[planet(140, 0.0, -150.0)]);
            assert!(hit, "the beam hits it with no deadly stellar");
            assert_eq!(npcs, 1);
            let (hit, lawful, npcs) = struck(&[on_the_npc()]);
            assert!(!hit, "killed before the beams");
            assert!(lawful, "no crime");
            assert_eq!(npcs, 0);
        }
    }

    /// `catalog`'s session, readied by `ready` before each tick, its NPC
    /// firing at the player from 100 above, ticked until the tick its
    /// first shot lands on the player, on which a deadly stellar covers
    /// the player: the session after that tick, the lines and sounds of
    /// the ticks before it taken.
    fn hit_as_a_deadly_stellar_covers_the_player(
        catalog: &FakePilotCatalog,
        ready: impl Fn(&mut Session),
    ) -> Session {
        let mut session = among(catalog, &[planet(140, 3000.0, 0.0)], 1, Rulebook::default());
        // The pass answers its strikes: a hit shows in the player's
        // reserves.
        let struck = |before: &Session, after: &Session| {
            let (was, now) = (before.reserves(), after.reserves());
            now.shield.now < was.shield.now || now.armor.now < was.armor.now
        };
        for _ in 0..60 {
            ready(&mut session);
            let mut trial = session.clone();
            fire_at_the_player(&mut trial, catalog);
            if struck(&session, &trial) {
                session.take_messages();
                session.take_sounds();
                session.chart.sites = vec![deadly_at(140, 0.0, 40.0)];
                session.chart.listed = session.chart.sites.clone();
                let before = session.clone();
                fire_at_the_player(&mut session, catalog);
                assert!(struck(&before, &session), "hit that tick");
                return session;
            }
            session = trial;
        }
        panic!("never hit");
    }

    fn fire_at_the_player(session: &mut Session, catalog: &FakePilotCatalog) {
        work_with(
            session,
            catalog,
            Controls::default(),
            &crate::session::tests::Firing,
            Rules::default(),
            &mut NeverFires,
        );
    }

    /// Which of the lines and sounds of a downed player `session` raised.
    fn downed_lines(session: &mut Session) -> (Vec<SimMessage>, Vec<SimSound>) {
        let messages = session
            .take_messages()
            .into_iter()
            .filter(|message| {
                matches!(
                    message,
                    SimMessage::ShipDisabled | SimMessage::ShipDestroyed
                )
            })
            .collect();
        let sounds = session
            .take_sounds()
            .into_iter()
            .filter(|sound| {
                matches!(
                    sound,
                    SimSound::ShipDowned | SimSound::Klaxon | SimSound::Halt(_)
                )
            })
            .collect();
        (messages, sounds)
    }

    #[test]
    fn a_hit_on_the_player_the_tick_a_deadly_stellar_kills_it_says_no_line_of_its_own() {
        let catalog = armed();
        // A hit that downs nothing: the player full.
        let mut session = hit_as_a_deadly_stellar_covers_the_player(&catalog, |session| {
            session.pilot.reserves.armor.now = 100_000.0;
        });
        assert_eq!(session.ship.condition, Condition::Destroyed, "and killed");
        assert_eq!(downed_lines(&mut session), (vec![], vec![]));
    }

    #[test]
    fn a_hit_that_disables_the_player_the_tick_a_deadly_stellar_kills_it_says_ship_disabled() {
        let catalog = armed();
        // Its shields gone and its armour just above the third that
        // disables it.
        let mut session = hit_as_a_deadly_stellar_covers_the_player(&catalog, |session| {
            let reserves = &mut session.pilot.reserves;
            reserves.shield.now = 0.0;
            reserves.armor.now = reserves.armor.max / 3.0 + 3.0;
        });
        assert_eq!(session.ship.condition, Condition::Destroyed, "and killed");
        assert_eq!(
            downed_lines(&mut session),
            (vec![SimMessage::ShipDisabled], vec![SimSound::ShipDowned]),
            "no klaxon and no halt"
        );
    }

    #[test]
    fn a_hit_that_breaks_the_player_up_the_tick_a_deadly_stellar_covers_it_says_ship_destroyed() {
        let catalog = armed();
        let mut session = hit_as_a_deadly_stellar_covers_the_player(&catalog, |session| {
            session.pilot.reserves.shield.now = 0.0;
            session.pilot.reserves.armor.now = 1.0;
        });
        assert!(
            matches!(session.ship.condition, Condition::Dying { .. }),
            "breaking up, so spared: {:?}",
            session.ship.condition
        );
        assert_eq!(session.player_end, None, "no hold");
        let (messages, sounds) = downed_lines(&mut session);
        // Disabled already by its armour, so only the breakup speaks.
        assert_eq!(messages, [SimMessage::ShipDestroyed]);
        assert_eq!(
            sounds,
            [SimSound::ShipDowned],
            "no klaxon on its first tick"
        );
    }

    /// Who fires the shot in [`disabled_then_killed`].
    pub(in crate::session) enum Gunner {
        /// The player.
        Player,
        /// A new independent NPC.
        Npc,
    }

    /// NPC `target`, its shields none and its armour low, is struck by a
    /// blaster shot of `gunner`'s, fired from 60 behind it along its
    /// heading, and on the tick the shot disables it a deadly stellar
    /// covering it kills it.
    pub(in crate::session) fn disabled_then_killed(
        session: &mut Session,
        target: NpcId,
        gunner: &Gunner,
    ) {
        let spec = crate::combat::weapon::WeaponSpec::new(&crate::session::tests::blaster());
        let weapon = spec.id;
        let npc = session.space.npc_mut(target).expect("there");
        npc.reserves.shield.now = 0.0;
        // Just above the third of its armour that disables it.
        npc.reserves.armor.now = npc.reserves.armor.max / 3.0 + 3.0;
        let at = npc.state.position;
        let heading = match gunner {
            Gunner::Player => session.ship.player.heading,
            Gunner::Npc => 0.0,
        };
        let from = at - crate::flight::facing(heading) * 60.0;
        let armed = || crate::combat::armament::Armament::new([(spec, 1)]);
        let gun = match gunner {
            Gunner::Player => {
                session.ship.player.position = from;
                session.ship.player.velocity = Vec2::ZERO;
                session.ship.fit.armament = armed();
                session.hold_trigger(crate::session::tests::FIRE);
                None
            }
            Gunner::Npc => {
                let mut npc = crate::testkit::npc(90, crate::stats::ShipStats::default());
                npc.state.position = from;
                npc.state.velocity = Vec2::ZERO;
                npc.state.heading = heading;
                npc.armament = armed();
                npc.target = Some(ShipRef::Npc(target));
                Some(session.space.traffic.add_npc(npc))
            }
        };
        let fire = |session: &mut Session| {
            if let Some(gun) = gun
                && let Some(npc) = session.space.npc_mut(gun)
            {
                npc.trigger = crate::combat::armament::Trigger {
                    only: Some(weapon),
                    ..crate::combat::armament::Trigger::default()
                };
            }
        };
        for _ in 0..40 {
            fire(session);
            let mut trial = session.clone();
            fight(&mut trial);
            let now = trial.space.npc(target).map(|npc| npc.condition);
            if now != Some(Condition::Intact) {
                assert_eq!(now, Some(Condition::Disabled), "the hit disables it");
                // The pass answers its strikes; the fight's own event says
                // a hit downed it.
                assert!(
                    trial.take_combat_events().contains(&CombatEvent::Disabled {
                        ship: ShipRef::Npc(target)
                    }),
                    "by a hit"
                );
                let at = session.space.npc(target).expect("there").state.position;
                session.chart.sites = vec![LandingSite {
                    frame_size: Some((2, 2)),
                    ..deadly_at(160, at.x, at.y)
                }];
                session.chart.listed = session.chart.sites.clone();
                fight(session);
                return;
            }
            *session = trial;
        }
        panic!("never disabled");
    }
}
