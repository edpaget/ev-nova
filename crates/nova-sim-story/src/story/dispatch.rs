//! The story dispatcher: the subscriptions registered, the budget one
//! settling may spend, and the dispatch itself, run through the
//! [`Stage`] port the session implements (see the module docs'
//! "Dispatch").

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, VecDeque};

use super::{StoryEvent, Subscription, SubscriptionId, dispatch_order};
use crate::control::{Script, Test};
use crate::rulebook::RuleSource;

/// A second subscription under an id already registered: refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("a story subscription {0:?} is registered already")]
pub struct DuplicateSubscription(pub SubscriptionId);

/// The subscriptions registered, each under its unique id.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Registry {
    subscriptions: BTreeMap<SubscriptionId, Subscription>,
}

impl Registry {
    /// Registers `subscription`, unless one is registered under its id
    /// already.
    pub fn register(&mut self, subscription: Subscription) -> Result<(), DuplicateSubscription> {
        match self.subscriptions.entry(subscription.id) {
            Entry::Occupied(_) => Err(DuplicateSubscription(subscription.id)),
            Entry::Vacant(slot) => {
                slot.insert(subscription);
                Ok(())
            }
        }
    }

    /// Takes the subscription registered under `id` out, if any.
    pub fn unregister(&mut self, id: SubscriptionId) -> Option<Subscription> {
        self.subscriptions.remove(&id)
    }

    /// The subscription registered under `id`, if any.
    #[must_use]
    pub fn get(&self, id: SubscriptionId) -> Option<&Subscription> {
        self.subscriptions.get(&id)
    }

    /// The ids of the subscriptions `event` runs, in [`dispatch_order`]'s
    /// order: the dispatch's snapshot.
    #[must_use]
    pub fn matching(&self, event: &StoryEvent) -> Vec<SubscriptionId> {
        dispatch_order(self.subscriptions.values(), event)
            .into_iter()
            .map(|sub| sub.id)
            .collect()
    }
}

/// How many events one settling dispatches at most: a guard against
/// scripts that raise each other for ever (see the module docs'
/// "Dispatch").
pub(crate) const EVENT_LIMIT: u32 = 64;

/// What one settling may still dispatch, and how many events it dropped
/// past [`EVENT_LIMIT`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Budget {
    spent: u32,
    dropped: u32,
}

impl Budget {
    /// Spends one dispatch: whether the budget allowed it. One refused is
    /// counted dropped.
    pub(crate) fn spend(&mut self) -> bool {
        if self.spent < EVENT_LIMIT {
            self.spent += 1;
            true
        } else {
            self.dropped += 1;
            false
        }
    }

    /// The events dropped so far.
    pub(crate) const fn dropped(self) -> u32 {
        self.dropped
    }
}

/// When the events a script raises dispatch: the Rulebook key
/// [`RuleKey::StoryReentry`](crate::RuleKey::StoryReentry).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reentry {
    /// Depth first, before the script's next operator: the original's.
    Nested,
    /// Once the script has settled, first in, first out.
    Queued,
}

impl From<RuleSource> for Reentry {
    fn from(source: RuleSource) -> Self {
        match source {
            RuleSource::Engine => Self::Nested,
            RuleSource::Bible => Self::Queued,
        }
    }
}

/// The port the dispatcher works through: the session implements it over
/// its own state, with the catalog and the chance a settling holds.
pub trait Stage {
    /// The ids of the subscriptions `event` runs, in order.
    fn matching(&self, event: &StoryEvent) -> Vec<SubscriptionId>;
    /// Whether the consumer of `id` lets it run on `event` now: its own
    /// predicate, beyond a control-bit guard, asked just before the
    /// subscription runs.
    fn admits(&self, id: SubscriptionId, event: &StoryEvent) -> bool;
    /// The subscription registered under `id`, if it still is.
    fn subscription(&self, id: SubscriptionId) -> Option<Subscription>;
    /// Whether the once-only subscription `id` has fired.
    fn fired(&self, id: SubscriptionId) -> bool;
    /// Records that the once-only subscription `id` has fired.
    fn mark_fired(&mut self, id: SubscriptionId);
    /// Whether `guard` holds now.
    fn holds(&self, guard: &Test) -> bool;
    /// Runs the consumer's own effect of `id` on `event`, if it has one.
    fn effect(&mut self, id: SubscriptionId, event: &StoryEvent);
    /// Runs `script`, calling `between` after each operator it applies.
    fn run(&mut self, script: &Script, between: &mut dyn FnMut(&mut dyn Stage));
    /// Applies the moves queued, which may raise events.
    fn settle_moves(&mut self);
    /// Applies a pending leave, which may raise events: whether one was.
    fn settle_leave(&mut self) -> bool;
    /// Takes the events raised since last taken, in order.
    fn take_raised(&mut self) -> VecDeque<StoryEvent>;
}

