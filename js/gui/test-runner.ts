// js/gui/test-runner.ts

export interface TestCase {
    name: string;
    code: string;
    expectedResult?: {
        stackTop?: any;
        stackLength?: number;
        output?: string;
        error?: boolean;
    };
    description?: string;
}

export class TestRunner {
    private testCases: TestCase[] = [
        // ========================================
        // 1. 基本的な算術演算 - 加算
        // ========================================
        {
            name: "加算_中置記法",
            code: "3 + 4",
            expectedResult: { stackTop: { type: "number", value: { numerator: 7, denominator: 1 } } },
            description: "中置記法による加算"
        },
        {
            name: "加算_前置記法",
            code: "+ 3 4",
            expectedResult: { stackTop: { type: "number", value: { numerator: 7, denominator: 1 } } },
            description: "前置記法による加算"
        },
        {
            name: "加算_後置記法",
            code: "3 4 +",
            expectedResult: { stackTop: { type: "number", value: { numerator: 7, denominator: 1 } } },
            description: "後置記法（RPN）による加算"
        },
        {
            name: "加算_混合_中置前置",
            code: "2 + 3 + 4",
            expectedResult: { stackTop: { type: "number", value: { numerator: 9, denominator: 1 } } },
            description: "中置記法の連続"
        },
        {
            name: "加算_混合_後置中置",
            code: "1 2 + + 3",
            expectedResult: { stackTop: { type: "number", value: { numerator: 6, denominator: 1 } } },
            description: "後置と中置の混合"
        },

        // ========================================
        // 2. 基本的な算術演算 - 減算
        // ========================================
        {
            name: "減算_中置記法",
            code: "10 - 3",
            expectedResult: { stackTop: { type: "number", value: { numerator: 7, denominator: 1 } } },
            description: "中置記法による減算"
        },
        {
            name: "減算_前置記法",
            code: "- 10 3",
            expectedResult: { stackTop: { type: "number", value: { numerator: 7, denominator: 1 } } },
            description: "前置記法による減算"
        },
        {
            name: "減算_後置記法",
            code: "10 3 -",
            expectedResult: { stackTop: { type: "number", value: { numerator: 7, denominator: 1 } } },
            description: "後置記法による減算"
        },
        {
            name: "減算_連続",
            code: "20 - 5 - 3",
            expectedResult: { stackTop: { type: "number", value: { numerator: 12, denominator: 1 } } },
            description: "連続した減算"
        },

        // ========================================
        // 3. 基本的な算術演算 - 乗算
        // ========================================
        {
            name: "乗算_中置記法",
            code: "6 * 7",
            expectedResult: { stackTop: { type: "number", value: { numerator: 42, denominator: 1 } } },
            description: "中置記法による乗算"
        },
        {
            name: "乗算_前置記法",
            code: "* 6 7",
            expectedResult: { stackTop: { type: "number", value: { numerator: 42, denominator: 1 } } },
            description: "前置記法による乗算"
        },
        {
            name: "乗算_後置記法",
            code: "6 7 *",
            expectedResult: { stackTop: { type: "number", value: { numerator: 42, denominator: 1 } } },
            description: "後置記法による乗算"
        },

        // ========================================
        // 4. 基本的な算術演算 - 除算
        // ========================================
        {
            name: "除算_中置記法",
            code: "15 / 3",
            expectedResult: { stackTop: { type: "number", value: { numerator: 5, denominator: 1 } } },
            description: "中置記法による除算"
        },
        {
            name: "除算_前置記法",
            code: "/ 15 3",
            expectedResult: { stackTop: { type: "number", value: { numerator: 5, denominator: 1 } } },
            description: "前置記法による除算"
        },
        {
            name: "除算_後置記法",
            code: "15 3 /",
            expectedResult: { stackTop: { type: "number", value: { numerator: 5, denominator: 1 } } },
            description: "後置記法による除算"
        },

        // ========================================
        // 5. 分数と小数
        // ========================================
        {
            name: "分数_加算",
            code: "1/2 + 1/3",
            expectedResult: { stackTop: { type: "number", value: { numerator: 5, denominator: 6 } } },
            description: "分数の加算"
        },
        {
            name: "分数_減算",
            code: "3/4 - 1/2",
            expectedResult: { stackTop: { type: "number", value: { numerator: 1, denominator: 4 } } },
            description: "分数の減算"
        },
        {
            name: "分数_乗算",
            code: "2/3 * 3/4",
            expectedResult: { stackTop: { type: "number", value: { numerator: 1, denominator: 2 } } },
            description: "分数の乗算"
        },
        {
            name: "分数_除算",
            code: "1/2 / 1/4",
            expectedResult: { stackTop: { type: "number", value: { numerator: 2, denominator: 1 } } },
            description: "分数の除算"
        },
        {
            name: "小数_加算",
            code: "0.5 + 0.25",
            expectedResult: { stackTop: { type: "number", value: { numerator: 3, denominator: 4 } } },
            description: "小数の加算（分数に変換）"
        },
        {
            name: "小数_乗算",
            code: "0.5 * 0.5",
            expectedResult: { stackTop: { type: "number", value: { numerator: 1, denominator: 4 } } },
            description: "小数の乗算"
        },

        // ========================================
        // 6. ベクトル基本操作
        // ========================================
        {
            name: "ベクトル_リテラル",
            code: "[ 1 2 3 ]",
            expectedResult: { 
                stackTop: { 
                    type: "vector", 
                    value: [
                        { type: "number", value: { numerator: 1, denominator: 1 } },
                        { type: "number", value: { numerator: 2, denominator: 1 } },
                        { type: "number", value: { numerator: 3, denominator: 1 } }
                    ]
                }
            },
            description: "ベクトルリテラルの作成"
        },
        {
            name: "ベクトル_ネスト",
            code: "[ [ 1 2 ] [ 3 4 ] ]",
            expectedResult: { 
                stackTop: { 
                    type: "vector", 
                    value: [
                        { type: "vector", value: [
                            { type: "number", value: { numerator: 1, denominator: 1 } },
                            { type: "number", value: { numerator: 2, denominator: 1 } }
                        ]},
                        { type: "vector", value: [
                            { type: "number", value: { numerator: 3, denominator: 1 } },
                            { type: "number", value: { numerator: 4, denominator: 1 } }
                        ]}
                    ]
                }
            },
            description: "ネストされたベクトル"
        },
        {
            name: "ベクトル_LENGTH",
            code: "[ 1 2 3 4 5 ] LENGTH",
            expectedResult: { stackTop: { type: "number", value: { numerator: 5, denominator: 1 } } },
            description: "ベクトルの長さ"
        },
        {
            name: "ベクトル_HEAD",
            code: "[ 10 20 30 ] HEAD",
            expectedResult: { stackTop: { type: "number", value: { numerator: 10, denominator: 1 } } },
            description: "ベクトルの先頭要素"
        },
        {
            name: "ベクトル_TAIL",
            code: "[ 10 20 30 ] TAIL",
            expectedResult: { 
                stackTop: { 
                    type: "vector", 
                    value: [
                        { type: "number", value: { numerator: 20, denominator: 1 } },
                        { type: "number", value: { numerator: 30, denominator: 1 } }
                    ]
                }
            },
            description: "ベクトルの末尾"
        },
        {
            name: "ベクトル_CONS",
            code: "5 [ 10 20 ] CONS",
            expectedResult: { 
                stackTop: { 
                    type: "vector", 
                    value: [
                        { type: "number", value: { numerator: 5, denominator: 1 } },
                        { type: "number", value: { numerator: 10, denominator: 1 } },
                        { type: "number", value: { numerator: 20, denominator: 1 } }
                    ]
                }
            },
            description: "要素を先頭に追加"
        },
        {
            name: "ベクトル_APPEND",
            code: "[ 10 20 ] 30 APPEND",
            expectedResult: { 
                stackTop: { 
                    type: "vector", 
                    value: [
                        { type: "number", value: { numerator: 10, denominator: 1 } },
                        { type: "number", value: { numerator: 20, denominator: 1 } },
                        { type: "number", value: { numerator: 30, denominator: 1 } }
                    ]
                }
            },
            description: "要素を末尾に追加"
        },
        {
            name: "ベクトル_REVERSE",
            code: "[ 1 2 3 ] REVERSE",
            expectedResult: { 
                stackTop: { 
                    type: "vector", 
                    value: [
                        { type: "number", value: { numerator: 3, denominator: 1 } },
                        { type: "number", value: { numerator: 2, denominator: 1 } },
                        { type: "number", value: { numerator: 1, denominator: 1 } }
                    ]
                }
            },
            description: "ベクトルの逆順"
        },
        {
            name: "ベクトル_NTH",
            code: "1 [ 10 20 30 ] NTH",
            expectedResult: { stackTop: { type: "number", value: { numerator: 20, denominator: 1 } } },
            description: "N番目の要素取得"
        },
        {
            name: "ベクトル_NTH_負インデックス",
            code: "-1 [ 10 20 30 ] NTH",
            expectedResult: { stackTop: { type: "number", value: { numerator: 30, denominator: 1 } } },
            description: "負のインデックスで要素取得"
        },

        // ========================================
        // 7. ベクトルへの暗黙の反復 - 基本
        // ========================================
        {
            name: "暗黙反復_加算_右",
            code: "[ 1 2 3 ] 10 +",
            expectedResult: { 
                stackTop: { 
                    type: "vector", 
                    value: [
                        { type: "number", value: { numerator: 11, denominator: 1 } },
                        { type: "number", value: { numerator: 12, denominator: 1 } },
                        { type: "number", value: { numerator: 13, denominator: 1 } }
                    ]
                }
            },
            description: "ベクトルの各要素に加算（右）"
        },
        {
            name: "暗黙反復_加算_左",
            code: "10 [ 1 2 3 ] +",
            expectedResult: { 
                stackTop: { 
                    type: "vector", 
                    value: [
                        { type: "number", value: { numerator: 11, denominator: 1 } },
                        { type: "number", value: { numerator: 12, denominator: 1 } },
                        { type: "number", value: { numerator: 13, denominator: 1 } }
                    ]
                }
            },
            description: "ベクトルの各要素に加算（左）"
        },
        {
            name: "暗黙反復_乗算",
            code: "[ 1 2 3 ] 2 *",
            expectedResult: { 
                stackTop: { 
                    type: "vector", 
                    value: [
                        { type: "number", value: { numerator: 2, denominator: 1 } },
                        { type: "number", value: { numerator: 4, denominator: 1 } },
                        { type: "number", value: { numerator: 6, denominator: 1 } }
                    ]
                }
            },
            description: "ベクトルの各要素を2倍"
        },
        {
            name: "暗黙反復_DUP",
            code: "[ 1 2 3 ] DUP",
            expectedResult: { stackLength: 2 },
            description: "ベクトルをDUP（暗黙の反復はしない）"
        },
        {
            name: "暗黙反復_比較",
            code: "[ 1 2 3 ] 2 >",
            expectedResult: { 
                stackTop: { 
                    type: "vector", 
                    value: [
                        { type: "boolean", value: false },
                        { type: "boolean", value: false },
                        { type: "boolean", value: true }
                    ]
                }
            },
            description: "ベクトルの各要素と2を比較"
        },

        // ========================================
        // 8. ベクトルへの暗黙の反復 - ネスト
        // ========================================
        {
            name: "暗黙反復_ネスト_加算",
            code: "[ [ 1 2 ] [ 3 4 ] ] 10 +",
            expectedResult: { 
                stackTop: { 
                    type: "vector", 
                    value: [
                        { type: "vector", value: [
                            { type: "number", value: { numerator: 11, denominator: 1 } },
                            { type: "number", value: { numerator: 12, denominator: 1 } }
                        ]},
                        { type: "vector", value: [
                            { type: "number", value: { numerator: 13, denominator: 1 } },
                            { type: "number", value: { numerator: 14, denominator: 1 } }
                        ]}
                    ]
                }
            },
            description: "ネストされたベクトルへの暗黙の反復"
        },
        {
            name: "暗黙反復_ネスト_乗算",
            code: "[ [ 1 2 ] [ 3 4 ] ] 2 *",
            expectedResult: { 
                stackTop: { 
                    type: "vector", 
                    value: [
                        { type: "vector", value: [
                            { type: "number", value: { numerator: 2, denominator: 1 } },
                            { type: "number", value: { numerator: 4, denominator: 1 } }
                        ]},
                        { type: "vector", value: [
                            { type: "number", value: { numerator: 6, denominator: 1 } },
                            { type: "number", value: { numerator: 8, denominator: 1 } }
                        ]}
                    ]
                }
            },
            description: "ネストされたベクトルへの乗算"
        },
        {
            name: "暗黙反復_深いネスト",
            code: "[ [ [ 1 2 ] [ 3 4 ] ] [ [ 5 6 ] [ 7 8 ] ] ] 1 +",
            expectedResult: { 
                stackTop: { 
                    type: "vector", 
                    value: [
                        { type: "vector", value: [
                            { type: "vector", value: [
                                { type: "number", value: { numerator: 2, denominator: 1 } },
                                { type: "number", value: { numerator: 3, denominator: 1 } }
                            ]},
                            { type: "vector", value: [
                                { type: "number", value: { numerator: 4, denominator: 1 } },
                                { type: "number", value: { numerator: 5, denominator: 1 } }
                            ]}
                        ]},
                        { type: "vector", value: [
                            { type: "vector", value: [
                                { type: "number", value: { numerator: 6, denominator: 1 } },
                                { type: "number", value: { numerator: 7, denominator: 1 } }
                            ]},
                            { type: "vector", value: [
                                { type: "number", value: { numerator: 8, denominator: 1 } },
                                { type: "number", value: { numerator: 9, denominator: 1 } }
                            ]}
                        ]}
                    ]
                }
            },
            description: "深くネストされたベクトルへの演算"
        },

        // ========================================
        // 9. カスタムワード定義と実行
        // ========================================
        {
            name: "カスタムワード_明示的定義",
            code: "3 4 + \"SEVEN\" DEF",
            expectedResult: { output: "Defined: SEVEN" },
            description: "カスタムワードの明示的定義"
        },
        {
            name: "カスタムワード_自動命名",
            code: "2 3 +",
            expectedResult: { stackTop: { type: "number", value: { numerator: 5, denominator: 1 } } },
            description: "自動ワード生成と実行"
        },
        {
            name: "カスタムワード_複合定義",
            code: "DUP * \"SQUARE\" DEF",
            expectedResult: { output: "Defined: SQUARE" },
            description: "複合操作のカスタムワード定義"
        },

        // ========================================
        // 10. カスタムワードとベクトル - 基本
        // ========================================
        {
            name: "カスタムワード_ベクトル_単純",
            code: "2 * \"DOUBLE\" DEF\n[ 1 2 3 ] DOUBLE",
            expectedResult: { 
                stackTop: { 
                    type: "vector", 
                    value: [
                        { type: "number", value: { numerator: 2, denominator: 1 } },
                        { type: "number", value: { numerator: 4, denominator: 1 } },
                        { type: "number", value: { numerator: 6, denominator: 1 } }
                    ]
                }
            },
            description: "カスタムワードをベクトルに適用"
        },
        {
            name: "カスタムワード_ベクトル_左配置",
            code: "2 * \"DOUBLE\" DEF\nDOUBLE [ 5 10 15 ]",
            expectedResult: { 
                stackTop: { 
                    type: "vector", 
                    value: [
                        { type: "number", value: { numerator: 10, denominator: 1 } },
                        { type: "number", value: { numerator: 20, denominator: 1 } },
                        { type: "number", value: { numerator: 30, denominator: 1 } }
                    ]
                }
            },
            description: "ベクトルの左にカスタムワード"
        },
        {
            name: "カスタムワード_複合_ベクトル",
            code: "DUP + \"TWICE\" DEF\n[ 3 4 5 ] TWICE",
            expectedResult: { 
                stackTop: { 
                    type: "vector", 
                    value: [
                        { type: "number", value: { numerator: 6, denominator: 1 } },
                        { type: "number", value: { numerator: 8, denominator: 1 } },
                        { type: "number", value: { numerator: 10, denominator: 1 } }
                    ]
                }
            },
            description: "複合カスタムワードをベクトルに適用"
        },

        // ========================================
        // 11. カスタムワードを含むカスタムワード
        // ========================================
        {
            name: "カスタムワード_ネスト定義",
            code: "2 * \"DOUBLE\" DEF\nDOUBLE DOUBLE \"QUADRUPLE\" DEF\n3 QUADRUPLE",
            expectedResult: { stackTop: { type: "number", value: { numerator: 12, denominator: 1 } } },
            description: "カスタムワードを含むカスタムワード"
        },
        {
            name: "カスタムワード_連鎖定義",
            code: "1 + \"INC\" DEF\nINC INC \"INC2\" DEF\n5 INC2",
            expectedResult: { stackTop: { type: "number", value: { numerator: 7, denominator: 1 } } },
            description: "カスタムワードの連鎖"
        },
        {
            name: "カスタムワード_複雑な連鎖",
            code: "DUP * \"SQ\" DEF\nSQ SQ \"POW4\" DEF\n2 POW4",
            expectedResult: { stackTop: { type: "number", value: { numerator: 16, denominator: 1 } } },
            description: "複雑なカスタムワードの連鎖"
        },

        // ========================================
        // 12. カスタムワードとネストベクトル
        // ========================================
        {
            name: "カスタムワード_ネストベクトル",
            code: "2 * \"DOUBLE\" DEF\n[ [ 1 2 ] [ 3 4 ] ] DOUBLE",
            expectedResult: { 
                stackTop: { 
                    type: "vector", 
                    value: [
                        { type: "vector", value: [
                            { type: "number", value: { numerator: 2, denominator: 1 } },
                            { type: "number", value: { numerator: 4, denominator: 1 } }
                        ]},
                        { type: "vector", value: [
                            { type: "number", value: { numerator: 6, denominator: 1 } },
                            { type: "number", value: { numerator: 8, denominator: 1 } }
                        ]}
                    ]
                }
            },
            description: "カスタムワードをネストベクトルに適用"
        },
        {
            name: "複合カスタムワード_ネストベクトル",
            code: "1 + \"INC\" DEF\nINC INC \"INC2\" DEF\n[ [ 1 2 ] [ 3 4 ] ] INC2",
            expectedResult: { 
                stackTop: { 
                    type: "vector", 
                    value: [
                        { type: "vector", value: [
                            { type: "number", value: { numerator: 3, denominator: 1 } },
                            { type: "number", value: { numerator: 4, denominator: 1 } }
                        ]},
                        { type: "vector", value: [
                            { type: "number", value: { numerator: 5, denominator: 1 } },
                            { type: "number", value: { numerator: 6, denominator: 1 } }
                        ]}
                    ]
                }
            },
            description: "複合カスタムワードをネストベクトルに適用"
        },
        {
            name: "深いネスト_カスタムワード",
            code: "10 + \"ADD10\" DEF\n[ [ [ 1 ] ] ] ADD10",
            expectedResult: { 
                stackTop: { 
                    type: "vector", 
                    value: [
                        { type: "vector", value: [
                            { type: "vector", value: [
                                { type: "number", value: { numerator: 11, denominator: 1 } }
                            ]}
                        ]}
                    ]
                }
            },
            description: "深くネストされたベクトルへのカスタムワード適用"
        },

        // ========================================
        // 13. スタック操作
        // ========================================
        {
            name: "スタック_DUP",
            code: "42 DUP",
            expectedResult: { stackLength: 2 },
            description: "スタックトップの複製"
        },
        {
            name: "スタック_DROP",
            code: "10 20 DROP",
            expectedResult: { stackTop: { type: "number", value: { numerator: 10, denominator: 1 } } },
            description: "スタックトップの削除"
        },
        {
            name: "スタック_SWAP",
            code: "10 20 SWAP",
            expectedResult: { stackTop: { type: "number", value: { numerator: 10, denominator: 1 } } },
            description: "上位2つの交換"
        },
        {
            name: "スタック_OVER",
            code: "10 20 OVER",
            expectedResult: { stackTop: { type: "number", value: { numerator: 10, denominator: 1 } } },
            description: "2番目の要素をコピー"
        },
        {
            name: "スタック_ROT",
            code: "10 20 30 ROT",
            expectedResult: { stackTop: { type: "number", value: { numerator: 10, denominator: 1 } } },
            description: "3番目を最上位へ"
        },
        {
            name: "スタック_NIP",
            code: "10 20 NIP",
            expectedResult: { stackTop: { type: "number", value: { numerator: 20, denominator: 1 } } },
            description: "2番目を削除"
        },

        // ========================================
        // 14. レジスタ操作
        // ========================================
        {
            name: "レジスタ_TO_R",
            code: "42 >R",
            expectedResult: { stackLength: 0 },
            description: "スタックからレジスタへ"
        },
        {
            name: "レジスタ_FROM_R",
            code: "42 >R R>",
            expectedResult: { stackTop: { type: "number", value: { numerator: 42, denominator: 1 } } },
            description: "レジスタからスタックへ"
        },
        {
            name: "レジスタ_FETCH",
            code: "42 >R R@",
            expectedResult: { stackTop: { type: "number", value: { numerator: 42, denominator: 1 } } },
            description: "レジスタの値をコピー"
        },
        {
            name: "レジスタ_R_ADD",
            code: "10 >R 5 R+",
            expectedResult: { stackTop: { type: "number", value: { numerator: 15, denominator: 1 } } },
            description: "レジスタとの加算"
        },
        {
            name: "レジスタ_R_SUB",
            code: "10 >R 20 R-",
            expectedResult: { stackTop: { type: "number", value: { numerator: 10, denominator: 1 } } },
            description: "レジスタとの減算"
        },
        {
            name: "レジスタ_R_MUL",
            code: "3 >R 4 R*",
            expectedResult: { stackTop: { type: "number", value: { numerator: 12, denominator: 1 } } },
            description: "レジスタとの乗算"
        },
        {
            name: "レジスタ_R_DIV",
            code: "2 >R 10 R/",
            expectedResult: { stackTop: { type: "number", value: { numerator: 5, denominator: 1 } } },
            description: "レジスタとの除算"
        },
        {
            name: "レジスタ_ベクトル_R_ADD",
            code: "10 >R [ 1 2 3 ] R+",
            expectedResult: { 
                stackTop: { 
                    type: "vector", 
                    value: [
                        { type: "number", value: { numerator: 11, denominator: 1 } },
                        { type: "number", value: { numerator: 12, denominator: 1 } },
                        { type: "number", value: { numerator: 13, denominator: 1 } }
                    ]
                }
            },
            description: "ベクトルとレジスタの演算"
        },

        // ========================================
        // 15. 比較演算
        // ========================================
        {
            name: "比較_GT",
            code: "5 3 >",
            expectedResult: { stackTop: { type: "boolean", value: true } },
            description: "より大きい"
        },
        {
            name: "比較_GE",
            code: "5 5 >=",
            expectedResult: { stackTop: { type: "boolean", value: true } },
            description: "以上"
        },
        {
            name: "比較_LT",
            code: "3 5 <",
            expectedResult: { stackTop: { type: "boolean", value: true } },
            description: "より小さい"
        },
        {
            name: "比較_LE",
            code: "5 5 <=",
            expectedResult: { stackTop: { type: "boolean", value: true } },
            description: "以下"
        },
        {
            name: "比較_EQ",
            code: "5 5 =",
            expectedResult: { stackTop: { type: "boolean", value: true } },
            description: "等しい"
        },
        {
            name: "比較_ベクトル",
            code: "[ 1 2 3 ] [ 1 2 3 ] =",
            expectedResult: { stackTop: { type: "boolean", value: true } },
            description: "ベクトルの等価比較"
        },

        // ========================================
        // 16. 論理演算
        // ========================================
        {
            name: "論理_AND_TT",
            code: "true true AND",
            expectedResult: { stackTop: { type: "boolean", value: true } },
            description: "論理積（true AND true）"
        },
        {
            name: "論理_AND_TF",
            code: "true false AND",
            expectedResult: { stackTop: { type: "boolean", value: false } },
            description: "論理積（true AND false）"
        },
        {
            name: "論理_OR_FF",
            code: "false false OR",
            expectedResult: { stackTop: { type: "boolean", value: false } },
            description: "論理和（false OR false）"
        },
        {
            name: "論理_OR_TF",
            code: "true false OR",
            expectedResult: { stackTop: { type: "boolean", value: true } },
            description: "論理和（true OR false）"
        },
        {
            name: "論理_NOT_T",
            code: "true NOT",
            expectedResult: { stackTop: { type: "boolean", value: false } },
            description: "論理否定（NOT true）"
        },
        {
            name: "論理_NOT_F",
            code: "false NOT",
            expectedResult: { stackTop: { type: "boolean", value: true } },
            description: "論理否定（NOT false）"
        },
        {
            name: "論理_ベクトル_NOT",
            code: "[ true false true ] NOT",
            expectedResult: { 
                stackTop: { 
                    type: "vector", 
                    value: [
                        { type: "boolean", value: false },
                        { type: "boolean", value: true },
                        { type: "boolean", value: false }
                    ]
                }
            },
            description: "ベクトルへの論理否定"
        },

        // ========================================
        // 17. 条件演算
        // ========================================
        {
            name: "条件_選択_TRUE",
            code: "true 10 20 ?",
            expectedResult: { stackTop: { type: "number", value: { numerator: 10, denominator: 1 } } },
            description: "条件選択（true）"
        },
        {
            name: "条件_選択_FALSE",
            code: "false 10 20 ?",
            expectedResult: { stackTop: { type: "number", value: { numerator: 20, denominator: 1 } } },
            description: "条件選択（false）"
        },
        {
            name: "条件_選択_ベクトル",
            code: "[ true false true ] 1 0 ?",
            expectedResult: { 
                stackTop: { 
                    type: "vector", 
                    value: [
                        { type: "number", value: { numerator: 1, denominator: 1 } },
                        { type: "number", value: { numerator: 0, denominator: 1 } },
                        { type: "number", value: { numerator: 1, denominator: 1 } }
                    ]
                }
            },
            description: "ベクトルによる条件選択"
        },
        {
            name: "条件_WHEN_TRUE",
            code: "42 true WHEN",
            expectedResult: { stackTop: { type: "number", value: { numerator: 42, denominator: 1 } } },
            description: "条件付き実行（true）"
        },
        {
            name: "条件_WHEN_FALSE",
            code: "42 false WHEN",
            expectedResult: { stackLength: 0 },
            description: "条件付き実行（false）"
        },

        // ========================================
        // 18. NIL関連
        // ========================================
        {
            name: "NIL_判定_TRUE",
            code: "nil NIL?",
            expectedResult: { stackTop: { type: "boolean", value: true } },
            description: "NIL判定（true）"
        },
        {
            name: "NIL_判定_FALSE",
            code: "42 NIL?",
            expectedResult: { stackTop: { type: "boolean", value: false } },
            description: "NIL判定（false）"
        },
        {
            name: "NIL_NOT_NIL_TRUE",
            code: "42 NOT-NIL?",
            expectedResult: { stackTop: { type: "boolean", value: true } },
            description: "NOT-NIL判定（true）"
        },
        {
            name: "NIL_NOT_NIL_FALSE",
            code: "nil NOT-NIL?",
            expectedResult: { stackTop: { type: "boolean", value: false } },
            description: "NOT-NIL判定（false）"
        },
        {
            name: "NIL_DEFAULT_使用",
            code: "nil 42 DEFAULT",
            expectedResult: { stackTop: { type: "number", value: { numerator: 42, denominator: 1 } } },
            description: "デフォルト値の使用"
        },
        {
            name: "NIL_DEFAULT_不使用",
            code: "10 42 DEFAULT",
            expectedResult: { stackTop: { type: "number", value: { numerator: 10, denominator: 1 } } },
            description: "デフォルト値の不使用"
        },
        {
            name: "NIL_ベクトル内",
            code: "[ 1 nil 3 ]",
            expectedResult: { 
                stackTop: { 
                    type: "vector", 
                    value: [
                        { type: "number", value: { numerator: 1, denominator: 1 } },
                        { type: "nil", value: null },
                        { type: "number", value: { numerator: 3, denominator: 1 } }
                    ]
                }
            },
            description: "ベクトル内のNIL"
        },
        {
            name: "NIL_三値論理_AND",
            code: "true nil AND",
            expectedResult: { stackTop: { type: "nil", value: null } },
            description: "三値論理（true AND nil）"
        },
        {
            name: "NIL_三値論理_OR",
            code: "false nil OR",
            expectedResult: { stackTop: { type: "nil", value: null } },
            description: "三値論理（false OR nil）"
        },

        // ========================================
        // 19. 複雑な式
        // ========================================
        {
            name: "複雑_算術式",
            code: "2 3 + 4 *",
            expectedResult: { stackTop: { type: "number", value: { numerator: 20, denominator: 1 } } },
            description: "複雑な算術式 (2+3)*4"
        },
        {
            name: "複雑_ベクトル演算",
            code: "[ 1 2 3 ] DUP + ",
            expectedResult: { 
                stackTop: { 
                    type: "vector", 
                    value: [
                        { type: "number", value: { numerator: 2, denominator: 1 } },
                        { type: "number", value: { numerator: 4, denominator: 1 } },
                        { type: "number", value: { numerator: 6, denominator: 1 } }
                    ]
                }
            },
            description: "ベクトルの自己加算"
        },
        {
            name: "複雑_連鎖演算",
            code: "5 DUP * DUP +",
            expectedResult: { stackTop: { type: "number", value: { numerator: 50, denominator: 1 } } },
            description: "5の二乗を2倍"
        },
        {
            name: "複雑_混合型ベクトル",
            code: "[ 1 true \"hello\" nil ]",
            expectedResult: { 
                stackTop: { 
                    type: "vector", 
                    value: [
                        { type: "number", value: { numerator: 1, denominator: 1 } },
                        { type: "boolean", value: true },
                        { type: "string", value: "hello" },
                        { type: "nil", value: null }
                    ]
                }
            },
            description: "混合型のベクトル"
        },

        // ========================================
        // 20. エラーケース
        // ========================================
        {
            name: "エラー_ゼロ除算",
            code: "5 0 /",
            expectedResult: { error: true },
            description: "ゼロ除算エラー"
        },
        {
            name: "エラー_スタックアンダーフロー",
            code: "+",
            expectedResult: { error: true },
            description: "スタックアンダーフローエラー"
        },
        {
            name: "エラー_未知のワード",
            code: "UNKNOWN_WORD",
            expectedResult: { error: true },
            description: "未知のワードエラー"
        },
        {
            name: "エラー_レジスタ空",
            code: "R@",
            expectedResult: { error: true },
            description: "レジスタが空のエラー"
        },
        {
            name: "エラー_ベクトル範囲外",
            code: "10 [ 1 2 3 ] NTH",
            expectedResult: { error: true },
            description: "ベクトルインデックス範囲外"
        },
        {
            name: "エラー_空ベクトルHEAD",
            code: "[ ] HEAD",
            expectedResult: { error: true },
            description: "空ベクトルのHEAD"
        },
        {
            name: "エラー_空ベクトルTAIL",
            code: "[ ] TAIL",
            expectedResult: { error: true },
            description: "空ベクトルのTAIL"
        },
        {
            name: "エラー_型不一致",
            code: "\"hello\" 5 +",
            expectedResult: { error: true },
            description: "型不一致エラー"
        },

        // ========================================
        // 21. 文字列操作
        // ========================================
        {
            name: "文字列_リテラル",
            code: "\"Hello, World!\"",
            expectedResult: { stackTop: { type: "string", value: "Hello, World!" } },
            description: "文字列リテラル"
        },
        {
            name: "文字列_ベクトル内",
            code: "[ \"a\" \"b\" \"c\" ]",
            expectedResult: { 
                stackTop: { 
                    type: "vector", 
                    value: [
                        { type: "string", value: "a" },
                        { type: "string", value: "b" },
                        { type: "string", value: "c" }
                    ]
                }
            },
            description: "文字列のベクトル"
        },

        // ========================================
        // 22. 特殊なケース
        // ========================================
        {
            name: "特殊_空ベクトル",
            code: "[ ]",
            expectedResult: { 
                stackTop: { 
                    type: "vector", 
                    value: []
                }
            },
            description: "空のベクトル"
        },
        {
            name: "特殊_単一要素ベクトル",
            code: "[ 42 ]",
            expectedResult: { 
                stackTop: { 
                    type: "vector", 
                    value: [
                        { type: "number", value: { numerator: 42, denominator: 1 } }
                    ]
                }
            },
            description: "単一要素のベクトル"
        },
        {
            name: "特殊_EMPTY判定_TRUE",
            code: "[ ] EMPTY?",
            expectedResult: { stackTop: { type: "boolean", value: true } },
            description: "空ベクトル判定（true）"
        },
        {
            name: "特殊_EMPTY判定_FALSE",
            code: "[ 1 ] EMPTY?",
            expectedResult: { stackTop: { type: "boolean", value: false } },
            description: "空ベクトル判定（false）"
        },

        // ========================================
        // 23. 高度な暗黙の反復パターン
        // ========================================
        {
            name: "高度_ベクトル同士の演算",
            code: "[ 1 2 3 ] [ 4 5 6 ] +",
            expectedResult: { 
                stackTop: { 
                    type: "vector", 
                    value: [
                        { type: "number", value: { numerator: 5, denominator: 1 } },
                        { type: "number", value: { numerator: 7, denominator: 1 } },
                        { type: "number", value: { numerator: 9, denominator: 1 } }
                    ]
                }
            },
            description: "ベクトル同士の要素ごとの加算"
        },
        {
            name: "高度_ベクトル長不一致",
            code: "[ 1 2 ] [ 3 4 5 ] +",
            expectedResult: { error: true },
            description: "長さの異なるベクトルの演算"
        },
        {
            name: "高度_カスタムワード連鎖_ベクトル",
            code: "2 * \"D\" DEF\nD D \"Q\" DEF\n[ 1 2 3 ] Q",
            expectedResult: { 
                stackTop: { 
                    type: "vector", 
                    value: [
                        { type: "number", value: { numerator: 4, denominator: 1 } },
                        { type: "number", value: { numerator: 8, denominator: 1 } },
                        { type: "number", value: { numerator: 12, denominator: 1 } }
                    ]
                }
            },
            description: "連鎖カスタムワードをベクトルに適用"
        },
        {
            name: "高度_複雑なネスト処理",
            code: "DUP * \"SQ\" DEF\n[ [ 2 3 ] [ 4 5 ] ] SQ",
            expectedResult: { 
                stackTop: { 
                    type: "vector", 
                    value: [
                        { type: "vector", value: [
                            { type: "number", value: { numerator: 4, denominator: 1 } },
                            { type: "number", value: { numerator: 9, denominator: 1 } }
                        ]},
                        { type: "vector", value: [
                            { type: "number", value: { numerator: 16, denominator: 1 } },
                            { type: "number", value: { numerator: 25, denominator: 1 } }
                        ]}
                    ]
                }
            },
            description: "二乗演算をネストベクトルに適用"
        }
    ];

