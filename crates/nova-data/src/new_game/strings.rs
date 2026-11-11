//! The engine's own `STR#` lists, for data that has none: the words a
//! hailed ship or stellar answers with, the status messages, prompts and
//! labels, the default pilot and ship names, the legal statuses and the
//! combat ratings, in plain English, at the IDs and indices the engine
//! reads them from ([`defaults`]).
//!
//! As [`default_interface`](super::records::default_interface) and
//! [`default_dialogs`](super::interface::default_dialogs) stand in for an
//! interface the bible doesn't author, these stand in for the string lists
//! it doesn't. They are engine text, not world text, so the bible has no
//! format word for them: it may later replace them through its authored
//! interface, the route rdm:task/map-authored-status-bar takes. The stock
//! game's side of the same question, a plug-in's `STR ` overriding one
//! string, is rdm:task/plugin-interface-string-overrides.
//!
//! [`fill`] appends each list after every mapped record, at an ID no
//! mapped `STR#` holds, so the goods' lists ([`goods`](super::goods)),
//! and any list a later mapping writes, keep their own text.
//!
//! **What each list holds.** Each list declares the entries its reader
//! asks for ([`DefaultList::entries`]); an index no reader asks for is
//! blank. Where a reader composes a line around an entry, the entry is
//! written to fit: a stellar's greeting (`STR#` 3002 #1-#5) and a mission
//! ship's welcome (`STR#` 3000 #41-#45) end in a space, as the name
//! follows, and a prompt the reader finishes (the ship naming prompts, the
//! reinforcement and scan hail lines) leaves its ending to it.
//!
//! **Left out.** A list is left out only when the new game can't reach its
//! reader: `STR#` 7000 + g (a `düde`'s `InfoTypes` 0x8000), 7500 + n
//! (`InfoTypes` 0x4000), 7100 and 7101 (a `përs`), 7600 (a `flët`), 1000
//! (a `sÿst` `Message`), 4002 (a mission's cargo type), the lists a
//! `mïsn` or set operator names, and 8100 and 8101 with `STR#` 2002
//! #190-#194, the drawn news, which the new game's ruleset shows the
//! story's news in place of. The `new-game-play` phase 2 Findings
//! (rdm:phase/new-game-play/phase-2-default-engine-strings) record every
//! list the engine reads, its reader, and why each is defaulted, mapped,
//! kept as a constant or left out.

use crate::decode::Entry;
use crate::records::string_list::StrList;
use crate::registry::AnyRecord;
use crate::wire::string::MacString;

/// One `STR#` list the engine reads, with the entries its reader asks for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefaultList {
    /// The `STR#` ID.
    pub id: i16,
    /// The resource name.
    pub name: &'static str,
    /// What reads it, in the engine's crates.
    pub reader: &'static str,
    /// Each entry its reader asks for, by its 1-based index, in order.
    pub entries: Vec<(u16, &'static str)>,
}

impl DefaultList {
    /// The list's strings, from #1 to its last entry, empty at an index no
    /// reader asks for.
    #[must_use]
    pub fn strings(&self) -> Vec<MacString> {
        let last = self.entries.iter().map(|&(at, _)| at).max().unwrap_or(0);
        let mut strings = vec![MacString::default(); usize::from(last)];
        for &(at, text) in &self.entries {
            if let Some(slot) = usize::from(at).checked_sub(1) {
                strings[slot] = text.into();
            }
        }
        strings
    }

    /// The list as a `STR#` record at its ID, named.
    fn record(&self) -> Entry<AnyRecord> {
        Entry {
            id: self.id,
            name: Some(self.name.to_owned()),
            record: AnyRecord::StrList(Box::new(StrList {
                strings: self.strings(),
            })),
        }
    }
}

/// How many variants each group of comm replies has: a conversation says
/// each reply in one variant, drawn as it opens.
const VARIANTS: u16 = 5;

