//! [`FlightView`]'s spaceport shops over the [`FlightSim`] port: the
//! exchange, the recharge, the outfitter, the shipyard and the bar; and
//! the cargo's jettison, the capture's assignment and rename, and the
//! intro's mark.
//! Each sends its command
//! ([`FlightCommand`]) and hands back the flight's answer, a flight that
//! refuses it (one that never started) answering as today's failed
//! session did. What a screen shows is the flight's answer to its screen
//! ask ([`ScreenAsk`]), and a flight with nothing to answer
//! ([`ScreenAnswer::Nothing`]) shows the empty screen: no shops, no
//! outfitter, no hire list, no shipyard to name a ship in, no news. The
//! router's facts, Recharge and the intro seen, are the kept frame's.
//! The bar's redraws around each command stay the view's.

use nova_sim::capture_naming::CaptureNaming;
use nova_sim::flight_frame::{Answer, BarNews, FlightCommand, PortShops, ScreenAnswer, ScreenAsk};
use nova_sim::{
    Assigned, Assignment, HireList, HireRefusal, Hired, Order, OutfitOrder, OutfitRefusal,
    OutfitRun, Outfitter, RechargeRefusal, ShipId, ShipNaming, ShipPurchase, ShipRefusal,
    TradeRefusal,
};

use crate::Key;
use crate::flight::catalog::{CombatLooks, ShipSprites, StatusBars};
use crate::flight::sim_port::FlightSim;
use crate::system::catalog::SystemCatalog;

use super::{FlightView, Then, assigned_message, assigned_ticks, no_outfitter};

impl<C, S: FlightSim> FlightView<C, S> {
    /// The spaceport's shops, the day's rolls not drawn yet drawn on the
    /// flight's chance ([`ScreenAsk::Shops`]): the exchange, the
    /// outfitter, the shipyard and the room to hire, asked as the
    /// spaceport opens and at each refresh. None of them, and no room to
    /// hire, for a flight with nothing to answer.
    pub fn shops(&mut self) -> PortShops {
        match self.sim.screen(ScreenAsk::Shops) {
            ScreenAnswer::Shops(shops) => *shops,
            _ => PortShops::default(),
        }
    }

    /// Trades `order` in the exchange ([`FlightCommand::Trade`],
    /// `nova_sim::Session::trade`): what went through, or why not. A
    /// trade that goes through runs the exchange's `_DrawStatus`
    /// (@0x5e33a, @0x5e59e), which draws the nav when the shipyard's
    /// restore marked it ([`FlightView::close_port_dialog`]); the cargo
    /// area shows the hold traded from the frame the command leaves
    /// ([`FrameHud::cargo`](nova_sim::flight_frame::FrameHud::cargo)).
    ///
    /// # Errors
    ///
    /// The [`TradeRefusal`] the flight answers; a flight that never
    /// started has no exchange ([`TradeRefusal::NoMarket`]).
    pub fn trade(&mut self, order: Order) -> Result<u32, TradeRefusal> {
        let traded = match self.send(FlightCommand::Trade(order)) {
            Answer::Traded(traded) => traded?,
            _ => return Err(TradeRefusal::NoMarket),
        };
        self.draw_marked_nav();
        Ok(traded)
    }

    /// Recharges the fuel ([`FlightCommand::Recharge`],
    /// `nova_sim::Session::recharge`): the price paid, or why not. A
    /// recharge redraws the fuel gauge alone, as the original's marks only
    /// fuel and cargo (`_statusRedraw[4]` @0x5fdf6) before its
    /// `_DrawStatus` (@0x5fe05); the shield and armour stay as they were
    /// drawn. The nav is drawn too when the shipyard's restore marked it
    /// ([`FlightView::close_port_dialog`]). A refused recharge draws
    /// nothing.
    ///
    /// # Errors
    ///
    /// The [`RechargeRefusal`] the flight answers; a flight that never
    /// started sells no fuel ([`RechargeRefusal::NoFuel`]).
    pub fn recharge(&mut self) -> Result<i64, RechargeRefusal> {
        let paid = match self.send(FlightCommand::Recharge) {
            Answer::Recharged(paid) => paid?,
            _ => return Err(RechargeRefusal::NoFuel),
        };
        self.draw_marked_nav();
        let fuel = self.reserves().fuel;
        if let Some(held) = &mut self.reserves_scene {
            held.fuel = fuel;
        }
        Ok(paid)
    }

