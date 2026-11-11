//! The story a spaceport's bar hosts under
//! [`RuleKey::BarStory`]'s other reading, the new game's: the landed
//! stellar's storyline offers that hold there and its own conversation,
//! which the player opens from the bar, and the news heard in the system,
//! which its News lists in place of the drawn news.
//!
//! - **The listing.** [`Session::bar_story`] lists the scenes
//!   ([`Play::bar_scenes`]: the offers that hold there, by mission id,
//!   then the stellar's own conversation), each offer by its title, with
//!   the news [`Session::heard_news`] lists for the pilot's system. It is
//!   read when asked
//!   ([`ScreenAsk::BarStory`](crate::flight_frame::ScreenAsk::BarStory)),
//!   as the bar opens and after each conversation opened from it closes,
//!   so it holds no state: an offer decided leaves it at the next ask, and
//!   one deferred stays.
//! - **Opening.**
//!   [`Converse::OpenListed`](crate::flight_frame::Converse::OpenListed)
//!   opens the scene at its index of the listing as it is now
//!   ([`Session::bar_scene`], then `Port::open_listed_scene`), as
//!   [`Converse::Open`](crate::flight_frame::Converse::Open) opens the one
//!   it picks (see the `storylines` module): the landing settled first, an
//!   offer's first opening applying its `on_offer`, an outcome deciding it.
//!   The scene counts shown this visit, so the next `Converse::Open` after
//!   it closes passes over it.
//! - **Under the engine's reading** the bar is the original's
//!   (`_DoBarDialog` @0x48ef5): there is no listing, and opening from it is
//!   refused. Under either reading the scenes still open by themselves at
//!   landing, and the bar's `mïsn` offers (`_OfferMissionFromPort`
//!   @0xa369e) are unchanged.
//!
//! The listing reads the [`Chart`](super::Chart) (the bar) and the
//! pilot's story, so it lives on [`Session`]. The list is read as the bar
//! opens and after each close, before any frame draws it, so no ordering
//! boundary is crossed.

use crate::catalog::StellarId;
use crate::flight_frame::{BarScene, BarStory};
use crate::rulebook::{RuleKey, RuleSource};
use crate::storylines::lifecycle::{Play, Scene};

use super::Session;
use super::storylines::scene_place;

impl Session {
    /// The story the landed stellar's bar hosts (see the module docs):
    /// `None` under [`RuleKey::BarStory`]'s engine reading, in flight, or
    /// at a stellar with no bar; else its listing, even an empty one. It
    /// takes `&mut self` only as the story layer's reads do, and changes
    /// nothing.
    pub fn bar_story(&mut self) -> Option<BarStory> {
        let scenes = self
            .listed_scenes()?
            .into_iter()
            .map(|scene| match scene {
                Scene::Offer(mission) => {
                    let record = self.statics.storylines.missions.get(&mission);
                    BarScene::Offer {
                        title: record.map(|r| r.title.clone()).unwrap_or_default(),
                        giver: record
                            .and_then(|r| r.offer.as_ref())
                            .and_then(|offer| offer.giver.clone()),
                        mission,
                    }
                }
                // A bar never lists a completion (`Play::bar_scenes`).
                Scene::Stellar(_) | Scene::Completion(_) => BarScene::Stellar,
            })
            .collect();
        Some(BarStory {
            scenes,
            news: self.heard_news(self.pilot.system),
        })
    }

    /// The scene at `index` of the landed stellar's bar's listing now
    /// ([`Session::bar_story`]), or `None` when there is none.
    pub(super) fn bar_scene(&mut self, index: usize) -> Option<Scene> {
        self.listed_scenes()?.into_iter().nth(index)
    }

    /// The scenes the landed stellar's bar lists now
    /// ([`Play::bar_scenes`]), each one whose story is held, or `None`
    /// when the bar hosts no story (see [`Session::bar_story`]).
    fn listed_scenes(&mut self) -> Option<Vec<Scene>> {
        let stellar = self.story_bar()?;
        let has_own = self.statics.stellar_dialogs.place(stellar).is_some();
        let scenes = Play {
            storylines: &self.statics.storylines,
            flags: &mut self.pilot.flags,
            story: &mut self.pilot.story,
            outfits: &mut self.out.story_outfits,
        }
        .bar_scenes(stellar, has_own);
        Some(
            scenes
                .into_iter()
                .filter(|scene| scene_place(&self.statics, scene).is_some())
                .collect(),
        )
    }

    /// The landed stellar, when its bar hosts the story: under
    /// [`RuleKey::BarStory`]'s other reading, landed at a stellar with a
    /// bar.
    fn story_bar(&self) -> Option<StellarId> {
        if self.setup.rules.source_for(RuleKey::BarStory) == RuleSource::Engine {
            return None;
        }
        self.chart.bar_site(self.mode.landed()).map(|site| site.id)
    }
}

