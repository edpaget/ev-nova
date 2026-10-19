//! What a capture reads of the fleet and leaves behind: the escorts the
//! original counts (`_CanHireEscorts` @0x5795-0x57e4, `_SetPlunderValues`
//! @0x92653-0x926c9), the fuel "Use As My Ship" draws (`_DoShipCapture`
//! @0x4177b-0x417bd), where the player's old ship comes in, and what
//! becomes of a person captured. All addresses are in `EV Nova.app`'s
//! `EV Nova` (i386).
//!
//! **The escorts counted** ([`Session::led_escorts`]). The original counts
//! the ships in system slots 1-63 that are active, led by the player
//! (+0x9a == 0), of AI type 6 and of no mission mark (+0xc8d2 == -1):
//! a launched fighter is AI type 5 (`_LaunchFighter` @0x3d771) and a
//! carried one has no slot, so neither counts, and a mission escort is
//! marked. The odds count only those of `InherentAI` above 2 (see
//! [`NovaBoarding::capture_odds`](crate::board::NovaBoarding)).
//! `_CanHireEscorts` is true while it counts 5 or fewer
//! ([`Session::can_hire_escorts`]), for a capture, a hire in the bar and
//! a person's join by hail alike. Nothing else caps the fleet: the pilot
//! file keeps one escort word per ship slot (`_WritePilotData`
//! @0x72653-0x72711), so a fleet can pass six when some of it is not in
//! the system. Ours counts each escort of the fleet whose NPC is in the
//! system, unmarked, and each the fleet has not yet placed since it last
//! entered a system or was flown (a loaded pilot's, docked, which the
//! original spawns at load, `_LoadPilotData` @0x759a4-0x75b2b); one whose
//! placement found no NPC (no record of its ship type, no berth) is not
//! there to count.
//!
//! **The fuel drawn** ([`CaptureFuel`]). The player's fuel is
//! `Rand(_ShipFuelCapacity(player))`, none for a capacity of none or less
//! (@0x4177b-0x417bd), read after the change of class (@0x416dd) and
//! before the outfits change (@0x41849-0x41a3e). `_ShipFuelCapacity`
//! gives the player's cache (`*0x3b508c`) while it holds one
//! (@0x2f55-0x2f6e), and otherwise the current class's `Fuel` plus each
//! owned outfit's `ModType` 12 × count, held to 0-32000, and caches it.
//! A reset invalidates it (@0xc397); the flight tick refreshes it after
//! the fuel's regeneration (`_HandlePlayer` @0x6d9cd), and so does an
//! Energy take (@0x940a5, @0x94100). So with no reset since then the
//! draw is below the old ship's capacity, and after one (a non-empty
//! `OnRetire` or `OnCapture`, a grant at the dialog's open, an ammunition
//! take) below the captured class's `Fuel` and the old outfits' `ModType`
//! 12.
//!
//! **The old ship** ([`Session::place_old_ship`]) is a fresh spawn of
//! the player's class (`_GenericRandomShipSpawn` @0x4114d) at the
//! player's position, velocity and heading before the swap
//! (@0x411bd-0x411ea), holding the player's six commodities
//! (@0x412e2-0x412fe, as [`RuleKey::CaptureCargo`] reads), its fuel its
//! class's bare `Fuel` (`_ShipFuelCapacity` of an NPC, @0x414a7), its
//! death timer none (@0x4146a). It flies into formation
//! (`_AIMakeEscortFlyInForm` @0x4156a), not put on its slot. The spawn
//! never writes its shield or armour (@0x3c89f-0x3cd3a), so the original
//! leaves whatever the slot's previous ship had, and `_IsDisabled`'s
//! armour branch reads the same (@0x414c3-0x4153b): uninitialised memory,
//! which ours can't model. Ours gives it its stock reserves, full.
//!
//! **A person captured** ([`RuleKey::CapturedPerson`]). Use As My Ship
//! loses the person for good (`_DoShipCapture` @0x41180). Use As Escort
//! (@0x9459b-0x947cf, `_AIShipCapturedByPlayer` @0x89741) clears only the
//! ship's person slot (+0xc8d0): by the engine reading the person stays
//! alive, keeps its grudge and can appear again; by the other it is gone,
//! as Use As My Ship has it.

