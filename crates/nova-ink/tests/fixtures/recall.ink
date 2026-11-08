// nova-ink's own fixture story: what the dialog port's contract story
// can't show without running Ink. Regenerate recall.ink.json with
// inklecate (see nova-ink's tests module).
VAR met = false
VAR name = "Tally"
LIST mood = calm, angry

-> END

=== ask ===
The clerk looks up.
* (work) [Ask about work] There's a run to Marrow.
    -> END
+ [Leave]
    # outcome: decline
    -> END

=== recall ===
{met: Welcome back, {name}.|Who are you?}
{ask.work: You asked about work.|You never asked.}
-> END

=== dice ===
{RANDOM(1, 1000)}
{RANDOM(1, 1000)}
{~calm|tense|bored|curious|tired|cheerful}
{~north|south|east|west|up|down}
-> END

=== broken ===
This knot is a tunnel, entered as a knot.
->->
