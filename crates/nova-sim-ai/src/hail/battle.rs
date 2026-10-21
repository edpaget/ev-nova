//! Request Assistance while an NPC threatens the player (`_DoCommDialog`
//! @0x96310-0x96514 in the `EV Nova` executable; see [`nova`](super::nova)'s
//! module docs, item 4), and the two routines that send a helper at a
//! threat: `_AIFightEnemyThreatToPlayer` ([`enemy_threat_pick`]) and
//! `_AIFightThreatToPlayer` ([`threat_pick`]).

use super::nova::mood_reply;
use super::reply::{COMEDIAN, IN_YOUR_DREAMS, NO_MONEY, ON_MY_WAY, PAY_ME_FIRST, RATHER_NOT};
use super::{Answer, Ask, Deed, Defence, Hail, Reply};
use crate::chance::Chance;
use crate::rulebook::RuleSource;
use crate::traffic::npc::{AiType, NpcId};

/// Request Assistance while the player is threatened (see the module
/// docs), `target` being [`RuleKey::BattleHelpTarget`]'s reading,
/// `brave_trader` [`RuleKey::BraveTraderBattleHelp`]'s and `gone`
/// [`RuleKey::BattleHelpGoneSlots`]'s, picking on `chance`.
///
/// [`RuleKey::BattleHelpTarget`]: crate::RuleKey::BattleHelpTarget
/// [`RuleKey::BraveTraderBattleHelp`]: crate::RuleKey::BraveTraderBattleHelp
/// [`RuleKey::BattleHelpGoneSlots`]: crate::RuleKey::BattleHelpGoneSlots
pub(super) fn defend(
    hail: &Hail,
    target: RuleSource,
    brave_trader: RuleSource,
    gone: RuleSource,
    chance: &mut dyn Chance,
) -> Answer {
    if !hail.threatened_by_enemy {
        return Answer::say(Reply::Comm(IN_YOUR_DREAMS));
    }
    if hail.dispositions.battle_help {
        return on_my_way(Defence {
            threat: enemy_threat_pick(hail, target, gone, chance),
            reinforce: true,
            drop_travel: false,
        });
    }
    let npc = hail.npc;
    if hail.escort_ai() || (npc.no_ai && !npc.spared) {
        return on_my_way(Defence {
            threat: threat_pick(hail, chance),
            reinforce: false,
            drop_travel: true,
        });
    }
    let helps = match npc.ai_type {
        AiType::WimpyTrader => false,
        AiType::BraveTrader => {
            brave_trader == RuleSource::Engine
                && npc.govt.is_none()
                && npc.berth.is_multiple_of(BRAVE_TRADER_SLOTS)
        }
        AiType::Warship | AiType::Interceptor => true,
    };
    if !helps {
        return Answer::say(Reply::Comm(RATHER_NOT));
    }
    let defence = Defence {
        threat: enemy_threat_pick(hail, target, gone, chance),
        reinforce: false,
        drop_travel: false,
    };
    Answer {
        ask: Some(Ask {
            paid: (Reply::Comm(ON_MY_WAY), Some(Deed::Defend(defence))),
            declined: (Reply::Comm(COMEDIAN), None),
            short: Reply::Comm(NO_MONEY),
        }),
        ..Answer::say(Reply::Comm(mood_reply(hail.mood, PAY_ME_FIRST)))
    }
}

/// A brave trader helps for a price only when its ship-table slot
/// divides by this (`imul 0x5556` @0x963a9-0x963cb).
pub const BRAVE_TRADER_SLOTS: u32 = 3;

/// "Okay, I'm on my way.", and it fights as `defence` says, at once and
/// for nothing.
fn on_my_way(defence: Defence) -> Answer {
    Answer {
        deed: Some(Deed::Defend(defence)),
        ..Answer::say(Reply::Comm(ON_MY_WAY))
    }
}

