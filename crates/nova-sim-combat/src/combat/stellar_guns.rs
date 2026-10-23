//! The stellar defence guns: `_HandleStellarWeapons` (@0x2ef3a-0x2f5d1),
//! which `_DoPlayGameWork` calls after `_HandleStellarSprites`
//! (@0x444d7), after the ship loop and before `_HandleBeams`, on either
//! pass; with the threat test it asks of each ship, `_IsThreatToStellar`
//! (@0x9ef7-0xa0ae), and the lead it fires at, `_CalcSpobLeadAngle`
//! (@0x2b2f0-0x2b42e) (stellar-fidelity phase 13).

use crate::catalog::{GovtId, StellarId, WeaponId};
use crate::geometry::Vec2;
use crate::govt::Governments;

use super::ShipRef;

/// The `spöb` `Flags2` bit that holds a stellar's fire until a defence
/// ship of it is in the system, and then turns it on the player and the
/// ships the player leads directly, whatever their standing
/// (`_IsThreatToStellar` @0x9f30-0x9f5c).
pub const GUARDED_BY_DEFENDERS: u16 = 0x0200;

/// A stellar gun's reload and burst count (the original's spöb +0x494 and
/// +0x2e): zeroed by the daily step and never saved.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GunTimer {
    /// The ticks until it may fire again: it fires once this is 0 or
    /// below.
    pub reload: f32,
    /// The shots fired in its burst so far.
    pub burst: u32,
}

/// The player's standing with a stellar, as `_IsThreatToStellar` reads it
/// for the player and the ships it leads directly (@0x9f79-0xa07e).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Standing {
    /// The government it is judged by, `G`: the stellar's own, or else the
    /// player's system's; none when both are independent.
    pub govt: Option<GovtId>,
    /// The player's legal record, as `RuleKey::LegalRecords` reads it.
    pub record: i16,
    /// The player's class's combat government (`shïp` +0x9f4).
    pub class_govt: Option<GovtId>,
    /// Whether the stellar's own government blesses the player with peace
    /// (the inlined `_PlayerBlessedByGovt(own, 0)` @0xa02e-0xa04a): never
    /// for an independent stellar.
    pub blessed: bool,
    /// Whether `G` carries the IFF scrambler's mark (gövt +0x83,
    /// @0xa05f-0xa06a).
    pub iff_marked: bool,
    /// Whether the stellar is the player's dock target (`_dockTarget`,
    /// @0xa06e-0xa07a).
    pub docking: bool,
}

/// A stellar with a gun, in the player's system, as the session builds it
/// for a tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StellarGun {
    /// Which stellar.
    pub stellar: StellarId,
    /// Where it is: its whole-pixel position, which its shots leave from.
    pub position: Vec2,
    /// Its weapon.
    pub weapon: WeaponId,
    /// Its own government, none for an independent stellar.
    pub govt: Option<GovtId>,
    /// Its `Flags2`, raw.
    pub flags2: u16,
    /// Whether it is dominated (+0x46): it then sees no threat.
    pub dominated: bool,
    /// Whether a defence ship of it is in the system (+0x47): what a
    /// [`GUARDED_BY_DEFENDERS`] stellar waits for.
    pub defended: bool,
    /// The player's standing with it.
    pub standing: Standing,
    /// Its reload and burst count.
    pub timer: GunTimer,
}

/// What `_IsThreatToStellar` reads of a ship.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Suspect {
    /// Which ship.
    pub ship: ShipRef,
    /// Its government, none for an independent ship or the player.
    pub govt: Option<GovtId>,
    /// The ship it follows ([`Fighter::leader`](super::Fighter::leader)).
    pub leader: Option<ShipRef>,
    /// Whether it is jumping in ([`Fighter::jumping_in`](super::Fighter::jumping_in)).
    pub jumping_in: bool,
}