/// The groups of `STR#` 3000 the engine says, group g's variant r at
/// 5g + r + 1, as `nova_sim_ai::hail::reply::Reply::locate` places it
/// (`_LoadResponse` @0x90a0b). The groups no reader says are blank.
const SHIP_GROUPS: &[(u16, [&str; 5])] = &[
    (
        0,
        [
            "Channel open.",
            "Go ahead.",
            "We read you.",
            "Receiving you.",
            "Channel's open.",
        ],
    ),
    (
        1,
        [
            "No response.",
            "There is no answer.",
            "The channel is silent.",
            "Nobody answers.",
            "Static.",
        ],
    ),
    (
        2,
        [
            "What is it you want?",
            "What do you want?",
            "State your business.",
            "Make it quick.",
            "What now?",
        ],
    ),
    (
        4,
        [
            "What can I do for you?",
            "Orders?",
            "Standing by.",
            "Ready when you are.",
            "What do you need?",
        ],
    ),
    (
        5,
        [
            "What can I do for you, sir?",
            "Orders, sir?",
            "Standing by, sir.",
            "Ready, sir.",
            "Your orders, sir?",
        ],
    ),
    (
        8,
        [
            "Glad to see you, ",
            "Good to see you, ",
            "There you are, ",
            "Right on time, ",
            "Welcome, ",
        ],
    ),
    (
        9,
        [
            "Nice to meet you.",
            "Pleased to meet you.",
            "Good day to you.",
            "Safe travels.",
            "Hello there.",
        ],
    ),
    (
        12,
        [
            "Come back when you have the money.",
            "You can't afford that.",
            "You don't have the credits.",
            "Not on your budget.",
            "Get the money first.",
        ],
    ),
    (
        13,
        [
            "Stop wasting my time.",
            "Leave me alone.",
            "I have nothing to say to you.",
            "Go away.",
            "Don't bother me.",
        ],
    ),
    (
        14,
        [
            "You're not in any trouble.",
            "You look fine to me.",
            "You don't need help.",
            "Nothing seems wrong with your ship.",
            "You're doing fine.",
        ],
    ),
    (
        16,
        [
            "I'm busy.",
            "Not now.",
            "I've got my hands full.",
            "Some other time.",
            "Can't talk now.",
        ],
    ),
    (
        17,
        [
            "I'd rather not.",
            "No, thank you.",
            "I'll pass.",
            "Not this time.",
            "I don't think so.",
        ],
    ),
    (
        18,
        [
            "You'll have to pay me first.",
            "It will cost you.",
            "That has a price.",
            "Pay up first.",
            "Not for free.",
        ],
    ),
    (
        19,
        [
            "In your dreams.",
            "Not a chance.",
            "Forget it.",
            "No deal.",
            "Never.",
        ],
    ),
    (
        20,
        [
            "A pleasure doing business with you.",
            "Thank you kindly.",
            "Done.",
            "We have a deal.",
            "Much obliged.",
        ],
    ),
    (
        22,
        [
            "Sorry, I'm just an escort.",
            "I'm only an escort.",
            "I have my own orders.",
            "I can't leave my post.",
            "I'm escorting someone else.",
        ],
    ),
    (
        23,
        [
            "You're lucky, I'm in a good mood today.",
            "I'll make you a good price.",
            "Today you get a bargain.",
            "I'm feeling generous.",
            "Lucky for you, I'm in a good mood.",
        ],
    ),
    (
        24,
        [
            "I'm in a bad mood, so it will cost you.",
            "This will cost you extra.",
            "Bad day. Expect to pay for it.",
            "You picked the wrong day to ask.",
            "My price just went up.",
        ],
    ),
    (
        25,
        [
            "Sorry, I can't do that.",
            "I'm not able to do that.",
            "That isn't something I can do.",
            "I can't help with that.",
            "Not possible, sorry.",
        ],
    ),
    (
        28,
        [
            "I'll help you if you pay me.",
            "Help costs money.",
            "For a fee, I can help.",
            "I'll help, for a price.",
            "Pay me and I'll help.",
        ],
    ),
    (
        29,
        [
            "Okay, I'm on my way.",
            "On my way.",
            "Coming.",
            "Heading over now.",
            "Right away.",
        ],
    ),
    (
        30,
        [
            "How dare you! Prepare to die!",
            "You'll regret that!",
            "Then we fight!",
            "That was a mistake!",
            "Now you've done it!",
        ],
    ),
    (
        31,
        [
            "What a comedian.",
            "Very funny.",
            "You must be joking.",
            "Nice try.",
            "Ha. No.",
        ],
    ),
];

/// The first group on `STR#` 3001: group g's variant r is #5(g - 38) +
/// r + 1 there (`Reply::locate`).
const FIRST_MORE_GROUP: u16 = 38;

/// The groups of `STR#` 3001 the engine says.
const MORE_SHIP_GROUPS: &[(u16, [&str; 5])] = &[(
    38,
    [
        "Goodbye, captain.",
        "Farewell, captain.",
        "It was an honour, captain.",
        "So long, captain.",
        "Good luck out there, captain.",
    ],
)];

