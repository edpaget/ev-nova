//! The pilot editor: the pilot flying, shown, and the drafts of each edit
//! the panel turns into a [`PilotEdit`] for the [`PilotDesk`].
//!
//! Each field is a draft the panel types into ([`PilotEditor::draft_mut`])
//! and applies. A draft that does not parse is told on the status line,
//! and the desk is not asked; a refusal is told there too, and a change
//! re-reads the sheet. The systems are read once, the first time a desk is
//! seen; a system's stellars when it is selected. The editor holds every
//! string the panel shows.

use nova_sim::catalog::{StellarId, SystemId};
use nova_sim::{Bit, GameDate, Reserve};

use super::pilot::{PilotDesk, PilotEdit, PilotSheet, Place};
use super::search::fold;

/// What the panel shows when no pilot is flying.
pub const NO_PILOT: &str = "No pilot flying";

/// How the panel names control bit `bit`, e.g. "Bit 42".
#[must_use]
pub fn bit_label(bit: Bit) -> String {
    format!("Bit {}", bit.get())
}

/// One of the editor's drafts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Draft {
    /// The cash.
    Credits,
    /// A reserve.
    Reserve(Reserve),
    /// The date, `YYYY-MM-DD`.
    Date,
    /// The filter over the systems, by name or ID.
    SystemFilter,
    /// The control bit looked up, by number.
    BitQuery,
}

/// The pilot editor (see the module docs).
#[derive(Debug, Default)]
pub struct PilotEditor {
    sheet: Option<PilotSheet>,
    /// Read once, the first time a desk is seen.
    systems: Option<Vec<Place<SystemId>>>,
    /// The selected system and its stellars.
    system: Option<SystemId>,
    stellars: Vec<Place<StellarId>>,
    stellar: Option<StellarId>,
    credits: String,
    shield: String,
    armor: String,
    fuel: String,
    date: String,
    system_filter: String,
    bit_query: String,
    status: String,
}

/// The name the panel gives `reserve`.
fn reserve_name(reserve: Reserve) -> &'static str {
    match reserve {
        Reserve::Shield => "Shield",
        Reserve::Armor => "Armour",
        Reserve::Fuel => "Fuel",
    }
}

/// `date` as the date draft writes it, `YYYY-MM-DD`.
fn date_draft(date: GameDate) -> String {
    format!("{:04}-{:02}-{:02}", date.year(), date.month(), date.day())
}

/// The date a draft writes, or why it writes none.
fn parse_date(text: &str) -> Result<GameDate, String> {
    let text = text.trim();
    let parts: Vec<&str> = text.split('-').collect();
    let [year, month, day] = parts[..] else {
        return Err(DATE_FORMAT.to_owned());
    };
    let (Ok(year), Ok(month), Ok(day)) = (year.parse(), month.parse(), day.parse()) else {
        return Err(DATE_FORMAT.to_owned());
    };
    GameDate::new(year, month, day).ok_or_else(|| format!("No such date: {text}"))
}

/// The name of the place `id` among `places`; none when it is not there.
fn name_of<Id: PartialEq + Copy>(places: &[Place<Id>], id: Id) -> String {
    places
        .iter()
        .find(|place| place.id == id)
        .map(|place| place.name.clone())
        .unwrap_or_default()
}

/// How a date draft is written.
const DATE_FORMAT: &str = "Dates are written YYYY-MM-DD";

impl PilotEditor {
    /// An editor with nothing read yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Reads the pilot flying from `desk`, and its systems the first time.
    /// The drafts start from the pilot's values when one is first seen.
    pub fn refresh(&mut self, desk: &dyn PilotDesk) {
        if self.systems.is_none() {
            self.systems = Some(desk.systems());
        }
        let sheet = desk.sheet();
        if let (None, Some(sheet)) = (&self.sheet, &sheet) {
            self.credits = sheet.credits.to_string();
            for reserve in [Reserve::Shield, Reserve::Armor, Reserve::Fuel] {
                *self.draft_mut(Draft::Reserve(reserve)) =
                    format!("{:.0}", sheet.gauge(reserve).now);
            }
            self.date = date_draft(sheet.date);
        }
        self.sheet = sheet;
    }

