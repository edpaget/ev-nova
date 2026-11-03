//! The readers of the outfit counts outside the shipyard, and whether
//! each leaves a purchase's held stock out.
//!
//! The original keeps two stores: the outfit counts (`*0x3b51a8`) and
//! the weapon array (the player's slot +0xc8, rounds +0xd0). A purchase
//! tops the array up to its stock weapons and `AmmoLoad`, and only the
//! next `_ShipStatsToSystemInfo` folds that top-up into the counts (see
//! the `stock_fold` module). In the port both stores are one map,
//! `Pilot::outfits`, which holds the top-up at once; by the engine reading
//! of [`RuleKey::PurchaseStockFold`](crate::rulebook::RuleKey::PurchaseStockFold)
//! `PlayerShip::held_stock` records the part still held in the array
//! alone. A site that models a read of the counts takes
//! `PlayerShip::counted_outfits`, the held part left out; a site that
//! models a read of the array keeps `Pilot::outfits`.

use super::{PlayerShip, Statics};
use crate::catalog::{MARINES, outfit_mods};
use crate::pilot::Pilot;

impl PlayerShip {
    /// Each marines outfit (`ModType` [`MARINES`]) in the outfit counts:
    /// its `ModVal`, and how many. A pirate's plunder of the player
    /// (`_AIPlunderShipContents` @0x841bd-0x84204, @0x84345-0x84388) and
    /// the boarding's odds (`_SetPlunderValues` @0x926cb-0x9271a) read
    /// them, a purchase's held stock left out.
    pub(super) fn marines(&self, pilot: &Pilot, statics: &Statics) -> Vec<(i16, u16)> {
        outfit_mods(&self.counted_outfits(pilot), &statics.outfits)
            .into_iter()
            .filter(|outfit| outfit.mod_type == MARINES)
            .map(|outfit| (outfit.mod_val, outfit.count))
            .collect()
    }
}

