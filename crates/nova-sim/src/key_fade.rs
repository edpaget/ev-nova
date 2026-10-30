//! What fades the flight cursor: which keys set `_keyPressedThisFrame`
//! (*0x3b5550), the byte `_HandleMouse` drops the cursor a level for
//! (@0x38427-0x3844a). `_PlayGame` clears it once a frame (@0x45b3b),
//! and `_HandlePlayer` writes it from its key blocks, each past its own
//! gates. The 29 writes come down to the [`FadeKey`] rows, and a pass's
//! answer is a [`KeyFades`] ([`FlightFrame::key_fades`](crate::flight_frame::FlightFrame::key_fades)),
//! decided by [`KeyFades::of`] from the plain [`KeyFacts`] the session
//! gathers. Which physical key reads which row is the view's.
//!
//! - **The jump needs no key.** J's block reaches its write (@0x6b113)
//!   with J down on a ship not disabled, or with +0x50 above 0, so through
//!   the pre-jump and the warp the byte is set on every pass, whatever is
//!   held (@0x6b0ed-0x6b113). Every "+0x50 ≤ 0" gate on a key then has no
//!   effect on the fade, and no row reads it.
//! - **Auto-eject needs no key.** @0x6f2cb is reached by the chord or by
//!   auto-eject armed (@0x6f23f-0x6f29a, @0x6f2c3).
//! - **No handlers, no fade.** The dead branch (@0x68853, @0x6888c), the
//!   pod's frames (@0x68ddd) and the pod's landing (@0x692ce) return to
//!   0x71113 before every key block ([`KeyFacts::handlers`]).
//! - **`_IsDisabled` (-0x2cb) holds while breaking up** (@0x2d79-0x2dcc),
//!   so it is not `Condition::Disabled` alone ([`KeyFacts::disabled`]).
//!
//! ## Left out
//!
//! - +0x4c ≤ 0 (@0x6ab15, @0x6aba0, @0x6cb62): the player's +0x4c is never
//!   above 0 (zeroed @0x66b6d).
//! - -0x2cd clear on the turn keys (@0x6c9b9): what sets it (@0x6af69,
//!   @0x6b0d3, @0x6b861, @0x6c892) has set the byte on the same pass.
//! - The arrival frame's early return (@0x6c625): that frame has +0x50
//!   above 0, so the byte is set already.
//! - `_lastKeypressTime` (*0xdec14, @0x46441-0x46456): written, never read.

/// The gate a key's write of the byte sits behind (see the module docs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FadeKey {
    /// No gate: Tab, R, `\`, W, B, Y, M, P, Return, H, \`, A, Command-K
    /// and the slot keys polled.
    Free,
    /// Not `_IsDisabled`: Up, Down, Left, Right, Z, Space, J and U.
    Helm,
    /// Not `_IsDisabled`, with a secondary selected: Ctrl (@0x6ab97).
    Secondary,
    /// A secondary selected, disabled or not: S (@0x6adda).
    WeaponSafety,
    /// The land key's gate: L.
    Land,
    /// The mission info's gate: I.
    MissionInfo,
    /// `_IsDisabled` or dying: Command-X (@0x6f22c-0x6f239).
    Eject,
    /// The countdown's start or last pass: Command-`-`
    /// (`SelfDestruct::chord_fades`).
    SelfDestruct,
}

impl FadeKey {
    /// Every gate.
    pub const ALL: [FadeKey; 8] = [
        Self::Free,
        Self::Helm,
        Self::Secondary,
        Self::WeaponSafety,
        Self::Land,
        Self::MissionInfo,
        Self::Eject,
        Self::SelfDestruct,
    ];
}

/// What a [`KeyFades`] is decided from: a pass's facts, gathered by the
/// session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // one fact a field, each a yes or no
pub struct KeyFacts {
    /// Whether a key block runs at all (see the module docs).
    pub handlers: bool,
    /// `_IsDisabled`: disabled, or breaking up.
    pub disabled: bool,
    /// Whether a jump is under way, the pre-jump included (+0x50 above 0).
    pub jump_under_way: bool,
    /// Whether a secondary weapon is selected (+0x72).
    pub secondary: bool,
    /// Whether the land key is taken.
    pub land: bool,
    /// Whether the mission info is taken.
    pub mission_info: bool,
    /// Whether the self-destruct chord's pass fades
    /// (`SelfDestruct::chord_fades`).
    pub self_destruct: bool,
    /// Whether auto-eject is armed, on a ship carrying it.
    pub auto_eject: bool,
}

impl KeyFacts {
    /// Whether `key`'s gate passes.
    fn passes(self, key: FadeKey) -> bool {
        match key {
            FadeKey::Free => true,
            FadeKey::Helm => !self.disabled,
            FadeKey::Secondary => !self.disabled && self.secondary,
            FadeKey::WeaponSafety => self.secondary,
            FadeKey::Land => self.land,
            FadeKey::MissionInfo => self.mission_info,
            FadeKey::Eject => self.disabled,
            FadeKey::SelfDestruct => self.self_destruct,
        }
    }
}

/// The [`FadeKey`]s that fade the cursor now, and whether it fades with
/// no key at all, as a set ([`FlightFrame::key_fades`](crate::flight_frame::FlightFrame::key_fades)).
/// The default is [`KeyFades::EVERY`], as a flight that failed to start
/// takes everything ([`Gates`](crate::flight_frame::Gates)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyFades(u16);

/// The bit that says the cursor fades with no key.
const UNBIDDEN: u16 = 1 << FadeKey::ALL.len();

impl KeyFades {
    /// Every key fades, and nothing fades with no key.
    pub const EVERY: Self = Self(UNBIDDEN - 1);

