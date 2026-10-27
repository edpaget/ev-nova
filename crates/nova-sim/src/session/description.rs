//! A fresh pilot's description: what a `dësc` loads to with no session to
//! take it in ([`Session::fresh_description`]), beside
//! [`Session::load_description`], which takes one in for the pilot
//! flying.

use super::Session;
use crate::control::Gate;
use crate::text_parse;

impl Session {
    /// `raw`, a `dësc` that may be missing, with its conditional text
    /// resolved for a new pilot
    /// ([`Gate::FRESH`]) by the default
    /// parse rules: what a flight that failed to start loads, as it has
    /// no pilot to read and no chance to draw `l33t` letters on. A
    /// missing one is empty, as [`Session::load_description`] leaves it.
    #[must_use]
    pub fn fresh_description(raw: Option<&str>) -> String {
        raw.map(|raw| {
            text_parse::conditional_text(raw, Gate::FRESH.pilot, text_parse::ParseRules::default())
        })
        .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_description_resolves_its_conditional_text_for_a_new_pilot() {
        assert_eq!(
            Session::fresh_description(Some("{b5 \"set\" \"clear\"}, sir.")),
            "clear, sir.",
            "bit 5 is clear for a new pilot"
        );
        assert_eq!(Session::fresh_description(Some("plain")), "plain");
        assert_eq!(Session::fresh_description(None), "", "a missing one");
    }
}
