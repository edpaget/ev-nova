//! A ship's parent link follows its berth: once another ship takes the
//! berth of a ship's carrier or lead, the link names the newcomer, as the
//! original's parent slot (+0x9a) does in the `EV Nova` executable.
//!
//! The original links a ship to its carrier or lead by slot. No slot test
//! clears +0x9a, and a spawn rewrites the slot and leaves every other
//! ship's +0x9a (`_GenericRandomShipSpawn` @0x3c8b1-0x3c90b), so once a
//! newcomer takes the parent's berth every reader of +0x9a reads the
//! newcomer:
//!
//! - **the marking pass's check** (`_AICheckParentExists` @0x89276-0x892a9)
//!   finds the slot active and not disabled, so it runs no succession and
//!   the follower stays on the newcomer
//!   ([`succession`](super::succession));
//! - **`_EscortAI`** (@0x838f5-0x83f27) tests the slot for active and not
//!   dying (@0x8392d-0x83942), follows the newcomer's jump (@0x839fd-0x83a92,
//!   its +0x6c copied), reads its order word and formation, holds its
//!   slot as the nav (+0x6c, @0x83d9c, @0x83e98, @0x83ef9) and never
//!   targets it (@0x83f11);
//! - **the high level**: goal 0xb pushes the newcomer onto goal 2
//!   (@0x8de71-0x8defa); goal 4's fleet tests treat it as the parent
//!   (@0x8e084-0x8e374, @0x8e66b-0x8e813); the fighter's turn-back writes
//!   the berth as its nav (@0x8d61a-0x8d621), and goal 5 docks with the
//!   newcomer, its nav rewritten from +0x9a each think (@0x8ebe5-0x8ed0c);
//!   goal 0xa forms on it (@0x8ed26-0x8eeeb);
//! - **the low level**: sub 0xd follows its jump (@0x859df-0x85bde), and
//!   the carrier dock reaches it by its own class's sprite (+0x76 of the
//!   nav's slot, @0x87f0b-0x87fb9) and docks into its bay
//!   (`_AIAddFighterToParent` @0x802b1-0x803e6), or stays out with no bay
//!   for the fighter (@0x80384);
//! - **`_AIDispatch`** cancels a follow of its jump on the newcomer's
//!   fight (@0x9012f-0x9019c).
//!
//! The engine links by [`NpcId`]: [`Npc::leader`], [`Npc::carrier`], a
//! fighter's [`Goal::Dock`], a follower's [`Goal::Follow`] and its
//! [`WarpGoal::Follow`]. So [`Traffic::slot_taken`] calls
//! [`Npc::follow_berth`] on every ship for each old ship of the berth it
//! collects (the gone record, and in the ship loop a wreck displayed
//! earlier in the loop), and each of those names the newcomer. The
//! carrier's reach is the newcomer's sprite, as the dock reads it live;
//! the fighter keeps its capacities, as its AI type stays 5. The
//! formation parent ([`Npc::form_parent`]) and swarm leader
//! ([`Npc::swarm_leader`]) are berths already, and the travel stellar
//! ([`Npc::travel`]) a slot, so they follow the berth with nothing
//! written.
//!
//! **By the other reading** of
//! [`RuleKey::NpcEscortAi`](crate::RuleKey::NpcEscortAi), the port's
//! earlier routing by ship, a link names its ship and nothing follows the
//! berth: a fleet escort whose lead dies keeps its lead, and a fighter
//! whose carrier is gone leaves it (`nova_sim`'s `lost_lead` module). The
//! traffic reads the reading the last think opened with.
//!
//! Nothing here plays a sound or draws a random number.

use super::Traffic;
use super::npc::{Npc, NpcId};
use super::warp_out::WarpGoal;
use crate::ai::Goal;
use crate::combat::ShipRef;
use crate::escort::NO_SPRITE;
use crate::rulebook::RuleSource;

