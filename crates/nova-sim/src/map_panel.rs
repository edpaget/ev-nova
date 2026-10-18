//! The galaxy map's two info panels, as `_MapDialogUpdate`
//! (@0xf5d1-0x1128c) decides them: what each line says about the system
//! selected, from a [`PanelInput`] the session gathers
//! (`nova_sim::Session::map_panel`). The view lays the lines out at
//! DITL 2000's offsets, measuring the text (the Ports wrap, the prompt's
//! centring and the date's right edge need widths).
//!
//! **The bottom strip** (DITL 2000 item 2, @0x10136-0x10a85):
//!
//! - On the hypergate map with the current system or nothing selected,
//!   the prompt (#308) and the date alone (@0x10213-0x102cd).
//! - Otherwise "Ports:" ([`Ports`], @0x102d2-0x1057a): the selected
//!   system's `NavDefs` stellars, raw, in slot order, each not
//!   uninhabited (`Flags` 0x20) and `_StellarIsLandable`
//!   ([`landable`]); neither the active byte nor the gate bits are read,
//!   so a landable hypergate is listed, and so is a stellar another
//!   system owns. "None" when none is; `<Unknown>` when the system is
//!   unexplored.
//! - "Navigation Hazards:" ([`hazards`], @0x10586-0x109aa): "gravity
//!   shear" when any stellar has a `Gravity` (all 16 slots, active or
//!   not, @0x1061f-0x106a9); the asteroid field by `Asteroids` (≤ 3
//!   sparse, ≤ 6 moderate, else dense, @0x106af-0x1074f); the
//!   interference by `Interference` (≤ 33 light, ≤ 66 moderate, else
//!   heavy, @0x10793-0x10833); the visibility by `Murk` (≤ 30 reduced,
//!   ≤ 60 severely reduced, else greatly reduced, @0x1086c-0x1090c). Each
//!   part counts only above 0, is followed by ", " when a later one
//!   follows, and the first character is upper-cased (`toupper`
//!   @0x10942); "None" when there is none (@0x10911-0x10933).
//! - The date (`DateToMedString`, @0x109af-0x10a6f).
//!
//! **The right column** (DITL 2000 item 6, @0x10a88-0x111bf):
//!
//! - The title ([`title`], @0x10b3f-0x10c76): in nav mode 3 with a
//!   selection, "Destination System:" whatever system the map selects;
//!   in mode 3 with none, "Current System:" for the current system, else
//!   "Selected System:"; in any other mode, "Current System:" for the
//!   current system, "Hypergate Destination:" for a system the hypergate
//!   map's gate offers (owner-first, as
//!   [`gate::hypergate_offers`](crate::gate::hypergate_offers) resolves
//!   them), else "Selected System:".
//! - Unexplored: the name reads `<Unknown>` and nothing more
//!   (@0x10cc5-0x10cd7). Not `_SystemIsInhabited` (read under
//!   `RuleKey::MapExplore` by the session): the name and "Uninhabited
//!   System" (@0x11183-0x111ba). Otherwise the name, the government
//!   ("Independent" for none, @0x10d53-0x10e2f), the legal status
//!   (`_DrawLegalStatusString` of the selected system, @0x10e34-0x10ed8),
//!   the goods traded ([`system_trades`], @0x10edd-0x10ff0) and the
//!   services ([`system_has_service`], @0x10ff5-0x111ba).
//!
//! **The explored level** tells only the goods and the services apart: at
//! level 1 both read `<Unknown>` (@0x10f34, @0x1107d, @0x11110), at level
//! 2 they are listed. The services' `<Unknown>` sits a pixel higher when
//! the system has no service ([`Services::Unknown`]'s `any`).
//!
//! With nothing selected (`_selSystem` −1) off the hypergate map the
//! original reads system −1, out of bounds (@0x10339-0x1035d,
//! @0x10ca3-0x10cc5); here no selection reads as an unexplored system.
//!
//! The function calls no `PlaySound` and makes no random draw. Its
//! `_cheatsActive` numbers, visible-region tests, partial-update skip and
//! `_didUpdateMap` are left out (rdm:task/map-dialog-update-left-out-branches).

use crate::catalog::SystemId;
use crate::exploration::{GATES, UNINHABITED};
use crate::landing::landable;
use crate::market::price_level;
use crate::navigation::NavMode;
use crate::pilot::ExploredLevel;

