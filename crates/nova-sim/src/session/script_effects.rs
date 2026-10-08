//! The set operators that move the player: `M` and `N` move it to
//! another system, the session's side of `_EvalSetExp`'s moves.
//!
//! **The original.** `_EvalSetExp` (@0x150fc) dispatches on the letter
//! through the table at 0xdd2fc (@0x15cc2):
//!
//! | Operator | Where | What |
//! |---|---|---|
//! | `M` | @0x1570e-0x158d4 | the player's system is set (ship +0x74) and its stellar cleared (+0x6c = -1); the first of the new system's nav slots that is not -1 is found, with no check of landability or gates. Landed, that stellar becomes +0x6c and nothing else changes; in flight, the ship is put at its x/y at rest. With no stellar, nothing more: in flight the position and velocity are kept |
//! | `N` | @0x158d9-0x159ad | as `M`, but the position and velocity are never touched, and `_dontMovePlayerAfterLanding` (0x316951) is set |
//! | both | @0x15807-0x1583e | every active ship whose parent is the player (+0x9a 0: its escorts and fighters out) takes the new system; the others keep the old one, and `_DoPlayGameWork` runs only the ships in the player's system, so they stay behind |
//! | both, landed | | `_dockedPortMissions` is filled with -1, and that is all |
//! | both, in flight | @0x158a4 | the shots, cargo boxes, explosions, smoke and asteroids are killed, the stellar sprites rebuilt, and `_MissionHandlePlayerEnteredNewSystem` runs |
//!
//! Neither calls `_SetupShipsInSystem` or `_AutoSetExploration`, whose
//! only callers are a new pilot, the main screen, the take-off and
//! `_HandlePlayer`'s death (@0x69114) and hyperspace arrival (@0x6c1f9)
//! paths, so in flight the new system is neither explored nor populated
//! at once. Neither touches `_hyperRoute`, and `_CueNextHyperRouteDest`
//! (@0xe2dc) targets the route's next system only when it is linked from
//! the current one, so a stale route cannot be jumped along until it is
//! replotted.
//!
//! The take-off (`_PlayerLandOnStellar` @0x633ad-0x63400) zeroes the
//! velocity, then: with `_dontMovePlayerAfterLanding` set, clears it and
//! keeps the position, which the landing never moved; otherwise puts the
//! ship at +0x6c's x/y, or at the centre with none. It explores the
//! system (`_AutoSetExploration` @0x634c8), and the spaceport sets it up
//! afterwards (`_SetupShipsInSystem`). A landed move never touches
//! `_dockedPort`, which `_DoPortDialog` stores as it opens (@0x5f93b) and
//! its sub-dialogs read, so the open spaceport serves the stellar it
//! opened on until it is left.
//!
//! **Here.** [`MoveToOp`] (`Mxxx`) and [`MoveKeepPositionOp`] (`Nxxx`)
//! only queue their move: a set expression runs in the middle of a
//! purchase, a capture or a hook, with no catalog to read the new system
//! from. [`Session::settle_script`] applies the moves in order at a safe
//! point, which whoever runs set expressions calls after each input and
//! tick, before saving; a move to a system that does not exist changes
//! nothing (the original indexes it blindly). Each move makes a save
//! due. In flight a move gives up a jump being prepared.
//!
//! In flight, the ship is in the new system: its stellars read, the
//! navigation target, a gate's pending entry, the shots and beams, the
//! strikes, the player's target, the boarding and the hail let go. `M`
//! puts it at rest on the system's first stellar, whatever that is (a
//! gate too); in a system with no stellar it stays where it is, moving
//! as it was, unless [`ScriptEffectRules::starless`] puts it at rest at
//! the centre. `N` keeps the position and velocity.
//!
//! Landed, only where the pilot is changes: its system, and by `M` its
//! stellar, with the ship at rest at the first stellar's position (at the
//! centre with none); `N` leaves the pilot on no stellar, the ship where
//! it is. The open spaceport keeps serving the stellar landed on, as
//! [`Session::market`] and the rest read it, until the take-off swaps the
//! new system's stellars in. There is no other reading of that: an `N`
//! leaves no stellar to rebuild the spaceport for. By `M` the pilot's
//! stellar is the first one only when it can be landed on and is not a
//! hypergate or wormhole, otherwise none: the pilot's stellar is what a
//! save keeps as the stellar last landed on, through the flight after
//! the take-off too, and [`Session::fly`] docks a reloaded pilot there,
//! so the original's choice would dock it on a gate or a stellar that
//! cannot be landed on, which nothing else in the engine allows (landing
//! on a gate enters it). Such a pilot reloads in flight at the centre;
//! one saved after a landed `N` does too.
//!
//! Every take-off now explores the system, as the original's does, which
//! changes nothing in a system explored already. It keeps the ship where
//! it touched down, instead of the stellar's centre, after an `N`: by
//! [`ScriptEffectRules::keep_flag`]'s engine reading after any `N` since
//! the last take-off, landed or in flight, as the original's flag stays
//! set; by the other, only after a landed one. The flag is not saved.
//!
//! What else a move does follows [`ScriptEffectRules::arrival`]. By the
//! engine, nothing: the course is kept, and J refuses while its next
//! system is not linked from the new one
//! ([`JumpRefusal::NoDestination`](crate::JumpRefusal)); the system is
//! explored by the next take-off; and in flight, in another system, only
//! the player's escorts and fighters out follow, as they are, the new
//! system's ships coming only with its arrivals over time (see
//! [`Traffic::retarget`](crate::Traffic::retarget)). By the other
//! reading the move is an arrival: the system is explored and the course
//! cleared at once, and in flight the fighters out are kept or abandoned
//! as the fighter rules say and the system is populated
//! ([`Session::populate`]).
//!
//! Not yet as the original: the mission work its moves do
//! (`_dockedPortMissions`, `_MissionHandlePlayerEnteredNewSystem`) waits
//! for missions, and the first stellar is the first of
//! [`PilotCatalog::landing_sites`] until stellars can be destroyed.