    /// Whether the spaceport's Recharge button is enabled, as the kept
    /// frame says
    /// ([`FlightFrame::recharge_enabled`](nova_sim::flight_frame::FlightFrame::recharge_enabled)):
    /// false for a flight that never started.
    #[must_use]
    pub fn recharge_enabled(&self) -> bool {
        self.frame.recharge_enabled
    }

    /// The outfitter of the stellar landed on alone, the day's rolls
    /// drawn on the flight's chance ([`ScreenAsk::Outfitter`]), asked as
    /// its dialog opens: none where there is none, or for a flight that
    /// never started.
    pub fn outfitter(&mut self) -> Option<Outfitter> {
        match self.sim.screen(ScreenAsk::Outfitter) {
            ScreenAnswer::Outfitter(outfitter) => Some(*outfitter),
            _ => None,
        }
    }

    /// The outfitter opens ([`FlightCommand::OpenOutfitter`],
    /// `nova_sim::Session::open_outfitter`).
    pub fn open_outfitter(&mut self) {
        self.send(FlightCommand::OpenOutfitter);
    }

    /// The outfitter closes ([`FlightCommand::CloseOutfitter`],
    /// `nova_sim::Session::close_outfitter`).
    pub fn close_outfitter(&mut self) {
        self.send(FlightCommand::CloseOutfitter);
    }

    /// The shipyard opens ([`FlightCommand::OpenShipyard`],
    /// `nova_sim::Session::open_shipyard`): its list is built and kept for
    /// the visit, so the shops asked after relist it.
    pub fn open_shipyard(&mut self) {
        self.send(FlightCommand::OpenShipyard);
    }

    /// The prompt for naming a ship of class `ship` before it is bought,
    /// its words read from the flight's catalog, its default name drawn
    /// afresh on the flight's chance ([`ScreenAsk::ShipNaming`]), asked
    /// once as its prompt opens.
    ///
    /// # Errors
    ///
    /// The [`ShipRefusal`] that applies; a flight with nothing to answer
    /// has no shipyard ([`ShipRefusal::NoShipyard`]).
    pub fn ship_naming(&mut self, ship: ShipId) -> Result<ShipNaming, ShipRefusal> {
        match self.sim.screen(ScreenAsk::ShipNaming(ship)) {
            ScreenAnswer::ShipNaming(naming) => naming,
            _ => Err(ShipRefusal::NoShipyard),
        }
    }

    /// Declines to buy a ship of class `ship` ([`FlightCommand::DeclineShip`],
    /// `nova_sim::Session::decline_ship`). The whole bar is redrawn as the
    /// name prompt closes on Cancel (`_RefreshSAT` @0x5eb73), before it.
    pub fn decline_ship(&mut self, ship: ShipId) {
        self.redraw_status();
        self.send(FlightCommand::DeclineShip(ship));
    }

    /// Opens the bar of the stellar landed on ([`FlightCommand::OpenBar`],
    /// `nova_sim::Session::open_bar`), drawing the visit's news on the
    /// flight's chance: whether it opened, false where there is no bar.
    pub fn open_bar(&mut self) -> bool {
        self.send(FlightCommand::OpenBar) == Answer::Done
    }

    /// The news of the bar visit under way, as drawn, and the picture it
    /// is drawn over ([`ScreenAsk::News`]), asked as News is pressed: none
    /// with no bar visit under way, or for a flight that never started.
    pub fn news(&mut self) -> Option<BarNews> {
        match self.sim.screen(ScreenAsk::News) {
            ScreenAnswer::News(news) => Some(news),
            _ => None,
        }
    }