#[cfg(test)]
mod tests {
    use nova_sim_catalog::storylines::{MissionGiver, StoryEffects, StoryMission};

    use super::*;
    use crate::catalog::MissionRecord;
    use crate::chance::NeverFires;
    use crate::flight_frame::{Answer, Converse};
    use crate::landing::StellarFlags;
    use crate::mission::screens::BAR;
    use crate::rulebook::Rulebook;
    use crate::session::mission_screens::tests::{docked, screens, to_mars};
    use crate::session::storylines::tests::{
        converse, flying, job, land, lines, talking, visit_again,
    };
    use crate::storylines::state::MissionState;
    use crate::testkit::FakePilotCatalog;
    use crate::{MissionId, VenueOffer};

    /// The new game's reading of the key.
    fn hosting() -> Rulebook {
        Rulebook::default().with_override(RuleKey::BarStory, RuleSource::Bible)
    }

    /// The dock job, titled, its offer releasing the news `word`.
    fn titled() -> StoryMission {
        StoryMission {
            title: "Hull plate".to_owned(),
            on_offer: StoryEffects {
                news: vec!["word".to_owned()],
                ..StoryEffects::default()
            },
            ..job(10)
        }
    }

    /// [`talking`] with the titled job, the dock a bar.
    fn bar_catalog() -> FakePilotCatalog {
        let mut catalog = talking(&[("job", titled())]);
        catalog.sites[0].1[0].flags |= StellarFlags::BAR;
        catalog
    }

    /// A session of `catalog`'s under `rules`, landed at the dock.
    fn landed_under(catalog: &FakePilotCatalog, rules: Rulebook) -> Session {
        let mut session = flying(catalog).with_rules(rules);
        land(&mut session, catalog);
        session
    }

    /// The titled job's listing: it names no giver.
    fn offer() -> BarScene {
        BarScene::Offer {
            mission: "job".to_owned(),
            title: "Hull plate".to_owned(),
            giver: None,
        }
    }

    fn scenes(session: &mut Session) -> Vec<BarScene> {
        session.bar_story().expect("the bar hosts the story").scenes
    }

    fn state(session: &Session) -> Option<MissionState> {
        session.pilot().story.mission("job")
    }

    #[test]
    fn under_the_engine_s_reading_the_bar_hosts_no_story_and_opens_nothing() {
        let catalog = bar_catalog();
        let mut session = landed_under(&catalog, Rulebook::default());
        assert_eq!(session.bar_story(), None);
        assert_eq!(
            converse(&mut session, Converse::OpenListed(0)),
            Answer::Refused
        );
        assert_eq!(state(&session), None, "nothing offered");
        assert_eq!(
            lines(&converse(&mut session, Converse::Open)),
            ["A job?"],
            "the landing still opens it"
        );
    }

    #[test]
    fn the_bar_lists_the_offer_by_its_title_then_the_stellar_s_own() {
        let catalog = bar_catalog();
        let mut session = landed_under(&catalog, hosting());
        assert_eq!(scenes(&mut session), [offer(), BarScene::Stellar]);
    }

    #[test]
    fn the_bar_lists_an_offer_with_the_giver_its_record_names() {
        let giver = MissionGiver {
            name: "Ode".to_owned(),
            role: "dockmaster".to_owned(),
        };
        let mut job = titled();
        job.offer.as_mut().expect("offered").giver = Some(giver.clone());
        let mut catalog = talking(&[("job", job)]);
        catalog.sites[0].1[0].flags |= StellarFlags::BAR;
        let mut session = landed_under(&catalog, hosting());
        assert_eq!(
            scenes(&mut session),
            [
                BarScene::Offer {
                    mission: "job".to_owned(),
                    title: "Hull plate".to_owned(),
                    giver: Some(giver),
                },
                BarScene::Stellar
            ]
        );
    }

    #[test]
    fn opening_from_the_bar_shows_each_scene_s_first_page() {
        let catalog = bar_catalog();
        let mut session = landed_under(&catalog, hosting());
        assert_eq!(
            lines(&converse(&mut session, Converse::OpenListed(0))),
            ["A job?"]
        );
        assert_eq!(converse(&mut session, Converse::Close), Answer::Done);
        assert_eq!(
            lines(&converse(&mut session, Converse::OpenListed(1))),
            ["Hello."]
        );
    }

