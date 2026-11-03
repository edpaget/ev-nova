---
paths:
  - "crates/nova-render/**"
---

# Slow or timing-out tests: check the target directory first

When the `nova-render::wgpu_offscreen` tests (or any other wgpu test) run for
seconds each or hit nextest's timeout, check how many files are in
`target/debug/deps` before you blame machine load or the code:

```sh
ls target/debug/deps | wc -l
```

A healthy count is a few thousand, and then the 23 offscreen tests take about
a second in parallel. Repeated `cargo-mutants` and stock runs can grow it to
hundreds of thousands. wgpu's Metal adapter setup reads the test binary's
directory, and nextest starts a new process for each test, so every test pays
that cost again. Run `cargo clean` in that worktree to fix it. Don't run the
tests serially or raise the timeouts to work around it. Long autopilot or
mutants sessions should clean the target directory now and then.

## Bound mutants and stock runs over GPU crates

Unbounded runs that drive wgpu have hung this Mac outright, even with a healthy
`target/debug/deps`. On 2026-11-02 and 2026-11-03 a phase that touched
`nova-render` ran workspace mutants and a workspace stock run. The machine
rebooted twice: a kernel watchdog panic, then a WindowServer watchdog timeout,
with `rustc` writing 137 GB in 11 minutes. The same checks, bounded as below,
ran without trouble.

- Run a GPU crate's mutants (`nova-render`, `nova`) with `-j 1` or `-j 2`,
  keeping its integration tests out, since they drive wgpu:
  `-- -E 'not binary_id(<crate>::it)' --test-threads 4`. Its unit tests carry
  the logic. The gate still runs the integration tests once.
- Run stock tests one crate at a time (`-p <crate> --test-threads 4`), not
  `--workspace`.
- Cap `CARGO_BUILD_JOBS` (6–8) for these runs, and run `mise run ci` once,
  not alongside a mutants run.
