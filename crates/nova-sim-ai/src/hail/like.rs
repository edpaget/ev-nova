//! Whether a hailed ship likes the player, and its [`Attitude`].
//!
//! The rules are the original's (`_AIDoesShipLikePlayer` @0x82177 in the
//! `EV Nova` executable). G is the ship's government, S the system's, L
//! the player's legal record in the system (its record with S; none in an
//! independent system), and tol a government's `CrimeTol`. In order:
//!
//! 1. A ship that threatens the player does not like it:
//!    `_ExtendedIsThreatToPlayer` @0x81b70 (called @0x8218c), read whole as
//!    [`Npc::threatens_player_extended`] with the player's sight of it
//!    ([`Surroundings::seen_by_player`], @0x81bc6). So a ship the player
//!    can't see is no threat, nor is the player's own escort (@0x81b9f);
//!    a ship attacking the player or a ship it leads (@0x81c2e-0x81c4a),
//!    one whose hit's target is the player, and a lead whose escort
//!    fights the player (@0x81cdd-0x81dbe) are. Its stellar-defender
//!    clause (+0x8c, @0x81bd3) is left out, as the engine has no defence
//!    ships (rdm:task/planet-domination).
//! 2. The player's escorts do, hired or mission escorts (leader 0,
//!    @0x82199). So does a ship whose government blesses the player with
//!    peace (rank `Flags` 0x0100, `_PlayerBlessedByGovt(G, 0)` @0x821b9,
//!    [`Surroundings::blessed`]), before the mission and legal tests
//!    below: under either reading of `rank_govt_reach` and
//!    `blessing_refresh`, as `nova_sim::Session::blessing`
//!    takes them. Rule 1 still comes first, so a blessing excuses no ship
//!    that threatens the player.
//! 3. Any other mission's special ship (@0x821c6-0x8221e) likes it when
//!    its slot's `ShipGoal` is 3 or 4 and its `ShipBehav` is 1, and
//!    otherwise does not, with no government test; one whose slot no
//!    longer holds a mission does not. The one such ship that likes it
//!    is a `ShipBehav` 1 ship not in the formation, disabled before it
//!    joined.
//! 4. An independent ship does.
//! 5. A xenophobic G in another government's system does not when L is
//!    above that government's tol. At home, and otherwise, the rules
//!    below apply. (The original compares an independent system against
//!    government index 0's tol, a slip that cannot matter, as L is none
//!    there.)
//! 6. In an independent system it does. When S is G, or allied with G,
//!    it does while L is at least -tol(S); when S is G's enemy, only
//!    while L is below -tol(S). When S is neutral to G, a nosy G
//!    (`Flags` 0x0002) does while L is at least -tol(S), and any other G
//!    does.
//!
//! The government byte +0x83 the original falls back to is the IFF
//! byte, which the engine never sets. It is not the blessing, which is the
//! government's byte +0x84 (see `nova_sim::rank`), read in rule 2.
//!
//! The [`Attitude`] is hostile while the ship threatens the player as in
//! rule 1, friendly while it likes it, and unfriendly otherwise. Every
//! comm-dialog caller of `_ExtendedIsThreatToPlayer` reads it: the
//! "Status: Hostile" line (`_CommDialogUpdate` @0x90625,
//! `nova_sim::HailView::hostile`), the middle button, Beg for Mercy for a
//! threat and Request Assistance otherwise (`_DrawCommDialogButtons`
//! @0x28d5a, and its press, `_DoCommDialog` @0x960fa), the opening reply
//! (@0x95ce1) and Greetings (@0x966da).

use crate::ai::Surroundings;
use crate::govt::NOSY;
use crate::hail::MissionHail;
use crate::traffic::ShipBehav;
use crate::traffic::ShipGoal;
use crate::traffic::npc::Npc;

/// How a hailed ship feels about the player.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Attitude {
    /// It likes the player.
    Friendly,
    /// It neither likes nor threatens the player.
    Unfriendly,
    /// It threatens the player.
    Hostile,
}

