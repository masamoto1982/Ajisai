/**
 * ============================================================================
 * Ajisai 共通テーマ設定
 * ============================================================================
 * このファイルを編集することで、アプリとReferenceサイトの
 * カラーテーマを一括で変更できます。
 *
 * 背景色を指定すると、文字色は自動的に計算されます。
 * - 暗い背景 → 明るい文字色
 * - 明るい背景 → 暗い文字色
 */

const AjisaiTheme = {
    // =========================================================================
    // カラーユーティリティ関数
    // =========================================================================

    // HEX → RGB 変換
    hexToRgb: function(hex) {
        const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
        return result ? {
            r: parseInt(result[1], 16),
            g: parseInt(result[2], 16),
            b: parseInt(result[3], 16)
        } : null;
    },

    // RGB → HEX 変換
    rgbToHex: function(r, g, b) {
        return '#' + [r, g, b].map(x => {
            const hex = Math.max(0, Math.min(255, Math.round(x))).toString(16);
            return hex.length === 1 ? '0' + hex : hex;
        }).join('');
    },

    // 相対輝度を計算 (0 = 黒, 1 = 白)
    getLuminance: function(hex) {
        const rgb = this.hexToRgb(hex);
        if (!rgb) return 0.5;
        const [r, g, b] = [rgb.r, rgb.g, rgb.b].map(v => {
            v /= 255;
            return v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4);
        });
        return 0.2126 * r + 0.7152 * g + 0.0722 * b;
    },

    // 色が明るいかどうか判定 (閾値: 0.5)
    isLight: function(hex) {
        return this.getLuminance(hex) > 0.5;
    },

    // 色を明るくする
    lighten: function(hex, amount) {
        const rgb = this.hexToRgb(hex);
        if (!rgb) return hex;
        return this.rgbToHex(
            rgb.r + (255 - rgb.r) * amount,
            rgb.g + (255 - rgb.g) * amount,
            rgb.b + (255 - rgb.b) * amount
        );
    },

    // 色を暗くする
    darken: function(hex, amount) {
        const rgb = this.hexToRgb(hex);
        if (!rgb) return hex;
        return this.rgbToHex(
            rgb.r * (1 - amount),
            rgb.g * (1 - amount),
            rgb.b * (1 - amount)
        );
    },

    // 背景色に対するコントラスト文字色を取得
    getContrastText: function(bgHex, lightText = '#ffffff', darkText = '#333333') {
        return this.isLight(bgHex) ? darkText : lightText;
    },

    // グラデーションから基準色を抽出
    extractBaseColor: function(gradient) {
        const hexMatch = gradient.match(/#[a-fA-F0-9]{6}/);
        return hexMatch ? hexMatch[0] : '#888888';
    },

    // 基調色を含んだ暗い色を生成（コードエディタ用）
    // 黒ベースに基調色を少し混ぜる
    getTintedDark: function(hex, baseValue = 0.15, tintAmount = 0.3) {
        const rgb = this.hexToRgb(hex);
        if (!rgb) return '#2d2d2d';
        // ベースの暗さ + 基調色のティント
        return this.rgbToHex(
            baseValue * 255 + rgb.r * tintAmount,
            baseValue * 255 + rgb.g * tintAmount,
            baseValue * 255 + rgb.b * tintAmount
        );
    },

    // =========================================================================
    // テーマ設定（ここを編集してテーマを変更）
    // =========================================================================

    // プライマリカラー（テーマの基準色）
    // primaryColor: '#6b5b95',  // ディープパープル（紫陽花カラー）
    primaryColor: '#8B0000',  // 深紅（テスト用）

    // 各エリアの背景色設定
    // ディープパープル（紫陽花）テーマ:
    // backgrounds: {
    //     header: '#6b5b95',
    //     page: '#f8f7fc',
    //     parent: '#e8e4f3',
    //     child: '#f0eef7',
    //     menu: '#7b6ba5'
    // },
    // 深紅テーマ（テスト用）:
    backgrounds: {
        header: '#8B0000',        // ヘッダー背景（深紅）
        page: '#fef8f8',          // ページ背景（淡いピンク）
        parent: '#f5e6e6',        // 親エリア背景
        child: '#faf0f0',         // 子エリア背景
        menu: '#a52a2a'           // メニューバー背景（ブラウン系）
    },

    // =========================================================================
    // 自動計算されたCSS変数を生成
    // =========================================================================

    generateVariables: function() {
        const bg = this.backgrounds;
        const primary = this.primaryColor;

        // 各背景に対するテキスト色を自動計算
        const headerText = this.getContrastText(bg.header);
        const pageText = this.getContrastText(bg.page);
        const parentText = this.getContrastText(bg.parent);
        const childText = this.getContrastText(bg.child);
        const menuText = this.getContrastText(bg.menu);

        // H1, H2, H3 の階層的な色を計算
        // ヘッダー用（暗い背景なら明るく、明るい背景なら暗く）
        const h1Color = headerText;

        // コンテンツエリア用（親エリアの背景に基づく）
        const isParentLight = this.isLight(bg.parent);
        const h2Color = isParentLight ? this.darken(primary, 0.1) : this.lighten(primary, 0.3);
        const h3Color = isParentLight ? this.lighten(primary, 0.1) : this.lighten(primary, 0.5);

        // セカンダリカラー（プライマリを少し明るく）
        const secondary = this.lighten(primary, 0.2);

        return {
            // カラーパレット
            "--color-primary": primary,
            "--color-secondary": secondary,
            "--color-light": bg.page,
            "--color-medium": this.darken(bg.page, 0.1),
            "--color-dark": bg.parent,
            "--color-text": pageText,
            "--color-text-light": this.isLight(bg.page) ? '#666666' : '#aaaaaa',

            // 背景色
            "--gradient-header": bg.header,
            "--gradient-background": bg.page,
            "--gradient-parent": bg.parent,
            "--gradient-child": bg.child,
            "--gradient-menu": bg.menu,
            "--gradient-menu-hover": this.lighten(bg.menu, 0.15),

            // ヘッダー用テキスト色（階層構造・自動計算）
            // main: タイトル, secondary: サブタイトル/バージョン, tertiary: 説明文
            "--header-text": headerText,
            "--header-text-secondary": this.isLight(bg.header) ? '#555555' : '#cccccc',
            "--header-text-tertiary": this.isLight(bg.header) ? '#777777' : '#aaaaaa',

            // メニュー/ボタン用テキスト色（自動計算）
            "--menu-text": menuText,
            "--menu-text-hover": this.isLight(bg.menu) ? '#000000' : '#ffffff',

            // 見出し色（階層構造・自動計算）
            "--heading-h1": h1Color,
            "--heading-h2": h2Color,
            "--heading-h3": h3Color,

            // ボーダー・角丸
            "--border-main": `solid 1px ${secondary}`,
            "--radius-main": "10px",

            // コードエディタテーマ（基調色に連動）
            "--code-bg": this.getTintedDark(primary),
            "--code-text": "#f8f8f2",
            "--code-comment": this.lighten(primary, 0.3),

            // 色覚配慮カラー
            "--color-builtin": "#E65100",
            "--color-dependency": "#E69500",
            "--color-non-dependency": "#009B68",
            "--color-stack": "#990099"
        };
    },

    // =========================================================================
    // テーマを適用する関数
    // =========================================================================

    apply: function() {
        const root = document.documentElement;
        const vars = this.generateVariables();

        for (const [key, value] of Object.entries(vars)) {
            root.style.setProperty(key, value);
        }
    },

    // 全設定を一つのオブジェクトとして取得
    getAll: function() {
        return this.generateVariables();
    }
};

// グローバルに公開
if (typeof window !== 'undefined') {
    window.AjisaiTheme = AjisaiTheme;
}
