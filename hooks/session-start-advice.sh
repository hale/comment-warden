#!/usr/bin/env bash
# States the comment rule once per session, so silent stripping is never a
# surprise. The text is a policy prompt: the binary's `advice` subcommand prints
# the repo's override from comment-warden.toml, or the shipped default. Logic
# lives in the tested binary; this script only finds it, and carries a terse
# fallback so the rule still lands if the binary is missing.
set -uo pipefail

root="${CLAUDE_PLUGIN_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}"

bin=""
for cand in "${COMMENT_WARDEN_BIN:-}" "${CLAUDE_PLUGIN_DATA:-}/bin/comment-warden" "$root/bin/comment-warden" "$(command -v comment-warden 2>/dev/null || true)"; do
  if [ -n "$cand" ] && [ -x "$cand" ]; then bin="$cand"; break; fi
done

if [ -n "$bin" ]; then
  exec "$bin" advice
fi

printf '%s\n' '{"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":"comment-warden is active: write a comment only when the code cannot say it itself, and default to none (including /// doc comments). Two tags survive — TRIPWIRE: names the edit it prevents, CONTEXT: records an external fact. Untagged comments are stripped automatically after each write; do not re-add them."}}'
