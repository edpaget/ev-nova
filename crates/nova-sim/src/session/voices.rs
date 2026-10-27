//! The escorts' voices in flight: the session's side of
//! [`voice`](crate::voice) and [`escort_voice`](crate::escort_voice).
//!
//! **The channel** ([`Space::voice`]) is the original's three globals and
//! its clip handle. A command ([`Session::command_escorts`](super::Session::command_escorts)), a victory
//! and a new target each queue a message on it, the last overwriting the
//! one waiting (`_SpeakVoiceMessage` @0x44558). Once an advance pass, in
//! its AI block before the thinks, `_HandleVoiceSound` (@0x44219) plays
//! it ([`Space::handle_voice`]), cutting off a clip still playing a pass
//! first. The pass head plays a frame's worth of the clip
//! ([`VoiceChannel::step`](crate::voice::VoiceChannel::step)), and landing
//! kills it (`_KillVoiceSound` @0x66daf, [`Space::kill_voice`]). Each
//! message is said in the voice set of the speaker's class's attributes
//! government ([`Statics::speech`]).
//!
//! **The variant** ([`Npc::voice`]) is rolled by [`Space::roll_voice`]
//! alone: as a fighter is launched (`_LaunchFighter` @0x3d9cc-0x3da3a; see
//! [`fighters`](super::fighters)), as a ship is captured
//! (`_AIShipCapturedByPlayer` @0x8974a-0x89799), for the old ship kept
//! on Use As My Ship (`_DoShipCapture` @0x41288-0x412db), and at the
//! first think of a ship the player leads that has none (`_EscortAI`
//! @0x83980-0x839f6, [`Space::answer_voice_cues`]). An escort keeps its
//! variant across jumps and landings, as its skill and afterburner (see
//! the `escorts` module's `KeptEscort`), but not across a save, and a
//! ship hired or spawned rolls none until that first think.
//!
//! **In a pass** (see the `game_work` module):
//!
//! 1. The pass head steps the clip's countdown.
//! 2. The collision passes' finishing hits queue victories
//!    ([`Space::speak_victories`]), overwriting a command's speech, which
//!    the view gave before the pass, as `_HandlePlayer` gives it.
//! 3. The AI block's [`Space::handle_voice`] disposes, halts or plays.
//! 4. The thinks queue their cues, and [`Space::answer_voice_cues`] rolls
//!    the variants and the 1-in-3 target calls against the channel as
//!    step 3 left it, in berth order, so the first speaker closes the
//!    gate for the rest, as `_AIDispatch`'s slot loop does.
//!
//! A victory or target call queued after step 3 plays at the next pass's
//! step 3, as in the original. Double time's second pass runs no
//! [`Space::handle_voice`] and no cues (`_DoPlayGameWork(0)` skips
//! @0x4402a-0x44350); a victory it queues waits for the next advance pass,
//! as the original's does. The separate parts
//! ([`Session::tick_traffic`](super::Session::tick_traffic)) answer their cues through the same
//! [`Space::answer_voice_cues`], so no stale cue reaches a later pass, but
//! none of them plays the channel.
//!
//! [`Npc::voice`]: crate::traffic::npc::Npc::voice

use super::{Outbox, Space, Statics};
use crate::catalog::ShipId;
use crate::chance::Chance;
use crate::combat::{EscortLoss, FinishingHit, ShipRef};
use crate::escort_voice::{VoiceCue, Voter, calls_target, command_speaker, victory_speaker};
use crate::traffic::npc::NpcId;
use crate::voice::{Speech, VoiceMessage, VoiceVariant, roll_variant};

/// The odds of `_EscortAI`'s target call: Rand(3) == 0 (@0x83df9-0x83e34).
const TARGET_CALL: u32 = 3;

impl Statics {
    /// What a ship of class `ship` in variant `voice` says for
    /// `message`: in the voice set of the class's attributes government
    /// (class +0x9f6, its `InherentGovt`), set 0 with none.
    pub(super) fn speech(
        &self,
        ship: ShipId,
        message: VoiceMessage,
        voice: Option<VoiceVariant>,
    ) -> Speech {
        Speech {
            message,
            set: self.voices.set_of(self.attributes_govt(ship)),
            variant: voice,
        }
    }

    /// The variant the attributes government of class `ship` forces, if
    /// any (+0x40).
    pub(super) fn forced_variant(&self, ship: ShipId) -> Option<VoiceVariant> {
        self.voices.forced_of(self.attributes_govt(ship))
    }

    /// The attributes government of class `ship` (+0x9f6): its record's
    /// `InherentGovt`, as attributes.
    fn attributes_govt(&self, ship: ShipId) -> Option<crate::catalog::GovtId> {
        self.ship_record(ship)
            .and_then(|record| record.inherent_govt)
    }
}

impl Space {
    /// `_HandleVoiceSound` (@0x31e20-0x31f94), once an advance pass: the
    /// channel disposes, halts or plays (see
    /// [`VoiceChannel::handle`](crate::voice::VoiceChannel::handle)), its
    /// clip picked from `statics`' voices on `chance`, any sound raised to
    /// `out`.
    pub(super) fn handle_voice(
        &mut self,
        statics: &Statics,
        out: &mut Outbox,
        chance: &mut (impl Chance + ?Sized),
    ) {
        if let Some(sound) = self.voice.handle(&statics.voices.table, chance) {
            out.sounds.push(sound);
        }
    }

    /// Rolls the variant of NPC `id`, the one writer of a rolled
    /// [`Npc::voice`](crate::traffic::npc::Npc::voice), on `chance`: the
    /// variant its class's government forces, else Rand(2)
    /// ([`roll_variant`]).
    pub(super) fn roll_voice(
        &mut self,
        id: NpcId,
        statics: &Statics,
        chance: &mut (impl Chance + ?Sized),
    ) {
        if let Some(npc) = self.npc_mut(id) {
            npc.voice = Some(roll_variant(statics.forced_variant(npc.ship), chance));
        }
    }

