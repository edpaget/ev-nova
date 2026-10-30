//! The ship a pirate plunders first: `_AIFindShipToPlunder`
//! (@0x7fd10-0x7feb5 in the `EV Nova` executable), which
//! [`pirate`](super::pirate) asks before any other target ([`find`]).
//!
//! Over the player and the NPCs in slot order (the player's slot 0 first),
//! a candidate is one that is:
//!
//! - not the pirate itself (@0x7fd3d), nor a ship it leads (+0x9a is the
//!   pirate, @0x7fd50), a fighter it launched among them;
//! - in the system (+0xb8, @0x7fd5d), with no visibility test;
//! - **not boarded** (+0xb9, @0x7fd6a, [`Npc::boarded`]): the player too,
//!   once a pirate has docked with it
//!   ([`PlayerSide::boarded`](crate::ai::PlayerSide::boarded));
//! - **disabled or breaking up** (`_IsDisabled` @0x7fd7a holds of both);
//! - not a mission's special ship (+0xc8d2, @0x7fd90);
//! - one a pirate takes on: the player (@0x7fd9e), whatever its class; a
//!   former escort of the player's (+0xc8de, @0x7fda3); an NPC of AI type
//!   2 or below (@0x7fdad); any other only when its class's `InherentAI`
//!   is 2 or below, or the pirate holds a destroying weapon
//!   ([`fire::destroys`], @0x7fd23, @0x7fdb7-0x7fdde). An NPC carrier's
//!   fighter reads as the original's AI type 5, any other ship with a
//!   leader as 6, and a ship the player paid off as 1, its leader kept
//!   ([`Npc::original_ai_type`]);
//! - of a class with a crew: `Crew` above none (@0x7fe04,
//!   [`HullSpec::crew`](crate::combat::hull::HullSpec::crew)), so never
//!   the escape pod;
//! - not an ally (@0x7fe0f-0x7fe38): a ship not led by the player whose
//!   government `_GovtAllies` the pirate's, its own included, is passed
//!   over; one the player leads is a candidate whatever its government,
//!   and the player itself, of no government, is never an ally.
//!
//! Of those it takes the nearest by `_XYFloatSquaredDist` truncated to a
//! whole number (`cvttss2si` @0x7fe5c), strictly nearer (`setl` @0x7fe64),
//! so a tie goes to the earlier slot and the player wins a tie with any
//! NPC. That is the float square truncated once, not the per-axis
//! truncation [`candidates::nearest`](super::candidates::nearest) reads.
//! The original sets the target and goal 0xd itself (@0x7fe91-0x7fea5);
//! here [`pirate`](super::pirate) does. It turns no ship on the player
//! (no `_AIMakeShipAttackPlayer`) and plays no sound.

use crate::ai::{Surroundings, fire};
use crate::combat::ShipRef;
use crate::geometry::Vec2;
use crate::traffic::npc::Npc;

/// The ship `npc`, a pirate, plunders first among `around`, if any (see
/// the module docs).
#[must_use]
pub fn find(npc: &Npc, around: &Surroundings) -> Option<ShipRef> {
    let destroys = fire::destroys(npc);
    let player = around
        .player
        .filter(|player| !player.boarded && player.condition.disabled() && player.hull.crew > 0)
        .map(|player| (ShipRef::Player, player.state.position));
    let npcs = around
        .by_slot()
        .into_iter()
        .filter(|other| other.id != npc.id && candidate(npc, other, around, destroys))
        .map(|other| (ShipRef::Npc(other.id), other.state.position));
    let mut nearest: Option<(ShipRef, i64)> = None;
    for (ship, at) in player.into_iter().chain(npcs) {
        let distance = squared_whole(at - npc.state.position);
        if nearest.is_none_or(|(_, best)| distance < best) {
            nearest = Some((ship, distance));
        }
    }
    nearest.map(|(ship, _)| ship)
}

/// Whether `other`, an NPC other than `npc`, is a candidate for `npc` to
/// plunder among `around`, `destroys` saying whether `npc` holds a
/// destroying weapon (see the module docs).
fn candidate(npc: &Npc, other: &Npc, around: &Surroundings, destroys: bool) -> bool {
    let led_by_player = other.leader_ref() == Some(ShipRef::Player);
    let takes_on = other.former.is_some()
        || other.original_ai_type() <= 2
        || other.hull.inherent_ai <= 2
        || destroys;
    other.leader_ref() != Some(ShipRef::Npc(npc.id))
        && other.condition.active()
        && !other.boarded
        && other.condition.disabled()
        && other.mission.is_none()
        && takes_on
        && other.hull.crew > 0
        && (led_by_player || !around.govts.allies(npc.govt, other.govt))
}

