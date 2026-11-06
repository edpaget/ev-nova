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
//!   hold as the session measures it. A completion's or a stellar's own
//!   conversation reads no outcome. Closing, or taking off, with none
//!   reached leaves the offer offered.
//! - **Landing and days.** The `Storylines` consumer's subscriptions run
//!   [`Play::landed`] on each landing and [`Play::day_passed`] on each
//!   day, in the story dispatch, which settles before the next frame and
//!   before the view asks for the conversation. A change makes a save
//!   due. A session whose data holds no storylines (the stock game's)
//!   registers neither.
//! - **News.** [`Session::heard_news`] lists what a system has heard over
//!   the galaxy's lanes ([`news::heard`]).

use crate::catalog::{StellarId, SystemId};
use crate::control::Script;
use crate::dialog::conversation::{Conversation, ConversationPage, DialogFile};
use crate::pilot::Pilot;
use crate::story::{
    Consumer, EventFilter, StoryEvent, StoryEventKind, Subscription, SubscriptionId,
};
use crate::storylines::lifecycle::{Play, Scene, outcome_of};
use crate::storylines::news::{self, HeardNews};
use nova_sim_catalog::dialogs::DialogAt;

use super::conversation::OpenConversation;
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
/// and successes, or a day gone by, over `pilot`'s story, each then
/// checking events. A change makes a save due.
pub(super) fn settle(statics: &Statics, pilot: &mut Pilot, out: &mut Outbox, event: &StoryEvent) {
    let mut play = Play {
        storylines: &statics.storylines,
        flags: &mut pilot.flags,
        story: &mut pilot.story,
    };
    let changed = match *event {
        StoryEvent::Landed { stellar } => play.landed(stellar),
        StoryEvent::DayPassed { .. } => play.day_passed(),
        _ => false,
    };
    out.save_due |= changed;
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
    /// file and its flags, an offer decided against `room` tons free: its
    /// first page, or `None` in flight or with none left.
    pub(super) fn open_next_scene(
        &mut self,
        statics: &Statics,
        setup: &Setup,
        pilot: &mut Pilot,
        landed: Option<StellarId>,
        room: i64,
    ) -> Option<ConversationPage> {
        let stellar = landed?;
        let has_own = statics.stellar_dialogs.place(stellar).is_some();
        let (scene, at) = loop {
            let scene = Play {
                storylines: &statics.storylines,
                flags: &mut pilot.flags,
                story: &mut pilot.story,
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
        };
        match &scene {
            Scene::Offer(id) => {
                play.make_offer(id);
            }
            Scene::Completion(id) => {
                play.complete(id);
            }
            Scene::Stellar(_) => {}
        }
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
        settle_outcome(&mut open, statics, pilot, room);
        let page = open.conversation.page().clone();
        self.conversation = Some(open);
        Some(page)
    }

    /// Forgets the scenes this visit has shown, as the ship takes off.
    pub(super) fn forget_scenes(&mut self) {
        self.scenes_shown.clear();
    }
}

/// Decides `open`'s offer by the first outcome its page reached, once,
/// against `room` tons free: nothing for a completion's or a stellar's
/// own conversation, one already decided, or a page with no outcome.
pub(super) fn settle_outcome(
    open: &mut OpenConversation,
    statics: &Statics,
    pilot: &mut Pilot,
    room: i64,
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
    Play {
        storylines: &statics.storylines,
        flags: &mut pilot.flags,
        story: &mut pilot.story,
    }
    .decide(id, outcome, room);
}

impl Session {
    /// The news `system` has heard by today, over the galaxy's lanes, by
    /// the day each broke and then by id, each with how many days ago it
    /// was heard (see the `storylines` module). None in the stock game.
    #[must_use]
    pub fn heard_news(&self, system: SystemId) -> Vec<HeardNews> {
        news::heard(
            &self.statics.storylines,
            &self.pilot.story,
            self.chart.galaxy.lanes(),
            system,
        )
    }
}

#[cfg(test)]
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
    use crate::chance::NeverFires;
    use crate::dialog::Value;
    use crate::dialog::fake::{FakeChoice, FakeDialog, FakeKnot, FakeLoader};
    use crate::flight_frame::{Answer, Converse, FlightCommand};
    use crate::session::apply::tests::apply;
    use crate::storylines::state::MissionState;
    use crate::testkit::{FakePilotCatalog, catalog, land_now};

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
        let dialog = FakeDialog::default()
            .declare("paid", Value::Bool(false))
            .knot(offer)
            .knot(done)
            .knot(greet)
            .knot(toll);
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
        assert_eq!(
            lines(&converse(&mut session, Converse::Open)),
            ["Paid in full.", "Signed for."],
            "on_success set `paid` before the knot was entered"
        );
        assert_eq!(state(&session, "job"), Some(MissionState::Done));
        assert!(session.pilot().flags.is_set("paid"));
        assert!(session.out.save_due);
    }

    #[test]
    fn an_accept_without_room_leaves_it_offered() {
        let catalog = talking(&[("job", job(1_000_000))]);
        let mut session = landed(&catalog);
        let room = free(&session);
        converse(&mut session, Converse::Open);
        converse(&mut session, Converse::Choose(0));
        assert_eq!(state(&session, "job"), Some(MissionState::Offered));
        assert_eq!(free(&session), room);
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
}
