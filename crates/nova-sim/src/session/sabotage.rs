//! The player's sabotage outfits (see [`sabotage`](crate::sabotage)).
//!
//! **The refresh.** The session's sabotage clock is refreshed by
//! `Session::reset_precalced`, the stand-in for
//! `_ResetPlayerPrecalcedValues` (see the `precalced` module for every
//! site), so every reset re-derives the cached flag.
//!
//! **The tick** runs in `_HandlePlayer`'s place ([`Session::game_work`],
//! [`Session::tick_again`]): after the player's point defence and before
//! its trigger, as the block (0x692e5) sits between them (@0x68c74,
//! @0x6ab55); so a kill that gives up a pre-jump comes after the point
//! defence rested through it. [`Session::tick_combat`] runs it at its
//! head, after its return while landed and before the eject check,
//! through a jump too. It is skipped
//! while the player is in the escape pod: `_HandlePlayer` reaches the
//! block (0x692e5) only once the pod's countdown is out (0x68d1d-0x68d25),
//! and runs it before its death-timer decrement (@0x6d7e6), the klaxon
//! (@0x6f1e4) and the eject (@0x6f22c), with `_HandleShipDisplay` after
//! them all. A player dying or destroyed as the last fight left it counts
//! nothing; a disabled one counts. When the clock goes off,
//! [`SimSound::OnboardExplosion`] sounds (@0x69375).
//!
//! **A kill outfit** ([`Sabotage::Kill`], @0x69384-0x696a2) sets the
//! player's armour, shield and jump timer to -1.0 (@0x693a0-0x693a6): the
//! fight that follows on the same tick finds the armour gone and starts
//! the ordinary breakup, and the jump is given up, in its pre-jump or
//! its warp (+0x50 = −1.0, @0x693a6). It then
//! halts the warp-up sounds ([`WARP_UP_HALTS`], @0x695ea, @0x6960d), and
//! the comm line names the first owned kill outfit with a name for its
//! count ([`kill_notice`]), or none, for 360 ticks
//! ([`SimMessage::Sabotaged`], @0x6969d); with none named and the data's
//! `l33t` mode on, it says the original's own words instead
//! ([`SimMessage::BombSetUp`], @0x69623-0x6963e). Nothing is removed and nothing
//! refreshes, so the flag stays set; the player dies, and a dying player
//! counts nothing, so the outfit goes off once.
//!
//! **Bomb outfits** ([`Sabotage::Bomb`], @0x696a7-0x69a0a). For each
//! owned bomb outfit in index order ([`bombs`]), the comm line names it
//! for 400 ticks ([`SimMessage::BombExploded`], @0x698b0, with no test of
//! its names); its `bööm`, when its first bomb slot names one, goes off
//! at the player's position, of size 0, with its own sound
//! (`_CreateExplosion` @0x698ef; the view plays a `bööm`'s sound for any
//! [`CombatEvent::Exploded`](crate::combat::CombatEvent::Exploded)); every
//! copy is removed (@0x698f7); and the player takes [`bomb_damage`] to
//! its armour alone, a hit that can neither kill nor disable
//! ([`take`](crate::combat::damage::take): `_DamageShip`
//! @0x699b2 with +0x28, +0x2c and +0x30 set and no source), the player
//! counted disabled before it as its armour then says (`_IsDisabled`
//! @0x3a9ad), so a second bomb sees the first's damage. The damage is at
//! least the armour capacity, so the bomb never kills: an intact player
//! is left at what a self-repair leaves, a disabled one at 1. Then
//! `_ResetPlayerPrecalcedValues` runs (@0x69a05), the sabotage clock's
//! refresh with it:
//! no bomb is left, so the flag clears and the next tick zeroes the
//! count, and the bombs go off once. No ship stats are rebuilt, as
//! `_ResetPlayerPrecalcedValues` rebuilds none: the stats and armament
//! follow the outfits at the next refit. No save is due, as the original
//! saves nothing here, unless that reset's trim cuts the cargo (see the
//! `precalced` module).
//!
//! *Ordering against the original.* [`Session::game_work`] runs the block
//! before the player's steering, as the original does; the separate parts
//! run it after [`Session::tick`]'s steering in the same step.

