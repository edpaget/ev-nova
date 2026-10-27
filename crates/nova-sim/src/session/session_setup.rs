//! A session's setup as one value: [`SessionSetup`], what the edge of the
//! program hands a session it has built, and [`CarryOver`], what one
//! flight of a run hands on to the next.
//!
//! The `with_*` builders each set one field of the session's `Setup` (the
//! rule set and the strategy ports) or hand it one piece of the run's
//! history. [`Session::set_up`] applies the whole setup in one call, in the
//! order the app's edge chains the builders, and [`Session::carry_over`]
//! reads back the history in one, so an adapter forwards one call each
//! way. The builders stay, as their own tests pin each field.

use std::fmt;
use std::rc::Rc;

use crate::catalog::CommCatalog;
use crate::combat::hull::DisableRule;
use crate::control::{ControlBits, NovaBits};
use crate::hire::{HireTerms, NovaHire};
use crate::person::{NovaPersons, PersonRules};
use crate::rulebook::Rulebook;
use crate::ship_layer::SlotSprites;
use crate::story::sprite_pool::StellarSpritePool;
use crate::traffic::kept_berths::KeptBerths;

use super::Session;
use super::last_refresh::LastRefresh;

/// Everything a session is set up with once it is built: its rule set,
/// the strategy ports it keeps, the string lists `T` names the ship from,
/// and what the run carried over from the flights before it
/// ([`Session::set_up`]). The default is what a built session already
/// has: every rule the engine's, Nova's ports, no string lists, and a
/// launch's carry-over, nothing yet.
#[derive(Clone)]
pub struct SessionSetup {
    /// How each disputed rule reads ([`Session::with_rules`]).
    pub rules: Rulebook,
    /// When the player's ship is disabled after a change of ship
    /// ([`Session::with_disable_rule`]).
    pub disable_rule: Rc<dyn DisableRule>,
    /// The fee and wage of a hire ([`Session::with_hire_terms`]).
    pub hire_terms: Rc<dyn HireTerms>,
    /// The control-bit test a ship's `Availability` and a person's
    /// `ActiveOn` go through ([`Session::with_control_bits`]).
    pub control_bits: Rc<dyn ControlBits>,
    /// How persons appear ([`Session::with_person_rules`]).
    pub person_rules: Rc<dyn PersonRules>,
    /// The string lists `T` names the ship from
    /// ([`Session::with_strings`]); none leaves the session's own.
    pub strings: Option<Rc<dyn CommCatalog>>,
    /// What the run carried over from the flights before.
    pub carry: CarryOver,
}

impl Default for SessionSetup {
    fn default() -> Self {
        Self {
            rules: Rulebook::default(),
            disable_rule: Session::nova_disable().0,
            hire_terms: Rc::new(NovaHire::default()),
            control_bits: Rc::new(NovaBits),
            person_rules: Rc::new(NovaPersons::default()),
            strings: None,
            carry: CarryOver::default(),
        }
    }
}

impl fmt::Debug for SessionSetup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SessionSetup")
            .field("rules", &self.rules)
            .field("disable_rule", &self.disable_rule)
            .field("hire_terms", &self.hire_terms)
            .field("control_bits", &self.control_bits)
            .field("person_rules", &self.person_rules)
            .field("strings", &self.strings.as_ref().map(|_| "Strings"))
            .field("carry", &self.carry)
            .finish()
    }
}

