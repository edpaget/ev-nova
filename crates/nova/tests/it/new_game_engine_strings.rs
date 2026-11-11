//! The engine's own strings reach every reader the new game runs: the
//! fixture (`nova_data::new_game::fixture::folder`) authors no `STR#` the
//! engine reads, so the new game's mapping supplies them
//! (`nova_data::new_game`'s `strings` module). These tests open the
//! fixture as the app opens it and hand its catalog to each reader, so an
//! index a reader asks for that the mapping's table left out reads blank
//! here. Which index each reader reads is pinned in its own crate; the
//! text in `nova_data`.
//!
//! The readers, all as the upstream crates publish them:
//!
//! - `STR#` 2002: the player info's [`Words::from_list`], the trade
//!   dialog's [`TradeWords::from_list`], the galaxy map panel's
//!   [`PanelWords::from_list`], the main menu pilot panel's
//!   [`PanelLabels::from_list`], a hail's [`AdviceWords::read`] and the
//!   christening's prompt ([`christening`]);
//! - `STR#` 128: the New Pilot dialog's [`name_defaults`] and the
//!   christening's default, rolled on the app's own chance over many
//!   seeds;
//! - `STR#` 134: every index [`legal_status_index`] gives;
//! - `STR#` 138: every index [`rating::string_index`] gives;
//! - `STR#` 1100: [`type_label`] of each of the fixture's stellars;
//! - `STR#` 3000/3001: each named ship comm group's [`Reply::Comm`] in
//!   every variant;
//! - `STR#` 3002 (and 3000): each [`PlanetReply`] in every variant.
//!
//! The legal status and combat rating are drawn through the session's
//! private `Chart::legal_status_in` and `Setup::combat_rating_text`, which
//! read entry index + 1 of their list; the tests read the list the same
//! way after the public index functions. `STR#` 5003's distress calls are
//! read only by the session's private `Session::call_for_help`, whose list
//! and count (`DISTRESS_LINES`, `DISTRESS_LINE_COUNT`) sit in a private
//! module, so it is left out here (`nova_data` pins its 20 entries).

use std::collections::BTreeSet;

use nova::chance::SplitMix;
use nova_data::GameData;
use nova_data::records::stellar::Stellar;
use nova_sim::CommCatalog;
use nova_sim::TradeWords;
use nova_sim::christening::christening;
use nova_sim::hail::advice::AdviceWords;
use nova_sim::hail::reply::{self, Reply};
use nova_sim::map_panel::PanelWords;
use nova_sim::new_pilot::name_defaults;
use nova_sim::planet_comm::{self, PlanetReply, type_label};
use nova_sim::player_info::{Domination, LEGAL_STATUS_STRINGS, Words, legal_status_index};
use nova_sim::rating::{self, RATING_CAP, RATING_STRINGS};
use nova_view::menu::pilot_panel::PanelLabels;

use crate::new_game_conversation::opened_from;

/// The `STR#` every 2002 word struct is read from.
const MESSAGES: i16 = reply::MESSAGES;

/// The fixture's catalog as the app opens it.
fn catalog() -> std::rc::Rc<GameData> {
    opened_from(&nova_data::new_game::fixture::folder())
}

/// The string fields `debug`, a reader's value as `{:?}` writes it, holds
/// empty: an empty string reads `""` after a field's `: `, or as an
/// array's entry, and text holding a `"` writes it `\"`, so no other text
/// matches.
fn blank_fields(debug: &str) -> usize {
    [": \"\"", "[\"\"", ", \"\""]
        .iter()
        .map(|blank| debug.matches(blank).count())
        .sum()
}

