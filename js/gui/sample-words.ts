// js/gui/sample-words.ts
// サンプルカスタムワード - 初回起動時に自動追加される
//
// 音楽DSLの拡張ワード（SEQ, SIM, PLAY等）はRust側でネイティブ実装として
// 自動登録される。ここではそれらの上に構築する純正律音階ワードを提供し、
// カスタムワードによるドメイン拡張のデモンストレーションとする。

import type { CustomWord } from '../wasm-types';

/**
 * サンプルカスタムワードの定義
 *
 * 以下の目的でサンプルワードを提供:
 * 1. カスタムワード機能のデモンストレーション
 * 2. 音楽DSL拡張ワードとの組み合わせ例
 * 3. カスタムワード間の依存関係の確認
 * 4. FORTH的なワード拡張の作法を示す
 *
 * 依存関係:
 *   C4 (独立 - 基準周波数)
 *     ├── D4, E4, F4, G4, A4, B4 (C4 を使用 - 純正律音階)
 *     └── C5 (C4 を使用 - オクターブ上)
 */
export const SAMPLE_CUSTOM_WORDS: CustomWord[] = [
    {
        name: 'C4',
        definition: '[ 261.63 ]',
        description: '純正律 C4 / ド (261.63Hz)',
    },
    {
        name: 'D4',
        definition: 'C4 [ 9/8 ] *',
        description: '純正律 D4 / レ (9/8)',
    },
    {
        name: 'E4',
        definition: 'C4 [ 5/4 ] *',
        description: '純正律 E4 / ミ (5/4)',
    },
    {
        name: 'F4',
        definition: 'C4 [ 4/3 ] *',
        description: '純正律 F4 / ファ (4/3)',
    },
    {
        name: 'G4',
        definition: 'C4 [ 3/2 ] *',
        description: '純正律 G4 / ソ (3/2)',
    },
    {
        name: 'A4',
        definition: 'C4 [ 5/3 ] *',
        description: '純正律 A4 / ラ (5/3)',
    },
    {
        name: 'B4',
        definition: 'C4 [ 15/8 ] *',
        description: '純正律 B4 / シ (15/8)',
    },
    {
        name: 'C5',
        definition: 'C4 [ 2 ] *',
        description: '純正律 C5 / 高いド (2/1)',
    },
];
