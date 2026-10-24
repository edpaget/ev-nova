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
use crate::traffic::npc::{Npc, NpcId};

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
        let index = self.fleet_index(id)?;
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
    pub fn escort_dialog(&self) -> Option<EscortDialogView> {
        let dialog = self.space.escorts.dialog?;
        let escort = self.pilot.escorts.get(self.fleet_index(dialog.npc)?)?;
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
    pub fn press_escort(&mut self, button: EscortButton) -> Option<EscortDialogView> {
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
        let index = self.fleet_index(dialog.npc)?;
        let escort = self.pilot.escorts.get_mut(index)?;
        let allowed = match pressed {
            EscortMark::Upgrade => dialog.can_upgrade,
            EscortMark::Sell => escort_dialog::can_sell(escort),
        };
        escort.mark = escort_dialog::toggle(escort.mark, pressed, allowed);
        self.escort_dialog()
    }

    /// The escort dialog closes, beeping.
    fn close_escort_dialog(&mut self) {
        self.space.escorts.dialog = None;
        self.out.sounds.push(SimSound::EscortDialogClosed);
    }

    /// The fleet index of escort NPC `id`, if it is in the fleet.
    pub(super) fn fleet_index(&self, id: NpcId) -> Option<usize> {
        self.space
            .escorts
            .fleet
            .iter()
            .position(|&placed| placed == Some(id))
    }
}

#[cfg(test)]
mod tests {
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
    fn upgradable() -> FakePilotCatalog {
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
    fn escorting(catalog: &FakePilotCatalog) -> Session {
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

    fn opened(session: &mut Session, catalog: &FakePilotCatalog) -> EscortDialogView {
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
}
