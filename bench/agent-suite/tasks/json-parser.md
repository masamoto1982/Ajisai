# Task: json-parser

Ported task (mirrors the JSON exercises used in external AI-language
comparisons). Tests parsing a JSON document, querying and transforming it,
and re-serializing to a canonical form.

## Background (language-independent)

You are given JSON text and must parse it, answer questions about it, and
emit canonical JSON. Ajisai ships a JSON facility (the `JSON` module:
`JSON@PARSE`, `JSON@STRINGIFY`, `JSON@GET`, `JSON@HAS`, `JSON@SET`,
`JSON@DELETE`, `JSON@KEYS`, …), so the task is to *compose* those operations
correctly — including operand order on the stack — rather than to write a
character-level parser by hand. `'JSON' IMPORT` makes the module words
available.

Canonical form: `JSON@STRINGIFY` emits objects with keys in input order, no
insignificant whitespace, and arrays as `[a,b,c]`. Round-tripping
`JSON@PARSE` then `JSON@STRINGIFY` normalizes whitespace.

## Solution contract

Write an Ajisai source file that imports `JSON` and defines:

- `ROUNDTRIP` — given a JSON string, leave its canonical string form
  (parse then stringify).
- `FIELD-JSON` — given a *parsed* value and a key, leave the canonical
  string form of that field's value (`JSON@GET` then `JSON@STRINGIFY`).
- `WITHOUT` — given a *parsed* object and a key, leave the canonical string
  form of the object with that key removed.
- `HAS-KEY` — given a *parsed* object and a key, leave the truth string
  (`JSON@HAS`).

Invocations may call `JSON@PARSE` directly to produce the parsed value the
last three words expect (your solution's `'JSON' IMPORT` covers them).

## Acceptance cases (13)

All comparisons are against the final stack as displayed (Ajisai strings
render inside single quotes, e.g. `'{"a":1}'`).

| id | invocation | expected |
|---|---|---|
| rt-object | `'{"a":1,"b":2}' ROUNDTRIP` | `'{"a":1,"b":2}'` |
| rt-array | `'[10,20,30]' ROUNDTRIP` | `'[10,20,30]'` |
| rt-nested | `'{"x":{"y":5}}' ROUNDTRIP` | `'{"x":{"y":5}}'` |
| rt-whitespace | `'  {  "a" : 1 }  ' ROUNDTRIP` | `'{"a":1}'` |
| rt-number | `'42' ROUNDTRIP` | `'42'` |
| rt-string | `'"hi"' ROUNDTRIP` | `'"hi"'` |
| rt-bool | `'true' ROUNDTRIP` | `'true'` |
| field-scalar | `'{"a":1,"b":2}' JSON@PARSE 'b' FIELD-JSON` | `'2'` |
| field-nested-object | `'{"x":{"y":5}}' JSON@PARSE 'x' FIELD-JSON` | `'{"y":5}'` |
| without-key | `'{"a":1,"b":2}' JSON@PARSE 'a' WITHOUT` | `'{"b":2}'` |
| without-missing | `'{"a":1}' JSON@PARSE 'z' WITHOUT` | `'{"a":1}'` |
| has-present | `'{"a":1}' JSON@PARSE 'a' HAS-KEY` | `'TRUE'` |
| has-absent | `'{"a":1}' JSON@PARSE 'z' HAS-KEY` | `'FALSE'` |

Known limitation (recorded, not asserted): in the current engine an empty
`{}` or `[]` round-trips to `null`, so those two edge inputs are deliberately
omitted from the cases rather than baking the quirk into an "expected"
answer. The benchmark protocol records such limitations under "error quality"
rather than as task failures.

## Run

```sh
./verify.sh json-parser your-solution.ajisai
```
