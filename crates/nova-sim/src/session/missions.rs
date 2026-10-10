//! The session's side of mission offers (see the
//! [`mission`](crate::mission) module): the visit's offers, rolled as the
//! player lands and kept for the visit, and the facts they are rolled
//! from.
//!
//! The roll is no loop of its own: [`Session::fly`] registers it as the
//! missions' first `Landed` subscription, [`ROLL`], whose consumer effect
//! rolls the offers at the stellar landed on (`_HandlePlayerDockRequest`
//! @0x66d2b-0x66d77), before any mission slot's landing hooks (see the
//! [`story`](crate::story) module's "Dispatch"). A pilot flown docked has
//! `Landed` raised for its stellar, after its `Explored`, as a load
//! forgets the stellar rolled at (@0x75ef5) and opens the spaceport, so
//! the first settling rolls its offers afresh.
//!
//! What forgets the stellar rolled at, so that the next landing rolls the
//! offers afresh, is [`Session::forget_offers`]: a jump's arrival
//! (@0x6bfce) and the developer tools' relocation through the entry
//! table, and a mission's abort, failure and success through missions
//! phase 2 (@0x99bcf, @0x99db5, @0xa03ef, @0xa0691). A landed `M` or `N`
//! empties the offers and keeps the stellar (@0x1584b, @0x15949). None of
//! it is saved.

use super::Session;
use crate::catalog::{MissionId, StellarId};
use crate::chance::Chance;
use crate::control::Script;
use crate::mission::{self, AvailFacts, Offer, Player, Refusal, Venue, Visit, World, stellars_of};
use crate::rules::Verdict;
use crate::story::{Consumer, EventFilter, StoryEventKind, Subscription, SubscriptionId};
use crate::wares;

/// The offer roll's subscription: the missions' first, before every
/// mission slot's (slots 0-15) landing hooks.
pub const ROLL: SubscriptionId = SubscriptionId {
    consumer: Consumer::Missions,
    record: -1,
    hook: 0,
};

impl Session {
    /// Registers the offer roll ([`ROLL`]) on every landing, with a blank
    /// guard and no script.
    pub(super) fn register_offer_roll(&mut self) {
        let roll = Subscription::new(
            ROLL,
            EventFilter::Any(StoryEventKind::Landed),
            Script::default(),
        );
        let registered = self.register_story(roll);
        debug_assert!(registered.is_ok(), "a fresh session registers it once");
    }

    /// Rolls the offers at `stellar`, the stellar landed on, drawing on
    /// `chance`, unless they were rolled there last (see the
    /// [`mission`](crate::mission) module).
    pub(super) fn roll_offers(&mut self, stellar: StellarId, chance: &mut dyn Chance) {
        let mut offers = std::mem::take(&mut self.offers);
        self.at_visit(stellar, |visit| {
            offers.roll(&self.missions, visit, chance);
        });
        self.offers = offers;
    }

    /// Forgets the stellar the offers were rolled at, so the next landing
    /// rolls them afresh: the hook a jump's arrival calls, and missions
    /// phase 2's abort, failure and success (@0x99bcf, @0x99db5,
    /// @0xa03ef, @0xa0691).
    pub(crate) fn forget_offers(&mut self) {
        self.offers.forget();
    }

    /// Empties the offers, keeping the stellar they were rolled at.
    pub(super) fn empty_offers(&mut self) {
        self.offers.empty();
    }

    /// The offers at `venue` of the visit under way, in the order rolled:
    /// none before the first landing settles.
    #[must_use]
    pub fn mission_offers(&self, venue: Venue) -> &[Offer] {
        self.offers.at(venue)
    }

    /// Why `mission` is not offered at the stellar the ship is docked at,
    /// at its own venue: every check that refuses, by name, in the
    /// checks' order, the visit's rolls read and nothing drawn
    /// ([`Checks::all_refusals`](crate::rules::Checks::all_refusals)).
    /// Empty when it is offered there; `None` when the ship is not docked
    /// or there is no such `mïsn`.
    #[must_use]
    pub fn offer_refusals(
        &self,
        mission: MissionId,
    ) -> Option<Vec<(&'static str, Verdict<Refusal>)>> {
        let record = self.missions.iter().find(|record| record.id == mission)?;
        let stellar = self.landed?;
        self.at_visit(stellar, |visit| {
            let facts = AvailFacts {
                visit,
                mission: record,
                venue: Venue::of(record.avail_loc),
                rolls: self.offers.table(),
            };
            mission::availability()
                .all_refusals(&facts)
                .into_iter()
                .map(|(descriptor, verdict)| (descriptor.name(), verdict))
                .collect()
        })
    }

