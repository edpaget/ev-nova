//! The control bits on the session: the pilot's side of
//! [`control`](crate::control).
//!
//! **The bit edit.** [`Session::set_control_bit`] sets or clears one of the
//! pilot's control bits, and [`Session::control_bit`] reads one. An edit
//! makes a save due, as a change made in the spaceport does.
//!
//! **The pilot view.** Control-bit tests read the pilot through [`Facts`]:
//! its bits, gender and explored systems as the pilot holds them; the game
//! always counts as paid for, as the engine takes the game as registered
//! (see [`hire`](crate::hire)); and an outfit is had when the pilot owns
//! one or when a fighter out of its bay would dock back into it, as the
//! original counts deployed fighters (`_GetToken` @0x14a83): the bay
//! launching its ship type, that bay's ammunition outfit of lowest ID, the
//! same routing a fighter docking takes.
//!
//! **Set expressions.** [`Session::run_set`] runs a set expression on the
//! session (see [`control::execute`](crate::control::execute)): it writes
//! the pilot's bits, drawing `R(...)` on the caller's [`Chance`], and
//! hands every other operator to [`Session::apply_op`], one match over the
//! set operators. It applies
//! Nova's `G`, `D` and `X` (see the `outfits` module), the ship changes
//! `C`, `E`, `H` and `T` (see the `ship_change` module), and the moves `M` and `N`, the
//! leave `Q`, which apply when [`Session::settle_script`] settles them,
//! the sound `P` (see the `script_effects` module), the stellars
//! destroyed and regenerated, `Y` and `U` (see the `stellars` module), and
//! the missions started, aborted and failed, `S`, `A` and `F` (see the
//! `missions` module), and the ranks activated and deactivated, `K` and
//! `L` (see the `ranks` module).
//! Any change to the pilot makes a save due.
//!
//! **The nested runner.** The story dispatcher runs its subscriptions'
//! scripts through the same runner, `Session::run_ops`, which calls it
//! back after each operator handed to `apply_op`, that operator applied:
//! where it settles the moves and dispatches the events raised before the
//! next operator (see the [`story`](crate::story) module's "Dispatch").
//! A script run inside another is one more run like it.
//!
//! **Hooks.** The records' set-expression hooks (the `chär`'s `OnStart`,
//! an outfit's `OnPurchase` and `OnSell`, a ship's `OnPurchase`,
//! `OnCapture` and `OnRetire`) run through [`Session::run_set`] where
//! the original runs them: see the `hooks` module.

use std::collections::BTreeSet;

use super::precalced::ResetCause;
use super::script_effects::ScriptMove;
use super::{MissionRun, Outbox, PlayerShip, Session, Setup, Statics};
use crate::catalog::{OutfitId, ShipId, SystemId, WeaponId};
use crate::chance::Chance;
use crate::combat::armament::{Armament, lowest_ammo_outfit};
use crate::control::{
    Bit, BitStore, ControlBitSet, Gate, PilotFacts, ScriptNote, SetExpr, SetOp, SetOpKind, execute,
};
use crate::pilot::{Gender, Pilot};
use crate::rulebook::RuleSource;
use crate::ship_change::OutfitCarry;

/// What a control-bit test reads about the session's pilot (see the module
/// docs).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Facts<'a> {
    /// The pilot.
    pub(crate) pilot: &'a Pilot,
    /// Each ammunition outfit, with the `wëap` it is the rounds of.
    pub(crate) ammo_outfits: &'a [(WeaponId, OutfitId)],
    /// The player's weapons, its fighter bays among them.
    pub(crate) armament: &'a Armament,
}

/// Whether a `P` test passes: always, the engine taking the game as
/// registered.
pub(super) const REGISTERED: bool = true;

impl Facts<'_> {
    /// The outfit a carried fighter of ship type `ship` docks back into:
    /// the first bay launching it, that bay's ammunition outfit of lowest
    /// ID, the routing [`Armament::stow`] takes.
    fn docks_into(&self, ship: ShipId) -> Option<OutfitId> {
        let bay = self.armament.bay_of(ship)?;
        lowest_ammo_outfit(self.ammo_outfits, bay)
    }

    /// The outfits an `o` test counts: each the pilot owns one of or
    /// more, and each carried fighter's dock outfit
    /// ([`Facts::docks_into`]).
    pub(super) fn counted_outfits(&self) -> impl Iterator<Item = OutfitId> + '_ {
        let owned = self
            .pilot
            .outfits()
            .filter(|&(_, count)| count > 0)
            .map(|(id, _)| id);
        let docked = self
            .pilot
            .escorts()
            .iter()
            .filter(|escort| escort.carried)
            .filter_map(|escort| self.docks_into(escort.ship));
        owned.chain(docked)
    }
}

