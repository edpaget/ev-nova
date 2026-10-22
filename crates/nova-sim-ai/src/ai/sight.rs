//! An NPC's sight of another ship: `_ShipVisibleToShip(ship, observer)`
//! (@0x971c-0x9809 in the `EV Nova` executable) with an NPC observer
//! ([`Surroundings::sees`]).
//!
//! The rule is [`cloak::visible`](crate::cloak::visible)'s: a ship coming
//! out of a gate is seen by nobody (@0x972d); an uncloaked one is seen;
//! a cloaked one is seen by an escort of it that has a cloak itself
//! (@0x9788-0x97a2), or by a ship with a cloak scanner on screen within
//! 200 of it on both axes (@0x97a4-0x97f8). The player is seen by the
//! same rule ([`sees_player`]), its escorts with a cloak of their own
//! seeing it cloaked. `_ShipVisibleToShip` tests no presence, so a ship
//! not in the system (or no player in it) reads as seen: the gates that
//! drop a gone target (`target::dropped`, [`Surroundings::live`]) are not
//! a search's.
//!
//! Its readers: an NPC's fire ([`fire`](crate::ai::fire), whose sight of
//! its target holds its fire: `_AIFireGun` @0x7ff25, `_AIFireTurret`
//! @0x80b69), the lost-sight search ([`lost_sight`](crate::ai::lost_sight),
//! @0x8d738), the escorts' pick of a nearby threat
//! ([`escort::threat_score`](crate::escort::threat_score),
//! [`escort::player_threat_score`](crate::escort::player_threat_score),
//! `_IsShipANearbyThreatToParent` @0x8370e), the target picks
//! ([`target`](crate::ai::target), `_SelectWarshipTarget` @0x89f04,
//! @0x89f7d, @0x8a0d2, @0x8a140, @0x8a538, @0x8a6df, @0x8a748, @0x8a9ba,
//! @0x8aa7b, @0x8ad92), the governments' 0x0004 word
//! ([`warship`](crate::ai::warship), `_WarshipAI` @0x8b7ce,
//! `_InterceptorAI` @0x8cb80), the interceptor's pick of a ship to
//! inspect ([`interceptor`](crate::ai::interceptor), `_InterceptorAI`
//! @0x8cd5f, @0x8ce13, @0x8d3cd), the mission hunter's attack call
//! ([`nova`](crate::ai::nova), `_HighLevelAIHandler` @0x8d6a8), the scan
//! ([`scan`](crate::scan), `_ScanPlayer` @0x7e53f), the person's hail
//! quotes (`nova_sim::Session`, `_HandleShip` @0x33c87) and the escort's
//! formation flight ([`fly_pass`](crate::traffic::fly_pass), @0x8ed5f).
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
        match ship {
            ShipRef::Npc(id) => self.npc(id).is_none_or(|other| npc_sees(observer, other)),
            ShipRef::Player => self
                .player
                .as_ref()
                .is_none_or(|player| sees_player(observer, player)),
        }
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
/// parent) or by a scanner on screen within 200 on both axes.
/// [`Surroundings::sees`] asks it for [`ShipRef::Player`]; the parent
/// rule of the player's close-formation escorts (`Traffic::think_cloaks`)
/// and the session's own sites read it directly.
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
    fn the_player_is_seen_by_its_cloak_and_a_ship_not_in_the_system_reads_seen() {
        use crate::ai::fixture::{cloaked_player, player, scanning};
        use crate::escort::{EscortDuty, FormationSlot};
        let (sites, govts) = (sites(), govts(0, 0));
        let sees_player = |observer: Npc, at: (f32, f32), cloak: bool| {
            let npcs = [observer];
            let mut around = around(&sites, &npcs, &govts, at, 0);
            if cloak {
                around.player = Some(cloaked_player(player(at.0, at.1)));
            }
            around.sees(&npcs[0], ShipRef::Player)
        };
        assert!(sees_player(warship(), (900.0, 0.0), false), "uncloaked");
        assert!(!sees_player(warship(), (900.0, 0.0), true), "cloaked");
        assert!(
            sees_player(scanning(warship()), (200.0, -200.0), true),
            "scanned within 200"
        );
        assert!(
            !sees_player(scanning(warship()), (200.5, 0.0), true),
            "scanner too far"
        );
        let mut escort = warship();
        escort.escort = Some(EscortDuty {
            formation: FormationSlot::default(),
            order: None,
        });
        escort.cloak_gear.device = true;
        assert!(
            sees_player(escort, (900.0, 0.0), true),
            "its cloaked escort"
        );

        let npcs = [warship()];
        let mut around = around(&sites, &npcs, &govts, (0.0, 0.0), 0);
        around.player = None;
        assert!(around.sees(&npcs[0], ShipRef::Player), "no player here");
        assert!(around.sees(&npcs[0], n(9)));
    }
}
