//! A pass's strikes weighed and answered one at a time
//! ([`Traffic::answer_in_turn`], [`Traffic::answer`]), as `_DamageShip`
//! runs hit by hit: each strike's answer flag and reactions read the NPCs
//! as the strikes before it left them.
//!
//! The answer flag (see [`answer_flag`](crate::ai::answer_flag)) reads
//! three things a reaction writes: the entry test reads the victim's
//! target and goal (+0x70 @0x3bb63, `_IsShipAttacking` @0x3bb6d), an NPC's
//! stray hit reads its maker's target (+0x70 @0x3b7d7), and an escort
//! near the player reads the victim's target (+0x70 == 0 @0x3b8da). So
//! the fight's strikes are weighed in the same turn as they are answered,
//! each after the reactions of the strikes before it
//! ([`Traffic::answer_in_turn`]). Nothing else the flag reads
//! (@0x3b408-0x3bb74) is written by a reaction. `_DamageShip`'s prologue
//! reads a plunderer's target and goal too (@0x3a95b-0x3a9a3), but for
//! the fight's can't-kill, not the flag; the fight reads its
//! [`Fighter`](crate::combat::Fighter)s as each phase began, a divergence
//! for two hits inside one phase, filed as rdm phases
//! play-feel-5-followups/phase-12-cant-kill-reads-plunderer-in-turn and
//! phase-13-fight-reads-targets-in-turn.
//!
//! Between two `_DamageShip` calls of one pass (the collision passes,
//! `_HandleShot`, `_HandleBeams`, `_HandleAsteroidHit`,
//! `_HandleShipDisplay`, `_HandlePlayer`) only the hit loop and its
//! callees run, and none of them writes a ship's target (+0x70), goal
//! (+0xc8c8) or the heading its low level last wanted (+0x68). The
//! target and goal the reaction writes (+0x70 @0x3bcf6, @0x3bd79; goal 4
//! by `_AIMakeShipAttackPlayer` @0x89d3e and `_DoGoodSamaritan` @0x82b82)
//! are in place when the next hit's focus test runs (@0x3bbf3-0x3bcac).
//! +0x68 only the low level writes (`_LowLevelAIHandler`, `_AIDispatch`
//! @0x8fb87, `_HandleShip` @0x347ba, `_DoShipCapture`, `_HandlePlayer`),
//! so each hit's test reads the heading wanted as the pass began
//! ([`wanted_headings`]).
//!
//! The person's grudge (@0x3be83-0x3beb3) lies on the path the focus
//! test's jump (@0x3bcac → 0x3bed2) skips, so it is decided in the same
//! turn ([`grudge`], [`Traffic::take_grudges`]).

use std::collections::BTreeMap;

use super::persons::GateView;
use super::{Traffic, World, autopilot};
use crate::ai::{Behaviour, Reaction, Surroundings};
use crate::chance::Chance;
use crate::combat::{ShipRef, Strike};
use crate::rulebook::RuleSource;
use crate::traffic::npc::{Mode, Npc, NpcId};

impl Traffic {
    /// Weighs and answers each of the fight's `strikes` in `world` in
    /// turn, in the order they landed, as `_DamageShip` runs hit by hit:
    /// first its answer flag, rolling on `chance` ([`Traffic::weigh`]),
    /// against the NPCs as the reactions of the strikes before it left
    /// them; then `hit` on the NPC it struck, if that is still in the
    /// traffic, with the strike weighed (the session's cloak on a hit,
    /// @0x3bb95-0x3bbdd, which `_DamageShip` runs between the flag and the
    /// reaction); then its grudge and reactions, as `behaviour` says
    /// ([`Traffic::answer`]).
    ///
    /// Three of the flag's reads see what an earlier reaction writes (see
    /// the module docs): the entry test (@0x3bb63-0x3bb74), an NPC's stray
    /// hit (@0x3b7bf-0x3b7dc) and an escort near the player (@0x3b8da).
    /// `hit` sees every strike on an NPC, a blast's or a shockwave's too;
    /// what it does with one that is no direct hit is its own affair.
    pub fn answer_in_turn(
        &mut self,
        behaviour: &(impl Behaviour + ?Sized),
        world: &World,
        strikes: &mut [Strike],
        chance: &mut dyn Chance,
        hit: &mut dyn FnMut(&mut Npc, &Strike),
    ) {
        let view = world.gate_view();
        let wanted = wanted_headings(&self.npcs);
        for strike in strikes {
            self.weigh(world, strike, chance);
            if let ShipRef::Npc(id) = strike.ship
                && let Some(npc) = self.npcs.iter_mut().find(|npc| npc.id == id)
            {
                hit(npc, strike);
            }
            self.answer_strike(behaviour, world, strike, &wanted, view);
        }
    }

