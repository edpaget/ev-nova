//! The order loop: an order of a count, run one pass at a time.
//!
//! The original's outfitter buys or sells a count one at a time: the buy
//! loop (`_DoOutfitDialog` @0x5c1bd-0x5c2ff) re-runs `_CanBuyOutfitItem`
//! after each grant, and the sell loop (@0x5c6ee-0x5d1b1) checks each
//! sale afresh. A plain click is a count of 1 (@0x5c69f), so a single
//! order and a counted one take the same loop.
//!
//! [`run`] and [`run_on`] are pure drivers: each hands every pass its
//! [`Pass`], the pass index, the whole count and whether an earlier pass
//! was refused. [`run`], the buy loop's, stops at the first refusal;
//! [`run_on`], the sell loop's, runs on past it to the count, as the
//! original's sell loop never breaks (@0x5d19c-0x5d1b1), and lets a pass
//! that went through restart it from another index ([`Onward::From`]),
//! as a sale that changes the listing does (@0x5d0fe-0x5d120). What one pass
//! does is the caller's, through the pass function it supplies (the
//! port for "run one pass"): the session's checks the order and applies
//! it. The loop's [`OrderRun`] keeps its first refusal;
//! [`OrderRun::counted`] gives the result a counted order reports.

/// One pass of a counted order: its index from 0, and the whole count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pass {
    /// The pass's index, from 0.
    pub index: u32,
    /// The whole count the order asks for.
    pub count: u32,
    /// Whether an earlier pass of this order was refused: the loop's copy
    /// of the original's once-an-order flag -0x63e, cleared once per
    /// order (@0x5c6ca) and set with a refusal's words (@0x5c7a0,
    /// @0x5ca3f, @0x5ce5a). Only [`run_on`] sets it; [`run`] stops at
    /// the first refusal, so no pass ever follows one.
    pub alerted: bool,
}

impl Pass {
    /// A plain click, a count of 1 (@0x5c69f): the pass of a single order.
    pub const SINGLE: Pass = Pass {
        index: 0,
        count: 1,
        alerted: false,
    };
}

/// Where and why a loop stopped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stopped<R> {
    /// The index of the pass refused.
    pub pass: u32,
    /// Why it was refused.
    pub refusal: R,
}

/// What a loop did: the passes done, and the refusal that stopped it, if
/// any.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrderRun<R> {
    /// How many passes went through.
    pub done: u32,
    /// The first pass refused, and why (where a stopping loop stops).
    pub stopped: Option<Stopped<R>>,
}

impl<R> OrderRun<R> {
    /// The result a counted order reports: the refusal when the first
    /// pass is refused, nothing having gone through; otherwise how many
    /// went through, a later refusal dropped.
    ///
    /// # Errors
    ///
    /// The first pass's refusal, when that pass was refused.
    pub fn counted(self) -> Result<u32, R> {
        match self.stopped {
            Some(Stopped { pass: 0, refusal }) => Err(refusal),
            _ => Ok(self.done),
        }
    }
}

/// Runs `pass` for each index from 0 up to `count`, handing it the
/// [`Pass`], and stops at the first refusal, as the original's buy loop
/// (@0x5c1bd-0x5c2ff) does; a plain click is a count of 1 (@0x5c69f). A
/// count of none runs no pass. No pass follows a refusal, so every pass
/// is handed `alerted: false`.
pub fn run<R>(count: u32, mut pass: impl FnMut(Pass) -> Result<(), R>) -> OrderRun<R> {
    let mut done = 0;
    while done < count {
        if let Err(refusal) = pass(Pass {
            index: done,
            count,
            alerted: false,
        }) {
            return OrderRun {
                done,
                stopped: Some(Stopped {
                    pass: done,
                    refusal,
                }),
            };
        }
        done += 1;
    }
    OrderRun {
        done,
        stopped: None,
    }
}

/// What a pass that went through does to the sell loop's index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Onward {
    /// The loop goes on to the next index.
    Next,
    /// The loop's index becomes `at` before its increment, as the sell
    /// loop's relisting resets -0x63a (@0x5d0fe-0x5d120), so the next
    /// pass is `at + 1`, while that is below the count.
    From(u32),
}

