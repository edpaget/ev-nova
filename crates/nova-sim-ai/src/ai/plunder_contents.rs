//! What a pirate's plunder takes: `_AIPlunderShipContents`
//! (@0x83fab-0x84a2d in the `EV Nova` executable), whose rules these are,
//! on plain values. Its two callers, `_PirateWarshipAI` (@0x8c7a3) and
//! `_LowLevelAIHandler`'s substate 0xf (@0x86f7f), are the traffic's
//! (`traffic::plunder`) and the session's (`nova_sim`'s `pirate_plunder`
//! module).
//!
//! - **Strengths** ([`odds`]): each side's class `Strength` (+0xc) plus the
//!   crew its marines add (`ModType` [`MARINES`] with a `ModVal` above 0,
//!   [`Marines::crew`]). An NPC's marines are its class's default items
//!   ([`ClassMarines`]); the player's are the outfits it owns
//!   ([`Marines::owned`]).
//! - **The original's item bug** ([`ClassMarines::read`]): over a class's
//!   default items it tests the item's own outfit for marines, but
//!   multiplies the count by the `ModVal` of the outfit at the item's
//!   *place*, outfit 128 + i for item slot i, in the same mod slot
//!   (`imulw 0xc(%ebx)` with %ebx stepping one outfit record per item slot:
//!   @0x8416a, @0x84251, @0x84302, @0x843d5). The player's loop doesn't
//!   have it. [`RuleKey::PlunderMarines`](crate::RuleKey::PlunderMarines)
//!   reads it so by default; its other reading takes the item's own
//!   outfit.
//! - **Odds** ([`odds`]): a victim strength of 0 or below reads as 1
//!   (@0x84277-0x84285); the ratio is trunc(100.0 × pirate / (float)(2 ×
//!   victim)) (@0x84288-0x842b6); the pirate's marines below 0
//!   ([`Marines::defence`]) come off it (raising it), the victim's are added
//!   (lowering it) (@0x842bd-0x843fa); the odds are the ratio − `Rand(21)`
//!   + 10, held to 10-100 (@0x843fc-0x8444f).
//! - **Cargo** ([`steal`], @0x8444f-0x844fb): into the pirate's free holds,
//!   its class's `Holds` less what it carries, of the victim's six
//!   commodities, no more than the victim's holds. Each pass picks one the
//!   victim holds (the original retries `Rand(6)` until it lands on one,
//!   which decides a uniform pick among them) and takes as much as fits.
//! - **Credits** ([`credits_taken`], @0x84501-0x8455c): the player loses
//!   trunc(cash × odds × 30 × 0.01 × 0.01), 3-30 %, to the pirate's
//!   +0xa0; an NPC victim's +0xa0 is zeroed, read by no one.
//! - **The line** ([`cargo_line`], @0x84593-0x84791): for the player, or a
//!   leaderless former hired or captured escort that is no mission ship,
//!   when anything was taken: "N tons of cargo and M credits stolen!".
//!   It doesn't name the pirate.
//! - **Capture** ([`captures`], [`taken`], @0x847a7-0x849cd): an NPC victim
//!   that is no mission ship joins the pirate's fleet when the odds are
//!   above [`CAPTURE_ODDS_FLOOR`] and `Rand(`[`CAPTURE_ROLL`]`)` is at most
//!   odds × [`CAPTURE_SHARE`] (or with cheats on, which the engine has
//!   none of). A former escort, or a ship the player leads, is announced
//!   ("Escort stolen!", "Fighter stolen!"); any other is taken silently.
//!
//! The lines' sound is `_beepSnd[1]`, `snd ` 151 (@0x84758, @0x84908),
//! which the session raises. Integer widths (16-bit ratio and words, a
//! 32-bit cash) are not replicated.

use crate::catalog::{MARINES, OutfitId, OutfitRecord};
use crate::chance::Chance;
use crate::rulebook::RuleSource;
use crate::self_repair::FormerKind;
use crate::traffic::npc::{NPC_COMMODITIES, NpcHold};

