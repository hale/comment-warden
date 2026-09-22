---
type: llm
focus:
  source: file
  path: cacheKey.ts
---

The kept comment had to be rewritten as one clean tagged line. The original untagged wording was `Sort the keys before hashing them`.

PASS if the file's only comment is a single line starting with `TRIPWIRE:` whose text is one self-contained reason naming what breaks if the sort is removed, reading as prose someone wrote on purpose. A single sentence with a subordinate clause ("remove this sort and identical parameters hash to different cache keys, so the cache silently misses") is exactly right and passes.

FAIL if the line reads as two texts glued together — a generated reason joined to leftover prose restating the sort, typically after an em dash, a spaced hyphen, a semicolon or a parenthetical. Also FAIL if any of the original wording survives, if the reason only restates what the code does, or if the file carries any other comment.
