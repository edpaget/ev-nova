//! Device-free commands and the tables that bind each device's inputs to
//! them.
//!
//! Every game key reaches a screen as a [`Command`], looked up through
//! the app's one [`Bindings`], never compared to a [`Key`](crate::Key).
//! The keyboard is the first device: its table is [`KeyMap`], with the
//! original's defaults (`_loadKeys` @0xcd5f8-0xce6c4, recorded in
//! rdm:phase/key-bindings/phase-2-command-key-map's findings). Phase 7
//! adds the gamepad's table beside it and an `Input` variant for it; no
//! screen changes.
//!
//! # How it fits together
//!
//! - [`command`] names every command, grouped by [`Context`], each with
//!   its [`Kind`] (held or pressed) as data on its enum. A screen matches
//!   its own context's enum and passes over the rest, so no crate above
//!   this one matches [`Command`] exhaustively.
//! - [`keyboard`] is the keyboard's side, the only one that holds a
//!   `Key`: [`KeyCombo`] (a key and the [`Modifiers`] it needs), the
//!   [`KeyMap`] with its defaults and its fixed twins
//!   ([`KeyMap::TWINS`]), [`KeysDown`], the keys held, [`Keyboard`],
//!   those and what each press meant, and the lookup over a key's input.
//! - [`bindings`] holds every device's table ([`Bindings`]), the lookup
//!   ([`Bindings::resolve`], [`Bindings::held`]), the clash query
//!   ([`Bindings::clashes`], and [`Bindings::clashes_with`] for a rebind
//!   not yet made) and the keys screen's rebind rule
//!   ([`Bindings::assign`]), with [`Devices`], what each device holds,
//!   handing each input to its device's side. Neither names a `Key`.
//! - [`contexts`] lists each key-reading screen's contexts, in order.
//! - [`reader`] is what a screen holds: a [`CommandReader`] over the
//!   app's [`SharedBindings`].
//! - [`saved`] is the key map's saved form: a combo's text, the bindings
//!   that differ from the defaults ([`KeyMap::overrides`]), the map they
//!   build again ([`KeyMap::with_overrides`]) and the text table the
//!   settings file holds ([`KeyMap::to_table`], [`KeyMap::from_table`]).
//!
//! # Decisions
//!
//! - **Combos and modifiers.** A [`KeyCombo`] is a key and the modifier
//!   flags it needs (Shift, Control, Option, Command), naming no side. It
//!   is met when its key goes down with at least those held, as the
//!   original reads only what it polls; when several combos on one key
//!   are met, the one needing the most modifiers wins a press, so
//!   Option-A beats A. Whether a flag counts its right-hand key is the
//!   context's ([`keyboard::sides`]): flight's `_HandlePlayer` polls the
//!   left keys' codes (0x37, 0x3a, 0x38), and the dialogs' filters read
//!   `EventRecord.modifiers`' bits, set by either.
//! - **Variants follow their base.** The original polls Option, Shift and
//!   Command by fixed key code beside a `_keyPrefs` key, so a rebound
//!   base keeps its variant: with the target key on T, Shift-T targets
//!   backwards. Each variant is its own command (so a pad can give it its
//!   own button), whose keyboard default is [`Binding::Follows`]: its
//!   base's combo plus its modifiers. The keyboard holds that link, not
//!   [`Command`], which names no device.
//! - **One key per command, fixed twins as data.** Where the original
//!   reads two keys for one action, the command binds the one its pref
//!   or label names, and the other is a fixed twin ([`KeyMap::TWINS`]):
//!   the lookup returns the twin's command, so a screen matches no key
//!   for it. A command bound to the same key in an active context beats
//!   a twin.
//! - **Defaults fill only missing entries.** `_loadKeys` gives a pref its
//!   constant only when no entry was saved, and a cleared recorder saves
//!   `ShortcutRecorder`'s empty code, so a cleared command stays unbound
//!   ([`KeyMap::from_entries`]). The lookup never falls back: a command
//!   that lost its key answers nothing.
//! - **Held state is commands.** A [`CommandReader`] keeps the keys down
//!   on the keyboard's side and the commands they hold on the screen's:
//!   it recomputes the held commands whenever a key or modifier changes,
//!   so Command pressed after X still ejects, but presses a command only
//!   as a key goes down. A release lets go of what its press meant.
//! - **Contexts active together are data.** Each screen's list
//!   ([`contexts`]) has its own context first, then the shared UI context
//!   everywhere but flight and the intro, and last the app's own (Caps
//!   Lock's double time), which the app's router reads on every screen,
//!   flight included. An entry may be conditional, switched by the
//!   screen ([`CommandReader::set_active`]): the escort menu's group keys
//!   are on only while the menu shows, and shadow flight's stellar slots
//!   then, as `_QuickKeyCheck` hides 1-4 (@0xa106-0xa124). The clash
//!   query counts a conditional context as active together with the rest
//!   of its list, and leaves out a shadowing context's pairs with the one
//!   it shadows.
//! - **How screens get the bindings.** The app edge owns one
//!   [`SharedBindings`] and hands it to every screen it builds; a screen
//!   built without one reads `KeyMap::default()`, so tests build screens
//!   as before.
//! - **Saved form.** A command saves by its stable name, `Context.Variant`
//!   ([`Command::name`]), and a combo as its W3C modifier names then
//!   `Code`'s W3C key name, joined by `+` (`Meta+Digit5`), so the file
//!   survives a winit upgrade; an unbound command saves as `""`. Only the
//!   bindings that differ from the defaults are saved, so a later default
//!   still reaches a player who never touched that key. A pinned table
//!   holds every default's name and text, so a rename or a
//!   `keyboard-types` upgrade can't change the file unnoticed. An unknown
//!   name or key loads as nothing saved: its command keeps its default.
//! - **Conflict rules on load.** A saved table can clash: two saved
//!   commands on one key, or a saved key a later build gives another
//!   command by default. [`KeyMap::with_overrides`] unbinds one command a
//!   clash, until none is left: of two saved commands the later in
//!   `Command`'s declaration order, and against a defaulted one the
//!   defaulted one, as the player's choice wins. A variant a saved base
//!   carries counts as saved.
//! - **The rebind rule.** The keys screen records a key through
//!   [`Bindings::assign`]: the command recorded keeps the combo, and each
//!   command that clashes with it, or with a variant following it, loses
//!   its key, one at a time until none clashes. A clash is worked out on
//!   the bindings the rebind makes ([`Bindings::clashes_with`]), so a
//!   variant moved with its base counts: Closest Target on W unbinds
//!   Select Secondary, and with it its Option variant, which Closest
//!   Ship's Option-W would meet. Of two variants of one base that land on
//!   one combo, the later loses. The original refuses the key instead
//!   (rdm:phase/key-bindings/phase-4-keys-screen's findings); the
//!   roadmap chose this rule.
//!
//! [`Kind`]: command::Kind

pub mod bindings;
pub mod command;
pub mod contexts;
pub mod keyboard;
pub mod reader;
pub mod saved;
#[cfg(test)]
pub(crate) mod test_keys;

pub use bindings::{Bindings, Change, Clash, Devices};
pub use command::{
    AppCommand, AssignmentCommand, BarCommand, CommCommand, Command, Context, EscortDialogCommand,
    EscortMenuCommand, FlightCommand, HaggleCommand, HireCommand, IntroCommand, Kind,
    MainMenuCommand, MainScreenCommand, MapCommand, MissionComputerCommand, OutfitterCommand,
    PauseCommand, PlanetCommCommand, PlayCommand, PlayerInfoCommand, PortCommand, RaceCommand,
    ShipyardCommand, SystemViewCommand, TradeCommand, UiCommand,
};
pub use contexts::ScreenContexts;
pub use keyboard::{Binding, KeyCombo, KeyMap, Keyboard, KeysDown, Modifiers, is_modifier_input};
pub use reader::{CommandInput, CommandReader, SharedBindings};
pub use saved::ComboTextError;