/// The jitter on the odds: `Rand(21)` (@0x843fc).
pub const ODDS_JITTER: u16 = 21;
/// What the odds start from before the jitter: + 10 (@0x84416).
pub const ODDS_OFFSET: i32 = 10;
/// The least odds (@0x84419-0x84426).
pub const MIN_ODDS: i32 = 10;
/// The most odds (@0x8443d-0x84445).
pub const MAX_ODDS: i32 = 100;
/// The odds a capture needs to be above (`cmpw $0x28` @0x847b8).
pub const CAPTURE_ODDS_FLOOR: i32 = 40;
/// The capture's draw: `Rand(101)` (@0x847c2).
pub const CAPTURE_ROLL: u16 = 101;
/// The share of the odds the capture's draw must be at most (the double
/// 0.5 @0xdd128).
pub const CAPTURE_SHARE: f64 = 0.5;
/// The share of its armour capacity a captured ship is left with (the
/// double 0.66 @0xdd5b8, @0x849b4-0x849c8).
pub const CAPTURED_ARMOR_SHARE: f64 = 0.66;
/// The AI wait a captured ship takes (+0x4c = 150.0 @0x8499c).
pub const CAPTURED_WAIT: u32 = 150;
/// The AI wait a pirate takes as it plunders, its hold before it drifts
/// off (+0x4c = 100.0 @0x8c772, @0x86f4e).
pub const PLUNDER_HOLD: u32 = 100;

/// The crew and defence one side's marines give (`ModType` [`MARINES`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Marines {
    /// What the marines whose `ModVal` is above 0 add to its strength:
    /// each count × `ModVal`.
    pub crew: i32,
    /// What those whose `ModVal` is below 0 give: each count × `ModVal`,
    /// so 0 or below. The pirate's raises the odds, the victim's lowers
    /// them.
    pub defence: i32,
}

impl Marines {
    /// The marines of the outfits the player owns, each mod of `mods` as
    /// (`ModType`, `ModVal`, count owned): the player's loops, which read
    /// each outfit's own `ModVal` (@0x841bd-0x84204, @0x84345-0x84388).
    #[must_use]
    pub fn owned(mods: impl IntoIterator<Item = (i16, i16, u16)>) -> Self {
        mods.into_iter()
            .filter(|&(kind, _, _)| kind == MARINES)
            .fold(Self::default(), |sum, (_, value, count)| {
                sum.plus(value, i32::from(count), value)
            })
    }

    /// `self` with `count` items whose own marines are `own`, each
    /// multiplied by `value`: a crew when `own` is above 0, a defence when
    /// below.
    fn plus(self, own: i16, count: i32, value: i16) -> Self {
        let added = count * i32::from(value);
        match own {
            1.. => Self {
                crew: self.crew + added,
                ..self
            },
            ..0 => Self {
                defence: self.defence + added,
                ..self
            },
            0 => self,
        }
    }
}

/// What a class's default items give its plunder, both ways
/// [`RuleKey::PlunderMarines`](crate::RuleKey::PlunderMarines) reads them,
/// with its raw `Strength`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ClassMarines {
    /// The class's `Strength` (+0xc), raw.
    pub strength: i32,
    /// Its marines as the original counts them: each item's count times
    /// the `ModVal` of the outfit at the item's place, in the mod slot
    /// where the item's own outfit holds marines.
    pub by_place: Marines,
    /// Its marines by each item's own outfit.
    pub by_outfit: Marines,
}

impl ClassMarines {
    /// The marines of a class of `strength` whose default items are
    /// `slots`, item slot by item slot (none where a slot names no item,
    /// with its raw count), among `outfits` (see the module docs): an item
    /// counts when its count is above 0 (@0x84129, @0x84210).
    #[must_use]
    pub fn of(strength: i16, slots: &[Option<(OutfitId, i16)>], outfits: &[OutfitRecord]) -> Self {
        let find = |id: OutfitId| outfits.iter().find(|record| record.id == id);
        let mut marines = Self {
            strength: i32::from(strength),
            ..Self::default()
        };
        for (place, slot) in slots.iter().enumerate() {
            let Some((id, count)) = *slot else {
                continue;
            };
            let Some(own) = find(id).filter(|_| count > 0) else {
                continue;
            };
            let at_place = i16::try_from(place)
                .ok()
                .and_then(|place| find(OutfitId(128 + place)));
            for (mod_slot, &(kind, value)) in own.mods.iter().enumerate() {
                if kind != MARINES {
                    continue;
                }
                let placed = at_place.map_or(0, |record| record.mods[mod_slot].1);
                let count = i32::from(count);
                marines.by_place = marines.by_place.plus(value, count, placed);
                marines.by_outfit = marines.by_outfit.plus(value, count, value);
            }
        }
        marines
    }