    /// Nothing fades.
    pub const NONE: Self = Self(0);

    /// What fades on a pass with these `facts` (see the module docs).
    #[must_use]
    pub fn of(facts: KeyFacts) -> Self {
        if !facts.handlers {
            return Self::NONE;
        }
        let keys = FadeKey::ALL
            .into_iter()
            .filter(|&key| facts.passes(key))
            .fold(Self::NONE, |fades, key| Self(fades.0 | Self::bit(key)));
        if facts.jump_under_way || facts.auto_eject {
            keys.with_unbidden()
        } else {
            keys
        }
    }

    /// Whether a key behind `key`'s gate fades the cursor.
    #[must_use]
    pub fn fades(self, key: FadeKey) -> bool {
        self.0 & Self::bit(key) != 0
    }

    /// Whether the cursor fades with no key at all.
    #[must_use]
    pub fn unbidden(self) -> bool {
        self.0 & UNBIDDEN != 0
    }

    /// These fades with `key` refused, the rest as they are.
    #[must_use]
    pub fn refusing(self, key: FadeKey) -> Self {
        Self(self.0 & !Self::bit(key))
    }

    /// These fades, fading with no key too.
    #[must_use]
    pub fn with_unbidden(self) -> Self {
        Self(self.0 | UNBIDDEN)
    }

    /// `key`'s bit.
    fn bit(key: FadeKey) -> u16 {
        1 << key as u16
    }
}

impl Default for KeyFades {
    /// [`KeyFades::EVERY`].
    fn default() -> Self {
        Self::EVERY
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A pass of an intact ship flying: nothing selected, every gate of
    /// the session's open.
    const FLYING: KeyFacts = KeyFacts {
        handlers: true,
        disabled: false,
        jump_under_way: false,
        secondary: false,
        land: true,
        mission_info: true,
        self_destruct: true,
        auto_eject: false,
    };

    /// The gates that fade under `fades`.
    fn fading(fades: KeyFades) -> Vec<FadeKey> {
        FadeKey::ALL
            .into_iter()
            .filter(|&key| fades.fades(key))
            .collect()
    }

    #[test]
    fn flying_every_gate_passes_but_those_a_fact_must_open() {
        let fades = KeyFades::of(FLYING);
        assert_eq!(
            fading(fades),
            [
                FadeKey::Free,
                FadeKey::Helm,
                FadeKey::Land,
                FadeKey::MissionInfo,
                FadeKey::SelfDestruct,
            ]
        );
        assert!(!fades.unbidden());
    }

    #[test]
    fn a_secondary_opens_ctrl_and_s() {
        let fades = KeyFades::of(KeyFacts {
            secondary: true,
            ..FLYING
        });
        assert!(fades.fades(FadeKey::Secondary));
        assert!(fades.fades(FadeKey::WeaponSafety));
    }

    #[test]
    fn disabled_refuses_the_helm_and_ctrl_and_opens_the_eject() {
        let fades = KeyFades::of(KeyFacts {
            disabled: true,
            secondary: true,
            ..FLYING
        });
        assert_eq!(
            fading(fades),
            [
                FadeKey::Free,
                FadeKey::WeaponSafety,
                FadeKey::Land,
                FadeKey::MissionInfo,
                FadeKey::Eject,
                FadeKey::SelfDestruct,
            ]
        );
    }

    #[test]
    fn each_of_the_sessions_gates_refuses_its_own_key_alone() {
        for (facts, key) in [
            (
                KeyFacts {
                    land: false,
                    ..FLYING
                },
                FadeKey::Land,
            ),
            (
                KeyFacts {
                    mission_info: false,
                    ..FLYING
                },
                FadeKey::MissionInfo,
            ),
            (
                KeyFacts {
                    self_destruct: false,
                    ..FLYING
                },
                FadeKey::SelfDestruct,
            ),
        ] {
            assert_eq!(
                KeyFades::of(facts),
                KeyFades::of(FLYING).refusing(key),
                "{key:?}"
            );
        }
    }

    #[test]
    fn a_jump_or_auto_eject_each_fade_with_no_key_and_refuse_no_key() {
        for facts in [
            KeyFacts {
                jump_under_way: true,
                ..FLYING
            },
            KeyFacts {
                auto_eject: true,
                ..FLYING
            },
        ] {
            let fades = KeyFades::of(facts);
            assert!(fades.unbidden(), "{facts:?}");
            assert_eq!(fading(fades), fading(KeyFades::of(FLYING)), "{facts:?}");
        }
    }

    #[test]
    fn with_no_handlers_nothing_fades_a_jump_included() {
        let fades = KeyFades::of(KeyFacts {
            handlers: false,
            jump_under_way: true,
            auto_eject: true,
            ..FLYING
        });
        assert_eq!(fades, KeyFades::NONE);
        assert!(!fades.unbidden());
        assert_eq!(fading(fades), []);
    }

    #[test]
    fn the_default_is_every_key_and_no_fade_unbidden() {
        assert_eq!(KeyFades::default(), KeyFades::EVERY);
        assert_eq!(fading(KeyFades::EVERY), FadeKey::ALL);
        assert!(!KeyFades::EVERY.unbidden());
    }

    #[test]
    fn refusing_takes_out_one_gate_and_with_unbidden_adds_only_the_fade() {
        let fades = KeyFades::EVERY.refusing(FadeKey::Land);
        assert!(!fades.fades(FadeKey::Land));
        assert_eq!(fading(fades).len(), FadeKey::ALL.len() - 1);
        let fades = KeyFades::NONE.with_unbidden();
        assert!(fades.unbidden());
        assert_eq!(fading(fades), []);
    }
}
