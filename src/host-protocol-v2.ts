// HostProtocolV2 consumer (migration plan §15, Phase 8).
//
// The front-end read path for the Semantic Spine wire format. V2 mirrors the
// six canonical value domains and the outward-observable stack, and carries no
// legacy concept (module catalog, tensor shape, logical-unknown axis, absence
// origin/recoverability, capability set). The producer is the Rust serializer
// `kernel::host_protocol_v2`; this module decodes what it emits.
//
// This is the sole current host document decoder. Consumers must reject older protocol tags.

/** The display intent carried alongside an observed value; never changes semantics. */
export type V2Presentation = 'structural' | 'interval' | 'timestamp';

/** One of the six canonical value domains, discriminated by `type`. */
export type V2Value =
    | { readonly type: 'scalar'; readonly numerator: string; readonly denominator: string }
    | { readonly type: 'scalar'; readonly exact: true }
    | { readonly type: 'boolean'; readonly value: boolean }
    | { readonly type: 'string'; readonly value: string }
    | { readonly type: 'vector'; readonly items: readonly V2Value[] }
    | { readonly type: 'nil'; readonly reason: string | null }
    | { readonly type: 'codeBlock' };

export interface V2ObservedValue {
    readonly value: V2Value;
    readonly presentation: V2Presentation;
}

export interface V2Document {
    readonly protocol: typeof V2_PROTOCOL;
    readonly stack: readonly V2ObservedValue[];
}

/** The protocol identifier every V2 document carries. */
export const V2_PROTOCOL = 'ajisai.host.v2';

const PRESENTATIONS: readonly V2Presentation[] = ['structural', 'interval', 'timestamp'];

/** Thrown when a value is not a well-formed HostProtocolV2 document. */
export class V2DecodeError extends Error {
    constructor(message: string) {
        super(message);
        this.name = 'V2DecodeError';
    }
}

const isObject = (value: unknown): value is Record<string, unknown> =>
    typeof value === 'object' && value !== null && !Array.isArray(value);

const decodeValue = (raw: unknown): V2Value => {
    if (!isObject(raw)) {
        throw new V2DecodeError('value node must be an object');
    }
    switch (raw.type) {
        case 'scalar': {
            if (raw.exact === true) {
                return { type: 'scalar', exact: true };
            }
            const { numerator, denominator } = raw;
            if (typeof numerator !== 'string' || typeof denominator !== 'string') {
                throw new V2DecodeError('scalar requires string numerator and denominator');
            }
            return { type: 'scalar', numerator, denominator };
        }
        case 'boolean':
            if (typeof raw.value !== 'boolean') {
                throw new V2DecodeError('boolean requires a boolean value');
            }
            return { type: 'boolean', value: raw.value };
        case 'string':
            if (typeof raw.value !== 'string') {
                throw new V2DecodeError('string requires a string value');
            }
            return { type: 'string', value: raw.value };
        case 'vector': {
            if (!Array.isArray(raw.items)) {
                throw new V2DecodeError('vector requires an items array');
            }
            return { type: 'vector', items: raw.items.map(decodeValue) };
        }
        case 'nil': {
            const { reason } = raw;
            if (reason !== null && typeof reason !== 'string') {
                throw new V2DecodeError('nil reason must be a string or null');
            }
            return { type: 'nil', reason };
        }
        case 'codeBlock':
            return { type: 'codeBlock' };
        default:
            throw new V2DecodeError(`unknown value type: ${String(raw.type)}`);
    }
};

const decodePresentation = (raw: unknown): V2Presentation => {
    if (typeof raw === 'string' && (PRESENTATIONS as readonly string[]).includes(raw)) {
        return raw as V2Presentation;
    }
    throw new V2DecodeError(`unknown presentation: ${String(raw)}`);
};

const decodeObservedValue = (raw: unknown): V2ObservedValue => {
    if (!isObject(raw)) {
        throw new V2DecodeError('observed value must be an object');
    }
    return {
        value: decodeValue(raw.value),
        presentation: decodePresentation(raw.presentation),
    };
};

/**
 * Decode and validate an arbitrary value as a HostProtocolV2 document, throwing
 * {@link V2DecodeError} on any malformed field. This is the safe boundary parser
 * a front-end read path uses instead of trusting the raw wire object.
 */
export const decodeV2Document = (raw: unknown): V2Document => {
    if (!isObject(raw)) {
        throw new V2DecodeError('document must be an object');
    }
    if (raw.protocol !== V2_PROTOCOL) {
        throw new V2DecodeError(`expected protocol ${V2_PROTOCOL}, got ${String(raw.protocol)}`);
    }
    if (!Array.isArray(raw.stack)) {
        throw new V2DecodeError('document stack must be an array');
    }
    return { protocol: V2_PROTOCOL, stack: raw.stack.map(decodeObservedValue) };
};
