export interface WindowPort {
    getInnerWidth(): number;
    getHardwareConcurrency(): number;
    addWindowEventListener<K extends keyof WindowEventMap>(
        type: K,
        listener: (event: WindowEventMap[K]) => void,
        options?: boolean | AddEventListenerOptions
    ): void;
    addDocumentEventListener<K extends keyof DocumentEventMap>(
        type: K,
        listener: (event: DocumentEventMap[K]) => void,
        options?: boolean | AddEventListenerOptions
    ): void;
    getBody(): HTMLElement;
}