    /// Answers the voice cues of the thinks since they were last taken,
    /// drawing on `chance`, for each ship the player leads in berth order
    /// (`_EscortAI`, in `_AIDispatch`'s slot loop): a ship with no variant
    /// rolls one (@0x83980-0x839f6); on a new threat, while the channel is
    /// idle, a ship that calls its targets ([`calls_target`]) says it on a
    /// Rand(3) of 0 (@0x83df9-0x83e34). The others' cues are dropped.
    ///
    /// The original speaks inside each slot's think; these settle after
    /// every think of the pass. Nothing between the two reads or writes
    /// the channel, so the difference crosses no boundary the player
    /// hears.
    pub(super) fn answer_voice_cues(
        &mut self,
        statics: &Statics,
        chance: &mut (impl Chance + ?Sized),
    ) {
        let cues = self.traffic.take_voice_cues();
        let led: Vec<NpcId> = self.player_led().map(|npc| npc.id).collect();
        for id in led {
            for &(_, cue) in cues.iter().filter(|&&(cued, _)| cued == id) {
                if self.npc(id).is_some_and(|npc| npc.voice.is_none()) {
                    self.roll_voice(id, statics, chance);
                }
                if cue != VoiceCue::NewThreat || !self.voice.idle() {
                    continue;
                }
                let Some(npc) = self.npc(id).filter(|npc| calls_target(npc)) else {
                    continue;
                };
                let speech = statics.speech(npc.ship, VoiceMessage::Target, npc.voice);
                if chance.below(TARGET_CALL) == 0 {
                    self.voice.speak(speech);
                }
            }
        }
    }

    /// `_IssueNewEscortCommand`'s voice (@0x665e6-0x6667a), for a command
    /// that changed something among `voters`: when some voter that isn't
    /// mute had its order changed, the speaker drawn on `chance` in berth
    /// order ([`command_speaker`]) says it attacks the target when
    /// `targeted`, else acknowledges, in its voice from `statics`.
    pub(super) fn acknowledge(
        &mut self,
        statics: &Statics,
        voters: &mut [Voter],
        targeted: bool,
        chance: &mut (impl Chance + ?Sized),
    ) {
        if !voters.iter().any(|voter| voter.speaks) {
            return;
        }
        voters.sort_by_key(|voter| voter.berth);
        let Some(speaker) = command_speaker(voters, chance) else {
            return;
        };
        let message = if targeted {
            VoiceMessage::Target
        } else {
            VoiceMessage::Acknowledge
        };
        self.voice
            .speak(statics.speech(speaker.ship, message, speaker.voice));
    }

    /// Queues the victory of each of `finished`, in order, the hits that
    /// started their ships dying (`_HandleShipHit` @0x36c7f-0x36d45): said
    /// by [`victory_speaker`] among the ships the player leads, if any, so
    /// the last one's stands. A ship of the fleet the phase lost, among
    /// `lost`, is led only until the strike that lost it, as `_DamageShip`
    /// takes it out of the fleet as that hit lands (@0x3afdd).
    ///
    /// The original speaks inside each hit; these settle after the
    /// phase's hits, before anything else reads or writes the channel, so
    /// the difference crosses no boundary the player hears.
    pub(super) fn speak_victories(
        &mut self,
        statics: &Statics,
        finished: &[FinishingHit],
        lost: &[EscortLoss],
    ) {
        for hit in finished {
            let gone = |id: NpcId| {
                lost.iter()
                    .any(|loss| loss.ship == ShipRef::Npc(id) && loss.strike < hit.strike)
            };
            let led = self.player_led().filter(|npc| !gone(npc.id));
            let speech = victory_speaker(led, hit.ship)
                .map(|npc| statics.speech(npc.ship, VoiceMessage::Victory, npc.voice));
            if let Some(speech) = speech {
                self.voice.speak(speech);
            }
        }
    }

