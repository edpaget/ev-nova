//! The keys the flight view's tests press, and their key-based reads of
//! the view's command-based ones.
//!
//! Flight reads commands through its reader ([`contexts::FLIGHT`]) and
//! holds no key of its own, so the tests spell the default keys of its
//! commands as literal keys (each pinned against [`KeyMap::default`] by
//! the `keymap` module's tests), and read a key's state as flight's reader
//! reads it now: its modifiers held and the escort menu as they are.

use std::collections::BTreeSet;

use crate::keymap::{
    Command, CommandReader, Context, FlightCommand as Fc, KeyMap, contexts, is_modifier_input,
};
use crate::{Input, Key};

use super::{FlightView, PlayerCommand, key};
use crate::flight::sim_port::FlightSim;

/// Face Target's default key, the original's `autopilot`.
pub(super) const FACE_TARGET_KEY: Key = Key::KeyA;
/// The afterburner's default key.
pub(super) const AFTERBURNER_KEY: Key = Key::KeyZ;
/// The primary fire's default key.
pub(super) const FIRE_KEY: Key = Key::Space;
/// The secondary fire's default key.
pub(super) const SECONDARY_KEY: Key = Key::ControlLeft;
/// Select Secondary's default key.
pub(super) const SELECT_KEY: Key = Key::KeyW;
/// Weapon Safety's default key.
pub(super) const DESELECT_KEY: Key = Key::KeyS;
/// Target Select's default key.
pub(super) const TARGET_KEY: Key = Key::Tab;
/// Closest Target's default key.
pub(super) const NEAREST_KEY: Key = Key::KeyR;
/// Land's default key.
pub(super) const LAND_KEY: Key = Key::KeyL;
/// The map key's default.
pub(super) const MAP_KEY: Key = Key::KeyM;
/// Hyper Jump's default key.
pub(super) const JUMP_KEY: Key = Key::KeyJ;
/// Board's default key.
pub(super) const BOARD_KEY: Key = Key::KeyB;
/// Communicate's default key.
pub(super) const HAIL_KEY: Key = Key::KeyY;
/// Jettison Cargo's default key, with Command.
pub(super) const JETTISON_KEY: Key = Key::KeyK;
/// Eject's default key, with Command.
pub(super) const EJECT_KEY: Key = Key::KeyX;
/// Self-Destruct's default key, with Command.
pub(super) const SELF_DESTRUCT_KEY: Key = Key::Minus;
/// Engage Cloak's default key.
pub(super) const CLOAK_KEY: Key = Key::KeyU;
/// Show Framerate's default key.
pub(super) const FRAME_RATE_KEY: Key = Key::F12;
/// Attack Target's default key.
pub(super) const ESCORT_ATTACK_KEY: Key = Key::KeyF;
/// Defend Me's default key.
pub(super) const ESCORT_DEFEND_KEY: Key = Key::KeyD;
/// Hold Position's default key.
pub(super) const ESCORT_HOLD_KEY: Key = Key::KeyV;
/// Recall's default key.
pub(super) const ESCORT_RECALL_KEY: Key = Key::KeyC;
/// Acknowledge's default key, which closes the escort menu.
pub(super) const MENU_CLOSE_KEY: Key = Key::Enter;
/// The escort menu key's default.
pub(super) const MENU_KEY: Key = Key::KeyE;
/// The escort menu's group keys.
pub(super) const GROUP_KEYS: [Key; 5] = [
    Key::Digit1,
    Key::Digit2,
    Key::Digit3,
    Key::Digit4,
    Key::Digit5,
];
/// Each stellar slot's default key and its F-key twin.
pub(super) const STELLAR_SLOT_KEYS: [[Key; 2]; 4] = [
    [Key::Digit1, Key::F1],
    [Key::Digit2, Key::F2],
    [Key::Digit3, Key::F3],
    [Key::Digit4, Key::F4],
];
/// The nearest stellar's twin, F5.
pub(super) const NEAREST_STELLAR_KEY: Key = Key::F5;
/// The nearest stellar's default key, with Command.
pub(super) const NEAREST_STELLAR_COMMAND_KEY: Key = Key::Digit5;
/// Nav Off's default key.
pub(super) const NAV_OFF_KEY: Key = Key::Backquote;
/// Hyper Select's default key.
pub(super) const HYPER_SELECT_KEY: Key = Key::Backslash;
/// Hyperspace Mode's default key.
pub(super) const HYPERSPACE_MODE_KEY: Key = Key::KeyH;
/// Zoom Out's default key.
pub(super) const ZOOM_OUT_KEY: Key = Key::Minus;
/// Zoom In's default key.
pub(super) const ZOOM_IN_KEY: Key = Key::Equal;
/// Zoom In's twin, the keypad's `+`.
pub(super) const ZOOM_IN_KEYPAD_KEY: Key = Key::NumpadAdd;

