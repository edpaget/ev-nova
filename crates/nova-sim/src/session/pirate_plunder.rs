//! The session's side of a pirate's plunder (`_AIPlunderShipContents`
//! @0x83fab-0x84a2d), which the traffic queues as
//! [`PirateEvent`]s and this drains right after the traffic's tick
//! ([`Session::answer_pirates`]).
//!
//! - **A dock with the player** ([`PirateEvent::PlayerDockedWith`]): the
//!   player is boarded ([`Session::player_boarded`], its slot's +0xb9,
//!   `_LowLevelAIHandler` @0x86fc7), so pirates pass it over or wait on it
//!   ([`PlayerSide::boarded`](crate::ai::PlayerSide::boarded)). It is
//!   cleared on each tick the player isn't disabled, as `_HandlePlayer`
//!   clears it each frame (@0x68b02), and kept only in flight: a ship slot
//!   field, it is never saved.
//! - **A plunder of the player** ([`PirateEvent::PlunderPlayer`]): the odds
//!   from the pirate's class `Strength` and marines against the player's
//!   class's raw `Strength` and the marines it owns
//!   ([`plunder_contents::odds`], `Rand(21)` drawn); its commodities 0-5,
//!   no `jünk` or mission cargo, moved into the pirate's free holds, no
//!   more than the player's ship's holds (`_ShipTotalHolds` @0x84007,
//!   [`plunder_contents::steal`]); 3-30 % of its credits
//!   ([`plunder_contents::credits_taken`]); the line "N tons of cargo and
//!   M credits stolen!" ([`SimMessage::CargoStolen`], 400 ticks) with
//!   `_beepSnd[1]` ([`SimSound::CargoStolen`], `snd ` 151) when anything
//!   went; then each active mission not yet failed whose `Flags` hold
//!   0x8000 ([`FAILS_IF_PLUNDERED`]) fails at once, in slot order
//!   (`_DoMissionFailure` @0x84a0f, its `OnFailure`, record and Fail text
//!   included).
//! - **A plunder of an NPC** ([`PirateEvent::Plundered`]): its cargo line,
//!   for a former hired or captured escort, then its capture's line,
//!   "Escort stolen!" or "Fighter stolen!" ([`SimMessage::ShipStolen`],
//!   400 ticks), each with `snd ` 151 ([`SimSound::ShipStolen`]).
//!
//! It all settles in the tick the pirate plunders, before the frame is
//! drawn and before any dialog opens; the boarded flag a tick after the
//! player is repaired, which the player never sees. The pirate's credits
//! (+0xa0) are read by no one, so the credits it gains are not kept.

use super::Session;
#[cfg(doc)]
use crate::ai::plunder_contents;
use crate::ai::plunder_contents::{
    Marines, Side, Taken, Victim, cargo_line, credits_taken, roll_odds, steal,
};
use crate::catalog::outfit_mods;
use crate::chance::Chance;
use crate::combat::hull::Condition;
use crate::market::Good;
use crate::message::SimMessage;
use crate::mission::FailedBy;
use crate::rulebook::RuleKey;
use crate::sound::SimSound;
use crate::traffic::npc::{NPC_COMMODITIES, NpcId};
use crate::traffic::plunder::PirateEvent;

/// `mïsn` `Flags` 0x8000: the mission fails when pirates plunder the
/// player (`_AIPlunderShipContents` @0x84a04: the record's +0x58 below 0).
pub const FAILS_IF_PLUNDERED: u16 = 0x8000;

