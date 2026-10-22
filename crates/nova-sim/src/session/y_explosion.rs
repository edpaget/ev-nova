//! `Y`'s explosion and its sound (crons-news-ranks-fidelity phase 44).
//! All addresses are in `EV Nova.app`'s `EV Nova` (i386).
//!
//! **The original.** `Y` (@0x15a38-0x15ae6) explodes the stellar when its
//! owner word (+0x14) is the player's system (+0x74, @0x15a64-0x15a6c),
//! with no landed test, when it is not destroyed already
//! (`_StellarIsDestroyed` @0x15a71) and its `ExplodType` (+0x46e) is set
//! (@0x15a85). The explosion is at the stellar's own position (+0x4,
//! `_XYShortToFloat` @0x15a8b), `_CreateExplosion(x, y, type, 0, 1)`
//! (@0x15a99-0x15ab4), which spawns it into the explod table and, its
//! flag set (@0x3f8ca), plays the `bööm`'s sound at once: `snd ` 300 + its
//! `SoundIndex` when that is 0-63 (@0x3f8d7-0x3f8e5, handles 0x3b5428
//! @0x3f90c), `_PlaySoundDistance(-1, snd, 6, &(x, y), player +0x18)`
//! (@0x3f8f1-0x3f91a). The countdown (@0x15ac8-0x15ad5) and the strength
//! (@0x15ade) follow.
//!
//! - On a jump's days the owner words are the jump's refresh's
//!   (@0x6bf3f): a stellar the refresh left unowned is held by no one
//!   until the entry's rebuild, so it never explodes. Only `_ResetPlayer`
//!   empties the explod table (@0x1d7fb), so an arrival stellar's
//!   explosion is on the arrival's first frame.
//! - Docked, the explosion is never drawn: the take-off's tail sets
//!   `_killExplods` (`_HandlePlayerDockRequest` @0x66f40-0x66f6f) and the
//!   take-off frame's `_HandleExplods` frees it (@0x2f5ff-0x2f607,
//!   @0x2f844-0x2f85e) before the sprite world is drawn (@0x45dff). Its
//!   sound has played while docked.
//!
//! **Here.** [`Session::explode_on_y`] reads the owner through the
//! galaxy's holder (`Chart::holder_of`), which leaves a stellar unowned
//! while a rebuild is awaited, and the stellar's own position from its
//! `spöb` (`Statics::stellar_position`), so a `Y` on a jump's day explodes
//! an arrival stellar, never a departure one. It raises
//! [`SimSound::Explosion`] at once, docked or not, from where the stellar
//! is to where the player is, rounded to whole pixels as the view hears a
//! fight. In flight it reports [`CombatEvent::ExplodedHeard`], which the
//! view draws with no sound of its own; docked it reports nothing. An
//! in-flight move kills it before the view takes it (see the
//! `killed_explosions` module).
//!
//! [`CombatEvent::ExplodedHeard`]: crate::combat::CombatEvent::ExplodedHeard

use super::Session;
use crate::catalog::{StellarId, StellarLife};
use crate::sound::{EXPLOSION_PRIORITY, SimSound};
use crate::story::wreck;