#[test]
fn a_blank_field_is_told_from_text_holding_quotes() {
    assert_eq!(
        blank_fields(r#"W { a: "", b: ["x", ""], c: ["", "y"] }"#),
        3
    );
    assert_eq!(blank_fields(r#"W { a: "\"", b: ["say \"\", ok"] }"#), 0);
}

#[test]
fn every_str_2002_word_struct_reads_each_of_its_words() {
    let game = catalog();
    let list = game.string_list(MESSAGES);
    for (reader, debug) in [
        (
            "player_info::Words::from_list",
            format!("{:?}", Words::from_list(&list)),
        ),
        (
            "TradeWords::from_list",
            format!("{:?}", TradeWords::from_list(&list)),
        ),
        (
            "map_panel::PanelWords::from_list",
            format!("{:?}", PanelWords::from_list(&list)),
        ),
        (
            "pilot_panel::PanelLabels::from_list",
            format!("{:?}", PanelLabels::from_list(&list)),
        ),
        (
            "hail::advice::AdviceWords::read",
            format!("{:?}", AdviceWords::read(&*game)),
        ),
    ] {
        assert_eq!(blank_fields(&debug), 0, "{reader}: {debug}");
    }
}

/// How many seeds of the app's own chance the name rolls are drawn on:
/// enough that each of a list's few defaults is drawn.
const SEEDS: u64 = 200;

#[test]
fn every_default_name_and_the_christening_read_their_text() {
    let game = catalog();
    for seed in 0..SEEDS {
        let named = name_defaults(&*game, &mut SplitMix::new(seed));
        assert!(!named.name.trim().is_empty(), "name, seed {seed}");
        assert!(!named.nickname.trim().is_empty(), "nickname, seed {seed}");
        let asked = christening(&*game, &[], nova_sim::ShipId(128), &mut SplitMix::new(seed));
        assert!(!asked.default.trim().is_empty(), "ship name, seed {seed}");
        assert!(
            !asked.prompt.trim().trim_end_matches(':').trim().is_empty(),
            "the prompt opens with words: {:?}",
            asked.prompt
        );
    }
}

/// Each index [`legal_status_index`] gives, a government not xenophobic,
/// over each domination and records on either side of each power-of-two
/// multiple of the tolerance, both signs, to far past any threshold.
fn legal_indices() -> BTreeSet<usize> {
    let mut indices = BTreeSet::new();
    for tolerance in [1_i64, 10, 100] {
        let records = (0..=24).flat_map(|power| {
            let at = tolerance << power;
            [at - 1, at, at + 1].into_iter().flat_map(|r| [r, -r])
        });
        for record in records.chain([0]) {
            for dominated in [Domination::None, Domination::Some, Domination::All] {
                indices.extend(legal_status_index(record, tolerance, false, dominated));
            }
        }
    }
    indices
}

#[test]
fn every_legal_status_the_index_reaches_reads_its_text() {
    let game = catalog();
    let list = game.string_list(LEGAL_STATUS_STRINGS);
    let indices = legal_indices();
    assert!(indices.len() > 1, "{indices:?}");
    for index in indices {
        let text = list.get(index).map(String::as_str);
        assert!(
            text.is_some_and(|text| !text.trim().is_empty()),
            "STR# {LEGAL_STATUS_STRINGS} #{}: {text:?}",
            index + 1
        );
    }
}

#[test]
fn every_combat_rating_reads_its_text() {
    let game = catalog();
    let list = game.string_list(RATING_STRINGS);
    let indices: BTreeSet<usize> = (0..=RATING_CAP).map(rating::string_index).collect();
    assert!(indices.len() > 1, "{indices:?}");
    for index in indices {
        let text = list.get(index).map(String::as_str);
        assert!(
            text.is_some_and(|text| !text.trim().is_empty()),
            "STR# {RATING_STRINGS} #{}: {text:?}",
            index + 1
        );
    }
}

#[test]
fn every_stellar_s_type_label_reads_its_text() {
    let game = catalog();
    let graphics: BTreeSet<i16> = game
        .records::<Stellar>()
        .filter_map(|(_, entry)| Some(entry.ok()?.record.graphic_type))
        .collect();
    assert!(!graphics.is_empty(), "the fixture has stellars");
    for graphic in graphics {
        assert!(
            !type_label(graphic, &*game).trim().is_empty(),
            "Type {graphic}"
        );
    }
}

#[test]
fn every_named_ship_comm_group_reads_its_text_in_every_variant() {
    let game = catalog();
    for group in [
        reply::CHANNEL_OPEN,
        reply::NO_RESPONSE,
        reply::WHAT_IS_IT,
        reply::ESCORT_OPENING,
        reply::FIGHTER_OPENING,
        reply::GLAD_TO_SEE_YOU,
        reply::NICE_TO_MEET_YOU,
        reply::NO_MONEY,
        reply::WASTING_TIME,
        reply::NOT_IN_TROUBLE,
        reply::BUSY,
        reply::RATHER_NOT,
        reply::PAY_ME_FIRST,
        reply::IN_YOUR_DREAMS,
        reply::PLEASURE,
        reply::JUST_AN_ESCORT,
        reply::GOOD_MOOD,
        reply::BAD_MOOD,
        reply::CANT_DO_THAT,
        reply::HELP_FOR_PAY,
        reply::ON_MY_WAY,
        reply::HOW_DARE_YOU,
        reply::COMEDIAN,
        reply::RELEASED,
    ] {
        for variant in 0..reply::VARIANTS {
            let said = Reply::Comm(group).say(variant, &*game, "Ada's ship");
            assert!(
                !said.trim().is_empty(),
                "group {group} variant {variant} at {:?}",
                Reply::locate(group, variant)
            );
        }
    }
}

#[test]
fn every_planet_reply_reads_its_text_in_every_variant() {
    let game = catalog();
    let variants = u8::try_from(planet_comm::VARIANTS).expect("a few");
    for reply in [
        PlanetReply::WhatDoYouWant,
        PlanetReply::NoResponse,
        PlanetReply::Greetings,
        PlanetReply::WastingTime,
        PlanetReply::NoBribe,
        PlanetReply::BribeAsked,
        PlanetReply::BribeDeclined,
        PlanetReply::Laugh,
        PlanetReply::Fight,
        PlanetReply::Dominated { station: false },
        PlanetReply::Dominated { station: true },
        PlanetReply::Released { station: false },
        PlanetReply::Released { station: true },
    ] {
        for variant in 0..variants {
            assert!(
                !reply.say(variant, "Hearth", &*game).trim().is_empty(),
                "{reply:?} variant {variant} at {:?}",
                reply.entry(variant)
            );
        }
    }
    for variant in 0..variants {
        let open = PlanetReply::ChannelOpen.say(variant, "Hearth", &*game);
        assert!(
            open.len() > "Hearth.".len(),
            "ChannelOpen variant {variant}: {open:?}"
        );
    }
}