use std::collections::BTreeSet;

use super::Session;
use crate::catalog::{LandingSite, PilotCatalog, StellarId, SystemId, TrafficCatalog};
use crate::chance::Chance;
use crate::control::{SetOp, SetOpHandler};
use crate::gate::GateKind;
use crate::geometry::Vec2;
use crate::landing::is_landable;
use crate::rulebook::{RuleKey, RuleSource, Rulebook};

/// The disputed rules of the moving set operators that the session
/// follows (see [`Session::with_script_effect_rules`]): the engine's by
/// default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScriptEffectRules {
    /// Where `M` puts a ship in flight in a system with no stellar
    /// ([`RuleKey::MoveStarless`]).
    pub starless: RuleSource,
    /// Whether a move is an arrival ([`RuleKey::MoveArrival`]).
    pub arrival: RuleSource,
    /// Whether an `N` in flight keeps the next take-off's landing
    /// position ([`RuleKey::MoveKeepFlag`]).
    pub keep_flag: RuleSource,
}

impl ScriptEffectRules {
    /// The rules `rulebook` chooses: its [`RuleKey::MoveStarless`],
    /// [`RuleKey::MoveArrival`] and [`RuleKey::MoveKeepFlag`] entries.
    #[must_use]
    pub fn from_rulebook(rulebook: &Rulebook) -> Self {
        Self {
            starless: rulebook.source_for(RuleKey::MoveStarless),
            arrival: rulebook.source_for(RuleKey::MoveArrival),
            keep_flag: rulebook.source_for(RuleKey::MoveKeepFlag),
        }
    }
}

/// A move a set expression queued, for [`Session::settle_script`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ScriptMove {
    /// `M`: to the system, at its first stellar.
    To(SystemId),
    /// `N`: to the system, keeping the position.
    KeepPosition(SystemId),
}

/// What [`Session::settle_script`] did.
#[must_use]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Settled {
    /// The system the last move applied went to, if any did.
    pub moved: Option<SystemId>,
}

impl Session {
    /// This session with the moving set operators following `rules` where
    /// the Bible and the engine disagree: the engine's by default.
    #[must_use]
    pub fn with_script_effect_rules(mut self, rules: ScriptEffectRules) -> Self {
        self.script_effect_rules = rules;
        self
    }

    /// The rules the moving set operators follow.
    #[must_use]
    pub fn script_effect_rules(&self) -> ScriptEffectRules {
        self.script_effect_rules
    }

    /// Applies what the set expressions run since the last settling
    /// queued, in order (see the module docs): the moves, each read from
    /// `catalog`, populating on `chance` where the rules say. Whoever runs
    /// set expressions calls it after each input and tick, before saving.
    pub fn settle_script(
        &mut self,
        catalog: &(impl PilotCatalog + TrafficCatalog + ?Sized),
        chance: &mut (impl Chance + ?Sized),
    ) -> Settled {
        let mut settled = Settled::default();
        for step in std::mem::take(&mut self.script_moves) {
            let (system, keep) = match step {
                ScriptMove::To(system) => (system, false),
                ScriptMove::KeepPosition(system) => (system, true),
            };
            if catalog.system_exists(system) {
                self.move_to(system, keep, catalog, chance);
                settled.moved = Some(system);
            }
        }
        settled
    }

