//! A hand-written fake of the dialog port, for the story layer's tests.
//!
//! A [`FakeLoader`] maps compiled texts to [`FakeDialog`]s scripted by
//! hand: knots of lines and choices, each choice once-only or sticky, with
//! its own lines and the variables it sets, and the variables the story
//! declares. The fake runs no Ink: a knot is entered by its whole path
//! (`ledger.open`), a choice's lines end the story, and a choice sets
//! fixed values rather than evaluating expressions.
//!
//! What it does is stated once, by the port's contract
//! ([`check_contract`](super::fixture::check_contract)), which it passes
//! over [`FakeLoader::meeting_the_contract`]. It also logs the
//! [`enter`](Dialog::enter) and [`choose`](Dialog::choose) calls it
//! receives, for core tests to assert on: each dialog's own
//! ([`FakeDialog::log`]), and every dialog a loader opened, in one list
//! ([`FakeLoader::log`]), for a test that hands the loader to the code
//! under test and keeps only the loader.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::rc::Rc;

use super::fixture::CONTRACT_JSON;
use super::{Choice, Dialog, DialogError, DialogLoader, Line, SavedDialog, Value};

/// A [`DialogLoader`] that opens the stories it was given, each by its
/// exact compiled text. Any other text is a [`DialogError::BadStory`].
#[derive(Clone, Debug, Default)]
pub struct FakeLoader {
    stories: Vec<(String, FakeDialog)>,
    log: Log,
}

impl FakeLoader {
    /// Opens `dialog` for the compiled text `compiled`.
    #[must_use]
    pub fn story(mut self, compiled: &str, dialog: FakeDialog) -> Self {
        self.stories.push((compiled.to_owned(), dialog));
        self
    }

    /// The [`enter`](Dialog::enter) and [`choose`](Dialog::choose) calls
    /// every dialog this loader opened received, in order.
    #[must_use]
    pub fn log(&self) -> Vec<DialogCall> {
        self.log.borrow().clone()
    }

    /// The contract story ([`CONTRACT_JSON`]), scripted.
    #[must_use]
    pub fn meeting_the_contract() -> Self {
        let greet = FakeKnot::new("greet")
            .line("Hello.", &["speaker: clerk"])
            .line("How can I help?", &[])
            .choice(
                FakeChoice::once("Ask about work", &["topic: work"])
                    .line("There's a run to Marrow.", &["outcome: accept"])
                    .sets("coins", Value::Int(5)),
            )
            .choice(FakeChoice::sticky("Leave", &[]).line("Goodbye.", &["outcome: decline"]));
        let ledger = FakeKnot::new("ledger.open")
            .line("The ledger is open.", &[])
            .choice(FakeChoice::sticky("Close it", &[]));
        let dialog = FakeDialog::default()
            .declare("met", Value::Bool(false))
            .declare("coins", Value::Int(3))
            .declare("rate", Value::Float(0.5))
            .declare("name", Value::Text("Tally".to_owned()))
            .knot(greet)
            .knot(ledger);
        Self::default().story(CONTRACT_JSON, dialog)
    }
}

impl DialogLoader for FakeLoader {
    fn open(&self, compiled: &str) -> Result<Box<dyn Dialog>, DialogError> {
        self.stories
            .iter()
            .find(|(text, _)| text == compiled)
            .map(|(_, dialog)| {
                let mut dialog = dialog.clone();
                dialog.log = Rc::clone(&self.log);
                Box::new(dialog) as Box<dyn Dialog>
            })
            .ok_or_else(|| DialogError::BadStory("the fake has no such story".to_owned()))
    }
}

/// A knot of a [`FakeDialog`]: its lines, then its choices.
#[derive(Clone, Debug, PartialEq)]
pub struct FakeKnot {
    path: String,
    lines: Vec<Line>,
    choices: Vec<FakeChoice>,
}

