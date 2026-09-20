# comment-warden — design

comment-warden is an AST comment warden. It starts from the view that a comment exists to stop a specific edit from silently breaking something, so a comment that stops no edit gets deleted. The tool enforces that mechanically. `--check` fails when an untagged comment survives, and `--strip` removes them. A comment is kept only when it is tagged (`TRIPWIRE:`/`CONTEXT:`) or exempt.

The behavior below is the contract. Types are chosen so that an illegal state does not compile. Every decision that governs correctness (what is a comment, what is exempt, what may be deleted) has a name and is never inferred from a bare `bool`.

## What a comment is

Parsing is done with tree-sitter and never with a line scanner. A `//` inside a string literal is not a comment. A `#` can be a comment, a shebang a builder turns real, or a directive, sometimes within three lines of one file. Only the AST can tell these apart, so the AST is what decides.

A **block** is one or more adjacent same-column line comments with no code between them. It is judged and stripped as a unit. The concept exists because a language grammar emits each `//` (or `#`) line as its own node, which makes consecutive line comments separate AST nodes. "These three lines are one comment" is a reading convention the grammar cannot express, so the tool has to reconstruct it. (A `/* … */` block comment is already a single node and needs none of this.)

A block takes the verdict of its **first** line, called the leader. An Untagged or Tagged leader absorbs the Untagged line-comment continuations directly below it, provided they are the same comment kind (a plain `//` and a `///` doc line do not merge). An Exempt leader, meaning a shebang, a directive, or a doc-surface `///`, starts its own single-line block and absorbs nothing.

Decision: a tagged leader keeps its continuations. The alternative was for a tagged leader to start its own block, and it was rejected because it truncated every multi-line note to its first line. The natural way to write a hazard is a tagged first line with prose below it, and that prose was silently deleted. The run still stops at any Tagged or Exempt line, so an Untagged leader can never absorb (and thus delete) a tag below it, which is why the rejected form was introduced in the first place. The rule goes by adjacency and knows nothing about intent. A tagged leader absorbs *any* untagged line directly below it, so a comment that merely abuts a tag survives too. That is the accepted price of keeping multi-line notes whole when there is no way to tell a genuine continuation from a coincidental neighbour.

Placement relative to code is a named enum, because the deletion span depends on it:

```rust
enum Placement {
    WholeLine,      // alone on its line(s)
    LeadingInline,  // starts the line, code follows: `/* x */ code`
    Trailing,       // code precedes it, comment runs to end of line
}
```

## The verdict

Every comment resolves to exactly one verdict. There is no `_` arm, so a new exemption must fail to compile until someone classifies it.

```rust
enum Verdict {
    Exempt(Exemption),  // something outside the edit relies on this text
    Tagged,             // opens with a configured tag and a non-empty reason
    Untagged,           // stops no edit — delete
}

enum Exemption {
    Shebang,                 // row 0, `#!`
    GeneratedFile,           // file opens `@generated` on its own first line
    Directive,               // shellcheck / swiftlint / swift-tools-version …
    DocSurface,              // a doc comment a codegen tool copies externally
}
```

The tags default to `TRIPWIRE:` (guards a specific edit) and `CONTEXT:` (records an external fact code can't encode), and are overridable per repo via `comment-warden.toml` `tags = [...]`. The check fires on the *untagged* and never reads which tag it found, so the number of tags plays no part in the mechanism. The count is there for author honesty: two tags exist because there are two honest reasons a comment survives. A hollow tag on something that stops no edit is a review-time judgment the gate cannot make. The gate fixes untagged comments and does nothing about dishonest ones.

## Exemptions are config, not source

`DocSurface` is the one exemption that is *project* policy rather than a universal fact. A `///` is exempt only where a generator copies its text into an artifact an external consumer reads (a GraphQL SDL description, a UniFFI docstring hashed into bindings). That set differs per repo, so it lives in `comment-warden.toml` at the repo root and is never hardcoded:

```toml
# A doc comment is exempt when a codegen tool copies its text out of the build.
# Match by the attribute on the item it documents (or on an enclosing type —
# an enum variant and a resolver method carry no attribute of their own).
[[doc_surface]]
lang = "rust"
item_attributes = ["Object", "ComplexObject", "SimpleObject", "Enum", "uniffi::export", "uniffi::Record"]
```

With no config, no `///` is exempt and every doc comment is judged, which is the honest default for a general repo. `pub` grants nothing, because being reachable within the build does not mean anything outside the build reads it. A doc comment in a test target is never a surface. Where a macro stands between a comment and its expansion the parser cannot decide, so those comments are exempt. Being wrong there deletes live API docs.

The structural exemptions (`Shebang`, `GeneratedFile`, `Directive`) are universal and built in.

## Two pluggable layers, kept separate

There are two ways the behaviour is tuned per repo. One is hard and one is soft, and they must not be conflated:

- **What is mechanically exempt from stripping.** These are the `doc_surface` rules above. This layer is enforcement: a comment matching a rule is never removed.
- **What the model is advised to do.** This is the *advice prompt*, injected once at SessionStart. It only steers. It shapes what the model writes, but the strip is what actually holds.

