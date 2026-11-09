//! The session's side of the new game's missions: the scenes a landed
//! visit shows, the outcome an offer's conversation reaches, the story's
//! landing and day hooks, and the news a system has heard.
//!
//! The rules are the story layer's ([`storylines`](crate::storylines));
//! this module forwards to them with the session's state: the data on
//! [`Statics`] (`storylines`, from the catalog), the pilot's world flags
//! and story state, the room left in the hold, and the lanes.
//!
//! - **Scenes.** [`Converse::Open`](crate::flight_frame::Converse::Open)
//!   opens the next scene due at the landed stellar that this visit hasn't
//!   shown: a completion due there, then an offer that holds there, then
//!   the stellar's own conversation ([`Play::next_scene`]). The visit's
//!   scenes shown are kept on the [`Port`] and forgotten at take-off, so
//!   each shows at most once a visit, and an offer deferred comes back at
//!   a later landing. An offer's first opening applies its `on_offer`, and
//!   a completion's opening completes the mission, each before the
//!   conversation enters its knot, so the scene reads the flags they set.
//! - **Outcomes.** After an offer's conversation opens, and after each
//!   choice, the first `outcome:` tag its page reached decides the offer
//!   once ([`outcome_of`], [`Play::decide`]), against the room left in the
//!   ship's own holds, measured as it decides: after the landing has
//!   dropped its deliveries off and the offer's `on_offer` has fired its
//!   events, which may load cargo. A completion's or a stellar's own
//!   conversation reads no outcome. Closing, or taking off, with none
//!   reached leaves the offer offered. An accept refused for room leaves
//!   it offered too, decided for the visit, and raises its refusal
//!   ([`RoomRefusal`], drained as
//!   [`FlightEvent::RoomRefused`](crate::flight_frame::FlightEvent::RoomRefused))
//!   so the view can tell the player. An offer's first opening, an
//!   accept or a decline, and a completion each make a save due; a defer,
//!   an offer opened again and a stellar's own conversation don't.
//! - **Landing and days.** The `Storylines` consumer's subscriptions run
//!   [`Play::landed`] on each landing and [`Play::day_passed`] on each
//!   day, in the story dispatch. A landing only raises its event, and the
//!   app asks for the visit's first scene as the spaceport opens, before
//!   the next settling dispatches it. So the visit's landing is settled
//!   once, by whichever comes first: the dispatch, or the first
//!   [`Converse::Open`](crate::flight_frame::Converse::Open), which settles
//!   it before picking a scene, so a delivery made by the landing shows
//!   its completion first ([`Port::settle_landing`]). A change makes a
//!   save due. A session whose data holds no storylines (the stock
//!   game's) registers neither hook, and settling its landing finds
//!   nothing to do.
//! - **Outfits.** An effect block's outfit grants and removals are handed
//!   back by the story layer ([`OutfitChange`]) onto the
//!   [`Outbox`]'s queue, and [`Session::apply_story_outfits`] makes them,
//!   in order, through the `G` and `D` set operators
//!   ([`Session::apply_op`]; see the `outfits` module), so the stock fold,
//!   the refit and the rules for each (`GrantMax`, `RefitReserves`,
//!   `RemoveRefund`) apply as they do to a `mïsn` script. Each entry that
//!   plays the storylines drains the queue before it returns: after
//!   [`Converse::Open`](crate::flight_frame::Converse::Open) and
//!   [`Converse::Choose`](crate::flight_frame::Converse::Choose), and
//!   after the `Storylines` consumer's arm of the story dispatch. The
//!   changes apply at the end of the call that made them, after the
//!   flags, events and outcomes it settled: nothing there reads outfits,
//!   and no frame, dialog page or save comes between. The state change
//!   that applied the block (an offer, a decision, a success, an event)
//!   has made a save due already.
//! - **Blockades.** A `take_point` mission is met by beating the
//!   holder's guards at its point, not by passing its gate: a hypergate
//!   entry from or to the point's gate meets nothing. The guards are the
//!   mission's blockade, a hidden stock `mïsn` the storylines start while
//!   the mission is active and unmet and end once it isn't
//!   ([`Session::sync_blockades`], at the end of
//!   [`Session::settle_storylines`] and as a pilot is loaded). The
//!   blockade's goals-met moment meets the mission
//!   ([`Session::blockade_broken`], [`Play::beat_holder`]) and settles the
//!   storylines before the moment's settling returns. See the
//!   `blockades` module.
//! - **News.** [`Session::heard_news`] lists what a system has heard over
//!   the galaxy's lanes ([`news::heard`]).

use crate::catalog::{StellarId, SystemId};
use crate::chance::Chance;
use crate::control::{Script, SetOp};
use crate::dialog::conversation::{Conversation, ConversationPage, DialogFile};
use crate::pilot::Pilot;
use crate::story::{
    Consumer, EventFilter, StoryEvent, StoryEventKind, Subscription, SubscriptionId,
};
use crate::storylines::effects::OutfitChange;
use crate::storylines::lifecycle::{Decision, Play, Scene, outcome_of};
use crate::storylines::news::{self, HeardNews};
use nova_sim_catalog::dialogs::DialogAt;
use nova_sim_catalog::storylines::MissionObjective;

use crate::flight_frame::RoomRefusal;

use super::conversation::OpenConversation;
use super::free_cargo::free_cargo_in;
use super::{MissionRun, Outbox, Port, Session, Setup, Statics};

/// The storylines' landing hook ([`Play::landed`]).
pub(super) const LANDED: SubscriptionId = SubscriptionId {
    consumer: Consumer::Storylines,
    record: 0,
    hook: 0,
};

/// The storylines' daily hook ([`Play::day_passed`]).
pub(super) const DAY: SubscriptionId = SubscriptionId {
    consumer: Consumer::Storylines,
    record: 0,
    hook: 1,
};

impl MissionRun {
    /// Registers the storylines' landing and daily hooks, each with a
    /// blank guard and no script, when `statics` holds any mission: never
    /// for the stock game.
    pub(super) fn register_storylines(&mut self, statics: &Statics) {
        if statics.storylines.is_empty() {
            return;
        }
        for (id, kind) in [
            (LANDED, StoryEventKind::Landed),
            (DAY, StoryEventKind::DayPassed),
        ] {
            let hook = Subscription::new(id, EventFilter::Any(kind), Script::default());
            let registered = self.register_story(hook);
            debug_assert!(registered.is_ok(), "a fresh session registers it once");
        }
    }
}

/// The `Storylines` consumer's effect on `event`: a landing's deliveries
/// and successes, unless the visit has settled them already
/// ([`Port::settle_landing`]), or a day gone by, over `pilot`'s story,
/// each then checking events. A change makes a save due.
pub(super) fn settle(
    statics: &Statics,
    port: &mut Port,
    pilot: &mut Pilot,
    out: &mut Outbox,
    event: &StoryEvent,
) {
    match *event {
        StoryEvent::Landed { stellar } => port.settle_landing(statics, pilot, out, stellar),
        StoryEvent::DayPassed { .. } => {
            out.save_due |= Play {
                storylines: &statics.storylines,
                flags: &mut pilot.flags,
                story: &mut pilot.story,
                outfits: &mut out.story_outfits,
            }
            .day_passed();
        }
        _ => {}
    }
}