impl Session {
    /// `Y`'s explosion of `stellar`, whose life is `life`, destroyed
    /// before the `Y` or not (see the module docs): its sound at once, and
    /// in flight its explosion for the view.
    pub(super) fn explode_on_y(
        &mut self,
        stellar: StellarId,
        life: &StellarLife,
        destroyed_before: bool,
    ) {
        let owned =
            self.chart.holder_of(&self.pilot, &self.statics, stellar) == Some(self.pilot.system);
        let Some(explosion) = wreck::explodes_on_y(life, owned, destroyed_before) else {
            return;
        };
        let at = self.statics.stellar_position(stellar);
        if let Some(&sound) = self.statics.boom_sounds.get(&explosion.boom) {
            let from = at - self.ship.player.position;
            #[allow(clippy::cast_possible_truncation)] // within the map
            let offset = (from.x.round() as i32, from.y.round() as i32);
            self.out.sounds.push(SimSound::Explosion {
                sound,
                offset,
                priority: EXPLOSION_PRIORITY,
            });
        }
        if self.mode.landed().is_none() {
            self.space.combat.explode_heard(at, explosion);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::catalog::{BoomId, Explosion, SoundId, StellarId, StellarLife, SystemId};
    use crate::chance::NeverFires;
    use crate::combat::CombatEvent;
    use crate::control::SetExpr;
    use crate::control::{Bit, Test};
    use crate::geometry::Vec2;
    use crate::pilot::Pilot;
    use crate::rulebook::{RuleSource, Rulebook};
    use crate::session::Session;
    use crate::session::crons::tests::scripted;
    use crate::session::jump_exploration::tests::unowned_160;
    use crate::session::kept_rescues::tests::{by, rescued_in_130};
    use crate::sound::SimSound;
    use crate::testkit::{FakePilotCatalog, catalog, jump, land_now, life, planet};

    /// `bööm` 132, explosion type 4.
    const BOOM: i16 = 4;

    /// [`catalog`] with 140, 131's stellar, at (100, 200), and 128 (130's)
    /// and 140 exploding in `bööm` 132, which sounds `snd ` 302; and
    /// `crons`.
    fn exploding(crons: Vec<crate::catalog::CronRecord>) -> FakePilotCatalog {
        let mut catalog = FakePilotCatalog {
            crons,
            boom_sounds: vec![(BoomId(132), SoundId(302))],
            ..catalog().living([128, 129, 140].map(|id| StellarLife {
                strength: 100,
                explosion: Explosion::decode(BOOM),
                ..life(id, 2)
            }))
        };
        for (system, sites) in &mut catalog.sites {
            if *system == SystemId(131) {
                *sites = vec![planet(140, 100.0, 200.0)];
            }
        }
        catalog
    }

    fn flown(catalog: &FakePilotCatalog, rules: Rulebook) -> Session {
        let mut session = Session::fly(catalog, Pilot::new(catalog, "").expect("starts"))
            .expect("flies")
            .with_rules(rules);
        let _ = session.settle_script(catalog, &mut NeverFires);
        session.take_sounds();
        session.take_combat_events();
        session
    }

    fn run(session: &mut Session, script: &str) {
        session.run_set(&SetExpr::parse(script).expect("parses"), &mut NeverFires);
    }

    /// The explosions among `events` a `Y` reports, where each is.
    fn heard_booms(events: &[CombatEvent]) -> Vec<Vec2> {
        events
            .iter()
            .filter_map(|event| match *event {
                CombatEvent::ExplodedHeard { at, explosion }
                    if Some(explosion) == Explosion::decode(BOOM) =>
                {
                    Some(at)
                }
                _ => None,
            })
            .collect()
    }

    /// The explosion sounds among `sounds`.
    fn explosions(sounds: &[SimSound]) -> Vec<SimSound> {
        sounds
            .iter()
            .copied()
            .filter(|sound| matches!(sound, SimSound::Explosion { .. }))
            .collect()
    }

    fn heard(offset: (i32, i32)) -> SimSound {
        SimSound::Explosion {
            sound: SoundId(302),
            offset,
            priority: 6,
        }
    }

    #[test]
    fn a_crons_y_on_a_jumps_day_explodes_an_arrival_stellar_and_not_a_departure_one() {
        let catalog = exploding(vec![scripted(128, 1, "Y140 Y128", "")]);
        let mut session = flown(&catalog, Rulebook::default());
        assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(131)));
        let player = session.player().position;
        assert!(
            (player - Vec2::new(-1350.0, 0.0)).length() < 1e-2,
            "the arrival block's: {player:?}"
        );
        assert_eq!(
            heard_booms(&session.take_combat_events()),
            [Vec2::new(100.0, 200.0)],
            "140's, at its own place; not 130's 128"
        );
        assert_eq!(
            explosions(&session.take_sounds()),
            [heard((1450, 200))],
            "once, heard from the arrival"
        );
        for id in [128, 140] {
            assert!(
                session.galaxy().destroyed(StellarId(id)),
                "{id} destroyed either way"
            );
        }
    }

