//! An active mission's special ships: where they are ([`ShipSystem`],
//! its `ShipSyst` resolved at accept by [`select_ship_system`]) and the
//! state its slot keeps of them ([`SpecialShips`]). How they spawn is the
//! [`traffic::spawn`](crate::traffic::spawn) module's "Mission ships".

use crate::catalog::{CommCatalog, GovtId, MissionShip, SystemId};
use crate::chance::Chance;
use crate::govt::Governments;
use crate::hyperspace::StarMap;

use super::offer::Stop;

/// Where an active mission's special ships are: its `ShipSyst` as
/// `_LoadCurrentMissionData` resolves it into the slot (+0x10) at accept
/// (@0xa0f67-0xa1142).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ShipSystem {
    /// In this system.
    At(SystemId),
    /// -6: in whatever system the player is in.
    FollowsPlayer,
    /// Nowhere: no system was found, or the code names none.
    #[default]
    Nowhere,
}

impl ShipSystem {
    /// Whether the ships are in `system`.
    #[must_use]
    pub fn holds(self, system: SystemId) -> bool {
        match self {
            Self::At(at) => at == system,
            Self::FollowsPlayer => true,
            Self::Nowhere => false,
        }
    }
}

/// What an active mission's slot keeps of its special ships: by default
/// none, nowhere, with no timer running.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpecialShips {
    /// Where they are (+0x10).
    pub system: ShipSystem,
    /// How many are left to spawn or still alive (+0x6): its `ShipCount`
    /// at accept, one fewer for each destroyed.
    pub left: i16,
    /// How many of those that come in later have come since the timer
    /// last restarted (+0x2e).
    pub arrived: i16,
    /// The appear timer those that come in later wait on (+0x4e).
    pub appear: i16,
    /// Their name, drawn at accept (+0x70): what `<SN>` reads, and what
    /// they are called while it is not empty.
    pub name: String,
    /// Their subtitle, drawn at accept (+0xb0), shown while it is not
    /// empty.
    pub subtitle: String,
}

impl Default for SpecialShips {
    fn default() -> Self {
        Self {
            system: ShipSystem::Nowhere,
            left: 0,
            arrived: 0,
            appear: NOT_APPEARING,
            name: String::new(),
            subtitle: String::new(),
        }
    }
}

/// `ShipStart`: the ships come in later, on the appear timer.
pub const COMES_LATER: i16 = 1;
/// `ShipStart`: the ships spawn cloaked.
pub const CLOAKED: i16 = 2;
/// `ShipBehav`: the ships escort the player.
pub const ESCORTS_PLAYER: i16 = 1;
/// `ShipGoal`: escort the ships.
pub const ESCORT_GOAL: i16 = 3;
/// The appear timer of escorts the player is to escort (30 @0xa0eec).
pub const ESCORT_APPEAR: i16 = 30;
/// `Rand(100) + 100`: any other appear timer.
pub const APPEAR_DRAW: u32 = 100;
/// The appear timer's value when it does not run (-1 @0xa0f09).
pub const NOT_APPEARING: i16 = -1;

/// The appear timer `ships` starts, at accept and at each restart:
/// [`ESCORT_APPEAR`] for ships that escort the player with `ShipGoal`
/// 3, else `Rand(100) + 100`, drawn (@0xa0ede-0xa0f03, @0x99f5b-0x99f9a).
pub fn appear_timer(ships: &MissionShip, chance: &mut dyn Chance) -> i16 {
    if ships.behav == ESCORTS_PLAYER && ships.goal == ESCORT_GOAL {
        ESCORT_APPEAR
    } else {
        i16::try_from(chance.below(APPEAR_DRAW) + APPEAR_DRAW).unwrap_or(i16::MAX)
    }
}

/// String `Rand(count) + 1` of `STR#` `list` in `strings`, drawn; empty,
/// drawing nothing, with no list or an empty one (@0xa1195-0xa1273).
fn drawn_string(list: Option<i16>, strings: &dyn CommCatalog, chance: &mut dyn Chance) -> String {
    let Some(list) = list else {
        return String::new();
    };
    let strings = strings.string_list(list);
    if strings.is_empty() {
        return String::new();
    }
    let n = u32::try_from(strings.len()).unwrap_or(u32::MAX);
    let at = usize::try_from(chance.below(n)).unwrap_or(0);
    strings.into_iter().nth(at).unwrap_or_default()
}

