//! The player's jettison chord held, `_HandlePlayer`'s Command-K block
//! (@0x6fd54-0x6fdaf). All addresses are in `EV Nova.app`'s `EV Nova`
//! (i386).
//!
//! The block has no latch: Command (0x37) and the jettison key (+0x1e)
//! down are its only gates, so it runs `_JettisonCargo(!Option)`
//! (@0x6fda1), the status redraw and `_ResetPlayerPrecalcedValues`
//! (@0x6fdaf) on every pass the chord is down, both of double time's. It
//! lies between the eject (@0x6f29e) and the self-destruct (@0x6fdb4), so
//! each pass's work runs it between [`Session::eject_check`] and
//! [`Session::self_destruct_tick`]. The view holds the chord with
//! [`Session::hold_jettison`] ([`StepInput::jettison`]).
//!
//! **Nothing aboard.** With no goods, no `jünk` and, with `all`, no
//! abortable mission cargo aboard, `_JettisonCargo` (@0x3dffd-0x3e3c4)
//! shows no failure (@0x3e17d), spawns no box (@0x3e282-0x3e2c0), plays no
//! sound (@0x3e306-0x3e33d) and says nothing (@0x3e3b0): only its reset
//! runs (@0x3e3b5). Ours' [`Session::jettison_cargo`] also begins a docked
//! run, letting go of an interface load a script queued, and marks a save
//! due; so a pass with nothing to jettison runs the reset alone.
//!
//! The pod (its passes return @0x68ddd before any key block) and a
//! destroyed ship (the dead branch, 0x6870d-0x6888c) jettison nothing, as
//! the self-destruct's countdown stands still there.
//!
//! [`StepInput::jettison`]: crate::flight_frame::StepInput::jettison

use super::{PlayerShip, Session};
use crate::chance::Chance;
use crate::combat::hull::Condition;
use crate::mission::cargo::jettison;

impl Session {
    /// Holds the jettison chord, Command and K down together, `Some(all)`
    /// with `all` unless Option is down too, or lets it go (`None`): what
    /// each pass's jettison reads (see the module docs).
    pub(crate) fn hold_jettison(&mut self, held: Option<bool>) {
        self.ship.hold_jettison(held);
    }

    /// One pass of the chord: while it is held, the cargo jettisoned
    /// ([`Session::jettison_cargo`]), drawing on `chance`, or with nothing
    /// to jettison the reset alone (see the module docs).
    pub(super) fn held_jettison_tick(&mut self, chance: &mut dyn Chance) {
        let Some(all) = self.ship.fate.jettison_held else {
            return;
        };
        if self.pod.is_some() || self.ship.condition == Condition::Destroyed {
            return;
        }
        if self.jettisons_something(all) {
            self.jettison_cargo(all, chance);
        } else {
            self.reset_precalced();
        }
    }

    /// Whether a jettison, `all` or not, would take anything: goods or
    /// `jünk` held, or with `all` an abortable mission's cargo aboard.
    fn jettisons_something(&self, all: bool) -> bool {
        let decided = jettison(
            &self.pilot.missions,
            |id| self.statics.lifecycle_record(id),
            all,
            self.pilot.goods_held(),
        );
        decided.total > 0 || !decided.slots.is_empty()
    }
}

impl PlayerShip {
    /// Holds the jettison chord, or lets it go (see
    /// [`Session::hold_jettison`]).
    fn hold_jettison(&mut self, held: Option<bool>) {
        self.fate.jettison_held = held;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chance::NeverFires;
    use crate::combat::Rules;
    use crate::flight::Controls;
    use crate::flight_frame::{StepInput, StepReport};
    use crate::market::Good;
    use crate::message::SimMessage;
    use crate::session::StepCollaborators;
    use crate::sound::SimSound;
    use crate::testkit::{FakePilotCatalog, catalog, work};

    /// A session of `catalog` in flight, holding `tons` of food.
    fn flying(catalog: &FakePilotCatalog, tons: u32) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.populate(catalog, &mut NeverFires);
        session.pilot.cargo.insert(Good::Commodity(0), tons);
        let _ = session.take_messages();
        let _ = session.take_sounds();
        session
    }