use super::Session;
use crate::board::MAX_ESCORTS;
use crate::catalog::{ShipId, ShipRecord, outfit_mods};
use crate::chance::Chance;
use crate::flight::ShipState;
use crate::market::Good;
use crate::pilot::Escort;
use crate::rulebook::{RuleKey, RuleSource};
use crate::stats::MORE_FUEL;
use crate::stats::ShipStats;
use crate::traffic::npc::{NPC_COMMODITIES, NpcHold, NpcId};

/// The most fuel `_ShipFuelCapacity` gives (@0x2fce-0x2fde).
pub(super) const FUEL_CAPACITY_MOST: i64 = 32_000;

/// The two capacities "Use As My Ship" can draw its fuel below, read
/// before the swap (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct CaptureFuel {
    /// The old ship's capacity, the cache's while it holds one.
    pub(super) old: i64,
    /// The captured class's `Fuel` and the old outfits' `ModType` 12,
    /// held to 0-32000: what a cache invalidated is worked out afresh as.
    pub(super) captured: i64,
}

impl CaptureFuel {
    /// The capacity the draw reads: the old one while the cache holds it
    /// (`cached`), and the one worked out afresh otherwise.
    pub(super) fn cap(self, cached: bool) -> i64 {
        if cached { self.old } else { self.captured }
    }

    /// The fuel drawn below the capacity the cache gives, on `chance`:
    /// none, with no draw, for a capacity of none or less (@0x4177b-0x417bd).
    pub(super) fn draw(self, cached: bool, chance: &mut dyn Chance) -> f32 {
        u32::try_from(self.cap(cached))
            .ok()
            .filter(|&cap| cap > 0)
            .map_or(0.0, |cap| chance.below(cap) as f32)
    }
}

impl Session {
    /// The ship class of each escort of the fleet the original counts as
    /// led by the player (see the module docs): not a fighter out, and
    /// with its NPC in the system and unmarked, or not placed yet since
    /// the fleet last entered a system.
    pub(super) fn led_escorts(&self) -> Vec<ShipId> {
        self.pilot
            .escorts
            .iter()
            .enumerate()
            .filter(|(_, escort)| !escort.carried)
            .filter_map(|(index, escort)| match self.fleet.get(index) {
                None => Some(escort.ship),
                Some(placed) => placed
                    .and_then(|id| self.npc(id))
                    .filter(|npc| npc.mission.is_none())
                    .map(|npc| npc.ship),
            })
            .collect()
    }

    /// Whether the fleet has room for one more escort, as
    /// `_CanHireEscorts` (@0x5795-0x57e4) counts it: fewer than
    /// [`MAX_ESCORTS`] led by the player ([`Session::led_escorts`]). A
    /// capture, a hire in the bar and a person's join by hail all read it.
    #[must_use]
    pub fn can_hire_escorts(&self) -> bool {
        self.led_escorts().len() < MAX_ESCORTS
    }

    /// The capacities "Use As My Ship" taking a ship of `record` can draw
    /// its fuel below, read before the swap (see the module docs).
    pub(super) fn capture_fuel(&self, record: &ShipRecord) -> CaptureFuel {
        let more: i64 = outfit_mods(&self.pilot.outfits, &self.outfits)
            .into_iter()
            .filter(|outfit| outfit.mod_type == MORE_FUEL)
            .map(|outfit| i64::from(outfit.mod_val) * i64::from(outfit.count))
            .sum();
        CaptureFuel {
            old: self.stats.fuel as i64,
            captured: (i64::from(record.fields.fuel) + more).clamp(0, FUEL_CAPACITY_MOST),
        }
    }

