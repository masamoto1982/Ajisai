/**
 * ============================================================================
 * Ajisai 共通テーマ設定
 * ============================================================================
 * このファイルを編集することで、アプリとReferenceサイトの
 * カラーテーマを一括で変更できます。
 */

const AjisaiTheme = {
    // -------------------------------------------------------------------------
    // カラーパレット（紫陽花テーマ）
    // -------------------------------------------------------------------------
    colors: {
        "--color-primary": "#6b5b95",    // 紫陽花カラー（メイン）
        "--color-secondary": "#8b7db5",  // 紫陽花カラー（アクセント）
        "--color-light": "#f8f7fc",      // 背景色（明るい）
        "--color-medium": "#d4cfe8",     // 背景色（中間）
        "--color-dark": "#e8e4f3",       // 背景色（濃いめ）
        "--color-text": "#333",          // テキスト色
        "--color-text-light": "#666"     // テキスト色（薄め）
    },

    // -------------------------------------------------------------------------
    // グラデーション設定（階層構造）
    // -------------------------------------------------------------------------
    gradients: {
        "--gradient-header": "linear-gradient(to bottom, var(--color-medium), var(--color-light), var(--color-medium))",
        "--gradient-background": "linear-gradient(to bottom, var(--color-secondary), var(--color-dark), var(--color-secondary))",
        "--gradient-parent": "linear-gradient(to bottom, var(--color-medium), var(--color-light), var(--color-medium))",
        "--gradient-child": "linear-gradient(to bottom, var(--color-light), var(--color-medium))"
    },

    // -------------------------------------------------------------------------
    // 見出し設定（階層構造）
    // -------------------------------------------------------------------------
    headings: {
        "--heading-h1": "var(--color-primary)",      // H1: プライマリ
        "--heading-h2": "var(--color-primary)",      // H2: プライマリ
        "--heading-h3": "var(--color-secondary)"     // H3: セカンダリ
    },

    // -------------------------------------------------------------------------
    // ボーダー・角丸設定
    // -------------------------------------------------------------------------
    borders: {
        "--border-main": "solid 1px var(--color-secondary)",
        "--radius-main": "10px"
    },

    // -------------------------------------------------------------------------
    // コードエディタテーマ（ダークテーマ）
    // -------------------------------------------------------------------------
    codeEditor: {
        "--code-bg": "#2d2d2d",
        "--code-text": "#f8f8f2",
        "--code-comment": "#75715e"
    },

    // -------------------------------------------------------------------------
    // 色覚配慮カラー（ワードボタン用）
    // -------------------------------------------------------------------------
    accessibility: {
        "--color-builtin": "#E65100",      // 組み込みワード（オレンジレッド）
        "--color-dependency": "#E69500",    // 依存カスタムワード（イエローオレンジ）
        "--color-non-dependency": "#009B68", // 非依存カスタムワード（ティールグリーン）
        "--color-stack": "#990099"          // スタック表示（マゼンタ）
    },

    // -------------------------------------------------------------------------
    // テーマを適用する関数
    // -------------------------------------------------------------------------
    apply: function() {
        const root = document.documentElement;

        // カラー適用
        for (const [key, value] of Object.entries(this.colors)) {
            root.style.setProperty(key, value);
        }

        // グラデーション適用
        for (const [key, value] of Object.entries(this.gradients)) {
            root.style.setProperty(key, value);
        }

        // 見出し適用
        for (const [key, value] of Object.entries(this.headings)) {
            root.style.setProperty(key, value);
        }

        // ボーダー・角丸適用
        for (const [key, value] of Object.entries(this.borders)) {
            root.style.setProperty(key, value);
        }

        // コードエディタテーマ適用
        for (const [key, value] of Object.entries(this.codeEditor)) {
            root.style.setProperty(key, value);
        }

        // 色覚配慮カラー適用
        for (const [key, value] of Object.entries(this.accessibility)) {
            root.style.setProperty(key, value);
        }
    },

    // -------------------------------------------------------------------------
    // 全設定を一つのオブジェクトとして取得
    // -------------------------------------------------------------------------
    getAll: function() {
        return {
            ...this.colors,
            ...this.gradients,
            ...this.headings,
            ...this.borders,
            ...this.codeEditor,
            ...this.accessibility
        };
    }
};

// グローバルに公開
if (typeof window !== 'undefined') {
    window.AjisaiTheme = AjisaiTheme;
}
