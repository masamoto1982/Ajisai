document.addEventListener('DOMContentLoaded', () => {
    // 設定ファイルチェック（フォールバック付き）
    const c = (typeof AjisaiConfig !== 'undefined') ? AjisaiConfig : {
        // フォールバック設定
        primaryColor: '#6b5b95',
        meta: {
            title: "Ajisai",
            subTitle: "FORTH-inspired Stack-based Language",
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
            { label: "Data Types", link: "types.html" },
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
    // 0. テーマカラーの適用 (共通テーマから)
    // ------------------------------------------------------------------------
    if (typeof AjisaiTheme !== 'undefined') {
        AjisaiTheme.apply();
    } else if (c.theme) {
        // フォールバック: SiteConfigのテーマを使用
        const root = document.documentElement;
        for (const [key, value] of Object.entries(c.theme)) {
            root.style.setProperty(key, value);
        }
    }

    // ------------------------------------------------------------------------
    // 1. ヘッダー情報の生成 (#js-header)
    //    アプリ側と完全に同じレイアウト（画像ロゴ + タイトル + ナビボタン）
    // ------------------------------------------------------------------------
    const headerEl = document.getElementById('js-header');
    if (headerEl) {
        // ナビゲーションボタンを生成（アプリ側の.header-actionsと同様）
        const navButtons = c.globalMenu.map(item =>
            `<a href="${item.link}" class="header-btn">${item.label}</a>`
        ).join('');

        // Demoボタンを追加
        const demoButton = `<a href="${c.social.demo.url}" class="header-btn" target="_blank" rel="noopener noreferrer">Demo</a>`;

        headerEl.innerHTML = `
            <img src="../images/ajisai-logo-min_w40.jpg" alt="Ajisai Logo" class="logo">
            <h1>${c.meta.title}</h1>
            <span class="version">${c.meta.subTitle}</span>
            <div class="header-actions">
                ${navButtons}
                ${demoButton}
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
            <div class="item-1">
                <p class="label">Reference</p>
                <ul>${serviceItems}</ul>
            </div>
            <div class="item-2">
                <p class="label">Links</p>
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
            <a href="index.html">Reference</a>
        `;
    }

    // ------------------------------------------------------------------------
    // 5. スライドショー機能
    // ------------------------------------------------------------------------
    const slideshow = document.getElementById("slideshow");
    if (slideshow) {
        const images = slideshow.getElementsByTagName("img");
        if (images.length > 0) {
            let currentIndex = 0;
            function showImage(index) {
                Array.from(images).forEach((img) => img.style.opacity = 0);
                currentIndex = (index + images.length) % images.length;
                images[currentIndex].style.opacity = 1;
            }
            showImage(0);
            setInterval(() => showImage(currentIndex + 1), 5000);
        }
    }
});
