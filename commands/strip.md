---
description: Strip every untagged comment in the repo (projection-safe; keeps TRIPWIRE: and exempt)
---

Run comment-warden's strip over the whole repository. It deletes untagged comments in place and keeps anything tagged `TRIPWIRE:` or exempt. If deleting a comment would alter code, it refuses that deletion.

Use the plugin's bundled binary if it exists. Otherwise fall back to one on PATH:

```bash
BIN="${CLAUDE_PLUGIN_ROOT}/bin/comment-warden"; [ -x "$BIN" ] || BIN=comment-warden
"$BIN" strip .
```

Report how many comments were stripped. Then list anything left in place, which means a comment whose removal would have changed code or a line no grammar could read, so the user can judge those by hand.
