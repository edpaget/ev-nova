//! The new game's conversations, wired to `nova-ink`'s Ink runtime: the
//! dialog loader every flight's session plays them through ([`loader`]),
//! and the check of each stellar's knot as the data opens ([`InkCheck`]).
//!
//! Both are one forward to `nova_ink::InkLoader`: the decisions are the
//! dialog port's (`nova_sim::dialog`) and `nova-data`'s
//! (`nova_data::new_game::dialogs`).

use std::fmt;
use std::rc::Rc;

use nova_data::new_game::dialogs::DialogCheck;
use nova_ink::InkLoader;
use nova_sim::dialog::DialogLoader;

/// The loader the game's sessions open the stellars' conversations
/// with, its runtime's random draws seeded from the run's `seed`.
#[must_use]
pub fn loader(seed: u64) -> Rc<dyn DialogLoader> {
    Rc::new(InkLoader::seeded(ink_seed(seed)))
}

/// A dialog loader the router keeps for every flight, shown in a
/// `Debug` by its port alone.
#[derive(Clone)]
pub struct SharedLoader(pub Rc<dyn DialogLoader>);

impl fmt::Debug for SharedLoader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DialogLoader")
    }
}

/// `seed` as the Ink runtime's seed: its low 31 bits, so it is never
/// negative.
fn ink_seed(seed: u64) -> i32 {
    i32::try_from(seed & 0x7fff_ffff).unwrap_or_default()
}

/// [`DialogCheck`] over `nova-ink`: a knot passes when the story opens and
/// it can be entered.
#[derive(Clone, Copy, Debug, Default)]
pub struct InkCheck;

impl DialogCheck for InkCheck {
    fn check(&self, compiled: &str, knot: &str) -> Result<(), String> {
        InkLoader::seeded(0)
            .open(compiled)
            .and_then(|mut dialog| dialog.enter(knot))
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use nova_data::new_game::dialogs::fixture::{DOCK_JOB_JSON, check_contract};

    use super::*;

    #[test]
    fn the_ink_check_meets_the_contract() {
        assert_eq!(check_contract(&InkCheck), Ok(()));
    }

    #[test]
    fn the_loader_plays_the_fixture_s_offer() {
        let mut dialog = loader(7).open(DOCK_JOB_JSON).expect("opens");
        dialog.enter("job_offer").expect("enters");
        let first = dialog.step().expect("steps").expect("a line");
        assert_eq!(first.text, "The dockmaster looks up from a slate.");
    }

    #[test]
    fn a_shared_loader_debugs_by_its_port() {
        assert_eq!(format!("{:?}", SharedLoader(loader(1))), "DialogLoader");
    }

    #[test]
    fn the_ink_seed_is_the_run_seed_s_low_31_bits() {
        assert_eq!(ink_seed(0), 0);
        assert_eq!(ink_seed(5), 5);
        assert_eq!(ink_seed(0x1_8000_0001), 1);
        assert_eq!(ink_seed(u64::MAX), i32::MAX);
    }
}