    /// The player's old ship, escort `index`, comes into the system after
    /// "Use As My Ship" (see the module docs): where the player was
    /// (`at`), holding `hold`, keeping formation by flying to its slot, its
    /// skill rolled on `chance` as `_DoShipCapture`'s spawn rolls it
    /// (@0x4114d, through `_GenericRandomShipSpawn`). Gives its NPC; none
    /// when its ship type has no record or no berth is free.
    pub(super) fn place_old_ship(
        &mut self,
        index: usize,
        at: ShipState,
        hold: NpcHold,
        chance: &mut dyn Chance,
    ) -> Option<NpcId> {
        let id = self.place_escort(index, chance)?;
        if let Some(npc) = self.npc_mut(id) {
            npc.state = at;
            npc.cargo = hold;
        }
        Some(id)
    }

    /// The record of the player's old ship as it joins the fleet on "Use
    /// As My Ship": of the player's class, stock and full as an NPC's
    /// stats are ([`ShipStats::for_npc`], by the engine its class's
    /// alone), but for its fuel, its class's bare `Fuel`, as an NPC's
    /// capacity is (`_ShipFuelCapacity` @0x2f47, @0x414a7; see the module
    /// docs).
    pub(super) fn old_ship_escort(&self) -> Escort {
        let stock = ShipStats::for_npc(
            self.fields,
            &outfit_mods(&self.defaults, &self.outfits),
            self.npc_items(),
        );
        let mut reserves = stock.full();
        reserves.fuel.now = f32::from(self.fields.fuel.max(0));
        Escort {
            ship: self.pilot.ship,
            reserves,
            order: None,
            carried: false,
            wage: None,
            person: None,
        }
    }

    /// The player's commodities 0-5 leave for the old ship's hold on "Use
    /// As My Ship", by the engine reading of [`RuleKey::CaptureCargo`]
    /// (@0x412e2-0x412fe), the `jünk` and the mission cargo staying: the
    /// hold they make, empty by the other reading.
    pub(super) fn commodities_for_old_ship(&mut self) -> NpcHold {
        let mut hold = NpcHold::default();
        if self.rules.source_for(RuleKey::CaptureCargo) == RuleSource::Engine {
            self.pilot.cargo.retain(|good, tons| match good {
                Good::Commodity(commodity) if usize::from(*commodity) < NPC_COMMODITIES => {
                    hold.add(*commodity, *tons);
                    false
                }
                _ => true,
            });
        }
        hold
    }

    /// The player holds the captured ship's commodities, `hold`, in place
    /// of its own, by the engine reading of [`RuleKey::CaptureCargo`]
    /// (@0x41760-0x41779); by the other the captured hold is lost.
    pub(super) fn take_captured_commodities(&mut self, hold: &NpcHold) {
        if self.rules.source_for(RuleKey::CaptureCargo) == RuleSource::Engine {
            for (good, tons) in hold.held() {
                self.pilot.cargo.insert(good, tons);
            }
        }
    }