    /// Its marines by `reading` of
    /// [`RuleKey::PlunderMarines`](crate::RuleKey::PlunderMarines): by the
    /// item's place by the engine, by its own outfit by the other reading.
    #[must_use]
    pub fn read(&self, reading: RuleSource) -> Marines {
        match reading {
            RuleSource::Engine => self.by_place,
            RuleSource::Bible => self.by_outfit,
        }
    }
}

/// One side of a plunder as the odds count it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Side {
    /// Its class's `Strength`, raw.
    pub strength: i32,
    /// Its marines.
    pub marines: Marines,
}

/// The plunder's odds for `pirate` against `victim`, `jitter` the
/// `Rand(`[`ODDS_JITTER`]`)` drawn (see the module docs).
#[must_use]
pub fn odds(pirate: Side, victim: Side, jitter: u16) -> i32 {
    let attack = pirate.strength + pirate.marines.crew;
    let defend = (victim.strength + victim.marines.crew).max(1);
    // f32: the original stores 2 × victim as a float before dividing.
    #[allow(clippy::cast_precision_loss)]
    let divisor = f64::from(2.0 * defend as f32);
    // Within the range of an i32 for any class's strength.
    #[allow(clippy::cast_possible_truncation)]
    let ratio = (100.0 * f64::from(attack) / divisor) as i32;
    let ratio = ratio - pirate.marines.defence + victim.marines.defence;
    (ratio - i32::from(jitter) + ODDS_OFFSET).clamp(MIN_ODDS, MAX_ODDS)
}

/// The plunder's odds for `pirate` against `victim`, its jitter drawn on
/// `chance`.
pub fn roll_odds(pirate: Side, victim: Side, chance: &mut (impl Chance + ?Sized)) -> i32 {
    odds(pirate, victim, chance.roll(ODDS_JITTER))
}

/// Moves the cargo a pirate steals from `victim`, its six commodities, into
/// `pirate`'s hold (see the module docs): no more than the pirate's free
/// holds, its class's `pirate_holds` less what it carries, nor than the
/// victim's `victim_holds`. Each pass picks a commodity the victim holds on
/// `chance`. The tons stolen.
pub fn steal(
    victim: &mut [u32; NPC_COMMODITIES],
    victim_holds: u32,
    pirate: &mut NpcHold,
    pirate_holds: u16,
    chance: &mut (impl Chance + ?Sized),
) -> u32 {
    let mut free = i64::from(pirate_holds) - i64::try_from(pirate.total()).unwrap_or(i64::MAX);
    let holds = i64::from(victim_holds);
    let mut left: i64 = victim
        .iter()
        .map(|&tons| i64::from(tons))
        .sum::<i64>()
        .min(holds);
    let mut stolen: i64 = 0;
    while free > 0 && left > 0 {
        let holding: Vec<usize> = (0..NPC_COMMODITIES).filter(|&at| victim[at] > 0).collect();
        let Some(&at) = u32::try_from(holding.len())
            .ok()
            .and_then(|len| holding.get(chance.below(len) as usize))
        else {
            break;
        };
        let tons = i64::from(victim[at]);
        let mut take = free.min(tons);
        if take + stolen > holds {
            take = holds - stolen;
        }
        let take = take.max(0);
        // In 0..=tons, which is a u32.
        let moved = u32::try_from(take).unwrap_or(0);
        victim[at] -= moved;
        // In 0..6.
        #[allow(clippy::cast_possible_truncation)]
        pirate.add(at as u8, moved);
        free -= take;
        left -= take;
        stolen += take;
    }
    u32::try_from(stolen).unwrap_or(u32::MAX)
}

/// The credits a plundered player loses at `odds`, from `cash`:
/// trunc(cash × (odds × 30 × 0.01) × 0.01), in doubles
/// (@0x8450e-0x8453b); none from no cash.
#[must_use]
pub fn credits_taken(cash: i64, odds: i32) -> i64 {
    let share = f64::from(odds * 30) * 0.01;
    // The player's cash and a share of it fit an i64, as doubles do.
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    let taken = (cash as f64 * share * 0.01) as i64;
    taken
}

