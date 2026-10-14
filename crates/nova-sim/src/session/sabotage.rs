//! The player's sabotage outfits (see [`sabotage`](crate::sabotage)).
//!
//! **The refresh.** The session's sabotage clock is refreshed with the
//! blessing table (`Session::refresh_blessing`, the stand-in for
//! `_ResetPlayerPrecalcedValues`; see the `ranks` module's **Blessing**),
//! so every refresh point re-derives the cached flag.
//!
//! **The tick** runs at the head of [`Session::tick_combat`], after its
//! return while landed or jumping and before the eject check, skipped
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
//! the ordinary breakup, and a pre-jump is given up. The jump's
//! hyperspace stage can't be cancelled, as the engine never ticks the
//! clock through it (see the `sabotage` module's left-out list). It then
//! halts the warp-up sounds ([`WARP_UP_HALTS`], @0x695ea, @0x6960d), and
//! the comm line names the first owned kill outfit with a name for its
//! count ([`kill_notice`]), or none, for 360 ticks
//! ([`SimMessage::Sabotaged`], @0x6969d). Nothing is removed and nothing
//! refreshes, so the flag stays set; the player dies, and a dying player
//! counts nothing, so the outfit goes off once.
//!
//! *Ordering against the original.* The block runs after
//! [`Session::tick`]'s steering in the same step, where the original runs
//! it before the steering: a difference inside a frame, which crosses no
//! boundary the player can see.

use super::Session;
use crate::chance::Chance;
use crate::combat::Rules;
use crate::combat::hull::Condition;
use crate::message::SimMessage;
use crate::sabotage::{Sabotage, kill_notice};
use crate::sound::{SimSound, WARP_UP_HALTS};

impl Session {
    /// The `dësc` IDs the player sees as it dies, from the outfits it
    /// owns, gated on the flag the last refresh cached
    /// ([`death_descs`](crate::sabotage::death_descs)).
    #[must_use]
    pub fn sabotage_death_descs(&self) -> Vec<i16> {
        crate::sabotage::death_descs(self.sabotage.flag(), &self.pilot.outfits, &self.outfits)
    }

    /// The sabotage clock's part of the refresh (see the module docs).
    pub(super) fn refresh_sabotage(&mut self) {
        self.sabotage.refresh(&self.pilot.outfits, &self.outfits);
    }

    /// A tick of the sabotage clock, and what goes off with it, drawing on
    /// `chance` (see the module docs).
    pub(super) fn sabotage_tick(&mut self, _rules: Rules, chance: &mut dyn Chance) {
        if self.pod.is_some() {
            return;
        }
        let dying = matches!(
            self.condition,
            Condition::Dying { .. } | Condition::Destroyed
        );
        let Some(flag) = self.sabotage.tick(dying, chance) else {
            return;
        };
        self.sounds.push(SimSound::OnboardExplosion);
        match flag {
            Sabotage::Kill => self.sabotage_kill(),
            Sabotage::Bomb | Sabotage::None => {}
        }
    }