/// What one flight of a run hands on to the next, as the original's
/// globals outlive a pilot: the ship slots' sprite history
/// ([`Session::with_slot_sprites`]), the stellar sprite pool
/// ([`Session::carry_stellar_sprite_pool`]), what the traffic's berths
/// keep ([`Session::with_kept_berths`]), and what the last galaxy refresh
/// left hidden, for a new pilot's neighbour pass
/// ([`Session::with_last_refresh`]). Each is handed on only when given:
/// `None` leaves the session's own, and the default carries nothing. A
/// session's [`Session::carry_over`] gives every one.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CarryOver {
    /// The ship and shot slots' layers, frames and places.
    pub slot_sprites: Option<SlotSprites>,
    /// The stellar sprite pool.
    pub stellar_sprite_pool: Option<StellarSpritePool>,
    /// What the traffic's berths keep.
    pub kept_berths: Option<KeptBerths>,
    /// What the last galaxy refresh left hidden: handed only to a new
    /// pilot's session.
    pub last_refresh: Option<LastRefresh>,
}

impl Session {
    /// This session set up with `setup`, as the builders set it up in the
    /// order the app's edge chained them: the disable rule, the rule set,
    /// the slot sprites, the stellar sprite pool and the kept berths when
    /// carried, the hire terms, the control bits, the persons, the string
    /// lists when given, and a new pilot's last refresh when carried.
    #[must_use]
    pub fn set_up(self, setup: SessionSetup) -> Self {
        let SessionSetup {
            rules,
            disable_rule,
            hire_terms,
            control_bits,
            person_rules,
            strings,
            carry,
        } = setup;
        let mut session = self.with_disable_rule(disable_rule).with_rules(rules);
        if let Some(slot_sprites) = carry.slot_sprites {
            session = session.with_slot_sprites(slot_sprites);
        }
        if let Some(pool) = carry.stellar_sprite_pool {
            session.carry_stellar_sprite_pool(pool);
        }
        if let Some(kept) = carry.kept_berths {
            session = session.with_kept_berths(kept);
        }
        session = session
            .with_hire_terms(hire_terms)
            .with_control_bits(control_bits)
            .with_person_rules(person_rules);
        if let Some(strings) = strings {
            session = session.with_strings(strings);
        }
        match carry.last_refresh {
            Some(carried) => session.with_last_refresh(carried),
            None => session,
        }
    }

