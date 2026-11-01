//! `_AIDispatch`'s loop, slot by slot ([`Session::dispatch_npcs`]): each
//! occupied slot's whole think, its turn in the fight and its fighters
//! put in the system, before the next slot's.
//!
//! `_DoPlayGameWork`'s loop over slots 1-63 rereads each slot's active
//! flag, AI type and system as it comes (@0x44270-0x442ae) and runs that
//! slot's `_AIDispatch` (@0x4429d), nothing else. So a slot reads what the
//! slots below it wrote this frame (a launched fighter, a new goal, target
//! or condition, a claim on the last free slot, a freed pod's slot) and
//! the slots above it as their last frame left them.
//!
//! `_LaunchFighter` marks the fighter's slot active as `_AIDispatch`'s
//! loop launches it (`_GenericRandomShipSpawn` @0x3c90b, @0x3d680), in its
//! carrier's system (@0x3d694), of AI type 5 (@0x3d771). So:
//!
//! - a fighter put in a slot above its carrier's is reached by the same
//!   loop in its launch frame: its `_AIDispatch` runs its whole AI, its
//!   fields on entry (mission mark -1 @0x3c9a9, fade 0 @0x3c9c7, +0x50 0
//!   @0x3d79d, substate and goal 0 @0x3d7a3/@0x3d7af) taking it past every
//!   gate to `_AIHandleCloaking` (@0x8fcaa), its type AI whatever the
//!   frame counter (@0x8fe20, @0x8fe25), `_EscortAI` (@0x90011-0x90016),
//!   `_HighLevelAIHandler` (@0x901e5) and `_LowLevelAIHandler` (@0x901f4),
//!   its point defence among it (`_AIFireTurret` →
//!   `_HandleShipPointDefense` @0x80b0d), and `_HandleShip` fires the
//!   pick it chose that frame (@0x34fce-0x34ffd). If it launches from a
//!   bay of its own it takes the next free slot ahead of any carrier
//!   above it;
//! - one put in a slot below (one freed earlier) has been passed, so it
//!   thinks first in the next frame, and fires nothing in its launch frame,
//!   as `_GenericRandomShipSpawn` clears its fire flag (@0x3cb60);
//! - whichever slot it took, every slot whose `_AIDispatch` comes after
//!   the launch can choose it, as a target (the hunt asks nothing of its
//!   goal or its frame, @0x8aa64-0x8aac8) or in its point defence's ship
//!   branch, which walks the live table (@0x394ce-0x3969d), once it
//!   fights the defender: it launches with goal 0 (`_AISetup` @0x7f3e8), so
//!   one above its carrier is a candidate after its launch-frame think,
//!   and one below only from its next frame's; a slot before the
//!   carrier's cannot.
//!
//! Every fighter moves and is displayed in its launch frame either way,
//! as the ship loop comes after the AI loop (@0x4445c-0x444c4). Nothing
//! here sounds: `_LaunchFighter` and `_GenericRandomShipSpawn` play
//! nothing, and the launch's own sound is `_AILaunchFighter`'s (@0x81466).
//! [`RuleKey::LaunchFrameAi`]'s other reading has every NPC decide in one
//! batch, from the tick as it began, and puts every fighter in the system
//! after the whole loop, neither thinking nor a candidate until the next
//! tick.
//!
//! An escape pod's wait runs at its own slot's turn too (`_AIDispatch`
//! @0x8fb62-0x8fbba), so its slot is free to the slots after it that
//! frame, and not to those before (`Traffic::think_run`). So do a ship's
//! own jump-out (@0x859b8) and a fighter's dock (@0x80404), the low
//! level's leaves (see the traffic's `dispatch_leave`): the slot is free
//! to a higher carrier's launch that frame, and a docked fighter goes
//! aboard at its slot ([`Session::dock_fighter`]), so a carrier above it
//! launches the round.

use std::collections::BTreeSet;

use super::{Session, Space, player_led};
use crate::ai::{Behaviour, fire};
use crate::chance::Chance;
use crate::combat::plunder::Plunderer;
use crate::combat::{GamePass, Rules, ShipRef, SlotShip};
use crate::rulebook::{RuleKey, RuleSource};
use crate::ship_layer::Slot;
use crate::traffic::npc::{Npc, NpcId};
use crate::traffic::plunder_dock::DockSub;

impl Session {
    /// `_AIDispatch`'s loop on `pass` by `rules`, the NPCs thinking as
    /// `behaviour` says, drawing on `chance` (see the module docs): for
    /// each occupied slot in ascending order, that slot's think
    /// ([`Traffic::think_run`](crate::traffic::Traffic::think_run)), the
    /// refuel it gave landed, its point defence and launches
    /// ([`Combat::defend_npcs`](crate::combat::Combat::defend_npcs)) and
    /// its fighters put in the system at once
    /// ([`Session::launch_fighters`]), a fighter that docked in its run
    /// taken aboard first ([`Session::dock_fighter`]; one with no bay
    /// stays out, [`Session::fighters_staying_out`], asked as the loop
    /// begins), and its work on the line settled
    /// ([`Session::settle_line_through`]): its calls, its scan, its
    /// plunder of the player and its refuel note, as the original says
    /// each inside its own `_AIDispatch`, so a higher slot's think reads
    /// the legal record a scan wrote and the reinforcement clock a call
    /// started. The next slot is read from the live
    /// table, so a fighter in a slot above is reached as its slot comes,
    /// and one below has been passed. On a pass that set a system up (not
    /// `thought`) no NPC thinks, and only a fighter launched in it does.
    /// By [`RuleKey::LaunchFrameAi`]'s other reading, one run of every
    /// slot, its fighters put in the system after it.
    ///
    /// Whatever session state a think reads must land each slot, as the
    /// fuel does here: `step_traffic` builds each run's world from the
    /// session, so it reads only what has been written back by then.
    ///
    /// What still settles outside its slot:
    ///
    /// - Within a slot the point defence comes after the whole think,
    ///   where `_AIFireTurret` runs in the low level. It is the ship's own
    ///   turn, so no other slot reads the difference.
    /// - The thinks' voice cues are answered after the loop, in berth
    ///   order ([`Space::answer_voice_cues`](super::Space::answer_voice_cues)).
    ///   They speak on the voice channel and raise no line, so no slot's
    ///   call reads them.
    ///
    /// The head of each ship's dispatch (its warp cancel, its jump's
    /// cancels, a leader's orders, the regroup draws and the disabled
    /// reset) runs at its own slot, inside its run
    /// (`nova_sim::traffic::dispatch_head`), and so do the high level's
    /// hold, turn-back and gate box, goal 0xb's block and its push of the
    /// leader, sub 0xd's step (`nova_sim::traffic::dispatch_follow`), a
    /// docking ship's low level, its match, creep, dock start and
    /// completion, a repair's line among them
    /// (`nova_sim::traffic::dispatch_dock`), a ship's own jump-out and a
    /// fighter's dock (`nova_sim::traffic::dispatch_leave`).
    pub(super) fn dispatch_npcs(
        &mut self,
        thought: bool,
        pass: GamePass,
        behaviour: &(impl Behaviour + ?Sized),
        rules: Rules,
        chance: &mut dyn Chance,
    ) {
        let by_slot = self.setup.rules.source_for(RuleKey::LaunchFrameAi) == RuleSource::Engine;
        let mut after: Option<u32> = None;
        let mut launched: BTreeSet<NpcId> = BTreeSet::new();
        // No bay comes or goes in the loop (see the `fighter_homes` module).
        let no_bay = self.fighters_staying_out();
        loop {
            let through = if by_slot {
                self.space.traffic.next_slot_after(after)
            } else {
                None
            };
            let in_run = |npc: &Npc| {
                after.is_none_or(|after| npc.berth > after)
                    && through.is_none_or(|through| npc.berth <= through)
                    && (thought || launched.contains(&npc.id))
            };
            let left = self.step_traffic(|traffic, world| {
                traffic.think_run(behaviour, world, in_run, &no_bay, chance)
            });
            // A fighter that docked at its slot goes aboard there
            // (`_AIAddFighterToParent` @0x803e6), so a carrier above it
            // launches the round.
            for (id, outcome) in left {
                self.dock_fighter(id, outcome);
            }
            // The fuel a refueller gave, so the next slot reads the tank
            // it left.
            self.space.take_refuel(&mut self.pilot);
            // A run whose triggers, as its thinks just wrote them, ask
            // nothing of the fight's loop launches and defends nothing:
            // only its notes are written, without the fighters' assembly.
            if self.space.dispatch_run_acts(after, through) {
                self.with_fighters(pass, rules, chance, |combat, fighters, arena, chance| {
                    let (after, through) = (after.map(Slot::Berth), through.map(Slot::Berth));
                    combat.defend_npcs(fighters, arena, after, through, chance);
                });
            } else {
                self.note_idle_run(pass);
            }
            let sorties = self.space.combat.take_sorties();
            launched.extend(self.launch_fighters(&sorties, chance));
            // The slot's work on the line, so the next slot's think reads
            // what it wrote.
            self.settle_line_through(through, rules.law, chance);
            let Some(slot) = through else {
                return;
            };
            after = Some(slot);
        }
    }

