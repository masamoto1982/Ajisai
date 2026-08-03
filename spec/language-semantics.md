
<p class="doc-nav"><a href="https://github.com/masamoto1982/Ajisai#readme">README</a> · <a href="docs/index.html">Reference</a> · <a href="https://masamoto1982.github.io/Ajisai/">Playground</a></p>

<h1 id="ajisai-language-semantics">Ajisai Language Semantics</h1>

<p>Status: <strong>Canonical</strong><br>
Version: <strong>2026-07-29</strong></p>

<p>
This document defines the correspondence between Ajisai source programs and observable values, states, effects, and diagnoses. It is a compact semantic kernel: differences between individual Words belong to the machine-readable vocabulary registry, not to parallel prose definitions.
</p>

<p>
Ajisai is built from ten concepts. Everything below is one of them, or a consequence of one of them.
</p>

<ol>
<li>Exact rational arithmetic, closed under square roots, with no rounding.</li>
<li>Three outcomes: a value, a reasoned absence, or an error.</li>
<li>A stack of values and vectors of values.</li>
<li>Code blocks, evaluated only when a Word asks for it.</li>
<li>One modifier axis: consume or keep.</li>
<li>A two-tier dictionary — sealed Core, user-defined User — with content-addressed identity.</li>
<li>A machine-readable contract for every Word.</li>
<li>A pre-execution check of user declarations against those contracts.</li>
<li>One host protocol, which is the only way anything outside the language observes it.</li>
<li>An executable conformance corpus that decides whether an implementation is Ajisai.</li>
</ol>

<nav class="toc">
<h2>Contents</h2>
<ol>
<li><a href="#lang-authority">Authority and Compatibility</a></li>
<li><a href="#lang-source">Source and Desugaring</a></li>
<li><a href="#lang-values">Value Domains</a></li>
<li><a href="#lang-machine">Machine State and Evaluation</a></li>
<li><a href="#lang-modifiers">Stack and Modifiers</a></li>
<li><a href="#lang-failure">Partiality and Failure</a></li>
<li><a href="#lang-collections">Collections and Higher-order Evaluation</a></li>
<li><a href="#lang-dictionary">Dictionary and Effects</a></li>
<li><a href="#lang-contract">Contracts and Static Checking</a></li>
<li><a href="#lang-observation">Observation and Host Protocol</a></li>
<li><a href="#lang-conformance">Conformance</a></li>
</ol>
</nav>

<h2 id="lang-authority">1. Authority and Compatibility</h2>

<h3 id="lang-authority-sources">LANG.AUTHORITY.SOURCES — Normative sources</h3>

<p>
This Language Semantics is authoritative for program meaning. <code>spec/words.json</code> is authoritative for the vocabulary, <code>spec/semantic-families.json</code> for the laws Words share, <code>spec/gui-semantics.md</code> for presentation, and <code>spec/host-protocol-v2.schema.json</code> for the boundary between them. <code>SPECIFICATION.html</code> is generated from those sources and is not edited directly.
</p>

<p>
Neither implementation layout nor explanatory text can override an observable contract. A document that is not in the list above defines nothing; <code>docs/dev/</code> holds design notes and history on those terms.
</p>

<h3 id="lang-authority-present">LANG.AUTHORITY.PRESENT — Present-tense description</h3>

<p>
Ajisai has three reading surfaces: the <strong>README</strong>, the <strong>Reference</strong>, and this <strong>Specification</strong>. Each describes the language as it currently is. A definition, an example, or an explanation may rest only on concepts the language has, and every Word it names must exist in the vocabulary registry.
</p>

<p>
Superseded designs, migration history, and the reasoning behind a change are recorded outside these three surfaces, in notes that define nothing. A negative statement belongs on a reading surface only when it constrains an implementation — "an implementation must not convert malformed use to NIL" is such a constraint — and not when its only content is a contrast with a design the language does not have.
</p>

