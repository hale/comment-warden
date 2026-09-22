# comment-warden evals

The advice prompt exists to get one outcome: the model writes only load-bearing comments, tags them `TRIPWIRE:`, and then stops thinking about comments. These evals measure whether the plugin gets there. They are also how the default advice prompt gets tuned. Run the suite over prompt variants and ship whichever one wins.

## Run

You need Claude Code v2.1.269+ (`claude update`), and the binary has to be bundled where the hook looks for it:

```bash
nix develop --command bash -c 'cargo build --release'
mkdir -p bin && cp "$(nix develop --command bash -c 'echo $CARGO_TARGET_DIR')/release/comment-warden" bin/comment-warden

claude plugin eval . --allow-tools Read Write Edit --judge-model sonnet
```

The runner refuses to load a plugin directory containing a hard-linked file, with `a file in the plugin has more than one name (a hard link)`. A stale in-tree `target/` from a non-devshell `cargo build` is full of them, and the message names a case rather than the offending file, so it reads as a broken case. `rm -rf target` (the dev shell builds outside the checkout anyway) before blaming the suite.

Each run gets a fresh `CLAUDE_CONFIG_DIR`, so a machine whose credential lives only in `~/.claude/.credentials.json` fails every run with `Not logged in · Please run /login` while an ordinary `claude -p` works. Reproduce it with `CLAUDE_CONFIG_DIR=$(mktemp -d) claude -p hi`; the fix is an API key in the environment, which the allowlist passes through as `ANTHROPIC_*`.

`--allow-tools Read Write Edit` is required. The cases write source files, and the PostToolUse strip hook only fires on a real Write/Edit. Each case runs once with the plugin and once without it, and the `Δ` is what the plugin contributed.

## Cases

There are two kinds of case, and you need to know which one you are looking at before you read its `Δ`.

**Discriminating** cases are ones where the plugin changes the outcome compared with no plugin, so `Δ > 0`:

- **strips-explanatory-comment**: the task asks for an explanatory comment, so the baseline model leaves an untagged one. With the plugin it is stripped. Measured `WITH 1.00 · WITHOUT 0.50 · Δ +0.50`.
- **advice-steers-clean** (`-swift`, `-nix` variants): a genuine hazard, with no mention of any tag. Only the SessionStart advice can teach the model to record it with a `TRIPWIRE:`/`CONTEXT:` tag and write nothing untagged around it. The Rust case is what the default advice was tuned against, and the Swift (`//`) and Nix (`#`) variants confirm the steering is not language-specific. Deterministic strip behaviour across the full language matrix and test-vs-app targets is covered by the Rust test suite (`tests/`) instead of here, because those tests need no model and run in CI.

**Guard-rail** cases check that the plugin doesn't make things *worse*. For these, `Δ ≈ 0` is the passing result, so don't read it as a null result:

- **writes-clean**: a trivial function that the model comments the same way with or without the plugin (i.e. not at all). All this confirms is that the plugin doesn't wrongly flag a clean file.
- **keeps-tagged**: a load-bearing `TRIPWIRE:` survives.
- **string-not-corrupted**: a string literal containing `//` is untouched.
- **no-churn**: after silent stripping, the model finishes without re-adding comments or fixating on them. That distraction is what this design exists to avoid.

**Audit-rewrite** cases check the one decision `/comment-warden:audit` phase 4 makes thousands of times: given a comment already classified as a keep, what text replaces it. They exist because a real bulk audit produced `# CONTEXT: <generated reason> — <original untagged comment>`, splicing the new reason onto the old text instead of replacing it. The plugin cannot catch that — the line is tagged, so the gate retains it, and the gate's continuation-line rule means an original comment left on the *following* line is retained too, as part of the tagged note. Only the phase 4 wording prevents it, so these are regression guards on that wording.

- **tags-without-splicing** (Ruby, `CONTEXT:`), **tags-without-splicing-ts** (TypeScript, `TRIPWIRE:`), **tags-batch-without-splicing** (Ruby, two keeps in one file, the shape of a phase 4 batch). Each gives the model an untagged load-bearing comment plus the fact behind it and asks for the tagged replacement.
- Graded mechanically for the splice signature (no em dash or spaced hyphen after the tag), for verbatim survival of the original wording, for exactly one tag per keep, and for a clean gate; plus an `llm` grader for whether the reason reads as one written sentence rather than two texts glued together.
- Measured on Haiku, the model phase 4 actually uses, at `runs: 3`. With the phase 4 wording that caused the bug — tag and reason "preserving the original text after it" — the three cases scored `0.94 / 0.78 / 0.89`, with `no-splice` failing 1/3, 2/3 and 2/3 and the original wording surviving verbatim on the line below the tag. With the corrected wording all three score `1.00`, and the `llm` grader passes 9/9 while failing all three spliced variants.

