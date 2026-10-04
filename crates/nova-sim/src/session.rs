//! A flight session: a [`Pilot`]'s ship in its system, flown one tick at a
//! time.
//!
//! A session flies a pilot ([`Session::fly`]), a new one from the first
//! `chär` ([`Session::start`]) or a saved one. The ship starts at rest at
//! the system's centre, facing up, or docked at the stellar the pilot last
//! landed on there. The pilot holds everything a save keeps (the ship,
//! system, date, course and reserves among them), and the session changes
//! it as the rules below say; the session itself keeps only what a flight
//! needs on top.
//!
//! Landing, taking off and each change made in the spaceport
//! ([`Session::transact`]) make a save due ([`Session::take_save_due`]):
//! whoever saves the pilot takes it after each input and saves then.
//! Arriving in a system explores it.
//!
//! The ship lands on a stellar of its system when the
//! [`landing`](crate::landing) rules allow it, which read the pilot's
//! legal record with the stellar's government, or with its system's on
//! the star map when the stellar has none: docked, it rests at the
//! stellar's centre and ticks move nothing until it takes off again, from
//! the same place. A landed ship cannot jump, nor a jumping one land.
//!
//! The player plots a course to a system on the star map, read once when
//! the session starts: the fewest jumps along the hyperlinks. A jump to
//! the next system on it begins when the [`hyperspace`](crate::hyperspace)
//! rules allow, and while it lasts ticks move nothing. When the jump is
//! over ([`Session::arrive`]) the ship is in the next system, at its edge,
//! with a jump's fuel used and the days its stats give a jump gone by, and
//! the rest of the course still ahead. In flight, fuel regenerates each tick at the rate the ship
//! and its outfits give.
//!
//! Everything about how the ship performs (its handling, the most shield,
//! armour and fuel it holds, its fuel regeneration and its cargo space)
//! comes from its [`ShipStats`]: its `shïp`'s fields, read when the session
//! starts (or from the record of a ship bought since), and the outfits the
//! pilot owns. A pilot from a save made
//! before outfits were kept owns its ship's default items.
//!
//! Each day that goes by steps the planetary events (see
//! [`market`](crate::market)), rolling whether each can start on the
//! [`Chance`] the caller passes to [`Session::arrive`].
//!
//! Landed at a trade center, the player trades on its exchange
//! ([`Session::market`], [`Session::trade`]), with the cargo space the ship
//! and its outfits give; the goods traded and the events that move their
//! prices are read when the session starts. A trade makes a save due; a
//! refused one changes nothing.
//!
//! Landed at an outfitter, the player buys and sells outfits one at a
//! time ([`Session::outfitter`], [`Session::outfit`]) as the
//! [`outfitter`] rules say, from the `oütf`s read when
//! the session starts. Each changes the stats at once: a gauge whose most
//! rises gains as much (a new tank comes full), and one whose most falls
//! keeps no more than it can hold. A change makes a save due; a refused
//! one changes nothing.
//!
//! Landed at a shipyard, the player buys a new ship
//! ([`Session::shipyard`], [`Session::buy_ship`]) as the [`shipyard`]
//! rules say, from the `shïp`s read when the session starts, trading in
//! the one flown. From then on the session flies the new ship: its fields
//! and default items come from its record, and the stats from them and
//! the outfits the purchase leaves. A purchase makes a save due; a refused
//! one changes nothing.
//!
//! Landed where fuel is sold, the player recharges
//! ([`Session::recharge`]), filling the tank as the
//! [`recharge`](crate::recharge) rules say. A refill makes a save due; a
//! refused one changes nothing.
//!
//! NPC traffic flies around the player (see [`traffic`](crate::traffic)):
//! a session starts with none, and its system is filled with its initial
//! population ([`Session::populate`]) on each arrival, replacing the last
//! system's, and on the first traffic tick in flight after the session
//! starts and after each take-off, as the original sets a system up on
//! arrival and on take-off. [`Session::tick_traffic`] advances it a tick,
//! NPCs deciding as a [`Behaviour`] says and rolling on the caller's
//! [`Chance`]; it stands still while the player is landed or jumping, as
//! the player's own ship does.
//!
//! As it goes the session emits [`SimSound`] events (thrust starting and
//! stopping, landing, taking off, a jump beginning and ending), which the
//! audio side drains with [`Session::take_sounds`]. A refused landing or
//! jump emits nothing.

use std::collections::BTreeMap;

use crate::ai::Behaviour;
use crate::catalog::{
    GovtId, LandingSite, OutfitId, OutfitRecord, PilotCatalog, ShipId, ShipRecord, StartError,
    StellarId, SystemId, TrafficCatalog,
};
use crate::chance::Chance;
use crate::date::GameDate;
use crate::flight::{Controls, ShipState, step};
use crate::fuel::regenerate;
use crate::geometry::Vec2;
use crate::handling::{Handling, ShipFields};
use crate::hyperspace::{JUMP_FUEL, JumpRefusal, RouteError, StarMap, arrival, check_jump};
use crate::landing::{LandingRefusal, check_landing};
use crate::market::{self, Goods, Market, Order, TradeRefusal};
use crate::outfitter::{self, OutfitOrder, OutfitRefusal, Outfitter, Shop, outfit_mods};
use crate::pilot::{self, Pilot};
use crate::recharge::{self, RechargeRefusal};
use crate::reserves::{Gauge, Reserves};
use crate::shipyard::{self, Quote, ShipPurchase, ShipRefusal, Shipyard, Yard};
use crate::sound::SimSound;
use crate::stats::ShipStats;
use crate::traffic::Traffic;
use crate::traffic::autopilot::Outcome;
use crate::traffic::npc::{Npc, NpcId};
use crate::traffic::table::SpawnTable;

/// The player's ship, flying in one system.
#[derive(Clone, Debug, PartialEq)]
pub struct Session {
    /// The pilot flying: everything a save keeps.
    pilot: Pilot,
    /// The ship's fields, read when the session starts.
    fields: ShipFields,
    /// The ship's default items, read when the session starts.
    defaults: BTreeMap<OutfitId, u16>,
    /// Every `oütf`, read when the session starts.
    outfits: Vec<OutfitRecord>,
    /// Every `shïp`, read when the session starts.
    ships: Vec<ShipRecord>,
    /// How the ship performs, with the outfits it carries.
    stats: ShipStats,
    player: ShipState,
    /// The system's stellars, read when the session starts.
    sites: Vec<LandingSite>,
    /// The stellar the ship is docked at, if it has landed.
    landed: Option<StellarId>,
    /// The star map, read when the session starts.
    star_map: StarMap,
    /// The system being jumped to, while a jump is under way.
    jumping: Option<SystemId>,
    /// The goods traded and the events that move their prices, read when
    /// the session starts.
    goods: Goods,
    /// Whether the ship is thrusting, as the last sounds told it.
    thrusting: bool,
    /// The sounds emitted since they were last taken.
    sounds: Vec<SimSound>,
    /// Whether the pilot has changed in a way that should be saved since
    /// this was last taken.
    save_due: bool,
    /// The NPCs in the system.
    traffic: Traffic,
    /// Whether the system is to be populated on the next traffic tick in
    /// flight: from the session's start, and after each take-off.
    traffic_due: bool,
}

impl Session {
    /// A new pilot's session, read from `catalog`: an unnamed
    /// [`Pilot::new`], flown.
    pub fn start(catalog: &impl PilotCatalog) -> Result<Self, StartError> {
        Self::fly(catalog, Pilot::new(catalog, "")?)
    }

    /// `pilot`'s session, its ship's fields and default items, the
    /// outfits, its system's stellars, the star map and the goods read
    /// from `catalog`, with the system marked explored. A pilot whose ship
    /// still carries its default items, from an old save, owns them now,
    /// and the reserves hold no more than the stats allow.
    ///
    /// A pilot last landed on a stellar of its system resumes docked there,
    /// silently, as the original resumes a pilot at its last planet.
    /// Otherwise (or when that stellar is no longer in the system, which
    /// the pilot then forgets) the ship starts at rest at the system's
    /// centre, facing up.
    ///
    /// # Errors
    ///
    /// When the ship cannot be read, or the system no longer exists.
    pub fn fly(catalog: &impl PilotCatalog, mut pilot: Pilot) -> Result<Self, StartError> {
        let ship = pilot.ship;
        let fields = catalog
            .ship_fields(ship)
            .map_err(|reason| StartError::Ship(ship, reason))?;
        if !catalog.system_exists(pilot.system) {
            return Err(StartError::NoSystem(pilot.system));
        }
        pilot.explore(pilot.system);
        let sites = catalog.landing_sites(pilot.system);
        let docked = pilot
            .stellar
            .and_then(|stellar| sites.iter().find(|site| site.id == stellar));
        let player = ShipState {
            position: docked.map_or(Vec2::ZERO, |site| site.position),
            ..ShipState::default()
        };
        let landed = docked.map(|site| site.id);
        pilot.stellar = landed;
        let defaults = pilot::default_outfits(catalog, ship);
        if pilot.default_outfits_pending {
            pilot.outfits.clone_from(&defaults);
            pilot.default_outfits_pending = false;
        }
        let mut session = Self {
            fields,
            defaults,
            outfits: catalog.outfits(),
            ships: catalog.ships(),
            // Refitted below, from the outfits the pilot owns.
            stats: ShipStats::default(),
            player,
            sites,
            landed,
            star_map: StarMap::new(catalog.star_map()),
            jumping: None,
            goods: Goods::read(catalog),
            thrusting: false,
            sounds: Vec::new(),
            save_due: false,
            traffic: Traffic::new(),
            traffic_due: true,
            pilot,
        };
        session.refit(false);
        Ok(session)
    }

    /// The ship's stats with the outfits the pilot owns.
    fn current_stats(&self) -> ShipStats {
        ShipStats::new(
            self.fields,
            &outfit_mods(&self.pilot.outfits, &self.outfits),
        )
    }

    /// Recomputes the stats from the outfits the pilot owns: each gauge
    /// holds up to the stats' most, keeping no more than that, and when
    /// `gain`, one whose most rose gains as much.
    fn refit(&mut self, gain: bool) {
        let stats = self.current_stats();
        let reserves = &mut self.pilot.reserves;
        for (gauge, max) in [
            (&mut reserves.shield, stats.shield),
            (&mut reserves.armor, stats.armor),
            (&mut reserves.fuel, stats.fuel),
        ] {
            refit(gauge, max, gain);
        }
        self.stats = stats;
    }

    /// Advances the session one tick under the player's `controls`, then
    /// regenerates fuel. A landed ship, or one jumping, does not move, and
    /// gains no fuel.
    pub fn tick(&mut self, controls: Controls) {
        if self.landed.is_none() && self.jumping.is_none() {
            if controls.thrust != self.thrusting {
                self.thrusting = controls.thrust;
                self.sounds.push(if controls.thrust {
                    SimSound::ThrustStarted
                } else {
                    SimSound::ThrustStopped
                });
            }
            step(&mut self.player, &self.stats.handling, controls);
            regenerate(&mut self.pilot.reserves.fuel, self.stats.fuel_regen);
        }
    }

    /// Fills the system with its initial NPC population, replacing any
    /// NPCs there, its traffic read from `catalog` and rolled on `chance`.
    pub fn populate(
        &mut self,
        catalog: &(impl TrafficCatalog + ?Sized),
        chance: &mut (impl Chance + ?Sized),
    ) {
        let system = self.pilot.system;
        let table = SpawnTable::resolve(
            catalog,
            system,
            self.star_map.govt(system),
            &self.ships,
            &self.outfits,
        );
        self.traffic.enter(table, chance);
        self.traffic_due = false;
    }

    /// Advances the NPC traffic one tick, NPCs deciding as `behaviour`
    /// says, rolling on `chance`. While the ship is landed or jumping, it
    /// stands still. The first tick in flight after the session starts,
    /// and after each take-off, instead populates the system
    /// ([`Session::populate`]) from `catalog`, as the original sets a
    /// system up on arrival and on take-off; its NPCs move from the next.
    pub fn tick_traffic(
        &mut self,
        catalog: &(impl TrafficCatalog + ?Sized),
        behaviour: &(impl Behaviour + ?Sized),
        chance: &mut (impl Chance + ?Sized),
    ) {
        if self.landed.is_none() && self.jumping.is_none() {
            if self.traffic_due {
                self.populate(catalog, chance);
            } else {
                self.traffic.tick(behaviour, &self.sites, chance);
            }
        }
    }

    /// The NPCs in the system, in the order they appeared.
    #[must_use]
    pub fn npcs(&self) -> &[Npc] {
        self.traffic.npcs()
    }

    /// The NPCs that left on the last traffic tick, and how.
    #[must_use]
    pub fn departed(&self) -> &[(NpcId, Outcome)] {
        self.traffic.departed()
    }

    /// The ship types the system's traffic can spawn, by ascending ID, so
    /// a view can read their sprites up front.
    #[must_use]
    pub fn traffic_ships(&self) -> Vec<ShipId> {
        self.traffic.ship_types()
    }

    /// Stops the thrust, if the ship was thrusting, as it lands or jumps.
    fn stop_thrust(&mut self) {
        if self.thrusting {
            self.thrusting = false;
            self.sounds.push(SimSound::ThrustStopped);
        }
    }

