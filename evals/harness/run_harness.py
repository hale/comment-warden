#!/usr/bin/env python3
"""Offline stand-in for `claude plugin eval` on the audit's phase-4 classify decision.

The real runner is unusable while sandboxed runs cannot authenticate, so this
drives the same cases through two real model calls per case: Haiku classifies
(the model phase 4 actually uses), then an independent judge scores the call
against the three tests. Mechanical grading applies the classification to a
throwaway copy and runs the real binary over it.

Usage: run_harness.py [--cases cases.json] [--out results/<stamp>] [--only ID]
"""

import argparse
import collections
import json
import os
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
BIN = os.environ.get("COMMENT_WARDEN_BIN") or str(HERE.parent.parent / "bin" / "comment-warden")

THREE_TESTS = """A comment survives only if it is load-bearing. Tag a survivor `TRIPWIRE:` when it
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
the call loses real information without it."""

WRITING_RULES = """When you keep a comment, the whole original block is replaced by one tagged line, so
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
call is still a call, and `unsure` is not a way to avoid reading the surrounding code."""

CLASSIFY_TEMPLATE = """You are the bulk classifier in a comment audit. Judge exactly one comment.

{policy}

{writing_rules}

Here is the file under audit:

```{lang}
{snippet}
```

The comment under judgement is:

```
{comment}
```

Classify it. Reply with nothing but a single JSON object on one line:

{{"decision": "keep-tripwire" | "keep-context" | "drop" | "unsure", "reason": "<one line>"}}

If the decision is a keep, `reason` is the text that goes after the tag and colon: one
self-contained sentence in your own words, with the tag itself left out. The whole
original comment is replaced by that one line, so anything worth saying has to be in it.
Do not reuse the original wording, and do not join clauses with an em dash or a spaced
hyphen. If the decision is a drop, the comment is deleted outright and `reason` is a
one-line explanation of which test it failed. If the decision is `unsure`, the comment
is left untouched for a human and `reason` names the tension in a phrase."""

JUDGE_TEMPLATE = """You are grading one classification made during a comment audit. Grade the
classification against the stated policy, not against your own taste.

{policy}

The file under audit:

```{lang}
{snippet}
```

The comment under judgement:

```
{comment}
```

The classifier answered `{decision}`, and that answer has already been applied. This is
the resulting file, which is what you are grading:

```{lang}
{applied}
```

A drop deletes the comment outright; a keep replaces the whole original block with the
single tagged line you see; `unsure` leaves the file untouched and sends the comment to
a human. Grade what is there, not what you imagine a keep preserves.

PASS if the decision is the one the three tests imply for this comment AND, for a keep,
the tagged line states a specific fact the code genuinely cannot convey, in wording that
stands on its own.

FAIL if the decision contradicts the tests, if a keep's line merely restates what the
code does or is generic enough to apply to any instance of the pattern, if it reads as
two texts glued together, if it asserts a specific cause the original comment never
established, if it narrates the comment ("explains that…", "documents the…") rather than
stating the fact, or if it carries narration that should have collapsed to the one
hidden fact.

`unsure` is graded on whether the tie is real. PASS it only when the three tests
genuinely point both ways here — the fact sits right on the line between what a nearby
name already conveys and what it does not, or the comment is hedged enough that both a
keep and a drop are defensible — and the stated tension is the actual one. FAIL it when
the tests give a clear answer and the classifier ducked the call.

Reply with nothing but a single JSON object on one line:

{{"verdict": "PASS" | "FAIL", "why": "<one line>"}}"""


HEDGE = re.compile(
    r"not sure|for some reason|i think|no idea|unclear why|todo:\s*why|\?\s*$",
    re.I | re.M,
)

MODEL_CALLS = collections.Counter()


def claude(prompt, model, timeout=300):
    MODEL_CALLS[model] += 1
    proc = subprocess.run(
        ["claude", "-p", "--model", model],
        input=prompt,
        capture_output=True,
        text=True,
        cwd=tempfile.gettempdir(),
        timeout=timeout,
    )
    if proc.returncode != 0:
        raise RuntimeError(f"claude {model} exited {proc.returncode}: {proc.stderr[:500]}")
    return proc.stdout.strip()


def parse_json_reply(text):
    m = re.search(r"\{.*\}", text, re.S)
    if not m:
        raise ValueError(f"no JSON object in reply: {text[:300]}")
    return json.loads(m.group(0))


def warden(subcmd, path):
    proc = subprocess.run([BIN, subcmd, str(path)], capture_output=True, text=True)
    return proc.returncode, proc.stdout + proc.stderr


SPLICE = re.compile(r"(TRIPWIRE|CONTEXT):[^\n]*(—|–|--| - |;)")
WORD = re.compile(r"[A-Za-z_][A-Za-z0-9_.!?]*")


def shingles(text, n=7):
    words = WORD.findall(text.lower())
    return {" ".join(words[i : i + n]) for i in range(len(words) - n + 1)}


