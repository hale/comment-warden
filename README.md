# comment-warden

Comment what the code cannot say. Delete the rest.

Some comments hold weight and the rest are decoration. A comment that holds
weight does one of two jobs. It stops a specific future edit from silently
breaking something, or it records a fact about the world outside the code, like
an upstream bug or a wire-format quirk, that no name, type, or test can hold.
Anything else is decoration, and decoration drifts because nothing checks it.
Most of it points at a naming or shape problem that is better fixed in the code,
and the reasoning behind a design belongs in the commit or the ADR. Rust already
works this way for one hazard: every `unsafe` block must carry a `// SAFETY:`
comment that justifies it. comment-warden applies the same bargain to comments in
general, and it enforces the rule from the AST so it doesn't come down to taste.

Two tags survive. Every other comment is stripped silently as you write:

- **`TRIPWIRE:`** stops a specific edit from silently breaking something.
  `// TRIPWIRE: delete this sleep and retries hammer the server`
- **`CONTEXT:`** records a fact the code cannot encode.
  `// CONTEXT: upstream returns 200 on failure, so we check the body`

Doc comments that a codegen tool copies out of the build are also kept. That list
is configured rather than guessed, and both the tags and the exemptions can be
overridden per repo.

Parsing is done with tree-sitter and never with a line scanner, so `//` inside a
string is not treated as a comment, and a `#` shebang that a build turns real is
not stripped. Stripping is **projection-safe**: after removing a comment, the tool
re-parses and requires every non-comment token to be byte-identical. If a
deletion would touch code, the tool refuses it and makes no guess.

## Install (as a Claude Code plugin)

```
/plugin marketplace add hale/comment-warden
/plugin install comment-warden
```

The plugin runs two hook events:

- **SessionStart** states the rule once, so silent stripping is never a surprise.
  On first run it also fetches the `comment-warden` binary for your platform into
  the plugin's data dir. The repo is public, so the download needs no auth. If
  you would rather supply the binary yourself, see *Provide the binary* below.
- **PostToolUse** runs after you Write or Edit a file and strips untagged
  comments in place. It is silent apart from, at most, a one-line factual
  receipt, which is there so an in-flight edit doesn't fail against a file that
  changed under it.

There are also two commands for a whole-repo sweep: `/comment-warden:check` and
`/comment-warden:strip`.

### Provide the binary

The plugin tree ships no binary. The SessionStart hook fetches it, as described
above. If you want to provide it another way, the hook looks in order at
`$COMMENT_WARDEN_BIN`, `$CLAUDE_PLUGIN_DATA/bin/comment-warden`, the plugin's own
`bin/comment-warden`, then `PATH`. You can build it with
`nix build github:hale/comment-warden` (or `cargo build --release`), or download
the archive for your platform from the latest
[release](https://github.com/hale/comment-warden/releases). The repo is public, so
no auth is needed either way. If no binary is ever found, the hook says so and
does not pretend the gate ran.

## Use it directly

```
comment-warden check <paths>...   # list untagged comments; exit 1 if any
comment-warden strip <paths>...   # remove them in place; exit 1 if any could not be removed safely
comment-warden hook               # reads a PostToolUse payload on stdin (what the hook calls)
```

Exit codes:

- `0` means clean.
- `1` means an untagged (or unstrippable) comment.
- `2` means nothing matched. Over a tree, that tells you the path list drifted.
  It does not tell you the tree is clean.
- `3` means the checker could not run. A `3` is never a verdict on your comments.

## Configure

Everything project-specific lives in `comment-warden.toml` at the repo root.

**The tags.** `TRIPWIRE` and `CONTEXT` are the defaults, and you need no config to
use them. If you want your own words, set `tags`. The list *fully replaces* the
defaults and nothing else has to change: no code, no re-registration.

```toml
tags = ["GUARD", "BECAUSE"]
```

**Doc-comment exemptions.** This says which doc comments a codegen tool copies
into an artifact that an external consumer reads. Those must survive:

```toml
[[doc_surface]]
lang = "rust"
item_attributes = ["Object", "ComplexObject", "SimpleObject", "Enum", "uniffi::export"]
```

With no config, every doc comment is judged like any other, which is the honest
default. `pub` grants nothing, and a doc comment in a test target is never a
surface.

**The advice prompt.** The SessionStart hook injects a short instruction so the
model writes only tagged, load-bearing comments in the first place. Override it
with `advice = "..."` or `advice_file = "docs/comment-policy.md"`.

## Supported languages

Rust, Swift, Nix, Bash, HCL, Python, TypeScript, JavaScript, Go, C, C++, YAML,
TOML, CSS. Adding one takes a grammar crate plus a registry row. (JSON and
Markdown are intentionally absent. JSON has no comments, and the block-level
Markdown grammar surfaces no comment node to act on.)

Nix gets special handling. A shell script written inside a `''…''` string is
re-parsed as bash so its `#` comments are seen. A comment found there is only
ever reported and never stripped, because editing a string's bytes could change
the program.

## Build from source

```
nix build            # or: cargo build --release
```

The repo is a Nix flake (fenix + crane). `nix develop` drops you in a shell with
the toolchain, and `nix flake check` runs fmt, clippy, tests, and `cargo audit`.

## Dogfooding

This repo runs comment-warden on itself, and it doubles as the worked example of
wiring the tool by hand instead of going through `/plugin install`.
`.claude/settings.json` registers the strip and advice hooks, so a Claude Code
session working here gates the code it writes. For the hook to find the binary,
build it (`nix build` or `cargo build --release`) and set `COMMENT_WARDEN_BIN`, or
drop it in `bin/`. On top of that, CI gates `src/` on every run via the
`comment-warden-self-check` flake check, so the crate has to pass its own rule to
build.

## Why two tags, not one — or ten

The gate fires on the *untagged*. It never reads which tag it found, so adding
more tags buys the mechanism nothing at the check itself. There are two because
there are two honest reasons a comment survives: it guards an edit (`TRIPWIRE:`)
or it records an external fact (`CONTEXT:`). Naming them apart keeps authors
honest about which one they mean. A tag fixes untagged comments and does nothing
about dishonest ones. A hollow `TRIPWIRE:` on something that guards nothing is a
review-time judgement that no tool can make.
