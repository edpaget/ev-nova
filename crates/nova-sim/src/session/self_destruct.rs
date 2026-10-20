//! The player's self-destruct, `_HandlePlayer`'s block Z
//! (@0x6fdb4-0x702bf; the countdown itself is
//! [`self_destruct`](crate::self_destruct)).
//!
//! The view holds the chord, Command and `-` together, with
//! [`Session::hold_self_destruct`], while the player's fate takes
//! [`PlayerCommand::SelfDestruct`](crate::PlayerCommand::SelfDestruct).
//! Each pass's work before the fight steps the countdown once, after the
//! eject check (block Z follows the eject at 0x6f22c), on both passes of a
//! double-time frame as `_HandlePlayer` runs on both, so there the count
//! falls by two a frame. Whether the player is dying is read at the head
//! of that work, before the self-repair, the sabotage clock and the eject,
//! as `_IsDying` (-0x2ca) is computed at the top of `_HandlePlayer`
//! (@0x68aba-0x68ac6).
//!
//! The countdown neither moves nor cancels while the player is in the
//! escape pod (its frames return @0x68ddd before any key handler) or
//! destroyed (the dead branch, 0x6870d-0x6888c), though the view then
//! sends the chord let go; nor while landed, where the fight's pass stands
//! still. The pod's rescue resets it, as `_ResetPlayer` does (@0x1d764;
//! see the `rescue` module). It is never saved.
//!
//! Each beat plays its beep and says its line:
//!
//! - **Started**: [`SimSound::SelfDestructStarted`] (`snd ` 153) and
//!   [`SelfDestructLine::Initiated`]. A dying player's start is silent.
//! - **Count**: [`SimSound::SelfDestructCountdown`] (`snd ` 152) and
//!   [`SelfDestructLine::In`], "seconds" every time by the engine, "1
//!   second" by the other reading of [`RuleKey::SelfDestructSecond`].
//! - **Cancelled**: [`SimSound::SelfDestructCancelled`] (`snd ` 151) and
//!   [`SelfDestructLine::Cancelled`].
//! - **Destruct**: [`SimSound::SelfDestructed`] (`snd ` 151) and
//!   [`SelfDestructLine::NiceDay`]; then the player's shield is 0, its
//!   armour -1.0 and its target let go (+0x54, +0x58, +0x70), and every
//!   NPC docked with it ([`docked_with_player`]) has its armour set to
//!   -1.0, its shield left alone (@0x7027c-0x702b2). The fight that
//!   follows on the same tick finds the armour gone and starts each
//!   breakup, as it does for the sabotage kill. The jump is left as it is
//!   (+0x50 is untouched), so a jump under way collapses as any dying
//!   player's does.
//!
//! *Ordering against the original.* The engine moves the traffic before
//! this work, where `_HandlePlayer` reads the NPCs before `_HandleShip`
//! moves them. A ship docked with the player matches its velocity, so the
//! 16-pixel test reads the same: a difference inside a frame that crosses
//! no boundary the player can see.
//!
//! Left out, with no counterpart in the engine: `_keyPressedThisFrame`
//! (*0x3b5550, set @0x6fe0d, @0x701d5) and the status redraw flag
//! (*0x3b506c +3, @0x70278).

use super::{PlayerShip, Session};
use crate::combat::hull::Condition;
use crate::message::{SelfDestructLine, SimMessage};
use crate::rulebook::{RuleKey, RuleSource};
use crate::self_destruct::Beat;
use crate::sound::SimSound;
use crate::traffic::docked::docked_with_player;

impl Session {
    /// Holds the self-destruct chord, Command and `-` down together, or
    /// lets it go: what the next pass's countdown reads. The session takes
    /// it only while the player's fate takes
    /// [`PlayerCommand::SelfDestruct`](crate::PlayerCommand::SelfDestruct).
    pub fn hold_self_destruct(&mut self, held: bool) {
        self.ship.hold_self_destruct(held);
    }

