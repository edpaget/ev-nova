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

    /// The handler for `kind`, if any.
    #[must_use]
    pub fn handler(&self, kind: SetOpKind) -> Option<Rc<dyn SetOpHandler<T>>> {
        self.handlers.get(&kind).cloned()
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
/// `chance` and handing every other operator to `apply`, which applies it
/// or gives back its kind as unhandled. Gives the kinds given back that
/// `reported` did not hold yet, in the order met, and adds them to it.
pub fn execute<T, F>(
    expr: &SetExpr,
    target: &mut T,
    apply: &mut F,
    chance: &mut dyn Chance,
    reported: &mut BTreeSet<SetOpKind>,
) -> Vec<SetOpKind>
where
    T: BitStore + ?Sized,
    F: FnMut(&SetOp, &mut T, &mut dyn Chance) -> Result<(), SetOpKind>,
{
    let mut unhandled = Vec::new();
    for op in &expr.ops {
        run(op, target, apply, chance, reported, &mut unhandled);
    }
    unhandled
}

/// Runs one operator.
fn run<T, F>(
    op: &SetOp,
    target: &mut T,
    apply: &mut F,
    chance: &mut dyn Chance,
    reported: &mut BTreeSet<SetOpKind>,
    unhandled: &mut Vec<SetOpKind>,
) where
    T: BitStore + ?Sized,
    F: FnMut(&SetOp, &mut T, &mut dyn Chance) -> Result<(), SetOpKind>,
{
    match op {
        SetOp::Set(bit) => target.bits_mut().set(*bit),
        SetOp::Clear(bit) => target.bits_mut().clear(*bit),
        SetOp::Toggle(bit) => target.bits_mut().toggle(*bit),
        SetOp::Random(first, second) => {
            let arm = if chance.roll(2) == 0 { second } else { first };
            run(arm, target, apply, chance, reported, unhandled);
        }
        _ => {
            if let Err(kind) = apply(op, target, chance)
                && reported.insert(kind)
            {
                unhandled.push(kind);
            }
        }
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

    /// Records each operator it applies.
    #[derive(Debug, Default)]
    struct Recording(RefCell<Vec<SetOp>>);

    impl SetOpHandler<Target> for Recording {
        fn apply(&self, op: &SetOp, _target: &mut Target, _chance: &mut dyn Chance) {
            self.0.borrow_mut().push(op.clone());
        }
    }

    /// A dispatch applying the operators of `kinds` by logging each one on
    /// `log` and on the target's log, and giving back every other kind.
    fn recording<'a>(
        kinds: &'a [SetOpKind],
        log: &'a RefCell<Vec<SetOp>>,
    ) -> impl FnMut(&SetOp, &mut Target, &mut dyn Chance) -> Result<(), SetOpKind> + 'a {
        move |op, target, _chance| {
            if !kinds.contains(&op.kind()) {
                return Err(op.kind());
            }
            log.borrow_mut().push(op.clone());
            target.log.push(format!("{op:?}"));
            Ok(())
        }
    }

    /// A dispatch applying nothing: it gives back every kind.
    fn nothing(
        op: &SetOp,
        _target: &mut Target,
        _chance: &mut dyn Chance,
    ) -> Result<(), SetOpKind> {
        Err(op.kind())
    }

    fn bit(n: u16) -> Bit {
        Bit::new(n).expect("in range")
    }

    fn parse(text: &str) -> SetExpr {
        SetExpr::parse(text).expect("parses")
    }

    /// Runs `text` on `target` with `apply`, rolling `rolls`; gives the
    /// kinds newly unhandled.
    fn run_on(
        text: &str,
        target: &mut Target,
        mut apply: impl FnMut(&SetOp, &mut Target, &mut dyn Chance) -> Result<(), SetOpKind>,
        rolls: &[u16],
    ) -> Vec<SetOpKind> {
        execute(
            &parse(text),
            target,
            &mut apply,
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
        let unhandled = run_on("b1 !b2 ^b3", &mut target, nothing, &[]);
        assert_eq!(target.set(), [1]);
        assert_eq!(unhandled, []);
        run_on("^b3 ^b1 !b9", &mut target, nothing, &[]);
        assert_eq!(target.set(), [3]);
    }

    #[test]
    fn r_runs_the_second_arm_on_a_0_and_the_first_on_a_1() {
        let mut target = Target::default();
        let mut chance = Scripted::rolling(&[0]);
        execute(
            &parse("R(b1 b2)"),
            &mut target,
            &mut nothing,
            &mut chance,
            &mut BTreeSet::new(),
        );
        assert_eq!(target.set(), [2]);
        assert_eq!(chance.sides_asked, [2], "a roll of two sides");
        let mut target = Target::default();
        run_on("R(b1 b2)", &mut target, nothing, &[1]);
        assert_eq!(target.set(), [1]);
    }

    #[test]
    fn the_dispatch_gets_its_operator_and_the_target_in_order() {
        let log = RefCell::new(Vec::new());
        let mut target = Target::default();
        let unhandled = run_on(
            "b1 S200 b2",
            &mut target,
            recording(&[SetOpKind::StartMission], &log),
            &[],
        );
        assert_eq!(unhandled, []);
        assert_eq!(
            *log.borrow(),
            [SetOp::StartMission(nova_data::MissionId(200))]
        );
        assert_eq!(target.log, ["bits", "StartMission(MissionId(200))", "bits"]);
        assert_eq!(target.set(), [1, 2]);
    }

    #[test]
    fn an_operator_with_no_handler_is_skipped_and_reported_once() {
        let mut target = Target::default();
        let mut reported = BTreeSet::new();
        let expr = parse("b1 G150 b2 G151 S300");
        let first = execute(
            &expr,
            &mut target,
            &mut nothing,
            &mut Scripted::default(),
            &mut reported,
        );
        assert_eq!(first, [SetOpKind::GrantOutfit, SetOpKind::StartMission]);
        assert_eq!(target.set(), [1, 2], "the bits after it still apply");
        let again = execute(
            &parse("G152 b3"),
            &mut target,
            &mut nothing,
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
        let log = RefCell::new(Vec::new());
        let grants = [SetOpKind::GrantOutfit];
        let mut target = Target::default();
        run_on("R(G150 b4)", &mut target, recording(&grants, &log), &[0]);
        assert_eq!(*log.borrow(), []);
        assert_eq!(target.set(), [4]);
        run_on("R(G150 b5)", &mut target, recording(&grants, &log), &[1]);
        assert_eq!(
            *log.borrow(),
            [SetOp::GrantOutfit(nova_data::OutfitId(150))]
        );
        assert_eq!(target.set(), [4], "b5 not drawn");
        let unhandled = run_on("R(b6 S300)", &mut target, recording(&grants, &log), &[0]);
        assert_eq!(unhandled, [SetOpKind::StartMission], "drawn, unhandled");
        let unhandled = run_on("R(b6 S300)", &mut target, recording(&grants, &log), &[1]);
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
        run_on(
            "S200",
            &mut Target::default(),
            |op: &SetOp, target: &mut Target, chance: &mut dyn Chance| {
                let handler = registry.handlers.get(&op.kind()).ok_or(op.kind())?;
                handler.apply(op, target, chance);
                Ok(())
            },
            &[],
        );
        assert_eq!(first.0.borrow().len(), 0);
        assert_eq!(second.0.borrow().len(), 1);
        assert_eq!(SetRegistry::<Target>::default().kinds().count(), 0);
    }
}
