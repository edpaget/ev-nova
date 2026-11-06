//! The dialog port's contract, played over the contract story.
//!
//! [`check_contract`] states what every [`DialogLoader`] and the
//! [`Dialog`]s it opens must do, over [`CONTRACT_JSON`], the compiled
//! [`CONTRACT_INK`]. It enters every knot of that story and takes every
//! choice in it. It runs against this crate's fake
//! ([`FakeLoader::meeting_the_contract`](super::fake::FakeLoader::meeting_the_contract))
//! here, and against `nova-ink`'s `nova_ink::InkLoader` there.
//!
//! # Regenerating the compiled story
//!
//! `contract.ink.json` is compiled by hand with Inky 0.15.2's `inklecate`
//! (ink 1.2, `inkVersion` 21), so no test needs `inklecate`. After editing
//! `contract.ink`, run from the workspace root:
//!
//! ```sh
//! "$INKY_INKLECATE" -o crates/nova-sim-story/src/dialog/fixture/contract.ink.json \
//!     crates/nova-sim-story/src/dialog/fixture/contract.ink
//! ```
//!
//! where `$INKY_INKLECATE` is Inky's bundled compiler (on macOS,
//! `/Applications/Inky.app/Contents/Resources/app.asar.unpacked/main-process/ink/inklecate_mac`),
//! then update the fake's script in
//! [`FakeLoader::meeting_the_contract`](super::fake::FakeLoader::meeting_the_contract)
//! to match. A JSON that has fallen behind its source fails the adapter's
//! contract run.

use super::{Choice, Dialog, DialogError, DialogLoader, Line, SavedDialog, Value};

/// The contract story's Ink source.
pub const CONTRACT_INK: &str = include_str!("fixture/contract.ink");

/// The contract story, compiled by `inklecate`.
pub const CONTRACT_JSON: &str = include_str!("fixture/contract.ink.json");

/// Checks the port's contract against `loader`, over [`CONTRACT_JSON`].
/// Each clause opens a fresh dialog:
///
/// 1. Text that isn't a story doesn't open.
/// 2. Nothing steps before a knot is entered.
/// 3. An unknown knot can't be entered.
/// 4. `greet` steps its lines with their tags, then offers its choices
///    with theirs; a choice past them is refused.
/// 5. Taking `Ask about work` steps its line, sets `coins`, and ends;
///    entering `greet` again offers only the sticky `Leave`, which steps
///    `Goodbye.` and ends.
/// 6. `ledger.open` steps its line and offers `Close it`, which ends.
/// 7. Variables read their declared values, an unknown one reads `None`,
///    a set reads back, and an undeclared one can't be set.
/// 8. A save puts back the variables, the spent choices and the choices
///    pending at the save, in the same dialog or a fresh one, whichever
///    knot it was in, and before any knot was entered; junk doesn't
///    restore.
/// 9. Entering a knot drops the choices that were pending.
/// 10. A failed enter changes nothing: the choices pending before it are
///     still offered and can still be taken.
///
/// # Errors
///
/// A message naming the first clause the loader breaks.
pub fn check_contract(loader: &impl DialogLoader) -> Result<(), String> {
    junk_is_not_a_story(loader)?;
    nothing_steps_before_a_knot_is_entered(loader)?;
    an_unknown_knot_cannot_be_entered(loader)?;
    greet_steps_its_lines_and_offers_its_choices(loader)?;
    a_choice_past_the_end_is_refused(loader)?;
    asking_about_work_pays_and_spends_the_choice(loader)?;
    the_ledger_stitch_is_entered_by_its_path(loader)?;
    variables_read_and_write(loader)?;
    a_save_restores_variables_spent_choices_and_position(loader)?;
    a_save_keeps_spent_choices_in_another_knot(loader)?;
    a_save_before_entering_restores_to_nothing(loader)?;
    entering_a_knot_drops_pending_choices(loader)?;
    a_failed_enter_keeps_pending_choices(loader)?;
    Ok(())
}

