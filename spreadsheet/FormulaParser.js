/**
 * FormulaParser.js - 数式パーサーと評価器
 *
 * 対応する関数カテゴリ:
 * - 数学: SUM, AVERAGE, MIN, MAX, COUNT, ROUND, ABS, SQRT, POWER
 * - 文字列: CONCAT, LEFT, RIGHT, MID, LEN, TRIM, UPPER, LOWER
 * - 論理: IF, AND, OR, NOT, TRUE, FALSE
 * - 検索: VLOOKUP, HLOOKUP, INDEX, MATCH
 * - 日付: TODAY, NOW, DATE, YEAR, MONTH, DAY
 * - 情報: ISNUMBER, ISTEXT, ISBLANK, ISERROR
 */

import { CellReference } from './CellReference.js';

/**
 * @typedef {Object} ASTNode
 * @property {string} type - 'number'|'string'|'boolean'|'reference'|'range'|'function'|'operator'|'error'
 * @property {*} value
 * @property {ASTNode[]} children
 */

/**
 * @typedef {Object} EvalContext
 * @property {string} sheetId - 現在のシートID
 * @property {string} cellRef - 現在のセル参照
 * @property {import('./SpreadsheetEngine.js').SpreadsheetEngine} engine - エンジン参照
 */

export class FormulaParser {
    constructor(engine) {
        /** @type {import('./SpreadsheetEngine.js').SpreadsheetEngine} */
        this.engine = engine;
        /** @type {Map<string, Function>} */
        this.functions = new Map();
        this.registerBuiltInFunctions();
    }

    // === トークナイザ ===

    /**
     * 数式をトークン配列に分割
     * @param {string} formula
     * @returns {Array<{type: string, value: string}>}
     */
    tokenize(formula) {
        const tokens = [];
        let pos = 0;

        while (pos < formula.length) {
            const char = formula[pos];

            // 空白スキップ
            if (/\s/.test(char)) {
                pos++;
                continue;
            }

            // 数値
            if (/\d/.test(char) || (char === '.' && /\d/.test(formula[pos + 1]))) {
                let num = '';
                while (pos < formula.length && /[\d.eE+-]/.test(formula[pos])) {
                    num += formula[pos++];
                }
                tokens.push({ type: 'NUMBER', value: num });
                continue;
            }

            // 文字列（ダブルクォート）
            if (char === '"') {
                let str = '';
                pos++; // 開始クォートをスキップ
                while (pos < formula.length && formula[pos] !== '"') {
                    if (formula[pos] === '\\' && pos + 1 < formula.length) {
                        pos++;
                    }
                    str += formula[pos++];
                }
                pos++; // 終了クォートをスキップ
                tokens.push({ type: 'STRING', value: str });
                continue;
            }

            // セル参照または関数名または名前付き範囲
            if (/[A-Za-z_$]/.test(char)) {
                let ident = '';
                while (pos < formula.length && /[A-Za-z0-9_$!:]/.test(formula[pos])) {
                    ident += formula[pos++];
                }

                // 範囲参照かチェック (A1:B2形式)
                if (ident.includes(':')) {
                    tokens.push({ type: 'RANGE', value: ident });
                }
                // シート参照かチェック (Sheet1!A1形式)
                else if (ident.includes('!')) {
                    tokens.push({ type: 'REFERENCE', value: ident });
                }
                // TRUE/FALSE
                else if (ident.toUpperCase() === 'TRUE') {
                    tokens.push({ type: 'BOOLEAN', value: 'TRUE' });
                }
                else if (ident.toUpperCase() === 'FALSE') {
                    tokens.push({ type: 'BOOLEAN', value: 'FALSE' });
                }
                // 関数名（次が開きかっこ）
                else if (formula[pos] === '(') {
                    tokens.push({ type: 'FUNCTION', value: ident.toUpperCase() });
                }
                // セル参照
                else if (/^[$]?[A-Z]+[$]?\d+$/i.test(ident)) {
                    tokens.push({ type: 'REFERENCE', value: ident });
                }
                // 名前付き範囲または未知の識別子
                else {
                    tokens.push({ type: 'IDENTIFIER', value: ident });
                }
                continue;
            }

            // 演算子
            if ('+-*/^%'.includes(char)) {
                tokens.push({ type: 'OPERATOR', value: char });
                pos++;
                continue;
            }

            // 比較演算子
            if (char === '<' || char === '>' || char === '=' || char === '!') {
                let op = char;
                pos++;
                if (pos < formula.length && (formula[pos] === '=' || formula[pos] === '>')) {
                    op += formula[pos++];
                }
                tokens.push({ type: 'COMPARISON', value: op });
                continue;
            }

            // かっこ
            if (char === '(') {
                tokens.push({ type: 'LPAREN', value: '(' });
                pos++;
                continue;
            }
            if (char === ')') {
                tokens.push({ type: 'RPAREN', value: ')' });
                pos++;
                continue;
            }

            // カンマ（引数区切り）
            if (char === ',') {
                tokens.push({ type: 'COMMA', value: ',' });
                pos++;
                continue;
            }

            // コロン（範囲）
            if (char === ':') {
                tokens.push({ type: 'COLON', value: ':' });
                pos++;
                continue;
            }

            // セミコロン（配列リテラル区切り）
            if (char === ';') {
                tokens.push({ type: 'SEMICOLON', value: ';' });
                pos++;
                continue;
            }

            // 不明な文字
            throw new Error(`Unexpected character: ${char}`);
        }

        tokens.push({ type: 'EOF', value: '' });
        return tokens;
    }

