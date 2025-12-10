/**
 * SpreadsheetGrid.js - スプレッドシートのグリッドUI
 *
 * Canvas + DOM ハイブリッド描画
 * - セルはCanvasで高速描画
 * - 編集中セルはDOMのinput/textareaを使用
 */

import { CellReference } from './CellReference.js';

export class SpreadsheetGrid extends HTMLElement {
    constructor() {
        super();

        /** @type {import('./SpreadsheetEngine.js').SpreadsheetEngine|null} */
        this.engine = null;

        /** @type {HTMLCanvasElement|null} */
        this.canvas = null;

        /** @type {CanvasRenderingContext2D|null} */
        this.ctx = null;

        /** @type {HTMLInputElement|null} */
        this.editorElement = null;

        // 表示状態
        this.scrollX = 0;
        this.scrollY = 0;
        this.visibleRange = { startCol: 0, endCol: 0, startRow: 0, endRow: 0 };

        // デフォルトサイズ
        this.defaultColWidth = 100;
        this.defaultRowHeight = 24;
        this.headerWidth = 50;  // 行ヘッダー幅
        this.headerHeight = 24; // 列ヘッダー高

        // 編集状態
        this.isEditing = false;
        this.editingCell = null;

        // リサイズ状態
        this.isResizingColumn = false;
        this.isResizingRow = false;
        this.resizeTarget = null;
        this.resizeStartPos = 0;
        this.resizeStartSize = 0;

        // 選択状態
        this.isSelecting = false;
        this.selectionStart = null;

        // デバイスピクセル比
        this.dpr = window.devicePixelRatio || 1;

        // カラー定義
        this.colors = {
            headerBg: '#f8f9fa',
            headerBorder: '#e0e0e0',
            gridLine: '#e2e2e2',
            selectionBg: 'rgba(26, 115, 232, 0.1)',
            selectionBorder: '#1a73e8',
            activeCellBorder: '#1a73e8',
            white: '#ffffff',
            black: '#000000',
            headerText: '#5f6368'
        };

        // バインドされたイベントハンドラ
        this._handleMouseDown = this._handleMouseDown.bind(this);
        this._handleMouseMove = this._handleMouseMove.bind(this);
        this._handleMouseUp = this._handleMouseUp.bind(this);
        this._handleDoubleClick = this._handleDoubleClick.bind(this);
        this._handleKeyDown = this._handleKeyDown.bind(this);
        this._handleWheel = this._handleWheel.bind(this);
        this._handleContextMenu = this._handleContextMenu.bind(this);
        this._handleResize = this._handleResize.bind(this);
    }

    // === ライフサイクル ===

    connectedCallback() {
        this._createCanvas();
        this._createEditor();
        this._attachEventListeners();
        this._resize();

        // ResizeObserverでサイズ変更を監視
        this.resizeObserver = new ResizeObserver(() => this._resize());
        this.resizeObserver.observe(this);
    }

    disconnectedCallback() {
        this._removeEventListeners();
        if (this.resizeObserver) {
            this.resizeObserver.disconnect();
        }
    }

    /**
     * Canvasを作成
     */
    _createCanvas() {
        this.canvas = document.createElement('canvas');
        this.canvas.style.cssText = 'position: absolute; top: 0; left: 0;';
        this.appendChild(this.canvas);
        this.ctx = this.canvas.getContext('2d');
    }

    /**
     * エディタ要素を作成
     */
    _createEditor() {
        this.editorElement = document.createElement('input');
        this.editorElement.type = 'text';
        this.editorElement.className = 'cell-editor';
        this.editorElement.style.display = 'none';
        this.editorElement.addEventListener('blur', () => this.commitEdit());
        this.editorElement.addEventListener('keydown', (e) => this._handleEditorKeyDown(e));
        this.editorElement.addEventListener('compositionstart', () => {
            this.editorElement.dataset.composing = 'true';
        });
        this.editorElement.addEventListener('compositionend', () => {
            this.editorElement.dataset.composing = 'false';
        });
        this.appendChild(this.editorElement);
    }

    /**
     * イベントリスナーを設定
     */
    _attachEventListeners() {
        this.canvas.addEventListener('mousedown', this._handleMouseDown);
        this.canvas.addEventListener('mousemove', this._handleMouseMove);
        this.canvas.addEventListener('dblclick', this._handleDoubleClick);
        this.canvas.addEventListener('wheel', this._handleWheel, { passive: false });
        this.canvas.addEventListener('contextmenu', this._handleContextMenu);
        document.addEventListener('mouseup', this._handleMouseUp);
        document.addEventListener('mousemove', this._handleMouseMove);
        this.addEventListener('keydown', this._handleKeyDown);
        window.addEventListener('resize', this._handleResize);

        // フォーカス可能にする
        this.tabIndex = 0;
    }

    /**
     * イベントリスナーを削除
     */
    _removeEventListeners() {
        this.canvas.removeEventListener('mousedown', this._handleMouseDown);
        this.canvas.removeEventListener('mousemove', this._handleMouseMove);
        this.canvas.removeEventListener('dblclick', this._handleDoubleClick);
        this.canvas.removeEventListener('wheel', this._handleWheel);
        this.canvas.removeEventListener('contextmenu', this._handleContextMenu);
        document.removeEventListener('mouseup', this._handleMouseUp);
        document.removeEventListener('mousemove', this._handleMouseMove);
        this.removeEventListener('keydown', this._handleKeyDown);
        window.removeEventListener('resize', this._handleResize);
    }

