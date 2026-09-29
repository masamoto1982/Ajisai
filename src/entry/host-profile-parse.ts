import type { HostProfile } from '../wasm-interpreter-types';

/**
 * The host profile the interpreter reports, or `null` when the text is not
 * one.
 *
 * Every field the display reads is checked here, not just that the text is
 * JSON: the profile only ever feeds a label, and a label is never worth
 * aborting the application over. Without this, a `limits` that is missing or
 * carries a non-number would throw from `Object.entries` or
 * `toLocaleString()` in the caller — past the parse guard, and into the
 * startup handler that abandons GUI initialization.
 */
export function parseHostProfile(json: string): HostProfile | null {
    let parsed: unknown;
    try {
        parsed = JSON.parse(json);
    } catch {
        return null;
    }
    if (typeof parsed !== 'object' || parsed === null) return null;
    const { profile, limits } = parsed as { profile?: unknown; limits?: unknown };
    if (typeof profile !== 'string') return null;
    if (typeof limits !== 'object' || limits === null || Array.isArray(limits)) return null;
    for (const value of Object.values(limits)) {
        if (typeof value !== 'number' || !Number.isFinite(value)) return null;
    }
    return { profile, limits: limits as Record<string, number> };
}