    /// Moves the player to `system`, keeping the position when `keep`
    /// (`N`) or else at its first stellar (`M`), read from `catalog` (see
    /// the module docs).
    fn move_to(
        &mut self,
        system: SystemId,
        keep: bool,
        catalog: &(impl PilotCatalog + TrafficCatalog + ?Sized),
        chance: &mut (impl Chance + ?Sized),
    ) {
        let sites = catalog.landing_sites(system);
        let first = sites.first().map(|site| (site.position, dockable(site)));
        let rules = self.script_effect_rules;
        let landed = self.landed.is_some();
        if keep {
            self.hold_position |= landed || rules.keep_flag == RuleSource::Engine;
        }
        let changed = system != self.pilot.system;
        self.pilot.system = system;
        self.save_due = true;
        if landed {
            self.pilot.stellar = None;
            if !keep {
                self.player.position = first.map_or(Vec2::ZERO, |(position, _)| position);
                self.player.velocity = Vec2::ZERO;
                self.pilot.stellar = first.and_then(|(_, dock)| dock);
            }
            self.next_sites = Some(sites);
            if rules.arrival == RuleSource::Bible {
                self.pilot.explore(system);
                self.pilot.course.clear();
            }
            return;
        }
        if !keep {
            match first {
                Some((position, _)) => self.put_at_rest(position),
                None if rules.starless == RuleSource::Bible => self.put_at_rest(Vec2::ZERO),
                None => {}
            }
        }
        if changed {
            self.pilot.stellar = None;
        }
        self.sites = sites;
        self.jump = None;
        self.stop_thrust();
        self.leave_scene();
        if rules.arrival == RuleSource::Bible {
            self.pilot.explore(system);
            self.pilot.course.clear();
            self.leave_with_fighters(false);
            self.populate(catalog, chance);
        } else if changed {
            let table = self.spawn_table(catalog);
            let fleet: BTreeSet<_> = self.fleet.iter().flatten().copied().collect();
            self.traffic.retarget(table, &fleet);
        }
    }

    /// Puts the ship at rest at `position`.
    fn put_at_rest(&mut self, position: Vec2) {
        self.player.position = position;
        self.player.velocity = Vec2::ZERO;
    }

    /// Lets go of what belongs to the scene left: the navigation target,
    /// a gate's pending entry, the shots and beams, the strikes, the
    /// player's target, the boarding and the hail.
    pub(super) fn leave_scene(&mut self) {
        self.nav_target = None;
        self.gate = None;
        self.combat.clear();
        self.strikes.clear();
        self.target = None;
        self.aboard = None;
        self.talk = None;
    }
}

/// `site`'s stellar, if a pilot may be docked there: one that can be
/// landed on and is no hypergate or wormhole.
fn dockable(site: &LandingSite) -> Option<StellarId> {
    (GateKind::of(site.flags2).is_none() && is_landable(site)).then_some(site.id)
}

/// `Mxxx`: moves the player to the system, at its first stellar (see the
/// module docs).
#[derive(Clone, Copy, Debug, Default)]
pub struct MoveToOp;

impl SetOpHandler<Session> for MoveToOp {
    fn apply(&self, op: &SetOp, session: &mut Session, _chance: &mut dyn Chance) {
        if let SetOp::MoveTo(system) = op {
            session.script_moves.push(ScriptMove::To(*system));
        }
    }
}

/// `Nxxx`: moves the player to the system, keeping its position (see the
/// module docs).
#[derive(Clone, Copy, Debug, Default)]
pub struct MoveKeepPositionOp;

