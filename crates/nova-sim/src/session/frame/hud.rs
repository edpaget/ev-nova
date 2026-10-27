//! The HUD's half of [`Session::fill_frame`]: the status bar's gauges,
//! cargo area, secondary weapon line and radar gear, the government its
//! `ïntf` is chosen by, and the target layer's brackets, Red Alert icon
//! and escort menu rows ([`FrameHud`]), each read from the sub-state that
//! holds it; and the target panel's target ([`FrameTarget`]).

use super::super::Session;
use crate::flight_frame::{FrameHud, FrameTarget};

impl Session {
    /// Fills `into` with what the HUD shows now, every field written. The
    /// gauges, the cargo area, the secondary weapon and its count, the
    /// radar gear, the interference, the `Interface` government, the nav
    /// brackets, the escort rows and whether an escort is flying are each
    /// one sub-state's; the target
    /// brackets read the chart and the space, so the fill is the
    /// session's.
    pub(super) fn fill_hud(&self, into: &mut FrameHud) {
        let FrameHud {
            reserves,
            cargo,
            secondary,
            secondary_line_count,
            radar_gear,
            interference,
            interface_government,
            nav_brackets,
            target_brackets,
            red_alert,
            escort_rows,
            escorts_flying,
        } = into;
        let (pilot, statics) = (&self.pilot, &self.statics);
        *reserves = super::super::reserves(pilot);
        *cargo = self.ship.status_cargo(pilot, statics, &self.setup);
        *secondary = self.ship.secondary();
        *secondary_line_count = self.ship.secondary_line_count(pilot, statics);
        *radar_gear = super::super::radar::radar_gear(pilot, statics);
        *interference = self.chart.interference(pilot, statics);
        *interface_government = super::super::interface_government(pilot, statics);
        *nav_brackets = self.chart.nav_brackets();
        *target_brackets = self.target_brackets();
        *red_alert = self.red_alert.icon();
        *escort_rows = self.space.escort_rows(pilot, statics);
        *escorts_flying = self.space.escorts_flying(pilot);
    }

    /// Fills `into` with the ship target as the target panel shows it, or
    /// none without one: its name and subtitle as the space gives them,
    /// a mission ship's drawn ones while it is marked
    /// (`Space::npc_name`, `Space::npc_subtitle`), and its status line and
    /// label, which read the statics, the missions and the disable rule
    /// ([`Session::target_status`], [`Session::target_tag`]). A target
    /// already filled keeps its strings' buffers, so a steady fill
    /// doesn't allocate.
    pub(super) fn fill_target(&self, into: &mut Option<FrameTarget>) {
        let Some(npc) = self.target() else {
            *into = None;
            return;
        };
        let name = self
            .space
            .npc_name(&self.pilot, &self.statics, npc)
            .unwrap_or_default();
        let subtitle = self.space.npc_subtitle(&self.pilot, npc);
        let (status, tag) = (self.target_status(npc), self.target_tag(npc));
        let target = into.get_or_insert_with(|| FrameTarget {
            npc: npc.id,
            name: String::new(),
            subtitle: None,
            status,
            tag,
        });
        target.npc = npc.id;
        name.clone_into(&mut target.name);
        match (subtitle, &mut target.subtitle) {
            (Some(subtitle), Some(kept)) => subtitle.clone_into(kept),
            (subtitle, kept) => *kept = subtitle.map(str::to_owned),
        }
        target.status = status;
        target.tag = tag;
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::filled;
    use super::*;
    use crate::catalog::{GovtId, OutfitId, StarSystem, StellarId, SystemId};
    use crate::escort::{ClassRow, EscortClass};
    use crate::iff::{MOD_ANTI_INTERFERENCE, MOD_IFF, RadarGear};
    use crate::market::Good;
    use crate::red_alert::AlertIcon;
    use crate::reserves::{Gauge, Reserves};
    use crate::session::StellarPick;
    use crate::session::tests::{ROCKET, armed, facing_an_npc, three_stellars, with_secondaries};
    use crate::target_panel::{TargetStatus, TargetTag};
    use crate::targeting::TargetPick;
    use crate::testkit::{FAST, FakePilotCatalog, catalog, outfit, ship};
    use crate::traffic::npc::NpcId;

    #[test]
    fn a_fill_writes_the_ships_reserves() {
        let session = Session::start(&catalog()).expect("starts");
        assert_eq!(
            filled(&session).hud.reserves,
            Reserves {
                shield: Gauge::full(30.0),
                armor: Gauge::full(45.0),
                fuel: Gauge::full(300.0),
            }
        );
    }

    #[test]
    fn a_fill_writes_what_the_cargo_area_lists() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.pilot.cargo.insert(Good::Commodity(1), 7);
        session.pilot.cash = 12_345;
        let cargo = filled(&session).hud.cargo;
        assert_eq!(cargo.cash, 12_345);
        assert_eq!(cargo.rows[1].as_ref().map(|row| row.tons), Some(7));
        assert_eq!(cargo, session.status_cargo());
    }