/// Every key `KeyMap::default()` binds a command of flight's contexts
/// to, and every twin of them: the keys flight reads.
pub(super) fn every_key() -> impl Iterator<Item = Key> {
    let map = KeyMap::default();
    let flight = [Context::Flight, Context::EscortMenu, Context::Play];
    let mut keys: Vec<Key> = flight
        .iter()
        .flat_map(|context| context.commands())
        .filter_map(|command| map.combo(command).map(|combo| combo.key))
        .chain(
            KeyMap::TWINS
                .iter()
                .filter(|twin| flight.contains(&twin.context))
                .map(|twin| twin.key),
        )
        .collect();
    keys.sort();
    keys.dedup();
    keys.into_iter()
}

/// The tests' reads of flight's keys, each key read as flight's reader
/// reads it now.
#[allow(dead_code, reason = "each read is some tests' alone")]
impl<C, S: FlightSim> FlightView<C, S> {
    /// What a press of `k` would mean now.
    pub(super) fn pressed(&self, k: Key) -> Option<Command> {
        self.keys.peek(&key(k, true))
    }

    /// Whether flight holds `k`: a modifier down, or its key down holding
    /// commands no close latched.
    pub(super) fn holds_key(&self, k: Key) -> bool {
        let press = key(k, true);
        if !self.keys.is_down(&press) {
            return false;
        }
        let meant = self.keys.meanings(&press);
        is_modifier_input(&press)
            || (!meant.is_empty()
                && meant
                    .iter()
                    .all(|command| !self.latched_down.contains(command)))
    }

    /// The keys flight holds ([`FlightView::holds_key`]).
    pub(super) fn held_keys(&self) -> BTreeSet<Key> {
        self.keys_down()
            .into_iter()
            .filter(|&k| self.holds_key(k))
            .collect()
    }

    /// The keys down whose commands a close latched
    /// ([`FlightView::latched_key`]).
    pub(super) fn latched_keys(&self) -> BTreeSet<Key> {
        self.keys_down()
            .into_iter()
            .filter(|&k| self.latched_key(k))
            .collect()
    }

    /// Every key flight's reader holds down.
    fn keys_down(&self) -> Vec<Key> {
        self.keys
            .down()
            .into_iter()
            .filter_map(|press| match press {
                Input::Key { key, .. } => Some(key),
                _ => None,
            })
            .collect()
    }

    /// Whether a close latched the commands `k` holds, its key down.
    pub(super) fn latched_key(&self, k: Key) -> bool {
        self.keys
            .meanings(&key(k, true))
            .iter()
            .any(|command| self.latched_down.contains(command))
    }

    /// Notes `k` down, as flight's reader follows a key it saw go down,
    /// and nothing more.
    pub(super) fn hold_key(&mut self, k: Key) {
        self.keys.follow(&key(k, true));
    }

    /// Notes each of `keys` down ([`FlightView::hold_key`]).
    pub(super) fn hold_keys(&mut self, keys: impl IntoIterator<Item = Key>) {
        for k in keys {
            self.hold_key(k);
        }
    }

    /// Notes `k` up, and nothing more.
    pub(super) fn let_go_key(&mut self, k: Key) {
        self.keys.follow(&key(k, false));
        self.latched_down.retain(|&command| self.keys.held(command));
    }

    /// Whether flight holds no key ([`FlightView::holds_keys`]).
    pub(super) fn holds_no_key(&self) -> bool {
        !self.holds_keys()
    }