/// Settles `stage`: its moves, then the events raised, each dispatched in
/// turn, first in, first out, the events a script raises dispatched as
/// `reentry` says, then a pending leave, and so on until nothing is left.
/// Gives how many events were dropped past [`EVENT_LIMIT`].
pub fn settle(stage: &mut dyn Stage, reentry: Reentry) -> u32 {
    let mut budget = Budget::default();
    let mut queue = VecDeque::new();
    loop {
        stage.settle_moves();
        queue.extend(stage.take_raised());
        if let Some(event) = queue.pop_front() {
            dispatch(stage, &event, reentry, &mut budget);
        } else if !stage.settle_leave() {
            return budget.dropped();
        }
    }
}

/// Dispatches `event` on `stage`: each subscription of the snapshot still
/// registered, not fired once already, whose guard holds now and whose
/// consumer admits it ([`Stage::admits`]), marked fired first when
/// once-only, then its effect, then its script, the events they raise
/// nested as `reentry` says. The event spends `budget` as its first
/// subscription runs, so one whose subscriptions all hold back costs
/// nothing, as one with none does; one the budget refuses is dropped,
/// none of its subscriptions run.
///
/// A `Landed` event walks live instead of over a snapshot: after each
/// subscription is considered it asks [`Stage::matching`] again and takes
/// the first id that sorts after the last one considered. One registered
/// mid-dispatch runs in this dispatch when it sorts after the cursor and
/// waits for the next landing when it doesn't. This mirrors
/// `_MissionLandCheck` (@0xa19f7-0xa1aec) reading each slot's active flag
/// as it reaches that slot. Other events keep the snapshot: their loops
/// in the original haven't been read for this.
fn dispatch(stage: &mut dyn Stage, event: &StoryEvent, reentry: Reentry, budget: &mut Budget) {
    let mut spent = false;
    let live = matches!(event, StoryEvent::Landed { .. });
    let mut snapshot = stage.matching(event).into_iter();
    let mut cursor: Option<SubscriptionId> = None;
    loop {
        let next = if live {
            stage
                .matching(event)
                .into_iter()
                .find(|&id| cursor.is_none_or(|last| id > last))
        } else {
            snapshot.next()
        };
        let Some(id) = next else {
            return;
        };
        cursor = Some(id);
        let Some(sub) = stage.subscription(id) else {
            continue;
        };
        if (sub.once && stage.fired(id)) || !stage.holds(&sub.guard) || !stage.admits(id, event) {
            continue;
        }
        if !spent {
            if !budget.spend() {
                return;
            }
            spent = true;
        }
        if sub.once {
            stage.mark_fired(id);
        }
        stage.effect(id, event);
        match reentry {
            Reentry::Nested => {
                nest(stage, budget);
                stage.run(&sub.script, &mut |stage| nest(stage, budget));
            }
            Reentry::Queued => stage.run(&sub.script, &mut |_| {}),
        }
    }
}

