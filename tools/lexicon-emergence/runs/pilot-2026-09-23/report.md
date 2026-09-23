# Lexicon-emergence run report

## Per generation

| condition | gen | agents | lexicon | correct | mean tokens | tokens ÷ expanded | Core-equivalent (H5) |
| --- | --- | --- | --- | --- | --- | --- | --- |
| bottleneck | 0 | 3 | 0 | 42/42 | 2.2 | 0.086 | 39/42 |
| bottleneck | 1 | 3 | 8 | 42/42 | 1.4 | 0.053 | 33/42 |
| bottleneck | 2 | 3 | 8 | 42/42 | 1.9 | 0.077 | 33/42 |
| solo | 0 | 3 | 0 | 42/42 | 1.8 | 0.073 | 42/42 |

## H1 — classes defined independently by two or more agents (24 of 34 classes, 132 Words)

- **10 agents** (G0A, G0C, G1A, G1B, G1C, G2A, G2B, G2C, S0A, S0B), joined at d0/d1:
  - `G0A.ACRONYM` = `G0A.WORDS [ CHARS 0 GET ] MAP JOIN UPPER` — upper-cased first letters of every word
  - `G0C.ACRONYM` = `G0C.WORDS [ CHARS 0 GET ] MAP JOIN UPPER` — upper-cased first letters of every word as one String
  - `G1A.ACRONYM` = `G0A.WORDS [ CHARS 0 GET ] MAP JOIN UPPER` — upper-cased first letters of each word of a String, as one String
  - `G1B.ACRONYM` = `G0A.WORDS [ CHARS 0 GET ] MAP JOIN UPPER` — upper-cased first letters of every word, as one String
  - `G1C.ACRONYM` = `G0A.WORDS [ CHARS 0 GET ] MAP JOIN UPPER` — upper-cased first letters of every word of a String
  - `G2A.ACRONYM` = `G0A.WORDS [ CHARS 0 GET ] MAP JOIN UPPER` — upper-cased first letters of every word of a space-separated String
  - `G2B.ACRONYM` = `G0A.WORDS [ CHARS 0 GET UPPER ] MAP JOIN` — upper-cased first letters of every word of a String, as one String
  - `G2C.ACRONYM` = `G0A.WORDS [ CHARS 0 GET ] MAP JOIN UPPER` — upper-cased first letters of every word of a String, as one String
  - `S0A.ACRONYM` = `S0A.WORDS [ CHARS 0 GET ] MAP JOIN UPPER` — upper-cased first letters of every word
  - `S0B.ACRONYM` = `S0B.WORDS [ CHARS 0 GET UPPER ] MAP JOIN` — upper-cased first letters of the space-separated words
- **10 agents** (G0B, G0C, G1A, G1B, G1C, G2A, G2B, G2C, S0A, S0C), joined at d0a:
  - `G0B.NORMALIZE` = `'G0B.N' BIND G0B.N G0B.N G0B.MIN - G0B.N G0B.RANGE /` — rescale each number to (x - min) / (max - min)
  - `G0C.NORMALIZE` = `'G0C.N' BIND G0C.N G0C.N G0C.MIN - G0C.N G0C.RANGE /` — min-max rescale each element to (x-min)/(max-min)
  - `G1A.NORMALIZE` = `'G1A.NS' BIND G1A.NS G1A.NS G1A.MIN - G1A.NS G1A.RANGE /` — min-max rescale a numeric Vector to (x - min) / (max - min)
  - `G1B.NORMALIZE` = `'G1B.N' BIND G1B.N G1B.N G1B.MIN - G1B.N G1B.RANGE /` — rescale each number to (x - min) / (max - min)
  - `G1C.NORMALIZE` = `'G1C.NV' BIND G1C.NV G1C.NV G1C.MIN - G1C.NV G1C.RANGE /` — rescale each number to (x - min) / (max - min)
  - `G2A.NORMALIZE` = `'G2A.XS' BIND G2A.XS G2A.XS G1A.MIN - G2A.XS G1A.RANGE /` — rescale each number to (x - min) / (max - min), Vector in original order
  - `G2B.NORMALIZE` = `'G2B.XS' BIND G2B.XS G2B.XS G1A.MIN - G2B.XS G1A.RANGE /` — rescale each number of a numeric Vector to (x - min) / (max - min)
  - `G2C.NORMALIZE` = `'G2C.XS' BIND G2C.XS G2C.XS G1A.MIN - G2C.XS G1A.RANGE /` — rescale each number to (x - min) / (max - min), Vector in original order
  - `S0A.NORMALIZE` = `'XS' BIND XS XS S0A.MIN - XS S0A.RANGE /` — min-max rescale to [0,1]
  - `S0C.NORMALIZE` = `'S0C.Z' BIND S0C.Z S0C.Z S0C.MIN - S0C.Z S0C.RANGE /` — min-max rescale of a numeric Vector to [0,1]
