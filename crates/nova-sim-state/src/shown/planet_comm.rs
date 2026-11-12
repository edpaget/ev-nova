//! The planet comm dialog's view: its buttons ([`PlanetCommButton`]), its
//! items ([`PlanetCommItem`]), its status line ([`PlanetStatus`]) and what
//! it shows ([`PlanetCommView`]), with the strings they say, from
//! `nova_sim_ai::planet_comm`, which re-exports them and keeps the
//! dialog's bribes, demands and replies.

use nova_sim_catalog::catalog::StellarId;

/// A button of the dialog, by what it says (`STR#` 150).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanetCommButton {
    /// "Greetings" (#22).
    Greetings,
    /// "Offer Bribe" (#24).
    OfferBribe,
    /// "Demand Tribute" (#45).
    DemandTribute,
    /// "Release" (#32).
    Release,
}

/// "Greetings": `STR#` 150 #22.
pub const GREETINGS: &str = "Greetings";

/// "Offer Bribe": `STR#` 150 #24.
pub const OFFER_BRIBE: &str = "Offer Bribe";

/// "Demand Tribute": `STR#` 150 #45.
pub const DEMAND_TRIBUTE: &str = "Demand Tribute";

/// "Release": `STR#` 150 #32.
pub const RELEASE: &str = "Release";

impl PlanetCommButton {
    /// What it says.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Greetings => GREETINGS,
            Self::OfferBribe => OFFER_BRIBE,
            Self::DemandTribute => DEMAND_TRIBUTE,
            Self::Release => RELEASE,
        }
    }
}

/// An item of the dialog pressed, by its place (`DITL` 1009 items 1 to
/// 3), as the filter and the track give it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanetCommItem {
    /// Item 1, "Close Channel".
    Close,
    /// Item 2, Greetings or Offer Bribe.
    Second,
    /// Item 3, Demand Tribute or Release, hidden for a stellar always
    /// dominated.
    Third,
}

/// The status line after "Status:" (`STR#` 2002 #196), as
/// `_PlanetCommDialogUpdate` draws it (@0x90fb6-0x910e1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanetStatus {
    /// "Owned" (#171): always dominated, in bright green.
    Owned,
    /// "Dominated" (#172), in bright green.
    Dominated,
    /// "Forbidden" (#173): refusing landing with a record of 0 or more,
    /// bold orange.
    Forbidden,
    /// "Hostile" (#174): refusing landing with a record below 0, bold red.
    Hostile,
}

/// "Status:": `STR#` 2002 #196.
pub const STATUS_LABEL: &str = "Status:";

/// "Owned": `STR#` 2002 #171.
pub const OWNED: &str = "Owned";

/// "Dominated": `STR#` 2002 #172.
pub const DOMINATED_STATUS: &str = "Dominated";

/// "Forbidden": `STR#` 2002 #173.
pub const FORBIDDEN: &str = "Forbidden";

/// "Hostile": `STR#` 2002 #174.
pub const HOSTILE: &str = "Hostile";

impl PlanetStatus {
    /// What it says.
    #[must_use]
    pub fn text(self) -> &'static str {
        match self {
            Self::Owned => OWNED,
            Self::Dominated => DOMINATED_STATUS,
            Self::Forbidden => FORBIDDEN,
            Self::Hostile => HOSTILE,
        }
    }
}

/// What the planet comm dialog shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanetCommView {
    /// The stellar hailed, whose sprite item 5 shows.
    pub stellar: StellarId,
    /// Its name, item 6's first line.
    pub name: String,
    /// Its type label, item 6's second line (`nova_sim_ai::planet_comm::type_label`).
    pub type_label: String,
    /// Its reply, item 4.
    pub reply: String,
    /// Item 2.
    pub second: PlanetCommButton,
    /// Item 3: none while hidden.
    pub third: Option<PlanetCommButton>,
    /// The status line, if any.
    pub status: Option<PlanetStatus>,
    /// The bribe's price, while the haggle dialog asks it.
    pub asking: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_buttons_and_statuses_say_their_strings() {
        assert_eq!(
            [
                PlanetCommButton::Greetings,
                PlanetCommButton::OfferBribe,
                PlanetCommButton::DemandTribute,
                PlanetCommButton::Release,
            ]
            .map(PlanetCommButton::label),
            ["Greetings", "Offer Bribe", "Demand Tribute", "Release"]
        );
        assert_eq!(
            [
                PlanetStatus::Owned,
                PlanetStatus::Dominated,
                PlanetStatus::Forbidden,
                PlanetStatus::Hostile,
            ]
            .map(PlanetStatus::text),
            ["Owned", "Dominated", "Forbidden", "Hostile"]
        );
    }
}
