import type { CustomWord } from '../wasm-types';

// サンプルワードの定義を更新した際はバージョンをインクリメントすること。
// persistence.ts のマイグレーションロジックが IndexedDB の古い定義を自動更新する。
export const SAMPLE_WORDS_VERSION = 4;

export const SAMPLE_CUSTOM_WORDS: CustomWord[] = [
    {
        name: 'GREETING',
        definition: "'Hello'",
        description: 'Hello world サンプル — 挨拶文字列',
    },
    {
        name: 'WORLD',
        definition: "'world'",
        description: 'Hello world サンプル — world 文字列',
    },
    {
        name: 'HELLO-WORLD',
        definition: "GREETING ' ' WORLD CONCAT CONCAT",
        description: 'Hello world サンプル — GREETING と WORLD を結合 "Hello world"',
    },
];
