(function () {
    const createDefaultConfig = () => ({
        meta: { title: 'Ajisai' },
        project: {
            name: 'Ajisai Programming Language',
            author: 'masamoto yamashiro',
            url: 'https://masamoto1982.github.io/Ajisai/',
            repository: 'https://github.com/masamoto1982/Ajisai'
        },
        serviceMenu: [
            { label: 'Syntax', link: 'syntax.html' },
            { label: 'Built-in Words', link: 'words.html' },
            { label: 'Data Model', link: 'types.html' },
            { label: 'Control Flow', link: 'control.html' },
            { label: 'Higher-Order', link: 'higher-order.html' }
        ],
        referenceMenu: [
            { label: 'Examples', link: 'examples.html' },
            { label: 'GitHub', link: 'https://github.com/masamoto1982/Ajisai' },
            { label: 'Demo', link: 'https://masamoto1982.github.io/Ajisai/' }
        ],
        version: '202604080203'
    });

    const renderDocsShell = (root, config) => {
        const headerEl = root.getElementById('js-header');
        if (headerEl) {
            headerEl.innerHTML = `<div class="app-header-top"><a href="https://masamoto1982.github.io/Ajisai/" class="app-brand-block" aria-label="Ajisai"><span class="logo-swap" aria-hidden="true"><img src="../images/ajisai-logo-thumbnail-w40.jpg" alt="" class="logo logo-default"><img src="../images/ajisai-qr.png" alt="" class="logo logo-qr"></span><div class="app-brand-meta"><h1>${config.meta.title}</h1><span class="version">ver.${config.version}</span></div></a></div>`;
        }

        const sideNavEl = root.getElementById('js-side-nav');
        if (sideNavEl) {
            const renderItem = (item) => `<li><a href="${item.link}"${item.link.startsWith('http') ? ' target="_blank" rel="noopener noreferrer"' : ''}>${item.label}${item.link.startsWith('http') ? ' &#x2197;' : ''}</a></li>`;
            sideNavEl.innerHTML = `<div class="nav-section"><p class="nav-section-title">Reference</p><ul>${config.serviceMenu.map(renderItem).join('')}</ul></div><div class="nav-section"><p class="nav-section-title">Links</p><ul>${config.referenceMenu.map(renderItem).join('')}</ul></div>`;
        }

        const footerEl = root.getElementById('js-footer');
        if (footerEl) {
            footerEl.innerHTML = `<span>&copy; ${new Date().getFullYear()} ${config.project.author}</span><a href="${config.project.repository}" target="_blank" rel="noopener noreferrer">GitHub</a>`;
        }
    };

    document.addEventListener('DOMContentLoaded', () => {
        const config = createDefaultConfig();
        renderDocsShell(document, config);
    });
})();