    /**
     * エンジンとの接続
     * @param {import('./SpreadsheetEngine.js').SpreadsheetEngine} engine
     */
    setEngine(engine) {
        this.engine = engine;

        // イベントリスナー設定
        engine.addEventListener('cellChanged', () => this.render());
        engine.addEventListener('selectionChanged', () => this.render());
        engine.addEventListener('sheetChanged', () => this.render());
        engine.addEventListener('recalculated', () => this.render());
        engine.addEventListener('columnWidthChanged', () => this.render());
        engine.addEventListener('rowHeightChanged', () => this.render());

        this.render();
    }

    /**
     * サイズ変更処理
     */
    _resize() {
        const rect = this.getBoundingClientRect();
        const width = rect.width;
        const height = rect.height;

        if (width === 0 || height === 0) return;

        // Canvasサイズ設定（デバイスピクセル比対応）
        this.canvas.width = width * this.dpr;
        this.canvas.height = height * this.dpr;
        this.canvas.style.width = width + 'px';
        this.canvas.style.height = height + 'px';

        this.ctx.scale(this.dpr, this.dpr);

        this._calculateVisibleRange();
        this.render();
    }

    _handleResize() {
        this._resize();
    }

    // === 描画 ===

    /**
     * 全体を再描画
     */
    render() {
        if (!this.ctx || !this.engine) return;

        const width = this.canvas.width / this.dpr;
        const height = this.canvas.height / this.dpr;

        // 背景クリア
        this.ctx.fillStyle = this.colors.white;
        this.ctx.fillRect(0, 0, width, height);

        this._calculateVisibleRange();
        this._drawGrid();
        this._drawColumnHeaders();
        this._drawRowHeaders();
        this._drawCells();
        this._drawSelection();
        this._drawCorner();
    }

    /**
     * 可視範囲を計算
     */
    _calculateVisibleRange() {
        if (!this.engine) return;

        const sheet = this.engine.getActiveSheet();
        if (!sheet) return;

        const width = this.canvas.width / this.dpr;
        const height = this.canvas.height / this.dpr;

        // 開始列
        let x = this.headerWidth;
        let startCol = 0;
        while (x < this.scrollX && startCol < 1000) {
            x += this._getColumnWidth(startCol);
            startCol++;
        }

        // 終了列
        let endCol = startCol;
        while (x < this.scrollX + width && endCol < 1000) {
            x += this._getColumnWidth(endCol);
            endCol++;
        }
        endCol++;

        // 開始行
        let y = this.headerHeight;
        let startRow = 1;
        while (y < this.scrollY && startRow < 10000) {
            y += this._getRowHeight(startRow);
            startRow++;
        }

        // 終了行
        let endRow = startRow;
        while (y < this.scrollY + height && endRow < 10000) {
            y += this._getRowHeight(endRow);
            endRow++;
        }
        endRow++;

        this.visibleRange = { startCol, endCol, startRow, endRow };
    }

    /**
     * グリッド線描画
     */
    _drawGrid() {
        const width = this.canvas.width / this.dpr;
        const height = this.canvas.height / this.dpr;
        const { startCol, endCol, startRow, endRow } = this.visibleRange;

        this.ctx.strokeStyle = this.colors.gridLine;
        this.ctx.lineWidth = 1;

        // 縦線
        let x = this.headerWidth - this.scrollX + this._getColumnX(startCol);
        for (let col = startCol; col <= endCol; col++) {
            const colWidth = this._getColumnWidth(col);
            this.ctx.beginPath();
            this.ctx.moveTo(Math.floor(x) + 0.5, this.headerHeight);
            this.ctx.lineTo(Math.floor(x) + 0.5, height);
            this.ctx.stroke();
            x += colWidth;
        }

        // 横線
        let y = this.headerHeight - this.scrollY + this._getRowY(startRow);
        for (let row = startRow; row <= endRow; row++) {
            const rowHeight = this._getRowHeight(row);
            this.ctx.beginPath();
            this.ctx.moveTo(this.headerWidth, Math.floor(y) + 0.5);
            this.ctx.lineTo(width, Math.floor(y) + 0.5);
            this.ctx.stroke();
            y += rowHeight;
        }
    }

    /**
     * 列ヘッダー描画（A, B, C...）
     */
    _drawColumnHeaders() {
        const { startCol, endCol } = this.visibleRange;
        const width = this.canvas.width / this.dpr;

        // ヘッダー背景
        this.ctx.fillStyle = this.colors.headerBg;
        this.ctx.fillRect(this.headerWidth, 0, width - this.headerWidth, this.headerHeight);

        // ヘッダー下線
        this.ctx.strokeStyle = this.colors.headerBorder;
        this.ctx.lineWidth = 1;
        this.ctx.beginPath();
        this.ctx.moveTo(this.headerWidth, this.headerHeight + 0.5);
        this.ctx.lineTo(width, this.headerHeight + 0.5);
        this.ctx.stroke();

        // 列名
        this.ctx.fillStyle = this.colors.headerText;
        this.ctx.font = '12px -apple-system, BlinkMacSystemFont, sans-serif';
        this.ctx.textAlign = 'center';
        this.ctx.textBaseline = 'middle';

        let x = this.headerWidth - this.scrollX + this._getColumnX(startCol);
        for (let col = startCol; col <= endCol; col++) {
            const colWidth = this._getColumnWidth(col);
            const colName = CellReference.indexToCol(col);
            this.ctx.fillText(colName, x + colWidth / 2, this.headerHeight / 2);
            x += colWidth;
        }
    }