fn open(loader: &impl DialogLoader) -> Result<Box<dyn Dialog>, String> {
    loader
        .open(CONTRACT_JSON)
        .map_err(|e| format!("the contract story doesn't open: {e}"))
}

fn line(text: &str, tags: &[&str]) -> Line {
    Line {
        text: text.to_owned(),
        tags: tags.iter().map(|&t| t.to_owned()).collect(),
    }
}

fn choice(text: &str, tags: &[&str]) -> Choice {
    Choice {
        text: text.to_owned(),
        tags: tags.iter().map(|&t| t.to_owned()).collect(),
    }
}

/// Enters `knot`, naming the clause on failure.
fn enter(dialog: &mut dyn Dialog, knot: &str, clause: &str) -> Result<(), String> {
    dialog
        .enter(knot)
        .map_err(|e| format!("{clause}: entering `{knot}` failed: {e}"))
}

/// Takes the choice at `index`, naming the clause on failure.
fn choose(dialog: &mut dyn Dialog, index: usize, clause: &str) -> Result<(), String> {
    dialog
        .choose(index)
        .map_err(|e| format!("{clause}: choice {index} failed: {e}"))
}

/// Steps until the dialog stops, and checks the lines it gave.
fn expect_lines(dialog: &mut dyn Dialog, expected: &[Line], clause: &str) -> Result<(), String> {
    let mut lines = Vec::new();
    // Bounded, so a dialog that never stops fails the clause rather than
    // hanging it.
    for _ in 0..=expected.len() {
        match dialog.step() {
            Ok(Some(line)) => lines.push(line),
            Ok(None) => break,
            Err(e) => return Err(format!("{clause}: stepping failed: {e}")),
        }
    }
    if lines == expected {
        Ok(())
    } else {
        Err(format!("{clause}: stepped {lines:?}, not {expected:?}"))
    }
}

fn expect_choices(dialog: &dyn Dialog, expected: &[Choice], clause: &str) -> Result<(), String> {
    let choices = dialog.choices();
    if choices == expected {
        Ok(())
    } else {
        Err(format!("{clause}: offered {choices:?}, not {expected:?}"))
    }
}

fn expect_variable(
    dialog: &dyn Dialog,
    name: &str,
    expected: Option<&Value>,
    clause: &str,
) -> Result<(), String> {
    match dialog.variable(name) {
        Ok(value) if value.as_ref() == expected => Ok(()),
        other => Err(format!(
            "{clause}: `{name}` read {other:?}, not {expected:?}"
        )),
    }
}

fn greet_lines() -> [Line; 2] {
    [
        line("Hello.", &["speaker: clerk"]),
        line("How can I help?", &[]),
    ]
}

fn greet_choices() -> [Choice; 2] {
    [
        choice("Ask about work", &["topic: work"]),
        choice("Leave", &[]),
    ]
}

fn junk_is_not_a_story(loader: &impl DialogLoader) -> Result<(), String> {
    match loader.open("not a story") {
        Err(DialogError::BadStory(_)) => Ok(()),
        Err(e) => Err(format!("junk: opened with {e:?}, not BadStory")),
        Ok(_) => Err("junk: opened as a story".to_owned()),
    }
}

fn nothing_steps_before_a_knot_is_entered(loader: &impl DialogLoader) -> Result<(), String> {
    let clause = "before entering";
    let mut dialog = open(loader)?;
    expect_lines(dialog.as_mut(), &[], clause)?;
    expect_choices(dialog.as_ref(), &[], clause)
}

fn an_unknown_knot_cannot_be_entered(loader: &impl DialogLoader) -> Result<(), String> {
    let mut dialog = open(loader)?;
    match dialog.enter("nowhere") {
        Err(DialogError::NoSuchKnot(knot)) if knot == "nowhere" => Ok(()),
        other => Err(format!(
            "unknown knot: entering `nowhere` gave {other:?}, not NoSuchKnot"
        )),
    }
}

