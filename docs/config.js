// Ajisai Documentation Site Configuration
const SiteConfig = {
    meta: {
        title: "Ajisai",
        subTitle: "FORTH-inspired Stack-based Programming Language",
        description: "Ajisai - A stack-based programming language inspired by FORTH, running on WebAssembly with a web-based interactive GUI.",
        keywords: "Ajisai, FORTH, stack-based, programming language, WebAssembly, Rust, TypeScript"
    },

    company: {
        name: "masamoto yamashiro",
        url: "https://github.com/masamoto1982",
        copyright: "2025 masamoto yamashiro"
    },

    repository: {
        url: "https://github.com/masamoto1982/Ajisai",
        demo: "https://masamoto1982.github.io/Ajisai/"
    },

    globalMenu: [
        { label: "Home", link: "index.html" },
        { label: "Philosophy", link: "philosophy.html" },
        { label: "About", link: "about.html" },
        { label: "Demo", link: "https://masamoto1982.github.io/Ajisai/", external: true },
        { label: "Repository", link: "https://github.com/masamoto1982/Ajisai", external: true }
    ],

    serviceMenu: [
        { label: "Syntax", link: "syntax.html" },
        { label: "Built-in Words", link: "words.html" },
        { label: "Data Types", link: "types.html" },
        { label: "Control Flow", link: "control.html" },
        { label: "Higher-Order Functions", link: "higher-order.html" }
    ],

    referenceMenu: [
        { label: "Examples", link: "examples.html" },
        { label: "Tutorial", link: "tutorial.html" }
    ],

    theme: {
        primaryColor: "#6b5b95",
        secondaryColor: "#88b04b",
        accentColor: "#f7cac9",
        textColor: "#333333",
        backgroundColor: "#ffffff",
        codeBackground: "#f4f4f4"
    }
};

// Generate header
function generateHeader() {
    const header = document.getElementById('site-header');
    if (!header) return;

    let menuHtml = SiteConfig.globalMenu.map(item => {
        const external = item.external ? ' target="_blank" rel="noopener noreferrer"' : '';
        const icon = item.external ? ' <span class="external-icon">&#x2197;</span>' : '';
        return `<a href="${item.link}"${external}>${item.label}${icon}</a>`;
    }).join('');

    header.innerHTML = `
        <div class="header-container">
            <div class="logo">
                <a href="index.html">
                    <span class="logo-text">${SiteConfig.meta.title}</span>
                    <span class="logo-sub">${SiteConfig.meta.subTitle}</span>
                </a>
            </div>
            <nav class="global-nav">
                ${menuHtml}
            </nav>
            <button class="menu-toggle" onclick="toggleMobileMenu()">
                <span></span><span></span><span></span>
            </button>
        </div>
        <nav class="mobile-nav" id="mobile-nav">
            ${menuHtml}
        </nav>
    `;
}

// Generate sidebar
function generateSidebar() {
    const sidebar = document.getElementById('sidebar');
    if (!sidebar) return;

    let serviceHtml = SiteConfig.serviceMenu.map(item =>
        `<a href="${item.link}">${item.label}</a>`
    ).join('');

    let referenceHtml = SiteConfig.referenceMenu.map(item =>
        `<a href="${item.link}">${item.label}</a>`
    ).join('');

    sidebar.innerHTML = `
        <div class="sidebar-section">
            <h3>Language Reference</h3>
            ${serviceHtml}
        </div>
        <div class="sidebar-section">
            <h3>Learning</h3>
            ${referenceHtml}
        </div>
    `;
}

// Generate footer
function generateFooter() {
    const footer = document.getElementById('site-footer');
    if (!footer) return;

    footer.innerHTML = `
        <div class="footer-container">
            <div class="footer-links">
                <a href="${SiteConfig.repository.url}" target="_blank" rel="noopener noreferrer">GitHub</a>
                <a href="${SiteConfig.repository.demo}" target="_blank" rel="noopener noreferrer">Demo</a>
            </div>
            <p class="copyright">
                &copy; ${SiteConfig.company.copyright}.
                Licensed under <a href="https://opensource.org/licenses/MIT" target="_blank" rel="noopener noreferrer">MIT License</a>.
            </p>
        </div>
    `;
}

// Toggle mobile menu
function toggleMobileMenu() {
    const mobileNav = document.getElementById('mobile-nav');
    if (mobileNav) {
        mobileNav.classList.toggle('open');
    }
}

// Initialize site
document.addEventListener('DOMContentLoaded', function() {
    generateHeader();
    generateSidebar();
    generateFooter();

    // Highlight current page in navigation
    const currentPage = window.location.pathname.split('/').pop() || 'index.html';
    document.querySelectorAll('.global-nav a, .sidebar-section a').forEach(link => {
        if (link.getAttribute('href') === currentPage) {
            link.classList.add('active');
        }
    });
});
