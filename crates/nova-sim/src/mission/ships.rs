//! An active mission's special ships: where they are ([`ShipSystem`],
//! its `ShipSyst` resolved at accept by [`select_ship_system`]) and the
//! state its slot keeps of them ([`SpecialShips`]). How they spawn is the
//! [`traffic::spawn`](crate::traffic::spawn) module's "Mission ships".

use crate::catalog::{CommCatalog, DudeId, GovtId, MissionShip, ShipId, SystemId};
use crate::chance::Chance;
use crate::gate::GateKind;
use crate::govt::{
    ALWAYS_FOLLOWS_THROUGH_HYPERGATE, ALWAYS_FOLLOWS_THROUGH_WORMHOLE, Governments,
    NEVER_FOLLOWS_THROUGH_HYPERGATE,
};
use crate::hyperspace::{Places, StarMap};

use super::goals::ShipGoal;
use super::select::GovtCode;

/// Where an active mission's special ships are: its `ShipSyst` as
/// `_LoadCurrentMissionData` resolves it into the slot (+0x10) at accept
/// (@0xa0f67-0xa1142).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ShipSystem {
    /// In this system.
    At(SystemId),
    /// -6: in whatever system the player is in.
    FollowsPlayer,
    /// Nowhere: no system was found, or the code names none.
    #[default]
    Nowhere,
}

impl ShipSystem {
    /// Whether the ships are with the player in `player`'s system: the
    /// system standing for theirs at its place among `places` is the
    /// player's, as the original resolves +0x10 through
    /// `_FindActiveCoLocatedSystem` (@0x42e5c, @0x43776), or they follow
    /// the player.
    #[must_use]
    pub fn with_player(self, player: SystemId, places: &Places) -> bool {
        match self {
            Self::At(at) => places.stand_in(at) == Some(player),
            Self::FollowsPlayer => true,
            Self::Nowhere => false,
        }
    }
}

/// What an active mission's slot keeps of its special ships: by default
/// none, nowhere, with no timer running.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpecialShips {
    /// Where they are (+0x10).
    pub system: ShipSystem,
    /// How many are left to spawn or still alive (+0x6): its `ShipCount`
    /// at accept, one fewer for each destroyed, and one fewer for each
    /// that leaves the system as the ships-left rule says
    /// ([`RuleKey::LeftRespawn`](crate::RuleKey::LeftRespawn)).
    pub left: i16,
    /// How many of those that come in later have come since the timer
    /// last restarted (+0x2e).
    pub arrived: i16,
    /// The appear timer those that come in later wait on (+0x4e).
    pub appear: i16,
    /// Their name, drawn at accept (+0x70): what `<SN>` reads, and what
    /// they are called while it is not empty.
    pub name: String,
    /// Their subtitle, drawn at accept (+0xb0), shown while it is not
    /// empty.
    pub subtitle: String,
    /// How many have been destroyed (+0x28), whoever destroyed them.
    pub destroyed: i16,
    /// How many have been boarded on the paths that count a board
    /// (+0x2a).
    pub boarded: i16,
    /// How many have been disabled (+0x2c).
    pub disabled: i16,
    /// How many of a chase-off goal's have left the system (+0x30).
    pub chased_off: i16,
    /// The one ship type they all fly under [`UNIFORM_SHIP_TYPE`]
    /// (+0x56): drawn from their `düde` at accept, or, for a slot that
    /// has none (a save from before it was kept, or a `düde` that could
    /// not be read then), at their next spawn, and a swapped person's
    /// type ([`swapped_ship_type`]). None without the flag. The original
    /// keeps the `düde`'s field index, the engine the type: the two
    /// differ only if the `düde` changed between accept and spawn, which
    /// the engine does not model.
    pub uniform_type: Option<ShipId>,
}

impl Default for SpecialShips {
    fn default() -> Self {
        Self {
            system: ShipSystem::Nowhere,
            left: 0,
            arrived: 0,
            appear: NOT_APPEARING,
            name: String::new(),
            subtitle: String::new(),
            destroyed: 0,
            boarded: 0,
            disabled: 0,
            chased_off: 0,
            uniform_type: None,
        }
    }
}

/// A mission's `ShipStart`, read: where and when its ships appear.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShipStart {
    /// -1 to -16: at the stellar of nav slot `-ShipStart - 1` as the
    /// system is set up, or at random when that slot is empty
    /// (@0x4300a-0x43091).
    Nav(u8),
    /// 0, or a code below -16 that no `NavDefs` slot answers: placed at
    /// random.
    Random,
    /// 1: they come in later, on the appear timer, as many as have not
    /// arrived since it last restarted.
    ComesLater,
    /// 2: they first come in on the appear timer, then are set up
    /// cloaked (`_DoShipCloak` @0x43114-0x43121).
    Cloaked,
    /// Any code above 2, which no stock mission uses: on the appear timer
    /// as 2, never cloaked.
    Timed,
}

impl ShipStart {
    /// The `ShipStart` `code` names.
    #[must_use]
    pub const fn from_code(code: i16) -> Self {
        match code {
            // -1 to -16: 0 to 15, which fits.
            -16..=-1 => Self::Nav((-code - 1) as u8),
            1 => Self::ComesLater,
            2 => Self::Cloaked,
            3.. => Self::Timed,
            _ => Self::Random,
        }
    }

    /// Whether the appear timer runs from accept: any `ShipStart` above 0
    /// (@0xa0ed4-0xa0f09).
    #[must_use]
    pub const fn timed(self) -> bool {
        matches!(self, Self::ComesLater | Self::Cloaked | Self::Timed)
    }
}

