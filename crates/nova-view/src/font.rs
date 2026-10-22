//! The game's interface fonts.

/// Which font a run of text is drawn in. EV Nova's interface uses Geneva
/// for most text and Charcoal for headings, and Times for some titles;
/// the renderer decides which real face draws each.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Font {
    /// Geneva, the default.
    #[default]
    Geneva,
    /// Charcoal.
    Charcoal,
    /// Times: QuickDraw font family 20, which titles the Shipyard Info
    /// panel (`_ShipDescDialogUpdate` `TextFont(0x14)` @0x4f9f7). The
    /// renderer draws it in the bundled Noto Serif. No resource names it:
    /// [`Font::named`] never gives it.
    Times,
}

impl Font {
    /// The font a game resource names (the `cölr` and `ïntf` font fields):
    /// "Charcoal", in any case, is [`Font::Charcoal`], and every other name
    /// [`Font::Geneva`].
    #[must_use]
    pub fn named(name: &str) -> Self {
        if name.eq_ignore_ascii_case("Charcoal") {
            Self::Charcoal
        } else {
            Self::Geneva
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn charcoal_is_named_in_any_case() {
        for name in ["Charcoal", "charcoal", "CHARCOAL", "cHaRcOaL"] {
            assert_eq!(Font::named(name), Font::Charcoal, "{name}");
        }
    }

    #[test]
    fn every_other_name_is_geneva() {
        for name in ["Geneva", "geneva", "", "Chicago", "Charcoal ", "Charcoa"] {
            assert_eq!(Font::named(name), Font::Geneva, "{name:?}");
        }
    }

    #[test]
    fn times_is_font_20_and_not_named_by_resources() {
        assert_ne!(Font::Times, Font::Geneva);
        assert_ne!(Font::Times, Font::Charcoal);
        for name in ["Times", "times", "TIMES"] {
            assert_eq!(Font::named(name), Font::Geneva, "{name}");
        }
    }

    #[test]
    fn the_default_is_geneva() {
        assert_eq!(Font::default(), Font::Geneva);
    }
}
