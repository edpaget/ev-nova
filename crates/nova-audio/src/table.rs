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
    /// Plays as the ship lands.
    pub landing: Option<SoundId>,
    /// Plays as the ship takes off.
    pub take_off: Option<SoundId>,
    /// Plays as a jump begins.
    pub jump: Option<SoundId>,
    /// Plays as a jump begins in double time.
    pub jump_double_time: Option<SoundId>,
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
    /// Plays as the main menu's E finds no pilot to enter the game with.
    pub enter_refused: Option<SoundId>,
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
    /// Plays as the bar's news dialog opens, and the mission info
    /// screen, the same `_beepSnd[1]`.
    pub news_open: Option<SoundId>,
    /// Plays as the bar's news dialog closes, and the mission info
    /// screen, the same `_beepSnd[2]`.
    pub news_close: Option<SoundId>,
    /// Plays as a zoom key zooms the floating map.
    pub floating_map_zoom: Option<SoundId>,
    /// Plays as an announced fleet arrives with its arrival quote.
    pub fleet_quote: Option<SoundId>,
    /// Plays as a ship calls for help.
    pub call_for_help: Option<SoundId>,
    /// Plays as an interceptor hails the player before its scan.
    pub scan_hail: Option<SoundId>,
    /// Plays as a person offers its mission on its hail.
    pub hail_offer: Option<SoundId>,
    /// Plays as a person says its hail quote in flight.
    pub hail_quote: Option<SoundId>,
    /// Plays as a nav key is taken: Nav Off, Hyperspace Mode, a stellar
    /// key or a click on a stellar.
    pub nav_selected: Option<SoundId>,
    /// Plays as Hyper Select moves through the system's links.
    pub hyper_selected: Option<SoundId>,
    /// Plays as a target key is taken: Target Select on every press, and
    /// Closest Target picking a ship other than the target.
    pub target_selected: Option<SoundId>,
    /// Plays as Option-backquote lets go of the ship target.
    pub target_cleared: Option<SoundId>,
    /// Plays as a first-flight tutorial hint is shown.
    pub tutorial_hint: Option<SoundId>,
    /// Plays as the player's ship is newly disabled or newly starts to
    /// break up.
    pub ship_downed: Option<SoundId>,
    /// Plays as the player's ship being downed fails a mission.
    pub downed_mission_failed: Option<SoundId>,
    /// Plays as a mission's special ship lost fails a mission that isn't
    /// hidden.
    pub mission_ship_lost: Option<SoundId>,
    /// Plays with the comm line said as a mission's special ship is
    /// boarded.
    pub mission_ship_boarded: Option<SoundId>,
    /// Plays as traffic control answers the player: the dock request's
    /// line and its denial, and the dock approach's greeting.
    pub traffic_control: Option<SoundId>,
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
    /// Plays as any board goes aboard: to plunder, or as a boarded person
    /// offers its mission.
    pub airlock: Option<SoundId>,
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
    /// - `landing`: 151, "Beep2". The guide: "151 Beep 2: ... request/grant
    ///   landing clearance". L lands at once here, so the clearance beep
    ///   marks the landing. The stellar's own sound (its `CustSndID`, 128
    ///   and up) is not the table's: the spaceport plays it as its ambient
    ///   sound once it is frontmost
    ///   ([`UiSound::PortAmbient`](nova_view::sound::UiSound::PortAmbient)).
    /// - `take_off`: none. The guide lists none, and the `NovaSwift` remake
    ///   (github.com/SirStig/NovaSwift, `Game/GameScene.swift`) notes that
    ///   "lifting off a planet is silent in the original".
    /// - `jump`: 128, "Warp up" (the guide: "128 Warp up"),
    ///   `_warpUpSnd[0]`, which `_HandlePlayer` plays as the warp stage
    ///   begins out of double time (@0x6b645-0x6b690).
    /// - `jump_double_time`: 129, "Warp up x2" (the stock resource's name
    ///   is "Warp up.x2"), `_warpUpSnd[1]` (loaded @0x1c12f-0x1c14f), which
    ///   `_HandlePlayer` plays in 128's place as the warp stage begins in
    ///   double time (@0x6b60d-0x6b63f, @0x6b66f-0x6b690). The session
    ///   decides both warp-ups' still-playing skips, so each event plays
    ///   its sound.
    /// - `arrival`: 130, "Warp out" (the guide: "130 Warp out"),
    ///   `_warpOutSnd` (loaded @0x1c152-0x1c16a), which `_HandlePlayer`
    ///   plays as the jump ends and the next system is set up
    ///   (@0x6bbd3-0x6bc08, or the fallback @0x6c6fc-0x6c706), and which
    ///   the hypergate (@0x63e57) and the wormhole (@0x6443d) play as well.
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
    /// - `enter_refused`: 153, `_beepSnd[3]` again, which
    ///   `_MainScreenChoice(3)` plays as the main screen's E finds no game
    ///   in progress, or a dying pilot (`PlaySound` @0x1aa89-0x1aaac).
    ///   `_LoadSounds` fills `_beepSnd` (@0x1c174) before `_DoMainScreen`
    ///   runs (`_InitEVNova` @0x47196, @0x47428).
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
    ///   it opens (`PlaySound` @0x47f36), and `_DoMissionInfoDialog` too
    ///   (@0x9e1d5).
    /// - `news_close`: 152, `_beepSnd[2]`, which `_DoNewsDialog` plays as
    ///   it closes (@0x4814e), and `_DoMissionInfoDialog` too
    ///   (@0x9e791).
    /// - `floating_map_zoom`: 150, `_beepSnd[0]`, which `_HandlePlayer`
    ///   plays as `-` or `=` zooms the floating map (@0x7103a, @0x710bc).
    /// - `fleet_quote`: 154, `_beepSnd[4]`, which `_HyperSpawnFleet` plays
    ///   as an announced fleet with a `Quote` arrives, even when its line
    ///   is empty (@0x42698-0x426bb).
    /// - `call_for_help`: 154, `_beepSnd[4]`, which `_AICallForHelp`
    ///   plays with a ship's distress call (@0x830e0-0x83109).
    /// - `scan_hail`: 154, `_beepSnd[4]`, which `_InterceptorAI` plays
    ///   with its pre-scan hail (@0x8cf9d-0x8cfbc).
    /// - `hail_offer`: 154, `_beepSnd[4]`, which
    ///   `_HandlePlayerCommunication` plays as a person offers its mission
    ///   on its hail (@0x62232-0x6223d).
    /// - `hail_quote`: 154, `_beepSnd[4]`, which `_ShowPersonHailQuote`
    ///   plays with every person's hail quote (@0x44586-0x445ad).
    /// - `nav_selected`: 150, `_beepSnd[0]`, which `_HandlePlayer` plays
    ///   on every press of Nav Off and Hyperspace Mode (@0x69b19), as a
    ///   stellar key finds a stellar (@0x69ebc), and as a click selects one
    ///   (@0x6a6c1).
    /// - `hyper_selected`: 152, `_beepSnd[2]`, as Hyper Select moves
    ///   through the system's links (@0x69c89).
    /// - `target_selected`: 152, `_beepSnd[2]`, which `_HandlePlayer`
    ///   plays on every press of Target Select (@0x69f64-0x69f6f) and as
    ///   Closest Target picks a ship other than the target
    ///   (@0x6aa32-0x6aa55) or a click in space targets one (@0x6a585).
    /// - `target_cleared`: 150, `_beepSnd[0]`, as Option-backquote lets go
    ///   of the ship target (@0x69a7d), or a click on the player's own
    ///   ship does (@0x6a585).
    /// - `tutorial_hint`: 154, `_beepSnd[4]`, as `_HandlePlayer` shows a
    ///   first-flight tutorial hint (@0x70a1c, @0x70c12, @0x70dab,
    ///   @0x70e59).
    /// - `ship_downed`: 151, `_beepSnd[1]`, which `_DamageShip` plays as
    ///   the player's ship is newly disabled (@0x3b11e) and as it newly
    ///   starts to break up (@0x3b221).
    /// - `downed_mission_failed`: 151, `_beepSnd[1]`, which `_DamageShip`
    ///   plays as either fails the player's first `Flags2` 0x0004 mission
    ///   (@0x3b04e, @0x3b2f1).
    /// - `mission_ship_lost`: 151, `_beepSnd[1]`, which
    ///   `_HandleShipDisplay` plays as a mission's special ship destroyed
    ///   (@0x2db5a) and `_DamageShip` as one disabled (@0x3af11) fails a
    ///   mission that isn't hidden.
    /// - `mission_ship_boarded`: 154, `_beepSnd[4]`, which
    ///   `_HandlePlayerBoardAttempt` plays with the comm line of a mission
    ///   ship boarded on the pickup path (@0x6556b) and the assist path
    ///   (@0x656ee).
    /// - `traffic_control`: 151, `_beepSnd[1]`, which
    ///   `_HandlePlayerDockRequest` plays with the request's line
    ///   (@0x67a69) and its denial (@0x67ec6), and
    ///   `_HandlePlayerDockApproach` with its greeting (@0x649a9).
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
    ///   @0x7039a-0x703bd) and as the jump key's refusals are told
    ///   (@0x6b89f, @0x6b9cc, @0x6ba72), and which
    ///   `_HandlePlayerCommunication` plays with every refusal of a hail,
    ///   with no target too (@0x626cf, @0x6206c, @0x6261c, @0x6264e). The
    ///   original plays it at priority 5, not modelled.
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
    /// - `airlock`: 390, "Airlock", which `_LoadSounds` loads
    ///   (@0x1c277-0x1c289) and `_HandlePlayerBoardAttempt` plays as any
    ///   board goes aboard: as a boarded person offers its mission, before
    ///   `_OfferOneMission` (@0x65907), and before `_DoPlunderDialog` on
    ///   its two plunder paths (@0x659b3, @0x65f00). The original plays it
    ///   at priority 8, not modelled.
    pub const ORIGINAL: Self = Self {
        engine: None,
        landing: Some(SoundId(151)),
        take_off: None,
        jump: Some(SoundId(128)),
        jump_double_time: Some(SoundId(129)),
        arrival: Some(SoundId(130)),
        button_down: Some(SoundId(600)),
        button_up: Some(SoundId(601)),
        alert: None,
        script_message: Some(SoundId(154)),
        escort_menu: Some(SoundId(150)),
        escorts_commanded: Some(SoundId(151)),
        escort_group: Some(SoundId(152)),
        no_escorts: Some(SoundId(153)),
        enter_refused: Some(SoundId(153)),
        escort_repaired: Some(SoundId(154)),
        red_alert: Some(SoundId(370)),
        jettison: Some(SoundId(154)),
        scan_failed: Some(SoundId(151)),
        smuggling: Some(SoundId(154)),
        news_open: Some(SoundId(151)),
        news_close: Some(SoundId(152)),
        floating_map_zoom: Some(SoundId(150)),
        fleet_quote: Some(SoundId(154)),
        call_for_help: Some(SoundId(154)),
        scan_hail: Some(SoundId(154)),
        hail_offer: Some(SoundId(154)),
        hail_quote: Some(SoundId(154)),
        nav_selected: Some(SoundId(150)),
        hyper_selected: Some(SoundId(152)),
        target_selected: Some(SoundId(152)),
        target_cleared: Some(SoundId(150)),
        tutorial_hint: Some(SoundId(154)),
        ship_downed: Some(SoundId(151)),
        downed_mission_failed: Some(SoundId(151)),
        mission_ship_lost: Some(SoundId(151)),
        mission_ship_boarded: Some(SoundId(154)),
        traffic_control: Some(SoundId(151)),
        klaxon: Some(SoundId(371)),
        eject: Some(SoundId(372)),
        onboard_explosion: Some(SoundId(300)),
        jump_collapsed: Some(SoundId(151)),
        refused: Some(SoundId(153)),
        airlock: Some(SoundId(390)),
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
    fn the_original_table_is_pinned_field_by_field() {
        let table = SoundTable::ORIGINAL;
        assert_eq!(table.engine, None, "the original thrusts in silence");
        assert_eq!(table.landing, Some(SoundId(151)), "Beep2");
        assert_eq!(table.take_off, None, "the original takes off in silence");
        assert_eq!(
            table.jump,
            Some(SoundId(128)),
            "Warp up, _warpUpSnd[0] @0x6b690"
        );
        assert_eq!(
            table.jump_double_time,
            Some(SoundId(129)),
            "Warp up x2, _warpUpSnd[1] @0x6b690"
        );
        assert_eq!(
            table.arrival,
            Some(SoundId(130)),
            "Warp out, _warpOutSnd @0x6bc08"
        );
        assert_eq!(table.button_down, Some(SoundId(600)), "Menu button down");
        assert_eq!(table.button_up, Some(SoundId(601)), "Menu button up");
        assert_eq!(table.alert, None, "the system's alert");
        assert_eq!(table.script_message, Some(SoundId(154)), "_beepSnd[4]");
        assert_eq!(table.escort_menu, Some(SoundId(150)), "_beepSnd[0]");
        assert_eq!(table.escorts_commanded, Some(SoundId(151)), "_beepSnd[1]");
        assert_eq!(table.escort_group, Some(SoundId(152)), "_beepSnd[2]");
        assert_eq!(table.no_escorts, Some(SoundId(153)), "_beepSnd[3]");
        assert_eq!(table.enter_refused, Some(SoundId(153)), "_beepSnd[3]");
        assert_eq!(table.escort_repaired, Some(SoundId(154)), "_beepSnd[4]");
        assert_eq!(table.red_alert, Some(SoundId(370)), "Red Alert");
        assert_eq!(table.jettison, Some(SoundId(154)), "_beepSnd[4]");
        assert_eq!(table.scan_failed, Some(SoundId(151)), "_beepSnd[1]");
        assert_eq!(table.smuggling, Some(SoundId(154)), "_beepSnd[4]");
        assert_eq!(table.news_open, Some(SoundId(151)), "_beepSnd[1]");
        assert_eq!(table.news_close, Some(SoundId(152)), "_beepSnd[2]");
        assert_eq!(table.floating_map_zoom, Some(SoundId(150)), "_beepSnd[0]");
        assert_eq!(table.fleet_quote, Some(SoundId(154)), "_beepSnd[4]");
        assert_eq!(table.call_for_help, Some(SoundId(154)), "_beepSnd[4]");
        assert_eq!(table.scan_hail, Some(SoundId(154)), "_beepSnd[4]");
        assert_eq!(table.hail_offer, Some(SoundId(154)), "_beepSnd[4]");
        assert_eq!(table.hail_quote, Some(SoundId(154)), "_beepSnd[4]");
        assert_eq!(table.nav_selected, Some(SoundId(150)), "_beepSnd[0]");
        assert_eq!(table.hyper_selected, Some(SoundId(152)), "_beepSnd[2]");
        assert_eq!(table.target_cleared, Some(SoundId(150)), "_beepSnd[0]");
        assert_eq!(table.target_selected, Some(SoundId(152)), "_beepSnd[2]");
        assert_eq!(table.tutorial_hint, Some(SoundId(154)), "_beepSnd[4]");
        assert_eq!(table.ship_downed, Some(SoundId(151)), "_beepSnd[1]");
        assert_eq!(
            table.downed_mission_failed,
            Some(SoundId(151)),
            "_beepSnd[1]"
        );
        assert_eq!(table.traffic_control, Some(SoundId(151)), "_beepSnd[1]");
        assert_eq!(table.mission_ship_lost, Some(SoundId(151)), "_beepSnd[1]");
        assert_eq!(
            table.mission_ship_boarded,
            Some(SoundId(154)),
            "_beepSnd[4]"
        );
        assert_eq!(table.klaxon, Some(SoundId(371)), "_klaxxonSnd");
        assert_eq!(table.eject, Some(SoundId(372)), "_escapePodSnd");
        assert_eq!(table.onboard_explosion, Some(SoundId(300)), "_explodSnd[0]");
        assert_eq!(table.jump_collapsed, Some(SoundId(151)), "_beepSnd[1]");
        assert_eq!(table.refused, Some(SoundId(153)), "_beepSnd[3]");
        assert_eq!(table.airlock, Some(SoundId(390)), "Airlock");
        assert_eq!(table.map_open, Some(SoundId(152)), "_beepSnd[2]");
        assert_eq!(table.map_close, Some(SoundId(150)), "_beepSnd[0]");
        assert_eq!(table.map_centre, Some(SoundId(154)), "_beepSnd[4]");
        assert_eq!(table.map_found, Some(SoundId(154)), "_beepSnd[4]");
        assert_eq!(table.map_not_found, Some(SoundId(153)), "_beepSnd[3]");
    }
}