/// A mission's `ShipBehav`, read: how its ships behave.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShipBehav {
    /// -1, or any code it does not read: as their `düde`'s AI decides.
    Own,
    /// 0: they attack the player (`_AIMakeShipAttackPlayer`
    /// @0x3d099-0x3d0bb, `_HighLevelAIHandler` @0x8d628-0x8d6ea).
    AttackPlayer,
    /// 1: they escort the player. Each joins the player's formation as a
    /// mission escort while it is intact (`_AIDispatch` @0x8fc03-0x8fc22
    /// gives it AI type 6 and leader 0 on every think) and flies as the
    /// player's escorts do, never one of the hired escorts; it leaves as
    /// its mission ends (see the session's `mission_ships` module).
    EscortPlayer,
    /// 2: they attack stellars (`_DeathStarAI`). Until
    /// rdm:phase/mission-ships/phase-12-stellar-attackers, as
    /// [`Self::Own`].
    AttackStellars,
}

impl ShipBehav {
    /// The `ShipBehav` `code` names.
    #[must_use]
    pub const fn from_code(code: i16) -> Self {
        match code {
            0 => Self::AttackPlayer,
            1 => Self::EscortPlayer,
            2 => Self::AttackStellars,
            _ => Self::Own,
        }
    }
}

/// The appear timer of escorts the player is to escort (30 @0xa0eec).
pub const ESCORT_APPEAR: i16 = 30;
/// `Rand(100) + 100`: any other appear timer.
pub const APPEAR_DRAW: u32 = 100;
/// The appear timer's value when it does not run (-1 @0xa0f09).
pub const NOT_APPEARING: i16 = -1;

/// The appear timer `ships` starts, at accept and at each restart:
/// [`ESCORT_APPEAR`] for ships that escort the player with `ShipGoal`
/// 3, else `Rand(100) + 100`, drawn (@0xa0ede-0xa0f03, @0x99f5b-0x99f9a).
pub fn appear_timer(ships: &MissionShip, chance: &mut dyn Chance) -> i16 {
    if ShipBehav::from_code(ships.behav) == ShipBehav::EscortPlayer
        && ShipGoal::from_code(ships.goal) == ShipGoal::Escort
    {
        ESCORT_APPEAR
    } else {
        i16::try_from(chance.below(APPEAR_DRAW) + APPEAR_DRAW).unwrap_or(i16::MAX)
    }
}

/// The appear timer of a slot whose ships a gate exit loses (0x7fff
/// @0x671bd): above the 32000 the timer counts down from, and not below
/// 0 as setup asks, so they come no more until the enter-system hook
/// restarts it.
pub const LOST_AT_GATE: i16 = 0x7fff;
/// `Rand(2)`: the odds of following through a hypergate (@0x67225).
const HYPERGATE_ODDS: u32 = 2;
/// `Rand(4)`: the odds of following through a wormhole (@0x67272).
const WORMHOLE_ODDS: u32 = 4;

/// What a gate exit sees of one active slot's ships.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GateSlot {
    /// Where they are (+0x10).
    pub system: ShipSystem,
    /// How they behave (+0xc).
    pub behav: ShipBehav,
    /// How many are left (+0x6).
    pub left: i16,
    /// Their `düde`'s government's `Flags2`: none for a mission of no
    /// `düde`, a `düde` that cannot be read, or an independent `düde` (of
    /// no government). A government that cannot be read has `Flags2` 0.
    /// [`through_gate`] reads none as never following.
    pub flags2: Option<u16>,
}

/// A slot's appear timer and arrived count as a gate exit sets them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GateTimer {
    /// The appear timer (+0x4e).
    pub appear: i16,
    /// The arrived count (+0x2e).
    pub arrived: i16,
}

/// What the player's exit from a gate of `kind` makes of `slot`, drawing
/// on `chance` (`_HandlePlayerDockRequest`'s loop A, @0x6716a-0x67294):
/// none, drawing nothing, unless its ships follow the player
/// ([`ShipSystem::FollowsPlayer`]) with some left. Then:
///
/// - Escorts of the player ([`ShipBehav::EscortPlayer`]): the timer stops
///   and none have arrived, so the exit's setup sets them up at once.
/// - Any other behaviour: lost ([`LOST_AT_GATE`], all arrived).
/// - Attackers of the player ([`ShipBehav::AttackPlayer`]) of a
///   government may follow instead: through a hypergate never with
///   [`NEVER_FOLLOWS_THROUGH_HYPERGATE`], always with
///   [`ALWAYS_FOLLOWS_THROUGH_HYPERGATE`], else on `Rand(2)` = 0;
///   through a wormhole always with [`ALWAYS_FOLLOWS_THROUGH_WORMHOLE`],
///   else on `Rand(4)` = 0. Followers' timer is 0 with none arrived, so
///   they all come in on the next frame.
pub fn through_gate(kind: GateKind, slot: &GateSlot, chance: &mut dyn Chance) -> Option<GateTimer> {
    if slot.system != ShipSystem::FollowsPlayer || slot.left <= 0 {
        return None;
    }
    if slot.behav == ShipBehav::EscortPlayer {
        return Some(GateTimer {
            appear: NOT_APPEARING,
            arrived: 0,
        });
    }
    let follows = slot.behav == ShipBehav::AttackPlayer
        && slot
            .flags2
            .is_some_and(|flags2| follows_through(kind, flags2, chance));
    Some(if follows {
        GateTimer {
            appear: 0,
            arrived: 0,
        }
    } else {
        GateTimer {
            appear: LOST_AT_GATE,
            arrived: slot.left,
        }
    })
}

