---
type: regex
target: { source: file, path: sessions_controller.rb }
pattern: '(TRIPWIRE|CONTEXT):[^\n]*(—|–|--| - )'
match: not_contains
---
