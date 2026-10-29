//! How many days a jump takes: the slowest of the player and the ships
//! that follow it (`_HandlePlayer` @0x6c09f-0x6c13a, in `EV Nova.app`'s
//! `EV Nova`, i386).
//!
//! **The player's trip.** `_ShipHyperTransitTime` (@0x43d0-0x4484)
//! starts from the ship type's `Mass` band ([`base_jump_days`]: 1 day,
//! 2 above 99 tons, 3 above 199) and, for slot 0 alone, adds each
//! outfit's `ModType` 22 (`cmpw $0, 0x86` @0x4413, @0x441d-0x4467), at
//! least 1: the player's [`ShipStats::jump_days`](crate::stats::ShipStats).
//!
//! **The ships that follow.** The arrival then walks slots 1-63
//! (@0x6c0b9-0x6c13a): each active ship whose leader (+0x9a) is the
//! player and that is not disabled. A launched fighter of the player's
//! (AI type 5) that can't jump (`_AIShipHasFuelForJump` @0x6c0ed) is
//! deactivated there, and so counts for nothing; each other one makes the
//! trip `max(trip, _ShipHyperTransitTime(slot))` (@0x6c10e-0x6c127). For
//! a ship above slot 0 that is its class's `Mass` band alone: the
//! outfits its class carries never count. Hired, captured and mission
//! escorts count, and the player's fighters that follow it.
//!
//! **Here.** [`Space::escorts_trip_days`] is that walk over
//! [`Space::player_led`], each ship intact and of a ship type with a
//! record; a fighter of the player's (one the player leads with a
//! [`carrier`](crate::traffic::npc::Npc::carrier)) counts only when it
//! will follow the jump: by the engine's
//! [`RuleKey::FighterRecall`] when [`fighter_can_jump`] passes, the test
//! `Session::abandon_fighters` drops the rest by; by the other reading,
//! which stows every fighter, never. [`Session::trip_days`] is the larger
//! of it and the player's own. The days are counted as the jump arrives,
//! before the entry abandons or stows the fighters, in the same tick and
//! before any frame, so that order crosses no boundary. The original
//! keeps the trip in a short; ours is a `u32`.
//!
//! **What it sets.** `_IncrementGameTime` runs `trip` times (@0x6c140-
//! 0x6c151), and the escorts are paid for the same trip
//! (`_DoEscortPayment(trip)` @0x6c616-0x6c620): both through
//! `Session::pass_jump_days`. No sound.

use super::{Session, Setup, Space, Statics};
use crate::combat::hull::Condition;
use crate::rulebook::{RuleKey, RuleSource};
use crate::stats::base_jump_days;

use super::fighters::fighter_can_jump;

impl Session {
    /// The days a jump takes (see the module docs): the larger of the
    /// player's own and the slowest ship that follows it. It reads the
    /// player's ship and the space, so it lives here.
    pub(super) fn trip_days(&self) -> u32 {
        self.ship
            .fit
            .stats
            .jump_days
            .max(self.space.escorts_trip_days(&self.statics, &self.setup))
    }
}

impl Space {
    /// The days the slowest ship the player leads takes to jump, or 1
    /// with none (see the module docs).
    pub(super) fn escorts_trip_days(&self, statics: &Statics, setup: &Setup) -> u32 {
        let fighters_follow = setup.rules.source_for(RuleKey::FighterRecall) == RuleSource::Engine;
        let fuel = setup.rules.source_for(RuleKey::NpcJumpFuel);
        self.player_led()
            .filter(|npc| npc.condition == Condition::Intact)
            .filter(|npc| {
                npc.carrier.is_none()
                    || (fighters_follow && fighter_can_jump(statics, fuel, npc.ship, Some(npc)))
            })
            .filter_map(|npc| statics.ship_record(npc.ship))
            .map(|record| base_jump_days(record.fields.mass))
            .max()
            .unwrap_or(1)
    }
}

#[cfg(test)]
mod tests {
    use crate::catalog::{ShipId, SystemId};
    use crate::chance::NeverFires;
    use crate::combat::hull::Condition;
    use crate::flight::Controls;
    use crate::pilot::{Escort, Pilot};
    use crate::reserves::Reserves;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::session::Session;
    use crate::session::escorts::tests::fleeted;
    use crate::session::fighters::tests::{VIPER, carrying, fleet, out};
    use crate::stats::HYPERSPACE_DAYS;
    use crate::testkit::{FakePilotCatalog, jump, outfit, work};

    /// The warship escort's ship type in [`fleeted`].
    const WARSHIP: ShipId = ShipId(150);

    /// [`fleeted`] with the warship of `Mass` `mass`.
    fn heavy(mass: i16) -> FakePilotCatalog {
        let mut catalog = fleeted();
        catalog.ship_records[1].fields.mass = mass;
        catalog
    }

