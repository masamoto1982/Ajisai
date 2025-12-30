/**
 * ============================================================================
 * Ajisai 統合設定ファイル
 * ============================================================================
 *
 * アプリとリファレンスの両方で使用される共通設定です。
 * テーマカラー、サイト情報、ナビゲーションを一元管理します。
 *
 * 【配置】
 *   /public/ajisai-config.js  ← この設定ファイル
 *   /public/ajisai-theme.js   ← テーマ計算エンジン（この設定を読み込む）
 */

const AjisaiConfig = {
    // =========================================================================
    // 1. テーマ設定
    // =========================================================================
    // 基調色のみを指定 - 他の全ての色はここから自動生成されます
    //
    // HTMLの階層構造に基づく色の濃さ:
    //   header/footer  = 基調色そのもの（最も濃い）
    //   body           = 基調色を極めて薄く（最も明るい）
    //   article        = 基調色を薄く
    //   section        = 基調色をやや薄く
    //   code           = 基調色を含んだ暗い色
    //
    // 文字色・枠線色は各エリアの背景色に基づいて自動決定されます。

    primaryColor: '#6b5b95',  // ディープパープル（紫陽花カラー）
    // primaryColor: '#8B0000',  // 深紅
    // primaryColor: '#2E7D32',  // フォレストグリーン
    // primaryColor: '#1565C0',  // オーシャンブルー

    // =========================================================================
    // 2. サイト基本情報
    // =========================================================================
    meta: {
        title: "Ajisai",
        subTitle: "FORTH-inspired Stack-based Language",
        copyrightYear: new Date().getFullYear()
    },

    // =========================================================================
    // 3. プロジェクト情報
    // =========================================================================
    project: {
        name: "Ajisai Programming Language",
        shortName: "Ajisai",
        author: "masamoto yamashiro",
        url: "https://masamoto1982.github.io/Ajisai/",
        repository: "https://github.com/masamoto1982/Ajisai"
    },

    // =========================================================================
    // 4. ナビゲーションメニュー設定
    // =========================================================================
    globalMenu: [
        { label: "Home", link: "index.html" },
        { label: "Philosophy", link: "philosophy.html" },
        { label: "About", link: "about.html" },
        { label: "Tutorial", link: "tutorial.html" }
    ],

    serviceMenu: [
        { label: "Syntax", link: "syntax.html" },
        { label: "Built-in Words", link: "words.html" },
        { label: "Data Types", link: "types.html" },
        { label: "Control Flow", link: "control.html" },
        { label: "Higher-Order", link: "higher-order.html" }
    ],

    referenceMenu: [
        { label: "Examples", link: "examples.html" },
        { label: "GitHub", link: "https://github.com/masamoto1982/Ajisai" },
        { label: "Demo", link: "https://masamoto1982.github.io/Ajisai/" }
    ],

    // =========================================================================
    // 5. ソーシャルリンク設定
    // =========================================================================
    social: {
        github: { url: "https://github.com/masamoto1982/Ajisai", label: "GitHub" },
        demo: { url: "https://masamoto1982.github.io/Ajisai/", label: "Try Demo" }
    }
};

// グローバルに公開
if (typeof window !== 'undefined') {
    window.AjisaiConfig = AjisaiConfig;
}
