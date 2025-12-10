/**
 * SheetManager.js - シート管理とシート間操作
 */

import { CellReference } from './CellReference.js';

export class SheetManager {
    /**
     * @param {import('./SpreadsheetEngine.js').SpreadsheetEngine} engine
     */
    constructor(engine) {
        this.engine = engine;
    }

    /**
     * シート参照を解決
     * 'Sheet1!A1' -> { sheetId, cellRef }
     * @param {string} reference
     * @param {string} currentSheetId
     * @returns {{sheetId: string, cellRef: string}|null}
     */
    resolveReference(reference, currentSheetId) {
        const parsed = CellReference.parse(reference);
        if (!parsed) return null;

        let sheetId = currentSheetId;

        if (parsed.sheetName) {
            const sheet = this.engine.getSheetByName(parsed.sheetName);
            if (!sheet) {
                return null;
            }
            sheetId = sheet.id;
        }

        return {
            sheetId,
            cellRef: parsed.col + parsed.row
        };
    }

    /**
     * 範囲参照を解決
     * 'Sheet1!A1:B2' -> { sheetId, startCell, endCell }
     * @param {string} rangeReference
     * @param {string} currentSheetId
     * @returns {{sheetId: string, startCell: string, endCell: string}|null}
     */
    resolveRangeReference(rangeReference, currentSheetId) {
        const range = CellReference.parseRange(rangeReference);
        if (!range) return null;

        let sheetId = currentSheetId;

        if (range.sheetName) {
            const sheet = this.engine.getSheetByName(range.sheetName);
            if (!sheet) {
                return null;
            }
            sheetId = sheet.id;
        }

        return {
            sheetId,
            startCell: range.start.col + range.start.row,
            endCell: range.end.col + range.end.row
        };
    }

    /**
     * 名前付き範囲を定義
     * @param {string} name
     * @param {string} sheetId
     * @param {string} rangeRef
     */
    defineNamedRange(name, sheetId, rangeRef) {
        if (!this.engine.workbook) return;

        const sheet = this.engine._getSheet(sheetId);
        if (!sheet) return;

        // 名前付き範囲はシート名付きで保存
        const fullRef = `${sheet.name}!${rangeRef}`;
        this.engine.workbook.namedRanges[name] = fullRef;

        this.engine._dispatchEvent('namedRangeDefined', { name, sheetId, rangeRef });
    }

    /**
     * 名前付き範囲を削除
     * @param {string} name
     */
    removeNamedRange(name) {
        if (!this.engine.workbook?.namedRanges) return;

        delete this.engine.workbook.namedRanges[name];

        this.engine._dispatchEvent('namedRangeRemoved', { name });
    }

    /**
     * 名前付き範囲を解決
     * @param {string} name
     * @returns {{sheetId: string, rangeRef: string}|null}
     */
    resolveNamedRange(name) {
        if (!this.engine.workbook?.namedRanges) return null;

        const fullRef = this.engine.workbook.namedRanges[name];
        if (!fullRef) return null;

        const range = CellReference.parseRange(fullRef);
        if (!range) return null;

        let sheetId = this.engine.workbook.activeSheetId;
        if (range.sheetName) {
            const sheet = this.engine.getSheetByName(range.sheetName);
            if (sheet) {
                sheetId = sheet.id;
            }
        }

        return {
            sheetId,
            rangeRef: CellReference.rangeToString(range, false)
        };
    }

    /**
     * すべての名前付き範囲を取得
     * @returns {Object<string, string>}
     */
    getAllNamedRanges() {
        return { ...(this.engine.workbook?.namedRanges || {}) };
    }

    /**
     * シート間の依存関係を取得
     * @returns {Map<string, Set<string>>} sheetId -> 依存されているsheetIdのSet
     */
    getSheetDependencies() {
        const dependencies = new Map();

        if (!this.engine.workbook) return dependencies;

        for (const sheet of this.engine.workbook.sheets) {
            dependencies.set(sheet.id, new Set());
        }

        // 各シートの数式を解析して依存関係を構築
        for (const sheet of this.engine.workbook.sheets) {
            for (const [cellRef, cell] of sheet.cells) {
                if (cell.formula) {
                    // 数式からシート参照を抽出
                    const refs = this._extractSheetReferences(cell.formula);
                    for (const refSheetName of refs) {
                        const refSheet = this.engine.getSheetByName(refSheetName);
                        if (refSheet && refSheet.id !== sheet.id) {
                            dependencies.get(sheet.id).add(refSheet.id);
                        }
                    }
                }
            }
        }

        return dependencies;
    }

