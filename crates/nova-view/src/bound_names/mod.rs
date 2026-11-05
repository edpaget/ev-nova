//! The one function every text that names a key goes through
//! ([`name_command`]): a command and the device to name it for in, the
//! name of the player's input for it out.
//!
//! A text names a command, never a key: the bindings say which key that
//! is, the player's layout ([`KeyNames`]) what it is called, and the
//! device ([`Device`], from the screen's [`CommandReader::device`]) which
//! table to read. Only the keyboard is named today. Phase 12
//! (rdm:phase/key-bindings/phase-12-gamepad-rebind-glyphs) names the pad's
//! buttons through the same call, so no text is written twice.
//!
//! # How a text spells its key ([`Spelling`])
//!
//! - **[`Spelling::Bound`]**: the command's combo, named through the
//!   layout ([`combo_name`]): `W`, `Cmd-K`, `Up Arrow`, and `Z` for the key
//!   at the US W on AZERTY. Every text of the engine's own uses it.
//! - **[`Spelling::Prose`]**: the same, spelled out where a sentence names
//!   the key: `Escape`, not `Esc` (the fallback prompts' "Press Return or
//!   Escape").
//! - **[`Spelling::Str129`]**: the tutorial hints, which the original names
//!   from `STR#` 129 by the bound key's Mac key code ([`str129`]), ignoring
//!   the layout. Under the `hint_key_names` rule's other reading
//!   (`nova_sim::RuleKey::HintKeyNames`), through the layout, as
//!   [`Spelling::Bound`].
//! - **[`Spelling::Fixed`]**: a name the original prints whatever is
//!   bound, such as the escort menu's row digits.
//! - **[`Spelling::Twin`]**: a fixed twin a help line names beside the
//!   command's key (the system view's WASD), named through the layout.
//!
//! A command left unbound has no name: a hint prints nothing in its
//! place (`Hit ''`, as the original's empty `STR#` 129 entry does), a
//! help line leaves the entry out, and a prompt drops it from its "or".
//!
//! # Every text that names a key
//!
//! Recorded by rdm:phase/key-bindings/phase-6-bound-key-names's findings,
//! with the original's addresses there.
//!
//! | # | Text | Commands named | Spelling |
//! |---|---|---|---|
//! | 1 | flight's land hint (`flight::view`, `_HandlePlayer` @0x70b01-0x70b1b) | Land | [`Spelling::Str129`] |
//! | 2 | flight's hyperspace hint (@0x70c3d-0x70cb2) | Galaxy Map, Hyper Jump | [`Spelling::Str129`] |
//! | 3 | the escort menu's rows (`flight::escorts`, `_DrawEscortMenuInFrame` @0x5649a) | Group 1-5 | [`Spelling::Fixed`], the row's digit |
//! | 4 | flight's help line, a developer overlay | about twenty flight and play commands, Pause | [`Spelling::Bound`] |
//! | 5 | the galaxy map's help line | the arrows, zoom, Confirm, Find | [`Spelling::Bound`], [`Spelling::Twin`] S, [`Spelling::Fixed`] `+` (the keypad's) |
//! | 6 | the galaxy map's Enter label | Confirm | [`Spelling::Bound`] |
//! | 7 | the system view's help line | the arrows, Activate, Back | [`Spelling::Bound`], [`Spelling::Twin`] WASD |
//! | 8 | the ship browser's footer | Left, Right | [`Spelling::Bound`] |
//! | 9 | `nova`'s router hint (`nova::app::screen`) | Tab, Fly, About, Preferences | [`Spelling::Bound`] |
//! | 10-21 | the twelve "Cannot show …" fallback prompts | each screen's Confirm and Back, or its own | [`Spelling::Prose`] ([`press_clause`]) |
//! | 22 | the keys screen's rows (`ui::keys`) | every command | [`Spelling::Bound`], the keyboard's column |
//!
//! Left as text: flight help's `Cmd-,` (the menu bar's Preferences
//! chord, not a command), the macOS menu bar's own key equivalents (drawn
//! by the OS), the comm dialog's and the mission offer's button letters
//! (button titles, not commands), and stock `dësc` prose that names keys
//! (5200, 5217, 5240: mission data the original prints verbatim).