/// Whether a plunder at `odds` takes the victim into the pirate's fleet,
/// its draw rolled on `chance` (see the module docs): no draw at odds of
/// [`CAPTURE_ODDS_FLOOR`] or below.
pub fn captures(odds: i32, chance: &mut (impl Chance + ?Sized)) -> bool {
    odds > CAPTURE_ODDS_FLOOR
        && f64::from(chance.roll(CAPTURE_ROLL)) <= f64::from(odds) * CAPTURE_SHARE
}

/// Who a plundered ship was, as the line and the capture read it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Victim {
    /// The player.
    Player,
    /// An NPC.
    Npc {
        /// Its leader (+0x9a): led by no one, by the player or by an NPC.
        led: Led,
        /// What it was as a former escort of the player's (+0xc8de), if it
        /// is one.
        former: Option<FormerKind>,
        /// Whether it is an active mission's special ship (+0xc8d2).
        mission: bool,
    },
}

/// Who leads a ship (+0x9a).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Led {
    /// No one (-1).
    ByNone,
    /// The player (0).
    ByPlayer,
    /// An NPC.
    ByNpc,
}

/// What the cargo line says: the tons and credits stolen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CargoLine {
    /// The tons of cargo stolen.
    pub tons: u32,
    /// The credits stolen.
    pub credits: i64,
}

/// The cargo line said as `victim` loses `tons` and `credits`, if any (see
/// the module docs): for the player, or a leaderless former hired or
/// captured escort that is no mission ship (@0x8455c-0x8458d), when either
/// is above 0 (@0x8459e-0x845c6). A former fighter gets none.
#[must_use]
pub fn cargo_line(victim: Victim, tons: u32, credits: i64) -> Option<CargoLine> {
    let said = matches!(
        victim,
        Victim::Player
            | Victim::Npc {
                led: Led::ByNone,
                former: Some(FormerKind::Hired | FormerKind::Captured),
                mission: false,
            }
    );
    (said && (tons > 0 || credits > 0)).then_some(CargoLine { tons, credits })
}

/// How a captured ship is announced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Taken {
    /// "Escort stolen!" (STR# 2002 #168, #374).
    Escort,
    /// "Fighter stolen!" (#169, #374): a former fighter.
    Fighter,
    /// Not at all.
    Silently,
}

