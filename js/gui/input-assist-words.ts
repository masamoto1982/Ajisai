import {
    createWordButtonElement,
    registerBackgroundClickListeners,
} from './dictionary-element-builders';

export interface InputAssistWordsCallbacks {
    readonly onAssistWordClick: (word: string) => void;
    readonly onBackgroundClick: () => void;
    readonly onBackgroundDoubleClick: () => void;
}

export interface InputAssistWords {
    readonly render: () => void;
}

const INPUT_ASSIST_WORDS: readonly string[] = Object.freeze([
    '1', '2', '3', '4', '5', '6', '7', '8', '9', '0',
    '[', ']', '{', '}', '(', ')',
    "'", '"', '+', '-', '*', '/', '~', '$', '%', '@', '=', '<', '>', '.', ',',
]);

export const createInputAssistWords = (
    container: HTMLElement,
    callbacks: InputAssistWordsCallbacks
): InputAssistWords => {
    const render = (): void => {
        container.innerHTML = '';

        for (const word of INPUT_ASSIST_WORDS) {
            const button = createWordButtonElement(
                word,
                `Insert ${word}`,
                'word-button input-assist-word',
                () => callbacks.onAssistWordClick(word)
            );

            container.appendChild(button);
        }
    };

    registerBackgroundClickListeners(
        container,
        callbacks.onBackgroundClick,
        callbacks.onBackgroundDoubleClick
    );

    return { render };
};
