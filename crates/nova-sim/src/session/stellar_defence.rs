//! The session's part of the stellars' defence fleets (see
//! `nova_sim_ai::traffic::stellar_defence`, which the traffic owns, and
//! `nova_sim_ai::ai::defence_ship`, which flies them):
//!
//! - **The +0x47 pass** ([`Session::mark_defended_stellars`]): each frame,
//!   after the ship loop and before the stellar guns, every nav stellar
//!   of the player's system (the chart's listing) is marked under attack
//!   exactly while one of its defence ships is in the system
//!   (`_DoPlayGameWork` @0x443d4-0x4444f), which the guns' `defended`
//!   reads (see the `stellar_guns` module).
//! - **The clear** (`Session::rebuild_quietly`): every rebuild point
//!   clears every stellar's mark, docked or not, as
//!   `_HandleStellarSystemVisibility` does (@0x32d5d), so a gate exit's
//!   extra arrivals pass sends no defender up.
//! - **The refund** ([`Session::refund_defenders`]): the defence ships
//!   dropped with the system's ships give their stellars one back each,
//!   beside the aux ships' give-back (`_DeleteAllComputerShips`
//!   @0x3c02a-0x3c06f): at a populate, a dropping move and a traffic
//!   Reset, which hands the counts on to the next traffic.
//! - **The spawn** ([`Session::spawn_stellar_defender`]): one defence
//!   ship sent up at once, the planet comm dialog's Demand Tribute's
//!   (`_DoPlanetCommDialog` @0x97814; see the `planet_comm` module).
//! - **The lend** ([`Session::lend_defence_ships`]): the defence ships
//!   left rest on the pilot (`Pilot::defence_ships`), where the save sees
//!   them; `Session::step_traffic` and [`Session::refund_defenders`] lend
//!   them to the traffic for the step and take them back, so every
//!   respawn, spawn and refund writes the one store. Outside a step the
//!   planet comm dialog and the daily regrowth change the pilot's.
//! - **The load** ([`keep_on_load`]): `_LoadPilotData` runs after
//!   `_ResetPlayerRecord` and sets each stellar's +0x46 to the saved byte
//!   for a stellar whose record exists (+0x45), else 0
//!   (@0x75963-0x75991), then +0x466 to the saved count for a stellar that
//!   exists and is not dominated, else 0 (@0x75d70-0x75db0). Here a saved
//!   domination or count for a stellar the data no longer lists is
//!   dropped, every stellar dominated by the one rule (at run time or by
//!   `Flags2` 0x0020) gets 0 left, and each kept count takes its
//!   `DefCount` from the data, for the refund's cap. +0x45 is the
//!   record-loaded byte, not "not destroyed" (destruction is +0x3c and
//!   +0x47c), so a destroyed stellar keeps its saved count. The load runs
//!   for a new pilot too, where it zeroes only a `Flags2` 0x0020
//!   stellar's count, which nothing reads: such a stellar offers no
//!   Demand Tribute, so it never launches.
//!
//! - **The capture's holders** ([`Space::note_capture_holders`],
//!   [`Space::free_capture_holders`]): a defender captured stops defending
//!   (`Session::join_fleet`, @0x9470d), and so does each ship holding the
//!   capture as `_AIShipCapturedByPlayer` walks them (@0x896b6-0x89704).
//!   `_AIShipBoardedByPlayer` leaves +0x8c (@0x897e6-0x8980f), so boarding
//!   and leaving, and Use As My Ship (@0x41309-0x413ac), keep them
//!   defending. The original's dialog runs before its boarding drops the
//!   holders; the engine's `Session::mark_boarded` runs before the plunder,
//!   so the holders are noted just before it. A mission's early board
//!   (@0x653c0, @0x6575d) drops them before the dialog in the original too,
//!   so a failed mission's ship boarded early notes none. Each boarding
//!   overwrites the note and only a capture reads it, so one left by a
//!   boarding that captured nothing is never read.
//!
//! Nothing here plays a sound.

use std::collections::BTreeSet;

