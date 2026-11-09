//! The session's side of [`escape_pod`](crate::escape_pod): the pods a
//! dying NPC ejects, set adrift as the ship loop raises each
//! [`CombatEvent::PodEjected`](crate::combat::CombatEvent::PodEjected)
//! (see the `npc_escape` module), stepped each flight pass, carried by
//! the player's wrap (see the `player_wrap` module), scooped for box slot
//! 0 (see the `pod_scoop` module), handed to the view in slot order
//! ([`FlightFrame::pods`](crate::flight_frame::FlightFrame::pods)) and
//! cleared wherever `_killShots` frees them. They are never saved.
//!
//! **Ordering.** `_DoPlayGameWork` collides the box layer, pods among it
//! (@0x43e70), then runs the ship loop, whose `_HandleShip` spawns them
//! (@0x44483), then `_HandleBoxes` (@0x444f0) and `_HandleEscapePods`
//! (@0x44513). The session steps the pods with the boxes, after the scoop
//! and before the ship loop (`Session::scoop_after_display`), and a pod
//! the ship loop spawns takes that pass's step at once
//! ([`EscapePods::spawn`](crate::escape_pod::EscapePods::spawn)). Stepping
//! early changes what the spawns read: which slots are free. The original
//! frees a slot only once its life is below 0 at a spawn, and a pod falls
//! below 0 in the step after the spawns, so the session's step frees only
//! the pods already below 0 before it
//! ([`EscapePods::step`](crate::escape_pod::EscapePods::step)): one that
//! falls below 0 holds its slot, hidden, through the pass's spawns, under
//! the caps too, and the next pass's step frees it. So each pass ends with
//! every pod in the slot, place and visibility the original's frame draws
//! it with, and the next pass's scoop meets a pod just fallen below 0
//! still hidden in its slot, as the original's collision does.
//!
//! **What a spawn reads** ([`PodSpawn`]): the pass's gain, whether the
//! pool is capped at `_podData`'s 32 (`RuleKey::CombatCaps`' engine
//! reading; by default not), and the rotations of shän 895, the escape
//! pod's class ([`ESCAPE_POD_SHIP`](crate::combat::hull::ESCAPE_POD_SHIP)),
//! whose master sprite `_InitSprites` clones the pods from (@0x1fc95).
//! The pod's rolls come from the session's chance.

use std::num::NonZeroU16;

use super::{Session, Setup, Space, Statics};
use crate::SpeedMult;
use crate::chance::Chance;
use crate::combat::hull::ESCAPE_POD_SHIP;
use crate::escape_pod::Pod;
use crate::geometry::Vec2;
use crate::rulebook::{RuleKey, RuleSource};

/// What a pass's pod spawn reads (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct PodSpawn {
    /// The pass's gain, which its first step takes.
    pub(super) gain: SpeedMult,
    /// Whether the pool is capped at `_podData`'s 32.
    pub(super) caps: bool,
    /// shän 895's `FramesPer` rotations.
    pub(super) frames: Option<NonZeroU16>,
}

impl Setup {
    /// What a pod spawned on a pass at `gain` reads (see the module docs).
    pub(super) fn pod_spawn(&self, statics: &Statics, gain: SpeedMult) -> PodSpawn {
        PodSpawn {
            gain,
            caps: self.rules.source_for(RuleKey::CombatCaps) == RuleSource::Engine,
            frames: Some(self.frames_of(statics, ESCAPE_POD_SHIP).frames),
        }
    }
}

impl Space {
    /// Sets a pod adrift from a ship at `at` moving at `velocity`, as
    /// `spawn` says, drawing on `chance` (`_SpawnEscapePod`).
    pub(super) fn spawn_pod(
        &mut self,
        at: Vec2,
        velocity: Vec2,
        spawn: PodSpawn,
        chance: &mut (impl Chance + ?Sized),
    ) {
        self.pods
            .spawn(at, velocity, spawn.gain, spawn.caps, spawn.frames, chance);
    }

    /// Steps the pods a pass at the frame's `gain`, as `_DoPlayGameWork`
    /// runs `_HandleEscapePods` (@0x44513) every flight pass, freeing
    /// only the pods already below 0 (see the module docs).
    pub(super) fn pods_tick(&mut self, gain: SpeedMult) {
        self.pods.step(gain);
    }