impl PilotFacts for Facts<'_> {
    fn bit(&self, bit: Bit) -> bool {
        self.pilot.control_bit(bit)
    }

    fn gender(&self) -> Gender {
        self.pilot.gender()
    }

    /// [`REGISTERED`].
    fn paid(&self, _days: u16) -> bool {
        REGISTERED
    }

    fn has_outfit(&self, outfit: OutfitId) -> bool {
        self.counted_outfits().any(|counted| counted == outfit)
    }

    fn explored(&self, system: SystemId) -> bool {
        self.pilot.has_explored(system)
    }
}

/// The session's pilot, as its `Facts` read it.
impl PilotFacts for Session {
    fn bit(&self, bit: Bit) -> bool {
        self.ship.facts(&self.pilot, &self.statics).bit(bit)
    }

    fn gender(&self) -> Gender {
        self.ship.facts(&self.pilot, &self.statics).gender()
    }

    fn paid(&self, days: u16) -> bool {
        self.ship.facts(&self.pilot, &self.statics).paid(days)
    }

    fn has_outfit(&self, outfit: OutfitId) -> bool {
        self.ship
            .facts(&self.pilot, &self.statics)
            .has_outfit(outfit)
    }

    fn explored(&self, system: SystemId) -> bool {
        self.ship.facts(&self.pilot, &self.statics).explored(system)
    }
}

/// A set expression writes the pilot's bits.
impl BitStore for Session {
    fn bits_mut(&mut self) -> &mut ControlBitSet {
        self.pilot.bits_mut()
    }
}

/// Sees each set operator `run_set` hands to `apply_op`, just before it
/// applies, with the session as it is when that operator runs (a test
/// seam: see the `hooks` module).
pub(crate) trait SetOpObserver: std::fmt::Debug {
    /// Sees `op` about to apply to `session`.
    fn saw(&self, op: &SetOp, session: &Session);
}

