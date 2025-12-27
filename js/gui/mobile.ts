// js/gui/mobile.ts - モバイル対応（関数型スタイル）

// ============================================================
// 型定義
// ============================================================

export interface MobileElements {
    readonly inputArea: HTMLElement;
    readonly outputArea: HTMLElement;
    readonly stackArea: HTMLElement;
    readonly dictionaryArea: HTMLElement;
}

export type ViewMode = 'input' | 'execution';

export interface MobileHandler {
    readonly isMobile: () => boolean;
    readonly getCurrentMode: () => ViewMode;
    readonly updateView: (mode: ViewMode) => void;
}

// ============================================================
// 定数
// ============================================================

const MOBILE_BREAKPOINT = 768;
const SWIPE_THRESHOLD = 50;

// ============================================================
// 純粋関数
// ============================================================

/**
 * 現在のウィンドウ幅がモバイルかどうかを判定
 */
const checkIsMobile = (): boolean => window.innerWidth <= MOBILE_BREAKPOINT;

/**
 * スワイプの方向を判定
 */
const detectSwipeDirection = (
    startX: number,
    startY: number,
    endX: number,
    endY: number
): 'left' | 'right' | 'up' | 'down' | null => {
    const deltaX = endX - startX;
    const deltaY = endY - startY;

    // 横方向のスワイプが縦方向より大きい場合のみ
    if (Math.abs(deltaX) > Math.abs(deltaY) && Math.abs(deltaX) > SWIPE_THRESHOLD) {
        return deltaX > 0 ? 'right' : 'left';
    }

    if (Math.abs(deltaY) > Math.abs(deltaX) && Math.abs(deltaY) > SWIPE_THRESHOLD) {
        return deltaY > 0 ? 'down' : 'up';
    }

    return null;
};

/**
 * モードを切り替える（トグル）
 */
const toggleMode = (currentMode: ViewMode): ViewMode =>
    currentMode === 'input' ? 'execution' : 'input';

/**
 * 入力モード時の表示設定を生成
 */
const getInputModeStyles = (): Record<keyof MobileElements, string> => ({
    inputArea: 'block',
    outputArea: 'none',
    stackArea: 'none',
    dictionaryArea: 'block'
});

/**
 * 実行モード時の表示設定を生成
 */
const getExecutionModeStyles = (): Record<keyof MobileElements, string> => ({
    inputArea: 'none',
    outputArea: 'block',
    stackArea: 'block',
    dictionaryArea: 'none'
});

/**
 * モードに応じた表示設定を取得
 */
const getStylesForMode = (mode: ViewMode): Record<keyof MobileElements, string> =>
    mode === 'input' ? getInputModeStyles() : getExecutionModeStyles();

// ============================================================
// 副作用関数: DOM操作
// ============================================================

const setElementDisplay = (element: HTMLElement, display: string): void => {
    element.style.display = display;
};

const resetElementDisplay = (element: HTMLElement): void => {
    element.style.display = '';
};

const applyMobileStyles = (
    elements: MobileElements,
    styles: Record<keyof MobileElements, string>
): void => {
    (Object.keys(styles) as Array<keyof MobileElements>).forEach(key => {
        setElementDisplay(elements[key], styles[key]);
    });
};

const resetAllStyles = (elements: MobileElements): void => {
    Object.values(elements).forEach(el => {
        if (el?.style) {
            resetElementDisplay(el);
        }
    });
};

// ============================================================
// ファクトリ関数: MobileHandler作成
// ============================================================

export const createMobileHandler = (elements: MobileElements): MobileHandler => {
    // 状態（クロージャで保持）
    let currentMode: ViewMode = 'input';
    let touchStartX = 0;
    let touchStartY = 0;

    // スワイプジェスチャーのハンドラ
    const handleSwipeGesture = (endX: number, endY: number): void => {
        const direction = detectSwipeDirection(touchStartX, touchStartY, endX, endY);

        if (direction === 'left' || direction === 'right') {
            const newMode = toggleMode(currentMode);
            updateView(newMode);
        }
    };

    // スワイプジェスチャーの設定
    const setupSwipeGestures = (): void => {
        if (!checkIsMobile()) return;

        const container = document.body;

        container.addEventListener('touchstart', (e: TouchEvent) => {
            const touch = e.changedTouches[0];
            if (touch) {
                touchStartX = touch.screenX;
                touchStartY = touch.screenY;
            }
        }, { passive: true });

        container.addEventListener('touchend', (e: TouchEvent) => {
            const touch = e.changedTouches[0];
            if (touch) {
                handleSwipeGesture(touch.screenX, touch.screenY);
            }
        }, { passive: true });
    };

    // 初期化
    setupSwipeGestures();

    // モバイル判定
    const isMobile = (): boolean => checkIsMobile();

    // 現在のモード取得
    const getCurrentMode = (): ViewMode => currentMode;

    // ビュー更新
    const updateView = (mode: ViewMode): void => {
        if (!checkIsMobile()) {
            // デスクトップモードでは全て表示
            resetAllStyles(elements);
            return;
        }

        currentMode = mode;
        const styles = getStylesForMode(mode);
        applyMobileStyles(elements, styles);
    };

    return {
        isMobile,
        getCurrentMode,
        updateView
    };
};

// 純粋関数をエクスポート（テスト用）
export const mobileUtils = {
    checkIsMobile,
    detectSwipeDirection,
    toggleMode,
    getStylesForMode,
    MOBILE_BREAKPOINT,
    SWIPE_THRESHOLD
};
