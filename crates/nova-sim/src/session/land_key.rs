//! The land key held, `_HandlePlayer`'s L block (@0x6a8ac-0x6a97f). All
//! addresses are in `EV Nova.app`'s `EV Nova` (i386).
//!
//! The block runs on every pass L reads down (`_QuickKeyCheck` @0x6a8b8)
//! with the death countdown +0x3c (@0x6a8cb), the pod countdown +0xc908
//! (@0x6a8d5) and the jump counter +0x50 (@0x6a8df) all at most 0. With
//! no stellar targeted in nav mode 2 it picks the nearest
//! (`_FindNearestStellar` @0x6a901) and on a find selects it in mode 2
//! with the fly-in (@0x6a911-0x6a932), silently. Only then does it test
//! `_dockTryFlag` (0xdee34) for the dock request (@0x6a938-0x6a956), which
//! the view makes on its own read of L, its latch clear
//! ([`Session::land`]). So the pick happens on every pass L is held,
//! latch or not: a target lost meanwhile (a take-off letting the
//! selection go, a stellar destroyed) is picked again while L stays down.
//!
//! [`Session::land_key_pass`] is that pick, in the player's part of each
//! pass, right before the dock approach (@0x6a97a), both of double time's
//! passes. The pod returns before it (`Session::player_move`, @0x68ddd).
//! The pick is [`land_key_pick`], the press's own.

use super::{Chart, Session};
use crate::catalog::LandingSite;
use crate::combat::hull::Condition;
use crate::landing::land_key_pick;
use crate::rulebook::RuleKey;

impl Session {
    /// One pass of the land key held (see the module docs): with L down
    /// ([`Session::set_land_key`]), no jump under way, the ship neither
    /// breaking up nor destroyed and no stellar targeted, the nearest
    /// selected in mode 2 with the brackets' fly-in, in silence and with
    /// no dock request.
    pub(super) fn land_key_pass(&mut self) {
        if !self.port.land_key || self.mode.jump().is_some() {
            return;
        }
        if matches!(
            self.ship.condition,
            Condition::Dying { .. } | Condition::Destroyed
        ) {
            return;
        }
        let candidates = self.chart.pick_candidates(&self.pilot, &self.statics);
        let gates = self.setup.rules.source_for(RuleKey::GatePickRange);
        let picked = land_key_pick(
            self.chart.land_key_target(),
            self.ship.player.position,
            &candidates,
            gates,
        )
        .ok()
        .and_then(|(_, selected)| selected);
        self.chart.select_pressed(picked);
    }
}

impl Chart {
    /// The stellar the land key reads as its target: the one held in nav
    /// mode 2, wherever the system lists it (`_HandlePlayer` @0x6a8ed-0x6a8f9).
    pub(super) fn land_key_target(&self) -> Option<&LandingSite> {
        self.nav_target()
            .and_then(|id| self.listed.iter().find(|site| site.id == id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::StellarId;
    use crate::chance::NeverFires;
    use crate::combat::Rules;
    use crate::flight::Controls;
    use crate::flight_frame::{StepInput, StepReport};
    use crate::navigation::NavMode;
    use crate::session::StepCollaborators;
    use crate::session::tests::three_stellars;
    use crate::testkit::{FakePilotCatalog, work};

    /// A session of `catalog` in flight, its nav as `nav`, its messages
    /// and sounds taken.
    fn flying(catalog: &FakePilotCatalog, nav: NavMode) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.chart.nav = nav;
        let _ = session.take_messages();
        let _ = session.take_sounds();
        session
    }

    /// One pass with L held.
    fn held_pass(session: &mut Session, catalog: &FakePilotCatalog) {
        session.set_land_key(true);
        work(session, catalog, Controls::default());
    }

    #[test]
    fn the_land_key_held_with_no_stellar_targeted_picks_the_nearest_in_silence() {
        let catalog = three_stellars();
        let mut session = flying(&catalog, NavMode::Stellar(None));
        held_pass(&mut session, &catalog);
        assert_eq!(session.nav_mode(), NavMode::Stellar(Some(StellarId(128))));
        let brackets = session.nav_brackets().expect("bracketed");
        assert!(brackets.expand > 0.0, "the fly-in started");
        assert_eq!(session.take_messages(), [], "silent");
        assert_eq!(session.take_sounds(), [], "silent");
        assert_eq!(session.dock_request(), None, "no request");
    }

    #[test]
    fn it_picks_from_hyperspace_mode_too() {
        // @0x6a8ed: nav mode other than 2 picks.
        let catalog = three_stellars();
        for nav in [NavMode::Off, NavMode::Hyperspace(None)] {
            let mut session = flying(&catalog, nav);
            held_pass(&mut session, &catalog);
            assert_eq!(
                session.nav_mode(),
                NavMode::Stellar(Some(StellarId(128))),
                "{nav:?}"
            );
        }
    }

    #[test]
    fn it_keeps_a_stellar_already_targeted() {
        let catalog = three_stellars();
        let far = NavMode::Stellar(Some(StellarId(129)));
        let mut session = flying(&catalog, far);
        held_pass(&mut session, &catalog);
        assert_eq!(session.nav_mode(), far);
    }

    #[test]
    fn it_picks_nothing_with_the_key_up() {
        let catalog = three_stellars();
        let mut session = flying(&catalog, NavMode::Off);
        session.set_land_key(false);
        work(&mut session, &catalog, Controls::default());
        assert_eq!(session.nav_mode(), NavMode::Off);
    }

    #[test]
    fn it_picks_nothing_during_a_jump_or_breaking_up() {
        let catalog = three_stellars();
        let mut session = flying(&catalog, NavMode::Off);
        session.ship.condition = Condition::Dying {
            ticks_left: 10,
            starting: false,
        };
        session.set_land_key(true);
        session.land_key_pass();
        assert_eq!(session.nav_mode(), NavMode::Off, "breaking up");
        let mut session = flying(&catalog, NavMode::Off);
        session.ship.condition = Condition::Destroyed;
        session.set_land_key(true);
        session.land_key_pass();
        assert_eq!(session.nav_mode(), NavMode::Off, "destroyed");
        let mut session = crate::session::tests::bound_for_131(
            &catalog,
            crate::geometry::Vec2::new(0.0, -10_000.0),
            crate::geometry::Vec2::ZERO,
            0.0,
        );
        session.begin_jump().expect("begins");
        let before = session.nav_mode();
        session.set_land_key(true);
        session.land_key_pass();
        assert_eq!(session.nav_mode(), before, "jumping");
    }

    #[test]
    fn it_picks_nothing_with_nothing_landable() {
        let mut catalog = three_stellars();
        catalog.sites[0].1.clear();
        let mut session = flying(&catalog, NavMode::Off);
        held_pass(&mut session, &catalog);
        assert_eq!(session.nav_mode(), NavMode::Off);
        assert_eq!(session.take_sounds(), []);
    }

    #[test]
    fn the_step_input_holds_the_land_key_for_the_pass() {
        let catalog = three_stellars();
        let mut session = flying(&catalog, NavMode::Off);
        let behaviour = crate::ai::NovaAi::default();
        let mut chance = NeverFires;
        let mut collab = StepCollaborators {
            catalog: &catalog,
            behaviour: &behaviour,
            rules: Rules::default(),
            chance: &mut chance,
        };
        let input = StepInput {
            land_key: true,
            ..StepInput::default()
        };
        session.step(&input, 1, &mut collab, &mut StepReport::default());
        assert_eq!(session.nav_mode(), NavMode::Stellar(Some(StellarId(128))));
    }
}
