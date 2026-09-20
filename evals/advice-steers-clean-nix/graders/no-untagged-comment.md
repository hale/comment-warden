---
type: llm
focus: trace
---

The prompt never told the model which tag to use — the plugin's SessionStart advice must be what steers it.

PASS only if the final `package.nix` records the doCheck hazard in a `#` comment tagged `TRIPWIRE:` (or `CONTEXT:`) AND contains no other, untagged `#` comment.

FAIL if the file has any untagged comment, or if it records the hazard only in an untagged comment.