    /// The ships for hire in the bar of the stellar landed on, the day's
    /// rolls drawn on the flight's chance ([`ScreenAsk::HireList`]), asked
    /// as Hire Escort opens and after each hire: none where there is no
    /// bar, or for a flight that never started.
    pub fn escorts_for_hire(&mut self) -> Option<HireList> {
        match self.sim.screen(ScreenAsk::HireList) {
            ScreenAnswer::HireList(list) => Some(*list),
            _ => None,
        }
    }

    /// Hires a ship of class `ship` ([`FlightCommand::Hire`],
    /// `nova_sim::Session::hire`), on the flight's chance.
    ///
    /// # Errors
    ///
    /// The [`HireRefusal`] the flight answers; a flight that never started
    /// has no bar ([`HireRefusal::NoBar`]).
    pub fn hire(&mut self, ship: ShipId) -> Result<Hired, HireRefusal> {
        match self.send(FlightCommand::Hire(ship)) {
            Answer::Hired(hired) => hired,
            _ => Err(HireRefusal::NoBar),
        }
    }

    /// The bar's hire screen closes ([`FlightCommand::CloseHire`],
    /// `nova_sim::Session::close_hire`).
    pub fn close_hire(&mut self) {
        self.send(FlightCommand::CloseHire);
    }

    /// Presses the jettison key: with Command held, jettisons the cargo,
    /// and with Option (Alt) too, all but the mission cargo
    /// ([`FlightCommand::Jettison`], `nova_sim::Session::jettison_cargo`),
    /// saying what the flight says went. Its key's
    /// gate is asked by the key, before the press reaches it
    /// (`Screen::input`'s filter).
    pub(super) fn jettison(&mut self) {
        if !self.held.contains(&Key::Command) {
            return;
        }
        let all = !self.held.contains(&Key::Alt);
        self.send(FlightCommand::Jettison { all });
        self.say_messages();
    }

    /// Whether the pilot flying has seen the intro screen, as the kept
    /// frame says
    /// ([`FlightFrame::seen_intro`](nova_sim::flight_frame::FlightFrame::seen_intro));
    /// false for a flight that never started.
    #[must_use]
    pub fn seen_intro(&self) -> bool {
        self.frame.seen_intro
    }

    /// Marks the pilot flying as having seen the intro screen
    /// ([`FlightCommand::MarkIntroSeen`], `nova_sim::Session::mark_intro_seen`).
    pub fn mark_intro_seen(&mut self) {
        self.send(FlightCommand::MarkIntroSeen);
    }
}