impl SpecialShips {
    /// The ships of a mission whose `mïsn` gives `ships`, as
    /// `_LoadCurrentMissionData` loads them at accept (@0xa0a38), drawing
    /// on `chance` in its order: the appear timer, for a `ShipStart` above
    /// 0 (see [`appear_timer`]), else none (@0xa0ed4-0xa0f09); their
    /// system ([`select_ship_system`] from `facts` over `map` and
    /// `govts`); their name from their `ShipNameID` and their subtitle
    /// from their `ShipSubtitle`, each string `Rand(count) + 1` of its
    /// `STR#` in `strings`, or empty (@0xa1195-0xa1358). Ships left are
    /// their `ShipCount`, and they have all arrived but for a `ShipStart`
    /// of 1 (@0xa0b39-0xa0b46, @0xa1468-0xa147d).
    pub fn at_accept(
        ships: &MissionShip,
        facts: &ShipFacts,
        (map, govts): (&StarMap, &Governments),
        strings: &dyn CommCatalog,
        chance: &mut dyn Chance,
    ) -> Self {
        let appear = if ships.start > 0 {
            appear_timer(ships, chance)
        } else {
            NOT_APPEARING
        };
        let system = select_ship_system(ships.syst, facts, map, govts, chance);
        let name = drawn_string(ships.name_id, strings, chance);
        let subtitle = drawn_string(ships.subtitle, strings, chance);
        Self {
            system,
            left: ships.count,
            arrived: if ships.start == COMES_LATER {
                0
            } else {
                ships.count
            },
            appear,
            name,
            subtitle,
        }
    }
}

/// What a `ShipSyst` code is resolved from at accept: the player's
/// system then, and the mission's stops as its offer drew them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShipFacts {
    /// The player's system as the mission is accepted.
    pub player_system: SystemId,
    /// Where it goes.
    pub destination: Option<Stop>,
    /// Where it is completed.
    pub return_to: Option<Stop>,
}

/// The systems a random `ShipSyst` code draws from, beside the tests
/// every one but -5 makes (see [`select_ship_system`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    /// -2: any (@0x9b9a7-0x9ba6b).
    Any,
    /// 9999-14999: governed by this government, `None` for 9999's
    /// independents (@0x9bae1-0x9bbcf).
    Govt(Option<GovtId>),
    /// 15000-19999: governed, by another government allied with this one
    /// (@0x9bbd4-0x9bd08).
    Ally(GovtId),
    /// 20000-24999: not governed by this government, an independent
    /// included (@0x9bd0d-0x9bdfb).
    NotGovt(GovtId),
    /// 25000-29999: governed by an enemy of this government
    /// (@0x9be00-0x9bf02).
    Enemy(GovtId),
    /// 30000-30999: governed by a government sharing a class with this
    /// one, its own included (@0x9bf07-0x9c009).
    SharedClass(GovtId),
    /// 31000-31999: governed by none sharing a class with this one, an
    /// independent included (@0x9c00e-0x9c111).
    NoSharedClass(GovtId),
}

impl Kind {
    /// Whether a system governed by `govt` is of this kind, the relations
    /// as `govts` say.
    fn takes(self, govt: Option<GovtId>, govts: &Governments) -> bool {
        match self {
            Self::Any => true,
            Self::Govt(of) => govt == of,
            Self::Ally(of) => govt.is_some_and(|govt| govt != of) && govts.allies(govt, Some(of)),
            Self::NotGovt(of) => govt != Some(of),
            Self::Enemy(of) => govts.enemies(govt, Some(of)),
            Self::SharedClass(of) => govts.shared_class(govt, Some(of)),
            Self::NoSharedClass(of) => !govts.shared_class(govt, Some(of)),
        }
    }
}

/// The `ShipSyst` -6: the ships follow the player.
const FOLLOWS_PLAYER: i16 = -6;

/// The government a code's index from `base` names: index + 128, none
/// below 0.
fn govt_from(code: i16, base: i16) -> Option<GovtId> {
    let index = code - base;
    (index >= 0).then(|| GovtId(index + 128))
}

