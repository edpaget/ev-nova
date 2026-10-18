//! The message line in flight: the session's [`CommLine`], `_commCount`,
//! and the calls the NPCs make on it (`_AICallForHelp` @0x82ffb-0x83257
//! and `_InterceptorAI`'s pre-scan hail @0x8ce4b-0x8cff5 in the `EV Nova`
//! executable).
//!
//! **The line.** The session keeps the one count every reader asks:
//!
//! - Each message it raises sets it as it is raised ([`Session::say`],
//!   [`SimMessage::ticks`]); so do the hail quotes (420, see the
//!   `persons` module) and the assisting ships' comm notes
//!   ([`CommNote::ticks`](crate::hail::CommNote::ticks)). The flight
//!   relays each for the ticks the session set, [`SimMessage::ticks`],
//!   [`PersonQuote::ticks`](super::PersonQuote::ticks) and
//!   [`CommNote::ticks`](crate::hail::CommNote::ticks).
//! - The flight's own lines, the ones the view raises and words itself
//!   (a refused jump, a boarding's line, an escort command, a line it
//!   composes at an arrival or a take-off), set it through
//!   [`Session::comm_shown`]. The view relays the session's own lines
//!   without setting it again: a relay comes after the steps of a frame
//!   have run, and in a fixed order, so setting it then would undo the
//!   countdown and the order the session raised them in.
//! - Return clears the line for a tick while one shows
//!   ([`Session::comm_acknowledged`], `_HandlePlayer` @0x6dd22-0x6dd65);
//!   a clear by name shows an empty line for a tick
//!   ([`Session::comm_shown`] of 1, @0x6b231-0x6b30d, @0x6dedb-0x6df29).
//! - A landing sets it to 0 (`_HandlePlayerDockRequest` @0x66d7f), and a
//!   flight starts at 0 (`_ResetPlayer` @0x1d904). No jump, gate or
//!   arrival writes it: an arrival's own line sets it.
//! - It counts down once a frame on the advance pass, at the head of the
//!   hail quotes' tick ([`Session::tick_quotes`]), never while landed,
//!   as `_HandleComm` runs on the advance pass only (`_DoPlayGameWork`
//!   @0x4421e). The original counts down at the head of its frame, before
//!   any ship thinks; ours counts down after the frame's traffic and
//!   fight, before the quotes, so a line raised by the frame's traffic
//!   or fight is read by the next frame's calls one tick shorter, as the
//!   original's is, and a hail quote one tick longer: a one-frame lag
//!   before a call that crosses no boundary the player sees.
//!
//! **The calls.** Each think's call ([`Decision::comm_call`](crate::ai::Decision::comm_call),
//! gathered by [`Traffic::take_comm_calls`](crate::traffic::Traffic::take_comm_calls))
//! is answered in [`Session::answer_comm_calls`], in NPC order, right
//! after the frame's decisions and fleet arrivals, so one call's line
//! keeps the next one in the same frame off the line:
//!
//! - **A distress call** is made when [`calls_for_help`] says so of the
//!   ship's facts as the session finds them ([`DistressFacts`]): its
//!   name ([`Session::npc_name`]: a mission's drawn ship name when not
//!   empty, else its class's; persons never call), a colon, two spaces
//!   and line `1 + Rand(20)` of `STR#` 5003, a missing line read as
//!   empty, with [`SimSound::CallForHelp`]
//!   ([`SimMessage::CallForHelp`], 240 ticks).
//! - **A pre-scan hail** is made when the interceptor has a government,
//!   the player carries mission cargo it scans for
//!   ([`hail_finds`](crate::mission::hail_finds)), and the line is
//!   idle: "<class>:  <ship name>, " and `STR#` 2002 #(381 + `Rand(3)`)
//!   and ".", with [`SimSound::ScanHail`] ([`SimMessage::ScanHail`], 400
//!   ticks). The player's ship name is `<PSN>`'s.
//! - **A call for reinforcements** ([`CommCall::Reinforcements`], after
//!   the think's own call) is `_AICallForReinforcements(ship, 0)`, which
//!   says nothing on the line itself (see the `reinforcements` module).
//!
//! The original makes each call inside its ship's think, against the
//! ships as the earlier slots' thinks left them; the engine answers the
//! frame's calls together, against the ships as the frame's traffic left
//! them, before the frame is drawn. A ship that left the system in the
//! frame calls nothing. A scan line from an earlier slot that would keep
//! a call off the line in the original is answered later here, in
//! [`Session::tick_combat`]: rdm:phase/combat-ai-fidelity/phase-15-comm-calls-in-slot-order.

