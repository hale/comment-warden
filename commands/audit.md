---
description: Adopt comment-warden on an existing codebase — calibrate on a sample, sweep the rest, apply, verify clean
---

This is the onboarding path for a codebase that predates the TRIPWIRE:/CONTEXT: convention. Do not run `strip` cold over such a codebase — a comment that carries real weight (an external fact, a guard against a specific future edit) looks identical to decoration until someone or something reads it, and blind stripping deletes both. This command reviews at a sample size a human can actually judge, then trusts a calibrated policy for the rest. Full line-by-line review of every flagged comment does not scale and is not the goal here — the working tree diff before commit is the safety net, not per-comment sign-off.

Use the plugin's bundled binary if it's there, otherwise fall back to one on PATH:

```bash
BIN="${CLAUDE_PLUGIN_ROOT}/bin/comment-warden"; [ -x "$BIN" ] || BIN=comment-warden
```

This command is written to run as the installed plugin, where
`$CLAUDE_PLUGIN_ROOT` is set. Outside that — dogfooding from a checkout — the
variable is unset and the fallback silently resolves to whatever `comment-warden`
is on `PATH`, which may be an old build or nothing at all. If you are not running
as the installed plugin, set `COMMENT_WARDEN_BIN` to a freshly built binary and
use that, or hardcode the path; do not trust the fallback.

## Phase 1 — Preflight: language coverage

List every file extension actually present in the repo (respect `.gitignore` — `git ls-files` is the simplest correct source if this is a git repo) with a count per extension. Compare against the tool's supported extensions (see README.md "Supported languages"). Report any extension with a non-trivial file count that has no mapping. Exit code 2 from `check` on a path means "nothing matched," not "clean" — never let a silent gap read as a clean bill of health. Tell the user what's covered and what isn't before doing anything else.

**Never pipe a command whose exit code is the signal you need.** `check ... | tail -3` reports `tail`'s status, not `check`'s, so a 2 reads as a 0 and the gap disappears. Redirect to a file and read the code separately:

```bash
"$BIN" check . > /tmp/warden-sweep.txt; echo "exit=$?"
```

### Functional pragmas

A comment a tool reads — a linter directive, a bundler magic comment — must never be stripped, because deleting it changes what the program does, and the projection-safety check cannot catch that (removing a comment never touches a non-comment token, so the re-parse always matches). The common ones are built in, so they never reach the sweep at all. Confirm that for this repo rather than assuming it: once you have the phase 2 output, grep it for pragma-shaped lines before going anywhere near phase 5, e.g.

```bash
grep -Ei 'disable|ignore|expect-error|nocheck|global |webpack|magic|@preserve|@license|noinspection|pragma|frozen_string' /tmp/warden-sweep.txt
```

Anything that comes back is a pragma the built-in list does not know about. The fix is config, not per-comment tagging: add it to `exempt_patterns` (prefix-matched against the comment body) in `comment-warden.toml`, or `exempt_paths` if a whole tree is involved. Re-run the sweep afterwards so the counts reflect the exemption.

## Phase 2 — Full sweep

Run `"$BIN" check .` over the repo (or the roots the user specifies). Redirect the output to a file rather than reading it all into context — it can be thousands of lines. Parse it programmatically (grep/awk/a short script) to get the full list of `file:line:comment text` and a total count.

## Phase 3 — Interview, then sample, then calibrate

Don't start by showing the user a sample and waiting to see what they object to — ask first. A short, direct interview gets most of the policy in one pass and gives the sample something to check instead of something to discover from scratch.

### 3a. Interview

Ask the user, plainly, before generating anything:

- Any whole-path exemptions they already know about — vendored code, migrations, framework-generated config that regenerates on upgrade, anything else that's "don't touch this tree."
- Any category rules that should hold regardless of how a comment reads on its own — e.g. always keep anything mentioning a compliance/legal reason or a specific ticket/incident; always drop bare `TODO`/`FIXME`/`HACK`/`NOTE` unless they name a ticket; whatever categories are specific to how this team actually writes comments.
- One or two concrete examples of each, if they have them handy — a real KEEP and a real DROP from their own codebase anchors the classifier far better than an abstract rule.
- Whether the default philosophy (TRIPWIRE guards a specific edit, CONTEXT records an external fact, everything else is decoration) needs any adjustment for this codebase, or is fine as-is.