    /**
     * 行ヘッダー描画（1, 2, 3...）
     */
    _drawRowHeaders() {
        const { startRow, endRow } = this.visibleRange;
        const height = this.canvas.height / this.dpr;

        // ヘッダー背景
        this.ctx.fillStyle = this.colors.headerBg;
        this.ctx.fillRect(0, this.headerHeight, this.headerWidth, height - this.headerHeight);

        // ヘッダー右線
        this.ctx.strokeStyle = this.colors.headerBorder;
        this.ctx.lineWidth = 1;
        this.ctx.beginPath();
        this.ctx.moveTo(this.headerWidth + 0.5, this.headerHeight);
        this.ctx.lineTo(this.headerWidth + 0.5, height);
        this.ctx.stroke();

        // 行番号
        this.ctx.fillStyle = this.colors.headerText;
        this.ctx.font = '12px -apple-system, BlinkMacSystemFont, sans-serif';
        this.ctx.textAlign = 'center';
        this.ctx.textBaseline = 'middle';

        let y = this.headerHeight - this.scrollY + this._getRowY(startRow);
        for (let row = startRow; row <= endRow; row++) {
            const rowHeight = this._getRowHeight(row);
            this.ctx.fillText(String(row), this.headerWidth / 2, y + rowHeight / 2);
            y += rowHeight;
        }
    }

    /**
     * 左上コーナー描画
     */
    _drawCorner() {
        this.ctx.fillStyle = this.colors.headerBg;
        this.ctx.fillRect(0, 0, this.headerWidth, this.headerHeight);

        this.ctx.strokeStyle = this.colors.headerBorder;
        this.ctx.lineWidth = 1;
        this.ctx.strokeRect(0.5, 0.5, this.headerWidth - 1, this.headerHeight - 1);
    }

    /**
     * セル内容描画
     */
    _drawCells() {
        if (!this.engine) return;

        const sheet = this.engine.getActiveSheet();
        if (!sheet) return;

        const { startCol, endCol, startRow, endRow } = this.visibleRange;

        for (let row = startRow; row <= endRow; row++) {
            for (let col = startCol; col <= endCol; col++) {
                const cellRef = CellReference.indexToCol(col) + row;
                this._drawCell(cellRef, col, row);
            }
        }
    }

    /**
     * 個別セル描画
     * @param {string} cellRef
     * @param {number} colIndex
     * @param {number} rowIndex
     */
    _drawCell(cellRef, colIndex, rowIndex) {
        // 編集中のセルはスキップ
        if (this.isEditing && this.editingCell === cellRef) return;

        const sheet = this.engine.getActiveSheet();
        if (!sheet) return;

        const cell = sheet.cells.get(cellRef);
        const rect = this._getCellRect(colIndex, rowIndex);

        // セル背景
        if (cell?.format?.backgroundColor && cell.format.backgroundColor !== 'transparent') {
            this.ctx.fillStyle = cell.format.backgroundColor;
            this.ctx.fillRect(rect.x, rect.y, rect.width, rect.height);
        }

        // セル内容
        if (cell) {
            const displayValue = cell.error || (cell.computed !== null ? String(cell.computed) : cell.raw);

            if (displayValue) {
                const format = cell.format || {};

                // フォント設定
                let fontStyle = '';
                if (format.bold) fontStyle += 'bold ';
                if (format.italic) fontStyle += 'italic ';
                const fontSize = format.fontSize || 13;
                const fontFamily = format.fontFamily || '-apple-system, BlinkMacSystemFont, sans-serif';
                this.ctx.font = `${fontStyle}${fontSize}px ${fontFamily}`;

                // 色設定
                this.ctx.fillStyle = cell.error ? '#cc0000' : (format.textColor || this.colors.black);

                // テキスト配置
                const textAlign = format.textAlign || 'left';
                this.ctx.textAlign = textAlign;
                this.ctx.textBaseline = 'middle';

                // テキスト位置計算
                let textX;
                const padding = 4;
                switch (textAlign) {
                    case 'center':
                        textX = rect.x + rect.width / 2;
                        break;
                    case 'right':
                        textX = rect.x + rect.width - padding;
                        break;
                    default:
                        textX = rect.x + padding;
                }

                const textY = rect.y + rect.height / 2;

                // テキストクリッピング
                this.ctx.save();
                this.ctx.beginPath();
                this.ctx.rect(rect.x + 1, rect.y + 1, rect.width - 2, rect.height - 2);
                this.ctx.clip();

                this.ctx.fillText(displayValue, textX, textY);
                this.ctx.restore();
            }
        }
    }

