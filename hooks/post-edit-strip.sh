#!/usr/bin/env bash
# Silently strips untagged comments from the file just written. All logic lives
# in the `comment-warden hook` subcommand (reads the PostToolUse payload on stdin,
# strips projection-safely, emits at most a terse factual receipt) so this
# script only has to find the binary. If it cannot, it says so once rather than
# failing silently — an inert gate the model believes is active is worse.
set -uo pipefail

root="${CLAUDE_PLUGIN_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}"

bin=""
for cand in "${COMMENT_WARDEN_BIN:-}" "${CLAUDE_PLUGIN_DATA:-}/bin/comment-warden" "$root/bin/comment-warden" "$(command -v comment-warden 2>/dev/null || true)"; do
  if [ -n "$cand" ] && [ -x "$cand" ]; then bin="$cand"; break; fi
done

if [ -z "$bin" ]; then
  printf '%s\n' '{"hookSpecificOutput":{"hookEventName":"PostToolUse","additionalContext":"comment-warden: binary not found (looked at COMMENT_WARDEN_BIN, the plugin bin/, and PATH). Comments in the file you just wrote are unchecked."}}'
  exit 0
fi

exec "$bin" hook
