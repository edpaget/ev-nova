//! The swarm branch of `_LowLevelAIHandler`'s substates 5, 0x11 and 7
//! (@0x85c87-0x85cbf, @0x8660b-0x8663d, @0x867e0-0x86818 in the `EV Nova`
//! executable), as [`plunder_dock::swarm_branch`] runs it: with a word
//! above 0 (+0xc906, signed, `testw; jle`), `_AIMaintainFormation(ship, 0)`
//! nudges the ship toward its slot and it takes its swarm leader's glow.
//!
//! - **Gates.** Each block needs a target (+0x70 not -1) and a ship not
//!   disabled (@0x85c70-0x85c7e, @0x8657d-0x8658b, @0x867cc-0x867da). A
//!   frame stands for both: [`pursuit::step`](crate::ai::pursuit::step)
//!   writes one only at the quarry of its goal, which is its +0x70, as the
//!   ship's own [`Npc::target`] lags a tick and is none for a fleeing
//!   escort, and only an intact ship's pursuit runs. Searching for a lost
//!   target it stops, so it runs no body ([`lost_sight`](super::lost_sight)).
//! - **Which goals.** The blocks read the substate, not the goal, so the
//!   branch runs in every goal flying 5, 7 or 0x11: attack, a stand-off
//!   band, an intact plunder (goal 0xd flies 7 and 0x11), flee close.
//! - **How many times.** Once per block run, so once per
//!   [`Manoeuvre::Flee`], [`Manoeuvre::Rush`] or [`Manoeuvre::Approach`]
//!   body of the frame: `[Flee, Rush]` twice, `[Dogfight, Rush]` once, and
//!   `[Swarm]`, `[Dogfight]`, `[BreakOff]` and `[Snipe]` never.
//! - **Whose slot** ([`slot_place`]). The nudge goes toward +0x28/+0x2c
//!   ([`Npc::slot_place`]), which the formation parent's
//!   `_AICalcFormationPositions` wrote, so +0xc92e picks the slot and not
//!   the swarm leader: attacking with a target it is the swarm leader; in
//!   any other state it is the ship's leader, the player included. A ship
//!   with no parent (+0xc92e above 63) is never nudged (@0x84f30), and
//!   nor is one whose jump is under way (+0x50 above 0, @0x84f16,
//!   [`formation_flight::maintain`](super::formation_flight::maintain)).
//!   A stale parent ([`fleet_formation`](super::fleet_formation)) opens the
//!   nudge for a ship of AI type 4 or below too, toward a place that may
//!   stand fixed. By the other reading of
//!   [`RuleKey::NpcFormation`](crate::RuleKey::NpcFormation), where nothing
//!   flies an NPC parent's slots, no such slot nudges; the player's
//!   formation is not that key's, and flies by either.
//! - **The glow copy** has no gate beyond the word: it reads the swarm
//!   leader's +0xc8d4, as the tick began, for every ship naming one, AI
//!   type 4 or below included ([`GlowCopy::Swarm`]). `_HandleShip` comes
//!   after, so it lands before the glow step
//!   ([`ship_display`](super::ship_display)). The copy of an empty slot's
//!   stale record is a read of a dead record, which can't be replicated:
//!   [`swarm::lead`] gives none there.
//! - **When.** It is low level, so it runs in `_AIDispatch`, on the advance
//!   pass alone: [`Traffic::pursue`] calls it at its end, for each ship
//!   whose pursuit ran this tick, and double time's second pass runs
//!   neither (see [`Traffic::move_in`]). So the fire after it reads the
//!   nudged position, as the original's does, and an
//!   [`immobile`](Npc::immobile) ship's undo of its move doesn't eat it.
//!
//! **Deferred.** Each substate's heading is [`pursuit::step`]'s, a moment
//! before the nudge and off the position as the tick began, where subs 5
//! and 7 branch first, and every ship's step runs before any ship's nudge,
//! where a lower slot's nudge is in place for a higher slot's high level.
//! The next tick reads the true position, and no boundary the player can
//! see lies between.
//!
//! Neither the nudge nor the copy plays a sound or draws a random number.
//!
//! [`GlowCopy::Swarm`]: super::ship_display::GlowCopy::Swarm
//! [`pursuit::step`]: crate::ai::pursuit::step

