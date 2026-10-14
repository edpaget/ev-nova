//! The session's side of the story runtime: the story events raised from
//! their sources and waiting for the next settling, the ships watched for
//! their first sighting (see the `story` module's "Story events"), the
//! daily step's participants run over the session's own state (its "Daily
//! step"), and the galaxy overlay's one writer, with the rule that keeps
//! the player in a visible system (its "Galaxy overlay"): one core with
//! two entries, [`Session::change_galaxy`] for any change, with a catalog,
//! and `Session::set_stellar_state`, catalog-free, for a stellar's
//! destroyed state alone.

use std::collections::{BTreeSet, VecDeque};

use super::entry::Dock;
use super::missions;
use super::script_effects::Settled;
use super::{ScriptNote, Session, SessionEvent};
use crate::catalog::{GovtId, LandingSite, PilotCatalog, StellarId, SystemId, TrafficCatalog};
use crate::chance::Chance;
use crate::control::{Script, Test};
use crate::market;
use crate::pilot::Pilot;
use crate::rulebook::{RuleKey, RuleSource};
use crate::story::day::{self, Day, Participant};
use crate::story::dispatch::{self, DuplicateSubscription, Reentry, Registry, Stage};
use crate::story::galaxy::hidden_by;
use crate::story::{
    Consumer, Countdown, Fate, GalaxyOverlay, HiddenMove, OverlaidGalaxy, StoryEvent, Subscription,
    SubscriptionId,
};
use crate::traffic::npc::NpcId;
use crate::{hire, outfitter, shipyard, wares};

/// Makes `to`'s system visible on `pilot`'s overlay, and so in `galaxy`,
/// when `to` says it must be: whether it did.
pub(super) fn show(pilot: &mut Pilot, galaxy: &mut OverlaidGalaxy, to: HiddenMove) -> bool {
    if to.make_visible {
        pilot.galaxy.set_visible(to.to, Some(true));
        galaxy.set_overlay(pilot.galaxy.clone());
    }
    to.make_visible
}

/// Moves `pilot` off its system when `galaxy` hides it, as the original
/// does (@0x998f1-0x99a36): to the visible system at the same place, or
/// to the chain head made visible ([`OverlaidGalaxy::hidden_move`]). Only
/// the system changes. Whether the pilot is in another system now.
pub(super) fn leave_hidden(pilot: &mut Pilot, galaxy: &mut OverlaidGalaxy) -> bool {
    let Some(to) = galaxy.hidden_move(pilot.system) else {
        return false;
    };
    show(pilot, galaxy, to);
    let moved = to.to != pilot.system;
    pilot.system = to.to;
    moved
}

/// The session's story runtime: the subscriptions registered, the events
/// raised and not yet settled, and the ships watched for their first
/// sighting. None of it is saved: the consumers register again as a pilot
/// is flown, and what has fired once is the pilot's.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct StoryRuntime {
    /// The subscriptions registered.
    registry: Registry,
    /// The events raised since the last settling, in the order raised.
    raised: VecDeque<StoryEvent>,
    /// The NPCs whose first sighting raises [`Fate::Observed`].
    pub(super) watched: BTreeSet<NpcId>,
}

/// The session settling, with the catalog and the chance it settles with
/// and what it has settled: the story dispatcher's [`Stage`].
struct SessionStage<'a, C: ?Sized> {
    session: &'a mut Session,
    catalog: &'a C,
    chance: &'a mut dyn Chance,
    settled: &'a mut Settled,
}

impl<C: PilotCatalog + TrafficCatalog + ?Sized> Stage for SessionStage<'_, C> {
    fn matching(&self, event: &StoryEvent) -> Vec<SubscriptionId> {
        self.session.story.registry.matching(event)
    }

    fn subscription(&self, id: SubscriptionId) -> Option<Subscription> {
        self.session.story.registry.get(id).cloned()
    }

    fn fired(&self, id: SubscriptionId) -> bool {
        self.session.pilot.story_fired.contains(&id)
    }

    fn mark_fired(&mut self, id: SubscriptionId) {
        self.session.pilot.story_fired.insert(id);
        self.session.save_due = true;
    }

    fn holds(&self, guard: &Test) -> bool {
        self.session.gate().allows(guard)
    }

    /// The one match on the consumer that routes its own effects, which
    /// are not set expressions (see the `story` module's "Dispatch").
    fn effect(&mut self, id: SubscriptionId, event: &StoryEvent) {
        match id.consumer {
            // The offer roll (see the `missions` module).
            // The offer roll, the galaxy refreshed first when it rolls
            // (`_RandomizeMissionData` @0xa175f; see the `missions`
            // module).
            Consumer::Offers => {
                if let (missions::ROLL, &StoryEvent::Landed { stellar }) = (id, event) {
                    if self.session.offers.due_at(stellar) {
                        self.session.refresh_galaxy(self.catalog);
                    }
                    self.session.roll_offers(stellar, &mut *self.chance);
                }
            }
            // The galaxy refresh after the land check (@0xa1af2) and the
            // re-offer, and each slot's landing hooks and deadline checks
            // (see the `missions` module).
            Consumer::Missions => match (id, event) {
                (missions::REOFFER, StoryEvent::Landed { .. }) => {
                    self.session.refresh_galaxy(self.catalog);
                    self.session.reoffer_missions(&mut *self.chance);
                }
                _ => self.session.mission_effect(id, event, &mut *self.chance),
            },
            // Each slot's enter-system hook (see the `mission_ships`
            // module).
            Consumer::MissionShips => {
                self.session
                    .mission_ships_effect(id, event, &mut *self.chance);
            }
            // Each other consumer's arm is filled by its own phase: the
            // crons', nebulae's and plug-ins'. A fixture has none.
            Consumer::Crons | Consumer::Nebulae | Consumer::Plugins | Consumer::Fixture => {}
        }
    }

    fn run(&mut self, script: &Script, between: &mut dyn FnMut(&mut dyn Stage)) {
        let Some(expr) = script.tree() else {
            return;
        };
        let SessionStage {
            session,
            catalog,
            chance,
            settled,
        } = self;
        session.run_ops(expr, &mut **chance, &mut |session, chance| {
            between(&mut SessionStage {
                session,
                catalog: *catalog,
                chance,
                settled,
            });
        });
    }

    fn settle_moves(&mut self) {
        self.session
            .settle_moves(self.catalog, &mut *self.chance, self.settled);
    }

    fn settle_leave(&mut self) -> bool {
        self.session
            .settle_leave(self.catalog, &mut *self.chance, self.settled)
    }

    fn take_raised(&mut self) -> VecDeque<StoryEvent> {
        std::mem::take(&mut self.session.story.raised)
    }
}

/// One day of the session, its draws on `chance`.
struct SessionDay<'a, C: Chance + ?Sized> {
    session: &'a mut Session,
    chance: &'a mut C,
}

impl<C: Chance + ?Sized> Day for SessionDay<'_, C> {
    fn run(&mut self, participant: Participant) {
        let session = &mut *self.session;
        match participant {
            Participant::Date => session.pilot.date = session.pilot.date.next_day(),
            // One pass over the crons, each script run at once (see the
            // `crons` module).
            Participant::Crons => session.crons_day(&mut *self.chance),
            Participant::Events => {
                // The events step out of the pilot, so that their
                // `ActivateOn` can read the pilot through the session's
                // control bits: no test reads the events.
                let mut events = std::mem::take(&mut session.pilot.events);
                market::step_day(&session.goods, &mut events, session.gate(), self.chance);
                session.pilot.events = events;
            }
            // The destroyed stellars' regeneration; the dominated ones'
            // defence regrowing joins it with rdm task planet-domination.
            Participant::Stellars => session.stellars_day(&mut *self.chance),
            Participant::DayRolls => session.draw_day_rolls(&mut self.chance),
            Participant::DayPassed => session.raise(StoryEvent::DayPassed {
                date: session.pilot.date,
            }),
            // The active missions' days left; their deadlines fail on the
            // missions' `DayPassed` subscriptions.
            Participant::MissionDays => session.pilot.missions.count_down(),
            // The active ranks' salaries (see the `ranks` module).
            Participant::Ranks => session.ranks_day(),
            // Each slot is filled by its own phase: the tribute by rdm
            // task planet-domination, and the reinforcements once they
            // are built.
            Participant::Tribute | Participant::Reinforcements => {}
        }
    }
}

impl Session {
    /// Raises `event` from its one source: it waits, with the others
    /// raised, for the next settling ([`Session::settle_script`]). A
    /// source only raises; nothing here dispatches.
    pub(super) fn raise(&mut self, event: StoryEvent) {
        self.story.raised.push_back(event);
    }

    /// How many events have been raised and not yet settled: a mark to
    /// raise earlier ones at ([`Session::raise_at`]).
    pub(super) fn story_mark(&self) -> usize {
        self.story.raised.len()
    }

    /// Raises `event` at `mark`, before the events raised since: where a
    /// source learns of an event only after later ones.
    pub(super) fn raise_at(&mut self, mark: usize, event: StoryEvent) {
        self.story.raised.insert(mark, event);
    }