- **9 agents** (G0A, G1A, G1B, G1C, G2A, G2C, S0A, S0B, S0C), joined at d1/d0/d0a:
  - `G0A.SD` = `G0A.VAR SQRT` — population standard deviation (exact sqrt)
  - `G1A.STDDEV` = `G0B.VAR SQRT` — population standard deviation of a numeric Vector, bare scalar
  - `G1B.STDDEV` = `G0B.VAR SQRT` — population standard deviation, bare scalar
  - `G1C.STDDEV` = `G0B.VAR SQRT` — population standard deviation, bare scalar
  - `G2A.STDDEV` = `G0B.VAR SQRT` — population standard deviation of a numeric Vector, bare scalar
  - `G2C.STDDEV` = `G0B.VAR SQRT` — population standard deviation of a numeric Vector, bare scalar
  - `S0A.STDDEV` = `S0A.VAR SQRT` — population standard deviation (exact)
  - `S0B.STD` = `S0B.VAR SQRT` — population standard deviation (exact)
  - `S0C.STDDEV` = `S0C.VARIANCE SQRT` — population standard deviation (exact) as a bare scalar
- **9 agents** (G0A, G0C, G1A, G1B, G1C, G2A, G2B, G2C, S0A), joined at d1:
  - `G0A.TITLE` = `G0A.WORDS [ G0A.CAP1 ] MAP G0A.UNWORDS` — title-case every word of a space-separated String
  - `G0C.TITLECASE` = `G0C.WORDS [ G0C.CAPITALIZE ] MAP G0C.UNWORDS` — capitalize every word of a space-separated String
  - `G1A.TITLECASE` = `G0A.WORDS [ G1A.CAP ] MAP G1A.UNWORDS` — capitalize every word of a space-separated String
  - `G1B.TITLECASE` = `G0A.WORDS [ G1B.CAP ] MAP G1B.UNWORDS` — capitalize every word of a space-separated String
  - `G1C.TITLECASE` = `G0A.WORDS [ G1C.CAP ] MAP G1C.UNWORDS` — capitalize every space-separated word of a String
  - `G2A.TITLE` = `G0A.WORDS [ CHARS G2A.CAP ' ' 2 COLLECT JOIN ] MAP JOIN TRIM` — space-separated String to title case, words joined by single spaces
  - `G2B.TITLECASE` = `G0A.WORDS [ G2B.CAP ] MAP JOIN TRIM` — title-case a space-separated String: every word's first letter upper-cased
  - `G2C.TITLE` = `G0A.WORDS [ G2C.CAP ' ' 2 COLLECT JOIN ] MAP JOIN CHARS [ -1 ] DROP JOIN` — title-case a space-separated String: each word capitalized, joined by single spaces
  - `S0A.TITLECASE` = `S0A.WORDS [ S0A.CAP ] MAP S0A.UNWORDS` — capitalize every word of a space-separated String
