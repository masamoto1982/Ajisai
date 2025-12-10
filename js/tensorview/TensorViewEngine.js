/**
 * TensorViewEngine.js - テンソルデータ管理
 */

export class TensorViewEngine extends EventTarget {
    constructor() {
        super();
        this.data = null;
        this.shape = [];
        this.rank = 0;
        this.sliceIndices = new Map();
        this.displayDimensions = [0, 1, 2];
    }

    setData(data) {
        this.data = data;
        this.shape = this.calculateShape(data);
        this.rank = this.shape.length;

        this.sliceIndices.clear();
        for (let i = 3; i < this.rank; i++) {
            this.sliceIndices.set(i, 0);
        }

        this.displayDimensions = [
            Math.min(0, this.rank - 1),
            Math.min(1, this.rank - 1),
            Math.min(2, this.rank - 1)
        ];

        this.notifyChange();
    }

    calculateShape(data, shape = []) {
        if (!Array.isArray(data)) {
            return shape;
        }
        shape.push(data.length);
        if (data.length > 0 && Array.isArray(data[0])) {
            return this.calculateShape(data[0], shape);
        }
        return shape;
    }

    setSlice(dimension, index) {
        if (dimension < 3 || dimension >= this.rank) return;

        const maxIndex = this.shape[dimension] - 1;
        this.sliceIndices.set(dimension, Math.max(0, Math.min(index, maxIndex)));
        this.notifyChange();
    }

    getSlicedData() {
        if (this.rank <= 3) {
            return this.padTo3D(this.data);
        }
        return this.sliceToThreeDimensions(this.data, 0);
    }

    sliceToThreeDimensions(data, currentDim) {
        if (currentDim >= this.rank - 3) {
            return this.padTo3D(data);
        }
        const sliceIndex = this.sliceIndices.get(currentDim) || 0;
        return this.sliceToThreeDimensions(data[sliceIndex], currentDim + 1);
    }

    padTo3D(data) {
        const shape = this.calculateShape(data);

        if (shape.length === 0) {
            return [[[data]]];
        }
        if (shape.length === 1) {
            return [[data]];
        }
        if (shape.length === 2) {
            return [data];
        }
        return data;
    }

    getValue(...indices) {
        let current = this.data;
        for (const idx of indices) {
            if (!Array.isArray(current) || idx >= current.length) {
                return null;
            }
            current = current[idx];
        }
        return current;
    }

    getStats() {
        const flat = this.flatten(this.data);
        if (flat.length === 0) return null;

        const min = Math.min(...flat);
        const max = Math.max(...flat);
        const sum = flat.reduce((a, b) => a + b, 0);
        const mean = sum / flat.length;

        return { min, max, mean, count: flat.length };
    }

    flatten(data) {
        if (!Array.isArray(data)) {
            return typeof data === 'number' ? [data] : [];
        }
        return data.flatMap(item => this.flatten(item));
    }

    notifyChange() {
        const slicedData = this.getSlicedData();
        const stats = this.getStats();

        this.dispatchEvent(new CustomEvent('dataChanged', {
            detail: {
                slicedData,
                shape: this.shape,
                stats,
                options: {
                    sliceIndices: Object.fromEntries(this.sliceIndices)
                }
            }
        }));
    }
}
