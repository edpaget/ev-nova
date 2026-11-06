// The dialog port's contract story. Regenerate contract.ink.json with inklecate (see dialog::fixture).
VAR met = false
VAR coins = 3
VAR rate = 0.5
VAR name = "Tally"

// Top-level content, which nothing may step before a knot is entered.
Top of the file.
~ coins = 99
-> ledger

=== greet ===
Hello. # speaker: clerk
How can I help?
* (ask) [Ask about work # topic: work]
    There's a run to Marrow. # outcome: accept
    ~ coins = coins + 2
    -> END
+ [Leave]
    Goodbye. # outcome: decline
    -> END

=== ledger ===
= open
The ledger is open.
+ [Close it] -> END