<h3 id="lang-authority-identity">LANG.AUTHORITY.IDENTITY — Language identity</h3>

<p>
Ajisai identity is the correspondence from normalized source to the ordered observation of stack, output, dictionary state, and structured diagnosis. Two implementations are semantically equivalent when that correspondence agrees for every conforming program.
</p>

<p>
The vocabulary is 57 canonical Words and 15 symbolic aliases. Aliases are surface forms of those Words and are not counted as vocabulary. Within the 57, a 35-Word Semantic Kernel carries the semantic identity of the language and 22 Standard Words carry its practical surface; both are ordinary Core Words in one flat dictionary, reached by their plain names, with contracts, laws, and conformance held to the same standard. Growth is not the goal: a proposed Word that is expressible as a user definition over the existing vocabulary does not belong in Core.
</p>

<h3 id="lang-authority-freedom">LANG.AUTHORITY.FREEDOM — Implementation freedom</h3>

<p>
AST, IR, dispatch, caching, storage layout, numeric representation, and optimization are unobservable. An implementation may change them when all observations and host protocol payload meanings remain unchanged.
</p>

<p>
Internal exact-real representation, Rust enum names, debug strings, allocation identity, GUI colors, and private serialization are not semantic discriminants. In particular an implementation may execute a Word by any route it likes, provided the route is unobservable.
</p>

<h2 id="lang-source">2. Source and Desugaring</h2>

<h3 id="lang-source-text">LANG.SOURCE.TEXT — Source domain</h3>

<p>
A program is Unicode text tokenized by the sealed Ajisai lexical grammar. Tokens distinguish literals, canonical Words, aliases, modifier forms, code blocks, vectors, definitions, and deletion.
</p>

<p>
Malformed delimiters, malformed literals, invalid names, and invalid definition forms are source errors. They do not denote NIL.
</p>

<h3 id="lang-source-normalize">LANG.SOURCE.NORMALIZE — Name normalization</h3>

<p>
Word lookup is case-insensitive through the canonical normalization. A symbolic alias resolves to exactly the same canonical Word contract and executor as its English name.
</p>

<p>
Normalization does not merge distinct value tags, invent dictionary entries, or change string and code-block contents.
</p>

<h3 id="lang-source-desugar">LANG.SOURCE.DESUGAR — Surface forms</h3>

<p>
Desugaring is deterministic and semantics-preserving. Modifier punctuation and the registered delimiter forms lower to canonical concepts before evaluation.
</p>

<p>
If \(D\) is desugaring and \(O\) observation, then \(O(p)=O(D(p))\) for every well-formed program \(p\). Sugar cannot introduce a new value domain or failure category.
</p>

<h3 id="lang-source-code">LANG.SOURCE.CODE — Code values</h3>

<p>
A code block is a tagged value containing source for later evaluation. Producing, storing, displaying, and evaluating code are distinct operations. Evaluation occurs only through a Word whose contract requests it — <code>EXEC</code>, <code>COND</code>, and the higher-order family.
</p>

<p>
Quoted code is not eagerly executed. Code and data are distinct value domains: executable source lives in a CodeBlock, and a Vector is data and is never executable.
</p>

<h2 id="lang-values">3. Value Domains</h2>

<h3 id="lang-values-disjoint">LANG.VALUES.DISJOINT — Tagged domains</h3>

<p>
Values form a disjoint tagged sum of exactly six domains: Scalar, Boolean, String, Vector, NIL, and CodeBlock. Two values are never equal merely because their encodings resemble one another.
</p>

<p>
In particular FALSE is not scalar zero, TRUE is not scalar one, and an absent value is not an ERROR.
</p>

<h3 id="lang-values-exact">LANG.VALUES.EXACT — Exact scalars</h3>

<p>
Every scalar denotes an exact element of the field \(\mathbb{Q}(\sqrt{d_1},\dots,\sqrt{d_k})\) generated over the rationals by square roots of non-negative rationals — a multiquadratic normal form \(\sum_d c_d\sqrt{d}\) with rational \(c_d\). Integer, fraction, decimal, and scientific-notation literals are source forms for exact rationals; <code>SQRT</code> is the only Word that leaves the rationals.
</p>

