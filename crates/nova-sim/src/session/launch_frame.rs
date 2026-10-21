//! A fighter an NPC launches, in the ship table at once
//! ([`Session::dispatch_npcs`]).
//!
//! `_LaunchFighter` marks the fighter's slot active as `_AIDispatch`'s
//! loop launches it (`_GenericRandomShipSpawn` @0x3c90b, @0x3d680), in its
//! carrier's system (@0x3d694), of AI type 5 (@0x3d771), and
//! `_DoPlayGameWork`'s loop over slots 1-63 rereads each slot's active
//! flag, AI type and system as it comes (@0x44270-0x442ae). So:
//!
//! - a fighter put in a slot above its carrier's is reached by the same
//!   loop in its launch frame: its `_AIDispatch` runs its whole AI (see
//!   `nova_sim::traffic`'s `launch_think`), its point defence among it
//!   (`_AIFireTurret` → `_HandleShipPointDefense` @0x80b0d), and
//!   `_HandleShip` fires the pick it chose that frame (@0x34fce-0x34ffd);
//! - one put in a slot below (one freed earlier) has been passed, so it
//!   thinks first in the next frame, and fires nothing in its launch frame,
//!   as `_GenericRandomShipSpawn` clears its fire flag (@0x3cb60);
//! - whichever slot it took, every slot whose `_AIDispatch` comes after
//!   the launch can choose it in its point defence's ship branch, which
//!   walks the live table (@0x394ce-0x3969d); a slot before the carrier's
//!   cannot.
//!
//! Every fighter moves and is displayed in its launch frame either way,
//! as the ship loop comes after the AI loop (@0x4445c-0x444c4). Nothing
//! here sounds: `_LaunchFighter` and `_GenericRandomShipSpawn` play
//! nothing, and the launch's own sound is `_AILaunchFighter`'s (@0x81466).
//! [`RuleKey::LaunchFrameAi`]'s other reading puts every fighter in the
//! system after the whole loop instead, neither thinking nor a candidate
//! until the next tick.
//!
//! The existing NPCs still think in one batch before the loop, so an NPC
//! in a slot above the carrier's thinks blind to the fighter, and the
//! fighter's own think reads them as their last frame left them
//! (rdm:phase/combat-ai-fidelity-cross-roadmap/phase-6-left-out-of-weapons-fidelity-phase-44).

use std::collections::BTreeSet;

use super::Session;
use crate::ai::Behaviour;
use crate::chance::Chance;
use crate::combat::{GamePass, Rules};
use crate::rulebook::{RuleKey, RuleSource};
use crate::ship_layer::Slot;
use crate::traffic::npc::NpcId;

