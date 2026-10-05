//! How a ship hit answers, and how others come to its help: Nova's
//! [`answer`], which the four AI types share.
//!
//! The rules are the original's (`_DamageShip` @0x3bb20-0x3bed2 and
//! `_DoGoodSamaritan` @0x3bef4 in the `EV Nova` executable):
//!
//! - **The victim**, and the lead of an escort hit, add the damage to
//!   their provocation and take the attacker as their target, unless the
//!   attacker is of their government or their fleet, they are still
//!   jumping in, or they already attack another ship nearer than half the
//!   attacker's distance ([`FOCUS`] of its distance squared) while facing
//!   within [`FOCUS_ANGLE`] of it: then the hit changes nothing.
//! - **A Good Samaritan**: for a hit by the player, every warship or
//!   interceptor W not already attacking takes the player as its target
//!   and attacks at once, unless W is an NPC enemy of the victim, or the
//!   victim's government is xenophobic and not W's; the victim's
//!   government is derelict or ignored when attacked (`Flags` 0x0020); or
//!   W is neither allied with the victim nor nosy (`Flags` 0x0002). For
//!   an independent W, only the derelict, 0x0020 and xenophobe tests
//!   apply; for an independent victim, only a nosy W answers. There is
//!   no limit on range.
//!
//! So Federation police answer the player's attack on a Civvies trader,
//! since the Civvies are Federation allies. The original's distress
//! message (`_AICallForHelp`) only tells the player, and waits for the
//! comm messages; its reinforcements (`_AICallForReinforcements`) are
//! not modelled yet.

use crate::ai::{Goal, Reaction, Surroundings};
use crate::combat::aim::{angle_off, bearing};
use crate::combat::{ShipRef, Strike};
use crate::govt::{DERELICT, IGNORED_WHEN_ATTACKED, NOSY};
use crate::traffic::npc::{Mode, Npc};

/// The share of the attacker's distance squared within which a ship
/// keeps attacking its own target (0.25 @0x3bc3b).
pub const FOCUS: f32 = 0.25;
/// How far off its nose, in degrees, a ship keeping its own target must
/// have it.
pub const FOCUS_ANGLE: f32 = 44.0;

/// How `npc` answers `strike` among `around` (see the module docs).
#[must_use]
pub fn answer(npc: &Npc, strike: &Strike, around: &Surroundings) -> Reaction {
    let me = ShipRef::Npc(npc.id);
    let escort_hit = match strike.ship {
        ShipRef::Npc(id) => around.npc(id).is_some_and(|hit| hit.leader == Some(npc.id)),
        ShipRef::Player => false,
    };
    if strike.ship == me || escort_hit {
        return if ignores(npc, strike.by, around) {
            Reaction::default()
        } else {
            Reaction {
                target: Some(strike.by),
                provoked: strike.damage,
                goal: None,
            }
        };
    }
    if strike.by == ShipRef::Player && good_samaritan(npc, strike.ship, around) {
        return Reaction {
            target: Some(ShipRef::Player),
            provoked: 0.0,
            goal: Some(Goal::Attack(ShipRef::Player)),
        };
    }
    Reaction::default()
}

/// Whether `npc`, hit (or its escort hit) by `attacker`, lets it be: the
/// attacker is of its government or fleet, it is still jumping in, or it
/// keeps to its own nearer target ahead of it.
fn ignores(npc: &Npc, attacker: ShipRef, around: &Surroundings) -> bool {
    let kin = match attacker {
        ShipRef::Player => false,
        ShipRef::Npc(id) => around.npc(id).is_some_and(|other| {
            (npc.govt.is_some() && other.govt == npc.govt) || other.fleet() == npc.fleet()
        }),
    };
    if kin || npc.mode != Mode::Flying {
        return true;
    }
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
    use crate::stats::ShipStats;
    use crate::testkit::{FAST, govt};
    use crate::traffic::npc::{AiType, NpcId};

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
        let mut npc = crate::testkit::npc(id, ShipStats::new(FAST, &[]));
        npc.govt = govt;
        npc.ai_type = ai_type;
        npc.state.position = Vec2::new(x, 0.0);
        npc
    }

    fn n(id: u32) -> ShipRef {
        ShipRef::Npc(NpcId(id))
    }

    fn strike(ship: ShipRef, by: ShipRef) -> Strike {
        Strike {
            ship,
            by,
            damage: 12.0,
            downed: None,
        }
    }

    fn answered(govts: &Governments, npcs: &[Npc], who: usize, hit: &Strike) -> Reaction {
        let around = Surroundings {
            govts,
            ..Surroundings::new(&[], npcs)
        };
        answer(&npcs[who], hit, &around)
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
    fn a_hit_by_its_own_government_or_fleet_or_while_jumping_in_is_ignored() {
        let govts = relations(0, 0);
        let mut npcs = [
            ship(1, Some(TRADERS), AiType::WimpyTrader, 0.0),
            ship(2, Some(TRADERS), AiType::Warship, 500.0),
        ];
        let hit = strike(n(1), n(2));
        assert_eq!(answered(&govts, &npcs, 0, &hit), Reaction::default());
        npcs[1].govt = Some(NEUTRAL);
        npcs[1].leader = Some(NpcId(1));
        assert_eq!(
            answered(&govts, &npcs, 0, &hit),
            Reaction::default(),
            "its escort"
        );
        npcs[1].leader = None;
        npcs[0].mode = Mode::JumpingIn { ticks_left: 4 };
        assert_eq!(answered(&govts, &npcs, 0, &hit), Reaction::default());
        npcs[0].mode = Mode::Flying;
        npcs[0].govt = None;
        npcs[1].govt = None;
        assert_eq!(
            answered(&govts, &npcs, 0, &hit),
            TAKES(n(2)),
            "two independents are no government"
        );
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
}