/// The `spöb` `Flags` bit of a stellar with a commodity exchange.
const TRADES: u32 = 0x0002;
/// The `spöb` `Flags` bits of each service, in the panel's order:
/// trading, outfitting and the shipyard (`_SystemHasService`
/// @0xd373-0xd43e).
const SERVICE_FLAGS: [u32; 3] = [0x0002, 0x0004, 0x0008];
/// How many standard commodities the panel asks about.
const COMMODITIES: u8 = 6;

/// The `STR#` 2002 strings the panels show (#308-#340).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PanelWords {
    /// #308, the hypergate map's prompt.
    pub hypergate_prompt: String,
    /// #309 "Ports:".
    pub ports: String,
    /// #310 `<Unknown>`.
    pub unknown: String,
    /// #311 "Navigation Hazards:".
    pub hazards: String,
    /// #312 "gravity shear".
    pub gravity_shear: String,
    /// #313-#315: "sparse", "moderate", "dense".
    pub asteroid_sizes: [String; 3],
    /// #316 "asteroid field".
    pub asteroid_field: String,
    /// #317-#319: "light", "moderate", "heavy".
    pub interference_levels: [String; 3],
    /// #320 "interference".
    pub interference: String,
    /// #321-#323: "reduced", "severely reduced", "greatly reduced".
    pub visibility_levels: [String; 3],
    /// #324 "visibility".
    pub visibility: String,
    /// #325 "Government:".
    pub government: String,
    /// #326 "Legal Status:".
    pub legal_status: String,
    /// #327 "Goods Traded:".
    pub goods_traded: String,
    /// #328 "Services:".
    pub services: String,
    /// #329-#331: "Trading", "Outfitting", "Shipyard".
    pub service_names: [String; 3],
    /// #332 "None Available".
    pub none_available: String,
    /// #333 "Independent".
    pub independent: String,
    /// #334 "Uninhabited System".
    pub uninhabited: String,
    /// #335 "None".
    pub none: String,
    /// #337 "Current System:".
    pub current: String,
    /// #338 "Selected System:".
    pub selected: String,
    /// #339 "Destination System:".
    pub destination: String,
    /// #340 "Hypergate Destination:".
    pub hypergate_destination: String,
}

impl PanelWords {
    /// The words `STR#` 2002 `list` holds, each read by its string number
    /// (1-based), empty when the list lacks it.
    #[must_use]
    pub fn from_list(list: &[String]) -> Self {
        let at = |n: usize| list.get(n - 1).cloned().unwrap_or_default();
        let three = |n: usize| [at(n), at(n + 1), at(n + 2)];
        Self {
            hypergate_prompt: at(308),
            ports: at(309),
            unknown: at(310),
            hazards: at(311),
            gravity_shear: at(312),
            asteroid_sizes: three(313),
            asteroid_field: at(316),
            interference_levels: three(317),
            interference: at(320),
            visibility_levels: three(321),
            visibility: at(324),
            government: at(325),
            legal_status: at(326),
            goods_traded: at(327),
            services: at(328),
            service_names: three(329),
            none_available: at(332),
            independent: at(333),
            uninhabited: at(334),
            none: at(335),
            current: at(337),
            selected: at(338),
            destination: at(339),
            hypergate_destination: at(340),
        }
    }
}

/// One of the selected system's `NavDefs` stellars, as the panel reads it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PanelStellar {
    /// Its name.
    pub name: String,
    /// Its `Flags`, raw.
    pub flags: u32,
    /// Its `Flags2`, raw.
    pub flags2: u16,
    /// Its `Gravity`, raw.
    pub gravity: i16,
    /// Whether it is active (+0x44): its holder is visible.
    pub active: bool,
    /// Whether it is destroyed.
    pub destroyed: bool,
}

impl PanelStellar {
    /// Whether it is uninhabited (`Flags` 0x20).
    fn uninhabited(&self) -> bool {
        self.flags & UNINHABITED != 0
    }

    /// Whether `_StellarIsLandable` takes it (@0xa989-0xa9f7).
    fn landable(&self) -> bool {
        landable(self.flags, self.destroyed)
    }

    /// The stellar test `_SystemTradesGoods` and `_SystemHasService`
    /// share: active, not uninhabited, landable and no gate
    /// (@0xd30c-0xd33b).
    fn serves(&self) -> bool {
        self.active && !self.uninhabited() && self.landable() && self.flags2 & GATES == 0
    }
}

