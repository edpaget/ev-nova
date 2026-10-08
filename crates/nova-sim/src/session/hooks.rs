//! The set-expression hooks on the session: the `chär`'s `OnStart`, an
//! `oütf`'s `OnPurchase` and `OnSell`, and a `shïp`'s `OnPurchase`,
//! `OnCapture` and `OnRetire`, each run where the original runs it
//! through `_EvalMissionBitSetString` (@0x99dc5).
//!
//! **Running one.** [`Session::run_script`] runs a record's [`Script`]
//! as [`Session::run_set`] runs a set expression: the bits written, `R(...)`
//! drawn on the caller's [`Chance`], the other operators handed to the
//! session's registry (by default `G`, `D`, `X`, `C`, `E`, `H`, `T`, `M`,
//! `N`, `Q` and `P`;
//! any other is skipped
//! and told once a session as a [`ScriptNote`](crate::ScriptNote)). A
//! blank script, or one that did not parse, does nothing at all: no save
//! is due and nothing is told.
//!
//! **Where each runs**, in the original's order:
//!
//! | Hook | Trigger | Order |
//! |---|---|---|
//! | `chär` `OnStart` | [`Session::begin`], once on a new pilot (`_DoNewPilot` @0x19345) | after the pilot is set up and flying (ship, default items, system explored), before its first save; a pilot opened from a save never runs it. By the engine no `shïp` `OnPurchase` runs for the starting ship; by the other reading of [`RuleKey::StartShipPurchase`](crate::RuleKey::StartShipPurchase) it runs once, right before `OnStart` |
//! | `oütf` `OnPurchase` | [`Session::outfit`], each unit bought (`_DoOutfitDialog` @0x5c2cb) | after the cash and the grant, even when the grant added nothing (a map, a paint); then an outfit removed after purchase goes, and the ship is refitted |
//! | `oütf` `OnSell` | [`Session::outfit`], each unit sold (@0x5cff6) | after one is removed and the cash credited, before the refit |
//! | `shïp` `OnRetire` | [`Session::buy_ship`], the ship traded in (`_DoShipyardDialog` @0x5ecff) | first, the old class still flown and its outfits still owned |
//! | `shïp` `OnPurchase` | [`Session::buy_ship`], the ship bought (@0x5f00d) | last: the new class, cash and outfits in place. By the engine the paint is cleared after it (@0x5f022), so a paint it grants is lost; by the other reading of [`RuleKey::PurchasePaintOrder`](crate::RuleKey::PurchasePaintOrder) the paint is cleared before it |
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
use crate::catalog::{PilotCatalog, ShipId};
use crate::chance::Chance;
use crate::control::Script;
use crate::rulebook::{RuleKey, RuleSource, Rulebook};

/// The disputed orders of the set-expression hooks that the session
/// follows (see [`Session::with_hook_rules`]): the engine's by default.
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
    /// This session with the hooks following `rules` where their order is
    /// disputed: the engine's by default.
    #[must_use]
    pub fn with_hook_rules(mut self, rules: HookRules) -> Self {
        // Shim until callers use with_rules (removed in this phase).
        self.rules = self
            .rules
            .with_override(RuleKey::PurchasePaintOrder, rules.purchase_paint_order);
        self.rules = self
            .rules
            .with_override(RuleKey::CaptureHookOrder, rules.capture_hook_order);
        self.rules = self
            .rules
            .with_override(RuleKey::StartShipPurchase, rules.start_ship_purchase);
        self
    }

    /// The rules the hooks follow.
    #[must_use]
    pub fn hook_rules(&self) -> HookRules {
        HookRules::from_rulebook(&self.rules)
    }

    /// Runs `script` on the session (see the module docs), drawing
    /// `R(...)` on `chance`: nothing at all when it is blank or did not
    /// parse.
    pub fn run_script(&mut self, script: &Script, chance: &mut (impl Chance + ?Sized)) {
        if let Some(expr) = script.tree().filter(|expr| !expr.ops.is_empty()) {
            self.run_set(expr, chance);
        }
    }

    /// Runs ship class `ship`'s `hook`, drawing on `chance`: nothing when
    /// the session has no record of it.
    pub(super) fn ship_hook(&mut self, ship: ShipId, hook: ShipHook, chance: &mut dyn Chance) {
        let Some(record) = self.ship_record(ship) else {
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

    /// Begins a new pilot's game: runs the first `chär`'s `OnStart` once,
    /// drawing on `chance` (see the module docs), and nothing when the
    /// `chär` cannot be read. For a pilot just created and flown, before
    /// its first save; never for one opened from a save.
    pub fn begin(&mut self, catalog: &impl PilotCatalog, chance: &mut dyn Chance) {
        let Ok(start) = catalog.first_character() else {
            return;
        };
        if self.hook_rules().start_ship_purchase == RuleSource::Bible {
            self.ship_hook(self.pilot.ship, ShipHook::Purchase, chance);
        }
        self.run_script(&start.on_start, chance);
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
    use crate::catalog::{CharacterStart, OutfitId, StartError};
    use crate::control::{Bit, ScriptNote, SetOp, SetOpHandler, SetOpKind};
    use crate::outfit_effects::Rgb15;
    use crate::outfitter::OutfitRefusal;
    use crate::pilot::Pilot;
    use crate::session::tests::{
        NEW, aboard, boardable, buy, captured, kitted, outfitted, outfitting, outfitting_effects,
        sell, shipbuying, take,
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
    }

    /// Records a [`Snap`] each time an `S<tag>` applies: the hooks are
    /// written with a distinct tag each.
    #[derive(Debug, Default)]
    struct Probe(RefCell<Vec<Snap>>);

    impl SetOpHandler<Session> for Probe {
        fn apply(&self, op: &SetOp, session: &mut Session, _chance: &mut dyn Chance) {
            if let SetOp::StartMission(mission) = op {
                let pilot = session.pilot();
                self.0.borrow_mut().push(Snap {
                    tag: mission.0,
                    ship: pilot.ship,
                    cash: pilot.cash(),
                    outfits: pilot.outfits.clone(),
                    escorts: pilot.escorts().len(),
                    paint: pilot.paint,
                });
            }
        }
    }

    /// `session` with the probe handling `S`, beside Nova's `G`, `D` and
    /// `X`.
    fn probed(session: Session) -> (Session, Rc<Probe>) {
        let probe = Rc::new(Probe::default());
        let registry = crate::nova_set_ops().with(SetOpKind::StartMission, probe.clone());
        (session.with_set_ops(Rc::new(registry)), probe)
    }

    fn tags(probe: &Probe) -> Vec<i16> {
        probe.0.borrow().iter().map(|snap| snap.tag).collect()
    }

    // Running a script.

    #[test]
    fn a_script_runs_as_its_set_expression() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.run_script(&Script::parse("b7 S300"), &mut Scripted::default());
        assert!(session.control_bit(bit(7)));
        assert!(session.take_save_due());
        assert_eq!(
            session.take_script_notes(),
            [ScriptNote::Unhandled(SetOpKind::StartMission)]
        );
    }

    #[test]
    fn a_blank_or_broken_script_does_nothing_at_all() {
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
        for record in &mut session.ships {
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
            assert_eq!(snap.escorts, 0, "the old ship not yet an escort");
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
        session.ships.retain(|record| record.id != ShipId(129));
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
