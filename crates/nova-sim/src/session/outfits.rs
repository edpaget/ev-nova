//! Granting and removing outfits on the session.

use super::Session;
use crate::outfit_effects::OutfitRules;

impl Session {
    /// This session with granting and removing outfits following `rules`
    /// where the Bible and the engine disagree: the engine's by default.
    #[must_use]
    pub fn with_outfit_rules(mut self, rules: OutfitRules) -> Self {
        self.outfit_rules = rules;
        self
    }

    /// The rules granting and removing outfits follow.
    #[must_use]
    pub fn outfit_rules(&self) -> OutfitRules {
        self.outfit_rules
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pilot::Pilot;
    use crate::rulebook::RuleSource;
    use crate::testkit::catalog;

    #[test]
    fn the_outfit_rules_are_the_engines_until_others_are_given() {
        let catalog = catalog();
        let session =
            Session::fly(&catalog, Pilot::new(&catalog, "Ada").expect("starts")).expect("flies");
        assert_eq!(session.outfit_rules(), OutfitRules::default());
        let rules = OutfitRules {
            map_explore: RuleSource::Bible,
            ..OutfitRules::default()
        };
        assert_eq!(session.with_outfit_rules(rules).outfit_rules(), rules);
    }
}