    /// One step of the countdown, the player `dying_at_head` as the pass's
    /// work before the fight began (see the module docs).
    pub(super) fn self_destruct_tick(&mut self, dying_at_head: bool) {
        if self.pod.is_some() || self.ship.condition == Condition::Destroyed {
            return;
        }
        let Some(beat) = self
            .ship
            .fate
            .self_destruct
            .step(self.ship.fate.self_destruct_held, dying_at_head)
        else {
            return;
        };
        let (sound, line) = match beat {
            Beat::Started => (SimSound::SelfDestructStarted, SelfDestructLine::Initiated),
            Beat::Count { seconds } => {
                let singular = seconds == 1
                    && self.setup.rules.source_for(RuleKey::SelfDestructSecond)
                        == RuleSource::Bible;
                (
                    SimSound::SelfDestructCountdown,
                    SelfDestructLine::In { seconds, singular },
                )
            }
            Beat::Cancelled => (SimSound::SelfDestructCancelled, SelfDestructLine::Cancelled),
            Beat::Destruct => (SimSound::SelfDestructed, SelfDestructLine::NiceDay),
        };
        self.out.sounds.push(sound);
        self.say(SimMessage::SelfDestruct(line));
        if beat == Beat::Destruct {
            self.destruct_player();
        }
    }

    /// The countdown's end (@0x7025b-0x702b2): the player's shield 0, its
    /// armour -1.0 and its target let go, and every NPC docked with it
    /// at -1.0 armour (see the module docs).
    fn destruct_player(&mut self) {
        let reserves = &mut self.pilot.reserves;
        reserves.shield.now = 0.0;
        reserves.armor.now = -1.0;
        self.target = None;
        let player = self.ship.player;
        for npc in self.traffic.npcs_mut() {
            if docked_with_player(npc, &player) {
                npc.reserves.armor.now = -1.0;
            }
        }
    }
}

