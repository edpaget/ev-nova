//! Set expressions: what happens when the player does something.

use std::collections::BTreeSet;
use std::ops::RangeInclusive;

use serde::{Deserialize, Serialize};

use super::test::MAX_NUMBER;
use super::token::{Cursor, Spec, is_space};
use super::{Bit, ParseError, ParseErrorKind};
use crate::wire::id::{
    MissionId, OutfitId, RankId, ShipId, SoundId, StellarId, StrListId, SystemId,
};

/// A parsed set expression: its operators in the order they run. A set
/// expression that failed to parse does nothing.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetExpr {
    /// The operators, in order; empty for a blank expression.
    pub ops: Vec<SetOp>,
}

/// One set operator.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SetOp {
    /// `bxxx`: set the bit.
    Set(Bit),
    /// `!bxxx`: clear the bit.
    Clear(Bit),
    /// `^bxxx`: toggle the bit.
    Toggle(Bit),
    /// `R(<op> <op>)`: run one of the two, chosen at random.
    Random(Box<SetOp>, Box<SetOp>),
    /// `Axxx`: abort the mission if it is active.
    AbortMission(MissionId),
    /// `Fxxx`: fail the mission if it is active.
    FailMission(MissionId),
    /// `Sxxx`: start the mission.
    StartMission(MissionId),
    /// `Gxxx`: grant one of the outfit.
    GrantOutfit(OutfitId),
    /// `Dxxx`: remove one of the outfit.
    RemoveOutfit(OutfitId),
    /// `Cxxx`: change the player's ship, keeping its outfits.
    ChangeShip(ShipId),
    /// `Exxx`: change the player's ship, keeping its outfits and adding the
    /// new ship's defaults.
    ChangeShipWithDefaults(ShipId),
    /// `Hxxx`: change the player's ship, dropping the outfits that are not
    /// persistent.
    ReplaceShip(ShipId),
    /// `Mxxx`: move the player to the system, at its first stellar.
    MoveTo(SystemId),
    /// `Nxxx`: move the player to the system, keeping the ship's position.
    MoveKeepPosition(SystemId),
    /// `Kxxx`: activate the rank.
    ActivateRank(RankId),
    /// `Lxxx`: deactivate the rank.
    DeactivateRank(RankId),
    /// `Pxxx`: play the sound.
    PlaySound(SoundId),
    /// `Yxxx`: destroy the stellar.
    DestroyStellar(StellarId),
    /// `Uxxx`: regenerate the stellar.
    RegenerateStellar(StellarId),
    /// `Txxx`: rename the player's ship from a string in the `STR#`.
    RenameShip(StrListId),
    /// `Qxxx`: make the player leave the stellar, with a message from the
    /// `STR#`.
    LeaveStellar(StrListId),
    /// `Xxxx`: mark the system explored.
    Explore(SystemId),
}

/// Which operator a [`SetOp`] is, without its operand: each is the
/// [`SetOp`] variant of the same name.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum SetOpKind {
    Set,
    Clear,
    Toggle,
    Random,
    AbortMission,
    FailMission,
    StartMission,
    GrantOutfit,
    RemoveOutfit,
    ChangeShip,
    ChangeShipWithDefaults,
    ReplaceShip,
    MoveTo,
    MoveKeepPosition,
    ActivateRank,
    DeactivateRank,
    PlaySound,
    DestroyStellar,
    RegenerateStellar,
    RenameShip,
    LeaveStellar,
    Explore,
}

/// How a set operator writes a bit.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BitWrite {
    /// `b`: set it.
    Set,
    /// `!b`: clear it.
    Clear,
    /// `^b`: toggle it.
    Toggle,
}

