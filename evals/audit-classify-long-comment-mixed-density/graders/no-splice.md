---
type: regex
target: { source: file, path: cart_totals.rb }
pattern: '(TRIPWIRE|CONTEXT):[^\n]*(—|–|--| - )'
match: not_contains
---