    /// `_KillVoiceSound` (@0x31f95-0x31ff2), as the ship lands
    /// (`_HandlePlayerDockRequest` @0x66daf): the clip loaded halted to
    /// `out` and the message waiting dropped.
    pub(super) fn kill_voice(&mut self, out: &mut Outbox) {
        if let Some(sound) = self.voice.kill() {
            out.sounds.push(sound);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::Session;
    use super::super::fighters::tests::{carrying, fleet_npcs, flying_by, pilot, sitting_ai};
    use super::*;
    use crate::ai::{Behaviour, Goal, Peaceful};
    use crate::catalog::{GovtId, HullRecord, ShipRecord, SoundId, SystemId};
    use crate::chance::NeverFires;
    use crate::combat::Rules;
    use crate::escort::{Commanded, EscortCommand, EscortGroup, EscortOrder};
    use crate::flight::Controls;
    use crate::pilot::Escort;
    use crate::rulebook::Rulebook;
    use crate::sound::SimSound;
    use crate::stats::ShipStats;
    use crate::testkit::work_with;
    use crate::testkit::{FAST, FakePilotCatalog, Ranged, hull, jump, ship};
    use crate::traffic::npc::{AiType, Npc};
    use crate::voice::Voices;
    use crate::warp::SoundLength;

    /// The fighter, a Viper (`EscortType` 0), of government 141: set 4,
    /// forced even.
    const VIPER: ShipId = ShipId(144);
    /// A warship escort (`EscortType` 2) of government 140: set 3, free.
    const WARSHIP: ShipId = ShipId(150);
    /// A medium escort (`EscortType` 1) of government 141.
    const MEDIUM_EVEN: ShipId = ShipId(152);
    /// A freighter escort (`EscortType` 3, a trader) of government 140.
    const FREIGHTER: ShipId = ShipId(153);
    /// A mute medium escort (`Flags2` 0x0010) of government 141.
    const DRONE: ShipId = ShipId(154);
    /// A medium escort (`EscortType` 1) of government 140.
    const MEDIUM: ShipId = ShipId(156);
    /// Set 3's government: `VoiceType` 3.
    const FREE: GovtId = GovtId(140);
    /// Set 4's government, forcing the even voice: `VoiceType` 2004.
    const EVEN: GovtId = GovtId(141);

    /// A clip a second long: 60 Mac ticks, 30 passes.
    const SECOND: SoundLength = SoundLength {
        frames: 22_050,
        rate: 22_050,
    };

    /// Set 3's clips, 5 acknowledgements, 3 target calls and 5 victories,
    /// and set 4's, 8 of each, each a second long; government 140 speaks
    /// set 3 and 141 set 4, forced even.
    fn voices() -> Voices {
        let run = |base: i16, count: i16| {
            (0..count).map(move |clip| (SoundId(base + clip), Some(SECOND)))
        };
        let sounds: Vec<(SoundId, Option<SoundLength>)> = run(1300, 5)
            .chain(run(1310, 3))
            .chain(run(1320, 5))
            .chain(run(1400, 8))
            .chain(run(1410, 8))
            .chain(run(1420, 8))
            .collect();
        Voices::read(&sounds, &[(FREE, 3), (EVEN, 2004)])
    }

    /// [`carrying`], voiced: the Viper of government 141, and escorts of
    /// each class of governments 140 and 141.
    fn voiced() -> FakePilotCatalog {
        let mut catalog = carrying();
        for record in &mut catalog.ship_records {
            record.inherent_govt = match record.id {
                VIPER => Some(EVEN),
                WARSHIP => Some(FREE),
                _ => record.inherent_govt,
            };
        }
        let class = |id: ShipId, escort_type, inherent_ai, govt| ShipRecord {
            escort_type,
            inherent_ai,
            inherent_govt: Some(govt),
            ..ship(id.0, FAST)
        };
        catalog.ship_records.extend([
            class(MEDIUM_EVEN, 1, 3, EVEN),
            class(FREIGHTER, 3, 1, FREE),
            class(DRONE, 1, 3, EVEN),
            class(MEDIUM, 1, 3, FREE),
        ]);
        catalog.hulls.extend([
            hull(MEDIUM_EVEN.0),
            hull(FREIGHTER.0),
            HullRecord {
                flags2: crate::combat::hull::MUTE,
                ..hull(DRONE.0)
            },
            hull(MEDIUM.0),
        ]);
        catalog.voices = voices();
        catalog
    }

    /// An escort of `ship`, of no standing order.
    fn escort(ship: ShipId) -> Escort {
        Escort {
            ship,
            reserves: ShipStats::new(FAST, &[]).full(),
            order: None,
            carried: false,
            wage: None,
            person: None,
            mark: None,
        }
    }

    /// `catalog`'s session flying with a fleet of `ships`, placed.
    fn fleet(catalog: &FakePilotCatalog, ships: &[ShipId]) -> Session {
        let escorts = ships.iter().map(|&ship| escort(ship)).collect();
        let session = flying_by(catalog, pilot(catalog, 0, escorts), Rulebook::default());
        assert_eq!(fleet_npcs(&session).len(), ships.len(), "placed");
        session
    }

    /// The NPC of escort `index`.
    fn escort_npc(session: &mut Session, index: usize) -> &mut Npc {
        let id = session.space.escorts.fleet[index].expect("placed");
        session.space.npc_mut(id).expect("in the system")
    }

    /// Escort `index`'s standing order set to `order`, record and NPC.
    fn order(session: &mut Session, index: usize, order: Option<EscortOrder>) {
        session.pilot.escorts[index].order = order;
        if let Some(duty) = &mut escort_npc(session, index).escort {
            duty.order = order;
        }
    }

    /// The voice sounds among `sounds`.
    fn voiced_only(sounds: Vec<SimSound>) -> Vec<SimSound> {
        sounds
            .into_iter()
            .filter(|sound| matches!(sound, SimSound::Voice { .. } | SimSound::HaltVoice { .. }))
            .collect()
    }

    /// The voice sounds of an advance pass of `session`, its NPCs as
    /// `behaviour` says, drawing on `chance`.
    fn heard(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        behaviour: &impl Behaviour,
        chance: &mut impl Chance,
    ) -> Vec<SimSound> {
        session.take_sounds();
        let rescued = session.game_work(
            Controls::default(),
            catalog,
            behaviour,
            Rules::default(),
            chance,
        );
        assert!(!rescued);
        voiced_only(session.take_sounds())
    }

    fn voice(sound: i16) -> SimSound {
        SimSound::Voice {
            sound: SoundId(sound),
        }
    }

    fn halt(sound: i16) -> SimSound {
        SimSound::HaltVoice {
            sound: SoundId(sound),
        }
    }

    // The variant (`_LaunchFighter`, `_DoShipCapture`,
    // `_AIShipCapturedByPlayer`, `_EscortAI`).

    /// The variant of the fighter the player's bay launches through an
    /// advance pass of `catalog`'s session, drawing on `chance`.
    fn launched_voice(catalog: &FakePilotCatalog, chance: &mut Ranged) -> Option<VoiceVariant> {
        let mut session = flying_by(catalog, pilot(catalog, 1, Vec::new()), Rulebook::default());
        session.hold_fire(false, true);
        // Peaceful thinks cue nothing: the voice is the launch's.
        session.game_work(
            Controls::default(),
            catalog,
            &Peaceful,
            Rules::default(),
            chance,
        );
        let fighters = fleet_npcs(&session);
        assert_eq!(fighters.len(), 1, "launched");
        fighters[0].voice
    }

    #[test]
    fn a_fighter_the_bay_launches_takes_its_governments_forced_variant() {
        let catalog = voiced();
        assert_eq!(
            launched_voice(&catalog, &mut Ranged::of(&[(2, 1)])),
            Some(VoiceVariant::Even)
        );
    }

    #[test]
    fn a_fighter_of_a_free_government_draws_its_variant_as_it_launches() {
        let mut catalog = voiced();
        for record in &mut catalog.ship_records {
            if record.id == VIPER {
                record.inherent_govt = Some(FREE);
            }
        }
        assert_eq!(
            launched_voice(&catalog, &mut Ranged::of(&[(2, 0)])),
            Some(VoiceVariant::Even)
        );
        assert_eq!(
            launched_voice(&catalog, &mut Ranged::of(&[(2, 1)])),
            Some(VoiceVariant::Odd)
        );
    }

    #[test]
    fn a_ship_captured_for_an_escort_and_the_old_ship_kept_draw_their_variants() {
        use crate::session::tests::{boardable, captured};
        use crate::session::{Assigned, Assignment};
        let catalog = boardable();
        let mut session = captured(&catalog);
        let id = session.space.escorts.aboard.expect("aboard").npc;
        assert_eq!(session.space.npc(id).expect("there").voice, None);
        assert_eq!(
            session.assign(Assignment::Escort, &mut Ranged::of(&[(2, 0)])),
            Some(Assigned::Escort)
        );
        assert_eq!(
            session.space.npc(id).expect("joined").voice,
            Some(VoiceVariant::Even)
        );
        let mut session = captured(&catalog);
        assert_eq!(
            session.assign(Assignment::MyShip, &mut Ranged::of(&[(2, 1)])),
            Some(Assigned::MyShip)
        );
        let old = fleet_npcs(&session).pop().expect("the old ship kept");
        assert_eq!(old.voice, Some(VoiceVariant::Odd));
    }

    #[test]
    fn an_escort_with_no_variant_rolls_one_at_its_first_think() {
        let catalog = voiced();
        for (ship, draw, rolled) in [
            (MEDIUM_EVEN, 1, VoiceVariant::Even),
            (MEDIUM, 0, VoiceVariant::Even),
            (MEDIUM, 1, VoiceVariant::Odd),
        ] {
            let mut session = fleet(&catalog, &[ship]);
            assert_eq!(escort_npc(&mut session, 0).voice, None, "{ship:?}");
            heard(
                &mut session,
                &catalog,
                &sitting_ai(),
                &mut Ranged::of(&[(2, draw)]),
            );
            assert_eq!(escort_npc(&mut session, 0).voice, Some(rolled), "{ship:?}");
            heard(
                &mut session,
                &catalog,
                &sitting_ai(),
                &mut Ranged::of(&[(2, 1 - draw)]),
            );
            assert_eq!(
                escort_npc(&mut session, 0).voice,
                Some(rolled),
                "{ship:?}: kept"
            );
        }
    }

    #[test]
    fn an_escorts_variant_is_kept_through_a_jump() {
        let catalog = voiced();
        let mut session = fleet(&catalog, &[MEDIUM_EVEN]);
        // Not what its government forces, so a roll would show.
        escort_npc(&mut session, 0).voice = Some(VoiceVariant::Odd);
        assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(131)));
        assert_eq!(escort_npc(&mut session, 0).voice, Some(VoiceVariant::Odd));
    }