/// Settles what the last step queued and dispatches the events raised,
/// in order, each depth first: the nested reading's step between a
/// script's operators.
fn nest(stage: &mut dyn Stage, budget: &mut Budget) {
    stage.settle_moves();
    for event in stage.take_raised() {
        dispatch(stage, &event, Reentry::Nested, budget);
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::BTreeSet;

    use super::*;
    use crate::catalog::{OutfitId, StellarId, SystemId};
    use crate::control::{Bit, ControlBitSet, Gate, NovaBits, PilotFacts, SetOp};
    use crate::pilot::Gender;
    use crate::story::{Consumer, Entrance, EventFilter, StoryEventKind};

    fn id(record: i16, hook: u8) -> SubscriptionId {
        SubscriptionId {
            consumer: Consumer::Fixture,
            record,
            hook,
        }
    }

    fn landed(at: i16) -> StoryEvent {
        StoryEvent::Landed {
            stellar: StellarId(at),
        }
    }

    fn on_landing(record: i16, hook: u8, filter: EventFilter) -> Subscription {
        Subscription::new(id(record, hook), filter, Script::parse("b1"))
    }

    #[test]
    fn a_second_subscription_under_one_id_is_refused_and_the_first_kept() {
        let mut registry = Registry::default();
        let first = on_landing(1, 0, EventFilter::Landed(StellarId(128)));
        let second = Subscription {
            script: Script::parse("b2"),
            ..first.clone()
        };
        assert_eq!(registry.register(first.clone()), Ok(()));
        assert_eq!(
            registry.register(second),
            Err(DuplicateSubscription(id(1, 0)))
        );
        assert_eq!(registry.get(id(1, 0)), Some(&first));
        assert_eq!(registry.get(id(1, 1)), None);
    }

    #[test]
    fn unregistering_gives_the_subscription_back_once() {
        let mut registry = Registry::default();
        let sub = on_landing(1, 0, EventFilter::Landed(StellarId(128)));
        registry.register(sub.clone()).expect("registers");
        assert_eq!(registry.unregister(id(1, 0)), Some(sub));
        assert_eq!(registry.unregister(id(1, 0)), None);
        assert_eq!(registry.get(id(1, 0)), None);
        assert_eq!(registry.matching(&landed(128)), []);
    }

    #[test]
    fn matching_is_dispatch_order_over_the_registered_whatever_their_order() {
        let any = EventFilter::Any(StoryEventKind::Landed);
        let subs = [
            on_landing(3, 0, any),
            on_landing(1, 2, EventFilter::Landed(StellarId(128))),
            on_landing(1, 1, EventFilter::Landed(StellarId(129))),
            on_landing(2, 0, any),
            on_landing(1, 0, any),
        ];
        let expected = dispatch_order(&subs, &landed(128))
            .into_iter()
            .map(|sub| sub.id)
            .collect::<Vec<_>>();
        assert_eq!(expected, [id(1, 0), id(1, 2), id(2, 0), id(3, 0)]);
        for turn in 0..subs.len() {
            let mut registry = Registry::default();
            let mut order = subs.to_vec();
            order.rotate_left(turn);
            for sub in order {
                registry.register(sub).expect("registers");
            }
            assert_eq!(registry.matching(&landed(128)), expected, "{turn}");
        }
    }

    #[test]
    fn a_budget_spends_64_then_refuses_and_counts_each_refusal() {
        let mut budget = Budget::default();
        for n in 0..EVENT_LIMIT {
            assert!(budget.spend(), "{n}");
        }
        assert_eq!(budget.dropped(), 0);
        assert!(!budget.spend());
        assert!(!budget.spend());
        assert_eq!(budget.dropped(), 2);
        assert_eq!(EVENT_LIMIT, 64);
    }

    #[test]
    fn the_engine_reading_nests_and_the_other_queues() {
        assert_eq!(Reentry::from(RuleSource::Engine), Reentry::Nested);
        assert_eq!(Reentry::from(RuleSource::Bible), Reentry::Queued);
    }

    /// The control bits a guard reads.
    #[derive(Debug, Default)]
    struct Bits(ControlBitSet);

    impl PilotFacts for Bits {
        fn bit(&self, bit: Bit) -> bool {
            self.0.get(bit)
        }

        fn gender(&self) -> Gender {
            Gender::Male
        }

        fn paid(&self, _days: u16) -> bool {
            true
        }

        fn has_outfit(&self, _outfit: OutfitId) -> bool {
            false
        }

        fn explored(&self, _system: SystemId) -> bool {
            false
        }
    }

    /// A stage that plays scripts on control bits and records every call.
    /// Its operators: a bit write; `X` raises `Explored` at once, `Y`
    /// raises `Landed` at once; `M` queues a move that raises
    /// `EnteredSystem` once settled; `Q` leaves, raising `TookOff` once
    /// settled. An effect raises, registers and unregisters as its tables
    /// say.
    #[derive(Debug, Default)]
    struct Play {
        registry: Registry,
        fired: BTreeSet<SubscriptionId>,
        bits: Bits,
        /// Each guard's name, for the log.
        guards: Vec<(Test, String)>,
        raised: VecDeque<StoryEvent>,
        moves: Vec<SystemId>,
        leave: bool,
        /// What each subscription's effect raises.
        effect_raises: BTreeMap<SubscriptionId, Vec<StoryEvent>>,
        /// The subscription each subscription's effect registers.
        effect_registers: BTreeMap<SubscriptionId, Subscription>,
        /// The subscription each subscription's effect unregisters.
        effect_unregisters: BTreeMap<SubscriptionId, SubscriptionId>,
        log: Vec<String>,
        /// The guards asked, with the answer: asked through a shared
        /// reference.
        asked: RefCell<Vec<String>>,
        /// The subscriptions their consumer refuses.
        refused: BTreeSet<SubscriptionId>,
        /// The consumer predicates asked, with the answer.
        admitted: RefCell<Vec<String>>,
    }

    impl Stage for Play {
        fn matching(&self, event: &StoryEvent) -> Vec<SubscriptionId> {
            self.registry.matching(event)
        }

        fn subscription(&self, id: SubscriptionId) -> Option<Subscription> {
            self.registry.get(id).cloned()
        }

        fn fired(&self, id: SubscriptionId) -> bool {
            self.fired.contains(&id)
        }

        fn mark_fired(&mut self, id: SubscriptionId) {
            self.log.push(format!("fired {}", id.record));
            self.fired.insert(id);
        }

        fn admits(&self, id: SubscriptionId, event: &StoryEvent) -> bool {
            let admits = !self.refused.contains(&id);
            self.admitted.borrow_mut().push(format!(
                "{} on {:?}? {admits}",
                id.record,
                event.kind()
            ));
            admits
        }

        fn holds(&self, guard: &Test) -> bool {
            let holds = Gate {
                control_bits: &NovaBits,
                pilot: &self.bits,
            }
            .allows(guard);
            if let Some((_, name)) = self.guards.iter().find(|(test, _)| test == guard) {
                self.asked.borrow_mut().push(format!("{name}? {holds}"));
            }
            holds
        }

        fn effect(&mut self, id: SubscriptionId, event: &StoryEvent) {
            let _ = event;
            self.log.push(format!("effect {}", id.record));
            self.raised
                .extend(self.effect_raises.get(&id).cloned().unwrap_or_default());
            if let Some(sub) = self.effect_registers.get(&id).cloned() {
                self.registry.register(sub).expect("registers");
            }
            if let Some(&gone) = self.effect_unregisters.get(&id) {
                self.registry.unregister(gone);
            }
        }

        fn run(&mut self, script: &Script, between: &mut dyn FnMut(&mut dyn Stage)) {
            for op in &script.tree().expect("parses").ops {
                let entry = match *op {
                    SetOp::Set(bit) => {
                        self.bits.0.set(bit);
                        format!("b{}", bit.get())
                    }
                    SetOp::Clear(bit) => {
                        self.bits.0.clear(bit);
                        format!("!b{}", bit.get())
                    }
                    SetOp::Explore(system) => {
                        self.raised.push_back(StoryEvent::Explored { system });
                        format!("X{}", system.0)
                    }
                    SetOp::DestroyStellar(stellar) => {
                        self.raised.push_back(StoryEvent::Landed { stellar });
                        format!("Y{}", stellar.0)
                    }
                    SetOp::MoveTo(system) => {
                        self.moves.push(system);
                        format!("M{}", system.0)
                    }
                    SetOp::LeaveStellar(_) => {
                        self.leave = true;
                        "Q".to_owned()
                    }
                    ref other => panic!("not played: {other:?}"),
                };
                self.log.push(entry);
                between(self);
            }
        }

        fn settle_moves(&mut self) {
            for system in std::mem::take(&mut self.moves) {
                self.log.push(format!("enter {}", system.0));
                self.raised.push_back(StoryEvent::EnteredSystem {
                    system,
                    how: Entrance::ScriptMove { landed: false },
                });
            }
        }

        fn settle_leave(&mut self) -> bool {
            if !std::mem::take(&mut self.leave) {
                return false;
            }
            self.log.push("leave".to_owned());
            self.raised.push_back(StoryEvent::TookOff {
                stellar: StellarId(128),
            });
            true
        }

        fn take_raised(&mut self) -> VecDeque<StoryEvent> {
            std::mem::take(&mut self.raised)
        }
    }

    const E1: StoryEvent = StoryEvent::Landed {
        stellar: StellarId(128),
    };

    fn entered_131() -> EventFilter {
        EventFilter::EnteredSystem(SystemId(131))
    }

    impl Play {
        /// Registers `record`'s subscription to `on`, running `script`
        /// behind `guard` (named `guard` in the log; blank for none).
        fn sub(&mut self, record: i16, on: EventFilter, guard: &str, script: &str) -> &mut Self {
            let test = Test::parse(guard);
            if !guard.is_empty() {
                self.guards.push((test.clone(), guard.to_owned()));
            }
            self.registry
                .register(Subscription {
                    guard: test,
                    ..Subscription::new(id(record, 0), on, Script::parse(script))
                })
                .expect("registers");
            self
        }

        /// Registers `record`'s once-only subscription to `on`.
        fn once(&mut self, record: i16, on: EventFilter, script: &str) -> &mut Self {
            self.registry
                .register(Subscription {
                    once: true,
                    ..Subscription::new(id(record, 0), on, Script::parse(script))
                })
                .expect("registers");
            self
        }

        /// Raises `events` and settles by `reentry`: the events dropped.
        fn settle(&mut self, reentry: Reentry, events: &[StoryEvent]) -> u32 {
            self.raised.extend(events);
            settle(self, reentry)
        }

        fn bit(&self, n: u16) -> bool {
            self.bits.0.get(Bit::new(n).expect("in range"))
        }
    }

    fn played(log: &[&str]) -> Vec<String> {
        log.iter().map(|&entry| entry.to_owned()).collect()
    }

    #[test]
    fn an_event_runs_its_matching_subscriptions_in_dispatch_order_and_none_other() {
        let mut play = Play::default();
        let any = EventFilter::Any(StoryEventKind::Landed);
        play.sub(3, any, "", "b3")
            .sub(1, any, "", "b1")
            .sub(2, EventFilter::Landed(StellarId(129)), "", "b2")
            .sub(4, EventFilter::Any(StoryEventKind::TookOff), "", "b4");
        assert_eq!(play.settle(Reentry::Nested, &[E1]), 0);
        assert_eq!(play.log, played(&["effect 1", "b1", "effect 3", "b3"]));
        let mut play = Play::default();
        play.sub(1, any, "", "b1");
        play.settle(
            Reentry::Nested,
            &[StoryEvent::Explored {
                system: SystemId(130),
            }],
        );
        assert_eq!(play.log, played(&[]), "unmatched");
    }

    #[test]
    fn a_failing_guard_runs_neither_effect_nor_script_and_leaves_a_once_unfired() {
        let any = EventFilter::Any(StoryEventKind::Landed);
        let mut play = Play::default();
        play.sub(1, any, "b9", "b1");
        play.registry.unregister(id(1, 0));
        play.registry
            .register(Subscription {
                guard: Test::parse("b9"),
                once: true,
                ..Subscription::new(id(1, 0), any, Script::parse("b1"))
            })
            .expect("registers");
        play.settle(Reentry::Nested, &[E1]);
        assert_eq!(play.log, played(&[]));
        assert_eq!(play.asked.take(), ["b9? false"]);
        assert!(!play.fired(id(1, 0)));
        play.bits.0.set(Bit::new(9).expect("in range"));
        play.settle(Reentry::Nested, &[E1]);
        assert_eq!(play.log, played(&["fired 1", "effect 1", "b1"]));
        assert_eq!(play.asked.take(), ["b9? true"]);
    }

    #[test]
    fn a_subscription_its_consumer_refuses_runs_nothing_and_a_once_stays_unfired() {
        let any = EventFilter::Any(StoryEventKind::Landed);
        let mut play = Play::default();
        play.once(1, any, "b1").sub(2, any, "", "b2");
        play.refused.insert(id(1, 0));
        play.settle(Reentry::Nested, &[E1]);
        assert_eq!(play.log, played(&["effect 2", "b2"]));
        assert!(!play.fired(id(1, 0)), "left unfired");
        assert_eq!(
            play.admitted.take(),
            ["1 on Landed? false", "2 on Landed? true"]
        );
        play.log.clear();
        play.refused.clear();
        play.settle(Reentry::Nested, &[E1]);
        assert_eq!(
            play.log,
            played(&["fired 1", "effect 1", "b1", "effect 2", "b2"])
        );
    }

    #[test]
    fn a_consumer_is_asked_once_the_guard_holds_just_before_its_subscription_runs() {
        let any = EventFilter::Any(StoryEventKind::Landed);
        let mut play = Play::default();
        play.sub(1, any, "", "b9")
            .sub(2, any, "b8", "b2")
            .sub(3, any, "b9", "b3");
        play.refused.insert(id(3, 0));
        play.settle(Reentry::Nested, &[E1]);
        assert_eq!(
            play.admitted.take(),
            ["1 on Landed? true", "3 on Landed? false"],
            "2's guard fails, so its consumer is not asked"
        );
        assert_eq!(play.asked.take(), ["b8? false", "b9? true"]);
        assert_eq!(play.log, played(&["effect 1", "b9"]));
    }

    #[test]
    fn an_event_whose_subscriptions_all_hold_back_costs_no_budget() {
        let explored = |system| StoryEvent::Explored {
            system: SystemId(system),
        };
        for reentry in [Reentry::Nested, Reentry::Queued] {
            let mut play = Play::default();
            play.sub(1, EventFilter::Explored(SystemId(131)), "b9", "b1")
                .sub(2, EventFilter::Explored(SystemId(132)), "", "b2")
                .once(3, EventFilter::Explored(SystemId(133)), "b3")
                .sub(4, EventFilter::Explored(SystemId(134)), "", "b4");
            play.refused.insert(id(2, 0));
            play.fired.insert(id(3, 0));
            for system in [131, 132, 133] {
                play.raised.extend([explored(system); 40]);
            }
            assert_eq!(play.settle(reentry, &[explored(134)]), 0, "{reentry:?}");
            assert_eq!(play.log, played(&["effect 4", "b4"]), "{reentry:?}");
        }
    }

    #[test]
    fn a_guard_is_asked_just_before_its_subscription_runs() {
        let any = EventFilter::Any(StoryEventKind::Landed);
        for reentry in [Reentry::Nested, Reentry::Queued] {
            let mut play = Play::default();
            play.sub(1, any, "", "b9 !b8")
                .sub(2, any, "b9", "b2")
                .sub(3, any, "b8", "b3");
            play.bits.0.set(Bit::new(8).expect("in range"));
            play.settle(reentry, &[E1]);
            assert!(play.bit(2), "opened by an earlier script, {reentry:?}");
            assert!(!play.bit(3), "closed by an earlier script, {reentry:?}");
        }
    }

    #[test]
    fn a_once_is_marked_before_its_effect_and_skipped_once_fired_even_nested_in_itself() {
        let landed = EventFilter::Any(StoryEventKind::Landed);
        let mut play = Play::default();
        play.once(1, landed, "Y128 b1");
        play.effect_raises.insert(id(1, 0), vec![E1]);
        play.settle(Reentry::Nested, &[E1]);
        assert_eq!(
            play.log,
            played(&["fired 1", "effect 1", "Y128", "b1"]),
            "re-raised by its effect and its script, it runs once"
        );
        play.log.clear();
        play.settle(Reentry::Nested, &[E1]);
        assert_eq!(play.log, played(&[]), "fired");
    }

    #[test]
    fn other_events_keep_the_snapshot_and_a_registration_waits_for_the_next_event() {
        let any = EventFilter::Any(StoryEventKind::Explored);
        let e1 = StoryEvent::Explored {
            system: SystemId(131),
        };
        for reentry in [Reentry::Nested, Reentry::Queued] {
            let mut play = Play::default();
            play.sub(1, any, "", "b1").sub(3, any, "", "b3");
            play.effect_unregisters.insert(id(1, 0), id(3, 0));
            play.effect_registers.insert(
                id(1, 0),
                Subscription::new(id(2, 0), any, Script::parse("b2")),
            );
            play.settle(reentry, &[e1]);
            assert_eq!(play.log, played(&["effect 1", "b1"]), "{reentry:?}");
            play.log.clear();
            play.effect_registers.clear();
            play.settle(reentry, &[e1]);
            assert_eq!(
                play.log,
                played(&["effect 1", "b1", "effect 2", "b2"]),
                "{reentry:?}"
            );
        }
    }

    #[test]
    fn a_landing_runs_a_subscription_registered_mid_dispatch_that_sorts_after_the_one_running() {
        let any = EventFilter::Any(StoryEventKind::Landed);
        for reentry in [Reentry::Nested, Reentry::Queued] {
            let mut play = Play::default();
            play.sub(1, any, "", "b1").sub(3, any, "", "b3");
            play.effect_unregisters.insert(id(1, 0), id(3, 0));
            play.effect_registers.insert(
                id(1, 0),
                Subscription::new(id(2, 0), any, Script::parse("b2")),
            );
            play.settle(reentry, &[E1]);
            assert_eq!(
                play.log,
                played(&["effect 1", "b1", "effect 2", "b2"]),
                "{reentry:?}: 2 runs, 3 dropped out"
            );
        }
    }

    #[test]
    fn a_landing_leaves_one_registered_mid_dispatch_that_sorts_before_for_the_next_landing() {
        let any = EventFilter::Any(StoryEventKind::Landed);
        for reentry in [Reentry::Nested, Reentry::Queued] {
            let mut play = Play::default();
            play.sub(2, any, "", "b2").sub(3, any, "", "b3");
            play.effect_registers.insert(
                id(2, 0),
                Subscription::new(id(1, 0), any, Script::parse("b1")),
            );
            play.settle(reentry, &[E1]);
            assert_eq!(
                play.log,
                played(&["effect 2", "b2", "effect 3", "b3"]),
                "{reentry:?}"
            );
            play.log.clear();
            play.effect_registers.clear();
            play.settle(reentry, &[E1]);
            assert_eq!(
                play.log,
                played(&["effect 1", "b1", "effect 2", "b2", "effect 3", "b3"]),
                "{reentry:?}"
            );
        }
    }

    /// A on `E1`: `b1 M131 b2`; B on entering 131: `b3`; C on `E1` after
    /// A: `b4`.
    fn abc() -> Play {
        let mut play = Play::default();
        play.sub(
            1,
            EventFilter::Any(StoryEventKind::Landed),
            "",
            "b1 M131 b2",
        )
        .sub(2, entered_131(), "", "b3")
        .sub(3, EventFilter::Any(StoryEventKind::Landed), "", "b4");
        play
    }

    #[test]
    fn nested_an_event_a_script_raises_runs_before_the_scripts_next_operator() {
        let mut play = abc();
        play.settle(Reentry::Nested, &[E1]);
        assert_eq!(
            play.log,
            played(&[
                "effect 1",
                "b1",
                "M131",
                "enter 131",
                "effect 2",
                "b3",
                "b2",
                "effect 3",
                "b4"
            ])
        );
    }

    #[test]
    fn nested_two_levels_deep_and_an_effects_events_before_the_script() {
        let mut play = Play::default();
        play.sub(
            1,
            EventFilter::Any(StoryEventKind::Landed),
            "",
            "b1 M131 b2",
        )
        .sub(2, entered_131(), "", "X131 b3")
        .sub(3, EventFilter::Explored(SystemId(131)), "", "b4")
        .sub(4, EventFilter::Explored(SystemId(132)), "", "b5");
        play.effect_raises.insert(
            id(1, 0),
            vec![StoryEvent::Explored {
                system: SystemId(132),
            }],
        );
        play.settle(Reentry::Nested, &[E1]);
        assert_eq!(
            play.log,
            played(&[
                "effect 1",
                "effect 4",
                "b5",
                "b1",
                "M131",
                "enter 131",
                "effect 2",
                "X131",
                "effect 3",
                "b4",
                "b3",
                "b2"
            ])
        );
    }

    #[test]
    fn queued_the_events_a_script_raises_wait_for_the_dispatch_then_go_in_order() {
        let mut play = abc();
        play.settle(Reentry::Queued, &[E1]);
        assert_eq!(
            play.log,
            played(&[
                "effect 1",
                "b1",
                "M131",
                "b2",
                "effect 3",
                "b4",
                "enter 131",
                "effect 2",
                "b3"
            ])
        );
        let mut play = Play::default();
        play.sub(
            1,
            EventFilter::Any(StoryEventKind::Landed),
            "",
            "X131 M132 X133",
        )
        .sub(2, EventFilter::Any(StoryEventKind::Explored), "", "b2")
        .sub(3, EventFilter::Any(StoryEventKind::EnteredSystem), "", "b3");
        play.settle(Reentry::Queued, &[E1]);
        assert_eq!(
            play.log,
            played(&[
                "effect 1",
                "X131",
                "M132",
                "X133",
                "enter 132",
                "effect 2",
                "b2",
                "effect 2",
                "b2",
                "effect 3",
                "b3"
            ]),
            "the moves settle before the queue, the events raised first in, first out"
        );
    }

    #[test]
    fn the_settle_applies_the_moves_first_and_the_leave_last_then_its_events() {
        for reentry in [Reentry::Nested, Reentry::Queued] {
            let mut play = Play::default();
            play.sub(1, EventFilter::Any(StoryEventKind::Landed), "", "Q1 b1")
                .sub(2, EventFilter::Any(StoryEventKind::TookOff), "", "b2")
                .sub(3, entered_131(), "", "b3");
            play.moves.push(SystemId(131));
            play.settle(reentry, &[E1]);
            assert_eq!(
                play.log,
                played(&[
                    "enter 131",
                    "effect 1",
                    "Q",
                    "b1",
                    "effect 3",
                    "b3",
                    "leave",
                    "effect 2",
                    "b2"
                ]),
                "{reentry:?}"
            );
        }
    }

    #[test]
    fn a_self_raising_chain_dispatches_64_events_and_drops_the_rest() {
        let explored = StoryEvent::Explored {
            system: SystemId(131),
        };
        for reentry in [Reentry::Nested, Reentry::Queued] {
            let mut play = Play::default();
            play.sub(1, EventFilter::Explored(SystemId(131)), "", "X131");
            let unmatched = [StoryEvent::Explored {
                system: SystemId(999),
            }; 100];
            play.raised.extend(unmatched);
            assert_eq!(play.settle(reentry, &[explored]), 1, "{reentry:?}");
            let runs = play.log.iter().filter(|entry| *entry == "X131").count();
            assert_eq!(runs, 64, "{reentry:?}: unmatched events cost nothing");
            play.log.clear();
            assert_eq!(play.settle(reentry, &[explored]), 1, "a fresh budget");
            assert_eq!(play.log.iter().filter(|entry| *entry == "X131").count(), 64);
        }
    }

    #[test]
    fn nested_the_budget_is_shared_across_the_levels() {
        let mut play = Play::default();
        play.sub(1, EventFilter::Explored(SystemId(131)), "", "X131 X131");
        let explored = StoryEvent::Explored {
            system: SystemId(131),
        };
        let dropped = play.settle(Reentry::Nested, &[explored]);
        let runs = play
            .log
            .iter()
            .filter(|entry| entry.starts_with("effect"))
            .count();
        assert_eq!(runs, 64);
        assert_eq!(
            dropped, 65,
            "the deepest's two events and every other level's second, from one budget"
        );
    }
}
