//! A dying NPC's escape pods and its last fighter, as `_HandleShip`'s
//! dying block runs them (@0x35080-0x3557b), by the rules of
//! [`npc_escape`](crate::npc_escape).
//!
//! **The death timer.** The fight counts an NPC's breakup down
//! ([`Condition::Dying`]'s `ticks_left`): the NPC's own step takes a tick
//! off it (`Condition::count_down`), and its display step sets an unset
//! one (see the fight's `display` module). `_HandleShip` takes the tick
//! off before its checks, and only while the timer is above 0
//! (@0x35083-0x350a3), so the pod checks read `ticks_left - 1` of an NPC
//! that was `Dying { ticks_left > 0 }` before the pass's fight, and run on
//! no other: not while the timer is unset, on the pass a hit breaks it up
//! and the one its display sets it on (`jbe 0x35305` @0x3508b skips the
//! whole pod block), nor on a stalled breakup's
//! ([`Breakup`](crate::combat::hull::Breakup)). The escape
//! ship's check sits outside that guard and asks only `_IsDying`
//! (@0x253d) as the NPC's own step finds it ([`AtStep`], noted by the
//! ship loop before the step's fire, `Session::ship_loop`): its death
//! timer running or its armour at none or below. It reads the same
//! timer, or 0 on the breakup's first tick or a stalled one. A breakup
//! the collision passes or the shots begin is found by that tick's step;
//! one a beam begins (`_HandleBeams` @0x4453d, after the whole loop) is
//! first found by the next tick's, its timer still 0, as the display
//! that sets it comes after the step. An NPC the fight destroys this
//! tick is still here, as `_HandleShip` runs before
//! `_HandleShipDisplay` explodes it (`_DoPlayGameWork` @0x44483,
//! @0x444ad), so its last pod comes out on its explosion tick.
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
//!    fighter leaves its fleet (`Space::lose_leads`).
//!
//! The view hears `snd ` 372 for each pod from where the NPC is
//! ([`ESCAPE_POD_SOUND`](crate::sound::ESCAPE_POD_SOUND)), as a fight
//! event rather than a [`SimSound`](crate::SimSound), which has no
//! position.
//!
//! **Ordering.** The original runs the block in `_HandleShip`, after the
//! NPC's death timer (@0x35083) and before its display (@0x444ad), on each
//! game-work pass. So does the ship loop ([`Session::npc_escape`] from
//! `Session::ship_loop`, rdm weapons-fidelity phase 47), on each of double
//! time's passes: an escape fighter launched into a slot above its
//! carrier's is reached later in the loop and drawn on its launch frame,
//! and the carrier's final explosion that tick reaches it. The words the
//! pass's stray beams stepped are settled just before
//! (`Session::step_stray_words`), so the pods read the crew pods left, as
//! the original steps a stray beam's word at its spawn. Only the NPC's
//! own beam steps its word, and a dying NPC fires none, so no fixture
//! reaches a word stepped in the pass that its pods then read.
//!
//! The separate parts (`Session::fight`) still run every NPC's block
//! after the beams, reading each NPC as its step found it, the stray
//! words stepped first; they are no game path.
//!
//! Left out: snd 372's priority 5 (`_PlaySoundDistance` @0x351fa,
//! @0x35300), which the view doesn't play by yet
//! (rdm:phase/ship-handling-fidelity/phase-20-npc-pod-sound-priority-and-fire-gate);
//! `_gameFrozen` (@0x35091), which the engine has no state for; and the
//! `_gSpeedMult` division of the interval (@0x3527a-0x35298), a division
//! by 1 at the engine's tick.
//!
//! [`PodEjected`]: crate::combat::CombatEvent::PodEjected

use super::{NpcBefore, Session, Space, Statics};
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

/// An NPC as its own step in the ship loop found it, before its fire and
/// its display (`Session::ship_loop`): whether `_HandleShip`'s escape-ship
/// check finds it dying (`_IsDying` @0x253d).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct AtStep {
    id: NpcId,
    /// `_IsDying`: its death timer runs, or its armour is at none or
    /// below on a ship that holds any, though no display has settled its
    /// condition yet (`Condition::is_dying`).
    dying: bool,
}