/// Whether `suspect` threatens `gun`, as `_IsThreatToStellar` decides
/// (@0x9ef7-0xa0ae), with the relations in `govts`, in its order:
///
/// 1. A dominated stellar sees no threat (@0x9f0c).
/// 2. A ship other than the player jumping in is none (@0x9f16-0x9f2a).
/// 3. A [`GUARDED_BY_DEFENDERS`] stellar sees none until it is
///    [`defended`](StellarGun::defended), and then sees the player and the
///    ships it leads directly, with no other test, and nothing else
///    (@0x9f30-0x9f5c).
/// 4. Any other ship but the player's side is a threat when it is an
///    enemy of the stellar's own government, never the system's
///    (`_GovtEnemies` @0x9f61-0x9f73 → @0xa080): an independent stellar's
///    is a xenophobe's alone.
/// 5. The player and the ships it leads directly are judged by the
///    player's [`Standing`] (@0x9f79-0xa07e) (see `player_side_threatens`).
#[must_use]
pub fn threatens(gun: &StellarGun, suspect: &Suspect, govts: &Governments) -> bool {
    if gun.dominated {
        return false;
    }
    let player = suspect.ship == ShipRef::Player;
    if !player && suspect.jumping_in {
        return false;
    }
    let player_side = player || suspect.leader == Some(ShipRef::Player);
    if gun.flags2 & GUARDED_BY_DEFENDERS != 0 {
        return gun.defended && player_side;
    }
    if player_side {
        player_side_threatens(gun, govts)
    } else {
        govts.enemies(gun.govt, suspect.govt)
    }
}

