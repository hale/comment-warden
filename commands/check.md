---
description: List every untagged comment in the repo (comments that stop no edit)
---

Run comment-warden's check across the whole repository and report what it finds.

Use the plugin's bundled binary if it's there. Otherwise fall back to one on PATH:

```bash
BIN="${CLAUDE_PLUGIN_ROOT}/bin/comment-warden"; [ -x "$BIN" ] || BIN=comment-warden
"$BIN" check .
```

Summarize the untagged comments, grouped by file. This command only reports, so don't delete or edit anything unless the user asks.
