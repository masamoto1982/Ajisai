# Lexicon-emergence run report

## Per generation

| condition | gen | agents | lexicon | correct | mean tokens | tokens ÷ expanded | Core-equivalent (H5) |
| --- | --- | --- | --- | --- | --- | --- | --- |
| bottleneck | 0 | 3 | 0 | 42/42 | 2.2 | 0.086 | 39/42 |
| bottleneck | 1 | 0 | 8 | 0/0 | null | null | 0/0 |
| solo | 0 | 3 | 0 | 42/42 | 1.8 | 0.073 | 42/42 |

## H1 — classes defined independently by two or more agents (22 of 29 classes, 86 Words)

- **6 agents** (G0A, G0B, G0C, S0A, S0B, S0C), joined at d0:
  - `G0A.WORDS` = `' ' TOKENIZE` — split a String on single spaces into a Vector of words
  - `G0B.WORDS` = `' ' TOKENIZE` — split a String on single spaces into a Vector of words
  - `G0C.WORDS` = `' ' TOKENIZE` — split a String on single spaces into a Vector of words
  - `S0A.WORDS` = `' ' TOKENIZE` — split a String on single spaces into a Vector of words
  - `S0B.WORDS` = `' ' TOKENIZE` — split a String on single spaces into a Vector of words
  - `S0C.WORDS` = `' ' TOKENIZE` — split a String on single spaces into a Vector of words
- **5 agents** (G0A, G0B, S0A, S0B, S0C), joined at d0/d1:
  - `G0A.SUM` = `0 [ + ] FOLD` — sum of a numeric Vector as a bare scalar
  - `G0B.SUM` = `0 [ + ] FOLD` — sum of a numeric Vector as a bare scalar
  - `S0A.SUM` = `'XS' BIND XS 0 [ + ] FOLD` — sum of a numeric Vector as a bare scalar
  - `S0B.SUM` = `0 [ + ] FOLD` — sum of a numeric Vector as a bare scalar
  - `S0C.SUM` = `0 [ + ] FOLD` — sum of a numeric Vector as a bare scalar
- **5 agents** (G0A, G0B, S0A, S0B, S0C), joined at d0a/d1:
  - `G0A.MEAN` = `'G0A.XS' BIND G0A.XS G0A.SUM G0A.XS LENGTH /` — arithmetic mean of a numeric Vector
  - `G0B.MEAN` = `'G0B.XS' BIND G0B.XS G0B.SUM G0B.XS LENGTH /` — arithmetic mean of a numeric Vector, bare scalar
  - `S0A.MEAN` = `'XS' BIND XS S0A.SUM XS LENGTH /` — arithmetic mean of a numeric Vector
  - `S0B.MEAN` = `'S0B.XS' BIND S0B.XS S0B.SUM S0B.XS LENGTH /` — arithmetic mean of a numeric Vector
  - `S0C.MEAN` = `'S0C.M' BIND S0C.M S0C.SUM S0C.M LENGTH /` — arithmetic mean of a numeric Vector as a bare scalar
- **5 agents** (G0B, G0A, S0A, S0B, S0C), joined at d1/d0a:
  - `G0B.DEV` = `KEEP G0B.MEAN -` — Vector of each number minus the mean
  - `G0A.DEVS` = `'G0A.DX' BIND G0A.DX G0A.DX G0A.MEAN -` — Vector of each element minus the mean
  - `S0A.DEVS` = `'XS' BIND XS XS S0A.MEAN -` — each element minus the mean
  - `S0B.DEV` = `'S0B.XS' BIND S0B.XS S0B.XS S0B.MEAN -` — Vector of each element minus the mean
  - `S0C.DEVIATIONS` = `'S0C.D' BIND S0C.D S0C.D S0C.MEAN -` — Vector of each element minus the mean
