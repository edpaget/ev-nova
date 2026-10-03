//! The game's two interface fonts.

/// Which font a run of text is drawn in. EV Nova's interface uses Geneva
/// for most text and Charcoal for headings; the renderer decides which
/// real face draws each.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Font {
    /// Geneva, the default.
    #[default]
    Geneva,
    /// Charcoal.
    Charcoal,
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
    fn the_default_is_geneva() {
        assert_eq!(Font::default(), Font::Geneva);
    }
}
