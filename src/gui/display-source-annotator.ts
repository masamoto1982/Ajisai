import type { Value } from '../wasm-interpreter-types';

type LiteralNode =
    | { readonly kind: 'number'; readonly source: string }
    | { readonly kind: 'vector'; readonly children: readonly LiteralNode[] }
    | { readonly kind: 'other' };

const NUMERIC_LITERAL_RE = /^[+-]?(?:(?:\d+(?:\.\d*)?)|(?:\.\d+))(?:\/[+-]?\d+)?(?:[eE][+-]?\d+)?$/;

const stripLineComment = (line: string): string => {
    let inString = false;
    for (let i = 0; i < line.length; i++) {
        const ch = line[i];
        if (ch === "'") {
            inString = !inString;
            continue;
        }
        if (!inString && ch === '#') return line.slice(0, i);
        if (!inString && ch === '/' && line[i + 1] === '/') return line.slice(0, i);
    }
    return line;
};

const tokenizeSource = (source: string): string[] => {
    const code = source.split('\n').map(stripLineComment).join('\n');
    const tokens: string[] = [];
    let i = 0;

    while (i < code.length) {
        const ch = code[i]!;
        if (/\s/.test(ch)) {
            i++;
            continue;
        }
        if (ch === '[' || ch === ']') {
            tokens.push(ch);
            i++;
            continue;
        }
        if (ch === "'") {
            let j = i + 1;
            while (j < code.length && code[j] !== "'") j++;
            if (j >= code.length) return [];
            tokens.push(code.slice(i, j + 1));
            i = j + 1;
            continue;
        }
        let j = i;
        while (j < code.length && !/\s/.test(code[j]!) && code[j] !== '[' && code[j] !== ']') {
            j++;
        }
        tokens.push(code.slice(i, j));
        i = j;
    }

    return tokens;
};

const parseLiteralAt = (tokens: readonly string[], start: number): { node: LiteralNode; next: number } | null => {
    const token = tokens[start];
    if (!token) return null;

    if (token === '[') {
        const children: LiteralNode[] = [];
        let next = start + 1;
        while (next < tokens.length && tokens[next] !== ']') {
            const parsed = parseLiteralAt(tokens, next);
            if (!parsed) return null;
            children.push(parsed.node);
            next = parsed.next;
        }
        if (tokens[next] !== ']') return null;
        return { node: { kind: 'vector', children }, next: next + 1 };
    }

    if (token === ']') return null;
    if (NUMERIC_LITERAL_RE.test(token)) return { node: { kind: 'number', source: token }, next: start + 1 };
    if (/^'.*'$/.test(token)) return { node: { kind: 'other' }, next: start + 1 };
    if (/^(?:TRUE|FALSE|NIL)$/i.test(token)) return { node: { kind: 'other' }, next: start + 1 };
    return null;
};

const parseLiteralOnlySource = (source: string): LiteralNode[] | null => {
    const tokens = tokenizeSource(source);
    if (tokens.length === 0) return null;

    const nodes: LiteralNode[] = [];
    let next = 0;
    while (next < tokens.length) {
        const parsed = parseLiteralAt(tokens, next);
        if (!parsed) return null;
        nodes.push(parsed.node);
        next = parsed.next;
    }
    return nodes;
};

const cloneValue = (value: Value): Value => ({
    ...value,
    value: Array.isArray(value.value)
        ? value.value.map(cloneValue)
        : value.value && typeof value.value === 'object'
            ? { ...value.value }
            : value.value
});

const collectNumericLiterals = (literal: LiteralNode, out: string[]): void => {
    if (literal.kind === 'number') {
        out.push(literal.source);
        return;
    }
    if (literal.kind === 'vector') {
        literal.children.forEach(child => collectNumericLiterals(child, out));
    }
};

const annotateTensorLikeValue = (value: Value, literal: LiteralNode): Value => {
    const numericLiterals: string[] = [];
    collectNumericLiterals(literal, numericLiterals);
    if (numericLiterals.length === 0 || !value.value || typeof value.value !== 'object') return value;

    const tensor = value.value as { data?: unknown[] };
    if (!Array.isArray(tensor.data)) return value;

    let index = 0;
    tensor.data = tensor.data.map(item => {
        if (index >= numericLiterals.length || !item || typeof item !== 'object') return item;
        const displaySource = numericLiterals[index++];
        return { ...(item as Record<string, unknown>), displaySource };
    });
    return value;
};

const annotateValue = (value: Value, literal: LiteralNode): Value => {
    const cloned = cloneValue(value);

    if (literal.kind === 'number' && cloned.type === 'number' && cloned.value && typeof cloned.value === 'object') {
        cloned.value = { ...(cloned.value as Record<string, unknown>), displaySource: literal.source };
        return cloned;
    }

    if (literal.kind === 'vector') {
        if (cloned.type === 'vector' && Array.isArray(cloned.value)) {
            cloned.value = cloned.value.map((child, index) => {
                const childLiteral = literal.children[index];
                return childLiteral ? annotateValue(child, childLiteral) : child;
            });
        } else if (cloned.type === 'tensor') {
            return annotateTensorLikeValue(cloned, literal);
        }
    }

    return cloned;
};

export const annotateStackDisplaySources = (stack: readonly Value[], source: string): Value[] | null => {
    const literals = parseLiteralOnlySource(source);
    if (!literals || literals.length === 0 || literals.length > stack.length) return null;

    const annotated = stack.map(cloneValue);
    const start = annotated.length - literals.length;
    literals.forEach((literal, index) => {
        annotated[start + index] = annotateValue(annotated[start + index]!, literal);
    });
    return annotated;
};
