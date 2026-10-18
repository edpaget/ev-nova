//! How the ships the traffic adds roll their afterburner (the original's
//! +0xbd; see [`afterburner`]): `Traffic::add` rolls each ship whose
//! spawn asks `_AIHasAfterburner` ([`afterburner::asks`]) against the
//! player's combat rating and the yardstick
//! [`World::afterburner_yardstick`] reads, and gives a person of `përs`
//! `Flags` [`PERSON_AFTERBURNER`] one whatever its roll (`_SpawnPerson`
//! @0x40d99-0x40da2).

use crate::ai::afterburner::{self, PERSON_AFTERBURNER};
use crate::chance::Chance;
use crate::rulebook::RuleSource;
use crate::traffic::World;
use crate::traffic::spawn::NewShip;
use crate::traffic::table::{ShipKind, SpawnPerson};

/// What a spawn rolls afterburners with: the player's combat rating, the
/// first `shïp` record's `Strength`, the yardstick's reading and the dice.
pub(crate) struct AfterburnerRoll<'c> {
    rating: i64,
    first: i16,
    reading: RuleSource,
    chance: &'c mut dyn Chance,
}

impl<'c> AfterburnerRoll<'c> {
    /// The roll `world` gives, on `chance`: none of a rating or yardstick
    /// with no player there.
    pub(crate) fn of(world: &World, chance: &'c mut dyn Chance) -> Self {
        let (rating, first) = world
            .player
            .map_or((0, 0), |player| (player.rating, player.yardstick));
        Self {
            rating,
            first,
            reading: world.afterburner_yardstick,
            chance,
        }
    }

