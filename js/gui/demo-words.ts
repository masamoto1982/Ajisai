import type { UserWord } from '../wasm-interpreter-types';

// サンプルワードの定義を更新した際はバージョンをインクリメントすること。
// persistence.ts のマイグレーションロジックが IndexedDB の古い定義を自動更新する。
export const DEMO_WORDS_VERSION = 7;

export const DEMO_USER_WORDS: UserWord[] = [
    {
        name: 'SAY-HELLO',
        definition: "'Hello' ,, PRINT",
        description: 'Outputに「Hello」を出力し、文字列を返す',
    },
    {
        name: 'SAY-WORLD',
        definition: "'World' ,, PRINT",
        description: 'Outputに「World」を出力し、文字列を返す',
    },
    {
        name: 'SAY-BANG',
        definition: "'!' ,, PRINT",
        description: 'Outputに「!」を出力し、文字列を返す',
    },
    {
        name: 'GREET',
        definition: "{ [ 1 ] = } { SAY-HELLO } { [ 2 ] = } { SAY-WORLD } { IDLE } { SAY-BANG } COND",
        description: '分岐 — 1→Hello、2→World、他→! を出力（COND）',
    },
    {
        name: 'GREET-ALL',
        definition: '{ GREET } MAP',
        description: '反復 — ベクトルの各要素をGREETで出力（MAP）',
    },
];