/// The runs of `STR#` 3002 a stellar says, by the first entry of each,
/// variant r at first + r (`nova_sim_ai::planet_comm::PlanetReply::entry`,
/// `_DoPlanetCommDialog` @0x96949).
const STELLAR_RUNS: &[(u16, [&str; 5])] = &[
    (
        1,
        [
            "This is ",
            "You have reached ",
            "Go ahead, this is ",
            "Traffic control, ",
            "Channel open to ",
        ],
    ),
    (
        6,
        [
            "You? Demand tribute? Come back with a real fleet.",
            "We are not afraid of you.",
            "That is the funniest thing we have heard all day.",
            "Run along, pilot.",
            "Tribute? From us? No.",
        ],
    ),
    (
        11,
        [
            "Our defences are ready for you.",
            "Then you will have to take it.",
            "We will fight you.",
            "Defence ships, launch.",
            "Try it, if you dare.",
        ],
    ),
    (
        21,
        [
            "Stop wasting our time.",
            "We have nothing to discuss.",
            "This channel is closing.",
            "Go away.",
            "We are busy.",
        ],
    ),
    (
        31,
        [
            "Then you will not be landing here.",
            "No payment, no landing.",
            "Suit yourself.",
            "Then stay away.",
            "Your loss.",
        ],
    ),
    (
        41,
        [
            "Landing here will cost you.",
            "Pay, and we will clear you to land.",
            "A small payment will open our docks.",
            "We could look the other way, for a price.",
            "Clearance has a price.",
        ],
    ),
    (
        46,
        [
            "We do not take bribes.",
            "Your money is no good here.",
            "We cannot be bought.",
            "Keep your credits.",
            "Bribes are not welcome here.",
        ],
    ),
];

/// The single entries of `STR#` 3002: a planet's and a station's word as
/// it gives in (#26, #27) and as it is freed (#36, #37).
const STELLAR_SINGLES: &[(u16, &str)] = &[
    (26, "We give in. This world will pay you tribute."),
    (27, "We give in. This station will pay you tribute."),
    (36, "This world is free again."),
    (37, "This station is free again."),
];

/// `STR#` 2002, the engine's messages and words: each entry an engine
/// reader asks for, by its index.
const MESSAGES: &[(u16, &str)] = &[
    (1, "ton"),
    (2, "tons"),
    (8, "Energy Status:"),
    (9, "maneuvering energy"),
    (12, "Shield Status:"),
    (17, "Armor Status:"),
    (32, "credit"),
    (33, "credits"),
    (60, "on"),
    (119, "Rename this captured ship:"),
    (120, "Please name your new"),
    (121, "Name your new"),
    (160, "Wait for me!"),
    (161, "Don't leave me behind!"),
    (162, "Wait for us!"),
    (163, "Don't leave us behind!"),
    (175, "Greetings."),
    (176, "is a good place to"),
    (177, "buy"),
    (178, "sell"),
    (179, "The last time I was"),
    (180, "at"),
    (181, "the price of"),
    (182, "was very"),
    (183, "was really"),
    (184, "was pretty"),
    (185, "low"),
    (186, "high"),
    (197, "Commodity:"),
    (198, "In Fleet:"),
    (199, "In Hold:"),
    (200, "Price:"),
    (201, "Low"),
    (202, "Med"),
    (203, "High"),
    (204, "Higher"),
    (205, "Lower"),
    (239, "jump"),
    (240, "jumps"),
    (251, "Pilot Name:"),
    (252, "Current Date:"),
    (253, "System:"),
    (254, "Combat Rating:"),
    (255, "Ship Name:"),
    (256, "Ship Class:"),
    (257, "Turn Rate:"),
    (258, "deg/sec"),
    (259, "Accel Rate:"),
    (260, "Max Speed:"),
    (261, "Failed"),
    (262, "plus"),
    (263, "only"),
    (264, "Expenses:"),
    (265, "Income:"),
    (266, "Net"),
    (267, "per day"),
    (268, "You have no cargo aboard your fleet's ships"),
    (269, "You have no cargo aboard your ship"),
    (270, "You have no extras on your ship"),
    (271, "You have no ranks or honours"),
    (272, "Extras on your ship:"),
    (273, "Ship trade-in value:"),
    (274, "Your ranks and honours:"),
    (276, "No pilot loaded"),
    (277, "has been killed"),
    (278, "Legal status in"),
    (279, "current system:"),
    (291, "Jettison your cargo?"),
    (306, "Sensors detect a"),
    (307, "reinforcement fleet approaching."),
    (308, "Choose where the hypergate sends you"),
    (309, "Ports:"),
    (310, "<Unknown>"),
    (311, "Navigation Hazards:"),
    (312, "gravity shear"),
    (313, "sparse"),
    (314, "moderate"),
    (315, "dense"),
    (316, "asteroid field"),
    (317, "light"),
    (318, "moderate"),
    (319, "heavy"),
    (320, "interference"),
    (321, "reduced"),
    (322, "severely reduced"),
    (323, "greatly reduced"),
    (324, "visibility"),
    (325, "Government:"),
    (326, "Legal Status:"),
    (327, "Goods Traded:"),
    (328, "Services:"),
    (329, "Trading"),
    (330, "Outfitting"),
    (331, "Shipyard"),
    (332, "None Available"),
    (333, "Independent"),
    (334, "Uninhabited System"),
    (335, "None"),
    (336, "none"),
    (337, "Current System:"),
    (338, "Selected System:"),
    (339, "Destination System:"),
    (340, "Hypergate Destination:"),
    (341, "captain"),
    (362, "Current cargo"),
    (363, "Other cargo:"),
    (364, "Free cargo space"),
    (365, "in your ship"),
    (366, "in your fleet"),
    (367, "mission cargo"),
    (368, "miscellaneous goods"),
    (369, "free in your ship"),
    (381, "prepare to be scanned"),
    (382, "hold still for a scan"),
    (383, "stand by to be scanned"),
    (391, "of"),
    (392, "and"),
    (393, "a"),
    (394, "an"),
    (396, "N/A"),
];

