import type { CustomWord } from '../wasm-types';

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
