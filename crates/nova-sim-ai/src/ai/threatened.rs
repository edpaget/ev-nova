//! Whether a ship's friend is threatened by the ship's enemy, as its
//! friend strength doubles such a friend (see [`odds`](crate::ai::odds)).
//!
//! The rules are the original's (`_IsShipThreatenedByMyEnemy` @0x824f6
//! and `_IsPlayerThreatenedByMyEnemy` @0x82483 in the `EV Nova`
//! executable). "My enemy" is `_IsShipMyEnemy` @0x8196c, two ships of two
//! governments at war ([`Governments::npc_enemies`](crate::govt::Governments::npc_enemies)),
//! so a ship of no government, the player's included, has none.
//!
//! - **The player as the friend** ([`player_threatened_by_enemy_of`]):
//!   some other NPC that threatens the player
//!   ([`Npc::threatens_player_extended`], `_ExtendedIsThreatToPlayer`
//!   @0x824b8, seen as the player sees it) is the asker's enemy
//!   (@0x824cf).
//! - **An NPC as the friend** ([`threatened_by_enemy_of`]): it threatens
//!   the player itself (`_ExtendedIsThreatToPlayer(friend)` @0x82525,
//!   whatever the player is to the asker); or some NPC k is the asker's
//!   enemy (@0x82581) and `_ExtendedIsThreatToShip(k, friend)` holds
//!   (@0x8256a), k being neither the friend nor the asker (@0x8254b-0x82561;
//!   the asker can't change the answer, as no ship is its own government's
//!   enemy). That test
//!   is true when its second ship attacks the first or a ship the first
//!   leads (@0x81aa9-0x81b4d), so it asks whether **the friend attacks k
//!   or a ship k leads** ([`Npc::threatens_npc_among`]), the reverse of
//!   what the name says: a friend under attack by the asker's enemy isn't
//!   threatened by it.

use crate::ai::Surroundings;
use crate::combat::ShipRef;
use crate::traffic::npc::Npc;

/// Whether the player, as `asker`'s friend, is threatened by `asker`'s
/// enemy among `around` (`_IsPlayerThreatenedByMyEnemy`, see the module
/// docs).
#[must_use]
pub fn player_threatened_by_enemy_of(asker: &Npc, around: &Surroundings) -> bool {
    around.npcs.iter().any(|threat| {
        threat.id != asker.id
            && threat.threatens_player_extended(around.npcs, around.seen_by_player(threat))
            && around.govts.npc_enemies(asker.govt, threat.govt)
    })
}

