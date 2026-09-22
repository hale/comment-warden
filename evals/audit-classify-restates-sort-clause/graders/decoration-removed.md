---
type: llm
focus:
  source: file
  path: snapshots_controller.rb
---

This comment was under audit at `app/controllers/api/snapshots_controller.rb:23`:

```
      .sort_by { |s| -s["revision"] } # desc by revision
```

The three tests say this is **decoration**: `sort_by { -s["revision"] }` is literally 'desc by revision'.

PASS if that comment no longer appears in the file — the classifier left it untagged and
the warden stripped it — and no tagged comment was invented in its place.

FAIL if the comment survives in any form, tagged or untagged, or if a `TRIPWIRE:` /
`CONTEXT:` line was written for it.