/// Where `scene`'s conversation is: its Ink file, compiled story and knot,
/// or `None` when its story isn't held.
pub(super) fn scene_place<'a>(statics: &'a Statics, scene: &Scene) -> Option<DialogAt<'a>> {
    let storylines = &statics.storylines;
    let place = match scene {
        Scene::Stellar(stellar) => return statics.stellar_dialogs.place(*stellar),
        Scene::Offer(id) => &storylines.missions.get(id)?.offer.as_ref()?.dialog,
        Scene::Completion(id) => &storylines.missions.get(id)?.complete.as_ref()?.dialog,
    };
    storylines.dialog(place)
}

impl Port {
    /// Opens the next scene due at `landed` this visit (see the module
    /// docs), replacing any open conversation, with `pilot`'s state of its
    /// file and its flags, an offer decided against the room left in
    /// `holds` once the landing and the offer have settled: its first
    /// page, or `None` in flight or with none left.
    pub(super) fn open_next_scene(
        &mut self,
        statics: &Statics,
        setup: &Setup,
        pilot: &mut Pilot,
        out: &mut Outbox,
        landed: Option<StellarId>,
        holds: u32,
    ) -> Option<ConversationPage> {
        let stellar = landed?;
        // The landing's story first, so its deliveries' completions are
        // due (see the module docs).
        self.settle_landing(statics, pilot, out, stellar);
        let has_own = statics.stellar_dialogs.place(stellar).is_some();
        let (scene, at) = loop {
            let scene = Play {
                storylines: &statics.storylines,
                flags: &mut pilot.flags,
                story: &mut pilot.story,
                outfits: &mut out.story_outfits,
            }
            .next_scene(stellar, &self.scenes_shown, has_own)?;
            self.scenes_shown.insert(scene.clone());
            // A scene whose story isn't held is passed over: the loader
            // keeps only missions whose stories are, so it isn't met.
            if let Some(at) = scene_place(statics, &scene) {
                break (scene, at);
            }
        };
        let mut play = Play {
            storylines: &statics.storylines,
            flags: &mut pilot.flags,
            story: &mut pilot.story,
            outfits: &mut out.story_outfits,
        };
        out.save_due |= match &scene {
            Scene::Offer(id) => play.make_offer(id),
            Scene::Completion(id) => play.complete(id),
            Scene::Stellar(_) => false,
        };
        let conversation = Conversation::open(
            setup.dialog_loader(),
            DialogFile {
                file: at.file,
                compiled: at.compiled,
            },
            at.knot,
            &mut pilot.dialogs,
            &pilot.flags,
        );
        let mut open = OpenConversation::new(stellar, scene, conversation);
        settle_outcome(&mut open, statics, pilot, out, holds);
        let page = open.conversation.page().clone();
        self.conversation = Some(open);
        Some(page)
    }

    /// Forgets the scenes this visit has shown, as the ship takes off.
    pub(super) fn forget_scenes(&mut self) {
        self.scenes_shown.clear();
        self.landing_settled = None;
    }

    /// Settles the storylines' side of the landing at `stellar`
    /// ([`Play::landed`]) once a visit: its deliveries, its successes and
    /// the events they make happen, over `pilot`'s story, a change making a
    /// save due. Nothing when this visit has settled it already.
    pub(super) fn settle_landing(
        &mut self,
        statics: &Statics,
        pilot: &mut Pilot,
        out: &mut Outbox,
        stellar: StellarId,
    ) {
        if self.landing_settled == Some(stellar) {
            return;
        }
        self.landing_settled = Some(stellar);
        out.save_due |= Play {
            storylines: &statics.storylines,
            flags: &mut pilot.flags,
            story: &mut pilot.story,
            outfits: &mut out.story_outfits,
        }
        .landed(stellar);
    }
}

/// Decides `open`'s offer by the first outcome its page reached, once,
/// against the room left in `holds`, measured from `pilot`'s cargo as it
/// decides, after whatever the landing and the offer loaded or dropped
/// off: nothing for a completion's or a stellar's own conversation, one
/// already decided, or a page with no outcome. A change makes a save due.
pub(super) fn settle_outcome(
    open: &mut OpenConversation,
    statics: &Statics,
    pilot: &mut Pilot,
    out: &mut Outbox,
    holds: u32,
) {
    let Scene::Offer(id) = &open.scene else {
        return;
    };
    if open.decided {
        return;
    }
    let Some(outcome) = outcome_of(open.conversation.lines()) else {
        return;
    };
    open.decided = true;
    let room = free_cargo_in(holds, pilot);
    let decision = Play {
        storylines: &statics.storylines,
        flags: &mut pilot.flags,
        story: &mut pilot.story,
        outfits: &mut out.story_outfits,
    }
    .decide(id, outcome, room);
    out.save_due |= decision.changed();
    if let Decision::NoRoom { tons, room } = decision {
        let mission = statics.storylines.missions.get(id);
        let cargo = match mission.and_then(|mission| mission.objective.as_ref()) {
            Some(MissionObjective::Deliver { cargo, .. }) => cargo.clone(),
            _ => String::new(),
        };
        out.room_refusals.push(RoomRefusal {
            mission: id.clone(),
            title: mission
                .map(|mission| mission.title.clone())
                .unwrap_or_default(),
            cargo,
            tons,
            room: room.max(0),
        });
    }
}

impl Session {
    /// Makes the outfit changes the storylines' effects have queued on the
    /// [`Outbox`], in order, each through `G` or `D` drawing on `chance`,
    /// and, when there were any, ends as a set expression does
    /// ([`Session::apply_set_ops`]): the observer sees each, the
    /// precalculated values reset, and a changed pilot marks the galaxy
    /// stale, so a `Visibility` test reading outfits is refreshed as after
    /// a script's `G` (see the module docs and the `precalced` module).
    ///
    /// The changes are made at the end of the call that queued them,
    /// rather than where their block applied, as the ship isn't the
    /// story's: nothing settled in between reads outfits, and no frame,
    /// dialog page or save comes between.
    pub(super) fn apply_story_outfits(&mut self, chance: &mut dyn Chance) {
        if self.out.story_outfits.is_empty() {
            return;
        }
        let changes = std::mem::take(&mut self.out.story_outfits);
        self.apply_set_ops(true, chance, |session, chance, apply| {
            for change in changes {
                let op = match change {
                    OutfitChange::Grant(outfit) => SetOp::GrantOutfit(outfit),
                    OutfitChange::Remove(outfit) => SetOp::RemoveOutfit(outfit),
                };
                apply(&op, session, chance);
            }
        });
    }