    /**
     * 数式からシート参照を抽出
     * @param {string} formula
     * @returns {string[]} シート名の配列
     */
    _extractSheetReferences(formula) {
        const sheetRefs = [];
        const regex = /(?:'([^']+)'|([A-Za-z_][A-Za-z0-9_]*))!/g;
        let match;

        while ((match = regex.exec(formula)) !== null) {
            const sheetName = match[1] || match[2];
            if (sheetName && !sheetRefs.includes(sheetName)) {
                sheetRefs.push(sheetName);
            }
        }

        return sheetRefs;
    }

    /**
     * シートを安全に削除できるかチェック
     * 他のシートから参照されている場合はfalse
     * @param {string} sheetId
     * @returns {{canDelete: boolean, referencingSheets: string[]}}
     */
    canDeleteSheet(sheetId) {
        const dependencies = this.getSheetDependencies();
        const referencingSheets = [];

        for (const [sid, deps] of dependencies) {
            if (sid !== sheetId && deps.has(sheetId)) {
                const sheet = this.engine._getSheet(sid);
                if (sheet) {
                    referencingSheets.push(sheet.name);
                }
            }
        }

        return {
            canDelete: referencingSheets.length === 0,
            referencingSheets
        };
    }

    /**
     * シート名変更時に他シートの参照を更新
     * @param {string} oldName
     * @param {string} newName
     */
    updateSheetReferences(oldName, newName) {
        if (!this.engine.workbook) return;

        const oldPattern = new RegExp(
            `(?:'${this._escapeRegex(oldName)}'|${this._escapeRegex(oldName)})!`,
            'g'
        );

        for (const sheet of this.engine.workbook.sheets) {
            for (const [cellRef, cell] of sheet.cells) {
                if (cell.formula) {
                    const newFormula = cell.formula.replace(oldPattern, (match) => {
                        // シート名にスペースや特殊文字が含まれる場合はクォート
                        if (/[^a-zA-Z0-9_]/.test(newName)) {
                            return `'${newName}'!`;
                        }
                        return `${newName}!`;
                    });

                    if (newFormula !== cell.formula) {
                        cell.formula = newFormula;
                        cell.raw = '=' + newFormula;
                    }
                }
            }
        }

        // 名前付き範囲も更新
        if (this.engine.workbook.namedRanges) {
            for (const [name, ref] of Object.entries(this.engine.workbook.namedRanges)) {
                const newRef = ref.replace(oldPattern, (match) => {
                    if (/[^a-zA-Z0-9_]/.test(newName)) {
                        return `'${newName}'!`;
                    }
                    return `${newName}!`;
                });

                if (newRef !== ref) {
                    this.engine.workbook.namedRanges[name] = newRef;
                }
            }
        }
    }

    /**
     * 正規表現用にエスケープ
     * @param {string} str
     * @returns {string}
     */
    _escapeRegex(str) {
        return str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    }

    /**
     * シートの計算順序を取得（依存関係に基づく）
     * @returns {string[]} シートIDの配列（計算順）
     */
    getCalculationOrder() {
        const dependencies = this.getSheetDependencies();
        const order = [];
        const visited = new Set();
        const visiting = new Set();

        const visit = (sheetId) => {
            if (visited.has(sheetId)) return;
            if (visiting.has(sheetId)) {
                // 循環参照検出
                console.warn('Circular sheet dependency detected');
                return;
            }

            visiting.add(sheetId);

            const deps = dependencies.get(sheetId);
            if (deps) {
                for (const depId of deps) {
                    visit(depId);
                }
            }

            visiting.delete(sheetId);
            visited.add(sheetId);
            order.push(sheetId);
        };

        for (const sheetId of dependencies.keys()) {
            visit(sheetId);
        }

        return order;
    }

    /**
     * シート間でセルをコピー
     * @param {string} sourceSheetId
     * @param {string} sourceRange
     * @param {string} targetSheetId
     * @param {string} targetCell
     * @param {Object} options
     */
    copyBetweenSheets(sourceSheetId, sourceRange, targetSheetId, targetCell, options = {}) {
        const clipboardData = this.engine.copy(sourceSheetId, sourceRange);
        if (clipboardData) {
            this.engine.paste(targetSheetId, targetCell, clipboardData, options);
        }
    }

    /**
     * シート間でセルを移動
     * @param {string} sourceSheetId
     * @param {string} sourceRange
     * @param {string} targetSheetId
     * @param {string} targetCell
     */
    moveBetweenSheets(sourceSheetId, sourceRange, targetSheetId, targetCell) {
        const clipboardData = this.engine.cut(sourceSheetId, sourceRange);
        if (clipboardData) {
            this.engine.paste(targetSheetId, targetCell, clipboardData);
        }
    }

    /**
     * 複数シートの同じセルに値を設定
     * @param {string[]} sheetIds
     * @param {string} cellRef
     * @param {string} value
     */
    setValueAcrossSheets(sheetIds, cellRef, value) {
        for (const sheetId of sheetIds) {
            this.engine.setCellValue(sheetId, cellRef, value);
        }
    }

    /**
     * 複数シートの同じセルに書式を設定
     * @param {string[]} sheetIds
     * @param {string} cellRef
     * @param {Object} format
     */
    setFormatAcrossSheets(sheetIds, cellRef, format) {
        for (const sheetId of sheetIds) {
            this.engine.setCellFormat(sheetId, cellRef, format);
        }
    }

    /**
     * シートを検索
     * @param {string} query - 検索文字列
     * @param {Object} options
     * @returns {Array<{sheetId: string, sheetName: string, cellRef: string, value: string}>}
     */
    searchAcrossSheets(query, options = {}) {
        const results = [];
        const {
            caseSensitive = false,
            wholeCell = false,
            searchInFormulas = false
        } = options;

        if (!this.engine.workbook) return results;

        const pattern = caseSensitive ? query : query.toLowerCase();

        for (const sheet of this.engine.workbook.sheets) {
            for (const [cellRef, cell] of sheet.cells) {
                let searchTarget = searchInFormulas
                    ? (cell.formula || String(cell.computed ?? ''))
                    : String(cell.computed ?? '');

                if (!caseSensitive) {
                    searchTarget = searchTarget.toLowerCase();
                }

                const matches = wholeCell
                    ? searchTarget === pattern
                    : searchTarget.includes(pattern);

                if (matches) {
                    results.push({
                        sheetId: sheet.id,
                        sheetName: sheet.name,
                        cellRef,
                        value: cell.raw
                    });
                }
            }
        }

        return results;
    }

    /**
     * 検索と置換（複数シート）
     * @param {string} find
     * @param {string} replace
     * @param {Object} options
     * @returns {number} 置換された数
     */
    replaceAcrossSheets(find, replace, options = {}) {
        const {
            caseSensitive = false,
            wholeCell = false,
            sheetIds = null // nullの場合は全シート
        } = options;

        if (!this.engine.workbook) return 0;

        let count = 0;
        const sheets = sheetIds
            ? this.engine.workbook.sheets.filter(s => sheetIds.includes(s.id))
            : this.engine.workbook.sheets;

        for (const sheet of sheets) {
            for (const [cellRef, cell] of sheet.cells) {
                const value = cell.raw;
                let newValue;

                if (wholeCell) {
                    const matches = caseSensitive
                        ? value === find
                        : value.toLowerCase() === find.toLowerCase();

                    if (matches) {
                        newValue = replace;
                    }
                } else {
                    const regex = new RegExp(
                        this._escapeRegex(find),
                        caseSensitive ? 'g' : 'gi'
                    );
                    newValue = value.replace(regex, replace);
                }

                if (newValue !== undefined && newValue !== value) {
                    this.engine.setCellValue(sheet.id, cellRef, newValue);
                    count++;
                }
            }
        }

        return count;
    }
}

export default SheetManager;