    // === パーサー ===

    /**
     * 数式をパースしてASTを生成
     * @param {string} formula - 数式文字列（=を除く）
     * @returns {ASTNode}
     */
    parse(formula) {
        this.tokens = this.tokenize(formula);
        this.pos = 0;
        return this.parseExpression();
    }

    /**
     * 現在のトークン取得
     * @returns {{type: string, value: string}}
     */
    currentToken() {
        return this.tokens[this.pos] || { type: 'EOF', value: '' };
    }

    /**
     * トークンを消費して次へ
     * @param {string} [expectedType]
     * @returns {{type: string, value: string}}
     */
    consume(expectedType) {
        const token = this.currentToken();
        if (expectedType && token.type !== expectedType) {
            throw new Error(`Expected ${expectedType}, got ${token.type}`);
        }
        this.pos++;
        return token;
    }

    /**
     * 式をパース
     * @returns {ASTNode}
     */
    parseExpression() {
        return this.parseComparison();
    }

    /**
     * 比較式をパース
     * @returns {ASTNode}
     */
    parseComparison() {
        let left = this.parseAddSub();

        while (this.currentToken().type === 'COMPARISON') {
            const op = this.consume().value;
            const right = this.parseAddSub();
            left = {
                type: 'operator',
                value: op,
                children: [left, right]
            };
        }

        return left;
    }

    /**
     * 加減算をパース
     * @returns {ASTNode}
     */
    parseAddSub() {
        let left = this.parseMulDiv();

        while (this.currentToken().type === 'OPERATOR' &&
               (this.currentToken().value === '+' || this.currentToken().value === '-')) {
            const op = this.consume().value;
            const right = this.parseMulDiv();
            left = {
                type: 'operator',
                value: op,
                children: [left, right]
            };
        }

        return left;
    }

    /**
     * 乗除算をパース
     * @returns {ASTNode}
     */
    parseMulDiv() {
        let left = this.parsePower();

        while (this.currentToken().type === 'OPERATOR' &&
               (this.currentToken().value === '*' || this.currentToken().value === '/' ||
                this.currentToken().value === '%')) {
            const op = this.consume().value;
            const right = this.parsePower();
            left = {
                type: 'operator',
                value: op,
                children: [left, right]
            };
        }

        return left;
    }

    /**
     * べき乗をパース
     * @returns {ASTNode}
     */
    parsePower() {
        let left = this.parseUnary();

        while (this.currentToken().type === 'OPERATOR' && this.currentToken().value === '^') {
            this.consume();
            const right = this.parseUnary();
            left = {
                type: 'operator',
                value: '^',
                children: [left, right]
            };
        }

        return left;
    }

    /**
     * 単項演算子をパース
     * @returns {ASTNode}
     */
    parseUnary() {
        if (this.currentToken().type === 'OPERATOR' &&
            (this.currentToken().value === '-' || this.currentToken().value === '+')) {
            const op = this.consume().value;
            const operand = this.parseUnary();
            return {
                type: 'operator',
                value: 'unary' + op,
                children: [operand]
            };
        }

        return this.parsePrimary();
    }

    /**
     * 基本要素をパース
     * @returns {ASTNode}
     */
    parsePrimary() {
        const token = this.currentToken();

        // 数値
        if (token.type === 'NUMBER') {
            this.consume();
            return { type: 'number', value: parseFloat(token.value), children: [] };
        }

        // 文字列
        if (token.type === 'STRING') {
            this.consume();
            return { type: 'string', value: token.value, children: [] };
        }

        // 真偽値
        if (token.type === 'BOOLEAN') {
            this.consume();
            return { type: 'boolean', value: token.value === 'TRUE', children: [] };
        }

        // 関数呼び出し
        if (token.type === 'FUNCTION') {
            return this.parseFunction();
        }

        // セル参照
        if (token.type === 'REFERENCE') {
            this.consume();
            return { type: 'reference', value: token.value, children: [] };
        }

        // 範囲参照
        if (token.type === 'RANGE') {
            this.consume();
            return { type: 'range', value: token.value, children: [] };
        }

        // 識別子（名前付き範囲など）
        if (token.type === 'IDENTIFIER') {
            this.consume();
            // 次がコロンなら範囲
            if (this.currentToken().type === 'COLON') {
                this.consume();
                const endToken = this.consume('REFERENCE');
                return { type: 'range', value: token.value + ':' + endToken.value, children: [] };
            }
            return { type: 'identifier', value: token.value, children: [] };
        }

        // かっこ
        if (token.type === 'LPAREN') {
            this.consume();
            const expr = this.parseExpression();
            this.consume('RPAREN');
            return expr;
        }

        throw new Error(`Unexpected token: ${token.type} (${token.value})`);
    }