    /// Watches the NPC `ship`: the next flight tick of the traffic that
    /// finds it in the system and observed raises its first sighting,
    /// [`Fate::Observed`], once, and forgets it. Observed is seen by the
    /// player ([`Session::sees`]: not waiting at the gate it comes out of
    /// ([`Npc::emerging`](crate::traffic::npc::Npc::emerging)), and uncloaked
    /// or within a cloak scanner's sight) and, for a ship that can cloak,
    /// on screen ([`Session::on_screen`]); until then it stays watched. A
    /// ship no longer in the system is forgotten, and populating the
    /// system or resetting its traffic forgets every one, its ships
    /// replaced (see the `story` module's findings).
    pub fn watch_ship(&mut self, ship: NpcId) {
        self.story.watched.insert(ship);
    }

    /// Raises the first sighting of each watched ship in the system that
    /// the player observes (`_ShipVisibleToShip` @0x971c, which hides a
    /// ship waiting at its gate @0x972d, as `_MissionObjectivesCheck`
    /// @0x9ea5c asks it, and on screen
    /// for one that can cloak, @0x9ea74-0x9eb09), forgetting it; keeps
    /// watching one not yet observed, and forgets one no longer here.
    pub(super) fn observe_watched(&mut self) {
        for ship in std::mem::take(&mut self.story.watched) {
            let Some(npc) = self.traffic.npcs().iter().find(|npc| npc.id == ship) else {
                continue;
            };
            if self.observes(npc) {
                self.raise(StoryEvent::ShipFate {
                    ship,
                    fate: Fate::Observed,
                });
            } else {
                self.story.watched.insert(ship);
            }
        }
    }

    /// The subscriptions `event` would run now, in the order they run.
    #[cfg(test)]
    pub(super) fn story_matching(&self, event: &StoryEvent) -> Vec<SubscriptionId> {
        self.story.registry.matching(event)
    }

    /// Lets go of the events raised, undispatched: a test's clean slate.
    #[cfg(test)]
    pub(super) fn drop_raised(&mut self) {
        self.story.raised.clear();
    }

    /// Registers `subscription`, which runs from the next event it matches
    /// (see the `story` module's "Dispatch").
    ///
    /// # Errors
    ///
    /// [`DuplicateSubscription`] when one is registered under its id
    /// already, which is kept.
    pub fn register_story(
        &mut self,
        subscription: Subscription,
    ) -> Result<(), DuplicateSubscription> {
        self.story.registry.register(subscription)
    }

    /// Unregisters the subscription `id`, and clears its once-fired entry,
    /// so one registered under the same id again starts fresh: whether one
    /// was registered. Clearing a fired entry makes a save due.
    pub fn unregister_story(&mut self, id: SubscriptionId) -> bool {
        if self.pilot.story_fired.remove(&id) {
            self.save_due = true;
        }
        self.story.registry.unregister(id).is_some()
    }

    /// Settles what the set expressions left and the events raised,
    /// through the story dispatcher, as [`RuleKey::StoryReentry`] reads:
    /// the moves, each read from `catalog`, then the events, each
    /// dispatched in turn, then a `Q`'s leave, noted on `settled`, and so
    /// on until nothing is left. Events dropped past the limit are told
    /// once, as [`ScriptNote::EventsDropped`].
    pub(super) fn settle_story<C>(
        &mut self,
        catalog: &C,
        chance: &mut dyn Chance,
        settled: &mut Settled,
    ) where
        C: PilotCatalog + TrafficCatalog + ?Sized,
    {
        let reentry = Reentry::from(self.rules.source_for(RuleKey::StoryReentry));
        let dropped = dispatch::settle(
            &mut SessionStage {
                session: self,
                catalog,
                chance,
                settled,
            },
            reentry,
        );
        if dropped > 0 {
            self.script_notes
                .push(ScriptNote::EventsDropped { dropped });
        }
    }

    /// The events raised and not yet settled, in order.
    #[cfg(test)]
    pub(crate) fn story_raised(&self) -> Vec<StoryEvent> {
        self.story.raised.iter().copied().collect()
    }

    /// Changes the galaxy overlay as `change` says: the one writer's entry
    /// for any change (see the `story` module's "Galaxy overlay"). A
    /// change that changes nothing does nothing more. Otherwise the
    /// overlaid map is rebuilt, and a
    /// player whose system is now hidden moves as the original moves it
    /// ([`OverlaidGalaxy::hidden_move`]): only its system changes, and no
    /// story event is raised, no system explored and no day passed. In
    /// flight [`SessionEvent::SystemChanged`] is told; landed, the
    /// spaceport stays its stellar's and the new system's stellars wait
    /// for the take-off, as after a landed move. The stellars are read again
    /// from `catalog` when the player moved or theirs changed government
    /// or were destroyed or regenerated;
    /// when the player's system or its government changed, the traffic
    /// goes on with every ship in the system, its arrivals drawn from the
    /// new table ([`Traffic::retarget`](crate::Traffic::retarget)). A save
    /// is due, and [`SessionEvent::GalaxyChanged`] is told once. Whether
    /// it changed anything.
    ///
    /// A visibility written here lasts only until the next refresh of the
    /// systems' `Visibility` ([`GalaxyOverlay::refresh_visibility`]), at
    /// a landing, a jump's arrival, a new pilot's start or a load, which
    /// overwrites every system's.
    pub fn change_galaxy<C>(&mut self, catalog: &C, change: impl FnOnce(&mut GalaxyOverlay)) -> bool
    where
        C: PilotCatalog + TrafficCatalog + ?Sized,
    {
        let govt = self.galaxy.govt(self.pilot.system);
        let Some(before) = self.write_galaxy(change) else {
            return false;
        };
        self.galaxy_changed(catalog, &before, govt);
        true
    }

    /// The systems the galaxy refresh hides now: each catalog system whose
    /// `Visibility` fails for the pilot ([`hidden_by`]).
    pub(super) fn hidden_now(&self) -> BTreeSet<SystemId> {
        hidden_by(
            self.galaxy.catalog_systems(),
            &self.visibility,
            &self.gate(),
        )
    }

    /// The galaxy refresh at a landing or a new pilot's start
    /// (`_PropagateMissionBitEffects` @0x99676): every system's visibility
    /// written from its `Visibility` through the one writer, then, when
    /// that changed anything, what any change does
    /// ([`Session::change_galaxy`]): the player moved out of a system now
    /// hidden, the traffic retargeted, [`SessionEvent::GalaxyChanged`]
    /// told and a save due. Stellar ownership follows from the overlay
    /// ([`OverlaidGalaxy::owner`]), derived before the move. Whether it
    /// changed anything.
    pub(super) fn refresh_galaxy<C>(&mut self, catalog: &C) -> bool
    where
        C: PilotCatalog + TrafficCatalog + ?Sized,
    {
        let hidden = self.hidden_now();
        self.change_galaxy(catalog, |overlay| {
            overlay.set_hidden(&hidden);
        })
    }

    /// A load's galaxy refresh (`_LoadPilotData` @0x751ca, @0x7531f): every
    /// system's visibility written from its `Visibility`, the saved flags
    /// never trusted, telling nothing and making no save due, as the
    /// load's rebuild tells nothing and the original's load saves nothing.
    /// A player whose system is now hidden moves as the original moves it
    /// ([`leave_hidden`]), docked or not. When the refresh changed
    /// anything, the stellars are rebuilt quietly after it, as the load's
    /// rebuild follows its refresh (@0x751ca, @0x751cf): in flight, the
    /// stellars shown are those the refreshed galaxy gives.
    pub(super) fn refresh_galaxy_quietly(&mut self, catalog: &(impl PilotCatalog + ?Sized)) {
        let hidden = self.hidden_now();
        if !self.pilot.galaxy.set_hidden(&hidden) {
            return;
        }
        self.galaxy.set_overlay(self.pilot.galaxy.clone());
        if leave_hidden(&mut self.pilot, &mut self.galaxy) {
            let system = self.pilot.system;
            let sites = self.read_sites(catalog, system);
            self.set_player_system(system, sites, Dock::AsItWas);
        }
        self.rebuild_quietly();
    }

    /// What any change to the overlay from `before` does once written, the
    /// player's system governed by `govt` before it (see
    /// [`Session::change_galaxy`]).
    fn galaxy_changed<C>(&mut self, catalog: &C, before: &GalaxyOverlay, govt: Option<GovtId>)
    where
        C: PilotCatalog + TrafficCatalog + ?Sized,
    {
        self.refresh_sites(catalog, before);
        let moved = leave_hidden(&mut self.pilot, &mut self.galaxy);
        // The original rewrites only the system numbers (@0x998f1-0x99a36):
        // docked, the spaceport open stays the stellar's.
        if moved {
            let system = self.pilot.system;
            let sites = self.read_sites(catalog, system);
            if self.set_player_system(system, sites, Dock::AsItWas) {
                self.emit(SessionEvent::SystemChanged);
            }
        }
        if moved || self.galaxy.govt(self.pilot.system) != govt {
            let table = self.spawn_table(catalog);
            let every: BTreeSet<NpcId> = self.traffic.npcs().iter().map(|npc| npc.id).collect();
            self.traffic.retarget(table, &every);
        }
        self.emit(SessionEvent::GalaxyChanged);
    }

