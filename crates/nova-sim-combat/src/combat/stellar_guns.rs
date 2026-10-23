//! The stellar defence guns: `_HandleStellarWeapons` (@0x2ef3a-0x2f5d1),
//! which `_DoPlayGameWork` calls after `_HandleStellarSprites`
//! (@0x444d7), after the ship loop and before `_HandleBeams`, on either
//! pass; with the threat test it asks of each ship, `_IsThreatToStellar`
//! (@0x9ef7-0xa0ae), and the lead it fires at, `_CalcSpobLeadAngle`
//! (@0x2b2f0-0x2b42e) (stellar-fidelity phase 13).

use crate::catalog::{GovtId, StellarId, WeaponId};
use crate::chance::Chance;
use crate::geometry::Vec2;
use crate::govt::Governments;
use crate::ship_layer::Slot;
use crate::trig::{self, LeadShot};

use super::armament::{self, Arsenal};
use super::jamming;
use super::phases::Arena;
use super::projectile::{Jam, Shot, ShotId, ShotLayer, Source};
use super::weapon::{Guidance, WeaponSpec};
use super::{Combat, CombatEvent, Fighter, FireSound, STELLAR_GUN_PRIORITY, ShipRef};

/// What a ready gun's reload loses a tick: `_gSpeedMult`, 1.0
/// (@0x2f5a1-0x2f5b2).
const SPEED_MULT: f32 = 1.0;

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

/// What a stellar's gun reads of a ship as it picks its target
/// (@0x2f003-0x2f11a) and leads it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mark {
    /// What the threat test reads of it.
    pub suspect: Suspect,
    /// Its ship slot: the walk goes by it, the player first.
    pub slot: Slot,
    /// Where it is.
    pub position: Vec2,
    /// How it moves, which the gun leads.
    pub velocity: Vec2,
    /// Whether it is in the ship table (+0xb8): not destroyed.
    pub active: bool,
    /// Whether it is cloaked (`_ShipIsCloaked` @0x2f074).
    pub cloaked: bool,
    /// Whether `_IsDisabled` holds for it (@0x2f0d6): not intact, or held
    /// disabled.
    pub disabled: bool,
}

impl Mark {
    /// `fighter` as the gun reads it.
    #[must_use]
    pub fn of(fighter: &Fighter) -> Self {
        Self {
            suspect: Suspect {
                ship: fighter.ship,
                govt: fighter.govt,
                leader: fighter.leader,
                jumping_in: fighter.jumping_in,
            },
            slot: fighter.slot,
            position: fighter.state.position,
            velocity: fighter.state.velocity,
            active: fighter.condition.active(),
            cloaked: fighter.cloaked,
            disabled: fighter.disabled_as_tick_began(),
        }
    }
}

/// The ship `gun`, firing `spec`, picks among `marks`, as
/// `_HandleStellarWeapons` does (@0x2f003-0x2f11a): walking them by slot,
/// the player first, it takes the nearest that is active, not cloaked, a
/// threat ([`threatens`]) and within the weapon's range ([`WeaponSpec::range`]),
/// its squared distance at most the range squared, in `f32`. A
/// disable-only or massless weapon (`MassDmg` 0) passes over a disabled
/// ship (@0x2f01e-0x2f033, @0x2f0d6). A new best needs a strictly smaller
/// distance, so a tie keeps the lower slot. Its index among `marks`.
#[must_use]
pub fn pick(
    gun: &StellarGun,
    spec: &WeaponSpec,
    marks: &[Mark],
    govts: &Governments,
) -> Option<usize> {
    let reach = spec.range() * spec.range();
    let spares_disabled = spec.disable_only() || spec.mass_damage == 0.0;
    let mut order: Vec<usize> = (0..marks.len()).collect();
    order.sort_by_key(|&index| marks[index].slot);
    let mut best: Option<(usize, f32)> = None;
    for index in order {
        let mark = &marks[index];
        if !mark.active
            || mark.cloaked
            || !threatens(gun, &mark.suspect, govts)
            || (spares_disabled && mark.disabled)
        {
            continue;
        }
        let (dx, dy) = (
            mark.position.x - gun.position.x,
            mark.position.y - gun.position.y,
        );
        let distance = dx * dx + dy * dy;
        // NaN fails both tests.
        let nearer = best.is_none_or(|(_, nearest)| distance < nearest);
        if distance <= reach && nearer {
            best = Some((index, distance));
        }
    }
    best.map(|(index, _)| index)
}