impl Session {
    /// `_AIDispatch`'s loop on `pass` by `rules`, the launched fighters
    /// thinking as `behaviour` says, drawing on `chance` (see the module
    /// docs): every NPC's point defence and launches in slot order
    /// ([`Combat::defend_npcs`](crate::combat::Combat::defend_npcs)), the
    /// loop stopping after each slot whose turn launched; that slot's
    /// fighters are put in the system at once
    /// ([`Session::launch_fighters`]), each in a slot above the
    /// carrier's thinks
    /// ([`Traffic::think_launched`](crate::traffic::Traffic::think_launched)),
    /// and the loop resumes from the next slot with them among its ships.
    /// By [`RuleKey::LaunchFrameAi`]'s other reading, one run that never
    /// stops, its fighters put in the system after it.
    ///
    /// The fighters' work on the line (their calls, their scans) is
    /// queued at their own berths as they think, among every other
    /// slot's, and the caller settles it all after this loop in slot order
    /// ([`Session::settle_line_queue`]), where the original says each in
    /// its own slot's `_AIDispatch`. Nothing in the loop draws a frame,
    /// opens a dialog, lands, jumps or saves the pilot, so the wait
    /// crosses no boundary the player sees: the lines are said before the
    /// frame is drawn.
    pub(super) fn dispatch_npcs(
        &mut self,
        pass: GamePass,
        behaviour: &(impl Behaviour + ?Sized),
        rules: Rules,
        chance: &mut dyn Chance,
    ) {
        let stop = self.setup.rules.source_for(RuleKey::LaunchFrameAi) == RuleSource::Engine;
        let mut after = None;
        loop {
            let mut stopped = None;
            self.with_fighters(pass, rules, chance, |combat, fighters, arena, chance| {
                stopped = combat.defend_npcs(fighters, arena, after, stop, chance);
            });
            let sorties = self.space.combat.take_sorties();
            let placed = self.launch_fighters(&sorties, chance);
            let Some(Slot::Berth(carrier)) = stopped else {
                return;
            };
            let above: BTreeSet<NpcId> = self
                .space
                .traffic
                .npcs()
                .iter()
                .filter(|npc| placed.contains(&npc.id) && npc.berth > carrier)
                .map(|npc| npc.id)
                .collect();
            if !above.is_empty() {
                self.step_traffic(|traffic, world| {
                    traffic.think_launched(&above, behaviour, world, chance);
                });
            }
            after = stopped;
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use crate::ai::pursuit::Pursuit;
    use crate::ai::{Behaviour, CommCall, Decision, Goal, Surroundings};
    use crate::catalog::{HullRecord, StockWeapon, WeaponId, WeaponRecord};
    use crate::chance::Chance;
    use crate::combat::armament::{Armament, LauncherRounds, Trigger};
    use crate::combat::weapon::WeaponSpec;
    use crate::combat::{CombatEvent, ShipRef};
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

    /// Keeps every goal and target; the carrier asks for its goal-4
    /// launch (and the snipe's after it when `twice`), each defender for
    /// its point defence, and every Viper for `fighter`.
    #[derive(Debug)]
    struct Scripted {
        carrier: NpcId,
        twice: bool,
        defenders: Vec<NpcId>,
        fighter: Trigger,
    }

    impl Behaviour for Scripted {
        fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            npc.goal
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
        let mut session = session(catalog, source);
        let mut raider = crate::testkit::npc(0, crate::stats::ShipStats::default());
        raider.state.position = Vec2::new(1000.0, 1100.0);
        raider.hull.pd_target = true;
        let raider = session.space.traffic.add_npc_at(raider, 20);
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
    /// pass which defenders fired at it.
    fn defended_against(source: RuleSource, berths: &Berths) -> (u32, [Vec<&'static str>; 2]) {
        let catalog = catalog();
        let mut session = session(&catalog, source);
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
        let (berth, fired) = defended_against(RuleSource::Engine, &above);
        assert_eq!(berth, 3);
        // The low defender first fires the next tick, the high's Quad
        // reloading then.
        assert_eq!(fired, [["high"], ["low"]]);
        let below = Berths {
            low: 2,
            carrier: 4,
            high: 6,
        };
        let (berth, fired) = defended_against(RuleSource::Engine, &below);
        assert_eq!(berth, 1);
        assert_eq!(fired, [["high"], ["low"]]);
    }

    #[test]
    fn by_the_other_reading_no_point_defence_sees_a_fighter_on_its_launch_tick() {
        let above = Berths {
            low: 1,
            carrier: 2,
            high: 6,
        };
        let (_, [launch, next]) = defended_against(RuleSource::Bible, &above);
        assert_eq!(launch, Vec::<&str>::new());
        assert_eq!(next, ["low", "high"]);
    }

    /// [`Scripted`], every Viper calling for help as it decides.
    #[derive(Debug)]
    struct CallingVipers(Scripted);

    impl Behaviour for CallingVipers {
        fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
            self.0.decide(npc, around, chance)
        }

        fn decision(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Decision {
            Decision {
                goal: self.0.decide(npc, around, chance),
                comm_call: (npc.ship == VIPER).then_some(CommCall::Distress),
                ..Decision::default()
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
        let script = CallingVipers(Scripted {
            carrier,
            twice: false,
            defenders: Vec::new(),
            fighter: Trigger::default(),
        });
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
}
