---
type: regex
target: { source: file, path: gcd.rs }
pattern: "//(?!/)(?!.*(TRIPWIRE|CONTEXT):)"
match: not_contains
---
