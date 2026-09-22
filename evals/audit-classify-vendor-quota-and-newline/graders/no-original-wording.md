---
type: regex
target: { source: file, path: invoice_exporter.rb }
pattern: 'NOTE\(jlq\): Batching a whole page into'
match: not_contains
---
