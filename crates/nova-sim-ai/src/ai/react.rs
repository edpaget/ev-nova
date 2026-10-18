//! How a ship hit answers, and how others come to its help: Nova's
//! [`answer`], which the four AI types share.
//!
//! The rules are the original's (`_DamageShip` @0x3bb20-0x3bed2 and
//! `_DoGoodSamaritan` @0x3bef4 in the `EV Nova` executable). A strike
//! reaches them only once answered ([`Strike::answered`]): `_DamageShip`'s
//! answer flag, which holds back a ship's kin, its fleet and much else,
//! skips the whole block when clear (see
//! [`answer_flag`](crate::ai::answer_flag)).
//!
//! - **The victim**, and the lead of an escort hit, add the damage to
//!   their provocation and take the attacker as their target, unless they
//!   already attack another ship nearer than half the attacker's distance
//!   ([`FOCUS`] of its distance squared) while facing within
//!   [`FOCUS_ANGLE`] of it: then the hit changes nothing. A ship still
//!   coming into the system answers so too, gliding in, waiting in its
//!   gate or bursting out of it: `_DamageShip` drops the reaction only
//!   while the victim's (or the lead's) hyperspace state +0x50 is above
//!   0 (@0x3b4b0-0x3b4c2, @0x3b97e-0x3b995, @0x3bab7-0x3bae4), which is a
//!   ship jumping out, and an arrival's never is: -1.0 waiting in a gate
//!   (@0x89555), -999.0 gliding in or bursting out (@0x82ece,
//!   @0x8f3c8-0x8f3d9), until `_AIMakeShipStopJumping` zeroes it
//!   (@0x82eb0). (Phase 6 read +0x50 as running through an arrival; it
//!   does not.) Which ships answer is the traffic's to say
//!   ([`RuleKey::ArrivalHits`](crate::RuleKey::ArrivalHits)). A victim
//!   with a wait that answers has it cut to 20 ticks (@0x3be6e-0x3be7c),
//!   which the traffic does
//!   ([`autopilot::HIT_WAIT`](crate::traffic::autopilot::HIT_WAIT)).
//! - **A Good Samaritan**: for a hit by the player on the ship it was
//!   meant for ([`Strike::intended`], @0x3bed2), every warship or
//!   interceptor W not already attacking takes the player as its target
//!   and attacks at once, unless W is an NPC enemy of the victim, or the
//!   victim's government is xenophobic and not W's; the victim's
//!   government is derelict or ignored when attacked (`Flags` 0x0020); or
//!   W is neither allied with the victim nor nosy (`Flags` 0x0002). For
//!   an independent W, only the derelict, 0x0020 and xenophobe tests
//!   apply; for an independent victim, only a nosy W answers. A W whose
//!   government blesses the player with peace (rank `Flags` 0x0100,
//!   [`Surroundings::blessed`]: `_PlayerBlessedByGovt(W's govt, 0)`
//!   @0x829bf) never answers, under either reading of `rank_govt_reach`
//!   and `blessing_refresh`; but the original reads the blessing only for
//!   a governed victim (an independent one jumps past it, @0x82919 to
//!   0x82aa0), so a nosy blessing W still answers an attack on an
//!   independent. (The attacker is always the player here: both callers
//!   pass it, so the original's clause for a ship the player leads,
//!   @0x829a0-0x829b2, never arises.) There is no limit on range. The original calls the same `_DoGoodSamaritan`
//!   when the player boards a ship, after the crime
//!   (`_HandlePlayerBoardAttempt`), so the session queues a strike of no
//!   damage, direct and intended, for a boarding
//!   (`nova_sim::Session::board`), which the blessing
//!   holds back alike.
//!   The Bible's "piracy police" are the interceptors alone: "acts as
//!   'piracy police' by attacking any ship that fires on or attempts to
//!   board another, non-enemy ship" (`düde` AI type 4). Which applies is
//!   the [`Rulebook`](crate::Rulebook)'s
//!   [`RuleKey::PiracyPolice`](crate::RuleKey::PiracyPolice) entry,
//!   passed to [`answer`]: by the engine, warships and interceptors
//!   answer; by the Bible, interceptors only.
//!
//! So Federation police answer the player's attack on a Civvies trader,
//! since the Civvies are Federation allies. The original's distress
//! message (`_AICallForHelp`) only tells the player (see
//! [`call_for_help`](super::call_for_help)); the system's reinforcement
//! fleet comes by its own call (`_AICallForReinforcements`, see
//! [`reinforcements`](super::reinforcements)), not by a hit.

