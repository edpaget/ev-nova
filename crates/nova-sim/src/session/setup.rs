//! How a session is set up: the rule set and the strategy ports the
//! `with_*` builders set, which only those builders and
//! [`Session::set_sound_quiet`](super::Session::set_sound_quiet) write.

use std::rc::Rc;

use crate::combat::hull::DisableRule;
use crate::control::{ControlBits, NovaBits};
use crate::gate::{GateArrivalRule, WormholeRule};
use crate::hire::{HireTerms, NovaHire};
use crate::hyperspace::{HyperSelectRule, HyperlinkRule, JumpZoneRule, MultiJumpRule};
use crate::person::{NovaPersons, PersonRules};
use crate::rulebook::Rulebook;

use super::{Session, control, hire, ship_change};

/// The rule set and strategy ports a session follows, in `Session`'s
/// former field order.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Setup {
    /// The rule a multi-jump follows.
    pub(super) multi_jump: MultiJumpRule,
    /// The rule Hyper Select follows.
    pub(super) hyper_select: HyperSelectRule,
    /// The rule jumps along the hyperlinks follow.
    pub(super) hyperlinks: HyperlinkRule,
    /// The rule for where the no-jump zone applies.
    pub(super) jump_zone: JumpZoneRule,
    /// The rule a ship coming out of a gate follows.
    pub(super) gate_arrival: GateArrivalRule,
    /// The rule an unlinked wormhole follows.
    pub(super) wormholes: WormholeRule,
    /// The fee and wage of a hire (see [`Session::with_hire_terms`]).
    pub(super) hire_terms: hire::Shared<dyn HireTerms>,
    /// The control-bit test a ship's `Availability` for hire goes through
    /// (see [`Session::with_control_bits`]).
    pub(super) control_bits: hire::Shared<dyn ControlBits>,
    /// Whether the player's sound effects are quiet, which by
    /// [`RuleKey::AlertBlink`](crate::rulebook::RuleKey::AlertBlink)'s engine reading blinks the Red Alert's
    /// icon; set by the view from the player's preferences, never saved.
    pub(super) sound_quiet: bool,
    /// What sees each set operator `run_set` hands to `apply_op`: none
    /// outside tests (see `control::SetOpObserver`).
    pub(super) op_observer: Option<hire::Shared<dyn control::SetOpObserver>>,
    /// How persons appear (see [`Session::with_person_rules`]).
    pub(super) person_rules: hire::Shared<dyn PersonRules>,
    /// When the player's ship is disabled after a change of ship (see
    /// [`Session::with_disable_rule`]).
    pub(super) disable_rule: hire::Shared<dyn DisableRule>,
    /// The string lists `T` names the ship from (see
    /// [`Session::with_strings`]).
    pub(super) strings: ship_change::Strings,
    /// How each disputed rule the session acts on reads (see
    /// [`Session::with_rules`]); never saved.
    pub(super) rules: Rulebook,
    /// Whether the data turns on the `l33t` mode (`_leetMode`; see
    /// [`Session::with_strings`]); never saved.
    pub(super) leet: bool,
}

impl Default for Setup {
    /// What a session starts with: each rule's default, Nova's hire terms,
    /// control bits, persons and disable rule, no set-op observer, no
    /// string lists, the default rule set, sound effects not quiet, and the
    /// `l33t` mode off.
    fn default() -> Self {
        Self {
            multi_jump: MultiJumpRule::default(),
            hyper_select: HyperSelectRule::default(),
            hyperlinks: HyperlinkRule::default(),
            jump_zone: JumpZoneRule::default(),
            gate_arrival: GateArrivalRule::default(),
            wormholes: WormholeRule::default(),
            hire_terms: hire::Shared(Rc::new(NovaHire::default())),
            control_bits: hire::Shared(Rc::new(NovaBits)),
            sound_quiet: false,
            op_observer: None,
            person_rules: hire::Shared(Rc::new(NovaPersons::default())),
            disable_rule: Session::nova_disable(),
            strings: ship_change::Strings::none(),
            rules: Rulebook::default(),
            leet: false,
        }
    }
}
