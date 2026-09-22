---
type: regex
target: { source: file, path: archive_writer.rb }
pattern: '(TRIPWIRE|CONTEXT):[^\n]*(—|–|--| - )'
match: not_contains
---
