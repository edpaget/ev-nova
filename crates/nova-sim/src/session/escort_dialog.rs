//! The escort dialog in flight: the session's side of
//! [`escort_dialog`](crate::escort_dialog).
//!
//! **Who gets it.** Hailing a ship opens the escort dialog in place of
//! the comm dialog when the ship is one of the fleet's escorts
//! ([`Npc::escort`]), not a carried fighter and of no mission
//! (`_DoCommDialog` @0x95721-0x95747: AI type 6, led by the player, not
//! a mission ship). It rolls no conversation, takes in no advice and
//! opens no comm talk, and it beeps
//! [`SimSound::EscortDialogOpened`] (`_beepSnd[4]` @0x95228). Mission
//! escorts, carried fighters and captured mission ships keep the comm
//! dialog. Whether the escort can be upgraded is worked out as the
//! dialog opens, as `_DoEscortDialog` sets `_canUpgradeEscort` once
//! (@0x952f6-0x9533e).
//!
//! **Its buttons** ([`Session::press_escort`]): Upgrade and Sell toggle
//! the escort's mark when allowed, and otherwise do nothing; the mark is
//! saved with the next save, as the original saves nothing here. Release
//! releases the escort at once, with every reset the comm dialog's
//! Release makes ([`Session::release`]), and closes the dialog. Close
//! Channel closes it. Every close beeps [`SimSound::EscortDialogClosed`]
//! (`_beepSnd[2]` @0x955d9).
//!
//! **The stale release** (@0x95601-0x95672). The comm dialog's Release
//! sets `_releaseCommEscort` and nothing but a comm dialog opening clears
//! it (see the `hail` module), so after a comm-dialog Release every escort
//! dialog closed before the next comm dialog releases its escort too, by
//! any closing button ([`Session::release_stale`]): a cargo share by its
//! class's `Holds`, out of the fleet, its class's `InherentAI`, and the
//! leave, with none of Release's other resets. After the dialog's own
//! Release it takes a second share, as `_DestroyPartialFleetCargo`
//! (@0xcd32) reads only the class's and the fleet's holds. The
//! `_statusRedraw[2]` after it (@0x95669) is left out with the rest below.
//!
//! It is the original's reading and no `Rulebook` key offers another: a
//! key needs a reason, and none applies. The Nova Bible says nothing of
//! it, the engine could not reach it before the comm dialog listed
//! Release for a captured mission ship, no plan reads it otherwise, and it
//! breaks no stock mission, cron or storyline, so it is no bug to set
//! right either. The run carries the flag to the next flight
//! ([`CarryOver::release_comm_escort`](super::CarryOver::release_comm_escort)),
//! as the original's outlives a pilot load (no pilot load, reset, new
//! pilot or launch routine touches 0x31b250), and it is never saved.
//! Carrying it across a pilot changes only whose escort the stale
//! release takes, so none of this reasoning changes and no key is added.
//!
//! Left out, as frame-timing and redraw bookkeeping the engine has no
//! use for: `_gSpeedMult` zeroed (@0x95693), `FlushEvents`,
//! `_ShowTheCursor`, `_commDialogMode` 2, `_statusRedraw` and
//! `_RefreshSAT`. The flight pauses under the dialog and redraws every
//! frame.

use super::Session;
use super::control::Facts;
use crate::SimSound;
use crate::control::Gate;
use crate::escort_dialog::{self, EscortButton, EscortDialogView};
use crate::fleet::EscortMark;
use crate::traffic::npc::{AiType, Npc, NpcId};

/// The escort dialog open: the escort hailed, and whether it can be
/// upgraded, worked out as the dialog opened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct EscortDialog {
    /// The escort's NPC.
    pub(super) npc: NpcId,
    /// `_canUpgradeEscort` (@0x952f6-0x9533e).
    pub(super) can_upgrade: bool,
}

