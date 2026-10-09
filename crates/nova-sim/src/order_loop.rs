//! The order loop: an order of a count, run one pass at a time.
//!
//! The original's outfitter buys or sells a count one at a time: the buy
//! loop (`_DoOutfitDialog` @0x5c1bd-0x5c2ff) re-runs `_CanBuyOutfitItem`
//! after each grant, and the sell loop (@0x5c6ee-0x5d1b1) checks each
//! sale afresh. A plain click is a count of 1 (@0x5c69f), so a single
//! order and a counted one take the same loop.
//!
//! [`run`] is a pure driver: it hands each pass its [`Pass`], the pass
//! index and the whole count, and stops at the first refusal. What one
//! pass does is the caller's, through the pass function it supplies (the
//! port for "run one pass"): the session's checks the order and applies
//! it. The loop's [`OrderRun`] keeps where and why it stopped;
//! [`OrderRun::counted`] gives the result a counted order reports.

/// One pass of a counted order: its index from 0, and the whole count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pass {
    /// The pass's index, from 0.
    pub index: u32,
    /// The whole count the order asks for.
    pub count: u32,
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
    /// The pass refused, and why, when one stopped the loop.
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
/// (@0x5c1bd-0x5c2ff) and sell loop (@0x5c6ee-0x5d1b1) do here; a plain
/// click is a count of 1 (@0x5c69f). A count of none runs no pass.
pub fn run<R>(count: u32, mut pass: impl FnMut(Pass) -> Result<(), R>) -> OrderRun<R> {
    let mut done = 0;
    while done < count {
        if let Err(refusal) = pass(Pass { index: done, count }) {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A pass that records each [`Pass`] it is handed and refuses with
    /// its index from `refuse_at` on.
    struct Recording {
        seen: Vec<Pass>,
        refuse_at: Option<u32>,
    }

    impl Recording {
        fn accepting() -> Self {
            Self {
                seen: Vec::new(),
                refuse_at: None,
            }
        }

        fn refusing_at(index: u32) -> Self {
            Self {
                seen: Vec::new(),
                refuse_at: Some(index),
            }
        }

        fn pass(&mut self, pass: Pass) -> Result<(), u32> {
            self.seen.push(pass);
            match self.refuse_at {
                Some(at) if pass.index >= at => Err(pass.index),
                _ => Ok(()),
            }
        }

        fn run(&mut self, count: u32) -> OrderRun<u32> {
            run(count, |pass| self.pass(pass))
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
        assert_eq!(accepting.seen, [Pass { index: 0, count: 1 }]);
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
        assert_eq!(refusing.seen, [Pass { index: 0, count: 1 }]);
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
        let at = |index| Pass { index, count: 5 };
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
        let at = |index| Pass { index, count: 4 };
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
