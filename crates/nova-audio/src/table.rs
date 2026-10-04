//! Which `snd ` each sound event plays.
//!
//! [`SoundTable::ORIGINAL`] is the original game's, pinned by tests. Each
//! entry's source is on its field. The `snd ` names are the stock
//! resources' own (all 227 are in `Nova Sounds.ndat`); "the guide" is the
//! community *EV Nova Resource ID Guide* (github.com/andrews05/evstuff,
//! `guides/resourceidguide.html`, "snd IDs"), which lists the engine's
//! fixed sounds.

use nova_data::SoundId;

/// The `snd ` each event plays; `None` is silent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SoundTable {
    /// Loops while the ship thrusts.
    pub engine: Option<SoundId>,
    /// Plays as the ship lands, before the stellar's own landing sound.
    pub landing: Option<SoundId>,
    /// Plays as the ship takes off.
    pub take_off: Option<SoundId>,
    /// Plays as a jump begins.
    pub jump: Option<SoundId>,
    /// Plays as a jump ends, in the next system.
    pub arrival: Option<SoundId>,
    /// Plays as an interface button is pressed.
    pub button_down: Option<SoundId>,
    /// Plays as a pressed interface button comes back up.
    pub button_up: Option<SoundId>,
}

impl SoundTable {
    /// The original game's sounds.
    ///
    /// - `engine`: none. No stock `snd ` is an engine sound, and the guide
    ///   reserves no ID for one: the original thrusts in silence.
    /// - `landing`: 151, "Beep2". The guide: "151 Beep 2: ... request/grant
    ///   landing clearance". L lands at once here, so the clearance beep
    ///   marks the landing. The stellar's own sound (its `CustSndID`, 10000
    ///   and up, the guide's "custom stellar landing sounds") follows it,
    ///   from the event.
    /// - `take_off`: none. The guide lists none, and the `NovaSwift` remake
    ///   (github.com/SirStig/NovaSwift, `Game/GameScene.swift`) notes that
    ///   "lifting off a planet is silent in the original".
    /// - `jump`: 128, "Warp up" (the guide: "128 Warp up"). 129 "Warp up
    ///   x2" is for the double-speed mode, which is not modelled.
    /// - `arrival`: 130, "Warp out" (the guide: "130 Warp out").
    /// - `button_down`: 600, "Menu button down" (the resource's name, and
    ///   the guide's).
    /// - `button_up`: 601, "Menu button up" (likewise).
    pub const ORIGINAL: Self = Self {
        engine: None,
        landing: Some(SoundId(151)),
        take_off: None,
        jump: Some(SoundId(128)),
        arrival: Some(SoundId(130)),
        button_down: Some(SoundId(600)),
        button_up: Some(SoundId(601)),
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_original_table_is_pinned_field_by_field() {
        let table = SoundTable::ORIGINAL;
        assert_eq!(table.engine, None, "the original thrusts in silence");
        assert_eq!(table.landing, Some(SoundId(151)), "Beep2");
        assert_eq!(table.take_off, None, "the original takes off in silence");
        assert_eq!(table.jump, Some(SoundId(128)), "Warp up");
        assert_eq!(table.arrival, Some(SoundId(130)), "Warp out");
        assert_eq!(table.button_down, Some(SoundId(600)), "Menu button down");
        assert_eq!(table.button_up, Some(SoundId(601)), "Menu button up");
    }
}
