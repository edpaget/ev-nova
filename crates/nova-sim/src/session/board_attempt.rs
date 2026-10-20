//! The session's side of the board attempt (`_HandlePlayerBoardAttempt`
//! @0x65000-0x66064), around [`Session::board`]: the target as the
//! checks see it, the refusal beep, the Good Samaritan's gate, the comm
//! line of a failed mission's ship and the assist path's hold.
//!
//! **The checks** ([`check_board`](crate::board::check_board)) see the
//! target with its class's `Crew` and whether it carries a mission's mark,
//! and read
//! [`RuleKey::CrewlessMissionShip`](crate::RuleKey::CrewlessMissionShip):
//! by the engine a mission's special ship is boarded whatever its crew.
//!
//! **The refusal beep**: each refusal said, #130 (@0x65ffe, the refused
//! `PickupMode` 2 pickup's too, @0x65366 → @0x65fdb), #131 (@0x65f9a)
//! and #132 (@0x65fcc), plays `_beepSnd[3]`, `snd ` 153, at priority 1
//! (not modelled; see rdm:task/refused-and-port-ambient-priorities-by-site):
//! [`SimSound::Refused`]. No target, and not facing it, return silently
//! (@0x65031, @0x65298 → @0x6605a), as does a board while landed, which
//! the original never reaches.
//!
//! **The Good Samaritan** (`_DoGoodSamaritan` @0x65ed9): the boarding
//! calls the ships around to the target's help only on its plunder paths
//! that reach it: a ship no person flies (@0x65db6, @0x65dd1), a person
//! with no `LinkMission` (@0x65a83, @0x65a9e), and a failed mission's
//! ship, which takes the first. A person with a `LinkMission` never calls
//! it, whatever its `Flags`: with 0x0200 its offer (@0x65907) or, its
//! mission unavailable, its plunder (@0x65991); without, the plunder past
//! it (@0x65a21 → @0x65ede). The fighter and escort cases that skip it are
//! the boarding and fighter-bay follow-ups' (rdm:task/boarding-follow-up,
//! rdm:task/fighter-bays-follow-up). The call is a strike that is answered
//! but not direct ([`Strike::direct`]): `_DoGoodSamaritan` writes only goal
//! 4 and the target on the ships its gates pass (@0x82891-0x82ba8), so the
//! ship boarded and its lead draw no hit's reaction, cut no wait and ask
//! no quote; they answer as Samaritans or not at all. The ship boarded is
//! still disabled when the call is answered, as its condition is settled
//! by the fight after the traffic answers, so it never answers its own.
//!
//! **A failed mission's ship** takes the pickup or assist path as any
//! mission's ship does, as neither tests whether the mission failed
//! (@0x652ee, @0x65575-0x65762), then goes on to the plunder (@0x657e3
//! → @0x6582b), uncharged as the slap passes its mark. The board then
//! says the path's comm line and opens the plunder; the line is held for
//! the view, once ([`Session::take_board_line`]).
//!
//! **The assist path's hold**: the ship boarded there has its AI wait
//! (+0x4c) set to [`BOARDED_WAIT`] (100.0 @0x65746), so it holds its
//! course, with no steering, thrust or fire, for that many ticks
//! (`Npc::wait_on_course`, `_HandleShip` @0x34442-0x34454,
//! @0x34a54-0x34a72, counted down @0x35026-0x35048). The pickup path sets
//! no wait.

use super::{Outbox, Session, Space, Statics};
use crate::board::{BoardRefusal, BoardTarget, MissionBoardLine};
use crate::combat::{Nominal, ShipRef, Strike};
use crate::sound::SimSound;
use crate::traffic::npc::{Npc, NpcId};

/// The AI wait, in ticks, of a ship boarded on a mission's assist path
/// (the float 100.0 @0x65746).
pub(super) const BOARDED_WAIT: u32 = 100;