    /// What the fight's loop would leave of a run on `pass` in which no
    /// NPC acts ([`Space::dispatch_run_acts`]): the field, and each ship's
    /// slot record, the player's and every NPC's, as
    /// [`Combat::note_idle_run`](crate::combat::Combat::note_idle_run)
    /// writes them. A ship that leaves in a later run of the loop keeps
    /// the record it had here, as when the run is fought.
    pub(super) fn note_idle_run(&mut self, pass: GamePass) {
        let player = SlotShip {
            class: self.pilot.ship,
            plunderer: Plunderer {
                target: self.chart.target.map(ShipRef::Npc),
                ..Plunderer::default()
            },
            position: self.ship.player.position,
        };
        let npcs = self.space.traffic.npcs().iter().map(|npc| {
            let ship = SlotShip {
                class: npc.ship,
                plunderer: npc_plunderer(npc),
                position: npc.state.position,
            };
            (player_led::slot_of(npc), ship)
        });
        let ships: Vec<(Slot, SlotShip)> = std::iter::once((Slot::Player, player))
            .chain(npcs)
            .collect();
        let field = self.field();
        self.space.combat.note_idle_run(pass, field, ships);
    }
}

impl Space {
    /// Whether any NPC in the slots above `after` and up to `through`,
    /// each bound open with none, asks anything of the fight's loop
    /// ([`Trigger::acts_in_dispatch`](crate::combat::armament::Trigger::acts_in_dispatch)),
    /// its trigger as the fight takes it ([`fire::gated`]). The fight
    /// takes the same ships ([`Combat::defend_npcs`](crate::combat::Combat::defend_npcs)),
    /// whether or not they thought.
    pub(super) fn dispatch_run_acts(&self, after: Option<u32>, through: Option<u32>) -> bool {
        self.traffic.npcs().iter().any(|npc| {
            after.is_none_or(|after| npc.berth > after)
                && through.is_none_or(|through| npc.berth <= through)
                && fire::gated(npc).acts_in_dispatch()
        })
    }
}