    /// Each intact NPC answers `strike` in `world`, if it was answered
    /// ([`Strike::answered`]), as `behaviour` says, against the NPCs as
    /// they stand and the headings they `wanted` as the pass began, the
    /// quote gate reading `view`: one strike's turn of `Traffic::react`
    /// (see there), its grudge queued in the same turn ([`grudge`]).
    pub(super) fn answer_strike(
        &mut self,
        behaviour: &(impl Behaviour + ?Sized),
        world: &World,
        strike: &Strike,
        wanted: &BTreeMap<NpcId, i16>,
        view: GateView,
    ) {
        if !strike.answered {
            return;
        }
        let reactions: Vec<Option<Reaction>> = {
            let around = Surroundings {
                wanted: Some(wanted),
                ..world.around(&self.npcs)
            };
            self.grudges.extend(grudge(strike, &around));
            self.npcs
                .iter()
                .map(|npc| {
                    let answers = match world.arrival_hits {
                        RuleSource::Engine => true,
                        RuleSource::Bible => npc.arrived(),
                    };
                    (answers && npc.active_not_disabled())
                        .then(|| behaviour.react(npc, strike, &around))
                })
                .collect()
        };
        for (at, reaction) in reactions.into_iter().enumerate() {
            let Some(reaction) = reaction else {
                continue;
            };
            let id = self.npcs[at].id;
            // The victim's reaction ran when it took a target or was
            // provoked: an answered hit by no ship, which takes none,
            // always has a nominal damage above none, as the answer
            // flag's no-damage branch clears the rest (see
            // `answer_flag`), so it cuts the wait as `_DamageShip`
            // does for any source but slot 0 (@0x3bd7e-0x3be7c).
            let ran = reaction.target.is_some() || reaction.provoked > 0.0;
            let on_itself = strike.direct && ran && strike.ship == ShipRef::Npc(id);
            let attacks_player = on_itself && strike.by == Some(ShipRef::Player);
            if let Some(target) = reaction.target {
                self.npcs[at].target = Some(target);
            }
            if attacks_player {
                // `_DamageShip` sets the target (+0x70 @0x3bcf6) before
                // it calls `_AIMakeShipAttackPlayer` (@0x3be69), whose
                // gate reads the ship as this hit and those before it
                // left it (@0x89c53).
                self.turn_with_quote(id, view);
            }
            let npc = &mut self.npcs[at];
            if attacks_player {
                // `_AIMakeShipAttackPlayer`'s +0x6c = -1 (@0x89d47),
                // whatever the ship was doing: a ship held at its gate
                // loses its gate there too (`Npc::hold_at_gate`).
                npc.travel_set = false;
            }
            if on_itself {
                autopilot::cut_wait(npc);
            }
            npc.provoked += reaction.provoked;
            match npc.mode {
                Mode::JumpingIn { .. } | Mode::LeavingGate { .. } => {}
                Mode::Emerging { .. } if attacks_player || reaction.goal.is_some() => {
                    npc.hold_at_gate();
                }
                _ => {
                    if let Some(goal) = reaction.goal {
                        npc.goal = goal;
                    }
                }
            }
        }
    }
}