**Audit-classify** cases check the decision *before* that one: given an untagged comment and its surrounding code, is it a keep or is it decoration? They encode the three tests written up in `commands/audit.md` phase 3 — does the adjacent code already say it, is the "why" a specific hidden fact or generic hand-waving, and is the length proportional to real density. Ten cases, all Ruby, all invented — every snippet, identifier, library and ticket number is made up, so each case is a self-contained probe of one known failure mode rather than a sample of anyone's code. Five are straightforward (two clean drops and three keeps that each carry a named external fact) and five are deliberately borderline, each sitting on one of the tests. Case bodies live in `harness/cases.json`; `harness/gen_cases.py` emits the `audit-classify-*` directories from it, so edit the JSON and regenerate rather than editing a case dir by hand.

Borderline cases are graded on whether the *reasoning* holds, not on a fixed answer. Their `llm` grader accepts either outcome as long as it is clean — the comment stripped with nothing invented in its place, or one tagged line stating a fact the code does not already convey — and fails a keep whose reason restates the code, is generic enough to fit any instance of the pattern, or preserves narration that should have collapsed to the single hidden fact. Pinning a borderline case to one answer would measure agreement with whoever wrote the case rather than with the policy.

`harness/run_harness.py` is an offline stand-in for the real runner, needed because sandboxed `claude plugin eval` runs cannot authenticate (see the `CLAUDE_CONFIG_DIR` note above). It puts each case to Haiku for a classification, applies that classification to a throwaway copy, grades it with the real binary — gate clean, no splice separator after the tag, no seven-word run of the original wording reused, one tag, one line, and for a drop that `strip` actually removes it — and then puts the *resulting file* to an independent Sonnet judge scored against the three tests. Mechanical and judge grades are recorded separately, because they catch different things: a classification can be syntactically perfect and still wrong about the comment. Results land in `harness/results/`, which is gitignored; headline numbers belong here.

Show the judge the applied file, never just the decision and reason. A first pass handed it only `decision: keep-context` plus the new reason, and it failed two correct keeps for "preserving narration that should have been collapsed" and for hedging wording that had in fact already been replaced — it had assumed a keep retains the original text. Both passed once it could see the file.

Two runs of all ten cases on Haiku, judged by Sonnet, scored `5/10` and `4/10`. Mechanical grading passed 19 of 20; the single failure was a reason returned as `CONTEXT: CONTEXT: …`, caught by `tagged-once` and not noticed by the judge, which failed that case on unrelated grounds. So the two grading paths are not redundant.

The split is sharper than the totals suggest, and it is not where the tests were aimed:

| case | run 1 | run 2 |
|---|---|---|
| restates-method-name, restates-sort-clause, session-string-key-ticket | pass | pass |
| vendor-quota-and-newline, basename-path-traversal, weakly-worded-only-explanation, long-comment-mixed-density | fail | fail |
| order-for-stable-results, handrolled-scan-not-matcher, empty-rescue-justification | pass | fail (flipped decision) |

Every clean drop and the one keep carrying a named external fact plus a ticket held across both runs. Every persistent failure was a keep, and almost all of them failed on the *replacement wording* rather than on the keep/drop call, which is a gap in phase 4 rather than in the three tests:

- **Meta-narration.** Reasons that describe the comment instead of being it — "Explains the vendor API limitation that forced …", "Explains the JSON serialization constraint …". No mechanical check catches this; there is no splice separator and no reused wording.
- **Invented certainty.** Pressed to write a crisp reason for a hedged comment, the model manufactured one: `for some reason … TODO: why?` came back as "breaks when used with category" as settled fact. This is the cost of the standing rule that a badly worded sole explanation leans keep.
- **Security words defeat the tests.** `# Use File.basename to prevent path traversal attacks`, inside a method already named `safe_entry_name`, was kept in both runs against both tests 1 and 2.

Decision stability is its own result: Haiku flipped decision family between runs on three of ten cases, all borderline. At `runs: 3` the real runner will average over that; a single-shot bulk classification in phase 4 will not.

### After hardening phase 4