/// The threat `_AIFightEnemyThreatToPlayer` (@0x82c80-0x82dba) sends the
/// ship hailed at, picked on `chance`, by `rule`, the reading of
/// [`RuleKey::BattleHelpTarget`](crate::RuleKey::BattleHelpTarget). It
/// counts the slots that hold a ship other than the helper, not disabled
/// or dying, targeting the player (+0x70 == 0) in AI state 3 or 4, in the
/// helper's system (@0x82c9f-0x82d11), then draws `Rand(63)+1` until a
/// slot passes the same tests (@0x82d24-0x82d99): a uniform pick among
/// [`Hail::threats_to_player`], here one draw below their count. It reads
/// no government, so by the engine any threat is picked, enemy of the
/// helper or not; by the other reading only one whose government
/// [`Governments::npc_enemies`](crate::govt::Governments::npc_enemies)
/// finds an enemy of the helper's, as its name says. With none, nothing
/// is drawn and there is no threat (+0x70 = -1, @0x82daa).
///
/// It reads no active flag (+0xb8), so the original also counts a slot
/// whose ship left with those fields intact: a hostile person swapped for
/// its mission ship on the hail (@0x62520), whose record the traffic keeps
/// at its berth ([`traffic::vacated`](crate::traffic::vacated)). By
/// `gone`, the reading of
/// [`RuleKey::BattleHelpGoneSlots`](crate::RuleKey::BattleHelpGoneSlots),
/// the engine counts those [`Hail::vacated_threats`] too, under the same
/// government reading; by the other, only ships in the system. A helper
/// sent at one says "Okay, I'm on my way." and lets it go at its next
/// think, as no ship in the system answers to it.
pub(crate) fn enemy_threat_pick(
    hail: &Hail,
    rule: RuleSource,
    gone: RuleSource,
    chance: &mut dyn Chance,
) -> Option<NpcId> {
    let me = hail.npc.govt;
    let vacated = hail
        .vacated_threats()
        .filter(|_| gone == RuleSource::Engine);
    let threats: Vec<NpcId> = hail
        .threats_to_player()
        .chain(vacated)
        .filter(|other| rule == RuleSource::Engine || hail.govts.npc_enemies(me, other.govt))
        .map(|npc| npc.id)
        .collect();
    pick(&threats, chance)
}

/// The threat `_AIFightThreatToPlayer` (@0x8183c-0x8196b) sends the ship
/// hailed at, picked on `chance`. It counts the slots 1-63 (@0x81847,
/// @0x818c1) other than the helper (@0x81858), not `_IsDisabled`
/// (@0x81879) or `_IsDying` (@0x8188f), in the helper's system
/// (@0x818a1-0x818a9), that `_IsThreatToPlayer` holds for (@0x818ae,
/// [`Npc::threatens_player_among`](crate::traffic::npc::Npc::threatens_player_among),
/// a ship attacking one the player leads among them), then draws
/// `Rand(63)+1` until a slot passes (@0x818d2-0x81947): a uniform pick,
/// here one draw below their count, whatever any government. With none
/// it clears the target (+0x70 = -1, @0x8195e) and nothing is drawn.
pub(crate) fn threat_pick(hail: &Hail, chance: &mut dyn Chance) -> Option<NpcId> {
    let me = hail.npc.id;
    let threats: Vec<NpcId> = hail
        .npcs
        .iter()
        .filter(|other| other.id != me && other.threatens_player_among(hail.npcs))
        .map(|npc| npc.id)
        .collect();
    pick(&threats, chance)
}

/// One of `ships`, picked uniformly on `chance`; none, drawing nothing,
/// from none.
fn pick(ships: &[NpcId], chance: &mut dyn Chance) -> Option<NpcId> {
    let count = u32::try_from(ships.len()).unwrap_or(u32::MAX);
    if count == 0 {
        return None;
    }
    ships.get(chance.below(count) as usize).copied()
}

#[cfg(test)]
mod tests {
    use super::super::fixture::{hail, ship};
    use super::super::reply::{BAD_MOOD, COMEDIAN, GOOD_MOOD, NO_MONEY, PAY_ME_FIRST};
    use super::super::{Ask, Dispositions, HailOption, RequestAssistance};
    use super::*;
    use crate::ai::Goal;
    use crate::catalog::{GovtId, GovtRecord};
    use crate::combat::ShipRef;
    use crate::combat::hull::Condition;
    use crate::govt::Governments;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::traffic::npc::{AiType, Npc};
    use nova_sim_base::fixture::Draws;
    use nova_sim_catalog::fixture::govt;