fn greet_steps_its_lines_and_offers_its_choices(loader: &impl DialogLoader) -> Result<(), String> {
    let clause = "greet";
    let mut dialog = open(loader)?;
    enter(dialog.as_mut(), "greet", clause)?;
    expect_choices(dialog.as_ref(), &[], "greet, before stepping")?;
    expect_lines(dialog.as_mut(), &greet_lines(), clause)?;
    expect_choices(dialog.as_ref(), &greet_choices(), clause)
}

fn a_choice_past_the_end_is_refused(loader: &impl DialogLoader) -> Result<(), String> {
    let clause = "choice past the end";
    let mut dialog = open(loader)?;
    enter(dialog.as_mut(), "greet", clause)?;
    expect_lines(dialog.as_mut(), &greet_lines(), clause)?;
    match dialog.choose(5) {
        Err(DialogError::NoSuchChoice { index: 5, count: 2 }) => {}
        other => {
            return Err(format!(
                "{clause}: choice 5 gave {other:?}, not NoSuchChoice {{ index: 5, count: 2 }}"
            ));
        }
    }
    match dialog.choose(2) {
        Err(DialogError::NoSuchChoice { index: 2, count: 2 }) => {}
        other => {
            return Err(format!(
                "{clause}: choice 2 gave {other:?}, not NoSuchChoice {{ index: 2, count: 2 }}"
            ));
        }
    }
    expect_choices(dialog.as_ref(), &greet_choices(), clause)
}

fn asking_about_work_pays_and_spends_the_choice(loader: &impl DialogLoader) -> Result<(), String> {
    let clause = "ask about work";
    let mut dialog = open(loader)?;
    enter(dialog.as_mut(), "greet", clause)?;
    expect_lines(dialog.as_mut(), &greet_lines(), clause)?;
    choose(dialog.as_mut(), 0, clause)?;
    expect_lines(
        dialog.as_mut(),
        &[line("There's a run to Marrow.", &["outcome: accept"])],
        clause,
    )?;
    expect_choices(dialog.as_ref(), &[], clause)?;
    expect_variable(dialog.as_ref(), "coins", Some(&Value::Int(5)), clause)?;

    let clause = "greet again";
    enter(dialog.as_mut(), "greet", clause)?;
    expect_lines(dialog.as_mut(), &greet_lines(), clause)?;
    expect_choices(dialog.as_ref(), &[choice("Leave", &[])], clause)?;
    choose(dialog.as_mut(), 0, clause)?;
    expect_lines(
        dialog.as_mut(),
        &[line("Goodbye.", &["outcome: decline"])],
        clause,
    )?;
    expect_choices(dialog.as_ref(), &[], clause)?;
    expect_variable(dialog.as_ref(), "coins", Some(&Value::Int(5)), clause)
}

fn the_ledger_stitch_is_entered_by_its_path(loader: &impl DialogLoader) -> Result<(), String> {
    let clause = "ledger.open";
    let mut dialog = open(loader)?;
    enter(dialog.as_mut(), "ledger.open", clause)?;
    expect_lines(dialog.as_mut(), &[line("The ledger is open.", &[])], clause)?;
    expect_choices(dialog.as_ref(), &[choice("Close it", &[])], clause)?;
    choose(dialog.as_mut(), 0, clause)?;
    expect_lines(dialog.as_mut(), &[], clause)?;
    expect_choices(dialog.as_ref(), &[], clause)
}

