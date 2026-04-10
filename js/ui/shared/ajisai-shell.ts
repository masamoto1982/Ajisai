import { renderAjisaiHeader } from './header-view';

export interface NavItem {
    readonly label: string;
    readonly link: string;
}

export interface DocsShellConfig {
    readonly meta: { readonly title: string };
    readonly project: { readonly name: string; readonly author: string; readonly url: string; readonly repository: string };
    readonly serviceMenu: readonly NavItem[];
    readonly referenceMenu: readonly NavItem[];
    readonly social: {
        readonly github: { readonly url: string; readonly label: string };
        readonly demo: { readonly url: string; readonly label: string };
    };
    readonly version: string;
}

const defaultConfig: DocsShellConfig = {
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
    social: {
        github: { url: 'https://github.com/masamoto1982/Ajisai', label: 'GitHub' },
        demo: { url: 'https://masamoto1982.github.io/Ajisai/', label: 'Try Demo' }
    },
    version: '202604080203'
};

export const renderDocsShell = (root: ParentNode, config: DocsShellConfig = defaultConfig): void => {
    const headerEl = root.querySelector('#js-header');
    if (headerEl instanceof HTMLElement) {
        renderAjisaiHeader(headerEl, {
            mode: 'reference',
            version: config.version,
            assetsPath: '../images',
            referenceHref: 'index.html'
        });
    }

    const sideNavEl = root.querySelector('#js-side-nav');
    if (sideNavEl instanceof HTMLElement) {
        const renderItem = (item: NavItem): string => `<li><a href="${item.link}"${item.link.startsWith('http') ? ' target="_blank" rel="noopener noreferrer"' : ''}>${item.label}${item.link.startsWith('http') ? ' &#x2197;' : ''}</a></li>`;
        sideNavEl.innerHTML = `<div class="nav-section"><p class="nav-section-title">Reference</p><ul>${config.serviceMenu.map(renderItem).join('')}</ul></div><div class="nav-section"><p class="nav-section-title">Links</p><ul>${config.referenceMenu.map(renderItem).join('')}</ul></div>`;
    }

    const footerEl = root.querySelector('#js-footer');
    if (footerEl instanceof HTMLElement) {
        footerEl.innerHTML = `<span>&copy; ${new Date().getFullYear()} ${config.project.author}</span><a href="${config.project.repository}" target="_blank" rel="noopener noreferrer">GitHub</a>`;
    }
};
