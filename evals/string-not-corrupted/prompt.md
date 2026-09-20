---
runs: 3
allowed_tools: [Read, Write, Edit]
---

Create a file `url.rs` with a function `fn scheme(u: &str) -> Option<&str>` that returns the substring before `"://"` in a URL, or `None` if there is no `"://"`. In the same file, add a `#[test]` that asserts `scheme("https://example.com")` returns `Some("https")`. Keep it minimal, with no explanatory comments.
