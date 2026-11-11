//! The ships that fly through space on their own: fleets (`fleets/`), and
//! the words a system names its traffic in ([`Traffic`]).
//!
//! A fleet is a group of ships flown the same way, by one faction or by
//! nobody's. A system names the fleets that fly in it, how often each
//! comes compared with the others, and how many ships it holds on
//! average. One fleet can fly in many systems.

use std::collections::BTreeMap;
use std::num::NonZeroU32;

use schemars::JsonSchema;
use serde::Deserialize;

use crate::id::Id;

/// A fleet (`fleets/<id>.md`): ships that fly through space together in
/// one way, for one faction or for nobody. A system's `traffic` names the
/// fleets that fly there.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Fleet {
    /// Display name (a placeholder).
    pub name: String,
    /// Whose ships they are (`factions/<id>.md`): they are friends and
    /// enemies of whom the faction is, and crimes against them count
    /// against the faction. Omit for ships that belong to nobody.
    #[serde(default)]
    pub faction: Option<Id>,
    /// How its ships fly.
    pub role: FleetRole,
    /// The ships it flies (`ships/<id>.md`, hulls or variants), at most
    /// sixteen, each with a weight: a ship with weight 2 comes twice as
    /// often as one with weight 1. A weight is at least 1.
    pub ships: BTreeMap<Id, NonZeroU32>,
}

/// How a fleet's ships fly. A pirate fleet is a `warship` fleet of a
/// faction whose warships attack the player on sight.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum FleetRole {
    /// Goes about its business and flees when attacked.
    Trader,
    /// Goes about its business and fights back when attacked.
    ArmedTrader,
    /// Hunts its faction's enemies, and the player when its faction or
    /// the player's legal record says so. It breaks off when its shields
    /// run low.
    Warship,
    /// Police: hunts as a warship does, and keeps fighting however low
    /// its shields run.
    Interceptor,
}

impl FleetRole {
    /// Every role.
    pub const ALL: [FleetRole; 4] = [
        FleetRole::Trader,
        FleetRole::ArmedTrader,
        FleetRole::Warship,
        FleetRole::Interceptor,
    ];

    /// The role as written in data.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            FleetRole::Trader => "trader",
            FleetRole::ArmedTrader => "armed-trader",
            FleetRole::Warship => "warship",
            FleetRole::Interceptor => "interceptor",
        }
    }
}

/// The ships that fly through a system on their own.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Traffic {
    /// The fleets that fly here (`fleets/<id>.md`), at most eight, each
    /// with a weight: a fleet with weight 2 comes twice as often as one
    /// with weight 1. A weight is at least 1. Leave it empty (`{}`) for a
    /// system nobody flies through.
    pub fleets: BTreeMap<Id, NonZeroU32>,
    /// How many ships the system holds on average. Omit for the game's
    /// usual number.
    #[serde(default)]
    pub average_ships: Option<u32>,
}

