//! What the status bar's target panel says of the target at its bottom:
//! its status line, and the label against the bottom right, as
//! `_DrawStatusTarg` (@0x4ace1, the normal path from @0x4b698) in the
//! `EV Nova` executable decides them. The view draws them
//! (`nova_view::flight::target`); the session gathers the facts
//! (`Session::target_status`, `Session::target_tag`).
//!
//! **The status line** ([`status`]), tested in this order:
//!
//! 1. [`TargetStatus::Hidden`] when the ship type's `Flags` 0x0200 is set
//!    ([`HIDES_STATUS`](crate::combat::hull::HIDES_STATUS), byte +0x9e9 &
//!    0x02) or its person's `ShieldMod` is below 0 (+0x61c, which
//!    `_LoadObjectData` sets to `ShieldMod / 100` @0x7c20e-0x7c239: the
//!    engine's [`NpcPerson::invincible`](crate::NpcPerson::invincible))
//!    (@0x4b941-0x4b970).
//! 2. [`TargetStatus::Shield`] while its shield is above 0 and it is not
//!    disabled (@0x4b9a4-0x4b9b6): `trunc(shield / capacity x 100)` in
//!    doubles while the capacity is at least the shield, else 100
//!    (@0x4bce7-0x4bd45).
//! 3. Disabled (`_IsDisabled`, @0x4ba6a): [`TargetStatus::Waiting`] for a
//!    rescue mission's ship held disabled
//!    (`Session::mission_ship_held`) whose armour is not below its
//!    disable threshold (@0x4bab1-0x4bb79; the same four tests
//!    `_IsDisabled` makes for its hold, @0x2d24-0x2d69, and the negation
//!    of its armour test, @0x2d79-0x2dcc), else
//!    [`TargetStatus::Disabled`] (@0x4bb7f).
//! 4. The ship type's `Flags` 0x0100
//!    ([`SHOWS_ARMOR`](crate::combat::hull::SHOWS_ARMOR)) (@0x4bbaa):
//!    [`TargetStatus::ArmorNa`] for an armour capacity of 0 or below,
//!    else [`TargetStatus::Armor`]: 0 for armour of 0 or below, 100 for
//!    armour over the capacity, else `trunc(armour x 100 / capacity)`
//!    (@0x4bbfa-0x4bd28).
//! 5. Otherwise by the ship type's `Shield` field, as `_LoadObjectData`
//!    stores it (+0x5c, a negative `Shield` made 5 x its absolute value,
//!    @0x7a6ea-0x7a6f6), not by the capacity (@0x4bc7a):
//!    [`TargetStatus::ShieldsDown`] above 0, else
//!    [`TargetStatus::NoShields`].
//!
//! **The label** ([`tag`], @0x4bd51): a ship with a government shows its
//! `TargetCode` ([`TargetTag::Code`]). A ship with none shows
//! [`TargetTag::Escort`] or [`TargetTag::Fighter`] while the player leads
//! it (+0x9a is 0: an escort hired or captured, the player's launched
//! fighters, a mission escort), else nothing (@0x4bdf7-0x4bece). It is a
//! fighter when it is one the player launched (AI type 5,
//! `_LaunchFighter` @0x3d771) and its ship type's `Mass` is
//! [`FIGHTER_MASS`] or less (@0x4be0f-0x4be3e).
//!
//! The original also shows "Escort" or "Fighter" for a former escort
//! disabled and dropped from the fleet, by the kind it remembers (+0xc8de,
//! @0x4be01-0x4be21). The engine does not remember it yet: see the
//! `player-led-ships` roadmap's jump clock and self-repair rejoin phase.

use crate::catalog::GovtId;
use crate::reserves::Reserves;

/// The heaviest ship type, in tons, a fighter the player launched is
/// labelled "Fighter" at (`cmpw $0x63` @0x4be35); a heavier one is an
/// "Escort".
pub const FIGHTER_MASS: i16 = 99;