    /// Frees every pod, as `_killShots` does (@0x39e75-0x39e84).
    pub(super) fn clear_pods(&mut self) {
        self.pods.clear();
    }

    /// Fills `into` with the pods adrift, in slot order.
    pub(super) fn fill_pods(&self, into: &mut Vec<Pod>) {
        into.clear();
        into.extend_from_slice(self.pods.pods());
    }
}

impl Session {
    /// The NPCs' escape pods adrift, the hidden ones too, in slot order
    /// (see [`escape_pod`](crate::escape_pod)).
    #[must_use]
    pub fn escape_pods(&self) -> &[Pod] {
        self.space.pods.pods()
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
pub(super) mod tests {
    use super::*;
    use crate::catalog::SystemId;
    use crate::chance::NeverFires;
    use crate::combat::CombatEvent;
    use crate::combat::hull::Condition;
    use crate::escape_pod::{EscapePods, MAX_PODS};
    use crate::flight::Controls;
    use crate::flight_frame::FlightFrame;
    use crate::gate::{GateRefusal, WORMHOLE};
    use crate::rulebook::Rulebook;
    use crate::session::tests::{at_wormhole, gate_site, wormholes};
    use crate::stats::ShipStats;
    use crate::testkit::{FakePilotCatalog, begin_jump_now, catalog, fly_out, land_now, work};
    use crate::traffic::npc::Npc;

    fn flying(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        assert_eq!(session.landed(), None);
        session.populate(catalog, &mut NeverFires);
        session
    }

    /// `session` with a pod spawned at `at` from rest, at full gain.
    fn with_pod(session: &mut Session, at: Vec2) {
        let spawn = session.setup.pod_spawn(&session.statics, SpeedMult::FULL);
        session
            .space
            .spawn_pod(at, Vec2::ZERO, spawn, &mut NeverFires);
    }

    /// An NPC at `at` that ejects a crew pod on the next pass: its death
    /// timer 51, which the pass takes to 50, half its `DeathDelay` of 100
    /// (see the `npc_escape` module).
    pub(in crate::session) fn ejecting(at: Vec2) -> Npc {
        let mut npc = crate::testkit::npc(0, ShipStats::default());
        npc.condition = Condition::Dying {
            ticks_left: 51,
            starting: false,
        };
        npc.hull.death_delay = 100;
        npc.hull.signed_death_delay = 100;
        npc.hull.pod_count = 2;
        npc.pods_left = 2;
        npc.reserves.armor.now = 0.0;
        npc.state.position = at;
        npc
    }

    /// `count` shown pods at rest far off, slot 0's life 0.5, so a pass
    /// at full gain takes it below 0, the others' 100.
    fn first_dying(count: usize) -> EscapePods {
        let pods: Vec<Pod> = (0..count)
            .map(|slot| Pod {
                position: Vec2::new(-2000.0, 0.0),
                velocity: Vec2::ZERO,
                life: if slot == 0 { 0.5 } else { 100.0 },
                frame: 0,
                shown: true,
            })
            .collect();
        EscapePods::adrift(&pods)
    }

    fn ejected(session: &mut Session) -> bool {
        session
            .take_combat_events()
            .iter()
            .any(|event| matches!(event, CombatEvent::PodEjected { .. }))
    }

    #[test]
    fn a_pod_whose_life_falls_below_0_in_a_pass_holds_its_slot_through_that_passs_spawns() {
        // `_SpawnEscapePod` (@0x44483) runs before `_HandleEscapePods`
        // (@0x44513), and frees a slot only once its life is below 0
        // (@0x45564-0x456bf): slot 0, at 0.5, is still held.
        let catalog = catalog();
        let mut session = flying(&catalog);
        session.space.pods = first_dying(2);
        session
            .space
            .traffic
            .add_npc(ejecting(Vec2::new(300.0, 0.0)));
        session.take_combat_events();
        work(&mut session, &catalog, Controls::default());
        assert!(ejected(&mut session));
        let pods = session.escape_pods();
        assert_eq!(pods.len(), 3, "the new pod in slot 2: {pods:?}");
        assert!(pods[0].life < 0.0 && !pods[0].shown, "slot 0 hidden");
        assert_eq!(pods[1].life, 99.0);
        assert!(pods[2].shown && pods[2].position.x > 0.0, "the new pod");
        // The next pass frees slot 0.
        let new = pods[2];
        work(&mut session, &catalog, Controls::default());
        let lives: Vec<f32> = session.escape_pods().iter().map(|pod| pod.life).collect();
        assert_eq!(lives, [98.0, new.life - 1.0]);
    }

    #[test]
    fn under_the_caps_a_pod_ejected_as_the_33rd_dies_is_not_kept() {
        let catalog = catalog();
        let mut session = flying(&catalog)
            .with_rules(Rulebook::default().with_override(RuleKey::CombatCaps, RuleSource::Engine));
        session.space.pods = first_dying(MAX_PODS);
        session
            .space
            .traffic
            .add_npc(ejecting(Vec2::new(300.0, 0.0)));
        session.take_combat_events();
        work(&mut session, &catalog, Controls::default());
        assert!(ejected(&mut session));
        let pods = session.escape_pods();
        assert_eq!(pods.len(), MAX_PODS);
        assert!(pods[0].life < 0.0 && !pods[0].shown, "slot 0 still held");
        assert!(pods.iter().all(|pod| pod.position.x < 0.0), "none new");
    }

    #[test]
    fn a_pass_steps_the_pods_at_its_gain() {
        let catalog = catalog();
        let mut session = flying(&catalog);
        with_pod(&mut session, Vec2::new(300.0, 0.0));
        let mut expected = session.space.pods.clone();
        work(&mut session, &catalog, Controls::default());
        expected.step(session.speed_mult());
        assert_eq!(session.escape_pods(), expected.pods());
        assert_ne!(
            session.escape_pods()[0].life,
            expected.pods()[0].life + 1.0,
            "stepped"
        );
    }

    #[test]
    fn the_frame_carries_the_pods_hidden_ones_too() {
        let catalog = catalog();
        let mut session = flying(&catalog);
        with_pod(&mut session, Vec2::new(300.0, 0.0));
        with_pod(&mut session, Vec2::new(-300.0, 0.0));
        session.space.pods.hide(1);
        let mut frame = FlightFrame::default();
        session.fill_frame(&mut frame);
        assert_eq!(frame.pods, session.escape_pods());
        assert_eq!(frame.pods.len(), 2);
        assert!(!frame.pods[1].shown);
    }

    #[test]
    fn landing_clears_the_pods() {
        let catalog = catalog();
        let mut session = flying(&catalog);
        with_pod(&mut session, Vec2::new(300.0, 0.0));
        land_now(&mut session).expect("lands");
        assert!(session.escape_pods().is_empty());
    }

    #[test]
    fn a_jumps_arrival_clears_the_pods() {
        // The arrival's `_killShots` frees the pods (@0x39e75-0x39e84).
        let catalog = catalog();
        let mut session = flying(&catalog);
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session, &catalog);
        begin_jump_now(&mut session, &catalog).expect("jumps");
        with_pod(&mut session, Vec2::new(300.0, 0.0));
        assert_eq!(session.escape_pods().len(), 1);
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(131))
        );
        assert!(session.escape_pods().is_empty());
    }

    #[test]
    fn a_wormhole_with_no_exit_clears_the_pods() {
        // The dock request's tail runs `_killShots` (@0x66f40), which
        // frees the pods (@0x39e75-0x39e84).
        let catalog = FakePilotCatalog {
            gates: vec![gate_site(400, (0.0, 0.0), WORMHOLE, &[], 0)],
            ..wormholes()
        };
        let mut session = at_wormhole(&catalog);
        with_pod(&mut session, Vec2::new(300.0, 0.0));
        assert_eq!(
            session.enter_wormhole(&catalog, &mut NeverFires),
            Err(GateRefusal::NoExit)
        );
        assert!(session.escape_pods().is_empty());
    }

    #[test]
    fn a_pods_spawn_reads_the_caps_and_shan_895s_rotations() {
        let catalog = catalog();
        let session = flying(&catalog);
        let spawn = session.setup.pod_spawn(&session.statics, SpeedMult::FULL);
        assert!(!spawn.caps, "uncapped by default");
        assert_eq!(
            spawn.frames,
            Some(
                session
                    .setup
                    .frames_of(&session.statics, ESCAPE_POD_SHIP)
                    .frames
            )
        );
        let capped = session
            .with_rules(Rulebook::default().with_override(RuleKey::CombatCaps, RuleSource::Engine));
        let spawn = capped.setup.pod_spawn(&capped.statics, SpeedMult::FULL);
        assert!(spawn.caps);
    }
}