    /**
     * 選択範囲描画
     */
    _drawSelection() {
        if (!this.engine) return;

        const sheet = this.engine.getActiveSheet();
        if (!sheet) return;

        const selection = sheet.selection;
        if (!selection) return;

        // 範囲選択
        if (selection.rangeStart && selection.rangeEnd) {
            const start = CellReference.parse(selection.rangeStart);
            const end = CellReference.parse(selection.rangeEnd);
            if (start && end) {
                const startCol = CellReference.colToIndex(start.col);
                const endCol = CellReference.colToIndex(end.col);
                const startRow = start.row;
                const endRow = end.row;

                const minCol = Math.min(startCol, endCol);
                const maxCol = Math.max(startCol, endCol);
                const minRow = Math.min(startRow, endRow);
                const maxRow = Math.max(startRow, endRow);

                const startRect = this._getCellRect(minCol, minRow);
                const endRect = this._getCellRect(maxCol, maxRow);

                const rangeRect = {
                    x: startRect.x,
                    y: startRect.y,
                    width: endRect.x + endRect.width - startRect.x,
                    height: endRect.y + endRect.height - startRect.y
                };

                // 範囲背景
                this.ctx.fillStyle = this.colors.selectionBg;
                this.ctx.fillRect(rangeRect.x, rangeRect.y, rangeRect.width, rangeRect.height);

                // 範囲枠線
                this.ctx.strokeStyle = this.colors.selectionBorder;
                this.ctx.lineWidth = 1;
                this.ctx.strokeRect(rangeRect.x + 0.5, rangeRect.y + 0.5, rangeRect.width - 1, rangeRect.height - 1);
            }
        }

        // アクティブセル
        if (selection.active) {
            const active = CellReference.parse(selection.active);
            if (active) {
                const colIndex = CellReference.colToIndex(active.col);
                const rect = this._getCellRect(colIndex, active.row);

                // アクティブセル枠線
                this.ctx.strokeStyle = this.colors.activeCellBorder;
                this.ctx.lineWidth = 2;
                this.ctx.strokeRect(rect.x + 1, rect.y + 1, rect.width - 2, rect.height - 2);
            }
        }
    }

    /**
     * 特定セルのみ再描画
     * @param {string} cellRef
     */
    renderCell(cellRef) {
        this.render(); // 簡易実装：全体再描画
    }

    // === 座標計算 ===

    /**
     * 列幅を取得
     * @param {number} colIndex
     * @returns {number}
     */
    _getColumnWidth(colIndex) {
        if (!this.engine) return this.defaultColWidth;
        const sheet = this.engine.getActiveSheet();
        if (!sheet) return this.defaultColWidth;
        const colRef = CellReference.indexToCol(colIndex);
        return sheet.columnWidths[colRef] ?? this.defaultColWidth;
    }

    /**
     * 行高を取得
     * @param {number} rowIndex
     * @returns {number}
     */
    _getRowHeight(rowIndex) {
        if (!this.engine) return this.defaultRowHeight;
        const sheet = this.engine.getActiveSheet();
        if (!sheet) return this.defaultRowHeight;
        return sheet.rowHeights[rowIndex] ?? this.defaultRowHeight;
    }

    /**
     * 列インデックスからX座標を取得（スクロール考慮なし）
     * @param {number} colIndex
     * @returns {number}
     */
    _getColumnX(colIndex) {
        let x = 0;
        for (let i = 0; i < colIndex; i++) {
            x += this._getColumnWidth(i);
        }
        return x;
    }

    /**
     * 行インデックスからY座標を取得（スクロール考慮なし）
     * @param {number} rowIndex
     * @returns {number}
     */
    _getRowY(rowIndex) {
        let y = 0;
        for (let i = 1; i < rowIndex; i++) {
            y += this._getRowHeight(i);
        }
        return y;
    }

    /**
     * セル参照から描画領域を取得
     * @param {number} colIndex
     * @param {number} rowIndex
     * @returns {{x: number, y: number, width: number, height: number}}
     */
    _getCellRect(colIndex, rowIndex) {
        const x = this.headerWidth + this._getColumnX(colIndex) - this.scrollX;
        const y = this.headerHeight + this._getRowY(rowIndex) - this.scrollY;
        const width = this._getColumnWidth(colIndex);
        const height = this._getRowHeight(rowIndex);

        return { x, y, width, height };
    }

    /**
     * セル参照文字列から描画領域を取得
     * @param {string} cellRef
     * @returns {{x: number, y: number, width: number, height: number}|null}
     */
    getCellRect(cellRef) {
        const parsed = CellReference.parse(cellRef);
        if (!parsed) return null;

        const colIndex = CellReference.colToIndex(parsed.col);
        return this._getCellRect(colIndex, parsed.row);
    }

    /**
     * ピクセル座標からセル参照を取得
     * @param {number} x
     * @param {number} y
     * @returns {{cellRef: string, area: string}|null}
     */
    getCellAtPosition(x, y) {
        // ヘッダー領域のチェック
        if (x < this.headerWidth && y < this.headerHeight) {
            return { cellRef: null, area: 'corner' };
        }
        if (x < this.headerWidth) {
            // 行ヘッダー
            const row = this._getRowAtY(y);
            return { cellRef: null, area: 'rowHeader', row };
        }
        if (y < this.headerHeight) {
            // 列ヘッダー
            const col = this._getColAtX(x);
            return { cellRef: null, area: 'colHeader', col };
        }

        // セル領域
        const col = this._getColAtX(x);
        const row = this._getRowAtY(y);

        if (col === null || row === null) return null;

        const cellRef = CellReference.indexToCol(col) + row;
        return { cellRef, area: 'cell', col, row };
    }

    /**
     * X座標から列インデックスを取得
     * @param {number} x
     * @returns {number|null}
     */
    _getColAtX(x) {
        let currentX = this.headerWidth - this.scrollX;
        let col = 0;

        while (currentX <= x && col < 1000) {
            currentX += this._getColumnWidth(col);
            if (currentX > x) return col;
            col++;
        }

        return null;
    }

    /**
     * Y座標から行インデックスを取得
     * @param {number} y
     * @returns {number|null}
     */
    _getRowAtY(y) {
        let currentY = this.headerHeight - this.scrollY;
        let row = 1;

        while (currentY <= y && row < 10000) {
            currentY += this._getRowHeight(row);
            if (currentY > y) return row;
            row++;
        }

        return null;
    }

