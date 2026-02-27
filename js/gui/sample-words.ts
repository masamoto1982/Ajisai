import type { CustomWord } from '../wasm-types';

// サンプルワードの定義を更新した際はバージョンをインクリメントすること。
// persistence.ts のマイグレーションロジックが IndexedDB の古い定義を自動更新する。
export const SAMPLE_WORDS_VERSION = 3;

export const SAMPLE_CUSTOM_WORDS: CustomWord[] = [
    {
        name: 'C4',
        definition: '264',
        description: '純正律 C4 / ド (264Hz)',
    },
    {
        name: 'D4',
        definition: 'C4 9 * 8 /',
        description: '純正律 D4 / レ (297Hz)',
    },
    {
        name: 'E4',
        definition: 'C4 5 * 4 /',
        description: '純正律 E4 / ミ (330Hz)',
    },
    {
        name: 'F4',
        definition: 'C4 4 * 3 /',
        description: '純正律 F4 / ファ (352Hz)',
    },
    {
        name: 'G4',
        definition: 'C4 3 * 2 /',
        description: '純正律 G4 / ソ (396Hz)',
    },
    {
        name: 'A4',
        definition: 'C4 5 * 3 /',
        description: '純正律 A4 / ラ (440Hz)',
    },
    {
        name: 'B4',
        definition: 'C4 15 * 8 /',
        description: '純正律 B4 / シ (495Hz)',
    },
    {
        name: 'C5',
        definition: 'C4 2 *',
        description: '純正律 C5 / 高いド (528Hz)',
    },
];
