// js/gui/fp-utils.ts - Functional utilities

/**
 * Pipeline: Apply functions from left to right
 * pipe(x, f, g, h) === h(g(f(x)))
 */
export function pipe<A>(value: A): A;
export function pipe<A, B>(value: A, fn1: (a: A) => B): B;
export function pipe<A, B, C>(value: A, fn1: (a: A) => B, fn2: (b: B) => C): C;
export function pipe<A, B, C, D>(value: A, fn1: (a: A) => B, fn2: (b: B) => C, fn3: (c: C) => D): D;
export function pipe<A, B, C, D, E>(value: A, fn1: (a: A) => B, fn2: (b: B) => C, fn3: (c: C) => D, fn4: (d: D) => E): E;
export function pipe(value: unknown, ...fns: Array<(arg: unknown) => unknown>): unknown {
    return fns.reduce((acc, fn) => fn(acc), value);
}

/**
 * Result type: Represents success/failure
 */
export type Result<T, E = Error> =
    | { ok: true; value: T }
    | { ok: false; error: E };

export const ok = <T>(value: T): Result<T, never> => ({ ok: true, value });
export const err = <E>(error: E): Result<never, E> => ({ ok: false, error });
