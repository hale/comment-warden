---
type: regex
target: { source: file, path: throttle.rb }
pattern: '(TRIPWIRE|CONTEXT):[^\n]*(—|–|--| - )'
match: not_contains
---