fn variables_read_and_write(loader: &impl DialogLoader) -> Result<(), String> {
    let clause = "variables";
    let mut dialog = open(loader)?;
    expect_variable(dialog.as_ref(), "met", Some(&Value::Bool(false)), clause)?;
    expect_variable(dialog.as_ref(), "coins", Some(&Value::Int(3)), clause)?;
    expect_variable(dialog.as_ref(), "rate", Some(&Value::Float(0.5)), clause)?;
    expect_variable(
        dialog.as_ref(),
        "name",
        Some(&Value::Text("Tally".to_owned())),
        clause,
    )?;
    expect_variable(dialog.as_ref(), "nope", None, clause)?;

    let sets = [
        ("met", Value::Bool(true)),
        ("coins", Value::Int(i32::MIN)),
        ("rate", Value::Float(1.25)),
        ("name", Value::Text("Ash".to_owned())),
    ];
    for (name, value) in sets {
        dialog
            .set_variable(name, value.clone())
            .map_err(|e| format!("{clause}: setting `{name}` failed: {e}"))?;
        expect_variable(dialog.as_ref(), name, Some(&value), clause)?;
    }
    match dialog.set_variable("nope", Value::Bool(true)) {
        Err(DialogError::Undeclared(name)) if name == "nope" => {}
        other => {
            return Err(format!(
                "{clause}: setting `nope` gave {other:?}, not Undeclared"
            ));
        }
    }
    expect_variable(dialog.as_ref(), "nope", None, clause)
}

fn a_save_restores_variables_spent_choices_and_position(
    loader: &impl DialogLoader,
) -> Result<(), String> {
    let clause = "save and restore";
    let saved_values = [
        ("met", Value::Bool(true)),
        ("rate", Value::Float(1.25)),
        ("name", Value::Text("Ash".to_owned())),
    ];
    let later_values = [
        ("met", Value::Bool(false)),
        ("rate", Value::Float(0.5)),
        ("name", Value::Text("Tally".to_owned())),
    ];
    let mut dialog = open(loader)?;
    set_all(dialog.as_mut(), &saved_values, clause)?;
    enter(dialog.as_mut(), "greet", clause)?;
    expect_lines(dialog.as_mut(), &greet_lines(), clause)?;
    let saved = dialog
        .save()
        .map_err(|e| format!("{clause}: saving failed: {e}"))?;

    choose(dialog.as_mut(), 0, clause)?;
    expect_lines(
        dialog.as_mut(),
        &[line("There's a run to Marrow.", &["outcome: accept"])],
        clause,
    )?;
    set_all(dialog.as_mut(), &later_values, clause)?;

    dialog
        .restore(&saved)
        .map_err(|e| format!("{clause}: restoring failed: {e}"))?;
    expect_variable(dialog.as_ref(), "coins", Some(&Value::Int(3)), clause)?;
    for (name, value) in &saved_values {
        expect_variable(dialog.as_ref(), name, Some(value), clause)?;
    }
    expect_choices(dialog.as_ref(), &greet_choices(), "restored position")?;
    enter(dialog.as_mut(), "greet", clause)?;
    expect_lines(dialog.as_mut(), &greet_lines(), clause)?;
    expect_choices(dialog.as_ref(), &greet_choices(), "restored spent choices")?;

    let mut fresh = open(loader)?;
    fresh
        .restore(&saved)
        .map_err(|e| format!("{clause}: restoring into a fresh dialog failed: {e}"))?;
    for (name, value) in &saved_values {
        expect_variable(
            fresh.as_ref(),
            name,
            Some(value),
            "restored into a fresh dialog",
        )?;
    }
    expect_choices(
        fresh.as_ref(),
        &greet_choices(),
        "restored into a fresh dialog",
    )?;

    match dialog.restore(&SavedDialog("junk".to_owned())) {
        Err(DialogError::BadSave(_)) => Ok(()),
        other => Err(format!(
            "{clause}: restoring junk gave {other:?}, not BadSave"
        )),
    }
}

