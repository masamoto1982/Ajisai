document.addEventListener('DOMContentLoaded', () => {
    // 設定ファイルチェック
    if (typeof SiteConfig === 'undefined') {
        console.error("Config file (SiteConfig) is not loaded.");
        return;
    }

    const c = SiteConfig;

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
    // ------------------------------------------------------------------------
    const headerEl = document.getElementById('js-header');
    if (headerEl) {
        headerEl.innerHTML = `
            <div class="logo">
                <a href="index.html">
                    <img src="images/logo.png" class="icon logo-icon" alt="${c.meta.title} Logo">
                </a>
            </div>
            <div class="title-area">
                <h1>
                    <span class="subtitle">${c.meta.subTitle}</span>
                    <a href="index.html">${c.meta.title}</a>
                </h1>
                <address>
                    A stack-based programming language inspired by FORTH<br>
                    Running on WebAssembly with Rust + TypeScript
                </address>
            </div>
        `;
    }

    // ------------------------------------------------------------------------
    // 2. グローバルナビゲーションの生成 (#js-global-nav)
    // ------------------------------------------------------------------------
    const navEl = document.getElementById('js-global-nav');
    if (navEl) {
        const listItems = c.globalMenu.map(item =>
            `<li><a href="${item.link}">${item.label}</a></li>`
        ).join('');
        navEl.innerHTML = `<ul class="menu">${listItems}</ul>`;
    }

    // ------------------------------------------------------------------------
    // 3. サイドバーの生成 (#js-side-nav)
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
    // 4. プロジェクト情報テーブル (#js-company-table)
    // ------------------------------------------------------------------------
    const companyTableEl = document.getElementById('js-company-table');
    if (companyTableEl) {
        companyTableEl.innerHTML = `
            <table>
                <tr><th>Project</th><td>${c.company.name}</td></tr>
                <tr><th>Author</th><td>${c.company.representative}</td></tr>
                <tr><th>License</th><td>MIT License</td></tr>
                <tr>
                    <th>Repository</th>
                    <td><a href="https://github.com/masamoto1982/Ajisai" target="_blank" rel="noopener noreferrer">github.com/masamoto1982/Ajisai</a></td>
                </tr>
                <tr>
                    <th>Demo</th>
                    <td><a href="${c.company.url}" target="_blank" rel="noopener noreferrer">${c.company.url}</a></td>
                </tr>
                <tr>
                    <th>Technology</th>
                    <td>Rust + WebAssembly + TypeScript</td>
                </tr>
            </table>
        `;
    }

    // ------------------------------------------------------------------------
    // 5. フッター (#js-footer)
    // ------------------------------------------------------------------------
    const footerEl = document.getElementById('js-footer');
    if (footerEl) {
        footerEl.innerHTML = `
            <a href="index.html" target="_self">
                &copy;<time>${c.meta.copyrightYear}</time> ${c.company.representative} - ${c.meta.title}
            </a>
            &nbsp;|&nbsp;
            <a href="https://github.com/masamoto1982/Ajisai" target="_blank" rel="noopener noreferrer">GitHub</a>
            &nbsp;|&nbsp;
            <a href="https://masamoto1982.github.io/Ajisai/" target="_blank" rel="noopener noreferrer">Demo</a>
        `;
    }

    // ------------------------------------------------------------------------
    // 6. スライドショー機能
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
