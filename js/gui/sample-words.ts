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
 *     └── Hz (C4 を使用 - 周波数変換)
 *   DO, RE, MI, FA, SO, LA, TI (独立 - 音程比率)
 */
export const SAMPLE_CUSTOM_WORDS: CustomWord[] = [
    {
        name: 'C4',
        definition: ': [ 261.63 ]',
        description: '基準周波数 C4 (261.63Hz)',
    },
    {
        name: 'Hz',
        definition: ': C4 *',
        description: '音程比率を周波数に変換',
    },
    {
        name: 'DO',
        definition: ': [ 1 ]',
        description: '純正律 ド (1/1)',
    },
    {
        name: 'RE',
        definition: ': [ 9/8 ]',
        description: '純正律 レ (9/8)',
    },
    {
        name: 'MI',
        definition: ': [ 5/4 ]',
        description: '純正律 ミ (5/4)',
    },
    {
        name: 'FA',
        definition: ': [ 4/3 ]',
        description: '純正律 ファ (4/3)',
    },
    {
        name: 'SO',
        definition: ': [ 3/2 ]',
        description: '純正律 ソ (3/2)',
    },
    {
        name: 'LA',
        definition: ': [ 5/3 ]',
        description: '純正律 ラ (5/3)',
    },
    {
        name: 'TI',
        definition: ': [ 15/8 ]',
        description: '純正律 シ (15/8)',
    },
];