/// `STR#` 5003, the twenty distress calls, one drawn evenly.
const DISTRESS: [&str; 20] = [
    "Mayday! Mayday! We're under attack!",
    "Help! Anyone out there, help us!",
    "We're taking fire! Need assistance!",
    "Our shields are failing! Help!",
    "Requesting help, we're under attack!",
    "We can't hold them off much longer!",
    "Somebody help us, please!",
    "We're being fired on! Any ship, respond!",
    "Distress call: we need help now!",
    "Under attack! Requesting aid!",
    "Our armour won't take much more!",
    "Any pilot in range, we need help!",
    "We're outgunned! Help us!",
    "Hostile ship on us! Need help!",
    "This is a distress call! Help!",
    "We're losing power! Somebody help!",
    "Please, anyone, we're under fire!",
    "We need an escort, now!",
    "Help! They're closing in!",
    "Mayday! Any ship, please respond!",
];

/// `STR#` 128: the New Pilot dialog's names (#1-#3) and nicknames
/// (#4-#6), and a christened ship's names (#7-#9), each field rolled among
/// its three.
const DEFAULT_NAMES: [&str; 9] = [
    "Ada Rowe",
    "Sam Okafor",
    "Jun Arlen",
    "Ace",
    "Sparrow",
    "Drifter",
    "Second Wind",
    "Long Haul",
    "Far Horizon",
];

/// `STR#` 134: the legal statuses, the status index n at #n + 1, for n
/// from 1 (a clean record) to 17 (every counting stellar dominated);
/// `nova_sim_port::player_info::legal_status_index` gives n.
const LEGAL_STATUSES: [&str; 17] = [
    "Clean",
    "Offender",
    "Petty criminal",
    "Criminal",
    "Felon",
    "Wanted",
    "Fugitive",
    "Outlaw",
    "Public enemy",
    "Citizen",
    "Good citizen",
    "Upstanding citizen",
    "Respected",
    "Honoured",
    "Hero",
    "Partly under your rule",
    "Under your rule",
];

/// `STR#` 138: the combat ratings, #1 to #11 by the rating's step
/// (`nova_sim::rating::string_index`).
const RATINGS: [&str; 11] = [
    "No ability",
    "Harmless",
    "Mostly harmless",
    "Novice",
    "Fair",
    "Average",
    "Competent",
    "Good",
    "Very good",
    "Dangerous",
    "Deadly",
];

/// `STR#` 1100's #1: the type label of a stellar of `Type` 0, which every
/// stellar the new game maps has.
const TYPE_LABEL: &str = "Stellar object";

