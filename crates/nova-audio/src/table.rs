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
    /// Plays as the interface refuses what was asked of it.
    pub alert: Option<SoundId>,
    /// Plays as a `Q` set operator's message is shown in flight.
    pub script_message: Option<SoundId>,
    /// Plays as the player becomes threatened: the Red Alert.
    pub red_alert: Option<SoundId>,
    /// Plays as cargo is jettisoned in flight.
    pub jettison: Option<SoundId>,
    /// Plays as a scan fails a mission whose cargo it found.
    pub scan_failed: Option<SoundId>,
    /// Plays as a scan catches the player smuggling.
    pub smuggling: Option<SoundId>,
    /// Plays as the bar's news dialog opens.
    pub news_open: Option<SoundId>,
    /// Plays as the bar's news dialog closes.
    pub news_close: Option<SoundId>,
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
    /// - `alert`: none. The original calls `SysBeep`, the system's alert
    ///   (as the Text Input dialog refuses a name too long,
    ///   `_EVTextInputDialog` @0x554a9-0x554e7), which no stock `snd `
    ///   holds.
    /// - `script_message`: 154, the fifth of the beeps `_LoadSounds` loads
    ///   from `snd ` 150 to 154 into `_beepSnd` (@0x1c174), which
    ///   `_PlayGame` plays as it shows a `Q`'s message in flight
    ///   (`_beepSnd[4]`, @0x4657a).
    /// - `red_alert`: 370, "Red Alert", which `_LoadSounds` loads into
    ///   `_redAlertSnd` (@0x1c218-0x1c22a) and `_HandlePlayer` plays
    ///   (@0x68cff). The original plays it at priority 5; the audio port
    ///   has no channel priorities, so that is not modelled.
    /// - `jettison`: 154, `_beepSnd[4]` again, which `_JettisonCargo`
    ///   plays when anything went in flight (@0x3e31a-0x3e33d), at
    ///   priority 1, not modelled either.
    /// - `scan_failed`: 151, `_beepSnd[1]`, which `_ScanPlayer` plays as
    ///   it fails a mission it found that is not hidden (@0x7e721-0x7e740),
    ///   at priority 1.
    /// - `smuggling`: 154, `_beepSnd[4]`, which `_ScanPlayer` plays as it
    ///   fines mission cargo or finds a `jünk` or an outfit
    ///   (@0x7ea3e-0x7ea5d, @0x7ee76-0x7ee95, @0x7f337-0x7f356), at
    ///   priority 1.
    /// - `news_open`: 151, `_beepSnd[1]`, which `_DoNewsDialog` plays as
    ///   it opens (`PlaySound` @0x47f36).
    /// - `news_close`: 152, `_beepSnd[2]`, which `_DoNewsDialog` plays as
    ///   it closes (@0x4814e).
    pub const ORIGINAL: Self = Self {
        engine: None,
        landing: Some(SoundId(151)),
        take_off: None,
        jump: Some(SoundId(128)),
        arrival: Some(SoundId(130)),
        button_down: Some(SoundId(600)),
        button_up: Some(SoundId(601)),
        alert: None,
        script_message: Some(SoundId(154)),
        red_alert: Some(SoundId(370)),
        jettison: Some(SoundId(154)),
        scan_failed: Some(SoundId(151)),
        smuggling: Some(SoundId(154)),
        news_open: Some(SoundId(151)),
        news_close: Some(SoundId(152)),
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
        assert_eq!(table.alert, None, "the system's alert");
        assert_eq!(table.script_message, Some(SoundId(154)), "_beepSnd[4]");
        assert_eq!(table.red_alert, Some(SoundId(370)), "Red Alert");
        assert_eq!(table.jettison, Some(SoundId(154)), "_beepSnd[4]");
        assert_eq!(table.scan_failed, Some(SoundId(151)), "_beepSnd[1]");
        assert_eq!(table.smuggling, Some(SoundId(154)), "_beepSnd[4]");
        assert_eq!(table.news_open, Some(SoundId(151)), "_beepSnd[1]");
        assert_eq!(table.news_close, Some(SoundId(152)), "_beepSnd[2]");
    }
}
