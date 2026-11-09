//! A mission's life, from offer to success, and the scenes a landed visit
//! shows.
//!
//! The rules come from the format's own docs (`missions/` in game-bible's
//! `data/README.md`, "Missions and world state"). Where those are silent,
//! the engine reads them as below, and the bible is asked to record the
//! reading:
//!
//! - **Offered.** A mission with an `offer` is offered at its stellar
//!   while its `requires` hold and it is unseen or
//!   [`Offered`](MissionState::Offered). Its `on_offer` applies once, the
//!   first time it is offered ([`Play::make_offer`]), so a mission offered
//!   again never releases its news twice.
//! - **The outcome** the offer's conversation reaches decides
//!   ([`Play::decide`]):
//!   - `accept` applies `on_accept`. With an objective the mission is then
//!     [`Active`](MissionState::Active), a `deliver` objective's tons
//!     loaded; without one it is done at once, and `on_success` applies
//!     too. An accept of a `deliver` mission whose tons don't fit the
//!     room left in the hold is refused: it is a defer, and nothing loads.
//!   - `decline` applies `on_decline`: the mission is
//!     [`Declined`](MissionState::Declined), and never offered again.
//!   - `defer`, or no outcome by the time the conversation closes,
//!     applies nothing: the mission is offered again at a later landing,
//!     while its `requires` hold.
//! - **Events.** A mission without an `offer` happens by itself once its
//!   `requires` hold and it is unseen: `on_offer` applies, then it is
//!   active (a `deliver` event's tons loaded whatever the room, as there
//!   is no conversation to refuse in), or done, with `on_success`, when it
//!   has no objective. Events are checked in id order after every change
//!   here, on each landing and on each day, again and again until none
//!   happens ([`Play::fire_events`]); each happens once, so that ends.
//! - **Objectives.** A `deliver` objective's cargo counts in the hold
//!   ([`StoryState::cargo_aboard`]). Landing at its stellar drops the
//!   cargo off and meets it ([`Play::landed`]). A `take_point` objective
//!   is met when the player leaves through its point's gate: a hypergate
//!   entry made from that gate that comes out ([`Play::passed_point`]).
//!   Arriving at the point from its partner meets nothing, nor does a
//!   landing. The format leaves taking a point open and the original has
//!   no such objective, so this is the engine's reading, with no rulebook
//!   key. It matches the sample's briefing ("Fly the blockade") and needs
//!   no ships; a holder's own gate is no bar to it, as a new-game
//!   government asks nothing of a pilot it hasn't met (`play-the-sample`
//!   phase 2 checked the sample's far gate, held by the Harrow band).
//!   `play-the-sample` phase 14 narrows it to beating the holder's ships.
//! - **Success.** A met mission with a completion conversation succeeds
//!   as that conversation opens at its stellar, or at the objective's
//!   when it names none ([`Play::complete`]), `on_success` applying
//!   before the conversation enters its knot so the scene reads the flags
//!   it sets. A met mission without one succeeds at the landing that
//!   meets it. The format has no failure.
//!
//! # A visit's scenes
//!
//! A landed visit shows, one at a time, the completions due at the
//! stellar (by mission id), then the offers that hold there (by mission
//! id), then the stellar's own conversation, each at most once a visit
//! ([`Play::next_scene`]). The scene is picked again each time one opens,
//! so a change one scene makes (a completion that makes an offer hold)
//! shows in the same visit.
//!
//! [`StoryState::cargo_aboard`]: super::state::StoryState::cargo_aboard

use std::collections::BTreeSet;

use nova_sim_catalog::storylines::{
    MissionObjective, Outcome, StoryEffects, StoryMission, Storylines,
};

use super::effects::{self, OutfitChange};
use super::requires;
use super::state::{MissionState, StoryState};
use crate::catalog::StellarId;
use crate::dialog::Line;
use crate::flags::WorldFlags;

/// A conversation a landed visit can show.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Scene {
    /// A mission's completion, by mission id.
    Completion(String),
    /// A mission's offer, by mission id.
    Offer(String),
    /// The stellar's own conversation.
    Stellar(StellarId),
}

/// The story played over a pilot's state: the data, and the pilot's world
/// flags and story state, which every rule here changes.
#[derive(Debug)]
pub struct Play<'a> {
    /// The missions, news and jump points.
    pub storylines: &'a Storylines,
    /// The pilot's world flags.
    pub flags: &'a mut WorldFlags,
    /// The pilot's story state.
    pub story: &'a mut StoryState,
    /// The outfit changes the effects applied hand back, in the order
    /// they applied ([`effects::apply`]), for the caller to make: the
    /// ship isn't the story's to change.
    pub outfits: &'a mut Vec<OutfitChange>,
}