impl FakeKnot {
    /// An empty knot, entered by `path` (`greet`, `ledger.open`).
    #[must_use]
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_owned(),
            lines: Vec::new(),
            choices: Vec::new(),
        }
    }

    /// Adds a line.
    #[must_use]
    pub fn line(mut self, text: &str, tags: &[&str]) -> Self {
        self.lines.push(line(text, tags));
        self
    }

    /// Adds a choice, offered once the lines are stepped.
    #[must_use]
    pub fn choice(mut self, choice: FakeChoice) -> Self {
        self.choices.push(choice);
        self
    }
}

/// A choice of a [`FakeKnot`]: what it offers, whether it is spent once
/// taken, the lines it steps and the variables it sets. Its lines end the
/// story.
#[derive(Clone, Debug, PartialEq)]
pub struct FakeChoice {
    offer: Choice,
    once: bool,
    lines: Vec<Line>,
    sets: Vec<(String, Value)>,
}

impl FakeChoice {
    /// A once-only choice (Ink's `*`), not offered again once taken.
    #[must_use]
    pub fn once(text: &str, tags: &[&str]) -> Self {
        Self::new(text, tags, true)
    }

    /// A sticky choice (Ink's `+`), offered every time.
    #[must_use]
    pub fn sticky(text: &str, tags: &[&str]) -> Self {
        Self::new(text, tags, false)
    }

    fn new(text: &str, tags: &[&str], once: bool) -> Self {
        Self {
            offer: Choice {
                text: text.to_owned(),
                tags: owned(tags),
            },
            once,
            lines: Vec::new(),
            sets: Vec::new(),
        }
    }

    /// Adds a line it steps once taken.
    #[must_use]
    pub fn line(mut self, text: &str, tags: &[&str]) -> Self {
        self.lines.push(line(text, tags));
        self
    }

    /// Sets `name` to `value` when taken.
    #[must_use]
    pub fn sets(mut self, name: &str, value: Value) -> Self {
        self.sets.push((name.to_owned(), value));
        self
    }
}

/// A call a [`FakeDialog`] received.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DialogCall {
    /// [`Dialog::enter`], with its knot.
    Enter(String),
    /// [`Dialog::choose`], with its index.
    Choose(usize),
}

/// The calls a dialog received, shared with the loader that opened it.
type Log = Rc<RefCell<Vec<DialogCall>>>;

/// Where a [`FakeDialog`] is: in a knot's lines (then at its choices), or
/// in a taken choice's lines (then at the end).
#[derive(Clone, Debug, PartialEq, Eq)]
struct At {
    knot: usize,
    choice: Option<usize>,
    next: usize,
}

/// A scripted story, built from [`FakeKnot`]s and declared variables.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FakeDialog {
    knots: Vec<FakeKnot>,
    variables: BTreeMap<String, Value>,
    spent: BTreeSet<(usize, usize)>,
    at: Option<At>,
    log: Log,
}

impl FakeDialog {
    /// Declares the variable `name`, holding `value`.
    #[must_use]
    pub fn declare(mut self, name: &str, value: Value) -> Self {
        self.variables.insert(name.to_owned(), value);
        self
    }

    /// Adds a knot.
    #[must_use]
    pub fn knot(mut self, knot: FakeKnot) -> Self {
        self.knots.push(knot);
        self
    }

    /// The [`enter`](Dialog::enter) and [`choose`](Dialog::choose) calls
    /// received, in order, whether or not they succeeded.
    #[must_use]
    pub fn log(&self) -> Vec<DialogCall> {
        self.log.borrow().clone()
    }

    /// The choices of the knot the dialog waits at, each with its index
    /// among the knot's choices.
    fn offered(&self) -> Vec<(usize, &FakeChoice)> {
        let Some(At {
            knot,
            choice: None,
            next,
        }) = self.at
        else {
            return Vec::new();
        };
        let knot_ = &self.knots[knot];
        if next < knot_.lines.len() {
            return Vec::new();
        }
        knot_
            .choices
            .iter()
            .enumerate()
            .filter(|&(i, choice)| !(choice.once && self.spent.contains(&(knot, i))))
            .collect()
    }

