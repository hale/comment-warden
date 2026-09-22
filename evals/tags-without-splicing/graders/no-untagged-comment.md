---
type: regex
target: { source: file, path: message.rb }
pattern: '^\s*#(?! frozen_string_literal)(?!.*(?:TRIPWIRE|CONTEXT):)'
flags: m
match: not_contains
---
