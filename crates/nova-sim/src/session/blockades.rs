//! The new game's blockades: the bridge between a `take_point` mission,
//! which the storylines model, and its blockade, the hidden stock `mïsn`
//! whose special ships are the holder's guards at the point
//! (`nova_data::new_game`'s `blockades` maps it, and the store's
//! `take_point` objective names it).
//!
//! The stock mission-ship machinery runs the guards: it sets them up at
//! the point's gate whenever the point's system is set up, sends them at
//! the player, brings back those left, counts their fates in the pilot's
//! classic slot and checks the chase-off goal (see the `mission_ships`
//! module). This module only keeps the two mission models in step:
//!
//! - **Start.** While a `take_point` mission is active and unmet, a
//!   classic slot holds its blockade: [`Session::sync_blockades`] starts
//!   one no slot holds as an `S` does ([`Session::script_start`]), never
//!   offered, so its guards come at the next setup of the point's system
//!   (accepted while landed in it, the take-off). With every slot taken
//!   nothing starts, and the next settle tries again.
//! - **Bridge.** The blockade's goals-met moment, every guard destroyed or
//!   chased off, meets the mission ([`Session::blockade_broken`],
//!   `nova_sim_story`'s `Play::beat_holder`): one with no completion
//!   succeeds at once and its `on_success` applies, a change makes a save
//!   due, and the storylines settle, the points synced, so a new owner is
//!   stamped on the gate before the next frame. The moment comes inside
//!   the settling of the last guard's fate, so nothing waits for a later
//!   tick.
//! - **End.** A blockade whose mission isn't active and unmet (met, done,
//!   or never accepted by a save the data disagrees with) is ended as
//!   `_AbortMission` ends one, with no `OnAbort` ([`Session::end_mission`]):
//!   the slot is freed and any guard left flies on unmarked.
//!
//! [`Session::sync_blockades`] runs at the end of
//! [`Session::settle_storylines`], so after every entry that plays the
//! storylines (`Converse::Open` and `Converse::Choose`, the `Storylines`
//! consumer's arm of the story dispatch, and the bridge itself), and as a
//! pilot is loaded ([`Session::load`]), so a save that disagrees with its
//! data is put right before the first frame. The stock game holds no
//! storylines, so nothing here does anything there.

use crate::catalog::MissionId;
use crate::chance::Chance;
use crate::storylines::lifecycle::Play;
use crate::storylines::state::MissionState;
use nova_sim_catalog::storylines::{MissionObjective, Storylines};

use super::Session;

/// Each `take_point` mission of `storylines` with a blockade: its id and
/// its blockade's `mïsn`.
fn blockades(storylines: &Storylines) -> impl Iterator<Item = (&String, MissionId)> {
    storylines
        .missions
        .iter()
        .filter_map(|(id, mission)| match mission.objective {
            Some(MissionObjective::TakePoint {
                blockade: Some(blockade),
                ..
            }) => Some((id, blockade)),
            _ => None,
        })
}

impl Session {
    /// Puts the blockades in step with their missions (see the module
    /// docs), drawing on `chance`: each mission active and unmet whose
    /// blockade no slot holds starts it, then each slot holding the
    /// blockade of a mission that isn't ends it.
    pub(super) fn sync_blockades(&mut self, chance: &mut dyn Chance) {
        let mut start = Vec::new();
        let mut end = Vec::new();
        for (id, blockade) in blockades(&self.statics.storylines) {
            let held = self.pilot.missions.holding(blockade);
            let holds = matches!(
                self.pilot.story.mission(id),
                Some(MissionState::Active { met: false, .. })
            );
            if holds {
                if held.is_empty() {
                    start.push(blockade);
                }
            } else {
                end.extend(held.into_iter().map(|slot| (slot, blockade)));
            }
        }
        for blockade in start {
            self.script_start(blockade, chance);
        }
        for (slot, blockade) in end {
            if let Some(record) = self.statics.mission_record(blockade).cloned() {
                self.end_mission(slot, &record, false, chance);
            }
        }
    }