/// Whether hailing `npc` opens the escort dialog: it is a fleet escort,
/// not a carried fighter, of no mission (@0x95721-0x95747).
pub(super) fn gets_escort_dialog(npc: &Npc) -> bool {
    npc.escort.is_some() && npc.carrier.is_none() && npc.mission.is_none()
}

impl Session {
    /// Opens the escort dialog for escort NPC `id` (see the module docs),
    /// beeping as it opens; none, opening nothing, when it is not in the
    /// fleet or its class has no record.
    pub(super) fn open_escort_dialog(&mut self, id: NpcId) -> Option<EscortDialogView> {
        let index = self.space.fleet_index(id)?;
        let escort = self.pilot.escorts.get(index)?;
        let record = self.statics.ship_record(escort.ship)?;
        let facts = Facts {
            pilot: &self.pilot,
            ammo_outfits: &self.statics.ammo_outfits,
            armament: &self.ship.fit.armament,
        };
        let gate = Gate {
            control_bits: &*self.setup.control_bits.0,
            pilot: &facts,
        };
        let can_upgrade =
            escort_dialog::can_upgrade(record, |ship| self.statics.ship_record(ship), &gate);
        self.space.escorts.dialog = Some(EscortDialog {
            npc: id,
            can_upgrade,
        });
        self.out.sounds.push(SimSound::EscortDialogOpened);
        self.escort_dialog()
    }

    /// What the escort dialog shows, while it is open.
    #[must_use]
    pub(crate) fn escort_dialog(&self) -> Option<EscortDialogView> {
        let dialog = self.space.escorts.dialog?;
        let escort = self
            .pilot
            .escorts
            .get(self.space.fleet_index(dialog.npc)?)?;
        let record = self.statics.ship_record(escort.ship)?;
        let pay = escort
            .hired()
            .then(|| self.setup.shown_wage(&self.statics, escort));
        Some(EscortDialogView::of(
            escort,
            record,
            dialog.can_upgrade,
            pay,
        ))
    }

    /// Presses `button` in the escort dialog (see the module docs), and
    /// gives what it shows after; none once it closes, or when it is not
    /// open.
    pub(crate) fn press_escort(&mut self, button: EscortButton) -> Option<EscortDialogView> {
        let dialog = self.space.escorts.dialog?;
        let pressed = match button {
            EscortButton::Close => {
                self.close_escort_dialog();
                return None;
            }
            EscortButton::Release => {
                self.release(dialog.npc);
                self.close_escort_dialog();
                return None;
            }
            EscortButton::Upgrade => EscortMark::Upgrade,
            EscortButton::Sell => EscortMark::Sell,
        };
        let index = self.space.fleet_index(dialog.npc)?;
        let escort = self.pilot.escorts.get_mut(index)?;
        let allowed = match pressed {
            EscortMark::Upgrade => dialog.can_upgrade,
            EscortMark::Sell => escort_dialog::can_sell(escort),
        };
        escort.mark = escort_dialog::toggle(escort.mark, pressed, allowed);
        self.escort_dialog()
    }

    /// The escort dialog closes, beeping; then, while the comm dialog's
    /// release flag is set, its escort is released again
    /// ([`Session::release_stale`]; see the module docs).
    fn close_escort_dialog(&mut self) {
        let dialog = self.space.escorts.dialog.take();
        self.out.sounds.push(SimSound::EscortDialogClosed);
        if let Some(dialog) = dialog
            && self.space.escorts.release_comm_escort
        {
            self.release_stale(dialog.npc);
        }
    }

