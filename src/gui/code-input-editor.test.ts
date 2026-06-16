import { describe, expect, it } from 'vitest';
import { computeAutoIndentInsertion } from './code-input-editor';

describe('computeAutoIndentInsertion', () => {
    it('preserves the current line indentation on newline', () => {
        const text = 'root\n    child';

        expect(computeAutoIndentInsertion(text, text.length)).toBe('\n    ');
    });

    it('increases indentation after an opening bracket', () => {
        const text = 'map [';

        expect(computeAutoIndentInsertion(text, text.length)).toBe('\n    ');
    });

    it('combines existing indentation with bracket continuation indentation', () => {
        const text = 'root\n\tcall (';

        expect(computeAutoIndentInsertion(text, text.length)).toBe('\n\t    ');
    });

    it('uses the line before the cursor when editing in the middle of the document', () => {
        const text = 'first\n  second\nthird';
        const cursor = 'first\n  second'.length;

        expect(computeAutoIndentInsertion(text, cursor)).toBe('\n  ');
    });
});
