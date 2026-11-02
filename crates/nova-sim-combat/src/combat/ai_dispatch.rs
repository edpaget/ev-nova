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
//!   finds a slot was settled as its carrier thought (`nova_sim::traffic`'s
//!   `launch_room`), so a launch asked for here has one.
//!
//! A launch shares no shot slot or exit cursor with point defence:
//! `_LaunchFighter` spawns a ship (`_GenericRandomShipSpawn`) and writes
//! nothing of its carrier's exits.
//!
//! **The loop in runs.** `_LaunchFighter` marks the fighter's slot active
//! at once (`_GenericRandomShipSpawn` @0x3c90b, @0x3d680), and the loop
//! rereads each slot as it comes, so the fight takes the loop in bounded
//! runs ([`Combat::defend_npcs`]'s `after` and `through`): the session
//! runs one slot's think and then its turn here, puts the fighters it
//! launched in the system, and takes the next slot with them among the
//! ships, each a candidate of every later slot's point defence and, above
//! its carrier, taking its own turn (`nova_sim::Session::dispatch_npcs`,
//! by [`RuleKey::LaunchFrameAi`](crate::RuleKey::LaunchFrameAi), whose
//! other reading is one run of every slot).

use super::defence::Candidate;
use super::projectile::Target;
use super::{
    BAY_LAUNCH_PRIORITY, Combat, Field, Fighter, FireSound, GamePass, PointDefenceRule, ShipRef,
    SlotShip, aim,
};
use crate::chance::Chance;
use crate::govt::Governments;
use crate::rulebook::RuleSource;
use crate::ship_layer::Slot;

impl Combat {
    /// `_AIDispatch`'s loop over the NPCs among `fighters` in slots above
    /// `run`'s first and up to its second, each bound open with none (see
    /// the module docs): by slot, each one's goal-4 launch, its point
    /// defence ([`Combat::defend_ship`]) at the ships as they were when
    /// the run began, then the snipe's launch, aimed among `targets`,
    /// point defence asking `rule` as [`Combat::defend_ship`] does,
    /// drawing on `chance`. A ship whose trigger asks nothing of the loop
    /// ([`Trigger::acts_in_dispatch`](super::armament::Trigger::acts_in_dispatch))
    /// is passed over, as it would enter
    /// none of the three; the session skips a run of only such ships by
    /// the same test (`nova_sim::Session::dispatch_npcs`).
    pub(super) fn dispatch_npcs(
        &mut self,
        fighters: &mut [Fighter],
        targets: &[Target],
        defence: (&dyn PointDefenceRule, &Governments),
        run: (Option<Slot>, Option<Slot>),
        chance: &mut dyn Chance,
    ) {
        let (after, through) = run;
        let candidates: Vec<Candidate> = fighters.iter().map(Fighter::as_candidate).collect();
        let mut order: Vec<usize> = (0..fighters.len())
            .filter(|&index| fighters[index].ship != ShipRef::Player)
            .filter(|&index| after.is_none_or(|after| fighters[index].slot > after))
            .filter(|&index| through.is_none_or(|through| fighters[index].slot <= through))
            .filter(|&index| fighters[index].trigger.acts_in_dispatch())
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

    /// What [`Combat::defend_npcs`] leaves of a run in which no NPC's
    /// trigger asks anything of the loop
    /// ([`Trigger::acts_in_dispatch`](super::armament::Trigger::acts_in_dispatch)):
    /// on `pass`, when it is not double time's second, the fight's `field`,
    /// and each of `ships` in its slot's record, as
    /// [`Combat::defend_npcs`] notes every fighter. So a session that
    /// skips such a run's fighters leaves the fight as the run would have,
    /// noting the ships it holds, the player too, in its own terms.
    pub fn note_idle_run(
        &mut self,
        pass: GamePass,
        field: Field,
        ships: impl IntoIterator<Item = (Slot, SlotShip)>,
    ) {
        if pass == GamePass::Again {
            return;
        }
        self.field = field;
        self.slot_ships.extend(ships);
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
        let launch =
            fighter
                .armament
                .launch_bay(*fighter.condition, fighter.rounds, chance, &mut |spec| {
                    aim::aim(spec, &state, &hull, target, RuleSource::Bible)
                });
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

/// `target` among `targets` while it can be hit: what the player's
/// weapons, an NPC's trigger and its bay aim at ([`Combat::fire`],
/// [`Combat::launch_bay`]); an NPC's pick aims at any [`active`] target.
pub(super) fn standing(target: Option<ShipRef>, targets: &[Target]) -> Option<&Target> {
    target.and_then(|ship| {
        targets
            .iter()
            .find(|target| target.ship == ship && target.condition.standing())
    })
}

/// `target` among `targets` while its slot is active
/// ([`Condition::active`](super::Condition::active)), breaking up
/// included: what an NPC's pick aims at ([`Combat::fire`]).
/// `_FireAIShipWeapon` (@0x8873d-0x89269) reads of its target only whether
/// there is one (+0x70 against -1, @0x88824, @0x889ea, @0x88bbf,
/// @0x88c0f), its position (+0x18, +0x1c) and whether it is slot 0, the
/// player (@0x88fb3), and calls no `_IsDying`; nor do `_SpawnShot`
/// (@0x3e550) or `_SpawnBeam` (@0x44ff3). A destroyed ship's slot isn't
/// active, so it is no target.
pub(super) fn active(target: Option<ShipRef>, targets: &[Target]) -> Option<&Target> {
    target.and_then(|ship| {
        targets
            .iter()
            .find(|target| target.ship == ship && target.condition.active())
    })
}
