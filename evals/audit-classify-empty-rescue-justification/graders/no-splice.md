---
type: regex
target: { source: file, path: chart_palette.rb }
pattern: '(TRIPWIRE|CONTEXT):[^\n]*(—|–|--| - )'
match: not_contains
---
