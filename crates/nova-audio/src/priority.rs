//! The priority each sound event plays at: the original's own, from the
//! call site that plays it (`_PlaySound`'s and `ST_PlaySound`'s second
//! argument, or a param block's +0x14), pinned by tests. The voice list
//! ([`crate::voices`]) plays by it.
//!
//! [`SoundTable`](crate::SoundTable) says which `snd ` each event plays;
//! this says how much it matters. A fight's sounds carry their own
//! priority ([`CombatSound::priority`](nova_view::sound::CombatSound)).
//!
//! Two events model sites that disagree, and play at the priority of the
//! site they model most often: [`REFUSED`] and [`PORT_AMBIENT`] (rdm task
//! `refused-and-port-ambient-priorities-by-site`).

use crate::port::Priority;

/// The landing beep, `_beepSnd[1]`: the dock request's clearance beep
/// (`_HandlePlayerDockRequest` @0x67a69, and its family @0x676aa,
/// @0x676fe, @0x67e8f, @0x67ec6), and `_PlayerLandOnStellar`'s
/// (@0x632ef), all at 1.
pub const LANDING: Priority = Priority::new(1);
/// A take-off sound. The original takes off in silence, so no site gives
/// one: the lowest.
pub const TAKE_OFF: Priority = Priority::new(1);
/// The warp-up, `_warpUpSnd[0]` or `[1]`: the warp param block
/// `_LoadSounds` fills with 0x7d00 (@0x1c18e), played by
/// `ST_PlaySoundParam` @0x6b690.
pub const JUMP: Priority = Priority::new(32_000);
/// The warp-out, `_warpOutSnd`: `_HandlePlayer` @0x6bc08 and @0x6c706,
/// the hypergate @0x63e61, the wormhole @0x64447, and `_WarpUpCallback`
/// @0x32049.
pub const ARRIVAL: Priority = Priority::new(50);
/// A main-menu button's click, 600 and 601: `_PlayMainScreenClickSound`
/// @0x177d9, @0x17812, and `_TrackMainScreenButtons` @0x19687, @0x19974,
/// @0x19ac4, @0x19c38.
pub const BUTTON: Priority = Priority::new(10);
/// A main-screen slide's start or end, 602 and 603: `_MainScreenAnimIdle`
/// @0x188e6, @0x1891c. Above the buttons' 10, so a click's 600 doesn't
/// cut them off.
pub const ROW_SLIDE: Priority = Priority::new(50);
/// The alert. The original's is `SysBeep`, outside the voice list, so no
/// site gives one: the lowest.
pub const ALERT: Priority = Priority::new(1);
/// A `Q`'s message in flight, `_beepSnd[4]`: `_PlayGame` @0x4657a.
pub const SCRIPT_MESSAGE: Priority = Priority::new(1);
/// A `P` set operator's sound: `_PlayGame` plays it on the one mission
/// handle with `PlaySound(.., 0x32, ..)` @0x46343. The Bible's reading,
/// played at once over the rest, has no site of its own and plays the
/// same sound at the same priority.
pub const SCRIPT: Priority = Priority::new(50);
/// The escort menu opening or closing, `_beepSnd[0]`: `_HandlePlayer`
/// @0x6deb4, @0x6e04d.
pub const ESCORT_MENU: Priority = Priority::new(1);
/// New escort orders, `_beepSnd[1]`: `_IssueNewEscortCommand` @0x665e1.
pub const ESCORTS_COMMANDED: Priority = Priority::new(1);
/// An escort group selected, `_beepSnd[2]`: `_HandlePlayer` @0x6e149.
pub const ESCORT_GROUP: Priority = Priority::new(1);
/// "You don't have any escorts.", `_beepSnd[3]`: `_HandlePlayer`
/// @0x6de82.
pub const NO_ESCORTS: Priority = Priority::new(1);
/// The main screen's E refused, `_beepSnd[3]`: `_MainScreenChoice`
/// @0x1aaac.
pub const ENTER_REFUSED: Priority = Priority::new(10);
/// An escort repaired, `_beepSnd[4]`: `_HandleShip` @0x33bd4.
pub const ESCORT_REPAIRED: Priority = Priority::new(8);
/// The player's propulsion repaired, `_beepSnd[4]`: `_HandlePlayer`'s
/// `PlaySound(.., 1, ..)` @0x68c4c, played @0x68c67.
pub const PROPULSION_REPAIRED: Priority = Priority::new(1);
/// The Red Alert, `_redAlertSnd`: `_HandlePlayer` @0x68cff.
pub const RED_ALERT: Priority = Priority::new(5);
/// Cargo jettisoned, `_beepSnd[4]`: `_JettisonCargo` @0x3e33d.
pub const JETTISON: Priority = Priority::new(1);
/// A scan failing a mission, `_beepSnd[1]`: `_ScanPlayer`'s
/// `ST_PlaySound(.., 1, ..)` @0x7e740.
pub const SCAN_FAILED: Priority = Priority::new(1);
/// A scan catching the player smuggling, `_beepSnd[4]`: `_ScanPlayer`'s
/// `ST_PlaySound(.., 1, ..)` @0x7ea5d, @0x7ee95, @0x7f356.
pub const SMUGGLING: Priority = Priority::new(1);
/// The news dialog or the mission info screen opening, `_beepSnd[1]`:
/// `_DoNewsDialog` @0x47f36, `_DoMissionInfoDialog` @0x9e1d5.
pub const NEWS_OPEN: Priority = Priority::new(1);
/// The news dialog or the mission info screen closing, `_beepSnd[2]`:
/// `_DoNewsDialog` @0x4814e, `_DoMissionInfoDialog` @0x9e791.
pub const NEWS_CLOSE: Priority = Priority::new(1);
/// A floating map zoom, `_beepSnd[0]`: `_HandlePlayer` @0x7103a,
/// @0x710bc.
pub const FLOATING_MAP_ZOOM: Priority = Priority::new(1);
/// A fleet's arrival quote, `_beepSnd[4]`: `_HyperSpawnFleet` @0x426bb.
pub const FLEET_QUOTE: Priority = Priority::new(1);
/// A ship's distress call, `_beepSnd[4]`: `_AICallForHelp`'s
/// `PlaySound(.., 1, ..)` @0x83109.
pub const CALL_FOR_HELP: Priority = Priority::new(1);
/// An interceptor's pre-scan hail, `_beepSnd[4]`: `_InterceptorAI`'s
/// `ST_PlaySound(.., 1, ..)` @0x8cfbc.
pub const SCAN_HAIL: Priority = Priority::new(1);
/// A reinforcement fleet approaching, `_beepSnd[4]`:
/// `_HandleReinforcements`' `PlaySound(.., 1, ..)` @0x3924b.
pub const REINFORCEMENTS: Priority = Priority::new(1);
/// A person's offer on its hail, `_beepSnd[4]`:
/// `_HandlePlayerCommunication` @0x6223d.
pub const HAIL_OFFER: Priority = Priority::new(1);
/// A person's hail quote, `_beepSnd[4]`: `_ShowPersonHailQuote`
/// @0x445ad.
pub const HAIL_QUOTE: Priority = Priority::new(1);
/// A nav key, `_beepSnd[0]`: `_HandlePlayer` @0x69b19, @0x69ebc,
/// @0x6a6c1.
pub const NAV_SELECTED: Priority = Priority::new(1);
/// Hyper Select, or a click on the floating map, `_beepSnd[2]`:
/// `_HandlePlayer` @0x69c89, @0x6a75d.
pub const HYPER_SELECTED: Priority = Priority::new(1);
/// A target key or click, `_beepSnd[2]`: `_HandlePlayer` @0x69f6f,
/// @0x6aa55, @0x6a585.
pub const TARGET_SELECTED: Priority = Priority::new(1);
/// The ship target let go, `_beepSnd[0]`: `_HandlePlayer` @0x69a7d,
/// @0x6a585.
pub const TARGET_CLEARED: Priority = Priority::new(1);
/// A tutorial hint, `_beepSnd[4]`: `_HandlePlayer` @0x70a1c, @0x70c12,
/// @0x70dab, @0x70e59.
pub const TUTORIAL_HINT: Priority = Priority::new(1);
/// The player's ship downed, `_beepSnd[1]`: `_DamageShip` @0x3b11e,
/// @0x3b221.
pub const SHIP_DOWNED: Priority = Priority::new(1);
/// A mission failed by the player's ship downed, `_beepSnd[1]`:
/// `_DamageShip` @0x3b04e, @0x3b2f1.
pub const DOWNED_MISSION_FAILED: Priority = Priority::new(1);
/// A pirate's cargo line, `_beepSnd[1]`: `_AIPlunderShipContents`
/// @0x84758.
pub const CARGO_STOLEN: Priority = Priority::new(1);
/// A pirate's stolen-escort line, `_beepSnd[1]`: `_AIPlunderShipContents`
/// @0x84908.
pub const SHIP_STOLEN: Priority = Priority::new(1);
/// A mission's special ship lost, `_beepSnd[1]`: `_HandleShipDisplay`
/// @0x2db5a, `_DamageShip` @0x3af11.
pub const MISSION_SHIP_LOST: Priority = Priority::new(1);
/// A mission's special ship boarded, `_beepSnd[4]`:
/// `_HandlePlayerBoardAttempt` @0x6556b, @0x656ee.
pub const MISSION_SHIP_BOARDED: Priority = Priority::new(8);
/// Traffic control's line, `_beepSnd[1]`: `_HandlePlayerDockRequest`
/// @0x67a69, @0x67ec6, `_HandlePlayerDockApproach` @0x649a9.
pub const TRAFFIC_CONTROL: Priority = Priority::new(1);
/// The klaxon, `_klaxxonSnd`: `_HandlePlayer` @0x6f227.
pub const KLAXON: Priority = Priority::new(15);
/// The eject, `_escapePodSnd`: `_HandlePlayer` @0x6f920.
pub const EJECT: Priority = Priority::new(50);
/// A sabotage outfit going off, `_explodSnd[0]`: `_HandlePlayer`
/// @0x69375.
pub const ONBOARD_EXPLOSION: Priority = Priority::new(100);
/// A collapsed pre-jump, `_beepSnd[1]`: `_HandlePlayer` @0x68aaf.
pub const JUMP_COLLAPSED: Priority = Priority::new(50);
/// The shared refusal beep, `_beepSnd[3]`. Most of the sites the event
/// models play it at 1: the mission info key (@0x70557), the jump key
/// (@0x6b89f, @0x6b9cc, @0x6ba72), two of the hail refusals (@0x62671,
/// and @0x626f2 from @0x626cf), the land key (@0x676aa, @0x676fe,
/// @0x67e8f) and Closest Target (@0x6aadb). The player info key
/// (@0x703bd) and two hail refusals (@0x6263f, and @0x626f2 from
/// @0x6206c) play it at 5.
pub const REFUSED: Priority = Priority::new(1);
/// The airlock, `snd ` 390: `_HandlePlayerBoardAttempt` @0x65907,
/// @0x659b3, @0x65f00.
pub const AIRLOCK: Priority = Priority::new(8);
/// The course map opening, `_beepSnd[2]`: `_HandlePlayer` @0x6a809.
pub const MAP_OPEN: Priority = Priority::new(1);
/// The course map closing, `_beepSnd[0]`: `_HandlePlayer` @0x6a85a.
pub const MAP_CLOSE: Priority = Priority::new(1);
/// A map centred on Space, `_beepSnd[4]`: `_MapFilter`'s
/// `PlaySound(.., 1, ..)` @0x140fe-0x14119.
pub const MAP_CENTRE: Priority = Priority::new(1);
/// A system found by the map's search, `_beepSnd[4]`: `_DoSystemSearch`'s
/// `PlaySound(.., 1, ..)` @0x120b6, played @0x120f3.
pub const MAP_FOUND: Priority = Priority::new(1);
/// No system found by the map's search, `_beepSnd[3]`: `_DoSystemSearch`'s
/// `PlaySound(.., 1, ..)` @0x120d8, played @0x120f3.
pub const MAP_NOT_FOUND: Priority = Priority::new(1);
/// The spaceport's ambient sound, `_planetSnd`: the planet param block
/// `_LoadSounds` fills with 0x7d00 (@0x1c1b8), played by
/// `ST_PlaySoundParam` from `_PortFilter` (@0x4f0e8, @0x4f106) and the
/// landscape's press (@0x5fe59). `_PlanetSoundCallback`'s replay of a
/// looped one as that voice ends plays at 10 (@0x55f9c).
pub const PORT_AMBIENT: Priority = Priority::new(32_000);
/// An intro picture's sound, snd 30003: `_ShowIntroScreen`'s
/// `_PlaySound(.., 0xa, ..)` @0x16ebb.
pub const INTRO_PICTURE: Priority = Priority::new(10);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_events_priority_is_pinned() {
        let pinned = [
            (LANDING, 1, "landing"),
            (TAKE_OFF, 1, "take-off"),
            (JUMP, 32_000, "warp-up"),
            (ARRIVAL, 50, "warp-out"),
            (BUTTON, 10, "button"),
            (ROW_SLIDE, 50, "row slide"),
            (ALERT, 1, "alert"),
            (SCRIPT_MESSAGE, 1, "Q message"),
            (SCRIPT, 50, "P sound"),
            (ESCORT_MENU, 1, "escort menu"),
            (ESCORTS_COMMANDED, 1, "escorts commanded"),
            (ESCORT_GROUP, 1, "escort group"),
            (NO_ESCORTS, 1, "no escorts"),
            (ENTER_REFUSED, 10, "enter refused"),
            (ESCORT_REPAIRED, 8, "escort repaired"),
            (PROPULSION_REPAIRED, 1, "propulsion repaired"),
            (RED_ALERT, 5, "red alert"),
            (JETTISON, 1, "jettison"),
            (SCAN_FAILED, 1, "scan failed"),
            (SMUGGLING, 1, "smuggling"),
            (NEWS_OPEN, 1, "news open"),
            (NEWS_CLOSE, 1, "news close"),
            (FLOATING_MAP_ZOOM, 1, "floating map zoom"),
            (FLEET_QUOTE, 1, "fleet quote"),
            (CALL_FOR_HELP, 1, "call for help"),
            (SCAN_HAIL, 1, "scan hail"),
            (REINFORCEMENTS, 1, "reinforcements"),
            (HAIL_OFFER, 1, "hail offer"),
            (HAIL_QUOTE, 1, "hail quote"),
            (NAV_SELECTED, 1, "nav selected"),
            (HYPER_SELECTED, 1, "hyper selected"),
            (TARGET_SELECTED, 1, "target selected"),
            (TARGET_CLEARED, 1, "target cleared"),
            (TUTORIAL_HINT, 1, "tutorial hint"),
            (SHIP_DOWNED, 1, "ship downed"),
            (DOWNED_MISSION_FAILED, 1, "downed mission failed"),
            (CARGO_STOLEN, 1, "cargo stolen"),
            (SHIP_STOLEN, 1, "ship stolen"),
            (MISSION_SHIP_LOST, 1, "mission ship lost"),
            (MISSION_SHIP_BOARDED, 8, "mission ship boarded"),
            (TRAFFIC_CONTROL, 1, "traffic control"),
            (KLAXON, 15, "klaxon"),
            (EJECT, 50, "eject"),
            (ONBOARD_EXPLOSION, 100, "onboard explosion"),
            (JUMP_COLLAPSED, 50, "jump collapsed"),
            (REFUSED, 1, "refused"),
            (AIRLOCK, 8, "airlock"),
            (MAP_OPEN, 1, "map open"),
            (MAP_CLOSE, 1, "map close"),
            (MAP_CENTRE, 1, "map centre"),
            (MAP_FOUND, 1, "map found"),
            (MAP_NOT_FOUND, 1, "map not found"),
            (PORT_AMBIENT, 32_000, "port ambient"),
            (INTRO_PICTURE, 10, "intro picture"),
        ];
        for (priority, number, what) in pinned {
            assert_eq!(priority.get(), number, "{what}");
        }
    }
}
