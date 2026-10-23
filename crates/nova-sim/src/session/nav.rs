//! The player's nav mode and its selection ([`NavMode`]), the original's
//! player +0x6a (the mode) and +0x6c (its selection), and every setter
//! of them the engine models. All addresses are in `EV Nova.app`'s
//! `EV Nova` (i386).
//!
//! **Readers.** The nav area draws by the mode (`_DrawStatusNav` −1 / 2 /
//! 3 @0x4a044, @0x4a09e, @0x4a198), and the radar hides its centre arrow
//! in mode 3, whatever the selection (@0x49cf5). J jumps only in mode 3
//! to the system selected, when the ship can jump there
//! (@0x6b11e-0x6b131): [`Session::begin_jump`] and
//! [`Session::jump_readiness`] read the selection, resolved by place, not
//! the course. The stellar selection ([`Session::nav_target`]) is read in
//! mode 2 only: by the nav brackets (`_HandleNavTargetSprites`
//! @0x2fcbf-0x2fccf), the land key (@0x6a8ed, @0x6a945), the person
//! offers and hailing (`_HandlePlayerCommunication` @0x61fac-0x61fbf).
//! A hyperspace selection is resolved by place whenever it is read, as
//! the original resolves its link slot through
//! `_FindActiveCoLocatedSystem` (@0x4be0); one with no visible system at
//! its place reads as none, so a hidden system is never named. The
//! galaxy map draws the system selected in mode 3 as a link from the
//! current system (`_DrawMap` @0xebd8-0xed31), and selects it as it
//! opens ([`Session::map_preselection`], `_DoSystemMap` @0x124ac-0x124ff).
//!
//! **Setters** (mode / selection):
//!
//! - Flight start, a new or a loaded pilot, and the escape pod's rescue
//!   (`_ResetPlayer` @0x1d4cd-0x1d4d5): off. The mode is not saved.
//! - The eject (@0x6f41b) and a destroyed player's every tick (the dead
//!   branch, @0x68718): the selection let go, the mode kept, as each
//!   writes +0x6c alone.
//! - Nav Off, backquote ([`Session::nav_off`], @0x69a94-0x69b63): off,
//!   the course kept, with [`SimSound::NavSelected`] on every press
//!   (@0x69b19). It shares its block with H ([`Session::nav_mode_keys`]).
//! - Option-backquote ([`Session::clear_target`], @0x69a17-0x69a90): the
//!   ship target let go, with [`SimSound::TargetCleared`] when there was
//!   one (@0x69a7d). The nav mode is untouched.
//! - Hyperspace Mode, H ([`Session::hyperspace_mode`], @0x69b76-0x69b89):
//!   the beep and the dock request dropped, then, unless the mode is 3 already, mode 3 with nothing
//!   selected and the course cued.
//! - The cue ([`Session::cue_course`], `_CueNextHyperRouteDest`
//!   @0xe2dc-0xe35f): when the course's first hop is a system the ship
//!   can jump to from here, mode 3 with it selected (@0xe33c-0xe34d).
//!   When the first hop has no visible system at its place, by
//!   [`RuleKey::CueUnplacedHop`]'s engine reading, mode 3 with the first
//!   Con slot that has none either, which reads as no destination (a −1
//!   = −1 match, @0xe334). Otherwise nothing changes. It makes no sound.
//! - Hyper Select, `\` ([`Session::select_next_system`],
//!   @0x69bb2-0x69cec): with links, [`SimSound::HyperSelected`]
//!   (@0x69c89), and mode 3 with the next link selected, cycling from the
//!   one selected in mode 3 and from the first in any other (@0x69c98,
//!   @0x69ca5-0x69cce). With none, the selection let go, the mode kept,
//!   in silence (@0x69cd4). The course is kept either way, as the press
//!   writes nothing else (@0x69c8e-0x69cd2); the
//!   [`HyperSelectRule::OneJumpCourse`] reading plots a one-jump course
//!   to the system selected instead.
//! - The stellar keys 1-4, F1-F4 and F5 ([`Session::select_stellar`],
//!   @0x69e1f-0x69f17): the beep whenever they find a stellar, even the
//!   one selected (@0x69ebc); then, unless it is selected in mode 2
//!   already (@0x69ec4-0x69ed6), mode 2 with it, its brackets flying in.
//! - A click in space ([`Session::click_in_space`], @0x6a5e9-0x6a717),
//!   when no ship took it and no jump is under way: the first stellar
//!   under it, passing over the one selected in mode 2 (@0x6a69d →
//!   @0x6a703), with the beep (@0x6a6c1), mode 2 with it and the fly-in;
//!   a click on none is silent.
//! - The land key ([`Session::land`], @0x6a8ed-0x6a932): with no stellar
//!   selected in mode 2, mode 2 with the nearest, in silence.
//! - Landing writes neither: the ship lands in mode 2, the land key
//!   having selected first.
//! - The take-off's tail ([`Session::take_off`], `_HandlePlayerDockRequest`
//!   @0x674d2-0x674d8): the selection let go, the mode kept, then the
//!   cue. The take-off notes the nav as it begins
//!   ([`Session::take_takeoff_nav`]), as the original's take-off redraw
//!   (@0x633a0) draws it before the deselect; the flight view holds it
//!   for a tick. That is the selection held while docked, so every
//!   take-off has one to show: the landed stellar in mode 2, which the
//!   original keeps at +0x6c as the stellar docked at
//!   (`_PlayerLandOnStellar` @0x632b2, @0x6336e, @0x6353e), or a landed
//!   `M`'s; none only after an `N` or an `M` to a system with no stellar.
//!   With a course, the land key's mode 2 replaced its mode-3 cue, so
//!   the landed stellar shows until the cue (@0x674d8 →
//!   `_CueNextHyperRouteDest` @0xe33c-0xe344) brings mode 3 back.
//! - A hyperspace arrival (@0x6be25-0x6be2b, @0x6c4cd): off, then, after
//!   the course's step, the cue, so mode 3 comes back while hops remain
//!   (the `entry` module's table).
//! - A gate's exit, which joins the take-off's tail (@0x66edb-0x66f40):
//!   the selection let go, the mode kept; the course is cleared, so the
//!   cue finds nothing. A wormhole writes nothing more (`_PlayerEnterWormhole`
//!   @0x64005), and its refusal lets go of the selection too.
//! - A hypergate (`_PlayerEnterHypergate`): off on every outcome
//!   (@0x637e4-0x637f2, @0x63fa2-0x63faa), and when it has no links.
//! - `M` or `N` in flight (`_EvalSetExp` @0x157e3, @0x158fd): the
//!   selection let go, the mode kept, as the move settles (the `entry`
//!   module's table). The developer tools' relocation, which has no
//!   original, does the same.
//! - `M` or `N` landed, at its operator, before its move is queued
//!   (`Chart::select_for_landed_move`; `M` @0x1570e-0x157f5, `N`
//!   @0x158d9-0x158fd): `M` in mode 2 selects the destination's first
//!   stellar that can be read (+0x6c, @0x156f2-0x15705), or none with
//!   none; any other `M`, and every `N`, lets go of the selection. The
//!   mode is never written, and a system that doesn't exist writes
//!   nothing (@0x15715-0x1571d, @0x158e0-0x158e8), as its move is
//!   skipped. In mode 2 the nav area names the stellar by its `spöb`
//!   name, though the system shown is still the one docked in
//!   (`_DrawStatusNav` @0x4a0fd-0x4a149), from the port's next nav
//!   redraw (see the flight view); the take-off lets go of it (@0x674d2).
//! - The port dialogs' restore ([`Session::restore_port_nav`]): each
//!   dialog saves both as it opens ([`Session::save_port_nav`]) and
//!   restores them, mode and selection exactly as saved, as it closes,
//!   after the close's whole-bar redraw (save / restore): the outfitter
//!   @0x5bafc-0x5bb12 / @0x5dbb8-0x5dbc5, the exchange @0x5dbed-0x5dc08 /
//!   @0x5e64d-0x5e65d, the shipyard @0x5e70f-0x5e71e / @0x5f8e8-0x5f8f5,
//!   the mission computer @0x5ffc0-0x5ffd0 / @0x60084-0x60096. So a
//!   landed `M` or `N` run inside one is undone as it closes. When the
//!   restored nav shows is the flight view's.
//! - The bar's restore ([`Session::restore_bar_nav`]): the bar saves both
//!   as it opens ([`Session::open_bar`], `_DoBarDialog` @0x48efe-0x48f11,
//!   the raw selection) and puts them back after each map opened from it
//!   (@0x492b7-0x492d3), keeping the save for the next map. Its own close
//!   restores nothing (@0x49414-0x4946f).
//! - The mission dialogs' maps ([`Session::map_nav`],
//!   [`Session::restore_map_nav`]): the offer (@0xa30eb-0xa30f7 /
//!   @0xa3108-0xa3110), the mission computer (@0xa3d54-0xa3d60 /
//!   @0xa3d86-0xa3d9e) and the accept's briefing text (`_ShowTextDialog`
//!   @0x5b14c-0x5b158 / @0x5b169-0x5b17d) save both before the map they
//!   open and write them back after it, over whatever the map's click or
//!   its close's cue wrote; the course is never saved, so a Shift-click's
//!   course stays. Mission info saves them (@0x9e442-0x9e44e) and writes
//!   them back only under `_missionDialog` (@0x9e45f-0x9e476), which its
//!   own map guard rules out, so its map's nav writes stand. Docked, the
//!   map's own close first puts back the +0x6c it saved as it opened
//!   (`_DoSystemMap` @0x12349, @0x13bee-0x13c06), after the bar's redraw;
//!   the flight view does both.
//! - A plain click on the course map in flight ([`Session::map_click`],
//!   `_DoSystemMap` @0x136d7-0x137c7): mode 3 with the system picked when
//!   the ship can jump there, the slot noted the first at its place
//!   (@0x13739-0x137bb); otherwise the selection let go, the mode kept
//!   (@0x13788-0x13790). Docked it writes neither (@0x136ec). A
//!   Shift-click writes neither, in flight or docked; the cue as the map
//!   closes (@0x13c16) selects the course's first hop. By
//!   [`RuleKey::MapPlotting`]'s other reading the click plots
//!   ([`Session::plot_course`]): a route cues its first hop, and no route
//!   lets go of the selection and keeps the mode.
//! - Clear Route ([`Session::clear_route`], @0x12c69-0x12dbf): the
//!   selection let go, the mode kept, the course cleared.
//! - The galaxy map's Tab or `\` ([`Session::cycle_map_selection`],
//!   `_MapFilter` @0x13f0f-0x13fac): mode 3 with the next Con slot that
//!   holds a link, stepping on from +0x6c, or with nothing; in silence,
//!   the course kept. Where a cycle outside mode 3 starts is
//!   [`RuleKey::MapCycleStart`]'s. The cue notes the slot it writes, the
//!   first at the hop's place (@0xe30f-0xe344), for the cycle to step on
//!   from.
//! - A click on the floating map ([`Session::select_on_floating_map`],
//!   `_HandlePlayer` @0x6a16c-0x6a762): on the current system's centre,
//!   mode 3 with nothing selected (@0x6a1d5-0x6a1de); on the box of one of
//!   its links ([`Session::floating_map_links`]), mode 3 with that link
//!   (@0x6a719-0x6a727). Either with [`SimSound::HyperSelected`]
//!   (`_beepSnd[2]`, @0x6a73a-0x6a75d). The flight view shows the map and
//!   tests the click.
//!
//! **Left out.** The floating map's preselect
//! (`_ShowFloatingMap` @0x11488), which only its names would read (the
//! galaxy-map-fidelity roadmap's); and the destroyed player's and the ship swap's per-frame clears
//! (@0x68712-0x68718, @0x6f41b-0x6f421). Mode 4, the hypergate map's, is
//! never drawn. The map panel's title reads the mode
//! (`_MapDialogUpdate` @0x10b68-0x10c76; see the
//! [`map_panel`](crate::map_panel) module).
//!
//! Of the landed `M`:
//!
//! - **The first stellar.** The original selects the first of the
//!   system's 16 `NavDefs` slots that isn't −1 (@0x15756-0x157f5), which
//!   is the first naming a `spöb` that loaded, so ours' first stellar
//!   that can be read is the original's. The loader (`_LoadObjectData`)
//!   sets a slot to −1 for an ID outside 128-2175 (@0x78199-0x781a3),
//!   and for one whose `spöb` didn't load (its +0x45 clear,
//!   @0x780f8-0x7817f). A missing `spöb` clears +0x45 (@0x779bd-0x779c6),
//!   and `_FlipSpob` refuses a record of 0x45d bytes or fewer
//!   (@0xa6047-0xa6051), ours' 1118-byte bound on a `spöb`. An ID past
//!   the last `spöb` reads a +0x45 the loader never writes (its pass
//!   stops at the last, @0x779cb-0x779d9) from memory `NewPtr` didn't
//!   clear (@0x1b7cd-0x1b7d4): not modelled.
//! - **A system with no `sÿst`.** For an ID in range with no `sÿst`
//!   (@0x15715-0x1571d), the original writes +0x74 and scans 16
//!   `NavDefs` words the loader never writes for a missing `sÿst`: it
//!   writes only +0x8, the links, +0x1eb, +0x1ed (@0x77a7e-0x77aaf) and
//!   +0xec (@0x7837e-0x7838c), and visits no ID past the last `sÿst`
//!   (@0x78393-0x783a1), in a table from `NewPtr` (@0x1b7f9-0x1b800).
//!   That reads uninitialised memory, so it can't be modelled; ours
//!   writes nothing, as its move skips the system.
//!
//! Off and mode 3 while docked: the original lands in mode 2
//! only (@0x6a8ed-0x6a932). In Off, a pilot resumed docked
//! (`_ResetPlayer` @0x1d4cd-0x1d4d5), a landed `M`'s +0x6c shows nowhere
//! ("Nav System Off") and the take-off lets go of it (@0x674d2), so ours,
//! with no selection in Off, writes nothing. Mode 3 docked is reachable
//! only through [`Session::land_at`], which has no original; there `M`
//! lets go of the selection.
//!
//! Of the port dialogs' save and restore: the original's mode 3 +0x6c is
//! a link slot of the current system, where ours keeps the system, and
//! mode 3 docked has no original, so its restore is not modelled apart;
//! the shops' map key (`_DoSystemMap`, with its own save and restore), as
//! ours opens no map from them (the computer's is a mission dialog's map,
//! above); and the flag at
//! @0x5ffb2-0x5ffba that gates the mission computer's branch, before its
//! save (@0x5ffc0-0x5ffd0), as ours opens the computer only where the
//! port offers Mission BBS.
//!
//! **The dock request** (`_dockTarget` / `_dockPermission`, see the
//! `clearance` module) is dropped by Nav Off and Hyperspace Mode, by a stellar key or a click
//! that changes the selection or the mode, and on every frame in mode 3;
//! the land key's own selection keeps it, and so does a script's `M` or
//! `N`.