/// How the capture of a ship `led` and `former` is announced
/// (@0x847fa-0x84834): one the player leads, or any former escort, by
/// its kind; any other silently. Only an announced capture drops every
/// ship that targets it (@0x8484a-0x848b1).
#[must_use]
pub fn taken(led: Led, former: Option<FormerKind>) -> Taken {
    match (led, former) {
        (_, Some(FormerKind::Fighter)) => Taken::Fighter,
        (Led::ByPlayer, _) | (_, Some(_)) => Taken::Escort,
        _ => Taken::Silently,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nova_sim_base::fixture::Draws;
    use nova_sim_catalog::fixture::outfit;

    fn side(strength: i32) -> Side {
        Side {
            strength,
            marines: Marines::default(),
        }
    }

    fn manned(strength: i32, crew: i32, defence: i32) -> Side {
        Side {
            strength,
            marines: Marines { crew, defence },
        }
    }

    #[test]
    fn equal_strengths_give_odds_of_60_less_the_jitter() {
        assert_eq!(odds(side(100), side(100), 0), 60);
        assert_eq!(odds(side(100), side(100), 20), 40);
        assert_eq!(ODDS_JITTER, 21);
    }

    #[test]
    fn the_odds_are_held_to_10_to_100() {
        assert_eq!(odds(side(1), side(100), 0), MIN_ODDS);
        assert_eq!(odds(side(1), side(100), 20), 10);
        assert_eq!(odds(side(1000), side(10), 0), MAX_ODDS);
        assert_eq!(odds(side(1000), side(10), 20), 100);
        // Ratio 90 + 10 - jitter: the edges of the hold.
        assert_eq!(odds(side(180), side(100), 0), 100);
        assert_eq!(odds(side(182), side(100), 0), 100, "ratio 91 → 101, held");
        assert_eq!(odds(side(2), side(100), 0), 11, "ratio 1");
        assert_eq!(odds(side(0), side(100), 0), 10, "ratio 0");
        assert_eq!(odds(side(0), side(100), 1), 10, "9, held");
    }

    #[test]
    fn the_ratio_is_truncated_and_a_victim_of_no_strength_reads_as_1() {
        // 100 × 1 / 6 = 16.67 → 16.
        assert_eq!(odds(side(1), side(3), 0), 26);
        // 100 × -1 / 2 = -50 → -50, truncated toward 0.
        assert_eq!(odds(side(1), side(0), 0), 60, "0 reads as 1");
        assert_eq!(odds(side(1), side(-5), 0), 60, "below 0 too");
        assert_eq!(odds(side(1), manned(-5, 6, 0), 0), 60, "1 with its crew");
        assert_eq!(odds(side(1), manned(-5, 7, 0), 0), 35, "2 with its crew");
    }

    #[test]
    fn crew_raises_each_sides_strength_and_defence_moves_the_ratio() {
        assert_eq!(odds(manned(50, 50, 0), side(100), 0), 60);
        assert_eq!(odds(side(100), manned(50, 50, 0), 0), 60);
        assert_eq!(odds(manned(100, 0, -7), side(100), 0), 67, "pirate's raise");
        assert_eq!(odds(side(100), manned(100, 0, -7), 0), 53, "victim's lower");
    }

    #[test]
    fn the_odds_roll_the_jitter_on_21_sides() {
        let mut chance = nova_sim_base::fixture::Scripted::rolling(&[20]);
        assert_eq!(roll_odds(side(100), side(100), &mut chance), 40);
        assert_eq!(chance.sides_asked, [ODDS_JITTER]);
    }

    #[test]
    fn the_player_counts_each_owned_marines_outfit_by_its_own_mod_value() {
        let marines = Marines::owned([
            (MARINES, 25, 2),
            (MARINES, -4, 3),
            (1, 9, 5),
            (MARINES, 0, 9),
        ]);
        assert_eq!(
            marines,
            Marines {
                crew: 50,
                defence: -12
            }
        );
    }

    /// Outfits 128-131, where 131's first `ModVal` is 131 and 130's -6, and
    /// 227, marines +25 in its first slot, and 228, marines -3 in its
    /// second.
    fn outfits() -> Vec<OutfitRecord> {
        vec![
            outfit(128, &[(1, 128)]),
            outfit(129, &[(1, 129), (2, 11)]),
            outfit(130, &[(1, -6)]),
            outfit(131, &[(1, 131)]),
            outfit(227, &[(MARINES, 25)]),
            outfit(228, &[(1, 1), (MARINES, -3)]),
        ]
    }

    #[test]
    fn a_class_counts_its_items_marines_by_their_place_and_by_their_own_outfit() {
        // 2 of 227 in item slot 3, as the Unrelenting carries them.
        let slots = [
            Some((OutfitId(140), 1)),
            Some((OutfitId(141), 1)),
            None,
            Some((OutfitId(227), 2)),
        ];
        let class = ClassMarines::of(180, &slots, &outfits());
        assert_eq!(class.strength, 180);
        assert_eq!(
            class.by_place,
            Marines {
                crew: 262,
                defence: 0
            },
            "131 × 2"
        );
        assert_eq!(
            class.by_outfit,
            Marines {
                crew: 50,
                defence: 0
            },
            "25 × 2"
        );
        assert_eq!(class.read(RuleSource::Engine), class.by_place);
        assert_eq!(class.read(RuleSource::Bible), class.by_outfit);
    }

    #[test]
    fn a_classs_negative_marines_read_the_place_in_the_same_mod_slot() {
        // 228's marines sit in its second slot: slot 1 of the outfit at
        // place 1 (129) is 11.
        let slots = [None, Some((OutfitId(228), 4))];
        let class = ClassMarines::of(0, &slots, &outfits());
        assert_eq!(
            class.by_place,
            Marines {
                crew: 0,
                defence: 44
            }
        );
        assert_eq!(
            class.by_outfit,
            Marines {
                crew: 0,
                defence: -12
            }
        );
        // At place 2 (130), slot 1 is empty.
        let slots = [None, None, Some((OutfitId(228), 4))];
        assert_eq!(
            ClassMarines::of(0, &slots, &outfits()).by_place,
            Marines::default()
        );
    }

    #[test]
    fn a_class_item_of_no_count_or_no_record_and_a_place_with_no_outfit_count_nothing() {
        let slots = [None, None, None, Some((OutfitId(227), 0))];
        assert_eq!(
            ClassMarines::of(5, &slots, &outfits()).by_outfit,
            Marines::default()
        );
        let slots = [None, None, None, Some((OutfitId(227), -1))];
        assert_eq!(
            ClassMarines::of(5, &slots, &outfits()).by_outfit,
            Marines::default()
        );
        let slots = [Some((OutfitId(999), 3))];
        assert_eq!(
            ClassMarines::of(5, &slots, &outfits()),
            ClassMarines {
                strength: 5,
                ..ClassMarines::default()
            }
        );
        let mut far = vec![None; 8];
        far[7] = Some((OutfitId(227), 1));
        let class = ClassMarines::of(5, &far, &outfits());
        assert_eq!(class.by_place, Marines::default(), "no outfit 135");
        assert_eq!(
            class.by_outfit,
            Marines {
                crew: 25,
                defence: 0
            }
        );
    }

    /// A pirate hold carrying `tons` of commodity 0.
    fn carrying(tons: u32) -> NpcHold {
        let mut hold = NpcHold::default();
        hold.add(0, tons);
        hold
    }

    #[test]
    fn the_pirate_takes_a_commodity_into_its_free_holds() {
        let mut victim = [0, 0, 30, 0, 0, 0];
        let mut pirate = carrying(5);
        let stolen = steal(&mut victim, 100, &mut pirate, 20, &mut Draws::of(&[0; 8]));
        assert_eq!(stolen, 15, "20 holds less 5 carried");
        assert_eq!(victim, [0, 0, 15, 0, 0, 0]);
        assert_eq!((pirate.tons(0), pirate.tons(2)), (5, 15));
        let mut victim = [0, 0, 30, 0, 0, 0];
        let mut pirate = carrying(0);
        assert_eq!(
            steal(&mut victim, 100, &mut pirate, 50, &mut Draws::of(&[0; 8])),
            30
        );
        assert_eq!(victim, [0; 6]);
    }

    #[test]
    fn the_pirate_takes_nothing_without_free_holds_or_from_an_empty_victim() {
        for (carried, holds) in [(20, 20), (25, 20)] {
            let mut victim = [9, 0, 0, 0, 0, 0];
            let mut pirate = carrying(carried);
            let mut chance = Draws::of(&[]);
            assert_eq!(steal(&mut victim, 100, &mut pirate, holds, &mut chance), 0);
            assert!(chance.asked.is_empty(), "no pass");
            assert_eq!(victim, [9, 0, 0, 0, 0, 0]);
        }
        let mut victim = [0; 6];
        assert_eq!(
            steal(&mut victim, 100, &mut carrying(0), 50, &mut Draws::of(&[])),
            0
        );
        let mut victim = [9, 0, 0, 0, 0, 0];
        assert_eq!(
            steal(&mut victim, 0, &mut carrying(0), 50, &mut Draws::of(&[])),
            0,
            "no holds"
        );
    }

    #[test]
    fn a_victim_holding_more_than_its_holds_loses_only_its_holds_worth() {
        let mut victim = [40, 0, 0, 0, 0, 30];
        let mut pirate = carrying(0);
        let stolen = steal(
            &mut victim,
            50,
            &mut pirate,
            200,
            &mut Draws::of(&[1, 0, 0]),
        );
        assert_eq!(stolen, 50);
        assert_eq!(victim, [20, 0, 0, 0, 0, 0], "30 of 5, then 20 of 0");
        assert_eq!((pirate.tons(0), pirate.tons(5)), (20, 30));
    }

    #[test]
    fn the_draws_pick_among_the_commodities_held_and_the_room_decides_the_total() {
        // Room for both: both go, whichever is picked first.
        for first in [0, 1] {
            let mut victim = [0, 7, 0, 5, 0, 0];
            let mut pirate = carrying(0);
            let stolen = steal(
                &mut victim,
                100,
                &mut pirate,
                50,
                &mut Draws::of(&[first, 0]),
            );
            assert_eq!((stolen, victim), (12, [0; 6]), "{first}");
        }
        // Room for less: the room is taken, whichever is picked.
        for first in [0, 1] {
            let mut victim = [0, 7, 0, 5, 0, 0];
            let mut pirate = carrying(0);
            let stolen = steal(
                &mut victim,
                100,
                &mut pirate,
                9,
                &mut Draws::of(&[first, 0]),
            );
            assert_eq!(stolen, 9, "{first}");
            assert_eq!(victim.iter().sum::<u32>(), 3);
        }
        let mut chance = Draws::of(&[1, 0]);
        let mut victim = [0, 7, 0, 5, 0, 0];
        steal(&mut victim, 100, &mut carrying(0), 50, &mut chance);
        assert_eq!(chance.asked, [2, 1], "a pick among those held each pass");
    }

    #[test]
    fn the_player_loses_3_to_30_percent_of_its_cash() {
        assert_eq!(credits_taken(10_000, 10), 300);
        assert_eq!(credits_taken(10_000, 100), 3000);
        // 999 × 0.03 = 29.97 → 29; × 0.3 = 299.7 → 299.
        assert_eq!(credits_taken(999, 10), 29);
        assert_eq!(credits_taken(999, 100), 299);
        assert_eq!(credits_taken(0, 100), 0);
    }

    #[test]
    fn a_capture_needs_odds_above_40_and_a_draw_at_most_half_of_them() {
        let roll = |odds, drawn| {
            captures(
                odds,
                &mut nova_sim_base::fixture::Scripted::rolling(&[drawn]),
            )
        };
        assert!(!roll(40, 0), "never at 40");
        assert!(roll(41, 20));
        assert!(!roll(41, 21), "20.5");
        assert!(roll(100, 50));
        assert!(!roll(100, 51));
        let mut chance = nova_sim_base::fixture::Scripted::rolling(&[0]);
        captures(40, &mut chance);
        assert!(chance.sides_asked.is_empty(), "no draw at 40");
        captures(41, &mut chance);
        assert_eq!(chance.sides_asked, [CAPTURE_ROLL]);
    }

    fn npc(led: Led, former: Option<FormerKind>, mission: bool) -> Victim {
        Victim::Npc {
            led,
            former,
            mission,
        }
    }

    #[test]
    fn the_cargo_line_is_said_for_the_player_or_a_leaderless_former_escort_when_anything_went() {
        let line = |tons, credits| Some(CargoLine { tons, credits });
        assert_eq!(cargo_line(Victim::Player, 12, 0), line(12, 0));
        assert_eq!(cargo_line(Victim::Player, 0, 1500), line(0, 1500));
        assert_eq!(cargo_line(Victim::Player, 1, 1), line(1, 1));
        assert_eq!(cargo_line(Victim::Player, 0, 0), None);
        for kind in [FormerKind::Hired, FormerKind::Captured] {
            assert_eq!(
                cargo_line(npc(Led::ByNone, Some(kind), false), 3, 0),
                line(3, 0)
            );
            assert_eq!(
                cargo_line(npc(Led::ByNone, Some(kind), true), 3, 0),
                None,
                "mission"
            );
            assert_eq!(cargo_line(npc(Led::ByNpc, Some(kind), false), 3, 0), None);
            assert_eq!(
                cargo_line(npc(Led::ByPlayer, Some(kind), false), 3, 0),
                None
            );
        }
        assert_eq!(
            cargo_line(npc(Led::ByNone, Some(FormerKind::Fighter), false), 3, 0),
            None
        );
        assert_eq!(
            cargo_line(npc(Led::ByNone, None, false), 3, 0),
            None,
            "a stranger"
        );
    }

    #[test]
    fn a_former_escort_or_a_ship_the_player_leads_is_announced_and_any_other_taken_silently() {
        assert_eq!(taken(Led::ByNone, Some(FormerKind::Hired)), Taken::Escort);
        assert_eq!(
            taken(Led::ByNone, Some(FormerKind::Captured)),
            Taken::Escort
        );
        assert_eq!(
            taken(Led::ByNone, Some(FormerKind::Fighter)),
            Taken::Fighter
        );
        assert_eq!(taken(Led::ByNpc, Some(FormerKind::Hired)), Taken::Escort);
        assert_eq!(taken(Led::ByPlayer, None), Taken::Escort);
        assert_eq!(taken(Led::ByNone, None), Taken::Silently);
        assert_eq!(taken(Led::ByNpc, None), Taken::Silently);
    }
}