use super::{PlayerShip, Session, Statics};
use crate::chance::Chance;
use crate::combat::Rules;
use crate::combat::damage::{Hit, take};
use crate::combat::hull::{Condition, DisableRule};
use crate::message::SimMessage;
use crate::pilot::Pilot;
use crate::sabotage::{Sabotage, bomb_damage, bombs, kill_notice};
use crate::sound::{SimSound, WARP_UP_HALTS};

impl Session {
    /// The `dësc` IDs the player sees as it dies, from the outfits it
    /// owns, gated on the flag the last refresh cached
    /// ([`death_descs`](crate::sabotage::death_descs)).
    #[must_use]
    pub fn sabotage_death_descs(&self) -> Vec<i16> {
        self.ship.sabotage_death_descs(&self.pilot, &self.statics)
    }

    /// A tick of the sabotage clock, and what goes off with it, drawing on
    /// `chance` (see the module docs).
    pub(super) fn sabotage_tick(&mut self, rules: Rules, chance: &mut dyn Chance) {
        if self.pod.is_some() {
            return;
        }
        let dying = matches!(
            self.ship.condition,
            Condition::Dying { .. } | Condition::Destroyed
        );
        let Some(flag) = self.ship.latches.sabotage.tick(dying, chance) else {
            return;
        };
        self.out.sounds.push(SimSound::OnboardExplosion);
        match flag {
            Sabotage::Kill => self.sabotage_kill(),
            Sabotage::Bomb => self.sabotage_bombs(rules.disable, chance),
            Sabotage::None => {}
        }
    }

    /// The bomb outfits go off (`_HandlePlayer` @0x696a7-0x69a0a; see the
    /// module docs' **Bomb outfits**), `rule` saying when the player is
    /// disabled, each bomb's damage drawn on `chance`.
    fn sabotage_bombs(&mut self, rule: &dyn DisableRule, chance: &mut dyn Chance) {
        for bomb in bombs(&self.pilot.outfits, &self.statics.outfits) {
            self.space.say(
                &mut self.out,
                SimMessage::BombExploded {
                    outfit: bomb.outfit,
                    one: bomb.one,
                },
            );
            if let Some(explosion) = bomb.explosion {
                self.space
                    .combat
                    .explode(self.ship.player.position, explosion);
            }
            self.pilot.outfits.remove(&bomb.outfit);
            let armor = self.pilot.reserves.armor;
            let was_disabled = rule.disabled(armor, &self.ship.fit.hull);
            let hit = Hit {
                mass: bomb_damage(armor.max, chance),
                energy: 0.0,
                passes_shields: true,
                cant_kill: true,
                cant_disable: true,
            };
            take(
                &mut self.pilot.reserves,
                hit,
                &self.ship.fit.hull,
                rule,
                was_disabled,
            );
        }
        self.reset_precalced();
    }

    /// A kill outfit goes off (`_HandlePlayer` @0x69384-0x696a2; see the
    /// module docs' **A kill outfit**).
    pub(crate) fn sabotage_kill(&mut self) {
        let reserves = &mut self.pilot.reserves;
        reserves.armor.now = -1.0;
        reserves.shield.now = -1.0;
        self.mode.end_jump();
        let outfit = kill_notice(&self.pilot.outfits, &self.statics.outfits);
        self.ship.halt_sounds(&mut self.out, &WARP_UP_HALTS);
        // With none named, the `l33t` mode says its own (@0x69623-0x6963e).
        self.space.say(
            &mut self.out,
            if outfit.is_none() && self.setup.leet {
                SimMessage::BombSetUp
            } else {
                SimMessage::Sabotaged { outfit }
            },
        );
    }
}

impl PlayerShip {
    /// The `dësc` IDs the player sees as it dies, from the outfits it
    /// owns, gated on the flag the last refresh cached
    /// ([`death_descs`](crate::sabotage::death_descs)).
    #[must_use]
    fn sabotage_death_descs(&self, pilot: &Pilot, statics: &Statics) -> Vec<i16> {
        crate::sabotage::death_descs(
            self.latches.sabotage.flag(),
            &pilot.outfits,
            &statics.outfits,
        )
    }

