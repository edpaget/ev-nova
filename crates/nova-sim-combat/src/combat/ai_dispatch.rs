//! `_AIDispatch`'s slot loop (`_DoPlayGameWork` @0x44270-0x442ae in the
//! `EV Nova` executable), as the fight runs it in step 4
//! ([`Combat::defend_npcs`]): each NPC, in slot order, launches its
//! fighters and runs its point defence in the order its AI calls them.
//!
//! - **The order within a slot.** `_HighLevelAIHandler` runs first and
//!   makes the goal-4 launch (`_AILaunchFighter` @0x8e6fa,
//!   [`Trigger::bays`](super::armament::Trigger::bays)); then
//!   `_LowLevelAIHandler`, whose substates call `_AIFireTurret`, which
//!   runs `_HandleShipPointDefense` first (@0x80b0d,
//!   [`Trigger::point_defence`](super::armament::Trigger::point_defence));
//!   last, the snipe's own launch after its turret (@0x8732f,
//!   [`Trigger::bays_after`](super::armament::Trigger::bays_after)).
//! - **The order of the slots.** The loop takes slot 1 to 63 in turn
//!   ([`Fighter::slot`]), so one NPC's launch and its sound come after
//!   every lower slot's point defence and launches, and before every
//!   higher slot's, and all of them before `_HandleShot` (@0x44385) and
//!   every NPC's pick in `_HandleShip` (@0x44483).
//! - **A launch** is `_AILaunchFighter`'s
//!   ([`Armament::launch_bay`](super::armament::Armament::launch_bay)):
//!   the bay's round spent and its reload set, a
//!   [`CombatEvent::Fired`](super::CombatEvent::Fired) at the carrier at
//!   [`BAY_LAUNCH_PRIORITY`] with no looped test (@0x81466), and a
//!   [`Sortie`](super::Sortie) the session puts in the system before
//!   `_HandleShot` (`nova_sim::Session::game_work`). Whether the fighter
//!   finds a slot was settled as the traffic thought (`nova_sim::traffic`'s
//!   room ledger), so a launch asked for here has one.
//!
//! A launch shares no shot slot or exit cursor with point defence:
//! `_LaunchFighter` spawns a ship (`_GenericRandomShipSpawn`) and writes
//! nothing of its carrier's exits. The fighter, placed after the whole
//! loop, neither thinks nor is a later slot's candidate until the next
//! tick, where the original's loop reaches its slot in its launch frame
//! (rdm:phase/weapons-fidelity/phase-44-fighter-launched-into-a-later-slot-thinks-on-its-launch-frame).

use super::defence::Candidate;
use super::projectile::Target;
use super::{BAY_LAUNCH_PRIORITY, Combat, Fighter, FireSound, PointDefenceRule, ShipRef, aim};
use crate::chance::Chance;
use crate::govt::Governments;
use crate::rulebook::RuleSource;

impl Combat {
    /// `_AIDispatch`'s loop over the NPCs among `fighters` (see the module
    /// docs): by slot, each one's goal-4 launch, its point defence
    /// ([`Combat::defend_ship`]) at the ships as they were when the step
    /// began, then the snipe's launch, aimed among `targets`, point
    /// defence asking `rule` as [`Combat::defend_ship`] does, drawing on
    /// `chance`.
    pub(super) fn dispatch_npcs(
        &mut self,
        fighters: &mut [Fighter],
        targets: &[Target],
        defence: (&dyn PointDefenceRule, &Governments),
        chance: &mut dyn Chance,
    ) {
        let candidates: Vec<Candidate> = fighters.iter().map(Fighter::as_candidate).collect();
        let mut order: Vec<usize> = (0..fighters.len())
            .filter(|&index| fighters[index].ship != ShipRef::Player)
            .collect();
        order.sort_by_key(|&index| fighters[index].slot);
        for index in order {
            let fighter = &mut fighters[index];
            let trigger = fighter.trigger;
            if trigger.bays {
                self.launch_bay(fighter, targets, chance);
            }
            if trigger.point_defence && !fighter.jumping {
                self.defend_ship(fighter, &candidates, targets, defence, chance);
            }
            if trigger.bays_after {
                self.launch_bay(fighter, targets, chance);
            }
        }
    }

    /// One `_AILaunchFighter` of `fighter`'s
    /// ([`Armament::launch_bay`](super::armament::Armament::launch_bay)),
    /// its fighter aimed at its target among `targets` while that stands,
    /// its inaccuracy drawn on `chance`: launched from its muzzle as any
    /// volley ([`Combat::launch_volley`]), so a
    /// [`Sortie`](super::Sortie), sounding at [`BAY_LAUNCH_PRIORITY`] with
    /// no looped test (@0x81429-0x81466).
    fn launch_bay(&mut self, fighter: &mut Fighter, targets: &[Target], chance: &mut dyn Chance) {
        let target = standing(fighter.target, targets);
        let (state, hull) = (fighter.state, fighter.hull);
        let launch = fighter.armament.launch_bay(
            *fighter.condition,
            fighter.rounds,
            chance,
            &mut self.reports,
            &mut |spec| aim::aim(spec, &state, &hull, target, RuleSource::Bible),
        );
        let Some(launch) = launch else {
            return;
        };
        let sound = |_| {
            Some(FireSound {
                priority: BAY_LAUNCH_PRIORITY,
                unless_playing: false,
            })
        };
        self.launch_volley(fighter, &[launch], sound, targets, chance);
    }
}

/// `target` among `targets` while it can be hit: what a ship's weapons
/// aim at ([`Combat::fire`], [`Combat::launch_bay`]).
pub(super) fn standing(target: Option<ShipRef>, targets: &[Target]) -> Option<&Target> {
    target.and_then(|ship| {
        targets
            .iter()
            .find(|target| target.ship == ship && target.condition.standing())
    })
}