    #[test]
    fn a_fill_writes_the_secondary_weapon_and_its_lines_count() {
        let armed = with_secondaries(catalog());
        let mut session = Session::start(&armed).expect("starts");
        let frame = filled(&session);
        assert_eq!(
            (frame.hud.secondary, frame.hud.secondary_line_count),
            (None, None)
        );
        session.select_secondary(false);
        let frame = filled(&session);
        assert_eq!(frame.hud.secondary, Some(ROCKET));
        assert_eq!(frame.hud.secondary_line_count, Some(3), "its 3 rockets");
        assert_eq!(
            frame.hud.secondary_line_count,
            session.secondary_line_count()
        );
    }

    /// `catalog()` with IFF and anti-interference 30 as outfits 300 and
    /// 302, and system 130's `Interference` 60.
    fn radar_catalog() -> FakePilotCatalog {
        let base = catalog();
        FakePilotCatalog {
            outfits: vec![
                outfit(300, &[(MOD_IFF, 0)]),
                outfit(302, &[(MOD_ANTI_INTERFERENCE, 30)]),
            ],
            star_map: base
                .star_map
                .iter()
                .map(|system| StarSystem {
                    interference: if system.id == SystemId(130) { 60 } else { 0 },
                    ..system.clone()
                })
                .collect(),
            ..base
        }
    }

    #[test]
    fn a_fill_writes_the_radar_gear_and_the_interference() {
        let catalog = radar_catalog();
        let mut session = Session::start(&catalog).expect("starts");
        let frame = filled(&session);
        assert_eq!(frame.hud.radar_gear, RadarGear::default());
        assert_eq!(frame.hud.interference, 60);
        session.pilot.outfits.insert(OutfitId(300), 1);
        session.pilot.outfits.insert(OutfitId(302), 1);
        let frame = filled(&session);
        assert_eq!(
            frame.hud.radar_gear,
            RadarGear {
                iff: true,
                density_scanner: false,
                anti_interference: 30,
            }
        );
        assert_eq!(frame.hud.interference, 30);
    }

    #[test]
    fn a_fill_writes_the_government_the_bar_is_chosen_by() {
        let mut record = ship(128, FAST);
        record.inherent_govt = Some(GovtId(130));
        let catalog = FakePilotCatalog {
            ship_records: vec![record],
            ..catalog()
        };
        let session = Session::start(&catalog).expect("starts");
        assert_eq!(filled(&session).hud.interface_government, Some(GovtId(130)));
        let session = Session::start(&crate::testkit::catalog()).expect("starts");
        assert_eq!(filled(&session).hud.interface_government, None);
    }

    #[test]
    fn a_fill_writes_the_nav_brackets() {
        let mut session = Session::start(&three_stellars()).expect("starts");
        assert_eq!(filled(&session).hud.nav_brackets, None);
        session.select_stellar(StellarPick::Slot(0));
        let brackets = filled(&session).hud.nav_brackets.expect("bracketed");
        assert_eq!(brackets.stellar, crate::catalog::StellarId(131));
        assert_eq!(Some(brackets), session.nav_brackets());
    }

