---
type: llm
focus:
  source: file
  path: invoice_exporter.rb
---

This comment was under audit at `app/lib/billing/invoice_exporter.rb:80`:

```
      # NOTE(jlq): Batching a whole page into one request instead of the obvious per-row calls, because the Sprocket Ledger API charges a row against the minute quota even when it rejects that row, so per-row retries lock the account out for the rest of the minute. The trailing newline is not optional either: their parser treats a body without it as truncated and silently drops the last row.
```

The three tests say this is a **keep**: Names two specific behaviours of a third-party API (a rejected row still costs quota, and the parser needs the trailing newline) which together explain why the obvious per-row loop was rejected. Unrecoverable from the code.

PASS if the file's only comment on that code is a single line tagged `TRIPWIRE:` or
`CONTEXT:` whose reason states that hidden fact in wording that stands on its own.

FAIL if the comment is gone, if it survives untagged, if the reason only restates what
the code does, if any of the original wording survives, or if the line reads as two
texts glued together.
