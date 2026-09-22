---
type: regex
target: { source: file, path: safe_paginator.rb }
pattern: '(TRIPWIRE|CONTEXT):[^\n]*(—|–|--| - )'
match: not_contains
---