    /// A kill outfit goes off (`_HandlePlayer` @0x69384-0x696a2; see the
    /// module docs' **A kill outfit**).
    fn sabotage_kill(&mut self) {
        let reserves = &mut self.pilot.reserves;
        reserves.armor.now = -1.0;
        reserves.shield.now = -1.0;
        if matches!(self.jump, Some(super::Jump::PreJump { .. })) {
            self.jump = None;
        }
        let outfit = kill_notice(&self.pilot.outfits, &self.outfits);
        self.sounds.push(SimSound::Halt(&WARP_UP_HALTS));
        self.messages.push(SimMessage::Sabotaged { outfit });
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
pub(super) mod tests {
    use super::*;
    use crate::catalog::OutfitId;
    use crate::catalog::{SoundId, SystemId};
    use crate::combat::{CombatEvent, ShipRef};
    use crate::pilot::Pilot;
    use crate::sabotage::FUSE;
    use crate::sabotage::{SABOTAGE_BOMB, SABOTAGE_KILL};
    use crate::session::tests::{armed, facing_an_npc};
    use crate::testkit::{Draws, FakePilotCatalog, fly_out, land_now, outfit};

    /// A kill outfit and a bomb outfit.
    pub(in crate::session) const KILL: i16 = 261;
    pub(in crate::session) const BOMB: i16 = 262;

    /// `armed`'s catalog with [`KILL`] and [`BOMB`].
    pub(in crate::session) fn sabotaged() -> FakePilotCatalog {
        let mut catalog = armed();
        catalog.outfits.extend([
            outfit(KILL, &[(SABOTAGE_KILL, 4100)]),
            outfit(BOMB, &[(SABOTAGE_BOMB, 0)]),
        ]);
        catalog
    }

    /// `catalog`'s new pilot owning `outfits`, each `(id, count)`, flown.
    pub(in crate::session) fn flown(catalog: &FakePilotCatalog, outfits: &[(i16, u16)]) -> Session {
        let mut pilot = Pilot::new(catalog, "Ada").expect("starts");
        pilot
            .outfits
            .extend(outfits.iter().map(|&(id, count)| (OutfitId(id), count)));
        Session::fly(catalog, pilot).expect("flies")
    }

    /// A fight tick drawing `draws`: its sounds.
    pub(in crate::session) fn fight(session: &mut Session, draws: &[u32]) -> Vec<SimSound> {
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
        session.refresh_blessing();
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
        session.refresh_blessing();
        session.pilot.outfits.remove(&OutfitId(KILL));
        session.pilot.outfits.insert(OutfitId(KILL), 0);
        assert_eq!(session.sabotage_death_descs(), Vec::<i16>::new());
    }

    #[test]
    fn a_flown_pilot_with_a_kill_outfit_counts_from_its_seed() {
        let catalog = sabotaged();
        let mut session = flown(&catalog, &[(KILL, 1)]);
        assert_eq!(session.sabotage.flag(), Sabotage::Kill, "@0x19ffb");
        let mut draws = Draws::of(&[37]);
        session.tick_combat(Rules::default(), &mut draws);
        assert_eq!(draws.asked.first(), Some(&100));
        assert_eq!(session.sabotage.counter(), 38.0);
        fight(&mut session, &[]);
        assert_eq!(session.sabotage.counter(), 39.0);
    }

    #[test]
    fn an_outfit_added_without_a_refresh_is_not_counted_until_the_next_refresh() {
        let catalog = sabotaged();
        let mut session = flown(&catalog, &[]);
        session.pilot.outfits.insert(OutfitId(BOMB), 1);
        fight(&mut session, &[]);
        assert_eq!(session.sabotage.flag(), Sabotage::None);
        assert_eq!(session.sabotage.counter(), 0.0);
        session.refresh_blessing();
        fight(&mut session, &[5]);
        assert_eq!(session.sabotage.flag(), Sabotage::Bomb);
        assert_eq!(session.sabotage.counter(), 6.0);
    }

    #[test]
    fn a_jettison_reseeds_a_running_counter() {
        let catalog = sabotaged();
        let mut session = flown(&catalog, &[(KILL, 1)]);
        fight(&mut session, &[50]);
        fight(&mut session, &[]);
        assert_eq!(session.sabotage.counter(), 52.0);
        session.jettison_cargo(false, &mut Draws::of(&[]));
        assert_eq!(session.sabotage.counter(), 0.0, "@0x3e3b5");
        fight(&mut session, &[10]);
        assert_eq!(session.sabotage.counter(), 11.0);
    }

    #[test]
    fn the_idle_settling_after_a_take_off_reseeds_again() {
        let catalog = sabotaged();
        let mut session = flown(&catalog, &[(KILL, 1)]);
        land_now(&mut session).expect("lands");
        session
            .take_off(&catalog, &mut Draws::of(&[]))
            .expect("takes off");
        fight(&mut session, &[10]);
        fight(&mut session, &[]);
        assert_eq!(session.sabotage.counter(), 12.0);
        let _ = session.settle_script(&catalog, &mut Draws::of(&[]));
        assert_eq!(session.sabotage.counter(), 0.0, "refreshed again");
        fight(&mut session, &[20]);
        assert_eq!(session.sabotage.counter(), 21.0);
        let _ = session.settle_script(&catalog, &mut Draws::of(&[]));
        assert_eq!(session.sabotage.counter(), 21.0, "only the first settling");
    }

    #[test]
    fn the_counter_stands_still_in_the_escape_pod() {
        let catalog = sabotaged();
        let mut session = flown(&catalog, &[(KILL, 1)]);
        fight(&mut session, &[50]);
        session.pod = Some(100);
        let sounds = fight(&mut session, &[]);
        assert_eq!(session.sabotage.counter(), 51.0);
        session.sabotage.set_counter(300.0);
        let sounds = [sounds, fight(&mut session, &[])].concat();
        assert_eq!(session.sabotage.counter(), 300.0);
        assert!(!sounds.contains(&SimSound::OnboardExplosion), "{sounds:?}");
    }

    #[test]
    fn a_disabled_players_counter_counts_and_a_dying_ones_does_not() {
        let catalog = sabotaged();
        let mut session = flown(&catalog, &[(KILL, 1)]);
        fight(&mut session, &[50]);
        session.condition = Condition::Disabled;
        fight(&mut session, &[]);
        assert_eq!(session.sabotage.counter(), 52.0, "disabled counts");
        for condition in [
            Condition::Dying {
                ticks_left: 50,
                starting: false,
            },
            Condition::Destroyed,
        ] {
            session.condition = condition;
            fight(&mut session, &[]);
            assert_eq!(session.sabotage.counter(), 52.0, "{condition:?}");
        }
    }

    #[test]
    fn the_clock_raises_snd_300_on_the_first_tick_its_counter_passes_300() {
        for id in [KILL, BOMB] {
            let catalog = sabotaged();
            let mut session = flown(&catalog, &[(id, 1)]);
            fight(&mut session, &[0]);
            session.sabotage.set_counter(299.0);
            let sounds = fight(&mut session, &[]);
            assert!(!sounds.contains(&SimSound::OnboardExplosion), "{sounds:?}");
            let sounds = fight(&mut session, &[]);
            assert_eq!(sounds.first(), Some(&SimSound::OnboardExplosion), "{id}");
        }
    }

    /// `catalog`'s pilot owning `outfits`, flown, its clock seeded and
    /// at the fuse: the next fight tick sets it off.
    pub(in crate::session) fn lit(catalog: &FakePilotCatalog, outfits: &[(i16, u16)]) -> Session {
        let mut session = flown(catalog, outfits);
        fight(&mut session, &[0]);
        session.sabotage.set_counter(FUSE);
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
            matches!(session.condition, Condition::Dying { starting: true, .. }),
            "{:?}",
            session.condition
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
            session.jump,
            Some(super::super::Jump::PreJump { .. })
        ));
        fight(&mut session, &[]);
        assert_eq!(session.jump, None);
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
    fn a_kill_outfit_stays_owned_and_the_flag_stays_set() {
        let catalog = sabotaged();
        let mut session = lit(&catalog, &[(KILL, 1)]);
        fight(&mut session, &[]);
        assert_eq!(session.pilot.outfits.get(&OutfitId(KILL)), Some(&1));
        assert_eq!(session.sabotage.flag(), Sabotage::Kill);
        assert_eq!(session.sabotage.counter(), 301.0);
    }

    #[test]
    fn a_kill_outfit_fires_once() {
        let catalog = sabotaged();
        let mut session = lit(&catalog, &[(KILL, 1)]);
        fight(&mut session, &[]);
        let mut ticks = 0;
        while session.condition != Condition::Destroyed {
            let sounds = fight(&mut session, &[]);
            assert!(!sounds.contains(&SimSound::OnboardExplosion), "{sounds:?}");
            ticks += 1;
            assert!(ticks < 1000, "never destroyed");
        }
        let sounds = fight(&mut session, &[]);
        assert!(!sounds.contains(&SimSound::OnboardExplosion), "{sounds:?}");
    }
}
