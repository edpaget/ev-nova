//! The set-expression hooks on the session: the `chär`'s `OnStart`, an
//! `oütf`'s `OnPurchase` and `OnSell`, and a `shïp`'s `OnPurchase`,
//! `OnCapture` and `OnRetire`, each run where the original runs it
//! through `_EvalMissionBitSetString` (@0x99dc5).
//!
//! **Running one.** [`Session::run_script`] runs a record's [`Script`]
//! as [`Session::run_set`] runs a set expression: the bits written, `R(...)`
//! drawn on the caller's [`Chance`], the other operators applied by
//! `Session::apply_op` (`G`, `D`, `X`, `C`, `E`, `H`, `T`, `M`, `N`, `Q`
//! and `P`; any other is skipped and told once a session as a
//! [`ScriptNote`](crate::ScriptNote)). A blank script does nothing at
//! all: no save is due and nothing is told. One that did not parse runs
//! no operator, but resets as it ends, as any script of text that is not
//! empty does (see the `precalced` module).
//!
//! **Where each runs**, in the original's order:
//!
//! | Hook | Trigger | Order |
//! |---|---|---|
//! | `chär` `OnStart` | [`Session::begin`], once on a new pilot (`_DoNewPilot` @0x19345) | after the pilot is set up and flying (ship, default items, system explored), before its first save; a pilot opened from a save never runs it. By the engine no `shïp` `OnPurchase` runs for the starting ship; by the other reading of [`RuleKey::StartShipPurchase`](crate::RuleKey::StartShipPurchase) it runs once, right before `OnStart` |
//! | `oütf` `OnPurchase` | [`Session::outfit`], each unit bought (`_DoOutfitDialog` @0x5c2cb) | after the cash and the grant, even when the grant added nothing (a map, a paint); then an outfit removed after purchase goes, and the ship is refitted |
//! | `oütf` `OnSell` | [`Session::outfit`], each unit sold (@0x5cff6) | after one is removed and the cash credited, before the refit |
//! | `shïp` `OnRetire` | [`Session::buy_ship`], the ship traded in (`_DoShipyardDialog` @0x5ecff) | first, the old class still flown and its outfits still owned |
//! | `shïp` `OnPurchase` | [`Session::buy_ship`], the ship bought (@0x5f00d) | last: the new class, cash and outfits in place, the reserves not yet filled. By the engine the paint is cleared after it (@0x5f022), so a paint it grants is lost; by the other reading of [`RuleKey::PurchasePaintOrder`](crate::RuleKey::PurchasePaintOrder) the paint is cleared before it. Either way the shield, armour and fuel are then filled, to the class and outfits it leaves (@0x5f051-0x5f07b) |
//! | `shïp` `OnCapture` | [`Session::plunder`]'s capture taken as an escort straight away, and [`Session::assign`]'s Use As Escort (`_DoPlunderDialog` @0x9460e) | before the ship joins the fleet |
//! | `shïp` `OnRetire`, then `OnCapture` | [`Session::assign`]'s Use As My Ship (`_DoShipCapture` @0x41662, @0x416c9) | the old class's `OnRetire`, then the captured class's `OnCapture`, the old class still flown. By the engine both run before the outfit swap, which strips a non-persistent outfit either grants, and before the fuel draw; by the other reading of [`RuleKey::CaptureHookOrder`](crate::RuleKey::CaptureHookOrder) they run after the swap (still before the class changes and the fuel draw). No hook runs when the prize is abandoned |
//!
//! A refused order, purchase or capture runs nothing; a ship class with
//! no record runs nothing.
//!
//! Three differences from the original's order cannot be seen, as no set
//! operator reads the cash or the fleet: the original debits a unit's
//! cash after its grant, here before; it credits the trade-in before
//! `OnRetire`, here after; and on Use As My Ship it spawns the old ship
//! as an escort before the hooks, here after.
//!
//! Elsewhere: the player ship's `OnRetire` when it is destroyed
//! (`_HandlePlayer` @0x6f49d) and its `OnPurchase` after the post-loss
//! reset (@0x68f55) wait for the player's death; the ship-change
//! operators `C`, `E` and `H` run no hook at all (`_EvalSetExp` never
//! calls `_EvalMissionBitSetString`; see the `ship_change` module).

use super::Session;
use crate::catalog::{PilotCatalog, ShipId, TrafficCatalog};
use crate::chance::Chance;
use crate::control::Script;
use crate::geometry::Vec2;
use crate::landing::nearest_stellar;
use crate::rulebook::{RuleKey, RuleSource, Rulebook};

/// The disputed orders of the set-expression hooks that the session
/// follows, projected from its rule set (see [`Session::with_rules`]):
/// the engine's by default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HookRules {
    /// Whether buying a ship clears the paint after the new ship's
    /// `OnPurchase` or before it ([`RuleKey::PurchasePaintOrder`]).
    pub purchase_paint_order: RuleSource,
    /// Whether Use As My Ship runs `OnRetire` and `OnCapture` before the
    /// outfit swap or after it ([`RuleKey::CaptureHookOrder`]).
    pub capture_hook_order: RuleSource,
    /// Whether a new pilot's starting ship runs its `OnPurchase`
    /// ([`RuleKey::StartShipPurchase`]).
    pub start_ship_purchase: RuleSource,
}

impl HookRules {
    /// The rules `rulebook` chooses: its [`RuleKey::PurchasePaintOrder`],
    /// [`RuleKey::CaptureHookOrder`] and [`RuleKey::StartShipPurchase`]
    /// entries.
    #[must_use]
    pub fn from_rulebook(rulebook: &Rulebook) -> Self {
        Self {
            purchase_paint_order: rulebook.source_for(RuleKey::PurchasePaintOrder),
            capture_hook_order: rulebook.source_for(RuleKey::CaptureHookOrder),
            start_ship_purchase: rulebook.source_for(RuleKey::StartShipPurchase),
        }
    }
}

/// A ship class's hooks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ShipHook {
    /// `OnPurchase`.
    Purchase,
    /// `OnCapture`.
    Capture,
    /// `OnRetire`.
    Retire,
}

impl Session {
    /// The rules the hooks follow, projected from the session's rule set
    /// ([`Session::with_rules`]).
    pub(super) fn hook_rules(&self) -> HookRules {
        HookRules::from_rulebook(&self.rules)
    }

    /// Runs `script` on the session (see the module docs), drawing
    /// `R(...)` on `chance`: nothing at all when its text was empty, and
    /// no operator but the reset its end runs when it did not parse
    /// ([`Script::runs`]).
    pub fn run_script(&mut self, script: &Script, chance: &mut (impl Chance + ?Sized)) {
        self.run_set(script.runs(), chance);
    }

    /// Runs ship class `ship`'s `hook`, drawing on `chance`: nothing when
    /// the session has no record of it.
    pub(super) fn ship_hook(&mut self, ship: ShipId, hook: ShipHook, chance: &mut dyn Chance) {
        let Some(record) = self.statics.ship_record(ship) else {
            return;
        };
        let script = match hook {
            ShipHook::Purchase => &record.on_purchase,
            ShipHook::Capture => &record.on_capture,
            ShipHook::Retire => &record.on_retire,
        }
        .clone();
        self.run_script(&script, chance);
    }

    /// Use As My Ship's hooks: the flown class's `OnRetire`, then
    /// `captured`'s `OnCapture`, drawing on `chance`.
    pub(super) fn retire_for_capture(&mut self, captured: ShipId, chance: &mut dyn Chance) {
        self.ship_hook(self.pilot.ship, ShipHook::Retire, chance);
        self.ship_hook(captured, ShipHook::Capture, chance);
    }

