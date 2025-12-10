/**
 * SpreadsheetToolbar.js - スプレッドシートのツールバー
 *
 * - 数式バー
 * - 書式設定ボタン
 * - シートタブ
 */

import { CellReference } from './CellReference.js';

export class SpreadsheetToolbar extends HTMLElement {
    constructor() {
        super();

        /** @type {import('./SpreadsheetEngine.js').SpreadsheetEngine|null} */
        this.engine = null;

        /** @type {import('./SpreadsheetGrid.js').SpreadsheetGrid|null} */
        this.grid = null;

        // DOM要素参照
        this.cellReferenceDisplay = null;
        this.formulaInput = null;
        this.formatToolbar = null;
        this.sheetTabs = null;

        // 状態
        this.isEditingFormula = false;
    }

    connectedCallback() {
        this.buildUI();
    }

    /**
     * エンジンとグリッドを接続
     * @param {import('./SpreadsheetEngine.js').SpreadsheetEngine} engine
     * @param {import('./SpreadsheetGrid.js').SpreadsheetGrid} grid
     */
    connect(engine, grid) {
        this.engine = engine;
        this.grid = grid;

        // イベントリスナー設定
        engine.addEventListener('selectionChanged', (e) => {
            this.updateForSelection(e.detail.selection?.active);
        });

        engine.addEventListener('cellChanged', (e) => {
            const sheet = engine.getActiveSheet();
            if (sheet?.selection?.active === e.detail.cellRef) {
                this.updateForSelection(e.detail.cellRef);
            }
        });

        engine.addEventListener('sheetAdded', () => this.updateSheetTabs());
        engine.addEventListener('sheetDeleted', () => this.updateSheetTabs());
        engine.addEventListener('sheetRenamed', () => this.updateSheetTabs());
        engine.addEventListener('activeSheetChanged', () => {
            this.updateSheetTabs();
            this.updateForSelection(engine.getActiveSheet()?.selection?.active);
        });

        // 初期更新
        this.updateSheetTabs();
        this.updateForSelection(engine.getActiveSheet()?.selection?.active);
    }

    /**
     * UIを構築
     */
    buildUI() {
        this.innerHTML = '';

        this.buildFormulaBar();
        this.buildFormatToolbar();
        this.buildSheetTabs();
    }

    /**
     * 数式バー部分を構築
     * [セル参照表示] [fx] [数式入力フィールド]
     */
    buildFormulaBar() {
        const formulaBar = document.createElement('div');
        formulaBar.className = 'formula-bar';

        // セル参照表示
        this.cellReferenceDisplay = document.createElement('div');
        this.cellReferenceDisplay.className = 'cell-reference';
        this.cellReferenceDisplay.textContent = 'A1';
        formulaBar.appendChild(this.cellReferenceDisplay);

        // fxラベル
        const fxLabel = document.createElement('div');
        fxLabel.className = 'fx-label';
        fxLabel.textContent = 'fx';
        formulaBar.appendChild(fxLabel);

        // 数式入力フィールド
        this.formulaInput = document.createElement('input');
        this.formulaInput.type = 'text';
        this.formulaInput.className = 'formula-input';
        this.formulaInput.placeholder = 'Enter a value or formula';

        this.formulaInput.addEventListener('focus', () => {
            this.isEditingFormula = true;
        });

        this.formulaInput.addEventListener('blur', () => {
            this.isEditingFormula = false;
            this.commitFormulaEdit();
        });

        this.formulaInput.addEventListener('keydown', (e) => {
            if (e.key === 'Enter') {
                this.commitFormulaEdit();
                this.grid?.focus();
                e.preventDefault();
            } else if (e.key === 'Escape') {
                this.cancelFormulaEdit();
                this.grid?.focus();
                e.preventDefault();
            }
        });

        formulaBar.appendChild(this.formulaInput);

        this.appendChild(formulaBar);
    }