    fn encode(&self) -> String {
        let mut lines = vec![SAVE_HEADER.to_owned()];
        lines.push(match &self.at {
            Some(At { knot, choice, next }) => {
                let choice = choice.map_or("-".to_owned(), |c| c.to_string());
                format!("at {} {choice} {next}", hex(&self.knots[*knot].path))
            }
            None => "at -".to_owned(),
        });
        for (knot, choice) in &self.spent {
            lines.push(format!("spent {} {choice}", hex(&self.knots[*knot].path)));
        }
        for (name, value) in &self.variables {
            let value = match value {
                Value::Bool(b) => format!("b {b}"),
                Value::Int(n) => format!("i {n}"),
                Value::Float(f) => format!("f {}", f.to_bits()),
                Value::Text(t) => format!("t {}", hex(t)),
            };
            lines.push(format!("var {} {value}", hex(name)));
        }
        lines.join("\n")
    }

    /// The state `saved` holds, or `None` if it isn't one this story's
    /// [`encode`](Self::encode) wrote.
    fn decode(&self, saved: &str) -> Option<Decoded> {
        let mut lines = saved.lines();
        if lines.next()? != SAVE_HEADER {
            return None;
        }
        let mut state = Decoded::default();
        for line in lines {
            let fields: Vec<&str> = line.split(' ').collect();
            match fields.as_slice() {
                ["at", "-"] => state.at = None,
                ["at", knot, choice, next] => {
                    let knot = self.knot_index(knot)?;
                    let choice = match *choice {
                        "-" => None,
                        c => Some(c.parse().ok()?),
                    };
                    state.at = Some(At {
                        knot,
                        choice,
                        next: next.parse().ok()?,
                    });
                }
                ["spent", knot, choice] => {
                    state
                        .spent
                        .insert((self.knot_index(knot)?, choice.parse().ok()?));
                }
                ["var", name, kind, value] => {
                    let value = match *kind {
                        "b" => Value::Bool(value.parse().ok()?),
                        "i" => Value::Int(value.parse().ok()?),
                        "f" => Value::Float(f32::from_bits(value.parse().ok()?)),
                        "t" => Value::Text(unhex(value)?),
                        _ => return None,
                    };
                    state.variables.insert(unhex(name)?, value);
                }
                _ => return None,
            }
        }
        Some(state)
    }

    /// The index of the knot whose path [`hex`] wrote as `path`.
    fn knot_index(&self, path: &str) -> Option<usize> {
        let path = unhex(path)?;
        self.knots.iter().position(|knot| knot.path == path)
    }
}

/// The first line of every state the fake saves.
const SAVE_HEADER: &str = "fake-dialog";

/// A saved state, read back.
#[derive(Default)]
struct Decoded {
    at: Option<At>,
    spent: BTreeSet<(usize, usize)>,
    variables: BTreeMap<String, Value>,
}

impl Dialog for FakeDialog {
    fn enter(&mut self, knot: &str) -> Result<(), DialogError> {
        self.log
            .borrow_mut()
            .push(DialogCall::Enter(knot.to_owned()));
        let index = self
            .knots
            .iter()
            .position(|k| k.path == knot)
            .ok_or_else(|| DialogError::NoSuchKnot(knot.to_owned()))?;
        self.at = Some(At {
            knot: index,
            choice: None,
            next: 0,
        });
        Ok(())
    }

    fn step(&mut self) -> Result<Option<Line>, DialogError> {
        let Some(at) = &mut self.at else {
            return Ok(None);
        };
        let knot = &self.knots[at.knot];
        let lines = match at.choice {
            None => &knot.lines,
            Some(choice) => &knot.choices[choice].lines,
        };
        let line = lines.get(at.next).cloned();
        if line.is_some() {
            at.next += 1;
        }
        Ok(line)
    }