    const ENGINE: RuleSource = RuleSource::Engine;
    const BIBLE: RuleSource = RuleSource::Bible;

    /// 140 (class 1, enemies with class 2), 142 (class 2) and 143 (class
    /// 3).
    fn rivals() -> Governments {
        Governments::new([
            GovtRecord {
                classes: [1, -1, -1, -1],
                enemies: [2, -1, -1, -1],
                ..govt(140)
            },
            GovtRecord {
                classes: [2, -1, -1, -1],
                ..govt(142)
            },
            GovtRecord {
                classes: [3, -1, -1, -1],
                ..govt(143)
            },
        ])
    }

    /// NPC `id` of `govt`, flying `goal`.
    fn flying(id: u32, govt: Option<i16>, goal: Goal) -> Npc {
        let mut npc = crate::traffic::fixture::npc(id, crate::stats::ShipStats::default());
        npc.govt = govt.map(GovtId);
        npc.goal = goal;
        npc
    }

    /// NPC 2, of 142 (an enemy of 140), attacking the player.
    fn pirate() -> Npc {
        flying(2, Some(142), Goal::Attack(ShipRef::Player))
    }

    /// NPC 1 of 140, of `ai_type`, idle.
    fn helper(ai_type: AiType) -> Npc {
        let mut npc = ship();
        npc.govt = Some(GovtId(140));
        npc.ai_type = ai_type;
        npc
    }