/// What the panels are decided from.
#[derive(Clone, Copy, Debug)]
pub struct PanelInput<'a> {
    /// The `STR#` 2002 words.
    pub words: &'a PanelWords,
    /// The system the map selects (`_selSystem`), if any.
    pub selected: Option<SystemId>,
    /// The player's system (player +0x74).
    pub current: SystemId,
    /// The player's nav mode and its selection (+0x6a, +0x6c).
    pub nav: NavMode,
    /// On the hypergate map, the systems its gate offers; `None` on any
    /// other map.
    pub hypergate: Option<&'a [SystemId]>,
    /// The selected system's explored level.
    pub level: ExploredLevel,
    /// The selected system's name.
    pub name: &'a str,
    /// Its government's name, `None` when it is independent.
    pub government: Option<&'a str>,
    /// Its `Asteroids`, as its asteroid field spawns from it.
    pub asteroids: i16,
    /// Its `Interference`, raw.
    pub interference: i16,
    /// Its `Murk` as loaded ([`crate::murk::loaded_murk`]), the runtime
    /// +0xbc the panel reads.
    pub murk: i16,
    /// Its `NavDefs` stellars, in slot order.
    pub stellars: &'a [PanelStellar],
    /// Whether it is inhabited (`_SystemIsInhabited`, as the session reads
    /// it).
    pub inhabited: bool,
    /// The legal status string in it.
    pub legal: &'a str,
    /// The standard commodities' names, by number.
    pub commodities: &'a [String],
    /// Today's date, as `DateToMedString` writes it.
    pub date: &'a str,
}

/// Both panels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapPanel {
    /// The bottom strip, DITL 2000 item 2.
    pub strip: Strip,
    /// The right column, DITL 2000 item 6.
    pub column: Column,
    /// The `STR#` 2002 words, for the labels and the `<Unknown>`s.
    pub words: PanelWords,
}

/// The bottom strip.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Strip {
    /// The hypergate map's prompt, and the date.
    HypergatePrompt {
        /// #308.
        prompt: String,
        /// Today's date.
        date: String,
    },
    /// The ports, the hazards and the date.
    Info {
        /// The ports.
        ports: Ports,
        /// The hazards' value: `<Unknown>` when unexplored.
        hazards: String,
        /// Today's date.
        date: String,
    },
}

/// The Ports line's value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ports {
    /// The stellars' names, in slot order.
    Listed(Vec<String>),
    /// "None": no stellar qualifies.
    None,
    /// `<Unknown>`: the system is unexplored.
    Unknown,
}

/// The right column.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Column {
    /// The title, one of #337-#340.
    pub title: String,
    /// What follows it.
    pub body: ColumnBody,
}

/// What the right column holds under its title.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ColumnBody {
    /// Unexplored: the name reads `<Unknown>` and nothing more.
    Unknown,
    /// Not inhabited: the name, then "Uninhabited System".
    Uninhabited {
        /// The system's name.
        name: String,
    },
    /// Inhabited: every row.
    Inhabited {
        /// The system's name.
        name: String,
        /// Its government's name, or "Independent".
        government: String,
        /// The legal status string.
        legal: String,
        /// The goods traded.
        goods: Listing,
        /// The services.
        services: Services,
    },
}

/// The Goods Traded rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Listing {
    /// The commodities' names, in commodity order.
    Items(Vec<String>),
    /// "None".
    None,
    /// `<Unknown>`: explored at level 1.
    Unknown,
}

/// The Services rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Services {
    /// The services' names, in order.
    Items(Vec<String>),
    /// "None Available".
    NoneAvailable,
    /// `<Unknown>`: explored at level 1. `any` is whether the system has
    /// a service: the original draws it 13 below the label with one (the
    /// first service's row, @0x1107d) and 12 below without (the "None
    /// Available" row, @0x11110).
    Unknown {
        /// Whether the system has a service.
        any: bool,
    },
}

/// Whether a system of these `stellars` trades `commodity`, as
/// `_SystemTradesGoods` (@0xd2c4-0xd372) reads it: one of them is active,
/// not uninhabited, landable and no gate, with the commodity exchange
/// (`Flags` 0x02) and a price level for it (`_CalcPortDemand`).
#[must_use]
pub fn system_trades(stellars: &[PanelStellar], commodity: u8) -> bool {
    stellars.iter().any(|stellar| {
        stellar.serves()
            && stellar.flags & TRADES != 0
            && price_level(stellar.flags, commodity).is_some()
    })
}

/// Whether a system of these `stellars` has service `service` (0
/// trading, 1 outfitting, 2 the shipyard), as `_SystemHasService`
/// (@0xd373-0xd43e) reads it: one of them is active, not uninhabited,
/// landable and no gate, with the service's `Flags` bit.
#[must_use]
pub fn system_has_service(stellars: &[PanelStellar], service: usize) -> bool {
    SERVICE_FLAGS.get(service).is_some_and(|&bit| {
        stellars
            .iter()
            .any(|stellar| stellar.serves() && stellar.flags & bit != 0)
    })
}