/// `off`'s square, as a float, truncated to a whole number
/// (`_XYFloatSquaredDist`, then `cvttss2si`).
fn squared_whole(off: Vec2) -> i64 {
    let squared = off.x * off.x + off.y * off.y;
    // The square of a distance in a system fits an i64 whole.
    #[allow(clippy::cast_possible_truncation)]
    let whole = squared as i64;
    whole
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::fixture::{PIRATES, POLICE, TRADERS, around, govts, n, player, ship, sites};
    use crate::catalog::{ShipId, WeaponRecord};
    use crate::combat::armament::Armament;
    use crate::combat::hull::Condition;
    use crate::combat::weapon::WeaponSpec;
    use crate::fleet::Escort;
    use crate::reserves::Reserves;
    use crate::self_repair::FormerEscort;
    use crate::traffic::npc::{AiType, MissionMark, NpcId};
    use nova_sim_catalog::fixture::weapon;

    const P: ShipRef = ShipRef::Player;

    /// The pirate, NPC 1 at the centre in berth 1.
    fn pirate() -> Npc {
        let mut npc = ship(1, PIRATES, AiType::Warship, 0.0, 0.0);
        npc.berth = 1;
        npc
    }

    /// A disabled trader, NPC `id` in berth `id`, `y` below the centre,
    /// of a class with a crew of 1 and `InherentAI` 2.
    fn wreck(id: u32, y: f32) -> Npc {
        let mut npc = ship(id, TRADERS, AiType::WimpyTrader, 0.0, y);
        npc.berth = id;
        npc.condition = Condition::Disabled;
        npc.hull.crew = 1;
        npc.hull.inherent_ai = 2;
        npc
    }

    /// The pirate's pick among `npcs` (it first), the player at `at` as
    /// `condition` with a crew of `crew`, if `at` is some.
    fn found_with(npcs: &[Npc], at: Option<((f32, f32), Condition, i16)>) -> Option<ShipRef> {
        let sites = sites();
        let govts = govts(0, 0);
        let mut around = around(&sites, npcs, &govts, (0.0, 0.0), 0);
        around.player = at.map(|((x, y), condition, crew)| {
            let mut side = player(x, y);
            side.condition = condition;
            side.hull.crew = crew;
            side
        });
        find(&npcs[0], &around)
    }

    /// The pirate's pick among `npcs`, no player in the system.
    fn found(npcs: &[Npc]) -> Option<ShipRef> {
        found_with(npcs, None)
    }

    #[test]
    fn a_pirate_picks_the_nearest_disabled_ship() {
        assert_eq!(
            found(&[pirate(), wreck(2, 300.0), wreck(3, 200.0)]),
            Some(n(3))
        );
        assert_eq!(
            found(&[pirate(), wreck(2, -200.0), wreck(3, 300.0)]),
            Some(n(2))
        );
        assert_eq!(found(&[pirate()]), None, "nothing to plunder");
    }

    #[test]
    fn a_tie_goes_to_the_earlier_slot_and_the_player_wins_any() {
        let mut later = wreck(2, 200.0);
        later.berth = 5;
        let earlier = Npc {
            berth: 4,
            ..wreck(3, -200.0)
        };
        assert_eq!(
            found(&[pirate(), later.clone(), earlier.clone()]),
            Some(n(3))
        );
        let downed = Some(((200.0, 0.0), Condition::Disabled, 3));
        assert_eq!(found_with(&[pirate(), later, earlier], downed), Some(P));
    }

    #[test]
    fn the_distance_is_the_float_square_truncated_once() {
        // 0.9 on both axes squares to 1.62, truncated to 1, a tie with
        // 1.0 on one axis; truncating each axis first would make it 0.
        let mut diagonal = wreck(2, 0.9);
        diagonal.state.position.x = 0.9;
        diagonal.berth = 3;
        let mut straight = wreck(3, 1.0);
        straight.berth = 2;
        assert_eq!(
            found(&[pirate(), diagonal.clone(), straight.clone()]),
            Some(n(3))
        );
        // Nearer than a whole square: 0.7 squared twice is 0.98, so 0.
        diagonal.state.position = Vec2::new(0.7, 0.7);
        assert_eq!(found(&[pirate(), diagonal, straight]), Some(n(2)));
    }

    #[test]
    fn the_distance_is_measured_from_the_pirate() {
        let mut away = pirate();
        away.state.position = Vec2::new(0.0, 500.0);
        // 400 and 100 from the pirate; 100 and 400 from the centre.
        assert_eq!(found(&[away, wreck(2, 100.0), wreck(3, 400.0)]), Some(n(3)));
    }

    #[test]
    fn an_intact_ship_is_no_candidate_and_one_breaking_up_is() {
        let mut intact = wreck(2, 100.0);
        intact.condition = Condition::Intact;
        assert_eq!(found(&[pirate(), intact.clone()]), None);
        intact.condition = Condition::Dying {
            ticks_left: 3,
            starting: false,
        };
        assert_eq!(
            found(&[pirate(), intact.clone()]),
            Some(n(2)),
            "`_IsDisabled` holds"
        );
        intact.condition = Condition::Destroyed;
        assert_eq!(found(&[pirate(), intact]), None, "its slot is free");
    }

    #[test]
    fn a_boarded_or_mission_ship_is_passed_over() {
        let mut boarded = wreck(2, 100.0);
        boarded.boarded = true;
        assert_eq!(found(&[pirate(), boarded, wreck(3, 300.0)]), Some(n(3)));
        let mut marked = wreck(2, 100.0);
        marked.mission = Some(MissionMark {
            slot: 0,
            behav: crate::traffic::ShipBehav::Own,
            formation: None,
            order: None,
        });
        assert_eq!(found(&[pirate(), marked, wreck(3, 300.0)]), Some(n(3)));
    }

    #[test]
    fn a_ship_the_pirate_leads_is_passed_over_and_one_another_leads_is_not() {
        let mut own = wreck(2, 100.0);
        own.leader = Some(NpcId(1));
        own.ai_type = AiType::WimpyTrader;
        assert_eq!(found(&[pirate(), own.clone()]), None);
        own.leader = Some(NpcId(7));
        assert_eq!(
            found(&[pirate(), own]),
            Some(n(2)),
            "a trader class, led elsewhere"
        );
    }

    #[test]
    fn a_class_needs_a_crew_above_none() {
        for (crew, picked) in [(-1, None), (0, None), (1, Some(n(2)))] {
            let mut crewed = wreck(2, 100.0);
            crewed.hull.crew = crew;
            assert_eq!(found(&[pirate(), crewed]), picked, "{crew}");
        }
    }

    #[test]
    fn the_player_is_a_candidate_whatever_its_class_with_a_crew_disabled() {
        let at = (100.0, 0.0);
        let pick = |condition, crew| found_with(&[pirate()], Some((at, condition, crew)));
        assert_eq!(pick(Condition::Disabled, 1), Some(P));
        assert_eq!(
            pick(
                Condition::Dying {
                    ticks_left: 2,
                    starting: false
                },
                1
            ),
            Some(P)
        );
        assert_eq!(pick(Condition::Intact, 1), None);
        assert_eq!(pick(Condition::Disabled, 0), None, "the escape pod's crew");
        assert_eq!(pick(Condition::Disabled, -1), None);
        let sites = sites();
        let govts = govts(0, 0);
        let npcs = [pirate(), wreck(2, 300.0)];
        let mut around = around(&sites, &npcs, &govts, (0.0, 0.0), 0);
        let mut boarded = player(at.0, at.1);
        boarded.condition = Condition::Disabled;
        boarded.hull.crew = 1;
        boarded.boarded = true;
        around.player = Some(boarded);
        assert_eq!(
            find(&npcs[0], &around),
            Some(n(2)),
            "a pirate docked with it"
        );
    }

    #[test]
    fn a_warship_typed_ship_needs_a_trader_class_or_the_pirates_destroying_weapons() {
        let typed = |ai_type, inherent_ai| {
            let mut npc = wreck(2, 100.0);
            npc.ai_type = ai_type;
            npc.hull.inherent_ai = inherent_ai;
            npc
        };
        assert_eq!(
            found(&[pirate(), typed(AiType::BraveTrader, 4)]),
            Some(n(2)),
            "AI 2"
        );
        assert_eq!(found(&[pirate(), typed(AiType::Warship, 2)]), Some(n(2)));
        assert_eq!(found(&[pirate(), typed(AiType::Warship, 3)]), None);
        assert_eq!(found(&[pirate(), typed(AiType::Interceptor, 3)]), None);
        let mut armed = pirate();
        armed.armament = Armament::new([(
            WeaponSpec::new(&WeaponRecord {
                mass_dmg: 1,
                ..weapon(128)
            }),
            1,
        )]);
        assert_eq!(found(&[armed, typed(AiType::Warship, 3)]), Some(n(2)));
    }

    #[test]
    fn a_ship_with_no_ai_reads_as_no_type_above_a_trader_whatever_its_class() {
        let mut hulk = wreck(2, 100.0);
        hulk.ai_type = AiType::Warship;
        hulk.hull.inherent_ai = 3;
        hulk.no_ai = true;
        assert_eq!(
            found(&[pirate(), hulk]),
            Some(n(2)),
            "AI type -1 is not above 2 (@0x7fdad)"
        );
    }

    /// A former escort record, of ship 128.
    fn former() -> FormerEscort {
        FormerEscort {
            record: Escort {
                ship: ShipId(128),
                reserves: Reserves::full(1.0, 1.0, 1.0),
                order: None,
                carried: false,
                wage: None,
                person: None,
                mark: None,
            },
        }
    }

    #[test]
    fn a_former_escort_is_a_candidate_whatever_its_class() {
        let mut escort = wreck(2, 100.0);
        escort.ai_type = AiType::Warship;
        escort.hull.inherent_ai = 3;
        assert_eq!(found(&[pirate(), escort.clone()]), None);
        escort.former = Some(former());
        assert_eq!(found(&[pirate(), escort.clone()]), Some(n(2)));
        escort.hull.crew = 0;
        assert_eq!(found(&[pirate(), escort]), None, "it still needs a crew");
    }

    #[test]
    fn a_led_or_carried_ship_reads_as_no_trader() {
        let mut fleet = wreck(2, 100.0);
        fleet.hull.inherent_ai = 3;
        fleet.leader = Some(NpcId(7));
        assert_eq!(found(&[pirate(), fleet.clone()]), None, "AI type 6");
        let mut fighter = wreck(2, 100.0);
        fighter.hull.inherent_ai = 3;
        fighter.carrier = Some(crate::bay::Carrier {
            ship: n(7),
            window: 100.0,
            reach: 40.0,
        });
        assert_eq!(found(&[pirate(), fighter.clone()]), None, "AI type 5");
        fighter.carrier = None;
        assert_eq!(
            found(&[pirate(), fighter]),
            Some(n(2)),
            "a wimpy trader again"
        );
    }

    #[test]
    fn a_led_ship_paid_off_then_disabled_reads_as_the_bribes_trader() {
        // The bribe writes 1 (@0x961f8), which the disabled reset leaves
        // alone, its class's 3 notwithstanding.
        let mut spared = wreck(2, 100.0);
        spared.hull.inherent_ai = 3;
        spared.leader = Some(NpcId(7));
        spared.take_bribe();
        spared.condition = Condition::Disabled;
        crate::traffic::disabled_reset::reset(&mut spared, crate::rulebook::RuleSource::Engine);
        assert_eq!(spared.ai_type, AiType::WimpyTrader, "the bribe's word");
        assert_eq!(found(&[pirate(), spared]), Some(n(2)), "AI type 1");
    }

    #[test]
    fn an_ally_is_passed_over_unless_the_player_leads_it() {
        // The traders are no ally of the pirates; a pirate of its own
        // government is, and so is its police's ally to a police pirate.
        let mut kin = wreck(2, 100.0);
        kin.govt = Some(PIRATES);
        assert_eq!(found(&[pirate(), kin.clone()]), None);
        let mut police = pirate();
        police.govt = Some(POLICE);
        assert_eq!(found(&[police.clone(), wreck(2, 100.0)]), None, "allied");
        let mut led = wreck(2, 100.0);
        led.escort = Some(crate::escort::EscortDuty {
            formation: crate::escort::FormationSlot::default(),
            order: None,
        });
        led.hull.inherent_ai = 2;
        assert_eq!(found(&[police, led]), Some(n(2)), "the player leads it");
        kin.govt = None;
        assert_eq!(
            found(&[pirate(), kin]),
            Some(n(2)),
            "an independent is no ally"
        );
    }
}