    /// The room a mission's cargo has, in tons, as `_TotalMissionCargoSpace`
    /// (@0xcffb) reckons it: the ship's own holds, with its cargo outfits,
    /// less the cargo held that the trader escorts' holds do not take; below
    /// 0 when it holds more. Mission cargo, which rides in the ship alone,
    /// is missions phase 5's.
    fn mission_cargo_space(&self) -> i64 {
        let own = i64::from(self.stats.capacity);
        let escorts = i64::from(self.capacity()) - own;
        let held: i64 = self.pilot.cargo.values().map(|&tons| i64::from(tons)).sum();
        own - (held - escorts).max(0)
    }

    /// `f` of the visit to `stellar`, one of the stellars shown: `None`
    /// when it is not shown.
    fn at_visit<T>(&self, stellar: StellarId, f: impl FnOnce(&Visit) -> T) -> Option<T> {
        let docked = *self.sites.iter().find(|site| site.id == stellar)?;
        let map = self.galaxy.map();
        let stellars = stellars_of(&self.galaxy, &self.listed_stellars);
        let ship = self.ship_record(self.pilot.ship);
        let player = Player {
            record: map
                .govt(self.shown)
                .map_or(0, |govt| self.pilot.legal_record(govt)),
            rating: self.pilot.combat_rating,
            ship: self.pilot.ship,
            inherent_ai: ship.map_or(0, |ship| ship.inherent_ai),
            inherent_govt: ship.and_then(|ship| ship.inherent_govt),
            combat_govt: ship.and_then(|ship| ship.combat_govt),
            contributed: wares::contributed(
                self.fields.contribute,
                &self.pilot.outfits,
                &self.outfits,
            ),
            fuel: self.pilot.reserves.fuel.now,
            free_cargo: self.mission_cargo_space(),
            cash: self.pilot.cash,
        };
        let visit = Visit {
            docked,
            system: self.shown,
            world: World {
                stellars: &stellars,
                map,
                govts: &self.govts,
            },
            player,
            active: &[],
            gate: self.gate(),
            rules: &self.rules,
            today: self.pilot.date,
        };
        Some(f(&visit))
    }

    /// The visit's roll table.
    #[cfg(test)]
    pub(crate) fn offer_rolls(&self) -> &mission::RollTable {
        self.offers.table()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{MissionRecord, StellarId, SystemId};
    use crate::chance::NeverFires;
    use crate::control::SetExpr;
    use crate::geometry::Vec2;
    use crate::mission::{ROLLED, Stop};
    use crate::pilot::Pilot;
    use crate::testkit::{Draws, FakePilotCatalog, catalog, jump, land_now, mission, planet};

    /// [`catalog`] with stellar 150 in system 132, two jumps from 130, 131
    /// linked back to 130, and missions 130 (to a random inhabited
    /// stellar), 131 (to 140) and 132 (`AvailRandom` 10), each at the
    /// mission computer.
    fn offering() -> FakePilotCatalog {
        let mut catalog = catalog();
        catalog.star_map[1].links.push(SystemId(130));
        catalog
            .sites
            .push((SystemId(132), vec![planet(150, 0.0, 0.0)]));
        let mut random = mission(130);
        random.travel_stel = -2;
        let mut fixed = mission(131);
        fixed.travel_stel = 140;
        let mut rare = mission(132);
        rare.avail_random = 10;
        catalog.missions = vec![random, fixed, rare];
        catalog
    }

    fn ids(offers: &[Offer]) -> Vec<i16> {
        offers.iter().map(|offer| offer.mission.0).collect()
    }

    /// A session of `catalog` landed on 128 and settled on `chance`.
    fn landed(catalog: &FakePilotCatalog, chance: &mut dyn Chance) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        let _ = session.settle_script(catalog, chance);
        session
    }

    /// Every roll's draw, then each of `more`.
    fn table_then(more: &[u32]) -> Vec<u32> {
        let mut asked = vec![100; ROLLED];
        asked.extend_from_slice(more);
        asked
    }

