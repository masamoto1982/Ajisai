/**
 * SpreadsheetEngine.js - スプレッドシートのコアエンジン
 *
 * 機能:
 * - セルデータの管理
 * - 数式計算の実行
 * - 依存関係の追跡
 * - Undo/Redo管理
 */

import { CellReference } from './CellReference.js';

/**
 * @typedef {Object} CellData
 * @property {string} raw - 入力された生の値
 * @property {string|null} formula - 数式（=で始まる場合、=を除いた部分）
 * @property {*} computed - 計算結果
 * @property {CellFormat} format - 書式設定
 * @property {string|null} error - エラーメッセージ
 */

/**
 * @typedef {Object} CellFormat
 * @property {string} fontFamily - フォント
 * @property {number} fontSize - フォントサイズ(px)
 * @property {boolean} bold - 太字
 * @property {boolean} italic - 斜体
 * @property {string} textAlign - 'left'|'center'|'right'
 * @property {string} verticalAlign - 'top'|'middle'|'bottom'
 * @property {string} backgroundColor - 背景色
 * @property {string} textColor - 文字色
 * @property {string} numberFormat - 数値フォーマット
 * @property {Object} borders - 罫線設定
 */

/**
 * @typedef {Object} SheetData
 * @property {string} id - 一意のID
 * @property {string} name - シート名
 * @property {Map<string, CellData>} cells - セルデータ（キー: 'A1'形式）
 * @property {Object<string, number>} columnWidths - 列幅（キー: 'A','B'...）
 * @property {Object<number, number>} rowHeights - 行高（キー: 1,2,3...）
 * @property {string|null} frozenCell - 固定位置（例: 'B2'）
 * @property {Selection} selection - 現在の選択状態
 */

/**
 * @typedef {Object} Selection
 * @property {string} active - アクティブセル（'A1'形式）
 * @property {string|null} rangeStart - 範囲選択開始
 * @property {string|null} rangeEnd - 範囲選択終了
 */

/**
 * @typedef {Object} WorkbookData
 * @property {string} version - データフォーマットバージョン
 * @property {SheetData[]} sheets - シート配列
 * @property {string} activeSheetId - アクティブシートID
 * @property {Object} namedRanges - 名前付き範囲
 */

/**
 * @typedef {Object} UndoAction
 * @property {string} type - アクションタイプ
 * @property {string} sheetId - 対象シートID
 * @property {*} oldValue - 変更前の値
 * @property {*} newValue - 変更後の値
 * @property {string|null} cellRef - 対象セル参照
 */

export class SpreadsheetEngine extends EventTarget {
    constructor() {
        super();
        /** @type {WorkbookData|null} */
        this.workbook = null;
        /** @type {Map<string, Set<string>>} セル依存関係（key: セル, value: 依存先セル群） */
        this.dependencyGraph = new Map();
        /** @type {Map<string, Set<string>>} 逆依存関係（key: セル, value: このセルに依存するセル群） */
        this.reverseDependencyGraph = new Map();
        /** @type {UndoAction[]} */
        this.undoStack = [];
        /** @type {UndoAction[]} */
        this.redoStack = [];
        this.maxUndoLevels = 100;
        /** @type {import('./FormulaParser.js').FormulaParser|null} */
        this.formulaParser = null;
    }

    /**
     * 数式パーサーを設定
     * @param {import('./FormulaParser.js').FormulaParser} parser
     */
    setFormulaParser(parser) {
        this.formulaParser = parser;
    }

    // === ワークブック操作 ===

    /**
     * 新規ワークブック作成
     * @returns {WorkbookData}
     */
    createWorkbook() {
        const sheet = this._createSheet('Sheet1');
        this.workbook = {
            version: '1.0',
            sheets: [sheet],
            activeSheetId: sheet.id,
            namedRanges: {}
        };
        this.dependencyGraph.clear();
        this.reverseDependencyGraph.clear();
        this.undoStack = [];
        this.redoStack = [];

        this._dispatchEvent('workbookCreated', { workbook: this.workbook });
        return this.workbook;
    }