/// Each group of `groups` at 5(g - `first`) + r + 1, as a list's entries.
fn grouped(groups: &[(u16, [&'static str; 5])], first: u16) -> Vec<(u16, &'static str)> {
    let mut entries = Vec::new();
    for &(group, lines) in groups {
        for (variant, text) in (0..).zip(lines) {
            entries.push((VARIANTS * (group - first) + variant + 1, text));
        }
    }
    entries
}

/// `texts` numbered from #`first`.
fn numbered(first: u16, texts: &[&'static str]) -> Vec<(u16, &'static str)> {
    (first..).zip(texts.iter().copied()).collect()
}

/// Every list the new game supplies, in ID order.
#[must_use]
pub fn defaults() -> Vec<DefaultList> {
    let mut stellar = STELLAR_RUNS
        .iter()
        .flat_map(|&(first, lines)| numbered(first, &lines))
        .chain(STELLAR_SINGLES.iter().copied())
        .collect::<Vec<_>>();
    stellar.sort_unstable_by_key(|&(at, _)| at);
    vec![
        DefaultList {
            id: 128,
            name: "Default names",
            reader: "`nova_sim_port::new_pilot::name_defaults`, `nova_sim_port::christening::christening`",
            entries: numbered(1, &DEFAULT_NAMES),
        },
        DefaultList {
            id: 134,
            name: "Legal statuses",
            reader: "the session's `Chart::legal_status_in` (`nova_sim`, player info)",
            entries: numbered(2, &LEGAL_STATUSES),
        },
        DefaultList {
            id: 138,
            name: "Combat ratings",
            reader: "the session's `Setup::combat_rating_text` (`nova_sim`)",
            entries: numbered(1, &RATINGS),
        },
        DefaultList {
            id: 1100,
            name: "Stellar type labels",
            reader: "`nova_sim_ai::planet_comm::type_label`",
            entries: vec![(1, TYPE_LABEL)],
        },
        DefaultList {
            id: 2002,
            name: "Messages and words",
            reader: "every `STR#` 2002 reader (see the phase 2 Findings)",
            entries: MESSAGES.to_vec(),
        },
        DefaultList {
            id: 3000,
            name: "Ship comm replies",
            reader: "`nova_sim_ai::hail::reply::Reply::say`, `nova_sim_ai::planet_comm::PlanetReply::say`",
            entries: grouped(SHIP_GROUPS, 0),
        },
        DefaultList {
            id: 3001,
            name: "More ship comm replies",
            reader: "`nova_sim_ai::hail::reply::Reply::say`",
            entries: grouped(MORE_SHIP_GROUPS, FIRST_MORE_GROUP),
        },
        DefaultList {
            id: 3002,
            name: "Stellar comm replies",
            reader: "`nova_sim_ai::planet_comm::PlanetReply::say`",
            entries: stellar,
        },
        DefaultList {
            id: 5003,
            name: "Distress calls",
            reader: "the session's `Session::call_for_help` (`nova_sim`)",
            entries: numbered(1, &DISTRESS),
        },
    ]
}

/// Appends to `records` each of [`defaults`] at an ID no `STR#` of
/// `records` already holds (see the module docs).
pub(super) fn fill(records: &mut Vec<Entry<AnyRecord>>) {
    let held: Vec<i16> = records
        .iter()
        .filter(|entry| matches!(entry.record, AnyRecord::StrList(_)))
        .map(|entry| entry.id)
        .collect();
    records.extend(
        defaults()
            .iter()
            .filter(|list| !held.contains(&list.id))
            .map(DefaultList::record),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(id: i16) -> DefaultList {
        defaults()
            .into_iter()
            .find(|list| list.id == id)
            .unwrap_or_else(|| panic!("STR# {id} is defaulted"))
    }

    fn entry(id: i16, index: u16) -> &'static str {
        list(id)
            .entries
            .iter()
            .find_map(|&(at, text)| (at == index).then_some(text))
            .unwrap_or_else(|| panic!("STR# {id} #{index} is declared"))
    }

    #[test]
    fn every_declared_entry_holds_plain_text() {
        let lists = defaults();
        assert!(!lists.is_empty());
        for list in &lists {
            assert!(!list.entries.is_empty(), "STR# {}", list.id);
            assert!(!list.name.is_empty() && !list.reader.is_empty());
            for &(index, text) in &list.entries {
                let at = format!("STR# {} #{index}", list.id);
                assert!(index >= 1, "{at}");
                assert!(!text.trim().is_empty(), "{at} is empty");
                assert!(!text.starts_with('*'), "{at} would be passed over");
                assert!(text.is_ascii(), "{at} is plain ASCII: {text:?}");
            }
            let indices: Vec<u16> = list.entries.iter().map(|&(at, _)| at).collect();
            assert!(indices.is_sorted(), "STR# {} in order", list.id);
            let mut unique = indices.clone();
            unique.dedup();
            assert_eq!(unique, indices, "STR# {} once each", list.id);
        }
        let mut ids: Vec<i16> = lists.iter().map(|list| list.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), lists.len(), "each list once");
    }

    #[test]
    fn a_list_s_strings_hold_each_entry_at_its_index_and_blanks_between() {
        let list = DefaultList {
            id: 1,
            name: "n",
            reader: "r",
            entries: vec![(2, "two"), (4, "four")],
        };
        let strings: Vec<String> = list.strings().iter().map(ToString::to_string).collect();
        assert_eq!(strings, ["", "two", "", "four"]);
    }

    #[test]
    fn a_comm_group_s_variants_sit_where_the_reader_locates_them() {
        let at = |id: i16| -> Vec<u16> { list(id).entries.iter().map(|&(at, _)| at).collect() };
        assert_eq!(at(3001), [1, 2, 3, 4, 5], "group 38 opens STR# 3001");
        assert_eq!(entry(3001, 1), "Goodbye, captain.");
        let ship = at(3000);
        assert_eq!(&ship[..5], [1, 2, 3, 4, 5], "group 0");
        assert!(ship.contains(&46) && ship.contains(&50), "group 9");
        assert_eq!(ship.last(), Some(&160), "group 31's last variant");
    }

    #[test]
    fn a_line_a_name_follows_ends_ready_for_it() {
        for variant in 0..5 {
            for text in [entry(3002, 1 + variant), entry(3000, 41 + variant)] {
                assert!(text.ends_with(' '), "{text:?} then a name");
            }
        }
    }

    #[test]
    fn a_line_the_reader_finishes_leaves_its_ending_to_it() {
        let finished = [
            (2002, 120),
            (2002, 121),
            (2002, 306),
            (2002, 381),
            (2002, 382),
            (2002, 383),
            (2002, 176),
        ];
        for (id, index) in finished {
            let text = entry(id, index);
            assert!(
                !text.ends_with(' ') && !text.ends_with('.') && !text.ends_with(':'),
                "STR# {id} #{index}: {text:?}"
            );
        }
    }

    /// Every list of [`defaults`], by ID.
    fn ids(records: &[Entry<AnyRecord>]) -> Vec<i16> {
        let mut ids: Vec<i16> = records
            .iter()
            .filter(|entry| matches!(entry.record, AnyRecord::StrList(_)))
            .map(|entry| entry.id)
            .collect();
        ids.sort_unstable();
        ids
    }

    fn own(id: i16) -> Entry<AnyRecord> {
        Entry {
            id,
            name: Some("Own".to_owned()),
            record: AnyRecord::StrList(Box::new(StrList {
                strings: vec!["mine".into()],
            })),
        }
    }

    #[test]
    fn the_fill_adds_every_default_to_records_with_none() {
        let mut records = Vec::new();
        fill(&mut records);
        let mut expected: Vec<i16> = defaults().iter().map(|list| list.id).collect();
        expected.sort_unstable();
        assert_eq!(ids(&records), expected);
        for list in defaults() {
            let entry = records
                .iter()
                .find(|entry| entry.id == list.id)
                .expect("added");
            assert_eq!(entry.name.as_deref(), Some(list.name));
            let AnyRecord::StrList(strings) = &entry.record else {
                panic!("a STR#");
            };
            assert_eq!(strings.strings, list.strings(), "STR# {}", list.id);
        }
    }

    #[test]
    fn the_fill_keeps_a_list_the_data_holds_and_adds_no_second() {
        let mut records = vec![own(2002), own(4000)];
        fill(&mut records);
        let mut expected: Vec<i16> = defaults().iter().map(|list| list.id).collect();
        expected.push(4000);
        expected.sort_unstable();
        assert_eq!(ids(&records), expected, "each ID once");
        let kept = records.iter().find(|entry| entry.id == 2002).expect("kept");
        assert_eq!(kept.name.as_deref(), Some("Own"));
        assert!(
            matches!(&kept.record, AnyRecord::StrList(list) if list.strings == ["mine".into()])
        );
    }
}