    /**
     * 関数呼び出しをパース
     * @returns {ASTNode}
     */
    parseFunction() {
        const funcName = this.consume('FUNCTION').value;
        this.consume('LPAREN');

        const args = [];
        while (this.currentToken().type !== 'RPAREN') {
            args.push(this.parseExpression());
            if (this.currentToken().type === 'COMMA') {
                this.consume();
            }
        }
        this.consume('RPAREN');

        return {
            type: 'function',
            value: funcName,
            children: args
        };
    }

    // === 評価 ===

    /**
     * ASTを評価して結果を返す
     * @param {ASTNode} ast
     * @param {EvalContext} context
     * @returns {*}
     */
    evaluate(ast, context) {
        if (!ast) return null;

        switch (ast.type) {
            case 'number':
            case 'string':
            case 'boolean':
                return ast.value;

            case 'reference':
                return this.evaluateReference(ast.value, context);

            case 'range':
                return this.evaluateRange(ast.value, context);

            case 'function':
                return this.evaluateFunction(ast, context);

            case 'operator':
                return this.evaluateOperator(ast, context);

            case 'identifier':
                // 名前付き範囲の解決
                return this.evaluateNamedRange(ast.value, context);

            case 'error':
                throw new Error(ast.value);

            default:
                throw new Error(`Unknown AST node type: ${ast.type}`);
        }
    }

    /**
     * セル参照を評価
     * @param {string} ref
     * @param {EvalContext} context
     * @returns {*}
     */
    evaluateReference(ref, context) {
        const parsed = CellReference.parse(ref);
        if (!parsed) {
            throw new Error(`Invalid cell reference: ${ref}`);
        }

        let sheetId = context.sheetId;
        if (parsed.sheetName) {
            const sheet = this.engine.getSheetByName(parsed.sheetName);
            if (!sheet) {
                throw new Error(`Sheet not found: ${parsed.sheetName}`);
            }
            sheetId = sheet.id;
        }

        const cellRef = parsed.col + parsed.row;
        const value = this.engine.getCellValue(sheetId, cellRef);

        // エラー値の伝播
        if (typeof value === 'string' && value.startsWith('#')) {
            throw new Error(value);
        }

        return value;
    }

    /**
     * 範囲参照を評価（配列として返す）
     * @param {string} rangeRef
     * @param {EvalContext} context
     * @returns {Array<Array<*>>}
     */
    evaluateRange(rangeRef, context) {
        const range = CellReference.parseRange(rangeRef);
        if (!range) {
            throw new Error(`Invalid range: ${rangeRef}`);
        }

        let sheetId = context.sheetId;
        if (range.sheetName) {
            const sheet = this.engine.getSheetByName(range.sheetName);
            if (!sheet) {
                throw new Error(`Sheet not found: ${range.sheetName}`);
            }
            sheetId = sheet.id;
        }

        const startCol = CellReference.colToIndex(range.start.col);
        const endCol = CellReference.colToIndex(range.end.col);
        const startRow = range.start.row;
        const endRow = range.end.row;

        const minCol = Math.min(startCol, endCol);
        const maxCol = Math.max(startCol, endCol);
        const minRow = Math.min(startRow, endRow);
        const maxRow = Math.max(startRow, endRow);

        const result = [];
        for (let row = minRow; row <= maxRow; row++) {
            const rowValues = [];
            for (let col = minCol; col <= maxCol; col++) {
                const cellRef = CellReference.indexToCol(col) + row;
                const value = this.engine.getCellValue(sheetId, cellRef);
                rowValues.push(value);
            }
            result.push(rowValues);
        }

        return result;
    }

    /**
     * 名前付き範囲を評価
     * @param {string} name
     * @param {EvalContext} context
     * @returns {*}
     */
    evaluateNamedRange(name, context) {
        const workbook = this.engine.workbook;
        if (!workbook?.namedRanges?.[name]) {
            throw new Error(`Unknown name: ${name}`);
        }

        const namedRange = workbook.namedRanges[name];
        return this.evaluateRange(namedRange, context);
    }

    /**
     * 関数を評価
     * @param {ASTNode} ast
     * @param {EvalContext} context
     * @returns {*}
     */
    evaluateFunction(ast, context) {
        const funcName = ast.value;
        const func = this.functions.get(funcName);

        if (!func) {
            throw new Error(`Unknown function: ${funcName}`);
        }

        // 引数を評価（遅延評価のため関数に渡す）
        const evalArg = (index) => {
            if (index >= ast.children.length) return undefined;
            return this.evaluate(ast.children[index], context);
        };

        // 引数のAST配列も渡す（範囲参照の処理のため）
        return func.call(this, ast.children, context, evalArg);
    }