use super::Session;
use crate::ai::CommCall;
use crate::ai::call_for_help::{DistressFacts, calls_for_help, threatens_player};
use crate::chance::Chance;
use crate::combat::hull::Condition;
use crate::comm_line::CommLine;
use crate::hail::{QUIET, likes_player};
use crate::message::SimMessage;
use crate::mission::hail_finds;
use crate::sound::SimSound;
use crate::traffic::npc::NpcId;

/// The `STR#` of the twenty distress lines, `_shipHelpRequests`
/// (`_LoadStrings` @0x71bf3-0x71c27).
pub const DISTRESS_LINES: i16 = 5003;

/// How many distress lines there are: `Rand(20)` (@0x831d6).
pub const DISTRESS_LINE_COUNT: u32 = 20;

/// The `STR#` whose entries #381-383 are the pre-scan hail's words
/// (0x7d2 @0x8cf6e).
pub const SCAN_HAIL_LINES: i16 = 2002;

/// The first of the pre-scan hail's entries: #381 (0x17d @0x8cf79), the
/// others following.
pub const SCAN_HAIL_FIRST: usize = 381;

/// How many entries the pre-scan hail picks among: `Rand(3)` (@0x8cf62).
pub const SCAN_HAIL_COUNT: u32 = 3;

impl Session {
    /// Raises `message` on the message line, setting the line's count to
    /// its ticks as `_DisplayComm` does (@0x9024b).
    pub(super) fn say(&mut self, message: SimMessage) {
        self.comm_line.show(message.ticks());
        self.messages.push(message);
    }

    /// The flight shows a line of its own for `ticks`: one the view
    /// raises and words itself, or composes from the session's (see the
    /// module docs). A line the session raised is not shown again here.
    pub fn comm_shown(&mut self, ticks: u16) {
        self.comm_line.show(ticks);
    }

    /// Return (`ackComm`) clears the message line: for a tick, while a
    /// line shows (`_HandlePlayer` @0x6dd22-0x6dd65).
    pub fn comm_acknowledged(&mut self) {
        self.comm_line.acknowledge();
    }

    /// The message line's countdown, as the calls read it.
    #[must_use]
    pub fn comm_line(&self) -> CommLine {
        self.comm_line
    }

    /// The frame's countdown of the message line (see the module docs).
    pub(super) fn count_down_comm_line(&mut self) {
        self.comm_line.tick();
    }

    /// A landing sets the line's count to 0 (`_HandlePlayerDockRequest`
    /// @0x66d7f).
    pub(super) fn reset_comm_line(&mut self) {
        self.comm_line.reset();
    }

    /// Answers the calls the traffic's decisions made since they were
    /// last taken, in order, each drawing on `chance` (see the module
    /// docs).
    pub(super) fn answer_comm_calls(&mut self, chance: &mut (impl Chance + ?Sized)) {
        for (id, call) in self.traffic.take_comm_calls() {
            match call {
                CommCall::Distress => self.call_for_help(id, chance),
                CommCall::PreScan => self.hail_before_scan(id, chance),
                CommCall::Reinforcements => self.call_for_reinforcements(id, false),
            }
        }
    }

