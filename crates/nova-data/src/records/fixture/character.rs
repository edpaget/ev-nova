//! The `chär` builder.

use super::Bytes;
use crate::records::character::Character;

/// Builds a `chär` ([`Character`]) record's bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CharacterBuilder(Bytes);

impl Default for CharacterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl CharacterBuilder {
    /// A zeroed `chär`.
    #[must_use]
    pub fn new() -> Self {
        Self(Bytes::zeroed::<Character>())
    }

    /// The `chär` `bytes`, to change some of its fields.
    ///
    /// # Panics
    ///
    /// If `bytes` isn't a whole `chär`.
    #[must_use]
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self(Bytes::of::<Character>(bytes))
    }

    /// The record's bytes.
    #[must_use]
    pub fn build(self) -> Vec<u8> {
        self.0.0
    }

    i16_setters! { Character:
        ship_type @ 0x04,
        kills @ 0x1E,
        intro_text_id @ 0x30,
        unknown_0x134 @ 0x134,
        unknown_0x136 @ 0x136,
        start_year @ 0x138,
    }

    /// Sets [`Character::cash`] (offset 0x00, i32).
    #[must_use]
    pub fn cash(mut self, value: i32) -> Self {
        self.0.put(0x00, &value.to_be_bytes());
        self
    }

    /// Sets [`Character::system`] (offset 0x06, 4 x i16) from slot 0.
    /// Slots past `ids` keep what they held.
    #[must_use]
    pub fn system(mut self, ids: &[i16]) -> Self {
        self.0.i16s(0x06, 4, ids);
        self
    }

    /// Sets [`Character::govt`] (offset 0x0E, 4 x i16) from slot 0.
    #[must_use]
    pub fn govt(mut self, ids: &[i16]) -> Self {
        self.0.i16s(0x0E, 4, ids);
        self
    }

    /// Sets [`Character::status`] (offset 0x16, 4 x i16) from slot 0.
    #[must_use]
    pub fn status(mut self, values: &[i16]) -> Self {
        self.0.i16s(0x16, 4, values);
        self
    }

    /// Sets [`Character::intro_pict`] (offset 0x20, 4 x i16) from slot 0.
    #[must_use]
    pub fn intro_pict(mut self, ids: &[i16]) -> Self {
        self.0.i16s(0x20, 4, ids);
        self
    }

    /// Sets [`Character::pict_delay`] (offset 0x28, 4 x i16) from slot 0.
    #[must_use]
    pub fn pict_delay(mut self, delays: &[i16]) -> Self {
        self.0.i16s(0x28, 4, delays);
        self
    }

    /// Sets [`Character::on_start`] (offset 0x32, 256 bytes).
    #[must_use]
    pub fn on_start(mut self, script: &str) -> Self {
        self.0.text(0x32, 256, script);
        self
    }

    /// Sets [`Character::date_prefix`] (offset 0x13A, 16 bytes).
    #[must_use]
    pub fn date_prefix(mut self, text: &str) -> Self {
        self.0.text(0x13A, 16, text);
        self
    }

    /// Sets [`Character::date_suffix`] (offset 0x14A, 16 bytes).
    #[must_use]
    pub fn date_suffix(mut self, text: &str) -> Self {
        self.0.text(0x14A, 16, text);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::Record;
    use crate::testutil::Buf;
    use crate::wire::id::{GovtId, PictId, ShipId, SystemId};

    fn character(builder: CharacterBuilder) -> Character {
        Buf(builder.build()).decode()
    }

    #[test]
    fn a_new_char_is_zeroed_and_full_size() {
        assert_eq!(
            CharacterBuilder::new().build(),
            vec![0; Character::SIZE.expect("fixed")]
        );
        assert_eq!(CharacterBuilder::default(), CharacterBuilder::new());
    }

    /// A setter's name, the setter, and the decoded field it sets.
    type CharCase = (
        &'static str,
        fn(CharacterBuilder, i16) -> CharacterBuilder,
        fn(&Character) -> i16,
    );

    #[test]
    fn every_i16_char_setter_writes_its_field() {
        let cases: [CharCase; 6] = [
            ("ship_type", CharacterBuilder::ship_type, |c| {
                c.ship_type.map_or(0, |id| id.0)
            }),
            ("kills", CharacterBuilder::kills, |c| c.kills),
            ("intro_text_id", CharacterBuilder::intro_text_id, |c| {
                c.intro_text_id.map_or(0, |id| id.0)
            }),
            ("unknown_0x134", CharacterBuilder::unknown_0x134, |c| {
                c.unknown_0x134
            }),
            ("unknown_0x136", CharacterBuilder::unknown_0x136, |c| {
                c.unknown_0x136
            }),
            ("start_year", CharacterBuilder::start_year, |c| c.start_year),
        ];
        for (name, set, get) in cases {
            let decoded = character(set(CharacterBuilder::new(), 0x1234));
            assert_eq!(get(&decoded), 0x1234, "{name}");
        }
    }

    #[test]
    fn the_cash_setter_writes_all_four_bytes() {
        assert_eq!(
            character(CharacterBuilder::new().cash(-70_000)).cash,
            -70_000
        );
        assert_eq!(
            character(CharacterBuilder::new().cash(0x0123_4567)).cash,
            0x0123_4567
        );
    }

    #[test]
    fn slot_setters_fill_their_four_slots_from_slot_zero() {
        let decoded = character(
            CharacterBuilder::new()
                .system(&[136, -1, 138, 139])
                .govt(&[129, 130, -1, 132])
                .status(&[50, 99, -20, 7])
                .intro_pict(&[8000, 8001, 8002, -1])
                .pict_delay(&[10, 11, 12, 13]),
        );
        assert_eq!(
            decoded.system,
            [
                Some(SystemId(136)),
                None,
                Some(SystemId(138)),
                Some(SystemId(139))
            ]
        );
        assert_eq!(
            decoded.govt,
            [
                Some(GovtId(129)),
                Some(GovtId(130)),
                None,
                Some(GovtId(132))
            ]
        );
        assert_eq!(decoded.status, [50, 99, -20, 7]);
        assert_eq!(
            decoded.intro_pict,
            [
                Some(PictId(8000)),
                Some(PictId(8001)),
                Some(PictId(8002)),
                None
            ]
        );
        assert_eq!(decoded.pict_delay, [10, 11, 12, 13]);
    }

    #[test]
    fn a_short_slot_list_leaves_the_later_slots_alone() {
        let decoded = character(CharacterBuilder::new().system(&[-1; 4]).system(&[128]));
        assert_eq!(decoded.system, [Some(SystemId(128)), None, None, None]);
        let decoded = character(CharacterBuilder::new().ship_type(128).system(&[130]));
        assert_eq!(
            decoded.ship_type,
            Some(ShipId(128)),
            "the slots start past it"
        );
    }

    #[test]
    #[should_panic(expected = "4 slots")]
    fn more_than_four_slots_is_a_mistake() {
        let _ = CharacterBuilder::new().govt(&[1; 5]);
    }

    #[test]
    fn text_setters_write_their_strings() {
        let decoded = character(
            CharacterBuilder::new()
                .on_start("b1 b2")
                .date_prefix("Year ")
                .date_suffix(" NC"),
        );
        assert_eq!(decoded.on_start.as_str(), "b1 b2");
        assert_eq!(decoded.date_prefix.as_str(), "Year ");
        assert_eq!(decoded.date_suffix.as_str(), " NC");
    }

    #[test]
    fn from_bytes_changes_only_the_fields_it_sets() {
        let base = CharacterBuilder::new().cash(1000).kills(5).build();
        let changed = CharacterBuilder::from_bytes(base).kills(6).build();
        assert_eq!(changed, CharacterBuilder::new().cash(1000).kills(6).build());
    }

    #[test]
    #[should_panic(expected = "a record is")]
    fn from_bytes_wants_a_whole_char() {
        let _ = CharacterBuilder::from_bytes(vec![0; 3]);
    }
}