    #[test]
    fn an_unseen_offer_opened_from_the_bar_is_offered_and_due_a_save_but_not_again() {
        let catalog = bar_catalog();
        let mut session = landed_under(&catalog, hosting());
        session.out.save_due = false;
        converse(&mut session, Converse::OpenListed(0));
        assert_eq!(state(&session), Some(MissionState::Offered));
        assert!(session.out.save_due, "its first opening");
        converse(&mut session, Converse::Close);
        session.out.save_due = false;
        assert_eq!(
            lines(&converse(&mut session, Converse::OpenListed(0))),
            ["A job?"]
        );
        assert!(!session.out.save_due, "opened again");
    }

    #[test]
    fn an_outcome_reached_from_the_bar_decides_the_offer_and_it_leaves_the_list() {
        let catalog = bar_catalog();
        let mut session = landed_under(&catalog, hosting());
        converse(&mut session, Converse::OpenListed(0));
        assert_eq!(
            lines(&converse(&mut session, Converse::Choose(0))),
            ["Bay two."]
        );
        assert_eq!(
            state(&session),
            Some(MissionState::Active {
                cargo: 10,
                met: false
            })
        );
        converse(&mut session, Converse::Close);
        assert_eq!(scenes(&mut session), [BarScene::Stellar]);
    }

    #[test]
    fn a_defer_from_the_bar_leaves_the_offer_listed() {
        let catalog = bar_catalog();
        let mut session = landed_under(&catalog, hosting());
        converse(&mut session, Converse::OpenListed(0));
        converse(&mut session, Converse::Choose(2));
        converse(&mut session, Converse::Close);
        assert_eq!(state(&session), Some(MissionState::Offered));
        assert_eq!(scenes(&mut session), [offer(), BarScene::Stellar]);
        assert_eq!(
            lines(&converse(&mut session, Converse::OpenListed(0))),
            ["A job?"]
        );
    }

    #[test]
    fn past_the_list_in_flight_or_without_a_bar_nothing_is_listed_or_opened() {
        let catalog = bar_catalog();
        let mut session = landed_under(&catalog, hosting());
        assert_eq!(
            converse(&mut session, Converse::OpenListed(2)),
            Answer::Refused
        );

        let mut session = flying(&catalog).with_rules(hosting());
        assert_eq!(session.bar_story(), None);
        assert_eq!(
            converse(&mut session, Converse::OpenListed(0)),
            Answer::Refused
        );

        let barless = talking(&[("job", titled())]);
        let mut session = landed_under(&barless, hosting());
        assert_eq!(session.bar_story(), None);
        assert_eq!(
            converse(&mut session, Converse::OpenListed(0)),
            Answer::Refused
        );
        assert_eq!(state(&session), None);
    }

    #[test]
    fn the_bar_lists_the_news_heard_in_the_system() {
        let catalog = bar_catalog();
        let mut session = landed_under(&catalog, hosting());
        assert_eq!(session.bar_story().expect("hosted").news, []);
        converse(&mut session, Converse::OpenListed(0));
        let heard = session.heard_news(session.pilot().system);
        assert_eq!(heard.len(), 1, "the offer released its news");
        assert_eq!(session.bar_story().expect("hosted").news, heard);
    }

    #[test]
    fn the_bar_s_misn_offer_is_offered_under_either_reading() {
        let catalog = screens(
            vec![MissionRecord {
                avail_loc: BAR,
                ..to_mars(141)
            }],
            Vec::new(),
        );
        for rules in [Rulebook::default(), hosting()] {
            let mut session = docked(&catalog).with_rules(rules);
            assert!(
                matches!(
                    session.next_offer(BAR, &mut NeverFires),
                    Some(VenueOffer::Prompt(prompt)) if prompt.mission == MissionId(141)
                ),
                "{rules:?}"
            );
        }
    }

    #[test]
    fn the_landing_still_opens_the_offer_and_a_deferred_one_at_a_later_landing() {
        let catalog = bar_catalog();
        let mut session = landed_under(&catalog, hosting());
        assert_eq!(lines(&converse(&mut session, Converse::Open)), ["A job?"]);
        converse(&mut session, Converse::Choose(2));
        converse(&mut session, Converse::Close);
        visit_again(&mut session, &catalog);
        assert_eq!(lines(&converse(&mut session, Converse::Open)), ["A job?"]);
    }

    #[test]
    fn a_scene_opened_from_the_bar_is_not_opened_again_by_the_landing() {
        let catalog = bar_catalog();
        let mut session = landed_under(&catalog, hosting());
        converse(&mut session, Converse::OpenListed(0));
        converse(&mut session, Converse::Close);
        assert_eq!(
            lines(&converse(&mut session, Converse::Open)),
            ["Hello."],
            "the offer is passed over"
        );
        converse(&mut session, Converse::Close);
        assert_eq!(converse(&mut session, Converse::Open), Answer::Refused);
        converse(&mut session, Converse::OpenListed(1));
        converse(&mut session, Converse::Close);
        assert_eq!(converse(&mut session, Converse::Open), Answer::Refused);
    }
}