    /**
     * ワークブック読み込み
     * @param {WorkbookData} data
     */
    loadWorkbook(data) {
        // シートのcellsをMapに変換
        this.workbook = {
            ...data,
            sheets: data.sheets.map(sheet => ({
                ...sheet,
                cells: sheet.cells instanceof Map
                    ? sheet.cells
                    : new Map(Object.entries(sheet.cells || {}))
            }))
        };

        this.dependencyGraph.clear();
        this.reverseDependencyGraph.clear();
        this.undoStack = [];
        this.redoStack = [];

        // 全シートの依存関係を再構築
        for (const sheet of this.workbook.sheets) {
            this._rebuildDependencies(sheet.id);
        }

        this._dispatchEvent('workbookLoaded', { workbook: this.workbook });
    }

    /**
     * ワークブック出力
     * @returns {WorkbookData}
     */
    exportWorkbook() {
        if (!this.workbook) return null;

        return {
            ...this.workbook,
            sheets: this.workbook.sheets.map(sheet => ({
                ...sheet,
                cells: Object.fromEntries(sheet.cells)
            }))
        };
    }

    /**
     * シートを作成
     * @param {string} name
     * @returns {SheetData}
     */
    _createSheet(name) {
        return {
            id: this._generateId(),
            name,
            cells: new Map(),
            columnWidths: {},
            rowHeights: {},
            frozenCell: null,
            selection: {
                active: 'A1',
                rangeStart: null,
                rangeEnd: null
            }
        };
    }

    /**
     * 一意のIDを生成
     * @returns {string}
     */
    _generateId() {
        return 'sheet_' + Date.now() + '_' + Math.random().toString(36).substr(2, 9);
    }

    // === シート操作 ===

    /**
     * シート追加
     * @param {string} [name] - シート名（省略時は自動生成）
     * @returns {SheetData}
     */
    addSheet(name) {
        if (!this.workbook) {
            this.createWorkbook();
            return this.workbook.sheets[0];
        }

        if (!name) {
            let index = this.workbook.sheets.length + 1;
            name = `Sheet${index}`;
            while (this.workbook.sheets.some(s => s.name === name)) {
                index++;
                name = `Sheet${index}`;
            }
        }

        const sheet = this._createSheet(name);
        this.workbook.sheets.push(sheet);

        this._dispatchEvent('sheetAdded', { sheet });
        return sheet;
    }

    /**
     * シート削除
     * @param {string} sheetId
     */
    deleteSheet(sheetId) {
        if (!this.workbook || this.workbook.sheets.length <= 1) {
            return; // 最低1シートは必要
        }

        const index = this.workbook.sheets.findIndex(s => s.id === sheetId);
        if (index === -1) return;

        const deletedSheet = this.workbook.sheets.splice(index, 1)[0];

        // アクティブシートが削除された場合
        if (this.workbook.activeSheetId === sheetId) {
            this.workbook.activeSheetId = this.workbook.sheets[Math.min(index, this.workbook.sheets.length - 1)].id;
        }

        // 依存関係をクリア
        this._clearSheetDependencies(sheetId);

        this._dispatchEvent('sheetDeleted', { sheetId, sheet: deletedSheet });
    }

    /**
     * シート名変更
     * @param {string} sheetId
     * @param {string} newName
     */
    renameSheet(sheetId, newName) {
        const sheet = this._getSheet(sheetId);
        if (!sheet) return;

        const oldName = sheet.name;
        sheet.name = newName;

        // TODO: シート参照を含む数式の更新

        this._dispatchEvent('sheetRenamed', { sheetId, oldName, newName });
    }

    /**
     * シート順序変更
     * @param {string} sheetId
     * @param {number} newIndex
     */
    reorderSheet(sheetId, newIndex) {
        if (!this.workbook) return;

        const currentIndex = this.workbook.sheets.findIndex(s => s.id === sheetId);
        if (currentIndex === -1) return;

        const [sheet] = this.workbook.sheets.splice(currentIndex, 1);
        this.workbook.sheets.splice(newIndex, 0, sheet);

        this._dispatchEvent('sheetReordered', { sheetId, oldIndex: currentIndex, newIndex });
    }

