//! Running a set expression: writing its bits, drawing its `R(...)`, and
//! handing every other operator to a registry.
//!
//! [`execute`] runs a [`SetExpr`]'s operators in order on a target that
//! holds control bits ([`BitStore`]):
//!
//! - `bxxx`, `!bxxx` and `^bxxx` set, clear and toggle the bit here, never
//!   through the registry.
//! - `R(a b)` draws `_Rand(2)` on the caller's [`Chance`]; the original
//!   skips whichever operator would run as the number drawn (0x151f7), so
//!   a draw of 0 runs `b` and a draw of 1 runs `a`, each half the time.
//!   The arm drawn runs as any other operator does.
//! - Every other operator goes to the [`SetOpHandler`] its [`SetOpKind`]
//!   has in the [`SetRegistry`]. One with no handler yet is skipped, never
//!   a panic, and the operators after it still run; its kind is reported
//!   the first time only, as the caller's `reported` set remembers.
//!
//! The later phases and roadmaps that give the operators meaning
//! (missions, outfits, ships, ranks, stellars) register their handlers
//! with [`SetRegistry::with`].

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::rc::Rc;

use nova_data::{SetExpr, SetOp, SetOpKind};

use super::ControlBitSet;
use crate::chance::Chance;

/// A target a set expression runs on: it holds control bits.
pub trait BitStore {
    /// The control bits, to write.
    fn bits_mut(&mut self) -> &mut ControlBitSet;
}

/// What a set operator other than a bit write or `R(...)` does to a
/// target `T`: one registered for its [`SetOpKind`].
pub trait SetOpHandler<T: ?Sized> {
    /// Applies `op` to `target`, rolling on `chance` if it rolls.
    fn apply(&self, op: &SetOp, target: &mut T, chance: &mut dyn Chance);
}

/// The handler each set operator kind has, if any.
pub struct SetRegistry<T: ?Sized> {
    handlers: BTreeMap<SetOpKind, Rc<dyn SetOpHandler<T>>>,
}

impl<T: ?Sized> SetRegistry<T> {
    /// A registry with no handler: every operator but the bit writes and
    /// `R(...)` is skipped.
    #[must_use]
    pub fn new() -> Self {
        Self {
            handlers: BTreeMap::new(),
        }
    }

    /// This registry with `handler` applying every operator of `kind`, in
    /// place of any it had.
    #[must_use]
    pub fn with(mut self, kind: SetOpKind, handler: Rc<dyn SetOpHandler<T>>) -> Self {
        self.handlers.insert(kind, handler);
        self
    }

    /// The kinds with a handler, ascending.
    pub fn kinds(&self) -> impl Iterator<Item = SetOpKind> + '_ {
        self.handlers.keys().copied()
    }
}

impl<T: ?Sized> Default for SetRegistry<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Lists the kinds with a handler.
impl<T: ?Sized> fmt::Debug for SetRegistry<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SetRegistry")
            .field("kinds", &self.handlers.keys().collect::<Vec<_>>())
            .finish()
    }
}

/// Runs `expr` on `target` (see the module docs), drawing `R(...)` on
/// `chance` and handing every other operator to `registry`. Gives the
/// kinds skipped for want of a handler that `reported` did not hold yet,
/// in the order met, and adds them to it.
pub fn execute<T: BitStore + ?Sized>(
    expr: &SetExpr,
    target: &mut T,
    registry: &SetRegistry<T>,
    chance: &mut dyn Chance,
    reported: &mut BTreeSet<SetOpKind>,
) -> Vec<SetOpKind> {
    let mut unhandled = Vec::new();
    for op in &expr.ops {
        run(op, target, registry, chance, reported, &mut unhandled);
    }
    unhandled
}