    /// The news `system` has heard by today, over each item's reach,
    /// frozen over the galaxy's lanes at its release (see the
    /// `gate_points` module), by the day each broke and then by id, each
    /// with how many days ago it was heard (see the `storylines` module).
    /// None in the stock game.
    #[must_use]
    pub fn heard_news(&self, system: SystemId) -> Vec<HeardNews> {
        news::heard(&self.statics.storylines, &self.pilot.story, system)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)] // shields compared as the refit leaves them
mod tests {
    //! What the session forwards to the story layer and what reaches the
    //! pilot and the page; the rules themselves are pinned in
    //! `nova_sim_story::storylines`.

    use std::collections::BTreeMap;
    use std::rc::Rc;
    use std::sync::Arc;

    use nova_sim_catalog::fixture::storylines::{deliver, offered_at, setting};
    use nova_sim_catalog::storylines::{
        MissionCompletion, NewsItem, PointAt, PointStability, PointState, Requires, StoryEffects,
        StoryMission, Storylines,
    };

    use super::*;
    use crate::catalog::{GovtId, OutfitId, OutfitRecord};
    use crate::chance::NeverFires;
    use crate::dialog::Value;
    use crate::dialog::fake::{FakeChoice, FakeDialog, FakeKnot, FakeLoader};
    use crate::flight_frame::{Answer, Converse, FlightCommand};
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::session::apply::tests::apply;
    use crate::stats::MORE_SHIELD;
    use crate::storylines::state::MissionState;
    use crate::testkit::{FakePilotCatalog, catalog, land_now, outfit};

    const STORY: &str = "job story";
    const FILE: &str = "dialog/job.ink";
    const DOCK: i16 = 128;

    /// The job's story: an offer whose choices end in `accept`,
    /// `decline` and `defer` (or none), its completion, whose first line
    /// shows while `paid` is set, and the planet's own greeting.
    fn loader() -> FakeLoader {
        let offer = FakeKnot::new("offer")
            .line("A job?", &[])
            .choice(FakeChoice::sticky("Take", &[]).line("Bay two.", &["outcome: accept"]))
            .choice(FakeChoice::sticky("Pass", &[]).line("Suit yourself.", &["outcome: decline"]))
            .choice(FakeChoice::sticky("Later", &[]).line("We'll wait.", &["outcome: defer"]))
            .choice(FakeChoice::sticky("Hmm", &[]).line("Well?", &[]));
        let done = FakeKnot::new("done")
            .line_when("paid", Value::Bool(true), "Paid in full.", &[])
            .line("Signed for.", &["outcome: decline"]);
        let greet = FakeKnot::new("greet").line("Hello.", &[]);
        let toll = FakeKnot::new("toll").line("The road's shut.", &[]);
        // An offer that reaches a second outcome after its first: `Later`
        // defers, then offers to take it after all; `Take` accepts, then
        // offers to take it again.
        let fickle = FakeKnot::new("fickle")
            .line("A job?", &[])
            .choice(
                FakeChoice::sticky("Later", &[])
                    .line("Later, then.", &["outcome: defer"])
                    .then("second"),
            )
            .choice(
                FakeChoice::sticky("Take", &[])
                    .line("Bay two.", &["outcome: accept"])
                    .then("second"),
            );
        let second = FakeKnot::new("second").line("Sure?", &[]).choice(
            FakeChoice::sticky("Take it after all", &[]).line("Fine.", &["outcome: accept"]),
        );
        // Offers whose first page reaches its outcome.
        let eager = FakeKnot::new("eager").line("Yours, then.", &["outcome: accept"]);
        let curt = FakeKnot::new("curt").line("Not for you.", &["outcome: decline"]);
        let dialog = FakeDialog::default()
            .declare("paid", Value::Bool(false))
            .knot(offer)
            .knot(done)
            .knot(greet)
            .knot(toll)
            .knot(fickle)
            .knot(second)
            .knot(eager)
            .knot(curt);
        FakeLoader::default().story(STORY, dialog)
    }

    /// The job: offered at the dock, `tons` of plate back to the dock, its
    /// completion there setting `paid`.
    fn job(tons: u32) -> StoryMission {
        StoryMission {
            objective: Some(deliver(tons, DOCK)),
            complete: Some(MissionCompletion {
                at: None,
                dialog: nova_sim_catalog::fixture::storylines::place(FILE, "done"),
            }),
            on_accept: setting(&["hauling"]),
            on_success: setting(&["paid"]),
            ..offered_at(DOCK, FILE, "offer")
        }
    }

    /// [`catalog`] with `missions`, the jump point `a/b` (stable), the
    /// news item `word` from system 130, and the dock greeting.
    fn talking(missions: &[(&str, StoryMission)]) -> FakePilotCatalog {
        let mut storylines =
            nova_sim_catalog::fixture::storylines::storylines(missions, FILE, STORY);
        storylines.points = BTreeMap::from([(
            "a/b".to_owned(),
            PointAt {
                gate: StellarId(140),
                owner: None,
                stability: PointStability::Stable,
            },
        )]);
        storylines.news = BTreeMap::from([(
            "word".to_owned(),
            NewsItem {
                headline: "Word".to_owned(),
                text: "It spreads.".to_owned(),
                from: SystemId(130),
            },
        )]);
        FakePilotCatalog {
            stellar_dialogs: nova_sim_catalog::dialogs::StellarDialogs::default().with(
                StellarId(DOCK),
                FILE,
                Arc::from(STORY),
                "greet",
            ),
            storylines,
            ..catalog()
        }
    }

    /// A session of `catalog`'s, wired to [`loader`], in flight.
    fn flying(catalog: &FakePilotCatalog) -> Session {
        Session::start(catalog)
            .expect("starts")
            .with_dialog_loader(Rc::new(loader()))
    }

    /// Lands at the dock and settles the landing's story.
    fn land(session: &mut Session, catalog: &FakePilotCatalog) {
        assert_eq!(land_now(session), Ok(StellarId(DOCK)));
        let _ = session.settle_script(catalog, &mut NeverFires);
    }

    /// Takes off from the dock and lands there again: a new visit.
    fn visit_again(session: &mut Session, catalog: &FakePilotCatalog) {
        assert_eq!(
            session.take_off(catalog, &mut NeverFires),
            Some(StellarId(DOCK))
        );
        let _ = session.settle_script(catalog, &mut NeverFires);
        land(session, catalog);
    }

    fn landed(catalog: &FakePilotCatalog) -> Session {
        let mut session = flying(catalog);
        land(&mut session, catalog);
        session
    }

    fn converse(session: &mut Session, converse: Converse) -> Answer {
        apply(session, FlightCommand::Converse(converse))
    }

    /// The lines of the page `answer` shows.
    fn lines(answer: &Answer) -> Vec<String> {
        match answer {
            Answer::Conversation(page) => page.lines.clone(),
            other => panic!("not a page: {other:?}"),
        }
    }

    fn state(session: &Session, id: &str) -> Option<MissionState> {
        session.pilot().story.mission(id)
    }

