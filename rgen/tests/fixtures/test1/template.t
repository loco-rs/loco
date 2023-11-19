---
to: tests/fixtures/test1/generated/{{name}}.txt
injections:
- into: tests/fixtures/test1/generated/prepend.txt
  prepend: true
  content: "this was prepended"
- into: tests/fixtures/test1/generated/append.txt
  append: true
  content: "this was appended"
- into: tests/fixtures/test1/generated/skipped.txt
  skip_if: "be skipped"
  append: true
  content: "this was appended"
- into: tests/fixtures/test1/generated/before.txt
  content: "// doc comment"
  before: "pub class"
- into: tests/fixtures/test1/generated/after.txt
  content: "field: integer"
  after: "pub class"
---

hello, this is the file body.

variable: {{ name | pascal_case }}
