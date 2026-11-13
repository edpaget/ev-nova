//! What the status bar's cargo area lists, as `_DrawStatusCargo`
//! (@0x4c313-0x4c85c) in the `EV Nova` executable decides it. The view
//! draws it (`nova_view::flight::hud`); the session gathers the facts
//! (`Session::status_cargo`).
//!
//! - **Commodity rows** (@0x4c422-0x4c4d2): each of the 6 standard
//!   commodities owns a row, the row step advancing for every one
//!   (@0x4c4c6), so a commodity not held (tons of 0 or below, @0x4c430)
//!   leaves its row empty. A held one shows its label, `STR#` 4003
//!   "Status Cargo" #1-#6 (loaded by `_LoadStrings` @0x71b57-0x71ba2),
//!   and its tons. `jünk` has no row.
//! - **Free** (@0x4c4d8-0x4c585): `_TotalFleetHolds` less `_TotalCargo`,
//!   0 when the hold is over its capacity (the `cmovs` @0x4c572).
//! - **Special** (@0x4c58a-0x4c6f3): A counts the missions, in slot
//!   order, with their cargo aboard (+0x35) and a cargo type (+0x12 not
//!   −1), keeping the first one's type; B counts the `jünk` held, in ID
//!   order, keeping the first. With A or B above 0 the line shows: for
//!   A == 1, the kept type's name, entry type + 1 of `STR#` 4002 "Cargo
//!   Abbreviations" (@0x4c681-0x4c698, the 256-entry table `_LoadStrings`
//!   fills @0x71c91-0x71cde); else for B == 1, the kept `jünk`'s `Abbrev`
//!   (@0x4c69a-0x4c6dd); else "Multiple" (@0x4c6e4). So A == 1 wins over
//!   any B, and A ≥ 2 with B == 1 shows the `jünk`'s name. A type with no
//!   entry (past the list's end, or outside the original's 256-entry
//!   table, which the original reads past) names nothing.
//! - **Credits** (@0x4c6f8-0x4c7ad): the player's cash; the view writes
//!   it as `_DrawCommaNumber` does.
//!
//! The original's 16-bit tons, free space and 32-bit cash are kept
//! wider. Nothing here plays a sound.

/// How many commodity rows the area has: the standard commodities
/// (`cmpl $0x6` @0x4c4cf).
pub const COMMODITY_ROWS: usize = 6;

/// `STR#` 4003, the commodity rows' labels.
pub const STATUS_CARGO_STRINGS: i16 = 4003;
/// `STR#` 4002, the names a mission's cargo type shows under "Special:".
pub const CARGO_ABBREVIATIONS: i16 = 4002;

/// One mission slot's cargo, as the "Special:" count reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MissionCargo {
    /// Whether its cargo is aboard (+0x35).
    pub aboard: bool,
    /// Its cargo type, raw: `None` for −1.
    pub cargo_type: Option<i16>,
}

/// What the cargo area is decided from, in the core's terms.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CargoFacts {
    /// The tons held of each standard commodity, by number.
    pub commodity_tons: [i64; COMMODITY_ROWS],
    /// The commodity rows' labels, `STR#` 4003 in order.
    pub commodity_labels: Vec<String>,
    /// The fleet's cargo space (`_TotalFleetHolds`).
    pub fleet_holds: i64,
    /// The tons the ship holds (`_TotalCargo`).
    pub total_cargo: i64,
    /// The missions held, in slot order.
    pub missions: Vec<MissionCargo>,
    /// The mission cargo types' names, `STR#` 4002 in order.
    pub cargo_names: Vec<String>,
    /// Each `jünk` the ship holds any of, in ID order: its `Abbrev` and
    /// the tons held.
    pub junk: Vec<(String, i64)>,
    /// The player's cash.
    pub cash: i64,
}

/// A held commodity's row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommodityRow {
    /// Its label (`STR#` 4003).
    pub label: String,
    /// The tons held.
    pub tons: i64,
}

/// What "Special:" names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Special {
    /// One mission's cargo type, or one `jünk`, by name.
    Named(String),
    /// "Multiple" (`STR#` 2002 #21).
    Multiple,
}

/// What the cargo area lists (see the module docs).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StatusCargo {
    /// The commodity rows, by commodity number: `None` for one not held.
    pub rows: [Option<CommodityRow>; COMMODITY_ROWS],
    /// The tons free.
    pub free: i64,
    /// What "Special:" names, or `None` when the line is not drawn.
    pub special: Option<Special>,
    /// The player's cash.
    pub cash: i64,
}