    // === スクロール ===

    /**
     * セルが見えるようにスクロール
     * @param {string} cellRef
     */
    scrollToCell(cellRef) {
        const parsed = CellReference.parse(cellRef);
        if (!parsed) return;

        const colIndex = CellReference.colToIndex(parsed.col);
        const rect = this._getCellRect(colIndex, parsed.row);
        const viewWidth = this.canvas.width / this.dpr - this.headerWidth;
        const viewHeight = this.canvas.height / this.dpr - this.headerHeight;

        // 左に見切れている場合
        if (rect.x < this.headerWidth) {
            this.scrollX = this._getColumnX(colIndex);
        }
        // 右に見切れている場合
        if (rect.x + rect.width > this.headerWidth + viewWidth) {
            this.scrollX = this._getColumnX(colIndex) + rect.width - viewWidth;
        }
        // 上に見切れている場合
        if (rect.y < this.headerHeight) {
            this.scrollY = this._getRowY(parsed.row);
        }
        // 下に見切れている場合
        if (rect.y + rect.height > this.headerHeight + viewHeight) {
            this.scrollY = this._getRowY(parsed.row) + rect.height - viewHeight;
        }

        this.render();
    }

    /**
     * スクロール位置設定
     * @param {number} x
     * @param {number} y
     */
    setScroll(x, y) {
        this.scrollX = Math.max(0, x);
        this.scrollY = Math.max(0, y);
        this.render();
    }

    // === 選択 ===

    /**
     * セル選択
     * @param {string} cellRef
     * @param {boolean} extend - 範囲拡張するか（Shift+クリック）
     */
    selectCell(cellRef, extend = false) {
        if (!this.engine) return;

        const sheet = this.engine.getActiveSheet();
        if (!sheet) return;

        if (extend && sheet.selection?.active) {
            // 範囲選択拡張
            this.engine.setSelection(
                sheet.id,
                sheet.selection.active,
                sheet.selection.active,
                cellRef
            );
        } else {
            // 単一セル選択
            this.engine.setSelection(sheet.id, cellRef, null, null);
        }

        this.scrollToCell(cellRef);
    }

    /**
     * 範囲選択
     * @param {string} startRef
     * @param {string} endRef
     */
    selectRange(startRef, endRef) {
        if (!this.engine) return;

        const sheet = this.engine.getActiveSheet();
        if (!sheet) return;

        this.engine.setSelection(sheet.id, startRef, startRef, endRef);
    }

    /**
     * 行全体選択
     * @param {number} rowIndex
     */
    selectRow(rowIndex) {
        if (!this.engine) return;

        const sheet = this.engine.getActiveSheet();
        if (!sheet) return;

        // A列から最後のデータ列まで選択
        const startRef = 'A' + rowIndex;
        const endRef = 'Z' + rowIndex; // 簡易実装

        this.engine.setSelection(sheet.id, startRef, startRef, endRef);
    }

    /**
     * 列全体選択
     * @param {number} colIndex
     */
    selectColumn(colIndex) {
        if (!this.engine) return;

        const sheet = this.engine.getActiveSheet();
        if (!sheet) return;

        const colRef = CellReference.indexToCol(colIndex);
        const startRef = colRef + '1';
        const endRef = colRef + '1000'; // 簡易実装

        this.engine.setSelection(sheet.id, startRef, startRef, endRef);
    }

    /**
     * 全セル選択
     */
    selectAll() {
        if (!this.engine) return;

        const sheet = this.engine.getActiveSheet();
        if (!sheet) return;

        this.engine.setSelection(sheet.id, 'A1', 'A1', 'Z1000');
    }

    // === 編集 ===

    /**
     * セル編集開始
     * @param {string} cellRef
     * @param {string|null} initialValue - 初期値（キー入力で開始した場合）
     */
    startEditing(cellRef, initialValue = null) {
        if (!this.engine) return;

        const sheet = this.engine.getActiveSheet();
        if (!sheet) return;

        const parsed = CellReference.parse(cellRef);
        if (!parsed) return;

        this.isEditing = true;
        this.editingCell = cellRef;

        const colIndex = CellReference.colToIndex(parsed.col);
        const rect = this._getCellRect(colIndex, parsed.row);

        // エディタ位置設定
        this.editorElement.style.display = 'block';
        this.editorElement.style.left = rect.x + 'px';
        this.editorElement.style.top = rect.y + 'px';
        this.editorElement.style.width = rect.width + 'px';
        this.editorElement.style.height = rect.height + 'px';

        // 値設定
        if (initialValue !== null) {
            this.editorElement.value = initialValue;
        } else {
            const cell = sheet.cells.get(cellRef);
            this.editorElement.value = cell?.raw || '';
        }

        this.editorElement.focus();
        this.editorElement.select();

        this.render();
    }

    /**
     * 編集確定
     */
    commitEdit() {
        if (!this.isEditing || !this.editingCell || !this.engine) return;

        const value = this.editorElement.value;
        const sheet = this.engine.getActiveSheet();

        if (sheet) {
            this.engine.setCellValue(sheet.id, this.editingCell, value);
        }

        this._endEditing();
    }

    /**
     * 編集キャンセル
     */
    cancelEdit() {
        if (!this.isEditing) return;
        this._endEditing();
    }

    /**
     * 編集終了
     */
    _endEditing() {
        this.isEditing = false;
        this.editingCell = null;
        this.editorElement.style.display = 'none';
        this.editorElement.value = '';
        this.focus();
        this.render();
    }