pub mod str129;

use nova_sim::RuleSource;

use crate::input::{Device, Key};
use crate::key_names::{FIXED_NAMES, KeyNames, combo_name, key_name, modifier_words};
use crate::keymap::{Bindings, Command, CommandReader, KeyCombo, UiCommand};

pub use str129::{STR_129, mac_code, str129_name};

/// How a text spells the key it names (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spelling {
    /// The command's binding, named through the layout.
    Bound,
    /// The command's binding, named through the layout, with the names a
    /// sentence spells out in full (`Escape`).
    Prose,
    /// The binding's key from `STR#` 129 under [`RuleSource::Engine`], or
    /// through the layout under [`RuleSource::Bible`].
    Str129(RuleSource),
    /// A name the original prints whatever is bound.
    Fixed(&'static str),
    /// A fixed twin the text names beside the command, through the layout.
    Twin(Key),
}

/// What `command` is called on `device`, as a text spells it: through
/// `bindings` and the player's layout `names`. `None` for a command with
/// no binding on the device, where the spelling reads one.
///
/// Every device names the keyboard's key for now: the mouse has no
/// binding of its own, and pad buttons get names (by the pad's family)
/// in phase 12 (rdm:phase/key-bindings/phase-12-gamepad-rebind-glyphs), so
/// no text changes with the device last used until then.
#[must_use]
pub fn name_command(
    bindings: &Bindings,
    names: &dyn KeyNames,
    command: impl Into<Command>,
    device: Device,
    spelling: Spelling,
) -> Option<String> {
    let command = command.into();
    match device {
        Device::Keyboard | Device::Mouse | Device::Gamepad(_) => {
            let combo = || bindings.keyboard.combo(command);
            match spelling {
                Spelling::Bound | Spelling::Str129(RuleSource::Bible) => {
                    combo().map(|combo| combo_name(names, combo))
                }
                Spelling::Prose => combo().map(|combo| prose_name(names, combo)),
                Spelling::Str129(RuleSource::Engine) => {
                    combo().map(|combo| str129_combo_name(names, combo))
                }
                Spelling::Fixed(text) => Some(text.to_owned()),
                Spelling::Twin(key) => Some(key_name(names, key)),
            }
        }
    }
}

/// What `command` is called on the device `reader` reads, as a text
/// spells it, through the bindings and the layout `reader` holds:
/// [`name_command`] for a screen.
#[must_use]
pub fn name_read(
    reader: &CommandReader,
    command: impl Into<Command>,
    spelling: Spelling,
) -> Option<String> {
    let shared = reader.bindings();
    shared.with(|bindings| {
        name_command(
            bindings,
            shared.names().as_ref(),
            command,
            reader.device(),
            spelling,
        )
    })
}

/// The keys whose fixed names a sentence spells out in full, each with
/// that spelling.
const PROSE_NAMES: [(Key, &str); 1] = [(Key::Escape, "Escape")];

/// `combo`'s name in a sentence: [`combo_name`], with [`PROSE_NAMES`]'s
/// key spelled out.
fn prose_name(names: &dyn KeyNames, combo: KeyCombo) -> String {
    let Some(&(_, spelled)) = PROSE_NAMES.iter().find(|&&(key, _)| key == combo.key) else {
        return combo_name(names, combo);
    };
    let mut name = modifier_words(combo.modifiers);
    name.push_str(spelled);
    name
}

/// `combo`'s name as the original's hints give it: the modifiers' words,
/// then the key's `STR#` 129 entry, which may be empty. A key the Mac has
/// no code for falls back to its layout name, so a working binding is
/// never hidden.
fn str129_combo_name(names: &dyn KeyNames, combo: KeyCombo) -> String {
    let mut name = modifier_words(combo.modifiers);
    match str129_name(combo.key) {
        Some(entry) => name.push_str(entry),
        None => name.push_str(&key_name(names, combo.key)),
    }
    name
}

/// The four arrows' fixed names, which [`group`] writes as `Arrows`.
fn arrow_names() -> [&'static str; 4] {
    [
        Key::ArrowUp,
        Key::ArrowDown,
        Key::ArrowLeft,
        Key::ArrowRight,
    ]
    .map(|arrow| {
        FIXED_NAMES
            .iter()
            .find_map(|&(key, name)| (key == arrow).then_some(name))
            .unwrap_or_default()
    })
}

