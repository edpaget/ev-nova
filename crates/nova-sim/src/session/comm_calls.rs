//! The message line in flight: the session's [`CommLine`], `_commCount`,
//! and the calls the NPCs make on it (`_AICallForHelp` @0x82ffb-0x83257
//! and `_InterceptorAI`'s pre-scan hail @0x8ce4b-0x8cff5 in the `EV Nova`
//! executable).
//!
//! **The line.** The session keeps the one count every reader asks:
//!
//! - Each message it raises sets it as it is raised ([`Space::say`],
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
//! queued on the line, [`Traffic::take_on_line`](crate::traffic::Traffic::take_on_line))
//! is answered in slot order, so one call's line keeps the next one in the
//! same frame off the line (see "Slot order" below):
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
//! **Slot order.** The original raises every line of the frame's ships
//! inside their thinks, one slot after another in ascending order
//! (`_DoPlayGameWork` @0x44270-0x442ae), after `_HandlePlayer`
//! (@0x43e19), and each through `_DisplayComm`, which sets `_commCount`
//! at once (@0x9024b): a ship's calls (`_AICallForHelp` @0x83248,
//! `_InterceptorAI` @0x8cff5), then its scan of the player
//! (`_ScanPlayer` from @0x8f11e-0x8f38a), its refuel note (@0x8f6a1),
//! and its repair note, plunder lines and dock (`_LowLevelAIHandler`
//! @0x86ef4, `_AIPlunderShipContents` @0x84791 and @0x84941, the dock
//! @0x86fc7). So each line keeps a later slot's call off the line, and a
//! later slot's line replaces an earlier one's. The traffic queues all of
//! it at each ship's berth (see `nova_sim::traffic::on_line`), the
//! refuel note among it (see `nova_sim::traffic::refuel_dock`), and
//! [`Session::settle_line_queue`] settles it after `_HandlePlayer`'s part
//! of the frame (the propulsion repair, the sabotage, and the eject in
//! the player's display step), so their lines come before every call, as
//! the original's do, and the scans' law is at hand: in
//! [`Session::game_work`] after `_AIDispatch`'s loop
//! (`Session::dispatch_npcs`), the fighters launched and thinking in it
//! among the slots, and again after the ship loop for the docks and
//! repairs the fly half made:
//!
//! - an earlier slot's scan line, stolen line, refuel or repair note
//!   keeps a later slot's distress call and pre-scan hail off, and their
//!   `snd ` 154, while a call from an earlier slot still shows and a
//!   later slot's line replaces it;
//! - the ships are kept in the order they came, not by slot, so the
//!   queue sorts its work by berth: a ship that came later into a lower
//!   berth is answered first.
//! - The reinforcement clock ticks after the frame's calls, as
//!   `_HandleReinforcements` (@0x44504) runs after the `_AIDispatch` loop
//!   (@0x4429d), so on a frame whose line work is queued its tick is held
//!   too, and runs right after it settles: a `ReinfTime` 1 call's fleet
//!   still arrives in its frame, and the frame's announcement still
//!   follows its calls and refuel note.
//!
//! The lines the view raises from `_HandlePlayer`'s keys (a jump's
//! refusal, an escort command, Return) set the line before the frame's
//! steps, so before every call too. The attack quotes a think raises
//! (`_ShowPersonHailQuote` via `_AIMakeShipAttackPlayer` @0x89ce1) are
//! still said after the fight: rdm:phase/combat-ai-fidelity/phase-28-left-out-of-phase-22.
//! A ship that left the system in the frame calls nothing.

