//! The session's hover test (see the [`hover`](crate::hover) module): the
//! ship under the pointer, then, with none and no jump under way, the
//! stellar.

use super::Session;
use crate::combat::hull::Condition;
use crate::geometry::Vec2;
use crate::hover::{DrawnShips, Hover, hover_gate, in_box, ship_under};
use crate::navigation::{NavMode, stellar_under, stellar_under_past};
use crate::sound::SimSound;

impl Session {
    /// What the pointer at `at`, in system coordinates, is over among the
    /// things the player can target, the ships boxed as `drawn` draws them
    /// (`_HandleMouse` @0x380e3-0x3828d): a ship ([`ship_under`], the
    /// player's own only while it has a target and, past the `_IsDying`
    /// skip @0x380f7 that comes before the slot-0 target rule
    /// @0x381c3-0x381d5, is intact or disabled), or else, unless a jump is
    /// under way, the pre-jump stage included (the player's +0x50 > 0,
    /// @0x381f2), the stellar a click there would pick ([`stellar_under`]).
    #[must_use]
    pub fn hover(&self, at: Vec2, drawn: &impl DrawnShips) -> Option<Hover> {
        ship_under(
            self.npcs(),
            self.scanner(),
            self.target.is_some()
                && matches!(self.condition, Condition::Intact | Condition::Disabled),
            at,
            drawn,
        )
        .or_else(|| {
            self.jump.is_none().then(|| {
                stellar_under(&self.sites, at, |site| self.drawn_frame_size(site))
                    .map(Hover::Stellar)
            })?
        })
    }