    fn choices(&self) -> Vec<Choice> {
        self.offered()
            .into_iter()
            .map(|(_, choice)| choice.offer.clone())
            .collect()
    }

    fn choose(&mut self, index: usize) -> Result<(), DialogError> {
        self.log.borrow_mut().push(DialogCall::Choose(index));
        let offered = self.offered();
        let count = offered.len();
        let &(taken, choice) = offered
            .get(index)
            .ok_or(DialogError::NoSuchChoice { index, count })?;
        let sets = choice.sets.clone();
        let knot = self.at.as_ref().map_or(0, |at| at.knot);
        self.spent.insert((knot, taken));
        self.variables.extend(sets);
        self.at = Some(At {
            knot,
            choice: Some(taken),
            next: 0,
        });
        Ok(())
    }

    fn variable(&self, name: &str) -> Result<Option<Value>, DialogError> {
        Ok(self.variables.get(name).cloned())
    }

    fn set_variable(&mut self, name: &str, value: Value) -> Result<(), DialogError> {
        let slot = self
            .variables
            .get_mut(name)
            .ok_or_else(|| DialogError::Undeclared(name.to_owned()))?;
        *slot = value;
        Ok(())
    }

    fn save(&self) -> Result<SavedDialog, DialogError> {
        Ok(SavedDialog(self.encode()))
    }

    fn restore(&mut self, saved: &SavedDialog) -> Result<(), DialogError> {
        let state = self
            .decode(&saved.0)
            .ok_or_else(|| DialogError::BadSave("not a state the fake saved".to_owned()))?;
        self.at = state.at;
        self.spent = state.spent;
        self.variables = state.variables;
        Ok(())
    }
}

fn line(text: &str, tags: &[&str]) -> Line {
    Line {
        text: text.to_owned(),
        tags: owned(tags),
    }
}

fn owned(tags: &[&str]) -> Vec<String> {
    tags.iter().map(|&t| t.to_owned()).collect()
}

/// `text`'s UTF-8 bytes in hex, so a saved name or text holds no spaces or
/// newlines.
fn hex(text: &str) -> String {
    text.bytes().fold(String::new(), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}

/// The text [`hex`] wrote, or `None`.
fn unhex(hex: &str) -> Option<String> {
    let bytes = hex
        .as_bytes()
        .chunks(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok())
        .collect::<Option<Vec<u8>>>()?;
    String::from_utf8(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialog::fixture::check_contract;

    #[test]
    fn the_fake_meets_the_contract() {
        assert_eq!(check_contract(&FakeLoader::meeting_the_contract()), Ok(()));
    }

    // The log is fake-only: the contract can't reach it.

    #[test]
    fn a_dialog_logs_the_calls_it_receives_failed_or_not() {
        let mut dialog = FakeDialog::default().knot(FakeKnot::new("k"));
        dialog.enter("k").unwrap();
        dialog.choose(3).unwrap_err();
        dialog.enter("nowhere").unwrap_err();
        assert_eq!(
            dialog.log(),
            [
                DialogCall::Enter("k".to_owned()),
                DialogCall::Choose(3),
                DialogCall::Enter("nowhere".to_owned()),
            ]
        );
    }

    #[test]
    fn a_loader_logs_the_calls_of_every_dialog_it_opened() {
        let loader = FakeLoader::meeting_the_contract();
        let mut first = loader.open(CONTRACT_JSON).unwrap();
        let mut second = loader.open(CONTRACT_JSON).unwrap();
        first.enter("greet").unwrap();
        while first.step().unwrap().is_some() {}
        second.enter("ledger.open").unwrap();
        first.choose(1).unwrap();
        assert_eq!(
            loader.log(),
            [
                DialogCall::Enter("greet".to_owned()),
                DialogCall::Enter("ledger.open".to_owned()),
                DialogCall::Choose(1),
            ]
        );
    }
}
