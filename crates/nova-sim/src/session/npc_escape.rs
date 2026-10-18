//! A dying NPC's escape pods and its last fighter, as `_HandleShip`'s
//! dying block runs them (@0x35080-0x3557b), by the rules of
//! [`npc_escape`](crate::npc_escape).
//!
//! **The death timer.** The fight counts an NPC's breakup down
//! ([`Condition::Dying`]'s `ticks_left`). `_HandleShip` takes a tick off
//! the timer before its checks, and only while it is above 0
//! (@0x35083-0x350a3), so the pod checks read `ticks_left - 1` of an NPC
//! that was `Dying { ticks_left > 0 }` before the fight's tick, and run on
//! no other: not on the breakup's first tick, whose timer is still the
//! spawn's 0 (`jbe 0x35305` @0x3508b skips the whole pod block), nor on a
//! stalled breakup's ([`Breakup`](crate::combat::hull::Breakup)). The escape
//! ship's check sits outside that guard and asks only `_IsDying`
//! (@0x253d): it reads the same timer, or 0 on the breakup's first tick
//! or a stalled one. An NPC the fight destroys this tick is still here,
//! as `_HandleShip` runs before `_HandleShipDisplay` explodes it
//! (`_DoPlayGameWork` @0x44483, @0x444ad), so its last pod comes out on
//! its explosion tick.
//!
//! In order, for each NPC in the system's order:
//!
//! 1. **The person's pod** ([`person_pod_due`]): a [`PodEjected`]
//!    (`_SpawnEscapePod` @0x351c3), its crew's pods left as they were.
//! 2. **A crew pod** ([`crew_pod_due`], on the pass's frame counter): one
//!    fewer pod left ([`Npc::pods_left`]), and a [`PodEjected`]
//!    (@0x352bf-0x352c9).
//! 3. **The escape ship** ([`escape_roll_due`], [`escape_ship_bay`]): on
//!    `Rand(3) == 0` its bay's fighter leaves it as any fighter launches
//!    (see the `fighters` module), along its heading turned by the bay's
//!    inaccuracy (`_LaunchFighter` @0x3d87d-0x3dc5b), at its target,
//!    spending no round and making no sound. Whatever the roll, every
//!    weapon and round it carries is then gone (@0x35558-0x35579), so it
//!    rolls once. Then the launch's cut lands where
//!    [`RuleKey::EscapeShipCut`] says: a shield above 1.0 rolled below its
//!    own (`Rand(trunc(shield))`), and half the velocity
//!    (@0x3550a-0x35553). Once its carrier is no longer intact, the
//!    fighter flies on its own (`Session::orphan_fighters`).
//!
//! The view hears `snd ` 372 for each pod from where the NPC is
//! ([`ESCAPE_POD_SOUND`](crate::sound::ESCAPE_POD_SOUND)), as a fight
//! event rather than a [`SimSound`](crate::SimSound), which has no
//! position.
//!
//! **Ordering.** The original runs the block in `_HandleShip`, after the
//! frame's collision passes and before its display pass; here it runs
//! after the fight's tick, on each of double time's passes as
//! `_HandleShip` runs on each game-work pass. Every pod event is shown
//! in the same frame, so the order crosses no boundary the player sees.
//!
//! Left out: snd 372's priority 5 (`_PlaySoundDistance` @0x351fa,
//! @0x35300), which the view doesn't play by yet
//! (rdm:phase/ship-handling-fidelity/phase-20-npc-pod-sound-priority-and-fire-gate);
//! `_gameFrozen` (@0x35091), which the engine has no state for; and the
//! `_gSpeedMult` division of the interval (@0x3527a-0x35298), a division
//! by 1 at the engine's tick.
//!
//! [`PodEjected`]: crate::combat::CombatEvent::PodEjected

use super::{NpcBefore, Session};
use crate::chance::Chance;
use crate::combat::armament::{Rounds, inaccuracy};
use crate::combat::hull::Condition;
use crate::combat::{ShipRef, Sortie};
use crate::escape::escape_ship_bay;
use crate::flight::normalized;
use crate::npc_escape::{ESCAPE_ROLL_ODDS, crew_pod_due, escape_roll_due, person_pod_due};
use crate::rulebook::{RuleKey, RuleSource};
use crate::traffic::npc::{Npc, NpcId};

