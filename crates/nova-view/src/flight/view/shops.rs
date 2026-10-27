//! [`FlightView`]'s spaceport shops over the [`FlightSim`] port: the
//! exchange, the recharge, the outfitter and the shipyard. Each sends its
//! command
//! ([`FlightCommand`]) and hands back the flight's answer, a flight that
//! refuses it (one that never started) answering as today's failed
//! session did; what a screen shows is read through
//! [`PortSource`](crate::flight::sim_port::PortSource). The bar's
//! redraws around each command stay the view's.

use nova_sim::flight_frame::{Answer, FlightCommand};
use nova_sim::{
    Market, Order, OutfitOrder, OutfitRefusal, OutfitRun, Outfitter, RechargeRefusal, ShipId,
    ShipNaming, ShipPurchase, ShipRefusal, Shipyard, TradeRefusal,
};

use crate::flight::catalog::{CombatLooks, ShipSprites, StatusBars};
use crate::flight::sim_port::FlightSim;
use crate::system::catalog::SystemCatalog;

use super::{FlightView, Then, no_outfitter};

impl<C, S: FlightSim> FlightView<C, S> {
    /// The exchange of the stellar landed on
    /// ([`PortSource::market`](crate::flight::sim_port::PortSource::market)):
    /// none for a flight that never started.
    #[must_use]
    pub fn market(&self) -> Option<Market> {
        self.sim.market()
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

    /// Whether the spaceport's Recharge button is enabled
    /// ([`PortSource::recharge_enabled`](crate::flight::sim_port::PortSource::recharge_enabled)):
    /// false for a flight that never started.
    #[must_use]
    pub fn recharge_enabled(&self) -> bool {
        self.sim.recharge_enabled()
    }

    /// The outfitter of the stellar landed on, the day's rolls drawn on
    /// the flight's chance
    /// ([`PortSource::outfitter`](crate::flight::sim_port::PortSource::outfitter)):
    /// none for a flight that never started.
    pub fn outfitter(&mut self) -> Option<Outfitter> {
        self.sim.outfitter()
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

    /// The shipyard of the stellar landed on, the day's rolls drawn on the
    /// flight's chance
    /// ([`PortSource::shipyard`](crate::flight::sim_port::PortSource::shipyard)):
    /// none for a flight that never started.
    pub fn shipyard(&mut self) -> Option<Shipyard> {
        self.sim.shipyard()
    }

    /// The prompt for naming a ship of class `ship` before it is bought,
    /// its words read from the flight's catalog, on the flight's chance
    /// ([`PortSource::ship_naming`](crate::flight::sim_port::PortSource::ship_naming)).
    ///
    /// # Errors
    ///
    /// The [`ShipRefusal`] that applies; a flight that never started has
    /// no shipyard.
    pub fn ship_naming(&mut self, ship: ShipId) -> Result<ShipNaming, ShipRefusal> {
        self.sim.ship_naming(ship)
    }

    /// Declines to buy a ship of class `ship` ([`FlightCommand::DeclineShip`],
    /// `nova_sim::Session::decline_ship`). The whole bar is redrawn as the
    /// name prompt closes on Cancel (`_RefreshSAT` @0x5eb73), before it.
    pub fn decline_ship(&mut self, ship: ShipId) {
        self.redraw_status();
        self.send(FlightCommand::DeclineShip(ship));
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
}
