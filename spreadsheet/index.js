/**
 * spreadsheet/index.js - スプレッドシート統合エントリポイント
 *
 * このファイルはAjisaiアプリケーションにスプレッドシートGUIを統合します。
 */

import { SpreadsheetEngine } from './SpreadsheetEngine.js';
import { SpreadsheetGrid } from './SpreadsheetGrid.js';
import { SpreadsheetToolbar } from './SpreadsheetToolbar.js';
import { FormulaParser } from './FormulaParser.js';
import { SheetManager } from './SheetManager.js';
import { CellReference } from './CellReference.js';

/**
 * スプレッドシートアプリケーションクラス
 */
export class SpreadsheetApp {
    constructor() {
        /** @type {SpreadsheetEngine} */
        this.engine = null;
        /** @type {FormulaParser} */
        this.formulaParser = null;
        /** @type {SheetManager} */
        this.sheetManager = null;
        /** @type {SpreadsheetGrid} */
        this.grid = null;
        /** @type {SpreadsheetToolbar} */
        this.toolbar = null;
        /** @type {HTMLElement} */
        this.container = null;

        // モード状態
        this.currentMode = 'text'; // 'text' | 'spreadsheet'
    }

    /**
     * スプレッドシートを初期化
     * @param {HTMLElement} inputArea - 配置先の要素
     */
    init(inputArea) {
        if (!inputArea) {
            console.error('[Spreadsheet] Input area not found');
            return;
        }

        console.log('[Spreadsheet] Initializing spreadsheet...');

        // エンジン初期化
        this.engine = new SpreadsheetEngine();

        // 数式パーサー初期化
        this.formulaParser = new FormulaParser(this.engine);
        this.engine.setFormulaParser(this.formulaParser);

        // シートマネージャー初期化
        this.sheetManager = new SheetManager(this.engine);

        // 初期ワークブック作成
        this.engine.createWorkbook();

        // UI作成
        this.createUI(inputArea);

        // Ajisaiとの連携設定
        this.setupAjisaiBridge();

        console.log('[Spreadsheet] Spreadsheet initialized');
    }

    /**
     * UI要素を作成
     * @param {HTMLElement} inputArea
     */
    createUI(inputArea) {
        // 既存の要素を取得（移動前に参照を保持）
        const heading = inputArea.querySelector('h2');
        const codeInput = inputArea.querySelector('#code-input');
        const controls = inputArea.querySelector('.controls');

        if (!codeInput) {
            console.error('[Spreadsheet] #code-input not found, aborting UI creation');
            return;
        }

        // モード切り替えボタンを作成
        const modeToggle = document.createElement('div');
        modeToggle.className = 'mode-toggle';
        modeToggle.innerHTML = `
            <button class="mode-toggle-btn active" data-mode="text">Text</button>
            <button class="mode-toggle-btn" data-mode="spreadsheet">Spreadsheet</button>
        `;

        // テキストモードコンテナを作成
        const textModeContainer = document.createElement('div');
        textModeContainer.className = 'text-mode-container';
        textModeContainer.style.display = 'flex';
        textModeContainer.style.flexDirection = 'column';
        textModeContainer.style.flex = '1';
        textModeContainer.style.minHeight = '0';

        // スプレッドシートコンテナを作成
        this.container = document.createElement('div');
        this.container.className = 'spreadsheet-container';
        this.container.style.display = 'none';
        this.container.style.flex = '1';
        this.container.style.minHeight = '0';

        // ツールバーを作成
        this.toolbar = document.createElement('spreadsheet-toolbar');
        this.container.appendChild(this.toolbar);

        // グリッドを作成
        this.grid = document.createElement('spreadsheet-grid');
        this.container.appendChild(this.grid);

        // 既存の要素をテキストモードコンテナに移動
        // 注意: appendChild は要素を移動する（コピーではない）
        textModeContainer.appendChild(codeInput);
        if (controls) {
            textModeContainer.appendChild(controls);
        }

        // h2の直後にモード切り替えを挿入
        if (heading && heading.nextSibling) {
            inputArea.insertBefore(modeToggle, heading.nextSibling);
        } else {
            inputArea.appendChild(modeToggle);
        }

        // テキストモードコンテナとスプレッドシートコンテナを追加
        inputArea.appendChild(textModeContainer);
        inputArea.appendChild(this.container);

        // グリッドとツールバーを接続（UI構築完了後）
        this.grid.setEngine(this.engine);
        this.toolbar.connect(this.engine, this.grid);

        // モード切り替えイベント
        modeToggle.addEventListener('click', (e) => {
            const btn = e.target.closest('.mode-toggle-btn');
            if (!btn) return;

            const mode = btn.dataset.mode;
            this.switchMode(mode);

            // ボタンの状態更新
            modeToggle.querySelectorAll('.mode-toggle-btn').forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
        });
    }