    // === リサイズ ===

    /**
     * 列リサイズハンドル位置かチェック
     * @param {number} x
     * @param {number} y
     * @returns {{type: string, col: number}|null}
     */
    _getResizeHandle(x, y) {
        if (y > this.headerHeight) return null;
        if (x < this.headerWidth) return null;

        let currentX = this.headerWidth - this.scrollX;
        for (let col = 0; col < 100; col++) {
            currentX += this._getColumnWidth(col);
            if (Math.abs(x - currentX) < 5) {
                return { type: 'column', col };
            }
            if (currentX > x) break;
        }

        return null;
    }

    /**
     * 列幅リサイズ開始
     * @param {number} col
     * @param {MouseEvent} event
     */
    startColumnResize(col, event) {
        this.isResizingColumn = true;
        this.resizeTarget = col;
        this.resizeStartPos = event.clientX;
        this.resizeStartSize = this._getColumnWidth(col);
    }

    /**
     * 行高リサイズ開始
     * @param {number} rowIndex
     * @param {MouseEvent} event
     */
    startRowResize(rowIndex, event) {
        this.isResizingRow = true;
        this.resizeTarget = rowIndex;
        this.resizeStartPos = event.clientY;
        this.resizeStartSize = this._getRowHeight(rowIndex);
    }

    /**
     * 列幅自動調整
     * @param {number} colIndex
     */
    autoFitColumn(colIndex) {
        if (!this.engine) return;

        const sheet = this.engine.getActiveSheet();
        if (!sheet) return;

        const colRef = CellReference.indexToCol(colIndex);
        let maxWidth = 50;

        // 各行のセルを確認
        for (let row = 1; row <= 100; row++) {
            const cellRef = colRef + row;
            const cell = sheet.cells.get(cellRef);
            if (cell) {
                const value = cell.error || (cell.computed !== null ? String(cell.computed) : cell.raw);
                if (value) {
                    // テキスト幅を測定
                    const format = cell.format || {};
                    let fontStyle = '';
                    if (format.bold) fontStyle += 'bold ';
                    if (format.italic) fontStyle += 'italic ';
                    const fontSize = format.fontSize || 13;
                    const fontFamily = format.fontFamily || '-apple-system, BlinkMacSystemFont, sans-serif';
                    this.ctx.font = `${fontStyle}${fontSize}px ${fontFamily}`;

                    const metrics = this.ctx.measureText(value);
                    maxWidth = Math.max(maxWidth, metrics.width + 16);
                }
            }
        }

        this.engine.setColumnWidth(sheet.id, colRef, Math.min(maxWidth, 400));
    }

    // === イベントハンドラ ===

    _handleMouseDown(event) {
        const rect = this.canvas.getBoundingClientRect();
        const x = event.clientX - rect.left;
        const y = event.clientY - rect.top;

        // リサイズハンドルチェック
        const resizeHandle = this._getResizeHandle(x, y);
        if (resizeHandle) {
            this.startColumnResize(resizeHandle.col, event);
            return;
        }

        const hit = this.getCellAtPosition(x, y);
        if (!hit) return;

        if (hit.area === 'corner') {
            this.selectAll();
        } else if (hit.area === 'rowHeader') {
            this.selectRow(hit.row);
        } else if (hit.area === 'colHeader') {
            this.selectColumn(hit.col);
        } else if (hit.area === 'cell' && hit.cellRef) {
            if (this.isEditing && this.editingCell !== hit.cellRef) {
                this.commitEdit();
            }

            this.selectCell(hit.cellRef, event.shiftKey);
            this.isSelecting = true;
            this.selectionStart = hit.cellRef;
        }
    }

    _handleMouseMove(event) {
        const rect = this.canvas.getBoundingClientRect();
        const x = event.clientX - rect.left;
        const y = event.clientY - rect.top;

        // 列リサイズ中
        if (this.isResizingColumn) {
            const delta = event.clientX - this.resizeStartPos;
            const newWidth = Math.max(30, this.resizeStartSize + delta);
            const sheet = this.engine?.getActiveSheet();
            if (sheet) {
                const colRef = CellReference.indexToCol(this.resizeTarget);
                this.engine.setColumnWidth(sheet.id, colRef, newWidth);
            }
            return;
        }

        // 行リサイズ中
        if (this.isResizingRow) {
            const delta = event.clientY - this.resizeStartPos;
            const newHeight = Math.max(16, this.resizeStartSize + delta);
            const sheet = this.engine?.getActiveSheet();
            if (sheet) {
                this.engine.setRowHeight(sheet.id, this.resizeTarget, newHeight);
            }
            return;
        }

        // 範囲選択中
        if (this.isSelecting && this.selectionStart) {
            const hit = this.getCellAtPosition(x, y);
            if (hit?.area === 'cell' && hit.cellRef) {
                this.selectRange(this.selectionStart, hit.cellRef);
            }
            return;
        }

        // カーソル変更
        const resizeHandle = this._getResizeHandle(x, y);
        if (resizeHandle) {
            this.canvas.style.cursor = 'col-resize';
        } else {
            this.canvas.style.cursor = 'default';
        }
    }

    _handleMouseUp(event) {
        this.isSelecting = false;
        this.selectionStart = null;
        this.isResizingColumn = false;
        this.isResizingRow = false;
        this.resizeTarget = null;
    }

