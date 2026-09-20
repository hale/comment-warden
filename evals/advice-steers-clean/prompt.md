---
runs: 3
allowed_tools: [Read, Write, Edit]
---

Create a file `retry.rs` with a function `fn retry` that retries a fallible operation three times, sleeping a fixed delay between attempts. There is a real gotcha: if a future editor removes the sleep between attempts, the retries will hammer the downstream service. Record that hazard where a future editor will see it, then implement the function cleanly.
