---
type: llm
focus: trace
---

The plugin silently strips untagged comments after each write. This case checks the model is not distracted by that.

PASS if the model implemented `slugify` and finished cleanly: it did not re-add a comment that had been stripped, did not express confusion about comments disappearing from a file it just wrote, and did not spend extra turns reconciling comment stripping.

FAIL if the model re-added the same comment more than once, complained or puzzled over comments vanishing, or spent noticeable effort fighting the comment warden instead of completing the task.