impl AtStep {
    pub(super) fn of(npc: &Npc) -> Self {
        Self {
            id: npc.id,
            dying: npc.condition.is_dying(npc.reserves.armor),
        }
    }
}

/// The death timers an NPC's checks read this tick, from its condition
/// before the fight's tick and whether its own step found it `dying`
/// ([`AtStep`]): the pods' (none unless it was dying with its timer above
/// 0) and the escape ship's (none unless its step found it dying).
fn timers(was: Condition, dying: bool) -> (Option<i32>, Option<i32>) {
    let pods = match was {
        Condition::Dying { ticks_left, .. } if ticks_left > 0 => {
            Some(i32::try_from(ticks_left - 1).unwrap_or(i32::MAX))
        }
        _ => None,
    };
    (pods, dying.then(|| pods.unwrap_or(0)))
}

/// Whether an NPC's escape pods or its escape-ship roll may run this
/// tick: it `was` dying with its timer above 0 before the fight's tick, or
/// its own step finds it dying (`at_step`; see [`timers`]). The ship loop
/// runs its dying block between its step and its display only then
/// (`Session::ship_loop`).
pub(super) fn escape_may_run(was: Condition, at_step: AtStep) -> bool {
    timers(was, at_step.dying) != (None, None)
}

impl Session {
    /// Runs each dying NPC's escape pods and its escape-ship roll for
    /// this tick, against what each NPC `was` before the fight's tick and
    /// how its own step found it (`stepped`), drawing on `chance` (see the
    /// module docs), as the separate parts run them (`Session::fight`).
    /// An NPC that took no step this tick takes neither.
    pub(super) fn npc_escapes(
        &mut self,
        was: &[NpcBefore],
        stepped: &[AtStep],
        chance: &mut dyn Chance,
    ) {
        let ids: Vec<(Condition, AtStep)> = self
            .space
            .traffic
            .npcs()
            .iter()
            .filter_map(|npc| {
                let before = was.iter().find(|before| before.id == npc.id)?;
                let at_step = stepped.iter().find(|at_step| at_step.id == npc.id)?;
                Some((before.condition, *at_step))
            })
            .collect();
        for (before, at_step) in ids {
            self.npc_escape(before, at_step, chance);
        }
    }

    /// Runs one NPC's escape pods and escape-ship roll for this tick (see
    /// the module docs), the NPC as its own step found it (`at_step`),
    /// against its condition before the pass's fight (`was`), drawing on
    /// `chance`. Nothing happens for an NPC no longer in the system.
    pub(super) fn npc_escape(&mut self, was: Condition, at_step: AtStep, chance: &mut dyn Chance) {
        let id = at_step.id;
        if self.space.npc(id).is_none() {
            return;
        }
        let frame = self.field().frame;
        let (pods, escape) = timers(was, at_step.dying);
        if let Some(timer) = pods {
            self.space.eject_pods(&self.statics, id, timer, frame);
        }
        if let Some(timer) = escape {
            self.escape_roll(id, timer, chance);
        }
    }