<p>
Arithmetic performs no intermediate rounding, and coefficients are arbitrary-precision, so a coefficient grows to whatever size the value requires. Comparison over this domain is <strong>total and decidable</strong>: every comparison of two scalars yields TRUE or FALSE in finite time. Values built through different histories compare equal when they denote the same real — \(\sqrt{8}\) and \(\sqrt{2}+\sqrt{2}\) are one value.
</p>

<p>
The internal representation is unobservable. Canonical continued-fraction display is derived from a value; it is not evidence of the runtime storage form.
</p>

<h3 id="lang-values-truth">LANG.VALUES.TRUTH — Two-valued truth</h3>

<p>
The Boolean domain has exactly two values, TRUE and FALSE. <code>AND</code>, <code>OR</code>, and <code>NOT</code> are the ordinary Boolean operations, and every comparison decides.
</p>

<p>
Absence and misuse live outside this domain: an operation that cannot produce a value produces NIL, which is absence rather than undecidability, and an operation that is malformed raises ERROR.
</p>

<h3 id="lang-values-nil">LANG.VALUES.NIL — Diagnostic absence</h3>

<p>
NIL is a value representing absence from a well-formed partial operation. It carries a <strong>reason</strong>: a stable, machine-readable identifier for why production failed. The reason is observable through <code>NIL-REASON</code> and through the protocol.
</p>

<p>
The reason is the entire observable content of a NIL: two NILs with the same reason are the same value. An implementation may emit richer diagnostics on the host channel, and no program behavior may depend on them.
</p>

<h3 id="lang-values-vector">LANG.VALUES.VECTOR — Vectors</h3>

<p>
A Vector is an ordered finite collection of values. Indexing is 0-origin and negative indices count from the end. Vectors nest, and nesting expresses ragged and grouped data.
</p>

<p>
Vector length is semantic even when storage is flattened, shared, or lazily materialized. Order and length are a Vector's whole observable structure, and a nested Vector is an element like any other.
</p>

<h2 id="lang-machine">4. Machine State and Evaluation</h2>

<h3 id="lang-machine-state">LANG.MACHINE.STATE — State</h3>

<p>
A machine state contains the data stack, the dictionary, the output stream, and the execution controls needed by observable contracts.
</p>

<p>
Host-only caches, allocation arenas, compiled plans, and counters are not semantic state.
</p>

<h3 id="lang-machine-transformers">LANG.MACHINE.TRANSFORMERS — Programs</h3>

<p>Each executable token denotes a partial state transformer. A program denotes left-to-right composition of those transformers after desugaring and name resolution.</p>

<p>Execution is deterministic relative to the initial state. Optimization may reassociate internal work only when the observable sequence is unchanged.</p>

<h3 id="lang-machine-word-contract">LANG.MACHINE.WORD — Word contracts</h3>

<p>A canonical Word contract selects a semantic family and supplies its differences: stack arity, consumption, NIL policy, projection condition and reason, error conditions, purity, determinism, effects, clause links, documentation, and executor key.</p>

<p>The executor must refine its contract. Aliases and documentation are projections of the same canonical entry, not independent semantic authorities.</p>

<h3 id="lang-machine-order">LANG.MACHINE.ORDER — Evaluation order</h3>

<p>Token evaluation, output emission, and dictionary mutation preserve their observable order.</p>

<h3 id="lang-machine-limits">LANG.MACHINE.LIMITS — Work limits</h3>

<p>Two limits exist and they mean different things. The <strong>execution-step limit</strong> bounds total work; exhausting it raises its registered ERROR. The <strong>materialization ceiling</strong> bounds how large a single generated collection may become; a well-formed request that exceeds it projects to NIL with reason <code>spaceExhausted</code>.</p>

