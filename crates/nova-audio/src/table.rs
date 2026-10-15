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
    /// Plays as the escort menu opens or closes.
    pub escort_menu: Option<SoundId>,
    /// Plays as an escort command changes some escort's orders.
    pub escorts_commanded: Option<SoundId>,
    /// Plays as the escort menu selects a group.
    pub escort_group: Option<SoundId>,
    /// Plays as the player asks for the escort menu with no escorts.
    pub no_escorts: Option<SoundId>,
    /// Plays as a disabled former escort repairs itself and rejoins.
    pub escort_repaired: Option<SoundId>,
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
    /// Plays, unless it is still playing, while the player's ship breaks
    /// up.
    pub klaxon: Option<SoundId>,
    /// Plays as the player ejects.
    pub eject: Option<SoundId>,
    /// Plays as a sabotage outfit goes off aboard the player's ship.
    pub onboard_explosion: Option<SoundId>,
    /// Plays as a disabled player's pre-jump collapses.
    pub jump_collapsed: Option<SoundId>,
    /// Plays as a key's action is refused, whether or not the player is
    /// told why.
    pub refused: Option<SoundId>,
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
    /// - `escort_menu`: 150, `_beepSnd[0]`, which `_HandlePlayer` plays as
    ///   E opens the escort menu (@0x6de8c-0x6deb4) and closes it
    ///   (@0x6e02b-0x6e04d).
    /// - `escorts_commanded`: 151, `_beepSnd[1]`, which
    ///   `_IssueNewEscortCommand` plays after "New escort orders assigned:"
    ///   (@0x665be-0x665e1).
    /// - `escort_group`: 152, `_beepSnd[2]`, as keys 1-5 select a group
    ///   (`_HandlePlayer` @0x6e126-0x6e149).
    /// - `no_escorts`: 153, `_beepSnd[3]`, with "You don't have any
    ///   escorts." (`_HandlePlayer` @0x6de08-0x6de87).
    /// - `escort_repaired`: 154, `_beepSnd[4]`, which `_HandleShip` plays
    ///   after "Escort repaired." or "Fighter repaired." as a former
    ///   escort rejoins (@0x33bc9-0x33bd4). The original plays it on
    ///   channel 8, which is not modelled.
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
    /// - `klaxon`: 371, which `_LoadSounds` loads into `_klaxxonSnd`
    ///   (@0x1c22c-0x1c23e) and `_HandlePlayer` plays while the player's
    ///   death timer is above 1.0 and it is not already playing
    ///   (@0x6f1e4-0x6f227, `_ST_IsSoundPlaying` @0x6f200). The original
    ///   plays it at priority 15, not modelled.
    /// - `eject`: 372, "Eject", which `_LoadSounds` loads into
    ///   `_escapePodSnd` (@0x1c240-0x1c252) and `_HandlePlayer` plays as the
    ///   player ejects, with no test of whether it is still playing
    ///   (`PlaySound` @0x6f920). The original plays it at priority 50, not
    ///   modelled.
    /// - `onboard_explosion`: 300, the first of the explosions
    ///   `_LoadSounds`' loop from 0x1c200 loads from `snd ` 300 to 363 into
    ///   `_explodSnd`, which `_HandlePlayer` plays as a sabotage outfit goes
    ///   off (`_explodSnd[0]`, `PlaySound` @0x69375), with no test of
    ///   whether it is still playing. The original plays it at priority
    ///   100, not modelled.
    /// - `jump_collapsed`: 151, "Beep2", `_beepSnd[1]`, which
    ///   `_HandlePlayer` plays as a disabled player's pre-jump collapses,
    ///   after halting the warp-up sounds (`_PlaySound` @0x68aaf). The
    ///   original plays it at priority 50, not modelled.
    /// - `refused`: 153, "Beep4", `_beepSnd[3]`, the shared refusal beep,
    ///   which `_HandlePlayer` plays as it refuses the player info key
    ///   during a jump or while the ship breaks up (`PlaySound`
    ///   @0x7039a-0x703bd). The original plays it at priority 5, not
    ///   modelled.
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
        escort_menu: Some(SoundId(150)),
        escorts_commanded: Some(SoundId(151)),
        escort_group: Some(SoundId(152)),
        no_escorts: Some(SoundId(153)),
        escort_repaired: Some(SoundId(154)),
        red_alert: Some(SoundId(370)),
        jettison: Some(SoundId(154)),
        scan_failed: Some(SoundId(151)),
        smuggling: Some(SoundId(154)),
        news_open: Some(SoundId(151)),
        news_close: Some(SoundId(152)),
        klaxon: Some(SoundId(371)),
        eject: Some(SoundId(372)),
        onboard_explosion: Some(SoundId(300)),
        jump_collapsed: Some(SoundId(151)),
        refused: Some(SoundId(153)),
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
        assert_eq!(table.escort_menu, Some(SoundId(150)), "_beepSnd[0]");
        assert_eq!(table.escorts_commanded, Some(SoundId(151)), "_beepSnd[1]");
        assert_eq!(table.escort_group, Some(SoundId(152)), "_beepSnd[2]");
        assert_eq!(table.no_escorts, Some(SoundId(153)), "_beepSnd[3]");
        assert_eq!(table.escort_repaired, Some(SoundId(154)), "_beepSnd[4]");
        assert_eq!(table.red_alert, Some(SoundId(370)), "Red Alert");
        assert_eq!(table.jettison, Some(SoundId(154)), "_beepSnd[4]");
        assert_eq!(table.scan_failed, Some(SoundId(151)), "_beepSnd[1]");
        assert_eq!(table.smuggling, Some(SoundId(154)), "_beepSnd[4]");
        assert_eq!(table.news_open, Some(SoundId(151)), "_beepSnd[1]");
        assert_eq!(table.news_close, Some(SoundId(152)), "_beepSnd[2]");
        assert_eq!(table.klaxon, Some(SoundId(371)), "_klaxxonSnd");
        assert_eq!(table.eject, Some(SoundId(372)), "_escapePodSnd");
        assert_eq!(table.onboard_explosion, Some(SoundId(300)), "_explodSnd[0]");
        assert_eq!(table.jump_collapsed, Some(SoundId(151)), "_beepSnd[1]");
        assert_eq!(table.refused, Some(SoundId(153)), "_beepSnd[3]");
    }
}