/// Both panels for `input` (see the module docs).
#[must_use]
pub fn map_panel(input: &PanelInput) -> MapPanel {
    MapPanel {
        strip: strip(input),
        column: Column {
            title: title(input).to_owned(),
            body: column_body(input),
        },
        words: input.words.clone(),
    }
}

/// Whether the selected system reads as explored: nothing selected reads
/// as unexplored (see the module docs).
fn explored(input: &PanelInput) -> bool {
    input.selected.is_some() && input.level > ExploredLevel::Unexplored
}

/// The bottom strip (@0x10213-0x10a6f).
fn strip(input: &PanelInput) -> Strip {
    let date = input.date.to_owned();
    let prompt = input.hypergate.is_some()
        && input
            .selected
            .is_none_or(|selected| selected == input.current);
    if prompt {
        return Strip::HypergatePrompt {
            prompt: input.words.hypergate_prompt.clone(),
            date,
        };
    }
    Strip::Info {
        ports: ports(input),
        hazards: hazards(input),
        date,
    }
}

/// The Ports line's value (@0x1035d-0x10566).
fn ports(input: &PanelInput) -> Ports {
    if !explored(input) {
        return Ports::Unknown;
    }
    let names: Vec<String> = input
        .stellars
        .iter()
        .filter(|stellar| !stellar.uninhabited() && stellar.landable())
        .map(|stellar| stellar.name.clone())
        .collect();
    if names.is_empty() {
        Ports::None
    } else {
        Ports::Listed(names)
    }
}

/// The word of `levels` for `value` by its two upper bounds, or `None`
/// when it is not above 0.
fn graded(value: i16, bounds: [i16; 2], levels: &[String; 3]) -> Option<&str> {
    if value <= 0 {
        None
    } else if value <= bounds[0] {
        Some(&levels[0])
    } else if value <= bounds[1] {
        Some(&levels[1])
    } else {
        Some(&levels[2])
    }
}

/// The Navigation Hazards line's value (@0x10602-0x109aa; see the module
/// docs).
#[must_use]
pub fn hazards(input: &PanelInput) -> String {
    let words = input.words;
    if !explored(input) {
        return words.unknown.clone();
    }
    let mut parts = Vec::new();
    if input.stellars.iter().any(|stellar| stellar.gravity != 0) {
        parts.push(words.gravity_shear.clone());
    }
    let graded_parts = [
        (
            input.asteroids,
            [3, 6],
            &words.asteroid_sizes,
            &words.asteroid_field,
        ),
        (
            input.interference,
            [33, 66],
            &words.interference_levels,
            &words.interference,
        ),
        (
            input.murk,
            [30, 60],
            &words.visibility_levels,
            &words.visibility,
        ),
    ];
    for (value, bounds, levels, noun) in graded_parts {
        if let Some(level) = graded(value, bounds, levels) {
            parts.push(format!("{level} {noun}"));
        }
    }
    let text = if parts.is_empty() {
        words.none.clone()
    } else {
        parts.join(", ")
    };
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_ascii_uppercase().to_string() + chars.as_str()
    })
}

/// The right column's title (@0x10b3f-0x10c76; see the module docs).
#[must_use]
pub fn title<'a>(input: &PanelInput<'a>) -> &'a str {
    let words = input.words;
    let current = input.selected == Some(input.current);
    match input.nav {
        NavMode::Hyperspace(Some(_)) => &words.destination,
        NavMode::Hyperspace(None) if current => &words.current,
        NavMode::Hyperspace(None) => &words.selected,
        _ if current => &words.current,
        _ if input
            .hypergate
            .zip(input.selected)
            .is_some_and(|(offers, selected)| offers.contains(&selected)) =>
        {
            &words.hypergate_destination
        }
        _ => &words.selected,
    }
}

