---
runs: 3
allowed_tools: [Read, Write, Edit]
---

Create `throttle.rb` from scratch — there is no existing file to read or edit. It holds a `Throttle` class whose `each_batch(records)` yields slices of 500 records and sleeps 0.1 seconds between them.

A comment audit on the codebase this class came from turned up two confirmed keeps that belong in it. Both are currently untagged.

First keep:

    # Sleep between batches

Its load-bearing fact: the endpoint rate-limits at 10 requests per second and answers 429 without a `Retry-After`, and this client does not retry, so removing the sleep silently drops writes. That makes it a `TRIPWIRE:`.

Second keep:

    # Batch size is 500 here

Its load-bearing fact: the endpoint rejects any payload over 512 entries with an unhelpful generic 400, so 500 is a ceiling imposed from outside, not a tuning choice. That makes it a `CONTEXT:`.

Carry both keeps into the new file, each directly above the code it guards, and each written as one clean line: the tag, a colon, then a single reason in your own words. Each reason has to stand on its own. Do not keep any of the original wording, and do not glue clauses together with an em dash or a spaced hyphen.