    /// Escort NPC `id` is released as `_DoEscortDialog`'s close releases
    /// it under a stale `_releaseCommEscort` (@0x95601-0x95672): it takes
    /// its share of the cargo by its ship class's `Holds`
    /// (`_DestroyPartialFleetCargo(+0x86)` @0x9561d), the fleet's holds
    /// as they stand, whether or not it is still in the fleet, so a second
    /// share after the dialog's own Release; it leaves the fleet (+0x9a
    /// -1), a save due when it was in it; its AI type is its class's
    /// `InherentAI` (+0x88); and it leaves (`_AIMakeShipLeave` @0x95652).
    /// None of Release's other resets is made: no +0xb9, +0xc8de or
    /// `_AISetup`.
    ///
    /// `_AIDispatch(ship, 1)` (@0x95664) thinks for it at once; the
    /// engine's next think does, a tick later, which crosses no boundary
    /// the player sees: no frame is drawn between the two.
    fn release_stale(&mut self, id: NpcId) {
        let Some(ship) = self.space.npc(id).map(|npc| npc.ship) else {
            return;
        };
        if let Some(holds) = self.statics.released_holds(&self.setup, ship) {
            self.destroy_cargo_share(holds, false, Some(id));
        }
        self.dismiss(id);
        let inherent = self
            .statics
            .ship_record(ship)
            .map(|record| AiType::from_raw(record.inherent_ai));
        if let Some(npc) = self.space.npc_mut(id) {
            if let Some(ai_type) = inherent {
                npc.ai_type = ai_type;
            }
            super::hail::make_leave(npc);
        }
    }
}

impl Session {
    /// Whether the comm dialog's release flag is set (see the module
    /// docs), to hand on to the next flight of the run
    /// ([`Session::with_release_comm_escort`]): the original's
    /// `_releaseCommEscort` (0x31b250, behind 0xdef2c) outlives a pilot
    /// load, though it is never saved.
    #[must_use]
    pub(crate) fn release_comm_escort(&self) -> bool {
        self.space.release_comm_escort()
    }

    /// This session with the comm dialog's release flag `set`, as the
    /// last flight of the run left it ([`Session::release_comm_escort`]),
    /// in place of a launch's clear one.
    #[must_use]
    pub(crate) fn with_release_comm_escort(mut self, set: bool) -> Self {
        self.space.set_release_comm_escort(set);
        self
    }
}

impl super::space::Space {
    /// Whether the comm dialog's release flag is set.
    fn release_comm_escort(&self) -> bool {
        self.escorts.release_comm_escort
    }

