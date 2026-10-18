//! The words outside the fight that a beam of stray `ExitType` steps (rdm
//! weapons-fidelity phase 31): `_SpawnBeam` reads `ExitType` signed and
//! indexes the cursors at +0xc8fe with it, unchecked, so a beam of
//! `ExitType` 5 steps +0xc908 as a cursor (@0x450fa-0x4523d): a word above
//! 3, read unsigned, is first drawn anew with `Rand(4)` (@0x450ff-0x45116),
//! then it moves on one by C's signed remainder by 4 (@0x45220-0x4523d;
//! `nova_sim::combat::exits::step_stray`). The fight raises each step
//! (`Combat::take_stray_steps`); the session steps the word here, by the
//! rulebook's [`RuleKey::StrayBeamExit`](crate::rulebook::RuleKey::StrayBeamExit).
//!
//! +0xc908 is two things:
//!
//! - On an NPC it is the crew pods left ([`Npc::pods_left`]), written raw
//!   from its class's `PodCount` (@0x3c4b0, @0x3ca48, @0x3cff7, @0x3d8fe),
//!   so a negative count steps through the draw.
//! - On the player it is the escape-pod countdown, -1 while flying
//!   (`_DoNewPilot` @0x1924b, `_ResetPlayer` @0x1d75b, `_HandlePlayer`
//!   @0x6f563, `_OpenPilotFile` @0x766b5), 350 from the eject (@0x6f669).
//!   `_HandlePlayer` flies the pod while it is above 0 (@0x68d1d) and
//!   rescues the pilot at 0. So a flying player stepped to 1-3 flies the
//!   pod and is rescued without having ejected; one stepped to 0 keeps 0,
//!   not -1, until the next writer of -1, and the next such beam steps it
//!   to 1 without a draw. The engine holds the countdown as
//!   `Session::pod` (none for -1 or 0) and the 0 outside the pod as
//!   `Session::pod_zeroed` ([`PodWord`]).
//!
//! The original steps at the beam's spawn; the engine settles after the
//! pass's fight, in the same tick, before the frame is drawn, which no
//! boundary the player can see lies between. No sound plays (phase 18).

use super::Session;
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::combat::exits::{StrayWord, step_stray};

/// Steps the signed ship word `word` as `_SpawnBeam` steps a cursor, read
/// unsigned for the redraw (`cmpw $3; jbe` @0x450ff).
fn step_word(word: i16, chance: &mut dyn Chance) -> i16 {
    let mut raw = word.cast_unsigned();
    step_stray(&mut raw, chance);
    raw.cast_signed()
}

/// The player's +0xc908 as the engine holds it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PodWord {
    /// The countdown while above 0 (`Session::pod`).
    pod: Option<u32>,
    /// Whether it is 0 outside the pod (`Session::pod_zeroed`).
    zeroed: bool,
}

impl PodWord {
    /// The word: the countdown, else 0 when zeroed, else -1.
    fn word(self) -> i16 {
        match self.pod {
            Some(left) => i16::try_from(left).unwrap_or(i16::MAX),
            None if self.zeroed => 0,
            None => -1,
        }
    }

    /// The state the word `word` gives.
    fn of(word: i16) -> Self {
        Self {
            pod: u32::try_from(word).ok().filter(|&left| left > 0),
            zeroed: word == 0,
        }
    }
}

impl Session {
    /// Steps each word the fight's stray beams raised
    /// (`Combat::take_stray_steps`), drawing on `chance`: an NPC's crew
    /// pods left, skipped if it is gone, or the player's pod countdown
    /// (see the module docs).
    pub(super) fn step_stray_words(&mut self, chance: &mut dyn Chance) {
        for (ship, word) in self.combat.take_stray_steps() {
            match (ship, word) {
                (ShipRef::Npc(id), StrayWord::Pods) => {
                    if let Some(npc) = self.npc_mut(id) {
                        npc.pods_left = step_word(npc.pods_left, chance);
                    }
                }
                (ShipRef::Player, StrayWord::Pods) => self.step_pod_word(chance),
                // Stepped within the fight.
                (_, StrayWord::FoldSet) => {}
            }
        }
    }