def is_trailing(comment):
    """True when the comment hangs off the end of a code line rather than owning its own."""
    return not comment.split("\n")[0].lstrip().startswith("#")


def comment_body(comment):
    """The prose of a comment block, with markers and any leading code removed."""
    parts = []
    for line in comment.split("\n"):
        stripped = line.lstrip()
        text = stripped[1:] if stripped.startswith("#") else line.split("#", 1)[1]
        parts.append(text.strip())
    return " ".join(p for p in parts if p)


def retag(comment, tag, reason):
    """The single tagged line that replaces a comment block, preserving any leading code."""
    first = comment.split("\n")[0]
    if is_trailing(comment):
        return f"{first.split('#')[0].rstrip()} # {tag}: {reason}"
    indent = first[: len(first) - len(first.lstrip())]
    return f"{indent}# {tag}: {reason}"


def apply_decision(case, decision, reason, workdir):
    """Write the throwaway file the classification implies. Returns (path, applied_text)."""
    snippet = case["snippet"]
    comment = case["comment"]
    if comment not in snippet:
        raise ValueError(f"{case['id']}: comment block not found verbatim in snippet")

    if decision in ("drop", "unsure"):
        applied = snippet
    else:
        tag = "TRIPWIRE" if decision == "keep-tripwire" else "CONTEXT"
        applied = snippet.replace(comment, retag(comment, tag, reason))

    path = workdir / case["filename"]
    path.write_text(applied)
    return path, applied


def mechanical(case, decision, reason, workdir):
    """Grade the classification with the real binary. Returns (ok, checks, final_text)."""
    checks = {}
    path, applied = apply_decision(case, decision, reason, workdir)

    if decision == "unsure":
        intact = case["comment"] in applied
        checks["comment_left_intact"] = (intact, "deferred untouched" if intact else "file was modified")
        rc, out = warden("check", path)
        checks["still_flagged"] = (rc == 1, f"check exit={rc}; an unsure item must stay flagged for the human")
    elif decision == "drop":
        rc, out = warden("strip", path)
        checks["strip_succeeded"] = (rc == 0, f"strip exit={rc} {out.strip()[:200]}")
        after = path.read_text()
        gone = comment_body(case["comment"]).split(".")[0] not in after
        checks["comment_removed"] = (gone, "comment gone after strip" if gone else "comment survived strip")
        rc, out = warden("check", path)
        checks["gate_clean"] = (rc == 0, f"check exit={rc} {out.strip()[:200]}")
    else:
        rc, out = warden("check", path)
        checks["gate_clean"] = (rc == 0, f"check exit={rc} {out.strip()[:200]}")
        spliced = bool(SPLICE.search(applied))
        checks["no_splice"] = (not spliced, "splice separator after tag" if spliced else "clean single reason")
        overlap = shingles(comment_body(case["comment"])) & shingles(reason)
        checks["no_verbatim_original"] = (
            not overlap,
            f"verbatim run survived: {sorted(overlap)[:1]}" if overlap else "no 7-word run reused",
        )
        tag_count = len(re.findall(r"(?:TRIPWIRE|CONTEXT):", applied))
        checks["tagged_once"] = (tag_count == 1, f"{tag_count} tag(s)")
        one_line = "\n" not in reason.strip()
        checks["single_line_reason"] = (one_line, "one line" if one_line else "reason spans lines")

    return all(ok for ok, _ in checks.values()), checks, path.read_text()


