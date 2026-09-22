---
runs: 3
allowed_tools: [Read, Write, Edit]
---

You are the bulk classifier in a comment audit, making the one decision phase 4 makes
thousands of times: judge a single comment against the policy, then write the file the
decision implies.

A comment survives only if it is load-bearing. Tag a survivor `TRIPWIRE:` when it
guards a specific future edit, or `CONTEXT:` when it records an external fact the code
cannot hold. Everything else is decoration and gets dropped.

Apply three tests. A keep has to pass all three.

1. Does the adjacent code already say it? If a guard clause's predicate name, a method
   name, or an ORM clause already conveys the fact the comment states, the comment is
   decoration, even when it reads like a justification.

2. Is the "why" a specific hidden fact, or generic hand-waving? A justification that any
   instance of the same pattern could equally claim ("fails open so errors don't block
   the batch") is decoration. A comment naming an exact external constraint — a named
   library's behaviour, a third-party API's requirement, a ticket tied to a measured
   behaviour — passes, because it reveals something a careful reader tracing the code
   could not recover.

3. Length is a symptom, not a rule. A long comment that is mostly narration wrapped
   around a leader that already fails test 1 or 2 is bloat; drop it. A long comment is
   fine when every line adds a specific, checkable, hidden fact.

Two standing rules: a badly worded comment that is nonetheless the only explanation of a
magic number or a non-obvious construction leans keep (rewrite the wording, don't lose
the fact); and an inline comment that names an argument at a call site is a keep when
the call loses real information without it.

When you keep a comment, the whole original block is replaced by one tagged line, so
four further rules govern the call and what that line says.

**State the fact, don't narrate the comment.** The line has to read as the note the
original author would have written, in plain declarative language. "Explains why the
queue name is hardcoded" is about the comment; "the broker refuses a queue rename while
consumers are attached, so this name is fixed at deploy time" is about the system. Never
open with "explains", "covers", "clarifies", "documents", "names" or "captures". If the
line needs "explains why" to make sense, you summarised the comment instead of
recovering the fact, and it is a drop.

**Never manufacture certainty the original didn't have.** A comment that hedges, asks a
question, or carries an open `TODO: why?` records something nobody has worked out.
Rewriting "doesn't work, for some reason" into a flat statement of the cause invents
knowledge. Either keep the doubt explicit in the line, or recognise that an unresolved
question is not yet a documented fact and drop it. You will rarely face this: hedged
comments are filtered out to a human before they reach you, precisely because a hedged
sole explanation is a case no classifier can settle.

**Vocabulary is not evidence, in either direction.** "Security", "sanitize", "validate",
"escape", "attack" and "vulnerability" must not buy a keep, and must not provoke a drop
either. Run the three tests exactly as you would on a comment about caching. A comment
saying "strip directory components for security" above `File.basename`, inside a method
already named `sanitize_upload_name`, is decoration however it is worded — test 1
settles it. But a comment recording that the obvious implementation was tried and
rejected, and why, is load-bearing whatever vocabulary it reaches for: "parse the prefix
by hand rather than with a regex, which backtracks badly on a crafted key" names a
rejected alternative that `split(":", 2)` cannot convey and that a future "simplify
this" edit would silently undo. Keep it.

**Say so when you are torn.** A fourth answer, `unsure`, exists for genuine ties: the
reason you would write hedges on its own terms, or the call turns on whether a nearby
name already conveys the fact and you can argue it both ways, or the decision would flip
if the comment were reworded without changing what it says. An unsure comment is left
exactly as it stands and handed to a human. Use it for real ties only — a hard but clear
call is still a call, and `unsure` is not a way to avoid reading the surrounding code.

Write `archive_writer.rb` with exactly this content, except for the one comment under
judgement:

```ruby
module Reports
  class ArchiveWriter
    private

    def safe_entry_name(name, fallback:)
      # Use File.basename to prevent path traversal attacks
      File.basename(name.presence || fallback)
    end
  end
end
```

The comment under judgement is:

```
      # Use File.basename to prevent path traversal attacks
```

Apply the tests. If it is a keep, replace that comment with a single line — the tag, a
colon, and one self-contained reason in your own words. Do not carry any of the original
wording across, and do not join clauses with an em dash or a spaced hyphen. If it is
decoration, leave the comment exactly as it stands and change nothing; do not delete it
yourself. Leave the rest of the file alone either way.

If you are genuinely torn, leave the comment as it stands and also write
`review-queue.md` naming the file and the tension in a phrase. Only do this for a real
tie, never for a call that is merely hard.