    // ... (既存のヘルパーメソッドはそのまま維持)
    
    private formatValue(value: any): string {
        // 既存の実装をそのまま使用
        if (!value) return 'undefined';
        
        switch (value.type) {
            case 'number':
                if (typeof value.value === 'object' && value.value !== null && 'numerator' in value.value && 'denominator' in value.value) {
                    const frac = value.value as { numerator: number; denominator: number };
                    if (frac.denominator === 1) {
                        return frac.numerator.toString();
                    } else {
                        return `${frac.numerator}/${frac.denominator}`;
                    }
                }
                return typeof value.value === 'string' ? value.value : String(value.value);
            case 'string':
                return `"${value.value}"`;
            case 'symbol':
                return String(value.value);
            case 'boolean':
                return value.value ? 'true' : 'false';
            case 'vector':
                if (Array.isArray(value.value)) {
                    return `[ ${value.value.map((v: any) => this.formatValue(v)).join(' ')} ]`;
                }
                return '[ ]';
            case 'nil':
                return 'nil';
            default:
                return String(value.value);
        }
    }

    // 以下、既存のメソッドは変更なし
    async runAllTests(): Promise<TestResult[]> { /* 既存実装 */ }
    async runSingleTest(testCase: TestCase): Promise<TestResult> { /* 既存実装 */ }
    private compareResults(expected: any, actual: any): boolean { /* 既存実装 */ }
    private deepEqual(a: any, b: any): boolean { /* 既存実装 */ }
    private formatTestValue(value: any): string { /* 既存実装 */ }
}
