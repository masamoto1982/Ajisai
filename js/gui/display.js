// js/gui/display.js

export class Display {
    init(elements) {
        this.elements = elements;
        this.mainOutput = '';
    }

    // メインの実行結果を表示
    showOutput(text) {
        this.mainOutput = text;
        this.elements.outputDisplay.textContent = this.mainOutput;
    }

    // エラーメッセージを表示
    showError(error) {
        const errorMessage = `Error: ${error.message || error}`;
        this.mainOutput = errorMessage;
        this.elements.outputDisplay.textContent = this.mainOutput;
    }

    // 補足情報（ステップ実行、保存通知など）を表示
    showInfo(text, append = false) {
        if (append && this.mainOutput) {
            this.elements.outputDisplay.textContent = `${this.mainOutput}\n${text}`;
        } else {
            this.elements.outputDisplay.textContent = text;
        }
    }

    updateStack(stack) {
        const display = this.elements.stackDisplay;
        display.innerHTML = '';
        
        if (!Array.isArray(stack) || stack.length === 0) {
            display.textContent = '(empty)';
            display.style.color = '#ccc';
            return;
        }
        
        display.style.color = '#333';
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
        if (value === null || value === undefined) {
            display.textContent = '(empty)';
            display.style.color = '#ccc';
        } else {
            display.style.color = '#333';
            display.textContent = this.formatValue(value);
        }
    }

    formatValue(item) {
    if (!item) return 'undefined';
    
    switch (item.type) {
        case 'number':
            // 修正箇所：分数オブジェクトを正しく処理する
            if (typeof item.value === 'object' && item.value !== null && 'numerator' in item.value && 'denominator' in item.value) {
                if (item.value.denominator === 1) {
                    return item.value.numerator.toString();
                } else {
                    return `${item.value.numerator}/${item.value.denominator}`;
                }
            }
            // 従来の数値や文字列形式の数値も念のため残す
            return typeof item.value === 'string' ? item.value : item.value.toString();
        case 'string':
            return `"${item.value}"`;
        case 'symbol':
            return item.value;
        case 'boolean':
            return item.value ? 'true' : 'false';
        case 'vector':
            return `[ ${item.value.map(v => this.formatValue(v)).join(' ')} ]`;
        case 'nil':
            return 'nil';
        case 'quotation':
            return '{ ... }';
        default:
            return JSON.stringify(item.value);
    }
}
}
