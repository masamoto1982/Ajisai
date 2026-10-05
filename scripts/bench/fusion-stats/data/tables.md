## 融合率

| 種別 | 実行されたブロック | 融合したブロック | ブロック率 | 処理要素 | 融合した要素 | 要素率 | 語で止まる | 値で止まる（実行時） | 値で止まる（外部束縛） | 混在 | 要素0のブロック |
|---|---|---|---|---|---|---|---|---|---|---|---|
| test | 347 | 183 | 52.7% | 498316 | 492837 | 98.9% | 115 | 48 | 1 | 0 | 12 |
| example | 44 | 34 | 77.3% | 200168 | 199925 | 99.9% | 10 | 0 | 0 | 0 | 3 |
| bench | 16 | 10 | 62.5% | 6852000 | 6450000 | 94.1% | 5 | 1 | 0 | 0 | 0 |
| business | 15 | 11 | 73.3% | 5602394 | 5600000 | 100.0% | 0 | 4 | 0 | 0 | 0 |
| all | 422 | 238 | 56.4% | 13152878 | 12742762 | 96.9% | 130 | 53 | 1 | 0 | 15 |

By Word (all kinds): MAP 112/241, FILTER 26/34, FOLD 86/128, SCAN 14/19

Blocks handed to DEF 592, to EXEC 8 (not fusion candidates). Calls linked exact 51133, ordinal 0, unlinked 20 (24 elements).
Static lowering vs runtime: agree 421, disagree 1.

## 止めた原因のランキング（全種別）

| 原因 | 最初に止めた（ブロック） | 同（要素） | それだけ追加で乗る（ブロック） | 同（要素） | 含まれる（ブロック） | 追加しても別の原因で止まる | 種別内訳（最初に止めた） |
|---|---|---|---|---|---|---|---|
| SQRT | 12 | 149 | 12 | 149 | 13 | 1 | test 12 |
| vector literal | 62 | 350929 | 5 | 100046 | 62 | 57 | test 57, example 1, bench 4 |
| RANGE | 7 | 50880 | 5 | 50480 | 7 | 2 | test 5, example 1, bench 1 |
| nested block → DEF | 5 | 12 | 5 | 12 | 6 | 1 | test 5 |
| nested block → MAP | 5 | 10 | 4 | 8 | 6 | 2 | test 4, example 1 |
| block leaves no value | 4 | 4 | 4 | 4 | 4 | 0 | test 4 |
| underflow | 4 | 4 | 4 | 4 | 4 | 0 | test 3, example 1 |
| COLLECT | 7 | 545 | 3 | 6 | 7 | 4 | test 4, example 3 |
| string literal | 3 | 3 | 3 | 3 | 5 | 2 | test 2, example 1 |
| REVERSE | 3 | 6 | 3 | 6 | 3 | 0 | test 2, example 1 |
| POW | 3 | 30 | 3 | 30 | 3 | 0 | test 3 |
| PRINT | 3 | 9 | 3 | 9 | 3 | 0 | test 3 |
| nested block → FOLD | 2 | 5 | 2 | 5 | 14 | 12 | test 1, example 1 |
| unresolved name X | 2 | 2 | 2 | 2 | 2 | 0 | test 2 |
| nested block → EXEC | 1 | 20 | 1 | 20 | 2 | 1 | test 1 |
| SORT | 1 | 1 | 1 | 1 | 1 | 0 | test 1 |
| nested block → FILTER | 1 | 50 | 1 | 50 | 1 | 0 | test 1 |
| unresolved name K | 1 | 1 | 1 | 1 | 1 | 0 | test 1 |
| BIND (destructuring / reserved name) | 1 | 1 | 1 | 1 | 1 | 0 | test 1 |
| NIL? | 1 | 2 | 1 | 2 | 1 | 0 | test 1 |
| NUM | 1 | 2 | 1 | 2 | 1 | 0 | test 1 |
| TAKE | 1 | 3 | 0 | 0 | 3 | 3 | test 1 |
| LENGTH | 0 | 0 | 0 | 0 | 45 | 45 |  |
| NIL literal | 0 | 0 | 0 | 0 | 3 | 3 |  |
| GET | 0 | 0 | 0 | 0 | 2 | 2 |  |
| nested block → SCAN | 0 | 0 | 0 | 0 | 1 | 1 |  |
| BIND | 0 | 0 | 0 | 0 | 1 | 1 |  |

## 止めた原因のランキング（テスト以外）

| 原因 | 最初に止めた（ブロック） | 同（要素） | それだけ追加で乗る（ブロック） | 同（要素） | 含まれる（ブロック） | 追加しても別の原因で止まる | 種別内訳（最初に止めた） |
|---|---|---|---|---|---|---|---|
| vector literal | 5 | 350005 | 2 | 100005 | 5 | 3 | example 1, bench 4 |
| COLLECT | 3 | 30 | 1 | 2 | 3 | 2 | example 3 |
| RANGE | 2 | 50200 | 1 | 50000 | 2 | 1 | example 1, bench 1 |
| nested block → FOLD | 1 | 2 | 1 | 2 | 3 | 2 | example 1 |
| underflow | 1 | 1 | 1 | 1 | 1 | 0 | example 1 |
| string literal | 1 | 1 | 1 | 1 | 1 | 0 | example 1 |
| nested block → MAP | 1 | 2 | 1 | 2 | 1 | 0 | example 1 |
| REVERSE | 1 | 2 | 1 | 2 | 1 | 0 | example 1 |
| GET | 0 | 0 | 0 | 0 | 2 | 2 |  |
| LENGTH | 0 | 0 | 0 | 0 | 2 | 2 |  |

