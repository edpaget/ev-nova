//! A number of tons as a shop's info box shows it ([`ShownTons`],
//! [`TonUnit`]), from `nova_sim_port::wares`, which re-exports them and
//! keeps the rules the outfitter and the shipyard share.

/// The unit a shop's info box puts after a number of tons, as
/// `_LoadStrings` loads them @0x71ce0-0x71d15.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TonUnit {
    /// `_tonString` (`STR#` 2002 #1, "ton").
    Ton,
    /// `_tonsString` (`STR#` 2002 #2, "tons").
    Tons,
}

impl TonUnit {
    /// The unit after `tons`: [`TonUnit::Ton`] at exactly 1, else
    /// [`TonUnit::Tons`].
    #[must_use]
    pub fn of(tons: i64) -> Self {
        if tons == 1 { Self::Ton } else { Self::Tons }
    }
}

/// A number of tons as a shop's info box shows it, with its unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShownTons {
    /// The number shown.
    pub tons: i64,
    /// The unit after it.
    pub unit: TonUnit,
}

impl ShownTons {
    /// `tons` with the unit [`TonUnit::of`] picks for it.
    #[must_use]
    pub fn of(tons: i64) -> Self {
        Self {
            tons,
            unit: TonUnit::of(tons),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ton_count_takes_the_singular_unit_at_exactly_one() {
        assert_eq!(TonUnit::of(1), TonUnit::Ton);
        for tons in [0, 2, -1] {
            assert_eq!(TonUnit::of(tons), TonUnit::Tons, "{tons}");
        }
        assert_eq!(
            ShownTons::of(1),
            ShownTons {
                tons: 1,
                unit: TonUnit::Ton
            }
        );
    }
}
