//! The explosions an in-flight move kills (crons-news-ranks-fidelity
//! phase 44). All addresses are in `EV Nova.app`'s `EV Nova` (i386).
//!
//! **The original.** An in-flight `M` or `N` sets `_killExplods`
//! (`_EvalSetExp` @0x158ba, @0x1597e), and so does `N`'s tail out of
//! range, which a move to a system with no record reaches too
//! (@0x1596e-0x15993). The flag lasts to the end of the `_PlayGame` frame
//! (@0x4637b-0x463a6), so that frame's `_HandleExplods` hides and frees
//! every explosion live (@0x2f5ff-0x2f607, @0x2f844-0x2f85e) before the
//! sprite world is drawn: the explosions made earlier in the frame are
//! never seen. A jump's explorations and days run inside one
//! `_HandlePlayer` call, so a move anywhere in the jump frees every
//! explosion of the jump, a `Y`'s on its days among them, at the
//! arrival's `_PrepareAllSprites` (@0x6c59c, `_HandleExplods` @0x3a7ce)
//! and again at the frame's own pass (@0x444e1). Their sounds have played
//! already.
//!
//! **Here.** The fight drops the explosions it has reported and the view
//! has not taken ([`Combat::kill_explosions`](crate::combat::Combat::kill_explosions)):
//!
//! - out of a jump, as the in-flight move's row of the entry table says
//!   (`Reset::Explosions`, in the `entry` module), at the settling, before
//!   the view takes the frame's events;
//! - in a jump, once its entry has run, when its window noted a move or a
//!   move's tail (see the `jump_moves` module).
//!
//! The view clears what it has drawn already as it lays out the scene the
//! move enters.

use super::Session;
use super::jump_moves::JumpWindow;

impl Session {
    /// Kills the explosions pending (see the module docs).
    pub(super) fn kill_explosions(&mut self) {
        self.space.combat.kill_explosions();
    }

    /// The jump's tail: kills the explosions the jump made when `window`
    /// noted an in-flight move or a move's tail (see the module docs).
    pub(super) fn kill_jump_explosions(&mut self, window: JumpWindow) {
        if window.killed {
            self.kill_explosions();
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::catalog::{BoomId, Explosion, SoundId, StellarLife, SystemId};
    use crate::chance::NeverFires;
    use crate::combat::CombatEvent;
    use crate::control::SetExpr;
    use crate::pilot::Pilot;
    use crate::session::Session;
    use crate::session::crons::tests::scripted;
    use crate::sound::SimSound;
    use crate::testkit::{FakePilotCatalog, catalog, jump, life, planet};

    /// [`catalog`] with 132 and 133 (stellar 150), and 128 and 140
    /// exploding in `bööm` 132, which sounds `snd ` 302; and `crons`.
    fn exploding(crons: Vec<crate::catalog::CronRecord>) -> FakePilotCatalog {
        let mut catalog = FakePilotCatalog {
            crons,
            boom_sounds: vec![(BoomId(132), SoundId(302))],
            ..catalog().living([128, 129, 140].map(|id| StellarLife {
                strength: 100,
                explosion: Explosion::decode(4),
                ..life(id, 2)
            }))
        };
        catalog.systems.extend([SystemId(132), SystemId(133)]);
        catalog
            .sites
            .push((SystemId(133), vec![planet(150, 300.0, -200.0)]));
        catalog
    }

    fn flown(catalog: &FakePilotCatalog) -> Session {
        let mut session =
            Session::fly(catalog, Pilot::new(catalog, "").expect("starts")).expect("flies");
        let _ = session.settle_script(catalog, &mut NeverFires);
        session.take_sounds();
        session.take_combat_events();
        session
    }

    fn explosions(session: &mut Session) -> usize {
        session
            .take_combat_events()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    CombatEvent::Exploded { .. } | CombatEvent::ExplodedHeard { .. }
                )
            })
            .count()
    }

    fn sounded(session: &mut Session) -> usize {
        session
            .take_sounds()
            .iter()
            .filter(|sound| matches!(sound, SimSound::Explosion { .. }))
            .count()
    }

    #[test]
    fn a_y_and_an_m_on_a_jumps_day_leave_no_explosion_but_its_sound() {
        for script in ["Y140 M133", "M133 Y150 Y140"] {
            let mut catalog = exploding(vec![scripted(128, 1, script, "")]);
            catalog.lives.push(StellarLife {
                strength: 100,
                explosion: Explosion::decode(4),
                ..life(150, 2)
            });
            let mut session = flown(&catalog);
            assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(133)));
            assert_eq!(explosions(&mut session), 0, "{script}: killed");
            assert_eq!(sounded(&mut session), 1, "{script}: heard");
        }
        // With no move, the arrival stellar's explosion stays.
        let catalog = exploding(vec![scripted(128, 1, "Y140", "")]);
        let mut session = flown(&catalog);
        assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(131)));
        assert_eq!(explosions(&mut session), 1);
    }

    fn run_and_settle(session: &mut Session, catalog: &FakePilotCatalog, script: &str) {
        session.run_set(&SetExpr::parse(script).expect("parses"), &mut NeverFires);
        let _ = session.settle_script(catalog, &mut NeverFires);
    }

    #[test]
    fn a_y_then_an_in_flight_m_shows_no_explosion() {
        let catalog = exploding(Vec::new());
        let mut session = flown(&catalog);
        run_and_settle(&mut session, &catalog, "Y129 M131");
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(
            explosions(&mut session),
            0,
            "killed before the view takes it"
        );
        assert_eq!(sounded(&mut session), 1);
    }

    #[test]
    fn an_n_out_of_range_in_flight_kills_them_too() {
        let catalog = exploding(Vec::new());
        let mut session = flown(&catalog);
        run_and_settle(&mut session, &catalog, "Y129 N3000");
        assert_eq!(session.system(), SystemId(130));
        assert_eq!(explosions(&mut session), 0);
        // A `Y` alone keeps its explosion for the view.
        run_and_settle(&mut session, &catalog, "Y128");
        assert_eq!(explosions(&mut session), 1);
    }
}