impl Session {
    /// This session with `observer` seeing each set operator `run_set`
    /// hands to `apply_op`, just before it applies.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn with_set_op_observer(mut self, observer: std::rc::Rc<dyn SetOpObserver>) -> Self {
        self.setup.op_observer = Some(super::hire::Shared(observer));
        self
    }

    /// Applies one set operator beyond the bit writes and `R(...)`.
    pub(crate) fn apply_op(&mut self, op: &SetOp, chance: &mut dyn Chance) {
        match op {
            SetOp::GrantOutfit(outfit) => self.script_grant(*outfit),
            SetOp::RemoveOutfit(outfit) => {
                self.ship
                    .script_remove(&mut self.pilot, &self.statics, &self.setup, *outfit);
            }
            SetOp::Explore(system) => self.chart.explore_system(&mut self.pilot, *system),
            SetOp::ChangeShip(ship) => self.change_ship(*ship, OutfitCarry::Keep, false),
            SetOp::ChangeShipWithDefaults(ship) => {
                let clamp = self.setup.ship_change_rules().clamps();
                self.change_ship(*ship, OutfitCarry::KeepWithDefaults, clamp);
            }
            SetOp::ReplaceShip(ship) => {
                let rules = self.setup.ship_change_rules();
                let carry = OutfitCarry::Persistent {
                    mask: rules.persistent(),
                };
                self.change_ship(*ship, carry, rules.clamps());
            }
            SetOp::RenameShip(list) => self.rename_ship(list.0, chance),
            SetOp::MoveTo(system) => {
                self.chart
                    .select_for_landed_move(&self.statics, self.mode.landed(), *system, true);
                self.run.queued.moves.push(ScriptMove::To(*system));
            }
            SetOp::MoveKeepPosition(system) => {
                self.chart.select_for_landed_move(
                    &self.statics,
                    self.mode.landed(),
                    *system,
                    false,
                );
                self.run
                    .queued
                    .moves
                    .push(ScriptMove::KeepPosition(*system));
            }
            SetOp::LeaveStellar(list) => self.leave_stellar(list.0, chance),
            SetOp::PlaySound(sound) => {
                self.run
                    .play_script_sound(&self.setup, &mut self.out, *sound);
            }
            SetOp::DestroyStellar(stellar) => self.destroy_stellar(*stellar, chance),
            SetOp::RegenerateStellar(stellar) => self.regenerate_stellar(*stellar),
            SetOp::AbortMission(mission) => self.script_abort(*mission, chance),
            SetOp::FailMission(mission) => self.script_fail(*mission),
            SetOp::StartMission(mission) => self.script_start(*mission, chance),
            SetOp::ActivateRank(rank) => {
                self.run
                    .activate_rank(&mut self.pilot, &self.statics, &self.setup, *rank);
            }
            SetOp::DeactivateRank(rank) => {
                self.run
                    .deactivate_rank(&mut self.pilot, &self.statics, &self.setup, *rank);
            }
            SetOp::OutOfRange { op, .. } => self.run.out_of_range(&self.setup, *op),
            // `execute` writes the bits and draws `R(...)` itself, never
            // handing them here.
            SetOp::Set(_) | SetOp::Clear(_) | SetOp::Toggle(_) | SetOp::Random(..) => {}
        }
    }

    /// Runs `expr` on the session (see the module docs), drawing `R(...)`
    /// on `chance`.
    pub fn run_set(&mut self, expr: &SetExpr, chance: &mut (impl Chance + ?Sized)) {
        self.run_ops(expr, &mut &mut *chance, &mut |_, _| {});
    }

    /// Runs `expr` as [`Session::run_set`] does, calling `between` after
    /// each operator handed to `apply_op`, with that operator applied:
    /// where the story dispatcher nests. A script run inside `between`
    /// runs as any other. Unless `expr` came from empty text
    /// ([`SetExpr::is_empty_text`]), the session then resets
    /// ([`Session::reset_precalced`]), as each of the original's
    /// set-string wrappers does once `_EvalSetExp` returns.
    pub(super) fn run_ops(
        &mut self,
        expr: &SetExpr,
        chance: &mut dyn Chance,
        between: &mut dyn FnMut(&mut Session, &mut dyn Chance),
    ) {
        let observer = self.setup.op_observer.clone();
        let before = self.pilot.clone();
        let mut apply = |op: &SetOp, session: &mut Session, chance: &mut dyn Chance| {
            if let Some(observer) = &observer {
                observer.0.saw(op, session);
            }
            session.apply_op(op, chance);
            between(session, chance);
            Ok(())
        };
        execute(expr, self, &mut apply, chance, &mut BTreeSet::new());
        // The set-string wrappers' tail into the reset, skipped only for
        // empty text (`_EvalCurrentMissionBitSetString` @0x99a58, @0x99a85;
        // `_EvalMissionBitSetString` @0x99dd1, @0x99dea; see the
        // `precalced` module). The original also resets inside `G`, `D`,
        // `C`, `E` and `H`, each through `_SystemInfoToShipStats`
        // (@0x15489, @0x154ef, @0x1566a); the engine resets once, here.
        // No frame is drawn, no dialog opens and nothing is saved between
        // the two, so the difference crosses no boundary the player sees.
        if !expr.is_empty_text() {
            self.reset_precalced_for(ResetCause::SetExpr);
        }
        if self.pilot != before {
            self.out.save_due = true;
            // What a `Visibility` test reads is the pilot's: one left as
            // it was cannot change the galaxy.
            self.chart.galaxy_stale = true;
        }
    }

    /// What running set expressions had to tell since this was last taken,
    /// in order; taking it empties the list.
    pub fn take_script_notes(&mut self) -> Vec<ScriptNote> {
        self.out.take_script_notes()
    }

    /// The session's control bits ([`Session::with_control_bits`]) and its
    /// pilot, as the rules that test a record's control bits ask them.
    pub(crate) fn gate(&self) -> Gate<'_> {
        Gate {
            control_bits: &*self.setup.control_bits.0,
            pilot: self,
        }
    }

    /// Whether the pilot's control bit `bit` is set.
    #[must_use]
    pub fn control_bit(&self, bit: Bit) -> bool {
        self.pilot.control_bit(bit)
    }

    /// Sets the pilot's control bit `bit` when `on`, and clears it
    /// otherwise; a save is due.
    pub fn set_control_bit(&mut self, bit: Bit, on: bool) {
        let bits = self.pilot.bits_mut();
        if on {
            bits.set(bit);
        } else {
            bits.clear(bit);
        }
        self.out.save_due = true;
    }
}