/// What the cargo area lists of `facts` (see the module docs).
#[must_use]
pub fn status_cargo(facts: &CargoFacts) -> StatusCargo {
    StatusCargo {
        rows: std::array::from_fn(|n| {
            let tons = facts.commodity_tons[n];
            (tons > 0).then(|| CommodityRow {
                label: facts.commodity_labels.get(n).cloned().unwrap_or_default(),
                tons,
            })
        }),
        free: (facts.fleet_holds - facts.total_cargo).max(0),
        special: special(facts),
        cash: facts.cash,
    }
}

/// What "Special:" names, if the line is drawn (@0x4c58a-0x4c6f3).
fn special(facts: &CargoFacts) -> Option<Special> {
    let mut types = facts
        .missions
        .iter()
        .filter(|mission| mission.aboard)
        .filter_map(|mission| mission.cargo_type);
    let mut junk = facts.junk.iter().filter(|(_, tons)| *tons > 0);
    let (first_type, missions) = (types.next(), types.count());
    let (first_junk, junks) = (junk.next(), junk.count());
    match (first_type, missions, first_junk, junks) {
        (None, _, None, _) => None,
        (Some(cargo_type), 0, _, _) => Some(Special::Named(cargo_name(facts, cargo_type))),
        (_, _, Some((name, _)), 0) => Some(Special::Named(name.clone())),
        _ => Some(Special::Multiple),
    }
}