    /// Whether a person captured as an escort is gone for good, as
    /// [`RuleKey::CapturedPerson`] reads: by the engine it lives on.
    pub(super) fn escort_capture_loses_person(&self) -> bool {
        self.rules.source_for(RuleKey::CapturedPerson) == RuleSource::Bible
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::board::{Assigned, Assignment, Boarding, PlunderView, Take, Taken};
    use crate::catalog::OutfitId;
    use crate::legal::NovaLaw;
    use crate::reserves::Reserves;
    use crate::session::tests::{ON_BOARD, aboard, alongside, board_by, boardable, take};
    use crate::testkit::{Draws, FakePilotCatalog};
    use crate::traffic::npc::MissionMark;

    /// An escort of ship `ship`, a fighter out when `carried`.
    fn escort(ship: i16, carried: bool) -> Escort {
        Escort {
            ship: ShipId(ship),
            reserves: Reserves::full(30.0, 45.0, 300.0),
            order: None,
            carried,
            wage: None,
            person: None,
        }
    }

    /// `session` with `escorts` on file, each placed in the system unless
    /// `unplaced` names its index.
    fn with_fleet(session: &mut Session, escorts: Vec<Escort>, unplaced: &[usize]) {
        session.pilot.escorts = escorts;
        session.fleet = (0..session.pilot.escorts.len())
            .map(|index| {
                if unplaced.contains(&index) {
                    None
                } else {
                    session.place_escort(index, &mut crate::chance::NeverFires)
                }
            })
            .collect();
    }

    /// [`boardable`] with interceptor 130 of crew 30 and trader 131.
    fn interceptors() -> FakePilotCatalog {
        let mut catalog = boardable();
        for record in &mut catalog.ship_records {
            if record.id == ShipId(130) {
                record.crew = 30;
                record.inherent_ai = 4;
            }
        }
        catalog
    }

    /// The odds boarding opens with.
    fn odds(session: &mut Session) -> u8 {
        match board_by(session, &NovaLaw::default()) {
            Ok(Boarding::Opened(view)) => view.odds,
            opened => panic!("{opened:?}"),
        }
    }

    // The escorts counted.

    #[test]
    fn the_escorts_counted_are_those_in_the_system_and_those_not_placed_yet() {
        let catalog = interceptors();
        let mut session = alongside(&catalog);
        session.pilot.escorts = vec![escort(130, false), escort(130, true), escort(131, false)];
        assert_eq!(
            session.led_escorts(),
            [ShipId(130), ShipId(131)],
            "not placed yet, the fighter out left out"
        );
        with_fleet(
            &mut session,
            vec![escort(130, false), escort(130, true), escort(131, false)],
            &[],
        );
        assert_eq!(session.led_escorts(), [ShipId(130), ShipId(131)], "placed");
        with_fleet(
            &mut session,
            vec![escort(130, false), escort(131, false)],
            &[0],
        );
        assert_eq!(session.led_escorts(), [ShipId(131)], "no NPC in the system");
        let id = session.fleet[1].expect("placed");
        session.npc_mut(id).expect("there").mission = Some(MissionMark {
            slot: 0,
            behav: crate::traffic::ShipBehav::Own,
            formation: None,
            order: None,
        });
        assert_eq!(session.led_escorts(), [], "a mission's mark");
    }

    #[test]
    fn the_odds_count_a_placed_warship_escort_and_the_room_any_placed_escort() {
        let catalog = interceptors();
        let mut session = alongside(&catalog);
        with_fleet(&mut session, vec![escort(130, false)], &[]);
        assert_eq!(odds(&mut session), 53, "13 crew against 3, and 10");
        let mut session = alongside(&catalog);
        with_fleet(&mut session, vec![escort(130, false)], &[0]);
        assert_eq!(odds(&mut session), ON_BOARD.odds, "not in the system");
        let mut session = alongside(&catalog);
        with_fleet(&mut session, vec![escort(130, true)], &[]);
        assert_eq!(odds(&mut session), ON_BOARD.odds, "a fighter out");
        let mut session = alongside(&catalog);
        with_fleet(&mut session, vec![escort(131, false); MAX_ESCORTS], &[]);
        assert_eq!(odds(&mut session), 0, "six traders fill the fleet");
        let mut session = alongside(&catalog);
        with_fleet(&mut session, vec![escort(131, false); MAX_ESCORTS - 1], &[]);
        assert_eq!(odds(&mut session), ON_BOARD.odds, "a trader adds no crew");
    }

    #[test]
    fn the_fleet_full_check_counts_the_escorts_in_the_system() {
        let catalog = boardable();
        let mut session = aboard(&catalog);
        with_fleet(&mut session, vec![escort(130, false); MAX_ESCORTS], &[]);
        assert!(!session.can_hire_escorts());
        assert_eq!(
            take(&mut session, Take::Capture, &[43, 1]).0,
            Taken::FleetFull
        );
        let mut session = aboard(&catalog);
        with_fleet(&mut session, vec![escort(130, false); MAX_ESCORTS], &[5]);
        assert!(
            session.can_hire_escorts(),
            "one of the six not in the system"
        );
        assert_eq!(
            take(&mut session, Take::Capture, &[43, 1]).0,
            Taken::Captured
        );
        assert_eq!(
            session.assign(Assignment::Escort, &mut Draws::of(&[])),
            Some(Assigned::Escort)
        );
        assert_eq!(session.pilot().escorts().len(), 7, "past six");
        assert!(!session.can_hire_escorts(), "six in the system now");
        let saved = crate::save::decode(&crate::save::encode(session.pilot())).expect("reads");
        assert_eq!(saved.escorts().len(), 7, "the save keeps every escort");
    }

    #[test]
    fn a_full_fleet_gives_no_odds_by_the_same_count() {
        let catalog = boardable();
        let mut session = alongside(&catalog);
        with_fleet(&mut session, vec![escort(130, false); MAX_ESCORTS], &[]);
        assert!(matches!(
            board_by(&mut session, &NovaLaw::default()),
            Ok(Boarding::Opened(PlunderView { odds: 0, .. }))
        ));
    }

    // The fuel.

    #[test]
    fn the_fuel_is_drawn_below_the_cap_the_cache_gives() {
        let fuel = CaptureFuel {
            old: 300,
            captured: 450,
        };
        assert_eq!(fuel.cap(true), 300);
        assert_eq!(fuel.cap(false), 450);
        let mut chance = Draws::of(&[123]);
        assert_eq!(fuel.draw(true, &mut chance), 123.0);
        assert_eq!(chance.asked, [300]);
        let mut chance = Draws::of(&[]);
        let none = CaptureFuel {
            old: 0,
            captured: -5,
        };
        assert_eq!(none.draw(true, &mut chance), 0.0);
        assert_eq!(none.draw(false, &mut chance), 0.0);
        assert!(chance.asked.is_empty(), "no draw for no tank");
    }

    #[test]
    fn the_captured_capacity_is_the_captured_fuel_and_the_old_outfits_held_to_32000() {
        let mut catalog = boardable();
        catalog
            .outfits
            .push(crate::testkit::outfit(330, &[(MORE_FUEL, 100)]));
        let mut session = alongside(&catalog);
        session.pilot.outfits.insert(OutfitId(330), 2);
        let record = session
            .ship_record(ShipId(129))
            .cloned()
            .expect("the trader");
        assert_eq!(
            session.capture_fuel(&record).captured,
            i64::from(record.fields.fuel) + 200
        );
        session.pilot.outfits.insert(OutfitId(330), 400);
        assert_eq!(session.capture_fuel(&record).captured, FUEL_CAPACITY_MOST);
        let mut empty = record.clone();
        empty.fields.fuel = -500;
        session.pilot.outfits.insert(OutfitId(330), 2);
        assert_eq!(session.capture_fuel(&empty).captured, 0, "held at none");
    }

    #[test]
    fn the_flight_tick_and_an_energy_take_cache_the_capacity_and_a_reset_clears_it() {
        let catalog = boardable();
        let mut session = alongside(&catalog);
        assert!(!session.fuel_cap_cached);
        session.tick(crate::flight::Controls::default());
        assert!(session.fuel_cap_cached, "the flight tick");
        session.reset_precalced();
        assert!(!session.fuel_cap_cached, "a reset");
        let mut session = aboard(&catalog);
        session.reset_precalced();
        take(&mut session, Take::Energy, &[]);
        assert!(session.fuel_cap_cached, "an energy take");
    }

    /// [`boardable`] with a fuel tank (`oütf` 330, 50 more fuel) and the
    /// trader (129) of `Fuel` 200, its `OnCapture` `on_capture`: the old
    /// ship with the tank holds 350, and the captured class with it 250.
    fn tanked(on_capture: &str) -> FakePilotCatalog {
        let mut catalog = boardable();
        catalog
            .outfits
            .push(crate::testkit::outfit(330, &[(MORE_FUEL, 50)]));
        for record in &mut catalog.ship_records {
            if record.id == ShipId(129) {
                record.fields.fuel = 200;
                record.on_capture = crate::control::Script::parse(on_capture);
            }
        }
        catalog
    }

    #[test]
    fn use_as_my_ship_draws_below_the_old_capacity_until_a_reset_and_then_below_the_captured() {
        use crate::rulebook::Rulebook;
        for order in [RuleSource::Engine, RuleSource::Bible] {
            for (why, on_capture, cached, bound) in [
                ("cached", "", true, 350),
                ("reset since", "", false, 250),
                ("OnCapture resets", "b9", true, 250),
            ] {
                let catalog = tanked(on_capture);
                let rules = Rulebook::default().with_override(RuleKey::CaptureHookOrder, order);
                let mut session = alongside(&catalog).with_rules(rules);
                session.pilot.outfits.insert(OutfitId(330), 1);
                session.refit(true, session.script_reserves());
                board_by(&mut session, &NovaLaw::default()).expect("boards");
                assert_eq!(
                    take(&mut session, Take::Capture, &[43, 1]).0,
                    Taken::Captured
                );
                session.fuel_cap_cached = cached;
                let mut chance = Draws::of(&[7]);
                assert_eq!(
                    session.assign(Assignment::MyShip, &mut chance),
                    Some(Assigned::MyShip)
                );
                assert_eq!(chance.asked, [bound], "{why}, {order:?}");
                assert_eq!(session.reserves().fuel.now, 7.0, "{why}");
            }
        }
    }

    #[test]
    fn an_ammunition_take_resets_so_the_draw_is_below_the_captured_capacity() {
        let mut catalog = crate::session::tests::ammo_aboard(1, 20);
        catalog
            .outfits
            .push(crate::testkit::outfit(330, &[(MORE_FUEL, 50)]));
        let mut session = crate::session::tests::ammo_session(&catalog);
        session.pilot.outfits.insert(OutfitId(330), 1);
        session.fuel_cap_cached = true;
        take(&mut session, Take::Ammo, &[]);
        assert!(!session.fuel_cap_cached);
        assert_eq!(
            take(&mut session, Take::Capture, &[99, 43, 1]).0,
            Taken::Captured
        );
        let mut chance = Draws::of(&[7]);
        session.assign(Assignment::MyShip, &mut chance);
        assert_eq!(chance.asked, [350], "the trader's 300 and the tank's 50");
    }

    #[test]
    fn an_ammunition_take_of_nothing_still_resets_and_an_energy_take_of_nothing_does_not() {
        use crate::scan::Latches;
        let catalog = boardable();
        // The ammunition button resets (@0x93cc9) before it looks for
        // rounds (@0x93cb8 refuses).
        let mut session = aboard(&catalog);
        session.fuel_cap_cached = true;
        crate::session::precalced::tests::convicted(&mut session);
        assert_eq!(take(&mut session, Take::Ammo, &[]).0, Taken::Missed);
        assert!(!session.fuel_cap_cached, "the ammunition reset");
        assert_eq!(session.scan_latches, Latches::ARMED, "the ammunition reset");
        // The energy refusal (@0x94231) only beeps.
        let mut session = aboard(&catalog);
        take(&mut session, Take::Energy, &[]);
        take(&mut session, Take::Miss, &[99]);
        session.fuel_cap_cached = true;
        crate::session::precalced::tests::convicted(&mut session);
        let latches = session.scan_latches;
        assert_eq!(take(&mut session, Take::Energy, &[]).0, Taken::Missed);
        assert!(session.fuel_cap_cached, "no energy reset");
        assert_eq!(session.scan_latches, latches, "no energy reset");
    }

    // Use As My Ship.

    /// [`captured`] with 4 tons of food and 6 of commodity 3, 5 of `jünk`
    /// and 2 of mission cargo aboard, and the trader holding 7 tons of
    /// commodity 2, by `source`'s reading of [`RuleKey::CaptureCargo`].
    fn swapping(catalog: &FakePilotCatalog, source: RuleSource) -> Session {
        use crate::rulebook::Rulebook;
        use crate::session::precalced::tests::JUNK;
        let rules = Rulebook::default().with_override(RuleKey::CaptureCargo, source);
        let mut session = crate::session::tests::captured(catalog).with_rules(rules);
        session.pilot.cargo = [
            (crate::market::Good::Commodity(0), 4),
            (crate::market::Good::Commodity(3), 6),
            (JUNK, 5),
        ]
        .into_iter()
        .collect();
        session
            .pilot
            .missions
            .put(0, crate::testkit::carrying(140, 2));
        session.traffic.npcs_mut()[0].cargo =
            NpcHold::of(Some((crate::market::Good::Commodity(2), 7)));
        session
    }

    #[test]
    fn by_the_engine_the_commodities_swap_with_the_captured_ships_hold() {
        use crate::market::Good;
        use crate::session::precalced::tests::JUNK;
        let catalog = boardable();
        let mut session = swapping(&catalog, RuleSource::Engine);
        assert_eq!(
            session.assign(Assignment::MyShip, &mut Draws::of(&[0])),
            Some(Assigned::MyShip)
        );
        assert_eq!(
            session.pilot().cargo().collect::<Vec<_>>(),
            [(Good::Commodity(2), 7), (JUNK, 5)],
            "the captured ship's in place of the player's, the jünk kept"
        );
        assert_eq!(
            session.pilot().missions().get(0).map(|m| m.cargo_tons),
            Some(2)
        );
        let old = session.fleet[0]
            .and_then(|id| session.npc(id))
            .expect("placed");
        assert_eq!(
            old.cargo.held().collect::<Vec<_>>(),
            [(Good::Commodity(0), 4), (Good::Commodity(3), 6)],
            "the player's six words copied to the old ship (@0x412e2-0x412fe)"
        );
        let mut session = swapping(&catalog, RuleSource::Engine);
        session.pilot.cargo.insert(Good::Commodity(6), 3);
        session.assign(Assignment::MyShip, &mut Draws::of(&[0]));
        assert_eq!(
            session.pilot().held(Good::Commodity(6)),
            3,
            "past the six words the ships hold, a good stays with the player"
        );
        let mut session = swapping(&catalog, RuleSource::Bible);
        session.assign(Assignment::MyShip, &mut Draws::of(&[0]));
        assert_eq!(
            session.pilot().cargo().collect::<Vec<_>>(),
            [(Good::Commodity(0), 4), (Good::Commodity(3), 6), (JUNK, 5)],
            "the other reading keeps them, and the captured hold is lost"
        );
        let old = session.fleet[0]
            .and_then(|id| session.npc(id))
            .expect("placed");
        assert_eq!(old.cargo, NpcHold::default());
    }

    #[test]
    fn the_old_ships_reserves_are_its_classs_alone_by_the_engine() {
        // The player's class carries armour plating (+50) and a tank (+50)
        // by default.
        let mut catalog = boardable();
        catalog.outfits.extend([
            crate::testkit::outfit(330, &[(MORE_FUEL, 50)]),
            crate::testkit::outfit(331, &[(crate::stats::MORE_ARMOR, 50)]),
        ]);
        catalog.defaults = vec![(ShipId(128), vec![(OutfitId(330), 1), (OutfitId(331), 1)])];
        for ship in &mut catalog.ship_records {
            if ship.id == ShipId(128) {
                ship.defaults = vec![(OutfitId(330), 1), (OutfitId(331), 1)];
            }
        }
        for (items, more) in [(RuleSource::Engine, 0.0), (RuleSource::Bible, 50.0)] {
            let session = alongside(&catalog).with_rules(
                crate::rulebook::Rulebook::default().with_override(RuleKey::NpcDefaultItems, items),
            );
            let armor = f32::from(session.fields.armor) + more;
            let fuel = f32::from(session.fields.fuel) + more;
            assert_eq!(session.fields.fuel, 300);
            let record = session.old_ship_escort();
            assert_eq!(
                (record.reserves.armor.max, record.reserves.fuel.max),
                (armor, fuel),
                "{items:?}"
            );
            assert_eq!(record.reserves.armor.now, armor, "full");
            assert_eq!(record.reserves.fuel.now, 300.0, "its class's bare Fuel");
        }
    }

    #[test]
    fn the_old_ship_rolls_its_skill_of_no_government() {
        let mut catalog = boardable();
        for ship in &mut catalog.ship_records {
            if ship.id == ShipId(128) {
                ship.skill_var = 37;
            }
        }
        let mut session = alongside(&catalog);
        board_by(&mut session, &NovaLaw::default()).expect("boards");
        assert_eq!(
            take(&mut session, Take::Capture, &[43, 1]).0,
            Taken::Captured
        );
        let mut chance = Draws::of(&[0]).keyed(75, 74);
        assert_eq!(
            session.assign(Assignment::MyShip, &mut chance),
            Some(Assigned::MyShip)
        );
        assert_eq!(chance.times_asked(75), 1);
        let id = session.fleet[0].expect("placed");
        let old = session.npc(id).expect("in the system");
        assert_eq!(
            (old.skill, old.skill_mult),
            ((f64::from(74 - 37 + 100) * 0.01) as f32, 1.0)
        );
    }

    #[test]
    fn the_old_ship_comes_in_where_the_player_was_with_its_bare_fuel_and_flies_to_its_slot() {
        use crate::ai::NovaAi;
        use crate::chance::NeverFires;
        use crate::geometry::Vec2;
        let mut catalog = boardable();
        catalog
            .outfits
            .push(crate::testkit::outfit(330, &[(MORE_FUEL, 50)]));
        catalog.defaults = vec![(ShipId(128), vec![(OutfitId(330), 1)])];
        let mut session = alongside(&catalog);
        session.player.velocity = Vec2::new(0.25, -0.125);
        session.player.heading = 90.0;
        session.traffic.npcs_mut()[0].state.velocity = Vec2::new(0.25, -0.125);
        session.traffic.npcs_mut()[0].state.heading = 90.0;
        board_by(&mut session, &NovaLaw::default()).expect("boards");
        assert_eq!(
            take(&mut session, Take::Capture, &[43, 1]).0,
            Taken::Captured
        );
        session.traffic.npcs_mut()[0].state.position = Vec2::new(100.0, 40.0);
        let left = session.player;
        assert_eq!(
            session.assign(Assignment::MyShip, &mut Draws::of(&[0])),
            Some(Assigned::MyShip)
        );
        let id = session.fleet[0].expect("placed");
        let old = session.npc(id).expect("in the system");
        assert_eq!(old.state, left, "the player's place, velocity and heading");
        assert_eq!(
            (old.reserves.fuel.now, old.reserves.fuel.max),
            (300.0, 300.0),
            "its class's bare Fuel (@0x414a7), its tank left out of both by the engine"
        );
        assert_eq!(
            session.pilot().escorts()[0].reserves.fuel.now,
            300.0,
            "its record lined up"
        );
        let slot = |session: &Session| {
            let old = session.npc(id).expect("in the system");
            let duty = old.escort.expect("an escort");
            crate::escort::slot_position(
                &session.player,
                duty.formation.slot,
                duty.formation.ships,
                duty.formation.spacing,
            )
        };
        let gap = |session: &Session| {
            (session.npc(id).expect("there").state.position - slot(session)).length()
        };
        let before = gap(&session);
        assert!(before > 50.0, "not on its slot: {before}");
        for _ in 0..60 {
            session.tick_traffic(&catalog, &NovaAi::default(), &mut NeverFires);
        }
        assert!(
            gap(&session) < before,
            "closing on its slot: {} from {before}",
            gap(&session)
        );
    }

    #[test]
    fn use_as_my_ship_lets_go_of_the_nav_selection_and_the_target() {
        use crate::catalog::SystemId;
        use crate::navigation::NavMode;
        let catalog = boardable();
        for (nav, after) in [
            (
                NavMode::Hyperspace(Some(SystemId(129))),
                NavMode::Hyperspace(None),
            ),
            (NavMode::Off, NavMode::Off),
        ] {
            let mut session = crate::session::tests::captured(&catalog);
            session.set_nav(nav);
            session.assign(Assignment::MyShip, &mut Draws::of(&[0]));
            assert_eq!(session.nav, after, "the mode kept (@0x4172e)");
            assert_eq!(session.target(), None, "@0x41736");
        }
    }
}