    // A command's acknowledgement (`_IssueNewEscortCommand`
    // @0x660d5-0x66690).

    /// Gives every escort `command`, drawing its speaker on
    /// [`NeverFires`]: what it reported.
    fn command(session: &mut Session, command: EscortCommand) -> Option<Commanded> {
        session.command_escorts(EscortGroup::All, command, &mut NeverFires)
    }

    #[test]
    fn a_command_is_acknowledged_in_the_speakers_set_at_the_next_pass() {
        // Set 3's acknowledgements are 1300-1304; Rand(5) of 2 plays 1302.
        // A warship acknowledges: the command's voice has no `EscortType`
        // gate.
        let catalog = voiced();
        let mut session = fleet(&catalog, &[WARSHIP]);
        session.take_sounds();
        assert!(command(&mut session, EscortCommand::Defend).is_some());
        assert_eq!(session.take_sounds(), [SimSound::EscortsCommanded]);
        assert_eq!(
            heard(
                &mut session,
                &catalog,
                &Peaceful,
                &mut Ranged::of(&[(5, 2)])
            ),
            [voice(1302)]
        );
    }

    #[test]
    fn an_attack_on_a_target_the_player_doesnt_lead_says_the_target_call() {
        // The #159 test (@0x6660a-0x6663c): set 3's target calls,
        // 1310-1312.
        let catalog = voiced();
        let mut session = fleet(&catalog, &[WARSHIP]);
        let stranger = session
            .space
            .traffic
            .add_npc(crate::testkit::npc(0, ShipStats::new(FAST, &[])));
        session.chart.target = Some(stranger);
        assert!(command(&mut session, EscortCommand::Attack).is_some_and(|c| c.targeted));
        assert_eq!(
            heard(
                &mut session,
                &catalog,
                &Peaceful,
                &mut Ranged::of(&[(3, 1)])
            ),
            [voice(1311)]
        );
        // With no target, the acknowledgement.
        let mut session = fleet(&catalog, &[WARSHIP]);
        assert!(command(&mut session, EscortCommand::Attack).is_some());
        assert_eq!(
            heard(
                &mut session,
                &catalog,
                &Peaceful,
                &mut Ranged::of(&[(5, 1)])
            ),
            [voice(1301)]
        );
    }

    #[test]
    fn a_fleet_all_mute_beeps_and_says_nothing() {
        let catalog = voiced();
        let mut session = fleet(&catalog, &[DRONE, DRONE]);
        session.take_sounds();
        assert!(command(&mut session, EscortCommand::Defend).is_some());
        assert_eq!(session.take_sounds(), [SimSound::EscortsCommanded]);
        assert!(session.space.voice.idle());
        assert_eq!(
            heard(&mut session, &catalog, &Peaceful, &mut Ranged::of(&[])),
            []
        );
    }

    #[test]
    fn only_an_order_changed_on_a_ship_that_isnt_mute_lets_a_command_speak() {
        // A mute escort's change sets changed but not speak (@0x661e3,
        // @0x66212): the speaking escort's order stands, so nothing is
        // said, though the command beeps.
        let catalog = voiced();
        let mut session = fleet(&catalog, &[DRONE, WARSHIP]);
        order(&mut session, 1, Some(EscortOrder::Defend));
        session.take_sounds();
        assert!(command(&mut session, EscortCommand::Defend).is_some());
        assert_eq!(session.take_sounds(), [SimSound::EscortsCommanded]);
        assert!(session.space.voice.idle());
    }

