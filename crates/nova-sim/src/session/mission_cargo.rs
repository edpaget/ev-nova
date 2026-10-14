//! The session's side of mission cargo (see the
//! [`mission`](crate::mission) module's "Cargo"): loading it as a
//! mission is accepted, picking it up and dropping it off as the player
//! lands, the first of each slot's landing hooks
//! ([`ARRIVAL`](super::missions::ARRIVAL)), or boards its ship, the
//! player's jettison ([`Session::jettison_cargo`]), and an interceptor's
//! scan of it once it has looked the player over.

use super::Session;
use super::mission_screens::MissionTextKind;
use crate::catalog::{MissionId, MissionRecord, StellarId};
use crate::chance::{Chance, NeverFires};
use crate::market::Good;
use crate::message::SimMessage;
use crate::mission::cargo::{
    JettisonMessage, PICKUP_BOARDING, PickupFacts, ScanOutcome, drops_off, jettison,
    loads_at_accept, picks_up_at_destination, pickup_checks, scan,
};
use crate::traffic::npc::NpcId;

impl Session {
    /// Loads the cargo of `record`, just accepted into `slot`, as the
    /// accept does after `BriefText` (`_DoMissionAccept`
    /// @0xa1f28-0xa1fbf): with `PickupMode` 0 it is aboard, whatever its
    /// type, and its `LoadCargText` is shown, filled for the slot.
    /// Nothing when the slot no longer holds the mission.
    pub(super) fn load_at_accept(&mut self, slot: u8, record: &MissionRecord) {
        if !loads_at_accept(record) {
            return;
        }
        let Some(active) = self.slot_holding(slot, record.id) else {
            return;
        };
        active.aboard = true;
        self.show_mission_text(MissionTextKind::LoadCargo, record.id, Some(slot));
    }

    /// The cargo check of the mission in `slot` as the player lands on
    /// `stellar` (`_MissionLandCargoCheck` @0x9edc5), before its failure
    /// and success. At its destination, `PickupMode` 1 with nothing
    /// aboard picks the cargo up: room for it puts it aboard, reaches the
    /// destination and shows its `LoadCargText`, filled for the slot; no
    /// room shows why and leaves the destination unreached, so the next
    /// landing there tries again. Any other landing at the destination
    /// reaches it. Then the cargo is dropped off as [`drops_off`] says,
    /// showing its `DumpCargoText`.
    pub(super) fn cargo_at_landing(&mut self, slot: u8, stellar: StellarId) {
        let Some(active) = self.pilot.missions.get(slot) else {
            return;
        };
        let record = self.lifecycle_record(active.mission);
        let at_destination = self.docked_at_stop(active.destination, stellar);
        let at_return = self.docked_at_stop(active.return_to, stellar);
        if at_destination {
            let reached = !picks_up_at_destination(&record, active) || self.pick_up(slot);
            if let Some(active) = self.slot_holding(slot, record.id)
                && reached
            {
                active.reached = true;
            }
        }
        let drops = self
            .pilot
            .missions
            .get(slot)
            .is_some_and(|active| drops_off(&record, active, at_destination, at_return));
        if drops && let Some(active) = self.slot_holding(slot, record.id) {
            active.aboard = false;
            self.show_mission_text(MissionTextKind::DumpCargo, record.id, Some(slot));
        }
    }

    /// Picks up the cargo of the mission in `slot`, a `PickupMode` 2
    /// mission whose special ship the player has boarded
    /// (`_HandlePlayerBoardAttempt` @0x652ee-0x653ed), through the
    /// pickup's checks: whether it is aboard now. False for a free slot,
    /// or a mission of another `PickupMode`. [`Session::board`] calls it
    /// as it boards the mission's special ship.
    pub fn pick_up_boarded_cargo(&mut self, slot: u8) -> bool {
        let Some(mission) = self.pilot.missions.get(slot).map(|active| active.mission) else {
            return false;
        };
        if self.lifecycle_record(mission).pickup_mode != PICKUP_BOARDING {
            return false;
        }
        let picked = self.pickup_passes(slot, mission);
        if let Some(active) = self.slot_holding(slot, mission)
            && picked
        {
            active.aboard = true;
        }
        picked
    }

    /// Picks up the cargo of the mission in `slot` at its destination:
    /// whether it is aboard now, its `LoadCargText` shown.
    fn pick_up(&mut self, slot: u8) -> bool {
        let Some(mission) = self.pilot.missions.get(slot).map(|active| active.mission) else {
            return false;
        };
        if !self.pickup_passes(slot, mission) {
            return false;
        }
        if let Some(active) = self.slot_holding(slot, mission) {
            active.aboard = true;
        }
        self.show_mission_text(MissionTextKind::LoadCargo, mission, Some(slot));
        true
    }

