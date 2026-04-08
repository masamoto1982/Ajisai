document.addEventListener('DOMContentLoaded', () => {
    // 設定ファイルチェック（フォールバック付き）
    const c = (typeof AjisaiConfig !== 'undefined') ? AjisaiConfig : {
        // フォールバック設定
        primaryColor: '#6b5b95',
        meta: {
            title: "Ajisai",
            subTitle: "Fractional Dataflow Language",
            copyrightYear: new Date().getFullYear()
        },
        project: {
            name: "Ajisai Programming Language",
            shortName: "Ajisai",
            author: "masamoto yamashiro",
            url: "https://masamoto1982.github.io/Ajisai/",
            repository: "https://github.com/masamoto1982/Ajisai"
        },
        globalMenu: [
            { label: "Home", link: "index.html" },
            { label: "Philosophy", link: "philosophy.html" },
            { label: "About", link: "about.html" },
            { label: "Tutorial", link: "tutorial.html" }
        ],
        serviceMenu: [
            { label: "Syntax", link: "syntax.html" },
            { label: "Built-in Words", link: "words.html" },
            { label: "Data Model", link: "types.html" },
            { label: "Control Flow", link: "control.html" },
            { label: "Higher-Order", link: "higher-order.html" }
        ],
        referenceMenu: [
            { label: "Examples", link: "examples.html" },
            { label: "GitHub", link: "https://github.com/masamoto1982/Ajisai" },
            { label: "Demo", link: "https://masamoto1982.github.io/Ajisai/" }
        ],
        social: {
            github: { url: "https://github.com/masamoto1982/Ajisai", label: "GitHub" },
            demo: { url: "https://masamoto1982.github.io/Ajisai/", label: "Try Demo" }
        }
    };

    // ------------------------------------------------------------------------
    // 0. テーマカラーの適用
    //    <head> のインラインスクリプトが <style id="theme-vars"> 経由で適用済み。
    //    ここで AjisaiTheme.apply() を呼ぶとインラインスタイルとして上書きされ、
    //    docs-reference-styles.css による CSS 変数オーバーライド（黄金比等）が
    //    無効になるため、呼び出しを行わない。
    // ------------------------------------------------------------------------

    // ------------------------------------------------------------------------
    // 1. ヘッダー情報の生成 (#js-header)
    //    アプリ側と完全に同じレイアウト（画像ロゴ + タイトル + ナビボタン）
    // ------------------------------------------------------------------------
    const headerEl = document.getElementById('js-header');
    if (headerEl) {
        // メイン画面と同一のヘッダー構造を生成
        headerEl.innerHTML = `
            <div class="app-header-top">
                <a href="https://masamoto1982.github.io/Ajisai/" class="app-brand-block" aria-label="Ajisai">
                    <img src="../images/ajisai-logo-thumbnail-w40.jpg" alt="Ajisai Logo" class="logo">
                    <div class="app-brand-meta">
                        <h1>${c.meta.title}</h1>
                        <span class="version">ver.202604080203</span>
                    </div>
                </a>
            </div>
            <div class="header-actions">
                <a href="index.html" class="reference-btn">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
                        <path d="M9 9h6v6M15 9l-6 6M5 3h14a2 2 0 012 2v14a2 2 0 01-2 2H5a2 2 0 01-2-2V5a2 2 0 012-2z"/>
                    </svg>
                    Reference
                </a>
                <a href="${c.social.demo.url}" class="test-btn" target="_blank" rel="noopener noreferrer">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
                        <path d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/>
                    </svg>
                    Test
                </a>
            </div>
        `;
    }

    // ------------------------------------------------------------------------
    // 2. サイドバーの生成 (#js-side-nav)
    // ------------------------------------------------------------------------
    const sideNavEl = document.getElementById('js-side-nav');
    if (sideNavEl) {
        const serviceItems = c.serviceMenu.map(item =>
            `<li><a href="${item.link}">${item.label}</a></li>`
        ).join('');

        const refItems = c.referenceMenu.map(item => {
            const isExternal = item.link.startsWith('http');
            const target = isExternal ? ' target="_blank" rel="noopener noreferrer"' : '';
            const icon = isExternal ? ' &#x2197;' : '';
            return `<li><a href="${item.link}"${target}>${item.label}${icon}</a></li>`;
        }).join('');

        const socialHtml = `
            <ul class="social-links">
                <li><a href="${c.social.github.url}" target="_blank" rel="noopener noreferrer">${c.social.github.label}</a></li>
                <li><a href="${c.social.demo.url}" target="_blank" rel="noopener noreferrer">${c.social.demo.label}</a></li>
            </ul>
        `;

        sideNavEl.innerHTML = `
            <div class="nav-section">
                <p class="nav-section-title">Reference</p>
                <ul>${serviceItems}</ul>
            </div>
            <div class="nav-section">
                <p class="nav-section-title">Links</p>
                <ul>${refItems}</ul>
            </div>
            ${socialHtml}
        `;
    }

    // ------------------------------------------------------------------------
    // 3. プロジェクト情報テーブル (#js-company-table)
    // ------------------------------------------------------------------------
    const companyTableEl = document.getElementById('js-company-table');
    if (companyTableEl) {
        companyTableEl.innerHTML = `
            <table>
                <tr><th>Project</th><td>${c.project.name}</td></tr>
                <tr><th>Author</th><td>${c.project.author}</td></tr>
                <tr><th>License</th><td>MIT License</td></tr>
                <tr>
                    <th>Repository</th>
                    <td><a href="https://github.com/masamoto1982/Ajisai" target="_blank" rel="noopener noreferrer">github.com/masamoto1982/Ajisai</a></td>
                </tr>
                <tr>
                    <th>Demo</th>
                    <td><a href="${c.project.url}" target="_blank" rel="noopener noreferrer">${c.project.url}</a></td>
                </tr>
                <tr>
                    <th>Technology</th>
                    <td>Rust + WebAssembly + TypeScript</td>
                </tr>
            </table>
        `;
    }

    // ------------------------------------------------------------------------
    // 4. フッター (#js-footer)
    //    アプリ側と同じ形式（年は自動更新）
    // ------------------------------------------------------------------------
    const footerEl = document.getElementById('js-footer');
    if (footerEl) {
        const currentYear = new Date().getFullYear();
        footerEl.innerHTML = `
            <span>&copy; ${currentYear} ${c.project.author}</span>
            <a href="https://github.com/masamoto1982/Ajisai" target="_blank" rel="noopener noreferrer">GitHub</a>
        `;
    }

});
