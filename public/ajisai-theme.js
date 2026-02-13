/**
 * ============================================================================
 * Ajisai テーマ計算エンジン
 * ============================================================================
 *
 * ajisai-config.js で指定された基調色（primaryColor）から、
 * 全ての背景色・文字色・枠線色を自動計算します。
 *
 * 【依存関係】
 *   このファイルより先に ajisai-config.js を読み込む必要があります。
 *
 * 【HTMLの階層構造に基づく色の濃さ】
 *   header/footer  = 基調色そのもの（最も濃い）
 *   body           = 基調色を極めて薄く（最も明るい）
 *   article        = 基調色を薄く
 *   section        = 基調色をやや薄く
 *   code           = 基調色を含んだ暗い色
 *
 * 【自動計算される色】
 *   文字色: 各エリアの背景色に基づいて自動決定（暗い背景→明るい文字）
 *   枠線色: 各エリアの背景色に基づいて自動決定（明るい背景→暗い枠線）
 */

const AjisaiTheme = {
    // =========================================================================
    // カラーユーティリティ関数
    // =========================================================================

    hexToRgb: function(hex) {
        const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
        return result ? {
            r: parseInt(result[1], 16),
            g: parseInt(result[2], 16),
            b: parseInt(result[3], 16)
        } : null;
    },

    rgbToHex: function(r, g, b) {
        return '#' + [r, g, b].map(x => {
            const hex = Math.max(0, Math.min(255, Math.round(x))).toString(16);
            return hex.length === 1 ? '0' + hex : hex;
        }).join('');
    },

    getLuminance: function(hex) {
        const rgb = this.hexToRgb(hex);
        if (!rgb) return 0.5;
        const [r, g, b] = [rgb.r, rgb.g, rgb.b].map(v => {
            v /= 255;
            return v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4);
        });
        return 0.2126 * r + 0.7152 * g + 0.0722 * b;
    },

    isLight: function(hex) {
        return this.getLuminance(hex) > 0.5;
    },

    lighten: function(hex, amount) {
        const rgb = this.hexToRgb(hex);
        if (!rgb) return hex;
        return this.rgbToHex(
            rgb.r + (255 - rgb.r) * amount,
            rgb.g + (255 - rgb.g) * amount,
            rgb.b + (255 - rgb.b) * amount
        );
    },

    darken: function(hex, amount) {
        const rgb = this.hexToRgb(hex);
        if (!rgb) return hex;
        return this.rgbToHex(
            rgb.r * (1 - amount),
            rgb.g * (1 - amount),
            rgb.b * (1 - amount)
        );
    },

    // 背景色に基づくメインテキスト色
    getTextColor: function(bgHex) {
        return this.isLight(bgHex) ? '#333333' : '#ffffff';
    },

    // 背景色に基づくセカンダリテキスト色（やや薄め）
    getTextColorSecondary: function(bgHex) {
        return this.isLight(bgHex) ? '#555555' : '#cccccc';
    },

    // 背景色に基づくターシャリテキスト色（さらに薄め）
    getTextColorTertiary: function(bgHex) {
        return this.isLight(bgHex) ? '#777777' : '#aaaaaa';
    },

    // 背景色に基づく枠線色（背景より濃い/薄い色）
    getBorderColor: function(bgHex) {
        // 明るい背景 → 暗い枠線、暗い背景 → 明るい枠線
        return this.isLight(bgHex) ? this.darken(bgHex, 0.25) : this.lighten(bgHex, 0.3);
    },

    // 基調色を含んだ暗い色を生成（コードエディタ用）
    getTintedDark: function(hex, baseValue = 0.12, tintAmount = 0.25) {
        const rgb = this.hexToRgb(hex);
        if (!rgb) return '#2d2d2d';
        return this.rgbToHex(
            baseValue * 255 + rgb.r * tintAmount,
            baseValue * 255 + rgb.g * tintAmount,
            baseValue * 255 + rgb.b * tintAmount
        );
    },

    // =========================================================================
    // 基調色の取得（ajisai-config.js から読み込み）
    // =========================================================================

    getPrimaryColor: function() {
        if (typeof AjisaiConfig !== 'undefined' && AjisaiConfig.primaryColor) {
            return AjisaiConfig.primaryColor;
        }
        // フォールバック（config.jsが読み込まれていない場合）
        console.warn('AjisaiConfig not found, using fallback color');
        return '#6b5b95';
    },

    // =========================================================================
    // 背景色の自動生成（HTMLの階層構造に対応）
    // =========================================================================

    getBackgrounds: function() {
        const p = this.getPrimaryColor();
        return {
            // header/footer: 基調色そのもの（最も濃い）
            header: p,
            // body: 極めて薄く（95%白に近づける）
            body: this.lighten(p, 0.94),
            // article/main: 薄く（85%白に近づける）
            article: this.lighten(p, 0.82),
            // section: やや薄く（88%白に近づける）
            section: this.lighten(p, 0.88),
            // menu/nav: ヘッダーより少し明るく
            menu: this.lighten(p, 0.12),
            // code: 暗いが基調色を含む
            code: this.getTintedDark(p)
        };
    },

    // =========================================================================
    // CSS変数の自動生成
    // =========================================================================

    generateVariables: function() {
        const primary = this.getPrimaryColor();
        const bg = this.getBackgrounds();
        const secondary = this.lighten(primary, 0.2);
        const rgb = this.hexToRgb(primary);

        return {
            // 基調色
            "--color-primary": primary,
            "--color-secondary": secondary,

            // 背景色（HTML階層に対応）
            "--bg-header": bg.header,
            "--bg-body": bg.body,
            "--bg-article": bg.article,
            "--bg-section": bg.section,
            "--bg-menu": bg.menu,
            "--bg-menu-hover": this.lighten(bg.menu, 0.15),
            "--bg-code": bg.code,
            "--bg-code-inline": `rgba(${rgb.r}, ${rgb.g}, ${rgb.b}, 0.1)`,

            // テキスト色（各背景に基づいて自動計算）
            "--text-on-header": this.getTextColor(bg.header),
            "--text-on-header-secondary": this.getTextColorSecondary(bg.header),
            "--text-on-header-tertiary": this.getTextColorTertiary(bg.header),
            "--text-on-body": this.getTextColor(bg.body),
            "--text-on-body-secondary": this.getTextColorSecondary(bg.body),
            "--text-on-article": this.getTextColor(bg.article),
            "--text-on-section": this.getTextColor(bg.section),
            "--text-on-menu": this.getTextColor(bg.menu),
            "--text-on-menu-hover": this.isLight(bg.menu) ? '#000000' : '#ffffff',
            "--text-on-code": "#f8f8f2",

            // 見出し色（コンテンツエリアの背景に基づく）
            "--heading-h1": this.getTextColor(bg.header),
            "--heading-h2": this.isLight(bg.article) ? this.darken(primary, 0.1) : this.lighten(primary, 0.3),
            "--heading-h3": this.isLight(bg.article) ? primary : this.lighten(primary, 0.5),

            // コード内コメント
            "--text-code-comment": this.lighten(primary, 0.4),

            // 枠線色（各エリアの背景に基づいて自動計算）
            "--border-on-header": this.getBorderColor(bg.header),
            "--border-on-body": this.getBorderColor(bg.body),
            "--border-on-article": this.getBorderColor(bg.article),
            "--border-on-section": this.getBorderColor(bg.section),
            "--border-on-menu": this.getBorderColor(bg.menu),
            "--border-on-code": this.getBorderColor(bg.code),

            // その他
            "--radius-main": "10px",

            // フォント（統一）
            "--font-stack-primary": '"Inter", -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Hiragino Kaku Gothic ProN", Meiryo, sans-serif',
            "--font-stack-mono": '"SF Mono", Consolas, "Liberation Mono", Monaco, monospace',

            // レイアウト（統一）
            "--max-width-container": "1200px",
            "--breakpoint-mobile": "768px",
            "--side-nav-flex-ratio": "1",
            "--main-content-flex-ratio": "3",

            // ====== 後方互換性のための変数（既存CSSとの互換） ======
            "--border-color": this.getBorderColor(bg.article),
            "--border-main": `solid 1px ${this.getBorderColor(bg.article)}`,
            "--color-secondary": this.getBorderColor(bg.article),
            "--gradient-header": bg.header,
            "--gradient-background": bg.body,
            "--gradient-parent": bg.article,
            "--gradient-child": bg.section,
            "--gradient-menu": bg.menu,
            "--gradient-menu-hover": this.lighten(bg.menu, 0.15),
            "--header-text": this.getTextColor(bg.header),
            "--header-text-secondary": this.getTextColorSecondary(bg.header),
            "--header-text-tertiary": this.getTextColorTertiary(bg.header),
            "--menu-text": this.getTextColor(bg.menu),
            "--menu-text-hover": this.isLight(bg.menu) ? '#000000' : '#ffffff',
            "--color-light": bg.body,
            "--color-medium": this.darken(bg.body, 0.1),
            "--color-dark": bg.article,
            "--color-text": this.getTextColor(bg.body),
            "--color-text-light": this.getTextColorSecondary(bg.body),
            "--code-bg": bg.code,
            "--code-text": "#f8f8f2",
            "--code-comment": this.lighten(primary, 0.4),
            "--code-inline-bg": `rgba(${rgb.r}, ${rgb.g}, ${rgb.b}, 0.1)`,

            // 色覚配慮カラー（固定）
            "--color-builtin": "#E65100",
            "--color-dependency": "#E69500",
            "--color-non-dependency": "#009B68",
            "--color-stack": "#990099",

            // シグネチャ型背景色（固定・色覚配慮）
            "--color-signature-map": "#E8F0FE",
            "--color-signature-form": "#E6F4EA",
            "--color-signature-fold": "#FDE8E0"
        };
    },

    // =========================================================================
    // テーマ適用
    // =========================================================================

    apply: function() {
        const root = document.documentElement;
        const vars = this.generateVariables();
        for (const [key, value] of Object.entries(vars)) {
            root.style.setProperty(key, value);
        }
    },

    getAll: function() {
        return this.generateVariables();
    }
};

if (typeof window !== 'undefined') {
    window.AjisaiTheme = AjisaiTheme;
}
