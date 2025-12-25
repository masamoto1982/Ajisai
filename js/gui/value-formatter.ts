// js/gui/value-formatter.ts - 値のフォーマットと比較

import type { Value, Fraction } from '../wasm-types';

// フォーマット設定
const SCIENTIFIC_THRESHOLD = 10; // 10桁以上で科学的記数法
const MANTISSA_PRECISION = 6;    // 仮数部の精度

/**
 * 整数を科学的記数法でフォーマット
 */
export function formatIntegerScientific(numStr: string): string {
    const isNegative = numStr.startsWith('-');
    const absNumStr = isNegative ? numStr.substring(1) : numStr;

    if (absNumStr.length < SCIENTIFIC_THRESHOLD) {
        return numStr;
    }

    const firstDigit = absNumStr[0];
    const remainingDigits = absNumStr.substring(1);
    const exponent = remainingDigits.length;

    let mantissa = firstDigit!;
    if (remainingDigits.length > 0) {
        const fractionalDigits = Math.min(MANTISSA_PRECISION - 1, remainingDigits.length);
        if (fractionalDigits > 0) {
            mantissa += '.' + remainingDigits.substring(0, fractionalDigits);
        }
    }

    mantissa = mantissa.replace(/\.?0+$/, '');
    if (isNegative) mantissa = '-' + mantissa;

    return `${mantissa}e${exponent}`;
}

/**
 * 分数を科学的記数法でフォーマット
 */
export function formatFractionScientific(numerStr: string, denomStr: string): string {
    if (denomStr === '1') {
        return formatIntegerScientific(numerStr);
    }

    const numSci = formatIntegerScientific(numerStr);
    const denSci = formatIntegerScientific(denomStr);

    if (numSci.includes('e') && denSci.includes('e')) {
        const numMatch = numSci.match(/^([+-]?\d+\.?\d*)e([+-]?\d+)$/);
        const denMatch = denSci.match(/^([+-]?\d+\.?\d*)e([+-]?\d+)$/);

        if (numMatch && denMatch) {
            const numMantissa = parseFloat(numMatch[1]!);
            const numExponent = parseInt(numMatch[2]!);
            const denMantissa = parseFloat(denMatch[1]!);
            const denExponent = parseInt(denMatch[2]!);

            let resultMantissa = numMantissa / denMantissa;
            let resultExponent = numExponent - denExponent;

            while (Math.abs(resultMantissa) >= 10) {
                resultMantissa /= 10;
                resultExponent += 1;
            }
            while (Math.abs(resultMantissa) < 1 && resultMantissa !== 0) {
                resultMantissa *= 10;
                resultExponent -= 1;
            }

            const rounded = resultMantissa.toPrecision(MANTISSA_PRECISION);
            return resultExponent === 0 ? rounded : `${rounded}e${resultExponent}`;
        }
    }

    return `${numSci}/${denSci}`;
}

/**
 * 分数をフォーマット（シンプル版）
 */
export function formatFraction(frac: Fraction): string {
    const denomStr = String(frac.denominator);
    const numerStr = String(frac.numerator);
    return formatFractionScientific(numerStr, denomStr);
}

/**
 * 値をフォーマット（テスト用シンプル版）
 */
export function formatValueSimple(value: Value): string {
    switch (value.type) {
        case 'number': {
            const frac = value.value as Fraction;
            if (frac.denominator === '1') {
                return frac.numerator;
            }
            return `${frac.numerator}/${frac.denominator}`;
        }
        case 'string':
            return `'${value.value}'`;
        case 'boolean':
            return value.value ? 'TRUE' : 'FALSE';
        case 'nil':
            return 'NIL';
        case 'vector':
            if (Array.isArray(value.value)) {
                const elements = value.value.map(v => formatValueSimple(v)).join(' ');
                return `[${elements ? ' ' + elements + ' ' : ''}]`;
            }
            return '[]';
        default:
            return JSON.stringify(value.value);
    }
}

/**
 * スタックをフォーマット（テスト用）
 */
export function formatStack(stack: Value[]): string {
    if (stack.length === 0) {
        return '[]';
    }
    const formatted = stack.map(v => formatValueSimple(v)).join(', ');
    return `[${formatted}]`;
}

/**
 * 値を比較
 */
export function compareValue(actual: Value, expected: Value): boolean {
    if (actual.type !== expected.type) {
        return false;
    }

    switch (actual.type) {
        case 'number': {
            const actualFrac = actual.value as Fraction;
            const expectedFrac = expected.value as Fraction;
            return actualFrac.numerator === expectedFrac.numerator &&
                   actualFrac.denominator === expectedFrac.denominator;
        }
        case 'vector':
            if (!Array.isArray(actual.value) || !Array.isArray(expected.value)) {
                return false;
            }
            return compareStack(actual.value, expected.value);
        case 'string':
        case 'boolean':
            return JSON.stringify(actual.value) === JSON.stringify(expected.value);
        case 'nil':
            return true;
        default:
            return JSON.stringify(actual.value) === JSON.stringify(expected.value);
    }
}

/**
 * スタックを比較
 */
export function compareStack(actual: Value[], expected: Value[]): boolean {
    if (actual.length !== expected.length) {
        return false;
    }

    for (let i = 0; i < actual.length; i++) {
        const a = actual[i];
        const e = expected[i];
        if (!a || !e || !compareValue(a, e)) {
            return false;
        }
    }

    return true;
}