- **8 agents** (G0A, G0B, G0C, G1A, G1B, G1C, G2C, S0A), joined at d1/d0a:
  - `G0A.CAP1` = `CHARS 'G0A.TW' BIND G0A.TW [ 1 ] TAKE JOIN UPPER G0A.TW [ 1 ] DROP JOIN 2 COLLECT JOIN` — upper-case the first letter of one word String
  - `G0B.CAP` = `CHARS 'G0B.C' BIND G0B.C [ 1 ] TAKE [ UPPER ] MAP G0B.C [ 1 ] DROP CONCAT JOIN` — upper-case the first letter of a String
  - `G0C.CAPITALIZE` = `CHARS 'G0C.C' BIND G0C.C [ 1 ] TAKE JOIN UPPER G0C.C [ 1 ] DROP JOIN 2 COLLECT JOIN` — upper-case the first character of a String
  - `G1A.CAP` = `'G1A.W' BIND G1A.W CHARS [ 1 ] TAKE JOIN UPPER CHARS G1A.W CHARS [ 1 ] DROP CONCAT JOIN` — upper-case the first letter of a non-empty String
  - `G1B.CAP` = `CHARS 'G1B.C' BIND G1B.C 0 G1B.C 0 GET UPPER PUT JOIN` — upper-case the first letter of a non-empty String
  - `G1C.CAP` = `CHARS 'G1C.C' BIND G1C.C [ 1 ] TAKE JOIN UPPER G1C.C [ 1 ] DROP JOIN 2 COLLECT JOIN` — upper-case the first character of a non-empty String
  - `G2C.CAP` = `CHARS 'G2C.C' BIND G2C.C [ 1 ] TAKE JOIN UPPER G2C.C [ 1 ] DROP JOIN 2 COLLECT JOIN` — one non-empty word String with its first letter upper-cased
  - `S0A.CAP` = `CHARS 'CS' BIND CS 0 GET UPPER CS [ 1 ] DROP JOIN 2 COLLECT JOIN` — upper-case the first letter of a String
- **7 agents** (G1A, G0B, G0C, G1B, G1C, S0A, S0C), joined at d0a:
  - `G1A.MIN` = `'G1A.R' BIND G1A.R G1A.R 0 GET [ MIN ] FOLD` — smallest element of a non-empty numeric Vector, bare scalar
  - `G0B.MIN` = `'G0B.R' BIND G0B.R G0B.R 0 GET [ MIN ] FOLD` — smallest element of a non-empty numeric Vector, bare scalar
  - `G0C.MIN` = `'G0C.L' BIND G0C.L G0C.L 0 GET [ MIN ] FOLD` — smallest element of a non-empty Vector as a bare scalar
  - `G1B.MIN` = `'G1B.R' BIND G1B.R G1B.R 0 GET [ MIN ] FOLD` — smallest element of a non-empty numeric Vector, bare scalar
  - `G1C.MIN` = `'G1C.R' BIND G1C.R G1C.R 0 GET [ MIN ] FOLD` — smallest element of a non-empty numeric Vector, bare scalar
  - `S0A.MIN` = `'XS' BIND XS XS 0 GET [ MIN ] FOLD` — smallest element as a bare scalar
  - `S0C.MIN` = `'S0C.N' BIND S0C.N S0C.N 0 GET [ MIN ] FOLD` — smallest element of a non-empty numeric Vector
- **7 agents** (G1A, G0B, G0C, G1B, G1C, S0A, S0C), joined at d0a:
  - `G1A.RANGE` = `'G1A.XS' BIND G1A.XS G0B.MAX G1A.XS G1A.MIN -` — largest minus smallest of a non-empty numeric Vector, bare scalar
  - `G0B.RANGE` = `'G0B.R' BIND G0B.R G0B.MAX G0B.R G0B.MIN -` — max minus min, bare scalar
  - `G0C.RANGE` = `'G0C.R' BIND G0C.R G0C.MAX G0C.R G0C.MIN -` — max minus min as a bare scalar
  - `G1B.RANGE` = `'G1B.X' BIND G1B.X G0B.MAX G1B.X G1B.MIN -` — max minus min of a non-empty numeric Vector, bare scalar
  - `G1C.RANGE` = `'G1C.RV' BIND G1C.RV G0B.MAX G1C.RV G1C.MIN -` — max minus min of a non-empty numeric Vector, bare scalar
  - `S0A.RANGE` = `'XS' BIND XS S0A.MAX XS S0A.MIN -` — max minus min
  - `S0C.RANGE` = `'S0C.R' BIND S0C.R S0C.MAX S0C.R S0C.MIN -` — max minus min of a numeric Vector
