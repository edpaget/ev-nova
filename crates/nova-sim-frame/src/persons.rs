//! A person's hail quote said, for the flight's message line
//! ([`PersonQuote`]), from `nova_sim::session::persons`, which re-exports
//! it and keeps the persons in flight.

use nova_sim_base::shown::person::QUOTE_SHOWN_TICKS;
use nova_sim_state::shown::combat::NpcId;

/// A person's hail quote said, for the flight's message line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersonQuote {
    /// The NPC that said it.
    pub npc: NpcId,
    /// Its words, its tags read.
    pub text: String,
}

impl PersonQuote {
    /// How long the quote shows: [`QUOTE_SHOWN_TICKS`] (@0x4464d), the
    /// one count the session sets the message line to and the flight
    /// shows it for.
    #[must_use]
    pub fn ticks(&self) -> u16 {
        QUOTE_SHOWN_TICKS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_quote_shows_for_as_long_as_a_hail_quote() {
        let quote = PersonQuote {
            npc: NpcId(0),
            text: String::new(),
        };
        assert_eq!(quote.ticks(), QUOTE_SHOWN_TICKS);
    }
}