    #[test]
    fn a_fill_writes_the_target_brackets() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 0);
        assert_eq!(filled(&session).hud.target_brackets, None);
        session.select_target(TargetPick::Next);
        let brackets = filled(&session).hud.target_brackets.expect("bracketed");
        assert_eq!(brackets.npc, NpcId(0));
        assert_eq!(Some(brackets), session.target_brackets());
    }

    #[test]
    fn a_fill_writes_the_red_alert_icon_shown() {
        let mut session = Session::start(&catalog()).expect("starts");
        assert_eq!(filled(&session).hud.red_alert, None);
        session.red_alert.check(true, false, true);
        session.red_alert.step(1);
        let icon = filled(&session).hud.red_alert;
        assert_eq!(icon, Some(AlertIcon::Red));
        assert_eq!(icon, session.red_alert_icon());
    }

    #[test]
    fn a_fill_writes_the_escort_menus_rows_in_their_order() {
        let session = Session::start(&catalog()).expect("starts");
        let rows = filled(&session).hud.escort_rows;
        assert_eq!(
            rows,
            EscortClass::ALL.map(|class| ClassRow {
                class,
                present: false,
                order: None,
            })
        );
        assert_eq!(rows, session.escort_menu());
    }

    #[test]
    fn a_fill_writes_whether_an_escort_is_flying() {
        use crate::session::mission_escorts::tests::{escorting, flying};
        use crate::session::mission_ships::tests::ships_catalog;
        let alone = Session::start(&catalog()).expect("starts");
        assert!(!filled(&alone).hud.escorts_flying, "no escort");
        let catalog = ships_catalog(escorting(-1, 0));
        let mut escorted = flying(&catalog);
        assert!(filled(&escorted).hud.escorts_flying, "two mission escorts");
        for npc in escorted.space.traffic.npcs_mut() {
            npc.condition = crate::combat::hull::Condition::Disabled;
        }
        let frame = filled(&escorted);
        assert!(
            frame.hud.escort_rows.iter().any(|row| row.present),
            "their row stays"
        );
        assert!(!frame.hud.escorts_flying, "both disabled");
    }

    // The target panel's target and the stellars' IFF.

    #[test]
    fn a_fill_writes_no_target_without_one() {
        let catalog = armed();
        let session = facing_an_npc(&catalog, 0);
        assert_eq!(filled(&session).target, None);
    }

    #[test]
    fn a_fill_writes_the_targets_name_subtitle_status_and_tag() {
        use crate::session::mission_ships::tests::{accepted, shipping, ships_catalog, take_off};
        let catalog = ships_catalog(shipping(-1, 0));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        let id = session.npcs()[0].id;
        session.space.traffic.npcs_mut()[0].govt = Some(GovtId(140));
        session.chart.target = Some(id);
        let npc = session.npcs()[0].clone();
        let target = filled(&session).target.expect("a target");
        assert_eq!(
            target,
            FrameTarget {
                npc: id,
                name: "Rover".to_owned(),
                subtitle: Some("Bounty Hunter".to_owned()),
                status: session.target_status(&npc),
                tag: Some(TargetTag::Code(GovtId(140))),
            }
        );
        assert_eq!(target.status, TargetStatus::Shield(100));
    }

    #[test]
    fn an_unmarked_targets_name_is_its_ship_types_with_no_subtitle() {
        use crate::session::mission_ships::tests::{accepted, shipping, ships_catalog, take_off};
        let catalog = ships_catalog(shipping(-1, 0));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        session.space.traffic.npcs_mut()[0].mission = None;
        session.chart.target = Some(session.npcs()[0].id);
        let target = filled(&session).target.expect("a target");
        assert_eq!(target.name, "Ship 200");
        assert_eq!(target.subtitle, None);
        assert_eq!(
            target.tag,
            Some(TargetTag::Code(GovtId(131))),
            "its government's"
        );
    }

    #[test]
    fn a_shown_stellar_has_its_iff_and_one_not_shown_none() {
        let mut session = Session::start(&three_stellars()).expect("starts");
        session.chart.sites.remove(0);
        let frame = filled(&session);
        let iff = |id| frame.stellar(StellarId(id)).expect("listed").iff;
        assert_eq!(iff(128), session.stellar_iff(StellarId(128)));
        assert!(iff(128).is_some());
        assert_eq!(iff(131), None, "not shown");
    }

    #[test]
    fn a_failed_sessions_hud_has_no_brackets_alert_or_escorts() {
        let hud = FrameHud::default();
        assert_eq!((hud.nav_brackets, hud.target_brackets), (None, None));
        assert_eq!(hud.red_alert, None);
        assert!(hud.escort_rows.iter().all(|row| !row.present));
        assert!(
            hud.escort_rows
                .iter()
                .map(|row| row.class)
                .eq(EscortClass::ALL)
        );
        assert_eq!(hud.reserves, Reserves::default());
    }
}