    /// Whether `ship`, of `kind`, flown by `person` if any, has an
    /// afterburner as it is added, rolled on this roll, or with none never
    /// rolled (see the module docs).
    pub(crate) fn has(
        roll: Option<&mut Self>,
        ship: &NewShip,
        kind: &ShipKind,
        person: Option<&SpawnPerson>,
    ) -> bool {
        // A class's `Strength` is none below none, so in 16 bits.
        #[allow(clippy::cast_possible_truncation)]
        let own = kind.hull.strength as i16;
        let rolled = afterburner::asks(ship)
            && roll.is_some_and(|roll| {
                let yardstick = afterburner::yardstick(roll.reading, roll.first, own);
                afterburner::has_afterburner(&kind.hull, roll.rating, yardstick, roll.chance)
            });
        rolled || person.is_some_and(|person| person.record.flags & PERSON_AFTERBURNER != 0)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::*;
    use crate::ai::PlayerSide;
    use crate::catalog::{PersonId, PersonRecord, ShipId};
    use crate::combat::hull::{AFTERBURNER_ALWAYS, AFTERBURNER_BY_RATING, HullSpec};
    use crate::flight::ShipState;
    use crate::traffic::npc::{AiType, MissionMark};
    use crate::traffic::spawn::Entry;
    use crate::traffic::table::SpawnTable;
    use crate::traffic::{ShipBehav, Traffic};
    use nova_sim_base::fixture::{Draws, Ranged};

    /// Ship types 200 (always), 201 (by rating, `Strength` 40) and 202
    /// (neither bit), and person 128 flying 202 with `përs` `Flags`
    /// `flags`.
    fn table(flags: u16) -> SpawnTable {
        let kind = |afterburner: u16, strength: f32| ShipKind {
            hull: HullSpec {
                afterburner,
                strength,
                ..HullSpec::default()
            },
            ..ShipKind::default()
        };
        SpawnTable {
            ships: BTreeMap::from([
                (ShipId(200), kind(AFTERBURNER_ALWAYS, 1.0)),
                (ShipId(201), kind(AFTERBURNER_BY_RATING, 40.0)),
                (ShipId(202), kind(0, 1.0)),
            ]),
            persons: BTreeMap::from([(
                PersonId(128),
                SpawnPerson {
                    record: PersonRecord {
                        flags,
                        ..nova_sim_catalog::fixture::person(128, 202)
                    },
                    linked: false,
                    kind: kind(0, 1.0),
                    reserves: crate::reserves::Reserves::default(),
                    condition: crate::combat::hull::Condition::Intact,
                    derelict: false,
                },
            )]),
            ..SpawnTable::default()
        }
    }

    /// A `düde` ship of type `ship`, placed.
    fn duede(ship: i16) -> NewShip {
        NewShip {
            ship: ShipId(ship),
            govt: None,
            ai_type: AiType::WimpyTrader,
            lead: None,
            state: ShipState::default(),
            entry: Entry::Placed,
            aggression: 0,
            booty: Some(0),
            info_types: 0,
            person: None,
            mission: None,
            aux: None,
            derelict: false,
            held: false,
            boarded: false,
            cloaked: false,
            quote: None,
            cargo: None,
        }
    }

    /// The afterburners of `ships` added among a world whose player has a
    /// combat rating of 1024 and `shïp` 128's `Strength` 2, the yardstick
    /// read as `reading`, persons flying with `përs` `Flags` `flags`,
    /// `Rand(1344)` giving `outcome`.
    fn added(ships: Vec<NewShip>, flags: u16, reading: RuleSource, outcome: u32) -> Vec<bool> {
        let mut traffic = Traffic::new();
        traffic.table = table(flags);
        let world = World {
            player: Some(PlayerSide {
                rating: 1024,
                yardstick: 2,
                ..crate::ai::fixture::player(0.0, 0.0)
            }),
            afterburner_yardstick: reading,
            ..World::new(&[])
        };
        let mut chance = Ranged::of(&[(1344, outcome)]);
        traffic.add(
            ships,
            &BTreeSet::new(),
            None,
            Some(&mut AfterburnerRoll::of(&world, &mut chance)),
        );
        traffic.npcs().iter().map(|npc| npc.afterburner).collect()
    }

    #[test]
    fn a_duede_ship_of_flags_0x0040_has_one_and_of_neither_bit_none() {
        assert_eq!(
            added(vec![duede(200), duede(202)], 0, RuleSource::Engine, 0),
            [true, false]
        );
    }

    #[test]
    fn a_duede_ship_of_flags_0x0020_rolls_the_rating_over_shïp_128s_strength() {
        // 1024 / 2 = 512 against the roll + 256.
        assert_eq!(added(vec![duede(201)], 0, RuleSource::Engine, 256), [true]);
        assert_eq!(added(vec![duede(201)], 0, RuleSource::Engine, 257), [false]);
        // By the other reading over its own `Strength` of 40: 25.
        assert_eq!(added(vec![duede(201)], 0, RuleSource::Bible, 0), [false]);
    }

    #[test]
    fn a_fleets_lead_rolls_and_its_escorts_have_none() {
        let lead = NewShip {
            booty: None,
            ..duede(200)
        };
        let escort = NewShip {
            lead: Some(0),
            ..lead
        };
        assert_eq!(
            added(vec![lead, escort], 0, RuleSource::Engine, 0),
            [true, false]
        );
    }

    #[test]
    fn a_person_rolls_and_has_one_whatever_the_roll_with_pers_flags_0x0002() {
        let person = NewShip {
            person: Some(PersonId(128)),
            booty: None,
            ..duede(202)
        };
        assert_eq!(added(vec![person], 0, RuleSource::Engine, 0), [false]);
        assert_eq!(added(vec![person], 0x0002, RuleSource::Engine, 0), [true]);
    }

    #[test]
    fn a_mission_ship_and_an_auxiliary_ship_have_none() {
        let mission = NewShip {
            mission: Some(MissionMark {
                slot: 0,
                behav: ShipBehav::Own,
                formation: None,
                order: None,
            }),
            ..duede(200)
        };
        let aux = NewShip {
            aux: Some(1),
            ..duede(200)
        };
        assert_eq!(
            added(vec![mission, aux], 0, RuleSource::Engine, 0),
            [false, false]
        );
    }

    // The spawn paths that roll, and the one that does not.

    /// A system of `avg_ships` on average, all of düde 128's ship 200,
    /// which always has an afterburner; its reinforcement fleet, 129, ship
    /// 200 leading one more.
    fn spawning(avg_ships: u32) -> SpawnTable {
        use crate::catalog::{
            DudeId, EscortRecord, FleetId, FleetRecord, GovtId, SystemReinforcements,
        };
        use crate::stats::ShipStats;
        use crate::traffic::table::SpawnDude;
        use nova_sim_base::fixture::FAST;
        SpawnTable {
            avg_ships,
            dudes: vec![(DudeId(128), 100)],
            dude_records: BTreeMap::from([(
                DudeId(128),
                SpawnDude {
                    ai_type: 1,
                    govt: Some(GovtId(130)),
                    ships: vec![(ShipId(200), 1)],
                    booty: 0,
                    info_types: 0,
                },
            )]),
            ships: BTreeMap::from([(
                ShipId(200),
                ShipKind {
                    stats: ShipStats::new(FAST, &[]),
                    hull: HullSpec {
                        afterburner: AFTERBURNER_ALWAYS,
                        ..HullSpec::default()
                    },
                    ..ShipKind::default()
                },
            )]),
            fleets: BTreeMap::from([(
                FleetId(129),
                FleetRecord {
                    id: FleetId(129),
                    lead: Some(ShipId(200)),
                    escorts: vec![EscortRecord {
                        ship: ShipId(200),
                        min: 1,
                        max: 1,
                    }],
                    govt: Some(GovtId(128)),
                    link_syst: -1,
                    appear_on: crate::control::Test::default(),
                    quote: None,
                    flags: 0,
                },
            )]),
            reinforcements: Some(SystemReinforcements {
                fleet: FleetId(129),
                time: 900,
                interval: 2,
            }),
            ..SpawnTable::default()
        }
    }

    /// The afterburners of `traffic`'s NPCs, in order.
    fn afterburners(traffic: &Traffic) -> Vec<bool> {
        traffic.npcs().iter().map(|npc| npc.afterburner).collect()
    }

    #[test]
    fn a_ship_the_setup_places_rolls_its_afterburner() {
        let mut traffic = Traffic::new();
        // One setup pass placing ship 200.
        traffic.enter(spawning(1), &mut Draws::of(&[6, 6, 0, 0, 750, 750, 0, 2]));
        assert_eq!(afterburners(&traffic), [true]);
    }

    #[test]
    fn a_ship_that_arrives_rolls_its_afterburner() {
        let mut traffic = Traffic::new();
        // In an empty system of AvgShips 1, nothing set up.
        traffic.table = spawning(1);
        // The arrival rolled below AvgShips 1, then düde 128's ship 200.
        traffic.enter_more_ships(World::new(&[]), &mut Draws::of(&[0, 6, 6, 0, 0, 90]));
        assert_eq!(afterburners(&traffic), [true]);
    }

    #[test]
    fn a_fleets_lead_brought_by_hyperspace_rolls_its_afterburner_and_its_escort_has_none() {
        let mut traffic = Traffic::new();
        traffic.enter(spawning(0), &mut Draws::of(&[]));
        traffic.bring_reinforcements(&World::new(&[]), &mut Draws::of(&[0]));
        assert_eq!(afterburners(&traffic), [true, false]);
    }

    #[test]
    fn a_ship_brought_in_as_it_is_rolls_no_afterburner() {
        let mut traffic = Traffic::new();
        traffic.enter(spawning(0), &mut Draws::of(&[]));
        let mission = NewShip {
            mission: Some(MissionMark {
                slot: 0,
                behav: ShipBehav::Own,
                formation: None,
                order: None,
            }),
            ..duede(200)
        };
        // Neither `_MissionDudeSpawn` nor `_SpecificDudeSpawn` asks, even
        // of a ship that would.
        traffic.bring_in_within(vec![mission, duede(200)], 8);
        assert_eq!(afterburners(&traffic), [false, false]);
    }
}
