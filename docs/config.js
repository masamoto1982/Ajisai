/**
 * ============================================================================
 * Ajisai Documentation Site Configuration
 * ============================================================================
 */

const SiteConfig = {
    // -------------------------------------------------------------------------
    // 1. デザインテーマ設定 (Color Palette)
    // -------------------------------------------------------------------------
    theme: {
        "--color-primary": "#6b5b95",   // 紫陽花カラー（メイン）
        "--color-secondary": "#8b7db5", // 紫陽花カラー（アクセント）
        "--color-light": "#f8f7fc",     // 背景色（明るい）
        "--color-medium": "#d4cfe8",    // 背景色（中間）
        "--color-dark": "#e8e4f3"       // 背景色（濃いめ）
    },

    // -------------------------------------------------------------------------
    // 2. サイト基本情報
    // -------------------------------------------------------------------------
    meta: {
        title: "Ajisai",
        subTitle: "FORTH-inspired Stack-based Language",
        copyrightYear: new Date().getFullYear()
    },

    // -------------------------------------------------------------------------
    // 3. プロジェクト情報
    // -------------------------------------------------------------------------
    company: {
        name: "Ajisai Programming Language",
        englishName: "Ajisai",
        representative: "masamoto yamashiro",
        postCode: "",
        address: "",
        tel: "",
        fax: "",
        email: "",
        url: "https://masamoto1982.github.io/Ajisai/",
        mapUrl: ""
    },

    // -------------------------------------------------------------------------
    // 4. ナビゲーションメニュー設定
    // -------------------------------------------------------------------------
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

    // -------------------------------------------------------------------------
    // 5. ソーシャルリンク設定
    // -------------------------------------------------------------------------
    social: {
        github: { url: "https://github.com/masamoto1982/Ajisai", label: "GitHub" },
        demo: { url: "https://masamoto1982.github.io/Ajisai/", label: "Try Demo" }
    }
};
