//! A pass's strikes answered one at a time ([`Traffic::answer`]), as
//! `_DamageShip` runs hit by hit: each strike's reactions read the NPCs
//! as the strikes before it left them.
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

use crate::ai::Surroundings;
use crate::combat::{ShipRef, Strike};
use crate::rulebook::RuleSource;
use crate::traffic::npc::{Npc, NpcId};

#[cfg(doc)]
use super::Traffic;

/// The heading each of `npcs` wanted as the pass began (+0x68): the last
/// body's heading of its pursuit frame while that frame is for its goal
/// ([`Pursuit::frame`](crate::ai::pursuit::Pursuit::frame)), read before
/// the first strike is answered. An NPC with no such frame is left out,
/// and the focus test takes the whole-degree bearing to its target
/// instead ([`Surroundings::wanted`](crate::ai::Surroundings::wanted)).
#[must_use]
pub fn wanted_headings(npcs: &[Npc]) -> BTreeMap<NpcId, i16> {
    npcs.iter()
        .filter_map(|npc| {
            npc.pursuit
                .frame
                .filter(|frame| frame.goal == npc.goal)
                .and_then(|frame| frame.last())
                .map(|body| (npc.id, body.heading))
        })
        .collect()
}

/// The NPC whose person `strike` gives a grudge among `around`, the
/// NPCs as the strikes before it left them: the victim of an answered
/// direct hit of the player's, unless, by
/// [`RuleKey::HitFocus`](crate::RuleKey::HitFocus)'s engine reading, it
/// keeps its target against the player
/// ([`react::keeps_target`](crate::ai::react::keeps_target)), as the
/// focus test's jump (@0x3bcac) skips the grudge. By the other reading
/// every answered hit of the player's on an NPC counts. An escort's
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