impl SetOpHandler<Session> for MoveKeepPositionOp {
    fn apply(&self, op: &SetOp, session: &mut Session, _chance: &mut dyn Chance) {
        if let SetOp::MoveKeepPosition(system) = op {
            session.script_moves.push(ScriptMove::KeepPosition(*system));
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::{
        DudeId, DudeRecord, GovtId, HullRecord, ShipId, StockWeapon, SystemTraffic, WeaponId,
        WeaponRecord,
    };
    use crate::chance::NeverFires;
    use crate::combat::Rules;
    use crate::control::{SetExpr, SetOpKind};
    use crate::gate::HYPERGATE;
    use crate::handling::ShipFields;
    use crate::hyperspace::JumpRefusal;
    use crate::pilot::{Escort, Pilot};
    use crate::reserves::Reserves;
    use crate::save;
    use crate::testkit::{
        FAST, FakePilotCatalog, Scripted, begin_jump_now, catalog, fly_out, hull, land_now, planet,
        ship, star, weapon,
    };
    use crate::traffic::npc::NpcId;

    /// A system of `avg` ships of düde 128, ship 129, for govt 140.
    fn traffic(system: i16, avg: i16) -> (SystemId, SystemTraffic) {
        let mut dude_types = [(-1, 0); 8];
        dude_types[0] = (128, 100);
        (
            SystemId(system),
            SystemTraffic {
                dude_types,
                avg_ships: avg,
                persons: Default::default(),
            },
        )
    }

    /// [`catalog`] with four systems: 130, the start (planets 128 at
    /// (30, -40) and 129), linked to 131; 131, linked to 132, its first
    /// stellar planet 140 at (100, 200), then 141; 132, with no stellar;
    /// and 133, linked to 130 alone, its first stellar the hypergate 150
    /// at (300, 0), then planet 151. Two ships fly in 130 and three in
    /// 131. Ship 129 has a record; ship 130, a fighter of no fuel, too.
    /// The player's ship carries a blaster.
    fn moving() -> FakePilotCatalog {
        let base = catalog();
        let gate = LandingSite {
            flags2: HYPERGATE,
            ..planet(150, 300.0, 0.0)
        };
        let dry = ShipFields { fuel: 0, ..FAST };
        FakePilotCatalog {
            systems: [130, 131, 132, 133].map(SystemId).to_vec(),
            sites: vec![
                (SystemId(130), base.sites[0].1.clone()),
                (
                    SystemId(131),
                    vec![planet(140, 100.0, 200.0), planet(141, -50.0, 0.0)],
                ),
                (SystemId(133), vec![gate, planet(151, 0.0, 300.0)]),
            ],
            star_map: vec![
                star(130, (0.0, 0.0), &[131]),
                star(131, (600.0, 0.0), &[132]),
                star(132, (600.0, 600.0), &[]),
                star(133, (-600.0, 0.0), &[130]),
            ],
            traffic: vec![traffic(130, 2), traffic(131, 3)],
            dudes: vec![(
                DudeId(128),
                DudeRecord {
                    ai_type: 1,
                    govt: Some(GovtId(140)),
                    ships: vec![(ShipId(129), 1)],
                    booty: 0,
                    info_types: 0,
                },
            )],
            ships: vec![(ShipId(128), Ok(FAST)), (ShipId(130), Ok(dry))],
            ship_records: vec![ship(129, FAST), ship(130, dry)],
            weapons: vec![WeaponRecord {
                reload: 2,
                count: 30,
                speed: 2000,
                ..weapon(128)
            }],
            hulls: vec![HullRecord {
                weapons: vec![StockWeapon {
                    weapon: WeaponId(128),
                    count: 1,
                    ammo: 0,
                }],
                ..hull(128)
            }],
            ..base
        }
    }

    /// An escort of ship class `ship`, a fighter out when `carried`.
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

    /// `pilot`'s session in flight in 130 among its traffic, no save due.
    fn flying_with(catalog: &FakePilotCatalog, pilot: Pilot) -> Session {
        let mut session = Session::fly(catalog, pilot).expect("flies");
        session.populate(catalog, &mut NeverFires);
        session.take_save_due();
        session
    }

    fn flying(catalog: &FakePilotCatalog) -> Session {
        flying_with(catalog, Pilot::new(catalog, "Ada").expect("starts"))
    }

    /// [`flying`], landed on planet 128 from the centre.
    fn landed(catalog: &FakePilotCatalog) -> Session {
        let mut session = flying(catalog);
        land_now(&mut session).expect("lands");
        session.take_save_due();
        session.take_sounds();
        session
    }

    fn run(session: &mut Session, text: &str) {
        session.run_set(
            &SetExpr::parse(text).expect("parses"),
            &mut Scripted::default(),
        );
    }

    fn settle(session: &mut Session, catalog: &FakePilotCatalog) -> Settled {
        session.settle_script(catalog, &mut NeverFires)
    }

    /// Runs `text` and settles it.
    fn moved(session: &mut Session, catalog: &FakePilotCatalog, text: &str) {
        run(session, text);
        let _ = settle(session, catalog);
    }

    fn ids(sites: &[LandingSite]) -> Vec<i16> {
        sites.iter().map(|site| site.id.0).collect()
    }

    // The rules.

    #[test]
    fn the_rules_are_the_engines_until_others_are_given() {
        let session = flying(&moving());
        assert_eq!(session.script_effect_rules(), ScriptEffectRules::default());
        let rules = ScriptEffectRules {
            arrival: RuleSource::Bible,
            ..ScriptEffectRules::default()
        };
        let session = session.with_script_effect_rules(rules);
        assert_eq!(session.script_effect_rules(), rules);
    }

    #[test]
    fn each_rule_follows_its_own_rulebook_entry() {
        let engine = ScriptEffectRules {
            starless: RuleSource::Engine,
            arrival: RuleSource::Engine,
            keep_flag: RuleSource::Engine,
        };
        assert_eq!(ScriptEffectRules::default(), engine);
        assert_eq!(
            ScriptEffectRules::from_rulebook(&Rulebook::default()),
            engine
        );
        let bible = |key| {
            ScriptEffectRules::from_rulebook(
                &Rulebook::default().with_override(key, RuleSource::Bible),
            )
        };
        assert_eq!(
            bible(RuleKey::MoveStarless),
            ScriptEffectRules {
                starless: RuleSource::Bible,
                ..engine
            }
        );
        assert_eq!(
            bible(RuleKey::MoveArrival),
            ScriptEffectRules {
                arrival: RuleSource::Bible,
                ..engine
            }
        );
        assert_eq!(
            bible(RuleKey::MoveKeepFlag),
            ScriptEffectRules {
                keep_flag: RuleSource::Bible,
                ..engine
            }
        );
    }

    // Through the registry.

    #[test]
    fn m_and_n_are_queued_until_settled() {
        let catalog = moving();
        let mut session = flying(&catalog);
        run(&mut session, "M131 N132");
        assert_eq!(session.system(), SystemId(130), "nothing yet");
        assert_eq!(session.take_script_notes(), []);
        assert!(!session.take_save_due());
        assert_eq!(settle(&mut session, &catalog).moved, Some(SystemId(132)));
        assert_eq!(session.system(), SystemId(132));
        assert!(session.take_save_due());
        assert_eq!(settle(&mut session, &catalog), Settled::default(), "once");
        assert!(!session.take_save_due());
    }

    #[test]
    fn the_moves_apply_in_order() {
        let catalog = moving();
        let mut session = flying(&catalog);
        moved(&mut session, &catalog, "M131 N133");
        assert_eq!(session.system(), SystemId(133));
        assert_eq!(session.player().position, Vec2::new(100.0, 200.0));
        assert_eq!(ids(&session.sites), [150, 151]);
    }

    #[test]
    fn a_move_to_a_system_that_is_not_there_changes_nothing() {
        let catalog = moving();
        let mut session = flying(&catalog);
        let before = session.clone();
        run(&mut session, "M150 N999");
        assert_eq!(settle(&mut session, &catalog), Settled::default());
        assert_eq!(session, before);
        assert!(!session.take_save_due());
    }

    // In flight.

    #[test]
    fn m_in_flight_puts_the_ship_at_rest_on_the_first_stellar() {
        let catalog = moving();
        let mut session = flying(&catalog);
        session.player.position = Vec2::new(10.0, 10.0);
        session.player.velocity = Vec2::new(5.0, -1.0);
        session.player.heading = 1.5;
        run(&mut session, "M131");
        assert_eq!(settle(&mut session, &catalog).moved, Some(SystemId(131)));
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(session.player().position, Vec2::new(100.0, 200.0));
        assert_eq!(session.player().velocity, Vec2::ZERO);
        assert_eq!(session.player().heading, 1.5, "heading kept");
        assert_eq!(ids(&session.sites), [140, 141]);
        assert_eq!(session.landed(), None);
        assert_eq!(session.pilot().stellar(), None);
        assert!(session.take_save_due());
        moved(&mut session, &catalog, "M133");
        assert_eq!(
            session.player().position,
            Vec2::new(300.0, 0.0),
            "a gate as any stellar"
        );
    }

    #[test]
    fn n_in_flight_keeps_the_position_and_velocity() {
        let catalog = moving();
        let mut session = flying(&catalog);
        session.player.position = Vec2::new(10.0, 10.0);
        session.player.velocity = Vec2::new(5.0, -1.0);
        moved(&mut session, &catalog, "N131");
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(session.player().position, Vec2::new(10.0, 10.0));
        assert_eq!(session.player().velocity, Vec2::new(5.0, -1.0));
        assert_eq!(ids(&session.sites), [140, 141]);
    }

    #[test]
    fn m_into_a_system_with_no_stellar_keeps_the_ship_moving_by_the_engine() {
        let catalog = moving();
        let mut session = flying(&catalog);
        session.player.position = Vec2::new(10.0, 10.0);
        session.player.velocity = Vec2::new(5.0, -1.0);
        moved(&mut session, &catalog, "M132");
        assert_eq!(session.system(), SystemId(132));
        assert_eq!(session.player().position, Vec2::new(10.0, 10.0));
        assert_eq!(session.player().velocity, Vec2::new(5.0, -1.0));
        assert_eq!(session.sites, []);
    }

    #[test]
    fn m_into_a_system_with_no_stellar_stops_at_the_centre_by_the_bible() {
        let catalog = moving();
        let rules = ScriptEffectRules {
            starless: RuleSource::Bible,
            ..ScriptEffectRules::default()
        };
        let mut session = flying(&catalog).with_script_effect_rules(rules);
        session.player.position = Vec2::new(10.0, 10.0);
        session.player.velocity = Vec2::new(5.0, -1.0);
        moved(&mut session, &catalog, "M132");
        assert_eq!(session.player().position, Vec2::ZERO);
        assert_eq!(session.player().velocity, Vec2::ZERO);
        session.player.velocity = Vec2::new(5.0, -1.0);
        moved(&mut session, &catalog, "N131");
        assert_eq!(session.player().velocity, Vec2::new(5.0, -1.0), "N keeps");
    }

    #[test]
    fn a_move_in_flight_lets_go_of_the_scene() {
        let catalog = moving();
        for text in ["M131", "N130"] {
            let mut session = flying(&catalog);
            session.hold_fire(true, false);
            session.tick_combat(Rules::default(), &mut NeverFires);
            assert_eq!(session.shots().len(), 1, "{text}");
            session.select_next_stellar();
            session.target = Some(NpcId(0));
            session.gate = Some(StellarId(128));
            session.strikes = session.combat.take_strikes();
            moved(&mut session, &catalog, text);
            assert_eq!(session.shots(), [], "{text}");
            assert_eq!(session.nav_target(), None, "{text}");
            assert_eq!(session.target, None, "{text}");
            assert_eq!(session.gate, None, "{text}");
            assert_eq!(session.strikes, [], "{text}");
        }
    }

    #[test]
    fn a_move_in_flight_gives_up_a_jump_being_prepared() {
        let catalog = moving();
        let mut session = flying(&catalog);
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        session.player.velocity = Vec2::new(0.0, 5.0);
        session.begin_jump().expect("accepted");
        assert!(session.preparing_jump().is_some());
        session.tick(crate::flight::Controls::default());
        assert!(session.thrusting(), "braking");
        moved(&mut session, &catalog, "N133");
        assert_eq!(session.preparing_jump(), None);
        assert!(!session.thrusting());
        assert_eq!(session.engine_glow(), 0);
    }

    #[test]
    fn by_the_engine_only_the_escorts_and_fighters_out_follow_into_another_system() {
        let catalog = moving();
        let mut pilot = Pilot::new(&catalog, "Ada").expect("starts");
        pilot.escorts = vec![escort(129, false), escort(130, true)];
        let mut session = flying_with(&catalog, pilot);
        assert_eq!(session.npcs().len(), 4);
        let fleet: Vec<NpcId> = session.fleet.iter().flatten().copied().collect();
        assert_eq!(fleet.len(), 2);
        moved(&mut session, &catalog, "M131");
        let left: Vec<NpcId> = session.npcs().iter().map(|npc| npc.id).collect();
        assert_eq!(left, fleet, "the others stay behind");
        assert_eq!(session.pilot().escorts().len(), 2, "the dry fighter too");
        assert_eq!(session.take_fighter_notes(), []);
        assert_eq!(session.traffic.ship_types(), [ShipId(129)], "131's table");
        for _ in 0..3 {
            session.tick_traffic(&catalog, &crate::ai::Peaceful, &mut NeverFires);
        }
        assert!(
            session.npcs().iter().all(|npc| fleet.contains(&npc.id)),
            "no initial population"
        );
    }

    #[test]
    fn by_the_engine_a_move_within_the_system_keeps_its_traffic() {
        let catalog = moving();
        let mut session = flying(&catalog);
        let npcs = session.npcs().to_vec();
        moved(&mut session, &catalog, "N130");
        assert_eq!(session.npcs(), npcs);
    }

    #[test]
    fn by_the_engine_the_system_is_not_explored_and_the_course_kept() {
        let catalog = moving();
        let mut session = flying(&catalog);
        session.plot_course(SystemId(132)).expect("a route");
        moved(&mut session, &catalog, "M133");
        assert!(!session.pilot().has_explored(SystemId(133)));
        assert_eq!(session.course(), [SystemId(131), SystemId(132)]);
    }

    #[test]
    fn j_refuses_while_the_courses_next_system_is_not_linked_from_here() {
        let catalog = moving();
        let mut session = flying(&catalog);
        session.plot_course(SystemId(131)).expect("a route");
        moved(&mut session, &catalog, "M133");
        fly_out(&mut session);
        assert_eq!(session.begin_jump(), Err(JumpRefusal::NoDestination));
        session.plot_course(SystemId(131)).expect("a route by 130");
        assert_eq!(session.course(), [SystemId(130), SystemId(131)]);
        assert_eq!(begin_jump_now(&mut session), Ok(SystemId(130)));
    }

    #[test]
    fn as_an_arrival_the_move_explores_clears_the_course_and_populates() {
        let catalog = moving();
        let rules = ScriptEffectRules {
            arrival: RuleSource::Bible,
            ..ScriptEffectRules::default()
        };
        let mut pilot = Pilot::new(&catalog, "Ada").expect("starts");
        pilot.escorts = vec![escort(129, false), escort(130, true)];
        let mut session = flying_with(&catalog, pilot).with_script_effect_rules(rules);
        session.plot_course(SystemId(132)).expect("a route");
        moved(&mut session, &catalog, "M131");
        assert!(session.pilot().has_explored(SystemId(131)));
        assert_eq!(session.course(), []);
        assert_eq!(session.pilot().escorts().len(), 1, "the dry fighter left");
        assert_eq!(
            session.take_fighter_notes(),
            [crate::bay::FighterNote::Abandoned(1)]
        );
        assert_eq!(session.npcs().len(), 4, "three ships and the escort");
        assert_eq!(session.fleet.iter().flatten().count(), 1);
    }

    // Landed.

    #[test]
    fn landed_m_moves_the_pilot_and_leaves_the_spaceport_as_it_is() {
        let catalog = moving();
        let mut session = landed(&catalog);
        assert_eq!(session.landed(), Some(StellarId(128)));
        session.plot_course(SystemId(131)).expect("a route");
        moved(&mut session, &catalog, "M131");
        assert_eq!(session.pilot().system(), SystemId(131));
        assert_eq!(session.pilot().stellar(), Some(StellarId(140)));
        assert_eq!(session.landed(), Some(StellarId(128)), "still docked");
        assert_eq!(ids(&session.sites), [128, 129], "the port's stellars");
        assert_eq!(session.player().position, Vec2::new(100.0, 200.0));
        assert!(!session.pilot().has_explored(SystemId(131)));
        assert_eq!(session.course(), [SystemId(131)], "kept");
        assert!(session.take_save_due());
    }

    #[test]
    fn landed_m_onto_a_gate_or_into_a_system_with_no_stellar_leaves_the_pilot_on_none() {
        let catalog = moving();
        let mut session = landed(&catalog);
        moved(&mut session, &catalog, "M133");
        assert_eq!(session.pilot().stellar(), None, "no docking at a gate");
        assert_eq!(session.player().position, Vec2::new(300.0, 0.0));
        moved(&mut session, &catalog, "M131 M132");
        assert_eq!(session.pilot().stellar(), None);
        assert_eq!(session.player().position, Vec2::ZERO);
        assert_eq!(session.landed(), Some(StellarId(128)));
    }

    #[test]
    fn a_pilot_saved_after_a_landed_m_flies_again_docked_at_the_new_stellar() {
        let catalog = moving();
        let mut session = landed(&catalog);
        moved(&mut session, &catalog, "M131");
        let saved = save::decode(&save::encode(session.pilot())).expect("loads");
        let flown = Session::fly(&catalog, saved).expect("flies");
        assert_eq!(flown.system(), SystemId(131));
        assert_eq!(flown.landed(), Some(StellarId(140)));
        assert_eq!(flown.player().position, Vec2::new(100.0, 200.0));
    }

    #[test]
    fn the_take_off_after_a_landed_m_is_in_the_new_system_at_its_first_stellar() {
        let catalog = moving();
        let mut session = landed(&catalog);
        moved(&mut session, &catalog, "M131");
        assert_eq!(session.take_off(), Some(StellarId(128)));
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(ids(&session.sites), [140, 141]);
        assert_eq!(session.player().position, Vec2::new(100.0, 200.0));
        assert!(session.pilot().has_explored(SystemId(131)), "explored");
        session.tick_traffic(&catalog, &crate::ai::Peaceful, &mut NeverFires);
        assert_eq!(session.npcs().len(), 3, "131 populated");
        let mut session = landed(&catalog);
        moved(&mut session, &catalog, "M132");
        session.take_off();
        assert_eq!(session.player().position, Vec2::ZERO, "the centre");
        assert_eq!(session.sites, []);
    }

    #[test]
    fn the_take_off_after_a_landed_n_is_from_where_the_ship_touched_down() {
        let catalog = moving();
        let mut session = flying(&catalog);
        session.player.position = Vec2::new(10.0, -10.0);
        land_now(&mut session).expect("lands");
        assert_eq!(session.player().position, Vec2::new(30.0, -40.0), "docked");
        moved(&mut session, &catalog, "N131");
        assert_eq!(session.pilot().stellar(), None);
        assert_eq!(session.landed(), Some(StellarId(128)));
        session.take_off();
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(session.player().position, Vec2::new(10.0, -10.0));
        assert_eq!(ids(&session.sites), [140, 141]);
        session.player.position = Vec2::new(-45.0, 0.0);
        land_now(&mut session).expect("lands on 141");
        session.take_off();
        assert_eq!(
            session.player().position,
            Vec2::new(-50.0, 0.0),
            "the flag spent"
        );
    }

    #[test]
    fn as_an_arrival_a_landed_move_explores_and_clears_the_course_at_once() {
        let catalog = moving();
        let rules = ScriptEffectRules {
            arrival: RuleSource::Bible,
            ..ScriptEffectRules::default()
        };
        let mut session = landed(&catalog).with_script_effect_rules(rules);
        session.plot_course(SystemId(131)).expect("a route");
        moved(&mut session, &catalog, "N131");
        assert!(session.pilot().has_explored(SystemId(131)));
        assert_eq!(session.course(), []);
        assert_eq!(session.landed(), Some(StellarId(128)));
    }

    #[test]
    fn a_relocation_after_a_landed_move_takes_off_where_it_docked() {
        let catalog = moving();
        let mut session = landed(&catalog);
        moved(&mut session, &catalog, "M133");
        session
            .relocate(&catalog, SystemId(131), StellarId(141))
            .expect("moves");
        session.take_off();
        assert_eq!(ids(&session.sites), [140, 141]);
        assert_eq!(session.player().position, Vec2::new(-50.0, 0.0));
    }

    // The take-off's flag.

    /// The ship lands from (10, -10) on planet 128, after `text` ran in
    /// flight under `keep_flag`, and takes off: where it is then.
    fn taken_off_after(text: &str, keep_flag: RuleSource) -> Vec2 {
        let catalog = moving();
        let rules = ScriptEffectRules {
            keep_flag,
            ..ScriptEffectRules::default()
        };
        let mut session = flying(&catalog).with_script_effect_rules(rules);
        moved(&mut session, &catalog, text);
        session.player.position = Vec2::new(10.0, -10.0);
        land_now(&mut session).expect("lands");
        session.take_off();
        session.player().position
    }

    #[test]
    fn by_the_engine_an_n_in_flight_keeps_the_next_take_offs_landing_position() {
        assert_eq!(
            taken_off_after("N130", RuleSource::Engine),
            Vec2::new(10.0, -10.0)
        );
        assert_eq!(
            taken_off_after("M130", RuleSource::Engine),
            Vec2::new(30.0, -40.0),
            "only N"
        );
    }

    #[test]
    fn by_the_other_reading_an_n_in_flight_leaves_the_take_off_as_any_other() {
        assert_eq!(
            taken_off_after("N130", RuleSource::Bible),
            Vec2::new(30.0, -40.0)
        );
    }

    #[test]
    fn by_the_other_reading_a_landed_n_still_keeps_the_landing_position() {
        let catalog = moving();
        let rules = ScriptEffectRules {
            keep_flag: RuleSource::Bible,
            ..ScriptEffectRules::default()
        };
        let mut session = flying(&catalog).with_script_effect_rules(rules);
        session.player.position = Vec2::new(10.0, -10.0);
        land_now(&mut session).expect("lands");
        moved(&mut session, &catalog, "N130");
        session.take_off();
        assert_eq!(session.player().position, Vec2::new(10.0, -10.0));
    }

    #[test]
    fn the_moves_are_registered_by_nova() {
        let kinds: Vec<SetOpKind> = crate::session::nova_set_ops().kinds().collect();
        assert!(kinds.contains(&SetOpKind::MoveTo));
        assert!(kinds.contains(&SetOpKind::MoveKeepPosition));
    }
}