impl SetExpr {
    /// Parses a set expression; blank has no operators.
    pub fn parse(src: &str) -> Result<Self, ParseError> {
        let mut cursor = Cursor::new(src);
        let mut ops = Vec::new();
        cursor.skip_space();
        while cursor.peek().is_some() {
            let op = if cursor.peek().map(|c| c.to_ascii_uppercase()) == Some(b'R') {
                random(&mut cursor)?
            } else {
                op(&mut cursor)?
            };
            let random = matches!(op, SetOp::Random(..));
            match cursor.peek() {
                Some(next) if !random && replaces(next) => continue,
                Some(next) if !random && !is_space(next) => return Err(after_op(&cursor)),
                _ => ops.push(op),
            }
            cursor.skip_space();
        }
        Ok(Self { ops })
    }

    /// The bits always written, in order: those outside `R(...)`.
    #[must_use]
    pub fn writes(&self) -> Vec<(Bit, BitWrite)> {
        self.ops.iter().filter_map(SetOp::bit_write).collect()
    }

    /// The bits that may be written, in order: those inside `R(...)`.
    #[must_use]
    pub fn maybe_writes(&self) -> Vec<(Bit, BitWrite)> {
        self.arms().filter_map(SetOp::bit_write).collect()
    }

    /// The bits whose old value matters: the toggled ones.
    #[must_use]
    pub fn reads(&self) -> BTreeSet<Bit> {
        self.all_ops()
            .filter_map(|op| match op {
                SetOp::Toggle(bit) => Some(*bit),
                _ => None,
            })
            .collect()
    }

    /// Every operator kind in the expression, the arms of `R(...)` included.
    #[must_use]
    pub fn kinds(&self) -> BTreeSet<SetOpKind> {
        self.all_ops().map(SetOp::kind).collect()
    }

    /// The operators inside `R(...)`, in order.
    fn arms(&self) -> impl Iterator<Item = &SetOp> {
        self.ops.iter().flat_map(|op| match op {
            SetOp::Random(a, b) => vec![&**a, &**b],
            _ => Vec::new(),
        })
    }

    /// Every operator: those outside `R(...)`, then those inside.
    fn all_ops(&self) -> impl Iterator<Item = &SetOp> {
        self.ops.iter().chain(self.arms())
    }
}

impl SetOp {
    /// Which operator this is.
    #[must_use]
    pub fn kind(&self) -> SetOpKind {
        match self {
            Self::Set(_) => SetOpKind::Set,
            Self::Clear(_) => SetOpKind::Clear,
            Self::Toggle(_) => SetOpKind::Toggle,
            Self::Random(..) => SetOpKind::Random,
            Self::AbortMission(_) => SetOpKind::AbortMission,
            Self::FailMission(_) => SetOpKind::FailMission,
            Self::StartMission(_) => SetOpKind::StartMission,
            Self::GrantOutfit(_) => SetOpKind::GrantOutfit,
            Self::RemoveOutfit(_) => SetOpKind::RemoveOutfit,
            Self::ChangeShip(_) => SetOpKind::ChangeShip,
            Self::ChangeShipWithDefaults(_) => SetOpKind::ChangeShipWithDefaults,
            Self::ReplaceShip(_) => SetOpKind::ReplaceShip,
            Self::MoveTo(_) => SetOpKind::MoveTo,
            Self::MoveKeepPosition(_) => SetOpKind::MoveKeepPosition,
            Self::ActivateRank(_) => SetOpKind::ActivateRank,
            Self::DeactivateRank(_) => SetOpKind::DeactivateRank,
            Self::PlaySound(_) => SetOpKind::PlaySound,
            Self::DestroyStellar(_) => SetOpKind::DestroyStellar,
            Self::RegenerateStellar(_) => SetOpKind::RegenerateStellar,
            Self::RenameShip(_) => SetOpKind::RenameShip,
            Self::LeaveStellar(_) => SetOpKind::LeaveStellar,
            Self::Explore(_) => SetOpKind::Explore,
        }
    }

    /// The bit this operator writes, and how, if it writes one.
    fn bit_write(&self) -> Option<(Bit, BitWrite)> {
        match *self {
            Self::Set(bit) => Some((bit, BitWrite::Set)),
            Self::Clear(bit) => Some((bit, BitWrite::Clear)),
            Self::Toggle(bit) => Some((bit, BitWrite::Toggle)),
            _ => None,
        }
    }
}