    /**
     * シート複製
     * @param {string} sheetId
     * @returns {SheetData}
     */
    duplicateSheet(sheetId) {
        const source = this._getSheet(sheetId);
        if (!source) return null;

        let newName = source.name + ' (Copy)';
        let counter = 1;
        while (this.workbook.sheets.some(s => s.name === newName)) {
            counter++;
            newName = source.name + ` (Copy ${counter})`;
        }

        const newSheet = {
            ...this._createSheet(newName),
            cells: new Map(source.cells),
            columnWidths: { ...source.columnWidths },
            rowHeights: { ...source.rowHeights }
        };

        this.workbook.sheets.push(newSheet);
        this._rebuildDependencies(newSheet.id);

        this._dispatchEvent('sheetDuplicated', { sourceId: sheetId, newSheet });
        return newSheet;
    }

    /**
     * アクティブシートを設定
     * @param {string} sheetId
     */
    setActiveSheet(sheetId) {
        if (!this.workbook) return;

        const sheet = this._getSheet(sheetId);
        if (!sheet) return;

        this.workbook.activeSheetId = sheetId;
        this._dispatchEvent('activeSheetChanged', { sheetId });
    }

    /**
     * アクティブシートを取得
     * @returns {SheetData|null}
     */
    getActiveSheet() {
        if (!this.workbook) return null;
        return this._getSheet(this.workbook.activeSheetId);
    }

    /**
     * シートを取得
     * @param {string} sheetId
     * @returns {SheetData|null}
     */
    _getSheet(sheetId) {
        if (!this.workbook) return null;
        return this.workbook.sheets.find(s => s.id === sheetId) || null;
    }

    /**
     * シート名からシートを取得
     * @param {string} name
     * @returns {SheetData|null}
     */
    getSheetByName(name) {
        if (!this.workbook) return null;
        return this.workbook.sheets.find(s => s.name === name) || null;
    }

    // === セル操作 ===

    /**
     * デフォルトの書式設定
     * @returns {CellFormat}
     */
    _getDefaultFormat() {
        return {
            fontFamily: '-apple-system, BlinkMacSystemFont, sans-serif',
            fontSize: 13,
            bold: false,
            italic: false,
            textAlign: 'left',
            verticalAlign: 'middle',
            backgroundColor: 'transparent',
            textColor: '#000000',
            numberFormat: 'general',
            borders: {}
        };
    }

    /**
     * セル値設定
     * @param {string} sheetId
     * @param {string} cellRef - 'A1'形式
     * @param {string} value - 入力値
     * @param {boolean} recordUndo - Undo記録するか
     */
    setCellValue(sheetId, cellRef, value, recordUndo = true) {
        const sheet = this._getSheet(sheetId);
        if (!sheet) return;

        const oldCell = sheet.cells.get(cellRef);
        const oldValue = oldCell ? { ...oldCell } : null;

        // 数式かどうかを判定
        const isFormula = typeof value === 'string' && value.startsWith('=');

        /** @type {CellData} */
        const newCell = {
            raw: value,
            formula: isFormula ? value.substring(1) : null,
            computed: null,
            format: oldCell?.format ? { ...oldCell.format } : this._getDefaultFormat(),
            error: null
        };

        // 値を計算
        if (isFormula && this.formulaParser) {
            try {
                const ast = this.formulaParser.parse(newCell.formula);
                const context = { sheetId, cellRef, engine: this };
                newCell.computed = this.formulaParser.evaluate(ast, context);

                // 依存関係を更新
                const deps = this.formulaParser.extractReferences(newCell.formula);
                this._updateDependencies(sheetId, cellRef, deps);
            } catch (e) {
                newCell.error = e.message || '#ERROR!';
                newCell.computed = null;
            }
        } else if (!isFormula) {
            // 数値かどうか判定
            const num = parseFloat(value);
            if (!isNaN(num) && value.trim() !== '') {
                newCell.computed = num;
            } else {
                newCell.computed = value;
            }
            // 数式でない場合は依存関係をクリア
            this._updateDependencies(sheetId, cellRef, []);
        }

        // 空の値の場合はセルを削除
        if (value === '' || value === null || value === undefined) {
            sheet.cells.delete(cellRef);
        } else {
            sheet.cells.set(cellRef, newCell);
        }

        // Undo記録
        if (recordUndo) {
            this._recordUndo({
                type: 'setCellValue',
                sheetId,
                cellRef,
                oldValue,
                newValue: newCell
            });
        }

        this._dispatchEvent('cellChanged', { sheetId, cellRef, oldValue, newValue: newCell });

        // 依存セルを再計算
        this._recalculateDependents(sheetId, cellRef);
    }