    /// Forgets the pilot: none is flying.
    pub fn forget(&mut self) {
        self.sheet = None;
    }

    /// The pilot flying, as last read.
    #[must_use]
    pub fn sheet(&self) -> Option<&PilotSheet> {
        self.sheet.as_ref()
    }

    /// `draft`, to type into.
    pub fn draft_mut(&mut self, draft: Draft) -> &mut String {
        match draft {
            Draft::Credits => &mut self.credits,
            Draft::Reserve(Reserve::Shield) => &mut self.shield,
            Draft::Reserve(Reserve::Armor) => &mut self.armor,
            Draft::Reserve(Reserve::Fuel) => &mut self.fuel,
            Draft::Date => &mut self.date,
            Draft::SystemFilter => &mut self.system_filter,
            Draft::BitQuery => &mut self.bit_query,
        }
    }

    /// A line about the pilot flying, or none when there is none.
    fn line(&self, write: impl FnOnce(&PilotSheet) -> String) -> String {
        self.sheet.as_ref().map(write).unwrap_or_default()
    }

    /// The cash, e.g. "Credits 12345".
    #[must_use]
    pub fn credits_line(&self) -> String {
        self.line(|sheet| format!("Credits {}", sheet.credits))
    }

    /// `reserve` out of its most, e.g. "Shield 120 / 300".
    #[must_use]
    pub fn reserve_line(&self, reserve: Reserve) -> String {
        self.line(|sheet| {
            let gauge = sheet.gauge(reserve);
            format!(
                "{} {:.0} / {:.0}",
                reserve_name(reserve),
                gauge.now,
                gauge.max
            )
        })
    }

    /// The date as the game writes it.
    #[must_use]
    pub fn date_line(&self) -> String {
        self.line(|sheet| sheet.date_text.clone())
    }

    /// Where the pilot is: "Sol: Earth, landed" or "Sol, in flight".
    #[must_use]
    pub fn location_line(&self) -> String {
        self.line(|sheet| match &sheet.landed {
            Some(stellar) => format!("{}: {}, landed", sheet.system.name, stellar.name),
            None => format!("{}, in flight", sheet.system.name),
        })
    }

    /// The bits set: "Bits set: 7, 42, 9999" or "No bits set".
    #[must_use]
    pub fn bits_line(&self) -> String {
        self.line(|sheet| {
            if sheet.bits.is_empty() {
                "No bits set".to_owned()
            } else {
                let numbers: Vec<String> =
                    sheet.bits.iter().map(|bit| bit.get().to_string()).collect();
                format!("Bits set: {}", numbers.join(", "))
            }
        })
    }

    /// What the last command did, or why it did nothing.
    #[must_use]
    pub fn status(&self) -> &str {
        &self.status
    }

    /// The systems whose name or ID the filter matches, in ID order.
    #[must_use]
    pub fn filtered_systems(&self) -> Vec<&Place<SystemId>> {
        let filter = self.system_filter.trim();
        let folded = fold(filter);
        self.systems
            .iter()
            .flatten()
            .filter(|place| place.id.0.to_string() == filter || fold(&place.name).contains(&folded))
            .collect()
    }

    /// The selected system's stellars.
    #[must_use]
    pub fn stellars(&self) -> &[Place<StellarId>] {
        &self.stellars
    }

    /// The selected system.
    #[must_use]
    pub fn selected_system(&self) -> Option<SystemId> {
        self.system
    }

    /// The selected stellar.
    #[must_use]
    pub fn selected_stellar(&self) -> Option<StellarId> {
        self.stellar
    }

    /// Selects `system`, reading its stellars from `desk`; the stellar
    /// selected is let go.
    pub fn select_system(&mut self, desk: &dyn PilotDesk, system: SystemId) {
        self.system = Some(system);
        self.stellars = desk.stellars(system);
        self.stellar = None;
    }

