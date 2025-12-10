/**
 * CellReference.js - セル参照のパースと変換
 *
 * セル参照形式:
 * - 単純参照: 'A1', 'B2', 'AA100'
 * - 絶対参照: '$A$1', 'A$1', '$A1'
 * - シート参照: 'Sheet1!A1', 'Sheet1!$A$1'
 * - 範囲参照: 'A1:C3', 'Sheet1!A1:B2'
 */

/**
 * @typedef {Object} CellRef
 * @property {string|null} sheetName - シート名（省略時はnull）
 * @property {string} col - 列（'A','B'...）
 * @property {number} row - 行（1,2,3...）
 * @property {boolean} absoluteCol - $A形式か
 * @property {boolean} absoluteRow - A$1形式か
 */

/**
 * @typedef {Object} RangeRef
 * @property {string|null} sheetName - シート名
 * @property {CellRef} start - 開始セル
 * @property {CellRef} end - 終了セル
 */

export class CellReference {
    // セル参照の正規表現パターン
    static CELL_PATTERN = /^(\$?)([A-Z]+)(\$?)(\d+)$/i;
    static RANGE_PATTERN = /^(.+?):(.+)$/;
    static SHEET_PATTERN = /^(.+?)!(.+)$/;

    /**
     * 列名を列インデックス（0始まり）に変換
     * @param {string} col - 'A', 'B', ... 'Z', 'AA', 'AB'...
     * @returns {number}
     */
    static colToIndex(col) {
        col = col.toUpperCase();
        let index = 0;
        for (let i = 0; i < col.length; i++) {
            index = index * 26 + (col.charCodeAt(i) - 64);
        }
        return index - 1; // 0始まりに変換
    }

    /**
     * 列インデックス（0始まり）を列名に変換
     * @param {number} index - 0, 1, 2...
     * @returns {string}
     */
    static indexToCol(index) {
        let col = '';
        index += 1; // 1始まりに変換
        while (index > 0) {
            const remainder = (index - 1) % 26;
            col = String.fromCharCode(65 + remainder) + col;
            index = Math.floor((index - 1) / 26);
        }
        return col;
    }

    /**
     * セル参照文字列をパース
     * @param {string} ref - 'A1', '$A$1', 'Sheet1!A1'等
     * @returns {CellRef|null}
     */
    static parse(ref) {
        if (!ref || typeof ref !== 'string') {
            return null;
        }

        let sheetName = null;
        let cellPart = ref.trim();

        // シート名の抽出
        const sheetMatch = cellPart.match(this.SHEET_PATTERN);
        if (sheetMatch) {
            sheetName = sheetMatch[1];
            // シート名がシングルクォートで囲まれている場合
            if (sheetName.startsWith("'") && sheetName.endsWith("'")) {
                sheetName = sheetName.slice(1, -1);
            }
            cellPart = sheetMatch[2];
        }

        // セル参照のパース
        const cellMatch = cellPart.match(this.CELL_PATTERN);
        if (!cellMatch) {
            return null;
        }

        return {
            sheetName,
            col: cellMatch[2].toUpperCase(),
            row: parseInt(cellMatch[4], 10),
            absoluteCol: cellMatch[1] === '$',
            absoluteRow: cellMatch[3] === '$'
        };
    }

    /**
     * 範囲参照文字列をパース
     * @param {string} ref - 'A1:C3', 'Sheet1!A1:B2'等
     * @returns {RangeRef|null}
     */
    static parseRange(ref) {
        if (!ref || typeof ref !== 'string') {
            return null;
        }

        let sheetName = null;
        let rangePart = ref.trim();

        // シート名の抽出
        const sheetMatch = rangePart.match(this.SHEET_PATTERN);
        if (sheetMatch) {
            sheetName = sheetMatch[1];
            if (sheetName.startsWith("'") && sheetName.endsWith("'")) {
                sheetName = sheetName.slice(1, -1);
            }
            rangePart = sheetMatch[2];
        }

        // 範囲の分割
        const rangeMatch = rangePart.match(this.RANGE_PATTERN);
        if (!rangeMatch) {
            // 単一セルの場合は範囲として扱う
            const single = this.parse(ref);
            if (single) {
                return {
                    sheetName: single.sheetName,
                    start: single,
                    end: { ...single }
                };
            }
            return null;
        }

        const start = this.parse(rangeMatch[1]);
        const end = this.parse(rangeMatch[2]);

        if (!start || !end) {
            return null;
        }

        return {
            sheetName,
            start: { ...start, sheetName: null },
            end: { ...end, sheetName: null }
        };
    }

