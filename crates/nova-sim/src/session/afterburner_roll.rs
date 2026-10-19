//! The afterburner roll (the original's +0xbd) of the ships the session
//! itself brings into the system: a fighter launched (see
//! [`fighters`](super::fighters)) and the player's escorts.
//!
//! - **Every roll** asks `_AIHasAfterburner` (@0x64f5-0x6594, see
//!   [`afterburner`]): none, unrolled, at a berth another ship still names
//!   as its swarm leader ([`swarm::named`], @0x64ff-0x6539), else by its
//!   class's flags and the player's rating over the yardstick
//!   [`RuleKey::AfterburnerYardstick`] says.
//! - **The player's escorts** roll it as `_SpawnEscort` does (@0x3ff51,
//!   stored @0x3ff56): a ship hired in the bar (`_DoShipyardDialog`
//!   @0x5f1f1), and each escort of a pilot loaded (`_LoadPilotData`
//!   @0x75a25 and @0x75b2b), against the loaded pilot's rating, as the
//!   pilot file never keeps the flag; here a loaded pilot's escorts roll as
//!   the first system is set up, having no NPC yet. "Use As My Ship"
//!   (`_DoShipCapture` @0x4123e-0x41243) rolls it for the old ship kept as
//!   an escort. "Use As Escort" (`_AIMakeEscortFlyInForm` @0x89b8e) and a
//!   person joining after a hail never write it: the ship keeps the flag
//!   its own spawn rolled ([`Session::enlist`]).
//! - **Through a jump, a landing and a take-off** each escort keeps its
//!   flag, as `_RespawnEscort` (@0x3d4b8-0x3d62c) never writes +0xbd and
//!   `_DeleteAllComputerShips(0)` keeps the player's slots whole
//!   (@0x3bf7e-0x3bfb6); the session carries it across the setup (see
//!   the `escorts` module's `KeptEscort`).
//! - No roll plays a sound.

use super::Session;
use crate::ai::{afterburner, swarm};
use crate::chance::Chance;
use crate::combat::hull::HullSpec;
use crate::rulebook::RuleKey;
use crate::traffic::npc::NpcId;

impl Session {
    /// Whether a ship of `hull` the session brings in at `berth` has an
    /// afterburner, rolled on `chance` (see the module docs): none,
    /// unrolled, at a berth another ship names as its swarm leader.
    pub(super) fn rolls_afterburner(
        &self,
        hull: &HullSpec,
        berth: u32,
        chance: &mut dyn Chance,
    ) -> bool {
        if swarm::named(berth, self.traffic.npcs()) {
            return false;
        }
        let side = self.player_side();
        // A class's `Strength` is none below none, so in 16 bits.
        #[allow(clippy::cast_possible_truncation)]
        let own = hull.strength as i16;
        let yardstick = afterburner::yardstick(
            self.rules.source_for(RuleKey::AfterburnerYardstick),
            side.yardstick,
            own,
        );
        afterburner::has_afterburner(hull, side.rating, yardstick, chance)
    }

