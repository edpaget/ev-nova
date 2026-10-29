//! [`FlightView`]'s HUD readouts: the radar, target and nav state, the
//! status refresh and the port dialog hand-offs.

use std::borrow::Cow;

use nova_sim::flight_frame::{FlightCommand, FlightFrame};
use nova_sim::{NavMode, RuleSource, ShipRef, StellarIff, TargetTag};

use crate::flight::cadence::{Drawn, retake_target};
use crate::flight::hud::{NavDestination, NavDisplay};
use crate::flight::radar::{self, RadarScene, RadarStellar};
use crate::flight::target::{Tag, TargetPanel};
use crate::spaceport::PortDialog;
use crate::time::ticks;

use super::{FlightView, radar_ships, stellar_name};
use crate::flight::sim_port::FlightSim;

/// The HUD, the radar and the target panel, over the [`FlightSim`] port:
/// each reads the flight's frame ([`FlightFrame::hud`],
/// [`FlightFrame::target`], the ships' and stellars' records), the
/// redraws the frame the view keeps, filled afresh first, and the draws
/// the frame they are handed ([`FlightView::draw_bar`]).
impl<C, S: FlightSim> FlightView<C, S> {
    /// What the radar shows in `frame` ([`RadarScene`]): the player's ship
    /// and the other ships where they are drawn, the scene's stellars in
    /// nav order with the shape and sprite width the radar reads, and
    /// blank while the ship is landed (`_playerIsLanded`,
    /// [`FlightFrame::docked`]); each with its IFF code (a stellar's
    /// [`FrameStellar::iff`](nova_sim::flight_frame::FrameStellar::iff),
    /// unlandable for one the frame has none for), the player's IFF and
    /// density scanner, this tick's static ([`FlightView::advance_hud`]),
    /// and whether the nav mode is hyperspace
    /// ([`FlightView::nav_mode`]). A flight that failed to start draws no
    /// bar, so what its radar would show is never seen.
    pub(super) fn radar_now(&self, frame: &FlightFrame) -> RadarScene {
        let stellars = self.scene.as_ref().map_or_else(Vec::new, |scene| {
            scene
                .stellars()
                .iter()
                .map(|stellar| RadarStellar {
                    position: stellar.position,
                    square: stellar.radar_square,
                    sprite_width: stellar
                        .drawn()
                        .and_then(|sheet| sheet.as_ref().ok())
                        .map(|sheet| sheet.frame_width),
                    iff: frame
                        .stellar(stellar.id)
                        .and_then(|record| record.iff)
                        .unwrap_or(StellarIff::Unlandable),
                })
                .collect()
        });
        let gear = frame.hud.radar_gear;
        let target = frame.target.as_ref().map(|target| target.npc);
        RadarScene {
            player: self.shown_position(),
            stellars,
            ships: radar_ships(target, &self.shown_npcs()),
            blank: frame.docked(),
            iff: gear.iff,
            density_scanner: gear.density_scanner,
            static_pattern: self.radar_static,
            hyperspace_mode: matches!(frame.nav_mode, NavMode::Hyperspace(_)),
        }
    }

    /// Redraws the whole status bar (`FlightView::redraw_status`) as a
    /// dialog closes: the original's `_RefreshSAT` at a dialog's close.
    /// The gauges an order left held show the order then. Its callers:
    ///
    /// - The shops: the outfitter (@0x5dba4), the exchange (@0x5e642) and
    ///   the shipyard (@0x5f8d4, after its `_RedrawStatus` @0x5f87c and
    ///   @0x5f8c5), through [`FlightView::close_port_dialog`], before the
    ///   nav's restore.
    /// - The bar's hire screen, the shipyard's function in hire mode
    ///   (@0x5f8d4), and the mission computer (@0xa40d7, or its refusal's
    ///   @0xa395b), the same way.
    /// - The bar (@0x4945e), News (@0x48153), the ship description, the
    ///   shipyard's or the hire screen's (@0x5bac2), player info
    ///   (@0x5a725) and mission info (@0x9e769), whatever opened it (the
    ///   spaceport, the bar, the computer, the outfitter, the exchange,
    ///   the shipyard, the hire screen, either's ship description or an
    ///   offer), as the app hears from the spaceport; player info and
    ///   mission info over flight too.
    ///
    /// The shipyard's name prompt redraws the same way as it closes
    /// ([`FlightView::buy_ship`], [`FlightView::decline_ship`], @0x5eb73),
    /// and so does every map in port, the bar's and the port screens'
    /// ([`FlightView::close_map`], [`FlightView::open_port_screen_map`],
    /// @0x13bc9). The closings the original redraws at that ours can't
    /// reach (the bar's Gamble text, Preferences' menu command, the shops' faded text dialogs, an OS
    /// update event) are recorded, with their addresses, in the
    /// spaceport's module docs
    /// ([`crate::spaceport::view`]).
    pub fn refresh_status(&mut self) {
        self.redraw_status();
    }