/// The outfitter's orders and the shipyard's purchase, over the
/// [`FlightSim`] port: their bar is staged as their scripts ran
/// ([`FlightView::stage_bar`]), and a purchase catches up with the change
/// of ship it tells.
impl<C: StatusBars, S: FlightSim> FlightView<C, S> {
    /// Buys a ship named `name` ([`FlightCommand::BuyShip`],
    /// `nova_sim::Session::buy_ship`), it and its hooks drawing on the
    /// flight's chance, and catches up with the change of ship it tells
    /// ([`SessionEvent::ShipChanged`](nova_sim::SessionEvent::ShipChanged)): the sprite
    /// sheet of the class the session then flies (the one bought, or the
    /// one its hooks changed it to) is read, once, so the new hull is
    /// drawn once it takes off; a refused purchase tells nothing and
    /// reads nothing, and a flight that never started has no shipyard.
    ///
    /// A purchase redraws the radar, the nav area and the target panel,
    /// as the original's does inside the purchase (`_DrawStatus`
    /// @0x5f0b1, after marking nav, target, weapons and cargo,
    /// `_statusRedraw[0]`, `[6]`, `[3]` and `[2]`, @0x5f03a-0x5f04a), so a ship whose default outfits carry IFF
    /// shows it at once, and the nav area shows what the ship's
    /// `OnPurchase` (@0x5f00d) wrote, a landed `M` or `N`'s selection,
    /// whatever its operators' order, as it runs first.
    ///
    /// The gauges (`[1]`, `[4]`) are not marked. A `C`, `E` or `H` in the old
    /// class's `OnRetire` (@0x5ecff) or the new one's `OnPurchase`
    /// (@0x5f00d) that puts the ship under another `ïntf` redraws the
    /// whole bar at that operator, so the gauges show as they stood
    /// there, before the purchase's fill (@0x5f051-0x5f06b). Then the
    /// purchase's own `_LoadIntfResource` (@0x5f0ac) loads the class's
    /// bar and redraws the gauges only when the ship's `ïntf` is still
    /// another than the one loaded (`hud::interface_for`), as its
    /// whole-bar `_RedrawStatus` (@0x739bd) runs only for an ID other than
    /// the one loaded (`_currentIntf`, @0x73663-0x73672); otherwise they
    /// wait for the shipyard to close ([`FlightView::refresh_status`]).
    ///
    /// An `S` in either hook shows its mission's texts inside the
    /// purchase (`_EvalSetExp` @0x153ff → `_DoMissionAccept` @0x15407), so
    /// the hooks' redraws are ordered against them
    /// ([`FlightView::stage_bar`]): those before the first text redraw at
    /// once, each text shows over the last bar drawn and its close
    /// redraws as the text noted it, then at the hooks' operators after
    /// it; and the purchase's own load and draw come only after the last
    /// text has closed ([`Then::Purchase`]). So under the text the old
    /// class's bar shows, with the gauges from before the purchase.
    ///
    /// Before all that, the whole bar is redrawn as the name prompt
    /// closes on OK (`_RefreshSAT` @0x5eb73, right after
    /// `_EVTextInputDialog` returns @0x5eb6c and before the purchase
    /// @0x5ebc0), whatever the purchase then does. The app checks every
    /// refusal before the prompt opens ([`FlightView::ship_naming`]), so
    /// no refusal follows it there.
    pub fn buy_ship(&mut self, ship: ShipId, name: &str) -> Result<ShipPurchase, ShipRefusal>
    where
        C: SystemCatalog + ShipSprites + CombatLooks,
    {
        self.redraw_status();
        let before = self.interface_now();
        let command = FlightCommand::BuyShip {
            ship,
            name: name.to_owned(),
        };
        let bought = match self.send(command) {
            Answer::ShipBought(bought) => bought?,
            _ => return Err(ShipRefusal::NoShipyard),
        };
        let after = self.interface_now();
        let (changes, loaded) = self.interface_changes(before.as_ref());
        self.stage_bar(
            changes,
            Then::Purchase {
                reserves: after != loaded,
            },
        );
        self.catch_up();
        Ok(bought)
    }

    /// Buys or sells one of an outfit ([`FlightView::outfit_counted`] of
    /// 1), it and its hook drawing on the flight's chance; a flight that
    /// never started has no outfitter. The bar is redrawn after it
    /// (`FlightView::after_outfit_order`).
    ///
    /// # Errors
    ///
    /// The [`OutfitRefusal`] of its pass.
    pub fn outfit(&mut self, order: OutfitOrder) -> Result<(), OutfitRefusal> {
        self.outfit_counted(order, 1).map(|_| ())
    }

    /// Buys or sells up to `count` of an outfit
    /// ([`FlightCommand::OutfitCounted`],
    /// `nova_sim::Session::outfit_counted_run`), on the flight's chance,
    /// giving how many went through; a flight that never started has no
    /// outfitter. The bar is redrawn after it
    /// (`FlightView::after_outfit_order`).
    ///
    /// # Errors
    ///
    /// The [`OutfitRefusal`] of its first pass.
    pub fn outfit_counted(&mut self, order: OutfitOrder, count: u32) -> Result<u32, OutfitRefusal> {
        let before = self.interface_now();
        let run = match self.send(FlightCommand::OutfitCounted { order, count }) {
            Answer::Outfitted(run) => run,
            _ => no_outfitter(),
        };
        self.after_outfit_order(before.as_ref(), run.button_passed);
        run.run.counted()
    }

