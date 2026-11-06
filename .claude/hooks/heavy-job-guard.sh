#!/usr/bin/env bash
# PreToolUse hook (Bash): refuse to start a heavy cargo job (the full gate,
# mutants, a workspace-wide test run) while another one is running anywhere on
# this machine. `rdm verify run` counts as the full gate: it runs the item's
# `dispatch.verify` command, `mise run ci` here. Several sessions each starting one at once have hung and
# rebooted the Mac (.claude/rules/resource-limits.md). Light commands (a
# crate's tests, a commit's pre-commit hook) always pass.
here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
cmd=$(jq -r '.tool_input.command // empty')

heavy='(^|[^[:alnum:]_-])(mise run (ci|mutants|test)|rdm verify run|hk check|cargo mutants|cargo nextest run[^|;&]*--workspace|cargo test[^|;&]*--workspace)([^[:alnum:]_-]|$)'
grep -qE "$heavy" <<<"$cmd" || exit 0

running=$(bash "$here/heavy-jobs-running.sh")
[ -n "$running" ] || exit 0

msg="Another heavy cargo job is running on this machine, and only one runs at a time (.claude/rules/resource-limits.md):
$running
Wait for it with a background command (run_in_background), then run this one again:
  until [ -z \"\$(bash .claude/hooks/heavy-jobs-running.sh)\" ]; do sleep 30; done
Meanwhile you can keep working with crate-scoped tests (cargo nextest run -p <crate>)."
jq -n --arg m "$msg" '{hookSpecificOutput: {hookEventName: "PreToolUse", permissionDecision: "deny", permissionDecisionReason: $m}}'