    #[test]
    fn landing_rolls_the_mission_computers_offers_as_the_landing_settles() {
        let catalog = offering();
        let mut session = Session::start(&catalog).expect("starts");
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert!(
            session.mission_offers(Venue::Computer).is_empty(),
            "in flight"
        );
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        assert!(
            session.mission_offers(Venue::Computer).is_empty(),
            "not settled"
        );
        let mut chance = Draws::of(&[]);
        let _ = session.settle_script(&catalog, &mut chance);
        assert_eq!(
            chance.asked,
            table_then(&[1]),
            "the table, then 130's stellar"
        );
        let offers = session.mission_offers(Venue::Computer);
        assert_eq!(ids(offers), [130, 131], "132's roll of 100 is above 10");
        let stop = |stellar, system| {
            Some(Stop {
                stellar: StellarId(stellar),
                system: Some(SystemId(system)),
            })
        };
        assert_eq!(offers[0].destination, stop(150, 132));
        assert_eq!(offers[1].destination, stop(140, 131));
        assert!(session.mission_offers(Venue::Outside).is_empty());
    }

    #[test]
    fn landing_again_on_the_same_stellar_keeps_the_offers_and_draws_nothing() {
        let catalog = offering();
        let mut session = landed(&catalog, &mut NeverFires);
        let offers = session.mission_offers(Venue::Computer).to_vec();
        assert_eq!(ids(&offers), [130, 131, 132], "every roll 1");
        assert_eq!(session.take_off(&mut NeverFires), Some(StellarId(128)));
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        let mut chance = Draws::of(&[]);
        let _ = session.settle_script(&catalog, &mut chance);
        assert!(chance.asked.is_empty(), "{:?}", chance.asked);
        assert_eq!(session.mission_offers(Venue::Computer), offers);
    }

    /// Takes `session` off and puts it at rest over 128 in 130.
    fn back_over_128(session: &mut Session) {
        let site = *session
            .sites
            .iter()
            .find(|site| site.id == StellarId(128))
            .expect("128 shown");
        session.player.position = site.position;
        session.player.velocity = Vec2::ZERO;
    }