/// Where `code`, a `ShipSyst`, puts a mission's ships, as
/// `_LoadCurrentMissionData` resolves it at accept (@0xa0f67-0xa1142),
/// from `facts`, over the visible systems of `map` and the relations of
/// `govts`, drawing on `chance`:
///
/// - -1: the player's system.
/// - -3: the destination's system, or with no destination the return's;
///   -4: the return's. Nowhere when there is none.
/// - -6: wherever the player is.
/// - 128-9998: that system.
/// - -2 and 9999-31999: one visible system not at the player's place
///   (not together with it), of the code's kind, drawn
///   (`_SelectMissionSystem` @0x9b993); nowhere when there is none. The
///   original redraws `Rand(2048)` until it hits one, which comes to one
///   draw over them by ascending ID; where it would draw forever, this is
///   nowhere.
/// - -5: one of the systems the player's system links to, drawn
///   (@0x9ba70-0x9bac7); nowhere with none.
/// - 32000 and above: the player's system, as `_SelectMissionSystem`
///   gives back its second argument for a code it does not read
///   (@0x9c012).
/// - Any other code: nowhere.
pub fn select_ship_system(
    code: i16,
    facts: &ShipFacts,
    map: &StarMap,
    govts: &Governments,
    chance: &mut dyn Chance,
) -> ShipSystem {
    let system_of = |stop: Option<Stop>| stop.and_then(|stop| stop.system);
    let kind = match code {
        -1 | 32_000.. => return ShipSystem::At(facts.player_system),
        -3 => {
            let stop = facts.destination.or(facts.return_to);
            return system_of(stop).map_or(ShipSystem::Nowhere, ShipSystem::At);
        }
        -4 => return system_of(facts.return_to).map_or(ShipSystem::Nowhere, ShipSystem::At),
        FOLLOWS_PLAYER => return ShipSystem::FollowsPlayer,
        -5 => return drawn(map.listed_links(facts.player_system), chance),
        128..=9998 => return ShipSystem::At(SystemId(code)),
        -2 => Kind::Any,
        9999..=14_999 => Kind::Govt(govt_from(code, 10_000)),
        15_000..=19_999 => Kind::Ally(GovtId(code - 15_000 + 128)),
        20_000..=24_999 => Kind::NotGovt(GovtId(code - 20_000 + 128)),
        25_000..=29_999 => Kind::Enemy(GovtId(code - 25_000 + 128)),
        30_000..=30_999 => Kind::SharedClass(GovtId(code - 30_000 + 128)),
        31_000..=31_999 => Kind::NoSharedClass(GovtId(code - 31_000 + 128)),
        _ => return ShipSystem::Nowhere,
    };
    let places = map.places();
    let candidates: Vec<SystemId> = map
        .systems()
        .filter(|&system| {
            !places.together(system, facts.player_system) && kind.takes(map.govt(system), govts)
        })
        .collect();
    drawn(&candidates, chance)
}