use super::autopilot::TopSpeeds;
use super::npc::Npc;
use super::plunder_dock;
use super::{Traffic, World};
use crate::ai::fire::Manoeuvre;
use crate::ai::swarm::{self, SwarmLead};
use crate::flight::{FlightRules, NpcFormation};
use crate::geometry::Vec2;

/// The place `_AIMaintainFormation(ship, 0)` nudges `npc` toward under
/// `flight` (see the module docs): its [`Npc::slot_place`] while it holds a
/// parent, none else, or on an NPC parent by the other reading of
/// [`RuleKey::NpcFormation`](crate::RuleKey::NpcFormation).
#[must_use]
pub(crate) fn slot_place(npc: &Npc, flight: FlightRules) -> Option<Vec2> {
    let parent = npc.form_parent?;
    (parent == 0 || flight.npc_formation == NpcFormation::Original)
        .then_some(npc.slot_place)
        .flatten()
}

/// Whether a body of `manoeuvre` is a block with the swarm branch: subs
/// 5, 0x11 and 7.
fn branches(manoeuvre: Manoeuvre) -> bool {
    matches!(
        manoeuvre,
        Manoeuvre::Flee | Manoeuvre::Rush | Manoeuvre::Approach
    )
}

impl Traffic {
    /// The swarm branch of each NPC whose pursuit `ran` this tick (one flag
    /// an NPC), in `world`, once for each body of its frame that has one
    /// (see the module docs).
    pub(super) fn swarm_low_level(&mut self, world: &World, ran: &[bool]) {
        let around = world.around(&self.npcs);
        let leads: Vec<Option<SwarmLead>> = self
            .npcs
            .iter()
            .map(|npc| swarm::lead(npc, &around))
            .collect();
        for ((npc, lead), _) in self
            .npcs
            .iter_mut()
            .zip(leads)
            .zip(ran)
            .filter(|&(_, &ran)| ran)
        {
            let Some(frame) = npc.pursuit.frame else {
                continue;
            };
            let slot = slot_place(npc, world.flight);
            let handling = npc.flying_handling(TopSpeeds::own_of(npc).own, world.flight);
            for _ in frame.iter().filter(|body| branches(body.manoeuvre)) {
                plunder_dock::swarm_branch(npc, lead, slot, &handling);
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::lost_sight::{LostSight, SearchTimer};
    use crate::ai::regroup::Kept;
    use crate::ai::{Behaviour, Decision, Goal, PlayerSide, Surroundings};
    use crate::catalog::GovtId;
    use crate::chance::{Chance, NeverFires};
    use crate::combat::ShipRef;
    use crate::combat::hull::Condition;
    use crate::escort::{EscortDuty, FORMATION_NUDGE, FormationSlot};
    use crate::flight::ShipState;
    use crate::stats::ShipStats;
    use crate::traffic::fixture::npc;
    use crate::traffic::npc::NpcId;
    use crate::traffic::ship_display::GlowCopy;
    use nova_sim_base::fixture::FAST;

    /// Every NPC keeps its goal and its target.
    #[derive(Debug)]
    struct Keep;

    impl Behaviour for Keep {
        fn decide(&self, npc: &Npc, _: &Surroundings, _: &mut dyn Chance) -> Goal {
            npc.goal
        }

        fn target(&self, npc: &Npc, _: &Surroundings) -> Option<ShipRef> {
            npc.target
        }
    }

    /// Every NPC keeps its goal, and its decision finds its target out of
    /// sight.
    #[derive(Debug)]
    struct Searches;

    impl Behaviour for Searches {
        fn decide(&self, npc: &Npc, _: &Surroundings, _: &mut dyn Chance) -> Goal {
            npc.goal
        }

        fn decision(&self, npc: &Npc, _: &Surroundings, _: &mut dyn Chance) -> Decision {
            Decision {
                goal: npc.goal,
                lost_sight: Some(LostSight::Searching(5)),
                ..Decision::default()
            }
        }
    }

    const FEDS: Option<GovtId> = Some(GovtId(128));
    /// Its swarm leader's glow as the tick began.
    const GLOW: i16 = 17;
    /// The player, 2000 up the screen: outside every box.
    const FAR_PLAYER: Vec2 = Vec2::new(0.0, -2000.0);
    /// Its swarm leader's berth.
    const SWARM_BERTH: u32 = 1;

    fn world(player: Vec2) -> World<'static> {
        let side: PlayerSide = crate::ai::fixture::player(player.x, player.y);
        World {
            player: Some(side),
            ..World::new(&[])
        }
    }

    /// A [`FAST`] ship of a class that doesn't swarm, at `at`.
    fn hull_at(at: Vec2) -> Npc {
        let mut ship = npc(0, ShipStats::new(FAST, &[]));
        ship.govt = FEDS;
        ship.state.position = at;
        ship
    }

    /// Traffic of a swarm leader at the centre at berth 1, and at berth 2
    /// a ship at (400, 300) led by it and naming it, attacking the player
    /// at [`FAR_PLAYER`], `change`d. So it approaches, its swarm leader
    /// being its own leader (@0x8e66b).
    fn traffic(change: impl FnOnce(&mut Npc, NpcId)) -> (Traffic, NpcId) {
        let mut traffic = Traffic::with_interval(1);
        let mut lead = hull_at(Vec2::ZERO);
        lead.display.glow = GLOW;
        let lead = traffic.add_npc_at(lead, SWARM_BERTH);
        let mut ship = hull_at(Vec2::new(400.0, 300.0));
        ship.leader = Some(lead);
        ship.goal = Goal::Attack(ShipRef::Player);
        ship.target = Some(ShipRef::Player);
        ship.swarm_leader = Some(SWARM_BERTH);
        change(&mut ship, lead);
        let id = traffic.add_npc_at(ship, 2);
        (traffic, id)
    }

    fn of(traffic: &Traffic, id: NpcId) -> &Npc {
        traffic
            .npcs()
            .iter()
            .find(|npc| npc.id == id)
            .expect("there")
    }

    /// `traffic`'s think half in `world`.
    fn think_in(traffic: &mut Traffic, world: &World<'_>) {
        traffic.think_holding(&Keep, *world, &[], &mut NeverFires);
    }

    /// The bodies of `id`'s frame, for the test's premise.
    fn bodies(traffic: &Traffic, id: NpcId) -> Vec<Manoeuvre> {
        of(traffic, id)
            .pursuit
            .frame
            .iter()
            .flat_map(|frame| frame.iter().map(|body| body.manoeuvre))
            .collect()
    }

    /// One nudge of `ship`, by its thrust.
    fn nudge(ship: &Npc) -> f32 {
        ship.flying_handling(TopSpeeds::own_of(ship).own, FlightRules::default())
            .accel
            * FORMATION_NUDGE
    }

    /// Where a ship at `from` ends after `times` nudges of `by` toward
    /// `to`, both axes far off it.
    fn nudged_from(from: Vec2, to: Vec2, by: f32, times: u32) -> Vec2 {
        let way =
            |from: f32, to: f32| (0..times).fold(from, |at, _| at + (to - from).signum() * by);
        Vec2::new(way(from.x, to.x), way(from.y, to.y))
    }

    const START: Vec2 = Vec2::new(400.0, 300.0);

    /// Asserts `id` of `traffic` went `times` nudges from [`START`] toward
    /// `place`, and copied `glow`.
    fn assert_branched(traffic: &Traffic, id: NpcId, place: Vec2, times: u32, glow: GlowCopy) {
        let ship = of(traffic, id);
        assert_eq!(
            ship.state.position,
            nudged_from(START, place, nudge(ship), times)
        );
        assert_eq!(ship.display.drive.glow, glow);
    }

    #[test]
    fn a_fleet_escort_attacking_with_a_swarm_leader_is_nudged_to_its_slot_on_it() {
        let (mut traffic, id) = traffic(|_, _| {});
        think_in(&mut traffic, &world(FAR_PLAYER));
        assert_eq!(bodies(&traffic, id), [Manoeuvre::Approach]);
        let place = of(&traffic, id).slot_place.expect("a place");
        assert_branched(&traffic, id, place, 1, GlowCopy::Swarm(GLOW));
        // Its slot is on its swarm leader, and the nudge came before it
        // moved: it thinks in place.
        let slot = of(&traffic, id).fleet_slot.expect("a slot");
        assert_eq!(place, slot.position(&ShipState::default()));
        assert_eq!(of(&traffic, id).state.velocity, Vec2::ZERO);
    }

    #[test]
    fn it_flies_on_from_the_nudged_place_and_shows_the_glow_as_the_tick_began() {
        let (mut traffic, id) = traffic(|_, _| {});
        let world = world(FAR_PLAYER);
        think_in(&mut traffic, &world);
        let nudged = of(&traffic, id).state.position;
        traffic.fly_holding(world, &std::collections::BTreeSet::new(), &mut NeverFires);
        let ship = of(&traffic, id);
        assert_eq!(ship.state.position, nudged + ship.state.velocity);
        // Copied, then stepped by one thrusting frame (`bank_and_glow`).
        assert!(
            (ship.display.glow - GLOW).abs() <= 1,
            "{}",
            ship.display.glow
        );
    }

    #[test]
    fn a_players_escort_attacking_with_a_swarm_leader_is_nudged_the_same() {
        let (mut traffic, id) = traffic(|ship, _| {
            ship.leader = None;
            ship.escort = Some(EscortDuty {
                formation: FormationSlot {
                    slot: 2,
                    ships: 2,
                    spacing: 30.0,
                },
                order: None,
            });
        });
        think_in(&mut traffic, &world(FAR_PLAYER));
        assert_eq!(bodies(&traffic, id), [Manoeuvre::Approach]);
        let ship = of(&traffic, id);
        assert_eq!(ship.form_parent, Some(SWARM_BERTH));
        let slot = ship.fleet_slot.expect("a slot on its swarm leader");
        let place = slot.position(&ShipState::default());
        assert_branched(&traffic, id, place, 1, GlowCopy::Swarm(GLOW));
    }

    /// `ship` fleeing the player within reach of it.
    fn fleeing(ship: &mut Npc) {
        ship.goal = Goal::Flee(ShipRef::Player);
        ship.target = None;
    }

    #[test]
    fn a_led_ship_fleeing_close_is_nudged_to_its_slot_on_its_own_leader() {
        // Its parent is its leader, which isn't its swarm leader: goal 3 is
        // not `_IsShipAttacking` (@0x82f6a).
        let (mut traffic, id) = traffic(|ship, _| {
            fleeing(ship);
            ship.swarm_leader = Some(9);
        });
        let mut other = hull_at(Vec2::new(-900.0, 900.0));
        other.display.glow = 5;
        traffic.add_npc_at(other, 9);
        think_in(&mut traffic, &world(START + Vec2::new(100.0, 100.0)));
        assert_eq!(bodies(&traffic, id), [Manoeuvre::Flee]);
        assert_eq!(of(&traffic, id).form_parent, Some(SWARM_BERTH));
        let place = of(&traffic, id).slot_place.expect("a place");
        assert_eq!(
            place,
            of(&traffic, id)
                .fleet_slot
                .expect("a slot")
                .position(&ShipState::default())
        );
        assert_branched(&traffic, id, place, 1, GlowCopy::Swarm(5));
    }

    #[test]
    fn a_players_escort_fleeing_close_is_nudged_to_its_slot_on_the_player() {
        let slot = FormationSlot {
            slot: 2,
            ships: 2,
            spacing: 30.0,
        };
        let (mut traffic, id) = traffic(|ship, _| {
            fleeing(ship);
            ship.leader = None;
            ship.escort = Some(EscortDuty {
                formation: slot,
                order: None,
            });
        });
        let player = START + Vec2::new(100.0, 100.0);
        think_in(&mut traffic, &world(player));
        assert_eq!(bodies(&traffic, id), [Manoeuvre::Flee]);
        let side = crate::ai::fixture::player(player.x, player.y);
        let place = slot.position(&side.state);
        assert_eq!(of(&traffic, id).slot_place, Some(place));
        assert_branched(&traffic, id, place, 1, GlowCopy::Swarm(GLOW));
    }

    #[test]
    fn flee_close_then_rush_nudges_and_copies_twice() {
        let (mut traffic, id) = traffic(|ship, _| {
            fleeing(ship);
            ship.afterburner = true;
        });
        think_in(&mut traffic, &world(START + Vec2::new(100.0, 100.0)));
        assert_eq!(
            bodies(&traffic, id),
            [Manoeuvre::Flee, Manoeuvre::Rush],
            "its attacker in the dogfight box"
        );
        let place = of(&traffic, id).slot_place.expect("a place");
        assert_branched(&traffic, id, place, 2, GlowCopy::Swarm(GLOW));
    }

    #[test]
    fn a_dogfight_rushing_in_the_same_frame_nudges_once() {
        let (mut traffic, id) = traffic(|ship, _| {
            ship.afterburner = true;
            ship.state.heading = 0.0;
        });
        // Up and left of it, 100 off each axis, within 30 of its heading of
        // 0 once rounded: the rush entry after the dogfight (@0x86371).
        think_in(&mut traffic, &world(START + Vec2::new(-90.0, -150.0)));
        assert_eq!(bodies(&traffic, id), [Manoeuvre::Dogfight, Manoeuvre::Rush]);
        let place = of(&traffic, id).slot_place.expect("a place");
        assert_branched(&traffic, id, place, 1, GlowCopy::Swarm(GLOW));
    }

    /// Asserts `id` of `traffic`, in a frame of `body`, ran no branch.
    fn assert_unbranched(traffic: &Traffic, id: NpcId, body: Manoeuvre) {
        assert_eq!(bodies(traffic, id), [body]);
        assert_eq!(of(traffic, id).state.position, START);
        assert_eq!(of(traffic, id).display.drive.glow, GlowCopy::None);
    }

    #[test]
    fn a_frame_of_a_swarm_a_dogfight_or_a_snipe_runs_no_branch() {
        // Its class swarms, and its leader is another.
        let (mut swarm, id) = traffic(|ship, lead| {
            ship.hull.swarms = true;
            ship.leader = Some(NpcId(lead.0 + 50));
        });
        if let Some(lead) = swarm.npcs_mut().first_mut() {
            lead.hull.swarms = true;
            lead.target = Some(ShipRef::Player);
        }
        think_in(&mut swarm, &world(FAR_PLAYER));
        assert_unbranched(&swarm, id, Manoeuvre::Swarm);
        let (mut dogfight, id) = traffic(|_, _| {});
        think_in(&mut dogfight, &world(START + Vec2::new(10.0, 10.0)));
        assert_unbranched(&dogfight, id, Manoeuvre::Dogfight);
        let (mut snipe, id) = traffic(|ship, _| {
            ship.goal = Goal::Snipe(ShipRef::Player);
        });
        think_in(&mut snipe, &world(FAR_PLAYER));
        assert_unbranched(&snipe, id, Manoeuvre::Snipe);
    }

    #[test]
    fn a_ship_of_ai_type_4_or_below_takes_the_glow_and_no_nudge() {
        // No leader, so no parent (+0xc92e -1, @0x84f30).
        let (mut traffic, id) = traffic(|ship, _| {
            fleeing(ship);
            ship.leader = None;
        });
        think_in(&mut traffic, &world(START + Vec2::new(100.0, 100.0)));
        assert_eq!(bodies(&traffic, id), [Manoeuvre::Flee]);
        assert_eq!(of(&traffic, id).state.position, START);
        assert_eq!(of(&traffic, id).display.drive.glow, GlowCopy::Swarm(GLOW));
    }

    #[test]
    fn with_a_stale_parent_it_is_nudged_too_toward_the_place_it_holds() {
        let fixed = Vec2::new(-700.0, -700.0);
        let (mut traffic, id) = traffic(|ship, _| {
            fleeing(ship);
            ship.leader = None;
            ship.form_parent = Some(SWARM_BERTH);
            ship.slot_place = Some(fixed);
        });
        think_in(&mut traffic, &world(START + Vec2::new(100.0, 100.0)));
        assert_eq!(of(&traffic, id).slot_place, Some(fixed), "unmarked");
        assert_branched(&traffic, id, fixed, 1, GlowCopy::Swarm(GLOW));
    }

    #[test]
    fn with_no_word_or_word_0_it_runs_neither() {
        let (mut unnamed, id) = traffic(|ship, _| ship.swarm_leader = None);
        think_in(&mut unnamed, &world(FAR_PLAYER));
        assert_unbranched(&unnamed, id, Manoeuvre::Approach);
        // Word 0 is the player's slot, and the gate is `testw; jle`: with a
        // place on the player to nudge to and a ship at berth 0 to copy, it
        // takes neither.
        let (mut on_player, id) = traffic(|ship, _| {
            ship.swarm_leader = Some(0);
            ship.fleet_slot = Some(FormationSlot {
                slot: 2,
                ships: 2,
                spacing: 30.0,
            });
        });
        let mut at_berth_0 = hull_at(Vec2::new(-900.0, 900.0));
        at_berth_0.display.glow = 5;
        on_player.add_npc_at(at_berth_0, 0);
        think_in(&mut on_player, &world(FAR_PLAYER));
        assert_eq!(of(&on_player, id).form_parent, Some(0));
        assert!(of(&on_player, id).slot_place.is_some(), "a place to go to");
        assert_unbranched(&on_player, id, Manoeuvre::Approach);
    }

    #[test]
    fn with_a_warp_under_way_it_copies_and_is_not_nudged() {
        let (mut traffic, id) = traffic(|ship, _| ship.warp.stage = 1.0);
        think_in(&mut traffic, &world(FAR_PLAYER));
        assert_eq!(bodies(&traffic, id), [Manoeuvre::Approach]);
        assert_eq!(of(&traffic, id).state.position, START);
        assert_eq!(of(&traffic, id).display.drive.glow, GlowCopy::Swarm(GLOW));
    }

    #[test]
    fn disabled_or_searching_it_runs_neither() {
        let (mut disabled, id) = traffic(|ship, _| ship.condition = Condition::Disabled);
        think_in(&mut disabled, &world(FAR_PLAYER));
        assert_eq!(of(&disabled, id).state.position, START);
        assert_eq!(of(&disabled, id).display.drive.glow, GlowCopy::None);
        let (mut searching, id) = traffic(|_, _| {});
        searching.think_holding(&Searches, world(FAR_PLAYER), &[], &mut NeverFires);
        assert_eq!(
            of(&searching, id).lost_sight,
            SearchTimer::Searching(5),
            "its decision wrote it"
        );
        assert_eq!(of(&searching, id).state.position, START);
        assert_eq!(of(&searching, id).display.drive.glow, GlowCopy::None);
    }

    /// Asserts a ship holding an Approach frame from its last think,
    /// `change`d so that its pursuit runs nothing, keeps that frame and
    /// runs no branch on the next think.
    fn assert_a_kept_frame_runs_no_branch(change: impl FnOnce(&mut Npc)) {
        let (mut traffic, id) = traffic(|_, _| {});
        let world = world(FAR_PLAYER);
        think_in(&mut traffic, &world);
        let nudged = of(&traffic, id).state.position;
        assert_ne!(nudged, START);
        let ship = traffic.npcs_mut().iter_mut().find(|npc| npc.id == id);
        let ship = ship.expect("there");
        change(ship);
        ship.display.drive.glow = GlowCopy::None;
        think_in(&mut traffic, &world);
        assert_eq!(bodies(&traffic, id), [Manoeuvre::Approach], "kept");
        assert_eq!(of(&traffic, id).state.position, nudged);
        assert_eq!(of(&traffic, id).display.drive.glow, GlowCopy::None);
    }

    #[test]
    fn a_frame_kept_from_the_last_tick_runs_no_branch_under_an_ai_wait() {
        // The engine's pursuit runs nothing under a wait, where the
        // original's low level runs on (see the phase's left-out items).
        assert_a_kept_frame_runs_no_branch(|ship| ship.ai_wait = 50);
    }

    #[test]
    fn a_frame_kept_through_the_regroup_pause_runs_no_branch() {
        // No high level is due at all here, where a wait's is due and skips.
        for kept in [Kept::Coast, Kept::Formation { guard: None }] {
            assert_a_kept_frame_runs_no_branch(|ship| {
                ship.goal = Goal::Regroup { kept };
            });
        }
    }

    #[test]
    fn a_frame_kept_by_a_ship_with_no_ai_runs_no_branch() {
        assert_a_kept_frame_runs_no_branch(|ship| ship.no_ai = true);
    }

    #[test]
    fn an_intact_plunders_approach_runs_it() {
        let (mut traffic, id) = traffic(|ship, _| {
            ship.goal = Goal::Plunder {
                ship: ShipRef::Player,
                hopeless: false,
            };
        });
        think_in(&mut traffic, &world(FAR_PLAYER));
        assert_eq!(bodies(&traffic, id), [Manoeuvre::Approach]);
        let place = of(&traffic, id).slot_place.expect("a place");
        assert_branched(&traffic, id, place, 1, GlowCopy::Swarm(GLOW));
    }

    #[test]
    fn double_times_second_pass_adds_neither() {
        let world = world(FAR_PLAYER);
        let run = |change: fn(&mut Npc)| {
            let (mut traffic, id) = traffic(|ship, _| change(ship));
            traffic.tick_in(&Keep, world, &[], &mut NeverFires);
            let at = of(&traffic, id).state.position;
            traffic.move_in(world, &mut NeverFires);
            (
                of(&traffic, id).state.position - at,
                of(&traffic, id).display.glow,
            )
        };
        let named = run(|_| {});
        let unnamed = run(|ship| ship.swarm_leader = None);
        assert_eq!(named.0, unnamed.0, "the second pass moved them alike");
        assert_eq!(named.1, GLOW + 2, "copied once, then two thrusting frames");
    }

    #[test]
    fn by_the_other_reading_of_the_npc_formation_only_the_players_slots_nudge() {
        let earlier = FlightRules {
            npc_formation: NpcFormation::Earlier,
            ..FlightRules::default()
        };
        let place = Vec2::new(5.0, 6.0);
        let on = |word| Npc {
            form_parent: Some(word),
            slot_place: Some(place),
            ..hull_at(Vec2::ZERO)
        };
        assert_eq!(slot_place(&on(1), earlier), None, "an NPC parent's");
        assert_eq!(slot_place(&on(0), earlier), Some(place), "the player's");
        assert_eq!(slot_place(&on(1), FlightRules::default()), Some(place));
        let parentless = Npc {
            form_parent: None,
            ..on(1)
        };
        assert_eq!(slot_place(&parentless, FlightRules::default()), None);
        // Through the traffic: the slot is worked out, and nothing flies it.
        let (mut traffic, id) = traffic(|_, _| {});
        let by_earlier = World {
            flight: earlier,
            ..world(FAR_PLAYER)
        };
        think_in(&mut traffic, &by_earlier);
        assert!(of(&traffic, id).slot_place.is_some());
        assert_eq!(of(&traffic, id).state.position, START);
        assert_eq!(of(&traffic, id).display.drive.glow, GlowCopy::Swarm(GLOW));
    }
}