    fn free(session: &Session) -> i64 {
        session.ship.free_cargo(&session.pilot)
    }

    #[test]
    fn open_answers_the_offer_that_holds_and_the_pilot_shows_it_offered() {
        let catalog = talking(&[("job", job(10))]);
        let mut session = landed(&catalog);
        let page = converse(&mut session, Converse::Open);
        assert_eq!(lines(&page), ["A job?"]);
        assert_eq!(state(&session, "job"), Some(MissionState::Offered));
    }

    #[test]
    fn the_outcome_reached_decides_the_offer() {
        let catalog = talking(&[("job", job(10))]);
        let mut session = landed(&catalog);
        converse(&mut session, Converse::Open);
        assert_eq!(
            lines(&converse(&mut session, Converse::Choose(0))),
            ["Bay two."]
        );
        assert_eq!(
            state(&session, "job"),
            Some(MissionState::Active {
                cargo: 10,
                met: false
            })
        );
        assert!(session.pilot().flags.is_set("hauling"));

        let mut session = landed(&catalog);
        converse(&mut session, Converse::Open);
        converse(&mut session, Converse::Choose(1));
        assert_eq!(state(&session, "job"), Some(MissionState::Declined));
    }

    #[test]
    fn after_the_offer_open_gives_the_stellar_s_own_then_is_refused() {
        let catalog = talking(&[("job", job(10))]);
        let mut session = landed(&catalog);
        converse(&mut session, Converse::Open);
        assert_eq!(converse(&mut session, Converse::Close), Answer::Done);
        assert_eq!(lines(&converse(&mut session, Converse::Open)), ["Hello."]);
        assert_eq!(converse(&mut session, Converse::Open), Answer::Refused);
    }

    #[test]
    fn a_deferred_or_undecided_offer_opens_again_at_the_next_landing() {
        let catalog = talking(&[("job", job(10))]);
        let mut session = landed(&catalog);
        converse(&mut session, Converse::Open);
        converse(&mut session, Converse::Choose(2));
        assert_eq!(state(&session, "job"), Some(MissionState::Offered));
        visit_again(&mut session, &catalog);
        assert_eq!(lines(&converse(&mut session, Converse::Open)), ["A job?"]);
        converse(&mut session, Converse::Choose(3));
        assert_eq!(
            state(&session, "job"),
            Some(MissionState::Offered),
            "no outcome"
        );
        converse(&mut session, Converse::Close);
        visit_again(&mut session, &catalog);
        assert_eq!(lines(&converse(&mut session, Converse::Open)), ["A job?"]);
    }

    #[test]
    fn a_delivery_fills_the_hold_empties_it_on_landing_and_completes_with_its_flags() {
        let catalog = talking(&[("job", job(10))]);
        let mut session = landed(&catalog);
        let room = free(&session);
        converse(&mut session, Converse::Open);
        converse(&mut session, Converse::Choose(0));
        assert_eq!(free(&session), room - 10);
        converse(&mut session, Converse::Close);
        visit_again(&mut session, &catalog);
        assert_eq!(free(&session), room, "dropped off");
        session.out.save_due = false;
        assert_eq!(
            lines(&converse(&mut session, Converse::Open)),
            ["Paid in full.", "Signed for."],
            "on_success set `paid` before the knot was entered"
        );
        assert_eq!(state(&session, "job"), Some(MissionState::Done));
        assert!(session.pilot().flags.is_set("paid"));
        assert!(session.out.save_due, "the completion makes a save due");
    }

    #[test]
    fn opening_a_first_offer_makes_a_save_due() {
        let catalog = talking(&[("job", job(10))]);
        let mut session = landed(&catalog);
        session.out.save_due = false;
        converse(&mut session, Converse::Open);
        assert_eq!(state(&session, "job"), Some(MissionState::Offered));
        assert!(session.out.save_due);
    }

    #[test]
    fn choosing_an_accept_or_a_decline_makes_a_save_due() {
        let catalog = talking(&[("job", job(10))]);
        for (choice, decided) in [
            (
                0,
                MissionState::Active {
                    cargo: 10,
                    met: false,
                },
            ),
            (1, MissionState::Declined),
        ] {
            let mut session = landed(&catalog);
            converse(&mut session, Converse::Open);
            session.out.save_due = false;
            converse(&mut session, Converse::Choose(choice));
            assert_eq!(state(&session, "job"), Some(decided));
            assert!(session.out.save_due, "choice {choice}");
        }
    }

    #[test]
    fn a_defer_no_outcome_a_reopened_offer_or_the_stellar_s_own_makes_no_save_due() {
        let catalog = talking(&[("job", job(10))]);
        let mut session = landed(&catalog);
        converse(&mut session, Converse::Open);
        session.out.save_due = false;
        converse(&mut session, Converse::Choose(2));
        assert_eq!(state(&session, "job"), Some(MissionState::Offered));
        assert!(!session.out.save_due, "a defer changes nothing");
        converse(&mut session, Converse::Close);
        assert_eq!(lines(&converse(&mut session, Converse::Open)), ["Hello."]);
        assert!(!session.out.save_due, "the stellar's own");

        visit_again(&mut session, &catalog);
        session.out.save_due = false;
        assert_eq!(lines(&converse(&mut session, Converse::Open)), ["A job?"]);
        assert!(!session.out.save_due, "offered before");
        converse(&mut session, Converse::Choose(3));
        assert!(!session.out.save_due, "no outcome");

        let catalog = talking(&[]);
        let mut session = landed(&catalog);
        session.out.save_due = false;
        assert_eq!(lines(&converse(&mut session, Converse::Open)), ["Hello."]);
        assert!(!session.out.save_due, "no mission");
    }

    /// [`landed`] with the hold full but for `room` tons.
    fn landed_with_room(catalog: &FakePilotCatalog, room: i64) -> Session {
        let mut session = landed(catalog);
        let filler = u32::try_from(free(&session) - room).expect("a hold's tons");
        session
            .pilot
            .cargo
            .insert(crate::market::Good::Commodity(0), filler);
        session
    }

    /// The job, titled.
    fn titled_job(tons: u32) -> StoryMission {
        StoryMission {
            title: "Plate for the dock".to_owned(),
            ..job(tons)
        }
    }

    #[test]
    fn an_accept_without_room_raises_its_refusal_and_leaves_the_offer() {
        let catalog = talking(&[("job", titled_job(10))]);
        let mut session = landed_with_room(&catalog, 5);
        converse(&mut session, Converse::Open);
        session.out.save_due = false;
        assert_eq!(
            lines(&converse(&mut session, Converse::Choose(0))),
            ["Bay two."],
            "the accept's page"
        );
        assert_eq!(
            session.out.room_refusals,
            [RoomRefusal {
                mission: "job".to_owned(),
                title: "Plate for the dock".to_owned(),
                cargo: "Plate".to_owned(),
                tons: 10,
                room: 5,
            }]
        );
        assert_eq!(state(&session, "job"), Some(MissionState::Offered));
        assert_eq!(free(&session), 5, "nothing loaded");
        assert!(!session.out.save_due, "nothing changed");
    }

