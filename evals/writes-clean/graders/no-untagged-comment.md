---
type: regex
target: { source: file, path: port.rs }
pattern: "//(?!/)(?!.*(TRIPWIRE|CONTEXT):)"
match: not_contains
---