    /// The writer's one core, for both its entries: changes the pilot's
    /// overlay as `change` says and, when that changed anything, the
    /// galaxy's, rebuilding the overlaid map, and makes a save due. It
    /// tells nothing: each entry tells what its change calls for. The
    /// overlay as it was, when it changed.
    pub(super) fn write_galaxy(
        &mut self,
        change: impl FnOnce(&mut GalaxyOverlay),
    ) -> Option<GalaxyOverlay> {
        let before = self.pilot.galaxy.clone();
        change(&mut self.pilot.galaxy);
        if self.pilot.galaxy == before {
            return None;
        }
        self.galaxy.set_overlay(self.pilot.galaxy.clone());
        self.save_due = true;
        Some(before)
    }

    /// Sets `stellar` destroyed with the countdown `destroyed`, or not
    /// destroyed with `None`: the one writer's catalog-free entry, which
    /// can change a stellar's destroyed state alone, so it never hides a
    /// system, moves the player or changes a government (see the `story`
    /// module's "Galaxy overlay"). A change makes a save due. When the
    /// stellar is destroyed or regenerated, not only counted down, the
    /// stellars shown and those waiting for the take-off are stamped
    /// afresh and [`SessionEvent::GalaxyChanged`] is told once. Whether it
    /// changed anything.
    pub(super) fn set_stellar_state(
        &mut self,
        stellar: StellarId,
        destroyed: Option<Countdown>,
    ) -> bool {
        let Some(before) = self.write_galaxy(|overlay| {
            overlay.set_destroyed(stellar, destroyed);
        }) else {
            return false;
        };
        if before.stellar(stellar).destroyed.is_some() != destroyed.is_some() {
            self.stamp_sites();
            self.emit(SessionEvent::GalaxyChanged);
        }
        true
    }

    /// Stamps the stellars shown, those their system lists, and those
    /// waiting for the take-off afresh
    /// through the galaxy ([`OverlaidGalaxy::site`]): only whether each is
    /// destroyed can have changed, which the overlay alone gives, so
    /// nothing is read again. Which stellars are shown is
    /// [`Session::set_player_system`]'s alone; this only stamps them.
    fn stamp_sites(&mut self) {
        let galaxy = &self.galaxy;
        for site in self
            .sites
            .iter_mut()
            .chain(&mut self.listed)
            .chain(self.next_sites.iter_mut().flatten())
        {
            *site = galaxy.site(*site);
        }
    }

    /// Reads the stellars again from `catalog` where the galaxy's change
    /// from `before` gave one of them another government, or destroyed or
    /// regenerated it: those the system shown lists, and those waiting for
    /// the take-off. Which stellars are shown is
    /// [`Session::set_player_system`]'s and the rebuild points' alone: the
    /// same are shown, read again.
    fn refresh_sites(&mut self, catalog: &(impl PilotCatalog + ?Sized), before: &GalaxyOverlay) {
        let now = &self.pilot.galaxy;
        let changed = |sites: &[LandingSite]| {
            sites.iter().any(|site| {
                let (was, is) = (before.stellar(site.id), now.stellar(site.id));
                was.govt != is.govt || was.destroyed.is_some() != is.destroyed.is_some()
            })
        };
        let shown_stale = changed(&self.listed);
        let waiting_stale = self.next_sites.as_deref().is_some_and(changed);
        if shown_stale {
            self.listed = self.read_sites(catalog, self.shown);
            let held: Vec<StellarId> = self.stellars_shown().collect();
            self.sites = self
                .listed
                .iter()
                .filter(|site| held.contains(&site.id))
                .copied()
                .collect();
        }
        if waiting_stale {
            let sites = self.read_sites(catalog, self.pilot.system);
            if let Some(waiting) = &mut self.next_sites {
                *waiting = sites;
            }
        }
    }

    /// The course as it is flown: each hop the visible system that stands
    /// for it ([`OverlaidGalaxy::same_place`]), cut at the first hop with
    /// none. The course saved ([`Session::course`]) keeps the systems
    /// plotted, so a hop hidden and shown again is followed again, as the
    /// original's `_CueNextHyperRouteDest` resolves each hop by place
    /// (@0xe2fa).
    #[must_use]
    pub fn course_shown(&self) -> Vec<SystemId> {
        self.pilot
            .course
            .iter()
            .map_while(|&hop| self.galaxy.same_place(hop))
            .collect()
    }

    /// Lets `days` days go by through the daily step, landed or in
    /// flight, the events rolled on `chance`, as a storyline walker waits:
    /// each day moves the date on, counts the active missions' days left
    /// down and raises a [`StoryEvent::DayPassed`] for the next settling
    /// ([`Session::settle_script`]) to dispatch, the missions' deadlines
    /// among its subscriptions. Unlike [`Session::set_date`], which lets
    /// no day go by, every participant of each day runs. No escort's wage
    /// is paid: the daily step pays none, and the take-off pays its own
    /// once after its days. A lapsed deadline fails its mission only in
    /// flight; docked it waits for the take-off, as the missions' deadline
    /// check says.
    pub fn let_days_pass(&mut self, days: u32, chance: &mut (impl Chance + ?Sized)) {
        self.pass_days(days, chance);
    }

    /// Lets `days` days go by through the daily step, each running its
    /// participants in the original's order, the events rolled on
    /// `chance`. No wage is paid here: the caller pays once after its
    /// days, as the original does.
    pub(super) fn pass_days(&mut self, days: u32, chance: &mut (impl Chance + ?Sized)) {
        day::pass_days(
            days,
            &mut SessionDay {
                session: self,
                chance,
            },
        );
    }