    #[test]
    fn a_refusal_counts_an_overfull_hold_as_no_room() {
        let catalog = talking(&[("job", titled_job(10))]);
        let mut session = landed_with_room(&catalog, 0);
        session
            .pilot
            .cargo
            .entry(crate::market::Good::Commodity(1))
            .or_insert(3);
        converse(&mut session, Converse::Open);
        converse(&mut session, Converse::Choose(0));
        let rooms: Vec<i64> = session.out.room_refusals.iter().map(|r| r.room).collect();
        assert_eq!(rooms, [0]);
    }

    #[test]
    fn a_fitting_accept_raises_no_refusal() {
        let catalog = talking(&[("job", titled_job(10))]);
        let mut session = landed_with_room(&catalog, 10);
        converse(&mut session, Converse::Open);
        converse(&mut session, Converse::Choose(0));
        assert_eq!(session.out.room_refusals, []);
        assert_eq!(
            state(&session, "job"),
            Some(MissionState::Active {
                cargo: 10,
                met: false
            })
        );
    }

    #[test]
    fn a_refused_offer_comes_back_at_the_next_landing() {
        let catalog = talking(&[("job", titled_job(10))]);
        let mut session = landed_with_room(&catalog, 5);
        converse(&mut session, Converse::Open);
        converse(&mut session, Converse::Choose(0));
        converse(&mut session, Converse::Close);
        visit_again(&mut session, &catalog);
        assert_eq!(lines(&converse(&mut session, Converse::Open)), ["A job?"]);
        assert_eq!(state(&session, "job"), Some(MissionState::Offered));
    }