    /// The goals-met moment of `mission`, drawing on `chance`: when it is
    /// a `take_point` mission's blockade, the holder is beaten (see the
    /// module docs), a change making a save due, and the storylines
    /// settle, which ends the blockade. Nothing for any other mission.
    pub(super) fn blockade_broken(&mut self, mission: MissionId, chance: &mut dyn Chance) {
        let Some(id) = blockades(&self.statics.storylines)
            .find_map(|(id, blockade)| (blockade == mission).then(|| id.clone()))
        else {
            return;
        };
        self.out.save_due |= Play {
            storylines: &self.statics.storylines,
            flags: &mut self.pilot.flags,
            story: &mut self.pilot.story,
            outfits: &mut self.out.story_outfits,
        }
        .beat_holder(&id);
        self.settle_storylines(chance);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use nova_sim_catalog::fixture::storylines::{offered_at, setting};
    use nova_sim_catalog::storylines::{PointAt, PointStability, PointState, StoryEffects};

    use super::*;
    use crate::catalog::{GovtId, MissionRecord, MissionShip, StellarId};
    use crate::chance::NeverFires;
    use crate::mission::Venue;
    use crate::pilot::Pilot;
    use crate::session::mission_ships::tests::{destroy, marked, settle, ships_catalog, take_off};
    use crate::testkit::{FakePilotCatalog, land_now};

    /// The blockade, mission 140.
    const BLOCKADE: MissionId = MissionId(140);
    /// The gate of the point the road takes: stellar 129, in system 130.
    const GATE: StellarId = StellarId(129);
    /// The guild's government, which the road's success gives the point.
    const GUILD: GovtId = GovtId(129);

    const ACTIVE: Option<MissionState> = Some(MissionState::Active {
        cargo: 0,
        met: false,
    });

    /// Two guards of düde 150 in system 130 at its first nav slot (128,
    /// where the player takes off), chased off to be beaten, attacking the
    /// player, hidden, never offered, going nowhere.
    fn guards() -> MissionShip {
        MissionShip {
            count: 2,
            goal: 6,
            syst: 130,
            dude: Some(crate::catalog::DudeId(150)),
            behav: 0,
            start: -1,
            name_id: None,
            subtitle: None,
        }
    }

    /// [`ships_catalog`] with mission 140 the blockade of `road`, a
    /// `take_point` on `a/b` (gate 129, held by the wardens) offered at
    /// 128 with no completion, its success setting `taken` and giving the
    /// point to the guild.
    fn blockading() -> FakePilotCatalog {
        let mut catalog = ships_catalog(guards());
        catalog.missions[0] = MissionRecord {
            avail_random: 0,
            flags: 0x0400,
            can_abort: 0,
            ..catalog.missions[0].clone()
        };
        catalog.storylines.missions.insert(
            "road".to_owned(),
            nova_sim_catalog::storylines::StoryMission {
                objective: Some(MissionObjective::TakePoint {
                    point: "a/b".to_owned(),
                    held_by: "warden".to_owned(),
                    blockade: Some(BLOCKADE),
                }),
                on_success: StoryEffects {
                    points: vec![PointState {
                        point: "a/b".to_owned(),
                        owner: Some("guild".to_owned()),
                        stability: None,
                    }],
                    ..setting(&["taken"])
                },
                ..offered_at(128, "dialog/road.ink", "offer")
            },
        );
        catalog.storylines.points = BTreeMap::from([(
            "a/b".to_owned(),
            PointAt {
                gate: GATE,
                owner: Some("warden".to_owned()),
                stability: PointStability::Stable,
            },
        )]);
        catalog
            .storylines
            .factions
            .insert("guild".to_owned(), GUILD);
        catalog
    }

    /// A session of `catalog` docked at 128 with `road` in `state`.
    fn docked_with(catalog: &FakePilotCatalog, state: Option<MissionState>) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        settle(&mut session, catalog);
        if let Some(state) = state {
            session
                .pilot
                .story
                .missions
                .insert("road".to_owned(), state);
        }
        session
    }

    /// The slots holding the blockade.
    fn holding(session: &Session) -> Vec<u8> {
        session.pilot.missions.holding(BLOCKADE)
    }

    // Start and end.

    #[test]
    fn an_active_take_point_starts_its_blockade_as_the_storylines_settle() {
        let catalog = blockading();
        let mut session = docked_with(&catalog, ACTIVE);
        assert_eq!(holding(&session), Vec::<u8>::new());
        session.out.save_due = false;
        session.settle_storylines(&mut NeverFires);
        assert_eq!(holding(&session), [0]);
        assert!(session.out.save_due, "an accept makes a save due");
    }