/// Runs `pass` for every index from 0 up to `count`, handing it the
/// [`Pass`], and runs on past a refusal, as the original's sell loop
/// (@0x5c6ee-0x5d1b1) never breaks (@0x5d19c-0x5d1b1). Every pass after
/// the first refused one is handed `alerted: true`. The result counts the
/// passes that went through and keeps the first refusal; a later one
/// never replaces it. A count of none runs no pass.
///
/// A pass that went through says where the loop goes on: [`Onward::Next`]
/// to the next index, or [`Onward::From`] to restart from the index after
/// the one it names, as a sale that changes the listing does
/// (@0x5d0fe-0x5d120). A restart keeps the first refusal and the
/// `alerted` flag, as -0x63e is never reset by it. `done` counts every
/// pass that went through, so it can pass the count. The loop ends only
/// if the restarts do: the driver relies on its pass for that (the
/// session's restarts come only from sales, and each lowers a count
/// owned).
pub fn run_on<R>(count: u32, mut pass: impl FnMut(Pass) -> Result<Onward, R>) -> OrderRun<R> {
    let mut done = 0;
    let mut stopped = None;
    let mut index = 0;
    while index < count {
        let alerted = stopped.is_some();
        let mut at = index;
        match pass(Pass {
            index,
            count,
            alerted,
        }) {
            Ok(onward) => {
                done += 1;
                if let Onward::From(from) = onward {
                    at = from;
                }
            }
            Err(refusal) if !alerted => {
                stopped = Some(Stopped {
                    pass: index,
                    refusal,
                });
            }
            Err(_) => {}
        }
        index = at + 1;
    }
    OrderRun { done, stopped }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A pass that records each [`Pass`] it is handed and refuses with
    /// its index from `refuse_at` on, or only at `refuse_at` when
    /// `only_there`.
    struct Recording {
        seen: Vec<Pass>,
        refuse_at: Option<u32>,
        only_there: bool,
    }

    impl Recording {
        fn accepting() -> Self {
            Self {
                seen: Vec::new(),
                refuse_at: None,
                only_there: false,
            }
        }

        /// Refuses from `index` on.
        fn refusing_at(index: u32) -> Self {
            Self {
                seen: Vec::new(),
                refuse_at: Some(index),
                only_there: false,
            }
        }

        /// Refuses at `index` alone, accepting every pass after it.
        fn refusing_only_at(index: u32) -> Self {
            Self {
                seen: Vec::new(),
                refuse_at: Some(index),
                only_there: true,
            }
        }

        fn pass(&mut self, pass: Pass) -> Result<(), u32> {
            self.seen.push(pass);
            match self.refuse_at {
                Some(at) if pass.index == at || (!self.only_there && pass.index > at) => {
                    Err(pass.index)
                }
                _ => Ok(()),
            }
        }

        fn run(&mut self, count: u32) -> OrderRun<u32> {
            run(count, |pass| self.pass(pass))
        }

        fn run_on(&mut self, count: u32) -> OrderRun<u32> {
            run_on(count, |pass| self.pass(pass).map(|()| Onward::Next))
        }
    }

    #[test]
    fn a_count_of_one_runs_one_pass_at_index_0() {
        let mut accepting = Recording::accepting();
        assert_eq!(
            accepting.run(1),
            OrderRun {
                done: 1,
                stopped: None
            }
        );
        assert_eq!(
            accepting.seen,
            [Pass {
                index: 0,
                count: 1,
                alerted: false
            }]
        );
        let mut refusing = Recording::refusing_at(0);
        assert_eq!(
            refusing.run(1),
            OrderRun {
                done: 0,
                stopped: Some(Stopped {
                    pass: 0,
                    refusal: 0
                })
            }
        );
        assert_eq!(
            refusing.seen,
            [Pass {
                index: 0,
                count: 1,
                alerted: false
            }]
        );
    }

    #[test]
    fn a_single_pass_is_index_0_of_a_count_of_1() {
        assert_eq!(
            Pass::SINGLE,
            Pass {
                index: 0,
                count: 1,
                alerted: false
            }
        );
        let mut accepting = Recording::accepting();
        accepting.run(1);
        assert_eq!(accepting.seen, [Pass::SINGLE]);
    }

    #[test]
    fn a_refusal_part_way_stops_the_loop_with_its_pass() {
        let mut refusing = Recording::refusing_at(2);
        assert_eq!(
            refusing.run(5),
            OrderRun {
                done: 2,
                stopped: Some(Stopped {
                    pass: 2,
                    refusal: 2
                })
            }
        );
        let at = |index| Pass {
            index,
            count: 5,
            alerted: false,
        };
        assert_eq!(refusing.seen, [at(0), at(1), at(2)], "none after it");
    }

    #[test]
    fn every_pass_accepted_runs_the_whole_count() {
        let mut accepting = Recording::accepting();
        assert_eq!(
            accepting.run(4),
            OrderRun {
                done: 4,
                stopped: None
            }
        );
        let at = |index| Pass {
            index,
            count: 4,
            alerted: false,
        };
        assert_eq!(accepting.seen, [at(0), at(1), at(2), at(3)]);
    }

    #[test]
    fn a_count_of_none_runs_no_pass() {
        let mut accepting = Recording::accepting();
        assert_eq!(
            accepting.run(0),
            OrderRun {
                done: 0,
                stopped: None
            }
        );
        assert_eq!(accepting.seen, []);
    }

    #[test]
    fn the_mock_refusing_only_at_an_index_accepts_the_passes_around_it() {
        let mut refusing = Recording::refusing_only_at(1);
        let at = |index| Pass {
            index,
            count: 3,
            alerted: false,
        };
        assert_eq!(refusing.pass(at(0)), Ok(()));
        assert_eq!(refusing.pass(at(1)), Err(1));
        assert_eq!(refusing.pass(at(2)), Ok(()));
        let mut from_here_on = Recording::refusing_at(1);
        assert_eq!(from_here_on.pass(at(0)), Ok(()));
        assert_eq!(from_here_on.pass(at(1)), Err(1));
        assert_eq!(from_here_on.pass(at(2)), Err(2));
    }

    #[test]
    fn running_on_hands_every_pass_and_marks_those_after_a_refusal_alerted() {
        let mut refusing = Recording::refusing_only_at(2);
        assert_eq!(
            refusing.run_on(5),
            OrderRun {
                done: 4,
                stopped: Some(Stopped {
                    pass: 2,
                    refusal: 2
                })
            }
        );
        let at = |index, alerted| Pass {
            index,
            count: 5,
            alerted,
        };
        assert_eq!(
            refusing.seen,
            [
                at(0, false),
                at(1, false),
                at(2, false),
                at(3, true),
                at(4, true)
            ]
        );
    }

    #[test]
    fn running_on_keeps_the_first_refusal_when_later_passes_refuse_too() {
        let mut refusing = Recording::refusing_at(1);
        assert_eq!(
            refusing.run_on(4),
            OrderRun {
                done: 1,
                stopped: Some(Stopped {
                    pass: 1,
                    refusal: 1
                })
            }
        );
        let at = |index, alerted| Pass {
            index,
            count: 4,
            alerted,
        };
        assert_eq!(
            refusing.seen,
            [at(0, false), at(1, false), at(2, true), at(3, true)]
        );
    }

    #[test]
    fn running_on_with_no_refusal_or_no_count_is_the_plain_loop() {
        let mut on = Recording::accepting();
        let mut plain = Recording::accepting();
        assert_eq!(
            on.run_on(4),
            OrderRun {
                done: 4,
                stopped: None
            }
        );
        assert_eq!(on.run_on(4), plain.run(4));
        assert_eq!(on.seen[4..], plain.seen[..]);
        assert!(plain.seen.iter().all(|pass| !pass.alerted));
        let mut none = Recording::accepting();
        assert_eq!(
            none.run_on(0),
            OrderRun {
                done: 0,
                stopped: None
            }
        );
        assert_eq!(none.seen, []);
    }

    #[test]
    fn running_on_from_an_index_goes_on_from_the_one_after_it() {
        let mut seen = Vec::new();
        let mut restarted = false;
        let run = run_on(4, |pass: Pass| -> Result<Onward, u32> {
            seen.push(pass.index);
            if pass.index == 2 && !restarted {
                restarted = true;
                return Ok(Onward::From(0));
            }
            Ok(Onward::Next)
        });
        assert_eq!(
            run,
            OrderRun {
                done: 6,
                stopped: None
            }
        );
        assert_eq!(seen, [0, 1, 2, 1, 2, 3]);
    }

    #[test]
    fn running_on_from_the_list_s_end_stops_unless_the_count_is_above_it() {
        let from_the_end = |count| {
            let mut seen = Vec::new();
            let run = run_on(count, |pass: Pass| -> Result<Onward, u32> {
                seen.push((pass.index, pass.count));
                Ok(if pass.index == 0 {
                    Onward::From(512)
                } else {
                    Onward::Next
                })
            });
            (run, seen)
        };
        let (run, seen) = from_the_end(3);
        assert_eq!(
            run,
            OrderRun {
                done: 1,
                stopped: None
            }
        );
        assert_eq!(seen, [(0, 3)]);
        let (run, seen) = from_the_end(515);
        assert_eq!(
            run,
            OrderRun {
                done: 3,
                stopped: None
            }
        );
        assert_eq!(seen, [(0, 515), (513, 515), (514, 515)]);
    }

    #[test]
    fn a_restart_keeps_the_first_refusal_and_the_alerted_flag() {
        let mut seen = Vec::new();
        let mut visits = [0u32; 4];
        let run = run_on(4, |pass: Pass| -> Result<Onward, u32> {
            seen.push((pass.index, pass.alerted));
            let index = usize::try_from(pass.index).unwrap();
            visits[index] += 1;
            let first = visits[index] == 1;
            match pass.index {
                1 if first => Err(1),
                2 if first => Ok(Onward::From(0)),
                _ => Ok(Onward::Next),
            }
        });
        assert_eq!(
            run,
            OrderRun {
                done: 5,
                stopped: Some(Stopped {
                    pass: 1,
                    refusal: 1
                })
            }
        );
        assert_eq!(
            seen,
            [
                (0, false),
                (1, false),
                (2, true),
                (1, true),
                (2, true),
                (3, true)
            ]
        );
    }

    #[test]
    fn counted_gives_the_first_passs_refusal_or_the_count_done() {
        let stopped_at = |pass| OrderRun {
            done: pass,
            stopped: Some(Stopped {
                pass,
                refusal: "refused",
            }),
        };
        assert_eq!(stopped_at(0).counted(), Err("refused"));
        assert_eq!(stopped_at(1).counted(), Ok(1));
        assert_eq!(stopped_at(3).counted(), Ok(3));
        let whole: OrderRun<&str> = OrderRun {
            done: 4,
            stopped: None,
        };
        assert_eq!(whole.counted(), Ok(4));
    }
}