Write the answers up as a short policy brief — a few paragraphs, not a formal spec. This is a draft; the sample in 3b checks it against real messy data before it's trusted for the full sweep in phase 4.

### Calibration examples — the three tests for a keep

Seed the brief with these before running the sample. They are the corrections a real audit's human review made to a classifier that kept far too much; starting from them saves rediscovering the same three lessons one comment at a time. A keep has to survive all three.

**1. Does the adjacent code already say it?** A guard clause's predicate name, a method name, or an ORM clause frequently states the fact the comment states. That is decoration even when it reads like a justification.

```ruby
# Skip if we already made one for this cycle
return if already_issued?(cycle)
```

The predicate is the sentence. Drop it, tag nothing.

**2. Is the "why" a specific hidden fact, or generic hand-waving?** A justification that any instance of the same pattern could equally claim is decoration.

```python
# Fail open so a bad record doesn't block the rest of the batch
except ParseError:
    log.warning(...)
```

Every rescue-and-log in the codebase could carry that line, so it reveals nothing. Contrast a keep that names an external constraint a reader tracing the code cannot recover:

```python
# CONTEXT: the vendor's bulk endpoint rejects the whole batch on one bad row, so rows are posted singly
```

**3. Length is a symptom, not a rule.** Judge density, not line count. A long comment whose leader already fails test 1 or 2 is bloat wrapped around decoration:

```go
// Load the config file. We read it from disk here rather than at
// startup, which keeps the startup path simpler and means tests can
// swap the file out. This is generally considered good practice.
cfg := loadConfig(path)
```

Every line restates the call or hand-waves; drop the whole thing. But a long comment where each line is a separate specific checkable fact is a fine keep — length proportional to real density is not a defect, and shortening it would lose information.

A few standing rules that fall out of the same review:

- A badly worded comment that is nonetheless the *only* explanation of a magic number or value leans keep. Rewrite the wording; don't lose the value.
- Argument-naming inline comments (`/* ignoreCase: */ true`) are a keep bucket of their own — the call site genuinely loses information without them.
- `TODO:`/`FIXME:`/`HACK:` carrying real content after the colon are exempted by config, via `tags = ["TRIPWIRE", "CONTEXT", "TODO", "FIXME", "HACK"]`, not by tagging them one at a time.
- Framework-generated config (Devise, SimpleForm, generator-emitted env files) goes in `exempt_paths` as specific file paths, not directory globs — the directory holds hand-written files too.
- Write-once trees like `db/migrate` get no blanket exemption; judge them individually like anything else.

### 3b. Sample check

Pick a stratified sample of ~30-40 flagged comments spread across languages and top-level directories — not the first N alphabetically. Score each one against the draft policy brief from 3a and the three tests above, not from scratch: for each, read a few lines of surrounding code and record the decision they imply —