/// What the right column holds under its title (@0x10cc5-0x111ba).
fn column_body(input: &PanelInput) -> ColumnBody {
    let words = input.words;
    if !explored(input) {
        return ColumnBody::Unknown;
    }
    let name = input.name.to_owned();
    if !input.inhabited {
        return ColumnBody::Uninhabited { name };
    }
    let charted = input.level == ExploredLevel::Charted;
    let goods: Vec<String> = (0..COMMODITIES)
        .filter(|&commodity| system_trades(input.stellars, commodity))
        .map(|commodity| {
            input
                .commodities
                .get(usize::from(commodity))
                .cloned()
                .unwrap_or_default()
        })
        .collect();
    let services: Vec<String> = (0..SERVICE_FLAGS.len())
        .filter(|&service| system_has_service(input.stellars, service))
        .map(|service| words.service_names[service].clone())
        .collect();
    ColumnBody::Inhabited {
        name,
        government: input
            .government
            .map_or_else(|| words.independent.clone(), str::to_owned),
        legal: input.legal.to_owned(),
        goods: match (charted, goods.is_empty()) {
            (false, _) => Listing::Unknown,
            (true, true) => Listing::None,
            (true, false) => Listing::Items(goods),
        },
        services: match (charted, services.is_empty()) {
            (false, empty) => Services::Unknown { any: !empty },
            (true, true) => Services::NoneAvailable,
            (true, false) => Services::Items(services),
        },
    }
}

#[cfg(test)]
mod tests {
    use std::sync::LazyLock;

    use super::*;
    use crate::catalog::StellarId;

    /// Each word its own number, "#n".
    fn words() -> PanelWords {
        let list: Vec<String> = (1..=340).map(|n| format!("#{n}")).collect();
        PanelWords::from_list(&list)
    }

    /// An inhabited, landable stellar named `name`, active, with every
    /// service and every commodity at a low price.
    fn port(name: &str) -> PanelStellar {
        PanelStellar {
            name: name.to_owned(),
            flags: 0x1111_1100 | 0x0001 | 0x0002 | 0x0004 | 0x0008,
            active: true,
            ..PanelStellar::default()
        }
    }

    const CURRENT: SystemId = SystemId(130);
    const OTHER: SystemId = SystemId(131);

    /// The panel for `stellars` in `OTHER`, selected, charted and
    /// inhabited, off the hypergate map with the nav system off; `change`
    /// changes the input first.
    fn panel_with<'a>(
        stellars: &'a [PanelStellar],
        change: impl FnOnce(&mut PanelInput<'a>),
    ) -> MapPanel {
        static WORDS: LazyLock<PanelWords> = LazyLock::new(words);
        static GOODS: LazyLock<Vec<String>> =
            LazyLock::new(|| (0..6).map(|n| format!("good {n}")).collect());
        let mut input = PanelInput {
            words: &WORDS,
            selected: Some(OTHER),
            current: CURRENT,
            nav: NavMode::Off,
            hypergate: None,
            level: ExploredLevel::Charted,
            name: "Sol",
            government: Some("Federation"),
            asteroids: 0,
            interference: 0,
            murk: 0,
            stellars,
            inhabited: true,
            legal: "Clean",
            commodities: &GOODS,
            date: "Jan 1, 1177",
        };
        change(&mut input);
        map_panel(&input)
    }

    fn ports_of(panel: &MapPanel) -> Ports {
        match &panel.strip {
            Strip::Info { ports, .. } => ports.clone(),
            Strip::HypergatePrompt { .. } => panic!("the prompt"),
        }
    }

    fn hazards_of(panel: &MapPanel) -> String {
        match &panel.strip {
            Strip::Info { hazards, .. } => hazards.clone(),
            Strip::HypergatePrompt { .. } => panic!("the prompt"),
        }
    }

    fn listed(names: &[&str]) -> Ports {
        Ports::Listed(names.iter().map(|&name| name.to_owned()).collect())
    }

    // The hypergate prompt.

    #[test]
    fn the_hypergate_map_prompts_for_the_current_system_and_for_no_selection() {
        let offers = [OTHER];
        for selected in [Some(CURRENT), None] {
            let panel = panel_with(&[], |input| {
                input.hypergate = Some(&offers);
                input.selected = selected;
            });
            assert_eq!(
                panel.strip,
                Strip::HypergatePrompt {
                    prompt: "#308".to_owned(),
                    date: "Jan 1, 1177".to_owned()
                },
                "{selected:?}"
            );
        }
        let panel = panel_with(&[], |input| input.hypergate = Some(&offers));
        assert!(matches!(panel.strip, Strip::Info { .. }), "an offer");
        let panel = panel_with(&[], |input| input.selected = Some(CURRENT));
        assert!(matches!(panel.strip, Strip::Info { .. }), "off the map");
    }

    // Ports.

    #[test]
    fn the_ports_leave_out_an_uninhabited_and_an_unlandable_stellar() {
        let uninhabited = PanelStellar {
            flags: 0x0001 | UNINHABITED,
            ..port("Jupiter")
        };
        let unlandable = PanelStellar {
            flags: 0x0002,
            ..port("Rock")
        };
        let panel = panel_with(
            &[port("Earth"), uninhabited, unlandable, port("Mars")],
            |_| {},
        );
        assert_eq!(ports_of(&panel), listed(&["Earth", "Mars"]));
    }