The advice prompt is the "comment instruction". It ships with a default whose job is to get the model to write only load-bearing comments and tag them from the start, so the strip rarely has anything to do and the model never churns re-adding. **That default is chosen by eval.** The eval suite scores prompt variants on exactly that outcome, and the winner is the shipped default. A repo overrides it in `comment-warden.toml`:

```toml
advice = "..."             # inline override, or
advice_file = "docs/comment-policy.md"
```

`comment-warden advice` reads that and prints the override or the default; the SessionStart hook only execs it. The default lives in the binary as a constant, so a repo with no config still gets the eval-tuned prompt.

## --strip is projection-safe

The load-bearing guarantee is that **stripping removes comments and nothing else.** Deleting an AST comment node cannot move a code byte. An off-by-one could, though, and so could a fused pair of tokens or a deletion reaching into a string that merely looked like a comment. So every deletion is *verified* rather than trusted.

The projection is the byte-text of every non-comment leaf node, in parse order:

```rust
fn code_tokens(lang, source) -> Vec<Vec<u8>>  // leaves, comment kinds excluded
```

A candidate deletion is applied only if `code_tokens` is byte-identical before and after. A comment whose removal would change it (a comment-shaped run inside a string, a region that failed to parse) is left in place and reported. The tool never guesses. Candidates apply highest-offset-first so earlier spans stay valid, and the whole result is re-verified before the file is written.

The deletion span depends on placement. `WholeLine` takes the line, indentation and trailing newline included. `Trailing` takes the run of spaces/tabs before it and keeps the newline. `LeadingInline` takes the comment and the whitespace after it, and keeps the code and its indent.

Consequences that must hold (property tests): `code_tokens(strip(x)) ==
code_tokens(x)`; `strip(strip(x)) == strip(x)`; `--check` is clean after `--strip` save for what was left in place; an `Exempt` or `Tagged` comment never disappears.

## Embedded languages

A shell script written inside a nix `''…''` string is one token to the nix grammar, so its `#` comments are invisible to a nix-only parse. The nix surrounding the string names it a script, through the attribute it binds to or the function it is an argument to. That naming goes by position and never by content. The body is then parsed again as bash over the string's byte-range via `Parser::set_included_ranges`, and the comments found map back to the outer file's coordinates. A body no rule names as a known language is opaque. A comment-shaped line inside it is reported as unclassified, and it is neither parsed nor stripped.

## Languages

A `Language` is the extension's entry in a registry, which is the one place a new language is added:

```rust
struct Language {
    id: LangId,
    grammar: tree_sitter::Language,
    comment_kinds: &'static [&'static str],
    doc_prefixes: DocPrefixes,       // `///` `//!` `/**` `/*!`
    delimiter_re: &'static str,      // for unclassified-line detection
}
```

It ships with the in-scope set (rust, swift, nix, bash, hcl, python) and the common ones (typescript, javascript, go, c, cpp, yaml, toml, css). Grammars are static crates, so adding one is a crate dependency plus a registry row. JSON has no comments and the block-level Markdown grammar surfaces no comment node, so both are omitted rather than shipped inert. A runtime `.wasm` grammar loader is a possible later extension that would drop the recompile step. It is out of scope for v1.

## CLI and exit codes

These are subcommands so that the two the hooks need (`hook`, `advice`) sit beside the two a human runs (`check`, `strip`) without flag soup:

```
comment-warden check <path>...   # list untagged; exit 1 if any
comment-warden strip <path>...   # remove untagged in place; exit 1 if any could not be removed safely
comment-warden hook              # reads a PostToolUse payload on stdin, strips, prints the receipt (what post-edit-strip.sh execs)
comment-warden advice            # prints the SessionStart advice prompt as hookSpecificOutput JSON (what session-start-advice.sh execs)
  --exclude-from <file>        # paths to skip, one per line
  --config <file>              # default: comment-warden.toml at repo root
```

The run returns a result rather than a bool. The exit code is derived from it, and a genuine failure is distinct from a run that completed and found violations:

```rust
enum Outcome { Clean, Violations, NothingScanned }
// Ok(Clean) -> 0 · Ok(Violations) -> 1 · Ok(NothingScanned) -> 2 · Err(_) -> 3
```

- **0** clean · **1** an untagged comment (or an unstrippable one) · **2** nothing matched, which over a whole tree means the path list or extension map drifted and does not mean the tree is clean · **3** the checker could not complete (bad args, unreadable file or config, a missing grammar). A 3 says nothing about whether the tree is clean.

## Errors

Infrastructure failures (io, parse of config) propagate with `anyhow` context that preserves the source, and they surface as exit 3. `Outcome` is not an error, because finding violations is a normal result. Nothing is `map_err(|_| …)`.

## Not in scope for v1

Runtime `.wasm` grammar loading; a hollow-tag detector (a review judgment); tag categories the gate treats differently from one another (it only ever asks tagged-or-not).