    /// Selects `stellar`, one of the selected system's.
    pub fn select_stellar(&mut self, stellar: StellarId) {
        self.stellar = Some(stellar);
    }

    /// Sends `edit` to `desk` and tells how it went: `done` on the status
    /// line and the sheet re-read when it was carried out, the refusal
    /// otherwise. Returns whether the pilot changed.
    fn send(&mut self, desk: &mut dyn PilotDesk, edit: PilotEdit, done: String) -> bool {
        match desk.edit(edit) {
            Ok(()) => {
                self.status = done;
                self.sheet = desk.sheet();
                true
            }
            Err(refusal) => {
                self.status = format!("Refused: {refusal}");
                false
            }
        }
    }

    /// Tells `problem` on the status line; the pilot did not change.
    fn tell(&mut self, problem: String) -> bool {
        self.status = problem;
        false
    }

    /// Sets the cash to the draft; returns whether the pilot changed.
    pub fn apply_credits(&mut self, desk: &mut dyn PilotDesk) -> bool {
        match self.credits.trim().parse() {
            Ok(credits) => self.send(
                desk,
                PilotEdit::Credits(credits),
                format!("Credits set to {credits}"),
            ),
            Err(_) => self.tell("Credits must be a whole number".to_owned()),
        }
    }

    /// Sets `reserve` to its draft; returns whether the pilot changed.
    pub fn apply_reserve(&mut self, desk: &mut dyn PilotDesk, reserve: Reserve) -> bool {
        let name = reserve_name(reserve);
        match self.draft_mut(Draft::Reserve(reserve)).trim().parse() {
            Ok(amount) => self.send(
                desk,
                PilotEdit::Reserve(reserve, amount),
                format!("{name} set"),
            ),
            Err(_) => self.tell(format!("{name} must be a number")),
        }
    }

    /// Sets the date to the draft; returns whether the pilot changed.
    pub fn apply_date(&mut self, desk: &mut dyn PilotDesk) -> bool {
        match parse_date(&self.date) {
            Ok(date) => self.send(desk, PilotEdit::Date(date), "Date set".to_owned()),
            Err(problem) => self.tell(problem),
        }
    }

    /// Moves the pilot to the selected stellar; returns whether the pilot
    /// changed.
    pub fn move_to(&mut self, desk: &mut dyn PilotDesk) -> bool {
        let (Some(system), Some(stellar)) = (self.system, self.stellar) else {
            return self.tell("Pick a stellar first".to_owned());
        };
        let system_name = name_of(self.systems.as_deref().unwrap_or_default(), system);
        let done = format!(
            "Moved to {system_name}: {}",
            name_of(&self.stellars, stellar)
        );
        self.send(desk, PilotEdit::MoveTo { system, stellar }, done)
    }

    /// The bit the query names, and whether it is set; otherwise why the
    /// query names none.
    ///
    /// # Errors
    ///
    /// "Bits run 0-9999" when the query is no bit number.
    pub fn bit_lookup(&self) -> Result<(Bit, bool), String> {
        let bit = self
            .bit_query
            .trim()
            .parse()
            .ok()
            .and_then(Bit::new)
            .ok_or_else(|| format!("Bits run 0-{}", Bit::MAX))?;
        Ok((bit, self.is_set(bit)))
    }

    /// Whether `bit` is set on the pilot flying; not when there is none.
    fn is_set(&self, bit: Bit) -> bool {
        self.sheet
            .as_ref()
            .is_some_and(|sheet| sheet.bits.contains(&bit))
    }