    /// Jettisons the cargo, drawing on `chance`, as `_JettisonCargo`
    /// (@0x3dffd) does (see [`jettison`]): every `jünk` goes; with `all`,
    /// each abortable mission's cargo aboard goes and its mission fails at
    /// once, as a missed deadline does, in slot order; then the
    /// commodities go. In flight the player is told: "Mission failed.",
    /// or else "Cargo jettisoned." or "Non-mission cargo jettisoned."
    /// when anything went ([`SimMessage`]); landed, nothing. The cargo
    /// boxes it leaves are rdm:phase/missions/phase-19-jettison-boxes.
    pub fn jettison_cargo(&mut self, all: bool, chance: &mut dyn Chance) {
        let decided = jettison(
            &self.pilot.missions,
            |id| self.lifecycle_record(id),
            all,
            self.pilot.goods_held(),
        );
        self.pilot
            .cargo
            .retain(|good, _| matches!(good, Good::Commodity(_)));
        for &slot in &decided.slots {
            let Some(mission) = self.pilot.missions.get(slot).map(|active| active.mission) else {
                continue;
            };
            let record = self.lifecycle_record(mission);
            if let Some(active) = self.slot_holding(slot, mission) {
                active.aboard = false;
            }
            self.quick_failure(slot, &record, chance);
        }
        self.pilot.cargo.clear();
        self.save_due = true;
        // `_JettisonCargo`'s refresh of the blessing (@0x3e3b5; see the
        // `ranks` module).
        self.refresh_blessing();
        if self.landed.is_some() {
            return;
        }
        if let Some(message) = decided.message {
            self.messages.push(match message {
                JettisonMessage::MissionFailed => SimMessage::MissionFailed,
                JettisonMessage::CargoJettisoned => SimMessage::CargoJettisoned,
                JettisonMessage::NonMissionCargoJettisoned => SimMessage::NonMissionCargoJettisoned,
            });
        }
    }

    /// Scans the player's mission cargo for each NPC that has looked the
    /// player over since the last
    /// ([`Traffic::take_scans`](crate::traffic::Traffic::take_scans)),
    /// drawing on `chance`.
    pub(super) fn take_scans(&mut self, chance: &mut dyn Chance) {
        for npc in self.traffic.take_scans() {
            self.scanned_by(npc, chance);
        }
    }

    /// The scan of the player's mission cargo by NPC `id` (`_ScanPlayer`
    /// @0x7e46e; see [`scan`]), drawing on `chance`: nothing for a ship
    /// gone, or of no government. A mission found with `Flags` 0x0020
    /// fails at once ([`Session::quick_failure`]), the player told unless
    /// it has `Flags` 0x0400; any other found costs the fine, the cash
    /// floored at none, and the player is told of the smuggling, the
    /// scanner named by the found mission's drawn ship name when it has
    /// one (@0x7e7ce), whatever ship scanned. The
    /// player is always visible to it (no cloak). Its jump-timing check
    /// (@0x7e562-0x7e633) and the scan of illegal `jünk` and outfits
    /// (@0x7eabd-0x7f3d7) are rdm:phase/missions/phase-21-illegal-cargo-scanning.
    fn scanned_by(&mut self, id: NpcId, chance: &mut dyn Chance) {
        let Some((scanner, govt)) = self
            .traffic
            .npcs()
            .iter()
            .find(|npc| npc.id == id)
            .and_then(|npc| Some((npc.ship, self.govts.get(npc.govt?)?.clone())))
        else {
            return;
        };
        let outcome = scan(
            &govt,
            &self.pilot.missions,
            |mission| self.lifecycle_record(mission),
            self.pilot.cash,
            chance,
        );
        match outcome {
            ScanOutcome::Nothing => {}
            ScanOutcome::Fail { slot, announce } => {
                let Some(mission) = self.pilot.missions.get(slot).map(|active| active.mission)
                else {
                    return;
                };
                let record = self.lifecycle_record(mission);
                if announce {
                    self.messages.push(SimMessage::MissionScanned);
                }
                self.quick_failure(slot, &record, chance);
            }
            ScanOutcome::Fined { slot, credits } => {
                self.pilot.cash = self.pilot.cash.saturating_sub(credits).max(0);
                self.save_due = true;
                let ship_name = self
                    .pilot
                    .missions
                    .get(slot)
                    .map(|active| active.ships.name.clone())
                    .filter(|name| !name.is_empty());
                self.messages.push(SimMessage::Smuggling {
                    scanner,
                    ship_name,
                    fine: credits,
                });
            }
        }
    }

