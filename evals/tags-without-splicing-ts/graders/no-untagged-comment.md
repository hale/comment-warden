---
type: regex
target: { source: file, path: cacheKey.ts }
pattern: '//(?!/)(?!.*(?:TRIPWIRE|CONTEXT):)'
match: not_contains
---