    /// What `_AICallForHelp` asks of NPC `id` now, if it is in the
    /// system.
    fn distress_facts(&self, id: NpcId) -> Option<DistressFacts> {
        let world = self.world();
        let around = world.around(self.npcs());
        let npc = around.npc(id)?;
        let inherent = self
            .ship_record(npc.ship)
            .and_then(|record| record.inherent_govt);
        Some(DistressFacts {
            person: npc.person.is_some(),
            quiet: self.govts.flag2(npc.govt, QUIET),
            quiet_inherent: self.govts.flag2(inherent, QUIET),
            line_free: self.comm_line.free_for_calls(),
            liked: likes_player(npc, &around, self.mission_hail(npc)),
            threatens: threatens_player(npc, &around, self.condition == Condition::Intact),
            player_dying: matches!(
                self.condition,
                Condition::Dying { .. } | Condition::Destroyed
            ),
            cloaked: npc.cloak.is_cloaked(),
        })
    }

    /// NPC `id`'s distress call, when [`calls_for_help`] says so.
    fn call_for_help(&mut self, id: NpcId, chance: &mut (impl Chance + ?Sized)) {
        if !self
            .distress_facts(id)
            .is_some_and(|facts| calls_for_help(&facts))
        {
            return;
        }
        let Some(npc) = self.npcs().iter().find(|npc| npc.id == id) else {
            return;
        };
        let name = self.npc_name(npc).unwrap_or_default().to_owned();
        let pick = chance.below(DISTRESS_LINE_COUNT) as usize;
        let line = self
            .nth_string(DISTRESS_LINES, 1 + pick)
            .unwrap_or_default();
        self.sounds.push(SimSound::CallForHelp);
        self.say(SimMessage::CallForHelp(format!("{name}:  {line}")));
    }

    /// Interceptor `id`'s pre-scan hail, when it has a government, the
    /// player carries mission cargo that government scans for, and the
    /// line is idle.
    fn hail_before_scan(&mut self, id: NpcId, chance: &mut (impl Chance + ?Sized)) {
        let Some(npc) = self.npcs().iter().find(|npc| npc.id == id) else {
            return;
        };
        let Some(govt) = npc.govt.and_then(|govt| self.govts.get(govt)) else {
            return;
        };
        let found = hail_finds(govt, &self.pilot.missions, |mission| {
            self.lifecycle_record(mission)
        });
        if !found || !self.comm_line.idle() {
            return;
        }
        let class = self.ship_name(npc.ship).unwrap_or_default();
        let ship_name = self.pilot.ship_name().unwrap_or_default();
        let pick = chance.below(SCAN_HAIL_COUNT) as usize;
        let words = self
            .nth_string(SCAN_HAIL_LINES, SCAN_HAIL_FIRST + pick)
            .unwrap_or_default();
        let text = format!("{class}:  {ship_name}, {words}.");
        self.sounds.push(SimSound::ScanHail);
        self.say(SimMessage::ScanHail(text));
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::rc::Rc;

    use super::*;
    use crate::ai::{Behaviour, Decision, Goal, Surroundings};
    use crate::catalog::{
        CommCatalog, GovtId, GovtRecord, MissionRecord, MissionShip, ShipId, ShipRecord,
    };
    use crate::chance::NeverFires;
    use crate::combat::ShipRef;
    use crate::mission::{ActiveMission, SpecialShips};
    use crate::pilot::Pilot;
    use crate::testkit::{
        Draws, FAST, FakePilotCatalog, carrying, catalog, govt, mission, npc, ship,
    };
    use crate::traffic::ShipBehav;
    use crate::traffic::npc::{AiType, MissionMark, Npc};

    /// Traders, scanning for bit 0x0004.
    const TRADERS: GovtId = GovtId(128);
    /// A quiet government (`Flags2` 0x0008).
    const QUIETS: GovtId = GovtId(129);
    /// The class of the ships that call.
    const CALLER: ShipId = ShipId(129);
    /// A class whose inherent government is quiet.
    const QUIET_CLASS: ShipId = ShipId(130);

    /// `STR#` 5003 "Help 1" to "Help 20", and 2002 #381-383 "halt",
    /// "stop" and "hold".
    #[derive(Debug)]
    struct Lines;

    impl CommCatalog for Lines {
        fn string_list(&self, id: i16) -> Vec<String> {
            match id {
                DISTRESS_LINES => (1..=20).map(|n| format!("Help {n}")).collect(),
                SCAN_HAIL_LINES => (1..=383)
                    .map(|n| match n {
                        381 => "halt".to_owned(),
                        382 => "stop".to_owned(),
                        383 => "hold".to_owned(),
                        _ => String::new(),
                    })
                    .collect(),
                _ => Vec::new(),
            }
        }
    }

    /// Each NPC listed decides its goal and makes its call; the others
    /// idle, calling nothing.
    #[derive(Debug, Default)]
    struct Calling(BTreeMap<NpcId, (Goal, CommCall)>);

    impl Behaviour for Calling {
        fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
            self.decision(npc, around, chance).goal
        }

        fn decision(
            &self,
            npc: &Npc,
            _around: &Surroundings,
            _chance: &mut dyn Chance,
        ) -> Decision {
            let (goal, call) = self
                .0
                .get(&npc.id)
                .map_or((Goal::Idle, None), |&(goal, call)| (goal, Some(call)));
            Decision {
                goal,
                comm_call: call,
                ..Decision::default()
            }
        }
    }