Phase 4 then grew four rules aimed at exactly those failures — state the fact rather than narrate the comment, never manufacture certainty the original lacked, treat security vocabulary as no evidence at all, and answer `unsure` on a genuine tie. A third run of the identical ten cases scored **`4/10`, the same as run 2**. The headline did not move. Which cases pass did, and that is where the result is:

| rule | evidence |
|---|---|
| vocabulary is not evidence | **worked.** `basename-path-traversal` was kept against both tests in both earlier runs; with the rule it dropped and passed. The only case fixed outright. |
| no meta-narration | **partial.** `vendor-quota-and-newline` had failed on "Explains the vendor API limitation that…"; it now states the constraint and passes. `handrolled-scan-not-matcher` still narrated ("a future maintainer must know why…"). |
| no manufactured certainty | **no effect.** `weakly-worded-only-explanation` failed all three runs, still asserting a hedged cause as settled. |
| the unsure bucket | **never fired.** Zero uses across ten cases, including four borderline ones that flip between runs. The model does not know when it is torn, so the escape hatch cannot be relied on to contain instability. |

Two regressions came with it, and both are the same trade. Pressed to write a direct declarative fact, the model pads the line: `session-string-key-ticket`, an easy keep that passed before, invented a second mechanism ("converts symbols to null", plus a claim about per-request deserialization) that the original never made; `long-comment-mixed-density` picked up a mechanical failure by copying a seven-word run of the source verbatim. Anti-narration pressure buys directness and pays for it in fact inflation.

The dominant remaining failure is **two texts glued together without the punctuation tell** — a generic clause concatenated onto the specific fact, cited by four of the six failures. The `no-splice` grader only catches the em dash and the spaced hyphen, so this is invisible mechanically and only the `llm` grader sees it.

Read the 4 → 4 carefully. With three borderline cases flipping between identical runs, one pass of ten single-shot classifications has error bars wider than the gap between 4 and 5.

## Conclusion: the judge was the variable, not the classifier

Every run above judged with Sonnet, and the "keeps are unreliable" reading they produced does not survive a control. Swapping the **judge** to Opus and changing nothing else — same Haiku classifier, same prompt, same ten cases — moved the score from `4/10` to `7/10`. Most of the measured keep-unreliability was judge strictness. Read the sections above as a record of how the harness was built, not as findings about the model.

With the judge held at Opus and only the classifier varied:

| classifier | overall | reference-keep cases | emitted keeps that hold up |
|---|---|---|---|
| Haiku | 7/10 | 4/6 | 3/4 |
| Sonnet, run a | 7/10 | 4/6 | 3/4 |
| Sonnet, run b | 8/10 | 4/6 | 3/3 |

Sonnet does not beat Haiku here. The reference-keep bucket is 4/6 for all three. **This is task-inherent difficulty, not a small-model ceiling**, and it does not justify paying for a larger model on the keep calls.

Sonnet is better in one specific, non-scoring way: calibration. On `weakly-worded-only-explanation` — the hedged `for some reason … TODO: why?` comment that Haiku resolved into invented certainty in all three earlier runs — Sonnet answered `unsure` both times and named the hedge as its reason. The `unsure` bucket fires for Sonnet and never fires for Haiku. Meta-narration and the semantic splice are largely absent from Sonnet's keeps. Stability improves without arriving: of the three cases that flipped under Haiku, two settle under Sonnet while `order-for-stable-results` still flips, and Sonnet adds its own `TRIPWIRE`/`CONTEXT` tag wobble on cases whose keep/drop call is stable.

**The two failures that persist across both Sonnet runs are disagreements about the policy, not model errors — and both are collisions this project's own advice created.**

- `handrolled-scan-not-matcher`: Sonnet drops it citing "vocabulary is not evidence"; the judge calls it a textbook `TRIPWIRE` recording a rejected alternative. Both read the written policy correctly. The vocabulary rule, added to stop security words buying a keep, now also suppresses genuine rejected-alternative keeps that happen to be about security.
- `weakly-worded-only-explanation`: "never manufacture certainty" says do not resolve the hedge; "a badly worded sole explanation leans keep" says keep it anyway. The policy does not say which wins, so a correct classifier defers and a correct judge marks the deferral wrong.

Both collisions were then fixed in phase 4 and the dataset re-run. The vocabulary rule is now symmetric — security words neither buy a keep nor provoke a drop, and a comment recording a *rejected alternative* is load-bearing whatever vocabulary it uses — and the certainty collision is resolved by precedence: a comment that is both hedged and the only explanation of its construction leaves classification altogether and goes to a human.