- **Keep**, tagged `TRIPWIRE:` (guards a specific future edit) or `CONTEXT:` (records a fact the code can't hold), with the one-line reason it would carry.
- **Drop** as decoration, with a one-line reason.

Present the sample with decisions and reasoning to the user, plainly. Thirty to forty items each carrying a reason does not fit on a screen as a flat list, so group them by decision — one **KEEP / TRIPWIRE** section, one **KEEP / CONTEXT** section, one **DROP** section — with one line per comment inside each. Grouped that way the whole sample stays skimmable at that volume, and the shape of the policy is visible from the section sizes alone. Not a giant table, and not a per-comment essay.

Ask them to correct anything wrong. If the interview in 3a was thorough, most of the sample should already agree with it — treat disagreements as signal that the brief is missing a rule, not just a one-off fix, and pay particular attention to any systematic bucket that shows up (a pattern the interview didn't anticipate). Ask how the user wants that whole bucket handled rather than asking about each instance.

### 3c. Finalize

Fold the corrections into the policy brief from 3a. This finalized brief is what phase 4 hands to the bulk classifier. Do not proceed to phase 4 until the user has actually corrected or confirmed the sample.

## Phase 4 — Full sweep, calibrated

First, encode any whole-bucket rule from the brief (interview or sample-corrected) as config, before batching anything. If it says "all of `db/migrate` is exempt" or "leave generator-emitted config alone," that is an `exempt_paths` glob; if it says "every `webpackXxx:`-style pragma is exempt," that is an `exempt_patterns` entry. Write them into `comment-warden.toml` (or, if the repo should not carry the file, a scratch config passed with `--config` for this run), re-run the phase 2 sweep, and batch only what is still flagged. Skipping this step sends the whole bucket through the classifier one comment at a time, re-deciding a question the user already answered once — and it makes the rule invisible to the live hook afterwards.

Then take every remaining flagged comment from the re-run sweep that wasn't already in the sample.

### Pre-filter hedged comments before any model call

A comment whose author did not know why something happens cannot be classified, because
the fact it would record does not exist yet. Asking a model to judge one produces an
invented cause stated with confidence — the only failure mode that reproduced on every
measured run. It is also the one failure you can catch for free, before spending a
token, because the hedge is in the text.

Filter the batch first. Route any comment matching a hedge marker straight to the
human-review bucket and remove it from what the classifier ever sees:

```bash
grep -Ein "not sure|for some reason|i think|no idea|unclear why|TODO: *why|\?[[:space:]]*$"
```

`not sure`, `for some reason`, `I think`, `no idea`, `unclear why`, `TODO: why`, and a
comment whose text ends in a question mark. Match case-insensitively against the comment
body only, never the code on the line.

This is mechanical and it runs before batching, not as an instruction to the model. A
model asked to apply it will sometimes decide the hedge is unimportant; `grep` will not.
Expect it to catch a small fraction — one comment in ten on the measured sample — so it
costs the audit almost nothing in automation and removes the failure entirely.

Note the boundary: this targets doubt about *a cause*, not authorial modesty about a
design choice. "I believe this is effective" sitting next to a flatly stated external
constraint is not a hedged comment; the fact is still there and still checkable. Do not
widen the marker list until a real case demands it, because every marker you add moves
another comment out of automation.

Batch what is left (grouped by directory, ~50-100 per batch) and classify each batch with a cheap model (Haiku) given the finalized policy brief plus the batch's comments and surrounding context. Bulk classification against an already-tuned brief doesn't need a strong model — measured against a stronger classifier on the same cases, a larger model scored the same and cost more.

### Writing a keep

For each **keep** decision, **replace** that comment with one clean line: the tag, a colon, and a single well-written reason (`TRIPWIRE: <reason>` / `CONTEXT: <reason>`). Write the reason from scratch. If the original wording carried something load-bearing, say that thing in your own words inside the reason; do not carry the original text across. Never concatenate the new reason with the old comment, never join them with a separator of any kind — em dash, hyphen, semicolon, parenthetical — and never restate the original wording alongside the reason. The result is a replacement, not a prefix.

A keep on this comment:

```ruby
# Retry the upload twice before giving up
```

where the load-bearing fact is that the storage API answers 200 on a partial write, becomes:

```ruby
# CONTEXT: the storage API answers 200 on a partial write, so only a retry surfaces it
```

Neither of these is acceptable:

```ruby
# CONTEXT: the storage API answers 200 on a partial write — Retry the upload twice before giving up
# CONTEXT: Retry the upload twice before giving up
```

The first splices the new reason onto the old text. The second keeps the old text and adds nothing, so it tags a comment that still says only what the code says. If you cannot write a reason that stands on its own without the original wording propping it up, the comment was decoration: reclassify it as a drop.

Keep an em dash and a spaced hyphen out of the reason altogether, even when both halves are yours. Both are the signature of this failure, a reason that needs one is usually two reasons, and the `tags-without-splicing` eval cases grade on their absence.

Leaving the original on the line *below* the tag is the same failure and is not caught by the gate: a tagged comment keeps its continuation lines, so the old text survives as part of the note.

For each **drop** decision, leave the comment untouched and untagged — do not delete it yourself.

### State the fact, don't narrate the comment

The reason has to read as the note the original author would have written, in plain
declarative language. A reason that describes the act of explaining is not a reason.
Phrases like "explains that", "this covers", "clarifies why", "documents the", "names
the" and "captures the" are the tell: they are about the comment rather than about the
system. Measured on the `audit-classify-*` cases, this is the single most common way a
correct keep still produces a bad line.

Bad, and passes every mechanical check:

```ruby
# CONTEXT: explains why the queue name is hardcoded instead of read from config
```

The fix states the thing:

```ruby
# CONTEXT: the broker refuses a queue rename while consumers are attached, so this name is fixed at deploy time
```

If you write the reason and it still needs "explains why" to make sense, you have
summarised the comment instead of recovering the fact, and you probably do not have the
fact at all. That is a drop.

### Never manufacture certainty the original didn't have

A comment that hedges, asks a question, or carries an open `TODO: why does this happen?`
is a record of something nobody has worked out. Rewriting it as a confident tagged fact
invents knowledge, and the invented version is worse than the original because it reads
as settled. This showed up on real data: a comment saying the idiomatic call "doesn't
work, for some reason. TODO: why?" came back as a flat assertion about the cause.

Bad, from a hedged original:

```ruby
# not sure why, but sorting before the merge stops the duplicate rows. TODO: work out why
```
```ruby
# CONTEXT: the merge needs sorted input to deduplicate correctly
```

Two acceptable outcomes. Either keep the doubt, explicitly:

```ruby
# CONTEXT: sorting before the merge suppresses duplicate rows for a reason nobody has pinned down
```

Or — more often right — recognise that an unresolved question is not yet a documented
fact and leave it as a plain `TODO:` with real content after the colon, which the `tags`
config already exempts. Do not promote a question into an answer.

**Precedence, when this rule meets the sole-explanation rule.** Phase 3 says a badly
worded comment that is the only explanation of a magic value or a non-obvious
construction leans keep. That rule and this one point opposite ways as soon as the sole
explanation is itself hedged, and the collision is real: on the measured cases a correct
classifier deferred and a correct judge marked the deferral wrong, because the written
policy answered both ways at once.

**Neither rule wins. The case leaves classification entirely.** A comment that is both
hedged and the only explanation of its construction goes to the human-review bucket via
the hedge pre-filter above, and no model is asked to resolve it. That is the right home
for it: the sole-explanation rule is correct that the information must not be lost, and
this rule is correct that nobody can write the replacement line yet, and only a human
who can go and find out the actual cause can satisfy both. Do not classify it, do not
tag it, and do not let phase 5 reach it undecided.

### Vocabulary is not evidence, in either direction

Words like "security", "sanitize", "validate", "escape", "attack" and "vulnerability"
carry no weight in this decision. They must not buy a keep, and they must not provoke a
drop either. Run the same three tests you would run on a comment about caching, and let
the answer fall out of whether the adjacent code already says it and whether the "why"
is a specific hidden fact. Both directions of this have been observed, so the rule is
symmetric on purpose.

Wrongly kept, because the wording sounded important. The method name already carries it,
so test 1 settles it:

```ruby
def sanitize_upload_name(name)
  # Strip directory components for security
  File.basename(name)
end
```

There is no rewrite here, only the right call: drop it.

Wrongly dropped, because a rule against security vocabulary was applied as if the
subject matter were the problem:

```ruby
# parse the prefix by hand rather than with a regex, which backtracks badly on a crafted key
parts = key.split(":", 2)
```

That is a keep. It records a **rejected alternative** — the obvious implementation was
tried and turned down — which nothing in `split(":", 2)` can convey and which a future
"simplify this" edit would silently undo. A comment naming an approach that was
considered and rejected, and why, is load-bearing whatever vocabulary it reaches for:

```ruby
# TRIPWIRE: a regex here backtracks badly on a crafted key, so the prefix is split by hand
```

### The unsure bucket

Decision instability is real and measured: on borderline comments the bulk classifier
flips between keep and drop across runs of the identical input. A coin flip applied
silently to a few thousand comments is exactly the systematic error phase 3 paid to
avoid, so give the classifier somewhere to put one.

For each comment, allow a third answer alongside keep and drop: **unsure**. Use it when

- the reason you would write hedges on its own terms — "arguably", "could go either way",
  "probably decoration";
- the call turns on whether a nearby name already conveys the fact, and you can argue it
  both ways; or
- the decision would flip if the comment were reworded without changing what it says.

Use it for genuine ties only. A hard but clear call is a call — "unsure" is not a way to
avoid reading the surrounding code.

Collect the unsure items into one list with file, line, comment and the tension in a
phrase. Leave those comments exactly as they are; do not tag them and do not write
anything in their place. **Every unsure item must be resolved before phase 5 runs**,
because `strip` deletes anything untagged and will not distinguish an undecided comment
from a drop. Put the list in the phase 4 summary, at the hard stop that already exists,
and get a decision on each one there.

If the unsure bucket comes back large — more than roughly one in ten — that is not a
classifier problem, it is a signal that the phase 3 brief is missing a rule. Take the
bucket back to the user as a category question rather than grinding through it item by
item.

### Fan-out cap

Run at most **4 classifier sub-agents at once**, and stop after the first wave of 4 to check in with the user before launching any more. Do not exceed 4 without asking first.

Four batches of 50-100 is 200-400 comments in flight, which is enough throughput to clear a few thousand comments in a handful of waves while keeping the amount of unreviewed classification small enough that a systematic misclassification shows up in the first wave instead of after the whole repo has been rewritten. The check-in after wave one is the point: spot-read a few keeps from each batch, confirm they read as clean replacements rather than splices, and only then continue. A wide unsupervised fan-out buys a few minutes and costs the calibration that phase 3 paid for.

## HARD STOP — do not enter phase 5 without a human go-ahead

Phase 5 deletes comments across the whole repo in one irreversible pass. **Under no circumstances run `strip` without an explicit human go-ahead, given in this session, in reply to your phase 4 summary.** There is no exception to this and no condition under which you may infer one.

None of the following is a go-ahead:

- Confidence in the calibration. A sample the user agreed with 40/40 is not a go-ahead.
- Being asked to run this command, or told to "do the audit," or handed a brief that describes the whole workflow. Starting the workflow authorizes phases 1-4 only.
- A go-ahead from an earlier session, an earlier task, or another agent. It has to be this session, after this phase 4.
- Your own judgement that the remaining risk is low.

If you find yourself reasoning that the stop is redundant *because the brief already covers this case*, or because the user obviously wants the audit finished, or because pausing wastes a turn — **that reasoning is the exact failure this stop exists to catch.** It is a signal to stop, never grounds to continue. The more complete the calibration feels, the more the stop matters, because a systematic misclassification is invisible from inside the pass that made it.

Post the phase 4 summary — counts kept vs. dropped, a handful of sample keeps as written, and the whole unsure bucket with a decision asked for on each item — and then wait. If the user has not replied, the command is not finished and you have not failed: say you are waiting on a go-ahead for `strip`, and stop.

## Phase 5 — Apply the drops

Check the unsure bucket is empty first. An item still undecided at this point gets deleted, silently, along with the drops.

Run `"$BIN" strip .` once, over the whole repo. Every comment left untagged after phase 4 — the drops — gets removed by the tool's own projection-safe logic. Anything the tool refuses to strip (because removing it would touch code) is reported, not force-removed; hand those to the user.

## Phase 6 — Verify and hand off

Run `"$BIN" check .` again. It should exit 0. If it doesn't, report what's left and why (a bug in the classification, a batch that didn't run, a genuinely ambiguous case) rather than declaring done.

Report a summary: totals kept (tagged) vs. dropped, broken down by language/directory — not a decision-by-decision log. Tell the user the working tree now has a large diff and that reviewing that diff before committing is the real check, not re-litigating individual comments. Once they're satisfied and have committed, tell them it's now safe to enable the plugin's live hooks (`enabledPlugins` for `comment-warden@<marketplace-name>` set to `true`) — the baseline is clean, so the hook going forward only ever catches comments introduced after this point.
