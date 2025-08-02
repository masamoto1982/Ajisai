// js/types.ts

export const Types = {
    NUMBER: 'number',
    BOOLEAN: 'boolean',
    STRING: 'string',
    SYMBOL: 'symbol',
    VECTOR: 'vector',
    NIL: 'nil',
    QUOTATION: 'quotation'
} as const;

export type ValueType = typeof Types[keyof typeof Types];

export interface Fraction {
    numerator: number;
    denominator: number;
}

export interface Value {
    type: ValueType;
    value: Fraction | string | boolean | Value[] | null | { type: 'quotation', length: number };
}

export const createValue = (value: any, type: ValueType): Value => ({
    value,
    type
});