    /// The flight's [`PlayerCommand`] a press of `k` sends now.
    pub(super) fn command_of(&self, k: Key) -> Option<PlayerCommand> {
        self.pressed(k).and_then(Self::player_command)
    }

    /// A press of `k` as an escort key ([`FlightView::escort_command`]).
    pub(super) fn escort_key(&mut self, k: Key) -> bool {
        self.sync_escort_keys();
        self.pressed(k)
            .is_some_and(|command| self.escort_command(command))
    }

    /// A press of `k` as a nav key ([`FlightView::nav_command`]).
    pub(super) fn nav_key(&mut self, k: Key) {
        if let Some(command) = self.pressed(k) {
            let column = self.column_of(&key(k, true), command);
            self.nav_command(command, column);
        }
    }

    /// A press of `k` as Hyper Select or Hyperspace Mode
    /// ([`FlightView::hyperspace_command`]).
    pub(super) fn hyperspace_key(&mut self, k: Key) {
        if let Some(Command::Flight(pressed)) = self.pressed(k) {
            self.hyperspace_command(pressed);
        }
    }

    /// A press of `k` as a zoom ([`FlightView::zoom_floating`]).
    pub(super) fn zoom_floating_map(&mut self, k: Key) {
        if let Some(Command::Flight(pressed @ (Fc::ZoomOut | Fc::ZoomIn))) = self.pressed(k) {
            self.zoom_floating(pressed);
        }
    }

    /// Whether `_HandlePlayer` polls held `k` this frame
    /// ([`FlightView::slot_polled`]).
    pub(super) fn slot_key_polled(&self, k: Key, escort_loop: bool) -> Option<bool> {
        self.pressed(k)
            .and_then(|command| self.slot_polled(command, escort_loop))
    }

    /// A press of Target Select with the modifiers held now
    /// ([`FlightView::pick_target`]).
    pub(super) fn press_target_key(&mut self) {
        let pressed = match self.pressed(TARGET_KEY) {
            Some(Command::Flight(pressed)) => pressed,
            _ => Fc::TargetSelect,
        };
        self.pick_target(pressed);
    }

    /// A press of Closest Target with the modifiers held now
    /// ([`FlightView::pick_nearest`]).
    pub(super) fn press_nearest_key(&mut self) {
        let pressed = match self.pressed(NEAREST_KEY) {
            Some(Command::Flight(pressed)) => pressed,
            _ => Fc::ClosestTarget,
        };
        self.pick_nearest(pressed);
    }

    /// A press of Select Secondary with the modifiers held now
    /// ([`FlightView::select_weapon`]).
    pub(super) fn press_weapon_select(&mut self) {
        let pressed = match self.pressed(SELECT_KEY) {
            Some(Command::Flight(pressed)) => pressed,
            _ => Fc::SelectSecondary,
        };
        self.select_weapon(pressed);
    }

    /// A hail, at the navigation stellar with Option down
    /// ([`FlightView::hail_at`]).
    pub(super) fn hail(&mut self) {
        let option = self.keys.is_down(&key(Key::AltLeft, true));
        self.hail_at(option);
    }

    /// A press of the jettison key with the modifiers held now: a tap of
    /// the chord with Command down ([`FlightView::jettison_tap`]), and
    /// nothing without.
    pub(super) fn jettison(&mut self) {
        if let Some(Command::Flight(pressed @ (Fc::JettisonCargo | Fc::JettisonNonMission))) =
            self.pressed(JETTISON_KEY)
        {
            self.jettison_tap(pressed);
        }
    }

    /// Whether a press of `k`, Option held or not and the escort menu
    /// open or shut, is taken during a jump
    /// ([`FlightView::taken_in_jump`]): a key that means nothing alone
    /// (K, X) as the first command on it.
    pub(super) fn taken_during_jump(k: Key, option: bool, menu_open: bool) -> bool {
        let mut reader = CommandReader::new(contexts::FLIGHT);
        reader.set_active(Context::EscortMenu, menu_open);
        if option {
            reader.follow(&key(Key::AltLeft, true));
        }
        let press: Input = key(k, true);
        reader
            .peek(&press)
            .or_else(|| reader.bound_to(&press).first().copied())
            .is_some_and(Self::taken_in_jump)
    }
}
