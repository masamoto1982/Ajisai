import type { UserWord } from '../wasm-interpreter-types';



export const DEMO_WORDS_VERSION = 8;

export const DEMO_USER_WORDS: UserWord[] = [
    {
        name: 'SAY-HELLO',
        definition: "'Hello' ,, PRINT",
        description: 'Map型: スタックトップを保持しつつOutputに「Hello」を出力する。入力: 任意、出力: 入力そのまま',
    },
    {
        name: 'SAY-WORLD',
        definition: "'World' ,, PRINT",
        description: 'Map型: スタックトップを保持しつつOutputに「World」を出力する。入力: 任意、出力: 入力そのまま',
    },
    {
        name: 'SAY-BANG',
        definition: "'!' ,, PRINT",
        description: 'Map型: スタックトップを保持しつつOutputに「!」を出力する。入力: 任意、出力: 入力そのまま',
    },
    {
        name: 'GREET',
        definition: "{ [ 1 ] = } { SAY-HELLO } { [ 2 ] = } { SAY-WORLD } { IDLE } { SAY-BANG } COND",
        description: 'Form型: 入力値に応じてHello/World/!を分岐出力する（CONDパターン例）。入力: Scalar(1|2|other)、出力: 入力そのまま',
    },
    {
        name: 'GREET-ALL',
        definition: '{ GREET } MAP',
        description: 'Form型: Vectorの各要素にGREETを適用する（MAPパターン例）。入力: Vector、出力: Vector',
    },
];