use super::{Chart, Jump, Outbox, Session, Setup, Statics};
use crate::catalog::{StellarId, SystemId};
use crate::hyperspace::{HyperSelectRule, next_hyper_destination};
use crate::landing::nearest_stellar;
use crate::navigation::NavMode;
use crate::pilot::Pilot;
use crate::sound::SimSound;
use crate::{RuleKey, RuleSource, StellarPick};

/// How many Con slots a `sÿst` has, `Con1-Con16`: the slots the
/// original's link cycles step through.
pub(super) const CON_SLOTS: usize = 16;

/// The IDs a Con slot can name: any other is an empty slot, as the loader
/// sets it to −1 (`_LoadObjectData` @0x7806d-0x780b8).
const SYSTEM_IDS: std::ops::RangeInclusive<i16> = 128..=2175;

/// The Con slot a mode-3 selection was written from, as the original
/// keeps it at +0x6c where ours keeps the system. A setter that knows the
/// slot notes it ([`Session::cycle_map_selection`], [`Session::cue_course`]);
/// it stands while the player is still in `system` and the raw selection
/// is still `selected`, and a later cycle steps on from it. Every other
/// write of the nav forgets it ([`Chart::set_nav`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct NavSlot {
    system: SystemId,
    slot: usize,
    selected: SystemId,
}

/// The nav mode and its selection a mission dialog saves before the map
/// it opens and writes back after it (+0x6a and +0x6c: the offer
/// @0xa30eb / @0xa3108, the mission computer @0xa3d54 / @0xa3d86, the
/// text dialog @0x5b14c / @0x5b169; mission info saves them @0x9e442 and
/// never writes them back). Ours keeps the raw selection and the Con slot
/// noted for it (`NavSlot`), which together are the original's +0x6c.
/// Taken with [`Session::map_nav`], put back with
/// [`Session::restore_map_nav`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SavedNav {
    nav: NavMode,
    slot: Option<NavSlot>,
}

impl Session {
    /// The player's nav mode and its selection, a hyperspace selection
    /// resolved by place: the visible system at its place, or none when
    /// there is none or it isn't in the catalog, as
    /// `_FindActiveCoLocatedSystem` gives −1 for a system with no `sÿst`
    /// (see the module docs).
    #[must_use]
    pub fn nav_mode(&self) -> NavMode {
        self.chart.nav_mode()
    }

    /// The nav mode and selection as the last take-off began, before
    /// the stay's days and its deselect, or `None` once taken: what the
    /// original's take-off redraw (`_PlayerLandOnStellar` @0x633a0) draws,
    /// as its deselect (@0x674d2) comes later in the same dock request
    /// (see the flight view's `took_off`).
    pub fn take_takeoff_nav(&mut self) -> Option<NavMode> {
        self.out.take_takeoff_nav()
    }

    /// The system the galaxy map selects as it opens, as `_DoSystemMap`
    /// does (@0x124ac-0x124ff): with a system selected in mode 3, that
    /// system resolved by place, or `None` when no visible system stands
    /// there (`_FindActiveCoLocatedSystem` gives −1, @0x124e6); otherwise
    /// the current system (@0x124f6-0x124ff). The original's on-planet
    /// branch (@0x124bd) gives the current system too, and ours' map
    /// opens only in flight. After the map's Tab / `\` cycle
    /// ([`Session::cycle_map_selection`]) it is the map's new selection,
    /// as `_MapFilter` sets `_selSystem` by the same rule
    /// (@0x1408c-0x140d8).
    #[must_use]
    pub fn map_preselection(&self) -> Option<SystemId> {
        self.chart.map_preselection(&self.pilot)
    }

    /// The stellar selected in mode 2, if any: the one every reader of the
    /// stellar selection reads.
    #[must_use]
    pub fn nav_target(&self) -> Option<StellarId> {
        self.chart.nav_target()
    }

    /// Saves the nav mode and its selection as a port dialog opens, as the
    /// original saves +0x6a and +0x6c (see the module's **Setters**). The
    /// raw selection is saved, not the one resolved by place.
    pub fn save_port_nav(&mut self) {
        self.chart.save_port_nav();
    }

    /// Restores the nav saved as the port dialog opened
    /// ([`Session::save_port_nav`]), mode and selection exactly as saved,
    /// as the dialog closes. With nothing saved it changes nothing, and a
    /// save is restored once.
    pub fn restore_port_nav(&mut self) {
        self.chart.restore_port_nav();
    }

    /// The nav as it stands, for a mission dialog to put back after the
    /// map it opens ([`Session::restore_map_nav`]): the mode and the raw
    /// selection with the Con slot noted for it (see [`SavedNav`]). It
    /// changes nothing.
    #[must_use]
    pub fn map_nav(&self) -> SavedNav {
        self.chart.map_nav()
    }

    /// Puts back the nav a mission dialog saved before the map it opened
    /// ([`Session::map_nav`]), mode, selection and noted slot exactly as
    /// saved, as the offer, the mission computer and the text dialog
    /// write +0x6a and +0x6c back after `_DoSystemMap` returns (see the
    /// module's **Setters**). The course is never saved, so a Shift-click's
    /// course stays.
    pub fn restore_map_nav(&mut self, saved: SavedNav) {
        self.chart.restore_map_nav(saved);
    }

    /// Puts back the selection saved as a map opened ([`Session::map_nav`]),
    /// keeping the mode, as `_DoSystemMap`'s close in port writes back the
    /// +0x6c it saved as it opened, and +0x6a not (@0x12349,
    /// @0x13bee-0x13c06): while the mode is the one saved, the nav as saved,
    /// its noted slot with it. Docked, the map writes the mode only by
    /// [`RuleKey::MapPlotting`]'s other reading, which has no original;
    /// there the selection saved belongs to another mode and nothing
    /// changes.
    pub fn restore_map_selection(&mut self, saved: SavedNav) {
        self.chart.restore_map_selection(saved);
    }

    /// Puts back the nav saved as the bar opened ([`Session::open_bar`]),
    /// mode and selection exactly as saved, as a map opened from the bar
    /// closes (see the module's **Setters**). The save is kept, as the
    /// bar's locals outlive each map, so every map is put back to the
    /// same nav. With nothing saved it changes nothing.
    pub fn restore_bar_nav(&mut self) {
        self.chart.restore_bar_nav();
    }

    /// Cues the course's next hop, as `_CueNextHyperRouteDest` (@0xe2dc)
    /// does: when the course's first hop as it is flown
    /// ([`Session::course_shown`]) is a system the ship can jump to from
    /// here, mode 3 with it selected; otherwise nothing changes. It makes
    /// no sound. The original writes the first Con slot whose system
    /// stands at the hop's place (@0xe30f-0xe344), which a later map
    /// cycle steps on from ([`Session::cycle_map_selection`]).
    pub fn cue_course(&mut self) {
        self.chart.cue_course(&self.pilot, &self.setup);
    }

    /// Runs `_HandlePlayer`'s Nav Off / Hyperspace Mode block for one
    /// press (@0x69ade-0x69b89), with `nav_off` the backquote (without
    /// Option) and `hyperspace` H: one [`SimSound::NavSelected`]
    /// (@0x69b19) and the dock request dropped (@0x69b1e-0x69b3c) for
    /// either key; then, with `nav_off`, the nav system off, the course
    /// kept (@0x69b41-0x69b5a); then, with `hyperspace`, unless the mode
    /// is 3 already, mode 3 with nothing selected and the course's next
    /// hop cued ([`Session::cue_course`], @0x69b63-0x69b89). Both in one
    /// press give one beep, the nav off, then mode 3 and the cue. During
    /// a jump (pre-jump stage too) nothing changes and it says so. The
    /// flight view keeps the block's latch (0xdee04), which says when a
    /// held key presses again.
    pub fn nav_mode_keys(&mut self, nav_off: bool, hyperspace: bool) -> bool {
        if self.mode.jump().is_some() {
            return false;
        }
        self.out.sounds.push(SimSound::NavSelected);
        self.port.dock = None;
        if nav_off {
            self.chart.set_nav(NavMode::Off);
        }
        if hyperspace && !matches!(self.chart.nav, NavMode::Hyperspace(_)) {
            self.chart.set_nav(NavMode::Hyperspace(None));
            self.cue_course();
        }
        true
    }

    /// Presses Nav Off (backquote) alone ([`Session::nav_mode_keys`]):
    /// the nav system off, the course kept, the dock request dropped,
    /// with [`SimSound::NavSelected`] on every press. During a jump
    /// (pre-jump stage too) nothing changes and it says so.
    pub fn nav_off(&mut self) -> bool {
        self.nav_mode_keys(true, false)
    }

    /// Presses Hyperspace Mode (H) alone ([`Session::nav_mode_keys`]):
    /// [`SimSound::NavSelected`] and the dock request dropped, then,
    /// unless the mode is 3 already, mode 3 with nothing selected and the
    /// course's next hop cued ([`Session::cue_course`]). During a jump
    /// (pre-jump stage too) nothing changes and it says so.
    pub fn hyperspace_mode(&mut self) -> bool {
        self.nav_mode_keys(false, true)
    }