    /// Begins a new pilot's game: starts the first-flight tutorial (see
    /// the [`tutorial`](crate::tutorial) module); by the engine's reading of
    /// [`RuleKey::NebulaNewPilot`](crate::RuleKey::NebulaNewPilot),
    /// withdraws the `Explored` events [`Session::fly`] raised, so the
    /// start system's nebulae wait for the first exploration (see the
    /// `nebulae` module); explores the start system's visible
    /// neighbours, or by the engine their heads (see the `explored`
    /// module), destroys the stellars that start the
    /// game destroyed (see the `stellars` module), then, by the engine's
    /// reading of
    /// [`RuleKey::CharacterStatusSpread`](crate::RuleKey::CharacterStatusSpread),
    /// spreads the first `chär`'s statuses to its governments' allies and
    /// enemies, in both legal stores ([`legal::spread_statuses`](crate::legal::spread_statuses)),
    /// then runs its `OnStart` once, drawing on `chance` (see the module docs),
    /// and nothing when the `chär` cannot be read, then refreshes the
    /// galaxy, so the systems its bits show are visible (`_PlayGame`
    /// @0x457e9; see the `story` module), rebuilds the stellars shown
    /// (see the `stellars` module), and last draws the offers' roll table
    /// on `chance`, as `_DoNewPilot`'s `_ResetPlayer` does (@0x18dd4).
    /// The original draws it before the `OnStart`; nothing here reads it,
    /// so it settles before the first frame either way. For a pilot just created and
    /// flown, before its first save; never for one opened from a save.
    ///
    /// It is the engine's `_DoNewPilot` seam, so it first puts the ship in
    /// space at (50, 50), at rest, facing up, as `_DoNewPilot`'s
    /// `_ResetPlayer(1)` does (@0x18dd4, @0x1d427-0x1d441), where nothing
    /// later moves it: [`Session::start`], which a test flies a new pilot
    /// with, keeps the centre. And after the `OnStart` and the refresh it
    /// records, as the stellar last landed on, the one `_DoNewPilot`
    /// saves (@0x1926e-0x192a6): the stellar shown nearest the ship
    /// ([`nearest_stellar`]), or else the first the system lists, or else
    /// none, so a new pilot reopened before it lands resumes over it.
    /// Left out: the original saves stellar index 0 when the system lists
    /// none (@0x19358); the engine keeps none, which resumes at the
    /// centre.
    pub fn begin(
        &mut self,
        catalog: &(impl PilotCatalog + TrafficCatalog),
        chance: &mut dyn Chance,
    ) {
        // `_ResetPlayer(1)` (@0x18dd4, @0x1d427-0x1d441).
        self.player.position = Vec2::new(50.0, 50.0);
        self.player.velocity = Vec2::ZERO;
        self.player.heading = 0.0;
        self.player_speed = 0.0;
        self.touchdown = self.player.position;
        // A new pilot starts the first-flight tutorial (`_DoNewPilot`
        // @0x19254; see the `tutorial` module).
        self.tutorial = crate::tutorial::Tutorial::NEW_PILOT;
        // `_DoNewPilot` explores its start system's nebulae while every
        // one is inactive (@0x19118; see the `nebulae` module).
        self.withdraw_new_pilot_explored();
        // `_DoNewPilot` explores the start system's neighbours
        // (@0x18f97-0x1902e) before its destroyed stellars (@0x192d6) and
        // its `OnStart` (see the `explored` module).
        self.explore_start_neighbours();
        self.destroy_from_the_start();
        if let Ok(start) = catalog.first_character() {
            // `_DoNewPilot`'s first `_ActivateCharResource` (@0x18e5f),
            // after `_ResetPlayerRecord` (@0x18de0) and before the
            // `OnStart`: the `chär`'s statuses spread (see the `legal`
            // module). By the Bible, `Pilot::new` and the flight's
            // derivation already keep each with its own government.
            if self.rules.source_for(crate::RuleKey::CharacterStatusSpread) == RuleSource::Engine {
                crate::legal::spread_statuses(
                    &mut self.pilot,
                    self.galaxy.governed(),
                    &start.legal,
                    &self.statics.govts,
                );
            }
            if self.hook_rules().start_ship_purchase == RuleSource::Bible {
                self.ship_hook(self.pilot.ship, ShipHook::Purchase, chance);
            }
            self.run_script(&start.on_start, chance);
        }
        // `_PlayGame`'s refresh, before its rebuild (@0x457e9, @0x457ee):
        // `_DoNewPilot`'s, before the `OnStart`, was the flight's.
        self.refresh_galaxy(catalog);
        // The stellar `_DoNewPilot` saves as last landed on (@0x1926e,
        // @0x1927b-0x192a6), into the file only (@0x1935f); the engine's
        // pilot is what it saves.
        self.pilot.stellar = nearest_stellar(self.player.position, &self.sites)
            .or_else(|| self.listed.first().map(|site| site.id));
        // Game entry's reset comes after `_DoNewPilot`'s
        // `OnStart` (@0x1925e, @0x19ffb; see the `ranks` module).
        self.reset_precalced();
        // `_DoNewPilot`'s rebuild point, after its `OnStart` (@0x1937f).
        self.rebuild_stellars();
        // `_DoNewPilot`'s `_ResetPlayer` (@0x18dd4) draws the offers' roll
        // table before its `OnStart`; nothing above reads the table, so
        // drawing it last settles the same table before the first frame
        // and the first save.
        self.draw_offer_rolls(chance);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::rc::Rc;

    use super::*;
    use crate::board::{Assigned, Assignment, Take, Taken};
    use crate::catalog::{CharacterStart, OutfitId, OutfitRecord, StartError, StellarId, SystemId};
    use crate::control::{Bit, SetOp};
    use crate::outfit_effects::Rgb15;
    use crate::outfitter::{FIRST_OUTFIT, OutfitFlags, OutfitRefusal, Reselection};
    use crate::pilot::Pilot;
    use crate::reserves::Reserves;
    use crate::session::control::SetOpObserver;
    use crate::session::tests::{
        BLASTER, NEW, aboard, boardable, buy, captured, kitted, outfitted, outfitting,
        outfitting_effects, outfitting_on_bit_7, sell, shipbuying, take,
    };
    use crate::shipyard::ShipRefusal;
    use crate::testkit::{Draws, FakePilotCatalog, Scripted, catalog, outfit};

    fn bit(n: u16) -> Bit {
        Bit::new(n).expect("a bit")
    }

    /// What the probe saw when a hook ran.
    #[derive(Clone, Debug, PartialEq)]
    struct Snap {
        /// The hook's tag: the mission `S<tag>` names.
        tag: i16,
        ship: ShipId,
        cash: i64,
        outfits: BTreeMap<OutfitId, u16>,
        escorts: usize,
        paint: Option<Rgb15>,
        reserves: Reserves,
    }

    /// Records a [`Snap`] each time an `S<tag>` runs: the hooks are
    /// written with a distinct tag each.
    #[derive(Debug, Default)]
    struct Probe(RefCell<Vec<Snap>>);

    impl SetOpObserver for Probe {
        fn saw(&self, op: &SetOp, session: &Session) {
            if let SetOp::StartMission(mission) = op {
                let pilot = session.pilot();
                self.0.borrow_mut().push(Snap {
                    tag: mission.0,
                    ship: pilot.ship,
                    cash: pilot.cash(),
                    outfits: pilot.outfits.clone(),
                    escorts: pilot.escorts().len(),
                    paint: pilot.paint,
                    reserves: pilot.reserves(),
                });
            }
        }
    }

    /// `session` with the probe watching each `S` as it runs.
    fn probed(session: Session) -> (Session, Rc<Probe>) {
        let probe = Rc::new(Probe::default());
        (session.with_set_op_observer(probe.clone()), probe)
    }

    fn tags(probe: &Probe) -> Vec<i16> {
        probe.0.borrow().iter().map(|snap| snap.tag).collect()
    }

    // Running a script.

    #[test]
    fn a_script_runs_as_its_set_expression() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.run_script(&Script::parse("b7 K200"), &mut Scripted::default());
        assert!(session.control_bit(bit(7)));
        assert!(
            session
                .pilot()
                .ranks()
                .contains(&crate::catalog::RankId(200))
        );
        assert!(session.take_save_due());
        assert_eq!(session.take_script_notes(), []);
    }