impl Combat {
    /// `_HandleStellarWeapons` (@0x2ef3a-0x2f5d1): each of `guns`, in the
    /// player's system's `NavDefs` order, fires at its target among
    /// `fighters` as its timer allows, by `arena`'s arsenal and
    /// governments, its inaccuracy and jamming rolls drawn on `chance`
    /// (see `Combat::guns_fire`). The session runs it after the ship
    /// loop and before `_HandleBeams`, on either pass, and keeps each
    /// gun's timer.
    pub fn fire_stellar_guns(
        &mut self,
        fighters: &[Fighter],
        arena: Arena<'_>,
        guns: &mut [StellarGun],
        chance: &mut (impl Chance + ?Sized),
    ) {
        self.field = arena.field;
        let mut source = &mut *chance;
        let chance: &mut dyn Chance = &mut source;
        let marks: Vec<Mark> = fighters.iter().map(Mark::of).collect();
        self.guns_fire(&marks, arena.arsenal, arena.govts, guns, chance);
    }

    /// Each of `guns` with a weapon in `arsenal`, among `marks`:
    ///
    /// - while its reload is above 0 it loses a tick and does nothing
    ///   else (@0x2efee-0x2eff9, @0x2f5a1-0x2f5b2), threat or none;
    /// - once ready it picks its target ([`pick`]); with none, nothing
    ///   happens and it stays ready;
    /// - with no free shot slot ([`Combat::seat`]), nothing happens either
    ///   (@0x2f120-0x2f151);
    /// - else it fires ([`Combat::stellar_shot`]), its reload becomes its
    ///   weapon's `Reload`, and a bursting weapon counts the shot, the
    ///   reload becoming `BurstReload` once the count reaches
    ///   `BurstCount` (`_WeaponBurstInterval(weapon, 1)` @0x831c), the
    ///   count back to 0 (@0x2f4be-0x2f545). So with `Reload` R it fires
    ///   every R + 1 ticks.
    /// - Each shot raises [`CombatEvent::StellarFired`] (@0x2f553-0x2f59a).
    fn guns_fire(
        &mut self,
        marks: &[Mark],
        arsenal: &Arsenal,
        govts: &Governments,
        guns: &mut [StellarGun],
        chance: &mut dyn Chance,
    ) {
        for gun in guns.iter_mut() {
            let Some(spec) = arsenal.weapon(gun.weapon).copied() else {
                continue;
            };
            if gun.timer.reload > 0.0 {
                gun.timer.reload -= SPEED_MULT;
                continue;
            }
            let Some(target) = pick(gun, &spec, marks, govts) else {
                continue;
            };
            if self.first_free_slot().is_none() {
                continue;
            }
            self.stellar_shot(gun, spec, &marks[target], chance);
            gun.timer.reload = spec.reload.trunc();
            if spec.burst_count > 0 {
                gun.timer.burst += 1;
                if gun.timer.burst >= armament::burst_interval(&spec, 1) {
                    gun.timer.burst = 0;
                    gun.timer.reload = spec.burst_reload;
                }
            }
            self.events.push(CombatEvent::StellarFired {
                stellar: gun.stellar,
                weapon: spec.id,
                at: gun.position,
                sound: FireSound {
                    priority: STELLAR_GUN_PRIORITY,
                    unless_playing: false,
                },
            });
        }
    }