    /**
     * セル値取得
     * @param {string} sheetId
     * @param {string} cellRef
     * @returns {CellData|null}
     */
    getCell(sheetId, cellRef) {
        const sheet = this._getSheet(sheetId);
        if (!sheet) return null;
        return sheet.cells.get(cellRef) || null;
    }

    /**
     * セルの計算値を取得
     * @param {string} sheetId
     * @param {string} cellRef
     * @returns {*}
     */
    getCellValue(sheetId, cellRef) {
        const cell = this.getCell(sheetId, cellRef);
        if (!cell) return null;
        if (cell.error) return cell.error;
        return cell.computed;
    }

    /**
     * 範囲のセル取得
     * @param {string} sheetId
     * @param {string} rangeRef - 'A1:C3'形式
     * @returns {Map<string, CellData>}
     */
    getCellRange(sheetId, rangeRef) {
        const result = new Map();
        const range = CellReference.parseRange(rangeRef);
        if (!range) return result;

        const sheet = this._getSheet(sheetId);
        if (!sheet) return result;

        for (const ref of CellReference.iterateRange(range)) {
            const cell = sheet.cells.get(ref);
            if (cell) {
                result.set(ref, cell);
            }
        }

        return result;
    }

    /**
     * セル書式設定
     * @param {string} sheetId
     * @param {string} cellRef
     * @param {Partial<CellFormat>} format
     */
    setCellFormat(sheetId, cellRef, format) {
        const sheet = this._getSheet(sheetId);
        if (!sheet) return;

        let cell = sheet.cells.get(cellRef);
        if (!cell) {
            // セルがない場合は空セルを作成
            cell = {
                raw: '',
                formula: null,
                computed: '',
                format: this._getDefaultFormat(),
                error: null
            };
            sheet.cells.set(cellRef, cell);
        }

        const oldFormat = { ...cell.format };
        cell.format = { ...cell.format, ...format };

        this._recordUndo({
            type: 'setCellFormat',
            sheetId,
            cellRef,
            oldValue: oldFormat,
            newValue: cell.format
        });

        this._dispatchEvent('cellFormatChanged', { sheetId, cellRef, format: cell.format });
    }

    /**
     * 範囲に書式適用
     * @param {string} sheetId
     * @param {string} rangeRef
     * @param {Partial<CellFormat>} format
     */
    setRangeFormat(sheetId, rangeRef, format) {
        const range = CellReference.parseRange(rangeRef);
        if (!range) return;

        for (const cellRef of CellReference.iterateRange(range)) {
            this.setCellFormat(sheetId, cellRef, format);
        }
    }

    // === 行・列操作 ===

    /**
     * 行挿入
     * @param {string} sheetId
     * @param {number} rowIndex - 挿入位置
     * @param {number} count - 挿入行数
     */
    insertRows(sheetId, rowIndex, count = 1) {
        const sheet = this._getSheet(sheetId);
        if (!sheet) return;

        // セル参照を更新
        const newCells = new Map();
        for (const [ref, cell] of sheet.cells) {
            const parsed = CellReference.parse(ref);
            if (parsed) {
                const updated = CellReference.updateForInsertDelete(parsed, 'row', rowIndex, count);
                if (updated) {
                    const newRef = CellReference.toString(updated, false);
                    newCells.set(newRef, cell);

                    // 数式内の参照も更新
                    if (cell.formula && this.formulaParser) {
                        cell.formula = this.formulaParser.updateReferences(cell.formula, {
                            type: 'row',
                            index: rowIndex,
                            count
                        });
                        cell.raw = '=' + cell.formula;
                    }
                }
            }
        }
        sheet.cells = newCells;

        // 行高を更新
        const newRowHeights = {};
        for (const [row, height] of Object.entries(sheet.rowHeights)) {
            const rowNum = parseInt(row, 10);
            if (rowNum >= rowIndex) {
                newRowHeights[rowNum + count] = height;
            } else {
                newRowHeights[rowNum] = height;
            }
        }
        sheet.rowHeights = newRowHeights;

        this._rebuildDependencies(sheetId);
        this._dispatchEvent('rowsInserted', { sheetId, rowIndex, count });
    }

