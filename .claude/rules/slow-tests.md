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
