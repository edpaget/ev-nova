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
//! ([`EscapePods::spawn`](crate::escape_pod::EscapePods::spawn)). Nothing
//! between reads a pod, so each pass ends with every pod where the
//! original's frame draws it.
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
    /// runs `_HandleEscapePods` (@0x44513) every flight pass.
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
mod tests {
    use super::*;
    use crate::chance::NeverFires;
    use crate::flight::Controls;
    use crate::flight_frame::FlightFrame;
    use crate::rulebook::Rulebook;
    use crate::testkit::{FakePilotCatalog, catalog, land_now, work};

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
