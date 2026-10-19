//! The player's ionization (see the [`ionization`](crate::ionization)
//! module): what `_HandlePlayer` does with its ion charge each frame, and
//! the slowed turn and thrust `_ShipTurnRate` and `_ShipAccelRate` give it.
//!
//! **The capacity and rate** ([`Session::ion_capacity`],
//! [`Session::ion_rate`]): its class's `IonizeMax` and deionization rate,
//! with its Ion Absorbers' and Ion Dissipaters' (`_ShipIonizationFactor`
//! @0x3225, `_ShipDeionizationRate` @0x6ec3; see
//! `nova_sim::stats::IonStats`).
//!
//! **The turn and thrust** ([`Session::ionized`]): every reader of the
//! player's `_ShipTurnRate` and `_ShipAccelRate` reads them slowed by the
//! charge as it is when it reads them: its flight's
//! ([`Session::flying_handling`], `_HandlePlayer`'s five turn and eleven
//! thrust reads), the NPCs' view of it (`PlayerSide::handling`), and the
//! Player Info rows (`_PlayerInfoDialogUpdate` @0x52cd0, @0x52d90). The
//! turn is spared from J on, as the player's +0x50 is above 0 through the
//! pre-jump and the warp.
//!
//! **The frame's block** ([`Session::ionize_tick`], @0x6d7fd-0x6d96d):
//! after the move and the box's clamp (@0x6d43f-0x6d4aa), before the fuel
//! regenerates (@0x6d97a) and the afterburner's block (@0x6f970), so the
//! afterburner's push reads the charge the block left. It runs on every
//! `_HandlePlayer` frame but the early returns (@0x68853 and @0x6888c
//! while destroyed, @0x68ddd while the escape pod counts down, @0x692ce
//! landing and @0x6c625 on a jump's arrival), the frames the afterburner's
//! block skips too: so it sheds in the pre-jump and the warp's run in
//! hyperspace, disabled or dying, and not while destroyed or in the pod.
//! Double time's second pass runs it again, as `_HandlePlayer` runs on
//! both passes.
//!
//! The original caches the capacity and rate (0x3b5068, 0x3b50a8); the
//! stats are worked out afresh whenever the outfits change, so they need
//! no copy. None of this plays a sound.

use super::Session;
use crate::flight::Flyer;
use crate::handling::Handling;
use crate::ionization::{Ionization, deionize, factor, lowered_top, pull};

impl Session {
    /// The ion charge at which the player is fully ionized: its class's
    /// `IonizeMax` and its Ion Absorbers' (see the module docs).
    #[must_use]
    pub fn ion_capacity(&self) -> f32 {
        self.stats.ion.capacity(Flyer::Player)
    }

    /// The ion charge the player sheds a tick: its class's rate and its
    /// Ion Dissipaters' (see the module docs).
    #[must_use]
    pub fn ion_rate(&self) -> f32 {
        self.stats.ion.rate(Flyer::Player)
    }

    /// The player's ionization factor, `_ShipIonizationFactor`: its charge
    /// over its capacity ([`factor`]).
    #[must_use]
    pub fn ion_factor(&self) -> f32 {
        factor(self.ion.charge, self.ion_capacity())
    }

    /// What the player's handling reads of its ionization now: its
    /// charge, its factor, and its jump under way from J on.
    #[must_use]
    pub fn ionization(&self) -> Ionization {
        Ionization {
            charge: self.ion.charge,
            factor: self.ion_factor(),
            jumping: self.jump.is_some(),
        }
    }

    /// `handling` slowed by the player's charge as it is now
    /// ([`Ionization::slows`]): its turn spared from J on.
    pub(super) fn ionized(&self, handling: Handling) -> Handling {
        self.ionization().slows(handling)
    }

    /// Sets the player's ionization directly, for
    /// [`fixture::set_player_ion`](crate::fixture::set_player_ion).
    #[cfg(any(test, feature = "fixture"))]
    pub(crate) fn set_player_ion(&mut self, ion: crate::combat::ion::Ion) {
        self.ion = ion;
    }