/// One of `systems`, drawn once over them all; nowhere, drawing nothing,
/// when there is none.
fn drawn(systems: &[SystemId], chance: &mut dyn Chance) -> ShipSystem {
    if systems.is_empty() {
        return ShipSystem::Nowhere;
    }
    let n = u32::try_from(systems.len()).unwrap_or(u32::MAX);
    let at = usize::try_from(chance.below(n)).unwrap_or(usize::MAX);
    systems
        .get(at)
        .copied()
        .map_or(ShipSystem::Nowhere, ShipSystem::At)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{GovtRecord, StarSystem, StellarId};
    use crate::testkit::{Draws, govt};

    const FED: GovtId = GovtId(128);
    const CIVVIES: GovtId = GovtId(129);
    const PIRATES: GovtId = GovtId(130);
    const NEUTRAL: GovtId = GovtId(131);

    /// The Federation (class 1, allied with 13, at war with 9), the
    /// Civvies (classes 1 and 13), the pirates (class 9) and a
    /// neutral (class 4).
    fn govts() -> Governments {
        Governments::new([
            GovtRecord {
                classes: [1, -1, -1, -1],
                allies: [13, -1, -1, -1],
                enemies: [9, -1, -1, -1],
                ..govt(128)
            },
            GovtRecord {
                classes: [1, 13, -1, -1],
                ..govt(129)
            },
            GovtRecord {
                classes: [9, -1, -1, -1],
                ..govt(130)
            },
            GovtRecord {
                classes: [4, -1, -1, -1],
                ..govt(131)
            },
        ])
    }

    fn system(id: i16, at: (f32, f32), links: &[i16], govt: Option<GovtId>) -> StarSystem {
        StarSystem {
            govt,
            ..crate::testkit::star(id, at, links)
        }
    }

    /// The player is in 130 (Federation), linked to 131 and 133; 131 is
    /// the Civvies', 132 the pirates', 133 independent, 134 the
    /// neutral's, 135 the Federation's but at 130's place, 136 the
    /// Federation's elsewhere, and 137 the pirates' but hidden.
    fn map() -> StarMap {
        StarMap::with_hidden(
            vec![
                system(130, (0.0, 0.0), &[131, 133], Some(FED)),
                system(131, (100.0, 0.0), &[], Some(CIVVIES)),
                system(132, (200.0, 0.0), &[], Some(PIRATES)),
                system(133, (300.0, 0.0), &[], None),
                system(134, (400.0, 0.0), &[], Some(NEUTRAL)),
                system(135, (0.0, 0.0), &[], Some(FED)),
                system(136, (500.0, 0.0), &[], Some(FED)),
                system(137, (600.0, 0.0), &[], Some(PIRATES)),
            ],
            &[SystemId(137)].into(),
        )
    }

    fn stop(stellar: i16, system: Option<i16>) -> Stop {
        Stop {
            stellar: StellarId(stellar),
            system: system.map(SystemId),
        }
    }

    fn facts() -> ShipFacts {
        ShipFacts {
            player_system: SystemId(130),
            destination: Some(stop(150, Some(132))),
            return_to: Some(stop(151, Some(134))),
        }
    }

    /// What `code` resolves to from [`facts`], drawing `draws`, and the
    /// bounds asked.
    fn resolved_with(code: i16, facts: &ShipFacts, draws: &[u32]) -> (ShipSystem, Vec<u32>) {
        let mut chance = Draws::of(draws);
        let resolved = select_ship_system(code, facts, &map(), &govts(), &mut chance);
        (resolved, chance.asked)
    }

    fn resolved(code: i16, draws: &[u32]) -> (ShipSystem, Vec<u32>) {
        resolved_with(code, &facts(), draws)
    }

    fn at(id: i16) -> ShipSystem {
        ShipSystem::At(SystemId(id))
    }

    #[test]
    fn the_fixed_codes_draw_nothing() {
        let none: Vec<u32> = Vec::new();
        assert_eq!(resolved(-1, &[]), (at(130), none.clone()), "the player's");
        assert_eq!(
            resolved(-3, &[]),
            (at(132), none.clone()),
            "the destination's"
        );
        assert_eq!(resolved(-4, &[]), (at(134), none.clone()), "the return's");
        assert_eq!(resolved(-6, &[]), (ShipSystem::FollowsPlayer, none.clone()));
        assert_eq!(resolved(128, &[]), (at(128), none.clone()));
        assert_eq!(resolved(9998, &[]), (at(9998), none.clone()), "unbounded");
        for code in [32_000, i16::MAX] {
            assert_eq!(resolved(code, &[]), (at(130), none.clone()), "{code}");
        }
        for code in [0, 1, 127, -7, i16::MIN] {
            assert_eq!(
                resolved(code, &[]),
                (ShipSystem::Nowhere, none.clone()),
                "{code}"
            );
        }
    }

    #[test]
    fn minus_3_takes_the_return_with_no_destination_and_both_are_nowhere_without_a_system() {
        let returning = ShipFacts {
            destination: None,
            ..facts()
        };
        assert_eq!(resolved_with(-3, &returning, &[]).0, at(134));
        let nowhere = ShipFacts {
            destination: None,
            return_to: None,
            ..facts()
        };
        assert_eq!(resolved_with(-3, &nowhere, &[]).0, ShipSystem::Nowhere);
        assert_eq!(resolved_with(-4, &nowhere, &[]).0, ShipSystem::Nowhere);
        let hidden = ShipFacts {
            destination: Some(stop(150, None)),
            return_to: Some(stop(151, None)),
            ..facts()
        };
        assert_eq!(resolved_with(-3, &hidden, &[]).0, ShipSystem::Nowhere);
        assert_eq!(resolved_with(-4, &hidden, &[]).0, ShipSystem::Nowhere);
    }

    #[test]
    fn minus_2_draws_any_visible_system_away_from_the_players_place() {
        // 131, 132, 133, 134, 136: not 130 nor 135 at its place, nor the
        // hidden 137.
        assert_eq!(resolved(-2, &[0]), (at(131), vec![5]));
        assert_eq!(resolved(-2, &[4]), (at(136), vec![5]));
    }

    #[test]
    fn minus_5_draws_a_system_the_players_system_links_to() {
        assert_eq!(resolved(-5, &[0]), (at(131), vec![2]));
        assert_eq!(resolved(-5, &[1]), (at(133), vec![2]));
        let alone = ShipFacts {
            player_system: SystemId(136),
            ..facts()
        };
        assert_eq!(
            resolved_with(-5, &alone, &[]),
            (ShipSystem::Nowhere, vec![])
        );
    }

    /// Every system `code` may draw, in order, with nothing drawn when
    /// there is none.
    fn pool(code: i16) -> Vec<i16> {
        let (first, asked) = resolved(code, &[0]);
        let Some(&n) = asked.first() else {
            assert_eq!(first, ShipSystem::Nowhere, "{code}");
            return Vec::new();
        };
        (0..n)
            .map(|draw| match resolved(code, &[draw]).0 {
                ShipSystem::At(system) => system.0,
                other => panic!("{code}: {other:?}"),
            })
            .collect()
    }

    #[test]
    fn the_government_codes_draw_over_their_kind_away_from_the_players_place() {
        assert_eq!(pool(10_000), [136], "the Federation's, not at 130's place");
        assert_eq!(pool(9999), [133], "the independents'");
        assert_eq!(pool(10_002), [132]);
        assert_eq!(
            pool(15_000),
            [131],
            "allied with the Federation, not its own"
        );
        assert_eq!(
            pool(15_001),
            [136],
            "the Federation lists the Civvies' class"
        );
        assert_eq!(
            pool(15_002),
            Vec::<i16>::new(),
            "none allied with the pirates"
        );
        assert_eq!(pool(20_000), [131, 132, 133, 134], "an independent too");
        assert_eq!(pool(25_000), [132], "its enemies");
        assert_eq!(pool(30_000), [131, 136], "sharing class 1, its own too");
        assert_eq!(
            pool(31_000),
            [132, 133, 134],
            "sharing none, an independent too"
        );
        assert_eq!(pool(20_002), [131, 133, 134, 136], "not the pirates'");
        assert_eq!(pool(25_002), [136], "the pirates' enemies");
        assert_eq!(pool(30_002), [132], "sharing the pirates' class 9");
        assert_eq!(pool(31_002), [131, 133, 134, 136], "sharing none with them");
    }

    #[test]
    fn an_independent_counts_as_an_enemy_of_a_xenophobe() {
        let govts = Governments::new([GovtRecord {
            flags: crate::govt::XENOPHOBIC,
            ..govt(128)
        }]);
        let mut chance = Draws::of(&[2]);
        let drawn = select_ship_system(25_000, &facts(), &map(), &govts, &mut chance);
        assert_eq!(
            (drawn, chance.asked),
            (at(133), vec![4]),
            "131, 132, 133 and 134"
        );
    }

    #[test]
    fn a_ship_system_holds_its_own_system_or_any_for_one_following_the_player() {
        assert!(at(130).holds(SystemId(130)));
        assert!(!at(130).holds(SystemId(131)));
        assert!(ShipSystem::FollowsPlayer.holds(SystemId(131)));
        assert!(!ShipSystem::Nowhere.holds(SystemId(130)));
    }

    /// `STR#` 7000 names, 7001 subtitles; 7002 is empty.
    struct Strings;

    impl CommCatalog for Strings {
        fn string_list(&self, id: i16) -> Vec<String> {
            match id {
                7000 => vec!["Ace".into(), "Deuce".into(), "Trey".into()],
                7001 => vec!["Hunter".into(), "Killer".into()],
                _ => Vec::new(),
            }
        }
    }

    /// Three ships from 7000 and 7001, in the player's system, of
    /// `ShipStart` `start`.
    fn shipping(start: i16) -> MissionShip {
        MissionShip {
            count: 3,
            goal: 0,
            syst: -2,
            start,
            name_id: Some(7000),
            subtitle: Some(7001),
            ..MissionShip::NONE
        }
    }

    fn accepted(ships: &MissionShip, draws: &[u32]) -> (SpecialShips, Vec<u32>) {
        let mut chance = Draws::of(draws);
        let accepted =
            SpecialShips::at_accept(ships, &facts(), (&map(), &govts()), &Strings, &mut chance);
        (accepted, chance.asked)
    }

    #[test]
    fn at_accept_the_timer_the_system_the_name_and_the_subtitle_are_drawn_in_order() {
        for start in [1, 2] {
            let (ships, asked) = accepted(&shipping(start), &[7, 1, 2, 0]);
            assert_eq!(asked, [100, 5, 3, 2], "{start}");
            assert_eq!(
                ships,
                SpecialShips {
                    system: at(132),
                    left: 3,
                    arrived: if start == 1 { 0 } else { 3 },
                    appear: 107,
                    name: "Trey".into(),
                    subtitle: "Hunter".into(),
                },
                "{start}"
            );
        }
    }

    #[test]
    fn a_ship_start_of_none_or_below_runs_no_timer_and_all_have_arrived() {
        for start in [0, -1, -16] {
            let (ships, asked) = accepted(&shipping(start), &[1, 2, 0]);
            assert_eq!(asked, [5, 3, 2], "{start}");
            assert_eq!(ships.appear, NOT_APPEARING);
            assert_eq!((ships.left, ships.arrived), (3, 3));
        }
    }

    #[test]
    fn escorts_of_an_escort_goal_appear_after_30_drawing_nothing() {
        let escorts = MissionShip {
            behav: ESCORTS_PLAYER,
            goal: ESCORT_GOAL,
            syst: -1,
            ..shipping(1)
        };
        let (ships, asked) = accepted(&escorts, &[0, 0]);
        assert_eq!((ships.appear, asked), (30, vec![3, 2]));
        for (behav, goal) in [(ESCORTS_PLAYER, 0), (0, ESCORT_GOAL)] {
            let other = MissionShip {
                goal,
                behav,
                ..escorts
            };
            assert_eq!(accepted(&other, &[99, 0, 0]).0.appear, 199);
        }
    }

    #[test]
    fn ships_of_no_behaviour_draw_their_timer_even_with_an_escort_goal() {
        let unbehaved = MissionShip {
            count: 1,
            goal: ESCORT_GOAL,
            start: COMES_LATER,
            ..MissionShip::NONE
        };
        assert_eq!(appear_timer(&unbehaved, &mut Draws::of(&[5])), 105);
    }

    #[test]
    fn with_no_list_or_an_empty_one_the_name_and_subtitle_are_empty_drawing_nothing() {
        let unnamed = MissionShip {
            syst: -1,
            name_id: None,
            subtitle: Some(7002),
            ..shipping(0)
        };
        let (ships, asked) = accepted(&unnamed, &[]);
        assert_eq!((ships.name.as_str(), ships.subtitle.as_str()), ("", ""));
        assert!(asked.is_empty());
    }

    #[test]
    fn no_ships_are_left_with_a_ship_count_of_none_or_below() {
        for count in [0, -1] {
            let none = MissionShip {
                count,
                syst: -1,
                ..shipping(0)
            };
            assert_eq!(accepted(&none, &[0, 0]).0.left, count);
        }
    }

    #[test]
    fn the_ships_state_starts_nowhere_with_none_left() {
        let ships = SpecialShips::default();
        assert_eq!(ships.system, ShipSystem::Nowhere);
        assert_eq!((ships.left, ships.arrived, ships.appear), (0, 0, -1));
        assert!(ships.name.is_empty() && ships.subtitle.is_empty());
    }
}