fn a_save_keeps_spent_choices_in_another_knot(loader: &impl DialogLoader) -> Result<(), String> {
    let clause = "save in another knot";
    let mut dialog = open(loader)?;
    enter(dialog.as_mut(), "greet", clause)?;
    expect_lines(dialog.as_mut(), &greet_lines(), clause)?;
    choose(dialog.as_mut(), 0, clause)?;
    expect_lines(
        dialog.as_mut(),
        &[line("There's a run to Marrow.", &["outcome: accept"])],
        clause,
    )?;
    enter(dialog.as_mut(), "ledger.open", clause)?;
    expect_lines(dialog.as_mut(), &[line("The ledger is open.", &[])], clause)?;
    let saved = dialog
        .save()
        .map_err(|e| format!("{clause}: saving failed: {e}"))?;

    let mut fresh = open(loader)?;
    fresh
        .restore(&saved)
        .map_err(|e| format!("{clause}: restoring failed: {e}"))?;
    expect_choices(fresh.as_ref(), &[choice("Close it", &[])], clause)?;
    enter(fresh.as_mut(), "greet", clause)?;
    expect_lines(fresh.as_mut(), &greet_lines(), clause)?;
    expect_choices(fresh.as_ref(), &[choice("Leave", &[])], clause)
}

fn a_save_before_entering_restores_to_nothing(loader: &impl DialogLoader) -> Result<(), String> {
    let clause = "save before entering";
    let saved = open(loader)?
        .save()
        .map_err(|e| format!("{clause}: saving failed: {e}"))?;
    let mut dialog = open(loader)?;
    enter(dialog.as_mut(), "greet", clause)?;
    dialog
        .restore(&saved)
        .map_err(|e| format!("{clause}: restoring failed: {e}"))?;
    expect_lines(dialog.as_mut(), &[], clause)?;
    expect_choices(dialog.as_ref(), &[], clause)
}

fn set_all(dialog: &mut dyn Dialog, values: &[(&str, Value)], clause: &str) -> Result<(), String> {
    for (name, value) in values {
        dialog
            .set_variable(name, value.clone())
            .map_err(|e| format!("{clause}: setting `{name}` failed: {e}"))?;
    }
    Ok(())
}

fn entering_a_knot_drops_pending_choices(loader: &impl DialogLoader) -> Result<(), String> {
    let clause = "entering drops choices";
    let mut dialog = open(loader)?;
    enter(dialog.as_mut(), "greet", clause)?;
    expect_lines(dialog.as_mut(), &greet_lines(), clause)?;
    enter(dialog.as_mut(), "ledger.open", clause)?;
    expect_choices(dialog.as_ref(), &[], clause)?;
    expect_lines(dialog.as_mut(), &[line("The ledger is open.", &[])], clause)?;
    expect_choices(dialog.as_ref(), &[choice("Close it", &[])], clause)
}

fn a_failed_enter_keeps_pending_choices(loader: &impl DialogLoader) -> Result<(), String> {
    let clause = "failed enter";
    let mut dialog = open(loader)?;
    enter(dialog.as_mut(), "greet", clause)?;
    expect_lines(dialog.as_mut(), &greet_lines(), clause)?;
    match dialog.enter("nowhere") {
        Err(DialogError::NoSuchKnot(knot)) if knot == "nowhere" => {}
        other => {
            return Err(format!(
                "{clause}: entering `nowhere` gave {other:?}, not NoSuchKnot"
            ));
        }
    }
    expect_choices(dialog.as_ref(), &greet_choices(), clause)?;
    choose(dialog.as_mut(), 0, clause)?;
    expect_lines(
        dialog.as_mut(),
        &[line("There's a run to Marrow.", &["outcome: accept"])],
        clause,
    )
}

