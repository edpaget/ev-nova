//! The priority each interface sound plays at: the original's own, from
//! the call site that plays it (`_PlaySound`'s and `ST_PlaySound`'s second
//! argument, or a param block's +0x14), pinned by tests. The voice list
//! ([`crate::voices`]) plays by it.
//!
//! [`SoundTable`](crate::SoundTable) says which `snd ` each event plays;
//! this says how much it matters. A fight's sounds carry their own
//! priority ([`CombatSound::priority`](nova_view::sound::CombatSound)),
//! and a flight session's sounds their own cue
//! ([`SimSound::cue`](nova_view::sound::SimSound::cue)).
//!
//! [`PORT_AMBIENT`] models sites that disagree, and plays at the priority
//! of the site it models most often (rdm task
//! `refused-and-port-ambient-priorities-by-site`).

use crate::port::Priority;

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
/// The escort menu opening or closing, `_beepSnd[0]`: `_HandlePlayer`
/// @0x6deb4, @0x6e04d.
pub const ESCORT_MENU: Priority = Priority::new(1);
/// An escort group selected, `_beepSnd[2]`: `_HandlePlayer` @0x6e149.
pub const ESCORT_GROUP: Priority = Priority::new(1);
/// "You don't have any escorts.", `_beepSnd[3]`: `_HandlePlayer`
/// @0x6de82.
pub const NO_ESCORTS: Priority = Priority::new(1);
/// The shipyard's or the hire screen's buy refused, `_beepSnd[3]`:
/// `_DoShipyardDialog` @0x5f238-0x5f256.
pub const BUY_REFUSED: Priority = Priority::new(1);
/// The main screen's E refused, `_beepSnd[3]`: `_MainScreenChoice`
/// @0x1aaac.
pub const ENTER_REFUSED: Priority = Priority::new(10);
/// The news dialog or the mission info screen opening, `_beepSnd[1]`:
/// `_DoNewsDialog` @0x47f36, `_DoMissionInfoDialog` @0x9e1d5.
pub const NEWS_OPEN: Priority = Priority::new(1);
/// The news dialog or the mission info screen closing, `_beepSnd[2]`:
/// `_DoNewsDialog` @0x4814e, `_DoMissionInfoDialog` @0x9e791.
pub const NEWS_CLOSE: Priority = Priority::new(1);
/// A floating map zoom, `_beepSnd[0]`: `_HandlePlayer` @0x7103a,
/// @0x710bc.
pub const FLOATING_MAP_ZOOM: Priority = Priority::new(1);
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
            (BUTTON, 10, "button"),
            (ROW_SLIDE, 50, "row slide"),
            (ALERT, 1, "alert"),
            (ESCORT_MENU, 1, "escort menu"),
            (ESCORT_GROUP, 1, "escort group"),
            (NO_ESCORTS, 1, "no escorts"),
            (BUY_REFUSED, 1, "buy refused"),
            (ENTER_REFUSED, 10, "enter refused"),
            (NEWS_OPEN, 1, "news open"),
            (NEWS_CLOSE, 1, "news close"),
            (FLOATING_MAP_ZOOM, 1, "floating map zoom"),
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