impl PlayerShip {
    /// Holds the self-destruct chord, Command and `-` down together, or
    /// lets it go: what the next pass's countdown reads. The session takes
    /// it only while the player's fate takes
    /// [`PlayerCommand::SelfDestruct`](crate::PlayerCommand::SelfDestruct).
    fn hold_self_destruct(&mut self, held: bool) {
        self.fate.self_destruct_held = held;
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::chance::NeverFires;
    use crate::combat::{Rules, ShipRef};
    use crate::flight::Controls;
    use crate::geometry::Vec2;
    use crate::rulebook::Rulebook;
    use crate::self_destruct::SelfDestruct;
    use crate::session::tests::{armed, facing_an_npc};
    use crate::testkit::{FakePilotCatalog, land_now};
    use crate::traffic::npc::NpcId;
    use crate::traffic::plunder_dock::{DockSub, Docking};

    /// A fight tick, the chord held or not: its sounds and lines.
    fn tick(session: &mut Session, held: bool) -> (Vec<SimSound>, Vec<SimMessage>) {
        session.take_sounds();
        session.take_messages();
        session.hold_self_destruct(held);
        session.tick_combat(Rules::default(), &mut NeverFires);
        (session.take_sounds(), session.take_messages())
    }

    /// Ticks `ticks` times with the chord held, giving the last tick's
    /// sounds and lines.
    fn hold(session: &mut Session, ticks: u32) -> (Vec<SimSound>, Vec<SimMessage>) {
        let mut last = (Vec::new(), Vec::new());
        for _ in 0..ticks {
            last = tick(session, true);
        }
        last
    }

    fn flying(catalog: &FakePilotCatalog) -> Session {
        facing_an_npc(catalog, 180)
    }

    fn line(line: SelfDestructLine) -> SimMessage {
        SimMessage::SelfDestruct(line)
    }

    fn count(seconds: u32, singular: bool) -> SimMessage {
        line(SelfDestructLine::In { seconds, singular })
    }

    /// Whether `sounds` hold `sound`.
    fn has(sounds: &[SimSound], sound: SimSound) -> bool {
        sounds.contains(&sound)
    }

    #[test]
    fn the_press_tick_beeps_153_and_says_the_sequence_is_initiated() {
        let catalog = armed();
        let mut session = flying(&catalog);
        let (sounds, messages) = tick(&mut session, true);
        assert!(has(&sounds, SimSound::SelfDestructStarted), "{sounds:?}");
        assert_eq!(messages, [line(SelfDestructLine::Initiated)]);
        assert_eq!(session.ship.fate.self_destruct.count(), Some(149));
    }

    #[test]
    fn ticks_30_60_90_120_beep_152_and_count_4_3_2_1_seconds() {
        for (rules, singular) in [
            (Rulebook::default(), false),
            (
                Rulebook::default().with_override(RuleKey::SelfDestructSecond, RuleSource::Bible),
                true,
            ),
        ] {
            let catalog = armed();
            let mut session = flying(&catalog).with_rules(rules);
            let mut said = Vec::new();
            for at in 1..=148 {
                let (sounds, messages) = tick(&mut session, true);
                if has(&sounds, SimSound::SelfDestructCountdown) {
                    said.push((at, messages));
                } else if at > 1 {
                    assert_eq!(messages, [], "tick {at}");
                }
            }
            assert_eq!(
                said,
                [
                    (30, vec![count(4, false)]),
                    (60, vec![count(3, false)]),
                    (90, vec![count(2, false)]),
                    (120, vec![count(1, singular)]),
                ],
                "singular {singular}"
            );
        }
    }

    #[test]
    fn tick_149_beeps_151_says_have_a_nice_day_and_destroys_the_player() {
        let catalog = armed();
        let mut session = flying(&catalog);
        session.target = Some(NpcId(1));
        hold(&mut session, 148);
        assert_eq!(session.player_condition(), Condition::Intact);
        let (sounds, messages) = tick(&mut session, true);
        assert!(has(&sounds, SimSound::SelfDestructed), "{sounds:?}");
        assert_eq!(messages, [line(SelfDestructLine::NiceDay)]);
        assert_eq!(session.pilot.reserves.shield.now, 0.0);
        assert!(session.pilot.reserves.armor.now <= -1.0);
        assert_eq!(session.target, None);
        assert!(matches!(
            session.player_condition(),
            Condition::Dying { .. } | Condition::Destroyed
        ));
        assert_eq!(session.ship.fate.self_destruct.count(), None);
    }

    #[test]
    fn the_destruct_takes_a_docked_pirate_and_leaves_a_ship_beside_it() {
        let catalog = armed();
        let mut session = flying(&catalog);
        let at = session.player().position;
        let velocity = session.player().velocity;
        let docked = session.traffic.npcs()[0].id;
        {
            let pirate = &mut session.traffic.npcs_mut()[0];
            pirate.state.position = at + Vec2::new(10.0, -10.0);
            pirate.state.velocity = velocity;
            pirate.docking = Some(Docking {
                ship: ShipRef::Player,
                sub: DockSub::Dock,
            });
        }
        let mut beside = session.traffic.npcs()[0].clone();
        beside.docking = None;
        let beside = session.traffic.add_npc(beside);
        let shield = |session: &Session, id| {
            let npc = session.npcs().iter().find(|npc| npc.id == id);
            npc.expect("there").reserves.shield.now
        };
        let beside_shield = shield(&session, beside);
        hold(&mut session, 148);
        session.take_combat_events();
        hold(&mut session, 1);
        let docked_npc = session.npcs().iter().find(|npc| npc.id == docked);
        let docked_npc = docked_npc.expect("breaking up");
        assert!(docked_npc.reserves.armor.now <= -1.0);
        assert!(matches!(docked_npc.condition, Condition::Dying { .. }));
        let beside_npc = session.npcs().iter().find(|npc| npc.id == beside);
        let beside_npc = beside_npc.expect("there");
        assert_eq!(beside_npc.condition, Condition::Intact);
        assert_eq!(beside_npc.reserves.shield.now, beside_shield);
        assert!(beside_npc.reserves.armor.now > 0.0);
    }

    #[test]
    fn a_release_beeps_151_and_says_the_sequence_is_cancelled() {
        let catalog = armed();
        let mut session = flying(&catalog);
        hold(&mut session, 40);
        let (sounds, messages) = tick(&mut session, false);
        assert!(has(&sounds, SimSound::SelfDestructCancelled), "{sounds:?}");
        assert_eq!(messages, [line(SelfDestructLine::Cancelled)]);
        assert_eq!(session.ship.fate.self_destruct.count(), None);
        assert_eq!(tick(&mut session, false), (vec![], vec![]));
    }

    #[test]
    fn a_dying_players_hold_is_silent() {
        let catalog = armed();
        let mut session = flying(&catalog);
        session.ship.condition = Condition::Dying {
            ticks_left: 1000,
            starting: false,
        };
        let (sounds, messages) = tick(&mut session, true);
        assert!(!has(&sounds, SimSound::SelfDestructStarted), "{sounds:?}");
        assert!(!has(&sounds, SimSound::Refused), "{sounds:?}");
        assert_eq!(messages, []);
        assert_eq!(session.ship.fate.self_destruct.count(), None);
    }

    #[test]
    fn a_countdown_started_while_disabled_runs_through_the_breakup_to_its_end() {
        let catalog = armed();
        let mut session = flying(&catalog);
        session.ship.condition = Condition::Disabled;
        hold(&mut session, 10);
        assert_eq!(session.ship.fate.self_destruct.count(), Some(140));
        session.ship.condition = Condition::Dying {
            ticks_left: 1000,
            starting: false,
        };
        let (sounds, _) = hold(&mut session, 139);
        assert!(has(&sounds, SimSound::SelfDestructed), "{sounds:?}");
    }

    #[test]
    fn in_the_pod_the_count_neither_moves_nor_cancels() {
        let catalog = armed();
        let mut session = flying(&catalog);
        hold(&mut session, 10);
        session.pod = Some(300);
        for _ in 0..5 {
            assert_eq!(tick(&mut session, false), (vec![], vec![]));
        }
        assert_eq!(session.ship.fate.self_destruct.count(), Some(140));
    }

    #[test]
    fn destroyed_the_count_neither_moves_nor_cancels() {
        let catalog = armed();
        let mut session = flying(&catalog);
        hold(&mut session, 10);
        session.ship.condition = Condition::Destroyed;
        let (sounds, messages) = tick(&mut session, false);
        assert!(!has(&sounds, SimSound::SelfDestructCancelled), "{sounds:?}");
        assert_eq!(messages, []);
        assert_eq!(session.ship.fate.self_destruct.count(), Some(140));
    }

    #[test]
    fn landed_nothing_steps() {
        let catalog = armed();
        let mut session = flying(&catalog);
        land_now(&mut session).expect("lands");
        let (sounds, messages) = tick(&mut session, true);
        assert!(!has(&sounds, SimSound::SelfDestructStarted), "{sounds:?}");
        assert_eq!(messages, []);
        assert_eq!(session.ship.fate.self_destruct, SelfDestruct::default());
    }

    #[test]
    fn in_double_time_a_frame_takes_the_count_down_by_two() {
        let catalog = armed();
        let mut session = flying(&catalog);
        hold(&mut session, 1);
        assert_eq!(session.ship.fate.self_destruct.count(), Some(149));
        session.tick_again(
            Controls::default(),
            &crate::ai::NovaAi::default(),
            Rules::default(),
            &mut NeverFires,
        );
        assert_eq!(session.ship.fate.self_destruct.count(), Some(148));
        hold(&mut session, 1);
        session.tick_again(
            Controls::default(),
            &crate::ai::NovaAi::default(),
            Rules::default(),
            &mut NeverFires,
        );
        assert_eq!(session.ship.fate.self_destruct.count(), Some(146));
    }
}