- **5 agents** (G0B, G0A, S0A, S0B, S0C), joined at d1/d0a:
  - `G0B.VAR` = `G0B.DEV 2 POW G0B.MEAN` — population variance, bare scalar
  - `G0A.VAR` = `G0A.DEVS 'G0A.VD' BIND G0A.VD G0A.VD * G0A.MEAN` — population variance
  - `S0A.VAR` = `S0A.DEVS 'DS' BIND DS DS * S0A.MEAN` — population variance
  - `S0B.VAR` = `S0B.DEV 'S0B.D' BIND S0B.D S0B.D * S0B.MEAN` — population variance
  - `S0C.VARIANCE` = `S0C.DEVIATIONS 'S0C.V' BIND S0C.V S0C.V * S0C.MEAN` — population variance as a bare scalar
- **5 agents** (G0A, G0C, S0A, S0B, S0C), joined at d1:
  - `G0A.TITLE` = `G0A.WORDS [ G0A.CAP1 ] MAP G0A.UNWORDS` — title-case every word of a space-separated String
  - `G0C.TITLECASE` = `G0C.WORDS [ G0C.CAPITALIZE ] MAP G0C.UNWORDS` — capitalize every word of a space-separated String
  - `S0A.TITLECASE` = `S0A.WORDS [ S0A.CAP ] MAP S0A.UNWORDS` — capitalize every word of a space-separated String
  - `S0B.TITLE` = `CHARS 'S0B.C' BIND S0B.C [ UPPER ] MAP S0B.C ' ' 1 COLLECT S0B.C -1 DROP CONCAT [ ' ' EQ ] MAP SELECT JOIN` — upper-case every character that starts the String or follows a space
  - `S0C.TITLECASE` = `CHARS 'S0C.C' BIND S0C.C [ UPPER ] MAP S0C.C [ ' ' ] S0C.C -1 DROP CONCAT [ ' ' = ] MAP SELECT JOIN` — upper-case the first letter of every space-separated word in a String
- **4 agents** (G0B, G0C, S0A, S0C), joined at d0a:
  - `G0B.MAX` = `'G0B.R' BIND G0B.R G0B.R 0 GET [ MAX ] FOLD` — largest element of a non-empty numeric Vector, bare scalar
  - `G0C.MAX` = `'G0C.H' BIND G0C.H G0C.H 0 GET [ MAX ] FOLD` — largest element of a non-empty Vector as a bare scalar
  - `S0A.MAX` = `'XS' BIND XS XS 0 GET [ MAX ] FOLD` — largest element as a bare scalar
  - `S0C.MAX` = `'S0C.X' BIND S0C.X S0C.X 0 GET [ MAX ] FOLD` — largest element of a non-empty numeric Vector
- **4 agents** (G0A, S0A, S0B, S0C), joined at d1/d0a:
  - `G0A.SD` = `G0A.VAR SQRT` — population standard deviation (exact sqrt)
  - `S0A.STDDEV` = `S0A.VAR SQRT` — population standard deviation (exact)
  - `S0B.STD` = `S0B.VAR SQRT` — population standard deviation (exact)
  - `S0C.STDDEV` = `S0C.VARIANCE SQRT` — population standard deviation (exact) as a bare scalar
- **4 agents** (G0A, G0B, G0C, S0A), joined at d1/d0a:
  - `G0A.CAP1` = `CHARS 'G0A.TW' BIND G0A.TW [ 1 ] TAKE JOIN UPPER G0A.TW [ 1 ] DROP JOIN 2 COLLECT JOIN` — upper-case the first letter of one word String
  - `G0B.CAP` = `CHARS 'G0B.C' BIND G0B.C [ 1 ] TAKE [ UPPER ] MAP G0B.C [ 1 ] DROP CONCAT JOIN` — upper-case the first letter of a String
  - `G0C.CAPITALIZE` = `CHARS 'G0C.C' BIND G0C.C [ 1 ] TAKE JOIN UPPER G0C.C [ 1 ] DROP JOIN 2 COLLECT JOIN` — upper-case the first character of a String
  - `S0A.CAP` = `CHARS 'CS' BIND CS 0 GET UPPER CS [ 1 ] DROP JOIN 2 COLLECT JOIN` — upper-case the first letter of a String