    /// The catalog: ships 128-130 (130's inherent government quiet), the
    /// traders and the quiet government, and mission 140 escorting the
    /// player's way (`ShipGoal` 3), its cargo masked 0x0006.
    fn calling_catalog() -> FakePilotCatalog {
        FakePilotCatalog {
            ship_records: vec![
                ship(128, FAST),
                ship(129, FAST),
                ShipRecord {
                    inherent_govt: Some(QUIETS),
                    ..ship(130, FAST)
                },
            ],
            govts: vec![
                GovtRecord {
                    scan_mask: 0x0004,
                    ..govt(128)
                },
                GovtRecord {
                    flags2: QUIET,
                    ..govt(129)
                },
            ],
            missions: vec![MissionRecord {
                scan_mask: 0x0006,
                ships: MissionShip {
                    goal: 3,
                    ..mission(140).ships
                },
                ..mission(140)
            }],
            ..catalog()
        }
    }

    /// A flight of `catalog` with the distress and scan lines, the line
    /// free and nothing raised yet.
    fn flying(catalog: &FakePilotCatalog) -> Session {
        let pilot = Pilot::new(catalog, "Ada").expect("a pilot");
        let mut session = Session::fly(catalog, pilot)
            .expect("flies")
            .with_strings(Rc::new(Lines));
        session.tick_traffic(catalog, &Peaceful, &mut NeverFires);
        let _ = (session.take_messages(), session.take_sounds());
        session.comm_line = CommLine::default();
        session
    }

    use crate::ai::Peaceful;

    /// Adds an NPC of `govt` flying `class`, at the player's side.
    fn add(session: &mut Session, govt: Option<GovtId>, class: ShipId) -> NpcId {
        let mut ship = npc(0, session.stats());
        ship.govt = govt;
        ship.ship = class;
        ship.state.position = session.player.position;
        session.traffic.add_npc(ship)
    }

    /// A trader of `govt` and `class` fleeing a pirate, calling for help;
    /// and the behaviour that has it flee.
    fn fleeing(session: &mut Session, govt: Option<GovtId>, class: ShipId) -> (NpcId, Calling) {
        let pirate = add(session, None, ShipId(128));
        let trader = add(session, govt, class);
        let flees = (Goal::Flee(ShipRef::Npc(pirate)), CommCall::Distress);
        (trader, Calling(BTreeMap::from([(trader, flees)])))
    }