    /**
     * 書式設定ツールバーを構築
     */
    buildFormatToolbar() {
        this.formatToolbar = document.createElement('div');
        this.formatToolbar.className = 'format-toolbar';

        // 太字ボタン
        this.formatToolbar.appendChild(this.createToolbarButton('bold', 'B', 'Bold (Ctrl+B)', () => this.toggleBold()));

        // 斜体ボタン
        this.formatToolbar.appendChild(this.createToolbarButton('italic', 'I', 'Italic (Ctrl+I)', () => this.toggleItalic()));

        this.formatToolbar.appendChild(this.createSeparator());

        // 配置ボタン
        this.formatToolbar.appendChild(this.createToolbarButton('align-left', '&#9664;', 'Align Left', () => this.setTextAlign('left')));
        this.formatToolbar.appendChild(this.createToolbarButton('align-center', '&#9632;', 'Align Center', () => this.setTextAlign('center')));
        this.formatToolbar.appendChild(this.createToolbarButton('align-right', '&#9654;', 'Align Right', () => this.setTextAlign('right')));

        this.formatToolbar.appendChild(this.createSeparator());

        // 背景色
        const bgColorPicker = this.createColorPicker('bg-color', 'Background Color', (color) => this.setBackgroundColor(color));
        this.formatToolbar.appendChild(bgColorPicker);

        // 文字色
        const textColorPicker = this.createColorPicker('text-color', 'Text Color', (color) => this.setTextColor(color));
        textColorPicker.style.color = '#000';
        this.formatToolbar.appendChild(textColorPicker);

        this.formatToolbar.appendChild(this.createSeparator());

        // Undo/Redo
        this.formatToolbar.appendChild(this.createToolbarButton('undo', '&#x21B6;', 'Undo (Ctrl+Z)', () => this.engine?.undo()));
        this.formatToolbar.appendChild(this.createToolbarButton('redo', '&#x21B7;', 'Redo (Ctrl+Y)', () => this.engine?.redo()));

        this.appendChild(this.formatToolbar);
    }

    /**
     * ツールバーボタン作成
     * @param {string} id
     * @param {string} label
     * @param {string} title
     * @param {Function} onClick
     * @returns {HTMLButtonElement}
     */
    createToolbarButton(id, label, title, onClick) {
        const button = document.createElement('button');
        button.className = 'toolbar-button';
        button.id = `toolbar-${id}`;
        button.innerHTML = label;
        button.title = title;
        button.addEventListener('click', (e) => {
            e.preventDefault();
            onClick();
        });
        return button;
    }

    /**
     * セパレーター作成
     * @returns {HTMLDivElement}
     */
    createSeparator() {
        const sep = document.createElement('div');
        sep.className = 'toolbar-separator';
        return sep;
    }

    /**
     * カラーピッカー作成
     * @param {string} id
     * @param {string} title
     * @param {Function} onChange
     * @returns {HTMLElement}
     */
    createColorPicker(id, title, onChange) {
        const wrapper = document.createElement('div');
        wrapper.className = 'color-picker-wrapper';
        wrapper.title = title;

        const button = document.createElement('button');
        button.className = 'toolbar-button color-picker-button';
        button.innerHTML = id === 'bg-color' ? '&#9632;' : 'A';

        const input = document.createElement('input');
        input.type = 'color';
        input.className = 'color-picker-input';
        input.value = id === 'bg-color' ? '#ffffff' : '#000000';

        input.addEventListener('input', (e) => {
            onChange(e.target.value);
        });

        button.addEventListener('click', () => {
            input.click();
        });

        wrapper.appendChild(button);
        wrapper.appendChild(input);

        return wrapper;
    }

    /**
     * シートタブバーを構築
     */
    buildSheetTabs() {
        this.sheetTabs = document.createElement('div');
        this.sheetTabs.className = 'sheet-tabs';

        // 新規シート追加ボタン
        const addButton = document.createElement('button');
        addButton.className = 'sheet-tab-add';
        addButton.textContent = '+';
        addButton.title = 'Add Sheet';
        addButton.addEventListener('click', () => {
            if (this.engine) {
                const sheet = this.engine.addSheet();
                this.engine.setActiveSheet(sheet.id);
            }
        });

        this.sheetTabs.appendChild(addButton);
        this.appendChild(this.sheetTabs);
    }

    /**
     * シートタブの更新
     */
    updateSheetTabs() {
        if (!this.engine || !this.sheetTabs) return;

        // 既存のタブを削除（追加ボタン以外）
        const addButton = this.sheetTabs.querySelector('.sheet-tab-add');
        this.sheetTabs.innerHTML = '';

        // シートタブを追加
        for (const sheet of this.engine.workbook?.sheets || []) {
            const tab = document.createElement('button');
            tab.className = 'sheet-tab';
            if (sheet.id === this.engine.workbook.activeSheetId) {
                tab.classList.add('active');
            }

            tab.textContent = sheet.name;
            tab.title = sheet.name;

            // クリックでシート切り替え
            tab.addEventListener('click', () => {
                this.engine.setActiveSheet(sheet.id);
            });

            // ダブルクリックでリネーム
            tab.addEventListener('dblclick', (e) => {
                e.preventDefault();
                this.startRenameSheet(sheet.id, tab);
            });

            // 右クリックでコンテキストメニュー
            tab.addEventListener('contextmenu', (e) => {
                e.preventDefault();
                this.showSheetContextMenu(sheet.id, e.clientX, e.clientY);
            });

            this.sheetTabs.appendChild(tab);
        }

        // 追加ボタンを最後に追加
        if (addButton) {
            this.sheetTabs.appendChild(addButton);
        }
    }