/// Whether `npc` likes the player among `around` (see the module docs),
/// `mission` its mission's `ShipGoal` and `ShipBehav` as the session reads
/// them when it is a mission's special ship ([`MissionHail`]; a slot
/// that no longer holds a mission reads as no goal and its own
/// behaviour), none otherwise.
#[must_use]
pub fn likes_player(npc: &Npc, around: &Surroundings, mission: Option<MissionHail>) -> bool {
    if threatens(npc, around) {
        return false;
    }
    if npc.escorts_player() || around.blessed(npc.govt) {
        return true;
    }
    if npc.mission.is_some() {
        return mission.is_some_and(|mission| {
            matches!(mission.goal, ShipGoal::Escort | ShipGoal::Observe)
                && mission.behav == ShipBehav::EscortPlayer
        });
    }
    let Some(govt) = npc.govt else {
        return true;
    };
    let govts = around.govts;
    let record = i32::from(around.record);
    let Some(system) = around.system_govt else {
        return true;
    };
    let tolerance = i32::from(govts.crime_tol(Some(system)));
    if govts.xenophobic(Some(govt)) && system != govt && record > tolerance {
        return false;
    }
    if govts.allies(Some(system), Some(govt)) {
        record >= -tolerance
    } else if govts.enemies(Some(system), Some(govt)) {
        record < -tolerance
    } else if govts.flag(Some(govt), NOSY) {
        record >= -tolerance
    } else {
        true
    }
}

/// How `npc` feels about the player among `around`, of `mission` (see
/// [`likes_player`] and the module docs).
#[must_use]
pub fn attitude(npc: &Npc, around: &Surroundings, mission: Option<MissionHail>) -> Attitude {
    if threatens(npc, around) {
        Attitude::Hostile
    } else if likes_player(npc, around, mission) {
        Attitude::Friendly
    } else {
        Attitude::Unfriendly
    }
}

