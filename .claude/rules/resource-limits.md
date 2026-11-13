# Resource limits: run the least that verifies the change

Several agent sessions share this Mac (18 cores, 128 GB), each in its own
worktree, with a rust-analyzer per open checkout. No single test is heavy:
the unit tests peak at about 50 MB and a stock test at about 155 MB. What has
hung and rebooted the machine is stacking: four sessions, each with cargo's
and nextest's one-per-core defaults, a mutants run building four mutants at
once, and GPU tests in parallel. On 2026-11-05 nine runaway mutant test
processes held 37 GB. On 2026-11-06 the Mac hung during a mutants run and was
powered off, and right after reboot a mutants rustc wrote 34 GB in nine
minutes.

## What enforces the limits

- `.config/nextest.toml`: 8 test threads per run (CI uses every core), and at
  most 2 wgpu tests at a time (the `gpu` test group).
- `hk.pkl`: the pre-commit hook runs each crate's clippy and nextest at the
  same time, each cargo capped at 8 jobs, build scripts included: FFmpeg's
  `make` joins cargo's jobserver. On a cold worktree (new, or after
  `cargo clean`) FFmpeg builds once in `target/debug` and once in
  `target/clippy`.
- `mise run mutants`: 2 mutants at a time, 8 jobserver tasks, and the
  `mutants` nextest profile, which kills a test after 20 s and runs 4 at once.
- `.claude/hooks/heavy-job-guard.sh`: refuses to start the full gate, a
  mutants run or a workspace-wide test run while one is already running in any
  worktree. When it refuses, wait in a background command, then retry:
  `until [ -z "$(bash .claude/hooks/heavy-jobs-running.sh)" ]; do sleep 30; done`.
  Keep working with crate-scoped tests meanwhile.

Don't raise these limits with flags to go faster. Don't sidestep the guard,
for example by calling `cargo` directly with the gate's steps.

## The verification ladder

Each rung runs only when the one below passes and the work has reached it.

1. **TDD loop**: the one test or module you're working on,
   `cargo nextest run -p <crate> <filter>`. Run the whole crate
   (`-p <crate>`, no filter) when a step is green, not after every edit.
2. **Commit**: the pre-commit hook runs clippy and nextest for each crate
   with staged changes. Don't run them yourself just before committing.
3. **Before needs-review, once, one at a time**:
   - the full gate, `mise run ci`, once;
   - the stock tests of the crates the change touches and their `<crate>-it`
     siblings, one crate at a time
     (`NOVA_DATA=... cargo nextest run -p <crate> --no-tests=warn -E 'test(/^stock(_|::)/)'`,
     or every crate's at once with `NOVA_DATA=... mise run stock`). A crate
     with no stock tests of its own (they moved to its `-it` sibling) then
     passes with a warning instead of failing;
   - mutants once over the phase's own commits
     (`--in-diff <(git diff <previous commit>...HEAD)`, `.claude/rules/mutants.md`).
   Run these one after another, never alongside each other. Rerun only the
   one that failed, and only the crate that failed it, after a fix.
4. **Landing** (`rdm:land`) runs the gate and stock tests again.

Never run `--workspace` tests, `mise run test` or `mise run ci` inside the
TDD loop or after each small change. Workspace-wide checks belong to rung 3.
