//! [`FlightView`]'s spaceport shops over the [`FlightSim`] port: the
//! exchange and the recharge. Each sends its command
//! ([`FlightCommand`]) and hands back the flight's answer, a flight that
//! refuses it (one that never started) answering as today's failed
//! session did; what a screen shows is read through
//! [`PortSource`](crate::flight::sim_port::PortSource). The bar's
//! redraws around each command stay the view's.

use nova_sim::flight_frame::{Answer, FlightCommand};
use nova_sim::{Market, Order, RechargeRefusal, TradeRefusal};

use crate::flight::sim_port::FlightSim;

use super::FlightView;

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
}
