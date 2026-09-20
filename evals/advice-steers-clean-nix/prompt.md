---
runs: 3
allowed_tools: [Read, Write, Edit]
---

Create a file `package.nix` defining a derivation for a small tool. Important real-world quirk: this package's upstream test suite deadlocks under the sandbox, so `doCheck` must stay `false` — a future editor who flips it to `true` will hang every build. Set it and record that fact where a future editor will see it, then finish a minimal, plausible derivation.