    /// An event on day 3 that closes `a/b`, releasing `word`, and a toll
    /// offered at the dock once `a/b` is dead.
    fn closing() -> [(&'static str, StoryMission); 2] {
        let close = StoryMission {
            requires: Requires {
                not_before_day: Some(3),
                ..Requires::default()
            },
            on_offer: StoryEffects {
                points: vec![PointState {
                    point: "a/b".to_owned(),
                    owner: None,
                    stability: Some(PointStability::Dead),
                }],
                news: vec!["word".to_owned()],
                ..StoryEffects::default()
            },
            ..StoryMission::default()
        };
        let toll = StoryMission {
            requires: Requires {
                points: vec![PointState {
                    point: "a/b".to_owned(),
                    owner: None,
                    stability: Some(PointStability::Dead),
                }],
                ..Requires::default()
            },
            ..offered_at(DOCK, FILE, "toll")
        };
        [("close", close), ("toll", toll)]
    }

    #[test]
    fn days_passing_fire_an_event_whose_change_offers_at_the_next_landing() {
        let catalog = talking(&closing());
        let mut session = flying(&catalog);
        session.let_days_pass(2, &mut NeverFires);
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert_eq!(session.pilot().story.day, 2);
        assert_eq!(state(&session, "close"), None);
        session.let_days_pass(1, &mut NeverFires);
        session.out.save_due = false;
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert_eq!(state(&session, "close"), Some(MissionState::Done));
        assert!(session.out.save_due);
        land(&mut session, &catalog);
        assert_eq!(
            lines(&converse(&mut session, Converse::Open)),
            ["The road's shut."]
        );
    }

    #[test]
    fn heard_news_forwards_over_the_session_s_lanes() {
        let catalog = talking(&closing());
        let mut session = flying(&catalog);
        assert_eq!(session.heard_news(SystemId(130)), []);
        session.let_days_pass(5, &mut NeverFires);
        let _ = session.settle_script(&catalog, &mut NeverFires);
        let heard = session.heard_news(SystemId(130));
        assert_eq!(
            heard,
            [HeardNews {
                id: "word".to_owned(),
                headline: "Word".to_owned(),
                text: "It spreads.".to_owned(),
                from: SystemId(130),
                age: 2,
            }]
        );
    }

    #[test]
    fn storylines_register_their_landing_and_day_hooks() {
        let session = flying(&talking(&closing()));
        let landed = StoryEvent::Landed {
            stellar: StellarId(DOCK),
        };
        assert!(session.run.story_matching(&landed).contains(&LANDED));
        let day = StoryEvent::DayPassed {
            date: session.pilot().date,
        };
        assert!(session.run.story_matching(&day).contains(&DAY));
    }

    #[test]
    fn a_catalog_with_no_storylines_registers_no_hook_and_shows_its_own_conversation() {
        let catalog = FakePilotCatalog {
            storylines: Storylines::default(),
            ..talking(&[])
        };
        let mut session = landed(&catalog);
        let ids = session.run.story_matching(&StoryEvent::Landed {
            stellar: StellarId(DOCK),
        });
        assert!(
            ids.iter().all(|id| id.consumer != Consumer::Storylines),
            "{ids:?}"
        );
        assert_eq!(lines(&converse(&mut session, Converse::Open)), ["Hello."]);
        assert_eq!(
            session.pilot().story,
            crate::storylines::state::StoryState::default()
        );
    }

    /// The job, offered by the `fickle` knot, which reaches a second
    /// outcome after its first.
    fn fickle() -> StoryMission {
        StoryMission {
            offer: offered_at(DOCK, FILE, "fickle").offer,
            ..job(10)
        }
    }

    /// The job, offered by the conversation at `knot`.
    fn offered_by(knot: &str) -> StoryMission {
        StoryMission {
            offer: offered_at(DOCK, FILE, knot).offer,
            on_decline: setting(&["declined"]),
            ..job(10)
        }
    }

    #[test]
    fn an_outcome_on_the_offer_s_first_page_decides_it_as_it_opens() {
        let catalog = talking(&[("job", offered_by("eager"))]);
        let mut session = landed(&catalog);
        assert_eq!(
            lines(&converse(&mut session, Converse::Open)),
            ["Yours, then."]
        );
        assert_eq!(
            state(&session, "job"),
            Some(MissionState::Active {
                cargo: 10,
                met: false
            })
        );
        assert!(session.pilot().flags.is_set("hauling"));

        let catalog = talking(&[("job", offered_by("curt"))]);
        let mut session = landed(&catalog);
        assert_eq!(
            lines(&converse(&mut session, Converse::Open)),
            ["Not for you."]
        );
        assert_eq!(state(&session, "job"), Some(MissionState::Declined));
        assert!(session.pilot().flags.is_set("declined"));
    }

    #[test]
    fn only_the_first_outcome_an_offer_reaches_decides_it() {
        let catalog = talking(&[("job", fickle())]);
        let mut session = landed(&catalog);
        let room = free(&session);
        converse(&mut session, Converse::Open);
        assert_eq!(
            lines(&converse(&mut session, Converse::Choose(0))),
            ["Later, then.", "Sure?"]
        );
        assert_eq!(
            lines(&converse(&mut session, Converse::Choose(0))),
            ["Fine."]
        );
        assert_eq!(
            state(&session, "job"),
            Some(MissionState::Offered),
            "deferred"
        );
        assert_eq!(free(&session), room, "nothing loaded");
        assert!(!session.pilot().flags.is_set("hauling"));
    }

    #[test]
    fn an_accept_refused_for_room_decides_the_offer_for_the_visit() {
        let catalog = talking(&[("job", fickle())]);
        let mut session = landed(&catalog);
        let room = free(&session);
        // The hold full but for 5 tons, short of the job's 10.
        let filler = u32::try_from(room - 5).expect("a hold's tons");
        session
            .pilot
            .cargo
            .insert(crate::market::Good::Commodity(0), filler);
        converse(&mut session, Converse::Open);
        converse(&mut session, Converse::Choose(1));
        assert_eq!(
            state(&session, "job"),
            Some(MissionState::Offered),
            "refused"
        );
        // Room made before the second accept page: it decides nothing.
        session.pilot.cargo.clear();
        assert_eq!(
            lines(&converse(&mut session, Converse::Choose(0))),
            ["Fine."]
        );
        assert_eq!(state(&session, "job"), Some(MissionState::Offered));
        assert_eq!(free(&session), room, "nothing loaded");
    }

    /// Lands at the dock without settling the landing's story, as the app
    /// asks for the first scene before its next settling.
    fn land_unsettled(session: &mut Session) {
        assert_eq!(land_now(session), Ok(StellarId(DOCK)));
    }

    #[test]
    fn open_settles_a_landing_not_yet_dispatched_before_picking_the_scene() {
        let catalog = talking(&[("job", job(10))]);
        let mut session = flying(&catalog);
        session.pilot.story.missions.insert(
            "job".to_owned(),
            MissionState::Active {
                cargo: 10,
                met: false,
            },
        );
        land_unsettled(&mut session);
        session.out.save_due = false;
        assert_eq!(
            lines(&converse(&mut session, Converse::Open)),
            ["Paid in full.", "Signed for."],
            "delivered by the landing: the completion first"
        );
        assert_eq!(state(&session, "job"), Some(MissionState::Done));
        assert!(session.out.save_due);
        assert_eq!(lines(&converse(&mut session, Converse::Open)), ["Hello."]);
    }

    #[test]
    fn a_landing_open_has_settled_is_not_settled_again_by_the_dispatch() {
        // Offered at the dock and bound for it: delivered only by a later
        // landing, never by the landing it was taken at.
        let catalog = talking(&[("job", job(10))]);
        let mut session = flying(&catalog);
        land_unsettled(&mut session);
        converse(&mut session, Converse::Open);
        converse(&mut session, Converse::Choose(0));
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert_eq!(
            state(&session, "job"),
            Some(MissionState::Active {
                cargo: 10,
                met: false
            })
        );
        visit_again(&mut session, &catalog);
        assert_eq!(
            state(&session, "job"),
            Some(MissionState::Active {
                cargo: 0,
                met: true
            }),
            "the next landing delivers it"
        );
    }

    #[test]
    fn a_first_page_accept_counts_the_room_the_landing_frees() {
        // `bulk` fills the hold and is bound for the dock with no
        // completion: the landing Open settles drops it off, and the
        // eager offer's accept then finds the room.
        let bulk = StoryMission {
            objective: Some(deliver(1, DOCK)),
            ..StoryMission::default()
        };
        let catalog = talking(&[("bulk", bulk), ("job", offered_by("eager"))]);
        let mut session = flying(&catalog);
        let room = free(&session);
        session.pilot.story.missions.insert(
            "bulk".to_owned(),
            MissionState::Active {
                cargo: u32::try_from(room).expect("a hold's tons"),
                met: false,
            },
        );
        assert_eq!(free(&session), 0, "full before the drop-off");
        land_unsettled(&mut session);
        assert_eq!(
            lines(&converse(&mut session, Converse::Open)),
            ["Yours, then."]
        );
        assert_eq!(state(&session, "bulk"), Some(MissionState::Done));
        assert_eq!(
            state(&session, "job"),
            Some(MissionState::Active {
                cargo: 10,
                met: false
            })
        );
        assert_eq!(free(&session), room - 10);
    }

    #[test]
    fn a_landing_open_settles_makes_a_save_due_though_the_scene_it_shows_does_not() {
        // `bulk` is bound for the dock with no completion and nothing else
        // is offered: only the landing's drop-off changes the pilot, and
        // the stellar's own conversation makes no save due of its own.
        let bulk = StoryMission {
            objective: Some(deliver(4, DOCK)),
            ..StoryMission::default()
        };
        let catalog = talking(&[("bulk", bulk)]);
        let mut session = flying(&catalog);
        let room = free(&session);
        session.pilot.story.missions.insert(
            "bulk".to_owned(),
            MissionState::Active {
                cargo: 4,
                met: false,
            },
        );
        assert_eq!(free(&session), room - 4);
        land_unsettled(&mut session);
        session.out.save_due = false;
        assert_eq!(lines(&converse(&mut session, Converse::Open)), ["Hello."]);
        assert_eq!(state(&session, "bulk"), Some(MissionState::Done));
        assert_eq!(free(&session), room, "dropped off");
        assert!(session.out.save_due, "the landing's drop-off");
    }

    #[test]
    fn a_first_page_accept_counts_the_cargo_its_offer_loads() {
        // The eager offer's `on_offer` sets `loading`, which fires `load`:
        // an event that fills the hold but for 5 tons, short of the job's
        // 10.
        let catalog = talking(&[]);
        let room = free(&flying(&catalog));
        let load = StoryMission {
            requires: Requires {
                flags: vec!["loading".to_owned()],
                ..Requires::default()
            },
            objective: Some(deliver(
                u32::try_from(room - 5).expect("a hold's tons"),
                DOCK + 1,
            )),
            ..StoryMission::default()
        };
        let job = StoryMission {
            on_offer: setting(&["loading"]),
            ..offered_by("eager")
        };
        let catalog = talking(&[("job", job), ("load", load)]);
        let mut session = landed(&catalog);
        assert_eq!(
            lines(&converse(&mut session, Converse::Open)),
            ["Yours, then."]
        );
        assert!(matches!(
            state(&session, "load"),
            Some(MissionState::Active { .. })
        ));
        assert_eq!(
            state(&session, "job"),
            Some(MissionState::Offered),
            "refused for room"
        );
        assert_eq!(free(&session), 5);
    }

    #[test]
    fn a_met_mission_completes_as_its_completion_opens() {
        let catalog = talking(&[("job", job(10))]);
        let mut session = landed(&catalog);
        session.pilot.story.missions.insert(
            "job".to_owned(),
            MissionState::Active {
                cargo: 0,
                met: true,
            },
        );
        // The completion's page carries `outcome: decline`, which a
        // completion doesn't read.
        assert_eq!(
            lines(&converse(&mut session, Converse::Open)),
            ["Paid in full.", "Signed for."]
        );
        assert_eq!(state(&session, "job"), Some(MissionState::Done));
        assert!(session.pilot().flags.is_set("paid"));
        assert_eq!(lines(&converse(&mut session, Converse::Open)), ["Hello."]);
    }

    // Outfits granted and removed by the effects (see the module docs).

    /// The shield outfit the job lends: +10 shield, at most 2 owned.
    const PASS: OutfitId = OutfitId(305);

    /// [`talking`] with [`PASS`] in the catalog.
    fn lending(missions: &[(&str, StoryMission)]) -> FakePilotCatalog {
        FakePilotCatalog {
            outfits: vec![OutfitRecord {
                max: 2,
                ..outfit(PASS.0, &[(MORE_SHIELD, 10)])
            }],
            ..talking(missions)
        }
    }

    /// Effects granting `grant` and removing `remove`, by ID.
    fn changing(grant: &[OutfitId], remove: &[OutfitId]) -> StoryEffects {
        StoryEffects {
            grant_outfits: grant.to_vec(),
            remove_outfits: remove.to_vec(),
            ..StoryEffects::default()
        }
    }

    /// The job, its accept lending the pass and its success taking it
    /// back.
    fn lent() -> StoryMission {
        let job = job(10);
        StoryMission {
            on_accept: StoryEffects {
                grant_outfits: vec![PASS],
                ..job.on_accept.clone()
            },
            on_success: StoryEffects {
                remove_outfits: vec![PASS],
                ..job.on_success.clone()
            },
            ..job
        }
    }

    fn owned(session: &Session) -> u16 {
        session.pilot().owned(PASS)
    }

    #[test]
    fn an_accept_that_grants_an_outfit_gives_the_pilot_one_and_refits_the_ship() {
        let catalog = lending(&[("job", lent())]);
        let mut session = landed(&catalog);
        let shield = session.stats().shield;
        converse(&mut session, Converse::Open);
        assert_eq!(owned(&session), 0, "offered: nothing yet");
        session.out.save_due = false;
        converse(&mut session, Converse::Choose(0));
        assert_eq!(owned(&session), 1);
        assert_eq!(session.stats().shield, shield + 10.0, "refitted, as G does");
        assert!(session.out.save_due);
        assert_eq!(session.out.story_outfits, []);
    }

    #[test]
    fn an_offer_s_first_opening_and_a_completion_make_their_changes_as_they_open() {
        let job = StoryMission {
            on_offer: changing(&[PASS, PASS], &[]),
            ..lent()
        };
        let catalog = lending(&[("job", job)]);
        let mut session = landed(&catalog);
        converse(&mut session, Converse::Open);
        assert_eq!(owned(&session), 2, "on_offer");
        assert_eq!(session.out.story_outfits, []);
        session.pilot.story.missions.insert(
            "job".to_owned(),
            MissionState::Active {
                cargo: 0,
                met: true,
            },
        );
        visit_again(&mut session, &catalog);
        assert_eq!(
            lines(&converse(&mut session, Converse::Open)),
            ["Paid in full.", "Signed for."]
        );
        assert_eq!(owned(&session), 1, "on_success, as the completion opens");
        assert_eq!(session.out.story_outfits, []);
    }

    #[test]
    fn a_landing_that_succeeds_makes_its_changes_in_the_dispatch() {
        let job = StoryMission {
            complete: None,
            ..lent()
        };
        let catalog = lending(&[("job", job)]);
        let mut session = flying(&catalog);
        session.pilot.outfits.insert(PASS, 1);
        session.pilot.story.missions.insert(
            "job".to_owned(),
            MissionState::Active {
                cargo: 10,
                met: false,
            },
        );
        land(&mut session, &catalog);
        assert_eq!(state(&session, "job"), Some(MissionState::Done));
        assert_eq!(owned(&session), 0, "taken back on success");
        assert_eq!(session.out.story_outfits, []);
    }

    #[test]
    fn an_event_a_day_fires_removes_an_outfit_in_flight() {
        let [(_, close), _] = closing();
        let close = StoryMission {
            on_offer: changing(&[], &[PASS]),
            ..close
        };
        let catalog = lending(&[("close", close)]);
        let mut session = flying(&catalog);
        session.pilot.outfits.insert(PASS, 2);
        session.let_days_pass(3, &mut NeverFires);
        session.out.save_due = false;
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert_eq!(state(&session, "close"), Some(MissionState::Done));
        assert_eq!(owned(&session), 1);
        assert!(session.out.save_due);
        assert_eq!(session.out.story_outfits, []);
    }

    #[test]
    fn removing_an_outfit_not_owned_changes_nothing() {
        let job = StoryMission {
            on_accept: changing(&[], &[PASS]),
            ..job(10)
        };
        let catalog = lending(&[("job", job)]);
        let mut session = landed(&catalog);
        let cash = session.pilot().cash();
        let shield = session.stats().shield;
        converse(&mut session, Converse::Open);
        converse(&mut session, Converse::Choose(0));
        assert!(
            matches!(state(&session, "job"), Some(MissionState::Active { .. })),
            "accepted"
        );
        assert_eq!(owned(&session), 0);
        assert!(!session.pilot().outfits.contains_key(&PASS));
        assert_eq!(session.pilot().cash(), cash);
        assert_eq!(session.stats().shield, shield);
    }

    #[test]
    fn a_grant_and_a_removal_of_one_outfit_in_one_block_leave_the_count() {
        let job = StoryMission {
            on_accept: changing(&[PASS], &[PASS]),
            ..job(10)
        };
        let catalog = lending(&[("job", job)]);
        let mut session = landed(&catalog);
        session.pilot.outfits.insert(PASS, 1);
        converse(&mut session, Converse::Open);
        converse(&mut session, Converse::Choose(0));
        assert_eq!(owned(&session), 1);
    }

    /// Records each set operator it sees.
    #[derive(Debug, Default)]
    struct Ops(std::cell::RefCell<Vec<SetOp>>);

    impl crate::session::control::SetOpObserver for Ops {
        fn saw(&self, op: &SetOp, _session: &Session) {
            self.0.borrow_mut().push(op.clone());
        }
    }

    #[test]
    fn the_set_op_observer_sees_each_story_grant_and_removal() {
        let job = StoryMission {
            on_accept: changing(&[PASS], &[PASS]),
            ..job(10)
        };
        let catalog = lending(&[("job", job)]);
        let ops = Rc::new(Ops::default());
        let mut session = landed(&catalog).with_set_op_observer(ops.clone());
        converse(&mut session, Converse::Open);
        converse(&mut session, Converse::Choose(0));
        assert_eq!(
            *ops.0.borrow(),
            [SetOp::GrantOutfit(PASS), SetOp::RemoveOutfit(PASS)]
        );
    }

    #[test]
    fn a_grant_at_the_max_lands_by_the_engine_and_is_refused_by_the_other_reading() {
        let job = StoryMission {
            on_accept: changing(&[PASS], &[]),
            ..job(10)
        };
        let catalog = lending(&[("job", job)]);
        for (source, count) in [(RuleSource::Engine, 3), (RuleSource::Bible, 2)] {
            let rules = Rulebook::default().with_override(RuleKey::GrantMax, source);
            let mut session = flying(&catalog).with_rules(rules);
            land(&mut session, &catalog);
            session.pilot.outfits.insert(PASS, 2);
            converse(&mut session, Converse::Open);
            converse(&mut session, Converse::Choose(0));
            assert!(
                matches!(state(&session, "job"), Some(MissionState::Active { .. })),
                "{source:?}: accepted"
            );
            assert_eq!(owned(&session), count, "{source:?}");
        }
    }

    #[test]
    fn the_changes_reset_as_a_script_s_end_does_and_stale_the_galaxy_when_the_pilot_changed() {
        let job = StoryMission {
            on_offer: changing(&[], &[PASS]),
            ..lent()
        };
        let catalog = lending(&[("job", job)]);
        let mut session = landed(&catalog);
        let stale = |session: &mut Session| {
            session.ship.fuel_cap_cached = true;
            session.chart.galaxy_stale = false;
        };
        stale(&mut session);
        converse(&mut session, Converse::Open);
        assert!(!session.ship.fuel_cap_cached, "reset");
        assert!(
            !session.chart.galaxy_stale,
            "a removal of none: the pilot is as it was"
        );
        stale(&mut session);
        converse(&mut session, Converse::Choose(0));
        assert!(!session.ship.fuel_cap_cached, "reset");
        assert!(session.chart.galaxy_stale, "granted");
        stale(&mut session);
        converse(&mut session, Converse::Close);
        assert_eq!(lines(&converse(&mut session, Converse::Open)), ["Hello."]);
        assert!(session.ship.fuel_cap_cached, "nothing queued: no reset");
        assert!(!session.chart.galaxy_stale);
    }

    /// The guild's `gövt`.
    const GUILD: GovtId = GovtId(129);

    /// The job, its accept handing `a/b` to the guild.
    fn handing_over() -> StoryMission {
        let job = job(10);
        StoryMission {
            on_accept: StoryEffects {
                points: vec![PointState {
                    point: "a/b".to_owned(),
                    owner: Some("guild".to_owned()),
                    stability: None,
                }],
                ..job.on_accept.clone()
            },
            ..job
        }
    }

    /// The site of planet 129, in the dock's system, as the chart holds
    /// it.
    fn far_planet(session: &Session) -> crate::catalog::LandingSite {
        *session
            .chart
            .sites
            .iter()
            .find(|site| site.id == StellarId(129))
            .expect("shown")
    }

    #[test]
    fn a_choice_that_changes_a_point_restamps_its_gate_before_it_answers() {
        let mut catalog = talking(&[("job", handing_over())]);
        // `a/b`'s gate is planet 129, so the chart holds its site.
        catalog
            .storylines
            .points
            .get_mut("a/b")
            .expect("talking's point")
            .gate = StellarId(129);
        catalog
            .storylines
            .factions
            .insert("guild".to_owned(), GUILD);
        let mut session = landed(&catalog);
        converse(&mut session, Converse::Open);
        assert_ne!(far_planet(&session).govt, Some(GUILD));
        session.take_events();
        assert_eq!(
            lines(&converse(&mut session, Converse::Choose(0))),
            ["Bay two."]
        );
        assert_eq!(far_planet(&session).govt, Some(GUILD), "restamped");
        assert!(
            session
                .take_events()
                .contains(&crate::session::events::SessionEvent::GalaxyChanged)
        );
    }

    /// Gates: leaving through a point's gate meets no `take_point`.
    mod gates {
        use super::*;
        use crate::gate::GateKind;
        use crate::navigation::NavMode;
        use crate::session::LandPress;
        use crate::testkit::{HEARTH, laned_stable};

        /// The gate in drift to hearth, which the pilot starts over.
        const TO_HEARTH: StellarId = StellarId(300);
        /// Its partner, in hearth.
        const FROM_DRIFT: StellarId = StellarId(310);

        /// [`laned_stable`] with the road, taking the point `point`, and
        /// its `complete`; its success gives `hearth/drift-road` to the
        /// guild.
        fn roads(point: &str, complete: Option<MissionCompletion>) -> FakePilotCatalog {
            let mut catalog = laned_stable();
            catalog.storylines.missions.insert(
                "road".to_owned(),
                StoryMission {
                    objective: Some(nova_sim_catalog::storylines::MissionObjective::TakePoint {
                        point: point.to_owned(),
                        held_by: "warden".to_owned(),
                        blockade: None,
                    }),
                    complete,
                    on_success: StoryEffects {
                        points: vec![PointState {
                            point: "hearth/drift-road".to_owned(),
                            owner: Some("guild".to_owned()),
                            stability: None,
                        }],
                        ..StoryEffects::default()
                    },
                    ..offered_at(DOCK, FILE, "offer")
                },
            );
            catalog
                .storylines
                .factions
                .insert("guild".to_owned(), GUILD);
            catalog
        }

        /// A new pilot over `catalog`, in drift over gate 300, the road
        /// accepted, at the gate's entry, no save due.
        fn at_the_entry(catalog: &FakePilotCatalog) -> Session {
            let rules = Rulebook::default()
                .with_override(RuleKey::GateLanes, RuleSource::Bible)
                .with_override(RuleKey::GateCourse, RuleSource::Bible);
            let mut session = Session::start_ruled(catalog, &rules).expect("starts");
            session.pilot.story.missions.insert(
                "road".to_owned(),
                MissionState::Active {
                    cargo: 0,
                    met: false,
                },
            );
            session.chart.set_nav(NavMode::Stellar(Some(TO_HEARTH)));
            let _ = session.land();
            session.dock_approach();
            assert_eq!(
                session.land(),
                Ok(LandPress::AtGate {
                    stellar: TO_HEARTH,
                    kind: GateKind::Hypergate,
                })
            );
            session.out.save_due = false;
            session.take_events();
            session
        }

        fn site(session: &Session, id: StellarId) -> crate::catalog::LandingSite {
            *session
                .chart
                .sites
                .iter()
                .find(|site| site.id == id)
                .expect("shown")
        }

        const NOT_MET: Option<MissionState> = Some(MissionState::Active {
            cargo: 0,
            met: false,
        });

        #[test]
        fn leaving_through_the_point_s_gate_meets_nothing() {
            let catalog = roads("drift/hearth-road", None);
            let mut session = at_the_entry(&catalog);
            assert_eq!(
                session.enter_hypergate(Some(HEARTH), &catalog, &mut NeverFires),
                Ok(HEARTH)
            );
            assert_eq!(state(&session, "road"), NOT_MET, "the holder isn't beaten");
            assert!(!session.out.save_due);
            assert_eq!(site(&session, FROM_DRIFT).govt, None, "the owner stays");
        }
    }
}