    /**
     * 演算子を評価
     * @param {ASTNode} ast
     * @param {EvalContext} context
     * @returns {*}
     */
    evaluateOperator(ast, context) {
        const op = ast.value;

        // 単項演算子
        if (op === 'unary-') {
            const operand = this.evaluate(ast.children[0], context);
            return -this.toNumber(operand);
        }
        if (op === 'unary+') {
            const operand = this.evaluate(ast.children[0], context);
            return this.toNumber(operand);
        }

        // 二項演算子
        const left = this.evaluate(ast.children[0], context);
        const right = this.evaluate(ast.children[1], context);

        switch (op) {
            case '+':
                return this.toNumber(left) + this.toNumber(right);
            case '-':
                return this.toNumber(left) - this.toNumber(right);
            case '*':
                return this.toNumber(left) * this.toNumber(right);
            case '/':
                const divisor = this.toNumber(right);
                if (divisor === 0) throw new Error('#DIV/0!');
                return this.toNumber(left) / divisor;
            case '%':
                return this.toNumber(left) % this.toNumber(right);
            case '^':
                return Math.pow(this.toNumber(left), this.toNumber(right));
            case '=':
                return left === right;
            case '<>':
            case '!=':
                return left !== right;
            case '<':
                return this.toNumber(left) < this.toNumber(right);
            case '>':
                return this.toNumber(left) > this.toNumber(right);
            case '<=':
                return this.toNumber(left) <= this.toNumber(right);
            case '>=':
                return this.toNumber(left) >= this.toNumber(right);
            default:
                throw new Error(`Unknown operator: ${op}`);
        }
    }

    // === ヘルパー ===

    /**
     * 値を数値に変換
     * @param {*} value
     * @returns {number}
     */
    toNumber(value) {
        if (value === null || value === undefined || value === '') return 0;
        if (typeof value === 'number') return value;
        if (typeof value === 'boolean') return value ? 1 : 0;
        const num = parseFloat(value);
        if (isNaN(num)) throw new Error('#VALUE!');
        return num;
    }

    /**
     * 値を文字列に変換
     * @param {*} value
     * @returns {string}
     */
    toString(value) {
        if (value === null || value === undefined) return '';
        return String(value);
    }

    /**
     * 値を真偽値に変換
     * @param {*} value
     * @returns {boolean}
     */
    toBoolean(value) {
        if (typeof value === 'boolean') return value;
        if (typeof value === 'number') return value !== 0;
        if (typeof value === 'string') {
            if (value.toUpperCase() === 'TRUE') return true;
            if (value.toUpperCase() === 'FALSE') return false;
            throw new Error('#VALUE!');
        }
        return Boolean(value);
    }

    /**
     * 範囲の値をフラット配列に変換
     * @param {*} value
     * @returns {Array}
     */
    flattenRange(value) {
        if (!Array.isArray(value)) return [value];
        const result = [];
        const flatten = (arr) => {
            for (const item of arr) {
                if (Array.isArray(item)) {
                    flatten(item);
                } else {
                    result.push(item);
                }
            }
        };
        flatten(value);
        return result;
    }

    // === 参照抽出 ===

    /**
     * 数式から参照セルを抽出
     * @param {string} formula
     * @returns {string[]}
     */
    extractReferences(formula) {
        const references = [];
        const tokens = this.tokenize(formula);

        for (const token of tokens) {
            if (token.type === 'REFERENCE') {
                references.push(token.value);
            } else if (token.type === 'RANGE') {
                // 範囲内のすべてのセルを展開
                const range = CellReference.parseRange(token.value);
                if (range) {
                    for (const ref of CellReference.iterateRange(range)) {
                        references.push(ref);
                    }
                }
            }
        }

        return references;
    }

    // === 参照更新 ===

    /**
     * セル参照を更新（行・列挿入/削除時）
     * @param {string} formula
     * @param {{type: string, index: number, count: number}} updateInfo
     * @returns {string}
     */
    updateReferences(formula, updateInfo) {
        const tokens = this.tokenize(formula);
        let result = '';

        for (let i = 0; i < tokens.length; i++) {
            const token = tokens[i];

            if (token.type === 'REFERENCE') {
                const parsed = CellReference.parse(token.value);
                if (parsed) {
                    const updated = CellReference.updateForInsertDelete(
                        parsed,
                        updateInfo.type,
                        updateInfo.index,
                        updateInfo.count
                    );
                    if (updated) {
                        result += CellReference.toString(updated, !!parsed.sheetName);
                    } else {
                        result += '#REF!';
                    }
                } else {
                    result += token.value;
                }
            } else if (token.type === 'RANGE') {
                const range = CellReference.parseRange(token.value);
                if (range) {
                    const startUpdated = CellReference.updateForInsertDelete(
                        range.start,
                        updateInfo.type,
                        updateInfo.index,
                        updateInfo.count
                    );
                    const endUpdated = CellReference.updateForInsertDelete(
                        range.end,
                        updateInfo.type,
                        updateInfo.index,
                        updateInfo.count
                    );

                    if (startUpdated && endUpdated) {
                        result += CellReference.toString(startUpdated, false);
                        result += ':';
                        result += CellReference.toString(endUpdated, false);
                    } else {
                        result += '#REF!';
                    }
                } else {
                    result += token.value;
                }
            } else if (token.type !== 'EOF') {
                result += token.value;
            }
        }

        return result;
    }

    // === 組み込み関数登録 ===