    /// Rolls the afterburner of escort NPC `id` on `chance`, as
    /// `_SpawnEscort` and `_DoShipCapture` do (see the module docs).
    pub(super) fn roll_escort_afterburner(&mut self, id: NpcId, chance: &mut dyn Chance) {
        let Some((hull, berth)) = self.npc(id).map(|npc| (npc.hull, npc.berth)) else {
            return;
        };
        let has = self.rolls_afterburner(&hull, berth, chance);
        if let Some(npc) = self.npc_mut(id) {
            npc.afterburner = has;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{boardable, captured};
    use super::*;
    use crate::ai::Peaceful;
    use crate::ai::afterburner::RATING_ROLL;
    use crate::catalog::{HullRecord, ShipId, ShipRecord, SystemId};
    use crate::combat::hull::{AFTERBURNER_ALWAYS, AFTERBURNER_BY_RATING};
    use crate::pilot::{Escort, Pilot};
    use crate::rulebook::{RuleSource, Rulebook};
    use crate::session::{Assigned, Assignment};
    use crate::testkit::{
        FAST, FakePilotCatalog, Ranged, bar, catalog, hull, jump_with, land_now, planet, ship,
    };

    /// The ship for hire.
    const HIRED: ShipId = ShipId(129);
    /// A roll of `Rand(1344)` a rating of 1000 over a yardstick of 1
    /// reaches: 0 + 256.
    const PASSES: (u32, u32) = (RATING_ROLL as u32, 0);
    /// One it doesn't: 1000 + 256.
    const FAILS: (u32, u32) = (RATING_ROLL as u32, 1000);

    /// [`catalog`] with planet 128 a bar, ship 129 of `flags` for hire
    /// every day, and the player's ship 128 of `Strength` 1, the
    /// engine's yardstick.
    fn barred(flags: u16) -> FakePilotCatalog {
        FakePilotCatalog {
            sites: vec![
                (SystemId(130), vec![bar(), planet(129, 2000.0, 0.0)]),
                (SystemId(131), vec![planet(140, 0.0, 0.0)]),
            ],
            ship_records: vec![
                ship(128, FAST),
                ShipRecord {
                    hire_random: 100,
                    ..ship(129, FAST)
                },
            ],
            hulls: vec![
                HullRecord {
                    strength: 1,
                    ..hull(128)
                },
                HullRecord { flags, ..hull(129) },
            ],
            ..catalog()
        }
    }

    /// `catalog`'s session landed at the bar, rich, of combat rating 1000.
    fn landed(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.pilot.cash = 100_000_000;
        session.pilot.combat_rating = 1000;
        land_now(&mut session).expect("lands");
        session
    }

    /// The last escort of the fleet's NPC, if it is in the system.
    fn last_escort(session: &Session) -> Option<&crate::traffic::npc::Npc> {
        let id = session.fleet.last().copied().flatten()?;
        session.npc(id)
    }

    /// Hires ship 129 rolling on `chance`: its NPC's afterburner.
    fn hired(session: &mut Session, chance: &mut Ranged) -> Option<bool> {
        session.hire(HIRED, chance).expect("hires");
        last_escort(session).map(|npc| npc.afterburner)
    }

    #[test]
    fn a_hired_escort_of_a_class_of_flags_0x0040_has_an_afterburner() {
        let catalog = barred(AFTERBURNER_ALWAYS);
        let mut session = landed(&catalog);
        let mut chance = Ranged::of(&[FAILS]);
        assert_eq!(hired(&mut session, &mut chance), Some(true));
        assert!(!chance.was_asked(u32::from(RATING_ROLL)), "unrolled");
    }

    #[test]
    fn a_hired_escort_of_flags_0x0020_rolls_against_the_rating() {
        let catalog = barred(AFTERBURNER_BY_RATING);
        let mut session = landed(&catalog);
        let mut chance = Ranged::of(&[PASSES]);
        assert_eq!(hired(&mut session, &mut chance), Some(true));
        assert!(chance.was_asked(u32::from(RATING_ROLL)));
        let mut session = landed(&catalog);
        assert_eq!(hired(&mut session, &mut Ranged::of(&[FAILS])), Some(false));
    }

    #[test]
    fn a_hired_escorts_roll_is_measured_by_the_yardstick_the_rulebook_reads() {
        // Ship 129's class is of `Strength` 2, the player's 1: a rating of
        // 1000 is 1000 over the engine's yardstick, 500 over the Bible's.
        // `Rand(1344)` of 300 needs 556, which only the engine's reaches.
        let mut catalog = barred(AFTERBURNER_BY_RATING);
        let own = catalog
            .hulls
            .iter_mut()
            .find(|record| record.id == HIRED)
            .expect("the hired class");
        own.strength = 2;
        for (source, has) in [(RuleSource::Engine, true), (RuleSource::Bible, false)] {
            let mut session = landed(&catalog).with_rules(
                Rulebook::default().with_override(RuleKey::AfterburnerYardstick, source),
            );
            let mut chance = Ranged::of(&[(u32::from(RATING_ROLL), 300)]);
            assert_eq!(hired(&mut session, &mut chance), Some(has), "{source:?}");
        }
    }

    #[test]
    fn a_hired_escort_of_neither_flag_has_none() {
        let catalog = barred(0);
        let mut session = landed(&catalog);
        let mut chance = Ranged::of(&[PASSES]);
        assert_eq!(hired(&mut session, &mut chance), Some(false));
        assert!(!chance.was_asked(u32::from(RATING_ROLL)));
    }

    /// `catalog`'s session flown from a pilot of combat rating 1000 whose
    /// fleet holds ship 129, its first system set up on `chance`.
    fn loaded(catalog: &FakePilotCatalog, chance: &mut Ranged) -> Session {
        let mut pilot = Pilot::new(catalog, "Ada").expect("starts");
        let reserves = crate::traffic::table::kind(
            catalog
                .ship_records
                .iter()
                .find(|r| r.id == HIRED)
                .expect("129"),
            &[],
            &crate::combat::armament::Arsenal::read(catalog),
        )
        .stats
        .full();
        pilot.escorts = vec![Escort {
            ship: HIRED,
            reserves,
            order: None,
            carried: false,
            wage: None,
            person: None,
        }];
        let mut session = Session::fly(catalog, pilot).expect("flies");
        session.pilot.combat_rating = 1000;
        session.tick_traffic(catalog, &Peaceful, chance);
        session
    }

    #[test]
    fn a_loaded_pilots_escorts_roll_their_afterburner_as_the_system_is_set_up() {
        // `_LoadPilotData` spawns each escort with `_SpawnEscort`
        // (@0x75a25), which rolls +0xbd: the pilot file never keeps it.
        let session = loaded(&barred(AFTERBURNER_ALWAYS), &mut Ranged::of(&[]));
        assert_eq!(last_escort(&session).map(|npc| npc.afterburner), Some(true));
        let session = loaded(&barred(AFTERBURNER_BY_RATING), &mut Ranged::of(&[FAILS]));
        assert_eq!(
            last_escort(&session).map(|npc| npc.afterburner),
            Some(false)
        );
        let session = loaded(&barred(AFTERBURNER_BY_RATING), &mut Ranged::of(&[PASSES]));
        assert_eq!(last_escort(&session).map(|npc| npc.afterburner), Some(true));
    }

    #[test]
    fn an_escorts_afterburner_is_kept_through_a_jump() {
        // `_RespawnEscort` never writes +0xbd (@0x3d4b8-0x3d62c).
        let catalog = barred(AFTERBURNER_BY_RATING);
        for (hire, again, has) in [(PASSES, FAILS, true), (FAILS, PASSES, false)] {
            let mut session = landed(&catalog);
            assert_eq!(hired(&mut session, &mut Ranged::of(&[hire])), Some(has));
            session
                .take_off(&catalog, &mut Ranged::of(&[again]))
                .expect("takes off");
            assert_eq!(
                last_escort(&session).map(|npc| npc.afterburner),
                Some(has),
                "taking off"
            );
            let mut chance = Ranged::of(&[again]);
            assert_eq!(
                jump_with(&mut session, &catalog, 131, &mut chance),
                Some(SystemId(131))
            );
            assert_eq!(
                last_escort(&session).map(|npc| npc.afterburner),
                Some(has),
                "after the jump"
            );
            assert!(!chance.was_asked(u32::from(RATING_ROLL)), "no roll");
        }
    }

    /// [`boardable`] with the player's class, 128, of `Flags` 0x0040.
    fn boardable_burning() -> FakePilotCatalog {
        let mut catalog = boardable();
        let own = catalog
            .hulls
            .iter_mut()
            .find(|record| record.id == ShipId(128))
            .expect("the player's class");
        own.flags |= AFTERBURNER_ALWAYS;
        catalog
    }

    #[test]
    fn the_old_ship_kept_on_use_as_my_ship_rolls_its_afterburner() {
        // `_DoShipCapture` rolls +0xbd for the old ship it keeps
        // (@0x4123e-0x41243).
        let catalog = boardable_burning();
        let mut session = captured(&catalog);
        assert_eq!(
            session.assign(Assignment::MyShip, &mut Ranged::of(&[])),
            Some(Assigned::MyShip)
        );
        let old = last_escort(&session).expect("the old ship kept");
        assert_eq!(old.ship, ShipId(128));
        assert!(old.afterburner);
    }

    #[test]
    fn use_as_escort_keeps_the_captured_ships_own_afterburner() {
        // `_AIMakeEscortFlyInForm` (@0x89b8e) never writes +0xbd.
        let catalog = boardable_burning();
        for own in [true, false] {
            let mut session = captured(&catalog);
            for npc in session.traffic.npcs_mut() {
                npc.afterburner = own;
            }
            let mut chance = Ranged::of(&[]);
            assert_eq!(
                session.assign(Assignment::Escort, &mut chance),
                Some(Assigned::Escort)
            );
            assert_eq!(last_escort(&session).map(|npc| npc.afterburner), Some(own));
            assert!(!chance.was_asked(u32::from(RATING_ROLL)));
        }
    }

    #[test]
    fn an_escort_placed_at_a_berth_a_swarmer_names_has_none_unrolled() {
        // `_AIHasAfterburner`'s first test (@0x64ff-0x6539): the ship at
        // berth 2 names berth 1, which the hire takes.
        let catalog = barred(AFTERBURNER_ALWAYS);
        let mut session = landed(&catalog);
        let mut swarmer = crate::traffic::fixture::npc(0, crate::stats::ShipStats::default());
        swarmer.swarm_leader = Some(1);
        session.traffic.add_npc_at(swarmer, 2);
        let mut chance = Ranged::of(&[PASSES]);
        assert_eq!(hired(&mut session, &mut chance), Some(false));
        assert_eq!(last_escort(&session).map(|npc| npc.berth), Some(1));
        assert!(!chance.was_asked(u32::from(RATING_ROLL)));
    }
}