/// The suffix an arrow's fixed name ends with, which [`group`] drops when
/// every name it joins is an arrow's.
const ARROW_SUFFIX: &str = " Arrow";

/// Several commands' names written as one, `separator` between them:
/// `Arrows` for the four arrows, in any order; `Left/Right` for arrows
/// alone, without their ` Arrow`; and otherwise each name as it is
/// (`F/D/V/C`, `WASD`). A command with no name is left out, and `None`
/// is all there is when none has one.
#[must_use]
pub fn group(names: impl IntoIterator<Item = Option<String>>, separator: &str) -> Option<String> {
    let names: Vec<String> = names.into_iter().flatten().collect();
    if names.is_empty() {
        return None;
    }
    let arrows = arrow_names();
    let is_arrow = |name: &String| arrows.contains(&name.as_str());
    if names.len() == arrows.len()
        && arrows
            .iter()
            .all(|arrow| names.iter().any(|name| name == arrow))
    {
        return Some("Arrows".to_owned());
    }
    if names.iter().all(is_arrow) {
        let short: Vec<&str> = names
            .iter()
            .map(|name| name.strip_suffix(ARROW_SUFFIX).unwrap_or(name))
            .collect();
        return Some(short.join(separator));
    }
    Some(names.join(separator))
}

/// The shared Confirm and Back, which every screen that couldn't lay
/// itself out reads to leave, in the order its prompt names them.
pub const CONFIRM_OR_BACK: [Command; 2] = [
    Command::Ui(UiCommand::Confirm),
    Command::Ui(UiCommand::Back),
];