/// Whether the player's side threatens `gun` (@0x9f79-0xa07e): with `G`
/// the government it is judged by ([`Standing::govt`]; none, no threat),
/// it is a candidate when the record is below −`CrimeTol(G)`; else, for a
/// xenophobic `G`, exactly when the record is below 0; else when the
/// player's class government is an enemy of the stellar's own government.
/// A candidate is spared by the own government's blessing, `G`'s IFF mark
/// or the stellar being the player's dock target.
fn player_side_threatens(gun: &StellarGun, govts: &Governments) -> bool {
    let standing = &gun.standing;
    let Some(judge) = standing.govt else {
        return false;
    };
    let record = i32::from(standing.record);
    let candidate = if record < -i32::from(govts.crime_tol(Some(judge))) {
        true
    } else if govts.xenophobic(Some(judge)) {
        record < 0
    } else {
        govts.enemies(standing.class_govt, gun.govt)
    };
    candidate && !standing.blessed && !standing.iff_marked && !standing.docking
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::GovtRecord;
    use crate::combat::NpcId;
    use crate::govt::XENOPHOBIC;
    use nova_sim_catalog::fixture::govt;

    const FEDERATION: GovtId = GovtId(128);
    const PIRATES: GovtId = GovtId(129);
    const LOCALS: GovtId = GovtId(130);
    const ALIENS: GovtId = GovtId(131);
    const ESCORT: ShipRef = ShipRef::Npc(NpcId(1));
    const NPC: ShipRef = ShipRef::Npc(NpcId(2));

    /// The Federation (class 1, `CrimeTol` 6) is the pirates' (class 2)
    /// enemy, the locals (class 3, `CrimeTol` 10) are at peace with
    /// everyone, and the aliens are xenophobic.
    fn govts() -> Governments {
        let federation = GovtRecord {
            classes: [1, -1, -1, -1],
            enemies: [2, -1, -1, -1],
            crime_tol: 6,
            ..govt(128)
        };
        let pirates = GovtRecord {
            classes: [2, -1, -1, -1],
            ..govt(129)
        };
        let locals = GovtRecord {
            classes: [3, -1, -1, -1],
            enemies: [2, -1, -1, -1],
            crime_tol: 10,
            ..govt(130)
        };
        let aliens = GovtRecord {
            classes: [4, -1, -1, -1],
            flags: XENOPHOBIC,
            ..govt(131)
        };
        Governments::new([federation, pirates, locals, aliens])
    }

    /// A Federation stellar, the player in good standing, flying a
    /// Federation class.
    fn gun() -> StellarGun {
        StellarGun {
            stellar: StellarId(128),
            position: Vec2::ZERO,
            weapon: WeaponId(196),
            govt: Some(FEDERATION),
            flags2: 0,
            dominated: false,
            defended: false,
            standing: Standing {
                govt: Some(FEDERATION),
                class_govt: Some(FEDERATION),
                ..Standing::default()
            },
            timer: GunTimer::default(),
        }
    }

    /// The gun with the player's record at `record`.
    fn at_record(gun: StellarGun, record: i16) -> StellarGun {
        StellarGun {
            standing: Standing {
                record,
                ..gun.standing
            },
            ..gun
        }
    }

    fn player() -> Suspect {
        Suspect {
            ship: ShipRef::Player,
            govt: None,
            leader: None,
            jumping_in: false,
        }
    }

    /// A ship of `govt` the player leads directly.
    fn escort(govt: Option<GovtId>) -> Suspect {
        Suspect {
            ship: ESCORT,
            govt,
            leader: Some(ShipRef::Player),
            jumping_in: false,
        }
    }

    /// An NPC of `govt` no one leads.
    fn npc(govt: Option<GovtId>) -> Suspect {
        Suspect {
            ship: NPC,
            govt,
            leader: None,
            jumping_in: false,
        }
    }

    fn threat(gun: &StellarGun, suspect: &Suspect) -> bool {
        threatens(gun, suspect, &govts())
    }

    #[test]
    fn a_dominated_stellar_sees_no_threat() {
        let dominated = StellarGun {
            dominated: true,
            ..at_record(gun(), -1000)
        };
        for suspect in [player(), escort(Some(PIRATES)), npc(Some(PIRATES))] {
            assert!(threat(&at_record(gun(), -1000), &player()));
            assert!(!threat(&dominated, &suspect), "{suspect:?}");
        }
    }

    #[test]
    fn an_npc_is_a_threat_when_an_enemy_of_the_stellars_own_government_unless_jumping_in() {
        assert!(threat(&gun(), &npc(Some(PIRATES))), "an enemy");
        let arriving = Suspect {
            jumping_in: true,
            ..npc(Some(PIRATES))
        };
        assert!(!threat(&gun(), &arriving), "jumping in");
        assert!(!threat(&gun(), &npc(Some(FEDERATION))), "its own");
        assert!(!threat(&gun(), &npc(Some(LOCALS))), "at peace");
        assert!(!threat(&gun(), &npc(None)), "independent");
    }

    #[test]
    fn an_independent_stellar_in_a_governed_system_threatens_only_xenophobes_among_npcs() {
        let independent = StellarGun {
            govt: None,
            ..gun()
        };
        assert!(
            !threat(&independent, &npc(Some(PIRATES))),
            "the system government's enemy"
        );
        assert!(threat(&independent, &npc(Some(ALIENS))), "a xenophobe");
    }

    #[test]
    fn a_ship_the_player_leads_jumping_in_is_no_threat() {
        let criminal = at_record(gun(), -1000);
        let arriving = Suspect {
            jumping_in: true,
            ..escort(None)
        };
        assert!(threat(&criminal, &escort(None)));
        assert!(!threat(&criminal, &arriving));
    }

    #[test]
    fn a_guarded_stellar_undefended_sees_no_threat_at_all() {
        let guarded = StellarGun {
            flags2: GUARDED_BY_DEFENDERS,
            ..at_record(gun(), -1000)
        };
        for suspect in [player(), escort(None), npc(Some(PIRATES))] {
            assert!(!threat(&guarded, &suspect), "{suspect:?}");
        }
    }

    #[test]
    fn a_guarded_stellar_defended_threatens_the_player_and_its_escorts_whatever_their_standing() {
        let guarded = StellarGun {
            flags2: GUARDED_BY_DEFENDERS,
            defended: true,
            standing: Standing {
                record: 100,
                blessed: true,
                iff_marked: true,
                docking: true,
                ..gun().standing
            },
            ..gun()
        };
        assert!(threat(&guarded, &player()));
        assert!(threat(&guarded, &escort(Some(FEDERATION))));
        assert!(!threat(&guarded, &npc(Some(PIRATES))), "an enemy NPC");
        let second_rank = Suspect {
            leader: Some(ESCORT),
            ..npc(None)
        };
        assert!(!threat(&guarded, &second_rank), "led by an escort");
    }

    #[test]
    fn the_player_is_a_threat_below_minus_crime_tol_or_as_an_enemy_of_the_stellars_own_government()
    {
        assert!(threat(&at_record(gun(), -7), &player()), "-CrimeTol - 1");
        assert!(!threat(&at_record(gun(), -6), &player()), "-CrimeTol");
        let pirate_class = StellarGun {
            standing: Standing {
                class_govt: Some(PIRATES),
                ..gun().standing
            },
            ..gun()
        };
        assert!(
            threat(&at_record(pirate_class, -6), &player()),
            "an enemy class"
        );
        assert!(threat(&pirate_class, &player()), "an enemy class at 0");
    }

    #[test]
    fn a_xenophobic_government_threatens_the_player_at_any_record_below_zero_alone() {
        let alien = StellarGun {
            govt: Some(ALIENS),
            standing: Standing {
                govt: Some(ALIENS),
                class_govt: Some(PIRATES),
                ..Standing::default()
            },
            ..gun()
        };
        assert!(threat(&at_record(alien, -1), &player()));
        assert!(
            !threat(&at_record(alien, 0), &player()),
            "though the class is its enemy"
        );
    }

    #[test]
    fn an_independent_stellar_reads_the_system_governments_tolerance_but_its_own_enemies() {
        let independent = StellarGun {
            govt: None,
            standing: Standing {
                govt: Some(FEDERATION),
                class_govt: Some(PIRATES),
                ..Standing::default()
            },
            ..gun()
        };
        assert!(
            !threat(&independent, &player()),
            "the system government's enemy class"
        );
        assert!(
            threat(&at_record(independent, -7), &player()),
            "its CrimeTol"
        );
        let alien_class = StellarGun {
            standing: Standing {
                class_govt: Some(ALIENS),
                ..independent.standing
            },
            ..independent
        };
        assert!(threat(&alien_class, &player()), "a xenophobic class");
        let xenophobic_system = StellarGun {
            standing: Standing {
                govt: Some(ALIENS),
                class_govt: None,
                record: -1,
                ..Standing::default()
            },
            ..independent
        };
        assert!(threat(&xenophobic_system, &player()), "its xenophobia");
    }

    #[test]
    fn an_independent_stellar_in_an_independent_system_never_threatens_the_player_side() {
        let nowhere = StellarGun {
            govt: None,
            standing: Standing {
                govt: None,
                class_govt: Some(ALIENS),
                record: -1000,
                ..Standing::default()
            },
            ..gun()
        };
        assert!(!threat(&nowhere, &player()));
        assert!(!threat(&nowhere, &escort(Some(ALIENS))));
        assert!(threat(&nowhere, &npc(Some(ALIENS))), "an NPC xenophobe");
    }

    #[test]
    fn an_escort_is_judged_by_the_players_standing_not_its_own_government() {
        assert!(!threat(&gun(), &escort(Some(PIRATES))), "in good standing");
        assert!(threat(&at_record(gun(), -7), &escort(Some(FEDERATION))));
    }

    #[test]
    fn the_stellars_own_government_blessing_peace_spares_the_player_side() {
        let blessed = StellarGun {
            standing: Standing {
                blessed: true,
                ..gun().standing
            },
            ..gun()
        };
        let criminal = at_record(blessed, -1000);
        assert!(!threat(&criminal, &player()));
        assert!(!threat(&criminal, &escort(None)));
        assert!(threat(&criminal, &npc(Some(PIRATES))), "NPCs unaffected");
    }

    #[test]
    fn a_system_governments_blessing_does_not_spare_at_an_independent_stellar() {
        // The session never sets `blessed` for an independent stellar: the
        // inlined `_PlayerBlessedByGovt(-1, 0)` is 0 (@0xa02e-0xa04a). So a
        // criminal is shot whatever the system's government thinks.
        let independent = StellarGun {
            govt: None,
            ..at_record(gun(), -7)
        };
        assert!(threat(&independent, &player()));
    }

    #[test]
    fn the_effective_governments_iff_mark_spares_the_player_side() {
        let marked = |gun: StellarGun| StellarGun {
            standing: Standing {
                iff_marked: true,
                ..gun.standing
            },
            ..gun
        };
        let criminal = at_record(gun(), -1000);
        assert!(!threat(&marked(criminal), &player()));
        assert!(!threat(&marked(criminal), &escort(None)));
        assert!(
            threat(&marked(gun()), &npc(Some(PIRATES))),
            "NPCs unaffected"
        );
        let independent = StellarGun {
            govt: None,
            ..criminal
        };
        assert!(threat(&independent, &player()));
        assert!(
            !threat(&marked(independent), &player()),
            "the system government's mark"
        );
    }

    #[test]
    fn the_players_dock_target_spares_the_player_side_alone() {
        let docking = StellarGun {
            standing: Standing {
                docking: true,
                ..gun().standing
            },
            ..at_record(gun(), -1000)
        };
        assert!(!threat(&docking, &player()));
        assert!(!threat(&docking, &escort(None)));
        assert!(threat(&docking, &npc(Some(PIRATES))));
    }
}