impl Play<'_> {
    /// The next scene to show at `stellar` this visit, none of `shown`:
    /// a completion due there, an offer that holds there, or, when it has
    /// one (`has_own`), the stellar's own conversation (see the module
    /// docs). `None` when none is left.
    #[must_use]
    pub fn next_scene(
        &self,
        stellar: StellarId,
        shown: &BTreeSet<Scene>,
        has_own: bool,
    ) -> Option<Scene> {
        let missions = &self.storylines.missions;
        let completions = missions
            .iter()
            .filter(|&(id, mission)| self.completion_due(id, mission, stellar))
            .map(|(id, _)| Scene::Completion(id.clone()));
        let offers = missions
            .iter()
            .filter(|&(id, mission)| self.offer_holds(id, mission, stellar))
            .map(|(id, _)| Scene::Offer(id.clone()));
        let own = has_own.then_some(Scene::Stellar(stellar));
        completions
            .chain(offers)
            .chain(own)
            .find(|scene| !shown.contains(scene))
    }

    /// Whether mission `id`'s completion is due at `stellar`: it is met,
    /// and its completion plays there.
    fn completion_due(&self, id: &str, mission: &StoryMission, stellar: StellarId) -> bool {
        matches!(
            self.story.mission(id),
            Some(MissionState::Active { met: true, .. })
        ) && mission.complete.is_some()
            && completion_at(mission) == Some(stellar)
    }

    /// Whether mission `id` is offered at `stellar` now: it is offered
    /// there, unseen or offered before, and its `requires` hold.
    fn offer_holds(&self, id: &str, mission: &StoryMission, stellar: StellarId) -> bool {
        mission
            .offer
            .as_ref()
            .is_some_and(|offer| offer.at == stellar)
            && matches!(self.story.mission(id), None | Some(MissionState::Offered))
            && self.holds(mission)
    }

    /// Whether `mission`'s `requires` hold now.
    fn holds(&self, mission: &StoryMission) -> bool {
        requires::holds(&mission.requires, self.storylines, self.flags, self.story)
    }

    /// Offers mission `id`: the first time, `on_offer` applies and it is
    /// [`Offered`](MissionState::Offered), then events are checked.
    /// Whether anything changed.
    pub fn make_offer(&mut self, id: &str) -> bool {
        let Some(mission) = self.storylines.missions.get(id) else {
            return false;
        };
        if self.story.mission(id).is_some() {
            return false;
        }
        self.apply(&mission.on_offer);
        self.story
            .missions
            .insert(id.to_owned(), MissionState::Offered);
        self.fire_events();
        true
    }

    /// Decides offered mission `id` by `outcome`, with `room` tons free in
    /// the hold (see the module docs), then checks events. Whether
    /// anything changed: nothing does for a defer, a refused accept, or a
    /// mission not offered.
    pub fn decide(&mut self, id: &str, outcome: Outcome, room: i64) -> bool {
        let Some(mission) = self.storylines.missions.get(id) else {
            return false;
        };
        if self.story.mission(id) != Some(MissionState::Offered) {
            return false;
        }
        match outcome {
            Outcome::Accept => {
                if load(mission) > room {
                    return false;
                }
                self.apply(&mission.on_accept);
                self.start(id, mission);
            }
            Outcome::Decline => {
                self.apply(&mission.on_decline);
                self.story
                    .missions
                    .insert(id.to_owned(), MissionState::Declined);
            }
            Outcome::Defer => return false,
        }
        self.fire_events();
        true
    }

    /// Completes met mission `id`, as its completion conversation opens:
    /// `on_success` applies and it is [`Done`](MissionState::Done), then
    /// events are checked. Whether anything changed: nothing does for a
    /// mission not met.
    pub fn complete(&mut self, id: &str) -> bool {
        let Some(mission) = self.storylines.missions.get(id) else {
            return false;
        };
        if !matches!(
            self.story.mission(id),
            Some(MissionState::Active { met: true, .. })
        ) {
            return false;
        }
        self.succeed(id, mission);
        self.fire_events();
        true
    }

    /// Lands at `stellar`: each active `deliver` mission bound there drops
    /// its cargo off and is met, each met mission with no completion
    /// conversation succeeds, then events are checked. Whether anything
    /// changed.
    pub fn landed(&mut self, stellar: StellarId) -> bool {
        let mut changed = false;
        for (id, mission) in &self.storylines.missions {
            let Some(MissionState::Active { met: false, .. }) = self.story.mission(id) else {
                continue;
            };
            if let Some(MissionObjective::Deliver { to, .. }) = mission.objective
                && to == stellar
            {
                self.story.missions.insert(
                    id.clone(),
                    MissionState::Active {
                        cargo: 0,
                        met: true,
                    },
                );
                changed = true;
            }
        }
        changed |= self.succeed_met();
        self.fire_events() || changed
    }

    /// Passes through the jump point `point` (`system/point`), leaving by
    /// its gate: each active `take_point` mission on `point` is met, each
    /// met mission with no completion conversation succeeds, then events
    /// are checked (see the module docs). Whether anything changed.
    pub fn passed_point(&mut self, point: &str) -> bool {
        let mut changed = false;
        for (id, mission) in &self.storylines.missions {
            let Some(MissionState::Active { met: false, cargo }) = self.story.mission(id) else {
                continue;
            };
            if let Some(MissionObjective::TakePoint { point: taken, .. }) = &mission.objective
                && taken == point
            {
                self.story
                    .missions
                    .insert(id.clone(), MissionState::Active { cargo, met: true });
                changed = true;
            }
        }
        changed |= self.succeed_met();
        self.fire_events() || changed
    }

    /// Each met mission with no completion conversation succeeds. Whether
    /// any did.
    fn succeed_met(&mut self) -> bool {
        let mut changed = false;
        for (id, mission) in &self.storylines.missions {
            if mission.complete.is_none()
                && matches!(
                    self.story.mission(id),
                    Some(MissionState::Active { met: true, .. })
                )
            {
                self.succeed(id, mission);
                changed = true;
            }
        }
        changed
    }

    /// Passes a day: the day of the game goes on by one, then events are
    /// checked. Always a change.
    pub fn day_passed(&mut self) -> bool {
        self.story.day = self.story.day.saturating_add(1);
        self.fire_events();
        true
    }

    /// Makes every event whose `requires` hold happen, in id order, again
    /// until none does (see the module docs). Whether any did.
    pub fn fire_events(&mut self) -> bool {
        let mut fired = false;
        while let Some((id, mission)) = self.storylines.missions.iter().find(|&(id, mission)| {
            mission.offer.is_none() && self.story.mission(id).is_none() && self.holds(mission)
        }) {
            self.apply(&mission.on_offer);
            self.start(id, mission);
            fired = true;
        }
        fired
    }

    /// Starts mission `id`, accepted or happened: active, its cargo
    /// loaded, with an objective; done, with `on_success`, without one.
    fn start(&mut self, id: &str, mission: &StoryMission) {
        if mission.objective.is_some() {
            let cargo = u32::try_from(load(mission)).unwrap_or(u32::MAX);
            self.story
                .missions
                .insert(id.to_owned(), MissionState::Active { cargo, met: false });
        } else {
            self.succeed(id, mission);
        }
    }

    /// Mission `id` succeeds: `on_success` applies, and it is done.
    fn succeed(&mut self, id: &str, mission: &StoryMission) {
        self.apply(&mission.on_success);
        self.story
            .missions
            .insert(id.to_owned(), MissionState::Done);
    }

    fn apply(&mut self, effects: &StoryEffects) {
        effects::apply(effects, self.flags, self.story, self.outfits);
    }
}