/// Rule 1: whether `npc` threatens the player among `around`, as
/// `_ExtendedIsThreatToPlayer` asks it with the player's sight of it.
fn threatens(npc: &Npc, around: &Surroundings) -> bool {
    npc.threatens_player_extended(around.npcs, around.seen_by_player(npc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::Goal;
    use crate::catalog::{GovtId, GovtRecord};
    use crate::combat::ShipRef;
    use crate::escort::FormationSlot;
    use crate::govt::{Governments, XENOPHOBIC};
    use crate::stats::ShipStats;
    use crate::traffic::ShipBehav;
    use crate::traffic::ShipGoal;
    use crate::traffic::npc::MissionMark;
    use nova_sim_base::fixture::FAST;
    use nova_sim_catalog::fixture::govt;

    const ME: GovtId = GovtId(140);
    const ALLY: GovtId = GovtId(141);
    const ENEMY: GovtId = GovtId(142);
    const NEUTRAL: GovtId = GovtId(143);
    const XENO: GovtId = GovtId(144);

    /// The ship's government 140 (class 1, with `flags`), its ally 141,
    /// its enemy 142, a neutral 143, and a xenophobe 144 of class 1; each
    /// of `CrimeTol` 6.
    fn govts(flags: u16) -> Governments {
        Governments::new([
            GovtRecord {
                flags,
                crime_tol: 6,
                classes: [1, -1, -1, -1],
                ..govt(140)
            },
            GovtRecord {
                crime_tol: 6,
                allies: [1, -1, -1, -1],
                classes: [2, -1, -1, -1],
                ..govt(141)
            },
            GovtRecord {
                crime_tol: 6,
                enemies: [1, -1, -1, -1],
                classes: [3, -1, -1, -1],
                ..govt(142)
            },
            GovtRecord {
                crime_tol: 6,
                classes: [4, -1, -1, -1],
                ..govt(143)
            },
            GovtRecord {
                flags: XENOPHOBIC,
                crime_tol: 6,
                classes: [1, -1, -1, -1],
                ..govt(144)
            },
        ])
    }

    /// A ship of `govt`, idle.
    fn ship(govt: Option<GovtId>) -> Npc {
        let mut npc = crate::traffic::fixture::npc(1, ShipStats::new(FAST, &[]));
        npc.govt = govt;
        npc
    }

    /// Whether a ship of `govt` (with `flags` for 140) likes the player
    /// of `record` in a system of `system`.
    fn likes(flags: u16, govt: Option<GovtId>, system: Option<GovtId>, record: i16) -> bool {
        let govts = govts(flags);
        let npc = ship(govt);
        let around = Surroundings {
            govts: &govts,
            system_govt: system,
            record,
            ..Surroundings::new(&[], &[])
        };
        likes_player(&npc, &around, None)
    }

    #[test]
    fn a_ship_threatening_the_player_never_likes_it() {
        let govts = govts(0);
        for govt in [None, Some(ME)] {
            let mut npc = ship(govt);
            npc.goal = Goal::Attack(ShipRef::Player);
            let around = Surroundings {
                govts: &govts,
                system_govt: Some(ME),
                record: 100,
                ..Surroundings::new(&[], &[])
            };
            assert!(!likes_player(&npc, &around, None), "{govt:?}");
            assert_eq!(attitude(&npc, &around, None), Attitude::Hostile);
        }
    }

    #[test]
    fn the_players_escort_likes_it_whatever_its_record() {
        let govts = govts(0);
        let mut escort = ship(Some(ALLY));
        escort.escort = Some(crate::escort::EscortDuty {
            formation: FormationSlot {
                slot: 2,
                ships: 2,
                spacing: 30.0,
            },
            order: None,
        });
        let around = Surroundings {
            govts: &govts,
            system_govt: Some(ME),
            record: -100,
            ..Surroundings::new(&[], &[])
        };
        assert!(!likes(0, Some(ALLY), Some(ME), -100), "not as a stranger");
        assert!(likes_player(&escort, &around, None));
        assert_eq!(attitude(&escort, &around, None), Attitude::Friendly);
    }

    #[test]
    fn an_independent_ship_likes_the_player_whatever_its_record() {
        assert!(likes(0, None, Some(ME), -100));
        assert!(likes(0, None, None, -100));
    }

    #[test]
    fn in_an_independent_system_every_ship_likes_the_player() {
        for govt in [ME, ENEMY, XENO] {
            assert!(likes(0x0002, Some(govt), None, -100), "{govt:?}");
        }
    }

    #[test]
    fn at_home_or_with_an_ally_it_likes_a_record_down_to_minus_the_systems_tolerance() {
        for system in [ME, ALLY] {
            assert!(likes(0, Some(ME), Some(system), -6), "{system:?}");
            assert!(!likes(0, Some(ME), Some(system), -7), "{system:?}");
            assert!(likes(0, Some(ME), Some(system), 30), "{system:?}");
        }
    }

    #[test]
    fn in_an_enemys_system_it_likes_only_a_record_below_minus_its_tolerance() {
        assert!(likes(0, Some(ME), Some(ENEMY), -7));
        assert!(!likes(0, Some(ME), Some(ENEMY), -6));
        assert!(!likes(0, Some(ME), Some(ENEMY), 30));
    }

    #[test]
    fn in_a_neutral_system_only_a_nosy_government_minds_the_record() {
        assert!(likes(0x0002, Some(ME), Some(NEUTRAL), -6));
        assert!(!likes(0x0002, Some(ME), Some(NEUTRAL), -7));
        assert!(likes(0, Some(ME), Some(NEUTRAL), -100));
    }

    #[test]
    fn a_xenophobe_at_home_likes_a_record_down_to_minus_its_tolerance() {
        assert!(likes(0, Some(XENO), Some(XENO), -6));
        assert!(!likes(0, Some(XENO), Some(XENO), -7));
        assert!(likes(0, Some(XENO), Some(XENO), 7), "at home a good record");
    }

    #[test]
    fn a_xenophobe_elsewhere_dislikes_a_record_above_that_tolerance_then_follows_the_rules() {
        // 144 is of class 1, which 141 is allied with.
        assert!(
            !likes(0, Some(XENO), Some(ALLY), 7),
            "above 141's tolerance"
        );
        assert!(likes(0, Some(XENO), Some(ALLY), 6), "then the ally rule");
        assert!(
            !likes(0, Some(XENO), Some(ALLY), -7),
            "which minds a bad record"
        );
        assert!(
            !likes(0, Some(XENO), Some(NEUTRAL), 0),
            "a xenophobe is everyone's enemy"
        );
        assert!(likes(0, Some(XENO), Some(NEUTRAL), -7));
    }

    #[test]
    fn the_attitude_is_friendly_unfriendly_or_hostile() {
        let govts = govts(0);
        let around = |record| Surroundings {
            govts: &govts,
            system_govt: Some(ME),
            record,
            ..Surroundings::new(&[], &[])
        };
        let npc = ship(Some(ME));
        assert_eq!(attitude(&npc, &around(0), None), Attitude::Friendly);
        assert_eq!(attitude(&npc, &around(-7), None), Attitude::Unfriendly);
        let mut hunting = npc.clone();
        hunting.goal = Goal::Snipe(ShipRef::Player);
        assert_eq!(attitude(&hunting, &around(0), None), Attitude::Hostile);
        let escort = Npc {
            escort: Some(crate::escort::EscortDuty {
                formation: FormationSlot::default(),
                order: None,
            }),
            ..crate::traffic::fixture::npc(5, ShipStats::new(FAST, &[]))
        };
        let npcs = [escort];
        let led = Surroundings {
            govts: &govts,
            system_govt: Some(ME),
            ..Surroundings::new(&[], &npcs)
        };
        let mut raider = npc.clone();
        raider.goal = Goal::Attack(ShipRef::Npc(crate::traffic::npc::NpcId(5)));
        assert_eq!(
            attitude(&raider, &led, None),
            Attitude::Hostile,
            "attacking a ship the player leads"
        );
        assert!(!likes_player(&raider, &led, None));
        assert_eq!(
            attitude(&raider, &around(0), None),
            Attitude::Friendly,
            "attacking a ship nobody leads"
        );
    }

    /// `npc`'s attitude and liking among `npcs`, of government 140 at home
    /// with a clean record, the player scanning for cloaks when `scanner`.
    fn feels(npc: &Npc, npcs: &[Npc], scanner: bool) -> (Attitude, bool) {
        let govts = govts(0);
        let around = Surroundings {
            govts: &govts,
            system_govt: Some(ME),
            cloak_scanner: scanner,
            ..Surroundings::new(&[], npcs)
        };
        (
            attitude(npc, &around, None),
            likes_player(npc, &around, None),
        )
    }

    /// A hired escort of the player, NPC `id`.
    fn hired(id: u32) -> Npc {
        Npc {
            escort: Some(crate::escort::EscortDuty {
                formation: FormationSlot::default(),
                order: None,
            }),
            ..crate::traffic::fixture::npc(id, ShipStats::new(FAST, &[]))
        }
    }

    const HOSTILE: (Attitude, bool) = (Attitude::Hostile, false);
    const FRIENDLY: (Attitude, bool) = (Attitude::Friendly, true);

    #[test]
    fn a_ship_attacking_a_hired_escort_is_hostile() {
        let mut raider = ship(Some(ME));
        raider.goal = Goal::Attack(ShipRef::Npc(crate::traffic::npc::NpcId(5)));
        assert_eq!(feels(&raider, &[hired(5)], false), HOSTILE);
    }

    #[test]
    fn a_ship_whose_hit_target_is_the_player_is_hostile() {
        let mut hit = ship(Some(ME));
        hit.target = Some(ShipRef::Player);
        assert_eq!(feels(&hit, &[], false), HOSTILE);
    }

    #[test]
    fn a_lead_whose_escort_attacks_the_player_is_hostile() {
        let mut lead = ship(Some(ME));
        lead.target = Some(ShipRef::Npc(crate::traffic::npc::NpcId(9)));
        let mut escort = ship(Some(ME));
        escort.id = crate::traffic::npc::NpcId(2);
        escort.leader = Some(lead.id);
        escort.goal = Goal::Attack(ShipRef::Player);
        assert_eq!(feels(&lead, &[escort], false), HOSTILE);
        assert_eq!(feels(&lead, &[], false), FRIENDLY, "without it");
    }

    #[test]
    fn a_cloaked_attacker_the_player_cannot_see_falls_through_to_the_other_rules() {
        let mut cloaked = ship(Some(ME));
        cloaked.goal = Goal::Attack(ShipRef::Player);
        cloaked.cloak = crate::cloak::Cloak {
            fade: crate::cloak::FADE_FULL,
            direction: 0,
        };
        assert_eq!(feels(&cloaked, &[], false), FRIENDLY, "unseen");
        assert_eq!(feels(&cloaked, &[], true), HOSTILE, "scanned on screen");
    }

    #[test]
    fn the_players_own_escort_attacking_it_is_no_threat_and_likes_it() {
        let mut escort = hired(1);
        escort.govt = Some(ENEMY);
        escort.goal = Goal::Attack(ShipRef::Player);
        assert_eq!(feels(&escort, &[], false), FRIENDLY);
    }

    // Mission ships.

    /// A ship of `govt` marked by slot 0's mission of `behav`, in the
    /// player's formation when `joined`.
    fn marked(govt: Option<GovtId>, behav: ShipBehav, joined: bool) -> Npc {
        let mut npc = ship(govt);
        npc.mission = Some(MissionMark {
            order: None,
            slot: 0,
            behav,
            formation: joined.then(FormationSlot::default),
        });
        npc
    }

    /// The mission of a marked ship whose slot holds one of `goal`, of
    /// `behav`.
    fn of(goal: ShipGoal, behav: ShipBehav) -> MissionHail {
        MissionHail { goal, behav }
    }

    /// Surroundings of `govts` in a system of `system`, the player's
    /// record there `record`.
    fn among(govts: &Governments, system: Option<GovtId>, record: i16) -> Surroundings<'_> {
        Surroundings {
            govts,
            system_govt: system,
            record,
            ..Surroundings::new(&[], &[])
        }
    }

    const GOALS: [ShipGoal; 9] = [
        ShipGoal::None,
        ShipGoal::Destroy,
        ShipGoal::Disable,
        ShipGoal::Board,
        ShipGoal::Escort,
        ShipGoal::Observe,
        ShipGoal::Rescue,
        ShipGoal::ChaseOff,
        ShipGoal::Other,
    ];
    const BEHAVS: [ShipBehav; 4] = [
        ShipBehav::Own,
        ShipBehav::AttackPlayer,
        ShipBehav::EscortPlayer,
        ShipBehav::AttackStellars,
    ];

    #[test]
    fn a_mission_escort_likes_the_player_whatever_its_government() {
        let govts = govts(0);
        for govt in [ENEMY, XENO] {
            let around = among(&govts, Some(ME), 100);
            assert!(!likes(0, Some(govt), Some(ME), 100), "not as a stranger");
            let escort = marked(Some(govt), ShipBehav::EscortPlayer, true);
            for goal in [ShipGoal::Escort, ShipGoal::Observe, ShipGoal::Destroy] {
                let mission = Some(of(goal, ShipBehav::EscortPlayer));
                assert!(likes_player(&escort, &around, mission), "{govt:?} {goal:?}");
                assert_eq!(attitude(&escort, &around, mission), Attitude::Friendly);
            }
        }
    }

    #[test]
    fn a_marked_ship_not_escorting_likes_the_player_only_for_goal_3_or_4_and_behaviour_1() {
        let govts = govts(0);
        for (govt, system, record) in [(None, None, 0), (Some(ENEMY), Some(ME), 100)] {
            let around = among(&govts, system, record);
            for behav in BEHAVS {
                let npc = marked(govt, behav, false);
                for goal in GOALS {
                    let liked = matches!(goal, ShipGoal::Escort | ShipGoal::Observe)
                        && behav == ShipBehav::EscortPlayer;
                    assert_eq!(
                        likes_player(&npc, &around, Some(of(goal, behav))),
                        liked,
                        "{govt:?} {behav:?} {goal:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_marked_ship_of_a_free_slot_or_with_no_mission_read_does_not_like_the_player() {
        let govts = govts(0);
        let around = among(&govts, None, 0);
        let npc = marked(None, ShipBehav::EscortPlayer, false);
        assert!(
            likes_player(&ship(None), &around, None),
            "unmarked, independent"
        );
        assert!(!likes_player(&npc, &around, None));
        let free = Some(of(ShipGoal::None, ShipBehav::Own));
        assert!(!likes_player(&npc, &around, free));
    }

    // The blessing.

    /// `around` with the player in the system, blessed by `blessed` alone
    /// when it is some.
    fn with_blessing<'a>(around: &Surroundings<'a>, blessed: Option<GovtId>) -> Surroundings<'a> {
        Surroundings {
            player: Some(crate::ai::PlayerSide {
                blessing: blessed
                    .map_or(crate::ai::Blessing::NONE, crate::ai::fixture::blessing_of),
                ..crate::ai::fixture::player(0.0, 0.0)
            }),
            ..*around
        }
    }

    #[test]
    fn a_blessing_government_likes_the_player_whatever_its_mission_or_laws() {
        let unmarked = |govt| (ship(Some(govt)), None);
        let free = (
            marked(Some(ME), ShipBehav::EscortPlayer, false),
            Some(of(ShipGoal::None, ShipBehav::Own)),
        );
        for (flags, (npc, mission), system, record, case) in [
            (
                0,
                unmarked(ME),
                ENEMY,
                30,
                "in an enemy's system, above its tolerance",
            ),
            (0, unmarked(XENO), ALLY, 7, "a xenophobe away from home"),
            (
                0x0002,
                unmarked(ME),
                NEUTRAL,
                -7,
                "nosy, below minus the tolerance",
            ),
            (0, free, ME, 0, "a mission ship of no goal"),
        ] {
            let govts = govts(flags);
            let around = among(&govts, Some(system), record);
            let govt = npc.govt;
            assert!(
                !likes_player(&npc, &with_blessing(&around, None), mission),
                "{case}: unblessed"
            );
            let blessed = with_blessing(&around, govt);
            assert!(likes_player(&npc, &blessed, mission), "{case}");
            assert_eq!(attitude(&npc, &blessed, mission), Attitude::Friendly);
        }
    }

    #[test]
    fn a_blessing_excuses_no_ship_that_threatens_the_player() {
        let govts = govts(0);
        let around = with_blessing(&among(&govts, Some(ME), 0), Some(ME));
        let mut npc = ship(Some(ME));
        assert!(likes_player(&npc, &around, None));
        npc.goal = Goal::Attack(ShipRef::Player);
        assert!(!likes_player(&npc, &around, None));
        assert_eq!(attitude(&npc, &around, None), Attitude::Hostile);
    }

    #[test]
    fn a_mission_ship_threatening_the_player_likes_it_only_in_the_formation() {
        // In the formation its leader (+0x9a) is the player, which
        // `_ExtendedIsThreatToPlayer` never counts (@0x81b9f); rule 2 then
        // likes it.
        let govts = govts(0);
        let around = among(&govts, None, 0);
        for joined in [false, true] {
            let mut npc = marked(None, ShipBehav::EscortPlayer, joined);
            npc.goal = Goal::Attack(ShipRef::Player);
            let mission = Some(of(ShipGoal::Escort, ShipBehav::EscortPlayer));
            assert_eq!(likes_player(&npc, &around, mission), joined, "{joined}");
        }
    }
}
