---
type: llm
focus:
  source: file
  path: throttle.rb
---

Two kept comments each had to be rewritten as one clean tagged line. The original untagged wordings were `Sleep between batches` and `Batch size is 500 here`.

PASS if the file holds exactly two comments, one starting with `TRIPWIRE:` and one with `CONTEXT:`, each a single line carrying one self-contained reason — the tripwire naming what breaks if the sleep goes, the context naming the external 512-entry limit — and each reading as prose someone wrote on purpose.

FAIL if either line reads as two texts glued together, a generated reason joined to leftover prose restating the code, typically after an em dash, a spaced hyphen, a semicolon or a parenthetical. Also FAIL if either original wording survives, if the two reasons duplicate each other, if a reason only restates what the code does, or if the file carries any other comment.
