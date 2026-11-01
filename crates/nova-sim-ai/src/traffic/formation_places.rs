//! The places of the NPC formations: the word +0x28/+0x2c of each
//! ship-table slot, which `_AICalcShipFormPos` writes (@0x84e64-0x84ef7)
//! and `_AIMaintainFormation` alone reads (@0x84fc2-0x85034).
//!
//! **The word** is the slot's, not the ship's: `_AISetup` and every spawn,
//! launch and capture leave it alone, so a new ship starts with its last
//! ship's, and a never-used slot reads 0. Nothing saves it. It is kept
//! twice, once for each reader:
//!
//! - [`BerthPlaces`], on the traffic, is the table's word as the frame's
//!   dispatches have left it. It lasts as the traffic's other berth words
//!   do, and is handed on ([`kept_berths`](super::kept_berths)). There is
//!   no rule reading: a ship reads the word, whichever ship wrote it.
//! - [`Npc::slot_place`](super::npc::Npc::slot_place) is the word as the
//!   ship's low level reads it this frame, as the fly half hands a flight
//!   only its ship.
//!
//! **The pass** ([`Traffic::write_places`]) stands for the writes of one
//! frame's `_AIDispatch` loop (`_DoPlayGameWork` @0x44270-0x442ae), slot by
//! slot, before any think:
//!
//! 1. Each ship takes its berth's word as the frame began.
//! 2. The player's followers' words are written (`_HandlePlayer`
//!    @0x6f1cb-0x6f1df, before every dispatch).
//! 3. Each parent whose dispatch gets there
//!    ([`reaches_formation_calc`](crate::ai::dispatch_gate::reaches_formation_calc),
//!    @0x8fcaf-0x8fcc3) writes its followers' words from where it stands
//!    ([`fleet_formation::written`]). A follower in a higher berth reads
//!    the word this frame; one in a lower berth has run its low level
//!    before its parent's dispatch, and reads it next frame.
//!
//! So a follower of a pod or a ship with no AI keeps the place last
//! written, at the arrival snap or by an earlier parent, however far the
//! parent moves. A parent writes before its own think, so before a drift
//! of its own (@0x8eb6b-0x8ebd4). With its parent gone the word stands, to
//! be read again if the ship finds a new parent above it. The pass reads
//! the arrivals' snaps ([`fleet_formation::snap`]), which it runs after;
//! `_HyperSpawnFleet` writes the word before any dispatch as well.
//!
//! **Between frames.** `_DoEscortDialog`'s stale release dispatches the
//! released ship itself (`_AIDispatch(ship, 1)` @0x95664, after
//! `_AIMakeShipLeave` @0x95652), which rewrites its followers' words from
//! where it stands, with no AI-type gate ([`Traffic::dispatch_places`]).
//!
//! `_RespawnEscort`'s 0 (@0x3d589-0x3d58f) and the player's snaps are not
//! built: the player's write overwrites them before any reader.
//!
//! None of these routines plays a sound or draws a random number.

use super::Traffic;
use super::berth_kept::BerthKept;
use super::fleet_formation::{self, Written};
use super::npc::NpcId;
use crate::ai::swarm::{self, FormationParent};
use crate::flight::ShipState;
use crate::geometry::Vec2;

/// The word +0x28/+0x2c of each berth (see the module docs).
pub type BerthPlaces = BerthKept<Vec2>;

impl Traffic {
    /// One frame's writes of the places (see the module docs), the player
    /// at `player` if it is here.
    pub(crate) fn write_places(&mut self, player: Option<&ShipState>) {
        for npc in &mut self.npcs {
            if let Some(word) = self.berth_places.left(npc.berth) {
                npc.slot_place = Some(word);
            }
        }
        if let Some(player) = player {
            self.write_players_places(player);
        }
        for written in fleet_formation::written(&self.npcs, None) {
            let Written {
                follower,
                berth,
                place,
                late,
            } = written;
            self.berth_places.write(berth, Some(place));
            if !late && let Some(npc) = self.npcs.iter_mut().find(|npc| npc.id == follower) {
                npc.slot_place = Some(place);
            }
        }
    }

