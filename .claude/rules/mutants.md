---
paths:
  - ".cargo/mutants.toml"
  - "mutants.out/**"
---

# Running cargo-mutants

Let the tool generate and run the mutants rather than editing source by hand.

- Run mutants once over everything on the branch:
  `mise run mutants -- --in-diff <(git diff main...HEAD)`. Use three dots:
  `git diff main` also holds `main`'s newer changes in reverse once `main`
  moves on, and mutates lines the branch never touched.
- To confirm the fixes for missed mutants, rerun the same command with
  `--iterate`. It skips every mutant already caught in `mutants.out/` and
  tests only the missed ones and any new code. Prefer this to `--file`
  reruns, which rebuild every mutant in the file.
- Add `--baseline=skip` when the gate has just passed: the baseline only
  rechecks that the unmutated tests pass.
- Start the run with `run_in_background` and wait for its completion
  notification. Don't poll it with `sleep` or `until` loops.
- Run one mutants session at a time across all worktrees. They share the
  machine's cores, and two at once slow both. `mise.toml` sets
  `CARGO_MUTANTS_JOBS=4`; pass `-j` to change it for one run.
- For each `MISSED` mutant, write a test that fails against that mutant, then
  rerun to confirm it is `caught`.
- For a mutant that genuinely can't change observable behaviour, add a pattern
  for it to `exclude_re` in `.cargo/mutants.toml` with a comment explaining why.
- Results are written to `mutants.out/` (gitignored). `missed.txt` lists the
  survivors.
- Repeated runs grow `target/debug/deps`, which slows the wgpu tests (see
  `.claude/rules/slow-tests.md`). Clean the target directory now and then in
  long sessions.
