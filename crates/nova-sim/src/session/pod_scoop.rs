//! A scooping ship takes an escape pod for box slot 0, as
//! `_ShipCollisionHandler`'s scoop branch reads a pod's sprite +0xd0 of 0
//! (@0x370db-0x371ac; see [`scoop`](crate::scoop)'s "Escape pods"), under
//! [`RuleKey::PodScoopsBoxZero`]'s engine reading, the default. By the
//! other reading, pods are never scooped.
//!
//! [`Session::scoop_boxes`] runs it under the boxes' own gate (a live
//! scoopable box, `_miningBoxesActive`), for each taker in its order, after
//! the taker's boxes, as `_SWCollideSpriteLayer` walks `_boxLayer`'s boxes
//! 0-63 before its pods 0-31 for each ship:
//!
//! - the player ([`Session::player_scoop_pods`]), testing its scoop flag
//!   again before each pod, as `_ResetPlayerPrecalcedValues` runs after
//!   each scoop (@0x371ac), so a full hold takes no more;
//! - each NPC that is scooping and in the ship layer
//!   ([`npc_scoop_pods`](crate::scoop::npc_scoop_pods)).
//!
//! Each shown pod a taker touches, in slot order, is hidden, box slot 0 is
//! taken (its box, if live, is gone) and a ton of slot 0's last contents
//! goes into the taker's hold, as a box's would; nothing while slot 0's
//! scoopable byte is clear. A pod spawned this pass isn't tested until the
//! next, as the collision (@0x43e70) runs before `_HandleShip` spawns it
//! (@0x44483). No sound and no message: the handler has no `PlaySound` or
//! `_beepSnd`.
//!
//! A pod touches by its mask, from shän 895's sheet
//! ([`ESCAPE_POD_SHIP`](crate::combat::hull::ESCAPE_POD_SHIP)), which the
//! session asks for while any pod is adrift (see the `sprite_sheets`
//! module), or else by the ship's disc
//! ([`Scooping::touches_pod`](crate::scoop::Scooping::touches_pod)).

use std::rc::Rc;

use super::Session;
use crate::catalog::MaskSheet;
use crate::combat::hull::ESCAPE_POD_SHIP;
use crate::rulebook::{RuleKey, RuleSource};
use crate::scoop::{Scooper, Scooping, take_touching_pod};
use crate::sprite_mask::RleSheet;

impl Session {
    /// Whether a scooping ship takes a pod for box slot 0, by
    /// [`RuleKey::PodScoopsBoxZero`].
    pub(super) fn pods_scooped(&self) -> bool {
        self.setup.rules.source_for(RuleKey::PodScoopsBoxZero) == RuleSource::Engine
    }

    /// shän 895's sheet, the pods' mask, where it was read.
    pub(super) fn pod_sheet(&self) -> Option<Rc<RleSheet>> {
        let base = self.statics.arsenal.sprite_base(ESCAPE_POD_SHIP);
        self.space
            .sprite_masks
            .sheet(MaskSheet::Ship(base))
            .map(Rc::clone)
    }