    /// The player's frame's ion block (see the module docs): a charge
    /// above none sheds the rate and the velocity is pulled toward the top
    /// speed lowered by the charge left; one at or below none is cleared.
    /// What it took off is kept in the ion's `shed`, for the fight's
    /// lockout ([`Ion::as_block_found`](crate::combat::ion::Ion::as_block_found)).
    pub(super) fn ionize_tick(&mut self) {
        let found = self.ion.charge;
        let Some(charge) = deionize(found, self.ion_rate()) else {
            self.ion.charge = 0.0;
            self.ion.shed = found;
            return;
        };
        self.ion.charge = charge;
        self.ion.shed = found - charge;
        let cap = lowered_top(self.top_speed(), self.ion_factor());
        self.player.velocity = pull(self.player.velocity, cap);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::{OutfitId, ShipId, SystemId};
    use crate::combat::ShipRef;
    use crate::combat::hull::Condition;
    use crate::combat::projectile::ShotId;
    use crate::flight::{Controls, Face, Turn};
    use crate::geometry::Vec2;
    use crate::handling::ShipFields;
    use crate::stats::{AFTERBURNER, ION_ABSORBER, ION_DISSIPATER};
    use crate::testkit::{FAST, FakePilotCatalog, catalog, outfit};

    /// The fake catalog with ship 128 of `IonizeMax` 400 and `Deionize`
    /// 50, shedding 0.5 a tick.
    fn ionizing() -> FakePilotCatalog {
        FakePilotCatalog {
            ships: vec![(
                ShipId(128),
                Ok(ShipFields {
                    ionize_max: 400,
                    deionize: 50,
                    ..FAST
                }),
            )],
            ..catalog()
        }
    }

    /// [`ionizing`] with ship 128 carrying `outfits` as default items, each
    /// `oütf` 200 up of these mods.
    fn carrying(mods: &[&[(i16, i16)]], count: u16) -> FakePilotCatalog {
        let outfits: Vec<_> = (0..)
            .zip(mods)
            .map(|(i, mods)| outfit(200 + i, mods))
            .collect();
        let defaults = outfits.iter().map(|outfit| (outfit.id, count)).collect();
        FakePilotCatalog {
            defaults: vec![(ShipId(128), defaults)],
            outfits,
            ..ionizing()
        }
    }

    /// A session of `catalog` at rest at the centre facing 90°, past its
    /// first tick and with its box settled at top speed, its charge
    /// `charge`.
    fn flying(catalog: &FakePilotCatalog, charge: f32) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.tick(Controls::default());
        session.settle_box();
        session.player.position = Vec2::ZERO;
        session.player.velocity = Vec2::ZERO;
        session.player.heading = 90.0;
        session.ion.charge = charge;
        session
    }

    fn thrust() -> Controls {
        Controls {
            thrust: true,
            ..Controls::default()
        }
    }

    // The charge.

    #[test]
    fn an_ionized_player_sheds_its_rate_a_tick() {
        let mut session = flying(&ionizing(), 10.0);
        assert_eq!(session.ion_rate(), 0.5);
        session.tick(Controls::default());
        assert_eq!(session.ion().charge, 9.5);
        session.tick(Controls::default());
        assert_eq!(session.ion().charge, 9.0);
    }

    #[test]
    fn the_charge_dips_below_none_for_a_tick_then_clears() {
        let mut session = flying(&ionizing(), 0.25);
        session.tick(Controls::default());
        assert_eq!(session.ion().charge, -0.25);
        session.tick(Controls::default());
        assert_eq!(session.ion().charge, 0.0);
    }

