/**
 * tensorview/index.js - テンソルビューア統合エントリポイント
 */

import { TensorViewEngine } from './TensorViewEngine.js';
import { IsometricRenderer } from './IsometricRenderer.js';
import { TensorViewControls } from './TensorViewControls.js';

export class TensorViewApp {
    constructor() {
        this.engine = null;
        this.renderer = null;
        this.controls = null;
        this.container = null;
    }

    init(container) {
        if (!container) {
            console.error('[TensorView] Container not found');
            return;
        }

        console.log('[TensorView] Initializing tensor viewer...');

        this.container = container;
        this.engine = new TensorViewEngine();
        this.renderer = new IsometricRenderer();
        this.controls = new TensorViewControls();

        this.createUI();
        this.setupEvents();

        console.log('[TensorView] Tensor viewer initialized');
    }

    createUI() {
        this.container.innerHTML = '';
        this.container.classList.add('tensor-view-container');

        const layout = document.createElement('div');
        layout.className = 'tensor-view-layout';

        // キャンバスエリア
        const canvasArea = document.createElement('div');
        canvasArea.className = 'tensor-canvas-area';
        this.renderer.init(canvasArea);
        layout.appendChild(canvasArea);

        // コントロールパネル
        const controlPanel = document.createElement('div');
        controlPanel.className = 'tensor-control-panel';
        this.controls.init(controlPanel, this.engine);
        layout.appendChild(controlPanel);

        // 情報表示
        const infoPanel = document.createElement('div');
        infoPanel.className = 'tensor-info-panel';
        infoPanel.innerHTML = `
            <div class="tensor-shape">Shape: <span id="tensor-shape-display">-</span></div>
            <div class="tensor-hover">Hover: <span id="tensor-hover-display">-</span></div>
        `;
        layout.appendChild(infoPanel);

        this.container.appendChild(layout);
    }

    setupEvents() {
        this.engine.addEventListener('dataChanged', (e) => {
            this.renderer.render(e.detail.slicedData, e.detail.options);
            this.updateShapeDisplay();
        });

        this.controls.addEventListener('sliceChanged', (e) => {
            this.engine.setSlice(e.detail.dimension, e.detail.index);
        });

        this.controls.addEventListener('colorSchemeChanged', (e) => {
            this.renderer.colorMapper.setScheme(e.detail.scheme);
            if (this.engine.data) {
                this.engine.notifyChange();
            }
        });

        this.renderer.addEventListener('cellHover', (e) => {
            this.updateHoverDisplay(e.detail);
        });
    }

    setTensor(data) {
        this.engine.setData(data);
    }

    displayValue(value) {
        const data = this.convertAjisaiValue(value);
        if (data) {
            this.setTensor(data);
        }
    }

    convertAjisaiValue(value) {
        if (!value) return null;

        if (value.type === 'number') {
            return [[[parseFloat(value.value.numerator) / parseFloat(value.value.denominator)]]];
        }

        if (value.type === 'vector') {
            return this.convertVector(value.value);
        }

        return null;
    }

    convertVector(vec) {
        if (!Array.isArray(vec)) return vec;

        return vec.map(item => {
            if (item.type === 'number') {
                return parseFloat(item.value.numerator) / parseFloat(item.value.denominator);
            }
            if (item.type === 'vector') {
                return this.convertVector(item.value);
            }
            return 0;
        });
    }

    updateShapeDisplay() {
        const el = document.getElementById('tensor-shape-display');
        if (el) {
            el.textContent = this.engine.shape.join(' × ');
        }
    }

    updateHoverDisplay(detail) {
        const el = document.getElementById('tensor-hover-display');
        if (el) {
            if (detail) {
                el.textContent = `[${detail.indices.join(', ')}] = ${detail.value.toFixed(4)}`;
            } else {
                el.textContent = '-';
            }
        }
    }
}

export function initTensorView(container) {
    const app = new TensorViewApp();
    app.init(container);
    window.ajisaiTensorView = app;
    return app;
}
