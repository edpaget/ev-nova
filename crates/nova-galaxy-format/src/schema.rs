//! JSON Schemas for editors, generated from the format's types so the two
//! can't drift: the `galaxy` tool writes them to `data/schema/`, and its
//! sample tests fail when the committed schemas differ from what the
//! types generate.

use schemars::JsonSchema;
use schemars::generate::SchemaSettings;

use crate::asset::{Asset, Budget};
use crate::fleet::{Livery, ModKit, Outfit, Ship};
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
        let (_, system) = &schemas()[2];
        let value: serde_json::Value = serde_json::from_str(system).unwrap();
        assert_eq!(value["additionalProperties"], false);
        assert_eq!(value["title"], "System");
        assert_eq!(value["$schema"], "http://json-schema.org/draft-07/schema#");
    }
}