    #[test]
    fn a_jumps_arrival_forgets_the_stellar_so_landing_back_rolls_afresh() {
        let catalog = offering();
        let mut session = landed(&catalog, &mut NeverFires);
        assert_eq!(session.take_off(&mut NeverFires), Some(StellarId(128)));
        let _ = session.settle_script(&catalog, &mut NeverFires);
        // From the centre, where `fly_out` heads straight out.
        session.player.position = Vec2::ZERO;
        assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(131)));
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert_eq!(
            ids(session.mission_offers(Venue::Computer)),
            [130, 131, 132],
            "kept until the next landing"
        );
        assert_eq!(jump(&mut session, &catalog, 130), Some(SystemId(130)));
        let _ = session.settle_script(&catalog, &mut NeverFires);
        back_over_128(&mut session);
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        let mut chance = Draws::of(&[]);
        let _ = session.settle_script(&catalog, &mut chance);
        assert_eq!(chance.asked, table_then(&[1]));
        assert_eq!(ids(session.mission_offers(Venue::Computer)), [130, 131]);
    }

    #[test]
    fn after_the_forget_hook_landing_again_on_the_same_stellar_rolls_afresh() {
        let catalog = offering();
        let mut session = landed(&catalog, &mut NeverFires);
        session.take_off(&mut NeverFires);
        session.forget_offers();
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        let mut chance = Draws::of(&[]);
        let _ = session.settle_script(&catalog, &mut chance);
        assert_eq!(chance.asked, table_then(&[1]));
        assert_eq!(session.offer_rolls().roll(MissionId(130)), Some(100));
    }

    #[test]
    fn a_landed_m_empties_the_offers_and_keeps_the_stellar() {
        let catalog = offering();
        let mut session = landed(&catalog, &mut NeverFires);
        session.run_set(&SetExpr::parse("M131").expect("parses"), &mut NeverFires);
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert_eq!(session.system(), SystemId(131));
        assert!(session.mission_offers(Venue::Computer).is_empty());
        assert_eq!(session.offers.rolled_at(), Some(StellarId(128)));
    }

    #[test]
    fn a_relocation_empties_the_offers_and_forgets_their_stellar() {
        let catalog = offering();
        let mut session = landed(&catalog, &mut NeverFires);
        session
            .relocate(&catalog, SystemId(131), StellarId(140))
            .expect("relocates");
        assert!(session.mission_offers(Venue::Computer).is_empty());
        assert_eq!(session.offers.rolled_at(), None);
    }

    #[test]
    fn a_pilot_flown_docked_has_its_offers_rolled_as_it_settles() {
        let catalog = offering();
        let mut pilot = Pilot::new(&catalog, "Ada").expect("starts");
        pilot.stellar = Some(StellarId(128));
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        assert!(session.mission_offers(Venue::Computer).is_empty());
        let mut chance = Draws::of(&[]);
        let _ = session.settle_script(&catalog, &mut chance);
        assert_eq!(chance.asked, table_then(&[1]));
        assert_eq!(ids(session.mission_offers(Venue::Computer)), [130, 131]);
    }

    #[test]
    fn flying_a_docked_pilot_raises_its_landing_after_its_exploration() {
        use crate::story::StoryEvent;
        let catalog = offering();
        let mut pilot = Pilot::new(&catalog, "Ada").expect("starts");
        pilot.stellar = Some(StellarId(128));
        let docked = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(
            docked.story_raised(),
            [
                StoryEvent::Explored {
                    system: SystemId(130)
                },
                StoryEvent::Landed {
                    stellar: StellarId(128)
                },
            ]
        );
        let in_flight = Session::start(&catalog).expect("starts");
        assert_eq!(
            in_flight.story_raised(),
            [StoryEvent::Explored {
                system: SystemId(130)
            }]
        );
    }

    #[test]
    fn the_roll_is_registered_and_runs_before_every_mission_slots_hooks() {
        let catalog = offering();
        let mut session = Session::start(&catalog).expect("starts");
        let slot = |record| SubscriptionId {
            consumer: Consumer::Missions,
            record,
            hook: 0,
        };
        assert!(ROLL < slot(0));
        assert!(ROLL > slot(-2));
        assert!(
            session.unregister_story(ROLL),
            "registered as the session starts"
        );
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert!(
            session.mission_offers(Venue::Computer).is_empty(),
            "unregistered, nothing rolls"
        );
    }

    #[test]
    fn the_offer_refusals_name_avail_random_and_draw_nothing() {
        let catalog = offering();
        let session = landed(&catalog, &mut Draws::of(&[]));
        assert_eq!(
            session.offer_refusals(MissionId(132)),
            Some(vec![(
                "AvailRandom",
                Verdict::Refuses(Refusal::AvailRandom)
            )])
        );
        assert_eq!(session.offer_refusals(MissionId(130)), Some(vec![]));
        assert_eq!(session.offer_refusals(MissionId(133)), None, "no such mïsn");
        let in_flight = Session::start(&catalog).expect("starts");
        assert_eq!(in_flight.offer_refusals(MissionId(130)), None);
    }

    #[test]
    fn a_missions_cargo_space_is_the_ships_own_less_the_cargo_the_escorts_do_not_take() {
        use crate::market::Good;
        use crate::pilot::Escort;
        let mut catalog = offering();
        catalog.ship_records = vec![crate::catalog::ShipRecord {
            inherent_ai: 1,
            ..crate::testkit::ship(
                129,
                crate::handling::ShipFields {
                    holds: 15,
                    ..crate::testkit::FAST
                },
            )
        }];
        catalog.missions = vec![MissionRecord {
            flags2: 0x0001,
            cargo_qty: 6,
            ..mission(140)
        }];
        let mut session = landed(&catalog, &mut NeverFires);
        session.pilot.escorts = vec![Escort {
            ship: crate::catalog::ShipId(129),
            reserves: crate::reserves::Reserves::default(),
            order: None,
            carried: false,
            wage: None,
            person: None,
        }];
        let refused = |session: &Session| {
            session
                .offer_refusals(MissionId(140))
                .expect("docked")
                .into_iter()
                .map(|(name, _)| name)
                .collect::<Vec<_>>()
        };
        let none: [&str; 0] = [];
        for (held, refusals) in [(0, &none[..]), (25, &none[..]), (30, &["cargo space"][..])] {
            session.pilot.cargo.insert(Good::Commodity(0), held);
            assert_eq!(
                refused(&session),
                refusals,
                "{held} tons held: 20 own, 15 the escort's"
            );
        }
        assert_eq!(session.mission_cargo_space(), 5);
        session.pilot.escorts.clear();
        session.pilot.cargo.insert(Good::Commodity(0), 14);
        assert_eq!(session.mission_cargo_space(), 6, "no escort");
        session.pilot.cargo.insert(Good::Commodity(0), 25);
        assert_eq!(session.mission_cargo_space(), -5, "over the holds");
    }

    #[test]
    fn the_offers_read_the_pilot_and_its_ship() {
        let mut catalog = offering();
        catalog.missions = vec![
            MissionRecord {
                avail_rating: 5,
                ..mission(140)
            },
            MissionRecord {
                avail_ship_type: 1128,
                ..mission(141)
            },
            MissionRecord {
                flags: 0x2000,
                ..mission(142)
            },
            MissionRecord {
                pay_val: -50100,
                ..mission(143)
            },
            MissionRecord {
                flags: 0x0008,
                ..mission(144)
            },
            MissionRecord {
                flags2: 0x0001,
                cargo_qty: 21,
                ..mission(145)
            },
            MissionRecord {
                require: 0x2,
                ..mission(146)
            },
            MissionRecord {
                avail_record: 3,
                ..mission(147)
            },
        ];
        let session = landed(&catalog, &mut NeverFires);
        let refused = |id| {
            session
                .offer_refusals(MissionId(id))
                .expect("docked")
                .into_iter()
                .map(|(name, _)| name)
                .collect::<Vec<_>>()
        };
        assert_eq!(refused(140), ["AvailRating"]);
        assert_eq!(refused(141), ["AvailShipType"], "flying 128");
        assert_eq!(refused(142), ["InherentAI"], "no ship record: AI 0");
        assert_eq!(refused(143), ["PayVal cash"]);
        assert_eq!(refused(144), [] as [&str; 0], "the tank holds 300");
        assert_eq!(refused(145), ["cargo space"], "20 tons");
        assert_eq!(refused(146), ["Require"], "the ship contributes 0x1");
        assert_eq!(refused(147), ["AvailRecord"]);
    }

    #[test]
    fn the_offers_take_a_pilot_and_ship_that_meet_them() {
        use crate::catalog::{GovtId, OutfitId, OutfitRecord, ShipRecord};
        let mut catalog = offering();
        // System 130 is govt 128's; the stellar docked at, 128, govt 129's.
        catalog.star_map[0].govt = Some(GovtId(128));
        catalog.sites[0].1[0].govt = Some(GovtId(129));
        catalog.ship_records = vec![ShipRecord {
            inherent_govt: Some(GovtId(130)),
            combat_govt: Some(GovtId(131)),
            ..crate::testkit::ship(128, crate::testkit::FAST)
        }];
        catalog.outfits = vec![OutfitRecord {
            contribute: 0x2,
            ..crate::testkit::outfit(200, &[])
        }];
        catalog.missions = vec![
            MissionRecord {
                avail_record: 3,
                ..mission(140)
            },
            MissionRecord {
                avail_rating: 5,
                ..mission(141)
            },
            MissionRecord {
                require: 0x2,
                ..mission(142)
            },
            MissionRecord {
                pay_val: -50100,
                ..mission(143)
            },
            MissionRecord {
                avail_ship_type: 2130,
                ..mission(144)
            },
            MissionRecord {
                avail_ship_type: 2131,
                ..mission(145)
            },
        ];
        let mut session = landed(&catalog, &mut NeverFires);
        session.pilot.set_legal_record(GovtId(128), 3);
        session.pilot.set_legal_record(GovtId(129), -5);
        session.pilot.combat_rating = 5;
        session.pilot.outfits.insert(OutfitId(200), 1);
        session.pilot.cash = 100;
        for (id, why) in [
            (
                140,
                "a record of 3 with the system's govt, not the stellar's",
            ),
            (141, "a rating of 5"),
            (142, "the outfit contributes 0x2"),
            (143, "100 credits: -50000 less -50100"),
            (144, "the ship's inherent govt, 130"),
            (145, "the ship's combat govt, 131"),
        ] {
            assert_eq!(
                session.offer_refusals(MissionId(id)),
                Some(vec![]),
                "{id}: {why}"
            );
        }
    }
}