## 止めた原因のランキング（テストのみ）

| 原因 | 最初に止めた（ブロック） | 同（要素） | それだけ追加で乗る（ブロック） | 同（要素） | 含まれる（ブロック） | 追加しても別の原因で止まる | 種別内訳（最初に止めた） |
|---|---|---|---|---|---|---|---|
| SQRT | 12 | 149 | 12 | 149 | 13 | 1 | test 12 |
| nested block → DEF | 5 | 12 | 5 | 12 | 6 | 1 | test 5 |
| RANGE | 5 | 680 | 4 | 480 | 5 | 1 | test 5 |
| block leaves no value | 4 | 4 | 4 | 4 | 4 | 0 | test 4 |
| vector literal | 57 | 924 | 3 | 41 | 57 | 54 | test 57 |
| nested block → MAP | 4 | 8 | 3 | 6 | 5 | 2 | test 4 |
| underflow | 3 | 3 | 3 | 3 | 3 | 0 | test 3 |
| POW | 3 | 30 | 3 | 30 | 3 | 0 | test 3 |
| PRINT | 3 | 9 | 3 | 9 | 3 | 0 | test 3 |
| COLLECT | 4 | 515 | 2 | 4 | 4 | 2 | test 4 |
| string literal | 2 | 2 | 2 | 2 | 4 | 2 | test 2 |
| unresolved name X | 2 | 2 | 2 | 2 | 2 | 0 | test 2 |
| REVERSE | 2 | 4 | 2 | 4 | 2 | 0 | test 2 |
| nested block → FOLD | 1 | 3 | 1 | 3 | 11 | 10 | test 1 |
| nested block → EXEC | 1 | 20 | 1 | 20 | 2 | 1 | test 1 |
| SORT | 1 | 1 | 1 | 1 | 1 | 0 | test 1 |
| nested block → FILTER | 1 | 50 | 1 | 50 | 1 | 0 | test 1 |
| unresolved name K | 1 | 1 | 1 | 1 | 1 | 0 | test 1 |
| BIND (destructuring / reserved name) | 1 | 1 | 1 | 1 | 1 | 0 | test 1 |
| NIL? | 1 | 2 | 1 | 2 | 1 | 0 | test 1 |
| NUM | 1 | 2 | 1 | 2 | 1 | 0 | test 1 |
| TAKE | 1 | 3 | 0 | 0 | 3 | 3 | test 1 |
| LENGTH | 0 | 0 | 0 | 0 | 43 | 43 |  |
| NIL literal | 0 | 0 | 0 | 0 | 3 | 3 |  |
| nested block → SCAN | 0 | 0 | 0 | 0 | 1 | 1 |  |
| BIND | 0 | 0 | 0 | 0 | 1 | 1 |  |

## 2 つ以上の原因の組み合わせ（上位 10）

| 組み合わせ | ブロック | 要素 | 種別 |
|---|---|---|---|
| LENGTH + vector literal | 38 | 200583 | test 36, bench 2 |
| nested block → FOLD + vector literal | 9 | 50244 | test 8, bench 1 |
| LENGTH + NIL literal + vector literal | 3 | 6 | test 3 |
| RANGE + nested block → FOLD | 2 | 400 | test 1, example 1 |
| COLLECT + TAKE | 2 | 511 | test 2 |
| LENGTH + string literal + vector literal | 2 | 3 | test 2 |
| COLLECT + GET | 2 | 28 | example 2 |
| nested block → MAP + vector literal | 1 | 3 | test 1 |
| nested block → DEF + nested block → MAP | 1 | 2 | test 1 |
| nested block → SCAN + vector literal | 1 | 2 | test 1 |
| （組み合わせの総数） | 65 | | |

## 値が原因で止まったブロック

| 原因 | ブロック | 呼び出し | 要素 | 種別 |
|---|---|---|---|---|
| seed が 1 要素ベクトル: 途中の値が機械語を超えた（lane walk は一般 tier を使わない） | 18 | 18 | 5853 | test 14, bench 1, business 3 |
| ERROR で中断（演算に型違いの値: 文字列・真偽値など） | 13 | 13 | 697 | test 13 |
| 要素・seed とも素の値（ゼロ除算の NIL・上限など。内訳は測っていない） | 10 | 10 | 94 | test 10 |
| seed が 1 要素ベクトル: ブロックの形が lane walk の条件外（比較・丸め等が累積値に触れる） | 8 | 8 | 740 | test 8 |
| seed がベクトル（複数要素・ネスト） | 2 | 2 | 40 | test 2 |
| 外部の名前の値が有理数・真偽値でない（lowering 時） | 1 | 1 | 2 | test 1 |
| seed が Nil | 1 | 1 | 2 | test 1 |
| 要素に代数的数（SQRT） | 1 | 1 | 20 | business 1 |

