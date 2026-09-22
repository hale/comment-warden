---
type: llm
focus:
  source: file
  path: sessions_controller.rb
---

This comment was under audit at `app/controllers/sessions_controller.rb:61`:

```
    # String key, not Symbol: the session store's JSON codec drops Symbol keys before readers see them (TICKET-1234).
```

The three tests say this is a **keep**: Guards a specific future edit (someone tidying the String key to a Symbol) with a named serializer behaviour and a ticket.

PASS if the file's only comment on that code is a single line tagged `TRIPWIRE:` or
`CONTEXT:` whose reason states that hidden fact in wording that stands on its own.

FAIL if the comment is gone, if it survives untagged, if the reason only restates what
the code does, if any of the original wording survives, or if the line reads as two
texts glued together.