    /**
     * シートリネーム開始
     * @param {string} sheetId
     * @param {HTMLElement} tabElement
     */
    startRenameSheet(sheetId, tabElement) {
        const sheet = this.engine?.workbook?.sheets.find(s => s.id === sheetId);
        if (!sheet) return;

        const input = document.createElement('input');
        input.type = 'text';
        input.className = 'sheet-tab-input';
        input.value = sheet.name;
        input.style.width = '100px';

        const commitRename = () => {
            const newName = input.value.trim();
            if (newName && newName !== sheet.name) {
                this.engine.renameSheet(sheetId, newName);
            }
            this.updateSheetTabs();
        };

        input.addEventListener('blur', commitRename);
        input.addEventListener('keydown', (e) => {
            if (e.key === 'Enter') {
                commitRename();
            } else if (e.key === 'Escape') {
                this.updateSheetTabs();
            }
        });

        tabElement.innerHTML = '';
        tabElement.appendChild(input);
        input.focus();
        input.select();
    }

    /**
     * シートコンテキストメニュー表示
     * @param {string} sheetId
     * @param {number} x
     * @param {number} y
     */
    showSheetContextMenu(sheetId, x, y) {
        // 既存のメニューを削除
        const existingMenu = document.querySelector('.context-menu');
        if (existingMenu) {
            existingMenu.remove();
        }

        const menu = document.createElement('div');
        menu.className = 'context-menu';
        menu.style.left = x + 'px';
        menu.style.top = y + 'px';

        const items = [
            { label: 'Rename', action: () => {
                const tab = this.sheetTabs.querySelector(`.sheet-tab.active`);
                if (tab) this.startRenameSheet(sheetId, tab);
            }},
            { label: 'Duplicate', action: () => this.engine?.duplicateSheet(sheetId) },
            { label: 'Delete', action: () => {
                if (this.engine?.workbook?.sheets.length > 1) {
                    this.engine.deleteSheet(sheetId);
                }
            }, disabled: this.engine?.workbook?.sheets.length <= 1 }
        ];

        for (const item of items) {
            const menuItem = document.createElement('div');
            menuItem.className = 'context-menu-item';
            if (item.disabled) {
                menuItem.classList.add('disabled');
            }
            menuItem.textContent = item.label;
            if (!item.disabled) {
                menuItem.addEventListener('click', () => {
                    item.action();
                    menu.remove();
                });
            }
            menu.appendChild(menuItem);
        }

        document.body.appendChild(menu);

        // クリックでメニューを閉じる
        const closeMenu = (e) => {
            if (!menu.contains(e.target)) {
                menu.remove();
                document.removeEventListener('click', closeMenu);
            }
        };
        setTimeout(() => {
            document.addEventListener('click', closeMenu);
        }, 0);
    }

    /**
     * 選択変更時の更新
     * @param {string} cellRef
     */
    updateForSelection(cellRef) {
        if (!cellRef || !this.engine) {
            this.cellReferenceDisplay.textContent = '';
            this.formulaInput.value = '';
            return;
        }

        // セル参照表示
        this.cellReferenceDisplay.textContent = cellRef;

        // 数式/値表示
        const sheet = this.engine.getActiveSheet();
        if (sheet) {
            const cell = sheet.cells.get(cellRef);
            this.formulaInput.value = cell?.raw || '';

            // 書式ボタンの状態更新
            this.updateFormatButtons(cell?.format);
        }
    }

    /**
     * 書式ボタンの状態更新
     * @param {Object} format
     */
    updateFormatButtons(format) {
        const boldBtn = this.formatToolbar?.querySelector('#toolbar-bold');
        const italicBtn = this.formatToolbar?.querySelector('#toolbar-italic');

        if (boldBtn) {
            boldBtn.classList.toggle('active', format?.bold);
        }
        if (italicBtn) {
            italicBtn.classList.toggle('active', format?.italic);
        }
    }