    /**
     * 行削除
     * @param {string} sheetId
     * @param {number} rowIndex
     * @param {number} count
     */
    deleteRows(sheetId, rowIndex, count = 1) {
        const sheet = this._getSheet(sheetId);
        if (!sheet) return;

        // セル参照を更新
        const newCells = new Map();
        for (const [ref, cell] of sheet.cells) {
            const parsed = CellReference.parse(ref);
            if (parsed) {
                const updated = CellReference.updateForInsertDelete(parsed, 'row', rowIndex, -count);
                if (updated) {
                    const newRef = CellReference.toString(updated, false);
                    newCells.set(newRef, cell);

                    // 数式内の参照も更新
                    if (cell.formula && this.formulaParser) {
                        cell.formula = this.formulaParser.updateReferences(cell.formula, {
                            type: 'row',
                            index: rowIndex,
                            count: -count
                        });
                        cell.raw = '=' + cell.formula;
                    }
                }
            }
        }
        sheet.cells = newCells;

        // 行高を更新
        const newRowHeights = {};
        for (const [row, height] of Object.entries(sheet.rowHeights)) {
            const rowNum = parseInt(row, 10);
            if (rowNum < rowIndex) {
                newRowHeights[rowNum] = height;
            } else if (rowNum >= rowIndex + count) {
                newRowHeights[rowNum - count] = height;
            }
        }
        sheet.rowHeights = newRowHeights;

        this._rebuildDependencies(sheetId);
        this._dispatchEvent('rowsDeleted', { sheetId, rowIndex, count });
    }

    /**
     * 列挿入
     * @param {string} sheetId
     * @param {string} colRef - 'A','B'等
     * @param {number} count
     */
    insertColumns(sheetId, colRef, count = 1) {
        const sheet = this._getSheet(sheetId);
        if (!sheet) return;

        const colIndex = CellReference.colToIndex(colRef);

        // セル参照を更新
        const newCells = new Map();
        for (const [ref, cell] of sheet.cells) {
            const parsed = CellReference.parse(ref);
            if (parsed) {
                const updated = CellReference.updateForInsertDelete(parsed, 'col', colIndex, count);
                if (updated) {
                    const newRef = CellReference.toString(updated, false);
                    newCells.set(newRef, cell);

                    // 数式内の参照も更新
                    if (cell.formula && this.formulaParser) {
                        cell.formula = this.formulaParser.updateReferences(cell.formula, {
                            type: 'col',
                            index: colIndex,
                            count
                        });
                        cell.raw = '=' + cell.formula;
                    }
                }
            }
        }
        sheet.cells = newCells;

        // 列幅を更新
        const newColumnWidths = {};
        for (const [col, width] of Object.entries(sheet.columnWidths)) {
            const idx = CellReference.colToIndex(col);
            if (idx >= colIndex) {
                newColumnWidths[CellReference.indexToCol(idx + count)] = width;
            } else {
                newColumnWidths[col] = width;
            }
        }
        sheet.columnWidths = newColumnWidths;

        this._rebuildDependencies(sheetId);
        this._dispatchEvent('columnsInserted', { sheetId, colRef, count });
    }

    /**
     * 列削除
     * @param {string} sheetId
     * @param {string} colRef
     * @param {number} count
     */
    deleteColumns(sheetId, colRef, count = 1) {
        const sheet = this._getSheet(sheetId);
        if (!sheet) return;

        const colIndex = CellReference.colToIndex(colRef);

        // セル参照を更新
        const newCells = new Map();
        for (const [ref, cell] of sheet.cells) {
            const parsed = CellReference.parse(ref);
            if (parsed) {
                const updated = CellReference.updateForInsertDelete(parsed, 'col', colIndex, -count);
                if (updated) {
                    const newRef = CellReference.toString(updated, false);
                    newCells.set(newRef, cell);

                    // 数式内の参照も更新
                    if (cell.formula && this.formulaParser) {
                        cell.formula = this.formulaParser.updateReferences(cell.formula, {
                            type: 'col',
                            index: colIndex,
                            count: -count
                        });
                        cell.raw = '=' + cell.formula;
                    }
                }
            }
        }
        sheet.cells = newCells;

        // 列幅を更新
        const newColumnWidths = {};
        for (const [col, width] of Object.entries(sheet.columnWidths)) {
            const idx = CellReference.colToIndex(col);
            if (idx < colIndex) {
                newColumnWidths[col] = width;
            } else if (idx >= colIndex + count) {
                newColumnWidths[CellReference.indexToCol(idx - count)] = width;
            }
        }
        sheet.columnWidths = newColumnWidths;

        this._rebuildDependencies(sheetId);
        this._dispatchEvent('columnsDeleted', { sheetId, colRef, count });
    }