- **4 agents** (G0A, G0C, S0A, S0B), joined at d0/d1:
  - `G0A.ACRONYM` = `G0A.WORDS [ CHARS 0 GET ] MAP JOIN UPPER` — upper-cased first letters of every word
  - `G0C.ACRONYM` = `G0C.WORDS [ CHARS 0 GET ] MAP JOIN UPPER` — upper-cased first letters of every word as one String
  - `S0A.ACRONYM` = `S0A.WORDS [ CHARS 0 GET ] MAP JOIN UPPER` — upper-cased first letters of every word
  - `S0B.ACRONYM` = `S0B.WORDS [ CHARS 0 GET UPPER ] MAP JOIN` — upper-cased first letters of the space-separated words
- **4 agents** (G0B, G0C, S0A, S0C), joined at d0a:
  - `G0B.MIN` = `'G0B.R' BIND G0B.R G0B.R 0 GET [ MIN ] FOLD` — smallest element of a non-empty numeric Vector, bare scalar
  - `G0C.MIN` = `'G0C.L' BIND G0C.L G0C.L 0 GET [ MIN ] FOLD` — smallest element of a non-empty Vector as a bare scalar
  - `S0A.MIN` = `'XS' BIND XS XS 0 GET [ MIN ] FOLD` — smallest element as a bare scalar
  - `S0C.MIN` = `'S0C.N' BIND S0C.N S0C.N 0 GET [ MIN ] FOLD` — smallest element of a non-empty numeric Vector
- **4 agents** (G0B, G0C, S0A, S0C), joined at d0a:
  - `G0B.RANGE` = `'G0B.R' BIND G0B.R G0B.MAX G0B.R G0B.MIN -` — max minus min, bare scalar
  - `G0C.RANGE` = `'G0C.R' BIND G0C.R G0C.MAX G0C.R G0C.MIN -` — max minus min as a bare scalar
  - `S0A.RANGE` = `'XS' BIND XS S0A.MAX XS S0A.MIN -` — max minus min
  - `S0C.RANGE` = `'S0C.R' BIND S0C.R S0C.MAX S0C.R S0C.MIN -` — max minus min of a numeric Vector
- **4 agents** (G0B, G0C, S0A, S0C), joined at d0a:
  - `G0B.NORMALIZE` = `'G0B.N' BIND G0B.N G0B.N G0B.MIN - G0B.N G0B.RANGE /` — rescale each number to (x - min) / (max - min)
  - `G0C.NORMALIZE` = `'G0C.N' BIND G0C.N G0C.N G0C.MIN - G0C.N G0C.RANGE /` — min-max rescale each element to (x-min)/(max-min)
  - `S0A.NORMALIZE` = `'XS' BIND XS XS S0A.MIN - XS S0A.RANGE /` — min-max rescale to [0,1]
  - `S0C.NORMALIZE` = `'S0C.Z' BIND S0C.Z S0C.Z S0C.MIN - S0C.Z S0C.RANGE /` — min-max rescale of a numeric Vector to [0,1]
- **3 agents** (G0A, G0B, S0B), joined at d0:
  - `G0A.LEN` = `CHARS LENGTH` — character count of a String
  - `G0B.LEN` = `CHARS LENGTH` — character count of a String
  - `S0B.LEN` = `CHARS LENGTH` — character count of a String
- **3 agents** (G0C, S0A, S0B), joined at d1:
  - `G0C.LONGEST` = `G0C.WORD-LENGTHS 0 [ MAX ] FOLD` — length of the longest word as a bare scalar
  - `S0A.LONGEST` = `S0A.WORDS [ CHARS LENGTH ] MAP 0 [ MAX ] FOLD` — length of the longest word in a String
  - `S0B.LONGEST` = `S0B.WORDS [ S0B.LEN ] MAP S0B.MAX` — length of the longest space-separated word
- **2 agents** (G0A, S0B), joined at d0:
  - `G0A.MIN` = `SORT 0 GET` — smallest element of a Vector
  - `S0B.MIN` = `SORT 0 GET` — smallest element of a Vector as a bare scalar
- **2 agents** (G0A, S0B), joined at d1:
  - `G0A.MAX` = `SORT -1 GET` — largest element of a Vector
  - `S0B.MAX` = `SORT REVERSE 0 GET` — largest element of a Vector as a bare scalar