/// Whether attackers of a government of `flags2` follow the player
/// through a gate of `kind`, drawing on `chance` (@0x671fb-0x67281).
fn follows_through(kind: GateKind, flags2: u16, chance: &mut dyn Chance) -> bool {
    match kind {
        GateKind::Hypergate if flags2 & NEVER_FOLLOWS_THROUGH_HYPERGATE != 0 => false,
        GateKind::Hypergate if flags2 & ALWAYS_FOLLOWS_THROUGH_HYPERGATE != 0 => true,
        GateKind::Hypergate => chance.below(HYPERGATE_ODDS) == 0,
        GateKind::Wormhole if flags2 & ALWAYS_FOLLOWS_THROUGH_WORMHOLE != 0 => true,
        GateKind::Wormhole => chance.below(WORMHOLE_ODDS) == 0,
    }
}

/// `Flags`: the mission's ships all fly one ship type
/// (`_MissionDudeSpawn` @0x3cda7; the type kept for later setups is
/// rdm:phase/mission-ships-fidelity/phase-1-uniform-ship-type). A person
/// swapped for the mission's ship hands it its own type through it.
pub const UNIFORM_SHIP_TYPE: u16 = 0x0800;

/// The ship type a person flying `person_ship`, swapped for the ship of a
/// mission of `Flags` `flags` whose `düde` lists `dude_ships`, gives that
/// ship (`_MissionDudeSpawn` @0x3cd90-0x3cdd3): its own type under
/// [`UNIFORM_SHIP_TYPE`] when the `düde` lists it; else none, and the
/// type is drawn from the `düde` as for any mission ship. The swap's one
/// ship left is the other half of the original's test (@0x3cdad).
#[must_use]
pub fn swapped_ship_type(flags: u16, dude_ships: &[ShipId], person_ship: ShipId) -> Option<ShipId> {
    (flags & UNIFORM_SHIP_TYPE != 0 && dude_ships.contains(&person_ship)).then_some(person_ship)
}

/// String `Rand(count) + 1` of `STR#` `list` in `strings`, drawn; empty,
/// drawing nothing, with no list or an empty one (@0xa1195-0xa1273).
fn drawn_string(list: Option<i16>, strings: &dyn CommCatalog, chance: &mut dyn Chance) -> String {
    let Some(list) = list else {
        return String::new();
    };
    let strings = strings.string_list(list);
    if strings.is_empty() {
        return String::new();
    }
    let n = u32::try_from(strings.len()).unwrap_or(u32::MAX);
    let at = usize::try_from(chance.below(n)).unwrap_or(0);
    strings.into_iter().nth(at).unwrap_or_default()
}

impl SpecialShips {
    /// The ships of a mission whose `mïsn` gives `ships`, as
    /// `_LoadCurrentMissionData` loads them at accept (@0xa0a38), drawing
    /// on `chance` in its order: the appear timer, for a timed `ShipStart`
    /// ([`ShipStart::timed`], see [`appear_timer`]), else none
    /// (@0xa0ed4-0xa0f09); their
    /// system ([`select_ship_system`] from `facts` over `map` and
    /// `govts`); their name from their `ShipNameID` and their subtitle
    /// from their `ShipSubtitle`, each string `Rand(count) + 1` of its
    /// `STR#` in `strings`, or empty (@0xa1195-0xa1358). Ships left are
    /// their `ShipCount`, and they have all arrived but for a `ShipStart`
    /// of 1 (@0xa0b39-0xa0b46, @0xa1468-0xa147d). None have been
    /// destroyed, boarded, disabled or chased off (@0xa1c35-0xa1c59).
    /// For a mission of `Flags` `flags` with [`UNIFORM_SHIP_TYPE`] and a
    /// `ShipDude`, the one type they all fly is `draw_type` of that
    /// `düde` (@0xa0b9f-0xa0bfe), else none.
    pub fn at_accept(
        ships: &MissionShip,
        flags: u16,
        facts: &ShipFacts,
        (map, govts): (&StarMap, &Governments),
        strings: &dyn CommCatalog,
        draw_type: &mut dyn FnMut(DudeId, &mut dyn Chance) -> Option<ShipId>,
        chance: &mut dyn Chance,
    ) -> Self {
        let uniform_type = ships
            .dude
            .filter(|_| flags & UNIFORM_SHIP_TYPE != 0)
            .and_then(|dude| draw_type(dude, &mut *chance));
        let start = ShipStart::from_code(ships.start);
        let appear = if start.timed() {
            appear_timer(ships, chance)
        } else {
            NOT_APPEARING
        };
        let system = select_ship_system(ships.syst, facts, map, govts, chance);
        let name = drawn_string(ships.name_id, strings, chance);
        let subtitle = drawn_string(ships.subtitle, strings, chance);
        Self {
            system,
            left: ships.count,
            arrived: if start == ShipStart::ComesLater {
                0
            } else {
                ships.count
            },
            appear,
            name,
            subtitle,
            destroyed: 0,
            boarded: 0,
            disabled: 0,
            chased_off: 0,
            uniform_type,
        }
    }
}

/// What a `ShipSyst` code is resolved from at accept: the player's
/// system then, and the systems holding the mission's stops' stellars
/// then.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShipFacts {
    /// The player's system as the mission is accepted.
    pub player_system: SystemId,
    /// Where it goes: `None` with no destination, else the system that
    /// holds its stellar now, `Some(None)` when none does.
    pub destination: Option<Option<SystemId>>,
    /// Where it is completed: `None` with no return, else the system that
    /// holds its stellar now, `Some(None)` when none does.
    pub return_to: Option<Option<SystemId>>,
}

/// The systems a random `ShipSyst` code draws from, beside the tests
/// every one but -5 makes (see [`select_ship_system`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    /// -2: any (@0x9b9a7-0x9ba6b).
    Any,
    /// 9999-31999: of the relation the code names (see [`Kind::takes`]).
    Relation(GovtCode),
}