    /**
     * 列幅設定
     * @param {string} sheetId
     * @param {string} colRef
     * @param {number} width
     */
    setColumnWidth(sheetId, colRef, width) {
        const sheet = this._getSheet(sheetId);
        if (!sheet) return;

        sheet.columnWidths[colRef] = width;
        this._dispatchEvent('columnWidthChanged', { sheetId, colRef, width });
    }

    /**
     * 行高設定
     * @param {string} sheetId
     * @param {number} rowIndex
     * @param {number} height
     */
    setRowHeight(sheetId, rowIndex, height) {
        const sheet = this._getSheet(sheetId);
        if (!sheet) return;

        sheet.rowHeights[rowIndex] = height;
        this._dispatchEvent('rowHeightChanged', { sheetId, rowIndex, height });
    }

    /**
     * 列幅取得
     * @param {string} sheetId
     * @param {string} colRef
     * @param {number} defaultWidth
     * @returns {number}
     */
    getColumnWidth(sheetId, colRef, defaultWidth = 100) {
        const sheet = this._getSheet(sheetId);
        if (!sheet) return defaultWidth;
        return sheet.columnWidths[colRef] ?? defaultWidth;
    }

    /**
     * 行高取得
     * @param {string} sheetId
     * @param {number} rowIndex
     * @param {number} defaultHeight
     * @returns {number}
     */
    getRowHeight(sheetId, rowIndex, defaultHeight = 24) {
        const sheet = this._getSheet(sheetId);
        if (!sheet) return defaultHeight;
        return sheet.rowHeights[rowIndex] ?? defaultHeight;
    }

    // === クリップボード操作 ===

    /**
     * 範囲コピー
     * @param {string} sheetId
     * @param {string} rangeRef
     * @returns {Object}
     */
    copy(sheetId, rangeRef) {
        const range = CellReference.parseRange(rangeRef);
        if (!range) return null;

        const cells = this.getCellRange(sheetId, rangeRef);
        const size = CellReference.getRangeSize(range);

        return {
            type: 'copy',
            sheetId,
            range,
            cells: Object.fromEntries(cells),
            size
        };
    }

    /**
     * 範囲切り取り
     * @param {string} sheetId
     * @param {string} rangeRef
     * @returns {Object}
     */
    cut(sheetId, rangeRef) {
        const data = this.copy(sheetId, rangeRef);
        if (!data) return null;

        data.type = 'cut';
        data.sourceRange = rangeRef;

        return data;
    }

    /**
     * 貼り付け
     * @param {string} sheetId
     * @param {string} targetCell
     * @param {Object} clipboardData
     * @param {Object} options
     */
    paste(sheetId, targetCell, clipboardData, options = {}) {
        if (!clipboardData) return;

        const target = CellReference.parse(targetCell);
        if (!target) return;

        const { valuesOnly = false, formatOnly = false } = options;

        const targetColIndex = CellReference.colToIndex(target.col);
        const startColIndex = CellReference.colToIndex(clipboardData.range.start.col);
        const startRow = clipboardData.range.start.row;

        for (const [ref, cellData] of Object.entries(clipboardData.cells)) {
            const srcCell = CellReference.parse(ref);
            if (!srcCell) continue;

            const srcColIndex = CellReference.colToIndex(srcCell.col);
            const deltaCol = srcColIndex - startColIndex;
            const deltaRow = srcCell.row - startRow;

            const newColIndex = targetColIndex + deltaCol;
            const newRow = target.row + deltaRow;

            if (newColIndex < 0 || newRow < 1) continue;

            const newRef = CellReference.indexToCol(newColIndex) + newRow;

            if (formatOnly) {
                this.setCellFormat(sheetId, newRef, cellData.format);
            } else if (valuesOnly) {
                // 計算値のみ貼り付け
                this.setCellValue(sheetId, newRef, String(cellData.computed ?? ''));
            } else {
                // すべて貼り付け
                this.setCellValue(sheetId, newRef, cellData.raw);
                if (cellData.format) {
                    this.setCellFormat(sheetId, newRef, cellData.format);
                }
            }
        }

        // 切り取りの場合は元を削除
        if (clipboardData.type === 'cut' && clipboardData.sourceRange) {
            const range = CellReference.parseRange(clipboardData.sourceRange);
            if (range) {
                for (const ref of CellReference.iterateRange(range)) {
                    this.setCellValue(clipboardData.sheetId, ref, '');
                }
            }
        }

        this._dispatchEvent('paste', { sheetId, targetCell, clipboardData });
    }