    /// A warship escort, hired when `wage` is some: its wage is its
    /// `Cost`'s, 100 a day.
    fn warship(wage: Option<i64>) -> Escort {
        Escort {
            ship: WARSHIP,
            reserves: Reserves::full(30.0, 45.0, 300.0),
            order: None,
            carried: false,
            wage,
            person: None,
            mark: None,
        }
    }

    /// `escorts` flying with `cash`, placed by the first traffic tick.
    fn escorted(catalog: &FakePilotCatalog, escorts: Vec<Escort>, cash: i64) -> Session {
        let mut pilot = Pilot::new(catalog, "Ada").expect("starts");
        pilot.escorts = escorts;
        pilot.cash = cash;
        let mut session = Session::fly(catalog, pilot).expect("flies");
        work(&mut session, catalog, Controls::default());
        session
    }

    /// A 1-day player led by a warship of `Mass` `mass`: its trip.
    fn led_by(mass: i16) -> u32 {
        let catalog = heavy(mass);
        let session = escorted(&catalog, vec![warship(None)], 0);
        assert_eq!(session.ship.fit.stats.jump_days, 1);
        session.trip_days()
    }

    #[test]
    fn the_heaviest_escort_sets_the_trip_by_its_mass_band() {
        // `_ShipHyperTransitTime` @0x43f6-0x440f.
        assert_eq!(led_by(250), 3);
        assert_eq!(led_by(200), 3);
        assert_eq!(led_by(199), 2);
        assert_eq!(led_by(150), 2);
        assert_eq!(led_by(100), 2);
    }

    #[test]
    fn escorts_of_99_tons_or_less_leave_the_trip_at_the_players() {
        assert_eq!(led_by(99), 1);
        assert_eq!(led_by(0), 1);
    }

    #[test]
    fn a_slow_player_keeps_its_trip_beside_a_light_escort() {
        let catalog = heavy(150);
        let mut session = escorted(&catalog, vec![warship(None)], 0);
        session.ship.fit.stats.jump_days = 3;
        assert_eq!(session.trip_days(), 3);
    }

    #[test]
    fn an_escorts_class_outfits_never_lengthen_its_trip() {
        // Only slot 0 adds ModType 22 (@0x4413).
        let mut catalog = heavy(150);
        catalog.outfits.push(outfit(300, &[(HYPERSPACE_DAYS, 1)]));
        catalog.ship_records[1].defaults = vec![(crate::catalog::OutfitId(300), 4)];
        let session = escorted(&catalog, vec![warship(None)], 0);
        assert_eq!(session.trip_days(), 2);
    }

    #[test]
    fn a_mission_escort_counts() {
        use crate::session::mission_escorts::tests::{escorting, flying};
        use crate::session::mission_ships::tests::ships_catalog;
        let mut catalog = ships_catalog(escorting(-1, 0));
        catalog.ship_records[1].fields.mass = 250;
        let session = flying(&catalog);
        assert!(
            session
                .player_led()
                .all(crate::traffic::npc::Npc::mission_escort)
        );
        assert_eq!(session.trip_days(), 3);
    }

    #[test]
    fn a_ship_the_player_doesnt_lead_counts_for_nothing() {
        let catalog = heavy(250);
        let mut session = escorted(&catalog, vec![warship(None)], 0);
        let id = session.space.escorts.fleet[0].expect("placed");
        let npc = session.space.npc_mut(id).expect("there");
        npc.escort = None;
        assert_eq!(session.player_led().count(), 0);
        assert_eq!(session.trip_days(), 1, "a stranger");
    }

    #[test]
    fn a_disabled_ship_the_player_leads_counts_for_nothing() {
        // `_IsDisabled` @0x6c0ce; a disabled mission escort stays led.
        use crate::session::mission_escorts::tests::{escorting, flying};
        use crate::session::mission_ships::tests::ships_catalog;
        let mut catalog = ships_catalog(escorting(-1, 0));
        catalog.ship_records[1].fields.mass = 250;
        let mut session = flying(&catalog);
        for npc in session.space.traffic.npcs_mut() {
            npc.condition = Condition::Disabled;
        }
        assert_eq!(session.player_led().count(), 2, "still led");
        assert_eq!(session.trip_days(), 1);
    }

    #[test]
    fn a_disabled_former_escort_counts_for_nothing() {
        let catalog = heavy(250);
        let mut session = escorted(&catalog, vec![warship(None)], 0);
        let id = session.space.escorts.fleet[0].expect("placed");
        session.space.npc_mut(id).expect("there").condition = Condition::Disabled;
        session.unfleet(id);
        assert_eq!(session.trip_days(), 1);
    }