    /// Plots a course from the system the ship is in to `to`, replacing any
    /// course, and gives it. When there is no route the course is cleared.
    pub fn plot_course(&mut self, to: SystemId) -> Result<&[SystemId], RouteError> {
        match self.star_map.route(self.pilot.system, to) {
            Ok(route) => {
                self.pilot.course = route;
                Ok(&self.pilot.course)
            }
            Err(error) => {
                self.pilot.course.clear();
                Err(error)
            }
        }
    }

    /// The systems still to jump to, in order, ending at the destination;
    /// none when no course is plotted or the destination has been reached.
    #[must_use]
    pub fn course(&self) -> &[SystemId] {
        &self.pilot.course
    }

    /// Begins a jump to the next system on the course, if the ship has not
    /// landed and the [`hyperspace`](crate::hyperspace) rules allow it, and
    /// gives that system; otherwise the refusal says why. Until it arrives,
    /// ticks move nothing.
    pub fn begin_jump(&mut self) -> Result<SystemId, JumpRefusal> {
        if self.landed.is_some() {
            return Err(JumpRefusal::Landed);
        }
        let next = check_jump(
            &self.player,
            self.pilot.reserves.fuel.now,
            self.pilot.course.first().copied(),
            self.stats.jump_distance,
        )?;
        self.jumping = Some(next);
        self.stop_thrust();
        self.sounds.push(SimSound::JumpBegan);
        Ok(next)
    }

    /// The system being jumped to, while a jump is under way.
    #[must_use]
    pub fn jumping(&self) -> Option<SystemId> {
        self.jumping
    }

    /// Ends the jump under way, if any, and gives the system arrived in:
    /// the jump's fuel is used, the date advances by the days the stats give
    /// a jump, the system is taken off the course, and the ship is placed at
    /// its edge facing the system it came from (see [`arrival`]) with its
    /// reserves as they were. Each day
    /// steps the planetary events, rolled on `chance`. The new system's
    /// stellars are read from `catalog`, and it is populated with its
    /// traffic ([`Session::populate`]), the last system's gone. `None`, and
    /// nothing changes, when no jump is under way.
    pub fn arrive(
        &mut self,
        catalog: &(impl PilotCatalog + TrafficCatalog),
        chance: &mut (impl Chance + ?Sized),
    ) -> Option<SystemId> {
        let next = self.jumping.take()?;
        let pilot = &mut self.pilot;
        pilot.reserves.fuel.now -= JUMP_FUEL;
        for _ in 0..self.stats.jump_days {
            pilot.date = pilot.date.next_day();
            market::step_day(&self.goods, &mut pilot.events, chance);
        }
        if pilot.course.first() == Some(&next) {
            pilot.course.remove(0);
        }
        let map = |id| self.star_map.position(id).unwrap_or_default();
        self.player = arrival(map(pilot.system), map(next), &self.stats.handling);
        pilot.system = next;
        pilot.stellar = None;
        pilot.explore(next);
        self.sites = catalog.landing_sites(next);
        self.populate(catalog, chance);
        self.sounds.push(SimSound::Arrived);
        Some(next)
    }

    /// The sounds emitted since they were last taken, in order; taking
    /// them empties the list.
    pub fn take_sounds(&mut self) -> Vec<SimSound> {
        std::mem::take(&mut self.sounds)
    }

    /// The star map, as read when the session started.
    #[must_use]
    pub fn star_map(&self) -> &StarMap {
        &self.star_map
    }

    /// Today's date.
    #[must_use]
    pub fn date(&self) -> GameDate {
        self.pilot.date
    }

    /// The fuel the ship gains each tick in flight.
    #[must_use]
    pub fn fuel_regen_per_tick(&self) -> f32 {
        self.stats.fuel_regen
    }

    /// Lands the ship on the stellar it is over, if it is not jumping and
    /// the [`landing`](crate::landing) rules allow it, with the pilot's
    /// legal record with the stellar's government, or the system's on the
    /// star map when the stellar has none: it docks at the
    /// stellar's centre, at rest, its heading and reserves unchanged.
    /// Otherwise it flies on, and the refusal says why.
    pub fn land(&mut self) -> Result<StellarId, LandingRefusal> {
        if self.jumping.is_some() {
            return Err(LandingRefusal::Jumping);
        }
        let stellar = check_landing(
            &self.player,
            &self.sites,
            self.star_map.govt(self.pilot.system),
            |govt| self.pilot.legal_record(govt),
        )?;
        let site = self.sites.iter().find(|site| site.id == stellar);
        if let Some(site) = site {
            self.player.position = site.position;
        }
        let stellar_sound = site.and_then(|site| site.landing_sound);
        self.player.velocity = Vec2::ZERO;
        self.landed = Some(stellar);
        self.pilot.stellar = Some(stellar);
        self.save_due = true;
        self.stop_thrust();
        self.sounds.push(SimSound::Landed { stellar_sound });
        Ok(stellar)
    }

    /// Takes off from the stellar the ship is docked at, and gives it; the
    /// ship flies again from the stellar's centre, at rest, and the next
    /// traffic tick populates the system afresh
    /// ([`Session::tick_traffic`]). `None`, and nothing changes, when it
    /// has not landed.
    pub fn take_off(&mut self) -> Option<StellarId> {
        let stellar = self.landed.take()?;
        self.traffic_due = true;
        self.sounds.push(SimSound::TookOff);
        self.save_due = true;
        Some(stellar)
    }

    /// Changes the pilot with `change`, while the ship is landed (in the
    /// spaceport), and says whether it did: in flight nothing changes and
    /// `change` is not called. A save is due after a change. `change` must
    /// not change the ship class, which only [`Session::buy_ship`]
    /// changes, nor the outfits, which only [`Session::outfit`] and
    /// [`Session::buy_ship`] change, so the fields and the stats follow.
    pub fn transact(&mut self, change: impl FnOnce(&mut Pilot)) -> bool {
        if self.landed.is_none() {
            return false;
        }
        change(&mut self.pilot);
        self.save_due = true;
        true
    }

    /// The exchange of the stellar the ship is docked at, if it has landed
    /// at a trade center.
    #[must_use]
    pub fn market(&self) -> Option<Market> {
        let stellar = self.landed?;
        let site = self.sites.iter().find(|site| site.id == stellar)?;
        market::market(
            &self.goods,
            stellar,
            site.flags,
            &self.pilot,
            self.stats.capacity,
        )
    }

    /// Trades on the exchange as `order` asks, and gives the tons moved: a
    /// change made in the spaceport, so a save is due. When the ship is
    /// not landed at a trade center, or the order would move nothing,
    /// nothing changes and the refusal says why.
    pub fn trade(&mut self, order: Order) -> Result<u32, TradeRefusal> {
        let market = self.market().ok_or(TradeRefusal::NoMarket)?;
        let tons = market.tons(order)?;
        let price = market.row(order.good).map_or(0, |row| row.price);
        self.transact(|pilot| market::settle(pilot, order, tons, price));
        Ok(tons)
    }

    /// The outfitter of the stellar the ship is docked at, if it has
    /// landed at one.
    #[must_use]
    pub fn outfitter(&self) -> Option<Outfitter> {
        let stellar = self.landed?;
        let site = self.sites.iter().find(|site| site.id == stellar)?;
        Shop {
            records: &self.outfits,
            fields: self.fields,
            defaults: &self.defaults,
            site,
        }
        .outfitter(&self.pilot)
    }

    /// Buys or sells one outfit as `order` asks: a change made in the
    /// spaceport, so a save is due, and the stats change with it. When the
    /// ship is not landed at an outfitter, or the order is refused,
    /// nothing changes and the refusal says why.
    pub fn outfit(&mut self, order: OutfitOrder) -> Result<(), OutfitRefusal> {
        let outfitter = self.outfitter().ok_or(OutfitRefusal::NoOutfitter)?;
        outfitter.check(order)?;
        let price = outfitter.row(order.outfit).map_or(0, |row| row.price);
        let record = self
            .outfits
            .iter()
            .find(|record| record.id == order.outfit)
            .cloned()
            .ok_or(OutfitRefusal::NotListed)?;
        self.transact(|pilot| outfitter::settle(pilot, &record, order.direction, price));
        self.refit(true);
        Ok(())
    }

    /// The shipyard of the stellar the ship is docked at, if it has landed
    /// at one.
    #[must_use]
    pub fn shipyard(&self) -> Option<Shipyard> {
        let stellar = self.landed?;
        let site = self.sites.iter().find(|site| site.id == stellar)?;
        Yard {
            ships: &self.ships,
            outfits: &self.outfits,
            fields: self.fields,
            site,
        }
        .shipyard(&self.pilot)
    }

    /// Buys a ship of class `ship`, trading in the one flown, and gives
    /// what the purchase did: a change made in the spaceport, so a save is
    /// due, and the session flies the new ship from then on, its fields,
    /// default items and stats read from its record. When the ship is not
    /// landed at a shipyard, or the purchase is refused, nothing changes
    /// and the refusal says why.
    pub fn buy_ship(&mut self, ship: ShipId) -> Result<ShipPurchase, ShipRefusal> {
        let shipyard = self.shipyard().ok_or(ShipRefusal::NoShipyard)?;
        shipyard.check(ship)?;
        let record = self
            .ships
            .iter()
            .find(|record| record.id == ship)
            .cloned()
            .ok_or(ShipRefusal::NotListed)?;
        let quote = Quote {
            price: shipyard.row(ship).map_or(0, |row| row.price),
            trade_in: shipyard.trade_in,
        };
        let old_mass = self.fields.mass;
        let outfits = std::mem::take(&mut self.outfits);
        let mut bought = None;
        self.transact(|pilot| {
            bought = Some(shipyard::purchase(
                pilot, old_mass, &record, quote, &outfits,
            ));
        });
        self.outfits = outfits;
        self.fields = record.fields;
        self.defaults = pilot::tally(record.defaults.iter().copied());
        self.refit(false);
        bought.ok_or(ShipRefusal::NoShipyard)
    }

    /// Fills the tank at the stellar the ship is docked at, as the
    /// [`recharge`] rules say, and gives the price paid: a change made in
    /// the spaceport, so a save is due. When the ship is not landed where
    /// fuel is sold, or the refill is refused, nothing changes and the
    /// refusal says why.
    pub fn recharge(&mut self) -> Result<i64, RechargeRefusal> {
        let stellar = self.landed.ok_or(RechargeRefusal::NoFuel)?;
        let site = self.sites.iter().find(|site| site.id == stellar);
        if !site.is_some_and(|site| recharge::sells_fuel(site.flags)) {
            return Err(RechargeRefusal::NoFuel);
        }
        let price = recharge::quote(self.pilot.reserves.fuel, self.pilot.cash)?;
        self.transact(|pilot| recharge::settle(pilot, price));
        Ok(price)
    }

    /// The ship's cargo space, in tons.
    #[must_use]
    pub fn capacity(&self) -> u32 {
        self.stats.capacity
    }

    /// How the ship performs, with the outfits it carries.
    #[must_use]
    pub fn stats(&self) -> ShipStats {
        self.stats
    }

    /// Whether the pilot should be saved: it has landed, taken off or
    /// changed in the spaceport since this was last taken. Taking it clears
    /// it.
    pub fn take_save_due(&mut self) -> bool {
        std::mem::take(&mut self.save_due)
    }

    /// The stellar the ship is docked at, if it has landed.
    #[must_use]
    pub fn landed(&self) -> Option<StellarId> {
        self.landed
    }

    /// The player's ship as it flies.
    #[must_use]
    pub fn player(&self) -> &ShipState {
        &self.player
    }

    /// The pilot flying.
    #[must_use]
    pub fn pilot(&self) -> &Pilot {
        &self.pilot
    }

    /// The ship's shield, armour and fuel.
    #[must_use]
    pub fn reserves(&self) -> Reserves {
        self.pilot.reserves
    }

    /// The system the player is in.
    #[must_use]
    pub fn system(&self) -> SystemId {
        self.pilot.system
    }

    /// The player's ship class.
    #[must_use]
    pub fn ship(&self) -> ShipId {
        self.pilot.ship
    }

    /// How the player's ship flies.
    #[must_use]
    pub fn handling(&self) -> Handling {
        self.stats.handling
    }

    /// The government the player belongs to, if any. Always `None`: a new
    /// pilot belongs to no government. Nova grants membership later, through
    /// storyline play, and the first `chär` names none (its `Govt1-4` set
    /// starting legal records, not membership).
    #[must_use]
    pub fn government(&self) -> Option<GovtId> {
        None
    }
}

