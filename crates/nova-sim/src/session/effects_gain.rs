//! The frame's gain the stellar defence guns count their reloads down by
//! (`_HandleStellarWeapons`' `_gSpeedMult`, @0x2f5a1, see
//! `nova_sim_base::speed_mult`).
//!
//! The guns reach it only through the fight's field: `Session::field`
//! fills [`Field::speed_mult`](crate::combat::Field::speed_mult) with
//! `Session::speed_mult`, `Session::with_fighters` builds the arena from
//! it, and `Combat::fire_stellar_guns` keeps the arena's field for the
//! guns, on either pass. The boxes, asteroids, fly-ins, reinforcements and
//! self-repair take the gain as a value, each pinned in the module that
//! hands it on (`cargo_boxes`, `asteroids`, `brackets`, `frame::hud`,
//! `reinforcements`, `self_repair` and `player_repair`).
//!
//! This test pins the wiring only: its expected value is the upstream
//! `Combat::fire_stellar_guns` at the same gain, on a copy of the state
//! before the pass, and differs from it at a full frame's.

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use crate::SpeedMult;
    use crate::catalog::StellarId;
    use crate::chance::NeverFires;
    use crate::combat::stellar_guns::{GunTimer, Standing, StellarGun};
    use crate::combat::{Field, GamePass, Rules};
    use crate::flight::Controls;
    use crate::geometry::Vec2;
    use crate::session::Session;
    use crate::session::stellar_guns::tests::{GUN, beside_128, gunned};
    use crate::testkit::{FakePilotCatalog, catalog, work};

    const HALF: SpeedMult = SpeedMult::new(0.5);

    /// The reload `session`'s stellar 128 counts down to from `timer` on
    /// a pass of `Combat::fire_stellar_guns` at `gain`, on a copy of its
    /// fight: a reloading gun reads no mark, so none is given.
    fn counted(session: &Session, timer: GunTimer, gain: SpeedMult) -> f32 {
        let mut guns = [StellarGun {
            stellar: StellarId(128),
            position: Vec2::new(30.0, -40.0),
            weapon: GUN,
            govt: None,
            flags2: 0,
            dominated: false,
            defended: false,
            standing: Standing::default(),
            timer,
        }];
        let arena = crate::combat::Arena {
            pass: GamePass::Advance { double_time: false },
            arsenal: &session.statics.arsenal,
            govts: &session.statics.govts,
            rules: Rules::default(),
            field: Field {
                speed_mult: gain,
                ..Field::default()
            },
        };
        let mut combat = session.space.combat.clone();
        combat.fire_stellar_guns(&[], arena, &mut guns, &mut NeverFires);
        guns[0].timer.reload
    }

    /// A gun of `Reload` 30 that fired at the criminal beside it.
    fn reloading() -> (FakePilotCatalog, Session) {
        let catalog = gunned(catalog(), &[128], 30);
        let mut session = beside_128(&catalog, -7);
        work(&mut session, &catalog, Controls::default());
        (catalog, session)
    }

    #[test]
    fn a_reloading_gun_counts_down_at_the_frames_gain() {
        let (catalog, mut session) = reloading();
        let timer = session.space.gun_timers[&StellarId(128)];
        assert!(timer.reload > 0.0, "reloading: {timer:?}");
        session.at_gain(HALF);
        let expected = counted(&session, timer, HALF);
        work(&mut session, &catalog, Controls::default());
        assert_eq!(session.space.gun_timers[&StellarId(128)].reload, expected);
        assert_ne!(
            expected,
            counted(&session, timer, SpeedMult::FULL),
            "the gain shows"
        );
    }
}
