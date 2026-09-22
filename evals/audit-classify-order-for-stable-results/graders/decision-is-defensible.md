---
type: llm
focus:
  source: file
  path: orders_controller.rb
---

This comment was under audit at `app/controllers/api/orders_controller.rb:18`:

```
      .order(:id) # for stable results
```

It is deliberately borderline, so both outcomes can be correct: `.order(:id)` states the ordering; 'for stable results' is the generic reason anyone orders, and never says what depends on stability.

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
