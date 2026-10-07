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
//! The ship lands on a stellar of its system, on the land key's second
//! press, when the [`landing`](crate::landing) rules allow it, which read
//! the pilot's legal record with the stellar's government, or with its
//! system's on the star map when the stellar has none: docked, it rests
//! at the stellar's centre and ticks move nothing until it takes off
//! again, from the same place. A landed ship cannot jump, nor a jumping
//! one land.
//!
//! The player plots a course to a system on the star map, read once when
//! the session starts: the fewest jumps along the hyperlinks, which the
//! session's [`HyperlinkRule`] (the engine's, one way along each system's
//! own links, unless [`Session::with_hyperlinks`] says otherwise) lets it
//! follow. J
//! ([`Session::begin_jump`]) is accepted when the
//! [`hyperspace`](crate::hyperspace) rules allow a jump to the next system
//! on it. The ship then flies the pre-jump stage on its own, the player's
//! controls ignored: it brakes and turns to the map bearing of that system
//! as [`pre_jump`] says ([`Session::preparing_jump`]), and the jump begins
//! once it is ready ([`Session::jumping`]), at once if it already is. A
//! jump accepted is committed. While the jump lasts ticks move nothing.
//! When it is over ([`Session::arrive`]) the ship is in the next system, at its edge,
//! with a jump's fuel used and the days its stats give a jump gone by, and
//! the rest of the course still ahead. A ship with a multi-jump outfit
//! passes on along the course in the same jump, as many systems as
//! [`hops_per_jump`] gives, and enters only the last; the session's
//! [`MultiJumpRule`] (the engine's unless [`Session::with_multi_jump`]
//! says otherwise) decides whether the chain costs one jump's fuel and
//! days or each hop's. In flight, fuel regenerates each tick at the rate the ship
//! and its outfits give.
//!
//! The land key's second press over a hypergate or wormhole, once
//! cleared, enters it rather than docking ([`Session::land`]); the
//! player then picks a hypergate's destination
//! ([`Session::open_hypergate`], [`Session::enter_hypergate`]),
//! while a wormhole picks its own ([`Session::enter_wormhole`]), as the
//! [`gate`](crate::gate) rules say. Neither uses fuel nor minds the
//! no-jump zone, and the ship comes out as the session's
//! [`GateArrivalRule`] says; by the engine's, at the exit gate with no
//! day passing.
//!
//! Everything about how the ship performs (its handling, the most shield,
//! armour and fuel it holds, its fuel regeneration and its cargo space)
//! comes from its [`ShipStats`]: its `shïp`'s fields, read when the session
//! starts (or from the record of a ship bought since), and the outfits the
//! pilot owns. A pilot from a save made
//! before outfits were kept owns its ship's default items.
//!
//! The player selects one of the system's stellars as the navigation
//! target ([`Session::select_next_stellar`]), in the order the
//! [`navigation`](crate::navigation) rule gives, or with the land key
//! ([`Session::land`]), which selects the nearest landable stellar when
//! there is no target and lands on the target when there is one. Landing
//! clears it. The target is not part of the pilot, so it is not saved,
//! and it is independent of the course.
//! Arriving in another system clears it.
//!
//! Hyper Select ([`Session::select_next_system`], the original's `\`
//! key) cycles the course through the systems linked with the one the
//! ship is in, as the session's [`HyperSelectRule`] says (the engine's
//! unless [`Session::with_hyper_select`] says otherwise), each press
//! leaving a one-jump course and clearing the navigation target.
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
//! As it goes the session emits [`SimSound`] events (thrust starting and
//! stopping, landing, taking off, a jump beginning and ending), which the
//! audio side drains with [`Session::take_sounds`]. A refused landing or
//! jump emits nothing.
//!
//! In flight the engine glow's base level ([`Session::engine_glow`])
//! ramps with the thrust each tick, as [`glow`](crate::glow) says, and
//! landing and beginning a jump put it out.

use std::collections::BTreeMap;

use crate::catalog::{
    DateAffixes, GateSite, GovtId, LandingSite, OutfitId, OutfitRecord, PilotCatalog, ShipId,
    ShipRecord, StartError, StellarId, SystemId,
};
use crate::chance::Chance;
use crate::date::{self, GameDate};
use crate::flight::{Controls, ShipState, step};
use crate::fuel::regenerate;
use crate::gate::{
    self, GateArrivalRule, GateKind, GateRefusal, WormholeRule, emerge, has_links, hypergate_exit,
    wormhole_exit,
};
use crate::geometry::Vec2;
use crate::glow::ramp_glow;
use crate::handling::{Handling, ShipFields};
use crate::hyperspace::{
    HyperSelectRule, HyperlinkRule, JUMP_FUEL, JumpRefusal, MultiJumpRule, RouteError, StarMap,
    arrival, check_jump, hops_per_jump, jump_bearing, next_hyper_destination,
};
use crate::landing::{LandOutcome, LandingRefusal, land_or_select};
use crate::market::{self, Goods, Market, Order, TradeRefusal};
use crate::message::SimMessage;
use crate::navigation::next_stellar;
use crate::outfitter::{self, OutfitOrder, OutfitRefusal, Outfitter, Shop, outfit_mods};
use crate::pilot::{self, Pilot};
use crate::pre_jump::{self, PreJump};
use crate::recharge::{self, RechargeRefusal};
use crate::reserves::{Gauge, Reserves};
use crate::shipyard::{self, Quote, ShipPurchase, ShipRefusal, Shipyard, Yard};
use crate::sound::SimSound;
use crate::stats::ShipStats;

/// A jump, from J being accepted until the ship arrives.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Jump {
    /// The ship brakes and turns towards `bearing`, the map bearing of
    /// `to` (none when there is nothing to turn to), before it jumps.
    PreJump { to: SystemId, bearing: Option<f32> },
    /// The jump to the system has begun.
    Hyperspace(SystemId),
}

/// What [`Session::land`] did, when it was not refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LandPress {
    /// What the [`landing`](crate::landing) rules gave: a stellar
    /// selected, or one landed on and docked at.
    Outcome(LandOutcome),
    /// The ship is cleared and over a hypergate or wormhole, and the
    /// entry awaits [`Session::enter_hypergate`] or
    /// [`Session::enter_wormhole`]: what the landing rules would let it
    /// land on, it enters instead (see [`gate`]).
    AtGate {
        /// The stellar.
        stellar: StellarId,
        /// What it is.
        kind: GateKind,
    },
}