/// Mission IDs the original acts on.
const MISSIONS: RangeInclusive<u16> = 128..=1127;
/// Outfit IDs the original acts on.
const OUTFITS: RangeInclusive<u16> = 128..=639;
/// Ship IDs the original acts on.
const SHIPS: RangeInclusive<u16> = 128..=895;
/// System and stellar IDs the original acts on.
const SYSTEMS: RangeInclusive<u16> = 128..=2175;
/// Rank IDs the original acts on.
const RANKS: RangeInclusive<u16> = 128..=255;
/// Any ID: the original passes sound and `STR#` IDs on unchecked.
const ANY: RangeInclusive<u16> = 0..=MAX_NUMBER;

/// The set operator letters, `B` among them (`!b`, `^b` and `R` are read
/// apart).
pub(super) fn letter_spec(letter: u8) -> Option<Spec<SetOp>> {
    let (range, build): (_, fn(u16) -> SetOp) = match letter {
        b'B' => (0..=Bit::MAX, |n| SetOp::Set(Bit(n))),
        b'A' => (MISSIONS, |n| SetOp::AbortMission(MissionId(n as i16))),
        b'F' => (MISSIONS, |n| SetOp::FailMission(MissionId(n as i16))),
        b'S' => (MISSIONS, |n| SetOp::StartMission(MissionId(n as i16))),
        b'G' => (OUTFITS, |n| SetOp::GrantOutfit(OutfitId(n as i16))),
        b'D' => (OUTFITS, |n| SetOp::RemoveOutfit(OutfitId(n as i16))),
        b'C' => (SHIPS, |n| SetOp::ChangeShip(ShipId(n as i16))),
        b'E' => (SHIPS, |n| SetOp::ChangeShipWithDefaults(ShipId(n as i16))),
        b'H' => (SHIPS, |n| SetOp::ReplaceShip(ShipId(n as i16))),
        b'M' => (SYSTEMS, |n| SetOp::MoveTo(SystemId(n as i16))),
        b'N' => (SYSTEMS, |n| SetOp::MoveKeepPosition(SystemId(n as i16))),
        b'K' => (RANKS, |n| SetOp::ActivateRank(RankId(n as i16))),
        b'L' => (RANKS, |n| SetOp::DeactivateRank(RankId(n as i16))),
        b'P' => (ANY, |n| SetOp::PlaySound(SoundId(n as i16))),
        b'Y' => (SYSTEMS, |n| SetOp::DestroyStellar(StellarId(n as i16))),
        b'U' => (SYSTEMS, |n| SetOp::RegenerateStellar(StellarId(n as i16))),
        b'T' => (ANY, |n| SetOp::RenameShip(StrListId(n as i16))),
        b'Q' => (ANY, |n| SetOp::LeaveStellar(StrListId(n as i16))),
        b'X' => (SYSTEMS, |n| SetOp::Explore(SystemId(n as i16))),
        _ => return None,
    };
    Some(Spec {
        range: Some(range),
        build,
    })
}

/// Whether `next`, straight after an operator's number, replaces that
/// operator: an operator letter other than `b` and `R`.
fn replaces(next: u8) -> bool {
    let letter = next.to_ascii_uppercase();
    letter != b'B' && next.is_ascii_alphabetic() && letter_spec(letter).is_some()
}

/// The error for a character that cannot follow an operator's number.
fn after_op(cursor: &Cursor<'_>) -> ParseError {
    match cursor.peek().map(|c| c.to_ascii_uppercase()) {
        Some(letter)
            if letter.is_ascii_alphabetic() && letter != b'R' && letter_spec(letter).is_none() =>
        {
            cursor.error(ParseErrorKind::UnknownOperator(cursor.char()))
        }
        _ => cursor.unexpected(),
    }
}

