//! What a hyperspace arrival says: the arrival line, or the system's
//! message buoy in its place (see the [`buoy`](crate::buoy) module for
//! `_DisplayMessage` and its addresses).
//!
//! Only a hyperspace arrival reads the buoy: a way in whose row of the
//! `entry` module's table raises the arrival message
//! (`Reset::ArrivalMessage`). Hypergate and wormhole exits never show one,
//! as `_DisplayMessage`'s only caller is `_HandlePlayer`'s hyperspace
//! arrival (@0x6c25b-0x6c27a). A buoy replaces the whole arrival line,
//! the abandoned-fighters suffix too: the fighters are still abandoned,
//! and the notes telling of them that the entry queued are dropped
//! (the `entry` module's `announce`). It plays no sound of its own, as `_DisplayMessage` calls no
//! `PlaySound`; [`SimSound::Arrived`](crate::SimSound::Arrived) sounds as
//! for any jump's arrival.

use super::Session;
use crate::buoy::{buoy_text, patch_id};
use crate::catalog::{PilotCatalog, StringPatch, SystemId};
use crate::message::SimMessage;
use crate::rulebook::{RuleKey, RuleSource};

impl Session {
    /// What a hyperspace arrival in `system` raises, its buoy's strings
    /// read from `catalog`: [`SimMessage::Buoy`] when its `Message` is
    /// not −1 and names a string `catalog` holds, or names none and
    /// [`RuleKey::MissingBuoyString`] reads the engine (an empty line);
    /// [`SimMessage::Arrived`] otherwise.
    pub(super) fn arrival_message<S>(&self, system: SystemId, catalog: &S) -> SimMessage
    where
        S: PilotCatalog + ?Sized,
    {
        let Some(index) = self.galaxy.map().buoy(system) else {
            return SimMessage::Arrived(system);
        };
        let patch = patch_id(index).map_or(StringPatch::Absent, |id| catalog.string_patch(id));
        match buoy_text(index, patch, &catalog.buoy_strings()) {
            Some(text) => SimMessage::Buoy(text),
            None => match self.rules.source_for(RuleKey::MissingBuoyString) {
                RuleSource::Engine => SimMessage::Buoy(String::new()),
                RuleSource::Bible => SimMessage::Arrived(system),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buoy::BUOY_STRINGS;
    use crate::chance::NeverFires;
    use crate::gate::GateArrivalRule;
    use crate::hyperspace::MultiJumpRule;
    use crate::rulebook::Rulebook;
    use crate::session::tests::{at_gate, at_wormhole, bound_for_134, chained, gated, wormholes};
    use crate::sound::SimSound;
    use crate::testkit::{FakePilotCatalog, Scripted, begin_jump_now, catalog, jump};

    /// `base` with `STR#` 1000 "one", "two", "three", and system `id`'s
    /// `Message` `message`.
    fn buoyed(mut base: FakePilotCatalog, id: i16, message: i16) -> FakePilotCatalog {
        base.strings.push((
            BUOY_STRINGS,
            ["one", "two", "three"].map(str::to_owned).to_vec(),
        ));
        let system = base
            .star_map
            .iter_mut()
            .find(|system| system.id == SystemId(id))
            .expect("on the map");
        system.message = message;
        base
    }

    /// The messages a jump from 130 to 131 raises, `Message` `message` in
    /// 131, `patches` held and `rule` reading `MissingBuoyString`.
    fn arriving(message: i16, patches: &[(i16, &str)], rule: RuleSource) -> Vec<SimMessage> {
        let mut buoys = buoyed(catalog(), 131, message);
        buoys.string_patches = patches
            .iter()
            .map(|&(id, text)| (id, StringPatch::Text(text.to_owned())))
            .collect();
        let mut session = Session::start(&buoys)
            .expect("starts")
            .with_rules(Rulebook::default().with_override(RuleKey::MissingBuoyString, rule));
        jump(&mut session, &buoys, 131).expect("arrives");
        session.take_messages()
    }

    fn buoy(text: &str) -> SimMessage {
        SimMessage::Buoy(text.to_owned())
    }

    const ARRIVED: SimMessage = SimMessage::Arrived(SystemId(131));

    /// The messages and sounds a jump from 130 to 131 raises, `Message`
    /// `message` in 131.
    fn heard(message: i16) -> (Vec<SimMessage>, Vec<SimSound>) {
        let buoys = buoyed(catalog(), 131, message);
        let mut session = Session::start(&buoys).expect("starts");
        jump(&mut session, &buoys, 131).expect("arrives");
        (session.take_messages(), session.take_sounds())
    }

    #[test]
    fn a_jump_into_a_buoyed_system_shows_its_string_in_place_of_the_arrival() {
        let (messages, sounds) = heard(2);
        assert_eq!(messages, [buoy("two")]);
        assert_eq!(sounds.last(), Some(&SimSound::Arrived));
        assert_eq!(sounds, heard(-1).1, "as a plain arrival's");
    }

    #[test]
    fn only_minus_1_means_no_buoy() {
        assert_eq!(arriving(-1, &[], RuleSource::Engine), [ARRIVED]);
        assert_eq!(arriving(0, &[], RuleSource::Engine), [buoy("")]);
        assert_eq!(arriving(-2, &[], RuleSource::Engine), [buoy("")]);
    }

    #[test]
    fn an_index_the_list_does_not_hold_is_an_empty_line_by_the_engine_and_the_arrival_by_the_other_reading()
     {
        for message in [0, 4] {
            assert_eq!(
                arriving(message, &[], RuleSource::Engine),
                [buoy("")],
                "{message}"
            );
            assert_eq!(
                arriving(message, &[], RuleSource::Bible),
                [ARRIVED],
                "{message}"
            );
        }
        assert_eq!(
            arriving(2, &[], RuleSource::Bible),
            [buoy("two")],
            "an index the list holds shows by both"
        );
    }

    #[test]
    fn a_str_999_past_the_index_wins_over_the_list() {
        for rule in [RuleSource::Engine, RuleSource::Bible] {
            assert_eq!(
                arriving(2, &[(1001, "patched")], rule),
                [buoy("patched")],
                "{rule:?}"
            );
            assert_eq!(
                arriving(4, &[(1003, "past")], rule),
                [buoy("past")],
                "{rule:?}"
            );
            assert_eq!(
                arriving(2, &[(1002, "another's")], rule),
                [buoy("two")],
                "{rule:?}"
            );
        }
    }

    #[test]
    fn a_str_that_cannot_be_read_shows_an_empty_line_by_both_readings() {
        for rule in [RuleSource::Engine, RuleSource::Bible] {
            for patch in [StringPatch::Empty, StringPatch::Unreadable] {
                let mut unread = buoyed(catalog(), 131, 4);
                unread.string_patches = vec![(1003, patch.clone())];
                let mut session = Session::start(&unread).expect("starts").with_rules(
                    Rulebook::default().with_override(RuleKey::MissingBuoyString, rule),
                );
                jump(&mut session, &unread, 131).expect("arrives");
                assert_eq!(session.take_messages(), [buoy("")], "{rule:?} {patch:?}");
            }
        }
    }

    #[test]
    fn a_hypergate_or_wormhole_exit_shows_no_buoy() {
        for rule in [GateArrivalRule::Engine, GateArrivalRule::LikeJump] {
            let gates = buoyed(gated(), 131, 2);
            let mut session = at_gate(&gates).with_gate_arrival(rule);
            session
                .enter_hypergate(Some(SystemId(131)), &gates, &mut NeverFires)
                .expect("enters");
            assert_eq!(
                session.take_messages(),
                [SimMessage::ExitedHypergate(SystemId(131))],
                "{rule:?}"
            );
            let holes = buoyed(wormholes(), 131, 2);
            let mut session = at_wormhole(&holes).with_gate_arrival(rule);
            session
                .enter_wormhole(&holes, &mut Scripted::rolling(&[0]))
                .expect("enters");
            assert_eq!(
                session.take_messages(),
                [SimMessage::PassedWormhole(SystemId(131))],
                "{rule:?}"
            );
        }
    }

    #[test]
    fn a_multi_jump_shows_only_the_last_systems_buoy() {
        let passing = buoyed(chained(catalog(), 10), 131, 2);
        let mut session = bound_for_134(&passing, MultiJumpRule::default());
        begin_jump_now(&mut session).expect("jumps");
        session.arrive(&passing, &mut NeverFires);
        assert_eq!(
            session.take_messages(),
            [SimMessage::Arrived(SystemId(134))],
            "131 passed through"
        );
        let ending = buoyed(chained(catalog(), 10), 134, 3);
        let mut session = bound_for_134(&ending, MultiJumpRule::default());
        begin_jump_now(&mut session).expect("jumps");
        session.arrive(&ending, &mut NeverFires);
        assert_eq!(session.take_messages(), [buoy("three")]);
    }
}