/// What the target panel's status line shows (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetStatus {
    /// Nothing: its type's `Flags` 0x0200 or its person's negative
    /// `ShieldMod` hides it.
    Hidden,
    /// "Disabled" (`STR#` 2002 #347).
    Disabled,
    /// "Waiting" (#348): a rescue mission's ship held disabled, its armour
    /// not below its threshold.
    Waiting,
    /// "Shield:" (#13) and this whole percentage.
    Shield(u32),
    /// "Armor:" (#16) and this whole percentage.
    Armor(u32),
    /// "Armor:" and "N/A" (#396): it can hold no armour.
    ArmorNa,
    /// "No Shields" (#14): its type has no shield.
    NoShields,
    /// "Shields Down" (#15): its type has a shield, and it is down.
    ShieldsDown,
}

/// What the label against the target panel's bottom right shows (see the
/// module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetTag {
    /// Its government's `TargetCode`.
    Code(GovtId),
    /// "Escort" (`STR#` 2002 #168).
    Escort,
    /// "Fighter" (#169).
    Fighter,
}

/// What [`status`] reads of the target.
// Each flag is one of the original's tests, read on its own.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StatusFacts {
    /// Its shield and armour, now and at most.
    pub reserves: Reserves,
    /// Whether it is disabled.
    pub disabled: bool,
    /// Whether a rescue mission holds it disabled whatever its armour.
    pub held: bool,
    /// Whether its armour is below its disable threshold.
    pub below_threshold: bool,
    /// Its ship type's `Flags` 0x0100: its armour shows with its shields
    /// down.
    pub shows_armor: bool,
    /// Its ship type's `Flags` 0x0200: the line is hidden.
    pub hides_status: bool,
    /// Its person's `ShieldMod` is below 0: the line is hidden.
    pub person_hides: bool,
    /// Its ship type's shield points, a negative `Shield` made 5 x its
    /// absolute value.
    pub type_shield: i64,
}

/// What the status line shows of a target with `facts` (see the module
/// docs).
#[must_use]
pub fn status(facts: &StatusFacts) -> TargetStatus {
    let StatusFacts {
        reserves: Reserves { shield, armor, .. },
        ..
    } = *facts;
    if facts.hides_status || facts.person_hides {
        TargetStatus::Hidden
    } else if shield.now > 0.0 && !facts.disabled {
        let (now, cap) = (f64::from(shield.now), f64::from(shield.max));
        TargetStatus::Shield(if cap >= now {
            (now / cap * 100.0) as u32
        } else {
            100
        })
    } else if facts.disabled {
        if facts.held && !facts.below_threshold {
            TargetStatus::Waiting
        } else {
            TargetStatus::Disabled
        }
    } else if facts.shows_armor {
        let cap = f64::from(armor.max);
        if cap <= 0.0 {
            TargetStatus::ArmorNa
        } else if armor.now <= 0.0 {
            TargetStatus::Armor(0)
        } else if f64::from(armor.now) > cap {
            TargetStatus::Armor(100)
        } else {
            TargetStatus::Armor((f64::from(armor.now * 100.0) / cap) as u32)
        }
    } else if facts.type_shield > 0 {
        TargetStatus::ShieldsDown
    } else {
        TargetStatus::NoShields
    }
}

/// What [`tag`] reads of the target.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TagFacts {
    /// Its government, if any.
    pub govt: Option<GovtId>,
    /// Whether the player leads it.
    pub led_by_player: bool,
    /// Whether it is a fighter the player launched.
    pub players_fighter: bool,
    /// Its ship type's `Mass`, raw.
    pub type_mass: i16,
}