    #[test]
    fn a_y_on_an_unrescued_stellar_on_a_jumps_day_does_nothing_but_destroy_it() {
        // 160, which 131 lists and no system owns or rescues: during the
        // days the refresh left it unowned (@0x6bf3f), until the entry's
        // rebuild, so no `Y` there explodes it. By the other reading the
        // holder derives the player's system, 131.
        let mut catalog = unowned_160().living([StellarLife {
            strength: 100,
            explosion: Explosion::decode(BOOM),
            ..life(160, 2)
        }]);
        catalog.boom_sounds = vec![(BoomId(132), SoundId(302))];
        let mut on_the_day = scripted(128, 1, "Y160", "");
        on_the_day.enable_on = Test::parse("b30");
        catalog.crons.push(on_the_day);
        for source in RuleSource::ALL {
            let mut session = rescued_in_130(&catalog, source);
            session.pilot.bits_mut().set(Bit::new(30).expect("a bit"));
            session.take_sounds();
            session.take_combat_events();
            assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(131)));
            assert_eq!(
                heard_booms(&session.take_combat_events()),
                by(source, Vec::new(), vec![Vec2::new(400.0, 300.0)]),
                "{source:?}"
            );
            assert_eq!(
                explosions(&session.take_sounds()).len(),
                by(source, 0, 1),
                "{source:?}"
            );
            assert!(session.galaxy().destroyed(StellarId(160)), "{source:?}");
        }
    }

    #[test]
    fn a_docked_y_sounds_at_once_and_shows_no_explosion() {
        // Docked at 128 (30, -40), 129 (2000, 0) is 130's.
        let catalog = exploding(Vec::new());
        let mut session = flown(&catalog, Rulebook::default());
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        session.take_sounds();
        session.take_combat_events();
        run(&mut session, "Y129");
        let docked = session.player().position;
        assert_eq!(docked, Vec2::new(30.0, -40.0));
        assert_eq!(explosions(&session.take_sounds()), [heard((1970, 40))]);
        assert_eq!(session.take_combat_events(), [], "never drawn");
        let _ = session.take_off(&catalog, &mut NeverFires);
        assert_eq!(heard_booms(&session.take_combat_events()), [], "nor after");

        // Another system's stellar makes neither, docked or flying.
        let mut session = flown(&catalog, Rulebook::default());
        run(&mut session, "Y140");
        assert_eq!(explosions(&session.take_sounds()), []);
        assert_eq!(session.take_combat_events(), []);
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        session.take_sounds();
        run(&mut session, "U140 Y140");
        assert_eq!(explosions(&session.take_sounds()), []);
        assert_eq!(session.take_combat_events(), []);
    }

    #[test]
    fn a_y_sounds_only_for_a_boom_with_a_sound() {
        // `_CreateExplosion` plays `SoundIndex` 0-63 alone
        // (@0x3f8dc-0x3f8e5); a `bööm` of none still explodes.
        let catalog = FakePilotCatalog {
            boom_sounds: Vec::new(),
            ..exploding(Vec::new())
        };
        let mut session = flown(&catalog, Rulebook::default());
        run(&mut session, "Y128");
        assert_eq!(explosions(&session.take_sounds()), []);
        assert_eq!(
            heard_booms(&session.take_combat_events()),
            [Vec2::new(30.0, -40.0)]
        );
    }

    #[test]
    fn a_y_of_a_stellar_destroyed_before_or_with_no_explod_type_is_silent() {
        let booming = exploding(Vec::new());
        let mut session = flown(&booming, Rulebook::default());
        run(&mut session, "Y128");
        assert_eq!(explosions(&session.take_sounds()).len(), 1);
        run(&mut session, "Y128");
        assert_eq!(explosions(&session.take_sounds()), [], "destroyed already");
        assert_eq!(heard_booms(&session.take_combat_events()).len(), 1);

        let catalog = FakePilotCatalog {
            boom_sounds: vec![(BoomId(132), SoundId(302))],
            ..catalog().living([life(128, 2)])
        };
        let mut session = flown(&catalog, Rulebook::default());
        run(&mut session, "Y128");
        assert_eq!(explosions(&session.take_sounds()), [], "no ExplodType");
        assert_eq!(session.take_combat_events(), []);
    }
}