def run_case(case, workdir, classifier_model="haiku", judge_model="sonnet"):
    lang = "ruby"
    before = sum(MODEL_CALLS.values())

    if HEDGE.search(comment_body(case["comment"])):
        return {
            "id": case["id"],
            "probes": case["probes"],
            "difficulty": case["difficulty"],
            "source": case["source"],
            "reference": case["reference"],
            "decision": "hedge-prefiltered",
            "reason": "matched the hedge pre-filter; routed to human review without a model call",
            "mechanical_pass": True,
            "mechanical_checks": {},
            "judge_verdict": "N/A",
            "judge_why": "not classified",
            "overall_pass": True,
            "model_calls": sum(MODEL_CALLS.values()) - before,
            "classifier_model": classifier_model,
            "judge_model": judge_model,
        }

    classify_prompt = CLASSIFY_TEMPLATE.format(
        policy=THREE_TESTS,
        writing_rules=WRITING_RULES,
        lang=lang,
        snippet=case["snippet"].rstrip(),
        comment=case["comment"],
    )
    raw = claude(classify_prompt, classifier_model)
    parsed = parse_json_reply(raw)
    decision = parsed["decision"].strip()
    reason = " ".join(parsed["reason"].split())
    if decision not in ("keep-tripwire", "keep-context", "drop", "unsure"):
        raise ValueError(f"{case['id']}: bad decision {decision!r}")

    mech_ok, checks, applied = mechanical(case, decision, reason, workdir)

    judge_prompt = JUDGE_TEMPLATE.format(
        policy=THREE_TESTS,
        lang=lang,
        snippet=case["snippet"].rstrip(),
        comment=case["comment"],
        decision=decision,
        applied=applied.rstrip(),
    )
    judged = parse_json_reply(claude(judge_prompt, judge_model))
    verdict = judged["verdict"].strip().upper()

    ref = case["reference"]
    ref_family = "keep" if ref.startswith("keep") else "drop"
    got_family = {True: "keep"}.get(decision.startswith("keep"), decision)

    return {
        "id": case["id"],
        "probes": case["probes"],
        "difficulty": case["difficulty"],
        "source": case["source"],
        "reference": ref,
        "decision": decision,
        "reason": reason,
        "applied": applied,
        "matches_reference_family": ref_family == got_family,
        "mechanical_pass": mech_ok,
        "mechanical_checks": {k: {"pass": ok, "detail": d} for k, (ok, d) in checks.items()},
        "judge_verdict": verdict,
        "judge_why": judged.get("why", ""),
        "overall_pass": mech_ok and verdict == "PASS",
        "classifier_model": classifier_model,
        "judge_model": judge_model,
        "model_calls": sum(MODEL_CALLS.values()) - before,
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--cases", default=str(HERE / "cases.json"))
    ap.add_argument("--out", default=None)
    ap.add_argument("--only", action="append")
    ap.add_argument("--classifier-model", default="haiku")
    ap.add_argument("--judge-model", default="sonnet")
    args = ap.parse_args()

    if not os.path.exists(BIN):
        sys.exit(f"comment-warden binary not found at {BIN}; set COMMENT_WARDEN_BIN")
    if args.classifier_model == args.judge_model:
        sys.exit("classifier and judge must differ; a model marking its own work is not a grade")

    spec = json.loads(pathlib.Path(args.cases).read_text())
    cases = [c for c in spec["cases"] if not args.only or c["id"] in args.only]

    workroot = pathlib.Path(tempfile.mkdtemp(prefix="cw-harness-"))
    results = []
    for case in cases:
        wd = workroot / case["id"]
        wd.mkdir(parents=True)
        try:
            r = run_case(case, wd, args.classifier_model, args.judge_model)
        except Exception as exc:
            r = {"id": case["id"], "error": str(exc), "overall_pass": False}
        results.append(r)
        mark = "PASS" if r.get("overall_pass") else "FAIL"
        print(
            f"{mark:4}  {r['id']:32} decision={r.get('decision', '-'):14} "
            f"mech={r.get('mechanical_pass')} judge={r.get('judge_verdict', '-')}",
            flush=True,
        )
    shutil.rmtree(workroot, ignore_errors=True)

    passed = sum(1 for r in results if r.get("overall_pass"))
    prefiltered = [r for r in results if r.get("decision") == "hedge-prefiltered"]
    classified = [r for r in results if r.get("decision") != "hedge-prefiltered"]
    by_ref = lambda fam: [r for r in classified if str(r.get("reference", "")).startswith(fam)]
    decided_keep = [r for r in classified if str(r.get("decision", "")).startswith("keep")]
    summary = {
        "hedge_prefiltered": [r["id"] for r in prefiltered],
        "prefiltered_model_calls": sum(r.get("model_calls", 0) for r in prefiltered),
        "classified": f"{sum(1 for r in classified if r.get('overall_pass'))}/{len(classified)}",
        "policy_version": spec["policy_version"],
        "classifier_model": args.classifier_model,
        "judge_model": args.judge_model,
        "reference_keep_passed": f"{sum(1 for r in by_ref('keep') if r.get('overall_pass'))}/{len(by_ref('keep'))}",
        "reference_drop_passed": f"{sum(1 for r in by_ref('drop') if r.get('overall_pass'))}/{len(by_ref('drop'))}",
        "emitted_keeps_passed": f"{sum(1 for r in decided_keep if r.get('overall_pass'))}/{len(decided_keep)}",
        "total": len(results),
        "passed": passed,
        "failed": len(results) - passed,
        "mechanical_failures": [r["id"] for r in results if r.get("mechanical_pass") is False],
        "judge_failures": [r["id"] for r in results if r.get("judge_verdict") == "FAIL"],
        "deferred_unsure": [r["id"] for r in results if r.get("decision") == "unsure"],
        "decided_and_passed": sum(
            1 for r in results if r.get("overall_pass") and r.get("decision") != "unsure"
        ),
        "reference_disagreements": [
            r["id"] for r in results if r.get("matches_reference_family") is False
        ],
        "results": results,
    }
    print(f"\n{passed}/{len(results)} passed")

    if args.out:
        out = pathlib.Path(args.out)
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(json.dumps(summary, indent=2) + "\n")
        print(f"wrote {out}")
    return 0 if passed == len(results) else 1


if __name__ == "__main__":
    sys.exit(main())
