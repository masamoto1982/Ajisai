/**
 * TensorView type definitions
 */

export interface TensorViewApp {
    engine: any;
    renderer: any;
    controls: any;
    container: HTMLElement | null;
    init(container: HTMLElement): void;
    setTensor(data: any): void;
    displayValue(value: any): void;
}

export function initTensorView(container: HTMLElement): TensorViewApp;
