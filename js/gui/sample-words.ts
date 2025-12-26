// js/gui/sample-words.ts
// サンプルカスタムワード - 初回起動時に自動追加される

import type { CustomWord } from '../wasm-types';

/**
 * サンプルカスタムワードの定義
 *
 * 以下の目的でサンプルワードを提供:
 * 1. カスタムワード機能のデモンストレーション
 * 2. 「!」フラグ（強制フラグ）の振る舞いテスト
 * 3. 辞書のimport/export機能の動作検証
 * 4. カスタムワード間の依存関係の確認
 *
 * 依存関係:
 *   TAX_RATE (独立)
 *     ├── TAX (TAX_RATE を使用)
 *     └── TAX_MULT (TAX_RATE を使用)
 *           └── TAX_INCL (TAX_MULT を使用)
 */
export const SAMPLE_CUSTOM_WORDS: CustomWord[] = [
    {
        name: 'TAX_RATE',
        definition: ': [ 0.1 ]',
        description: '消費税率 (10%)',
    },
    {
        name: 'TAX',
        definition: ': TAX_RATE *',
        description: '税額を計算 (価格 → 税額)',
    },
    {
        name: 'TAX_MULT',
        definition: ': 1 TAX_RATE +',
        description: '税込み倍率 (1.1)',
    },
    {
        name: 'TAX_INCL',
        definition: ': TAX_MULT *',
        description: '税込価格を計算 (価格 → 税込価格)',
    },
];
