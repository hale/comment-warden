---
type: llm
focus:
  source: file
  path: safe_paginator.rb
---

This comment was under audit at `app/lib/safe_paginator.rb:28`:

```
      # the more idiomatic .exists? doesn't work once the relation is grouped, for some reason. TODO: why?
      # but, select("1 as one").limit(1).present? *should* be equivalent. I think.
```

It is deliberately borderline, so both outcomes can be correct: The wording hedges ('for some reason', 'I think'), but it is the only record that the idiomatic `.exists?` was tried and failed, which is the whole reason for the strange select/limit/present? shape. Rewrite the wording, do not lose the fact.

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