/// Sets `gauge` to hold up to `max`, keeping no more than that; when
/// `gain` and that is more than it held, it gains the difference.
fn refit(gauge: &mut Gauge, max: f32, gain: bool) {
    if gain {
        gauge.now += (max - gauge.max).max(0.0);
    }
    gauge.max = max;
    gauge.now = gauge.now.min(max);
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::{CharacterStart, LandingSite, SoundId, StellarId};
    use crate::catalog::{CommodityStrings, DisasterId, DisasterRecord, JunkId, JunkRecord};
    use crate::chance::NeverFires;
    use crate::flight::Turn;
    use crate::fuel::FUEL_SCOOP;
    use crate::geometry::Vec2;
    use crate::handling::ShipFields;
    use crate::hyperspace::{ARRIVAL_DISTANCE, DAYS_PER_JUMP, JumpRefusal, RouteError, StarMap};
    use crate::landing::LandingRefusal;
    use crate::landing::StellarFlags;
    use crate::market::{Direction, Good, Lot, Order, TradeRefusal};
    use crate::reserves::{Gauge, Reserves};
    use crate::stats::{HYPERSPACE_DAYS, HYPERSPACE_DISTANCE};
    use crate::testkit::{
        FAST, FakePilotCatalog, START, Scripted, catalog, edge_lander, fly_out, jump, jump_with,
        outfit, planet, starting,
    };

    #[test]
    fn a_session_flies_the_first_chärs_ship_with_its_handling() {
        let catalog = catalog();
        let session = Session::start(&catalog).expect("starts");
        assert_eq!(session.ship(), ShipId(128));
        assert_eq!(session.system(), SystemId(130));
        assert_eq!(
            session.handling(),
            crate::stats::ShipStats::new(FAST, &[]).handling
        );
        let asked = catalog.ships_asked.borrow();
        assert!(
            !asked.is_empty() && asked.iter().all(|&ship| ship == ShipId(128)),
            "{asked:?}"
        );
    }

    #[test]
    fn the_ship_starts_at_rest_at_the_centre_facing_up() {
        let session = Session::start(&catalog()).expect("starts");
        assert_eq!(
            *session.player(),
            ShipState {
                position: Vec2::ZERO,
                velocity: Vec2::ZERO,
                heading: 0.0,
            }
        );
        assert_eq!(
            session.reserves(),
            Reserves {
                shield: Gauge::full(30.0),
                armor: Gauge::full(45.0),
                fuel: Gauge::full(300.0),
            }
        );
    }

    #[test]
    fn a_new_pilot_belongs_to_no_government() {
        let session = Session::start(&catalog()).expect("starts");
        assert_eq!(session.government(), None);
    }

    #[test]
    fn a_tick_leaves_the_reserves_as_they_are() {
        let mut session = Session::start(&catalog()).expect("starts");
        let full = session.reserves();
        for _ in 0..30 {
            session.tick(Controls {
                thrust: true,
                ..Controls::default()
            });
        }
        assert_eq!(session.reserves(), full);
        assert_ne!(session.player().position, Vec2::ZERO);
    }

    #[test]
    fn the_first_system_that_exists_is_the_start() {
        let pick = |slots| Session::start(&starting(slots)).map(|s| s.system());
        assert_eq!(
            pick([None, Some(999), Some(131), Some(130)]),
            Ok(SystemId(131))
        );
        assert_eq!(pick([Some(131), Some(130), None, None]), Ok(SystemId(131)));
        assert_eq!(pick([None, None, None, Some(130)]), Ok(SystemId(130)));
    }

    #[test]
    fn no_system_that_exists_is_an_error_naming_the_slots() {
        let slots = [None, Some(999), None, Some(-5)];
        assert_eq!(
            Session::start(&starting(slots)),
            Err(StartError::NoStartingSystem(slots.map(|s| s.map(SystemId))))
        );
        assert_eq!(
            Session::start(&starting([None; 4])),
            Err(StartError::NoStartingSystem([None; 4]))
        );
    }

    #[test]
    fn a_missing_or_broken_chär_is_its_error() {
        for error in [
            StartError::NoCharacter,
            StartError::Character("chär 128: too short".to_owned()),
        ] {
            let catalog = FakePilotCatalog {
                character: Err(error.clone()),
                ..catalog()
            };
            assert_eq!(Session::start(&catalog), Err(error));
            assert_eq!(*catalog.ships_asked.borrow(), []);
        }
    }

    #[test]
    fn a_chär_without_a_ship_is_an_error() {
        let catalog = FakePilotCatalog {
            character: Ok(CharacterStart {
                ship: None,
                systems: [Some(SystemId(130)), None, None, None],
                start: START,
                ..CharacterStart::default()
            }),
            ..catalog()
        };
        assert_eq!(Session::start(&catalog), Err(StartError::NoShip));
    }

    #[test]
    fn an_unreadable_ship_is_an_error_with_its_reason() {
        let catalog = FakePilotCatalog {
            ships: Vec::new(),
            ..catalog()
        };
        assert_eq!(
            Session::start(&catalog),
            Err(StartError::Ship(ShipId(128), "no shïp 128".to_owned()))
        );
    }

    #[test]
    fn a_tick_steps_the_player_under_the_controls() {
        let mut session = Session::start(&catalog()).expect("starts");
        let controls = Controls {
            thrust: true,
            turn: Turn::Right,
            reverse: false,
        };
        let mut expected = *session.player();
        for _ in 0..5 {
            session.tick(controls);
            step(&mut expected, &session.handling(), controls);
        }
        assert_eq!(*session.player(), expected);
        assert_ne!(expected, ShipState::default());
    }

    // Landing.

    const THRUST: Controls = Controls {
        thrust: true,
        turn: Turn::None,
        reverse: false,
    };

    #[test]
    fn a_session_reads_its_systems_landing_sites_once() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(*catalog.sites_asked.borrow(), [SystemId(130)]);
        session.land().expect("lands");
        session.take_off();
        session.land().expect("lands again");
        assert_eq!(*catalog.sites_asked.borrow(), [SystemId(130)]);
        let other = starting([Some(131), None, None, None]);
        let mut session = Session::start(&other).expect("starts");
        assert_eq!(*other.sites_asked.borrow(), [SystemId(131)]);
        assert_eq!(session.land(), Ok(StellarId(140)));
    }

    #[test]
    fn a_new_pilots_record_is_clean() {
        let session = Session::start(&catalog()).expect("starts");
        for govt in [128, 140, 150] {
            assert_eq!(session.pilot().legal_record(GovtId(govt)), 0, "{govt}");
        }
        assert_eq!(session.landed(), None);
    }

    #[test]
    fn landing_docks_the_ship_at_the_stellar_at_rest() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.tick(Controls {
            turn: Turn::Right,
            ..THRUST
        });
        let flying = *session.player();
        assert_ne!(flying.velocity, Vec2::ZERO);
        assert_eq!(session.land(), Ok(StellarId(128)));
        assert_eq!(session.landed(), Some(StellarId(128)));
        assert_eq!(
            *session.player(),
            ShipState {
                position: Vec2::new(30.0, -40.0),
                velocity: Vec2::ZERO,
                ..flying
            }
        );
    }

    #[test]
    fn a_tick_while_landed_moves_nothing() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.land().expect("lands");
        let docked = *session.player();
        for _ in 0..10 {
            session.tick(Controls {
                turn: Turn::Left,
                ..THRUST
            });
        }
        assert_eq!(*session.player(), docked);
    }

    #[test]
    fn taking_off_leaves_the_ship_at_the_stellar_at_rest_and_it_flies_again() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.tick(Controls {
            turn: Turn::Right,
            ..THRUST
        });
        session.land().expect("lands");
        let docked = *session.player();
        assert_eq!(session.take_off(), Some(StellarId(128)));
        assert_eq!(session.landed(), None);
        assert_eq!(*session.player(), docked);
        assert_eq!(docked.position, Vec2::new(30.0, -40.0));
        assert_eq!(docked.velocity, Vec2::ZERO);
        session.tick(THRUST);
        let mut expected = docked;
        step(&mut expected, &session.handling(), THRUST);
        assert_eq!(*session.player(), expected);
        assert_ne!(expected, docked);
    }

    #[test]
    fn taking_off_without_landing_does_nothing() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.tick(THRUST);
        let flying = *session.player();
        assert_eq!(session.take_off(), None);
        assert_eq!(*session.player(), flying);
        assert_eq!(session.take_off(), None, "nor twice");
    }

    #[test]
    fn a_refused_landing_is_its_refusal_and_the_ship_flies_on() {
        let mut session = Session::start(&catalog()).expect("starts");
        for _ in 0..20 {
            session.tick(THRUST);
        }
        let flying = *session.player();
        let refusal = session.land();
        assert_eq!(
            refusal,
            crate::landing::check_landing(
                &flying,
                &[planet(128, 30.0, -40.0), planet(129, 2000.0, 0.0)],
                None,
                |_| 0
            )
        );
        assert!(refusal.is_err(), "{refusal:?}");
        assert_eq!(session.landed(), None);
        assert_eq!(*session.player(), flying);
        session.tick(Controls::default());
        assert_ne!(*session.player(), flying, "still flying");

        let empty = FakePilotCatalog {
            sites: Vec::new(),
            ..catalog()
        };
        let mut session = Session::start(&empty).expect("starts");
        assert_eq!(session.land(), Err(LandingRefusal::NoStellars));
    }

    /// Two governments, with a pilot's record of 10 and 9.
    const LAWFUL: GovtId = GovtId(140);
    const WANTED: GovtId = GovtId(150);

    /// [`catalog`] with system 130's planets, 128 and 129, each governed
    /// by `stellars` and needing a record of 10, and the system governed
    /// by `system`.
    fn governed(stellars: [Option<GovtId>; 2], system: Option<GovtId>) -> FakePilotCatalog {
        let needs = |site, govt| LandingSite {
            govt,
            min_status: 10,
            ..site
        };
        let mut catalog = catalog();
        catalog.sites[0].1 = vec![
            needs(planet(128, 30.0, -40.0), stellars[0]),
            needs(planet(129, 2000.0, 0.0), stellars[1]),
        ];
        catalog.star_map[0].govt = system;
        catalog
    }

    /// A new pilot of `catalog` whose record is 10 with [`LAWFUL`] and 9
    /// with [`WANTED`].
    fn on_record(catalog: &FakePilotCatalog) -> Pilot {
        let mut pilot = Pilot::new(catalog, "Law").expect("starts");
        pilot.set_legal_record(LAWFUL, 10);
        pilot.set_legal_record(WANTED, 9);
        pilot
    }

    /// Lands `pilot` on `stellar` in system 130 from right over it: docked
    /// there, taking off and landing again.
    fn land_at(
        catalog: &FakePilotCatalog,
        pilot: &Pilot,
        stellar: i16,
    ) -> Result<StellarId, LandingRefusal> {
        let mut pilot = pilot.clone();
        pilot.stellar = Some(StellarId(stellar));
        let mut session = Session::fly(catalog, pilot).expect("flies");
        assert_eq!(session.take_off(), Some(StellarId(stellar)));
        session.land()
    }

    fn denied(stellar: i16) -> Result<StellarId, LandingRefusal> {
        Err(LandingRefusal::Denied {
            stellar: StellarId(stellar),
            station: false,
            min_status: 10,
        })
    }

    #[test]
    fn landing_reads_the_pilots_record_with_each_stellars_government() {
        let catalog = governed([Some(LAWFUL), Some(WANTED)], None);
        let pilot = on_record(&catalog);
        assert_eq!(land_at(&catalog, &pilot, 128), Ok(StellarId(128)));
        assert_eq!(land_at(&catalog, &pilot, 129), denied(129));
    }

    #[test]
    fn landing_on_a_stellar_without_a_government_reads_its_systems_record() {
        let lawful = governed([None, None], Some(LAWFUL));
        let pilot = on_record(&lawful);
        assert_eq!(land_at(&lawful, &pilot, 129), Ok(StellarId(129)));
        let wanted = governed([None, Some(LAWFUL)], Some(WANTED));
        assert_eq!(land_at(&wanted, &pilot, 128), denied(128));
        assert_eq!(land_at(&wanted, &pilot, 129), Ok(StellarId(129)));
    }

    #[test]
    fn a_loaded_pilots_records_decide_landing() {
        let catalog = governed([Some(LAWFUL), Some(WANTED)], None);
        let loaded =
            crate::save::decode(&crate::save::encode(&on_record(&catalog))).expect("loads");
        assert_eq!(loaded.legal_record(LAWFUL), 10);
        assert_eq!(loaded.legal_record(WANTED), 9);
        assert_eq!(land_at(&catalog, &loaded, 128), Ok(StellarId(128)));
        assert_eq!(land_at(&catalog, &loaded, 129), denied(129));
    }

    // Hyperspace.

    fn ids(route: &[i16]) -> Vec<SystemId> {
        route.iter().copied().map(SystemId).collect()
    }

    fn dmy(session: &Session) -> (u8, u8, i32) {
        let date = session.date();
        (date.day(), date.month(), date.year())
    }

    #[test]
    fn a_session_reads_the_star_map_and_the_date_once_when_it_starts() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(dmy(&session), (23, 6, 1177));
        assert_eq!(*catalog.star_map_reads.borrow(), 1);
        session.plot_course(SystemId(132)).expect("a route");
        jump(&mut session, &catalog, 132);
        jump(&mut session, &catalog, 132);
        assert_eq!(*catalog.star_map_reads.borrow(), 1);
        assert_eq!(session.system(), SystemId(132));
        assert_eq!(session.course(), []);
        assert_eq!(session.jumping(), None);
    }

    #[test]
    fn the_star_map_is_the_catalogs() {
        let session = Session::start(&catalog()).expect("starts");
        assert_eq!(*session.star_map(), StarMap::new(catalog().star_map));
        assert_eq!(
            session.star_map().position(SystemId(132)),
            Some(Vec2::new(600.0, 600.0))
        );
    }

    #[test]
    fn plotting_a_course_keeps_the_route_and_a_failed_plot_clears_it() {
        let mut session = Session::start(&catalog()).expect("starts");
        assert_eq!(session.course(), []);
        assert_eq!(
            session.plot_course(SystemId(132)),
            Ok(&ids(&[131, 132])[..])
        );
        assert_eq!(session.course(), ids(&[131, 132]));
        assert_eq!(session.plot_course(SystemId(131)), Ok(&ids(&[131])[..]));
        assert_eq!(session.course(), ids(&[131]));
        assert_eq!(
            session.plot_course(SystemId(133)),
            Err(RouteError::Unreachable)
        );
        assert_eq!(session.course(), []);
        session.plot_course(SystemId(132)).expect("a route");
        assert_eq!(
            session.plot_course(SystemId(130)),
            Err(RouteError::AlreadyThere)
        );
        assert_eq!(session.course(), []);
        session.plot_course(SystemId(132)).expect("a route");
        assert_eq!(session.plot_course(SystemId(999)), Err(RouteError::Unknown));
        assert_eq!(session.course(), []);
    }

    #[test]
    fn a_jump_is_refused_without_a_destination_too_close_or_without_fuel() {
        let mut session = Session::start(&catalog()).expect("starts");
        assert_eq!(session.begin_jump(), Err(JumpRefusal::NoDestination));
        session.plot_course(SystemId(131)).expect("a route");
        assert_eq!(
            session.begin_jump(),
            Err(JumpRefusal::TooClose { distance: 0.0 })
        );
        assert_eq!(session.jumping(), None);

        let empty = FakePilotCatalog {
            ships: vec![(ShipId(128), Ok(ShipFields { fuel: 99, ..FAST }))],
            ..catalog()
        };
        let mut session = Session::start(&empty).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        assert_eq!(
            session.begin_jump(),
            Err(JumpRefusal::NoFuel { fuel: 99.0 })
        );
        assert_eq!(session.jumping(), None);
    }

    #[test]
    fn while_jumping_ticks_move_nothing() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        assert_eq!(session.begin_jump(), Ok(SystemId(131)));
        assert_eq!(session.jumping(), Some(SystemId(131)));
        let leaving = *session.player();
        for _ in 0..10 {
            session.tick(THRUST);
        }
        assert_eq!(*session.player(), leaving);
        assert_eq!(session.system(), SystemId(130), "not there yet");
    }

    #[test]
    fn arriving_takes_a_jumps_fuel_and_a_day() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(131)));
        assert_eq!(
            session.reserves().fuel,
            Gauge {
                now: 200.0,
                max: 300.0
            }
        );
        assert_eq!(dmy(&session), (24, 6, 1177));
        let shield = session.reserves().shield;
        assert_eq!(shield, Gauge::full(30.0), "the other reserves carry over");
    }

    #[test]
    fn arriving_puts_the_ship_at_the_edge_facing_the_system_it_came_from() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        fly_out(&mut session);
        session.begin_jump().expect("jumps");
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(131))
        );
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(session.course(), ids(&[132]));
        assert_eq!(session.jumping(), None);
        assert_eq!(
            *session.player(),
            crate::hyperspace::arrival(Vec2::ZERO, Vec2::new(600.0, 0.0), &session.handling())
        );
        assert_eq!(session.player().position, Vec2::new(-1000.0, 0.0));
    }

    #[test]
    fn arriving_reads_the_new_systems_stellars_and_landing_uses_them() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        jump(&mut session, &catalog, 131);
        assert_eq!(
            *catalog.sites_asked.borrow(),
            [SystemId(130), SystemId(131)]
        );
        // Planet 140 at 131's centre: drift there and land.
        let refused = session.land();
        assert!(
            matches!(refused, Err(LandingRefusal::TooFar { nearest, .. }) if nearest == StellarId(140)),
            "{refused:?}"
        );
    }

    #[test]
    fn arriving_when_not_jumping_does_nothing() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        let before = session.clone();
        assert_eq!(session.arrive(&catalog, &mut NeverFires), None);
        assert_eq!(session, before);
        assert_eq!(*catalog.sites_asked.borrow(), [SystemId(130)]);
    }

    #[test]
    fn a_second_jump_continues_the_route_to_the_destination() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        jump(&mut session, &catalog, 132);
        assert_eq!(session.system(), SystemId(131));
        session.tick(Controls::default());
        assert_eq!(
            session.begin_jump(),
            Err(JumpRefusal::TooClose { distance: 994.0 }),
            "drifting in from the edge"
        );
        assert_eq!(jump(&mut session, &catalog, 132), Some(SystemId(132)));
        assert_eq!(session.system(), SystemId(132));
        assert_eq!(session.course(), []);
        assert_eq!(session.reserves().fuel.now, 100.0);
        assert_eq!(dmy(&session), (25, 6, 1177));
        // From 131, north of 132 on screen: it arrives at the top edge.
        assert_eq!(session.player().position, Vec2::new(0.0, -1000.0));
        assert_eq!(session.begin_jump(), Err(JumpRefusal::NoDestination));
    }

    /// The catalog with ship 128 regenerating a unit of fuel every
    /// `regen` ticks.
    fn regenerating(regen: i16) -> FakePilotCatalog {
        FakePilotCatalog {
            ships: vec![(
                ShipId(128),
                Ok(ShipFields {
                    fuel_regen: regen,
                    ..FAST
                }),
            )],
            ..catalog()
        }
    }

    #[test]
    fn fuel_regenerates_each_tick_at_the_ships_rate_up_to_full() {
        let catalog = regenerating(2);
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(session.fuel_regen_per_tick(), 0.5);
        jump(&mut session, &catalog, 131);
        assert_eq!(session.reserves().fuel.now, 200.0);
        for _ in 0..10 {
            session.tick(Controls::default());
        }
        assert_eq!(session.reserves().fuel.now, 205.0);
        for _ in 0..1000 {
            session.tick(Controls::default());
        }
        assert_eq!(session.reserves().fuel, Gauge::full(300.0));
        let still = Session::start(&self::catalog()).expect("starts");
        assert_eq!(still.fuel_regen_per_tick(), 0.0);
    }

    #[test]
    fn the_ships_default_outfits_add_to_its_fuel_regeneration() {
        // A unit every 8 ticks from the ship, and two scoops each giving a
        // unit every 4 ticks; a mod of another type gives nothing.
        let catalog = FakePilotCatalog {
            defaults: vec![(ShipId(128), vec![(OutfitId(200), 2), (OutfitId(201), 1)])],
            outfits: vec![outfit(200, &[(FUEL_SCOOP, 4)]), outfit(201, &[(1, 1)])],
            ..regenerating(8)
        };
        let mut session = Session::start(&catalog).expect("starts");
        let read = (
            catalog.defaults_asked.borrow().clone(),
            *catalog.outfit_reads.borrow(),
        );
        assert!(read.0.iter().all(|&ship| ship == ShipId(128)), "{read:?}");
        assert_eq!(session.fuel_regen_per_tick(), 0.125 + 0.5);
        jump(&mut session, &catalog, 131);
        for _ in 0..8 {
            session.tick(Controls::default());
        }
        assert_eq!(session.reserves().fuel.now, 205.0);
        assert_eq!(
            (
                catalog.defaults_asked.borrow().clone(),
                *catalog.outfit_reads.borrow()
            ),
            read,
            "read only when it starts"
        );
    }

    /// A session landed on planet 140 at 131's edge, far enough out to
    /// jump, with fuel and 132 still to go.
    fn landed_at_the_edge(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        jump(&mut session, catalog, 132);
        assert_eq!(session.land(), Ok(StellarId(140)));
        session
    }

    #[test]
    fn a_jump_is_refused_while_landed() {
        let catalog = edge_lander();
        let mut session = landed_at_the_edge(&catalog);
        let docked = session.clone();
        assert_eq!(session.begin_jump(), Err(JumpRefusal::Landed));
        assert_eq!(session, docked, "nothing changes");
        session.take_off();
        assert_eq!(session.begin_jump(), Ok(SystemId(132)), "once off");
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(132))
        );
        assert_eq!(session.landed(), None);
    }

    #[test]
    fn landing_is_refused_while_jumping() {
        let catalog = edge_lander();
        let mut session = landed_at_the_edge(&catalog);
        session.take_off();
        session.begin_jump().expect("jumps from over planet 140");
        let jumping = session.clone();
        assert_eq!(session.land(), Err(LandingRefusal::Jumping));
        assert_eq!(session, jumping, "nothing changes");
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(132))
        );
        assert_eq!(session.landed(), None);
    }

    #[test]
    fn fuel_does_not_regenerate_while_landed_or_jumping() {
        let catalog = edge_lander();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        jump(&mut session, &catalog, 132);
        assert_eq!(session.reserves().fuel.now, 200.0);
        assert_eq!(session.land(), Ok(StellarId(140)));
        for _ in 0..10 {
            session.tick(Controls::default());
        }
        assert_eq!(session.reserves().fuel.now, 200.0);
        session.take_off();
        session.tick(Controls::default());
        assert_eq!(session.reserves().fuel.now, 201.0);

        session.begin_jump().expect("jumps from the edge");
        for _ in 0..10 {
            session.tick(Controls::default());
        }
        assert_eq!(session.reserves().fuel.now, 201.0);
    }

    // The pilot.

    #[test]
    fn landing_records_the_stellar_on_the_pilot_and_a_save_is_due() {
        let mut session = Session::start(&catalog()).expect("starts");
        assert!(!session.take_save_due(), "nothing to save yet");
        assert_eq!(session.pilot().stellar(), None);
        session.land().expect("lands");
        assert_eq!(session.pilot().stellar(), Some(StellarId(128)));
        assert!(session.take_save_due());
        assert!(!session.take_save_due(), "taking it clears it");
    }

    #[test]
    fn a_refused_landing_records_nothing_and_no_save_is_due() {
        let mut session = Session::start(&catalog()).expect("starts");
        for _ in 0..20 {
            session.tick(THRUST);
        }
        session.land().expect_err("refused");
        assert_eq!(session.pilot().stellar(), None);
        assert!(!session.take_save_due());
    }

    #[test]
    fn taking_off_keeps_the_stellar_and_a_save_is_due() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.land().expect("lands");
        session.take_save_due();
        session.take_off();
        assert_eq!(session.pilot().stellar(), Some(StellarId(128)));
        assert!(session.take_save_due());
        session.take_off();
        assert!(!session.take_save_due(), "not landed: nothing happens");
    }

    #[test]
    fn a_transaction_while_landed_changes_the_pilot_and_a_save_is_due() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.land().expect("lands");
        session.take_save_due();
        assert!(session.transact(|pilot| pilot.set_cash(500)));
        assert_eq!(session.pilot().cash(), 500);
        assert!(session.take_save_due());
    }

    #[test]
    fn a_transaction_in_flight_is_refused_and_changes_nothing() {
        let mut session = Session::start(&catalog()).expect("starts");
        let before = session.clone();
        let mut called = false;
        assert!(!session.transact(|pilot| {
            called = true;
            pilot.set_cash(500);
        }));
        assert!(!called);
        assert_eq!(session, before);
        assert!(!session.take_save_due());
    }

    #[test]
    fn arriving_explores_the_system_and_forgets_the_stellar() {
        let catalog = edge_lander();
        let mut session = landed_at_the_edge(&catalog);
        assert_eq!(session.pilot().stellar(), Some(StellarId(140)));
        assert_eq!(
            session.pilot().explored().collect::<Vec<_>>(),
            [SystemId(130), SystemId(131)]
        );
        session.take_off();
        session.take_save_due();
        session.begin_jump().expect("jumps from the edge");
        session.arrive(&catalog, &mut NeverFires).expect("arrives");
        let pilot = session.pilot();
        assert_eq!(pilot.system(), SystemId(132));
        assert_eq!(pilot.stellar(), None);
        assert_eq!(
            pilot.explored().collect::<Vec<_>>(),
            [SystemId(130), SystemId(131), SystemId(132)]
        );
        assert!(!session.take_save_due(), "a jump alone saves nothing");
    }

    /// A pilot landed on planet 128 in system 130, with 7 credits and a
    /// day gone by, taken from a session.
    fn landed_pilot(catalog: &FakePilotCatalog) -> Pilot {
        let mut session = Session::start(catalog).expect("starts");
        session.land().expect("lands");
        session.transact(|pilot| pilot.set_cash(7));
        session.pilot().clone()
    }

    #[test]
    fn flying_a_pilot_landed_at_a_stellar_resumes_docked_there() {
        let catalog = catalog();
        let pilot = landed_pilot(&catalog);
        let session = Session::fly(&catalog, pilot.clone()).expect("flies");
        assert_eq!(session.landed(), Some(StellarId(128)));
        assert_eq!(session.player().position, Vec2::new(30.0, -40.0));
        assert_eq!(session.player().velocity, Vec2::ZERO);
        assert_eq!(*session.pilot(), pilot);
        let mut session = session;
        assert_eq!(session.take_sounds(), [], "no landing sound");
        assert!(!session.take_save_due());
    }

    #[test]
    fn flying_a_pilot_whose_stellar_is_gone_starts_at_the_centre() {
        let pilot = landed_pilot(&catalog());
        let moved = FakePilotCatalog {
            sites: vec![(SystemId(130), vec![planet(129, 2000.0, 0.0)])],
            ..catalog()
        };
        let session = Session::fly(&moved, pilot).expect("flies");
        assert_eq!(session.landed(), None);
        assert_eq!(*session.player(), ShipState::default());
        assert_eq!(session.pilot().stellar(), None);
    }

    #[test]
    fn flying_a_pilot_in_a_system_that_no_longer_exists_is_an_error() {
        let pilot = landed_pilot(&catalog());
        let gone = FakePilotCatalog {
            systems: vec![SystemId(131)],
            ..catalog()
        };
        assert_eq!(
            Session::fly(&gone, pilot),
            Err(StartError::NoSystem(SystemId(130)))
        );
    }

    #[test]
    fn flying_a_pilot_explores_its_system() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        jump(&mut session, &catalog, 131);
        let mut pilot = session.pilot().clone();
        pilot.explored.clear();
        let session = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(
            session.pilot().explored().collect::<Vec<_>>(),
            [SystemId(131)]
        );
    }

    // Sounds.

    #[test]
    fn holding_thrust_starts_it_once_and_letting_go_stops_it_once() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.tick(Controls::default());
        assert_eq!(session.take_sounds(), []);
        for _ in 0..5 {
            session.tick(THRUST);
        }
        assert_eq!(session.take_sounds(), [SimSound::ThrustStarted]);
        for _ in 0..5 {
            session.tick(Controls {
                turn: Turn::Left,
                ..Controls::default()
            });
        }
        assert_eq!(session.take_sounds(), [SimSound::ThrustStopped]);
        session.tick(THRUST);
        session.tick(Controls::default());
        session.tick(THRUST);
        assert_eq!(
            session.take_sounds(),
            [
                SimSound::ThrustStarted,
                SimSound::ThrustStopped,
                SimSound::ThrustStarted
            ]
        );
    }

    #[test]
    fn taking_the_sounds_empties_them() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.tick(THRUST);
        assert_eq!(session.take_sounds(), [SimSound::ThrustStarted]);
        assert_eq!(session.take_sounds(), []);
    }

    /// The catalog with planet 128 playing `snd ` 10032 when landed on.
    fn sounding() -> FakePilotCatalog {
        FakePilotCatalog {
            sites: vec![(
                SystemId(130),
                vec![LandingSite {
                    landing_sound: Some(SoundId(10_032)),
                    ..planet(128, 30.0, -40.0)
                }],
            )],
            ..catalog()
        }
    }

    #[test]
    fn landing_emits_landed_with_the_stellars_sound() {
        let mut session = Session::start(&sounding()).expect("starts");
        session.land().expect("lands");
        assert_eq!(
            session.take_sounds(),
            [SimSound::Landed {
                stellar_sound: Some(SoundId(10_032))
            }]
        );
        let mut silent = Session::start(&catalog()).expect("starts");
        silent.land().expect("lands");
        assert_eq!(
            silent.take_sounds(),
            [SimSound::Landed {
                stellar_sound: None
            }]
        );
    }

    #[test]
    fn landing_while_thrusting_stops_the_thrust_first() {
        let mut session = Session::start(&sounding()).expect("starts");
        session.tick(Controls {
            turn: Turn::Right,
            ..THRUST
        });
        assert_eq!(session.take_sounds(), [SimSound::ThrustStarted]);
        session.land().expect("lands");
        assert_eq!(
            session.take_sounds(),
            [
                SimSound::ThrustStopped,
                SimSound::Landed {
                    stellar_sound: Some(SoundId(10_032))
                }
            ]
        );
        session.tick(THRUST);
        assert_eq!(session.take_sounds(), [], "docked, nothing thrusts");
        session.take_off();
        assert_eq!(session.take_sounds(), [SimSound::TookOff]);
        session.tick(THRUST);
        assert_eq!(
            session.take_sounds(),
            [SimSound::ThrustStarted],
            "off again"
        );
    }

    #[test]
    fn taking_off_emits_took_off_and_not_taking_off_emits_nothing() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.take_off();
        assert_eq!(session.take_sounds(), [], "not landed");
        session.land().expect("lands");
        session.take_sounds();
        session.take_off();
        assert_eq!(session.take_sounds(), [SimSound::TookOff]);
        session.take_off();
        assert_eq!(session.take_sounds(), [], "nor twice");
    }

    #[test]
    fn a_refused_landing_emits_nothing() {
        let mut session = Session::start(&catalog()).expect("starts");
        for _ in 0..20 {
            session.tick(THRUST);
        }
        session.take_sounds();
        session.land().expect_err("refused");
        assert_eq!(session.take_sounds(), []);
        let mut jumping = Session::start(&edge_lander()).expect("starts");
        jumping.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut jumping);
        jumping.begin_jump().expect("jumps");
        jumping.take_sounds();
        assert_eq!(jumping.land(), Err(LandingRefusal::Jumping));
        assert_eq!(jumping.take_sounds(), []);
    }

    #[test]
    fn a_jump_emits_jump_began_then_arrived_stopping_the_thrust_first() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        session.tick(THRUST);
        let thrusting = session.take_sounds();
        assert_eq!(thrusting.last(), Some(&SimSound::ThrustStarted));
        session.begin_jump().expect("jumps");
        assert_eq!(
            session.take_sounds(),
            [SimSound::ThrustStopped, SimSound::JumpBegan]
        );
        session.tick(THRUST);
        assert_eq!(session.take_sounds(), [], "jumping, nothing thrusts");
        session.arrive(&catalog, &mut NeverFires).expect("arrives");
        assert_eq!(session.take_sounds(), [SimSound::Arrived]);
        session.arrive(&catalog, &mut NeverFires);
        assert_eq!(session.take_sounds(), [], "no jump under way");
    }

    #[test]
    fn a_jump_without_thrust_emits_only_jump_began() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        session.tick(Controls::default());
        session.take_sounds();
        session.begin_jump().expect("jumps");
        assert_eq!(session.take_sounds(), [SimSound::JumpBegan]);
    }

    #[test]
    fn a_refused_jump_emits_nothing() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.tick(THRUST);
        session.take_sounds();
        assert_eq!(session.begin_jump(), Err(JumpRefusal::NoDestination));
        assert_eq!(session.take_sounds(), []);
        let catalog = edge_lander();
        let mut landed = landed_at_the_edge(&catalog);
        landed.take_sounds();
        assert_eq!(landed.begin_jump(), Err(JumpRefusal::Landed));
        assert_eq!(landed.take_sounds(), []);
    }

    // The exchange.

    /// Food at 75 and metal at 200, as `STR#` 4000 and 4004 have them.
    fn food_and_metal() -> CommodityStrings {
        CommodityStrings {
            names: ["Food", "Industrial", "Medical", "Luxury", "Metal"]
                .map(str::to_owned)
                .to_vec(),
            base_prices: ["75", "350", "750", "900", "200"]
                .map(str::to_owned)
                .to_vec(),
        }
    }

    /// Food at medium and metal at low.
    const TRADES: u32 = StellarFlags::CAN_LAND | StellarFlags::TRADE_CENTER | 2 << 28 | 1 << 12;

    /// The catalog with planet 128 at the centre, which the ship starts
    /// over, a trade center trading food (75) and metal (160), and the
    /// first `chär` holding 1000 credits; ship 128 holds 20 tons.
    fn exchange() -> FakePilotCatalog {
        FakePilotCatalog {
            character: Ok(CharacterStart {
                ship: Some(ShipId(128)),
                systems: [Some(SystemId(130)), None, None, None],
                start: START,
                cash: 1000,
                ..CharacterStart::default()
            }),
            sites: vec![(
                SystemId(130),
                vec![LandingSite {
                    flags: TRADES,
                    ..planet(128, 0.0, 0.0)
                }],
            )],
            commodities: food_and_metal(),
            ..catalog()
        }
    }

    fn order(good: Good, direction: Direction, lot: Lot) -> Order {
        Order {
            good,
            direction,
            lot,
        }
    }

    const FOOD: Good = Good::Commodity(0);
    const METAL: Good = Good::Commodity(4);

    #[test]
    fn a_session_reads_the_goods_once_when_it_starts() {
        let catalog = exchange();
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(*catalog.goods_reads.borrow(), 3);
        session.land().expect("lands");
        assert!(session.market().is_some());
        session.take_off();
        jump(&mut session, &catalog, 131);
        assert_eq!(*catalog.goods_reads.borrow(), 3);
    }

    #[test]
    fn the_cargo_space_is_the_ships_holds_and_its_cargo_pods() {
        let session = Session::start(&exchange()).expect("starts");
        assert_eq!(session.capacity(), 20);
        let catalog = FakePilotCatalog {
            ships: vec![(ShipId(128), Ok(ShipFields { holds: -3, ..FAST }))],
            defaults: vec![(ShipId(128), vec![(OutfitId(200), 2)])],
            outfits: vec![outfit(200, &[(crate::market::MORE_CARGO, 5)])],
            ..exchange()
        };
        let session = Session::start(&catalog).expect("starts");
        assert_eq!(session.capacity(), 13);
    }

    #[test]
    fn there_is_an_exchange_only_while_landed_at_a_trade_center() {
        let catalog = exchange();
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(session.market(), None, "in flight");
        session.land().expect("lands");
        let market = session.market().expect("an exchange");
        let prices: Vec<_> = market.rows.iter().map(|r| (r.good, r.price)).collect();
        assert_eq!(prices, [(FOOD, 75), (METAL, 160)]);
        assert_eq!((market.cash, market.capacity, market.free), (1000, 20, 20));
        session.take_off();
        assert_eq!(session.market(), None, "taken off");
        let mut plain = Session::start(&self::catalog()).expect("starts");
        plain.land().expect("lands");
        assert_eq!(plain.market(), None, "no trade center");
    }

    #[test]
    fn buying_pays_loads_and_makes_a_save_due() {
        let mut session = Session::start(&exchange()).expect("starts");
        session.land().expect("lands");
        session.take_save_due();
        assert_eq!(session.trade(order(FOOD, Direction::Buy, Lot::One)), Ok(1));
        assert_eq!(session.pilot().cash(), 925);
        assert_eq!(session.pilot().held(FOOD), 1);
        assert!(session.take_save_due());
        assert_eq!(session.trade(order(METAL, Direction::Buy, Lot::Max)), Ok(5));
        assert_eq!(session.pilot().cash(), 125, "cash ran out first");
        assert_eq!(session.market().expect("an exchange").free, 14);
    }

    #[test]
    fn buying_stops_when_the_hold_is_full() {
        let catalog = FakePilotCatalog {
            ships: vec![(ShipId(128), Ok(ShipFields { holds: 3, ..FAST }))],
            ..exchange()
        };
        let mut session = Session::start(&catalog).expect("starts");
        session.land().expect("lands");
        assert_eq!(session.trade(order(FOOD, Direction::Buy, Lot::Max)), Ok(3));
        session.take_save_due();
        assert_eq!(
            session.trade(order(FOOD, Direction::Buy, Lot::One)),
            Err(TradeRefusal::NoSpace)
        );
        assert_eq!(session.pilot().cash(), 775);
        assert!(!session.take_save_due(), "nothing changed");
    }

    #[test]
    fn selling_pays_the_local_price() {
        let mut session = Session::start(&exchange()).expect("starts");
        session.land().expect("lands");
        assert_eq!(session.trade(order(METAL, Direction::Buy, Lot::Max)), Ok(6));
        assert_eq!(session.pilot().cash(), 40);
        session.take_save_due();
        assert_eq!(
            session.trade(order(METAL, Direction::Sell, Lot::One)),
            Ok(1)
        );
        assert_eq!(session.pilot().cash(), 40 + 160);
        assert!(session.take_save_due());
        assert_eq!(
            session.trade(order(METAL, Direction::Sell, Lot::Max)),
            Ok(5)
        );
        assert_eq!(session.pilot().cash(), 1000);
        assert_eq!(session.pilot().held(METAL), 0);
    }

    #[test]
    fn a_refused_trade_changes_nothing_and_makes_no_save_due() {
        let mut session = Session::start(&exchange()).expect("starts");
        let flying = session.clone();
        assert_eq!(
            session.trade(order(FOOD, Direction::Buy, Lot::One)),
            Err(TradeRefusal::NoMarket)
        );
        assert_eq!(session, flying);
        session.land().expect("lands");
        session.take_save_due();
        let landed = session.clone();
        for refused in [
            order(FOOD, Direction::Sell, Lot::One),
            order(Good::Commodity(1), Direction::Buy, Lot::One),
        ] {
            assert!(session.trade(refused).is_err(), "{refused:?}");
        }
        assert_eq!(session, landed);
        assert!(!session.take_save_due());
    }

    #[test]
    fn junk_is_bought_and_sold_like_a_commodity() {
        let opals = JunkRecord {
            id: JunkId(146),
            name: "Opals".to_owned(),
            base_price: 100,
            sold_at: vec![StellarId(128)],
            bought_at: vec![StellarId(140)],
            buy_on: String::new(),
            sell_on: String::new(),
        };
        let catalog = edge_lander();
        let catalog = FakePilotCatalog {
            character: exchange().character,
            sites: vec![
                (
                    SystemId(130),
                    vec![LandingSite {
                        flags: TRADES,
                        ..planet(128, 0.0, 0.0)
                    }],
                ),
                (
                    SystemId(131),
                    vec![LandingSite {
                        flags: TRADES,
                        ..planet(140, -1000.0, 0.0)
                    }],
                ),
            ],
            junk: vec![opals],
            ..catalog
        };
        let opals = Good::Junk(JunkId(146));
        let mut session = Session::start(&catalog).expect("starts");
        session.land().expect("lands");
        assert_eq!(
            session.trade(order(opals, Direction::Buy, Lot::Max)),
            Ok(12)
        );
        assert_eq!(session.pilot().cash(), 1000 - 12 * 80);
        assert_eq!(
            session.trade(order(opals, Direction::Sell, Lot::One)),
            Err(TradeRefusal::NotTraded),
            "not bought here"
        );
        session.take_off();
        jump(&mut session, &catalog, 131);
        assert_eq!(session.land(), Ok(StellarId(140)));
        assert_eq!(
            session.trade(order(opals, Direction::Sell, Lot::Max)),
            Ok(12)
        );
        assert_eq!(session.pilot().cash(), 1000 - 12 * 80 + 12 * 125);
    }

    /// A food surplus at planet 140 (-15, 30 days, 35 % a day), and planet
    /// 140 a trade center trading food at 75.
    fn surplus() -> FakePilotCatalog {
        let landers = edge_lander();
        FakePilotCatalog {
            sites: vec![(
                SystemId(131),
                vec![LandingSite {
                    flags: TRADES,
                    ..planet(140, -1000.0, 0.0)
                }],
            )],
            commodities: food_and_metal(),
            disasters: vec![DisasterRecord {
                id: DisasterId(128),
                name: "An enormous food surplus".to_owned(),
                stellar: 140,
                commodity: 0,
                price_delta: -15,
                duration: 30,
                freq: 35,
                activate_on: String::new(),
            }],
            ..landers
        }
    }

    #[test]
    fn an_event_starts_on_arrival_when_the_chance_fires_and_moves_the_price() {
        let catalog = surplus();
        let mut session = Session::start(&catalog).expect("starts");
        let mut chance = Scripted::answering(&[true]);
        jump_with(&mut session, &catalog, 131, &mut chance);
        assert_eq!(chance.asked, [35], "one roll for the one day");
        assert_eq!(
            session.pilot().events().collect::<Vec<_>>(),
            [(DisasterId(128), 30)]
        );
        assert_eq!(session.land(), Ok(StellarId(140)));
        let market = session.market().expect("an exchange");
        assert_eq!(market.row(FOOD).map(|row| row.price), Some(60));
        assert_eq!(market.events, ["An enormous food surplus"]);
    }

    #[test]
    fn no_event_starts_when_the_chance_does_not_fire() {
        let catalog = surplus();
        let mut session = Session::start(&catalog).expect("starts");
        let mut chance = Scripted::default();
        jump_with(&mut session, &catalog, 131, &mut chance);
        assert_eq!(chance.asked, [35]);
        assert_eq!(session.pilot().events().count(), 0);
        session.land().expect("lands");
        let market = session.market().expect("an exchange");
        assert_eq!(market.row(FOOD).map(|row| row.price), Some(75));
        assert_eq!(market.events, Vec::<String>::new());
    }

    #[test]
    fn each_day_of_a_jump_ages_the_events() {
        let catalog = surplus();
        let mut session = Session::start(&catalog).expect("starts");
        session.pilot.events.insert(DisasterId(128), 5);
        jump(&mut session, &catalog, 131);
        assert_eq!(
            session.pilot().events().collect::<Vec<_>>(),
            [(
                DisasterId(128),
                5 - u16::try_from(DAYS_PER_JUMP).expect("few")
            )]
        );
    }

    /// `catalog` with ship 128 carrying `count` of outfit 320, which has
    /// `mod_type` at `mod_val`, as a default item: a new pilot owns them.
    fn owning(
        catalog: FakePilotCatalog,
        mod_type: i16,
        mod_val: i16,
        count: u16,
    ) -> FakePilotCatalog {
        FakePilotCatalog {
            defaults: vec![(ShipId(128), vec![(OutfitId(320), count)])],
            outfits: vec![outfit(320, &[(mod_type, mod_val)])],
            ..catalog
        }
    }

    #[test]
    fn a_hyperspace_distance_outfit_lets_the_ship_jump_nearer_the_centre() {
        // The stock Horizontal Booster: -500, halving the no-jump zone.
        let booster = owning(catalog(), HYPERSPACE_DISTANCE, -500, 1);
        let mut session = Session::start(&booster).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        session.player.position = Vec2::new(0.0, 600.0);
        assert_eq!(session.begin_jump(), Ok(SystemId(131)));
        assert_eq!(
            session.arrive(&booster, &mut NeverFires),
            Some(SystemId(131))
        );
        assert_eq!(
            session.player().position.length(),
            ARRIVAL_DISTANCE,
            "it still drops out at the standard edge"
        );

        let catalog = catalog();
        let mut plain = Session::start(&catalog).expect("starts");
        plain.plot_course(SystemId(131)).expect("a route");
        plain.player.position = Vec2::new(0.0, 600.0);
        assert_eq!(
            plain.begin_jump(),
            Err(JumpRefusal::TooClose { distance: 600.0 })
        );
        assert_eq!(plain.jumping(), None);
    }

    #[test]
    fn a_hyperspace_speed_outfit_makes_each_jump_take_more_days() {
        let slower = owning(surplus(), HYPERSPACE_DAYS, 1, 1);
        let mut session = Session::start(&slower).expect("starts");
        let mut chance = Scripted::default();
        jump_with(&mut session, &slower, 131, &mut chance);
        assert_eq!(chance.asked, [35, 35], "one roll for each of two days");
        assert_eq!(dmy(&session), (25, 6, 1177));
        assert_eq!(session.reserves().fuel.now, 200.0, "still one jump's fuel");

        // The stock -1 cannot take a jump below a day.
        let quicker = owning(surplus(), HYPERSPACE_DAYS, -1, 1);
        let mut session = Session::start(&quicker).expect("starts");
        let mut chance = Scripted::default();
        jump_with(&mut session, &quicker, 131, &mut chance);
        assert_eq!(chance.asked, [35]);
        assert_eq!(dmy(&session), (24, 6, 1177));
    }

    #[test]
    fn a_pilot_flown_with_cargo_has_less_space_and_can_sell_it() {
        let catalog = exchange();
        let mut session = Session::start(&catalog).expect("starts");
        session.land().expect("lands");
        assert_eq!(session.trade(order(METAL, Direction::Buy, Lot::Max)), Ok(6));
        let pilot = session.pilot().clone();
        let mut resumed = Session::fly(&catalog, pilot).expect("flies");
        let market = resumed.market().expect("docked at the exchange");
        assert_eq!(market.free, 14);
        assert_eq!(market.row(METAL).map(|row| row.held), Some(6));
        assert_eq!(
            resumed.trade(order(METAL, Direction::Sell, Lot::Max)),
            Ok(6)
        );
        assert_eq!(resumed.pilot().cash(), 1000);
    }

    // The outfitter.

    use crate::catalog::{GovtId, OutfitRecord};
    use crate::outfitter::{OutfitFlags, OutfitOrder, OutfitRefusal};
    use crate::stats::{MORE_FUEL, MORE_SHIELD, MORE_SPEED};

    const SPEED: OutfitId = OutfitId(300);
    const CARGO: OutfitId = OutfitId(301);
    const SHIELD: OutfitId = OutfitId(302);
    const TANK: OutfitId = OutfitId(303);
    const SCOOP: OutfitId = OutfitId(304);

    /// Planet 128 at the centre, which the ship starts over: an outfitter
    /// of tech level 5, of government 128, and a trade center as
    /// [`TRADES`] says.
    fn outfitter_site() -> LandingSite {
        LandingSite {
            flags: TRADES | StellarFlags::OUTFITTER,
            tech_level: 5,
            govt: Some(GovtId(128)),
            ..planet(128, 0.0, 0.0)
        }
    }

    /// The catalog with planet 128 an outfitter selling a speed booster
    /// (+300), a cargo pod (+10 tons), a shield (+50), a fuel tank (+100)
    /// and a fuel scoop (a unit every 10 ticks), each a ton and 1000
    /// credits; the first `chär` holds 25,000 credits.
    fn outfitting() -> FakePilotCatalog {
        FakePilotCatalog {
            character: Ok(CharacterStart {
                cash: 25_000,
                ..exchange().character.expect("a chär")
            }),
            sites: vec![(SystemId(130), vec![outfitter_site()])],
            commodities: food_and_metal(),
            outfits: vec![
                outfit(300, &[(MORE_SPEED, 300)]),
                outfit(301, &[(crate::market::MORE_CARGO, 10)]),
                outfit(302, &[(MORE_SHIELD, 50)]),
                outfit(303, &[(MORE_FUEL, 100)]),
                outfit(304, &[(FUEL_SCOOP, 10)]),
            ],
            ..catalog()
        }
    }

    fn buy(outfit: OutfitId) -> OutfitOrder {
        OutfitOrder {
            outfit,
            direction: Direction::Buy,
        }
    }

    fn sell(outfit: OutfitId) -> OutfitOrder {
        OutfitOrder {
            outfit,
            direction: Direction::Sell,
        }
    }

    fn outfitted(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.land().expect("lands");
        session.take_save_due();
        session
    }

    #[test]
    fn there_is_an_outfitter_only_while_landed_at_one() {
        let catalog = outfitting();
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(session.outfitter(), None, "in flight");
        assert_eq!(session.outfit(buy(SPEED)), Err(OutfitRefusal::NoOutfitter));
        session.land().expect("lands");
        let outfitter = session.outfitter().expect("an outfitter");
        assert_eq!(outfitter.rows.len(), 5);
        assert_eq!((outfitter.cash, outfitter.free_mass), (25_000, 30));
        let plain = FakePilotCatalog {
            sites: vec![(
                SystemId(130),
                vec![LandingSite {
                    flags: TRADES,
                    ..outfitter_site()
                }],
            )],
            ..outfitting()
        };
        let mut session = outfitted(&plain);
        assert_eq!(session.outfitter(), None, "no outfitter here");
        assert_eq!(session.outfit(buy(SPEED)), Err(OutfitRefusal::NoOutfitter));
    }

    #[test]
    fn a_session_reads_the_outfits_once_when_it_starts() {
        let catalog = outfitting();
        let mut session = outfitted(&catalog);
        let reads = *catalog.outfit_reads.borrow();
        session.outfit(buy(SPEED)).expect("bought");
        session.outfitter().expect("an outfitter");
        session.take_off();
        jump(&mut session, &catalog, 131);
        assert_eq!(*catalog.outfit_reads.borrow(), reads);
    }

    #[test]
    fn a_speed_booster_raises_the_top_speed_and_selling_it_lowers_it_again() {
        let mut session = outfitted(&outfitting());
        let before = session.handling();
        assert_eq!(session.outfit(buy(SPEED)), Ok(()));
        assert_eq!(session.handling().max_speed, before.max_speed + 3.0);
        assert_eq!(session.handling().accel, before.accel);
        assert_eq!(session.pilot().owned(SPEED), 1);
        assert_eq!(session.pilot().cash(), 24_000);
        assert_eq!(session.outfit(sell(SPEED)), Ok(()));
        assert_eq!(session.handling(), before);
        assert_eq!(session.pilot().cash(), 24_500, "sold for half");
    }

    #[test]
    fn a_cargo_pod_adds_space_to_the_exchange_too() {
        let mut session = outfitted(&outfitting());
        assert_eq!(session.capacity(), 20);
        session.outfit(buy(CARGO)).expect("bought");
        assert_eq!(session.capacity(), 30);
        assert_eq!(session.market().expect("an exchange").free, 30);
        session.outfit(sell(CARGO)).expect("sold");
        assert_eq!(session.capacity(), 20);
    }

    #[test]
    fn selling_space_below_the_cargo_held_is_allowed_and_none_is_free() {
        let mut session = outfitted(&outfitting());
        session.outfit(buy(CARGO)).expect("bought");
        let food = order(FOOD, Direction::Buy, Lot::Max);
        assert_eq!(session.trade(food), Ok(30));
        assert_eq!(session.outfit(sell(CARGO)), Ok(()));
        let market = session.market().expect("an exchange");
        assert_eq!((market.capacity, market.free), (20, 0));
        assert_eq!(session.pilot().held(FOOD), 30);
    }

    #[test]
    fn a_shield_or_tank_comes_full_and_selling_it_keeps_no_more_than_fits() {
        let mut session = outfitted(&outfitting());
        session.pilot.reserves.fuel.now = 200.0;
        session.pilot.reserves.shield.now = 10.0;
        session.outfit(buy(TANK)).expect("bought");
        assert_eq!(
            session.reserves().fuel,
            Gauge {
                now: 300.0,
                max: 400.0
            }
        );
        session.outfit(buy(SHIELD)).expect("bought");
        assert_eq!(
            session.reserves().shield,
            Gauge {
                now: 60.0,
                max: 80.0
            }
        );
        assert_eq!(session.reserves().armor, Gauge::full(45.0), "unchanged");
        session.outfit(sell(TANK)).expect("sold");
        assert_eq!(
            session.reserves().fuel,
            Gauge {
                now: 300.0,
                max: 300.0
            }
        );
        session.outfit(sell(SHIELD)).expect("sold");
        assert_eq!(
            session.reserves().shield,
            Gauge {
                now: 30.0,
                max: 30.0
            }
        );
        session.pilot.reserves.fuel.now = 50.0;
        session.outfit(buy(TANK)).expect("bought");
        session.outfit(sell(TANK)).expect("sold");
        assert_eq!(
            session.reserves().fuel,
            Gauge {
                now: 150.0,
                max: 300.0
            }
        );
    }

    #[test]
    fn a_fuel_scoop_adds_to_the_regeneration_and_selling_it_takes_it_away() {
        let mut session = outfitted(&outfitting());
        assert_eq!(session.fuel_regen_per_tick(), 0.0);
        session.outfit(buy(SCOOP)).expect("bought");
        assert_eq!(session.fuel_regen_per_tick(), 0.1);
        assert_eq!(session.stats().fuel_regen, 0.1);
        session.outfit(sell(SCOOP)).expect("sold");
        assert_eq!(session.fuel_regen_per_tick(), 0.0);
    }

    #[test]
    fn an_outfit_bought_or_sold_makes_a_save_due_and_a_refused_one_changes_nothing() {
        let mut session = outfitted(&outfitting());
        let before = session.clone();
        assert_eq!(session.outfit(sell(SPEED)), Err(OutfitRefusal::NoneOwned));
        assert_eq!(
            session.outfit(buy(OutfitId(999))),
            Err(OutfitRefusal::NotListed)
        );
        assert_eq!(session, before);
        assert!(!session.take_save_due());
        session.outfit(buy(SPEED)).expect("bought");
        assert!(session.take_save_due());
        session.outfit(sell(SPEED)).expect("sold");
        assert!(session.take_save_due());
    }

    #[test]
    fn buying_is_refused_by_cash_free_mass_or_max() {
        let heavy = OutfitRecord {
            mass: 20,
            max: 3,
            ..outfit(305, &[])
        };
        let dear = OutfitRecord {
            cost: 30_000,
            ..outfit(306, &[])
        };
        let catalog = FakePilotCatalog {
            outfits: vec![heavy, dear],
            ..outfitting()
        };
        let mut session = outfitted(&catalog);
        assert_eq!(
            session.outfit(buy(OutfitId(306))),
            Err(OutfitRefusal::CannotAfford)
        );
        assert_eq!(session.outfit(buy(OutfitId(305))), Ok(()));
        let before = session.clone();
        assert_eq!(
            session.outfit(buy(OutfitId(305))),
            Err(OutfitRefusal::NoSpace)
        );
        assert_eq!(session, before);
        let roomy = FakePilotCatalog {
            ships: vec![(
                ShipId(128),
                Ok(ShipFields {
                    free_mass: 100,
                    ..FAST
                }),
            )],
            ..catalog
        };
        let mut session = outfitted(&roomy);
        for _ in 0..3 {
            session.outfit(buy(OutfitId(305))).expect("bought");
        }
        assert_eq!(
            session.outfit(buy(OutfitId(305))),
            Err(OutfitRefusal::MaxOwned)
        );
        assert_eq!(session.pilot().owned(OutfitId(305)), 3);
    }

    #[test]
    fn an_owned_outfit_that_sells_anywhere_sells_where_it_is_not_for_sale() {
        let map = OutfitRecord {
            tech_level: 9,
            cost: 4001,
            flags: OutfitFlags::SELL_ANYWHERE,
            ..outfit(310, &[])
        };
        let catalog = FakePilotCatalog {
            defaults: vec![(ShipId(128), vec![(OutfitId(310), 2)])],
            outfits: vec![map],
            ..outfitting()
        };
        let mut session = outfitted(&catalog);
        let outfitter = session.outfitter().expect("an outfitter");
        let row = outfitter.row(OutfitId(310)).expect("listed, sell-only");
        assert_eq!(row.buy, Err(OutfitRefusal::NotForSale));
        assert_eq!(row.sell, Ok(()));
        assert_eq!(session.outfit(sell(OutfitId(310))), Ok(()));
        assert_eq!(session.pilot().owned(OutfitId(310)), 1);
        assert_eq!(session.pilot().cash(), 25_000 + 2000);
    }

    #[test]
    fn a_new_pilots_default_items_are_owned_and_its_reserves_full_with_them() {
        let catalog = FakePilotCatalog {
            defaults: vec![(ShipId(128), vec![(TANK, 1), (SHIELD, 2)])],
            ..outfitting()
        };
        let session = Session::start(&catalog).expect("starts");
        assert_eq!(
            session.pilot().outfits().collect::<Vec<_>>(),
            [(SHIELD, 2), (TANK, 1)]
        );
        assert_eq!(
            session.reserves(),
            Reserves {
                shield: Gauge::full(130.0),
                armor: Gauge::full(45.0),
                fuel: Gauge::full(400.0),
            }
        );
        let outfitter = outfitted(&catalog).outfitter().expect("an outfitter");
        assert_eq!(outfitter.free_mass, 30, "free mass is on top of them");
    }

    #[test]
    fn flying_a_pilot_with_outfits_gives_their_stats() {
        let catalog = outfitting();
        let mut session = outfitted(&catalog);
        session.outfit(buy(SPEED)).expect("bought");
        session.outfit(buy(TANK)).expect("bought");
        let text = crate::save::encode(session.pilot());
        let pilot = crate::save::decode(&text).expect("a pilot");
        let resumed = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(resumed.stats(), session.stats());
        assert_eq!(resumed.reserves(), session.reserves());
        assert_eq!(resumed.pilot().owned(SPEED), 1);
    }

    #[test]
    fn flying_a_pilot_from_before_outfits_gives_it_the_ships_default_items() {
        let catalog = FakePilotCatalog {
            defaults: vec![(ShipId(128), vec![(TANK, 1), (SCOOP, 1)])],
            ..outfitting()
        };
        let mut pilot = Pilot::new(&catalog, "Old").expect("starts");
        pilot.outfits.clear();
        pilot.default_outfits_pending = true;
        pilot.reserves = Reserves::full(30.0, 45.0, 300.0);
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(
            session.pilot().outfits().collect::<Vec<_>>(),
            [(TANK, 1), (SCOOP, 1)]
        );
        assert!(!session.pilot().default_outfits_pending);
        assert_eq!(session.fuel_regen_per_tick(), 0.1);
        assert_eq!(
            session.reserves().fuel,
            Gauge {
                now: 300.0,
                max: 400.0
            },
            "as saved, with room for more"
        );
        assert!(!session.take_save_due());
        let saved: serde_json::Value =
            serde_json::from_str(&crate::save::encode(session.pilot())).expect("JSON");
        assert_eq!(
            saved["outfits"],
            serde_json::json!([{"outfit": 303, "count": 1}, {"outfit": 304, "count": 1}])
        );
    }

    #[test]
    fn flying_a_pilot_keeps_no_more_in_its_reserves_than_its_stats_hold() {
        let catalog = outfitting();
        let mut pilot = Pilot::new(&catalog, "").expect("starts");
        pilot.reserves = Reserves {
            shield: Gauge {
                now: 90.0,
                max: 100.0,
            },
            armor: Gauge {
                now: 20.0,
                max: 45.0,
            },
            fuel: Gauge {
                now: 350.0,
                max: 500.0,
            },
        };
        let session = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(
            session.reserves(),
            Reserves {
                shield: Gauge {
                    now: 30.0,
                    max: 30.0
                },
                armor: Gauge {
                    now: 20.0,
                    max: 45.0
                },
                fuel: Gauge {
                    now: 300.0,
                    max: 300.0
                },
            }
        );
    }

    // The shipyard.

    use crate::catalog::ShipRecord;
    use crate::shipyard::{ShipPurchase, ShipRefusal};
    use crate::testkit::ship;

    /// Ship 129: faster, with more shield, 15 tons of cargo space and 12
    /// free, of mass 25, regenerating a unit of fuel every 5 ticks.
    const HEAVY: ShipFields = ShipFields {
        speed: 900,
        shield: 80,
        fuel_regen: 5,
        holds: 15,
        free_mass: 12,
        mass: 25,
        ..FAST
    };

    /// Planet 128 at the centre, which the ship starts over: a shipyard
    /// and outfitter of tech level 5, and a trade center.
    fn shipyard_site() -> LandingSite {
        LandingSite {
            flags: TRADES | StellarFlags::OUTFITTER | StellarFlags::SHIPYARD,
            ..outfitter_site()
        }
    }

    /// [`outfitting`], where planet 128 is a shipyard too, selling ship
    /// 128 (FAST, 10,000 credits) and ship 129 ([`HEAVY`], 17,500
    /// credits, carrying a fuel tank).
    fn shipbuying() -> FakePilotCatalog {
        FakePilotCatalog {
            sites: vec![(SystemId(130), vec![shipyard_site()])],
            ships: vec![(ShipId(128), Ok(FAST)), (ShipId(129), Ok(HEAVY))],
            defaults: vec![(ShipId(129), vec![(TANK, 1)])],
            ship_records: vec![
                ship(128, FAST),
                ShipRecord {
                    cost: 17_500,
                    defaults: vec![(TANK, 1)],
                    ..ship(129, HEAVY)
                },
            ],
            ..outfitting()
        }
    }

    const NEW: ShipId = ShipId(129);

    #[test]
    fn there_is_a_shipyard_only_while_landed_at_one() {
        let catalog = shipbuying();
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(session.shipyard(), None, "in flight");
        assert_eq!(session.buy_ship(NEW), Err(ShipRefusal::NoShipyard));
        session.land().expect("lands");
        let shipyard = session.shipyard().expect("a shipyard");
        assert_eq!(shipyard.rows.len(), 2);
        assert_eq!((shipyard.cash, shipyard.trade_in), (25_000, 2500));
        assert_eq!(shipyard.current, ShipId(128));
        let plain = FakePilotCatalog {
            sites: vec![(SystemId(130), vec![outfitter_site()])],
            ..shipbuying()
        };
        let mut session = outfitted(&plain);
        assert_eq!(session.shipyard(), None, "no shipyard here");
        assert_eq!(session.buy_ship(NEW), Err(ShipRefusal::NoShipyard));
    }

    #[test]
    fn a_session_reads_the_ship_records_once_when_it_starts() {
        let catalog = shipbuying();
        let mut session = outfitted(&catalog);
        assert_eq!(*catalog.ship_record_reads.borrow(), 1);
        session.buy_ship(NEW).expect("bought");
        session.shipyard().expect("a shipyard");
        session.take_off();
        jump(&mut session, &catalog, 131);
        assert_eq!(*catalog.ship_record_reads.borrow(), 1);
    }

    #[test]
    fn buying_a_ship_makes_a_save_due_and_a_refused_one_changes_nothing() {
        let catalog = shipbuying();
        let mut session = outfitted(&catalog);
        let before = session.clone();
        assert_eq!(session.buy_ship(ShipId(999)), Err(ShipRefusal::NotListed));
        let mut poor = session.clone();
        poor.pilot.cash = 14_999;
        let poorer = poor.clone();
        assert_eq!(poor.buy_ship(NEW), Err(ShipRefusal::CannotAfford));
        assert_eq!(poor, poorer);
        assert!(!poor.take_save_due());
        assert_eq!(session, before);
        assert!(!session.take_save_due());
        assert_eq!(
            session.buy_ship(NEW),
            Ok(ShipPurchase {
                price: 17_500,
                trade_in: 2500,
                sold_back: BTreeMap::new(),
                refund: 0,
                left_behind: BTreeMap::new(),
            })
        );
        assert!(session.take_save_due());
        assert_eq!(session.ship(), NEW);
        assert_eq!(session.pilot().cash(), 25_000 - 17_500 + 2500);
        assert_eq!(session.pilot().outfits().collect::<Vec<_>>(), [(TANK, 1)]);
    }

    #[test]
    fn after_a_purchase_the_stats_are_the_new_ships_with_its_outfits() {
        let catalog = shipbuying();
        let mut session = outfitted(&catalog);
        session.outfit(buy(SPEED)).expect("bought");
        session.buy_ship(NEW).expect("bought");
        let stats = ShipStats::new(
            HEAVY,
            &[crate::fuel::OutfitMod {
                mod_type: MORE_FUEL,
                mod_val: 100,
                count: 1,
            }],
        );
        assert_eq!(session.stats(), stats, "the booster went with the old ship");
        assert_eq!(session.handling(), stats.handling);
        assert_eq!(session.handling().max_speed, 9.0);
        assert_eq!(session.capacity(), 15);
        assert_eq!(session.reserves(), stats.full());
        assert_eq!(session.reserves().fuel, Gauge::full(400.0));
        assert_eq!(session.fuel_regen_per_tick(), 0.2);
        assert_eq!(session.market().expect("an exchange").capacity, 15);
    }

    #[test]
    fn after_a_purchase_the_outfitter_reads_the_new_ships_free_mass_and_defaults() {
        let catalog = shipbuying();
        let mut session = outfitted(&catalog);
        session.buy_ship(NEW).expect("bought");
        let outfitter = session.outfitter().expect("an outfitter");
        assert_eq!(outfitter.free_mass, 12, "its tank is fitted on top");
        let shipyard = session.shipyard().expect("a shipyard");
        assert_eq!(shipyard.current, NEW);
        assert_eq!(shipyard.trade_in, 17_500 / 4 + 500, "the hull and its tank");
    }

    #[test]
    fn after_take_off_the_ship_flies_at_the_new_top_speed() {
        let catalog = shipbuying();
        let mut session = outfitted(&catalog);
        session.buy_ship(NEW).expect("bought");
        session.take_off().expect("took off");
        let mut expected = *session.player();
        for _ in 0..200 {
            session.tick(THRUST);
            step(&mut expected, &ShipStats::new(HEAVY, &[]).handling, THRUST);
        }
        assert_eq!(*session.player(), expected);
        assert!((session.player().velocity.length() - 9.0).abs() < 1e-3);
    }

    #[test]
    fn a_purchase_sells_back_a_persistent_outfit_priced_on_the_ship_flown() {
        // A persistent armour plate, 100 credits a ton of ship and 20 tons,
        // fitted to ship 128 (mass 40): it does not fit ship 129's 12 tons
        // free, and sells back at half of 100 x 40, not of 100 x 25.
        const PLATE: OutfitId = OutfitId(320);
        let plate = OutfitRecord {
            mass: 20,
            cost: 100,
            max: 1,
            flags: OutfitFlags::PERSISTENT | OutfitFlags::PRICE_BY_MASS,
            ..outfit(320, &[])
        };
        let mut catalog = shipbuying();
        catalog.outfits.push(plate);
        catalog.defaults.push((ShipId(128), vec![(PLATE, 1)]));
        let mut session = outfitted(&catalog);
        assert_eq!(session.pilot().owned(PLATE), 1);
        assert_ne!(FAST.mass, HEAVY.mass);
        let bought = session.buy_ship(NEW).expect("bought");
        assert_eq!(bought.sold_back, BTreeMap::from([(PLATE, 1)]));
        assert_eq!(bought.refund, 100 * i64::from(FAST.mass) / 2);
        assert_eq!(bought.trade_in, 2500, "persistent: not in the trade-in");
        assert_eq!(session.pilot().cash(), 25_000 - 17_500 + 2500 + 2000);
        assert_eq!(session.pilot().owned(PLATE), 0);
    }

    #[test]
    fn a_pilot_saved_after_a_purchase_flies_again_as_it_was() {
        let catalog = shipbuying();
        let mut session = outfitted(&catalog);
        session
            .trade(order(FOOD, Direction::Buy, Lot::Max))
            .expect("bought");
        session.buy_ship(NEW).expect("bought");
        let text = crate::save::encode(session.pilot());
        let pilot = crate::save::decode(&text).expect("a pilot");
        assert_eq!(pilot, *session.pilot());
        let resumed = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(resumed.ship(), NEW);
        assert_eq!(resumed.pilot().outfits().collect::<Vec<_>>(), [(TANK, 1)]);
        assert_eq!(resumed.pilot().cash(), session.pilot().cash());
        assert_eq!(resumed.pilot().held(FOOD), 15);
        assert_eq!(resumed.reserves(), session.reserves());
        assert_eq!(resumed.stats(), session.stats());
        assert_eq!(*resumed.pilot(), *session.pilot(), "no default added twice");
    }

    // Recharging.

    /// A session landed at the exchange's inhabited planet 128 with 1000
    /// credits and half a tank (150 of 300), no save due.
    fn half_empty_at_port() -> Session {
        let mut session = Session::start(&exchange()).expect("starts");
        session.land().expect("lands");
        session.pilot.reserves.fuel.now = 150.0;
        session.take_save_due();
        session
    }

    #[test]
    fn recharging_fills_the_tank_charges_the_price_and_makes_a_save_due() {
        let mut session = half_empty_at_port();
        assert_eq!(session.recharge(), Ok(150));
        assert_eq!(session.reserves().fuel, Gauge::full(300.0));
        assert_eq!(session.pilot().cash(), 1000 - 150);
        assert!(session.take_save_due());
    }

    /// Asserts `session.recharge()` is refused with `refusal`, changing
    /// nothing and making no save due.
    fn assert_refused(mut session: Session, refusal: RechargeRefusal) {
        let before = session.clone();
        assert_eq!(session.recharge(), Err(refusal));
        assert_eq!(session, before, "nothing changes");
        assert!(!session.take_save_due());
    }

    #[test]
    fn a_full_tank_is_not_recharged() {
        let mut session = half_empty_at_port();
        session.pilot.reserves.fuel.now = 300.0;
        assert_refused(session, RechargeRefusal::Full);
    }

    #[test]
    fn a_refill_the_pilot_cannot_pay_for_is_refused() {
        let mut session = half_empty_at_port();
        session.pilot.cash = 149;
        assert_refused(session, RechargeRefusal::CannotAfford);
    }

    #[test]
    fn an_uninhabited_stellar_sells_no_fuel() {
        let catalog = FakePilotCatalog {
            sites: vec![(
                SystemId(130),
                vec![LandingSite {
                    flags: StellarFlags::CAN_LAND | StellarFlags::UNINHABITED,
                    ..planet(128, 0.0, 0.0)
                }],
            )],
            ..exchange()
        };
        let mut session = Session::start(&catalog).expect("starts");
        session.land().expect("lands");
        session.pilot.reserves.fuel.now = 150.0;
        session.take_save_due();
        assert_refused(session, RechargeRefusal::NoFuel);
    }

    #[test]
    fn no_fuel_is_sold_in_flight() {
        let mut session = half_empty_at_port();
        session.take_off();
        session.take_save_due();
        assert_refused(session, RechargeRefusal::NoFuel);
    }
    // Traffic.

    use crate::ai::Peaceful;
    use crate::catalog::{DudeId, DudeRecord, SystemTraffic};
    use crate::testkit::Draws;
    use crate::traffic::autopilot::Outcome;
    use crate::traffic::npc::AiType;

    /// [`catalog`] with `avg` ships on average in `system`, all of düde
    /// 128: ship 129 (fast) with AI `ai_type`, for govt 140.
    fn trafficked(system: i16, avg: i16, ai_type: i16) -> FakePilotCatalog {
        let mut dude_types = [(-1, 0); 8];
        dude_types[0] = (128, 100);
        FakePilotCatalog {
            traffic: vec![(
                SystemId(system),
                SystemTraffic {
                    dude_types,
                    avg_ships: avg,
                },
            )],
            dudes: vec![(
                DudeId(128),
                DudeRecord {
                    ai_type,
                    govt: Some(GovtId(140)),
                    ships: vec![(ShipId(129), 1)],
                },
            )],
            ship_records: vec![ship(129, FAST)],
            ..catalog()
        }
    }

    #[test]
    fn a_session_starts_with_no_traffic_until_it_is_populated() {
        let catalog = trafficked(130, 2, 1);
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(session.npcs(), []);
        assert_eq!(session.traffic_ships(), []);
        session.populate(&catalog, &mut NeverFires);
        assert_eq!(session.npcs().len(), 2);
        assert_eq!(session.traffic_ships(), [ShipId(129)]);
    }

    #[test]
    fn arriving_populates_the_new_system_from_its_dudes() {
        let catalog = trafficked(131, 2, 1);
        let mut session = Session::start(&catalog).expect("starts");
        jump(&mut session, &catalog, 131);
        let npcs = session.npcs();
        assert_eq!(npcs.len(), 2);
        for npc in npcs {
            assert_eq!(
                (npc.ship, npc.govt, npc.ai_type),
                (ShipId(129), Some(GovtId(140)), AiType::WimpyTrader)
            );
            assert_eq!(npc.stats, ShipStats::new(FAST, &[]));
        }
        assert_eq!(session.traffic_ships(), [ShipId(129)]);
    }

    #[test]
    fn arriving_clears_the_last_systems_traffic() {
        let catalog = trafficked(130, 2, 3);
        let mut session = Session::start(&catalog).expect("starts");
        session.populate(&catalog, &mut NeverFires);
        assert_eq!(session.npcs().len(), 2);
        jump(&mut session, &catalog, 131);
        assert_eq!(session.npcs(), []);
        assert_eq!(session.traffic_ships(), []);
    }

    #[test]
    fn populating_replaces_the_npcs() {
        let catalog = trafficked(130, 3, 3);
        let mut session = Session::start(&catalog).expect("starts");
        session.populate(&catalog, &mut NeverFires);
        let first: Vec<_> = session.npcs().iter().map(|npc| npc.id).collect();
        session.populate(&catalog, &mut NeverFires);
        let second: Vec<_> = session.npcs().iter().map(|npc| npc.id).collect();
        assert_eq!(second.len(), 3);
        assert!(
            first.iter().all(|id| !second.contains(id)),
            "{first:?} {second:?}"
        );
        let mut persons = Draws::of(&[0, 0, 0]);
        session.populate(&catalog, &mut persons);
        assert_eq!(session.npcs(), []);
        assert_eq!(persons.asked, [7, 7, 7]);
    }

    #[test]
    fn traffic_arrives_over_time() {
        let catalog = trafficked(130, 1, 3);
        let mut session = Session::start(&catalog).expect("starts");
        session.populate(&catalog, &mut Draws::of(&[0]));
        assert_eq!(session.npcs(), []);
        session.tick_traffic(&catalog, &Peaceful, &mut Draws::of(&[2]));
        assert_eq!(session.npcs(), []);
        session.tick_traffic(&catalog, &Peaceful, &mut Draws::of(&[0, 6, 6, 0, 0, 0]));
        assert_eq!(session.npcs().len(), 1, "jumping in");
    }

    /// Ticks the traffic until an NPC leaves, and gives how.
    fn first_departure(session: &mut Session, catalog: &FakePilotCatalog) -> Outcome {
        for _ in 0..3000 {
            session.tick_traffic(catalog, &Peaceful, &mut NeverFires);
            if let Some(&(_, outcome)) = session.departed().first() {
                return outcome;
            }
        }
        panic!("nobody left: {:?}", session.npcs());
    }

    #[test]
    fn traders_land_and_others_jump_out() {
        for (ai_type, how) in [
            (1, Outcome::Landed(StellarId(129))),
            (2, Outcome::Landed(StellarId(129))),
            (3, Outcome::JumpedOut),
            (4, Outcome::JumpedOut),
        ] {
            let catalog = trafficked(130, 1, ai_type);
            let mut session = Session::start(&catalog).expect("starts");
            session.populate(&catalog, &mut NeverFires);
            assert_eq!(first_departure(&mut session, &catalog), how, "AI {ai_type}");
            assert_eq!(session.npcs(), []);
        }
    }

    #[test]
    fn traffic_stands_still_while_landed_or_jumping() {
        let mut catalog = trafficked(130, 1, 1);
        let elsewhere = (SystemId(131), catalog.traffic[0].1);
        catalog.traffic.push(elsewhere);
        let mut session = Session::start(&catalog).expect("starts");
        session.populate(&catalog, &mut NeverFires);
        session.land().expect("lands on planet 128");
        let before = session.npcs().to_vec();
        session.tick_traffic(&catalog, &Peaceful, &mut Draws::of(&[0]));
        assert_eq!(session.npcs(), before, "landed");
        let mut session = Session::start(&catalog).expect("starts");
        session.populate(&catalog, &mut NeverFires);
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        session.begin_jump().expect("jumps");
        let before = session.npcs().to_vec();
        let mut chance = Draws::of(&[]);
        session.tick_traffic(&catalog, &Peaceful, &mut chance);
        assert_eq!(session.npcs(), before, "jumping");
        assert!(chance.asked.is_empty());
        session.arrive(&catalog, &mut NeverFires).expect("arrives");
        let before = session.npcs().to_vec();
        session.tick_traffic(&catalog, &Peaceful, &mut chance);
        assert_eq!(session.npcs().len(), 1);
        assert_ne!(session.npcs()[0].state, before[0].state, "flying again");
    }

    fn npc_ids(session: &Session) -> Vec<NpcId> {
        session.npcs().iter().map(|npc| npc.id).collect()
    }

    #[test]
    fn a_session_populates_its_system_on_its_first_traffic_tick() {
        let catalog = trafficked(130, 2, 1);
        let mut session = Session::start(&catalog).expect("starts");
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert_eq!(npc_ids(&session), [NpcId(0), NpcId(1)]);
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert_eq!(npc_ids(&session), [NpcId(0), NpcId(1)], "populated once");
    }

    #[test]
    fn the_tick_that_populates_the_system_moves_no_one() {
        let catalog = trafficked(130, 2, 1);
        let mut ticked = Session::start(&catalog).expect("starts");
        ticked.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        let mut populated = Session::start(&catalog).expect("starts");
        populated.populate(&catalog, &mut NeverFires);
        assert_eq!(ticked.npcs(), populated.npcs());
    }

    #[test]
    fn taking_off_repopulates_the_system_on_the_next_traffic_tick() {
        let catalog = trafficked(130, 2, 1);
        let mut session = Session::start(&catalog).expect("starts");
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        session.land().expect("lands on planet 128");
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert_eq!(npc_ids(&session), [NpcId(0), NpcId(1)], "landed");
        session.take_off().expect("took off");
        assert_eq!(npc_ids(&session), [NpcId(0), NpcId(1)], "until it ticks");
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert_eq!(npc_ids(&session), [NpcId(2), NpcId(3)]);
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert_eq!(npc_ids(&session), [NpcId(2), NpcId(3)], "populated once");
    }

    #[test]
    fn a_ship_landed_from_the_start_populates_once_it_takes_off() {
        let catalog = trafficked(130, 2, 1);
        let mut session = Session::start(&catalog).expect("starts");
        session.land().expect("lands on planet 128");
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert_eq!(npc_ids(&session), [], "landed");
        session.take_off().expect("took off");
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert_eq!(npc_ids(&session), [NpcId(0), NpcId(1)]);
    }

    #[test]
    fn a_system_populated_on_arrival_is_not_populated_again_on_its_first_tick() {
        let catalog = trafficked(131, 2, 1);
        let mut session = Session::start(&catalog).expect("starts");
        jump(&mut session, &catalog, 131);
        assert_eq!(npc_ids(&session), [NpcId(0), NpcId(1)]);
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert_eq!(npc_ids(&session), [NpcId(0), NpcId(1)]);
    }

    #[test]
    fn a_populated_system_is_not_populated_again_on_its_first_tick() {
        let catalog = trafficked(130, 2, 1);
        let mut session = Session::start(&catalog).expect("starts");
        session.populate(&catalog, &mut NeverFires);
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert_eq!(npc_ids(&session), [NpcId(0), NpcId(1)]);
    }
}
