//! An NPC's sight of another ship: `_ShipVisibleToShip(ship, observer)`
//! (@0x971c-0x9809 in the `EV Nova` executable) with an NPC observer
//! ([`Surroundings::sees`]).
//!
//! The rule is [`cloak::visible`](crate::cloak::visible)'s: a ship coming
//! out of a gate is seen by nobody (@0x972d); an uncloaked one is seen;
//! a cloaked one is seen by an escort of it that has a cloak itself
//! (@0x9788-0x97a2), or by a ship with a cloak scanner on screen within
//! 200 of it on both axes (@0x97a4-0x97f8). The player can cloak, but
//! [`Surroundings::sees`] reads it as seen until
//! rdm:phase/cloaking-fidelity/phase-9-npc-sight-of-cloaked-player;
//! [`sees_player`] is the rule for it, which only the player's escorts'
//! parent rule asks so far. `_ShipVisibleToShip` tests no presence, so a ship not
//! in the system reads as seen: the gates that drop a gone target
//! (`target::dropped`, [`Surroundings::live`]) are not a search's.
//!
//! Its readers: an NPC's fire ([`fire`](crate::ai::fire), whose sight of
//! its target holds its fire), the lost-sight search
//! ([`lost_sight`](crate::ai::lost_sight)) and the player's escort's pick
//! of a nearby threat ([`escort::threat_score`](crate::escort::threat_score),
//! `_IsShipANearbyThreatToParent` @0x8370e), the target picks'
//! candidates ([`target`](crate::ai::target), `_SelectWarshipTarget`
//! @0x8a0d2, @0x8a538, @0x8aa7b, @0x8ad92) and the interceptor's pick of
//! a ship to inspect ([`interceptor`](crate::ai::interceptor),
//! `_InterceptorAI` @0x8cd5f, @0x8ce13).
//!
//! # Not modelled
//!
//! The observer +0xc8d0 == 0x3ff branch (@0x973a): the shareware nag
//! ship sees everything. The engine has no nag ship.

use crate::ai::{PlayerSide, Surroundings};
use crate::cloak::{Observer, visible};
use crate::combat::ShipRef;
use crate::traffic::npc::Npc;

impl Surroundings<'_> {
    /// Whether `observer` sees `ship`: `_ShipVisibleToShip(ship,
    /// observer)` (@0x971c; see the module docs).
    #[must_use]
    pub fn sees(&self, observer: &Npc, ship: ShipRef) -> bool {
        let ShipRef::Npc(id) = ship else {
            return true;
        };
        self.npc(id).is_none_or(|other| npc_sees(observer, other))
    }
}

/// Whether `observer` sees NPC `other`: `_ShipVisibleToShip(other,
/// observer)` for two NPCs (@0x971c; see the module docs).
#[must_use]
pub fn npc_sees(observer: &Npc, other: &Npc) -> bool {
    visible(
        other.cloak,
        other.state.position,
        other.emerging(),
        Observer {
            at: observer.state.position,
            scanner: observer.cloak_gear.scanner.on_screen,
            cloaked_escort: observer.leader == Some(other.id) && observer.cloak_gear.device,
        },
    )
}

/// Whether `observer` sees the `player`: `_ShipVisibleToShip(player,
/// observer)` (@0x971c) with the player the target. The player never
/// emerges from a gate as an NPC does, and a cloaked player is seen by an
/// escort of it with a cloak itself (@0x9788-0x97a2, the player its
/// parent) or by a scanner on screen within 200 on both axes. The parent
/// rule of the player's close-formation escorts reads it
/// (`Traffic::think_cloaks`); [`Surroundings::sees`] does not ask it yet
/// (rdm:phase/cloaking-fidelity/phase-9-npc-sight-of-cloaked-player).
#[must_use]
pub fn sees_player(observer: &Npc, player: &PlayerSide) -> bool {
    visible(
        player.cloak,
        player.state.position,
        false,
        Observer {
            at: observer.state.position,
            scanner: observer.cloak_gear.scanner.on_screen,
            cloaked_escort: observer.escorts_player() && observer.cloak_gear.device,
        },
    )
}

