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
//! [`Chance`], seeing the player's ship, the governments read when the
//! session starts, the system's government and the player's legal record
//! with it, and answering the fight's strikes since the last traffic
//! tick; it stands still while the player is landed or jumping, as the
//! player's own ship does.
//!
//! Ships fight ([`combat`](crate::combat)): the player holds a fire
//! command ([`Session::hold_trigger`]) and aims at its target, and each
//! NPC holds the fire command and aims at the target its [`Behaviour`]
//! last decided, and [`Session::tick_combat`] advances the fight a tick
//! among the player and the NPCs, with each ship's weapons, read when the
//! session starts, by the fight's [`Rules`]: disabling ships as a
//! [`DisableRule`](crate::combat::hull::DisableRule) says, and every
//! ship's point defence engaging the missiles fired at it, with no target
//! needed, as a [`PointDefenceRule`](crate::combat::defence::PointDefenceRule)
//! says. The player's weapons are its ship's stock weapons
//! and its weapon outfits, firing the rounds of its ammunition outfits and
//! its fuel, so a fight changes the pilot only in its shield, armour and
//! fuel and the ammunition it owns. The fight stands still while the
//! player is landed or jumping, and its shots and beams are gone once the
//! player lands or arrives elsewhere. An NPC destroyed is taken out of the
//! system. The player's ship, once it is not intact, ignores the controls
//! and drifts, cannot land or jump, and once destroyed stays where it is.
//!
//! The player's crimes change its legal records as the fight's
//! [`LegalCode`] says: disabling an NPC, and breaking one up (which is
//! disabling it too when it was intact), each against the NPC's
//! government. A mere hit is no crime, as in the original.
//!
//! The player targets an NPC ([`Session::select_target`]), the nearest,
//! the nearest threat or the next in turn as the
//! [`targeting`](crate::targeting) rules say, and
//! fires its primary weapons and the secondary selected
//! ([`Session::hold_fire`], [`Session::select_secondary`]): the first the
//! ship carries to start with, kept through a refit while the ship still
//! carries it. The target is let go once it starts breaking up, is
//! destroyed, lands or jumps out, when the system is populated afresh,
//! and when the player lands; it is kept through a jump, and gone on
//! arrival with the last system's traffic. Neither the target nor the
//! secondary is saved.
//!
//! As it goes the session emits [`SimSound`] events (thrust starting and
//! stopping, landing, taking off, a jump beginning and ending), which the
//! audio side drains with [`Session::take_sounds`]. A refused landing or
//! jump emits nothing. The fight's [`CombatEvent`]s are drained with
//! [`Session::take_combat_events`], and the [`SimDiagnostic`]s about game
//! data the simulation does not handle yet, each once a session, with
//! [`Session::take_diagnostics`].

use std::collections::BTreeMap;

use crate::ai::{Behaviour, PlayerSide};
use crate::catalog::{
    CombatCatalog, GovtId, LandingSite, OutfitId, OutfitRecord, PilotCatalog, ShipId, ShipRecord,
    StartError, StellarId, SystemId, TrafficCatalog, WeaponId,
};
use crate::chance::Chance;
use crate::combat::armament::{
    Armament, Arsenal, OutfitRounds, Trigger, next_secondary, outfit_rounds,
};
use crate::combat::beam::Beam;
use crate::combat::hull::{Condition, HullSpec};
use crate::combat::projectile::Shot;
use crate::combat::report::SimDiagnostic;
use crate::combat::weapon::Ammo;
use crate::combat::{Combat, CombatEvent, Downed, Fighter, Rules, ShipRef, Strike};
use crate::date::GameDate;
use crate::flight::{Controls, ShipState, step};
use crate::fuel::regenerate;
use crate::geometry::Vec2;
use crate::govt::Governments;
use crate::handling::{Handling, ShipFields};
use crate::hyperspace::{JUMP_FUEL, JumpRefusal, RouteError, StarMap, arrival, check_jump};
use crate::landing::{LandingRefusal, check_landing};
use crate::legal::{self, Crime, LegalCode};
use crate::market::{self, Goods, Market, Order, TradeRefusal};
use crate::outfitter::{self, OutfitOrder, OutfitRefusal, Outfitter, Shop, outfit_mods};
use crate::pilot::{self, Pilot};
use crate::recharge::{self, RechargeRefusal};
use crate::reserves::{Gauge, Reserves};
use crate::shipyard::{self, Quote, ShipPurchase, ShipRefusal, Shipyard, Yard};
use crate::sound::SimSound;
use crate::stats::ShipStats;
use crate::targeting::{self, TargetPick};
use crate::traffic::autopilot::Outcome;
use crate::traffic::npc::{Npc, NpcId};
use crate::traffic::table::SpawnTable;
use crate::traffic::{Traffic, World};

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
    /// Every weapon and ship type's combat fields, read when the session
    /// starts.
    arsenal: Arsenal,
    /// Every government and their relations, read when the session
    /// starts.
    govts: Governments,
    /// Each ammunition outfit, with the weapon it is the rounds of.
    ammo_outfits: Vec<(WeaponId, OutfitId)>,
    /// The player's ship type's hull.
    hull: HullSpec,
    /// The player's weapons.
    armament: Armament,
    /// How the player's ship is holding up.
    condition: Condition,
    /// The fire command the player holds.
    trigger: Trigger,
    /// The shots and beams in flight, and what the fight reports.
    combat: Combat,
    /// The NPC the player targets, if any.
    target: Option<NpcId>,
    /// The secondary weapon the player has selected, if any.
    secondary: Option<WeaponId>,
    /// The fight's strikes since the last traffic tick, for the NPCs to
    /// answer.
    strikes: Vec<Strike>,
}

impl Session {
    /// A new pilot's session, read from `catalog`: an unnamed
    /// [`Pilot::new`], flown.
    pub fn start(catalog: &(impl PilotCatalog + CombatCatalog)) -> Result<Self, StartError> {
        Self::fly(catalog, Pilot::new(catalog, "")?)
    }

    /// `pilot`'s session, its ship's fields and default items, the
    /// outfits, its system's stellars, the star map, the goods, and the
    /// weapons and ship types' combat fields read from `catalog`, with the
    /// system marked explored. A pilot whose ship
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
    pub fn fly(
        catalog: &(impl PilotCatalog + CombatCatalog),
        mut pilot: Pilot,
    ) -> Result<Self, StartError> {
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
        let outfits = catalog.outfits();
        let mut session = Self {
            fields,
            defaults,
            ammo_outfits: Arsenal::ammo_outfits(&outfits),
            outfits,
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
            arsenal: Arsenal::read(catalog),
            govts: Governments::read(catalog),
            // Refitted below, from the ship and the outfits the pilot owns.
            hull: HullSpec::default(),
            armament: Armament::default(),
            condition: Condition::Intact,
            trigger: Trigger::default(),
            combat: Combat::default(),
            target: None,
            secondary: None,
            strikes: Vec::new(),
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
    /// `gain`, one whose most rose gains as much. The hull and the weapons
    /// follow the ship and the outfits, ready to fire.
    fn refit(&mut self, gain: bool) {
        self.hull = self.arsenal.hull(self.pilot.ship);
        self.armament = self
            .arsenal
            .player(self.pilot.ship, &self.pilot.outfits, &self.outfits);
        let carried = self.secondary.filter(|&id| {
            self.armament
                .secondaries()
                .any(|secondary| secondary.id == id)
        });
        self.secondary = carried.or_else(|| self.next_secondary(None, false));
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
    /// gains no fuel. A ship that is not intact ignores the controls and
    /// drifts, and a destroyed one stays where it is.
    pub fn tick(&mut self, controls: Controls) {
        if self.landed.is_none() && self.jumping.is_none() {
            let controls = match self.condition {
                Condition::Intact => controls,
                Condition::Disabled | Condition::Dying { .. } => Controls::default(),
                Condition::Destroyed => {
                    self.stop_thrust();
                    self.player.velocity = Vec2::ZERO;
                    return;
                }
            };
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
    /// NPCs there (the target with them), its traffic read from `catalog`
    /// and rolled on `chance`.
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
            &self.govts,
            &self.ships,
            &self.outfits,
            &self.arsenal,
        );
        self.traffic.enter(table, chance);
        self.traffic_due = false;
        self.strikes.clear();
        self.clear_lost_target();
    }

    /// Advances the NPC traffic one tick, NPCs deciding as `behaviour`
    /// says, rolling on `chance`. While the ship is landed or jumping, it
    /// stands still. The first tick in flight after the session starts,
    /// and after each take-off, instead populates the system
    /// ([`Session::populate`]) from `catalog`, as the original sets a
    /// system up on arrival and on take-off; its NPCs move from the next.
    /// A target that landed or jumped out is let go.
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
                let system_govt = self.star_map.govt(self.pilot.system);
                let world = World {
                    sites: &self.sites,
                    player: Some(PlayerSide {
                        state: self.player,
                        condition: self.condition,
                        reserves: self.pilot.reserves,
                        hull: self.hull,
                        handling: self.stats.handling,
                    }),
                    govts: &self.govts,
                    system_govt,
                    record: system_govt.map_or(0, |govt| self.pilot.legal_record(govt)),
                };
                let strikes = std::mem::take(&mut self.strikes);
                self.traffic.tick_in(behaviour, world, &strikes, chance);
            }
        }
        self.clear_lost_target();
    }

    /// Holds `trigger`, the player's fire command, until another is held.
    pub fn hold_trigger(&mut self, trigger: Trigger) {
        self.trigger = trigger;
    }

    /// Holds the player's fire keys: the `primary` trigger, and the
    /// `secondary` one on the secondary weapon selected.
    pub fn hold_fire(&mut self, primary: bool, secondary: bool) {
        self.hold_trigger(Trigger {
            primary,
            secondary: self.secondary.filter(|_| secondary),
            only: None,
        });
    }

    /// The secondary after `current`, `backwards` or not (see
    /// [`next_secondary`]), skipping one hidden while the pilot owns none
    /// of its rounds.
    fn next_secondary(&self, current: Option<WeaponId>, backwards: bool) -> Option<WeaponId> {
        next_secondary(self.armament.mounts(), current, backwards, |ammo| {
            outfit_rounds(&self.pilot.outfits, &self.ammo_outfits, ammo)
        })
    }