    // === 計算エンジン ===

    /**
     * 全セル再計算
     */
    recalculate() {
        if (!this.workbook) return;

        for (const sheet of this.workbook.sheets) {
            for (const [cellRef, cell] of sheet.cells) {
                if (cell.formula && this.formulaParser) {
                    try {
                        const ast = this.formulaParser.parse(cell.formula);
                        const context = { sheetId: sheet.id, cellRef, engine: this };
                        cell.computed = this.formulaParser.evaluate(ast, context);
                        cell.error = null;
                    } catch (e) {
                        cell.error = e.message || '#ERROR!';
                        cell.computed = null;
                    }
                }
            }
        }

        this._dispatchEvent('recalculated');
    }

    /**
     * 特定セルと依存セルを再計算
     * @param {string} sheetId
     * @param {string} cellRef
     */
    recalculateCell(sheetId, cellRef) {
        const cell = this.getCell(sheetId, cellRef);
        if (!cell || !cell.formula || !this.formulaParser) return;

        try {
            const ast = this.formulaParser.parse(cell.formula);
            const context = { sheetId, cellRef, engine: this };
            cell.computed = this.formulaParser.evaluate(ast, context);
            cell.error = null;
        } catch (e) {
            cell.error = e.message || '#ERROR!';
            cell.computed = null;
        }

        this._dispatchEvent('cellChanged', { sheetId, cellRef, newValue: cell });
    }

    /**
     * 依存セルを再計算
     * @param {string} sheetId
     * @param {string} cellRef
     */
    _recalculateDependents(sheetId, cellRef) {
        const key = `${sheetId}!${cellRef}`;
        const dependents = this.reverseDependencyGraph.get(key);
        if (!dependents) return;

        for (const depKey of dependents) {
            const [depSheetId, depCellRef] = depKey.split('!');
            this.recalculateCell(depSheetId, depCellRef);
            // 再帰的に依存セルを更新
            this._recalculateDependents(depSheetId, depCellRef);
        }
    }

    /**
     * 依存関係グラフ更新
     * @param {string} sheetId
     * @param {string} cellRef
     * @param {Array} dependencies
     */
    _updateDependencies(sheetId, cellRef, dependencies) {
        const key = `${sheetId}!${cellRef}`;

        // 既存の依存関係をクリア
        const oldDeps = this.dependencyGraph.get(key);
        if (oldDeps) {
            for (const dep of oldDeps) {
                const reverseDeps = this.reverseDependencyGraph.get(dep);
                if (reverseDeps) {
                    reverseDeps.delete(key);
                }
            }
        }

        // 新しい依存関係を設定
        const newDeps = new Set();
        for (const dep of dependencies) {
            const depRef = CellReference.parse(dep);
            if (depRef) {
                const depSheetId = depRef.sheetName
                    ? this.getSheetByName(depRef.sheetName)?.id || sheetId
                    : sheetId;
                const depKey = `${depSheetId}!${depRef.col}${depRef.row}`;
                newDeps.add(depKey);

                // 逆依存関係を更新
                if (!this.reverseDependencyGraph.has(depKey)) {
                    this.reverseDependencyGraph.set(depKey, new Set());
                }
                this.reverseDependencyGraph.get(depKey).add(key);
            }
        }

        if (newDeps.size > 0) {
            this.dependencyGraph.set(key, newDeps);
        } else {
            this.dependencyGraph.delete(key);
        }
    }