    #[test]
    fn a_blank_or_broken_script_runs_no_operator() {
        for script in [Script::default(), Script::parse("S300&")] {
            let (mut session, probe) = probed(Session::start(&catalog()).expect("starts"));
            session.run_script(&script, &mut Scripted::default());
            assert!(!session.take_save_due(), "{script:?}");
            assert_eq!(session.take_script_notes(), [], "{script:?}");
            assert!(tags(&probe).is_empty(), "{script:?}");
        }
    }

    #[test]
    fn the_hook_rules_follow_their_three_rulebook_keys() {
        assert_eq!(
            HookRules::from_rulebook(&Rulebook::default()),
            HookRules::default()
        );
        assert_eq!(
            HookRules::default(),
            HookRules {
                purchase_paint_order: RuleSource::Engine,
                capture_hook_order: RuleSource::Engine,
                start_ship_purchase: RuleSource::Engine,
            }
        );
        let bible = |key| {
            HookRules::from_rulebook(&Rulebook::default().with_override(key, RuleSource::Bible))
        };
        assert_eq!(
            bible(RuleKey::PurchasePaintOrder),
            HookRules {
                purchase_paint_order: RuleSource::Bible,
                ..HookRules::default()
            }
        );
        assert_eq!(
            bible(RuleKey::CaptureHookOrder),
            HookRules {
                capture_hook_order: RuleSource::Bible,
                ..HookRules::default()
            }
        );
        assert_eq!(
            bible(RuleKey::StartShipPurchase),
            HookRules {
                start_ship_purchase: RuleSource::Bible,
                ..HookRules::default()
            }
        );
        assert_eq!(bible(RuleKey::CrimeGains), HookRules::default());
    }

    // The outfitter.

    /// The speed booster (300) bought and sold.
    const SPEED: OutfitId = OutfitId(300);
    /// A map of 1 jump (306), a paint (308), a shield removed after
    /// purchase (309).
    const MAP: OutfitId = OutfitId(306);
    const PAINTER: OutfitId = OutfitId(308);
    const REMOVED: OutfitId = OutfitId(309);
    /// The fuel tank (303).
    const TANK: OutfitId = OutfitId(303);

    /// `catalog` with `outfit`'s `OnPurchase` and `OnSell` set.
    fn hooked_outfit(
        mut catalog: FakePilotCatalog,
        outfit: OutfitId,
        on_purchase: &str,
        on_sell: &str,
    ) -> FakePilotCatalog {
        for record in &mut catalog.outfits {
            if record.id == outfit {
                record.on_purchase = Script::parse(on_purchase);
                record.on_sell = Script::parse(on_sell);
            }
        }
        catalog
    }

    #[test]
    fn buying_an_outfit_runs_its_on_purchase_once_after_the_cash_and_the_grant() {
        let catalog = hooked_outfit(outfitting(), SPEED, "S201", "S202");
        let (mut session, probe) = probed(outfitted(&catalog));
        assert_eq!(session.outfit(buy(SPEED), &mut Scripted::default()), Ok(()));
        let snaps = probe.0.borrow().clone();
        assert_eq!(tags(&probe), [201], "once, and not OnSell");
        assert_eq!(snaps[0].cash, 24_000, "paid");
        assert_eq!(snaps[0].outfits.get(&SPEED), Some(&1), "granted");
        assert_eq!(session.outfit(buy(SPEED), &mut Scripted::default()), Ok(()));
        assert_eq!(tags(&probe), [201, 201], "once each unit");
    }

    #[test]
    fn a_refused_purchase_or_sale_runs_no_hook() {
        let catalog = hooked_outfit(outfitting(), SPEED, "S201", "S202");
        let (mut session, probe) = probed(Session::start(&catalog).expect("starts"));
        assert_eq!(
            session.outfit(buy(SPEED), &mut Scripted::default()),
            Err(OutfitRefusal::NoOutfitter),
            "in flight"
        );
        let (mut session, probe_landed) = probed(outfitted(&catalog));
        assert!(
            session
                .outfit(sell(SPEED), &mut Scripted::default())
                .is_err(),
            "none owned"
        );
        session.pilot.cash = 10;
        assert!(
            session
                .outfit(buy(SPEED), &mut Scripted::default())
                .is_err(),
            "too dear"
        );
        assert!(tags(&probe).is_empty());
        assert!(tags(&probe_landed).is_empty());
    }

    #[test]
    fn an_outfit_the_grant_does_not_add_still_runs_its_on_purchase() {
        let catalog = hooked_outfit(outfitting_effects(), MAP, "S203", "");
        let catalog = hooked_outfit(catalog, PAINTER, "S204", "");
        let (mut session, probe) = probed(outfitted(&catalog));
        session
            .outfit(buy(MAP), &mut Scripted::default())
            .expect("bought");
        session
            .outfit(buy(PAINTER), &mut Scripted::default())
            .expect("bought");
        assert_eq!(tags(&probe), [203, 204]);
        assert_eq!(session.pilot().owned(MAP), 0, "nothing added");
        assert!(probe.0.borrow()[1].paint.is_some(), "painted by then");
    }

    #[test]
    fn an_outfit_removed_after_purchase_is_still_owned_when_its_hook_runs() {
        let catalog = hooked_outfit(outfitting_effects(), REMOVED, "S205", "");
        let (mut session, probe) = probed(outfitted(&catalog));
        session
            .outfit(buy(REMOVED), &mut Scripted::default())
            .expect("bought");
        assert_eq!(probe.0.borrow()[0].outfits.get(&REMOVED), Some(&1));
        assert_eq!(session.pilot().owned(REMOVED), 0, "gone after");
    }

    #[test]
    fn a_grant_in_on_purchase_lands_and_is_refitted() {
        let catalog = hooked_outfit(outfitting(), SPEED, "G303 b9", "");
        let mut session = outfitted(&catalog);
        let fuel = session.reserves().fuel.max;
        session
            .outfit(buy(SPEED), &mut Scripted::default())
            .expect("bought");
        assert_eq!(session.pilot().owned(TANK), 1);
        assert_eq!(session.reserves().fuel.max, fuel + 100.0, "refitted");
        assert!(session.control_bit(bit(9)));
    }

    #[test]
    fn on_purchase_draws_on_the_chance_given() {
        let catalog = hooked_outfit(outfitting(), SPEED, "R(b1 b2)", "");
        let mut session = outfitted(&catalog);
        let mut chance = Scripted::rolling(&[1]);
        session.outfit(buy(SPEED), &mut chance).expect("bought");
        assert_eq!(chance.sides_asked, [2]);
        assert!(session.control_bit(bit(1)));
        assert!(!session.control_bit(bit(2)));
    }

