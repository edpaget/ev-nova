//! JSON Schemas for editors, generated from the format's types so the two
//! can't drift: the `galaxy` tool writes them to `data/schema/`, and its
//! sample tests fail when the committed schemas differ from what the
//! types generate.

use schemars::JsonSchema;
use schemars::generate::SchemaSettings;

use crate::asset::{Asset, Budget};
use crate::fleet::{Livery, ModKit, Outfit, Ship};
use crate::goods::Good;
use crate::model::{
    Character, Faction, Kind, Mission, News, Region, Start, Stellar, Storyline, System,
};

/// Each kind's schema as `(file name, pretty JSON)`, in folder order.
#[must_use]
pub fn schemas() -> Vec<(String, String)> {
    vec![
        one::<Region>(),
        one::<Faction>(),
        one::<System>(),
        one::<Stellar>(),
        one::<Good>(),
        one::<Storyline>(),
        one::<Mission>(),
        one::<News>(),
        one::<Character>(),
        one::<Ship>(),
        one::<ModKit>(),
        one::<Outfit>(),
        one::<Livery>(),
        one::<Asset>(),
        one::<Budget>(),
        one::<Start>(),
    ]
}

/// Draft 7, which YAML editors support most widely.
fn one<T: JsonSchema + Kind>() -> (String, String) {
    let schema = SchemaSettings::draft07()
        .into_generator()
        .into_root_schema_for::<T>();
    let json = serde_json::to_string_pretty(&schema).expect("a schema serializes");
    (format!("{}.schema.json", T::NAME), json + "\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The schema named `name`.
    fn schema(name: &str) -> (String, String) {
        schemas()
            .into_iter()
            .find(|(file, _)| file == name)
            .expect("a schema of that name")
    }

    /// The schema named `name`, parsed.
    fn value(name: &str) -> serde_json::Value {
        serde_json::from_str(&schema(name).1).expect("JSON")
    }

    #[test]
    fn there_is_one_schema_per_kind() {
        let names: Vec<_> = schemas().into_iter().map(|(name, _)| name).collect();
        assert_eq!(
            names,
            [
                "region.schema.json",
                "faction.schema.json",
                "system.schema.json",
                "stellar.schema.json",
                "good.schema.json",
                "storyline.schema.json",
                "mission.schema.json",
                "news.schema.json",
                "character.schema.json",
                "ship.schema.json",
                "mod.schema.json",
                "outfit.schema.json",
                "livery.schema.json",
                "asset.schema.json",
                "budget.schema.json",
                "start.schema.json",
            ]
        );
    }

    #[test]
    fn a_schema_rejects_unknown_fields() {
        let (_, system) = &schema("system.schema.json");
        let value: serde_json::Value = serde_json::from_str(system).unwrap();
        assert_eq!(value["additionalProperties"], false);
        assert_eq!(value["title"], "System");
        assert_eq!(value["$schema"], "http://json-schema.org/draft-07/schema#");
    }

    #[test]
    fn the_mission_schema_lists_the_outfit_effects() {
        let (_, mission) = &schema("mission.schema.json");
        let value: serde_json::Value = serde_json::from_str(mission).unwrap();
        let effects = &value["definitions"]["Effects"]["properties"];
        for field in ["grant_outfits", "remove_outfits"] {
            assert!(
                effects[field]["description"]
                    .as_str()
                    .is_some_and(|d| d.contains("outfit")),
                "{field}: {mission}"
            );
        }
    }

    #[test]
    fn the_stellar_schema_lists_its_conversation() {
        let (_, stellar) = &schema("stellar.schema.json");
        let value: serde_json::Value = serde_json::from_str(stellar).unwrap();
        let dialog = &value["properties"]["dialog"];
        assert!(dialog.is_object(), "{stellar}");
        assert!(
            dialog["description"]
                .as_str()
                .is_some_and(|d| d.contains("conversation")),
            "{dialog}"
        );
    }

    #[test]
    fn the_good_schema_describes_its_name_and_price() {
        let good = value("good.schema.json");
        assert_eq!(good["title"], "Good");
        assert_eq!(good["additionalProperties"], false);
        assert!(
            good["properties"]["price"]["description"]
                .as_str()
                .is_some_and(|d| d.contains("credits a ton")),
            "{good}"
        );
        assert!(good["properties"]["name"].is_object(), "{good}");
        assert_eq!(good["required"], serde_json::json!(["name", "price"]));
    }

    #[test]
    fn the_outfit_schema_lists_the_stellars_that_sell_it() {
        let outfit = value("outfit.schema.json");
        let sold_at = &outfit["properties"]["sold_at"];
        assert_eq!(sold_at["type"], "array", "{sold_at}");
        let pattern = sold_at["items"]["pattern"].as_str().or_else(|| {
            let name = sold_at["items"]["$ref"].as_str()?.rsplit('/').next()?;
            outfit["definitions"][name]["pattern"].as_str()
        });
        assert_eq!(pattern, Some("^[a-z0-9]+(-[a-z0-9]+)*$"), "{outfit}");
        let description = sold_at["description"].as_str().unwrap_or_default();
        assert!(
            description.contains("stellars/") && description.contains("outfitter"),
            "{sold_at}"
        );
        let required = outfit["required"].as_array().expect("required");
        assert!(!required.contains(&"sold_at".into()), "{required:?}");
    }

    #[test]
    fn the_stellar_schema_maps_each_good_it_prices_to_a_level() {
        let stellar = value("stellar.schema.json");
        let prices = &stellar["properties"]["prices"];
        assert_eq!(prices["type"], "object", "{prices}");
        assert!(
            prices["description"]
                .as_str()
                .is_some_and(|d| d.contains("goods/")),
            "{prices}"
        );
        let keyed = prices["patternProperties"]
            .as_object()
            .expect("keyed by good");
        let (pattern, level) = keyed.iter().next().expect("one pattern");
        assert_eq!(pattern, "^[a-z0-9]+(-[a-z0-9]+)*$", "a good's id");
        let level = level["$ref"].as_str().expect("a level for each good");
        let name = level.rsplit('/').next().expect("a definition");
        let levels: Vec<&str> = stellar["definitions"][name]["oneOf"]
            .as_array()
            .expect("the levels")
            .iter()
            .filter_map(|level| level["const"].as_str())
            .collect();
        assert_eq!(levels, ["low", "medium", "high"], "{stellar}");
        let required = stellar["required"].as_array().expect("required");
        assert!(!required.contains(&"prices".into()), "{required:?}");
    }
}
