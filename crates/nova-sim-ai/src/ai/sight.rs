//! An NPC's sight of another ship: `_ShipVisibleToShip(ship, observer)`
//! (@0x971c-0x9809 in the `EV Nova` executable) with an NPC observer
//! ([`Surroundings::sees`]).
//!
//! The rule is [`cloak::visible`](crate::cloak::visible)'s: a ship coming
//! out of a gate is seen by nobody (@0x972d); an uncloaked one is seen;
//! a cloaked one is seen by an escort of it that has a cloak
//! (@0x9788-0x97a2, `_HasCloak`: its own, or the target's area cloak it
//! rides in close formation, [`rides_area_cloak`]), or by a ship with a
//! cloak scanner on screen within 200 of it on both axes (@0x97a4-0x97f8).
//! The player is seen by the same rule ([`sees_player`]), its escorts with
//! a cloak of their own, or riding its area cloak, seeing it cloaked. `_ShipVisibleToShip` tests no presence, so a ship
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
use crate::area_cloak;
use crate::cloak::{Observer, visible};
use crate::combat::ShipRef;
use crate::traffic::npc::Npc;

impl Surroundings<'_> {
    /// Whether `observer` sees `ship`: `_ShipVisibleToShip(ship,
    /// observer)` (@0x971c; see the module docs).
    #[must_use]
    pub fn sees(&self, observer: &Npc, ship: ShipRef) -> bool {
        sees_among(observer, ship, self.npcs, self.player.as_ref())
    }

    /// Whether `npc` sees its leader, `_ShipVisibleToShip(leader, npc)`
    /// (@0x8ed5f): see [`sees_leader`].
    #[must_use]
    pub fn sees_leader(&self, npc: &Npc) -> bool {
        sees_leader(npc, self.npcs, self.player.as_ref())
    }

    /// Whether `npc` rides its leader's area cloak: see
    /// [`rides_area_cloak`].
    #[must_use]
    pub fn rides_area_cloak(&self, npc: &Npc) -> bool {
        rides_area_cloak(npc, self.npcs, self.player.as_ref())
    }
}

/// Whether `observer` sees `ship` among `npcs` and the `player` (`None`
/// while it isn't in the system): [`Surroundings::sees`].
fn sees_among(observer: &Npc, ship: ShipRef, npcs: &[Npc], player: Option<&PlayerSide>) -> bool {
    match ship {
        ShipRef::Npc(id) => npcs
            .iter()
            .find(|other| other.id == id)
            .is_none_or(|other| npc_sees(observer, other)),
        ShipRef::Player => player.is_none_or(|player| sees_player(observer, player)),
    }
}

/// Whether `npc` sees its leader among `npcs` and the `player` (`None`
/// while it isn't in the system), `_ShipVisibleToShip(leader, npc)`
/// (@0x8ed5f): the leader its single parent slot (+0x9a) names,
/// [`Npc::leader_ref`], so the player for an escort of the player even
/// if it still names an NPC lead. A ship with no leader, or whose leader
/// is gone, reads as seeing it. The formation flight
/// ([`fly_pass`](crate::traffic::fly_pass)), the cloak's parent rule
/// (`Traffic::think_cloaks`) and the session's gravity check
/// (`nova_sim::Session`) all read it, so they agree on one leader and one
/// sight.
#[must_use]
pub fn sees_leader(npc: &Npc, npcs: &[Npc], player: Option<&PlayerSide>) -> bool {
    npc.leader_ref()
        .is_none_or(|lead| sees_among(npc, lead, npcs, player))
}

/// Whether `npc` rides its leader's area cloak among `npcs` and the
/// `player` (`None` while it isn't in the system), as `_HasCloak`'s area
/// branch asks (@0x964c-0x96bf, [`area_cloak::rides`]): the leader its
/// single parent slot (+0x9a) names ([`Npc::leader_ref`]), cloaked and
/// carrying an area cloak, with `npc` in close formation by the stored
/// mark of its last flight ([`Npc::close_form`]). False with no leader or
/// a gone one. Sight ([`npc_sees`], [`sees_player`]) and the session's
/// fight terms read the stored mark, as they run before the frame's
/// flight; the cloak think builds its ride from its own `Around`, and the
/// hold from the close formation this tick's flight picks
/// (`Traffic::hold_cloaks`).
#[must_use]
pub fn rides_area_cloak(npc: &Npc, npcs: &[Npc], player: Option<&PlayerSide>) -> bool {
    let (cloaked, area) = match npc.leader_ref() {
        Some(ShipRef::Player) => match player {
            Some(player) => (player.cloak.is_cloaked(), player.area_cloak),
            None => return false,
        },
        Some(ShipRef::Npc(id)) => match npcs.iter().find(|other| other.id == id) {
            Some(lead) => (lead.cloak.is_cloaked(), lead.cloak_gear.area),
            None => return false,
        },
        None => return false,
    };
    area_cloak::rides(npc.close_form, cloaked, area)
}