    /// What this session hands on to the next flight of the run
    /// ([`CarryOver`]), every part of it: its slot sprites
    /// ([`Session::slot_sprites`]), its stellar sprite pool
    /// ([`Session::stellar_sprite_pool`]), its kept berths
    /// ([`Session::kept_berths`]) and its last refresh
    /// ([`Session::last_refresh`]).
    #[must_use]
    pub fn carry_over(&self) -> CarryOver {
        CarryOver {
            slot_sprites: Some(self.slot_sprites().clone()),
            stellar_sprite_pool: Some(self.stellar_sprite_pool().clone()),
            kept_berths: Some(self.kept_berths()),
            last_refresh: Some(self.last_refresh()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{StellarId, SystemId};
    use crate::rulebook::{RuleKey, RuleSource};
    use crate::ship_layer::{ShipLayer, Slot};
    use crate::testkit::catalog;

    /// String lists naming one ship, with the `l33t` mode on.
    struct Lists;

    impl CommCatalog for Lists {
        fn string_list(&self, id: i16) -> Vec<String> {
            if id == 4003 {
                vec!["Bluebird".to_owned()]
            } else {
                Vec::new()
            }
        }

        fn leet_mode(&self) -> bool {
            true
        }
    }

    /// A setup with every field unlike a built session's own.
    fn unlike_setup() -> SessionSetup {
        let mut slot_sprites = SlotSprites::default();
        slot_sprites.layers.put(Slot::Berth(2), ShipLayer::Led);
        slot_sprites
            .frames
            .clone_frames(Slot::Berth(2), crate::catalog::ShipId(140));
        let mut stellar_sprite_pool = StellarSpritePool::default();
        stellar_sprite_pool.rebuild((2000..2016).map(StellarId));
        stellar_sprite_pool.place(StellarId(2015), (-7, 9));
        stellar_sprite_pool.rebuild([]);
        let mut traffic = crate::Traffic::new();
        traffic.set_berth_person(2, "Ace");
        SessionSetup {
            rules: Rulebook::default()
                .with_override(RuleKey::ColocatedResolution, RuleSource::Bible)
                .with_override(RuleKey::BreakupStall, RuleSource::Bible),
            disable_rule: Rc::new(crate::NovaDisable),
            hire_terms: Rc::new(NovaHire::default()),
            control_bits: Rc::new(NovaBits),
            person_rules: Rc::new(NovaPersons::default()),
            strings: Some(Rc::new(Lists)),
            carry: CarryOver {
                slot_sprites: Some(slot_sprites),
                stellar_sprite_pool: Some(stellar_sprite_pool),
                kept_berths: Some(traffic.kept_berths()),
                last_refresh: Some(LastRefresh::hiding([SystemId(131)])),
            },
        }
    }

    /// `session` set up with `setup` by the builders, one at a time, in
    /// the order the app's edge chained them.
    fn chained(session: Session, setup: SessionSetup) -> Session {
        let carry = setup.carry;
        let mut session = session
            .with_disable_rule(setup.disable_rule)
            .with_rules(setup.rules)
            .with_slot_sprites(carry.slot_sprites.expect("slot sprites"));
        session.carry_stellar_sprite_pool(carry.stellar_sprite_pool.expect("a pool"));
        let session = session
            .with_kept_berths(carry.kept_berths.expect("kept berths"))
            .with_hire_terms(setup.hire_terms)
            .with_control_bits(setup.control_bits)
            .with_person_rules(setup.person_rules)
            .with_strings(setup.strings.expect("lists"));
        session.with_last_refresh(carry.last_refresh.expect("a refresh"))
    }

    #[test]
    fn a_setup_does_what_the_builders_chained_do() {
        let built = Session::start(&catalog()).expect("starts");
        let setup = unlike_setup();
        let mut set_up = built.clone().set_up(setup.clone());
        let chained = chained(built, setup);
        assert_eq!(
            set_up.setup.strings.0.string_list(4003),
            ["Bluebird"],
            "the setup's lists"
        );
        assert!(set_up.setup.leet, "their l33t mode");
        // Each `with_strings` wraps its lists afresh, and the lists
        // compare by their handle.
        set_up.setup.strings = chained.setup.strings.clone();
        assert_eq!(set_up, chained);
    }

    #[test]
    fn a_default_setup_leaves_a_built_session_as_it_was() {
        for catalog in [catalog(), crate::session::tests::three_stellars()] {
            let built = Session::start(&catalog).expect("starts");
            let mut set_up = built.clone().set_up(SessionSetup::default());
            assert_eq!(set_up.setup.strings, built.setup.strings, "its own lists");
            assert_eq!(set_up.chart.carried_refresh, None, "no refresh carried");
            // The ports are Nova's afresh, and compare by their handle.
            set_up.setup.disable_rule = built.setup.disable_rule.clone();
            set_up.setup.hire_terms = built.setup.hire_terms.clone();
            set_up.setup.control_bits = built.setup.control_bits.clone();
            set_up.setup.person_rules = built.setup.person_rules.clone();
            assert_eq!(set_up, built);
        }
    }

    #[test]
    fn the_carry_over_is_the_sessions_history() {
        let setup = unlike_setup();
        let carried = setup.carry.clone();
        let session = Session::start(&catalog()).expect("starts").set_up(setup);
        let over = session.carry_over();
        assert_eq!(over.slot_sprites.as_ref(), Some(session.slot_sprites()));
        assert_eq!(over.slot_sprites, carried.slot_sprites);
        assert_eq!(
            over.stellar_sprite_pool.as_ref(),
            Some(session.stellar_sprite_pool())
        );
        assert_eq!(over.kept_berths, Some(session.kept_berths()));
        assert_eq!(over.kept_berths, carried.kept_berths);
        assert_eq!(over.last_refresh, Some(session.last_refresh()));
    }
}