    #[test]
    fn selling_an_outfit_runs_its_on_sell_once_after_the_cash_and_the_removal() {
        let catalog = hooked_outfit(outfitting(), SPEED, "S201", "S202");
        let (mut session, probe) = probed(outfitted(&catalog));
        session
            .outfit(buy(SPEED), &mut Scripted::default())
            .expect("bought");
        session
            .outfit(sell(SPEED), &mut Scripted::default())
            .expect("sold");
        assert_eq!(tags(&probe), [201, 202]);
        let sold = &probe.0.borrow()[1];
        assert_eq!(sold.outfits.get(&SPEED), None, "removed");
        assert!(sold.cash > 24_000, "credited: {}", sold.cash);
    }

    #[test]
    fn a_counted_sale_refused_part_way_runs_on_sell_for_each_sale_made() {
        // A -10-ton pod whose OnSell toggles bit 400, three owned with a
        // 50-ton outfit: 30 + 30 - 50 = 10 tons free, so one pod sold
        // leaves none and the second pass is refused. Two passes keep the
        // count even: an OnSell on the refused pass would clear the bit.
        let pod = OutfitId(310);
        let mut catalog = outfitting();
        catalog.outfits.push(OutfitRecord {
            mass: -10,
            on_sell: Script::parse("^b400"),
            ..outfit(310, &[])
        });
        catalog.outfits.push(OutfitRecord {
            mass: 50,
            ..outfit(311, &[])
        });
        let mut session = outfitted(&catalog);
        session.pilot.outfits = BTreeMap::from([(OutfitId(311), 1), (pod, 3)]);
        let free = session.outfitter(&mut Scripted::default()).expect("open");
        assert_eq!(free.free_mass, 10);
        let order = session.outfit_order(sell(pod), 2, &mut Scripted::default());
        assert_eq!(order.run.done, 1);
        assert_eq!(
            order
                .run
                .stopped
                .map(|stopped| (stopped.pass, stopped.refusal)),
            Some((1, OutfitRefusal::NegativeFreeMass))
        );
        assert!(session.control_bit(bit(400)), "toggled once: one sale");
        assert_eq!(session.pilot().owned(pod), 2);
    }

    #[test]
    fn by_the_engine_on_sell_bits_that_list_an_outfit_reset_the_selection() {
        // The booster's OnSell sets bit 7, which lists the hidden 305, so
        // its first sale changes the listing.
        let mut catalog = hooked_outfit(outfitting_on_bit_7(), SPEED, "", "b7");
        catalog.outfits.push(outfit(128, &[]));
        let sold = |source| {
            let mut session = outfitted(&catalog).with_rules(
                crate::rulebook::Rulebook::default()
                    .with_override(crate::RuleKey::RelistedSale, source),
            );
            session.pilot.outfits = BTreeMap::from([(SPEED, 2), (BLASTER, 1)]);
            let listed = session.outfitter(&mut Scripted::default()).expect("open");
            assert_eq!(listed.row(OutfitId(305)), None, "hidden until bit 7");
            let order = session.outfit_order(sell(SPEED), 2, &mut Scripted::default());
            assert_eq!((order.run.done, order.run.stopped), (2, None));
            assert!(session.control_bit(bit(7)));
            (session.pilot().owned(SPEED), session.pilot().owned(BLASTER))
        };
        assert_eq!(sold(crate::RuleSource::Engine), (1, 0), "a blaster sold");
        assert_eq!(sold(crate::RuleSource::Bible), (0, 1), "two boosters");
    }

    #[test]
    fn by_the_engine_on_purchase_bits_that_list_an_outfit_reset_the_selection() {
        // The booster's OnPurchase sets bit 7, which lists the hidden 305.
        let catalog = hooked_outfit(outfitting_on_bit_7(), SPEED, "b7", "");
        let bought = |source| {
            let mut session = outfitted(&catalog).with_rules(
                crate::rulebook::Rulebook::default()
                    .with_override(crate::RuleKey::RelistedSelection, source),
            );
            let listed = session.outfitter(&mut Scripted::default()).expect("open");
            assert_eq!(listed.row(OutfitId(305)), None, "hidden until bit 7");
            let order = session.outfit_order(buy(SPEED), 1, &mut Scripted::default());
            assert_eq!(order.run.done, 1);
            let relisted = session.outfitter(&mut Scripted::default()).expect("open");
            assert!(relisted.row(OutfitId(305)).is_some(), "listed by bit 7");
            order.after
        };
        assert_eq!(
            bought(crate::RuleSource::Engine),
            Reselection::Reset(FIRST_OUTFIT)
        );
        assert_eq!(bought(crate::RuleSource::Bible), Reselection::Kept);
    }

    #[test]
    fn by_the_engine_a_buy_that_hides_the_only_row_leaves() {
        // The only outfit listed hides itself once bought: its OnPurchase
        // sets bit 7, its Availability is `!b7`, and it is taken away
        // after purchase, so nothing is listed.
        let catalog = FakePilotCatalog {
            outfits: vec![OutfitRecord {
                flags: OutfitFlags::REMOVE_AFTER_PURCHASE | OutfitFlags::HIDE_UNLESS_AVAILABLE,
                availability: crate::control::Test::parse("!b7"),
                on_purchase: Script::parse("b7"),
                ..outfit(310, &[])
            }],
            ..outfitting()
        };
        let mut session = outfitted(&catalog);
        let listed = session.outfitter(&mut Scripted::default()).expect("open");
        assert_eq!(listed.rows.len(), 1);
        let order = session.outfit_order(buy(OutfitId(310)), 1, &mut Scripted::default());
        assert_eq!(order.run.done, 1);
        assert_eq!(order.after, Reselection::Leave);
        let emptied = session.outfitter(&mut Scripted::default()).expect("open");
        assert_eq!(emptied.rows, []);
    }

    /// [`outfitting_on_bit_7`] also selling a blaster (128) whose
    /// `OnSell` is `on_sell`, owning `blasters` of it, by `source`'s
    /// reading of `RelistedSale`.
    fn selling_blasters(on_sell: &str, blasters: u16, source: crate::RuleSource) -> Session {
        let mut catalog = outfitting_on_bit_7();
        catalog.outfits.push(OutfitRecord {
            on_sell: Script::parse(on_sell),
            ..outfit(128, &[])
        });
        let mut session = outfitted(&catalog).with_rules(
            crate::rulebook::Rulebook::default()
                .with_override(crate::RuleKey::RelistedSale, source),
        );
        session.pilot.outfits = BTreeMap::from([(BLASTER, blasters)]);
        session
    }

    #[test]
    fn by_the_engine_a_sale_that_relists_and_grants_itself_back_ends_at_the_restart_ceiling() {
        // Each sale toggles bit 7, which lists or hides 305, and grants
        // the blaster back: the original would restart forever. The port
        // ends the order once it has restarted 512 times, on the 513th
        // sale.
        let mut session = selling_blasters("^b7 G128", 1, crate::RuleSource::Engine);
        let order = session.outfit_order(sell(BLASTER), 2, &mut Scripted::default());
        assert_eq!((order.run.done, order.run.stopped), (513, None));
        assert_eq!(session.pilot().owned(BLASTER), 1);
        assert!(session.control_bit(bit(7)), "toggled 513 times");
        let mut bible = selling_blasters("^b7 G128", 1, crate::RuleSource::Bible);
        let order = bible.outfit_order(sell(BLASTER), 2, &mut Scripted::default());
        assert_eq!((order.run.done, order.run.stopped), (2, None));
    }

