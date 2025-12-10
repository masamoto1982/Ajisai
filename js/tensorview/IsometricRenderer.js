/**
 * IsometricRenderer.js - アイソメトリック投影による3D描画
 */

import { ColorMapper } from './ColorMapper.js';

export class IsometricRenderer extends EventTarget {
    constructor() {
        super();
        this.canvas = null;
        this.ctx = null;
        this.container = null;

        this.angle = Math.PI / 6;
        this.cosA = Math.cos(this.angle);
        this.sinA = Math.sin(this.angle);

        this.cubeSize = 30;
        this.offsetX = 0;
        this.offsetY = 0;

        this.colorMapper = new ColorMapper();
        this.hoveredCell = null;
        this.currentData = null;
        this.dpr = window.devicePixelRatio || 1;

        this._handleMouseMove = this._handleMouseMove.bind(this);
        this._handleMouseLeave = this._handleMouseLeave.bind(this);
        this._handleResize = this._handleResize.bind(this);
    }

    init(container) {
        this.container = container;

        this.canvas = document.createElement('canvas');
        this.canvas.className = 'isometric-canvas';
        this.ctx = this.canvas.getContext('2d');

        container.appendChild(this.canvas);

        this._attachEvents();
        this._resize();

        this.resizeObserver = new ResizeObserver(() => this._resize());
        this.resizeObserver.observe(container);
    }

    _attachEvents() {
        this.canvas.addEventListener('mousemove', this._handleMouseMove);
        this.canvas.addEventListener('mouseleave', this._handleMouseLeave);
        window.addEventListener('resize', this._handleResize);
    }

    _resize() {
        const rect = this.container.getBoundingClientRect();
        const width = rect.width || 400;
        const height = rect.height || 300;

        this.canvas.width = width * this.dpr;
        this.canvas.height = height * this.dpr;
        this.canvas.style.width = width + 'px';
        this.canvas.style.height = height + 'px';

        this.ctx.setTransform(1, 0, 0, 1, 0, 0);
        this.ctx.scale(this.dpr, this.dpr);

        this.offsetX = width / 2;
        this.offsetY = height / 2;

        if (this.currentData) {
            this.render(this.currentData);
        }
    }

    _handleResize() {
        this._resize();
    }

    render(data, options = {}) {
        this.currentData = data;

        const width = this.canvas.width / this.dpr;
        const height = this.canvas.height / this.dpr;

        this.ctx.fillStyle = '#1a1a2e';
        this.ctx.fillRect(0, 0, width, height);

        if (!data || !data.length) return;

        const dimX = data.length;
        const dimY = data[0]?.length || 1;
        const dimZ = data[0]?.[0]?.length || 1;

        this.colorMapper.setRange(data);
        this.autoScale(dimX, dimY, dimZ);
        this.drawGrid(dimX, dimY, dimZ);
        this.drawAxes(dimX, dimY, dimZ);

        for (let x = dimX - 1; x >= 0; x--) {
            for (let y = dimY - 1; y >= 0; y--) {
                for (let z = 0; z < dimZ; z++) {
                    const value = data[x]?.[y]?.[z];
                    if (typeof value === 'number') {
                        const isHovered = this.hoveredCell &&
                            this.hoveredCell.x === x &&
                            this.hoveredCell.y === y &&
                            this.hoveredCell.z === z;
                        this.drawCube(x, y, z, value, isHovered);
                    }
                }
            }
        }
    }

    autoScale(dimX, dimY, dimZ) {
        const width = this.canvas.width / this.dpr;
        const height = this.canvas.height / this.dpr;

        const maxDim = Math.max(dimX, dimY, dimZ);
        const targetSize = Math.min(width, height) * 0.6;

        this.cubeSize = Math.max(10, Math.min(40, targetSize / maxDim));
    }

    toScreen(x, y, z) {
        const isoX = (x - y) * this.cubeSize * this.cosA;
        const isoY = (x + y) * this.cubeSize * this.sinA - z * this.cubeSize;

        return {
            x: this.offsetX + isoX,
            y: this.offsetY + isoY
        };
    }

    drawCube(x, y, z, value, isHovered = false) {
        const baseColor = this.colorMapper.getColor(value);

        const pts = {
            topFront: this.toScreen(x, y, z + 1),
            topRight: this.toScreen(x + 1, y, z + 1),
            topBack: this.toScreen(x + 1, y + 1, z + 1),
            topLeft: this.toScreen(x, y + 1, z + 1),
            botFront: this.toScreen(x, y, z),
            botRight: this.toScreen(x + 1, y, z),
            botBack: this.toScreen(x + 1, y + 1, z),
            botLeft: this.toScreen(x, y + 1, z)
        };

        const topColor = this.adjustBrightness(baseColor, 1.2);
        const leftColor = this.adjustBrightness(baseColor, 0.8);
        const rightColor = this.adjustBrightness(baseColor, 1.0);

        this.drawFace([pts.topFront, pts.topRight, pts.topBack, pts.topLeft], topColor, isHovered);
        this.drawFace([pts.topFront, pts.topLeft, pts.botLeft, pts.botFront], leftColor, isHovered);
        this.drawFace([pts.topFront, pts.botFront, pts.botRight, pts.topRight], rightColor, isHovered);

        if (isHovered) {
            this.ctx.strokeStyle = '#ffffff';
            this.ctx.lineWidth = 2;
            this.drawFaceOutline([pts.topFront, pts.topRight, pts.topBack, pts.topLeft]);
        }
    }