/// A fallback prompt's last sentence, for a screen that couldn't lay
/// itself out: ` Press Return or Escape to close it.`, leading space
/// included, naming each of `commands` on the device `reader` reads
/// ([`Spelling::Prose`]) joined by " or ", and then `to`. An unbound
/// command is left out, and the sentence is empty when none is bound.
#[must_use]
pub fn press_clause(reader: &CommandReader, commands: &[Command], to: &str) -> String {
    let keys: Vec<String> = commands
        .iter()
        .filter_map(|&command| name_read(reader, command, Spelling::Prose))
        .collect();
    if keys.is_empty() {
        return String::new();
    }
    format!(" Press {} to {to}.", keys.join(" or "))
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::key_names::NoKeyNames;
    use crate::key_names::fixture::FakeKeyNames;
    use crate::keymap::{
        EscortMenuCommand, FlightCommand, Modifiers, SharedBindings, UiCommand, contexts,
    };

    fn named(
        bindings: &Bindings,
        names: &dyn KeyNames,
        command: impl Into<Command>,
        spelling: Spelling,
    ) -> Option<String> {
        name_command(bindings, names, command, Device::Keyboard, spelling)
    }

    fn land_on(combo: KeyCombo) -> Bindings {
        Bindings::default().rebound(FlightCommand::LandDock, combo)
    }

    const ENGINE: Spelling = Spelling::Str129(RuleSource::Engine);
    const BIBLE: Spelling = Spelling::Str129(RuleSource::Bible);

    #[test]
    fn bound_names_the_binding_through_the_layout() {
        let defaults = Bindings::default();
        let weapon = FlightCommand::SelectSecondary;
        assert_eq!(
            named(&defaults, &NoKeyNames, weapon, Spelling::Bound).as_deref(),
            Some("W")
        );
        assert_eq!(
            named(&defaults, &FakeKeyNames::azerty(), weapon, Spelling::Bound).as_deref(),
            Some("Z")
        );
        assert_eq!(
            named(
                &defaults,
                &NoKeyNames,
                FlightCommand::JettisonCargo,
                Spelling::Bound
            )
            .as_deref(),
            Some("Cmd-K")
        );
        assert_eq!(
            named(
                &defaults.clone().unbound(weapon),
                &NoKeyNames,
                weapon,
                Spelling::Bound
            ),
            None
        );
    }

    #[test]
    fn prose_spells_escape_out_and_names_the_rest_as_bound_does() {
        let defaults = Bindings::default();
        assert_eq!(
            named(&defaults, &NoKeyNames, UiCommand::Back, Spelling::Prose).as_deref(),
            Some("Escape")
        );
        assert_eq!(
            named(&defaults, &NoKeyNames, UiCommand::Confirm, Spelling::Prose).as_deref(),
            Some("Return")
        );
        let shifted = defaults.clone().rebound(
            UiCommand::Back,
            KeyCombo::with(Key::Escape, Modifiers::SHIFT),
        );
        assert_eq!(
            named(&shifted, &NoKeyNames, UiCommand::Back, Spelling::Prose).as_deref(),
            Some("Shift-Escape")
        );
        assert_eq!(
            named(
                &defaults,
                &FakeKeyNames::azerty(),
                FlightCommand::SelectSecondary,
                Spelling::Prose
            )
            .as_deref(),
            Some("Z")
        );
        assert_eq!(
            named(
                &defaults.clone().unbound(UiCommand::Back),
                &NoKeyNames,
                UiCommand::Back,
                Spelling::Prose
            ),
            None
        );
    }

    #[test]
    fn str129_names_the_bound_key_by_its_mac_code_and_the_bible_through_the_layout() {
        let azerty = FakeKeyNames::azerty();
        let on_w = land_on(KeyCombo::key(Key::KeyW));
        assert_eq!(
            named(&on_w, &azerty, FlightCommand::LandDock, ENGINE).as_deref(),
            Some("W")
        );
        assert_eq!(
            named(&on_w, &azerty, FlightCommand::LandDock, BIBLE).as_deref(),
            Some("Z")
        );
        let on_up = land_on(KeyCombo::key(Key::ArrowUp));
        assert_eq!(
            named(&on_up, &azerty, FlightCommand::LandDock, ENGINE).as_deref(),
            Some("Up")
        );
        assert_eq!(
            named(&on_up, &azerty, FlightCommand::LandDock, BIBLE).as_deref(),
            Some("Up Arrow")
        );
    }

    #[test]
    fn str129_reads_the_keypad_modifiers_and_empty_entries_as_the_table_does() {
        let pad = land_on(KeyCombo::key(Key::NumpadAdd));
        assert_eq!(
            named(&pad, &NoKeyNames, FlightCommand::LandDock, ENGINE).as_deref(),
            Some("Kpd +")
        );
        let shifted = land_on(KeyCombo::with(Key::KeyL, Modifiers::SHIFT));
        assert_eq!(
            named(&shifted, &NoKeyNames, FlightCommand::LandDock, ENGINE).as_deref(),
            Some("Shift-L")
        );
        let iso = land_on(KeyCombo::key(Key::IntlBackslash));
        assert_eq!(
            named(&iso, &NoKeyNames, FlightCommand::LandDock, ENGINE).as_deref(),
            Some("")
        );
        let unbound = Bindings::default().unbound(FlightCommand::LandDock);
        assert_eq!(
            named(&unbound, &NoKeyNames, FlightCommand::LandDock, ENGINE),
            None
        );
        assert_eq!(
            named(&unbound, &NoKeyNames, FlightCommand::LandDock, BIBLE),
            None
        );
    }

    #[test]
    fn str129_falls_back_to_the_layout_for_a_key_the_mac_has_no_code_for() {
        let media = land_on(KeyCombo::key(Key::MediaPlayPause));
        assert_eq!(
            named(&media, &NoKeyNames, FlightCommand::LandDock, ENGINE).as_deref(),
            Some("MediaPlayPause")
        );
    }

    #[test]
    fn fixed_reads_its_text_whatever_is_bound() {
        let group = EscortMenuCommand::Group1;
        let rebound = Bindings::default().rebound(group, KeyCombo::key(Key::KeyQ));
        let unbound = Bindings::default().unbound(group);
        for bindings in [Bindings::default(), rebound, unbound] {
            assert_eq!(
                named(&bindings, &NoKeyNames, group, Spelling::Fixed("1")).as_deref(),
                Some("1")
            );
        }
    }

    #[test]
    fn twin_names_its_key_through_the_layout() {
        let twin = Spelling::Twin(Key::KeyW);
        assert_eq!(
            named(
                &Bindings::default(),
                &FakeKeyNames::azerty(),
                UiCommand::Up,
                twin
            )
            .as_deref(),
            Some("Z")
        );
        assert_eq!(
            named(
                &Bindings::default().unbound(UiCommand::Up),
                &NoKeyNames,
                UiCommand::Up,
                twin
            )
            .as_deref(),
            Some("W")
        );
    }

    #[test]
    fn name_read_names_through_the_reader_s_bindings_and_layout() {
        let shared = SharedBindings::new(
            Bindings::default().rebound(UiCommand::Confirm, KeyCombo::key(Key::KeyW)),
        )
        .with_key_names(Rc::new(FakeKeyNames::azerty()));
        let reader = CommandReader::new(contexts::DIALOG).with_bindings(shared);
        assert_eq!(
            name_read(&reader, UiCommand::Confirm, Spelling::Bound).as_deref(),
            Some("Z")
        );
    }

    fn some(names: &[&str]) -> Vec<Option<String>> {
        names.iter().map(|&name| Some(name.to_owned())).collect()
    }

    #[test]
    fn the_four_arrows_in_any_order_are_arrows() {
        assert_eq!(
            group(
                some(&["Up Arrow", "Down Arrow", "Left Arrow", "Right Arrow"]),
                "/"
            )
            .as_deref(),
            Some("Arrows")
        );
        assert_eq!(
            group(
                some(&["Left Arrow", "Right Arrow", "Up Arrow", "Down Arrow"]),
                "/"
            )
            .as_deref(),
            Some("Arrows")
        );
    }

    #[test]
    fn arrows_alone_drop_their_suffix() {
        assert_eq!(
            group(some(&["Left Arrow", "Right Arrow"]), "/").as_deref(),
            Some("Left/Right")
        );
        assert_eq!(
            group(
                some(&["Up Arrow", "Up Arrow", "Left Arrow", "Right Arrow"]),
                "/"
            )
            .as_deref(),
            Some("Up/Up/Left/Right"),
            "four arrows, but not the four"
        );
    }

    #[test]
    fn other_names_join_as_they_are_and_an_unnamed_one_is_left_out() {
        assert_eq!(
            group(some(&["F", "D", "V", "C"]), "/").as_deref(),
            Some("F/D/V/C")
        );
        assert_eq!(
            group(some(&["W", "A", "S", "D"]), "").as_deref(),
            Some("WASD")
        );
        assert_eq!(
            group(some(&["W", "Down Arrow", "Left Arrow", "Right Arrow"]), "/").as_deref(),
            Some("W/Down Arrow/Left Arrow/Right Arrow")
        );
        assert_eq!(
            group([Some("F".to_owned()), None, Some("C".to_owned())], "/").as_deref(),
            Some("F/C")
        );
        assert_eq!(group([None, None], "/"), None);
    }

    #[test]
    fn press_clause_joins_the_bound_keys_with_or() {
        let commands = [
            Command::Ui(UiCommand::Confirm),
            Command::Ui(UiCommand::Back),
        ];
        let reader = CommandReader::new(contexts::DIALOG);
        assert_eq!(
            press_clause(&reader, &commands, "close it"),
            " Press Return or Escape to close it."
        );
        let rebound = CommandReader::new(contexts::DIALOG).with_bindings(SharedBindings::new(
            Bindings::default().rebound(UiCommand::Confirm, KeyCombo::key(Key::KeyQ)),
        ));
        assert_eq!(
            press_clause(&rebound, &commands, "close it"),
            " Press Q or Escape to close it."
        );
        let one = CommandReader::new(contexts::DIALOG).with_bindings(SharedBindings::new(
            Bindings::default().unbound(UiCommand::Back),
        ));
        assert_eq!(
            press_clause(&one, &commands, "take off"),
            " Press Return to take off."
        );
        let none = CommandReader::new(contexts::DIALOG).with_bindings(SharedBindings::new(
            Bindings::default()
                .unbound(UiCommand::Back)
                .unbound(UiCommand::Confirm),
        ));
        assert_eq!(press_clause(&none, &commands, "close it"), "");
    }
}