    /// NPC `id`'s escape-ship roll, its death timer at `timer`, drawing on
    /// `chance`.
    fn escape_roll(&mut self, id: NpcId, timer: i32, chance: &mut dyn Chance) {
        let Some(npc) = self.space.npc(id) else {
            return;
        };
        let bay = escape_ship_bay(
            self.statics.arsenal.weapons(),
            |weapon| npc.rounds.held(weapon),
            &self.statics.ships,
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
        if let Some(npc) = self.space.npc_mut(id) {
            npc.armament = crate::combat::armament::Armament::default();
            npc.rounds = crate::combat::armament::LauncherRounds::default();
        }
        let Some(launched) = launched else {
            return;
        };
        let cut = match self.setup.rules.source_for(RuleKey::EscapeShipCut) {
            RuleSource::Engine => match launched {
                Some(_) => self
                    .space
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
                &mut self.ship.player.velocity,
            ),
            ShipRef::Npc(id) => {
                let Some(npc) = self
                    .space
                    .traffic
                    .npcs_mut()
                    .iter_mut()
                    .find(|npc| npc.id == id)
                else {
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

impl Space {
    /// NPC `id`'s person's pod and crew pod, its death timer at `timer`,
    /// on `frame`.
    fn eject_pods(&mut self, statics: &Statics, id: NpcId, timer: i32, frame: i32) {
        let Some(npc) = self.npc(id) else {
            return;
        };
        let death_delay = npc.hull.signed_death_delay;
        let govt_flags = npc
            .govt
            .map(|govt| statics.govts.get(govt).map_or(0, |record| record.flags));
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
        escaping_from(catalog(), inaccuracy)
    }

    /// `catalog` with the Viper, its bay of `inaccuracy` and the two
    /// governments.
    fn escaping_from(mut catalog: FakePilotCatalog, inaccuracy: i16) -> FakePilotCatalog {
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
        npc.pods_left = i16::try_from(pods).unwrap_or(i16::MAX);
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
            zero_fuel: false,
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
            if session.space.npc(id).is_none() {
                return ticks;
            }
            session.space.pass_frame = tick + frame_offset;
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
        let id = session.space.traffic.add_npc(doomed(100, 10));
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
        let id = session.space.traffic.add_npc(doomed(100, 2));
        let left = |session: &Session| session.space.npc(id).map(|npc| npc.pods_left);
        session.space.pass_frame = 0;
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
        let id = session.space.traffic.add_npc(doomed);
        session.space.pass_frame = 0;
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
            let id = session.space.traffic.add_npc(doomed);
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
        let id = session.space.traffic.add_npc_at(doomed, berth);
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
            session.space.pass_frame = tick;
            session.tick_combat(Rules::default(), &mut *chance);
            if session.space.npc(id).is_none() {
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
        let carrier = session.space.npc(id).expect("dying");
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
        let held = session.space.npc(id).expect("dying");
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
        let held = session.space.npc(id).expect("dying");
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
            session
                .space
                .npc_mut(id)
                .expect("there")
                .reserves
                .shield
                .now = shield;
            let mut chance = Rolls::launching(0, 7);
            session.tick_combat(Rules::default(), &mut chance);
            let held = session.space.npc(id).expect("dying");
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
                .space
                .traffic
                .free_berth(session.setup.spawn_reserve(SPAWN_RESERVE))
                .is_some()
            {
                let mut far = npc(0, ShipStats::default());
                far.state.position = Vec2::new(5000.0, 5000.0);
                session.space.traffic.add_npc(far);
            }
            session.ship.player.velocity = Vec2::new(2.0, -4.0);
            let mut chance = Rolls::launching(0, 7);
            session.tick_combat(Rules::default(), &mut chance);
            assert_eq!(chance.rolls(), 1, "{cut:?}");
            assert_eq!(vipers(&session), [], "{cut:?}: no room");
            assert_eq!(session.pilot.reserves.shield.now, shield, "{cut:?}");
            assert_eq!(session.ship.player.velocity, velocity, "{cut:?}");
            let carrier = session.space.npc(id).expect("dying");
            assert_eq!(carrier.rounds, LauncherRounds::default(), "{cut:?}");
        }
    }

    #[test]
    fn the_timers_read_the_condition_before_the_tick_and_at_the_step() {
        let dying = |ticks_left, starting| Condition::Dying {
            ticks_left,
            starting,
        };
        assert_eq!(
            timers(Condition::Intact, true),
            (None, Some(0)),
            "the breakup tick"
        );
        assert_eq!(timers(dying(100, true), true), (Some(99), Some(99)));
        assert_eq!(
            timers(dying(3, false), true),
            (Some(2), Some(2)),
            "the explosion tick"
        );
        assert_eq!(
            timers(dying(0, true), true),
            (None, Some(0)),
            "a stalled breakup, or one a beam began"
        );
        assert_eq!(timers(Condition::Intact, false), (None, None));
        assert_eq!(timers(Condition::Disabled, false), (None, None));
    }

    #[test]
    fn the_step_finds_a_ship_dying_by_its_timer_or_its_armour() {
        // `_IsDying` (@0x253d): the timer above 0, or the armour at none or
        // below, though no display has settled the condition yet.
        let mut ship = npc(0, ShipStats::new(FAST, &[]));
        assert!(!AtStep::of(&ship).dying, "intact");
        ship.condition = Condition::Disabled;
        assert!(!AtStep::of(&ship).dying, "disabled");
        ship.reserves.armor.now = 0.0;
        assert!(AtStep::of(&ship).dying, "its armour gone");
        ship.reserves.armor.now = 1.0;
        ship.condition = Condition::Dying {
            ticks_left: 0,
            starting: true,
        };
        assert!(AtStep::of(&ship).dying, "breaking up");
        ship.reserves.armor.now = 0.0;
        ship.condition = Condition::Destroyed;
        assert!(!AtStep::of(&ship).dying, "gone");
    }

    #[test]
    fn a_ship_type_holding_no_armour_is_never_found_dying_by_its_armour() {
        // `_IsDying` reads the core's `armour_gone`: a type with no armour
        // at all (stock `shïp` 895) has none to lose.
        let mut ship = npc(0, ShipStats::new(FAST, &[]));
        ship.reserves.armor.max = 0.0;
        ship.reserves.armor.now = 0.0;
        assert!(!AtStep::of(&ship).dying, "intact");
        ship.condition = Condition::Disabled;
        assert!(!AtStep::of(&ship).dying, "disabled");
    }

    #[test]
    fn a_carrier_holding_no_armour_never_rolls_its_escape_ship() {
        let catalog = escaping(0);
        let mut session = flying(&catalog, RuleSource::Engine);
        let mut carrier = carrying(doomed(100, 0), 2);
        carrier.reserves.armor.max = 0.0;
        let id = session.space.traffic.add_npc(carrier);
        let mut chance = Rolls::launching(0, 0);
        for tick in 0..5 {
            session.space.pass_frame = tick;
            session.tick_combat(Rules::default(), &mut chance);
        }
        assert_eq!(chance.rolls(), 0, "never read dying");
        assert!(vipers(&session).is_empty(), "nothing launched");
        assert_eq!(
            session.space.npc(id).map(|npc| npc.condition),
            Some(Condition::Intact),
            "it never breaks up"
        );
    }

    /// The session's armed catalog, the player's weapon a 200-pixel beam
    /// doing 5 mass and 5 energy damage, with the escape ship's Viper and
    /// bay; its NPC, a carrier of `DeathDelay` 100 holding 2 escape
    /// fighters, 100 pixels ahead of the player, its shield gone and 1
    /// armour left; the player's trigger held.
    fn beamed() -> (FakePilotCatalog, Session) {
        let beam = WeaponRecord {
            guidance: 0,
            count: 3,
            reload: 10,
            speed: 0,
            beam_length: 200,
            mass_dmg: 5,
            energy_dmg: 5,
            ..crate::session::tests::blaster()
        };
        carrier_ahead(FakePilotCatalog {
            weapons: vec![beam],
            ..crate::session::tests::armed()
        })
    }

    /// `armed`, the session's armed catalog with its weapon as the test
    /// gives it, with the escape ship's Viper and bay; its NPC, a carrier
    /// of `DeathDelay` 100 holding 2 escape fighters, 100 pixels ahead of
    /// the player, its shield gone and 1 armour left; the player's
    /// trigger held.
    fn carrier_ahead(armed: FakePilotCatalog) -> (FakePilotCatalog, Session) {
        let catalog = escaping_from(armed, 0);
        let mut session = crate::session::tests::facing_an_npc(&catalog, 180);
        let carrier = &mut session.space.traffic.npcs_mut()[0];
        carrier.rounds = LauncherRounds::from([(BAY, 2)]);
        carrier.hull.death_delay = 100;
        carrier.hull.signed_death_delay = 100;
        carrier.reserves.shield.now = 0.0;
        carrier.reserves.armor.now = 1.0;
        session.hold_trigger(crate::session::tests::FIRE);
        (catalog, session)
    }

    /// Keeps every NPC's goal and holds no trigger.
    #[derive(Debug)]
    struct Keep;

    impl crate::ai::Behaviour for Keep {
        fn decide(
            &self,
            npc: &Npc,
            _around: &crate::ai::Surroundings,
            _chance: &mut dyn Chance,
        ) -> crate::ai::Goal {
            npc.goal
        }
    }

    #[test]
    fn an_npc_a_beam_breaks_up_rolls_its_escape_ship_on_the_next_pass() {
        // `_HandleBeams` (@0x4453d) breaks it up after its `_HandleShip`
        // (@0x44483), whose escape check (@0x3547d) first finds it dying
        // on the next frame, its timer still 0 (@0x253d).
        let (catalog, mut session) = beamed();
        let mut chance = Rolls::launching(1, 0);
        let controls = crate::flight::Controls::default();
        let rules = Rules::default();
        assert!(!session.game_work(controls, &catalog, &Keep, rules, &mut chance));
        let condition = session.npcs()[0].condition;
        assert!(
            matches!(condition, Condition::Dying { ticks_left: 0, .. }),
            "broken up by the beam: {condition:?}"
        );
        assert_eq!(chance.rolls(), 0, "not on the beam's pass");
        assert!(!session.game_work(controls, &catalog, &Keep, rules, &mut chance));
        assert_eq!(chance.rolls(), 1, "on the next");
    }

    #[test]
    fn an_npc_a_shot_breaks_up_in_the_collision_passes_rolls_in_that_pass() {
        // The collision passes (@0x43e70) come before its `_HandleShip`.
        let (catalog, mut session) = carrier_ahead(FakePilotCatalog {
            weapons: vec![WeaponRecord {
                mass_dmg: 50,
                energy_dmg: 50,
                ..crate::session::tests::blaster()
            }],
            ..crate::session::tests::armed()
        });
        let mut chance = Rolls::launching(1, 0);
        let controls = crate::flight::Controls::default();
        for _ in 0..40 {
            assert!(!session.game_work(controls, &catalog, &Keep, Rules::default(), &mut chance));
            if matches!(session.npcs()[0].condition, Condition::Dying { .. }) {
                assert_eq!(chance.rolls(), 1, "on the breakup's pass");
                return;
            }
            assert_eq!(chance.rolls(), 0);
        }
        panic!("never broken up");
    }

    /// A session in flight whose one NPC is a carrier of `DeathDelay`
    /// `death_delay` and mass 200 at berth `berth`, `condition`, its
    /// armour gone, holding 2 escape fighters, facing 90 and targeting the
    /// player; the catalog, the session and the carrier.
    fn dying_carrier(
        berth: u32,
        death_delay: u32,
        condition: Condition,
    ) -> (FakePilotCatalog, Session, NpcId) {
        let catalog = escaping_from(crate::session::tests::armed(), 0);
        let mut session = crate::session::tests::facing_an_npc(&catalog, 180);
        let carrier = &mut session.space.traffic.npcs_mut()[0];
        carrier.berth = berth;
        carrier.rounds = LauncherRounds::from([(BAY, 2)]);
        carrier.hull.death_delay = death_delay;
        carrier.hull.signed_death_delay = i32::try_from(death_delay).unwrap_or(i32::MAX);
        carrier.hull.mass = 200.0;
        carrier.reserves.armor.now = 0.0;
        carrier.condition = condition;
        carrier.target = Some(ShipRef::Player);
        carrier.state.heading = 90.0;
        let id = carrier.id;
        (catalog, session, id)
    }

    /// Counting down, 50 ticks left: at or below half a `DeathDelay` of
    /// 100, so its escape ship rolls.
    const HALFWAY: Condition = Condition::Dying {
        ticks_left: 50,
        starting: false,
    };

    /// An advance pass of `session`, its NPCs keeping their goals, the
    /// escape roll launching.
    fn launch_pass(session: &mut Session, catalog: &FakePilotCatalog) {
        let mut chance = Rolls::launching(0, 0);
        let controls = crate::flight::Controls::default();
        assert!(!session.game_work(controls, catalog, &Keep, Rules::default(), &mut chance));
        assert_eq!(chance.rolls(), 1);
    }

    #[test]
    fn an_escape_fighter_launched_above_its_carrier_is_drawn_on_its_launch_frame() {
        // `_LaunchFighter` makes its slot active at once (@0x3c90b,
        // @0x3d680), so the ship loop (@0x4445c-0x444c4) reaches berth 2
        // after the carrier's: its `_HandleShip` moves it, unsteered, and
        // its display shows it before the frame.
        let (catalog, mut session, carrier) = dying_carrier(1, 100, HALFWAY);
        launch_pass(&mut session, &catalog);
        let fighter = &vipers(&session)[0];
        assert_eq!(fighter.berth, 2);
        let from = session.space.npc(carrier).expect("dying").state.position;
        assert_eq!(fighter.state.position, from + fighter.state.velocity);
        assert_eq!(fighter.state.heading, 90.0, "unsteered");
        assert!(
            session.draw_order().contains(&ShipRef::Npc(fighter.id)),
            "{:?}",
            session.draw_order()
        );
    }

    #[test]
    fn an_escape_fighter_launched_below_its_carrier_is_drawn_from_the_next_pass() {
        // In berth 1, below the carrier's 2, the loop has passed it.
        let (catalog, mut session, carrier) = dying_carrier(2, 100, HALFWAY);
        launch_pass(&mut session, &catalog);
        let fighter = vipers(&session)[0].clone();
        assert_eq!(fighter.berth, 1);
        let from = session.space.npc(carrier).expect("dying").state.position;
        assert_eq!(fighter.state.position, from, "not moved");
        assert!(!session.draw_order().contains(&ShipRef::Npc(fighter.id)));
        let controls = crate::flight::Controls::default();
        let mut chance = Rolls::launching(0, 0);
        assert!(!session.game_work(controls, &catalog, &Keep, Rules::default(), &mut chance));
        assert!(session.draw_order().contains(&ShipRef::Npc(fighter.id)));
    }

    #[test]
    fn the_carriers_final_explosion_reaches_its_escape_fighter() {
        // `DeathDelay` 0: the step finds it dying and rolls (@0x3547d), and
        // its display then explodes it (@0x2d8ef-0x2da9c), its shockwave
        // reaching the fighter in berth 2, launched before that display
        // and not yet shown.
        let (catalog, mut session, carrier) = dying_carrier(1, 0, Condition::Intact);
        session.take_combat_events();
        launch_pass(&mut session, &catalog);
        assert!(session.space.npc(carrier).is_none(), "exploded");
        let fighter = vipers(&session)[0].id;
        let struck = session.take_combat_events().into_iter().any(|event| {
            matches!(
                event,
                CombatEvent::Hit { ship, weapon: None, late: false, .. }
                    if ship == ShipRef::Npc(fighter)
            )
        });
        assert!(struck);
    }

    #[test]
    fn a_dying_npcs_crew_pod_comes_out_in_its_turn_of_the_ship_loop() {
        // `_HandleShip`'s pod block (@0x352bf-0x352c9) reads the death
        // timer the pass began with, less its tick: 51 to 50, half its
        // `DeathDelay` of 100, where the first crew pod comes out.
        let starting = Condition::Dying {
            ticks_left: 51,
            starting: false,
        };
        let (catalog, mut session, carrier) = dying_carrier(1, 100, starting);
        let npc = session.space.npc_mut(carrier).expect("there");
        npc.hull.pod_count = 2;
        npc.pods_left = 2;
        session.take_combat_events();
        let mut chance = Rolls::launching(1, 0);
        let controls = crate::flight::Controls::default();
        assert!(!session.game_work(controls, &catalog, &Keep, Rules::default(), &mut chance));
        let ejected = session.take_combat_events().into_iter().any(|event| {
            matches!(event, CombatEvent::PodEjected { ship, .. } if ship == ShipRef::Npc(carrier))
        });
        assert!(ejected);
        assert_eq!(session.space.npc(carrier).map(|npc| npc.pods_left), Some(1));
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
        let id = session.space.traffic.add_npc(moving);
        crate::fixture::place(&mut session, id, Vec2::new(40.0, -80.0));
        let placed = session.space.npc(id).expect("there").state;
        assert_eq!(
            (placed.position, placed.velocity),
            (Vec2::new(40.0, -80.0), Vec2::ZERO)
        );
    }
}