    /// A hail of `npc` among `npcs` and `govts`, an enemy threatening the
    /// player, of no battle help.
    fn threatened<'a>(npc: &'a Npc, npcs: &'a [Npc], govts: &'a Governments) -> Hail<'a> {
        Hail {
            player_threatened: true,
            threatened_by_enemy: true,
            npcs,
            govts,
            ..hail(npc)
        }
    }

    /// How `request` answers `hail`, drawing `draws`, and the draws asked.
    fn pressed(request: RequestAssistance, hail: &Hail, draws: &[u32]) -> (Answer, Vec<u32>) {
        let mut chance = Draws::of(draws);
        let answer = request.press(hail, &mut chance);
        (answer, chance.asked)
    }

    /// Request Assistance by the engine.
    fn engine() -> RequestAssistance {
        RequestAssistance::default()
    }

    fn says(group: u8) -> Answer {
        Answer::say(Reply::Comm(group))
    }

    /// A defence of `threat` that neither calls for reinforcements nor
    /// drops the travel stellar: a paid helper's.
    fn paid(threat: NpcId) -> Deed {
        Deed::Defend(Defence {
            threat: Some(threat),
            reinforce: false,
            drop_travel: false,
        })
    }

    /// The price a helper asks to fight `threat`, in the mood of `group`.
    fn price(group: u8, threat: NpcId) -> Answer {
        Answer {
            first: None,
            reply: Some(Reply::Comm(group)),
            deed: None,
            ask: Some(Ask {
                paid: (Reply::Comm(ON_MY_WAY), Some(paid(threat))),
                declined: (Reply::Comm(COMEDIAN), None),
                short: Reply::Comm(NO_MONEY),
            }),
        }
    }

    #[test]
    fn a_warship_or_interceptor_asks_its_price_in_its_mood_to_fight_a_threat() {
        // @0x96465-0x964ee: the mood line 0x17, 0x18 or 0x12, then
        // `_DoShipCommPayment`; paid, 0x1d and `_AIFightEnemyThreatToPlayer`
        // with no reinforcement call; declined 0x1f; short 0x0c.
        let govts = rivals();
        let npcs = [pirate()];
        for ai_type in [AiType::Warship, AiType::Interceptor] {
            let npc = helper(ai_type);
            for (mood, group) in [
                (0.79, GOOD_MOOD),
                (0.8, PAY_ME_FIRST),
                (1.19, PAY_ME_FIRST),
                (1.2, BAD_MOOD),
            ] {
                let hail = Hail {
                    mood,
                    ..threatened(&npc, &npcs, &govts)
                };
                let (answer, asked) = pressed(engine(), &hail, &[0]);
                assert_eq!(answer, price(group, NpcId(2)), "{ai_type:?} {mood}");
                assert_eq!(asked, [1], "the pick, at the press");
            }
        }
    }

    #[test]
    fn a_free_helping_warship_still_asks_its_price() {
        // `_DoShipCommPayment` reads no free help (-0x148).
        let govts = rivals();
        let npcs = [pirate()];
        let npc = helper(AiType::Warship);
        let hail = Hail {
            dispositions: Dispositions {
                free_help: true,
                ..Dispositions::default()
            },
            ..threatened(&npc, &npcs, &govts)
        };
        assert_eq!(
            pressed(engine(), &hail, &[0]).0,
            price(PAY_ME_FIRST, NpcId(2))
        );
    }

    /// A brave trader at `berth`, of `govt`.
    fn brave(berth: u32, govt: Option<i16>) -> Npc {
        let mut npc = helper(AiType::BraveTrader);
        npc.berth = berth;
        npc.govt = govt.map(GovtId);
        npc
    }

    #[test]
    fn an_independent_brave_trader_whose_slot_divides_by_three_asks_its_price_by_the_engine() {
        // @0x963a9-0x963d9: +0x86 divisible by 3 and +0x98 -1.
        let govts = rivals();
        let npcs = [pirate()];
        for berth in [3, 6, 63] {
            let npc = brave(berth, None);
            let (answer, asked) = pressed(engine(), &threatened(&npc, &npcs, &govts), &[0]);
            assert_eq!(answer, price(PAY_ME_FIRST, NpcId(2)), "{berth}");
            assert_eq!(asked, [1]);
        }
        for (berth, govt) in [(4, None), (5, None), (3, Some(140)), (6, Some(143))] {
            let npc = brave(berth, govt);
            let (answer, asked) = pressed(engine(), &threatened(&npc, &npcs, &govts), &[0]);
            assert_eq!(answer, says(RATHER_NOT), "{berth} {govt:?}");
            assert!(asked.is_empty(), "{berth} {govt:?}");
        }
    }

    #[test]
    fn by_the_other_reading_no_brave_trader_helps() {
        let govts = rivals();
        let npcs = [pirate()];
        let bible = RequestAssistance {
            brave_trader_help: BIBLE,
            ..engine()
        };
        let npc = brave(3, None);
        let (answer, asked) = pressed(bible, &threatened(&npc, &npcs, &govts), &[0]);
        assert_eq!(answer, says(RATHER_NOT));
        assert!(asked.is_empty());
    }

    #[test]
    fn a_wimpy_trader_would_rather_not_and_draws_nothing() {
        let govts = rivals();
        let npcs = [pirate()];
        for berth in [3, 4] {
            let mut npc = brave(berth, None);
            npc.ai_type = AiType::WimpyTrader;
            let (answer, asked) = pressed(engine(), &threatened(&npc, &npcs, &govts), &[]);
            assert_eq!(answer, says(RATHER_NOT));
            assert!(asked.is_empty());
        }
    }

    /// A defence of `threat` by `_AIFightThreatToPlayer`: no call, the
    /// travel stellar dropped.
    fn free(threat: Option<NpcId>) -> Answer {
        Answer {
            deed: Some(Deed::Defend(Defence {
                threat,
                reinforce: false,
                drop_travel: true,
            })),
            ..says(ON_MY_WAY)
        }
    }

    /// Ships the original reads as of AI type 0 or above 4: one of no AI,
    /// a mission escort, a fleet escort and an NPC's carried fighter.
    fn escorts() -> Vec<Npc> {
        let mut no_ai = helper(AiType::Warship);
        no_ai.no_ai = true;
        let mut mission = helper(AiType::WimpyTrader);
        mission.mission = Some(crate::traffic::npc::MissionMark {
            slot: 0,
            behav: crate::traffic::ShipBehav::EscortPlayer,
            formation: Some(crate::escort::FormationSlot::default()),
            order: None,
        });
        let mut fleet = helper(AiType::WimpyTrader);
        fleet.leader = Some(NpcId(9));
        let mut fighter = helper(AiType::BraveTrader);
        fighter.carrier = Some(crate::bay::Carrier {
            ship: ShipRef::Npc(NpcId(9)),
            window: 100.0,
            reach: 40.0,
        });
        vec![no_ai, mission, fleet, fighter]
    }

    #[test]
    fn a_ship_of_ai_type_zero_or_above_four_fights_any_threat_for_nothing() {
        // @0x964f8-0x9650f: 0x1d and `_AIFightThreatToPlayer`, no price.
        let govts = rivals();
        // The threat: a ship of a government that is no enemy, attacking
        // the player.
        let npcs = [flying(3, Some(143), Goal::Attack(ShipRef::Player))];
        for npc in escorts() {
            let hail = threatened(&npc, &npcs, &govts);
            let (answer, asked) = pressed(engine(), &hail, &[0]);
            assert_eq!(answer, free(Some(NpcId(3))), "{npc:?}");
            assert_eq!(asked, [1]);
            let none = threatened(&npc, &[], &govts);
            let (answer, asked) = pressed(engine(), &none, &[]);
            assert_eq!(answer, free(None), "no threat to pick");
            assert!(asked.is_empty());
        }
    }

    #[test]
    fn a_spared_fleet_escort_reads_as_the_wimpy_trader_the_bribe_made_it() {
        // The bribe writes AI type 1 (@0x961f8).
        let govts = rivals();
        let npcs = [pirate()];
        for mut npc in escorts() {
            npc.spared = true;
            npc.ai_type = AiType::WimpyTrader;
            let (answer, _) = pressed(engine(), &threatened(&npc, &npcs, &govts), &[0]);
            assert_eq!(answer, says(RATHER_NOT), "{npc:?}");
        }
    }

    #[test]
    fn rank_battle_help_with_no_threat_to_pick_still_calls_for_reinforcements() {
        // @0x9636d follows the pick whatever it found.
        let govts = rivals();
        let npc = helper(AiType::Warship);
        let hail = Hail {
            dispositions: Dispositions {
                battle_help: true,
                ..Dispositions::default()
            },
            ..threatened(&npc, &[], &govts)
        };
        let (answer, asked) = pressed(engine(), &hail, &[]);
        assert_eq!(
            answer,
            Answer {
                deed: Some(Deed::Defend(Defence {
                    threat: None,
                    reinforce: true,
                    drop_travel: false,
                })),
                ..says(ON_MY_WAY)
            }
        );
        assert!(asked.is_empty());
    }

    #[test]
    fn rank_battle_help_by_the_other_reading_skips_a_threat_of_no_enemy() {
        let govts = rivals();
        let npcs = [
            flying(3, Some(143), Goal::Attack(ShipRef::Player)),
            pirate(),
        ];
        let npc = helper(AiType::WimpyTrader);
        let hail = Hail {
            dispositions: Dispositions {
                battle_help: true,
                ..Dispositions::default()
            },
            ..threatened(&npc, &npcs, &govts)
        };
        let bible = RequestAssistance {
            battle_help_target: BIBLE,
            ..engine()
        };
        let (answer, asked) = pressed(bible, &hail, &[0]);
        assert_eq!(
            answer.deed,
            Some(Deed::Defend(Defence {
                threat: Some(NpcId(2)),
                reinforce: true,
                drop_travel: false,
            }))
        );
        assert_eq!(asked, [1]);
        let (answer, asked) = pressed(engine(), &hail, &[0]);
        assert_eq!(
            answer.deed,
            Some(Deed::Defend(Defence {
                threat: Some(NpcId(3)),
                reinforce: true,
                drop_travel: false,
            }))
        );
        assert_eq!(asked, [2]);
    }

    #[test]
    fn request_assistance_reads_all_three_battle_keys_from_the_rulebook() {
        assert_eq!(
            RequestAssistance::from_rulebook(&Rulebook::default()),
            engine()
        );
        let rulebook = Rulebook::default()
            .with_override(RuleKey::BattleHelpTarget, BIBLE)
            .with_override(RuleKey::BraveTraderBattleHelp, BIBLE)
            .with_override(RuleKey::BattleHelpGoneSlots, BIBLE);
        let request = RequestAssistance::from_rulebook(&rulebook);
        assert_eq!(request.battle_help_target, BIBLE);
        assert_eq!(request.brave_trader_help, BIBLE);
        assert_eq!(request.battle_help_gone, BIBLE);
        assert_eq!(request.quiet_hails, ENGINE);
        let only_gone = Rulebook::default().with_override(RuleKey::BattleHelpGoneSlots, BIBLE);
        let request = RequestAssistance::from_rulebook(&only_gone);
        assert_eq!(request.battle_help_gone, BIBLE);
        assert_eq!(request.battle_help_target, ENGINE);
        assert_eq!(request.brave_trader_help, ENGINE);
    }

    // The target picks.

    /// What `pick` picks among `npcs` for NPC 1 of 140, drawing `draws`,
    /// and the draws asked.
    fn picked(
        npcs: &[Npc],
        draws: &[u32],
        pick: impl Fn(&Hail, &mut dyn Chance) -> Option<NpcId>,
    ) -> (Option<NpcId>, Vec<u32>) {
        let govts = rivals();
        let npc = helper(AiType::Warship);
        let mut chance = Draws::of(draws);
        let picked = pick(&threatened(&npc, npcs, &govts), &mut chance);
        (picked, chance.asked)
    }

    /// `_AIFightEnemyThreatToPlayer`'s pick under `rule`, counting the
    /// vacated berths by the engine.
    fn enemy(rule: RuleSource) -> impl Fn(&Hail, &mut dyn Chance) -> Option<NpcId> {
        enemy_counting(rule, ENGINE)
    }

    /// `_AIFightEnemyThreatToPlayer`'s pick under `rule`, counting the
    /// vacated berths by `gone`.
    fn enemy_counting(
        rule: RuleSource,
        gone: RuleSource,
    ) -> impl Fn(&Hail, &mut dyn Chance) -> Option<NpcId> {
        move |hail, chance| enemy_threat_pick(hail, rule, gone, chance)
    }

    /// What `pick` picks among `npcs` and the vacated berths' last ships
    /// `vacated` for NPC 1 of 140, drawing `draws`, and the draws asked.
    fn picked_among(
        npcs: &[Npc],
        vacated: &[Npc],
        draws: &[u32],
        pick: impl Fn(&Hail, &mut dyn Chance) -> Option<NpcId>,
    ) -> (Option<NpcId>, Vec<u32>) {
        let govts = rivals();
        let npc = helper(AiType::Warship);
        let mut chance = Draws::of(draws);
        let hail = Hail {
            vacated,
            ..threatened(&npc, npcs, &govts)
        };
        let picked = pick(&hail, &mut chance);
        (picked, chance.asked)
    }

    /// NPC 7 of 142 (an enemy of 140), attacking the player, the last
    /// ship of berth 5: a hostile person swapped for its mission ship.
    fn swapped_out() -> Npc {
        let mut npc = flying(7, Some(142), Goal::Attack(ShipRef::Player));
        npc.target = Some(ShipRef::Player);
        npc.berth = 5;
        npc
    }

    #[test]
    fn by_the_engine_the_enemy_pick_counts_and_picks_a_swapped_out_hostile_person() {
        // @0x82c80-0x82dba test no active flag (+0xb8), and the swap
        // clears only that (@0x62520).
        let npcs = [pirate()];
        let vacated = [swapped_out()];
        let gone = enemy_counting(ENGINE, ENGINE);
        assert_eq!(
            picked_among(&npcs, &vacated, &[1], &gone),
            (Some(NpcId(7)), vec![2])
        );
        assert_eq!(
            picked_among(&npcs, &vacated, &[0], &gone),
            (Some(NpcId(2)), vec![2])
        );
        assert_eq!(
            picked_among(&[], &vacated, &[0], &gone),
            (Some(NpcId(7)), vec![1]),
            "alone"
        );
    }

    #[test]
    fn by_the_other_reading_of_battle_help_gone_slots_only_ships_in_the_system_are_counted() {
        let npcs = [pirate()];
        let vacated = [swapped_out()];
        for rule in [ENGINE, BIBLE] {
            assert_eq!(
                picked_among(&npcs, &vacated, &[0], enemy_counting(rule, BIBLE)),
                (Some(NpcId(2)), vec![1]),
                "{rule:?}"
            );
            assert_eq!(
                picked_among(&[], &vacated, &[], enemy_counting(rule, BIBLE)),
                (None, vec![]),
                "{rule:?}"
            );
        }
    }

    #[test]
    fn a_vacated_ship_that_does_not_threaten_the_player_is_not_counted() {
        // +0x70 == 0 and AI state 3 or 4 (@0x82d77-0x82d8c), not
        // `_IsDisabled` (@0x82d5a) or `_IsDying` (@0x82d6a).
        let npcs = [pirate()];
        let leaving = flying(7, Some(142), Goal::JumpOut);
        let on_an_npc = flying(7, Some(142), Goal::Attack(ShipRef::Npc(NpcId(3))));
        let mut disabled = swapped_out();
        disabled.condition = Condition::Disabled;
        let mut dying = swapped_out();
        dying.condition = Condition::Dying {
            ticks_left: 0,
            starting: true,
        };
        for npc in [leaving, on_an_npc, disabled, dying] {
            let vacated = [Npc { berth: 5, ..npc }];
            assert_eq!(
                picked_among(&npcs, &vacated, &[0], enemy(ENGINE)),
                (Some(NpcId(2)), vec![1]),
                "{:?} {:?}",
                vacated[0].goal,
                vacated[0].condition
            );
        }
    }

    #[test]
    fn a_vacated_berth_held_again_is_not_counted() {
        // A ship taking the slot rewrites +0x70 and +0xc8c8; the traffic
        // drops the record then, and the pick holds to it too.
        let mut taken = pirate();
        taken.berth = 5;
        let npcs = [taken];
        let vacated = [swapped_out()];
        assert_eq!(
            picked_among(&npcs, &vacated, &[0], enemy(ENGINE)),
            (Some(NpcId(2)), vec![1]),
            "berth 5 held by NPC 2"
        );
        // The helper's own slot (@0x82d36-0x82d48): the hailed helper is
        // NPC 1 at berth 0's fixture berth.
        let mut at_helpers = swapped_out();
        at_helpers.berth = helper(AiType::Warship).berth;
        assert_eq!(
            picked_among(&[pirate()], &[at_helpers], &[0], enemy(ENGINE)),
            (Some(NpcId(2)), vec![1]),
            "the helper's berth"
        );
    }

    #[test]
    fn under_battle_help_target_bible_a_vacated_ship_of_no_enemy_government_is_not_counted() {
        let npcs = [pirate()];
        let mut friend = swapped_out();
        friend.govt = Some(GovtId(143));
        assert_eq!(
            picked_among(
                &npcs,
                &[friend.clone()],
                &[0],
                enemy_counting(BIBLE, ENGINE)
            ),
            (Some(NpcId(2)), vec![1])
        );
        assert_eq!(
            picked_among(&npcs, &[friend], &[1], enemy_counting(ENGINE, ENGINE)),
            (Some(NpcId(7)), vec![2]),
            "by the engine any government"
        );
    }

    #[test]
    fn the_free_pick_never_counts_a_vacated_ship() {
        // `_AIFightThreatToPlayer` asks `_IsThreatToPlayer` (@0x7f501),
        // which tests +0xb8.
        let npcs = [pirate()];
        assert_eq!(
            picked_among(&npcs, &[swapped_out()], &[0], threat_pick),
            (Some(NpcId(2)), vec![1])
        );
    }

    #[test]
    fn a_vacated_threat_alone_does_not_threaten_the_player() {
        // `_IsPlayerThreatened` (@0x8245c) and `_IsPlayerThreatenedByMyEnemy`
        // (@0x824a0) test +0xb8.
        let govts = rivals();
        let npcs = [helper(AiType::Warship)];
        let around = crate::ai::Surroundings {
            govts: &govts,
            ..crate::ai::Surroundings::new(&[], &npcs)
        };
        let talk = crate::hail::Conversation::open(
            100_000,
            false,
            0xC000,
            &mut Draws::of(&[3, 40, 1, 1, 0, 1]),
        );
        let vacated = [swapped_out()];
        let hail = Hail {
            vacated: &vacated,
            ..Hail::new(
                &npcs[0],
                &around,
                Dispositions::default(),
                &talk,
                None,
                None,
            )
        };
        assert!(!hail.player_threatened);
        assert!(!hail.threatened_by_enemy);
        let (_, asked) = pressed(engine(), &hail, &[]);
        assert!(asked.is_empty(), "nothing drawn");
    }

    #[test]
    fn rank_battle_help_sent_at_a_vacated_ship_still_calls_for_reinforcements() {
        // @0x9636d follows the pick whatever it found.
        let govts = rivals();
        let npc = helper(AiType::Warship);
        let vacated = [swapped_out()];
        let hail = Hail {
            dispositions: Dispositions {
                battle_help: true,
                ..Dispositions::default()
            },
            vacated: &vacated,
            ..threatened(&npc, &[], &govts)
        };
        let (answer, asked) = pressed(engine(), &hail, &[0]);
        assert_eq!(
            answer,
            Answer {
                deed: Some(Deed::Defend(Defence {
                    threat: Some(NpcId(7)),
                    reinforce: true,
                    drop_travel: false,
                })),
                ..says(ON_MY_WAY)
            }
        );
        assert_eq!(asked, [1]);
    }

    #[test]
    fn by_the_engine_the_enemy_pick_takes_any_threat_enemy_or_not() {
        // @0x82c9f-0x82d99 read no government.
        let npcs = [
            pirate(),
            flying(3, Some(143), Goal::Attack(ShipRef::Player)),
            flying(4, Some(142), Goal::Idle),
        ];
        assert_eq!(
            picked(&npcs, &[0], enemy(ENGINE)),
            (Some(NpcId(2)), vec![2])
        );
        assert_eq!(
            picked(&npcs, &[1], enemy(ENGINE)),
            (Some(NpcId(3)), vec![2])
        );
    }

    #[test]
    fn by_the_other_reading_the_enemy_pick_takes_only_an_enemy_of_the_helpers() {
        let npcs = [
            flying(3, Some(143), Goal::Attack(ShipRef::Player)),
            pirate(),
            flying(4, None, Goal::Attack(ShipRef::Player)),
        ];
        assert_eq!(picked(&npcs, &[0], enemy(BIBLE)), (Some(NpcId(2)), vec![1]));
        let none = [flying(3, Some(143), Goal::Attack(ShipRef::Player))];
        assert_eq!(picked(&none, &[], enemy(BIBLE)), (None, vec![]));
    }

    /// NPC `id`, a mission escort of the player's.
    fn led(id: u32) -> Npc {
        let mut npc = flying(id, Some(143), Goal::Idle);
        npc.mission = Some(crate::traffic::npc::MissionMark {
            slot: 0,
            behav: crate::traffic::ShipBehav::EscortPlayer,
            formation: Some(crate::escort::FormationSlot::default()),
            order: None,
        });
        npc
    }

    #[test]
    fn the_free_pick_takes_a_ship_attacking_one_the_player_leads_and_the_enemy_pick_does_not() {
        // `_IsThreatToPlayer` (@0x7f572-0x7f590) against the inline +0x70
        // == 0 test (@0x82cda).
        let npcs = [
            led(4),
            flying(5, Some(142), Goal::Attack(ShipRef::Npc(NpcId(4)))),
        ];
        assert_eq!(picked(&npcs, &[0], threat_pick), (Some(NpcId(5)), vec![1]));
        for rule in [ENGINE, BIBLE] {
            assert_eq!(picked(&npcs, &[], enemy(rule)), (None, vec![]));
        }
    }

    #[test]
    fn the_free_pick_takes_any_threat_whatever_its_government() {
        let npcs = [
            flying(3, Some(143), Goal::Attack(ShipRef::Player)),
            flying(4, None, Goal::Snipe(ShipRef::Player)),
            flying(5, Some(142), Goal::Idle),
        ];
        assert_eq!(picked(&npcs, &[1], threat_pick), (Some(NpcId(4)), vec![2]));
    }

    #[test]
    fn neither_pick_takes_the_helper() {
        let mut me = helper(AiType::Warship);
        me.goal = Goal::Attack(ShipRef::Player);
        let npcs = [me, flying(3, Some(142), Goal::Attack(ShipRef::Player))];
        assert_eq!(picked(&npcs, &[0], threat_pick), (Some(NpcId(3)), vec![1]));
        for rule in [ENGINE, BIBLE] {
            assert_eq!(picked(&npcs, &[0], enemy(rule)), (Some(NpcId(3)), vec![1]));
        }
        let alone = [npcs[0].clone()];
        assert_eq!(picked(&alone, &[], threat_pick), (None, vec![]));
        assert_eq!(picked(&alone, &[], enemy(ENGINE)), (None, vec![]));
    }
}