impl Kind {
    /// Whether a system governed by `govt` is of this kind, the relations
    /// as `govts` say.
    fn takes(self, govt: Option<GovtId>, govts: &Governments) -> bool {
        let Self::Relation(code) = self else {
            return true;
        };
        match code {
            // Governed by this government, an independent for 9999
            // (@0x9bae1-0x9bbcf).
            GovtCode::Govt(of) => govt == of,
            // Governed, by another government allied with this one
            // (@0x9bbd4-0x9bd08).
            GovtCode::Ally(of) => {
                govt.is_some_and(|govt| govt != of) && govts.allies(govt, Some(of))
            }
            // Not governed by this government, an independent included
            // (@0x9bd0d-0x9bdfb).
            GovtCode::NotGovt(of) => govt != Some(of),
            // Governed by an enemy of this government (@0x9be00-0x9bf02).
            GovtCode::Enemy(of) => govts.enemies(govt, Some(of)),
            // Governed by a government sharing a class with this one, its
            // own included (@0x9bf07-0x9c009).
            GovtCode::SharedClass(of) => govts.shared_class(govt, Some(of)),
            // Governed by none sharing a class with this one, an
            // independent included (@0x9c00e-0x9c111).
            GovtCode::NoSharedClass(of) => !govts.shared_class(govt, Some(of)),
        }
    }
}

/// The `ShipSyst` -6: the ships follow the player.
const FOLLOWS_PLAYER: i16 = -6;

/// Where `code`, a `ShipSyst`, puts a mission's ships, as
/// `_LoadCurrentMissionData` resolves it at accept (@0xa0f67-0xa1142),
/// from `facts`, over the visible systems of `map` and the relations of
/// `govts`, drawing on `chance`:
///
/// - -1: the player's system.
/// - -3: the system that holds the destination's stellar, or with no
///   destination the return's; -4: the return's. Each is the stellar's
///   owner (+0x14) read live at accept, the rescue's included, with no
///   `_FindSystemFromStellar` fallback: nowhere when no system holds it,
///   and with a destination that no system holds, -3 never falls to the
///   return (@0xa0fb3-0xa105f). The original also writes a crashlog line
///   for -3 and -4 with no stellar (@0xa0fec-0xa1010, @0xa1066-0xa106d),
///   left out as it changes no state.
/// - -6: wherever the player is.
/// - 128-9998: that system.
/// - -2 and 9999-31999: one visible system not at the player's place
///   (not together with it), of the code's kind, drawn
///   (`_SelectMissionSystem` @0x9b993); nowhere when there is none. The
///   original redraws `Rand(2048)` until it hits one, which comes to one
///   draw over them by ascending ID; where it would draw forever, this is
///   nowhere.
/// - -5: one of the systems the player's system links to, drawn
///   (@0x9ba70-0x9bac7); nowhere with none.
/// - 32000 and above: the player's system, as `_SelectMissionSystem`
///   gives back its second argument for a code it does not read
///   (@0x9c012).
/// - Any other code: nowhere.
pub fn select_ship_system(
    code: i16,
    facts: &ShipFacts,
    map: &StarMap,
    govts: &Governments,
    chance: &mut dyn Chance,
) -> ShipSystem {
    let at =
        |held: Option<Option<SystemId>>| held.flatten().map_or(ShipSystem::Nowhere, ShipSystem::At);
    let kind = match code {
        -1 | 32_000.. => return ShipSystem::At(facts.player_system),
        -3 => return at(facts.destination.or(facts.return_to)),
        -4 => return at(facts.return_to),
        FOLLOWS_PLAYER => return ShipSystem::FollowsPlayer,
        -5 => return drawn(map.listed_links(facts.player_system), chance),
        128..=9998 => return ShipSystem::At(SystemId(code)),
        -2 => Kind::Any,
        _ => match GovtCode::of(code) {
            Some(code) => Kind::Relation(code),
            None => return ShipSystem::Nowhere,
        },
    };
    let places = map.places();
    let candidates: Vec<SystemId> = map
        .systems()
        .filter(|&system| {
            !places.together(system, facts.player_system) && kind.takes(map.govt(system), govts)
        })
        .collect();
    drawn(&candidates, chance)
}

