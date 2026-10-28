//! Which `snd ` each interface sound plays, and the engine loop.
//!
//! [`SoundTable::ORIGINAL`] is the original game's, pinned by tests. Each
//! entry's source is on its field. The `snd ` names are the stock
//! resources' own (all 227 are in `Nova Sounds.ndat`); "the guide" is the
//! community *EV Nova Resource ID Guide* (github.com/andrews05/evstuff,
//! `guides/resourceidguide.html`, "snd IDs"), which lists the engine's
//! fixed sounds. The priority each event plays at is
//! [`crate::priority`]'s. A flight session's sounds aren't the table's:
//! each says what it plays itself
//! ([`SimSound::cue`](nova_view::sound::SimSound::cue)).

use nova_data::SoundId;

/// The `snd ` each event plays; `None` is silent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SoundTable {
    /// Loops while the ship thrusts.
    pub engine: Option<SoundId>,
    /// Plays as an interface button is pressed.
    pub button_down: Option<SoundId>,
    /// Plays as a pressed interface button comes back up.
    pub button_up: Option<SoundId>,
    /// Plays as a main-screen row starts to slide in.
    pub row_slide_start: Option<SoundId>,
    /// Plays as a main-screen row has slid in.
    pub row_slide_end: Option<SoundId>,
    /// Plays as the interface refuses what was asked of it.
    pub alert: Option<SoundId>,
    /// Plays as the escort menu opens or closes.
    pub escort_menu: Option<SoundId>,
    /// Plays as the escort menu selects a group.
    pub escort_group: Option<SoundId>,
    /// Plays as the player asks for the escort menu with no escorts.
    pub no_escorts: Option<SoundId>,
    /// Plays as the shipyard's B or the hire screen's H asks with no ship
    /// selected, or one that cannot be bought or hired.
    pub buy_refused: Option<SoundId>,
    /// Plays as the main menu's E finds no pilot to enter the game with.
    pub enter_refused: Option<SoundId>,
    /// Plays as the bar's news dialog opens, and the mission info
    /// screen, the same `_beepSnd[1]`.
    pub news_open: Option<SoundId>,
    /// Plays as the bar's news dialog closes, and the mission info
    /// screen, the same `_beepSnd[2]`.
    pub news_close: Option<SoundId>,
    /// Plays as a zoom key zooms the floating map.
    pub floating_map_zoom: Option<SoundId>,
    /// Plays as M opens the course map in flight.
    pub map_open: Option<SoundId>,
    /// Plays as the course map closes in flight.
    pub map_close: Option<SoundId>,
    /// Plays as Space centres a map on the current system.
    pub map_centre: Option<SoundId>,
    /// Plays as a map's Find finds a system.
    pub map_found: Option<SoundId>,
    /// Plays as a map's Find finds none.
    pub map_not_found: Option<SoundId>,
}