- **7 agents** (G0C, G1A, G1B, G1C, G2A, S0A, S0B), joined at d1/d0:
  - `G0C.LONGEST` = `G0C.WORD-LENGTHS 0 [ MAX ] FOLD` — length of the longest word as a bare scalar
  - `G1A.LONGEST` = `G0A.WORDS [ G0A.LEN ] MAP G0B.MAX` — character count of the longest word in a String
  - `G1B.LONGEST` = `G0A.WORDS [ G0A.LEN ] MAP G0B.MAX` — character count of the longest word in a space-separated String
  - `G1C.LONGEST` = `G0A.WORDS [ G0A.LEN ] MAP G0B.MAX` — character count of the longest word in a String
  - `G2A.LONGEST` = `G0A.WORDS [ G2A.LEN ] MAP G0B.MAX` — length of the longest word in a space-separated String, bare scalar
  - `S0A.LONGEST` = `S0A.WORDS [ CHARS LENGTH ] MAP 0 [ MAX ] FOLD` — length of the longest word in a String
  - `S0B.LONGEST` = `S0B.WORDS [ S0B.LEN ] MAP S0B.MAX` — length of the longest space-separated word
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
- **4 agents** (G0B, G0C, S0A, S0C), joined at d0a:
  - `G0B.MAX` = `'G0B.R' BIND G0B.R G0B.R 0 GET [ MAX ] FOLD` — largest element of a non-empty numeric Vector, bare scalar
  - `G0C.MAX` = `'G0C.H' BIND G0C.H G0C.H 0 GET [ MAX ] FOLD` — largest element of a non-empty Vector as a bare scalar
  - `S0A.MAX` = `'XS' BIND XS XS 0 GET [ MAX ] FOLD` — largest element as a bare scalar
  - `S0C.MAX` = `'S0C.X' BIND S0C.X S0C.X 0 GET [ MAX ] FOLD` — largest element of a non-empty numeric Vector
- **4 agents** (G0A, G0B, G2A, S0B), joined at d0:
  - `G0A.LEN` = `CHARS LENGTH` — character count of a String
  - `G0B.LEN` = `CHARS LENGTH` — character count of a String
  - `G2A.LEN` = `CHARS LENGTH` — character count of a String, bare scalar
  - `S0B.LEN` = `CHARS LENGTH` — character count of a String
- **4 agents** (G0C, G1A, G1C, S0B), joined at d1/d0:
  - `G0C.LONG-WORDS` = `G0C.WORDS [ CHARS LENGTH 3 > ] FILTER` — words longer than 3 characters, in order
  - `G1A.LONGWORDS` = `G0A.WORDS [ G0A.LEN 3 > ] FILTER` — Vector of words longer than 3 characters, original order
  - `G1C.LONGWORDS` = `G0A.WORDS [ G0A.LEN 3 > ] FILTER` — Vector of words longer than 3 characters, original order
  - `S0B.LONGWORDS` = `S0B.WORDS [ S0B.LEN 3 > ] FILTER` — words longer than 3 characters, in order
