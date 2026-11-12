//! A bar visit's news ([`News`]), the `STR#` its fallbacks come from
//! ([`MISC_STRINGS`]) and the picture it is drawn over by default
//! ([`DEFAULT_NEWS_PICTURE`]), from `nova_sim_story::news`, which
//! re-exports them and keeps choosing the news.

/// `STR#` 2002, the miscellaneous strings the fallbacks come from.
pub const MISC_STRINGS: i16 = 2002;

/// The `PICT` the news is drawn over at an independent stellar, under a
/// government whose `NewsPic` is -1, or when the picture asked for is
/// missing (`_DoNewsDialog` @0x47f4a, @0x47f66, @0x47f98).
pub const DEFAULT_NEWS_PICTURE: i16 = 9000;

/// A bar visit's news: the commercial line and the story, raw, their
/// tags not filled.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct News {
    /// The commercial line.
    pub commercial: String,
    /// The story.
    pub story: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fallbacks_are_str_2002s_and_the_default_picture_is_pict_9000() {
        assert_eq!((MISC_STRINGS, DEFAULT_NEWS_PICTURE), (2002, 9000));
    }
}
