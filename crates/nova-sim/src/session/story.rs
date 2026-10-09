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
use super::script_effects::Settled;
use super::{ScriptNote, Session, SessionEvent};
use crate::catalog::{LandingSite, PilotCatalog, StellarId, SystemId, TrafficCatalog};
use crate::chance::Chance;
use crate::control::{Script, Test};
use crate::market;
use crate::pilot::Pilot;
use crate::rulebook::RuleKey;
use crate::story::day::{self, Day, Participant};
use crate::story::dispatch::{self, DuplicateSubscription, Reentry, Registry, Stage};
use crate::story::{
    Consumer, Countdown, Fate, GalaxyOverlay, HiddenMove, OverlaidGalaxy, StoryEvent, Subscription,
    SubscriptionId,
};
use crate::traffic::npc::NpcId;

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
    fn effect(&mut self, id: SubscriptionId, _event: &StoryEvent) {
        match id.consumer {
            // Each consumer's arm is filled by its own phase: mission
            // ships' goal counts by mission-ships phase 2, a mission's pay,
            // legal record and days by missions phase 2, and the crons',
            // nebulae's and plug-ins' by theirs. A fixture has none.
            Consumer::MissionShips
            | Consumer::Missions
            | Consumer::Crons
            | Consumer::Nebulae
            | Consumer::Plugins
            | Consumer::Fixture => {}
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
        self.session.settle_leave(&mut *self.chance, self.settled)
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
            Participant::DayPassed => session.raise(StoryEvent::DayPassed {
                date: session.pilot.date,
            }),
            // Each slot is filled by its own phase: the crons by
            // crons-news-ranks phase 1, the missions' days left by missions
            // phase 2, the tribute by rdm task planet-domination, the
            // reinforcements once they are built, and the ranks by
            // crons-news-ranks phase 3. The day rolls are still drawn
            // afresh at each landing.
            Participant::Crons
            | Participant::MissionDays
            | Participant::Tribute
            | Participant::Reinforcements
            | Participant::Ranks
            | Participant::DayRolls => {}
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
    /// finds it in the system raises its first sighting,
    /// [`Fate::Observed`], once, and forgets it. A ship no longer in the
    /// system is forgotten, and populating the system or resetting its
    /// traffic forgets every one, its ships replaced (see the `story`
    /// module's findings).
    pub fn watch_ship(&mut self, ship: NpcId) {
        self.story.watched.insert(ship);
    }

    /// Raises the first sighting of each watched ship in the system, and
    /// forgets every watched ship: seen, or no longer here. Every ship in
    /// the system is visible, as the engine has no cloak and no ship
    /// emerging from a gate (`_ShipVisibleToShip` @0x971c).
    pub(super) fn observe_watched(&mut self) {
        for ship in std::mem::take(&mut self.story.watched) {
            if self.traffic.npcs().iter().any(|npc| npc.id == ship) {
                self.raise(StoryEvent::ShipFate {
                    ship,
                    fate: Fate::Observed,
                });
            }
        }
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
    pub fn change_galaxy<C>(&mut self, catalog: &C, change: impl FnOnce(&mut GalaxyOverlay)) -> bool
    where
        C: PilotCatalog + TrafficCatalog + ?Sized,
    {
        let govt = self.galaxy.govt(self.pilot.system);
        let Some(before) = self.write_galaxy(change) else {
            return false;
        };
        self.refresh_sites(catalog, &before);
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
        true
    }

    /// The writer's one core, for both its entries: changes the pilot's
    /// overlay as `change` says and, when that changed anything, the
    /// galaxy's, rebuilding the overlaid map, and makes a save due. It
    /// tells nothing: each entry tells what its change calls for. The
    /// overlay as it was, when it changed.
    fn write_galaxy(&mut self, change: impl FnOnce(&mut GalaxyOverlay)) -> Option<GalaxyOverlay> {
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

    /// Stamps the stellars shown and those waiting for the take-off afresh
    /// through the galaxy ([`OverlaidGalaxy::site`]): only whether each is
    /// destroyed can have changed, which the overlay alone gives, so
    /// nothing is read again. Which stellars are shown is
    /// [`Session::set_player_system`]'s alone; this only stamps them.
    fn stamp_sites(&mut self) {
        let galaxy = &self.galaxy;
        for site in self
            .sites
            .iter_mut()
            .chain(self.next_sites.iter_mut().flatten())
        {
            *site = galaxy.site(*site);
        }
    }

    /// Reads the stellars again from `catalog` where the galaxy's change
    /// from `before` gave one of them another government, or destroyed or
    /// regenerated it: those shown, and those waiting for the take-off.
    /// Which stellars are shown is [`Session::set_player_system`]'s alone;
    /// this only refreshes them.
    fn refresh_sites(&mut self, catalog: &(impl PilotCatalog + ?Sized), before: &GalaxyOverlay) {
        let now = &self.pilot.galaxy;
        let changed = |sites: &[LandingSite]| {
            sites.iter().any(|site| {
                let (was, is) = (before.stellar(site.id), now.stellar(site.id));
                was.govt != is.govt || was.destroyed.is_some() != is.destroyed.is_some()
            })
        };
        let shown_stale = changed(&self.sites);
        let waiting_stale = self.next_sites.as_deref().is_some_and(changed);
        if shown_stale {
            self.sites = self.read_sites(catalog, self.shown);
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{DisasterId, OutfitId, ShipId, StellarId, SystemId};
    use crate::chance::NeverFires;
    use crate::control::{Script, ScriptNote, SetExpr, Test};
    use crate::date::GameDate;
    use crate::market::ActiveEvent;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
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
        session.take_off(&mut NeverFires).expect("takes off");
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
        session.take_off(&mut chance).expect("takes off");
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
        session.take_off(&mut chance).expect("takes off");
        assert_eq!(chance.asked, Vec::<u8>::new(), "no roll");
    }

    #[test]
    fn the_date_saved_at_take_off_holds_its_days() {
        let mut session = shopping();
        let landed = session.date();
        session
            .buy_ship(NEW, "Kestrel", &mut NeverFires)
            .expect("buys");
        session.take_off(&mut NeverFires).expect("takes off");
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
        session.take_off(&mut NeverFires).expect("takes off");
        let mut expected = days_after(landed, 2);
        expected.extend([EXPLORED_130, TOOK_OFF_128]);
        assert_eq!(session.story_raised(), expected);
    }

    #[test]
    fn by_the_other_stay_reading_a_take_off_raises_only_explored_and_took_off() {
        let rules = Rulebook::default().with_override(RuleKey::StayDays, RuleSource::Bible);
        let mut session = quiet(shopping().with_rules(rules));
        assert_eq!(session.outfit(buy(SPEED), &mut NeverFires), Ok(()));
        session.take_off(&mut NeverFires).expect("takes off");
        assert_eq!(session.story_raised(), [EXPLORED_130, TOOK_OFF_128]);
    }

    #[test]
    fn a_take_off_unlanded_raises_nothing() {
        let catalog = testkit::catalog();
        let mut session = quiet(Session::start(&catalog).expect("starts"));
        assert_eq!(session.take_off(&mut NeverFires), None);
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
        session.take_off(&mut NeverFires).expect("takes off");
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
        session.take_off(&mut NeverFires).expect("takes off");
        land_and_settle(&mut session, &catalog);
        assert_eq!(bits(&session), [5], "once, and twice toggled");
        assert!(session.take_save_due());

        let saved = crate::save::decode(&crate::save::encode(session.pilot())).expect("loads");
        let mut session = Session::fly(&catalog, saved).expect("flies");
        register(&mut session, subs());
        session.take_off(&mut NeverFires).expect("takes off");
        land_and_settle(&mut session, &catalog);
        assert_eq!(bits(&session), [5, 6], "still fired after the reload");

        assert!(session.unregister_story(fixture(1)));
        assert!(!session.unregister_story(fixture(1)), "gone");
        assert!(session.take_save_due(), "its once-fired entry cleared");
        session.take_off(&mut NeverFires).expect("takes off");
        land_and_settle(&mut session, &catalog);
        assert_eq!(bits(&session), [5], "unregistered, it runs no more");
        register(&mut session, [subs()[0].clone()]);
        session.take_off(&mut NeverFires).expect("takes off");
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
        session.take_off(&mut NeverFires).expect("takes off");
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
}