    /// `dialog` closes, as `_DoPortDialog` closes each: its close
    /// redraws the whole bar first ([`FlightView::refresh_status`],
    /// `_RefreshSAT`), then the nav saved as it opened is restored
    /// ([`Session::restore_port_nav`](nova_sim::Session::restore_port_nav); the outfitter @0x5dbb8-0x5dbc5,
    /// the exchange @0x5e64d-0x5e65d, the shipyard @0x5f8e8-0x5f8f5, the
    /// computer @0x60084-0x60096), undoing a landed `M` or `N` run inside
    /// it. When the restored nav shows:
    ///
    /// - The outfitter and the exchange: at once, as `_DoPortDialog` runs
    ///   `_DrawStatus` right after each returns (@0x5ff04-0x5ff10,
    ///   @0x5fe99-0x5fea5 → @0x5ff10).
    /// - The mission computer: at once, as its close draws nav and cargo
    ///   itself (@0x6009a-0x600a6), even when the restore changed nothing.
    ///   Its whole-bar redraw is its loop's close's `_RefreshSAT`
    ///   (@0xa40d7), or a refusal's after its text (@0xa395b). The
    ///   re-entry after an accept skips it when the slots are full
    ///   (@0xa3814) or nothing is left (@0xa391a), but follows an accept
    ///   whose own redraws ([`FlightView::accept_mission`]) ran in the same
    ///   input. Nothing a docked session holds changes while its texts
    ///   show, so ours' redraw there draws what is already shown: an
    ///   ordering difference no frame crosses, not filed.
    /// - The shipyard, and the bar's hire screen, the same function in
    ///   hire mode: the bar keeps what its close drew, before the
    ///   restore, as no `_DrawStatus` follows it in the port
    ///   (@0x5ff85-0x5ffa3 marks only `_statusRedraw[2]`). Its restore
    ///   marks nav (`_statusRedraw[0]` @0x5f8fe), drawn at the port's next
    ///   `_DrawStatus` ours models: a trade (@0x5e33a, @0x5e59e), a
    ///   recharge (@0x5fe05), an outfitter order (@0x5c5a1, @0x5d328), each
    ///   only when it goes through, or any nav redraw.
    pub fn close_port_dialog(&mut self, dialog: PortDialog) {
        self.refresh_status();
        self.send(FlightCommand::RestorePortNav);
        if matches!(dialog, PortDialog::Shipyard | PortDialog::Hire) {
            self.nav_marked = true;
        } else {
            self.retake_nav();
        }
    }

    /// An outfitter that listed nothing returned without opening
    /// ([`SpaceportView::set_opened_outfitter`](crate::spaceport::SpaceportView::set_opened_outfitter)):
    /// `_DoPortDialog` still runs `_DrawStatus` after it, as after every
    /// outfitter (@0x5ff09-0x5ff10), which draws a nav the shipyard's
    /// restore left marked. An empty shipyard or hire screen gets no such
    /// draw: `_DoPortDialog` marks only cargo after them (@0x5ff85-0x5ffa3).
    pub fn empty_outfitter_returned(&mut self) {
        self.draw_marked_nav();
    }

    /// The port's `_DrawStatus` with nothing else redrawing nav: the nav
    /// area is redrawn when the shipyard's restore marked it
    /// ([`FlightView::close_port_dialog`]).
    pub(super) fn draw_marked_nav(&mut self) {
        if self.nav_marked {
            self.retake_nav();
        }
    }