    /// A traffic tick of `catalog` as `behaviour` decides, drawing
    /// `draws`: the messages and sounds it raised, and the bounds asked.
    fn ticked(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        behaviour: &Calling,
        draws: &[u32],
    ) -> (Vec<SimMessage>, Vec<SimSound>, Vec<u32>) {
        let mut chance = Draws::of(draws);
        session.tick_traffic(catalog, behaviour, &mut chance);
        (session.take_messages(), session.take_sounds(), chance.asked)
    }

    fn help(text: &str) -> Vec<SimMessage> {
        vec![SimMessage::CallForHelp(text.to_owned())]
    }

    #[test]
    fn a_fleeing_ship_that_likes_the_player_calls_for_help_from_str_5003() {
        let catalog = calling_catalog();
        let mut session = flying(&catalog);
        let (_, calls) = fleeing(&mut session, Some(TRADERS), CALLER);
        let (messages, sounds, asked) = ticked(&mut session, &catalog, &calls, &[19]);
        assert_eq!(messages, help("Ship 129:  Help 20"), "line 1 + Rand(20)");
        assert_eq!(sounds, [SimSound::CallForHelp]);
        assert_eq!(asked, [DISTRESS_LINE_COUNT]);
        assert!(!session.comm_line().free_for_calls(), "the line shows it");
        let mut first = flying(&catalog);
        let (_, calls) = fleeing(&mut first, Some(TRADERS), CALLER);
        assert_eq!(
            ticked(&mut first, &catalog, &calls, &[0]).0,
            help("Ship 129:  Help 1")
        );
    }

    #[test]
    fn a_line_showing_keeps_the_call_off_until_it_counts_down_to_0() {
        let catalog = calling_catalog();
        let mut session = flying(&catalog);
        let (_, calls) = fleeing(&mut session, Some(TRADERS), CALLER);
        session.comm_shown(2);
        assert_eq!(ticked(&mut session, &catalog, &calls, &[0]).0, []);
        session.tick_quotes(&catalog, &mut NeverFires);
        assert_eq!(ticked(&mut session, &catalog, &calls, &[0]).0, [], "at 1");
        session.tick_quotes(&catalog, &mut NeverFires);
        let (messages, sounds, _) = ticked(&mut session, &catalog, &calls, &[0]);
        assert_eq!(messages, help("Ship 129:  Help 1"), "at 0");
        assert_eq!(sounds, [SimSound::CallForHelp]);
    }

    #[test]
    fn of_two_ships_calling_in_one_tick_only_the_first_speaks() {
        let catalog = calling_catalog();
        let mut session = flying(&catalog);
        let (first, mut calls) = fleeing(&mut session, Some(TRADERS), CALLER);
        let second = add(&mut session, Some(TRADERS), ShipId(128));
        let flees = calls.0[&first];
        calls.0.insert(second, flees);
        let (messages, sounds, asked) = ticked(&mut session, &catalog, &calls, &[2, 3]);
        assert_eq!(messages, help("Ship 129:  Help 3"));
        assert_eq!(sounds, [SimSound::CallForHelp]);
        assert_eq!(asked, [DISTRESS_LINE_COUNT], "the second draws nothing");
    }

    #[test]
    fn a_fleet_quote_in_the_same_tick_keeps_the_call_off() {
        let catalog = calling_catalog();
        let mut session = flying(&catalog);
        let (_, calls) = fleeing(&mut session, Some(TRADERS), CALLER);
        session.say(SimMessage::FleetArrivalQuote("Hail".to_owned()));
        let _ = session.take_messages();
        assert_eq!(ticked(&mut session, &catalog, &calls, &[0]).0, []);
    }