/// Whether `observer` sees NPC `other`: `_ShipVisibleToShip(other,
/// observer)` for two NPCs (@0x971c; see the module docs). Its
/// cloaked-escort term (@0x9788-0x97a2) holds for an observer whose parent
/// slot (+0x9a, [`Npc::leader_ref`]) names `other` and that has a cloak
/// ([`area_cloak::has_cloak`]): its own, or `other`'s area cloak it rides
/// in close formation. A player's escort's parent is the player, whatever
/// NPC lead it still names.
#[must_use]
pub fn npc_sees(observer: &Npc, other: &Npc) -> bool {
    let rides = area_cloak::rides(
        observer.close_form,
        other.cloak.is_cloaked(),
        other.cloak_gear.area,
    );
    visible(
        other.cloak,
        other.state.position,
        other.emerging(),
        Observer {
            at: observer.state.position,
            scanner: observer.cloak_gear.scanner.on_screen,
            cloaked_escort: observer.leader_ref() == Some(ShipRef::Npc(other.id))
                && area_cloak::has_cloak(observer.cloak_gear.device, rides),
        },
    )
}

/// Whether `observer` sees the `player`: `_ShipVisibleToShip(player,
/// observer)` (@0x971c) with the player the target. The player never
/// emerges from a gate as an NPC does, and a cloaked player is seen by an
/// escort of it (@0x9788-0x97a2, the player its parent) with a cloak of
/// its own or riding the player's area cloak in close formation
/// ([`area_cloak::has_cloak`]), or by a scanner on screen within 200 on
/// both axes. [`Surroundings::sees`] asks it for [`ShipRef::Player`]; the
/// parent rule of the player's close-formation escorts
/// (`Traffic::think_cloaks`) and the session's own sites read it directly.
#[must_use]
pub fn sees_player(observer: &Npc, player: &PlayerSide) -> bool {
    let rides = area_cloak::rides(
        observer.close_form,
        player.cloak.is_cloaked(),
        player.area_cloak,
    );
    visible(
        player.cloak,
        player.state.position,
        false,
        Observer {
            at: observer.state.position,
            scanner: observer.cloak_gear.scanner.on_screen,
            cloaked_escort: observer.escorts_player()
                && area_cloak::has_cloak(observer.cloak_gear.device, rides),
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
    fn an_npc_sees_its_leader_by_its_leader_ref_the_player_before_an_npc_lead() {
        use crate::ai::fixture::{cloaked_player, player};
        use crate::escort::{EscortDuty, FormationSlot};
        let (sites, govts) = (sites(), govts(0, 0));
        let sees_leader = |observer: Npc, lead: Npc, cloaked: bool| {
            let npcs = [observer, lead];
            let mut around = around(&sites, &npcs, &govts, (900.0, 0.0), 0);
            if cloaked {
                around.player = Some(cloaked_player(player(900.0, 0.0)));
            }
            around.sees_leader(&npcs[0])
        };
        assert!(
            sees_leader(warship(), cloaked(other(900.0, 0.0)), true),
            "no leader"
        );
        let mut follower = warship();
        follower.leader = Some(NpcId(2));
        assert!(
            !sees_leader(follower.clone(), cloaked(other(900.0, 0.0)), false),
            "its cloaked lead"
        );
        assert!(
            sees_leader(follower.clone(), other(900.0, 0.0), true),
            "its uncloaked lead"
        );
        follower.leader = Some(NpcId(9));
        assert!(
            sees_leader(follower, cloaked(other(900.0, 0.0)), true),
            "a lead not here"
        );
        let mut escort = warship();
        escort.escort = Some(EscortDuty {
            formation: FormationSlot::default(),
            order: None,
        });
        assert!(
            !sees_leader(escort.clone(), other(900.0, 0.0), true),
            "its cloaked player"
        );
        assert!(
            sees_leader(escort.clone(), cloaked(other(900.0, 0.0)), false),
            "its uncloaked player"
        );
        // An escort of the player follows the player (+0x9a 0), whatever
        // NPC lead it still names.
        escort.leader = Some(NpcId(2));
        assert!(
            !sees_leader(escort.clone(), other(900.0, 0.0), true),
            "the player first"
        );
        let npcs = [escort];
        let mut around = around(&sites, &npcs, &govts, (0.0, 0.0), 0);
        around.player = None;
        assert!(around.sees_leader(&npcs[0]), "no player here");
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

    /// A fleet escort of NPC 2 with no cloak of its own, in close
    /// formation as `close_form` says.
    fn fleet_escort(close_form: bool) -> Npc {
        let mut escort = warship();
        escort.leader = Some(NpcId(2));
        escort.close_form = close_form;
        escort
    }

    /// NPC 2, 900 off, fully cloaked, with an area cloak as `area` says.
    fn cloaked_lead(area: bool) -> Npc {
        let mut lead = cloaked(other(900.0, 0.0));
        lead.cloak_gear.area = area;
        lead
    }

    /// The player's escort, with no cloak of its own, in close formation
    /// as `close_form` says.
    fn players_escort(close_form: bool) -> Npc {
        use crate::escort::{EscortDuty, FormationSlot};
        let mut escort = warship();
        escort.escort = Some(EscortDuty {
            formation: FormationSlot::default(),
            order: None,
        });
        escort.close_form = close_form;
        escort
    }

    /// The player 900 off, fully cloaked, with an area cloak as `area`
    /// says.
    fn area_cloaked_player(area: bool) -> crate::ai::PlayerSide {
        use crate::ai::fixture::{cloaked_player, player};
        crate::ai::PlayerSide {
            area_cloak: area,
            ..cloaked_player(player(900.0, 0.0))
        }
    }

    #[test]
    fn an_escort_in_close_formation_sees_its_area_cloaked_npc_leader() {
        assert!(sees(&fleet_escort(true), cloaked_lead(true)));
        assert!(super::npc_sees(&fleet_escort(true), &cloaked_lead(true)));
    }

    #[test]
    fn an_escort_in_close_formation_sees_the_area_cloaked_player() {
        assert!(super::sees_player(
            &players_escort(true),
            &area_cloaked_player(true)
        ));
    }

    #[test]
    fn an_escort_out_of_close_formation_rides_no_area_cloak() {
        assert!(!sees(&fleet_escort(false), cloaked_lead(true)));
        assert!(!super::sees_player(
            &players_escort(false),
            &area_cloaked_player(true)
        ));
        let npcs = [fleet_escort(false), cloaked_lead(true)];
        assert!(!super::rides_area_cloak(&npcs[0], &npcs, None));
    }

    #[test]
    fn an_escort_of_a_leader_without_an_area_cloak_sees_it_cloaked_only_with_a_cloak_of_its_own() {
        assert!(!sees(&fleet_escort(true), cloaked_lead(false)));
        assert!(!super::sees_player(
            &players_escort(true),
            &area_cloaked_player(false)
        ));
        let mut own = fleet_escort(true);
        own.cloak_gear.device = true;
        assert!(sees(&own, cloaked_lead(false)));
        let mut own = players_escort(true);
        own.cloak_gear.device = true;
        assert!(super::sees_player(&own, &area_cloaked_player(false)));
    }

    #[test]
    fn a_players_escort_naming_an_npc_lead_is_no_cloaked_escort_of_that_lead() {
        // `_ShipVisibleToShip` compares the observer's single parent slot
        // (+0x9a), which is the player for the player's escort.
        let mut escort = players_escort(true);
        escort.leader = Some(NpcId(2));
        escort.cloak_gear.device = true;
        assert!(!super::npc_sees(&escort, &cloaked_lead(true)));
        assert!(super::npc_sees(
            &fleet_escort_with_cloak(),
            &cloaked_lead(false)
        ));
    }

    /// [`fleet_escort`] out of close formation, with a cloak of its own.
    fn fleet_escort_with_cloak() -> Npc {
        let mut escort = fleet_escort(false);
        escort.cloak_gear.device = true;
        escort
    }

    #[test]
    fn an_escort_rides_the_area_cloak_of_the_leader_its_parent_slot_names_and_none_once_it_is_gone()
    {
        use super::rides_area_cloak;
        let npcs = [fleet_escort(true), cloaked_lead(true)];
        assert!(rides_area_cloak(&npcs[0], &npcs, None), "its NPC lead");
        assert!(
            rides_area_cloak(&npcs[0], &npcs, Some(&area_cloaked_player(false))),
            "the player is not its leader"
        );
        let unclouded = [fleet_escort(true), other(900.0, 0.0)];
        assert!(
            !rides_area_cloak(&unclouded[0], &unclouded, None),
            "an uncloaked lead"
        );
        let gone = [fleet_escort(true)];
        assert!(!rides_area_cloak(&gone[0], &gone, None), "its lead gone");
        let mut lone = warship();
        lone.close_form = true;
        let alone = [lone, cloaked_lead(true)];
        assert!(!rides_area_cloak(&alone[0], &alone, None), "no leader");
        let mut escort = players_escort(true);
        escort.leader = Some(NpcId(2));
        let led = [escort, cloaked_lead(true)];
        assert!(
            !rides_area_cloak(&led[0], &led, Some(&area_cloaked_player(false))),
            "the player is its leader, with no area cloak"
        );
        assert!(
            rides_area_cloak(&led[0], &led, Some(&area_cloaked_player(true))),
            "the player's area cloak"
        );
        assert!(
            !rides_area_cloak(&led[0], &led, None),
            "no player in the system"
        );
        let (sites, govts) = (sites(), govts(0, 0));
        let mut around = around(&sites, &led, &govts, (900.0, 0.0), 0);
        around.player = Some(area_cloaked_player(true));
        assert!(around.rides_area_cloak(&led[0]));
    }
}