    #[test]
    fn the_block_records_what_it_took_off_the_charge() {
        let catalog = FakePilotCatalog {
            ships: vec![(
                ShipId(128),
                Ok(ShipFields {
                    ionize_max: 100,
                    deionize: 100,
                    ..FAST
                }),
            )],
            ..catalog()
        };
        let mut session = flying(&catalog, 3.0);
        assert_eq!(session.ion_rate(), 1.0);
        session.tick(Controls::default());
        assert_eq!((session.ion().charge, session.ion().shed), (2.0, 1.0));
        // A charge at or below none is cleared: the whole of it is shed.
        let mut cleared = flying(&catalog, -0.5);
        cleared.tick(Controls::default());
        assert_eq!((cleared.ion().charge, cleared.ion().shed), (0.0, -0.5));
    }

    #[test]
    fn the_colour_outlasts_the_charge() {
        let mut session = flying(&ionizing(), 0.25);
        session.ion.colour = 0x0000_80ff;
        session.tick(Controls::default());
        session.tick(Controls::default());
        assert_eq!(session.ion().colour, 0x0000_80ff, "the display clears it");
    }

    #[test]
    fn a_destroyed_player_and_its_pod_shed_nothing() {
        let mut destroyed = flying(&ionizing(), 10.0);
        destroyed.condition = Condition::Destroyed;
        destroyed.tick(Controls::default());
        assert_eq!(destroyed.ion().charge, 10.0);
        let mut pod = flying(&ionizing(), 10.0);
        pod.pod = Some(100);
        pod.tick(Controls::default());
        assert_eq!(pod.pod, Some(99), "the pod flew");
        assert_eq!(pod.ion().charge, 10.0);
    }

    #[test]
    fn a_disabled_or_dying_player_sheds_its_rate() {
        let dying = Condition::Dying {
            ticks_left: 100,
            starting: false,
        };
        for condition in [Condition::Disabled, dying] {
            let mut session = flying(&ionizing(), 10.0);
            session.condition = condition;
            session.tick(Controls::default());
            assert_eq!(session.ion().charge, 9.5, "{condition:?}");
        }
    }

    #[test]
    fn the_player_sheds_through_the_pre_jump_and_the_warp() {
        let mut session = super::super::tests::bound_for_131(
            &ionizing(),
            super::super::tests::OUT,
            Vec2::ZERO,
            0.0,
        );
        session.tick(Controls::default());
        assert_eq!(session.begin_jump(), Ok(SystemId(131)));
        session.ion.charge = 100.0;
        for tick in 1..=20 {
            session.tick(Controls::default());
            assert!(session.jump.is_some(), "{tick}");
            assert_eq!(session.ion().charge, 100.0 - 0.5 * tick as f32, "{tick}");
        }
    }

    #[test]
    fn double_times_second_pass_sheds_again() {
        let mut session = flying(&ionizing(), 10.0);
        session.tick(Controls::default());
        session.tick_again(
            Controls::default(),
            crate::combat::Rules::default(),
            &mut crate::chance::NeverFires,
        );
        assert_eq!(session.ion().charge, 9.0);
    }

    // The turn and thrust.

    #[test]
    fn an_ionized_player_thrusts_slower_by_the_slowdown() {
        let mut plain = flying(&ionizing(), 0.0);
        plain.tick(thrust());
        let full = plain.player.velocity;
        // A factor of 0.25 after the tick's thrust: 100.5 of 400 as it
        // thrusts.
        let mut session = flying(&ionizing(), 100.5);
        assert_eq!(session.ion_factor(), 100.5 / 400.0);
        session.tick(thrust());
        let expected = (f64::from(full.y.abs()) * (1.0 - f64::from(100.5_f32 / 400.0))) as f32;
        assert!(
            (session.player.velocity.y.abs() - expected).abs() < 1e-6,
            "{:?} {expected}",
            session.player.velocity
        );
    }

    #[test]
    fn an_ionized_player_turns_slower_and_at_most_by_0_7() {
        let session = flying(&ionizing(), 200.0);
        let own = session.flight_rules().handling(session.handling());
        let flying = session.flying_handling();
        assert_eq!(flying.turn_rate, own.turn_rate * 0.5);
        assert_eq!(flying.accel, (f64::from(own.accel) * 0.5) as f32);
        assert_eq!(flying.max_speed, session.top_speed(), "the pull holds it");
        let swamped = flying_session(4000.0).flying_handling();
        assert_eq!(
            swamped.turn_rate,
            (f64::from(own.turn_rate) * (1.0 - f64::from(0.7_f32))) as f32
        );
    }