    #[test]
    fn by_the_engine_a_random_on_sell_that_revisits_a_state_still_ends_as_the_original() {
        // Each sale toggles bit 7, relisting, and draws: a 1 grants the
        // blaster back, a 0 sets bit 8. Drawing 1, 1, 1, 0: the third sale
        // brings back the counts, listing and bits of the first restart,
        // yet the fourth sells the last blaster, and the restart after it
        // finds none, as the original's order does.
        let mut session = selling_blasters("^b7 R(G128 b8)", 1, crate::RuleSource::Engine);
        let mut chance = Scripted::default().and_rolling(&[1, 1, 1, 0]);
        let order = session.outfit_order(sell(BLASTER), 2, &mut chance);
        assert_eq!(order.run.done, 4);
        assert_eq!(
            order
                .run
                .stopped
                .map(|stopped| (stopped.pass, stopped.refusal)),
            Some((1, OutfitRefusal::NoneOwned))
        );
        assert_eq!(session.pilot().owned(BLASTER), 0);
        assert!(!session.control_bit(bit(7)), "toggled four times");
        assert!(session.control_bit(bit(8)), "the last draw");
    }

    #[test]
    fn by_the_engine_a_sale_that_grants_back_more_than_it_sells_ends_at_the_restart_ceiling() {
        // Each sale toggles bit 7, relisting, and grants two blasters
        // back, so the counts owned grow and no restart ever repeats: the
        // order ends once 512 restarts are made, on the 513th sale.
        let mut session = selling_blasters("^b7 G128 G128", 1, crate::RuleSource::Engine);
        let order = session.outfit_order(sell(BLASTER), 2, &mut Scripted::default());
        assert_eq!((order.run.done, order.run.stopped), (513, None));
        assert_eq!(session.pilot().owned(BLASTER), 1 + 513);
    }

    #[test]
    fn by_the_engine_a_relisting_that_alternates_its_listing_sells_until_none_is_left() {
        // Each sale toggles bit 7, so every sale relists, the listing
        // alternating, and the passes restart on the blaster. The counts
        // owned fall with each sale, so the order sells all four blasters,
        // well under the restart ceiling, and the pass after the last
        // finds none.
        let mut session = selling_blasters("^b7", 4, crate::RuleSource::Engine);
        let order = session.outfit_order(sell(BLASTER), 3, &mut Scripted::default());
        assert_eq!(order.run.done, 4);
        assert_eq!(
            order
                .run
                .stopped
                .map(|stopped| (stopped.pass, stopped.refusal)),
            Some((1, OutfitRefusal::NoneOwned))
        );
        assert_eq!(session.pilot().owned(BLASTER), 0);
    }

    // The shipyard.

    /// [`shipbuying`] with ship 128's `OnRetire` and `OnPurchase`, and
    /// ship 129's, as given.
    fn hooked_ships(retire_128: &str, purchase_129: &str) -> FakePilotCatalog {
        let mut catalog = shipbuying();
        for record in &mut catalog.ship_records {
            if record.id == ShipId(128) {
                record.on_retire = Script::parse(retire_128);
                record.on_purchase = Script::parse("S290");
            } else {
                record.on_retire = Script::parse("S291");
                record.on_purchase = Script::parse(purchase_129);
            }
        }
        catalog
    }

    #[test]
    fn buying_a_ship_retires_the_old_class_then_purchases_the_new() {
        let catalog = hooked_ships("S210", "S211");
        let (mut session, probe) = probed(outfitted(&catalog));
        session.pilot.outfits.insert(SPEED, 1);
        session
            .buy_ship(NEW, "Kestrel", &mut Scripted::default())
            .expect("bought");
        assert_eq!(tags(&probe), [210, 211], "each once, in order");
        let snaps = probe.0.borrow().clone();
        assert_eq!(snaps[0].ship, ShipId(128), "retired as the old class");
        assert_eq!(snaps[0].outfits.get(&SPEED), Some(&1), "its outfits");
        assert_eq!(snaps[1].ship, NEW);
        assert_eq!(snaps[1].outfits.get(&TANK), Some(&1), "the new defaults");
        assert_eq!(snaps[1].outfits.get(&SPEED), None);
        assert_eq!(snaps[1].cash, session.pilot().cash(), "paid by then");
        assert!(snaps[1].cash < 25_000);
    }

    #[test]
    fn a_refused_ship_purchase_runs_no_hook() {
        let catalog = hooked_ships("S210", "S211");
        let (mut session, probe) = probed(outfitted(&catalog));
        session.pilot.cash = 0;
        assert!(
            session
                .buy_ship(NEW, "Kestrel", &mut Scripted::default())
                .is_err()
        );
        assert_eq!(
            session.buy_ship(ShipId(999), "Kestrel", &mut Scripted::default()),
            Err(ShipRefusal::NotListed)
        );
        assert!(tags(&probe).is_empty());
    }

    /// [`hooked_ships`] where ship 129's `OnPurchase` grants the paint
    /// (outfit 308).
    fn painting_purchase() -> FakePilotCatalog {
        let mut catalog = hooked_ships("", "G308");
        catalog
            .outfits
            .push(outfit(308, &[(crate::outfit_effects::PAINT, 0x7C00)]));
        catalog
    }

    #[test]
    fn by_the_engine_a_paint_granted_by_on_purchase_is_cleared() {
        let catalog = painting_purchase();
        let mut session = outfitted(&catalog);
        session.pilot.paint = Rgb15::of(0x1F);
        session
            .buy_ship(NEW, "Kestrel", &mut Scripted::default())
            .expect("bought");
        assert_eq!(session.pilot.paint, None);
        assert!(session.take_save_due());
    }

    #[test]
    fn by_the_other_reading_a_paint_granted_by_on_purchase_stays() {
        let catalog = painting_purchase();
        let rules =
            Rulebook::default().with_override(RuleKey::PurchasePaintOrder, RuleSource::Bible);
        let mut session = outfitted(&catalog).with_rules(rules);
        session.pilot.paint = Rgb15::of(0x1F);
        session
            .buy_ship(NEW, "Kestrel", &mut Scripted::default())
            .expect("bought");
        assert_eq!(session.pilot.paint, Rgb15::of(0x7C00));
        let mut plain = outfitted(&hooked_ships("", "")).with_rules(rules);
        plain.pilot.paint = Rgb15::of(0x1F);
        plain
            .buy_ship(NEW, "Kestrel", &mut Scripted::default())
            .expect("bought");
        assert_eq!(plain.pilot.paint, None, "the old paint still goes");
    }

    #[test]
    fn on_purchase_runs_before_the_reserves_are_filled() {
        let catalog = hooked_ships("", "S211");
        let (mut session, probe) = probed(outfitted(&catalog));
        session.pilot.reserves.shield.now = 1.0;
        session
            .buy_ship(NEW, "Kestrel", &mut Scripted::default())
            .expect("bought");
        assert_eq!(tags(&probe), [211]);
        let snaps = probe.0.borrow().clone();
        assert_eq!(
            snaps[0].reserves.shield.now, 1.0,
            "OnPurchase sees the old shield, held within the new most"
        );
        assert_eq!(session.reserves(), Reserves::full(80.0, 45.0, 400.0));
    }

