#!/usr/bin/env bash
# Prints one line per heavy cargo job running anywhere on this machine, in any
# worktree or session: the full gate, a mutants run, or a workspace-wide test
# run. Prints nothing when none is running. heavy-job-guard.sh uses it, and an
# agent waits on it in a background command:
#   until [ -z "$(bash .claude/hooks/heavy-jobs-running.sh)" ]; do sleep 30; done
# The patterns are anchored at the start of the command line, so a shell whose
# own command line mentions them (like the wait loop above) never matches.
# See .claude/rules/resource-limits.md.
{
  pgrep -lx cargo-mutants
  pgrep -lf '^[^ ]*mise run (ci|mutants|test)( |$)'
  pgrep -lf '^[^ ]*cargo-nextest nextest run .*--workspace'
} 2>/dev/null | sort -u