crate::model::kinds! {
    Fleet => "fleet" in "fleets",
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontmatter::parse;
    use crate::model::tests::assert_data_names;
    use crate::model::{Mission, Objective, System};

    fn read<T: serde::de::DeserializeOwned>(yaml: &str) -> Result<T, String> {
        parse::<T>(&format!("---\n{yaml}---\n")).map(|(t, _)| t)
    }

    fn weights(map: &BTreeMap<Id, NonZeroU32>) -> Vec<(&str, u32)> {
        map.iter().map(|(id, w)| (id.as_str(), w.get())).collect()
    }

    #[test]
    fn a_fleet_reads_its_name_faction_role_and_weighted_ships() {
        let fleet: Fleet = read(
            "name: Raider band\nfaction: raider\nrole: warship\n\
             ships: {raider-skiff: 2, armoured-skiff: 1}\n",
        )
        .unwrap();
        assert_eq!(fleet.name, "Raider band");
        assert_eq!(fleet.faction.as_ref().map(Id::as_str), Some("raider"));
        assert_eq!(fleet.role, FleetRole::Warship);
        assert_eq!(
            weights(&fleet.ships),
            [("armoured-skiff", 1), ("raider-skiff", 2)]
        );
    }

    #[test]
    fn a_fleet_that_names_no_faction_has_none() {
        let fleet: Fleet = read("name: Haulers\nrole: trader\nships: {hauler: 1}\n").unwrap();
        assert_eq!(fleet.faction, None);
    }

    #[test]
    fn a_fleet_refuses_an_unknown_role_an_unknown_key_and_a_weight_of_zero() {
        let role = read::<Fleet>("name: F\nrole: pirate\nships: {s: 1}\n").unwrap_err();
        assert!(role.contains("pirate"), "{role}");
        let key = read::<Fleet>("name: F\nrole: trader\nships: {s: 1}\nescorts: 2\n").unwrap_err();
        assert!(key.contains("unknown field `escorts`"), "{key}");
        let zero = read::<Fleet>("name: F\nrole: trader\nships: {s: 0}\n");
        assert!(zero.is_err(), "{zero:?}");
        let ships = read::<Fleet>("name: F\nrole: trader\n").unwrap_err();
        assert!(ships.contains("missing field `ships`"), "{ships}");
    }

    #[test]
    fn a_fleet_role_is_named_as_in_data() {
        assert_data_names(&FleetRole::ALL, FleetRole::as_str);
    }

    const SYSTEM: &str = "name: Fringe\nregion: rim\nmap: [0, 0]\n";

    #[test]
    fn a_system_reads_its_traffic() {
        let system: System = read(&format!(
            "{SYSTEM}traffic:\n  fleets: {{raider-band: 50, warden-patrol: 30}}\n  average_ships: 6\n"
        ))
        .unwrap();
        let traffic = system.traffic.expect("traffic");
        assert_eq!(
            weights(&traffic.fleets),
            [("raider-band", 50), ("warden-patrol", 30)]
        );
        assert_eq!(traffic.average_ships, Some(6));
    }

    #[test]
    fn a_system_that_names_no_traffic_reads_none() {
        let system: System = read(SYSTEM).unwrap();
        assert!(system.traffic.is_none());
        let empty: System = read(&format!("{SYSTEM}traffic: {{fleets: {{}}}}\n")).unwrap();
        let traffic = empty.traffic.expect("traffic");
        assert!(traffic.fleets.is_empty());
        assert_eq!(traffic.average_ships, None);
    }

    #[test]
    fn traffic_refuses_a_missing_fleets_a_weight_of_zero_and_an_unknown_key() {
        let missing = read::<System>(&format!("{SYSTEM}traffic: {{average_ships: 3}}\n"));
        assert!(missing.is_err(), "{missing:?}");
        let zero = read::<System>(&format!("{SYSTEM}traffic: {{fleets: {{f: 0}}}}\n"));
        assert!(zero.is_err(), "{zero:?}");
        let key = read::<System>(&format!("{SYSTEM}traffic: {{fleets: {{}}, ships: 3}}\n"));
        assert!(key.is_err(), "{key:?}");
    }

    const TAKE_POINT: &str = "title: T\nstoryline: s\nact: committed\n\
         objective:\n  take_point:\n    point: fringe/drift-road\n    held_by: warden\n";

    fn guards(mission: &Mission) -> Option<u32> {
        match mission.objective {
            Some(Objective::TakePoint { guards, .. }) => guards.map(NonZeroU32::get),
            _ => panic!("a take_point"),
        }
    }

    #[test]
    fn a_take_point_reads_its_guards_and_reads_none_when_it_names_none() {
        let named: Mission = read(&format!("{TAKE_POINT}    guards: 2\n")).unwrap();
        assert_eq!(guards(&named), Some(2));
        let unnamed: Mission = read(TAKE_POINT).unwrap();
        assert_eq!(guards(&unnamed), None);
    }

    #[test]
    fn a_take_point_refuses_no_guards() {
        let zero = read::<Mission>(&format!("{TAKE_POINT}    guards: 0\n"));
        assert!(zero.is_err(), "{zero:?}");
    }

    /// The names `schema`'s object at `pointer` requires, and whether it
    /// describes `word`.
    fn optional_and_described(schema: &serde_json::Value, pointer: &str, word: &str) {
        let object = schema.pointer(pointer).expect(pointer);
        let required: Vec<&str> = object["required"]
            .as_array()
            .map(|r| r.iter().filter_map(|r| r.as_str()).collect())
            .unwrap_or_default();
        assert!(!required.contains(&word), "{word}: {required:?}");
        assert!(
            object["properties"][word]["description"]
                .as_str()
                .is_some_and(|d| !d.is_empty()),
            "{word}: {object}"
        );
    }

    /// The committed schema named `name`, parsed.
    fn schema(name: &str) -> serde_json::Value {
        let (_, json) = (crate::schema::schemas().into_iter())
            .find(|(file, _)| file == name)
            .expect("a schema of that name");
        serde_json::from_str(&json).expect("JSON")
    }

    #[test]
    fn the_schemas_list_the_traffic_words_as_optional_where_they_are() {
        let system = schema("system.schema.json");
        optional_and_described(&system, "", "traffic");
        optional_and_described(&system, "/definitions/Traffic", "average_ships");
        assert_eq!(
            system["definitions"]["Traffic"]["required"],
            serde_json::json!(["fleets"])
        );
        let mission = schema("mission.schema.json");
        let take_point = (mission["definitions"]["Objective"]["oneOf"].as_array())
            .expect("the objectives")
            .iter()
            .find(|o| o["required"] == serde_json::json!(["take_point"]))
            .expect("take_point")
            .clone();
        optional_and_described(&take_point, "/properties/take_point", "guards");
        let fleet = schema("fleet.schema.json");
        optional_and_described(&fleet, "", "faction");
        assert_eq!(
            fleet["required"],
            serde_json::json!(["name", "role", "ships"])
        );
        let keyed = fleet["properties"]["ships"]["patternProperties"]
            .as_object()
            .expect("keyed by ship");
        let weight = keyed.values().next().expect("a weight for each ship");
        assert_eq!(weight["minimum"], 1, "{fleet}");
    }

    #[test]
    fn the_loader_loads_the_fleets_by_file_name() {
        let source = crate::fixture::galaxy().with(
            "fleets/haulers.md",
            "---\nname: Haulers\nrole: trader\nships: {skiff: 1}\n---\nThey haul.\n",
        );
        let (galaxy, problems) = crate::load::load(&source);
        assert!(problems.is_empty(), "{problems:?}");
        let id = Id::try_from("haulers".to_owned()).unwrap();
        let haulers = &galaxy.fleets[&id];
        assert_eq!(
            (haulers.path.as_str(), haulers.data.role),
            ("fleets/haulers.md", FleetRole::Trader)
        );
        assert_eq!(haulers.body, "They haul.\n");
    }

    #[test]
    fn a_fleet_with_an_unknown_role_fails_to_load_and_is_reported_once() {
        let source = crate::fixture::galaxy().with(
            "fleets/odd.md",
            "---\nname: Odd\nrole: pirate\nships: {skiff: 1}\n---\n",
        );
        let (galaxy, problems) = crate::load::load(&source);
        assert!(galaxy.fleets.is_empty());
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert_eq!(problems[0].file, "fleets/odd.md");
        assert!(galaxy.unreadable.contains("fleets/odd.md"));
    }
}