    /// Selects the next secondary weapon, or the one before it when
    /// `backwards`, in the order they were mounted, wrapping (see
    /// [`next_secondary`]).
    pub fn select_secondary(&mut self, backwards: bool) {
        self.secondary = self.next_secondary(self.secondary, backwards);
    }

    /// The secondary weapon selected, if any: the first the ship carries
    /// when the session starts, and kept through a refit while the ship
    /// still carries it.
    #[must_use]
    pub fn secondary(&self) -> Option<WeaponId> {
        self.secondary
    }

    /// The rounds the selected secondary has left, for one that fires
    /// rounds of ammunition; none for one that fires without, or burns
    /// fuel.
    #[must_use]
    pub fn secondary_rounds(&self) -> Option<u32> {
        let spec = self.arsenal.weapon(self.secondary?)?;
        match spec.ammo {
            Ammo::Rounds(ammo) => {
                Some(outfit_rounds(&self.pilot.outfits, &self.ammo_outfits, ammo))
            }
            _ => None,
        }
    }

    /// Advances the fight a tick among the player and the NPCs, each aimed
    /// at its target, by `rules`, drawing each shot's inaccuracy and
    /// spread on `chance` (see [`Combat::tick`]); each NPC destroyed is taken out of
    /// the system, and the target is let go once it is breaking up or
    /// gone. While the ship is landed or jumping, it stands still.
    pub fn tick_combat(&mut self, rules: Rules, chance: &mut (impl Chance + ?Sized)) {
        if self.landed.is_some() || self.jumping.is_some() {
            return;
        }
        let was: Vec<(NpcId, Condition, Option<GovtId>)> = self
            .traffic
            .npcs()
            .iter()
            .map(|npc| (npc.id, npc.condition, npc.govt))
            .collect();
        let pilot = &mut self.pilot;
        let mut rounds = OutfitRounds {
            owned: &mut pilot.outfits,
            sources: &self.ammo_outfits,
        };
        let mut fighters = vec![Fighter {
            ship: ShipRef::Player,
            ship_type: pilot.ship,
            fleet: ShipRef::Player,
            govt: None,
            state: self.player,
            hull: self.hull,
            shield_regen: self.stats.shield_regen,
            armor_regen: self.stats.armor_regen,
            trigger: self.trigger,
            target: self.target.map(ShipRef::Npc),
            reserves: &mut pilot.reserves,
            condition: &mut self.condition,
            armament: &mut self.armament,
            rounds: &mut rounds,
        }];
        for npc in self.traffic.npcs_mut() {
            let fleet = ShipRef::Npc(npc.fleet());
            let Npc {
                id,
                ship,
                govt,
                stats,
                state,
                hull,
                trigger,
                target,
                reserves,
                condition,
                armament,
                rounds,
                ..
            } = npc;
            fighters.push(Fighter {
                ship: ShipRef::Npc(*id),
                ship_type: *ship,
                fleet,
                govt: *govt,
                state: *state,
                hull: *hull,
                shield_regen: stats.shield_regen,
                armor_regen: stats.armor_regen,
                trigger: *trigger,
                target: *target,
                reserves,
                condition,
                armament,
                rounds,
            });
        }
        self.combat
            .tick(&mut fighters, &self.arsenal, &self.govts, rules, chance);
        let strikes = self.combat.take_strikes();
        self.punish(&strikes, &was, rules.law);
        self.strikes.extend(strikes);
        let destroyed: Vec<NpcId> = self
            .traffic
            .npcs()
            .iter()
            .filter(|npc| npc.condition == Condition::Destroyed)
            .map(|npc| npc.id)
            .collect();
        for id in destroyed {
            self.traffic.remove(id);
        }
        self.clear_lost_target();
    }

    /// Convicts the player of its crimes among `strikes`, as `law` says:
    /// disabling an NPC, and breaking one up, which is disabling it too
    /// when it `was` intact before (as `_DamageShip` slaps both). Each
    /// victim's condition and government are as it `was` before the
    /// fight's tick.
    fn punish(
        &mut self,
        strikes: &[Strike],
        was: &[(NpcId, Condition, Option<GovtId>)],
        law: &dyn LegalCode,
    ) {
        for strike in strikes.iter().filter(|strike| strike.by == ShipRef::Player) {
            let ShipRef::Npc(id) = strike.ship else {
                continue;
            };
            let Some(&(_, condition, govt)) = was.iter().find(|(npc, ..)| *npc == id) else {
                continue;
            };
            let crimes: &[Crime] = match (strike.downed, condition) {
                (Some(Downed::Disabled), _) => &[Crime::Disable],
                (Some(Downed::BreakingUp), Condition::Intact) => &[Crime::Disable, Crime::Kill],
                (Some(Downed::BreakingUp), _) => &[Crime::Kill],
                (None, _) => &[],
            };
            for &crime in crimes {
                let changes = law.penalties(crime, govt, &self.govts);
                legal::convict(&mut self.pilot, &changes);
            }
        }
    }

    /// Lets go of the target once it is no longer in the system (it was
    /// destroyed, landed or jumped out, or the system was populated
    /// afresh) or no longer targetable (it is breaking up), as the
    /// original does every frame.
    fn clear_lost_target(&mut self) {
        if self.target().is_none_or(|npc| !targeting::targetable(npc)) {
            self.target = None;
        }
    }

    /// Picks the player's target as `pick` says (see
    /// [`targeting`](crate::targeting)), and gives it: the nearest NPC, or
    /// the nearest threat, the target unchanged when there is none; or the
    /// next. While the ship is landed or jumping, nothing changes.
    pub fn select_target(&mut self, pick: TargetPick) -> Option<NpcId> {
        if self.landed.is_none() && self.jumping.is_none() {
            let npcs = self.traffic.npcs();
            let from = self.player.position;
            match pick {
                TargetPick::Nearest => {
                    let nearest = targeting::nearest(npcs, from, |_| true);
                    self.target = nearest.or(self.target);
                }
                TargetPick::NearestThreat => {
                    let nearest = targeting::nearest(npcs, from, Npc::threatens_player);
                    self.target = nearest.or(self.target);
                }
                TargetPick::Next => self.target = targeting::next(npcs, self.target),
            }
        }
        self.target
    }

    /// The NPC the player targets, if any.
    #[must_use]
    pub fn target(&self) -> Option<&Npc> {
        let id = self.target?;
        self.npcs().iter().find(|npc| npc.id == id)
    }

    /// The player's ship type's hull: how it breaks up and dies.
    #[must_use]
    pub fn hull(&self) -> HullSpec {
        self.hull
    }

    /// How the player's ship is holding up.
    #[must_use]
    pub fn player_condition(&self) -> Condition {
        self.condition
    }

    /// The shots in flight.
    #[must_use]
    pub fn shots(&self) -> &[Shot] {
        self.combat.shots()
    }

    /// The beams being fired.
    #[must_use]
    pub fn beams(&self) -> &[Beam] {
        self.combat.beams()
    }

    /// What has happened in the fight since this was last taken, in order;
    /// taking it empties the list.
    pub fn take_combat_events(&mut self) -> Vec<CombatEvent> {
        self.combat.take_events()
    }

