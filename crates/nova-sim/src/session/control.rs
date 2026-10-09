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
//! set operators, which other roadmaps extend with a match arm. It applies
//! Nova's `G`, `D` and `X` (see the `outfits` module), the ship changes
//! `C`, `E`, `H` and `T` (see the `ship_change` module), and the moves `M` and `N`, the
//! leave `Q`, which apply when [`Session::settle_script`] settles them,
//! and the sound `P` (see the `script_effects` module). Any change to the pilot makes
//! a save due. An operator nothing handles is skipped, and its kind told
//! once a session as a [`ScriptNote`] ([`Session::take_script_notes`]);
//! which kinds were told is never saved. The operators still unhandled
//! are those of the missions (`A`, `F`, `S`), ranks (`K`, `L`) and
//! stellars (`Y`, `U`), whose arms other work adds.
//!
//! **Hooks.** The records' set-expression hooks (the `chär`'s `OnStart`,
//! an outfit's `OnPurchase` and `OnSell`, a ship's `OnPurchase`,
//! `OnCapture` and `OnRetire`) run through [`Session::run_set`] where
//! the original runs them: see the `hooks` module.

use super::Session;
use super::script_effects::ScriptMove;
use crate::catalog::{OutfitId, ShipId, SystemId, WeaponId};
use crate::chance::Chance;
use crate::combat::armament::{Armament, lowest_ammo_outfit};
use crate::control::{
    Bit, BitStore, ControlBitSet, Gate, PilotFacts, ScriptNote, SetExpr, SetOp, SetOpKind, execute,
};
use crate::pilot::{Gender, Pilot};
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

impl Facts<'_> {
    /// The outfit a carried fighter of ship type `ship` docks back into:
    /// the first bay launching it, that bay's ammunition outfit of lowest
    /// ID, the routing [`Armament::stow`] takes.
    fn docks_into(&self, ship: ShipId) -> Option<OutfitId> {
        let bay = self.armament.bay_of(ship)?;
        lowest_ammo_outfit(self.ammo_outfits, bay)
    }
}

impl PilotFacts for Facts<'_> {
    fn bit(&self, bit: Bit) -> bool {
        self.pilot.control_bit(bit)
    }

    fn gender(&self) -> Gender {
        self.pilot.gender()
    }

    /// Always: the engine takes the game as registered.
    fn paid(&self, _days: u16) -> bool {
        true
    }

    fn has_outfit(&self, outfit: OutfitId) -> bool {
        self.pilot.owned(outfit) > 0
            || self
                .pilot
                .escorts()
                .iter()
                .filter(|escort| escort.carried)
                .any(|escort| self.docks_into(escort.ship) == Some(outfit))
    }

    fn explored(&self, system: SystemId) -> bool {
        self.pilot.has_explored(system)
    }
}

/// The session's pilot, as its `Facts` read it.
impl PilotFacts for Session {
    fn bit(&self, bit: Bit) -> bool {
        self.facts().bit(bit)
    }

    fn gender(&self) -> Gender {
        self.facts().gender()
    }

    fn paid(&self, days: u16) -> bool {
        self.facts().paid(days)
    }

    fn has_outfit(&self, outfit: OutfitId) -> bool {
        self.facts().has_outfit(outfit)
    }

    fn explored(&self, system: SystemId) -> bool {
        self.facts().explored(system)
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
        self.op_observer = Some(super::hire::Shared(observer));
        self
    }

    /// Applies one set operator beyond the bit writes and `R(...)`; gives
    /// back its kind when nothing here applies it yet (another roadmap's
    /// operator).
    pub(crate) fn apply_op(
        &mut self,
        op: &SetOp,
        chance: &mut dyn Chance,
    ) -> Result<(), SetOpKind> {
        match op {
            SetOp::GrantOutfit(outfit) => self.script_grant(*outfit),
            SetOp::RemoveOutfit(outfit) => self.script_remove(*outfit),
            SetOp::Explore(system) => self.explore_system(*system),
            SetOp::ChangeShip(ship) => self.change_ship(*ship, OutfitCarry::Keep, false),
            SetOp::ChangeShipWithDefaults(ship) => {
                let clamp = self.ship_change_rules().clamps();
                self.change_ship(*ship, OutfitCarry::KeepWithDefaults, clamp);
            }
            SetOp::ReplaceShip(ship) => {
                let rules = self.ship_change_rules();
                let carry = OutfitCarry::Persistent {
                    mask: rules.persistent(),
                };
                self.change_ship(*ship, carry, rules.clamps());
            }
            SetOp::RenameShip(list) => self.rename_ship(list.0, chance),
            SetOp::MoveTo(system) => self.queued.moves.push(ScriptMove::To(*system)),
            SetOp::MoveKeepPosition(system) => {
                self.queued.moves.push(ScriptMove::KeepPosition(*system));
            }
            SetOp::LeaveStellar(list) => self.leave_stellar(list.0, chance),
            SetOp::PlaySound(sound) => self.play_script_sound(*sound),
            // Missions, ranks and stellars: other roadmaps add their arms here.
            SetOp::AbortMission(_)
            | SetOp::FailMission(_)
            | SetOp::StartMission(_)
            | SetOp::ActivateRank(_)
            | SetOp::DeactivateRank(_)
            | SetOp::DestroyStellar(_)
            | SetOp::RegenerateStellar(_)
            // `execute` writes the bits and draws `R(...)` itself, never
            // handing them here.
            | SetOp::Set(_)
            | SetOp::Clear(_)
            | SetOp::Toggle(_)
            | SetOp::Random(..) => return Err(op.kind()),
        }
        Ok(())
    }

