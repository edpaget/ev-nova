//! The stock ship look: the new game's hulls drawn in stock EV Nova
//! sprites from the player's own `Nova Files`, as the settings file turns
//! it on. Off by default.
//!
//! The settings file (the one that keeps the sound settings and the
//! rules; see [`config`](crate::config)) holds it in one top-level object:
//!
//! ```json
//! {
//!   "stock_ship_look": { "on": true, "ships": { "light-freighter": 136 } }
//! }
//! ```
//!
//! - [`ON`]: whether the look is on. Absent, or not `true`, it is off.
//! - [`SHIPS`]: the player's overrides, each a bible ship id and the stock
//!   `shïp` ID whose sprites it borrows. A ship with none borrows its
//!   nearest base's, then its role's default (`nova_data::stock_look`).
//!
//! The stock art comes from `NOVA_DATA`, read once at startup
//! ([`crate::startup::run`]): without it, or with a folder that can't be
//! listed or loads nothing, there is one warning and every hull draws its
//! stand-in. The stock ruleset ignores the setting, as the stock game
//! draws its own sprites.
//!
//! It is a player's setting, not a [`Rulebook`] rule: the original has no
//! new game, and the simulation must never see it. Nothing in the game
//! writes it (the settings keeper keeps it as a key it doesn't own), and
//! none of it is part of the pilot save. The stock art is copyrighted:
//! the setting holds IDs alone, and nothing the look reads is written
//! anywhere ([`nova_data::StockArt`]).
//!
//! [`wire`] gives the art and the flight's looks; [`images`] builds the
//! game's images `main` hands the renderer, the art among them, so a
//! borrowed frame the flight draws decodes.
//!
//! [`Rulebook`]: nova_sim::Rulebook

use std::collections::BTreeMap;
use std::rc::Rc;

use nova_audio::SettingsStore;
use nova_data::stock_look::resolve;
use nova_data::{GameData, InterfaceData, StockArt};
use nova_render::GameImages;
use nova_sim::Rulebook;
use nova_view::flight::StockShipLooks;
use serde_json::{Map, Value};

use crate::rulebook::{eight_bit_reading, thirty_two_bit_reading};

/// The settings file's field holding the stock ship look.
pub const STOCK_SHIP_LOOK: &str = "stock_ship_look";
/// The setting that turns the look on.
pub const ON: &str = "on";
/// The setting holding the player's overrides.
pub const SHIPS: &str = "ships";

/// The stock ship look as the settings file sets it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StockLook {
    /// Whether it is on.
    pub on: bool,
    /// The player's overrides: a bible ship id to a stock `shïp` ID.
    pub ships: BTreeMap<String, i16>,
}

