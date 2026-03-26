import type { UserWord } from '../wasm-interpreter-types';

// サンプルワードの定義を更新した際はバージョンをインクリメントすること。
// persistence.ts のマイグレーションロジックが IndexedDB の古い定義を自動更新する。
export const DEMO_WORDS_VERSION = 5;

export const DEMO_USER_WORDS: UserWord[] = [
    {
        name: 'SAY-HELLO',
        definition: "'Hello' PRINT",
        description: 'サンプル① — Outputに「Hello」を出力',
    },
    {
        name: 'SAY-WORLD',
        definition: "'World' PRINT",
        description: 'サンプル② — Outputに「World」を出力',
    },
    {
        name: 'SAY-BY-SIGN',
        definition:
            ">> [ 0 ] < >> 'Hello' PRINT >> [ 0 ] = >> 'Hello World' PRINT >>> 'World' PRINT",
        description:
            'サンプル④ — スタックトップが負なら「Hello」、0なら「Hello World」、正なら「World」を出力（シェブロン分岐）',
    },
    {
        name: 'SAY-HELLO-WORLD',
        definition: ": SAY-HELLO SAY-WORLD ; [ 3 ] TIMES 'Hello World' PRINT",
        description:
            'サンプル⑤ — HelloとWorldの出力を3回繰り返し、最後に「Hello World」で締めくくる（TIMES反復）',
    },
];