    /// One pass with the chord held as `held`.
    fn pass(session: &mut Session, held: Option<bool>) {
        session.hold_jettison(held);
        work(session, &catalog(), Controls::default());
    }

    fn jettisoned(sounds: &[SimSound]) -> usize {
        sounds
            .iter()
            .filter(|&&sound| sound == SimSound::CargoJettisoned)
            .count()
    }

    #[test]
    fn command_k_held_jettisons_on_every_pass() {
        let catalog = catalog();
        let mut session = flying(&catalog, 5);
        pass(&mut session, Some(true));
        assert_eq!(session.pilot().goods_held(), 0, "the first pass");
        session.pilot.cargo.insert(Good::Commodity(0), 5);
        pass(&mut session, Some(true));
        assert_eq!(session.pilot().goods_held(), 0, "the second pass");
        let said: Vec<SimMessage> = session
            .take_messages()
            .into_iter()
            .filter(|message| *message == SimMessage::CargoJettisoned)
            .collect();
        assert_eq!(said.len(), 2, "said twice");
        assert_eq!(jettisoned(&session.take_sounds()), 2);
    }

    #[test]
    fn let_go_it_jettisons_nothing() {
        let catalog = catalog();
        let mut session = flying(&catalog, 5);
        pass(&mut session, None);
        assert_eq!(session.pilot().goods_held(), 5);
    }

    #[test]
    fn a_double_time_frame_jettisons_on_both_passes() {
        let catalog = catalog();
        let mut session = flying(&catalog, 5);
        pass(&mut session, Some(true));
        session.pilot.cargo.insert(Good::Commodity(0), 5);
        session.tick_again(
            Controls::default(),
            &crate::ai::NovaAi::default(),
            Rules::default(),
            &mut NeverFires,
        );
        assert_eq!(session.pilot().goods_held(), 0, "the second pass too");
        assert_eq!(jettisoned(&session.take_sounds()), 2);
    }

    #[test]
    fn option_keeps_the_mission_cargo_each_pass() {
        // `_JettisonCargo(!Option)` (@0x6fda1): Option-Command-K keeps it.
        let catalog = catalog();
        let mut session = flying(&catalog, 5);
        pass(&mut session, Some(false));
        let said = session.take_messages();
        assert!(
            said.contains(&SimMessage::NonMissionCargoJettisoned),
            "{said:?}"
        );
    }

    #[test]
    fn nothing_in_the_pod_or_destroyed() {
        let catalog = catalog();
        let mut session = flying(&catalog, 5);
        session.ship.condition = Condition::Destroyed;
        session.hold_jettison(Some(true));
        session.held_jettison_tick(&mut NeverFires);
        assert_eq!(session.pilot().goods_held(), 5, "destroyed");
        let mut session = flying(&catalog, 5);
        session.pod = Some(10);
        session.hold_jettison(Some(true));
        session.held_jettison_tick(&mut NeverFires);
        assert_eq!(session.pilot().goods_held(), 5, "in the pod");
    }

    #[test]
    fn a_pass_with_nothing_aboard_lets_no_interface_load_go_and_marks_no_save() {
        // With nothing aboard `_JettisonCargo` runs only its reset
        // (@0x3e3b5).
        let catalog = catalog();
        let mut session = flying(&catalog, 0);
        let _ = session.take_save_due();
        session.out.interface_loads_cleared = false;
        session.hold_jettison(Some(true));
        session.held_jettison_tick(&mut NeverFires);
        assert!(!session.out.interface_loads_cleared, "no docked run");
        assert!(!session.take_save_due(), "nothing changed");
        assert_eq!(session.take_messages(), []);
        assert_eq!(jettisoned(&session.take_sounds()), 0);
    }

    #[test]
    fn the_step_input_holds_the_chord() {
        let catalog = catalog();
        let mut session = flying(&catalog, 5);
        let behaviour = crate::ai::NovaAi::default();
        let mut chance = NeverFires;
        let mut collab = StepCollaborators {
            catalog: &catalog,
            behaviour: &behaviour,
            rules: Rules::default(),
            chance: &mut chance,
        };
        let input = StepInput {
            jettison: Some(true),
            ..StepInput::default()
        };
        session.step(&input, 1, &mut collab, &mut StepReport::default());
        assert_eq!(session.pilot().goods_held(), 0);
    }
}
