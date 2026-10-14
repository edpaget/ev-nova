//! The session's stand-in for the original's
//! `_ResetPlayerPrecalcedValues` (@0xc357-0xca32):
//! [`Session::reset_precalced`].
//!
//! **What it does.** The parts the engine models, in the original's
//! order: it re-arms `_ScanPlayer`'s contraband latches (@0xc3ef-0xc5a3,
//! @0xc7b0; see [`scan::Latches`](crate::scan::Latches)), trims the
//! commodities to the fleet's holds (@0xc7d8-0xc887), and takes the
//! active ranks the blessing table is built from (@0xc8be-0xc981; see
//! the `ranks` module). The trim is not idempotent: it cuts only the
//! commodities, by the holds over everything held, so with any `jünk` or
//! mission cargo aboard the cargo is still past the holds after it, and
//! each reset cuts again, as the original's does.
//!
//! **Where it runs**, as the original calls it:
//!
//! | Original | Engine |
//! |---|---|
//! | `_EnterGameFromMainScreen` @0x19ffb | [`Session::fly`], and [`Session::begin`] after the `chär`'s `OnStart` |
//! | `_HandlePlayerDockRequest` @0x6709b | a take-off ([`Session::take_off`]) and a gate exit, before the setup |
//! | `_HandlePlayer` @0x6c51f | a jump's arrival ([`Session::arrive`]), after the days, the enter-system hook and the setup |
//! | `_JettisonCargo` @0x3e3b5, the jettison key's @0x6fdaf | [`Session::jettison_cargo`]; every good is gone by then, so its trim never cuts |
//! | `_DoPlunderDialog` @0x94a0b | the plunder dialog closing |
//! | `_DoMissionCargoPickup` @0x98316 | a pickup that fits, from a boarding or a landing, before the cargo is aboard |
//! | `_EvalCurrentMissionBitSetString` @0x99a85, `_EvalMissionBitSetString` @0x99dea | the end of every set expression whose text is not empty, one of only spaces or one that did not parse included (`Session::run_ops`): a hook, a mission's script, a crön, an `OnExplore`, a story subscription's |
//!
//! *Ordering against the original.* The original runs a take-off's, a
//! gate exit's and an arrival's story work (the days, the explorations,
//! the take-off's subscriptions) before its reset; the engine runs it
//! when the script next settles, after. Every script that work runs
//! resets as it ends, in both, and no frame is drawn in between, so the
//! settling runs no reset of its own: one there would trim again.
//!
//! **The trim's exception.** By the other reading of
//! [`RuleKey::ShipChangeCargo`], a set expression's reset does not trim,
//! whatever the expression holds, so a `C`, `E` or `H` keeps every ton
//! past the new hold until a reset outside a set expression.

use super::Session;
use crate::market;
use crate::rulebook::{RuleKey, RuleSource};
use crate::scan::Latches;

/// What runs a reset, which the trim's exceptions read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ResetCause {
    /// The end of a set expression that is not empty (@0x99a85,
    /// @0x99dea).
    SetExpr,
    /// Any other site.
    Other,
}

impl Session {
    /// Runs every part of `_ResetPlayerPrecalcedValues` the engine models,
    /// in the original's order (see the module docs): re-arms
    /// `_ScanPlayer`'s contraband latches (@0xc3ef-0xc5a3, @0xc7b0),
    /// trims the commodities to the fleet's holds (@0xc7d8-0xc887), and
    /// takes the active ranks the blessing table is built from
    /// (@0xc8be-0xc981).
    ///
    /// The trim cuts each commodity to trunc(held × holds / total), in
    /// single floats, when the fleet's holds ([`Session::capacity`]) are
    /// below everything held, the mission cargo aboard counted
    /// ([`Pilot::total_cargo`](crate::pilot::Pilot::total_cargo), as
    /// `_TotalCargo` counts it); the `jünk` and the mission cargo are
    /// never cut ([`market::trim_commodities`]). The original then floors
    /// a negative commodity or `jünk` at none (@0xc837-0xc893); the
    /// engine's tons are unsigned, so that holds by construction. A trim
    /// that changes the cargo makes a save due, as the original's sets
    /// its dirty flag (@0xc82e).
    pub(super) fn reset_precalced(&mut self) {
        self.reset_precalced_for(ResetCause::Other);
    }