    /// Sets `bit` when it is clear, and clears it when it is set; returns
    /// whether the pilot changed.
    pub fn toggle_bit(&mut self, desk: &mut dyn PilotDesk, bit: Bit) -> bool {
        let on = !self.is_set(bit);
        let done = format!("{} {}", bit_label(bit), if on { "set" } else { "cleared" });
        self.send(desk, PilotEdit::Bit(bit, on), done)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use nova_sim::Gauge;

    use super::*;
    use crate::devtools::pilot::EditRefusal;

    fn bit(n: u16) -> Bit {
        Bit::new(n).expect("in range")
    }

    fn place<Id>(id: Id, name: &str) -> Place<Id> {
        Place {
            id,
            name: name.to_owned(),
        }
    }

    fn sheet() -> PilotSheet {
        PilotSheet {
            name: "Ada".to_owned(),
            credits: 12_345,
            shield: Gauge {
                now: 120.4,
                max: 300.0,
            },
            armor: Gauge::full(45.0),
            fuel: Gauge {
                now: 99.6,
                max: 300.0,
            },
            date: GameDate::new(1177, 6, 23).expect("a date"),
            date_text: "June 23, 1177 NC".to_owned(),
            system: place(SystemId(130), "Sol"),
            landed: Some(place(StellarId(128), "Earth")),
            bits: vec![bit(7), bit(42), bit(9999)],
        }
    }

    /// Answers with a canned sheet, systems and stellars, refuses every
    /// edit with `refusal` when there is one, and records each edit and
    /// how many times the systems were read. A credits edit carried out
    /// changes the sheet's credits, so a re-read shows it.
    struct MockDesk {
        sheet: Option<PilotSheet>,
        refusal: Option<EditRefusal>,
        edits: Vec<PilotEdit>,
        system_reads: Cell<usize>,
    }

    impl MockDesk {
        fn new() -> Self {
            Self {
                sheet: Some(sheet()),
                refusal: None,
                edits: Vec::new(),
                system_reads: Cell::new(0),
            }
        }

        fn refusing(refusal: EditRefusal) -> Self {
            Self {
                refusal: Some(refusal),
                ..Self::new()
            }
        }
    }

    impl PilotDesk for MockDesk {
        fn sheet(&self) -> Option<PilotSheet> {
            self.sheet.clone()
        }

        fn systems(&self) -> Vec<Place<SystemId>> {
            self.system_reads.set(self.system_reads.get() + 1);
            vec![
                place(SystemId(128), "Alpha Centauri"),
                place(SystemId(130), "Sol"),
                place(SystemId(300), "Fréd"),
            ]
        }

        fn stellars(&self, system: SystemId) -> Vec<Place<StellarId>> {
            match system.0 {
                130 => vec![
                    place(StellarId(128), "Earth"),
                    place(StellarId(129), "Mars"),
                ],
                128 => vec![place(StellarId(140), "Proxima")],
                _ => Vec::new(),
            }
        }

        fn edit(&mut self, edit: PilotEdit) -> Result<(), EditRefusal> {
            self.edits.push(edit);
            if let Some(refusal) = self.refusal {
                return Err(refusal);
            }
            if let (PilotEdit::Credits(credits), Some(sheet)) = (edit, self.sheet.as_mut()) {
                sheet.credits = credits;
            }
            Ok(())
        }
    }

    fn editor(desk: &MockDesk) -> PilotEditor {
        let mut editor = PilotEditor::new();
        editor.refresh(desk);
        editor
    }

    fn typed(editor: &mut PilotEditor, draft: Draft, text: &str) {
        text.clone_into(editor.draft_mut(draft));
    }

    #[test]
    fn with_no_pilot_there_is_no_sheet() {
        let editor = PilotEditor::new();
        assert_eq!(editor.sheet(), None);
        let desk = MockDesk {
            sheet: None,
            ..MockDesk::new()
        };
        assert_eq!(self::editor(&desk).sheet(), None);
        let mut editor = self::editor(&MockDesk::new());
        assert_eq!(editor.sheet(), Some(&sheet()));
        editor.forget();
        assert_eq!(editor.sheet(), None);
        assert_eq!(NO_PILOT, "No pilot flying");
    }

    #[test]
    fn the_lines_show_the_sheet() {
        let editor = editor(&MockDesk::new());
        assert_eq!(editor.credits_line(), "Credits 12345");
        assert_eq!(editor.reserve_line(Reserve::Shield), "Shield 120 / 300");
        assert_eq!(editor.reserve_line(Reserve::Armor), "Armour 45 / 45");
        assert_eq!(editor.reserve_line(Reserve::Fuel), "Fuel 100 / 300");
        assert_eq!(editor.date_line(), "June 23, 1177 NC");
        assert_eq!(editor.location_line(), "Sol: Earth, landed");
        assert_eq!(editor.bits_line(), "Bits set: 7, 42, 9999");
        assert_eq!(editor.status(), "");
    }

    #[test]
    fn in_flight_and_with_no_bits_the_lines_say_so() {
        let desk = MockDesk {
            sheet: Some(PilotSheet {
                landed: None,
                bits: Vec::new(),
                ..sheet()
            }),
            ..MockDesk::new()
        };
        let editor = editor(&desk);
        assert_eq!(editor.location_line(), "Sol, in flight");
        assert_eq!(editor.bits_line(), "No bits set");
    }

    #[test]
    fn with_no_pilot_the_lines_are_empty() {
        let editor = PilotEditor::new();
        assert_eq!(editor.credits_line(), "");
        assert_eq!(editor.reserve_line(Reserve::Fuel), "");
        assert_eq!(editor.date_line(), "");
        assert_eq!(editor.location_line(), "");
        assert_eq!(editor.bits_line(), "");
    }

    #[test]
    fn the_drafts_start_from_the_first_pilot_seen_and_are_kept_after() {
        let mut desk = MockDesk::new();
        let mut editor = editor(&desk);
        assert_eq!(editor.draft_mut(Draft::Credits), "12345");
        assert_eq!(editor.draft_mut(Draft::Reserve(Reserve::Shield)), "120");
        assert_eq!(editor.draft_mut(Draft::Reserve(Reserve::Armor)), "45");
        assert_eq!(editor.draft_mut(Draft::Reserve(Reserve::Fuel)), "100");
        assert_eq!(editor.draft_mut(Draft::Date), "1177-06-23");
        assert_eq!(editor.draft_mut(Draft::SystemFilter), "");
        assert_eq!(editor.draft_mut(Draft::BitQuery), "");
        typed(&mut editor, Draft::Credits, "5");
        desk.sheet.as_mut().expect("a pilot").credits = 77;
        editor.refresh(&desk);
        assert_eq!(editor.draft_mut(Draft::Credits), "5", "kept");
        assert_eq!(editor.credits_line(), "Credits 77");
        editor.forget();
        editor.refresh(&desk);
        assert_eq!(editor.draft_mut(Draft::Credits), "77", "a pilot seen anew");
    }

    #[test]
    fn each_draft_is_its_own() {
        let mut editor = PilotEditor::new();
        let drafts = [
            Draft::Credits,
            Draft::Reserve(Reserve::Shield),
            Draft::Reserve(Reserve::Armor),
            Draft::Reserve(Reserve::Fuel),
            Draft::Date,
            Draft::SystemFilter,
            Draft::BitQuery,
        ];
        for (i, &draft) in drafts.iter().enumerate() {
            typed(&mut editor, draft, &i.to_string());
        }
        for (i, &draft) in drafts.iter().enumerate() {
            assert_eq!(*editor.draft_mut(draft), i.to_string(), "{draft:?}");
        }
    }

    #[test]
    fn applying_credits_sends_them_and_rereads_the_sheet() {
        let mut desk = MockDesk::new();
        let mut editor = editor(&desk);
        typed(&mut editor, Draft::Credits, " 500 ");
        assert!(editor.apply_credits(&mut desk));
        assert_eq!(desk.edits, [PilotEdit::Credits(500)]);
        assert_eq!(editor.credits_line(), "Credits 500");
        assert_eq!(editor.status(), "Credits set to 500");
    }

    #[test]
    fn credits_that_are_no_whole_number_send_nothing() {
        let mut desk = MockDesk::new();
        let mut editor = editor(&desk);
        for text in ["", "1.5", "lots"] {
            typed(&mut editor, Draft::Credits, text);
            assert!(!editor.apply_credits(&mut desk), "{text}");
            assert_eq!(editor.status(), "Credits must be a whole number");
        }
        assert_eq!(desk.edits, []);
    }

    #[test]
    fn applying_a_reserve_sends_it() {
        let mut desk = MockDesk::new();
        let mut editor = editor(&desk);
        for (reserve, name) in [
            (Reserve::Shield, "Shield"),
            (Reserve::Armor, "Armour"),
            (Reserve::Fuel, "Fuel"),
        ] {
            typed(&mut editor, Draft::Reserve(reserve), "12.5");
            assert!(editor.apply_reserve(&mut desk, reserve));
            assert_eq!(desk.edits.last(), Some(&PilotEdit::Reserve(reserve, 12.5)));
            assert_eq!(editor.status(), format!("{name} set"));
            typed(&mut editor, Draft::Reserve(reserve), "full");
            assert!(!editor.apply_reserve(&mut desk, reserve));
            assert_eq!(editor.status(), format!("{name} must be a number"));
        }
        assert_eq!(desk.edits.len(), 3);
    }

    #[test]
    fn applying_a_date_sends_it() {
        let mut desk = MockDesk::new();
        let mut editor = editor(&desk);
        typed(&mut editor, Draft::Date, "1180-02-29");
        assert!(editor.apply_date(&mut desk));
        assert_eq!(
            desk.edits,
            [PilotEdit::Date(GameDate::new(1180, 2, 29).expect("a date"))]
        );
        assert_eq!(editor.status(), "Date set");
    }

    #[test]
    fn a_date_that_is_no_date_sends_nothing() {
        let mut desk = MockDesk::new();
        let mut editor = editor(&desk);
        typed(&mut editor, Draft::Date, "1177-02-30");
        assert!(!editor.apply_date(&mut desk));
        assert_eq!(editor.status(), "No such date: 1177-02-30");
        for text in [
            "",
            "1177-02",
            "1177-02-03-04",
            "June 3",
            "1177-x-03",
            "1177-02-300",
        ] {
            typed(&mut editor, Draft::Date, text);
            assert!(!editor.apply_date(&mut desk), "{text}");
            assert_eq!(editor.status(), "Dates are written YYYY-MM-DD", "{text}");
        }
        assert_eq!(desk.edits, []);
    }

    #[test]
    fn the_systems_are_read_once_and_filtered_by_name_or_id() {
        let desk = MockDesk::new();
        let mut editor = editor(&desk);
        editor.refresh(&desk);
        editor.refresh(&desk);
        assert_eq!(desk.system_reads.get(), 1);
        let names = |editor: &PilotEditor| {
            editor
                .filtered_systems()
                .into_iter()
                .map(|place| place.name.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(names(&editor), ["Alpha Centauri", "Sol", "Fréd"]);
        typed(&mut editor, Draft::SystemFilter, "SOL");
        assert_eq!(names(&editor), ["Sol"]);
        typed(&mut editor, Draft::SystemFilter, "fred");
        assert_eq!(names(&editor), ["Fréd"], "accents folded");
        typed(&mut editor, Draft::SystemFilter, " 128 ");
        assert_eq!(names(&editor), ["Alpha Centauri"], "by ID");
        typed(&mut editor, Draft::SystemFilter, "12");
        assert_eq!(names(&editor), [] as [String; 0], "an ID matches whole");
        assert!(PilotEditor::new().filtered_systems().is_empty());
    }

    #[test]
    fn selecting_a_system_reads_its_stellars_and_lets_the_stellar_go() {
        let desk = MockDesk::new();
        let mut editor = editor(&desk);
        assert_eq!(editor.selected_system(), None);
        editor.select_system(&desk, SystemId(130));
        assert_eq!(editor.selected_system(), Some(SystemId(130)));
        assert_eq!(
            editor.stellars(),
            [
                place(StellarId(128), "Earth"),
                place(StellarId(129), "Mars")
            ]
        );
        editor.select_stellar(StellarId(129));
        assert_eq!(editor.selected_stellar(), Some(StellarId(129)));
        editor.select_system(&desk, SystemId(128));
        assert_eq!(editor.selected_stellar(), None);
        assert_eq!(editor.stellars(), [place(StellarId(140), "Proxima")]);
    }

    #[test]
    fn moving_sends_the_selected_stellar() {
        let mut desk = MockDesk::new();
        let mut editor = editor(&desk);
        assert!(!editor.move_to(&mut desk));
        assert_eq!(editor.status(), "Pick a stellar first");
        editor.select_system(&desk, SystemId(128));
        assert!(!editor.move_to(&mut desk));
        assert_eq!(editor.status(), "Pick a stellar first");
        assert_eq!(desk.edits, []);
        editor.select_stellar(StellarId(140));
        assert!(editor.move_to(&mut desk));
        assert_eq!(
            desk.edits,
            [PilotEdit::MoveTo {
                system: SystemId(128),
                stellar: StellarId(140)
            }]
        );
        assert_eq!(editor.status(), "Moved to Alpha Centauri: Proxima");
    }

    #[test]
    fn a_refusal_is_told_and_the_pilot_unchanged() {
        let mut desk = MockDesk::refusing(EditRefusal::InFlight);
        let mut editor = editor(&desk);
        editor.select_system(&desk, SystemId(128));
        editor.select_stellar(StellarId(140));
        assert!(!editor.move_to(&mut desk));
        assert_eq!(editor.status(), "Refused: the ship must be landed to move");
        typed(&mut editor, Draft::Credits, "5");
        assert!(!editor.apply_credits(&mut desk));
        assert_eq!(editor.credits_line(), "Credits 12345");
        assert_eq!(desk.edits.len(), 2);
    }

    #[test]
    fn a_success_rereads_the_sheet_before_the_next_frame() {
        let mut desk = MockDesk::new();
        let mut editor = editor(&desk);
        desk.sheet.as_mut().expect("a pilot").date_text = "later".to_owned();
        typed(&mut editor, Draft::Reserve(Reserve::Fuel), "1");
        editor.apply_reserve(&mut desk, Reserve::Fuel);
        assert_eq!(editor.date_line(), "later");
    }

    #[test]
    fn the_bit_query_names_a_bit_and_whether_it_is_set() {
        let mut editor = editor(&MockDesk::new());
        typed(&mut editor, Draft::BitQuery, "42");
        assert_eq!(editor.bit_lookup(), Ok((bit(42), true)));
        typed(&mut editor, Draft::BitQuery, " 43 ");
        assert_eq!(editor.bit_lookup(), Ok((bit(43), false)));
        typed(&mut editor, Draft::BitQuery, "9999");
        assert_eq!(editor.bit_lookup(), Ok((bit(9999), true)));
        for text in ["", "10000", "-1", "b42", "65536"] {
            typed(&mut editor, Draft::BitQuery, text);
            assert_eq!(
                editor.bit_lookup(),
                Err("Bits run 0-9999".to_owned()),
                "{text}"
            );
        }
        let mut editor = PilotEditor::new();
        typed(&mut editor, Draft::BitQuery, "42");
        assert_eq!(editor.bit_lookup(), Ok((bit(42), false)), "no pilot");
    }

    #[test]
    fn a_bit_is_labelled_by_its_number() {
        assert_eq!(bit_label(bit(0)), "Bit 0");
        assert_eq!(bit_label(bit(9999)), "Bit 9999");
    }

    #[test]
    fn toggling_a_bit_sends_its_other_state() {
        let mut desk = MockDesk::new();
        let mut editor = editor(&desk);
        assert!(editor.toggle_bit(&mut desk, bit(42)));
        assert_eq!(editor.status(), "Bit 42 cleared");
        assert!(editor.toggle_bit(&mut desk, bit(43)));
        assert_eq!(editor.status(), "Bit 43 set");
        assert_eq!(
            desk.edits,
            [
                PilotEdit::Bit(bit(42), false),
                PilotEdit::Bit(bit(43), true)
            ]
        );
    }
}