/// The share of its velocity the escape launch's cut leaves (0.5 at
/// 0xdd694, @0x35539-0x3554e).
pub const ESCAPE_CUT_VELOCITY: f32 = 0.5;
/// The shield above which the escape launch's cut rolls a new one (1.0
/// at 0xdd068, @0x3550f).
pub const ESCAPE_CUT_SHIELD: f32 = 1.0;

/// The death timers an NPC's checks read this tick, from its condition
/// before and after the fight's tick: the pods' (none unless it was dying
/// with its timer above 0) and the escape ship's (none unless it is
/// dying, or was destroyed this tick).
fn timers(was: Condition, now: Condition) -> (Option<i32>, Option<i32>) {
    let pods = match was {
        Condition::Dying { ticks_left, .. } if ticks_left > 0 => {
            Some(i32::try_from(ticks_left - 1).unwrap_or(i32::MAX))
        }
        _ => None,
    };
    let dying = matches!(now, Condition::Dying { .. })
        || (now == Condition::Destroyed && was != Condition::Destroyed);
    (pods, dying.then(|| pods.unwrap_or(0)))
}

impl Session {
    /// Runs each dying NPC's escape pods and its escape-ship roll for
    /// this tick, against what each NPC `was` before the fight's tick,
    /// drawing on `chance` (see the module docs).
    pub(super) fn npc_escapes(&mut self, was: &[NpcBefore], chance: &mut dyn Chance) {
        let frame = self.field().frame;
        let ids: Vec<(NpcId, Condition)> = self
            .traffic
            .npcs()
            .iter()
            .filter_map(|npc| {
                let before = was.iter().find(|before| before.id == npc.id)?;
                Some((npc.id, before.condition))
            })
            .collect();
        for (id, before) in ids {
            let Some(npc) = self.npc(id) else {
                continue;
            };
            let (pods, escape) = timers(before, npc.condition);
            if let Some(timer) = pods {
                self.eject_pods(id, timer, frame);
            }
            if let Some(timer) = escape {
                self.escape_roll(id, timer, chance);
            }
        }
    }

    /// NPC `id`'s person's pod and crew pod, its death timer at `timer`,
    /// on `frame`.
    fn eject_pods(&mut self, id: NpcId, timer: i32, frame: i32) {
        let Some(npc) = self.npc(id) else {
            return;
        };
        let death_delay = npc.hull.signed_death_delay;
        let govt_flags = npc
            .govt
            .map(|govt| self.govts.get(govt).map_or(0, |record| record.flags));
        let person = npc
            .person
            .is_some_and(|person| person_pod_due(timer, death_delay, person.flags, govt_flags));
        let crew = crew_pod_due(
            timer,
            death_delay,
            i32::from(npc.hull.pod_count),
            i32::from(npc.pods_left),
            frame,
        );
        let (at, velocity) = (npc.state.position, npc.state.velocity);
        if person {
            self.combat.eject_pod(ShipRef::Npc(id), at, velocity);
        }
        if crew {
            if let Some(npc) = self.npc_mut(id) {
                npc.pods_left -= 1;
            }
            self.combat.eject_pod(ShipRef::Npc(id), at, velocity);
        }
    }

    /// NPC `id`'s escape-ship roll, its death timer at `timer`, drawing on
    /// `chance`.
    fn escape_roll(&mut self, id: NpcId, timer: i32, chance: &mut dyn Chance) {
        let Some(npc) = self.npc(id) else {
            return;
        };
        let bay = escape_ship_bay(
            self.arsenal.weapons(),
            |weapon| npc.rounds.held(weapon),
            &self.ships,
        );
        if !escape_roll_due(true, bay.is_some(), timer, npc.hull.signed_death_delay) {
            return;
        }
        let sortie = bay
            .filter(|_| chance.below(ESCAPE_ROLL_ODDS) == 0)
            .and_then(|bay| {
                Some(Sortie {
                    carrier: ShipRef::Npc(id),
                    bay: bay.id,
                    ship: bay.carried?,
                    from: npc.state,
                    heading: normalized(
                        npc.state.heading + inaccuracy(bay.inaccuracy, &mut *chance),
                    ),
                    target: npc.target,
                })
            });
        let launched = sortie.map(|sortie| self.launch_fighter(&sortie, &mut *chance));
        if let Some(npc) = self.npc_mut(id) {
            npc.armament = crate::combat::armament::Armament::default();
            npc.rounds = crate::combat::armament::LauncherRounds::default();
        }
        let Some(launched) = launched else {
            return;
        };
        let cut = match self.rules.source_for(RuleKey::EscapeShipCut) {
            RuleSource::Engine => match launched {
                Some(_) => self
                    .traffic
                    .npcs()
                    .iter()
                    .find(|npc| npc.berth == 1)
                    .map(|npc| ShipRef::Npc(npc.id)),
                None => Some(ShipRef::Player),
            },
            RuleSource::Bible => launched.map(ShipRef::Npc),
        };
        if let Some(ship) = cut {
            self.cut_escape(ship, chance);
        }
    }

