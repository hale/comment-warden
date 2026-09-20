# comment-warden evals

The advice prompt exists to get one outcome: the model writes only load-bearing comments, tags them `TRIPWIRE:`, and then stops thinking about comments. These evals measure whether the plugin gets there. They are also how the default advice prompt gets tuned. Run the suite over prompt variants and ship whichever one wins.

## Run

You need Claude Code v2.1.269+ (`claude update`), and the binary has to be bundled where the hook looks for it:

```bash
nix develop --command bash -c 'cargo build --release'
mkdir -p bin && cp "$(nix develop --command bash -c 'echo $CARGO_TARGET_DIR')/release/comment-warden" bin/comment-warden

claude plugin eval . --allow-tools Read Write Edit --judge-model sonnet
```

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
