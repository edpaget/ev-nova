//! The session's side of mission cargo (see the
//! [`mission`](crate::mission) module's "Cargo"): loading it as a
//! mission is accepted, picking it up and dropping it off as the player
//! lands, the first of each slot's landing hooks
//! ([`ARRIVAL`](super::missions::ARRIVAL)), or boards its ship, the
//! player's jettison ([`Session::jettison_cargo`]), and an interceptor's
//! scan of the player once it has looked the player over: the mission
//! cargo, then the illegal `jünk` and outfits (see the
//! [`scan`](crate::scan) module).

use super::Session;
use super::mission_screens::MissionTextKind;
use crate::ai::Goal;
use crate::catalog::{GovtRecord, MissionId, MissionRecord, ShipId, StellarId};
use crate::chance::{Chance, NeverFires};
use crate::combat::ShipRef;
use crate::legal::{self, Crime, LegalCode};
use crate::market::Good;
use crate::message::SimMessage;
use crate::mission::cargo::{
    JettisonMessage, PICKUP_BOARDING, PickupFacts, ScanOutcome, drops_off, jettison,
    loads_at_accept, picks_up_at_destination, pickup_checks, scan,
};
use crate::rulebook::RuleKey;
use crate::scan::{self, Contraband};
use crate::sound::SimSound;
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
    /// when anything went ([`SimMessage`]); landed, nothing. In flight,
    /// when anything went, it sounds [`SimSound::CargoJettisoned`]
    /// (@0x3e2fe-0x3e33d), a mission failed or not, and drops the cargo
    /// boxes behind the player and its trader escorts, drawing on
    /// `chance` (see the `cargo_boxes` module); landed, neither.
    pub fn jettison_cargo(&mut self, all: bool, chance: &mut dyn Chance) {
        let goods = self.pilot.goods_held();
        let decided = jettison(
            &self.pilot.missions,
            |id| self.lifecycle_record(id),
            all,
            goods,
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
        // `_JettisonCargo`'s reset (@0x3e3b5; see the `precalced` module):
        // every good is gone, so its trim never cuts.
        self.reset_precalced();
        if self.landed.is_some() {
            return;
        }
        self.drop_cargo_boxes(decided.total, goods, chance);
        if decided.total > 0 {
            self.sounds.push(SimSound::CargoJettisoned);
        }
        if let Some(message) = decided.message {
            self.messages.push(match message {
                JettisonMessage::MissionFailed => SimMessage::MissionFailed,
                JettisonMessage::CargoJettisoned => SimMessage::CargoJettisoned,
                JettisonMessage::NonMissionCargoJettisoned => SimMessage::NonMissionCargoJettisoned,
            });
        }
    }

    /// Scans the player for each NPC that has looked the player over
    /// since the last
    /// ([`Traffic::take_scans`](crate::traffic::Traffic::take_scans)),
    /// convicting by `law` and drawing on `chance`. `Session::tick_combat`
    /// takes them, after its return while the player is landed or jumping:
    /// that return is the engine's reading of `_ScanPlayer`'s jump-timing
    /// check (@0x7e562-0x7e633; see the [`scan`](crate::scan) module), so
    /// no scan reaches a player whose warp has begun. The scans wait from
    /// the traffic tick that queued them to the combat tick of the same
    /// frame, so they settle before the frame is drawn; a scan queued as
    /// the warp begins is dropped with the system's traffic.
    pub(super) fn take_scans(&mut self, law: &dyn LegalCode, chance: &mut dyn Chance) {
        for npc in self.traffic.take_scans() {
            self.scanned_by(npc, law, chance);
        }
    }

    /// The scan of the player by NPC `id` (`_ScanPlayer` @0x7e46e),
    /// convicting by `law` and drawing on `chance`: nothing for a ship
    /// gone or of no government, or that does not go on (see
    /// [`scan::goes_on`]). Then the mission cargo (see [`scan`]): a
    /// mission found with `Flags` 0x0020 fails at once
    /// ([`Session::quick_failure`]), the player told, with
    /// [`SimSound::ScanFailedMission`], unless it has `Flags` 0x0400; any
    /// other found costs the fine, the cash floored at none, and the player
    /// is told of the smuggling, with [`SimSound::Smuggling`], the scanner
    /// named by the found mission's drawn ship name when it has one
    /// (@0x7e7ce), whatever ship scanned. Either find sets the scanner
    /// attacking the player (@0x7e7b1-0x7e7c0, @0x7e8f6-0x7e905).
    ///
    /// Then, found or not, the contraband ([`scan::contraband`], against
    /// the cash left): a find convicts the player of [`Crime::Smuggle`]
    /// against the scanner's government and takes the ranks it reaches
    /// (`_SlapWithPenalty` @0x7ebcc, @0x7f01f, passing slot -1, so a
    /// mission ship's scan convicts too), sets the scanner attacking the
    /// player, takes the fine, the cash floored at none, tells the player
    /// ([`SimMessage::IllegalJunk`], [`SimMessage::IllegalOutfit`]) with
    /// [`SimSound::Smuggling`], and clears its latch. The player is always
    /// visible to the scanner (no cloak).
    fn scanned_by(&mut self, id: NpcId, law: &dyn LegalCode, chance: &mut dyn Chance) {
        let Some(npc) = self.traffic.npcs().iter().find(|npc| npc.id == id) else {
            return;
        };
        let (scanner, ai_type) = (npc.ship, npc.ai_type);
        let off = self.player.position - npc.state.position;
        let person = npc
            .person
            .and_then(|person| self.traffic.person(person.id))
            .map(|person| person.record.name.clone());
        let Some(govt_id) = npc.govt else {
            return;
        };
        let Some(govt) = self.govts.get(govt_id).cloned() else {
            return;
        };
        if !scan::goes_on(ai_type, &govt, off, chance) {
            return;
        }
        self.scan_mission_cargo(id, scanner, &govt, chance);
        let carried = scan::Carried {
            cargo: &self.pilot.cargo,
            junk: self.goods.junk(),
            outfits: &self.pilot.outfits,
            outfit_records: &self.outfits,
        };
        let rules = scan::ScanRules {
            outfits_past_junk: self.rules.source_for(RuleKey::ScanOutfitsPastJunk),
            outfit_fine: self.rules.source_for(RuleKey::OutfitScanFine),
        };
        let Some(found) =
            scan::contraband(&govt, &carried, self.pilot.cash, self.scan_latches, rules)
        else {
            return;
        };
        let changes = law.penalties(Crime::Smuggle, Some(govt_id), &self.govts);
        legal::convict(&mut self.pilot, &changes, self.galaxy.governed());
        self.lose_ranks_to(Crime::Smuggle, Some(govt_id));
        self.set_on_player(id);
        let (Contraband::Junk { fine, .. } | Contraband::Outfit { fine, .. }) = found;
        self.take_scan_fine(fine);
        self.sounds.push(SimSound::Smuggling);
        self.messages.push(match found {
            Contraband::Junk { id: junk, fine } => SimMessage::IllegalJunk {
                scanner,
                person,
                junk,
                fine,
            },
            Contraband::Outfit {
                id: outfit,
                one,
                fine,
            } => SimMessage::IllegalOutfit {
                scanner,
                person,
                outfit,
                one,
                fine,
            },
        });
        self.scan_latches.clear(&found);
    }

    /// The mission cargo's part of the scan by NPC `id`, of ship type
    /// `scanner` and government `govt` (see [`Session::scanned_by`]).
    fn scan_mission_cargo(
        &mut self,
        id: NpcId,
        scanner: ShipId,
        govt: &GovtRecord,
        chance: &mut dyn Chance,
    ) {
        let outcome = scan(
            govt,
            &self.pilot.missions,
            |mission| self.lifecycle_record(mission),
            self.pilot.cash,
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
                    self.sounds.push(SimSound::ScanFailedMission);
                    self.messages.push(SimMessage::MissionScanned);
                }
                self.quick_failure(slot, &record, chance);
                self.set_on_player(id);
            }
            ScanOutcome::Fined { slot, credits } => {
                let ship_name = self
                    .pilot
                    .missions
                    .get(slot)
                    .map(|active| active.ships.name.clone())
                    .filter(|name| !name.is_empty());
                self.set_on_player(id);
                self.take_scan_fine(credits);
                self.sounds.push(SimSound::Smuggling);
                self.messages.push(SimMessage::Smuggling {
                    scanner,
                    ship_name,
                    fine: credits,
                });
            }
        }
    }

    /// Sets NPC `id` attacking the player, as a scan's find does: target
    /// the player, no stellar, AI state attack (+0x70, +0x6c, +0xc8c8, as
    /// `_AIMakeShipAttackPlayer` @0x89d3e-0x89d4d sets them).
    fn set_on_player(&mut self, id: NpcId) {
        if let Some(npc) = self.traffic.npcs_mut().iter_mut().find(|npc| npc.id == id) {
            npc.goal = Goal::Attack(ShipRef::Player);
            npc.target = Some(ShipRef::Player);
        }
    }

    /// Takes a scan's fine of `credits` from the cash, floored at none
    /// (@0x7ea0f-0x7ea2d, @0x7ee41-0x7ee65, @0x7f2dd-0x7f326); a fine of
    /// none changes nothing and makes no save due.
    fn take_scan_fine(&mut self, credits: i64) {
        if credits != 0 {
            self.pilot.cash = self.pilot.cash.saturating_sub(credits).max(0);
            self.save_due = true;
        }
    }

    /// Whether the pickup's checks pass for the cargo of `mission` in
    /// `slot` (`_DoMissionCargoPickup` @0x98292): the ship's holds, then
    /// the free space, for its tons. A refusal is shown; a pass resets
    /// ([`Session::reset_precalced`]), before the cargo is aboard.
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
                // `_DoMissionCargoPickup`'s reset, for a pickup that fits,
                // before the cargo is aboard (@0x98316, then the caller's
                // @0x9ee41; see the `precalced` module).
                self.reset_precalced();
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

    /// Makes rank 200 active outside any reset, as no set expression can,
    /// each resetting as it ends.
    fn grant_blessing_rank(session: &mut Session) {
        session.pilot.ranks.insert(crate::catalog::RankId(200));
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

    // The jettison's boxes and sound.

    use crate::SimSound;

    #[test]
    fn the_boxes_dropped_count_the_mission_cargo_unless_option_keeps_it() {
        for (all, boxes) in [(true, 4), (false, 3)] {
            let catalog = cargo_missions();
            let mut session = loaded(&catalog, &[140, 141]);
            in_flight(&mut session, &catalog);
            // 16 tons of goods, and mission 140's 4 with them: past the
            // holds, which the take-off's reset would have trimmed.
            session.pilot.cargo.insert(Good::Commodity(0), 14);
            session.jettison_cargo(all, &mut NeverFires);
            assert_eq!(session.cargo_boxes().len(), boxes, "{all}");
        }
    }

    #[test]
    fn a_jettison_of_mission_cargo_alone_drops_a_box_and_sounds() {
        let catalog = cargo_missions();
        let mut session = docked(&catalog);
        session
            .accept_mission(MissionId(140), &mut NeverFires)
            .expect("accepted");
        in_flight(&mut session, &catalog);
        let _ = session.take_sounds();
        session.jettison_cargo(true, &mut NeverFires);
        assert_eq!(session.cargo_boxes().len(), 1, "its 4 tons");
        assert_eq!(session.take_messages(), [SimMessage::MissionFailed]);
        assert_eq!(
            session.take_sounds(),
            [SimSound::CargoJettisoned],
            "a mission failed or not"
        );
    }

    #[test]
    fn an_unabortable_missions_cargo_kept_alone_drops_nothing_and_sounds_nothing() {
        let catalog = cargo_missions();
        let mut session = docked(&catalog);
        session
            .accept_mission(MissionId(141), &mut NeverFires)
            .expect("accepted");
        in_flight(&mut session, &catalog);
        let _ = session.take_sounds();
        session.jettison_cargo(true, &mut NeverFires);
        assert!(session.cargo_boxes().is_empty());
        assert_eq!(session.take_sounds(), []);
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

    /// The interceptor gives the inspection up in a traffic tick, and the
    /// combat tick after it takes the scan, drawing `draw`.
    fn looked_over(session: &mut Session, catalog: &FakePilotCatalog, draw: u32) -> Vec<u32> {
        let mut chance = Draws::of(&[draw]);
        session.tick_traffic(catalog, &LookedOver, &mut chance);
        session.tick_combat(crate::Rules::default(), &mut chance);
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
                    order: None,
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

    // The contraband.

    use crate::catalog::{JunkRecord, OutfitId, OutfitRecord, RankId, RankRecord};
    use crate::rulebook::{RuleSource, Rulebook};
    use crate::scan::Latches;
    use crate::testkit::outfit;

    const BIO_WEAPONS: JunkId = JunkId(150);
    const BLASTER: OutfitId = OutfitId(150);

    /// [`policed`] of no missions, fining `fine`, with bio-weapons
    /// (`jünk` 150) and blasters (outfit 150), both masked 0x0004.
    fn contraband_catalog(fine: i16) -> FakePilotCatalog {
        let mut catalog = policed(vec![], fine);
        catalog.junk.push(JunkRecord {
            id: BIO_WEAPONS,
            name: "Bio-weapons".to_owned(),
            abbrev: "Bio".to_owned(),
            base_price: 1000,
            sold_at: Vec::new(),
            bought_at: Vec::new(),
            buy_on: crate::control::Test::default(),
            sell_on: crate::control::Test::default(),
            flags: 0,
            scan_mask: 0x0004,
            lc_name: "bio-weapons".to_owned(),
        });
        catalog.outfits.push(OutfitRecord {
            scan_mask: 0x0004,
            lc_name: "blaster".to_owned(),
            lc_plural: "blasters".to_owned(),
            ..outfit(150, &[])
        });
        catalog
    }

    /// A session of `catalog` in flight, inspected by an interceptor of
    /// the Federation, holding `tons` of bio-weapons and `blasters`
    /// blasters, with `cash`; its sounds and save taken.
    fn smuggler(catalog: &FakePilotCatalog, tons: u32, blasters: u16, cash: i64) -> Session {
        let mut session = inspected(catalog, &[], Some(GovtId(128)));
        if tons > 0 {
            session.pilot.cargo.insert(Good::Junk(BIO_WEAPONS), tons);
        }
        if blasters > 0 {
            session.pilot.outfits.insert(BLASTER, blasters);
        }
        session.pilot.cash = cash;
        let _ = session.take_sounds();
        let _ = session.take_save_due();
        session
    }

    /// The interceptor, the last NPC.
    fn police(session: &mut Session) -> &mut Npc {
        session
            .traffic
            .npcs_mut()
            .last_mut()
            .expect("the interceptor")
    }

    /// The interceptor inspects the player again, where it is.
    fn inspect_again(session: &mut Session) {
        let at = session.player.position;
        let police = police(session);
        police.goal = Goal::Inspect(ShipRef::Player);
        police.state.position = at;
    }

    fn illegal_junk(fine: i64) -> SimMessage {
        SimMessage::IllegalJunk {
            scanner: ShipId(129),
            person: None,
            junk: BIO_WEAPONS,
            fine,
        }
    }

    fn illegal_outfit(one: bool, fine: i64) -> SimMessage {
        SimMessage::IllegalOutfit {
            scanner: ShipId(129),
            person: None,
            outfit: BLASTER,
            one,
            fine,
        }
    }

    #[test]
    fn the_session_names_a_junk_and_an_outfit_by_their_records() {
        let catalog = contraband_catalog(0);
        let session = Session::start(&catalog).expect("starts");
        assert_eq!(session.junk_lc_name(BIO_WEAPONS), Some("bio-weapons"));
        assert_eq!(session.junk_lc_name(JunkId(151)), None);
        assert_eq!(session.outfit_names(BLASTER), Some(("blaster", "blasters")));
        assert_eq!(session.outfit_names(OutfitId(151)), None);
    }

    #[test]
    fn a_scan_finding_illegal_junk_convicts_and_fines_the_smuggler_and_attacks() {
        let catalog = contraband_catalog(500);
        let mut session = smuggler(&catalog, 2, 0, 1200);
        let record = session.pilot().legal_record(GovtId(128));
        assert_eq!(looked_over(&mut session, &catalog, 0), [100]);
        assert_eq!(session.pilot().cash(), 700);
        assert!(session.take_save_due());
        assert_eq!(session.take_messages(), [illegal_junk(500)]);
        assert_eq!(session.take_sounds(), [SimSound::Smuggling]);
        assert_eq!(
            session.pilot().legal_record(GovtId(128)),
            record - 5,
            "SmugPenalty (@0x7ebcc)"
        );
        let police = police(&mut session);
        assert_eq!(police.goal, Goal::Attack(ShipRef::Player));
        assert_eq!(police.target, Some(ShipRef::Player));
        assert_eq!(
            session.scan_latches,
            Latches {
                junk: false,
                outfits: true
            }
        );
    }

    #[test]
    fn a_junk_conviction_stops_junk_scans_until_a_reset_and_lets_the_outfits_be_scanned() {
        let catalog = contraband_catalog(500);
        let mut session = smuggler(&catalog, 2, 1, 100_000);
        looked_over(&mut session, &catalog, 0);
        assert_eq!(session.take_messages(), [illegal_junk(500)]);
        inspect_again(&mut session);
        looked_over(&mut session, &catalog, 0);
        assert_eq!(
            session.take_messages(),
            [illegal_outfit(true, 500)],
            "the jünk latch is clear"
        );
        assert_eq!(
            session.take_sounds(),
            [SimSound::Smuggling, SimSound::Smuggling]
        );
        inspect_again(&mut session);
        looked_over(&mut session, &catalog, 0);
        assert_eq!(session.take_messages(), [], "both latches clear");
        assert_eq!(session.pilot().cash(), 99_000);
        // A jettison is a reset (@0x3e3b5): it re-arms both latches, and the
        // bio-weapons are gone with it, so the blaster is found again.
        session.jettison_cargo(false, &mut NeverFires);
        assert_eq!(session.scan_latches, Latches::ARMED);
        let _ = session.take_messages();
        inspect_again(&mut session);
        looked_over(&mut session, &catalog, 0);
        assert_eq!(session.take_messages(), [illegal_outfit(true, 500)]);
    }

    #[test]
    fn any_set_expression_rearms_the_latches_a_conviction_cleared() {
        let catalog = contraband_catalog(500);
        let mut session = smuggler(&catalog, 2, 0, 100_000);
        looked_over(&mut session, &catalog, 0);
        assert_eq!(session.take_messages(), [illegal_junk(500)]);
        assert!(!session.scan_latches.junk);
        session.run_set(
            &crate::control::SetExpr::parse("b1").expect("parses"),
            &mut NeverFires,
        );
        assert_eq!(session.scan_latches, Latches::ARMED, "@0x99a85");
        inspect_again(&mut session);
        looked_over(&mut session, &catalog, 0);
        assert_eq!(session.take_messages(), [illegal_junk(500)], "found again");
    }

    #[test]
    fn a_scan_finding_an_illegal_outfit_names_one_or_more_and_fines_only_above_none() {
        let catalog = contraband_catalog(500);
        let mut session = smuggler(&catalog, 0, 3, 1200);
        looked_over(&mut session, &catalog, 0);
        assert_eq!(session.take_messages(), [illegal_outfit(false, 500)]);
        assert_eq!(session.pilot().cash(), 700);
        assert!(session.take_save_due());
        assert_eq!(session.take_sounds(), [SimSound::Smuggling]);
        assert_eq!(police(&mut session).goal, Goal::Attack(ShipRef::Player));
        assert_eq!(
            session.scan_latches,
            Latches {
                junk: true,
                outfits: false
            }
        );
        let catalog = contraband_catalog(-10);
        let mut session = smuggler(&catalog, 0, 1, 50_000);
        looked_over(&mut session, &catalog, 0);
        assert_eq!(
            session.take_messages(),
            [illegal_outfit(true, 0)],
            "jle @0x7f222"
        );
        assert_eq!(session.pilot().cash(), 50_000);
        assert!(!session.take_save_due(), "no fine, no save");
        assert_eq!(session.take_sounds(), [SimSound::Smuggling]);
    }

    #[test]
    fn by_the_other_reading_of_outfit_scan_fine_a_negative_fine_takes_its_percent() {
        let catalog = contraband_catalog(-10);
        let mut session = smuggler(&catalog, 0, 1, 50_000);
        session.rules =
            Rulebook::default().with_override(RuleKey::OutfitScanFine, RuleSource::Bible);
        looked_over(&mut session, &catalog, 0);
        assert_eq!(session.take_messages(), [illegal_outfit(true, 5000)]);
        assert_eq!(session.pilot().cash(), 45_000);
    }

    #[test]
    fn scannable_junk_shadows_the_outfits_unless_the_other_reading_scans_past_it() {
        let mut catalog = contraband_catalog(500);
        catalog.junk[0].scan_mask = 0x0800;
        for (source, messages) in [
            (RuleSource::Engine, vec![]),
            (RuleSource::Bible, vec![illegal_outfit(true, 500)]),
        ] {
            let mut session = smuggler(&catalog, 1, 1, 1200);
            session.rules = Rulebook::default().with_override(RuleKey::ScanOutfitsPastJunk, source);
            looked_over(&mut session, &catalog, 0);
            assert_eq!(session.take_messages(), messages, "{source:?}");
        }
    }

    #[test]
    fn a_contraband_conviction_takes_a_0x0040_rank_and_leaves_a_0x0004_one() {
        let mut catalog = contraband_catalog(0);
        catalog.ranks = vec![
            RankRecord {
                affil_govt: Some(GovtId(128)),
                flags: crate::rank::LOST_TO_CRIME,
                ..crate::testkit::rank(130)
            },
            RankRecord {
                affil_govt: Some(GovtId(128)),
                flags: crate::rank::LOST_TO_VIOLENCE,
                ..crate::testkit::rank(131)
            },
        ];
        let mut session = smuggler(&catalog, 1, 0, 1200);
        session.pilot.ranks.extend([RankId(130), RankId(131)]);
        looked_over(&mut session, &catalog, 0);
        assert_eq!(session.take_messages(), [illegal_junk(0)]);
        assert_eq!(
            session.pilot().ranks().iter().copied().collect::<Vec<_>>(),
            [RankId(131)],
            "0x0040 on any crime (@0x9c81); 0x0004 only on types 1 and 3"
        );
    }

    #[test]
    fn a_scan_by_a_person_names_it_by_the_persons_name() {
        let mut catalog = contraband_catalog(0);
        let mut slots: [(Option<crate::catalog::PersonId>, i16); 8] = Default::default();
        slots[0] = (Some(crate::catalog::PersonId(600)), 100);
        catalog.traffic = vec![(
            SystemId(130),
            crate::catalog::SystemTraffic {
                dude_types: [(-1, 0); 8],
                avg_ships: 0,
                persons: slots,
            },
        )];
        catalog.persons = vec![crate::catalog::PersonRecord {
            name: "Ace".to_owned(),
            govt: Some(GovtId(128)),
            ai_type: 4,
            ..crate::testkit::person(600, 129)
        }];
        catalog.ship_records = vec![crate::testkit::ship(129, crate::testkit::FAST)];
        let mut session = loaded(&catalog, &[]);
        in_flight(&mut session, &catalog);
        session.tick_traffic(&catalog, &LookedOver, &mut NeverFires);
        session.pilot.cargo.insert(Good::Junk(BIO_WEAPONS), 1);
        let at = session.player.position;
        let ace = session
            .traffic
            .npcs_mut()
            .iter_mut()
            .find(|npc| npc.person.is_some())
            .expect("Ace is here");
        ace.goal = Goal::Inspect(ShipRef::Player);
        ace.state.position = at;
        let _ = session.take_messages();
        looked_over(&mut session, &catalog, 0);
        assert_eq!(
            session.take_messages(),
            [SimMessage::IllegalJunk {
                scanner: ShipId(129),
                person: Some("Ace".to_owned()),
                junk: BIO_WEAPONS,
                fine: 0
            }]
        );
    }

    #[test]
    fn a_scanner_that_is_no_warship_or_interceptor_scans_nothing() {
        let catalog = contraband_catalog(500);
        for ai_type in [AiType::WimpyTrader, AiType::BraveTrader] {
            let mut session = smuggler(&catalog, 1, 0, 1200);
            police(&mut session).ai_type = ai_type;
            assert!(looked_over(&mut session, &catalog, 0).is_empty(), "no draw");
            assert_eq!(session.take_messages(), [], "{ai_type:?}");
        }
        let mut session = smuggler(&catalog, 1, 0, 1200);
        police(&mut session).ai_type = AiType::Warship;
        looked_over(&mut session, &catalog, 0);
        assert_eq!(session.take_messages(), [illegal_junk(500)]);
    }

    #[test]
    fn a_scanner_gone_out_of_reach_by_the_combat_tick_scans_nothing() {
        let catalog = contraband_catalog(500);
        let mut session = smuggler(&catalog, 1, 0, 1200);
        let mut chance = Draws::of(&[0]);
        session.tick_traffic(&catalog, &LookedOver, &mut chance);
        police(&mut session).state.position.x += 101.0;
        session.tick_combat(crate::Rules::default(), &mut chance);
        assert!(chance.asked.is_empty());
        assert_eq!(session.take_messages(), []);
    }

    /// `_ScanPlayer`'s jump-timing check holds a scan back once the warp
    /// has begun (@0x7e562-0x7e633): here `tick_combat`'s return while
    /// jumping does, so a scan queued as the jump begins comes to
    /// nothing, while one through the pre-jump braking convicts.
    #[test]
    fn a_scan_queued_as_the_jump_begins_is_held_and_one_before_it_convicts() {
        use crate::testkit::begin_jump_now;
        let catalog = contraband_catalog(500);
        // Far out, still moving, so J brakes before the jump.
        let queued = |session: &mut Session, chance: &mut Draws| {
            session.plot_course(SystemId(131)).expect("a route");
            session.player.position = Vec2::new(0.0, -5000.0);
            session.player.velocity = Vec2::new(4.0, 0.0);
            inspect_again(session);
            session.tick_traffic(&catalog, &LookedOver, chance);
        };
        let mut session = smuggler(&catalog, 1, 0, 1200);
        let mut chance = Draws::of(&[0]);
        queued(&mut session, &mut chance);
        begin_jump_now(&mut session).expect("jumps");
        assert!(session.jumping().is_some());
        let at = session.player.position;
        police(&mut session).state.position = at;
        session.tick_combat(crate::Rules::default(), &mut chance);
        assert!(chance.asked.is_empty(), "held: no draw");
        assert_eq!(session.take_messages(), []);
        assert_eq!(session.pilot().cash(), 1200);
        assert_eq!(session.scan_latches, Latches::ARMED);
        let mut session = smuggler(&catalog, 1, 0, 1200);
        let mut chance = Draws::of(&[0]);
        queued(&mut session, &mut chance);
        session.begin_jump().expect("accepted");
        assert!(session.preparing_jump().is_some(), "braking");
        let at = session.player.position;
        police(&mut session).state.position = at;
        session.tick_combat(crate::Rules::default(), &mut chance);
        assert_eq!(chance.asked, [100]);
        assert_eq!(session.take_messages(), [illegal_junk(500)]);
    }

    // The mission branch's sounds and attack.

    #[test]
    fn a_shown_scan_failure_beeps_151_and_a_hidden_one_is_silent_and_both_attack() {
        for (flags, sounds) in [
            (0x0020, vec![SimSound::ScanFailedMission]),
            (0x0420, vec![]),
        ] {
            let catalog = policed(vec![masked(140, 0x0004, flags)], 500);
            let mut session = inspected(&catalog, &[140], Some(GovtId(128)));
            let _ = session.take_sounds();
            looked_over(&mut session, &catalog, 0);
            assert_eq!(session.take_sounds(), sounds, "{flags:#x}");
            assert_eq!(police(&mut session).goal, Goal::Attack(ShipRef::Player));
            assert_eq!(police(&mut session).target, Some(ShipRef::Player));
        }
    }

    #[test]
    fn a_mission_fine_beeps_154_and_attacks_with_no_fine_too() {
        for fine in [500, 0] {
            let catalog = policed(vec![masked(140, 0x0004, 0)], fine);
            let mut session = inspected(&catalog, &[140], Some(GovtId(128)));
            session.pilot.cash = 1200;
            let _ = session.take_sounds();
            let _ = session.take_save_due();
            looked_over(&mut session, &catalog, 0);
            assert_eq!(session.take_sounds(), [SimSound::Smuggling], "{fine}");
            assert_eq!(police(&mut session).goal, Goal::Attack(ShipRef::Player));
            assert_eq!(session.take_save_due(), fine != 0);
        }
    }

    #[test]
    fn a_scan_fining_a_mission_and_finding_junk_convicts_both_against_the_cash_left() {
        let mut catalog = contraband_catalog(-10);
        catalog.missions = vec![masked(140, 0x0004, 0)];
        let mut session = inspected(&catalog, &[140], Some(GovtId(128)));
        session.pilot.cargo.insert(Good::Junk(BIO_WEAPONS), 1);
        session.pilot.cash = 50_000;
        let _ = session.take_sounds();
        looked_over(&mut session, &catalog, 0);
        assert_eq!(
            session.take_messages(),
            [
                SimMessage::Smuggling {
                    scanner: ShipId(129),
                    ship_name: None,
                    fine: 5000
                },
                illegal_junk(4500)
            ],
            "10% of 50,000, then of 45,000"
        );
        assert_eq!(session.pilot().cash(), 40_500);
        assert_eq!(
            session.take_sounds(),
            [SimSound::Smuggling, SimSound::Smuggling]
        );
    }
}