#[cfg(test)]
mod tests {
    use crate::ai::fixture::{TRADERS, around, cloaked, govts, n, ship, sites};
    use crate::combat::ShipRef;
    use crate::traffic::npc::{AiType, Mode, Npc, NpcId};

    fn sees(observer: &Npc, other: Npc) -> bool {
        let (sites, govts) = (sites(), govts(0, 0));
        let npcs = [observer.clone(), other];
        let around = around(&sites, &npcs, &govts, (0.0, 0.0), 0);
        around.sees(&npcs[0], n(2))
    }

    fn warship() -> Npc {
        ship(1, TRADERS, AiType::Warship, 0.0, 0.0)
    }

    fn other(x: f32, y: f32) -> Npc {
        ship(2, TRADERS, AiType::WimpyTrader, x, y)
    }

    #[test]
    fn an_uncloaked_ship_is_seen_and_a_cloaked_one_is_not() {
        assert!(sees(&warship(), other(50.0, 0.0)));
        assert!(!sees(&warship(), cloaked(other(50.0, 0.0))));
    }

    #[test]
    fn a_scanner_on_screen_sees_a_cloaked_ship_within_200_on_both_axes() {
        let mut scanning = warship();
        scanning.cloak_gear.scanner.on_screen = true;
        assert!(sees(&scanning, cloaked(other(200.0, -200.0))));
        assert!(!sees(&scanning, cloaked(other(200.5, 0.0))));
        assert!(!sees(&scanning, cloaked(other(0.0, -200.5))));
    }

    #[test]
    fn an_escort_with_a_cloak_sees_its_cloaked_leader() {
        let mut escort = warship();
        escort.leader = Some(NpcId(2));
        escort.cloak_gear.device = true;
        assert!(sees(&escort, cloaked(other(900.0, 0.0))));
        escort.cloak_gear.device = false;
        assert!(!sees(&escort, cloaked(other(900.0, 0.0))));
        let mut stranger = warship();
        stranger.cloak_gear.device = true;
        assert!(!sees(&stranger, cloaked(other(900.0, 0.0))), "not its lead");
    }

    #[test]
    fn a_ship_emerging_from_a_gate_is_unseen_even_uncloaked() {
        let mut emerging = other(50.0, 0.0);
        emerging.mode = Mode::Emerging { ticks_left: 5 };
        assert!(!sees(&warship(), emerging));
    }

    #[test]
    fn an_escort_with_a_cloak_sees_the_cloaked_player_and_one_without_sees_it_only_by_scanner() {
        use crate::ai::fixture::player;
        use crate::escort::{EscortDuty, FormationSlot};
        let mut cloaked_player = player(900.0, 0.0);
        cloaked_player.cloak = crate::cloak::Cloak {
            fade: crate::cloak::FADE_FULL,
            direction: 0,
        };
        let mut escort = warship();
        escort.escort = Some(EscortDuty {
            formation: FormationSlot::default(),
            order: None,
        });
        assert!(escort.escorts_player());
        escort.cloak_gear.device = true;
        assert!(super::sees_player(&escort, &cloaked_player), "its escort");
        escort.cloak_gear.device = false;
        assert!(!super::sees_player(&escort, &cloaked_player), "no cloak");
        assert!(
            super::sees_player(&escort, &player(900.0, 0.0)),
            "uncloaked"
        );
        let mut stranger = warship();
        stranger.cloak_gear.device = true;
        assert!(!super::sees_player(&stranger, &cloaked_player), "not its");
        stranger.cloak_gear.scanner.on_screen = true;
        assert!(!super::sees_player(&stranger, &cloaked_player), "too far");
        let near = crate::ai::PlayerSide {
            state: player(200.0, -200.0).state,
            ..cloaked_player
        };
        assert!(super::sees_player(&stranger, &near), "scanned within 200");
    }

    #[test]
    fn the_player_is_always_seen_and_a_ship_not_in_the_system_reads_seen() {
        let (sites, govts) = (sites(), govts(0, 0));
        let npcs = [warship()];
        let around = around(&sites, &npcs, &govts, (0.0, 0.0), 0);
        assert!(around.sees(&npcs[0], ShipRef::Player));
        assert!(around.sees(&npcs[0], n(9)));
    }
}