/// One of `systems`, drawn once over them all; nowhere, drawing nothing,
/// when there is none.
fn drawn(systems: &[SystemId], chance: &mut dyn Chance) -> ShipSystem {
    if systems.is_empty() {
        return ShipSystem::Nowhere;
    }
    let n = u32::try_from(systems.len()).unwrap_or(u32::MAX);
    let at = usize::try_from(chance.below(n)).unwrap_or(usize::MAX);
    systems
        .get(at)
        .copied()
        .map_or(ShipSystem::Nowhere, ShipSystem::At)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{GovtRecord, StarSystem};
    use crate::hyperspace::CoLocated;
    use crate::testkit::{Draws, govt};

    #[test]
    fn with_flags_0x0800_a_swapped_ship_flies_the_persons_type_when_the_dude_lists_it() {
        let dude = [ShipId(137), ShipId(136)];
        assert_eq!(UNIFORM_SHIP_TYPE, 0x0800);
        assert_eq!(
            swapped_ship_type(0x0900, &dude, ShipId(136)),
            Some(ShipId(136)),
            "mission 132's Terrapin"
        );
        assert_eq!(
            swapped_ship_type(0x0800, &dude, ShipId(137)),
            Some(ShipId(137))
        );
        assert_eq!(
            swapped_ship_type(0x0800, &dude, ShipId(135)),
            None,
            "not listed"
        );
        assert_eq!(
            swapped_ship_type(0x07ff, &dude, ShipId(136)),
            None,
            "drawn without 0x0800"
        );
    }

    const FED: GovtId = GovtId(128);
    const CIVVIES: GovtId = GovtId(129);
    const PIRATES: GovtId = GovtId(130);
    const NEUTRAL: GovtId = GovtId(131);

    /// The Federation (class 1, allied with 13, at war with 9), the
    /// Civvies (classes 1 and 13), the pirates (class 9) and a
    /// neutral (class 4).
    fn govts() -> Governments {
        Governments::new([
            GovtRecord {
                classes: [1, -1, -1, -1],
                allies: [13, -1, -1, -1],
                enemies: [9, -1, -1, -1],
                ..govt(128)
            },
            GovtRecord {
                classes: [1, 13, -1, -1],
                ..govt(129)
            },
            GovtRecord {
                classes: [9, -1, -1, -1],
                ..govt(130)
            },
            GovtRecord {
                classes: [4, -1, -1, -1],
                ..govt(131)
            },
        ])
    }

    fn system(id: i16, at: (f32, f32), links: &[i16], govt: Option<GovtId>) -> StarSystem {
        StarSystem {
            govt,
            ..crate::testkit::star(id, at, links)
        }
    }

    /// The player is in 130 (Federation), linked to 131 and 133; 131 is
    /// the Civvies', 132 the pirates', 133 independent, 134 the
    /// neutral's, 135 the Federation's but at 130's place, 136 the
    /// Federation's elsewhere, and 137 the pirates' but hidden.
    fn map() -> StarMap {
        map_resolving(CoLocated::Lowest)
    }

    /// [`map`], a place two visible systems share resolved as `colocated`
    /// says.
    fn map_resolving(colocated: CoLocated) -> StarMap {
        StarMap::with_hidden(
            vec![
                system(130, (0.0, 0.0), &[131, 133], Some(FED)),
                system(131, (100.0, 0.0), &[], Some(CIVVIES)),
                system(132, (200.0, 0.0), &[], Some(PIRATES)),
                system(133, (300.0, 0.0), &[], None),
                system(134, (400.0, 0.0), &[], Some(NEUTRAL)),
                system(135, (0.0, 0.0), &[], Some(FED)),
                system(136, (500.0, 0.0), &[], Some(FED)),
                system(137, (600.0, 0.0), &[], Some(PIRATES)),
            ],
            &[SystemId(137)].into(),
            colocated,
        )
    }

    fn facts() -> ShipFacts {
        ShipFacts {
            player_system: SystemId(130),
            destination: Some(Some(SystemId(132))),
            return_to: Some(Some(SystemId(134))),
        }
    }

    /// What `code` resolves to from [`facts`], drawing `draws`, and the
    /// bounds asked.
    fn resolved_with(code: i16, facts: &ShipFacts, draws: &[u32]) -> (ShipSystem, Vec<u32>) {
        let mut chance = Draws::of(draws);
        let resolved = select_ship_system(code, facts, &map(), &govts(), &mut chance);
        (resolved, chance.asked)
    }

    fn resolved(code: i16, draws: &[u32]) -> (ShipSystem, Vec<u32>) {
        resolved_with(code, &facts(), draws)
    }

    fn at(id: i16) -> ShipSystem {
        ShipSystem::At(SystemId(id))
    }

    #[test]
    fn the_fixed_codes_draw_nothing() {
        let none: Vec<u32> = Vec::new();
        assert_eq!(resolved(-1, &[]), (at(130), none.clone()), "the player's");
        assert_eq!(
            resolved(-3, &[]),
            (at(132), none.clone()),
            "the destination's"
        );
        assert_eq!(resolved(-4, &[]), (at(134), none.clone()), "the return's");
        assert_eq!(resolved(-6, &[]), (ShipSystem::FollowsPlayer, none.clone()));
        assert_eq!(resolved(128, &[]), (at(128), none.clone()));
        assert_eq!(resolved(9998, &[]), (at(9998), none.clone()), "unbounded");
        for code in [32_000, i16::MAX] {
            assert_eq!(resolved(code, &[]), (at(130), none.clone()), "{code}");
        }
        for code in [0, 1, 127, -7, i16::MIN] {
            assert_eq!(
                resolved(code, &[]),
                (ShipSystem::Nowhere, none.clone()),
                "{code}"
            );
        }
    }

    #[test]
    fn minus_3_takes_the_return_with_no_destination_and_both_are_nowhere_without_a_holder() {
        let returning = ShipFacts {
            destination: None,
            ..facts()
        };
        assert_eq!(resolved_with(-3, &returning, &[]).0, at(134));
        let nowhere = ShipFacts {
            destination: None,
            return_to: None,
            ..facts()
        };
        assert_eq!(resolved_with(-3, &nowhere, &[]).0, ShipSystem::Nowhere);
        assert_eq!(resolved_with(-4, &nowhere, &[]).0, ShipSystem::Nowhere);
        let unheld = ShipFacts {
            destination: Some(None),
            return_to: Some(None),
            ..facts()
        };
        assert_eq!(resolved_with(-3, &unheld, &[]).0, ShipSystem::Nowhere);
        assert_eq!(
            resolved_with(-4, &unheld, &[]).0,
            ShipSystem::Nowhere,
            "an owner of -1 is nowhere, with no _FindSystemFromStellar fallback (@0xa1031-0xa105f)"
        );
    }

    #[test]
    fn minus_3_is_nowhere_when_its_destination_has_no_holder_though_a_return_has_one() {
        let facts = ShipFacts {
            destination: Some(None),
            ..facts()
        };
        assert_eq!(
            resolved_with(-3, &facts, &[]).0,
            ShipSystem::Nowhere,
            "a travel stellar is there, so the return is never read (@0xa0fc8-0xa0fd0)"
        );
        assert_eq!(resolved_with(-4, &facts, &[]).0, at(134));
    }

    #[test]
    fn minus_2_draws_any_visible_system_away_from_the_players_place() {
        // 131, 132, 133, 134, 136: not 130 nor 135 at its place, nor the
        // hidden 137.
        assert_eq!(resolved(-2, &[0]), (at(131), vec![5]));
        assert_eq!(resolved(-2, &[4]), (at(136), vec![5]));
    }

    #[test]
    fn minus_5_draws_a_system_the_players_system_links_to() {
        assert_eq!(resolved(-5, &[0]), (at(131), vec![2]));
        assert_eq!(resolved(-5, &[1]), (at(133), vec![2]));
        let alone = ShipFacts {
            player_system: SystemId(136),
            ..facts()
        };
        assert_eq!(
            resolved_with(-5, &alone, &[]),
            (ShipSystem::Nowhere, vec![])
        );
    }

    /// Every system `code` may draw, in order, with nothing drawn when
    /// there is none.
    fn pool(code: i16) -> Vec<i16> {
        let (first, asked) = resolved(code, &[0]);
        let Some(&n) = asked.first() else {
            assert_eq!(first, ShipSystem::Nowhere, "{code}");
            return Vec::new();
        };
        (0..n)
            .map(|draw| match resolved(code, &[draw]).0 {
                ShipSystem::At(system) => system.0,
                other => panic!("{code}: {other:?}"),
            })
            .collect()
    }

    #[test]
    fn the_government_codes_draw_over_their_kind_away_from_the_players_place() {
        assert_eq!(pool(10_000), [136], "the Federation's, not at 130's place");
        assert_eq!(pool(9999), [133], "the independents'");
        assert_eq!(pool(10_002), [132]);
        assert_eq!(
            pool(15_000),
            [131],
            "allied with the Federation, not its own"
        );
        assert_eq!(
            pool(15_001),
            [136],
            "the Federation lists the Civvies' class"
        );
        assert_eq!(
            pool(15_002),
            Vec::<i16>::new(),
            "none allied with the pirates"
        );
        assert_eq!(pool(20_000), [131, 132, 133, 134], "an independent too");
        assert_eq!(pool(25_000), [132], "its enemies");
        assert_eq!(pool(30_000), [131, 136], "sharing class 1, its own too");
        assert_eq!(
            pool(31_000),
            [132, 133, 134],
            "sharing none, an independent too"
        );
        assert_eq!(pool(20_002), [131, 133, 134, 136], "not the pirates'");
        assert_eq!(pool(25_002), [136], "the pirates' enemies");
        assert_eq!(pool(30_002), [132], "sharing the pirates' class 9");
        assert_eq!(pool(31_002), [131, 133, 134, 136], "sharing none with them");
    }

    #[test]
    fn an_independent_counts_as_an_enemy_of_a_xenophobe() {
        let govts = Governments::new([GovtRecord {
            flags: crate::govt::XENOPHOBIC,
            ..govt(128)
        }]);
        let mut chance = Draws::of(&[2]);
        let drawn = select_ship_system(25_000, &facts(), &map(), &govts, &mut chance);
        assert_eq!(
            (drawn, chance.asked),
            (at(133), vec![4]),
            "131, 132, 133 and 134"
        );
    }

    #[test]
    fn a_ship_start_reads_its_nav_slots_the_timed_starts_and_else_random() {
        assert_eq!(ShipStart::from_code(-1), ShipStart::Nav(0));
        assert_eq!(ShipStart::from_code(-16), ShipStart::Nav(15));
        for code in [0, -17, i16::MIN] {
            assert_eq!(ShipStart::from_code(code), ShipStart::Random, "{code}");
        }
        assert_eq!(ShipStart::from_code(1), ShipStart::ComesLater);
        assert_eq!(ShipStart::from_code(2), ShipStart::Cloaked);
        for code in [3, i16::MAX] {
            assert_eq!(ShipStart::from_code(code), ShipStart::Timed, "{code}");
        }
    }

    #[test]
    fn only_the_starts_above_0_run_the_appear_timer() {
        for (code, timed) in [(-1, false), (0, false), (1, true), (2, true), (3, true)] {
            assert_eq!(ShipStart::from_code(code).timed(), timed, "{code}");
        }
    }

    #[test]
    fn a_ship_behav_reads_0_to_2_and_else_the_ships_own_ai() {
        assert_eq!(ShipBehav::from_code(0), ShipBehav::AttackPlayer);
        assert_eq!(ShipBehav::from_code(1), ShipBehav::EscortPlayer);
        assert_eq!(ShipBehav::from_code(2), ShipBehav::AttackStellars);
        for code in [-1, 3, i16::MIN, i16::MAX] {
            assert_eq!(ShipBehav::from_code(code), ShipBehav::Own, "{code}");
        }
    }

    #[test]
    fn ships_are_with_the_player_in_their_system_or_any_when_following() {
        let map = map();
        let places = map.places();
        assert!(at(130).with_player(SystemId(130), places));
        assert!(!at(130).with_player(SystemId(131), places));
        assert!(
            at(135).with_player(SystemId(130), places),
            "by the original's resolution 130 stands for 135, though visible"
        );
        let itself = map_resolving(CoLocated::Itself);
        assert!(
            !at(135).with_player(SystemId(130), itself.places()),
            "135 is visible: it stands for itself, not 130 at its place"
        );
        assert!(ShipSystem::FollowsPlayer.with_player(SystemId(131), places));
        assert!(!ShipSystem::Nowhere.with_player(SystemId(130), places));
    }

    #[test]
    fn ships_in_a_hidden_system_are_with_the_player_in_the_one_standing_for_it() {
        // 138 is hidden at 130's place, 139 hidden alone.
        let map = StarMap::with_hidden(
            vec![
                system(130, (0.0, 0.0), &[], Some(FED)),
                system(138, (0.0, 0.0), &[], Some(FED)),
                system(139, (100.0, 0.0), &[], None),
            ],
            &[SystemId(138), SystemId(139)].into(),
            CoLocated::Lowest,
        );
        let places = map.places();
        assert!(at(138).with_player(SystemId(130), places));
        assert!(!at(138).with_player(SystemId(138), places));
        assert!(
            !at(139).with_player(SystemId(139), places),
            "nothing stands for it"
        );
    }

    /// `STR#` 7000 names, 7001 subtitles; 7002 is empty.
    struct Strings;

    impl CommCatalog for Strings {
        fn string_list(&self, id: i16) -> Vec<String> {
            match id {
                7000 => vec!["Ace".into(), "Deuce".into(), "Trey".into()],
                7001 => vec!["Hunter".into(), "Killer".into()],
                _ => Vec::new(),
            }
        }
    }

    /// Three ships from 7000 and 7001, in the player's system, of
    /// `ShipStart` `start`.
    fn shipping(start: i16) -> MissionShip {
        MissionShip {
            count: 3,
            goal: 0,
            syst: -2,
            start,
            name_id: Some(7000),
            subtitle: Some(7001),
            ..MissionShip::NONE
        }
    }

    fn accepted(ships: &MissionShip, draws: &[u32]) -> (SpecialShips, Vec<u32>) {
        let (accepted, asked, _) = accepted_flagged(ships, 0, draws);
        (accepted, asked)
    }

    /// The ships of a mission of `Flags` `flags`, accepted, drawing on
    /// `draws`, a ship type of 201 drawn from any `düde`: with the draws
    /// asked and the `düde`s the type was drawn from.
    fn accepted_flagged(
        ships: &MissionShip,
        flags: u16,
        draws: &[u32],
    ) -> (SpecialShips, Vec<u32>, Vec<DudeId>) {
        let mut chance = Draws::of(draws);
        let mut asked = Vec::new();
        let mut draw = |dude: DudeId, _: &mut dyn Chance| {
            asked.push(dude);
            Some(ShipId(201))
        };
        let accepted = SpecialShips::at_accept(
            ships,
            flags,
            &facts(),
            (&map(), &govts()),
            &Strings,
            &mut draw,
            &mut chance,
        );
        (accepted, chance.asked, asked)
    }

    #[test]
    fn a_0x0800_missions_ships_take_one_type_drawn_at_accept() {
        let ships = MissionShip {
            dude: Some(DudeId(150)),
            ..shipping(0)
        };
        let (accepted, _, drawn_from) = accepted_flagged(&ships, 0x0800 | 0x0001, &[1, 2, 0]);
        assert_eq!(accepted.uniform_type, Some(ShipId(201)));
        assert_eq!(drawn_from, [DudeId(150)], "once, from its düde");
    }

    #[test]
    fn a_mission_without_0x0800_draws_no_type_at_accept() {
        let ships = MissionShip {
            dude: Some(DudeId(150)),
            ..shipping(0)
        };
        let (accepted, _, drawn_from) = accepted_flagged(&ships, 0xf7ff, &[1, 2, 0]);
        assert_eq!(accepted.uniform_type, None);
        assert!(drawn_from.is_empty());
    }

    #[test]
    fn a_0x0800_mission_with_no_dude_draws_none() {
        let (accepted, _, drawn_from) = accepted_flagged(&shipping(0), 0x0800, &[1, 2, 0]);
        assert_eq!(accepted.uniform_type, None);
        assert!(drawn_from.is_empty());
    }

    #[test]
    fn at_accept_the_timer_the_system_the_name_and_the_subtitle_are_drawn_in_order() {
        for start in [1, 2] {
            let (ships, asked) = accepted(&shipping(start), &[7, 1, 2, 0]);
            assert_eq!(asked, [100, 5, 3, 2], "{start}");
            assert_eq!(
                ships,
                SpecialShips {
                    system: at(132),
                    left: 3,
                    arrived: if start == 1 { 0 } else { 3 },
                    appear: 107,
                    name: "Trey".into(),
                    subtitle: "Hunter".into(),
                    destroyed: 0,
                    boarded: 0,
                    disabled: 0,
                    chased_off: 0,
                    uniform_type: None,
                },
                "{start}"
            );
        }
    }

    #[test]
    fn a_ship_start_of_none_or_below_runs_no_timer_and_all_have_arrived() {
        for start in [0, -1, -16] {
            let (ships, asked) = accepted(&shipping(start), &[1, 2, 0]);
            assert_eq!(asked, [5, 3, 2], "{start}");
            assert_eq!(ships.appear, NOT_APPEARING);
            assert_eq!((ships.left, ships.arrived), (3, 3));
        }
    }

    /// `ShipBehav` 1: they escort the player.
    const ESCORTS_PLAYER: i16 = 1;
    /// `ShipGoal` 3: the player escorts them.
    const ESCORT_GOAL: i16 = 3;

    #[test]
    fn escorts_of_an_escort_goal_appear_after_30_drawing_nothing() {
        let escorts = MissionShip {
            behav: ESCORTS_PLAYER,
            goal: ESCORT_GOAL,
            syst: -1,
            ..shipping(1)
        };
        let (ships, asked) = accepted(&escorts, &[0, 0]);
        assert_eq!((ships.appear, asked), (30, vec![3, 2]));
        for (behav, goal) in [(ESCORTS_PLAYER, 0), (0, ESCORT_GOAL)] {
            let other = MissionShip {
                goal,
                behav,
                ..escorts
            };
            assert_eq!(accepted(&other, &[99, 0, 0]).0.appear, 199);
        }
    }

    #[test]
    fn ships_of_no_behaviour_draw_their_timer_even_with_an_escort_goal() {
        let unbehaved = MissionShip {
            count: 1,
            goal: ESCORT_GOAL,
            start: 1,
            ..MissionShip::NONE
        };
        assert_eq!(appear_timer(&unbehaved, &mut Draws::of(&[5])), 105);
    }

    #[test]
    fn with_no_list_or_an_empty_one_the_name_and_subtitle_are_empty_drawing_nothing() {
        let unnamed = MissionShip {
            syst: -1,
            name_id: None,
            subtitle: Some(7002),
            ..shipping(0)
        };
        let (ships, asked) = accepted(&unnamed, &[]);
        assert_eq!((ships.name.as_str(), ships.subtitle.as_str()), ("", ""));
        assert!(asked.is_empty());
    }

    #[test]
    fn no_ships_are_left_with_a_ship_count_of_none_or_below() {
        for count in [0, -1] {
            let none = MissionShip {
                count,
                syst: -1,
                ..shipping(0)
            };
            assert_eq!(accepted(&none, &[0, 0]).0.left, count);
        }
    }

    // A gate exit.

    /// What a gate exit of `kind` makes of a slot following the player of
    /// `behav` with 2 ships left, its `düde`'s government of `flags2`,
    /// drawing `draws`, and the bounds asked.
    fn at_exit(
        kind: GateKind,
        behav: i16,
        flags2: Option<u16>,
        draws: &[u32],
    ) -> (Option<GateTimer>, Vec<u32>) {
        let slot = GateSlot {
            system: ShipSystem::FollowsPlayer,
            behav: ShipBehav::from_code(behav),
            left: 2,
            flags2,
        };
        let mut chance = Draws::of(draws);
        let timer = through_gate(kind, &slot, &mut chance);
        (timer, chance.asked)
    }

    const LOST: Option<GateTimer> = Some(GateTimer {
        appear: LOST_AT_GATE,
        arrived: 2,
    });
    const FOLLOWS: Option<GateTimer> = Some(GateTimer {
        appear: 0,
        arrived: 0,
    });
    const BOTH: [GateKind; 2] = [GateKind::Hypergate, GateKind::Wormhole];

    #[test]
    fn a_slot_not_following_the_player_or_with_none_left_is_untouched_drawing_nothing() {
        for kind in BOTH {
            for (system, left) in [
                (ShipSystem::At(SystemId(130)), 2),
                (ShipSystem::Nowhere, 2),
                (ShipSystem::FollowsPlayer, 0),
                (ShipSystem::FollowsPlayer, -1),
            ] {
                let slot = GateSlot {
                    system,
                    behav: ShipBehav::AttackPlayer,
                    left,
                    flags2: Some(0),
                };
                let mut chance = Draws::of(&[0]);
                assert_eq!(
                    through_gate(kind, &slot, &mut chance),
                    None,
                    "{system:?} {left}"
                );
                assert!(chance.asked.is_empty());
            }
        }
    }

    #[test]
    fn escorts_of_the_player_are_set_up_at_once_drawing_nothing() {
        for kind in BOTH {
            assert_eq!(
                at_exit(kind, 1, Some(0), &[0]),
                (
                    Some(GateTimer {
                        appear: NOT_APPEARING,
                        arrived: 0
                    }),
                    vec![]
                )
            );
        }
    }

    #[test]
    fn ships_of_another_behaviour_are_lost_drawing_nothing() {
        assert_eq!(LOST_AT_GATE, 0x7fff, "above the 32000 counted down");
        for kind in BOTH {
            for behav in [-1, 2, 3] {
                assert_eq!(
                    at_exit(kind, behav, Some(0x00e0), &[0]),
                    (LOST, vec![]),
                    "{kind:?} {behav}"
                );
            }
            assert_eq!(
                at_exit(kind, 0, None, &[0]),
                (LOST, vec![]),
                "{kind:?}: an independent never follows"
            );
        }
    }

    #[test]
    fn through_a_hypergate_attackers_follow_by_their_governments_flags_or_on_rand_2() {
        let gate = GateKind::Hypergate;
        assert_eq!(at_exit(gate, 0, Some(0x0020), &[0]), (LOST, vec![]));
        assert_eq!(
            at_exit(gate, 0, Some(0x00a0), &[0]),
            (LOST, vec![]),
            "the wormhole bit is not read"
        );
        assert_eq!(at_exit(gate, 0, Some(0x0040), &[1]), (FOLLOWS, vec![]));
        assert_eq!(
            at_exit(gate, 0, Some(0x0060), &[1]),
            (LOST, vec![]),
            "0x0020 first"
        );
        assert_eq!(at_exit(gate, 0, Some(0x0080), &[0]), (FOLLOWS, vec![2]));
        assert_eq!(at_exit(gate, 0, Some(0), &[1]), (LOST, vec![2]));
    }

    #[test]
    fn through_a_wormhole_attackers_follow_by_their_governments_flag_or_on_rand_4() {
        let hole = GateKind::Wormhole;
        assert_eq!(at_exit(hole, 0, Some(0x0080), &[3]), (FOLLOWS, vec![]));
        assert_eq!(at_exit(hole, 0, Some(0x00a0), &[3]), (FOLLOWS, vec![]));
        assert_eq!(at_exit(hole, 0, Some(0), &[0]), (FOLLOWS, vec![4]));
        for draw in 1..4 {
            assert_eq!(
                at_exit(hole, 0, Some(0), &[draw]),
                (LOST, vec![4]),
                "{draw}"
            );
        }
        assert_eq!(
            at_exit(hole, 0, Some(0x0020), &[0]),
            (FOLLOWS, vec![4]),
            "0x0020 is still rolled"
        );
        assert_eq!(at_exit(hole, 0, Some(0x0040), &[1]), (LOST, vec![4]));
    }

    #[test]
    fn the_ships_state_starts_nowhere_with_none_left() {
        let ships = SpecialShips::default();
        assert_eq!(ships.system, ShipSystem::Nowhere);
        assert_eq!((ships.left, ships.arrived, ships.appear), (0, 0, -1));
        assert!(ships.name.is_empty() && ships.subtitle.is_empty());
        assert_eq!(
            (
                ships.destroyed,
                ships.boarded,
                ships.disabled,
                ships.chased_off
            ),
            (0, 0, 0, 0)
        );
    }
}