    #[test]
    fn a_mute_escort_that_changes_first_leaves_the_voice_to_the_next() {
        let catalog = voiced();
        let mut session = fleet(&catalog, &[DRONE, WARSHIP]);
        assert!(command(&mut session, EscortCommand::Defend).is_some());
        assert_eq!(
            heard(
                &mut session,
                &catalog,
                &Peaceful,
                &mut Ranged::of(&[(5, 0)])
            ),
            [voice(1300)],
            "the warship's set 3, not the drone's set 4"
        );
    }

    #[test]
    fn the_speaker_is_drawn_in_berth_order() {
        // The first changed escort in slot order takes the voice; a later
        // one only on a Rand(2) of 0, which `NeverFires` never draws. The
        // medium is the fleet's second, at the lower berth.
        let catalog = voiced();
        let mut session = fleet(&catalog, &[WARSHIP, MEDIUM_EVEN]);
        let berths = [
            escort_npc(&mut session, 0).berth,
            escort_npc(&mut session, 1).berth,
        ];
        escort_npc(&mut session, 0).berth = berths[1];
        escort_npc(&mut session, 1).berth = berths[0];
        assert!(command(&mut session, EscortCommand::Defend).is_some());
        assert_eq!(
            heard(
                &mut session,
                &catalog,
                &Peaceful,
                &mut Ranged::of(&[(8, 2)])
            ),
            [voice(1402)],
            "set 4's: its variant unset, Rand(8)"
        );
    }

    #[test]
    fn a_command_that_changes_nothing_says_nothing() {
        let catalog = voiced();
        let mut session = fleet(&catalog, &[WARSHIP]);
        order(&mut session, 0, Some(EscortOrder::Defend));
        session.take_sounds();
        assert_eq!(command(&mut session, EscortCommand::Defend), None);
        assert_eq!(session.take_sounds(), []);
        assert!(session.space.voice.idle());
    }

    /// A session with one Viper carried by the player, flying home with
    /// a target, its order `order`; and the target, the player's too.
    fn returning(
        catalog: &FakePilotCatalog,
        order: Option<EscortOrder>,
    ) -> (Session, crate::traffic::npc::NpcId) {
        let mut session = fleet(catalog, &[]);
        session.pilot.escorts.push(Escort {
            carried: true,
            order,
            ..escort(VIPER)
        });
        let placed = session.place_escort(0, &mut NeverFires);
        session.space.escorts.fleet.push(placed);
        let target = session
            .space
            .traffic
            .add_npc(crate::testkit::npc(0, ShipStats::new(FAST, &[])));
        session.chart.target = Some(target);
        let fighter = escort_npc(&mut session, 0);
        assert!(fighter.carrier.is_some(), "carried");
        fighter.goal = Goal::Dock(crate::combat::ShipRef::Player);
        fighter.target = Some(crate::combat::ShipRef::Npc(target));
        session.take_sounds();
        (session, target)
    }

    #[test]
    fn under_any_command_but_return_a_fighter_flying_home_lets_go_of_its_target_and_reports() {
        // `_IssueNewEscortCommand` @0x6627a-0x662b3: the fighter's order
        // and target already the command's, the return alone changes it.
        let catalog = voiced();
        for given in [
            EscortCommand::Attack,
            EscortCommand::Defend,
            EscortCommand::Hold,
            EscortCommand::Recall,
        ] {
            let (mut session, _) = returning(&catalog, given.order());
            assert!(
                command(&mut session, given).is_some(),
                "{given:?}: reported"
            );
            assert_eq!(
                session.take_sounds(),
                [SimSound::EscortsCommanded],
                "{given:?}: beeps"
            );
            assert_eq!(escort_npc(&mut session, 0).target, None, "{given:?}");
        }
    }

    #[test]
    fn under_return_to_hangar_a_fighter_flying_home_keeps_its_target() {
        let catalog = voiced();
        let (mut session, target) = returning(&catalog, Some(EscortOrder::Dock));
        assert_eq!(command(&mut session, EscortCommand::Dock), None);
        assert_eq!(session.take_sounds(), []);
        assert_eq!(
            escort_npc(&mut session, 0).target,
            Some(crate::combat::ShipRef::Npc(target))
        );
    }

    #[test]
    fn a_returning_fighter_takes_the_voice_with_none_chosen() {
        // The Viper's order changes and it returns: it speaks set 4, even.
        let catalog = voiced();
        let (mut session, _) = returning(&catalog, None);
        escort_npc(&mut session, 0).voice = Some(VoiceVariant::Even);
        assert!(command(&mut session, EscortCommand::Defend).is_some());
        assert_eq!(
            heard(
                &mut session,
                &catalog,
                &Peaceful,
                &mut Ranged::of(&[(4, 3)])
            ),
            [voice(1406)]
        );
    }

    // `_EscortAI`'s 1-in-3 call on a new target (@0x83cdc-0x83e34).

    /// `ships`' fleet defending the player, with a raider (an
    /// interceptor, which [`sitting_ai`] keeps as it is) 100 below the
    /// player attacking it: the session and the raider.
    fn raided(
        catalog: &FakePilotCatalog,
        ships: &[ShipId],
    ) -> (Session, crate::traffic::npc::NpcId) {
        let mut session = fleet(catalog, ships);
        for index in 0..ships.len() {
            order(&mut session, index, Some(EscortOrder::Defend));
        }
        let mut raider = crate::testkit::npc(0, ShipStats::new(FAST, &[]));
        raider.ai_type = AiType::Interceptor;
        raider.state.position = session.player().position + crate::geometry::Vec2::new(0.0, 100.0);
        raider.goal = Goal::Attack(crate::combat::ShipRef::Player);
        raider.target = Some(crate::combat::ShipRef::Player);
        let id = session.space.traffic.add_npc(raider);
        (session, id)
    }