    fn flying_session(charge: f32) -> Session {
        flying(&ionizing(), charge)
    }

    #[test]
    fn the_turn_keys_turn_an_ionized_player_slower() {
        let turn = Controls {
            turn: Turn::Right,
            ..Controls::default()
        };
        let mut plain = flying(&ionizing(), 0.0);
        plain.tick(turn);
        let mut session = flying(&ionizing(), 200.5);
        session.tick(turn);
        assert!(
            session.player.heading - 90.0 < plain.player.heading - 90.0,
            "{} {}",
            session.player.heading,
            plain.player.heading
        );
    }

    #[test]
    fn from_j_on_the_player_turns_at_its_full_rate_but_thrusts_slower() {
        let mut session = super::super::tests::bound_for_131(
            &ionizing(),
            super::super::tests::OUT,
            Vec2::ZERO,
            0.0,
        );
        session.tick(Controls::default());
        session.ion.charge = 200.0;
        let out = session.flying_handling();
        assert_eq!(session.begin_jump(), Ok(SystemId(131)));
        let jumping = session.flying_handling();
        assert_eq!(jumping.turn_rate, out.turn_rate * 2.0);
        assert_eq!(jumping.accel, out.accel);
    }

    #[test]
    fn the_npcs_see_the_players_slowed_turn_and_thrust_at_its_own_top_speed() {
        let session = flying(&ionizing(), 200.0);
        let side = session.player_side().handling;
        let own = session.handling();
        assert_eq!(side.turn_rate, own.turn_rate * 0.5);
        assert_eq!(side.accel, (f64::from(own.accel) * 0.5) as f32);
        assert_eq!(side.max_speed, own.max_speed);
    }

    // The pull.

    #[test]
    fn past_the_lowered_top_the_player_is_pulled_back_0_025_a_tick_after_the_move() {
        let mut session = flying(&ionizing(), 200.5);
        let top = session.top_speed();
        session.player.velocity = Vec2::new(top, -top);
        session.tick(Controls::default());
        assert_eq!(session.player.position, Vec2::new(top, -top), "moved first");
        let pulled = |v: f32| (f64::from(v) - 0.025) as f32;
        assert_eq!(
            session.player.velocity,
            Vec2::new(pulled(top), -pulled(top))
        );
        for _ in 0..9 {
            session.tick(Controls::default());
        }
        let x = session.player.velocity.x;
        assert!((x - (top - 0.25)).abs() < 1e-5, "{x}");
    }

    #[test]
    fn within_the_lowered_top_the_player_is_not_pulled() {
        let mut session = flying(&ionizing(), 200.5);
        let slow = Vec2::new(session.top_speed() * 0.25, 0.0);
        session.player.velocity = slow;
        session.tick(Controls::default());
        assert_eq!(session.player.velocity, slow);
    }

    #[test]
    fn an_uncharged_player_is_not_pulled() {
        let mut session = flying(&ionizing(), 0.0);
        let top = session.top_speed();
        session.player.velocity = Vec2::new(top, 0.0);
        session.tick(Controls::default());
        assert_eq!(session.player.velocity, Vec2::new(top, 0.0));
    }

    #[test]
    fn the_afterburner_pushes_on_the_charge_the_block_left() {
        let catalog = FakePilotCatalog {
            outfits: vec![outfit(197, &[(AFTERBURNER, 9)])],
            defaults: vec![(ShipId(128), vec![(OutfitId(197), 1)])],
            ..ionizing()
        };
        let afterburner = Controls {
            afterburner: true,
            face: Face::Off,
            ..Controls::default()
        };
        // 0.25 sheds to below none before the push: a full push.
        let mut session = flying(&catalog, 0.25);
        let full = session.flight_rules().handling(session.handling()).accel;
        session.tick(afterburner);
        assert!(session.afterburner_on());
        let pushed = session.player.velocity.length();
        assert!((pushed - full * 2.75).abs() < 1e-5, "{pushed} {full}");
    }

