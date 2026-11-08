//! nova-ink's own fixture story (`tests/fixtures/recall.ink`, regenerated
//! as nova-ink's unit tests module says), played through the dialog port
//! alone: every knot but `broken` (whose runtime error the unit tests
//! pin) and `dice` (whose random draws they pin), and every choice.

use nova_ink::InkLoader;
use nova_sim_story::dialog::{Dialog, DialogLoader};

const RECALL_JSON: &str = include_str!("../fixtures/recall.ink.json");

/// Enters `knot`, steps its lines and lists its choices, then takes each
/// of `picks` in turn, stepping the lines that follow; each line is
/// written as its text and its tags, each choice offered with a `?`, and
/// each choice taken with a `>`.
fn play(dialog: &mut dyn Dialog, knot: &str, picks: &[usize], transcript: &mut Vec<String>) {
    transcript.push(format!("enter {knot}"));
    dialog.enter(knot).expect("the knot is entered");
    for &pick in picks.iter().chain([&usize::MAX]) {
        while let Some(line) = dialog.step().expect("the line steps") {
            let mut text = line.text;
            for tag in &line.tags {
                text.push_str(" #");
                text.push_str(tag);
            }
            transcript.push(text);
        }
        let choices = dialog.choices();
        for choice in &choices {
            transcript.push(format!("? {}", choice.text));
        }
        if pick == usize::MAX {
            break;
        }
        transcript.push(format!("> {}", choices[pick].text));
        dialog.choose(pick).expect("the choice is taken");
    }
}

#[test]
fn the_recall_story_plays_through_every_knot_and_choice() {
    let mut dialog = InkLoader::seeded(0).open(RECALL_JSON).unwrap();
    let mut transcript = Vec::new();
    play(dialog.as_mut(), "recall", &[], &mut transcript);
    play(dialog.as_mut(), "ask", &[1], &mut transcript);
    play(dialog.as_mut(), "ask", &[0], &mut transcript);
    play(dialog.as_mut(), "ask", &[0], &mut transcript);
    play(dialog.as_mut(), "recall", &[], &mut transcript);

    assert_eq!(
        transcript,
        [
            "enter recall",
            "Who are you?",
            "You never asked.",
            "enter ask",
            "The clerk looks up.",
            "? Ask about work",
            "? Leave",
            "> Leave",
            " #outcome: decline",
            "enter ask",
            "The clerk looks up.",
            "? Ask about work",
            "? Leave",
            "> Ask about work",
            "There's a run to Marrow.",
            "enter ask",
            "The clerk looks up.",
            "? Leave",
            "> Leave",
            " #outcome: decline",
            "enter recall",
            "Who are you?",
            "You asked about work.",
        ]
    );
}