    #[test]
    fn each_fact_the_session_finds_can_keep_the_call_off() {
        let catalog = calling_catalog();
        let silent = |setup: &dyn Fn(&mut Session, NpcId), govt, class, why: &str| {
            let mut session = flying(&catalog);
            let (trader, calls) = fleeing(&mut session, govt, class);
            setup(&mut session, trader);
            let (messages, sounds, asked) = ticked(&mut session, &catalog, &calls, &[0]);
            assert_eq!((messages, sounds, asked), (vec![], vec![], vec![]), "{why}");
        };
        let nothing = |_: &mut Session, _| {};
        silent(&nothing, Some(QUIETS), CALLER, "a quiet government");
        silent(
            &nothing,
            Some(TRADERS),
            QUIET_CLASS,
            "a quiet inherent government",
        );
        silent(
            &|session, trader| {
                session.npc_mut(trader).expect("there").person =
                    Some(crate::traffic::npc::NpcPerson {
                        id: crate::catalog::PersonId(600),
                        flags: 0,
                        coward: 0,
                        comm_quote: -1,
                        hail_quote: -1,
                        mission: false,
                        portrait: None,
                        invincible: false,
                        grudge: false,
                        quoted: false,
                        quoted_at: None,
                    });
            },
            Some(TRADERS),
            CALLER,
            "a person",
        );
        silent(
            &|session, trader| {
                session.npc_mut(trader).expect("there").cloak = crate::cloak::Cloak {
                    fade: crate::cloak::FADE_FULL,
                    direction: 0,
                };
            },
            Some(TRADERS),
            CALLER,
            "cloaked",
        );
        silent(
            &|session, _| session.condition = Condition::Destroyed,
            Some(TRADERS),
            CALLER,
            "the player destroyed",
        );
        silent(
            &|session, _| {
                session.condition = Condition::Dying {
                    ticks_left: 10,
                    starting: false,
                };
            },
            Some(TRADERS),
            CALLER,
            "the player breaking up",
        );
        silent(
            &|session, trader| {
                session.pilot.missions.put(0, named("Rover"));
                session.npc_mut(trader).expect("there").mission = Some(MissionMark {
                    slot: 0,
                    behav: ShipBehav::AttackPlayer,
                    formation: None,
                    order: None,
                });
            },
            Some(TRADERS),
            CALLER,
            "disliked: a mission ship not escorting the player",
        );
    }

    /// Mission 140 in slot 0 with its special ships named `name`.
    fn named(name: &str) -> ActiveMission {
        ActiveMission {
            ships: SpecialShips {
                name: name.to_owned(),
                ..SpecialShips::default()
            },
            ..carrying(140, 4)
        }
    }

    #[test]
    fn a_mission_ship_signs_with_its_drawn_name_unless_it_is_empty() {
        let catalog = calling_catalog();
        for (name, signed) in [("Rover", "Rover"), ("", "Ship 129")] {
            let mut session = flying(&catalog);
            session.pilot.missions.put(0, named(name));
            let (trader, calls) = fleeing(&mut session, Some(TRADERS), CALLER);
            session.npc_mut(trader).expect("there").mission = Some(MissionMark {
                slot: 0,
                behav: ShipBehav::EscortPlayer,
                formation: None,
                order: None,
            });
            assert_eq!(
                ticked(&mut session, &catalog, &calls, &[0]).0,
                help(&format!("{signed}:  Help 1")),
                "{name:?}"
            );
        }
    }

    /// An interceptor of `govt` picking the player afresh, the player
    /// carrying `cargo` in slot 0, the line idle (-1) once `counted`, at
    /// 0 otherwise, drawing `draws`.
    fn hailed(
        govt: Option<GovtId>,
        cargo: Option<ActiveMission>,
        counted: bool,
        draws: &[u32],
    ) -> (Vec<SimMessage>, Vec<SimSound>, Vec<u32>) {
        let catalog = calling_catalog();
        let mut session = flying(&catalog);
        if let Some(cargo) = cargo {
            session.pilot.missions.put(0, cargo);
        }
        let police = add(&mut session, govt, CALLER);
        session.npc_mut(police).expect("there").ai_type = AiType::Interceptor;
        if counted {
            session.count_down_comm_line();
        }
        let picks = (Goal::Inspect(ShipRef::Player), CommCall::PreScan);
        ticked(
            &mut session,
            &catalog,
            &Calling(BTreeMap::from([(police, picks)])),
            draws,
        )
    }