    #[test]
    fn the_ports_keep_a_landable_gate_an_inactive_stellar_and_one_another_system_owns() {
        let gate = PanelStellar {
            flags2: 0x1000,
            ..port("HG-Primus")
        };
        let inactive = PanelStellar {
            active: false,
            ..port("Far")
        };
        let panel = panel_with(&[gate, inactive], |_| {});
        assert_eq!(ports_of(&panel), listed(&["HG-Primus", "Far"]));
    }

    #[test]
    fn a_destroyed_stellar_is_a_port_exactly_when_landable_only_once_destroyed() {
        let ruin = |flags: u32| PanelStellar {
            flags,
            destroyed: true,
            ..port("Ruin")
        };
        let panel = panel_with(&[ruin(0x0001)], |_| {});
        assert_eq!(ports_of(&panel), Ports::None);
        let panel = panel_with(&[ruin(0x0081)], |_| {});
        assert_eq!(ports_of(&panel), listed(&["Ruin"]));
        let whole = PanelStellar {
            flags: 0x0081,
            ..port("Whole")
        };
        let panel = panel_with(&[whole], |_| {});
        assert_eq!(ports_of(&panel), Ports::None, "not yet destroyed");
    }

    #[test]
    fn the_ports_are_none_with_no_port_and_unknown_when_unexplored() {
        let panel = panel_with(&[], |_| {});
        assert_eq!(ports_of(&panel), Ports::None);
        let panel = panel_with(&[port("Earth")], |input| {
            input.level = ExploredLevel::Unexplored;
        });
        assert_eq!(ports_of(&panel), Ports::Unknown);
        let panel = panel_with(&[port("Earth")], |input| input.selected = None);
        assert_eq!(ports_of(&panel), Ports::Unknown, "no selection");
    }

    // Hazards.

    fn hazards_at(asteroids: i16, interference: i16, murk: i16) -> String {
        hazards_of(&panel_with(&[], |input| {
            input.asteroids = asteroids;
            input.interference = interference;
            input.murk = murk;
        }))
    }

    #[test]
    fn each_hazard_reads_its_word_by_its_thresholds() {
        let cases: [((i16, i16, i16), &str); 13] = [
            ((1, 0, 0), "#313 #316"),
            ((3, 0, 0), "#313 #316"),
            ((4, 0, 0), "#314 #316"),
            ((6, 0, 0), "#314 #316"),
            ((7, 0, 0), "#315 #316"),
            ((0, 33, 0), "#317 #320"),
            ((0, 34, 0), "#318 #320"),
            ((0, 66, 0), "#318 #320"),
            ((0, 67, 0), "#319 #320"),
            ((0, 0, 30), "#321 #324"),
            ((0, 0, 31), "#322 #324"),
            ((0, 0, 60), "#322 #324"),
            ((0, 0, 61), "#323 #324"),
        ];
        for ((asteroids, interference, murk), expected) in cases {
            assert_eq!(
                hazards_at(asteroids, interference, murk),
                expected,
                "{asteroids} {interference} {murk}"
            );
        }
        assert_eq!(hazards_at(0, 0, 0), "#335", "None");
        assert_eq!(hazards_at(-4, -1, -50), "#335", "none below 1");
    }

    #[test]
    fn the_hazards_join_with_commas_and_gravity_comes_first_from_any_slot() {
        let heavy = PanelStellar {
            gravity: -2,
            active: false,
            flags: 0,
            ..port("Hole")
        };
        let panel = panel_with(&[port("Earth"), heavy.clone()], |input| {
            input.asteroids = 2;
            input.murk = 70;
        });
        assert_eq!(hazards_of(&panel), "#312, #313 #316, #323 #324");
        let panel = panel_with(&[heavy], |_| {});
        assert_eq!(hazards_of(&panel), "#312", "gravity alone");
        assert_eq!(hazards_at(1, 1, 1), "#313 #316, #317 #320, #321 #324");
    }