    /// [`carrying`] with the Viper of `Mass` 250 and `Fuel` `fuel`, one
    /// out, by `recall`.
    fn viper_out(fuel: i16, recall: RuleSource) -> Session {
        let mut catalog = carrying();
        catalog.ship_records[1].fields.mass = 250;
        catalog.ship_records[1].fields.fuel = fuel;
        let mut session = fleet(&catalog, 0, vec![out()]);
        session.setup.rules = Rulebook::default().with_override(RuleKey::FighterRecall, recall);
        let npc = session.player_led().next().expect("out");
        assert_eq!(npc.ship, VIPER);
        assert!(npc.carrier.is_some(), "a fighter");
        session
    }

    #[test]
    fn a_fighter_that_can_jump_counts_and_one_that_cant_does_not() {
        // `_AIShipHasFuelForJump` @0x6c0ed: a class `Fuel` above 99.
        assert_eq!(viper_out(300, RuleSource::Engine).trip_days(), 3);
        assert_eq!(viper_out(100, RuleSource::Engine).trip_days(), 3);
        assert_eq!(viper_out(99, RuleSource::Engine).trip_days(), 1);
    }

    #[test]
    fn by_the_bible_recall_no_fighter_counts() {
        assert_eq!(viper_out(300, RuleSource::Bible).trip_days(), 1);
    }

    // Through the arrival.

    #[test]
    fn a_jump_with_a_heavy_escort_takes_its_days_and_pays_its_wages_for_them() {
        let catalog = heavy(250);
        let mut session = escorted(&catalog, vec![warship(Some(100))], 1000);
        let mut date = session.date();
        assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(131)));
        for _ in 0..3 {
            date = date.next_day();
        }
        assert_eq!(session.date(), date, "three days");
        assert_eq!(session.pilot().cash(), 700, "three days' wages");
    }

    #[test]
    fn each_daily_step_of_the_trip_runs() {
        use crate::story::StoryEvent;
        let catalog = heavy(250);
        let mut session = escorted(&catalog, vec![warship(None)], 0);
        session.story_raised();
        jump(&mut session, &catalog, 131).expect("arrives");
        let days = session
            .story_raised()
            .into_iter()
            .filter(|event| matches!(event, StoryEvent::DayPassed { .. }))
            .count();
        assert_eq!(days, 3);
    }

    #[test]
    fn by_per_hop_each_hop_takes_the_escorts_days() {
        use crate::hyperspace::MultiJumpRule;
        use crate::session::tests::chained;
        let catalog = chained(heavy(250), 2);
        let mut pilot = Pilot::new(&catalog, "Ada").expect("starts");
        pilot.escorts = vec![warship(Some(100))];
        pilot.cash = 1000;
        let mut session = Session::fly(&catalog, pilot)
            .expect("flies")
            .with_multi_jump(MultiJumpRule::PerHop);
        work(&mut session, &catalog, Controls::default());
        session.plot_course(SystemId(134)).expect("a route");
        let mut date = session.date();
        crate::testkit::fly_out(&mut session, &catalog);
        crate::testkit::begin_jump_now(&mut session, &catalog).expect("jumps");
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(134))
        );
        for _ in 0..9 {
            date = date.next_day();
        }
        assert_eq!(session.date(), date, "three hops of three days");
        assert_eq!(session.pilot().cash(), 100, "nine days' wages");
    }

    #[test]
    fn a_gate_exit_taking_a_jumps_days_takes_the_escorts_trip() {
        use crate::gate::{GateArrivalRule, GateKind};
        use crate::session::gate_follow::tests::{exit_gate_raising, following, gated};
        use crate::session::mission_ships::tests::{accepted, take_off};
        for (rule, days) in [(GateArrivalRule::LikeJump, 3), (GateArrivalRule::Engine, 0)] {
            let mut catalog = gated(following(1, 0), 0);
            for record in &mut catalog.ship_records {
                if record.id == ShipId(200) {
                    record.fields.mass = 250;
                }
            }
            let mut session = accepted(&catalog).with_gate_arrival(rule);
            take_off(&mut session, &catalog);
            assert!(
                session
                    .player_led()
                    .any(crate::traffic::npc::Npc::mission_escort)
            );
            let mut date = session.date();
            exit_gate_raising(&mut session, &catalog, GateKind::Hypergate, &mut NeverFires);
            for _ in 0..days {
                date = date.next_day();
            }
            assert_eq!(session.date(), date, "{rule:?}");
        }
    }

    #[test]
    fn a_fighter_abandoned_on_the_arrival_does_not_stretch_it() {
        let mut catalog = carrying();
        catalog.ship_records[1].fields.mass = 250;
        catalog.ship_records[1].fields.fuel = 0;
        let mut session = fleet(&catalog, 0, vec![out()]);
        let mut date = session.date();
        assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(131)));
        assert_eq!(session.pilot().escorts(), [], "abandoned");
        date = date.next_day();
        assert_eq!(session.date(), date, "the player's one day");
    }
}