use crate::ai::{Goal, Reaction, Surroundings};
use crate::combat::aim::{angle_off, bearing};
use crate::combat::{ShipRef, Strike};
use crate::govt::{DERELICT, IGNORED_WHEN_ATTACKED, NOSY};
use crate::rulebook::RuleSource;
use crate::traffic::npc::{AiType, Npc};

/// The share of the attacker's distance squared within which a ship
/// keeps attacking its own target (0.25 @0x3bc3b).
pub const FOCUS: f32 = 0.25;
/// How far off its nose, in degrees, a ship keeping its own target must
/// have it.
pub const FOCUS_ANGLE: f32 = 44.0;

/// How `npc` answers `strike` among `around`, the Good Samaritans being
/// those `piracy_police` says (see the module docs).
#[must_use]
pub fn answer(
    npc: &Npc,
    strike: &Strike,
    around: &Surroundings,
    piracy_police: RuleSource,
) -> Reaction {
    let me = ShipRef::Npc(npc.id);
    let escort_hit = match strike.ship {
        ShipRef::Npc(id) => around.npc(id).is_some_and(|hit| hit.leader == Some(npc.id)),
        ShipRef::Player => false,
    };
    if strike.ship == me || escort_hit {
        // A hit by no ship provokes no one (`_DamageShip` sets a target
        // only for a ship source, @0x3bcf0, @0x3bd64).
        let Some(by) = strike.by else {
            return Reaction::default();
        };
        return if ignores(npc, by, around) {
            Reaction::default()
        } else {
            Reaction {
                target: Some(by),
                provoked: strike.damage,
                goal: None,
            }
        };
    }
    let polices = match piracy_police {
        RuleSource::Engine => true,
        RuleSource::Bible => npc.ai_type == AiType::Interceptor,
    };
    let samaritans = strike.by == Some(ShipRef::Player) && strike.intended;
    if samaritans && polices && good_samaritan(npc, strike.ship, around) {
        return Reaction {
            target: Some(ShipRef::Player),
            provoked: 0.0,
            goal: Some(Goal::Attack(ShipRef::Player)),
        };
    }
    Reaction::default()
}

/// Whether `npc`, hit (or its escort hit) by `attacker`, lets it be: it
/// keeps to its own nearer target ahead of it. Kin is not read: the
/// answer flag held it back already ([`answer_flag`](crate::ai::answer_flag)).
/// Its mode is not read: `_DamageShip` skips a reaction only while +0x50
/// is above 0 (@0x3b4b0-0x3b4c2), a ship jumping out, never one coming in
/// (see the module docs).
fn ignores(npc: &Npc, attacker: ShipRef, around: &Surroundings) -> bool {
    let squared = |ship: ShipRef| {
        around.state_of(ship).map(|state| {
            let off = state.position - npc.state.position;
            (off.x * off.x + off.y * off.y, state.position)
        })
    };
    let Some(own) = npc.goal.attacking().filter(|&own| around.live(own)) else {
        return false;
    };
    match (squared(own), squared(attacker)) {
        (Some((near, at)), Some((far, _))) => {
            near < FOCUS * far
                && angle_off(npc.state.heading, bearing(npc.state.position, at)) <= FOCUS_ANGLE
        }
        _ => false,
    }
}