    /// Redraws the HUD at once, as the original's `_RedrawStatus`
    /// (@0x4cac0) does: the radar's, nav area's, target panel's and
    /// gauges' snapshots are taken afresh, but the refresh timer and the
    /// target's blink are left as they are.
    ///
    /// The original calls it at flight start (`_PlayGame` @0x457d8, ours
    /// the first frame, which shows the radar as it is), on landing and
    /// take-off
    /// (`_PlayerLandOnStellar`), on arrival in a system (`_HandlePlayer`;
    /// ours whenever a system is laid out), on closing the map
    /// (`_DoSystemMap`), on entering a hypergate (`_PlayerEnterHypergate`,
    /// ours as the map closes) and on closing the comm dialog
    /// ([`FlightView::hang_up`]), the mission dialog
    /// ([`FlightView::accept_mission`], [`FlightView::refuse_mission`]) and
    /// the text dialog ([`FlightView::take_mission_texts`]). In port, and
    /// for a person's offer answered over flight, the mission dialog's
    /// and the text dialog's are held until the texts they wait for close
    /// ([`FlightView::close_mission_text`]); this
    /// redraw drops any still held, so a text never shows over a later
    /// redraw.
    ///
    /// In port the original redraws the bar at other points too, each
    /// taken here in part: the radar alone after an outfitter order, and
    /// everything at each `C`, `E` or `H` of its `OnPurchase` or `OnSell`,
    /// or of a docked mission script, that puts the ship under another
    /// `ïntf` (`_LoadIntfResource` @0x156c7 → @0x739bd,
    /// [`FlightView::stage_bar`]),
    /// the radar, nav and target on a ship purchase and everything on one
    /// under another `ïntf` ([`FlightView::buy_ship`]), the fuel gauge on a recharge
    /// ([`FlightView::recharge`]), everything as the shipyard's name
    /// prompt closes ([`FlightView::buy_ship`],
    /// [`FlightView::decline_ship`]), and everything as a dialog closes
    /// ([`FlightView::refresh_status`]).
    pub(super) fn redraw_status(&mut self) {
        self.bar_steps.clear();
        self.redraw_whole();
    }

    /// The whole bar's snapshots taken afresh ([`FlightView::redraw_status`]),
    /// keeping the redraws held for the mission texts shown.
    pub(super) fn redraw_whole(&mut self) {
        self.retake_radar();
        self.retake_nav();
        self.retake_target();
        self.retake_reserves();
    }

    /// Takes the gauges' snapshot afresh, from the kept frame.
    pub(super) fn retake_reserves(&mut self) {
        self.reserves_scene = Some(self.reserves());
    }

    /// Takes the radar's snapshot afresh, and nothing else: what the
    /// original's outfitter redraws after each order that keeps the
    /// `ïntf`. Its buy (`_DrawStatus` @0x5c5a1) and sell (@0x5d328) mark
    /// only the cargo area, which ours draws live, and `_DrawStatus`
    /// redraws the radar on its 15-tick timer, nearly always run out in
    /// port. A refused order changes no gear, so the radar redrawn is the
    /// same. An order whose script, `OnPurchase` (@0x5c2cb) or `OnSell`
    /// (@0x5cff6), changes the `ïntf` redraws the whole bar instead, as at
    /// its last change (`_EvalSetExp` @0x156c7 → `_RedrawStatus(1)`
    /// @0x739bd, [`FlightView::after_outfit_order`]).
    ///
    /// It reads the kept frame, which the commands of the docked and
    /// travel code that calls it, directly or through
    /// [`FlightView::redraw_whole`], have just filled afresh.
    pub(super) fn retake_radar(&mut self) {
        self.radar_scene = Some(self.radar_now(&self.frame));
    }

    /// What the target panel is drawn for in `frame`: the target, its ship
    /// type and its shield, from its own record
    /// ([`FlightFrame::ship`]); none without a target.
    fn target_drawn(frame: &FlightFrame) -> Option<Drawn> {
        let target = frame.target.as_ref()?;
        let record = frame.ship(ShipRef::Npc(target.npc))?;
        Some(Drawn {
            npc: target.npc,
            ship: record.class,
            shield: record.shield.now,
        })
    }