impl Session {
    /// Answers what the pirates did this tick, drawing on `chance` (see
    /// the module docs).
    pub(super) fn answer_pirates(&mut self, chance: &mut dyn Chance) {
        if self.condition != Condition::Disabled {
            self.player_boarded = false;
        }
        for event in self.traffic.take_pirate_events() {
            match event {
                PirateEvent::PlayerDockedWith { .. } => self.player_boarded = true,
                PirateEvent::PlunderPlayer { pirate } => self.plundered_by(pirate, chance),
                PirateEvent::Plundered { line, taken, .. } => {
                    if let Some(line) = line {
                        self.say_stolen_cargo(line.tons, line.credits);
                    }
                    if let Some(Taken::Escort | Taken::Fighter) = taken {
                        self.sounds.push(SimSound::ShipStolen);
                        self.say(SimMessage::ShipStolen {
                            fighter: taken == Some(Taken::Fighter),
                        });
                    }
                }
            }
        }
    }

    /// Whether a pirate has docked with the player's ship (+0xb9), while it
    /// is disabled (see the module docs).
    #[must_use]
    pub fn player_boarded(&self) -> bool {
        self.player_boarded
    }

    /// The player plundered by NPC `pirate`, drawing on `chance` (see the
    /// module docs): nothing when the pirate has gone.
    fn plundered_by(&mut self, pirate: NpcId, chance: &mut dyn Chance) {
        let reading = self.rules.source_for(RuleKey::PlunderMarines);
        let Some(npc) = self.traffic.npcs().iter().find(|npc| npc.id == pirate) else {
            return;
        };
        let attacker = Side {
            strength: npc.marines.strength,
            marines: npc.marines.read(reading),
        };
        let (mut hold, pirate_holds) = (npc.cargo, npc.holds);
        let defender = Side {
            strength: self
                .ship_record(self.pilot.ship)
                .map_or(0, |record| i32::from(record.strength)),
            marines: Marines::owned(
                outfit_mods(&self.pilot.outfits, &self.outfits)
                    .into_iter()
                    .map(|outfit| (outfit.mod_type, outfit.mod_val, outfit.count)),
            ),
        };
        let odds = roll_odds(attacker, defender, chance);
        let mut words: [u32; NPC_COMMODITIES] = std::array::from_fn(|commodity| {
            // In 0..6.
            #[allow(clippy::cast_possible_truncation)]
            let good = Good::Commodity(commodity as u8);
            self.pilot.cargo.get(&good).copied().unwrap_or(0)
        });
        let tons = steal(
            &mut words,
            self.stats.capacity,
            &mut hold,
            pirate_holds,
            chance,
        );
        for (commodity, tons) in words.into_iter().enumerate() {
            // In 0..6.
            #[allow(clippy::cast_possible_truncation)]
            let good = Good::Commodity(commodity as u8);
            if tons == 0 {
                self.pilot.cargo.remove(&good);
            } else {
                self.pilot.cargo.insert(good, tons);
            }
        }
        if let Some(npc) = self.npc_mut(pirate) {
            npc.cargo = hold;
        }
        let credits = credits_taken(self.pilot.cash, odds);
        self.pilot.cash -= credits;
        if let Some(line) = cargo_line(Victim::Player, tons, credits) {
            self.say_stolen_cargo(line.tons, line.credits);
        }
        let plundered: Vec<u8> = self
            .pilot
            .missions
            .iter()
            .filter(|(_, active)| active.failed.is_none())
            .filter(|(_, active)| {
                self.lifecycle_record(active.mission).flags & FAILS_IF_PLUNDERED != 0
            })
            .map(|(slot, _)| slot)
            .collect();
        for slot in plundered {
            self.mission_failure_at(slot, FailedBy::Marked, chance);
        }
    }