impl Outbox {
    /// What running set expressions had to tell since this was last taken,
    /// in order; taking it empties the list.
    fn take_script_notes(&mut self) -> Vec<ScriptNote> {
        std::mem::take(&mut self.script_notes)
    }
}

impl PlayerShip {
    /// What a control-bit test reads about the pilot.
    pub(super) fn facts<'a>(&'a self, pilot: &'a Pilot, statics: &'a Statics) -> Facts<'a> {
        Facts {
            pilot,
            ammo_outfits: &statics.ammo_outfits,
            armament: &self.fit.armament,
        }
    }
}

impl MissionRun {
    /// An operator whose number is out of range (see
    /// [`SetOp::OutOfRange`]). `_EvalSetExp` skips each such operator and
    /// goes on to the next (@0x15873; C, E and H by @0x156e0, the bits by
    /// their signed checks @0x1524a, @0x15271, @0x15298), but for `N`,
    /// whose check (@0x158e3) runs the move's tail (@0x1593f): that queues
    /// a stay by [`ScriptEffectRules::out_of_range_move`]'s engine reading,
    /// and does nothing by the other.
    ///
    /// `P`, `T` and `Q` have no check, and a number past 32767, the
    /// engine's widest ID, names no resource, so each does what the
    /// original does with a missing one: `T` nothing (an empty pick
    /// skips, @0x15b47); `Q` a blank `Q` (`_GetRandomIndString` @0x72f0f
    /// leaves the message empty, @0x15c02); `P` a sound that does not
    /// load, which by [`ScriptEffectRules::sound`]'s engine reading
    /// replaces a held one with none (@0x15a30; `_PlayGame`
    /// @0x4631b-0x46348).
    ///
    /// [`ScriptEffectRules::out_of_range_move`]: crate::ScriptEffectRules::out_of_range_move
    /// [`ScriptEffectRules::sound`]: crate::ScriptEffectRules::sound
    fn out_of_range(&mut self, setup: &Setup, op: SetOpKind) {
        match op {
            SetOpKind::MoveKeepPosition
                if setup.script_effect_rules().out_of_range_move == RuleSource::Engine =>
            {
                self.queued.moves.push(ScriptMove::Stay);
            }
            SetOpKind::PlaySound => self.drop_script_sound(setup),
            SetOpKind::LeaveStellar => self.leave_on_no_list(setup),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;
    use crate::control::{SetExpr, SetOp};
    use crate::save;
    use crate::testkit::Scripted;
    use crate::testkit::catalog;

    fn session() -> Session {
        let catalog = catalog();
        Session::fly(&catalog, Pilot::new(&catalog, "Ada").expect("starts")).expect("flies")
    }

    fn bit(n: u16) -> Bit {
        Bit::new(n).expect("in range")
    }

    #[test]
    fn a_control_bit_can_be_set_and_cleared() {
        let mut session = session();
        assert!(!session.control_bit(bit(42)));
        session.set_control_bit(bit(42), true);
        assert!(session.control_bit(bit(42)));
        assert!(!session.control_bit(bit(43)));
        assert!(session.pilot().control_bit(bit(42)));
        session.set_control_bit(bit(42), true);
        assert!(session.control_bit(bit(42)), "setting it again keeps it");
        session.set_control_bit(bit(42), false);
        assert!(!session.control_bit(bit(42)));
        session.set_control_bit(bit(42), false);
        assert!(!session.control_bit(bit(42)), "clearing it again keeps it");
    }

    #[test]
    fn each_edit_makes_a_save_due() {
        let mut session = session();
        assert!(!session.take_save_due());
        session.set_control_bit(bit(7), true);
        assert!(session.take_save_due());
        assert!(!session.take_save_due());
        session.set_control_bit(bit(7), false);
        assert!(session.take_save_due());
    }

    #[test]
    fn the_pilot_view_reads_the_bits_gender_and_explored_systems() {
        let mut session = session();
        session.set_control_bit(bit(12), true);
        let facts = session.ship.facts(&session.pilot, &session.statics);
        assert!(facts.bit(bit(12)));
        assert!(!facts.bit(bit(13)));
        assert_eq!(facts.gender(), Gender::Male);
        let start = session.pilot().system();
        assert!(facts.explored(start));
        assert!(!facts.explored(SystemId(start.0 + 1)));
        let catalog = catalog();
        let female = Pilot::new(&catalog, "Eve")
            .expect("starts")
            .with_gender(Gender::Female);
        let session = Session::fly(&catalog, female).expect("flies");
        assert_eq!(
            session
                .ship
                .facts(&session.pilot, &session.statics)
                .gender(),
            Gender::Female
        );
    }

    #[test]
    fn the_session_answers_a_test_as_its_pilot_view_does() {
        let catalog = catalog();
        let mut pilot = Pilot::new(&catalog, "Eve")
            .expect("starts")
            .with_gender(Gender::Female);
        pilot.outfits.insert(OutfitId(300), 1);
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        session.set_control_bit(bit(12), true);
        let start = session.pilot().system();
        let view: &dyn PilotFacts = &session;
        assert!(view.bit(bit(12)));
        assert!(!view.bit(bit(13)));
        assert_eq!(view.gender(), Gender::Female);
        assert!(view.paid(30));
        assert!(view.has_outfit(OutfitId(300)));
        assert!(!view.has_outfit(OutfitId(301)));
        assert!(view.explored(start));
        assert!(!view.explored(SystemId(start.0 + 1)));
        let male = self::session();
        assert_eq!(PilotFacts::gender(&male), Gender::Male);
    }

    #[test]
    fn the_pilot_view_always_counts_the_game_as_paid() {
        let session = session();
        for days in [0, 30, u16::MAX] {
            assert!(
                session
                    .ship
                    .facts(&session.pilot, &session.statics)
                    .paid(days),
                "{days}"
            );
        }
    }

    #[test]
    fn the_pilot_view_has_an_outfit_the_pilot_owns() {
        let catalog = catalog();
        let mut pilot = Pilot::new(&catalog, "Ada").expect("starts");
        pilot.outfits.insert(OutfitId(300), 1);
        let session = Session::fly(&catalog, pilot).expect("flies");
        assert!(
            session
                .ship
                .facts(&session.pilot, &session.statics)
                .has_outfit(OutfitId(300))
        );
        assert!(
            !session
                .ship
                .facts(&session.pilot, &session.statics)
                .has_outfit(OutfitId(301))
        );
    }

    fn set(text: &str) -> SetExpr {
        SetExpr::parse(text).expect("parses")
    }

    #[test]
    fn running_a_set_expression_writes_the_bits_and_makes_a_save_due() {
        let mut session = session();
        session.run_set(&set("b7"), &mut Scripted::default());
        assert!(session.control_bit(bit(7)));
        assert!(session.take_save_due());
        session.run_set(&set("b7"), &mut Scripted::default());
        assert!(!session.take_save_due(), "no bit changed");
        session.run_set(&set("R(b8 !b7)"), &mut Scripted::rolling(&[0]));
        assert!(!session.control_bit(bit(7)));
        assert!(!session.control_bit(bit(8)));
        assert!(session.take_save_due());
    }

    /// Records the pilot's name and bit 1 for each operator it sees.
    #[derive(Debug, Default)]
    struct Recording(RefCell<Vec<(String, bool)>>);

    impl SetOpObserver for Recording {
        fn saw(&self, _op: &SetOp, session: &Session) {
            self.0.borrow_mut().push((
                session.pilot().name().to_owned(),
                session.control_bit(bit(1)),
            ));
        }
    }

    #[test]
    fn the_set_op_observer_sees_the_session_as_the_operator_runs() {
        let recording = Rc::new(Recording::default());
        let mut session = session().with_set_op_observer(recording.clone());
        session.run_set(&set("b1 X131"), &mut Scripted::default());
        assert_eq!(*recording.0.borrow(), [("Ada".to_owned(), true)]);
        assert_eq!(session.take_script_notes(), []);
    }

    #[test]
    fn apply_op_leaves_the_bit_writes_and_r_to_execute() {
        let mut session = session();
        for op in &set("b1 ^b2 R(b3 b4)").ops {
            session.apply_op(op, &mut Scripted::default());
        }
        for n in 1..=4 {
            assert!(!session.control_bit(bit(n)), "b{n}");
        }
        assert!(!session.take_save_due());
    }

    /// Records whether system 131 is explored each time it sees an
    /// operator.
    #[derive(Debug, Default)]
    struct Explored(RefCell<Vec<bool>>);

    impl SetOpObserver for Explored {
        fn saw(&self, _op: &SetOp, session: &Session) {
            self.0
                .borrow_mut()
                .push(session.pilot().has_explored(SystemId(131)));
        }
    }

    #[test]
    fn the_set_op_observer_sees_the_session_before_the_operator_applies() {
        let explored = Rc::new(Explored::default());
        let mut session = session().with_set_op_observer(explored.clone());
        session.run_set(&set("X131"), &mut Scripted::default());
        assert_eq!(*explored.0.borrow(), [false], "seen before X applied");
        assert!(session.pilot().has_explored(SystemId(131)), "then applied");
    }

    #[test]
    fn between_runs_after_each_operator_handed_on_with_its_effect_applied() {
        let mut session = session();
        let mut seen = Vec::new();
        session.run_ops(
            &set("b1 X131 S300 b2 X132"),
            &mut Scripted::default(),
            &mut |session, _| {
                seen.push((
                    session.pilot().has_explored(SystemId(131)),
                    session.pilot().has_explored(SystemId(132)),
                    session.control_bit(bit(2)),
                ));
            },
        );
        assert_eq!(
            seen,
            [
                (true, false, false),
                (true, false, false),
                (true, true, true)
            ],
            "after X131, S300 and X132; never after a bit write"
        );
    }

    /// `STR#` 128 holds one message.
    struct Told;

    impl crate::catalog::CommCatalog for Told {
        fn string_list(&self, id: i16) -> Vec<String> {
            if id == 128 {
                vec!["Leave.".to_owned()]
            } else {
                Vec::new()
            }
        }
    }

    #[test]
    fn every_set_operator_applies() {
        let catalog = catalog();
        let mut session = session().with_strings(Rc::new(Told));
        crate::testkit::land_now(&mut session).expect("lands");
        session.take_sounds();
        session.run_set(
            &set(
                "A128 F129 S130 G128 D128 C128 E128 H128 M130 N130 K128 L128 P128 Y128 U128 \
                  T128 Q128 X131",
            ),
            &mut Scripted::default(),
        );
        assert_eq!(session.take_script_notes(), []);
        assert!(session.pilot().has_explored(SystemId(131)), "X ran");
        let settled = session.settle_script(&catalog, &mut Scripted::default());
        assert_eq!(settled.moved, Some(SystemId(130)), "M and N queued");
        assert!(session.leave_due(), "Q held");
        let _ = session.take_off(&catalog, &mut Scripted::default());
        session.tick(crate::flight::Controls::default());
        assert!(
            session
                .take_sounds()
                .contains(&crate::sound::SimSound::Script {
                    sound: crate::catalog::SoundId(128),
                    exclusive: true
                }),
            "P held to the tick"
        );
    }

    #[test]
    fn any_change_to_the_pilot_by_a_set_expression_makes_a_save_due() {
        let mut session = session();
        session.run_set(&set("X131"), &mut Scripted::default());
        assert!(session.take_save_due());
        session.run_set(&set("X131"), &mut Scripted::default());
        assert!(!session.take_save_due(), "explored already");
        session.run_set(&set("G300"), &mut Scripted::default());
        assert!(session.take_save_due());
    }

    #[test]
    fn an_edited_bit_is_saved() {
        let mut session = session();
        session.set_control_bit(bit(9999), true);
        session.set_control_bit(bit(3), true);
        session.set_control_bit(bit(3), false);
        let saved = save::decode(&save::encode(session.pilot())).expect("loads");
        assert!(saved.control_bit(bit(9999)));
        assert!(!saved.control_bit(bit(3)));
        assert_eq!(saved.control_bits(), session.pilot().control_bits());
    }
}
