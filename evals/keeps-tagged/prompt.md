---
runs: 3
allowed_tools: [Read, Write, Edit]
---

Create a file `retry.rs` with a function that retries a fallible operation three times with a fixed delay between attempts. There is a real gotcha here: if someone later removes the delay between attempts, the code will hammer the downstream service. Leave a single comment that records that hazard so a future editor doesn't remove the delay — prefix it with `TRIPWIRE:` so the comment warden retains it — then implement the function.
