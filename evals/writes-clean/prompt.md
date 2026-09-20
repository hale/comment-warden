---
runs: 3
allowed_tools: [Read, Write, Edit]
---

Create a new file `port.rs` with a function `fn parse_port(s: &str) -> Result<u16, String>` that trims surrounding whitespace, parses the input as a `u16`, and returns a clear error message when parsing fails. Keep it minimal.