    /// Presses Option-backquote: lets go of the ship target, with
    /// [`SimSound::TargetCleared`], and says whether there was one. With
    /// none it is silent. The nav mode is untouched, and the original
    /// takes it during a jump too.
    pub fn clear_target(&mut self) -> bool {
        self.chart.clear_target(&mut self.out)
    }

    /// Makes the stellar `pick` finds the stellar selected, as the
    /// original's stellar slot keys and F5 do, and gives it. The course
    /// is left alone. `None`, and nothing changes, during a jump
    /// (pre-jump stage too, as the original skips its nav inputs while
    /// jumping) or when `pick` finds nothing: an empty slot or no
    /// stellar with the can-land flag.
    ///
    /// A key that finds a stellar raises [`SimSound::NavSelected`], even
    /// for the one selected (@0x69ebc). Selecting changes the mode to 2
    /// and starts the brackets' fly-in ([`Session::nav_brackets`]), and
    /// drops the dock request (@0x69efd-0x69f12), only when it is not the
    /// stellar selected in mode 2 already (@0x69ec4-0x69ed6). A click in
    /// space is [`Session::click_in_space`], whose walk passes over the
    /// stellar selected in mode 2 and goes on to any under it
    /// (@0x6a69d → @0x6a703).
    pub fn select_stellar(&mut self, pick: StellarPick) -> Option<StellarId> {
        if self.mode.jump().is_some() {
            return None;
        }
        let picked = match pick {
            StellarPick::Slot(slot) => self.chart.sites.get(slot).map(|site| site.id),
            StellarPick::Nearest => nearest_stellar(
                self.ship.player.position,
                &self.chart.sites,
                self.setup.rules.source_for(RuleKey::GatePickRange),
            ),
        }?;
        let selected = self.chart.nav == NavMode::Stellar(Some(picked));
        self.out.sounds.push(SimSound::NavSelected);
        if !selected {
            self.chart.nav_fly_in.start();
            self.chart.set_nav(NavMode::Stellar(Some(picked)));
            self.port.dock = None;
        }
        Some(picked)
    }

    /// Presses Hyper Select (the original's `\` key): selects in mode 3
    /// the next system the session's [`HyperSelectRule`](crate::HyperSelectRule)
    /// cycles to ([`next_hyper_destination`]), from the one selected in
    /// mode 3 and from the start in any other, with
    /// [`SimSound::HyperSelected`], and gives it. With no system to offer
    /// it lets go of the selection, keeps the mode and makes no sound
    /// (@0x69cd4), and gives `None`. `None`, and nothing changes, during a
    /// jump (pre-jump stage too).
    ///
    /// By the engine's reading the course is kept: the press moves only
    /// the selection, as `_HandlePlayer` @0x69c8e-0x69cd2 writes nothing
    /// but +0x6a and +0x6c, and the galaxy map draws the selection as its
    /// own link. J then jumps to the system selected, and an arrival off
    /// the course clears it, or drops its first hop when it is there
    /// (`course::after_arrival`). By
    /// [`HyperSelectRule::OneJumpCourse`](crate::HyperSelectRule::OneJumpCourse)
    /// the press plots a one-jump course to the system selected, the
    /// course that reading's cycle starts from.
    pub fn select_next_system(&mut self) -> Option<SystemId> {
        self.chart.select_next_system(
            &mut self.pilot,
            &self.setup,
            &mut self.out,
            self.mode.jump(),
        )
    }

    /// The links the floating map's clicks test, as `_HandlePlayer` walks
    /// them (@0x6a1e9-0x6a332): the current system's raw Con slots, in
    /// record order, repeats and itself kept, each only when it is in the
    /// catalog and a visible system stands at its place
    /// (`_FindActiveCoLocatedSystem` @0x6a21f). Each is the raw slot, not
    /// the system standing for it: its box is at the slot's own position
    /// (@0x6a25c-0x6a2af), and mode 3 selects it raw (@0x6a719-0x6a727).
    #[must_use]
    pub fn floating_map_links(&self) -> Vec<SystemId> {
        self.chart.floating_map_links(&self.pilot)
    }

    /// The galaxy map's Tab or `\`, as `_MapFilter` cycles the hyperspace
    /// selection off the hypergate map (@0x13f0f-0x13fac): mode 3 with the
    /// next of the current system's Con slots that holds a link
    /// ([`Session::floating_map_links`]' test), stepping on from the slot
    /// +0x6c holds, past the 16th back to the first, the selection the
    /// slot's raw system, as the floating map's click writes it; with no
    /// such slot, mode 3 with nothing selected. Where it starts:
    ///
    /// - In mode 3, after the slot the selection stands in: the slot the
    ///   last cycle or the cue wrote, while it stands, so a repeated link
    ///   is visited once for each slot; else the first slot naming it, or
    ///   standing at its place; else the first slot.
    /// - In mode 2 with a stellar, as [`RuleKey::MapCycleStart`] says: by
    ///   the engine, after the slot the stellar's index (its ID less 128)
    ///   names when that is 0-15, and at the first slot otherwise; by the
    ///   other reading at the first slot.
    /// - Otherwise at the first slot, as +0x6c is −1 in Off and with no
    ///   selection.
    ///
    /// It makes no sound and writes nothing else: the course and the dock
    /// request are kept. Docked it does nothing and says so, as on a
    /// planet the key falls through (`_onPlanet` @0x13d20-0x13d28). It
    /// doesn't read the jump: ours' map can't open during one. Then the
    /// original redraws the nav area (@0x13fb2-0x1406e), which the flight
    /// view does, and selects [`Session::map_preselection`] on the map
    /// (@0x1408c-0x140d8).
    pub fn cycle_map_selection(&mut self) -> bool {
        self.chart
            .cycle_map_selection(&self.pilot, &self.setup, self.mode.landed())
    }

    /// A click hit on the floating map, as `_HandlePlayer` takes it: mode
    /// 3 with `link` selected, one of [`Session::floating_map_links`]
    /// (@0x6a719-0x6a727), or with nothing for the current system's centre
    /// (@0x6a1d5-0x6a1de), with [`SimSound::HyperSelected`] (`_beepSnd[2]`,
    /// @0x6a73a-0x6a75d). The course is kept. During a jump (pre-jump
    /// stage too) nothing changes and it says so, as the click is gated
    /// on the jump state (@0x6a145).
    pub fn select_on_floating_map(&mut self, link: Option<SystemId>) -> bool {
        self.chart
            .select_on_floating_map(&mut self.out, self.mode.jump(), link)
    }
}

impl Outbox {
    /// The nav mode and selection as the last take-off began, before
    /// the stay's days and its deselect, or `None` once taken: what the
    /// original's take-off redraw (`_PlayerLandOnStellar` @0x633a0) draws,
    /// as its deselect (@0x674d2) comes later in the same dock request
    /// (see the flight view's `took_off`).
    fn take_takeoff_nav(&mut self) -> Option<NavMode> {
        self.takeoff_nav.take()
    }
}

impl Chart {
    /// The player's nav mode and its selection, a hyperspace selection
    /// resolved by place: the visible system at its place, or none when
    /// there is none or it isn't in the catalog, as
    /// `_FindActiveCoLocatedSystem` gives −1 for a system with no `sÿst`
    /// (see the module docs).
    #[must_use]
    pub(super) fn nav_mode(&self) -> NavMode {
        match self.nav {
            NavMode::Hyperspace(selected) => NavMode::Hyperspace(
                selected
                    .filter(|&system| self.galaxy.exists(system))
                    .and_then(|system| self.galaxy.same_place(system)),
            ),
            mode => mode,
        }
    }

    /// The system the galaxy map selects as it opens, as `_DoSystemMap`
    /// does (@0x124ac-0x124ff): with a system selected in mode 3, that
    /// system resolved by place, or `None` when no visible system stands
    /// there (`_FindActiveCoLocatedSystem` gives −1, @0x124e6); otherwise
    /// the current system (@0x124f6-0x124ff). The original's on-planet
    /// branch (@0x124bd) gives the current system too, and ours' map
    /// opens only in flight. After the map's Tab / `\` cycle
    /// ([`Session::cycle_map_selection`]) it is the map's new selection,
    /// as `_MapFilter` sets `_selSystem` by the same rule
    /// (@0x1408c-0x140d8).
    #[must_use]
    fn map_preselection(&self, pilot: &Pilot) -> Option<SystemId> {
        match self.nav {
            NavMode::Hyperspace(Some(_)) => self.nav_mode().hyperspace(),
            _ => Some(pilot.system),
        }
    }

    /// The stellar selected in mode 2, if any: the one every reader of the
    /// stellar selection reads.
    #[must_use]
    pub(super) fn nav_target(&self) -> Option<StellarId> {
        self.nav.stellar()
    }

    /// The system J jumps to: the one selected in mode 3, resolved by
    /// place, when the ship can jump there from here (@0x6b11e-0x6b131).
    pub(super) fn jump_selection(&self, pilot: &Pilot, setup: &Setup) -> Option<SystemId> {
        let NavMode::Hyperspace(Some(selected)) = self.nav_mode() else {
            return None;
        };
        let jumps = self.galaxy.map().jumps(pilot.system, setup.hyperlinks);
        jumps.contains(&selected).then_some(selected)
    }

    /// Writes the nav mode and its selection, as each of the original's
    /// setters writes +0x6a and +0x6c, and forgets the Con slot noted for
    /// the old selection ([`NavSlot`]). A setter that writes mode 3 from a
    /// slot it knows ([`Session::cycle_map_selection`],
    /// [`Session::cue_course`]) notes that slot after. Every other one's
    /// selection stands in the first slot naming it
    /// ([`Chart::selected_slot`]). The floating map's click writes that
    /// slot, the first whose box it hits, as a repeated link's slots
    /// share a box (@0x6a332 → @0x6a719). Hyper Select writes the slot it
    /// steps to (@0x69cce), and ours, whose cycle visits each link once
    /// ([`HyperSelectRule`]), stands it in that link's first slot, the
    /// one the original steps to from any slot before it. The port
    /// dialogs' and the bar's restores write mode 3 only after
    /// [`Session::land_at`], which has no original.
    pub(super) fn set_nav(&mut self, nav: NavMode) {
        self.nav = nav;
        self.nav_slot = None;
    }

    /// Lets go of the selection and keeps the mode.
    pub(super) fn deselect(&mut self) {
        self.set_nav(self.nav.deselected());
    }

    /// A landed `M` (`first`) or `N` into `system` writes the selection
    /// at its operator, as `_EvalSetExp` does (`M` @0x1570e-0x157f5, `N`
    /// @0x158d9-0x158fd), before its move is queued: `M` in mode 2 selects
    /// the system's first stellar that can be read, or none with none;
    /// any other `M`, and every `N`, lets go of the selection and keeps
    /// the mode, so Off stays off. Nothing is written for a system that
    /// doesn't exist, as the move skips it too, nor in flight, where the
    /// move's settling lets go of the selection (the `entry` module's
    /// table). The original lets go of it in flight at the operator too
    /// (@0x157e3, @0x158fd); ours' settling comes later in the same input
    /// or tick, before any frame is drawn, so the player never sees it.
    /// Why the first stellar that can be read and a system that doesn't
    /// exist are the original's, as far as it can be modelled, is in the
    /// module docs.
    pub(super) fn select_for_landed_move(
        &mut self,
        statics: &Statics,
        landed: Option<StellarId>,
        system: SystemId,
        first: bool,
    ) {
        if landed.is_none() || !self.galaxy.exists(system) {
            return;
        }
        self.set_nav(match self.nav {
            NavMode::Stellar(_) if first => {
                NavMode::Stellar(statics.first_stellars.get(&system).copied())
            }
            mode => mode.deselected(),
        });
    }

    /// Saves the nav mode and its selection as a port dialog opens, as the
    /// original saves +0x6a and +0x6c (see the module's **Setters**). The
    /// raw selection is saved, not the one resolved by place.
    fn save_port_nav(&mut self) {
        self.port_nav = Some(self.nav);
    }

    /// Restores the nav saved as the port dialog opened
    /// ([`Session::save_port_nav`]), mode and selection exactly as saved,
    /// as the dialog closes. With nothing saved it changes nothing, and a
    /// save is restored once.
    fn restore_port_nav(&mut self) {
        if let Some(saved) = self.port_nav.take() {
            self.set_nav(saved);
        }
    }

