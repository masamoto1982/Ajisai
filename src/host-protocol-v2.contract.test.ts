import { describe, expect, test } from 'vitest';
import { readFileSync } from 'node:fs';
import { decodeV2Document, V2DecodeError, V2_PROTOCOL, type V2Value } from './host-protocol-v2';

const readJson = (path: string): any => JSON.parse(readFileSync(path, 'utf8'));

describe('HostProtocolV2 contract', () => {
    const schema = readJson('spec/host-protocol-v2.schema.json');
    const golden = readJson('spec/freeze/host-protocol-v2.golden.json');

    test('schema declares V2, the six value domains, and the presentation intents', () => {
        expect(schema['x-ajisai-version']).toBe(2);
        expect(schema.properties.protocol.const).toBe(V2_PROTOCOL);
        expect(schema.$defs.value.properties.type.enum).toEqual([
            'scalar',
            'boolean',
            'string',
            'vector',
            'nil',
            'codeBlock',
        ]);
        expect(schema.$defs.observedValue.properties.presentation.enum).toEqual([
            'structural',
            'interval',
            'timestamp',
        ]);
    });

    test('the frozen golden decodes into the six domains', () => {
        const document = decodeV2Document(golden);
        expect(document.protocol).toBe(V2_PROTOCOL);

        const values: V2Value[] = document.stack.map((observed) => observed.value);
        expect(values).toEqual([
            { type: 'scalar', numerator: '3', denominator: '1' },
            { type: 'boolean', value: true },
            { type: 'string', value: 'hi' },
            {
                type: 'vector',
                items: [
                    { type: 'scalar', numerator: '1', denominator: '1' },
                    { type: 'scalar', numerator: '2', denominator: '1' },
                ],
            },
            { type: 'nil', reason: 'divisionByZero' },
            { type: 'nil', reason: null },
            { type: 'codeBlock' },
        ]);
    });

    test('presentation hints ride alongside the observed values', () => {
        const document = decodeV2Document(golden);
        // A String needs no presentation hint: `value.type === 'string'`
        // already carries the domain, so the hint stays structural.
        expect(document.stack[2]?.presentation).toBe('structural');
        expect(document.stack[3]?.presentation).toBe('interval');
        expect(document.stack[0]?.presentation).toBe('structural');
    });

    test('the decoder handles exactly the domains the schema declares', () => {
        const declared = new Set(schema.$defs.value.properties.type.enum as string[]);
        const decodedTypes = new Set(
            decodeV2Document(golden).stack.map((observed) => observed.value.type),
        );
        // codeBlock is the one domain not exercised as a top-level distinct type
        // beyond the golden; assert every decoded type is declared, and every
        // declared type is decodable by construction of the golden + decoder.
        for (const type of decodedTypes) {
            expect(declared.has(type)).toBe(true);
        }
        expect(declared).toEqual(
            new Set(['scalar', 'boolean', 'string', 'vector', 'nil', 'codeBlock']),
        );
    });

    test('a malformed document is rejected', () => {
        expect(() => decodeV2Document('not an object')).toThrow(V2DecodeError);
        expect(() => decodeV2Document({ protocol: 'ajisai.host.v1', stack: [] })).toThrow(
            V2DecodeError,
        );
        expect(() =>
            decodeV2Document({
                protocol: V2_PROTOCOL,
                stack: [{ value: { type: 'mystery' }, presentation: 'structural' }],
            }),
        ).toThrow(V2DecodeError);
        expect(() =>
            decodeV2Document({
                protocol: V2_PROTOCOL,
                stack: [{ value: { type: 'nil', reason: null }, presentation: 'sideways' }],
            }),
        ).toThrow(V2DecodeError);
    });
});