// The stats compare exact sums of whole mod values.
#[allow(clippy::float_cmp)]
#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::collections::BTreeMap;

    use crate::catalog::{CronId, CronRecord, OutfitId, ShipId, ShipRecord};
    use crate::chance::NeverFires;
    use crate::control::{PilotFacts, Test};
    use crate::cron::CronRefusal;
    use crate::landing::StellarFlags;
    use crate::rulebook::Rulebook;
    use crate::rules::Verdict;
    use crate::session::Session;
    use crate::session::tests::{GUN, LAUNCHER, ROCKETS, TANK};
    use crate::shipyard::ShipFlags3;
    use crate::testkit::{FAST, FakePilotCatalog, cron, land_now, ship};

    use super::super::stock_fold::fixture::{at_once, bought, stocked};

    /// The three answers every reader is pinned by: (a) by the engine,
    /// after the purchase; (b) by the engine, after `fold` runs; (c) by
    /// the other reading, after the purchase.
    fn readings<T>(
        catalog: &FakePilotCatalog,
        setup: impl Fn(&mut Session),
        fold: impl Fn(&mut Session),
        read: impl Fn(&mut Session) -> T,
    ) -> [T; 3] {
        let mut held = bought(catalog, Rulebook::default());
        setup(&mut held);
        let a = read(&mut held);
        fold(&mut held);
        let b = read(&mut held);
        let mut at_once = bought(catalog, at_once());
        setup(&mut at_once);
        let c = read(&mut at_once);
        [a, b, c]
    }

    /// The outfitter's fold (@0x5bb57), for the readers of a stay.
    fn outfitter(session: &mut Session) {
        session.open_outfitter();
        session.close_outfitter();
    }

    /// Nothing between the purchase and the read.
    fn stay(_: &mut Session) {}

    /// The take-off, for the readers in flight: the held part survives
    /// it, as no in-flight code folds.
    fn take_off(catalog: &FakePilotCatalog) -> impl Fn(&mut Session) + '_ {
        |session| {
            session.take_off(catalog, &mut NeverFires);
        }
    }

    /// A landing's fold (@0x63211), for the readers in flight.
    fn land(session: &mut Session) {
        land_now(session).expect("lands");
    }

    /// A landing's fold and a take-off again, for the readers in flight.
    fn relaunch(catalog: &FakePilotCatalog) -> impl Fn(&mut Session) + '_ {
        move |session| {
            land(session);
            take_off(catalog)(session);
        }
    }

    /// `catalog` with `edit` made to outfit `id`'s record.
    fn with_outfit(
        mut catalog: FakePilotCatalog,
        id: OutfitId,
        edit: impl FnOnce(&mut crate::catalog::OutfitRecord),
    ) -> FakePilotCatalog {
        let record = catalog
            .outfits
            .iter_mut()
            .find(|record| record.id == id)
            .expect("listed");
        edit(record);
        catalog
    }

    #[test]
    fn the_held_stock_survives_a_take_off() {
        let catalog = stocked();
        let mut session = bought(&catalog, Rulebook::default());
        let held = session.ship.held_stock.clone();
        take_off(&catalog)(&mut session);
        assert_eq!(session.ship.held_stock, held);
        assert_ne!(held, BTreeMap::new());
    }

    #[test]
    fn the_counted_outfits_leave_the_held_stock_out_by_the_engine() {
        let catalog = stocked();
        let session = bought(&catalog, Rulebook::default());
        let counted = session.ship.counted_outfits(&session.pilot);
        assert!(matches!(counted, Cow::Owned(_)));
        assert_eq!(*counted, BTreeMap::from([(TANK, 1)]));
        let mut session = session;
        session.ship.held_stock = BTreeMap::from([(GUN, 5), (LAUNCHER, 0), (ROCKETS, 19)]);
        assert_eq!(
            *session.ship.counted_outfits(&session.pilot),
            BTreeMap::from([(TANK, 1), (LAUNCHER, 1), (ROCKETS, 1)]),
            "the held part out, saturating at none"
        );
    }

    #[test]
    fn the_counted_outfits_are_the_pilots_own_with_nothing_held() {
        let catalog = stocked();
        let session = bought(&catalog, at_once());
        let counted = session.ship.counted_outfits(&session.pilot);
        assert!(matches!(counted, Cow::Borrowed(_)));
        assert_eq!(*counted, session.pilot.outfits);
        assert_eq!(counted.get(&GUN), Some(&1));
    }

    #[test]
    fn an_o_test_leaves_a_held_outfit_out_until_the_fold() {
        let catalog = stocked();
        assert_eq!(
            readings(&catalog, stay, outfitter, |session| session.has_outfit(GUN)),
            [false, true, true]
        );
    }

    /// A for-hire class whose `Availability` is `o` of the held gun.
    const GUNNED: ShipId = ShipId(131);

    /// [`stocked`] with planet 128 a bar too, hiring out [`GUNNED`] every
    /// day.
    fn hiring() -> FakePilotCatalog {
        let mut catalog = stocked();
        catalog.sites[0].1[0].flags |= StellarFlags::BAR;
        catalog.ship_records.push(ShipRecord {
            availability: Test::parse("o305"),
            hire_random: 100,
            ..ship(GUNNED.0, FAST)
        });
        catalog
    }

    /// Whether the bar lets [`GUNNED`] be hired.
    fn gunned_for_hire(session: &mut Session) -> bool {
        session
            .escorts_for_hire(&mut NeverFires)
            .expect("a bar")
            .row(GUNNED)
            .is_some_and(|row| row.hire.is_ok())
    }

    #[test]
    fn the_hire_screens_availability_leaves_a_held_outfit_out_until_it_closes() {
        let catalog = hiring();
        assert_eq!(
            readings(&catalog, stay, Session::close_hire, gunned_for_hire),
            [false, true, true],
            "_DoBarDialog folds as the hire screen closes (@0x49235)"
        );
    }

    /// The `Contribute` bit [`licensed`] gives the held gun.
    const GUN_BIT: u64 = 0x8;

    /// [`stocked`] with the gun contributing [`GUN_BIT`].
    fn licensed() -> FakePilotCatalog {
        with_outfit(stocked(), GUN, |record| record.contribute = GUN_BIT)
    }

    #[test]
    fn the_contributed_bits_leave_a_held_outfits_out_until_the_fold() {
        let catalog = licensed();
        let contributes = |session: &mut Session| {
            session.ship.contributed(&session.pilot, &session.statics) & GUN_BIT != 0
        };
        assert_eq!(
            readings(&catalog, stay, outfitter, contributes),
            [false, true, true],
            "_GetPlayerContributeBits @0x7730-0x775f reads the counts"
        );
    }

    #[test]
    fn the_hire_screens_require_leaves_a_held_outfits_bits_out_until_it_closes() {
        let mut catalog = licensed();
        catalog.sites[0].1[0].flags |= StellarFlags::BAR;
        catalog.ship_records.push(ShipRecord {
            require: GUN_BIT,
            flags3: ShipFlags3::HIDE_UNLESS_REQUIRED,
            hire_random: 100,
            ..ship(GUNNED.0, FAST)
        });
        let listed = |session: &mut Session| {
            session
                .escorts_for_hire(&mut NeverFires)
                .expect("a bar")
                .row(GUNNED)
                .is_some()
        };
        assert_eq!(
            readings(&catalog, stay, Session::close_hire, listed),
            [false, true, true],
            "_SetupPortAvailableShipTypes @0xbd09, before @0x49235 folds"
        );
    }

    #[test]
    fn a_crons_require_in_flight_leaves_a_held_outfits_bits_out_until_the_landing() {
        let mut catalog = licensed();
        catalog.crons.push(CronRecord {
            require: GUN_BIT,
            ..cron(128)
        });
        let required = |session: &mut Session| {
            session
                .cron_refusals(CronId(128))
                .expect("a crön")
                .iter()
                .any(|(_, verdict)| *verdict == Verdict::Refuses(CronRefusal::Require))
        };
        assert_eq!(
            readings(&catalog, take_off(&catalog), land, required),
            [true, false, false],
            "_ActivateCron @0x38681 on a jump's days"
        );
    }

    // The scan.

    use crate::SimMessage;
    use crate::catalog::{GovtId, GovtRecord, Penalties};
    use crate::legal::NovaLaw;
    use crate::testkit::{Draws, govt, npc};
    use crate::traffic::npc::AiType;

    /// [`stocked`] with the gun masked 0x0004 and the planet's
    /// government, 128, scanning for it without a fine.
    fn smuggling() -> FakePilotCatalog {
        let mut catalog = with_outfit(stocked(), GUN, |record| record.scan_mask = 0x0004);
        catalog.govts = vec![GovtRecord {
            penalties: Penalties {
                smuggle: 5,
                ..Penalties::default()
            },
            scan_mask: 0x0004,
            news_pic: -1,
            ..govt(128)
        }];
        catalog
    }

    /// Whether an interceptor of government 128 beside the player finds
    /// the gun: `Some(one)` when it does.
    fn scan(session: &mut Session) -> Option<bool> {
        let mut police = npc(0, session.stats());
        police.govt = Some(GovtId(128));
        police.ai_type = AiType::Interceptor;
        police.ship = ShipId(129);
        police.state.position = session.ship.player.position;
        let id = session.space.traffic.add_npc(police);
        let _ = session.take_messages();
        session.scanned_by(id, &NovaLaw::default(), &mut Draws::of(&[0]));
        session
            .take_messages()
            .into_iter()
            .find_map(|message| match message {
                SimMessage::IllegalOutfit { outfit, one, .. } if outfit == GUN => Some(one),
                _ => None,
            })
    }

    #[test]
    fn a_scan_in_flight_leaves_a_held_outfit_out_until_the_landing() {
        let catalog = smuggling();
        assert_eq!(
            readings(&catalog, take_off(&catalog), relaunch(&catalog), scan),
            [None, Some(true), Some(true)],
            "_ScanPlayer @0x7efd1-0x7eff6 reads the counts"
        );
    }

    #[test]
    fn a_scan_counts_one_of_an_outfit_owned_once_beside_a_held_one() {
        let catalog = smuggling();
        let owned_one_more = |session: &mut Session| {
            take_off(&catalog)(session);
            *session.pilot.outfits.entry(GUN).or_default() += 1;
        };
        let [engine, _, other] = readings(&catalog, owned_one_more, stay, scan);
        assert_eq!(engine, Some(true), "one in the counts (@0x7f10e)");
        assert_eq!(other, Some(false), "two owned");
    }

    // The marines.

    use crate::catalog::MARINES;

    #[test]
    fn the_marines_leave_a_held_outfits_out_until_the_fold() {
        let catalog = with_outfit(stocked(), LAUNCHER, |record| record.mods[1] = (MARINES, 3));
        let marines =
            |session: &mut Session| session.ship.marines(&session.pilot, &session.statics);
        assert_eq!(
            readings(&catalog, take_off(&catalog), relaunch(&catalog), marines),
            [vec![], vec![(3, 1)], vec![(3, 1)]],
            "_AIPlunderShipContents @0x841bd-0x84204 and _SetPlunderValues \
             @0x926cb-0x9271a read the counts"
        );
    }

    // The stats.

    use crate::stats::{MORE_SHIELD, MORE_SPEED};

    /// [`stocked`] with the held gun given a second mod of `mod_type`.
    fn second_mod(mod_type: i16, mod_val: i16) -> FakePilotCatalog {
        with_outfit(stocked(), GUN, |record| {
            record.mods[1] = (mod_type, mod_val);
        })
    }

    #[test]
    fn the_stats_leave_a_held_outfits_later_mods_out_until_the_fold() {
        let speed = |session: &mut Session| session.stats().handling.max_speed;
        let [plain, ..] = readings(&stocked(), take_off(&stocked()), stay, speed);
        let catalog = second_mod(MORE_SPEED, 300);
        let [held, folded, at_once] =
            readings(&catalog, take_off(&catalog), relaunch(&catalog), speed);
        assert_eq!(held, plain, "_ShipMaxSpeed @0x374a-0x37a1 reads the counts");
        assert!(folded > plain, "{folded} > {plain}");
        assert_eq!(at_once, folded);
        let shield = |session: &mut Session| session.stats().shield;
        let catalog = second_mod(MORE_SHIELD, 50);
        let [held, folded, at_once] = readings(&catalog, stay, outfitter, shield);
        assert_eq!([folded - held, at_once - held], [50.0, 50.0]);
    }

    #[test]
    fn the_purchase_fills_the_shield_to_the_stats_without_the_held_part() {
        let catalog = second_mod(MORE_SHIELD, 50);
        let class = |session: &mut Session| session.reserves().shield;
        let [held, _, at_once] = readings(&catalog, stay, stay, class);
        assert_eq!(held.now, held.max, "full");
        assert_eq!(at_once.now - held.now, 50.0, "class + 50 at once");
        assert_eq!(at_once.max, at_once.now);
    }

    #[test]
    fn the_fold_raises_the_most_of_each_reserve_and_nothing_held() {
        let catalog = second_mod(MORE_SHIELD, 50);
        let mut session = bought(&catalog, Rulebook::default());
        let before = session.reserves().shield;
        outfitter(&mut session);
        let after = session.reserves().shield;
        assert_eq!(after.max - before.max, 50.0, "the new most");
        assert_eq!(after.now, before.now, "no gain: the fold writes no reserve");
    }

    // The capture's fuel.

    #[test]
    fn the_captures_fuel_leaves_a_held_outfits_fuel_out_until_the_fold() {
        use crate::stats::MORE_FUEL;
        let catalog = second_mod(MORE_FUEL, 100);
        let captured = |session: &mut Session| {
            let record = session
                .statics
                .ship_record(ShipId(128))
                .cloned()
                .expect("ship 128");
            session
                .ship
                .capture_fuel(&session.pilot, &session.statics, &record)
                .captured
        };
        let [held, folded, at_once] =
            readings(&catalog, take_off(&catalog), relaunch(&catalog), captured);
        assert_eq!(
            held,
            i64::from(FAST.fuel) + 100,
            "the tank's alone: _ShipFuelCapacity @0x41786 reads the counts"
        );
        assert_eq!([folded, at_once], [held + 100, held + 100]);
    }

    // The gear readers.

    use crate::catalog::GovtId as Govt;
    use crate::cloak::{MOD_CLOAK, MOD_SCANNER, SCAN_SCREEN};
    use crate::combat::hull::Condition;
    use crate::combat::jamming::JAMMING_MOD;
    use crate::escape::{MOD_AUTO_EJECT, MOD_ESCAPE_POD};
    use crate::iff::{MOD_ANTI_INTERFERENCE, MOD_IFF};
    use crate::murk::MOD_MURK;
    use crate::sabotage::{SABOTAGE_BOMB, SABOTAGE_KILL};
    use crate::scoop::MOD_ASTEROID_SCOOP;
    use crate::self_repair::MOD_REPAIR;
    use crate::session::iff_scramblers::IFF_SCRAMBLER;
    use crate::session::reinforcements::{EVERY_GOVERNMENT, INHIBITOR};

    /// A gear reader: its name, the second mod the held launcher is given, and
    /// what the reader answers of a session, read on a copy where the
    /// reading would change it.
    type Gear<'a> = (&'static str, (i16, i16), &'a dyn Fn(&Session) -> String);

    /// A copy of `session` dying, its death timer unset.
    fn dying(session: &Session) -> Session {
        let mut probe = session.clone();
        probe.ship.condition = Condition::Dying {
            ticks_left: 0,
            starting: true,
        };
        probe
    }

    #[test]
    fn the_gear_readers_leave_a_held_outfits_later_mods_out_until_the_landing() {
        let class = 7;
        let gears: [Gear; 15] = [
            ("_HasCloak", (MOD_CLOAK, 0), &|session| {
                format!(
                    "{:?}",
                    session.ship.cloak_gear(&session.pilot, &session.statics)
                )
            }),
            (
                "_ShipCanSeeCloakedShips",
                (MOD_SCANNER, SCAN_SCREEN),
                &|session| format!("{:?}", session.scanner()),
            ),
            ("_ShipHasIFF", (MOD_IFF, 0), &|session| {
                format!("{:?}", session.radar_gear())
            }),
            (
                "_ShipAntiInterference",
                (MOD_ANTI_INTERFERENCE, -20),
                &|session| session.interference().to_string(),
            ),
            ("_PlayerEffectiveMurk", (MOD_MURK, 15), &|session| {
                session.effective_murk().to_string()
            }),
            ("the IFF scrambler", (IFF_SCRAMBLER, class), &|session| {
                session.space.iff_marked.contains(&Govt(128)).to_string()
            }),
            ("the inhibitor", (INHIBITOR, EVERY_GOVERNMENT), &|session| {
                session.space.reinforcements.every_government.to_string()
            }),
            ("_ShipCanSelfRepair", (MOD_REPAIR, 0), &|session| {
                let mut probe = session.clone();
                probe.ship.condition = Condition::Disabled;
                probe.player_repair_tick(crate::Rules::default(), &mut Draws::of(&[0]));
                format!("{:?}", probe.ship.condition)
            }),
            ("_ShipCanScoop", (MOD_ASTEROID_SCOOP, 0), &|session| {
                session
                    .ship
                    .player_scoops(&session.pilot, &session.statics)
                    .to_string()
            }),
            ("_HasEscapePod", (MOD_ESCAPE_POD, 0), &|session| {
                let mut probe = dying(session);
                let hulk = probe.ship.condition;
                probe.try_eject(true, hulk, &mut NeverFires).to_string()
            }),
            ("_HasAutoEject", (MOD_AUTO_EJECT, 0), &|session| {
                let probe = dying(session);
                probe
                    .ship
                    .auto_eject_fires(&probe.pilot, &probe.statics)
                    .to_string()
            }),
            ("the sabotage flag", (SABOTAGE_BOMB, 1), &|session| {
                format!("{:?}", session.ship.latches.sabotage.flag())
            }),
            ("the bombs", (SABOTAGE_BOMB, 1), &|session| {
                let mut probe = session.clone();
                let _ = probe.take_messages();
                probe.sabotage_bombs(&crate::NovaDisable, &mut NeverFires);
                format!("{:?}", probe.take_messages())
            }),
            ("the kill outfit", (SABOTAGE_KILL, 5000), &|session| {
                let mut probe = session.clone();
                let _ = probe.take_messages();
                probe.sabotage_kill();
                let descs = probe
                    .ship
                    .sabotage_death_descs(&probe.pilot, &probe.statics);
                format!("{:?} {descs:?}", probe.take_messages())
            }),
            ("_ShipECM", (JAMMING_MOD, 40), &|session| {
                format!("{:?}", session.player_jamming())
            }),
        ];
        for (name, second, read) in gears {
            let edit = |mut catalog: FakePilotCatalog| {
                catalog.govts.push(GovtRecord {
                    classes: [class, -1, -1, -1],
                    ..govt(128)
                });
                catalog
            };
            let read = |session: &mut Session| read(session);
            let catalog = edit(stocked());
            let [plain, ..] = readings(&catalog, take_off(&catalog), stay, read);
            // The launcher, as ship 128 stocks none: an IFF scrambler or an
            // inhibitor the start's guns held would stay marked.
            let catalog = edit(with_outfit(stocked(), LAUNCHER, |record| {
                record.mods[1] = second;
            }));
            let [held, folded, at_once] =
                readings(&catalog, take_off(&catalog), relaunch(&catalog), read);
            assert_eq!(held, plain, "{name}: held out");
            assert_ne!(folded, plain, "{name}: folded");
            assert_eq!(at_once, folded, "{name}: at once");
        }
    }

    #[test]
    fn a_bomb_going_off_leaves_its_held_part_in_the_weapon_array() {
        let catalog = with_outfit(stocked(), LAUNCHER, |record| {
            record.mods[1] = (SABOTAGE_BOMB, 1);
        });
        let left = |session: &mut Session| {
            *session.pilot.outfits.entry(LAUNCHER).or_default() += 1;
            session.sabotage_bombs(&crate::NovaDisable, &mut NeverFires);
            session.pilot.owned(LAUNCHER)
        };
        let [engine, _, other] = readings(&catalog, take_off(&catalog), stay, left);
        assert_eq!(
            engine, 1,
            "the counts' one zeroed, the held one kept (@0x696b8)"
        );
        assert_eq!(other, 0);
    }

    // The plunder dialog: its grant pick and its folds.

    use crate::board::{NovaBoarding, Take};
    use crate::session::tests::{
        ACE_DRAWS, aboard, alongside_ace, board_drawing, boardable, granting, take,
    };

    #[test]
    fn the_grant_pick_leaves_a_held_outfit_out_of_its_count_and_max() {
        let catalog = with_outfit(stocked(), GUN, |record| record.max = 1);
        let pick = |session: &mut Session| {
            let record = (session.statics.outfits.iter())
                .find(|record| record.id == GUN)
                .cloned()
                .expect("the gun");
            let stock =
                session
                    .ship
                    .grant_stock(&session.pilot, &session.statics, &session.setup, &record);
            (stock.owned, stock.at_max)
        };
        assert_eq!(
            readings(&catalog, stay, outfitter, pick),
            [(0, false), (1, true), (1, true)],
            "_HasMaxOfItem @0x93119, @0x93192 reads the counts"
        );
    }

    /// The booster [`granting`] grants, held to a `Max` of 1.
    const BOOSTER: OutfitId = OutfitId(200);

    /// [`alongside_ace`] in [`granting`] with the booster's `Max` 1, the
    /// player owning one, and `held` of it held.
    fn boarding_with(held: BTreeMap<OutfitId, u16>) -> Session {
        let catalog = with_outfit(granting(), BOOSTER, |record| record.max = 1);
        let mut session = alongside_ace(&catalog);
        session.pilot.outfits.insert(BOOSTER, 1);
        session.pilot.outfits.insert(TANK, 1);
        session.ship.held_stock = held;
        session
    }

    #[test]
    fn a_grant_picks_a_held_outfit_and_folds_once_it_has_picked() {
        let mut session = boarding_with(BTreeMap::from([(BOOSTER, 1)]));
        let (boarded, _) = board_drawing(&mut session, NovaBoarding::default(), &ACE_DRAWS);
        assert!(boarded.is_ok(), "{boarded:?}");
        assert_eq!(
            session.take_grant().map(|granted| granted.outfit),
            Some(BOOSTER),
            "not at its Max in the counts (@0x93119)"
        );
        assert_eq!(
            session.ship.held_stock,
            BTreeMap::new(),
            "folded (@0x931d9)"
        );
    }

    #[test]
    fn a_grant_that_picks_nothing_leaves_the_stock_held() {
        let held = BTreeMap::from([(TANK, 1)]);
        let mut session = boarding_with(held.clone());
        let (boarded, _) = board_drawing(&mut session, NovaBoarding::default(), &ACE_DRAWS);
        assert!(boarded.is_ok(), "{boarded:?}");
        assert_eq!(session.take_grant(), None, "the booster at its Max");
        assert_eq!(session.ship.held_stock, held, "no pick, no fold (@0x93441)");
    }

    #[test]
    fn the_ammo_press_folds_the_stock() {
        let catalog = boardable();
        let pressed = |press: Take| {
            let mut session = aboard(&catalog);
            session.pilot.outfits.insert(TANK, 1);
            session.ship.held_stock = BTreeMap::from([(TANK, 1)]);
            let _ = take(&mut session, press, &[0; 8]);
            session.ship.held_stock.is_empty()
        };
        assert!(pressed(Take::Ammo), "_DoPlunderDialog @0x93cc9");
        assert!(!pressed(Take::Credits), "no fold of its own");
    }
}