    /// What the target panel shows of the target in `frame` (see
    /// [`TargetPanel`], [`FrameTarget`](nova_sim::flight_frame::FrameTarget)):
    /// its name, its subtitle or its ship type's card's, the card's
    /// picture, its status line, and its label, a government's code read
    /// from the catalog; none without a target.
    pub(super) fn target_now(&self, frame: &FlightFrame) -> Option<TargetPanel> {
        let target = frame.target.as_ref()?;
        let class = frame.ship(ShipRef::Npc(target.npc))?.class;
        let card = self.cards.get(&class);
        let tag = target.tag.and_then(|tag| match tag {
            TargetTag::Code(govt) => self.codes.get(&govt).cloned().flatten().map(Tag::Code),
            TargetTag::Escort => Some(Tag::Escort),
            TargetTag::Fighter => Some(Tag::Fighter),
        });
        Some(TargetPanel {
            name: target.name.clone(),
            subtitle: target
                .subtitle
                .clone()
                .or_else(|| card.map(|card| card.subtitle.clone()))
                .unwrap_or_default(),
            picture: card.and_then(|card| card.picture),
            status: target.status,
            tag,
        })
    }

    /// Takes the target panel's snapshot afresh, from the kept frame.
    pub(super) fn retake_target(&mut self) {
        self.target_scene = Some((
            Self::target_drawn(&self.frame),
            self.target_now(&self.frame),
        ));
    }

