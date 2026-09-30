// parseHostProfile: the shape guard between `host_profile()` and the labels
// that display it. Every rejected row below is a text that JSON.parse accepts
// and the display code would otherwise throw on.
//
// DUT: src/entry/host-profile-parse.ts

import { describe, expect, it } from 'vitest';
import { parseHostProfile } from './host-profile-parse';

describe('parseHostProfile', () => {
    it('accepts the shape the interpreter reports', () => {
        const text = JSON.stringify({
            profile: 'browser-playground',
            limits: { maxExecutionSteps: 1_000_000, materializedElements: 1_000_000 },
        });
        expect(parseHostProfile(text)).toEqual({
            profile: 'browser-playground',
            limits: { maxExecutionSteps: 1_000_000, materializedElements: 1_000_000 },
        });
    });

    it('accepts an empty ceiling table', () => {
        expect(parseHostProfile('{"profile":"p","limits":{}}')).toEqual({ profile: 'p', limits: {} });
    });

    it('rejects text that is not JSON', () => {
        expect(parseHostProfile('')).toBeNull();
        expect(parseHostProfile('not json')).toBeNull();
    });

    it('rejects JSON that is not an object', () => {
        expect(parseHostProfile('null')).toBeNull();
        expect(parseHostProfile('42')).toBeNull();
        expect(parseHostProfile('"browser-playground"')).toBeNull();
        expect(parseHostProfile('[]')).toBeNull();
    });

    it('rejects a missing or non-string profile name', () => {
        expect(parseHostProfile('{"limits":{}}')).toBeNull();
        expect(parseHostProfile('{"profile":1,"limits":{}}')).toBeNull();
    });

    it('rejects a missing, null, array or non-object limits table', () => {
        expect(parseHostProfile('{"profile":"p"}')).toBeNull();
        expect(parseHostProfile('{"profile":"p","limits":null}')).toBeNull();
        expect(parseHostProfile('{"profile":"p","limits":[1]}')).toBeNull();
        expect(parseHostProfile('{"profile":"p","limits":"5"}')).toBeNull();
    });

    it('rejects a ceiling that is not a finite number', () => {
        expect(parseHostProfile('{"profile":"p","limits":{"a":"5"}}')).toBeNull();
        expect(parseHostProfile('{"profile":"p","limits":{"a":null}}')).toBeNull();
        expect(parseHostProfile('{"profile":"p","limits":{"a":1e999}}')).toBeNull();
    });
});