    #[test]
    fn a_second_settle_starts_no_second_blockade() {
        let catalog = blockading();
        let mut session = docked_with(&catalog, ACTIVE);
        session.settle_storylines(&mut NeverFires);
        session.settle_storylines(&mut NeverFires);
        assert_eq!(holding(&session), [0]);
    }

    #[test]
    fn the_blockade_is_never_offered_at_a_landing() {
        let catalog = blockading();
        let session = docked_with(&catalog, None);
        for venue in [Venue::Computer, Venue::Outside] {
            assert!(
                session
                    .mission_offers(venue)
                    .iter()
                    .all(|offer| offer.mission != BLOCKADE),
                "{venue:?}"
            );
        }
        assert_eq!(
            holding(&session),
            Vec::<u8>::new(),
            "nor started unaccepted"
        );
    }

    #[test]
    fn a_met_or_done_take_point_s_blockade_is_ended() {
        let catalog = blockading();
        for state in [
            MissionState::Active {
                cargo: 0,
                met: true,
            },
            MissionState::Done,
        ] {
            let mut session = docked_with(&catalog, ACTIVE);
            session.settle_storylines(&mut NeverFires);
            assert_eq!(holding(&session), [0]);
            session
                .pilot
                .story
                .missions
                .insert("road".to_owned(), state);
            session.settle_storylines(&mut NeverFires);
            assert_eq!(holding(&session), Vec::<u8>::new(), "{state:?}");
        }
    }

    #[test]
    fn a_loaded_pilot_s_missing_blockade_is_started() {
        let catalog = blockading();
        let mut pilot = Pilot::new(&catalog, "Ada").expect("a pilot");
        pilot
            .story
            .missions
            .insert("road".to_owned(), ACTIVE.expect("a state"));
        let session = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(session.pilot.missions.holding(BLOCKADE), [0]);
    }

    #[test]
    fn a_catalog_with_no_storylines_starts_nothing() {
        let mut catalog = blockading();
        catalog.storylines = nova_sim_catalog::storylines::Storylines::default();
        let mut session = docked_with(&catalog, ACTIVE);
        session.settle_storylines(&mut NeverFires);
        assert_eq!(holding(&session), Vec::<u8>::new());
    }

    // The bridge.

    /// A session of [`blockading`] that has accepted the road at 128 and
    /// taken off among the guards.
    fn among_the_guards(catalog: &FakePilotCatalog) -> Session {
        let mut session = docked_with(catalog, ACTIVE);
        session.settle_storylines(&mut NeverFires);
        take_off(&mut session, catalog);
        assert_eq!(marked(&session), [0, 0], "the two guards");
        session
    }

    #[test]
    fn beating_the_guards_meets_the_road_succeeds_it_and_frees_the_slot() {
        let catalog = blockading();
        let mut session = among_the_guards(&catalog);
        destroy(&mut session, 0);
        settle(&mut session, &catalog);
        assert_eq!(
            session.pilot.story.mission("road"),
            ACTIVE,
            "one guard is left"
        );
        session.out.save_due = false;
        destroy(&mut session, 0);
        settle(&mut session, &catalog);
        assert_eq!(
            session.pilot.story.mission("road"),
            Some(MissionState::Done)
        );
        assert!(session.pilot.flags.is_set("taken"));
        assert_eq!(
            session.chart.galaxy.gate_govt(GATE),
            Some(GUILD),
            "the owner stamped on the gate"
        );
        assert!(session.out.save_due);
        assert_eq!(holding(&session), Vec::<u8>::new(), "the slot is freed");
    }

    #[test]
    fn the_fixture_s_destroy_takes_a_guard_out_and_counts_it() {
        let catalog = blockading();
        let mut session = among_the_guards(&catalog);
        let guard = session.npcs()[0].id;
        crate::fixture::destroy(&mut session, guard);
        assert_eq!(marked(&session), [0], "one guard left");
        let left = session.pilot.missions.get(0).expect("held").ships.left;
        assert_eq!(left, 1);
        crate::fixture::destroy(&mut session, guard);
        assert_eq!(marked(&session), [0], "one gone already: nothing");
    }

    #[test]
    fn another_mission_s_goals_met_moment_touches_no_storyline() {
        let catalog = blockading();
        let mut session = among_the_guards(&catalog);
        session.blockade_broken(MissionId(141), &mut NeverFires);
        assert_eq!(session.pilot.story.mission("road"), ACTIVE);
        assert_eq!(holding(&session), [0]);
    }
}