use super::{Session, Space, holds_quarry};
use crate::catalog::{ListedStellar, StellarId};
use crate::chance::Chance;
use crate::domination;
use crate::pilot::Pilot;
use crate::traffic::npc::NpcId;
use crate::traffic::stellar_defence::DefenceShips;

impl Session {
    /// The frame's +0x47 pass (see the module docs).
    pub(super) fn mark_defended_stellars(&mut self) {
        self.space
            .traffic
            .recompute_defence_alarms(&self.chart.listed);
    }

    /// The defence ships among the system's ships but those in `kept`
    /// give their stellars one back each, as the ships are dropped (see the
    /// module docs).
    pub(super) fn refund_defenders(&mut self, kept: &BTreeSet<NpcId>) {
        self.lend_defence_ships();
        self.space.traffic.refund_defenders(kept);
        self.lend_defence_ships();
    }

    /// Swaps the pilot's defence ships left with the traffic's: called in
    /// pairs, it lends them to the traffic for one step and takes them
    /// back (see the module docs).
    pub(super) fn lend_defence_ships(&mut self) {
        self.space
            .traffic
            .swap_defence_ships(&mut self.pilot.defence_ships);
    }

    /// Sends up one of `stellar`'s defence ships at once, drawing on
    /// `chance`, as `_SpawnStellarDefenseShip` does (see
    /// `nova_sim_ai::traffic::Traffic::spawn_defender`): its number, or
    /// none when the player's system lists no such stellar, it has no
    /// `DefenseDude` or no slot is free.
    pub(crate) fn spawn_stellar_defender(
        &mut self,
        stellar: StellarId,
        chance: &mut dyn Chance,
    ) -> Option<NpcId> {
        let site = *self.chart.listed.iter().find(|site| site.id == stellar)?;
        self.step_traffic(|traffic, world| traffic.spawn_defender(&site, &world, chance))
    }
}

impl Space {
    /// Notes the ships holding NPC `id` as it is boarded, none when
    /// `boarded_early` (see the module docs' "The capture's holders").
    pub(super) fn note_capture_holders(&mut self, id: NpcId, boarded_early: bool) {
        self.escorts.capture_holders = if boarded_early {
            Vec::new()
        } else {
            self.traffic
                .npcs()
                .iter()
                .filter(|npc| holds_quarry(npc, id))
                .map(|npc| npc.id)
                .collect()
        };
    }

    /// The ships noted stop defending, the note used up; one that has
    /// gone is skipped (see the module docs' "The capture's holders").
    pub(super) fn free_capture_holders(&mut self) {
        let holders = std::mem::take(&mut self.escorts.capture_holders);
        for npc in self.traffic.npcs_mut() {
            if holders.contains(&npc.id) {
                npc.defends = None;
            }
        }
    }
}

/// What a load keeps of `pilot`'s saved domination and defence ships left,
/// with `listed` the stellars the data lists, as `_LoadPilotData` does
/// after `_ResetPlayerRecord` (see the module docs' "The load").
pub(super) fn keep_on_load(pilot: &mut Pilot, listed: &[ListedStellar]) {
    let find = |stellar: StellarId| {
        listed
            .binary_search_by_key(&stellar, |listed| listed.id)
            .ok()
            .map(|at| &listed[at])
    };
    let unlisted: Vec<StellarId> = pilot
        .galaxy
        .stellars()
        .filter(|&(id, change)| change.dominated && find(id).is_none())
        .map(|(id, _)| id)
        .collect();
    for stellar in unlisted {
        pilot.galaxy.set_dominated(stellar, false);
    }
    let saved = std::mem::take(&mut pilot.defence_ships);
    let mut kept = DefenceShips::default();
    for (stellar, left, _) in saved.saved() {
        if let Some(listed) = find(stellar) {
            kept.set_left_of(stellar, listed.def_count, left);
        }
    }
    for listed in listed {
        if domination::dominated_by(pilot.galaxy.stellar(listed.id).dominated, listed.flags2) {
            kept.set_left_of(listed.id, listed.def_count, 0);
        }
    }
    pilot.defence_ships = kept;
}