    /// The nav as it stands, for a mission dialog to put back after the
    /// map it opens ([`Session::restore_map_nav`]): the mode and the raw
    /// selection with the Con slot noted for it (see [`SavedNav`]). It
    /// changes nothing.
    #[must_use]
    fn map_nav(&self) -> SavedNav {
        SavedNav {
            nav: self.nav,
            slot: self.nav_slot,
        }
    }

    /// Puts back the nav a mission dialog saved before the map it opened
    /// ([`Session::map_nav`]), mode, selection and noted slot exactly as
    /// saved, as the offer, the mission computer and the text dialog
    /// write +0x6a and +0x6c back after `_DoSystemMap` returns (see the
    /// module's **Setters**). The course is never saved, so a Shift-click's
    /// course stays.
    fn restore_map_nav(&mut self, saved: SavedNav) {
        self.nav = saved.nav;
        self.nav_slot = saved.slot;
    }

    /// Puts back the selection saved as a map opened ([`Session::map_nav`]),
    /// keeping the mode, as `_DoSystemMap`'s close in port writes back the
    /// +0x6c it saved as it opened, and +0x6a not (@0x12349,
    /// @0x13bee-0x13c06): while the mode is the one saved, the nav as saved,
    /// its noted slot with it. Docked, the map writes the mode only by
    /// [`RuleKey::MapPlotting`]'s other reading, which has no original;
    /// there the selection saved belongs to another mode and nothing
    /// changes.
    fn restore_map_selection(&mut self, saved: SavedNav) {
        if std::mem::discriminant(&self.nav) == std::mem::discriminant(&saved.nav) {
            self.restore_map_nav(saved);
        }
    }

    /// Puts back the nav saved as the bar opened ([`Session::open_bar`]),
    /// mode and selection exactly as saved, as a map opened from the bar
    /// closes (see the module's **Setters**). The save is kept, as the
    /// bar's locals outlive each map, so every map is put back to the
    /// same nav. With nothing saved it changes nothing.
    fn restore_bar_nav(&mut self) {
        if let Some(saved) = self.bar_nav {
            self.set_nav(saved);
        }
    }

    /// Cues the course's next hop, as `_CueNextHyperRouteDest` (@0xe2dc)
    /// does: when the course's first hop as it is flown
    /// ([`Session::course_shown`]) is a system the ship can jump to from
    /// here, mode 3 with it selected; otherwise nothing changes. It makes
    /// no sound. The original writes the first Con slot whose system
    /// stands at the hop's place (@0xe30f-0xe344), which a later map
    /// cycle steps on from ([`Session::cycle_map_selection`]).
    pub(super) fn cue_course(&mut self, pilot: &Pilot, setup: &Setup) {
        let Some(&hop) = self.course_shown(pilot).first() else {
            if !pilot.course.is_empty() {
                self.cue_unplaced_hop(pilot, setup);
            }
            return;
        };
        if self
            .galaxy
            .map()
            .jumps(pilot.system, setup.hyperlinks)
            .contains(&hop)
        {
            self.select_at_place(pilot, hop);
        }
    }

    /// The cue of a first hop with no visible system at its place, as
    /// [`RuleKey::CueUnplacedHop`] says. By the engine,
    /// `_CueNextHyperRouteDest` resolves the hop to −1 (@0xe2fa) and
    /// matches it against each Con slot's own resolution (@0xe334): the
    /// first of the current system's [`CON_SLOTS`] that is set and names a
    /// system with no visible system at its place, or an ID in 128-2175
    /// with no `sÿst`, gives mode 3 with that slot, raw, and the slot noted
    /// (@0xe33c-0xe34d). An ID outside 128-2175 is an empty slot, as the
    /// loader sets it to −1 (`_LoadObjectData` @0x7806d-0x780b8). The
    /// selection reads as none ([`Session::nav_mode`]), J refuses it, and
    /// the map draws no mode-3 link for it, as `_DrawMap` skips a slot
    /// resolving to −1 (@0xec25-0xec2b). By the other reading nothing
    /// changes.
    fn cue_unplaced_hop(&mut self, pilot: &Pilot, setup: &Setup) {
        if setup.rules.source_for(RuleKey::CueUnplacedHop) != RuleSource::Engine {
            return;
        }
        let slots = self.galaxy.con_slots(pilot.system);
        let dead = slots.iter().take(CON_SLOTS).position(|&slot| {
            SYSTEM_IDS.contains(&slot.0)
                && (!self.galaxy.exists(slot) || self.galaxy.same_place(slot).is_none())
        });
        if let Some(slot) = dead {
            let selected = slots[slot];
            self.set_nav(NavMode::Hyperspace(Some(selected)));
            self.nav_slot = Some(NavSlot {
                system: pilot.system,
                slot,
                selected,
            });
        }
    }

    /// Mode 3 with `system` selected, the Con slot noted
    /// ([`NavSlot`]) the first of the current system's whose system stands
    /// at its place, as `_CueNextHyperRouteDest` (@0xe30f-0xe344) and the
    /// map's click (`_DoSystemMap` @0x13739-0x137bb) write +0x6c.
    pub(super) fn select_at_place(&mut self, pilot: &Pilot, system: SystemId) {
        self.set_nav(NavMode::Hyperspace(Some(system)));
        self.nav_slot = self.galaxy.same_place(system).and_then(|place| {
            let slot = self.slot_at(pilot, |slot| self.galaxy.same_place(slot) == Some(place))?;
            Some(NavSlot {
                system: pilot.system,
                slot,
                selected: system,
            })
        });
    }

    /// Presses Option-backquote: lets go of the ship target, with
    /// [`SimSound::TargetCleared`], and says whether there was one. With
    /// none it is silent. The nav mode is untouched, and the original
    /// takes it during a jump too.
    fn clear_target(&mut self, out: &mut Outbox) -> bool {
        let had = self.target.take().is_some();
        if had {
            out.sounds.push(SimSound::TargetCleared);
        }
        had
    }

    /// Presses Hyper Select (the original's `\` key): selects in mode 3
    /// the next system the session's [`HyperSelectRule`](crate::HyperSelectRule)
    /// cycles to ([`next_hyper_destination`]), from the one selected in
    /// mode 3 and from the start in any other, with
    /// [`SimSound::HyperSelected`], and gives it. With no system to offer
    /// it lets go of the selection, keeps the mode and makes no sound
    /// (@0x69cd4), and gives `None`. `None`, and nothing changes, during a
    /// jump (pre-jump stage too).
    ///
    /// By the engine's reading the course is kept: the press moves only
    /// the selection, as `_HandlePlayer` @0x69c8e-0x69cd2 writes nothing
    /// but +0x6a and +0x6c, and the galaxy map draws the selection as its
    /// own link. J then jumps to the system selected, and an arrival off
    /// the course clears it, or drops its first hop when it is there
    /// (`course::after_arrival`). By
    /// [`HyperSelectRule::OneJumpCourse`](crate::HyperSelectRule::OneJumpCourse)
    /// the press plots a one-jump course to the system selected, the
    /// course that reading's cycle starts from.
    fn select_next_system(
        &mut self,
        pilot: &mut Pilot,
        setup: &Setup,
        out: &mut Outbox,
        jump: Option<Jump>,
    ) -> Option<SystemId> {
        if jump.is_some() {
            return None;
        }
        let selected = self.nav_mode().hyperspace();
        let Some(next) = next_hyper_destination(
            self.galaxy.map(),
            pilot.system,
            &self.course_shown(pilot),
            selected,
            setup.hyper_select,
            setup.hyperlinks,
        ) else {
            self.deselect();
            return None;
        };
        out.sounds.push(SimSound::HyperSelected);
        if setup.hyper_select == HyperSelectRule::OneJumpCourse {
            pilot.course = vec![next];
        }
        self.set_nav(NavMode::Hyperspace(Some(next)));
        Some(next)
    }

    /// The links the floating map's clicks test, as `_HandlePlayer` walks
    /// them (@0x6a1e9-0x6a332): the current system's raw Con slots, in
    /// record order, repeats and itself kept, each only when it is in the
    /// catalog and a visible system stands at its place
    /// (`_FindActiveCoLocatedSystem` @0x6a21f). Each is the raw slot, not
    /// the system standing for it: its box is at the slot's own position
    /// (@0x6a25c-0x6a2af), and mode 3 selects it raw (@0x6a719-0x6a727).
    #[must_use]
    fn floating_map_links(&self, pilot: &Pilot) -> Vec<SystemId> {
        self.galaxy
            .con_slots(pilot.system)
            .iter()
            .copied()
            .filter(|&slot| self.flags(slot))
            .collect()
    }

    /// Whether a Con slot naming `slot` is one the link cycles and the
    /// floating map take: in the catalog, with a visible system at its
    /// place (`_FindActiveCoLocatedSystem` ≠ −1, `_MapFilter`
    /// @0x13f4f-0x13f65, `_HandlePlayer` @0x6a21f).
    fn flags(&self, slot: SystemId) -> bool {
        self.galaxy.exists(slot) && self.galaxy.same_place(slot).is_some()
    }

    /// The first of the current system's [`CON_SLOTS`] Con slots whose
    /// system `test` passes, empty slots (−1) passed over.
    fn slot_at(&self, pilot: &Pilot, test: impl Fn(SystemId) -> bool) -> Option<usize> {
        self.galaxy
            .con_slots(pilot.system)
            .iter()
            .take(CON_SLOTS)
            .position(|&slot| self.galaxy.exists(slot) && test(slot))
    }

    /// The Con slot the raw mode-3 selection `selected` stands in: the one
    /// noted as it was written, while it stands ([`NavSlot`]); else the
    /// first naming it; else the first whose system stands at its place.
    pub(super) fn selected_slot(&self, pilot: &Pilot, selected: SystemId) -> Option<usize> {
        if let Some(noted) = self.nav_slot
            && noted.system == pilot.system
            && noted.selected == selected
        {
            return Some(noted.slot);
        }
        self.slot_at(pilot, |slot| slot == selected).or_else(|| {
            let place = self.galaxy.same_place(selected)?;
            self.slot_at(pilot, |slot| self.galaxy.same_place(slot) == Some(place))
        })
    }

    /// What +0x6c holds as the map's cycle starts, the slot it steps on
    /// from (see [`Session::cycle_map_selection`]): in mode 3 the slot the
    /// selection stands in; in mode 2 with a stellar, by the engine's
    /// [`RuleKey::MapCycleStart`], the stellar's index (its ID less 128);
    /// otherwise −1, as every write of Off writes +0x6c = −1 with it
    /// (`_ResetPlayer` @0x1d4cd-0x1d4d5, Nav Off @0x69b54-0x69b5a, an
    /// arrival @0x6be25-0x6be2b, the hypergate @0x637e4-0x637f2), and so
    /// does every deselect.
    fn cycle_from(&self, pilot: &Pilot, setup: &Setup) -> i32 {
        match self.nav {
            NavMode::Hyperspace(Some(selected)) => self
                .selected_slot(pilot, selected)
                .and_then(|slot| i32::try_from(slot).ok())
                .unwrap_or(-1),
            NavMode::Stellar(Some(stellar))
                if setup.rules.source_for(RuleKey::MapCycleStart) == RuleSource::Engine =>
            {
                i32::from(stellar.0) - 128
            }
            _ => -1,
        }
    }