    /// A left click at `at`, in system coordinates, in space, the ships
    /// boxed as `drawn` draws them, as the original's click block
    /// (`_HandlePlayer` @0x6a000-0x6a764) takes it once the view has ruled
    /// out the status panel (@0x6a065, @0x6a36d) and the open escort menu
    /// (@0x6a060-0x6a12b), and gives what it took: the ship it targeted,
    /// [`Hover::Player`] when it let go of the target, or the stellar it
    /// selected. `None`, and nothing changes, while landed, or when the
    /// click changes nothing. The press-only latch (`0xdede8`,
    /// @0x6a000-0x6a01b, @0x6a764) is the view's press event.
    ///
    /// **Ships** (@0x6a353-0x6a5e4), during a jump too, as the block
    /// reads no jump state until the stellars: a click on the player's own
    /// ship lets go of the target (+0x70 := −1, @0x6a3fc), whether or not
    /// there is one; then the first NPC, in slot order, that the hover
    /// test would take ([`hover_gate`]: intact or disabled, not cloaked
    /// unless the scanner targets cloaked ships or it escorts the player,
    /// of a targetable type, @0x6a402-0x6a50c) and whose box holds the
    /// click ([`in_box`]) becomes the target (@0x6a347). When the target
    /// changed (@0x6a51c) the click beeps, [`SimSound::TargetSelected`]
    /// (`_beepSnd[2]`, 152) with a target and [`SimSound::TargetCleared`]
    /// (`_beepSnd[0]`, 150) without (@0x6a53a-0x6a585), and a new target's
    /// brackets fly in when its sprite is on screen (@0x6a5ac-0x6a5e4).
    /// That beep sits behind the ship's +0x6e word being 1 (@0x6a526),
    /// which it always is ([`Session::select_target`]), so the gate is not
    /// modelled.
    ///
    /// **Stellars** (@0x6a5e9-0x6a717), only when the target did not
    /// change and no jump is under way (+0x50 > 0, @0x6a5ef): the first
    /// stellar under the click, the one selected in mode 2 passed over so
    /// the walk goes on to any under it (@0x6a69d → @0x6a703), becomes the
    /// selection in mode 2 with [`SimSound::NavSelected`] (`_beepSnd[0]`,
    /// @0x6a6c1), its brackets flying in and the dock request dropped
    /// (@0x6a6c6-0x6a6fc). A click on none is silent.
    ///
    /// Left out: the floating map's clicks (@0x6a130-0x6a342,
    /// @0x6a719-0x6a762, hud-fidelity phase 24, as the engine draws no
    /// floating map), the hidden-stellar skip (spob +0x44, @0x6a632,
    /// task nav-selection-follow-up) and the box's shape (stellar-fidelity
    /// phase 9).
    pub fn click_in_space(&mut self, at: Vec2, drawn: &impl DrawnShips) -> Option<Hover> {
        if self.landed.is_some() {
            return None;
        }
        let before = self.target;
        let mut took = None;
        if drawn.player().is_some_and(|ship| in_box(at, ship)) {
            self.target = None;
            took = Some(Hover::Player);
        }
        let scanner = self.scanner();
        if let Some(id) = self
            .npcs()
            .iter()
            .filter(|npc| hover_gate(npc, scanner))
            .find(|npc| drawn.npc(npc.id).is_some_and(|ship| in_box(at, ship)))
            .map(|npc| npc.id)
        {
            self.target = Some(id);
            took = Some(Hover::Ship(id));
        }
        if self.target != before {
            self.sounds.push(if self.target.is_some() {
                SimSound::TargetSelected
            } else {
                SimSound::TargetCleared
            });
            self.retargeted(before);
            return took;
        }
        if self.jump.is_some() {
            return None;
        }
        let selected = self.nav.stellar();
        let picked = stellar_under_past(&self.sites, at, selected, |site| {
            self.drawn_frame_size(site)
        })?;
        self.sounds.push(SimSound::NavSelected);
        self.nav_fly_in.start();
        self.nav = NavMode::Stellar(Some(picked));
        self.dock = None;
        Some(Hover::Stellar(picked))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{StellarId, SystemId};
    use crate::hover::DrawnBox;
    use crate::session::Jump;
    use crate::sound::SimSound;
    use crate::stats::ShipStats;
    use crate::testkit::{catalog, npc};
    use crate::traffic::npc::NpcId;

    /// Every ship drawn in a 60-pixel square at `at`.
    struct AllAt(Vec2);

    impl DrawnShips for AllAt {
        fn npc(&self, _id: NpcId) -> Option<DrawnBox> {
            Some(self.square())
        }

        fn player(&self) -> Option<DrawnBox> {
            Some(self.square())
        }
    }

    impl AllAt {
        fn square(&self) -> DrawnBox {
            DrawnBox {
                centre: self.0,
                width: 60,
                height: 60,
            }
        }
    }

    /// Planet 128, at (30, -40), 100 square.
    const OVER_128: Vec2 = Vec2::new(30.0, -40.0);

    #[test]
    fn with_no_ship_under_the_pointer_a_stellar_is_hovered_but_not_during_a_jump() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        let elsewhere = AllAt(Vec2::new(5000.0, 5000.0));
        assert_eq!(
            session.hover(OVER_128, &elsewhere),
            Some(Hover::Stellar(StellarId(128)))
        );
        assert_eq!(session.hover(Vec2::new(500.0, 500.0), &elsewhere), None);
        session.jump = Some(Jump::PreJump {
            to: SystemId(131),
            bearing: None,
            warp: None,
        });
        assert_eq!(session.hover(OVER_128, &elsewhere), None, "pre-jump");
        session.jump = Some(Jump::Hyperspace {
            to: SystemId(131),
            warp: 0,
        });
        assert_eq!(session.hover(OVER_128, &elsewhere), None, "jumping");
    }

