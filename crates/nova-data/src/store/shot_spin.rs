//! Finding a shot's sprite.

/// The `spïn` of a weapon's first `Graphic` (the Bible: 3000-3255).
const FIRST_SHOT_SPIN: i16 = 3000;

/// The `spïn` ID a shot of `wëap` `Graphic` `graphic` is drawn with: the
/// original clones `spïn` 3000 + `Graphic` (`_SpawnShot`
/// @0x3f1a1-0x3f1c1). `None` for a `Graphic` below 0, which draws
/// nothing; a `Graphic` past the last `spïn` ID saturates at it.
#[must_use]
pub const fn shot_spin_id(graphic: i16) -> Option<i16> {
    if graphic < 0 {
        None
    } else {
        Some(FIRST_SHOT_SPIN.saturating_add(graphic))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shots_spin_is_3000_plus_its_graphic_and_none_below_0() {
        assert_eq!(shot_spin_id(-1), None);
        assert_eq!(shot_spin_id(i16::MIN), None);
        assert_eq!(shot_spin_id(0), Some(3000));
        assert_eq!(shot_spin_id(7), Some(3007));
        assert_eq!(shot_spin_id(255), Some(3255));
        assert_eq!(shot_spin_id(i16::MAX), Some(i16::MAX), "saturates");
    }
}