impl SoundTable {
    /// The original game's sounds.
    ///
    /// - `engine`: none. No stock `snd ` is an engine sound, and the guide
    ///   reserves no ID for one: the original thrusts in silence.
    /// - `button_down`: 600, "Menu button down" (the resource's name, and
    ///   the guide's).
    /// - `button_up`: 601, "Menu button up" (likewise).
    /// - `row_slide_start`: 602, "Menu button start" (the stock
    ///   resource's name), which `_MainScreenAnimIdle` plays as a slide
    ///   steps from its first frame (@0x188fc).
    /// - `row_slide_end`: 603, "Menu button end", which it plays as a
    ///   slide reaches its count (@0x18932).
    /// - `alert`: none. The original calls `SysBeep`, the system's alert
    ///   (as the Text Input dialog refuses a name too long,
    ///   `_EVTextInputDialog` @0x554a9-0x554e7), which no stock `snd `
    ///   holds.
    /// - `escort_menu`: 150, `_beepSnd[0]`, which `_HandlePlayer` plays as
    ///   E opens the escort menu (@0x6de8c-0x6deb4) and closes it
    ///   (@0x6e02b-0x6e04d).
    /// - `escort_group`: 152, `_beepSnd[2]`, as keys 1-5 select a group
    ///   (`_HandlePlayer` @0x6e126-0x6e149).
    /// - `no_escorts`: 153, `_beepSnd[3]`, with "You don't have any
    ///   escorts." (`_HandlePlayer` @0x6de08-0x6de87).
    /// - `buy_refused`: 153, `_beepSnd[3]`, which `_DoShipyardDialog`'s
    ///   item 1 plays when it has no ship to buy or hire, or one it
    ///   cannot (`PlaySound` @0x5f238-0x5f256).
    /// - `enter_refused`: 153, `_beepSnd[3]` again, which
    ///   `_MainScreenChoice(3)` plays as the main screen's E finds no game
    ///   in progress, or a dying pilot (`PlaySound` @0x1aa89-0x1aaac).
    ///   `_LoadSounds` fills `_beepSnd` (@0x1c174) before `_DoMainScreen`
    ///   runs (`_InitEVNova` @0x47196, @0x47428).
    /// - `news_open`: 151, `_beepSnd[1]`, which `_DoNewsDialog` plays as
    ///   it opens (`PlaySound` @0x47f36), and `_DoMissionInfoDialog` too
    ///   (@0x9e1d5).
    /// - `news_close`: 152, `_beepSnd[2]`, which `_DoNewsDialog` plays as
    ///   it closes (@0x4814e), and `_DoMissionInfoDialog` too
    ///   (@0x9e791).
    /// - `floating_map_zoom`: 150, `_beepSnd[0]`, which `_HandlePlayer`
    ///   plays as `-` or `=` zooms the floating map (@0x7103a, @0x710bc).
    /// - `map_open`: 152, "Beep3", `_beepSnd[2]`, which `_HandlePlayer`
    ///   plays as the map key's press passes every gate, just before it
    ///   calls `_DoSystemMap` (`PlaySound` @0x6a7e4-0x6a809). A refused
    ///   press plays nothing (every gate jumps past it, to @0x6a89f).
    /// - `map_close`: 150, "Beep1", `_beepSnd[0]`, which `_HandlePlayer`
    ///   plays with no condition once `_DoSystemMap` returns
    ///   (@0x6a842-0x6a85a), however the map closed. The hypergate's map
    ///   plays neither: `_PlayerEnterHypergate` calls `_DoSystemMap`
    ///   (@0x6385b) with no `PlaySound` before or after it.
    /// - `map_centre`: 154, "Beep5", `_beepSnd[4]`, which `_MapFilter`
    ///   plays first as Space goes down, on every map (`PlaySound`
    ///   @0x14119).
    /// - `map_found`: 154, "Beep5", `_beepSnd[4]`, which
    ///   `_DoSystemSearch` plays as it selects the system found
    ///   (@0x120c6-0x120f3).
    /// - `map_not_found`: 153, "Beep4", `_beepSnd[3]`, which
    ///   `_DoSystemSearch` plays when it finds none (@0x120d0-0x120f3).
    ///   Neither plays for an empty text (@0x11e26-0x11e5d).
    pub const ORIGINAL: Self = Self {
        engine: None,
        button_down: Some(SoundId(600)),
        button_up: Some(SoundId(601)),
        row_slide_start: Some(SoundId(602)),
        row_slide_end: Some(SoundId(603)),
        alert: None,
        escort_menu: Some(SoundId(150)),
        escort_group: Some(SoundId(152)),
        no_escorts: Some(SoundId(153)),
        buy_refused: Some(SoundId(153)),
        enter_refused: Some(SoundId(153)),
        news_open: Some(SoundId(151)),
        news_close: Some(SoundId(152)),
        floating_map_zoom: Some(SoundId(150)),
        map_open: Some(SoundId(152)),
        map_close: Some(SoundId(150)),
        map_centre: Some(SoundId(154)),
        map_found: Some(SoundId(154)),
        map_not_found: Some(SoundId(153)),
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::too_many_lines)] // one assertion for each of the table's fields
    fn the_original_table_is_pinned_field_by_field() {
        let table = SoundTable::ORIGINAL;
        assert_eq!(table.engine, None, "the original thrusts in silence");
        assert_eq!(table.button_down, Some(SoundId(600)), "Menu button down");
        assert_eq!(table.button_up, Some(SoundId(601)), "Menu button up");
        assert_eq!(
            table.row_slide_start,
            Some(SoundId(602)),
            "Menu button start"
        );
        assert_eq!(table.row_slide_end, Some(SoundId(603)), "Menu button end");
        assert_eq!(table.alert, None, "the system's alert");
        assert_eq!(table.escort_menu, Some(SoundId(150)), "_beepSnd[0]");
        assert_eq!(table.escort_group, Some(SoundId(152)), "_beepSnd[2]");
        assert_eq!(table.no_escorts, Some(SoundId(153)), "_beepSnd[3]");
        assert_eq!(table.buy_refused, Some(SoundId(153)), "_beepSnd[3]");
        assert_eq!(table.enter_refused, Some(SoundId(153)), "_beepSnd[3]");
        assert_eq!(table.news_open, Some(SoundId(151)), "_beepSnd[1]");
        assert_eq!(table.news_close, Some(SoundId(152)), "_beepSnd[2]");
        assert_eq!(table.floating_map_zoom, Some(SoundId(150)), "_beepSnd[0]");
        assert_eq!(table.map_open, Some(SoundId(152)), "_beepSnd[2]");
        assert_eq!(table.map_close, Some(SoundId(150)), "_beepSnd[0]");
        assert_eq!(table.map_centre, Some(SoundId(154)), "_beepSnd[4]");
        assert_eq!(table.map_found, Some(SoundId(154)), "_beepSnd[4]");
        assert_eq!(table.map_not_found, Some(SoundId(153)), "_beepSnd[3]");
    }
}