/// Whether `friend`, a friend of `asker`, is threatened by `asker`'s enemy
/// among `around` (`_IsShipThreatenedByMyEnemy`, see the module docs).
#[must_use]
pub fn threatened_by_enemy_of(friend: ShipRef, asker: ShipRef, around: &Surroundings) -> bool {
    let asker_npc = match asker {
        ShipRef::Npc(id) => around.npc(id),
        ShipRef::Player => None,
    };
    let ShipRef::Npc(id) = friend else {
        return asker_npc.is_some_and(|asker| player_threatened_by_enemy_of(asker, around));
    };
    let Some(friend) = around.npc(id) else {
        return false;
    };
    if friend.threatens_player_extended(around.npcs, around.seen_by_player(friend)) {
        return true;
    }
    let Some(asker) = asker_npc else {
        return false;
    };
    around.npcs.iter().any(|enemy| {
        enemy.id != friend.id
            && around.govts.npc_enemies(asker.govt, enemy.govt)
            && friend.threatens_npc_among(enemy.id, around.npcs)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::Goal;
    use crate::ai::fixture::{NEUTRAL, PIRATES, POLICE, TRADERS, around, govts, n, ship, sites};
    use crate::cloak::{Cloak, FADE_FULL};
    use crate::traffic::npc::{AiType, NpcId};

    const P: ShipRef = ShipRef::Player;

    /// A trader (NPC 1, the asker) and its friend, police NPC 2.
    fn trader() -> Npc {
        ship(1, TRADERS, AiType::Warship, 0.0, 0.0)
    }

    fn friend() -> Npc {
        ship(2, POLICE, AiType::Warship, 100.0, 0.0)
    }

    /// Whether `npcs[1]` is threatened by `npcs[0]`'s enemy.
    fn threatened(npcs: &[Npc]) -> bool {
        let govts = govts(0, 0);
        let sites = sites();
        let around = around(&sites, npcs, &govts, (0.0, -500.0), 0);
        threatened_by_enemy_of(ShipRef::Npc(npcs[1].id), ShipRef::Npc(npcs[0].id), &around)
    }

    #[test]
    fn a_friend_attacking_the_askers_enemy_or_a_ship_it_leads_is_threatened() {
        let mut friend = friend();
        friend.goal = Goal::Attack(n(3));
        let pirate = ship(3, PIRATES, AiType::Warship, 300.0, 0.0);
        assert!(threatened(&[trader(), friend.clone(), pirate.clone()]));
        let mut escort = ship(4, NEUTRAL, AiType::Warship, 400.0, 0.0);
        escort.leader = Some(NpcId(3));
        friend.goal = Goal::Attack(n(4));
        assert!(
            threatened(&[trader(), friend.clone(), pirate, escort.clone()]),
            "a ship the enemy leads"
        );
        let neutral = ship(3, NEUTRAL, AiType::Warship, 300.0, 0.0);
        assert!(
            !threatened(&[trader(), friend, neutral, escort]),
            "a ship a non-enemy leads"
        );
    }

    #[test]
    fn a_friend_attacked_by_the_askers_enemy_is_not_threatened() {
        let mut pirate = ship(3, PIRATES, AiType::Warship, 300.0, 0.0);
        pirate.goal = Goal::Attack(n(2));
        assert!(!threatened(&[trader(), friend(), pirate]));
    }

    #[test]
    fn a_friend_fleeing_the_askers_enemy_is_threatened() {
        let mut friend = friend();
        friend.goal = Goal::Flee(n(3));
        let pirate = ship(3, PIRATES, AiType::Warship, 300.0, 0.0);
        assert!(threatened(&[trader(), friend, pirate]));
    }

    #[test]
    fn a_friend_threatening_the_player_is_threatened_whatever_the_player_is_to_the_asker() {
        let mut friend = friend();
        friend.goal = Goal::Attack(P);
        assert!(threatened(&[trader(), friend.clone()]));
        friend.goal = Goal::Idle;
        assert!(!threatened(&[trader(), friend.clone()]));
        friend.goal = Goal::Attack(P);
        friend.cloak = Cloak {
            fade: FADE_FULL,
            direction: 0,
        };
        assert!(
            !threatened(&[trader(), friend]),
            "unseen by the player, it is no threat to it"
        );
    }

    #[test]
    fn an_independent_asker_or_enemy_threatens_nothing() {
        let mut friend = friend();
        friend.goal = Goal::Attack(n(3));
        let mut independent = ship(3, PIRATES, AiType::Warship, 300.0, 0.0);
        independent.govt = None;
        assert!(!threatened(&[trader(), friend.clone(), independent]));
        let pirate = ship(3, PIRATES, AiType::Warship, 300.0, 0.0);
        let mut asker = trader();
        asker.govt = None;
        assert!(!threatened(&[asker, friend, pirate]));
    }

    #[test]
    fn a_friend_is_never_its_own_enemy_threat() {
        // Government 144 is both allied with and at war with the traders,
        // and the friend of it attacks its own escort.
        let govts = crate::govt::Governments::new([
            crate::catalog::GovtRecord {
                classes: [1, -1, -1, -1],
                ..nova_sim_catalog::fixture::govt(140)
            },
            crate::catalog::GovtRecord {
                allies: [1, -1, -1, -1],
                enemies: [1, -1, -1, -1],
                ..nova_sim_catalog::fixture::govt(144)
            },
        ]);
        let mut friend = ship(2, crate::catalog::GovtId(144), AiType::Warship, 100.0, 0.0);
        friend.goal = Goal::Attack(n(3));
        let mut escort = ship(3, NEUTRAL, AiType::Warship, 200.0, 0.0);
        escort.leader = Some(NpcId(2));
        let npcs = [trader(), friend, escort];
        let sites = sites();
        let around = around(&sites, &npcs, &govts, (0.0, -500.0), 0);
        assert!(!threatened_by_enemy_of(n(2), n(1), &around));
    }

    /// Whether the player, a friend of NPC 1 (`npcs[0]`), is threatened.
    fn player_threatened(npcs: &[Npc]) -> bool {
        let govts = govts(0, 0);
        let sites = sites();
        let around = around(&sites, npcs, &govts, (0.0, -500.0), 0);
        threatened_by_enemy_of(P, ShipRef::Npc(npcs[0].id), &around)
    }

    #[test]
    fn the_player_is_threatened_by_an_enemy_of_the_asker_threatening_it_and_no_other() {
        let mut pirate = ship(3, PIRATES, AiType::Warship, 300.0, 0.0);
        pirate.goal = Goal::Attack(P);
        assert!(player_threatened(&[trader(), pirate.clone()]));
        let mut neutral = pirate.clone();
        neutral.govt = Some(NEUTRAL);
        assert!(!player_threatened(&[trader(), neutral]), "no enemy");
        let mut calm = pirate;
        calm.goal = Goal::Idle;
        assert!(!player_threatened(&[trader(), calm]), "no threat");
        let mut asker = trader();
        asker.goal = Goal::Attack(P);
        assert!(!player_threatened(&[asker]), "not the asker itself");
    }

    #[test]
    fn the_player_asking_has_no_enemy() {
        let mut friend = friend();
        friend.goal = Goal::Attack(n(3));
        let pirate = ship(3, PIRATES, AiType::Warship, 300.0, 0.0);
        let npcs = [friend, pirate];
        let govts = govts(0, 0);
        let sites = sites();
        let around = around(&sites, &npcs, &govts, (0.0, -500.0), 0);
        assert!(!threatened_by_enemy_of(n(2), P, &around));
        assert!(!threatened_by_enemy_of(n(9), n(3), &around), "not there");
    }
}