    /// The voice sounds of `passes` advance passes of `session`, each a
    /// list, [`sitting_ai`] deciding, drawing on `chance`, every escort
    /// dropping its target before each, so each think searches anew.
    fn searching(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        passes: usize,
        chance: &mut Ranged,
    ) -> Vec<Vec<SimSound>> {
        (0..passes)
            .map(|_| {
                for index in 0..session.space.escorts.fleet.len() {
                    let npc = escort_npc(session, index);
                    npc.target = None;
                    npc.goal = Goal::Formation { guard: None };
                }
                heard(session, catalog, &sitting_ai(), chance)
            })
            .collect()
    }

    #[test]
    fn a_fighter_or_medium_escort_taking_a_threat_calls_it_on_a_rand_3_of_0() {
        // The call is queued at the think, after the pass's
        // `_HandleVoiceSound`, so it plays the pass after.
        let catalog = voiced();
        for (ship, clip) in [(MEDIUM, 1310), (VIPER, 1410)] {
            let (mut session, _) = raided(&catalog, &[ship]);
            escort_npc(&mut session, 0).voice = Some(VoiceVariant::Even);
            let mut chance = Ranged::of(&[(3, 0), (4, 0)]);
            assert_eq!(
                heard(&mut session, &catalog, &sitting_ai(), &mut chance),
                [],
                "{ship:?}"
            );
            assert_eq!(
                heard(&mut session, &catalog, &sitting_ai(), &mut chance),
                [voice(clip)],
                "{ship:?}"
            );
        }
    }

    /// `ships`' fleet raided as [`raided`] makes it, the first two
    /// escorts' berths swapped, so berth order runs against the fleet's
    /// order and the order the thinks queue their cues in.
    fn raided_swapped(
        catalog: &FakePilotCatalog,
        ships: &[ShipId],
    ) -> (Session, crate::traffic::npc::NpcId) {
        let (mut session, raider) = raided(catalog, ships);
        let berths = [
            escort_npc(&mut session, 0).berth,
            escort_npc(&mut session, 1).berth,
        ];
        assert!(berths[0] < berths[1]);
        escort_npc(&mut session, 0).berth = berths[1];
        escort_npc(&mut session, 1).berth = berths[0];
        (session, raider)
    }

    #[test]
    fn of_two_escorts_taking_a_threat_in_one_pass_the_lower_berth_calls_it() {
        // Both call their targets and both draw Rand(3) of 0, but the
        // first in berth order, the even medium, closes the gate: the
        // medium's call, said after, would overwrite it. Set 4's target
        // calls are 1410-1417, the even voice Rand(4) × 2; set 3's,
        // 1310-1312, Rand(3).
        let catalog = voiced();
        let (mut session, _) = raided_swapped(&catalog, &[MEDIUM, MEDIUM_EVEN]);
        for index in 0..2 {
            escort_npc(&mut session, index).voice = Some(VoiceVariant::Even);
        }
        let mut chance = Ranged::of(&[(3, 0), (4, 1)]);
        assert_eq!(
            heard(&mut session, &catalog, &sitting_ai(), &mut chance),
            []
        );
        assert_eq!(
            heard(&mut session, &catalog, &sitting_ai(), &mut chance),
            [voice(1412)],
            "the even medium's set 4, at the lower berth"
        );
    }

    #[test]
    fn of_two_escorts_with_no_variant_each_takes_one_in_the_same_pass() {
        // Each roll stands alone, so which escort draws first is not
        // asked. A Rand(2) of 1 gives the free medium the odd voice; the
        // other keeps its government's even one. Neither calls its
        // target, on Rand(3) of 1.
        let catalog = voiced();
        let (mut session, _) = raided_swapped(&catalog, &[MEDIUM, MEDIUM_EVEN]);
        heard(
            &mut session,
            &catalog,
            &sitting_ai(),
            &mut Ranged::of(&[(2, 1), (3, 1)]),
        );
        assert_eq!(
            [0, 1].map(|index| escort_npc(&mut session, index).voice),
            [Some(VoiceVariant::Odd), Some(VoiceVariant::Even)],
            "the free one rolled, the other forced"
        );
    }

    #[test]
    fn a_warship_freighter_or_mute_escort_never_calls_a_target_nor_does_a_draw_of_1() {
        let catalog = voiced();
        for ship in [WARSHIP, FREIGHTER, DRONE] {
            let (mut session, _) = raided(&catalog, &[ship]);
            let heard = searching(&mut session, &catalog, 3, &mut Ranged::of(&[(3, 0)]));
            assert!(heard.iter().all(Vec::is_empty), "{ship:?}: {heard:?}");
            assert!(session.space.voice.idle(), "{ship:?}");
        }
        let (mut session, _) = raided(&catalog, &[MEDIUM]);
        let heard = searching(&mut session, &catalog, 3, &mut Ranged::of(&[(3, 1)]));
        assert!(heard.iter().all(Vec::is_empty), "{heard:?}");
    }

    #[test]
    fn an_escort_keeping_its_target_calls_nothing() {
        let catalog = voiced();
        let (mut session, raider) = raided(&catalog, &[MEDIUM]);
        let target = crate::combat::ShipRef::Npc(raider);
        let npc = escort_npc(&mut session, 0);
        npc.target = Some(target);
        npc.goal = Goal::Attack(target);
        let mut chance = Ranged::of(&[(3, 0)]);
        for _ in 0..3 {
            assert_eq!(
                heard(&mut session, &catalog, &sitting_ai(), &mut chance),
                []
            );
            assert_eq!(
                escort_npc(&mut session, 0).goal,
                Goal::Attack(target),
                "kept"
            );
        }
    }

    #[test]
    fn no_target_is_called_until_the_pass_after_a_clip_has_played_out() {
        // The command's acknowledgement plays on the first pass, a second
        // long: 60 Mac ticks, 2 a pass. Its countdown reaches 0 at the
        // 31st pass's head, whose `_HandleVoiceSound` disposes it, so that
        // pass's think may call, and the call plays on the 32nd.
        let catalog = voiced();
        let (mut session, _) = raided(&catalog, &[MEDIUM]);
        order(&mut session, 0, None);
        assert!(command(&mut session, EscortCommand::Defend).is_some());
        let heard = searching(
            &mut session,
            &catalog,
            32,
            &mut Ranged::of(&[(3, 0), (5, 4)]),
        );
        assert_eq!(heard[0], [voice(1304)], "the acknowledgement");
        assert!(heard[1..31].iter().all(Vec::is_empty), "{heard:?}");
        assert_eq!(heard[31], [voice(1310)], "the call");
    }