    #[test]
    fn each_npc_in_the_system_sheds_its_class_rate_a_tick_as_it_flies() {
        use crate::session::tests::{arming, blaster, with_an_npc_at};
        let catalog = arming(blaster(), blaster(), 0);
        let mut session = with_an_npc_at(&catalog, 750, 850, 180);
        let id = session.npcs()[0].id;
        session.npc_mut(id).expect("there").ion.charge = 10.0;
        session.tick_traffic(
            &catalog,
            &crate::ai::NovaAi::default(),
            &mut crate::chance::NeverFires,
        );
        // `Deionize` 0 sheds 1 a tick.
        assert_eq!(session.npc(id).expect("there").ion.charge, 9.0);
    }

    // The display.

    #[test]
    fn the_players_display_tints_it_and_clears_a_spent_colour() {
        use crate::combat::ion_tint::IonTint;
        let mut session = flying(&ionizing(), 200.5);
        session.ion.colour = 0x0000_80ff;
        session.tick(Controls::default());
        session.tick_combat(
            crate::combat::Rules::default(),
            &mut crate::testkit::Draws::of(&[0]),
        );
        assert_eq!(
            session.player_ion_tint(),
            Some(IonTint {
                level: 14,
                colour: 0x0000_80ff
            })
        );
        session.ion.charge = 0.0;
        session.tick(Controls::default());
        session.tick_combat(
            crate::combat::Rules::default(),
            &mut crate::chance::NeverFires,
        );
        assert_eq!(session.player_ion_tint(), None);
        assert_eq!(session.ion(), crate::combat::ion::Ion::default());
    }

    #[test]
    fn the_murk_colour_rule_reaches_the_view_through_the_session() {
        use crate::rulebook::{RuleKey, RuleSource, Rulebook};
        let session = flying(&ionizing(), 0.0);
        assert_eq!(session.ion_tint_murk_colour(), RuleSource::Engine);
        let bible = session.with_rules(
            Rulebook::default().with_override(RuleKey::IonTintMurkColour, RuleSource::Bible),
        );
        assert_eq!(bible.ion_tint_murk_colour(), RuleSource::Bible);
    }

    // The lockout.

    /// The ticks, from 1, on which the player fires its blaster of
    /// `Seeker` 0x0020 (reload 2), holding fire from a charge of `charge`
    /// shedding 1 a tick, over 8 steps.
    fn fired_ticks(charge: f32) -> Vec<u32> {
        use crate::combat::ion::CANT_FIRE_IONIZED;
        use crate::session::tests::armed;
        let mut catalog = armed();
        catalog.weapons[0].seeker = CANT_FIRE_IONIZED;
        catalog.ships = vec![(
            ShipId(128),
            Ok(ShipFields {
                ionize_max: 100,
                deionize: 100,
                ..FAST
            }),
        )];
        let mut session = Session::start(&catalog).expect("starts");
        session.ion.charge = charge;
        session.hold_fire(true, false);
        let mut fired = Vec::new();
        for tick in 1..=8 {
            let before = session.shots().len();
            session.tick(Controls::default());
            session.tick_combat(
                crate::combat::Rules::default(),
                &mut crate::chance::NeverFires,
            );
            if session.shots().len() > before {
                fired.push(tick);
            }
        }
        fired
    }

    #[test]
    fn an_ionized_players_seeker_0x0020_gun_is_held_until_its_charge_is_shed() {
        assert_eq!(fired_ticks(0.0), [1, 3, 5, 7], "uncharged, every 2");
        // Its hold reads the charge as each tick's block found it: 3, 2
        // and 1 on ticks 1 to 3, each holding it, then 0 on tick 4, which
        // frees it. So it fires on tick 1, before its first hold, is held
        // through tick 4 (the hold's 1.0 counting down on tick 4), and
        // fires again on tick 5, then every 2.
        assert_eq!(fired_ticks(3.0), [1, 5, 7]);
        // Found at 6 to 1 on ticks 1 to 6, 0 on tick 7: fires on tick 8.
        assert_eq!(fired_ticks(6.0), [1, 8]);
    }