    /// `_HandlePlayer`'s write: the word of each ship whose formation
    /// parent is the player at `player`, from its place in the player's
    /// formation.
    fn write_players_places(&mut self, player: &ShipState) {
        for npc in &mut self.npcs {
            if swarm::formation_parent(npc) == Some(FormationParent::Player)
                && let Some(slot) = npc.formation()
            {
                let place = slot.position(player);
                self.berth_places.write(npc.berth, Some(place));
                npc.slot_place = Some(place);
            }
        }
    }

    /// The writes of `parent`'s dispatch between frames (see the module
    /// docs): only the berths' words, which its followers read at the next
    /// pass, and none for a pod.
    pub fn dispatch_places(&mut self, parent: NpcId) {
        for written in fleet_formation::written(&self.npcs, Some(parent)) {
            self.berth_places.write(written.berth, Some(written.place));
        }
    }

    /// The word `berth` holds, none for a berth never written.
    #[must_use]
    pub fn berth_place(&self, berth: u32) -> Option<Vec2> {
        self.berth_places.left(berth)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::{Behaviour, Goal, Surroundings};
    use crate::chance::Chance;
    use crate::combat::hull::{Condition, ESCAPE_POD_SHIP};
    use crate::escort::{EscortDuty, FormationSlot};
    use crate::stats::ShipStats;
    use crate::traffic::World;
    use crate::traffic::fixture::assisting::world_with;
    use crate::traffic::fixture::npc;
    use crate::traffic::npc::{Mode, Npc};
    use nova_sim_base::fixture::{Draws, FAST};

    /// Every NPC keeps its goal.
    #[derive(Debug)]
    struct Keep;

    impl Behaviour for Keep {
        fn decide(&self, npc: &Npc, _: &Surroundings, _: &mut dyn Chance) -> Goal {
            npc.goal
        }
    }

    const LEAD_BERTH: u32 = 5;
    const BELOW: u32 = 3;
    const ABOVE: u32 = 7;

    /// A [`FAST`] ship holding station, at rest at (`x`, -200).
    fn lead_at(x: f32) -> Npc {
        let mut lead = npc(0, ShipStats::new(FAST, &[]));
        lead.goal = Goal::Hold;
        lead.state.position = Vec2::new(x, -200.0);
        lead
    }

    /// A [`FAST`] follower of `lead`, keeping with it.
    fn follower(lead: NpcId) -> Npc {
        let mut ship = npc(0, ShipStats::new(FAST, &[]));
        ship.leader = Some(lead);
        ship.goal = Goal::Follow { lead, guard: None };
        ship
    }

    /// A traffic of a lead at [`LEAD_BERTH`] and a follower at each of
    /// `berths`: the lead, then the followers.
    fn fleet(berths: &[u32]) -> (Traffic, NpcId, Vec<NpcId>) {
        let mut traffic = Traffic::with_interval(1);
        let lead = traffic.add_npc_at(lead_at(100.0), LEAD_BERTH);
        let followers = berths
            .iter()
            .map(|&berth| traffic.add_npc_at(follower(lead), berth))
            .collect();
        (traffic, lead, followers)
    }

    fn of(traffic: &Traffic, id: NpcId) -> &Npc {
        traffic
            .npcs()
            .iter()
            .find(|npc| npc.id == id)
            .expect("there")
    }

    fn of_mut(traffic: &mut Traffic, id: NpcId) -> &mut Npc {
        traffic
            .npcs_mut()
            .iter_mut()
            .find(|npc| npc.id == id)
            .expect("there")
    }

    /// `traffic` thought a tick, in an empty world.
    fn think(traffic: &mut Traffic) {
        traffic.think_holding(&Keep, World::new(&[]), &[], &mut Draws::of(&[]));
    }

    /// `traffic` ticked, in an empty world.
    fn tick(traffic: &mut Traffic) {
        traffic.tick_in(&Keep, World::new(&[]), &[], &mut Draws::of(&[]));
    }

    /// Moves `lead` `by` along x: where it now stands.
    fn shift(traffic: &mut Traffic, lead: NpcId, by: f32) -> ShipState {
        let lead = of_mut(traffic, lead);
        lead.state.position.x += by;
        lead.state
    }

    /// The place `id`'s slot makes of a parent at `parent`.
    fn place_of(traffic: &Traffic, id: NpcId, parent: &ShipState) -> Vec2 {
        of(traffic, id).fleet_slot.expect("a slot").position(parent)
    }

    #[test]
    fn a_follower_above_its_parent_reads_this_frames_place_and_one_below_reads_the_frame_befores() {
        let (mut traffic, lead, followers) = fleet(&[BELOW, ABOVE]);
        let (below, above) = (followers[0], followers[1]);
        let first = of(&traffic, lead).state;
        think(&mut traffic);
        assert_eq!(of(&traffic, below).slot_place, None, "no word yet");
        let on_first = |traffic: &Traffic, id| Some(place_of(traffic, id, &first));
        assert_eq!(of(&traffic, above).slot_place, on_first(&traffic, above));
        assert_eq!(
            traffic.berth_place(BELOW),
            on_first(&traffic, below),
            "the word is written all the same"
        );
        let second = shift(&mut traffic, lead, 60.0);
        think(&mut traffic);
        assert_eq!(of(&traffic, below).slot_place, on_first(&traffic, below));
        assert_eq!(
            of(&traffic, above).slot_place,
            Some(place_of(&traffic, above, &second))
        );
        assert_eq!(
            traffic.berth_place(BELOW),
            Some(place_of(&traffic, below, &second))
        );
    }

    /// A matched follower at `berth` of a lead that has just moved 100
    /// along x, halfway between its two places: where a tick leaves it,
    /// off the place of the lead as it stood.
    fn nudged_after_a_jump_of_the_lead(berth: u32) -> f32 {
        let (mut traffic, lead, followers) = fleet(&[berth]);
        let follower = followers[0];
        traffic.form_fleets(None);
        let old = of(&traffic, lead).state;
        let was = place_of(&traffic, follower, &old);
        shift(&mut traffic, lead, 100.0);
        of_mut(&mut traffic, follower).state.position = was + Vec2::new(50.0, 0.0);
        tick(&mut traffic);
        of(&traffic, follower).state.position.x - was.x
    }

    /// The nudge of a [`FAST`] ship (see
    /// [`maintain`](crate::traffic::formation_flight::maintain)).
    const NUDGE: f32 = 1.8;

    fn assert_near(a: f32, b: f32) {
        assert!((a - b).abs() < 1e-4, "{a} is not {b}");
    }

    #[test]
    fn a_follower_below_its_parent_is_nudged_toward_the_frame_befores_place() {
        assert_near(nudged_after_a_jump_of_the_lead(BELOW), 50.0 - NUDGE);
    }

    #[test]
    fn a_follower_above_its_parent_is_nudged_toward_this_frames_place() {
        assert_near(nudged_after_a_jump_of_the_lead(ABOVE), 50.0 + NUDGE);
    }

    /// The followers of a parent whose dispatch `stops` stops keep the word
    /// `written` puts down, which also applies `stops` where its writer
    /// needs it, through several ticks of its moving, and are nudged toward
    /// it.
    fn keeps_the_place(
        stops: impl Fn(&mut Npc),
        written: impl Fn(&mut Traffic, NpcId, &dyn Fn(&mut Npc)),
    ) {
        let (mut traffic, lead, followers) = fleet(&[ABOVE]);
        let follower = followers[0];
        let start = of(&traffic, lead).state;
        written(&mut traffic, lead, &stops);
        let slot = of(&traffic, follower).fleet_slot;
        assert!(slot.is_some(), "the scene");
        let word = place_of(&traffic, follower, &start);
        assert_eq!(of(&traffic, follower).slot_place, Some(word), "the scene");
        for _ in 0..5 {
            shift(&mut traffic, lead, 40.0);
            think(&mut traffic);
            let there = of(&traffic, follower);
            assert_eq!(there.slot_place, Some(word));
            assert_eq!(there.fleet_slot, slot, "still numbered");
        }
        assert_eq!(traffic.berth_place(ABOVE), Some(word));
        // Near its parent, and off the place it still keeps.
        of_mut(&mut traffic, lead).state.position = word + Vec2::new(0.0, 100.0);
        of_mut(&mut traffic, follower).state.position = word + Vec2::new(20.0, 0.0);
        tick(&mut traffic);
        let nudged = of(&traffic, follower).state.position.x - word.x;
        assert_near(nudged, 20.0 - NUDGE);
    }

    /// The arrival snap of an already stopped parent writes the followers'
    /// words: it numbers the slots itself, and is the only writer.
    fn snapped(traffic: &mut Traffic, lead: NpcId, stops: &dyn Fn(&mut Npc)) {
        stops(of_mut(traffic, lead));
        fleet_formation::snap(&mut traffic.npcs, lead, &mut traffic.berth_places);
    }

    /// A dispatch of the parent writes them, before it stops.
    fn dispatched(traffic: &mut Traffic, lead: NpcId, stops: &dyn Fn(&mut Npc)) {
        traffic.form_fleets(None);
        stops(of_mut(traffic, lead));
    }

    fn no_ai(parent: &mut Npc) {
        parent.no_ai = true;
    }

    fn pod(parent: &mut Npc) {
        parent.ship = ESCAPE_POD_SHIP;
        parent.mode = Mode::Adrift { ticks_left: 100 };
    }

    #[test]
    fn the_followers_of_a_parent_with_no_ai_keep_the_places_last_written() {
        keeps_the_place(no_ai, snapped);
        keeps_the_place(no_ai, dispatched);
    }

    #[test]
    fn the_followers_of_a_pod_parent_keep_the_places_last_written() {
        keeps_the_place(pod, snapped);
        keeps_the_place(pod, dispatched);
    }

    #[test]
    fn a_disabled_parent_still_writes_its_followers_places() {
        let (mut traffic, lead, followers) = fleet(&[ABOVE]);
        for condition in [
            Condition::Disabled,
            Condition::Dying {
                ticks_left: 1,
                starting: false,
            },
        ] {
            of_mut(&mut traffic, lead).condition = condition;
            let state = shift(&mut traffic, lead, 40.0);
            traffic.form_fleets(None);
            assert_eq!(
                of(&traffic, followers[0]).slot_place,
                Some(place_of(&traffic, followers[0], &state)),
                "{condition:?}"
            );
        }
    }

    #[test]
    fn the_places_are_where_a_parent_stood_before_its_own_think() {
        let (mut traffic, lead, followers) = fleet(&[ABOVE]);
        let there = of_mut(&mut traffic, lead);
        there.goal = Goal::Drift;
        there.ai_wait = 10;
        let start = there.state;
        think(&mut traffic);
        let lead = of(&traffic, lead);
        assert_ne!(lead.state, start, "the scene: it drifted");
        let follower = followers[0];
        assert_eq!(
            of(&traffic, follower).slot_place,
            Some(place_of(&traffic, follower, &start))
        );
    }

    #[test]
    fn a_new_ship_below_its_parent_reads_what_its_berth_last_held() {
        let (mut traffic, lead, followers) = fleet(&[BELOW]);
        traffic.form_fleets(None);
        let word = traffic.berth_place(BELOW);
        assert!(word.is_some(), "the scene");
        traffic.npcs.retain(|npc| npc.id != followers[0]);
        let new = traffic.add_npc_at(follower(lead), BELOW);
        assert_eq!(of(&traffic, new).slot_place, None, "until the pass");
        shift(&mut traffic, lead, 60.0);
        traffic.form_fleets(None);
        assert_eq!(of(&traffic, new).slot_place, word);
    }

    #[test]
    fn a_never_used_berth_reads_zero() {
        let (mut traffic, _, followers) = fleet(&[BELOW]);
        traffic.form_fleets(None);
        let ship = of(&traffic, followers[0]);
        assert_eq!(ship.slot_place, None);
        assert_eq!(ship.parent_word(), Some(Vec2::ZERO));
    }

    /// The player's formation place of [`hired`].
    const HIRED: FormationSlot = FormationSlot {
        slot: 2,
        ships: 3,
        spacing: 30.0,
    };

    /// A [`FAST`] escort of the player, at [`HIRED`].
    fn hired() -> Npc {
        let mut ship = npc(0, ShipStats::new(FAST, &[]));
        ship.escort = Some(EscortDuty {
            formation: HIRED,
            order: None,
        });
        ship
    }

    #[test]
    fn a_think_writes_the_players_followers_places_from_the_player_in_its_world() {
        let mut traffic = Traffic::with_interval(1);
        let hired = traffic.add_npc_at(hired(), BELOW);
        let world = world_with(Condition::Intact);
        let player = world.player.expect("the player").state;
        let on_player = Some(HIRED.position(&player));
        traffic.think_holding(&Keep, world, &[], &mut Draws::of(&[]));
        assert_eq!(of(&traffic, hired).slot_place, on_player);
        assert_eq!(traffic.berth_place(BELOW), on_player);
    }

    #[test]
    fn a_ship_of_the_player_reads_the_players_place_and_keeps_it_when_a_parent_above_it_takes_it() {
        let mut traffic = Traffic::with_interval(1);
        let lead = traffic.add_npc_at(lead_at(100.0), LEAD_BERTH);
        let hired = traffic.add_npc_at(hired(), BELOW);
        let world = world_with(Condition::Intact);
        let player = world.player.expect("the player").state;
        let on_player = Some(HIRED.position(&player));
        traffic.form_fleets(Some(&player));
        assert_eq!(of(&traffic, hired).slot_place, on_player);
        assert_eq!(traffic.berth_place(BELOW), on_player);
        // A pirate takes it, and it joins the fleet above it.
        let ship = of_mut(&mut traffic, hired);
        ship.escort = None;
        ship.leader = Some(lead);
        traffic.form_fleets(Some(&player));
        assert_eq!(
            of(&traffic, hired).slot_place,
            on_player,
            "the player's word stands for its first frame"
        );
        let state = of(&traffic, lead).state;
        let next = Some(place_of(&traffic, hired, &state));
        assert_eq!(traffic.berth_place(BELOW), next);
        traffic.form_fleets(Some(&player));
        assert_eq!(of(&traffic, hired).slot_place, next);
    }

    #[test]
    fn with_the_player_gone_its_followers_keep_their_words() {
        let mut traffic = Traffic::with_interval(1);
        let hired = traffic.add_npc_at(hired(), BELOW);
        let world = world_with(Condition::Intact);
        let player = world.player.expect("the player").state;
        let on_player = Some(HIRED.position(&player));
        traffic.form_fleets(Some(&player));
        assert_eq!(of(&traffic, hired).slot_place, on_player, "the scene");
        traffic.form_fleets(None);
        assert_eq!(of(&traffic, hired).slot_place, on_player);
        assert_eq!(traffic.berth_place(BELOW), on_player);
    }

    #[test]
    fn only_the_players_followers_take_the_players_place() {
        let (mut traffic, _, followers) = fleet(&[ABOVE]);
        let mut hired = npc(0, ShipStats::new(FAST, &[]));
        hired.leader = None;
        let loner = traffic.add_npc_at(hired, BELOW);
        let world = world_with(Condition::Intact);
        let player = world.player.expect("the player").state;
        traffic.form_fleets(Some(&player));
        assert_eq!(of(&traffic, loner).slot_place, None, "no parent");
        let lead_state = traffic.npcs()[0].state;
        assert_eq!(
            of(&traffic, followers[0]).slot_place,
            Some(place_of(&traffic, followers[0], &lead_state)),
            "its parent's"
        );
    }

    #[test]
    fn a_parent_gone_leaves_the_word_and_a_later_parent_above_it_is_read_a_frame_late() {
        let (mut traffic, lead, followers) = fleet(&[BELOW]);
        let follower = followers[0];
        let later = traffic.add_npc_at(lead_at(900.0), ABOVE);
        traffic.form_fleets(None);
        traffic.form_fleets(None);
        let word = of(&traffic, follower).slot_place;
        assert!(word.is_some(), "the scene");
        traffic.npcs.retain(|npc| npc.id != lead);
        traffic.form_fleets(None);
        let left = of(&traffic, follower);
        assert_eq!(left.fleet_slot, None, "no parent");
        assert_eq!(left.slot_place, word);
        assert_eq!(traffic.berth_place(BELOW), word);
        of_mut(&mut traffic, follower).leader = Some(later);
        traffic.form_fleets(None);
        assert_eq!(of(&traffic, follower).slot_place, word, "a frame late");
        traffic.form_fleets(None);
        let state = of(&traffic, later).state;
        assert_eq!(
            of(&traffic, follower).slot_place,
            Some(place_of(&traffic, follower, &state))
        );
    }

    #[test]
    fn a_follower_gone_leaves_the_late_word_to_the_next_ship_there() {
        let (mut traffic, lead, followers) = fleet(&[BELOW]);
        let state = of(&traffic, lead).state;
        traffic.form_fleets(None);
        let word = Some(place_of(&traffic, followers[0], &state));
        traffic.npcs.retain(|npc| npc.id != followers[0]);
        traffic.form_fleets(None);
        assert_eq!(traffic.berth_place(BELOW), word);
    }

    /// A matched [`FAST`] ship far enough off its slot to be nudged by the
    /// advance pass, near a leader moving (1, 0), facing 3 degrees off the
    /// leader's 30: the second pass moves it and steps its heading, and
    /// nudges it not at all.
    fn second_pass_moves(mut traffic: Traffic, id: NpcId, world: &World) {
        let ship = of_mut(&mut traffic, id);
        ship.state = ShipState {
            position: Vec2::new(20.0, 0.0),
            velocity: Vec2::new(1.0, 0.0),
            heading: 27.0,
        };
        traffic.move_in(*world, &mut Draws::of(&[]));
        let ship = of(&traffic, id);
        assert_eq!(ship.state.heading, 28.0, "still turning");
        assert_eq!(ship.state.velocity, Vec2::new(1.0, 0.0));
        assert_eq!(ship.state.position, Vec2::new(21.0, 0.0), "no nudge");
    }

    /// Moving (1, 0) at 30 degrees.
    const LEADING: ShipState = ShipState {
        position: Vec2::new(0.0, 100.0),
        velocity: Vec2::new(1.0, 0.0),
        heading: 30.0,
    };

    #[test]
    fn the_second_pass_nudges_no_fleet_escort() {
        let (mut traffic, lead, followers) = fleet(&[ABOVE]);
        of_mut(&mut traffic, lead).state = LEADING;
        traffic.form_fleets(None);
        of_mut(&mut traffic, followers[0]).slot_place = Some(Vec2::ZERO);
        second_pass_moves(traffic, followers[0], &World::new(&[]));
    }

    #[test]
    fn the_second_pass_nudges_no_escort_of_the_player() {
        let mut traffic = Traffic::with_interval(1);
        let mut hired = npc(0, ShipStats::new(FAST, &[]));
        hired.goal = Goal::Formation { guard: None };
        hired.escort = Some(EscortDuty {
            formation: FormationSlot {
                slot: 2,
                ships: 2,
                spacing: 30.0,
            },
            order: None,
        });
        let hired = traffic.add_npc(hired);
        let side = world_with(Condition::Intact).player.expect("the player");
        let world = World {
            player: Some(crate::ai::PlayerSide {
                state: LEADING,
                ..side
            }),
            ..World::new(&[])
        };
        second_pass_moves(traffic, hired, &world);
    }

    #[test]
    fn dispatch_places_writes_the_word_for_the_next_frame() {
        let (mut traffic, lead, followers) = fleet(&[BELOW]);
        let follower = followers[0];
        traffic.form_fleets(None);
        traffic.form_fleets(None);
        let frame_before = of(&traffic, follower).slot_place;
        let released = shift(&mut traffic, lead, 60.0);
        traffic.dispatch_places(lead);
        let word = Some(place_of(&traffic, follower, &released));
        assert_eq!(traffic.berth_place(BELOW), word);
        assert_eq!(of(&traffic, follower).slot_place, frame_before, "not yet");
        shift(&mut traffic, lead, 60.0);
        traffic.form_fleets(None);
        assert_eq!(of(&traffic, follower).slot_place, word);
    }

    #[test]
    fn dispatch_places_is_no_write_for_a_pod_and_needs_no_ai_type() {
        let (mut traffic, lead, _) = fleet(&[BELOW]);
        of_mut(&mut traffic, lead).no_ai = true;
        traffic.dispatch_places(lead);
        assert!(traffic.berth_place(BELOW).is_some(), "no AI-type gate");
        let (mut traffic, lead, _) = fleet(&[BELOW]);
        of_mut(&mut traffic, lead).ship = ESCAPE_POD_SHIP;
        traffic.dispatch_places(lead);
        assert_eq!(traffic.berth_place(BELOW), None);
    }

    #[test]
    fn dispatch_places_writes_only_that_parents_followers() {
        let (mut traffic, lead, _) = fleet(&[BELOW]);
        let other = traffic.add_npc_at(lead_at(900.0), 9);
        traffic.add_npc_at(follower(other), 10);
        traffic.dispatch_places(lead);
        assert_eq!(traffic.berth_place(10), None);
    }
}
