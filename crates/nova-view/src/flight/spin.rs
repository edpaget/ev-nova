//! The index a spinning shot's frames start on, as `_SpawnShot` sets it
//! (shot +0x34, @0x3e96e-0x3e9b4): 0 for a weapon of `Flags` 0x0004,
//! otherwise `Rand(36)`, 0 to 35, whatever its sheet holds, so a start at
//! or past the sheet's frame count shows its last frame until its first
//! step ([`shot_frame`](super::weapons::shot_frame)).
//!
//! The simulation keeps no frame of a shot, so the view draws each start
//! the first time it sees the shot, on its own chance, never the
//! simulation's, and keeps it while the shot flies.

use std::collections::BTreeMap;

use nova_sim::Chance;
use nova_sim::combat::projectile::ShotId;

use super::weapons::SPINS;

/// `Flags`: the shot's frames start on the first (@0x3e976-0x3e985).
pub const FIRST_FRAME_START: u16 = 0x0004;

/// How many indexes a start is drawn from: `_Rand(0x24)` (@0x3e996).
pub const SPIN_STARTS: u32 = 36;

/// Each shot in flight's spin start, by its number.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SpinStarts {
    starts: BTreeMap<ShotId, u8>,
}

impl SpinStarts {
    /// Notes the shots in flight, each with its weapon's `Flags`: a
    /// spinning shot (`Flags` 0x0001) seen for the first time draws its
    /// start on `chance`, 0 under [`FIRST_FRAME_START`]; a shot no longer
    /// among them is forgotten. The original draws a start for every shot,
    /// but only a spinning one's is ever shown, so the others draw
    /// nothing and start at 0.
    pub fn step(
        &mut self,
        shots: impl IntoIterator<Item = (ShotId, u16)>,
        chance: &mut dyn Chance,
    ) {
        let mut kept = BTreeMap::new();
        for (id, flags) in shots {
            let start = self.starts.get(&id).copied().unwrap_or_else(|| {
                if flags & SPINS != 0 && flags & FIRST_FRAME_START == 0 {
                    u8::try_from(chance.below(SPIN_STARTS)).unwrap_or(0)
                } else {
                    0
                }
            });
            kept.insert(id, start);
        }
        self.starts = kept;
    }

    /// Shot `id`'s start; 0 for one not seen yet.
    #[must_use]
    pub fn start(&self, id: ShotId) -> u8 {
        self.starts.get(&id).copied().unwrap_or(0)
    }

    /// How many shots it keeps a start for.
    #[must_use]
    pub fn len(&self) -> usize {
        self.starts.len()
    }

    /// Whether it keeps none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.starts.is_empty()
    }

    /// Forgets every start, as the shots are cleared.
    pub fn clear(&mut self) {
        self.starts.clear();
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::*;

    /// Gives its draws in turn, recording what it was asked.
    struct Draws {
        draws: VecDeque<u32>,
        asked: Vec<u32>,
    }

    impl Draws {
        fn of(draws: &[u32]) -> Self {
            Self {
                draws: draws.iter().copied().collect(),
                asked: Vec::new(),
            }
        }
    }

    impl Chance for Draws {
        fn fires(&mut self, _percent: u8) -> bool {
            false
        }

        fn below(&mut self, n: u32) -> u32 {
            self.asked.push(n);
            self.draws.pop_front().expect("a draw")
        }
    }

    #[test]
    fn a_shot_draws_its_start_below_36_once_and_keeps_it() {
        let mut starts = SpinStarts::default();
        let mut chance = Draws::of(&[20, 3]);
        starts.step([(ShotId(1), SPINS)], &mut chance);
        assert_eq!(starts.start(ShotId(1)), 20);
        starts.step(
            [(ShotId(1), SPINS), (ShotId(2), SPINS | 0x0002)],
            &mut chance,
        );
        assert_eq!((starts.start(ShotId(1)), starts.start(ShotId(2))), (20, 3));
        assert_eq!(chance.asked, [36, 36], "one draw a shot");
        assert_eq!(starts.start(ShotId(9)), 0, "one not seen");
    }

    #[test]
    fn a_shot_of_flags_0x0004_always_starts_at_0_and_draws_nothing() {
        let mut starts = SpinStarts::default();
        let mut chance = Draws::of(&[]);
        starts.step([(ShotId(1), FIRST_FRAME_START | SPINS)], &mut chance);
        assert_eq!(starts.start(ShotId(1)), 0);
        assert!(chance.asked.is_empty());
        assert_eq!(starts.len(), 1);
    }

    #[test]
    fn a_shot_that_does_not_spin_starts_at_0_and_draws_nothing() {
        let mut starts = SpinStarts::default();
        let mut chance = Draws::of(&[]);
        starts.step([(ShotId(1), 0x0002)], &mut chance);
        assert_eq!(starts.start(ShotId(1)), 0);
        assert!(chance.asked.is_empty());
    }

    #[test]
    fn a_shot_no_longer_in_flight_is_forgotten() {
        let mut starts = SpinStarts::default();
        let mut chance = Draws::of(&[7, 8, 9]);
        starts.step([(ShotId(1), SPINS), (ShotId(2), SPINS)], &mut chance);
        assert_eq!(starts.len(), 2);
        assert!(!starts.is_empty());
        starts.step([(ShotId(2), SPINS)], &mut chance);
        assert_eq!(starts.len(), 1);
        assert_eq!(starts.start(ShotId(1)), 0, "forgotten");
        assert_eq!(starts.start(ShotId(2)), 8);
        starts.step([(ShotId(1), SPINS)], &mut chance);
        assert_eq!(starts.start(ShotId(1)), 9, "a number seen anew draws anew");
        starts.clear();
        assert!(starts.is_empty());
    }
}
