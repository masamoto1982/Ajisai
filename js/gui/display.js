export class Display {
    constructor() {
        this.elements = {};
    }

    init(elements) {
        this.elements = elements;
    }

    showOutput(text) {
        this.elements.outputDisplay.textContent = text;
    }

    showError(error) {
        this.elements.outputDisplay.textContent = `Error: ${error.message || error}`;
    }

    showStepInfo(result) {
        if (result.output) {
            if (result.hasMore) {
                const position = result.position || 0;
                const total = result.total || 0;
                this.elements.outputDisplay.textContent = 
                    result.output + `\nStep ${position}/${total}: Press Ctrl+Enter to continue...`;
            } else {
                this.elements.outputDisplay.textContent = result.output || 'OK (Step execution completed)';
            }
        } else {
            if (result.hasMore) {
                const position = result.position || 0;
                const total = result.total || 0;
                this.elements.outputDisplay.textContent = 
                    `Step ${position}/${total}: Press Ctrl+Enter to continue...`;
            } else {
                this.elements.outputDisplay.textContent = 'OK (Step execution completed)';
            }
        }
    }

    updateStack(stack) {
        const display = this.elements.stackDisplay;
        display.innerHTML = '';
        
        if (!Array.isArray(stack) || stack.length === 0) {
            const emptySpan = document.createElement('span');
            emptySpan.textContent = '(empty)';
            emptySpan.style.color = '#ccc';
            display.appendChild(emptySpan);
            return;
        }
        
        const container = document.createElement('div');
        container.style.display = 'flex';
        container.style.flexWrap = 'wrap-reverse';
        container.style.justifyContent = 'flex-start';
        container.style.alignContent = 'flex-start';
        
        stack.forEach((item, index) => {
            const elem = document.createElement('span');
            elem.className = 'stack-item';
            elem.textContent = this.formatValue(item);
            
            if (index === stack.length - 1) {
                elem.style.fontWeight = 'bold';
                elem.style.opacity = '1';
            } else {
                elem.style.opacity = '0.7';
            }
            
            elem.style.margin = '2px 4px';
            elem.style.padding = '2px 6px';
            elem.style.backgroundColor = '#e0e0e0';
            elem.style.borderRadius = '3px';
            
            container.appendChild(elem);
        });
        
        display.appendChild(container);
    }

    updateRegister(value) {
        const display = this.elements.registerDisplay;
        display.innerHTML = '';
        
        if (value === null || value === undefined) {
            const emptySpan = document.createElement('span');
            emptySpan.textContent = '(empty)';
            emptySpan.style.color = '#ccc';
            display.appendChild(emptySpan);
        } else {
            display.textContent = this.formatValue(value);
        }
    }

    formatValue(item) {
        // 直接値を確認（WASMからの値の場合）
        if (!item) return 'undefined';
        
        // typeプロパティがある場合（新しい形式）
        if (item.type) {
            switch (item.type) {
                case 'number':
                    return typeof item.value === 'string' ? item.value : item.value.toString();
                case 'string':
                    return `"${item.value}"`;
                case 'symbol':
                    return item.value;
                case 'boolean':
                    return item.value ? 'true' : 'false';
                case 'vector':
                    if (Array.isArray(item.value)) {
                        const elements = item.value.map(v => this.formatValue(v)).join(' ');
                        return `[ ${elements} ]`;
                    }
                    return '[ ]';
                case 'nil':
                    return 'nil';
                case 'quotation':
                    return '{ ... }';
                default:
                    return JSON.stringify(item.value);
            }
        }
        
        // val_typeプロパティがある場合（古い形式）
        if (item.val_type) {
            const val = this.convertWasmValue(item);
            return this.formatValue(val);
        }
        
        // プリミティブ値の場合
        if (typeof item === 'number') return item.toString();
        if (typeof item === 'string') return `"${item}"`;
        if (typeof item === 'boolean') return item ? 'true' : 'false';
        if (item === null) return 'nil';
        
        return JSON.stringify(item);
    }

    convertWasmValue(wasmValue) {
        if (!wasmValue || wasmValue === null) return null;
        
        if (wasmValue.type === 'vector' && Array.isArray(wasmValue.value)) {
            return {
                type: 'vector',
                value: wasmValue.value.map(v => this.convertWasmValue(v))
            };
        }
        
        const typeMap = {
            'number': 'number',
            'string': 'string',
            'boolean': 'boolean',
            'symbol': 'symbol',
            'nil': 'nil',
            'quotation': 'quotation'
        };
        
        return {
            type: typeMap[wasmValue.type] || wasmValue.type,
            value: wasmValue.value
        };
    }
}
