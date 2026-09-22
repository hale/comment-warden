---
type: regex
target: { source: file, path: invoice_exporter.rb }
pattern: '(TRIPWIRE|CONTEXT):[^\n]*(—|–|--| - )'
match: not_contains
---
