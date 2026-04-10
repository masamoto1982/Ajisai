(function (global) {
    const renderHeader = (root, options) => {
        const mode = options?.mode || 'web';
        const version = options?.version || '202604102001';
        const assetsPath = options?.assetsPath || './public/images';
        const referenceHref = options?.referenceHref || 'docs/index.html';

        const normalizedAssetsPath = assetsPath.replace(/\/+$/, '');
        const logoDefaultSrc = new URL(`${normalizedAssetsPath}/ajisai-logo-thumbnail-w40.jpg`, window.location.href).toString();
        const logoQrSrc = new URL(`${normalizedAssetsPath}/ajisai-qr.png`, window.location.href).toString();

        const headerHtml = `
            <div class="app-header-top">
                <a href="https://masamoto1982.github.io/Ajisai/" class="app-brand-block" aria-label="Ajisai">
                    <span class="logo-swap" aria-hidden="true">
                        <img src="${logoDefaultSrc}" alt="" class="logo logo-default">
                        <img src="${logoQrSrc}" alt="" class="logo logo-qr">
                    </span>
                    <div class="app-brand-meta">
                        <h1>Ajisai</h1>
                        <span class="version">ver.${version}</span>
                    </div>
                </a>
                ${mode === 'web' ? '<span id="offline-indicator" class="offline-indicator" style="display: none;">Offline</span>' : ''}
            </div>
            <div class="header-actions">
                <a href="${referenceHref}" class="reference-btn" ${mode === 'web' ? 'target="_blank"' : ''}>
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
                        <path d="M9 9h6v6M15 9l-6 6M5 3h14a2 2 0 012 2v14a2 2 0 01-2 2H5a2 2 0 01-2-2V5a2 2 0 012-2z"/>
                    </svg>
                    Reference
                </a>
                ${mode === 'web'
                    ? `<button id="test-btn" class="test-btn" type="button">
                        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
                            <path d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/>
                        </svg>
                        Test
                    </button>`
                    : `<a href="https://masamoto1982.github.io/Ajisai/" class="test-btn" target="_blank" rel="noopener noreferrer">
                        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
                            <path d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/>
                        </svg>
                        Test
                    </a>`}
            </div>
        `;

        root.innerHTML = headerHtml;
    };

    global.AjisaiSharedUI = {
        renderHeader
    };
})(window);