    #[test]
    fn a_ship_under_the_pointer_comes_before_the_stellar() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        let id = session.traffic.add_npc(npc(0, ShipStats::default()));
        assert_eq!(
            session.hover(OVER_128, &AllAt(OVER_128)),
            Some(Hover::Ship(id))
        );
    }

    #[test]
    fn the_players_ship_is_hovered_while_it_holds_a_target() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        let far = Vec2::new(-5000.0, 0.0);
        assert_eq!(session.hover(far, &AllAt(far)), None);
        session.target = Some(NpcId(99));
        assert_eq!(session.hover(far, &AllAt(far)), Some(Hover::Player));
    }

    #[test]
    fn a_dying_or_destroyed_players_ship_is_not_hovered_though_it_holds_a_target() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        let far = Vec2::new(-5000.0, 0.0);
        session.target = Some(NpcId(99));
        session.condition = Condition::Disabled;
        assert_eq!(session.hover(far, &AllAt(far)), Some(Hover::Player));
        session.condition = Condition::Dying {
            ticks_left: 3,
            starting: false,
        };
        assert_eq!(session.hover(far, &AllAt(far)), None, "dying");
        session.condition = Condition::Destroyed;
        assert_eq!(session.hover(far, &AllAt(far)), None, "destroyed");
    }

    // A click in space.

    /// The view's boxes, canned: each NPC listed, and the player's ship.
    #[derive(Default)]
    struct Drawn {
        npcs: std::collections::BTreeMap<NpcId, DrawnBox>,
        player: Option<DrawnBox>,
    }

    impl DrawnShips for Drawn {
        fn npc(&self, id: NpcId) -> Option<DrawnBox> {
            self.npcs.get(&id).copied()
        }

        fn player(&self) -> Option<DrawnBox> {
            self.player
        }
    }

    /// A 60-pixel square at `at`, too tall for the click margin.
    fn square(at: Vec2) -> DrawnBox {
        DrawnBox {
            centre: at,
            width: 60,
            height: 60,
        }
    }

    impl Drawn {
        /// Also draws NPC `id` at `at`.
        fn with(mut self, id: NpcId, at: Vec2) -> Self {
            self.npcs.insert(id, square(at));
            self
        }

        /// Also draws the player's ship at `at`.
        fn player_at(mut self, at: Vec2) -> Self {
            self.player = Some(square(at));
            self
        }
    }

    /// Off every stellar of the test catalogue.
    const SPACE: Vec2 = Vec2::new(-5000.0, 5000.0);
    /// Where the player's ship is drawn in these tests, off every stellar.
    const PLAYER_AT: Vec2 = Vec2::new(-5000.0, 0.0);

    /// A started session with `ships` NPCs, its sounds taken.
    fn flying(ships: u32) -> (Session, Vec<NpcId>) {
        let mut session = Session::start(&catalog()).expect("starts");
        let ids = (0..ships)
            .map(|n| session.traffic.add_npc(npc(n, ShipStats::default())))
            .collect();
        session.take_sounds();
        (session, ids)
    }

    fn started_fly_in() -> crate::brackets::FlyIn {
        let mut fly_in = crate::brackets::FlyIn::default();
        fly_in.start();
        fly_in
    }

    #[test]
    fn a_click_on_a_ship_targets_it_with_the_target_beep_and_its_fly_in() {
        let (mut session, ids) = flying(1);
        let drawn = Drawn::default().with(ids[0], SPACE);
        assert_eq!(
            session.click_in_space(SPACE, &drawn),
            Some(Hover::Ship(ids[0]))
        );
        assert_eq!(session.target, Some(ids[0]));
        assert_eq!(session.take_sounds(), [SimSound::TargetSelected]);
        assert_eq!(session.target_fly_in, started_fly_in());
    }

    #[test]
    fn a_click_on_the_players_ship_lets_go_of_the_target_with_the_clear_beep() {
        let (mut session, ids) = flying(1);
        session.target = Some(ids[0]);
        let drawn = Drawn::default().player_at(PLAYER_AT);
        assert_eq!(
            session.click_in_space(PLAYER_AT, &drawn),
            Some(Hover::Player)
        );
        assert_eq!(session.target, None);
        assert_eq!(session.take_sounds(), [SimSound::TargetCleared]);
    }

    #[test]
    fn a_click_that_changes_nothing_is_silent() {
        let (mut session, ids) = flying(1);
        let drawn = Drawn::default()
            .with(ids[0], Vec2::new(5000.0, 5000.0))
            .player_at(PLAYER_AT);
        assert_eq!(session.click_in_space(SPACE, &drawn), None, "space");
        assert_eq!(
            session.click_in_space(PLAYER_AT, &drawn),
            None,
            "own ship, no target"
        );
        assert_eq!(session.target, None);
        session.target = Some(ids[0]);
        assert_eq!(
            session.click_in_space(Vec2::new(5000.0, 5000.0), &drawn),
            None,
            "the target again"
        );
        assert_eq!(session.target, Some(ids[0]));
        assert_eq!(session.take_sounds(), []);
        assert_eq!(session.nav_mode(), crate::NavMode::Off);
    }

    #[test]
    fn a_click_on_the_target_drawn_over_a_stellar_selects_the_stellar() {
        let (mut session, ids) = flying(1);
        session.target = Some(ids[0]);
        let drawn = Drawn::default().with(ids[0], OVER_128);
        assert_eq!(
            session.click_in_space(OVER_128, &drawn),
            Some(Hover::Stellar(StellarId(128)))
        );
        assert_eq!(session.target, Some(ids[0]), "kept");
        assert_eq!(session.nav_target(), Some(StellarId(128)));
        assert_eq!(session.take_sounds(), [SimSound::NavSelected]);
    }

    #[test]
    fn a_click_on_another_ship_drawn_over_a_stellar_targets_the_ship_only() {
        let (mut session, ids) = flying(1);
        let drawn = Drawn::default().with(ids[0], OVER_128);
        assert_eq!(
            session.click_in_space(OVER_128, &drawn),
            Some(Hover::Ship(ids[0]))
        );
        assert_eq!(session.nav_mode(), crate::NavMode::Off, "nav unchanged");
        assert_eq!(session.take_sounds(), [SimSound::TargetSelected]);
    }

    #[test]
    fn a_click_passes_over_a_cloaked_ship_without_the_scanner_and_takes_it_with_it() {
        use crate::cloak::{Cloak, FADE_FULL, MOD_SCANNER, SCAN_TARGET};
        let (mut session, ids) = flying(2);
        session.traffic.npcs_mut()[0].cloak = Cloak {
            fade: FADE_FULL,
            direction: 0,
        };
        let drawn = Drawn::default().with(ids[0], SPACE).with(ids[1], SPACE);
        assert_eq!(
            session.click_in_space(SPACE, &drawn),
            Some(Hover::Ship(ids[1])),
            "passed over"
        );
        session.target = None;
        session.outfits = vec![crate::testkit::outfit(250, &[(MOD_SCANNER, SCAN_TARGET)])];
        session
            .pilot
            .outfits
            .insert(crate::catalog::OutfitId(250), 1);
        assert_eq!(
            session.click_in_space(SPACE, &drawn),
            Some(Hover::Ship(ids[0])),
            "with the scanner"
        );
    }

    #[test]
    fn of_two_overlapping_ships_the_first_takes_the_click() {
        let (mut session, ids) = flying(2);
        let drawn = Drawn::default().with(ids[1], SPACE).with(ids[0], SPACE);
        assert_eq!(
            session.click_in_space(SPACE, &drawn),
            Some(Hover::Ship(ids[0]))
        );
    }

    #[test]
    fn during_a_jump_a_click_targets_a_ship_but_picks_no_stellar() {
        for jump in [
            Jump::PreJump {
                to: SystemId(131),
                bearing: None,
                warp: None,
            },
            Jump::Hyperspace {
                to: SystemId(131),
                warp: 0,
            },
        ] {
            let (mut session, ids) = flying(1);
            session.jump = Some(jump);
            let drawn = Drawn::default().with(ids[0], SPACE);
            assert_eq!(session.click_in_space(OVER_128, &drawn), None, "{jump:?}");
            assert_eq!(session.nav_mode(), crate::NavMode::Off, "{jump:?}");
            assert_eq!(session.take_sounds(), [], "{jump:?}");
            assert_eq!(
                session.click_in_space(SPACE, &drawn),
                Some(Hover::Ship(ids[0])),
                "{jump:?}"
            );
            assert_eq!(
                session.take_sounds(),
                [SimSound::TargetSelected],
                "{jump:?}"
            );
        }
    }

    #[test]
    fn a_click_on_the_stellar_selected_goes_on_to_one_under_it() {
        let mut catalog = catalog();
        catalog.sites[0].1 = vec![
            crate::testkit::planet(131, 0.0, 0.0),
            crate::testkit::planet(128, 20.0, 0.0),
        ];
        let mut session = Session::start(&catalog).expect("starts");
        let none = Drawn::default();
        session.nav = crate::NavMode::Stellar(Some(StellarId(131)));
        session.take_sounds();
        assert_eq!(
            session.click_in_space(Vec2::new(10.0, 0.0), &none),
            Some(Hover::Stellar(StellarId(128)))
        );
        assert_eq!(session.nav_target(), Some(StellarId(128)));
        assert_eq!(session.take_sounds(), [SimSound::NavSelected]);
        assert_eq!(
            session.click_in_space(Vec2::new(60.0, 0.0), &none),
            None,
            "only the one selected under the click"
        );
        assert_eq!(session.nav_target(), Some(StellarId(128)));
        assert_eq!(session.take_sounds(), []);
    }

    #[test]
    fn a_click_while_landed_does_nothing() {
        let (mut session, ids) = flying(1);
        session.landed = Some(StellarId(128));
        let drawn = Drawn::default().with(ids[0], SPACE);
        assert_eq!(session.click_in_space(SPACE, &drawn), None);
        assert_eq!(session.click_in_space(OVER_128, &drawn), None);
        assert_eq!(session.target, None);
        assert_eq!(session.nav_mode(), crate::NavMode::Off);
        assert_eq!(session.take_sounds(), []);
    }
}