    /// Sets the comm dialog's release flag to `set`.
    fn set_release_comm_escort(&mut self, set: bool) {
        self.escorts.release_comm_escort = set;
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::ai::Goal;
    use crate::catalog::ShipId;
    use crate::chance::NeverFires;
    use crate::control::Test;
    use crate::escort_dialog::{SaleLine, UpgradeLine};
    use crate::fleet::OrderWord;
    use crate::hail::{HailOptions, ShownWage};
    use crate::self_repair::FormerEscort;
    use crate::session::Hailed;
    use crate::session::hail::tests::{hailable, npc, targeting};
    use crate::testkit::{Draws, FakePilotCatalog, ship};
    use crate::traffic::npc::MissionMark;

    /// [`hailable`] with the Cruiser (129) upgrading to class 130 for
    /// 6,000 and selling for 499, its subtitle "Heavy Warship"; class 130
    /// is always available.
    pub(in crate::session) fn upgradable() -> FakePilotCatalog {
        let mut catalog = hailable();
        let cruiser = &mut catalog.ship_records[0];
        cruiser.upgrade_to = Some(ShipId(130));
        cruiser.esc_upgrd_cost = 6_000;
        cruiser.esc_sell_value = 499;
        cruiser.subtitle = "Heavy Warship".to_owned();
        catalog
            .ship_records
            .push(ship(130, crate::handling::ShipFields::default()));
        catalog
    }

    /// `catalog`'s Cruiser (NPC 0) captured into the fleet, and targeted.
    pub(in crate::session) fn escorting(catalog: &FakePilotCatalog) -> Session {
        let mut session = targeting(catalog);
        session.join_fleet(NpcId(0), &mut NeverFires);
        session.chart.target = Some(NpcId(0));
        session.take_save_due();
        session.take_sounds();
        session
    }

    /// Presses the comm key on the target, drawing on nothing.
    fn hail_key(session: &mut Session, catalog: &FakePilotCatalog) -> (Hailed, Vec<u32>) {
        let mut chance = Draws::of(&[]);
        let hailed = session
            .hail_key(catalog, &HailOptions::default(), false, &mut chance)
            .expect("answered");
        (hailed, chance.asked)
    }

    pub(in crate::session) fn opened(
        session: &mut Session,
        catalog: &FakePilotCatalog,
    ) -> EscortDialogView {
        match hail_key(session, catalog).0 {
            Hailed::Escort(view) => view,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn hailing_a_fleet_escort_opens_the_escort_dialog_rolling_nothing() {
        let catalog = upgradable();
        let mut session = escorting(&catalog);
        let (hailed, asked) = hail_key(&mut session, &catalog);
        let Hailed::Escort(view) = hailed else {
            panic!("{hailed:?}");
        };
        assert_eq!(asked, [0_u32; 0], "no conversation is rolled");
        assert_eq!(view.ship, ShipId(129));
        assert!(!view.hired, "captured");
        assert_eq!(view.comm_name, "Cruiser");
        assert_eq!(view.subtitle.as_deref(), Some("Heavy Warship"));
        assert_eq!(view.upgrade, UpgradeLine::Cost(ShownWage::kept(6_000)));
        assert_eq!(view.sale, Some(SaleLine::Price(ShownWage::kept(499))));
        assert_eq!(view.pay, None);
        assert_eq!(session.take_sounds(), [SimSound::EscortDialogOpened]);
        assert_eq!(session.space.escorts.talk, None, "no comm talk opens");
        assert_eq!(session.escort_dialog(), Some(view));
    }

    #[test]
    fn the_dialog_shows_the_escort_hailed_whatever_its_place_in_the_fleet() {
        let catalog = upgradable();
        let mut session = escorting(&catalog);
        let first = crate::pilot::Escort {
            ship: ShipId(130),
            ..session.pilot.escorts[0]
        };
        session.pilot.escorts.insert(0, first);
        session.space.escorts.fleet.insert(0, None);
        assert_eq!(opened(&mut session, &catalog).ship, ShipId(129));
    }

    #[test]
    fn a_hired_escort_shows_its_pay_and_can_not_be_sold() {
        let catalog = upgradable();
        let mut session = escorting(&catalog);
        session.pilot.escorts[0].wage = Some(100);
        let view = opened(&mut session, &catalog);
        assert!(view.hired);
        assert_eq!(view.sale, None);
        assert_eq!(
            view.pay,
            Some(
                session
                    .setup
                    .shown_wage(&session.statics, &session.pilot.escorts[0])
            )
        );
        assert!(!view.can_sell);
        let view = session.press_escort(EscortButton::Sell).expect("open");
        assert_eq!(view.mark, None, "refused");
    }

    #[test]
    fn a_carried_fighter_and_a_mission_escort_keep_the_comm_dialog() {
        let catalog = upgradable();
        let mut fighter = escorting(&catalog);
        npc(&mut fighter).carrier = Some(crate::bay::Carrier {
            ship: crate::combat::ShipRef::Player,
            window: 100.0,
            reach: 40.0,
        });
        let mut chance = crate::testkit::Draws::of(&[0, 20, 1, 1, 0]);
        let hailed = fighter.hail_key(&catalog, &HailOptions::default(), false, &mut chance);
        assert!(matches!(hailed, Ok(Hailed::Comm(_))), "{hailed:?}");
        assert_eq!(fighter.escort_dialog(), None);
        let mut missioned = escorting(&catalog);
        npc(&mut missioned).mission = Some(MissionMark {
            slot: 0,
            behav: crate::mission::ships::ShipBehav::EscortPlayer,
            formation: None,
            order: None,
        });
        let mut chance = crate::testkit::Draws::of(&[0, 20, 1, 1, 0]);
        let hailed = missioned.hail_key(&catalog, &HailOptions::default(), false, &mut chance);
        assert!(matches!(hailed, Ok(Hailed::Comm(_))), "{hailed:?}");
    }

    #[test]
    fn upgrade_and_sell_toggle_the_escorts_mark_and_never_both_stand() {
        let catalog = upgradable();
        let mut session = escorting(&catalog);
        opened(&mut session, &catalog);
        let view = session.press_escort(EscortButton::Upgrade).expect("open");
        assert_eq!(view.mark, Some(EscortMark::Upgrade));
        assert_eq!(view.upgrade, UpgradeLine::Marked);
        assert_eq!(session.pilot.escorts[0].mark, Some(EscortMark::Upgrade));
        let view = session.press_escort(EscortButton::Sell).expect("open");
        assert_eq!(view.mark, Some(EscortMark::Sell));
        assert_eq!(view.sale, Some(SaleLine::Marked));
        let view = session.press_escort(EscortButton::Sell).expect("open");
        assert_eq!(view.mark, None);
        assert!(!session.take_save_due(), "the next save writes the mark");
        assert_eq!(session.take_sounds(), [SimSound::EscortDialogOpened]);
    }

    #[test]
    fn upgrade_is_refused_when_the_upgrade_class_is_not_available() {
        let mut catalog = upgradable();
        catalog.ship_records[1].availability = Test::parse("b7");
        let mut session = escorting(&catalog);
        let view = opened(&mut session, &catalog);
        assert!(!view.can_upgrade);
        assert_eq!(view.upgrade, UpgradeLine::Cannot);
        let view = session.press_escort(EscortButton::Upgrade).expect("open");
        assert_eq!(view.mark, None);
    }

    #[test]
    fn close_channel_closes_the_dialog_with_its_beep() {
        let catalog = upgradable();
        let mut session = escorting(&catalog);
        opened(&mut session, &catalog);
        session.take_sounds();
        assert_eq!(session.press_escort(EscortButton::Close), None);
        assert_eq!(session.take_sounds(), [SimSound::EscortDialogClosed]);
        assert_eq!(session.escort_dialog(), None);
        assert!(session.is_escort(NpcId(0)), "still in the fleet");
        assert_eq!(session.press_escort(EscortButton::Upgrade), None, "closed");
    }

    #[test]
    fn release_releases_the_escort_at_once_with_the_comm_dialogs_resets() {
        let catalog = upgradable();
        let mut session = escorting(&catalog);
        opened(&mut session, &catalog);
        session.take_sounds();
        npc(&mut session).order_word = OrderWord(2);
        let record = session.pilot.escorts[0];
        npc(&mut session).former = Some(FormerEscort { record });
        assert_eq!(session.press_escort(EscortButton::Release), None);
        assert!(!session.is_escort(NpcId(0)));
        assert!(session.pilot.escorts.is_empty());
        let released = npc(&mut session);
        assert_eq!(released.goal, Goal::JumpOut);
        assert!(released.boarded, "never boardable (+0xb9)");
        assert_eq!(released.former, None, "+0xc8de");
        assert_eq!(released.order_word, OrderWord::NONE, "_AISetup's +0xc90a");
        assert!(session.take_save_due());
        assert_eq!(session.take_sounds(), [SimSound::EscortDialogClosed]);
        assert_eq!(session.escort_dialog(), None);
    }

    #[test]
    fn the_comm_dialogs_release_makes_the_same_resets() {
        let catalog = upgradable();
        let mut session = escorting(&catalog);
        npc(&mut session).order_word = OrderWord(2);
        let record = session.pilot.escorts[0];
        npc(&mut session).former = Some(FormerEscort { record });
        session.release(NpcId(0));
        let released = npc(&mut session);
        assert!(released.boarded);
        assert_eq!(released.former, None);
        assert_eq!(released.order_word, OrderWord::NONE);
    }

    // The stale release flag (`_releaseCommEscort`, @0x95601-0x95672).

    /// [`escorting`] holding 30 tons of food, with a second escort, a copy
    /// of the first (NPC 1), released by the comm dialog's Release and
    /// hang-up, its share of the food taken: the session and the food
    /// left. The first escort is targeted again, its AI type a warship's
    /// (its class's `InherentAI` is not), its order word 2 and its former
    /// record set, so a reset shows.
    fn after_a_comm_release(catalog: &FakePilotCatalog) -> (Session, u32) {
        use crate::market::Good;
        let mut session = escorting(catalog);
        let copy = crate::traffic::npc::Npc {
            escort: None,
            ..session.npcs()[0].clone()
        };
        let second = session.space.traffic.add_npc(copy);
        session.join_fleet(second, &mut NeverFires);
        session.pilot.cargo = std::iter::once((Good::Commodity(0), 30)).collect();
        session.chart.target = Some(second);
        crate::session::hail::tests::hail(&mut session, catalog);
        session
            .answer(1, catalog, &HailOptions::default(), &mut NeverFires)
            .expect("open");
        session.hang_up();
        assert!(!session.is_escort(second), "released by the comm dialog");
        assert!(session.space.escorts.release_comm_escort);
        session.chart.target = Some(NpcId(0));
        let record = session.pilot.escorts[0];
        let first = npc(&mut session);
        first.ai_type = crate::traffic::npc::AiType::Warship;
        first.order_word = OrderWord(2);
        first.former = Some(FormerEscort { record });
        session.take_save_due();
        session.take_sounds();
        let held = session.pilot().held(Good::Commodity(0));
        (session, held)
    }

    #[test]
    fn after_a_comm_release_closing_an_escort_dialog_releases_its_escort_too() {
        use crate::market::Good;
        let catalog = upgradable();
        let (mut session, held) = after_a_comm_release(&catalog);
        assert!(held < 30, "the comm release took its share: {held}");
        opened(&mut session, &catalog);
        session.take_sounds();
        assert_eq!(session.press_escort(EscortButton::Close), None);
        assert_eq!(
            session.take_sounds(),
            [SimSound::EscortDialogClosed],
            "the beep first (@0x955fc)"
        );
        assert!(!session.is_escort(NpcId(0)), "+0x9a -1 (@0x95633)");
        assert!(session.pilot.escorts.is_empty());
        assert!(
            session.pilot().held(Good::Commodity(0)) < held,
            "_DestroyPartialFleetCargo (@0x9561d)"
        );
        let released = npc(&mut session).clone();
        assert_eq!(
            released.ai_type,
            crate::traffic::npc::AiType::WimpyTrader,
            "its class's InherentAI (@0x95640)"
        );
        assert_eq!(released.goal, Goal::JumpOut, "_AIMakeShipLeave (@0x95652)");
        assert!(!released.boarded, "no +0xb9");
        assert_eq!(released.order_word, OrderWord(2), "no _AISetup");
        assert!(released.former.is_some(), "no +0xc8de");
        assert!(session.take_save_due());
        assert!(session.space.escorts.release_comm_escort, "kept set");
    }

    #[test]
    fn a_stale_flag_takes_a_second_share_after_the_escort_dialogs_own_release() {
        use crate::market::Good;
        let catalog = upgradable();
        let held_after_release = |stale: bool| {
            let mut session = escorting(&catalog);
            session.pilot.cargo = std::iter::once((Good::Commodity(0), 30)).collect();
            session.space.escorts.release_comm_escort = stale;
            opened(&mut session, &catalog);
            session.take_sounds();
            assert_eq!(session.press_escort(EscortButton::Release), None);
            assert_eq!(session.take_sounds(), [SimSound::EscortDialogClosed]);
            assert!(!session.is_escort(NpcId(0)));
            session.pilot().held(Good::Commodity(0))
        };
        let once = held_after_release(false);
        let twice = held_after_release(true);
        assert!(once < 30, "the dialog's own release takes a share: {once}");
        assert!(
            twice < once,
            "a stale flag takes another, the escort out of the fleet (@0xcd32): {twice}"
        );
    }

    #[test]
    fn a_comm_dialog_in_between_clears_the_flag_and_the_escort_dialog_releases_nothing() {
        use crate::market::Good;
        let catalog = upgradable();
        let (mut session, held) = after_a_comm_release(&catalog);
        let stranger = session
            .npcs()
            .iter()
            .find(|npc| npc.id != NpcId(0))
            .expect("the released ship, leaving")
            .id;
        session.chart.target = Some(stranger);
        crate::session::hail::tests::hail(&mut session, &catalog);
        session.hang_up();
        assert!(!session.space.escorts.release_comm_escort, "@0x9576a");
        session.chart.target = Some(NpcId(0));
        opened(&mut session, &catalog);
        session.press_escort(EscortButton::Close);
        assert!(session.is_escort(NpcId(0)), "kept");
        assert_eq!(session.pilot().held(Good::Commodity(0)), held);
        assert_eq!(npc(&mut session).goal, Goal::Formation { guard: None });
    }

    #[test]
    fn a_fresh_session_has_no_release_flag_and_the_builder_sets_it() {
        let catalog = upgradable();
        let fresh = Session::start(&catalog).expect("starts");
        assert!(!fresh.release_comm_escort(), "BSS reads 0");
        let set = fresh.clone().with_release_comm_escort(true);
        assert!(set.release_comm_escort());
        assert!(!set.with_release_comm_escort(false).release_comm_escort());
    }

    /// Another pilot, Bea, flown and set up with `carry`, its first
    /// ship (NPC 0) captured into the fleet and targeted, holding 30 tons
    /// of food: the session and the food.
    fn next_pilot_escorting(
        catalog: &FakePilotCatalog,
        carry: crate::session::CarryOver,
    ) -> (Session, u32) {
        use crate::market::Good;
        let pilot = crate::pilot::Pilot::new(catalog, "Bea").expect("a pilot");
        let mut session =
            Session::fly(catalog, pilot)
                .expect("flies")
                .set_up(crate::session::SessionSetup {
                    carry,
                    ..Default::default()
                });
        session.populate(catalog, &mut crate::session::hail::tests::placed(0, -300));
        npc(&mut session).state.velocity = crate::geometry::Vec2::ZERO;
        session.pilot.cash = 100_000;
        session.join_fleet(NpcId(0), &mut NeverFires);
        session.chart.target = Some(NpcId(0));
        session.pilot.cargo = std::iter::once((Good::Commodity(0), 30)).collect();
        session.take_save_due();
        session.take_sounds();
        let held = session.pilot().held(Good::Commodity(0));
        (session, held)
    }

    /// The next pilot's escort dialog opened and closed by Close.
    fn close_the_next_pilots_dialog(session: &mut Session, catalog: &FakePilotCatalog) {
        opened(session, catalog);
        session.take_sounds();
        assert_eq!(session.press_escort(EscortButton::Close), None);
        assert_eq!(session.take_sounds(), [SimSound::EscortDialogClosed]);
    }

    #[test]
    fn a_comm_release_carried_to_the_next_pilot_releases_its_first_escort_dialogs_escort() {
        use crate::market::Good;
        let catalog = upgradable();
        let (first, _) = after_a_comm_release(&catalog);
        let (mut session, held) = next_pilot_escorting(&catalog, first.carry_over());
        assert!(session.release_comm_escort(), "carried");
        close_the_next_pilots_dialog(&mut session, &catalog);
        assert!(!session.is_escort(NpcId(0)), "+0x9a -1 (@0x95633)");
        assert!(session.pilot.escorts.is_empty());
        assert!(
            session.pilot().held(Good::Commodity(0)) < held,
            "_DestroyPartialFleetCargo (@0x9561d)"
        );
        let released = npc(&mut session).clone();
        assert_eq!(
            released.ai_type,
            AiType::WimpyTrader,
            "its class's InherentAI (@0x95640)"
        );
        assert_eq!(released.goal, Goal::JumpOut, "_AIMakeShipLeave (@0x95652)");
        assert!(session.take_save_due());
        assert!(session.release_comm_escort(), "kept set");
    }

    #[test]
    fn a_comm_dialog_before_the_carry_leaves_the_next_pilots_escort_kept() {
        use crate::market::Good;
        let catalog = upgradable();
        let (mut first, _) = after_a_comm_release(&catalog);
        let stranger = first
            .npcs()
            .iter()
            .find(|npc| npc.id != NpcId(0))
            .expect("the released ship, leaving")
            .id;
        first.chart.target = Some(stranger);
        crate::session::hail::tests::hail(&mut first, &catalog);
        first.hang_up();
        let carry = first.carry_over();
        assert_eq!(carry.release_comm_escort, Some(false), "@0x9576a");
        let (mut session, held) = next_pilot_escorting(&catalog, carry);
        close_the_next_pilots_dialog(&mut session, &catalog);
        assert!(session.is_escort(NpcId(0)), "kept");
        assert_eq!(session.pilot().held(Good::Commodity(0)), held);
    }

    #[test]
    fn a_launchs_carry_leaves_the_next_pilots_escort_kept() {
        let catalog = upgradable();
        let (mut session, _) = next_pilot_escorting(&catalog, crate::session::CarryOver::default());
        assert!(!session.release_comm_escort());
        close_the_next_pilots_dialog(&mut session, &catalog);
        assert!(session.is_escort(NpcId(0)), "kept");
    }

    #[test]
    fn the_marks_stay_through_a_new_system_and_a_landing() {
        let catalog = upgradable();
        let mut session = escorting(&catalog);
        opened(&mut session, &catalog);
        session.press_escort(EscortButton::Upgrade);
        session.press_escort(EscortButton::Close);
        session.populate(&catalog, &mut NeverFires);
        assert_eq!(session.pilot.escorts[0].mark, Some(EscortMark::Upgrade));
        session.dock(crate::catalog::StellarId(128));
        assert!(session.mode.landed().is_some());
        assert_eq!(session.pilot.escorts[0].mark, Some(EscortMark::Upgrade));
    }

    /// `catalog`'s escort with its dialog open, unmarked.
    pub(in crate::session) fn dialog_open(catalog: &FakePilotCatalog) -> Session {
        let mut session = escorting(catalog);
        opened(&mut session, catalog);
        assert!(session.escort_dialog().is_some());
        session
    }

    /// The escort dialog is closed: nothing shows, and Upgrade, pressed,
    /// marks nobody.
    pub(in crate::session) fn assert_closed(session: &mut Session) {
        assert_eq!(session.space.escorts.dialog, None, "closed");
        assert_eq!(session.escort_dialog(), None);
        let marks: Vec<_> = session
            .pilot
            .escorts
            .iter()
            .map(|escort| escort.mark)
            .collect();
        assert_eq!(session.press_escort(EscortButton::Upgrade), None);
        let after: Vec<_> = session
            .pilot
            .escorts
            .iter()
            .map(|escort| escort.mark)
            .collect();
        assert_eq!(after, marks, "no mark changed");
    }

    #[test]
    fn a_landing_closes_the_escort_dialog() {
        let catalog = upgradable();
        let mut session = dialog_open(&catalog);
        session.dock(crate::catalog::StellarId(128));
        assert_closed(&mut session);
    }

    #[test]
    fn setting_up_a_system_closes_the_escort_dialog() {
        let catalog = upgradable();
        let mut session = dialog_open(&catalog);
        session.populate(&catalog, &mut NeverFires);
        assert_closed(&mut session);
    }

    #[test]
    fn a_hail_closes_the_escort_dialog() {
        let catalog = upgradable();
        let mut session = dialog_open(&catalog);
        session.chart.target = None;
        let hailed = session.hail(&catalog, &HailOptions::default(), &mut NeverFires);
        assert!(hailed.is_err(), "no target answers");
        assert_closed(&mut session);
    }

    #[test]
    fn the_comm_key_closes_the_escort_dialog() {
        let catalog = upgradable();
        let mut session = dialog_open(&catalog);
        session.chart.target = None;
        let hailed = session.hail_key(&catalog, &HailOptions::default(), false, &mut NeverFires);
        assert!(hailed.is_err(), "no target answers");
        assert_closed(&mut session);
    }
}