/// Reads one operator other than `R`: `!bxxx`, `^bxxx`, or a letter and its
/// number.
fn op(cursor: &mut Cursor<'_>) -> Result<SetOp, ParseError> {
    let build: fn(Bit) -> SetOp = match cursor.peek() {
        Some(b'!') => SetOp::Clear,
        Some(b'^') => SetOp::Toggle,
        _ => return cursor.operand(letter_spec, ParseErrorKind::UnknownOperator),
    };
    cursor.bump();
    if cursor.peek().map(|c| c.to_ascii_uppercase()) != Some(b'B') {
        return Err(cursor.error(ParseErrorKind::NegatedNonBit));
    }
    cursor.bump();
    Ok(build(Bit(cursor.number(0..=Bit::MAX)?)))
}

/// Reads `R(<op> <op>)`, in the one spelling the original runs as written.
fn random(cursor: &mut Cursor<'_>) -> Result<SetOp, ParseError> {
    let bad = |cursor: &Cursor<'_>| cursor.error(ParseErrorKind::BadRandom);
    cursor.bump();
    if cursor.peek() != Some(b'(') {
        return Err(bad(cursor));
    }
    let open = cursor.pos();
    let unclosed = ParseError {
        at: open,
        kind: ParseErrorKind::Unclosed('('),
    };
    cursor.bump();
    // The original skips the first operator by stepping over the one
    // character after the `(`: it must be the operator's letter.
    match cursor.peek() {
        None => return Err(unclosed),
        Some(first) if !first.is_ascii_alphabetic() || first.eq_ignore_ascii_case(&b'R') => {
            return Err(bad(cursor));
        }
        Some(_) => {}
    }
    let first = op(cursor)?;
    match cursor.peek() {
        None => return Err(unclosed),
        Some(b')') => return Err(bad(cursor)),
        Some(next) if !is_space(next) => return Err(after_op(cursor)),
        Some(_) => cursor.bump(),
    }
    // One separator only: a second would run the skipped operator's turn.
    match cursor.peek() {
        None => return Err(unclosed),
        Some(next) if is_space(next) || next == b')' || next.eq_ignore_ascii_case(&b'R') => {
            return Err(bad(cursor));
        }
        Some(_) => {}
    }
    let second = op(cursor)?;
    cursor.skip_space();
    match cursor.peek() {
        Some(b')') => {
            cursor.bump();
            Ok(SetOp::Random(Box::new(first), Box::new(second)))
        }
        None => Err(unclosed),
        Some(_) => Err(bad(cursor)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::expr::ParseErrorKind as K;

    fn parse(src: &str) -> Vec<SetOp> {
        SetExpr::parse(src)
            .unwrap_or_else(|err| panic!("{src:?}: {err}"))
            .ops
    }

    fn fails(src: &str) -> ParseError {
        SetExpr::parse(src).expect_err(src)
    }

    fn err(at: usize, kind: ParseErrorKind) -> ParseError {
        ParseError { at, kind }
    }

    fn b(n: u16) -> Bit {
        Bit::new(n).expect("in range")
    }

    fn random(a: SetOp, b: SetOp) -> SetOp {
        SetOp::Random(Box::new(a), Box::new(b))
    }

    fn range(value: u32, min: u16, max: u16) -> ParseErrorKind {
        K::NumberOutOfRange { value, min, max }
    }

    #[test]
    fn a_blank_set_does_nothing() {
        assert_eq!(parse(""), []);
        assert_eq!(parse(" \t "), []);
    }

    #[test]
    fn b_sets_clears_and_toggles_bits_from_0_to_9999() {
        assert_eq!(parse("b0"), [SetOp::Set(b(0))]);
        assert_eq!(parse("B9999"), [SetOp::Set(b(9999))]);
        assert_eq!(parse("!b12"), [SetOp::Clear(b(12))]);
        assert_eq!(parse("!B12"), [SetOp::Clear(b(12))]);
        assert_eq!(parse("^b12"), [SetOp::Toggle(b(12))]);
        assert_eq!(fails("b10000"), err(1, range(10000, 0, 9999)));
        assert_eq!(fails("!b10000"), err(2, range(10000, 0, 9999)));
    }

    #[test]
    fn every_operator_letter_reads_in_either_case_within_its_range() {
        // (letter, lowest, highest, what the lowest builds)
        let cases: [(char, u16, u16, SetOp); 19] = [
            ('A', 128, 1127, SetOp::AbortMission(MissionId(128))),
            ('F', 128, 1127, SetOp::FailMission(MissionId(128))),
            ('S', 128, 1127, SetOp::StartMission(MissionId(128))),
            ('G', 128, 639, SetOp::GrantOutfit(OutfitId(128))),
            ('D', 128, 639, SetOp::RemoveOutfit(OutfitId(128))),
            ('C', 128, 895, SetOp::ChangeShip(ShipId(128))),
            ('E', 128, 895, SetOp::ChangeShipWithDefaults(ShipId(128))),
            ('H', 128, 895, SetOp::ReplaceShip(ShipId(128))),
            ('M', 128, 2175, SetOp::MoveTo(SystemId(128))),
            ('N', 128, 2175, SetOp::MoveKeepPosition(SystemId(128))),
            ('K', 128, 255, SetOp::ActivateRank(RankId(128))),
            ('L', 128, 255, SetOp::DeactivateRank(RankId(128))),
            ('P', 0, 32767, SetOp::PlaySound(SoundId(0))),
            ('Y', 128, 2175, SetOp::DestroyStellar(StellarId(128))),
            ('U', 128, 2175, SetOp::RegenerateStellar(StellarId(128))),
            ('T', 0, 32767, SetOp::RenameShip(StrListId(0))),
            ('Q', 0, 32767, SetOp::LeaveStellar(StrListId(0))),
            ('X', 128, 2175, SetOp::Explore(SystemId(128))),
            ('B', 0, 9999, SetOp::Set(b(0))),
        ];
        for (letter, min, max, op) in cases {
            let lower = letter.to_ascii_lowercase();
            assert_eq!(
                parse(&format!("{letter}{min}")),
                std::slice::from_ref(&op),
                "{letter}"
            );
            assert_eq!(parse(&format!("{lower}{min}")), [op], "{lower}");
            assert!(
                SetExpr::parse(&format!("{letter}{max}")).is_ok(),
                "{letter}{max}"
            );
            let past = u32::from(max) + 1;
            assert_eq!(
                fails(&format!("{letter}{past}")),
                err(1, range(past, min, max))
            );
            if min > 0 {
                let below = u32::from(min) - 1;
                assert_eq!(
                    fails(&format!("{letter}{below}")),
                    err(1, range(below, min, max))
                );
            }
        }
    }

    #[test]
    fn each_operator_builds_its_own_variant() {
        assert_eq!(
            parse("A200 F201 S202 G203 D204 C205 E206 H207 M208 N209 K210 L211"),
            [
                SetOp::AbortMission(MissionId(200)),
                SetOp::FailMission(MissionId(201)),
                SetOp::StartMission(MissionId(202)),
                SetOp::GrantOutfit(OutfitId(203)),
                SetOp::RemoveOutfit(OutfitId(204)),
                SetOp::ChangeShip(ShipId(205)),
                SetOp::ChangeShipWithDefaults(ShipId(206)),
                SetOp::ReplaceShip(ShipId(207)),
                SetOp::MoveTo(SystemId(208)),
                SetOp::MoveKeepPosition(SystemId(209)),
                SetOp::ActivateRank(RankId(210)),
                SetOp::DeactivateRank(RankId(211)),
            ]
        );
        assert_eq!(
            parse("P212 Y213 U214 T215 Q216 X217"),
            [
                SetOp::PlaySound(SoundId(212)),
                SetOp::DestroyStellar(StellarId(213)),
                SetOp::RegenerateStellar(StellarId(214)),
                SetOp::RenameShip(StrListId(215)),
                SetOp::LeaveStellar(StrListId(216)),
                SetOp::Explore(SystemId(217)),
            ]
        );
    }

    #[test]
    fn the_bible_s_examples_parse() {
        assert_eq!(
            parse("b1 b2 !b3 ^b4"),
            [
                SetOp::Set(b(1)),
                SetOp::Set(b(2)),
                SetOp::Clear(b(3)),
                SetOp::Toggle(b(4)),
            ]
        );
        assert_eq!(
            parse("b1 R(b2 !b3)"),
            [
                SetOp::Set(b(1)),
                random(SetOp::Set(b(2)), SetOp::Clear(b(3)))
            ]
        );
    }

    #[test]
    fn spaces_and_tabs_separate_operators_anywhere() {
        assert_eq!(
            parse("\t b1 \t\t g128  "),
            [SetOp::Set(b(1)), SetOp::GrantOutfit(OutfitId(128))]
        );
    }

    #[test]
    fn random_picks_between_two_operators() {
        assert_eq!(
            parse("r(g374\tg261)"),
            [random(
                SetOp::GrantOutfit(OutfitId(374)),
                SetOp::GrantOutfit(OutfitId(261))
            )]
        );
        assert_eq!(
            parse("R(b1 ^b2 )"),
            [random(SetOp::Set(b(1)), SetOp::Toggle(b(2)))]
        );
        assert_eq!(
            parse("R(b1 b2)b3"),
            [random(SetOp::Set(b(1)), SetOp::Set(b(2))), SetOp::Set(b(3))]
        );
    }

    #[test]
    fn random_written_any_other_way_is_rejected() {
        assert_eq!(fails("R (b1 b2)"), err(1, K::BadRandom));
        assert_eq!(fails("R( b1 b2)"), err(2, K::BadRandom));
        assert_eq!(fails("R(!b1 b2)"), err(2, K::BadRandom));
        assert_eq!(fails("R(^b1 b2)"), err(2, K::BadRandom));
        assert_eq!(fails("R(b1)"), err(4, K::BadRandom));
        assert_eq!(fails("R(b1  b2)"), err(5, K::BadRandom));
        assert_eq!(fails("R(b1 b2 b3)"), err(8, K::BadRandom));
        assert_eq!(fails("R(R(b1 b2) b3)"), err(2, K::BadRandom));
        assert_eq!(fails("R(b1 R(b2 b3))"), err(5, K::BadRandom));
        assert_eq!(fails("R(b1 )"), err(5, K::BadRandom));
        assert_eq!(fails("R"), err(1, K::BadRandom));
        assert_eq!(fails("R(b1 b2"), err(1, K::Unclosed('(')));
        assert_eq!(fails("R(b1"), err(1, K::Unclosed('(')));
        assert_eq!(fails("R("), err(1, K::Unclosed('(')));
        assert_eq!(fails("R(b1 z2)"), err(5, K::UnknownOperator('z')));
        assert_eq!(fails("R(b1S2 b3)"), err(4, K::Unexpected('S')));
    }

    #[test]
    fn an_operator_straight_after_another_s_number_replaces_it() {
        // The stock mïsn 868 has `S862S863`: the original starts only 863.
        assert_eq!(
            parse("A867 S862S863 S864"),
            [
                SetOp::AbortMission(MissionId(867)),
                SetOp::StartMission(MissionId(863)),
                SetOp::StartMission(MissionId(864)),
            ]
        );
        assert_eq!(parse("b12g128"), [SetOp::GrantOutfit(OutfitId(128))]);
        // A `b` there would keep the pending operator, and an `R` would
        // drop it too: neither is accepted.
        assert_eq!(fails("S862b863"), err(4, K::Unexpected('b')));
        assert_eq!(fails("b1R(b2 b3)"), err(2, K::Unexpected('R')));
        assert_eq!(fails("b1!b2"), err(2, K::Unexpected('!')));
    }

    #[test]
    fn not_and_toggle_apply_only_to_bits() {
        assert_eq!(fails("!g128"), err(1, K::NegatedNonBit));
        assert_eq!(fails("^ b1"), err(1, K::NegatedNonBit));
        assert_eq!(fails("!"), err(1, K::NegatedNonBit));
    }

    #[test]
    fn unknown_letters_missing_numbers_and_stray_characters_are_rejected() {
        assert_eq!(fails("b12 Z3"), err(4, K::UnknownOperator('Z')));
        assert_eq!(fails("b12 i3"), err(4, K::UnknownOperator('i')));
        assert_eq!(fails("b12 3"), err(4, K::UnknownOperator('3')));
        assert_eq!(fails("b12z"), err(3, K::UnknownOperator('z')));
        assert_eq!(fails("g"), err(1, K::MissingNumber));
        assert_eq!(fails("b1 g b2"), err(4, K::MissingNumber));
        assert_eq!(fails("b1 (b2)"), err(3, K::UnknownOperator('(')));
        assert_eq!(fails("b1)"), err(2, K::Unexpected(')')));
        assert_eq!(fails("b1,b2"), err(2, K::Unexpected(',')));
    }

    #[test]
    fn writes_lists_the_bits_always_written_and_maybe_writes_those_in_random() {
        let expr = SetExpr::parse("b1 !b2 g128 ^b3 R(b4 !b5) R(g129 ^b6)").expect("parses");
        assert_eq!(
            expr.writes(),
            [
                (b(1), BitWrite::Set),
                (b(2), BitWrite::Clear),
                (b(3), BitWrite::Toggle)
            ]
        );
        assert_eq!(
            expr.maybe_writes(),
            [
                (b(4), BitWrite::Set),
                (b(5), BitWrite::Clear),
                (b(6), BitWrite::Toggle)
            ]
        );
        assert!(SetExpr::default().writes().is_empty());
        assert!(SetExpr::default().maybe_writes().is_empty());
    }

    #[test]
    fn reads_lists_only_the_toggled_bits() {
        let expr = SetExpr::parse("^b9 b1 !b2 R(b3 ^b4) ^b9").expect("parses");
        let bits: Vec<u16> = expr.reads().into_iter().map(Bit::get).collect();
        assert_eq!(bits, [4, 9]);
        assert!(SetExpr::parse("b1 !b2").expect("parses").reads().is_empty());
    }

    #[test]
    fn kinds_lists_every_operator_kind_with_random_arms() {
        let expr = SetExpr::parse("b1 b2 R(c128 !b3) m128").expect("parses");
        assert_eq!(
            expr.kinds().into_iter().collect::<Vec<_>>(),
            [
                SetOpKind::Set,
                SetOpKind::Clear,
                SetOpKind::Random,
                SetOpKind::ChangeShip,
                SetOpKind::MoveTo,
            ]
        );
        assert!(SetExpr::default().kinds().is_empty());
    }

    #[test]
    fn each_operator_reports_its_kind() {
        let expr = SetExpr::parse(
            "b1 !b1 ^b1 R(b1 b2) A128 F128 S128 G128 D128 C128 E128 H128 M128 N128 K128 L128 \
             P128 Y128 U128 T128 Q128 X128",
        )
        .expect("parses");
        let kinds: Vec<SetOpKind> = expr.ops.iter().map(SetOp::kind).collect();
        assert_eq!(
            kinds,
            [
                SetOpKind::Set,
                SetOpKind::Clear,
                SetOpKind::Toggle,
                SetOpKind::Random,
                SetOpKind::AbortMission,
                SetOpKind::FailMission,
                SetOpKind::StartMission,
                SetOpKind::GrantOutfit,
                SetOpKind::RemoveOutfit,
                SetOpKind::ChangeShip,
                SetOpKind::ChangeShipWithDefaults,
                SetOpKind::ReplaceShip,
                SetOpKind::MoveTo,
                SetOpKind::MoveKeepPosition,
                SetOpKind::ActivateRank,
                SetOpKind::DeactivateRank,
                SetOpKind::PlaySound,
                SetOpKind::DestroyStellar,
                SetOpKind::RegenerateStellar,
                SetOpKind::RenameShip,
                SetOpKind::LeaveStellar,
                SetOpKind::Explore,
            ]
        );
    }
}