    _handleDoubleClick(event) {
        const rect = this.canvas.getBoundingClientRect();
        const x = event.clientX - rect.left;
        const y = event.clientY - rect.top;

        // 列ヘッダーのダブルクリックで自動幅調整
        const resizeHandle = this._getResizeHandle(x, y);
        if (resizeHandle) {
            this.autoFitColumn(resizeHandle.col);
            return;
        }

        const hit = this.getCellAtPosition(x, y);
        if (hit?.area === 'cell' && hit.cellRef) {
            this.startEditing(hit.cellRef);
        }
    }

    _handleKeyDown(event) {
        if (this.isEditing) return;

        const sheet = this.engine?.getActiveSheet();
        if (!sheet) return;

        const selection = sheet.selection;
        if (!selection?.active) return;

        const active = CellReference.parse(selection.active);
        if (!active) return;

        const colIndex = CellReference.colToIndex(active.col);

        let handled = true;
        let newCellRef = null;

        switch (event.key) {
            case 'ArrowUp':
                if (active.row > 1) {
                    newCellRef = active.col + (active.row - 1);
                }
                break;
            case 'ArrowDown':
                newCellRef = active.col + (active.row + 1);
                break;
            case 'ArrowLeft':
                if (colIndex > 0) {
                    newCellRef = CellReference.indexToCol(colIndex - 1) + active.row;
                }
                break;
            case 'ArrowRight':
                newCellRef = CellReference.indexToCol(colIndex + 1) + active.row;
                break;
            case 'Tab':
                if (event.shiftKey) {
                    if (colIndex > 0) {
                        newCellRef = CellReference.indexToCol(colIndex - 1) + active.row;
                    }
                } else {
                    newCellRef = CellReference.indexToCol(colIndex + 1) + active.row;
                }
                break;
            case 'Enter':
                if (event.shiftKey) {
                    if (active.row > 1) {
                        newCellRef = active.col + (active.row - 1);
                    }
                } else {
                    newCellRef = active.col + (active.row + 1);
                }
                break;
            case 'Home':
                if (event.ctrlKey || event.metaKey) {
                    newCellRef = 'A1';
                } else {
                    newCellRef = 'A' + active.row;
                }
                break;
            case 'Delete':
            case 'Backspace':
                this.engine.setCellValue(sheet.id, selection.active, '');
                break;
            case 'F2':
                this.startEditing(selection.active);
                break;
            case 'a':
                if (event.ctrlKey || event.metaKey) {
                    this.selectAll();
                } else {
                    handled = false;
                }
                break;
            case 'c':
                if (event.ctrlKey || event.metaKey) {
                    this._copyToClipboard();
                } else {
                    handled = false;
                }
                break;
            case 'v':
                if (event.ctrlKey || event.metaKey) {
                    this._pasteFromClipboard();
                } else {
                    handled = false;
                }
                break;
            case 'x':
                if (event.ctrlKey || event.metaKey) {
                    this._cutToClipboard();
                } else {
                    handled = false;
                }
                break;
            case 'z':
                if (event.ctrlKey || event.metaKey) {
                    if (event.shiftKey) {
                        this.engine.redo();
                    } else {
                        this.engine.undo();
                    }
                } else {
                    handled = false;
                }
                break;
            case 'y':
                if (event.ctrlKey || event.metaKey) {
                    this.engine.redo();
                } else {
                    handled = false;
                }
                break;
            case 'b':
                if (event.ctrlKey || event.metaKey) {
                    this._toggleBold();
                } else {
                    handled = false;
                }
                break;
            case 'i':
                if (event.ctrlKey || event.metaKey) {
                    this._toggleItalic();
                } else {
                    handled = false;
                }
                break;
            case 'Escape':
                // 選択解除
                break;
            default:
                handled = false;
        }

        if (newCellRef) {
            this.selectCell(newCellRef, event.shiftKey);
        }

        if (handled) {
            event.preventDefault();
            event.stopPropagation();
        } else if (event.key.length === 1 && !event.ctrlKey && !event.metaKey) {
            // 通常の文字入力で編集開始
            this.startEditing(selection.active, event.key);
            event.preventDefault();
        }
    }

    _handleEditorKeyDown(event) {
        // IME入力中は何もしない
        if (this.editorElement.dataset.composing === 'true') {
            return;
        }

        const sheet = this.engine?.getActiveSheet();
        if (!sheet) return;

        const selection = sheet.selection;
        if (!selection?.active) return;

        const active = CellReference.parse(selection.active);
        if (!active) return;

        const colIndex = CellReference.colToIndex(active.col);
        let nextCellRef = null;

        switch (event.key) {
            case 'Enter':
                this.commitEdit();
                if (event.shiftKey) {
                    if (active.row > 1) {
                        nextCellRef = active.col + (active.row - 1);
                    }
                } else {
                    nextCellRef = active.col + (active.row + 1);
                }
                event.preventDefault();
                break;
            case 'Tab':
                this.commitEdit();
                if (event.shiftKey) {
                    if (colIndex > 0) {
                        nextCellRef = CellReference.indexToCol(colIndex - 1) + active.row;
                    }
                } else {
                    nextCellRef = CellReference.indexToCol(colIndex + 1) + active.row;
                }
                event.preventDefault();
                break;
            case 'Escape':
                this.cancelEdit();
                event.preventDefault();
                break;
        }

        if (nextCellRef) {
            this.selectCell(nextCellRef);
        }
    }

    _handleWheel(event) {
        event.preventDefault();

        const deltaX = event.deltaX;
        const deltaY = event.deltaY;

        this.scrollX = Math.max(0, this.scrollX + deltaX);
        this.scrollY = Math.max(0, this.scrollY + deltaY);

        this.render();
    }