    /// The daily step's day rolls, as [`RuleKey::DayRolls`] reads: by the
    /// engine, every ship class's hire and buy roll and every outfit's
    /// buy roll drawn afresh on `chance` in the original's order
    /// ([`wares::draw_day`], `_IncrementGameTime` @0xb7c4-0xb833), each
    /// read as the bar and the shops read it. Every redraw due is dropped
    /// with them: the day drew those classes afresh already. By the other
    /// reading, nothing: a landing forgets the rolls instead.
    fn draw_day_rolls(&mut self, chance: &mut dyn Chance) {
        if self.rules.source_for(RuleKey::DayRolls) != RuleSource::Engine {
            return;
        }
        let buy_random = self.rules.source_for(RuleKey::BuyRandom);
        wares::draw_day(
            &mut self.hire_rolls,
            &mut self.ship_rolls,
            &mut self.outfit_rolls,
            self.ships.iter().map(|ship| {
                (
                    ship.id,
                    hire::hire_roll(ship.hire_random),
                    shipyard::buy_roll(ship.buy_random, buy_random),
                )
            }),
            self.outfits.iter().map(|record| {
                (
                    record.id,
                    outfitter::buy_roll(record.buy_random, buy_random),
                )
            }),
            chance,
        );
        self.ship_redraws.clear();
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::catalog::{DisasterId, OutfitId, ShipId, StellarId, SystemId};
    use crate::chance::NeverFires;
    use crate::control::{Script, ScriptNote, SetExpr, Test};
    use crate::date::GameDate;
    use crate::market::ActiveEvent;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::session::entry::NoTraffic;
    use crate::session::tests::{NEW, buy, outfitted, sell, shipbuying, surplus};
    use crate::session::{SessionEvent, Settled};
    use crate::story::{
        Consumer, DuplicateSubscription, Entrance, EventFilter, StoryEvent, StoryEventKind,
        Subscription, SubscriptionId,
    };
    use crate::testkit::{self, FakePilotCatalog, Scripted, jump, land_now};

    const SPEED: OutfitId = OutfitId(300);

    /// The date `days` days after `date`.
    fn after(date: GameDate, days: usize) -> GameDate {
        std::iter::successors(Some(date), |date| Some(date.next_day()))
            .nth(days)
            .expect("a date")
    }

    /// Takes off from `session`, landed, and gives how many days passed.
    fn days_at_take_off(session: &mut Session) -> usize {
        let landed = session.date();
        session
            .take_off(&NoTraffic, &mut NeverFires)
            .expect("takes off");
        (0..10)
            .find(|&days| after(landed, days) == session.date())
            .expect("fewer than ten days")
    }

    /// [`shipbuying`]'s session, landed at its outfitter and shipyard.
    fn shopping() -> Session {
        outfitted(&shipbuying())
    }

    #[test]
    fn a_take_off_passes_a_day_and_rolls_its_events() {
        let catalog = surplus();
        let mut session = Session::start(&catalog).expect("starts");
        jump(&mut session, &catalog, 131).expect("arrives");
        assert_eq!(land_now(&mut session), Ok(StellarId(140)));
        let landed = session.date();
        let mut chance = Scripted::default();
        session.take_off(&catalog, &mut chance).expect("takes off");
        assert_eq!(session.date(), after(landed, 1));
        assert_eq!((landed.day(), landed.month(), landed.year()), (24, 6, 1177));
        assert_eq!(chance.asked, [35], "one roll for the one day");
    }

    #[test]
    fn an_outfit_bought_or_sold_adds_a_day_at_take_off() {
        let mut session = shopping();
        assert_eq!(session.outfit(buy(SPEED), &mut NeverFires), Ok(()));
        assert_eq!(days_at_take_off(&mut session), 2, "bought");

        let mut session = shopping();
        session.transact(|pilot| {
            pilot.outfits.insert(SPEED, 1);
        });
        assert_eq!(session.outfit(sell(SPEED), &mut NeverFires), Ok(()));
        assert_eq!(days_at_take_off(&mut session), 2, "sold");

        let mut session = shopping();
        assert!(session.outfit(sell(SPEED), &mut NeverFires).is_err());
        assert_eq!(days_at_take_off(&mut session), 1, "refused");
    }

    #[test]
    fn a_cancelled_count_of_an_allowed_order_still_adds_the_day() {
        let mut session = shopping();
        assert_eq!(
            session.outfit_counted(buy(SPEED), 0, &mut NeverFires),
            Ok(0)
        );
        assert_eq!(session.pilot().owned(SPEED), 0, "nothing moved");
        assert_eq!(days_at_take_off(&mut session), 2, "allowed");

        let mut session = shopping();
        assert_eq!(
            session.outfit_counted(sell(SPEED), 0, &mut NeverFires),
            Ok(0)
        );
        assert_eq!(days_at_take_off(&mut session), 1, "refused");
    }

    #[test]
    fn a_ship_bought_adds_four_days_at_take_off() {
        let mut session = shopping();
        session
            .buy_ship(NEW, "Kestrel", &mut NeverFires)
            .expect("buys");
        assert_eq!(days_at_take_off(&mut session), 5);

        let mut session = shopping();
        assert!(
            session
                .buy_ship(ShipId(999), "Kestrel", &mut NeverFires)
                .is_err()
        );
        assert_eq!(days_at_take_off(&mut session), 1, "refused");
    }

    #[test]
    fn an_outfit_and_a_ship_add_five_days() {
        let mut session = shopping();
        assert_eq!(session.outfit(buy(SPEED), &mut NeverFires), Ok(()));
        session
            .buy_ship(NEW, "Kestrel", &mut NeverFires)
            .expect("buys");
        assert_eq!(days_at_take_off(&mut session), 6);
    }

    #[test]
    fn escorts_sold_or_upgraded_pass_half_their_count() {
        let mut session = shopping();
        session.stay.escorts_changed = 4;
        assert_eq!(days_at_take_off(&mut session), 3);
    }

    #[test]
    fn a_new_landing_forgets_the_last_stay() {
        let mut session = shopping();
        assert_eq!(session.outfit(buy(SPEED), &mut NeverFires), Ok(()));
        assert_eq!(days_at_take_off(&mut session), 2);
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        assert_eq!(days_at_take_off(&mut session), 1);
    }

    #[test]
    fn by_the_other_reading_a_stay_passes_no_day() {
        let rules = Rulebook::default().with_override(RuleKey::StayDays, RuleSource::Bible);
        assert_eq!(
            shopping().rules().source_for(RuleKey::StayDays),
            RuleSource::Engine,
            "by default"
        );
        let mut session = shopping().with_rules(rules);
        assert_eq!(
            session.rules().source_for(RuleKey::StayDays),
            RuleSource::Bible
        );
        assert_eq!(session.outfit(buy(SPEED), &mut NeverFires), Ok(()));
        session
            .buy_ship(NEW, "Kestrel", &mut NeverFires)
            .expect("buys");
        session.stay.escorts_changed = 4;
        assert_eq!(days_at_take_off(&mut session), 0);

        let catalog = surplus();
        let mut session = Session::start(&catalog).expect("starts").with_rules(rules);
        jump(&mut session, &catalog, 131).expect("arrives");
        land_now(&mut session).expect("lands");
        let mut chance = Scripted::default();
        session.take_off(&catalog, &mut chance).expect("takes off");
        assert_eq!(chance.asked, Vec::<u8>::new(), "no roll");
    }

    /// [`surplus`], with ship classes 130 (hire 50, buy 60) and 129
    /// (hire 30, buy 40) and outfit 301 (buy 70) on a chance.
    fn rolling_days() -> FakePilotCatalog {
        use crate::catalog::{OutfitRecord, ShipRecord};
        let mut catalog = surplus();
        catalog.ship_records.extend([
            ShipRecord {
                hire_random: 50,
                buy_random: 60,
                ..testkit::ship(130, testkit::FAST)
            },
            ShipRecord {
                hire_random: 30,
                buy_random: 40,
                ..testkit::ship(129, testkit::FAST)
            },
        ]);
        catalog.outfits.push(OutfitRecord {
            buy_random: 70,
            ..testkit::outfit(301, &[])
        });
        catalog
    }

    #[test]
    fn a_day_draws_the_shop_and_hire_rolls_in_the_originals_order() {
        let catalog = rolling_days();
        let mut session = Session::start(&catalog).expect("starts");
        jump(&mut session, &catalog, 131).expect("arrives");
        land_now(&mut session).expect("lands");
        let mut chance = Scripted::default();
        session.take_off(&catalog, &mut chance).expect("takes off");
        assert_eq!(
            chance.asked,
            [35, 30, 40, 50, 60, 70],
            "after the day's event, each ship's hire then buy, then the outfit"
        );
    }

    #[test]
    fn each_day_draws_the_day_rolls_again() {
        let catalog = rolling_days();
        let mut session = Session::start(&catalog).expect("starts");
        let mut chance = Scripted::default();
        session.pass_days(2, &mut chance);
        assert_eq!(
            chance.asked,
            [35, 30, 40, 50, 60, 70, 35, 30, 40, 50, 60, 70]
        );
    }

    #[test]
    fn by_the_other_reading_a_day_draws_no_roll() {
        let rules = Rulebook::default().with_override(RuleKey::DayRolls, RuleSource::Bible);
        let catalog = rolling_days();
        let mut session = Session::start(&catalog).expect("starts").with_rules(rules);
        let mut chance = Scripted::default();
        session.pass_days(2, &mut chance);
        assert_eq!(chance.asked, [35, 35], "the event's rolls alone");
    }

    #[test]
    fn the_date_saved_at_take_off_holds_its_days() {
        let mut session = shopping();
        let landed = session.date();
        session
            .buy_ship(NEW, "Kestrel", &mut NeverFires)
            .expect("buys");
        session
            .take_off(&NoTraffic, &mut NeverFires)
            .expect("takes off");
        assert!(session.take_save_due());
        let saved = crate::save::decode(&crate::save::encode(session.pilot())).expect("loads");
        assert_eq!(saved.date(), after(landed, 5));
    }

    /// [`surplus`]'s session, its event under way with 5 days left.
    fn eventful(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.pilot.events.insert(
            DisasterId(128),
            ActiveEvent {
                days: 5,
                stellar: None,
            },
        );
        session
    }

    #[test]
    fn set_date_runs_no_participant() {
        let catalog = surplus();
        let mut session = eventful(&catalog);
        let date = after(session.date(), 3);
        session.set_date(date);
        assert_eq!(session.date(), date);
        assert_eq!(
            session.pilot().events().collect::<Vec<_>>(),
            [(DisasterId(128), 5)]
        );
    }

    #[test]
    fn loading_a_save_passes_no_day() {
        let catalog = surplus();
        let session = eventful(&catalog);
        let saved = crate::save::decode(&crate::save::encode(session.pilot())).expect("loads");
        let flown = Session::fly(&catalog, saved).expect("flies");
        assert_eq!(flown.date(), session.date());
        assert_eq!(
            flown.pilot().events().collect::<Vec<_>>(),
            [(DisasterId(128), 5)]
        );
    }

    const LANDED_128: StoryEvent = StoryEvent::Landed {
        stellar: StellarId(128),
    };
    const TOOK_OFF_128: StoryEvent = StoryEvent::TookOff {
        stellar: StellarId(128),
    };

    #[test]
    fn raised_events_queue_in_order_until_a_settle_takes_them() {
        let catalog = testkit::catalog();
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(session.story_raised(), [EXPLORED_130], "the start's only");
        session.raise(LANDED_128);
        session.raise(TOOK_OFF_128);
        assert_eq!(
            session.story_raised(),
            [EXPLORED_130, LANDED_128, TOOK_OFF_128]
        );
        assert_eq!(session.take_events(), [], "never the view's outbox");
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert_eq!(session.story_raised(), [], "settled");
    }

    /// `session` with the events raised so far let go of, so a test sees
    /// only what follows.
    fn quiet(mut session: Session) -> Session {
        session.story.raised.clear();
        session
    }

    #[test]
    fn a_landing_raises_landed_once() {
        let catalog = testkit::catalog();
        let mut session = quiet(Session::start(&catalog).expect("starts"));
        session.land().expect("selects");
        assert_eq!(session.story_raised(), [], "the first L only selects");
        session.land().expect("lands");
        assert_eq!(session.story_raised(), [LANDED_128]);
    }

    #[test]
    fn a_refused_landing_or_a_press_over_a_gate_raises_nothing() {
        let empty = FakePilotCatalog {
            sites: Vec::new(),
            ..testkit::catalog()
        };
        let mut session = quiet(Session::start(&empty).expect("starts"));
        assert!(land_now(&mut session).is_err());
        assert_eq!(session.story_raised(), [], "refused");

        let catalog = crate::session::tests::gated();
        let session = Session::start(&catalog).expect("starts");
        let raised_at_start = session.story_raised();
        let session = crate::session::tests::at_gate(&catalog);
        assert_eq!(session.story_raised(), raised_at_start, "over a gate");
    }

    /// `DayPassed` for each of the `days` days after `from`, in order.
    fn days_after(from: GameDate, days: usize) -> Vec<StoryEvent> {
        (1..=days)
            .map(|n| StoryEvent::DayPassed {
                date: after(from, n),
            })
            .collect()
    }

    const EXPLORED_130: StoryEvent = StoryEvent::Explored {
        system: SystemId(130),
    };

    #[test]
    fn a_take_off_raises_its_days_then_explored_then_took_off() {
        let mut session = quiet(shopping());
        assert_eq!(session.outfit(buy(SPEED), &mut NeverFires), Ok(()));
        let landed = session.date();
        session
            .take_off(&NoTraffic, &mut NeverFires)
            .expect("takes off");
        let mut expected = days_after(landed, 2);
        expected.extend([EXPLORED_130, TOOK_OFF_128]);
        assert_eq!(session.story_raised(), expected);
    }

    #[test]
    fn by_the_other_stay_reading_a_take_off_raises_only_explored_and_took_off() {
        let rules = Rulebook::default().with_override(RuleKey::StayDays, RuleSource::Bible);
        let mut session = quiet(shopping().with_rules(rules));
        assert_eq!(session.outfit(buy(SPEED), &mut NeverFires), Ok(()));
        session
            .take_off(&NoTraffic, &mut NeverFires)
            .expect("takes off");
        assert_eq!(session.story_raised(), [EXPLORED_130, TOOK_OFF_128]);
    }

    #[test]
    fn a_take_off_unlanded_raises_nothing() {
        let catalog = testkit::catalog();
        let mut session = quiet(Session::start(&catalog).expect("starts"));
        assert_eq!(session.take_off(&catalog, &mut NeverFires), None);
        assert_eq!(session.story_raised(), []);
    }

    #[test]
    fn a_jump_raises_a_day_passed_for_each_of_its_days_dated_that_day() {
        let catalog = crate::session::tests::surplus();
        let mut session = quiet(Session::start(&catalog).expect("starts"));
        session.stats.jump_days = 3;
        let left = session.date();
        jump(&mut session, &catalog, 131).expect("arrives");
        let days: Vec<StoryEvent> = session
            .story_raised()
            .into_iter()
            .filter(|event| matches!(event, StoryEvent::DayPassed { .. }))
            .collect();
        assert_eq!(days, days_after(left, 3));
    }

    #[test]
    fn set_date_and_a_load_raise_no_day() {
        let catalog = surplus();
        let mut session = quiet(eventful(&catalog));
        session.set_date(after(session.date(), 3));
        assert_eq!(session.story_raised(), []);
        let saved = crate::save::decode(&crate::save::encode(session.pilot())).expect("loads");
        let flown = Session::fly(&catalog, saved).expect("flies");
        assert!(
            !flown
                .story_raised()
                .iter()
                .any(|event| matches!(event, StoryEvent::DayPassed { .. })),
            "{:?}",
            flown.story_raised()
        );
    }

    fn entered(system: i16, how: Entrance) -> StoryEvent {
        StoryEvent::EnteredSystem {
            system: SystemId(system),
            how,
        }
    }

    fn explored(system: i16) -> StoryEvent {
        StoryEvent::Explored {
            system: SystemId(system),
        }
    }

    #[test]
    fn a_jump_raises_the_arrivals_explored_then_its_days_then_entered_system() {
        let catalog = testkit::catalog();
        let mut session = quiet(Session::start(&catalog).expect("starts"));
        session.stats.jump_days = 2;
        let left = session.date();
        jump(&mut session, &catalog, 131).expect("arrives");
        let mut expected = vec![explored(131)];
        expected.extend(days_after(left, 2));
        expected.push(entered(131, Entrance::Jump));
        assert_eq!(session.story_raised(), expected);
    }

    #[test]
    fn a_new_pilot_and_a_load_each_raise_their_systems_explored_once() {
        let catalog = testkit::catalog();
        let session = Session::start(&catalog).expect("starts");
        assert_eq!(session.story_raised(), [EXPLORED_130], "a new pilot");
        let mut pilot = session.pilot().clone();
        pilot.system = SystemId(131);
        let saved = crate::save::decode(&crate::save::encode(&pilot)).expect("loads");
        let flown = Session::fly(&catalog, saved).expect("flies");
        assert_eq!(flown.story_raised(), [explored(131)], "a load");
    }

    #[test]
    fn a_jumps_explored_follows_the_events_raised_before_it() {
        let catalog = testkit::catalog();
        let mut session = quiet(Session::start(&catalog).expect("starts"));
        session.raise(LANDED_128);
        let left = session.date();
        jump(&mut session, &catalog, 131).expect("arrives");
        let mut expected = vec![LANDED_128, explored(131)];
        expected.extend(days_after(left, 1));
        expected.push(entered(131, Entrance::Jump));
        assert_eq!(session.story_raised(), expected);
    }

    #[test]
    fn a_gate_raises_entered_system_then_explored_its_days_first_like_a_jump() {
        use crate::gate::GateArrivalRule;
        use crate::session::tests::{at_gate, gated};
        let catalog = gated();
        let mut session = quiet(at_gate(&catalog));
        session
            .enter_hypergate(Some(SystemId(131)), &catalog, &mut NeverFires)
            .expect("enters");
        assert_eq!(
            session.story_raised(),
            [entered(131, Entrance::Gate), explored(131)],
            "by the engine"
        );

        let mut session = quiet(at_gate(&catalog).with_gate_arrival(GateArrivalRule::LikeJump));
        let left = session.date();
        session
            .enter_hypergate(Some(SystemId(131)), &catalog, &mut NeverFires)
            .expect("enters");
        let mut expected = days_after(left, 1);
        expected.extend([entered(131, Entrance::Gate), explored(131)]);
        assert_eq!(session.story_raised(), expected, "like a jump");
    }

    /// `session` after running `M131` and settling its move.
    fn moved(mut session: Session, catalog: &FakePilotCatalog) -> Vec<StoryEvent> {
        let expr = SetExpr::parse("M131").expect("parses");
        session.run_set(&expr, &mut NeverFires);
        let mut session = quiet(session);
        session.settle_moves(catalog, &mut NeverFires, &mut Settled::default());
        session.story_raised()
    }

    #[test]
    fn a_move_raises_entered_system_and_as_an_arrival_explored() {
        let catalog = testkit::catalog();
        for (reading, landed) in [
            (RuleSource::Engine, true),
            (RuleSource::Engine, false),
            (RuleSource::Bible, true),
            (RuleSource::Bible, false),
        ] {
            let rules = Rulebook::default().with_override(RuleKey::MoveArrival, reading);
            let mut session = Session::start(&catalog).expect("starts").with_rules(rules);
            if landed {
                land_now(&mut session).expect("lands");
            }
            let mut expected = vec![entered(131, Entrance::ScriptMove { landed })];
            if reading == RuleSource::Bible {
                expected.push(explored(131));
            }
            assert_eq!(moved(session, &catalog), expected, "{reading:?} {landed}");
        }
    }

    #[test]
    fn a_relocation_raises_entered_system_then_explored() {
        let catalog = testkit::catalog();
        let mut session = Session::start(&catalog).expect("starts");
        land_now(&mut session).expect("lands");
        let mut session = quiet(session);
        session
            .relocate(&catalog, SystemId(131), StellarId(140))
            .expect("moves");
        assert_eq!(
            session.story_raised(),
            [entered(131, Entrance::Relocate), explored(131)]
        );
    }

    #[test]
    fn a_multi_jump_raises_the_passed_systems_explored_before_its_days() {
        use crate::hyperspace::MultiJumpRule;
        use crate::session::tests::{bound_for_134, chained};
        for (rule, mod_val, days) in [(MultiJumpRule::Engine, 2, 1), (MultiJumpRule::PerHop, 1, 2)]
        {
            let catalog = chained(testkit::catalog(), mod_val);
            let mut session = bound_for_134(&catalog, rule);
            testkit::begin_jump_now(&mut session).expect("jumps");
            let mut session = quiet(session);
            let left = session.date();
            assert_eq!(
                session.arrive(&catalog, &mut NeverFires),
                Some(SystemId(132))
            );
            let mut expected = vec![explored(131), explored(132)];
            expected.extend(days_after(left, days));
            expected.push(entered(132, Entrance::Jump));
            assert_eq!(session.story_raised(), expected, "{rule:?}");
        }
    }

    #[test]
    fn a_jump_off_its_course_raises_the_system_it_reaches() {
        use crate::hyperspace::MultiJumpRule;
        use crate::session::tests::{bound_for_134, chained};
        let catalog = chained(testkit::catalog(), 10);
        let mut session = bound_for_134(&catalog, MultiJumpRule::Engine);
        testkit::begin_jump_now(&mut session).expect("jumps");
        session.plot_course(SystemId(133)).expect("a route");
        let mut session = quiet(session);
        let left = session.date();
        session.arrive(&catalog, &mut NeverFires).expect("arrives");
        let mut expected = vec![explored(131)];
        expected.extend(days_after(left, 1));
        expected.push(entered(131, Entrance::Jump));
        assert_eq!(session.story_raised(), expected);
    }

    // The dispatch, through fixture subscriptions.

    fn fixture(record: i16) -> SubscriptionId {
        SubscriptionId {
            consumer: Consumer::Fixture,
            record,
            hook: 0,
        }
    }

    /// `record`'s fixture subscription to `on`, running `script` behind
    /// `guard`.
    fn sub(record: i16, on: EventFilter, guard: &str, script: &str) -> Subscription {
        Subscription {
            guard: Test::parse(guard),
            ..Subscription::new(fixture(record), on, Script::parse(script))
        }
    }

    fn register(session: &mut Session, subs: impl IntoIterator<Item = Subscription>) {
        for sub in subs {
            session.register_story(sub).expect("registers");
        }
    }

    fn bits(session: &Session) -> Vec<u16> {
        session
            .pilot()
            .bits
            .iter()
            .map(crate::control::Bit::get)
            .collect()
    }

    /// Lands `session` and settles it.
    fn land_and_settle(session: &mut Session, catalog: &FakePilotCatalog) {
        land_now(session).expect("lands");
        let _ = session.settle_script(catalog, &mut NeverFires);
    }

    const ANY_LANDING: EventFilter = EventFilter::Any(StoryEventKind::Landed);

    #[test]
    fn a_subscription_runs_its_script_when_its_event_fires_and_its_guard_holds() {
        let catalog = testkit::catalog();
        let mut session = Session::start(&catalog).expect("starts");
        register(
            &mut session,
            [
                sub(1, EventFilter::Landed(StellarId(128)), "b9", "b4"),
                sub(2, EventFilter::Landed(StellarId(129)), "", "b5"),
            ],
        );
        land_and_settle(&mut session, &catalog);
        assert_eq!(bits(&session), [0_u16; 0], "b9 clear");
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        session.set_control_bit(crate::control::Bit::new(9).expect("a bit"), true);
        land_and_settle(&mut session, &catalog);
        assert_eq!(
            bits(&session),
            [4, 9],
            "b9 set, and not the other stellar's"
        );
    }

    #[test]
    fn registering_an_id_twice_is_refused() {
        let catalog = testkit::catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session
            .register_story(sub(1, ANY_LANDING, "", "b1"))
            .expect("registers");
        assert_eq!(
            session.register_story(sub(1, ANY_LANDING, "", "b2")),
            Err(DuplicateSubscription(fixture(1)))
        );
        land_and_settle(&mut session, &catalog);
        assert_eq!(bits(&session), [1], "the first kept");
    }

    #[test]
    fn a_once_subscription_fires_once_and_stays_fired_across_a_save_and_reload() {
        let catalog = testkit::catalog();
        let subs = || {
            [
                Subscription {
                    once: true,
                    ..sub(1, ANY_LANDING, "", "^b5")
                },
                sub(2, ANY_LANDING, "", "^b6"),
            ]
        };
        let mut session = Session::start(&catalog).expect("starts");
        register(&mut session, subs());
        land_and_settle(&mut session, &catalog);
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        land_and_settle(&mut session, &catalog);
        assert_eq!(bits(&session), [5], "once, and twice toggled");
        assert!(session.take_save_due());

        let saved = crate::save::decode(&crate::save::encode(session.pilot())).expect("loads");
        let mut session = Session::fly(&catalog, saved).expect("flies");
        // The `Landed` a pilot resumed docked raises settles before the
        // fixture registers again.
        let _ = session.settle_script(&catalog, &mut NeverFires);
        register(&mut session, subs());
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        land_and_settle(&mut session, &catalog);
        assert_eq!(bits(&session), [5, 6], "still fired after the reload");

        assert!(session.unregister_story(fixture(1)));
        assert!(!session.unregister_story(fixture(1)), "gone");
        assert!(session.take_save_due(), "its once-fired entry cleared");
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        land_and_settle(&mut session, &catalog);
        assert_eq!(bits(&session), [5], "unregistered, it runs no more");
        register(&mut session, [subs()[0].clone()]);
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        land_and_settle(&mut session, &catalog);
        assert_eq!(bits(&session), [6], "registered afresh, it runs again");
    }

    #[test]
    fn marking_a_once_only_subscription_fired_makes_a_save_due() {
        let catalog = testkit::catalog();
        let mut session = Session::start(&catalog).expect("starts");
        register(
            &mut session,
            [Subscription {
                once: true,
                ..sub(1, EventFilter::Any(StoryEventKind::Explored), "", "")
            }],
        );
        assert!(!session.take_save_due(), "nothing else makes one due");
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert!(session.pilot().story_fired.contains(&fixture(1)));
        assert!(session.take_save_due(), "the start's Explored fired it");
    }

    #[test]
    fn unregistering_an_unfired_subscription_makes_no_save_due() {
        let catalog = testkit::catalog();
        let mut session = Session::start(&catalog).expect("starts");
        register(&mut session, [sub(1, ANY_LANDING, "", "b1")]);
        session.take_save_due();
        assert!(session.unregister_story(fixture(1)));
        assert!(!session.take_save_due());
    }

    /// A on landing: `b1 M131 b2`; B on entering 131, while `b1 & !b2`:
    /// `b3`; C on landing, after A, while `!b3`: `b4`; under `reading` of
    /// `StoryReentry`.
    fn reentrant(catalog: &FakePilotCatalog, reading: Option<RuleSource>) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        if let Some(reading) = reading {
            session = session
                .with_rules(Rulebook::default().with_override(RuleKey::StoryReentry, reading));
        }
        register(
            &mut session,
            [
                sub(1, ANY_LANDING, "", "b1 M131 b2"),
                sub(
                    2,
                    EventFilter::EnteredSystem(SystemId(131)),
                    "b1 & !b2",
                    "b3",
                ),
                sub(3, ANY_LANDING, "!b3", "b4"),
            ],
        );
        session
    }