/// The stock ship look saved in `store`, and the warnings to print.
///
/// With no store, nothing saved, settings that cannot be read or are not
/// a JSON object, or no [`STOCK_SHIP_LOOK`] object, it is off, silently:
/// the sound settings' keeper warns about settings it cannot use. A
/// [`STOCK_SHIP_LOOK`] that is not an object, or an [`ON`] that is not a
/// bool, is off, with a warning; a [`SHIPS`] that is not an object is
/// ignored, and an entry of it that is not a whole number within a
/// `shïp` ID's range is left out, each with a warning naming its key.
#[must_use]
pub fn setting(store: Option<&mut (dyn SettingsStore + '_)>) -> (StockLook, Vec<String>) {
    let Some(store) = store else {
        return (StockLook::default(), Vec::new());
    };
    let Ok(Some(text)) = store.read() else {
        return (StockLook::default(), Vec::new());
    };
    let Ok(Value::Object(settings)) = serde_json::from_str::<Value>(&text) else {
        return (StockLook::default(), Vec::new());
    };
    let location = store.location();
    let mut warnings = Vec::new();
    let look = match settings.get(STOCK_SHIP_LOOK) {
        None => return (StockLook::default(), warnings),
        Some(Value::Object(look)) => look,
        Some(_) => {
            warnings.push(format!(
                "nova: the saved \"{STOCK_SHIP_LOOK}\" in {location} is not an object; \
                 leaving the stock ship look off"
            ));
            return (StockLook::default(), warnings);
        }
    };
    let on = match look.get(ON) {
        None => false,
        Some(Value::Bool(on)) => *on,
        Some(_) => {
            warnings.push(format!(
                "nova: the saved \"{STOCK_SHIP_LOOK}\".\"{ON}\" in {location} is not true or \
                 false; leaving the stock ship look off"
            ));
            false
        }
    };
    let ships = overrides(look, &location, &mut warnings);
    (StockLook { on, ships }, warnings)
}

/// The overrides `look` holds: each [`SHIPS`] entry that is a `shïp` ID,
/// with a warning for each that isn't, and for a [`SHIPS`] that isn't an
/// object.
fn overrides(
    look: &Map<String, Value>,
    location: &str,
    warnings: &mut Vec<String>,
) -> BTreeMap<String, i16> {
    let ships = match look.get(SHIPS) {
        None => return BTreeMap::new(),
        Some(Value::Object(ships)) => ships,
        Some(_) => {
            warnings.push(format!(
                "nova: the saved \"{STOCK_SHIP_LOOK}\".\"{SHIPS}\" in {location} is not an \
                 object; ignoring it"
            ));
            return BTreeMap::new();
        }
    };
    ships
        .iter()
        .filter_map(|(key, value)| {
            let stock = value.as_i64().and_then(|id| i16::try_from(id).ok());
            if stock.is_none() {
                warnings.push(format!(
                    "nova: the saved \"{STOCK_SHIP_LOOK}\".\"{SHIPS}\".\"{key}\" in {location} \
                     is not a shïp ID; leaving it out"
                ));
            }
            Some((key.clone(), stock?))
        })
        .collect()
}

/// The borrowed look over `stock`, the player's stock data opened from
/// `NOVA_DATA`, for the new game's `game_data` under `look`: the stock art
/// for the renderer and the looks for the flight, sharing it, and the
/// warnings to print ([`resolve`]'s, each one line). None when the look is
/// off or there is no stock data.
#[must_use]
pub fn wire(
    game_data: &GameData,
    look: &StockLook,
    stock: Option<GameData>,
) -> (Option<(Rc<StockArt>, StockShipLooks)>, Vec<String>) {
    let Some(stock) = stock.filter(|_| look.on) else {
        return (None, Vec::new());
    };
    let art = Rc::new(StockArt::new(stock));
    let resolved = resolve(game_data, &art, &look.ships);
    let warnings = resolved
        .warnings
        .iter()
        .map(|warning| format!("nova: the stock ship look: {warning}"))
        .collect();
    let looks = StockShipLooks::new(Rc::clone(&art), resolved.borrows);
    (Some((art, looks)), warnings)
}

/// The game's images, as `main` hands them to the renderer: `data`'s,
/// 8-bit and 32-bit sheets read as `rulebook` says, the `interface`
/// file's pictures, and the borrowed stock ship look from the `stock` art
/// [`wire`] gave alone.
#[must_use]
pub fn images(
    data: &Rc<GameData>,
    rulebook: &Rulebook,
    interface: Option<Rc<InterfaceData>>,
    stock: Option<Rc<StockArt>>,
) -> GameImages<Rc<GameData>> {
    let images = GameImages::new(Rc::clone(data), eight_bit_reading(rulebook))
        .with_thirty_two_bit(thirty_two_bit_reading(rulebook));
    let images = interface
        .into_iter()
        .fold(images, GameImages::with_interface);
    stock.into_iter().fold(images, GameImages::with_stock_art)
}

#[cfg(test)]
mod tests {
    use nova_audio::recording::MemorySettings;
    use nova_data::ShipId;
    use nova_data::stock_look::fixture::{stock_art, stock_store};
    use nova_data::stock_look::{ROLE_DEFAULTS, Scale};
    use nova_view::flight::ShipSprites;

    use super::*;

    fn look_of(text: &str) -> (StockLook, Vec<String>) {
        setting(Some(&mut MemorySettings::holding(text)))
    }

    fn on_with(ships: &[(&str, i16)]) -> StockLook {
        StockLook {
            on: true,
            ships: ships
                .iter()
                .map(|&(key, stock)| (key.to_owned(), stock))
                .collect(),
        }
    }

    #[test]
    fn without_a_setting_the_look_is_off() {
        assert_eq!(setting(None), (StockLook::default(), vec![]), "no store");
        assert_eq!(
            setting(Some(&mut MemorySettings::new())),
            (StockLook::default(), vec![]),
            "nothing saved"
        );
        for text in ["{}", r#"{"sound": false}"#, r#"{"stock_ship_look": {}}"#] {
            assert_eq!(look_of(text), (StockLook::default(), vec![]), "{text}");
        }
        assert_eq!(
            look_of(r#"{"stock_ship_look": {"on": false}}"#),
            (StockLook::default(), vec![])
        );
        assert_eq!(
            (STOCK_SHIP_LOOK, ON, SHIPS),
            ("stock_ship_look", "on", "ships")
        );
    }

    #[test]
    fn settings_that_cannot_be_used_leave_it_off_silently() {
        for text in ["not json", "", "[1, 2]", r#""stock_ship_look""#] {
            assert_eq!(look_of(text), (StockLook::default(), vec![]), "{text}");
        }
        let mut store = MemorySettings::holding(r#"{"stock_ship_look": {"on": true}}"#);
        store.fail_reads(true);
        assert_eq!(setting(Some(&mut store)), (StockLook::default(), vec![]));
    }

    #[test]
    fn on_turns_it_on_and_ships_are_the_overrides() {
        assert_eq!(
            look_of(r#"{"stock_ship_look": {"on": true}}"#),
            (on_with(&[]), vec![])
        );
        assert_eq!(
            look_of(r#"{"stock_ship_look": {"on": true, "ships": {"hauler": 136, "cutter": -1}}}"#),
            (on_with(&[("hauler", 136), ("cutter", -1)]), vec![])
        );
        assert_eq!(
            look_of(r#"{"stock_ship_look": {"ships": {"hauler": 136}}}"#),
            (
                StockLook {
                    on: false,
                    ..on_with(&[("hauler", 136)])
                },
                vec![]
            ),
            "kept, though off"
        );
    }

    #[test]
    fn a_look_that_is_not_an_object_is_off_with_a_warning() {
        for value in ["true", "[]", "null", r#""on""#] {
            let text = format!(r#"{{"stock_ship_look": {value}}}"#);
            assert_eq!(
                look_of(&text),
                (
                    StockLook::default(),
                    vec![
                        "nova: the saved \"stock_ship_look\" in memory is not an object; \
                         leaving the stock ship look off"
                            .to_owned()
                    ]
                ),
                "{text}"
            );
        }
    }

    #[test]
    fn an_on_that_is_not_a_bool_is_off_with_a_warning() {
        for value in [r#""true""#, "1", "null", "{}"] {
            let text = format!(r#"{{"stock_ship_look": {{"on": {value}}}}}"#);
            assert_eq!(
                look_of(&text),
                (
                    StockLook::default(),
                    vec![
                        "nova: the saved \"stock_ship_look\".\"on\" in memory is not true or \
                         false; leaving the stock ship look off"
                            .to_owned()
                    ]
                ),
                "{text}"
            );
        }
    }

    #[test]
    fn ships_that_are_not_an_object_are_ignored_with_a_warning() {
        for value in ["true", "[136]", "null", "136"] {
            let text = format!(r#"{{"stock_ship_look": {{"on": true, "ships": {value}}}}}"#);
            assert_eq!(
                look_of(&text),
                (
                    on_with(&[]),
                    vec![
                        "nova: the saved \"stock_ship_look\".\"ships\" in memory is not an \
                         object; ignoring it"
                            .to_owned()
                    ]
                ),
                "{text}"
            );
        }
    }

    #[test]
    fn an_entry_that_is_not_a_ship_id_is_left_out_with_a_warning_naming_its_key() {
        for value in [r#""136""#, "136.5", "40000", "-40000", "null", "[]"] {
            let text = format!(
                r#"{{"stock_ship_look": {{"on": true, "ships": {{"hauler": {value}, "cutter": 137}}}}}}"#
            );
            assert_eq!(
                look_of(&text),
                (
                    on_with(&[("cutter", 137)]),
                    vec![
                        "nova: the saved \"stock_ship_look\".\"ships\".\"hauler\" in memory is \
                         not a shïp ID; leaving it out"
                            .to_owned()
                    ]
                ),
                "{text}"
            );
        }
    }

    /// A new game's store with hull 128, `hauler`, a freighter, 30 long.
    fn new_game() -> GameData {
        use nova_data::Record;
        use nova_data::ShipLine;
        use nova_data::records::fixture::ShipAnimBuilder;
        use nova_data::records::ship_anim::ShipAnim;
        let anim = ShipAnimBuilder::new()
            .base_x_size(12)
            .base_y_size(30)
            .build();
        nova_data::store::fixture::store_of(&[(ShipAnim::TYPE, 128, anim)]).with_ship_lines(
            BTreeMap::from([(
                ShipId(128),
                ShipLine {
                    chain: vec!["hauler".to_owned()],
                    role: Some(nova_data::stock_look::Role::Freighter),
                },
            )]),
        )
    }

    fn freighter() -> i16 {
        ROLE_DEFAULTS
            .iter()
            .find(|(role, _)| *role == nova_data::stock_look::Role::Freighter)
            .map(|&(_, stock)| stock)
            .expect("a default")
    }

    #[test]
    fn wiring_a_look_that_is_on_over_stock_data_gives_the_art_and_the_looks() {
        let (wired, warnings) = wire(
            &new_game(),
            &on_with(&[]),
            Some(stock_store(&[(freighter(), 40)])),
        );
        assert_eq!(warnings, Vec::<String>::new());
        let (art, looks) = wired.expect("wired");
        assert!(art.ship_sprite_info(ShipId(freighter())).is_ok());
        let sheet = looks.ship_sheet(ShipId(128)).expect("borrowed");
        assert_eq!(sheet.size(), (30, 30), "scaled to the hull's length");
        assert_eq!(
            sheet.image,
            nova_view::SheetImage::Stock {
                ship: freighter(),
                image: nova_data::ShipImage::Base,
                scale: Scale::of(30, 40),
            }
        );
    }

    #[test]
    fn wiring_gives_resolve_s_warnings_each_as_one_line() {
        let (wired, warnings) = wire(
            &new_game(),
            &on_with(&[("zeppelin", 300)]),
            Some(stock_store(&[(freighter(), 40)])),
        );
        assert!(wired.is_some());
        assert_eq!(
            warnings,
            [
                "nova: the stock ship look: 1 override names no new-game ship, starting with \
              `zeppelin`; it is ignored"
            ]
        );
    }

    #[test]
    fn a_look_that_is_off_or_has_no_stock_data_wires_nothing() {
        let stock = || Some(stock_store(&[(freighter(), 40)]));
        assert_eq!(
            wire(&new_game(), &StockLook::default(), stock()).1,
            Vec::<String>::new()
        );
        assert!(
            wire(&new_game(), &StockLook::default(), stock())
                .0
                .is_none()
        );
        assert!(wire(&new_game(), &on_with(&[]), None).0.is_none());
        assert_eq!(
            wire(&new_game(), &on_with(&[]), None).1,
            Vec::<String>::new()
        );
        let _ = stock_art(&[]);
    }
}