- **3 agents** (G0B, G1C, S0A), joined at d1/d0:
  - `G0B.UNWORDS` = `[ ' ' 2 COLLECT ] MAP FLATTEN JOIN TRIM` — join a Vector of Strings with single spaces
  - `G1C.UNWORDS` = `[ ' ' 2 COLLECT ] MAP FLATTEN [ -1 ] DROP JOIN` — join a non-empty Vector of Strings with single spaces
  - `S0A.UNWORDS` = `[ ' ' 2 COLLECT ] MAP FLATTEN [ -1 ] DROP JOIN` — join a Vector of Strings with single spaces
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
- **2 agents** (G0C, G2C), joined at d0:
  - `G0C.WORD-LENGTHS` = `G0C.WORDS [ CHARS LENGTH ] MAP` — Vector of character counts of each word
  - `G2C.LENS` = `G0A.WORDS [ CHARS LENGTH ] MAP` — Vector of the character length of each space-separated word of a String
- **2 agents** (S0B, S0C), joined at d1:
  - `S0B.TITLE` = `CHARS 'S0B.C' BIND S0B.C [ UPPER ] MAP S0B.C ' ' 1 COLLECT S0B.C -1 DROP CONCAT [ ' ' EQ ] MAP SELECT JOIN` — upper-case every character that starts the String or follows a space
  - `S0C.TITLECASE` = `CHARS 'S0C.C' BIND S0C.C [ UPPER ] MAP S0C.C [ ' ' ] S0C.C -1 DROP CONCAT [ ' ' = ] MAP SELECT JOIN` — upper-case the first letter of every space-separated word in a String

## H2 — lexicon entries built from other entries

- bottleneck gen1: 3/8
- bottleneck gen2: 4/8

## H4 — Core Words by number of correct solutions reaching them

`BIND` 93 · `FOLD` 91 · `LENGTH` 84 · `TOKENIZE` 82 · `ADD` 60 · `SUB` 60 · `DIV` 60 · `GET` 52 · `CHARS` 48 · `MAP` 36 · `MAX` 31 · `JOIN` 24 · `UPPER` 24 · `KEEP` 21 · `MIN` 20 · `POW` 14 · `GT` 12 · `SQRT` 12 · `DROP` 12 · `UNIQUE` 12 · `TALLY` 12 · `FILTER` 12 · `MUL` 10 · `COLLECT` 9 · `TAKE` 8 · `CONCAT` 5 · `FLATTEN` 5 · `SORT` 5 · `TRIM` 5 · `REVERSE` 4 · `SELECT` 2 · `EQ` 2 · `PUT` 1

Unused (67): `TRUE` `FALSE` `AND` `OR` `NOT` `LT` `LTE` `GTE` `MOD` `FLOOR` `CEIL` `ROUND` `QUANTIZE` `ABS` `NEG` `GCD` `RATIO` `EXP` `LN` `SIN` `COS` `ATAN` `PI` `RANDOM` `RANGE` `FILL` `SHAPE` `RESHAPE` `DEPTH` `ORDER` `ZIP` `GROUP` `INDEX-OF` `MEMBER` `BSEARCH` `RECORD` `KEYS` `VALUES` `AT` `WITH` `WITHOUT` `HAS?` `MERGE` `SCAN` `ANY` `ALL` `RANK` `LOWER` `SEARCH` `REPLACE` `NUM` `STR` `FORMAT` `JSON-DECODE` `JSON-ENCODE` `EXEC` `CONTRACT` `FAIL` `NIL` `NIL?` `NIL-REASON` `ABSENT` `DEF` `DEL` `DEFINED?` `DIGEST` `PRINT`

## H5 — correct solutions whose Core-only expansion disagreed

- G0B stats.deviations
- G0B stats.variance
- G0B stats.stddev
- G1A stats.deviations
- G1A stats.variance
- G1A stats.stddev
- G1B stats.deviations
- G1B stats.variance
- G1B stats.stddev
- G1C stats.deviations
- G1C stats.variance
- G1C stats.stddev
- G2A stats.deviations
- G2A stats.variance
- G2A stats.stddev
- G2B stats.deviations
- G2B stats.variance
- G2B stats.stddev
- G2C stats.deviations
- G2C stats.variance
- G2C stats.stddev
