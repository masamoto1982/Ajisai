// js/gui/fp-utils.ts - 関数型ユーティリティ

/**
 * パイプライン: 左から右へ関数を適用
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
 * 関数合成: 右から左へ関数を合成
 * compose(f, g, h)(x) === f(g(h(x)))
 */
export function compose<A, B>(fn1: (a: A) => B): (a: A) => B;
export function compose<A, B, C>(fn2: (b: B) => C, fn1: (a: A) => B): (a: A) => C;
export function compose<A, B, C, D>(fn3: (c: C) => D, fn2: (b: B) => C, fn1: (a: A) => B): (a: A) => D;
export function compose(...fns: Array<(arg: unknown) => unknown>): (arg: unknown) => unknown {
    return (value: unknown) => fns.reduceRight((acc, fn) => fn(acc), value);
}

/**
 * 部分適用: 最初の引数を固定
 */
export const partial = <T, U extends unknown[], R>(
    fn: (first: T, ...rest: U) => R,
    first: T
): ((...rest: U) => R) => (...rest: U) => fn(first, ...rest);

/**
 * カリー化（2引数関数用）
 */
export const curry2 = <A, B, C>(fn: (a: A, b: B) => C) =>
    (a: A) => (b: B): C => fn(a, b);

/**
 * カリー化（3引数関数用）
 */
export const curry3 = <A, B, C, D>(fn: (a: A, b: B, c: C) => D) =>
    (a: A) => (b: B) => (c: C): D => fn(a, b, c);

/**
 * Identity関数
 */
export const identity = <T>(x: T): T => x;

/**
 * 定数関数
 */
export const constant = <T>(x: T) => (): T => x;

/**
 * Maybe型: null/undefinedを安全に扱う
 */
export type Maybe<T> = T | null | undefined;

export const isNothing = <T>(value: Maybe<T>): value is null | undefined =>
    value === null || value === undefined;

export const isJust = <T>(value: Maybe<T>): value is T =>
    !isNothing(value);

export const map = <T, U>(fn: (value: T) => U) =>
    (value: Maybe<T>): Maybe<U> =>
        isJust(value) ? fn(value) : value;

export const getOrElse = <T>(defaultValue: T) =>
    (value: Maybe<T>): T =>
        isJust(value) ? value : defaultValue;

/**
 * Result型: 成功/失敗を表現
 */
export type Result<T, E = Error> =
    | { ok: true; value: T }
    | { ok: false; error: E };

export const ok = <T>(value: T): Result<T, never> => ({ ok: true, value });
export const err = <E>(error: E): Result<never, E> => ({ ok: false, error });

export const mapResult = <T, U, E>(fn: (value: T) => U) =>
    (result: Result<T, E>): Result<U, E> =>
        result.ok ? ok(fn(result.value)) : result;

export const flatMapResult = <T, U, E>(fn: (value: T) => Result<U, E>) =>
    (result: Result<T, E>): Result<U, E> =>
        result.ok ? fn(result.value) : result;

/**
 * 副作用を分離するためのIO型（簡易版）
 */
export type IO<T> = () => T;

export const runIO = <T>(io: IO<T>): T => io();

export const mapIO = <T, U>(fn: (value: T) => U) =>
    (io: IO<T>): IO<U> =>
        () => fn(io());

export const flatMapIO = <T, U>(fn: (value: T) => IO<U>) =>
    (io: IO<T>): IO<U> =>
        () => fn(io())();

/**
 * 配列操作のユーティリティ
 */
export const head = <T>(arr: T[]): Maybe<T> => arr[0];
export const tail = <T>(arr: T[]): T[] => arr.slice(1);
export const last = <T>(arr: T[]): Maybe<T> => arr[arr.length - 1];
export const init = <T>(arr: T[]): T[] => arr.slice(0, -1);

/**
 * オブジェクト操作
 */
export const prop = <K extends string>(key: K) =>
    <T extends Record<K, unknown>>(obj: T): T[K] => obj[key];

export const pick = <K extends string>(...keys: K[]) =>
    <T extends Record<K, unknown>>(obj: T): Pick<T, K> =>
        keys.reduce((acc, key) => ({ ...acc, [key]: obj[key] }), {} as Pick<T, K>);

/**
 * DOM操作のユーティリティ（副作用を明示化）
 */
export const querySelector = (selector: string): IO<Maybe<Element>> =>
    () => document.querySelector(selector);

export const querySelectorAll = (selector: string): IO<NodeListOf<Element>> =>
    () => document.querySelectorAll(selector);

export const getElementById = (id: string): IO<Maybe<HTMLElement>> =>
    () => document.getElementById(id);

export const createElement = (tag: string): IO<HTMLElement> =>
    () => document.createElement(tag);

export const setTextContent = (text: string) =>
    (element: HTMLElement): IO<HTMLElement> =>
        () => { element.textContent = text; return element; };

export const setStyle = (styles: Partial<CSSStyleDeclaration>) =>
    (element: HTMLElement): IO<HTMLElement> =>
        () => { Object.assign(element.style, styles); return element; };

export const addClass = (className: string) =>
    (element: HTMLElement): IO<HTMLElement> =>
        () => { element.classList.add(className); return element; };

export const removeClass = (className: string) =>
    (element: HTMLElement): IO<HTMLElement> =>
        () => { element.classList.remove(className); return element; };

export const appendChild = (parent: HTMLElement) =>
    (child: HTMLElement): IO<HTMLElement> =>
        () => { parent.appendChild(child); return child; };

export const addEventListener = <K extends keyof HTMLElementEventMap>(
    event: K,
    handler: (e: HTMLElementEventMap[K]) => void
) => (element: HTMLElement): IO<HTMLElement> =>
    () => { element.addEventListener(event, handler); return element; };

/**
 * 非同期版Result
 */
export type AsyncResult<T, E = Error> = Promise<Result<T, E>>;

export const tryCatchAsync = async <T>(
    fn: () => Promise<T>
): AsyncResult<T, Error> => {
    try {
        return ok(await fn());
    } catch (e) {
        return err(e instanceof Error ? e : new Error(String(e)));
    }
};

/**
 * タップ: 副作用を実行しつつ値をそのまま返す（デバッグ用）
 */
export const tap = <T>(fn: (value: T) => void) =>
    (value: T): T => { fn(value); return value; };

/**
 * when: 条件に応じて関数を適用
 */
export const when = <T>(predicate: (value: T) => boolean, fn: (value: T) => T) =>
    (value: T): T => predicate(value) ? fn(value) : value;

/**
 * unless: 条件がfalseのときに関数を適用
 */
export const unless = <T>(predicate: (value: T) => boolean, fn: (value: T) => T) =>
    (value: T): T => predicate(value) ? value : fn(value);