    #[test]
    fn by_the_engine_an_event_a_script_raises_runs_before_the_scripts_next_operator() {
        let catalog = testkit::catalog();
        for reading in [None, Some(RuleSource::Engine)] {
            let mut session = reentrant(&catalog, reading);
            land_and_settle(&mut session, &catalog);
            assert_eq!(bits(&session), [1, 2, 3], "{reading:?}");
            assert_eq!(session.system(), SystemId(131));
        }
    }

    #[test]
    fn by_the_other_reading_an_event_a_script_raises_waits_for_the_script() {
        let catalog = testkit::catalog();
        let mut session = reentrant(&catalog, Some(RuleSource::Bible));
        land_and_settle(&mut session, &catalog);
        assert_eq!(bits(&session), [1, 2, 4]);
        assert_eq!(session.system(), SystemId(131));
    }

    /// Counts the `M` operators handed on.
    #[derive(Debug, Default)]
    struct Moves(std::cell::Cell<usize>);

    impl crate::session::control::SetOpObserver for Moves {
        fn saw(&self, op: &crate::control::SetOp, _session: &Session) {
            if matches!(op, crate::control::SetOp::MoveTo(_)) {
                self.0.set(self.0.get() + 1);
            }
        }
    }

    #[test]
    fn a_settling_dispatches_at_most_64_events_and_tells_the_rest_dropped() {
        let catalog = testkit::catalog();
        for reading in [RuleSource::Engine, RuleSource::Bible] {
            let moves = std::rc::Rc::new(Moves::default());
            let mut session = Session::start(&catalog)
                .expect("starts")
                .with_rules(Rulebook::default().with_override(RuleKey::StoryReentry, reading))
                .with_set_op_observer(moves.clone());
            register(
                &mut session,
                [sub(
                    1,
                    EventFilter::Any(StoryEventKind::EnteredSystem),
                    "",
                    "M131",
                )],
            );
            let m131 = SetExpr::parse("M131").expect("parses");
            for round in 0..2 {
                session.run_set(&m131, &mut NeverFires);
                let settled = session.settle_script(&catalog, &mut NeverFires);
                assert_eq!(settled.moved, Some(SystemId(131)), "{reading:?} {round}");
                assert_eq!(moves.0.take(), 65, "{reading:?} {round}");
                assert_eq!(
                    session.take_script_notes(),
                    [ScriptNote::EventsDropped { dropped: 1 }],
                    "{reading:?} {round}: a fresh budget each settling"
                );
            }
        }
    }