    /// [`Session::reset_precalced`] for `cause`: by the other reading of
    /// [`RuleKey::ShipChangeCargo`], the reset at a set expression's end
    /// leaves the cargo past the holds untrimmed (see the module docs).
    pub(super) fn reset_precalced_for(&mut self, cause: ResetCause) {
        self.scan_latches = Latches::ARMED;
        if self.trims(cause) {
            self.trim_to_holds();
        }
        self.blessing_ranks.clone_from(&self.pilot.ranks);
    }

    /// Whether the reset for `cause` trims the commodities.
    fn trims(&self, cause: ResetCause) -> bool {
        match cause {
            ResetCause::SetExpr => {
                self.rules.source_for(RuleKey::ShipChangeCargo) == RuleSource::Engine
            }
            ResetCause::Other => true,
        }
    }

    /// The reset's trim of the commodities (see
    /// [`Session::reset_precalced`]).
    fn trim_to_holds(&mut self) {
        let holds = self.capacity();
        let total = self.pilot.total_cargo();
        let before = self.pilot.cargo.clone();
        market::trim_commodities(&mut self.pilot.cargo, holds, total);
        if self.pilot.cargo != before {
            self.save_due = true;
        }
    }
}

#[cfg(test)]
pub(super) mod tests {
    use std::collections::BTreeMap;

    use crate::catalog::{JunkId, RankId};
    use crate::chance::NeverFires;
    use crate::control::{Script, SetExpr};
    use crate::market::Good;
    use crate::scan::Latches;
    use crate::session::Session;
    use crate::session::ranks::tests::{blessed, blessing_rank};
    use crate::testkit::{FakePilotCatalog, Scripted, carrying, catalog, jump, land_now};

    /// A `jünk` the tests hold.
    pub(in crate::session) const JUNK: Good = Good::Junk(JunkId(150));

    /// Clears both of `_ScanPlayer`'s latches, as a conviction of each
    /// kind does ([`Latches::clear`]).
    pub(in crate::session) fn convicted(session: &mut Session) {
        session.scan_latches = Latches {
            junk: false,
            outfits: false,
        };
    }

    /// Loads `session` past its fleet's holds: commodities 0 and 2 each
    /// as many tons as the holds, 4 tons of [`JUNK`], and a mission of 3
    /// tons aboard in slot 0. Gives the cargo a reset leaves: each
    /// commodity cut by the holds over everything held, the `jünk` kept.
    pub(in crate::session) fn overloaded(session: &mut Session) -> BTreeMap<Good, u32> {
        let holds = session.capacity();
        session.pilot.cargo = [
            (Good::Commodity(0), holds),
            (Good::Commodity(2), holds),
            (JUNK, 4),
        ]
        .into_iter()
        .collect();
        session.pilot.missions.put(0, carrying(140, 3));
        let total = 2 * holds + 4 + 3;
        let ratio = holds as f32 / total as f32;
        let cut = (holds as f32 * ratio) as u32;
        [
            (Good::Commodity(0), cut),
            (Good::Commodity(2), cut),
            (JUNK, 4),
        ]
        .into_iter()
        .filter(|&(_, tons)| tons > 0)
        .collect()
    }

    fn cargo(session: &Session) -> BTreeMap<Good, u32> {
        session.pilot().cargo().collect()
    }

    fn blessing_catalog() -> FakePilotCatalog {
        FakePilotCatalog {
            ranks: vec![blessing_rank()],
            ..catalog()
        }
    }

