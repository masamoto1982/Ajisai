const escapeInline = (text: string, parent: ParentNode): void => {
    const pattern = /`([^`]+)`/g;
    let lastIndex = 0;
    let match: RegExpExecArray | null;
    while ((match = pattern.exec(text)) !== null) {
        if (match.index > lastIndex) {
            parent.appendChild(document.createTextNode(text.slice(lastIndex, match.index)));
        }
        const code = document.createElement('code');
        code.className = 'md-inline-code';
        code.textContent = match[1] ?? '';
        parent.appendChild(code);
        lastIndex = match.index + match[0].length;
    }
    if (lastIndex < text.length) {
        parent.appendChild(document.createTextNode(text.slice(lastIndex)));
    }
};

const renderHeading = (level: number, text: string, container: ParentNode): void => {
    const tag = `h${Math.min(level, 6)}` as keyof HTMLElementTagNameMap;
    const heading = document.createElement(tag);
    heading.className = `md-heading md-h${level}`;
    escapeInline(text, heading);
    container.appendChild(heading);
};

const renderCodeBlock = (lang: string, lines: string[], container: ParentNode): void => {
    const pre = document.createElement('pre');
    pre.className = `md-code-block md-code-${lang || 'plain'}`;
    const code = document.createElement('code');
    if (lang) code.className = `language-${lang}`;
    code.textContent = lines.join('\n');
    pre.appendChild(code);
    container.appendChild(pre);
};

const renderList = (items: string[], container: ParentNode): void => {
    const ul = document.createElement('ul');
    ul.className = 'md-list';
    items.forEach(item => {
        const li = document.createElement('li');
        escapeInline(item, li);
        ul.appendChild(li);
    });
    container.appendChild(ul);
};

const renderParagraph = (lines: string[], container: ParentNode): void => {
    const p = document.createElement('p');
    p.className = 'md-paragraph';
    escapeInline(lines.join(' '), p);
    container.appendChild(p);
};

export const renderMarkdownToFragment = (markdown: string): DocumentFragment => {
    const fragment = document.createDocumentFragment();
    const lines = markdown.split('\n');
    let i = 0;
    let paragraph: string[] = [];
    let listItems: string[] = [];

    const flushParagraph = (): void => {
        if (paragraph.length > 0) {
            renderParagraph(paragraph, fragment);
            paragraph = [];
        }
    };
    const flushList = (): void => {
        if (listItems.length > 0) {
            renderList(listItems, fragment);
            listItems = [];
        }
    };

    while (i < lines.length) {
        const line = lines[i] ?? '';
        const fenceMatch = line.match(/^```(\w*)\s*$/);
        if (fenceMatch) {
            flushParagraph();
            flushList();
            const lang = fenceMatch[1] ?? '';
            const codeLines: string[] = [];
            i++;
            while (i < lines.length && !(lines[i] ?? '').match(/^```\s*$/)) {
                codeLines.push(lines[i] ?? '');
                i++;
            }
            renderCodeBlock(lang, codeLines, fragment);
            i++;
            continue;
        }

        const headingMatch = line.match(/^(#{1,6})\s+(.+)$/);
        if (headingMatch) {
            flushParagraph();
            flushList();
            renderHeading((headingMatch[1] ?? '').length, headingMatch[2] ?? '', fragment);
            i++;
            continue;
        }

        const listMatch = line.match(/^[-*]\s+(.+)$/);
        if (listMatch) {
            flushParagraph();
            listItems.push(listMatch[1] ?? '');
            i++;
            continue;
        }

        if (line.trim() === '') {
            flushParagraph();
            flushList();
            i++;
            continue;
        }

        flushList();
        paragraph.push(line);
        i++;
    }
    flushParagraph();
    flushList();
    return fragment;
};