    #[test]
    fn a_d_in_on_purchase_leaves_the_shield_full_without_a_surplus() {
        let mut catalog = hooked_ships("", "D302");
        for record in &mut catalog.ship_records {
            if record.id == NEW {
                record.defaults.push((OutfitId(302), 1));
            }
        }
        for (ship, defaults) in &mut catalog.defaults {
            if *ship == NEW {
                defaults.push((OutfitId(302), 1));
            }
        }
        let mut session = outfitted(&catalog);
        session
            .buy_ship(NEW, "Kestrel", &mut Scripted::default())
            .expect("bought");
        assert_eq!(session.pilot().owned(OutfitId(302)), 0, "removed");
        assert_eq!(session.reserves().shield.max, 80.0);
        assert_eq!(session.reserves().shield.now, 80.0, "no surplus kept");
        assert_eq!(session.reserves(), session.stats().full());
    }

    #[test]
    fn a_change_to_a_smaller_class_in_on_purchase_leaves_its_reserves_full() {
        let catalog = hooked_ships("", "C128");
        let mut session = outfitted(&catalog);
        session
            .buy_ship(NEW, "Kestrel", &mut Scripted::default())
            .expect("bought");
        assert_eq!(session.ship(), ShipId(128));
        assert_eq!(session.reserves(), Reserves::full(30.0, 45.0, 400.0));
    }

    #[test]
    fn a_change_to_a_larger_class_in_on_purchase_fills_its_reserves() {
        let mut catalog = hooked_ships("", "E130");
        let heavy = catalog
            .ship_records
            .iter()
            .find(|record| record.id == NEW)
            .expect("ship 129")
            .fields;
        let larger = crate::handling::ShipFields {
            armor: 90,
            fuel: 500,
            ..heavy
        };
        catalog.ships.push((ShipId(130), Ok(larger)));
        catalog.ship_records.push(crate::testkit::ship(130, larger));
        let mut session = outfitted(&catalog);
        session
            .buy_ship(NEW, "Kestrel", &mut Scripted::default())
            .expect("bought");
        assert_eq!(session.ship(), ShipId(130));
        assert_eq!(session.reserves().armor.now, 90.0);
        assert_eq!(session.reserves(), session.stats().full());
    }

    #[test]
    fn the_fill_comes_after_the_paint_under_either_paint_order() {
        for source in [RuleSource::Engine, RuleSource::Bible] {
            let rules = Rulebook::default().with_override(RuleKey::PurchasePaintOrder, source);
            let mut session = outfitted(&hooked_ships("", "C128")).with_rules(rules);
            session
                .buy_ship(NEW, "Kestrel", &mut Scripted::default())
                .expect("bought");
            assert_eq!(
                session.reserves(),
                Reserves::full(30.0, 45.0, 400.0),
                "{source:?}"
            );
        }
    }

    // Capture.

    /// `catalog` with ship 128's `OnRetire` and `OnCapture` and ship 129's
    /// (the trader's) as given.
    fn hooked_capture(
        mut catalog: FakePilotCatalog,
        retire_128: &str,
        capture_129: &str,
    ) -> FakePilotCatalog {
        for record in &mut catalog.ship_records {
            match record.id.0 {
                128 => {
                    record.on_retire = Script::parse(retire_128);
                    record.on_capture = Script::parse("S292");
                    record.on_purchase = Script::parse("S293");
                }
                129 => {
                    record.on_retire = Script::parse("S294");
                    record.on_capture = Script::parse(capture_129);
                    record.on_purchase = Script::parse("S295");
                }
                _ => {}
            }
        }
        catalog
    }

    /// [`aboard`] `catalog`'s trader, probed, the player's ship 128 then
    /// losing its crew, so a capture joins the fleet at once.
    fn aboard_crewless(catalog: &FakePilotCatalog) -> (Session, Rc<Probe>) {
        let mut session = aboard(catalog);
        for record in &mut session.statics_mut().ships {
            if record.id == ShipId(128) {
                record.crew = 0;
            }
        }
        probed(session)
    }

    #[test]
    fn a_capture_taken_as_an_escort_runs_on_capture_once_before_it_joins() {
        let catalog = hooked_capture(boardable(), "S220", "S221");
        let (mut session, probe) = aboard_crewless(&catalog);
        assert_eq!(
            take(&mut session, Take::Capture, &[0, 1]).0,
            Taken::Escorted
        );
        assert_eq!(tags(&probe), [221]);
        assert_eq!(probe.0.borrow()[0].escorts, 0, "not yet joined");
        assert_eq!(session.pilot().escorts().len(), 1);
    }

    #[test]
    fn a_capture_that_fails_trips_or_finds_the_fleet_full_runs_no_hook() {
        let catalog = hooked_capture(boardable(), "S220", "S221");
        for (draws, taken) in [
            (&[44][..], Taken::CaptureFailed),
            (&[0, 0][..], Taken::Tripped),
        ] {
            let (mut session, probe) = aboard_crewless(&catalog);
            assert_eq!(take(&mut session, Take::Capture, draws).0, taken);
            assert!(tags(&probe).is_empty(), "{taken:?}");
        }
        let (mut session, probe) = aboard_crewless(&catalog);
        let escort = session.pilot().escorts().first().copied();
        let filler = escort.unwrap_or(crate::pilot::Escort {
            ship: ShipId(129),
            reserves: crate::reserves::Reserves::default(),
            order: None,
            carried: false,
            wage: None,
            person: None,
        });
        session.pilot.escorts = vec![filler; crate::board::MAX_ESCORTS];
        assert_eq!(
            take(&mut session, Take::Capture, &[0, 1]).0,
            Taken::FleetFull
        );
        assert!(tags(&probe).is_empty());
    }

    #[test]
    fn a_capture_awaiting_assignment_runs_on_capture_when_used_as_an_escort() {
        let catalog = hooked_capture(boardable(), "S220", "S221");
        let (mut session, probe) = probed(aboard(&catalog));
        assert_eq!(
            take(&mut session, Take::Capture, &[0, 1]).0,
            Taken::Captured
        );
        assert!(tags(&probe).is_empty(), "it waits for the assignment");
        assert_eq!(
            session.assign(Assignment::Escort, &mut Draws::of(&[])),
            Some(Assigned::Escort)
        );
        assert_eq!(tags(&probe), [221]);
        assert_eq!(probe.0.borrow()[0].escorts, 0, "before it joins");
    }

    /// [`kitted`] with these hooks, captured and awaiting its assignment,
    /// owning one persistent outfit (400) and one not (401).
    fn captured_kitted(
        retire_128: &str,
        capture_129: &str,
        rules: Rulebook,
    ) -> (Session, Rc<Probe>) {
        let catalog = hooked_capture(kitted(), retire_128, capture_129);
        let mut session = captured(&catalog).with_rules(rules);
        session.pilot.outfits.insert(OutfitId(400), 1);
        session.pilot.outfits.insert(OutfitId(401), 1);
        probed(session)
    }

    #[test]
    fn use_as_my_ship_retires_the_old_class_then_captures_the_new_before_the_swap() {
        let (mut session, probe) = captured_kitted("S222", "S223", Rulebook::default());
        assert_eq!(
            session.assign(Assignment::MyShip, &mut Draws::of(&[])),
            Some(Assigned::MyShip)
        );
        assert_eq!(tags(&probe), [222, 223]);
        for snap in probe.0.borrow().iter() {
            assert_eq!(snap.ship, ShipId(128), "the old class still flown");
            assert_eq!(
                snap.outfits.get(&OutfitId(401)),
                Some(&1),
                "before the swap"
            );
            assert_eq!(
                snap.escorts, 1,
                "the old ship already an escort (@0x41188-0x4156f)"
            );
        }
        assert_eq!(session.ship(), ShipId(129));
    }