    drawFace(points, color, isHovered) {
        this.ctx.beginPath();
        this.ctx.moveTo(points[0].x, points[0].y);
        for (let i = 1; i < points.length; i++) {
            this.ctx.lineTo(points[i].x, points[i].y);
        }
        this.ctx.closePath();

        this.ctx.fillStyle = color;
        this.ctx.fill();

        this.ctx.strokeStyle = 'rgba(255,255,255,0.2)';
        this.ctx.lineWidth = 0.5;
        this.ctx.stroke();
    }

    drawFaceOutline(points) {
        this.ctx.beginPath();
        this.ctx.moveTo(points[0].x, points[0].y);
        for (let i = 1; i < points.length; i++) {
            this.ctx.lineTo(points[i].x, points[i].y);
        }
        this.ctx.closePath();
        this.ctx.stroke();
    }

    adjustBrightness(hexColor, factor) {
        const r = parseInt(hexColor.slice(1, 3), 16);
        const g = parseInt(hexColor.slice(3, 5), 16);
        const b = parseInt(hexColor.slice(5, 7), 16);

        const newR = Math.min(255, Math.floor(r * factor));
        const newG = Math.min(255, Math.floor(g * factor));
        const newB = Math.min(255, Math.floor(b * factor));

        return `rgb(${newR}, ${newG}, ${newB})`;
    }

    drawGrid(dimX, dimY, dimZ) {
        this.ctx.strokeStyle = 'rgba(255,255,255,0.1)';
        this.ctx.lineWidth = 1;

        for (let x = 0; x <= dimX; x++) {
            const start = this.toScreen(x, 0, 0);
            const end = this.toScreen(x, dimY, 0);
            this.ctx.beginPath();
            this.ctx.moveTo(start.x, start.y);
            this.ctx.lineTo(end.x, end.y);
            this.ctx.stroke();
        }

        for (let y = 0; y <= dimY; y++) {
            const start = this.toScreen(0, y, 0);
            const end = this.toScreen(dimX, y, 0);
            this.ctx.beginPath();
            this.ctx.moveTo(start.x, start.y);
            this.ctx.lineTo(end.x, end.y);
            this.ctx.stroke();
        }
    }

    drawAxes(dimX, dimY, dimZ) {
        this.ctx.font = '12px monospace';
        this.ctx.fillStyle = '#888';

        const xEnd = this.toScreen(dimX + 1, 0, 0);
        this.ctx.fillText('X', xEnd.x, xEnd.y);

        const yEnd = this.toScreen(0, dimY + 1, 0);
        this.ctx.fillText('Y', yEnd.x, yEnd.y);

        const zEnd = this.toScreen(0, 0, dimZ + 1);
        this.ctx.fillText('Z', zEnd.x, zEnd.y);
    }

    _handleMouseMove(e) {
        const rect = this.canvas.getBoundingClientRect();
        const mouseX = e.clientX - rect.left;
        const mouseY = e.clientY - rect.top;

        const hit = this.hitTest(mouseX, mouseY);

        if (hit) {
            if (!this.hoveredCell ||
                this.hoveredCell.x !== hit.x ||
                this.hoveredCell.y !== hit.y ||
                this.hoveredCell.z !== hit.z) {

                this.hoveredCell = hit;
                this.render(this.currentData);

                this.dispatchEvent(new CustomEvent('cellHover', {
                    detail: {
                        indices: [hit.x, hit.y, hit.z],
                        value: hit.value
                    }
                }));
            }
        } else if (this.hoveredCell) {
            this.hoveredCell = null;
            this.render(this.currentData);
            this.dispatchEvent(new CustomEvent('cellHover', { detail: null }));
        }
    }

    _handleMouseLeave() {
        if (this.hoveredCell) {
            this.hoveredCell = null;
            this.render(this.currentData);
            this.dispatchEvent(new CustomEvent('cellHover', { detail: null }));
        }
    }

    hitTest(mouseX, mouseY) {
        if (!this.currentData) return null;

        const dimX = this.currentData.length;
        const dimY = this.currentData[0]?.length || 1;
        const dimZ = this.currentData[0]?.[0]?.length || 1;

        for (let x = 0; x < dimX; x++) {
            for (let y = 0; y < dimY; y++) {
                for (let z = dimZ - 1; z >= 0; z--) {
                    if (this.isPointInCube(mouseX, mouseY, x, y, z)) {
                        return {
                            x, y, z,
                            value: this.currentData[x][y][z]
                        };
                    }
                }
            }
        }

        return null;
    }

    isPointInCube(mouseX, mouseY, x, y, z) {
        const pts = [
            this.toScreen(x, y, z + 1),
            this.toScreen(x + 1, y, z + 1),
            this.toScreen(x + 1, y + 1, z + 1),
            this.toScreen(x, y + 1, z + 1)
        ];

        return this.isPointInPolygon(mouseX, mouseY, pts);
    }

    isPointInPolygon(x, y, polygon) {
        let inside = false;
        for (let i = 0, j = polygon.length - 1; i < polygon.length; j = i++) {
            const xi = polygon[i].x, yi = polygon[i].y;
            const xj = polygon[j].x, yj = polygon[j].y;

            if (((yi > y) !== (yj > y)) &&
                (x < (xj - xi) * (y - yi) / (yj - yi) + xi)) {
                inside = !inside;
            }
        }
        return inside;
    }

    dispose() {
        this.canvas.removeEventListener('mousemove', this._handleMouseMove);
        this.canvas.removeEventListener('mouseleave', this._handleMouseLeave);
        window.removeEventListener('resize', this._handleResize);
        if (this.resizeObserver) {
            this.resizeObserver.disconnect();
        }
    }
}