    #[test]
    fn a_hooks_move_dispatches_its_events_at_the_next_settling() {
        let catalog = testkit::catalog();
        let mut session = Session::start(&catalog).expect("starts");
        register(
            &mut session,
            [sub(1, EventFilter::EnteredSystem(SystemId(131)), "", "b3")],
        );
        session.run_set(&SetExpr::parse("M131").expect("parses"), &mut NeverFires);
        assert_eq!(bits(&session), [0_u16; 0], "deferred");
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert_eq!(bits(&session), [3]);
    }

    // The writer's catalog-free entry.

    fn left(days_left: u16) -> Countdown {
        Countdown { days_left }
    }

    #[test]
    fn setting_a_stellars_state_tells_the_galaxy_changed_only_as_it_is_destroyed_or_regenerated() {
        let catalog = testkit::catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.take_events();
        session.take_save_due();
        let s = StellarId(128);
        assert!(!session.set_stellar_state(s, None), "whole already");
        assert_eq!(session.take_events(), []);
        assert!(!session.take_save_due());
        assert!(session.set_stellar_state(s, Some(left(3))));
        assert_eq!(session.take_events(), [SessionEvent::GalaxyChanged]);
        assert!(session.take_save_due());
        assert_eq!(session.pilot().galaxy.stellar(s).destroyed, Some(left(3)));
        assert_eq!(*session.galaxy().overlay(), session.pilot().galaxy);
        assert!(session.set_stellar_state(s, Some(left(2))), "counted down");
        assert_eq!(session.take_events(), [], "still destroyed");
        assert!(session.take_save_due());
        assert!(!session.set_stellar_state(s, Some(left(2))));
        assert!(!session.take_save_due());
        assert!(session.set_stellar_state(s, None));
        assert_eq!(session.take_events(), [SessionEvent::GalaxyChanged]);
        assert!(session.take_save_due());
        assert!(session.galaxy().overlay().is_empty());
    }

