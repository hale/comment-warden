#!/usr/bin/env python3
"""Emit one `claude plugin eval` case directory per entry in cases.json.

The offline harness and the real runner have to agree on what is being asked, so
both read the same cases.json and the same policy text. Run this after editing
cases.json; the generated directories are committed so the suite works without it.
"""

import json
import pathlib
import re
import shutil

from run_harness import THREE_TESTS, WRITING_RULES, comment_body

HERE = pathlib.Path(__file__).resolve().parent
EVALS = HERE.parent
PREFIX = "audit-classify-"

PROMPT = """---
runs: 3
allowed_tools: [Read, Write, Edit]
---

You are the bulk classifier in a comment audit, making the one decision phase 4 makes
thousands of times: judge a single comment against the policy, then write the file the
decision implies.

{policy}

{writing_rules}

Write `{filename}` with exactly this content, except for the one comment under
judgement:

```ruby
{snippet}
```

The comment under judgement is:

```
{comment}
```

Apply the tests. If it is a keep, replace that comment with a single line — the tag, a
colon, and one self-contained reason in your own words. Do not carry any of the original
wording across, and do not join clauses with an em dash or a spaced hyphen. If it is
decoration, leave the comment exactly as it stands and change nothing; do not delete it
yourself. Leave the rest of the file alone either way.

If you are genuinely torn, leave the comment as it stands and also write
`review-queue.md` naming the file and the tension in a phrase. Only do this for a real
tie, never for a call that is merely hard.
"""

KEEP_LLM = """---
type: llm
focus:
  source: file
  path: {filename}
---

This comment was under audit at `{source}`:

```
{comment}
```

The three tests say this is a **keep**: {reference_note}

PASS if the file's only comment on that code is a single line tagged `TRIPWIRE:` or
`CONTEXT:` whose reason states that hidden fact in wording that stands on its own.

FAIL if the comment is gone, if it survives untagged, if the reason only restates what
the code does, if any of the original wording survives, or if the line reads as two
texts glued together.
"""

DROP_LLM = """---
type: llm
focus:
  source: file
  path: {filename}
---

This comment was under audit at `{source}`:

```
{comment}
```

The three tests say this is **decoration**: {reference_note}

PASS if that comment no longer appears in the file — the classifier left it untagged and
the warden stripped it — and no tagged comment was invented in its place.

FAIL if the comment survives in any form, tagged or untagged, or if a `TRIPWIRE:` /
`CONTEXT:` line was written for it.
"""

BORDERLINE_LLM = """---
type: llm
focus:
  source: file
  path: {filename}
---

This comment was under audit at `{source}`:

```
{comment}
```

It is deliberately borderline, so both outcomes can be correct: {reference_note}

PASS if the file shows one of three clean outcomes. Either the comment is gone entirely —
left untagged and stripped, with nothing invented in its place — or it has been replaced
by a single line tagged `TRIPWIRE:` or `CONTEXT:` whose reason states a specific fact the
surrounding code does not already convey, in wording that stands on its own — or the
classifier declared itself torn, leaving the code alone and naming the real tension in
`review-queue.md`.

FAIL if the comment survives untagged in a form the warden should have taken, if a keep's
reason merely restates the code or is generic enough to fit any instance of the pattern,
if a keep preserves the original wording or narration that should have collapsed to the
one hidden fact, or if the line reads as two texts glued together.
"""

NO_SPLICE = """---
type: regex
target: {{ source: file, path: {filename} }}
pattern: '(TRIPWIRE|CONTEXT):[^\\n]*(—|–|--| - )'
match: not_contains
---
"""

NO_ORIGINAL = """---
type: regex
target: {{ source: file, path: {filename} }}
pattern: '{phrase}'
match: not_contains
---
"""

CODE_INTACT = """---
type: regex
target: {{ source: file, path: {filename} }}
pattern: '{token}'
---
"""

COMMENT_GONE = """---
type: regex
target: {{ source: file, path: {filename} }}
pattern: '{phrase}'
match: not_contains
---
"""


def escape(text):
    return re.escape(text).replace("\\ ", " ").replace("'", "''")


def first_words(comment, n=6):
    return " ".join(comment_body(comment).split()[:n])


def code_token(snippet):
    m = re.search(r"^\s*def ([a-z_][a-z0-9_?!]*)", snippet, re.M)
    if m:
        return f"def {m.group(1)}"
    m = re.search(r"^\s*class ([A-Za-z:]+)", snippet, re.M)
    return f"class {m.group(1)}"


def write(path, text):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


def main():
    spec = json.loads((HERE / "cases.json").read_text())
    for stale in EVALS.glob(f"{PREFIX}*"):
        shutil.rmtree(stale)

    for case in spec["cases"]:
        d = EVALS / f"{PREFIX}{case['id']}"
        fields = dict(
            case,
            policy=THREE_TESTS,
            writing_rules=WRITING_RULES,
            snippet=case["snippet"].rstrip(),
            phrase=escape(first_words(case["comment"])),
            token=escape(code_token(case["snippet"])),
        )
        write(d / "prompt.md", PROMPT.format(**fields))
        write(d / "graders" / "code-intact.md", CODE_INTACT.format(**fields))

        if case["difficulty"] == "borderline":
            write(d / "graders" / "decision-is-defensible.md", BORDERLINE_LLM.format(**fields))
            write(d / "graders" / "no-splice.md", NO_SPLICE.format(**fields))
        elif case["reference"] == "drop":
            write(d / "graders" / "decoration-removed.md", DROP_LLM.format(**fields))
            write(d / "graders" / "comment-gone.md", COMMENT_GONE.format(**fields))
        else:
            write(d / "graders" / "keep-is-clean.md", KEEP_LLM.format(**fields))
            write(d / "graders" / "no-splice.md", NO_SPLICE.format(**fields))
            write(d / "graders" / "no-original-wording.md", NO_ORIGINAL.format(**fields))
        print(f"wrote {d.relative_to(EVALS.parent)}")


if __name__ == "__main__":
    main()