    #[test]
    fn the_hazards_first_letter_is_upper_cased() {
        let words = PanelWords {
            asteroid_sizes: ["sparse".into(), "moderate".into(), "dense".into()],
            asteroid_field: "asteroid field".into(),
            interference_levels: ["light".into(), "moderate".into(), "heavy".into()],
            interference: "interference".into(),
            ..words()
        };
        let commodities = Vec::new();
        let input = PanelInput {
            words: &words,
            selected: Some(OTHER),
            current: CURRENT,
            nav: NavMode::Off,
            hypergate: None,
            level: ExploredLevel::Explored,
            name: "",
            government: None,
            asteroids: 2,
            interference: 20,
            murk: 0,
            stellars: &[],
            inhabited: false,
            legal: "",
            commodities: &commodities,
            date: "",
        };
        assert_eq!(hazards(&input), "Sparse asteroid field, light interference");
        let empty = PanelWords {
            none: String::new(),
            ..PanelWords::default()
        };
        let input = PanelInput {
            words: &empty,
            asteroids: 0,
            interference: 0,
            ..input
        };
        assert_eq!(hazards(&input), "", "nothing to upper-case");
    }

    #[test]
    fn the_hazards_are_unknown_when_unexplored() {
        let panel = panel_with(&[], |input| {
            input.level = ExploredLevel::Unexplored;
            input.asteroids = 5;
        });
        assert_eq!(hazards_of(&panel), "#310");
    }

    // Titles.

