//! The goods a pilot trades (`goods/`), and the words a stellar prices
//! them in: a stellar's `prices` maps each good it trades to a
//! [`PriceLevel`].
//!
//! These are kept small: a good is a name and a base price, and a port
//! prices it low, medium or high. Legality, prices that drift with what a
//! port has heard, and fuel are left for the spaceport's design.

use schemars::JsonSchema;
use serde::Deserialize;

/// A trade good (`goods/<id>.md`): something a pilot buys at one port and
/// sells at another. A stellar that trades it names it in its `prices`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Good {
    /// Display name (a placeholder).
    pub name: String,
    /// The base price, in credits a ton: what a port that prices it
    /// `medium` asks. A `low` port asks less and a `high` port more.
    pub price: u32,
}

/// How dear a good is at a stellar that trades it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum PriceLevel {
    /// Below the base price: a good place to buy it.
    Low,
    /// The base price.
    Medium,
    /// Above the base price: a good place to sell it.
    High,
}

impl PriceLevel {
    /// The level as written in data.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            PriceLevel::Low => "low",
            PriceLevel::Medium => "medium",
            PriceLevel::High => "high",
        }
    }
}

crate::model::kinds! {
    Good => "good" in "goods",
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontmatter::parse;
    use crate::id::Id;
    use crate::model::Stellar;
    use crate::model::tests::assert_data_names;

    fn read<T: serde::de::DeserializeOwned>(yaml: &str) -> Result<T, String> {
        parse::<T>(&format!("---\n{yaml}---\n")).map(|(t, _)| t)
    }

    fn id(text: &str) -> Id {
        Id::try_from(text.to_owned()).expect("an id")
    }

    #[test]
    fn a_good_reads_its_name_and_price() {
        let good: Good = read("name: Grain\nprice: 100\n").unwrap();
        assert_eq!((good.name.as_str(), good.price), ("Grain", 100));
    }

    #[test]
    fn a_good_refuses_a_field_it_does_not_have() {
        let error = read::<Good>("name: Grain\nprice: 100\nlegal: false\n").unwrap_err();
        assert!(error.contains("unknown field `legal`"), "{error}");
        let error = read::<Good>("name: Grain\n").unwrap_err();
        assert!(error.contains("missing field `price`"), "{error}");
    }

    #[test]
    fn a_price_level_is_named_as_in_data() {
        use PriceLevel::{High, Low, Medium};
        assert_data_names(&[Low, Medium, High], PriceLevel::as_str);
    }

    const STELLAR: &str = "name: Dock\nkind: station\nsystem: alpha\nposition: [0, 0]\n";

    #[test]
    fn a_stellar_reads_the_level_of_each_good_it_prices() {
        let stellar: Stellar = read(&format!(
            "{STELLAR}services: [trade]\nprices: {{grain: low, machinery: high, alloys: medium}}\n"
        ))
        .unwrap();
        let prices: Vec<_> = stellar
            .prices
            .iter()
            .map(|(good, &level)| (good.as_str(), level))
            .collect();
        assert_eq!(
            prices,
            [
                ("alloys", PriceLevel::Medium),
                ("grain", PriceLevel::Low),
                ("machinery", PriceLevel::High)
            ]
        );
    }

    #[test]
    fn a_stellar_that_leaves_out_its_prices_prices_nothing() {
        let stellar: Stellar = read(STELLAR).unwrap();
        assert!(stellar.prices.is_empty());
    }

    #[test]
    fn an_unknown_level_or_a_good_that_is_not_an_id_is_an_error() {
        assert!(read::<Stellar>(&format!("{STELLAR}prices: {{grain: cheap}}\n")).is_err());
        let error = read::<Stellar>(&format!("{STELLAR}prices: {{Grain: low}}\n")).unwrap_err();
        assert!(error.contains("`Grain`"), "{error}");
    }

    #[test]
    fn the_loader_loads_the_goods_by_file_name() {
        let source = crate::fixture::galaxy().with(
            "goods/grain.md",
            "---\nname: Grain\nprice: 100\n---\nFood.\n",
        );
        let (galaxy, problems) = crate::load::load(&source);
        assert!(problems.is_empty(), "{problems:?}");
        let grain = &galaxy.goods[&id("grain")];
        assert_eq!(
            (grain.path.as_str(), grain.data.price),
            ("goods/grain.md", 100)
        );
        assert_eq!(grain.body, "Food.\n");
    }

    #[test]
    fn the_fixture_galaxy_has_no_goods() {
        let (galaxy, _) = crate::load::load(&crate::fixture::galaxy());
        assert!(galaxy.goods.is_empty());
    }
}