    /// The galaxy map's Tab or `\`, as `_MapFilter` cycles the hyperspace
    /// selection off the hypergate map (@0x13f0f-0x13fac): mode 3 with the
    /// next of the current system's Con slots that holds a link
    /// ([`Session::floating_map_links`]' test), stepping on from the slot
    /// +0x6c holds, past the 16th back to the first, the selection the
    /// slot's raw system, as the floating map's click writes it; with no
    /// such slot, mode 3 with nothing selected. Where it starts:
    ///
    /// - In mode 3, after the slot the selection stands in: the slot the
    ///   last cycle or the cue wrote, while it stands, so a repeated link
    ///   is visited once for each slot; else the first slot naming it, or
    ///   standing at its place; else the first slot.
    /// - In mode 2 with a stellar, as [`RuleKey::MapCycleStart`] says: by
    ///   the engine, after the slot the stellar's index (its ID less 128)
    ///   names when that is 0-15, and at the first slot otherwise; by the
    ///   other reading at the first slot.
    /// - Otherwise at the first slot, as +0x6c is −1 in Off and with no
    ///   selection.
    ///
    /// It makes no sound and writes nothing else: the course and the dock
    /// request are kept. Docked it does nothing and says so, as on a
    /// planet the key falls through (`_onPlanet` @0x13d20-0x13d28). It
    /// doesn't read the jump: ours' map can't open during one. Then the
    /// original redraws the nav area (@0x13fb2-0x1406e), which the flight
    /// view does, and selects [`Session::map_preselection`] on the map
    /// (@0x1408c-0x140d8).
    fn cycle_map_selection(
        &mut self,
        pilot: &Pilot,
        setup: &Setup,
        landed: Option<StellarId>,
    ) -> bool {
        if landed.is_some() {
            return false;
        }
        // The step goes on from +0x6c, past the last slot to the first and
        // below the first to the last (@0x13f77-0x13f9b): a start past the
        // slots begins the walk at the first.
        let start = usize::try_from(self.cycle_from(pilot, setup) + 1).unwrap_or(CON_SLOTS - 1);
        let slots = self.galaxy.con_slots(pilot.system);
        let slots = &slots[..slots.len().min(CON_SLOTS)];
        let next = (start..slots.len()).chain(0..start).find_map(|at| {
            slots
                .get(at)
                .copied()
                .filter(|&slot| self.flags(slot))
                .map(|slot| (at, slot))
        });
        self.set_nav(NavMode::Hyperspace(next.map(|(_, selected)| selected)));
        self.nav_slot = next.map(|(slot, selected)| NavSlot {
            system: pilot.system,
            slot,
            selected,
        });
        true
    }

    /// A click hit on the floating map, as `_HandlePlayer` takes it: mode
    /// 3 with `link` selected, one of [`Session::floating_map_links`]
    /// (@0x6a719-0x6a727), or with nothing for the current system's centre
    /// (@0x6a1d5-0x6a1de), with [`SimSound::HyperSelected`] (`_beepSnd[2]`,
    /// @0x6a73a-0x6a75d). The course is kept. During a jump (pre-jump
    /// stage too) nothing changes and it says so, as the click is gated
    /// on the jump state (@0x6a145).
    fn select_on_floating_map(
        &mut self,
        out: &mut Outbox,
        jump: Option<Jump>,
        link: Option<SystemId>,
    ) -> bool {
        if jump.is_some() {
            return false;
        }
        self.set_nav(NavMode::Hyperspace(link));
        out.sounds.push(SimSound::HyperSelected);
        true
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::chance::NeverFires;
    use crate::control::SetExpr;
    use crate::flight::Controls;
    use crate::gate::GateRefusal;
    use crate::hyperspace::{HyperSelectRule, JumpReadiness, JumpRefusal, RouteError};
    use crate::landing::LandOutcome;
    use crate::pilot::Pilot;
    use crate::session::LandPress;
    use crate::session::tests::{
        alternates, armed, at_gate, at_wormhole, facing_an_npc, gated, hide, hub, three_stellars,
        with_alternate, wormholes,
    };
    use crate::sound::SimSound;
    use crate::targeting::TargetPick;
    use crate::testkit::{
        FakePilotCatalog, begin_jump_now, catalog, click_stellar, fly_out, hideable, land_now,
        star, starting,
    };
    use crate::{RuleKey, RuleSource, Rulebook};

    fn ids(route: &[i16]) -> Vec<SystemId> {
        route.iter().copied().map(SystemId).collect()
    }

    fn hyperspace(system: i16) -> NavMode {
        NavMode::Hyperspace(Some(SystemId(system)))
    }

    fn stellar(id: i16) -> NavMode {
        NavMode::Stellar(Some(StellarId(id)))
    }

    /// The session with its sounds taken.
    fn quiet(mut session: Session) -> Session {
        session.take_sounds();
        session
    }

    // Start and load.

    #[test]
    fn a_new_pilot_flies_with_the_nav_system_off() {
        let session = Session::start(&three_stellars()).expect("starts");
        assert_eq!(session.nav_mode(), NavMode::Off);
    }

    #[test]
    fn a_pilot_loaded_with_a_course_flies_with_the_nav_system_off_and_cannot_jump() {
        let catalog = catalog();
        let mut pilot = Pilot::new(&catalog, "").expect("a pilot");
        pilot.course = ids(&[131, 132]);
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(session.nav_mode(), NavMode::Off);
        assert_eq!(session.course(), ids(&[131, 132]), "the course kept");
        fly_out(&mut session);
        assert_eq!(session.begin_jump(), Err(JumpRefusal::NoDestination));
    }

    // Nav Off.

    #[test]
    fn nav_off_with_a_course_turns_the_nav_off_keeps_the_course_and_refuses_the_jump() {
        let mut session = Session::start(&hub()).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        assert_eq!(session.nav_mode(), hyperspace(131));
        let mut session = quiet(session);
        assert!(session.nav_off());
        assert_eq!(session.nav_mode(), NavMode::Off);
        assert_eq!(session.course(), ids(&[131, 132]), "the course kept");
        assert_eq!(session.take_sounds(), [SimSound::NavSelected]);
        fly_out(&mut session);
        assert_eq!(session.jump_readiness(), JumpReadiness::Blocked);
        assert_eq!(session.begin_jump(), Err(JumpRefusal::NoDestination));
    }

    #[test]
    fn nav_off_beeps_on_every_press_and_makes_no_save_due() {
        let mut session = quiet(Session::start(&three_stellars()).expect("starts"));
        for _ in 0..2 {
            assert!(session.nav_off());
            assert_eq!(session.take_sounds(), [SimSound::NavSelected]);
        }
        assert!(!session.take_save_due());
    }

    #[test]
    fn nav_off_and_hyperspace_mode_are_refused_during_a_jump() {
        let mut session = Session::start(&hub()).expect("starts");
        session.select_next_system();
        fly_out(&mut session);
        session.ship.player.heading = 90.0;
        assert_eq!(session.begin_jump(), Ok(SystemId(134)));
        let mut session = quiet(session);
        for stage in ["pre-jump", "in hyperspace"] {
            assert!(!session.nav_off(), "{stage}");
            assert!(!session.hyperspace_mode(), "{stage}");
            assert_eq!(session.nav_mode(), hyperspace(134), "{stage}");
            assert_eq!(session.take_sounds(), [], "{stage}");
            if session.jumping().is_none() {
                begin_jump_now(&mut session).expect("jumps");
                session.take_sounds();
            }
        }
    }

    // Hyperspace Mode.

    #[test]
    fn hyperspace_mode_with_no_course_selects_no_destination_and_the_jump_is_refused() {
        let mut session = quiet(Session::start(&three_stellars()).expect("starts"));
        session.select_stellar(StellarPick::Slot(0));
        session.take_sounds();
        assert!(session.hyperspace_mode());
        assert_eq!(session.nav_mode(), NavMode::Hyperspace(None));
        assert_eq!(session.take_sounds(), [SimSound::NavSelected]);
        fly_out(&mut session);
        assert_eq!(session.begin_jump(), Err(JumpRefusal::NoDestination));
    }

    #[test]
    fn hyperspace_mode_cues_an_adjacent_first_hop_and_the_jump_goes_there() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        session.nav_off();
        assert!(session.hyperspace_mode());
        assert_eq!(session.nav_mode(), hyperspace(131));
        fly_out(&mut session);
        assert_eq!(begin_jump_now(&mut session), Ok(SystemId(131)));
    }