    /// Steps the player's +0xc908, drawing on `chance` (see the module
    /// docs).
    fn step_pod_word(&mut self, chance: &mut dyn Chance) {
        let held = PodWord {
            pod: self.pod,
            zeroed: self.pod_zeroed,
        };
        let stepped = PodWord::of(step_word(held.word(), chance));
        self.pod = stepped.pod;
        self.pod_zeroed = stepped.zeroed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::WeaponRecord;
    use crate::chance::NeverFires;
    use crate::combat::Rules;
    use crate::player_fate::PlayerFate;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::session::tests::{FIRE, armed, blaster, facing_an_npc};
    use crate::testkit::FakePilotCatalog;
    use nova_sim_base::fixture::{Draws, Ranged};

    #[test]
    fn a_word_of_0_to_3_steps_on_one_round_from_3_without_a_draw() {
        for (word, stepped) in [(0, 1), (2, 3), (3, 0)] {
            let mut draws = Draws::of(&[]);
            assert_eq!(step_word(word, &mut draws), stepped, "{word}");
            assert!(draws.asked.is_empty(), "{word}: no draw");
        }
    }

    #[test]
    fn a_word_above_3_unsigned_is_drawn_below_4_then_stepped() {
        for word in [5, -1, -3, i16::MIN] {
            for draw in 0..4 {
                let mut draws = Draws::of(&[draw]);
                let stepped = step_word(word, &mut draws);
                assert_eq!(stepped, (draw as i16 + 1) % 4, "{word}, {draw}");
                assert_eq!(draws.asked, [4], "{word}");
            }
        }
    }

    #[test]
    fn the_players_pod_word_is_the_countdown_else_0_when_zeroed_else_minus_1() {
        let word = |pod, zeroed| PodWord { pod, zeroed }.word();
        assert_eq!(word(None, false), -1);
        assert_eq!(word(None, true), 0);
        assert_eq!(word(Some(3), false), 3);
        assert_eq!(word(Some(350), false), 350);
        assert_eq!(word(Some(u32::MAX), false), i16::MAX);
        let of = |w| {
            let PodWord { pod, zeroed } = PodWord::of(w);
            (pod, zeroed)
        };
        assert_eq!(of(-1), (None, false));
        assert_eq!(of(0), (None, true));
        assert_eq!(of(1), (Some(1), false));
        assert_eq!(of(3), (Some(3), false));
    }

    /// A beam of `ExitType` 5 that fires every tick.
    fn pod_beam() -> WeaponRecord {
        WeaponRecord {
            guidance: 0,
            reload: 0,
            count: 1,
            beam_length: 300,
            exit_type: 5,
            ..blaster()
        }
    }

    /// [`armed`] with [`pod_beam`] for its weapon, its NPC firing one too.
    fn beamed() -> FakePilotCatalog {
        FakePilotCatalog {
            weapons: vec![pod_beam()],
            ..armed()
        }
    }

    /// `catalog`'s session facing its NPC by `reading` of
    /// [`RuleKey::StrayBeamExit`].
    fn session(catalog: &FakePilotCatalog, reading: RuleSource) -> Session {
        let rules = Rulebook::default().with_override(RuleKey::StrayBeamExit, reading);
        facing_an_npc(catalog, 180).with_rules(rules)
    }

    /// The player fires one tick of its beam on `chance` and lets go.
    fn fire(session: &mut Session, chance: &mut dyn Chance) {
        session.hold_trigger(FIRE);
        session.tick_combat(Rules::default(), chance);
        session.hold_trigger(crate::combat::armament::Trigger::default());
    }

    #[test]
    fn a_flying_player_firing_a_beam_of_exit_type_5_is_stepped_into_the_pod_and_rescued() {
        let catalog = beamed();
        let mut session = session(&catalog, RuleSource::Engine);
        fire(&mut session, &mut Ranged::of(&[(4, 0)]));
        assert_eq!(session.pod, Some(1), "-1 drawn to 0, then on to 1");
        assert!(matches!(session.player_fate(), PlayerFate::Ejected { .. }));
        session.tick(crate::flight::Controls::default());
        assert!(session.rescue_due, "the pod flew its last tick");
        assert!(session.rescue_if_due(&catalog, &mut NeverFires));
        assert!(session.take_rescue_desc().is_some(), "the rescue ran");
    }

    #[test]
    fn a_player_stepped_to_0_stays_out_of_the_pod_and_steps_to_1_next_time_without_a_draw() {
        let catalog = beamed();
        let mut session = session(&catalog, RuleSource::Engine);
        fire(&mut session, &mut Ranged::of(&[(4, 3)]));
        assert_eq!((session.pod, session.pod_zeroed), (None, true));
        assert_eq!(session.player_fate(), PlayerFate::Flying);
        fire(&mut session, &mut Ranged::of(&[]));
        assert_eq!((session.pod, session.pod_zeroed), (Some(1), false));
    }

    #[test]
    fn a_zeroed_pod_word_reads_as_flying_after_landing() {
        // `_ResetPlayer` from `_HandlePlayerDockRequest` writes -1
        // (@0x6709b), so the next stray beam draws again.
        let catalog = armed();
        let mut session = Session::start(&catalog).expect("starts");
        session.pod_zeroed = true;
        assert!(crate::testkit::land_now(&mut session).is_ok());
        session.take_off(&catalog, &mut NeverFires);
        assert_eq!((session.pod, session.pod_zeroed), (None, false));
        let mut chance = Ranged::of(&[(4, 1)]);
        session.step_pod_word(&mut chance);
        assert_eq!(chance.asked, [4], "-1 is drawn");
        assert_eq!(session.pod, Some(2));
    }

    #[test]
    fn a_zeroed_pod_word_reads_as_flying_after_a_gates_exit() {
        // The gate's exit runs the dock request's `_ResetPlayer` too
        // (@0x6709b).
        let catalog = crate::session::tests::gated();
        let mut session = crate::session::tests::at_gate(&catalog);
        session.pod_zeroed = true;
        session
            .enter_hypergate(
                Some(crate::catalog::SystemId(131)),
                &catalog,
                &mut crate::testkit::Scripted::default(),
            )
            .expect("enters");
        assert_eq!((session.pod, session.pod_zeroed), (None, false));
    }

    #[test]
    fn a_player_in_the_pod_is_drawn_and_stepped() {
        let catalog = beamed();
        let mut session = session(&catalog, RuleSource::Engine);
        session.pod = Some(10);
        let mut chance = Ranged::of(&[(4, 2)]);
        session.step_pod_word(&mut chance);
        assert_eq!(session.pod, Some(3));
        session.step_pod_word(&mut chance);
        assert_eq!((session.pod, session.pod_zeroed), (None, true), "3 to 0");
    }

    #[test]
    fn by_the_other_reading_the_players_beam_steps_no_pod_word() {
        let catalog = beamed();
        let mut session = session(&catalog, RuleSource::Bible);
        fire(&mut session, &mut Ranged::of(&[(4, 0)]));
        assert_eq!((session.pod, session.pod_zeroed), (None, false));
        assert_eq!(session.player_fate(), PlayerFate::Flying);
    }

    #[test]
    fn an_npc_firing_a_beam_of_exit_type_5_steps_its_crew_pods_left_by_the_engine() {
        let catalog = beamed();
        for (reading, held, draw, stepped) in [
            (RuleSource::Engine, 2, 0, 3),
            (RuleSource::Engine, 3, 0, 0),
            (RuleSource::Engine, 5, 1, 2),
            (RuleSource::Engine, -3, 2, 3),
            (RuleSource::Bible, 2, 0, 2),
            (RuleSource::Bible, -3, 0, -3),
        ] {
            let mut session = session(&catalog, reading);
            let id = session.npcs()[0].id;
            if let Some(npc) = session.npc_mut(id) {
                npc.pods_left = held;
            }
            session.tick_traffic(&catalog, &super::super::tests::Firing, &mut NeverFires);
            session.tick_combat(Rules::default(), &mut Ranged::of(&[(4, draw)]));
            assert_eq!(
                session.npc(id).map(|npc| npc.pods_left),
                Some(stepped),
                "{reading:?}: {held}"
            );
        }
    }
}