    /**
     * モード切り替え
     * @param {'text'|'spreadsheet'} mode
     */
    switchMode(mode) {
        this.currentMode = mode;

        const textContainer = document.querySelector('.text-mode-container');
        const spreadsheetContainer = this.container;

        if (mode === 'text') {
            if (textContainer) textContainer.style.display = 'flex';
            if (spreadsheetContainer) spreadsheetContainer.style.display = 'none';
        } else {
            if (textContainer) textContainer.style.display = 'none';
            if (spreadsheetContainer) spreadsheetContainer.style.display = 'flex';
            // スプレッドシート表示時に再描画
            setTimeout(() => {
                this.grid?._resize();
            }, 0);
        }

        // イベント発火
        document.dispatchEvent(new CustomEvent('spreadsheet-mode-change', {
            detail: { mode }
        }));
    }

    /**
     * Ajisaiとの連携設定
     */
    setupAjisaiBridge() {
        // スプレッドシートデータをAjisaiで使えるようにグローバルに公開
        window.ajisaiSpreadsheet = {
            engine: this.engine,
            sheetManager: this.sheetManager,

            /**
             * セル値を取得
             * @param {string} cellRef - 'A1'形式
             * @returns {*}
             */
            getCell: (cellRef) => {
                const sheet = this.engine.getActiveSheet();
                if (!sheet) return null;
                return this.engine.getCellValue(sheet.id, cellRef);
            },

            /**
             * セル値を設定
             * @param {string} cellRef - 'A1'形式
             * @param {string} value
             */
            setCell: (cellRef, value) => {
                const sheet = this.engine.getActiveSheet();
                if (!sheet) return;
                this.engine.setCellValue(sheet.id, cellRef, String(value));
            },

            /**
             * 範囲の値を取得（2次元配列）
             * @param {string} rangeRef - 'A1:C3'形式
             * @returns {Array<Array<*>>}
             */
            getRange: (rangeRef) => {
                const sheet = this.engine.getActiveSheet();
                if (!sheet) return [];

                const range = CellReference.parseRange(rangeRef);
                if (!range) return [];

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
                        const ref = CellReference.indexToCol(col) + row;
                        rowValues.push(this.engine.getCellValue(sheet.id, ref));
                    }
                    result.push(rowValues);
                }
                return result;
            },

            /**
             * 範囲に値を設定（2次元配列）
             * @param {string} startCell - 'A1'形式
             * @param {Array<Array<*>>} values
             */
            setRange: (startCell, values) => {
                const sheet = this.engine.getActiveSheet();
                if (!sheet) return;

                const start = CellReference.parse(startCell);
                if (!start) return;

                const startCol = CellReference.colToIndex(start.col);
                const startRow = start.row;

                for (let r = 0; r < values.length; r++) {
                    const row = values[r];
                    if (!Array.isArray(row)) continue;

                    for (let c = 0; c < row.length; c++) {
                        const ref = CellReference.indexToCol(startCol + c) + (startRow + r);
                        this.engine.setCellValue(sheet.id, ref, String(row[c] ?? ''));
                    }
                }
            },

            /**
             * ワークブックをJSON形式でエクスポート
             * @returns {Object}
             */
            exportWorkbook: () => {
                return this.engine.exportWorkbook();
            },

            /**
             * JSONからワークブックをインポート
             * @param {Object} data
             */
            importWorkbook: (data) => {
                this.engine.loadWorkbook(data);
                this.grid?.render();
            },

            /**
             * 数式を評価
             * @param {string} formula - '=SUM(A1:A10)'形式
             * @returns {*}
             */
            evaluate: (formula) => {
                if (!formula.startsWith('=')) {
                    formula = '=' + formula;
                }

                const sheet = this.engine.getActiveSheet();
                if (!sheet) return null;

                try {
                    const ast = this.formulaParser.parse(formula.substring(1));
                    return this.formulaParser.evaluate(ast, {
                        sheetId: sheet.id,
                        cellRef: 'A1',
                        engine: this.engine
                    });
                } catch (e) {
                    return '#ERROR: ' + e.message;
                }
            }
        };
    }

    /**
     * スプレッドシートデータをAjisai形式に変換
     * @returns {Object}
     */
    toAjisaiFormat() {
        const workbook = this.engine.exportWorkbook();
        if (!workbook) return null;

        const result = {
            sheets: {}
        };

        for (const sheet of workbook.sheets) {
            const sheetData = {};
            for (const [cellRef, cell] of Object.entries(sheet.cells)) {
                sheetData[cellRef] = cell.computed !== null ? cell.computed : cell.raw;
            }
            result.sheets[sheet.name] = sheetData;
        }

        return result;
    }

    /**
     * リソースのクリーンアップ
     */
    destroy() {
        if (this.container) {
            this.container.remove();
        }
        window.ajisaiSpreadsheet = undefined;
    }
}

// シングルトンインスタンス
export let spreadsheetApp = null;

/**
 * スプレッドシートを初期化（グローバルエントリポイント）
 * @param {HTMLElement} inputArea
 * @returns {SpreadsheetApp}
 */
export function initSpreadsheet(inputArea) {
    if (!spreadsheetApp) {
        spreadsheetApp = new SpreadsheetApp();
        spreadsheetApp.init(inputArea);
    }
    return spreadsheetApp;
}

// エクスポート
export {
    SpreadsheetEngine,
    SpreadsheetGrid,
    SpreadsheetToolbar,
    FormulaParser,
    SheetManager,
    CellReference
};
