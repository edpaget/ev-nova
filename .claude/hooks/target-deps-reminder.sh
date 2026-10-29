#!/usr/bin/env bash
# PreToolUse hook (Bash): before a cargo, mise, nextest or hk command, remind
# the agent to `cargo clean` any ev-nova checkout whose target/debug/deps holds
# more than the limit. A bloated deps folder slows every test process that
# scans it, and many of them at once have stalled WindowServer on an APFS lock
# (2026-10-28). See .claude/rules/slow-tests.md. Silent otherwise; never blocks.
limit=50000
main=/Users/edward/Projects/ev-nova
trees=/Users/edward/Projects/ev-nova__worktrees
input=$(cat)
cmd=$(jq -r '.tool_input.command // empty' <<<"$input")
cwd=$(jq -r '.cwd // empty' <<<"$input")
grep -qE '(^|[^[:alnum:]_-])(cargo|mise|nextest|hk)([^[:alnum:]_-]|$)' <<<"$cmd" || exit 0

# The checkouts the command can touch: any named in it, else the cwd's.
roots=$(grep -oE "$trees/[^/ \"']+|$main([/ \"']|$)" <<<"$cmd" | sed -E "s#[/ \"']\$##" | sort -u)
if [ -z "$roots" ]; then
  case "$cwd" in
    "$trees"/*) roots="$trees/$(cut -d/ -f1 <<<"${cwd#"$trees"/}")" ;;
    "$main"|"$main"/*) roots=$main ;;
    *) exit 0 ;;
  esac
fi

over=""
while IFS= read -r r; do
  [ -d "$r/target/debug/deps" ] || continue
  n=$(ls -f "$r/target/debug/deps" | wc -l | tr -d ' ')
  [ "$n" -gt "$limit" ] && over="$over $r ($n files)"
done <<<"$roots"
[ -n "$over" ] || exit 0

msg="target/debug/deps is over $limit files in:$over. Run \`cargo clean\` in that checkout before this command (once nothing else is building there). A bloated deps folder slows every test process and has stalled WindowServer; see .claude/rules/slow-tests.md."
jq -n --arg m "$msg" '{hookSpecificOutput: {hookEventName: "PreToolUse", additionalContext: $m}}'