    /**
     * 組み込み関数を登録
     */
    registerBuiltInFunctions() {
        // === 数学関数 ===

        this.registerFunction('SUM', (args, ctx, evalArg) => {
            let sum = 0;
            for (let i = 0; i < args.length; i++) {
                const value = evalArg(i);
                const flat = this.flattenRange(value);
                for (const v of flat) {
                    if (typeof v === 'number') sum += v;
                    else if (v !== null && v !== undefined && v !== '') {
                        const num = parseFloat(v);
                        if (!isNaN(num)) sum += num;
                    }
                }
            }
            return sum;
        });

        this.registerFunction('AVERAGE', (args, ctx, evalArg) => {
            let sum = 0;
            let count = 0;
            for (let i = 0; i < args.length; i++) {
                const value = evalArg(i);
                const flat = this.flattenRange(value);
                for (const v of flat) {
                    if (typeof v === 'number') {
                        sum += v;
                        count++;
                    } else if (v !== null && v !== undefined && v !== '') {
                        const num = parseFloat(v);
                        if (!isNaN(num)) {
                            sum += num;
                            count++;
                        }
                    }
                }
            }
            if (count === 0) throw new Error('#DIV/0!');
            return sum / count;
        });

        this.registerFunction('MIN', (args, ctx, evalArg) => {
            let min = Infinity;
            for (let i = 0; i < args.length; i++) {
                const value = evalArg(i);
                const flat = this.flattenRange(value);
                for (const v of flat) {
                    if (typeof v === 'number') {
                        min = Math.min(min, v);
                    } else if (v !== null && v !== undefined && v !== '') {
                        const num = parseFloat(v);
                        if (!isNaN(num)) min = Math.min(min, num);
                    }
                }
            }
            return min === Infinity ? 0 : min;
        });

        this.registerFunction('MAX', (args, ctx, evalArg) => {
            let max = -Infinity;
            for (let i = 0; i < args.length; i++) {
                const value = evalArg(i);
                const flat = this.flattenRange(value);
                for (const v of flat) {
                    if (typeof v === 'number') {
                        max = Math.max(max, v);
                    } else if (v !== null && v !== undefined && v !== '') {
                        const num = parseFloat(v);
                        if (!isNaN(num)) max = Math.max(max, num);
                    }
                }
            }
            return max === -Infinity ? 0 : max;
        });

        this.registerFunction('COUNT', (args, ctx, evalArg) => {
            let count = 0;
            for (let i = 0; i < args.length; i++) {
                const value = evalArg(i);
                const flat = this.flattenRange(value);
                for (const v of flat) {
                    if (typeof v === 'number') count++;
                    else if (v !== null && v !== undefined && v !== '' && !isNaN(parseFloat(v))) {
                        count++;
                    }
                }
            }
            return count;
        });

        this.registerFunction('COUNTA', (args, ctx, evalArg) => {
            let count = 0;
            for (let i = 0; i < args.length; i++) {
                const value = evalArg(i);
                const flat = this.flattenRange(value);
                for (const v of flat) {
                    if (v !== null && v !== undefined && v !== '') count++;
                }
            }
            return count;
        });

        this.registerFunction('ROUND', (args, ctx, evalArg) => {
            const num = this.toNumber(evalArg(0));
            const digits = args.length > 1 ? this.toNumber(evalArg(1)) : 0;
            const factor = Math.pow(10, digits);
            return Math.round(num * factor) / factor;
        });

        this.registerFunction('ROUNDDOWN', (args, ctx, evalArg) => {
            const num = this.toNumber(evalArg(0));
            const digits = args.length > 1 ? this.toNumber(evalArg(1)) : 0;
            const factor = Math.pow(10, digits);
            return Math.floor(num * factor) / factor;
        });

        this.registerFunction('ROUNDUP', (args, ctx, evalArg) => {
            const num = this.toNumber(evalArg(0));
            const digits = args.length > 1 ? this.toNumber(evalArg(1)) : 0;
            const factor = Math.pow(10, digits);
            return Math.ceil(num * factor) / factor;
        });

        this.registerFunction('ABS', (args, ctx, evalArg) => {
            return Math.abs(this.toNumber(evalArg(0)));
        });

        this.registerFunction('SQRT', (args, ctx, evalArg) => {
            const num = this.toNumber(evalArg(0));
            if (num < 0) throw new Error('#NUM!');
            return Math.sqrt(num);
        });

        this.registerFunction('POWER', (args, ctx, evalArg) => {
            const base = this.toNumber(evalArg(0));
            const exp = this.toNumber(evalArg(1));
            return Math.pow(base, exp);
        });

        this.registerFunction('MOD', (args, ctx, evalArg) => {
            const num = this.toNumber(evalArg(0));
            const divisor = this.toNumber(evalArg(1));
            if (divisor === 0) throw new Error('#DIV/0!');
            return num % divisor;
        });

        this.registerFunction('INT', (args, ctx, evalArg) => {
            return Math.floor(this.toNumber(evalArg(0)));
        });

        this.registerFunction('PI', () => Math.PI);

        this.registerFunction('SIN', (args, ctx, evalArg) => Math.sin(this.toNumber(evalArg(0))));
        this.registerFunction('COS', (args, ctx, evalArg) => Math.cos(this.toNumber(evalArg(0))));
        this.registerFunction('TAN', (args, ctx, evalArg) => Math.tan(this.toNumber(evalArg(0))));
        this.registerFunction('LOG', (args, ctx, evalArg) => {
            const num = this.toNumber(evalArg(0));
            const base = args.length > 1 ? this.toNumber(evalArg(1)) : 10;
            if (num <= 0) throw new Error('#NUM!');
            return Math.log(num) / Math.log(base);
        });
        this.registerFunction('LN', (args, ctx, evalArg) => {
            const num = this.toNumber(evalArg(0));
            if (num <= 0) throw new Error('#NUM!');
            return Math.log(num);
        });
        this.registerFunction('EXP', (args, ctx, evalArg) => Math.exp(this.toNumber(evalArg(0))));

        // === 文字列関数 ===

        this.registerFunction('CONCAT', (args, ctx, evalArg) => {
            let result = '';
            for (let i = 0; i < args.length; i++) {
                result += this.toString(evalArg(i));
            }
            return result;
        });

        this.registerFunction('CONCATENATE', (args, ctx, evalArg) => {
            let result = '';
            for (let i = 0; i < args.length; i++) {
                result += this.toString(evalArg(i));
            }
            return result;
        });

        this.registerFunction('LEFT', (args, ctx, evalArg) => {
            const str = this.toString(evalArg(0));
            const count = args.length > 1 ? this.toNumber(evalArg(1)) : 1;
            return str.substring(0, count);
        });

        this.registerFunction('RIGHT', (args, ctx, evalArg) => {
            const str = this.toString(evalArg(0));
            const count = args.length > 1 ? this.toNumber(evalArg(1)) : 1;
            return str.substring(str.length - count);
        });

        this.registerFunction('MID', (args, ctx, evalArg) => {
            const str = this.toString(evalArg(0));
            const start = this.toNumber(evalArg(1)) - 1; // 1-based
            const count = this.toNumber(evalArg(2));
            return str.substring(start, start + count);
        });

        this.registerFunction('LEN', (args, ctx, evalArg) => {
            return this.toString(evalArg(0)).length;
        });

        this.registerFunction('TRIM', (args, ctx, evalArg) => {
            return this.toString(evalArg(0)).trim();
        });

        this.registerFunction('UPPER', (args, ctx, evalArg) => {
            return this.toString(evalArg(0)).toUpperCase();
        });

        this.registerFunction('LOWER', (args, ctx, evalArg) => {
            return this.toString(evalArg(0)).toLowerCase();
        });

        this.registerFunction('PROPER', (args, ctx, evalArg) => {
            return this.toString(evalArg(0)).replace(/\w\S*/g, txt =>
                txt.charAt(0).toUpperCase() + txt.substr(1).toLowerCase()
            );
        });

        this.registerFunction('SUBSTITUTE', (args, ctx, evalArg) => {
            const str = this.toString(evalArg(0));
            const oldText = this.toString(evalArg(1));
            const newText = this.toString(evalArg(2));
            const instance = args.length > 3 ? this.toNumber(evalArg(3)) : 0;

            if (instance === 0) {
                return str.split(oldText).join(newText);
            }

            let count = 0;
            return str.replace(new RegExp(oldText.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'g'), (match) => {
                count++;
                return count === instance ? newText : match;
            });
        });

        this.registerFunction('REPT', (args, ctx, evalArg) => {
            const str = this.toString(evalArg(0));
            const times = this.toNumber(evalArg(1));
            return str.repeat(Math.max(0, Math.floor(times)));
        });

        this.registerFunction('FIND', (args, ctx, evalArg) => {
            const findText = this.toString(evalArg(0));
            const withinText = this.toString(evalArg(1));
            const startNum = args.length > 2 ? this.toNumber(evalArg(2)) : 1;
            const pos = withinText.indexOf(findText, startNum - 1);
            if (pos === -1) throw new Error('#VALUE!');
            return pos + 1;
        });

        this.registerFunction('SEARCH', (args, ctx, evalArg) => {
            const findText = this.toString(evalArg(0)).toLowerCase();
            const withinText = this.toString(evalArg(1)).toLowerCase();
            const startNum = args.length > 2 ? this.toNumber(evalArg(2)) : 1;
            const pos = withinText.indexOf(findText, startNum - 1);
            if (pos === -1) throw new Error('#VALUE!');
            return pos + 1;
        });

        this.registerFunction('REPLACE', (args, ctx, evalArg) => {
            const str = this.toString(evalArg(0));
            const start = this.toNumber(evalArg(1)) - 1;
            const numChars = this.toNumber(evalArg(2));
            const newText = this.toString(evalArg(3));
            return str.substring(0, start) + newText + str.substring(start + numChars);
        });

        this.registerFunction('TEXT', (args, ctx, evalArg) => {
            const value = evalArg(0);
            const format = this.toString(evalArg(1));
            // 簡易実装
            if (typeof value === 'number') {
                if (format.includes('%')) {
                    return (value * 100).toFixed(format.split('.')[1]?.replace(/[^0]/g, '').length || 0) + '%';
                }
                const decimals = (format.split('.')[1] || '').replace(/[^0]/g, '').length;
                return value.toFixed(decimals);
            }
            return this.toString(value);
        });

        this.registerFunction('VALUE', (args, ctx, evalArg) => {
            const str = this.toString(evalArg(0));
            const num = parseFloat(str.replace(/[,$]/g, ''));
            if (isNaN(num)) throw new Error('#VALUE!');
            return num;
        });

        // === 論理関数 ===

        this.registerFunction('IF', (args, ctx, evalArg) => {
            const condition = this.toBoolean(evalArg(0));
            if (condition) {
                return args.length > 1 ? evalArg(1) : true;
            } else {
                return args.length > 2 ? evalArg(2) : false;
            }
        });

        this.registerFunction('AND', (args, ctx, evalArg) => {
            for (let i = 0; i < args.length; i++) {
                if (!this.toBoolean(evalArg(i))) return false;
            }
            return true;
        });

        this.registerFunction('OR', (args, ctx, evalArg) => {
            for (let i = 0; i < args.length; i++) {
                if (this.toBoolean(evalArg(i))) return true;
            }
            return false;
        });

        this.registerFunction('NOT', (args, ctx, evalArg) => {
            return !this.toBoolean(evalArg(0));
        });

        this.registerFunction('TRUE', () => true);
        this.registerFunction('FALSE', () => false);

        this.registerFunction('IFERROR', (args, ctx, evalArg) => {
            try {
                return evalArg(0);
            } catch (e) {
                return args.length > 1 ? evalArg(1) : '';
            }
        });

        this.registerFunction('IFNA', (args, ctx, evalArg) => {
            try {
                const result = evalArg(0);
                if (result === '#N/A') {
                    return args.length > 1 ? evalArg(1) : '';
                }
                return result;
            } catch (e) {
                if (e.message === '#N/A') {
                    return args.length > 1 ? evalArg(1) : '';
                }
                throw e;
            }
        });

        // === 検索関数 ===

        this.registerFunction('VLOOKUP', (args, ctx, evalArg) => {
            const lookupValue = evalArg(0);
            const tableArray = evalArg(1);
            const colIndex = this.toNumber(evalArg(2));
            const rangeLookup = args.length > 3 ? this.toBoolean(evalArg(3)) : true;

            if (!Array.isArray(tableArray)) throw new Error('#VALUE!');
            if (colIndex < 1 || colIndex > (tableArray[0]?.length || 0)) throw new Error('#REF!');

            for (let i = 0; i < tableArray.length; i++) {
                const rowValue = tableArray[i][0];
                if (rangeLookup) {
                    // 近似一致
                    if (rowValue === lookupValue) {
                        return tableArray[i][colIndex - 1];
                    }
                } else {
                    // 完全一致
                    if (rowValue === lookupValue) {
                        return tableArray[i][colIndex - 1];
                    }
                }
            }
            throw new Error('#N/A');
        });

        this.registerFunction('HLOOKUP', (args, ctx, evalArg) => {
            const lookupValue = evalArg(0);
            const tableArray = evalArg(1);
            const rowIndex = this.toNumber(evalArg(2));
            const rangeLookup = args.length > 3 ? this.toBoolean(evalArg(3)) : true;

            if (!Array.isArray(tableArray) || tableArray.length === 0) throw new Error('#VALUE!');
            if (rowIndex < 1 || rowIndex > tableArray.length) throw new Error('#REF!');

            const firstRow = tableArray[0];
            for (let i = 0; i < firstRow.length; i++) {
                if (rangeLookup) {
                    if (firstRow[i] === lookupValue) {
                        return tableArray[rowIndex - 1][i];
                    }
                } else {
                    if (firstRow[i] === lookupValue) {
                        return tableArray[rowIndex - 1][i];
                    }
                }
            }
            throw new Error('#N/A');
        });

        this.registerFunction('INDEX', (args, ctx, evalArg) => {
            const array = evalArg(0);
            const rowNum = this.toNumber(evalArg(1));
            const colNum = args.length > 2 ? this.toNumber(evalArg(2)) : 1;

            if (!Array.isArray(array)) {
                if (rowNum === 1 && colNum === 1) return array;
                throw new Error('#REF!');
            }

            if (rowNum < 1 || rowNum > array.length) throw new Error('#REF!');
            const row = array[rowNum - 1];

            if (!Array.isArray(row)) {
                if (colNum === 1) return row;
                throw new Error('#REF!');
            }

            if (colNum < 1 || colNum > row.length) throw new Error('#REF!');
            return row[colNum - 1];
        });

        this.registerFunction('MATCH', (args, ctx, evalArg) => {
            const lookupValue = evalArg(0);
            const lookupArray = this.flattenRange(evalArg(1));
            const matchType = args.length > 2 ? this.toNumber(evalArg(2)) : 1;

            for (let i = 0; i < lookupArray.length; i++) {
                if (matchType === 0) {
                    // 完全一致
                    if (lookupArray[i] === lookupValue) return i + 1;
                } else if (matchType === 1) {
                    // 以下で最大
                    if (lookupArray[i] === lookupValue) return i + 1;
                    if (lookupArray[i] > lookupValue && i > 0) return i;
                } else if (matchType === -1) {
                    // 以上で最小
                    if (lookupArray[i] === lookupValue) return i + 1;
                    if (lookupArray[i] < lookupValue && i > 0) return i;
                }
            }
            throw new Error('#N/A');
        });

        // === 日付関数 ===

        this.registerFunction('TODAY', () => {
            const now = new Date();
            return new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
        });

        this.registerFunction('NOW', () => {
            return Date.now();
        });

        this.registerFunction('DATE', (args, ctx, evalArg) => {
            const year = this.toNumber(evalArg(0));
            const month = this.toNumber(evalArg(1)) - 1;
            const day = this.toNumber(evalArg(2));
            return new Date(year, month, day).getTime();
        });

        this.registerFunction('YEAR', (args, ctx, evalArg) => {
            const date = new Date(this.toNumber(evalArg(0)));
            return date.getFullYear();
        });

        this.registerFunction('MONTH', (args, ctx, evalArg) => {
            const date = new Date(this.toNumber(evalArg(0)));
            return date.getMonth() + 1;
        });

        this.registerFunction('DAY', (args, ctx, evalArg) => {
            const date = new Date(this.toNumber(evalArg(0)));
            return date.getDate();
        });

        this.registerFunction('HOUR', (args, ctx, evalArg) => {
            const date = new Date(this.toNumber(evalArg(0)));
            return date.getHours();
        });

        this.registerFunction('MINUTE', (args, ctx, evalArg) => {
            const date = new Date(this.toNumber(evalArg(0)));
            return date.getMinutes();
        });

        this.registerFunction('SECOND', (args, ctx, evalArg) => {
            const date = new Date(this.toNumber(evalArg(0)));
            return date.getSeconds();
        });

        // === 情報関数 ===

        this.registerFunction('ISNUMBER', (args, ctx, evalArg) => {
            const value = evalArg(0);
            return typeof value === 'number' && !isNaN(value);
        });

        this.registerFunction('ISTEXT', (args, ctx, evalArg) => {
            const value = evalArg(0);
            return typeof value === 'string';
        });

        this.registerFunction('ISBLANK', (args, ctx, evalArg) => {
            const value = evalArg(0);
            return value === null || value === undefined || value === '';
        });

        this.registerFunction('ISERROR', (args, ctx, evalArg) => {
            try {
                const value = evalArg(0);
                return typeof value === 'string' && value.startsWith('#');
            } catch (e) {
                return true;
            }
        });

        this.registerFunction('ISNA', (args, ctx, evalArg) => {
            try {
                const value = evalArg(0);
                return value === '#N/A';
            } catch (e) {
                return e.message === '#N/A';
            }
        });

        this.registerFunction('ISLOGICAL', (args, ctx, evalArg) => {
            return typeof evalArg(0) === 'boolean';
        });

        this.registerFunction('TYPE', (args, ctx, evalArg) => {
            const value = evalArg(0);
            if (typeof value === 'number') return 1;
            if (typeof value === 'string') return 2;
            if (typeof value === 'boolean') return 4;
            if (typeof value === 'string' && value.startsWith('#')) return 16;
            if (Array.isArray(value)) return 64;
            return 0;
        });

        // === その他 ===

        this.registerFunction('RAND', () => Math.random());

        this.registerFunction('RANDBETWEEN', (args, ctx, evalArg) => {
            const bottom = Math.ceil(this.toNumber(evalArg(0)));
            const top = Math.floor(this.toNumber(evalArg(1)));
            return Math.floor(Math.random() * (top - bottom + 1)) + bottom;
        });

        this.registerFunction('CHOOSE', (args, ctx, evalArg) => {
            const index = this.toNumber(evalArg(0));
            if (index < 1 || index >= args.length) throw new Error('#VALUE!');
            return evalArg(index);
        });

        this.registerFunction('COLUMN', (args, ctx, evalArg) => {
            if (args.length === 0) {
                const parsed = CellReference.parse(ctx.cellRef);
                return CellReference.colToIndex(parsed.col) + 1;
            }
            const ref = args[0];
            if (ref.type === 'reference') {
                const parsed = CellReference.parse(ref.value);
                return CellReference.colToIndex(parsed.col) + 1;
            }
            throw new Error('#VALUE!');
        });

        this.registerFunction('ROW', (args, ctx, evalArg) => {
            if (args.length === 0) {
                const parsed = CellReference.parse(ctx.cellRef);
                return parsed.row;
            }
            const ref = args[0];
            if (ref.type === 'reference') {
                const parsed = CellReference.parse(ref.value);
                return parsed.row;
            }
            throw new Error('#VALUE!');
        });
    }

    /**
     * カスタム関数登録
     * @param {string} name
     * @param {Function} fn
     */
    registerFunction(name, fn) {
        this.functions.set(name.toUpperCase(), fn);
    }
}

export default FormulaParser;