/// `npc` as its shots' plunder reads it (see the fight's `plunder`): an
/// NPC, plundering in goal 0xd, docked with its victim in goal 4 with
/// sub 0xf (`_AIShipIsPlunderingShip` @0x83f6e), and its target.
pub(super) fn npc_plunderer(npc: &Npc) -> Plunderer {
    Plunderer {
        npc: true,
        plundering: npc.goal.plundering().is_some(),
        docked: npc.goal.attacking().is_some()
            && npc
                .docking
                .is_some_and(|docking| docking.sub == DockSub::Dock),
        target: npc.target,
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use crate::ai::pursuit::Pursuit;
    use crate::ai::{Behaviour, CommCall, Decision, Goal, Surroundings};
    use crate::catalog::{HullRecord, StockWeapon, WeaponId, WeaponRecord};
    use crate::chance::{Chance, NeverFires};
    use crate::combat::armament::{Armament, LauncherRounds, Trigger};
    use crate::combat::weapon::WeaponSpec;
    use crate::combat::{CombatEvent, GamePass, Rules, ShipRef};
    use crate::geometry::Vec2;
    use crate::message::SimMessage;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::session::Session;
    use crate::session::fighters::tests::{
        BAY, GUN, QUAD, VIPER, armed_npc, bay, carrier_npc, carrying, flying_by, game_pass, pilot,
    };
    use crate::sound::SimSound;
    use crate::testkit::{FakePilotCatalog, weapon};
    use crate::traffic::npc::{Npc, NpcId};

    /// The Quad Light Blaster Turret's point defence: 20 pixels a tick
    /// for 12, every 5 ticks.
    fn quad() -> WeaponRecord {
        WeaponRecord {
            guidance: 9,
            reload: 5,
            count: 12,
            speed: 2000,
            mass_dmg: 1,
            energy_dmg: 4,
            ..weapon(133)
        }
    }

    /// [`carrying`], the Viper of a class point defence fires on
    /// (`Flags2` 0x0008), carrying its gun and the Quad.
    fn catalog() -> FakePilotCatalog {
        let mut catalog = carrying();
        catalog.weapons.push(quad());
        let viper = catalog
            .hulls
            .iter_mut()
            .find(|hull| hull.id == VIPER)
            .expect("the Viper's");
        *viper = HullRecord {
            flags2: 0x0008,
            weapons: vec![
                StockWeapon {
                    weapon: GUN,
                    count: 1,
                    ammo: 0,
                },
                StockWeapon {
                    weapon: QUAD,
                    count: 1,
                    ammo: 0,
                },
            ],
            ..viper.clone()
        };
        catalog
    }

    /// Keeps every goal and target but a Viper's, which attacks its
    /// target and ends its glide, as `_EscortAI`'s Attack block does under
    /// a fighting carrier (+0x4c = -1.0, @0x83d62); the carrier asks for
    /// its goal-4 launch (and the snipe's after it when `twice`), each
    /// defender for its point defence, and every Viper for `fighter`.
    #[derive(Debug)]
    struct Scripted {
        carrier: NpcId,
        twice: bool,
        defenders: Vec<NpcId>,
        fighter: Trigger,
    }

    impl Behaviour for Scripted {
        fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
            self.decision(npc, around, chance).goal
        }

        fn decision(
            &self,
            npc: &Npc,
            _around: &Surroundings,
            _chance: &mut dyn Chance,
        ) -> Decision {
            let viper = npc.ship == VIPER;
            Decision {
                goal: match npc.target {
                    Some(target) if viper => Goal::Attack(target),
                    _ => npc.goal,
                },
                ends_wait: viper,
                ..Decision::default()
            }
        }

        fn trigger(&self, npc: &Npc, _around: &Surroundings) -> Trigger {
            if npc.id == self.carrier {
                Trigger {
                    bays: true,
                    bays_after: self.twice,
                    ..Trigger::default()
                }
            } else if self.defenders.contains(&npc.id) {
                Trigger {
                    point_defence: true,
                    ..Trigger::default()
                }
            } else if npc.ship == VIPER {
                self.fighter
            } else {
                Trigger::default()
            }
        }

        fn target(&self, npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
            npc.target
        }
    }

    /// A session by `source`'s reading of [`RuleKey::LaunchFrameAi`].
    fn session(catalog: &FakePilotCatalog, source: RuleSource) -> Session {
        let rules = Rulebook::default()
            .with_override(RuleKey::FighterLaunch, RuleSource::Engine)
            .with_override(RuleKey::LaunchFrameAi, source);
        let session = flying_by(catalog, pilot(catalog, 0, Vec::new()), rules);
        assert_eq!(session.npcs(), [], "an empty system");
        session
    }

    /// Adds, at `berth`, a pirate carrier near (1000, 1000), leading
    /// itself, attacking `target`, with 2 Vipers aboard.
    fn carrier_at(session: &mut Session, berth: u32, target: NpcId) -> NpcId {
        let mut npc = carrier_npc(NpcId(0), 2);
        npc.leader = None;
        npc.state.position = Vec2::new(1000.0, 1000.0);
        npc.goal = Goal::Attack(ShipRef::Npc(target));
        npc.target = Some(ShipRef::Npc(target));
        session.space.traffic.add_npc_at(npc, berth)
    }

    /// The Vipers in the system, lowest berth first.
    fn vipers(session: &Session) -> Vec<Npc> {
        let mut vipers: Vec<Npc> = session
            .npcs()
            .iter()
            .filter(|npc| npc.ship == VIPER)
            .cloned()
            .collect();
        vipers.sort_by_key(|npc| npc.berth);
        vipers
    }

    /// The ships heard firing `weapon` among `events`.
    fn firing(events: &[CombatEvent], weapon: WeaponId) -> Vec<ShipRef> {
        events
            .iter()
            .filter_map(|event| match *event {
                CombatEvent::Fired {
                    ship, weapon: w, ..
                } if w == weapon => Some(ship),
                _ => None,
            })
            .collect()
    }

    /// A raid at a pirate carrier: the raider, of a class point defence
    /// fires on, attacks the carrier from 100 pixels off; the carrier, at
    /// `carrier_berth`, attacks it back and launches; the raider sits at
    /// berth 20. The session, the carrier and the raider.
    fn raid(
        catalog: &FakePilotCatalog,
        source: RuleSource,
        carrier_berth: u32,
    ) -> (Session, NpcId, NpcId) {
        raid_at(catalog, source, carrier_berth, 20)
    }

    /// [`raid`], the raider at `raider_berth`.
    fn raid_at(
        catalog: &FakePilotCatalog,
        source: RuleSource,
        carrier_berth: u32,
        raider_berth: u32,
    ) -> (Session, NpcId, NpcId) {
        let mut session = session(catalog, source);
        let mut raider = crate::testkit::npc(0, crate::stats::ShipStats::default());
        raider.state.position = Vec2::new(1000.0, 1100.0);
        raider.hull.pd_target = true;
        let raider = session.space.traffic.add_npc_at(raider, raider_berth);
        let carrier = carrier_at(&mut session, carrier_berth, raider);
        let npc = session.space.npc_mut(raider).expect("there");
        npc.goal = Goal::Attack(ShipRef::Npc(carrier));
        npc.target = Some(ShipRef::Npc(carrier));
        (session, carrier, raider)
    }

    /// Point defence asked for alone.
    const DEFENDING: Trigger = Trigger {
        primary: false,
        secondary: None,
        only: None,
        bays: false,
        bays_after: false,
        point_defence: true,
        keep: None,
    };

    /// Over two passes of [`raid`] whose carrier sits at `carrier_berth`,
    /// its Viper defending: the Viper's berth, and whether it fired its
    /// point defence on each pass.
    fn defends(source: RuleSource, carrier_berth: u32) -> (u32, [bool; 2]) {
        let catalog = catalog();
        let (mut session, carrier, _) = raid(&catalog, source, carrier_berth);
        let script = Scripted {
            carrier,
            twice: false,
            defenders: Vec::new(),
            fighter: DEFENDING,
        };
        let mut fired = [false; 2];
        for heard in &mut fired {
            game_pass(&mut session, &catalog, &script);
            let launched = vipers(&session);
            assert_eq!(launched.len(), 1, "one launch");
            let events = session.take_combat_events();
            *heard = firing(&events, QUAD).contains(&ShipRef::Npc(launched[0].id));
        }
        (vipers(&session)[0].berth, fired)
    }

    #[test]
    fn a_fighter_launched_above_its_carrier_fires_its_point_defence_on_its_launch_tick() {
        // Its slot comes in the same `_AIDispatch` loop (@0x44270-0x442ae)
        // after its carrier's launch made it active (@0x3c90b): it thinks
        // and its `_AIFireTurret` (@0x80b0d) fires at the raider attacking
        // its carrier, its leader (@0x394ce-0x3969d).
        // The Quad then reloads through the next tick.
        assert_eq!(defends(RuleSource::Engine, 1), (2, [true, false]));
    }

    #[test]
    fn a_fighter_launched_below_its_carrier_first_fires_its_point_defence_the_next_tick() {
        // Slot 1, free, was passed before slot 5 launched.
        assert_eq!(defends(RuleSource::Engine, 5), (1, [false, true]));
    }

    #[test]
    fn by_the_other_reading_a_fighter_above_its_carrier_waits_for_the_next_tick() {
        assert_eq!(defends(RuleSource::Bible, 1), (2, [false, true]));
    }

    /// Fires the gun as its pick.
    const GUNNING: Trigger = Trigger {
        primary: false,
        secondary: None,
        only: Some(GUN),
        bays: false,
        bays_after: false,
        point_defence: false,
        keep: None,
    };

    /// One pass of [`raid`] whose carrier sits at `carrier_berth`, its
    /// Viper firing its gun: the Viper, and whether it fired its gun.
    fn picks(source: RuleSource, carrier_berth: u32) -> (Npc, bool) {
        let catalog = catalog();
        let (mut session, carrier, _) = raid(&catalog, source, carrier_berth);
        let script = Scripted {
            carrier,
            twice: false,
            defenders: Vec::new(),
            fighter: GUNNING,
        };
        game_pass(&mut session, &catalog, &script);
        let viper = vipers(&session).remove(0);
        let fired = firing(&session.take_combat_events(), GUN).contains(&ShipRef::Npc(viper.id));
        (viper, fired)
    }

    #[test]
    fn a_fighter_launched_above_its_carrier_fires_its_pick_and_pursues_on_its_launch_tick() {
        // `_HandleShip` fires the pick +0x72 while the fire flag is set
        // (@0x34fce-0x34ffd), which its think set; its low level ran
        // (@0x901f4) before the ship loop moves it (@0x4445c).
        let (viper, fired) = picks(RuleSource::Engine, 1);
        assert_eq!(viper.berth, 2);
        assert!(fired, "its gun");
        assert_ne!(viper.pursuit, Pursuit::default(), "its pursuit");
    }

    #[test]
    fn a_fighter_launched_into_a_wrecks_slot_takes_the_earlier_slots_aim() {
        // A launch takes the first free slot in the AI loop
        // (@0x3c8c6-0x3c8e9) and leaves every other ship's +0x70, so the
        // earlier slot's test finds its target active again: the fighter,
        // friend or not. An earlier slot whose high level runs has let go
        // at its slot test (@0x8d4e4-0x8d50d), before the launch; one in
        // its AI wait skips it (@0x8d4b9-0x8d4d2) and still holds it.
        let catalog = catalog();
        let (mut session, carrier, _) = raid(&catalog, RuleSource::Engine, 5);
        let plain = || crate::testkit::npc(0, crate::stats::ShipStats::default());
        let wreck = session.space.traffic.add_npc_at(plain(), 3);
        let aim = ShipRef::Npc(wreck);
        let holder = |ai_wait| {
            let mut holding = plain();
            holding.goal = Goal::Attack(aim);
            holding.target = Some(aim);
            holding.ai_wait = ai_wait;
            holding
        };
        let holding = session.space.traffic.add_npc_at(holder(50), 1);
        let thinking = session.space.traffic.add_npc_at(holder(0), 2);
        session.space.traffic.remove_destroyed(wreck);
        let script = Scripted {
            carrier,
            twice: false,
            defenders: Vec::new(),
            fighter: Trigger::default(),
        };
        game_pass(&mut session, &catalog, &script);
        let viper = vipers(&session).remove(0);
        assert_eq!(viper.berth, 3, "the wreck's slot");
        let npc = session.space.npc(holding).expect("there");
        assert_eq!(npc.target, Some(ShipRef::Npc(viper.id)));
        assert_eq!(npc.goal, Goal::Attack(ShipRef::Npc(viper.id)));
        let npc = session.space.npc(thinking).expect("there");
        assert_ne!(npc.target, Some(ShipRef::Npc(viper.id)), "let go first");
    }

    #[test]
    fn a_fighter_launched_below_its_carrier_fires_nothing_and_has_no_pursuit_on_its_launch_tick() {
        // `_GenericRandomShipSpawn` clears its fire flag (@0x3cb60).
        let (viper, fired) = picks(RuleSource::Engine, 5);
        assert_eq!(viper.berth, 1);
        assert!(!fired);
        assert_eq!(viper.pursuit, Pursuit::default());
        let (viper, fired) = picks(RuleSource::Bible, 1);
        assert_eq!(viper.berth, 2);
        assert!(!fired, "by the other reading");
        assert_eq!(viper.pursuit, Pursuit::default());
    }

    /// The defenders' berths and the carrier's: the low defender is the
    /// carrier's target and the high one's leader.
    struct Berths {
        low: u32,
        carrier: u32,
        high: u32,
    }

    /// Over two passes, with point defence of the Quad on the low and
    /// high defenders near (1000, 1000) by `source`, and a carrier
    /// launching a Viper at the low one: the Viper's berth, and on each
    /// pass which defenders fired at it. A plain ship asking nothing of
    /// the fight sits at each of `idle`.
    fn defended_against(
        source: RuleSource,
        berths: &Berths,
        idle: &[u32],
    ) -> (u32, [Vec<&'static str>; 2]) {
        let catalog = catalog();
        let mut session = session(&catalog, source);
        for &berth in idle {
            let mut plain = crate::testkit::npc(0, crate::stats::ShipStats::default());
            plain.state.position = Vec2::new(-1000.0, -1000.0);
            session.space.traffic.add_npc_at(plain, berth);
        }
        let low = session
            .space
            .traffic
            .add_npc_at(armed_npc(1000.0, 1100.0, quad()), berths.low);
        let mut wing = armed_npc(1030.0, 1100.0, quad());
        wing.leader = Some(low);
        let high = session.space.traffic.add_npc_at(wing, berths.high);
        let carrier = carrier_at(&mut session, berths.carrier, low);
        let script = Scripted {
            carrier,
            twice: false,
            defenders: vec![low, high],
            fighter: Trigger::default(),
        };
        let mut fired = [Vec::new(), Vec::new()];
        for heard in &mut fired {
            game_pass(&mut session, &catalog, &script);
            let events = session.take_combat_events();
            *heard = [(low, "low"), (high, "high")]
                .into_iter()
                .filter(|&(defender, _)| firing(&events, QUAD).contains(&ShipRef::Npc(defender)))
                .map(|(_, name)| name)
                .collect();
        }
        (vipers(&session)[0].berth, fired)
    }

    #[test]
    fn a_later_slots_point_defence_fires_at_a_fighter_on_its_launch_tick_above_or_below() {
        // `_HandleShipPointDefense`'s ship branch walks the live table
        // (@0x394ce-0x3969d): the high defender's turn comes after the
        // launch; the low one's came before it.
        let above = Berths {
            low: 1,
            carrier: 2,
            high: 6,
        };
        let (berth, fired) = defended_against(RuleSource::Engine, &above, &[]);
        assert_eq!(berth, 3);
        // The low defender first fires the next tick, the high's Quad
        // reloading then.
        assert_eq!(fired, [["high"], ["low"]]);
        let below = Berths {
            low: 2,
            carrier: 4,
            high: 6,
        };
        let (berth, fired) = defended_against(RuleSource::Engine, &below, &[]);
        assert_eq!(berth, 1);
        // Below its carrier it thinks first the next tick: it launched with
        // goal 0 (`_AISetup` @0x7f3e8), fighting no one, so neither
        // defender fires at it until then.
        assert_eq!(fired, [vec![], vec!["low", "high"]]);
    }

    #[test]
    fn by_the_other_reading_no_point_defence_sees_a_fighter_on_its_launch_tick() {
        let above = Berths {
            low: 1,
            carrier: 2,
            high: 6,
        };
        let (_, [launch, next]) = defended_against(RuleSource::Bible, &above, &[]);
        assert_eq!(launch, Vec::<&str>::new());
        assert_eq!(next, ["low", "high"]);
    }

    /// [`Scripted`], every Viper and the second's ship calling for help as
    /// they decide.
    #[derive(Debug)]
    struct CallingVipers(Scripted, Option<NpcId>);

    impl Behaviour for CallingVipers {
        fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
            self.0.decide(npc, around, chance)
        }

        fn decision(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Decision {
            let calls = npc.ship == VIPER || Some(npc.id) == self.1;
            Decision {
                comm_call: calls.then_some(CommCall::Distress),
                ..self.0.decision(npc, around, chance)
            }
        }

        fn trigger(&self, npc: &Npc, around: &Surroundings) -> Trigger {
            self.0.trigger(npc, around)
        }

        fn target(&self, npc: &Npc, around: &Surroundings) -> Option<ShipRef> {
            self.0.target(npc, around)
        }
    }

    /// One pass of [`raid`] whose carrier sits at `carrier_berth`, its
    /// Viper calling for help: whether the call was said, with its sound.
    fn called(source: RuleSource, carrier_berth: u32) -> bool {
        let catalog = catalog();
        let (mut session, carrier, _) = raid(&catalog, source, carrier_berth);
        let _ = (session.take_messages(), session.take_sounds());
        let script = CallingVipers(
            Scripted {
                carrier,
                twice: false,
                defenders: Vec::new(),
                fighter: Trigger::default(),
            },
            None,
        );
        game_pass(&mut session, &catalog, &script);
        assert_eq!(vipers(&session).len(), 1, "one launch");
        let said = session
            .take_messages()
            .iter()
            .any(|message| matches!(message, SimMessage::CallForHelp(_)));
        let sounded = session.take_sounds().contains(&SimSound::CallForHelp);
        assert_eq!(said, sounded, "the call and its sound together");
        said
    }

    #[test]
    fn a_fighter_launched_above_its_carrier_has_its_call_answered_on_its_launch_tick() {
        // Its `_AIDispatch` makes the call in its launch frame
        // (`_AICallForHelp` @0x82ffb), before the frame is drawn.
        assert!(called(RuleSource::Engine, 1));
        assert!(!called(RuleSource::Engine, 5), "below: it thinks next tick");
        assert!(!called(RuleSource::Bible, 1), "by the other reading");
    }

    /// One pass of [`raid`] whose carrier at slot 1 launches a calling
    /// Viper, with a calling bystander at `caller_berth`: the Viper's
    /// berth, and the calls said.
    fn calls_said(caller_berth: u32) -> (u32, Vec<String>) {
        let catalog = catalog();
        let (mut session, carrier, _) = raid(&catalog, RuleSource::Engine, 1);
        let caller = session.space.traffic.add_npc_at(bystander(), caller_berth);
        let _ = (session.take_messages(), session.take_sounds());
        let script = CallingVipers(
            Scripted {
                carrier,
                twice: false,
                defenders: Vec::new(),
                fighter: Trigger::default(),
            },
            Some(caller),
        );
        game_pass(&mut session, &catalog, &script);
        let said = session
            .take_messages()
            .into_iter()
            .filter_map(|message| match message {
                SimMessage::CallForHelp(text) => Some(text),
                _ => None,
            })
            .collect();
        (vipers(&session)[0].berth, said)
    }

    #[test]
    fn a_launched_fighters_call_keeps_a_higher_slots_off_the_line() {
        // `_DisplayComm` sets `_commCount` at once (@0x9024b), so the
        // Viper's call at slot 2 keeps slot 3's off (@0x8308c).
        let (berth, said) = calls_said(3);
        assert_eq!(berth, 2);
        assert_eq!(said.len(), 1, "{said:?}");
        assert!(said[0].starts_with("Ship 144"), "{said:?}");
    }

    #[test]
    fn a_lower_slots_call_keeps_a_launched_fighters_off_the_line() {
        let (berth, said) = calls_said(2);
        assert_eq!(berth, 3);
        assert_eq!(said.len(), 1, "{said:?}");
        assert!(!said[0].starts_with("Ship 144"), "{said:?}");
    }

    #[test]
    fn a_pass_with_no_think_thinks_only_a_launched_fighter() {
        // A pass that set a system up thinks no NPC: the carrier launches
        // from the trigger it kept, and the fighter above it thinks.
        let catalog = catalog();
        let (mut session, carrier, raider) = raid(&catalog, RuleSource::Engine, 1);
        session.space.npc_mut(carrier).expect("there").trigger = Trigger {
            bays: true,
            ..Trigger::default()
        };
        let order = Order::of(carrier);
        session.dispatch_npcs(
            false,
            GamePass::Advance { double_time: false },
            &order,
            Rules::default(),
            &mut NeverFires,
        );
        let launched = vipers(&session);
        assert_eq!(launched.len(), 1, "launched");
        assert_eq!(launched[0].berth, 2);
        assert_eq!(order.thought.into_inner(), [launched[0].id]);
        assert_eq!(
            session.space.npc(raider).expect("there").goal,
            Goal::Attack(ShipRef::Npc(carrier))
        );
    }

    #[test]
    fn two_fighters_one_carrier_launches_in_a_tick_each_think_above_it() {
        // The snipe's second `_AILaunchFighter` (@0x8732f) on a bay of
        // `Reload` 0: two fighters, in slots 2 and 3, both reached after
        // slot 1's turn.
        let catalog = catalog();
        let (mut session, carrier, _) = raid(&catalog, RuleSource::Engine, 1);
        let npc = session.space.npc_mut(carrier).expect("there");
        npc.armament = Armament::new([(WeaponSpec::new(&WeaponRecord { reload: 0, ..bay() }), 1)]);
        npc.rounds = LauncherRounds::from([(BAY, 3)]);
        let script = Scripted {
            carrier,
            twice: true,
            defenders: Vec::new(),
            fighter: DEFENDING,
        };
        game_pass(&mut session, &catalog, &script);
        let launched = vipers(&session);
        let berths: Vec<u32> = launched.iter().map(|npc| npc.berth).collect();
        assert_eq!(berths, [2, 3]);
        let heard = firing(&session.take_combat_events(), QUAD);
        for viper in &launched {
            assert!(heard.contains(&ShipRef::Npc(viper.id)), "{}", viper.berth);
        }
    }

    // The think at its slot's turn: `_AIDispatch` runs each slot's whole AI
    // in turn (@0x44270-0x442ae), so a slot reads what the slots below it
    // wrote this frame, and the slots above it as their last frame left
    // them. These drive hand-written `Behaviour`s: what the real picks do
    // with what they read is `nova_sim_ai`'s to pin.

    /// [`Scripted`], but the `watcher` attacks the first Viper it reads in
    /// the system, a ship the hunt can choose however new
    /// (@0x8aa64-0x8aac8).
    #[derive(Debug)]
    struct Watching {
        script: Scripted,
        watcher: NpcId,
    }

    impl Behaviour for Watching {
        fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
            self.decision(npc, around, chance).goal
        }

        fn decision(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Decision {
            let seen = around.npcs.iter().find(|other| other.ship == VIPER);
            match seen {
                Some(viper) if npc.id == self.watcher => Decision {
                    goal: Goal::Attack(ShipRef::Npc(viper.id)),
                    ..Decision::default()
                },
                _ => self.script.decision(npc, around, chance),
            }
        }

        fn trigger(&self, npc: &Npc, around: &Surroundings) -> Trigger {
            self.script.trigger(npc, around)
        }

        fn target(&self, npc: &Npc, around: &Surroundings) -> Option<ShipRef> {
            self.script.target(npc, around)
        }
    }

    /// A ship near the fight, keeping its goal and doing nothing.
    fn bystander() -> Npc {
        let mut npc = crate::testkit::npc(0, crate::stats::ShipStats::default());
        npc.state.position = Vec2::new(1000.0, 1200.0);
        npc
    }

    /// One pass and another of [`raid`] whose carrier, at `carrier_berth`,
    /// launches with a [`Watching`] ship at `watcher_berth`: the Viper,
    /// and whether the watcher attacked it after each pass.
    fn watched(source: RuleSource, carrier_berth: u32, watcher_berth: u32) -> (Npc, [bool; 2]) {
        let catalog = catalog();
        let (mut session, carrier, _) = raid(&catalog, source, carrier_berth);
        let watcher = session.space.traffic.add_npc_at(bystander(), watcher_berth);
        let script = Watching {
            script: Scripted {
                carrier,
                twice: false,
                defenders: Vec::new(),
                fighter: Trigger::default(),
            },
            watcher,
        };
        let mut seen = [false; 2];
        for attacked in &mut seen {
            game_pass(&mut session, &catalog, &script);
            let viper = vipers(&session)[0].id;
            *attacked = session.space.npc(watcher).expect("there").goal
                == Goal::Attack(ShipRef::Npc(viper));
        }
        (vipers(&session).remove(0), seen)
    }

    #[test]
    fn an_existing_npc_above_a_carrier_can_target_its_fighter_on_the_launch_tick() {
        // The launch at slot 3 marks the fighter's slot active at once
        // (@0x3c90b), and slot 6's `_AIDispatch` comes after it.
        let (viper, seen) = watched(RuleSource::Engine, 3, 6);
        assert_eq!(viper.berth, 1);
        assert_eq!(seen, [true, true]);
    }

    #[test]
    fn an_npc_below_the_carrier_sees_its_fighter_only_the_next_tick() {
        let (viper, seen) = watched(RuleSource::Engine, 3, 2);
        assert_eq!(viper.berth, 1);
        assert_eq!(seen, [false, true]);
    }

    #[test]
    fn by_the_other_reading_no_npc_sees_a_fighter_on_its_launch_tick() {
        let (_, seen) = watched(RuleSource::Bible, 3, 6);
        assert_eq!(seen, [false, true]);
    }

    /// What one NPC's think read of another, which is `other`.
    type Read = (NpcId, Goal, Option<ShipRef>);

    /// Each thinking NPC among `movers` writes `goal` and `target`,
    /// recording what it read of the other movers; the carrier launches.
    #[derive(Debug)]
    struct Reading {
        movers: Vec<NpcId>,
        goal: Goal,
        target: ShipRef,
        reads: std::cell::RefCell<Vec<(NpcId, Read)>>,
        script: Scripted,
    }

    impl Behaviour for Reading {
        fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
            self.decision(npc, around, chance).goal
        }

        fn decision(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Decision {
            if !self.movers.contains(&npc.id) {
                return self.script.decision(npc, around, chance);
            }
            for other in around.npcs.iter().filter(|other| other.id != npc.id) {
                if self.movers.contains(&other.id) {
                    let read = (other.id, other.goal, other.target);
                    self.reads.borrow_mut().push((npc.id, read));
                }
            }
            Decision {
                goal: self.goal,
                ..Decision::default()
            }
        }

        fn trigger(&self, npc: &Npc, around: &Surroundings) -> Trigger {
            self.script.trigger(npc, around)
        }

        fn target(&self, npc: &Npc, around: &Surroundings) -> Option<ShipRef> {
            if self.movers.contains(&npc.id) {
                Some(self.target)
            } else {
                self.script.target(npc, around)
            }
        }
    }

    /// One pass with movers ([`Reading`]) at slots 2 and 5, and when
    /// `bay_between` a carrier launching at slot 3, by `source`: what the
    /// lower mover read of the higher, what the higher read of the
    /// lower, each its goal and target, and what they both wrote.
    fn read_by(source: RuleSource, bay_between: bool) -> [(Goal, Option<ShipRef>); 3] {
        let catalog = catalog();
        let (mut session, carrier, raider) = raid(&catalog, source, 7);
        let low = session.space.traffic.add_npc_at(bystander(), 2);
        let high = session.space.traffic.add_npc_at(bystander(), 5);
        let launcher = if bay_between {
            carrier_at(&mut session, 3, raider)
        } else {
            NpcId(u32::MAX)
        };
        let (quarry, written) = (ShipRef::Npc(raider), Goal::Attack(ShipRef::Npc(carrier)));
        let reading = Reading {
            movers: vec![low, high],
            goal: written,
            target: quarry,
            reads: std::cell::RefCell::default(),
            script: Scripted {
                carrier: launcher,
                twice: false,
                defenders: Vec::new(),
                fighter: Trigger::default(),
            },
        };
        game_pass(&mut session, &catalog, &reading);
        assert_eq!(!vipers(&session).is_empty(), bay_between, "the launch");
        let reads = reading.reads.into_inner();
        let of = |reader: NpcId| {
            reads
                .iter()
                .find(|(by, _)| *by == reader)
                .map(|&(_, (_, goal, target))| (goal, target))
                .expect("it read")
        };
        [of(low), of(high), (written, Some(quarry))]
    }

    #[test]
    fn a_slot_reads_what_the_slots_below_it_wrote_this_tick() {
        // Slot 5's `_AIDispatch` comes after slot 2's, wherever a ready bay
        // falls: goals and targets are written in the think, and read in
        // the next slot's.
        for bay_between in [false, true] {
            let [lower, higher, written] = read_by(RuleSource::Engine, bay_between);
            assert_eq!(lower, (Goal::Idle, None), "{bay_between}: as it was");
            assert_eq!(higher, written, "{bay_between}: slot 2's new ones");
        }
    }

    #[test]
    fn by_the_other_reading_every_npc_decides_from_the_tick_as_it_began() {
        for bay_between in [false, true] {
            let [lower, higher, _] = read_by(RuleSource::Bible, bay_between);
            assert_eq!(lower, (Goal::Idle, None), "{bay_between}");
            assert_eq!(higher, (Goal::Idle, None), "{bay_between}");
        }
    }

    /// Records what each Viper read of the ship at slot `high`, which
    /// writes a new goal; otherwise [`Scripted`].
    #[derive(Debug)]
    struct ReadingUp {
        high: NpcId,
        read: std::cell::RefCell<Vec<Goal>>,
        script: Scripted,
    }

    impl Behaviour for ReadingUp {
        fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
            self.decision(npc, around, chance).goal
        }

        fn decision(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Decision {
            if npc.id == self.high {
                return Decision {
                    goal: Goal::JumpOut,
                    ..Decision::default()
                };
            }
            if npc.ship == VIPER {
                let high = around.npc(self.high).expect("there");
                self.read.borrow_mut().push(high.goal);
            }
            self.script.decision(npc, around, chance)
        }

        fn trigger(&self, npc: &Npc, around: &Surroundings) -> Trigger {
            self.script.trigger(npc, around)
        }

        fn target(&self, npc: &Npc, around: &Surroundings) -> Option<ShipRef> {
            self.script.target(npc, around)
        }
    }

    #[test]
    fn a_launched_fighter_reads_a_higher_slot_as_its_last_frame_left_it() {
        // The fighter at slot 2 runs before slot 6's `_AIDispatch`.
        let catalog = catalog();
        let (mut session, carrier, _) = raid(&catalog, RuleSource::Engine, 1);
        let high = session.space.traffic.add_npc_at(bystander(), 6);
        let reading = ReadingUp {
            high,
            read: std::cell::RefCell::default(),
            script: Scripted {
                carrier,
                twice: false,
                defenders: Vec::new(),
                fighter: Trigger::default(),
            },
        };
        game_pass(&mut session, &catalog, &reading);
        assert_eq!(vipers(&session)[0].berth, 2);
        assert_eq!(session.space.npc(high).expect("there").goal, Goal::JumpOut);
        assert_eq!(reading.read.into_inner(), [Goal::Idle]);
    }

    /// Records each NPC as it decides; otherwise [`Scripted`].
    #[derive(Debug)]
    struct Order {
        thought: std::cell::RefCell<Vec<NpcId>>,
        script: Scripted,
    }

    impl Order {
        /// Recording as [`Scripted`] launches from `carrier`.
        fn of(carrier: NpcId) -> Self {
            Self {
                thought: std::cell::RefCell::default(),
                script: Scripted {
                    carrier,
                    twice: false,
                    defenders: Vec::new(),
                    fighter: Trigger::default(),
                },
            }
        }
    }

    impl Behaviour for Order {
        fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
            self.decision(npc, around, chance).goal
        }

        fn decision(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Decision {
            self.thought.borrow_mut().push(npc.id);
            self.script.decision(npc, around, chance)
        }

        fn trigger(&self, npc: &Npc, around: &Surroundings) -> Trigger {
            self.script.trigger(npc, around)
        }

        fn target(&self, npc: &Npc, around: &Surroundings) -> Option<ShipRef> {
            self.script.target(npc, around)
        }
    }

    #[test]
    fn every_npc_thinks_once_in_slot_order() {
        // The launcher at slot 1, its fighter reached at slot 2; a ready
        // carrier launching nothing at slot 5; added out of slot order.
        let catalog = catalog();
        let (mut session, carrier, raider) = raid(&catalog, RuleSource::Engine, 1);
        let ready = carrier_at(&mut session, 5, raider);
        let third = session.space.traffic.add_npc_at(bystander(), 3);
        let order = Order::of(carrier);
        game_pass(&mut session, &catalog, &order);
        let viper = vipers(&session)[0].clone();
        assert_eq!(viper.berth, 2);
        assert_eq!(
            order.thought.into_inner(),
            [carrier, viper.id, third, ready, raider]
        );
    }

    /// [`catalog`], the Viper's hull carrying a bay of two Vipers.
    fn fighters_carrying_a_bay() -> FakePilotCatalog {
        let mut catalog = catalog();
        catalog
            .hulls
            .iter_mut()
            .find(|hull| hull.id == VIPER)
            .expect("the Viper's")
            .weapons
            .push(StockWeapon {
                weapon: BAY,
                count: 1,
                ammo: 2,
            });
        catalog
    }

    /// Keeps every goal and target, ends a Viper's glide, and asks every
    /// ship for the goal-4 launch.
    #[derive(Debug)]
    struct EveryBay;

    impl Behaviour for EveryBay {
        fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            npc.goal
        }

        fn decision(
            &self,
            npc: &Npc,
            _around: &Surroundings,
            _chance: &mut dyn Chance,
        ) -> Decision {
            Decision {
                goal: npc.goal,
                ends_wait: npc.ship == VIPER,
                ..Decision::default()
            }
        }

        fn trigger(&self, _npc: &Npc, _around: &Surroundings) -> Trigger {
            Trigger {
                bays: true,
                ..Trigger::default()
            }
        }

        fn target(&self, npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
            npc.target
        }
    }

    /// Puts [`bystander`]s in `berths`.
    fn fill(session: &mut Session, berths: impl IntoIterator<Item = u32>) {
        for berth in berths {
            session.space.traffic.add_npc_at(bystander(), berth);
        }
    }

    /// A pass of carriers at slots 1 and 10 by `source`, slots 2 and 3 all
    /// that's free: the leaders of the ships at slots 2 and 3, the high
    /// carrier, and whether it was heard launching.
    fn at_the_last_slot(source: RuleSource) -> ([Option<NpcId>; 2], Npc, NpcId, bool) {
        let catalog = fighters_carrying_a_bay();
        let mut session = session(&catalog, source);
        let (low, high) = (
            carrier_at(&mut session, 1, NpcId(0)),
            carrier_at(&mut session, 10, NpcId(0)),
        );
        let quarry = ShipRef::Npc(low);
        session.space.npc_mut(high).expect("there").target = Some(quarry);
        session.space.npc_mut(high).expect("there").goal = Goal::Attack(quarry);
        let quarry = ShipRef::Npc(high);
        session.space.npc_mut(low).expect("there").target = Some(quarry);
        session.space.npc_mut(low).expect("there").goal = Goal::Attack(quarry);
        fill(&mut session, (4..=55).filter(|&berth| berth != 10));
        game_pass(&mut session, &catalog, &EveryBay);
        let leader_at = |berth: u32| {
            session
                .npcs()
                .iter()
                .find(|npc| npc.berth == berth)
                .and_then(|npc| npc.leader)
        };
        let leaders = [leader_at(2), leader_at(3)];
        let heard = firing(&session.take_combat_events(), BAY).contains(&ShipRef::Npc(high));
        let high_carrier = session.space.npc(high).expect("there").clone();
        (leaders, high_carrier, low, heard)
    }

    #[test]
    fn with_one_slot_left_a_fighter_launches_from_its_own_bay_and_the_higher_carrier_does_not() {
        // Slot 1 launches into slot 2, whose fighter's `_AIDispatch` comes
        // next and launches into slot 3, the last below the reserve
        // (@0x3c8c6-0x3c8e9). Slot 10's `_LaunchFighter` finds none, and
        // `_AILaunchFighter` returns before the sound (@0x81466), the
        // round (@0x81481) and the reload (@0x81491).
        let (leaders, high, low, heard) = at_the_last_slot(RuleSource::Engine);
        assert_eq!(leaders[0], Some(low));
        let first = leaders[1].expect("a ship at slot 3");
        assert_ne!(first, low);
        assert_ne!(first, high.id);
        assert_eq!(high.rounds.slot(BAY), 2);
        assert!(high.armament.bay_to_launch(&high.rounds).is_some());
        assert!(!heard);
    }

    #[test]
    fn by_the_other_reading_the_higher_carrier_takes_the_last_slot() {
        let (leaders, high, low, heard) = at_the_last_slot(RuleSource::Bible);
        assert_eq!(leaders, [Some(low), Some(high.id)]);
        assert_eq!(high.rounds.slot(BAY), 1);
        assert!(heard);
    }

    /// A pod at `berth` whose wait has run out.
    fn expired_pod(session: &mut Session, berth: u32) -> NpcId {
        let mut pod = bystander();
        pod.ship = crate::combat::hull::ESCAPE_POD_SHIP;
        pod.mode = crate::traffic::npc::Mode::Adrift { ticks_left: 0 };
        session.space.traffic.add_npc_at(pod, berth)
    }

    #[test]
    fn an_expired_pods_slot_is_not_free_to_a_carrier_below_it() {
        // The pod's slot frees at its own `_AIDispatch` (@0x8fbb3), which
        // comes after slot 2's launch, so the launch takes slot 4.
        let catalog = catalog();
        let (mut session, carrier, _) = raid(&catalog, RuleSource::Engine, 2);
        fill(&mut session, [1]);
        let pod = expired_pod(&mut session, 3);
        game_pass(&mut session, &catalog, &Order::of(carrier));
        assert_eq!(vipers(&session)[0].berth, 4);
        assert!(session.space.npc(pod).is_none(), "the pod is gone");
    }

    #[test]
    fn an_expired_pods_slot_is_free_to_a_carrier_above_it_at_the_cap() {
        // Slot 3's `_AIDispatch` frees it before slot 5's launch reads it.
        let catalog = catalog();
        let (mut session, carrier, _) = raid(&catalog, RuleSource::Engine, 5);
        fill(
            &mut session,
            (1..=55).filter(|&berth| ![3, 5, 20].contains(&berth)),
        );
        expired_pod(&mut session, 3);
        game_pass(&mut session, &catalog, &Order::of(carrier));
        assert_eq!(vipers(&session).len(), 1, "it launched");
        assert_eq!(vipers(&session)[0].berth, 3);
    }

    // The low level's leaves at their slot (the traffic's
    // `dispatch_leave`).

    /// A Viper at `berth` docking with `carrier`, at its side within its
    /// reach.
    fn docking_viper(session: &mut Session, berth: u32, carrier: NpcId) -> NpcId {
        let at = session.space.npc(carrier).expect("there").state.position;
        let mut fighter = crate::testkit::npc(0, crate::stats::ShipStats::default());
        fighter.ship = VIPER;
        fighter.state.position = at + Vec2::new(10.0, 0.0);
        fighter.goal = Goal::Dock(ShipRef::Npc(carrier));
        fighter.carrier = Some(crate::bay::Carrier {
            ship: ShipRef::Npc(carrier),
            window: 100.0,
            reach: 60.0,
        });
        session.space.traffic.add_npc_at(fighter, berth)
    }

    /// A ship at `berth` on the last pass of its own warp out.
    fn jumping_out(session: &mut Session, berth: u32) -> NpcId {
        let mut runner = bystander();
        runner.state.position = Vec2::new(-2000.0, 0.0);
        runner.goal = Goal::JumpOut;
        runner.begin_own_jump();
        runner.warp.stage = 3.0;
        runner.warp.since = 100_000;
        session.space.traffic.add_npc_at(runner, berth)
    }

    /// [`raid`]'s carrier at slot 5 at the cap, every slot below the
    /// reserve taken but 3, which `leaver` takes: the session, the carrier
    /// and the leaver.
    fn at_the_cap_with(
        catalog: &FakePilotCatalog,
        leaver: impl FnOnce(&mut Session, NpcId) -> NpcId,
    ) -> (Session, NpcId, NpcId) {
        at_the_cap_above(catalog, 5, leaver)
    }

    /// [`at_the_cap_with`], the carrier at slot `carrier_berth`.
    fn at_the_cap_above(
        catalog: &FakePilotCatalog,
        carrier_berth: u32,
        leaver: impl FnOnce(&mut Session, NpcId) -> NpcId,
    ) -> (Session, NpcId, NpcId) {
        let (mut session, carrier, _) = raid(catalog, RuleSource::Engine, carrier_berth);
        fill(
            &mut session,
            (1..=55).filter(|&berth| ![3, carrier_berth, 20].contains(&berth)),
        );
        let leaver = leaver(&mut session, carrier);
        (session, carrier, leaver)
    }

    #[test]
    fn at_the_cap_a_fighter_launched_into_a_slot_a_lower_jump_out_freed_passes_it_to_the_holders_below_the_leaver_and_above_the_launch()
     {
        // The launch rewrites slot 3 and leaves every +0x70
        // (`_GenericRandomShipSpawn` @0x3c8b1-0x3c90b). Slot 2's high level
        // kept the slot while it was active and finds the newcomer at its
        // `_HandleShip`; slot 4's let go at its slot test (@0x8d4e4-0x8d50d);
        // slot 5's, in its AI wait, skipped it (@0x8d4b9-0x8d4d2) and still
        // held it; slot 7's comes after the launch and finds it active.
        let catalog = catalog();
        let (mut session, carrier, runner) =
            at_the_cap_above(&catalog, 6, |session, _| jumping_out(session, 3));
        let aim = ShipRef::Npc(runner);
        let holder = |session: &mut Session, berth: u32, ai_wait: u32| {
            let id = session.space.traffic.holder_of(berth).expect("filled");
            let npc = session.space.npc_mut(id).expect("there");
            npc.goal = Goal::Attack(aim);
            npc.target = Some(aim);
            npc.ai_wait = ai_wait;
            id
        };
        let below = holder(&mut session, 2, 0);
        let between = holder(&mut session, 4, 0);
        let waiting = holder(&mut session, 5, 50);
        let above = holder(&mut session, 7, 0);
        game_pass(&mut session, &catalog, &Order::of(carrier));
        assert!(session.space.npc(runner).is_none(), "jumped out");
        let launched = vipers(&session);
        assert_eq!(launched.len(), 1, "it launched");
        assert_eq!(launched[0].berth, 3);
        let new = Some(ShipRef::Npc(launched[0].id));
        let target = |id| session.space.npc(id).expect("there").target;
        assert_eq!(target(below), new, "below the leaver");
        assert_eq!(target(between), None, "let go at its slot");
        assert_eq!(target(waiting), new, "its high level skipped");
        assert_eq!(target(above), new, "above the launch");
    }

    /// Whether `carrier` was heard launching among `events`.
    fn launched_from(events: &[CombatEvent], carrier: NpcId) -> bool {
        firing(events, BAY).contains(&ShipRef::Npc(carrier))
    }

    #[test]
    fn at_the_cap_a_slot_a_lower_fighters_dock_frees_is_free_to_a_higher_carriers_launch_that_tick()
    {
        // `_AIAddFighterToParent` frees slot 3 at its own `_AIDispatch`
        // (@0x80404), before slot 5's `_GenericRandomShipSpawn` rereads it
        // (@0x3c8c6-0x3c8e9).
        let catalog = catalog();
        let (mut session, carrier, fighter) = at_the_cap_with(&catalog, |session, carrier| {
            docking_viper(session, 3, carrier)
        });
        game_pass(&mut session, &catalog, &Order::of(carrier));
        assert!(session.space.npc(fighter).is_none(), "aboard");
        let launched = vipers(&session);
        assert_eq!(launched.len(), 1, "it launched");
        assert_eq!(launched[0].berth, 3);
        assert!(launched_from(&session.take_combat_events(), carrier));
    }

    #[test]
    fn at_the_cap_a_slot_a_lower_ships_jump_out_frees_is_free_to_a_higher_carriers_launch_that_tick()
     {
        // Sub 4 frees slot 3 at its own `_AIDispatch` (@0x859b8).
        let catalog = catalog();
        let (mut session, carrier, runner) =
            at_the_cap_with(&catalog, |session, _| jumping_out(session, 3));
        game_pass(&mut session, &catalog, &Order::of(carrier));
        assert!(session.space.npc(runner).is_none(), "jumped out");
        let launched = vipers(&session);
        assert_eq!(launched.len(), 1, "it launched");
        assert_eq!(launched[0].berth, 3);
        assert!(launched_from(&session.take_combat_events(), carrier));
    }

    #[test]
    fn a_fighter_docking_below_its_carrier_restocks_the_bay_its_carrier_launches_from_that_tick() {
        // The bay's count goes up at the fighter's slot (@0x803e6), so the
        // carrier above it, holding one round in a bay of `Reload` 0,
        // finds a second for the snipe's launch after its goal-4 one
        // (@0x8732f). (A bay at 0 has its reload raised by the dock,
        // @0x803a4-0x803c0, and launches nothing that tick in either
        // order.)
        let catalog = catalog();
        let (mut session, carrier, _) = raid(&catalog, RuleSource::Engine, 5);
        let npc = session.space.npc_mut(carrier).expect("there");
        npc.armament = Armament::new([(WeaponSpec::new(&WeaponRecord { reload: 0, ..bay() }), 1)]);
        npc.rounds = LauncherRounds::from([(BAY, 1)]);
        let fighter = docking_viper(&mut session, 3, carrier);
        let script = Scripted {
            carrier,
            twice: true,
            defenders: Vec::new(),
            fighter: Trigger::default(),
        };
        game_pass(&mut session, &catalog, &script);
        assert!(session.space.npc(fighter).is_none(), "aboard");
        assert_eq!(vipers(&session).len(), 2, "both rounds launched");
    }

    // A run in which no NPC asks anything of the fight's loop is skipped
    // (rdm combat-ai-fidelity-cross-roadmap phase 10): it would launch and
    // defend nothing, so only its notes are written.

    /// An NPC of the plain test ship, its trigger `trigger`.
    fn triggered(trigger: Trigger) -> Npc {
        let mut npc = crate::testkit::npc(0, crate::stats::ShipStats::default());
        npc.trigger = trigger;
        npc
    }

    #[test]
    fn a_run_acts_only_when_a_ship_in_it_asks_for_a_launch_or_point_defence() {
        let catalog = catalog();
        let mut session = session(&catalog, RuleSource::Engine);
        let traffic = &mut session.space.traffic;
        traffic.add_npc_at(triggered(GUNNING), 2);
        traffic.add_npc_at(triggered(DEFENDING), 4);
        let snipe = Trigger {
            bays_after: true,
            ..Trigger::default()
        };
        traffic.add_npc_at(triggered(snipe), 6);
        let goal_four = Trigger {
            bays: true,
            ..Trigger::default()
        };
        traffic.add_npc_at(triggered(goal_four), 8);
        let acts = |after, through| session.space.dispatch_run_acts(after, through);
        assert!(!acts(None, Some(2)), "a pick fires in the ship loop");
        assert!(acts(Some(2), Some(4)), "point defence");
        assert!(!acts(Some(4), Some(5)), "an empty run");
        assert!(acts(Some(5), Some(6)), "the snipe's launch");
        assert!(acts(Some(6), Some(8)), "goal 4's launch");
        assert!(!acts(Some(8), None), "none above");
        assert!(acts(None, None), "the batch");
    }

    #[test]
    fn a_skipped_runs_notes_leave_the_fight_as_its_loop_would() {
        let catalog = catalog();
        let mut session = session(&catalog, RuleSource::Engine);
        let mut plain = triggered(Trigger::default());
        plain.state.position = Vec2::new(10.0, 20.0);
        plain.target = Some(ShipRef::Player);
        let plain = session.space.traffic.add_npc_at(plain, 3);
        let mut plunderer = triggered(Trigger::default());
        plunderer.goal = Goal::Plunder {
            ship: ShipRef::Npc(plain),
            hopeless: false,
        };
        plunderer.target = Some(ShipRef::Npc(plain));
        session.space.traffic.add_npc_at(plunderer, 5);
        let mut docked = triggered(Trigger::default());
        docked.goal = Goal::Attack(ShipRef::Npc(plain));
        docked.target = Some(ShipRef::Npc(plain));
        docked.docking = Some(crate::traffic::plunder_dock::Docking {
            ship: ShipRef::Npc(plain),
            sub: crate::traffic::plunder_dock::DockSub::Dock,
        });
        session.space.traffic.add_npc_at(docked, 7);
        session.chart.target = Some(plain);
        session.space.pass_frame = 9;
        for pass in [GamePass::Advance { double_time: false }, GamePass::Again] {
            let mut looped = session.clone();
            looped.with_fighters(
                pass,
                Rules::default(),
                &mut NeverFires,
                |combat, fighters, arena, chance| {
                    combat.defend_npcs(fighters, arena, None, None, chance);
                },
            );
            let mut noted = session.clone();
            noted.note_idle_run(pass);
            assert_eq!(noted.space.combat, looped.space.combat, "{pass:?}");
            let wrote = looped.space.combat != session.space.combat;
            assert_eq!(wrote, pass != GamePass::Again, "{pass:?}");
        }
    }

    #[test]
    fn a_dispatch_of_skipped_runs_leaves_the_fights_notes_as_a_fought_run_would() {
        // Every run here asks nothing of the loop, so each is skipped and
        // noted by `note_idle_run`; the fight must end as one fought run
        // over the same ships leaves it: the field and every slot record.
        let catalog = catalog();
        let mut session = session(&catalog, RuleSource::Engine);
        let mut plain = triggered(Trigger::default());
        plain.state.position = Vec2::new(10.0, 20.0);
        plain.target = Some(ShipRef::Player);
        let plain = session.space.traffic.add_npc_at(plain, 2);
        let mut plunderer = triggered(Trigger::default());
        plunderer.state.position = Vec2::new(-40.0, 5.0);
        plunderer.goal = Goal::Plunder {
            ship: ShipRef::Npc(plain),
            hopeless: false,
        };
        plunderer.target = Some(ShipRef::Npc(plain));
        session.space.traffic.add_npc_at(plunderer, 4);
        session.chart.target = Some(plain);
        session.space.pass_frame = 9;
        let was = session.space.combat.clone();
        let script = Scripted {
            carrier: NpcId(999),
            twice: false,
            defenders: Vec::new(),
            fighter: Trigger::default(),
        };
        let pass = GamePass::Advance { double_time: false };
        session.dispatch_npcs(true, pass, &script, Rules::default(), &mut NeverFires);
        assert!(
            !session.space.dispatch_run_acts(None, None),
            "every run was idle"
        );
        let mut fought = session.clone();
        fought.space.combat = was.clone();
        fought.with_fighters(
            pass,
            Rules::default(),
            &mut NeverFires,
            |combat, fighters, arena, chance| {
                combat.defend_npcs(fighters, arena, None, None, chance);
            },
        );
        assert_eq!(session.space.combat, fought.space.combat);
        assert_ne!(session.space.combat, was, "the runs wrote their notes");
    }

    #[test]
    fn a_carrier_and_a_defender_after_idle_runs_still_launch_and_fire_on_the_launch_tick() {
        // The idle ships at 2 and 5 ask nothing of the loop, so their runs
        // are skipped; the carrier's and the high defender's are not.
        let above = Berths {
            low: 1,
            carrier: 3,
            high: 7,
        };
        let (berth, fired) = defended_against(RuleSource::Engine, &above, &[2, 5]);
        assert_eq!(berth, 4);
        assert_eq!(fired, [["high"], ["low"]]);
    }

    #[test]
    fn an_npc_idle_in_its_own_run_is_still_a_later_slots_point_defence_candidate() {
        // The raider at 2 attacks the carrier at 1 and asks nothing of the
        // loop: its run is skipped, and the Viper launched into slot 3
        // fires at it on its launch tick.
        let catalog = catalog();
        let (mut session, carrier, raider) = raid_at(&catalog, RuleSource::Engine, 1, 2);
        let script = Scripted {
            carrier,
            twice: false,
            defenders: Vec::new(),
            fighter: DEFENDING,
        };
        game_pass(&mut session, &catalog, &script);
        let launched = vipers(&session);
        assert_eq!(launched.len(), 1, "one launch");
        assert_eq!(launched[0].berth, 3);
        let events = session.take_combat_events();
        assert!(firing(&events, QUAD).contains(&ShipRef::Npc(launched[0].id)));
        let npc = session.space.npc(raider).expect("there");
        assert!(!npc.trigger.acts_in_dispatch(), "idle in its own run");
    }
}