    /// The diagnostics about game data the simulation does not handle yet,
    /// each made once a session, since they were last taken; taking them
    /// empties the list.
    pub fn take_diagnostics(&mut self) -> Vec<SimDiagnostic> {
        self.combat.take_diagnostics()
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

    /// Ship type `ship`'s name, as its [`ShipRecord`] gives it, if the
    /// session has its record.
    #[must_use]
    pub fn ship_name(&self, ship: ShipId) -> Option<&str> {
        let record = self.ships.iter().find(|record| record.id == ship)?;
        Some(&record.name)
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
    /// landed, is intact and the [`hyperspace`](crate::hyperspace) rules
    /// allow it, and gives that system; otherwise the refusal says why.
    /// Until it arrives, ticks move nothing.
    pub fn begin_jump(&mut self) -> Result<SystemId, JumpRefusal> {
        if self.landed.is_some() {
            return Err(JumpRefusal::Landed);
        }
        if self.condition != Condition::Intact {
            return Err(JumpRefusal::Disabled);
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
    /// traffic ([`Session::populate`]), the last system's gone, and so are
    /// the shots and beams in flight. `None`, and nothing changes, when no
    /// jump is under way.
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
        self.combat.clear();
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

    /// Lands the ship on the stellar it is over, if it is not jumping, is
    /// intact and the [`landing`](crate::landing) rules allow it, with the
    /// pilot's legal record with the stellar's government, or the system's
    /// on the star map when the stellar has none: it docks at the
    /// stellar's centre, at rest, its heading and reserves unchanged, and
    /// the shots and beams in flight and the target are gone. Otherwise it
    /// flies on, and the refusal says why.
    pub fn land(&mut self) -> Result<StellarId, LandingRefusal> {
        if self.jumping.is_some() {
            return Err(LandingRefusal::Jumping);
        }
        if self.condition != Condition::Intact {
            return Err(LandingRefusal::Disabled);
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
        self.combat.clear();
        self.target = None;
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
    fn a_ship_type_is_named_by_its_ship_record() {
        let session = Session::start(&shipbuying()).expect("starts");
        assert_eq!(session.ship_name(ShipId(129)), Some("Ship 129"));
        assert_eq!(session.ship_name(ShipId(128)), Some("Ship 128"));
        assert_eq!(session.ship_name(ShipId(130)), None, "no such record");
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
    fn every_ai_type_lands_peacefully() {
        for ai_type in 1..=4 {
            let how = Outcome::Landed(StellarId(129));
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

    // Fleets.

    use crate::catalog::{EscortRecord, FleetId, FleetRecord};

    /// `flët` `id`, linked by `link_syst`: a ship 129 lead and 1-3 ship
    /// 130 escorts, for govt 150.
    fn fleet(id: i16, link_syst: i16) -> FleetRecord {
        FleetRecord {
            id: FleetId(id),
            lead: Some(ShipId(129)),
            escorts: vec![EscortRecord {
                ship: ShipId(130),
                min: 1,
                max: 3,
            }],
            govt: Some(GovtId(150)),
            link_syst,
            appear_on: String::new(),
        }
    }

    /// The `LinkSyst` of the systems government `govt` governs.
    fn govt_link(govt: i16) -> i16 {
        10_000 + (govt - 128)
    }

    /// [`trafficked`] with one ship on average in 130 and 131, governed
    /// on the star map by 140 and 141, and `flët`s 130 (slot 2), linked to
    /// govt 140, and 131 (slot 3), linked to govt 141.
    fn fleeted() -> FakePilotCatalog {
        let mut catalog = trafficked(130, 1, 3);
        let elsewhere = (SystemId(131), catalog.traffic[0].1);
        catalog.traffic.push(elsewhere);
        catalog.star_map[0].govt = Some(GovtId(140));
        catalog.star_map[1].govt = Some(GovtId(141));
        catalog.fleets = vec![fleet(130, govt_link(140)), fleet(131, govt_link(141))];
        catalog.ship_records.push(ship(130, FAST));
        catalog
    }

    /// A setup pass that draws the `LinkSyst` fleet in `slot`, with three
    /// escorts.
    fn linked_fleet_draws(slot: u32) -> Draws {
        Draws::of(&[1, 0, slot, 0, 2])
    }

    /// Each NPC's ship, government and leader.
    fn fleet_of(session: &Session) -> Vec<(ShipId, Option<GovtId>, Option<NpcId>)> {
        session
            .npcs()
            .iter()
            .map(|npc| (npc.ship, npc.govt, npc.leader))
            .collect()
    }

    /// A [`fleet`]'s lead, as NPC `lead`, and its three escorts.
    fn a_fleet_led_by(lead: u32) -> Vec<(ShipId, Option<GovtId>, Option<NpcId>)> {
        let escort = (ShipId(130), Some(GovtId(150)), Some(NpcId(lead)));
        vec![
            (ShipId(129), Some(GovtId(150)), None),
            escort,
            escort,
            escort,
        ]
    }

    #[test]
    fn populating_brings_in_the_fleet_linked_to_the_systems_government() {
        let catalog = fleeted();
        let mut session = Session::start(&catalog).expect("starts");
        session.populate(&catalog, &mut linked_fleet_draws(2));
        assert_eq!(fleet_of(&session), a_fleet_led_by(0));
        session.populate(&catalog, &mut linked_fleet_draws(3));
        assert_eq!(fleet_of(&session), [], "linked to another government");
    }

    #[test]
    fn arriving_brings_in_the_fleet_linked_to_the_new_systems_government() {
        let catalog = fleeted();
        let mut session = Session::start(&catalog).expect("starts");
        jump_with(&mut session, &catalog, 131, &mut linked_fleet_draws(3));
        assert_eq!(fleet_of(&session), a_fleet_led_by(0));
        session.populate(&catalog, &mut linked_fleet_draws(2));
        assert_eq!(fleet_of(&session), [], "linked to another government");
    }

    #[test]
    fn a_fleet_the_systems_dude_types_name_arrives_when_its_roll_fires() {
        let mut catalog = trafficked(130, 1, 3);
        catalog.traffic[0].1.dude_types[1] = (-132, 30);
        catalog.fleets = vec![fleet(132, 0)];
        catalog.ship_records.push(ship(130, FAST));
        let mut session = Session::start(&catalog).expect("starts");
        session.populate(&catalog, &mut Draws::of(&[0]));
        assert_eq!(fleet_of(&session), [], "a person");
        session.tick_traffic(&catalog, &Peaceful, &mut Draws::of(&[1, 30, 0]));
        assert_eq!(fleet_of(&session), [], "31 is above its 30%");
        session.tick_traffic(&catalog, &Peaceful, &mut Draws::of(&[1, 29, 0, 0, 2]));
        assert_eq!(fleet_of(&session), a_fleet_led_by(0));
    }

    // Combat.

    use crate::ai::{Behaviour, Goal, Surroundings};
    use crate::catalog::{GovtRecord, HullRecord, StockWeapon, WeaponRecord};
    use crate::combat::armament::MOD_AMMO;
    use crate::combat::defence::Allegiance;
    use crate::combat::hull::DisableRule;
    use crate::combat::weapon::Explosion;
    use crate::legal::{Crime, LegalCode, NovaLaw};
    use crate::rulebook::RuleSource;
    use crate::testkit::{hull, weapon};
    use crate::traffic::npc::Npc;

    /// A blaster firing every other tick, 20 pixels a tick for 30 ticks,
    /// doing 5 mass and 10 energy damage.
    fn blaster() -> WeaponRecord {
        WeaponRecord {
            reload: 2,
            count: 30,
            speed: 2000,
            mass_dmg: 5,
            energy_dmg: 10,
            ..weapon(128)
        }
    }

    /// Ship `id` armed with one of `weapon`, 30 pixels across, breaking
    /// up for 2 ticks before `bööm` 133 destroys it.
    fn armed_hull(id: i16, weapon: i16) -> HullRecord {
        HullRecord {
            weapons: vec![StockWeapon {
                weapon: WeaponId(weapon),
                count: 1,
                ammo: 0,
            }],
            death_delay: 2,
            explode2: 5,
            size: Some(30),
            ..hull(id)
        }
    }

    /// [`trafficked`] with the player (ship 128) and its traffic (ship
    /// 129, a trader: 30 shield, 45 armour) each carrying a blaster.
    fn armed() -> FakePilotCatalog {
        FakePilotCatalog {
            weapons: vec![blaster()],
            hulls: vec![armed_hull(128, 128), armed_hull(129, 128)],
            ..trafficked(130, 1, 1)
        }
    }

    /// `catalog`'s session with its one NPC placed 100 pixels above the
    /// player (at the centre, facing up), facing `heading`.
    fn facing_an_npc(catalog: &FakePilotCatalog, heading: u32) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.populate(catalog, &mut Draws::of(&[6, 6, 0, 0, 750, 650, heading]));
        assert_eq!(session.npcs()[0].state.position, Vec2::new(0.0, -100.0));
        session
    }

    const FIRE: Trigger = Trigger {
        primary: true,
        secondary: None,
        only: None,
    };

    /// The fight's events about NPC 0, other than its firing.
    fn about_the_npc(events: &[CombatEvent]) -> Vec<CombatEvent> {
        events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    CombatEvent::Disabled { .. }
                        | CombatEvent::BreakingUp { .. }
                        | CombatEvent::Destroyed { .. }
                )
            })
            .copied()
            .collect()
    }

    #[test]
    fn the_players_trigger_fires_its_stock_weapon_and_disables_then_destroys_an_npc() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        session.hold_trigger(FIRE);
        let mut gauges = Vec::new();
        let mut events = Vec::new();
        for _ in 0..60 {
            session.tick_combat(Rules::default(), &mut NeverFires);
            events.extend(session.take_combat_events());
            let Some(npc) = session.npcs().first() else {
                break;
            };
            gauges.push((
                npc.reserves.shield.now,
                npc.reserves.armor.now,
                npc.condition,
            ));
            if npc.condition == Condition::Disabled {
                break;
            }
        }
        let last = *gauges.last().expect("ticks");
        assert_eq!(last, (-3.0, 10.0, Condition::Disabled), "{gauges:?}");
        let first_armour = gauges.iter().position(|g| g.1 < 45.0).expect("hit");
        assert!(gauges[first_armour].0 <= 0.0, "shields first: {gauges:?}");
        assert!(
            gauges[..first_armour]
                .iter()
                .any(|g| g.0 > 0.0 && g.0 < 30.0)
        );
        assert_eq!(
            about_the_npc(&events),
            [CombatEvent::Disabled {
                ship: ShipRef::Npc(NpcId(0))
            }]
        );
        assert!(events.contains(&CombatEvent::Fired {
            ship: ShipRef::Player,
            weapon: WeaponId(128),
            at: Vec2::ZERO
        }));
        assert_eq!(session.npcs().len(), 1, "disabled, and still alive");
        for _ in 0..60 {
            session.tick_combat(Rules::default(), &mut NeverFires);
            events.extend(session.take_combat_events());
        }
        let ending: Vec<_> = about_the_npc(&events);
        assert!(
            matches!(
                ending[..],
                [
                    CombatEvent::Disabled { .. },
                    CombatEvent::BreakingUp { .. },
                    CombatEvent::Destroyed {
                        ship: ShipRef::Npc(NpcId(0)),
                        ship_type: ShipId(129),
                        explosion: Some(Explosion { .. }),
                        size: 0.0,
                        ..
                    }
                ]
            ),
            "{ending:?}"
        );
        assert_eq!(session.npcs(), [], "gone once destroyed");
        assert_eq!(session.player_condition(), Condition::Intact);
    }

    /// Every NPC idles and holds its trigger.
    #[derive(Debug)]
    struct Firing;

    impl Behaviour for Firing {
        fn decide(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            Goal::Idle
        }

        fn trigger(&self, _npc: &Npc, _around: &Surroundings) -> Trigger {
            FIRE
        }
    }

    #[test]
    fn an_npcs_trigger_fires_at_the_player_through_the_same_fight() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        let mut events = Vec::new();
        for _ in 0..12 {
            session.tick_traffic(&catalog, &Firing, &mut NeverFires);
            session.tick_combat(Rules::default(), &mut NeverFires);
            events.extend(session.take_combat_events());
        }
        assert!(events.contains(&CombatEvent::Fired {
            ship: ShipRef::Npc(NpcId(0)),
            weapon: WeaponId(128),
            at: Vec2::new(0.0, -100.0)
        }));
        assert!(
            session.reserves().shield.now < 30.0,
            "{:?}",
            session.reserves()
        );
        assert!(
            session
                .shots()
                .iter()
                .all(|shot| shot.firer == ShipRef::Npc(NpcId(0))),
            "the player held no trigger"
        );
    }

    #[test]
    fn the_players_ammunition_outfits_and_fuel_pay_for_its_shots() {
        let rocket = WeaponRecord {
            ammo_type: 10,
            ..blaster()
        };
        let mut catalog = FakePilotCatalog {
            weapons: vec![WeaponRecord {
                id: WeaponId(138),
                ..rocket
            }],
            hulls: vec![armed_hull(128, 138)],
            outfits: vec![outfit(300, &[(MOD_AMMO, 138)])],
            defaults: vec![(ShipId(128), vec![(OutfitId(300), 2)])],
            ..catalog()
        };
        let mut session = Session::start(&catalog).expect("starts");
        session.hold_trigger(FIRE);
        for _ in 0..8 {
            session.tick_combat(Rules::default(), &mut NeverFires);
        }
        assert_eq!(session.pilot().outfits().count(), 0, "both rockets fired");
        assert_eq!(session.shots().len(), 2);
        let fuelled = WeaponRecord {
            ammo_type: -1100,
            ..blaster()
        };
        catalog.weapons = vec![fuelled];
        catalog.hulls = vec![armed_hull(128, 128)];
        let mut session = Session::start(&catalog).expect("starts");
        session.hold_trigger(FIRE);
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(session.reserves().fuel.now, 290.0, "10 units a shot");
    }

    #[test]
    fn a_beam_weapon_fires_beams() {
        let laser = WeaponRecord {
            guidance: 0,
            count: 5,
            beam_length: 100,
            ..blaster()
        };
        let catalog = FakePilotCatalog {
            weapons: vec![laser],
            ..armed()
        };
        let mut session = Session::start(&catalog).expect("starts");
        session.hold_trigger(FIRE);
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(session.beams().len(), 1);
        assert_eq!(session.beams()[0].firer, ShipRef::Player);
        assert_eq!(session.shots(), []);
    }

    #[test]
    fn the_fight_stands_still_while_landed_or_jumping() {
        let mut catalog = armed();
        catalog.traffic.push((SystemId(131), catalog.traffic[0].1));
        let mut session = Session::start(&catalog).expect("starts");
        session.hold_trigger(FIRE);
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(session.shots().len(), 1);
        session.land().expect("lands on planet 128");
        assert_eq!(session.shots(), [], "gone on landing");
        session.take_combat_events();
        let mut chance = Draws::of(&[]);
        for _ in 0..3 {
            session.tick_combat(Rules::default(), &mut chance);
        }
        assert_eq!(session.take_combat_events(), [], "landed, though reloaded");
        assert_eq!(session.shots(), []);
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        session.hold_trigger(FIRE);
        session.tick_combat(Rules::default(), &mut chance);
        assert_eq!(session.shots().len(), 1);
        session.begin_jump().expect("jumps");
        session.take_combat_events();
        for _ in 0..3 {
            session.tick_combat(Rules::default(), &mut chance);
        }
        assert_eq!(session.take_combat_events(), [], "jumping, though reloaded");
        session.arrive(&catalog, &mut NeverFires).expect("arrives");
        assert_eq!(session.shots(), [], "gone on arriving");
        assert!(chance.asked.is_empty());
    }

    /// Disables every ship.
    #[derive(Debug)]
    struct Disabling;

    impl DisableRule for Disabling {
        fn disabled(&self, _armor: Gauge, _hull: &HullSpec) -> bool {
            true
        }
    }

    const DISABLING: Rules = Rules {
        disable: &Disabling,
        defence: &Allegiance,
        law: &NovaLaw {
            crime_gains: RuleSource::Engine,
        },
    };

    // What NPCs see.

    /// What an NPC sees: the player, the system's government, the record
    /// there, and how many governments there are.
    type Seen = (Option<crate::ai::PlayerSide>, Option<GovtId>, i16, usize);

    /// Records what each NPC sees as it decides, and each strike it
    /// answers; decides nothing new.
    #[derive(Debug, Default)]
    struct Spy {
        seen: std::cell::RefCell<Vec<Seen>>,
        struck: std::cell::RefCell<Vec<crate::combat::Strike>>,
    }

    impl Behaviour for Spy {
        fn decide(&self, npc: &Npc, around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            self.seen.borrow_mut().push((
                around.player,
                around.system_govt,
                around.record,
                around.govts.ids().count(),
            ));
            npc.goal
        }

        fn react(
            &self,
            _npc: &Npc,
            strike: &crate::combat::Strike,
            _around: &Surroundings,
        ) -> crate::ai::Reaction {
            self.struck.borrow_mut().push(*strike);
            crate::ai::Reaction::default()
        }
    }

    #[test]
    fn npcs_see_the_player_the_governments_and_the_record_in_the_system() {
        let mut catalog = armed();
        catalog.star_map[0].govt = Some(GovtId(150));
        catalog.govts = vec![crate::testkit::govt(150), crate::testkit::govt(151)];
        let mut session = facing_an_npc(&catalog, 180);
        session.pilot.set_legal_record(GovtId(150), -20);
        session.pilot.set_legal_record(GovtId(151), 30);
        let spy = Spy::default();
        session.tick_traffic(&catalog, &spy, &mut NeverFires);
        let seen = spy.seen.take();
        assert_eq!(seen.len(), 1);
        let (player, system, record, govts) = seen[0];
        let player = player.expect("the player flies here");
        assert_eq!(player.state, *session.player());
        assert_eq!(player.condition, Condition::Intact);
        assert_eq!(player.reserves, session.reserves());
        assert_eq!(player.hull, session.hull());
        assert_eq!(player.handling, session.handling());
        assert_eq!((system, record, govts), (Some(GovtId(150)), -20, 2));
        catalog.star_map[0].govt = None;
        let mut session = facing_an_npc(&catalog, 180);
        session.pilot.set_legal_record(GovtId(150), -20);
        session.tick_traffic(&catalog, &spy, &mut NeverFires);
        let (_, system, record, _) = spy.seen.take()[0];
        assert_eq!((system, record), (None, 0), "an independent system");
    }

    #[test]
    fn npcs_answer_the_fights_strikes_on_the_next_traffic_tick_once() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        session.hold_trigger(FIRE);
        let spy = Spy::default();
        let mut hits = 0;
        for _ in 0..30 {
            session.tick_combat(Rules::default(), &mut NeverFires);
            session.tick_traffic(&catalog, &spy, &mut NeverFires);
            let struck = spy.struck.take();
            hits += struck.len();
            assert!(struck.len() <= 1, "each strike once: {struck:?}");
            for strike in struck {
                assert_eq!(
                    (strike.ship, strike.by),
                    (ShipRef::Npc(NpcId(0)), ShipRef::Player)
                );
            }
        }
        assert!(hits > 3, "{hits}");
        session.tick_combat(Rules::default(), &mut NeverFires);
        session.land().expect("lands");
        session.take_off();
        session.tick_traffic(&catalog, &spy, &mut NeverFires);
        assert_eq!(spy.struck.take(), [], "gone with the system's traffic");
    }

    // Crimes.

    /// The NPCs' government 140 (class 1: disabling costs 3, killing 7),
    /// its ally 141, its enemy 142 (disabling 8, killing 10) and a
    /// neutral, 143.
    fn lawful_govts() -> Vec<GovtRecord> {
        let penalised = |id, disable, kill| GovtRecord {
            penalties: crate::catalog::Penalties {
                disable,
                kill,
                ..crate::catalog::Penalties::default()
            },
            ..crate::testkit::govt(id)
        };
        vec![
            GovtRecord {
                classes: [1, -1, -1, -1],
                ..penalised(140, 3, 7)
            },
            GovtRecord {
                allies: [1, -1, -1, -1],
                ..penalised(141, 20, 20)
            },
            GovtRecord {
                enemies: [1, -1, -1, -1],
                ..penalised(142, 8, 10)
            },
            penalised(143, 20, 20),
        ]
    }

    /// [`armed`], with [`lawful_govts`].
    fn lawful() -> FakePilotCatalog {
        FakePilotCatalog {
            govts: lawful_govts(),
            ..armed()
        }
    }

    /// The pilot's records with governments 140-143.
    fn records(session: &Session) -> [i16; 4] {
        [140, 141, 142, 143].map(|govt| session.pilot().legal_record(GovtId(govt)))
    }

    /// Records each crime it is asked about, and the victim's
    /// government, and changes nothing.
    #[derive(Debug, Default)]
    struct Witness {
        seen: std::cell::RefCell<Vec<(Crime, Option<GovtId>)>>,
    }

    impl LegalCode for Witness {
        fn penalties(
            &self,
            crime: Crime,
            victim: Option<GovtId>,
            _govts: &Governments,
        ) -> Vec<(GovtId, i32)> {
            self.seen.borrow_mut().push((crime, victim));
            Vec::new()
        }
    }

    #[test]
    fn disabling_an_npc_lowers_the_records_as_the_law_says_once_and_breaking_it_up_again() {
        let catalog = lawful();
        let mut session = facing_an_npc(&catalog, 180);
        session.hold_trigger(FIRE);
        let mut seen = Vec::new();
        for _ in 0..80 {
            session.tick_combat(Rules::default(), &mut NeverFires);
            let condition = session.npcs().first().map(|npc| npc.condition);
            if seen.last().is_none_or(|(last, _)| *last != condition) {
                seen.push((condition, records(&session)));
            }
        }
        assert_eq!(
            seen,
            [
                (Some(Condition::Intact), [0, 0, 0, 0]),
                (Some(Condition::Disabled), [-3, -3, 4, 10]),
                (Some(Condition::Dying { ticks_left: 1 }), [-10, -10, 9, 20]),
                (Some(Condition::Dying { ticks_left: 0 }), [-10, -10, 9, 20]),
                (None, [-10, -10, 9, 20]),
            ],
            "hits that leave it intact change nothing; by the engine's law, the neutral gains"
        );
        let saved = crate::save::decode(&crate::save::encode(session.pilot())).expect("reads");
        assert_eq!(&saved, session.pilot(), "the records saved as they are");
    }

    #[test]
    fn the_law_hears_of_a_disabling_then_a_killing_against_the_victims_government() {
        let catalog = lawful();
        let mut session = facing_an_npc(&catalog, 180);
        session.hold_trigger(FIRE);
        let witness = Witness::default();
        let rules = Rules {
            law: &witness,
            ..Rules::default()
        };
        for _ in 0..80 {
            session.tick_combat(rules, &mut NeverFires);
        }
        assert_eq!(session.npcs(), [], "destroyed");
        assert_eq!(
            witness.seen.take(),
            [
                (Crime::Disable, Some(GovtId(140))),
                (Crime::Kill, Some(GovtId(140)))
            ]
        );
        assert_eq!(records(&session), [0; 4], "as the witness says");
    }

    #[test]
    fn breaking_up_an_intact_ship_at_once_is_disabling_it_and_killing_it() {
        let catalog = FakePilotCatalog {
            weapons: vec![WeaponRecord {
                mass_dmg: 500,
                energy_dmg: 500,
                ..blaster()
            }],
            ..lawful()
        };
        let mut session = facing_an_npc(&catalog, 180);
        session.hold_trigger(FIRE);
        let witness = Witness::default();
        let rules = Rules {
            law: &witness,
            ..Rules::default()
        };
        for _ in 0..20 {
            session.tick_combat(rules, &mut NeverFires);
            session.hold_trigger(Trigger::default());
        }
        assert_eq!(
            witness.seen.take(),
            [
                (Crime::Disable, Some(GovtId(140))),
                (Crime::Kill, Some(GovtId(140)))
            ],
            "as the original's _DamageShip slaps both"
        );
    }

    #[test]
    fn an_npc_downing_another_is_no_crime() {
        let catalog = FakePilotCatalog {
            traffic: lawful()
                .traffic
                .into_iter()
                .map(|(system, traffic)| {
                    (
                        system,
                        SystemTraffic {
                            avg_ships: 2,
                            ..traffic
                        },
                    )
                })
                .collect(),
            ..lawful()
        };
        let mut session = Session::start(&catalog).expect("starts");
        // NPC 0 at (0, -100) and NPC 1 at (100, -100), both facing right.
        session.populate(
            &catalog,
            &mut Draws::of(&[6, 6, 0, 0, 750, 650, 90, 2, 6, 6, 0, 0, 850, 650, 90, 2]),
        );
        let witness = Witness::default();
        let rules = Rules {
            law: &witness,
            ..Rules::default()
        };
        for _ in 0..80 {
            session.tick_traffic(&catalog, &Firing, &mut NeverFires);
            session.tick_combat(rules, &mut NeverFires);
        }
        assert_eq!(session.npcs().len(), 1, "NPC 0 destroyed NPC 1");
        assert_eq!(witness.seen.take(), []);
    }

    #[test]
    fn a_disabled_player_drifts_ignoring_the_controls() {
        let catalog = armed();
        let mut session = Session::start(&catalog).expect("starts");
        session.tick(Controls {
            thrust: true,
            ..Controls::default()
        });
        session.take_sounds();
        let moving = *session.player();
        session.tick_combat(DISABLING, &mut NeverFires);
        assert_eq!(session.player_condition(), Condition::Disabled);
        session.tick(Controls {
            thrust: true,
            turn: Turn::Left,
            reverse: false,
        });
        let drifted = *session.player();
        assert_eq!(drifted.velocity, moving.velocity, "no thrust");
        assert_eq!(drifted.heading, moving.heading, "no turn");
        assert_eq!(drifted.position, moving.position + moving.velocity);
        assert_eq!(session.take_sounds(), [SimSound::ThrustStopped]);
        session.hold_trigger(FIRE);
        session.tick_combat(DISABLING, &mut NeverFires);
        assert_eq!(session.shots(), [], "nor fires");
    }

    #[test]
    fn a_ship_that_is_not_intact_can_neither_land_nor_jump() {
        let catalog = armed();
        let mut session = Session::start(&catalog).expect("starts");
        session.tick_combat(DISABLING, &mut NeverFires);
        assert_eq!(
            session.land(),
            Err(LandingRefusal::Disabled),
            "over planet 128"
        );
        assert_eq!(session.landed(), None);
        assert!(!session.take_save_due(), "nothing to save");
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        session.take_sounds();
        session.tick_combat(DISABLING, &mut NeverFires);
        assert_eq!(session.begin_jump(), Err(JumpRefusal::Disabled));
        assert_eq!(session.jumping(), None);
        assert_eq!(session.take_sounds(), [], "a refusal emits nothing");
    }

    /// An NPC one shot of whose disables, breaks up and destroys the
    /// player at once: no shield, armour gone.
    fn deadly() -> FakePilotCatalog {
        let mut catalog = armed();
        catalog.weapons = vec![WeaponRecord {
            mass_dmg: 100,
            flags: 0x0020,
            ..blaster()
        }];
        catalog
    }

    #[test]
    fn a_destroyed_player_stays_destroyed_and_stops_moving() {
        let catalog = deadly();
        let mut session = facing_an_npc(&catalog, 180);
        let mut events = Vec::new();
        for _ in 0..20 {
            session.tick_traffic(&catalog, &Firing, &mut NeverFires);
            session.tick_combat(Rules::default(), &mut NeverFires);
            events.extend(session.take_combat_events());
        }
        assert_eq!(session.player_condition(), Condition::Destroyed);
        assert!(events.iter().any(|event| matches!(
            event,
            CombatEvent::Destroyed {
                ship: ShipRef::Player,
                ship_type: ShipId(128),
                ..
            }
        )));
        let at = *session.player();
        session.tick(Controls {
            thrust: true,
            ..Controls::default()
        });
        assert_eq!(session.player().position, at.position);
        assert_eq!(session.player().velocity, Vec2::ZERO);
        assert_eq!(session.land(), Err(LandingRefusal::Disabled));
        assert!(!session.take_save_due());
    }

    #[test]
    fn diagnostics_are_drained_once() {
        let mut catalog = armed();
        catalog.weapons[0].flags2 = 0x8000;
        let mut session = Session::start(&catalog).expect("starts");
        session.hold_trigger(FIRE);
        for _ in 0..5 {
            session.tick_combat(Rules::default(), &mut NeverFires);
        }
        assert_eq!(
            session.take_diagnostics(),
            [SimDiagnostic::UnimplementedWeaponFlag {
                weapon: WeaponId(128),
                field: crate::combat::flags::FlagField::Flags2,
                bit: 0x8000
            }]
        );
        assert_eq!(session.take_diagnostics(), []);
    }

    #[test]
    fn shields_regenerate_in_the_fight_at_the_ships_rate() {
        let mut catalog = armed();
        catalog.ships = vec![(
            ShipId(128),
            Ok(ShipFields {
                shield_rech: 1000,
                ..FAST
            }),
        )];
        let mut session = facing_an_npc(&catalog, 180);
        for _ in 0..12 {
            session.tick_traffic(&catalog, &Firing, &mut NeverFires);
            session.tick_combat(Rules::default(), &mut NeverFires);
        }
        let hurt = session.reserves().shield.now;
        assert!(hurt < 30.0);
        session.traffic.remove(NpcId(0));
        session.combat.clear();
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(session.reserves().shield.now, hurt + 1.0);
    }

    // Refitting for the fight.

    use crate::catalog::BoomId;
    use crate::combat::armament::MOD_WEAPON;
    use crate::combat::weapon::PASSES_SHIELDS;

    /// The weapons the player fired in `events`.
    fn fired_by_the_player(events: &[CombatEvent]) -> Vec<WeaponId> {
        events
            .iter()
            .filter_map(|event| match event {
                CombatEvent::Fired {
                    ship: ShipRef::Player,
                    weapon,
                    ..
                } => Some(*weapon),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_weapon_outfit_bought_fires_once_the_ship_takes_off() {
        let mut catalog = FakePilotCatalog {
            weapons: vec![blaster()],
            hulls: vec![hull(128)],
            ..outfitting()
        };
        catalog.outfits.push(outfit(305, &[(MOD_WEAPON, 128)]));
        let mut session = Session::start(&catalog).expect("starts");
        session.hold_trigger(FIRE);
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(session.take_combat_events(), [], "no weapon yet");
        session.land().expect("lands at the outfitter");
        session.outfit(buy(OutfitId(305))).expect("bought");
        session.take_off().expect("took off");
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(
            fired_by_the_player(&session.take_combat_events()),
            [WeaponId(128)]
        );
        assert_eq!(session.shots().len(), 1);
    }

    /// [`shipbuying`] where the player's ship 128 carries a blaster
    /// (weapon 128), 30 pixels across with `bööm` 133 destroying it, and
    /// ship 129 ([`NEW`]) a cannon (weapon 129), 120 pixels across with
    /// `bööm` 135; the system's traffic is a trader, ship 130, whose gun
    /// (weapon 130) passes the shields and destroys at one shot.
    fn armed_shipyard() -> FakePilotCatalog {
        let traffic = trafficked(130, 1, 1);
        let mut catalog = FakePilotCatalog {
            weapons: vec![
                blaster(),
                WeaponRecord {
                    id: WeaponId(129),
                    ..blaster()
                },
                WeaponRecord {
                    id: WeaponId(130),
                    mass_dmg: 1000,
                    flags: PASSES_SHIELDS,
                    ..blaster()
                },
            ],
            hulls: vec![
                armed_hull(128, 128),
                HullRecord {
                    size: Some(120),
                    explode2: 7,
                    ..armed_hull(129, 129)
                },
                armed_hull(130, 130),
            ],
            traffic: traffic.traffic,
            dudes: vec![(
                DudeId(128),
                DudeRecord {
                    ai_type: 1,
                    govt: Some(GovtId(140)),
                    ships: vec![(ShipId(130), 1)],
                },
            )],
            ..shipbuying()
        };
        catalog.ship_records.push(ship(130, FAST));
        catalog
    }

    #[test]
    fn the_players_hull_is_its_ship_types() {
        let session = Session::start(&armed()).expect("starts");
        assert_eq!(session.hull(), HullSpec::new(&armed_hull(128, 128)));
        assert_eq!(session.hull().death_delay, 2);
    }

    #[test]
    fn a_ship_bought_fights_with_its_own_weapons_size_and_explosion() {
        let catalog = armed_shipyard();
        let mut session = outfitted(&catalog);
        session.buy_ship(NEW).expect("bought");
        session.take_off().expect("took off");
        session.hold_trigger(FIRE);
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(
            fired_by_the_player(&session.take_combat_events()),
            [WeaponId(129)],
            "the new ship's cannon, not the old one's blaster"
        );
        session.hold_trigger(Trigger::default());
        session.combat.clear();
        session.populate(&catalog, &mut Draws::of(&[6, 6, 0, 0, 750, 650, 180]));
        assert_eq!(session.npcs()[0].state.position, Vec2::new(0.0, -100.0));
        // The NPC fires straight down, passing 20 pixels from the player's
        // centre: beyond the old ship's hit radius (9.9), within the new
        // one's (39.6).
        session.player.position = Vec2::new(20.0, 0.0);
        let mut events = Vec::new();
        for _ in 0..20 {
            session.tick_traffic(&catalog, &Firing, &mut NeverFires);
            session.tick_combat(Rules::default(), &mut NeverFires);
            events.extend(session.take_combat_events());
        }
        assert_eq!(session.player_condition(), Condition::Destroyed);
        assert!(
            events.iter().any(|event| matches!(
                event,
                CombatEvent::Destroyed {
                    ship: ShipRef::Player,
                    ship_type: NEW,
                    explosion: Some(Explosion {
                        boom: BoomId(135),
                        extra: false
                    }),
                    ..
                }
            )),
            "{events:?}"
        );
    }

    // Targeting.

    use crate::targeting::TargetPick;

    /// Every NPC idles.
    #[derive(Debug)]
    struct Idling;

    impl Behaviour for Idling {
        fn decide(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            Goal::Idle
        }
    }

    /// `catalog`'s session with two NPCs: 0 at (100, -100) and 1 at
    /// (-50, 0), nearer the player at the centre.
    fn two_npcs(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        let draws = [6, 6, 0, 0, 850, 650, 0, 2, 6, 6, 0, 0, 700, 750, 0, 2];
        session.populate(catalog, &mut Draws::of(&draws));
        assert_eq!(npc_ids(&session), [NpcId(0), NpcId(1)]);
        session
    }

    fn target_id(session: &Session) -> Option<NpcId> {
        session.target().map(|npc| npc.id)
    }

    #[test]
    fn selecting_the_next_target_walks_the_npcs_then_none() {
        let catalog = trafficked(130, 2, 3);
        let mut session = two_npcs(&catalog);
        assert_eq!(target_id(&session), None);
        let picks: Vec<_> = (0..4)
            .map(|_| session.select_target(TargetPick::Next))
            .collect();
        assert_eq!(
            picks,
            [Some(NpcId(0)), Some(NpcId(1)), None, Some(NpcId(0))]
        );
        assert_eq!(target_id(&session), Some(NpcId(0)));
        assert_eq!(session.target().map(|npc| npc.ship), Some(ShipId(129)));
    }

    #[test]
    fn selecting_the_nearest_target_picks_the_nearest_npc() {
        let catalog = trafficked(130, 2, 3);
        let mut session = two_npcs(&catalog);
        assert_eq!(session.select_target(TargetPick::Nearest), Some(NpcId(1)));
        assert_eq!(target_id(&session), Some(NpcId(1)));
        session.player.position = Vec2::new(90.0, -90.0);
        assert_eq!(session.select_target(TargetPick::Nearest), Some(NpcId(0)));
    }

    #[test]
    fn selecting_the_nearest_threat_picks_the_nearest_npc_fighting_the_player() {
        let catalog = trafficked(130, 2, 3);
        let mut session = two_npcs(&catalog);
        // NPC 1 is nearer; NPC 0 attacks the player.
        let threatened = |session: &mut Session, goal: Goal| {
            session.traffic.npcs_mut()[0].goal = goal;
            session.target = None;
            session.select_target(TargetPick::NearestThreat)
        };
        assert_eq!(
            threatened(&mut session, Goal::Attack(ShipRef::Player)),
            Some(NpcId(0))
        );
        assert_eq!(
            threatened(&mut session, Goal::Flee(ShipRef::Player)),
            Some(NpcId(0)),
            "fleeing from it is a threat, as _IsThreatToPlayer says"
        );
        assert_eq!(
            threatened(&mut session, Goal::Inspect(ShipRef::Player)),
            None
        );
        session.target = Some(NpcId(1));
        assert_eq!(
            session.select_target(TargetPick::NearestThreat),
            Some(NpcId(1)),
            "none: the target unchanged"
        );
        session.traffic.npcs_mut()[0].goal = Goal::Attack(ShipRef::Player);
        session.traffic.npcs_mut()[0].condition = Condition::Disabled;
        session.target = None;
        assert_eq!(
            session.select_target(TargetPick::NearestThreat),
            None,
            "a disabled ship is no threat"
        );
        assert_eq!(session.select_target(TargetPick::Nearest), Some(NpcId(1)));
    }

    #[test]
    fn the_nearest_with_nothing_to_pick_leaves_the_target_as_it_was() {
        let catalog = trafficked(130, 2, 3);
        let mut session = Session::start(&catalog).expect("starts");
        session.target = Some(NpcId(7));
        assert_eq!(session.select_target(TargetPick::Nearest), Some(NpcId(7)));
        assert_eq!(session.target, Some(NpcId(7)));
    }

    #[test]
    fn nothing_is_targeted_while_landed() {
        let catalog = trafficked(130, 2, 3);
        let mut session = two_npcs(&catalog);
        session.land().expect("lands on planet 128");
        assert_eq!(session.select_target(TargetPick::Next), None);
        assert_eq!(session.select_target(TargetPick::Nearest), None);
        assert_eq!(target_id(&session), None);
    }

    #[test]
    fn the_target_is_kept_through_a_jump_and_cleared_on_arrival() {
        let mut catalog = trafficked(130, 2, 3);
        catalog.traffic.push((SystemId(131), catalog.traffic[0].1));
        let mut session = two_npcs(&catalog);
        session.select_target(TargetPick::Next);
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        session.begin_jump().expect("jumps");
        assert_eq!(session.select_target(TargetPick::Next), Some(NpcId(0)));
        assert_eq!(session.select_target(TargetPick::Nearest), Some(NpcId(0)));
        session.tick_traffic(&catalog, &Idling, &mut NeverFires);
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(target_id(&session), Some(NpcId(0)), "kept while jumping");
        session.arrive(&catalog, &mut NeverFires).expect("arrives");
        assert!(!session.npcs().is_empty());
        assert_eq!(target_id(&session), None, "cleared on arrival");
        assert_eq!(session.target, None);
    }

    #[test]
    fn the_target_is_kept_through_ticks_and_while_it_is_disabled() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        session.select_target(TargetPick::Next);
        for _ in 0..5 {
            session.tick(Controls::default());
            session.tick_traffic(&catalog, &Idling, &mut NeverFires);
            session.tick_combat(Rules::default(), &mut NeverFires);
        }
        assert_eq!(target_id(&session), Some(NpcId(0)));
        session.tick_combat(DISABLING, &mut NeverFires);
        assert_eq!(session.npcs()[0].condition, Condition::Disabled);
        assert_eq!(target_id(&session), Some(NpcId(0)), "a disabled target");
        assert_eq!(session.select_target(TargetPick::Nearest), Some(NpcId(0)));
    }

    #[test]
    fn the_target_is_cleared_as_soon_as_it_starts_breaking_up() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        session.select_target(TargetPick::Next);
        session.hold_trigger(FIRE);
        for _ in 0..200 {
            session.tick_combat(Rules::default(), &mut NeverFires);
            let events = session.take_combat_events();
            if events
                .iter()
                .any(|event| matches!(event, CombatEvent::BreakingUp { .. }))
            {
                assert_eq!(npc_ids(&session), [NpcId(0)], "still breaking up");
                assert_eq!(target_id(&session), None);
                assert_eq!(session.target, None);
                return;
            }
            assert_eq!(target_id(&session), Some(NpcId(0)));
        }
        panic!("never broke up: {:?}", session.npcs());
    }

    #[test]
    fn the_target_is_cleared_when_it_is_destroyed_and_gone() {
        let mut catalog = armed();
        catalog.hulls[1].death_delay = 0;
        let mut session = facing_an_npc(&catalog, 180);
        session.select_target(TargetPick::Next);
        session.traffic.npcs_mut()[0].reserves.armor.now = 0.0;
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(npc_ids(&session), [], "destroyed at once");
        assert_eq!(session.target, None);
    }

    #[test]
    fn the_target_is_cleared_when_it_lands_or_jumps_out() {
        for ai_type in [1, 3] {
            let catalog = trafficked(130, 1, ai_type);
            let mut session = Session::start(&catalog).expect("starts");
            session.populate(&catalog, &mut NeverFires);
            session.select_target(TargetPick::Next);
            assert_eq!(target_id(&session), Some(NpcId(0)));
            first_departure(&mut session, &catalog);
            assert_eq!(session.departed()[0].0, NpcId(0), "AI {ai_type}");
            assert_eq!(session.target, None, "AI {ai_type}");
        }
    }

    #[test]
    fn the_target_is_cleared_when_the_system_is_populated_afresh() {
        let catalog = trafficked(130, 2, 3);
        let mut session = two_npcs(&catalog);
        session.select_target(TargetPick::Next);
        session.populate(&catalog, &mut NeverFires);
        assert_eq!(session.target, None);
    }

    #[test]
    fn the_target_is_cleared_on_landing() {
        let catalog = trafficked(130, 2, 3);
        let mut session = two_npcs(&catalog);
        session.select_target(TargetPick::Next);
        session.land().expect("lands on planet 128");
        assert_eq!(npc_ids(&session), [NpcId(0), NpcId(1)]);
        assert_eq!(session.target, None);
    }

    // The secondary weapon.

    use crate::combat::weapon::SECONDARY;

    const ROCKET: WeaponId = WeaponId(140);
    const MISSILE: WeaponId = WeaponId(141);
    const TORCH: WeaponId = WeaponId(142);

    /// Secondary weapon `id`, a [`blaster`] spending `ammo_type`.
    fn secondary(id: i16, ammo_type: i16) -> WeaponRecord {
        WeaponRecord {
            id: WeaponId(id),
            flags: SECONDARY,
            ammo_type,
            ..blaster()
        }
    }

    /// One of each of `weapons`.
    fn carrying(id: i16, weapons: &[i16]) -> HullRecord {
        HullRecord {
            weapons: weapons
                .iter()
                .map(|&weapon| StockWeapon {
                    weapon: WeaponId(weapon),
                    count: 1,
                    ammo: 0,
                })
                .collect(),
            ..hull(id)
        }
    }

    /// `catalog` with ship 128 carrying a blaster (128) and three
    /// secondaries: rockets ([`ROCKET`]) firing rounds of their own, of
    /// which its default ammunition outfit 310 brings 3; missiles
    /// ([`MISSILE`]), unlimited; and a torch ([`TORCH`]) burning fuel. Ship
    /// 129 carries a blaster, a torch and missiles, in that order.
    fn with_secondaries(catalog: FakePilotCatalog) -> FakePilotCatalog {
        let mut outfits = catalog.outfits.clone();
        outfits.push(outfit(310, &[(MOD_AMMO, 140)]));
        FakePilotCatalog {
            weapons: vec![
                blaster(),
                secondary(140, 12),
                secondary(141, -1),
                secondary(142, -1100),
            ],
            hulls: vec![
                carrying(128, &[128, 140, 141, 142]),
                carrying(129, &[128, 142, 141]),
            ],
            outfits,
            defaults: vec![(ShipId(128), vec![(OutfitId(310), 3)])],
            ..catalog
        }
    }

    #[test]
    fn the_first_secondary_is_selected_at_start() {
        let session = Session::start(&with_secondaries(catalog())).expect("starts");
        assert_eq!(session.secondary(), Some(ROCKET));
        let unarmed = Session::start(&catalog()).expect("starts");
        assert_eq!(unarmed.secondary(), None);
        assert_eq!(unarmed.secondary_rounds(), None);
    }

    #[test]
    fn selecting_a_secondary_cycles_through_them_either_way() {
        let mut session = Session::start(&with_secondaries(catalog())).expect("starts");
        let mut picked = Vec::new();
        for backwards in [false, false, false, true, true] {
            session.select_secondary(backwards);
            picked.push(session.secondary());
        }
        assert_eq!(
            picked,
            [
                Some(MISSILE),
                Some(TORCH),
                Some(ROCKET),
                Some(TORCH),
                Some(MISSILE)
            ]
        );
    }

    #[test]
    fn holding_fire_fires_the_selected_secondary_and_the_primaries_as_asked() {
        let catalog = with_secondaries(catalog());
        for (primary, secondary, fired) in [
            (false, true, vec![ROCKET]),
            (true, false, vec![WeaponId(128)]),
            (true, true, vec![WeaponId(128), ROCKET]),
            (false, false, vec![]),
        ] {
            let mut session = Session::start(&catalog).expect("starts");
            session.hold_fire(primary, secondary);
            session.tick_combat(Rules::default(), &mut NeverFires);
            assert_eq!(
                fired_by_the_player(&session.take_combat_events()),
                fired,
                "{primary} {secondary}"
            );
        }
        let mut session = Session::start(&catalog).expect("starts");
        session.select_secondary(false);
        session.hold_fire(false, true);
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(
            fired_by_the_player(&session.take_combat_events()),
            [MISSILE]
        );
    }

    #[test]
    fn the_secondarys_rounds_follow_its_ammunition() {
        let mut session = Session::start(&with_secondaries(catalog())).expect("starts");
        assert_eq!(session.secondary_rounds(), Some(3));
        session.hold_fire(false, true);
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(session.secondary_rounds(), Some(2));
        session.select_secondary(false);
        assert_eq!(session.secondary_rounds(), None, "missiles: unlimited");
        session.select_secondary(false);
        assert_eq!(session.secondary_rounds(), None, "a torch burns fuel");
    }

    #[test]
    fn an_outfit_bought_keeps_the_secondary_selected() {
        let catalog = with_secondaries(outfitting());
        let mut session = outfitted(&catalog);
        session.select_secondary(false);
        assert_eq!(session.secondary(), Some(MISSILE));
        session.outfit(buy(SPEED)).expect("bought");
        assert_eq!(session.secondary(), Some(MISSILE));
    }

    #[test]
    fn a_ship_bought_keeps_the_secondary_it_carries_or_selects_its_first() {
        let catalog = with_secondaries(shipbuying());
        let mut session = outfitted(&catalog);
        session.buy_ship(NEW).expect("bought");
        assert_eq!(session.secondary(), Some(TORCH), "its first");
        let mut session = outfitted(&catalog);
        session.select_secondary(false);
        session.buy_ship(NEW).expect("bought");
        assert_eq!(session.secondary(), Some(MISSILE), "still carried");
    }

    // Guided weapons, turrets and point defence.

    /// `weapon` 130 (with blaster damage) of `guidance`, firing every
    /// `reload` ticks.
    fn guided(guidance: i16, reload: i16) -> WeaponRecord {
        WeaponRecord {
            id: WeaponId(130),
            reload,
            guidance,
            ..blaster()
        }
    }

    /// [`trafficked`] with the player carrying `player` and its traffic
    /// carrying `npc`, the player's ship type's `Flags` `flags`.
    fn arming(player: WeaponRecord, npc: WeaponRecord, flags: u16) -> FakePilotCatalog {
        FakePilotCatalog {
            weapons: vec![player, npc],
            hulls: vec![
                HullRecord {
                    flags,
                    ..armed_hull(128, player.id.0)
                },
                armed_hull(129, npc.id.0),
            ],
            ..trafficked(130, 1, 1)
        }
    }

    /// `catalog`'s session with its one NPC at (`x`, `y`) from the player
    /// (at the centre, facing up), facing `heading`.
    fn with_an_npc_at(catalog: &FakePilotCatalog, x: u32, y: u32, heading: u32) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.populate(catalog, &mut Draws::of(&[6, 6, 0, 0, x, y, heading]));
        session
    }

    #[test]
    fn the_player_fires_a_turret_at_its_target_astern_unless_its_ship_is_blind_there() {
        for (flags, fires) in [(0, true), (0x4000, false)] {
            let catalog = arming(guided(4, 2), blaster(), flags);
            let mut session = with_an_npc_at(&catalog, 750, 850, 0);
            assert_eq!(session.npcs()[0].state.position, Vec2::new(0.0, 100.0));
            session.select_target(TargetPick::Nearest);
            session.hold_fire(true, false);
            for _ in 0..12 {
                session.tick_combat(Rules::default(), &mut NeverFires);
            }
            let shield = session.npcs()[0].reserves.shield.now;
            assert_eq!(shield < 30.0, fires, "{flags:#x}: {shield}");
        }
        let catalog = arming(guided(4, 2), blaster(), 0);
        let mut session = with_an_npc_at(&catalog, 750, 850, 0);
        session.hold_fire(true, false);
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(session.shots(), [], "no target, no fire");
    }

    /// A homing missile: 10 pixels a tick for 60 ticks, turning 7 degrees
    /// a tick, standing 4 of point defence.
    fn homing_missile(id: i16, reload: i16) -> WeaponRecord {
        WeaponRecord {
            id: WeaponId(id),
            guidance: 1,
            reload,
            count: 60,
            speed: 1000,
            guided_turn: 70,
            durability: 4,
            ..blaster()
        }
    }

    #[test]
    fn the_players_homing_missile_hits_its_target_and_without_one_flies_through_it() {
        for (targeted, hit) in [(true, true), (false, false)] {
            let catalog = arming(homing_missile(134, 1000), blaster(), 0);
            let mut session = facing_an_npc(&catalog, 180);
            if targeted {
                session.select_target(TargetPick::Nearest);
            }
            session.hold_fire(true, false);
            for _ in 0..30 {
                session.tick_combat(Rules::default(), &mut NeverFires);
                session.hold_fire(false, false);
            }
            let shield = session.npcs()[0].reserves.shield.now;
            assert_eq!(shield < 30.0, hit, "{targeted}: {shield}");
            assert_eq!(
                session.shots().is_empty(),
                hit,
                "{targeted}: spent or flying on"
            );
        }
    }

    /// Every NPC idles, targets the player and holds its trigger.
    #[derive(Debug)]
    struct Attacking;

    impl Behaviour for Attacking {
        fn decide(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            Goal::Idle
        }

        fn trigger(&self, _npc: &Npc, _around: &Surroundings) -> Trigger {
            FIRE
        }

        fn target(&self, _npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
            Some(ShipRef::Player)
        }
    }

    #[test]
    fn the_players_point_defence_shoots_down_an_npcs_missile_with_no_target_of_its_own() {
        let quad = WeaponRecord {
            id: WeaponId(133),
            guidance: 9,
            reload: 5,
            count: 12,
            speed: 2000,
            mass_dmg: 1,
            energy_dmg: 4,
            ..blaster()
        };
        let slow = WeaponRecord {
            speed: 500,
            count: 200,
            ..homing_missile(134, 1000)
        };
        let catalog = arming(quad, slow, 0);
        let mut session = with_an_npc_at(&catalog, 750, 550, 180);
        assert_eq!(session.npcs()[0].state.position, Vec2::new(0.0, -200.0));
        let mut events = Vec::new();
        for _ in 0..40 {
            session.tick_traffic(&catalog, &Attacking, &mut NeverFires);
            session.tick_combat(Rules::default(), &mut NeverFires);
            events.extend(session.take_combat_events());
        }
        assert_eq!(session.target(), None, "the player targets nothing");
        assert!(
            events.contains(&CombatEvent::Fired {
                ship: ShipRef::Npc(NpcId(0)),
                weapon: WeaponId(134),
                at: Vec2::new(0.0, -200.0)
            }),
            "{events:?}"
        );
        let downed = events
            .iter()
            .filter(|event| matches!(event, CombatEvent::ShotDown { .. }))
            .count();
        assert_eq!(downed, 1, "{events:?}");
        assert_eq!(session.reserves().shield.now, 30.0, "untouched");
        assert!(
            session
                .shots()
                .iter()
                .all(|shot| shot.weapon.id != WeaponId(134)),
            "the missile is gone"
        );
    }

    // Fights, by Nova's AI.

    /// Traders (140, class 1: disabling costs 3), police allied with them
    /// (141, retreating by `Flags` 0x0010, `MaxOdds` 200), xenophobes
    /// (142) and neutrals (143, `CrimeTol` 6).
    fn skirmish_govts() -> Vec<GovtRecord> {
        let tolerant = |id| GovtRecord {
            crime_tol: 6,
            penalties: crate::catalog::Penalties {
                disable: 3,
                kill: 7,
                ..crate::catalog::Penalties::default()
            },
            ..crate::testkit::govt(id)
        };
        vec![
            GovtRecord {
                classes: [1, -1, -1, -1],
                ..tolerant(140)
            },
            GovtRecord {
                flags: crate::govt::WARSHIPS_RETREAT,
                allies: [1, -1, -1, -1],
                classes: [2, -1, -1, -1],
                max_odds: 200,
                ..tolerant(141)
            },
            GovtRecord {
                flags: crate::govt::XENOPHOBIC,
                ..tolerant(142)
            },
            tolerant(143),
        ]
    }

    /// A point-defence turret firing every 5 ticks at missiles within
    /// 360 pixels.
    fn point_defence() -> WeaponRecord {
        WeaponRecord {
            guidance: 9,
            reload: 5,
            count: 12,
            speed: 2000,
            mass_dmg: 1,
            energy_dmg: 4,
            ..weapon(133)
        }
    }

    /// A homing missile, 5 pixels a tick for 100 ticks, fired every 30.
    fn homing() -> WeaponRecord {
        WeaponRecord {
            guidance: 1,
            reload: 30,
            count: 100,
            speed: 500,
            guided_turn: 70,
            mass_dmg: 20,
            energy_dmg: 10,
            ..weapon(134)
        }
    }

    /// Ship `id` of strength `strength` carrying one of each of `weapons`.
    fn fitted(id: i16, strength: i16, weapons: &[i16]) -> HullRecord {
        HullRecord {
            strength,
            weapons: weapons
                .iter()
                .map(|&weapon| StockWeapon {
                    weapon: WeaponId(weapon),
                    count: 1,
                    ammo: 0,
                })
                .collect(),
            ..hull(id)
        }
    }

    /// System 130 (governed by `system_govt`) trafficked by `dudes`, each
    /// (AI type, government, ship) at an equal share; the player's tough
    /// ship 128 (300 shield, 450 armour) carries a blaster and point
    /// defence. Ship 129 is an unarmed trader, 130-132 carry a blaster,
    /// 133 and 134 a homing missile and 135 point defence.
    fn skirmish(dudes: &[(i16, i16, i16)], system_govt: Option<i16>) -> FakePilotCatalog {
        let mut dude_types = [(-1, 0); 8];
        for (slot, _) in dudes.iter().enumerate() {
            dude_types[slot] = (128 + slot as i16, 100 / dudes.len() as i16);
        }
        let mut catalog = FakePilotCatalog {
            ships: vec![(
                ShipId(128),
                Ok(ShipFields {
                    shield: 300,
                    armor: 450,
                    ..FAST
                }),
            )],
            weapons: vec![blaster(), point_defence(), homing()],
            hulls: vec![
                fitted(128, 100, &[128, 133]),
                fitted(129, 10, &[]),
                fitted(130, 100, &[128]),
                fitted(131, 100, &[128]),
                fitted(132, 100, &[128]),
                fitted(133, 100, &[134]),
                fitted(134, 10, &[134]),
                fitted(135, 100, &[133]),
            ],
            govts: skirmish_govts(),
            traffic: vec![(
                SystemId(130),
                SystemTraffic {
                    dude_types,
                    avg_ships: dudes.len() as i16,
                },
            )],
            dudes: dudes
                .iter()
                .enumerate()
                .map(|(slot, &(ai_type, govt, ship))| {
                    (
                        DudeId(128 + slot as i16),
                        DudeRecord {
                            ai_type,
                            govt: Some(GovtId(govt)),
                            ships: vec![(ShipId(ship), 1)],
                        },
                    )
                })
                .collect(),
            ship_records: (129..=135).map(|id| ship(id, FAST)).collect(),
            ..catalog()
        };
        catalog.star_map[0].govt = system_govt.map(GovtId);
        catalog
    }

    /// Setup draws placing, in order, düde `slot` of `of` at (`x`, `y`)
    /// facing `heading`, each of aggression 2.
    fn placing(of: usize, ships: &[(usize, i32, i32, u32)]) -> Draws {
        let share = 100 / of as u32;
        let draws: Vec<u32> = ships
            .iter()
            .flat_map(|&(slot, x, y, heading)| {
                [
                    6,
                    6,
                    slot as u32 * share,
                    0,
                    (x + 750) as u32,
                    (y + 750) as u32,
                    heading,
                    0,
                ]
            })
            .collect();
        Draws::of(&draws)
    }

    /// A tick of the fight, then of Nova's traffic.
    fn fight(session: &mut Session, catalog: &FakePilotCatalog) {
        session.tick_combat(Rules::default(), &mut NeverFires);
        session.tick_traffic(catalog, &crate::ai::NovaAi::default(), &mut NeverFires);
    }

    #[test]
    fn the_players_attack_on_a_trader_puts_it_to_flight_and_brings_the_police() {
        // The trader 100 ahead of the player; the police far behind.
        let catalog = skirmish(&[(1, 140, 129), (4, 141, 130)], Some(140));
        let mut session = Session::start(&catalog).expect("starts");
        session.populate(
            &catalog,
            &mut placing(2, &[(0, 0, -100, 0), (1, 0, 740, 0)]),
        );
        let (trader, police) = (NpcId(0), NpcId(1));
        let goal_of = |session: &Session, id| {
            session
                .npcs()
                .iter()
                .find(|npc| npc.id == id)
                .map(|npc| npc.goal)
        };
        session.hold_trigger(FIRE);
        let mut fled = false;
        let mut picked_while_fleeing = None;
        let mut disabled_at = None;
        for tick in 0..400 {
            fight(&mut session, &catalog);
            if goal_of(&session, trader) == Some(Goal::Flee(ShipRef::Player)) && !fled {
                fled = true;
                picked_while_fleeing = session.select_target(TargetPick::NearestThreat);
            }
            let downed = session
                .npcs()
                .iter()
                .any(|npc| npc.id == trader && npc.condition == Condition::Disabled);
            if downed && disabled_at.is_none() {
                disabled_at = Some(tick);
                session.hold_trigger(Trigger::default());
                assert_eq!(
                    [140, 141, 142, 143].map(|govt| session.pilot().legal_record(GovtId(govt))),
                    [-3, -3, 1, 1],
                    "the traders and their allies the police lower; the xenophobes and \
                     the neutrals, not allied with them, pleased by half theirs"
                );
                let npc_at = |id| {
                    session
                        .npcs()
                        .iter()
                        .find(|npc| npc.id == id)
                        .map(|npc| npc.state.position.length())
                };
                assert!(npc_at(trader) < npc_at(police), "the trader is nearer");
                assert_eq!(
                    session.select_target(TargetPick::NearestThreat),
                    Some(police),
                    "the disabled trader is no threat; the police are"
                );
            }
            if disabled_at.is_some() && session.reserves().shield.now < 300.0 {
                break;
            }
        }
        assert!(fled, "the trader fled from the player");
        assert_eq!(
            picked_while_fleeing,
            Some(trader),
            "a ship fleeing from the player is a threat to it"
        );
        assert!(disabled_at.is_some(), "the trader was disabled");
        assert_eq!(
            goal_of(&session, police),
            Some(Goal::Attack(ShipRef::Player))
        );
        assert!(
            session.reserves().shield.now < 300.0,
            "the police's shots reached the player: {:?}",
            session.reserves()
        );
        let saved = crate::save::decode(&crate::save::encode(session.pilot())).expect("reads");
        assert_eq!(&saved, session.pilot());
    }

    #[test]
    fn a_neutral_warship_hunts_a_player_wanted_below_its_tolerance() {
        let catalog = skirmish(&[(3, 143, 132)], Some(143));
        for (record, hunted) in [(-6, false), (-7, true)] {
            let mut session = Session::start(&catalog).expect("starts");
            session.pilot.set_legal_record(GovtId(143), record);
            session.populate(&catalog, &mut placing(1, &[(0, 300, 0, 0)]));
            fight(&mut session, &catalog);
            assert_eq!(
                session.npcs()[0].goal == Goal::Attack(ShipRef::Player),
                hunted,
                "{record}: {:?}",
                session.npcs()[0].goal
            );
        }
    }

    #[test]
    fn a_xenophobes_warship_attacks_the_player_and_the_police_whichever_is_nearer() {
        let catalog = skirmish(&[(3, 142, 131), (4, 141, 130)], Some(140));
        let mut session = Session::start(&catalog).expect("starts");
        session.populate(
            &catalog,
            &mut placing(2, &[(0, 0, -300, 0), (1, 700, 700, 0)]),
        );
        fight(&mut session, &catalog);
        assert_eq!(session.npcs()[0].goal, Goal::Attack(ShipRef::Player));
        let mut session = Session::start(&catalog).expect("starts");
        session.populate(
            &catalog,
            &mut placing(2, &[(0, 0, -300, 0), (1, 0, -350, 0)]),
        );
        fight(&mut session, &catalog);
        assert_eq!(session.npcs()[0].goal, Goal::Attack(ShipRef::Npc(NpcId(1))));
    }

    /// Every NPC of AI type 1 attacks NPC `at`, firing its missile; the
    /// rest idle.
    #[derive(Debug)]
    struct Volley {
        at: ShipRef,
    }

    impl Behaviour for Volley {
        fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            if npc.ai_type == crate::traffic::npc::AiType::WimpyTrader {
                Goal::Attack(self.at)
            } else {
                Goal::Idle
            }
        }

        fn trigger(&self, npc: &Npc, _around: &Surroundings) -> Trigger {
            if npc.ai_type == crate::traffic::npc::AiType::WimpyTrader {
                Trigger {
                    only: Some(WeaponId(134)),
                    ..Trigger::default()
                }
            } else {
                Trigger::default()
            }
        }

        fn target(&self, npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
            npc.goal.quarry()
        }
    }

    /// Whether point defence shot a missile down over `ticks` of `session`
    /// under `volley`.
    fn shot_down(session: &mut Session, catalog: &FakePilotCatalog, volley: &Volley) -> bool {
        let mut downed = false;
        for _ in 0..60 {
            session.tick_traffic(catalog, volley, &mut NeverFires);
            session.tick_combat(Rules::default(), &mut NeverFires);
            downed |= session
                .take_combat_events()
                .iter()
                .any(|event| matches!(event, CombatEvent::ShotDown { .. }));
        }
        downed
    }

    #[test]
    fn point_defence_engages_a_xenophobes_missile_but_not_an_allys() {
        // A xenophobe's missile boat (AI 1 here, so the volley fires it)
        // 300 above the player.
        let catalog = skirmish(&[(1, 142, 133)], Some(140));
        let mut session = Session::start(&catalog).expect("starts");
        session.populate(&catalog, &mut placing(1, &[(0, 0, -300, 180)]));
        let at_player = Volley {
            at: ShipRef::Player,
        };
        assert!(shot_down(&mut session, &catalog, &at_player));
        // A trader's missile at the police, allied: their point defence
        // lets it be, and it hits.
        let catalog = skirmish(&[(1, 140, 134), (4, 141, 135)], Some(140));
        let mut session = Session::start(&catalog).expect("starts");
        session.populate(
            &catalog,
            &mut placing(2, &[(0, 0, -600, 180), (1, 0, -300, 0)]),
        );
        let at_police = Volley {
            at: ShipRef::Npc(NpcId(1)),
        };
        assert!(!shot_down(&mut session, &catalog, &at_police));
        assert!(session.npcs()[1].reserves.shield.now < 30.0, "it hit");
    }
}
