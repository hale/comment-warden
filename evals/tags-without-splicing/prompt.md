---
runs: 3
allowed_tools: [Read, Write, Edit]
---

Create `message.rb` from scratch — there is no existing file to read or edit. It holds a `Message` class whose `content=` writer JSON-encodes a `Payload::Blob` and passes any other value straight through to `@content`.

A comment audit on the codebase this class came from turned up one confirmed keep that belongs above that writer. As written it is untagged:

    # Override content= to handle Payload::Blob objects properly

The load-bearing fact behind it is this: `Payload::Blob` has no round-trippable string form. Its `inspect` output is lossy, so the writer has to JSON-encode the blob before it reaches storage, or binary content comes back corrupted.

Carry that keep into the new file as a `CONTEXT:` comment, written as one clean line: the tag, a colon, then a single reason in your own words. The reason has to stand on its own. Do not keep any of the original wording, and do not glue clauses together with an em dash or a spaced hyphen.