## ネストしたブロックとユーザー定義語

- 中に別のブロック（MAP/FILTER/FOLD/SCAN/EXEC/DEF に渡すもの）を含むために止まったブロック: 29（うちそれだけが原因: 14、要素 50779）
- ユーザー定義語を呼ぶブロック: 48（融合した 26）
- ユーザー定義語の本体が原因で止まったブロック: 9（本体だけが原因: 9）

| ブロック | 呼ぶ語 | 本体が融合サブセット内か | 結果 | 種別 |
|---|---|---|---|---|
| `[ MULW ]` | MULW | MULW: 内 | fused | test |
| `[ S ]` | S | S: 外（SORT） | word | test |
| `[ D 1 ADD ]` | D | D: 内 | fused | test |
| `[ D ]` | D | D: 内 | fused | test |
| `[ S ]` | S | S: 外（vector literal, nested block → FOLD） | word | test |
| `[ T ]` | T, S | S: 外（vector literal, nested block → FOLD）; T: 外（） | word | test |
| `[ F ]` | F | F: 内 | fused | test |
| `[ F ]` | F | F: 内 | fused | test |
| `[ F ]` | F | F: 内 | fused | test |
| `[ G ]` | G, D | D: 内; G: 内 | fused | test |
| `[ G D ]` | G, D | D: 内; G: 内 | fused | test |
| `[ 'X' BIND X SQ X ADD ]` | SQ | SQ: 内 | fused | test |
| `[ 'X' BIND X BADX ]` | BADX | BADX: 外（unresolved name X） | word | test |
| `[ BADK ]` | BADK | BADK: 外（unresolved name K） | word | test |
| `[ PLUS ]` | PLUS | PLUS: 内 | fused | test |
| `[ PLUS ]` | PLUS | PLUS: 内 | fused | test |
| `[ 5 PLUS ]` | PLUS | PLUS: 内 | fused | test |
| `[ PLUS ]` | PLUS | PLUS: 外（underflow） | word | test |
| `[ BIG ]` | BIG | BIG: 内 | fused | test |
| `[ 'X' BIND 1 X INV ]` | INV | INV: 内 | fused | test |
| `[ VADD ]` | VADD | VADD: 外（vector literal） | word | test |
| `[ D ]` | D | D: 内 | fused | test |
| `[ D ]` | D | D: 内 | fused | test |
| `[ D ]` | D | D: 内 | fused | test |
| `[ D ]` | D | D: 内 | fused | test |
| `[ G ]` | G, F, E, D | D: 内; E: 内; F: 内; G: 内 | fused | test |
| `[ U ]` | U, T | T: 内; U: 内 | fused | test |
| `[ D 1 ADD ]` | D | D: 内 | fused | test |
| `[ 'X' BIND X B ]` | B | B: 外（unresolved name X） | word | test |
| `[ WRAP ]` | WRAP | WRAP: 外（COLLECT） | word | test |
| `[ G [ 1 ADD ] 'G' DEF ]` | G | G: 内 | word | test |
| `[ INC ]` | INC | INC: 内 | fused | test |
| `[ [ 7 ] LENGTH G ]` | G | G: 内 | word | test |
| `[ [ 7 ] LENGTH F ]` | F | F: 内 | word | test |
| `[ [ 7 ] LENGTH T ]` | T | T: 内 | word | test |
| `[ [ 7 ] LENGTH ADD T3 T3 1 ADD ]` | T3 | T3: 内 | word | test |
| `[ 'X' BIND [ 9 ] 'K' DEF X K ADD ]` | K | K: 内 | word | test |
| `[ 'X' BIND [ 0.5 1e2 ] 'K' DEF X K ADD ]` | K | K: 内 | word | test |
| `[ [ 7 ] LENGTH 'X' BIND X F X 1 ADD 2 MUL F AND ]` | F | F: 内 | word | test |
| `[ [ 10 ] 'F' DEF F 1 ADD 2 MUL ADD ]` | F | F: 内 | word | test |
| `[ 'X' BIND [ 10 ] 'F' DEF X F ADD F MUL ]` | F | F: 内 | word | test |
| `[ [ 9 ] LENGTH G ADD ]` | G, F | F: 内; G: 内 | word | test |
| `[ 2 COLLECT 0 GET STEP ]` | STEP, DELTA | DELTA: 内; STEP: 内 | word | example |
| `[ 2 COLLECT 0 GET STEP ]` | STEP, DELTA | DELTA: 内; STEP: 内 | word | example |
| `[ DBL ]` | DBL | DBL: 内 | fused | example |
| `[ DBL ]` | DBL | DBL: 内 | fused | example |
| `[ AFFINE ]` | AFFINE | AFFINE: 内 | fused | bench |
| `[ G ]` | G, DBL | DBL: 内; G: 内 | fused | bench |
