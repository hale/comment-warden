---
type: llm
focus:
  source: file
  path: cart_totals.rb
---

This comment was under audit at `app/lib/storefront/cart_totals.rb:149`:

```
      # Sum the stored amount columns — no price list lookup needed at render time.
      # line_items is already loaded by the caller; select avoids a second query.
      # amount_cents / amount_parts can be nil for carts imported from the old
      # storefront, for gift wrapping, and for shipping-only lines — skip those rows.
      #
      # Sum as BigDecimal for accuracy, then cast to Float at the boundary:
      # BigDecimal serializes to JSON as a quoted string, which breaks the web
      # client's `.toFixed()` calls. Float gives 15-17 digits, plenty for display.
```

It is deliberately borderline, so both outcomes can be correct: Most of the block restates the code (the sum, the select, the nil guard), but the last clause is a genuinely hidden fact: BigDecimal serializes to JSON as a quoted string and breaks the web client's `.toFixed()`. A keep here must collapse to that one fact; keeping the whole block is the failure this case looks for.

PASS if the file shows one of three clean outcomes. Either the comment is gone entirely —
left untagged and stripped, with nothing invented in its place — or it has been replaced
by a single line tagged `TRIPWIRE:` or `CONTEXT:` whose reason states a specific fact the
surrounding code does not already convey, in wording that stands on its own — or the
classifier declared itself torn, leaving the code alone and naming the real tension in
`review-queue.md`.

FAIL if the comment survives untagged in a form the warden should have taken, if a keep's
reason merely restates the code or is generic enough to fit any instance of the pattern,
if a keep preserves the original wording or narration that should have collapsed to the
one hidden fact, or if the line reads as two texts glued together.