    _handleContextMenu(event) {
        event.preventDefault();

        // コンテキストメニューのカスタム実装（必要に応じて）
        this.dispatchEvent(new CustomEvent('contextmenu', {
            detail: {
                x: event.clientX,
                y: event.clientY,
                cellRef: this.getCellAtPosition(
                    event.clientX - this.canvas.getBoundingClientRect().left,
                    event.clientY - this.canvas.getBoundingClientRect().top
                )?.cellRef
            }
        }));
    }

    // === クリップボード操作 ===

    async _copyToClipboard() {
        if (!this.engine) return;

        const sheet = this.engine.getActiveSheet();
        if (!sheet) return;

        const selection = sheet.selection;
        if (!selection?.active) return;

        const rangeRef = selection.rangeStart && selection.rangeEnd
            ? `${selection.rangeStart}:${selection.rangeEnd}`
            : selection.active;

        this._clipboardData = this.engine.copy(sheet.id, rangeRef);

        // システムクリップボードにも書き込み
        const text = this._getSelectionText();
        if (text) {
            try {
                await navigator.clipboard.writeText(text);
            } catch (e) {
                console.warn('Failed to copy to system clipboard:', e);
            }
        }
    }

    async _cutToClipboard() {
        if (!this.engine) return;

        const sheet = this.engine.getActiveSheet();
        if (!sheet) return;

        const selection = sheet.selection;
        if (!selection?.active) return;

        const rangeRef = selection.rangeStart && selection.rangeEnd
            ? `${selection.rangeStart}:${selection.rangeEnd}`
            : selection.active;

        this._clipboardData = this.engine.cut(sheet.id, rangeRef);

        // 元のセルを削除
        const range = CellReference.parseRange(rangeRef);
        if (range) {
            for (const ref of CellReference.iterateRange(range)) {
                this.engine.setCellValue(sheet.id, ref, '');
            }
        }
    }

    async _pasteFromClipboard() {
        if (!this.engine) return;

        const sheet = this.engine.getActiveSheet();
        if (!sheet) return;

        const selection = sheet.selection;
        if (!selection?.active) return;

        if (this._clipboardData) {
            this.engine.paste(sheet.id, selection.active, this._clipboardData);
        } else {
            // システムクリップボードから読み込み
            try {
                const text = await navigator.clipboard.readText();
                if (text) {
                    this._pasteText(text);
                }
            } catch (e) {
                console.warn('Failed to read from system clipboard:', e);
            }
        }
    }

    _pasteText(text) {
        if (!this.engine) return;

        const sheet = this.engine.getActiveSheet();
        if (!sheet) return;

        const selection = sheet.selection;
        if (!selection?.active) return;

        const active = CellReference.parse(selection.active);
        if (!active) return;

        const lines = text.split('\n');
        let row = active.row;
        let col = CellReference.colToIndex(active.col);

        for (const line of lines) {
            const cells = line.split('\t');
            let currentCol = col;

            for (const cellValue of cells) {
                const cellRef = CellReference.indexToCol(currentCol) + row;
                this.engine.setCellValue(sheet.id, cellRef, cellValue);
                currentCol++;
            }

            row++;
        }
    }

    _getSelectionText() {
        if (!this.engine) return '';

        const sheet = this.engine.getActiveSheet();
        if (!sheet) return '';

        const selection = sheet.selection;
        if (!selection?.active) return '';

        const rangeRef = selection.rangeStart && selection.rangeEnd
            ? `${selection.rangeStart}:${selection.rangeEnd}`
            : selection.active;

        const range = CellReference.parseRange(rangeRef);
        if (!range) return '';

        const startCol = CellReference.colToIndex(range.start.col);
        const endCol = CellReference.colToIndex(range.end.col);
        const startRow = range.start.row;
        const endRow = range.end.row;

        const minCol = Math.min(startCol, endCol);
        const maxCol = Math.max(startCol, endCol);
        const minRow = Math.min(startRow, endRow);
        const maxRow = Math.max(startRow, endRow);

        const lines = [];
        for (let row = minRow; row <= maxRow; row++) {
            const cells = [];
            for (let col = minCol; col <= maxCol; col++) {
                const cellRef = CellReference.indexToCol(col) + row;
                const cell = sheet.cells.get(cellRef);
                const value = cell ? (cell.computed !== null ? String(cell.computed) : cell.raw) : '';
                cells.push(value);
            }
            lines.push(cells.join('\t'));
        }

        return lines.join('\n');
    }

    // === 書式操作 ===

    _toggleBold() {
        if (!this.engine) return;

        const sheet = this.engine.getActiveSheet();
        if (!sheet) return;

        const selection = sheet.selection;
        if (!selection?.active) return;

        const cell = sheet.cells.get(selection.active);
        const currentBold = cell?.format?.bold || false;

        this.engine.setCellFormat(sheet.id, selection.active, { bold: !currentBold });
    }

    _toggleItalic() {
        if (!this.engine) return;

        const sheet = this.engine.getActiveSheet();
        if (!sheet) return;

        const selection = sheet.selection;
        if (!selection?.active) return;

        const cell = sheet.cells.get(selection.active);
        const currentItalic = cell?.format?.italic || false;

        this.engine.setCellFormat(sheet.id, selection.active, { italic: !currentItalic });
    }
}

// カスタム要素登録
customElements.define('spreadsheet-grid', SpreadsheetGrid);

export default SpreadsheetGrid;
