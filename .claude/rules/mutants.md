---
paths:
  - ".cargo/mutants.toml"
  - "mutants.out/**"
---

# Running cargo-mutants

Let the tool generate and run the mutants rather than editing source by hand.

- Run mutants once per phase (or task), over that phase's own commits:
  `mise run mutants -- --in-diff <(git diff <previous commit>...HEAD)`, where
  `<previous commit>` is the commit the phase started from (its
  `started_head`). On a roadmap's shared worktree, `main...HEAD` would take in
  every earlier phase's changes too. Use three dots:
  `git diff main` also holds `main`'s newer changes in reverse once `main`
  moves on, and mutates lines the branch never touched.
- cargo-mutants tests each mutant with its own package's tests only. A crate
  with an `<x>-it` sibling keeps its integration tests there, so a run that
  mutates such a crate passes `--test-package` for every mutated package plus
  the sibling:
  `mise run mutants -- --test-package nova-sim --test-package nova-sim-it --in-diff …`.
  Without it, the integration tests never see the crate's mutants.
- To confirm the fixes for missed mutants, rerun the same command with
  `--iterate`. It skips every mutant already caught in `mutants.out/` and
  tests only the missed ones and any new code. Prefer this to `--file`
  reruns, which rebuild every mutant in the file.
- Add `--baseline=skip` when the gate has just passed: the baseline only
  rechecks that the unmutated tests pass.
- Start the run with `run_in_background` and wait for its completion
  notification. Don't poll it with `sleep` or `until` loops.
- Run one mutants session at a time across all worktrees, and never alongside
  the full gate. `.claude/hooks/heavy-job-guard.sh` refuses the run while
  another heavy job is going. `mise run mutants` builds two mutants at once
  (`CARGO_MUTANTS_JOBS=2`) with the `mutants` nextest profile; don't raise
  them (`.claude/rules/resource-limits.md`).
- To wait for another session's run, use the helper in a background command:
  `until [ -z "$(bash .claude/hooks/heavy-jobs-running.sh)" ]; do sleep 30; done`.
  Never use `pgrep -f "cargo-mutants ..."` in a wait loop: `-f` matches whole
  command lines, and the waiting shell's own command line contains the
  pattern, so the loop finds itself and never ends.
- For each `MISSED` mutant, write a test that fails against that mutant, then
  rerun to confirm it is `caught`.
- For a mutant that genuinely can't change observable behaviour, add a pattern
  for it to `exclude_re` in `.cargo/mutants.toml` with a comment explaining why.
- Results are written to `mutants.out/` (gitignored). `missed.txt` lists the
  survivors.
- Repeated runs grow `target/debug/deps`, which slows the wgpu tests (see
  `.claude/rules/slow-tests.md`). Clean the target directory now and then in
  long sessions.