    #[test]
    fn no_target_is_called_while_a_message_waits() {
        // The pass's `_HandleVoiceSound` (@0x44219) plays the command's
        // waiting message before `_AIDispatch` (@0x4429d), so the escort
        // takes the raider with its clip loaded, and calls no target.
        let catalog = voiced();
        let (mut session, raider) = raided(&catalog, &[MEDIUM]);
        order(&mut session, 0, None);
        assert!(command(&mut session, EscortCommand::Defend).is_some());
        session.take_sounds();
        work_with(
            &mut session,
            &catalog,
            Controls::default(),
            &sitting_ai(),
            Rules::default(),
            &mut Ranged::of(&[(3, 0)]),
        );
        assert_eq!(
            escort_npc(&mut session, 0).goal,
            Goal::Attack(crate::combat::ShipRef::Npc(raider)),
            "it took the raider"
        );
        assert_eq!(
            voiced_only(session.take_sounds()),
            [voice(1304)],
            "the acknowledgement alone"
        );
        assert_eq!(session.space.voice.pending(), None, "no target called");
    }

    #[test]
    fn the_separate_parts_answer_their_thinks_cues() {
        // `tick_traffic` answers the cues it queued, so none is left for
        // a later pass to answer on its own draws.
        let catalog = voiced();
        let (mut session, _) = raided(&catalog, &[MEDIUM]);
        work_with(
            &mut session,
            &catalog,
            Controls::default(),
            &sitting_ai(),
            Rules::default(),
            &mut Ranged::of(&[(2, 0), (3, 1)]),
        );
        assert_eq!(escort_npc(&mut session, 0).voice, Some(VoiceVariant::Even));
    }

    // `_HandleShipHit`'s victory (@0x36c7f-0x36d45).

    /// Keeps every ship's goal and target as they are.
    #[derive(Debug)]
    struct Keep;

    impl Behaviour for Keep {
        fn decide(
            &self,
            npc: &Npc,
            _around: &crate::ai::Surroundings,
            _chance: &mut dyn crate::chance::Chance,
        ) -> Goal {
            npc.goal
        }

        fn target(
            &self,
            npc: &Npc,
            _around: &crate::ai::Surroundings,
        ) -> Option<crate::combat::ShipRef> {
            npc.target
        }
    }

    /// The session tests' armed catalog, the player and its traffic each
    /// carrying a blaster, voiced, with a warship escort and a freighter
    /// escort of government 140, which speaks set 3.
    fn armed_voiced() -> FakePilotCatalog {
        let mut catalog = crate::session::tests::armed();
        catalog.ship_records.extend([
            ShipRecord {
                escort_type: 2,
                inherent_ai: 3,
                inherent_govt: Some(FREE),
                ..ship(WARSHIP.0, FAST)
            },
            ShipRecord {
                escort_type: 3,
                inherent_ai: 1,
                inherent_govt: Some(FREE),
                ..ship(FREIGHTER.0, FAST)
            },
        ]);
        catalog.hulls.extend([hull(WARSHIP.0), hull(FREIGHTER.0)]);
        catalog.voices = voices();
        catalog
    }

    /// `catalog`'s session facing its NPC, the victim, 100 above, its
    /// shield down and its armour at 1, and the player's fleet of `ships`
    /// beside the player, each attacking the victim: the session and the
    /// victim.
    fn attacking(
        catalog: &FakePilotCatalog,
        ships: &[ShipId],
    ) -> (Session, crate::combat::ShipRef) {
        let mut session = crate::session::tests::facing_an_npc(catalog, 180);
        let victim = &mut session.space.traffic.npcs_mut()[0];
        victim.reserves.shield.now = 0.0;
        victim.reserves.armor.now = 1.0;
        let victim = crate::combat::ShipRef::Npc(victim.id);
        for (index, &ship) in ships.iter().enumerate() {
            session.pilot.escorts.push(escort(ship));
            let placed = session.place_escort(index, &mut NeverFires);
            session.space.escorts.fleet.push(placed);
            let npc = escort_npc(&mut session, index);
            npc.goal = Goal::Attack(victim);
            npc.target = Some(victim);
        }
        (session, victim)
    }

    /// Fires the player's blaster at the victim and runs passes, [`Keep`]
    /// deciding, until the victim starts dying: the voice sounds of the
    /// pass it started dying on, and of the pass after.
    fn victory(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        victim: crate::combat::ShipRef,
    ) -> [Vec<SimSound>; 2] {
        let dying = |session: &Session| {
            session
                .npcs()
                .iter()
                .find(|npc| crate::combat::ShipRef::Npc(npc.id) == victim)
                .is_some_and(|npc| npc.condition.dying_or_destroyed(npc.reserves.armor))
        };
        session.hold_trigger(crate::session::tests::FIRE);
        for _ in 0..20 {
            let mut chance = Ranged::of(&[(5, 3)]);
            let now = heard(session, catalog, &Keep, &mut chance);
            session.hold_trigger(crate::combat::armament::Trigger::default());
            if dying(session) {
                return [now, heard(session, catalog, &Keep, &mut chance)];
            }
        }
        panic!("the victim never started dying");
    }

    #[test]
    fn an_escort_attacking_the_ship_a_shot_finishes_says_the_victory_that_pass() {
        // The collision passes come before `_HandleVoiceSound`, so the
        // victory plays in the pass that finished the victim: set 3's
        // victories 1320-1324, Rand(5) of 3.
        let catalog = armed_voiced();
        let (mut session, victim) = attacking(&catalog, &[WARSHIP]);
        assert_eq!(
            victory(&mut session, &catalog, victim),
            [vec![voice(1323)], vec![]]
        );
    }