    /// The sabotage clock's part of the refresh (see the module docs).
    pub(super) fn refresh_sabotage(&mut self, pilot: &Pilot, statics: &Statics) {
        self.latches
            .sabotage
            .refresh(&pilot.outfits, &statics.outfits);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::OutfitId;
    use crate::catalog::{BoomId, ShipId, SoundId, SystemId};
    use crate::combat::hull::{HullSpec, TOUGH};
    use crate::combat::weapon::Explosion;
    use crate::combat::{CombatEvent, ShipRef};
    use crate::geometry::Vec2;
    use crate::pilot::Pilot;
    use crate::reserves::Gauge;
    use crate::sabotage::FUSE;
    use crate::sabotage::{SABOTAGE_BOMB, SABOTAGE_KILL};
    use crate::session::tests::{armed, facing_an_npc};
    use crate::testkit::{Draws, FakePilotCatalog, fly_out, land_now, outfit};

    /// A kill outfit and a bomb outfit.
    const KILL: i16 = 261;
    const BOMB: i16 = crate::testkit::BOMB;

    /// `armed`'s catalog with [`KILL`] and [`BOMB`].
    fn sabotaged() -> FakePilotCatalog {
        let mut catalog = armed();
        catalog.outfits.extend([
            outfit(KILL, &[(SABOTAGE_KILL, 4100)]),
            crate::testkit::bomb_outfit(),
        ]);
        catalog
    }

    /// `catalog`'s new pilot owning `outfits`, each `(id, count)`, flown.
    fn flown(catalog: &FakePilotCatalog, outfits: &[(i16, u16)]) -> Session {
        let mut pilot = Pilot::new(catalog, "Ada").expect("starts");
        pilot
            .outfits
            .extend(outfits.iter().map(|&(id, count)| (OutfitId(id), count)));
        Session::fly(catalog, pilot).expect("flies")
    }

    /// A fight tick drawing `draws`: its sounds.
    fn fight(session: &mut Session, draws: &[u32]) -> Vec<SimSound> {
        session.take_sounds();
        session.tick_combat(Rules::default(), &mut Draws::of(draws));
        session.take_sounds()
    }

    #[test]
    fn a_sessions_sabotage_death_descs_come_from_the_pilots_outfits() {
        let catalog = sabotaged();
        let mut session = facing_an_npc(&catalog, 180);
        assert_eq!(session.sabotage_death_descs(), Vec::<i16>::new());
        session.pilot.outfits.insert(OutfitId(KILL), 1);
        session.reset_precalced();
        assert_eq!(session.sabotage_death_descs(), [4100]);
    }

    #[test]
    fn a_sessions_death_descs_need_the_cached_flag() {
        let catalog = sabotaged();
        let mut session = facing_an_npc(&catalog, 180);
        session.pilot.outfits.insert(OutfitId(KILL), 1);
        assert_eq!(
            session.sabotage_death_descs(),
            Vec::<i16>::new(),
            "gained since the last refresh (@0x6873e)"
        );
        session.reset_precalced();
        session.pilot.outfits.remove(&OutfitId(KILL));
        session.pilot.outfits.insert(OutfitId(KILL), 0);
        assert_eq!(session.sabotage_death_descs(), Vec::<i16>::new());
    }

    #[test]
    fn a_flown_pilot_with_a_kill_outfit_counts_from_its_seed() {
        let catalog = sabotaged();
        let mut session = flown(&catalog, &[(KILL, 1)]);
        assert_eq!(
            session.ship.latches.sabotage.flag(),
            Sabotage::Kill,
            "@0x19ffb"
        );
        let mut draws = Draws::of(&[37]);
        session.tick_combat(Rules::default(), &mut draws);
        assert_eq!(draws.asked.first(), Some(&100));
        assert_eq!(session.ship.latches.sabotage.counter(), 38.0);
        fight(&mut session, &[]);
        assert_eq!(session.ship.latches.sabotage.counter(), 39.0);
    }

    #[test]
    fn an_outfit_added_without_a_refresh_is_not_counted_until_the_next_refresh() {
        let catalog = sabotaged();
        let mut session = flown(&catalog, &[]);
        session.pilot.outfits.insert(OutfitId(BOMB), 1);
        fight(&mut session, &[]);
        assert_eq!(session.ship.latches.sabotage.flag(), Sabotage::None);
        assert_eq!(session.ship.latches.sabotage.counter(), 0.0);
        session.reset_precalced();
        fight(&mut session, &[5]);
        assert_eq!(session.ship.latches.sabotage.flag(), Sabotage::Bomb);
        assert_eq!(session.ship.latches.sabotage.counter(), 6.0);
    }

    #[test]
    fn a_jettison_reseeds_a_running_counter() {
        let catalog = sabotaged();
        let mut session = flown(&catalog, &[(KILL, 1)]);
        fight(&mut session, &[50]);
        fight(&mut session, &[]);
        assert_eq!(session.ship.latches.sabotage.counter(), 52.0);
        session.jettison_cargo(false, &mut Draws::of(&[]));
        assert_eq!(session.ship.latches.sabotage.counter(), 0.0, "@0x3e3b5");
        fight(&mut session, &[10]);
        assert_eq!(session.ship.latches.sabotage.counter(), 11.0);
    }

    #[test]
    fn the_idle_settling_after_a_take_off_runs_no_reset_of_its_own() {
        let catalog = sabotaged();
        let mut session = flown(&catalog, &[(KILL, 1)]);
        land_now(&mut session).expect("lands");
        session
            .take_off(&catalog, &mut Draws::of(&[]))
            .expect("takes off");
        fight(&mut session, &[10]);
        fight(&mut session, &[]);
        assert_eq!(
            session.ship.latches.sabotage.counter(),
            12.0,
            "the take-off's reset"
        );
        let _ = session.settle_script(&catalog, &mut Draws::of(&[]));
        assert_eq!(
            session.ship.latches.sabotage.counter(),
            12.0,
            "no script ran, so no reset (see the `precalced` module)"
        );
    }

    #[test]
    fn the_counter_stands_still_in_the_escape_pod() {
        let catalog = sabotaged();
        let mut session = flown(&catalog, &[(KILL, 1)]);
        fight(&mut session, &[50]);
        session.pod = Some(100);
        let sounds = fight(&mut session, &[]);
        assert_eq!(session.ship.latches.sabotage.counter(), 51.0);
        session.ship.latches.sabotage.set_counter(300.0);
        let sounds = [sounds, fight(&mut session, &[])].concat();
        assert_eq!(session.ship.latches.sabotage.counter(), 300.0);
        assert!(!sounds.contains(&SimSound::OnboardExplosion), "{sounds:?}");
    }

    #[test]
    fn a_disabled_players_counter_counts_and_a_dying_ones_does_not() {
        let catalog = sabotaged();
        let mut session = flown(&catalog, &[(KILL, 1)]);
        fight(&mut session, &[50]);
        session.ship.condition = Condition::Disabled;
        fight(&mut session, &[]);
        assert_eq!(
            session.ship.latches.sabotage.counter(),
            52.0,
            "disabled counts"
        );
        for condition in [
            Condition::Dying {
                ticks_left: 50,
                starting: false,
            },
            Condition::Destroyed,
        ] {
            session.ship.condition = condition;
            fight(&mut session, &[]);
            assert_eq!(
                session.ship.latches.sabotage.counter(),
                52.0,
                "{condition:?}"
            );
        }
    }

    #[test]
    fn the_clock_raises_snd_300_on_the_first_tick_its_counter_passes_300() {
        for id in [KILL, BOMB] {
            let catalog = sabotaged();
            let mut session = flown(&catalog, &[(id, 1)]);
            fight(&mut session, &[0]);
            session.ship.latches.sabotage.set_counter(299.0);
            let sounds = fight(&mut session, &[]);
            assert!(!sounds.contains(&SimSound::OnboardExplosion), "{sounds:?}");
            let sounds = fight(&mut session, &[]);
            assert_eq!(sounds.first(), Some(&SimSound::OnboardExplosion), "{id}");
        }
    }

    /// `catalog`'s pilot owning `outfits`, flown, its clock seeded and
    /// at the fuse: the next fight tick sets it off.
    fn lit(catalog: &FakePilotCatalog, outfits: &[(i16, u16)]) -> Session {
        let mut session = flown(catalog, outfits);
        fight(&mut session, &[0]);
        session.ship.latches.sabotage.set_counter(FUSE);
        session.take_messages();
        session.take_combat_events();
        session
    }

    #[test]
    fn a_kill_outfit_breaks_the_player_up_on_the_firing_tick() {
        let catalog = sabotaged();
        let mut session = lit(&catalog, &[(KILL, 1)]);
        let sounds = fight(&mut session, &[]);
        assert!(
            matches!(
                session.ship.condition,
                Condition::Dying { starting: true, .. }
            ),
            "{:?}",
            session.ship.condition
        );
        let reserves = session.pilot.reserves;
        assert_eq!((reserves.shield.now, reserves.armor.now), (-1.0, -1.0));
        assert!(session.take_combat_events().iter().any(|event| matches!(
            event,
            CombatEvent::BreakingUp {
                ship: ShipRef::Player,
                ..
            }
        )));
        assert!(!sounds.contains(&SimSound::Klaxon), "{sounds:?}");
    }

    #[test]
    fn a_kill_outfit_cancels_a_pre_jump() {
        let catalog = sabotaged();
        let mut session = lit(&catalog, &[(KILL, 1)]);
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        session.begin_jump().expect("jumps");
        assert!(matches!(
            session.mode.jump(),
            Some(super::super::Jump::PreJump { .. })
        ));
        fight(&mut session, &[]);
        assert_eq!(session.mode.jump(), None);
    }

    #[test]
    fn a_kill_outfit_cancels_a_warp_under_way_as_its_clock_runs_through_it() {
        // At rest facing east, the bearing: the jump begins at once, and
        // the fight, and the clock with it, runs through the warp.
        let catalog = sabotaged();
        let mut session = lit(&catalog, &[(KILL, 1)]);
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        session.ship.player.velocity = crate::geometry::Vec2::ZERO;
        session.ship.player.heading = 90.0;
        session.begin_jump().expect("jumps");
        assert!(session.jumping().is_some(), "its warp is under way");
        fight(&mut session, &[]);
        assert_eq!(session.mode.jump(), None, "+0x50 = -1.0 (@0x693a6)");
    }

    #[test]
    fn a_kill_outfit_raises_snd_300_then_halts_128_and_129_only() {
        let catalog = sabotaged();
        let mut session = lit(&catalog, &[(KILL, 1)]);
        assert_eq!(
            fight(&mut session, &[]),
            [
                SimSound::OnboardExplosion,
                SimSound::Halt(&[SoundId(128), SoundId(129)])
            ]
        );
    }

    #[test]
    fn a_kill_outfit_names_the_first_owned_kill_outfit_for_360_ticks() {
        let mut catalog = sabotaged();
        catalog.outfits.push(outfit(260, &[(SABOTAGE_KILL, 0)]));
        for (count, one) in [(1, true), (2, false)] {
            let mut session = lit(&catalog, &[(KILL, 1), (260, count)]);
            fight(&mut session, &[]);
            let messages = session.take_messages();
            assert_eq!(
                messages,
                [SimMessage::Sabotaged {
                    outfit: Some((OutfitId(260), one))
                }]
            );
            assert_eq!(messages[0].ticks(), 360);
        }
    }

    #[test]
    fn a_kill_outfit_with_no_named_outfit_says_a_bomb_has_exploded() {
        let mut catalog = sabotaged();
        for record in &mut catalog.outfits {
            record.lc_name.clear();
        }
        let mut session = lit(&catalog, &[(KILL, 1)]);
        fight(&mut session, &[]);
        assert_eq!(
            session.take_messages(),
            [SimMessage::Sabotaged { outfit: None }]
        );
    }

    #[test]
    fn with_leet_mode_a_kill_outfit_with_no_named_outfit_says_someone_set_up_the_bomb() {
        let mut catalog = sabotaged();
        for record in &mut catalog.outfits {
            record.lc_name.clear();
        }
        let mut session = lit(&catalog, &[(KILL, 1)]);
        session.setup.leet = true;
        fight(&mut session, &[]);
        let messages = session.take_messages();
        assert_eq!(messages, [SimMessage::BombSetUp]);
        assert_eq!(messages[0].ticks(), 360);
        let catalog = sabotaged();
        let mut session = lit(&catalog, &[(KILL, 1)]);
        session.setup.leet = true;
        fight(&mut session, &[]);
        assert!(
            matches!(
                session.take_messages()[..],
                [SimMessage::Sabotaged { outfit: Some(_) }]
            ),
            "an outfit named is named in l33t mode too"
        );
    }

    #[test]
    fn a_kill_outfit_stays_owned_and_the_flag_stays_set() {
        let catalog = sabotaged();
        let mut session = lit(&catalog, &[(KILL, 1)]);
        fight(&mut session, &[]);
        assert_eq!(session.pilot.outfits.get(&OutfitId(KILL)), Some(&1));
        assert_eq!(session.ship.latches.sabotage.flag(), Sabotage::Kill);
        assert_eq!(session.ship.latches.sabotage.counter(), 301.0);
    }

    #[test]
    fn a_kill_outfit_fires_once() {
        let catalog = sabotaged();
        let mut session = lit(&catalog, &[(KILL, 1)]);
        fight(&mut session, &[]);
        let mut ticks = 0;
        while session.ship.condition != Condition::Destroyed {
            let sounds = fight(&mut session, &[]);
            assert!(!sounds.contains(&SimSound::OnboardExplosion), "{sounds:?}");
            ticks += 1;
            assert!(ticks < 1000, "never destroyed");
        }
        let sounds = fight(&mut session, &[]);
        assert!(!sounds.contains(&SimSound::OnboardExplosion), "{sounds:?}");
    }

    /// [`sabotaged`] with the player's ship of 300 armour, and of 100
    /// shields, tough or not.
    fn armoured(tough: bool) -> FakePilotCatalog {
        let mut catalog = sabotaged();
        for (id, fields) in &mut catalog.ships {
            if *id == ShipId(128)
                && let Ok(fields) = fields
            {
                fields.armor = 300;
                fields.shield = 100;
            }
        }
        for hull in &mut catalog.hulls {
            if hull.id == ShipId(128) && tough {
                hull.flags |= TOUGH;
            }
        }
        catalog
    }

    #[test]
    fn a_bomb_outfit_goes_off_for_400_ticks_and_every_copy_is_gone() {
        let catalog = armoured(false);
        for (count, one) in [(1, true), (3, false)] {
            let mut session = lit(&catalog, &[(BOMB, count)]);
            let sounds = fight(&mut session, &[]);
            assert_eq!(sounds.first(), Some(&SimSound::OnboardExplosion));
            let messages = session.take_messages();
            assert_eq!(
                messages,
                [SimMessage::BombExploded {
                    outfit: OutfitId(BOMB),
                    one
                }]
            );
            assert_eq!(messages[0].ticks(), 400);
            assert_eq!(session.pilot.outfits.get(&OutfitId(BOMB)), None);
        }
    }

    #[test]
    fn a_bomb_explodes_its_boom_at_the_players_position_only_for_mod_val_128_to_191() {
        for (mod_val, boom) in [(127, None), (128, Some(128)), (191, Some(191)), (192, None)] {
            let mut catalog = armoured(false);
            catalog
                .outfits
                .push(outfit(263, &[(SABOTAGE_BOMB, mod_val)]));
            let mut session = lit(&catalog, &[(263, 1)]);
            session.ship.player.position = Vec2::new(40.0, -25.0);
            fight(&mut session, &[]);
            let exploded: Vec<_> = session
                .take_combat_events()
                .into_iter()
                .filter_map(|event| match event {
                    CombatEvent::Exploded {
                        at,
                        explosion,
                        size,
                    } => Some((at, explosion, size)),
                    _ => None,
                })
                .collect();
            // Of no size: `_CreateExplosion(.., 0, 1)` @0x698ef.
            let expected: Vec<_> = boom
                .map(|boom| {
                    (
                        session.ship.player.position,
                        Explosion {
                            boom: BoomId(boom),
                            extra: false,
                        },
                        0.0,
                    )
                })
                .into_iter()
                .collect();
            assert_eq!(exploded, expected, "{mod_val}");
        }
    }

    /// The shield and armour a bomb leaves a player of `catalog` with
    /// `armor` armour before it, disabled or not, the damage drawn
    /// `draw` above the capacity. A disabled player first draws for its
    /// self-repair (see the `player_repair` module), which 1 refuses.
    fn bombed(catalog: &FakePilotCatalog, armor: f32, disabled: bool, draw: u32) -> (f32, f32) {
        let mut session = lit(catalog, &[(BOMB, 1)]);
        session.pilot.reserves.armor.now = armor;
        let mut draws = if disabled {
            session.ship.condition = Condition::Disabled;
            Draws::of(&[1, draw])
        } else {
            Draws::of(&[draw])
        };
        session.tick_combat(Rules::default(), &mut draws);
        assert!(draws.asked.contains(&151), "Rand(300 x 0.5 + 1)");
        let reserves = session.pilot.reserves;
        (reserves.shield.now, reserves.armor.now)
    }

    #[test]
    fn the_bomb_never_kills_a_300_armour_player_ends_at_100_99_with_shields_unchanged() {
        let catalog = armoured(false);
        for draw in [0, 150] {
            let (shield, armor) = bombed(&catalog, 300.0, false, draw);
            assert_eq!(shield, 100.0, "{draw}");
            assert!((armor - 100.99).abs() < 1e-4, "300 x 0.3333 + 1: {armor}");
        }
    }

    #[test]
    fn a_bomb_on_a_tough_hull_leaves_31_armour() {
        assert_eq!(bombed(&armoured(true), 300.0, false, 0), (100.0, 31.0));
    }

    #[test]
    fn a_bomb_on_an_already_disabled_player_leaves_1_armour() {
        assert_eq!(bombed(&armoured(false), 90.0, true, 0), (100.0, 1.0));
    }

    #[test]
    fn two_bomb_outfits_each_post_their_notice_and_hit_in_index_order() {
        let mut catalog = armoured(false);
        catalog.outfits.push(outfit(263, &[(SABOTAGE_BOMB, 128)]));
        let mut session = lit(&catalog, &[(263, 2), (BOMB, 1)]);
        let mut draws = Draws::of(&[0, 0]);
        session.tick_combat(Rules::default(), &mut draws);
        assert_eq!(
            session.take_messages(),
            [
                SimMessage::BombExploded {
                    outfit: OutfitId(BOMB),
                    one: true
                },
                SimMessage::BombExploded {
                    outfit: OutfitId(263),
                    one: false
                },
            ]
        );
        assert_eq!(&draws.asked[..2], [151, 151]);
        // The first leaves 100.99, so the second finds the player intact
        // again and floors it the same way, not at 1.
        let armor = session.pilot.reserves.armor.now;
        assert!((armor - 100.99).abs() < 1e-4, "{armor}");
        assert_eq!(session.pilot.outfits.get(&OutfitId(263)), None);
    }

    #[test]
    fn a_second_bomb_finds_the_player_as_the_first_left_it() {
        /// Calls a ship disabled below 200 armour.
        #[derive(Debug)]
        struct Below200;

        impl DisableRule for Below200 {
            fn disabled(&self, armor: Gauge, _hull: &HullSpec) -> bool {
                armor.now < 200.0
            }
        }

        let mut catalog = armoured(false);
        catalog.outfits.push(outfit(263, &[(SABOTAGE_BOMB, 0)]));
        let mut session = lit(&catalog, &[(BOMB, 1), (263, 1)]);
        let rules = Rules {
            disable: &Below200,
            ..Rules::default()
        };
        session.tick_combat(rules, &mut Draws::of(&[0, 0]));
        // The first leaves 100.99, still disabled by this rule, so the
        // second finds the player disabled and leaves it at 1.
        assert_eq!(session.pilot.reserves.armor.now, 1.0);
    }

    #[test]
    fn a_bomb_fires_once_its_refresh_clears_the_flag_and_the_next_tick_zeroes_the_counter() {
        use crate::catalog::RankId;
        use crate::session::ranks::tests::{blessed, blessing_rank};
        let mut catalog = armoured(false);
        catalog.ranks = vec![blessing_rank()];
        let mut session = lit(&catalog, &[(BOMB, 1)]);
        session.pilot.ranks.insert(RankId(200));
        assert!(!blessed(&session));
        session.ship.latches.scan_latches = crate::scan::Latches {
            junk: false,
            outfits: false,
        };
        let sounds = fight(&mut session, &[]);
        assert_eq!(sounds.first(), Some(&SimSound::OnboardExplosion));
        assert_eq!(
            session.ship.latches.sabotage.flag(),
            Sabotage::None,
            "@0x69a05"
        );
        assert!(
            session.ship.latches.sabotage.counter() > FUSE,
            "left as it was"
        );
        assert!(blessed(&session), "re-taken");
        assert_eq!(
            session.ship.latches.scan_latches,
            crate::scan::Latches::ARMED
        );
        let sounds = fight(&mut session, &[]);
        assert!(!sounds.contains(&SimSound::OnboardExplosion), "{sounds:?}");
        assert_eq!(session.ship.latches.sabotage.counter(), 0.0, "@0x69a0c");
    }

    #[test]
    fn a_bomb_makes_no_save_due() {
        let catalog = armoured(false);
        let mut session = lit(&catalog, &[(BOMB, 1)]);
        session.take_save_due();
        fight(&mut session, &[]);
        assert_eq!(session.pilot.outfits.get(&OutfitId(BOMB)), None, "gone off");
        assert!(!session.take_save_due());
    }
}