    /**
     * 数式編集確定
     */
    commitFormulaEdit() {
        if (!this.engine) return;

        const sheet = this.engine.getActiveSheet();
        if (!sheet?.selection?.active) return;

        const value = this.formulaInput.value;
        this.engine.setCellValue(sheet.id, sheet.selection.active, value);
    }

    /**
     * 数式編集キャンセル
     */
    cancelFormulaEdit() {
        if (!this.engine) return;

        const sheet = this.engine.getActiveSheet();
        if (!sheet?.selection?.active) return;

        const cell = sheet.cells.get(sheet.selection.active);
        this.formulaInput.value = cell?.raw || '';
    }

    // === 書式設定アクション ===

    /**
     * 選択範囲を取得
     * @returns {string[]} セル参照の配列
     */
    _getSelectedCells() {
        if (!this.engine) return [];

        const sheet = this.engine.getActiveSheet();
        if (!sheet?.selection) return [];

        const { active, rangeStart, rangeEnd } = sheet.selection;

        if (rangeStart && rangeEnd) {
            const range = CellReference.parseRange(`${rangeStart}:${rangeEnd}`);
            if (range) {
                return [...CellReference.iterateRange(range)];
            }
        }

        return active ? [active] : [];
    }

    toggleBold() {
        if (!this.engine) return;

        const sheet = this.engine.getActiveSheet();
        if (!sheet?.selection?.active) return;

        const cells = this._getSelectedCells();
        if (cells.length === 0) return;

        // 最初のセルの状態を基準にトグル
        const firstCell = sheet.cells.get(cells[0]);
        const newBold = !firstCell?.format?.bold;

        for (const cellRef of cells) {
            this.engine.setCellFormat(sheet.id, cellRef, { bold: newBold });
        }

        this.grid?.render();
    }

    toggleItalic() {
        if (!this.engine) return;

        const sheet = this.engine.getActiveSheet();
        if (!sheet?.selection?.active) return;

        const cells = this._getSelectedCells();
        if (cells.length === 0) return;

        const firstCell = sheet.cells.get(cells[0]);
        const newItalic = !firstCell?.format?.italic;

        for (const cellRef of cells) {
            this.engine.setCellFormat(sheet.id, cellRef, { italic: newItalic });
        }

        this.grid?.render();
    }

    setTextAlign(align) {
        if (!this.engine) return;

        const sheet = this.engine.getActiveSheet();
        if (!sheet?.selection?.active) return;

        const cells = this._getSelectedCells();
        for (const cellRef of cells) {
            this.engine.setCellFormat(sheet.id, cellRef, { textAlign: align });
        }

        this.grid?.render();
    }

    setBackgroundColor(color) {
        if (!this.engine) return;

        const sheet = this.engine.getActiveSheet();
        if (!sheet?.selection?.active) return;

        const cells = this._getSelectedCells();
        for (const cellRef of cells) {
            this.engine.setCellFormat(sheet.id, cellRef, { backgroundColor: color });
        }

        this.grid?.render();
    }

    setTextColor(color) {
        if (!this.engine) return;

        const sheet = this.engine.getActiveSheet();
        if (!sheet?.selection?.active) return;

        const cells = this._getSelectedCells();
        for (const cellRef of cells) {
            this.engine.setCellFormat(sheet.id, cellRef, { textColor: color });
        }

        this.grid?.render();
    }

    setBorder(borderStyle) {
        if (!this.engine) return;

        const sheet = this.engine.getActiveSheet();
        if (!sheet?.selection?.active) return;

        const cells = this._getSelectedCells();
        for (const cellRef of cells) {
            this.engine.setCellFormat(sheet.id, cellRef, { borders: borderStyle });
        }

        this.grid?.render();
    }

    setNumberFormat(format) {
        if (!this.engine) return;

        const sheet = this.engine.getActiveSheet();
        if (!sheet?.selection?.active) return;

        const cells = this._getSelectedCells();
        for (const cellRef of cells) {
            this.engine.setCellFormat(sheet.id, cellRef, { numberFormat: format });
        }

        this.grid?.render();
    }

    mergeCells() {
        // TODO: セル結合機能
        console.log('Merge cells not implemented yet');
    }

    unmergeCells() {
        // TODO: セル結合解除機能
        console.log('Unmerge cells not implemented yet');
    }
}

customElements.define('spreadsheet-toolbar', SpreadsheetToolbar);

export default SpreadsheetToolbar;