<p>Both are host safety controls rather than language-semantic constraints: their numeric values are implementation freedom, but the outcome category each produces is normative.</p>

<h2 id="lang-modifiers">5. Stack and Modifiers</h2>

<h3 id="lang-stack-order">LANG.STACK.ORDER — Stack observation</h3>

<p>The stack is an ordered sequence with a distinguished top. A Word takes its operands from the top and returns its results to the top.</p>

<p>A display transformation cannot reorder, coerce, drop, or invent stack values.</p>

<h3 id="lang-modifiers-consumption">LANG.MODIFIERS.CONSUMPTION — The consumption axis</h3>

<p>There is exactly one modifier axis. By default a Word consumes the operands it reads; <code>KEEP</code> leaves them on the stack beneath the result, in their existing order.</p>

<p>A Word selects operands from the top of the stack, validates its registered contract, computes or projects the result, and then commits consumption according to the axis. ERROR does not masquerade as a successful NIL projection.</p>

<h2 id="lang-failure">6. Partiality and Failure</h2>

<h3 id="lang-failure-trichotomy">LANG.FAILURE.TRICHOTOMY — Value, absence, misuse</h3>

<p>Every attempted operation ends in exactly one of three categories. Success yields its registered outputs. Well-formed partial failure yields NIL with a reason. Malformed use yields ERROR.</p>

<p>An implementation must not convert malformed use to NIL and must not raise ERROR merely because a registered partial projection has no value. Recovery operates on absence alone: <code>VENT</code> acts on a NIL top, while an ERROR propagates and halts evaluation.</p>

<h3 id="lang-failure-project">LANG.FAILURE.PROJECT — Projection</h3>

<p>A projection condition is a semantic predicate over well-formed inputs. When it holds, the Word produces NIL with the reason its contract registers.</p>

<p>Division by zero, domain exclusion, out-of-range indexing, failed parsing, and space exhaustion remain distinct reasons.</p>

<h3 id="lang-failure-error">LANG.FAILURE.ERROR — Errors</h3>

<p>Arity failure, nonconforming type, malformed source, invalid dictionary operation, and an exhausted execution-step limit raise their registered ERROR category.</p>

<p>ERROR halts evaluation and propagates. It is never a truth value and never a stack value.</p>

<h3 id="lang-failure-passthrough">LANG.FAILURE.PASSTHROUGH — NIL passthrough</h3>

<p>For a passthrough family, an input NIL flows to the registered output position without changing its reason. Passthrough does not re-run a projection condition and does not relabel the absence.</p>

<p>A non-passthrough Word handles NIL only as its family or contract states.</p>

<h3 id="lang-failure-recovery">LANG.FAILURE.RECOVERY — Recovery</h3>

<p><code>VENT</code> is the single recovery form: a non-NIL top passes through and the following source unit is skipped; a NIL top is discarded and the following source unit is evaluated as the fallback.</p>

<p>Recovery does not erase absence from already emitted output.</p>

<h2 id="lang-collections">7. Collections and Higher-order Evaluation</h2>

<h3 id="lang-collections-lift">LANG.COLLECTIONS.LIFT — Element lifting</h3>

<p>An arithmetic or comparison Word applies element-wise when given vectors. Two vectors combine element-wise when their lengths are equal; a scalar combines with every element of a vector. Any other pairing is ERROR.</p>

<p>Each lane preserves the exactness, truth, NIL, and ERROR distinctions of the scalar law. Vectorization cannot turn an ERROR lane into NIL.</p>

<h3 id="lang-collections-higher">LANG.COLLECTIONS.HIGHER — Higher-order evaluation</h3>

<p><code>MAP</code>, <code>FILTER</code>, <code>FOLD</code>, <code>ANY</code>, and <code>ALL</code> evaluate a CodeBlock once per visited element, in index order, with the block's stack effect isolated to its own operands.</p>

<p>Element visitation order is observable where a supplied block can emit output or mutate dictionary state.</p>