    /// Puts `gun`'s shot of `spec` at `target` in flight, as
    /// `_HandleStellarWeapons` builds it (@0x2f157-0x2f46f), a slot being
    /// free:
    ///
    /// - from the stellar's position, launched by no ship
    ///   ([`Source::Other`], source -1), so it hits only its target; its
    ///   target and the ship its damage is aimed at are the target, and
    ///   so are its firer and fleet, which nothing reads of such a shot;
    /// - headed at the lead angle from the stellar, which has no velocity
    ///   (`_CalcSpobLeadAngle` @0x2b2f0, [`trig::lead_angle`]; a rocket's
    ///   timing for guidance 6, and no guidance gate), turned by its
    ///   inaccuracy ([`armament::inaccuracy`]) and wrapped once
    ///   ([`trig::wrap_once`], @0x2f2f1-0x2f3c9);
    /// - its velocity `_Accel(trunc(heading), Speed)` from rest
    ///   (@0x2f3ce-0x2f3fe);
    /// - its jamming rolls drawn ([`jamming::roll`], @0x2f40c-0x2f454),
    ///   with no sensor-interference roll: it is never interfered;
    /// - its disable-only and durability its weapon's;
    /// - in the shot layer whatever its guidance (@0x2f46f);
    /// - seated as [`Combat::seat`] seats it, writing no miss roll and,
    ///   by [`RuleKey::ReusedShotSlotRoll`](crate::RuleKey::ReusedShotSlotRoll)'s
    ///   engine reading, carrying the slot's leftover; its spin timer is
    ///   written 0 (@0x2f26d) whatever
    ///   [`RuleKey::ReusedShotSlotSpin`](crate::RuleKey::ReusedShotSlotSpin)
    ///   says.
    fn stellar_shot(
        &mut self,
        gun: &StellarGun,
        spec: WeaponSpec,
        target: &Mark,
        chance: &mut dyn Chance,
    ) {
        let lead = LeadShot {
            speed: spec.speed,
            rocket: spec.guidance == Guidance::Rocket,
        };
        let aim = trig::lead_angle(
            gun.position,
            Vec2::ZERO,
            (target.position, target.velocity),
            Some(lead),
        );
        let heading =
            trig::wrap_once(f32::from(aim) + armament::inaccuracy(spec.inaccuracy, chance));
        let ship = target.suspect.ship;
        let mut shot = Shot {
            weapon: spec,
            firer: ship,
            fleet: ship,
            source: Source::Other,
            position: gun.position,
            velocity: trig::accel(trig::whole(heading), spec.speed),
            heading,
            age: 0,
            id: ShotId::default(),
            target: Some(ship),
            aimed_at: Some(ship),
            jam: Jam::Clear,
            durability: spec.durability,
            generation: 0,
            disable_only: spec.disable_only(),
            miss_roll: None,
            layer: ShotLayer::Shots,
            turret_over: false,
            jam_rolls: jamming::roll(spec.jam_vuln, chance),
            moved_from: None,
            slot: 0,
            spin_timer: 0,
            firer_slot: None,
        };
        let Some(taken) = self.seat(&mut shot) else {
            return;
        };
        self.slots.reset_spin(taken);
        shot.spin_timer = 0;
        shot.id = self.next_id();
        self.insert_seated(&shot);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::{GovtRecord, WeaponRecord};
    use crate::chance::NeverFires;
    use crate::combat::weapon::DISABLE_ONLY;
    use crate::combat::{MAX_SHOTS, NpcId};
    use crate::govt::XENOPHOBIC;
    use crate::rulebook::RuleSource;
    use nova_sim_base::fixture::Draws;
    use nova_sim_catalog::fixture::{govt, weapon};

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

    /// The stellar's weapon: reaching 600 pixels, of `MassDmg` 10.
    fn blaster() -> WeaponSpec {
        let record = WeaponRecord {
            mass_dmg: 10,
            ..weapon(196)
        };
        WeaponSpec {
            chain_range: 600.0,
            ..WeaponSpec::new(&record)
        }
    }

    /// `suspect` at slot `slot`, at (`x`, `y`), at rest, active, intact
    /// and seen.
    fn mark(suspect: Suspect, slot: Slot, x: f32, y: f32) -> Mark {
        Mark {
            suspect,
            slot,
            position: Vec2::new(x, y),
            velocity: Vec2::ZERO,
            active: true,
            cloaked: false,
            disabled: false,
        }
    }

    /// An enemy NPC at berth `berth`, at (`x`, `y`).
    fn pirate(berth: u32, x: f32, y: f32) -> Mark {
        mark(npc(Some(PIRATES)), Slot::Berth(berth), x, y)
    }

    fn picked(gun: &StellarGun, spec: &WeaponSpec, marks: &[Mark]) -> Option<usize> {
        pick(gun, spec, marks, &govts())
    }

    #[test]
    fn the_gun_picks_the_nearest_threat() {
        let criminal = at_record(gun(), -1000);
        let marks = [
            mark(player(), Slot::Player, 300.0, 0.0),
            pirate(1, 0.0, 200.0),
            pirate(2, 0.0, -250.0),
        ];
        assert_eq!(picked(&criminal, &blaster(), &marks), Some(1));
    }

    #[test]
    fn on_a_tie_the_gun_picks_the_lower_slot_the_player_first() {
        let criminal = at_record(gun(), -1000);
        let marks = [
            pirate(3, 0.0, 300.0),
            pirate(2, 0.0, -300.0),
            mark(player(), Slot::Player, 300.0, 0.0),
        ];
        assert_eq!(picked(&criminal, &blaster(), &marks), Some(2));
        assert_eq!(picked(&criminal, &blaster(), &marks[..2]), Some(1));
    }

    #[test]
    fn the_gun_passes_over_a_destroyed_a_cloaked_or_a_harmless_ship() {
        let destroyed = Mark {
            active: false,
            ..pirate(1, 10.0, 0.0)
        };
        let cloaked = Mark {
            cloaked: true,
            ..pirate(2, 20.0, 0.0)
        };
        let friend = mark(npc(Some(FEDERATION)), Slot::Berth(3), 30.0, 0.0);
        let marks = [destroyed, cloaked, friend, pirate(4, 40.0, 0.0)];
        assert_eq!(picked(&gun(), &blaster(), &marks), Some(3));
        assert_eq!(picked(&gun(), &blaster(), &marks[..3]), None);
    }

    #[test]
    fn the_gun_reaches_exactly_its_range_and_no_further() {
        let edge = [pirate(1, 600.0, 0.0)];
        assert_eq!(picked(&gun(), &blaster(), &edge), Some(0), "r² is in");
        let beyond = [pirate(1, 600.0_f32.next_up(), 0.0)];
        assert_eq!(picked(&gun(), &blaster(), &beyond), None);
        let nan = [pirate(1, f32::NAN, 0.0)];
        assert_eq!(picked(&gun(), &blaster(), &nan), None, "NaN fails");
    }

    #[test]
    fn the_range_is_measured_from_the_stellar_wherever_it_is() {
        let away = StellarGun {
            position: Vec2::new(1000.0, -1000.0),
            ..gun()
        };
        let near = [pirate(1, 1100.0, -1050.0)];
        assert_eq!(picked(&away, &blaster(), &near), Some(0));
        let at_origin = [pirate(1, 0.0, 0.0)];
        assert_eq!(picked(&away, &blaster(), &at_origin), None);
    }

    #[test]
    fn a_disable_only_or_massless_gun_passes_over_a_disabled_ship_and_an_ordinary_one_does_not() {
        let disabled = Mark {
            disabled: true,
            ..pirate(1, 10.0, 0.0)
        };
        let marks = [disabled, pirate(2, 50.0, 0.0)];
        assert_eq!(picked(&gun(), &blaster(), &marks), Some(0), "ordinary");
        let only_disables = WeaponSpec {
            flags2: DISABLE_ONLY,
            ..blaster()
        };
        assert_eq!(
            picked(&gun(), &only_disables, &marks),
            Some(1),
            "disable-only"
        );
        let ion = WeaponSpec {
            mass_damage: 0.0,
            ..blaster()
        };
        assert_eq!(picked(&gun(), &ion, &marks), Some(1), "massless");
        assert_eq!(picked(&gun(), &ion, &marks[..1]), None);
    }

    // The shot and the timers.

    /// A blaster turret, 196, of `Speed` 1000 (10 pixels a tick), `Count`
    /// 50 and `Reload` 3: reaching 500 pixels.
    fn turret() -> WeaponRecord {
        WeaponRecord {
            reload: 3,
            count: 50,
            speed: 1000,
            mass_dmg: 10,
            guidance: -1,
            ..weapon(196)
        }
    }

    /// The arsenal of `record`.
    fn arsenal_of(record: WeaponRecord) -> Arsenal {
        Arsenal::new(&[record], Vec::new())
    }

    /// A pirate at (`x`, `y`) moving at `velocity`.
    fn moving_pirate(x: f32, y: f32, velocity: Vec2) -> Mark {
        Mark {
            velocity,
            ..pirate(1, x, y)
        }
    }

    /// `guns` fire once among `marks` with `record` in the arsenal.
    fn fire(
        combat: &mut Combat,
        guns: &mut [StellarGun],
        marks: &[Mark],
        record: WeaponRecord,
        chance: &mut dyn Chance,
    ) {
        let arsenal = arsenal_of(record);
        combat.guns_fire(marks, &arsenal, &govts(), guns, chance);
    }

    /// The one shot `record`'s gun fires at `marks`, its draws on
    /// `chance`.
    fn one_shot(record: WeaponRecord, marks: &[Mark], chance: &mut dyn Chance) -> Shot {
        let mut combat = Combat::default();
        fire(&mut combat, &mut [gun()], marks, record, chance);
        assert_eq!(combat.shots().len(), 1);
        combat.shots()[0]
    }

    /// The ticks out of `ticks` on which `record`'s gun fires at a pirate
    /// in range, its timer starting at `timer`.
    fn firing_ticks(record: WeaponRecord, timer: GunTimer, ticks: u32) -> Vec<u32> {
        let mut combat = Combat::default();
        let mut guns = [StellarGun { timer, ..gun() }];
        let marks = [pirate(1, 100.0, 0.0)];
        let mut fired = Vec::new();
        for tick in 0..ticks {
            let before = combat.shots().len();
            fire(&mut combat, &mut guns, &marks, record, &mut NeverFires);
            if combat.shots().len() > before {
                fired.push(tick);
            }
        }
        fired
    }

    #[test]
    fn the_gun_leads_its_target_from_the_stellar_at_rest() {
        let target = moving_pirate(300.0, 0.0, Vec2::new(0.0, -4.0));
        let shot = one_shot(turret(), &[target], &mut NeverFires);
        let spec = WeaponSpec::new(&turret());
        let lead = trig::lead_angle(
            Vec2::ZERO,
            Vec2::ZERO,
            (target.position, target.velocity),
            Some(LeadShot {
                speed: spec.speed,
                rocket: false,
            }),
        );
        assert_ne!(lead, 90, "leads off the bearing");
        assert_eq!(shot.heading, f32::from(lead));
        assert_eq!(shot.velocity, trig::accel(lead, 10.0), "no firer velocity");
        assert_eq!(shot.position, Vec2::ZERO, "from the stellar");
    }

    #[test]
    fn a_rocket_gun_leads_with_the_rockets_timing_and_a_homing_gun_leads_too() {
        let target = moving_pirate(300.0, 0.0, Vec2::new(0.0, -4.0));
        let lead = |rocket| {
            f32::from(trig::lead_angle(
                Vec2::ZERO,
                Vec2::ZERO,
                (target.position, target.velocity),
                Some(LeadShot {
                    speed: 10.0,
                    rocket,
                }),
            ))
        };
        assert_ne!(lead(true), lead(false));
        let rocket = WeaponRecord {
            guidance: 6,
            ..turret()
        };
        assert_eq!(
            one_shot(rocket, &[target], &mut NeverFires).heading,
            lead(true)
        );
        let homing = WeaponRecord {
            guidance: 1,
            ..turret()
        };
        assert_eq!(
            one_shot(homing, &[target], &mut NeverFires).heading,
            lead(false)
        );
    }

    #[test]
    fn the_inaccuracy_turns_the_lead_and_the_heading_is_wrapped_once() {
        let target = pirate(1, 17.0, -400.0);
        let wild = WeaponRecord {
            inaccuracy: 10,
            ..turret()
        };
        let mut steady = Draws::of(&[]);
        assert_eq!(
            one_shot(turret(), &[target], &mut steady).heading,
            2.0,
            "a lead of 2"
        );
        assert!(steady.asked.is_empty(), "no draw for inaccuracy 0");
        let mut chance = Draws::of(&[]).keyed(20, 5);
        let shot = one_shot(wild, &[target], &mut chance);
        assert_eq!(shot.heading, 357.0, "2 + 5 - 10, wrapped");
        assert_eq!(shot.velocity, trig::accel(357, 10.0));
        assert_eq!(chance.times_asked(20), 1);
    }

    #[test]
    fn the_shot_is_launched_by_no_ship_at_its_target() {
        let shot = one_shot(turret(), &[pirate(1, 100.0, 0.0)], &mut NeverFires);
        assert_eq!(shot.source, Source::Other);
        assert_eq!((shot.target, shot.aimed_at), (Some(NPC), Some(NPC)));
        assert_eq!((shot.firer, shot.fleet), (NPC, NPC));
        assert_eq!((shot.firer_slot, shot.generation), (None, 0));
        assert_eq!(shot.attacker(), None);
        assert_eq!(shot.jam, Jam::Clear);
        assert_eq!(shot.age, 0);
        assert_eq!(shot.moved_from, None);
    }

    #[test]
    fn a_homing_gun_shot_is_put_in_the_shot_layer() {
        let homing = WeaponRecord {
            guidance: 1,
            ..turret()
        };
        assert_eq!(
            ShotLayer::spawned(Guidance::Homing, true, false),
            ShotLayer::Homing
        );
        let shot = one_shot(homing, &[pirate(1, 100.0, 0.0)], &mut NeverFires);
        assert_eq!(shot.layer, ShotLayer::Shots);
    }

    #[test]
    fn the_shots_disable_only_and_durability_are_its_weapons() {
        let tough = WeaponRecord {
            durability: 40,
            flags2: DISABLE_ONLY,
            ..turret()
        };
        let shot = one_shot(tough, &[pirate(1, 100.0, 0.0)], &mut NeverFires);
        assert!(shot.disable_only);
        assert_eq!(shot.durability, 40.0);
        let shot = one_shot(turret(), &[pirate(1, 100.0, 0.0)], &mut NeverFires);
        assert!(!shot.disable_only);
    }

    #[test]
    fn the_shot_rolls_its_jamming_and_no_interference() {
        let torpedo = WeaponRecord {
            guidance: 1,
            seeker: 0x0008,
            jam_vuln: [0, 0, 0, 90],
            ..turret()
        };
        let mut combat = Combat::default();
        combat.field.interference = 100;
        let mut chance = Draws::of(&[]).keyed(91, 37);
        fire(
            &mut combat,
            &mut [gun()],
            &[pirate(1, 100.0, 0.0)],
            torpedo,
            &mut chance,
        );
        let shot = combat.shots()[0];
        assert_eq!(shot.jam_rolls, [0, 0, 0, 37]);
        assert_eq!(shot.jam, Jam::Clear, "never interfered");
        assert_eq!(chance.asked, [91], "no interference roll");
    }

    #[test]
    fn the_shot_starts_its_spin_timer_at_0_whatever_its_slot_left() {
        for reading in [RuleSource::Engine, RuleSource::Bible] {
            let mut combat = Combat::default();
            combat.set_reused_shot_slot_spin(reading);
            combat.slots.spin_pass(0, 9);
            combat.slots.spin_pass(0, 9);
            fire(
                &mut combat,
                &mut [gun()],
                &[pirate(1, 100.0, 0.0)],
                turret(),
                &mut NeverFires,
            );
            assert_eq!(combat.shots()[0].spin_timer, 0, "{reading:?}");
            assert_eq!(combat.slots.spin(0), 0, "{reading:?}: the slot's too");
        }
    }

    #[test]
    fn the_shot_carries_its_slots_leftover_miss_roll_by_the_engine_alone() {
        for (reading, roll) in [(RuleSource::Engine, Some(3)), (RuleSource::Bible, None)] {
            let mut combat = Combat::default();
            combat.set_reused_shot_slot_roll(reading);
            combat.slots.write_roll(0, Some(3));
            fire(
                &mut combat,
                &mut [gun()],
                &[pirate(1, 100.0, 0.0)],
                turret(),
                &mut NeverFires,
            );
            assert_eq!(combat.shots()[0].miss_roll, roll, "{reading:?}");
            assert_eq!(combat.slots.roll(0), Some(3), "{reading:?}: none written");
        }
    }

    #[test]
    fn a_ready_gun_fires_then_waits_its_reload_and_a_tick() {
        assert_eq!(
            firing_ticks(turret(), GunTimer::default(), 10),
            [0, 4, 8],
            "Reload 3"
        );
    }

    #[test]
    fn the_reload_counts_down_with_no_threat_near() {
        let mut combat = Combat::default();
        let mut guns = [StellarGun {
            timer: GunTimer {
                reload: 3.0,
                burst: 0,
            },
            ..gun()
        }];
        for _ in 0..3 {
            fire(&mut combat, &mut guns, &[], turret(), &mut NeverFires);
        }
        assert_eq!(guns[0].timer.reload, 0.0);
        fire(
            &mut combat,
            &mut guns,
            &[pirate(1, 100.0, 0.0)],
            turret(),
            &mut NeverFires,
        );
        assert_eq!(combat.shots().len(), 1, "fires at once");
    }

    #[test]
    fn a_ready_gun_with_no_threat_stays_ready_and_fires_on_the_first_tick_one_appears() {
        let mut combat = Combat::default();
        let mut guns = [gun()];
        for _ in 0..5 {
            fire(&mut combat, &mut guns, &[], turret(), &mut NeverFires);
        }
        assert_eq!(guns[0].timer, GunTimer::default());
        assert!(combat.take_events().is_empty());
        fire(
            &mut combat,
            &mut guns,
            &[pirate(1, 100.0, 0.0)],
            turret(),
            &mut NeverFires,
        );
        assert_eq!(combat.shots().len(), 1);
        assert_eq!(guns[0].timer.reload, 3.0);
    }

    #[test]
    fn a_negative_reload_fires_every_tick() {
        let frantic = WeaponRecord {
            reload: -5,
            ..turret()
        };
        assert_eq!(firing_ticks(frantic, GunTimer::default(), 4), [0, 1, 2, 3]);
    }

    #[test]
    fn a_bursting_gun_fires_its_burst_then_waits_its_burst_reload() {
        let burst = WeaponRecord {
            reload: 2,
            burst_count: 3,
            burst_reload: 10,
            ..turret()
        };
        assert_eq!(
            firing_ticks(burst, GunTimer::default(), 25),
            [0, 3, 6, 17, 20, 23]
        );
        let mut combat = Combat::default();
        let mut guns = [gun()];
        for _ in 0..7 {
            fire(
                &mut combat,
                &mut guns,
                &[pirate(1, 100.0, 0.0)],
                burst,
                &mut NeverFires,
            );
        }
        assert_eq!(
            guns[0].timer,
            GunTimer {
                reload: 10.0,
                burst: 0
            },
            "the count back to 0"
        );
    }

    #[test]
    fn with_no_free_slot_under_the_caps_the_gun_holds_its_fire_and_its_timer() {
        let mut combat = Combat::default();
        combat.set_combat_caps(RuleSource::Engine);
        let filler = one_shot(turret(), &[pirate(1, 100.0, 0.0)], &mut NeverFires);
        combat.shots = (0..MAX_SHOTS).map(|slot| Shot { slot, ..filler }).collect();
        let bursting = WeaponRecord {
            burst_count: 3,
            ..turret()
        };
        let timer = GunTimer {
            reload: 0.0,
            burst: 1,
        };
        let mut guns = [StellarGun { timer, ..gun() }];
        let marks = [pirate(1, 100.0, 0.0)];
        fire(&mut combat, &mut guns, &marks, bursting, &mut NeverFires);
        assert_eq!(combat.shots().len(), MAX_SHOTS);
        assert!(combat.take_events().is_empty());
        assert_eq!(guns[0].timer, timer);
        combat.shots.retain(|shot| shot.slot != 17);
        fire(&mut combat, &mut guns, &marks, bursting, &mut NeverFires);
        assert_eq!(combat.shots().len(), MAX_SHOTS, "fired into the freed slot");
        assert_eq!(guns[0].timer.burst, 2);
    }

    #[test]
    fn each_shot_raises_one_stellar_fired_event_from_the_stellar() {
        let mut combat = Combat::default();
        let mut guns = [StellarGun {
            position: Vec2::new(40.0, -20.0),
            ..gun()
        }];
        let marks = [pirate(1, 100.0, 0.0)];
        fire(&mut combat, &mut guns, &marks, turret(), &mut NeverFires);
        assert_eq!(
            combat.take_events(),
            [CombatEvent::StellarFired {
                stellar: StellarId(128),
                weapon: WeaponId(196),
                at: Vec2::new(40.0, -20.0),
                sound: FireSound {
                    priority: 5,
                    unless_playing: false,
                },
            }]
        );
        fire(&mut combat, &mut guns, &marks, turret(), &mut NeverFires);
        assert!(combat.take_events().is_empty(), "reloading");
        assert_eq!(STELLAR_GUN_PRIORITY, 5);
    }

    #[test]
    fn a_gun_whose_weapon_is_missing_does_nothing() {
        let mut combat = Combat::default();
        let mut guns = [StellarGun {
            weapon: WeaponId(999),
            ..gun()
        }];
        fire(
            &mut combat,
            &mut guns,
            &[pirate(1, 100.0, 0.0)],
            turret(),
            &mut NeverFires,
        );
        assert!(combat.shots().is_empty());
        assert!(combat.take_events().is_empty());
    }

    #[test]
    fn each_gun_fires_in_turn_with_its_own_timer() {
        let mut combat = Combat::default();
        let mut guns = [
            gun(),
            StellarGun {
                stellar: StellarId(129),
                timer: GunTimer {
                    reload: 1.0,
                    burst: 0,
                },
                ..gun()
            },
        ];
        let marks = [pirate(1, 100.0, 0.0)];
        fire(&mut combat, &mut guns, &marks, turret(), &mut NeverFires);
        fire(&mut combat, &mut guns, &marks, turret(), &mut NeverFires);
        let ids: Vec<ShotId> = combat.shots().iter().map(|shot| shot.id).collect();
        assert_eq!(ids, [ShotId(0), ShotId(1)]);
        assert_eq!(combat.shots()[1].slot, 1);
    }
}