    /// Runs `expr` on the session (see the module docs), drawing `R(...)`
    /// on `chance`.
    pub fn run_set(&mut self, expr: &SetExpr, chance: &mut (impl Chance + ?Sized)) {
        let observer = self.op_observer.clone();
        let before = self.pilot.clone();
        let mut told = std::mem::take(&mut self.unhandled_ops);
        let mut apply = |op: &SetOp, session: &mut Session, chance: &mut dyn Chance| {
            if let Some(observer) = &observer {
                observer.0.saw(op, session);
            }
            session.apply_op(op, chance)
        };
        let unhandled = execute(expr, self, &mut apply, &mut &mut *chance, &mut told);
        self.unhandled_ops = told;
        if self.pilot != before {
            self.save_due = true;
        }
        self.script_notes
            .extend(unhandled.into_iter().map(ScriptNote::Unhandled));
    }

    /// What running set expressions had to tell since this was last taken,
    /// in order; taking it empties the list.
    pub fn take_script_notes(&mut self) -> Vec<ScriptNote> {
        std::mem::take(&mut self.script_notes)
    }

    /// What a control-bit test reads about the pilot.
    pub(crate) fn facts(&self) -> Facts<'_> {
        Facts {
            pilot: &self.pilot,
            ammo_outfits: &self.ammo_outfits,
            armament: &self.armament,
        }
    }

    /// The session's control bits ([`Session::with_control_bits`]) and its
    /// pilot, as the rules that test a record's control bits ask them.
    pub(crate) fn gate(&self) -> Gate<'_> {
        Gate {
            control_bits: &*self.control_bits.0,
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
        self.save_due = true;
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;
    use crate::control::{SetExpr, SetOp, SetOpKind};
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
        let facts = session.facts();
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
        assert_eq!(session.facts().gender(), Gender::Female);
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
            assert!(session.facts().paid(days), "{days}");
        }
    }

    #[test]
    fn the_pilot_view_has_an_outfit_the_pilot_owns() {
        let catalog = catalog();
        let mut pilot = Pilot::new(&catalog, "Ada").expect("starts");
        pilot.outfits.insert(OutfitId(300), 1);
        let session = Session::fly(&catalog, pilot).expect("flies");
        assert!(session.facts().has_outfit(OutfitId(300)));
        assert!(!session.facts().has_outfit(OutfitId(301)));
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
    fn apply_op_applies_novas_eleven_and_gives_back_the_rest() {
        let mut session = session();
        let expr = set(
            "A128 F129 S130 G128 D128 C128 E128 H128 M130 N130 K128 L128 P128 Y128 U128 \
              T128 Q128 X131 b1 !b1 ^b1 R(b1 b2)",
        );
        let given_back: Vec<_> = expr
            .ops
            .iter()
            .filter_map(|op| {
                session
                    .apply_op(op, &mut Scripted::default())
                    .err()
                    .map(|kind| (kind, op.kind()))
            })
            .collect();
        assert!(
            given_back.iter().all(|(kind, own)| kind == own),
            "{given_back:?}"
        );
        assert_eq!(
            given_back
                .into_iter()
                .map(|(kind, _)| kind)
                .collect::<Vec<_>>(),
            [
                SetOpKind::AbortMission,
                SetOpKind::FailMission,
                SetOpKind::StartMission,
                SetOpKind::ActivateRank,
                SetOpKind::DeactivateRank,
                SetOpKind::DestroyStellar,
                SetOpKind::RegenerateStellar,
                SetOpKind::Set,
                SetOpKind::Clear,
                SetOpKind::Toggle,
                SetOpKind::Random,
            ]
        );
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
    fn an_unhandled_operator_is_noted_once_and_the_next_still_applies() {
        let mut session = session();
        session.run_set(&set("S300 b4"), &mut Scripted::default());
        assert!(session.control_bit(bit(4)));
        session.run_set(&set("S301 b5"), &mut Scripted::default());
        assert!(session.control_bit(bit(5)));
        assert_eq!(
            session.take_script_notes(),
            [ScriptNote::Unhandled(SetOpKind::StartMission)]
        );
        assert_eq!(session.take_script_notes(), [], "taken");
        session.run_set(&set("K128"), &mut Scripted::default());
        assert_eq!(
            session.take_script_notes(),
            [ScriptNote::Unhandled(SetOpKind::ActivateRank)]
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
    fn the_operators_still_unhandled_are_those_of_other_roadmaps() {
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
        assert_eq!(
            session.take_script_notes(),
            [
                SetOpKind::AbortMission,
                SetOpKind::FailMission,
                SetOpKind::StartMission,
                SetOpKind::ActivateRank,
                SetOpKind::DeactivateRank,
                SetOpKind::DestroyStellar,
                SetOpKind::RegenerateStellar,
            ]
            .map(ScriptNote::Unhandled)
        );
        assert!(session.pilot().has_explored(SystemId(131)), "X ran");
        let settled = session.settle_script(&catalog, &mut Scripted::default());
        assert_eq!(settled.moved, Some(SystemId(130)), "M and N queued");
        assert_eq!(
            settled.took_off,
            Some(crate::catalog::StellarId(128)),
            "Q queued"
        );
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