    #[test]
    fn an_interceptor_picking_a_smuggler_hails_it_before_the_scan() {
        let (messages, sounds, asked) = hailed(Some(TRADERS), Some(carrying(140, 4)), true, &[1]);
        assert_eq!(
            messages,
            [SimMessage::ScanHail(
                "Ship 129:  Ship 128, stop.".to_owned()
            )]
        );
        assert_eq!(sounds, [SimSound::ScanHail]);
        assert_eq!(asked, [SCAN_HAIL_COUNT]);
        let untyped = ActiveMission {
            cargo_type: None,
            ..carrying(140, 4)
        };
        assert_eq!(
            hailed(Some(TRADERS), Some(untyped), true, &[2]).0,
            [SimMessage::ScanHail(
                "Ship 129:  Ship 128, hold.".to_owned()
            )],
            "no cargo-type test"
        );
    }

    #[test]
    fn the_pre_scan_hail_needs_a_government_cargo_aboard_and_an_idle_line() {
        let quiet = (vec![], vec![], vec![]);
        assert_eq!(
            hailed(None, Some(carrying(140, 4)), true, &[0]),
            quiet,
            "no government"
        );
        assert_eq!(hailed(Some(TRADERS), None, true, &[0]), quiet, "no cargo");
        let away = ActiveMission {
            aboard: false,
            ..carrying(140, 4)
        };
        assert_eq!(
            hailed(Some(TRADERS), Some(away), true, &[0]),
            quiet,
            "not aboard"
        );
        assert_eq!(
            hailed(Some(QUIETS), Some(carrying(140, 4)), true, &[0]),
            quiet,
            "a mask sharing no bit"
        );
        assert_eq!(
            hailed(Some(TRADERS), Some(carrying(140, 4)), false, &[0]),
            quiet,
            "the line at 0, not yet idle"
        );
    }

    #[test]
    fn the_flights_own_lines_and_return_set_the_line() {
        let catalog = calling_catalog();
        let mut session = flying(&catalog);
        session.comm_shown(240);
        assert!(!session.comm_line().free_for_calls());
        session.comm_acknowledged();
        assert!(!session.comm_line().free_for_calls(), "1 tick left");
        session.tick_quotes(&catalog, &mut NeverFires);
        assert!(session.comm_line().free_for_calls(), "Return freed it");
        session.comm_acknowledged();
        session.tick_quotes(&catalog, &mut NeverFires);
        assert!(
            session.comm_line().idle(),
            "nothing showing: Return leaves it"
        );
    }

    #[test]
    fn a_landing_sets_the_line_to_0() {
        let catalog = calling_catalog();
        let mut session = flying(&catalog);
        session.comm_shown(240);
        crate::testkit::land_now(&mut session).expect("lands");
        assert_eq!(session.comm_line(), CommLine::default(), "@0x66d7f");
    }

    #[test]
    fn a_message_the_session_raises_sets_the_line_to_its_ticks() {
        let catalog = calling_catalog();
        let mut session = flying(&catalog);
        session.say(SimMessage::ScanHail(String::new()));
        for _ in 0..399 {
            session.tick_quotes(&catalog, &mut NeverFires);
        }
        assert!(!session.comm_line().free_for_calls(), "399 counted");
        session.tick_quotes(&catalog, &mut NeverFires);
        assert!(session.comm_line().free_for_calls(), "400 counted");
    }
}