### Final state

A deterministic hedge pre-filter now runs in phase 4 **before any model call**, matching `not sure`, `for some reason`, `I think`, `no idea`, `unclear why`, `TODO: why`, and a comment body ending in a question mark. It fired on exactly one of the ten cases in all three final runs, with **zero model calls spent on it** — the harness counts calls per case to prove that rather than assume it.

| run | overall | classified | reference-drop | emitted keeps that hold |
|---|---|---|---|---|
| Haiku a | 7/10 | 6/9 | 4/4 | 2/5 |
| Haiku b | 7/10 | 6/9 | 4/4 | 2/5 |
| Sonnet | 8/10 | 7/9 | 4/4 | 2/4 |

What the fixes achieved, precisely:

- **The certainty collision is gone.** `weakly-worded-only-explanation` failed in all five earlier runs, under both classifiers and both judges. It now never reaches a model. This was the one reproducible failure and it is closed.
- **The vocabulary collision is half gone.** Before the rewording Sonnet dropped `handrolled-scan-not-matcher` twice; all three final runs now keep it, so the rule no longer suppresses a rejected-alternative keep. The keep's *wording* still fails the judge in all three, but for the ordinary reason keeps fail — a generic rationale padded with narration — not because two rules contradict. The policy contradiction is resolved; the writing problem is not, and no wording change has moved it in three attempts.
- **Drops are now perfect and stable.** `4/4` on the reference-drop bucket in every final run, across both classifiers.
- **Keeps hold up about two times in five.** This is the residual and it has survived every intervention.

Mechanical and `llm` grading stayed complementary to the end: in the final runs the mechanical checks failed two lines the judge had passed (verbatim reuse of a seven-word run from the original, twice). Neither path alone is sufficient.

### Recommendation

1. **Haiku throughout, no tier.** With the judge held constant, Sonnet scores within noise of Haiku and costs more. The one thing Sonnet did better — declining to resolve a hedged comment — is now handled by the pre-filter for free.
2. **Run the hedge pre-filter before batching.** Mechanical, not an instruction to a model. It costs about one comment in ten of automation and closes the only failure that reproduced every time.
3. **Trust the drops in bulk.** They are the volume and they have been correct and stable across every run and both classifiers.
4. **Read every emitted keep.** Keeps are roughly a third of the flagged set and only about two in five come back with a line worth committing. This is a real, bounded review cost, not a defect to be wordsmithed away — three rounds of rule changes did not move it.
5. **Keep both graders.** The mechanical checks catch splices and verbatim reuse the judge waves through; the judge catches generic rationales and narration the regexes cannot see.

The bottleneck is writing a good replacement line, not deciding keep from drop. Anyone extending this should aim there, and should not expect a prompt change to fix it.

A first full run measured `Δ 0.00` on all four guard-rail cases. That is expected: the baseline output is already clean, so the most the plugin can do is match it. The plugin's value only shows up where the baseline *would* leave an untagged comment, and that is what the discriminating case measures. To tune the advice prompt, add more discriminating cases (tasks that provoke comments) and compare `Δ` across prompt variants.

## Tuning the advice prompt

The default advice is picked by eval, and taste doesn't come into it. Each candidate goes to the model as its only guidance, via `append_system_prompt` on a plugin-free run so that nothing but the wording shapes the output. The model writes a hazard-bearing file, and the raw output is scored with `comment-warden check`. That is the real binary, and only it gets block-grouping and doc-comment exemptions right (an LLM judge does not). A variant scores when the file records the hazard with a tag and also passes the gate clean.

Tuning (six runs each, scored on the real binary):

| candidate | records hazard | passes gate clean |
|---|---|---|
| terse, tags only | 6/6 | 2/6 |
| + name `///` docs as strippable | 6/6 | **6/6** |

Every variant records the hazard, so the difference between them is churn. Two failure modes cost the baseline. The model does not infer that "comment" includes a `///` doc comment, and it writes multi-line hazard notes.

The first is the model's to fix, and naming `///` explicitly closes it. That one clause is the shipped `DEFAULT_ADVICE`, and it generalised to a second `CONTEXT:` probe (6/6).

The second was the *gate's* to fix. A tagged leader now keeps its continuation lines (see SPEC.md), so a multi-line note survives whole and the advice needs no rule about it. Before that fix, reaching 6/6 also took an extra "keep each tag to one line" clause. The clause became dead weight once the gate stopped truncating notes, so it was dropped. `advice-steers-clean` guards the result through the real plugin path.