    /// The new shots `firer` has in flight among `seen`'s, which it adds
    /// them to.
    fn new_shots(session: &Session, firer: ShipRef, seen: &mut Vec<ShotId>) -> usize {
        let fresh: Vec<ShotId> = session
            .shots()
            .iter()
            .filter(|shot| shot.firer == firer && !seen.contains(&shot.id))
            .map(|shot| shot.id)
            .collect();
        seen.extend(&fresh);
        fresh.len()
    }

    /// A `Seeker` 0x0020 blaster firing every tick.
    fn held_gun() -> crate::catalog::WeaponRecord {
        crate::catalog::WeaponRecord {
            reload: 0,
            seeker: crate::combat::ion::CANT_FIRE_IONIZED,
            ..crate::session::tests::blaster()
        }
    }

    /// An ionizing blaster: a factor of 1 for a ship of `IonizeMax` 100.
    fn ion_bolt() -> crate::catalog::WeaponRecord {
        crate::catalog::WeaponRecord {
            ionization: 100,
            ..crate::session::tests::blaster()
        }
    }

    /// `armed()` with the player's class and NPC 0's of `IonizeMax` 100.
    fn of_ionize_max_100() -> FakePilotCatalog {
        use crate::session::tests::armed;
        use crate::testkit::ship;
        let fields = ShipFields {
            ionize_max: 100,
            ..FAST
        };
        FakePilotCatalog {
            ships: vec![(ShipId(128), Ok(fields))],
            ship_records: vec![ship(129, fields)],
            ..armed()
        }
    }

    /// The fights, from 1, on which `gunner` fires, and the first fight
    /// after which `charged` holds, over 12 fights of `session`.
    fn fired_around_the_hit(
        session: &mut Session,
        gunner: ShipRef,
        charged: impl Fn(&Session) -> bool,
    ) -> (Vec<u32>, u32) {
        let mut seen = Vec::new();
        let mut fired = Vec::new();
        let mut hit = None;
        for fight in 1..=12 {
            session.tick_combat(
                crate::combat::Rules::default(),
                &mut crate::chance::NeverFires,
            );
            if new_shots(session, gunner, &mut seen) > 0 {
                fired.push(fight);
            }
            if hit.is_none() && charged(session) {
                hit = Some(fight);
            }
        }
        (fired, hit.expect("dosed"))
    }

    #[test]
    fn a_hit_holds_the_players_seeker_0x0020_gun_from_the_second_tick_after_it() {
        use crate::session::tests::knocking;
        let catalog = of_ionize_max_100();
        let mut session = knocking(
            &catalog,
            (Some(held_gun()), 100.0),
            (Some(ion_bolt()), 100.0),
        );
        assert_eq!(session.ion_capacity(), 100.0);
        let (fired, hit) = fired_around_the_hit(&mut session, ShipRef::Player, |session| {
            session.ion().charge > 0.0
        });
        let hit_and_after: Vec<u32> = (1..=hit + 1).collect();
        assert_eq!(
            fired, hit_and_after,
            "fires on the hit's fight and the next"
        );
    }

    #[test]
    fn a_hit_holds_an_npcs_seeker_0x0020_gun_on_the_tick_it_lands() {
        use crate::session::tests::knocking;
        let catalog = of_ionize_max_100();
        let mut session = knocking(
            &catalog,
            (Some(ion_bolt()), 100.0),
            (Some(held_gun()), 100.0),
        );
        let npc = ShipRef::Npc(session.npcs()[0].id);
        let (fired, hit) = fired_around_the_hit(&mut session, npc, |session| {
            session.npcs()[0].ion_factor() >= 1.0
        });
        let before_the_hit: Vec<u32> = (1..hit).collect();
        assert_eq!(fired, before_the_hit, "held from the hit's own fight");
    }