/// Whether `npc` answers the player's attack on `victim` among `around`
/// (see the module docs).
fn good_samaritan(npc: &Npc, victim: ShipRef, around: &Surroundings) -> bool {
    let ShipRef::Npc(victim_id) = victim else {
        return false;
    };
    if victim_id == npc.id
        || npc.ai_type.trades()
        || npc.goal.attacking().is_some()
        || around.npc(victim_id).is_none()
    {
        return false;
    }
    let govts = around.govts;
    let theirs = around.govt_of(victim);
    let own = npc.govt;
    let nosy = govts.flag(own, NOSY);
    if theirs.is_none() {
        return nosy;
    }
    if around.blessed(own) {
        return false;
    }
    if govts.flag(theirs, DERELICT) || govts.flag(theirs, IGNORED_WHEN_ATTACKED) {
        return false;
    }
    if govts.xenophobic(theirs) && theirs != own {
        return false;
    }
    own.is_none() || (!govts.npc_enemies(own, theirs) && (govts.allies(own, theirs) || nosy))
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::{GovtId, GovtRecord};
    use crate::geometry::Vec2;
    use crate::govt::{DERELICT, Governments, IGNORED_WHEN_ATTACKED, NOSY, XENOPHOBIC};
    use crate::rulebook::RuleSource;
    use crate::stats::ShipStats;
    use crate::traffic::npc::{AiType, Mode, NpcId};
    use nova_sim_base::fixture::FAST;
    use nova_sim_catalog::fixture::govt;

    const TRADERS: GovtId = GovtId(140);
    const POLICE: GovtId = GovtId(141);
    const PIRATES: GovtId = GovtId(142);
    const NEUTRAL: GovtId = GovtId(143);
    const XENO: GovtId = GovtId(144);

    /// Traders (class 1), police allied with them, pirates at war with
    /// them, a neutral and a xenophobe, with `flags` on the traders and
    /// `nosy` on the neutral.
    fn relations(flags: u16, nosy: u16) -> Governments {
        Governments::new([
            GovtRecord {
                flags,
                classes: [1, -1, -1, -1],
                ..govt(140)
            },
            GovtRecord {
                allies: [1, -1, -1, -1],
                classes: [2, -1, -1, -1],
                ..govt(141)
            },
            GovtRecord {
                enemies: [1, -1, -1, -1],
                classes: [3, -1, -1, -1],
                ..govt(142)
            },
            GovtRecord {
                flags: nosy,
                ..govt(143)
            },
            GovtRecord {
                flags: XENOPHOBIC,
                ..govt(144)
            },
        ])
    }

    /// NPC `id` of `govt` and `ai_type` at (`x`, 0), facing up.
    fn ship(id: u32, govt: Option<GovtId>, ai_type: AiType, x: f32) -> Npc {
        let mut npc = crate::traffic::fixture::npc(id, ShipStats::new(FAST, &[]));
        npc.govt = govt;
        npc.ai_type = ai_type;
        npc.state.position = Vec2::new(x, 0.0);
        npc
    }

    fn n(id: u32) -> ShipRef {
        ShipRef::Npc(NpcId(id))
    }

    use crate::combat::fixture::strike;

    fn answered(govts: &Governments, npcs: &[Npc], who: usize, hit: &Strike) -> Reaction {
        answered_by(RuleSource::Engine, govts, npcs, who, hit)
    }

    fn answered_by(
        police: RuleSource,
        govts: &Governments,
        npcs: &[Npc],
        who: usize,
        hit: &Strike,
    ) -> Reaction {
        let around = Surroundings {
            govts,
            ..Surroundings::new(&[], npcs)
        };
        answer(&npcs[who], hit, &around, police)
    }

    const TAKES: fn(ShipRef) -> Reaction = |attacker| Reaction {
        target: Some(attacker),
        provoked: 12.0,
        goal: None,
    };

    #[test]
    fn the_victim_takes_its_attacker_and_adds_the_damage() {
        let govts = relations(0, 0);
        let npcs = [
            ship(1, Some(TRADERS), AiType::WimpyTrader, 0.0),
            ship(2, Some(PIRATES), AiType::Warship, 500.0),
        ];
        let hit = strike(n(1), n(2));
        assert_eq!(answered(&govts, &npcs, 0, &hit), TAKES(n(2)));
        assert_eq!(
            answered(&govts, &npcs, 0, &strike(n(1), ShipRef::Player)),
            TAKES(ShipRef::Player)
        );
        assert_eq!(
            answered(&govts, &npcs, 1, &hit),
            Reaction::default(),
            "the attacker"
        );
    }

    #[test]
    fn a_hit_by_no_ship_provokes_no_one() {
        let govts = relations(0, 0);
        let mut escort = ship(2, Some(TRADERS), AiType::Warship, 50.0);
        escort.leader = Some(NpcId(1));
        let npcs = [ship(1, Some(TRADERS), AiType::WimpyTrader, 0.0), escort];
        for hit in [n(1), n(2)] {
            let by_none = Strike {
                by: None,
                ..strike(hit, n(1))
            };
            for who in 0..2 {
                assert_eq!(
                    answered(&govts, &npcs, who, &by_none),
                    Reaction::default(),
                    "{hit:?} {who}"
                );
            }
        }
    }

    #[test]
    fn the_lead_of_an_escort_hit_takes_its_attacker_too() {
        let govts = relations(0, 0);
        let mut escort = ship(2, Some(TRADERS), AiType::Warship, 50.0);
        escort.leader = Some(NpcId(1));
        let npcs = [
            ship(1, Some(TRADERS), AiType::WimpyTrader, 0.0),
            escort,
            ship(3, Some(PIRATES), AiType::Warship, 500.0),
        ];
        let hit = strike(n(2), n(3));
        assert_eq!(answered(&govts, &npcs, 0, &hit), TAKES(n(3)));
        assert_eq!(answered(&govts, &npcs, 1, &hit), TAKES(n(3)));
    }

    #[test]
    fn an_answered_hit_by_its_own_government_or_fleet_is_taken() {
        // Kin is the answer flag's (`ai::answer_flag`, @0x3b4cd-0x3b4fc):
        // a hit that reaches the reaction was answered, and the reaction
        // block, the lead's part included (@0x3bcfa-0x3bd79), has no kin
        // test of its own.
        let govts = relations(0, 0);
        let mut npcs = [
            ship(1, Some(TRADERS), AiType::WimpyTrader, 0.0),
            ship(2, Some(TRADERS), AiType::Warship, 500.0),
            ship(3, Some(TRADERS), AiType::Warship, 50.0),
        ];
        npcs[2].leader = Some(NpcId(1));
        let hit = strike(n(1), n(2));
        assert_eq!(answered(&govts, &npcs, 0, &hit), TAKES(n(2)));
        assert_eq!(
            answered(&govts, &npcs, 0, &strike(n(3), n(2))),
            TAKES(n(2)),
            "its escort hit by its government"
        );
        npcs[1].govt = Some(NEUTRAL);
        npcs[1].leader = Some(NpcId(1));
        assert_eq!(
            answered(&govts, &npcs, 0, &hit),
            TAKES(n(2)),
            "by its escort"
        );
        npcs[1].leader = None;
        npcs[0].govt = None;
        npcs[1].govt = None;
        assert_eq!(
            answered(&govts, &npcs, 0, &hit),
            TAKES(n(2)),
            "two independents"
        );
    }

    #[test]
    fn a_ship_gliding_in_waiting_in_its_gate_or_bursting_out_answers_a_hit_as_a_flying_one() {
        // `_DamageShip` drops a reaction only while +0x50 is above 0, a
        // ship jumping out (@0x3b4b0-0x3b4c2, @0x3b97e-0x3b995,
        // @0x3bab7-0x3bae4); an arrival's +0x50 is -1.0 in its gate
        // (@0x89555) and -999.0 gliding or bursting (@0x82ece, @0x8f3c8).
        let govts = relations(0, 0);
        for mode in [
            Mode::JumpingIn { tick: 0 },
            Mode::Emerging { ticks_left: 4 },
            Mode::HeldAtGate { ticks_left: 4 },
            Mode::LeavingGate { tick: 1 },
            Mode::Coasting { ticks_left: 30 },
        ] {
            let mut npcs = [
                ship(1, Some(TRADERS), AiType::WimpyTrader, 0.0),
                ship(2, Some(PIRATES), AiType::Warship, 500.0),
                ship(3, Some(TRADERS), AiType::WimpyTrader, 900.0),
            ];
            npcs[0].mode = mode;
            assert_eq!(
                answered(&govts, &npcs, 0, &strike(n(1), n(2))),
                TAKES(n(2)),
                "{mode:?}"
            );
            assert_eq!(
                answered(&govts, &npcs, 0, &strike(n(1), ShipRef::Player)),
                TAKES(ShipRef::Player),
                "{mode:?}"
            );
            npcs[2].leader = Some(NpcId(1));
            assert_eq!(
                answered(&govts, &npcs, 0, &strike(n(3), n(2))),
                TAKES(n(2)),
                "{mode:?}: its escort hit"
            );
        }
    }

    #[test]
    fn a_ship_keeps_a_target_nearer_than_half_the_attackers_distance_ahead_of_it() {
        let govts = relations(0, 0);
        // The ship faces up; its target 50 above, the attacker 100 right.
        let mut hunter = ship(1, Some(POLICE), AiType::Warship, 0.0);
        hunter.goal = Goal::Attack(n(2));
        let mut quarry = ship(2, Some(PIRATES), AiType::Warship, 0.0);
        quarry.state.position = Vec2::new(0.0, -49.9);
        let attacker = ship(3, Some(PIRATES), AiType::Warship, 100.0);
        let npcs = [hunter, quarry, attacker];
        let hit = strike(n(1), n(3));
        assert_eq!(
            answered(&govts, &npcs, 0, &hit),
            Reaction::default(),
            "kept"
        );
        let mut farther = npcs.clone();
        farther[1].state.position = Vec2::new(0.0, -50.0);
        assert_eq!(answered(&govts, &farther, 0, &hit), TAKES(n(3)));
        // Near enough, but 45 degrees off the nose; then 1.
        let mut aside = npcs.clone();
        aside[1].state.position = Vec2::new(-30.0, -30.0);
        assert_eq!(answered(&govts, &aside, 0, &hit), TAKES(n(3)));
        aside[0].state.heading = 316.0;
        assert_eq!(answered(&govts, &aside, 0, &hit), Reaction::default());
        let mut gone = npcs.clone();
        gone[0].goal = Goal::Attack(n(9));
        assert_eq!(
            answered(&govts, &gone, 0, &hit),
            TAKES(n(3)),
            "its target gone"
        );
        let shift = |ships: &[Npc]| -> Vec<Npc> {
            ships
                .iter()
                .cloned()
                .map(|mut npc| {
                    npc.state.position = npc.state.position + Vec2::new(300.0, -200.0);
                    npc
                })
                .collect()
        };
        assert_eq!(
            answered(&govts, &shift(&npcs), 0, &hit),
            Reaction::default(),
            "measured from where it is"
        );
        assert_eq!(answered(&govts, &shift(&farther), 0, &hit), TAKES(n(3)));
        let mut by_its_target = npcs.clone();
        by_its_target[0].goal = Goal::Attack(n(3));
        assert_eq!(
            answered(&govts, &by_its_target, 0, &hit),
            TAKES(n(3)),
            "hit by the ship it attacks"
        );
        assert_eq!((FOCUS, FOCUS_ANGLE), (0.25, 44.0));
    }

    /// A Samaritan W of `govt` and `ai_type` answering the player's hit
    /// on a trader of `victim`.
    fn samaritan(
        govts: &Governments,
        w: Option<GovtId>,
        ai_type: AiType,
        victim: Option<GovtId>,
    ) -> Reaction {
        let npcs = [
            ship(1, victim, AiType::WimpyTrader, 0.0),
            ship(2, w, ai_type, 3000.0),
        ];
        answered(govts, &npcs, 1, &strike(n(1), ShipRef::Player))
    }

    const ANSWERS: Reaction = Reaction {
        target: Some(ShipRef::Player),
        provoked: 0.0,
        goal: Some(Goal::Attack(ShipRef::Player)),
    };

    #[test]
    fn allied_warships_and_interceptors_answer_the_players_attack_anywhere() {
        let govts = relations(0, 0);
        for ai_type in [AiType::Warship, AiType::Interceptor] {
            for w in [Some(POLICE), Some(TRADERS)] {
                assert_eq!(
                    samaritan(&govts, w, ai_type, Some(TRADERS)),
                    ANSWERS,
                    "{w:?} {ai_type:?}"
                );
            }
        }
        for ai_type in [AiType::WimpyTrader, AiType::BraveTrader] {
            assert_eq!(
                samaritan(&govts, Some(POLICE), ai_type, Some(TRADERS)),
                Reaction::default(),
                "{ai_type:?}"
            );
        }
    }

    #[test]
    fn no_samaritan_answers_the_players_unintended_hit() {
        let govts = relations(0, 0);
        let npcs = [
            ship(1, Some(TRADERS), AiType::WimpyTrader, 0.0),
            ship(2, Some(POLICE), AiType::Warship, 3000.0),
        ];
        let hit = strike(n(1), ShipRef::Player);
        assert_eq!(answered(&govts, &npcs, 1, &hit), ANSWERS, "intended");
        let stray = Strike {
            intended: false,
            ..hit
        };
        assert_eq!(
            answered(&govts, &npcs, 1, &stray),
            Reaction::default(),
            "a stray hit on a ship the player wasn't aiming at (@0x3bed2)"
        );
        assert_eq!(
            answered(&govts, &npcs, 0, &stray),
            Reaction {
                target: Some(ShipRef::Player),
                provoked: 12.0,
                goal: None,
            },
            "the victim still answers it"
        );
    }

    #[test]
    fn by_the_bible_only_interceptors_police_piracy() {
        let govts = relations(0, 0);
        let answer_of = |police, ai_type| {
            let npcs = [
                ship(1, Some(TRADERS), AiType::WimpyTrader, 0.0),
                ship(2, Some(POLICE), ai_type, 3000.0),
            ];
            answered_by(police, &govts, &npcs, 1, &strike(n(1), ShipRef::Player))
        };
        assert_eq!(answer_of(RuleSource::Engine, AiType::Warship), ANSWERS);
        assert_eq!(answer_of(RuleSource::Engine, AiType::Interceptor), ANSWERS);
        assert_eq!(answer_of(RuleSource::Bible, AiType::Interceptor), ANSWERS);
        assert_eq!(
            answer_of(RuleSource::Bible, AiType::Warship),
            Reaction::default()
        );
        // A warship hit itself still answers by the Bible.
        let npcs = [
            ship(1, Some(TRADERS), AiType::Warship, 0.0),
            ship(2, Some(PIRATES), AiType::Warship, 500.0),
        ];
        assert_eq!(
            answered_by(RuleSource::Bible, &govts, &npcs, 0, &strike(n(1), n(2))),
            TAKES(n(2))
        );
    }

    #[test]
    fn a_samaritan_already_attacking_or_hit_by_another_keeps_to_its_fight() {
        let govts = relations(0, 0);
        let mut npcs = [
            ship(1, Some(TRADERS), AiType::WimpyTrader, 0.0),
            ship(2, Some(POLICE), AiType::Warship, 3000.0),
        ];
        npcs[1].goal = Goal::Attack(n(9));
        let hit = strike(n(1), ShipRef::Player);
        assert_eq!(answered(&govts, &npcs, 1, &hit), Reaction::default());
        npcs[1].goal = Goal::Snipe(n(9));
        assert_eq!(answered(&govts, &npcs, 1, &hit), Reaction::default());
        npcs[1].goal = Goal::Flee(n(9));
        assert_eq!(
            answered(&govts, &npcs, 1, &hit),
            ANSWERS,
            "fleeing is no fight"
        );
        let by_npc = strike(n(1), n(3));
        npcs[1].goal = Goal::Idle;
        assert_eq!(
            answered(&govts, &npcs, 1, &by_npc),
            Reaction::default(),
            "only the player's attacks"
        );
    }

    #[test]
    fn no_samaritan_answers_for_an_enemy_a_xenophobe_a_derelict_or_the_ignored() {
        let govts = relations(0, 0);
        assert_eq!(
            samaritan(&govts, Some(PIRATES), AiType::Warship, Some(TRADERS)),
            Reaction::default(),
            "W at war with the victim"
        );
        assert_eq!(
            samaritan(&govts, Some(POLICE), AiType::Warship, Some(XENO)),
            Reaction::default(),
            "a xenophobic victim"
        );
        assert_eq!(
            samaritan(&govts, Some(XENO), AiType::Warship, Some(XENO)),
            ANSWERS,
            "its own xenophobes"
        );
        for flags in [DERELICT, IGNORED_WHEN_ATTACKED] {
            assert_eq!(
                samaritan(
                    &relations(flags, 0),
                    Some(POLICE),
                    AiType::Warship,
                    Some(TRADERS)
                ),
                Reaction::default(),
                "{flags:#x}"
            );
        }
    }

    #[test]
    fn a_neutral_answers_only_when_nosy() {
        assert_eq!(
            samaritan(
                &relations(0, 0),
                Some(NEUTRAL),
                AiType::Warship,
                Some(TRADERS)
            ),
            Reaction::default()
        );
        assert_eq!(
            samaritan(
                &relations(0, NOSY),
                Some(NEUTRAL),
                AiType::Warship,
                Some(TRADERS)
            ),
            ANSWERS
        );
    }

    #[test]
    fn an_independent_samaritan_minds_only_derelicts_the_ignored_and_xenophobes() {
        let govts = relations(0, 0);
        assert_eq!(
            samaritan(&govts, None, AiType::Warship, Some(TRADERS)),
            ANSWERS
        );
        assert_eq!(
            samaritan(&govts, None, AiType::Warship, Some(PIRATES)),
            ANSWERS,
            "nobody's enemy"
        );
        assert_eq!(
            samaritan(&govts, None, AiType::Warship, Some(XENO)),
            Reaction::default()
        );
        assert_eq!(
            samaritan(
                &relations(IGNORED_WHEN_ATTACKED, 0),
                None,
                AiType::Warship,
                Some(TRADERS)
            ),
            Reaction::default()
        );
    }

    #[test]
    fn for_an_independent_victim_only_the_nosy_answer() {
        assert_eq!(
            samaritan(&relations(0, 0), Some(POLICE), AiType::Warship, None),
            Reaction::default()
        );
        assert_eq!(
            samaritan(&relations(0, NOSY), Some(NEUTRAL), AiType::Warship, None),
            ANSWERS
        );
        assert_eq!(
            samaritan(&relations(0, NOSY), None, AiType::Warship, None),
            Reaction::default(),
            "an independent is not nosy"
        );
    }

    // The blessing.

    /// [`samaritan`] for a warship W, the player blessed by `blessing`,
    /// its strike of `damage`.
    fn samaritan_blessed(
        govts: &Governments,
        w: Option<GovtId>,
        victim: Option<GovtId>,
        blessing: crate::ai::Blessing,
        damage: f32,
    ) -> Reaction {
        let npcs = [
            ship(1, victim, AiType::WimpyTrader, 0.0),
            ship(2, w, AiType::Warship, 3000.0),
        ];
        let around = Surroundings {
            govts,
            player: Some(crate::ai::PlayerSide {
                blessing,
                ..crate::ai::fixture::player(0.0, -100.0)
            }),
            ..Surroundings::new(&[], &npcs)
        };
        let hit = Strike {
            damage,
            ..strike(n(1), ShipRef::Player)
        };
        answer(&npcs[1], &hit, &around, RuleSource::Engine)
    }

    #[test]
    fn a_blessing_government_is_no_good_samaritan() {
        let govts = relations(0, 0);
        let blessed = crate::ai::fixture::blessing_of(POLICE);
        let unblessed = crate::ai::Blessing::NONE;
        for (damage, case) in [(12.0, "a shot"), (0.0, "a boarding")] {
            assert_eq!(
                samaritan_blessed(&govts, Some(POLICE), Some(TRADERS), unblessed, damage),
                ANSWERS,
                "{case}: unblessed"
            );
            assert_eq!(
                samaritan_blessed(&govts, Some(POLICE), Some(TRADERS), blessed, damage),
                Reaction::default(),
                "{case}"
            );
            assert_eq!(
                samaritan_blessed(
                    &govts,
                    Some(POLICE),
                    Some(TRADERS),
                    crate::ai::fixture::blessing_of(TRADERS),
                    damage
                ),
                ANSWERS,
                "{case}: the victim's blessing is not W's"
            );
        }
    }

    #[test]
    fn a_nosy_blessing_warship_still_answers_an_attack_on_an_independent() {
        let govts = relations(0, NOSY);
        let blessed = crate::ai::fixture::blessing_of(NEUTRAL);
        assert_eq!(
            samaritan_blessed(&govts, Some(NEUTRAL), None, blessed, 12.0),
            ANSWERS
        );
        assert_eq!(
            samaritan_blessed(&govts, Some(NEUTRAL), Some(TRADERS), blessed, 12.0),
            Reaction::default(),
            "a governed victim"
        );
    }
}
