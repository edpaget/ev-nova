//! How a session is set up: the rule set and the strategy ports the
//! `with_*` builders set, which only those builders,
//! [`Session::set_sound_quiet`](super::Session::set_sound_quiet) and
//! [`Session::set_sprite_load`](super::Session::set_sprite_load) write.
//!
//! The travel rules (a multi-jump's hops, Hyper Select, the hyperlinks,
//! the no-jump zone, a gate's arrival and an unlinked wormhole's exit)
//! are not held apart: each is read from the rule set's key where it
//! decides, by the accessors here, so whatever sets the rule set
//! (`Session::with_rules`, or `Session::load` with the rulebook it is
//! given) sets them.

use std::rc::Rc;

use crate::combat::SpriteLoad;
use crate::combat::hull::DisableRule;
use crate::control::{ControlBits, NovaBits};
use crate::gate::{GateArrivalRule, WormholeRule};
use crate::hire::{HireTerms, NovaHire};
use crate::hyperspace::{HyperSelectRule, HyperlinkRule, JumpZoneRule, MultiJumpRule};
use crate::person::{NovaPersons, PersonRules};
use crate::rulebook::{RuleKey, Rulebook};

use super::{Session, control, hire, ship_change};

/// The rule set and strategy ports a session follows, in `Session`'s
/// former field order.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Setup {
    /// The fee and wage of a hire (see [`Session::with_hire_terms`]).
    pub(super) hire_terms: hire::Shared<dyn HireTerms>,
    /// The control-bit test a ship's `Availability` for hire goes through
    /// (see [`Session::with_control_bits`]).
    pub(super) control_bits: hire::Shared<dyn ControlBits>,
    /// Whether the player's sound effects are quiet, which by
    /// [`RuleKey::AlertBlink`](crate::rulebook::RuleKey::AlertBlink)'s engine reading blinks the Red Alert's
    /// icon; set by the view from the player's preferences, never saved.
    pub(super) sound_quiet: bool,
    /// What of each ship's sprites loads, which every hull taken into
    /// play is loaded by (see the `sprite_load` module); set by the view
    /// from the player's preferences, never saved.
    pub(super) sprite_load: SpriteLoad,
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
    /// What opens the stellars' compiled conversations (see
    /// [`Session::with_dialog_loader`]): none until the game wires one, so
    /// a conversation shows that it can't open.
    pub(super) dialogs: Option<super::conversation::Loader>,
}

impl Default for Setup {
    /// What a session starts with: each rule's default, Nova's hire terms,
    /// control bits, persons and disable rule, no set-op observer, no
    /// string lists, the default rule set, sound effects not quiet, every
    /// sprite loaded, the `l33t` mode off, and no dialog loader.
    fn default() -> Self {
        Self {
            hire_terms: hire::Shared(Rc::new(NovaHire::default())),
            control_bits: hire::Shared(Rc::new(NovaBits)),
            sound_quiet: false,
            sprite_load: SpriteLoad::default(),
            op_observer: None,
            person_rules: hire::Shared(Rc::new(NovaPersons::default())),
            disable_rule: Session::nova_disable(),
            strings: ship_change::Strings::none(),
            rules: Rulebook::default(),
            leet: false,
            dialogs: None,
        }
    }
}

impl Setup {
    /// How [`RuleKey::MultiJump`] reads: how a multi-jump chains a jump's
    /// hops.
    pub(super) fn multi_jump(&self) -> MultiJumpRule {
        MultiJumpRule::from(self.rules.source_for(RuleKey::MultiJump))
    }

    /// How [`RuleKey::HyperSelect`] reads: which systems Hyper Select
    /// cycles through.
    pub(super) fn hyper_select(&self) -> HyperSelectRule {
        HyperSelectRule::from(self.rules.source_for(RuleKey::HyperSelect))
    }

    /// How [`RuleKey::Hyperlinks`] reads: which systems a jump can reach.
    pub(super) fn hyperlinks(&self) -> HyperlinkRule {
        HyperlinkRule::from(self.rules.source_for(RuleKey::Hyperlinks))
    }

    /// How [`RuleKey::JumpZone`] reads: where the no-jump zone applies.
    pub(super) fn jump_zone(&self) -> JumpZoneRule {
        JumpZoneRule::from(self.rules.source_for(RuleKey::JumpZone))
    }

    /// How [`RuleKey::GateArrival`] reads: where a ship comes out of a
    /// hypergate or wormhole.
    pub(super) fn gate_arrival(&self) -> GateArrivalRule {
        GateArrivalRule::from(self.rules.source_for(RuleKey::GateArrival))
    }

    /// How [`RuleKey::WormholeExit`] reads: which wormhole an unlinked
    /// wormhole leads to.
    pub(super) fn wormholes(&self) -> WormholeRule {
        WormholeRule::from(self.rules.source_for(RuleKey::WormholeExit))
    }
}

/// The default rulebook with `key` read so that it gives the travel rule
/// `rule`: [`RuleSource::Engine`](crate::rulebook::RuleSource::Engine)
/// for the engine's reading and
/// [`RuleSource::Bible`](crate::rulebook::RuleSource::Bible) for the other,
/// as `R::from` maps them.
#[cfg(test)]
pub(crate) fn reading<R>(key: crate::rulebook::RuleKey, rule: R) -> Rulebook
where
    R: From<crate::rulebook::RuleSource> + Copy + PartialEq + std::fmt::Debug,
{
    let source = crate::rulebook::RuleSource::ALL
        .into_iter()
        .find(|&source| R::from(source) == rule)
        .unwrap_or_else(|| panic!("no source reads {rule:?}"));
    Rulebook::default().with_override(key, source)
}
