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
//! | `_DoPlunderDialog` @0x94a0b | the plunder dialog closing, an assignment's once the ship is assigned |
//! | `_DoShipCapture`'s `_ShipStatsToSystemInfo` @0x419fd and `_SystemInfoToShipStats` @0x41a3e | Use As My Ship ([`Session::assign`]), after the swap of the commodities [`RuleKey::CaptureCargo`] reads, the old ship in the fleet |
//! | `_DoMissionCargoPickup` @0x98316 | a pickup that fits, from a boarding or a landing, before the cargo is aboard |
//! | `_MissionLandCargoCheck` @0x9eff9 | a drop-off that shows its `DumpCargoText`, the only one the original resets after (@0x9ef82) |
//! | `_PlayerLandOnStellar`'s `_ShipStatsToSystemInfo` @0x63211 | a landing, before the port takes its prices |
//! | `_DoOutfitDialog`'s `_SystemInfoToShipStats` @0x5c305, @0x5cffb | an outfit bought or sold ([`Session::outfit`]), after the refit, each pass |
//! | `_DoShipyardDialog`'s `_SystemInfoToShipStats` @0x5ee99, `_DestroyPartialFleetCargo` @0x5ef9f | a ship bought ([`Session::buy_ship`]), once its cargo is settled; both trims are the purchase rule's ([`RuleKey::PurchaseCargo`]), so this reset trims nothing more |
//! | `_DestroyPartialFleetCargo`'s tail @0xcff6, from `_DamageShip` @0x3b013 and `_HandleShip` @0x350e6 | a trader escort lost, or a trader mission escort breaking up, once out of the fleet (`Session::lose_cargo_share`) |
//! | the same, from `_DoEscortPayment` @0x40240 | a trader defecting on pay day, still in the fleet |
//! | the same, from `_DoCommDialog` @0x96864 (and `_DoEscortDialog` @0x9549d, @0x9561d) | an escort released by hail, still in the fleet, of any ship type as [`RuleKey::ReleaseCargoShare`] reads |
//! | `_EvalCurrentMissionBitSetString` @0x99a85, `_EvalMissionBitSetString` @0x99dea | the end of every set expression whose text is not empty, one of only spaces or one that did not parse included (`Session::run_ops`): a hook, a mission's script, a crön, an `OnExplore`, a story subscription's |
//!
//! *Ordering against the original.* The original runs a take-off's, a
//! gate exit's and an arrival's story work (the days, the explorations,
//! the take-off's subscriptions) before its reset; the engine runs it
//! when the script next settles, after. Every script that work runs
//! resets as it ends, in both, and no frame is drawn in between, so the
//! settling runs no reset of its own: one there would trim again.
//!
//! The trim measures the fleet's holds as `_TotalFleetHolds` does
//! (@0xc24d), from the outfits owned at that moment, so an `OnSell`'s
//! reset already sees the outfit sold gone.
//!
//! **The trim's exceptions.** By the other reading of
//! [`RuleKey::ShipChangeCargo`], a set expression's reset does not trim,
//! whatever the expression holds, so a `C`, `E` or `H` keeps every ton
//! past the new hold until a reset outside a set expression. By the other
//! reading of [`RuleKey::OutfitSaleCargo`], no reset through an outfit
//! sale trims, its `OnSell`'s included.
//!
//! **Left out.** The sites and parts of the reset left out are listed,
//! each with its address and reason, in
//! rdm:phase/cargo-and-scanning/phase-6-trim-wherever-reset-runs; those
//! the engine can model now (the player info, outfitter, hire and
//! plunder dialogs' own resets, the buy loop's single reset, the `jünk`
//! flag latches, and Use As My Ship's hook resets) wait for
//! rdm:phase/cargo-and-scanning/phase-8-reset-at-the-dialogs-and-the-junk-latches.

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
    /// A ship purchase, whose trims are the purchase rule's
    /// ([`shipyard::keep_cargo`](crate::shipyard::keep_cargo)): its reset
    /// trims nothing more.
    Purchase,
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
        self.junk_latches = market::JunkLatches::of(&self.goods, &self.pilot.cargo);
        if self.trims(cause) {
            self.trim_to_holds();
        }
        self.blessing_ranks.clone_from(&self.pilot.ranks);
    }

    /// Whether the reset for `cause` trims the commodities: never through
    /// an outfit sale that keeps the cargo ([`Session::sell_keeps_cargo`]).
    fn trims(&self, cause: ResetCause) -> bool {
        if self.sale_keeps_cargo {
            return false;
        }
        match cause {
            ResetCause::SetExpr => {
                self.rules.source_for(RuleKey::ShipChangeCargo) == RuleSource::Engine
            }
            ResetCause::Purchase => false,
            ResetCause::Other => true,
        }
    }

    /// Whether the outfit sale about to run keeps the cargo past the holds
    /// through its resets, `OnSell`'s included: by the other reading of
    /// [`RuleKey::OutfitSaleCargo`].
    pub(super) fn sell_keeps_cargo(&self) -> bool {
        self.rules.source_for(RuleKey::OutfitSaleCargo) == RuleSource::Bible
    }

    /// The reset's trim of the commodities (see
    /// [`Session::reset_precalced`]), against the fleet's holds as
    /// `_TotalFleetHolds` works them out (@0xc24d): from the outfits owned
    /// now, so an `OnSell`'s reset sees the outfit sold gone before the
    /// sale's refit.
    fn trim_to_holds(&mut self) {
        let holds = market::fleet_holds(self.current_stats().capacity, self.escort_holds());
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
    use crate::market::{Good, JunkLatches};
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

    /// [`catalog`] with `jünk` 150 ([`JUNK`]) tribbles, 151 perishable
    /// and 152 both.
    fn flagged_junk() -> FakePilotCatalog {
        use crate::catalog::JunkRecord;
        use crate::market::{PERISHABLE, TRIBBLES};
        let junk = |id, flags| JunkRecord {
            id: JunkId(id),
            name: format!("Junk {id}"),
            abbrev: String::new(),
            base_price: 10,
            sold_at: Vec::new(),
            bought_at: Vec::new(),
            buy_on: crate::control::Test::default(),
            sell_on: crate::control::Test::default(),
            flags,
            scan_mask: 0,
            lc_name: String::new(),
        };
        FakePilotCatalog {
            junk: vec![
                junk(150, TRIBBLES),
                junk(151, PERISHABLE),
                junk(152, TRIBBLES | PERISHABLE),
            ],
            ..catalog()
        }
    }

    fn latches(tribbles: bool, perishable: bool) -> JunkLatches {
        JunkLatches {
            tribbles,
            perishable,
        }
    }

    #[test]
    fn every_reset_sets_the_junk_latches_from_the_junk_held() {
        use super::ResetCause;
        let rotting = Good::Junk(JunkId(151));
        let both = Good::Junk(JunkId(152));
        for cause in [ResetCause::Other, ResetCause::SetExpr, ResetCause::Purchase] {
            let mut session = Session::start(&flagged_junk()).expect("starts");
            assert_eq!(
                session.junk_latches,
                latches(false, false),
                "clear at the start"
            );
            session.pilot.cargo = [(JUNK, 1), (rotting, 1), (both, 1)].into();
            session.reset_precalced_for(cause);
            assert_eq!(session.junk_latches, latches(true, true), "{cause:?}");
            session.pilot.cargo = [(both, 3)].into();
            session.reset_precalced_for(cause);
            assert_eq!(
                session.junk_latches,
                latches(true, false),
                "{cause:?}: a good of both flags alone sets only the tribbles latch"
            );
            session.pilot.cargo = [(rotting, 2), (Good::Commodity(0), 4)].into();
            session.reset_precalced_for(cause);
            assert_eq!(session.junk_latches, latches(false, true), "{cause:?}");
        }
    }

    #[test]
    fn jettisoning_resets_once_the_cargo_is_gone() {
        let mut session = Session::start(&flagged_junk()).expect("starts");
        session.pilot.cargo = [(JUNK, 2), (Good::Junk(JunkId(151)), 3)].into();
        session.reset_precalced();
        assert_eq!(session.junk_latches, latches(true, true));
        convicted(&mut session);
        session.jettison_cargo(true, &mut NeverFires);
        assert_eq!(session.scan_latches, Latches::ARMED, "@0x3e3b5");
        assert_eq!(
            session.junk_latches,
            latches(false, false),
            "the reset ran after every good was gone"
        );
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

    // The spaceport.

    #[test]
    fn a_landing_resets() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        let trimmed = overloaded(&mut session);
        convicted(&mut session);
        land_now(&mut session).expect("lands");
        assert_eq!(cargo(&session), trimmed, "@0x63211");
        assert_eq!(session.scan_latches, Latches::ARMED);
    }

    /// The outfitter's cargo pod, 10 tons.
    const POD: crate::catalog::OutfitId = crate::catalog::OutfitId(301);

    #[test]
    fn the_outfitter_resets_as_it_opens_and_as_it_closes() {
        use crate::session::tests::{outfitted, outfitting};
        let mut session = outfitted(&outfitting());
        let trimmed = overloaded(&mut session);
        convicted(&mut session);
        session.open_outfitter();
        assert_eq!(session.scan_latches, Latches::ARMED, "@0x5bb57");
        assert_eq!(cargo(&session), trimmed);
        let trimmed = overloaded(&mut session);
        convicted(&mut session);
        session.close_outfitter();
        assert_eq!(session.scan_latches, Latches::ARMED, "@0x5db10");
        assert_eq!(cargo(&session), trimmed);
    }

    /// The outfitter's close forgets the venue offers were last entered
    /// at (`_ResetMissionOfferVenue` @0x5dbd1), so entering the outfitter
    /// again forgets the offers whose accept was refused.
    #[test]
    fn the_outfitter_closing_forgets_the_offer_venue() {
        use crate::catalog::MissionId;
        use crate::mission::screens::OUTFITTER;
        use crate::session::tests::{outfitted, outfitting};
        let mut session = outfitted(&outfitting());
        session.enter_offer_venue(OUTFITTER);
        session.suppress_offer(MissionId(200));
        session.enter_offer_venue(OUTFITTER);
        assert!(
            session.suppressed.contains(&MissionId(200)),
            "the same venue"
        );
        session.close_outfitter();
        assert_eq!(session.offer_venue, None);
        session.enter_offer_venue(OUTFITTER);
        assert!(session.suppressed.is_empty());
    }

    #[test]
    fn an_outfit_bought_resets() {
        use crate::session::tests::{buy, outfitted, outfitting};
        let mut session = outfitted(&outfitting());
        convicted(&mut session);
        session
            .outfit(buy(crate::catalog::OutfitId(300)), &mut NeverFires)
            .expect("bought");
        assert_eq!(session.scan_latches, Latches::ARMED, "@0x5c305");
    }

    #[test]
    fn an_outfit_bought_trims_the_cargo_past_the_holds_against_the_holds_it_brings() {
        use crate::session::tests::{buy, outfitted, outfitting};
        let mut session = outfitted(&outfitting());
        assert_eq!(session.capacity(), 20);
        session.pilot.cargo = [
            (Good::Commodity(0), 30),
            (Good::Commodity(2), 30),
            (JUNK, 4),
        ]
        .into_iter()
        .collect();
        session.pilot.missions.put(0, carrying(140, 3));
        session.outfit(buy(POD), &mut NeverFires).expect("bought");
        assert_eq!(session.capacity(), 30);
        assert_eq!(
            cargo(&session),
            [
                (Good::Commodity(0), 13),
                (Good::Commodity(2), 13),
                (JUNK, 4)
            ]
            .into(),
            "30 x 30/67 after the pod is aboard, not 30 x 20/67 (@0x5c305)"
        );
    }

    /// A counted buy of 3 pods, 20 tons to 50, with 30 + 30 commodities,
    /// 4 tons of `jünk` and 3 of mission cargo aboard: the buy loop
    /// resets once, after it (@0x5c305), so 30 x 50/67 = 22 of each. A
    /// reset after each pass would give 30 x 30/67 = 13, then nothing
    /// more to cut.
    #[test]
    fn a_counted_buy_resets_once_after_its_loop() {
        use crate::session::tests::{buy, outfitted, outfitting};
        let mut session = outfitted(&outfitting());
        session.pilot.cargo = [
            (Good::Commodity(0), 30),
            (Good::Commodity(2), 30),
            (JUNK, 4),
        ]
        .into();
        session.pilot.missions.put(0, carrying(140, 3));
        let run = session.outfit_order(buy(POD), 3, &mut NeverFires);
        assert_eq!((run.run.done, run.run.stopped), (3, None));
        assert_eq!(session.capacity(), 50);
        assert_eq!(
            cargo(&session),
            [
                (Good::Commodity(0), 22),
                (Good::Commodity(2), 22),
                (JUNK, 4)
            ]
            .into()
        );
    }

    #[test]
    fn a_cancelled_buy_whose_button_passes_still_resets() {
        use crate::session::tests::{buy, outfitted, outfitting};
        let mut session = outfitted(&outfitting());
        let trimmed = overloaded(&mut session);
        convicted(&mut session);
        let run = session.outfit_order(buy(POD), 0, &mut NeverFires);
        assert_eq!(run.run.done, 0);
        assert_eq!(session.scan_latches, Latches::ARMED, "@0x5c305");
        assert_eq!(cargo(&session), trimmed);
    }

    #[test]
    fn a_buy_refused_at_its_button_resets_nothing() {
        use crate::session::tests::{buy, outfitted, outfitting};
        for count in [0, 1, 3] {
            let mut session = outfitted(&outfitting());
            session.pilot.cash = 0;
            let _ = overloaded(&mut session);
            let untouched = cargo(&session);
            convicted(&mut session);
            let run = session.outfit_order(buy(POD), count, &mut NeverFires);
            assert_eq!(run.run.done, 0, "{count}");
            assert_ne!(session.scan_latches, Latches::ARMED, "{count}");
            assert_eq!(cargo(&session), untouched, "{count}");
        }
    }

    #[test]
    fn a_buy_loop_stopped_after_its_first_pass_resets_once() {
        use crate::session::tests::{buy, outfitted, outfitting};
        let mut session = outfitted(&outfitting());
        let price = session
            .outfitter(&mut NeverFires)
            .and_then(|outfitter| outfitter.row(POD).map(|row| row.buy_price))
            .expect("listed");
        session.pilot.cash = price;
        session.pilot.cargo = [
            (Good::Commodity(0), 30),
            (Good::Commodity(2), 30),
            (JUNK, 4),
        ]
        .into();
        session.pilot.missions.put(0, carrying(140, 3));
        convicted(&mut session);
        let run = session.outfit_order(buy(POD), 3, &mut NeverFires);
        assert_eq!(run.run.done, 1);
        assert!(run.run.stopped.is_some(), "the cash spent");
        assert_eq!(session.scan_latches, Latches::ARMED);
        assert_eq!(
            cargo(&session),
            [
                (Good::Commodity(0), 13),
                (Good::Commodity(2), 13),
                (JUNK, 4)
            ]
            .into(),
            "30 x 30/67, once"
        );
    }

    #[test]
    fn an_outfit_sale_below_the_cargo_trims_it_by_the_engine_and_keeps_it_by_the_other_reading() {
        use crate::rulebook::{RuleKey, RuleSource, Rulebook};
        use crate::session::tests::{buy, outfitted, outfitting, sell};
        let thirty: BTreeMap<Good, u32> =
            [(Good::Commodity(0), 15), (Good::Commodity(1), 15)].into();
        let twenty: BTreeMap<Good, u32> =
            [(Good::Commodity(0), 10), (Good::Commodity(1), 10)].into();
        for on_sell in ["", "b9"] {
            let mut catalog = outfitting();
            for record in &mut catalog.outfits {
                if record.id == POD {
                    record.on_sell = Script::parse(on_sell);
                }
            }
            for source in RuleSource::ALL {
                let mut session = outfitted(&catalog).with_rules(
                    Rulebook::default().with_override(RuleKey::OutfitSaleCargo, source),
                );
                session.outfit(buy(POD), &mut NeverFires).expect("bought");
                assert_eq!(session.capacity(), 30);
                session.pilot.cargo = thirty.clone();
                convicted(&mut session);
                session.outfit(sell(POD), &mut NeverFires).expect("sold");
                assert_eq!(session.capacity(), 20);
                assert_eq!(session.scan_latches, Latches::ARMED, "@0x5cffb");
                let why = format!("{source:?}, OnSell {on_sell:?}");
                if source == RuleSource::Engine {
                    assert_eq!(cargo(&session), twenty, "{why}: 20/30 of each");
                } else {
                    assert_eq!(cargo(&session), thirty, "{why}: kept past the hold");
                    session
                        .take_off(&catalog, &mut NeverFires)
                        .expect("takes off");
                    assert_eq!(cargo(&session), twenty, "{why}: the take-off trims");
                }
            }
        }
    }

    /// The `OnSell` reset measures the holds live (`_TotalFleetHolds`
    /// @0xc24d): with the pod sold, 20 tons, not the 30 the stats still
    /// hold before the sale's refit. With 15 + 15 commodities and 5 tons
    /// of `jünk`, `OnSell`'s trim gives 15 x 20/35 = 8 of each, and the
    /// sale's own 8 x 20/21 = 7; read stale, 15 x 30/35 = 12, then
    /// 12 x 20/29 = 8.
    #[test]
    fn an_on_sell_reset_trims_against_the_holds_left_once_the_outfit_is_sold() {
        use crate::rulebook::{RuleKey, RuleSource, Rulebook};
        use crate::session::tests::{buy, outfitted, outfitting, sell};
        let mut catalog = outfitting();
        for record in &mut catalog.outfits {
            if record.id == POD {
                record.on_sell = Script::parse("b9");
            }
        }
        let mut session = outfitted(&catalog).with_rules(
            Rulebook::default().with_override(RuleKey::OutfitSaleCargo, RuleSource::Engine),
        );
        session.outfit(buy(POD), &mut NeverFires).expect("bought");
        assert_eq!(session.capacity(), 30);
        session.pilot.cargo = [
            (Good::Commodity(0), 15),
            (Good::Commodity(1), 15),
            (JUNK, 5),
        ]
        .into();
        session.outfit(sell(POD), &mut NeverFires).expect("sold");
        assert_eq!(session.capacity(), 20);
        assert_eq!(
            cargo(&session),
            [(Good::Commodity(0), 7), (Good::Commodity(1), 7), (JUNK, 5)].into(),
            "8 if OnSell's trim read the 30 tons the stats held"
        );
    }

    #[test]
    fn a_ship_bought_resets_once_its_cargo_is_settled_and_trims_no_third_time() {
        use crate::session::tests::{NEW, outfitted, shipbuying};
        let catalog = shipbuying();
        let mut session = outfitted(&catalog);
        session.pilot.cargo = [(Good::Commodity(0), 20)].into();
        session.pilot.missions.put(0, carrying(140, 11));
        convicted(&mut session);
        session
            .buy_ship(NEW, "Kestrel", &mut NeverFires)
            .expect("bought");
        assert_eq!(session.capacity(), 15);
        assert_eq!(session.scan_latches, Latches::ARMED, "@0x5ee99, @0x5ef9f");
        assert_eq!(
            cargo(&session),
            [(Good::Commodity(0), 6)].into(),
            "20 x 15/31, then 9 x 15/20, the purchase's two trims; 17 tons are \
             still past the holds, but no third"
        );
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