/// The fixture's reach into the defences (`crate::fixture::alarm_stellar`,
/// `crate::fixture::defence_ships_left`).
#[cfg(any(test, feature = "fixture"))]
impl Session {
    /// Marks `stellar` under attack.
    pub(crate) fn alarm_stellar(&mut self, stellar: StellarId) {
        self.space.traffic.defences_mut().alarm(stellar);
    }

    /// The defence ships `stellar`, one the system lists, has left.
    pub(crate) fn defence_ships_left(&self, stellar: StellarId) -> Option<i16> {
        self.chart
            .listed
            .iter()
            .find(|site| site.id == stellar)
            .map(|site| self.pilot.defence_ships.left(site))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::Goal;
    use crate::board::{Assigned, Assignment, Boarding, NovaBoarding, Take, Taken};
    use crate::catalog::{DudeId, DudeRecord, LandingSite, ShipId, SystemId};
    use crate::chance::NeverFires;
    use crate::combat::ShipRef;
    use crate::control::SetExpr;
    use crate::flight::Controls;
    use crate::legal::NovaLaw;
    use crate::mission::FailedBy;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::session::capture_naming::tests::use_as_my_ship;
    use crate::session::mission_ships::tests::{in_flight_with, rescuing};
    use crate::session::tests::{alongside, board_by, boardable, take};
    use crate::testkit::{Draws, FAST, FakePilotCatalog, catalog, jump, land_now, ship, work};

    const EARTH: StellarId = StellarId(128);

    /// The test catalog with stellar 128 defended by düde 140 (ship 129),
    /// `DefCount` `def_count`.
    fn defended(def_count: i16) -> FakePilotCatalog {
        let mut base = catalog();
        for (_, sites) in &mut base.sites {
            for site in sites.iter_mut().filter(|site| site.id == EARTH) {
                *site = LandingSite {
                    defense_dude: Some(DudeId(140)),
                    def_count,
                    ..*site
                };
            }
        }
        base.dudes.push((
            DudeId(140),
            DudeRecord {
                ai_type: 1,
                govt: None,
                ships: vec![(ShipId(129), 1)],
                booty: 0,
                info_types: 0,
            },
        ));
        base.ship_records.push(ship(129, FAST));
        base
    }

    /// `catalog`'s session in flight, its system populated.
    fn flying(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        work(&mut session, catalog, Controls::default());
        session
    }

    /// The ships of `session` defending `stellar`.
    fn up(session: &Session, stellar: StellarId) -> usize {
        session
            .npcs()
            .iter()
            .filter(|npc| npc.defends == Some(stellar))
            .count()
    }

    /// Earth, as `session` lists it.
    fn earth(session: &Session) -> LandingSite {
        *session
            .chart
            .listed
            .iter()
            .find(|site| site.id == EARTH)
            .expect("listed")
    }

    #[test]
    fn a_stellar_listed_sends_up_a_defender_at_once_and_one_unlisted_none() {
        let catalog = defended(3);
        let mut session = flying(&catalog);
        let id = session.spawn_stellar_defender(EARTH, &mut NeverFires);
        assert!(
            id.is_some(),
            "its DefenseDude drawn from the system's table"
        );
        assert_eq!(up(&session, EARTH), 1);
        assert!(session.space.traffic.defences().under_attack(EARTH));
        assert_eq!(
            session.spawn_stellar_defender(StellarId(140), &mut NeverFires),
            None,
            "system 131's"
        );
    }

    #[test]
    fn the_frame_pass_keeps_a_stellar_marked_while_its_defender_is_up() {
        // A wave of 1: the respawn never tops it up past the one.
        let catalog = defended(1);
        let mut session = flying(&catalog);
        let id = session
            .spawn_stellar_defender(EARTH, &mut NeverFires)
            .expect("up");
        work(&mut session, &catalog, Controls::default());
        assert!(session.space.traffic.defences().under_attack(EARTH));
        assert_eq!(up(&session, EARTH), 1);
        // Destroyed in the ship loop: the frame's pass clears the mark, so
        // the next frame's respawn sends none up.
        session.space.traffic.remove(id);
        session.mark_defended_stellars();
        assert!(!session.space.traffic.defences().under_attack(EARTH));
        work(&mut session, &catalog, Controls::default());
        assert_eq!(up(&session, EARTH), 0);
    }

    #[test]
    fn a_jump_or_a_landing_clears_every_stellars_mark() {
        let catalog = defended(0);
        let mut session = flying(&catalog);
        session.space.traffic.defences_mut().alarm(EARTH);
        session.space.traffic.defences_mut().alarm(StellarId(400));
        jump(&mut session, &catalog, 131);
        let defences = session.space.traffic.defences();
        assert!(!defences.under_attack(EARTH));
        assert!(!defences.under_attack(StellarId(400)));
        let mut session = flying(&catalog);
        session.ship.player.position = crate::geometry::Vec2::new(30.0, -40.0);
        session.ship.player.velocity = crate::geometry::Vec2::ZERO;
        session.space.traffic.defences_mut().alarm(EARTH);
        assert_eq!(land_now(&mut session), Ok(EARTH));
        assert!(!session.space.traffic.defences().under_attack(EARTH));
    }

    #[test]
    fn a_jump_gives_back_the_defenders_left_in_space_capped_at_the_def_count() {
        let catalog = defended(2);
        let mut session = flying(&catalog);
        for _ in 0..2 {
            session.spawn_stellar_defender(EARTH, &mut NeverFires);
        }
        let site = earth(&session);
        session.pilot.defence_ships.set_left(&site, 1);
        jump(&mut session, &catalog, 131);
        assert_eq!(up(&session, EARTH), 0);
        assert_eq!(
            session.pilot.defence_ships.left(&site),
            2,
            "1 + 2, capped at DefCount 2"
        );
    }

    #[test]
    fn a_relocation_gives_back_the_defenders_and_hands_the_counts_on() {
        // The developer move's traffic Reset drops every ship as
        // `_DeleteAllComputerShips` does, refunding before the counts pass
        // to the fresh traffic.
        let catalog = defended(3);
        let mut session = flying(&catalog);
        let site = earth(&session);
        session.pilot.defence_ships.set_left(&site, 0);
        session
            .spawn_stellar_defender(EARTH, &mut NeverFires)
            .expect("up");
        let here = session.system();
        session.land_at(&catalog, here, EARTH).expect("lands");
        assert_eq!(up(&session, EARTH), 0, "a fresh traffic");
        assert_eq!(
            session.pilot.defence_ships.left(&site),
            1,
            "0 + 1, kept across the Reset, not DefCount 3 decoded afresh"
        );
    }

    /// The rules with `MoveLeftBehind` read as `reading`.
    fn moving(reading: RuleSource) -> Rulebook {
        Rulebook::default().with_override(RuleKey::MoveLeftBehind, reading)
    }

    #[test]
    fn a_move_that_drops_the_ships_left_behind_gives_back_their_defenders() {
        // By the engine the ships stay in the table until the display pass
        // frees them, nothing given back; by the other reading the move
        // drops them, refunding each defender.
        for (reading, left) in [(RuleSource::Bible, 1), (RuleSource::Engine, 0)] {
            let catalog = defended(3);
            let mut session = Session::start(&catalog)
                .expect("starts")
                .with_rules(moving(reading));
            work(&mut session, &catalog, Controls::default());
            let site = earth(&session);
            session.pilot.defence_ships.set_left(&site, 0);
            session
                .spawn_stellar_defender(EARTH, &mut NeverFires)
                .expect("up");
            session.run_set(&SetExpr::parse("M131").expect("parses"), &mut NeverFires);
            let _ = session.settle_script(&catalog, &mut NeverFires);
            assert_eq!(session.system(), SystemId(131), "{reading:?}");
            assert_eq!(session.pilot.defence_ships.left(&site), left, "{reading:?}");
        }
    }

    #[test]
    fn the_fixture_alarms_a_stellar_and_reads_its_ships_left() {
        let catalog = defended(3);
        let mut session = flying(&catalog);
        assert_eq!(crate::fixture::defence_ships_left(&session, EARTH), Some(3));
        assert_eq!(
            crate::fixture::defence_ships_left(&session, StellarId(129)),
            Some(0),
            "129's own count, none"
        );
        assert_eq!(
            crate::fixture::defence_ships_left(&session, StellarId(140)),
            None,
            "not listed"
        );
        crate::fixture::alarm_stellar(&mut session, EARTH);
        work(&mut session, &catalog, Controls::default());
        assert_eq!(up(&session, EARTH), 1);
        assert_eq!(crate::fixture::defence_ships_left(&session, EARTH), Some(2));
    }

    #[test]
    fn the_counts_rest_on_the_pilot_between_steps() {
        let catalog = defended(3);
        let mut session = flying(&catalog);
        let site = earth(&session);
        crate::fixture::alarm_stellar(&mut session, EARTH);
        work(&mut session, &catalog, Controls::default());
        assert_eq!(up(&session, EARTH), 1, "the respawn's");
        assert_eq!(session.pilot().defence_ships.left(&site), 2, "spent");
        assert_eq!(
            session.space.traffic.defences().ships(),
            &crate::traffic::stellar_defence::DefenceShips::default(),
            "lent back"
        );
        jump(&mut session, &catalog, 131);
        assert_eq!(session.pilot().defence_ships.left(&site), 3, "refunded");
        assert_eq!(
            session.space.traffic.defences().ships(),
            &crate::traffic::stellar_defence::DefenceShips::default()
        );
    }

    /// `session`'s pilot saved and loaded into a session of `catalog`.
    fn reloaded(session: &Session, catalog: &FakePilotCatalog) -> Session {
        let text = crate::save::encode(session.pilot());
        let pilot = crate::save::decode(&text).expect("loads");
        Session::fly(catalog, pilot).expect("flies")
    }

    #[test]
    fn a_stellar_dominated_at_run_time_stays_so_across_a_save_with_its_fee_waived() {
        let mut catalog = defended(3);
        catalog.sites[0].1[0].fee = 100;
        let mut session = flying(&catalog);
        session
            .chart
            .set_dominated(&mut session.pilot, &mut session.out, EARTH, true);
        let session = reloaded(&session, &catalog);
        let site = earth(&session);
        assert!(crate::domination::dominated(&site), "@0x7597b");
        assert_eq!(crate::landing::fee_due(&site), None);
    }

    #[test]
    fn a_spent_count_survives_a_save_and_a_dominated_stellars_loads_as_0() {
        let catalog = defended(3);
        let mut session = flying(&catalog);
        let site = earth(&session);
        session.pilot.defence_ships.set_left(&site, 1);
        let back = reloaded(&session, &catalog);
        assert_eq!(back.pilot().defence_ships.left(&site), 1, "kept");
        session
            .chart
            .set_dominated(&mut session.pilot, &mut session.out, EARTH, true);
        let back = reloaded(&session, &catalog);
        assert_eq!(
            back.pilot().defence_ships.left(&site),
            0,
            "dominated: 0 (@0x75d76, @0x75da7)"
        );
        let mut always = defended(3);
        always.sites[0].1[0].flags2 = crate::market::DOMINATED;
        let mut session = flying(&always);
        let site = earth(&session);
        session.pilot.defence_ships.set_left(&site, 2);
        let back = reloaded(&session, &always);
        assert_eq!(back.pilot().defence_ships.left(&site), 0, "Flags2 0x0020");
    }

    #[test]
    fn a_destroyed_stellar_keeps_its_saved_count() {
        // The load's test is +0x45, the record loaded (@0x75d70), not
        // destruction (+0x3c, +0x47c).
        let catalog = defended(3);
        let mut session = flying(&catalog);
        let site = earth(&session);
        session.pilot.defence_ships.set_left(&site, 1);
        session.chart.set_stellar_state(
            &mut session.pilot,
            &mut session.out,
            EARTH,
            Some(crate::story::Countdown { days_left: 5 }),
        );
        let back = reloaded(&session, &catalog);
        assert_eq!(back.pilot().defence_ships.left(&site), 1);
    }

    #[test]
    fn what_a_save_keeps_of_a_stellar_the_data_no_longer_has_is_dropped() {
        let catalog = defended(3);
        let mut session = flying(&catalog);
        let gone = StellarId(999);
        session.pilot.defence_ships.set_left_of(gone, 3, 1);
        session.pilot.galaxy.set_dominated(gone, true);
        let back = reloaded(&session, &catalog);
        assert_eq!(
            back.pilot().defence_ships,
            crate::traffic::stellar_defence::DefenceShips::default(),
            "@0x75d70"
        );
        assert!(!back.pilot().galaxy.stellar(gone).dominated, "@0x75980");
    }

    #[test]
    fn a_kept_count_takes_its_def_count_from_the_data_for_the_refund() {
        let catalog = defended(3);
        let mut session = flying(&catalog);
        session.pilot.defence_ships.set_left_of(EARTH, 1000, 3);
        let mut back = reloaded(&session, &catalog);
        back.pilot.defence_ships.refund(EARTH);
        assert_eq!(
            back.pilot().defence_ships.left_of(EARTH, 3),
            3,
            "capped at the data's DefCount 3"
        );
    }

    #[test]
    fn a_captured_defender_defends_no_stellar() {
        let catalog = defended(3);
        let mut session = flying(&catalog);
        let id = session
            .spawn_stellar_defender(EARTH, &mut NeverFires)
            .expect("up");
        session.join_fleet(id, &mut NeverFires);
        assert_eq!(session.space.npc(id).map(|npc| npc.defends), Some(None));
    }

    // The holders of a captured ship.

    /// What ship `id` defends.
    fn defends(session: &Session, id: NpcId) -> Option<StellarId> {
        session.space.npc(id).and_then(|npc| npc.defends)
    }

    /// Adds a defender of Earth, a copy of the police (NPC 1), that
    /// targets and fights `quarry`.
    fn defender_aimed_at(session: &mut Session, quarry: ShipRef) -> NpcId {
        let mut npc = session.npcs()[1].clone();
        npc.defends = Some(EARTH);
        npc.target = Some(quarry);
        npc.goal = Goal::Attack(quarry);
        session.space.traffic.add_npc(npc)
    }

    /// [`alongside`] with a defender holding the trader (NPC 0) and one
    /// aimed at the player: (the holder, the other).
    fn held(session: &mut Session) -> (NpcId, NpcId) {
        (
            defender_aimed_at(session, ShipRef::Npc(NpcId(0))),
            defender_aimed_at(session, ShipRef::Player),
        )
    }

    /// Boards the target and captures it, its dialog left where the
    /// capture leaves it.
    fn board_and_capture(session: &mut Session) -> Taken {
        let boarded = board_by(session, &NovaLaw::default());
        assert!(matches!(boarded, Ok(Boarding::Opened(_))), "{boarded:?}");
        take(session, Take::Capture, &[0, 1]).0
    }

    #[test]
    fn use_as_escort_stops_the_captured_ships_holders_defending_only() {
        let catalog = boardable();
        let mut session = alongside(&catalog);
        let (holder, other) = held(&mut session);
        assert_eq!(board_and_capture(&mut session), Taken::Captured);
        assert_eq!(defends(&session, holder), Some(EARTH), "not yet captured");
        assert_eq!(
            session.assign(Assignment::Escort, &mut Draws::of(&[])),
            Some(Assigned::Escort)
        );
        assert_eq!(defends(&session, holder), None, "@0x896b6-0x89704");
        assert_eq!(defends(&session, other), Some(EARTH));
    }

    #[test]
    fn a_crewless_capture_stops_the_holders_defending_only() {
        let catalog = boardable();
        let mut session = alongside(&catalog);
        for record in &mut session.statics_mut().ships {
            if record.id == ShipId(128) {
                record.crew = 0;
            }
        }
        let (holder, other) = held(&mut session);
        assert_eq!(board_and_capture(&mut session), Taken::Escorted);
        assert_eq!(defends(&session, holder), None);
        assert_eq!(defends(&session, other), Some(EARTH));
    }

    #[test]
    fn a_holder_of_a_ship_boarded_and_left_keeps_defending() {
        let catalog = boardable();
        let mut session = alongside(&catalog);
        let (holder, _) = held(&mut session);
        let boarded = board_by(&mut session, &NovaLaw::default());
        assert!(matches!(boarded, Ok(Boarding::Opened(_))), "{boarded:?}");
        session.space.escorts.aboard = None;
        assert_eq!(defends(&session, holder), Some(EARTH), "@0x897e6-0x8980f");
        let target = session.space.npc(holder).map(|npc| npc.target);
        assert_eq!(target, Some(None), "let go all the same");
    }

    #[test]
    fn a_holder_of_a_ship_taken_as_my_ship_keeps_defending() {
        let catalog = boardable();
        let mut session = alongside(&catalog);
        let (holder, _) = held(&mut session);
        assert_eq!(board_and_capture(&mut session), Taken::Captured);
        let assigned = use_as_my_ship(&mut session, "Prize", &mut NeverFires);
        assert!(assigned.is_some());
        assert_eq!(defends(&session, holder), Some(EARTH), "@0x41309-0x413ac");
    }

    #[test]
    fn a_repelled_boardings_holder_keeps_defending_through_a_later_capture() {
        let catalog = boardable();
        let mut session = alongside(&catalog);
        let (holder, _) = held(&mut session);
        let mut second = session.npcs()[0].clone();
        session.space.traffic.npcs_mut()[0].booty = Some(0);
        second.booty = None;
        let second = session.space.traffic.add_npc(second);
        let repelling = NovaBoarding {
            empty_booty: RuleSource::Bible,
            ..NovaBoarding::default()
        };
        assert_eq!(
            session.board(&NovaLaw::default(), &repelling, &mut Draws::of(&[])),
            Ok(Boarding::Repelled)
        );
        session.chart.target = Some(second);
        assert_eq!(board_and_capture(&mut session), Taken::Captured);
        session.assign(Assignment::Escort, &mut Draws::of(&[]));
        assert_eq!(defends(&session, holder), Some(EARTH));
    }

    /// Whether a holder of a failed mission's ship, boarded with `left`
    /// ships left and captured, still defends, and whether that board
    /// said a line.
    fn failed_mission_holder_keeps_defending(left: i16) -> (bool, bool) {
        let catalog = rescuing(5, left, 0x0001);
        let mut session = in_flight_with(&catalog, left);
        if let Some(active) = session.pilot.missions.get_mut(0) {
            active.failed = Some(FailedBy::Marked);
        }
        let mut npc = session.npcs()[0].clone();
        npc.mission = None;
        let quarry = session.npcs()[0].id;
        npc.defends = Some(EARTH);
        npc.target = Some(ShipRef::Npc(quarry));
        let holder = session.space.traffic.add_npc(npc);
        session.chart.target = Some(quarry);
        session.ship.player.position = session.npcs()[0].state.position;
        let boarded = board_by(&mut session, &NovaLaw::default());
        assert!(matches!(boarded, Ok(Boarding::Opened(_))), "{boarded:?}");
        let spoke = session.take_board_line().is_some();
        if let Some(aboard) = &mut session.space.escorts.aboard {
            aboard.plunder.odds = 100;
        }
        let taken = take(&mut session, Take::Capture, &[0, 1]).0;
        if taken == Taken::Captured {
            session.assign(Assignment::Escort, &mut Draws::of(&[]));
        } else {
            assert_eq!(taken, Taken::Escorted);
        }
        (defends(&session, holder).is_some(), spoke)
    }

    #[test]
    fn a_failed_missions_ship_boarded_early_keeps_its_holder_through_a_capture() {
        assert_eq!(
            failed_mission_holder_keeps_defending(1),
            (true, true),
            "dropped by the early board, before the plunder (@0x6575d)"
        );
    }

    #[test]
    fn a_failed_missions_ship_with_no_early_board_loses_its_holder_to_a_capture() {
        assert_eq!(failed_mission_holder_keeps_defending(2), (false, false));
    }
}
