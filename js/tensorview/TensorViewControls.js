/**
 * TensorViewControls.js - スライス選択UIなど
 */

export class TensorViewControls extends EventTarget {
    constructor() {
        super();
        this.container = null;
        this.engine = null;
        this.sliders = new Map();
    }

    init(container, engine) {
        this.container = container;
        this.engine = engine;

        this.buildUI();

        engine.addEventListener('dataChanged', (e) => {
            this.updateSliders(e.detail.shape);
        });
    }

    buildUI() {
        this.container.innerHTML = `
            <div class="control-section">
                <h4>Dimensions</h4>
                <div id="slice-controls"></div>
            </div>
            <div class="control-section">
                <h4>Color Scheme</h4>
                <select id="color-scheme-select">
                    <option value="diverging">Diverging (Red-Blue)</option>
                    <option value="sequential">Sequential (Purple-Cyan)</option>
                </select>
            </div>
        `;

        const schemeSelect = this.container.querySelector('#color-scheme-select');
        schemeSelect.addEventListener('change', (e) => {
            this.dispatchEvent(new CustomEvent('colorSchemeChanged', {
                detail: { scheme: e.target.value }
            }));
        });
    }

    updateSliders(shape) {
        const sliceControls = this.container.querySelector('#slice-controls');
        sliceControls.innerHTML = '';

        if (shape.length <= 3) {
            sliceControls.innerHTML = '<p class="dim-info">3D or less - no slicing needed</p>';
            return;
        }

        for (let dim = 0; dim < shape.length - 3; dim++) {
            const maxIdx = shape[dim] - 1;
            const sliderDiv = document.createElement('div');
            sliderDiv.className = 'slice-slider';
            sliderDiv.innerHTML = `
                <label>Dim ${dim} (size: ${shape[dim]})</label>
                <input type="range" min="0" max="${maxIdx}" value="0" data-dim="${dim}">
                <span class="slider-value">0</span>
            `;

            const slider = sliderDiv.querySelector('input');
            const valueDisplay = sliderDiv.querySelector('.slider-value');

            slider.addEventListener('input', (e) => {
                const value = parseInt(e.target.value, 10);
                valueDisplay.textContent = value;
                this.dispatchEvent(new CustomEvent('sliceChanged', {
                    detail: { dimension: dim, index: value }
                }));
            });

            sliceControls.appendChild(sliderDiv);
            this.sliders.set(dim, slider);
        }
    }
}