/// The heading each of `npcs` wanted as the pass began (+0x68,
/// [`Npc::wanted_heading`]), read before the first strike is answered. An NPC with no such frame is left out,
/// and the focus test takes the whole-degree bearing to its target
/// instead ([`Surroundings::wanted`](crate::ai::Surroundings::wanted)).
#[must_use]
pub fn wanted_headings(npcs: &[Npc]) -> BTreeMap<NpcId, i16> {
    npcs.iter()
        .filter_map(|npc| npc.wanted_heading().map(|heading| (npc.id, heading)))
        .collect()
}

/// The NPC whose person `strike` gives a grudge among `around`, the
/// NPCs as the strikes before it left them: the victim of an answered
/// direct hit of the player's, unless, by
/// [`RuleKey::HitFocus`](crate::RuleKey::HitFocus)'s engine reading, it
/// keeps its target against the player
/// ([`react::keeps_target`](crate::ai::react::keeps_target)), as the
/// focus test's jump (@0x3bcac) skips the grudge. By the other reading
/// every answered hit of the player's on an NPC counts. A hit on a
/// stellar's defence ship gives none (@0x3be88). An escort's
/// shot, a hit by no ship, and a blast or shockwave (never answered)
/// give none. Whether the victim answers (its arrival, its condition)
/// is not read, as the grudge's test reads neither.
#[must_use]
pub fn grudge(strike: &Strike, around: &Surroundings) -> Option<NpcId> {
    if strike.by != Some(ShipRef::Player) || !strike.answered {
        return None;
    }
    let ShipRef::Npc(id) = strike.ship else {
        return None;
    };
    // No grudge for a hit on a stellar's defence ship (@0x3be88).
    if around.npc(id).is_some_and(|npc| npc.defends.is_some()) {
        return None;
    }
    let kept = around.hit_focus == RuleSource::Engine
        && around
            .npc(id)
            .is_some_and(|npc| crate::ai::react::keeps_target(npc, npc, ShipRef::Player, around));
    (!kept).then_some(id)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use crate::ai::Goal;
    use crate::ai::fire::Manoeuvre;
    use crate::ai::nova::NovaAi;
    use crate::ai::pursuit::{Body, Frame};
    use crate::combat::fixture::strike;
    use crate::combat::hull::Condition;
    use crate::combat::{ShipRef, Strike};
    use crate::geometry::Vec2;
    use crate::rulebook::RuleSource;
    use crate::stats::ShipStats;
    use crate::traffic::fixture::npc;
    use crate::traffic::npc::{AiType, Mode, Npc, NpcId};
    use crate::traffic::{Traffic, World};
    use nova_sim_base::fixture::FAST;

    fn n(id: u32) -> ShipRef {
        ShipRef::Npc(NpcId(id))
    }

    /// NPC `id`, a wimpy trader at (`x`, `y`) facing up, idle.
    fn ship(id: u32, x: f32, y: f32) -> Npc {
        let mut ship = npc(id, ShipStats::new(FAST, &[]));
        ship.ai_type = AiType::WimpyTrader;
        ship.state.position = Vec2::new(x, y);
        ship
    }

    /// NPC `id` at (`x`, `y`) facing up, attacking `quarry` (its goal
    /// and its target), its frame wanting heading 0, dead ahead.
    fn hunter(id: u32, x: f32, y: f32, quarry: ShipRef) -> Npc {
        let mut hunter = ship(id, x, y);
        hunter.goal = Goal::Attack(quarry);
        hunter.target = Some(quarry);
        hunter.pursuit.frame = Some(Frame {
            goal: hunter.goal,
            bodies: [
                Some(Body {
                    manoeuvre: Manoeuvre::Dogfight,
                    heading: 0,
                }),
                None,
            ],
            speed_cap: None,
        });
        hunter
    }

    /// `npcs` once [`NovaAi`]'s traffic has answered `strikes` in one
    /// call, the player at `player`.
    fn answered_with(player: Vec2, npcs: Vec<Npc>, strikes: &[Strike]) -> Vec<Npc> {
        let mut traffic = Traffic::with_interval(1);
        traffic.npcs = npcs;
        let world = World {
            player: Some(crate::ai::fixture::player(player.x, player.y)),
            ..World::new(&[])
        };
        traffic.answer(&NovaAi::default(), &world, strikes);
        traffic.npcs
    }

    /// [`answered_with`] the player 100 above the centre.
    fn answered(npcs: Vec<Npc>, strikes: &[Strike]) -> Vec<Npc> {
        answered_with(Vec2::new(0.0, -100.0), npcs, strikes)
    }

    #[test]
    fn a_second_hit_in_a_pass_reads_the_turn_the_first_gave() {
        // The player's hit turns the idle ship on the player (+0x70
        // @0x3bcf6, goal 4 @0x89d3e); the next hit, by an NPC 60 right
        // (3600 > 0.25 × 10000), finds it attacking the player dead ahead
        // and keeps it: no target, no provocation (@0x3bcac).
        let ships = answered(
            vec![ship(1, 0.0, 0.0), ship(2, 60.0, 0.0)],
            &[strike(n(1), ShipRef::Player), strike(n(1), n(2))],
        );
        assert_eq!(ships[0].target, Some(ShipRef::Player));
        assert_eq!(ships[0].goal, Goal::Attack(ShipRef::Player));
        assert_eq!(ships[0].provoked, 24.0, "the player's hit alone");
        // The other way round, the NPC's hit leaves it idle, so the
        // player's then turns it, both provoking.
        let ships = answered(
            vec![ship(1, 0.0, 0.0), ship(2, 60.0, 0.0)],
            &[strike(n(1), n(2)), strike(n(1), ShipRef::Player)],
        );
        assert_eq!(ships[0].target, Some(ShipRef::Player));
        assert_eq!(ships[0].goal, Goal::Attack(ShipRef::Player));
        assert_eq!(ships[0].provoked, 48.0);
    }

    #[test]
    fn a_hit_measures_against_the_target_the_hit_before_it_set() {
        // Attacking X 200 above, the ship is hit first by A 20 above (400
        // is not above 0.25 × 40000): it takes A (+0x70 @0x3bcf6), its
        // goal still 4. Then B 60 right (3600 > 0.25 × 400, but not above
        // 0.25 × 40000) finds it attacking A, dead ahead, and is kept.
        let ships = answered(
            vec![
                hunter(1, 0.0, 0.0, n(3)),
                ship(3, 0.0, -200.0),
                ship(4, 0.0, -20.0),
                ship(5, 60.0, 0.0),
            ],
            &[strike(n(1), n(4)), strike(n(1), n(5))],
        );
        assert_eq!(ships[0].target, Some(n(4)));
        assert_eq!(ships[0].goal, Goal::Attack(n(3)));
        assert_eq!(ships[0].provoked, 24.0, "A's hit alone");
    }

    #[test]
    fn a_hit_reads_the_heading_the_ship_wanted_as_the_pass_began() {
        // Attacking X 300 above, its frame wanting heading 0, the ship is
        // hit by the player 100 right (10000 is not above 0.25 × 90000):
        // it turns on the player, 90 degrees off its nose. +0x68 is the
        // low level's alone, so the next hit, by B 60 below (3600 >
        // 0.25 × 10000), reads heading 0 still, not the bearing to the
        // player, and is kept.
        let ships = answered_with(
            Vec2::new(100.0, 0.0),
            vec![
                hunter(1, 0.0, 0.0, n(3)),
                ship(3, 0.0, -300.0),
                ship(5, 0.0, 60.0),
            ],
            &[strike(n(1), ShipRef::Player), strike(n(1), n(5))],
        );
        assert_eq!(ships[0].goal, Goal::Attack(ShipRef::Player));
        assert_eq!(ships[0].target, Some(ShipRef::Player));
        assert_eq!(ships[0].provoked, 24.0, "the player's hit alone");
    }

    /// The grudges [`NovaAi`]'s traffic queues answering `strikes` in one
    /// call, the player 100 above the centre, read by `focus`; then
    /// whether taking them emptied the queue.
    fn grudges(focus: RuleSource, npcs: Vec<Npc>, strikes: &[Strike]) -> Vec<NpcId> {
        let world = World {
            hit_focus: focus,
            ..World::new(&[])
        };
        grudges_in(&world, npcs, strikes).0
    }

    /// The grudges [`NovaAi`]'s traffic queues answering `strikes` in one
    /// call in `world`, the player 100 above the centre, and the NPCs it
    /// leaves; asserting taking them emptied the queue.
    fn grudges_in(world: &World, npcs: Vec<Npc>, strikes: &[Strike]) -> (Vec<NpcId>, Vec<Npc>) {
        let mut traffic = Traffic::with_interval(1);
        traffic.npcs = npcs;
        let world = World {
            player: Some(crate::ai::fixture::player(0.0, -100.0)),
            ..*world
        };
        traffic.answer(&NovaAi::default(), &world, strikes);
        let taken = traffic.take_grudges();
        assert_eq!(traffic.take_grudges(), [], "taken");
        (taken, traffic.npcs)
    }

    #[test]
    fn a_victim_that_does_not_answer_still_holds_the_grudge() {
        // The grudge's test (@0x3be83-0x3beb3) reads neither the victim's
        // arrival nor its condition: a ship still in its gate, by the other
        // reading of arrival hits, or a disabled one, answers nothing but
        // its person still holds the grudge.
        let by_player = strike(n(1), ShipRef::Player);
        let mut waiting = ship(1, 0.0, 0.0);
        waiting.mode = Mode::Emerging { ticks_left: 60 };
        let world = World {
            arrival_hits: RuleSource::Bible,
            ..World::new(&[])
        };
        let mut disabled = ship(1, 0.0, 0.0);
        disabled.condition = Condition::Disabled;
        for (why, world, victim) in [
            ("not arrived", world, waiting),
            ("disabled", World::new(&[]), disabled),
        ] {
            let (taken, ships) = grudges_in(&world, vec![victim], &[by_player]);
            assert_eq!(taken, [NpcId(1)], "{why}");
            assert_eq!(ships[0].target, None, "{why}: not answered");
            assert_eq!(ships[0].provoked, 0.0, "{why}: not answered");
        }
    }

    #[test]
    fn each_answered_hit_of_the_players_queues_its_victims_grudge() {
        let idle = || vec![ship(1, 0.0, 0.0), ship(2, 60.0, 0.0)];
        let by_player = strike(n(1), ShipRef::Player);
        for focus in [RuleSource::Engine, RuleSource::Bible] {
            assert_eq!(
                grudges(focus, idle(), &[by_player]),
                [NpcId(1)],
                "{focus:?}"
            );
            assert_eq!(
                grudges(focus, idle(), &[strike(n(1), n(2))]),
                [],
                "{focus:?}: an NPC's hit"
            );
            let unanswered = Strike {
                answered: false,
                ..by_player
            };
            assert_eq!(grudges(focus, idle(), &[unanswered]), [], "{focus:?}");
            let on_player = strike(ShipRef::Player, ShipRef::Player);
            assert_eq!(grudges(focus, idle(), &[on_player]), [], "{focus:?}");
        }
    }

    /// What [`Traffic::answer_in_turn`] makes of the fight's strikes.
    mod weighed_in_turn {
        use super::*;
        use crate::ai::fixture::{NEUTRAL, PIRATES, TRADERS, govts, player};
        use crate::bay::Carrier;
        use crate::catalog::EscortClass;
        use crate::combat::Nominal;
        use crate::escort::{EscortDuty, FormationSlot};
        use crate::govt::Governments;
        use nova_sim_base::fixture::Draws;

        /// NPC `id` of `govt`, an idle warship at (`x`, `y`) facing up.
        fn warship(id: u32, govt: crate::catalog::GovtId, x: f32, y: f32) -> Npc {
            crate::ai::fixture::ship(id, govt, AiType::Warship, x, y)
        }

        /// The fight's `strike`, unweighed, as the fight makes it.
        fn unweighed(strike: Strike) -> Strike {
            Strike {
                answered: false,
                ..strike
            }
        }

        /// A traffic of `npcs`, and the world they fly in with the
        /// player 100 above the centre.
        fn traffic_of(npcs: Vec<Npc>) -> Traffic {
            let mut traffic = Traffic::with_interval(1);
            traffic.npcs = npcs;
            traffic
        }

        fn world(govts: &Governments) -> World<'_> {
            World {
                player: Some(player(0.0, -100.0)),
                govts,
                ..World::new(&[])
            }
        }

        /// Each strike's answer flag and the NPCs once the traffic of
        /// `npcs` has answered `strikes` in turn, rolling on `draws`, and
        /// what it asked; with no hook.
        fn in_turn(
            npcs: Vec<Npc>,
            strikes: &[Strike],
            draws: &[u32],
        ) -> (Vec<bool>, Vec<Npc>, Vec<u32>) {
            let govts = govts(0, 0);
            let mut traffic = traffic_of(npcs);
            let mut strikes = strikes.to_vec();
            let mut chance = Draws::of(draws);
            traffic.answer_in_turn(
                &NovaAi::default(),
                &world(&govts),
                &mut strikes,
                &mut chance,
                &mut |_, _| {},
            );
            let answered = strikes.iter().map(|strike| strike.answered).collect();
            (answered, traffic.npcs, chance.asked)
        }

        /// Each strike's answer flag once the traffic of `npcs` has
        /// weighed `strikes` in a batch, before any reaction.
        fn in_a_batch(npcs: Vec<Npc>, strikes: &[Strike]) -> Vec<bool> {
            let govts = govts(0, 0);
            let mut traffic = traffic_of(npcs);
            let mut strikes = strikes.to_vec();
            traffic.weigh_strikes(&world(&govts), &mut strikes, &mut Draws::of(&[]));
            strikes.iter().map(|strike| strike.answered).collect()
        }

        #[test]
        fn a_fighters_hit_after_the_players_meets_the_entry_test_the_players_reaction_set() {
            // The player's hit turns the idle ship on the player (+0x70
            // @0x3bcf6, goal 4 @0x89d3e). The pirate fighter's hit then
            // finds it with a target and attacking, so the reaction
            // block's entry test (@0x3bb63-0x3bb74) holds it back. The
            // batch, reading the ship idle as the pass began, answers it.
            let ships = || {
                let mut fighter = warship(2, PIRATES, 60.0, 0.0);
                fighter.class = EscortClass::Fighter;
                vec![warship(1, TRADERS, 0.0, 0.0), fighter]
            };
            let strikes = [
                unweighed(strike(n(1), ShipRef::Player)),
                unweighed(strike(n(1), n(2))),
            ];
            let (answered, npcs, _) = in_turn(ships(), &strikes, &[]);
            assert_eq!(answered, [true, false]);
            assert_eq!(npcs[0].target, Some(ShipRef::Player));
            assert_eq!(npcs[0].goal, Goal::Attack(ShipRef::Player));
            assert_eq!(npcs[0].provoked, 24.0, "the player's hit alone");
            assert_eq!(in_a_batch(ships(), &strikes), [true, true], "the batch");
            // The other way round, the fighter's hit finds the ship idle,
            // and the player's, of no fighter class, passes the test.
            let reversed = [strikes[1], strikes[0]];
            let (answered, npcs, _) = in_turn(ships(), &reversed, &[]);
            assert_eq!(answered, [true, true], "reversed");
            assert_eq!(npcs[0].target, Some(ShipRef::Player), "reversed");
        }

        #[test]
        fn an_npcs_stray_hit_on_the_ship_that_just_hit_it_is_answered() {
            // V hits B, whose reaction takes V as its target (+0x70
            // @0x3bcf6); B's stray hit on V then reads that target
            // (@0x3b7bf-0x3b7dc) and is answered. In the batch B had none.
            let ships = || {
                vec![
                    warship(1, TRADERS, 0.0, 0.0),
                    warship(2, PIRATES, 60.0, 0.0),
                ]
            };
            let strikes = [
                unweighed(strike(n(2), n(1))),
                Strike {
                    intended: false,
                    ..unweighed(strike(n(1), n(2)))
                },
            ];
            let (answered, npcs, _) = in_turn(ships(), &strikes, &[]);
            assert_eq!(answered, [true, true]);
            assert_eq!(npcs[1].target, Some(n(1)), "B on V");
            assert_eq!(npcs[0].target, Some(n(2)), "V on B");
            assert_eq!(in_a_batch(ships(), &strikes), [true, false], "the batch");
        }

        #[test]
        fn the_players_fighters_hit_on_a_ship_the_players_hit_turned_on_it_is_let_pass() {
            // The player's hit turns V, 100 from the player, on the
            // player; its own fighter's hit then finds V targeting the
            // player within 320 (@0x3b8da) and is let pass. The fighter is
            // of no fighter class, so the entry test can't hold it back.
            let ships = || {
                let mut fighter = warship(2, PIRATES, 60.0, 0.0);
                fighter.escort = Some(EscortDuty {
                    formation: FormationSlot::default(),
                    order: None,
                });
                fighter.carrier = Some(Carrier {
                    ship: ShipRef::Player,
                    window: 0.0,
                    reach: 0.0,
                });
                assert_ne!(fighter.class, EscortClass::Fighter);
                vec![warship(1, TRADERS, 0.0, 0.0), fighter]
            };
            let strikes = [
                unweighed(strike(n(1), ShipRef::Player)),
                unweighed(strike(n(1), n(2))),
            ];
            let (answered, npcs, _) = in_turn(ships(), &strikes, &[]);
            assert_eq!(answered, [true, false]);
            assert_eq!(npcs[0].target, Some(ShipRef::Player));
            assert_eq!(in_a_batch(ships(), &strikes), [true, true], "the batch");
        }

        #[test]
        fn an_answer_zeroes_the_stray_provocation_the_next_stray_hit_reads() {
            // The player's hit is answered and zeroes the neutral's 60
            // (@0x3bb83); its stray hit after, not forgiven by the roll
            // (`_Rand(4)` @0x3b830 comes up 0), finds it below 50 and is
            // not answered, adding its 7.
            let mut neutral = warship(1, NEUTRAL, 0.0, 0.0);
            neutral.stray = 60.0;
            let stray = Strike {
                intended: false,
                weight: 4.0,
                ..unweighed(strike(n(1), ShipRef::Player))
            };
            let strikes = [unweighed(strike(n(1), ShipRef::Player)), stray];
            let (answered, npcs, asked) = in_turn(vec![neutral.clone()], &strikes, &[0]);
            assert_eq!(answered, [true, false]);
            assert_eq!(npcs[0].stray, 7.0);
            assert_eq!(asked, [4], "the stray hit's roll, on its own path");
            // Alone, the stray hit reads the 60 and is answered.
            let (answered, npcs, asked) = in_turn(vec![neutral], &[stray], &[0]);
            assert_eq!(answered, [true]);
            assert_eq!(npcs[0].stray, 0.0);
            assert_eq!(asked, [4]);
        }

        #[test]
        fn the_hook_sees_each_strike_on_an_npc_weighed_before_its_reaction() {
            let govts = govts(0, 0);
            let mut traffic = traffic_of(vec![warship(1, TRADERS, 0.0, 0.0)]);
            let by_player = unweighed(strike(n(1), ShipRef::Player));
            let mut strikes = [
                by_player,
                strike(ShipRef::Player, n(1)),
                by_player,
                unweighed(strike(n(9), ShipRef::Player)),
            ];
            let mut seen = Vec::new();
            traffic.answer_in_turn(
                &NovaAi::default(),
                &world(&govts),
                &mut strikes,
                &mut Draws::of(&[]),
                &mut |npc, strike| seen.push((npc.id, strike.answered, npc.goal, npc.target)),
            );
            let on_player = Goal::Attack(ShipRef::Player);
            assert_eq!(
                seen,
                [
                    (NpcId(1), true, Goal::Idle, None),
                    (NpcId(1), true, on_player, Some(ShipRef::Player)),
                ],
                "not the strike on the player, nor the one on a ship gone"
            );
        }

        #[test]
        fn the_hook_may_change_the_ship_before_its_reaction_reads_it() {
            // The hook runs between the flag and the reaction, as
            // `_DamageShip` cloaks a ship (@0x3bb95-0x3bbdd) before it
            // answers: a ship the hook takes out of the fight answers
            // nothing.
            let govts = govts(0, 0);
            let mut traffic = traffic_of(vec![warship(1, TRADERS, 0.0, 0.0)]);
            let mut strikes = [unweighed(strike(n(1), ShipRef::Player))];
            traffic.answer_in_turn(
                &NovaAi::default(),
                &world(&govts),
                &mut strikes,
                &mut Draws::of(&[]),
                &mut |npc, _| npc.condition = Condition::Disabled,
            );
            assert!(strikes[0].answered);
            assert_eq!(traffic.npcs[0].target, None);
            assert_eq!(traffic.npcs[0].provoked, 0.0);
        }

        #[test]
        fn a_strike_the_flag_clears_gets_no_reaction_and_no_grudge() {
            // A hit of no damage (@0x3b6f2-0x3b73e), though it came in
            // marked answered: the weighing rewrites the mark.
            let govts = govts(0, 0);
            let mut traffic = traffic_of(vec![warship(1, TRADERS, 0.0, 0.0)]);
            let mut strikes = [Strike {
                nominal: Nominal {
                    mass: 0.0,
                    energy: 0.0,
                },
                ..strike(n(1), ShipRef::Player)
            }];
            assert!(strikes[0].answered);
            traffic.answer_in_turn(
                &NovaAi::default(),
                &world(&govts),
                &mut strikes,
                &mut Draws::of(&[]),
                &mut |_, _| {},
            );
            assert!(!strikes[0].answered);
            assert_eq!(traffic.npcs[0].target, None);
            assert_eq!(traffic.npcs[0].provoked, 0.0);
            assert_eq!(traffic.take_grudges(), []);
            // Answered, the same hit of 12 queues its grudge.
            let mut strikes = [unweighed(strike(n(1), ShipRef::Player))];
            traffic.answer_in_turn(
                &NovaAi::default(),
                &world(&govts),
                &mut strikes,
                &mut Draws::of(&[]),
                &mut |_, _| {},
            );
            assert_eq!(traffic.take_grudges(), [NpcId(1)]);
        }
    }

    #[test]
    fn a_hit_on_a_stellars_defence_ship_holds_no_grudge() {
        let mut guard = ship(1, 0.0, 0.0);
        guard.defends = Some(crate::catalog::StellarId(128));
        let by_player = strike(n(1), ShipRef::Player);
        for focus in [RuleSource::Engine, RuleSource::Bible] {
            assert_eq!(
                grudges(focus, vec![guard.clone()], &[by_player]),
                [],
                "{focus:?}: @0x3be88"
            );
        }
    }

    #[test]
    fn a_hit_the_ship_keeps_its_target_against_holds_no_grudge_by_the_engine() {
        // Attacking X 150 above, dead ahead: the player 100 above is kept
        // (10000 > 0.25 × 22500), skipping the grudge (@0x3bcac).
        let hunting = || vec![hunter(1, 0.0, 0.0, n(3)), ship(3, 0.0, -150.0)];
        let by_player = strike(n(1), ShipRef::Player);
        assert_eq!(grudges(RuleSource::Engine, hunting(), &[by_player]), []);
        assert_eq!(
            grudges(RuleSource::Bible, hunting(), &[by_player]),
            [NpcId(1)],
            "every answered hit by the other reading"
        );
        // Idle, the player's first hit turns it on the player and holds
        // the grudge; its second, the player now its faced target, is kept.
        assert_eq!(
            grudges(
                RuleSource::Engine,
                vec![ship(1, 0.0, 0.0)],
                &[by_player, by_player]
            ),
            [NpcId(1)]
        );
    }
}