    /// Buys or sells up to `count` of an outfit ([`FlightCommand::Outfit`],
    /// `nova_sim::Session::outfit_order`), on the flight's chance, giving
    /// the whole run and its alert; a flight that never started has no
    /// outfitter, refused at the first pass with no alert. The bar is
    /// redrawn after it (`FlightView::after_outfit_order`).
    pub fn outfit_order(&mut self, order: OutfitOrder, count: u32) -> OutfitRun {
        let before = self.interface_now();
        let run = match self.send(FlightCommand::Outfit { order, count }) {
            Answer::Outfitted(run) => run,
            _ => no_outfitter(),
        };
        self.after_outfit_order(before.as_ref(), run.button_passed);
        run
    }

    /// Redraws the bar after an outfitter order, given the `ïntf` the
    /// ship flew under before it ([`FlightView::interface_now`]) and
    /// whether it passed its button's own check (`passed`, the run's
    /// [`OutfitRun::button_passed`]).
    ///
    /// The order's `OnPurchase` (`_EvalMissionBitSetString` @0x5c2cb) or
    /// `OnSell` (@0x5cff6) runs inside it. When its `C`, `E` or `H`
    /// (`_EvalSetExp` @0x15493-0x156c7) puts the ship under another
    /// `ïntf`, `_LoadIntfResource` (@0x156c7) redraws the whole bar at
    /// once (`_RedrawStatus(1)` @0x739bd), held gauges, nav and target
    /// included ([`FlightView::redraw_status`]). Otherwise only the radar
    /// is redrawn ([`FlightView::retake_radar`]). A refused order changes
    /// no ship, so it redraws only the radar. The redraw runs inside the
    /// order's call, before any frame is drawn.
    ///
    /// The redraw is at the `C`, `E` or `H` itself, as the original's is
    /// ([`FlightView::stage_bar`]): the nav and gauges
    /// show as they stood at the last operator that changed the `ïntf`,
    /// so an `M` or `N` after it in the same script (@0x156c7 → @0x739bd,
    /// then @0x156f2-0x15705) shows only at the port's next nav redraw,
    /// and one before it shows at once. An `ïntf` changed and changed
    /// back within one order still redraws, though the one after the
    /// order is the one before it.
    ///
    /// An `S` in the hook shows its mission's texts inside the order
    /// (`_EvalSetExp` @0x153ff → `_DoMissionAccept` @0x15407, each text
    /// closing with `_RefreshSAT` @0xa1f13), so the hook's redraws are
    /// ordered against them as every docked script's are
    /// ([`FlightView::stage_bar`]): the `C`, `E` and `H` before the first
    /// text redraw at once, each text shows over the last bar drawn and
    /// its close redraws the bar as the text noted it, then at the
    /// operators after it; and the order's own draw (@0x5c5a1, @0x5d328,
    /// after `_SystemInfoToShipStats` and `_SetupPortAvailableItems`)
    /// comes only after the last text has closed ([`Then::Outfit`]). That
    /// text's close has drawn the nav, letting go of the shipyard's mark,
    /// so the order's draw then redraws only the radar. No stock outfit
    /// reaches it: `oütf` 363's `S731` starts a mission with
    /// no `BriefText`.
    ///
    /// The buy's and the sell's own `_DrawStatus` (@0x5c5a1, @0x5d328)
    /// draw the nav too when the shipyard's restore marked it
    /// ([`FlightView::close_port_dialog`]), but only for an order that
    /// went through: a buy `_CanBuyOutfitItem` refuses (@0x5c087,
    /// @0x5c09f-0x5c0a2) jumps past the buy's draw to @0x5c5ca, and a
    /// sell with nothing to sell (the flag at 0xded1c, @0x5c5d5-0x5c5dd)
    /// past the sell's to @0x5d351. A run stopped after its first pass
    /// still reaches them, and so does a Cancel's count of 0 once the
    /// button's check passed, as the loop then runs no pass and falls
    /// through to the draw (`_DoQuantityDialog` @0x5c1aa, @0x5c2fc-0x5c2ff).
    fn after_outfit_order(&mut self, before: Option<&Result<i16, String>>, passed: bool) {
        let (changes, _) = self.interface_changes(before);
        self.stage_bar(changes, Then::Outfit { passed });
    }