impl Traffic {
    /// Every ship's parent link that names `old`, whose berth `newcomer`
    /// takes, names the newcomer, by the engine's reading of the parent
    /// link (see the module docs).
    pub(super) fn parents_follow_berth(&mut self, old: NpcId, newcomer: &Npc) {
        if self.parent_link != RuleSource::Engine {
            return;
        }
        let reach = newcomer.hull.sprite_size.unwrap_or(NO_SPRITE);
        for npc in &mut self.npcs {
            npc.follow_berth(old, newcomer.id, reach);
        }
    }
}

impl Npc {
    /// Each of its links to `old` as its parent, its carrier or lead, names
    /// `new`, which took `old`'s berth, of sprite width `reach` (see the
    /// module docs): its leader, its carrier (its reach `reach`, its
    /// capacities kept), a dock with it, a follow of it and a follow of its
    /// jump. Any other link is kept.
    pub fn follow_berth(&mut self, old: NpcId, new: NpcId, reach: f32) {
        if self.leader == Some(old) {
            self.leader = Some(new);
        }
        if let Some(carrier) = self
            .carrier
            .as_mut()
            .filter(|c| c.ship == ShipRef::Npc(old))
        {
            carrier.ship = ShipRef::Npc(new);
            carrier.reach = reach;
        }
        match &mut self.goal {
            Goal::Dock(ship) if *ship == ShipRef::Npc(old) => *ship = ShipRef::Npc(new),
            Goal::Follow { lead, .. } if *lead == old => *lead = new,
            _ => {}
        }
        if self.warp.goal == Some(WarpGoal::Follow(ShipRef::Npc(old))) {
            self.warp.goal = Some(WarpGoal::Follow(ShipRef::Npc(new)));
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::bay::Carrier;
    use crate::rulebook::RuleSource;
    use crate::stats::ShipStats;
    use crate::traffic::fixture::npc;
    use crate::traffic::gone::Freed;

    const OLD: NpcId = NpcId(4);
    const NEW: NpcId = NpcId(7);
    const OTHER: NpcId = NpcId(5);

    /// A fighter of `carrier`, led by it, docking with it and following
    /// its jump.
    fn fighter_of(carrier: NpcId) -> Npc {
        let mut fighter = npc(1, ShipStats::default());
        fighter.leader = Some(carrier);
        fighter.take_carrier(
            Carrier {
                ship: ShipRef::Npc(carrier),
                window: 100.0,
                reach: 40.0,
            },
            RuleSource::Engine,
        );
        fighter.goal = Goal::Dock(ShipRef::Npc(carrier));
        fighter.warp.goal = Some(WarpGoal::Follow(ShipRef::Npc(carrier)));
        fighter
    }

    #[test]
    fn every_link_to_the_old_ship_names_the_newcomer() {
        let mut fighter = fighter_of(OLD);
        let shield = fighter.reserves.shield.max;
        fighter.follow_berth(OLD, NEW, 55.0);
        assert_eq!(fighter.leader, Some(NEW));
        let carrier = fighter.carrier.expect("still a fighter");
        assert_eq!(carrier.ship, ShipRef::Npc(NEW));
        assert_eq!(carrier.reach, 55.0, "the newcomer's sprite");
        assert_eq!(carrier.window, 100.0, "its own");
        assert_eq!(fighter.reserves.shield.max, shield, "its capacities kept");
        assert_eq!(fighter.goal, Goal::Dock(ShipRef::Npc(NEW)));
        assert_eq!(fighter.warp.goal, Some(WarpGoal::Follow(ShipRef::Npc(NEW))));
        let mut follower = npc(2, ShipStats::default());
        follower.goal = Goal::Follow {
            lead: OLD,
            guard: Some(ShipRef::Player),
        };
        follower.follow_berth(OLD, NEW, 55.0);
        assert_eq!(
            follower.goal,
            Goal::Follow {
                lead: NEW,
                guard: Some(ShipRef::Player)
            }
        );
    }

    #[test]
    fn a_link_to_another_ship_is_kept() {
        let mut fighter = fighter_of(OTHER);
        let before = fighter.clone();
        fighter.follow_berth(OLD, NEW, 55.0);
        assert_eq!(fighter, before);
        let mut player = fighter_of(OTHER);
        player.carrier = Some(Carrier {
            ship: ShipRef::Player,
            window: 100.0,
            reach: 40.0,
        });
        player.goal = Goal::Dock(ShipRef::Player);
        player.warp.goal = Some(WarpGoal::Follow(ShipRef::Player));
        let before = player.clone();
        player.follow_berth(OLD, NEW, 55.0);
        assert_eq!(player, before, "the player's fighter");
        let mut attacker = npc(3, ShipStats::default());
        attacker.goal = Goal::Attack(ShipRef::Npc(OLD));
        attacker.follow_berth(OLD, NEW, 55.0);
        assert_eq!(attacker.goal, Goal::Attack(ShipRef::Npc(OLD)), "not a link");
    }

    /// Every ship keeps its goal and target.
    #[derive(Debug)]
    struct Keep;

    impl crate::ai::Behaviour for Keep {
        fn decide(
            &self,
            npc: &Npc,
            _: &crate::ai::Surroundings,
            _: &mut dyn crate::chance::Chance,
        ) -> Goal {
            npc.goal
        }
    }

    /// A traffic whose carrier at berth 3 has jumped out, its record
    /// kept, and whose fighter is at berth 5: the traffic, the carrier
    /// and the fighter.
    fn carrier_gone() -> (Traffic, NpcId, NpcId) {
        let mut traffic = Traffic::with_interval(1);
        let carrier = traffic.add_npc_at(npc(0, ShipStats::default()), 3);
        let fighter = traffic.add_npc_at(fighter_of(carrier), 5);
        let record = traffic.npcs()[0].clone();
        traffic.remove(carrier);
        traffic.left_the_system(record, Freed::InDispatch);
        (traffic, carrier, fighter)
    }

    fn carrier_of(traffic: &Traffic, fighter: NpcId) -> Carrier {
        let fighter = traffic.npcs().iter().find(|npc| npc.id == fighter);
        fighter.and_then(|npc| npc.carrier).expect("a fighter")
    }

    /// A ship of sprite width `sprite`, if any.
    fn sprited(sprite: Option<f32>) -> Npc {
        let mut ship = npc(0, ShipStats::default());
        ship.hull.sprite_size = sprite;
        ship
    }

    #[test]
    fn a_ship_taking_the_carriers_berth_is_its_fighters_carrier_reached_by_its_sprite() {
        // The dock reads the nav's slot's class (+0x76) for its sprite
        // width (@0x87f0b-0x87f21).
        for (sprite, reach) in [(Some(72.0), 72.0), (None, NO_SPRITE)] {
            let (mut traffic, _, fighter) = carrier_gone();
            let new = traffic.add_npc_at(sprited(sprite), 3);
            let carrier = carrier_of(&traffic, fighter);
            assert_eq!(carrier.ship, ShipRef::Npc(new), "{sprite:?}");
            assert_eq!(carrier.reach, reach, "{sprite:?}");
        }
    }

    #[test]
    fn a_ship_taking_another_berth_changes_no_link() {
        let (mut traffic, carrier, fighter) = carrier_gone();
        traffic.add_npc_at(sprited(Some(72.0)), 4);
        assert_eq!(carrier_of(&traffic, fighter).ship, ShipRef::Npc(carrier));
    }

    #[test]
    fn by_the_other_reading_a_link_keeps_its_ship() {
        // The port's earlier routing by ship (`RuleKey::NpcEscortAi`).
        let (mut traffic, carrier, fighter) = carrier_gone();
        let world = crate::traffic::World {
            npc_escort_ai: RuleSource::Bible,
            ..crate::traffic::World::new(&[])
        };
        traffic.open_think(&Keep, world, &[], &mut crate::chance::NeverFires);
        traffic.add_npc_at(sprited(Some(72.0)), 3);
        let kept = carrier_of(&traffic, fighter);
        assert_eq!((kept.ship, kept.reach), (ShipRef::Npc(carrier), 40.0));
        let (mut traffic, _, fighter) = carrier_gone();
        let new = traffic.add_npc_at(sprited(Some(72.0)), 3);
        assert_eq!(
            carrier_of(&traffic, fighter).ship,
            ShipRef::Npc(new),
            "by the engine's, before any think"
        );
    }
}