/// The stellar `mission`'s completion plays at: its own, else its
/// `deliver` objective's. `None` when neither names one.
#[must_use]
pub fn completion_at(mission: &StoryMission) -> Option<StellarId> {
    let complete = mission.complete.as_ref()?;
    complete.at.or(match mission.objective {
        Some(MissionObjective::Deliver { to, .. }) => Some(to),
        _ => None,
    })
}

/// The tons `mission`'s objective loads: a `deliver`'s, else none.
fn load(mission: &StoryMission) -> i64 {
    match mission.objective {
        Some(MissionObjective::Deliver { tons, .. }) => i64::from(tons),
        _ => 0,
    }
}

/// The outcome `lines` reach: the first tag `outcome: <value>` whose value
/// names one (`accept`, `decline` or `defer`), spaces around either part
/// trimmed. A tag of another key, or whose value names no outcome, is
/// passed over.
#[must_use]
pub fn outcome_of(lines: &[Line]) -> Option<Outcome> {
    lines.iter().flat_map(|line| &line.tags).find_map(|tag| {
        let (key, value) = tag.split_once(':')?;
        (key.trim() == "outcome").then(|| Outcome::parse(value.trim()))?
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use nova_sim_catalog::fixture::storylines::{deliver, offered_at, requiring, setting};
    use nova_sim_catalog::storylines::{
        MissionCompletion, NewsItem, PointAt, PointStability, PointState, Requires,
    };

    use super::*;
    use crate::catalog::{OutfitId, SystemId};
    use crate::storylines::state::PointChange;

    const DOCK: i16 = 128;
    const CAMP: i16 = 129;
    const FILE: &str = "dialog/job.ink";

    fn place(knot: &str) -> nova_sim_catalog::storylines::DialogPlace {
        nova_sim_catalog::fixture::storylines::place(FILE, knot)
    }

    /// A pilot's flags and story, the data they play over, and the outfit
    /// changes handed back.
    struct Fixture {
        storylines: Storylines,
        flags: WorldFlags,
        story: StoryState,
        outfits: Vec<OutfitChange>,
    }

    impl Fixture {
        fn new(missions: &[(&str, StoryMission)]) -> Self {
            Self {
                storylines: Storylines {
                    missions: missions
                        .iter()
                        .map(|(id, mission)| ((*id).to_owned(), mission.clone()))
                        .collect(),
                    news: BTreeMap::from([(
                        "word".to_owned(),
                        NewsItem {
                            headline: "Word".to_owned(),
                            text: String::new(),
                            from: SystemId(128),
                        },
                    )]),
                    points: BTreeMap::from([(
                        "hearth/side".to_owned(),
                        PointAt {
                            gate: StellarId(140),
                            owner: None,
                            stability: PointStability::Stable,
                        },
                    )]),
                    ..Storylines::default()
                },
                flags: WorldFlags::default(),
                story: StoryState::default(),
                outfits: Vec::new(),
            }
        }

        fn play(&mut self) -> Play<'_> {
            Play {
                storylines: &self.storylines,
                flags: &mut self.flags,
                story: &mut self.story,
                outfits: &mut self.outfits,
            }
        }

        fn state(&self, id: &str) -> Option<MissionState> {
            self.story.mission(id)
        }

        fn next(&mut self, stellar: i16, shown: &[Scene], has_own: bool) -> Option<Scene> {
            let shown = shown.iter().cloned().collect();
            self.play().next_scene(StellarId(stellar), &shown, has_own)
        }
    }

    fn offer() -> Scene {
        Scene::Offer("job".to_owned())
    }

    /// The dock job: offered at the dock, ten tons to the camp, completed
    /// there, each effect block setting a flag of its name.
    fn job() -> StoryMission {
        StoryMission {
            objective: Some(deliver(10, CAMP)),
            complete: Some(MissionCompletion {
                at: None,
                dialog: place("done"),
            }),
            on_offer: setting(&["offered"]),
            on_accept: setting(&["accepted"]),
            on_decline: setting(&["declined"]),
            on_success: setting(&["succeeded"]),
            ..offered_at(DOCK, FILE, "offer")
        }
    }

    const ACTIVE: MissionState = MissionState::Active {
        cargo: 10,
        met: false,
    };

    #[test]
    fn on_offer_applies_once_across_two_offers() {
        let mut f = Fixture::new(&[(
            "job",
            StoryMission {
                on_offer: StoryEffects {
                    news: vec!["word".to_owned()],
                    ..setting(&["offered"])
                },
                ..job()
            },
        )]);
        assert!(f.play().make_offer("job"));
        assert_eq!(f.state("job"), Some(MissionState::Offered));
        assert!(f.flags.is_set("offered"));
        f.flags.clear("offered");
        f.story.day = 4;
        assert!(!f.play().make_offer("job"), "offered again: no change");
        assert!(!f.flags.is_set("offered"));
        assert_eq!(f.story.news["word"], 0);
    }

    #[test]
    fn a_cascade_hands_back_its_outfit_changes_in_order() {
        let pass = OutfitId(130);
        let mut f = Fixture::new(&[
            (
                "job",
                StoryMission {
                    on_accept: StoryEffects {
                        grant_outfits: vec![pass],
                        ..setting(&["accepted"])
                    },
                    ..job()
                },
            ),
            (
                "recall",
                StoryMission {
                    offer: None,
                    requires: requiring(&["accepted"]),
                    on_offer: StoryEffects {
                        remove_outfits: vec![pass],
                        ..StoryEffects::default()
                    },
                    ..StoryMission::default()
                },
            ),
        ]);
        f.play().make_offer("job");
        assert_eq!(f.outfits, []);
        assert!(f.play().decide("job", Outcome::Accept, 10));
        assert_eq!(
            f.outfits,
            [OutfitChange::Grant(pass), OutfitChange::Remove(pass)]
        );
    }

    #[test]
    fn an_unknown_mission_changes_nothing() {
        let mut f = Fixture::new(&[]);
        assert!(!f.play().make_offer("job"));
        assert!(!f.play().decide("job", Outcome::Accept, 100));
        assert!(!f.play().complete("job"));
        assert_eq!(f.story, StoryState::default());
    }

    #[test]
    fn accepting_with_an_objective_applies_on_accept_and_loads_the_cargo() {
        let mut f = Fixture::new(&[("job", job())]);
        f.play().make_offer("job");
        assert!(f.play().decide("job", Outcome::Accept, 10));
        assert_eq!(f.state("job"), Some(ACTIVE));
        assert!(f.flags.is_set("accepted"));
        assert!(!f.flags.is_set("succeeded"));
        assert_eq!(f.story.cargo_aboard(), 10);
    }

    #[test]
    fn accepting_without_an_objective_is_done_and_applies_on_success() {
        let mut f = Fixture::new(&[(
            "job",
            StoryMission {
                objective: None,
                ..job()
            },
        )]);
        f.play().make_offer("job");
        assert!(f.play().decide("job", Outcome::Accept, 0));
        assert_eq!(f.state("job"), Some(MissionState::Done));
        assert!(f.flags.is_set("accepted"));
        assert!(f.flags.is_set("succeeded"));
    }

    #[test]
    fn an_accept_without_room_is_a_defer_and_loads_nothing() {
        let mut f = Fixture::new(&[("job", job())]);
        f.play().make_offer("job");
        assert!(!f.play().decide("job", Outcome::Accept, 9));
        assert_eq!(f.state("job"), Some(MissionState::Offered));
        assert!(!f.flags.is_set("accepted"));
        assert_eq!(f.story.cargo_aboard(), 0);
        assert_eq!(f.next(DOCK, &[], false), Some(offer()), "offered again");
    }

    #[test]
    fn a_decline_applies_on_decline_and_is_never_offered_again() {
        let mut f = Fixture::new(&[("job", job())]);
        f.play().make_offer("job");
        assert!(f.play().decide("job", Outcome::Decline, 100));
        assert_eq!(f.state("job"), Some(MissionState::Declined));
        assert!(f.flags.is_set("declined"));
        assert_eq!(f.next(DOCK, &[], false), None);
        assert!(!f.play().decide("job", Outcome::Accept, 100), "decided");
    }

    #[test]
    fn a_defer_changes_nothing_and_offers_again_while_requires_hold() {
        let mut f = Fixture::new(&[(
            "job",
            StoryMission {
                requires: requiring(&["open"]),
                ..job()
            },
        )]);
        f.flags.set("open");
        f.play().make_offer("job");
        let before = (f.flags.clone(), f.story.clone());
        assert!(!f.play().decide("job", Outcome::Defer, 100));
        assert_eq!((f.flags.clone(), f.story.clone()), before);
        assert_eq!(f.next(DOCK, &[], false), Some(offer()));
        f.flags.clear("open");
        assert_eq!(f.next(DOCK, &[], false), None, "requires no longer hold");
    }

    #[test]
    fn only_an_offered_mission_is_decided() {
        let mut f = Fixture::new(&[("job", job())]);
        assert!(!f.play().decide("job", Outcome::Accept, 100), "not offered");
        assert_eq!(f.state("job"), None);
    }

    #[test]
    fn landing_at_the_deliver_stellar_drops_the_cargo_and_meets_it() {
        let mut f = Fixture::new(&[("job", job())]);
        f.play().make_offer("job");
        f.play().decide("job", Outcome::Accept, 10);
        assert!(!f.play().landed(StellarId(DOCK)), "elsewhere");
        assert_eq!(f.state("job"), Some(ACTIVE));
        assert!(f.play().landed(StellarId(CAMP)));
        assert_eq!(
            f.state("job"),
            Some(MissionState::Active {
                cargo: 0,
                met: true
            })
        );
        assert_eq!(f.story.cargo_aboard(), 0);
        assert!(!f.flags.is_set("succeeded"), "its completion is to play");
        assert!(!f.play().landed(StellarId(CAMP)), "met already");
    }

    #[test]
    fn a_completion_is_due_where_it_plays_once_met_and_completing_succeeds() {
        let mut f = Fixture::new(&[("job", job())]);
        f.play().make_offer("job");
        f.play().decide("job", Outcome::Accept, 10);
        assert_eq!(f.next(CAMP, &[], false), None, "not met");
        assert!(!f.play().complete("job"), "not met");
        f.play().landed(StellarId(CAMP));
        let done = Scene::Completion("job".to_owned());
        assert_eq!(f.next(CAMP, &[], false), Some(done.clone()));
        assert_eq!(f.next(DOCK, &[], false), None);
        assert!(f.play().complete("job"));
        assert_eq!(f.state("job"), Some(MissionState::Done));
        assert!(f.flags.is_set("succeeded"));
        assert_eq!(f.next(CAMP, &[], false), None);
    }

    #[test]
    fn a_completion_at_its_own_stellar_plays_there() {
        let mission = StoryMission {
            complete: Some(MissionCompletion {
                at: Some(StellarId(DOCK)),
                dialog: place("done"),
            }),
            ..job()
        };
        assert_eq!(completion_at(&mission), Some(StellarId(DOCK)));
        assert_eq!(
            completion_at(&job()),
            Some(StellarId(CAMP)),
            "the deliver's"
        );
        let take = StoryMission {
            objective: Some(MissionObjective::TakePoint {
                point: "hearth/side".to_owned(),
                held_by: "warden".to_owned(),
                blockade: None,
            }),
            ..job()
        };
        assert_eq!(completion_at(&take), None);
        let mut f = Fixture::new(&[("job", mission)]);
        f.play().make_offer("job");
        f.play().decide("job", Outcome::Accept, 10);
        f.play().landed(StellarId(CAMP));
        assert_eq!(f.next(CAMP, &[], false), None);
        assert_eq!(
            f.next(DOCK, &[], false),
            Some(Scene::Completion("job".to_owned()))
        );
    }

    #[test]
    fn without_a_completion_it_succeeds_on_the_landing_that_meets_it() {
        let mut f = Fixture::new(&[(
            "job",
            StoryMission {
                complete: None,
                ..job()
            },
        )]);
        f.play().make_offer("job");
        f.play().decide("job", Outcome::Accept, 10);
        assert!(f.play().landed(StellarId(CAMP)));
        assert_eq!(f.state("job"), Some(MissionState::Done));
        assert!(f.flags.is_set("succeeded"));
        assert_eq!(f.story.cargo_aboard(), 0);
    }

    #[test]
    fn a_take_point_mission_stays_active_through_landings_and_days() {
        let mut f = Fixture::new(&[(
            "road",
            StoryMission {
                objective: Some(MissionObjective::TakePoint {
                    point: "hearth/side".to_owned(),
                    held_by: "warden".to_owned(),
                    blockade: None,
                }),
                complete: None,
                ..job()
            },
        )]);
        f.play().make_offer("road");
        assert!(f.play().decide("road", Outcome::Accept, 0));
        let active = Some(MissionState::Active {
            cargo: 0,
            met: false,
        });
        assert_eq!(f.state("road"), active);
        for stellar in [DOCK, CAMP, 140] {
            f.play().landed(StellarId(stellar));
        }
        f.play().day_passed();
        assert_eq!(f.state("road"), active);
    }

    /// A `take_point` objective on `point`.
    fn taking(point: &str) -> MissionObjective {
        MissionObjective::TakePoint {
            point: point.to_owned(),
            held_by: "warden".to_owned(),
            blockade: None,
        }
    }

    /// The road: offered at the dock, taking the side road's point, with
    /// no completion, its success giving the point to the guild.
    fn road() -> StoryMission {
        StoryMission {
            objective: Some(taking("hearth/side")),
            complete: None,
            on_success: StoryEffects {
                points: vec![PointState {
                    point: "hearth/side".to_owned(),
                    owner: Some("guild".to_owned()),
                    stability: None,
                }],
                ..setting(&["succeeded"])
            },
            ..job()
        }
    }

    /// `f` with mission `id` offered and accepted.
    fn accepted(f: &mut Fixture, id: &str) {
        f.play().make_offer(id);
        assert!(f.play().decide(id, Outcome::Accept, 100));
    }

    #[test]
    fn passing_the_point_meets_its_take_point_and_one_without_a_completion_succeeds() {
        let mut f = Fixture::new(&[("road", road())]);
        accepted(&mut f, "road");
        assert!(f.play().passed_point("hearth/side"));
        assert_eq!(f.state("road"), Some(MissionState::Done));
        assert!(f.flags.is_set("succeeded"));
        assert_eq!(
            f.story.points["hearth/side"],
            PointChange {
                owner: Some("guild".to_owned()),
                stability: None,
            }
        );
        assert!(!f.play().passed_point("hearth/side"), "done: no change");
    }

    #[test]
    fn passing_the_point_leaves_one_with_a_completion_met_for_its_scene() {
        let mut f = Fixture::new(&[(
            "road",
            StoryMission {
                complete: Some(MissionCompletion {
                    at: Some(StellarId(CAMP)),
                    dialog: place("done"),
                }),
                ..road()
            },
        )]);
        accepted(&mut f, "road");
        assert!(f.play().passed_point("hearth/side"));
        assert_eq!(
            f.state("road"),
            Some(MissionState::Active {
                cargo: 0,
                met: true
            })
        );
        assert!(!f.flags.is_set("succeeded"), "its completion is to play");
        assert_eq!(
            f.next(CAMP, &[], false),
            Some(Scene::Completion("road".to_owned()))
        );
        assert!(!f.play().passed_point("hearth/side"), "met already");
    }

    #[test]
    fn passing_the_point_meets_nothing_else() {
        let mut f = Fixture::new(&[
            (
                "elsewhere",
                StoryMission {
                    objective: Some(taking("hearth/main")),
                    ..road()
                },
            ),
            ("job", job()),
            ("unseen", road()),
        ]);
        accepted(&mut f, "elsewhere");
        accepted(&mut f, "job");
        assert!(!f.play().passed_point("hearth/side"));
        assert_eq!(
            f.state("elsewhere"),
            Some(MissionState::Active {
                cargo: 0,
                met: false
            }),
            "another point's"
        );
        assert_eq!(f.state("job"), Some(ACTIVE), "a deliver");
        assert_eq!(f.state("unseen"), None, "not accepted");
        f.play().make_offer("unseen");
        assert!(
            !f.play().passed_point("hearth/side"),
            "offered, not accepted"
        );
        assert_eq!(f.state("unseen"), Some(MissionState::Offered));
    }

    #[test]
    fn passing_the_point_makes_an_event_waiting_on_the_success_happen() {
        let mut f = Fixture::new(&[
            ("road", road()),
            (
                "event",
                waiting_on(Requires {
                    missions_done: vec!["road".to_owned()],
                    ..Requires::default()
                }),
            ),
        ]);
        accepted(&mut f, "road");
        assert_eq!(f.state("event"), None);
        assert!(f.play().passed_point("hearth/side"));
        assert_eq!(f.state("event"), Some(MissionState::Done));
        assert!(f.flags.is_set("fired"));
    }

    #[test]
    fn passing_the_point_with_nothing_to_meet_changes_nothing() {
        let mut f = Fixture::new(&[]);
        assert!(!f.play().passed_point("hearth/side"));
        assert_eq!(f.story, StoryState::default());
    }

    /// An event on day `day`, releasing `word` and closing the side road.
    fn closing(day: u32) -> StoryMission {
        StoryMission {
            offer: None,
            requires: Requires {
                not_before_day: Some(day),
                ..Requires::default()
            },
            on_offer: StoryEffects {
                points: vec![PointState {
                    point: "hearth/side".to_owned(),
                    owner: None,
                    stability: Some(PointStability::Dead),
                }],
                news: vec!["word".to_owned()],
                ..StoryEffects::default()
            },
            on_success: setting(&["closed"]),
            ..StoryMission::default()
        }
    }

    #[test]
    fn an_event_happens_on_its_day_and_once() {
        let mut f = Fixture::new(&[("close", closing(2))]);
        assert!(f.play().day_passed());
        assert_eq!(f.story.day, 1);
        assert_eq!(f.state("close"), None);
        f.play().day_passed();
        assert_eq!(f.state("close"), Some(MissionState::Done));
        assert!(f.flags.is_set("closed"), "no objective: on_success");
        assert_eq!(f.story.news["word"], 2);
        assert_eq!(
            f.story.points["hearth/side"],
            PointChange {
                owner: None,
                stability: Some(PointStability::Dead),
            }
        );
        f.flags.clear("closed");
        f.play().day_passed();
        assert!(!f.play().fire_events());
        assert!(!f.flags.is_set("closed"), "once");
    }

    #[test]
    fn an_event_with_an_objective_starts_active_with_its_cargo_whatever_the_room() {
        let mut f = Fixture::new(&[(
            "haul",
            StoryMission {
                objective: Some(deliver(500, CAMP)),
                ..closing(0)
            },
        )]);
        assert!(f.play().fire_events());
        assert_eq!(
            f.state("haul"),
            Some(MissionState::Active {
                cargo: 500,
                met: false
            })
        );
        assert!(!f.flags.is_set("closed"));
    }

    #[test]
    fn a_chain_of_two_events_happens_in_one_cascade() {
        let second = StoryMission {
            requires: Requires {
                missions_done: vec!["close".to_owned()],
                ..Requires::default()
            },
            on_offer: setting(&["second"]),
            ..StoryMission::default()
        };
        // `a-second` sorts before `close`, so only the cascade fires it.
        let mut f = Fixture::new(&[("a-second", second), ("close", closing(0))]);
        assert!(f.play().landed(StellarId(DOCK)));
        assert_eq!(f.state("close"), Some(MissionState::Done));
        assert_eq!(f.state("a-second"), Some(MissionState::Done));
        assert!(f.flags.is_set("second"));
    }

    /// An event that happens once `requires` hold, setting `fired`.
    fn waiting_on(requires: Requires) -> StoryMission {
        StoryMission {
            requires,
            on_offer: setting(&["fired"]),
            ..StoryMission::default()
        }
    }

    /// The job and an event waiting on `requires`, neither seen.
    fn with_event(requires: Requires) -> Fixture {
        Fixture::new(&[("job", job()), ("event", waiting_on(requires))])
    }

    #[test]
    fn an_offer_s_effects_make_an_event_happen_at_once() {
        let mut f = with_event(requiring(&["offered"]));
        assert!(f.play().make_offer("job"));
        assert_eq!(f.state("event"), Some(MissionState::Done));
        assert!(f.flags.is_set("fired"));
    }

    #[test]
    fn an_accept_s_effects_make_an_event_happen_at_once() {
        let mut f = with_event(requiring(&["accepted"]));
        f.play().make_offer("job");
        assert_eq!(f.state("event"), None);
        assert!(f.play().decide("job", Outcome::Accept, 10));
        assert_eq!(f.state("event"), Some(MissionState::Done));
    }

    #[test]
    fn a_decline_s_effects_make_an_event_happen_at_once() {
        let mut f = with_event(requiring(&["declined"]));
        f.play().make_offer("job");
        assert!(f.play().decide("job", Outcome::Decline, 10));
        assert_eq!(f.state("event"), Some(MissionState::Done));
    }

    #[test]
    fn a_completion_makes_an_event_waiting_on_the_mission_happen_at_once() {
        let mut f = with_event(Requires {
            missions_done: vec!["job".to_owned()],
            ..Requires::default()
        });
        f.story.missions.insert(
            "job".to_owned(),
            MissionState::Active {
                cargo: 0,
                met: true,
            },
        );
        assert!(f.play().complete("job"));
        assert_eq!(f.state("event"), Some(MissionState::Done));
        assert!(f.flags.is_set("fired"));
    }

    #[test]
    fn an_event_with_an_objective_started_by_an_effect_is_active_at_once() {
        let mut f = Fixture::new(&[
            ("job", job()),
            (
                "haul",
                StoryMission {
                    objective: Some(deliver(5, DOCK)),
                    ..waiting_on(requiring(&["offered"]))
                },
            ),
        ]);
        f.play().make_offer("job");
        assert_eq!(
            f.state("haul"),
            Some(MissionState::Active {
                cargo: 5,
                met: false
            })
        );
    }

    #[test]
    fn an_effect_that_makes_an_offer_hold_makes_it_the_next_scene() {
        let toll = StoryMission {
            requires: Requires {
                missions_done: vec!["job".to_owned()],
                ..Requires::default()
            },
            ..offered_at(CAMP, FILE, "toll")
        };
        let mut f = Fixture::new(&[("job", job()), ("toll", toll)]);
        f.play().make_offer("job");
        f.play().decide("job", Outcome::Accept, 10);
        f.play().landed(StellarId(CAMP));
        let done = Scene::Completion("job".to_owned());
        assert_eq!(f.next(CAMP, &[], true), Some(done.clone()));
        f.play().complete("job");
        assert_eq!(
            f.next(CAMP, &[done], true),
            Some(Scene::Offer("toll".to_owned()))
        );
    }

    #[test]
    fn a_visit_shows_completions_then_offers_then_the_stellar_s_own_each_once() {
        let at_camp = |knot: &str| StoryMission {
            requires: requiring(&["go"]),
            ..offered_at(CAMP, FILE, knot)
        };
        let mut f = Fixture::new(&[
            ("job", job()),
            ("b-offer", at_camp("b")),
            ("a-offer", at_camp("a")),
            ("elsewhere", offered_at(DOCK, FILE, "x")),
        ]);
        f.flags.set("go");
        f.story.missions.insert(
            "job".to_owned(),
            MissionState::Active {
                cargo: 0,
                met: true,
            },
        );
        let mut shown = Vec::new();
        while let Some(scene) = f.next(CAMP, &shown, true) {
            shown.push(scene);
        }
        assert_eq!(
            shown,
            [
                Scene::Completion("job".to_owned()),
                Scene::Offer("a-offer".to_owned()),
                Scene::Offer("b-offer".to_owned()),
                Scene::Stellar(StellarId(CAMP)),
            ]
        );
        assert_eq!(
            f.next(CAMP, &[], false),
            Some(Scene::Completion("job".to_owned())),
            "a new visit starts again"
        );
        assert_eq!(f.next(CAMP, &shown[..3], false), None, "none of its own");
    }

    #[test]
    fn an_offer_whose_requires_fail_is_not_a_scene() {
        let mut f = Fixture::new(&[(
            "job",
            StoryMission {
                requires: requiring(&["go"]),
                ..job()
            },
        )]);
        assert_eq!(
            f.next(DOCK, &[], true),
            Some(Scene::Stellar(StellarId(DOCK)))
        );
        f.flags.set("go");
        assert_eq!(f.next(DOCK, &[], true), Some(offer()));
        f.story
            .missions
            .insert("job".to_owned(), MissionState::Done);
        assert_eq!(f.next(DOCK, &[], false), None, "done: not offered again");
    }

    #[test]
    fn day_passed_counts_the_day_of_the_game() {
        let mut f = Fixture::new(&[]);
        assert!(f.play().day_passed());
        assert!(f.play().day_passed());
        assert_eq!(f.story.day, 2);
        f.story.day = u32::MAX;
        f.play().day_passed();
        assert_eq!(f.story.day, u32::MAX);
    }

    fn line(text: &str, tags: &[&str]) -> Line {
        Line {
            text: text.to_owned(),
            tags: tags.iter().map(|&tag| tag.to_owned()).collect(),
        }
    }

    #[test]
    fn the_outcome_is_the_first_outcome_tag() {
        assert_eq!(outcome_of(&[]), None);
        assert_eq!(
            outcome_of(&[line("Hi.", &["speaker: clerk"])]),
            None,
            "another key"
        );
        assert_eq!(
            outcome_of(&[
                line("Hi.", &["speaker: clerk"]),
                line("Bay two.", &["mood: glad", "outcome: accept"]),
                line("Bye.", &["outcome: decline"]),
            ]),
            Some(Outcome::Accept)
        );
        assert_eq!(
            outcome_of(&[line("Later.", &["outcome:defer"])]),
            Some(Outcome::Defer)
        );
        assert_eq!(
            outcome_of(&[line("Bye.", &["  outcome :  decline "])]),
            Some(Outcome::Decline)
        );
    }

    #[test]
    fn an_outcome_tag_naming_no_outcome_is_passed_over() {
        assert_eq!(
            outcome_of(&[
                line("Hmm.", &["outcome: later"]),
                line("No.", &["outcome: decline"]),
            ]),
            Some(Outcome::Decline)
        );
        assert_eq!(outcome_of(&[line("Hmm.", &["outcome: later"])]), None);
        assert_eq!(outcome_of(&[line("Hmm.", &["outcome"])]), None);
    }
}