    #[test]
    fn the_reset_cuts_each_commodity_by_the_holds_over_everything_held() {
        let mut session = Session::start(&catalog()).expect("starts");
        assert_eq!(session.capacity(), 20);
        session.pilot.cargo = [
            (Good::Commodity(0), 10),
            (Good::Commodity(2), 13),
            (JUNK, 4),
        ]
        .into_iter()
        .collect();
        session.pilot.missions.put(0, carrying(140, 3));
        let _ = session.take_save_due();
        session.reset_precalced();
        assert_eq!(
            cargo(&session),
            [(Good::Commodity(0), 6), (Good::Commodity(2), 8), (JUNK, 4)].into(),
            "20/30 of each commodity, truncated; the jünk kept (@0xc7d8-0xc82c)"
        );
        assert_eq!(
            session.pilot().missions().get(0).map(|m| m.cargo_tons),
            Some(3)
        );
        assert!(session.take_save_due(), "@0xc82e");
    }

    #[test]
    fn a_reset_within_the_holds_changes_no_cargo_and_makes_no_save_due() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.pilot.cargo = [(Good::Commodity(0), 10), (JUNK, 7)].into_iter().collect();
        session.pilot.missions.put(0, carrying(140, 3));
        let before = cargo(&session);
        let _ = session.take_save_due();
        session.reset_precalced();
        assert_eq!(cargo(&session), before, "20 held in 20 tons");
        assert!(!session.take_save_due());
    }

    #[test]
    fn a_commodity_cut_to_none_is_gone() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.pilot.cargo = [(Good::Commodity(1), 1), (JUNK, 40)].into_iter().collect();
        session.reset_precalced();
        assert_eq!(cargo(&session), [(JUNK, 40)].into());
    }

    #[test]
    fn the_reset_rearms_the_latches_and_takes_the_blessing() {
        let catalog = blessing_catalog();
        let mut session = Session::start(&catalog).expect("starts");
        convicted(&mut session);
        session.pilot.ranks.insert(RankId(200));
        assert!(!blessed(&session));
        session.reset_precalced();
        assert_eq!(session.scan_latches, Latches::ARMED);
        assert!(blessed(&session));
    }

    // The sites the session already reset at, now trimming.

    #[test]
    fn a_take_off_trims_the_cargo_past_the_holds() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        land_now(&mut session).expect("lands");
        let trimmed = overloaded(&mut session);
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        assert_eq!(cargo(&session), trimmed, "@0x6709b");
    }

    #[test]
    fn a_jump_arrival_trims_the_cargo_past_the_holds() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        let trimmed = overloaded(&mut session);
        jump(&mut session, &catalog, 131).expect("arrives");
        assert_eq!(cargo(&session), trimmed, "@0x6c51f");
    }

    #[test]
    fn a_gate_exit_trims_the_cargo_past_the_holds() {
        use crate::gate::GateKind;
        use crate::session::gate_follow::tests::{exit_gate, following, gated};
        for kind in [GateKind::Hypergate, GateKind::Wormhole] {
            let catalog = gated(following(0, 0), 0);
            let mut session = Session::start(&catalog).expect("starts");
            let trimmed = overloaded(&mut session);
            exit_gate(&mut session, &catalog, kind, &mut NeverFires);
            assert_eq!(cargo(&session), trimmed, "{kind:?}: @0x6709b");
        }
    }

    #[test]
    fn flying_a_pilot_and_beginning_a_game_trim_the_cargo_past_the_holds() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        let trimmed = overloaded(&mut session);
        let pilot = session.pilot().clone();
        let flown = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(cargo(&flown), trimmed, "@0x19ffb");
        session.begin(&catalog, &mut Scripted::default());
        assert_eq!(cargo(&session), trimmed, "after OnStart, @0x19ffb");
    }

    #[test]
    fn closing_the_plunder_dialog_trims_the_cargo_past_the_holds() {
        use crate::board::Take;
        use crate::session::tests::{aboard, boardable, take};
        let catalog = boardable();
        let mut session = aboard(&catalog);
        let trimmed = overloaded(&mut session);
        convicted(&mut session);
        let _ = take(&mut session, Take::Abort, &[]);
        assert_eq!(cargo(&session), trimmed, "@0x94a0b");
        assert_eq!(session.scan_latches, Latches::ARMED);
    }

    // The end of a set expression (@0x99a85, @0x99dea).

    #[test]
    fn a_set_expression_resets_as_it_ends_and_not_between_its_operators() {
        let catalog = blessing_catalog();
        let mut session = Session::start(&catalog).expect("starts");
        let trimmed = overloaded(&mut session);
        let untouched = cargo(&session);
        convicted(&mut session);
        let _ = session.take_save_due();
        let mut seen = Vec::new();
        session.run_ops(
            &SetExpr::parse("b1 K200").expect("parses"),
            &mut Scripted::default(),
            &mut |session, _| {
                seen.push((
                    cargo(session) == untouched,
                    session.scan_latches == Latches::ARMED,
                    blessed(session),
                ));
            },
        );
        assert_eq!(
            seen,
            [(true, false, false)],
            "after the K200, inside the expression, nothing is reset"
        );
        assert_eq!(cargo(&session), trimmed);
        assert_eq!(session.scan_latches, Latches::ARMED);
        assert!(blessed(&session), "the K200 taken as the expression ends");
        assert!(session.take_save_due());
    }

    #[test]
    fn only_a_script_of_no_text_at_all_skips_the_reset() {
        for (text, resets) in [
            ("", false),
            (" ", true),
            ("\t", true),
            ("b1", true),
            ("S300&", true),
        ] {
            let mut session = Session::start(&catalog()).expect("starts");
            let trimmed = overloaded(&mut session);
            let untouched = cargo(&session);
            convicted(&mut session);
            session.run_script(&Script::parse(text), &mut Scripted::default());
            let reset = session.scan_latches == Latches::ARMED;
            assert_eq!(reset, resets, "{text:?}");
            let want = if resets { &trimmed } else { &untouched };
            assert_eq!(&cargo(&session), want, "{text:?}");
        }
        let mut session = Session::start(&catalog()).expect("starts");
        convicted(&mut session);
        session.run_script(&Script::default(), &mut Scripted::default());
        assert_ne!(session.scan_latches, Latches::ARMED, "a blank hook");
    }

    #[test]
    fn a_mission_script_and_a_story_script_reset_as_they_end() {
        use crate::catalog::StellarId;
        use crate::story::{Consumer, EventFilter, StoryEvent, Subscription, SubscriptionId};
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        let trimmed = overloaded(&mut session);
        session.run_mission_script(0, &Script::parse("b2"), &mut Scripted::default());
        assert_eq!(cargo(&session), trimmed, "a mission's script");
        let mut session = Session::start(&catalog).expect("starts");
        let subscription = Subscription::new(
            SubscriptionId {
                consumer: Consumer::Fixture,
                record: 1,
                hook: 0,
            },
            EventFilter::Landed(StellarId(128)),
            Script::parse("b42"),
        );
        session.register_story(subscription).expect("registers");
        let trimmed = overloaded(&mut session);
        session.raise(StoryEvent::Landed {
            stellar: StellarId(128),
        });
        let untouched = cargo(&session);
        assert_ne!(untouched, trimmed);
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert_eq!(cargo(&session), trimmed, "a story subscription's script");
    }

    #[test]
    fn the_overloaded_helper_gives_what_a_reset_leaves() {
        let mut session = Session::start(&catalog()).expect("starts");
        let trimmed = overloaded(&mut session);
        assert_eq!(
            trimmed,
            [(Good::Commodity(0), 8), (Good::Commodity(2), 8), (JUNK, 4)].into(),
            "20 x 20/47"
        );
        session.reset_precalced();
        assert_eq!(cargo(&session), trimmed);
    }
}