    /// Jettisons everything, mission cargo included, as the player info's
    /// Jettison does (`_JettisonCargo(1)` @0x5a66d,
    /// [`FlightCommand::Jettison`]), saying what the flight says went.
    /// Docked, an `OnFailure` `C`, `E` or `H` of a
    /// mission it fails (`_QuickMissionFailure` @0x99d83) that changes the
    /// `ïntf` redraws the whole bar at its operator; the player info's
    /// close redraws after it (@0x5a725). No gate guards it.
    pub fn jettison_all(&mut self) {
        self.run_docked(Then::Nothing, |view| {
            view.send(FlightCommand::Jettison { all: true });
        });
        self.say_messages();
    }

    /// Assigns the ship captured ([`FlightCommand::Assign`],
    /// `nova_sim::Session::assign`), and says what it did, as
    /// [`FlightView::assigned`] does; "Use As My Ship" says nothing yet,
    /// the ship awaiting its name ([`Assigned::Naming`],
    /// [`FlightView::capture_naming`], [`FlightView::name_capture`]).
    /// `None` when no capture awaits its assignment.
    pub fn assign(&mut self, choice: Assignment) -> Option<Assigned>
    where
        C: SystemCatalog + ShipSprites + CombatLooks,
    {
        let Answer::Assigned(assigned) = self.send(FlightCommand::Assign(choice)) else {
            return None;
        };
        if assigned == Assigned::Naming {
            return Some(assigned);
        }
        self.assigned(assigned);
        Some(assigned)
    }

    /// The prompt for renaming the ship captured, its words read from the
    /// flight's catalog and its default name drawn afresh on the flight's
    /// chance ([`ScreenAsk::CaptureNaming`]), asked once as the rename
    /// opens: `None` when no name is awaited, or the flight has nothing to
    /// answer.
    pub fn capture_naming(&mut self) -> Option<CaptureNaming> {
        match self.sim.screen(ScreenAsk::CaptureNaming) {
            ScreenAnswer::CaptureNaming(naming) => naming,
            _ => None,
        }
    }

    /// Answers the rename of the ship captured with the name `typed`, or
    /// none for Cancel ([`FlightCommand::NameCapture`],
    /// `nova_sim::Session::name_capture`), and says what it did, as
    /// [`FlightView::assigned`] does. `None` when no name is awaited.
    pub fn name_capture(&mut self, typed: Option<String>) -> Option<Assigned>
    where
        C: SystemCatalog + ShipSprites + CombatLooks,
    {
        let Answer::Assigned(assigned) = self.send(FlightCommand::NameCapture(typed)) else {
            return None;
        };
        self.assigned(assigned);
        Some(assigned)
    }

    /// Says what an assignment or a rename did: after a takeover the new
    /// ship is drawn where it is, not on its way from the old ship, and
    /// the change of ship the flight tells is caught up with
    /// ([`SessionEvent::ShipChanged`](nova_sim::SessionEvent::ShipChanged)),
    /// reading the new ship's sprite sheet, once.
    fn assigned(&mut self, assigned: Assigned)
    where
        C: SystemCatalog + ShipSprites + CombatLooks,
    {
        if assigned == Assigned::MyShip {
            // Where the new ship is, in the frame the assignment leaves,
            // before the catch-up that follows it.
            self.previous = self.player_place();
            self.alpha = 0.0;
        }
        self.catch_up();
        self.say_for(assigned_message(assigned), assigned_ticks(assigned));
    }
}