/// What the label shows of a target with `facts`, if anything (see the
/// module docs).
#[must_use]
pub fn tag(facts: &TagFacts) -> Option<TargetTag> {
    if let Some(govt) = facts.govt {
        Some(TargetTag::Code(govt))
    } else if !facts.led_by_player {
        None
    } else if facts.players_fighter && facts.type_mass <= FIGHTER_MASS {
        Some(TargetTag::Fighter)
    } else {
        Some(TargetTag::Escort)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reserves::Gauge;

    /// Facts of a target with `shield` of 40 and `armor` of 50, its type
    /// shielded, intact.
    fn facts(shield: f32, armor: f32) -> StatusFacts {
        StatusFacts {
            reserves: Reserves {
                shield: Gauge {
                    now: shield,
                    max: 40.0,
                },
                armor: Gauge {
                    now: armor,
                    max: 50.0,
                },
                fuel: Gauge::full(100.0),
            },
            type_shield: 40,
            ..StatusFacts::default()
        }
    }

    fn with_caps(shield: (f32, f32), armor: (f32, f32)) -> StatusFacts {
        StatusFacts {
            reserves: Reserves {
                shield: Gauge {
                    now: shield.0,
                    max: shield.1,
                },
                armor: Gauge {
                    now: armor.0,
                    max: armor.1,
                },
                fuel: Gauge::default(),
            },
            ..facts(0.0, 0.0)
        }
    }

    #[test]
    fn shields_up_show_their_truncated_percentage() {
        assert_eq!(status(&facts(29.3, 50.0)), TargetStatus::Shield(73));
        assert_eq!(status(&facts(40.0, 50.0)), TargetStatus::Shield(100));
        let tiny = with_caps((0.1, 50.0), (50.0, 50.0));
        assert_eq!(status(&tiny), TargetStatus::Shield(0));
        let just_under = with_caps((39.9, 50.0), (50.0, 50.0));
        assert_eq!(status(&just_under), TargetStatus::Shield(79));
    }

    #[test]
    fn a_shield_over_its_capacity_shows_a_hundred() {
        let over = with_caps((60.0, 50.0), (50.0, 50.0));
        assert_eq!(status(&over), TargetStatus::Shield(100));
        let none_held = with_caps((5.0, 0.0), (50.0, 50.0));
        assert_eq!(status(&none_held), TargetStatus::Shield(100));
    }

    #[test]
    fn a_disabled_ship_says_so_even_with_its_shields_up() {
        let disabled = StatusFacts {
            disabled: true,
            ..facts(30.0, 10.0)
        };
        assert_eq!(status(&disabled), TargetStatus::Disabled);
        let down = StatusFacts {
            disabled: true,
            ..facts(0.0, 10.0)
        };
        assert_eq!(status(&down), TargetStatus::Disabled);
    }

    #[test]
    fn a_held_rescue_ship_not_below_its_threshold_is_waiting() {
        let held = StatusFacts {
            disabled: true,
            held: true,
            ..facts(0.0, 50.0)
        };
        assert_eq!(status(&held), TargetStatus::Waiting);
        let shielded = StatusFacts {
            disabled: true,
            held: true,
            ..facts(40.0, 50.0)
        };
        assert_eq!(status(&shielded), TargetStatus::Waiting);
        let below = StatusFacts {
            below_threshold: true,
            ..held
        };
        assert_eq!(status(&below), TargetStatus::Disabled);
    }

    #[test]
    fn a_held_ship_not_disabled_is_not_waiting() {
        let held = StatusFacts {
            held: true,
            ..facts(0.0, 50.0)
        };
        assert_eq!(status(&held), TargetStatus::ShieldsDown);
    }

    /// Facts of a target whose type shows its armour, shields down.
    fn armoured(armor: f32, capacity: f32) -> StatusFacts {
        StatusFacts {
            shows_armor: true,
            ..with_caps((0.0, 40.0), (armor, capacity))
        }
    }

    #[test]
    fn a_type_that_shows_its_armour_shows_it_with_its_shields_down() {
        assert_eq!(status(&armoured(20.0, 50.0)), TargetStatus::Armor(40));
        assert_eq!(status(&armoured(49.9, 50.0)), TargetStatus::Armor(99));
        assert_eq!(status(&armoured(50.0, 50.0)), TargetStatus::Armor(100));
        assert_eq!(status(&armoured(0.0, 50.0)), TargetStatus::Armor(0));
        assert_eq!(status(&armoured(-5.0, 50.0)), TargetStatus::Armor(0));
        assert_eq!(status(&armoured(0.1, 50.0)), TargetStatus::Armor(0));
        assert_eq!(status(&armoured(80.0, 50.0)), TargetStatus::Armor(100));
    }

    #[test]
    fn armour_at_its_capacity_is_reckoned_in_single_floats_as_the_originals() {
        // Only armour over the capacity is 100 at once; at it, 0.001 x 100
        // in single floats is a hair under 0.1, so it truncates to 99.
        assert_eq!(status(&armoured(0.001, 0.001)), TargetStatus::Armor(99));
    }

    #[test]
    fn a_type_that_shows_its_armour_but_holds_none_shows_n_a() {
        assert_eq!(status(&armoured(10.0, 0.0)), TargetStatus::ArmorNa);
        assert_eq!(status(&armoured(10.0, -5.0)), TargetStatus::ArmorNa);
        assert_eq!(status(&armoured(0.0, 0.0)), TargetStatus::ArmorNa);
    }

    #[test]
    fn a_type_that_shows_its_armour_shows_its_shield_while_up() {
        let up = StatusFacts {
            shows_armor: true,
            ..facts(20.0, 10.0)
        };
        assert_eq!(status(&up), TargetStatus::Shield(50));
    }

    #[test]
    fn otherwise_shields_down_or_no_shields_by_the_types_shield() {
        let down = |type_shield| StatusFacts {
            type_shield,
            ..facts(0.0, 50.0)
        };
        assert_eq!(status(&down(1)), TargetStatus::ShieldsDown);
        assert_eq!(status(&down(5)), TargetStatus::ShieldsDown);
        assert_eq!(status(&down(0)), TargetStatus::NoShields);
        assert_eq!(status(&down(-1)), TargetStatus::NoShields);
        let negative = StatusFacts {
            reserves: Reserves {
                shield: Gauge {
                    now: -3.0,
                    max: 0.0,
                },
                ..facts(0.0, 50.0).reserves
            },
            type_shield: 0,
            ..facts(0.0, 50.0)
        };
        assert_eq!(status(&negative), TargetStatus::NoShields);
    }

    #[test]
    fn the_types_flags_or_an_invincible_person_hide_the_line_first() {
        let by_type = StatusFacts {
            hides_status: true,
            ..facts(20.0, 50.0)
        };
        assert_eq!(status(&by_type), TargetStatus::Hidden);
        let by_person = StatusFacts {
            person_hides: true,
            ..facts(20.0, 50.0)
        };
        assert_eq!(status(&by_person), TargetStatus::Hidden);
        let disabled = StatusFacts {
            disabled: true,
            held: true,
            ..by_type
        };
        assert_eq!(status(&disabled), TargetStatus::Hidden);
        let armour = StatusFacts {
            person_hides: true,
            ..armoured(20.0, 50.0)
        };
        assert_eq!(status(&armour), TargetStatus::Hidden);
    }

    #[test]
    fn a_government_shows_its_code() {
        let facts = TagFacts {
            govt: Some(GovtId(128)),
            led_by_player: true,
            players_fighter: true,
            type_mass: 10,
        };
        assert_eq!(tag(&facts), Some(TargetTag::Code(GovtId(128))));
    }

    #[test]
    fn a_ship_the_player_leads_with_no_government_is_an_escort() {
        let facts = TagFacts {
            led_by_player: true,
            type_mass: 10,
            ..TagFacts::default()
        };
        assert_eq!(tag(&facts), Some(TargetTag::Escort));
    }

    #[test]
    fn the_players_fighter_up_to_ninety_nine_tons_is_a_fighter() {
        let fighter = |type_mass| TagFacts {
            led_by_player: true,
            players_fighter: true,
            type_mass,
            ..TagFacts::default()
        };
        assert_eq!(FIGHTER_MASS, 99);
        assert_eq!(tag(&fighter(99)), Some(TargetTag::Fighter));
        assert_eq!(tag(&fighter(-1)), Some(TargetTag::Fighter));
        assert_eq!(tag(&fighter(100)), Some(TargetTag::Escort));
    }

    #[test]
    fn a_ship_the_player_does_not_lead_with_no_government_shows_nothing() {
        let traffic = TagFacts::default();
        assert_eq!(tag(&traffic), None);
        let escorts_fighter = TagFacts {
            players_fighter: true,
            type_mass: 10,
            ..TagFacts::default()
        };
        assert_eq!(tag(&escorts_fighter), None);
    }
}
