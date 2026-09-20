#!/usr/bin/env bash
# On session start, fetch the comment-warden binary for this platform into the
# plugin's persistent data dir, so the strip hook has something to run without
# the user building it by hand. The repo is private, so the download uses the
# user's existing `gh` auth. Silent and best-effort: any failure just leaves the
# binary absent, and the strip hook reports that on its own. Delete the cached
# binary to force a re-fetch (e.g. after a plugin update).
set -uo pipefail

data="${CLAUDE_PLUGIN_DATA:-}"
[ -n "$data" ] || exit 0
bin="$data/bin/comment-warden"
[ -x "$bin" ] && exit 0

command -v gh >/dev/null 2>&1 || exit 0
command -v tar >/dev/null 2>&1 || exit 0

case "$(uname -sm)" in
  "Darwin arm64") target=aarch64-apple-darwin ;;
  "Darwin x86_64") target=x86_64-apple-darwin ;;
  "Linux aarch64" | "Linux arm64") target=aarch64-unknown-linux-musl ;;
  "Linux x86_64") target=x86_64-unknown-linux-musl ;;
  *) exit 0 ;;
esac

tmp="$(mktemp -d)" || exit 0
trap 'rm -rf "$tmp"' EXIT

if gh release download --repo hale/comment-warden \
    --pattern "comment-warden-${target}.tar.gz" --dir "$tmp" >/dev/null 2>&1; then
  tar -xzf "$tmp/comment-warden-${target}.tar.gz" -C "$tmp" >/dev/null 2>&1 || exit 0
  found="$(find "$tmp" -type f -name comment-warden | head -1)"
  [ -n "$found" ] || exit 0
  mkdir -p "$data/bin"
  cp "$found" "$bin" && chmod +x "$bin"
fi
exit 0
