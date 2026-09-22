---
type: llm
focus:
  source: file
  path: message.rb
---

The kept comment had to be rewritten as one clean tagged line. The original untagged wording was `Override content= to handle Payload::Blob objects properly`.

PASS if the file's only comment is a single line starting with `CONTEXT:` whose text is one self-contained reason about why a `Payload::Blob` has to be JSON-encoded, reading as prose someone wrote on purpose. A single sentence with a subordinate clause ("… has no round-trippable string form, so it must be JSON-encoded before storage") is exactly right and passes.

FAIL if the line reads as two texts glued together — a generated reason joined to leftover prose describing what the writer does, typically after an em dash, a spaced hyphen, a semicolon or a parenthetical. Also FAIL if any of the original wording survives, if the reason only restates what the code does, or if the file carries any other comment.