    /// The player, `scooping`, takes each shown pod it touches while its
    /// scoop flag holds (see the module docs), the pods drawn from
    /// `sheet`.
    pub(super) fn player_scoop_pods(&mut self, scooping: &Scooping, sheet: Option<&Rc<RleSheet>>) {
        while self.player_scoops() {
            let Some(haul) = take_touching_pod(
                &mut self.space.boxes,
                &mut self.space.pods,
                sheet,
                scooping,
                Scooper::Player,
            ) else {
                break;
            };
            self.add_player_haul(haul);
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::cargo_box::SlotBytes;
    use crate::catalog::{JunkId, OutfitId};
    use crate::chance::NeverFires;
    use crate::combat::hull::Condition;
    use crate::escape_pod::{EscapePods, Pod};
    use crate::flight::Controls;
    use crate::geometry::Vec2;
    use crate::market::Good;
    use crate::rulebook::Rulebook;
    use crate::scoop::{BoxFrames, MOD_ASTEROID_SCOOP};
    use crate::stats::ShipStats;
    use crate::testkit::{FakePilotCatalog, catalog, npc, outfit, work, work_with};
    use crate::traffic::npc::{Npc, NpcHold, NpcId};

    const SCOOP: OutfitId = OutfitId(300);

    /// Where the far boxes are: out of every test ship's reach.
    const FAR: Vec2 = Vec2::new(2000.0, 0.0);

    /// The test catalog with the scoop outfit.
    fn scoopable() -> FakePilotCatalog {
        let mut catalog = catalog();
        catalog
            .outfits
            .push(outfit(SCOOP.0, &[(MOD_ASTEROID_SCOOP, 0)]));
        catalog
    }

    /// `catalog`'s session in flight at rest by `rule`, holding `tons` of
    /// food out of 20, owning the scoop when `scoop`.
    fn flying(catalog: &FakePilotCatalog, rule: RuleSource, tons: u32, scoop: bool) -> Session {
        let mut session = Session::start(catalog)
            .expect("starts")
            .with_rules(Rulebook::default().with_override(RuleKey::PodScoopsBoxZero, rule));
        session.populate(catalog, &mut NeverFires);
        session.pilot.cargo.insert(Good::Commodity(0), tons);
        if scoop {
            session.pilot.outfits.insert(SCOOP, 1);
        }
        session
    }

    /// A scooping player with an empty hold, by `rule`.
    fn scooper(rule: RuleSource) -> (FakePilotCatalog, Session) {
        let catalog = scoopable();
        let session = flying(&catalog, rule, 0, true);
        (catalog, session)
    }

    /// A shown pod at rest at `at`.
    fn pod_at(at: Vec2) -> Pod {
        Pod {
            position: at,
            velocity: Vec2::ZERO,
            life: 100.0,
            frame: 0,
            shown: true,
        }
    }

    /// Puts shown pods at rest at `at`.
    fn pods_at(session: &mut Session, at: &[Vec2]) {
        let pods: Vec<Pod> = at.iter().copied().map(pod_at).collect();
        session.space.pods = EscapePods::adrift(&pods);
    }

    /// Puts a shown pod at rest on the player's ship.
    fn pod_on_ship(session: &mut Session) {
        let at = session.player().position;
        pods_at(session, &[at]);
    }

    /// Drops a scoopable box holding `contents` at `at`.
    fn box_at(session: &mut Session, at: Vec2, contents: i16) {
        session
            .space
            .boxes
            .spawn_scoopable(at, Some(contents), 1, &mut NeverFires);
    }

    /// Every good held, with its tons, leaving out goods of none.
    fn held(session: &Session) -> Vec<(Good, u32)> {
        session
            .pilot()
            .cargo()
            .filter(|&(_, tons)| tons > 0)
            .collect()
    }

    fn shown(session: &Session) -> Vec<bool> {
        session.escape_pods().iter().map(|pod| pod.shown).collect()
    }

    fn contents(session: &Session) -> Vec<Option<i16>> {
        session
            .cargo_boxes()
            .iter()
            .map(|cargo_box| cargo_box.contents)
            .collect()
    }

    /// A pass of `session` on its catalog.
    fn pass(session: &mut Session, catalog: &FakePilotCatalog) {
        work(session, catalog, Controls::default());
    }

    #[test]
    fn a_scooping_player_on_a_pod_hides_it_and_hauls_live_box_slot_0() {
        for rule in [RuleSource::Engine, RuleSource::Bible] {
            let (catalog, mut session) = scooper(rule);
            box_at(&mut session, FAR, 3);
            pod_on_ship(&mut session);
            pass(&mut session, &catalog);
            if rule == RuleSource::Engine {
                assert_eq!(shown(&session), [false], "the pod hidden");
                assert!(session.cargo_boxes().is_empty(), "box slot 0 gone");
                assert_eq!(held(&session), [(Good::Commodity(3), 1)]);
            } else {
                assert_eq!(shown(&session), [true], "never scooped");
                assert_eq!(contents(&session), [Some(3)]);
                assert_eq!(held(&session), []);
            }
        }
    }

    #[test]
    fn a_dead_slot_0_last_scoopable_gives_a_free_ton() {
        for rule in [RuleSource::Engine, RuleSource::Bible] {
            let (catalog, mut session) = scooper(rule);
            box_at(&mut session, FAR, 3);
            // Slot 1's live box keeps the gate open.
            box_at(&mut session, FAR + Vec2::new(0.0, 500.0), 5);
            let disc = Scooping::disc(FAR, 1.0);
            let taken = session
                .space
                .boxes
                .take_touching(&disc, &BoxFrames::default());
            assert_eq!(taken.map(|made| made.contents), Some(Some(3)));
            pod_on_ship(&mut session);
            pass(&mut session, &catalog);
            if rule == RuleSource::Engine {
                assert_eq!(held(&session), [(Good::Commodity(3), 1)]);
                assert_eq!(shown(&session), [false]);
            } else {
                assert_eq!(held(&session), []);
                assert_eq!(shown(&session), [true]);
            }
            assert_eq!(contents(&session), [Some(5)], "nothing else changes");
        }
    }

    #[test]
    fn a_slot_0_last_jettisoned_takes_no_pod() {
        let (catalog, mut session) = scooper(RuleSource::Engine);
        let far = crate::flight::ShipState {
            position: FAR,
            ..session.ship.player
        };
        session.space.boxes.spawn(&far, None, &mut NeverFires);
        box_at(&mut session, FAR + Vec2::new(0.0, 500.0), 5);
        pod_on_ship(&mut session);
        pass(&mut session, &catalog);
        assert_eq!(held(&session), []);
        assert_eq!(shown(&session), [true]);
        assert_eq!(session.cargo_boxes().len(), 2);
    }

    #[test]
    fn no_live_scoopable_box_takes_no_pod() {
        let (catalog, mut session) = scooper(RuleSource::Engine);
        box_at(&mut session, FAR, 3);
        session.space.boxes.take_slot_zero();
        assert_eq!(
            session.space.boxes.slot_zero(),
            SlotBytes {
                scoopable: true,
                contents: Some(3)
            }
        );
        pod_on_ship(&mut session);
        pass(&mut session, &catalog);
        assert_eq!(held(&session), []);
        assert_eq!(shown(&session), [true], "the gate is shut");
    }

    #[test]
    fn a_hidden_pod_or_one_out_of_reach_is_not_taken() {
        let (catalog, mut session) = scooper(RuleSource::Engine);
        box_at(&mut session, FAR, 3);
        let at = session.player().position;
        pods_at(&mut session, &[at, at + Vec2::new(300.0, 0.0)]);
        session.space.pods.hide(0);
        pass(&mut session, &catalog);
        assert_eq!(held(&session), []);
        assert_eq!(shown(&session), [false, true]);
        assert_eq!(contents(&session), [Some(3)]);
    }

    #[test]
    fn taking_box_0_and_a_pod_in_one_pass_gains_two_tons() {
        let (catalog, mut session) = scooper(RuleSource::Engine);
        let at = session.player().position;
        box_at(&mut session, at, 3);
        pods_at(&mut session, &[at]);
        pass(&mut session, &catalog);
        assert_eq!(held(&session), [(Good::Commodity(3), 2)]);
        assert_eq!(shown(&session), [false]);
    }

    #[test]
    fn each_pod_takes_slot_0_again_until_the_hold_is_full() {
        let catalog = scoopable();
        let mut session = flying(&catalog, RuleSource::Engine, 18, true);
        box_at(&mut session, FAR, 3);
        let at = session.player().position;
        pods_at(&mut session, &[at, at, at]);
        pass(&mut session, &catalog);
        assert_eq!(
            held(&session),
            [(Good::Commodity(0), 18), (Good::Commodity(3), 2)]
        );
        assert_eq!(shown(&session), [false, false, true], "the third left");
    }

    #[test]
    fn a_full_hold_or_no_scoop_takes_no_pod() {
        let catalog = scoopable();
        for (tons, scoop, why) in [(20, true, "a full hold"), (0, false, "no scoop")] {
            let mut session = flying(&catalog, RuleSource::Engine, tons, scoop);
            box_at(&mut session, FAR, 3);
            pod_on_ship(&mut session);
            pass(&mut session, &catalog);
            assert_eq!(shown(&session), [true], "{why}");
            assert_eq!(contents(&session), [Some(3)], "{why}");
        }
    }

    #[test]
    fn a_disabled_player_takes_no_pod() {
        let (_, mut session) = scooper(RuleSource::Engine);
        session.ship.condition = Condition::Disabled;
        box_at(&mut session, FAR, 3);
        pod_on_ship(&mut session);
        work_with(
            &mut session,
            &catalog(),
            Controls::default(),
            &crate::ai::Peaceful,
            crate::session::tests::DISABLING,
            &mut NeverFires,
        );
        assert_eq!(session.ship.condition, Condition::Disabled);
        assert_eq!(shown(&session), [true]);
        assert_eq!(held(&session), []);
    }

    #[test]
    fn a_junk_slot_0_hauls_junk_for_the_player() {
        let (catalog, mut session) = scooper(RuleSource::Engine);
        box_at(&mut session, FAR, 1018);
        pod_on_ship(&mut session);
        pass(&mut session, &catalog);
        assert_eq!(held(&session), [(Good::Junk(JunkId(146)), 1)]);
    }

    #[test]
    fn a_pod_scoop_sounds_nothing() {
        let (catalog, mut session) = scooper(RuleSource::Engine);
        let (_, mut twin) = scooper(RuleSource::Engine);
        for session in [&mut session, &mut twin] {
            box_at(session, FAR, 3);
            let _ = session.take_sounds();
        }
        pod_on_ship(&mut session);
        pass(&mut session, &catalog);
        pass(&mut twin, &catalog);
        assert_eq!(shown(&session), [false]);
        assert_eq!(session.take_sounds(), twin.take_sounds());
    }

    /// A scooping NPC, a miner in state 17, at `at`, its disc 10 across
    /// its centre, its hold empty.
    fn scooping_npc(at: Vec2) -> Npc {
        let mut npc = npc(1, ShipStats::default());
        npc.goal = crate::ai::Goal::Scoop;
        npc.state.position = at;
        npc.hull.hit_radius = 10.0;
        npc
    }

    fn hold_of(session: &Session, id: NpcId) -> NpcHold {
        session.space.npc(id).expect("here").cargo
    }

    #[test]
    fn a_scooping_npc_takes_a_pod_into_its_own_hold() {
        for rule in [RuleSource::Engine, RuleSource::Bible] {
            let catalog = scoopable();
            let mut session = flying(&catalog, rule, 0, false);
            let at = Vec2::new(300.0, 0.0);
            let id = session.space.traffic.add_npc(scooping_npc(at));
            box_at(&mut session, FAR, 2);
            pods_at(&mut session, &[at]);
            pass(&mut session, &catalog);
            let hold = hold_of(&session, id);
            if rule == RuleSource::Engine {
                assert_eq!((hold.tons(2), hold.total()), (1, 1));
                assert_eq!(shown(&session), [false]);
                assert!(session.cargo_boxes().is_empty());
            } else {
                assert_eq!(hold, NpcHold::default());
                assert_eq!(shown(&session), [true]);
            }
            assert_eq!(held(&session), [], "the player's hold is untouched");
        }
    }

    #[test]
    fn a_scooping_npc_takes_a_junk_slot_0_for_nothing_but_still_hides_the_pod() {
        let catalog = scoopable();
        let mut session = flying(&catalog, RuleSource::Engine, 0, false);
        let at = Vec2::new(300.0, 0.0);
        let id = session.space.traffic.add_npc(scooping_npc(at));
        box_at(&mut session, FAR, 1018);
        pods_at(&mut session, &[at]);
        pass(&mut session, &catalog);
        assert_eq!(hold_of(&session, id), NpcHold::default());
        assert_eq!(shown(&session), [false]);
        assert!(session.cargo_boxes().is_empty(), "box 0 still dies");
    }

    #[test]
    fn the_players_escort_never_takes_a_pod() {
        let catalog = scoopable();
        let mut session = flying(&catalog, RuleSource::Engine, 0, false);
        let at = Vec2::new(300.0, 0.0);
        let escort = Npc {
            escort: Some(crate::escort::EscortDuty {
                formation: crate::escort::FormationSlot::default(),
                order: None,
            }),
            ..scooping_npc(at)
        };
        let id = session.space.traffic.add_npc(escort);
        box_at(&mut session, FAR, 2);
        pods_at(&mut session, &[at]);
        pass(&mut session, &catalog);
        assert_eq!(hold_of(&session, id), NpcHold::default());
        assert_eq!(shown(&session), [true]);
    }

    #[test]
    fn the_player_takes_a_pod_before_a_scooping_npc_on_it() {
        let (catalog, mut session) = scooper(RuleSource::Engine);
        let at = session.player().position;
        let id = session.space.traffic.add_npc(scooping_npc(at));
        box_at(&mut session, FAR, 2);
        pods_at(&mut session, &[at]);
        pass(&mut session, &catalog);
        assert_eq!(held(&session), [(Good::Commodity(2), 1)]);
        assert_eq!(hold_of(&session, id), NpcHold::default());
    }

    #[test]
    fn the_scoop_asks_for_shan_895s_sheet_while_a_pod_is_adrift() {
        let (catalog, mut session) = scooper(RuleSource::Engine);
        pass(&mut session, &catalog);
        let base = session.statics.arsenal.sprite_base(ESCAPE_POD_SHIP);
        let asked = |catalog: &FakePilotCatalog| {
            catalog
                .rle_sheets_asked
                .borrow()
                .contains(&MaskSheet::Ship(base))
        };
        assert!(!asked(&catalog), "no pod, no ask");
        pods_at(&mut session, &[FAR]);
        pass(&mut session, &catalog);
        assert!(asked(&catalog));
    }

    /// A sheet of 36 frames `side` square, each drawn wholly as `mark`.
    fn whole_frames(side: usize, mark: char) -> RleSheet {
        let row: String = std::iter::repeat_n(mark, side).collect();
        let rows: Vec<&str> = vec![row.as_str(); side];
        let frames: Vec<&[&str]> = vec![rows.as_slice(); 36];
        nova_sim_base::fixture::rle::sheet(&frames)
    }

    #[test]
    fn a_pod_is_taken_only_where_its_mask_touches_the_players() {
        for (mark, taken) in [('#', true), ('.', false)] {
            let mut catalog = scoopable();
            let session = flying(&catalog, RuleSource::Engine, 0, true);
            let arsenal = &session.statics.arsenal;
            let (player, pod) = (
                arsenal.sprite_base(session.pilot.ship),
                arsenal.sprite_base(ESCAPE_POD_SHIP),
            );
            catalog.rle_sheets = vec![
                (MaskSheet::Ship(player), whole_frames(4, '#')),
                (MaskSheet::Ship(pod), whole_frames(8, mark)),
            ];
            let mut session = flying(&catalog, RuleSource::Engine, 0, true);
            box_at(&mut session, FAR, 3);
            pod_on_ship(&mut session);
            // The first pass asks for the sheets after the pods' spawn
            // would; the second meets the pod by them.
            session.load_sprite_masks(&catalog);
            assert!(session.pod_sheet().is_some(), "read");
            pass(&mut session, &catalog);
            assert_eq!(shown(&session), [!taken], "a pod drawn as {mark}");
        }
    }
}