    #[test]
    fn setting_a_stellars_state_stamps_the_stellars_shown_and_those_waiting() {
        let catalog = testkit::catalog();
        let mut session = Session::start(&catalog).expect("starts");
        land_now(&mut session).expect("lands");
        session.run_set(&SetExpr::parse("M131").expect("parses"), &mut NeverFires);
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert!(session.next_sites.is_some(), "131's wait for the take-off");
        let reads = session_reads(&catalog);
        session.set_stellar_state(StellarId(140), Some(left(1)));
        session.set_stellar_state(StellarId(129), Some(left(1)));
        session.assert_docked_among_shown();
        let destroyed = |sites: &[LandingSite]| -> Vec<(i16, bool)> {
            sites
                .iter()
                .map(|site| (site.id.0, site.destroyed))
                .collect()
        };
        assert_eq!(destroyed(&session.sites), [(128, false), (129, true)]);
        assert_eq!(
            destroyed(session.next_sites.as_deref().expect("waiting")),
            [(140, true)]
        );
        assert_eq!(session_reads(&catalog), reads, "nothing read again");
        session.set_stellar_state(StellarId(129), None);
        assert_eq!(destroyed(&session.sites), [(128, false), (129, false)]);
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        assert_eq!(destroyed(&session.sites), [(140, true)]);
    }

    /// How many times `catalog`'s stellars were read.
    fn session_reads(catalog: &FakePilotCatalog) -> usize {
        catalog.sites_asked.borrow().len()
    }

    #[test]
    fn a_change_to_the_galaxy_leaving_the_stellars_shown_as_they_were_reads_none_again() {
        let catalog = testkit::catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.set_stellar_state(StellarId(128), Some(left(3)));
        let reads = session_reads(&catalog);
        assert!(session.change_galaxy(&catalog, |overlay| {
            overlay.set_destroyed(StellarId(128), Some(left(2)));
            overlay.set_destroyed(StellarId(300), Some(left(2)));
        }));
        assert_eq!(session_reads(&catalog), reads, "a countdown, and another's");
        assert!(session.sites[0].destroyed);
    }

    #[test]
    fn a_change_to_the_galaxy_that_destroys_a_stellar_shown_reads_it_again() {
        let catalog = testkit::catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.take_events();
        assert!(session.change_galaxy(&catalog, |overlay| {
            overlay.set_destroyed(StellarId(128), Some(left(3)));
        }));
        assert!(session.sites[0].destroyed);
        assert_eq!(session.take_events(), [SessionEvent::GalaxyChanged]);
        assert!(session.change_galaxy(&catalog, |overlay| {
            overlay.set_destroyed(StellarId(128), None);
        }));
        assert!(!session.sites[0].destroyed);
    }

    #[test]
    fn with_nothing_registered_a_settling_is_as_before_and_tells_nothing() {
        let catalog = testkit::catalog();
        let mut session = Session::start(&catalog).expect("starts");
        land_now(&mut session).expect("lands");
        session.run_set(&SetExpr::parse("M131").expect("parses"), &mut NeverFires);
        session.raise(TOOK_OFF_128);
        assert_eq!(
            session.settle_script(&catalog, &mut NeverFires),
            Settled {
                moved: Some(SystemId(131)),
                took_off: None,
                message: None,
            }
        );
        assert_eq!(session.take_script_notes(), []);
        assert_eq!(session.story_raised(), []);
    }

    // Letting days pass, as a walker does.

    /// Fixture subscriptions on `DayPassed` that set b1, b2 and so on, a
    /// bit a day: the last bit's first, so each day climbs one rung.
    fn day_ladder(rungs: i16) -> Vec<Subscription> {
        (1..=rungs)
            .map(|n| {
                let guard = if n == 1 {
                    "!b1".to_owned()
                } else {
                    format!("b{} & !b{n}", n - 1)
                };
                sub(
                    rungs + 1 - n,
                    EventFilter::Any(StoryEventKind::DayPassed),
                    &guard,
                    &format!("b{n}"),
                )
            })
            .collect()
    }

    #[test]
    fn letting_days_pass_moves_the_date_on_and_raises_a_day_passed_for_each() {
        let catalog = testkit::catalog();
        let mut session = quiet(Session::start(&catalog).expect("starts"));
        register(&mut session, day_ladder(4));
        let from = session.date();
        session.let_days_pass(3, &mut NeverFires);
        assert_eq!(session.date(), after(from, 3));
        assert_eq!(session.story_raised(), days_after(from, 3));
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert_eq!(bits(&session), [1, 2, 3], "three days dispatched");
    }

    #[test]
    fn letting_days_pass_steps_the_events_and_pays_no_wage() {
        let catalog = surplus();
        let mut session = eventful(&catalog);
        session.pilot.escorts.push(crate::pilot::Escort {
            ship: ShipId(128),
            reserves: crate::reserves::Reserves::full(30.0, 45.0, 300.0),
            order: None,
            carried: false,
            wage: Some(100),
            person: None,
        });
        session.set_credits(1000);
        session.let_days_pass(2, &mut NeverFires);
        assert_eq!(
            session.pilot().events().collect::<Vec<_>>(),
            [(DisasterId(128), 3)],
            "the event stepped each day"
        );
        assert_eq!(session.pilot().cash(), 1000, "no wage paid");
        assert_eq!(session.take_pay_notes(), []);
    }

    /// [`testkit::catalog`] with mission 140 offered at 128, going
    /// nowhere, timed two days, its `OnFailure` setting b40.
    fn timed() -> FakePilotCatalog {
        FakePilotCatalog {
            missions: vec![crate::catalog::MissionRecord {
                avail_stel: 128,
                time_limit: 2,
                on_failure: Script::parse("b40"),
                ..testkit::mission(140)
            }],
            ..testkit::catalog()
        }
    }

    /// A session of `catalog` docked at 128, holding mission 140.
    fn holding_timed(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        land_now(&mut session).expect("lands");
        let _ = session.settle_script(catalog, &mut NeverFires);
        session
            .accept_mission(crate::catalog::MissionId(140), &mut NeverFires)
            .expect("accepted");
        session
    }

    #[test]
    fn days_let_pass_in_flight_fail_a_mission_past_its_deadline() {
        let catalog = timed();
        let mut session = holding_timed(&catalog);
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert_eq!(session.pilot().missions().missions().len(), 1, "held");
        session.let_days_pass(3, &mut NeverFires);
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert!(
            session
                .pilot()
                .control_bit(crate::control::Bit::new(40).expect("a bit"))
        );
        assert_eq!(session.pilot().missions().missions(), [], "freed");
    }

    #[test]
    fn days_let_pass_docked_leave_a_mission_past_its_deadline_unfailed() {
        let catalog = timed();
        let mut session = holding_timed(&catalog);
        session.let_days_pass(3, &mut NeverFires);
        let _ = session.settle_script(&catalog, &mut NeverFires);
        let active = session.pilot().missions().get(0).expect("held");
        assert_eq!((active.days_left, active.failed), (Some(0), None));
        assert!(
            !session
                .pilot()
                .control_bit(crate::control::Bit::new(40).expect("a bit"))
        );
    }

    // The galaxy refresh.

    /// `system`'s `Visibility`, `test`.
    fn visibility(system: i16, test: &str) -> crate::catalog::SystemVisibility {
        crate::catalog::SystemVisibility {
            id: SystemId(system),
            visibility: Test::parse(test),
        }
    }

    /// [`testkit::catalog`], 131 also linked back to 130, with these
    /// `Visibility` tests.
    fn gated(tests: &[(i16, &str)]) -> FakePilotCatalog {
        let mut catalog = testkit::catalog();
        catalog.star_map[1].links.push(SystemId(130));
        catalog.visibility = tests
            .iter()
            .map(|&(system, test)| visibility(system, test))
            .collect();
        catalog
    }

    /// [`gated`], with 134 at 131's place, (600, 0), listing 130 and
    /// holding planet 141.
    fn gated_alternates(tests: &[(i16, &str)]) -> FakePilotCatalog {
        let mut catalog = gated(tests);
        catalog
            .star_map
            .push(testkit::star(134, (600.0, 0.0), &[130]));
        catalog.systems.push(SystemId(134));
        catalog
            .sites
            .push((SystemId(134), vec![testkit::planet(141, 0.0, 0.0)]));
        catalog
    }