impl From<LandOutcome> for LandPress {
    fn from(outcome: LandOutcome) -> Self {
        Self::Outcome(outcome)
    }
}

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
    /// The stellar selected as the navigation target, one of `sites`.
    nav_target: Option<StellarId>,
    /// The star map, read when the session starts.
    star_map: StarMap,
    /// The jump under way, from J being accepted until the ship arrives.
    jump: Option<Jump>,
    /// The rule a multi-jump follows.
    multi_jump: MultiJumpRule,
    /// The rule Hyper Select follows.
    hyper_select: HyperSelectRule,
    /// The rule jumps along the hyperlinks follow.
    hyperlinks: HyperlinkRule,
    /// The hypergate or wormhole the land key has just been pressed over,
    /// cleared: its entry awaits until the next tick.
    gate: Option<StellarId>,
    /// The rule a ship coming out of a gate follows.
    gate_arrival: GateArrivalRule,
    /// The rule an unlinked wormhole follows.
    wormholes: WormholeRule,
    /// The goods traded and the events that move their prices, read when
    /// the session starts.
    goods: Goods,
    /// Whether the ship is thrusting, as the last sounds told it.
    thrusting: bool,
    /// The engine glow's base level (see [`glow`](crate::glow)).
    engine_glow: u8,
    /// The sounds emitted since they were last taken.
    sounds: Vec<SimSound>,
    /// The messages raised since they were last taken.
    messages: Vec<SimMessage>,
    /// Whether the pilot has changed in a way that should be saved since
    /// this was last taken.
    save_due: bool,
    /// What the date is wrapped in when it is displayed, read when the
    /// session starts.
    date_affixes: DateAffixes,
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
            nav_target: None,
            star_map: StarMap::new(catalog.star_map()),
            jump: None,
            multi_jump: MultiJumpRule::default(),
            hyper_select: HyperSelectRule::default(),
            hyperlinks: HyperlinkRule::default(),
            gate: None,
            gate_arrival: GateArrivalRule::default(),
            wormholes: WormholeRule::default(),
            goods: Goods::read(catalog),
            thrusting: false,
            engine_glow: 0,
            sounds: Vec::new(),
            messages: Vec::new(),
            save_due: false,
            date_affixes: catalog.date_affixes(),
            pilot,
        };
        session.refit(false);
        Ok(session)
    }

    /// This session with multi-jumps following `rule`; the engine's by
    /// default.
    #[must_use]
    pub fn with_multi_jump(mut self, rule: MultiJumpRule) -> Self {
        self.multi_jump = rule;
        self
    }

    /// This session with Hyper Select following `rule`; the engine's by
    /// default.
    #[must_use]
    pub fn with_hyper_select(mut self, rule: HyperSelectRule) -> Self {
        self.hyper_select = rule;
        self
    }

    /// This session with jumps following the hyperlinks under `rule`; the
    /// engine's by default.
    #[must_use]
    pub fn with_hyperlinks(mut self, rule: HyperlinkRule) -> Self {
        self.hyperlinks = rule;
        self
    }

    /// This session with ships coming out of hypergates and wormholes as
    /// `rule` says; the engine's by default.
    #[must_use]
    pub fn with_gate_arrival(mut self, rule: GateArrivalRule) -> Self {
        self.gate_arrival = rule;
        self
    }

    /// This session with unlinked wormholes leading where `rule` says; the
    /// engine's by default.
    #[must_use]
    pub fn with_wormholes(mut self, rule: WormholeRule) -> Self {
        self.wormholes = rule;
        self
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

    /// Advances the session one tick under the player's `controls`, ramps
    /// the engine glow with the thrust, then regenerates fuel. During the
    /// pre-jump stage the controls are ignored: the ship flies the stage
    /// ([`pre_jump::fly`]) instead, and the jump begins on the tick it is
    /// ready. A landed ship, or one in hyperspace, does not move, glows not
    /// at all, and gains no fuel. Any tick ends a gate's pending entry: the
    /// original's hypergate map is modal, so nothing ticks between the
    /// land key and the pick.
    pub fn tick(&mut self, controls: Controls) {
        self.gate = None;
        if self.landed.is_some() {
            return;
        }
        match self.jump {
            None => {
                self.thrust(controls.thrust);
                step(&mut self.player, &self.stats.handling, controls);
            }
            Some(Jump::PreJump { to, bearing }) => {
                let handling = &self.stats.handling;
                let thrust =
                    pre_jump::fly(&mut self.player, handling, bearing, !self.stats.fast_jump);
                self.thrust(thrust);
                self.start_jump_if_ready(to, bearing);
            }
            Some(Jump::Hyperspace(_)) => return,
        }
        regenerate(&mut self.pilot.reserves.fuel, self.stats.fuel_regen);
    }

    /// Sounds the thrust starting or stopping as `thrust` says, and ramps
    /// the engine glow with it.
    fn thrust(&mut self, thrust: bool) {
        if thrust != self.thrusting {
            self.thrusting = thrust;
            self.sounds.push(if thrust {
                SimSound::ThrustStarted
            } else {
                SimSound::ThrustStopped
            });
        }
        self.engine_glow = ramp_glow(self.engine_glow, thrust);
    }

    /// Stops the thrust, if the ship was thrusting, and puts the engine
    /// glow out, even as it fades after the thrust, as the ship lands or
    /// jumps.
    fn stop_thrust(&mut self) {
        self.engine_glow = 0;
        if self.thrusting {
            self.thrusting = false;
            self.sounds.push(SimSound::ThrustStopped);
        }
    }

    /// Plots a course from the system the ship is in to `to`, replacing any
    /// course, and gives it: the session's [`HyperlinkRule`] decides which
    /// links it follows. When there is no route the course is cleared.
    pub fn plot_course(&mut self, to: SystemId) -> Result<&[SystemId], RouteError> {
        match self.star_map.route(self.pilot.system, to, self.hyperlinks) {
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

    /// Presses the jump key: accepts a jump to the next system on the
    /// course, if the ship has not landed and the
    /// [`hyperspace`](crate::hyperspace) rules allow it, and gives that
    /// system; otherwise the refusal says why.
    ///
    /// An accepted jump starts the pre-jump stage: the ship brakes and
    /// turns to the map bearing of that system as [`pre_jump`] says, and
    /// the jump itself begins on the tick it is ready, or here and now if
    /// it is ready already. Once accepted the jump is committed: J again
    /// during the stage gives the same system and changes nothing, without
    /// asking the rules again, so a ship that brakes into its no-jump zone
    /// still jumps. Until it arrives, ticks in hyperspace move nothing.
    /// Only this first hop of a multi-jump turns and slows: the rest are
    /// made in hyperspace, on arrival.
    pub fn begin_jump(&mut self) -> Result<SystemId, JumpRefusal> {
        if self.landed.is_some() {
            return Err(JumpRefusal::Landed);
        }
        if let Some(Jump::PreJump { to, .. }) = self.jump {
            return Ok(to);
        }
        let next = check_jump(
            &self.player,
            self.pilot.reserves.fuel.now,
            self.pilot.course.first().copied(),
            self.stats.jump_distance,
        )?;
        let map = |id| self.star_map.position(id);
        let bearing = map(self.pilot.system)
            .zip(map(next))
            .and_then(|(from, to)| jump_bearing(from, to));
        self.jump = Some(Jump::PreJump { to: next, bearing });
        self.start_jump_if_ready(next, bearing);
        Ok(next)
    }

    /// Begins the jump to `to`, if the ship is ready to turn no further
    /// towards `bearing` and slow no more: the thrust stops and the glow
    /// goes out.
    fn start_jump_if_ready(&mut self, to: SystemId, bearing: Option<f32>) {
        let handling = &self.stats.handling;
        if pre_jump::stage(&self.player, handling, bearing, !self.stats.fast_jump) == PreJump::Ready
        {
            self.jump = Some(Jump::Hyperspace(to));
            self.stop_thrust();
            self.sounds.push(SimSound::JumpBegan);
        }
    }

    /// The system being jumped to, once the jump itself has begun and
    /// until the ship arrives; `None` during the pre-jump stage.
    #[must_use]
    pub fn jumping(&self) -> Option<SystemId> {
        match self.jump {
            Some(Jump::Hyperspace(to)) => Some(to),
            _ => None,
        }
    }

    /// The system a jump is bound for while the ship brakes and turns
    /// before it (see [`Session::begin_jump`]); `None` before J and once
    /// the jump has begun.
    #[must_use]
    pub fn preparing_jump(&self) -> Option<SystemId> {
        match self.jump {
            Some(Jump::PreJump { to, .. }) => Some(to),
            _ => None,
        }
    }

    /// Ends the jump under way, if any, and gives the system arrived in.
    /// `None`, and nothing changes, when no jump has begun, during the
    /// pre-jump stage too.
    ///
    /// The jump makes as many hops along the course as
    /// [`hops_per_jump`] gives for the ship's multi-jump total and the
    /// session's [`MultiJumpRule`]: one without a multi-jump outfit. It
    /// chains only when its first hop was to the course's next system (a
    /// course replotted during the jump is not followed), and stops early
    /// where the course ends. Under [`MultiJumpRule::Engine`] the whole
    /// chain uses one jump's fuel and the days the stats give one jump;
    /// under [`MultiJumpRule::PerHop`] every hop does, and the chain stops
    /// at a hop the ship has not the fuel for. Each day steps the planetary
    /// events, rolled on `chance`.
    ///
    /// The systems hopped are taken off the course. Those passed through
    /// on the way are not entered; only the last is: explored, its
    /// stellars read from `catalog`, the navigation target cleared, and
    /// the ship placed just outside its no-jump zone, at rest on the side
    /// facing the system it last left (see [`arrival`]), with its reserves
    /// as they were, so it can jump on at once. It raises
    /// [`SimMessage::Arrived`] for that last system.
    pub fn arrive(
        &mut self,
        catalog: &impl PilotCatalog,
        chance: &mut (impl Chance + ?Sized),
    ) -> Option<SystemId> {
        let Some(Jump::Hyperspace(first)) = self.jump else {
            return None;
        };
        self.jump = None;
        self.take_jump_cost(chance);
        let (mut from, mut at) = (self.pilot.system, first);
        if self.pilot.course.first() == Some(&first) {
            self.pilot.course.remove(0);
            for _ in 1..hops_per_jump(self.stats.multi_jump, self.multi_jump) {
                let Some(&next) = self.pilot.course.first() else {
                    break;
                };
                if self.multi_jump == MultiJumpRule::PerHop {
                    if self.pilot.reserves.fuel.now < JUMP_FUEL {
                        break;
                    }
                    self.take_jump_cost(chance);
                }
                self.pilot.course.remove(0);
                (from, at) = (at, next);
            }
        }
        let map = |id| self.star_map.position(id).unwrap_or_default();
        self.player = arrival(map(from), map(at), self.stats.jump_distance);
        let pilot = &mut self.pilot;
        pilot.system = at;
        pilot.stellar = None;
        pilot.explore(at);
        self.sites = catalog.landing_sites(at);
        self.nav_target = None;
        self.sounds.push(SimSound::Arrived);
        self.messages.push(SimMessage::Arrived(at));
        Some(at)
    }

    /// Uses a jump's fuel and lets its days go by ([`Session::pass_jump_days`]).
    fn take_jump_cost(&mut self, chance: &mut (impl Chance + ?Sized)) {
        self.pilot.reserves.fuel.now -= JUMP_FUEL;
        self.pass_jump_days(chance);
    }

    /// Lets the days the stats give a jump go by, each stepping the
    /// planetary events, rolled on `chance`.
    fn pass_jump_days(&mut self, chance: &mut (impl Chance + ?Sized)) {
        let pilot = &mut self.pilot;
        for _ in 0..self.stats.jump_days {
            pilot.date = pilot.date.next_day();
            market::step_day(&self.goods, &mut pilot.events, chance);
        }
    }

    /// Selects the next of the system's stellars as the navigation target,
    /// as the [`navigation`](crate::navigation) rule says, and gives it;
    /// `None`, and nothing is selected, when the system has no stellars.
    pub fn select_next_stellar(&mut self) -> Option<StellarId> {
        let stellars: Vec<StellarId> = self.sites.iter().map(|site| site.id).collect();
        self.nav_target = next_stellar(&stellars, self.nav_target);
        self.nav_target
    }

    /// Presses Hyper Select (the original's `\` key): plots a one-jump
    /// course to the next system the session's [`HyperSelectRule`] cycles
    /// to ([`next_hyper_destination`]) and gives it, clearing the stellar
    /// navigation target, as the original switches its nav selection to
    /// hyperspace. `None`, and nothing changes, during a jump (pre-jump
    /// stage too) or when the system has no links to offer.
    ///
    /// The original keeps the plotted route and only moves its selection,
    /// so cycling back to the route's first hop there keeps the whole
    /// route; here the course is the jump target, so a press always
    /// leaves a one-jump course.
    pub fn select_next_system(&mut self) -> Option<SystemId> {
        if self.jump.is_some() {
            return None;
        }
        let next = next_hyper_destination(
            &self.star_map,
            self.pilot.system,
            &self.pilot.course,
            self.nav_target.is_some(),
            self.hyper_select,
            self.hyperlinks,
        )?;
        self.pilot.course = vec![next];
        self.nav_target = None;
        Some(next)
    }

    /// The stellar selected as the navigation target, if any.
    #[must_use]
    pub fn nav_target(&self) -> Option<StellarId> {
        self.nav_target
    }

    /// The sounds emitted since they were last taken, in order; taking
    /// them empties the list.
    pub fn take_sounds(&mut self) -> Vec<SimSound> {
        std::mem::take(&mut self.sounds)
    }

    /// The messages raised since they were last taken, in order; taking
    /// them empties the list.
    pub fn take_messages(&mut self) -> Vec<SimMessage> {
        std::mem::take(&mut self.messages)
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

    /// Today's date as it is displayed, with the first `chär`'s prefix and
    /// suffix: for example "June 23, 1177 NC".
    #[must_use]
    pub fn date_text(&self) -> String {
        date::date_text(self.pilot.date, &self.date_affixes)
    }

    /// The fuel the ship gains each tick in flight.
    #[must_use]
    pub fn fuel_regen_per_tick(&self) -> f32 {
        self.stats.fuel_regen
    }

    /// Presses the land key, unless a jump is under way, its pre-jump stage
    /// included: as the
    /// [`landing`](crate::landing) rules say, with the navigation target
    /// and the pilot's legal record with the stellar's government, or the
    /// system's on the star map when the stellar has none. A press that
    /// selects a stellar makes it the navigation target and lands on
    /// nothing. A press that lands docks the ship at the stellar's centre,
    /// at rest, its heading and reserves unchanged, and clears the target,
    /// so that once it takes off L requests clearance again. Otherwise the
    /// ship flies on, its target kept, and the refusal says why.
    ///
    /// A press that would land on a hypergate or wormhole
    /// ([`GateKind::of`] its `Flags2`) enters it instead: it does not dock
    /// and makes no sound, and gives [`LandPress::AtGate`]. The entry
    /// then awaits [`Session::enter_hypergate`] or
    /// [`Session::enter_wormhole`], until the next tick.
    pub fn land(&mut self) -> Result<LandPress, LandingRefusal> {
        if self.jump.is_some() {
            return Err(LandingRefusal::Jumping);
        }
        let outcome = land_or_select(
            self.nav_target,
            &self.player,
            &self.sites,
            self.star_map.govt(self.pilot.system),
            |govt| self.pilot.legal_record(govt),
        )?;
        match outcome {
            LandOutcome::Selected { stellar, .. } => self.nav_target = Some(stellar),
            LandOutcome::Landed(stellar) => {
                if let Some(kind) = self.gate_kind(stellar) {
                    self.gate = Some(stellar);
                    return Ok(LandPress::AtGate { stellar, kind });
                }
                self.dock(stellar);
            }
        }
        Ok(outcome.into())
    }

    /// What `stellar`, one of the system's, leads through, if it is a
    /// hypergate or wormhole.
    #[must_use]
    pub fn gate_kind(&self, stellar: StellarId) -> Option<GateKind> {
        let site = self.sites.iter().find(|site| site.id == stellar)?;
        GateKind::of(site.flags2)
    }

    /// The pending entry, if it is through a gate of `kind`; it ends here
    /// whatever it is.
    fn take_gate(&mut self, kind: GateKind) -> Option<StellarId> {
        self.gate
            .take()
            .filter(|&stellar| self.gate_kind(stellar) == Some(kind))
    }

    /// The links of `stellar` among `sites`; none when no system lists it.
    fn links_of(sites: &[GateSite], stellar: StellarId) -> [Option<StellarId>; 8] {
        sites
            .iter()
            .find(|site| site.id == stellar)
            .map_or([None; 8], |site| site.links)
    }

    /// Decides whether the hypergate whose entry is pending opens the
    /// hypergate map, read from `catalog`'s gate sites. A hypergate with
    /// links ([`has_links`], as `_StellarNumHyperLinks` counts them) offers
    /// the systems its links lead to ([`gate::hypergate_destinations`]) for
    /// the player to choose among, and the entry awaits
    /// [`Session::enter_hypergate`]: even when none leads anywhere, the
    /// map opens with nothing to pick, and closing it cancels. One without
    /// links is refused ([`GateRefusal::NoLinks`]), which ends the entry
    /// and clears the navigation target, as [`Session::enter_hypergate`]
    /// refuses. [`GateRefusal::NotAtGate`] when no hypergate entry is
    /// pending.
    pub fn open_hypergate(
        &mut self,
        catalog: &impl PilotCatalog,
    ) -> Result<Vec<SystemId>, GateRefusal> {
        let here = self
            .gate
            .filter(|&stellar| self.gate_kind(stellar) == Some(GateKind::Hypergate))
            .ok_or(GateRefusal::NotAtGate)?;
        let sites = catalog.gate_sites();
        let links = Self::links_of(&sites, here);
        if has_links(&links) {
            Ok(gate::hypergate_destinations(&links, &sites))
        } else {
            self.gate = None;
            self.nav_target = None;
            Err(GateRefusal::NoLinks)
        }
    }

    /// Enters the hypergate whose entry is pending, for the system `to`
    /// the player picked on the map (`None` for no pick), and gives the
    /// system the ship comes out in: through the first of its links that
    /// leads there ([`hypergate_exit`]), read from `catalog`. Any outcome
    /// ends the entry; a refusal also clears the navigation target, as the
    /// original's does, and otherwise changes nothing. See
    /// [`Session::enter_wormhole`] for the arrival.
    pub fn enter_hypergate(
        &mut self,
        to: Option<SystemId>,
        catalog: &impl PilotCatalog,
        chance: &mut (impl Chance + ?Sized),
    ) -> Result<SystemId, GateRefusal> {
        let here = self
            .take_gate(GateKind::Hypergate)
            .ok_or(GateRefusal::NotAtGate)?;
        let sites = catalog.gate_sites();
        let links = Self::links_of(&sites, here);
        let exit = if has_links(&links) {
            to.and_then(|to| Some((to, *hypergate_exit(&links, &sites, to, &self.star_map)?)))
                .ok_or(GateRefusal::Cancelled)
        } else {
            Err(GateRefusal::NoLinks)
        };
        match exit {
            Ok((system, exit)) => {
                self.come_out(system, &exit, catalog, chance);
                self.messages.push(SimMessage::ExitedHypergate(system));
                Ok(system)
            }
            Err(refusal) => {
                self.nav_target = None;
                Err(refusal)
            }
        }
    }

    /// Enters the wormhole whose entry is pending and gives the system the
    /// ship comes out in: through the wormhole [`wormhole_exit`] rolls on
    /// `chance`, under the session's [`WormholeRule`] (the engine's unless
    /// [`Session::with_wormholes`] says otherwise), read from `catalog`.
    /// Any outcome ends the entry; a refusal changes nothing else.
    ///
    /// Out of a hypergate or wormhole the ship comes out as the session's
    /// [`GateArrivalRule`] says (the engine's unless
    /// [`Session::with_gate_arrival`] says otherwise): by the engine's at
    /// the exit gate, as [`emerge`] says, with no day passing; like a jump,
    /// where a jump between the two systems arrives, with a jump's days
    /// passing, each stepping the planetary events on `chance`. Either way
    /// no fuel is used, the minimum jump distance does not count, and the
    /// ship is in the new system, explored, its stellars read, with the
    /// course and the navigation target cleared, its thrust stopped and
    /// [`SimSound::Arrived`]. It raises [`SimMessage::PassedWormhole`] or
    /// [`SimMessage::ExitedHypergate`].
    pub fn enter_wormhole(
        &mut self,
        catalog: &impl PilotCatalog,
        chance: &mut (impl Chance + ?Sized),
    ) -> Result<SystemId, GateRefusal> {
        let here = self
            .take_gate(GateKind::Wormhole)
            .ok_or(GateRefusal::NotAtGate)?;
        let sites = catalog.gate_sites();
        let exit = *wormhole_exit(here, self.pilot.system, &sites, self.wormholes, chance)
            .ok_or(GateRefusal::NoExit)?;
        self.come_out(exit.system, &exit, catalog, chance);
        self.messages.push(SimMessage::PassedWormhole(exit.system));
        Ok(exit.system)
    }

    /// Brings the ship out of `exit` in `system`: see
    /// [`Session::enter_wormhole`].
    fn come_out(
        &mut self,
        system: SystemId,
        exit: &GateSite,
        catalog: &impl PilotCatalog,
        chance: &mut (impl Chance + ?Sized),
    ) {
        self.player = match self.gate_arrival {
            GateArrivalRule::Engine => emerge(exit, self.stats.handling.max_speed, chance),
            GateArrivalRule::LikeJump => {
                let map = |id| self.star_map.position(id).unwrap_or_default();
                let placed = arrival(
                    map(self.pilot.system),
                    map(system),
                    self.stats.jump_distance,
                );
                self.pass_jump_days(chance);
                placed
            }
        };
        let pilot = &mut self.pilot;
        pilot.system = system;
        pilot.stellar = None;
        pilot.explore(system);
        pilot.course.clear();
        self.sites = catalog.landing_sites(system);
        self.nav_target = None;
        self.stop_thrust();
        self.sounds.push(SimSound::Arrived);
    }

    /// Docks the ship at `stellar`, one of the system's.
    fn dock(&mut self, stellar: StellarId) {
        self.nav_target = None;
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
    }

    /// Takes off from the stellar the ship is docked at, and gives it; the
    /// ship flies again from the stellar's centre, at rest. `None`, and
    /// nothing changes, when it has not landed.
    pub fn take_off(&mut self) -> Option<StellarId> {
        let stellar = self.landed.take()?;
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

    /// Whether the ship flew its last tick under thrust. Landing and
    /// beginning a jump stop it, as they stop the thrust sound.
    #[must_use]
    pub fn thrusting(&self) -> bool {
        self.thrusting
    }

    /// The engine glow's base level, as [`glow`](crate::glow) says: each
    /// tick in flight thrust raises it by 1 up to
    /// [`GLOW_CRUISE`](crate::GLOW_CRUISE) and coasting lowers it by 1 to
    /// 0, so it stays within 0 to 24. It starts at 0, and landing and
    /// beginning a jump put it back to 0.
    #[must_use]
    pub fn engine_glow(&self) -> u8 {
        self.engine_glow
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
    use crate::catalog::{CharacterStart, GateSite, LandingSite, SoundId, StellarId};
    use crate::catalog::{CommodityStrings, DisasterId, DisasterRecord, JunkId, JunkRecord};
    use crate::chance::NeverFires;
    use crate::clock::TICKS_PER_SECOND;
    use crate::flight::Turn;
    use crate::fuel::FUEL_SCOOP;
    use crate::gate::{GateArrivalRule, GateKind, GateRefusal, HYPERGATE, WORMHOLE, WormholeRule};
    use crate::geometry::Vec2;
    use crate::glow::GLOW_CRUISE;
    use crate::handling::ShipFields;
    use crate::hyperspace::{
        ARRIVAL_DISTANCE, HyperSelectRule, HyperlinkRule, JumpRefusal, MIN_JUMP_DISTANCE,
        RouteError, StarMap,
    };
    use crate::landing::StellarFlags;
    use crate::landing::{Clearance, LandOutcome, LandingRefusal};
    use crate::market::{Direction, Good, Lot, Order, TradeRefusal};
    use crate::pre_jump::slow_enough;
    use crate::reserves::{Gauge, Reserves};
    use crate::stats::{
        FAST_JUMP, FAST_JUMP_HULL, HYPERSPACE_DAYS, HYPERSPACE_DISTANCE, MULTI_JUMP,
    };
    use crate::testkit::{
        FAST, FakePilotCatalog, START, Scripted, begin_jump_now, catalog, edge_lander, fly_out,
        jump, jump_with, land_now, outfit, planet, star, starting,
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
        land_now(&mut session).expect("lands");
        session.take_off();
        land_now(&mut session).expect("lands again");
        assert_eq!(*catalog.sites_asked.borrow(), [SystemId(130)]);
        let other = starting([Some(131), None, None, None]);
        let mut session = Session::start(&other).expect("starts");
        assert_eq!(*other.sites_asked.borrow(), [SystemId(131)]);
        assert_eq!(land_now(&mut session), Ok(StellarId(140)));
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
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
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
        land_now(&mut session).expect("lands");
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
        land_now(&mut session).expect("lands");
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
        let refusal = land_now(&mut session);
        assert_eq!(session.nav_target(), Some(StellarId(128)), "kept");
        assert_eq!(
            refusal,
            crate::landing::check_landing(&flying, &planet(128, 30.0, -40.0), None, |_| 0)
                .map(|_| StellarId(128))
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
        assert_eq!(land_now(&mut session), Err(LandingRefusal::NoStellars));
    }

    #[test]
    fn the_first_l_selects_the_nearest_landable_stellar_and_does_not_land() {
        let mut session = Session::start(&catalog()).expect("starts");
        let before = *session.player();
        assert_eq!(
            session.land(),
            Ok(LandOutcome::Selected {
                stellar: StellarId(128),
                station: false,
                clearance: Clearance::Granted,
            }
            .into())
        );
        assert_eq!(session.nav_target(), Some(StellarId(128)));
        assert_eq!(
            session.landed(),
            None,
            "over it and at rest, yet not landed"
        );
        assert_eq!(session.pilot().stellar(), None);
        assert_eq!(*session.player(), before);
        assert!(!session.take_save_due());
        assert_eq!(session.take_sounds(), []);
    }

    #[test]
    fn the_second_l_lands_on_the_target_and_clears_it() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.land().expect("selects");
        assert_eq!(
            session.land(),
            Ok(LandOutcome::Landed(StellarId(128)).into())
        );
        assert_eq!(session.landed(), Some(StellarId(128)));
        assert_eq!(session.nav_target(), None, "cleared on landing");
        assert!(session.take_save_due());
        assert_eq!(
            session.take_sounds(),
            [SimSound::Landed {
                stellar_sound: None
            }]
        );
        // Once off again, L requests clearance afresh.
        session.take_off();
        assert!(matches!(
            session.land(),
            Ok(LandPress::Outcome(LandOutcome::Selected { .. }))
        ));
        assert_eq!(session.landed(), None);
    }

    #[test]
    fn l_with_a_target_selected_by_tab_lands_on_it_at_once() {
        let mut session = Session::start(&catalog()).expect("starts");
        assert_eq!(session.select_next_stellar(), Some(StellarId(128)));
        assert_eq!(
            session.land(),
            Ok(LandOutcome::Landed(StellarId(128)).into())
        );
        // Planet 129 is far away: L refuses and keeps it.
        session.take_off();
        session.select_next_stellar();
        assert_eq!(session.select_next_stellar(), Some(StellarId(129)));
        assert_eq!(
            session.land(),
            Err(LandingRefusal::TooFar {
                stellar: StellarId(129),
                station: false,
            })
        );
        assert_eq!(session.nav_target(), Some(StellarId(129)));
        assert_eq!(session.landed(), None);
    }

    #[test]
    fn l_reads_the_pilots_record_for_the_clearance_it_gives() {
        let catalog = governed([Some(WANTED), Some(LAWFUL)], None);
        let mut session = Session::fly(&catalog, on_record(&catalog)).expect("flies");
        assert_eq!(
            session.land(),
            Ok(LandOutcome::Selected {
                stellar: StellarId(128),
                station: false,
                clearance: Clearance::Denied,
            }
            .into())
        );
        assert_eq!(session.nav_target(), Some(StellarId(128)), "still selected");
        let catalog = governed([None, None], Some(LAWFUL));
        let mut session = Session::fly(&catalog, on_record(&catalog)).expect("flies");
        assert!(matches!(
            session.land(),
            Ok(LandPress::Outcome(LandOutcome::Selected {
                clearance: Clearance::Granted,
                ..
            }))
        ));
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
        land_now(&mut session)
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
    fn the_date_reads_with_the_chärs_affixes_and_moves_on_with_each_jump() {
        let catalog = FakePilotCatalog {
            date_affixes: DateAffixes {
                prefix: "Year ".to_owned(),
                suffix: " NC".to_owned(),
            },
            ..catalog()
        };
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(session.date_text(), "Year June 23, 1177 NC");
        jump(&mut session, &catalog, 131);
        assert_eq!(session.date_text(), "Year June 24, 1177 NC");
        jump(&mut session, &catalog, 132);
        assert_eq!(session.date_text(), "Year June 25, 1177 NC");
        assert_eq!(*catalog.date_affix_reads.borrow(), 1);
    }

    #[test]
    fn a_flown_pilot_reads_its_date_with_the_chärs_affixes() {
        let catalog = catalog();
        let pilot = Pilot::new(&catalog, "Flown").expect("starts");
        let session = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(session.date_text(), "June 23, 1177 NC");
        assert_eq!(*catalog.date_affix_reads.borrow(), 1);
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
        assert_eq!(begin_jump_now(&mut session), Ok(SystemId(131)));
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
        begin_jump_now(&mut session).expect("jumps");
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(131))
        );
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(session.course(), ids(&[132]));
        assert_eq!(session.jumping(), None);
        assert_eq!(
            *session.player(),
            crate::hyperspace::arrival(
                Vec2::ZERO,
                Vec2::new(600.0, 0.0),
                crate::hyperspace::MIN_JUMP_DISTANCE
            )
        );
        assert_eq!(session.player().position, Vec2::new(-1001.0, 0.0));
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
        let refused = land_now(&mut session);
        assert!(
            matches!(refused, Err(LandingRefusal::TooFar { stellar, .. }) if stellar == StellarId(140)),
            "{refused:?}"
        );
    }

    #[test]
    fn arriving_emits_an_arrived_message_naming_the_system() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        jump(&mut session, &catalog, 131);
        assert_eq!(
            session.take_messages(),
            [SimMessage::Arrived(SystemId(131))]
        );
        assert_eq!(session.take_messages(), [], "taking empties the list");
    }

    #[test]
    fn no_message_before_arrival() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        assert_eq!(session.arrive(&catalog, &mut NeverFires), None);
        assert_eq!(session.take_messages(), [], "no jump, no message");
        fly_out(&mut session);
        session.begin_jump().expect("pre-jump");
        session.tick(Controls::default());
        assert_eq!(session.preparing_jump(), Some(SystemId(131)));
        assert_eq!(session.take_messages(), [], "pre-jump");
        begin_jump_now(&mut session).expect("jumps");
        assert_eq!(session.take_messages(), [], "jumping");
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
            session.player().position,
            Vec2::new(-ARRIVAL_DISTANCE, 0.0),
            "resting at the edge"
        );
        assert_eq!(jump(&mut session, &catalog, 132), Some(SystemId(132)));
        assert_eq!(session.system(), SystemId(132));
        assert_eq!(session.course(), []);
        assert_eq!(session.reserves().fuel.now, 100.0);
        assert_eq!(dmy(&session), (25, 6, 1177));
        // From 131, north of 132 on screen: it arrives at the top edge.
        assert_eq!(session.player().position, Vec2::new(0.0, -1001.0));
        assert_eq!(session.begin_jump(), Err(JumpRefusal::NoDestination));
    }

    // The pre-jump stage.

    /// A session on course for 131, due east on the map, with the ship
    /// put at `position`, moving at `velocity` and facing `heading`.
    fn bound_for_131(
        catalog: &FakePilotCatalog,
        position: Vec2,
        velocity: Vec2,
        heading: f32,
    ) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        session.player = ShipState {
            position,
            velocity,
            heading,
        };
        session
    }

    /// Far enough out to jump, below the centre.
    const OUT: Vec2 = Vec2 { x: 0.0, y: 1200.0 };

    #[test]
    fn a_jump_begun_facing_away_turns_at_the_turn_rate_and_begins_facing_the_bearing() {
        // AC1: at rest facing up, 90 degrees off east, at 3 degrees a tick.
        let mut session = bound_for_131(&catalog(), OUT, Vec2::ZERO, 0.0);
        assert_eq!(session.begin_jump(), Ok(SystemId(131)));
        assert_eq!(session.preparing_jump(), Some(SystemId(131)));
        assert_eq!(session.jumping(), None);
        for k in 1..=29_u8 {
            session.tick(Controls::default());
            assert_eq!(session.player().heading, 3.0 * f32::from(k), "tick {k}");
            assert_eq!(session.jumping(), None, "tick {k}");
            assert_eq!(session.preparing_jump(), Some(SystemId(131)), "tick {k}");
        }
        session.tick(Controls::default());
        assert_eq!(session.player().heading, 90.0);
        assert_eq!(session.jumping(), Some(SystemId(131)), "facing the bearing");
        assert_eq!(session.preparing_jump(), None);
        assert_eq!(session.player().position, OUT, "at rest throughout");
    }

    /// Flies out with `catalog`'s ship, begins a jump and checks that it
    /// turns to the bearing and jumps without ever slowing down.
    fn assert_fast_jumps(catalog: &FakePilotCatalog) {
        let mut session = bound_for_131(catalog, Vec2::ZERO, Vec2::ZERO, 0.0);
        fly_out(&mut session);
        let racing = session.player().velocity.length();
        assert!(racing > 5.0, "{:?}", session.player());
        session.begin_jump().expect("jumps");
        for _ in 0..1000 {
            if session.jumping().is_some() {
                break;
            }
            session.tick(Controls::default());
            let now = *session.player();
            assert!(now.velocity.length() >= racing, "slowed: {now:?}");
        }
        assert_eq!(session.jumping(), Some(SystemId(131)));
        let leaving = *session.player();
        assert_eq!(leaving.heading, 90.0, "it still turned");
        assert!(!slow_enough(leaving.velocity), "{leaving:?}");
    }

    #[test]
    fn a_ship_with_a_fast_jump_outfit_jumps_without_slowing() {
        assert_fast_jumps(&FakePilotCatalog {
            defaults: vec![(ShipId(128), vec![(OutfitId(200), 1)])],
            outfits: vec![outfit(200, &[(FAST_JUMP, 1)])],
            ..catalog()
        });
    }

    #[test]
    fn a_fast_jumping_hull_jumps_without_slowing() {
        assert_fast_jumps(&FakePilotCatalog {
            ships: vec![(
                ShipId(128),
                Ok(ShipFields {
                    flags2: FAST_JUMP_HULL,
                    ..FAST
                }),
            )],
            ..catalog()
        });
    }

    #[test]
    fn a_jump_begun_while_moving_slows_the_ship_before_it_begins() {
        // AC2: fly_out leaves the ship racing north, facing it. A ship with
        // neither a fast-jump outfit nor a fast-jumping hull slows first.
        let mut session = bound_for_131(&catalog(), Vec2::ZERO, Vec2::ZERO, 0.0);
        fly_out(&mut session);
        let racing = session.player().velocity.length();
        assert!(racing > 5.0, "{:?}", session.player());
        session.begin_jump().expect("jumps");
        let mut slowest = racing;
        for _ in 0..1000 {
            if session.jumping().is_some() {
                break;
            }
            let before = *session.player();
            assert!(
                !(slow_enough(before.velocity) && before.heading == 90.0),
                "ready but still waiting: {before:?}"
            );
            session.tick(Controls::default());
            slowest = slowest.min(session.player().velocity.length());
        }
        assert_eq!(session.jumping(), Some(SystemId(131)));
        let leaving = *session.player();
        assert!(slow_enough(leaving.velocity), "{leaving:?}");
        assert!(leaving.velocity.length() < racing - 3.0, "{leaving:?}");
        assert_eq!(leaving.velocity.length(), slowest, "{leaving:?}");
        assert_eq!(leaving.heading, 90.0);
    }

    #[test]
    fn a_jump_begun_at_rest_facing_the_bearing_begins_at_once() {
        // AC3.
        let mut session = bound_for_131(&catalog(), OUT, Vec2::ZERO, 90.0);
        assert_eq!(session.begin_jump(), Ok(SystemId(131)));
        assert_eq!(session.jumping(), Some(SystemId(131)));
        assert_eq!(session.preparing_jump(), None);
        assert_eq!(session.take_sounds(), [SimSound::JumpBegan]);
    }

    #[test]
    fn a_jump_begun_drifting_slowly_facing_the_bearing_begins_at_once() {
        let drifting = Vec2::new(1.9, -1.9);
        let mut session = bound_for_131(&catalog(), OUT, drifting, 90.0);
        assert_eq!(session.begin_jump(), Ok(SystemId(131)));
        assert_eq!(session.jumping(), Some(SystemId(131)));
    }

    #[test]
    fn the_players_controls_are_ignored_during_the_pre_jump_stage() {
        let mut session = bound_for_131(&catalog(), Vec2::ZERO, Vec2::ZERO, 0.0);
        fly_out(&mut session);
        session.begin_jump().expect("jumps");
        let mut steered = session.clone();
        let fighting = Controls {
            thrust: true,
            turn: Turn::Left,
            reverse: false,
        };
        while session.jumping().is_none() {
            session.tick(Controls::default());
            steered.tick(fighting);
            assert_eq!(steered, session);
        }
    }

    #[test]
    fn braking_sounds_its_thrust_and_glows_and_the_jump_puts_both_out() {
        let mut session = bound_for_131(&catalog(), Vec2::ZERO, Vec2::ZERO, 0.0);
        fly_out(&mut session);
        session.tick(Controls::default());
        session.take_sounds();
        session.begin_jump().expect("jumps");
        assert_eq!(session.take_sounds(), [], "nothing until it begins");
        let mut sounds = Vec::new();
        let mut brightest = 0;
        while session.jumping().is_none() {
            session.tick(Controls::default());
            brightest = brightest.max(session.engine_glow());
            sounds.extend(session.take_sounds());
        }
        assert_eq!(sounds.first(), Some(&SimSound::ThrustStarted), "braking");
        assert_eq!(sounds.last(), Some(&SimSound::JumpBegan));
        let began = sounds.iter().filter(|s| **s == SimSound::JumpBegan).count();
        assert_eq!(began, 1);
        assert!(brightest > 0, "the brake glows");
        assert_eq!(session.engine_glow(), 0, "jumping");
        assert!(!session.thrusting());
    }

    #[test]
    fn a_jump_that_begins_mid_thrust_stops_the_thrust_first() {
        let mut session = bound_for_131(&catalog(), OUT, Vec2::ZERO, 90.0);
        session.tick(THRUST);
        session.take_sounds();
        assert_eq!(session.begin_jump(), Ok(SystemId(131)));
        assert_eq!(
            session.take_sounds(),
            [SimSound::ThrustStopped, SimSound::JumpBegan]
        );
        assert_eq!(session.engine_glow(), 0);
    }

    #[test]
    fn fuel_regenerates_during_the_pre_jump_stage() {
        let catalog = regenerating(2);
        let mut session = bound_for_131(&catalog, OUT, Vec2::ZERO, 0.0);
        session.pilot.reserves.fuel.now = 150.0;
        session.begin_jump().expect("jumps");
        for _ in 0..10 {
            session.tick(Controls::default());
        }
        assert_eq!(session.preparing_jump(), Some(SystemId(131)));
        assert_eq!(session.reserves().fuel.now, 155.0);
    }

    #[test]
    fn j_again_during_the_pre_jump_stage_changes_nothing() {
        let mut session = bound_for_131(&catalog(), OUT, Vec2::ZERO, 0.0);
        session.begin_jump().expect("jumps");
        session.tick(Controls::default());
        session.take_sounds();
        let turning = session.clone();
        assert_eq!(session.begin_jump(), Ok(SystemId(131)));
        assert_eq!(session, turning, "no restart, no sound");
    }

    #[test]
    fn a_jump_is_committed_even_once_braking_carries_the_ship_inside_the_no_jump_zone() {
        // Just outside the zone, racing inward and facing along its motion:
        // it coasts a long way in while it turns to brake.
        let edge = Vec2::new(0.0, -ARRIVAL_DISTANCE);
        let mut session = bound_for_131(&catalog(), edge, Vec2::new(0.0, 6.0), 180.0);
        assert_eq!(session.begin_jump(), Ok(SystemId(131)));
        for _ in 0..30 {
            session.tick(Controls::default());
        }
        let inside = session.player().position.length();
        assert!(inside < MIN_JUMP_DISTANCE, "inside: {inside}");
        assert_eq!(session.preparing_jump(), Some(SystemId(131)));
        session.take_sounds();
        let braking = session.clone();
        assert_eq!(session.begin_jump(), Ok(SystemId(131)), "J again");
        assert_eq!(session, braking, "no restart, no sound, not refused");
        begin_jump_now(&mut session).expect("jumps");
        assert_eq!(session.jumping(), Some(SystemId(131)), "still goes ahead");
    }

    #[test]
    fn landing_is_refused_during_the_pre_jump_stage() {
        let mut session = bound_for_131(&catalog(), Vec2::new(30.0, -40.0), Vec2::ZERO, 0.0);
        session.stats.jump_distance = 0.0;
        session.begin_jump().expect("jumps from over planet 128");
        session.take_sounds();
        let turning = session.clone();
        assert_eq!(land_now(&mut session), Err(LandingRefusal::Jumping));
        assert_eq!(session, turning, "nothing changes");
    }

    #[test]
    fn arriving_during_the_pre_jump_stage_does_nothing() {
        let catalog = catalog();
        let mut session = bound_for_131(&catalog, OUT, Vec2::ZERO, 0.0);
        session.begin_jump().expect("jumps");
        let turning = session.clone();
        assert_eq!(session.arrive(&catalog, &mut NeverFires), None);
        assert_eq!(session, turning);
        begin_jump_now(&mut session).expect("jumps");
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(131))
        );
    }

    #[test]
    fn the_jump_goes_where_it_was_bound_when_j_was_pressed() {
        let catalog = catalog();
        let mut session = bound_for_131(&catalog, OUT, Vec2::ZERO, 0.0);
        session.begin_jump().expect("jumps");
        assert_eq!(
            session.plot_course(SystemId(133)),
            Err(RouteError::Unreachable)
        );
        assert_eq!(session.course(), [], "the course is gone");
        session.tick(Controls::default());
        assert_eq!(session.preparing_jump(), Some(SystemId(131)));
        begin_jump_now(&mut session).expect("jumps");
        assert_eq!(session.jumping(), Some(SystemId(131)));
        assert_eq!(session.player().heading, 90.0, "the bearing to 131");
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(131))
        );
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
        assert_eq!(land_now(&mut session), Ok(StellarId(140)));
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
        assert_eq!(begin_jump_now(&mut session), Ok(SystemId(132)), "once off");
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
        begin_jump_now(&mut session).expect("jumps from over planet 140");
        let jumping = session.clone();
        assert_eq!(land_now(&mut session), Err(LandingRefusal::Jumping));
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
        assert_eq!(land_now(&mut session), Ok(StellarId(140)));
        for _ in 0..10 {
            session.tick(Controls::default());
        }
        assert_eq!(session.reserves().fuel.now, 200.0);
        session.take_off();
        session.tick(Controls::default());
        assert_eq!(session.reserves().fuel.now, 201.0);

        begin_jump_now(&mut session).expect("jumps from the edge");
        let leaving = session.reserves().fuel.now;
        for _ in 0..10 {
            session.tick(Controls::default());
        }
        assert_eq!(session.reserves().fuel.now, leaving, "in hyperspace");
    }

    // The pilot.

    #[test]
    fn landing_records_the_stellar_on_the_pilot_and_a_save_is_due() {
        let mut session = Session::start(&catalog()).expect("starts");
        assert!(!session.take_save_due(), "nothing to save yet");
        assert_eq!(session.pilot().stellar(), None);
        land_now(&mut session).expect("lands");
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
        land_now(&mut session).expect_err("refused");
        assert_eq!(session.pilot().stellar(), None);
        assert!(!session.take_save_due());
    }

    #[test]
    fn taking_off_keeps_the_stellar_and_a_save_is_due() {
        let mut session = Session::start(&catalog()).expect("starts");
        land_now(&mut session).expect("lands");
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
        land_now(&mut session).expect("lands");
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
        begin_jump_now(&mut session).expect("jumps from the edge");
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
        land_now(&mut session).expect("lands");
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
    fn the_session_is_thrusting_while_the_last_tick_flew_thrust() {
        let mut session = Session::start(&sounding()).expect("starts");
        assert!(!session.thrusting(), "at rest");
        session.tick(THRUST);
        assert!(session.thrusting());
        session.tick(Controls::default());
        assert!(!session.thrusting(), "coasting");
        session.tick(THRUST);
        session.land().expect("selects");
        assert!(matches!(
            session.land(),
            Ok(LandPress::Outcome(LandOutcome::Landed(_)))
        ));
        assert!(!session.thrusting(), "landed");
        session.tick(THRUST);
        assert!(!session.thrusting(), "docked, nothing thrusts");
    }

    #[test]
    fn beginning_a_jump_stops_the_session_thrusting() {
        // At rest facing east, the bearing to 131: the jump begins on J.
        let mut session = bound_for_131(&catalog(), OUT, Vec2::ZERO, 90.0);
        session.tick(THRUST);
        assert!(session.thrusting());
        session.begin_jump().expect("jumps");
        assert_eq!(session.jumping(), Some(SystemId(131)));
        assert!(!session.thrusting());
        session.tick(THRUST);
        assert!(!session.thrusting(), "jumping, nothing thrusts");
    }

    // The engine glow.

    #[test]
    fn the_engine_glow_ramps_up_with_thrust_and_down_after_it() {
        let mut session = Session::start(&catalog()).expect("starts");
        assert_eq!(session.engine_glow(), 0, "a new session");
        for k in 1..=30 {
            session.tick(THRUST);
            assert_eq!(session.engine_glow(), k.min(GLOW_CRUISE), "{k} ticks");
        }
        for k in 1..=30 {
            session.tick(Controls::default());
            assert_eq!(
                session.engine_glow(),
                GLOW_CRUISE.saturating_sub(k),
                "{k} ticks coasting"
            );
        }
    }

    /// A slow ship, at most half a pixel a tick, over a landable planet at
    /// the centre: it can thrust and still land.
    fn slow_lander() -> FakePilotCatalog {
        FakePilotCatalog {
            ships: vec![(ShipId(128), Ok(ShipFields { speed: 50, ..FAST }))],
            sites: vec![(SystemId(130), vec![planet(128, 0.0, 0.0)])],
            ..catalog()
        }
    }

    #[test]
    fn landing_puts_the_engine_glow_out_even_while_coasting() {
        let mut session = Session::start(&slow_lander()).expect("starts");
        for _ in 0..10 {
            session.tick(THRUST);
        }
        for _ in 0..2 {
            session.tick(Controls::default());
        }
        assert!(!session.thrusting());
        assert_eq!(session.engine_glow(), 8, "still glowing as it coasts");
        land_now(&mut session).expect("lands");
        assert_eq!(session.engine_glow(), 0, "landed");
        session.tick(THRUST);
        assert_eq!(session.engine_glow(), 0, "docked, nothing glows");
    }

    #[test]
    fn beginning_a_jump_puts_the_engine_glow_out() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        for _ in 0..5 {
            session.tick(THRUST);
        }
        session.tick(Controls::default());
        assert!(!session.thrusting());
        assert!(session.engine_glow() > 0, "glowing as it coasts");
        begin_jump_now(&mut session).expect("jumps");
        assert_eq!(session.engine_glow(), 0, "jumping");
        session.tick(THRUST);
        assert_eq!(session.engine_glow(), 0, "jumping, nothing glows");
        session.arrive(&catalog, &mut NeverFires).expect("arrives");
        assert_eq!(session.engine_glow(), 0, "arrived");
    }

    #[test]
    fn a_refused_jump_leaves_the_engine_glow_as_it_was() {
        let mut session = Session::start(&catalog()).expect("starts");
        for _ in 0..5 {
            session.tick(THRUST);
        }
        assert_eq!(session.begin_jump(), Err(JumpRefusal::NoDestination));
        assert_eq!(session.engine_glow(), 5);
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
        land_now(&mut session).expect("lands");
        assert_eq!(
            session.take_sounds(),
            [SimSound::Landed {
                stellar_sound: Some(SoundId(10_032))
            }]
        );
        let mut silent = Session::start(&catalog()).expect("starts");
        land_now(&mut silent).expect("lands");
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
        land_now(&mut session).expect("lands");
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
        land_now(&mut session).expect("lands");
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
        land_now(&mut session).expect_err("refused");
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
        // At rest facing east, the bearing to 131: the jump begins on J.
        let mut session = bound_for_131(&catalog, OUT, Vec2::ZERO, 90.0);
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
        // Turning at rest to the bearing, it never thrusts.
        let mut session = bound_for_131(&catalog(), OUT, Vec2::ZERO, 0.0);
        session.tick(Controls::default());
        session.take_sounds();
        begin_jump_now(&mut session).expect("jumps");
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
        land_now(&mut session).expect("lands");
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
        land_now(&mut session).expect("lands");
        let market = session.market().expect("an exchange");
        let prices: Vec<_> = market.rows.iter().map(|r| (r.good, r.price)).collect();
        assert_eq!(prices, [(FOOD, 75), (METAL, 160)]);
        assert_eq!((market.cash, market.capacity, market.free), (1000, 20, 20));
        session.take_off();
        assert_eq!(session.market(), None, "taken off");
        let mut plain = Session::start(&self::catalog()).expect("starts");
        land_now(&mut plain).expect("lands");
        assert_eq!(plain.market(), None, "no trade center");
    }

    #[test]
    fn buying_pays_loads_and_makes_a_save_due() {
        let mut session = Session::start(&exchange()).expect("starts");
        land_now(&mut session).expect("lands");
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
        land_now(&mut session).expect("lands");
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
        land_now(&mut session).expect("lands");
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
        land_now(&mut session).expect("lands");
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
        land_now(&mut session).expect("lands");
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
        assert_eq!(land_now(&mut session), Ok(StellarId(140)));
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
        assert_eq!(land_now(&mut session), Ok(StellarId(140)));
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
        land_now(&mut session).expect("lands");
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
            [(DisasterId(128), 4)],
            "a light hull's one day"
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
        assert_eq!(begin_jump_now(&mut session), Ok(SystemId(131)));
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
    fn after_arriving_and_a_second_of_flight_the_ship_can_jump_on() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        assert_eq!(jump(&mut session, &catalog, 132), Some(SystemId(131)));
        for _ in 0..TICKS_PER_SECOND {
            session.tick(Controls::default());
        }
        assert_eq!(session.begin_jump(), Ok(SystemId(132)));
    }

    #[test]
    fn with_a_raised_jump_distance_the_ship_arrives_outside_it_and_can_jump_on() {
        let raised = owning(catalog(), HYPERSPACE_DISTANCE, 250, 1);
        let mut session = Session::start(&raised).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        session.player.position = Vec2::new(0.0, 1300.0);
        assert_eq!(begin_jump_now(&mut session), Ok(SystemId(131)));
        assert_eq!(
            session.arrive(&raised, &mut NeverFires),
            Some(SystemId(131))
        );
        let distance = session.player().position.length();
        assert!(distance >= 1250.0, "outside the raised zone: {distance}");
        for _ in 0..TICKS_PER_SECOND {
            session.tick(Controls::default());
        }
        assert_eq!(session.begin_jump(), Ok(SystemId(132)));
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
    fn a_heavy_hull_jumps_for_three_days_and_the_dampener_takes_one_off() {
        let heavy = |catalog: FakePilotCatalog| FakePilotCatalog {
            ships: vec![(ShipId(128), Ok(ShipFields { mass: 250, ..FAST }))],
            ..catalog
        };
        let catalog = heavy(surplus());
        let mut session = Session::start(&catalog).expect("starts");
        let mut chance = Scripted::default();
        jump_with(&mut session, &catalog, 131, &mut chance);
        assert_eq!(
            chance.asked,
            [35, 35, 35],
            "one roll for each of three days"
        );
        assert_eq!(dmy(&session), (26, 6, 1177));
        let fuel = session.reserves().fuel;
        assert_eq!(fuel.max - fuel.now, 100.0, "one jump's fuel");

        // The stock Sutherland Alluvial Dampener: -1.
        let dampened = heavy(owning(surplus(), HYPERSPACE_DAYS, -1, 1));
        let mut session = Session::start(&dampened).expect("starts");
        let mut chance = Scripted::default();
        jump_with(&mut session, &dampened, 131, &mut chance);
        assert_eq!(chance.asked, [35, 35]);
        assert_eq!(dmy(&session), (25, 6, 1177));
    }

    // Multi-jump.

    /// `base` on a longer map: 130 (0, 0) links to 131 (600, 0) and 133
    /// (-600, 0), 131 to 132 (600, 600) and 132 to 134 (1200, 600), so the
    /// course to 134 is 131, 132, 134; ship 128 carries one outfit of
    /// multi-jump `mod_val`.
    fn chained(base: FakePilotCatalog, mod_val: i16) -> FakePilotCatalog {
        FakePilotCatalog {
            star_map: vec![
                star(130, (0.0, 0.0), &[131, 133]),
                star(131, (600.0, 0.0), &[132]),
                star(132, (600.0, 600.0), &[134]),
                star(133, (-600.0, 0.0), &[]),
                star(134, (1200.0, 600.0), &[]),
            ],
            ..owning(base, MULTI_JUMP, mod_val, 1)
        }
    }

    /// Plots a course to 134, flies out and begins the jump.
    fn bound_for_134(catalog: &FakePilotCatalog, rule: MultiJumpRule) -> Session {
        let mut session = Session::start(catalog)
            .expect("starts")
            .with_multi_jump(rule);
        session.plot_course(SystemId(134)).expect("a route");
        assert_eq!(session.course(), ids(&[131, 132, 134]));
        fly_out(&mut session);
        session
    }

    fn arrivals(session: &mut Session) -> usize {
        session
            .take_sounds()
            .into_iter()
            .filter(|sound| *sound == SimSound::Arrived)
            .count()
    }

    #[test]
    fn a_multi_jump_passes_modval_systems_along_the_course_for_one_jumps_fuel_and_days() {
        let catalog = chained(catalog(), 2);
        let mut session = bound_for_134(&catalog, MultiJumpRule::default());
        assert_eq!(begin_jump_now(&mut session), Ok(SystemId(131)));
        assert_eq!(session.jumping(), Some(SystemId(131)), "the first hop");
        session.take_sounds();
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(132))
        );
        assert_eq!(session.system(), SystemId(132));
        assert_eq!(session.course(), ids(&[134]));
        assert_eq!(session.jumping(), None);
        assert_eq!(session.reserves().fuel.now, 200.0, "one jump's fuel");
        assert_eq!(dmy(&session), (24, 6, 1177), "one jump's day");
        assert_eq!(
            session.pilot().explored().collect::<Vec<_>>(),
            [SystemId(130), SystemId(132)],
            "131 passed through"
        );
        assert_eq!(
            *catalog.sites_asked.borrow(),
            [SystemId(130), SystemId(132)]
        );
        assert_eq!(arrivals(&mut session), 1);
        assert_eq!(
            *session.player(),
            crate::hyperspace::arrival(
                Vec2::new(600.0, 0.0),
                Vec2::new(600.0, 600.0),
                MIN_JUMP_DISTANCE
            ),
            "facing the system it last left"
        );
        assert_eq!(session.player().position, Vec2::new(0.0, -1001.0));
    }

    #[test]
    fn a_multi_jump_emits_one_arrived_message_for_the_final_system() {
        let catalog = chained(catalog(), 10);
        let mut session = bound_for_134(&catalog, MultiJumpRule::default());
        begin_jump_now(&mut session).expect("jumps");
        session.arrive(&catalog, &mut NeverFires);
        assert_eq!(
            session.take_messages(),
            [SimMessage::Arrived(SystemId(134))]
        );
    }

    #[test]
    fn a_multi_jump_rolls_the_events_for_one_jumps_days() {
        let catalog = chained(surplus(), 2);
        let mut session = bound_for_134(&catalog, MultiJumpRule::default());
        begin_jump_now(&mut session).expect("jumps");
        let mut chance = Scripted::default();
        assert_eq!(session.arrive(&catalog, &mut chance), Some(SystemId(132)));
        assert_eq!(chance.asked, [35], "one roll for the one day");
    }

    #[test]
    fn the_chain_stops_where_the_course_ends() {
        // The stock Multi-Jumping Organ: 10.
        let catalog = chained(catalog(), 10);
        let mut session = bound_for_134(&catalog, MultiJumpRule::default());
        begin_jump_now(&mut session).expect("jumps");
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(134))
        );
        assert_eq!(session.course(), []);
        assert_eq!(session.reserves().fuel.now, 200.0);
        assert_eq!(dmy(&session), (24, 6, 1177));
        assert_eq!(
            session.player().position,
            Vec2::new(-1001.0, 0.0),
            "facing 132, west of 134"
        );
        assert_eq!(session.begin_jump(), Err(JumpRefusal::NoDestination));
    }

    #[test]
    fn an_engine_multi_jump_chains_on_with_no_fuel_left_after_the_first_hop() {
        let catalog = chained(catalog(), 10);
        let mut session = bound_for_134(&catalog, MultiJumpRule::default());
        session.pilot.reserves.fuel.now = 100.0;
        begin_jump_now(&mut session).expect("jumps");
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(134))
        );
        assert_eq!(session.course(), []);
        assert_eq!(session.reserves().fuel.now, 0.0, "one jump's fuel");
        assert_eq!(dmy(&session), (24, 6, 1177), "one jump's day");
    }

    #[test]
    fn a_multi_jump_modval_of_one_makes_a_single_hop() {
        let catalog = chained(catalog(), 1);
        let mut session = bound_for_134(&catalog, MultiJumpRule::default());
        begin_jump_now(&mut session).expect("jumps");
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(131))
        );
        assert_eq!(session.course(), ids(&[132, 134]));
    }

    #[test]
    fn a_course_replotted_during_the_jump_is_not_chained() {
        let catalog = chained(catalog(), 10);
        let mut session = bound_for_134(&catalog, MultiJumpRule::default());
        begin_jump_now(&mut session).expect("jumps");
        session.plot_course(SystemId(133)).expect("a route");
        assert_eq!(session.course(), ids(&[133]));
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(131))
        );
        assert_eq!(session.course(), ids(&[133]));
        assert_eq!(session.reserves().fuel.now, 200.0);
    }

    #[test]
    fn per_hop_makes_one_plus_modval_hops_each_taking_fuel_and_days() {
        let catalog = chained(catalog(), 1);
        let mut session = bound_for_134(&catalog, MultiJumpRule::PerHop);
        begin_jump_now(&mut session).expect("jumps");
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(132))
        );
        assert_eq!(session.course(), ids(&[134]));
        assert_eq!(session.reserves().fuel.now, 100.0, "two jumps' fuel");
        assert_eq!(dmy(&session), (25, 6, 1177), "two jumps' days");

        let catalog = chained(surplus(), 1);
        let mut session = bound_for_134(&catalog, MultiJumpRule::PerHop);
        begin_jump_now(&mut session).expect("jumps");
        let mut chance = Scripted::default();
        session.arrive(&catalog, &mut chance);
        assert_eq!(chance.asked, [35, 35], "one roll for each hop's day");
    }

    #[test]
    fn per_hop_stops_cleanly_when_a_hop_lacks_fuel() {
        let catalog = chained(catalog(), 10);
        let mut session = bound_for_134(&catalog, MultiJumpRule::PerHop);
        session.pilot.reserves.fuel.now = 250.0;
        begin_jump_now(&mut session).expect("jumps");
        session.take_sounds();
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(132))
        );
        assert_eq!(session.reserves().fuel.now, 50.0);
        assert_eq!(dmy(&session), (25, 6, 1177));
        assert_eq!(session.course(), ids(&[134]));
        assert_eq!(
            session.pilot().explored().collect::<Vec<_>>(),
            [SystemId(130), SystemId(132)]
        );
        assert_eq!(arrivals(&mut session), 1);
    }

    #[test]
    fn per_hop_takes_a_hop_with_exactly_a_jumps_fuel() {
        let catalog = chained(catalog(), 10);
        let mut session = bound_for_134(&catalog, MultiJumpRule::PerHop);
        session.pilot.reserves.fuel.now = 200.0;
        begin_jump_now(&mut session).expect("jumps");
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(132))
        );
        assert_eq!(session.reserves().fuel.now, 0.0);
    }

    #[test]
    fn per_hop_stops_where_the_course_ends() {
        let catalog = chained(catalog(), 10);
        let mut session = bound_for_134(&catalog, MultiJumpRule::PerHop);
        begin_jump_now(&mut session).expect("jumps");
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(134))
        );
        assert_eq!(session.course(), []);
        assert_eq!(session.reserves().fuel.now, 0.0);
        assert_eq!(dmy(&session), (26, 6, 1177));
    }

    #[test]
    fn a_pilot_flown_with_cargo_has_less_space_and_can_sell_it() {
        let catalog = exchange();
        let mut session = Session::start(&catalog).expect("starts");
        land_now(&mut session).expect("lands");
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
        land_now(&mut session).expect("lands");
        session.take_save_due();
        session
    }

    #[test]
    fn there_is_an_outfitter_only_while_landed_at_one() {
        let catalog = outfitting();
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(session.outfitter(), None, "in flight");
        assert_eq!(session.outfit(buy(SPEED)), Err(OutfitRefusal::NoOutfitter));
        land_now(&mut session).expect("lands");
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
        land_now(&mut session).expect("lands");
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
        land_now(&mut session).expect("lands");
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
        land_now(&mut session).expect("lands");
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

    // Navigation.

    /// [`catalog`] with system 130 holding three planets whose navigation
    /// order, 131, 128, 129, is not their order by distance from the
    /// centre, where the ship starts: 128 is nearest, then 131, then 129.
    fn three_stellars() -> FakePilotCatalog {
        let mut catalog = catalog();
        catalog.sites[0].1 = vec![
            planet(131, 900.0, 0.0),
            planet(128, 30.0, -40.0),
            planet(129, 2000.0, 0.0),
        ];
        catalog
    }

    #[test]
    fn a_session_starts_with_no_navigation_target() {
        let session = Session::start(&three_stellars()).expect("starts");
        assert_eq!(session.nav_target(), None);
    }

    #[test]
    fn tab_selects_the_systems_stellars_in_navigation_order_and_wraps() {
        let mut session = Session::start(&three_stellars()).expect("starts");
        for expected in [131, 128, 129, 131, 128] {
            assert_eq!(session.select_next_stellar(), Some(StellarId(expected)));
            assert_eq!(session.nav_target(), Some(StellarId(expected)));
        }
    }

    #[test]
    fn a_system_with_no_stellars_has_no_target_to_select() {
        let empty = FakePilotCatalog {
            sites: Vec::new(),
            ..catalog()
        };
        let mut session = Session::start(&empty).expect("starts");
        assert_eq!(session.select_next_stellar(), None);
        assert_eq!(session.nav_target(), None);
    }

    #[test]
    fn the_target_and_the_course_are_independent() {
        let mut session = Session::start(&three_stellars()).expect("starts");
        session.select_next_stellar();
        session.plot_course(SystemId(132)).expect("a route");
        assert_eq!(session.nav_target(), Some(StellarId(131)));
        session.select_next_stellar();
        assert_eq!(session.course(), ids(&[131, 132]));
        assert_eq!(session.nav_target(), Some(StellarId(128)));
    }

    #[test]
    fn selecting_a_target_makes_no_save_due() {
        let mut session = Session::start(&three_stellars()).expect("starts");
        session.select_next_stellar();
        assert!(!session.take_save_due());
    }

    #[test]
    fn arriving_in_another_system_clears_the_target() {
        let catalog = three_stellars();
        let mut session = Session::start(&catalog).expect("starts");
        session.select_next_stellar();
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        begin_jump_now(&mut session).expect("jumps");
        assert_eq!(
            session.nav_target(),
            Some(StellarId(131)),
            "kept while jumping"
        );
        session.arrive(&catalog, &mut NeverFires).expect("arrives");
        assert_eq!(session.nav_target(), None);
        assert_eq!(session.select_next_stellar(), Some(StellarId(140)));
    }

    // Hyper Select.

    /// [`three_stellars`] with 130 a hub: it lists 134, 131 (twice),
    /// itself, a missing 999 and 135, in that Con order; 136 lists 130
    /// one way; 131 lists 132; 133 lists nothing and exists too.
    fn hub() -> FakePilotCatalog {
        let mut catalog = three_stellars();
        catalog.star_map = vec![
            star(130, (0.0, 0.0), &[134, 131, 131, 130, 999, 135]),
            star(131, (600.0, 0.0), &[132]),
            star(132, (600.0, 600.0), &[]),
            star(133, (-600.0, 0.0), &[]),
            star(134, (0.0, -600.0), &[]),
            star(135, (0.0, 600.0), &[]),
            star(136, (-600.0, -600.0), &[130]),
        ];
        catalog.systems.push(SystemId(133));
        catalog
    }

    #[test]
    fn the_hyper_select_key_cycles_the_listed_systems_and_wraps() {
        let mut session = Session::start(&hub()).expect("starts");
        for expected in [134, 131, 135, 134] {
            assert_eq!(session.select_next_system(), Some(SystemId(expected)));
            assert_eq!(session.course(), ids(&[expected]));
        }
    }

    #[test]
    fn a_hyper_select_replaces_a_multi_jump_course_with_one_jump() {
        let mut session = Session::start(&hub()).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        assert_eq!(session.course(), ids(&[131, 132]));
        assert_eq!(session.select_next_system(), Some(SystemId(135)));
        assert_eq!(session.course(), ids(&[135]));
    }

    #[test]
    fn a_hyper_select_clears_the_stellar_nav_target() {
        let mut session = Session::start(&hub()).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        session.select_next_stellar();
        assert_eq!(
            session.select_next_system(),
            Some(SystemId(134)),
            "a stellar targeted: the first listed, not the one after 131"
        );
        assert_eq!(session.nav_target(), None);
        assert_eq!(session.select_next_system(), Some(SystemId(131)));
    }

    #[test]
    fn a_hyper_select_makes_no_save_due() {
        let mut session = Session::start(&hub()).expect("starts");
        session.select_next_system();
        assert!(!session.take_save_due());
    }

    #[test]
    fn a_jump_goes_to_the_selected_system() {
        let catalog = hub();
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(session.select_next_system(), Some(SystemId(134)));
        assert_eq!(session.select_next_system(), Some(SystemId(131)));
        fly_out(&mut session);
        assert_eq!(begin_jump_now(&mut session), Ok(SystemId(131)));
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(131))
        );
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(session.course(), []);
    }

    #[test]
    fn no_hyper_select_during_a_jump() {
        let mut session = Session::start(&hub()).expect("starts");
        session.select_next_system();
        fly_out(&mut session);
        session.player.heading = 90.0;
        assert_eq!(session.begin_jump(), Ok(SystemId(134)));
        assert_eq!(session.preparing_jump(), Some(SystemId(134)));
        assert_eq!(session.select_next_system(), None, "pre-jump");
        assert_eq!(session.course(), ids(&[134]));
        begin_jump_now(&mut session).expect("jumps");
        assert_eq!(session.jumping(), Some(SystemId(134)));
        assert_eq!(session.select_next_system(), None, "in hyperspace");
        assert_eq!(session.course(), ids(&[134]));
    }

    #[test]
    fn a_system_with_no_listed_links_selects_nothing() {
        let mut catalog = hub();
        catalog.character = starting([Some(133), None, None, None]).character;
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(session.system(), SystemId(133));
        assert_eq!(session.select_next_system(), None);
        assert_eq!(session.course(), []);
    }

    #[test]
    fn plotting_a_course_follows_one_way_links_by_the_engines_reading() {
        // 136 lists 130 one way: by the engine, 130 cannot jump to it.
        let mut session = Session::start(&hub()).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        assert_eq!(
            session.plot_course(SystemId(136)),
            Err(RouteError::Unreachable)
        );
        assert_eq!(session.course(), [], "cleared");
        let mut session = Session::start(&hub())
            .expect("starts")
            .with_hyperlinks(HyperlinkRule::BothWays);
        assert_eq!(session.plot_course(SystemId(136)), Ok(&ids(&[136])[..]));
    }

    #[test]
    fn hyper_select_by_the_one_jump_course_reading_never_jumps_against_a_one_way_link() {
        let cycle = |hyperlinks| {
            let mut session = Session::start(&hub())
                .expect("starts")
                .with_hyper_select(HyperSelectRule::OneJumpCourse)
                .with_hyperlinks(hyperlinks);
            (0..4)
                .map(|_| session.select_next_system().expect("one"))
                .collect::<Vec<_>>()
        };
        assert_eq!(cycle(HyperlinkRule::Engine), ids(&[131, 134, 135, 131]));
        assert_eq!(cycle(HyperlinkRule::BothWays), ids(&[131, 134, 135, 136]));
    }

    #[test]
    fn hyper_select_by_the_one_jump_course_reading_cycles_every_jump() {
        let mut session = Session::start(&hub())
            .expect("starts")
            .with_hyper_select(HyperSelectRule::OneJumpCourse);
        assert_eq!(session.select_next_system(), Some(SystemId(131)));
        assert_eq!(session.select_next_system(), Some(SystemId(134)));
        session.plot_course(SystemId(132)).expect("a route");
        assert_eq!(
            session.select_next_system(),
            Some(SystemId(131)),
            "a multi-jump course: the first"
        );
    }

    // Hypergates and wormholes.

    /// Stellar `id` at (`x`, `y`) in `system`, with `flags2`, its `links`
    /// and its exit angle.
    fn gate_site(
        id: i16,
        system: i16,
        (x, y): (f32, f32),
        flags2: u16,
        links: &[i16],
        exit_angle: i16,
    ) -> GateSite {
        let mut slots = [None; 8];
        for (slot, &link) in slots.iter_mut().zip(links) {
            *slot = Some(StellarId(link));
        }
        GateSite {
            id: StellarId(id),
            system: SystemId(system),
            position: Vec2::new(x, y),
            flags2,
            links: slots,
            exit_angle,
        }
    }

    /// A landable stellar `id` at the centre with `flags2`.
    fn gate_landing(id: i16, flags2: u16) -> LandingSite {
        LandingSite {
            flags2,
            ..planet(id, 0.0, 0.0)
        }
    }

    /// The ship starts over hypergate 300, at system 130's centre, linked
    /// to 310 in 131 at (100, 200), heading ships out on 90°, then to 320
    /// in 132 (a random heading), then to 999, which no system lists.
    fn gated() -> FakePilotCatalog {
        FakePilotCatalog {
            sites: vec![
                (SystemId(130), vec![gate_landing(300, HYPERGATE)]),
                (SystemId(131), vec![planet(140, 0.0, 0.0)]),
            ],
            gates: vec![
                gate_site(300, 130, (0.0, 0.0), 0x1200, &[310, 320, 999], 120),
                gate_site(310, 131, (100.0, 200.0), HYPERGATE, &[300], 90),
                gate_site(320, 132, (-50.0, 0.0), HYPERGATE, &[], -1),
            ],
            ..catalog()
        }
    }

    /// [`gated`]'s session, with L pressed twice over the hypergate: the
    /// entry awaits the player's pick.
    fn at_gate(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        assert!(matches!(
            session.land(),
            Ok(LandPress::Outcome(LandOutcome::Selected { .. }))
        ));
        assert_eq!(
            session.land(),
            Ok(LandPress::AtGate {
                stellar: StellarId(300),
                kind: GateKind::Hypergate,
            })
        );
        session
    }

    #[test]
    fn l_twice_over_a_cleared_hypergate_awaits_the_entry_and_does_not_dock() {
        let catalog = gated();
        let mut session = at_gate(&catalog);
        assert_eq!(session.landed(), None);
        assert_eq!(session.pilot().stellar(), None);
        assert!(!session.take_save_due());
        assert!(
            !session
                .take_sounds()
                .iter()
                .any(|sound| matches!(sound, SimSound::Landed { .. })),
            "no landing sound"
        );
        assert_eq!(
            session.open_hypergate(&catalog),
            Ok(ids(&[131, 132])),
            "each linked gate's system, in slot order"
        );
        assert_eq!(session.gate_kind(StellarId(300)), Some(GateKind::Hypergate));
        assert_eq!(session.gate_kind(StellarId(310)), None, "not here");
    }

    #[test]
    fn no_destinations_are_offered_without_an_entry_pending() {
        let catalog = gated();
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(
            session.open_hypergate(&catalog),
            Err(GateRefusal::NotAtGate)
        );
        assert_eq!(*catalog.gate_reads.borrow(), 0, "not read at the start");
    }

    #[test]
    fn a_tick_ends_a_pending_entry() {
        let catalog = gated();
        let mut session = at_gate(&catalog);
        session.tick(Controls::default());
        assert_eq!(
            session.open_hypergate(&catalog),
            Err(GateRefusal::NotAtGate)
        );
        assert_eq!(
            session.enter_hypergate(Some(SystemId(131)), &catalog, &mut NeverFires),
            Err(GateRefusal::NotAtGate)
        );
        assert_eq!(session.system(), SystemId(130));
    }

    #[test]
    fn entering_a_hypergate_comes_out_of_the_gate_picked_at_half_speed() {
        let catalog = gated();
        let mut session = at_gate(&catalog);
        session.plot_course(SystemId(132)).expect("a route");
        session.take_sounds();
        let mut chance = Scripted::default();
        assert_eq!(
            session.enter_hypergate(Some(SystemId(131)), &catalog, &mut chance),
            Ok(SystemId(131))
        );
        assert_eq!(session.system(), SystemId(131));
        let player = *session.player();
        assert_eq!(player.position, Vec2::new(100.0, 200.0));
        assert_eq!(player.heading, 90.0);
        let half = session.handling().max_speed / 2.0;
        assert!((player.velocity - Vec2::new(half, 0.0)).length() < 1e-4);
        assert!(session.pilot().has_explored(SystemId(131)));
        assert_eq!(session.pilot().stellar(), None);
        assert_eq!(session.course(), [], "the course cleared");
        assert_eq!(session.nav_target(), None);
        assert_eq!(session.reserves().fuel, Gauge::full(300.0), "no fuel");
        assert_eq!(dmy(&session), (23, 6, 1177), "no day passes");
        assert!(chance.asked.is_empty() && chance.sides_asked.is_empty());
        assert_eq!(session.take_sounds(), [SimSound::Arrived]);
        assert_eq!(
            session.take_messages(),
            [SimMessage::ExitedHypergate(SystemId(131))]
        );
        assert_eq!(
            catalog.sites_asked.borrow().last(),
            Some(&SystemId(131)),
            "the new system's stellars read"
        );
        assert_eq!(
            session.enter_hypergate(Some(SystemId(130)), &catalog, &mut chance),
            Err(GateRefusal::NotAtGate),
            "the entry is over"
        );
    }

    #[test]
    fn a_gate_with_an_angle_outside_0_to_359_heads_the_ship_out_at_random() {
        let catalog = gated();
        let mut session = at_gate(&catalog);
        let mut chance = Scripted::rolling(&[270]);
        assert_eq!(
            session.enter_hypergate(Some(SystemId(132)), &catalog, &mut chance),
            Ok(SystemId(132))
        );
        assert_eq!(chance.sides_asked, [360]);
        assert_eq!(session.player().heading, 270.0);
        assert_eq!(session.player().position, Vec2::new(-50.0, 0.0));
    }

    #[test]
    fn no_pick_or_one_no_link_leads_to_cancels_and_clears_the_target() {
        let catalog = gated();
        for to in [None, Some(SystemId(133))] {
            let mut session = at_gate(&catalog);
            assert_eq!(session.nav_target(), Some(StellarId(300)));
            assert_eq!(
                session.enter_hypergate(to, &catalog, &mut NeverFires),
                Err(GateRefusal::Cancelled),
                "{to:?}"
            );
            assert_eq!(session.nav_target(), None);
            assert_eq!(session.system(), SystemId(130));
            assert_eq!(session.player().position, Vec2::ZERO);
            assert_eq!(session.take_messages(), []);
            assert_eq!(
                session.enter_hypergate(Some(SystemId(131)), &catalog, &mut NeverFires),
                Err(GateRefusal::NotAtGate),
                "the entry is over"
            );
        }
    }

    #[test]
    fn a_hypergate_without_links_leads_nowhere() {
        let catalog = FakePilotCatalog {
            gates: vec![gate_site(300, 130, (0.0, 0.0), HYPERGATE, &[0, -2], 0)],
            ..gated()
        };
        let mut session = at_gate(&catalog);
        assert_eq!(
            session.enter_hypergate(Some(SystemId(131)), &catalog, &mut NeverFires),
            Err(GateRefusal::NoLinks)
        );
        assert_eq!(session.nav_target(), None);
        assert_eq!(session.system(), SystemId(130));
    }

    #[test]
    fn a_hypergate_whose_links_lead_nowhere_cancels() {
        let catalog = FakePilotCatalog {
            gates: vec![gate_site(300, 130, (0.0, 0.0), HYPERGATE, &[999], 0)],
            ..gated()
        };
        let mut session = at_gate(&catalog);
        assert_eq!(
            session.enter_hypergate(None, &catalog, &mut NeverFires),
            Err(GateRefusal::Cancelled)
        );
    }

    #[test]
    fn a_hypergate_with_links_offers_its_map_even_when_they_lead_nowhere() {
        for (links, offered) in [(&[310, 320][..], ids(&[131, 132])), (&[999][..], vec![])] {
            let mut catalog = gated();
            catalog.gates[0] = gate_site(300, 130, (0.0, 0.0), HYPERGATE, links, 0);
            let mut session = at_gate(&catalog);
            assert_eq!(
                session.open_hypergate(&catalog),
                Ok(offered),
                "choose among them: {links:?}"
            );
            assert_eq!(session.nav_target(), Some(StellarId(300)), "{links:?}");
            assert_eq!(
                session.enter_hypergate(None, &catalog, &mut NeverFires),
                Err(GateRefusal::Cancelled),
                "the entry still awaits the pick: {links:?}"
            );
        }
    }

    #[test]
    fn a_hypergate_without_links_refuses_to_offer_its_map_and_ends_the_entry() {
        let catalog = FakePilotCatalog {
            gates: vec![gate_site(300, 130, (0.0, 0.0), HYPERGATE, &[0, -2], 0)],
            ..gated()
        };
        let mut session = at_gate(&catalog);
        assert_eq!(session.open_hypergate(&catalog), Err(GateRefusal::NoLinks));
        assert_eq!(session.nav_target(), None, "the target cleared");
        assert_eq!(session.system(), SystemId(130));
        assert_eq!(
            session.enter_hypergate(None, &catalog, &mut NeverFires),
            Err(GateRefusal::NotAtGate),
            "the entry is over"
        );
    }

    /// The ship starts over unlinked wormhole 400, at system 130's centre;
    /// unlinked wormhole 410 is in 131 at (-300, 40), and wormhole 420,
    /// linked to 410, in 132.
    fn wormholes() -> FakePilotCatalog {
        FakePilotCatalog {
            sites: vec![
                (SystemId(130), vec![gate_landing(400, 0x2200)]),
                (SystemId(131), vec![planet(140, 0.0, 0.0)]),
            ],
            gates: vec![
                gate_site(400, 130, (0.0, 0.0), 0x2200, &[], 120),
                gate_site(410, 131, (-300.0, 40.0), WORMHOLE, &[], 0),
                gate_site(420, 132, (5.0, 5.0), WORMHOLE, &[410], 0),
            ],
            ..catalog()
        }
    }

    /// `catalog`'s session with L pressed twice over the wormhole.
    fn at_wormhole(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.land().expect("clearance");
        assert_eq!(
            session.land(),
            Ok(LandPress::AtGate {
                stellar: StellarId(400),
                kind: GateKind::Wormhole,
            })
        );
        session
    }

    #[test]
    fn entering_a_wormhole_comes_out_of_the_one_rolled() {
        let catalog = wormholes();
        let mut session = at_wormhole(&catalog);
        assert_eq!(
            session.open_hypergate(&catalog),
            Err(GateRefusal::NotAtGate),
            "not a hypergate"
        );
        session.take_sounds();
        let mut chance = Scripted::rolling(&[0]);
        assert_eq!(
            session.enter_wormhole(&catalog, &mut chance),
            Ok(SystemId(131))
        );
        assert_eq!(chance.sides_asked, [2]);
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(session.player().position, Vec2::new(-300.0, 40.0));
        assert_eq!(session.player().heading, 0.0);
        assert_eq!(session.reserves().fuel, Gauge::full(300.0));
        assert_eq!(dmy(&session), (23, 6, 1177));
        assert_eq!(session.take_sounds(), [SimSound::Arrived]);
        assert_eq!(
            session.take_messages(),
            [SimMessage::PassedWormhole(SystemId(131))]
        );
        assert_eq!(
            session.enter_wormhole(&catalog, &mut chance),
            Err(GateRefusal::NotAtGate)
        );
    }

    #[test]
    fn a_wormhole_rolled_onto_a_linked_one_comes_out_where_it_went_in() {
        let catalog = wormholes();
        let mut session = at_wormhole(&catalog);
        let mut chance = Scripted::rolling(&[1]);
        assert_eq!(
            session.enter_wormhole(&catalog, &mut chance),
            Ok(SystemId(130))
        );
        assert_eq!(session.player().position, Vec2::ZERO);
        assert_eq!(session.player().heading, 120.0);
        assert_eq!(
            session.take_messages(),
            [SimMessage::PassedWormhole(SystemId(130))]
        );
    }

    #[test]
    fn by_the_bible_a_wormhole_rolls_over_the_unlinked_ones_only() {
        let catalog = wormholes();
        let mut session = at_wormhole(&catalog).with_wormholes(WormholeRule::UnlinkedOnly);
        let mut chance = Scripted::default();
        assert_eq!(
            session.enter_wormhole(&catalog, &mut chance),
            Ok(SystemId(131))
        );
        assert_eq!(chance.sides_asked, [1]);
    }

    #[test]
    fn a_wormhole_with_nowhere_to_lead_refuses() {
        let catalog = FakePilotCatalog {
            gates: vec![gate_site(400, 130, (0.0, 0.0), WORMHOLE, &[], 0)],
            ..wormholes()
        };
        let mut session = at_wormhole(&catalog);
        assert_eq!(
            session.enter_wormhole(&catalog, &mut NeverFires),
            Err(GateRefusal::NoExit)
        );
        assert_eq!(session.system(), SystemId(130));
        assert_eq!(session.take_messages(), []);
        assert_eq!(
            session.enter_wormhole(&catalog, &mut NeverFires),
            Err(GateRefusal::NotAtGate),
            "the entry is over"
        );
    }

    #[test]
    fn each_gate_is_entered_only_its_own_way() {
        let hypergates = gated();
        let mut session = at_gate(&hypergates);
        assert_eq!(
            session.enter_wormhole(&hypergates, &mut NeverFires),
            Err(GateRefusal::NotAtGate)
        );
        assert_eq!(session.system(), SystemId(130));
        let holes = wormholes();
        let mut session = at_wormhole(&holes);
        assert_eq!(
            session.enter_hypergate(Some(SystemId(131)), &holes, &mut NeverFires),
            Err(GateRefusal::NotAtGate)
        );
        assert_eq!(session.system(), SystemId(130));
    }

    #[test]
    fn like_a_jump_a_gate_places_the_ship_as_a_jump_and_lets_its_days_pass() {
        let catalog = FakePilotCatalog {
            disasters: surplus().disasters,
            ..gated()
        };
        let mut session = at_gate(&catalog).with_gate_arrival(GateArrivalRule::LikeJump);
        let mut chance = Scripted::answering(&[true]);
        assert_eq!(
            session.enter_hypergate(Some(SystemId(131)), &catalog, &mut chance),
            Ok(SystemId(131))
        );
        assert_eq!(
            *session.player(),
            crate::hyperspace::arrival(Vec2::ZERO, Vec2::new(600.0, 0.0), MIN_JUMP_DISTANCE)
        );
        assert_eq!(dmy(&session), (24, 6, 1177), "a jump's day");
        assert_eq!(chance.asked, [35], "the day's events rolled");
        assert_eq!(session.pilot().events().count(), 1);
        assert_eq!(session.reserves().fuel, Gauge::full(300.0), "still no fuel");
    }

    #[test]
    fn by_the_engine_a_gate_lets_no_day_pass_even_with_events_to_roll() {
        let catalog = FakePilotCatalog {
            disasters: surplus().disasters,
            ..gated()
        };
        let mut session = at_gate(&catalog);
        let mut chance = Scripted::answering(&[true]);
        session
            .enter_hypergate(Some(SystemId(131)), &catalog, &mut chance)
            .expect("enters");
        assert!(chance.asked.is_empty());
        assert_eq!(session.pilot().events().count(), 0);
    }

    #[test]
    fn a_gate_entered_mid_thrust_stops_the_thrust() {
        let catalog = gated();
        let mut session = at_gate(&catalog);
        session.thrusting = true;
        session.engine_glow = 10;
        session.take_sounds();
        session
            .enter_hypergate(Some(SystemId(131)), &catalog, &mut NeverFires)
            .expect("enters");
        assert_eq!(
            session.take_sounds(),
            [SimSound::ThrustStopped, SimSound::Arrived]
        );
        assert_eq!(session.engine_glow(), 0);
        assert!(!session.thrusting());
    }
}