    #[test]
    fn double_times_second_pass_holds_on_the_charge_as_its_own_block_found_it() {
        use crate::session::tests::armed;
        let mut catalog = armed();
        catalog.weapons[0] = held_gun();
        catalog.ships = vec![(
            ShipId(128),
            Ok(ShipFields {
                ionize_max: 100,
                deionize: 100,
                ..FAST
            }),
        )];
        let mut session = Session::start(&catalog).expect("starts");
        session.ion.charge = 1.5;
        session.hold_fire(true, false);
        session.tick(Controls::default());
        session.tick_combat(
            crate::combat::Rules::default(),
            &mut crate::chance::NeverFires,
        );
        // The second pass's block finds 0.5 and sheds it to -0.5: the
        // hold reads the 0.5, so the gun is held.
        session.tick_again(
            Controls::default(),
            crate::combat::Rules::default(),
            &mut crate::chance::NeverFires,
        );
        assert_eq!(session.ion().charge, -0.5);
        let reload = session.armament.mounts()[0].reload;
        assert_eq!(reload, crate::combat::ion::HELD_RELOAD);
    }

    /// The fights, from 1, on which NPC 0, of class `IonizeMax` 100 and
    /// charged `charge`, fires its blaster of `Seeker` 0x0020 (reload 2)
    /// at the player 100 above it, over 3 fights; and its reload as they
    /// end.
    fn npc_fired_ticks(charge: f32) -> (Vec<u32>, f32) {
        use crate::combat::ion::CANT_FIRE_IONIZED;
        use crate::session::tests::{armed, blaster, knocking};
        use crate::testkit::ship;
        let mut catalog = armed();
        catalog.ship_records = vec![ship(
            129,
            ShipFields {
                ionize_max: 100,
                ..FAST
            },
        )];
        let gun = crate::catalog::WeaponRecord {
            seeker: CANT_FIRE_IONIZED,
            ..blaster()
        };
        let mut session = knocking(&catalog, (None, 100.0), (Some(gun), 100.0));
        let npc = &mut session.traffic.npcs_mut()[0];
        assert_eq!(npc.stats.ion.capacity(Flyer::Npc), 100.0, "its class's");
        npc.ion.charge = charge;
        let mut fired = Vec::new();
        for tick in 1..=3 {
            let before = session.shots().len();
            session.tick_combat(
                crate::combat::Rules::default(),
                &mut crate::chance::NeverFires,
            );
            if session.shots().len() > before {
                fired.push(tick);
            }
        }
        let reload = session.npcs()[0].armament.mounts()[0].reload;
        (fired, reload)
    }

    #[test]
    fn an_npcs_seeker_0x0020_gun_is_held_from_a_factor_of_1_of_its_class_ionize_max() {
        // Its hold comes before its fire, so even its first shot is held,
        // and the reload step counts the hold's 1.0 down.
        assert_eq!(npc_fired_ticks(100.0), (vec![], 0.0), "factor 1: held");
        assert_eq!(npc_fired_ticks(250.0), (vec![], 0.0), "factor 2.5: held");
        assert_eq!(npc_fired_ticks(99.0), (vec![1, 3], 1.0), "0.99: fires");
        assert_eq!(npc_fired_ticks(0.0), (vec![1, 3], 1.0), "uncharged");
    }

    // The dissipator.

    #[test]
    fn an_ion_dissipator_raises_the_capacity_and_the_rate() {
        let dissipator: &[(i16, i16)] = &[(ION_ABSORBER, 200), (ION_DISSIPATER, 50)];
        let mut session = flying(&carrying(&[dissipator], 2), 300.0);
        assert_eq!(session.ion_capacity(), 800.0);
        assert_eq!(session.ion_rate(), 1.5);
        assert_eq!(session.ion_factor(), 300.0 / 800.0);
        session.tick(Controls::default());
        assert_eq!(session.ion().charge, 298.5);
        let plain = flying(&ionizing(), 300.0);
        assert!(plain.ion_factor() > session.ion_factor());
    }
}