    fn title_of<'a>(change: impl FnOnce(&mut PanelInput<'a>)) -> String {
        panel_with(&[], change).column.title
    }

    #[test]
    fn nav_mode_3_with_a_selection_titles_any_system_the_destination() {
        for selected in [Some(CURRENT), Some(OTHER), None] {
            assert_eq!(
                title_of(|input| {
                    input.nav = NavMode::Hyperspace(Some(SystemId(140)));
                    input.selected = selected;
                }),
                "#339",
                "{selected:?}"
            );
        }
    }

    #[test]
    fn nav_mode_3_without_a_selection_titles_current_or_selected() {
        let mode_3 = |input: &mut PanelInput| input.nav = NavMode::Hyperspace(None);
        assert_eq!(
            title_of(|input| {
                mode_3(input);
                input.selected = Some(CURRENT);
            }),
            "#337"
        );
        assert_eq!(title_of(mode_3), "#338");
        let offers = [OTHER];
        assert_eq!(
            title_of(|input| {
                mode_3(input);
                input.hypergate = Some(&offers);
            }),
            "#338",
            "mode 3 never titles a hypergate destination"
        );
    }

    #[test]
    fn off_mode_3_the_title_is_current_then_hypergate_destination_then_selected() {
        let offers = [OTHER, CURRENT];
        for nav in [NavMode::Off, NavMode::Stellar(Some(StellarId(128)))] {
            assert_eq!(
                title_of(|input| {
                    input.nav = nav;
                    input.selected = Some(CURRENT);
                    input.hypergate = Some(&offers);
                }),
                "#337",
                "{nav:?}: current beats the gate"
            );
            assert_eq!(
                title_of(|input| {
                    input.nav = nav;
                    input.hypergate = Some(&offers);
                }),
                "#340",
                "{nav:?}"
            );
            assert_eq!(
                title_of(|input| {
                    input.nav = nav;
                    input.hypergate = Some(&[CURRENT]);
                }),
                "#338",
                "{nav:?}: not offered"
            );
            assert_eq!(
                title_of(|input| input.nav = nav),
                "#338",
                "{nav:?}: no gate"
            );
        }
    }

    #[test]
    fn no_selection_titles_selected_and_reads_unexplored() {
        let panel = panel_with(&[port("Earth")], |input| input.selected = None);
        assert_eq!(panel.column.title, "#338");
        assert_eq!(panel.column.body, ColumnBody::Unknown);
    }

    // The column.

    #[test]
    fn an_unexplored_system_reads_unknown_alone() {
        let panel = panel_with(&[port("Earth")], |input| {
            input.level = ExploredLevel::Unexplored;
        });
        assert_eq!(panel.column.body, ColumnBody::Unknown);
    }

    #[test]
    fn an_uninhabited_system_reads_its_name_and_uninhabited() {
        let panel = panel_with(&[port("Earth")], |input| input.inhabited = false);
        assert_eq!(
            panel.column.body,
            ColumnBody::Uninhabited {
                name: "Sol".to_owned()
            }
        );
    }

    fn government_of(panel: &MapPanel) -> String {
        match &panel.column.body {
            ColumnBody::Inhabited { government, .. } => government.clone(),
            body => panic!("{body:?}"),
        }
    }

    #[test]
    fn the_government_is_its_name_or_independent() {
        let panel = panel_with(&[port("Earth")], |_| {});
        assert_eq!(government_of(&panel), "Federation");
        let panel = panel_with(&[port("Earth")], |input| input.government = None);
        assert_eq!(government_of(&panel), "#333");
    }

    #[test]
    fn the_legal_status_and_name_are_the_inputs() {
        let panel = panel_with(&[port("Earth")], |_| {});
        let ColumnBody::Inhabited { name, legal, .. } = panel.column.body else {
            panic!("inhabited");
        };
        assert_eq!((name.as_str(), legal.as_str()), ("Sol", "Clean"));
    }

    // Goods and services by level.

    fn goods_and_services(panel: &MapPanel) -> (Listing, Services) {
        match &panel.column.body {
            ColumnBody::Inhabited {
                goods, services, ..
            } => (goods.clone(), services.clone()),
            body => panic!("{body:?}"),
        }
    }

    #[test]
    fn at_level_1_the_goods_and_services_are_unknown() {
        let explored = |input: &mut PanelInput| input.level = ExploredLevel::Explored;
        let panel = panel_with(&[port("Earth")], explored);
        assert_eq!(
            goods_and_services(&panel),
            (Listing::Unknown, Services::Unknown { any: true })
        );
        let bare = PanelStellar {
            flags: 0x0001,
            ..port("Bare")
        };
        let panel = panel_with(&[bare], explored);
        assert_eq!(
            goods_and_services(&panel),
            (Listing::Unknown, Services::Unknown { any: false })
        );
    }

    fn strings(names: &[&str]) -> Vec<String> {
        names.iter().map(|&name| name.to_owned()).collect()
    }

    #[test]
    fn at_level_2_the_goods_and_services_are_listed_in_order() {
        // Commodities 1 (low) and 4 (high); trading and the shipyard.
        let trader = PanelStellar {
            flags: 0x0100_4000 | 0x0001 | 0x0002 | 0x0008,
            ..port("Trader")
        };
        let panel = panel_with(&[trader], |_| {});
        assert_eq!(
            goods_and_services(&panel),
            (
                Listing::Items(strings(&["good 1", "good 4"])),
                Services::Items(strings(&["#329", "#331"]))
            )
        );
        let bare = PanelStellar {
            flags: 0x0001,
            ..port("Bare")
        };
        let panel = panel_with(&[bare], |_| {});
        assert_eq!(
            goods_and_services(&panel),
            (Listing::None, Services::NoneAvailable)
        );
    }

    #[test]
    fn a_commodity_is_traded_only_at_an_active_inhabited_landable_exchange_with_its_price() {
        let base = port("Earth");
        assert!(system_trades(std::slice::from_ref(&base), 0));
        let unpriced = PanelStellar {
            flags: 0x0000_0003,
            ..base.clone()
        };
        let no_exchange = PanelStellar {
            flags: base.flags & !TRADES,
            ..base.clone()
        };
        let inactive = PanelStellar {
            active: false,
            ..base.clone()
        };
        let uninhabited = PanelStellar {
            flags: base.flags | UNINHABITED,
            ..base.clone()
        };
        let unlandable = PanelStellar {
            flags: base.flags & !1,
            ..base.clone()
        };
        let gate = PanelStellar {
            flags2: 0x2000,
            ..base.clone()
        };
        for (stellar, why) in [
            (unpriced, "no price"),
            (no_exchange, "no exchange"),
            (inactive, "inactive"),
            (uninhabited, "uninhabited"),
            (unlandable, "unlandable"),
            (gate, "a gate"),
        ] {
            assert!(!system_trades(std::slice::from_ref(&stellar), 0), "{why}");
            assert!(
                !system_has_service(std::slice::from_ref(&stellar), 0) || why == "no price",
                "{why}"
            );
        }
        assert!(
            !system_trades(std::slice::from_ref(&base), COMMODITIES),
            "no such commodity"
        );
        assert!(!system_has_service(&[base], 3), "no such service");
    }

    #[test]
    fn each_service_reads_its_own_flag() {
        for (service, bit) in SERVICE_FLAGS.into_iter().enumerate() {
            let stellar = PanelStellar {
                flags: 0x0001 | bit,
                ..port("One")
            };
            let has: Vec<bool> = (0..3)
                .map(|n| system_has_service(std::slice::from_ref(&stellar), n))
                .collect();
            let expected: Vec<bool> = (0..3).map(|n| n == service).collect();
            assert_eq!(has, expected, "{service}");
        }
    }

    #[test]
    fn the_words_read_their_numbers_and_a_missing_one_is_empty() {
        let words = words();
        assert_eq!(words.hypergate_prompt, "#308");
        assert_eq!(words.none, "#335");
        assert_eq!(words.hypergate_destination, "#340");
        assert_eq!(words.visibility_levels, ["#321", "#322", "#323"]);
        let short = PanelWords::from_list(&strings(&["one"]));
        assert_eq!(short, PanelWords::default());
    }
}