    #[test]
    fn by_the_other_reading_use_as_my_ship_runs_its_hooks_after_the_swap() {
        let rules = Rulebook::default().with_override(RuleKey::CaptureHookOrder, RuleSource::Bible);
        let (mut session, probe) = captured_kitted("S222", "S223", rules);
        session.assign(Assignment::MyShip, &mut Draws::of(&[]));
        assert_eq!(tags(&probe), [222, 223]);
        for snap in probe.0.borrow().iter() {
            assert_eq!(snap.ship, ShipId(128), "the class not yet changed");
            assert_eq!(snap.outfits.get(&OutfitId(401)), None, "after the swap");
            assert_eq!(
                snap.outfits.get(&OutfitId(402)),
                Some(&1),
                "the new defaults"
            );
        }
    }

    #[test]
    fn a_non_persistent_outfit_on_capture_grants_is_stripped_by_the_engine_and_kept_otherwise() {
        for (source, kept) in [(RuleSource::Engine, 0), (RuleSource::Bible, 1)] {
            let rules = Rulebook::default().with_override(RuleKey::CaptureHookOrder, source);
            let (mut session, _) = captured_kitted("", "G401 G400", rules);
            session.pilot.outfits.remove(&OutfitId(401));
            session.pilot.outfits.remove(&OutfitId(400));
            session.assign(Assignment::MyShip, &mut Draws::of(&[]));
            assert_eq!(session.pilot().owned(OutfitId(401)), kept, "{source:?}");
            assert_eq!(
                session.pilot().owned(OutfitId(400)),
                1,
                "persistent: {source:?}"
            );
        }
    }

    #[test]
    fn an_abandoned_prize_runs_no_hook() {
        let (mut session, probe) = captured_kitted("S222", "S223", Rulebook::default());
        session
            .statics_mut()
            .ships
            .retain(|record| record.id != ShipId(129));
        assert_eq!(
            session.assign(Assignment::MyShip, &mut Draws::of(&[])),
            Some(Assigned::Abandoned)
        );
        assert!(tags(&probe).is_empty());
    }

    #[test]
    fn on_capture_draws_before_the_fuel() {
        let (mut session, _) = captured_kitted("", "R(b1 b2)", Rulebook::default());
        let mut chance = Draws::of(&[1, 7]);
        session.assign(Assignment::MyShip, &mut chance);
        assert_eq!(chance.asked, [2, 300], "the roll, then the fuel");
        assert!(session.control_bit(bit(1)));
        assert_eq!(session.reserves().fuel.now, 7.0);
    }

    #[test]
    fn by_the_other_reading_on_capture_still_draws_before_the_fuel() {
        let rules = Rulebook::default().with_override(RuleKey::CaptureHookOrder, RuleSource::Bible);
        let (mut session, _) = captured_kitted("", "R(b1 b2)", rules);
        let mut chance = Draws::of(&[1, 7]);
        session.assign(Assignment::MyShip, &mut chance);
        assert_eq!(chance.asked, [2, 300], "the roll, then the fuel");
        assert!(session.control_bit(bit(1)));
        assert_eq!(session.reserves().fuel.now, 7.0);
    }

    // A new pilot.

    /// [`catalog`] whose `chär`'s `OnStart` is `on_start`, and whose ship
    /// 128 has a record with `OnPurchase` `S31` and a default fuel tank.
    fn starting_with(on_start: &str) -> FakePilotCatalog {
        let base = catalog();
        let mut ship = crate::testkit::ship(128, crate::testkit::FAST);
        ship.on_purchase = Script::parse("S231");
        FakePilotCatalog {
            character: Ok(CharacterStart {
                on_start: Script::parse(on_start),
                ..base.character.clone().expect("a chär")
            }),
            ship_records: vec![ship],
            defaults: vec![(ShipId(128), vec![(OutfitId(200), 1)])],
            outfits: vec![outfit(200, &[])],
            ..base
        }
    }

    fn new_pilot(catalog: &FakePilotCatalog) -> Session {
        Session::fly(catalog, Pilot::new(catalog, "Ada").expect("starts")).expect("flies")
    }

    #[test]
    fn beginning_runs_the_chärs_on_start_once_after_the_pilot_flies() {
        let catalog = starting_with("S230 b5");
        let (mut session, probe) = probed(new_pilot(&catalog));
        assert!(tags(&probe).is_empty(), "flying alone runs nothing");
        session.begin(&catalog, &mut Scripted::default());
        assert_eq!(tags(&probe), [230], "once, and no starting OnPurchase");
        assert_eq!(probe.0.borrow()[0].outfits.get(&OutfitId(200)), Some(&1));
        assert!(session.control_bit(bit(5)));
        assert!(session.take_save_due());
    }

    /// Records the explored systems each time it sees an operator.
    #[derive(Debug, Default)]
    struct ExploredSeen(RefCell<Vec<Vec<i16>>>);

    impl SetOpObserver for ExploredSeen {
        fn saw(&self, _op: &SetOp, session: &Session) {
            let explored = session.pilot().explored().map(|system| system.0);
            self.0.borrow_mut().push(explored.collect());
        }
    }

    #[test]
    fn begin_explores_the_start_systems_visible_neighbours() {
        // 130 lists 131, at 127's place, then 133, then 132, hidden; 127
        // is hidden too. `X999` explores nothing: the observer sees it.
        let mut catalog = starting_with("X999");
        catalog.star_map[0].links = [131, 133, 132].map(crate::catalog::SystemId).to_vec();
        catalog
            .star_map
            .push(crate::testkit::star(127, (600.0, 0.0), &[]));
        for (hidden, test) in [(127, "b1"), (132, "b2")] {
            catalog.visibility.push(crate::catalog::SystemVisibility {
                id: crate::catalog::SystemId(hidden),
                visibility: crate::control::Test::parse(test),
            });
        }
        for source in RuleSource::ALL {
            let seen = Rc::new(ExploredSeen::default());
            let mut session = new_pilot(&catalog)
                .with_rules(Rulebook::default().with_override(RuleKey::ExploredByPlace, source))
                .with_set_op_observer(seen.clone());
            session.begin(&catalog, &mut Scripted::default());
            let explored: Vec<i16> = session.pilot().explored().map(|system| system.0).collect();
            let (on_start, after): (&[i16], &[i16]) = match source {
                RuleSource::Engine => (&[127, 130, 133], &[127, 130, 131, 133]),
                RuleSource::Bible => (&[130, 131, 133], &[130, 131, 133]),
            };
            assert_eq!(*seen.0.borrow(), [on_start.to_vec()], "{source:?}: OnStart");
            assert_eq!(explored, after, "{source:?}: after begin's rebuild");
        }
    }

    /// [`starting_with`] whose `chär` gives government 128 a status of
    /// 20, its systems 130 of 128, 131 of its ally 129 (`InitialRec` 5),
    /// 132 of its enemy 130 and 133 independent.
    fn with_a_status() -> FakePilotCatalog {
        use crate::catalog::{GovtId, GovtRecord, SystemId};
        let mut catalog = starting_with("");
        let start = catalog.character.as_mut().expect("a chär");
        start.legal = [Some((GovtId(128), 20)), None, None, None];
        for (system, govt) in [(130, Some(128)), (131, Some(129)), (132, Some(130))] {
            let star = catalog
                .star_map
                .iter_mut()
                .find(|star| star.id == SystemId(system))
                .expect("a system");
            star.govt = govt.map(GovtId);
        }
        catalog.govts = vec![
            GovtRecord {
                classes: [1, -1, -1, -1],
                ..crate::testkit::govt(128)
            },
            GovtRecord {
                allies: [1, -1, -1, -1],
                initial_rec: 5,
                ..crate::testkit::govt(129)
            },
            GovtRecord {
                enemies: [1, -1, -1, -1],
                ..crate::testkit::govt(130)
            },
        ];
        catalog
    }