    /**
     * CellRefを文字列に変換
     * @param {CellRef} cellRef
     * @param {boolean} includeSheet - シート名を含めるか
     * @returns {string}
     */
    static toString(cellRef, includeSheet = true) {
        let result = '';

        if (includeSheet && cellRef.sheetName) {
            // シート名にスペースや特殊文字が含まれる場合はクォートで囲む
            if (/[^a-zA-Z0-9_]/.test(cellRef.sheetName)) {
                result += `'${cellRef.sheetName}'!`;
            } else {
                result += `${cellRef.sheetName}!`;
            }
        }

        if (cellRef.absoluteCol) result += '$';
        result += cellRef.col;
        if (cellRef.absoluteRow) result += '$';
        result += cellRef.row;

        return result;
    }

    /**
     * RangeRefを文字列に変換
     * @param {RangeRef} rangeRef
     * @param {boolean} includeSheet
     * @returns {string}
     */
    static rangeToString(rangeRef, includeSheet = true) {
        let result = '';

        if (includeSheet && rangeRef.sheetName) {
            if (/[^a-zA-Z0-9_]/.test(rangeRef.sheetName)) {
                result += `'${rangeRef.sheetName}'!`;
            } else {
                result += `${rangeRef.sheetName}!`;
            }
        }

        result += this.toString(rangeRef.start, false);
        result += ':';
        result += this.toString(rangeRef.end, false);

        return result;
    }

    /**
     * セル参照を移動（相対参照の場合）
     * @param {CellRef} cellRef
     * @param {number} deltaCol - 列方向の移動量
     * @param {number} deltaRow - 行方向の移動量
     * @returns {CellRef}
     */
    static offset(cellRef, deltaCol, deltaRow) {
        const newColIndex = cellRef.absoluteCol
            ? this.colToIndex(cellRef.col)
            : this.colToIndex(cellRef.col) + deltaCol;

        const newRow = cellRef.absoluteRow
            ? cellRef.row
            : cellRef.row + deltaRow;

        // 範囲チェック
        if (newColIndex < 0 || newRow < 1) {
            return null;
        }

        return {
            sheetName: cellRef.sheetName,
            col: this.indexToCol(newColIndex),
            row: newRow,
            absoluteCol: cellRef.absoluteCol,
            absoluteRow: cellRef.absoluteRow
        };
    }

    /**
     * 行・列の挿入/削除時にセル参照を更新
     * @param {CellRef} cellRef
     * @param {'row'|'col'} type - 挿入/削除の種類
     * @param {number} index - 挿入/削除位置
     * @param {number} count - 挿入/削除数（負の値は削除）
     * @param {string|null} targetSheet - 対象シート（nullの場合は全シート）
     * @returns {CellRef|null} - nullの場合は参照が無効になった
     */
    static updateForInsertDelete(cellRef, type, index, count, targetSheet = null) {
        // 対象シートのチェック
        if (targetSheet && cellRef.sheetName && cellRef.sheetName !== targetSheet) {
            return { ...cellRef };
        }

        if (type === 'row') {
            const row = cellRef.row;
            if (count > 0) {
                // 行挿入
                if (row >= index) {
                    return { ...cellRef, row: row + count };
                }
            } else {
                // 行削除
                const deleteCount = -count;
                if (row >= index && row < index + deleteCount) {
                    return null; // 削除された行への参照
                }
                if (row >= index + deleteCount) {
                    return { ...cellRef, row: row - deleteCount };
                }
            }
        } else if (type === 'col') {
            const colIndex = this.colToIndex(cellRef.col);
            if (count > 0) {
                // 列挿入
                if (colIndex >= index) {
                    return { ...cellRef, col: this.indexToCol(colIndex + count) };
                }
            } else {
                // 列削除
                const deleteCount = -count;
                if (colIndex >= index && colIndex < index + deleteCount) {
                    return null; // 削除された列への参照
                }
                if (colIndex >= index + deleteCount) {
                    return { ...cellRef, col: this.indexToCol(colIndex - deleteCount) };
                }
            }
        }

        return { ...cellRef };
    }

