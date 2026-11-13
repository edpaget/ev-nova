//! What an order loop did ([`OrderRun`]), where it stopped ([`Stopped`])
//! and the pass a check is asked at ([`Pass`]), from
//! `nova_sim_port::order_loop`, which re-exports them and keeps the loop's
//! drivers.

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
    /// @0x5ca3f, @0x5ce5a). Only `nova_sim_port::order_loop::run_on` sets
    /// it; `nova_sim_port::order_loop::run` stops at the first refusal, so no
    /// pass ever follows one.
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

#[cfg(test)]
mod tests {
    use super::*;

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
