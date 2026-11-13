//! How long a person's hail quote shows ([`QUOTE_SHOWN_TICKS`]), from
//! `nova_sim_story::person`, which re-exports it and keeps the quotes'
//! rules.

/// How long a hail quote shows, and keeps any other from being said at
/// random, in ticks (420 frames, 0x1a4 @0x4464d).
pub const QUOTE_SHOWN_TICKS: u16 = 420;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_quote_shows_for_420_ticks() {
        assert_eq!(QUOTE_SHOWN_TICKS, 420);
    }
}