- **2 agents** (G0A, S0B), joined at d1:
  - `G0A.RANGE` = `'G0A.RX' BIND G0A.RX G0A.MAX G0A.RX G0A.MIN -` — max minus min
  - `S0B.RANGE` = `'S0B.XS' BIND S0B.XS S0B.MAX S0B.XS S0B.MIN -` — max minus min
- **2 agents** (G0A, S0B), joined at d1:
  - `G0A.NORM` = `'G0A.NX' BIND G0A.NX G0A.NX G0A.MIN - G0A.NX G0A.RANGE /` — min-max rescale each element to [0,1]
  - `S0B.NORM` = `'S0B.XS' BIND S0B.XS S0B.XS S0B.MIN - S0B.XS S0B.RANGE /` — min-max rescale each element to [0,1]
- **2 agents** (G0A, G0C), joined at d1:
  - `G0A.UNWORDS` = `[ 'G0A.JW' BIND ' ' G0A.JW 2 COLLECT JOIN ] MAP JOIN CHARS [ 1 ] DROP JOIN` — join a Vector of Strings with single spaces
  - `G0C.UNWORDS` = `[ ' ' 2 COLLECT JOIN ] MAP JOIN TRIM` — join a Vector of Strings with single spaces
- **2 agents** (G0B, S0A), joined at d1:
  - `G0B.UNWORDS` = `[ ' ' 2 COLLECT ] MAP FLATTEN JOIN TRIM` — join a Vector of Strings with single spaces
  - `S0A.UNWORDS` = `[ ' ' 2 COLLECT ] MAP FLATTEN [ -1 ] DROP JOIN` — join a Vector of Strings with single spaces
- **2 agents** (G0C, S0B), joined at d1:
  - `G0C.LONG-WORDS` = `G0C.WORDS [ CHARS LENGTH 3 > ] FILTER` — words longer than 3 characters, in order
  - `S0B.LONGWORDS` = `S0B.WORDS [ S0B.LEN 3 > ] FILTER` — words longer than 3 characters, in order

## H2 — lexicon entries built from other entries

- bottleneck gen1: 3/8

## H4 — Core Words by number of correct solutions reaching them

`BIND` 45 · `FOLD` 43 · `LENGTH` 42 · `TOKENIZE` 40 · `ADD` 30 · `SUB` 30 · `DIV` 30 · `GET` 27 · `CHARS` 24 · `MAP` 18 · `MAX` 13 · `JOIN` 12 · `UPPER` 12 · `MUL` 10 · `MIN` 8 · `GT` 6 · `SQRT` 6 · `DROP` 6 · `UNIQUE` 6 · `TALLY` 6 · `FILTER` 6 · `COLLECT` 5 · `SORT` 5 · `TAKE` 3 · `CONCAT` 3 · `REVERSE` 3 · `KEEP` 3 · `SELECT` 2 · `EQ` 2 · `POW` 2 · `FLATTEN` 2 · `TRIM` 2

Unused (68): `TRUE` `FALSE` `AND` `OR` `NOT` `LT` `LTE` `GTE` `MOD` `FLOOR` `CEIL` `ROUND` `QUANTIZE` `ABS` `NEG` `GCD` `RATIO` `EXP` `LN` `SIN` `COS` `ATAN` `PI` `RANDOM` `RANGE` `FILL` `SHAPE` `RESHAPE` `DEPTH` `ORDER` `ZIP` `PUT` `GROUP` `INDEX-OF` `MEMBER` `BSEARCH` `RECORD` `KEYS` `VALUES` `AT` `WITH` `WITHOUT` `HAS?` `MERGE` `SCAN` `ANY` `ALL` `RANK` `LOWER` `SEARCH` `REPLACE` `NUM` `STR` `FORMAT` `JSON-DECODE` `JSON-ENCODE` `EXEC` `CONTRACT` `FAIL` `NIL` `NIL?` `NIL-REASON` `ABSENT` `DEF` `DEL` `DEFINED?` `DIGEST` `PRINT`

## H5 — correct solutions whose Core-only expansion disagreed

- G0B stats.deviations
- G0B stats.variance
- G0B stats.stddev