impl Session {
    /// The comm line a failed mission's ship said as the last board
    /// plundered it, on its pickup or assist path, once: taking it leaves
    /// none. Any other board leaves none here: a standing mission's ship
    /// says its line in [`Boarding::MissionShip`](crate::board::Boarding::MissionShip).
    pub fn take_board_line(&mut self) -> Option<MissionBoardLine> {
        self.out.take_board_line()
    }

    /// Puts the player's ship over NPC `id`, at its place, velocity and
    /// heading, and targets it, for
    /// [`fixture::put_player_over`](crate::fixture::put_player_over). An
    /// NPC not in the system changes nothing.
    #[cfg(any(test, feature = "fixture"))]
    pub(crate) fn put_player_over(&mut self, id: NpcId) {
        if let Some(state) = self.space.npc(id).map(|npc| npc.state) {
            self.ship.player = state;
            self.chart.target = Some(id);
        }
    }
}

impl Outbox {
    /// Beeps [`SimSound::Refused`] for `refusal` when it is said, as
    /// `_HandlePlayerBoardAttempt` plays `_beepSnd[3]` before each line
    /// (see the module docs); a silent one ([`BoardRefusal::silent`])
    /// plays nothing.
    pub(super) fn beep_refusal(&mut self, refusal: BoardRefusal) {
        if !refusal.silent() {
            self.sounds.push(SimSound::Refused);
        }
    }

    /// The comm line a failed mission's ship said as the last board
    /// plundered it, on its pickup or assist path, once: taking it leaves
    /// none. Any other board leaves none here: a standing mission's ship
    /// says its line in [`Boarding::MissionShip`](crate::board::Boarding::MissionShip).
    fn take_board_line(&mut self) -> Option<MissionBoardLine> {
        self.board_line.take()
    }
}

impl Statics {
    /// `npc` as the boarding checks see it: its class's `Crew` (none
    /// without a record) and whether it carries a mission's mark, failed,
    /// standing or stale.
    pub(super) fn board_target(&self, npc: &Npc) -> BoardTarget {
        BoardTarget {
            state: npc.state,
            condition: npc.condition,
            boarded: npc.boarded,
            crew: self.crew(npc.ship),
            mission: npc.mission.is_some(),
            reach: npc.hull.board_reach,
        }
    }
}

impl Space {
    /// Calls the NPCs to `npc`'s help as for a hit by the player: an
    /// intended strike of no damage answered on the next traffic tick
    /// (`_DoGoodSamaritan`, called by `_HandlePlayerBoardAttempt` itself,
    /// with no aim to test), unless `npc` is a person with a
    /// `LinkMission`, whose boarding never calls it (see the module docs).
    /// It never passes `_DamageShip`, so the answer flag never weighs it:
    /// it is answered as it is. Nor is it direct, so neither the ship
    /// boarded nor its lead draws a hit's reaction from it: only the Good
    /// Samaritans answer, as `_DoGoodSamaritan` (@0x82823) writes goal 4
    /// and the target alone (@0x82b82-0x82b97).
    ///
    /// The original calls it (@0x65ed9) before the plunder dialog opens
    /// (@0x65f27), and the engine answers it on the next traffic tick,
    /// after the dialog: the dialog shows no ship's goal or target and the
    /// pilot can't be saved in it, so the lag crosses no boundary the
    /// player sees or keeps.
    pub(super) fn call_good_samaritans(&mut self, npc: &Npc) {
        if !calls_good_samaritan(npc) {
            return;
        }
        self.strikes.push(Strike {
            ship: ShipRef::Npc(npc.id),
            by: Some(ShipRef::Player),
            damage: 0.0,
            downed: None,
            direct: false,
            intended: true,
            lock: false,
            nominal: Nominal::default(),
            weight: 0.0,
            answered: true,
        });
    }

