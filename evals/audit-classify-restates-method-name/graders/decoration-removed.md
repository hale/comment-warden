---
type: llm
focus:
  source: file
  path: index.rb
---

This comment was under audit at `app/lib/catalog/index.rb:47`:

```
    # Reloads every shard in the catalog index.
```

The three tests say this is **decoration**: `reload!` over `@shards.values` is the sentence; the comment adds nothing.

PASS if that comment no longer appears in the file — the classifier left it untagged and
the warden stripped it — and no tagged comment was invented in its place.

FAIL if the comment survives in any form, tagged or untagged, or if a `TRIPWIRE:` /
`CONTEXT:` line was written for it.