    fn visible(session: &Session, system: i16) -> bool {
        session.galaxy().visible(SystemId(system))
    }

    /// Puts the ship at rest over 140, at 131's centre, and lands there.
    fn land_on_140(session: &mut Session) -> Result<StellarId, crate::landing::LandingRefusal> {
        session.player.position = crate::geometry::Vec2::ZERO;
        session.player.velocity = crate::geometry::Vec2::ZERO;
        land_now(session)
    }

    /// Runs `script` and settles it.
    fn set(session: &mut Session, catalog: &FakePilotCatalog, script: &str) {
        session.run_set(&SetExpr::parse(script).expect("parses"), &mut NeverFires);
        let _ = session.settle_script(catalog, &mut NeverFires);
    }

    #[test]
    fn a_new_pilot_flies_with_its_systems_visibility_evaluated() {
        let catalog = gated(&[(132, "b50"), (131, "!b50")]);
        let session = Session::start(&catalog).expect("starts");
        assert!(!visible(&session, 132));
        assert!(visible(&session, 131));
        assert_eq!(
            session.pilot().galaxy.system(SystemId(132)).visible,
            Some(false)
        );
    }

    #[test]
    fn by_the_engine_a_bit_a_settled_script_sets_shows_only_at_the_next_refresh() {
        let catalog = gated(&[(132, "b50")]);
        let mut session = Session::start(&catalog).expect("starts");
        session.take_events();
        set(&mut session, &catalog, "b50");
        assert!(!visible(&session, 132), "not yet");
        assert_eq!(session.take_events(), []);
        land_now(&mut session).expect("lands");
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert!(visible(&session, 132), "at the landing");
        assert!(session.take_events().contains(&SessionEvent::GalaxyChanged));
    }

    #[test]
    fn a_jumps_arrival_refreshes_before_its_days_so_a_crons_bit_waits_for_the_landing() {
        let mut catalog = gated(&[(132, "b50")]);
        catalog.crons = vec![crate::catalog::CronRecord {
            on_start: Script::parse("b50"),
            ..testkit::cron(150)
        }];
        let mut session = Session::start(&catalog).expect("starts");
        jump(&mut session, &catalog, 131).expect("arrives");
        assert!(
            session.control_bit(crate::control::Bit::new(50).expect("a bit")),
            "set on the jump's day"
        );
        assert!(!visible(&session, 132), "refreshed before the days");
        assert_eq!(land_on_140(&mut session), Ok(StellarId(140)));
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert!(visible(&session, 132), "at the next landing");
    }

    #[test]
    fn a_jumps_arrival_shows_a_bit_set_before_it_and_resolves_by_the_flags_as_they_stood() {
        let catalog = gated_alternates(&[(131, "!b50"), (134, "b50")]);
        let mut session = quiet(Session::start(&catalog).expect("starts"));
        assert!(visible(&session, 131) && !visible(&session, 134));
        set(&mut session, &catalog, "b50");
        session.take_events();
        assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(134)));
        assert!(!visible(&session, 131) && visible(&session, 134));
        assert_eq!(
            session.story_raised().first(),
            Some(&explored(131)),
            "explored as 131 stood before the refresh"
        );
        assert!(session.take_events().contains(&SessionEvent::GalaxyChanged));
    }

    #[test]
    fn a_new_pilots_on_start_shows_its_bits_after_begin() {
        let mut catalog = gated(&[(132, "b50")]);
        if let Ok(start) = &mut catalog.character {
            start.on_start = Script::parse("b50");
        }
        let mut session = Session::start(&catalog).expect("starts");
        assert!(
            !visible(&session, 132),
            "the flight's refresh, before OnStart"
        );
        session.begin(&catalog, &mut NeverFires);
        assert!(visible(&session, 132), "_PlayGame's refresh, after it");
    }

    #[test]
    fn a_load_recomputes_the_saved_visibility_quietly() {
        let catalog = gated(&[(132, "b50"), (133, "!b50")]);
        let mut pilot = crate::pilot::Pilot::new(&catalog, "Ada").expect("starts");
        pilot
            .bits_mut()
            .set(crate::control::Bit::new(50).expect("a bit"));
        pilot.galaxy.set_visible(SystemId(132), Some(false));
        pilot.galaxy.set_visible(SystemId(133), Some(true));
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        assert!(visible(&session, 132), "saved hidden, its test holds");
        assert!(!visible(&session, 133), "saved shown, its test fails");
        assert!(!session.take_events().contains(&SessionEvent::GalaxyChanged));
        assert!(!session.take_save_due(), "nothing to save");
    }

    #[test]
    fn a_load_whose_refresh_hides_the_players_system_moves_it_to_the_one_at_its_place() {
        let catalog = gated_alternates(&[(131, "b50")]);
        let mut pilot = crate::pilot::Pilot::new(&catalog, "Ada").expect("starts");
        pilot.system = SystemId(131);
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(session.system(), SystemId(134));
        assert_eq!(session.sites[0].id, StellarId(141), "134's stellars");
        assert!(!session.take_save_due());
    }

    #[test]
    fn a_chain_head_the_move_shows_owns_nothing_and_hides_again_at_the_next_refresh() {
        let catalog = gated(&[(131, "!b50")]);
        let mut session = Session::start(&catalog).expect("starts");
        jump(&mut session, &catalog, 131).expect("arrives");
        let owner = |session: &Session| session.galaxy().owner(&[SystemId(131)]);
        assert_eq!(owner(&session), Some(SystemId(131)));
        set(&mut session, &catalog, "b50");
        assert_eq!(land_on_140(&mut session), Ok(StellarId(140)));
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert_eq!(session.system(), SystemId(131), "alone at its place");
        assert!(visible(&session, 131));
        assert_eq!(
            session.pilot().galaxy.system(SystemId(131)).visible,
            Some(true),
            "shown by the move"
        );
        assert_eq!(owner(&session), None, "it owns no stellar");
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert_eq!(land_on_140(&mut session), Ok(StellarId(140)));
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert_eq!(
            session.pilot().galaxy.system(SystemId(131)).visible,
            Some(true),
            "hidden by the refresh and shown again by the move"
        );
        assert_eq!(owner(&session), None, "still owning nothing");
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        let _ = session.settle_script(&catalog, &mut NeverFires);
        jump(&mut session, &catalog, 130).expect("arrives");
        assert!(!visible(&session, 131), "the arrival's refresh hides it");
        assert_eq!(
            session.pilot().galaxy.system(SystemId(131)).visible,
            Some(false)
        );
    }

    /// Nova's control bits, counting the tests asked.
    #[derive(Debug, Default)]
    struct Counting(std::cell::Cell<usize>);

    impl crate::control::ControlBits for Counting {
        fn allows(
            &self,
            test: &crate::control::TestExpr,
            pilot: &dyn crate::control::PilotFacts,
        ) -> bool {
            self.0.set(self.0.get() + 1);
            crate::control::holds(test, pilot)
        }
    }

    /// A session of `catalog` with `visibility_refresh` reading `source`,
    /// its control bits counting the tests asked.
    fn refreshing(catalog: &FakePilotCatalog, source: RuleSource) -> (Session, Rc<Counting>) {
        let counting = Rc::new(Counting::default());
        let session = Session::start(catalog)
            .expect("starts")
            .with_rules(Rulebook::default().with_override(RuleKey::VisibilityRefresh, source))
            .with_control_bits(counting.clone());
        (session, counting)
    }

    #[test]
    fn by_the_other_refresh_reading_a_bit_shows_once_its_script_settles() {
        let catalog = gated(&[(132, "b50")]);
        let (mut session, _) = refreshing(&catalog, RuleSource::Bible);
        session.take_events();
        set(&mut session, &catalog, "b50");
        assert!(visible(&session, 132), "right after settling");
        assert_eq!(session.take_events(), [SessionEvent::GalaxyChanged]);
        set(&mut session, &catalog, "!b50");
        assert!(!visible(&session, 132));
    }

    #[test]
    fn by_the_engine_refresh_reading_a_bit_waits_for_the_next_refresh_point() {
        let catalog = gated(&[(132, "b50")]);
        let (mut session, _) = refreshing(&catalog, RuleSource::Engine);
        set(&mut session, &catalog, "b50");
        assert!(!visible(&session, 132), "not after settling");
        land_now(&mut session).expect("lands");
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert!(visible(&session, 132), "at the landing");
    }

    #[test]
    fn by_the_other_refresh_reading_an_idle_settling_evaluates_nothing() {
        let catalog = gated(&[(132, "b50"), (133, "!b50")]);
        let (mut session, counting) = refreshing(&catalog, RuleSource::Bible);
        let _ = session.settle_script(&catalog, &mut NeverFires);
        let asked = counting.0.get();
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert_eq!(counting.0.get(), asked, "no script ran");
        set(&mut session, &catalog, "b1");
        assert_eq!(counting.0.get(), asked + 2, "both tests, once");
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert_eq!(counting.0.get(), asked + 2, "and not again");
    }
}
