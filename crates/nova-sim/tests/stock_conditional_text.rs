//! The conditional text over the stock `dësc`s: 316 hold one (214 `{G`,
//! 129 `{P`, 115 `{b`, with 136 `\"` inside their strings). Resolved for
//! a pilot, none keeps a brace or an escape. Skips, passing, when
//! `NOVA_DATA` is unset.
//!
//! - `dësc` 3004 ends `{P30"License" "License, REQUIRES YOU TO REGISTER
//!   EV NOVA"}`: paid for, it reads "License".
//! - `dësc` 472 holds `{G "man" "woman"}`.

mod common;

use nova_data::records::desc::Desc;
use nova_data::{GameData, Record};
use nova_sim::control::{Bit, Gate, PilotFacts};
use nova_sim::text_parse::{ParseRules, conditional_text};
use nova_sim::{Gender, OutfitId, SystemId};

/// A new pilot but female.
#[derive(Debug)]
struct Female;

impl PilotFacts for Female {
    fn bit(&self, _bit: Bit) -> bool {
        false
    }

    fn gender(&self) -> Gender {
        Gender::Female
    }

    fn paid(&self, _days: u16) -> bool {
        true
    }

    fn has_outfit(&self, _outfit: OutfitId) -> bool {
        false
    }

    fn explored(&self, _system: SystemId) -> bool {
        false
    }
}

/// Stock `dësc` `id`'s text.
fn text(data: &GameData, id: i16) -> String {
    data.get::<Desc>(id)
        .expect("there")
        .expect("decodes")
        .record
        .text
        .as_str()
        .to_owned()
}

#[test]
fn no_stock_desc_keeps_a_brace_or_an_escape_once_resolved() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let mut conditional = 0;
    for &id in data.ids(Desc::TYPE) {
        let Some(Ok(desc)) = data.get::<Desc>(id) else {
            continue;
        };
        let raw = desc.record.text.as_str();
        if raw.contains('{') {
            conditional += 1;
        }
        let resolved = conditional_text(raw, Gate::FRESH.pilot, ParseRules::default());
        assert!(
            !resolved.contains('{') && !resolved.contains('}') && !resolved.contains("\\\""),
            "dësc {id} resolved: {resolved:?}"
        );
    }
    assert_eq!(conditional, 316);
}

#[test]
fn a_stock_outfits_licence_line_reads_paid_for() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let resolved = conditional_text(&text(&data, 3004), Gate::FRESH.pilot, ParseRules::default());
    assert!(
        resolved.ends_with("Requires: Heavy Weapons License"),
        "{resolved:?}"
    );
}

#[test]
fn a_stock_stellars_text_reads_by_the_pilots_gender() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let raw = text(&data, 472);
    let male = conditional_text(&raw, Gate::FRESH.pilot, ParseRules::default());
    let female = conditional_text(&raw, &Female, ParseRules::default());
    assert!(male.contains("to the man who guided"), "{male:?}");
    assert!(female.contains("to the woman who guided"), "{female:?}");
}
