import type { UserWord } from '../wasm-interpreter-types';

// サンプルワードの定義を更新した際はバージョンをインクリメントすること。
// persistence.ts のマイグレーションロジックが IndexedDB の古い定義を自動更新する。
export const DEMO_WORDS_VERSION = 6;

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
        name: 'SAY-HELLO-WORLD',
        definition: "SAY-HELLO SAY-WORLD 'Hello World' PRINT",
        description:
            'サンプル⑤ — Hello、World、Hello Worldを順に出力',
    },
];