/// Cargo type `cargo_type`'s name: entry `cargo_type` + 1 of `STR#` 4002,
/// empty for one with no entry.
fn cargo_name(facts: &CargoFacts, cargo_type: i16) -> String {
    u8::try_from(cargo_type)
        .ok()
        .and_then(|index| facts.cargo_names.get(usize::from(index)))
        .cloned()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(list: &[&str]) -> Vec<String> {
        list.iter().map(|&s| s.to_owned()).collect()
    }

    /// Stock's labels and cargo names, a 50-ton hold with nothing in it,
    /// and no cash.
    fn facts() -> CargoFacts {
        CargoFacts {
            commodity_labels: strings(&["Food:", "Ind:", "Med:", "LuxG:", "Met:", "Equ:"]),
            cargo_names: strings(&[
                "Food",
                "IndGds",
                "MedGds",
                "LuxGds",
                "Metal",
                "Equpt",
                "Passengers",
            ]),
            fleet_holds: 50,
            ..CargoFacts::default()
        }
    }

    fn aboard(cargo_type: i16) -> MissionCargo {
        MissionCargo {
            aboard: true,
            cargo_type: Some(cargo_type),
        }
    }

    fn junk(name: &str, tons: i64) -> (String, i64) {
        (name.to_owned(), tons)
    }

    fn special_of(missions: Vec<MissionCargo>, junk: Vec<(String, i64)>) -> Option<Special> {
        status_cargo(&CargoFacts {
            missions,
            junk,
            ..facts()
        })
        .special
    }

    fn named(name: &str) -> Special {
        Special::Named(name.to_owned())
    }

    #[test]
    fn each_held_commodity_has_its_own_row_and_the_rest_stay_empty() {
        let cargo = status_cargo(&CargoFacts {
            commodity_tons: [0, 12, 0, 0, -3, 1],
            ..facts()
        });
        let row = |label: &str, tons| {
            Some(CommodityRow {
                label: label.to_owned(),
                tons,
            })
        };
        assert_eq!(
            cargo.rows,
            [None, row("Ind:", 12), None, None, None, row("Equ:", 1)]
        );
    }

    #[test]
    fn a_row_with_no_label_in_the_list_is_labelled_with_nothing() {
        let cargo = status_cargo(&CargoFacts {
            commodity_tons: [0, 0, 0, 0, 0, 4],
            commodity_labels: strings(&["Food:"]),
            ..facts()
        });
        assert_eq!(
            cargo.rows[5],
            Some(CommodityRow {
                label: String::new(),
                tons: 4
            })
        );
    }

    #[test]
    fn junk_has_no_row() {
        let cargo = status_cargo(&CargoFacts {
            junk: vec![junk("Opals", 5)],
            ..facts()
        });
        assert_eq!(cargo.rows, StatusCargo::default().rows);
    }

    #[test]
    fn free_space_is_the_holds_less_the_cargo_and_never_below_zero() {
        let free = |fleet_holds, total_cargo| {
            status_cargo(&CargoFacts {
                fleet_holds,
                total_cargo,
                ..facts()
            })
            .free
        };
        assert_eq!(free(50, 12), 38);
        assert_eq!(free(50, 50), 0);
        assert_eq!(free(50, 61), 0);
    }

    #[test]
    fn no_mission_cargo_and_no_junk_draw_no_special_line() {
        assert_eq!(special_of(Vec::new(), Vec::new()), None);
    }

    #[test]
    fn one_missions_cargo_aboard_is_named_from_the_abbreviations() {
        assert_eq!(
            special_of(vec![aboard(6)], Vec::new()),
            Some(named("Passengers"))
        );
        assert_eq!(
            special_of(vec![aboard(1)], Vec::new()),
            Some(named("IndGds"))
        );
    }

    #[test]
    fn a_type_with_no_entry_names_nothing() {
        for cargo_type in [7, 256, 1000, -2] {
            assert_eq!(
                special_of(vec![aboard(cargo_type)], Vec::new()),
                Some(named("")),
                "{cargo_type}"
            );
        }
    }

    #[test]
    fn a_type_past_the_256_entry_table_names_nothing_even_with_a_longer_list() {
        let long = CargoFacts {
            cargo_names: (0..300).map(|n| n.to_string()).collect(),
            ..facts()
        };
        let one = |cargo_type| {
            status_cargo(&CargoFacts {
                missions: vec![aboard(cargo_type)],
                ..long.clone()
            })
            .special
        };
        assert_eq!(one(255), Some(named("255")));
        assert_eq!(one(256), Some(named("")));
    }

    #[test]
    fn two_missions_cargo_is_multiple() {
        assert_eq!(
            special_of(vec![aboard(6), aboard(2)], Vec::new()),
            Some(Special::Multiple)
        );
    }

    #[test]
    fn a_mission_without_cargo_aboard_or_without_a_cargo_type_is_not_counted() {
        let missions = vec![
            MissionCargo {
                aboard: false,
                cargo_type: Some(2),
            },
            MissionCargo {
                aboard: true,
                cargo_type: None,
            },
            aboard(6),
        ];
        assert_eq!(special_of(missions, Vec::new()), Some(named("Passengers")));
    }

    #[test]
    fn the_first_slots_cargo_is_named() {
        let missions = vec![
            MissionCargo {
                aboard: false,
                cargo_type: Some(2),
            },
            aboard(3),
        ];
        assert_eq!(special_of(missions, Vec::new()), Some(named("LuxGds")));
    }

    #[test]
    fn one_junk_alone_is_named_by_its_abbrev() {
        assert_eq!(
            special_of(Vec::new(), vec![junk("Opals", 3)]),
            Some(named("Opals"))
        );
    }

    #[test]
    fn junk_with_no_tons_is_not_counted() {
        assert_eq!(special_of(Vec::new(), vec![junk("Opals", 0)]), None);
        assert_eq!(
            special_of(Vec::new(), vec![junk("Opals", 0), junk("Ice", 2)]),
            Some(named("Ice"))
        );
    }

    #[test]
    fn two_junks_are_multiple() {
        assert_eq!(
            special_of(Vec::new(), vec![junk("Opals", 3), junk("Ice", 1)]),
            Some(Special::Multiple)
        );
    }

    #[test]
    fn one_missions_cargo_wins_over_any_junk() {
        assert_eq!(
            special_of(vec![aboard(0)], vec![junk("Opals", 3)]),
            Some(named("Food"))
        );
        assert_eq!(
            special_of(vec![aboard(0)], vec![junk("Opals", 3), junk("Ice", 1)]),
            Some(named("Food"))
        );
    }

    #[test]
    fn several_missions_cargo_and_one_junk_name_the_junk() {
        assert_eq!(
            special_of(vec![aboard(0), aboard(6)], vec![junk("Opals", 3)]),
            Some(named("Opals"))
        );
    }

    #[test]
    fn several_missions_cargo_and_several_junks_are_multiple() {
        assert_eq!(
            special_of(
                vec![aboard(0), aboard(6)],
                vec![junk("Opals", 3), junk("Ice", 1)]
            ),
            Some(Special::Multiple)
        );
    }

    #[test]
    fn the_cash_is_passed_through() {
        let cargo = status_cargo(&CargoFacts {
            cash: 1_234_567,
            ..facts()
        });
        assert_eq!(cargo.cash, 1_234_567);
    }
}
