---
type: regex
target: { source: file, path: throttle.rb }
pattern: '[Ss]leep between batches|[Bb]atch size is 500 here'
match: not_contains
---