    /// The cargo line of a plunder of `tons` and `credits`, with its beep.
    fn say_stolen_cargo(&mut self, tons: u32, credits: i64) {
        self.sounds.push(SimSound::CargoStolen);
        self.say(SimMessage::CargoStolen { tons, credits });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::pirate::PirateWarship;
    use crate::ai::plunder_contents::{CargoLine, ClassMarines};
    use crate::catalog::{JunkId, MARINES, MissionRecord, OutfitId, ShipRecord};
    use crate::combat::ShipRef;
    use crate::control::{Bit, Script};
    use crate::session::tests::{armed, facing_an_npc};
    use crate::testkit::{Draws, FAST, FakePilotCatalog, carrying, mission, outfit, ship};
    use crate::traffic::plunder_dock::{DockSub, Docking};

    fn bit(n: u16) -> Bit {
        Bit::new(n).expect("in range")
    }

    /// Mission `id` with `Flags` `flags`, its `OnFailure` setting bit
    /// `id` - 100.
    fn plunderable(id: i16, flags: u16) -> MissionRecord {
        MissionRecord {
            flags,
            on_failure: Script::parse(&format!("b{}", id - 100)),
            ..mission(id)
        }
    }

    /// [`armed`] holding `missions`.
    fn catalog_of(missions: Vec<MissionRecord>) -> FakePilotCatalog {
        FakePilotCatalog {
            missions,
            ..armed()
        }
    }

    /// A session of `catalog` whose NPC is a pirate of `Strength` 1 with
    /// free holds of `holds`, the player disabled with no `Strength`,
    /// holding 30 tons of commodity 1, 5 of commodity 3 and 10 of a
    /// `jünk`, and 10,000 credits, its sounds and messages taken.
    fn plundering(catalog: &FakePilotCatalog, holds: u16) -> (Session, NpcId) {
        let mut session = facing_an_npc(catalog, 180);
        let pirate = session.npcs()[0].id;
        let npc = session.npc_mut(pirate).expect("here");
        npc.holds = holds;
        npc.cargo = crate::traffic::npc::NpcHold::default();
        npc.marines = ClassMarines {
            strength: 1,
            ..ClassMarines::default()
        };
        session.condition = Condition::Disabled;
        session.pilot.cargo.clear();
        session.pilot.cargo.insert(Good::Commodity(1), 30);
        session.pilot.cargo.insert(Good::Commodity(3), 5);
        session.pilot.cargo.insert(Good::Junk(JunkId(128)), 10);
        session.pilot.cash = 10_000;
        let _ = session.take_sounds();
        let _ = session.take_messages();
        (session, pirate)
    }

    /// `session` answering a plunder of the player by `pirate`, every draw
    /// `drawn`.
    fn answer(session: &mut Session, pirate: NpcId, drawn: u32) {
        session
            .traffic
            .queue_pirate_event(PirateEvent::PlunderPlayer { pirate });
        session.answer_pirates(&mut Draws::of(&[drawn; 16]));
    }

    #[test]
    fn a_plundered_player_loses_its_commodities_up_to_the_pirates_free_holds() {
        let catalog = catalog_of(vec![]);
        let (mut session, pirate) = plundering(&catalog, 20);
        answer(&mut session, pirate, 0);
        let left = |good| session.pilot.cargo.get(&good).copied().unwrap_or(0);
        assert_eq!(
            left(Good::Commodity(1)) + left(Good::Commodity(3)),
            15,
            "20 of 35 taken"
        );
        assert_eq!(left(Good::Junk(JunkId(128))), 10, "no jünk");
        let hold = session.npc(pirate).expect("here").cargo;
        assert_eq!(hold.tons(1) + hold.tons(3), 20);
    }

    #[test]
    fn a_plundered_player_loses_no_more_than_its_ships_holds() {
        let catalog = catalog_of(vec![]);
        let (mut session, pirate) = plundering(&catalog, 500);
        let capacity = session.stats.capacity;
        session
            .pilot
            .cargo
            .insert(Good::Commodity(1), capacity + 40);
        answer(&mut session, pirate, 0);
        let hold = session.npc(pirate).expect("here").cargo;
        assert_eq!(hold.tons(1) + hold.tons(3), capacity, "_ShipTotalHolds");
    }

    #[test]
    fn a_plundered_player_loses_the_odds_share_of_its_credits_and_is_told() {
        // Strengths 1 and none: a ratio of 50, odds of 60 less the jitter.
        let catalog = catalog_of(vec![]);
        for (jitter, cash) in [(0, 10_000 - 1800), (20, 10_000 - 1200)] {
            let (mut session, pirate) = plundering(&catalog, 0);
            answer(&mut session, pirate, jitter);
            assert_eq!(session.pilot.cash, cash, "jitter {jitter}");
            assert_eq!(
                session.take_messages(),
                [SimMessage::CargoStolen {
                    tons: 0,
                    credits: 10_000 - cash
                }]
            );
            assert_eq!(session.take_sounds(), [SimSound::CargoStolen]);
        }
    }

    /// The cash left after a plunder of [`plundering`]'s player by its
    /// pirate in a session of `catalog`, the pilot owning `owned`, every
    /// draw 0 (no jitter).
    fn cash_after(catalog: &FakePilotCatalog, owned: &[(i16, u16)]) -> i64 {
        let (mut session, pirate) = plundering(catalog, 0);
        for &(id, count) in owned {
            session.pilot.outfits.insert(OutfitId(id), count);
        }
        answer(&mut session, pirate, 0);
        session.pilot.cash
    }

    #[test]
    fn the_players_class_strength_lowers_the_odds_and_so_the_credits_taken() {
        // The pirate's Strength 1 against the player's class's 2: a ratio
        // of 25, odds of 35. The pirate's own class (129) has none.
        let mut catalog = catalog_of(vec![]);
        catalog.ship_records.push(ShipRecord {
            strength: 2,
            ..ship(128, FAST)
        });
        assert_ne!(credits_taken(10_000, 35), 1800, "not the odds of 60");
        assert_eq!(
            cash_after(&catalog, &[]),
            10_000 - credits_taken(10_000, 35)
        );
    }

    #[test]
    fn the_players_own_marines_crew_lowers_the_odds_and_so_the_credits_taken() {
        // Three marines of ModVal 1: a crew of 3 beside no Strength, a
        // ratio of 16, odds of 26.
        let mut catalog = catalog_of(vec![]);
        catalog.outfits.push(outfit(260, &[(MARINES, 1)]));
        assert_eq!(
            cash_after(&catalog, &[(260, 3)]),
            10_000 - credits_taken(10_000, 26)
        );
    }

    #[test]
    fn the_players_own_marines_defence_lowers_the_odds_and_so_the_credits_taken() {
        // Four of ModVal -5: a defence of -20, so a ratio of 50 less 20,
        // odds of 40.
        let mut catalog = catalog_of(vec![]);
        catalog.outfits.push(outfit(261, &[(MARINES, -5)]));
        assert_eq!(
            cash_after(&catalog, &[(261, 4)]),
            10_000 - credits_taken(10_000, 40)
        );
    }

    #[test]
    fn a_plunder_that_takes_nothing_says_nothing() {
        let catalog = catalog_of(vec![]);
        let (mut session, pirate) = plundering(&catalog, 0);
        session.pilot.cash = 0;
        answer(&mut session, pirate, 0);
        assert_eq!(session.take_messages(), []);
        assert_eq!(session.take_sounds(), []);
    }

    #[test]
    fn the_plunder_fails_every_unfailed_mission_flagged_0x8000_after_its_line() {
        let catalog = catalog_of(vec![
            plunderable(140, 0x8000),
            plunderable(141, 0x8000),
            plunderable(142, 0x0400),
            plunderable(143, 0x8000),
        ]);
        let (mut session, pirate) = plundering(&catalog, 20);
        for (slot, id) in [(0, 140), (1, 141), (2, 142), (3, 143)] {
            let mut active = carrying(id, 0);
            if id == 143 {
                active.failed = Some(FailedBy::Quick);
            }
            assert!(session.pilot.missions.put(slot, active));
        }
        answer(&mut session, pirate, 0);
        assert_eq!(session.pilot.missions.get(0), None, "failed and freed");
        assert_eq!(session.pilot.missions.get(1), None);
        assert!(session.pilot.missions.get(2).is_some(), "no 0x8000");
        assert!(session.pilot.missions.get(3).is_some(), "failed already");
        assert!(session.pilot.control_bit(bit(40)), "its OnFailure ran");
        assert!(session.pilot.control_bit(bit(41)));
        assert!(!session.pilot.control_bit(bit(42)));
        assert_eq!(
            session.take_messages().first(),
            Some(&SimMessage::CargoStolen {
                tons: 20,
                credits: 1800
            }),
            "the line first"
        );
    }

    #[test]
    fn a_pirate_gone_plunders_nothing() {
        let catalog = catalog_of(vec![]);
        let (mut session, _) = plundering(&catalog, 20);
        answer(&mut session, NpcId(9999), 0);
        assert_eq!(session.pilot.cash, 10_000);
        assert_eq!(session.take_messages(), []);
    }

    #[test]
    fn a_dock_boards_the_player_until_it_is_no_longer_disabled() {
        let catalog = catalog_of(vec![]);
        let (mut session, pirate) = plundering(&catalog, 20);
        assert!(!session.player_boarded());
        session
            .traffic
            .queue_pirate_event(PirateEvent::PlayerDockedWith { pirate });
        session.answer_pirates(&mut Draws::of(&[]));
        assert!(session.player_boarded());
        assert!(session.player_side().boarded);
        session.answer_pirates(&mut Draws::of(&[]));
        assert!(session.player_boarded(), "still disabled");
        session.condition = Condition::Intact;
        session.answer_pirates(&mut Draws::of(&[]));
        assert!(!session.player_boarded(), "@0x68b02");
        assert!(!session.player_side().boarded);
    }

    #[test]
    fn an_npcs_plunder_says_its_cargo_line_and_its_captures_line_with_their_beeps() {
        let catalog = catalog_of(vec![]);
        let (mut session, pirate) = plundering(&catalog, 20);
        let plundered = |line, taken| PirateEvent::Plundered {
            pirate,
            victim: NpcId(77),
            tons: 3,
            line,
            taken,
        };
        for event in [
            plundered(
                Some(CargoLine {
                    tons: 3,
                    credits: 0,
                }),
                Some(Taken::Escort),
            ),
            plundered(None, Some(Taken::Fighter)),
            plundered(None, Some(Taken::Silently)),
            plundered(None, None),
        ] {
            session.traffic.queue_pirate_event(event);
        }
        session.answer_pirates(&mut Draws::of(&[]));
        assert_eq!(
            session.take_messages(),
            [
                SimMessage::CargoStolen {
                    tons: 3,
                    credits: 0
                },
                SimMessage::ShipStolen { fighter: false },
                SimMessage::ShipStolen { fighter: true },
            ]
        );
        assert_eq!(
            session.take_sounds(),
            [
                SimSound::CargoStolen,
                SimSound::ShipStolen,
                SimSound::ShipStolen
            ]
        );
    }

    #[test]
    fn a_docked_pirates_plunder_reaches_the_player_in_the_traffics_tick() {
        let catalog = catalog_of(vec![plunderable(140, 0x8000)]);
        let (mut session, pirate) = plundering(&catalog, 20);
        assert!(session.pilot.missions.put(0, carrying(140, 0)));
        session.player_boarded = true;
        session.hull.crew = 1;
        let npc = session.npc_mut(pirate).expect("here");
        npc.goal = crate::ai::Goal::Attack(ShipRef::Player);
        npc.target = Some(ShipRef::Player);
        npc.docking = Some(Docking {
            ship: ShipRef::Player,
            sub: DockSub::Dock,
        });
        npc.ai_wait = 0;
        session.tick_traffic(
            &catalog,
            &PirateWarship::default(),
            &mut Draws::of(&[0; 64]),
        );
        assert_eq!(
            session.npc(pirate).map(|npc| npc.goal),
            Some(crate::ai::Goal::Drift)
        );
        assert!(session.pilot.cash < 10_000);
        assert_eq!(session.pilot.missions.get(0), None, "failed");
        assert!(session.take_sounds().contains(&SimSound::CargoStolen));
    }
}
