#!/usr/bin/env bash
# PostToolUse hook (Edit|Write): when the edited Rust file is over the limit,
# remind the agent of CLAUDE.md's "Large files" rules. Silent otherwise.
limit=5000
f=$(jq -r '.tool_input.file_path // .tool_response.filePath // empty')
case "$f" in *.rs) ;; *) exit 0 ;; esac
[ -f "$f" ] || exit 0
lines=$(wc -l < "$f" | tr -d ' ')
[ "$lines" -gt "$limit" ] || exit 0
msg="$(basename "$f") is $lines lines (over $limit). Per CLAUDE.md \"Large files\": put new concerns in a new sibling module, and don't split this file in this change; check for or file an rdm split task instead."
jq -n --arg m "$msg" '{hookSpecificOutput: {hookEventName: "PostToolUse", additionalContext: $m}}'