    /// The escape launch's cut on `ship`: a shield above
    /// [`ESCAPE_CUT_SHIELD`] rolled below its own on `chance`, and
    /// [`ESCAPE_CUT_VELOCITY`] of its velocity.
    fn cut_escape(&mut self, ship: ShipRef, chance: &mut dyn Chance) {
        let (shield, velocity) = match ship {
            ShipRef::Player => (
                &mut self.pilot.reserves.shield.now,
                &mut self.player.velocity,
            ),
            ShipRef::Npc(id) => {
                let Some(npc) = self.traffic.npcs_mut().iter_mut().find(|npc| npc.id == id) else {
                    return;
                };
                let Npc {
                    reserves, state, ..
                } = npc;
                (&mut reserves.shield.now, &mut state.velocity)
            }
        };
        if *shield > ESCAPE_CUT_SHIELD {
            *shield = chance.below(*shield as u32) as f32;
        }
        *velocity = *velocity * ESCAPE_CUT_VELOCITY;
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::{GovtId, PersonId, ShipId, ShipRecord, WeaponId, WeaponRecord};
    use crate::combat::armament::LauncherRounds;
    use crate::combat::{CombatEvent, Rules};
    use crate::geometry::Vec2;
    use crate::handling::ShipFields;
    use crate::pilot::Pilot;
    use crate::rulebook::Rulebook;
    use crate::stats::ShipStats;
    use crate::testkit::{FAST, FakePilotCatalog, catalog, govt, hull, npc, ship, weapon};
    use crate::traffic::SPAWN_RESERVE;
    use crate::traffic::npc::NpcPerson;

    /// The escape bay: launching the Viper, of no inaccuracy.
    const BAY: WeaponId = WeaponId(149);
    /// The escape ship, a Viper of `Flags` 0x8000 and a shield of 50.
    const VIPER: ShipId = ShipId(144);
    /// A government of `Flags` 0x0100.
    const RETREATING: GovtId = GovtId(140);
    /// A government without it.
    const STAYING: GovtId = GovtId(141);

    /// Answers `Rand(3)` with `escape` and every other draw with `other`,
    /// or the last outcome below it; records each `n` asked.
    #[derive(Debug, Default)]
    struct Rolls {
        escape: u32,
        other: u32,
        asked: Vec<u32>,
    }

    impl Rolls {
        fn launching(escape: u32, other: u32) -> Self {
            Self {
                escape,
                other,
                asked: Vec::new(),
            }
        }

        /// How many escape rolls it was asked.
        fn rolls(&self) -> usize {
            self.asked
                .iter()
                .filter(|&&n| n == ESCAPE_ROLL_ODDS)
                .count()
        }
    }

    impl Chance for Rolls {
        fn fires(&mut self, _percent: u8) -> bool {
            false
        }

        fn below(&mut self, n: u32) -> u32 {
            self.asked.push(n);
            if n == ESCAPE_ROLL_ODDS {
                self.escape
            } else {
                self.other.min(n.saturating_sub(1))
            }
        }
    }

    /// The test catalog with the Viper, its bay of `inaccuracy` and the
    /// two governments.
    fn escaping(inaccuracy: i16) -> FakePilotCatalog {
        let mut catalog = catalog();
        catalog.ship_records.push(ShipRecord {
            flags: 0x8000,
            ..ship(VIPER.0, ShipFields { shield: 50, ..FAST })
        });
        catalog.hulls.push(hull(VIPER.0));
        catalog.weapons.push(WeaponRecord {
            guidance: 99,
            ammo_type: VIPER.0,
            speed: 400,
            inaccuracy,
            ..weapon(BAY.0)
        });
        catalog.govts.extend([
            crate::catalog::GovtRecord {
                flags: 0x0100,
                ..govt(RETREATING.0)
            },
            crate::catalog::GovtRecord {
                flags: 0x00ff,
                ..govt(STAYING.0)
            },
        ]);
        catalog
    }

    fn flying(catalog: &FakePilotCatalog, cut: RuleSource) -> Session {
        let pilot = Pilot::new(catalog, "Ada").expect("starts");
        Session::fly(catalog, pilot)
            .expect("flies")
            .with_rules(Rulebook::default().with_override(RuleKey::EscapeShipCut, cut))
    }

    /// An NPC of `DeathDelay` `death_delay` and `pods` crew pods at
    /// (300, 0), moving at (2, 4), its armour gone: it breaks up on the
    /// next fight tick.
    fn doomed(death_delay: i16, pods: u16) -> Npc {
        let mut npc = npc(0, ShipStats::new(FAST, &[]));
        npc.hull.death_delay = u32::try_from(death_delay).unwrap_or(0);
        npc.hull.signed_death_delay = i32::from(death_delay);
        npc.hull.pod_count = pods;
        npc.pods_left = pods;
        npc.state.position = Vec2::new(300.0, 0.0);
        npc.state.velocity = Vec2::new(2.0, 4.0);
        npc.reserves.armor.now = 0.0;
        npc
    }

    /// A person of `flags`.
    fn person(flags: u16) -> NpcPerson {
        NpcPerson {
            id: PersonId(128),
            flags,
            coward: 0,
            comm_quote: -1,
            hail_quote: -1,
            mission: false,
            portrait: None,
            invincible: false,
            grudge: false,
            quoted: false,
            quoted_at: None,
        }
    }

    /// `npc` with the escape bay, `fighters` aboard.
    fn carrying(mut npc: Npc, fighters: i32) -> Npc {
        npc.rounds = LauncherRounds::from([(BAY, fighters)]);
        npc
    }

    /// Fights ticks on `chance` from the breakup on, the pass's frame at
    /// the tick's count from the breakup plus `frame_offset`, until `id`
    /// is gone: the ticks from the breakup (0) on which it ejected a pod,
    /// each once a pod.
    fn pod_ticks(
        session: &mut Session,
        id: NpcId,
        frame_offset: i16,
        chance: &mut Rolls,
    ) -> Vec<i16> {
        let mut ticks = Vec::new();
        for tick in 0..400_i16 {
            if session.npc(id).is_none() {
                return ticks;
            }
            session.pass_frame = tick + frame_offset;
            session.tick_combat(Rules::default(), &mut *chance);
            for event in session.take_combat_events() {
                if let CombatEvent::PodEjected { ship, at, velocity } = event {
                    assert_eq!(ship, ShipRef::Npc(id));
                    assert_eq!(at, Vec2::new(300.0, 0.0), "where the ship is");
                    assert_eq!(velocity, Vec2::new(2.0, 4.0), "at its velocity");
                    ticks.push(tick);
                }
            }
        }
        panic!("never destroyed");
    }

    #[test]
    fn a_dying_liner_ejects_its_first_pod_at_half_its_delay_then_on_each_interval_frame() {
        // DeathDelay 100 and PodCount 10: an interval of 10. The timer
        // after tick t's decrement is 100 - t, at 50 on tick 50, and the
        // ship explodes on tick 98, at 2.
        let catalog = escaping(0);
        let mut session = flying(&catalog, RuleSource::Engine);
        let id = session.traffic.add_npc(doomed(100, 10));
        let ticks = pod_ticks(&mut session, id, 2, &mut Rolls::default());
        assert_eq!(
            ticks,
            [50, 58, 68, 78, 88, 98],
            "the first at once, the rest on frames a multiple of 10, the last on the explosion tick"
        );
    }

    #[test]
    fn the_crew_pods_stop_once_none_is_left() {
        // PodCount 2: an interval of max(10, trunc(50 x 0.4)) = 20.
        let catalog = escaping(0);
        let mut session = flying(&catalog, RuleSource::Engine);
        let id = session.traffic.add_npc(doomed(100, 2));
        let left = |session: &Session| session.npc(id).map(|npc| npc.pods_left);
        session.pass_frame = 0;
        session.tick_combat(Rules::default(), &mut Rolls::default());
        assert_eq!(left(&session), Some(2), "the breakup tick ejects none");
        let ticks = pod_ticks(&mut session, id, 1, &mut Rolls::default());
        assert_eq!(ticks, [49, 59], "then none on frame 80");
    }

    #[test]
    fn the_breakup_tick_ejects_no_pod_though_the_escape_roll_fires() {
        // DeathDelay 0: destroyed on the breakup tick, whose timer is the
        // spawn's 0, at or below half the delay and equal to it.
        let catalog = escaping(0);
        let mut session = flying(&catalog, RuleSource::Engine);
        let mut doomed = carrying(doomed(0, 10), 1);
        doomed.person = Some(person(0x0002));
        let id = session.traffic.add_npc(doomed);
        session.pass_frame = 0;
        let mut chance = Rolls::launching(0, 0);
        session.tick_combat(Rules::default(), &mut chance);
        let events = session.take_combat_events();
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, CombatEvent::PodEjected { .. })),
            "{events:?}"
        );
        assert!(
            events.iter().any(|event| matches!(
                event,
                CombatEvent::Destroyed { ship, .. } if *ship == ShipRef::Npc(id)
            )),
            "destroyed this tick"
        );
        assert_eq!(chance.rolls(), 1, "the escape ship's roll");
        assert!(
            session.npcs().iter().any(|npc| npc.ship == VIPER),
            "and its launch"
        );
    }

    #[test]
    fn a_persons_pod_comes_at_exactly_half_the_delay_beside_any_crew_pod() {
        let catalog = escaping(0);
        let ticks_of = |death_delay, pods, flags, govt| {
            let mut session = flying(&catalog, RuleSource::Engine);
            let mut doomed = doomed(death_delay, pods);
            doomed.person = Some(person(flags));
            doomed.govt = govt;
            let id = session.traffic.add_npc(doomed);
            pod_ticks(&mut session, id, 7, &mut Rolls::default())
        };
        assert_eq!(ticks_of(100, 0, 0x0002, None), [50], "at timer 50 only");
        assert_eq!(
            ticks_of(100, 1, 0x0002, None),
            [50, 50],
            "the person's and the crew's together"
        );
        assert!(
            ticks_of(101, 0, 0x0002, None).is_empty(),
            "an odd delay: never"
        );
        assert!(ticks_of(100, 0, 0x0002, Some(RETREATING)).is_empty());
        assert_eq!(ticks_of(100, 0, 0x0002, Some(STAYING)), [50]);
        assert!(ticks_of(100, 0, 0xfffd, None).is_empty(), "no 0x0002");
    }

    /// A session of `cut`'s reading with a dying carrier of
    /// `DeathDelay` 100 holding 2 escape fighters, its berth `berth`, its
    /// shield 40 and targeting the player; the carrier.
    fn carrier(catalog: &FakePilotCatalog, cut: RuleSource, berth: u32) -> (Session, NpcId) {
        let mut session = flying(catalog, cut);
        let mut doomed = carrying(doomed(100, 0), 2);
        doomed.reserves.shield.now = 40.0;
        doomed.target = Some(ShipRef::Player);
        doomed.state.heading = 90.0;
        let id = session.traffic.add_npc_at(doomed, berth);
        (session, id)
    }

    /// The Vipers in the system.
    fn vipers(session: &Session) -> Vec<Npc> {
        session
            .npcs()
            .iter()
            .filter(|npc| npc.ship == VIPER)
            .cloned()
            .collect()
    }

    /// Fights `session` to `id`'s end on `chance`.
    fn to_the_end(session: &mut Session, id: NpcId, chance: &mut Rolls) {
        for tick in 0..200 {
            session.pass_frame = tick;
            session.tick_combat(Rules::default(), &mut *chance);
            if session.npc(id).is_none() {
                return;
            }
        }
        panic!("never destroyed");
    }

    #[test]
    fn by_the_other_reading_the_escape_fighter_leaves_on_a_roll_of_0_cut_once() {
        let catalog = escaping(0);
        let (mut session, id) = carrier(&catalog, RuleSource::Bible, 1);
        let mut chance = Rolls::launching(0, 7);
        session.tick_combat(Rules::default(), &mut chance);
        assert_eq!(chance.rolls(), 1, "on the breakup tick");
        let launched = vipers(&session);
        assert_eq!(launched.len(), 1);
        let fighter = &launched[0];
        assert_eq!(fighter.state.position, Vec2::new(300.0, 0.0));
        assert_eq!(fighter.state.heading, 90.0, "the carrier's, no inaccuracy");
        assert_eq!(fighter.target, Some(ShipRef::Player));
        let pushed = crate::bay::launch_velocity(Vec2::new(2.0, 4.0), 90.0, 4.0, 6.0);
        assert_eq!(fighter.state.velocity, pushed * 0.5, "half its launch");
        assert_eq!(fighter.reserves.shield.now, 7.0, "Rand(50)");
        assert!(chance.asked.contains(&50));
        let carrier = session.npc(id).expect("dying");
        assert_eq!(carrier.reserves.shield.now, 40.0, "untouched");
        assert_eq!(carrier.state.velocity, Vec2::new(2.0, 4.0));
        assert_eq!(
            carrier.rounds,
            LauncherRounds::default(),
            "every round gone"
        );
        assert!(carrier.armament.mounts().is_empty(), "and every weapon");
        to_the_end(&mut session, id, &mut chance);
        assert_eq!(chance.rolls(), 1, "no second roll");
        assert_eq!(vipers(&session).len(), 1);
        assert_eq!(vipers(&session)[0].carrier, None, "let go of its carrier");
    }

    #[test]
    fn by_the_engine_the_cut_lands_on_berth_1_whichever_ship_holds_it() {
        let catalog = escaping(0);
        // The carrier at berth 2: the fighter takes berth 1, and the cut.
        let (mut session, id) = carrier(&catalog, RuleSource::Engine, 2);
        let mut chance = Rolls::launching(0, 7);
        session.tick_combat(Rules::default(), &mut chance);
        let fighter = &vipers(&session)[0];
        assert_eq!(fighter.berth, 1);
        let pushed = crate::bay::launch_velocity(Vec2::new(2.0, 4.0), 90.0, 4.0, 6.0);
        assert_eq!(fighter.state.velocity, pushed * 0.5);
        assert_eq!(fighter.reserves.shield.now, 7.0);
        let held = session.npc(id).expect("dying");
        assert_eq!(held.reserves.shield.now, 40.0);
        // The carrier at berth 1: the fighter takes berth 2, the carrier the
        // cut.
        let (mut session, id) = carrier(&catalog, RuleSource::Engine, 1);
        let mut chance = Rolls::launching(0, 7);
        session.tick_combat(Rules::default(), &mut chance);
        let fighter = &vipers(&session)[0];
        assert_eq!(fighter.berth, 2);
        assert_eq!(fighter.state.velocity, pushed, "uncut");
        assert_eq!(fighter.reserves.shield.now, 50.0);
        let held = session.npc(id).expect("dying");
        assert_eq!(held.reserves.shield.now, 7.0, "Rand(40)");
        assert!(chance.asked.contains(&40));
        assert_eq!(held.state.velocity, Vec2::new(1.0, 2.0));
        to_the_end(&mut session, id, &mut chance);
        assert_eq!(chance.rolls(), 1);
    }

    #[test]
    fn the_cut_rolls_only_a_shield_above_1() {
        let catalog = escaping(0);
        for (shield, cut) in [(1.0, 1.0), (1.5, 0.0), (0.5, 0.5)] {
            let (mut session, id) = carrier(&catalog, RuleSource::Engine, 1);
            session.npc_mut(id).expect("there").reserves.shield.now = shield;
            let mut chance = Rolls::launching(0, 7);
            session.tick_combat(Rules::default(), &mut chance);
            let held = session.npc(id).expect("dying");
            assert_eq!(held.reserves.shield.now, cut, "{shield}");
            assert_eq!(held.state.velocity, Vec2::new(1.0, 2.0), "{shield}");
        }
    }

    #[test]
    fn a_roll_other_than_0_launches_nothing_and_never_rolls_again() {
        let catalog = escaping(0);
        for cut in [RuleSource::Engine, RuleSource::Bible] {
            let (mut session, id) = carrier(&catalog, cut, 1);
            let mut chance = Rolls::launching(1, 7);
            to_the_end(&mut session, id, &mut chance);
            assert_eq!(chance.rolls(), 1, "{cut:?}");
            assert_eq!(vipers(&session), [], "{cut:?}");
            assert_eq!(session.pilot.reserves.shield.now, 30.0, "{cut:?}");
        }
    }

    #[test]
    fn the_escape_fighter_leaves_along_its_carriers_heading_turned_by_the_bays_inaccuracy() {
        // Inaccuracy 5: Rand(10) - 5, 7 - 5 = 2 degrees off 90.
        let catalog = escaping(5);
        let (mut session, _) = carrier(&catalog, RuleSource::Bible, 1);
        let mut chance = Rolls::launching(0, 7);
        session.tick_combat(Rules::default(), &mut chance);
        assert_eq!(vipers(&session)[0].state.heading, 92.0);
        assert!(chance.asked.contains(&10));
    }

    #[test]
    fn with_no_free_berth_the_engine_cuts_the_player_and_the_other_reading_nothing() {
        let catalog = escaping(0);
        for (cut, shield, velocity) in [
            (RuleSource::Engine, 7.0, Vec2::new(1.0, -2.0)),
            (RuleSource::Bible, 30.0, Vec2::new(2.0, -4.0)),
        ] {
            let (mut session, id) = carrier(&catalog, cut, 1);
            while session
                .traffic
                .free_berth(session.spawn_reserve(SPAWN_RESERVE))
                .is_some()
            {
                let mut far = npc(0, ShipStats::default());
                far.state.position = Vec2::new(5000.0, 5000.0);
                session.traffic.add_npc(far);
            }
            session.player.velocity = Vec2::new(2.0, -4.0);
            let mut chance = Rolls::launching(0, 7);
            session.tick_combat(Rules::default(), &mut chance);
            assert_eq!(chance.rolls(), 1, "{cut:?}");
            assert_eq!(vipers(&session), [], "{cut:?}: no room");
            assert_eq!(session.pilot.reserves.shield.now, shield, "{cut:?}");
            assert_eq!(session.player.velocity, velocity, "{cut:?}");
            let carrier = session.npc(id).expect("dying");
            assert_eq!(carrier.rounds, LauncherRounds::default(), "{cut:?}");
        }
    }

    #[test]
    fn the_timers_read_the_condition_before_and_after_the_tick() {
        let dying = |ticks_left, starting| Condition::Dying {
            ticks_left,
            starting,
        };
        assert_eq!(
            timers(Condition::Intact, dying(100, true)),
            (None, Some(0)),
            "the breakup tick"
        );
        assert_eq!(
            timers(Condition::Disabled, Condition::Destroyed),
            (None, Some(0)),
            "broken up and destroyed at once"
        );
        assert_eq!(
            timers(dying(100, true), dying(99, false)),
            (Some(99), Some(99))
        );
        assert_eq!(
            timers(dying(3, false), Condition::Destroyed),
            (Some(2), Some(2)),
            "the explosion tick"
        );
        assert_eq!(
            timers(dying(0, true), dying(0, true)),
            (None, Some(0)),
            "a stalled breakup"
        );
        assert_eq!(timers(Condition::Intact, Condition::Intact), (None, None));
        assert_eq!(timers(Condition::Intact, Condition::Disabled), (None, None));
        assert_eq!(
            timers(Condition::Destroyed, Condition::Destroyed),
            (None, None)
        );
    }

    /// The test-support `fixture::place`, which only other crates' tests
    /// call, tested here where a session's traffic can be reached.
    #[test]
    fn place_puts_a_ship_where_it_is_told_at_rest() {
        let catalog = catalog();
        let pilot = Pilot::new(&catalog, "Ada").expect("starts");
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        let mut moving = npc(0, ShipStats::default());
        moving.state.velocity = Vec2::new(3.0, 1.0);
        let id = session.traffic.add_npc(moving);
        crate::fixture::place(&mut session, id, Vec2::new(40.0, -80.0));
        let placed = session.npc(id).expect("there").state;
        assert_eq!(
            (placed.position, placed.velocity),
            (Vec2::new(40.0, -80.0), Vec2::ZERO)
        );
    }
}