<h3 id="lang-collections-budget">LANG.COLLECTIONS.BUDGET — Materialization</h3>

<p><code>RANGE</code> and <code>FILL</code> honor the materialization ceiling. A well-formed request that cannot materialize within it yields NIL with reason <code>spaceExhausted</code>; malformed dimensions remain ERROR.</p>

<h2 id="lang-dictionary">8. Dictionary and Effects</h2>

<h3 id="lang-dictionary-resolution">LANG.DICTIONARY.RESOLUTION — Deterministic lookup</h3>

<p>The dictionary has two tiers. <strong>Core</strong> holds the 57 canonical Words and is sealed: a Core name cannot be redefined or deleted. <strong>User</strong> holds definitions made by <code>DEF</code>. Resolution is a deterministic function of the normalized name and the current dictionary, and User never shadows Core.</p>

<p>Those two tiers are the whole dictionary: a name resolves in Core or in User, and a Core name is reachable by itself. <code>LOOKUP</code>, hover, the Reference, and execution must identify the same canonical entry.</p>

<h3 id="lang-dictionary-mutation">LANG.DICTIONARY.MUTATION — User Words</h3>

<p><code>DEF</code> binds a name to a code block; <code>DEL</code> removes a User Word and fails with ERROR if any other definition still depends on it. A definition may call itself. A dictionary mutation commits atomically or raises ERROR with no partial visible mutation.</p>

<p>Every Word has a <strong>content identity</strong>: a digest over its normalized definition and the identities of the Words it calls. Two definitions with the same identity are the same Word, and a change to a dependency changes the identity of everything that depends on it.</p>

<h3 id="lang-effects-output">LANG.EFFECTS.OUTPUT — Output</h3>

<p>Output is the only effect that leaves the machine. <code>PRINT</code> consumes its operand and appends it to the ordered output stream; <code>KEEP</code> retains it like any other Word. No other Word emits output, so the output stream is the whole of what a host observes a program doing.</p>

<p>A host may render, capture, or discard the output stream, but may not reorder it or change language-side validation.</p>

<p>A Word may have exactly one other effect, and it stays inside the machine: <code>DEF</code> and <code>DEL</code> change the dictionary, under LANG.DICTIONARY.MUTATION. Output emission and dictionary mutation are therefore the two effects, and LANG.MACHINE.ORDER orders both of them against token evaluation.</p>

<p>Every other Word changes nothing: given the same stack and dictionary it produces the same result. <code>LOOKUP</code> reads the dictionary, so it depends on one without changing it. A Word that evaluates a supplied code block has the effects of that block and no others, so it is pure exactly when the block is.</p>

<h3 id="lang-source-reflection">LANG.SOURCE.REFLECTION — Explicit code/data reflection</h3>

<p>CodeBlock and Vector remain disjoint domains: a Vector is data and is never directly executable. <code>REFLECT</code> is the sole reversible boundary between a CodeBlock token sequence and the versioned canonical Vector <code>[ 'AJISAI-CODE-1' token-record... ]</code>. Records preserve every token variant and its original Number lexeme, String content, and Symbol spelling; display text is not authoritative code.</p>

<p>Reflection is pure, deterministic, non-evaluating, independent of dictionary state, and performs no name resolution or mutation. Strictly malformed code data is ERROR. On legal values it is an involution under token/structural equality. <code>EXEC</code> continues to accept only CodeBlock. <code>REFLECT</code> is neither a String parser nor a macro expander.</p>

<h2 id="lang-contract">9. Contracts and Static Checking</h2>

<h3 id="lang-contract-registry">LANG.CONTRACT.REGISTRY — Machine-readable contracts</h3>

<p>Every Core Word's contract is a machine-readable record in <code>spec/words.json</code>, conforming to <code>spec/words.schema.json</code>. The record is the single place a Word's arity, consumption, NIL policy, projection reason, error conditions, purity, and documentation are stated.</p>