    /// What the target panel shows this frame, of `frame`'s target: by
    /// [`RuleKey::HudCadence`](nova_sim::RuleKey::HudCadence)'s engine
    /// reading, the snapshot taken at its last redraw while it is of the
    /// target the player has now; by the other, before the first redraw,
    /// or once the target has changed, the panel as it is now
    /// ([`FlightView::target_now`]).
    pub(super) fn target_shown(&self, frame: &FlightFrame) -> Cow<'_, Option<TargetPanel>> {
        match &self.target_scene {
            Some((drawn, shown))
                if self.hud_waits()
                    && drawn.map(|drawn| drawn.npc)
                        == Self::target_drawn(frame).map(|now| now.npc) =>
            {
                Cow::Borrowed(shown)
            }
            _ => Cow::Owned(self.target_now(frame)),
        }
    }

    /// Rolls this tick's radar static against the interference in the
    /// player's system ([`radar::static_frame`], on the effects' chance,
    /// never the simulation's), as `_HandleStatus` (@0x494d7) does every
    /// frame; then moves the HUD's timer to the view's elapsed ticks, and
    /// takes the radar's and the nav area's snapshots afresh when each
    /// refreshes ([`HudCadence::advance`](crate::flight::cadence::HudCadence::advance)). The nav area's is also taken
    /// afresh whenever the nav mode has changed since it was taken, as
    /// each of the original's selection changes marks the area for
    /// redrawing at once (`_statusRedraw[0]`, e.g. @0x69b39, @0x69efa,
    /// @0x6a919, @0xe34d, @0x1379b). This is settled before the frame is
    /// drawn, and [`FlightView::nav_shown`] shows a changed mode at once
    /// anyway, so no boundary is crossed. While the take-off's held nav
    /// lasts ([`FlightView::took_off`]) the area is not taken; the first
    /// advance after the take-off stamps it with its tick, and the first
    /// past that tick ends it ([`NavFlash::advance`](super::NavFlash::advance)). The target panel's
    /// is taken
    /// afresh as [`retake_target`] says: on a change of target, on its
    /// 45-tick redraw, and on a radar refresh once its shield or ship type
    /// has changed. The crossing test may settle the 45-tick redraw a
    /// frame after the original's exact `% 45` frame would, which the
    /// player never sees.
    ///
    /// The original rolls the threshold in `_HandleStatus` and picks the
    /// pattern as `_DrawStatusRadar` draws; ours rolls both here, once a
    /// tick, which changes no decision the player can see.
    pub(super) fn advance_hud(&mut self) {
        self.radar_static =
            radar::static_frame(self.frame.hud.interference, &mut self.effects_chance);
        let now = u64::try_from(ticks(self.elapsed)).unwrap_or(u64::MAX);
        let refresh = self.hud_cadence.advance(now);
        if refresh.radar {
            self.retake_radar();
        }
        self.nav_flash = self
            .nav_flash
            .and_then(|flash| flash.advance(u128::from(now)));
        if self.nav_flash.is_none() && (refresh.nav || !self.nav_mode_shown()) {
            self.retake_nav();
        }
        let shown = self.target_scene.as_ref().and_then(|(drawn, _)| *drawn);
        let now = Self::target_drawn(&self.frame);
        if self.target_scene.is_none() || retake_target(shown.as_ref(), now.as_ref(), refresh) {
            self.retake_target();
        }
    }

    /// What the nav area shows this frame: by [`RuleKey::HudCadence`](nova_sim::RuleKey::HudCadence)'s
    /// engine reading, the snapshot taken at its last refresh, text and
    /// colours both, in flight while the nav mode is the one it was taken
    /// for and while docked whatever it is; by the other, before the first
    /// refresh, or in flight once the nav mode has changed since, the nav
    /// area as it is now ([`FlightView::nav_now`]).
    ///
    /// Flight doesn't advance in port, so while docked only the port's
    /// nav redraws ours models retake the snapshot
    /// ([`FlightView::redraw_status`]: landing, a docked start, a shop
    /// closing, an accept or refusal, a mission text closing
    /// ([`FlightView::close_mission_text`]), and an outfitter order or a
    /// docked mission script whose `C`, `E` or `H` changes the `ïntf`, as
    /// at that change; a ship purchase, [`FlightView::buy_ship`];
    /// a port dialog's close, [`FlightView::close_port_dialog`]; and the
    /// other closings [`FlightView::refresh_status`] lists). A
    /// landed `M` or `N`'s write (`_EvalSetExp` @0x156f2-0x15705,
    /// @0x158fd) shows from the next of them, as the original's port bar
    /// shows it only at its next nav redraw; one run inside a port dialog
    /// is undone as it closes. A trade, a recharge, an outfitter order
    /// that keeps the `ïntf` leave it held, unless
    /// the shipyard's restore marked the nav, which a trade, recharge or
    /// outfitter order that goes through then draws. The original's port
    /// `_DrawStatus` also draws nav when it lands on `TickCount() % 30 ==
    /// 0` (@0x4c88a-0x4c8b6), recorded and not built (see the `cadence`
    /// module). A write held in port shows at the next redraw ours
    /// models, or the take-off lets go of it. The take-off's own held nav
    /// shows in flight whatever the mode, in the first frame drawn after
    /// it and to the end of that frame's tick ([`FlightView::took_off`]).
    /// The snapshot shows in flight whatever the mode, too, while the
    /// texts of a person's offer answered over flight show
    /// ([`FlightView::answer_in_flight`]).
    pub(super) fn nav_shown(&self, frame: &FlightFrame) -> NavDisplay {
        match &self.nav_scene {
            Some((_, shown))
                if self.hud_waits()
                    && (frame.docked()
                        || !self.bar_steps.is_empty()
                        || self.nav_flash.is_some()
                        || self.nav_mode_shown()) =>
            {
                shown.clone()
            }
            _ => self.nav_now(),
        }
    }

    /// Whether the HUD's content waits for its refreshes: the engine's
    /// reading of [`RuleKey::HudCadence`](nova_sim::RuleKey::HudCadence),
    /// as the view's presentation holds it
    /// ([`Presentation`](super::stepping::Presentation)).
    pub(super) fn hud_waits(&self) -> bool {
        self.presentation.hud_cadence == RuleSource::Engine
    }

    /// What the radar shows this frame: by [`RuleKey::HudCadence`](nova_sim::RuleKey::HudCadence)'s
    /// engine reading, the snapshot taken at its last refresh; by the
    /// other, or before the first refresh, the radar as it is now
    /// ([`FlightView::radar_now`]).
    ///
    /// While docked, where flight doesn't advance, it is retaken at the
    /// port's redraws ([`FlightView::redraw_status`]): after each
    /// outfitter order, on a ship purchase and as a shop closes, so an
    /// IFF bought or sold shows on the port's bar at once. The original's
    /// port `_DrawStatus` skips the radar within 15 Mac ticks of its last
    /// redraw (@0x4c911-0x4c923), and flips the blink when it doesn't
    /// (@0x4c93d-0x4c94f). Ours, with no tick clock in port, always
    /// retakes it and keeps landing's blink: recorded, not built, by
    /// operator decision on 2026-10-15. The player could see the gate only
    /// on an outfitter order buying or selling IFF within 0.25 s of the
    /// last radar redraw, and the flip only in the blank radar's centre
    /// arrow, which no landable stock stellar is far enough out to draw.
    /// The port's other timers are recorded in the `cadence` module's
    /// docs.
    pub(super) fn radar_shown(&self, frame: &FlightFrame) -> Cow<'_, RadarScene> {
        match &self.radar_scene {
            Some(shown) if self.hud_waits() => Cow::Borrowed(shown),
            _ => Cow::Owned(self.radar_now(frame)),
        }
    }

    /// A port dialog (the outfitter, exchange, shipyard, the bar's hire
    /// screen or the mission computer) opens: the nav is saved
    /// ([`FlightCommand::SavePortNav`], [`Session::save_port_nav`](nova_sim::Session::save_port_nav)), as
    /// each of the original's saves +0x6a
    /// and +0x6c as it opens (the outfitter @0x5bafc-0x5bb12, the exchange
    /// @0x5dbed-0x5dc08, the shipyard and the hire screen
    /// @0x5e70f-0x5e71e, the computer @0x5ffc0-0x5ffd0). A
    /// session that failed has no nav to save.
    pub fn open_port_dialog(&mut self) {
        self.send(FlightCommand::SavePortNav);
    }

    /// Takes the nav area's snapshot afresh, noting how many mission texts
    /// the session had queued then.
    pub(super) fn retake_nav(&mut self) {
        self.nav_marked = false;
        self.nav_scene = Some((self.nav_mode(), self.nav_now()));
        self.nav_texts = self.frame.texts_queued;
    }

    /// The player's nav mode ([`FlightFrame::nav_mode`](nova_sim::flight_frame::FlightFrame::nav_mode));
    /// off without a session.
    pub(super) fn nav_mode(&self) -> NavMode {
        self.frame.nav_mode
    }

    /// What the HUD's nav area shows now, for the session's nav mode
    /// ([`FlightView::nav_display`]).
    pub(super) fn nav_now(&self) -> NavDisplay {
        self.nav_display(self.nav_mode())
    }

    /// What the HUD's nav area shows for nav mode `mode` (see
    /// [`NavDisplay`]): in mode 2 the stellar selected, by its name in the
    /// scene, or, for one the scene doesn't hold, such as a landed `M`'s
    /// in another system, by its `spöb` name, as `_DrawStatusNav` reads it
    /// with no check of the system (@0x4a0fd-0x4a149) ("No Destination"
    /// for none, or one with no name); in mode 3 the system selected, by
    /// its name on the map if the pilot has explored it ("No Destination"
    /// for none), with whether the ship can jump there now; with the nav
    /// system off, "Nav System Off". Each read from what the flight gave:
    /// the opening's names ([`Names::stellar`](nova_sim::flight_frame::Names::stellar))
    /// and the kept frame
    /// ([`FlightFrame::has_explored`](nova_sim::flight_frame::FlightFrame::has_explored),
    /// [`FlightFrame::jump_readiness`](nova_sim::flight_frame::FlightFrame::jump_readiness)).
    /// `mode` may be held or noted from before (a take-off's, a text's, an
    /// interface load's): its system reads as explored or not as the
    /// frame's set stands now, as `_DrawStatusNav` reads the flag as it
    /// draws.
    pub(super) fn nav_display(&self, mode: NavMode) -> NavDisplay {
        match mode {
            NavMode::Off => NavDisplay::Off,
            NavMode::Stellar(target) => NavDisplay::Stellar(target.and_then(|target| {
                stellar_name(
                    self.scene.as_ref(),
                    |id| self.names.stellar(id).map(str::to_owned),
                    target,
                )
            })),
            NavMode::Hyperspace(next) => NavDisplay::Hyperspace {
                destination: next.map_or(NavDestination::None, |next| {
                    self.frame
                        .has_explored(next)
                        .then(|| self.map.model().system(next))
                        .flatten()
                        .map_or(NavDestination::Unexplored, |system| {
                            NavDestination::Named(system.entry.name.clone())
                        })
                }),
                readiness: self.frame.jump_readiness,
            },
        }
    }

    /// Whether the nav area's snapshot was taken for the nav mode the
    /// player is in now.
    fn nav_mode_shown(&self) -> bool {
        let mode = self.nav_mode();
        self.nav_scene
            .as_ref()
            .is_some_and(|(shown, _)| *shown == mode)
    }
}