    /// Whether the pickup's checks pass for the cargo of `mission` in
    /// `slot` (`_DoMissionCargoPickup` @0x98292): the ship's holds, then
    /// the free space, for its tons. A refusal is shown; a pass refreshes
    /// the blessing table.
    fn pickup_passes(&mut self, slot: u8, mission: MissionId) -> bool {
        let tons = self
            .pilot
            .missions
            .get(slot)
            .map_or(0, |active| active.cargo_tons);
        let facts = PickupFacts {
            cargo: self.room_for(tons),
            rules: &self.rules,
        };
        match pickup_checks().first_refusal(&facts, &mut NeverFires) {
            Ok(()) => {
                // `_DoMissionCargoPickup`'s refresh of the blessing, for a
                // pickup that fits (@0x98316; see the `ranks` module).
                self.refresh_blessing();
                true
            }
            Err(refusal) => {
                self.show_text(
                    MissionTextKind::NoRoom,
                    mission,
                    refusal.message().to_owned(),
                );
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::catalog::{CommCatalog, MissionId, MissionTexts, PlaceNames, StellarId, SystemId};
    use crate::chance::NeverFires;
    use crate::control::Script;
    use crate::session::MissionText;
    use crate::testkit::{FakePilotCatalog, catalog, land_now, mission, with_variants};

    /// `STR#` 4001, the cargo names.
    #[derive(Debug)]
    struct Cargoes;

    impl CommCatalog for Cargoes {
        fn string_list(&self, id: i16) -> Vec<String> {
            match id {
                4001 => ["food", "industrial", "medical"]
                    .map(str::to_owned)
                    .to_vec(),
                _ => Vec::new(),
            }
        }
    }

    /// [`catalog`] with 131 linked back to 130, stellar 129 named Luna,
    /// `missions` and `texts`.
    fn cargo_catalog(missions: Vec<MissionRecord>, texts: Vec<MissionTexts>) -> FakePilotCatalog {
        let mut catalog = catalog();
        catalog.star_map[1].links.push(SystemId(130));
        catalog.place_names = PlaceNames {
            stellars: [(StellarId(129), "Luna".to_owned())].into(),
            systems: [(SystemId(130), "Sol".to_owned())].into(),
        };
        catalog.missions = missions;
        catalog.mission_texts = texts;
        catalog
    }

    /// A session of `catalog`, told [`Cargoes`], landed on 128 and
    /// settled, every roll 1, its texts taken.
    fn docked(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog)
            .expect("starts")
            .with_strings(Rc::new(Cargoes));
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        let _ = session.settle_script(catalog, &mut NeverFires);
        let _ = session.take_mission_texts();
        session
    }

    /// Mission `id`, offered at 128, to 129, carrying 5 tons of medical
    /// goods, loaded as `pickup_mode` says and unloaded as
    /// `drop_off_mode` says.
    fn courier(id: i16, pickup_mode: i16, drop_off_mode: i16) -> MissionRecord {
        MissionRecord {
            avail_stel: 128,
            travel_stel: 129,
            cargo_type: 2,
            cargo_qty: 5,
            pickup_mode,
            drop_off_mode,
            ..mission(id)
        }
    }

    /// Mission `id`'s texts: briefed "Brief.", completed "Done.", with
    /// these load and dump texts.
    fn texts(id: i16, load: Option<&str>, dump: Option<&str>) -> MissionTexts {
        MissionTexts {
            brief: Some("Brief.".to_owned()),
            comp: Some("Done.".to_owned()),
            load_cargo: load.map(str::to_owned),
            dump_cargo: dump.map(str::to_owned),
            ..MissionTexts::named(MissionId(id), "Courier")
        }
    }

    fn text(kind: MissionTextKind, mission: i16, text: &str) -> MissionText {
        MissionText {
            kind,
            mission: MissionId(mission),
            text: text.to_owned(),
        }
    }

    const LOAD: &str = "Loaded <CQ> tons of <CT> for <DST>.";

    fn aboard(session: &Session, slot: u8) -> bool {
        session.pilot().missions().get(slot).expect("held").aboard
    }

    #[test]
    fn accepting_a_pickup_mode_0_mission_loads_its_cargo_after_its_brief() {
        let catalog = cargo_catalog(vec![courier(140, 0, 0)], vec![texts(140, Some(LOAD), None)]);
        let mut session = docked(&catalog);
        let slot = session
            .accept_mission(MissionId(140), &mut NeverFires)
            .expect("accepted");
        assert!(aboard(&session, slot));
        assert_eq!(
            session.take_mission_texts(),
            [
                text(MissionTextKind::Brief, 140, "Brief."),
                text(
                    MissionTextKind::LoadCargo,
                    140,
                    "Loaded 5 tons of medical for Luna."
                ),
            ],
            "filled for the slot"
        );
        assert_eq!(session.pilot().total_cargo(), 5);
    }

    #[test]
    fn a_pickup_mode_0_mission_with_no_cargo_type_is_aboard_too() {
        let catalog = cargo_catalog(
            vec![MissionRecord {
                cargo_type: -1,
                ..courier(140, 0, 0)
            }],
            vec![texts(140, None, None)],
        );
        let mut session = docked(&catalog);
        let slot = session
            .accept_mission(MissionId(140), &mut NeverFires)
            .expect("accepted");
        assert!(aboard(&session, slot));
        assert_eq!(
            session.take_mission_texts(),
            [text(MissionTextKind::Brief, 140, "Brief.")],
            "no LoadCargText"
        );
    }

    #[test]
    fn the_other_pickup_modes_load_nothing_at_accept() {
        for mode in [1, 2, -1] {
            let catalog = cargo_catalog(
                vec![courier(140, mode, 0)],
                vec![texts(140, Some(LOAD), None)],
            );
            let mut session = docked(&catalog);
            let slot = session
                .accept_mission(MissionId(140), &mut NeverFires)
                .expect("accepted");
            assert!(!aboard(&session, slot), "{mode}");
            assert_eq!(
                session.take_mission_texts(),
                [text(MissionTextKind::Brief, 140, "Brief.")],
                "{mode}"
            );
        }
    }

    #[test]
    fn an_on_accept_that_aborts_its_own_mission_loads_nothing() {
        let catalog = cargo_catalog(
            vec![MissionRecord {
                on_accept: Script::parse("A140"),
                ..courier(140, 0, 0)
            }],
            vec![texts(140, Some(LOAD), None)],
        );
        let mut session = docked(&catalog);
        session
            .accept_mission(MissionId(140), &mut NeverFires)
            .expect("accepted");
        assert_eq!(session.pilot().total_cargo(), 0);
        assert!(
            session
                .take_mission_texts()
                .iter()
                .all(|shown| shown.kind != MissionTextKind::LoadCargo)
        );
    }

    // At a landing.

    use crate::catalog::MissionShip;
    use crate::geometry::Vec2;
    use crate::market::Good;
    use crate::mission::cargo::{NO_HOLDS_TO_LOAD, NO_SPACE_TO_LOAD};

    const DUMP: &str = "Unloaded <CQ> tons at <DST>.";

    /// Takes `session` off if it is landed, puts it at rest over
    /// `stellar` (one of 130's), lands there and settles.
    fn land_on(session: &mut Session, catalog: &FakePilotCatalog, stellar: i16) {
        if session.landed().is_some() {
            session
                .take_off(catalog, &mut NeverFires)
                .expect("takes off");
            let _ = session.settle_script(catalog, &mut NeverFires);
        }
        let site = *session
            .sites
            .iter()
            .find(|site| site.id == StellarId(stellar))
            .expect("shown");
        session.player.position = site.position;
        session.player.velocity = Vec2::ZERO;
        assert_eq!(land_now(session), Ok(StellarId(stellar)));
        let _ = session.settle_script(catalog, &mut NeverFires);
    }

    /// [`courier`] completed back at 128.
    fn round_trip(id: i16, pickup_mode: i16, drop_off_mode: i16) -> MissionRecord {
        MissionRecord {
            return_stel: 128,
            ..courier(id, pickup_mode, drop_off_mode)
        }
    }

    /// A session docked at 128 with `record` accepted into slot 0 and
    /// the texts taken.
    fn accepted(catalog: &FakePilotCatalog) -> Session {
        let mut session = docked(catalog);
        assert_eq!(
            session.accept_mission(MissionId(140), &mut NeverFires),
            Ok(0)
        );
        let _ = session.take_mission_texts();
        session
    }

    fn reached(session: &Session) -> bool {
        session.pilot().missions().get(0).expect("held").reached
    }

    fn kinds(session: &mut Session) -> Vec<MissionTextKind> {
        session
            .take_mission_texts()
            .into_iter()
            .map(|shown| shown.kind)
            .collect()
    }

    #[test]
    fn pickup_mode_1_loads_at_the_destination_reaching_it() {
        let catalog = cargo_catalog(
            vec![round_trip(140, 1, -1)],
            vec![texts(140, Some(LOAD), Some(DUMP))],
        );
        let mut session = accepted(&catalog);
        assert!(!aboard(&session, 0), "not at accept");
        land_on(&mut session, &catalog, 129);
        assert!(aboard(&session, 0));
        assert!(reached(&session));
        assert_eq!(
            session.take_mission_texts(),
            [text(
                MissionTextKind::LoadCargo,
                140,
                "Loaded 5 tons of medical for Luna."
            )]
        );
        land_on(&mut session, &catalog, 129);
        assert_eq!(kinds(&mut session), [], "aboard: loaded once");
    }

    #[test]
    fn a_pickup_without_the_holds_or_the_space_is_refused_and_tried_again_next_landing() {
        let catalog = cargo_catalog(
            vec![round_trip(140, 1, -1)],
            vec![texts(140, Some(LOAD), None)],
        );
        let mut session = accepted(&catalog);
        session.pilot.missions.get_mut(0).expect("held").cargo_tons = 21;
        land_on(&mut session, &catalog, 129);
        assert_eq!(
            session.take_mission_texts(),
            [text(MissionTextKind::NoRoom, 140, NO_HOLDS_TO_LOAD)],
            "20 holds"
        );
        assert!(!aboard(&session, 0));
        assert!(!reached(&session), "not reached");
        session.pilot.missions.get_mut(0).expect("held").cargo_tons = 5;
        session.pilot.cargo.insert(Good::Commodity(0), 16);
        land_on(&mut session, &catalog, 129);
        assert_eq!(
            session.take_mission_texts(),
            [text(MissionTextKind::NoRoom, 140, NO_SPACE_TO_LOAD)],
            "4 free"
        );
        assert!(!aboard(&session, 0));
        assert!(!reached(&session));
        session.pilot.cargo.clear();
        land_on(&mut session, &catalog, 129);
        assert!(aboard(&session, 0), "room at last");
        assert!(reached(&session));
        assert_eq!(kinds(&mut session), [MissionTextKind::LoadCargo]);
    }

    #[test]
    fn pickup_mode_1_with_drop_off_0_loads_and_unloads_on_one_landing() {
        let catalog = cargo_catalog(
            vec![round_trip(140, 1, 0)],
            vec![texts(140, Some(LOAD), Some(DUMP))],
        );
        let mut session = accepted(&catalog);
        land_on(&mut session, &catalog, 129);
        assert_eq!(
            kinds(&mut session),
            [MissionTextKind::LoadCargo, MissionTextKind::DumpCargo]
        );
        assert!(!aboard(&session, 0));
        assert!(reached(&session));
    }

    #[test]
    fn pickup_mode_2_loads_on_boarding_its_ship_and_not_at_the_destination() {
        let catalog = cargo_catalog(
            vec![round_trip(140, 2, 1)],
            vec![texts(140, Some(LOAD), Some(DUMP))],
        );
        let mut session = accepted(&catalog);
        land_on(&mut session, &catalog, 129);
        assert!(!aboard(&session, 0), "not at the destination");
        assert!(reached(&session));
        assert_eq!(kinds(&mut session), []);
        assert!(session.pick_up_boarded_cargo(0));
        assert!(aboard(&session, 0));
        assert!(!session.pick_up_boarded_cargo(1), "a free slot");
        let mut session = accepted(&catalog);
        session.pilot.missions.get_mut(0).expect("held").cargo_tons = 21;
        assert!(!session.pick_up_boarded_cargo(0));
        assert!(!aboard(&session, 0));
        assert_eq!(
            session.take_mission_texts(),
            [text(MissionTextKind::NoRoom, 140, NO_HOLDS_TO_LOAD)]
        );
        let catalog = cargo_catalog(vec![round_trip(140, 1, 1)], vec![]);
        let mut session = accepted(&catalog);
        assert!(!session.pick_up_boarded_cargo(0), "PickupMode 1");
        assert!(!aboard(&session, 0));
    }

    /// Runs `K200`.
    fn grant_blessing_rank(session: &mut Session) {
        session.run_set(
            &crate::control::SetExpr::parse("K200").expect("parses"),
            &mut NeverFires,
        );
    }

    #[test]
    fn a_boarded_cargo_pickup_refreshes_the_blessing() {
        use crate::session::ranks::tests::{blessed, blessing_rank};
        let mut catalog = cargo_catalog(vec![round_trip(140, 2, 1)], vec![]);
        catalog.ranks = vec![blessing_rank()];
        let mut session = accepted(&catalog);
        session.pilot.missions.get_mut(0).expect("held").cargo_tons = 21;
        grant_blessing_rank(&mut session);
        assert!(!session.pick_up_boarded_cargo(0), "no room");
        assert!(!blessed(&session), "only a pickup that fits (@0x98316)");
        session.pilot.missions.get_mut(0).expect("held").cargo_tons = 5;
        assert!(session.pick_up_boarded_cargo(0));
        assert!(blessed(&session));
    }

    #[test]
    fn a_landing_cargo_pickup_refreshes_the_blessing() {
        use crate::session::ranks::tests::{blessed, blessing_rank};
        let mut catalog = cargo_catalog(vec![round_trip(140, 1, 1)], vec![]);
        catalog.ranks = vec![blessing_rank()];
        let mut session = accepted(&catalog);
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        let _ = session.settle_script(&catalog, &mut NeverFires);
        grant_blessing_rank(&mut session);
        assert!(!blessed(&session));
        let site = *session
            .sites
            .iter()
            .find(|site| site.id == StellarId(129))
            .expect("shown");
        session.player.position = site.position;
        session.player.velocity = Vec2::ZERO;
        assert_eq!(land_now(&mut session), Ok(StellarId(129)));
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert!(aboard(&session, 0));
        assert!(blessed(&session), "the pickup's, shared with boarding");
    }

    #[test]
    fn pickup_mode_minus_1_never_loads_so_never_unloads() {
        let catalog = cargo_catalog(
            vec![courier(140, -1, 0)],
            vec![texts(140, Some(LOAD), Some(DUMP))],
        );
        let mut session = accepted(&catalog);
        assert!(!aboard(&session, 0));
        land_on(&mut session, &catalog, 129);
        assert_eq!(kinds(&mut session), [MissionTextKind::Comp], "completed");
    }

    #[test]
    fn drop_off_0_unloads_at_the_destination_alone() {
        let catalog = cargo_catalog(
            vec![round_trip(140, 0, 0)],
            vec![texts(140, Some(LOAD), Some(DUMP))],
        );
        let mut session = accepted(&catalog);
        land_on(&mut session, &catalog, 128);
        assert!(aboard(&session, 0), "the return, not the destination");
        assert_eq!(kinds(&mut session), []);
        land_on(&mut session, &catalog, 129);
        assert!(!aboard(&session, 0));
        assert_eq!(
            session.take_mission_texts(),
            [text(
                MissionTextKind::DumpCargo,
                140,
                "Unloaded 5 tons at Luna."
            )]
        );
        assert_eq!(session.pilot().total_cargo(), 0);
    }

    #[test]
    fn drop_off_1_unloads_at_the_return_before_the_comp_text() {
        let catalog = cargo_catalog(
            vec![round_trip(140, 0, 1)],
            vec![texts(140, Some(LOAD), Some(DUMP))],
        );
        let mut session = accepted(&catalog);
        land_on(&mut session, &catalog, 129);
        assert!(aboard(&session, 0), "the destination, not the return");
        assert_eq!(kinds(&mut session), []);
        land_on(&mut session, &catalog, 128);
        assert_eq!(
            kinds(&mut session),
            [MissionTextKind::DumpCargo, MissionTextKind::Comp]
        );
        assert_eq!(session.pilot().missions().iter().count(), 0, "completed");
    }

    #[test]
    fn pickup_mode_1_picks_up_at_another_stellar_of_its_destinations_place_and_name() {
        let catalog = with_variants(cargo_catalog(
            vec![round_trip(140, 1, -1)],
            vec![texts(140, Some(LOAD), None)],
        ));
        let mut session = accepted(&catalog);
        land_on(&mut session, &catalog, 151);
        assert!(aboard(&session, 0), "151 is 129's variant");
        assert!(reached(&session));
        assert_eq!(kinds(&mut session), [MissionTextKind::LoadCargo]);
    }

    #[test]
    fn drop_off_mode_1_drops_off_at_another_stellar_of_its_returns_place_and_name() {
        let catalog = with_variants(cargo_catalog(
            vec![round_trip(140, 0, 1)],
            vec![texts(140, Some(LOAD), Some(DUMP))],
        ));
        let mut session = accepted(&catalog);
        land_on(&mut session, &catalog, 129);
        land_on(&mut session, &catalog, 150);
        assert_eq!(
            kinds(&mut session),
            [MissionTextKind::DumpCargo, MissionTextKind::Comp],
            "150 is 128's variant"
        );
        assert_eq!(session.pilot().missions().iter().count(), 0, "completed");
    }

    #[test]
    fn drop_off_1_keeps_the_cargo_aboard_until_the_goals_are_met_unless_there_is_no_goal() {
        let catalog = cargo_catalog(
            vec![MissionRecord {
                ships: MissionShip {
                    count: 1,
                    goal: 0,
                    ..MissionShip::NONE
                },
                ..round_trip(140, 0, 1)
            }],
            vec![texts(140, Some(LOAD), Some(DUMP))],
        );
        let mut session = accepted(&catalog);
        land_on(&mut session, &catalog, 129);
        land_on(&mut session, &catalog, 128);
        assert!(aboard(&session, 0), "a goal not met");
        assert_eq!(kinds(&mut session), []);
        let catalog = cargo_catalog(
            vec![MissionRecord {
                // Its ship comes later, so its goals stay unmet.
                ships: MissionShip {
                    count: 1,
                    goal: -1,
                    start: 1,
                    ..MissionShip::NONE
                },
                flags: 0x0001,
                ..round_trip(140, 0, 1)
            }],
            vec![texts(140, Some(LOAD), Some(DUMP))],
        );
        let mut session = accepted(&catalog);
        assert!(!session.pilot().missions().get(0).expect("held").goals_met);
        land_on(&mut session, &catalog, 129);
        land_on(&mut session, &catalog, 128);
        assert!(!aboard(&session, 0), "no ShipGoal");
        assert_eq!(kinds(&mut session), [MissionTextKind::DumpCargo]);
        assert_eq!(session.pilot().missions().iter().count(), 1, "goals unmet");
    }

    #[test]
    fn drop_off_minus_1_keeps_the_cargo_until_its_slot_is_freed() {
        let catalog = cargo_catalog(
            vec![courier(140, 0, -1)],
            vec![texts(140, Some(LOAD), Some(DUMP))],
        );
        let mut session = accepted(&catalog);
        assert_eq!(session.pilot().total_cargo(), 5);
        land_on(&mut session, &catalog, 129);
        assert_eq!(
            kinds(&mut session),
            [MissionTextKind::Comp],
            "no DumpCargoText"
        );
        assert_eq!(session.pilot().total_cargo(), 0, "gone with its slot");
    }

    #[test]
    fn a_mission_whose_record_is_gone_loads_and_unloads_nothing_at_a_landing() {
        let catalog = cargo_catalog(
            vec![round_trip(140, -1, -1), round_trip(141, 0, -1)],
            vec![],
        );
        let mut session = docked(&catalog);
        for id in [140, 141] {
            session
                .accept_mission(MissionId(id), &mut NeverFires)
                .expect("accepted");
        }
        session.missions.clear();
        land_on(&mut session, &catalog, 128);
        assert!(aboard(&session, 1), "no DropOffMode: kept at its return");
        land_on(&mut session, &catalog, 129);
        assert!(!aboard(&session, 0), "no PickupMode: nothing picked up");
    }

    #[test]
    fn a_mission_with_no_cargo_type_still_shows_its_dump_text() {
        let catalog = cargo_catalog(
            vec![MissionRecord {
                cargo_type: -1,
                cargo_qty: 0,
                ..round_trip(140, 0, 0)
            }],
            vec![texts(140, None, Some("Dumped."))],
        );
        let mut session = accepted(&catalog);
        land_on(&mut session, &catalog, 129);
        assert_eq!(
            session.take_mission_texts(),
            [text(MissionTextKind::DumpCargo, 140, "Dumped.")]
        );
    }

    // The jettison.

    use crate::SimMessage;
    use crate::catalog::JunkId;
    use crate::control::Bit;

    fn bit(n: u16) -> Bit {
        Bit::new(n).expect("in range")
    }

    /// Missions 140 (4 tons of medical goods, its `OnFailure` setting bit
    /// 40), 141 (6 tons, unabortable, its `OnFailure` setting bit 41) and
    /// 142 (3 tons, hidden, its `OnFailure` setting bit 42), each loaded
    /// at accept, to 129 and back to 128.
    fn cargo_missions() -> FakePilotCatalog {
        let failing = |id: i16, tons: i16, bit: u16| MissionRecord {
            cargo_qty: tons,
            on_failure: Script::parse(&format!("b{bit}")),
            ..round_trip(id, 0, 1)
        };
        cargo_catalog(
            vec![
                failing(140, 4, 40),
                MissionRecord {
                    can_abort: 0,
                    ..failing(141, 6, 41)
                },
                MissionRecord {
                    flags: 0x0400,
                    ..failing(142, 3, 42)
                },
            ],
            vec![],
        )
    }

    /// A session of `catalog` that accepted `missions` docked at 128,
    /// holding 3 tons of food and 2 of a jünk, its texts taken.
    fn loaded(catalog: &FakePilotCatalog, missions: &[i16]) -> Session {
        let mut session = docked(catalog);
        for &id in missions {
            session
                .accept_mission(MissionId(id), &mut NeverFires)
                .expect("accepted");
        }
        session.pilot.cargo.insert(Good::Commodity(0), 3);
        session.pilot.cargo.insert(Good::Junk(JunkId(146)), 2);
        let _ = session.take_mission_texts();
        session
    }

    /// Takes `session` off and settles, its messages taken.
    fn in_flight(session: &mut Session, catalog: &FakePilotCatalog) {
        session
            .take_off(catalog, &mut NeverFires)
            .expect("takes off");
        let _ = session.settle_script(catalog, &mut NeverFires);
        let _ = session.take_messages();
    }

    fn held_missions(session: &Session) -> Vec<(u8, i16)> {
        session
            .pilot()
            .missions()
            .iter()
            .map(|(slot, active)| (slot, active.mission.0))
            .collect()
    }

    #[test]
    fn jettisoning_all_in_flight_fails_each_abortable_mission_whose_cargo_goes() {
        let catalog = cargo_missions();
        let mut session = loaded(&catalog, &[140, 141]);
        in_flight(&mut session, &catalog);
        let _ = session.take_save_due();
        session.jettison_cargo(true, &mut NeverFires);
        assert_eq!(session.pilot().goods_held(), 0, "the food and the jünk");
        assert!(session.control_bit(bit(40)), "OnFailure ran");
        assert!(!session.control_bit(bit(41)));
        assert_eq!(held_missions(&session), [(1, 141)], "140 ended");
        let kept = session.pilot().missions().get(1).expect("held");
        assert!(kept.aboard, "the unabortable mission's cargo stays");
        assert_eq!(kept.failed, None);
        assert_eq!(session.pilot().total_cargo(), 6);
        assert_eq!(session.take_messages(), [SimMessage::MissionFailed]);
        assert_eq!(session.offers.rolled_at(), None, "the offers forgotten");
        assert!(session.take_save_due());
    }

    #[test]
    fn jettisoning_with_option_keeps_every_missions_cargo() {
        let catalog = cargo_missions();
        let mut session = loaded(&catalog, &[140, 141]);
        in_flight(&mut session, &catalog);
        session.jettison_cargo(false, &mut NeverFires);
        assert_eq!(session.pilot().goods_held(), 0);
        assert_eq!(session.pilot().total_cargo(), 10, "4 and 6 aboard");
        assert_eq!(held_missions(&session), [(0, 140), (1, 141)]);
        assert!(!session.control_bit(bit(40)));
        assert_eq!(
            session.take_messages(),
            [SimMessage::NonMissionCargoJettisoned]
        );
    }

    #[test]
    fn a_hidden_missions_cargo_fails_it_silently_and_the_cargo_is_jettisoned() {
        let catalog = cargo_missions();
        let mut session = loaded(&catalog, &[142]);
        in_flight(&mut session, &catalog);
        session.jettison_cargo(true, &mut NeverFires);
        assert!(session.control_bit(bit(42)));
        assert_eq!(held_missions(&session), []);
        assert_eq!(session.take_messages(), [SimMessage::CargoJettisoned]);
    }

    #[test]
    fn jettisoning_landed_does_the_same_and_says_nothing() {
        let catalog = cargo_missions();
        let mut session = loaded(&catalog, &[140, 141]);
        let _ = session.take_messages();
        session.jettison_cargo(true, &mut NeverFires);
        assert_eq!(session.pilot().goods_held(), 0);
        assert!(session.control_bit(bit(40)));
        assert_eq!(held_missions(&session), [(1, 141)]);
        assert_eq!(session.take_messages(), []);
    }

    #[test]
    fn jettisoning_nothing_says_nothing() {
        let catalog = cargo_missions();
        let mut session = docked(&catalog);
        in_flight(&mut session, &catalog);
        session.jettison_cargo(true, &mut NeverFires);
        assert_eq!(session.take_messages(), []);
        session.jettison_cargo(false, &mut NeverFires);
        assert_eq!(session.take_messages(), []);
    }

    #[test]
    fn an_unabortable_missions_cargo_alone_is_kept_with_nothing_said() {
        let catalog = cargo_missions();
        let mut session = docked(&catalog);
        session
            .accept_mission(MissionId(141), &mut NeverFires)
            .expect("accepted");
        in_flight(&mut session, &catalog);
        session.jettison_cargo(true, &mut NeverFires);
        assert_eq!(session.take_messages(), []);
        let kept = session.pilot().missions().get(0).expect("held");
        assert_eq!((kept.aboard, kept.failed), (true, None));
    }

    // The scan.

    use crate::ai::{Behaviour, Goal, Surroundings};
    use crate::catalog::{GovtId, GovtRecord, Penalties, ShipId};
    use crate::combat::ShipRef;
    use crate::testkit::{Draws, govt, npc};
    use crate::traffic::npc::{AiType, Npc};

    /// [`cargo_catalog`] of `missions` with the Federation (128): a
    /// `SmugPenalty` of 5, scanning for bit 0x0004, fining `fine`.
    fn policed(missions: Vec<MissionRecord>, fine: i16) -> FakePilotCatalog {
        let mut catalog = cargo_catalog(missions, vec![]);
        catalog.govts = vec![GovtRecord {
            penalties: Penalties {
                smuggle: 5,
                ..Penalties::default()
            },
            scan_mask: 0x0004,
            scan_fine: fine,
            news_pic: -1,
            ..govt(128)
        }];
        catalog
    }

    /// Mission `id` of [`round_trip`] masked `mask`, with `Flags` `flags`;
    /// its `OnFailure` sets bit 45.
    fn masked(id: i16, mask: u16, flags: u16) -> MissionRecord {
        MissionRecord {
            scan_mask: mask,
            flags,
            on_failure: Script::parse("b45"),
            ..round_trip(id, 0, 1)
        }
    }

    /// Ends every goal: an inspection is over.
    #[derive(Debug)]
    struct LookedOver;

    impl Behaviour for LookedOver {
        fn decide(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            Goal::Idle
        }
    }

    /// A session of `catalog` in flight with `missions` accepted and an
    /// interceptor of `govt`, flying ship 129, inspecting the player where
    /// it is, its messages taken.
    fn inspected(catalog: &FakePilotCatalog, missions: &[i16], govt: Option<GovtId>) -> Session {
        let mut session = loaded(catalog, missions);
        in_flight(&mut session, catalog);
        session.tick_traffic(catalog, &LookedOver, &mut NeverFires);
        let mut police = npc(0, session.stats());
        police.govt = govt;
        police.ai_type = AiType::Interceptor;
        police.ship = ShipId(129);
        police.goal = Goal::Inspect(ShipRef::Player);
        police.state.position = session.player.position;
        session.traffic.add_npc(police);
        let _ = session.take_messages();
        session
    }

    /// The interceptor gives the inspection up, drawing `draw` for the
    /// scan.
    fn looked_over(session: &mut Session, catalog: &FakePilotCatalog, draw: u32) -> Vec<u32> {
        let mut chance = Draws::of(&[draw]);
        session.tick_traffic(catalog, &LookedOver, &mut chance);
        chance.asked
    }

    #[test]
    fn a_scan_finding_a_0x0020_missions_cargo_fails_it_at_once() {
        let catalog = policed(vec![masked(140, 0x0006, 0x0020)], 500);
        let mut session = inspected(&catalog, &[140], Some(GovtId(128)));
        let cash = session.pilot().cash();
        assert_eq!(looked_over(&mut session, &catalog, 75), [100]);
        assert!(session.control_bit(bit(45)), "OnFailure ran");
        assert_eq!(held_missions(&session), [], "ended");
        assert_eq!(session.take_messages(), [SimMessage::MissionScanned]);
        assert_eq!(session.pilot().cash(), cash, "no fine");
    }

    #[test]
    fn a_hidden_0x0020_mission_found_fails_silently() {
        let catalog = policed(vec![masked(140, 0x0004, 0x0420)], 500);
        let mut session = inspected(&catalog, &[140], Some(GovtId(128)));
        looked_over(&mut session, &catalog, 0);
        assert!(session.control_bit(bit(45)));
        assert_eq!(held_missions(&session), []);
        assert_eq!(session.take_messages(), []);
    }

    #[test]
    fn a_scan_sharing_no_mask_bit_leaves_the_mission_active() {
        let catalog = policed(vec![masked(140, 0x0003, 0x0020)], 500);
        let mut session = inspected(&catalog, &[140], Some(GovtId(128)));
        let cash = session.pilot().cash();
        looked_over(&mut session, &catalog, 0);
        assert!(!session.control_bit(bit(45)));
        assert_eq!(held_missions(&session), [(0, 140)]);
        assert_eq!(session.take_messages(), []);
        assert_eq!(session.pilot().cash(), cash);
    }

    #[test]
    fn a_scan_finding_any_other_missions_cargo_fines_the_smuggler() {
        let catalog = policed(vec![masked(140, 0x0004, 0)], 500);
        let mut session = inspected(&catalog, &[140], Some(GovtId(128)));
        session.pilot.cash = 1200;
        let _ = session.take_save_due();
        looked_over(&mut session, &catalog, 0);
        assert_eq!(session.pilot().cash(), 700);
        assert!(session.take_save_due());
        assert_eq!(held_missions(&session), [(0, 140)], "still active");
        assert!(!session.control_bit(bit(45)));
        assert_eq!(
            session.take_messages(),
            [SimMessage::Smuggling {
                scanner: ShipId(129),
                ship_name: None,
                fine: 500
            }]
        );
        let mut session = inspected(&catalog, &[140], Some(GovtId(128)));
        session.pilot.cash = 100;
        looked_over(&mut session, &catalog, 0);
        assert_eq!(session.pilot().cash(), 0, "floored at none");
    }

    /// `_ScanPlayer` @0x7e7ce reads the drawn ship name (+0x70) of the
    /// slot whose cargo it found, `%edi`, the loop's slot pointer, never
    /// the scanner's mark: that name, when not empty, names any scanner.
    #[test]
    fn a_scan_finding_a_mission_with_a_ship_name_names_the_scanner_by_it() {
        let catalog = policed(vec![masked(140, 0x0004, 0), masked(141, 0x0001, 0)], 500);
        for scanner_slot in [None, Some(1)] {
            let mut session = inspected(&catalog, &[140, 141], Some(GovtId(128)));
            for (slot, name) in [(0, "Rover"), (1, "Ace")] {
                if let Some(active) = session.pilot.missions.get_mut(slot) {
                    active.ships.name = name.to_owned();
                }
            }
            if let Some(police) = session.traffic.npcs_mut().last_mut() {
                police.mission = scanner_slot.map(|slot| crate::traffic::npc::MissionMark {
                    slot,
                    behav: crate::mission::ships::ShipBehav::Own,
                    formation: None,
                });
            }
            looked_over(&mut session, &catalog, 0);
            assert_eq!(
                session.take_messages(),
                [SimMessage::Smuggling {
                    scanner: ShipId(129),
                    ship_name: Some("Rover".to_owned()),
                    fine: 500
                }],
                "{scanner_slot:?}"
            );
        }
    }

    #[test]
    fn a_scan_drawing_above_75_or_by_an_ungoverned_ship_finds_nothing() {
        let catalog = policed(vec![masked(140, 0x0004, 0x0020)], 500);
        let mut session = inspected(&catalog, &[140], Some(GovtId(128)));
        assert_eq!(looked_over(&mut session, &catalog, 76), [100]);
        assert_eq!(held_missions(&session), [(0, 140)]);
        assert_eq!(session.take_messages(), []);
        let mut session = inspected(&catalog, &[140], None);
        assert!(looked_over(&mut session, &catalog, 0).is_empty(), "no draw");
        assert_eq!(held_missions(&session), [(0, 140)]);
        let mut session = inspected(&catalog, &[140], Some(GovtId(130)));
        assert!(looked_over(&mut session, &catalog, 0).is_empty(), "no gövt");
        assert_eq!(held_missions(&session), [(0, 140)]);
    }
}