<p>Prose that restates a contract is a projection of that record and carries no independent authority.</p>

<h3 id="lang-contract-check">LANG.CONTRACT.CHECK — Pre-execution check</h3>

<p>A user definition may carry a declaration of its own arity, purity, and NIL behavior. <code>ajisai check --contract</code> verifies that declaration against the Core contracts of the Words it calls, <strong>without running the program</strong>.</p>

<p>The check is deliberately <strong>conservative and partial</strong>. It reports exactly three outcomes per declaration: <em>verified</em>, <em>violated</em>, or <em>cannot verify</em>. Anything outside the syntactic fragment the inference analyzes — a higher-order body whose block is not statically known, or dynamic control — is reported as <em>cannot verify</em> and is never silently passed. A tool that reports <em>verified</em> for an unanalyzable body is nonconforming.</p>

<h2 id="lang-observation">10. Observation and Host Protocol</h2>

<h3 id="lang-observation-projections">LANG.OBSERVATION.PROJECTIONS — Observable surfaces</h3>

<p>Ajisai exposes four projections: Input, Output, Stack, and Dictionary. Presentation may tile or select them according to the Presentation Profile, but their language-side contents are determined here.</p>

<p>Output is the ordered text observation; Stack is the ordered typed values; Dictionary is the resolved Core and User catalog; Input is normalized source and host editing state where applicable.</p>

<h3 id="lang-observation-protocol">LANG.OBSERVATION.PROTOCOL — The host protocol</h3>

<p>One host protocol is current. It fixes execute, step, reset, stack collection and snapshot, Core and User metadata, lookup, and the structured ExecuteResult, Value, and absence payloads. It is the only channel through which anything outside the language observes it.</p>

<p>Every document carries a protocol version field, so a future breaking change is identifiable rather than silent. Within the current version only optional fields may be added; existing field deletion, rename, semantic change, and tuple reorder or reshape are forbidden. A breaking change raises the version, and the superseded protocol is removed rather than kept as a second reader: an implementation offers exactly one protocol.</p>

<h3 id="lang-observation-firewall">LANG.OBSERVATION.FIREWALL — Semantic firewall</h3>

<p>External consumers branch only on published protocol axes. They do not branch on Rust types, debug strings, display text, internal numeric form, or incidental GUI state.</p>

<p>The GUI never infers exact equality, stack effect, resolution, or absence reasons on its own.</p>

<h3 id="lang-observation-diagnosis">LANG.OBSERVATION.DIAGNOSIS — Diagnosis</h3>

<p>An ERROR carries a stable category identifier and a human-readable message; a NIL carries its reason. Those identifiers are the machine-readable surface, and human wording may evolve only where they are preserved.</p>

<h2 id="lang-conformance">11. Conformance</h2>

<h3 id="lang-conformance-corpus">LANG.CONFORMANCE.CORPUS — Executable correspondence</h3>

<p>The conformance corpus links each case to one or more clause IDs and compares stack, output, dictionary state, and diagnosis identifiers. An implementation is Ajisai when it preserves the source-to-observation correspondence for every case.</p>

<p>The corpus is the decision procedure for the question "is this Ajisai?". No prose answers it.</p>

<h3 id="lang-conformance-families">LANG.CONFORMANCE.FAMILIES — Family laws</h3>

<p>Every semantic family has law tests for arity, the consumption modifier, NIL policy, projection, ERROR boundaries, lifting, purity, and effects as applicable. Every Word contract has at least one conformance path to its family and clause IDs.</p>

<h3 id="lang-conformance-change">LANG.CONFORMANCE.CHANGE — Change discipline</h3>

<p>A semantic change begins in one authoritative source, regenerates all derived surfaces, updates clause-linked conformance cases, and demonstrates unchanged observations unless the change is explicitly versioned.</p>

<!-- INCLUDE:12-ai-first-implementation-rules -->

<!-- INCLUDE:presentation-profile -->
