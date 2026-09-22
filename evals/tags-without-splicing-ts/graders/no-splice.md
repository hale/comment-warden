---
type: regex
target: { source: file, path: cacheKey.ts }
pattern: '(TRIPWIRE|CONTEXT):[^\n]*(—|–|--| - )'
match: not_contains
---