    fn records(session: &Session) -> ([i16; 4], [i16; 3]) {
        use crate::catalog::{GovtId, SystemId};
        let pilot = session.pilot();
        (
            [130, 131, 132, 133].map(|system| pilot.system_record(SystemId(system))),
            [128, 129, 130].map(|govt| pilot.legal_record(GovtId(govt))),
        )
    }

    #[test]
    fn by_the_engine_beginning_spreads_the_chärs_statuses_to_allies_and_enemies() {
        let catalog = with_a_status();
        let mut session = new_pilot(&catalog);
        assert_eq!(
            records(&session),
            ([20, 5, 0, 0], [20, 0, 0]),
            "flying alone derives each system from its government, as a load does"
        );
        session.begin(&catalog, &mut Scripted::default());
        assert_eq!(
            records(&session),
            ([20, 20, -20, 0], [20, 20, -20]),
            "both stores: the ally's and the enemy's too"
        );
    }

    #[test]
    fn by_the_bible_a_chärs_status_stays_with_its_own_government() {
        let catalog = with_a_status();
        let rules =
            Rulebook::default().with_override(RuleKey::CharacterStatusSpread, RuleSource::Bible);
        let mut session = new_pilot(&catalog).with_rules(rules);
        session.begin(&catalog, &mut Scripted::default());
        assert_eq!(records(&session), ([20, 5, 0, 0], [20, 0, 0]));
    }

    #[test]
    fn on_start_sees_the_starting_system_explored() {
        let catalog = starting_with("X131");
        let mut session = new_pilot(&catalog);
        session.begin(&catalog, &mut Scripted::default());
        assert!(session.pilot().has_explored(crate::catalog::SystemId(130)));
        assert!(session.pilot().has_explored(crate::catalog::SystemId(131)));
    }

    #[test]
    fn by_the_other_reading_the_starting_ship_runs_its_on_purchase_before_on_start() {
        let catalog = starting_with("S230");
        let rules =
            Rulebook::default().with_override(RuleKey::StartShipPurchase, RuleSource::Bible);
        let (mut session, probe) = probed(new_pilot(&catalog).with_rules(rules));
        session.begin(&catalog, &mut Scripted::default());
        assert_eq!(tags(&probe), [231, 230]);
    }

    #[test]
    fn a_blank_on_start_or_an_unreadable_chär_does_nothing() {
        let catalog = starting_with("");
        let mut session = new_pilot(&catalog);
        session.begin(&catalog, &mut Scripted::default());
        assert!(!session.take_save_due());
        assert_eq!(session.take_script_notes(), []);
        let unreadable = FakePilotCatalog {
            character: Err(StartError::NoCharacter),
            ..starting_with("b5")
        };
        let mut session = new_pilot(&catalog);
        session.begin(&unreadable, &mut Scripted::default());
        assert!(!session.control_bit(bit(5)));
        assert!(!session.take_save_due());
    }

    #[test]
    fn begin_puts_the_new_pilots_ship_at_50_50_at_rest_facing_up() {
        let catalog = starting_with("");
        let mut session = new_pilot(&catalog);
        session.player.heading = 90.0;
        session.player.velocity = Vec2::new(3.0, 4.0);
        session.begin(&catalog, &mut Scripted::default());
        let player = session.player();
        assert_eq!(player.position, Vec2::new(50.0, 50.0));
        assert_eq!(player.velocity, Vec2::ZERO);
        assert_eq!(player.heading, 0.0);
        assert_eq!(session.landed(), None, "in space, not landed");
    }

    #[test]
    fn begin_records_the_stellar_nearest_50_50_as_last_landed() {
        // 130 holds 128 at (30, -40) and 129 at (2000, 0).
        let catalog = starting_with("");
        let mut session = new_pilot(&catalog);
        assert_eq!(session.pilot().stellar(), None);
        session.begin(&catalog, &mut Scripted::default());
        assert_eq!(session.pilot().stellar(), Some(StellarId(128)));
    }

    #[test]
    fn begin_records_the_first_listed_stellar_when_none_can_be_landed_on() {
        let mut catalog = starting_with("");
        catalog.sites = vec![(
            SystemId(130),
            [(129, 2000.0), (128, 30.0)]
                .map(|(id, x)| crate::catalog::LandingSite {
                    flags: 0,
                    ..crate::testkit::planet(id, x, 0.0)
                })
                .to_vec(),
        )];
        let mut session = new_pilot(&catalog);
        session.begin(&catalog, &mut Scripted::default());
        assert_eq!(session.pilot().stellar(), Some(StellarId(129)));
    }

    #[test]
    fn begin_picks_the_stellar_nearest_where_it_puts_the_ship_not_the_centre() {
        // 128 is nearest the centre, 129 nearest (50, 50).
        let mut catalog = starting_with("");
        catalog.sites = vec![(
            SystemId(130),
            vec![
                crate::testkit::planet(128, -20.0, -20.0),
                crate::testkit::planet(129, 60.0, 60.0),
            ],
        )];
        let mut session = new_pilot(&catalog);
        assert_eq!(session.player().position, Vec2::ZERO, "flown at the centre");
        session.begin(&catalog, &mut Scripted::default());
        assert_eq!(session.pilot().stellar(), Some(StellarId(129)));
    }

    #[test]
    fn begin_falls_back_to_the_first_stellar_listed_though_it_is_not_shown() {
        // 130 lists 127 first, but 129, visible and lower, owns it, so 130
        // doesn't show it; none of the three can be landed on.
        let mut catalog = starting_with("");
        let unlandable = |id| crate::catalog::LandingSite {
            flags: 0,
            ..crate::testkit::planet(id, 0.0, 0.0)
        };
        catalog.sites = vec![
            (SystemId(129), vec![unlandable(127)]),
            (SystemId(130), [127, 128, 129].map(unlandable).to_vec()),
        ];
        catalog.systems.push(SystemId(129));
        catalog
            .star_map
            .push(crate::testkit::star(129, (-600.0, 600.0), &[]));
        let mut session = new_pilot(&catalog);
        session.begin(&catalog, &mut Scripted::default());
        let shown: Vec<StellarId> = session.sites.iter().map(|site| site.id).collect();
        assert_eq!(shown, [StellarId(128), StellarId(129)], "127 not shown");
        assert_eq!(session.pilot().stellar(), Some(StellarId(127)));
    }

    #[test]
    fn begin_records_no_stellar_when_the_system_lists_none() {
        let mut catalog = starting_with("");
        catalog.sites = vec![(SystemId(130), Vec::new())];
        let mut session = new_pilot(&catalog);
        session.begin(&catalog, &mut Scripted::default());
        assert_eq!(session.pilot().stellar(), None);
    }

    #[test]
    fn a_ship_with_no_record_runs_no_hook() {
        let catalog = FakePilotCatalog {
            ship_records: Vec::new(),
            ..starting_with("")
        };
        let rules =
            Rulebook::default().with_override(RuleKey::StartShipPurchase, RuleSource::Bible);
        let (mut session, probe) = probed(new_pilot(&catalog).with_rules(rules));
        session.begin(&catalog, &mut Scripted::default());
        assert!(tags(&probe).is_empty());
    }
}