/// Runs one operator.
fn run<T: BitStore + ?Sized>(
    op: &SetOp,
    target: &mut T,
    registry: &SetRegistry<T>,
    chance: &mut dyn Chance,
    reported: &mut BTreeSet<SetOpKind>,
    unhandled: &mut Vec<SetOpKind>,
) {
    match op {
        SetOp::Set(bit) => target.bits_mut().set(*bit),
        SetOp::Clear(bit) => target.bits_mut().clear(*bit),
        SetOp::Toggle(bit) => target.bits_mut().toggle(*bit),
        SetOp::Random(first, second) => {
            let arm = if chance.roll(2) == 0 { second } else { first };
            run(arm, target, registry, chance, reported, unhandled);
        }
        _ => match registry.handlers.get(&op.kind()) {
            Some(handler) => handler.apply(op, target, chance),
            None => {
                if reported.insert(op.kind()) {
                    unhandled.push(op.kind());
                }
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::control::Bit;
    use crate::testkit::Scripted;

    /// A target holding control bits and a log of what happened to it.
    #[derive(Debug, Default)]
    struct Target {
        bits: ControlBitSet,
        log: Vec<String>,
    }

    impl BitStore for Target {
        fn bits_mut(&mut self) -> &mut ControlBitSet {
            self.log.push("bits".to_owned());
            &mut self.bits
        }
    }

    impl Target {
        fn set(&self) -> Vec<u16> {
            self.bits.iter().map(Bit::get).collect()
        }
    }

    /// Records each operator it applies, on the target's log and its own.
    #[derive(Debug, Default)]
    struct Recording(RefCell<Vec<SetOp>>);

    impl SetOpHandler<Target> for Recording {
        fn apply(&self, op: &SetOp, target: &mut Target, _chance: &mut dyn Chance) {
            self.0.borrow_mut().push(op.clone());
            target.log.push(format!("{op:?}"));
        }
    }

    fn bit(n: u16) -> Bit {
        Bit::new(n).expect("in range")
    }

    fn parse(text: &str) -> SetExpr {
        SetExpr::parse(text).expect("parses")
    }

    /// Runs `text` on `target` with `registry`, rolling `rolls`; gives the
    /// kinds newly unhandled.
    fn run_on(
        text: &str,
        target: &mut Target,
        registry: &SetRegistry<Target>,
        rolls: &[u16],
    ) -> Vec<SetOpKind> {
        execute(
            &parse(text),
            target,
            registry,
            &mut Scripted::rolling(rolls),
            &mut BTreeSet::new(),
        )
    }

    #[test]
    fn the_bit_operators_set_clear_and_toggle() {
        let mut target = Target {
            bits: [bit(2), bit(3)].into_iter().collect(),
            ..Target::default()
        };
        let unhandled = run_on("b1 !b2 ^b3", &mut target, &SetRegistry::new(), &[]);
        assert_eq!(target.set(), [1]);
        assert_eq!(unhandled, []);
        run_on("^b3 ^b1 !b9", &mut target, &SetRegistry::new(), &[]);
        assert_eq!(target.set(), [3]);
    }

    #[test]
    fn r_runs_the_second_arm_on_a_0_and_the_first_on_a_1() {
        let mut target = Target::default();
        let mut chance = Scripted::rolling(&[0]);
        execute(
            &parse("R(b1 b2)"),
            &mut target,
            &SetRegistry::new(),
            &mut chance,
            &mut BTreeSet::new(),
        );
        assert_eq!(target.set(), [2]);
        assert_eq!(chance.sides_asked, [2], "a roll of two sides");
        let mut target = Target::default();
        run_on("R(b1 b2)", &mut target, &SetRegistry::new(), &[1]);
        assert_eq!(target.set(), [1]);
    }

    #[test]
    fn a_registered_handler_gets_its_operator_and_the_target_in_order() {
        let recording = Rc::new(Recording::default());
        let registry = SetRegistry::new().with(SetOpKind::StartMission, recording.clone());
        let mut target = Target::default();
        let unhandled = run_on("b1 S200 b2", &mut target, &registry, &[]);
        assert_eq!(unhandled, []);
        assert_eq!(
            *recording.0.borrow(),
            [SetOp::StartMission(nova_data::MissionId(200))]
        );
        assert_eq!(target.log, ["bits", "StartMission(MissionId(200))", "bits"]);
        assert_eq!(target.set(), [1, 2]);
    }

    #[test]
    fn an_operator_with_no_handler_is_skipped_and_reported_once() {
        let mut target = Target::default();
        let mut reported = BTreeSet::new();
        let registry = SetRegistry::new();
        let expr = parse("b1 G150 b2 G151 S300");
        let first = execute(
            &expr,
            &mut target,
            &registry,
            &mut Scripted::default(),
            &mut reported,
        );
        assert_eq!(first, [SetOpKind::GrantOutfit, SetOpKind::StartMission]);
        assert_eq!(target.set(), [1, 2], "the bits after it still apply");
        let again = execute(
            &parse("G152 b3"),
            &mut target,
            &registry,
            &mut Scripted::default(),
            &mut reported,
        );
        assert_eq!(again, [], "reported before");
        assert_eq!(target.set(), [1, 2, 3]);
        assert_eq!(
            reported,
            BTreeSet::from([SetOpKind::GrantOutfit, SetOpKind::StartMission])
        );
    }

    #[test]
    fn a_handler_inside_r_runs_only_when_its_arm_is_drawn() {
        let recording = Rc::new(Recording::default());
        let registry = SetRegistry::new().with(SetOpKind::GrantOutfit, recording.clone());
        let mut target = Target::default();
        run_on("R(G150 b4)", &mut target, &registry, &[0]);
        assert_eq!(*recording.0.borrow(), []);
        assert_eq!(target.set(), [4]);
        run_on("R(G150 b5)", &mut target, &registry, &[1]);
        assert_eq!(
            *recording.0.borrow(),
            [SetOp::GrantOutfit(nova_data::OutfitId(150))]
        );
        assert_eq!(target.set(), [4], "b5 not drawn");
        let unhandled = run_on("R(b6 S300)", &mut target, &registry, &[0]);
        assert_eq!(unhandled, [SetOpKind::StartMission], "drawn, unhandled");
        let unhandled = run_on("R(b6 S300)", &mut target, &registry, &[1]);
        assert_eq!(unhandled, [], "not drawn, so not reported");
    }

    #[test]
    fn a_registry_lists_its_kinds_and_a_later_handler_replaces_one() {
        let first = Rc::new(Recording::default());
        let second = Rc::new(Recording::default());
        let registry = SetRegistry::new()
            .with(SetOpKind::StartMission, first.clone())
            .with(SetOpKind::AbortMission, Rc::new(Recording::default()))
            .with(SetOpKind::StartMission, second.clone());
        assert_eq!(
            registry.kinds().collect::<Vec<_>>(),
            [SetOpKind::AbortMission, SetOpKind::StartMission]
        );
        assert_eq!(
            format!("{registry:?}"),
            "SetRegistry { kinds: [AbortMission, StartMission] }"
        );
        run_on("S200", &mut Target::default(), &registry, &[]);
        assert_eq!(first.0.borrow().len(), 0);
        assert_eq!(second.0.borrow().len(), 1);
        assert_eq!(SetRegistry::<Target>::default().kinds().count(), 0);
    }
}