use super::{Outbox, Session, Setup, Space, Statics};
use crate::ai::CommCall;
use crate::ai::call_for_help::{DistressFacts, calls_for_help, threatens_player};
use crate::chance::Chance;
use crate::combat::hull::Condition;
use crate::comm_line::CommLine;
use crate::hail::{Help, QUIET, likes_player};
use crate::legal::LegalCode;
use crate::message::SimMessage;
use crate::mission::hail_finds;
use crate::pilot::Pilot;
use crate::sound::SimSound;
use crate::traffic::npc::NpcId;
use crate::traffic::on_line::OnLine;

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
    /// The flight shows a line of its own for `ticks`: one the view
    /// raises and words itself, or composes from the session's (see the
    /// module docs). A line the session raised is not shown again here.
    pub(crate) fn comm_shown(&mut self, ticks: u16) {
        self.space.comm_shown(ticks);
    }

    /// Return (`ackComm`) clears the message line: for a tick, while a
    /// line shows (`_HandlePlayer` @0x6dd22-0x6dd65).
    pub fn comm_acknowledged(&mut self) {
        self.space.comm_acknowledged();
    }

    /// The message line's countdown, as the calls read it.
    #[must_use]
    pub fn comm_line(&self) -> CommLine {
        self.space.comm_line()
    }

    /// The flight counts the message line down a tick of a pass it
    /// shows a line in: the plunder dialog's loop, whose `_HandleComm`
    /// (@0x949a1) comes after a take's line, which the flight words and
    /// shows ([`Session::comm_shown`]) after the session's own pass (see
    /// the session's `plunder_loop` module).
    pub fn comm_ticked(&mut self) {
        self.space.comm_ticked();
    }

    /// Answers NPC `id`'s `call`, made by a think that read the `odds`
    /// against it, drawing on `chance`.
    fn answer_call(
        &mut self,
        id: NpcId,
        call: CommCall,
        odds: f32,
        chance: &mut (impl Chance + ?Sized),
    ) {
        match call {
            CommCall::Distress => self.call_for_help(id, chance),
            CommCall::PreScan => self.space.hail_before_scan(
                &self.pilot,
                &self.statics,
                &self.setup,
                &mut self.out,
                id,
                chance,
            ),
            CommCall::Reinforcements => self.call_for_reinforcements(id, false, Some(odds)),
        }
    }

    /// Settles the frame's work on the message line
    /// ([`Traffic::take_on_line`](crate::traffic::Traffic::take_on_line)),
    /// in slot order: each scan of the player ([`Session::scanned_by`]),
    /// convicting by `law`; each call; each pirate's or repairer's deed
    /// ([`Session::answer_pirate`]); and each refuel's note, drawing on
    /// `chance`, so an earlier slot's line keeps a later slot's call off
    /// as the original's does (see the module docs). Then the
    /// reinforcement clock's tick held behind them runs, if one was (see
    /// the `reinforcements` module). Nothing settles while the player is
    /// landed.
    ///
    /// *Ordering against the original.* The plunder's cargo, credits and
    /// mission failures and the repair's armour settle here, after the
    /// loop, where the original carries them out inside it. All of it is
    /// later in the pass than the loop, but before the shots, the frame's
    /// drawing and any dialog, landing, jump or save, so no boundary is
    /// crossed.
    ///
    /// The pass settles it jumping or not; each scan is held past the
    /// warp's lead-in by `_ScanPlayer`'s jump-timing check
    /// (@0x7e562-0x7e633; see [`Session::scanned_by`]). The work waits from
    /// the think that queued it to its pass's settling, so it settles
    /// before the frame is drawn. In the warp the view steps the session only
    /// while its lead-in lasts, and the arrival comes after its last step,
    /// so work queued in the lead-in settles before the arrival too. Work
    /// the arrival would cut off is dropped with the system's traffic, as
    /// the original's would be held.
    pub(super) fn settle_line_queue(&mut self, law: &dyn LegalCode, chance: &mut dyn Chance) {
        if self.mode.landed().is_some() {
            return;
        }
        for work in self.space.traffic.take_on_line() {
            match work {
                OnLine::Scan(id) => self.scanned_by(id, law, chance),
                OnLine::Call(id, call, odds) => self.answer_call(id, call, odds, chance),
                OnLine::Pirate(event) => self.answer_pirate(event, chance),
                OnLine::RefuelDone(id, ship) => {
                    // `_beepSnd[1]` (@0x8f585-0x8f5b0), with the line.
                    let sound = SimSound::EnergyTransferComplete;
                    self.space.say_help_done(
                        &self.pilot,
                        &self.statics,
                        &mut self.out,
                        id,
                        ship,
                        Help::Refuel,
                        sound,
                    );
                }
            }
        }
        if std::mem::take(&mut self.space.reinforcements_held) {
            self.tick_reinforcements(chance);
        }
    }

    /// What `_AICallForHelp` asks of NPC `id` now, if it is in the
    /// system.
    fn distress_facts(&self, id: NpcId) -> Option<DistressFacts> {
        let world = self.world();
        let around = world.around(self.npcs());
        let npc = around.npc(id)?;
        let inherent = self
            .statics
            .ship_record(npc.ship)
            .and_then(|record| record.inherent_govt);
        Some(DistressFacts {
            person: npc.person.is_some(),
            quiet: self.statics.govts.flag2(npc.govt, QUIET),
            quiet_inherent: self.statics.govts.flag2(inherent, QUIET),
            line_free: self.space.comm_line.free_for_calls(),
            liked: likes_player(npc, &around, self.mission_hail(npc)),
            threatens: threatens_player(npc, &around, self.ship.condition == Condition::Intact),
            player_dying: matches!(
                self.ship.condition,
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
            .setup
            .nth_string(DISTRESS_LINES, 1 + pick)
            .unwrap_or_default();
        self.out.sounds.push(SimSound::CallForHelp);
        self.space.say(
            &mut self.out,
            SimMessage::CallForHelp(format!("{name}:  {line}")),
        );
    }
}

impl Space {
    /// Raises `message` on the message line, setting the line's count to
    /// its ticks as `_DisplayComm` does (@0x9024b).
    pub(super) fn say(&mut self, out: &mut Outbox, message: SimMessage) {
        self.comm_line.show(message.ticks());
        out.messages.push(message);
    }

    /// The flight shows a line of its own for `ticks`: one the view
    /// raises and words itself, or composes from the session's (see the
    /// module docs). A line the session raised is not shown again here.
    fn comm_shown(&mut self, ticks: u16) {
        self.comm_line.show(ticks);
    }

    /// Return (`ackComm`) clears the message line: for a tick, while a
    /// line shows (`_HandlePlayer` @0x6dd22-0x6dd65).
    fn comm_acknowledged(&mut self) {
        self.comm_line.acknowledge();
    }

    /// The message line's countdown, as the calls read it.
    #[must_use]
    fn comm_line(&self) -> CommLine {
        self.comm_line
    }

    /// The flight counts the message line down a tick of a pass it
    /// shows a line in: the plunder dialog's loop, whose `_HandleComm`
    /// (@0x949a1) comes after a take's line, which the flight words and
    /// shows ([`Session::comm_shown`]) after the session's own pass (see
    /// the session's `plunder_loop` module).
    fn comm_ticked(&mut self) {
        self.count_down_comm_line();
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

    /// Interceptor `id`'s pre-scan hail, when it has a government, the
    /// player carries mission cargo that government scans for, and the
    /// line is idle.
    fn hail_before_scan(
        &mut self,
        pilot: &Pilot,
        statics: &Statics,
        setup: &Setup,
        out: &mut Outbox,
        id: NpcId,
        chance: &mut (impl Chance + ?Sized),
    ) {
        let Some(npc) = self.npcs().iter().find(|npc| npc.id == id) else {
            return;
        };
        let Some(govt) = npc.govt.and_then(|govt| statics.govts.get(govt)) else {
            return;
        };
        let found = hail_finds(govt, &pilot.missions, |mission| {
            statics.lifecycle_record(mission)
        });
        if !found || !self.comm_line.idle() {
            return;
        }
        let class = statics.ship_name(npc.ship).unwrap_or_default();
        let ship_name = pilot.ship_name().unwrap_or_default();
        let pick = chance.below(SCAN_HAIL_COUNT) as usize;
        let words = setup
            .nth_string(SCAN_HAIL_LINES, SCAN_HAIL_FIRST + pick)
            .unwrap_or_default();
        let text = format!("{class}:  {ship_name}, {words}.");
        out.sounds.push(SimSound::ScanHail);
        self.say(out, SimMessage::ScanHail(text));
    }
}

#[cfg(test)]
mod tests {
    use crate::combat::Rules;
    use crate::flight::Controls;
    use crate::testkit::work_with;
    use std::collections::{BTreeMap, BTreeSet};
    use std::rc::Rc;

    use super::*;
    use crate::ai::{Behaviour, Decision, Goal, Surroundings};
    use crate::catalog::{
        CommCatalog, GovtId, GovtRecord, MissionRecord, MissionShip, Penalties, ShipId, ShipRecord,
        SystemId,
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
    use crate::traffic::plunder::PirateEvent;

    /// Traders, scanning for bit 0x0004.
    const TRADERS: GovtId = GovtId(128);
    /// A quiet government (`Flags2` 0x0008).
    const QUIETS: GovtId = GovtId(129);
    /// The police, scanning for bit 0x0004 and punishing smuggling, its
    /// fine 500.
    const POLICE: GovtId = GovtId(130);
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

    /// Each NPC listed decides its goal and makes its call, if any; the
    /// others idle, calling nothing. Each NPC of the second plunders the
    /// player in its think, drifting after.
    #[derive(Debug, Default)]
    struct Calling(BTreeMap<NpcId, (Goal, Option<CommCall>)>, BTreeSet<NpcId>);

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
            if self.1.contains(&npc.id) {
                return Decision {
                    goal: Goal::Drift,
                    plunders: Some(ShipRef::Player),
                    ..Decision::default()
                };
            }
            let (goal, call) = self.0.get(&npc.id).copied().unwrap_or((Goal::Idle, None));
            Decision {
                goal,
                comm_call: call,
                ..Decision::default()
            }
        }
    }

    /// The catalog: ships 128-130 (130's inherent government quiet), the
    /// traders, the quiet government and the police, and mission 140 escorting the
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
                GovtRecord {
                    scan_mask: 0x0004,
                    scan_fine: 500,
                    penalties: Penalties {
                        smuggle: 5,
                        ..Penalties::default()
                    },
                    ..govt(130)
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
        work_with(
            &mut session,
            catalog,
            Controls::default(),
            &Peaceful,
            Rules::default(),
            &mut NeverFires,
        );
        let _ = (session.take_messages(), session.take_sounds());
        session.space.comm_line = CommLine::default();
        session
    }

    use crate::ai::Peaceful;

    /// Adds an NPC of `govt` flying `class`, at the player's side.
    fn add(session: &mut Session, govt: Option<GovtId>, class: ShipId) -> NpcId {
        let mut ship = npc(0, session.stats());
        ship.govt = govt;
        ship.ship = class;
        ship.state.position = session.ship.player.position;
        session.space.traffic.add_npc(ship)
    }

    /// A trader of `govt` and `class` fleeing a pirate, calling for help;
    /// and the behaviour that has it flee.
    fn fleeing(session: &mut Session, govt: Option<GovtId>, class: ShipId) -> (NpcId, Calling) {
        let pirate = add(session, None, ShipId(128));
        let trader = add(session, govt, class);
        let flees = (Goal::Flee(ShipRef::Npc(pirate)), Some(CommCall::Distress));
        (trader, calling([(trader, flees)]))
    }

    /// `calls`, no NPC plundering.
    fn calling<const N: usize>(calls: [(NpcId, (Goal, Option<CommCall>)); N]) -> Calling {
        Calling(BTreeMap::from(calls), BTreeSet::new())
    }

    /// A pass of `catalog` as `behaviour` decides, which settles the
    /// calls, drawing `draws`: the messages and sounds it raised, and the
    /// bounds asked.
    fn ticked(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        behaviour: &Calling,
        draws: &[u32],
    ) -> (Vec<SimMessage>, Vec<SimSound>, Vec<u32>) {
        stepped(session, catalog, behaviour, draws)
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
        session.space.say(
            &mut session.out,
            SimMessage::FleetArrivalQuote("Hail".to_owned()),
        );
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
            assert_eq!((messages, asked), (vec![], vec![]), "{why}");
            // A breakup's klaxon is the pass's.
            assert!(!sounds.contains(&SimSound::CallForHelp), "{why}");
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
                session.space.npc_mut(trader).expect("there").person =
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
                        zero_fuel: false,
                    });
            },
            Some(TRADERS),
            CALLER,
            "a person",
        );
        silent(
            &|session, trader| {
                session.space.npc_mut(trader).expect("there").cloak = crate::cloak::Cloak {
                    fade: crate::cloak::FADE_FULL,
                    direction: 0,
                };
            },
            Some(TRADERS),
            CALLER,
            "cloaked",
        );
        silent(
            &|session, _| session.ship.condition = Condition::Destroyed,
            Some(TRADERS),
            CALLER,
            "the player destroyed",
        );
        silent(
            &|session, _| {
                session.ship.condition = Condition::Dying {
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
                session.space.npc_mut(trader).expect("there").mission = Some(MissionMark {
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
            session.space.npc_mut(trader).expect("there").mission = Some(MissionMark {
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
        session.space.npc_mut(police).expect("there").ai_type = AiType::Interceptor;
        if counted {
            session.space.count_down_comm_line();
        }
        let picks = (Goal::Inspect(ShipRef::Player), Some(CommCall::PreScan));
        ticked(&mut session, &catalog, &calling([(police, picks)]), draws)
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
        session
            .space
            .say(&mut session.out, SimMessage::ScanHail(String::new()));
        for _ in 0..399 {
            session.tick_quotes(&catalog, &mut NeverFires);
        }
        assert!(!session.comm_line().free_for_calls(), "399 counted");
        session.tick_quotes(&catalog, &mut NeverFires);
        assert!(session.comm_line().free_for_calls(), "400 counted");
    }

    // The calls against the scans, in a pass of the game's work.

    /// A pass of the game's work by `source`'s reading of
    /// [`RuleKey::LaunchFrameAi`](crate::rulebook::RuleKey::LaunchFrameAi),
    /// the player carrying mission 140's cargo, which the traders (of a
    /// `SmugPenalty` of 5) scan for: `calling` sets up the NPCs whose calls
    /// are made in the pass, in lower slots, then an interceptor of the
    /// traders gives up inspecting the player, so it scans and finds the
    /// cargo. The messages and sounds the pass raised.
    fn scanned_while(
        source: crate::rulebook::RuleSource,
        calling: impl FnOnce(&mut Session) -> Calling,
    ) -> (Vec<SimMessage>, Vec<SimSound>) {
        use crate::rulebook::{RuleKey, Rulebook};
        let mut catalog = calling_catalog();
        catalog.govts[0].penalties.smuggle = 5;
        let mut session = flying(&catalog)
            .with_rules(Rulebook::default().with_override(RuleKey::LaunchFrameAi, source));
        session.pilot.missions.put(0, carrying(140, 4));
        let calls = calling(&mut session);
        let scanner = add(&mut session, Some(TRADERS), CALLER);
        let npc = session.space.npc_mut(scanner).expect("there");
        npc.ai_type = AiType::Interceptor;
        npc.goal = Goal::Inspect(ShipRef::Player);
        let mut chance = crate::testkit::Ranged::of(&[(100, 0)]);
        let rescued = session.game_work(
            crate::flight::Controls::default(),
            &catalog,
            &calls,
            crate::Rules::default(),
            &mut chance,
        );
        assert!(!rescued);
        (session.take_messages(), session.take_sounds())
    }

    /// The smuggling message of the interceptor's scan.
    fn finds_smuggling(messages: &[SimMessage]) -> bool {
        messages
            .iter()
            .any(|message| matches!(message, SimMessage::Smuggling { .. }))
    }

    #[test]
    fn a_lower_slots_distress_call_in_a_scans_pass_is_said_before_the_scans_message() {
        // The pass's calls and scans settle after `_AIDispatch`'s loop in
        // slot order: the caller's slot is below the scanner's, and the
        // scan's message does not ask the line, so it follows the call.
        use crate::rulebook::RuleSource;
        for source in [RuleSource::Engine, RuleSource::Bible] {
            let (messages, sounds) =
                scanned_while(source, |session| fleeing(session, Some(TRADERS), CALLER).1);
            assert_eq!(
                messages.first(),
                Some(&SimMessage::CallForHelp("Ship 129:  Help 20".to_owned())),
                "{source:?}"
            );
            assert!(finds_smuggling(&messages), "{source:?}: {messages:?}");
            assert_eq!(
                sounds,
                [SimSound::CallForHelp, SimSound::Smuggling],
                "{source:?}"
            );
        }
    }

    #[test]
    fn a_lower_slots_pre_scan_hail_in_a_scans_pass_is_said_before_the_scans_message() {
        use crate::rulebook::RuleSource;
        for source in [RuleSource::Engine, RuleSource::Bible] {
            let (messages, sounds) = scanned_while(source, |session| {
                let police = add(session, Some(TRADERS), CALLER);
                session.space.npc_mut(police).expect("there").ai_type = AiType::Interceptor;
                session.space.count_down_comm_line();
                let picks = (Goal::Inspect(ShipRef::Player), Some(CommCall::PreScan));
                calling([(police, picks)])
            });
            assert_eq!(
                messages.first(),
                Some(&SimMessage::ScanHail(
                    "Ship 129:  Ship 128, hold.".to_owned()
                )),
                "{source:?}"
            );
            assert!(finds_smuggling(&messages), "{source:?}: {messages:?}");
            assert_eq!(
                sounds,
                [SimSound::ScanHail, SimSound::Smuggling],
                "{source:?}"
            );
        }
    }

    // The calls and scans in slot order (see the module docs).

    /// Adds an interceptor of the police at the player's side, inspecting
    /// the player; it gives the inspection up on its next think.
    fn scanner(session: &mut Session) -> NpcId {
        let police = add(session, Some(POLICE), CALLER);
        let npc = session.space.npc_mut(police).expect("there");
        npc.ai_type = AiType::Interceptor;
        npc.goal = Goal::Inspect(ShipRef::Player);
        police
    }

    /// A pass as `behaviour` decides, drawing `draws`: the messages and
    /// sounds it raised, and the bounds asked.
    fn stepped(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        behaviour: &Calling,
        draws: &[u32],
    ) -> (Vec<SimMessage>, Vec<SimSound>, Vec<u32>) {
        let mut chance = Draws::of(draws);
        work_with(
            session,
            catalog,
            Controls::default(),
            behaviour,
            crate::Rules::default(),
            &mut chance,
        );
        (session.take_messages(), session.take_sounds(), chance.asked)
    }

    /// The police's find of mission 140's cargo: the fine, told.
    fn smuggling() -> SimMessage {
        SimMessage::Smuggling {
            scanner: CALLER,
            ship_name: None,
            fine: 500,
        }
    }

    #[test]
    fn an_earlier_slots_scan_line_keeps_a_later_slots_distress_call_off() {
        let catalog = calling_catalog();
        let mut session = flying(&catalog);
        session.pilot.missions.put(0, carrying(140, 4));
        scanner(&mut session);
        let (_, calls) = fleeing(&mut session, Some(TRADERS), CALLER);
        let (messages, sounds, asked) = stepped(&mut session, &catalog, &calls, &[0, 0]);
        assert_eq!(messages, [smuggling()], "no call for help");
        assert_eq!(sounds, [SimSound::Smuggling], "no snd 154");
        assert_eq!(asked, [100], "the scan's roll, no distress line drawn");
    }

    #[test]
    fn a_call_by_an_earlier_slot_shows_and_a_later_slots_scan_replaces_it() {
        let catalog = calling_catalog();
        let mut session = flying(&catalog);
        session.pilot.missions.put(0, carrying(140, 4));
        let (_, calls) = fleeing(&mut session, Some(TRADERS), CALLER);
        scanner(&mut session);
        let (messages, sounds, asked) = stepped(&mut session, &catalog, &calls, &[0, 0]);
        assert_eq!(
            messages,
            [help("Ship 129:  Help 1"), vec![smuggling()]].concat()
        );
        assert_eq!(sounds, [SimSound::CallForHelp, SimSound::Smuggling]);
        assert_eq!(asked, [DISTRESS_LINE_COUNT, 100]);
        let mut line = CommLine::default();
        line.show(smuggling().ticks());
        assert_eq!(session.comm_line(), line, "the scan's count");
    }

    #[test]
    fn an_earlier_slots_scan_line_keeps_a_later_interceptors_pre_scan_hail_off() {
        let catalog = calling_catalog();
        let mut session = flying(&catalog);
        session.pilot.missions.put(0, carrying(140, 4));
        scanner(&mut session);
        let hailer = add(&mut session, Some(TRADERS), CALLER);
        session.space.npc_mut(hailer).expect("there").ai_type = AiType::Interceptor;
        session.space.count_down_comm_line();
        assert!(session.comm_line().idle());
        let picks = (Goal::Inspect(ShipRef::Player), Some(CommCall::PreScan));
        let calls = calling([(hailer, picks)]);
        let (messages, sounds, asked) = stepped(&mut session, &catalog, &calls, &[0, 0]);
        assert_eq!(messages, [smuggling()], "no hail");
        assert_eq!(sounds, [SimSound::Smuggling], "no snd 154");
        assert_eq!(asked, [100]);
    }

    #[test]
    fn a_call_held_behind_a_scan_that_finds_nothing_is_still_made_in_the_pass() {
        let catalog = calling_catalog();
        let mut session = flying(&catalog);
        scanner(&mut session);
        let (_, calls) = fleeing(&mut session, Some(TRADERS), CALLER);
        let mut chance = Draws::of(&[0, 2]);
        work_with(
            &mut session,
            &catalog,
            Controls::default(),
            &calls,
            Rules::default(),
            &mut chance,
        );
        assert_eq!(session.take_messages(), help("Ship 129:  Help 3"));
        assert_eq!(session.take_sounds(), [SimSound::CallForHelp]);
        assert_eq!(chance.asked, [100, DISTRESS_LINE_COUNT], "scan, then call");
    }

    #[test]
    fn a_held_call_settles_on_a_jumping_step_too() {
        let catalog = calling_catalog();
        let mut session = flying(&catalog);
        session.plot_course(SystemId(131)).expect("a route");
        session.ship.player = crate::flight::ShipState {
            position: crate::geometry::Vec2::new(0.0, -5000.0),
            velocity: crate::geometry::Vec2::ZERO,
            heading: 90.0,
        };
        session.begin_jump().expect("jumps");
        assert!(session.in_warp_lead_in());
        let _ = (session.take_messages(), session.take_sounds());
        scanner(&mut session);
        let (_, calls) = fleeing(&mut session, Some(TRADERS), CALLER);
        let mut chance = Draws::of(&[0, 0]);
        work_with(
            &mut session,
            &catalog,
            Controls::default(),
            &calls,
            Rules::default(),
            &mut chance,
        );
        assert!(session.jumping().is_some(), "still jumping");
        assert_eq!(session.take_messages(), help("Ship 129:  Help 1"));
        assert_eq!(session.take_sounds(), [SimSound::CallForHelp]);
    }

    // The frame's other lines in slot order with the calls (see the
    // module docs).

    /// A pass as `behaviour` decides, drawing `draws`, `queued` on the line
    /// with its thinks' calls, each at its actor's berth: the messages,
    /// comm notes and sounds it raised.
    fn stepped_with(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        behaviour: &Calling,
        draws: &[u32],
        queued: &[PirateEvent],
    ) -> (Vec<SimMessage>, Vec<crate::hail::CommNote>, Vec<SimSound>) {
        let mut chance = Draws::of(draws);
        session.space.queued_in_frame.extend_from_slice(queued);
        work_with(
            session,
            catalog,
            Controls::default(),
            behaviour,
            Rules::default(),
            &mut chance,
        );
        (
            session.take_messages(),
            session.take_comm(),
            session.take_sounds(),
        )
    }

    /// Adds a pirate that plunders the player as `calls` has it, the
    /// player holding no cargo and 10,000 credits, so a plunder at the
    /// odds' floor of 10 takes 300 credits.
    fn plunderer(session: &mut Session, calls: &mut Calling) -> NpcId {
        let pirate = add(session, None, ShipId(128));
        calls.1.insert(pirate);
        session.pilot.cargo.clear();
        session.pilot.cash = 10_000;
        pirate
    }

    /// The pirate's line: no tons and 300 credits stolen.
    fn stolen() -> SimMessage {
        SimMessage::CargoStolen {
            tons: 0,
            credits: 300,
        }
    }

    /// The line's count as `message` set it.
    fn showing(message: &SimMessage) -> CommLine {
        let mut line = CommLine::default();
        line.show(message.ticks());
        line
    }

    #[test]
    fn an_earlier_slots_stolen_line_keeps_a_later_slots_distress_call_off() {
        let catalog = calling_catalog();
        let mut session = flying(&catalog);
        let mut calls = Calling::default();
        plunderer(&mut session, &mut calls);
        let (trader, fled) = fleeing(&mut session, Some(TRADERS), CALLER);
        calls.0.insert(trader, fled.0[&trader]);
        let (messages, sounds, asked) = stepped(&mut session, &catalog, &calls, &[0, 0]);
        assert_eq!(messages, [stolen()], "no call for help");
        assert_eq!(sounds, [SimSound::CargoStolen]);
        assert_eq!(asked, [21], "the odds' jitter, no distress line drawn");
    }

    #[test]
    fn an_earlier_slots_stolen_line_keeps_a_later_interceptors_pre_scan_hail_off() {
        let catalog = calling_catalog();
        let mut session = flying(&catalog);
        session.pilot.missions.put(0, carrying(140, 4));
        let mut calls = Calling::default();
        plunderer(&mut session, &mut calls);
        let hailer = add(&mut session, Some(TRADERS), CALLER);
        session.space.npc_mut(hailer).expect("there").ai_type = AiType::Interceptor;
        session.space.count_down_comm_line();
        assert!(session.comm_line().idle());
        calls.0.insert(
            hailer,
            (Goal::Inspect(ShipRef::Player), Some(CommCall::PreScan)),
        );
        let (messages, sounds, asked) = stepped(&mut session, &catalog, &calls, &[0, 0]);
        assert_eq!(messages, [stolen()], "no hail");
        assert_eq!(sounds, [SimSound::CargoStolen], "no snd 154");
        assert_eq!(asked, [21]);
    }

    #[test]
    fn a_later_slots_stolen_line_replaces_an_earlier_slots_call() {
        let catalog = calling_catalog();
        let mut session = flying(&catalog);
        let (_, mut calls) = fleeing(&mut session, Some(TRADERS), CALLER);
        plunderer(&mut session, &mut calls);
        let (messages, sounds, asked) = stepped(&mut session, &catalog, &calls, &[0, 0]);
        assert_eq!(
            messages,
            [help("Ship 129:  Help 1"), vec![stolen()]].concat()
        );
        assert_eq!(sounds, [SimSound::CallForHelp, SimSound::CargoStolen]);
        assert_eq!(asked, [DISTRESS_LINE_COUNT, 21]);
        assert_eq!(session.comm_line(), showing(&stolen()), "the stolen line's");
    }

    /// A former escort `pirate` stole, said in the frame's slot order.
    fn escort_stolen(pirate: NpcId) -> PirateEvent {
        PirateEvent::Plundered {
            pirate,
            victim: NpcId(9999),
            tons: 0,
            line: None,
            taken: Some(crate::ai::plunder_contents::Taken::Escort),
        }
    }

    #[test]
    fn an_escort_stolen_line_from_an_earlier_slot_keeps_a_later_slots_call_off() {
        let catalog = calling_catalog();
        let mut session = flying(&catalog);
        let pirate = add(&mut session, None, ShipId(128));
        let (_, calls) = fleeing(&mut session, Some(TRADERS), CALLER);
        let (messages, _, sounds) = stepped_with(
            &mut session,
            &catalog,
            &calls,
            &[0],
            &[escort_stolen(pirate)],
        );
        assert_eq!(messages, [SimMessage::ShipStolen { fighter: false }]);
        assert_eq!(sounds, [SimSound::ShipStolen]);
    }

    #[test]
    fn an_escort_stolen_line_from_a_later_slot_replaces_an_earlier_slots_call() {
        let catalog = calling_catalog();
        let mut session = flying(&catalog);
        let (_, calls) = fleeing(&mut session, Some(TRADERS), CALLER);
        let pirate = add(&mut session, None, ShipId(128));
        let (messages, _, sounds) = stepped_with(
            &mut session,
            &catalog,
            &calls,
            &[0],
            &[escort_stolen(pirate)],
        );
        assert_eq!(
            messages,
            [
                help("Ship 129:  Help 1"),
                vec![SimMessage::ShipStolen { fighter: false }]
            ]
            .concat()
        );
        assert_eq!(sounds, [SimSound::CallForHelp, SimSound::ShipStolen]);
    }

    /// Adds a ship docked at the player refuelling it, the player's tank
    /// above a jump's fuel, so the refuel is done on its first tick.
    fn refueller(session: &mut Session) -> NpcId {
        let helper = add(session, Some(TRADERS), ShipId(128));
        let npc = session.space.npc_mut(helper).expect("there");
        npc.goal = Goal::Assist(crate::hail::Help::Refuel);
        npc.target = Some(ShipRef::Player);
        session.pilot.reserves.fuel.now = 150.0;
        helper
    }

    #[test]
    fn an_earlier_refuellers_note_keeps_a_later_slots_call_off() {
        let catalog = calling_catalog();
        let mut session = flying(&catalog);
        let helper = refueller(&mut session);
        let (_, calls) = fleeing(&mut session, Some(TRADERS), CALLER);
        let (messages, notes, sounds) = stepped_with(&mut session, &catalog, &calls, &[0], &[]);
        assert_eq!(messages, [], "no call for help");
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].from, helper);
        assert_eq!(sounds, [SimSound::EnergyTransferComplete]);
    }

    #[test]
    fn an_earlier_slots_call_shows_and_a_later_refuellers_note_replaces_it() {
        let catalog = calling_catalog();
        let mut session = flying(&catalog);
        let (_, calls) = fleeing(&mut session, Some(TRADERS), CALLER);
        refueller(&mut session);
        let (messages, notes, sounds) = stepped_with(&mut session, &catalog, &calls, &[0], &[]);
        assert_eq!(messages, help("Ship 129:  Help 1"));
        assert_eq!(notes.len(), 1);
        assert_eq!(
            sounds,
            [SimSound::CallForHelp, SimSound::EnergyTransferComplete]
        );
    }

    #[test]
    fn an_earlier_repairers_note_keeps_a_later_slots_call_off() {
        let catalog = calling_catalog();
        let mut session = flying(&catalog);
        let helper = add(&mut session, Some(TRADERS), ShipId(128));
        let (_, calls) = fleeing(&mut session, Some(TRADERS), CALLER);
        let repaired = PirateEvent::PlayerRepaired {
            helper,
            ship: ShipId(128),
        };
        let (messages, notes, sounds) =
            stepped_with(&mut session, &catalog, &calls, &[0], &[repaired]);
        assert_eq!(messages, [], "no call for help");
        assert_eq!(notes.len(), 1);
        assert_eq!(sounds, [SimSound::RepairsComplete]);
    }

    #[test]
    fn an_earlier_slots_call_shows_and_a_later_repairers_note_replaces_it() {
        let catalog = calling_catalog();
        let mut session = flying(&catalog);
        let (_, calls) = fleeing(&mut session, Some(TRADERS), CALLER);
        let helper = add(&mut session, Some(TRADERS), ShipId(128));
        let repaired = PirateEvent::PlayerRepaired {
            helper,
            ship: ShipId(128),
        };
        let (messages, notes, sounds) =
            stepped_with(&mut session, &catalog, &calls, &[0], &[repaired]);
        assert_eq!(messages, help("Ship 129:  Help 1"));
        assert_eq!(notes.len(), 1);
        assert_eq!(sounds, [SimSound::CallForHelp, SimSound::RepairsComplete]);
    }

    /// `calling_catalog` with a bomb outfit.
    fn bombing_catalog() -> FakePilotCatalog {
        let mut catalog = calling_catalog();
        catalog.outfits.push(crate::testkit::bomb_outfit());
        catalog
    }

    /// Has `session`'s player own the bomb, its clock seeded and at the
    /// fuse, so it goes off on the next pass.
    fn light_the_bomb(session: &mut Session) {
        session
            .pilot
            .outfits
            .insert(crate::catalog::OutfitId(crate::testkit::BOMB), 1);
        session.reset_precalced();
        let _ = session
            .ship
            .latches
            .sabotage
            .tick(false, &mut Draws::of(&[0]));
        session
            .ship
            .latches
            .sabotage
            .set_counter(crate::sabotage::FUSE);
    }

    /// The bomb's line.
    fn bombed() -> SimMessage {
        SimMessage::BombExploded {
            outfit: crate::catalog::OutfitId(crate::testkit::BOMB),
            one: true,
        }
    }

    #[test]
    fn a_bomb_line_keeps_the_frames_calls_off() {
        let catalog = bombing_catalog();
        let mut session = flying(&catalog);
        let (_, calls) = fleeing(&mut session, Some(TRADERS), CALLER);
        light_the_bomb(&mut session);
        let (messages, sounds, _) = stepped(&mut session, &catalog, &calls, &[0, 0, 0]);
        assert_eq!(messages, [bombed()], "no call for help");
        assert!(!sounds.contains(&SimSound::CallForHelp), "{sounds:?}");
    }

    #[test]
    fn a_call_shows_on_the_frame_before_a_sabotage_line_and_the_line_replaces_it() {
        let catalog = bombing_catalog();
        let mut session = flying(&catalog);
        let (trader, calls) = fleeing(&mut session, Some(TRADERS), CALLER);
        let (messages, _, _) = stepped(&mut session, &catalog, &calls, &[0]);
        assert_eq!(messages, help("Ship 129:  Help 1"), "frame N");
        light_the_bomb(&mut session);
        let quiet = calling([(trader, (Goal::Idle, None))]);
        let (messages, _, _) = stepped(&mut session, &catalog, &quiet, &[0, 0]);
        assert_eq!(messages, [bombed()], "frame N + 1");
        assert_eq!(session.comm_line(), showing(&bombed()), "the bomb's count");
    }

    #[test]
    fn an_eject_line_keeps_the_frames_calls_off() {
        let catalog = crate::testkit::with_escape_ship(calling_catalog());
        let mut session = flying(&catalog);
        let (_, calls) = fleeing(&mut session, Some(TRADERS), CALLER);
        session
            .pilot
            .outfits
            .insert(crate::catalog::OutfitId(crate::testkit::ESCAPE_FIGHTERS), 2);
        session.ship.condition = Condition::Disabled;
        session.hold_eject(true);
        let (messages, sounds, _) = stepped(&mut session, &catalog, &calls, &[0; 8]);
        assert_eq!(
            messages,
            [SimMessage::EscapingIn {
                ship: ShipId(crate::testkit::ESCAPE_SHIP)
            }],
            "no call for help"
        );
        assert!(!sounds.contains(&SimSound::CallForHelp), "{sounds:?}");
    }

    #[test]
    fn a_propulsion_repair_line_sets_the_line_and_keeps_the_frames_calls_off() {
        let mut catalog = calling_catalog();
        catalog.outfits.push(crate::testkit::outfit(
            437,
            &[(crate::self_repair::MOD_REPAIR, 0)],
        ));
        let mut session = flying(&catalog);
        let (_, calls) = fleeing(&mut session, Some(TRADERS), CALLER);
        session
            .pilot
            .outfits
            .insert(crate::catalog::OutfitId(437), 1);
        session.pilot.reserves.armor.now = 3.0;
        session.ship.condition = Condition::Disabled;
        let (messages, sounds, asked) = stepped(&mut session, &catalog, &calls, &[0, 0]);
        assert_eq!(
            messages,
            [SimMessage::PropulsionRepaired],
            "no call for help"
        );
        assert_eq!(sounds, [SimSound::PropulsionRepaired]);
        assert_eq!(
            asked,
            [crate::self_repair::REPAIR_ODDS],
            "no distress line drawn"
        );
        assert_eq!(
            session.comm_line(),
            showing(&SimMessage::PropulsionRepaired),
            "@0x68c3f"
        );
    }

    #[test]
    fn a_call_from_a_reused_lower_berth_is_answered_first() {
        let catalog = calling_catalog();
        let mut session = flying(&catalog);
        let pirate = add(&mut session, None, ShipId(128));
        let flee = |at: u32, class: ShipId, session: &mut Session| {
            let mut ship = npc(0, session.stats());
            ship.govt = Some(TRADERS);
            ship.ship = class;
            ship.state.position = session.ship.player.position;
            session.space.traffic.add_npc_at(ship, at)
        };
        let high = flee(5, CALLER, &mut session);
        let low = flee(2, ShipId(128), &mut session);
        let flees = (Goal::Flee(ShipRef::Npc(pirate)), Some(CommCall::Distress));
        let calls = calling([(high, flees), (low, flees)]);
        let (messages, _, asked) = stepped(&mut session, &catalog, &calls, &[0, 0]);
        assert_eq!(messages, help("Ship 128:  Help 1"), "berth 2's");
        assert_eq!(asked, [DISTRESS_LINE_COUNT], "berth 5's kept off");
    }
}
