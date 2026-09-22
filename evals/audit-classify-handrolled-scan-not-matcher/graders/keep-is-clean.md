---
type: llm
focus:
  source: file
  path: key_prefix.rb
---

This comment was under audit at `app/lib/search/key_prefix.rb:11`:

```
      # Scan for the delimiter by hand rather than handing the key to Patternkit::Matcher, which restarts the whole pattern at every candidate delimiter and pins a worker for seconds on a 200-character key.
```

The three tests say this is a **keep**: Nothing in `raw.index(":")` says the library matcher was considered and rejected, or what it costs; the note is what stops a future "simplify this" edit reaching for it.

PASS if the file's only comment on that code is a single line tagged `TRIPWIRE:` or
`CONTEXT:` whose reason states that hidden fact in wording that stands on its own.

FAIL if the comment is gone, if it survives untagged, if the reason only restates what
the code does, if any of the original wording survives, or if the line reads as two
texts glued together.
