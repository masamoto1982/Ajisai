/**
 * ColorMapper.js - 値から色への変換
 */

export class ColorMapper {
    constructor() {
        this.min = -1;
        this.max = 1;
        this.scheme = 'diverging';
    }

    setRange(data) {
        const flat = this.flatten(data);
        if (flat.length === 0) return;

        this.min = Math.min(...flat);
        this.max = Math.max(...flat);

        if (this.scheme === 'diverging') {
            const absMax = Math.max(Math.abs(this.min), Math.abs(this.max));
            this.min = -absMax;
            this.max = absMax;
        }
    }

    flatten(data) {
        if (!Array.isArray(data)) {
            return typeof data === 'number' ? [data] : [];
        }
        return data.flatMap(item => this.flatten(item));
    }

    getColor(value) {
        switch (this.scheme) {
            case 'diverging':
                return this.getDivergingColor(value);
            case 'sequential':
                return this.getSequentialColor(value);
            default:
                return this.getDivergingColor(value);
        }
    }

    getDivergingColor(value) {
        const t = this.normalize(value);

        if (t < 0.5) {
            const s = t * 2;
            const r = 220;
            const g = Math.floor(80 + s * 175);
            const b = Math.floor(80 + s * 175);
            return this.rgbToHex(r, g, b);
        } else {
            const s = (t - 0.5) * 2;
            const r = Math.floor(255 - s * 175);
            const g = Math.floor(255 - s * 125);
            const b = 220;
            return this.rgbToHex(r, g, b);
        }
    }

    getSequentialColor(value) {
        const t = this.normalize(value);

        const r = Math.floor(70 + t * 50);
        const g = Math.floor(50 + t * 180);
        const b = Math.floor(150 + t * 80);

        return this.rgbToHex(r, g, b);
    }

    normalize(value) {
        if (this.max === this.min) return 0.5;
        return (value - this.min) / (this.max - this.min);
    }

    rgbToHex(r, g, b) {
        return '#' + [r, g, b].map(x => {
            const hex = Math.max(0, Math.min(255, x)).toString(16);
            return hex.length === 1 ? '0' + hex : hex;
        }).join('');
    }

    setScheme(scheme) {
        this.scheme = scheme;
    }
}