    /// Holds NPC `id`, boarded on a mission's assist path, on its course
    /// for [`BOARDED_WAIT`] ticks (@0x65746, see the module docs).
    pub(super) fn hold_boarded(&mut self, id: NpcId) {
        if let Some(npc) = self.npc_mut(id) {
            npc.wait_on_course(BOARDED_WAIT);
        }
    }
}

/// Whether boarding `npc` calls the Good Samaritans: unless a person
/// with a `LinkMission` flies it (@0x6585c-0x65861).
fn calls_good_samaritan(npc: &Npc) -> bool {
    npc.person.is_none_or(|person| !person.mission)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::Goal;
    use crate::board::{Boarding, NovaBoarding};
    use crate::catalog::{MissionId, ShipId};
    use crate::chance::NeverFires;
    use crate::combat::hull::Condition;
    use crate::geometry::Vec2;
    use crate::legal::NovaLaw;
    use crate::mission::FailedBy;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::session::mission_ships::tests::{
        DRIFT, board_at, board_first, cargo_aboard, in_flight_with, picking_up, rescuing, ships_of,
    };
    use crate::session::person_offers::tests as person;
    use crate::session::tests::{alongside, board_by, boardable};
    use crate::testkit::FakePilotCatalog;
    use crate::traffic::npc::{AiType, Mode};

    /// `catalog` with ship class `id` of no `Crew`.
    fn crewless(mut catalog: FakePilotCatalog, id: i16) -> FakePilotCatalog {
        for record in &mut catalog.ship_records {
            if record.id == ShipId(id) {
                record.crew = 0;
            }
        }
        catalog
    }

    /// The rulebook with [`RuleKey::CrewlessMissionShip`] read as `source`.
    fn crewless_rule(source: RuleSource) -> Rulebook {
        Rulebook::default().with_override(RuleKey::CrewlessMissionShip, source)
    }

    /// How many [`SimSound::Refused`] are among the sounds raised since the
    /// last taken.
    fn refusal_beeps(session: &mut Session) -> usize {
        session
            .take_sounds()
            .into_iter()
            .filter(|sound| *sound == SimSound::Refused)
            .count()
    }

    /// A board refused: why, how the session is set up for it, and the
    /// refusal.
    type Refusal = (&'static str, fn(&mut Session), BoardRefusal);

    /// Fails the mission in slot 0, leaving it there.
    fn fail(session: &mut Session) {
        if let Some(active) = session.pilot.missions.get_mut(0) {
            active.failed = Some(FailedBy::Marked);
        }
    }

    // The crew.

    #[test]
    fn a_mission_ship_of_no_crew_is_boarded_and_counted_and_by_the_other_reading_refused() {
        let catalog = crewless(rescuing(5, 1, 0x0001), 200);
        let mut session = in_flight_with(&catalog, 1);
        let (boarding, _) = board_first(&mut session);
        assert_eq!(
            boarding,
            Ok(Boarding::MissionShip(Some(MissionBoardLine::Boarded))),
            "@0x652bd skips the crew test"
        );
        assert_eq!(ships_of(&session).boarded, 1);

        let mut session = in_flight_with(&catalog, 1).with_rules(crewless_rule(RuleSource::Bible));
        let _ = session.take_sounds();
        let (boarding, _) = board_first(&mut session);
        assert_eq!(boarding, Err(BoardRefusal::CantBoard));
        assert_eq!(refusal_beeps(&mut session), 1);
        assert_eq!(ships_of(&session).boarded, 0);
        assert!(!session.npcs()[0].boarded);
    }

    #[test]
    fn a_ship_of_no_mission_and_no_crew_too_fast_to_board_says_so_by_the_engine() {
        let catalog = crewless(boardable(), 129);
        for (source, refusal) in [
            (RuleSource::Engine, BoardRefusal::TooFast),
            (RuleSource::Bible, BoardRefusal::CantBoard),
        ] {
            let mut session = alongside(&catalog).with_rules(crewless_rule(source));
            session.ship.player.velocity = Vec2::new(0.0, 0.6);
            assert_eq!(
                board_by(&mut session, &NovaLaw::default()),
                Err(refusal),
                "{source:?}"
            );
            let mut session = alongside(&catalog).with_rules(crewless_rule(source));
            assert_eq!(
                board_by(&mut session, &NovaLaw::default()),
                Err(BoardRefusal::CantBoard),
                "{source:?}: over it, at its speed"
            );
        }
    }

    // The player's condition.

    #[test]
    fn a_disabled_player_boards_and_a_dying_one_is_refused() {
        let catalog = boardable();
        let mut session = alongside(&catalog);
        session.set_player_condition(Condition::Disabled);
        assert!(matches!(
            board_by(&mut session, &NovaLaw::default()),
            Ok(Boarding::Opened(_))
        ));
        assert!(session.boarding().is_some(), "the plunder opens");
        let mut session = alongside(&catalog);
        session.set_player_condition(Condition::Dying {
            ticks_left: 10,
            starting: false,
        });
        let _ = session.take_sounds();
        assert_eq!(
            board_by(&mut session, &NovaLaw::default()),
            Err(BoardRefusal::CantBoard)
        );
        assert_eq!(session.take_sounds(), [SimSound::Refused]);
        assert!(!session.npcs()[0].boarded);
    }

    // The refusal beep.

    #[test]
    fn each_refusal_said_beeps_once() {
        let catalog = boardable();
        let refusals: [Refusal; 4] = [
            (
                "boarded already",
                |session| session.space.traffic.npcs_mut()[0].boarded = true,
                BoardRefusal::CantBoard,
            ),
            (
                "not disabled",
                |session| session.space.traffic.npcs_mut()[0].condition = Condition::Intact,
                BoardRefusal::CantBoard,
            ),
            (
                "too fast",
                |session| session.ship.player.velocity = Vec2::new(0.6, 0.0),
                BoardRefusal::TooFast,
            ),
            (
                "too far",
                |session| {
                    session.space.traffic.npcs_mut()[0].state.position = Vec2::new(0.0, 500.0);
                },
                BoardRefusal::TooFar,
            ),
        ];
        for (why, set_up, refusal) in refusals {
            let mut session = alongside(&catalog);
            set_up(&mut session);
            let _ = session.take_sounds();
            assert_eq!(
                board_by(&mut session, &NovaLaw::default()),
                Err(refusal),
                "{why}"
            );
            assert_eq!(session.take_sounds(), [SimSound::Refused], "{why}");
        }
        let catalog = crewless(boardable(), 129);
        let mut session = alongside(&catalog);
        let _ = session.take_sounds();
        assert_eq!(
            board_by(&mut session, &NovaLaw::default()),
            Err(BoardRefusal::CantBoard)
        );
        assert_eq!(session.take_sounds(), [SimSound::Refused], "no crew");
    }

    #[test]
    fn no_target_not_facing_it_or_a_landed_ship_is_refused_silently() {
        let catalog = boardable();
        let silent: [Refusal; 3] = [
            (
                "no target",
                |session| session.chart.target = None,
                BoardRefusal::NoTarget,
            ),
            (
                "not facing it",
                |session| session.ship.player.heading = 90.0,
                BoardRefusal::Misaligned,
            ),
            (
                "landed",
                |session| {
                    session.mode = crate::session::Mode::Landed(crate::catalog::StellarId(128));
                },
                BoardRefusal::NoTarget,
            ),
        ];
        for (why, set_up, refusal) in silent {
            let mut session = alongside(&catalog);
            set_up(&mut session);
            let _ = session.take_sounds();
            assert_eq!(
                board_by(&mut session, &NovaLaw::default()),
                Err(refusal),
                "{why}"
            );
            assert_eq!(session.take_sounds(), [], "{why}");
        }
    }

    #[test]
    fn a_pickup_refused_for_room_beeps_the_refusal_once() {
        let catalog = picking_up(2, 1, 0x0001);
        let mut session = in_flight_with(&catalog, 1);
        if let Some(active) = session.pilot.missions.get_mut(0) {
            active.cargo_tons = 100_000;
        }
        let _ = session.take_sounds();
        let (boarding, _) = board_at(&mut session, 0, DRIFT);
        assert_eq!(boarding, Err(BoardRefusal::CantBoard));
        assert_eq!(refusal_beeps(&mut session), 1, "@0x65366 → @0x65ffe");
    }

    // The Good Samaritan.

    #[test]
    fn boarding_a_person_with_a_link_mission_calls_no_good_samaritan() {
        let offered = person::boardable(0);
        let mut session = person::alongside_ace(&offered);
        assert!(matches!(
            person::board(&mut session).0,
            Ok(Boarding::Offered(_))
        ));
        assert_eq!(session.space.strikes, [], "the offer (@0x65907)");

        let mut unavailable = person::boardable(0);
        unavailable.missions[0].avail_random = 50;
        let mut session = person::alongside_ace(&unavailable);
        session.run.offers.set_roll(MissionId(130), 51);
        assert!(matches!(
            person::board(&mut session).0,
            Ok(Boarding::Opened(_))
        ));
        assert_eq!(session.space.strikes, [], "its plunder (@0x65991)");

        let mut hailed = person::boardable(0);
        hailed.persons[0].flags = 0;
        let mut session = person::alongside_ace(&hailed);
        assert!(matches!(
            person::board(&mut session).0,
            Ok(Boarding::Opened(_))
        ));
        assert_eq!(
            session.space.strikes,
            [],
            "without 0x0200 (@0x65a21 → @0x65ede)"
        );
    }

    /// The zero-damage strike by the player on NPC `npc` that calls the
    /// Good Samaritans.
    fn samaritan_call(npc: u32) -> Strike {
        Strike {
            ship: ShipRef::Npc(crate::traffic::npc::NpcId(npc)),
            by: Some(ShipRef::Player),
            damage: 0.0,
            downed: None,
            direct: false,
            intended: true,
            lock: false,
            nominal: Nominal::default(),
            weight: 0.0,
            answered: true,
        }
    }

    #[test]
    fn boarding_an_unnamed_ship_a_person_of_no_link_mission_or_a_failed_missions_ship_calls_it() {
        let catalog = boardable();
        let mut session = alongside(&catalog);
        board_by(&mut session, &NovaLaw::default()).expect("boards");
        assert_eq!(session.space.strikes, [samaritan_call(0)], "unnamed");

        let mut unlinked = person::boardable(0);
        unlinked.persons[0].link_mission = None;
        let mut session = person::alongside_ace(&unlinked);
        assert!(matches!(
            person::board(&mut session).0,
            Ok(Boarding::Opened(_))
        ));
        assert_eq!(session.space.strikes, [samaritan_call(0)], "no LinkMission");

        let catalog = rescuing(5, 1, 0x0001);
        let mut session = in_flight_with(&catalog, 1);
        fail(&mut session);
        let (boarding, _) = board_first(&mut session);
        assert!(matches!(boarding, Ok(Boarding::Opened(_))), "{boarding:?}");
        let id = session.npcs()[0].id;
        assert_eq!(
            session.space.strikes,
            [samaritan_call(id.0)],
            "a failed mission"
        );
    }

    /// [`alongside`]'s trader (NPC 0) escorting the police (NPC 1), an
    /// ally of its government, of `ai_type`, idle: boarded, its call
    /// answered.
    fn board_escort_of(ai_type: AiType) -> Session {
        let catalog = boardable();
        let mut session = alongside(&catalog);
        let npcs = session.space.traffic.npcs_mut();
        npcs[0].leader = Some(NpcId(1));
        npcs[1].ai_type = ai_type;
        npcs[1].goal = Goal::Idle;
        npcs[1].target = None;
        board_by(&mut session, &NovaLaw::default()).expect("boards");
        session.answer_strikes(&crate::ai::NovaAi::default());
        session
    }

    #[test]
    fn boarding_an_escort_leaves_its_trader_lead_alone_and_calls_its_warship_lead() {
        // `_DoGoodSamaritan` writes goal 4 and the target alone, on the
        // slots its gates pass (@0x82891-0x82ba8): no lead's reaction.
        let session = board_escort_of(AiType::WimpyTrader);
        let lead = &session.npcs()[1];
        assert_eq!((lead.target, lead.goal), (None, Goal::Idle), "a trader");
        let session = board_escort_of(AiType::Warship);
        let lead = &session.npcs()[1];
        assert_eq!(lead.target, Some(ShipRef::Player), "a warship");
        assert_eq!(lead.goal, Goal::Attack(ShipRef::Player), "a warship");
    }

    // A failed mission's ship.

    #[test]
    fn a_failed_missions_ship_on_the_assist_path_is_counted_says_its_line_and_is_plundered_uncharged()
     {
        let catalog = rescuing(5, 1, 0x0001);
        let mut session = in_flight_with(&catalog, 1);
        fail(&mut session);
        let _ = session.take_sounds();
        let (boarding, crimes) = board_first(&mut session);
        assert!(matches!(boarding, Ok(Boarding::Opened(_))), "{boarding:?}");
        assert!(session.boarding().is_some(), "plundered");
        assert_eq!(crimes, [], "no BoardPenalty: the slap passes the mark");
        assert_eq!(ships_of(&session).boarded, 1, "counted (@0x65730)");
        assert_eq!(
            session.take_board_line(),
            Some(MissionBoardLine::Boarded),
            "#126"
        );
        assert_eq!(session.take_board_line(), None, "once");
        assert_eq!(
            session.take_sounds(),
            [SimSound::MissionShipBoarded, SimSound::Boarded],
            "the line's beep, then the airlock"
        );
        assert!(session.npcs()[0].boarded);
        assert_eq!(
            session.npcs()[0].mode,
            Mode::Coasting {
                ticks_left: BOARDED_WAIT
            },
            "held, as the assist path holds any ship"
        );
    }

    #[test]
    fn a_failed_missions_pickup_ship_picks_up_counts_then_is_plundered() {
        let catalog = picking_up(2, 1, 0x0001);
        let mut session = in_flight_with(&catalog, 1);
        fail(&mut session);
        let (boarding, crimes) = board_at(&mut session, 0, DRIFT);
        assert!(matches!(boarding, Ok(Boarding::Opened(_))), "{boarding:?}");
        assert_eq!(crimes, []);
        assert!(cargo_aboard(&session), "picked up");
        assert_eq!(ships_of(&session).boarded, 1, "@0x653a7");
        assert_eq!(
            session.take_board_line(),
            Some(MissionBoardLine::Retrieved { cargo: Some(1) })
        );
        assert_eq!(session.ship.player.velocity, DRIFT);
        assert_eq!(session.npcs()[0].mode, Mode::Flying, "no hold");
    }

    #[test]
    fn a_board_with_no_line_to_say_holds_none_for_the_view() {
        let catalog = rescuing(5, 2, 0x0001);
        let mut session = in_flight_with(&catalog, 2);
        fail(&mut session);
        let (boarding, _) = board_first(&mut session);
        assert!(matches!(boarding, Ok(Boarding::Opened(_))), "{boarding:?}");
        assert_eq!(
            session.take_board_line(),
            None,
            "a failed mission's ship off both paths: two ships left"
        );
        let catalog = boardable();
        let mut session = alongside(&catalog);
        board_by(&mut session, &NovaLaw::default()).expect("boards");
        assert_eq!(session.take_board_line(), None, "a plain board");
        let catalog = rescuing(5, 1, 0x0001);
        let mut session = in_flight_with(&catalog, 1);
        let (boarding, _) = board_first(&mut session);
        assert_eq!(
            boarding,
            Ok(Boarding::MissionShip(Some(MissionBoardLine::Boarded)))
        );
        assert_eq!(session.take_board_line(), None, "said by the boarding");
    }

    #[test]
    fn put_over_a_ship_the_player_targets_it_at_its_place_speed_and_heading() {
        let catalog = boardable();
        let mut session = alongside(&catalog);
        session.chart.target = None;
        session.ship.player.position = Vec2::new(300.0, -40.0);
        session.ship.player.heading = 90.0;
        let police = session.npcs()[1].clone();
        crate::fixture::put_player_over(&mut session, police.id);
        assert_eq!(session.target().map(|npc| npc.id), Some(police.id));
        assert_eq!(*session.player(), police.state);
        crate::fixture::put_player_over(&mut session, crate::traffic::npc::NpcId(99));
        assert_eq!(*session.player(), police.state, "no such ship");
    }

    #[test]
    fn a_second_board_forgets_the_line_the_first_left() {
        let catalog = rescuing(5, 1, 0x0001);
        let mut session = in_flight_with(&catalog, 1);
        fail(&mut session);
        let _ = board_first(&mut session);
        session.chart.target = None;
        assert_eq!(
            session.board(
                &NovaLaw::default(),
                &NovaBoarding::default(),
                &mut NeverFires
            ),
            Err(BoardRefusal::NoTarget)
        );
        assert_eq!(session.take_board_line(), None);
    }

    // The assist path's hold.

    #[test]
    fn a_ship_boarded_on_the_assist_path_holds_still_for_100_ticks_then_flies_on() {
        let catalog = rescuing(5, 1, 0x0001);
        let mut session = in_flight_with(&catalog, 1);
        // A rescue ship held disabled at its full armour, so intact once
        // boarded.
        let armor = &mut session.space.traffic.npcs_mut()[0].reserves.armor;
        armor.now = armor.max;
        let (boarding, _) = board_first(&mut session);
        assert_eq!(
            boarding,
            Ok(Boarding::MissionShip(Some(MissionBoardLine::Boarded)))
        );
        assert_eq!(BOARDED_WAIT, 100);
        let held = session.npcs()[0].clone();
        assert_eq!(held.mode, Mode::Coasting { ticks_left: 100 });
        let tick = |session: &mut Session| {
            session.tick_traffic(&catalog, &crate::ai::NovaAi::default(), &mut NeverFires);
            session.tick_combat(crate::combat::Rules::default(), &mut NeverFires);
        };
        for at in 0..BOARDED_WAIT {
            assert!(session.npcs()[0].wait_running(), "{at}");
            tick(&mut session);
            let npc = &session.npcs()[0];
            assert_eq!(npc.condition, Condition::Intact, "{at}: no longer held");
            assert_eq!(npc.state.heading, held.state.heading, "{at}: no turn");
            assert_eq!(npc.state.velocity, held.state.velocity, "{at}: no thrust");
            assert_eq!(npc.state.position, held.state.position, "{at}: still");
            assert_eq!(npc.goal, held.goal, "{at}");
        }
        assert_eq!(session.npcs()[0].mode, Mode::Flying, "the wait is out");
        let moved = (0..300).any(|_| {
            tick(&mut session);
            session
                .npcs()
                .first()
                .is_none_or(|npc| npc.state != held.state)
        });
        assert!(moved, "flies on");
    }

    #[test]
    fn a_pickup_ship_boarded_is_not_held() {
        let catalog = picking_up(2, 1, 0x0001);
        let mut session = in_flight_with(&catalog, 1);
        let (boarding, _) = board_first(&mut session);
        assert!(
            matches!(boarding, Ok(Boarding::MissionShip(Some(_)))),
            "{boarding:?}"
        );
        assert_eq!(
            session.npcs()[0].mode,
            Mode::Flying,
            "@0x652ee sets no wait"
        );
    }
}