    #[test]
    fn hyperspace_mode_in_mode_3_changes_nothing_but_beeps() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        session.chart.nav = NavMode::Hyperspace(None);
        let mut session = quiet(session);
        assert!(session.hyperspace_mode());
        assert_eq!(session.nav_mode(), NavMode::Hyperspace(None), "not cued");
        assert_eq!(session.take_sounds(), [SimSound::NavSelected]);
    }

    // The Nav Off / Hyperspace Mode block.

    /// A session in mode 3 at the course's first hop, with a dock request
    /// standing, and its sounds taken.
    fn cued_with_a_request() -> Session {
        let mut session = Session::start(&catalog()).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        assert_eq!(session.nav_mode(), hyperspace(131));
        session.port.dock = Some(crate::clearance::DockRequest {
            stellar: StellarId(128),
            permission: 0,
        });
        quiet(session)
    }

    #[test]
    fn the_block_with_both_keys_beeps_once_turns_off_then_cues_and_drops_the_request() {
        let mut session = cued_with_a_request();
        session.chart.nav = NavMode::Hyperspace(None);
        assert!(session.nav_mode_keys(true, true));
        assert_eq!(session.take_sounds(), [SimSound::NavSelected]);
        assert_eq!(session.nav_mode(), hyperspace(131), "off, then cued");
        assert_eq!(session.dock_request(), None);
    }

    #[test]
    fn the_block_with_h_alone_in_mode_3_keeps_the_selection_beeps_and_drops_the_request() {
        let mut session = cued_with_a_request();
        session.chart.nav = NavMode::Hyperspace(Some(SystemId(130)));
        assert!(session.nav_mode_keys(false, true));
        assert_eq!(session.take_sounds(), [SimSound::NavSelected]);
        assert_eq!(session.nav_mode(), hyperspace(130), "kept");
        assert_eq!(session.dock_request(), None);
    }

    #[test]
    fn the_block_with_nav_off_alone_turns_off_and_does_not_cue() {
        let mut session = cued_with_a_request();
        assert!(session.nav_mode_keys(true, false));
        assert_eq!(session.take_sounds(), [SimSound::NavSelected]);
        assert_eq!(session.nav_mode(), NavMode::Off);
        assert_eq!(session.dock_request(), None);
    }

    #[test]
    fn the_block_is_refused_in_silence_during_a_jump() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        begin_jump_now(&mut session).expect("jumps");
        let mut session = quiet(session);
        assert!(!session.nav_mode_keys(true, true));
        assert_eq!(session.take_sounds(), []);
        assert_eq!(session.nav_mode(), hyperspace(131));
    }

    // Hyper Select.

    #[test]
    fn hyper_select_from_off_starts_at_the_first_link_and_cycles_in_mode_3() {
        let mut session = quiet(Session::start(&hub()).expect("starts"));
        for expected in [134, 131, 135, 134] {
            assert_eq!(session.select_next_system(), Some(SystemId(expected)));
            assert_eq!(session.nav_mode(), hyperspace(expected));
            assert_eq!(session.course(), [], "no course plotted");
            assert_eq!(session.take_sounds(), [SimSound::HyperSelected]);
        }
    }

    #[test]
    fn hyper_select_cycles_from_the_system_selected_not_the_course() {
        let mut session = Session::start(&hub()).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        session.chart.nav = hyperspace(135);
        assert_eq!(session.select_next_system(), Some(SystemId(134)), "wraps");
    }

    #[test]
    fn hyper_select_from_a_stellar_starts_at_the_first_link() {
        let mut session = Session::start(&hub()).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        session.select_stellar(StellarPick::Slot(0));
        assert_eq!(session.select_next_system(), Some(SystemId(134)));
        assert_eq!(session.nav_mode(), hyperspace(134));
    }

    /// [`hub`]'s session flown from 133, which lists no links.
    fn linkless() -> Session {
        let mut catalog = hub();
        catalog.character = starting([Some(133), None, None, None]).character;
        let session = Session::start(&catalog).expect("starts");
        assert_eq!(session.system(), SystemId(133));
        quiet(session)
    }

    #[test]
    fn hyper_select_with_no_links_in_mode_3_selects_no_destination_in_silence() {
        let mut session = linkless();
        assert!(session.hyperspace_mode());
        session.chart.nav = hyperspace(130);
        session.take_sounds();
        assert_eq!(session.select_next_system(), None);
        assert_eq!(session.nav_mode(), NavMode::Hyperspace(None));
        assert_eq!(session.take_sounds(), []);
        assert_eq!(session.course(), []);
    }

    #[test]
    fn hyper_select_with_no_links_lets_go_of_a_stellar_and_keeps_mode_2() {
        let mut session = linkless();
        session.chart.nav = stellar(128);
        assert_eq!(session.select_next_system(), None);
        assert_eq!(session.nav_mode(), NavMode::Stellar(None));
        assert_eq!(session.take_sounds(), []);
    }

    #[test]
    fn hyper_select_by_the_one_jump_course_reading_beeps_and_selects_too() {
        let mut session = quiet(
            Session::start(&hub())
                .expect("starts")
                .with_hyper_select(HyperSelectRule::OneJumpCourse),
        );
        assert_eq!(session.select_next_system(), Some(SystemId(131)));
        assert_eq!(session.nav_mode(), hyperspace(131));
        assert_eq!(session.take_sounds(), [SimSound::HyperSelected]);
    }

    // The stellar keys, the click and the land key.

    #[test]
    fn a_stellar_key_beeps_whenever_it_finds_a_stellar_even_the_one_selected() {
        let mut session = quiet(Session::start(&three_stellars()).expect("starts"));
        for _ in 0..2 {
            assert_eq!(
                session.select_stellar(StellarPick::Slot(0)),
                Some(StellarId(131))
            );
            assert_eq!(session.nav_mode(), stellar(131));
            assert_eq!(session.take_sounds(), [SimSound::NavSelected]);
        }
        assert_eq!(
            session.select_stellar(StellarPick::Nearest),
            Some(StellarId(128))
        );
        assert_eq!(session.take_sounds(), [SimSound::NavSelected]);
        assert_eq!(session.select_stellar(StellarPick::Slot(3)), None, "empty");
        assert_eq!(session.take_sounds(), [], "an empty slot is silent");
        assert_eq!(session.nav_mode(), stellar(128));
    }

    #[test]
    fn a_click_on_the_stellar_selected_alone_does_nothing_and_makes_no_sound() {
        let mut session = quiet(Session::start(&three_stellars()).expect("starts"));
        let on_131 = crate::geometry::Vec2::new(900.0, 0.0);
        assert_eq!(click_stellar(&mut session, on_131), Some(StellarId(131)));
        assert_eq!(session.take_sounds(), [SimSound::NavSelected]);
        session.tick(Controls::default());
        let expand = session.nav_brackets().expect("bracketed").expand;
        assert_eq!(click_stellar(&mut session, on_131), None);
        assert_eq!(session.take_sounds(), []);
        assert_eq!(session.nav_mode(), stellar(131));
        assert_eq!(
            session.nav_brackets().expect("bracketed").expand,
            expand,
            "no fly-in"
        );
    }

    #[test]
    fn a_click_on_a_stellar_from_mode_3_beeps_and_selects_it() {
        let mut session = Session::start(&three_stellars()).expect("starts");
        session.hyperspace_mode();
        let mut session = quiet(session);
        let on_131 = crate::geometry::Vec2::new(900.0, 0.0);
        assert_eq!(click_stellar(&mut session, on_131), Some(StellarId(131)));
        assert_eq!(session.nav_mode(), stellar(131));
        assert_eq!(session.take_sounds(), [SimSound::NavSelected]);
    }

    #[test]
    fn the_land_key_selects_the_nearest_in_silence_from_any_other_mode() {
        for before in [
            NavMode::Off,
            NavMode::Stellar(None),
            NavMode::Hyperspace(None),
            hyperspace(131),
        ] {
            let mut session = quiet(Session::start(&three_stellars()).expect("starts"));
            session.chart.nav = before;
            assert!(matches!(
                session.land(),
                Ok(LandPress::Outcome(LandOutcome::Requested { .. }))
            ));
            assert_eq!(session.nav_mode(), stellar(128), "{before:?}");
            assert!(
                !session.take_sounds().contains(&SimSound::NavSelected),
                "{before:?}"
            );
        }
    }

    // The jump.

    #[test]
    fn the_jump_goes_to_the_system_selected_rather_than_the_courses_first_hop() {
        let mut session = Session::start(&hub()).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        session.chart.nav = hyperspace(134);
        fly_out(&mut session);
        assert_eq!(session.jump_readiness(), JumpReadiness::Clear);
        assert_eq!(begin_jump_now(&mut session), Ok(SystemId(134)));
    }

    #[test]
    fn a_selection_the_ship_cannot_jump_to_is_no_destination() {
        let mut session = Session::start(&hub()).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        for mode in [hyperspace(132), hyperspace(136), stellar(128), NavMode::Off] {
            session.chart.nav = mode;
            fly_out(&mut session);
            assert_eq!(
                session.begin_jump(),
                Err(JumpRefusal::NoDestination),
                "{mode:?}"
            );
        }
    }

    // The arrival, the landing and the take-off.

    #[test]
    fn an_arrival_turns_the_nav_off_or_cues_the_next_hop_while_the_course_goes_on() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        fly_out(&mut session);
        begin_jump_now(&mut session).expect("jumps");
        session.arrive(&catalog, &mut NeverFires).expect("arrives");
        assert_eq!(session.course(), ids(&[132]));
        assert_eq!(session.nav_mode(), hyperspace(132), "cued");

        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        begin_jump_now(&mut session).expect("jumps");
        session.arrive(&catalog, &mut NeverFires).expect("arrives");
        assert_eq!(session.course(), []);
        assert_eq!(session.nav_mode(), NavMode::Off, "the course is done");
    }

    #[test]
    fn landing_keeps_the_stellar_selected_and_the_take_off_lets_go_of_it() {
        let catalog = three_stellars();
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        assert_eq!(session.nav_mode(), stellar(128), "landing writes neither");
        session.take_off(&catalog, &mut NeverFires);
        assert_eq!(session.nav_mode(), NavMode::Stellar(None));
        assert_eq!(session.nav_target(), None);
    }

    #[test]
    fn the_take_off_with_a_course_cues_its_first_hop() {
        let catalog = three_stellars();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        session.take_off(&catalog, &mut NeverFires);
        assert_eq!(session.nav_mode(), hyperspace(131));
    }

    // Gates and set expressions.

    #[test]
    fn a_hypergate_turns_the_nav_off_on_exit_on_cancel_and_without_links() {
        let catalog = gated();
        let mut session = at_gate(&catalog);
        assert_eq!(
            session.enter_hypergate(Some(SystemId(131)), &catalog, &mut NeverFires),
            Ok(SystemId(131))
        );
        assert_eq!(session.nav_mode(), NavMode::Off, "exit");

        let mut session = at_gate(&catalog);
        assert_eq!(
            session.enter_hypergate(None, &catalog, &mut NeverFires),
            Err(GateRefusal::Cancelled)
        );
        assert_eq!(session.nav_mode(), NavMode::Off, "cancel");

        let catalog = FakePilotCatalog {
            gates: vec![],
            ..gated()
        };
        let mut session = at_gate(&catalog);
        assert_eq!(session.open_hypergate(&catalog), Err(GateRefusal::NoLinks));
        assert_eq!(session.nav_mode(), NavMode::Off, "no links");
    }

    #[test]
    fn a_wormhole_lets_go_of_the_selection_and_keeps_the_mode() {
        let catalog = wormholes();
        let mut session = at_wormhole(&catalog);
        assert_eq!(session.nav_mode(), stellar(400));
        session
            .enter_wormhole(&catalog, &mut crate::testkit::Scripted::rolling(&[0]))
            .expect("comes out");
        assert_eq!(session.nav_mode(), NavMode::Stellar(None));
    }

    #[test]
    fn a_wormhole_with_no_exit_lets_go_of_the_selection_and_keeps_the_mode() {
        let mut catalog = wormholes();
        catalog.gates.retain(|site| site.id == StellarId(400));
        for (before, after) in [
            (stellar(400), NavMode::Stellar(None)),
            (hyperspace(131), NavMode::Hyperspace(None)),
        ] {
            let mut session = at_wormhole(&catalog);
            session.chart.nav = before;
            assert_eq!(session.nav_mode(), before, "a selection to let go of");
            assert_eq!(
                session.enter_wormhole(&catalog, &mut NeverFires),
                Err(GateRefusal::NoExit)
            );
            assert_eq!(session.nav_mode(), after, "{before:?}");
        }
    }

    /// Runs `M131` and settles it.
    fn move_to_131(session: &mut Session, catalog: &FakePilotCatalog) {
        session.run_set(&SetExpr::parse("M131").expect("parses"), &mut NeverFires);
        let _ = session.settle_script(catalog, &mut NeverFires);
    }

    #[test]
    fn an_m_in_flight_lets_go_of_the_selection_and_keeps_the_mode() {
        for (before, after) in [
            (stellar(128), NavMode::Stellar(None)),
            (hyperspace(131), NavMode::Hyperspace(None)),
            (NavMode::Off, NavMode::Off),
        ] {
            let catalog = catalog();
            let mut session = Session::start(&catalog).expect("starts");
            session.plot_course(SystemId(132)).expect("a route");
            session.chart.nav = before;
            move_to_131(&mut session, &catalog);
            assert_eq!(session.system(), SystemId(131));
            assert_eq!(session.nav_mode(), after, "{before:?}");
        }
    }

    #[test]
    fn in_flight_the_operator_leaves_the_selection_to_the_settling() {
        let catalog = catalog();
        for expr in ["M131", "N131"] {
            let mut session = Session::start(&catalog).expect("starts");
            session.chart.nav = stellar(128);
            run(&mut session, expr);
            assert_eq!(session.nav_mode(), stellar(128), "{expr}");
            let _ = session.settle_script(&catalog, &mut NeverFires);
            assert_eq!(session.nav_mode(), NavMode::Stellar(None), "{expr}");
        }
    }

    /// [`three_stellars`], where 132 exists with no stellar, landed on
    /// 128 in mode 2.
    fn landed() -> (FakePilotCatalog, Session) {
        let mut catalog = three_stellars();
        catalog.systems.push(SystemId(132));
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        assert_eq!(session.nav_mode(), stellar(128));
        (catalog, session)
    }

    /// Runs `expr`, unsettled.
    fn run(session: &mut Session, expr: &str) {
        session.run_set(&SetExpr::parse(expr).expect("parses"), &mut NeverFires);
    }

    #[test]
    fn an_m_landed_selects_the_destinations_first_stellar_at_once() {
        let (catalog, mut session) = landed();
        run(&mut session, "M131");
        assert_eq!(session.nav_mode(), stellar(140), "at the operator");
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert_eq!(session.pilot().system(), SystemId(131));
        assert_eq!(session.landed(), Some(StellarId(128)), "still docked");
        assert_eq!(session.nav_mode(), stellar(140), "settled");
        session.take_off(&catalog, &mut NeverFires);
        assert_eq!(
            session.nav_mode(),
            NavMode::Stellar(None),
            "the take-off lets go of it"
        );
    }

    #[test]
    fn the_take_off_notes_the_nav_held_before_its_deselect() {
        let (catalog, mut session) = landed();
        assert_eq!(session.take_takeoff_nav(), None, "no take-off yet");
        run(&mut session, "M131");
        let _ = session.settle_script(&catalog, &mut NeverFires);
        session.take_off(&catalog, &mut NeverFires);
        assert_eq!(session.nav_mode(), NavMode::Stellar(None), "let go");
        assert_eq!(session.take_takeoff_nav(), Some(stellar(140)), "as held");
        assert_eq!(session.take_takeoff_nav(), None, "taken");
        session.take_off(&catalog, &mut NeverFires);
        assert_eq!(session.take_takeoff_nav(), None, "not landed");
    }

    #[test]
    fn an_m_landed_to_a_system_with_no_stellar_lets_go() {
        let (catalog, mut session) = landed();
        run(&mut session, "M132");
        assert_eq!(
            session.nav_mode(),
            NavMode::Stellar(None),
            "at the operator"
        );
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert_eq!(session.pilot().system(), SystemId(132));
        assert_eq!(session.nav_mode(), NavMode::Stellar(None), "settled");
    }

    #[test]
    fn an_m_landed_in_off_or_mode_3_selects_no_stellar() {
        for (before, after) in [
            (NavMode::Off, NavMode::Off),
            (hyperspace(131), NavMode::Hyperspace(None)),
        ] {
            let (catalog, mut session) = landed();
            session.chart.nav = before;
            run(&mut session, "M131");
            assert_eq!(session.nav_mode(), after, "{before:?} at the operator");
            let _ = session.settle_script(&catalog, &mut NeverFires);
            assert_eq!(session.pilot().system(), SystemId(131));
            assert_eq!(session.nav_mode(), after, "{before:?} settled");
        }
    }

    #[test]
    fn an_n_landed_lets_go_and_keeps_the_mode() {
        for (before, after) in [
            (stellar(128), NavMode::Stellar(None)),
            (hyperspace(131), NavMode::Hyperspace(None)),
            (NavMode::Off, NavMode::Off),
        ] {
            let (catalog, mut session) = landed();
            session.chart.nav = before;
            run(&mut session, "N131");
            assert_eq!(session.nav_mode(), after, "{before:?} at the operator");
            let _ = session.settle_script(&catalog, &mut NeverFires);
            assert_eq!(session.pilot().system(), SystemId(131));
            assert_eq!(session.nav_mode(), after, "{before:?} settled");
        }
    }

    #[test]
    fn a_landed_move_to_a_system_that_does_not_exist_writes_nothing() {
        for expr in ["M999", "N999"] {
            for before in [stellar(128), hyperspace(131)] {
                let (catalog, mut session) = landed();
                session.chart.nav = before;
                run(&mut session, expr);
                assert_eq!(session.nav_mode(), before, "{expr} {before:?}");
                let _ = session.settle_script(&catalog, &mut NeverFires);
                assert_eq!(session.pilot().system(), SystemId(130), "no move");
                assert_eq!(session.nav_mode(), before, "{expr} {before:?}");
            }
        }
    }

    // The port dialogs' save and restore.

    #[test]
    fn a_port_dialogs_restore_undoes_a_landed_m_or_n() {
        for expr in ["M131", "N131"] {
            for before in [stellar(128), hyperspace(131), NavMode::Off] {
                let (catalog, mut session) = landed();
                session.chart.nav = before;
                session.save_port_nav();
                run(&mut session, expr);
                let _ = session.settle_script(&catalog, &mut NeverFires);
                session.restore_port_nav();
                assert_eq!(session.nav_mode(), before, "{expr} {before:?}");
                assert_eq!(
                    session.pilot().system(),
                    SystemId(131),
                    "only the nav is restored"
                );
            }
        }
    }

    #[test]
    fn a_port_nav_restore_without_a_save_changes_nothing_and_restores_once() {
        let (catalog, mut session) = landed();
        session.restore_port_nav();
        assert_eq!(session.nav_mode(), stellar(128), "nothing saved");

        session.save_port_nav();
        session.restore_port_nav();
        run(&mut session, "N131");
        let _ = session.settle_script(&catalog, &mut NeverFires);
        session.restore_port_nav();
        assert_eq!(
            session.nav_mode(),
            NavMode::Stellar(None),
            "the save was restored already"
        );
    }

    // A mission dialog's map's save and restore.

    #[test]
    fn a_dialogs_map_nav_is_put_back_with_its_slot_and_the_course_kept() {
        let mut session = Session::start(&hub()).expect("starts");
        // `hub`'s 130 lists 131 in slots 1 and 2: the second is noted.
        let noted = NavSlot {
            system: SystemId(130),
            slot: 2,
            selected: SystemId(131),
        };
        session.chart.nav = hyperspace(131);
        session.chart.nav_slot = Some(noted);
        let saved = session.map_nav();
        assert_eq!(session.chart.nav, hyperspace(131), "saving changes nothing");
        session.chart.select_at_place(&session.pilot, SystemId(134));
        session.pilot.course = ids(&[135]);
        session.restore_map_nav(saved);
        assert_eq!(session.chart.nav, hyperspace(131));
        assert_eq!(session.chart.nav_slot, Some(noted), "the slot with it");
        assert_eq!(session.course(), ids(&[135]), "the course kept");
    }

    #[test]
    fn a_dialogs_map_nav_puts_back_any_mode_raw() {
        for before in [NavMode::Off, stellar(128), NavMode::Hyperspace(None)] {
            let mut session = Session::start(&hub()).expect("starts");
            session.chart.nav = before;
            let saved = session.map_nav();
            session.chart.select_at_place(&session.pilot, SystemId(134));
            session.restore_map_nav(saved);
            assert_eq!(session.chart.nav, before, "{before:?}");
            assert_eq!(session.chart.nav_slot, None, "{before:?}: none noted");
        }
    }

    #[test]
    fn the_maps_put_back_of_its_selection_keeps_the_mode() {
        // Clear Route docked lets go of the selection, keeping the mode:
        // the put-back gives it back.
        let (_, mut session) = landed();
        let saved = session.map_nav();
        session.clear_route();
        assert_eq!(session.nav_mode(), NavMode::Stellar(None));
        session.restore_map_selection(saved);
        assert_eq!(session.nav_mode(), stellar(128));
        // In another mode now, which only the planner's plotting gives
        // docked, the selection saved names nothing of it.
        let saved = session.map_nav();
        session.chart.set_nav(hyperspace(131));
        session.restore_map_selection(saved);
        assert_eq!(session.nav_mode(), hyperspace(131), "left as it is");
    }

    // The map.

    #[test]
    fn plotting_a_course_cues_its_first_hop_from_any_mode() {
        for before in [NavMode::Off, stellar(128), NavMode::Hyperspace(None)] {
            let mut session = Session::start(&three_stellars()).expect("starts");
            session.chart.nav = before;
            session.plot_course(SystemId(132)).expect("a route");
            assert_eq!(session.nav_mode(), hyperspace(131), "{before:?}");
        }
    }

    #[test]
    fn a_failed_plot_lets_go_of_the_selection_and_keeps_the_mode() {
        for (before, after) in [
            (hyperspace(134), NavMode::Hyperspace(None)),
            (stellar(128), NavMode::Stellar(None)),
            (NavMode::Off, NavMode::Off),
        ] {
            let mut session = Session::start(&hub()).expect("starts");
            session.chart.nav = before;
            assert_eq!(
                session.plot_course(SystemId(136)),
                Err(RouteError::Unreachable)
            );
            assert_eq!(session.nav_mode(), after, "{before:?}");
        }
    }

    #[test]
    fn cueing_selects_an_adjacent_first_hop_and_nothing_else() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.pilot.course = ids(&[131, 132]);
        session.cue_course();
        assert_eq!(session.nav_mode(), hyperspace(131));

        for before in [NavMode::Off, stellar(128), NavMode::Hyperspace(None)] {
            session.chart.nav = before;
            session.pilot.course = ids(&[132]);
            session.cue_course();
            assert_eq!(session.nav_mode(), before, "132 is not linked from 130");
            session.pilot.course.clear();
            session.cue_course();
            assert_eq!(session.nav_mode(), before, "no course");
        }
        assert_eq!(session.take_sounds(), [], "silent");
    }

    /// `catalog` [`hideable`]'s session, its course's first hop 132
    /// hidden alone at its place.
    fn unplaced(mut catalog: FakePilotCatalog, source: RuleSource) -> Session {
        hideable(&mut catalog);
        let mut session = Session::start(&catalog)
            .expect("starts")
            .with_rules(Rulebook::default().with_override(RuleKey::CueUnplacedHop, source));
        assert!(hide(&mut session, &catalog, &[132]));
        session.pilot.course = ids(&[132]);
        quiet(session)
    }

    #[test]
    fn the_cue_of_an_unplaced_hop_selects_the_first_dead_slot_by_the_engine() {
        // `hub`'s 130 lists 134, 131, 131, 130, 999 and 135: 999, in the
        // ID range with no `sÿst`, is the first slot with nothing at its
        // place (@0xe334).
        let mut session = unplaced(hub(), RuleSource::Engine);
        session.cue_course();
        assert_eq!(session.chart.nav, hyperspace(999), "the raw slot");
        assert_eq!(
            session.nav_mode(),
            NavMode::Hyperspace(None),
            "no destination"
        );
        assert_eq!(
            session
                .map_drawing(None, &[], crate::MapMarks::NONE)
                .mode3_link,
            None,
            "no link drawn"
        );
        assert_eq!(session.take_sounds(), [], "silent");
        fly_out(&mut session);
        assert_eq!(session.begin_jump(), Err(JumpRefusal::NoDestination));
        assert!(session.cycle_map_selection());
        assert_eq!(session.chart.nav, hyperspace(135), "after the slot noted");
    }

    #[test]
    fn the_cue_of_an_unplaced_hop_selects_a_slot_to_a_hidden_system_with_nothing_at_its_place() {
        let mut catalog = catalog();
        catalog.star_map[0] = star(130, (0.0, 0.0), &[131, 132]);
        let mut session = unplaced(catalog, RuleSource::Engine);
        session.cue_course();
        assert_eq!(session.chart.nav, hyperspace(132));
        assert_eq!(session.nav_mode(), NavMode::Hyperspace(None));
    }

    #[test]
    fn the_cue_of_an_unplaced_hop_passes_over_empty_slots_and_ids_out_of_range() {
        // -1 and 50 are empty slots in the original (@0x7806d-0x780b8).
        let mut catalog = catalog();
        catalog.star_map[0] = star(130, (0.0, 0.0), &[131, -1, 50, 999]);
        let mut session = unplaced(catalog, RuleSource::Engine);
        session.cue_course();
        assert_eq!(session.chart.nav, hyperspace(999));
    }

    #[test]
    fn the_cue_of_an_unplaced_hop_changes_nothing_by_the_other_reading_or_with_no_dead_slot() {
        let mut session = unplaced(hub(), RuleSource::Bible);
        session.chart.nav = stellar(128);
        session.cue_course();
        assert_eq!(session.chart.nav, stellar(128), "by the other reading");

        for source in RuleSource::ALL {
            let mut session = unplaced(catalog(), source);
            session.chart.nav = stellar(128);
            session.cue_course();
            assert_eq!(
                session.chart.nav,
                stellar(128),
                "130 lists 131 alone, {source:?}"
            );
        }
    }

    #[test]
    fn a_placed_first_hop_cues_as_ever_by_both_readings() {
        for source in RuleSource::ALL {
            let mut session = Session::start(&catalog())
                .expect("starts")
                .with_rules(Rulebook::default().with_override(RuleKey::CueUnplacedHop, source));
            session.pilot.course = ids(&[131, 132]);
            session.cue_course();
            assert_eq!(session.nav_mode(), hyperspace(131), "{source:?}");
        }
    }

    #[test]
    fn a_selection_not_in_the_catalog_reads_as_none() {
        let mut session = Session::start(&hub()).expect("starts");
        session.chart.nav = hyperspace(999);
        assert_eq!(session.nav_mode(), NavMode::Hyperspace(None));
        assert_eq!(session.map_preselection(), None);
    }

    #[test]
    fn the_selection_and_the_cue_resolve_a_hidden_system_by_place() {
        let catalog = alternates();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        assert_eq!(session.nav_mode(), hyperspace(131));
        hide(&mut session, &catalog, &[131]);
        assert_eq!(
            session.nav_mode(),
            hyperspace(134),
            "134 stands at its place"
        );
        session.chart.nav = NavMode::Off;
        session.cue_course();
        assert_eq!(session.nav_mode(), hyperspace(134));
        hide(&mut session, &catalog, &[134]);
        assert_eq!(
            session.nav_mode(),
            NavMode::Hyperspace(None),
            "nothing stands there: no hidden system named"
        );
    }

    // Option-backquote.

    #[test]
    fn clearing_the_target_beeps_only_when_there_was_one_and_leaves_the_nav_mode() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 0);
        session.select_stellar(StellarPick::Slot(0));
        assert!(session.select_target(TargetPick::Nearest).is_some());
        let mut session = quiet(session);
        assert!(session.clear_target());
        assert_eq!(session.target(), None);
        assert_eq!(session.take_sounds(), [SimSound::TargetCleared]);
        assert!(!session.clear_target(), "no target");
        assert_eq!(session.take_sounds(), []);
        assert_eq!(session.nav_mode(), stellar(128));
    }

    // The galaxy map's preselect.

    #[test]
    fn the_map_preselects_the_current_system_unless_a_system_is_selected_in_mode_3() {
        let mut session = Session::start(&hub()).expect("starts");
        assert_eq!(session.nav_mode(), NavMode::Off);
        assert_eq!(session.map_preselection(), Some(SystemId(130)), "off");
        let picked = session.select_stellar(StellarPick::Slot(0));
        assert_eq!(session.nav_mode(), NavMode::Stellar(picked));
        assert!(picked.is_some());
        assert_eq!(session.map_preselection(), Some(SystemId(130)), "a stellar");
        assert!(session.hyperspace_mode());
        assert_eq!(session.nav_mode(), NavMode::Hyperspace(None));
        assert_eq!(
            session.map_preselection(),
            Some(SystemId(130)),
            "mode 3 with nothing selected"
        );
        session.plot_course(SystemId(132)).expect("a route");
        assert_eq!(session.map_preselection(), Some(SystemId(131)), "the cue");
    }

    #[test]
    fn the_map_preselects_a_hidden_selection_by_place_or_nothing() {
        let catalog = alternates();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        assert_eq!(session.map_preselection(), Some(SystemId(131)));
        hide(&mut session, &catalog, &[131]);
        assert_eq!(
            session.map_preselection(),
            Some(SystemId(134)),
            "134 stands at its place"
        );
        hide(&mut session, &catalog, &[134]);
        assert_eq!(
            session.map_preselection(),
            None,
            "nothing stands there: no selection, not the current system"
        );
    }

    // The floating map.

    #[test]
    fn the_floating_maps_links_are_the_raw_slots_in_order_that_exist() {
        let session = Session::start(&hub()).expect("starts");
        assert_eq!(
            session.floating_map_links(),
            ids(&[134, 131, 131, 130, 135]),
            "repeats and itself kept, 999 left out"
        );
    }

    #[test]
    fn a_floating_map_link_with_nothing_visible_at_its_place_is_left_out() {
        let catalog = alternates();
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(session.floating_map_links(), ids(&[131]));
        hide(&mut session, &catalog, &[131]);
        assert_eq!(
            session.floating_map_links(),
            ids(&[131]),
            "134 stands at its place: the raw slot is kept"
        );
        hide(&mut session, &catalog, &[134]);
        assert_eq!(session.floating_map_links(), []);
    }

    #[test]
    fn a_floating_map_click_sets_mode_3_with_its_system_or_nothing_and_beeps() {
        let mut session = Session::start(&hub()).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        session.take_sounds();
        assert!(session.select_on_floating_map(Some(SystemId(135))));
        assert_eq!(session.nav_mode(), hyperspace(135));
        assert_eq!(session.take_sounds(), [SimSound::HyperSelected]);
        assert_eq!(session.course(), ids(&[131]), "the course kept");
        session.select_stellar(StellarPick::Slot(0));
        session.take_sounds();
        assert!(session.select_on_floating_map(None));
        assert_eq!(session.nav_mode(), NavMode::Hyperspace(None));
        assert_eq!(session.take_sounds(), [SimSound::HyperSelected]);
    }

    // The galaxy map's Tab / `\` cycle.

    /// Cycles the map's selection once, and gives the raw selection.
    fn cycle(session: &mut Session) -> NavMode {
        assert!(session.cycle_map_selection());
        session.chart.nav
    }

    #[test]
    fn the_map_cycle_sets_mode_3_with_the_next_listed_link_from_the_selection_and_wraps() {
        // `hub`'s 130 lists 134, 131, 131, 130, 999, 135.
        let mut session = Session::start(&hub()).expect("starts");
        session.chart.nav = hyperspace(134);
        assert_eq!(cycle(&mut session), hyperspace(131));
        assert_eq!(session.nav_mode(), hyperspace(131));
        session.chart.nav = hyperspace(135);
        assert_eq!(cycle(&mut session), hyperspace(134), "wraps");
    }

    #[test]
    fn the_map_cycle_visits_a_repeated_link_once_per_slot() {
        let mut session = Session::start(&hub()).expect("starts");
        for expected in [134, 131, 131, 130, 135, 134] {
            assert_eq!(cycle(&mut session), hyperspace(expected));
        }
    }

    /// [`alternates`] with 130 listing 133, the missing 999, 131 and
    /// itself, in that order, then 131 and 133 hidden: 134 stands at 131's
    /// place, and nothing at 133's.
    fn shadowed() -> Session {
        let mut catalog = catalog();
        catalog.star_map[0] = star(130, (0.0, 0.0), &[133, 999, 131, 130]);
        with_alternate(&mut catalog);
        let mut session = Session::start(&catalog).expect("starts");
        assert!(hide(&mut session, &catalog, &[131, 133]));
        session
    }

    #[test]
    fn a_map_cycle_after_hyper_select_steps_on_from_the_first_slot_naming_its_system() {
        // `hub`'s 130 lists 134, 131, 131, 130, 999, 135. The cycle notes
        // slot 2 for 131; Hyper Select comes back round to 131, writing
        // slot 1 (@0x69cce), and the next cycle steps on to slot 2.
        let mut session = Session::start(&hub()).expect("starts");
        for _ in 0..3 {
            cycle(&mut session);
        }
        assert_eq!(session.chart.nav, hyperspace(131), "slot 2");
        for expected in [135, 134, 131] {
            assert_eq!(session.select_next_system(), Some(SystemId(expected)));
        }
        assert_eq!(cycle(&mut session), hyperspace(131), "slot 2, after slot 1");
    }

    #[test]
    fn a_map_cycle_after_a_floating_map_click_steps_on_from_the_first_slot_naming_its_system() {
        // The click writes the first slot whose box it hits
        // (@0x6a332 → @0x6a719), and a repeated link's slots share a box.
        let mut session = Session::start(&hub()).expect("starts");
        for _ in 0..3 {
            cycle(&mut session);
        }
        assert!(session.nav_off());
        assert!(session.select_on_floating_map(Some(SystemId(131))));
        assert_eq!(cycle(&mut session), hyperspace(131), "slot 2, after slot 1");
    }

    #[test]
    fn the_map_cycle_passes_over_slots_with_no_visible_system_and_keeps_the_raw_slot() {
        let mut session = shadowed();
        assert_eq!(cycle(&mut session), hyperspace(131), "the raw slot");
        assert_eq!(session.nav_mode(), hyperspace(134), "read by place");
        assert_eq!(cycle(&mut session), hyperspace(130), "itself");
        assert_eq!(
            cycle(&mut session),
            hyperspace(131),
            "133 and 999 passed over"
        );
    }

    #[test]
    fn the_map_cycle_from_off_or_mode_3_with_none_starts_at_the_first_link() {
        for before in [
            NavMode::Off,
            NavMode::Hyperspace(None),
            NavMode::Stellar(None),
        ] {
            let mut session = Session::start(&hub()).expect("starts");
            session.chart.nav = before;
            assert_eq!(cycle(&mut session), hyperspace(134), "{before:?}");
        }
    }

    #[test]
    fn the_map_cycle_in_a_system_with_no_links_selects_none_in_mode_3() {
        for before in [NavMode::Off, stellar(128), hyperspace(130)] {
            let mut session = linkless();
            session.chart.nav = before;
            assert_eq!(cycle(&mut session), NavMode::Hyperspace(None), "{before:?}");
        }
    }

    #[test]
    fn the_map_cycle_makes_no_sound_and_keeps_the_course() {
        let mut session = Session::start(&hub()).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        let mut session = quiet(session);
        assert_eq!(cycle(&mut session), hyperspace(131), "after the cue's slot");
        assert_eq!(session.take_sounds(), []);
        assert_eq!(session.course(), ids(&[131, 132]), "the course kept");
    }

    #[test]
    fn no_map_cycle_while_landed() {
        let mut session = Session::start(&hub()).expect("starts");
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        assert!(!session.cycle_map_selection());
        assert_eq!(session.nav_mode(), stellar(128));
    }

    #[test]
    fn a_map_cycle_after_the_cue_steps_on_from_the_first_slot_at_the_hops_place() {
        // `_CueNextHyperRouteDest` writes the first slot whose system
        // stands at the hop's place (@0xe30f-0xe344): here 131's, hidden
        // behind 134, before the slot naming 134 itself.
        let mut catalog = catalog();
        catalog.star_map[0] = star(130, (0.0, 0.0), &[131, 134, 132]);
        with_alternate(&mut catalog);
        let mut session = Session::start(&catalog).expect("starts");
        assert!(hide(&mut session, &catalog, &[131]));
        session.pilot.course = ids(&[134]);
        session.cue_course();
        assert_eq!(session.chart.nav, hyperspace(134));
        assert_eq!(cycle(&mut session), hyperspace(134), "slot 1, after slot 0");
        assert_eq!(cycle(&mut session), hyperspace(132));
    }

    #[test]
    fn a_map_cycle_from_a_stellar_starts_after_its_index_by_the_engine_and_at_the_first_link_by_the_other_reading()
     {
        let other = Rulebook::default().with_override(RuleKey::MapCycleStart, RuleSource::Bible);
        for (stellar_id, engine, by_other) in [
            (129, 131, 134), // index 1: slot 2 next
            (128, 131, 134), // index 0: slot 1 next
            (143, 134, 134), // index 15: wraps to slot 0
            (150, 134, 134), // index 22: slot 0
        ] {
            for (rules, expected) in [(Rulebook::default(), engine), (other, by_other)] {
                let source = rules.source_for(RuleKey::MapCycleStart);
                let mut session = Session::start(&hub()).expect("starts").with_rules(rules);
                session.chart.nav = stellar(stellar_id);
                assert_eq!(
                    cycle(&mut session),
                    hyperspace(expected),
                    "{stellar_id} by {source:?}"
                );
            }
        }
    }

    #[test]
    fn a_map_cycle_from_a_system_no_slot_stands_for_starts_at_the_first_link() {
        let mut session = Session::start(&hub()).expect("starts");
        session.chart.nav = hyperspace(132);
        assert_eq!(cycle(&mut session), hyperspace(134));
    }

    #[test]
    fn a_map_cycle_from_a_system_no_slot_names_steps_on_from_the_slot_at_its_place() {
        // 134 shares 131's place, and 131, the lower, stands for it.
        let mut catalog = catalog();
        catalog.star_map[0] = star(130, (0.0, 0.0), &[132, 131]);
        with_alternate(&mut catalog);
        let mut session = Session::start(&catalog).expect("starts");
        session.chart.nav = hyperspace(134);
        assert_eq!(cycle(&mut session), hyperspace(132), "after 131's slot");
    }

    #[test]
    fn a_map_cycle_steps_from_below_the_first_slot_to_the_last() {
        let mut catalog = hub();
        let mut links = [-1; 16];
        (links[0], links[15]) = (134, 135);
        catalog.star_map[0] = star(130, (0.0, 0.0), &links);
        for (stellar_id, expected) in [(100, 135), (142, 135), (143, 134)] {
            let mut session = Session::start(&catalog).expect("starts");
            session.chart.nav = stellar(stellar_id);
            assert_eq!(
                cycle(&mut session),
                hyperspace(expected),
                "index {}",
                stellar_id - 128
            );
        }
    }

    #[test]
    fn a_map_cycle_from_a_stellar_counts_the_empty_slots_by_the_engine() {
        let mut catalog = hub();
        catalog.star_map[0] = star(130, (0.0, 0.0), &[-1, 131, -1, 132]);
        let mut session = Session::start(&catalog).expect("starts");
        session.chart.nav = stellar(129);
        assert_eq!(cycle(&mut session), hyperspace(132), "after slot 1, 131's");
    }

    #[test]
    fn a_floating_map_click_does_nothing_during_a_jump() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        begin_jump_now(&mut session).expect("jumps");
        let mode = session.nav_mode();
        session.take_sounds();
        assert!(!session.select_on_floating_map(None));
        assert_eq!(session.nav_mode(), mode);
        assert_eq!(session.take_sounds(), []);
    }
}