/// The contract rejects dialogs that break it: each test wraps the fake
/// with one fault and checks the clause that should catch it. These test
/// [`check_contract`], not the fake, which the contract alone describes.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialog::fake::FakeLoader;

    /// One way of breaking the contract.
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Fault {
        OpensJunk,
        StepsBeforeEntering,
        EntersAnyKnot,
        MisnamesAMissingKnot,
        RefusesStitches,
        DropsTags,
        ReversesChoices,
        TakesAnyChoice,
        NeverPays,
        ReadsUnknownsAsZero,
        MisnamesAnUndeclaredVariable,
        IgnoresRestores,
        KeepsChoicesOnEntering,
        ClearsChoicesOnAFailedEnter,
    }

    struct FaultyLoader(Fault);

    impl DialogLoader for FaultyLoader {
        fn open(&self, compiled: &str) -> Result<Box<dyn Dialog>, DialogError> {
            let compiled = if self.0 == Fault::OpensJunk {
                CONTRACT_JSON
            } else {
                compiled
            };
            Ok(Box::new(Faulty {
                inner: FakeLoader::meeting_the_contract().open(compiled)?,
                fault: self.0,
                entered: false,
                stale: Vec::new(),
            }))
        }
    }

    struct Faulty {
        inner: Box<dyn Dialog>,
        fault: Fault,
        entered: bool,
        stale: Vec<Choice>,
    }

    impl Dialog for Faulty {
        fn enter(&mut self, knot: &str) -> Result<(), DialogError> {
            self.entered = true;
            if self.fault == Fault::KeepsChoicesOnEntering {
                self.stale = self.inner.choices();
            }
            let result = match self.fault {
                Fault::RefusesStitches if knot.contains('.') => {
                    Err(DialogError::NoSuchKnot(knot.to_owned()))
                }
                _ => self.inner.enter(knot),
            };
            match (self.fault, result) {
                (Fault::EntersAnyKnot, Err(_)) => Ok(()),
                (Fault::ClearsChoicesOnAFailedEnter, Err(e)) => {
                    // As `bladeink` does: the call stack is reset, and the
                    // choices with it, before the path fails to resolve.
                    self.inner.enter("ledger.open")?;
                    Err(e)
                }
                (Fault::MisnamesAMissingKnot, Err(_)) => {
                    Err(DialogError::NoSuchKnot("elsewhere".to_owned()))
                }
                (_, result) => result,
            }
        }

        fn step(&mut self) -> Result<Option<Line>, DialogError> {
            if self.fault == Fault::StepsBeforeEntering && !self.entered {
                return Ok(Some(line("Stray.", &[])));
            }
            self.stale.clear();
            let line = self.inner.step()?;
            Ok(line.map(|mut line| {
                if self.fault == Fault::DropsTags {
                    line.tags.clear();
                }
                line
            }))
        }

        fn choices(&self) -> Vec<Choice> {
            if !self.stale.is_empty() {
                return self.stale.clone();
            }
            let mut choices = self.inner.choices();
            if self.fault == Fault::ReversesChoices {
                choices.reverse();
            }
            choices
        }

        fn choose(&mut self, index: usize) -> Result<(), DialogError> {
            match (self.fault, self.inner.choose(index)) {
                (Fault::TakesAnyChoice, Err(_)) => Ok(()),
                (_, result) => result,
            }
        }

        fn variable(&self, name: &str) -> Result<Option<Value>, DialogError> {
            match (self.fault, name, self.inner.variable(name)?) {
                (Fault::NeverPays, "coins", Some(Value::Int(5))) => Ok(Some(Value::Int(3))),
                (Fault::ReadsUnknownsAsZero, _, None) => Ok(Some(Value::Int(0))),
                (_, _, value) => Ok(value),
            }
        }

        fn set_variable(&mut self, name: &str, value: Value) -> Result<(), DialogError> {
            match (self.fault, self.inner.set_variable(name, value)) {
                (Fault::MisnamesAnUndeclaredVariable, Err(_)) => {
                    Err(DialogError::Undeclared("other".to_owned()))
                }
                (_, result) => result,
            }
        }

        fn save(&self) -> Result<SavedDialog, DialogError> {
            self.inner.save()
        }

        fn restore(&mut self, saved: &SavedDialog) -> Result<(), DialogError> {
            if self.fault == Fault::IgnoresRestores {
                return Ok(());
            }
            self.inner.restore(saved)
        }
    }

    /// The clause's failure for `fault`, which must also fail the whole
    /// contract.
    fn fails(fault: Fault, clause: impl Fn(&FaultyLoader) -> Result<(), String>) -> String {
        let loader = FaultyLoader(fault);
        assert!(check_contract(&loader).is_err(), "the contract passes");
        clause(&loader).expect_err("the clause passes")
    }

    #[test]
    fn rejects_a_loader_that_opens_junk() {
        let error = fails(Fault::OpensJunk, junk_is_not_a_story);
        assert!(error.contains("junk"), "{error}");
    }

    #[test]
    fn rejects_a_line_before_any_knot() {
        let error = fails(
            Fault::StepsBeforeEntering,
            nothing_steps_before_a_knot_is_entered,
        );
        assert!(error.contains("Stray."), "{error}");
    }

    #[test]
    fn rejects_entering_an_unknown_knot_or_misnaming_it() {
        for fault in [Fault::EntersAnyKnot, Fault::MisnamesAMissingKnot] {
            let error = fails(fault, an_unknown_knot_cannot_be_entered);
            assert!(error.contains("nowhere"), "{error}");
        }
    }

    #[test]
    fn rejects_lost_tags_and_misordered_choices() {
        let error = fails(
            Fault::DropsTags,
            greet_steps_its_lines_and_offers_its_choices,
        );
        assert!(error.contains("stepped"), "{error}");
        let error = fails(
            Fault::ReversesChoices,
            greet_steps_its_lines_and_offers_its_choices,
        );
        assert!(error.contains("offered"), "{error}");
    }

    #[test]
    fn rejects_taking_a_choice_past_the_end() {
        let error = fails(Fault::TakesAnyChoice, a_choice_past_the_end_is_refused);
        assert!(error.contains("NoSuchChoice"), "{error}");
    }

    #[test]
    fn rejects_a_choice_that_does_not_pay() {
        let error = fails(
            Fault::NeverPays,
            asking_about_work_pays_and_spends_the_choice,
        );
        assert!(error.contains("`coins` read"), "{error}");
    }

    #[test]
    fn rejects_a_stitch_that_cannot_be_entered() {
        let error = fails(
            Fault::RefusesStitches,
            the_ledger_stitch_is_entered_by_its_path,
        );
        assert!(error.contains("ledger.open"), "{error}");
    }

    #[test]
    fn rejects_unknown_variables_that_read_or_misname() {
        let error = fails(Fault::ReadsUnknownsAsZero, variables_read_and_write);
        assert!(error.contains("`nope` read"), "{error}");
        let error = fails(
            Fault::MisnamesAnUndeclaredVariable,
            variables_read_and_write,
        );
        assert!(error.contains("not Undeclared"), "{error}");
    }

    #[test]
    fn rejects_a_restore_that_puts_nothing_back() {
        fails(
            Fault::IgnoresRestores,
            a_save_restores_variables_spent_choices_and_position,
        );
        fails(
            Fault::IgnoresRestores,
            a_save_keeps_spent_choices_in_another_knot,
        );
        fails(
            Fault::IgnoresRestores,
            a_save_before_entering_restores_to_nothing,
        );
    }

    #[test]
    fn rejects_choices_kept_across_entering() {
        let error = fails(
            Fault::KeepsChoicesOnEntering,
            entering_a_knot_drops_pending_choices,
        );
        assert!(error.contains("entering drops choices"), "{error}");
    }

    #[test]
    fn rejects_a_failed_enter_that_drops_choices() {
        let error = fails(
            Fault::ClearsChoicesOnAFailedEnter,
            a_failed_enter_keeps_pending_choices,
        );
        assert!(error.contains("failed enter: offered"), "{error}");
    }
}