    #[test]
    fn a_trader_escort_or_one_on_another_ship_says_no_victory() {
        let catalog = armed_voiced();
        let (mut session, victim) = attacking(&catalog, &[FREIGHTER]);
        assert_eq!(
            victory(&mut session, &catalog, victim),
            [vec![], vec![]],
            "a trader"
        );
        let (mut session, victim) = attacking(&catalog, &[WARSHIP]);
        let elsewhere = crate::combat::ShipRef::Player;
        let npc = escort_npc(&mut session, 0);
        npc.target = Some(elsewhere);
        npc.goal = Goal::Attack(elsewhere);
        assert_eq!(
            victory(&mut session, &catalog, victim),
            [vec![], vec![]],
            "on another"
        );
    }

    #[test]
    fn of_two_escorts_attacking_the_victim_the_higher_berth_says_the_victory() {
        // Each speaks in slot order, so the last stands: the trader of the
        // two speaks none, so the warship's set 3, then the freighter's
        // class swapped to speak set 4 shows which.
        let mut catalog = armed_voiced();
        for record in &mut catalog.ship_records {
            if record.id == FREIGHTER {
                record.inherent_ai = 3;
                record.inherent_govt = Some(EVEN);
            }
        }
        let (mut session, victim) = attacking(&catalog, &[WARSHIP, FREIGHTER]);
        let berths = [
            escort_npc(&mut session, 0).berth,
            escort_npc(&mut session, 1).berth,
        ];
        assert!(berths[0] < berths[1]);
        let heard = victory(&mut session, &catalog, victim);
        assert!(
            matches!(heard[0].as_slice(), [SimSound::Voice { sound }] if (1420..1428).contains(&sound.0)),
            "the freighter's set 4: {heard:?}"
        );
        let (mut session, victim) = attacking(&catalog, &[WARSHIP, FREIGHTER]);
        escort_npc(&mut session, 0).berth = berths[1] + 1;
        assert_eq!(
            victory(&mut session, &catalog, victim)[0],
            [voice(1323)],
            "the warship, higher now"
        );
    }

    #[test]
    fn an_escort_the_phase_lost_before_the_finishing_hit_says_no_victory() {
        // `_DamageShip` takes a disabled escort out of the fleet as its hit
        // lands (@0x3afdd), so a later hit's victory never reads it led.
        let mut catalog = armed_voiced();
        for record in &mut catalog.ship_records {
            if record.id == FREIGHTER {
                record.inherent_ai = 3;
                record.inherent_govt = Some(EVEN);
            }
        }
        let (mut session, victim) = attacking(&catalog, &[WARSHIP, FREIGHTER]);
        let higher = crate::combat::ShipRef::Npc(escort_npc(&mut session, 1).id);
        let said = |session: &mut Session, lost_at: usize| {
            session.space.voice = crate::voice::VoiceChannel::default();
            session.space.speak_victories(
                &session.statics,
                &[FinishingHit {
                    ship: victim,
                    strike: 1,
                }],
                &[EscortLoss {
                    ship: higher,
                    strike: lost_at,
                }],
            );
            session.space.voice.pending().and_then(|speech| speech.set)
        };
        assert_eq!(said(&mut session, 0), Some(3), "lost first: the warship");
        assert_eq!(
            said(&mut session, 1),
            Some(4),
            "lost by that hit: still led"
        );
        assert_eq!(said(&mut session, 2), Some(4), "lost after");
    }

    #[test]
    fn a_victory_overwrites_a_command_given_before_the_pass() {
        let catalog = armed_voiced();
        let (mut session, _) = attacking(&catalog, &[WARSHIP]);
        order(&mut session, 0, Some(EscortOrder::Attack));
        // The finishing pass is the hit's: give the command just before
        // it by running every pass but that one first.
        let mut probe = session.clone();
        let mut passes = 0;
        probe.hold_trigger(crate::session::tests::FIRE);
        loop {
            passes += 1;
            heard(&mut probe, &catalog, &Keep, &mut Ranged::of(&[]));
            probe.hold_trigger(crate::combat::armament::Trigger::default());
            if probe.npcs()[0]
                .condition
                .dying_or_destroyed(probe.npcs()[0].reserves.armor)
            {
                break;
            }
        }
        session.hold_trigger(crate::session::tests::FIRE);
        for _ in 1..passes {
            heard(&mut session, &catalog, &Keep, &mut Ranged::of(&[]));
            session.hold_trigger(crate::combat::armament::Trigger::default());
        }
        assert!(command(&mut session, EscortCommand::Defend).is_some());
        assert_eq!(
            heard(&mut session, &catalog, &Keep, &mut Ranged::of(&[(5, 3)])),
            [voice(1323)],
            "the victory, not the acknowledgement"
        );
    }

    // `_HandleVoiceSound`'s halt and `_KillVoiceSound`.

    #[test]
    fn a_command_while_a_clip_plays_halts_it_then_plays_the_next_pass() {
        let catalog = voiced();
        let mut session = fleet(&catalog, &[WARSHIP]);
        assert!(command(&mut session, EscortCommand::Defend).is_some());
        let mut chance = Ranged::of(&[(5, 2)]);
        assert_eq!(
            heard(&mut session, &catalog, &Peaceful, &mut chance),
            [voice(1302)]
        );
        assert!(command(&mut session, EscortCommand::Hold).is_some());
        assert_eq!(
            heard(&mut session, &catalog, &Peaceful, &mut chance),
            [halt(1302)]
        );
        assert_eq!(
            heard(&mut session, &catalog, &Peaceful, &mut chance),
            [voice(1302)]
        );
    }

    #[test]
    fn landing_halts_the_clip_playing_and_drops_the_message_waiting() {
        let catalog = voiced();
        let mut session = fleet(&catalog, &[WARSHIP]);
        assert!(command(&mut session, EscortCommand::Defend).is_some());
        heard(
            &mut session,
            &catalog,
            &Peaceful,
            &mut Ranged::of(&[(5, 2)]),
        );
        assert!(command(&mut session, EscortCommand::Hold).is_some());
        crate::testkit::land_now(&mut session).expect("lands");
        assert_eq!(voiced_only(session.take_sounds()), [halt(1302)]);
        assert!(session.space.voice.idle());
    }
}