    /**
     * シートの依存関係を再構築
     * @param {string} sheetId
     */
    _rebuildDependencies(sheetId) {
        const sheet = this._getSheet(sheetId);
        if (!sheet || !this.formulaParser) return;

        for (const [cellRef, cell] of sheet.cells) {
            if (cell.formula) {
                const deps = this.formulaParser.extractReferences(cell.formula);
                this._updateDependencies(sheetId, cellRef, deps);
            }
        }
    }

    /**
     * シートの依存関係をクリア
     * @param {string} sheetId
     */
    _clearSheetDependencies(sheetId) {
        const prefix = `${sheetId}!`;

        for (const [key] of this.dependencyGraph) {
            if (key.startsWith(prefix)) {
                this.dependencyGraph.delete(key);
            }
        }

        for (const [key, deps] of this.reverseDependencyGraph) {
            if (key.startsWith(prefix)) {
                this.reverseDependencyGraph.delete(key);
            } else {
                for (const dep of deps) {
                    if (dep.startsWith(prefix)) {
                        deps.delete(dep);
                    }
                }
            }
        }
    }

    // === Undo/Redo ===

    /**
     * Undo操作を記録
     * @param {UndoAction} action
     */
    _recordUndo(action) {
        this.undoStack.push(action);
        if (this.undoStack.length > this.maxUndoLevels) {
            this.undoStack.shift();
        }
        this.redoStack = []; // Redoスタックをクリア
    }

    /**
     * 操作を元に戻す
     */
    undo() {
        const action = this.undoStack.pop();
        if (!action) return;

        this._applyAction(action, true);
        this.redoStack.push(action);

        this._dispatchEvent('undo', { action });
    }

    /**
     * 操作をやり直す
     */
    redo() {
        const action = this.redoStack.pop();
        if (!action) return;

        this._applyAction(action, false);
        this.undoStack.push(action);

        this._dispatchEvent('redo', { action });
    }

    /**
     * アクションを適用
     * @param {UndoAction} action
     * @param {boolean} isUndo
     */
    _applyAction(action, isUndo) {
        const value = isUndo ? action.oldValue : action.newValue;

        switch (action.type) {
            case 'setCellValue':
                if (value) {
                    const sheet = this._getSheet(action.sheetId);
                    if (sheet) {
                        sheet.cells.set(action.cellRef, value);
                    }
                } else {
                    const sheet = this._getSheet(action.sheetId);
                    if (sheet) {
                        sheet.cells.delete(action.cellRef);
                    }
                }
                this._dispatchEvent('cellChanged', {
                    sheetId: action.sheetId,
                    cellRef: action.cellRef,
                    newValue: value
                });
                break;

            case 'setCellFormat':
                const cell = this.getCell(action.sheetId, action.cellRef);
                if (cell) {
                    cell.format = value;
                    this._dispatchEvent('cellFormatChanged', {
                        sheetId: action.sheetId,
                        cellRef: action.cellRef,
                        format: value
                    });
                }
                break;
        }
    }

    /**
     * Undo可能か
     * @returns {boolean}
     */
    canUndo() {
        return this.undoStack.length > 0;
    }

    /**
     * Redo可能か
     * @returns {boolean}
     */
    canRedo() {
        return this.redoStack.length > 0;
    }

    // === 選択操作 ===

    /**
     * 選択を設定
     * @param {string} sheetId
     * @param {string} active - アクティブセル
     * @param {string|null} rangeStart - 範囲開始
     * @param {string|null} rangeEnd - 範囲終了
     */
    setSelection(sheetId, active, rangeStart = null, rangeEnd = null) {
        const sheet = this._getSheet(sheetId);
        if (!sheet) return;

        sheet.selection = {
            active,
            rangeStart,
            rangeEnd
        };

        this._dispatchEvent('selectionChanged', {
            sheetId,
            selection: sheet.selection
        });
    }

    /**
     * 選択を取得
     * @param {string} sheetId
     * @returns {Selection|null}
     */
    getSelection(sheetId) {
        const sheet = this._getSheet(sheetId);
        return sheet?.selection || null;
    }

    // === イベント発火 ===

    /**
     * イベントを発火
     * @param {string} type
     * @param {Object} detail
     */
    _dispatchEvent(type, detail = {}) {
        this.dispatchEvent(new CustomEvent(type, { detail }));
    }
}

export default SpreadsheetEngine;