    /**
     * 範囲内のセル参照を列挙
     * @param {RangeRef} rangeRef
     * @yields {string} セル参照文字列（'A1'形式）
     */
    static *iterateRange(rangeRef) {
        const startCol = this.colToIndex(rangeRef.start.col);
        const endCol = this.colToIndex(rangeRef.end.col);
        const startRow = rangeRef.start.row;
        const endRow = rangeRef.end.row;

        const minCol = Math.min(startCol, endCol);
        const maxCol = Math.max(startCol, endCol);
        const minRow = Math.min(startRow, endRow);
        const maxRow = Math.max(startRow, endRow);

        for (let row = minRow; row <= maxRow; row++) {
            for (let col = minCol; col <= maxCol; col++) {
                yield this.indexToCol(col) + row;
            }
        }
    }

    /**
     * 範囲のサイズを取得
     * @param {RangeRef} rangeRef
     * @returns {{cols: number, rows: number}}
     */
    static getRangeSize(rangeRef) {
        const startCol = this.colToIndex(rangeRef.start.col);
        const endCol = this.colToIndex(rangeRef.end.col);
        const startRow = rangeRef.start.row;
        const endRow = rangeRef.end.row;

        return {
            cols: Math.abs(endCol - startCol) + 1,
            rows: Math.abs(endRow - startRow) + 1
        };
    }

    /**
     * セル参照が有効かどうかチェック
     * @param {string} ref
     * @returns {boolean}
     */
    static isValid(ref) {
        return this.parse(ref) !== null;
    }

    /**
     * 範囲参照が有効かどうかチェック
     * @param {string} ref
     * @returns {boolean}
     */
    static isValidRange(ref) {
        return this.parseRange(ref) !== null;
    }

    /**
     * セルが範囲内にあるかチェック
     * @param {string} cellRef - 'A1'形式
     * @param {RangeRef} rangeRef
     * @returns {boolean}
     */
    static isInRange(cellRef, rangeRef) {
        const cell = this.parse(cellRef);
        if (!cell) return false;

        const startCol = this.colToIndex(rangeRef.start.col);
        const endCol = this.colToIndex(rangeRef.end.col);
        const startRow = rangeRef.start.row;
        const endRow = rangeRef.end.row;

        const cellCol = this.colToIndex(cell.col);
        const cellRow = cell.row;

        const minCol = Math.min(startCol, endCol);
        const maxCol = Math.max(startCol, endCol);
        const minRow = Math.min(startRow, endRow);
        const maxRow = Math.max(startRow, endRow);

        return cellCol >= minCol && cellCol <= maxCol &&
               cellRow >= minRow && cellRow <= maxRow;
    }

    /**
     * 2つのセル参照が同じかチェック
     * @param {CellRef} a
     * @param {CellRef} b
     * @returns {boolean}
     */
    static equals(a, b) {
        if (!a || !b) return false;
        return a.col === b.col &&
               a.row === b.row &&
               a.sheetName === b.sheetName;
    }

    /**
     * セル参照をソート用に比較
     * @param {string} a - 'A1'形式
     * @param {string} b - 'A1'形式
     * @returns {number}
     */
    static compare(a, b) {
        const cellA = this.parse(a);
        const cellB = this.parse(b);

        if (!cellA || !cellB) return 0;

        const colDiff = this.colToIndex(cellA.col) - this.colToIndex(cellB.col);
        if (colDiff !== 0) return colDiff;

        return cellA.row - cellB.row;
    }

    /**
     * A1形式からR1C1形式に変換
     * @param {string} ref - 'A1'形式
     * @returns {string} - 'R1C1'形式
     */
    static toR1C1(ref) {
        const cell = this.parse(ref);
        if (!cell) return ref;

        return `R${cell.row}C${this.colToIndex(cell.col) + 1}`;
    }

    /**
     * R1C1形式からA1形式に変換
     * @param {string} ref - 'R1C1'形式
     * @returns {string} - 'A1'形式
     */
    static fromR1C1(ref) {
        const match = ref.match(/^R(\d+)C(\d+)$/i);
        if (!match) return ref;

        const row = parseInt(match[1], 10);
        const col = parseInt(match[2], 10);

        return this.indexToCol(col - 1) + row;
    }
}

export default CellReference;
